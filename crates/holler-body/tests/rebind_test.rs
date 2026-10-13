#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #839
//! `session/rebind`, the body's half (#839, D2 and P2; Acceptance 9): an attach-mode session is
//! re-pointed to another harness session on the same endpoint, fail closed, within 5 s, and its
//! name does not change. The one source of the id it attaches to is the session task's own
//! configuration, so every later reader follows it: presence, the re-attach after a turn that
//! ended in error, and `query/support`'s live probe. A restart reverts it (not persisted).
//!
//! Driven at the wire: `prompt_dispatch::handle_rebind` answers one `session/rebind` request on
//! the outbound channel, as the connection loop spawns it, over a real `SessionManager` whose
//! attach driver talks to the fake OpenCode server (`fake_server.rs`, with its stall mode). The
//! fake answers one existence verdict for every id, so the cases toggle `exists_v1`/`exists_v2`.

#[path = "http_attach_driver_test/fake_server.rs"]
#[allow(dead_code)] // #839: this file uses a few of the fake's controls
mod fake_server;

use std::sync::Arc;
use std::time::{Duration, Instant};

use fake_server::{session_error, session_idle, FakeServer};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::prompt_dispatch::{handle_rebind, REBIND_DEADLINE, REBIND_REPLY_WAIT};
use holler_body::registry::SessionRegistry;
use holler_body::session_manager::{PromptOutcome, SessionManager};
use holler_proto::rebind::Rebind;
use holler_proto::SessionName;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const OLD: &str = "ses_000000000000000000000000a1";
const NEW: &str = "ses_000000000000000000000000b2";
const WAIT: Duration = Duration::from_secs(10);

fn name(text: &str) -> SessionName {
    SessionName::parse(text).unwrap()
}

fn attach(session: &str, endpoint: &str, id: &str) -> SessionConfig {
    SessionConfig {
        name: name(session),
        harness: "opencode".to_owned(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Http,
        endpoint: Some(endpoint.to_owned()),
        session_id: Some(id.to_owned()),
        auth_method: None,
    }
}

/// A spawn-mode session that is never prompted here (its driver comes up lazily).
fn spawned(session: &str) -> SessionConfig {
    SessionConfig {
        mode: SessionMode::Spawn,
        command: Some(vec!["true".to_owned()]),
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: None,
        ..attach(session, "", "")
    }
}

fn start(configs: &[SessionConfig]) -> Arc<SessionManager> {
    Arc::new(SessionManager::start(&SessionRegistry::from_sessions(
        configs.to_vec(),
    )))
}

/// The id the session `session` advertises in presence now (`None`: no row).
fn advertised(sm: &SessionManager, session: &str) -> Option<String> {
    let doc = sm.presence_doc("kiwi".to_owned());
    let row = doc.sessions.into_iter().find(|ad| ad.name == session)?;
    Some(row.harness_session_id.unwrap_or_default())
}

/// Wait, bounded, until `session`'s first attach of `id` has succeeded. A row is visible before
/// its task runs (the mirror is initialised at `start`) and hidden while the first attach runs, so
/// visibility alone is not "attached": wait for the attach's existence check (a request after
/// `since`) first, then for the row.
async fn until_attached(
    sm: &SessionManager,
    server: &FakeServer,
    since: usize,
    session: &str,
    id: &str,
) {
    until_request(server, since, "GET", &format!("/session/{id}")).await;
    let deadline = Instant::now() + WAIT;
    while advertised(sm, session).is_none() {
        assert!(Instant::now() < deadline, "{session} never attached");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// One `session/rebind {session, harness_session_id: to}` answered by the body: the reply frame.
async fn rebind(sm: &Arc<SessionManager>, session: &str, to: &str) -> Value {
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    let params = Rebind {
        session: session.to_owned(),
        harness_session_id: to.to_owned(),
    };
    handle_rebind(
        Arc::clone(sm),
        "h-01HTESTREBIND000000000000".to_owned(),
        params,
        tx,
    )
    .await;
    let frame = tokio::time::timeout(WAIT, rx.recv())
        .await
        .expect("a reply")
        .expect("one frame");
    serde_json::from_str(frame.to_text().unwrap()).unwrap()
}

/// `(code, data.reason)` of a refusal frame.
fn refusal(frame: &Value) -> (i64, Option<String>) {
    let error = frame
        .get("error")
        .unwrap_or_else(|| panic!("a refusal: {frame}"));
    let reason = error
        .pointer("/data/reason")
        .and_then(Value::as_str)
        .map(str::to_owned);
    (error["code"].as_i64().unwrap(), reason)
}

/// Wait, bounded, until the fake has recorded `method path` at or after request `since`.
async fn until_request(server: &FakeServer, since: usize, method: &str, path: &str) {
    let deadline = Instant::now() + WAIT;
    while !server
        .requests()
        .iter()
        .skip(since)
        .any(|r| r.method == method && r.path == path)
    {
        assert!(
            Instant::now() < deadline,
            "{method} {path} never arrived: {:?}",
            server.requests()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Wait, bounded, until the fake has accepted an `/event` connection beyond the first `seen`
/// (read `server.event_connections()` before the action that makes the body open a new stream).
/// `push_event` reaches only connections already subscribed, so pushing earlier loses the event
/// and the turn never ends (#864).
async fn until_event_stream(server: &FakeServer, seen: usize) {
    let deadline = Instant::now() + WAIT;
    while server.event_connections() <= seen {
        assert!(
            Instant::now() < deadline,
            "no /event connection after the first {seen}: the body never reconnected"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// One prompt to `alpha` that the fake ends with `end` on `id`: proves which session it reached.
async fn prompt_reaches(
    sm: &Arc<SessionManager>,
    server: &FakeServer,
    id: &str,
    streams_seen: usize,
) -> PromptOutcome {
    let since = server.requests().len();
    let manager = Arc::clone(sm);
    let turn = tokio::spawn(async move {
        manager
            .prompt(&name("alpha"), "h-01HTESTREBINDPROMPT000000", "hi", false)
            .await
    });
    until_request(
        server,
        since,
        "POST",
        &format!("/session/{id}/prompt_async"),
    )
    .await;
    until_event_stream(server, streams_seen).await;
    server.push_event(session_idle(id));
    tokio::time::timeout(WAIT, turn)
        .await
        .expect("the turn ends")
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rebind_re_points_the_session_and_keeps_its_name() {
    let server = FakeServer::start().await;
    let sm = start(&[attach("alpha", &server.endpoint(), OLD)]);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    let since = server.requests().len();
    let streams = server.event_connections();
    let frame = rebind(&sm, "alpha", NEW).await;
    assert_eq!(
        frame["result"],
        serde_json::json!({"session": "alpha", "harness_session_id": NEW}),
        "{frame}"
    );
    assert!(
        server
            .requests()
            .iter()
            .skip(since)
            .any(|r| r.method == "GET" && r.path.ends_with(&format!("/session/{NEW}"))),
        "the existence check of attach, never session/new"
    );
    assert!(!server.state.lock().unwrap().session_new_called);
    assert_eq!(
        advertised(&sm, "alpha").as_deref(),
        Some(NEW),
        "presence advertises it, same name"
    );
    prompt_reaches(&sm, &server, NEW, streams).await;
}

/// The refusals, each failing closed with the old binding kept: an unknown name, a spawn
/// session, an id the endpoint does not have.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_refusals_keep_the_old_binding() {
    let server = FakeServer::start().await;
    let sm = start(&[attach("alpha", &server.endpoint(), OLD), spawned("beta")]);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    assert_eq!(
        refusal(&rebind(&sm, "gamma", NEW).await),
        (-32003, None),
        "unknown session"
    );
    let spawn = refusal(&rebind(&sm, "beta", NEW).await);
    assert_eq!(spawn, (-32602, Some("not an attach session".to_owned())));
    {
        let mut state = server.state.lock().unwrap();
        state.exists_v1 = false;
        state.exists_v2 = false;
    }
    let missing = refusal(&rebind(&sm, "alpha", NEW).await);
    assert_eq!(
        missing,
        (-32602, Some("session not found on the endpoint".to_owned()))
    );
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(OLD));
    prompt_reaches(&sm, &server, OLD, 0).await;
}

/// A session with a turn in flight is `session_busy`, with the existing timing fields, and keeps
/// its binding.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_busy_session_refuses_the_rebind() {
    let server = FakeServer::start().await;
    let sm = start(&[attach("alpha", &server.endpoint(), OLD)]);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    let manager = Arc::clone(&sm);
    let turn = tokio::spawn(async move {
        manager
            .prompt(&name("alpha"), "h-01HTESTREBINDBUSY00000000", "hi", false)
            .await
    });
    until_request(&server, 0, "POST", &format!("/session/{OLD}/prompt_async")).await;
    let frame = rebind(&sm, "alpha", NEW).await;
    assert_eq!(refusal(&frame).0, -32009, "{frame}");
    assert!(frame.pointer("/error/data/state").is_some(), "{frame}");
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(OLD));
    until_event_stream(&server, 0).await;
    server.push_event(session_idle(OLD));
    tokio::time::timeout(WAIT, turn)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

/// I5: an endpoint that does not answer the existence check is `not_connected` ("endpoint did
/// not answer") within the 5 s deadline plus the handler's 1 s, the old binding kept for certain:
/// the attach that finishes after the deadline is dropped, never swapped in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_silent_endpoint_is_refused_in_time_and_a_late_attach_is_never_swapped_in() {
    assert_eq!(
        (REBIND_DEADLINE, REBIND_REPLY_WAIT),
        (Duration::from_secs(5), Duration::from_secs(6))
    );
    let server = FakeServer::start().await;
    let sm = start(&[attach("alpha", &server.endpoint(), OLD)]);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    server.set_stall_exists(true);
    let started = Instant::now();
    let frame = rebind(&sm, "alpha", NEW).await;
    let took = started.elapsed();
    assert_eq!(
        refusal(&frame),
        (-32004, Some("endpoint did not answer".to_owned())),
        "{frame}"
    );
    assert!(
        took >= Duration::from_secs(4) && took < Duration::from_secs(7),
        "answered in {took:?}"
    );

    let since = server.requests().len();
    server.set_stall_exists(false);
    until_request(&server, since, "GET", &format!("/session/{NEW}")).await;
    assert_eq!(
        advertised(&sm, "alpha").as_deref(),
        Some(OLD),
        "the late attach is not swapped in"
    );
    prompt_reaches(&sm, &server, OLD, 0).await;
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(OLD));
}

/// P2, one source of the id: after a rebind, a turn that ends in error drops the driver, and the
/// next prompt re-attaches the NEW id (its existence check and its `prompt_async`); nothing after
/// the rebind reaches the old id, and presence advertises the new one throughout.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_re_attach_after_an_error_follows_the_new_id() {
    let server = FakeServer::start().await;
    let sm = start(&[attach("alpha", &server.endpoint(), OLD)]);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    let streams = server.event_connections();
    rebind(&sm, "alpha", NEW).await;
    let since = server.requests().len();
    let manager = Arc::clone(&sm);
    let turn = tokio::spawn(async move {
        manager
            .prompt(&name("alpha"), "h-01HTESTREBINDERR000000000", "hi", false)
            .await
    });
    until_request(
        &server,
        since,
        "POST",
        &format!("/session/{NEW}/prompt_async"),
    )
    .await;
    until_event_stream(&server, streams).await;
    server.push_event(session_error(NEW));
    tokio::time::timeout(WAIT, turn)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(NEW));

    let again = server.requests().len();
    // The error dropped the driver: the next prompt re-attaches and opens a stream of its own.
    let streams = server.event_connections();
    prompt_reaches(&sm, &server, NEW, streams).await;
    let after: Vec<String> = server
        .requests()
        .iter()
        .skip(again)
        .map(|r| format!("{} {}", r.method, r.path))
        .collect();
    assert!(
        after
            .iter()
            .any(|r| r.starts_with("GET") && r.ends_with(NEW)),
        "re-attached the new id: {after:?}"
    );
    let old: Vec<_> = server
        .requests()
        .iter()
        .skip(since)
        .filter(|r| r.path.contains(OLD))
        .cloned()
        .collect();
    assert!(
        old.is_empty(),
        "nothing reaches the old id after the rebind: {old:?}"
    );
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(NEW));
}

/// `query/support`'s live probe reads the live id: the configurations a body answers it from are
/// `live_configs`, the as-started ones with each attach session's id from the session task.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_support_probes_the_live_id() {
    let server = FakeServer::start().await;
    let configs = [attach("alpha", &server.endpoint(), OLD)];
    let sm = start(&configs);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    rebind(&sm, "alpha", NEW).await;
    let live = sm.live_configs(&configs);
    assert_eq!(live[0].session_id.as_deref(), Some(NEW));
    assert_eq!(live[0].name, configs[0].name);
    let since = server.requests().len();
    let support =
        tokio::task::spawn_blocking(move || holler_body::query::local_support("opencode", &live))
            .await
            .unwrap()
            .unwrap();
    assert!(support.ok, "{support:?}");
    let probed: Vec<String> = server
        .requests()
        .iter()
        .skip(since)
        .map(|r| r.path.clone())
        .collect();
    assert!(
        probed.iter().all(|p| !p.contains(OLD)) && probed.iter().any(|p| p.ends_with(NEW)),
        "{probed:?}"
    );
}

/// P2: the rebind is not persisted. A second manager over the same configuration (a restarted
/// body) attaches the configured id again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restart_reverts_to_the_configured_id() {
    let server = FakeServer::start().await;
    let configs = [attach("alpha", &server.endpoint(), OLD)];
    let sm = start(&configs);
    until_attached(&sm, &server, 0, "alpha", OLD).await;
    rebind(&sm, "alpha", NEW).await;
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(NEW));
    Arc::try_unwrap(sm)
        .ok()
        .expect("the only handle")
        .shutdown()
        .await;
    let since = server.requests().len();
    let restarted = start(&configs);
    until_attached(&restarted, &server, since, "alpha", OLD).await;
    assert_eq!(advertised(&restarted, "alpha").as_deref(), Some(OLD));
}

/// The first moment: presence read right after `start`, before the session task has run, already
/// advertises the configured id for an attach row (the mirror is initialised where the presence
/// is built), so no reader ever sees an attach row with no id.
#[tokio::test(flavor = "current_thread")]
async fn presence_advertises_the_configured_id_before_the_task_runs() {
    let sm = start(&[attach("alpha", "http://127.0.0.1:9", OLD)]);
    assert_eq!(advertised(&sm, "alpha").as_deref(), Some(OLD));
}

/// #864: `push_event` reaches only an `/event` connection already subscribed, so an idle pushed
/// before the body's stream is up is lost and the turn never ends. The fake counts subscriptions
/// and the wait helper blocks on the count: this pins both with no body in the loop, so it fails
/// at once (not after `WAIT`) if either stops holding.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_event_pushed_before_the_stream_connects_is_lost_and_the_wait_sees_the_connection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let server = FakeServer::start().await;
    assert_eq!(server.event_connections(), 0);
    server.push_event(session_idle(NEW));

    let mut stream = tokio::net::TcpStream::connect(server.endpoint().trim_start_matches("http://"))
        .await
        .unwrap();
    stream
        .write_all(b"GET /event HTTP/1.1\r\nHost: fake\r\n\r\n")
        .await
        .unwrap();
    until_event_stream(&server, 0).await;
    assert_eq!(server.event_connections(), 1);

    let mut seen = Vec::new();
    let mut buf = [0u8; 1024];
    let early = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            seen.extend_from_slice(&buf[..n]);
            if String::from_utf8_lossy(&seen).contains("session.idle") {
                return;
            }
        }
    })
    .await;
    assert!(early.is_err(), "the idle pushed before the connection must not replay");
}
