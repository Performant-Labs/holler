#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! `FakeClaude` (#831, ADR-0021 section 15 item 15): what only the fake has, beyond the
//! conformance suite it passes (`claude_conformance_test.rs`). Its processes are modelled on
//! the spike (`docs/research/claude-code-pane-spike.md`): a process holds one session, a
//! transcript exists only once the session was first prompted, `--session-id` of a session that
//! is held or has a transcript is `session-in-use`, a message to a process Holler did not start
//! (no `crossSessionInbound: accept`) is held behind a dialog, a person's `/clear` moves the
//! process to a new session and keeps its pid, and a stopped process leaves its transcript.

use holler_pane::ports::{Activity, LoginStatus, SessionChoice};
use holler_pane::{AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneName, ServeSpec};
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::claude::{ClaudeStatus, FakeClaude};
use holler_pane_testkit::fault::PortOp;

const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn spec() -> ServeSpec {
    ServeSpec::default().with_cwd("/srv/demo")
}

fn endpoint(pane: &str, pid: u32) -> HarnessEndpoint {
    HarnessEndpoint::Process {
        pane: name(pane),
        pid,
    }
}

/// Whether `argv` holds `flag` immediately followed by `value`.
fn has_pair(argv: &Argv, flag: &str, value: &str) -> bool {
    argv.as_slice()
        .windows(2)
        .any(|w| w[0] == flag && w[1] == value)
}

/// Launch a process for `pane` on `choice` through the fake's own argv; its pid.
fn launch(fake: &FakeClaude, pane: &str, choice: SessionChoice) -> u32 {
    let argv = fake.process_command(&name(pane), &spec(), &choice).unwrap();
    fake.start(&argv).expect("the fake's own argv").unwrap()
}

#[test]
fn a_fresh_launch_holds_its_session_and_is_idle() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let argv = fake
        .process_command(&name(C1), &spec(), &SessionChoice::Fresh(id.clone()))
        .unwrap();
    assert!(has_pair(&argv, "--session-id", &id), "{argv:?}");
    assert!(
        fake.processes().is_empty(),
        "process_command starts nothing"
    );

    let pid = fake.start(&argv).expect("the fake's own argv").unwrap();
    let view = fake.process(pid).expect("the started process");
    assert_eq!(view.pane, name(C1));
    assert_eq!(view.session_id, id);
    assert_eq!(view.status, ClaudeStatus::Idle);
    assert_eq!(view.waiting_for, None);
    assert!(
        view.accept_inbound,
        "Holler's launch accepts cross-session messages"
    );
    assert!(view.inbox.is_empty());
    assert_eq!(fake.locate_process(&name(C1), &id).unwrap(), Some(pid));
    assert!(
        !fake.has_conversation(&id),
        "no transcript until the first prompt"
    );
}

#[test]
fn start_ignores_an_argv_it_did_not_build() {
    let fake = FakeClaude::new();
    let other = Argv::new(vec!["opencode".to_owned(), "serve".to_owned()]);
    assert!(fake.start(&other).is_none());
    assert!(fake.processes().is_empty());
}

#[test]
fn a_session_held_by_a_running_process_is_in_use() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let argv = fake
        .process_command(&name(C1), &spec(), &SessionChoice::Fresh(id.clone()))
        .unwrap();
    fake.start(&argv).unwrap().unwrap();
    let again = fake.start(&argv).expect("the fake's own argv");
    assert_eq!(again.unwrap_err().code(), "session-in-use");
    assert_eq!(fake.processes().len(), 1, "the refused start runs nothing");
}

#[test]
fn session_id_of_a_session_that_has_a_conversation_is_in_use() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    fake.seed_conversation(&id);
    let argv = fake
        .process_command(&name(C1), &spec(), &SessionChoice::Fresh(id.clone()))
        .unwrap();
    assert!(has_pair(&argv, "--session-id", &id), "{argv:?}");
    let started = fake.start(&argv).expect("the fake's own argv");
    assert_eq!(started.unwrap_err().code(), "session-in-use");
}

#[test]
fn keep_resumes_a_session_that_has_a_conversation() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    fake.seed_conversation(&id);
    let argv = fake
        .process_command(&name(C1), &spec(), &SessionChoice::Keep(id.clone()))
        .unwrap();
    assert!(has_pair(&argv, "--resume", &id), "{argv:?}");
    let pid = fake.start(&argv).unwrap().unwrap();
    assert_eq!(fake.process(pid).unwrap().session_id, id);
}

#[test]
fn the_first_delivered_message_starts_the_conversation() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id.clone()));
    fake.send_prompt(&endpoint(C1, pid), &id, "hello from the hub")
        .unwrap();
    assert_eq!(fake.process(pid).unwrap().inbox, vec!["hello from the hub"]);
    assert!(fake.has_conversation(&id));
}

#[test]
fn a_message_to_a_process_holler_did_not_start_is_held() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = fake.run_by_hand(&name(C1), &id);
    assert!(!fake.process(pid).unwrap().accept_inbound);
    let refused = fake
        .send_prompt(&endpoint(C1, pid), &id, "hello")
        .unwrap_err();
    assert_eq!(refused.code(), "inbound-held");
    assert!(
        fake.process(pid).unwrap().inbox.is_empty(),
        "nothing was delivered"
    );
    assert_eq!(
        fake.session_activity(&endpoint(C1, pid), &id).unwrap(),
        Activity::WaitingPermission,
        "the held message waits behind a dialog"
    );
}

#[test]
fn stop_ends_the_panes_processes_and_keeps_their_conversations() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id.clone()));
    let other = fake.new_session_id().unwrap();
    let kept = launch(&fake, C2, SessionChoice::Fresh(other));
    fake.send_prompt(&endpoint(C1, pid), &id, "hello").unwrap();

    fake.stop(&name(C1));

    assert!(fake.process(pid).is_none(), "the pane's process is gone");
    assert!(
        fake.process(kept).is_some(),
        "another pane's process runs on"
    );
    assert!(!fake.health(&endpoint(C1, pid)).unwrap());
    assert_eq!(fake.locate_process(&name(C1), &id).unwrap(), None);
    assert!(
        fake.has_conversation(&id),
        "the transcript stays, so a relaunch can resume"
    );
}

#[test]
fn the_faults_set_the_activity_agents_reports() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id.clone()));
    let at = endpoint(C1, pid);

    fake.stall(pid).unwrap();
    assert_eq!(fake.session_activity(&at, &id).unwrap(), Activity::Busy);

    fake.hold_permission(pid).unwrap();
    assert_eq!(
        fake.session_activity(&at, &id).unwrap(),
        Activity::WaitingPermission
    );
    assert_eq!(
        fake.process(pid).unwrap().waiting_for.as_deref(),
        Some("permission prompt")
    );

    fake.hold_question(pid).unwrap();
    assert_eq!(
        fake.session_activity(&at, &id).unwrap(),
        Activity::HoldingQuestion
    );
    assert_eq!(
        fake.process(pid).unwrap().waiting_for.as_deref(),
        Some("input needed")
    );

    fake.set_idle(pid).unwrap();
    assert_eq!(fake.session_activity(&at, &id).unwrap(), Activity::Idle);
    assert_eq!(fake.process(pid).unwrap().waiting_for, None);
}

#[test]
fn sigint_ends_the_process() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id));
    fake.sigint(pid).unwrap();
    assert!(
        fake.process(pid).is_none(),
        "SIGINT ends the process: it is no abort"
    );
    assert!(!fake.health(&endpoint(C1, pid)).unwrap());
}

#[test]
fn a_persons_clear_moves_the_shown_session_and_keeps_the_pid() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id.clone()));
    let at = endpoint(C1, pid);
    let pane_id = holler_pane::PaneId::new(format!("scratch:{C1}"));

    let cleared = fake.clear(pid).unwrap();

    assert_ne!(cleared, id);
    assert_eq!(
        fake.process(pid).unwrap().session_id,
        cleared,
        "the same process"
    );
    assert_eq!(
        fake.shown_session(&pane_id, &at).unwrap(),
        Some(cleared.clone())
    );
    assert_eq!(fake.locate_process(&name(C1), &id).unwrap(), None);
    assert_eq!(fake.locate_process(&name(C1), &cleared).unwrap(), Some(pid));
}

#[test]
fn login_status_is_ok_until_the_login_goes_missing() {
    let fake = FakeClaude::new();
    assert_eq!(fake.login_status().unwrap(), LoginStatus::Ok);
    fake.set_login(LoginStatus::Missing);
    assert_eq!(fake.login_status().unwrap(), LoginStatus::Missing);
}

#[test]
fn session_ids_are_distinct_and_counted() {
    let fake = FakeClaude::new();
    let a = fake.new_session_id().unwrap();
    let b = fake.new_session_id().unwrap();
    assert!(!a.is_empty() && !b.is_empty());
    assert_ne!(a, b);
}

// --- agent_view (#647 part 2) ---

/// A process endpoint answers the empty view (a single process cannot tell); a port endpoint
/// is `server-unsupported`, the one conversion a single-process harness makes, so a call the
/// router sent to the wrong harness is loud rather than an empty view (plan review W-4).
#[test]
fn agent_view_is_empty_for_a_process_and_refused_for_a_port() {
    let fake = FakeClaude::new();
    let id = fake.new_session_id().unwrap();
    let pid = launch(&fake, C1, SessionChoice::Fresh(id.clone()));
    assert_eq!(
        fake.agent_view(&endpoint(C1, pid), &id, 3),
        Ok(AgentView::default())
    );
    match fake.agent_view(&HarnessEndpoint::Port(48100), &id, 3) {
        Err(PaneError::Refused { code, .. }) => assert_eq!(code.as_str(), "server-unsupported"),
        other => panic!("a port endpoint must be server-unsupported, got {other:?}"),
    }
}

/// `agent_view` passes the faults first, as every port method does, under its own op.
#[test]
fn a_fault_on_agent_view_fails_that_call_and_is_logged() {
    assert_eq!(ClaudeOp::AgentView.as_str(), "harness.agent_view");
    let fake = FakeClaude::new();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    fake.faults()
        .fail_next(ClaudeOp::AgentView, unavailable.clone());
    assert_eq!(
        fake.agent_view(&endpoint(C1, 4242), "ses_1", 3),
        Err(unavailable)
    );
    assert_eq!(fake.faults().calls().last(), Some(&ClaudeOp::AgentView));
}
