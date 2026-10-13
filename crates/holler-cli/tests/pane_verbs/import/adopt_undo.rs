//! The real run's undo under the switch (#831, AC10): all of the plan or nothing, in two shapes.
//!
//! - **Create** (a new `fleet`): the profile first, then the records; a failure deletes the
//!   failed record if it landed, every record written, then the profile. Unchanged by the
//!   story, here with an adopted record in the grid.
//! - **Extend** (an existing `fleet`, with specs to add): the records first, then the one
//!   `fleet` `cas_put`, so the undo is record deletes only and `fleet` is left as it was. A
//!   failed record write, a failed or conflicting write of `fleet`, and a concurrent change of
//!   `fleet` between the read and the write each leave no new record. An undo step that fails is
//!   `profile-conflict`, whose text says what remains and how to look at it, and never names a
//!   command that deletes `fleet` or acts on a live pane (`holler profile delete`, `holler pane
//!   close`).
//!
//! The failed record is deleted by the undo only when it is the one this run planned: another
//! actor's record of the same name, a member of `fleet`, survives.

use std::sync::atomic::{AtomicUsize, Ordering};

use holler_cli::output::Format;
use holler_pane::pane::PaneRole;
use holler_pane::{Actor, Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Watch};
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_pane_testkit::profile_store::ProfileStoreOp;

use super::adopt_rig::{AdoptRig, ADOPT, ANTHROPIC_MODEL, CLAUDE_SESSION};
use super::rig::{error_of, FLEET};
use super::seams::{NthCasPut, NthPut};
use crate::doctor::rig::Seed;

const A: &str = "hj-c1r1";
const B: &str = "hj-c1r2";

/// `A` (OpenCode) and `B` (Claude Code): the records are written in name order, `A` then `B`.
fn grid() -> AdoptRig {
    let rig = AdoptRig::new(&[Seed::new(A, 1, 1), Seed::new(B, 2, 1)]);
    rig.write_env(B, ANTHROPIC_MODEL, &[]);
    rig.typed_claude(B, CLAUDE_SESSION);
    rig
}

fn unavailable() -> PaneError {
    PaneError::Unavailable {
        what: "made-up".to_owned(),
    }
}

/// A pane store over `inner` that runs a hook before or after the N-th `cas_put` made through
/// it (1 is the first), then makes the write: how a test has another actor change the world at
/// an exact point of the run. Every other call passes through.
struct Hooked<'a> {
    inner: &'a dyn PaneStore,
    count: AtomicUsize,
    before: Vec<(usize, Box<dyn Fn() + Send + Sync + 'a>)>,
    after: Vec<(usize, Box<dyn Fn() + Send + Sync + 'a>)>,
}

impl<'a> Hooked<'a> {
    fn new(inner: &'a dyn PaneStore) -> Self {
        Self {
            inner,
            count: AtomicUsize::new(0),
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    fn before_put(mut self, n: usize, hook: impl Fn() + Send + Sync + 'a) -> Self {
        self.before.push((n, Box::new(hook)));
        self
    }

    fn after_put(mut self, n: usize, hook: impl Fn() + Send + Sync + 'a) -> Self {
        self.after.push((n, Box::new(hook)));
        self
    }
}

impl PaneStore for Hooked<'_> {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.inner.list()
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        let n = self.count.fetch_add(1, Ordering::SeqCst) + 1;
        for (at, hook) in &self.before {
            if *at == n {
                hook();
            }
        }
        let result = self.inner.cas_put(pane, expected_generation);
        for (at, hook) in &self.after {
            if *at == n {
                hook();
            }
        }
        result
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.inner.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.inner.watch(since)
    }
}

/// The run failed with `code` (exit 1), saying nothing was kept and that `fleet` itself is as it
/// was; returns the message.
fn assert_undone(run: &crate::verb_harness::Outcome, code: &str) -> String {
    assert_eq!(run.code, 1, "{run:?}");
    let (got, message) = error_of(run);
    assert_eq!(got, code, "{message}");
    assert!(message.contains("nothing was kept"), "{message}");
    assert!(
        !message.contains("profile delete") && !message.contains("pane close"),
        "an undone run names no remedy: {message}"
    );
    message
}

// --- extending an existing fleet ------------------------------------------------------------

/// The first record's write fails: nothing was written, so nothing is deleted, and `fleet` was
/// never touched.
#[test]
fn an_extension_whose_first_record_write_fails_keeps_nothing() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    let store = NthCasPut::new(
        &rig.world.panes,
        vec![(1, NthPut::Fail(PaneError::Conflict))],
    );
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    let message = assert_undone(&run, "generation-conflict");
    assert!(message.contains("was not changed"), "{message}");
    assert!(rig.records().is_empty(), "no record");
    assert_eq!(rig.fleet_now(), before, "fleet is as it was");
}

/// The second record's write fails: the first is deleted, newest first, and `fleet` is as it was
/// (the profile is never deleted in an extension).
#[test]
fn an_extension_whose_second_record_write_fails_deletes_the_first_only() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    let mark = rig.mark_all();
    let store = NthCasPut::new(
        &rig.world.panes,
        vec![(2, NthPut::Fail(PaneError::Conflict))],
    );
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    let message = assert_undone(&run, "generation-conflict");
    assert!(message.contains("was not changed"), "{message}");
    let calls = rig.since(&mark).world;
    assert!(
        !calls.profiles.iter().any(|op| matches!(
            op,
            ProfileStoreOp::CasPut | ProfileStoreOp::Delete | ProfileStoreOp::Rename
        )),
        "fleet is not written, deleted or renamed: {:?}",
        calls.profiles
    );
    assert!(rig.records().is_empty(), "no record");
    assert_eq!(rig.fleet_now(), before, "fleet is as it was");
}

/// A record whose write landed and answered `timeout` is found by the undo and deleted too.
#[test]
fn an_extension_record_that_landed_and_timed_out_is_deleted_too() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    let timeout = PaneError::Timeout {
        op: "pane_store.cas_put".to_owned(),
    };
    let store = NthCasPut::new(&rig.world.panes, vec![(2, NthPut::ApplyThenFail(timeout))]);
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    assert_undone(&run, "timeout");
    assert!(rig.records().is_empty(), "both records are gone");
    assert_eq!(rig.fleet_now(), before);
}

/// The one write of `fleet` fails: every record written is deleted, newest first (the undo's
/// deletes follow the two writes in the pane store's log), and `fleet` is as it was.
#[test]
fn a_failed_fleet_write_deletes_every_record_and_leaves_fleet_as_it_was() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    rig.world
        .profiles
        .faults()
        .fail_next(ProfileStoreOp::CasPut, PaneError::Conflict);
    let mark = rig.mark_all();
    let run = rig.run_routed(&[ADOPT], Format::Json);
    let message = assert_undone(&run, "generation-conflict");
    assert!(message.contains("was not changed"), "{message}");
    let writes: Vec<PaneStoreOp> = rig
        .since(&mark)
        .world
        .panes
        .into_iter()
        .filter(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete))
        .collect();
    assert_eq!(
        writes,
        [
            PaneStoreOp::CasPut,
            PaneStoreOp::CasPut,
            PaneStoreOp::Delete,
            PaneStoreOp::Delete
        ],
        "the records first, then (after the fleet write failed) their deletes"
    );
    assert!(rig.records().is_empty(), "no record");
    assert_eq!(rig.fleet_now(), before, "fleet is as it was");
}

/// Another actor changes `fleet` between the run's read and its write (after the last record is
/// written): the write is a real `generation-conflict`, the records are deleted, and `fleet` is
/// what the other actor made it, not reverted.
#[test]
fn a_concurrent_change_of_fleet_is_a_conflict_that_keeps_the_other_actors_version() {
    let rig = grid();
    rig.seed_fleet(&[]);
    let other = Actor::parse("another-actor").expect("an actor");
    let changed = sample_profile(FLEET, &["demo-other"]).expect("a profile");
    let profiles = &rig.world.profiles;
    let store = Hooked::new(&*rig.world.panes).after_put(2, || {
        profiles
            .concurrent_put(&changed, &other)
            .expect("the other actor writes fleet");
    });
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    assert_undone(&run, "generation-conflict");
    assert!(rig.records().is_empty(), "no record");
    let now = rig.fleet_now();
    assert_eq!(
        now.generation, 2,
        "the other actor's write, and none of the run's"
    );
    let names: Vec<&str> = now.panes.iter().map(|s| s.pane.as_str()).collect();
    assert_eq!(names, ["demo-other"], "the other actor's version stays");
}

/// The failed record is deleted by the undo only if it is the one this run planned. Another
/// actor creates a record of the same name, a member of `fleet`, just before the run's write of
/// it: the write fails `generation-conflict`, and the undo must not delete that record.
#[test]
fn the_undo_leaves_another_actors_record_of_the_same_name_alone() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    let mut theirs = rig.expected_record(B, PaneRole::Agent);
    theirs.model.effort = "low".to_owned();
    let panes: &FakePaneStore = &rig.world.panes;
    let store = Hooked::new(panes).before_put(2, || {
        panes
            .concurrent_put(&theirs)
            .expect("the other actor's record");
    });
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    assert_undone(&run, "generation-conflict");
    let survivor = rig
        .record(B)
        .expect("the other actor's record survives the undo");
    assert_eq!(
        survivor.model.effort, "low",
        "it is theirs, not the planned one"
    );
    assert_eq!(survivor.profile.as_ref().map(|p| p.as_str()), Some(FLEET));
    assert!(rig.record(A).is_none(), "this run's own record is deleted");
    assert_eq!(rig.fleet_now(), before, "fleet is as it was");
}

/// An undo step fails: `profile-conflict`, whose text names the failing write, the records that
/// remain and the two commands that only read (`holler pane list`, `holler profile show
/// 'fleet'`), and says a re-run skips the kept records. It never contains `profile delete` (an
/// extension has no profile to delete) nor `pane close` (which would act on a live pane).
#[test]
fn a_failed_undo_step_of_an_extension_names_what_remains_and_no_deleting_command() {
    let rig = grid();
    let before = rig.seed_fleet(&[]);
    rig.world
        .panes
        .faults()
        .fail_next(PaneStoreOp::Delete, unavailable());
    let store = NthCasPut::new(
        &rig.world.panes,
        vec![(2, NthPut::Fail(PaneError::Conflict))],
    );
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "profile-conflict", "{message}");
    assert!(!message.contains("profile delete"), "{message}");
    assert!(!message.contains("pane close"), "{message}");
    assert!(
        message.contains(A),
        "names the record that remains: {message}"
    );
    for command in ["holler pane list", "holler profile show 'fleet'"] {
        assert!(message.contains(command), "{command:?} in {message}");
    }
    assert!(
        message.contains("already-imported") || message.contains("already imported"),
        "a re-run skips the kept records: {message}"
    );
    assert!(
        rig.record(A).is_some(),
        "the record whose delete failed remains"
    );
    assert!(rig.record(B).is_none());
    assert_eq!(rig.fleet_now(), before, "fleet is as it was");
}

// --- creating a new fleet (unchanged by the story) --------------------------------------------

/// A new `fleet` with an adopted record in it: the second write fails, the first record and the
/// profile are deleted, as before.
#[test]
fn a_creation_with_an_adopted_pane_is_undone_with_the_profile() {
    let rig = grid();
    let store = NthCasPut::new(
        &rig.world.panes,
        vec![(2, NthPut::Fail(PaneError::Conflict))],
    );
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    let message = assert_undone(&run, "generation-conflict");
    assert!(message.contains("fleet"), "{message}");
    assert!(rig.records().is_empty(), "no record");
    assert!(rig.fleet().is_none(), "the profile fleet is deleted");
}

/// ...and when the profile's delete fails, the remedy still names the command that removes the
/// `fleet` this run created.
#[test]
fn a_creation_whose_profile_delete_fails_still_names_profile_delete() {
    let rig = grid();
    rig.world
        .profiles
        .faults()
        .fail_next(ProfileStoreOp::Delete, unavailable());
    let store = NthCasPut::new(
        &rig.world.panes,
        vec![(2, NthPut::Fail(PaneError::Conflict))],
    );
    let run = rig.run_routed_store(&store, &[ADOPT], Format::Json);
    let (code, message) = error_of(&run);
    assert_eq!(code, "profile-conflict", "{message}");
    assert!(
        message.contains("holler profile delete 'fleet'"),
        "{message}"
    );
    assert!(!message.contains("pane close"), "{message}");
}
