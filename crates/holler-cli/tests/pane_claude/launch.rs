//! `holler pane launch --harness claude` (#831, ADR-0021 section 15 items 6, 10, 11 and 12; plan
//! review W10, W11 and W13): a Claude Code pane is one process in the pane's tmux session, started
//! on a fresh session, recorded with its pid and no port; every refusal of the plan writes nothing
//! and starts nothing.

use holler_cli::output::Format;
use holler_pane::pane::{HarnessKind, Health, ModelSpec};
use holler_pane::{HerdrPort, HostPort};
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;

use crate::rig::{
    cell, data_of, error_of, has_pair, launch_claude, name, replacing, Calls, Rig, CLAUDE, CWD,
    LOGIN_VARIABLES, PROFILE,
};

/// A refused plan: exit 3 with `code`, nothing live touched, no record, no process.
fn assert_refused_before_anything(
    rig: &Rig,
    run: &crate::verb_harness::Outcome,
    calls: &Calls,
    code: &str,
) {
    assert_eq!(run.code, 3, "{run:?}");
    let (got, _) = error_of(run);
    assert_eq!(got, code, "{run:?}");
    assert!(calls.acted_on_nothing_live(), "nothing live: {calls:?}");
    assert!(calls.wrote_no_record(), "no write: {calls:?}");
    assert!(rig.maybe_record(CLAUDE).is_none(), "no record");
    assert!(rig.process_of(CLAUDE).is_none(), "no Claude process");
}

#[test]
fn a_claude_launch_records_a_pid_no_port_and_a_fresh_session_of_record() {
    let rig = Rig::new();
    let (run, calls) = rig.run_logged(&launch_claude(CLAUDE, "r1c2", &[]), Format::Json);
    let data = data_of(&run);
    assert_eq!(data["pane"]["harness"]["kind"], "claude", "{data}");
    assert!(
        data["pane"]["harness"].get("port").is_none(),
        "no port key: {data}"
    );

    let record = rig.record(CLAUDE);
    assert_eq!(record.harness.kind, HarnessKind::Claude);
    assert_eq!(record.harness.port, None);
    assert_eq!(record.harness.health, Health::Healthy);
    let session = record
        .session_of_record
        .clone()
        .expect("a session of record");
    assert_eq!(
        record.last_observed.shown.as_deref(),
        Some(session.as_str())
    );

    let process = rig
        .process_of(CLAUDE)
        .expect("the pane's Claude process runs");
    assert_eq!(
        record.harness.pid,
        Some(process.pid),
        "the pid recorded is the process's"
    );
    assert_eq!(
        process.session_id, session,
        "the process holds the session of record"
    );

    let runs = rig.runs_of(CLAUDE);
    assert!(
        runs.iter()
            .any(|argv| has_pair(argv, "--session-id", &session)),
        "the host ran the process on the fresh session: {runs:?}"
    );
    assert_eq!(
        calls.opencode,
        vec![],
        "no server, session or TUI call: {calls:?}"
    );
    assert!(
        !calls
            .herdr
            .iter()
            .any(|op| matches!(op, HerdrOp::SendText | HerdrOp::SendKeys)),
        "nothing is typed into the pane (I4)"
    );
    assert!(
        rig.herdr
            .snapshot()
            .unwrap()
            .panes
            .iter()
            .any(|p| p.pane_id == record.herdr.pane_id),
        "the Herdr pane exists"
    );
}

#[test]
fn the_launch_passes_the_specs_model_and_directory_to_the_process_argv() {
    let rig = Rig::new();
    let run = rig.run(&launch_claude(CLAUDE, "r1c2", &[]), Format::Json);
    assert_eq!(run.code, 0, "{run:?}");
    let specs = rig.claude().process_specs();
    let last = specs.last().expect("process_command was asked");
    assert_eq!(
        last.model,
        Some(ModelSpec {
            provider: "anthropic".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "high".to_owned(),
        })
    );
    assert_eq!(last.cwd.as_deref(), Some(CWD));
    assert_eq!(last.agent, None);
}

#[test]
fn a_launch_into_a_profile_stores_the_harness_and_the_policy_none() {
    let rig = Rig::new();
    let run = rig.run(
        &launch_claude(CLAUDE, "r1c2", &["--profile", PROFILE]),
        Format::Json,
    );
    assert_eq!(run.code, 0, "{run:?}");
    let spec = rig
        .profile()
        .panes
        .into_iter()
        .find(|s| s.pane == CLAUDE)
        .expect("the profile's spec for the pane");
    assert_eq!(spec.harness.kind, HarnessKind::Claude);
    assert_eq!(spec.harness.port_policy, "none");
}

#[test]
fn a_claude_launch_with_a_port_is_port_policy_unsupported() {
    let rig = Rig::new();
    let argv = replacing(
        launch_claude(CLAUDE, "r1c2", &[]),
        "--port-policy",
        "fixed:48100",
    );
    let (run, calls) = rig.run_logged(&argv, Format::Json);
    assert_refused_before_anything(&rig, &run, &calls, "port-policy-unsupported");
}

#[test]
fn an_opencode_launch_with_the_policy_none_is_port_policy_unsupported() {
    let rig = Rig::new();
    let argv = replacing(launch_claude(CLAUDE, "r1c2", &[]), "--harness", "opencode");
    let (run, calls) = rig.run_logged(&argv, Format::Json);
    assert_refused_before_anything(&rig, &run, &calls, "port-policy-unsupported");
}

#[test]
fn an_unknown_harness_is_usage() {
    let rig = Rig::new();
    let argv = replacing(launch_claude(CLAUDE, "r1c2", &[]), "--harness", "codex");
    let (run, calls) = rig.run_logged(&argv, Format::Json);
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(error_of(&run).0, "usage");
    assert!(
        calls.acted_on_nothing_live() && calls.wrote_no_record(),
        "{calls:?}"
    );
}

#[test]
fn a_claude_launch_with_an_agent_is_agent_unsupported() {
    let rig = Rig::new();
    let argv = launch_claude(CLAUDE, "r1c2", &["--agent", "reviewer"]);
    let (run, calls) = rig.run_logged(&argv, Format::Json);
    assert_refused_before_anything(&rig, &run, &calls, "agent-unsupported");
}

#[test]
fn a_login_variable_in_the_specs_env_is_refused_by_its_name() {
    for variable in LOGIN_VARIABLES {
        let rig = Rig::new();
        let argv = launch_claude(CLAUDE, "r1c2", &["--env", variable]);
        let (run, calls) = rig.run_logged(&argv, Format::Json);
        assert_refused_before_anything(&rig, &run, &calls, "claude-login-env-refused");
        let (_, message) = error_of(&run);
        assert!(
            message.contains(variable),
            "the message names {variable}: {message}"
        );
    }
}

#[test]
fn an_ordinary_env_name_is_launched_with() {
    let rig = Rig::new();
    let run = rig.run(
        &launch_claude(CLAUDE, "r1c2", &["--env", "DEMO_URL"]),
        Format::Json,
    );
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        rig.record(CLAUDE)
            .env
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["DEMO_URL"]
    );
}

/// W11: a crash can leave a `claude` running in the pane's tmux session with no record. A launch
/// there would start a second one while the first keeps running (and spending), so it is refused,
/// as `port-in-use` refuses a server left behind.
#[test]
fn a_launch_into_a_session_that_already_runs_claude_is_process_in_use() {
    let rig = Rig::new();
    let pane = name(CLAUDE);
    rig.host.ensure_session(&pane, CWD).unwrap();
    rig.host
        .run(
            &pane,
            CWD,
            &holler_pane::Argv::new(vec!["claude".to_owned()]),
        )
        .unwrap();
    let ran_before = rig.host.runs().len();

    let (run, calls) = rig.run_logged(&launch_claude(CLAUDE, "r1c2", &[]), Format::Json);

    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "process-in-use", "{run:?}");
    assert!(calls.wrote_no_record(), "{calls:?}");
    assert!(
        !calls
            .host
            .iter()
            .any(|op| matches!(op, HostOp::Run | HostOp::StopOwned)),
        "nothing started or stopped: {calls:?}"
    );
    assert_eq!(rig.host.runs().len(), ran_before);
    assert!(rig.process_of(CLAUDE).is_none(), "no second Claude process");
    assert!(rig.maybe_record(CLAUDE).is_none());
}

#[test]
fn a_text_launch_prints_one_line_naming_the_pane() {
    let rig = Rig::new();
    let run = rig.run(&launch_claude(CLAUDE, "r1c2", &[]), Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.out.contains(CLAUDE), "{run:?}");
    for variable in LOGIN_VARIABLES {
        assert!(
            !run.out.contains(variable) && !run.err.contains(variable),
            "{run:?}"
        );
    }
}

/// Section 15 item 6: `pane get` of a Claude Code pane shows its pid and no port: the text line
/// `harness: claude port=- pid=<pid>`, and in JSON a harness of kind `claude` with the pid and no
/// `port` key.
#[test]
fn pane_get_of_a_claude_pane_shows_its_pid_and_no_port() {
    let rig = Rig::new();
    let record = rig.seed_claude(CLAUDE, cell(1, 2));
    let pid = record.harness.pid.expect("a Claude record has a pid");

    let text = rig.run(&["pane", "get", CLAUDE], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    let line = format!("harness: claude port=- pid={pid}");
    assert!(text.out.lines().any(|l| l == line), "`{line}`: {text:?}");

    let json = rig.run(&["pane", "get", CLAUDE], Format::Json);
    let harness = &data_of(&json)["pane"]["harness"];
    assert_eq!(harness["kind"], "claude", "{harness}");
    assert_eq!(harness["pid"], pid, "{harness}");
    assert!(harness.get("port").is_none(), "no port key: {harness}");
}
