#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! The Claude Code conformance suite (#831, ADR-0021 section 15 item 15; plan review W1, W12):
//! `FakeClaude` passes it, the suite lists the cases it documents, and a harness that breaks one
//! rule fails the case that pins that rule (one mutant per case, as `harness_conformance_test.rs`
//! does for the server harness).
//!
//! The suite is run by the fake here, by the router of the two fakes
//! (`fake_harnesses_test.rs`) and by the real adapter against a scratch Claude Code (opt-in,
//! `holler-adapter-claude/tests/real_claude_test.rs`). A launcher and a stopper are passed, as for
//! the server suite (#802): the port only builds the argv, and the host runs it in the pane's
//! tmux session and stops it there.

use holler_pane::ports::{Activity, CompactModel, CompactOutcome, LoginStatus, SessionChoice};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
    TurnAgent,
};
use holler_pane_testkit::claude::FakeClaude;
use holler_pane_testkit::conformance::claude::{claude_cases, run_claude_conformance, ClaudeRig};
use holler_pane_testkit::conformance::Conformance;

/// The case ids, in order. W1 (switch keeps its `session-not-found`) is case 6; W12 (delivery is
/// proven at `send_prompt`) is cases 8 and 9. #647 part 2's `agent_view` is case 13, and #878's
/// `compact` is case 14.
const DOCUMENTED_CASES: [&str; 14] = [
    "process-command-starts-nothing",
    "launch-then-located",
    "new-session-ids-are-distinct",
    "located-process-is-healthy-and-idle",
    "shown-is-the-held-session",
    "resume-without-conversation-is-session-not-found",
    "keep-without-conversation-starts-it",
    "prompt-to-another-session-is-session-mismatch",
    "resume-after-a-prompt-keeps-the-session",
    "stopped-process-is-not-healthy",
    "abort-is-unsupported",
    "login-status-answers",
    "agent-view-of-a-process-is-empty",
    "compact-is-unsupported",
];

fn launch_fake(fake: &FakeClaude, _: &PaneName, argv: &Argv) -> Result<(), PaneError> {
    match fake.start(argv) {
        Some(started) => started.map(drop),
        None => Err(PaneError::Usage {
            message: "not an argv the fake's process_command built".to_owned(),
        }),
    }
}

fn stop_fake(fake: &FakeClaude, name: &PaneName) -> Result<(), PaneError> {
    fake.stop(name);
    Ok(())
}

#[test]
fn the_fake_passes_the_claude_conformance_suite() {
    let result = run_claude_conformance(
        || (FakeClaude::new(), ClaudeRig::sample(), ()),
        launch_fake,
        stop_fake,
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn the_claude_suite_runs_the_documented_cases() {
    assert_eq!(claude_cases(), DOCUMENTED_CASES.to_vec());
}

#[test]
fn the_sample_claude_rig_is_two_panes_in_a_scratch_directory() {
    let rig = ClaudeRig::sample();
    assert_eq!(
        rig.panes,
        [
            PaneName::parse("demo-c1r1").unwrap(),
            PaneName::parse("demo-c2r1").unwrap()
        ]
    );
    assert_eq!(
        rig.pane_ids,
        [
            PaneId::new("scratch:demo-c1r1"),
            PaneId::new("scratch:demo-c2r1")
        ]
    );
    assert_eq!(rig.cwd, "/srv/demo");
}

// --- mutation check: a broken harness fails, on the named case ---------------------------

/// The one rule a mutant breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Broken {
    /// `process_command` also starts the process it describes.
    ProcessCommandStarts,
    /// `locate_process` never finds a process.
    NeverLocated,
    /// `new_session_id` answers the same id every time.
    RepeatsSessionId,
    /// `session_activity` answers `Busy` for an idle process.
    ReportsBusy,
    /// `shown_session` never tells.
    ShowsNothing,
    /// `process_command` of `Resume` starts a session that has no conversation (W1).
    ResumeStartsAnything,
    /// `process_command` of `Keep` refuses a session that has no conversation.
    KeepRefusesUnknown,
    /// `send_prompt` delivers to whatever session the process holds, whatever was named.
    PromptsAnySession,
    /// `send_prompt` answers `Ok` and delivers nothing.
    DropsPrompt,
    /// `health` answers `true` for a process that is gone.
    HealthyForever,
    /// `abort` answers `Ok`, as if a signal were an abort.
    AcksAbort,
    /// `login_status` fails.
    LoginFails,
    /// `agent_view` guesses a turn it cannot know (#647 part 2).
    GuessesATurn,
    /// `compact` answers an outcome for a harness that cannot compact (#878).
    AcksCompact,
}

/// The fake, with one rule broken.
struct Mutant {
    inner: FakeClaude,
    broken: Broken,
}

impl Mutant {
    fn is(&self, broken: Broken) -> bool {
        self.broken == broken
    }
}

impl HarnessPort for Mutant {
    fn serve_command(&self, n: &PaneName, port: u16, s: &ServeSpec) -> Result<Argv, PaneError> {
        self.inner.serve_command(n, port, s)
    }
    fn verify_serving(&self, at: &HarnessEndpoint, s: &ServeSpec) -> Result<(), PaneError> {
        self.inner.verify_serving(at, s)
    }
    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        if self.is(Broken::HealthyForever) {
            return Ok(true);
        }
        self.inner.health(at)
    }
    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.inner.create_session(at)
    }
    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(at)
    }
    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        if self.is(Broken::AcksAbort) {
            return Ok(());
        }
        self.inner.abort(at, session)
    }
    fn attach_tui(&self, p: &PaneId, at: &HarnessEndpoint, s: &str) -> Result<(), PaneError> {
        self.inner.attach_tui(p, at, s)
    }
    fn select_session(&self, p: &PaneId, s: &str) -> Result<(), PaneError> {
        self.inner.select_session(p, s)
    }
    fn shown_session(&self, p: &PaneId, at: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
        if self.is(Broken::ShowsNothing) {
            return Ok(None);
        }
        self.inner.shown_session(p, at)
    }
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        if self.is(Broken::DropsPrompt) {
            return Ok(());
        }
        if self.is(Broken::PromptsAnySession) {
            let pane = PaneId::new("scratch:any");
            let held = self.inner.shown_session(&pane, at)?.unwrap_or_default();
            return self.inner.send_prompt(at, &held, text);
        }
        self.inner.send_prompt(at, session, text)
    }
    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        if self.is(Broken::ReportsBusy) {
            return Ok(Activity::Busy);
        }
        self.inner.session_activity(at, session)
    }
    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        choice: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        let choice = match (self.broken, choice) {
            (Broken::ResumeStartsAnything, SessionChoice::Resume(id)) => {
                SessionChoice::Keep(id.clone())
            }
            (Broken::KeepRefusesUnknown, SessionChoice::Keep(id)) => {
                SessionChoice::Resume(id.clone())
            }
            _ => choice.clone(),
        };
        let argv = self.inner.process_command(name, spec, &choice)?;
        if self.is(Broken::ProcessCommandStarts) {
            let _ = self.inner.start(&argv);
        }
        Ok(argv)
    }
    fn new_session_id(&self) -> Result<String, PaneError> {
        if self.is(Broken::RepeatsSessionId) {
            return Ok("the-same-session".to_owned());
        }
        self.inner.new_session_id()
    }
    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        if self.is(Broken::NeverLocated) {
            return Ok(None);
        }
        self.inner.locate_process(name, session)
    }
    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        if self.is(Broken::LoginFails) {
            return Err(PaneError::Unavailable {
                what: "claude auth status".to_owned(),
            });
        }
        self.inner.login_status()
    }
    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        let view = self.inner.agent_view(at, session, max_turns)?;
        if self.is(Broken::GuessesATurn) {
            return Ok(view.with_turns(vec![TurnAgent::new(None, Some(false))]));
        }
        Ok(view)
    }
    fn compact(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        model: Option<&CompactModel>,
    ) -> Result<CompactOutcome, PaneError> {
        if self.is(Broken::AcksCompact) {
            return Ok(CompactOutcome {
                tokens_before: 90_000,
                tokens_after: 12_000,
            });
        }
        self.inner.compact(at, session, model)
    }
}

fn run_mutant(broken: Broken) -> Conformance {
    run_claude_conformance(
        || {
            let mutant = Mutant {
                inner: FakeClaude::new(),
                broken,
            };
            (mutant, ClaudeRig::sample(), ())
        },
        |m: &Mutant, name: &PaneName, argv: &Argv| launch_fake(&m.inner, name, argv),
        |m: &Mutant, name: &PaneName| stop_fake(&m.inner, name),
    )
}

/// The suite must reject the run and name `case` among its failures, each with a reason.
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
fn every_case_has_a_mutant_that_it_catches() {
    let pairs = [
        (
            Broken::ProcessCommandStarts,
            "process-command-starts-nothing",
        ),
        (Broken::NeverLocated, "launch-then-located"),
        (Broken::RepeatsSessionId, "new-session-ids-are-distinct"),
        (Broken::ReportsBusy, "located-process-is-healthy-and-idle"),
        (Broken::ShowsNothing, "shown-is-the-held-session"),
        (
            Broken::ResumeStartsAnything,
            "resume-without-conversation-is-session-not-found",
        ),
        (
            Broken::KeepRefusesUnknown,
            "keep-without-conversation-starts-it",
        ),
        (
            Broken::PromptsAnySession,
            "prompt-to-another-session-is-session-mismatch",
        ),
        (
            Broken::DropsPrompt,
            "resume-after-a-prompt-keeps-the-session",
        ),
        (Broken::HealthyForever, "stopped-process-is-not-healthy"),
        (Broken::AcksAbort, "abort-is-unsupported"),
        (Broken::LoginFails, "login-status-answers"),
        (Broken::GuessesATurn, "agent-view-of-a-process-is-empty"),
        (Broken::AcksCompact, "compact-is-unsupported"),
    ];
    let covered: Vec<&str> = pairs.iter().map(|&(_, case)| case).collect();
    assert_eq!(
        covered,
        DOCUMENTED_CASES.to_vec(),
        "one mutant per case, in order"
    );
    for (broken, case) in pairs {
        assert_fails_on(run_mutant(broken), case);
    }
}
