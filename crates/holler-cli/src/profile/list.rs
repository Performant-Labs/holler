//! `holler profile list` (story #662, epic #633): every profile with its spec count, its
//! live count and its generation, sorted by slug.
//!
//! A profile's live panes are its members, the pane records whose `profile` has its slug
//! (`holler_pane::profile_diff::is_member`, ADR-0021 section 3). The verb reads the two
//! registries, one `list` call each, and nothing else: no adapter, no probe, no write.
//!
//! `count` is shared with the other profile verbs (as `super::list::count`), because the
//! frozen `profile/mod.rs` admits no new module.

use clap::Args;
use holler_pane::profile_diff::is_member;
use holler_pane::{PaneError, Ports, ProfileName};
use serde::Serialize;

use crate::output::{emit, ErrorBody, VerbCtx};

/// List profiles.
///
/// One line per profile, sorted by slug: `NAME (SLUG): N panes, L live, generation G`,
/// where N is the number of specs and L the number of member panes (the panes whose
/// profile is this one); `no profiles` when there is none.
///
/// `--format=json` prints one envelope whose data is an array of
/// `{"name", "slug", "panes", "live", "generation"}`, sorted by slug.
#[derive(Args, Debug)]
pub struct ProfileList {}

/// Run `holler profile list`: read the profiles and the panes, and print one row per
/// profile.
pub fn run(_args: &ProfileList, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = rows(ctx.ports).map_err(|e| ErrorBody::from(&e));
    emit(&mut ctx.sink, ctx.format, result, |rows| render(rows))
}

/// One profile in summary. Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize)]
struct ProfileRow {
    name: ProfileName,
    slug: String,
    /// The number of specs.
    panes: usize,
    /// The number of member panes.
    live: usize,
    generation: u64,
}

/// Every profile as a row, sorted by slug (the port promises no order). A store error
/// passes through with its own code.
fn rows(ports: Ports<'_>) -> Result<Vec<ProfileRow>, PaneError> {
    let mut profiles = ports.profile_store.list()?;
    let panes = ports.pane_store.list()?;
    profiles.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(profiles
        .into_iter()
        .map(|profile| ProfileRow {
            live: panes
                .iter()
                .filter(|pane| is_member(pane, &profile.name))
                .count(),
            panes: profile.panes.len(),
            slug: profile.slug,
            generation: profile.generation,
            name: profile.name,
        })
        .collect())
}

/// The text form: one line per row, or `no profiles`. A profile name holds no control
/// character (`ProfileName` refuses one) and a slug is ASCII letters, digits and `-`, so
/// both print as they are.
fn render(rows: &[ProfileRow]) -> String {
    if rows.is_empty() {
        return "no profiles".to_owned();
    }
    rows.iter()
        .map(|row| {
            format!(
                "{} ({}): {}, {} live, generation {}",
                row.name,
                row.slug,
                count(row.panes, "pane"),
                row.live,
                row.generation
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `n` and `noun`, with the noun in the plural unless `n` is 1: `1 pane`, `2 panes`,
/// `0 specs`.
pub(crate) fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
