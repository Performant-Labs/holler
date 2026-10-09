#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
//! The fake harness passes its own conformance suite, the suite lists the cases it
//! documents, and a harness that breaks one rule, or has one of raw OpenCode's quirks
//! on, fails the case that pins that rule (the mutation check; #638, slice e).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The suite is
//! the unit under test here: a suite that cannot tell a broken harness from a good one
//! proves nothing about the OpenCode adapter later.

use std::sync::Mutex;

use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};
use holler_pane_testkit::conformance::harness::{
    harness_cases, run_harness_conformance, HarnessRig,
};
use holler_pane_testkit::conformance::Conformance;
use holler_pane_testkit::harness::{FakeHarness, Quirk};

/// The 15 case ids of the brief's table, in order.
const DOCUMENTED_CASES: [&str; 15] = [
    "health-of-unserved-port-is-false",
    "serve-then-healthy",
    "fresh-server-has-no-sessions",
    "create-session-is-listed",
    "sessions-shared-across-servers",
    "calls-to-unserved-port-are-unavailable",
    "abort-known-session",
    "abort-unknown-is-session-not-found",
    "shown-without-tui-is-none",
    "attach-shows-the-session",
    "attach-unknown-is-session-not-found",
    "select-switches-the-shown-session",
    "select-unknown-is-session-not-found",
    "select-without-tui-fails",
    "select-reaches-only-its-pane",
];

#[test]
fn the_fake_passes_the_harness_conformance_suite() {
    let result = run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_harness_suite_runs_the_documented_cases() {
    assert_eq!(harness_cases(), DOCUMENTED_CASES.to_vec());
}

#[test]
fn the_sample_rig_is_two_ports_and_two_panes() {
    let rig = HarnessRig::sample();
    assert_eq!(rig.ports, [48100, 48101]);
    assert_eq!(
        rig.panes,
        [
            PaneId::new("scratch:demo-c1r1"),
            PaneId::new("scratch:demo-c2r1")
        ]
    );
}

// --- mutation check: a broken harness fails, on the named case ---

/// The suite must reject the run and name `case` among its failures.
fn assert_fails_on(result: Conformance, case: &str) {
    let failures = result.expect_err("a broken harness must not pass the suite");
    assert!(
        failures.iter().any(|f| f.case == case),
        "expected case `{case}` among the failures, got: {failures:?}"
    );
    assert!(
        failures.iter().all(|f| !f.detail.is_empty()),
        "every failure says why: {failures:?}"
    );
}

#[test]
fn a_harness_that_acks_select_without_a_tui_fails() {
    let result = run_harness_conformance(|| {
        let harness = FakeHarness::new();
        harness.set_quirk(Quirk::SelectAckedWithoutTui, true);
        (harness, HarnessRig::sample(), ())
    });
    assert_fails_on(result, "select-without-tui-fails");
}

#[test]
fn a_harness_that_acks_abort_of_an_unknown_id_fails() {
    let result = run_harness_conformance(|| {
        let harness = FakeHarness::new();
        harness.set_quirk(Quirk::AbortUnknownAcked, true);
        (harness, HarnessRig::sample(), ())
    });
    assert_fails_on(result, "abort-unknown-is-session-not-found");
}

#[test]
fn a_harness_whose_servers_do_not_share_a_data_dir_fails() {
    let result = run_harness_conformance(|| {
        let harness = FakeHarness::new();
        harness.set_data_dir(48101, "other");
        (harness, HarnessRig::sample(), ())
    });
    assert_fails_on(result, "sessions-shared-across-servers");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct harness: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `health` answers `true` for every port.
    HealthAlwaysTrue,
    /// `serve` leaves a stray session behind (the "ping" session).
    PingSession,
    /// `create_session` returns the first id it ever minted.
    ReusesSessionId,
    /// `attach_tui` of an unknown id shows the latest session instead.
    AttachFallsBackToLatest,
    /// `select_session` of an unknown id sends the TUI home before refusing.
    SelectUnknownGoesHome,
    /// `select_session` switches every attached TUI of the rig.
    SelectBroadcasts,
}

struct Mutant {
    inner: FakeHarness,
    broken: Break,
    first_id: Mutex<Option<String>>,
}

impl Mutant {
    fn new(broken: Break) -> Self {
        Self {
            inner: FakeHarness::new(),
            broken,
            first_id: Mutex::new(None),
        }
    }
}

fn is_unknown_session(error: &PaneError) -> bool {
    matches!(error, PaneError::SessionNotFound { .. })
}

impl HarnessPort for Mutant {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        let pid = self.inner.serve(name, port)?;
        if self.broken == Break::PingSession {
            self.inner.seed_session(port);
        }
        Ok(pid)
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        if self.broken == Break::HealthAlwaysTrue {
            return Ok(true);
        }
        self.inner.health(port)
    }

    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        let id = self.inner.create_session(port)?;
        if self.broken != Break::ReusesSessionId {
            return Ok(id);
        }
        let mut first = self.first_id.lock().unwrap();
        Ok(first.get_or_insert(id).clone())
    }

    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(port)
    }

    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.abort(port, session)
    }

    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        match self.inner.attach_tui(pane, port, session) {
            Err(e) if self.broken == Break::AttachFallsBackToLatest && is_unknown_session(&e) => {
                match self.inner.list_sessions(port)?.last() {
                    Some(latest) => self.inner.attach_tui(pane, port, latest),
                    None => Err(e),
                }
            }
            other => other,
        }
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        let result = self.inner.select_session(pane, session);
        match (&result, self.broken) {
            (Err(e), Break::SelectUnknownGoesHome) if is_unknown_session(e) => {
                self.inner.navigate(pane, None).ok();
            }
            (Ok(()), Break::SelectBroadcasts) => {
                for other in HarnessRig::sample().panes {
                    if other != *pane && self.inner.tui(&other).is_some() {
                        self.inner.select_session(&other, session).ok();
                    }
                }
            }
            _ => {}
        }
        result
    }

    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane)
    }
}

fn assert_suite_fails_on(broken: Break, case: &str) {
    let result = run_harness_conformance(|| (Mutant::new(broken), HarnessRig::sample(), ()));
    assert_fails_on(result, case);
}

#[test]
fn the_unbroken_harness_wrapper_passes() {
    let result =
        run_harness_conformance(|| (Mutant::new(Break::Nothing), HarnessRig::sample(), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn a_harness_whose_health_is_always_true_fails() {
    assert_suite_fails_on(Break::HealthAlwaysTrue, "health-of-unserved-port-is-false");
}

#[test]
fn a_harness_whose_serve_leaves_a_session_behind_fails() {
    assert_suite_fails_on(Break::PingSession, "fresh-server-has-no-sessions");
}

#[test]
fn a_harness_that_reuses_a_session_id_fails() {
    assert_suite_fails_on(Break::ReusesSessionId, "create-session-is-listed");
}

#[test]
fn a_harness_whose_attach_of_an_unknown_id_shows_the_latest_session_fails() {
    assert_suite_fails_on(
        Break::AttachFallsBackToLatest,
        "attach-unknown-is-session-not-found",
    );
}

#[test]
fn a_harness_whose_select_of_an_unknown_id_goes_home_fails() {
    assert_suite_fails_on(
        Break::SelectUnknownGoesHome,
        "select-unknown-is-session-not-found",
    );
}

#[test]
fn a_harness_whose_select_switches_every_tui_fails() {
    assert_suite_fails_on(Break::SelectBroadcasts, "select-reaches-only-its-pane");
}
