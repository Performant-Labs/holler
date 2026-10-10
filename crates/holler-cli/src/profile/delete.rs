//! `holler profile delete NAME [--keep-panes]` (story #662, epic #633): delete a profile, refused
//! while any pane belongs to it; with `--keep-panes`, each such pane is detached first and keeps
//! running.
//!
//! A profile's live panes are its members, the pane records whose `profile` has its slug
//! (`holler_pane::profile_diff::is_member`, ADR-0021 section 3). No transaction spans the two
//! registries (ADR-0021 section 8), so `--keep-panes` writes in the order that never leaves a
//! pane naming a profile that is gone: each member leaves first (its `Pane.profile` becomes none
//! by compare-and-swap), then the profile is deleted. A failed detach stops the verb and nothing
//! is re-attached: detaching is what was asked for, running the delete again converges, and a
//! re-attach could fail too (#662 Decision 8).
//!
//! The verb reads and writes the two registries and nothing else: no adapter, no probe, and
//! nothing from the process environment.
//!
//! `detach`, `single_quoted` and `pane_list` are shared with `create` (as
//! `super::delete::...`), because the frozen `profile/mod.rs` admits no new module.

use clap::Args;
use holler_pane::profile_diff::is_member;
use holler_pane::{Actor, Pane, PaneError, PaneName, PaneStore, Ports, Profile, ProfileName};
use serde::Serialize;

use super::list::count;
use crate::output::{emit, ErrorBody, ErrorCode, VerbCtx};

/// Who the profile's change log says deleted it (#662 Decision 9).
const ACTOR: &str = "holler profile delete";

/// Delete a profile.
///
/// Refused while any pane belongs to the profile. With `--keep-panes`, each such pane is
/// detached first (its profile becomes none and it keeps running), then the profile is deleted.
/// Prints `deleted profile NAME (SLUG)`, then `detached, still running: PANE, ...` when panes
/// were detached.
///
/// `--format=json` prints one envelope whose data is `{"name", "slug", "detached"}`, the
/// detached panes in order.
#[derive(Args, Debug)]
pub struct ProfileDelete {
    /// The profile's name.
    pub name: String,
    /// First detach every member pane (its `profile` becomes none; the pane keeps running).
    #[arg(long)]
    pub keep_panes: bool,
}

/// Run `holler profile delete`: check, detach the members with `--keep-panes`, delete the
/// profile, and print the result once.
pub fn run(args: &ProfileDelete, ctx: &mut VerbCtx<'_>) -> i32 {
    let ports = ctx.ports;
    let result = plan(args, ports)
        .map_err(|error| ErrorBody::from(&error))
        .and_then(|plan| write(plan, ports));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `profile delete`. Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize)]
struct Deleted {
    name: ProfileName,
    slug: String,
    /// The panes whose `profile` was cleared, in member order (`[]` without `--keep-panes`).
    detached: Vec<PaneName>,
}

/// What the delete writes, decided before the first write.
struct Plan {
    /// The profile as stored: its name, slug and the generation the delete expects.
    profile: Profile,
    /// Its members, in `list()` order, for `--keep-panes` to detach (without the flag the plan
    /// refuses any member, so this is empty).
    members: Vec<Pane>,
    actor: Actor,
}

/// The checks, in order, all before the first write (#662 B3): NAME parses (`usage`), the
/// profile exists (`profile-not-found`), and it has no member unless `--keep-panes` was given
/// (`profile-has-live-panes`). A store error passes through with its own code.
fn plan(args: &ProfileDelete, ports: Ports<'_>) -> Result<Plan, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let actor = Actor::parse(ACTOR)?;
    let profile = ports
        .profile_store
        .get(&name)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", name.as_str()),
        })?;
    let members: Vec<Pane> = ports
        .pane_store
        .list()?
        .into_iter()
        .filter(|pane| is_member(pane, &profile.name))
        .collect();
    if !members.is_empty() && !args.keep_panes {
        return Err(has_live_panes(&profile.name, &members));
    }
    Ok(Plan {
        profile,
        members,
        actor,
    })
}

/// The writes of `plan`: detach each member, then delete the profile at the generation it was
/// read at.
fn write(plan: Plan, ports: Ports<'_>) -> Result<Deleted, ErrorBody> {
    let Plan {
        profile,
        members,
        actor,
    } = plan;
    let detached = detach_all(ports.pane_store, &members)?;
    ports
        .profile_store
        .delete(&profile.name, profile.generation, &actor)
        .map_err(|error| ErrorBody::from(&delete_failed(error, &profile.name, &detached)))?;
    Ok(Deleted {
        name: profile.name,
        slug: profile.slug,
        detached,
    })
}

/// Detach each of `members` in order and return their names. The first failure stops the verb
/// with the store's code, saying what was detached so far; nothing is re-attached (#662
/// Decision 8) and the profile is not deleted.
fn detach_all(pane_store: &dyn PaneStore, members: &[Pane]) -> Result<Vec<PaneName>, ErrorBody> {
    let mut detached = Vec::with_capacity(members.len());
    for member in members {
        if let Err(error) = detach(pane_store, member) {
            return Err(ErrorBody {
                code: ErrorCode::from(&error),
                message: format!(
                    "{error}; detached so far: {}; the profile was not deleted; run the delete again",
                    pane_list(&detached)
                ),
            });
        }
        detached.push(member.name.clone());
    }
    Ok(detached)
}

/// `pane` leaves its profile: the record as read, with `profile` none, written by
/// compare-and-swap at the generation it was read at. Returns the stored record. Leaving is
/// never a move between profiles, so the registry's `pane-in-other-profile` rule cannot refuse
/// it; a record that changed since it was read is `generation-conflict`.
pub(crate) fn detach(pane_store: &dyn PaneStore, pane: &Pane) -> Result<Pane, PaneError> {
    let left = Pane {
        profile: None,
        ..pane.clone()
    };
    pane_store.cas_put(&left, pane.generation)
}

/// `profile-has-live-panes` for `name` and its `members`, with the command that detaches them.
fn has_live_panes(name: &ProfileName, members: &[Pane]) -> PaneError {
    PaneError::ProfileHasLivePanes {
        what: format!(
            "{:?} has {} ({}); run holler profile delete {} --keep-panes to detach them",
            name.as_str(),
            count(members.len(), "live pane"),
            pane_list(members.iter().map(|pane| &pane.name)),
            single_quoted(name.as_str())
        ),
    }
}

/// The error of the profile delete: a `generation-conflict` after at least one detach is
/// `profile-conflict`, since the detaches stay and the profile moved after them; anything
/// else, and a `generation-conflict` when nothing was detached, passes through.
fn delete_failed(error: PaneError, name: &ProfileName, detached: &[PaneName]) -> PaneError {
    match error {
        PaneError::Conflict if !detached.is_empty() => PaneError::ProfileConflict {
            what: format!(
                "{:?} changed after its panes were detached ({}); run holler profile show {}, then the delete again",
                name.as_str(),
                pane_list(detached),
                single_quoted(name.as_str())
            ),
        },
        other => other,
    }
}

/// The text form: `deleted profile NAME (SLUG)`, then the detached panes when there are any.
/// A profile name holds no control character (`ProfileName` refuses one) and a slug is ASCII
/// letters, digits and `-`, so both print as they are.
fn render(deleted: &Deleted) -> String {
    let first = format!("deleted profile {} ({})", deleted.name, deleted.slug);
    if deleted.detached.is_empty() {
        first
    } else {
        format!(
            "{first}\ndetached, still running: {}",
            pane_list(&deleted.detached)
        )
    }
}

/// `text` POSIX-single-quoted, read back by a shell as one word: each `'` becomes the four
/// characters `'\''`, every other character stays, and the whole is wrapped in `'...'`. A
/// profile name in a command that a message suggests goes through it (#662 B1), so the command
/// pastes into a shell as it is, even for `Some Profile` or a name with a quote or `$(...)`.
pub(crate) fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Pane names joined by `, `, or `none` when there is none. A pane name is lowercase ASCII
/// letters, digits and `-` (ADR 0005), so each prints as it is.
pub(crate) fn pane_list<'a>(names: impl IntoIterator<Item = &'a PaneName>) -> String {
    let names: Vec<&str> = names.into_iter().map(PaneName::as_str).collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}
