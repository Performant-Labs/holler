#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
//! The fake harness passes its own conformance suite, the suite lists the cases it
//! documents, and a harness that breaks one rule, or has one of raw OpenCode's quirks
//! on, fails the case that pins that rule (the mutation check; #638, slice e).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The suite is
//! the unit under test here: a suite that cannot tell a broken harness from a good one
//! proves nothing about the OpenCode adapter later.
//!
//! #802: `serve_command` only builds an argv, so the suite starts a server through a
//! launcher the caller passes, generic over the subject. The fake and its mutants launch
//! through the fake's scenario method `start_server`.

use std::sync::Mutex;

use holler_pane::ports::{CompactModel, CompactOutcome, COMPACT_MODEL_UNPINNED};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
    TurnAgent,
};
use holler_pane_testkit::conformance::harness::{
    harness_cases, run_harness_conformance, HarnessRig,
};
use holler_pane_testkit::conformance::Conformance;
use holler_pane_testkit::harness::{FakeHarness, Quirk};

/// The 16 case ids of the brief's table, in order (#802 renamed case 2 and added case 16),
/// #647 part 2's `agent_view` case and #878's three `compact` cases.
const DOCUMENTED_CASES: [&str; 20] = [
    "health-of-unserved-port-is-false",
    "launch-then-healthy",
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
    "serve-command-carries-the-agent",
    "agent-view-of-a-fresh-session-has-no-turns",
    "compact-of-unserved-port-is-unavailable",
    "compact-of-unknown-session-is-session-not-found",
    "compact-shrinks-the-context-or-is-refused",
];

/// The fake's launcher: start the server the argv describes, as a host's `run` would.
fn launch_fake(harness: &FakeHarness, _name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
    match harness.start_server(argv) {
        Some(started) => started.map(drop),
        None => Err(PaneError::Usage {
            message: "not an argv the fake's serve_command built".to_owned(),
        }),
    }
}

#[test]
fn the_fake_passes_the_harness_conformance_suite() {
    let result = run_harness_conformance(
        || (FakeHarness::new(), HarnessRig::sample(), ()),
        launch_fake,
    );
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
    let result = run_harness_conformance(
        || {
            let harness = FakeHarness::new();
            harness.set_quirk(Quirk::SelectAckedWithoutTui, true);
            (harness, HarnessRig::sample(), ())
        },
        launch_fake,
    );
    assert_fails_on(result, "select-without-tui-fails");
}

#[test]
fn a_harness_that_acks_abort_of_an_unknown_id_fails() {
    let result = run_harness_conformance(
        || {
            let harness = FakeHarness::new();
            harness.set_quirk(Quirk::AbortUnknownAcked, true);
            (harness, HarnessRig::sample(), ())
        },
        launch_fake,
    );
    assert_fails_on(result, "abort-unknown-is-session-not-found");
}

#[test]
fn a_harness_whose_servers_do_not_share_a_data_dir_fails() {
    let result = run_harness_conformance(
        || {
            let harness = FakeHarness::new();
            harness.set_data_dir(48101, "other");
            (harness, HarnessRig::sample(), ())
        },
        launch_fake,
    );
    assert_fails_on(result, "sessions-shared-across-servers");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct harness: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `health` answers `true` for every port.
    HealthAlwaysTrue,
    /// Launching the server leaves a stray session behind (the "ping" session).
    PingSession,
    /// `serve_command` drops the spec's agent: the argv would start a server without it
    /// (#802).
    DropsAgent,
    /// `serve_command` starts the server it describes: it is not pure (#802).
    ServeCommandStarts,
    /// `create_session` returns the first id it ever minted.
    ReusesSessionId,
    /// `attach_tui` of an unknown id shows the latest session instead.
    AttachFallsBackToLatest,
    /// `select_session` of an unknown id sends the TUI home before refusing.
    SelectUnknownGoesHome,
    /// `select_session` switches every attached TUI of the rig.
    SelectBroadcasts,
    /// `agent_view` answers an empty view whatever it is asked, as a wrapper that answers a
    /// constant would (#647 part 2).
    AgentViewAlwaysEmpty,
    /// `agent_view` invents a turn the session never had.
    AgentViewInventsATurn,
    /// `compact` of a port with no server answers an outcome instead of `unavailable` (#878).
    CompactAcksAnUnservedPort,
    /// `compact` of an id the server does not know answers an outcome instead of
    /// `session-not-found` (#878).
    CompactAcksAnUnknownSession,
    /// `compact` answers a context that did not shrink, and does so in place of the refusal a
    /// harness with no model of its own owes (#878).
    CompactDoesNotShrink,
}

/// An outcome that shrank, which a broken `compact` answers where it owed a refusal.
const SHRUNK: CompactOutcome = CompactOutcome {
    tokens_before: 90_000,
    tokens_after: 12_000,
};

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

/// The mutants' launcher: the fake's, and the "ping" session of [`Break::PingSession`]
/// on the port of the server it started.
fn launch_mutant(mutant: &Mutant, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
    launch_fake(&mutant.inner, name, argv)?;
    if mutant.broken == Break::PingSession {
        for port in HarnessRig::sample().ports {
            if mutant.inner.server(port).is_some_and(|s| s.name == *name) {
                mutant.inner.seed_session(port);
            }
        }
    }
    Ok(())
}

impl HarnessPort for Mutant {
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        let asked = match self.broken {
            Break::DropsAgent => ServeSpec::default(),
            _ => spec.clone(),
        };
        let argv = self.inner.serve_command(name, port, &asked)?;
        if self.broken == Break::ServeCommandStarts {
            self.inner.start_server(&argv);
        }
        Ok(argv)
    }

    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.inner.verify_serving(at, spec)
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        if self.broken == Break::HealthAlwaysTrue {
            return Ok(true);
        }
        self.inner.health(at)
    }

    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        let id = self.inner.create_session(at)?;
        if self.broken != Break::ReusesSessionId {
            return Ok(id);
        }
        let mut first = self.first_id.lock().unwrap();
        Ok(first.get_or_insert(id).clone())
    }

    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(at)
    }

    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.inner.abort(at, session)
    }

    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        match self.inner.attach_tui(pane, at, session) {
            Err(e) if self.broken == Break::AttachFallsBackToLatest && is_unknown_session(&e) => {
                match self.inner.list_sessions(at)?.last() {
                    Some(latest) => self.inner.attach_tui(pane, at, latest),
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

    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane, at)
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        match self.broken {
            Break::AgentViewAlwaysEmpty => Ok(AgentView::default()),
            Break::AgentViewInventsATurn => self
                .inner
                .agent_view(at, session, max_turns)
                .map(|view| view.with_turns(vec![TurnAgent::new(None, Some(false))])),
            _ => self.inner.agent_view(at, session, max_turns),
        }
    }

    fn compact(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        model: Option<&CompactModel>,
    ) -> Result<CompactOutcome, PaneError> {
        if self.broken == Break::CompactAcksAnUnservedPort && !self.inner.health(at)? {
            return Ok(SHRUNK);
        }
        match (self.broken, self.inner.compact(at, session, model)) {
            (Break::CompactAcksAnUnknownSession, Err(e)) if is_unknown_session(&e) => Ok(SHRUNK),
            (Break::CompactDoesNotShrink, Err(e))
                if e.code() == COMPACT_MODEL_UNPINNED.as_str() =>
            {
                Ok(CompactOutcome {
                    tokens_before: 4_000,
                    tokens_after: 4_000,
                })
            }
            (_, answer) => answer,
        }
    }
}

fn assert_suite_fails_on(broken: Break, case: &str) {
    let result = run_harness_conformance(
        || (Mutant::new(broken), HarnessRig::sample(), ()),
        launch_mutant,
    );
    assert_fails_on(result, case);
}

#[test]
fn the_unbroken_harness_wrapper_passes() {
    let result = run_harness_conformance(
        || (Mutant::new(Break::Nothing), HarnessRig::sample(), ()),
        launch_mutant,
    );
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

/// #802: an argv that would start a server without the agent the spec asks for fails the
/// agent case (an implementation either carries the key or refuses `agent-unsupported`).
#[test]
fn a_harness_whose_serve_command_drops_the_agent_fails() {
    assert_suite_fails_on(Break::DropsAgent, "serve-command-carries-the-agent");
}

/// #802: `serve_command` starts nothing; one that starts the server fails case 2, which
/// checks that the port is still not healthy before the launcher runs.
#[test]
fn a_harness_whose_serve_command_starts_the_server_fails() {
    assert_suite_fails_on(Break::ServeCommandStarts, "launch-then-healthy");
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

#[test]
fn a_harness_whose_agent_view_is_always_empty_fails() {
    assert_suite_fails_on(
        Break::AgentViewAlwaysEmpty,
        "agent-view-of-a-fresh-session-has-no-turns",
    );
}

#[test]
fn a_harness_whose_agent_view_invents_a_turn_fails() {
    assert_suite_fails_on(
        Break::AgentViewInventsATurn,
        "agent-view-of-a-fresh-session-has-no-turns",
    );
}

/// #878: a `compact` on a port with no server is `unavailable`, checked before the id.
#[test]
fn a_harness_that_compacts_on_an_unserved_port_fails() {
    assert_suite_fails_on(
        Break::CompactAcksAnUnservedPort,
        "compact-of-unserved-port-is-unavailable",
    );
}

/// #878: an unknown id is `session-not-found`, as `abort` and `send_prompt` answer.
#[test]
fn a_harness_that_compacts_an_unknown_session_fails() {
    assert_suite_fails_on(
        Break::CompactAcksAnUnknownSession,
        "compact-of-unknown-session-is-session-not-found",
    );
}

/// #878: an outcome whose context did not shrink is not a compaction.
#[test]
fn a_harness_whose_compaction_does_not_shrink_the_context_fails() {
    assert_suite_fails_on(
        Break::CompactDoesNotShrink,
        "compact-shrinks-the-context-or-is-refused",
    );
}
