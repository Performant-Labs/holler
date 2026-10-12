//! The real run's writes (#650): all of the plan, or nothing. The contract of `profile create
//! --from-current` (#662), with one difference, operator decision 5: every record this run
//! writes is new, so its undo deletes the records it wrote, where `create`'s detaches.
//!
//! 1. **Checks first**, before any write: no pane is blocked (`import-blocked`), and the
//!    profile `fleet` does not exist (`profile-exists`) unless there is nothing to write.
//! 2. **The profile first**, through `insert_profile`, the one place a create race becomes
//!    `profile-exists`: a pane record can name only a profile that exists.
//! 3. **Then each record**, in name order, by compare-and-swap at generation 0.
//! 4. **A failure records nothing** (I3). The undo runs newest first and stops at its first
//!    failed step: (a) the record whose write failed is deleted when it landed after all (a
//!    `timeout` may have), which `get` shows as a member of `fleet`; (b) each record this run
//!    wrote is deleted at the generation its write returned; (c) the profile is deleted at the
//!    generation it was stored at. A completed undo answers the failed write's own error. A
//!    failed step answers `profile-conflict`, naming the commands that reconcile, never one
//!    that acts on a live pane (`pane close` would).

use holler_pane::import::Plan;
use holler_pane::profile_diff::is_member;
use holler_pane::{Actor, Pane, PaneError, PaneName, Ports, Profile, ProfileName};

use crate::output::{ErrorBody, ErrorCode};
use crate::profile::create::insert_profile;
use crate::profile::delete::{pane_list, single_quoted};

/// Who the profile's change log says created it, and deleted it in an undo.
const ACTOR: &str = "holler pane import";

/// What a real run wrote.
pub(super) struct Written {
    /// The profile as the store returned it.
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
    let Some(profile) = &plan.profile else {
        return Ok(None);
    };
    let actor = Actor::parse(ACTOR).map_err(body)?;
    if ports
        .profile_store
        .get(&profile.name)
        .map_err(body)?
        .is_some()
    {
        return Err(body(exists(&profile.name, &plan.records)));
    }
    let stored = insert_profile(ports.profile_store, profile, &actor).map_err(body)?;
    let mut written = Vec::with_capacity(plan.records.len());
    for record in &plan.records {
        match ports.pane_store.cas_put(record, 0) {
            Ok(pane) => written.push(pane),
            Err(error) => {
                let failed = Failed {
                    pane: &record.name,
                    error,
                };
                return Err(failed.undone(ports, &stored, &actor, &written));
            }
        }
    }
    Ok(Some(Written {
        recorded: written.len(),
        profile: stored,
    }))
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
struct Failed<'a> {
    pane: &'a PaneName,
    error: PaneError,
}

/// Where the undo stopped.
enum Stop {
    /// At a record: these records, and the profile, remain.
    Records {
        kept: Vec<PaneName>,
        error: PaneError,
    },
    /// At the profile, every record deleted.
    Profile(PaneError),
}

impl Failed<'_> {
    /// Undo the run (steps (a) to (c) of the module docs) and say how it ended.
    fn undone(
        &self,
        ports: Ports<'_>,
        profile: &Profile,
        actor: &Actor,
        written: &[Pane],
    ) -> ErrorBody {
        match self.undo(ports, profile, actor, written) {
            Ok(()) => ErrorBody {
                code: ErrorCode::from(&self.error),
                message: format!(
                    "{}; nothing was kept (the records written so far and the profile {} were \
                     deleted)",
                    self.error, profile.name
                ),
            },
            Err(stop) => ErrorBody::from(&PaneError::ProfileConflict {
                what: self.reconcile(profile, &stop),
            }),
        }
    }

    /// Steps (a) to (c), stopping at the first that fails.
    fn undo(
        &self,
        ports: Ports<'_>,
        profile: &Profile,
        actor: &Actor,
        written: &[Pane],
    ) -> Result<(), Stop> {
        let all_kept = |landed: bool| {
            let mut kept: Vec<PaneName> = written.iter().map(|pane| pane.name.clone()).collect();
            if landed {
                kept.push(self.pane.clone());
            }
            kept
        };
        // A failed `get` cannot say whether the failed write landed: the records known to
        // remain are named, and the remedy reconciles either way.
        let landed = ports
            .pane_store
            .get(self.pane)
            .map_err(|error| Stop::Records {
                kept: all_kept(false),
                error,
            })?;
        if let Some(record) = landed.filter(|record| is_member(record, &profile.name)) {
            ports
                .pane_store
                .delete(self.pane, record.generation)
                .map_err(|error| Stop::Records {
                    kept: all_kept(true),
                    error,
                })?;
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
        ports
            .profile_store
            .delete(&profile.name, profile.generation, actor)
            .map_err(Stop::Profile)
    }

    /// The `profile-conflict` text of an undo that stopped: what failed, what remains, and
    /// the commands that reconcile.
    fn reconcile(&self, profile: &Profile, stop: &Stop) -> String {
        let fleet = single_quoted(profile.name.as_str());
        let failed = format!(
            "{:?}: the record of {} could not be written ({})",
            profile.name.as_str(),
            self.pane,
            self.error
        );
        match stop {
            Stop::Profile(error) => format!(
                "{failed}; the records this run wrote were deleted, and deleting the profile \
                 failed ({error}); reconcile with: holler profile delete {fleet}, then run \
                 holler pane import again"
            ),
            Stop::Records { kept, error } => format!(
                "{failed}; the undo stopped when a delete failed ({error}), so the records of {} \
                 and the profile remain; reconcile with: holler profile delete {fleet} \
                 --keep-panes, then run holler pane import again (the kept records are already \
                 imported and are skipped), then holler profile delete {fleet} --keep-panes and \
                 holler profile create {fleet} --from-current",
                pane_list(kept)
            ),
        }
    }
}
