//! #802: the harness server of a launch and a relaunch. `HarnessPort::serve_command` builds
//! the server's argv in the plan (pure, from a `ServeSpec` carrying the in-flight spec's
//! agent); the act runs that argv in the pane's tmux session through `HostPort::run`, in the
//! spec's directory, waits for the server's health (`TxOptions.boot_poll`), and asks
//! `verify_serving` before the session of record exists. Every case runs over launch's rig,
//! whose linked host starts the fake server an argv of `serve_command` describes.
//!
//! Cross-port order is read from the fakes' own logs at a hooked step (plan review W-8): the
//! rig keeps one call log per port and no merged one.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use holler_cli::output::Format;
use holler_pane::ports::AGENT_UNSUPPORTED;
use holler_pane::tx_launch::{self, TxFailure, TxOptions};
use holler_pane::{HarnessPort, HostPort, PaneError, PaneId, ProbeResult, ServeSpec};
use holler_pane_testkit::harness::{HarnessOp, ServerState};
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;

use super::rig::{
    agent, argv, data_of, error_of, launch_with, name, occurrences, Called, Rig, PANE, PORT, PROBE,
    PROBE_FLAGS, STEP,
};
use super::{launch_request, steps};
use crate::list::Rig as Fakes;

/// The phrase every reconcile step starts with.
const STEP_PHRASE: &str = "to reconcile, run";

/// The agent the cases ask for.
const AGENT: &str = "feature-implementor";

/// The spec of `--agent AGENT`.
fn with_agent(key: &str) -> ServeSpec {
    ServeSpec::default().with_agent(agent(key))
}

/// The refusal a harness answers when it cannot apply an agent.
fn agent_unsupported() -> PaneError {
    PaneError::Refused {
        code: AGENT_UNSUPPORTED,
        message: "this harness cannot apply an OpenCode agent yet".to_owned(),
    }
}

/// How many calls of `op` `ops` holds.
fn count(ops: &[HarnessOp], op: HarnessOp) -> usize {
    ops.iter().filter(|each| **each == op).count()
}

/// AC 2: an `agent-unsupported` refusal from `serve_command` comes from the plan: exit 3, no
/// reconcile step, nothing live called but `serve_command` itself, no write, and P keeps its
/// generation.
#[test]
fn an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing() {
    for format in [Format::Json, Format::Text] {
        let rig = Rig::new();
        rig.fakes
            .harness
            .faults()
            .fail_next(HarnessOp::ServeCommand, agent_unsupported());
        let argv = launch_with(&["--profile", "demo", "--agent", AGENT]);
        let run = rig.run(&argv, format);
        assert_eq!(run.code, 3, "{format:?}: {run:?}");
        let message = if format == Format::Json {
            let (code, message) = error_of(&run);
            assert_eq!(code, "agent-unsupported");
            message
        } else {
            run.err.clone()
        };
        assert_eq!(occurrences(&message, STEP_PHRASE), 0, "no step: {message}");
        assert_eq!(run.calls.harness, [HarnessOp::ServeCommand]);
        assert_eq!(run.calls.herdr, Vec::<HerdrOp>::new(), "no Herdr call");
        assert_eq!(run.calls.host, Vec::<HostOp>::new(), "no host call");
        assert!(run.calls.probes.is_empty(), "no probe");
        assert!(!run.calls.panes.contains(&PaneStoreOp::CasPut));
        assert!(!run.calls.profiles.contains(&ProfileStoreOp::CasPut));
        assert_eq!(rig.profile("demo").generation, 1, "P keeps its generation");
        assert_eq!(rig.record(PANE), None);
        assert_eq!(rig.herdr_ids(), Vec::<PaneId>::new());
        rig.assert_matches(&run, PANE);
        rig.assert_no_keystroke();
    }
}

/// AC 2: on a relaunch the refusal is the plan's too, so nothing of the running pane is
/// stopped: its server runs on and its record is unchanged.
#[test]
fn an_agent_refusal_in_the_plan_leaves_a_relaunched_pane_running() {
    let rig = Rig::new();
    let before = rig.live(false);
    let server = rig.fakes.harness.server(PORT).unwrap();
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::ServeCommand, agent_unsupported());
    let run = rig.run(&["pane", "relaunch", PANE, "--agent", AGENT], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "agent-unsupported");
    assert_eq!(occurrences(&message, STEP_PHRASE), 0, "{message}");
    assert_eq!(run.calls.host, Vec::<HostOp>::new(), "nothing was stopped");
    assert_eq!(rig.fakes.harness.server(PORT), Some(server), "it runs on");
    assert_eq!(rig.record(PANE), Some(before));
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// What the fakes had logged when a hook fired.
#[derive(Debug, Clone, Default)]
struct Seen {
    herdr: Vec<HerdrOp>,
    host: Vec<HostOp>,
    harness: Vec<HarnessOp>,
    probes: usize,
}

/// A hook that keeps what the fakes had logged when it fired.
fn snapshot_into(seen: &Arc<Mutex<Option<Seen>>>) -> impl FnOnce(&Fakes, &Called) + Send {
    let seen = Arc::clone(seen);
    move |f, _| {
        *seen.lock().unwrap() = Some(Seen {
            herdr: f.herdr.faults().calls(),
            host: f.host.faults().calls(),
            harness: f.harness.faults().calls(),
            probes: f.prober.calls().len(),
        });
    }
}

fn taken(seen: &Arc<Mutex<Option<Seen>>>, what: &str) -> Seen {
    seen.lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| panic!("the hook at {what} fired"))
}

/// AC 2: `serve_command` is the plan's first call: before the probe, Herdr's version and
/// snapshot, the port check and every host call.
#[test]
fn serve_command_runs_in_the_plan_before_any_other_call() {
    let rig = Rig::new();
    rig.fakes.prober.script(argv(&PROBE), ProbeResult::Ok);
    let at_command = Arc::new(Mutex::new(None));
    rig.after(HarnessOp::ServeCommand, snapshot_into(&at_command));
    let run = rig.run(&launch_with(&PROBE_FLAGS), Format::Json);
    data_of(&run);
    // A fresh rig: every call logged when the hook fired is the run's own (the rig's own
    // reads before a run are Herdr snapshots, never `version`).
    let seen = taken(&at_command, "serve_command");
    assert!(!seen.herdr.contains(&HerdrOp::Version), "{seen:?}");
    assert_eq!(seen.host, Vec::<HostOp>::new(), "{seen:?}");
    assert_eq!(seen.probes, 0, "{seen:?}");
    assert_eq!(seen.harness, [HarnessOp::ServeCommand], "{seen:?}");
    assert_eq!(run.calls.probes.len(), 1, "the probe ran after it");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 3 and AC 4: the in-flight spec's agent reaches `serve_command`; the act ensures the
/// session, runs the spec's command, then exactly the argv the plan built, both in the
/// spec's directory, waits for the server's health, verifies the spec, and only then creates
/// the session of record and attaches the TUI.
#[test]
fn the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies() {
    let rig = Rig::new();
    let (at_start, at_verify) = (Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None)));
    rig.after_server_start(snapshot_into(&at_start));
    rig.after(HarnessOp::VerifyServing, snapshot_into(&at_verify));
    let extra = [
        "--agent",
        AGENT,
        "--command-arg",
        "sleep",
        "--command-arg",
        "600",
    ];
    let run = rig.run(&launch_with(&extra), Format::Json);
    data_of(&run);

    let start = taken(&at_start, "the server's start");
    let host = [HostOp::EnsureSession, HostOp::Run, HostOp::Run];
    assert_eq!(
        start.host, host,
        "the session, the command, then the server"
    );
    assert_eq!(
        steps(&start.harness),
        [HarnessOp::ServeCommand, HarnessOp::Health],
        "nothing after the plan's reads (and the port's re-check) yet"
    );
    let verify = taken(&at_verify, "verify_serving");
    assert!(
        count(&verify.harness, HarnessOp::Health) > count(&start.harness, HarnessOp::Health),
        "the health wait ran between the start and the verify: {verify:?}"
    );
    assert_eq!(
        steps(&verify.harness),
        [
            HarnessOp::ServeCommand,
            HarnessOp::Health,
            HarnessOp::VerifyServing
        ]
    );
    let all = [
        HarnessOp::ServeCommand,
        HarnessOp::Health,
        HarnessOp::VerifyServing,
        HarnessOp::CreateSession,
        HarnessOp::AttachTui,
        HarnessOp::ShownSession,
    ];
    assert_eq!(steps(&run.calls.harness), all);

    let spec = with_agent(AGENT);
    assert_eq!(rig.fakes.harness.serve_specs(), std::slice::from_ref(&spec));
    let runs = rig.fakes.host.runs();
    let planned = rig
        .fakes
        .harness
        .serve_command(&name(PANE), PORT, &spec)
        .unwrap();
    let expected = vec![
        (name(PANE), "/srv/demo".to_owned(), argv(&["sleep", "600"])),
        (name(PANE), "/srv/demo".to_owned(), planned),
    ];
    assert_eq!(
        runs, expected,
        "exactly the planned argv, in the spec's cwd"
    );
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(server.agent, Some(agent(AGENT)), "{server:?}");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 3: a relaunch passes the in-flight spec's agent: a `--agent` that differs from the
/// recorded one reaches `serve_command` (the fake recorded the new key), and without the flag
/// the recorded key does.
#[test]
fn relaunch_passes_the_in_flight_agent_to_serve_command() {
    let rig = Rig::new();
    let mut theirs = rig.live(false);
    theirs.opencode_agent = Some(agent("orchestrator"));
    rig.fakes.panes.concurrent_put(&theirs).unwrap();

    let run = rig.run(&["pane", "relaunch", PANE, "--agent", AGENT], Format::Json);
    data_of(&run);
    let asked = rig.fakes.harness.serve_specs();
    assert_eq!(asked.last(), Some(&with_agent(AGENT)), "{asked:?}");
    let server = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(server.agent, Some(agent(AGENT)));
    rig.assert_matches(&run, PANE);

    let run = rig.run(&["pane", "relaunch", PANE], Format::Json);
    data_of(&run);
    let asked = rig.fakes.harness.serve_specs();
    assert_eq!(asked.last(), Some(&with_agent(AGENT)), "the stored key");
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 4 (plan review W-7): a relaunch into a tmux session made for another directory runs the
/// server in the pane spec's directory, not the session's; the session keeps its own.
#[test]
fn a_relaunch_runs_the_server_in_the_spec_directory_not_the_sessions() {
    let rig = Rig::new();
    let record = rig.live(false);
    let tmux = name(PANE);
    rig.fakes.host.end_session(&tmux).unwrap();
    rig.fakes
        .host
        .ensure_session(&tmux, "/srv/elsewhere")
        .unwrap();
    let before = rig.fakes.host.runs().len();
    let run = rig.run(&["pane", "relaunch", PANE], Format::Json);
    data_of(&run);
    let runs = rig.fakes.host.runs();
    let new = &runs[before..];
    assert_eq!(new.len(), 1, "the server only: {new:?}");
    assert_eq!(new[0].1, record.host.cwd, "the spec's directory");
    assert_ne!(new[0].1, "/srv/elsewhere");
    assert_eq!(
        rig.fakes.host.cwd(&tmux).as_deref(),
        Some("/srv/elsewhere"),
        "the session keeps the directory it was made with"
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 6: the health wait polls every `TxOptions.boot_poll`, 150 ms by default.
#[test]
fn boot_poll_defaults_to_150_ms() {
    assert_eq!(TxOptions::default().boot_poll, Duration::from_millis(150));
}

/// AC 6: the wait is a wait: a server that does not answer its first poll but answers a later
/// one inside the budget is launched.
#[test]
fn the_health_wait_retries_until_the_server_answers() {
    let rig = Rig::new();
    rig.after_server_start(|f, c| f.harness.freeze(c.port.unwrap()).unwrap());
    // Each hook fires once, after a `health` call: the first one that finds the server frozen
    // has seen it not answer, and thaws it for the next poll.
    for _ in 0..6 {
        rig.after(HarnessOp::Health, |f, _| {
            if f.harness.server(PORT).map(|s| s.state) == Some(ServerState::Frozen) {
                f.harness.thaw(PORT).unwrap();
            }
        });
    }
    let run = rig.run(&launch_with(&[]), Format::Json);
    data_of(&run);
    let polls = count(&run.calls.harness, HarnessOp::Health);
    assert!(
        polls >= 3,
        "the plan's check and two polls: {:?}",
        run.calls.harness
    );
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 6 and Scope 2 (I5, the brief as amended in round 1): the health wait has no budget of
/// its own, and each of its `health` calls is a live step with the run's budget checked before
/// it. A server run that returns after the budget has run out (every host call 400 ms slow
/// from the plan's port check on, against a 600 ms budget: `ensure_session` ends inside it,
/// `run` past it) is never asked: the run is the budget's `timeout`, and the only `health`
/// calls are the plan's port check and the act's re-check before the run.
#[test]
fn no_health_poll_starts_once_the_budget_has_run_out() {
    let rig = Rig::new();
    rig.after(HarnessOp::Health, |f, _| {
        f.host.faults().set_delay(Some(Duration::from_millis(400)));
    });
    let options = TxOptions {
        budget: Duration::from_millis(600),
        ..TxOptions::default()
    };
    let (answer, calls) = rig.engine(|ports| tx_launch::launch(ports, &launch_request(), &options));
    rig.fakes.host.faults().set_delay(None);
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
    assert_eq!(
        calls.host,
        [HostOp::EnsureSession, HostOp::Run, HostOp::StopOwned],
        "the server ran, then the rollback"
    );
    assert_eq!(
        calls.harness,
        [
            HarnessOp::ServeCommand,
            HarnessOp::Health,
            HarnessOp::Health
        ],
        "no health call after the run: the budget had run out"
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_no_keystroke();
}

/// AC 7: a `verify_serving` refusal after a healthy start goes through the rollback: the
/// session's processes are stopped (the server among them), the Herdr pane the run made is
/// closed, no session of record is created and nothing is recorded; the act had begun, so the
/// reconcile step is printed once.
#[test]
fn a_verify_refusal_rolls_back_and_stops_the_server() {
    let rig = Rig::new();
    let refusal = PaneError::Refused {
        code: AGENT_UNSUPPORTED,
        message: "the running server does not apply the agent".to_owned(),
    };
    rig.fakes
        .harness
        .faults()
        .fail_next(HarnessOp::VerifyServing, refusal);
    let run = rig.run(&launch_with(&["--agent", AGENT]), Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "agent-unsupported");
    assert_eq!(occurrences(&message, STEP_PHRASE), 1, "{message}");
    assert!(message.contains(STEP), "{message}");
    assert!(
        run.calls.host.contains(&HostOp::StopOwned),
        "{:?}",
        run.calls.host
    );
    assert!(!run.calls.harness.contains(&HarnessOp::CreateSession));
    assert!(
        run.calls.herdr.contains(&HerdrOp::Close),
        "{:?}",
        run.calls.herdr
    );
    let server = rig
        .fakes
        .harness
        .server(PORT)
        .expect("the server had started");
    assert_eq!(server.state, ServerState::Killed, "the server is stopped");
    assert_eq!(rig.record(PANE), None);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// Plan review W-5: a server that answers on the port after the plan checked it (another
/// process took the port) is refused just before the server's run, after the spec's command:
/// `port-in-use`, from the act, so the run rolls back and prints the reconcile step. That
/// server is never adopted and never stopped, and no server argv is run.
#[test]
fn the_port_is_checked_again_just_before_the_server_runs() {
    let rig = Rig::new();
    rig.after(HarnessOp::Health, |f, _| {
        let other = name("demo-c2r1");
        let argv = f
            .harness
            .serve_command(&other, PORT, &ServeSpec::default())
            .unwrap();
        f.harness.start_server(&argv).unwrap().unwrap();
    });
    let run = rig.run(
        &launch_with(&["--command-arg", "sleep", "--command-arg", "600"]),
        Format::Json,
    );
    assert_eq!(run.code, 3, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "port-in-use");
    assert_eq!(occurrences(&message, STEP_PHRASE), 1, "{message}");
    let runs = rig.fakes.host.runs();
    assert_eq!(
        runs,
        vec![(name(PANE), "/srv/demo".to_owned(), argv(&["sleep", "600"]))],
        "the command ran, the server did not"
    );
    let theirs = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(theirs.name.as_str(), "demo-c2r1");
    assert_eq!(
        theirs.state,
        ServerState::Running,
        "never adopted, never stopped"
    );
    assert!(!run.calls.harness.contains(&HarnessOp::VerifyServing));
    assert_eq!(
        rig.herdr_ids(),
        Vec::<PaneId>::new(),
        "the pane it made is closed"
    );
    assert_eq!(rig.record(PANE), None);
    rig.assert_no_keystroke();
}

/// AC 8: a relaunch of a pane whose server runs stops it (`stop_owned`, before anything is
/// started), and the new server comes up on the same port: the old one is gone.
#[test]
fn relaunch_stops_the_old_server_and_starts_one_on_the_same_port() {
    let rig = Rig::new();
    rig.live(false);
    let old = rig.fakes.harness.server(PORT).unwrap();
    let run = rig.run(&["pane", "relaunch", PANE], Format::Json);
    data_of(&run);
    assert_eq!(
        run.calls.host,
        [HostOp::StopOwned, HostOp::EnsureSession, HostOp::Run],
        "stop, then the session and the server"
    );
    let new = rig.fakes.harness.server(PORT).unwrap();
    assert_eq!(new.state, ServerState::Running);
    assert_ne!(new.pid, old.pid, "a new server, the old one gone");
    assert_eq!(rig.record(PANE).unwrap().harness.port, PORT);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}

/// AC 9: the record a launch writes has no `harness.pid` key on the wire or at rest.
#[test]
fn a_launched_record_has_no_pid_key() {
    let rig = Rig::new();
    let run = rig.run(&launch_with(&[]), Format::Json);
    let data = data_of(&run);
    assert!(
        data["pane"]["harness"].get("pid").is_none(),
        "the reply: {data}"
    );
    let stored = serde_json::to_value(rig.record(PANE).unwrap()).unwrap();
    let harness = stored["harness"].as_object().unwrap();
    assert!(!harness.contains_key("pid"), "the record: {stored}");
    assert_eq!(harness["port"], PORT);
    rig.assert_matches(&run, PANE);
    rig.assert_no_keystroke();
}
