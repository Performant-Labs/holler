//! The `ProfileScope` conformance suite: [`run_profile_scope_conformance`] runs the cases
//! [`profile_scope_cases`] lists against any `ProfileScope`, each built over a fresh
//! `FakeProfileStore` and `FakePaneStore` seeded with the fixture below. The suite drives
//! the scope and reads what it did back from the two fakes (slice c of #638, #688).
//!
//! The cases pin the port's rules (`holler_pane::profile`; ADR-0021 section 8). `resolve`
//! scopes a verb to the panes whose records name the profile, and `edit_spec` keeps the I8
//! write order: the profile is written first, by a compare-and-swap at its generation, so
//! the act already sees the edit. A conflict on that first write is
//! `generation-conflict`, and the act never runs. A failed act puts the specs back by a
//! second write, so the specs equal what they were, the generation has moved by two and
//! the log shows the edit and its reversal (ADR-0021 "Decisions taken", item 1). A
//! conflict on that restoring write is `profile-conflict`. Without a profile only the act
//! runs.
//!
//! **The fixture.** The profile store holds `Demo Alpha` with specs for `demo-c1r1`,
//! `demo-c2r1` and `demo-c3r1`, and `Demo Beta` with a spec for `demo-c3r1`, each at
//! generation 1 (g below) and written by the actor `conformance`. The pane store holds
//! `demo-c1r1` and `demo-c2r1` in Alpha, `demo-c3r1` in Beta and `demo-c4r1` in no profile,
//! so Alpha's `demo-c3r1` entry is a detached spec. `Demo Gamma` does not exist. Seeding
//! bypasses both call logs. `s'` is the sample spec of `demo-c1r1` with one field changed,
//! and an act that succeeds counts its runs. "Unchanged" means that `get` and `log` of the
//! profile and the profile store's `list` show what they did before, and a case reads the
//! log as `(generation, actor, kind)`. The suite pins no actor (only that an edit and its
//! reversal share one), no `what` text beyond the profile name of case 11's
//! `profile-conflict`, and no order between the scope's two pane store calls.
//!
//! **What the suite fixes that the port leaves open**, as the store suites did for #639 and
//! #661. Each point carries an `ASSUMPTION (#663)` comment at its case or helper, and the
//! fake the same comment at its code. #663 confirms or amends them here first:
//!
//! - the real scope can be built over any `ProfileStore` and `PaneStore` (the `build`
//!   closure);
//! - the scope writes no pane record: recording the pane is the verb's, inside its act,
//!   whose signature returns `()` (cases 5 and 9);
//! - a `Set` replaces an entry in place, keeping its index (case 5);
//! - the first write is not retried on a conflict (case 10), and a restore is one
//!   `cas_put` at g + 1 (cases 9 and 11);
//! - `profile-not-found` comes before `pane-in-other-profile` (case 13);
//! - `edit_spec(None, ..)` makes no profile store call (case 12);
//! - **open, for #663 to decide (no case pins it):** a restoring write that fails with
//!   anything but a conflict (`timeout`, `store-corrupt`, `unavailable`). The fake returns
//!   that error as it is, so the profile keeps an edit nothing live matches, the error does
//!   not name the profile, and the act's error is lost. ADR-0021 section 8 decides only the
//!   conflict (step 6). #663 may instead name the profile and its unrestored specs, or
//!   print the reconcile step; whichever it picks, the fake and this list are amended to
//!   match.
//!
//! `ASSUMPTION (#661/#663)`: for a `Set` for a pane of another profile, the scope checks
//! `pane-in-other-profile` itself before the profile write (case 14), as well as the pane
//! registry doing so inside its compare-and-swap (ADR-0021 "Decisions taken", item 2), so
//! that nothing is written to the profile and nothing live moves for a pane that cannot
//! join it. It compares slugs, as the fake pane store does, and it refuses with or without
//! `--spec-only`, since the scope cannot see that an act is empty. ADR-0021 section 8,
//! step 1 states it. A `Remove` is not checked (case 15): a detached spec stays removable
//! (ADR-0021 section 8, and section 9's `pane close` row).
//!
//! **#663's acceptance, by case.** "Membership refusal" is cases 3 and 14. "Every-pane
//! scope" is case 1 (and 2 for a named pane). "A failed act leaves the profile's specs
//! equal to before (the generation has moved by two)" is case 9. "A successful act bumps
//! it once" is case 5 (and 6, 7 and 15). "A stale generation gives `profile-conflict`" is
//! case 11, a stale *restore*; a stale *first write* is `generation-conflict` (case 10),
//! under ADR-0021 section 8, step 2. "No profile is touched without `--profile`" is case
//! 12. "The profile must exist" (`profile-not-found`) is cases 4 and 13.
//!
//! This file holds cases 1 to 8 (`resolve` and the edits that succeed) and the helpers
//! every case uses. The child module `act` holds cases 9 to 15: the act that fails, the
//! two conflicts, an edit without a profile and the refusals before any write.

mod act;

use std::sync::Arc;

use holler_pane::{
    Pane, PaneError, PaneStore, Profile, ProfileName, ProfileScope, ProfileSpec, ProfileStore,
    SpecEdit,
};

use super::pane_store::{pane_name, profile_name};
use super::profile_store::{actor, history, sample, Step, ALPHA, BETA, C1, C2, C3, GAMMA, UPDATED};
use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};
use crate::fixture::{sample_pane, sample_spec};
use crate::pane_store::FakePaneStore;
use crate::profile_store::FakeProfileStore;

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&Bench<'_>) -> Result<(), String>;

/// What `edit_spec` answered.
type Edited = Result<Option<Profile>, PaneError>;

const NOT_IN_PROFILE: &str = "pane-not-in-profile";
const PROFILE_NOT_FOUND: &str = "profile-not-found";

/// The fixture's pane in no profile: a neutral name, as `C1` to `C3` are.
const C4: &str = "demo-c4r1";

/// The generation of every profile of the fixture (g in the module docs).
const SEEDED: u64 = 1;

/// The suite, in order: the one table that the runner iterates and
/// [`profile_scope_cases`] lists.
const CASES: [(&str, Case); 15] = [
    (
        "resolve-every-pane-of-the-profile",
        resolve_every_pane_of_the_profile,
    ),
    ("resolve-named-member", resolve_named_member),
    (
        "resolve-non-member-is-pane-not-in-profile",
        resolve_non_member_is_pane_not_in_profile,
    ),
    (
        "resolve-missing-profile-is-profile-not-found",
        resolve_missing_profile_is_profile_not_found,
    ),
    (
        "edit-set-replaces-the-entry-and-bumps-once",
        edit_set_replaces_the_entry_and_bumps_once,
    ),
    (
        "edit-set-adds-a-missing-entry",
        edit_set_adds_a_missing_entry,
    ),
    ("edit-remove-drops-the-entry", edit_remove_drops_the_entry),
    ("the-act-sees-the-edit", the_act_sees_the_edit),
    (
        "failed-act-restores-the-specs",
        act::failed_act_restores_the_specs,
    ),
    (
        "first-write-conflict-is-generation-conflict",
        act::first_write_conflict_is_generation_conflict,
    ),
    (
        "restore-conflict-is-profile-conflict",
        act::restore_conflict_is_profile_conflict,
    ),
    (
        "no-profile-runs-only-the-act",
        act::no_profile_runs_only_the_act,
    ),
    (
        "missing-profile-is-profile-not-found-before-the-act",
        act::missing_profile_is_profile_not_found_before_the_act,
    ),
    (
        "pane-in-other-profile-before-any-write",
        act::pane_in_other_profile_before_any_write,
    ),
    (
        "remove-of-a-detached-spec-is-not-refused",
        act::remove_of_a_detached_spec_is_not_refused,
    ),
];

/// One case's scope under test and the two seeded fakes it was built over.
struct Seeded<S> {
    scope: S,
    profiles: Arc<FakeProfileStore>,
    panes: Arc<FakePaneStore>,
}

/// What a case sees: the scope under test and the two fakes it was built over.
struct Bench<'a> {
    scope: &'a dyn ProfileScope,
    profiles: &'a FakeProfileStore,
    panes: &'a FakePaneStore,
}

/// The ids of the cases [`run_profile_scope_conformance`] runs, in the order it runs
/// them.
pub fn profile_scope_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

// ASSUMPTION (#663): the real scope can be built over any `ProfileStore` and `PaneStore`,
// so `build` makes it over the two fakes the suite seeds.
/// Run every case of [`profile_scope_cases`], in order, and return every case that did
/// not hold.
///
/// Per case the suite seeds a fresh `FakeProfileStore` and `FakePaneStore` with the
/// fixture of the module docs (seeding bypasses the call logs), then `build(profiles,
/// panes)` makes the scope under test over them. The suite drives the scope and inspects
/// the two fakes. A fixture that cannot be seeded fails every case, with the reason. How
/// each implementation runs the suite (the test kit cannot name the CLI):
///
/// ```text
/// // the fake:
/// assert_eq!(run_profile_scope_conformance(|profiles, panes| FakeProfileScope::new(profiles, panes, actor)), Ok(()));
/// // the real scope (#663, in holler-cli's tests): the CLI's ProfileScope built over the two fakes.
/// ```
pub fn run_profile_scope_conformance<S, F>(mut build: F) -> Conformance
where
    S: ProfileScope,
    F: FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S,
{
    run_cases(
        &CASES,
        || (seed(&mut build), ()),
        |case, seeded| {
            let seeded = seeded.as_ref().map_err(String::clone)?;
            case(&Bench {
                scope: &seeded.scope,
                profiles: &seeded.profiles,
                panes: &seeded.panes,
            })
        },
    )
}

/// A fresh fixture and the scope `build` makes over it.
fn seed<S>(
    build: &mut impl FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S,
) -> Result<Seeded<S>, String> {
    let profiles = [sample(ALPHA, &[C1, C2, C3])?, sample(BETA, &[C3])?];
    let profiles = FakeProfileStore::seeded(profiles, &actor()?)
        .map_err(|e| format!("the fixture's profiles cannot be seeded: {e}"))?;
    let panes = seeded_panes().map_err(|e| format!("the fixture's panes cannot be seeded: {e}"))?;
    let (profiles, panes) = (Arc::new(profiles), Arc::new(panes));
    let scope = build(Arc::clone(&profiles), Arc::clone(&panes));
    Ok(Seeded {
        scope,
        profiles,
        panes,
    })
}

/// The fixture's pane store: c1 and c2 in Alpha, c3 in Beta and c4 in no profile.
fn seeded_panes() -> Result<FakePaneStore, PaneError> {
    let (alpha, beta) = (ProfileName::parse(ALPHA)?, ProfileName::parse(BETA)?);
    FakePaneStore::seeded([
        Pane {
            profile: Some(alpha.clone()),
            ..sample_pane(C1)?
        },
        Pane {
            profile: Some(alpha),
            ..sample_pane(C2)?
        },
        Pane {
            profile: Some(beta),
            ..sample_pane(C3)?
        },
        sample_pane(C4)?,
    ])
}

// --- the cases ---

/// Case 1: `resolve(Alpha, None)` is the stored Alpha with the panes whose records name
/// it, `[c1, c2]`, each equal to its record. Not c3, although Alpha holds a spec for it
/// (its record names Beta), and not c4 (no profile).
fn resolve_every_pane_of_the_profile(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let call = "resolve(Alpha, None)";
    let scope = succeeds(call, b.scope.resolve(&alpha, None))?;
    expect_eq(
        &format!("the profile of {call}"),
        &scope.profile,
        &stored(b, &alpha)?,
    )?;
    expect_eq(
        &format!("the panes of {call}"),
        scope.panes,
        records(b, &[C1, C2])?,
    )
}

/// Case 2: `resolve(Alpha, Some(c1))` is the stored Alpha with c1's record alone.
fn resolve_named_member(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let call = "resolve(Alpha, Some(c1))";
    let scope = succeeds(call, b.scope.resolve(&alpha, Some(&pane_name(C1)?)))?;
    expect_eq(
        &format!("the profile of {call}"),
        &scope.profile,
        &stored(b, &alpha)?,
    )?;
    expect_eq(
        &format!("the panes of {call}"),
        scope.panes,
        records(b, &[C1])?,
    )
}

/// Case 3: a named pane whose record does not name the profile is
/// `pane-not-in-profile`: c3, whose record names Beta although Alpha holds a spec for
/// it, and c4, which is in no profile.
fn resolve_non_member_is_pane_not_in_profile(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    for (pane, call) in [
        (C3, "resolve(Alpha, Some(c3))"),
        (C4, "resolve(Alpha, Some(c4))"),
    ] {
        let named = b.scope.resolve(&alpha, Some(&pane_name(pane)?));
        expect_code(call, named, NOT_IN_PROFILE)?;
    }
    Ok(())
}

/// Case 4: a missing profile is `profile-not-found`, with no pane and with a pane named.
fn resolve_missing_profile_is_profile_not_found(b: &Bench<'_>) -> Result<(), String> {
    let gamma = profile_name(GAMMA)?;
    let every = b.scope.resolve(&gamma, None);
    expect_code("resolve(Gamma, None)", every, PROFILE_NOT_FOUND)?;
    let named = b.scope.resolve(&gamma, Some(&pane_name(C1)?));
    expect_code("resolve(Gamma, Some(c1))", named, PROFILE_NOT_FOUND)
}

// ASSUMPTION (#663): a `Set` replaces an entry in place, keeping its index, and the scope
// writes no pane record.
/// Case 5: a `Set` of `s'` for c1 stores Alpha at g + 1 with `s'` at c1's index and the
/// other entries unchanged, and answers the profile as stored. The act ran once, the log
/// gained one `updated` entry at g + 1, and the pane store is unchanged.
fn edit_set_replaces_the_entry_and_bumps_once(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let log_before = history(b.profiles, &alpha)?;
    let panes_before = succeeds("pane_store.list", b.panes.list())?;
    let call = "edit_spec(Alpha, c1, Set(s'))";
    let edited = edit(b, Some(&alpha), C1, &set(changed_spec(C1)))?;
    let expected = vec![changed_spec(C1), sample_spec(C2), sample_spec(C3)];
    expect_edited(b, call, edited, &expected)?;
    expect_eq(
        &format!("what the log gained after {call}"),
        gained(b, &alpha, &log_before)?,
        vec![(SEEDED + 1, UPDATED)],
    )?;
    expect_eq(
        &format!("pane_store.list after {call}"),
        succeeds("pane_store.list", b.panes.list())?,
        panes_before,
    )
}

/// Case 6: a `Set` for c4, which has no entry (and no profile), appends its spec after
/// the three entries, at g + 1.
fn edit_set_adds_a_missing_entry(b: &Bench<'_>) -> Result<(), String> {
    let call = "edit_spec(Alpha, c4, Set(sample_spec(c4)))";
    let edited = edit(b, Some(&profile_name(ALPHA)?), C4, &set(sample_spec(C4)))?;
    expect_edited(b, call, edited, &specs(&[C1, C2, C3, C4]))
}

/// Case 7: a `Remove` for c2 drops its entry at g + 1, and c1 and c3 keep their order.
fn edit_remove_drops_the_entry(b: &Bench<'_>) -> Result<(), String> {
    let call = "edit_spec(Alpha, c2, Remove)";
    let edited = edit(b, Some(&profile_name(ALPHA)?), C2, &SpecEdit::Remove)?;
    expect_edited(b, call, edited, &specs(&[C1, C3]))
}

/// Case 8: inside the act, `get(Alpha)` already holds the edit at g + 1: the profile is
/// written before the act runs (I8).
fn the_act_sees_the_edit(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let mut seen = None;
    let (edited, _) = edit_with(b, Some(&alpha), C1, &set(changed_spec(C1)), || {
        seen = Some(b.profiles.get(&alpha));
        Ok(())
    })?;
    succeeds("edit_spec(Alpha, c1, Set(s'))", edited)?;
    let seen = seen.ok_or_else(|| "edit_spec(Alpha, c1, Set(s')) never ran the act".to_owned())?;
    let inside = succeeds("get(Alpha) inside the act", seen)?
        .ok_or_else(|| "get(Alpha) inside the act found no profile".to_owned())?;
    expect_eq(
        "the generation get(Alpha) shows inside the act",
        inside.generation,
        SEEDED + 1,
    )?;
    expect_eq(
        "the specs get(Alpha) shows inside the act",
        inside.panes,
        vec![changed_spec(C1), sample_spec(C2), sample_spec(C3)],
    )
}

// --- helpers (the cases of `act` use them too) ---

/// The profile `name` as the profile store holds it now, which must exist.
fn stored(b: &Bench<'_>, name: &ProfileName) -> Result<Profile, String> {
    succeeds(&format!("get({name})"), b.profiles.get(name))?
        .ok_or_else(|| format!("get({name}) found no profile; the fixture holds one"))
}

/// The records of the panes `names`, in that order, as the pane store holds them now.
fn records(b: &Bench<'_>, names: &[&str]) -> Result<Vec<Pane>, String> {
    names
        .iter()
        .map(|text| {
            let name = pane_name(text)?;
            succeeds(&format!("pane_store.get({name})"), b.panes.get(&name))?
                .ok_or_else(|| format!("the pane store has no record of {name}"))
        })
        .collect()
}

/// One sample spec per entry of `panes`, in order: the specs of the fixture's profiles.
fn specs(panes: &[&str]) -> Vec<ProfileSpec> {
    panes.iter().copied().map(sample_spec).collect()
}

/// `sample_spec(pane)` with one field changed, its soft context ceiling up by one, so
/// that a `Set` of it shows (`s'` in the module docs).
fn changed_spec(pane: &str) -> ProfileSpec {
    let mut spec = sample_spec(pane);
    spec.context.soft = spec.context.soft.saturating_add(1);
    spec
}

/// The edit that sets `spec`.
fn set(spec: ProfileSpec) -> SpecEdit {
    SpecEdit::Set(Box::new(spec))
}

/// `edit_spec(profile, pane, change)` with an act that counts its runs, runs `inside`
/// and answers what it answers; returns what `edit_spec` answered and the runs.
fn edit_with(
    b: &Bench<'_>,
    profile: Option<&ProfileName>,
    pane: &str,
    change: &SpecEdit,
    mut inside: impl FnMut() -> Result<(), PaneError>,
) -> Result<(Edited, u32), String> {
    let pane = pane_name(pane)?;
    let mut runs = 0;
    let edited = b.scope.edit_spec(profile, &pane, change, &mut || {
        runs += 1;
        inside()
    });
    Ok((edited, runs))
}

/// [`edit_with`] an act that succeeds.
fn edit(
    b: &Bench<'_>,
    profile: Option<&ProfileName>,
    pane: &str,
    change: &SpecEdit,
) -> Result<(Edited, u32), String> {
    edit_with(b, profile, pane, change, || Ok(()))
}

/// `Ok` when the edit of Alpha that `call` names stored `specs` at g + 1 and answered
/// the profile as it is stored now, its act having run once.
fn expect_edited(
    b: &Bench<'_>,
    call: &str,
    (edited, runs): (Edited, u32),
    specs: &[ProfileSpec],
) -> Result<(), String> {
    let edited = succeeds(call, edited)?.ok_or_else(|| {
        format!("{call} answered Ok(None); with a profile it answers the edited profile")
    })?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    expect_eq(
        &format!("what {call} answered, against get(Alpha)"),
        &edited,
        &stored(b, &profile_name(ALPHA)?)?,
    )?;
    expect_eq(
        &format!("the generation after {call}"),
        edited.generation,
        SEEDED + 1,
    )?;
    expect_eq(
        &format!("the specs after {call}"),
        edited.panes.as_slice(),
        specs,
    )
}

/// The `(generation, kind)` of each entry the log of `name` gained since `before`, its
/// `history` read earlier. The log must still start with `before`, and the new entries
/// must share one actor: the scope's, which the suite does not pin.
fn gained(
    b: &Bench<'_>,
    name: &ProfileName,
    before: &[Step],
) -> Result<Vec<(u64, &'static str)>, String> {
    let now = history(b.profiles, name)?;
    let added = now.strip_prefix(before).ok_or_else(|| {
        format!("the log of {name} no longer starts with what it held before: {now:?}")
    })?;
    match added.first() {
        Some((_, first, _)) if added.iter().any(|(_, who, _)| who != first) => Err(format!(
            "the entries added to the log of {name} carry more than one actor: {added:?}"
        )),
        _ => Ok(added
            .iter()
            .map(|&(generation, _, kind)| (generation, kind))
            .collect()),
    }
}
