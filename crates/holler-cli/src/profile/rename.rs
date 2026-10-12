//! `holler profile rename OLD NEW` (story #665, epic #633): rename a profile, and with it every
//! member pane's `profile`, in one transaction on the hub.
//!
//! The verb reads OLD (found by name or slug; `profile-not-found`), refuses a NEW equal to OLD's
//! stored name (`usage`), and sends one `ProfileStore::rename` with the generation it read and
//! its own actor. The rest is the hub's `profile/rename` (ADR-0021 section 8, "Rename as
//! built"): `profile-exists` for a NEW whose slug another profile holds, `generation-conflict`
//! when OLD moved since the read, and, in one transaction, the record, the two logs and every
//! member pane's `profile`. No pane is detached, relaunched or touched live, and the verb writes
//! no pane itself.
//!
//! The verb reads and writes the profile registry and nothing else: no pane call, no adapter, no
//! probe.

use clap::Args;
use holler_pane::{Actor, PaneError, Ports, Profile, ProfileName};
use serde::Serialize;

use crate::output::{emit, ErrorBody, VerbCtx};

/// Who the profile's change log says renamed it.
const ACTOR: &str = "holler profile rename";

/// Rename a profile; the panes that belong to it follow, untouched.
///
/// The profile keeps its specs and its history, and every pane that belongs to it names the new
/// name, in one transaction on the hub; no pane is detached, relaunched or touched live. A new
/// name that differs only in spelling is a rename too. Prints `renamed profile OLD (SLUG) to NEW
/// (SLUG), generation G`.
///
/// `--format=json` prints one envelope whose data is `{"profile", "from"}`: the renamed profile
/// as stored, and the `{"name", "slug"}` it had.
#[derive(Args, Debug)]
pub struct ProfileRename {
    /// The profile's name now (spaces allowed).
    pub old: String,
    /// Its new name.
    pub new: String,
}

/// Run `holler profile rename`: read OLD, check NEW, rename, and print the result once.
pub fn run(args: &ProfileRename, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = rename(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `profile rename`. Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize)]
struct Renamed {
    /// The profile as the store returned it, under its new name.
    profile: Profile,
    /// The name and slug it had.
    from: Former,
}

/// What a renamed profile was called.
#[derive(Debug, Serialize)]
struct Former {
    name: ProfileName,
    slug: String,
}

/// The checks, in order, then the one write: OLD and NEW parse (`usage`), OLD exists
/// (`profile-not-found`), NEW is not OLD's stored name (`usage`). A store error passes through
/// with its own code.
fn rename(args: &ProfileRename, ports: Ports<'_>) -> Result<Renamed, PaneError> {
    let old = ProfileName::parse(&args.old)?;
    let new = ProfileName::parse(&args.new)?;
    let actor = Actor::parse(ACTOR)?;
    let stored = ports
        .profile_store
        .get(&old)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", old.as_str()),
        })?;
    if new == stored.name {
        return Err(PaneError::Usage {
            message: format!(
                "{:?} is the profile's name already: give another name",
                new.as_str()
            ),
        });
    }
    let profile = ports
        .profile_store
        .rename(&stored.name, &new, stored.generation, &actor)?;
    Ok(Renamed {
        profile,
        from: Former {
            name: stored.name,
            slug: stored.slug,
        },
    })
}

/// The text form: `renamed profile OLD (SLUG) to NEW (SLUG), generation G`. A profile name holds
/// no control character (`ProfileName` refuses one) and a slug is ASCII letters, digits and `-`,
/// so each prints as it is.
fn render(renamed: &Renamed) -> String {
    let profile = &renamed.profile;
    format!(
        "renamed profile {} ({}) to {} ({}), generation {}",
        renamed.from.name, renamed.from.slug, profile.name, profile.slug, profile.generation
    )
}
