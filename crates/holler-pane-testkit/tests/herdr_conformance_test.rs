#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #683
//! The fake Herdr passes its own conformance suite in both placements, the suite
//! lists the cases it documents, and a Herdr that breaks one rule fails the case that
//! pins that rule (the mutation check; #683, slice d).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The suite
//! is the unit under test here: a suite that cannot tell a transposing adapter from a
//! good one proves nothing about the real adapter later.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use holler_pane::{
    GridPos, HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId,
};
use holler_pane_testkit::conformance::herdr::{herdr_cases, run_herdr_conformance, HerdrFixture};
use holler_pane_testkit::herdr::{FakeHerdr, HerdrVersion, Placement};

const SESSION: &str = "scratch";
const WORKSPACE: &str = "scratch";

/// The 11 case ids of the brief's table, in order.
const DOCUMENTED_CASES: [&str; 11] = [
    "ensure-r2c1-reads-back-as-r2c1",
    "ensure-r1c2-is-grid-out-of-range",
    "ensure-is-idempotent",
    "ensure-never-moves-another-pane",
    "closed-id-is-never-reused",
    "ids-unique-and-stable-when-a-sibling-closes",
    "close-unknown-is-pane-not-found",
    "close-twice-is-pane-not-found",
    "calls-on-a-closed-pane-are-pane-not-found",
    "read-returns-at-most-max-lines",
    "version-is-reported",
];

/// The fake serving a scratch session whose workspace is 2 rows by 1 column.
fn scratch_fake() -> FakeHerdr {
    FakeHerdr::new(SESSION)
        .with_workspace(WORKSPACE, 2, 1)
        .expect("workspace")
}

fn fixture<H>(port: H) -> HerdrFixture<H> {
    HerdrFixture {
        port,
        session: SESSION.to_owned(),
        workspace: WORKSPACE.to_owned(),
    }
}

#[test]
fn the_fake_passes_the_herdr_conformance_suite() {
    let result = run_herdr_conformance(|| (fixture(scratch_fake()), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_fake_in_split_only_mode_passes_the_suite() {
    let result = run_herdr_conformance(|| {
        let port = scratch_fake();
        port.set_placement(Placement::SplitOnly);
        (fixture(port), ())
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn the_suite_runs_the_documented_cases() {
    assert_eq!(herdr_cases(), DOCUMENTED_CASES.to_vec());
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

/// A port that counts every call made after its case's guard was dropped.
struct GuardWatcher {
    inner: FakeHerdr,
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

impl HerdrPort for GuardWatcher {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        self.note();
        self.inner.ensure_pane(spec)
    }
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        self.note();
        self.inner.send_text(pane, text)
    }
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        self.note();
        self.inner.send_keys(pane, keys)
    }
    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        self.note();
        self.inner.read(pane, max_lines)
    }
    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.note();
        self.inner.close(pane)
    }
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        self.note();
        self.inner.snapshot()
    }
    fn version(&self) -> Result<String, PaneError> {
        self.note();
        self.inner.version()
    }
}

#[test]
fn the_suite_builds_a_fresh_fixture_per_case() {
    let calls = Arc::new(AtomicUsize::new(0));
    let after_drop = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let built = Arc::new(AtomicUsize::new(0));

    let result = run_herdr_conformance(|| {
        built.fetch_add(1, Ordering::SeqCst);
        let over = Arc::new(AtomicBool::new(false));
        let port = GuardWatcher {
            inner: scratch_fake(),
            over: Arc::clone(&over),
            calls: Arc::clone(&calls),
            after_drop: Arc::clone(&after_drop),
        };
        let guard = CaseGuard {
            over,
            dropped: Arc::clone(&dropped),
        };
        (fixture(port), guard)
    });

    assert_eq!(result, Ok(()));
    assert_eq!(built.load(Ordering::SeqCst), DOCUMENTED_CASES.len());
    assert!(calls.load(Ordering::SeqCst) > 0, "the suite drove the port");
    assert_eq!(
        after_drop.load(Ordering::SeqCst),
        0,
        "no call may reach the port after its guard was dropped"
    );
    assert_eq!(
        dropped.load(Ordering::SeqCst),
        DOCUMENTED_CASES.len(),
        "every guard is dropped by the end of the run"
    );
}

// --- mutation check: a broken Herdr fails, on the named case ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct Herdr: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `ensure_pane` swaps row and column on the way in and back on the way out.
    Transposes,
    /// `snapshot` reports every pane at its transposed cell.
    ReadsBackTransposed,
    /// An occupied cell's pane is closed and a new one made.
    CreatesOnEveryEnsure,
    /// Making a pane first closes and re-ensures every other pane of the workspace.
    RebuildsTheWorkspace,
    /// The outward id is `"<workspace>:<grid>"`, mapped back through the snapshot.
    IdsFromPosition,
    /// The outward id is `"n<index in the snapshot>"`, mapped back by index.
    IdsFromSnapshotOrder,
    /// `close` of an unknown id is `Ok`.
    CloseIsIdempotent,
    /// A closed id still answers `send_text`, `send_keys` and `read`.
    ClosedPaneStillAnswers,
    /// `read` always asks for every line.
    ReadIgnoresMaxLines,
}

struct Mutant {
    inner: FakeHerdr,
    broken: Break,
    closed: Mutex<HashSet<PaneId>>,
}

fn swapped(grid: GridPos) -> GridPos {
    GridPos {
        row: grid.col,
        col: grid.row,
    }
}

impl Mutant {
    fn new(broken: Break) -> Self {
        Self {
            inner: scratch_fake(),
            broken,
            closed: Mutex::new(HashSet::new()),
        }
    }

    fn was_closed(&self, pane: &PaneId) -> bool {
        let closed = self.closed.lock().unwrap_or_else(PoisonError::into_inner);
        closed.contains(pane)
    }

    /// The id this Herdr shows the caller for `pane`, the fake's own `pane`.
    fn outward(&self, pane: HerdrPane) -> Result<HerdrPane, PaneError> {
        let pane_id = match self.broken {
            Break::IdsFromPosition => PaneId::new(format!("{}:{}", pane.workspace, pane.grid)),
            Break::IdsFromSnapshotOrder => {
                let at = self
                    .inner
                    .snapshot()?
                    .panes
                    .iter()
                    .position(|p| p.pane_id == pane.pane_id);
                PaneId::new(format!("n{}", at.unwrap_or(usize::MAX)))
            }
            _ => return Ok(pane),
        };
        Ok(HerdrPane { pane_id, ..pane })
    }

    /// The fake's id for an id this Herdr handed out; an id it never did stays as is,
    /// which the fake does not know either.
    fn inward(&self, outward: &PaneId) -> Result<PaneId, PaneError> {
        let panes = self.inner.snapshot()?.panes;
        let found = match self.broken {
            Break::IdsFromPosition => panes
                .into_iter()
                .find(|p| format!("{}:{}", p.workspace, p.grid) == outward.as_str()),
            Break::IdsFromSnapshotOrder => outward
                .as_str()
                .strip_prefix('n')
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| panes.into_iter().nth(n)),
            _ => None,
        };
        Ok(found.map_or_else(|| outward.clone(), |p| p.pane_id))
    }

    fn workspace_panes(&self) -> Result<Vec<HerdrPane>, PaneError> {
        Ok(self
            .inner
            .snapshot()?
            .panes
            .into_iter()
            .filter(|p| p.session == SESSION && p.workspace == WORKSPACE)
            .collect())
    }

    fn spec_at(grid: GridPos) -> HerdrSpec {
        HerdrSpec {
            session: SESSION.to_owned(),
            workspace: WORKSPACE.to_owned(),
            grid,
        }
    }

    /// Close and re-ensure every pane of the workspace but the one at `keep`.
    fn rebuild_others(&self, keep: GridPos) -> Result<(), PaneError> {
        for other in self
            .workspace_panes()?
            .into_iter()
            .filter(|p| p.grid != keep)
        {
            self.inner.close(&other.pane_id)?;
            self.inner.ensure_pane(&Self::spec_at(other.grid))?;
        }
        Ok(())
    }
}

impl HerdrPort for Mutant {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        let made = match self.broken {
            Break::Transposes => {
                let flipped = HerdrSpec {
                    grid: swapped(spec.grid),
                    ..spec.clone()
                };
                let pane = self.inner.ensure_pane(&flipped)?;
                HerdrPane {
                    grid: swapped(pane.grid),
                    ..pane
                }
            }
            Break::CreatesOnEveryEnsure => {
                if let Some(old) = self.workspace_panes()?.iter().find(|p| p.grid == spec.grid) {
                    self.inner.close(&old.pane_id)?;
                }
                self.inner.ensure_pane(spec)?
            }
            Break::RebuildsTheWorkspace => {
                self.rebuild_others(spec.grid)?;
                self.inner.ensure_pane(spec)?
            }
            _ => self.inner.ensure_pane(spec)?,
        };
        self.outward(made)
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        if self.broken == Break::ClosedPaneStillAnswers && self.was_closed(pane) {
            return Ok(());
        }
        self.inner.send_text(&self.inward(pane)?, text)
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        if self.broken == Break::ClosedPaneStillAnswers && self.was_closed(pane) {
            return Ok(());
        }
        self.inner.send_keys(&self.inward(pane)?, keys)
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        if self.broken == Break::ClosedPaneStillAnswers && self.was_closed(pane) {
            return Ok(String::new());
        }
        let wanted = if self.broken == Break::ReadIgnoresMaxLines {
            usize::MAX
        } else {
            max_lines
        };
        self.inner.read(&self.inward(pane)?, wanted)
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        let result = self.inner.close(&self.inward(pane)?);
        match (&result, self.broken) {
            (Err(PaneError::PaneNotFound { .. }), Break::CloseIsIdempotent) => Ok(()),
            (Ok(()), _) => {
                let mut closed = self.closed.lock().unwrap_or_else(PoisonError::into_inner);
                closed.insert(pane.clone());
                Ok(())
            }
            _ => result,
        }
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        let mut panes = Vec::new();
        for pane in self.inner.snapshot()?.panes {
            let pane = match self.broken {
                Break::ReadsBackTransposed => HerdrPane {
                    grid: swapped(pane.grid),
                    ..pane
                },
                _ => pane,
            };
            panes.push(self.outward(pane)?);
        }
        Ok(HerdrSnapshot { panes })
    }

    fn version(&self) -> Result<String, PaneError> {
        self.inner.version()
    }
}

/// The suite must reject `broken` and name every one of `cases` among its failures.
fn assert_suite_fails_on(broken: Break, cases: &[&str]) {
    let result = run_herdr_conformance(|| (fixture(Mutant::new(broken)), ()));
    let failures = result.expect_err("a broken Herdr must not pass the suite");
    for case in cases {
        assert!(
            failures.iter().any(|f| f.case == *case),
            "expected case `{case}` among the failures of {broken:?}, got: {failures:?}"
        );
    }
    assert!(
        failures.iter().all(|f| !f.detail.is_empty()),
        "every failure says why: {failures:?}"
    );
}

#[test]
fn the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone() {
    let result = run_herdr_conformance(|| (fixture(Mutant::new(Break::Nothing)), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn a_transposing_herdr_fails() {
    assert_suite_fails_on(
        Break::Transposes,
        &[
            "ensure-r1c2-is-grid-out-of-range",
            "ensure-r2c1-reads-back-as-r2c1",
        ],
    );
}

#[test]
fn a_herdr_that_reads_back_transposed_fails() {
    assert_suite_fails_on(
        Break::ReadsBackTransposed,
        &["ensure-r2c1-reads-back-as-r2c1"],
    );
}

#[test]
fn a_herdr_that_creates_on_every_ensure_fails() {
    assert_suite_fails_on(Break::CreatesOnEveryEnsure, &["ensure-is-idempotent"]);
}

#[test]
fn a_herdr_that_rebuilds_the_workspace_fails() {
    assert_suite_fails_on(
        Break::RebuildsTheWorkspace,
        &["ensure-never-moves-another-pane"],
    );
}

#[test]
fn a_herdr_with_position_ids_fails() {
    assert_suite_fails_on(Break::IdsFromPosition, &["closed-id-is-never-reused"]);
}

#[test]
fn a_herdr_that_renumbers_on_close_fails() {
    assert_suite_fails_on(
        Break::IdsFromSnapshotOrder,
        &["ids-unique-and-stable-when-a-sibling-closes"],
    );
}

#[test]
fn a_herdr_whose_close_is_idempotent_fails() {
    assert_suite_fails_on(
        Break::CloseIsIdempotent,
        &[
            "close-twice-is-pane-not-found",
            "close-unknown-is-pane-not-found",
        ],
    );
}

#[test]
fn a_herdr_whose_closed_panes_still_answer_fails() {
    assert_suite_fails_on(
        Break::ClosedPaneStillAnswers,
        &["calls-on-a-closed-pane-are-pane-not-found"],
    );
}

#[test]
fn a_herdr_that_ignores_max_lines_fails() {
    assert_suite_fails_on(
        Break::ReadIgnoresMaxLines,
        &["read-returns-at-most-max-lines"],
    );
}

#[test]
fn an_unsupported_herdr_fails_the_version_case() {
    let result = run_herdr_conformance(|| {
        let port = scratch_fake();
        port.set_version(HerdrVersion::Unsupported);
        (fixture(port), ())
    });
    let failures = result.expect_err("an unsupported build must not pass the suite");
    assert!(
        failures.iter().any(|f| f.case == "version-is-reported"),
        "got: {failures:?}"
    );
    assert!(
        failures.iter().all(|f| f.case == "version-is-reported"),
        "the version alone breaks only its own case, got: {failures:?}"
    );
}
