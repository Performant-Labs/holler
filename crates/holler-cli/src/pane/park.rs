//! `holler pane park [PANE] --reason TEXT --release-when WHEN [--profile NAME]` (story #646,
//! part 1): take panes out of service by setting the `hold` of their records to `parked`.
//!
//! `holler pane unpark` (`unpark.rs`) is the same run with the opposite change. The engine, the
//! scope rule and the renderer live here and `unpark.rs` calls them as `super::park`, because
//! the frozen `pane/mod.rs` admits no new module (as `profile/list.rs` shares `count`).
//!
//! - **What changes.** Only the record's `hold`, by one compare-and-swap per pane at the
//!   generation it was read at, never retried. No adapter is called and no profile is written,
//!   so a pane never moves in Herdr and its processes are untouched (ADR-0021 section 3, "Park
//!   and unpark as built"). With no live act there is no reconcile step to print after a failed
//!   write (sections 8 and 12 ask for one after a live act): running the verb again is the
//!   remedy. A write that timed out may have landed, and the rerun then reports that pane as
//!   already in the asked state.
//! - **Scope.** `PANE` alone is that record (`pane-not-found` when there is none). With
//!   `--profile P` the panes are what `ProfileScope::resolve` answers: every member of P, or the
//!   named pane, which must belong to P (`pane-not-in-profile`; a missing P is
//!   `profile-not-found`). These are the arms of `holler_pane::reconcile`'s private `resolve`, so
//!   `pane doctor` and `pane park` read a missing pane the same way. The panes are taken in name
//!   order, and all of them are read before the first write.
//! - **Idempotent.** Park changes only a pane whose hold is `none`, unpark only a parked one. A
//!   pane already in the asked state, or drained, is left exactly as it is (a parked pane keeps
//!   its reason, release condition and `since`) and reported with `changed: false`.
//! - **A failed write** stops the run with that error's code and the message `<pane>: <error>`,
//!   followed, when two or more panes are in scope, by `; parked by this run before it: <names or
//!   none>; not reached: <names or none>`. The envelope cannot carry data with a failure, so the
//!   message carries the partial result; the idempotence makes the rerun safe.
//! - **Exit codes** are `class_of`'s: 0 when the run completes, whether it changed a pane or
//!   not; 2 `usage`; 3 `pane-not-found`, `profile-not-found` or `pane-not-in-profile`; 1 a store
//!   that fails.
//! - **Output.** JSON `data` is `{"panes": [{"name", "changed", "generation", "hold"}]}` in name
//!   order, `generation` the record's after the run and `hold` in `Hold`'s own serde form, its
//!   text raw. Text is one line per pane, with the stored text quoted by `findings::quoted`, so
//!   no control sequence reaches the terminal.
//! - **The text guards.** `--reason` and `--release-when` are trimmed and stored trimmed. One that
//!   is blank, holds a control character or is longer than [`MAX_TEXT_CHARS`] characters is
//!   `usage`, refused here and not by clap, with a message that never holds the value. The record
//!   does not enforce this (the hub stores the hold it is given), so a reader still escapes it.
//!
//! Nothing in Holler reads the hold yet. Whether a parked pane refuses a prompt is decided at
//! `send_prompt` by a later part of #646, not in a verb.

use clap::Args;
use holler_pane::findings::quoted;
use holler_pane::pane::Hold;
use holler_pane::{Pane, PaneError, PaneName, Ports, ProfileName};
use holler_proto::clock::now_millis;
use serde::Serialize;

use super::args::ProfileOpt;
use crate::output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx};

/// The verb, as its usage message names it.
const VERB: &str = "park";

/// The longest `--reason` or `--release-when`, in characters, once trimmed.
///
/// It is the cap of the prompt hold's reason (`holler_hub::holds::MAX_REASON_CHARS`), but that
/// guard cuts a longer reason where this one refuses it, as the pane domain's own guards do
/// (`ProfileName`, `Actor`): the record is shared state that every reader prints. The two holds
/// are kept apart (ADR-0021 section 1), so the constant is not shared.
const MAX_TEXT_CHARS: usize = 200;

/// Take a pane out of service until it is unparked.
#[derive(Args, Debug)]
pub struct PanePark {
    /// Park only this pane (default with `--profile`: every pane of the profile).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    /// Why the pane is parked (one line, at most 200 characters).
    #[arg(long, value_name = "TEXT")]
    pub reason: String,
    /// When the park ends, a time or a condition (one line, at most 200 characters).
    #[arg(long, value_name = "WHEN")]
    pub release_when: String,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane park`: park the panes in scope and print what each one is now.
pub fn run(args: &PanePark, ctx: &mut VerbCtx<'_>) -> i32 {
    run_hold_change(park_request(args), ctx)
}

/// Type park's arguments before anything is read, reporting only the first failure, in this
/// order: the target ([`HoldTarget::parse`]), then `--reason`, then `--release-when`. The run's
/// one `since` is taken here, so every pane it parks carries the same one.
fn park_request(args: &PanePark) -> Result<HoldRequest, PaneError> {
    let target = HoldTarget::parse(VERB, args.pane.as_deref(), args.profile.profile.as_deref())?;
    let reason = checked_text("--reason", &args.reason)?;
    let release_when = checked_text("--release-when", &args.release_when)?;
    Ok(HoldRequest {
        target,
        change: HoldChange::Park {
            reason,
            release_when,
            since: now_millis(),
        },
    })
}

/// `raw` trimmed, or `usage` when it is blank, holds a control character or is longer than
/// [`MAX_TEXT_CHARS`] characters, checked in that order. The message names `flag` and never the
/// value, so no control character of it can reach the output.
fn checked_text(flag: &str, raw: &str) -> Result<String, PaneError> {
    let text = raw.trim();
    let rule = if text.is_empty() {
        Some("must not be blank".to_owned())
    } else if text.chars().any(char::is_control) {
        Some("must not contain a control character".to_owned())
    } else if text.chars().count() > MAX_TEXT_CHARS {
        Some(format!("must be at most {MAX_TEXT_CHARS} characters"))
    } else {
        None
    };
    match rule {
        Some(rule) => Err(PaneError::Usage {
            message: format!("{flag} {rule}"),
        }),
        None => Ok(text.to_owned()),
    }
}

/// A typed park or unpark run: the panes it acts on and the change it makes to each.
#[derive(Debug)]
pub(super) struct HoldRequest {
    pub(super) target: HoldTarget,
    pub(super) change: HoldChange,
}

/// The panes a run acts on, typed from `PANE` and `--profile`.
#[derive(Debug)]
pub(super) enum HoldTarget {
    /// `PANE` alone: that pane's record.
    Named(PaneName),
    /// `--profile P`, with or without `PANE`: what `ProfileScope::resolve` answers.
    Profile {
        profile: ProfileName,
        pane: Option<PaneName>,
    },
}

impl HoldTarget {
    /// Type `PANE` and `--profile` for `pane <verb>`, reporting only the first failure: neither
    /// given is `usage` (`pane <verb> needs a PANE or --profile NAME`), then a bad pane name,
    /// then a bad profile name, each in its parser's own words.
    pub(super) fn parse(
        verb: &str,
        pane: Option<&str>,
        profile: Option<&str>,
    ) -> Result<Self, PaneError> {
        let pane = pane.map(PaneName::parse);
        match (pane, profile.map(ProfileName::parse)) {
            (None, None) => Err(PaneError::Usage {
                message: format!("pane {verb} needs a PANE or --profile NAME"),
            }),
            (Some(pane), None) => Ok(Self::Named(pane?)),
            (pane, Some(profile)) => {
                let pane = pane.transpose()?;
                Ok(Self::Profile {
                    profile: profile?,
                    pane,
                })
            }
        }
    }
}

/// The one change a run makes to each pane in scope.
#[derive(Debug)]
pub(super) enum HoldChange {
    /// Park a pane whose hold is `none`, with these trimmed texts and the run's `since`
    /// (milliseconds since the Unix epoch, the verb's clock).
    Park {
        reason: String,
        release_when: String,
        since: i64,
    },
    /// Unpark a parked pane.
    Unpark,
}

impl HoldChange {
    /// The hold this change writes on a pane whose hold is `hold`, or `None` when the pane is
    /// left as it is: park changes only a pane whose hold is `none`, unpark only a parked one,
    /// and a drained pane is never changed.
    fn next_hold(&self, hold: &Hold) -> Option<Hold> {
        match (self, hold) {
            (
                Self::Park {
                    reason,
                    release_when,
                    since,
                },
                Hold::None,
            ) => Some(Hold::Parked {
                reason: reason.clone(),
                release_when: release_when.clone(),
                since: *since,
            }),
            (Self::Unpark, Hold::Parked { .. }) => Some(Hold::None),
            _ => None,
        }
    }

    /// What the change did to a pane, in a failed run's message (`parked by this run ...`).
    fn done_word(&self) -> &'static str {
        match self {
            Self::Park { .. } => "parked",
            Self::Unpark => "unparked",
        }
    }
}

/// Run a typed request and print its report, or the error that typing it gave, in the run's
/// format. Both verbs end here.
pub(super) fn run_hold_change(
    request: Result<HoldRequest, PaneError>,
    ctx: &mut VerbCtx<'_>,
) -> i32 {
    match request {
        Ok(request) => {
            let report = change_holds(&request, ctx.ports);
            emit(&mut ctx.sink, ctx.format, report, |report| {
                render(&request, report)
            })
        }
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// What a run reports: the panes in scope, in name order. Its JSON form is `{"panes": [...]}`.
#[derive(Debug, Serialize)]
struct HoldReport {
    panes: Vec<PaneHold>,
}

/// One pane of the report: its record's name, generation and hold after the run, and whether
/// this run wrote it. Its JSON keys are its fields, in this order; `hold` is `Hold`'s own serde
/// form.
#[derive(Debug, Serialize)]
struct PaneHold {
    name: PaneName,
    changed: bool,
    generation: u64,
    hold: Hold,
}

impl PaneHold {
    /// A pane this run left as it is.
    fn left(record: &Pane) -> Self {
        Self {
            name: record.name.clone(),
            changed: false,
            generation: record.generation,
            hold: record.hold.clone(),
        }
    }

    /// A pane this run wrote, as the store returned it.
    fn written(stored: Pane) -> Self {
        Self {
            name: stored.name,
            changed: true,
            generation: stored.generation,
            hold: stored.hold,
        }
    }
}

/// Make the request's change to every pane in scope, in name order, by one compare-and-swap
/// each at the generation it was read at. The first write that fails ends the run; see the
/// module doc for its message.
fn change_holds(request: &HoldRequest, ports: Ports<'_>) -> Result<HoldReport, ErrorBody> {
    let scope = in_scope(&request.target, ports).map_err(|error| ErrorBody::from(&error))?;
    let mut panes = Vec::with_capacity(scope.len());
    for (at, record) in scope.iter().enumerate() {
        let Some(hold) = request.change.next_hold(&record.hold) else {
            panes.push(PaneHold::left(record));
            continue;
        };
        let changed = Pane {
            hold,
            ..record.clone()
        };
        match ports.pane_store.cas_put(&changed, record.generation) {
            Ok(stored) => panes.push(PaneHold::written(stored)),
            Err(error) => {
                let progress = progress(&request.change, &scope, at, &panes);
                return Err(ErrorBody {
                    code: ErrorCode::from(&error),
                    message: format!("{}: {error}{progress}", record.name),
                });
            }
        }
    }
    Ok(HoldReport { panes })
}

/// The records in scope, in name order: the one record of a pane named alone
/// (`pane-not-found` when there is none), or what `ProfileScope::resolve` answers for
/// `--profile`, whose membership rule is the only one. The verb sorts them itself, so the order
/// does not depend on the scope's.
fn in_scope(target: &HoldTarget, ports: Ports<'_>) -> Result<Vec<Pane>, PaneError> {
    let mut scope = match target {
        HoldTarget::Named(name) => {
            let record = ports
                .pane_store
                .get(name)?
                .ok_or_else(|| PaneError::PaneNotFound {
                    what: name.to_string(),
                })?;
            vec![record]
        }
        HoldTarget::Profile { profile, pane } => ports.scope.resolve(profile, pane.as_ref())?.panes,
    };
    scope.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(scope)
}

/// What a run that stopped at `scope[failed]` says after the error, when two or more panes are
/// in scope (nothing otherwise): `; <parked> by this run before it: <names>; not reached:
/// <names>`, an empty list being `none`. Only the panes `before` that this run wrote are
/// listed as changed, not one it left as it is.
fn progress(change: &HoldChange, scope: &[Pane], failed: usize, before: &[PaneHold]) -> String {
    if scope.len() < 2 {
        return String::new();
    }
    let changed: Vec<&str> = before
        .iter()
        .filter(|pane| pane.changed)
        .map(|pane| pane.name.as_str())
        .collect();
    let not_reached: Vec<&str> = scope
        .iter()
        .skip(failed + 1)
        .map(|pane| pane.name.as_str())
        .collect();
    format!(
        "; {} by this run before it: {}; not reached: {}",
        change.done_word(),
        listed(&changed),
        listed(&not_reached)
    )
}

/// `names` joined by `, `, or `none` when there is none.
fn listed(names: &[&str]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

/// The text form: one line per pane, or `no panes in profile <q>` for a profile with no member.
fn render(request: &HoldRequest, report: &HoldReport) -> String {
    match &request.target {
        HoldTarget::Profile { profile, .. } if report.panes.is_empty() => {
            format!("no panes in profile {}", quoted(profile.as_str()))
        }
        _ => report
            .panes
            .iter()
            .map(|pane| line(&request.change, pane))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// `<name>: <state>`. A parked pane is `parked (reason <q>, release when <q>)` when this run
/// parked it and `already parked (...)` when it was already; a pane with no hold is `unparked`
/// when this run unparked it and `not parked` otherwise; a drained pane is park's `drained,
/// left as it is` or unpark's `not parked`.
fn line(change: &HoldChange, pane: &PaneHold) -> String {
    let state = match &pane.hold {
        Hold::Parked {
            reason,
            release_when,
            ..
        } => {
            let parked = format!(
                "parked (reason {}, release when {})",
                quoted(reason),
                quoted(release_when)
            );
            if pane.changed {
                parked
            } else {
                format!("already {parked}")
            }
        }
        Hold::None if pane.changed => "unparked".to_owned(),
        Hold::None => "not parked".to_owned(),
        Hold::Drained => match change {
            HoldChange::Park { .. } => "drained, left as it is".to_owned(),
            HoldChange::Unpark => "not parked".to_owned(),
        },
    };
    format!("{}: {state}", pane.name)
}
