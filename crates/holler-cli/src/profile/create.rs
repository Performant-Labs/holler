//! `holler profile create NAME [--from-current | --from PROFILE]` (story #662, epic #633): make a
//! profile that is empty, a detached copy of another profile's specs (`--from`), or a snapshot
//! of every pane in the pane registry that makes each pane a member (`--from-current`).
//!
//! - **Checks first.** Every check runs before the first write (#662 B3): the names parse
//!   (`usage`), NAME is not taken by name or slug (`profile-exists`), PROFILE exists
//!   (`profile-not-found`), and with `--from-current` no pane belongs to another profile
//!   (`pane-in-other-profile`, every such pane at once).
//! - **One create write.** Every form stores its record through `insert_profile`, the one
//!   place that reports the create race as `profile-exists` (B7). Every new record is built by
//!   `holler_pane::profile_snapshot::profile_from_panes`, so no `Profile` is built by hand, and
//!   `--from-current`'s specs come from each pane's record (its position from `herdr.grid`,
//!   never from its name), with no Herdr call.
//! - **Write order.** No transaction spans the two registries (ADR-0021 section 8), so
//!   `--from-current` writes the profile first (a pane can name only a profile that exists),
//!   then each pane joins in `list()` order, its `Pane.profile` set by compare-and-swap. A
//!   failed join undoes everything, since a failure records nothing (I3, #662 B2): the pane
//!   whose join failed leaves if its join landed after all (a `timeout` may have), then each
//!   joined pane leaves, newest first, then the profile is deleted. The undo stops at its first
//!   failed step and reports `profile-conflict` with the commands that reconcile, so the
//!   profile still exists whenever a member may remain.
//!
//! The verb reads and writes the two registries and nothing else: no adapter, no probe, and
//! nothing from the process environment.

use clap::Args;
use holler_pane::profile_diff::is_member;
use holler_pane::profile_snapshot::profile_from_panes;
use holler_pane::{
    Actor, Pane, PaneError, PaneName, PaneStore, Ports, Profile, ProfileName, ProfileStore,
};
use serde::Serialize;

use super::delete::{detach, pane_list, single_quoted};
use super::list::count;
use crate::output::{emit, ErrorBody, ErrorCode, VerbCtx};

/// Who the profile's change log says created it (#662 Decision 9).
const ACTOR: &str = "holler profile create";

/// Create a profile: empty, a detached copy of another (--from), or a snapshot of every pane (--from-current).
///
/// `--from PROFILE` copies PROFILE's specs as they are, and no pane joins. `--from-current`
/// makes one spec per pane in the registry, from its record, and makes each pane a member; a
/// pane that belongs to another profile refuses the whole create. Prints `created profile NAME
/// (SLUG): N specs, generation G`, then `members: PANE, ...` for `--from-current` or `copied
/// from PROFILE; no pane joined` for `--from`.
///
/// `--format=json` prints one envelope whose data is `{"profile", "members"}`: the profile as
/// stored and the panes that joined, in order.
#[derive(Args, Debug)]
pub struct ProfileCreate {
    /// The profile's name (spaces allowed).
    pub name: String,
    /// Snapshot every pane in the registry into specs and make each pane a member.
    #[arg(long, conflicts_with = "from")]
    pub from_current: bool,
    /// Copy the specs of PROFILE; no pane joins (a detached copy).
    #[arg(long, value_name = "PROFILE")]
    pub from: Option<String>,
}

/// Run `holler profile create`: check, write the profile and (with `--from-current`) the
/// memberships, and print the result once.
pub fn run(args: &ProfileCreate, ctx: &mut VerbCtx<'_>) -> i32 {
    let ports = ctx.ports;
    let result = plan(args, ports)
        .map_err(|error| ErrorBody::from(&error))
        .and_then(|plan| write(plan, ports));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `profile create`. Its JSON keys are its serialized fields, in this order.
#[derive(Debug, Serialize)]
struct Created {
    /// The profile as the store returned it.
    profile: Profile,
    /// The panes that joined, in join order (`[]` for an empty profile and for `--from`).
    members: Vec<PaneName>,
    /// Which form ran, for the text form's second line.
    #[serde(skip)]
    form: Form,
}

/// The three forms of `profile create`.
#[derive(Debug)]
enum Form {
    /// No flag: a profile with no specs.
    Empty,
    /// `--from`: a copy of the specs of this stored profile.
    CopyOf(ProfileName),
    /// `--from-current`: a snapshot of every pane, each of which joins.
    Current,
}

/// What the create writes, decided before the first write.
struct Plan {
    /// The new record, at generation 0 (the store sets the generation and the stamps).
    profile: Profile,
    /// The panes that join it, in `list()` order: every pane for `--from-current`, else none.
    joining: Vec<Pane>,
    form: Form,
    actor: Actor,
}

/// The checks, in order, all before the first write (#662 B3): NAME and PROFILE parse
/// (`usage`), NAME is not taken (`profile-exists`), then the form's own read. A store error
/// passes through with its own code.
fn plan(args: &ProfileCreate, ports: Ports<'_>) -> Result<Plan, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let from = args.from.as_deref().map(ProfileName::parse).transpose()?;
    let actor = Actor::parse(ACTOR)?;
    if let Some(stored) = ports.profile_store.get(&name)? {
        return Err(taken(&stored.name));
    }
    match from {
        Some(from) => copy_of(ports.profile_store, &name, &from, actor),
        None if args.from_current => snapshot(ports.pane_store, &name, actor),
        None => Ok(Plan {
            profile: profile_from_panes(&name, &[]),
            joining: Vec::new(),
            form: Form::Empty,
            actor,
        }),
    }
}

/// `--from PROFILE`: a detached copy of PROFILE's specs, verbatim (a spec may name a pane of
/// another profile, or a pane with no record), with no pane read or written. PROFILE is found
/// by name or slug; a missing one is `profile-not-found`.
fn copy_of(
    store: &dyn ProfileStore,
    name: &ProfileName,
    from: &ProfileName,
    actor: Actor,
) -> Result<Plan, PaneError> {
    let source = store.get(from)?.ok_or_else(|| PaneError::ProfileNotFound {
        what: format!("{:?}", from.as_str()),
    })?;
    Ok(Plan {
        profile: Profile {
            panes: source.panes,
            ..profile_from_panes(name, &[])
        },
        joining: Vec::new(),
        form: Form::CopyOf(source.name),
        actor,
    })
}

/// `--from-current`: one spec per pane in the registry, in `list()` order, each from its
/// record, and every pane joins. A pane that belongs to another profile refuses the whole
/// create before anything is written, every such pane named, in `list()` order. The pane
/// registry refuses the same move inside its compare-and-swap; checking first keeps a refused
/// create from writing anything. A pane that already names NAME's slug joins like any other.
fn snapshot(store: &dyn PaneStore, name: &ProfileName, actor: Actor) -> Result<Plan, PaneError> {
    let panes = store.list()?;
    let others: Vec<String> = panes
        .iter()
        .filter(|pane| !is_member(pane, name))
        .filter_map(|pane| {
            pane.profile
                .as_ref()
                .map(|other| format!("{} is in profile {:?}", pane.name, other.as_str()))
        })
        .collect();
    if !others.is_empty() {
        return Err(PaneError::PaneInOtherProfile {
            what: others.join("; "),
        });
    }
    Ok(Plan {
        profile: profile_from_panes(name, &panes),
        joining: panes,
        form: Form::Current,
        actor,
    })
}

/// The writes of `plan`: the profile first, since a pane can name only a profile that exists
/// (the hub's membership check), then each pane joins.
fn write(plan: Plan, ports: Ports<'_>) -> Result<Created, ErrorBody> {
    let profile = insert_profile(ports.profile_store, &plan.profile, &plan.actor)
        .map_err(|error| ErrorBody::from(&error))?;
    let members = join(
        ports.pane_store,
        ports.profile_store,
        &profile,
        &plan.joining,
        &plan.actor,
    )
    .map_err(|failed| failed.body(&profile.name))?;
    Ok(Created {
        profile,
        members,
        form: plan.form,
    })
}

/// Store `profile` as a new record (expected generation 0) and return it as stored: the one
/// create write of every form of `profile create` (#662 B7), so the race mapping exists once.
///
/// A `generation-conflict` means another writer created the name between the caller's read and
/// this write (the hub's store answers `next_generation(1, 0)`): to the operator that is a
/// taken name, `profile-exists`. Every other error passes through, the store's own
/// `profile-exists` (the slug is filed under another spelling) included. It takes the port and
/// answers a `PaneError`, with no CLI type, so a caller outside this verb can use it.
pub(crate) fn insert_profile(
    store: &dyn ProfileStore,
    profile: &Profile,
    actor: &Actor,
) -> Result<Profile, PaneError> {
    store
        .cas_put(profile, 0, actor)
        .map_err(|error| match error {
            PaneError::Conflict => taken(&profile.name),
            other => other,
        })
}

/// `profile-exists` for `name`: `"<name>" (slug <slug>)`.
fn taken(name: &ProfileName) -> PaneError {
    PaneError::ProfileExists {
        what: format!("{:?} (slug {})", name.as_str(), name.slug()),
    }
}

/// How a join failed: the pane whose join failed, the store's error, and the error of the
/// first undo step that failed (`None` when the undo completed).
struct JoinFailed {
    pane: PaneName,
    error: PaneError,
    undo_error: Option<PaneError>,
}

impl JoinFailed {
    /// The envelope's error for a failed join into `profile` (#662 B4): the join's own code
    /// when the undo completed, else `profile-conflict` naming the commands that reconcile. One
    /// line either way.
    fn body(&self, profile: &ProfileName) -> ErrorBody {
        match &self.undo_error {
            None => ErrorBody {
                code: ErrorCode::from(&self.error),
                message: format!(
                    "{}; nothing was kept (the profile and the memberships made so far were undone)",
                    self.error
                ),
            },
            Some(undo_error) => {
                let quoted = single_quoted(profile.as_str());
                ErrorBody::from(&PaneError::ProfileConflict {
                    what: format!(
                        "{:?}: pane {} could not join ({}) and the undo failed ({undo_error}); \
                         reconcile with: holler profile show {quoted} and holler profile delete \
                         {quoted} --keep-panes",
                        profile.as_str(),
                        self.pane,
                        self.error
                    ),
                })
            }
        }
    }
}

/// Make each of `panes` a member of `profile`, in order, and return their names: each record
/// as listed, with `profile` set, written by compare-and-swap at the generation it was listed
/// at. A failed join is undone (see `undo`), and the failure says how far the undo got.
fn join(
    pane_store: &dyn PaneStore,
    profile_store: &dyn ProfileStore,
    profile: &Profile,
    panes: &[Pane],
    actor: &Actor,
) -> Result<Vec<PaneName>, JoinFailed> {
    let mut joined = Vec::with_capacity(panes.len());
    for pane in panes {
        let member = Pane {
            profile: Some(profile.name.clone()),
            ..pane.clone()
        };
        match pane_store.cas_put(&member, pane.generation) {
            Ok(stored) => joined.push(stored),
            Err(error) => {
                let undone = undo(
                    pane_store,
                    profile_store,
                    profile,
                    &pane.name,
                    &joined,
                    actor,
                );
                return Err(JoinFailed {
                    pane: pane.name.clone(),
                    error,
                    undo_error: undone.err(),
                });
            }
        }
    }
    Ok(joined.into_iter().map(|pane| pane.name).collect())
}

/// Undo a failed join of `failed` (#662 B2), stopping at the first step that fails: `failed`
/// leaves if its record shows it joined after all (a write that answered `timeout` or
/// `unavailable` may have landed); each pane in `joined` leaves, newest first; then the
/// profile is deleted at the generation it was stored at. Stopping at the first failure keeps
/// the profile whenever a member may remain, so the remedy the error names can always run.
fn undo(
    pane_store: &dyn PaneStore,
    profile_store: &dyn ProfileStore,
    profile: &Profile,
    failed: &PaneName,
    joined: &[Pane],
    actor: &Actor,
) -> Result<(), PaneError> {
    if let Some(record) = pane_store
        .get(failed)?
        .filter(|record| is_member(record, &profile.name))
    {
        detach(pane_store, &record)?;
    }
    for pane in joined.iter().rev() {
        detach(pane_store, pane)?;
    }
    profile_store.delete(&profile.name, profile.generation, actor)
}

/// The text form: `created profile NAME (SLUG): N specs, generation G`, then the form's own
/// line. A profile name holds no control character (`ProfileName` refuses one) and a slug is
/// ASCII letters, digits and `-`, so both print as they are.
fn render(created: &Created) -> String {
    let profile = &created.profile;
    let first = format!(
        "created profile {} ({}): {}, generation {}",
        profile.name,
        profile.slug,
        count(profile.panes.len(), "spec"),
        profile.generation
    );
    match &created.form {
        Form::Empty => first,
        Form::CopyOf(source) => format!("{first}\ncopied from {source}; no pane joined"),
        Form::Current => format!("{first}\nmembers: {}", pane_list(&created.members)),
    }
}
