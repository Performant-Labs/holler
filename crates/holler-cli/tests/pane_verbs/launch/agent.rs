//! #644 part 2: the OpenCode agent through `launch` and `relaunch`. The result reports the
//! agent the pane was recorded with (the text line's `, agent KEY` and `data.agent`), the
//! `--help` of both verbs says what `--agent` does and lists the new key, and a server that does
//! not honour the spec's agent (`verify_serving`'s refusal, an open code the harness owns) fails
//! the run through the engine's rollback with nothing recorded. Every case runs over launch's
//! rig; the record half of the agent (#700) and the `ServeSpec` handed to the engine (#802) are
//! pinned where they were first tested (`launch.rs`, `relaunch.rs`, `launch/serve.rs`).

use holler_cli::output::Format;
use holler_cli::pane::profile_scope::reconcile_step;
use holler_pane::error::RefusalCode;
use holler_pane::{HarnessPort, PaneError, ServeSpec};
use holler_pane_testkit::harness::{HarnessOp, ServerState, AGENT_MISMATCH};
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use serde_json::Value;

use super::rig::{
    agent, data_of, error_of, launch_with, launch_without, name, occurrences, profile_name, Rig,
    Run, PANE, PORT, STEP,
};

/// The agent the cases ask for.
const AGENT: &str = "feature-implementor";

/// The phrase every reconcile step starts with.
const STEP_PHRASE: &str = "to reconcile, run";

/// A stand-in for the open code the OpenCode adapter will answer for a key its server does not
/// define (#642 part 3 names it): the verbs pass any open code through unchanged.
const STAND_IN: RefusalCode = RefusalCode::from_static("agent-unusable");
const STAND_IN_MESSAGE: &str = "the OpenCode server defines no agent with that key";

/// What the fake's own `verify_serving` says of a server running another agent (E17).
const MISMATCH_MESSAGE: &str = "the server on port 48100 does not run the agent the spec asks for";

/// The `--help` of `holler <argv...>`, its whitespace folded to single spaces (clap joins and
/// may wrap the doc comment's lines).
fn folded_help(argv: &[&str]) -> String {
    crate::get::help(argv)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text line a live run prints for `PANE` at `r2c1` in `main` on `PORT`, with `session`.
fn result_line(verb: &str, session: &str) -> String {
    format!("{verb} {PANE} at r2c1 in main: session {session}, harness port {PORT}")
}

/// The session of record the run stored for `PANE`.
fn recorded_session(rig: &Rig) -> String {
    rig.record(PANE)
        .and_then(|record| record.session_of_record)
        .expect("a record with a session of record")
}

/// Scope 1 (text): `launch --agent KEY` prints `, agent KEY` after the harness port, and a
/// launch without `--agent` prints exactly the line it printed before this story (golden).
#[test]
fn the_text_line_names_the_agent_and_is_unchanged_without_one() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&["--agent", AGENT]), Format::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    let want = format!(
        "{}, agent {AGENT}",
        result_line("launched", &recorded_session(&rig))
    );
    assert_eq!(run.out.lines().collect::<Vec<_>>(), [want.as_str()]);
    rig.assert_matches(&run, PANE);

    let rig = Rig::new();
    let run = rig.run(&launch_with(&[]), Format::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    let golden = result_line("launched", &recorded_session(&rig));
    assert_eq!(run.out.lines().collect::<Vec<_>>(), [golden.as_str()]);
    assert!(!run.out.contains("agent"), "{run:?}");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Scope 1 (text), A's warn 2: the key goes through the line's one text rule for stored strings
/// (`list::text_value`). `-` is a valid key and also the line's empty value, so it prints
/// quoted and never reads as "no agent".
#[test]
fn a_key_that_reads_as_the_empty_value_is_quoted() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&["--agent=-"]), Format::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    let want = format!(
        "{}, agent \"-\"",
        result_line("launched", &recorded_session(&rig))
    );
    assert_eq!(run.out.lines().collect::<Vec<_>>(), [want.as_str()]);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Scope 1 (JSON): `data.agent` follows `spec_only`, every earlier key keeps its place, and it
/// is the stored record's `opencode_agent`: the key, or `null` for a pane with none. The order
/// is read on the raw output (a parsed `Value` sorts its keys).
#[test]
fn data_carries_the_recorded_agent_after_spec_only() {
    let cases: [(&[&str], Value); 2] = [
        (&["--agent", AGENT], Value::from(AGENT)),
        (&[], Value::Null),
    ];
    for (extra, want) in cases {
        let rig = Rig::new();
        let run = rig.run(&launch_with(extra), Format::Json);
        let data = data_of(&run);
        assert_eq!(data.get("agent"), Some(&want), "{extra:?}: {data}");
        assert_eq!(
            data["agent"], data["pane"]["opencode_agent"],
            "{extra:?}: the record's own key: {data}"
        );
        let tail = format!(r#""profile":null,"spec_only":false,"agent":{want}}}"#);
        assert!(
            run.out.contains(r#""data":{"verb":"launch","pane":{"#) && run.out.contains(&tail),
            "{extra:?}: the keys verb, pane, profile, spec_only, agent in order: {}",
            run.out
        );
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// Scope 1 (JSON): with `--spec-only` nothing is recorded, so `data.agent` is `null` even when
/// the request names an agent (it is never read from the request's spec).
#[test]
fn spec_only_reports_no_agent() {
    let rig = Rig::new();
    let mut argv = launch_without("--herdr-session");
    argv.extend(["--profile", "demo", "--spec-only", "--agent", AGENT]);
    let run = rig.run(&argv, Format::Json);
    let data = data_of(&run);
    assert_eq!(data.get("agent"), Some(&Value::Null), "{data}");
    assert!(
        run.out.contains(r#""spec_only":true,"agent":null}"#),
        "{}",
        run.out
    );
    let spec = &rig.profile("demo").panes[0];
    assert_eq!(spec.opencode_agent, Some(agent(AGENT)), "the spec has it");
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Scope 4: both verbs' `--help` say what `--agent` does without the flag (a new spec has none,
/// an existing one keeps its own) and list `agent` among the data keys.
#[test]
fn the_help_of_both_verbs_names_the_agent() {
    for verb in ["launch", "relaunch"] {
        // clap drops a help line's final period, so neither phrase ends with one.
        let help = folded_help(&["pane", verb]);
        assert!(
            help.contains(
                "Without it, a new spec has none (the server's default agent) and an existing \
                 spec keeps its own"
            ),
            "{verb}: {help}"
        );
        assert!(
            !help.contains("Without it, the server's default agent"),
            "{verb}: the old line, false for a relaunch: {help}"
        );
        assert!(
            help.contains(r#"{"verb", "pane", "profile", "spec_only", "agent"}"#),
            "{verb}: {help}"
        );
    }
}

/// How a row of the refusal table gets its refusal from `verify_serving`.
#[derive(Debug, Clone, Copy)]
enum Refusal {
    /// The harness answers an open code of its own (the adapter's, stood in for).
    StandIn,
    /// The running server runs another agent than the spec's: the fake's own `agent-mismatch`.
    Mismatch,
}

impl Refusal {
    /// Arm `rig` so the run's `verify_serving` refuses; the refusal's code and message.
    fn arm(self, rig: &Rig) -> (String, &'static str) {
        match self {
            Refusal::StandIn => {
                let refusal = PaneError::Refused {
                    code: STAND_IN,
                    message: STAND_IN_MESSAGE.to_owned(),
                };
                rig.fakes
                    .harness
                    .faults()
                    .fail_next(HarnessOp::VerifyServing, refusal);
                (STAND_IN.as_str().to_owned(), STAND_IN_MESSAGE)
            }
            Refusal::Mismatch => {
                // The server the run started is replaced by one running another agent.
                rig.after_server_start(|f, _| {
                    f.harness.kill(PORT).unwrap();
                    let other = ServeSpec::default().with_agent(agent("orchestrator"));
                    let argv = f.harness.serve_command(&name(PANE), PORT, &other).unwrap();
                    f.harness.start_server(&argv).unwrap().unwrap();
                });
                (AGENT_MISMATCH.as_str().to_owned(), MISMATCH_MESSAGE)
            }
        }
    }
}

/// What every refused run shows: exit 3 with the harness's code and its message followed once
/// by `step`, `verify_serving` asked, no session of record created, nothing recorded, and the
/// server the run started stopped by the rollback (`stop_owned` `stops` times in all). `row`
/// names the case.
fn assert_refused(rig: &Rig, run: &Run, want: (String, &str), step: &str, stops: usize, row: &str) {
    let (code, harness_message) = want;
    assert_eq!(run.code, 3, "{row}: {run:?}");
    let (got_code, message) = error_of(run);
    assert_eq!(
        (got_code, message.as_str()),
        (code, format!("{harness_message}; {step}").as_str()),
        "{row}"
    );
    assert_eq!(occurrences(&message, STEP_PHRASE), 1, "{row}: {message}");
    assert!(
        run.calls.harness.contains(&HarnessOp::VerifyServing),
        "{row}"
    );
    assert!(
        !run.calls.harness.contains(&HarnessOp::CreateSession),
        "{row}"
    );
    assert!(!run.calls.panes.contains(&PaneStoreOp::CasPut), "{row}");
    let stopped = run.calls.host.iter().filter(|op| **op == HostOp::StopOwned);
    assert_eq!(stopped.count(), stops, "{row}: {:?}", run.calls.host);
    let server = rig
        .fakes
        .harness
        .server(PORT)
        .expect("a server had started");
    assert_eq!(server.state, ServerState::Killed, "{row}: stopped");
    rig.assert_no_keystroke();
}

/// Acceptance 4: a `verify_serving` refusal on a launch fails the run (`assert_refused`) and
/// records nothing. With `--profile`, P's specs are put back by a second write (the I8
/// compensation, `[1, 2, 3]`). The launch row with `agent-unsupported` is #802's
/// `a_verify_refusal_rolls_back_and_stops_the_server`.
#[test]
fn a_launch_the_server_refuses_records_nothing() {
    let rows: [(Refusal, bool); 3] = [
        (Refusal::StandIn, false),
        (Refusal::Mismatch, false),
        (Refusal::StandIn, true),
    ];
    for (refusal, with_profile) in rows {
        let rig = Rig::new();
        let want = refusal.arm(&rig);
        let mut extra = vec!["--agent", AGENT];
        let mut step = STEP.to_owned();
        let mut want_log: &[u64] = &[1];
        if with_profile {
            extra.extend(["--profile", "demo"]);
            step = reconcile_step(Some(&profile_name("demo")));
            want_log = &[1, 2, 3];
        }
        let run = rig.run(&launch_with(&extra), Format::Json);
        let row = format!("{refusal:?}, profile {with_profile}");
        assert_refused(&rig, &run, want, &step, 1, &row);
        assert_eq!(rig.record(PANE), None, "{row}");
        let profile = rig.profile("demo");
        assert!(profile.panes.is_empty(), "{row}: {:?}", profile.panes);
        let log: Vec<u64> = rig
            .profile_log("demo")
            .iter()
            .map(|e| e.generation)
            .collect();
        assert_eq!(log, want_log, "{row}");
        rig.assert_matches(&run, PANE);
    }
}

/// Acceptance 4 on a relaunch (A's warn 4): the refusal comes after B1, so the old server was
/// stopped and the rollback stops the new one (`stop_owned` twice), and the record is unchanged
/// (same generation, same agent).
#[test]
fn a_relaunch_the_server_refuses_leaves_the_record_unchanged() {
    for refusal in [Refusal::StandIn, Refusal::Mismatch] {
        let rig = Rig::new();
        let before = rig.live(false);
        let want = refusal.arm(&rig);
        let run = rig.run(&["pane", "relaunch", PANE, "--agent", AGENT], Format::Json);
        let row = format!("{refusal:?}");
        assert_refused(&rig, &run, want, STEP, 2, &row);
        assert_eq!(rig.record(PANE), Some(before), "{row}: unchanged");
    }
}
