//! Cases 9 to 15 of the `ProfileScope` suite, in a file of their own so that no file of
//! the suite nears the 600-line lint: an act that fails and the restoring write, the two
//! conflicts (on the first write and on the restore), an edit without a profile, and the
//! refusals that come before any write.

use holler_pane::{PaneError, PaneStore, SpecEdit};

use super::{
    changed_spec, edit, edit_with, expect_edited, gained, set, specs, stored, Bench, C4,
    PROFILE_NOT_FOUND, SEEDED,
};
use crate::conformance::pane_store::profile_name;
use crate::conformance::profile_store::{
    actor, history, sample, shown, unchanged, ALPHA, BETA, C1, C2, C3, GAMMA, UPDATED,
};
use crate::conformance::{expect_code, expect_eq, succeeds};
use crate::fixture::sample_spec;
use crate::profile_store::ProfileStoreOp;

const CONFLICT: &str = "generation-conflict";
const PROFILE_CONFLICT: &str = "profile-conflict";
const IN_OTHER_PROFILE: &str = "pane-in-other-profile";

// ASSUMPTION (#663): a restore is one `cas_put` at g + 1, and the scope writes no pane
// record.
/// Case 9: an act that fails with `unavailable` makes `edit_spec` answer exactly that
/// error. Alpha's specs equal what they were before, but its generation is g + 2: the
/// log gained two `updated` entries by one actor, the edit at g + 1 and its reversal at
/// g + 2 (ADR-0021 "Decisions taken", item 1). The act ran once and the pane store is
/// unchanged.
pub(super) fn failed_act_restores_the_specs(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let before = stored(b, &alpha)?;
    let log_before = history(b.profiles, &alpha)?;
    let panes_before = succeeds("pane_store.list", b.panes.list())?;
    let call = "edit_spec(Alpha, c1, Set(s')) with an act that fails";
    let (edited, runs) = edit_with(b, Some(&alpha), C1, &set(changed_spec(C1)), || {
        Err(act_failed())
    })?;
    expect_eq(&format!("what {call} answered"), edited, Err(act_failed()))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    let after = stored(b, &alpha)?;
    expect_eq(
        &format!("the specs after {call}"),
        &after.panes,
        &before.panes,
    )?;
    expect_eq(
        &format!("the generation after {call}"),
        after.generation,
        SEEDED + 2,
    )?;
    expect_eq(
        &format!("what the log gained after {call}"),
        gained(b, &alpha, &log_before)?,
        vec![(SEEDED + 1, UPDATED), (SEEDED + 2, UPDATED)],
    )?;
    expect_eq(
        &format!("pane_store.list after {call}"),
        succeeds("pane_store.list", b.panes.list())?,
        panes_before,
    )
}

// ASSUMPTION (#663): the first write is not retried on a conflict.
/// Case 10: a conflict on the first write (a one-shot `generation-conflict` on the
/// profile store's `cas_put`, as another writer causes) is `generation-conflict` before
/// anything live moves: the act never ran and Alpha is unchanged (ADR-0021 section 8,
/// step 2).
pub(super) fn first_write_conflict_is_generation_conflict(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let before = shown(b.profiles, &alpha)?;
    b.profiles
        .faults()
        .fail_next(ProfileStoreOp::CasPut, PaneError::Conflict);
    let call = "edit_spec(Alpha, c1, Set(s')) when its first write conflicts";
    let (edited, runs) = edit(b, Some(&alpha), C1, &set(changed_spec(C1)))?;
    expect_code(call, edited, CONFLICT)?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 0)?;
    unchanged(b.profiles, &alpha, &before, call)
}

// ASSUMPTION (#663): a restore is one `cas_put` at g + 1, not retried.
/// Case 11: during the act, another writer stores Alpha with other specs, and then the
/// act fails. The restoring write conflicts, so `edit_spec` is `profile-conflict` naming
/// Alpha, and Alpha is the other writer's version, at g + 2 with its specs (ADR-0021
/// section 8, step 6). The act ran once.
pub(super) fn restore_conflict_is_profile_conflict(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let (other, who) = (sample(ALPHA, &[C4])?, actor()?);
    let mut moved = None;
    let call = "edit_spec(Alpha, c1, Set(s')) when Alpha moves during an act that fails";
    let (edited, runs) = edit_with(b, Some(&alpha), C1, &set(changed_spec(C1)), || {
        moved = Some(b.profiles.concurrent_put(&other, &who));
        Err(act_failed())
    })?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    if let Some(put) = moved {
        succeeds("the other writer's concurrent_put inside the act", put)?;
    }
    match edited {
        Err(e) if e.code() == PROFILE_CONFLICT && !e.to_string().contains(ALPHA) => Err(format!(
            "{call}: the `{PROFILE_CONFLICT}` does not name {ALPHA:?}: {e}"
        )),
        edited => expect_code(call, edited, PROFILE_CONFLICT),
    }?;
    let now = stored(b, &alpha)?;
    expect_eq(
        &format!("the generation of Alpha after {call}"),
        now.generation,
        SEEDED + 2,
    )?;
    expect_eq(
        &format!("the specs of Alpha after {call}"),
        now.panes,
        other.panes,
    )
}

// ASSUMPTION (#663): without a profile the scope makes no profile store call.
/// Case 12: without a profile, `edit_spec` only runs the act: `Ok(None)` when the act
/// succeeds and the act's own error when it fails, the act running once each time, and
/// the profile store saw no call at all.
pub(super) fn no_profile_runs_only_the_act(b: &Bench<'_>) -> Result<(), String> {
    let call = "edit_spec(None, c1, Set(s'))";
    let (edited, runs) = edit(b, None, C1, &set(changed_spec(C1)))?;
    expect_eq(&format!("what {call} answered"), edited, Ok(None))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    let call = "edit_spec(None, c1, Set(s')) with an act that fails";
    let (edited, runs) = edit_with(b, None, C1, &set(changed_spec(C1)), || Err(act_failed()))?;
    expect_eq(&format!("what {call} answered"), edited, Err(act_failed()))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    expect_eq(
        "the calls the profile store saw",
        b.profiles.faults().calls(),
        Vec::new(),
    )
}

// ASSUMPTION (#663): `profile-not-found` comes before `pane-in-other-profile`.
/// Case 13: a `Set` in a missing profile is `profile-not-found` before the act and
/// before any write, for c4 (in no profile) and for c1 (in Alpha). A scope that checked
/// membership first would answer c1's with `pane-in-other-profile`, so the profile is
/// checked before the pane. A `Set` is the only edit that membership is checked for.
pub(super) fn missing_profile_is_profile_not_found_before_the_act(
    b: &Bench<'_>,
) -> Result<(), String> {
    let gamma = profile_name(GAMMA)?;
    for (pane, call) in [
        (C4, "edit_spec(Gamma, c4, Set(sample_spec(c4)))"),
        (C1, "edit_spec(Gamma, c1, Set(sample_spec(c1)))"),
    ] {
        let (edited, runs) = edit(b, Some(&gamma), pane, &set(sample_spec(pane)))?;
        expect_code(call, edited, PROFILE_NOT_FOUND)?;
        expect_eq(&format!("the runs of the act of {call}"), runs, 0)?;
    }
    no_write(b, "a Set in a missing profile")
}

// ASSUMPTION (#661/#663): the scope refuses a `Set` for a pane of another profile itself,
// before the profile write, as well as the pane registry inside its compare-and-swap
// (ADR-0021 section 8, step 1, and "Decisions taken", item 2), with or without
// `--spec-only`. A `Remove` is not checked (case 15).
/// Case 14: a `Set` for c3, whose record names Beta, is `pane-in-other-profile` before
/// anything is written or moved: the act never ran, the profile store saw no `cas_put`,
/// and Alpha is unchanged.
pub(super) fn pane_in_other_profile_before_any_write(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let before = shown(b.profiles, &alpha)?;
    let call = "edit_spec(Alpha, c3, Set(sample_spec(c3)))";
    let (edited, runs) = edit(b, Some(&alpha), C3, &set(sample_spec(C3)))?;
    expect_code(call, edited, IN_OTHER_PROFILE)?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 0)?;
    no_write(b, call)?;
    unchanged(b.profiles, &alpha, &before, call)
}

/// Case 15: a `Remove` for c3, whose record names Beta while Alpha holds a detached spec
/// for it, is not refused: Alpha's specs are `[c1, c2]` at g + 1, the act ran once, and
/// Beta and the pane store are unchanged. ADR-0021 section 8 says a detached spec is not
/// refused, and its section 9 gives `pane close` no `pane-in-other-profile`.
pub(super) fn remove_of_a_detached_spec_is_not_refused(b: &Bench<'_>) -> Result<(), String> {
    let beta = profile_name(BETA)?;
    let beta_before = stored(b, &beta)?;
    let panes_before = succeeds("pane_store.list", b.panes.list())?;
    let call = "edit_spec(Alpha, c3, Remove)";
    let edited = edit(b, Some(&profile_name(ALPHA)?), C3, &SpecEdit::Remove)?;
    expect_edited(b, call, edited, &specs(&[C1, C2]))?;
    expect_eq(
        &format!("Beta after {call}"),
        stored(b, &beta)?,
        beta_before,
    )?;
    expect_eq(
        &format!("pane_store.list after {call}"),
        succeeds("pane_store.list", b.panes.list())?,
        panes_before,
    )
}

/// The error the failing acts of the cases answer.
fn act_failed() -> PaneError {
    PaneError::Unavailable {
        what: "act".to_owned(),
    }
}

/// `Ok` when the profile store saw no `cas_put`; `after` names what was called, for the
/// detail.
fn no_write(b: &Bench<'_>, after: &str) -> Result<(), String> {
    let calls = b.profiles.faults().calls();
    if calls.contains(&ProfileStoreOp::CasPut) {
        Err(format!(
            "the profile store saw a cas_put after {after}: {calls:?}"
        ))
    } else {
        Ok(())
    }
}
