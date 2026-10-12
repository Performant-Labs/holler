//! `holler pane launch PANE` (story #644): the live path, its rollbacks and the budget (brief
//! AC 1-8, 11b, 13a, 17 and 24).
//!
//! Every case runs over the rig of `launch/rig.rs` (#643's fakes, a linked host and harness
//! hooks) and ends with `assert_no_keystroke`; every case but AC 6 and 7 also ends with
//! `assert_matches` (the registry equals the fakes). The refusals before the act are in
//! `launch/guards.rs`, the `--profile` cases in `launch/profiles.rs`, the harness server's
//! cases of #802 (`serve_command` in the plan, the server run in the pane's tmux session, the
//! health wait, `verify_serving`) in `launch/serve.rs`, the OpenCode agent's result and
//! refusals of #644 part 2 in `launch/agent.rs`, and relaunch's cases in `relaunch.rs`, which
//! reuses this rig.

pub(crate) mod rig;

mod agent;
mod displaced;
mod guards;
mod profiles;
mod record_bytes;
mod serve;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use holler_cli::output::Format;
use holler_pane::pane::{HarnessInfo, Health, HostInfo, LastObserved, PaneRole};
use holler_pane::tx_launch::{self, LaunchRequest, TxFailure, TxOptions};
use holler_pane::{GridPos, HarnessEndpoint, HarnessPort, Pane, PaneError, PaneId, ProbeResult};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fixture::sample_pane;
use holler_pane_testkit::harness::{HarnessOp, ServerState};
use holler_pane_testkit::herdr::{HerdrOp, PROTOCOL_22_VERSION};
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use serde_json::json;

use crate::verb_harness::parse::assert_spec_flags_accepted;
use displaced::Displace;
use rig::{
    argv, data_of, error_of, launch_replacing, launch_spec, launch_with, name, occurrences, Rig,
    LAUNCH, PANE, PORT, PROBE, PROBE_FLAGS, STEP,
};

/// `ops` with every run of one op folded into one: the order of the steps, whatever number
/// of health polls the wait made.
pub(crate) fn steps<T: PartialEq + Clone>(ops: &[T]) -> Vec<T> {
    let mut steps = ops.to_vec();
    steps.dedup();
    steps
}

/// A run's budget for the engine cases that wait out a server that never answers.
pub(crate) const SHORT_BUDGET: Duration = Duration::from_millis(700);

/// The engine's own request for `LAUNCH`.
pub(crate) fn launch_request() -> LaunchRequest {
    LaunchRequest {
        name: name(PANE),
        herdr_session: Some("scratch".to_owned()),
        spec: launch_spec(),
        profile: None,
        spec_only: false,
    }
}

/// The cell `LAUNCH` names.
const R2C1: GridPos = GridPos { row: 2, col: 1 };

/// AC 1: a launch records exactly what the fakes show, through the one call order.
#[test]
fn launch_records_what_the_fakes_show() {
    let rig = Rig::new();
    let run = rig.run(LAUNCH, Format::Text);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{run:?}");
    let record = rig.record(PANE).expect("a record");
    let sessions = rig
        .fakes
        .harness
        .list_sessions(&HarnessEndpoint::Port(PORT))
        .unwrap();
    let [sid] = sessions.as_slice() else {
        panic!("exactly one session, no ping session: {sessions:?}");
    };
    assert_eq!(run.out.lines().count(), 1, "{run:?}");
    assert!(
        run.out.contains("r2c1") && run.out.contains(sid.as_str()),
        "{run:?}"
    );
    let server = rig.fakes.harness.server(PORT).expect("a server");
    let sample = sample_pane(PANE).unwrap();
    let expected = Pane {
        generation: 1,
        herdr: rig.herdr_pane_at(R2C1).expect("a Herdr pane at r2c1"),
        host: HostInfo {
            name: "localhost".to_owned(),
            tmux: PANE.to_owned(),
            cwd: "/srv/demo".to_owned(),
            herdr_api_version: Some(PROTOCOL_22_VERSION.to_owned()),
        },
        harness: HarnessInfo {
            port: Some(PORT),
            // #802: the server is a process of the pane's tmux session; no pid is recorded.
            pid: None,
            health: Health::Healthy,
            ..sample.harness
        },
        session_of_record: Some(sid.clone()),
        // SHOWN is O1's observation; DRIVEN is never inferred (decision 22).
        last_observed: LastObserved {
            shown: Some(sid.clone()),
            driven: None,
            ..record.last_observed
        },
        // #700: no --agent flag and no base spec, so the record holds no key.
        opencode_agent: None,
        ..sample
    };
    assert_eq!(record, expected, "no profile, no command, no probe result");
    let shown = rig.fakes.harness.tui(&record.herdr.pane_id).unwrap().shown;
    assert_eq!(
        shown.as_ref(),
        Some(sid),
        "the TUI shows the session of record"
    );
    // #802: the argv is built in the plan, the server's health is waited for, and a spec
    // with no agent is verified too.
    let harness = [
        HarnessOp::ServeCommand,
        HarnessOp::Health,
        HarnessOp::VerifyServing,
        HarnessOp::CreateSession,
        HarnessOp::AttachTui,
        HarnessOp::ShownSession,
    ];
    assert_eq!(steps(&run.calls.harness), harness);
    let herdr = [
        HerdrOp::Version,
        HerdrOp::Snapshot,
        HerdrOp::EnsurePane,
        HerdrOp::Snapshot,
    ];
    assert_eq!(run.calls.herdr, herdr);
    assert!(
        run.calls.profiles.is_empty(),
        "AC 16f: no profile, no profile call"
    );
    let runs = rig.fakes.host.runs();
    let [(session, cwd, server_argv)] = runs.as_slice() else {
        panic!("AC 11b: no command, so the server's argv is the one run: {runs:?}");
    };
    assert_eq!((session.as_str(), cwd.as_str()), (PANE, "/srv/demo"));
    assert_eq!(
        rig.fakes.harness.start_server(server_argv),
        Some(Ok(server.pid)),
        "the argv run is the running server's own"
    );
    assert!(run.calls.probes.is_empty(), "no check, no probe");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #700 (A's DECISION 1, positive path): a launch with `--agent` stores the key in the
/// pane record — the record literal writes the spec's key, not `None`.
#[test]
fn launch_with_agent_stores_the_key_in_the_record() {
    let rig = Rig::new();
    let run = rig.run(
        &launch_with(&["--agent", "feature-implementor"]),
        Format::Json,
    );
    let data = data_of(&run);
    let record = rig.record(PANE).unwrap();
    assert_eq!(
        record.opencode_agent.as_ref().map(|key| key.as_str()),
        Some("feature-implementor")
    );
    // #644 part 2: the result reports it, and the one write that recorded the pane did.
    assert_eq!(data["agent"], "feature-implementor", "{data}");
    let puts = run
        .calls
        .panes
        .iter()
        .filter(|op| **op == PaneStoreOp::CasPut)
        .count();
    assert_eq!((puts, record.generation), (1, 1), "{:?}", run.calls.panes);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 3 (C1): the model, effort, env names, ceilings and role are recorded.
#[test]
fn launch_records_model_effort_env_and_ceilings() {
    let rig = Rig::new();
    let mut argv = launch_replacing("--model", "p1/m1");
    for (flag, value) in [
        ("--effort", "high"),
        ("--ctx-soft", "1000"),
        ("--ctx-hard", "2000"),
    ] {
        let at = argv.iter().position(|a| *a == flag).unwrap();
        argv[at + 1] = value;
    }
    argv.extend([
        "--env",
        "ALPHA_TOKEN",
        "--env",
        "BETA_URL",
        "--role",
        "orchestrator",
    ]);
    let run = rig.run(&argv, Format::Json);
    data_of(&run);
    let record = rig.record(PANE).unwrap();
    let model = (
        record.model.provider.as_str(),
        record.model.model_id.as_str(),
    );
    assert_eq!(
        (model, record.model.effort.as_str()),
        (("p1", "m1"), "high")
    );
    let env: Vec<&str> = record.env.iter().map(|e| e.as_str()).collect();
    assert_eq!(env, ["ALPHA_TOKEN", "BETA_URL"]);
    assert_eq!((record.context.soft, record.context.hard), (1000, 2000));
    assert_eq!(record.role, PaneRole::Orchestrator);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Launch `LAUNCH` through the engine with [`SHORT_BUDGET`]; it must end as the budget's
/// `timeout` (`pane.launch`) after the act began, well inside a generous bound, with nothing
/// recorded and the Herdr pane it made closed. Returns the calls it made.
fn assert_waits_out_the_budget(rig: &Rig) -> rig::Calls {
    let options = TxOptions {
        budget: SHORT_BUDGET,
        ..TxOptions::default()
    };
    let started = Instant::now();
    let (answer, calls) = rig.engine(|ports| tx_launch::launch(ports, &launch_request(), &options));
    let took = started.elapsed();
    let timeout = PaneError::Timeout {
        op: "pane.launch".to_owned(),
    };
    assert_eq!(
        answer,
        Err(TxFailure {
            error: timeout,
            acted: true
        })
    );
    assert!(took < Duration::from_secs(3), "bounded: {took:?}");
    let reached = |op| calls.harness.contains(&op);
    assert!(!reached(HarnessOp::VerifyServing), "{:?}", calls.harness);
    assert!(!reached(HarnessOp::CreateSession) && !reached(HarnessOp::AttachTui));
    assert!(calls.herdr.contains(&HerdrOp::Close), "{:?}", calls.herdr);
    assert_eq!(
        rig.herdr_ids(),
        Vec::<PaneId>::new(),
        "the created pane is closed"
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_no_keystroke();
    calls
}

/// AC 4a (#802): a frozen server of the pane holds its port, so the server the act starts
/// never answers: the health wait runs out the budget before any session or TUI exists.
#[test]
fn launch_onto_a_frozen_server_times_out_before_any_session() {
    let rig = Rig::new();
    rig.serve(PANE, PORT);
    rig.fakes.harness.freeze(PORT).unwrap();
    assert_waits_out_the_budget(&rig);
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_ne!(
        server.state,
        ServerState::Running,
        "never adopted: {server:?}"
    );
}

/// AC 4b, rewritten by #802 (its AC 6): a server that never becomes healthy after the host
/// started it ends the launch as `timeout` (no longer `unavailable`), and the rollback stops
/// the session's processes, the server among them, and closes the Herdr pane the run made.
#[test]
fn launch_times_out_when_the_server_never_gets_healthy() {
    let rig = Rig::new();
    rig.after_server_start(|f, c| f.harness.freeze(c.port.unwrap()).unwrap());
    let calls = assert_waits_out_the_budget(&rig);
    assert!(calls.host.contains(&HostOp::StopOwned), "{:?}", calls.host);
    assert_eq!(
        rig.fakes.harness.server(PORT).unwrap().state,
        ServerState::Killed,
        "the rollback stopped the server it started"
    );
    let polls = calls
        .harness
        .iter()
        .filter(|op| **op == HarnessOp::Health)
        .count();
    assert!(
        polls >= 3,
        "the wait polled more than once: {:?}",
        calls.harness
    );
}

/// AC 5a: a Herdr pane that vanishes after the attach is observed, and nothing is recorded.
#[test]
fn launch_fails_when_its_herdr_pane_vanishes() {
    let rig = Rig::new();
    let vanished = Arc::new(Mutex::new(None));
    let seen = vanished.clone();
    rig.after(HarnessOp::AttachTui, move |f, c| {
        let pane = c.pane.clone().unwrap();
        f.herdr.vanish(&pane).unwrap();
        *seen.lock().unwrap() = Some(pane);
    });
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "unavailable");
    let pane: PaneId = vanished.lock().unwrap().clone().expect("the attach ran");
    assert!(message.contains(pane.as_str()), "names the pane: {message}");
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 6b: another writer that creates the record during the act wins; the run fails loudly
/// with the reconcile step, writes nothing more and leaves what it made live.
#[test]
fn launch_record_conflict_fails_loudly() {
    let rig = Rig::new();
    rig.after_server_start(|f, _| {
        f.panes.concurrent_put(&sample_pane(PANE).unwrap()).unwrap();
    });
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "generation-conflict");
    assert!(message.ends_with(&format!("; {STEP}")), "{message}");
    let theirs = rig.record(PANE).unwrap();
    assert_eq!(
        theirs,
        Pane {
            generation: 1,
            ..sample_pane(PANE).unwrap()
        }
    );
    assert!(rig.herdr_pane_at(R2C1).is_some(), "the live pane is left");
    assert!(
        rig.fakes.host.sessions().contains(&name(PANE)),
        "the tmux session is left"
    );
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(server.state, ServerState::Running, "the server is left");
    rig.assert_no_keystroke();
}

/// AC 7: a crash between steps writes no record; what it leaves is observable. A crash runs
/// no rollback (#802): the server the host started stays in the pane's tmux session.
#[test]
fn a_crash_mid_launch_leaves_no_record() {
    let rig = Rig::new();
    rig.after_server_start(|_, _| panic!("the CLI crashes as the server starts (#644 AC 7)"));
    let mark = rig.mark();
    let crashed = catch_unwind(AssertUnwindSafe(|| rig.run(LAUNCH, Format::Text)));
    assert!(crashed.is_err(), "the run crashed: {crashed:?}");
    let calls = rig.calls_since(&mark);
    assert!(
        !calls.panes.contains(&PaneStoreOp::CasPut),
        "{:?}",
        calls.panes
    );
    assert_eq!(rig.record(PANE), None);
    assert!(rig.herdr_pane_at(R2C1).is_some(), "the Herdr pane is left");
    assert!(
        rig.fakes.host.sessions().contains(&name(PANE)),
        "the tmux session is left"
    );
    let server = rig.fakes.harness.server(PORT).expect("the server started");
    assert_eq!(
        server.state,
        ServerState::Running,
        "and is left: {server:?}"
    );
    rig.assert_no_keystroke();
}

/// AC 8: a failed step rolls back what the run made, with the step's own code and the
/// reconcile step once.
#[test]
fn a_failed_attach_rolls_back() {
    let rig = Rig::new();
    let missing = PaneError::SessionNotFound {
        what: "ses_gone".into(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::AttachTui, missing);
    let run = rig.run(LAUNCH, Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "session-not-found");
    assert_eq!(occurrences(&message, "to reconcile, run"), 1, "{message}");
    assert!(message.contains(STEP), "{message}");
    assert!(
        run.calls.herdr.contains(&HerdrOp::Close),
        "{:?}",
        run.calls.herdr
    );
    assert_eq!(
        rig.herdr_ids(),
        Vec::<PaneId>::new(),
        "the created pane is closed"
    );
    assert_eq!(
        rig.fakes.host.sessions(),
        vec![name(PANE)],
        "the tmux session stays"
    );
    let ps = holler_pane::HostPort::ps(&rig.fakes.host, &name(PANE)).unwrap();
    assert_eq!(ps, Vec::<u32>::new(), "its processes are stopped");
    assert_eq!(
        rig.fakes.harness.server(PORT).unwrap().state,
        ServerState::Killed
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 11b: the command reaches the host as the argv given, element by element, never a shell
/// line, and is recorded.
#[test]
fn the_command_reaches_the_host_as_argv() {
    let rig = Rig::new();
    let extra = [
        "--command-arg",
        "prog",
        "--command-arg",
        "a b",
        "--command-arg",
        "$(id);x",
    ];
    let run = rig.run(&launch_with(&extra), Format::Json);
    data_of(&run);
    let command = argv(&["prog", "a b", "$(id);x"]);
    let runs = rig.fakes.host.runs();
    assert_eq!(runs.len(), 2, "the command, then the server: {runs:?}");
    assert_eq!(
        runs[0],
        (name(PANE), "/srv/demo".to_owned(), command.clone())
    );
    assert_eq!(rig.record(PANE).unwrap().command, Some(command));
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 13a: every grid form reaches Herdr as row 2, column 1, and prints as `r2c1`.
#[test]
fn every_grid_form_reaches_herdr_as_row_2_col_1() {
    for grid in ["c1r2", "r2c1", "2,1"] {
        let text_rig = Rig::new();
        let text = text_rig.run(&launch_replacing("--grid", grid), Format::Text);
        assert_eq!(text.code, 0, "{grid}: {text:?}");
        assert!(text.out.contains("r2c1"), "{grid}: {text:?}");
        assert_eq!(text_rig.record(PANE).unwrap().herdr.grid, R2C1, "{grid}");
        assert!(text_rig.herdr_pane_at(R2C1).is_some(), "{grid}");
        text_rig.assert_matches(&text, PANE);
        let json_rig = Rig::new();
        let data = data_of(&json_rig.run(&launch_replacing("--grid", grid), Format::Json));
        let expected = json!({"row": 2, "col": 1, "pos": "r2c1"});
        assert_eq!(data["pane"]["herdr"]["grid"], expected, "{grid}: {data}");
        for rig in [&text_rig, &json_rig] {
            rig.assert_no_keystroke();
        }
    }
}

/// One AC 17 case: how to set its rig up, its argv and its exit code.
struct Case {
    ac: &'static str,
    setup: fn() -> Rig,
    argv: Vec<&'static str>,
    exit: i32,
}

/// AC 17: each case's JSON run is one valid envelope with nothing on stderr; the exit code is
/// the same in both formats; a failing text run prints exactly `error: <the JSON message>`.
#[test]
fn exit_codes_equal_across_formats() {
    let cases = vec![
        Case {
            ac: "1",
            setup: Rig::new,
            argv: LAUNCH.to_vec(),
            exit: 0,
        },
        Case {
            ac: "8",
            setup: failing_attach,
            argv: LAUNCH.to_vec(),
            exit: 3,
        },
        Case {
            ac: "9a",
            setup: already_live,
            argv: LAUNCH.to_vec(),
            exit: 3,
        },
        Case {
            ac: "10a",
            setup: failing_probe,
            argv: launch_with(&PROBE_FLAGS),
            exit: 3,
        },
        Case {
            ac: "11a",
            setup: Rig::new,
            argv: launch_with(&["--command-json", "\"opencode serve\""]),
            exit: 3,
        },
        Case {
            ac: "13b",
            setup: Rig::new,
            argv: launch_replacing("--grid", "21"),
            exit: 3,
        },
        Case {
            ac: "15",
            setup: Rig::new,
            argv: vec!["pane", "launch", PANE, "--herdr-session", "scratch"],
            exit: 2,
        },
        Case {
            ac: "16e",
            setup: Rig::new,
            argv: launch_with(&["--profile", "nope"]),
            exit: 3,
        },
        Case {
            ac: "6a",
            setup: relaunch_conflict,
            argv: vec!["pane", "relaunch", PANE],
            exit: 1,
        },
    ];
    for case in cases {
        let (text_rig, json_rig) = ((case.setup)(), (case.setup)());
        let text = text_rig.run(&case.argv, Format::Text);
        let json = json_rig.run(&case.argv, Format::Json);
        let ac = case.ac;
        assert_eq!(json.code, case.exit, "AC {ac}: {json:?}");
        assert_eq!(text.code, json.code, "AC {ac}: {text:?} / {json:?}");
        assert_eq!(json.err, "", "AC {ac}: JSON writes nothing on stderr");
        let envelope = check_envelope(&json.out, json.code)
            .unwrap_or_else(|fault| panic!("AC {ac}: {fault}: {json:?}"));
        if let Some(error) = envelope.error {
            assert_eq!(
                text.out, "",
                "AC {ac}: a failing text run prints nothing on stdout"
            );
            assert_eq!(text.err, format!("error: {}\n", error.message), "AC {ac}");
            if ["8", "6a"].contains(&ac) {
                let once = occurrences(&error.message, STEP);
                assert_eq!(once, 1, "AC {ac}: the step once: {}", error.message);
            }
        }
        for rig in [&text_rig, &json_rig] {
            rig.assert_no_keystroke();
        }
    }
}

/// AC 8's setup: the next attach fails with `session-not-found`.
fn failing_attach() -> Rig {
    let rig = Rig::new();
    let missing = PaneError::SessionNotFound {
        what: "ses_gone".into(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::AttachTui, missing);
    rig
}

/// AC 10a's setup: the probe of `PROBE_FLAGS` fails, missing `qwen38`.
fn failing_probe() -> Rig {
    let rig = Rig::new();
    let missing = ProbeResult::Failed {
        missing: vec!["qwen38".to_owned()],
    };
    rig.fakes.prober.script(argv(&PROBE), missing);
    rig
}

/// AC 9a's setup: `demo-c1r1` is already live.
fn already_live() -> Rig {
    let rig = Rig::new();
    rig.live(false);
    rig
}

/// AC 6a's setup: `demo-c1r1` is launched, and another writer moves its record during the
/// next relaunch's act.
fn relaunch_conflict() -> Rig {
    let rig = Rig::new();
    let theirs = Pane {
        role: PaneRole::Orchestrator,
        ..rig.live(false)
    };
    rig.after_server_start(move |f, _| {
        f.panes.concurrent_put(&theirs).unwrap();
    });
    rig
}

/// AC 24 (I5): a run whose budget runs out stops before the next step, rolls back and answers
/// `timeout`, well inside a generous bound. With every harness call 500 ms slow, the plan's
/// two (`serve_command`, the port check) leave too little of 1.3 s for the server step.
#[test]
fn the_budget_bounds_a_slow_launch() {
    let rig = Rig::new();
    let request = launch_request();
    let options = TxOptions {
        budget: Duration::from_millis(1300),
        ..TxOptions::default()
    };
    rig.fakes
        .harness
        .faults()
        .set_delay(Some(Duration::from_millis(500)));
    let started = Instant::now();
    let (answer, calls) = rig.engine(|ports| tx_launch::launch(ports, &request, &options));
    let took = started.elapsed();
    rig.fakes.harness.faults().set_delay(None);
    let timeout = PaneError::Timeout {
        op: "pane.launch".to_owned(),
    };
    assert_eq!(
        answer,
        Err(TxFailure {
            error: timeout,
            acted: true
        })
    );
    assert!(took < Duration::from_secs(4), "bounded: {took:?}");
    assert_eq!(
        calls.harness[..2],
        [HarnessOp::ServeCommand, HarnessOp::Health],
        "the plan's two calls"
    );
    let reached = |op| calls.harness.contains(&op);
    assert!(
        !reached(HarnessOp::VerifyServing) && !reached(HarnessOp::CreateSession),
        "stops before A5: {:?}",
        calls.harness
    );
    let herdr = [
        HerdrOp::Version,
        HerdrOp::Snapshot,
        HerdrOp::EnsurePane,
        HerdrOp::Close,
    ];
    assert_eq!(calls.herdr, herdr);
    assert_eq!(
        rig.herdr_ids(),
        Vec::<PaneId>::new(),
        "the created pane is closed"
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_no_keystroke();
}

/// #808 E11 test 1: a label is not a place. The outside world labels the new pane between its
/// `ensure_pane` answer (no label) and O2's snapshot (`x`); the four fields O2 compares are
/// unchanged, so the launch passes O2 and its record carries no label. A whole-value O2 check
/// fails this run `unavailable`.
#[test]
fn launch_passes_o2_when_only_the_panes_label_changed() {
    let rig = Rig::new();
    rig.after(HarnessOp::AttachTui, |f, c| {
        let pane = c.pane.clone().expect("attach_tui names its pane");
        f.herdr.rename(&pane, "x").unwrap();
    });
    let run = rig.run(LAUNCH, Format::Json);
    data_of(&run);
    let listed = rig.herdr_pane_at(R2C1).expect("a Herdr pane at r2c1");
    assert_eq!(
        listed.label.as_deref(),
        Some("x"),
        "Herdr reports the label"
    );
    let record = rig.record(PANE).expect("a record");
    assert_eq!(record.herdr.pane_id, listed.pane_id);
    assert_eq!(record.herdr.label, None, "the record carries no label");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// #808 E11 test 3: O2 compares the pane's place, not only its id. O2's snapshot listing the
/// pane's id in another cell, another workspace or another session fails the launch
/// `unavailable`, naming the pane, and nothing is recorded (an O2 that compared only the
/// session and pane id would pass the first two). Test 2 is AC 5a's
/// `launch_fails_when_its_herdr_pane_vanishes`, unchanged.
#[test]
fn launch_fails_o2_when_its_pane_is_listed_elsewhere() {
    for how in [Displace::Grid, Displace::Workspace, Displace::Session] {
        let rig = Rig::new();
        rig.displace_after_attach(how);
        let run = rig.run(LAUNCH, Format::Json);
        assert_eq!(run.code, 1, "{how:?}: {run:?}");
        let (code, message) = error_of(&run);
        assert_eq!(code, "unavailable", "{how:?}: {message}");
        assert!(
            message.contains("w1:p1"),
            "{how:?} names the pane: {message}"
        );
        assert_eq!(rig.record(PANE), None, "{how:?}");
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// The rig's own check: a seeded live pane (what relaunch's cases start from) already
/// satisfies `assert_matches`, and P holds its spec, so a relaunch case fails only on relaunch.
#[test]
fn the_rig_seeds_a_pane_the_fakes_agree_with() {
    let rig = Rig::new();
    let record = rig.live(true);
    let run = rig.run(&["pane", "get", PANE], Format::Json);
    data_of(&run);
    rig.assert_matches(&run, PANE);
    assert_eq!(
        rig.profile("demo").panes,
        [launch_spec()],
        "P's spec is LAUNCH's"
    );
    assert_eq!(record.generation, 1);
}

/// Story #670's AC 5, kept: every shared spec flag (and all of them together) parses on
/// `launch` (a missing `PANE` counts as accepted).
#[test]
fn pane_launch_accepts_every_spec_flag() {
    assert_spec_flags_accepted("launch");
}
