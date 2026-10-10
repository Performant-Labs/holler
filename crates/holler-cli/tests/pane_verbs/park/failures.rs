//! AC 8: a failed store call fails the run with the pane named, and a profile-wide run
//! stops at its first failed write and says what it changed and what it did not reach
//! (brief Decision 7). The checks take the verb, so `unpark.rs` runs the same ones (AC 8d).

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Watch};
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};

use super::rig::{
    assert_failure, assert_failure_message, held, member, pane, profile, run_both, Both, Rig, Verb,
};

/// `PaneError::Conflict`'s text.
const CONFLICT: &str =
    "the record changed since it was read (generation conflict); read it again and retry";

/// A pane store that delegates to the fake and answers `generation-conflict`, without
/// writing, on its `fail_on`-th `cas_put` (counting from 1; 0 never fails).
struct FailNthCasPut {
    inner: Arc<FakePaneStore>,
    fail_on: AtomicUsize,
    seen: AtomicUsize,
}

impl FailNthCasPut {
    fn seen(&self) -> usize {
        self.seen.load(Ordering::SeqCst)
    }

    fn disarm(&self) {
        self.fail_on.store(0, Ordering::SeqCst);
    }
}

impl PaneStore for FailNthCasPut {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.inner.list()
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        let call = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
        if call == self.fail_on.load(Ordering::SeqCst) {
            return Err(PaneError::Conflict);
        }
        self.inner.cas_put(pane, expected_generation)
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.inner.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.inner.watch(since)
    }
}

/// `argv` in both formats, each over a fresh rig of `panes` and `profiles` whose pane
/// store is a [`FailNthCasPut`] failing on `fail_on`; returns the run and each rig's
/// wrapper (text first).
fn run_failing(
    panes: &dyn Fn() -> Vec<Pane>,
    profiles: &[(&str, &[&str])],
    fail_on: usize,
    argv: &[&str],
) -> (Both, Vec<Arc<FailNthCasPut>>) {
    let wrappers = RefCell::new(Vec::new());
    let seed = || {
        Rig::wrapped(
            panes(),
            profiles
                .iter()
                .map(|(name, members)| profile(name, members)),
            |fake| {
                let wrapper = Arc::new(FailNthCasPut {
                    inner: fake,
                    fail_on: AtomicUsize::new(fail_on),
                    seen: AtomicUsize::new(0),
                });
                wrappers.borrow_mut().push(wrapper.clone());
                wrapper
            },
        )
    };
    let both = run_both(seed, argv);
    (both, wrappers.into_inner())
}

/// The three members of `Demo Alpha`, each starting from `first`'s hold for `demo-c1r1`
/// and `verb.changeable()` for the other two.
fn three_members(verb: Verb, first: holler_pane::pane::Hold) -> Vec<Pane> {
    vec![
        held(member("demo-c1r1", "Demo Alpha"), first),
        held(member("demo-c2r1", "Demo Alpha"), verb.changeable()),
        held(member("demo-c3r1", "Demo Alpha"), verb.changeable()),
    ]
}

const ALPHA: (&str, &[&str]) = ("Demo Alpha", &["demo-c1r1", "demo-c2r1", "demo-c3r1"]);

/// For each rig of `both`: the panes of `done` are in the verb's state at generation 2,
/// every other seeded pane is unchanged.
fn assert_changed_only(both: &Both, verb: Verb, done: &[&str]) {
    for rig in [&both.text_rig, &both.json_rig] {
        for seed in &rig.seeded {
            let record = rig.record(seed.name.as_str());
            if done.contains(&seed.name.as_str()) {
                assert!(verb.is_done(&record.hold), "{verb:?} {record:?}");
                assert_eq!(record.generation, 2, "{}", seed.name);
            } else {
                assert_eq!(record, *seed, "{} is unchanged", seed.name);
            }
        }
    }
}

/// AC 8 for `verb` (8a-8c for park; 8d is the same for unpark).
pub(crate) fn assert_failures_name_the_pane_and_stop(verb: Verb) {
    one_pane_failures(verb);
    profile_wide_runs_stop_at_the_first_failed_write(verb);
    the_suffix_edges(verb);
}

/// (a) and (b): one named pane; the error names it, with no suffix, and nothing changes.
fn one_pane_failures(verb: Verb) {
    let one = || Rig::new([held(pane("demo-c1r1"), verb.changeable())], []);
    let argv = verb.argv(&["demo-c1r1"]);

    let conflict = || {
        let rig = one();
        rig.panes
            .faults()
            .fail_next(PaneStoreOp::CasPut, PaneError::Conflict);
        rig
    };
    let both = run_both(conflict, &argv);
    assert_failure_message(
        &both,
        "generation-conflict",
        1,
        &format!("demo-c1r1: {CONFLICT}"),
    );
    for rig in [&both.text_rig, &both.json_rig] {
        rig.assert_all_unchanged();
    }

    let unreachable = || {
        let rig = one();
        rig.panes
            .faults()
            .set(Some(Fault::Fail(PaneError::Unavailable {
                what: "pane store".into(),
            })));
        rig
    };
    let both = run_both(unreachable, &argv);
    assert_failure(&both, "unavailable", 1);
    for rig in [&both.text_rig, &both.json_rig] {
        rig.panes.faults().set(None);
        rig.assert_all_unchanged();
    }
}

/// (c): a profile-wide run stops at its first failed write; the suffix names what this
/// run changed before it and what it did not reach; the rerun finishes the job.
fn profile_wide_runs_stop_at_the_first_failed_write(verb: Verb) {
    let done = verb.done();
    let argv = verb.argv(&["--profile", "Demo Alpha"]);
    let fresh = || three_members(verb, verb.changeable());
    let cases: [(usize, &str, &[&str]); 3] = [
        (
            1,
            "demo-c1r1: {C}; {D} by this run before it: none; not reached: demo-c2r1, demo-c3r1",
            &[],
        ),
        (
            2,
            "demo-c2r1: {C}; {D} by this run before it: demo-c1r1; not reached: demo-c3r1",
            &["demo-c1r1"],
        ),
        (
            3,
            "demo-c3r1: {C}; {D} by this run before it: demo-c1r1, demo-c2r1; not reached: none",
            &["demo-c1r1", "demo-c2r1"],
        ),
    ];
    for (fail_on, message, changed) in cases {
        let message = message.replace("{C}", CONFLICT).replace("{D}", done);
        let (both, wrappers) = run_failing(&fresh, &[ALPHA], fail_on, &argv);
        assert_failure_message(&both, "generation-conflict", 1, &message);
        assert_changed_only(&both, verb, changed);
        for wrapper in &wrappers {
            assert_eq!(
                wrapper.seen(),
                fail_on,
                "{verb:?}: no write after the failed one"
            );
        }
        if fail_on == 2 {
            wrappers[0].disarm();
            assert_the_rerun_finishes(verb, &both.text_rig, &argv);
        }
    }
}

/// The rerun after a failure on `demo-c2r1`, with no fault: it leaves `demo-c1r1` (which
/// the first run changed) alone and changes the other two.
fn assert_the_rerun_finishes(verb: Verb, rig: &Rig, argv: &[&str]) {
    let rerun = rig.run(argv, Format::Text);
    assert_eq!(rerun.code, 0, "{rerun:?}");
    assert_eq!(
        rerun.out,
        format!(
            "{}\n{}\n{}\n",
            verb.done_line_again("demo-c1r1"),
            verb.done_line("demo-c2r1"),
            verb.done_line("demo-c3r1")
        )
    );
    assert_eq!(rig.record("demo-c1r1").generation, 2, "not written again");
    for name in ["demo-c1r1", "demo-c2r1", "demo-c3r1"] {
        assert!(verb.is_done(&rig.record(name).hold), "{name}");
    }
    rig.assert_no_live_call_and_no_profile_write();
}

/// The edges of the suffix (Decision 7): a pane left alone is not listed as changed, and
/// a one-member profile gets the bare message.
fn the_suffix_edges(verb: Verb) {
    let done = verb.done();
    let first_done = || three_members(verb, verb.already());
    let argv = verb.argv(&["--profile", "Demo Alpha"]);
    let (both, _) = run_failing(&first_done, &[ALPHA], 1, &argv);
    assert_failure_message(
        &both,
        "generation-conflict",
        1,
        &format!(
            "demo-c2r1: {CONFLICT}; {done} by this run before it: none; not reached: demo-c3r1"
        ),
    );

    let solo = || vec![held(member("demo-c5r1", "Demo Solo"), verb.changeable())];
    let (both, _) = run_failing(
        &solo,
        &[("Demo Solo", &["demo-c5r1"])],
        1,
        &verb.argv(&["--profile", "Demo Solo"]),
    );
    assert_failure_message(
        &both,
        "generation-conflict",
        1,
        &format!("demo-c5r1: {CONFLICT}"),
    );
}

#[test]
fn park_failures_name_the_pane_and_stop() {
    assert_failures_name_the_pane_and_stop(Verb::Park);
}
