//! One step of a live `profile apply` run (#664): re-read the step's record and P (§9), then
//! create, adopt, report or relaunch, with the profile move of a take-over first. The parent
//! module has the rules; a dry run never reaches this file.
//!
//! **The twins.** The leave and the join are the only writes apply makes itself, and both exist
//! in `holler-cli` already (`profile::delete::detach`, one pane's write of
//! `profile::create::join`). `holler-pane` cannot depend on `holler-cli`, so `leave` and `join`
//! below are private copies, the decision of #664's brief; a follow-up of #664 merges them.
//! Until then each stays what its original does for one pane: the record as read, with
//! `profile` none or P, written by one compare-and-swap at the generation it was read at,
//! answering what that write answers. The join has no undo and deletes no profile: apply did not
//! make P (C10).

use crate::findings::quoted;
use crate::profile_diff::{is_member, PaneDiff};
use crate::tx_launch::{self, LaunchRequest, RelaunchRequest, TxOptions};
use crate::{Pane, PaneError, PaneName, PaneStore, Ports, ProfileName, ProfileSpec};

use super::{
    in_other_profile, other_profile, stored_profile, Action, ApplyRequest, Halt, Outcome, Planned,
    Row, Step,
};

/// What every step of one run works with.
pub(super) struct Cx<'a> {
    pub(super) ports: Ports<'a>,
    pub(super) request: &'a ApplyRequest,
    pub(super) options: &'a TxOptions,
}

/// Run `planned`'s step. A row refused or failed at planning makes the step the plan says, and
/// calls nothing.
pub(super) fn step(cx: &Cx<'_>, planned: &Planned) -> Step {
    let action = planned.row.action();
    let made = match &planned.row {
        Row::Refuse(_) | Row::Unobserved(_) => return planned.dry(),
        Row::Create(name) => create(cx, planned, name),
        Row::Adopt { name, note } => adopt(cx, planned, name).map(|step| step.noted(note.clone())),
        Row::Report { name, diff, note } => {
            report(cx, planned, name, diff).map(|step| step.noted(note.clone()))
        }
        Row::Relaunch { name, why } => {
            relaunch(cx, planned, name).map(|step| step.noted(Some(why.clone())))
        }
    };
    made.unwrap_or_else(|halt| planned.halted(action, halt))
}

/// Row b: launch the pane from P's spec as re-read, in the Herdr session the run was given. A
/// record that appeared since the plan is launch's to refuse (`pane-exists`).
fn create(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
    let fresh = Fresh::read(cx, name)?;
    let request = LaunchRequest {
        name: name.clone(),
        herdr_session: cx.request.herdr_session.clone(),
        spec: fresh.spec,
        profile: Some(fresh.profile),
        spec_only: false,
    };
    tx_launch::launch(cx.ports, &request, cx.options)?;
    Ok(planned.step(Action::Create, Outcome::Created))
}

/// Row c: set `Pane.profile` to P if it is not already, on the record as re-read (C10): no write
/// when it is in P, the join when it is in no profile, and under `--take-over` the leave and then
/// the join when it is in another. Nothing live is touched.
fn adopt(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
    let fresh = Fresh::read(cx, name)?;
    let record = fresh.record(name)?;
    let profile = &fresh.profile;
    if !is_member(&record, profile) {
        let pane_store = cx.ports.pane_store;
        match &record.profile {
            None => {
                join(pane_store, &record, profile)?;
            }
            Some(_) if cx.request.take_over => {
                take_over(pane_store, &record, profile)?;
            }
            Some(_) => return Err(other_profile(&record, profile).into()),
        }
    }
    Ok(planned.step(Action::Adopt, Outcome::Adopted))
}

/// Row d: report the pane's difference from its spec, its row of `diff_profile` from the plan.
/// Under `--take-over` a pane of another profile moves to P first (the leave, then the join);
/// otherwise nothing is written. Nothing live is touched.
fn report(cx: &Cx<'_>, planned: &Planned, name: &PaneName, diff: &PaneDiff) -> Result<Step, Halt> {
    let fresh = Fresh::read(cx, name)?;
    let record = fresh.record(name)?;
    if cx.request.take_over && in_other_profile(&record, &fresh.profile) {
        take_over(cx.ports.pane_store, &record, &fresh.profile)?;
    }
    Ok(Step {
        diff: Some(diff.clone()),
        ..planned.step(Action::Report, Outcome::Reported)
    })
}

/// Row f: relaunch the agent in place (`grid_given: false`) from P's spec as re-read, through
/// `tx_launch::relaunch` unchanged. Under `--take-over` a pane of another profile leaves it and
/// joins P first, so the relaunch's record is the one the join stored, whose generation its
/// record write expects (C11); if the relaunch then fails, the pane stays in P and nothing is
/// compensated (O1). Without a take-over the relaunch's own write sets `Pane.profile` to P.
fn relaunch(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
    let fresh = Fresh::read(cx, name)?;
    let record = fresh.record(name)?;
    let profile = fresh.profile;
    let (record, moved) = if cx.request.take_over && in_other_profile(&record, &profile) {
        let from = record.profile.clone();
        (take_over(cx.ports.pane_store, &record, &profile)?, from)
    } else {
        (record, None)
    };
    let request = RelaunchRequest {
        record,
        spec: fresh.spec,
        grid_given: false,
        profile: Some(profile.clone()),
        spec_only: false,
    };
    if let Err(failure) = tx_launch::relaunch(cx.ports, &request, cx.options) {
        let note = moved.map(|from| {
            let (from, to) = (quoted(from.as_str()), quoted(profile.as_str()));
            format!(
                "{name} left profile {from} and joined {to} before the relaunch, and stays in {to}"
            )
        });
        return Err(Halt {
            note,
            ..Halt::from(failure)
        });
    }
    Ok(planned.step(Action::Relaunch, Outcome::Relaunched))
}

/// A step's record and P as they are now (§9), and P's spec for the pane.
struct Fresh {
    record: Option<Pane>,
    /// P's stored name: the profile the step's writes and runs name.
    profile: ProfileName,
    spec: ProfileSpec,
}

impl Fresh {
    /// Re-read `name`'s record and P. P gone is `profile-not-found`, and P with no spec for the
    /// pane any more is `pane-not-in-profile`: both refusals, so the run goes on.
    fn read(cx: &Cx<'_>, name: &PaneName) -> Result<Self, PaneError> {
        let record = cx.ports.pane_store.get(name)?;
        let profile = stored_profile(cx.ports, &cx.request.profile)?;
        let Some(spec) = profile
            .panes
            .into_iter()
            .find(|spec| spec.pane == name.as_str())
        else {
            return Err(PaneError::PaneNotInProfile {
                what: format!(
                    "{} holds no spec for {name} any more (it changed during the run)",
                    quoted(profile.name.as_str())
                ),
            });
        };
        Ok(Self {
            record,
            profile: profile.name,
            spec,
        })
    }

    /// The re-read record, which a step of rows c, d and f needs: `pane-not-found` when it was
    /// deleted during the run.
    fn record(&self, name: &PaneName) -> Result<Pane, PaneError> {
        self.record.clone().ok_or_else(|| PaneError::PaneNotFound {
            what: format!("{name} (its record was deleted during the run)"),
        })
    }
}

/// Under `--take-over`: `record`, of another profile Q, leaves Q and then joins `profile`, two
/// compare-and-swaps (§3, C10), and the record the join stored is returned. A join that fails
/// after the leave landed answers its own error, noted: the pane is in no profile now, and apply
/// writes no re-join of Q (O4).
fn take_over(
    pane_store: &dyn PaneStore,
    record: &Pane,
    profile: &ProfileName,
) -> Result<Pane, Halt> {
    let left = leave(pane_store, record)?;
    join(pane_store, &left, profile).map_err(|error| {
        let from = record
            .profile
            .as_ref()
            .map_or_else(String::new, |from| quoted(from.as_str()));
        let state = if matches!(
            error,
            PaneError::Timeout { .. } | PaneError::Unavailable { .. }
        ) {
            "may be in no profile now (a join that answered so may have landed)"
        } else {
            "is in no profile now"
        };
        let note = format!(
            "{} left profile {from} before its join to {} failed, so it {state}; apply wrote no \
             re-join of {from}: run apply again",
            record.name,
            quoted(profile.as_str())
        );
        Halt {
            note: Some(note),
            ..Halt::from(error)
        }
    })
}

/// The leave, the twin of `holler-cli`'s `profile::delete::detach` (see the module docs): the
/// record as read, with `profile` none, by compare-and-swap at the generation it was read at.
/// Returns the stored record.
fn leave(pane_store: &dyn PaneStore, record: &Pane) -> Result<Pane, PaneError> {
    let left = Pane {
        profile: None,
        ..record.clone()
    };
    pane_store.cas_put(&left, record.generation)
}

/// The join of one pane, the twin of `holler-cli`'s `profile::create::join` for one pane (see the
/// module docs): the record as read, with `profile` P, by compare-and-swap at the generation it
/// was read at. Returns the stored record.
fn join(
    pane_store: &dyn PaneStore,
    record: &Pane,
    profile: &ProfileName,
) -> Result<Pane, PaneError> {
    let member = Pane {
        profile: Some(profile.clone()),
        ..record.clone()
    };
    pane_store.cas_put(&member, record.generation)
}
