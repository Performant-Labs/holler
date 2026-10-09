#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
//! The fake pane registry passes its own conformance suite, the suite lists the
//! cases it documents, and a registry that breaks one rule fails the case that
//! pins that rule (the mutation check; #638, slice a).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The
//! suite is the unit under test here: a suite that cannot tell a broken store from
//! a good one proves nothing about the hub's registry later.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Watch};
use holler_pane_testkit::conformance::pane_store::{pane_store_cases, run_pane_store_conformance};
use holler_pane_testkit::pane_store::FakePaneStore;

/// The 19 case ids of the brief's table, in order.
const DOCUMENTED_CASES: [&str; 19] = [
    "get-missing-is-none",
    "list-empty",
    "create-at-zero-stored-at-one",
    "submitted-generation-ignored",
    "create-over-existing-conflicts",
    "update-bumps-by-one",
    "stale-generation-conflicts",
    "expected-ahead-conflicts",
    "list-sorted-by-name",
    "delete-at-current-generation",
    "delete-stale-conflicts",
    "delete-missing-is-pane-not-found",
    "recreate-after-delete-starts-at-one",
    "watch-from-zero-yields-current-state",
    "watch-follows-each-change",
    "watch-resumes-without-gap-or-repeat",
    "watch-idle-is-ok-none-and-stays-usable",
    "failed-write-changes-nothing",
    "pane-in-other-profile",
];

#[test]
fn the_fake_passes_the_pane_store_conformance_suite() {
    let result = run_pane_store_conformance(|| (FakePaneStore::new(), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_suite_runs_the_documented_cases() {
    assert_eq!(pane_store_cases(), DOCUMENTED_CASES.to_vec());
}

// --- the guard lives for the case ---

/// A guard whose `Drop` records that the case is over.
struct CaseGuard {
    over: Arc<AtomicBool>,
    dropped: Arc<AtomicUsize>,
}

impl Drop for CaseGuard {
    fn drop(&mut self) {
        self.over.store(true, Ordering::SeqCst);
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

/// A store that counts every call made after its case's guard was dropped.
struct GuardWatcher {
    inner: FakePaneStore,
    over: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
    after_drop: Arc<AtomicUsize>,
}

impl GuardWatcher {
    fn note(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.over.load(Ordering::SeqCst) {
            self.after_drop.fetch_add(1, Ordering::SeqCst);
        }
    }
}

impl PaneStore for GuardWatcher {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.note();
        self.inner.get(name)
    }
    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.note();
        self.inner.list()
    }
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.note();
        self.inner.cas_put(pane, expected_generation)
    }
    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.note();
        self.inner.delete(name, expected_generation)
    }
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.note();
        self.inner.watch(since)
    }
}

#[test]
fn the_guard_lives_for_the_case() {
    let calls = Arc::new(AtomicUsize::new(0));
    let after_drop = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let built = Arc::new(AtomicUsize::new(0));

    let result = run_pane_store_conformance(|| {
        built.fetch_add(1, Ordering::SeqCst);
        let over = Arc::new(AtomicBool::new(false));
        let store = GuardWatcher {
            inner: FakePaneStore::new(),
            over: Arc::clone(&over),
            calls: Arc::clone(&calls),
            after_drop: Arc::clone(&after_drop),
        };
        let guard = CaseGuard {
            over,
            dropped: Arc::clone(&dropped),
        };
        (store, guard)
    });

    assert_eq!(result, Ok(()));
    assert_eq!(
        built.load(Ordering::SeqCst),
        DOCUMENTED_CASES.len(),
        "`fresh` is called once per case"
    );
    assert!(
        calls.load(Ordering::SeqCst) > 0,
        "the suite drove the store"
    );
    assert_eq!(
        after_drop.load(Ordering::SeqCst),
        0,
        "no call may reach the store after its guard was dropped"
    );
    assert_eq!(
        dropped.load(Ordering::SeqCst),
        DOCUMENTED_CASES.len(),
        "every guard is dropped by the end of the run"
    );
}

// --- mutation check: a broken store fails, on the named case ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct store: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `cas_put` writes at whatever the current generation is.
    NoCas,
    /// `cas_put` returns the generation the caller submitted.
    KeepsSubmittedGeneration,
    /// `delete` of a missing name with `expected != 0` is `generation-conflict`.
    GenerationBeforeExistence,
    /// `watch(since)` resumes one change early.
    RepeatsOnResume,
    /// A watch ends when idle instead of yielding `Ok(None)`.
    EndsStreamWhenIdle,
    /// `cas_put` lets a pane change profile (clears the stored profile first).
    LetsPaneChangeProfile,
}

struct Mutant {
    inner: FakePaneStore,
    broken: Break,
}

impl Mutant {
    fn new(broken: Break) -> Self {
        Self {
            inner: FakePaneStore::new(),
            broken,
        }
    }

    fn current_generation(&self, name: &PaneName) -> Result<u64, PaneError> {
        Ok(self.inner.get(name)?.map_or(0, |p| p.generation))
    }
}

impl PaneStore for Mutant {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.inner.list()
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        match self.broken {
            Break::NoCas => {
                let current = self.current_generation(&pane.name)?;
                self.inner.cas_put(pane, current)
            }
            Break::KeepsSubmittedGeneration => {
                let stored = self.inner.cas_put(pane, expected_generation)?;
                Ok(Pane {
                    generation: pane.generation,
                    ..stored
                })
            }
            Break::LetsPaneChangeProfile => match self.inner.get(&pane.name)? {
                Some(stored) if stored.profile.is_some() => {
                    let cleared = Pane {
                        profile: None,
                        ..stored
                    };
                    let bumped = self.inner.concurrent_put(&cleared)?;
                    self.inner.cas_put(pane, bumped.generation)
                }
                _ => self.inner.cas_put(pane, expected_generation),
            },
            _ => self.inner.cas_put(pane, expected_generation),
        }
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        if self.broken == Break::GenerationBeforeExistence
            && expected_generation != 0
            && self.inner.get(name)?.is_none()
        {
            return Err(PaneError::Conflict);
        }
        self.inner.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        match self.broken {
            Break::RepeatsOnResume => self.inner.watch(Cursor(since.0.saturating_sub(1))),
            Break::EndsStreamWhenIdle => {
                let inner = self.inner.watch(since)?;
                Ok(Box::new(inner.map_while(|item| match item {
                    Ok(None) => None,
                    other => Some(other),
                })))
            }
            _ => self.inner.watch(since),
        }
    }
}

/// The suite must reject `broken` and name `case` among its failures.
fn assert_suite_fails_on(broken: Break, case: &str) {
    let result = run_pane_store_conformance(|| (Mutant::new(broken), ()));
    let failures = result.expect_err("a broken store must not pass the suite");
    assert!(
        failures.iter().any(|f| f.case == case),
        "expected case `{case}` among the failures of {broken:?}, got: {failures:?}"
    );
    assert!(
        failures.iter().all(|f| !f.detail.is_empty()),
        "every failure says why: {failures:?}"
    );
}

#[test]
fn the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone() {
    let result = run_pane_store_conformance(|| (Mutant::new(Break::Nothing), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn a_store_without_cas_fails() {
    assert_suite_fails_on(Break::NoCas, "stale-generation-conflicts");
}

#[test]
fn a_store_that_keeps_the_submitted_generation_fails() {
    assert_suite_fails_on(
        Break::KeepsSubmittedGeneration,
        "submitted-generation-ignored",
    );
}

#[test]
fn a_store_that_checks_generation_before_existence_fails() {
    assert_suite_fails_on(
        Break::GenerationBeforeExistence,
        "delete-missing-is-pane-not-found",
    );
}

#[test]
fn a_store_that_repeats_on_resume_fails() {
    assert_suite_fails_on(
        Break::RepeatsOnResume,
        "watch-resumes-without-gap-or-repeat",
    );
}

#[test]
fn a_store_that_ends_the_stream_when_idle_fails() {
    assert_suite_fails_on(
        Break::EndsStreamWhenIdle,
        "watch-idle-is-ok-none-and-stays-usable",
    );
}

#[test]
fn a_store_that_lets_a_pane_change_profile_fails() {
    assert_suite_fails_on(Break::LetsPaneChangeProfile, "pane-in-other-profile");
}
