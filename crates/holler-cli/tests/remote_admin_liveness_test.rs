#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #508
//! AC 16(a)-(e) (issue #508/#509, epic #506, MO 5): the admin loop's own
//! liveness handling (`crates/holler-hub/src/circuit/admin.rs::run`) —
//! WS pings keep flowing during an in-flight request, an in-flight request
//! is never timed out, an idle connection is closed, several concurrent
//! requests on one socket answer in completion order, and a client drop
//! mid-`say` leaves the hub healthy. Split out of `remote_admin_test.rs`
//! (AC 3-13) purely to keep that file under this crate's 900-line guard —
//! see `support::remote_admin_rig`'s own doc for why the rig is shared, not
//! duplicated.
//!
//! Every test here runs the hub with a low `HOLLER_WS_PING_INTERVAL_MS` /
//! `HOLLER_HUB_LIVENESS_TIMEOUT_MS` (the brief's own example: 200/1000) and a
//! `stub-acp --slow --chunks N` session, so a turn reliably outlasts the
//! liveness timeout without the test itself sleeping blindly for it —
//! readiness is still always observed via `wait_for`, never a fixed sleep,
//! per this crate's own harness discipline.

mod support;

use std::time::Duration;

use support::raw_ws::{decode_next, live_socket, wait_for_close, WsClient};
use support::remote_admin_rig::{rig_with_env, run};
use support::{roster_json, wait_for, Body, Hub, StateDir};

const LOW_LIVENESS_ENV: &[(&str, &str)] = &[("HOLLER_WS_PING_INTERVAL_MS", "200"), ("HOLLER_HUB_LIVENESS_TIMEOUT_MS", "1000")];

fn liveness_rig(sessions: &[(&str, &[&str])]) -> (StateDir, StateDir, Hub, Body, String, String) {
    rig_with_env(sessions, LOW_LIVENESS_ENV)
}

async fn send_admin_say(admin: &mut WsClient, id: &str, session: &str) {
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": id, "method": "admin/say",
        "params": { "session": session, "text": "slow", "queue": false, "timeout_ms": 30_000 },
    });
    futures_util::SinkExt::send(admin, tokio_tungstenite::tungstenite::Message::text(req.to_string()))
        .await
        .expect("send admin/say");
}

/// **AC 16(a).** While an `admin/say` to the slow session is in flight, the
/// admin socket receives at least one WS Ping frame before the response, and
/// the socket stays open (the response still arrives).
#[tokio::test]
async fn pings_flow_during_an_inflight_request() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "8"])]);
    let mut admin = live_socket(&body_state, &hub, &token_id, "admin-a", "admin").await;
    send_admin_say(&mut admin, "b-say-a", "b/alpha").await;

    let mut saw_ping = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let msg = tokio::time::timeout_at(deadline, futures_util::StreamExt::next(&mut admin))
            .await
            .unwrap_or_else(|_| panic!("no admin/say response within 15s (saw_ping so far: {saw_ping})"))
            .expect("the socket stayed open")
            .expect("no ws-level error");
        match msg {
            tokio_tungstenite::tungstenite::Message::Ping(_) => saw_ping = true,
            tokio_tungstenite::tungstenite::Message::Text(t) => {
                let env = holler_proto::decode(t.as_str()).expect("a valid v2 frame");
                match env {
                    holler_proto::Envelope::Response { id, .. } if id == "b-say-a" => break,
                    holler_proto::Envelope::Error { .. } => panic!("admin/say was refused: {env:?}"),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    assert!(saw_ping, "AC 16(a): must observe at least one WS Ping before the admin/say response");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
    let _ = hub_state;
}

/// **AC 16(b).** An admin socket that sends `admin/say` then sends and reads
/// nothing for longer than the liveness timeout still gets its response —
/// an in-flight request is never timed out (`admin.rs::run`'s own
/// `if inflight == 0` guard on the liveness-timeout `select!` arm).
#[tokio::test]
async fn inflight_request_is_never_timed_out() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "8"])]);
    let mut admin = live_socket(&body_state, &hub, &token_id, "admin-b", "admin").await;
    send_admin_say(&mut admin, "b-say-b", "b/alpha").await;

    // Deliberately silent for longer than the rig's 1000ms liveness timeout.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let reply = tokio::time::timeout(Duration::from_secs(10), decode_next(&mut admin))
        .await
        .unwrap_or_else(|_| panic!("AC 16(b): the admin/say response never arrived — timed out mid-flight"))
        .expect("a response frame");
    let holler_proto::Envelope::Response { id, .. } = reply else {
        panic!("expected the admin/say response, got {reply:?}");
    };
    assert_eq!(id, "b-say-b");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
    let _ = hub_state;
}

/// **AC 16(b) regression (PR-Agent review, 2026-09-28).** An inline-answered
/// request (`circuit/ping`) arriving while an `admin/say` is still spawned
/// must not reset `inflight` to 0 — only a reply from the spawned task
/// itself may decrement it (`admin.rs`'s `Reply::Spawned` vs `Reply::Inline`).
/// Before this was fixed, every reply on the shared channel decremented the
/// counter regardless of origin, so an inline reply arriving mid-flight
/// could re-arm the liveness-timeout arm and close the socket before the
/// slow `admin/say`'s real response ever came back.
#[tokio::test]
async fn an_inline_reply_does_not_reset_inflight_while_a_spawned_request_is_pending() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "8"])]);
    let mut admin = live_socket(&body_state, &hub, &token_id, "admin-f", "admin").await;
    send_admin_say(&mut admin, "b-say-f", "b/alpha").await;

    // An inline-answered circuit/ping, sent right after the spawned
    // admin/say — its reply travels the same reply channel and arrives
    // well before the slow say's.
    let ping_req = serde_json::json!({ "jsonrpc": "2.0", "id": "b-ping-f", "method": "circuit/ping", "params": {} });
    futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::text(ping_req.to_string()))
        .await
        .expect("send circuit/ping");
    let ping_reply = tokio::time::timeout(Duration::from_secs(5), decode_next(&mut admin))
        .await
        .expect("circuit/ping answered")
        .expect("a response frame");
    let holler_proto::Envelope::Response { id, .. } = ping_reply else {
        panic!("expected the circuit/ping response, got {ping_reply:?}");
    };
    assert_eq!(id, "b-ping-f");

    // Deliberately silent for longer than the rig's 1000ms liveness
    // timeout, exactly as `inflight_request_is_never_timed_out` does — if
    // the ping's inline reply wrongly zeroed `inflight`, the hub closes
    // this socket here instead of waiting for the say to finish.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let reply = tokio::time::timeout(Duration::from_secs(10), decode_next(&mut admin))
        .await
        .unwrap_or_else(|_| panic!("the admin/say response never arrived — an inline reply falsely reset inflight"))
        .expect("a response frame");
    let holler_proto::Envelope::Response { id, .. } = reply else {
        panic!("expected the admin/say response, got {reply:?}");
    };
    assert_eq!(id, "b-say-f");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
    let _ = hub_state;
}

/// **AC 16(c).** An admin socket that completes its hello and then sends and
/// reads nothing is closed within the liveness timeout plus a bounded
/// margin, and the hub logs `admin_dropped` for it.
#[tokio::test]
async fn idle_admin_socket_is_closed_within_the_liveness_timeout() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &[])]);
    let mut admin = live_socket(&body_state, &hub, &token_id, "admin-c", "admin").await;

    // Genuinely idle: no request, and no read at all for the whole window —
    // reading (even just to observe a WS Ping) makes this WS library answer
    // it with an automatic Pong, which is itself a frame and would keep
    // refreshing the hub's own liveness clock (`admin.rs::run`'s
    // `frame_outcome` treats any inbound Ping/Pong as `Continue`). MO 5's
    // "idle" means no traffic in *either* direction, so only *after*
    // sleeping past the timeout do we read once, to observe the close that
    // must already have happened.
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let closed = tokio::time::timeout(Duration::from_secs(5), wait_for_close(&mut admin)).await;
    assert!(closed.is_ok(), "AC 16(c): an idle admin socket must already be closed after the liveness timeout + margin");
    wait_for(Duration::from_secs(5), || hub.log_text().contains("admin_dropped").then_some(()))
        .expect("AC 16(c): admin_dropped must be logged for the idle-closed socket");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
    let _ = hub_state;
}

/// **AC 16(d).** One admin socket sends two `admin/*` requests with
/// different ids without waiting; the fast `admin/roster` reply arrives
/// before the slow `admin/say` reply, each carrying its own request's id.
#[tokio::test]
async fn concurrent_requests_on_one_socket_answer_in_completion_order() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "8"])]);
    let mut admin = live_socket(&body_state, &hub, &token_id, "admin-d", "admin").await;

    send_admin_say(&mut admin, "b-say-d", "b/alpha").await;
    let roster_req = serde_json::json!({ "jsonrpc": "2.0", "id": "b-roster-d", "method": "admin/roster", "params": {} });
    futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::text(roster_req.to_string()))
        .await
        .expect("send admin/roster");

    let mut order = Vec::new();
    while order.len() < 2 {
        let env = tokio::time::timeout(Duration::from_secs(15), decode_next(&mut admin)).await.expect("no timeout").expect("a response frame");
        match env {
            holler_proto::Envelope::Response { id, .. } => order.push(id),
            other => panic!("unexpected frame: {other:?}"),
        }
    }
    assert_eq!(
        order,
        vec!["b-roster-d".to_string(), "b-say-d".to_string()],
        "AC 16(d): the fast admin/roster reply must arrive before the slow admin/say reply: {order:?}"
    );

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
    let _ = hub_state;
}

/// **AC 16(e).** A client that closes its socket (clean close / an abrupt
/// drop) while its `admin/say` is in flight leaves the hub healthy: the turn
/// still completes (the session settles back to `idle` with `last_turn`
/// set), and a subsequent remote `roster`/`say` both succeed.
async fn drop_mid_say_leaves_the_hub_healthy(hub_state: &StateDir, body_state: &StateDir, hub: &Hub, token_id: &str, hostname: &str, clean: bool) {
    let mut admin = live_socket(body_state, hub, token_id, hostname, "admin").await;
    send_admin_say(&mut admin, "b-saydrop", "b/alpha").await;
    // Let the request actually reach the dispatch before pulling the rug.
    tokio::time::sleep(Duration::from_millis(100)).await;

    if clean {
        let _ = futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::Close(None)).await;
    }
    drop(admin); // the abrupt case: no close frame at all, just gone.

    wait_for(Duration::from_secs(10), || {
        let v = roster_json(hub_state);
        let rows = v.get("rows")?.as_array()?;
        rows.iter().any(|r| r["name"] == "b/alpha" && r["state"] == "idle" && !r["last_turn"].is_null()).then_some(())
    })
    .unwrap_or_else(|| panic!("AC 16(e): the session never settled back to idle with last_turn after a {}-close mid-say", if clean { "clean" } else { "abrupt" }));

    let ws_url = hub.ws_url();
    let roster_out = run(body_state, &["roster", "--server", &ws_url, "--json"]);
    assert!(roster_out.status.success(), "AC 16(e): remote roster after the drop: {}", String::from_utf8_lossy(&roster_out.stderr));
    let say_out = run(body_state, &["say", "b/alpha", "after-drop", "--server", &ws_url]);
    assert!(say_out.status.success(), "AC 16(e): remote say after the drop: {}", String::from_utf8_lossy(&say_out.stderr));
}

#[tokio::test]
async fn client_clean_close_mid_say_leaves_the_hub_healthy() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "5"])]);
    drop_mid_say_leaves_the_hub_healthy(&hub_state, &body_state, &hub, &token_id, "admin-e-clean", true).await;
    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

#[tokio::test]
async fn client_abrupt_drop_mid_say_leaves_the_hub_healthy() {
    let (hub_state, body_state, hub, body, token_id, _ws_url) = liveness_rig(&[("alpha", &["--slow", "--chunks", "5"])]);
    drop_mid_say_leaves_the_hub_healthy(&hub_state, &body_state, &hub, &token_id, "admin-e-abrupt", false).await;
    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}
