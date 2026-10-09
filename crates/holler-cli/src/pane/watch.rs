//! `holler pane watch` (story #643, epic #633): the pane store's change feed as a stream,
//! one line per change, NDJSON in JSON mode.
//!
//! "Each change once" is the port's contract, not a dedupe here: the verb prints every
//! change the stream yields, in order. From cursor 0 that is the current state (one put
//! per pane) and then every later change; from `--since N`, every change after N
//! (`holler_pane::ports::Watch`). The verb reads only `ProfileScope::resolve` (with
//! `--profile`, before the stream opens) and `PaneStore::watch`, and it writes nothing.

use std::collections::BTreeSet;

use clap::Args;
use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, Ports, Watch};
use serde::Serialize;

use super::args::ProfileOpt;
use super::list::{profile_name, text_value, PaneRow, COLUMNS};
use crate::output::{emit_error, emit_stream, ErrorBody, VerbCtx};

/// Stream pane changes, one line per change.
///
/// The stream starts with the current state, one put per pane, then prints each change as
/// it happens. With `--since CURSOR` it prints every change after that cursor instead, so
/// passing the "cursor" of the last line seen resumes without a gap or a repeat. With
/// `--until-idle` it stops as soon as nothing more is owed; without it, it runs until it is
/// stopped. A store error ends it (exit 1): start it again with `--since`.
///
/// Text prints `cursor=N put NAME` followed by the `pane list` cells as `key=value`, for
/// example `cursor=4 put demo-c1r2 pos=r2c1 profile=- project=/srv/demo health=unknown
/// shown=- driven=- sync=- hold=none`, or `cursor=N delete NAME`.
///
/// `--format=json` prints NDJSON, one envelope per line, whose data is `{"cursor": N,
/// "name": NAME, "change": "put" or "delete", "pane": ROW or null}`, ROW being a `pane list`
/// row. A watch with nothing to print prints nothing, in JSON mode too.
///
/// With PANE, only that pane's changes are printed. With `--profile`, only the changes of
/// that profile's panes, and a named pane must belong to it: a change is printed when the
/// pane's record names the profile, or when the pane was in it as of the last record the
/// watch saw, so a pane that leaves the profile (or is deleted while in it) prints that one
/// change. With `--since`, the membership starts as the profile's panes now, so a replayed
/// change that took a pane out of the profile is printed only if the replay showed that pane
/// in the profile first.
#[derive(Args, Debug)]
pub struct PaneWatch {
    /// Print only this pane's changes.
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    /// Print the changes after this cursor (the "cursor" of a line already seen) instead of
    /// the current state.
    #[arg(long, value_name = "CURSOR")]
    pub since: Option<u64>,
    /// Stop as soon as nothing more is owed, instead of waiting for the next change.
    #[arg(long)]
    pub until_idle: bool,
}

/// Run `holler pane watch`: refuse before the stream opens, or print its changes until it
/// ends.
pub fn run(args: &PaneWatch, ctx: &mut VerbCtx<'_>) -> i32 {
    match open(args, ctx.ports) {
        Ok(changes) => emit_stream(&mut ctx.sink, ctx.format, changes, render),
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// Type the names (`usage`), check the scope with `--profile` (`profile-not-found`,
/// `pane-not-in-profile`), then open the store's watch (`usage` for a cursor ahead of the
/// store's last change). Every refusal comes before the first line.
fn open(args: &PaneWatch, ports: Ports<'_>) -> Result<Changes, PaneError> {
    let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
    let members = match profile_name(&args.profile)? {
        Some(profile) => {
            let scope = ports.scope.resolve(&profile, pane.as_ref())?;
            Some(Members {
                slug: profile.slug(),
                names: scope.panes.into_iter().map(|member| member.name).collect(),
            })
        }
        None => None,
    };
    let watch = ports
        .pane_store
        .watch(Cursor(args.since.unwrap_or_default()))?;
    Ok(Changes {
        watch,
        pane,
        members,
        until_idle: args.until_idle,
    })
}

/// One line of the stream, and the data of its envelope in JSON mode.
#[derive(Debug, Serialize)]
struct PaneChange {
    /// The change's place in the store's feed: pass it as `--since` to resume after it.
    cursor: Cursor,
    name: PaneName,
    change: ChangeKind,
    /// The pane after a put, as a `pane list` row; `null` after a delete.
    pane: Option<PaneRow>,
}

/// What a change did to the pane's record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChangeKind {
    Put,
    Delete,
}

impl From<PaneEvent> for PaneChange {
    fn from(event: PaneEvent) -> Self {
        let pane = event.pane.as_deref().map(PaneRow::from);
        let change = if pane.is_some() {
            ChangeKind::Put
        } else {
            ChangeKind::Delete
        };
        Self {
            cursor: event.cursor,
            name: event.name,
            change,
            pane,
        }
    }
}

/// The text line: `cursor=N put NAME` and the `pane list` cells as `key=value` (the column
/// name in lower case), or `cursor=N delete NAME`.
fn render(change: &PaneChange) -> String {
    let name = text_value(change.name.as_str());
    let Some(row) = &change.pane else {
        return format!("cursor={} delete {name}", change.cursor.0);
    };
    let mut line = format!("cursor={} put {name}", change.cursor.0);
    for (column, cell) in COLUMNS.iter().zip(row.cells()).skip(1) {
        line.push_str(&format!(" {}={cell}", column.to_ascii_lowercase()));
    }
    line
}

/// The changes a watch prints, in the feed's order: each event that passes the filters,
/// until the stream ends, errs, or (with `--until-idle`) has nothing more to give.
struct Changes {
    watch: Watch<PaneEvent>,
    /// With PANE: only that pane's events.
    pane: Option<PaneName>,
    /// With `--profile`: only the events of its panes.
    members: Option<Members>,
    /// End at the first idle answer instead of waiting on.
    until_idle: bool,
}

impl Iterator for Changes {
    type Item = Result<PaneChange, ErrorBody>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.watch.next()? {
                Ok(Some(event)) => {
                    if self.admits(&event) {
                        return Some(Ok(event.into()));
                    }
                }
                // Idle. Waiting on is not a busy loop: the port's `next()` blocks for up to
                // its bound (the hub's long-poll window) before it answers idle.
                Ok(None) => {
                    if self.until_idle {
                        return None;
                    }
                }
                // Any error ends the stream (the port's rule); `emit_stream` reports it.
                Err(error) => return Some(Err(ErrorBody::from(&error))),
            }
        }
    }
}

impl Changes {
    /// Whether the watch prints `event`: with PANE it must be that pane's, and with
    /// `--profile` it must concern the profile (which also updates the membership).
    fn admits(&mut self, event: &PaneEvent) -> bool {
        if self.pane.as_ref().is_some_and(|name| *name != event.name) {
            return false;
        }
        self.members
            .as_mut()
            .is_none_or(|members| members.admits(event))
    }
}

/// The panes of the `--profile` a watch follows. It starts as the panes `resolve` returned
/// and is updated by every event, so a pane that joins the profile is followed from then on.
struct Members {
    /// The profile's slug: membership is compared by slug, as everywhere.
    slug: String,
    /// The panes in the profile as of the last record the watch saw.
    names: BTreeSet<PaneName>,
}

impl Members {
    /// Whether `event` concerns the profile: its record names it, or the pane was in it as
    /// of the last record seen (a pane leaving the profile, or deleted while in it). The
    /// set then follows the event.
    fn admits(&mut self, event: &PaneEvent) -> bool {
        let was = self.names.contains(&event.name);
        let is = event
            .pane
            .as_deref()
            .is_some_and(|pane| self.names_profile(pane));
        if is {
            self.names.insert(event.name.clone());
        } else {
            self.names.remove(&event.name);
        }
        was || is
    }

    /// Whether the record `pane` names the profile.
    fn names_profile(&self, pane: &Pane) -> bool {
        pane.profile
            .as_ref()
            .is_some_and(|profile| profile.slug() == self.slug)
    }
}
