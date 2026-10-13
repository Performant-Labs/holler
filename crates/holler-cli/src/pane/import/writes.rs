//! The real run's writes (#650): all of the plan, or nothing. The contract of `profile create
//! --from-current` (#662), with one difference, operator decision 5: every record this run
//! writes is new, so its undo deletes the records it wrote, where `create`'s detaches.
//!
//! 1. **Checks first**, before any write: no pane is blocked (`import-blocked`), and, when the
//!    plan creates `fleet`, it does not exist (`profile-exists`) unless there is nothing to
//!    write.
//! 2. **Create** ([`FleetPlan::Create`]): **the profile first**, through `insert_profile`, the
//!    one place a create race becomes `profile-exists`: a pane record can name only a profile
//!    that exists. **Then each record**, in name order, by compare-and-swap at generation 0.
//! 3. **Extend** ([`FleetPlan::Extend`], `--adopt-claude-code` and an existing `fleet`, #831):
//!    **the records first**, in name order (the profile exists, so they may name it), **then
//!    `fleet`**, by one compare-and-swap of the plan's profile at the generation the plan read
//!    it at, with the actor `holler pane import`; a `fleet` that changed since is
//!    `generation-conflict`. When the plan adds no spec, `fleet` is not written.
//! 4. **A failure records nothing** (I3). The undo runs newest first and stops at its first
//!    failed step: (a) the record whose write failed is deleted when it landed after all (a
//!    `timeout` may have): when `get` shows it as a member of `fleet` and equal to the record
//!    this run planned, apart from its generation, so another actor's record of that name is
//!    never deleted; (b) each record this run wrote is deleted at the generation its write
//!    returned; (c) the profile, when this run created it, is deleted at the generation it was
//!    stored at. An extension's undo is (a) and (b) only: `fleet` is left as it was. A
//!    completed undo answers the failed write's own error. A failed step answers
//!    `profile-conflict`, naming what remains and the commands that reconcile or show it, never
//!    one that acts on a live pane (`pane close` would), and, for an extension, never one that
//!    deletes the `fleet` it did not create.

use holler_pane::import::{FleetPlan, Plan};
use holler_pane::profile_diff::is_member;
use holler_pane::{Actor, Pane, PaneError, PaneName, Ports, Profile, ProfileName};

use crate::output::{ErrorBody, ErrorCode};
use crate::profile::create::insert_profile;
use crate::profile::delete::{pane_list, single_quoted};

/// Who the profile's change log says created or extended it, and deleted it in an undo.
const ACTOR: &str = "holler pane import";

/// What a real run wrote.
pub(super) struct Written {
    /// The profile as the store returned it (as read, for an extension that added no spec).
    pub profile: Profile,
    /// How many records it wrote.
    pub recorded: usize,
}

/// Write `plan` (see the module docs): `None` when there is nothing to write.
pub(super) fn write(plan: &Plan, ports: Ports<'_>) -> Result<Option<Written>, ErrorBody> {
    let body = |error: PaneError| ErrorBody::from(&error);
    if let Some(refusal) = plan.refusal() {
        return Err(body(refusal));
    }
    let actor = Actor::parse(ACTOR).map_err(body)?;
    match &plan.fleet {
        FleetPlan::None => Ok(None),
        FleetPlan::Create(profile) => create(plan, profile, &actor, ports).map(Some),
        FleetPlan::Extend {
            profile,
            generation_read,
            added,
        } => {
            let records = write_records(&plan.records, &profile.name, None, ports)?;
            let profile = if added.is_empty() {
                profile.clone()
            } else {
                ports
                    .profile_store
                    .cas_put(profile, *generation_read, &actor)
                    .map_err(|error| {
                        let failed = Failed {
                            write: Write::Fleet,
                            error,
                        };
                        failed.undone(ports, &profile.name, None, &records)
                    })?
            };
            Ok(Some(Written {
                recorded: records.len(),
                profile,
            }))
        }
    }
}

/// Step 2: the profile, then the records.
fn create(
    plan: &Plan,
    profile: &Profile,
    actor: &Actor,
    ports: Ports<'_>,
) -> Result<Written, ErrorBody> {
    let body = |error: PaneError| ErrorBody::from(&error);
    if ports
        .profile_store
        .get(&profile.name)
        .map_err(body)?
        .is_some()
    {
        return Err(body(exists(&profile.name, &plan.records)));
    }
    let stored = insert_profile(ports.profile_store, profile, actor).map_err(body)?;
    let records = write_records(&plan.records, &stored.name, Some((&stored, actor)), ports)?;
    Ok(Written {
        recorded: records.len(),
        profile: stored,
    })
}

/// Each record of `records`, in order, at generation 0; on a failure, the undo of the run
/// (`created` is the profile this run created, and its actor; `None` in an extension).
fn write_records(
    records: &[Pane],
    fleet: &ProfileName,
    created: Option<(&Profile, &Actor)>,
    ports: Ports<'_>,
) -> Result<Vec<Pane>, ErrorBody> {
    let mut written = Vec::with_capacity(records.len());
    for record in records {
        match ports.pane_store.cas_put(record, 0) {
            Ok(pane) => written.push(pane),
            Err(error) => {
                let failed = Failed {
                    write: Write::Record(record),
                    error,
                };
                return Err(failed.undone(ports, fleet, created, &written));
            }
        }
    }
    Ok(written)
}

/// `profile-exists`: the profile `fleet` exists, and `records` are still to be written.
fn exists(fleet: &ProfileName, records: &[Pane]) -> PaneError {
    PaneError::ProfileExists {
        what: format!(
            "{:?}; the import creates it with the panes not imported yet ({}), so it adds none \
             to a profile that exists (see holler profile show {})",
            fleet.as_str(),
            pane_list(records.iter().map(|record| &record.name)),
            single_quoted(fleet.as_str())
        ),
    }
}

/// The write that failed.
enum Write<'a> {
    /// A record's compare-and-swap: the record this run planned.
    Record(&'a Pane),
    /// The one compare-and-swap of an extension's `fleet`, which has no failing pane.
    Fleet,
}

/// The write that failed, and its error.
struct Failed<'a> {
    write: Write<'a>,
    error: PaneError,
}

/// Where the undo stopped.
enum Stop {
    /// At a record: these records remain (and the profile, when this run created it).
    Records {
        kept: Vec<PaneName>,
        error: PaneError,
    },
    /// At the profile this run created, every record deleted.
    Profile(PaneError),
}

impl Failed<'_> {
    /// Undo the run (steps (a) to (c) of the module docs; (c) only when `created` names the
    /// profile this run created) and say how it ended.
    fn undone(
        &self,
        ports: Ports<'_>,
        fleet: &ProfileName,
        created: Option<(&Profile, &Actor)>,
        written: &[Pane],
    ) -> ErrorBody {
        match self.undo(ports, fleet, created, written) {
            Ok(()) => {
                let kept = match created {
                    Some(_) => {
                        format!("the records written so far and the profile {fleet} were deleted")
                    }
                    None => format!(
                        "the records written so far were deleted; the profile {fleet} was not \
                         changed"
                    ),
                };
                ErrorBody {
                    code: ErrorCode::from(&self.error),
                    message: format!("{}; nothing was kept ({kept})", self.error),
                }
            }
            Err(stop) => ErrorBody::from(&PaneError::ProfileConflict {
                what: self.reconcile(fleet, created.is_some(), &stop),
            }),
        }
    }

    /// Steps (a) to (c), stopping at the first that fails.
    fn undo(
        &self,
        ports: Ports<'_>,
        fleet: &ProfileName,
        created: Option<(&Profile, &Actor)>,
        written: &[Pane],
    ) -> Result<(), Stop> {
        let all_kept = |landed: Option<&PaneName>| {
            let mut kept: Vec<PaneName> = written.iter().map(|pane| pane.name.clone()).collect();
            kept.extend(landed.cloned());
            kept
        };
        if let Write::Record(planned) = self.write {
            // A failed `get` cannot say whether the failed write landed: the records known to
            // remain are named, and the remedy reconciles either way.
            let landed = ports
                .pane_store
                .get(&planned.name)
                .map_err(|error| Stop::Records {
                    kept: all_kept(None),
                    error,
                })?;
            let ours = |record: &Pane| {
                is_member(record, fleet)
                    && Pane {
                        generation: 0,
                        ..record.clone()
                    } == *planned
            };
            if let Some(record) = landed.filter(ours) {
                ports
                    .pane_store
                    .delete(&planned.name, record.generation)
                    .map_err(|error| Stop::Records {
                        kept: all_kept(Some(&planned.name)),
                        error,
                    })?;
            }
        }
        for (left, pane) in written.iter().enumerate().rev() {
            ports
                .pane_store
                .delete(&pane.name, pane.generation)
                .map_err(|error| Stop::Records {
                    kept: written[..=left].iter().map(|p| p.name.clone()).collect(),
                    error,
                })?;
        }
        match created {
            Some((profile, actor)) => ports
                .profile_store
                .delete(&profile.name, profile.generation, actor)
                .map_err(Stop::Profile),
            None => Ok(()),
        }
    }

    /// The `profile-conflict` text of an undo that stopped: what failed, what remains, and
    /// the commands that reconcile (for a `fleet` this run created) or show it (for one it
    /// extended, which it never deletes).
    fn reconcile(&self, fleet: &ProfileName, created: bool, stop: &Stop) -> String {
        let quoted = single_quoted(fleet.as_str());
        let what = match self.write {
            Write::Record(planned) => {
                format!("the record of {} could not be written", planned.name)
            }
            Write::Fleet => "the profile could not be extended".to_owned(),
        };
        let failed = format!("{:?}: {what} ({})", fleet.as_str(), self.error);
        match (stop, created) {
            (Stop::Profile(error), _) => format!(
                "{failed}; the records this run wrote were deleted, and deleting the profile \
                 failed ({error}); reconcile with: holler profile delete {quoted}, then run \
                 holler pane import again"
            ),
            (Stop::Records { kept, error }, true) => format!(
                "{failed}; the undo stopped when a delete failed ({error}), so the records of {} \
                 and the profile remain; reconcile with: holler profile delete {quoted} \
                 --keep-panes, then run holler pane import again (the kept records are already \
                 imported and are skipped), then holler profile delete {quoted} --keep-panes and \
                 holler profile create {quoted} --from-current",
                pane_list(kept)
            ),
            (Stop::Records { kept, error }, false) => format!(
                "{failed}; the undo stopped when a delete failed ({error}), so the records of {} \
                 remain, and the profile {fleet} was not changed; see them with holler pane list \
                 and the profile with holler profile show {quoted}; a re-run of holler pane \
                 import skips the kept records (already-imported) and does not add their specs",
                pane_list(kept)
            ),
        }
    }
}
