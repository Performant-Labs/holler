//! `holler pane doctor` over a Claude Code pane (#831, ADR-0021 section 15 items 6 and 13; plan
//! review W2 and W3): the per-pane chain of a single process has no session list, so a healthy,
//! idle pane is clean; a person's `/clear` is the one mismatch, fixable by a relaunch on the
//! session of record while the pane is idle and never while it works; a missing login is one
//! problem per pass; a wait is a warning with its reason; a gone process is `server-down`.

use holler_cli::output::Format;
use holler_pane::pane::Health;
use holler_pane::ports::LoginStatus;
use holler_pane_testkit::host::HostOp;

use crate::rig::{cell, data_of, findings, has_pair, kinds, Rig, CLAUDE, OPENCODE, PORT};

const DOCTOR: &[&str] = &["pane", "doctor"];

fn doctor(rig: &Rig, extra: &[&str]) -> Vec<serde_json::Value> {
    let mut argv = DOCTOR.to_vec();
    argv.extend_from_slice(extra);
    findings(&data_of(&rig.run(&argv, Format::Json)))
}

/// A mixed grid: one live OpenCode pane and one live, idle Claude pane with a conversation.
fn mixed() -> Rig {
    let rig = Rig::new();
    rig.seed_opencode(OPENCODE, cell(1, 1), PORT);
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    rig
}

/// W3 and the acceptance line "doctor is clean".
#[test]
fn doctor_is_clean_for_a_healthy_idle_claude_pane_beside_an_opencode_pane() {
    let rig = mixed();
    let found = doctor(&rig, &[]);
    assert_eq!(kinds(&found), Vec::<String>::new(), "{found:?}");
    let record = rig.record(CLAUDE);
    assert_eq!(record.harness.health, Health::Healthy);
    assert_eq!(
        record.last_observed.shown, record.session_of_record,
        "observed and recorded"
    );
}

/// W3: a person's `/clear` shows another session in the same process. It is a fixable
/// `shown-driven-mismatch`, and nothing that needs a server's session list is reported.
#[test]
fn a_persons_clear_is_a_fixable_shown_driven_mismatch() {
    let rig = mixed();
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    let cleared = rig.claude().clear(pid).unwrap();

    let found = doctor(&rig, &[]);

    assert_eq!(kinds(&found), vec!["shown-driven-mismatch"], "{found:?}");
    let mismatch = &found[0];
    assert_eq!(mismatch["pane"], CLAUDE);
    assert_eq!(mismatch["session"], cleared.as_str());
    assert_eq!(mismatch["fix"], "fixable");
    assert_eq!(
        mismatch["remedy"],
        format!("holler pane doctor {CLAUDE} --fix")
    );
}

/// W2: `--fix` on an idle Claude pane relaunches it on the session of record, and the pass
/// reports it fixed, with no `observe-failed` from a record write that raced the relaunch.
#[test]
fn fix_relaunches_an_idle_claude_pane_on_its_session_of_record() {
    let rig = mixed();
    let before = rig.record(CLAUDE);
    let (session, old_pid) = (
        before.session_of_record.clone().unwrap(),
        before.harness.pid.unwrap(),
    );
    rig.claude().clear(old_pid).unwrap();
    let mark = rig.mark();

    let found = doctor(&rig, &[CLAUDE, "--fix"]);

    assert_eq!(kinds(&found), vec!["shown-driven-mismatch"], "{found:?}");
    assert_eq!(found[0]["fix"], "fixed", "{found:?}");
    let calls = rig.since(&mark);
    assert!(
        calls.host.contains(&HostOp::StopOwned) && calls.host.contains(&HostOp::Run),
        "{calls:?}"
    );
    let process = rig.process_of(CLAUDE).expect("a process runs");
    assert_ne!(process.pid, old_pid);
    assert_eq!(process.session_id, session, "back on the session of record");
    let after = rig.record(CLAUDE);
    assert_eq!(
        after.session_of_record.as_deref(),
        Some(session.as_str()),
        "never changed"
    );
    assert_eq!(after.harness.pid, Some(process.pid));
    let last = rig.runs_of(CLAUDE).pop().unwrap();
    assert!(has_pair(&last, "--resume", &session), "{last:?}");

    let again = doctor(&rig, &[CLAUDE]);
    assert_eq!(
        kinds(&again),
        Vec::<String>::new(),
        "a second pass is clean: {again:?}"
    );
}

/// W2: an unattended repair never drops a running turn.
#[test]
fn fix_does_not_relaunch_a_busy_claude_pane() {
    let rig = mixed();
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    rig.claude().clear(pid).unwrap();
    rig.claude().stall(pid).unwrap();
    let mark = rig.mark();

    let found = doctor(&rig, &[CLAUDE, "--fix"]);

    let mismatch = found
        .iter()
        .find(|f| f["kind"] == "shown-driven-mismatch")
        .unwrap_or_else(|| panic!("the mismatch is reported: {found:?}"));
    assert_eq!(mismatch["fix"], "not-fixable", "{mismatch}");
    assert_eq!(mismatch["remedy"], format!("holler pane relaunch {CLAUDE}"));
    let calls = rig.since(&mark);
    assert!(
        !calls
            .host
            .iter()
            .any(|op| matches!(op, HostOp::StopOwned | HostOp::Run)),
        "nothing stopped or started: {calls:?}"
    );
    assert_eq!(
        rig.process_of(CLAUDE).map(|p| p.pid),
        Some(pid),
        "the busy process runs on"
    );
}

#[test]
fn a_missing_login_is_one_problem_per_pass() {
    let rig = mixed();
    rig.seed_claude("demo-c2r2", cell(2, 2));
    rig.claude().set_login(LoginStatus::Missing);

    let found = doctor(&rig, &[]);

    let missing: Vec<_> = found
        .iter()
        .filter(|f| f["kind"] == "claude-login-missing")
        .collect();
    assert_eq!(
        missing.len(),
        1,
        "once per pass, whatever the number of Claude panes: {found:?}"
    );
    let text = missing[0].to_string();
    assert!(
        !text.contains("auth status"),
        "the remedy never names a command that reads a login: {text}"
    );
}

#[test]
fn a_fleet_with_no_claude_pane_never_asks_about_the_login() {
    let rig = Rig::new();
    rig.seed_opencode(OPENCODE, cell(1, 1), PORT);
    rig.claude().set_login(LoginStatus::Missing);
    let mark = rig.mark();

    let found = doctor(&rig, &[]);

    assert_eq!(kinds(&found), Vec::<String>::new(), "{found:?}");
    assert_eq!(
        rig.since(&mark).claude,
        vec![],
        "no call reaches the Claude harness"
    );
}

#[test]
fn a_claude_pane_waiting_on_a_permission_is_a_pane_waiting_warning_with_its_reason() {
    let rig = mixed();
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    rig.claude().hold_permission(pid).unwrap();

    let found = doctor(&rig, &[]);

    assert_eq!(kinds(&found), vec!["pane-waiting"], "{found:?}");
    assert_eq!(found[0]["pane"], CLAUDE);
    let message = found[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("permission prompt"),
        "the reason is named: {message}"
    );
}

#[test]
fn a_claude_process_that_is_gone_is_server_down() {
    let rig = mixed();
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    rig.claude().sigint(pid).unwrap();

    let found = doctor(&rig, &[]);

    let down: Vec<_> = found
        .iter()
        .filter(|f| f["kind"] == "server-down")
        .collect();
    assert_eq!(down.len(), 1, "{found:?}");
    assert_eq!(down[0]["pane"], CLAUDE);
    assert_eq!(down[0]["remedy"], format!("holler pane relaunch {CLAUDE}"));
    assert!(
        !found.iter().any(|f| f["kind"] == "observe-failed"),
        "nothing failed to observe: {found:?}"
    );
}
