#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #688
//! `FakeProfileScope`'s own mechanisms: how faults in the two stores it wraps reach a
//! caller, the order of the calls it makes, its own guards (a spec filed under the wrong
//! pane, an absent entry, slug membership, a detached spec), the one-shot hook before the
//! restoring write, and the actor it logs. The generic scope rules (I8 order, restore,
//! refusals) are the conformance suite's job; this file pins what only the fake has
//! (#638, slice c part 2, #688).

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use holler_pane::{
    Actor, Pane, PaneError, PaneName, PaneStore, Profile, ProfileChange, ProfileName, ProfileScope,
    ProfileStore, SpecEdit,
};
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::fixture::{sample_pane, sample_profile, sample_spec};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";
const C3: &str = "demo-c3r1";
const C4: &str = "demo-c4r1";

fn actor(text: &str) -> Actor {
    Actor::parse(text).unwrap()
}

fn pname(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn alpha() -> ProfileName {
    ProfileName::parse("Demo Alpha").unwrap()
}

fn beta() -> ProfileName {
    ProfileName::parse("Demo Beta").unwrap()
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn corrupt() -> PaneError {
    PaneError::StoreCorrupt {
        what: "store".to_owned(),
    }
}

fn unavailable(what: &str) -> PaneError {
    PaneError::Unavailable {
        what: what.to_owned(),
    }
}

fn set(pane: &str) -> SpecEdit {
    SpecEdit::Set(Box::new(sample_spec(pane)))
}

/// The pane names of a profile's specs, in order.
fn spec_panes(profile: &Profile) -> Vec<&str> {
    profile
        .panes
        .iter()
        .map(|spec| spec.pane.as_str())
        .collect()
}

/// A pane of the fixture, a member of `profile` when one is given.
fn member(name: &str, profile: Option<&str>) -> Pane {
    Pane {
        profile: profile.map(|text| ProfileName::parse(text).unwrap()),
        ..sample_pane(name).unwrap()
    }
}

/// The scope under test and the two fakes it wraps. Alpha holds specs for c1, c2 and c3
/// (c3 is detached: its pane belongs to Beta) at generation 1, Beta holds c3. Panes: c1
/// and c2 in Alpha, c3 in Beta, c4 in none. Seeded by "seeder"; the scope logs as
/// "scope-actor".
struct Rig {
    scope: Arc<FakeProfileScope>,
    profiles: Arc<FakeProfileStore>,
    panes: Arc<FakePaneStore>,
}

fn rig() -> Rig {
    rig_with(Vec::new())
}

fn rig_with(extra: Vec<Pane>) -> Rig {
    let seeder = actor("seeder");
    let profiles = Arc::new(
        FakeProfileStore::seeded(
            [
                sample_profile("Demo Alpha", &[C1, C2, C3]).unwrap(),
                sample_profile("Demo Beta", &[C3]).unwrap(),
            ],
            &seeder,
        )
        .unwrap(),
    );
    let mut records = vec![
        member(C1, Some("Demo Alpha")),
        member(C2, Some("Demo Alpha")),
        member(C3, Some("Demo Beta")),
        member(C4, None),
    ];
    records.extend(extra);
    let panes = Arc::new(FakePaneStore::seeded(records).unwrap());
    let scope = Arc::new(FakeProfileScope::new(
        profiles.clone(),
        panes.clone(),
        actor("scope-actor"),
    ));
    Rig {
        scope,
        profiles,
        panes,
    }
}

type Edited = Result<Option<Profile>, PaneError>;

/// `edit_spec` with an act that runs `inside` and then answers `outcome`; returns the
/// result and how many times the act ran.
fn edit_with(
    rig: &Rig,
    profile: Option<&ProfileName>,
    pane: &str,
    edit: &SpecEdit,
    inside: impl Fn(),
    outcome: Result<(), PaneError>,
) -> (Edited, u32) {
    let runs = Cell::new(0_u32);
    let mut act = || {
        runs.set(runs.get() + 1);
        inside();
        outcome.clone()
    };
    let result = rig.scope.edit_spec(profile, &pname(pane), edit, &mut act);
    (result, runs.get())
}

fn edit(
    rig: &Rig,
    profile: Option<&ProfileName>,
    pane: &str,
    edit: &SpecEdit,
    outcome: Result<(), PaneError>,
) -> (Edited, u32) {
    edit_with(rig, profile, pane, edit, || {}, outcome)
}

fn stored(rig: &Rig, name: &ProfileName) -> Profile {
    rig.profiles.get(name).unwrap().unwrap()
}

fn code_of<T>(result: Result<T, PaneError>) -> Option<String> {
    result.err().map(|e| e.code().to_owned())
}

// --- faults reach the scope through its stores (AC3) ---

#[test]
fn a_wedged_profile_store_times_out_resolve_and_edit_spec() {
    let r = rig();
    r.profiles.faults().set(Some(Fault::Wedged));
    assert_eq!(
        r.scope.resolve(&alpha(), None),
        Err(timeout("profile_store.get"))
    );
    let (result, runs) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert_eq!(result, Err(timeout("profile_store.get")));
    assert_eq!(runs, 0);

    r.profiles.faults().set(None);
    assert!(r.scope.resolve(&alpha(), None).is_ok());
    let (result, runs) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert!(matches!(result, Ok(Some(_))), "{result:?}");
    assert_eq!(runs, 1);
}

#[test]
fn a_wedged_pane_store_times_out_resolve_and_edit_spec() {
    let r = rig();
    r.panes.faults().set(Some(Fault::Wedged));
    assert_eq!(
        r.scope.resolve(&alpha(), None),
        Err(timeout("pane_store.list"))
    );
    assert_eq!(
        r.scope.resolve(&alpha(), Some(&pname(C1))),
        Err(timeout("pane_store.get"))
    );
    let (result, runs) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert_eq!(result, Err(timeout("pane_store.get")));
    assert_eq!(runs, 0);
    assert!(
        !r.profiles
            .faults()
            .calls()
            .contains(&ProfileStoreOp::CasPut),
        "nothing is written when the pane record cannot be read"
    );
}

#[test]
fn a_corrupt_profile_store_fails_closed() {
    let r = rig();
    r.profiles.faults().set(Some(Fault::Fail(corrupt())));
    assert_eq!(r.scope.resolve(&alpha(), None), Err(corrupt()));
    let (result, runs) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert_eq!(result, Err(corrupt()));
    assert_eq!(runs, 0);
}

#[test]
fn a_failed_restore_returns_its_own_error() {
    let r = rig();
    let (result, runs) = edit_with(
        &r,
        Some(&alpha()),
        C1,
        &set(C1),
        || {
            r.profiles
                .faults()
                .fail_next(ProfileStoreOp::CasPut, corrupt());
        },
        Err(unavailable("act")),
    );
    assert_eq!(result, Err(corrupt()));
    assert_eq!(runs, 1);
    // The edit stays: the restoring write never happened.
    let alpha_now = stored(&r, &alpha());
    assert_eq!(alpha_now.generation, 2);
}

#[test]
fn without_a_profile_a_wedged_profile_store_is_not_called() {
    let r = rig();
    r.profiles.faults().set(Some(Fault::Wedged));
    let (result, runs) = edit(&r, None, C1, &set(C1), Ok(()));
    assert_eq!(result, Ok(None));
    assert_eq!(runs, 1);
}

#[test]
fn the_calls_follow_the_i8_order() {
    let ok = rig();
    let (result, _) = edit(&ok, Some(&alpha()), C1, &set(C1), Ok(()));
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        ok.profiles.faults().calls(),
        vec![ProfileStoreOp::Get, ProfileStoreOp::CasPut]
    );
    assert_eq!(ok.panes.faults().calls(), vec![PaneStoreOp::Get]);

    let failed = rig();
    let (result, _) = edit(
        &failed,
        Some(&alpha()),
        C1,
        &set(C1),
        Err(unavailable("act")),
    );
    assert_eq!(result, Err(unavailable("act")));
    assert_eq!(
        failed.profiles.faults().calls(),
        vec![
            ProfileStoreOp::Get,
            ProfileStoreOp::CasPut,
            ProfileStoreOp::CasPut
        ]
    );
}

// --- the fake's own rules (AC4) ---

#[test]
fn a_set_whose_spec_names_another_pane_is_usage() {
    let r = rig();
    let (result, runs) = edit(&r, Some(&alpha()), C1, &set(C2), Ok(()));
    assert_eq!(code_of(result).as_deref(), Some("usage"));
    assert_eq!(runs, 0);
    assert!(!r
        .profiles
        .faults()
        .calls()
        .contains(&ProfileStoreOp::CasPut));
}

#[test]
fn removing_an_absent_entry_still_writes_the_profile() {
    let r = rig();
    let before = stored(&r, &alpha());
    let (result, runs) = edit(&r, Some(&alpha()), C4, &SpecEdit::Remove, Ok(()));
    let written = result.unwrap().unwrap();
    assert_eq!(runs, 1);
    assert_eq!(written.generation, before.generation + 1);
    assert_eq!(written.panes, before.panes);
}

#[test]
fn membership_compares_slugs() {
    // Display names that differ only in case and punctuation share a slug.
    let r = rig_with(vec![member("demo-c5r1", Some("DEMO-ALPHA"))]);
    let listed = r.scope.resolve(&alpha(), None).unwrap();
    let names: Vec<String> = listed.panes.iter().map(|p| p.name.to_string()).collect();
    assert_eq!(names, vec![C1, C2, "demo-c5r1"]);

    let (result, runs) = edit(&r, Some(&alpha()), "demo-c5r1", &set("demo-c5r1"), Ok(()));
    let written = result.unwrap().unwrap();
    assert_eq!(runs, 1);
    assert_eq!(spec_panes(&written), vec![C1, C2, C3, "demo-c5r1"]);
}

#[test]
fn removing_a_detached_spec_is_not_refused() {
    let r = rig();
    let before = stored(&r, &alpha());
    let beta_before = stored(&r, &beta());
    let panes_before = r.panes.list().unwrap();
    let pane_calls_before = r.panes.faults().calls();
    assert_eq!(pane_calls_before, vec![PaneStoreOp::List]);

    let (result, runs) = edit(&r, Some(&alpha()), C3, &SpecEdit::Remove, Ok(()));
    let written = result.unwrap().unwrap();
    assert_eq!(runs, 1);
    assert_eq!(spec_panes(&written), vec![C1, C2]);
    assert_eq!(written.generation, before.generation + 1);
    assert_eq!(stored(&r, &beta()), beta_before);
    assert_eq!(r.panes.list().unwrap(), panes_before);
    // The pane record is still read for a Remove, and read once.
    assert_eq!(
        r.panes.faults().calls(),
        vec![PaneStoreOp::List, PaneStoreOp::Get, PaneStoreOp::List]
    );
}

/// Arm `before_next_restore` so that another writer stores `other` and the count of
/// runs goes up.
fn arm_other_writer(r: &Rig, other: &Profile, runs: &Arc<AtomicUsize>) {
    let profiles = r.profiles.clone();
    let other = other.clone();
    let runs = runs.clone();
    r.scope.before_next_restore(move || {
        runs.fetch_add(1, Ordering::SeqCst);
        profiles
            .concurrent_put(&other, &actor("other-writer"))
            .unwrap();
    });
}

#[test]
fn a_hook_before_the_restore_makes_it_profile_conflict() {
    let r = rig();
    let other = sample_profile("Demo Alpha", &[C4]).unwrap();
    let runs = Arc::new(AtomicUsize::new(0));
    arm_other_writer(&r, &other, &runs);

    let (result, act_runs) = edit(&r, Some(&alpha()), C1, &set(C1), Err(unavailable("act")));
    match result {
        Err(PaneError::ProfileConflict { what }) => {
            assert!(what.contains("Demo Alpha"), "names the profile: {what}");
        }
        unexpected => panic!("expected profile-conflict, got {unexpected:?}"),
    }
    assert_eq!(act_runs, 1);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    let now = stored(&r, &alpha());
    assert_eq!(now.generation, 3, "edit at 2, the other writer at 3");
    assert_eq!(now.panes, other.panes);

    // One-shot: the next failed act restores normally (two more writes).
    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Err(unavailable("act")));
    assert_eq!(result, Err(unavailable("act")));
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "the hook is dropped after it ran"
    );
    let now = stored(&r, &alpha());
    assert_eq!(now.generation, 5);
    assert_eq!(now.panes, other.panes);
}

#[test]
fn a_hook_stays_armed_until_a_failed_act_of_an_edit_with_a_profile() {
    let r = rig();
    let other = sample_profile("Demo Alpha", &[C4]).unwrap();
    let runs = Arc::new(AtomicUsize::new(0));
    arm_other_writer(&r, &other, &runs);

    // A failing act without a profile never reaches the restore.
    let (result, _) = edit(&r, None, C1, &set(C1), Err(unavailable("act")));
    assert_eq!(result, Err(unavailable("act")));
    // Neither does a first write that conflicts.
    r.profiles
        .faults()
        .fail_next(ProfileStoreOp::CasPut, PaneError::Conflict);
    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert_eq!(code_of(result).as_deref(), Some("generation-conflict"));
    // Nor a succeeding act.
    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Ok(()));
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(runs.load(Ordering::SeqCst), 0);

    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Err(unavailable("act")));
    assert_eq!(code_of(result).as_deref(), Some("profile-conflict"));
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[test]
fn arming_the_hook_again_replaces_an_unused_one() {
    let r = rig();
    let other = sample_profile("Demo Alpha", &[C4]).unwrap();
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    arm_other_writer(&r, &other, &first);
    arm_other_writer(&r, &other, &second);

    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Err(unavailable("act")));
    assert_eq!(code_of(result).as_deref(), Some("profile-conflict"));
    assert_eq!(first.load(Ordering::SeqCst), 0);
    assert_eq!(second.load(Ordering::SeqCst), 1);
}

#[test]
fn a_hook_may_arm_the_hook_again_without_deadlocking() {
    // The scope's own lock must not be held while the hook runs. A deadlock would hang
    // the test, so the edit runs on a thread the test gives up on after a bound.
    let r = rig();
    let again = r.scope.clone();
    r.scope
        .before_next_restore(move || again.before_next_restore(|| {}));

    let scope = r.scope.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut act = || Err(unavailable("act"));
        let result = scope.edit_spec(Some(&alpha()), &pname(C1), &set(C1), &mut act);
        sender.send(result).ok();
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("edit_spec returned: the hook ran outside the scope's lock");
    assert_eq!(result, Err(unavailable("act")));
}

#[test]
fn resolve_of_a_pane_with_no_record_is_pane_not_in_profile() {
    let r = rig();
    let result = r.scope.resolve(&alpha(), Some(&pname("demo-c9r1")));
    assert_eq!(code_of(result).as_deref(), Some("pane-not-in-profile"));
}

#[test]
fn the_refusals_name_the_pane_and_the_profile() {
    let r = rig();
    match r.scope.resolve(&alpha(), Some(&pname(C3))) {
        Err(PaneError::PaneNotInProfile { what }) => {
            assert!(what.contains(C3) && what.contains("Demo Alpha"), "{what}");
        }
        other => panic!("expected pane-not-in-profile, got {other:?}"),
    }
    let (result, _) = edit(&r, Some(&alpha()), C3, &set(C3), Ok(()));
    match result {
        Err(PaneError::PaneInOtherProfile { what }) => {
            assert!(what.contains(C3) && what.contains("Demo Alpha"), "{what}");
        }
        other => panic!("expected pane-in-other-profile, got {other:?}"),
    }
}

#[test]
fn the_log_carries_the_scopes_actor() {
    let r = rig();
    let (result, _) = edit(&r, Some(&alpha()), C1, &set(C1), Err(unavailable("act")));
    assert_eq!(result, Err(unavailable("act")));

    let log = r.profiles.log(&alpha()).unwrap();
    assert_eq!(log.len(), 3, "created, the edit, its reversal: {log:?}");
    assert_eq!(log[0].actor, actor("seeder"));
    for entry in &log[1..] {
        assert!(
            matches!(entry.change, ProfileChange::Updated { .. }),
            "{entry:?}"
        );
        assert_eq!(entry.actor, actor("scope-actor"));
    }
}

#[test]
fn the_scope_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FakeProfileScope>();
}
