#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #850
//! A Claude Code row under the session manager (#850, Acceptance 3; ADR 0014 points 7 to 11): the
//! same session task, queue, presence and crash isolation as any attach row, over a
//! `FakeClaude` the manager is handed with `SessionManager::start_with(&registry, Some(port))`
//! (D1; `start` stays the call with no port).
//!
//! `session/rebind` is driven at the wire as `rebind_test.rs` (#839) drives it: `handle_rebind`
//! answers one request on the outbound channel. The row's refusals are the existing ones (the
//! rebind rule): an id no process holds is `-32602` "session not found on the endpoint", a lookup
//! still running at the deadline is `-32004` "endpoint did not answer", a row mid-turn is `-32009`,
//! the old binding kept in each.

use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::prompt_dispatch::handle_rebind;
use holler_body::registry::SessionRegistry;
use holler_body::session_manager::{PromptOutcome, SessionManager};
use holler_pane::ports::SessionChoice;
use holler_pane::{HarnessPort, PaneName, ServeSpec};
use holler_pane_testkit::claude::{ClaudeOp, FakeClaude};
use holler_proto::rebind::Rebind;
use holler_proto::{Mode, SessionAd, SessionName, SessionState};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const ONE: &str = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11";
const TWO: &str = "1c7a9e30-8d2b-4f65-a1c4-3e5d7b9f2c42";
const THREE: &str = "2d8bafa1-9e3c-4076-b2d5-4f6e8cafd353";
const WAIT: Duration = Duration::from_secs(15);

fn name(text: &str) -> SessionName {
    SessionName::parse(text).unwrap()
}

fn row(pane: &str, session: &str) -> SessionConfig {
    SessionConfig {
        name: name(pane),
        harness: "claude".to_owned(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: Some(session.to_owned()),
        auth_method: None,
    }
}

/// A process of `pane` on `session`, started as the fake's own argv describes.
fn start_process(fake: &FakeClaude, pane: &str, session: &str, resume: bool) -> u32 {
    let choice = if resume {
        SessionChoice::Resume(session.to_owned())
    } else {
        SessionChoice::Fresh(session.to_owned())
    };
    let argv = fake
        .process_command(
            &PaneName::parse(pane).unwrap(),
            &ServeSpec::default().with_cwd("/srv/demo"),
            &choice,
        )
        .unwrap();
    fake.start(&argv).expect("the fake's own argv").unwrap()
}

fn manager(rows: &[SessionConfig], port: Option<Arc<dyn HarnessPort>>) -> Arc<SessionManager> {
    Arc::new(SessionManager::start_with(
        &SessionRegistry::from_sessions(rows.to_vec()),
        port,
    ))
}

fn fake_and_port() -> (Arc<FakeClaude>, Option<Arc<dyn HarnessPort>>) {
    let fake = Arc::new(FakeClaude::new());
    let port: Arc<dyn HarnessPort> = fake.clone();
    (fake, Some(port))
}

async fn until(what: &str, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !check() {
        assert!(Instant::now() < deadline, "never: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn ad(sm: &SessionManager, pane: &str) -> Option<SessionAd> {
    sm.presence_doc("kiwi".to_owned())
        .sessions
        .into_iter()
        .find(|ad| ad.name == pane)
}

/// Wait until the row's first attach has run (the fake was asked who holds a session) and the row
/// is visible: a row is hidden while its first attach runs.
async fn until_attached(sm: &SessionManager, fake: &FakeClaude, pane: &str) {
    until("the attach looked for the process", || {
        fake.faults().calls().contains(&ClaudeOp::LocateProcess)
    })
    .await;
    until("the row is advertised", || ad(sm, pane).is_some()).await;
}

fn inbox(fake: &FakeClaude, pid: u32) -> Vec<String> {
    fake.process(pid).map(|p| p.inbox).unwrap_or_default()
}

fn polls(fake: &FakeClaude) -> usize {
    fake.faults()
        .calls()
        .iter()
        .filter(|op| **op == ClaudeOp::SessionActivity)
        .count()
}

async fn until_polled_twice(fake: &FakeClaude) {
    let before = polls(fake);
    until("the driver polls twice", || polls(fake) >= before + 2).await;
}

type Turn = tokio::task::JoinHandle<PromptOutcome>;

fn prompt(sm: &Arc<SessionManager>, pane: &str, id: &str, text: &str, queue: bool) -> Turn {
    let (sm, pane, id, text) = (Arc::clone(sm), name(pane), id.to_owned(), text.to_owned());
    tokio::spawn(async move { sm.prompt(&pane, id, text, queue).await.unwrap() })
}

async fn finished(turn: Turn) -> PromptOutcome {
    tokio::time::timeout(WAIT, turn)
        .await
        .expect("the turn ends")
        .unwrap()
}

fn stop_reason(outcome: &PromptOutcome) -> &str {
    match outcome {
        PromptOutcome::Result { stop_reason, .. } => stop_reason,
        other => panic!("expected a finished turn, got {other:?}"),
    }
}

/// Run one prompt to the end: the process is seen busy, then idle.
async fn short_turn(sm: &Arc<SessionManager>, fake: &FakeClaude, pane: &str, pid: u32, text: &str) -> PromptOutcome {
    let turn = prompt(sm, pane, "h-01HTESTCLAUDE000000000001", text, false);
    until("the message reaches the process", || {
        inbox(fake, pid).last().map(String::as_str) == Some(text)
    })
    .await;
    fake.stall(pid).unwrap();
    until_polled_twice(fake).await;
    fake.set_idle(pid).unwrap();
    finished(turn).await
}

// --- the row on the roster --------------------------------------------------------------------

#[tokio::test]
async fn the_row_is_advertised_attach_claude_with_its_session_id() {
    let (fake, port) = fake_and_port();
    start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;
    let advertised = ad(&sm, "c1r1").unwrap();
    assert_eq!(advertised.harness, "claude");
    assert_eq!(advertised.mode, Mode::Attach);
    assert_eq!(advertised.harness_session_id.as_deref(), Some(ONE));
    assert_eq!(advertised.state, SessionState::Idle);
    assert!(advertised.pending.is_none());
}

#[tokio::test]
async fn the_row_is_omitted_until_a_process_holds_its_session() {
    let (fake, port) = fake_and_port();
    let sm = manager(&[row("c1r1", ONE)], port);
    until("the attach looked for the process", || {
        fake.faults().calls().contains(&ClaudeOp::LocateProcess)
    })
    .await;
    // The attach has run and failed closed: the row stays off the roster, nothing was started.
    assert!(ad(&sm, "c1r1").is_none(), "no process holds the session, so no row");
    assert!(fake.processes().is_empty());
    assert!(!fake.faults().calls().contains(&ClaudeOp::SendPrompt));
}

#[tokio::test]
async fn the_presence_follows_the_turn_working_input_required_working_idle() {
    let (fake, port) = fake_and_port();
    let pid = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;
    let turn = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000002", "needs a permission", false);
    until("working", || ad(&sm, "c1r1").is_some_and(|a| a.state == SessionState::Working)).await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.hold_permission(pid).unwrap();
    until("input-required", || {
        ad(&sm, "c1r1").is_some_and(|a| a.state == SessionState::InputRequired)
    })
    .await;
    assert!(ad(&sm, "c1r1").unwrap().pending.is_none(), "nothing pending: the pane answers it");
    fake.stall(pid).unwrap();
    until("working again", || ad(&sm, "c1r1").is_some_and(|a| a.state == SessionState::Working)).await;
    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(turn).await), "end_turn");
    until("idle", || ad(&sm, "c1r1").is_some_and(|a| a.state == SessionState::Idle)).await;
}

// --- the queue, the crash isolation, shutdown --------------------------------------------------

#[tokio::test]
async fn a_second_prompt_queues_and_runs_after_the_first() {
    let (fake, port) = fake_and_port();
    let pid = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;

    let first = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000003", "first", false);
    until("the first message", || inbox(&fake, pid) == ["first"]).await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;

    // A plain second prompt is refused while the first runs; a queued one waits its turn.
    let refused = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000004", "refused", false);
    assert!(matches!(finished(refused).await, PromptOutcome::Busy { .. }));
    let second = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000005", "second", true);
    assert_eq!(inbox(&fake, pid), ["first"], "the second is not sent while the first runs");

    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(first).await), "end_turn");
    until("the second message", || inbox(&fake, pid) == ["first", "second"]).await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(second).await), "end_turn");
}

/// A turn that ended `error` drops the driver, and the next prompt attaches again: it finds the
/// process that holds the session now, however the pane was relaunched in between.
#[tokio::test]
async fn after_a_turn_that_ended_error_the_next_prompt_re_attaches() {
    let (fake, port) = fake_and_port();
    let first = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;

    let turn = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000006", "lost", false);
    until("the message", || inbox(&fake, first) == ["lost"]).await;
    fake.stall(first).unwrap();
    until_polled_twice(&fake).await;
    fake.sigint(first).unwrap();
    assert_eq!(stop_reason(&finished(turn).await), "error", "no process holds the session");

    let second = start_process(&fake, "c1r1", ONE, true);
    let outcome = short_turn(&sm, &fake, "c1r1", second, "again").await;
    assert_eq!(stop_reason(&outcome), "end_turn");
    assert_eq!(inbox(&fake, second), ["again"]);
}

/// The cancel rule at the task: `session/cancel` in flight is refused naming the pane verb and the
/// turn runs on; `replace` is refused in flight and is a plain prompt when none is.
#[tokio::test]
async fn cancel_and_replace_in_flight_are_refused_and_replace_when_idle_is_a_prompt() {
    let (fake, port) = fake_and_port();
    let pid = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;

    assert_eq!(sm.cancel(&name("c1r1")).await.unwrap(), Ok(()), "idle: it applies at once");

    let turn = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000007", "long", false);
    until("the message", || inbox(&fake, pid) == ["long"]).await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;

    let refused = sm.cancel(&name("c1r1")).await.unwrap().expect_err("refused in flight");
    assert!(refused.contains("holler interrupt --pane c1r1"), "{refused}");
    match sm.replace(&name("c1r1"), "redirect").await.unwrap() {
        PromptOutcome::Error(message) => {
            assert!(message.contains("holler interrupt --pane c1r1"), "{message}");
        }
        other => panic!("a replace in flight is refused, got {other:?}"),
    }
    assert_eq!(inbox(&fake, pid), ["long"], "neither wrote anything to the process");
    assert!(!turn.is_finished(), "the turn runs on");
    assert!(!fake.faults().calls().contains(&ClaudeOp::Abort));

    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(turn).await), "end_turn");

    // Idle again: a replace is the plain prompt.
    let replaced: Turn = {
        let sm = Arc::clone(&sm);
        tokio::spawn(async move { sm.replace(&name("c1r1"), "redirect").await.unwrap() })
    };
    until("the redirect reaches the process", || inbox(&fake, pid) == ["long", "redirect"]).await;
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(replaced).await), "end_turn");
}

#[tokio::test]
async fn shutdown_touches_nothing() {
    let (fake, port) = fake_and_port();
    let pid = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;
    let calls = fake.faults().calls();
    Arc::try_unwrap(sm).ok().expect("the only handle").shutdown().await;
    assert!(fake.process(pid).is_some(), "the process still runs");
    assert!(inbox(&fake, pid).is_empty(), "nothing was written");
    let after = fake.faults().calls();
    assert!(
        !after[calls.len()..]
            .iter()
            .any(|op| matches!(op, ClaudeOp::SendPrompt | ClaudeOp::Abort | ClaudeOp::ProcessCommand)),
        "shutdown wrote or started nothing: {:?}",
        &after[calls.len()..]
    );
}

#[tokio::test]
async fn two_rows_on_one_body_share_the_one_port_and_each_finds_its_own_process() {
    let (fake, port) = fake_and_port();
    let a = start_process(&fake, "c1r1", ONE, false);
    let b = start_process(&fake, "c2r1", TWO, false);
    let sm = manager(&[row("c1r1", ONE), row("c2r1", TWO)], port);
    until_attached(&sm, &fake, "c1r1").await;
    until_attached(&sm, &fake, "c2r1").await;
    assert_eq!(ad(&sm, "c1r1").unwrap().harness_session_id.as_deref(), Some(ONE));
    assert_eq!(ad(&sm, "c2r1").unwrap().harness_session_id.as_deref(), Some(TWO));

    assert_eq!(stop_reason(&short_turn(&sm, &fake, "c2r1", b, "to b").await), "end_turn");
    assert_eq!(inbox(&fake, b), ["to b"]);
    assert!(inbox(&fake, a).is_empty(), "the other row's process got nothing");
    assert_eq!(stop_reason(&short_turn(&sm, &fake, "c1r1", a, "to a").await), "end_turn");
    assert_eq!(inbox(&fake, a), ["to a"]);
}

/// A body started with no port (`SessionManager::start`) cannot attach a Claude row: the row stays
/// off the roster, and a prompt is refused with a reason that names the missing configuration
/// instead of hanging or panicking.
#[tokio::test]
async fn a_body_started_with_no_port_refuses_the_row_s_attach_naming_the_missing_configuration() {
    let sm = Arc::new(SessionManager::start(&SessionRegistry::from_sessions(vec![row("c1r1", ONE)])));
    // The task attaches before it reads its mailbox, so the reply below is after the attach failed.
    match sm.prompt(&name("c1r1"), "h-01HTESTCLAUDE000000000008", "hi", false).await.unwrap() {
        PromptOutcome::Error(message) => {
            assert!(message.to_lowercase().contains("claude"), "names what is missing: {message}");
        }
        other => panic!("expected the attach refused, got {other:?}"),
    }
    assert!(ad(&sm, "c1r1").is_none(), "the row never attaches");
}

// --- session/rebind (the rebind rule) ----------------------------------------------------------

async fn rebind(sm: &Arc<SessionManager>, pane: &str, to: &str) -> Value {
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    let params = Rebind { session: pane.to_owned(), harness_session_id: to.to_owned() };
    handle_rebind(Arc::clone(sm), "h-01HTESTCLAUDEREBIND0000001".to_owned(), params, tx).await;
    let frame = tokio::time::timeout(WAIT, rx.recv()).await.expect("a reply").expect("one frame");
    serde_json::from_str(frame.to_text().unwrap()).unwrap()
}

/// `(code, data.reason)` of a refusal frame.
fn refusal(frame: &Value) -> (i64, Option<String>) {
    let error = frame.get("error").unwrap_or_else(|| panic!("a refusal: {frame}"));
    (
        error["code"].as_i64().unwrap(),
        error.pointer("/data/reason").and_then(Value::as_str).map(str::to_owned),
    )
}

#[tokio::test]
async fn rebind_to_a_held_session_swaps_and_advertises_it_and_to_an_unheld_one_is_refused_keeping_the_binding() {
    let (fake, port) = fake_and_port();
    let old = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;

    // The pane engine relaunched the pane on a new session before it asks the body to follow.
    fake.sigint(old).unwrap();
    let new = start_process(&fake, "c1r1", TWO, false);
    let started = fake.faults().calls().len();
    let frame = rebind(&sm, "c1r1", TWO).await;
    assert_eq!(frame["result"], serde_json::json!({"session": "c1r1", "harness_session_id": TWO}), "{frame}");
    assert_eq!(ad(&sm, "c1r1").unwrap().harness_session_id.as_deref(), Some(TWO), "same name, new id");
    let calls = fake.faults().calls();
    assert!(
        calls[started..].iter().all(|op| matches!(op, ClaudeOp::LocateProcess | ClaudeOp::SessionActivity)),
        "the check that the target exists is the attach: it only looks: {:?}",
        &calls[started..]
    );

    // A target no process holds is refused as an id the endpoint does not have; the binding stays.
    let frame = rebind(&sm, "c1r1", THREE).await;
    assert_eq!(refusal(&frame), (-32602, Some("session not found on the endpoint".to_owned())), "{frame}");
    assert_eq!(ad(&sm, "c1r1").unwrap().harness_session_id.as_deref(), Some(TWO));

    // The new binding is the one a prompt reaches.
    assert_eq!(stop_reason(&short_turn(&sm, &fake, "c1r1", new, "after the rebind").await), "end_turn");
    assert_eq!(inbox(&fake, new), ["after the rebind"]);
}

#[tokio::test]
async fn rebind_of_a_row_mid_turn_is_refused_session_busy() {
    let (fake, port) = fake_and_port();
    let pid = start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;
    let turn = prompt(&sm, "c1r1", "h-01HTESTCLAUDE000000000009", "long", false);
    until("the message", || inbox(&fake, pid) == ["long"]).await;
    let frame = rebind(&sm, "c1r1", TWO).await;
    assert_eq!(refusal(&frame).0, -32009, "{frame}");
    assert_eq!(ad(&sm, "c1r1").unwrap().harness_session_id.as_deref(), Some(ONE));
    fake.stall(pid).unwrap();
    until_polled_twice(&fake).await;
    fake.set_idle(pid).unwrap();
    assert_eq!(stop_reason(&finished(turn).await), "end_turn");
}

/// A lookup still running at the rebind's deadline is dropped and refused as an endpoint that did
/// not answer; the old binding is kept.
#[tokio::test]
async fn a_rebind_lookup_past_the_deadline_is_refused_endpoint_did_not_answer() {
    let (fake, port) = fake_and_port();
    start_process(&fake, "c1r1", ONE, false);
    let sm = manager(&[row("c1r1", ONE)], port);
    until_attached(&sm, &fake, "c1r1").await;
    fake.faults().set_delay(Some(Duration::from_secs(8)));
    let frame = rebind(&sm, "c1r1", TWO).await;
    assert_eq!(refusal(&frame), (-32004, Some("endpoint did not answer".to_owned())), "{frame}");
    fake.faults().set_delay(None);
    assert_eq!(ad(&sm, "c1r1").unwrap().harness_session_id.as_deref(), Some(ONE));
}
