#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #194: test assertions

//! RED/GREEN tests for the HTTP attach driver (issue #194) against a fake
//! OpenCode HTTP server (`fake_server.rs`, hand-rolled — see its own doc
//! comment for why no `axum`/`hyper` dev-dependency was added).

#[path = "http_attach_driver_test/fake_server.rs"]
mod fake_server;

use std::time::Duration;

use fake_server::{
    message_updated_assistant, multi_question_request, part_updated_text, permission_request,
    question_request, session_error, session_idle, FakeServer,
};
use holler_body::acp_driver::{DriverError, DriverEvent, DriverState, Status, StopReason};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::http_attach_driver::HttpAttachDriver;
use holler_proto::SessionName;
use rstest::rstest;
use serde_json::json;

fn config(endpoint: &str, session_id: &str) -> SessionConfig {
    SessionConfig {
        name: SessionName::parse("alpha").expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Http,
        endpoint: Some(endpoint.to_string()),
        session_id: Some(session_id.to_string()),
    }
}

async fn next_event_within(
    stream: &mut holler_body::acp_driver::DriverEventStream,
    timeout: Duration,
) -> DriverEvent {
    use futures_util::StreamExt;
    tokio::time::timeout(timeout, stream.next())
        .await
        .expect("event should arrive within the timeout")
        .expect("event channel should not be closed")
}

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn attach_404_fails_closed_no_session_new_call() {
    let server = FakeServer::start().await;
    {
        let mut state = server.state.lock().unwrap();
        state.exists_v1 = false;
        state.exists_v2 = false;
    }

    let result = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_missing")).await;

    match result {
        Err(DriverError::NotFound(_)) => {}
        Err(other) => panic!("expected NotFound, got a different error: {other}"),
        Ok(_) => panic!("attach to a missing session must fail closed, not succeed"),
    }
    assert!(
        !server.state.lock().unwrap().session_new_called,
        "attach must never call POST /session"
    );
    assert!(
        server
            .requests()
            .iter()
            .all(|r| !(r.method == "POST" && r.path == "/session")),
        "no POST /session request should ever have been recorded"
    );
}

#[tokio::test]
async fn prompt_streams_deltas_then_idle_done() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_1"))
        .await
        .expect("attach to an existing session must succeed");

    let mut stream = driver.prompt("say hi").await;

    // Give the driver a moment to open its SSE connection before the events
    // it needs are broadcast.
    tokio::time::sleep(Duration::from_millis(150)).await;
    server.push_event(message_updated_assistant("m1", "ses_1"));
    server.push_event(part_updated_text("m1", "ses_1", "hello"));
    server.push_event(part_updated_text("m1", "ses_1", " world"));
    server.push_event(session_idle("ses_1"));

    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Chunk("hello".to_string())
    );
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Chunk(" world".to_string())
    );
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Done(StopReason::EndTurn)
    );
    assert_eq!(driver.status(), Status::Idle);
}

#[tokio::test]
async fn cancel_posts_interrupt_and_yields_cancelled() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_2"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("do a long thing").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    server.push_event(message_updated_assistant("m1", "ses_2"));
    server.push_event(part_updated_text("m1", "ses_2", "working..."));
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Chunk("working...".to_string())
    );

    let cancel_task = tokio::spawn(async move {
        let result = driver.cancel().await;
        (driver, result)
    });

    // Wait for the interrupt POST to actually arrive, then simulate
    // OpenCode's own real cancelled-turn idle event.
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let saw_interrupt = server
            .requests()
            .iter()
            .any(|r| r.method == "POST" && r.path == "/api/session/ses_2/interrupt");
        if saw_interrupt {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "interrupt POST never arrived");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    server.push_event(session_idle("ses_2"));

    let (driver, result) = tokio::time::timeout(WAIT, cancel_task)
        .await
        .expect("cancel task should finish")
        .expect("cancel task should not panic");
    assert_eq!(result, Ok(StopReason::Cancelled));
    assert_eq!(driver.status(), Status::Idle);
}

#[tokio::test]
async fn sse_drop_mid_turn_reconnects_and_completes() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_3"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("say hi").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    server.push_event(message_updated_assistant("m1", "ses_3"));
    server.push_event(part_updated_text("m1", "ses_3", "before drop"));
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Chunk("before drop".to_string())
    );

    server.force_disconnect();
    // Give the driver time to notice the drop and reconnect (bounded by the
    // backoff schedule's first attempt: full jitter over 0..=1000ms, plus the
    // actual HTTP reconnect round trip). 1200ms cut it too close under a
    // loaded CI runner (confirmed via repeated real CI failures, not a local
    // repro) — widened to leave real margin rather than tightening the
    // assertion window further.
    tokio::time::sleep(Duration::from_millis(4_000)).await;
    server.push_event(part_updated_text("m1", "ses_3", "after reconnect"));
    server.push_event(session_idle("ses_3"));

    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Chunk("after reconnect".to_string())
    );
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Done(StopReason::EndTurn)
    );
}

#[tokio::test]
async fn pending_question_polled_and_surfaced_as_input_required() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_4"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("ask something").await;
    server.set_questions(vec![question_request(
        "q1",
        "ses_4",
        "Proceed?",
        &["Yes", "No"],
    )]);

    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::InputRequired)
    );
    assert_eq!(driver.status(), Status::InputRequired);
    let pending = driver.pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].prompt, "Proceed?");
    assert_eq!(pending[0].options, vec!["Yes".to_string(), "No".to_string()]);
}

#[tokio::test]
async fn answer_posts_reply_and_turn_resumes() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_5"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("ask something").await;
    server.set_questions(vec![question_request(
        "q1",
        "ses_5",
        "Proceed?",
        &["Yes", "No"],
    )]);
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::InputRequired)
    );

    driver.answer("Yes").await.expect("answer should be accepted");
    let reply_req = server
        .requests()
        .into_iter()
        .find(|r| r.method == "POST" && r.path == "/question/q1/reply")
        .expect("a reply POST should have been recorded");
    let body: serde_json::Value = serde_json::from_str(&reply_req.body).unwrap();
    assert_eq!(body, json!({"answers": [["Yes"]]}));

    // Simulate OpenCode clearing the question once answered, and the turn
    // resuming and completing.
    server.set_questions(vec![]);
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::Working)
    );
    server.push_event(session_idle("ses_5"));
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Done(StopReason::EndTurn)
    );
}

#[tokio::test]
async fn multi_question_choice_splits_on_comma() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_6"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("ask two things").await;
    server.set_questions(vec![multi_question_request(
        "q2",
        "ses_6",
        &[("First?", &["Yes", "No"][..]), ("Second?", &["A", "B", "C"][..])],
    )]);
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::InputRequired)
    );

    driver.answer("Yes,2").await.expect("comma-separated answer should be accepted");
    let reply_req = server
        .requests()
        .into_iter()
        .find(|r| r.method == "POST" && r.path == "/question/q2/reply")
        .expect("a reply POST should have been recorded");
    let body: serde_json::Value = serde_json::from_str(&reply_req.body).unwrap();
    assert_eq!(body, json!({"answers": [["Yes"], ["C"]]}));
}

#[tokio::test]
async fn child_session_question_is_seen() {
    let server = FakeServer::start().await;
    server.set_child_sessions(vec![
        json!({"id": "ses_7_child", "parentID": "ses_7"}),
    ]);
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_7"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("delegate to a subagent").await;
    server.set_questions(vec![question_request(
        "q3",
        "ses_7_child",
        "Continue?",
        &["Yes", "No"],
    )]);

    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::InputRequired)
    );
    let pending = driver.pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].prompt, "Continue?");
}

#[tokio::test]
async fn permission_reply_uses_permission_shape_not_answers() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_8"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("do something needing permission").await;
    server.set_permissions(vec![permission_request("p1", "ses_8", "write file")]);
    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::State(DriverState::InputRequired)
    );

    driver.answer("once").await.expect("permission answer should be accepted");
    let reply_req = server
        .requests()
        .into_iter()
        .find(|r| r.method == "POST" && r.path == "/permission/p1/reply")
        .expect("a permission reply POST should have been recorded");
    let body: serde_json::Value = serde_json::from_str(&reply_req.body).unwrap();
    assert_eq!(body, json!({"reply": "once"}));
}

#[tokio::test]
async fn session_error_event_yields_done_error() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_9"))
        .await
        .expect("attach must succeed");

    let mut stream = driver.prompt("do something that fails").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    server.push_event(session_error("ses_9"));

    assert_eq!(
        next_event_within(&mut stream, WAIT).await,
        DriverEvent::Done(StopReason::Error)
    );
}

#[tokio::test]
async fn shutdown_does_not_touch_fake_server() {
    let server = FakeServer::start().await;
    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_10"))
        .await
        .expect("attach must succeed");

    driver.shutdown().await.expect("shutdown should succeed");

    // The fake server is still alive and answers a fresh existence check —
    // shutdown never touched the attached session.
    let still_alive = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_10")).await;
    assert!(still_alive.is_ok(), "the fake server must still answer after our shutdown");
}

/// Both existence-check forms were independently confirmed live to work
/// (see `http_attach_driver.rs`'s module doc) — `attach()` must succeed
/// whichever one a given `opencode serve` answers under, and — since
/// `prompt_async`/`interrupt` were *also* confirmed to each need one fixed
/// form regardless of that (an earlier "one resolved prefix for everything"
/// draft of this driver would have silently broken `interrupt` on a v1-only
/// endpoint) — every subsequent call must always go to its own real route,
/// not whichever prefix the existence check happened to resolve.
#[rstest]
#[case(true, false)] // only the bare existence-check form answers
#[case(false, true)] // only the /api existence-check form answers
#[tokio::test]
async fn both_route_prefixes_supported(#[case] v1: bool, #[case] v2: bool) {
    let server = FakeServer::start().await;
    {
        let mut state = server.state.lock().unwrap();
        state.exists_v1 = v1;
        state.exists_v2 = v2;
    }

    let driver = HttpAttachDriver::attach(&config(&server.endpoint(), "ses_prefix"))
        .await
        .expect("attach must succeed under either supported existence-check form");

    driver.prompt("hello").await;
    // Fire-and-forget: the fake server never emits the idle/error event
    // `cancel()` would otherwise wait (up to `CANCEL_TIMEOUT`) for — this
    // test only cares that the interrupt POST itself lands on the right
    // route, not about `cancel()`'s own return value.
    tokio::spawn(async move {
        let _ = driver.cancel().await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let requests = server.requests();
    assert!(
        requests
            .iter()
            .any(|r| r.method == "POST" && r.path == "/session/ses_prefix/prompt_async"),
        "prompt_async must always be sent bare, regardless of the existence-check prefix"
    );
    assert!(
        requests
            .iter()
            .any(|r| r.method == "POST" && r.path == "/api/session/ses_prefix/interrupt"),
        "interrupt must always be sent /api-prefixed, regardless of the existence-check prefix"
    );
}
