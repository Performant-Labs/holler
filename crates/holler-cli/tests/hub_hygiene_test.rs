//! Connection hygiene / registry / lockout e2e (issue #184).
//!
//! A **fresh implementation against current `main`**, not a rebase of the
//! stale `feat/184-hub-registry` (PR #223) — see that PR and PR #289
//! (`fix/224-spurious-lockout-oneshot-panic`) for the design this ports:
//! `feat/184-hub-registry`'s `registry.rs`/`lockout.rs` for the supersede/
//! revoke/lockout shape, and #289 for the `PeerState::tripped_since` design
//! (a real bug it found and fixed on that stale branch) and the CI-only race
//! its own `five_bad_auths_lock_out_peer_for_window` hit (fixed here the same
//! way: never send anything from the probing connection — only listen for
//! the hub's own unprompted close).
//!
//! Drives the real binary with raw `tokio-tungstenite` clients (no body
//! process) against a real hub, the same harness conventions as
//! `hub_serve_test.rs`: every test gets its own [`StateDir`], readiness is
//! observed via [`wait_for`], hygiene tunables are set per-test via
//! [`Hub::start_with_env`] (never the ambient environment).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #184

use std::time::Duration;

use ed25519_dalek::{Signer, SigningKey};
use futures_util::{SinkExt, StreamExt};
use holler_proto::{decode, Envelope};
use serde_json::json;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream};

mod support;
use support::{holler_cmd, hub_status_json, wait_for, Hub, StateDir, STARTUP_WAIT};

type WsClient = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_ws(url: &str) -> WsClient {
    tokio_tungstenite::connect_async(url).await.expect("dial the hub's WebSocket listener").0
}

/// A fresh Ed25519 signing keypair, deterministic per `seed` (so tests stay
/// reproducible) — what a raw test client registers at join and signs
/// `circuit/prove` with (issue #323).
fn fresh_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// Mint a token and redeem it in-process (no subprocess, no live hub
/// required for this half) via the `holler_hub` library directly — generates
/// a fresh Ed25519 keypair, registers its public half, and returns
/// `(token_id, signing_key)`, the pair a raw client authenticates with
/// (issue #323: no more bearer credential).
fn mint_and_redeem(state: &StateDir, label: &str) -> (String, SigningKey) {
    let hub_state = holler_hub::state::HubState::from_root(state.path().to_path_buf());
    let minted = holler_hub::token::mint(label, 24 * 3600, &hub_state).expect("mint a token");
    // Deterministic-but-distinct seed per label so concurrently-minted tokens
    // in the same test never collide on the same keypair.
    let seed = label.bytes().fold(0u8, |a, b| a.wrapping_add(b)).wrapping_add(1);
    let signing_key = fresh_signing_key(seed);
    let body_pubkey = hex::encode(signing_key.verifying_key().to_bytes());
    let client_id = holler_hub::token::redeem(&minted.secret, label, &body_pubkey, &hub_state)
        .expect("redeem the just-minted token");
    let _ = client_id;
    (minted.record.token_id, signing_key)
}

/// Run the `circuit/authenticate` → `circuit/prove` challenge-response
/// (issue #323) and read back the final answer. `Ok(())` is `{ok:true}`;
/// `Err(WireError)` is the hub's refusal at either step (still followed by a
/// close in every case — the caller decides whether to keep draining).
/// `advertised_url` is bound into the signed transcript — real callers here
/// always use the hub's own `ws://` URL (there is no MITM/relay in this
/// test's topology, only good/bad-signature scenarios).
async fn send_authenticate(
    ws: &mut WsClient,
    token_id: &str,
    signing_key: &SigningKey,
    hostname: &str,
    advertised_url: &str,
) -> Result<(), holler_proto::WireError> {
    let req = json!({
        "jsonrpc": "2.0",
        "id": "b-auth1",
        "method": "circuit/authenticate",
        "params": { "token_id": token_id, "hostname": hostname, "advertised_url": advertised_url },
    });
    ws.send(Message::text(req.to_string())).await.expect("send circuit/authenticate");
    let env = decode_next(ws).await.expect("an answer to circuit/authenticate");
    let nonce = match env {
        Envelope::Response { result, .. } => result
            .and_then(|v| v.get("nonce").and_then(|n| n.as_str()).map(String::from))
            .expect("circuit/authenticate result carries a nonce challenge"),
        Envelope::Error { error, .. } => return Err(error),
        other => panic!("unexpected reply to circuit/authenticate: {other:?}"),
    };

    let transcript = holler_proto::transcript::build(holler_proto::PROTOCOL_VERSION, "body", &nonce, token_id, advertised_url);
    let signature = signing_key.sign(&transcript);
    let prove = json!({
        "jsonrpc": "2.0",
        "id": "b-prove1",
        "method": "circuit/prove",
        "params": { "token_id": token_id, "signature": hex::encode(signature.to_bytes()) },
    });
    ws.send(Message::text(prove.to_string())).await.expect("send circuit/prove");
    let env = decode_next(ws).await.expect("an answer to circuit/prove");
    match env.error() {
        Some(e) => Err(e.clone()),
        None => Ok(()),
    }
}

/// The bidirectional `circuit/hello` exchange a live body runs right after a
/// successful `circuit/authenticate` (see `circuit::hello_exchange`): send
/// ours, read the hub's answer, then answer the hub's own hello request with
/// `{}`. Panics (test failure) if either half does not arrive.
async fn run_hello(ws: &mut WsClient, hostname: &str) {
    let hello = json!({
        "jsonrpc": "2.0",
        "id": "b-hello1",
        "method": "circuit/hello",
        "params": { "protocol": 2, "protocol_min": 2, "protocol_max": 2, "role": "body", "hostname": hostname, "harnesses": [] },
    });
    ws.send(Message::text(hello.to_string())).await.expect("send circuit/hello");
    let _ = decode_next(ws).await.expect("the hub's answer to our circuit/hello");

    // The hub's own half: a request we must answer with `{}`.
    let hub_hello = decode_next(ws).await.expect("the hub's own circuit/hello request");
    let Envelope::Request { id, .. } = hub_hello else {
        panic!("expected the hub's circuit/hello request, got {hub_hello:?}");
    };
    let ack = json!({ "jsonrpc": "2.0", "id": id, "result": {} });
    ws.send(Message::text(ack.to_string())).await.expect("ack the hub's hello");
}

/// Authenticate + hello in one call: the full handshake a real body runs to
/// reach the live session loop.
async fn go_live(ws: &mut WsClient, token_id: &str, signing_key: &SigningKey, hostname: &str, advertised_url: &str) {
    send_authenticate(ws, token_id, signing_key, hostname, advertised_url)
        .await
        .expect("authenticate must succeed");
    run_hello(ws, hostname).await;
}

/// Read the next frame and decode it as a v2 envelope, skipping ping/pong.
/// `None` on close or EOF.
async fn decode_next(ws: &mut WsClient) -> Option<Envelope> {
    loop {
        match ws.next().await {
            None => return None,
            Some(Ok(Message::Text(t))) => return Some(decode(t.as_str()).expect("a valid v2 frame")),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => continue,
            Some(Ok(Message::Close(_))) => return None,
            Some(Ok(Message::Binary(_))) => panic!("unexpected binary frame"),
            Some(Err(e)) => panic!("ws stream error: {e:?}"),
        }
    }
}

/// Read frames until a WS Close arrives (or EOF), returning the close code if
/// the peer sent one. Skips any application frames along the way (a
/// superseded/revoked connection may have one pending notification first).
async fn wait_for_close(ws: &mut WsClient) -> Option<u16> {
    loop {
        match ws.next().await {
            None => return None, // EOF with no explicit close frame.
            Some(Ok(Message::Close(Some(frame)))) => return Some(frame.code.into()),
            Some(Ok(Message::Close(None))) => return None,
            Some(Ok(_)) => continue, // an application frame (e.g. circuit/superseded) — keep draining.
            Some(Err(_)) => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// registry: supersede-on-reauth
// ---------------------------------------------------------------------------

/// A second `circuit/authenticate` for the same token sends the **old**
/// socket a `circuit/superseded` notification, then closes it with WS code
/// **1000** — and the new socket is admitted normally.
#[tokio::test]
async fn reauth_supersedes_and_closes_old_socket_with_notification() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, signing_key) = mint_and_redeem(&state, "reauth-body");
    let ws_url = hub.ws_url();

    let mut old = connect_ws(&ws_url).await;
    go_live(&mut old, &token_id, &signing_key, "reauth-body", &ws_url).await;

    let mut fresh = connect_ws(&ws_url).await;
    go_live(&mut fresh, &token_id, &signing_key, "reauth-body", &ws_url).await;

    // The old socket must see the notification, then a 1000 close.
    let note = decode_next(&mut old).await.expect("the old socket must receive circuit/superseded");
    assert_eq!(note.method(), Some("circuit/superseded"), "unexpected frame on the old socket: {note:?}");
    let code = wait_for_close(&mut old).await;
    assert_eq!(code, Some(1000), "the old socket must close with code 1000");

    // The new socket is fully live: a `circuit/ping` still round-trips.
    let ping = json!({ "jsonrpc": "2.0", "id": "b-ping1", "method": "circuit/ping", "params": {} });
    fresh.send(Message::text(ping.to_string())).await.expect("send circuit/ping on the new socket");
    let ack = decode_next(&mut fresh).await.expect("the new socket must still answer circuit/ping");
    assert!(ack.result().is_some(), "circuit/ping must succeed on the surviving socket: {ack:?}");
}

// ---------------------------------------------------------------------------
// registry: revoke force-closes
// ---------------------------------------------------------------------------

/// `hub token revoke ID` on a bound, connected token closes the socket
/// immediately with WS code **1008**; a reconnect attempt then fails
/// `-32002`.
#[tokio::test]
async fn revoke_force_closes_live_connection() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, signing_key) = mint_and_redeem(&state, "revoke-body");
    let ws_url = hub.ws_url();

    let mut ws = connect_ws(&ws_url).await;
    go_live(&mut ws, &token_id, &signing_key, "revoke-body", &ws_url).await;

    let out = holler_cmd(&state)
        .args(["hub", "token", "revoke", &token_id])
        .output()
        .expect("run `hub token revoke`");
    assert!(out.status.success(), "hub token revoke must succeed: {}", String::from_utf8_lossy(&out.stderr));

    let code = wait_for_close(&mut ws).await;
    assert_eq!(code, Some(1008), "a revoked live socket must close with code 1008");

    // Reconnecting to a revoked token fails `-32002` (issue #323's
    // `revoked_token_still_fails_closed_with_32002`: existing behaviour
    // survives the credential → public-key redesign).
    let mut retry = connect_ws(&ws_url).await;
    let err = send_authenticate(&mut retry, &token_id, &signing_key, "revoke-body", &ws_url)
        .await
        .expect_err("re-authenticating a revoked token must fail");
    assert_eq!(err.code, -32002, "a revoked token must fail closed: {err:?}");
}

// ---------------------------------------------------------------------------
// hygiene: oversized frame
// ---------------------------------------------------------------------------

/// A frame over `HOLLER_MAX_FRAME_BYTES` closes the socket with **1009**.
#[tokio::test]
async fn oversized_frame_closes_1009() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_MAX_FRAME_BYTES", "1024")]);

    let mut ws = connect_ws(&hub.ws_url()).await;
    // A syntactically-valid (if oversized) v2 request: the codec never even
    // sees it — the frame is rejected at the WebSocket layer before decode.
    let huge = "x".repeat(4096);
    let req = json!({ "jsonrpc": "2.0", "id": "b-huge", "method": "query/status", "params": { "junk": huge } });
    ws.send(Message::text(req.to_string())).await.expect("send the oversized frame");

    let code = wait_for_close(&mut ws).await;
    assert_eq!(code, Some(1009), "an oversized frame must close with code 1009");
}

// ---------------------------------------------------------------------------
// hygiene: pre-auth timeout
// ---------------------------------------------------------------------------

/// A socket that sends nothing at all is closed once `HOLLER_PRE_AUTH_TIMEOUT_MS`
/// elapses.
#[tokio::test]
async fn silent_socket_closed_after_pre_auth_timeout() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_PRE_AUTH_TIMEOUT_MS", "500")]);

    let mut ws = connect_ws(&hub.ws_url()).await;
    // Send nothing. The hub must close this socket on its own within ~500ms.
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        matches!(ws.next().await, None | Some(Ok(Message::Close(_))))
    })
    .await
    .unwrap_or(false);
    assert!(closed, "a silent socket must be closed once the pre-auth timeout elapses");
}

// ---------------------------------------------------------------------------
// hygiene: unauthenticated-connection cap
// ---------------------------------------------------------------------------

/// The 5th unauthenticated connection over a cap of 4 is refused at once
/// with **1013**, before the hub reads a frame from it.
#[tokio::test]
async fn preauth_cap_rejects_65th_socket() {
    let state = StateDir::new();
    // A generous pre-auth timeout: the 4 slot-holding sockets must not be
    // reclaimed by their own timeout mid-test.
    let hub = Hub::start_with_env(
        &state,
        &[("HOLLER_MAX_PREAUTH_CONNECTIONS", "4"), ("HOLLER_PRE_AUTH_TIMEOUT_MS", "30000")],
    );

    // 4 sockets that connect and send nothing (holding their pre-auth slot).
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(connect_ws(&hub.ws_url()).await);
    }

    // The 5th must be refused immediately (1013), before reading a frame.
    let mut fifth = connect_ws(&hub.ws_url()).await;
    let code = wait_for_close(&mut fifth).await;
    assert_eq!(code, Some(1013), "the connection over the pre-auth cap must be refused with code 1013");

    // The 4 held sockets are unaffected (still open — no close observed yet).
    for ws in &mut held {
        let immediate = tokio::time::timeout(Duration::from_millis(200), ws.next()).await;
        assert!(immediate.is_err(), "a slot-holding socket under the cap must not be closed");
    }
}

// ---------------------------------------------------------------------------
// hygiene: failed-auth lockout
// ---------------------------------------------------------------------------

/// 5 failed `circuit/authenticate`s from one peer lock that peer out for the
/// configured window; a 6th connection attempt is refused with **1008**
/// before the hub reads a frame; after the window lapses, connections are
/// accepted again.
#[tokio::test]
async fn five_bad_auths_lock_out_peer_for_window() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(
        &state,
        &[("HOLLER_LOCKOUT_MAX_FAILURES", "5"), ("HOLLER_LOCKOUT_WINDOW_MS", "2000"), ("HOLLER_LOCKOUT_DURATION_MS", "2000")],
    );
    let (token_id, _signing_key) = mint_and_redeem(&state, "lockout-body");
    let ws_url = hub.ws_url();
    // A signature-proving keypair that was never registered on `token_id` —
    // the issue #323 equivalent of "wrong credential": the signature it
    // produces can never verify against the record's real `body_pubkey`.
    let wrong_key = fresh_signing_key(200);

    for i in 0..5 {
        let mut ws = connect_ws(&ws_url).await;
        let err = send_authenticate(&mut ws, &token_id, &wrong_key, "lockout-body", &ws_url)
            .await
            .expect_err("a bad signature must fail authentication");
        assert_eq!(err.code, -32002, "bad-auth attempt {i} must be -32002: {err:?}");
    }

    // The 6th connection attempt from the same peer (loopback — NOT exempt)
    // must be refused immediately, before the hub ever reads a frame from
    // it. Per PR #289's own CI-only-race fix: do not send or drop anything
    // from this connection — hold the whole split socket open and only
    // *listen* for the hub's unprompted close (sending our own close first
    // races the hub's close on a fast/lean CI runner).
    let mut sixth = connect_ws(&hub.ws_url()).await;
    let code = wait_for_close(&mut sixth).await;
    assert_eq!(code, Some(1008), "a locked-out peer's connection must be refused with code 1008");
    drop(sixth);

    // After the window lapses, the peer is admitted again: a bad-credential
    // attempt now gets the *normal* auth-failure roundtrip (an error frame,
    // not an immediate pre-frame close) — proof the lockout itself lifted,
    // not just that this one attempt happened to look different.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let mut retry = connect_ws(&ws_url).await;
    let err = send_authenticate(&mut retry, &token_id, &wrong_key, "lockout-body", &ws_url)
        .await
        .expect_err("still a bad signature");
    assert_eq!(err.code, -32002, "after the cooldown, auth failures are answered normally again: {err:?}");
}

// ---------------------------------------------------------------------------
// hygiene: slow token store does not stall a sibling connection
// ---------------------------------------------------------------------------

/// A 200ms-slow token-store lookup (the `HOLLER_TEST_HOOKS`-gated delay hook)
/// on one connection's `circuit/authenticate` must not delay a *sibling*,
/// already-live connection's own `circuit/ping` round trip — proof the store
/// call runs on `spawn_blocking`'s dedicated pool, not the connection's own
/// executor thread.
#[tokio::test]
async fn slow_token_store_does_not_stall_sibling_ping() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_TEST_HOOKS", "1"), ("HOLLER_TEST_TOKEN_STORE_DELAY_MS", "200")]);
    let (sibling_token, sibling_key) = mint_and_redeem(&state, "sibling-body");
    let (slow_token, slow_key) = mint_and_redeem(&state, "slow-body");
    let ws_url = hub.ws_url();

    // Bring the sibling fully live first (its own authenticate also pays the
    // store delay — twice, once per `circuit/authenticate`/`circuit/prove`
    // step — that is not what is being measured).
    let mut sibling = connect_ws(&ws_url).await;
    go_live(&mut sibling, &sibling_token, &sibling_key, "sibling-body", &ws_url).await;

    // Start a second connection's authenticate concurrently (it will sit on
    // the delayed store for ~200ms per step) without awaiting it yet.
    let slow_url = ws_url.clone();
    let slow_handle = tokio::spawn(async move {
        let mut ws = connect_ws(&slow_url).await;
        go_live(&mut ws, &slow_token, &slow_key, "slow-body", &slow_url).await;
        ws
    });

    // Give the slow authenticate a moment to actually be in flight on the
    // blocking pool, then measure the sibling's own `circuit/ping` — purely
    // local (no store access at all) — round trip.
    tokio::time::sleep(Duration::from_millis(30)).await;
    let started = std::time::Instant::now();
    let ping = json!({ "jsonrpc": "2.0", "id": "b-sibping", "method": "circuit/ping", "params": {} });
    sibling.send(Message::text(ping.to_string())).await.expect("send circuit/ping");
    let ack = decode_next(&mut sibling).await.expect("the sibling must answer circuit/ping promptly");
    let elapsed = started.elapsed();
    assert!(ack.result().is_some(), "circuit/ping must succeed: {ack:?}");
    assert!(
        elapsed < Duration::from_millis(150),
        "a sibling's circuit/ping took {elapsed:?} while a 200ms store lookup was in flight elsewhere — \
         looks like the store call is blocking the connection's own executor thread"
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), slow_handle).await.expect("the slow authenticate must still finish");
}

// ---------------------------------------------------------------------------
// hygiene: peer address in status
// ---------------------------------------------------------------------------

/// `hub status --json`'s `clients_detail[].peer` reports the connection's
/// real transport peer address.
#[tokio::test]
async fn peer_addr_present_in_status_json() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, signing_key) = mint_and_redeem(&state, "peer-body");
    let ws_url = hub.ws_url();

    let mut ws = connect_ws(&ws_url).await;
    go_live(&mut ws, &token_id, &signing_key, "peer-body", &ws_url).await;

    let doc = wait_for(STARTUP_WAIT, || {
        let d = hub_status_json(&state);
        let has_client = d.get("clients_detail").and_then(|c| c.as_array()).is_some_and(|a| !a.is_empty());
        has_client.then_some(d)
    })
    .expect("hub status must report the live connection");

    let peer = doc["clients_detail"][0]["peer"].as_str().expect("clients_detail[0].peer is a string");
    assert!(peer.starts_with("127.0.0.1:"), "peer must be the real loopback transport address, got {peer:?}");
}

// ---------------------------------------------------------------------------
// regression: issue #224's double-poll panic (minimal repro)
// ---------------------------------------------------------------------------

/// One fresh hub, one legitimate `circuit/authenticate` — must not panic and
/// must return a real success response. The minimal repro of the bug PR #289
/// found and fixed on the (unmerged) stale branch: a connection's teardown
/// path polling an already-resolved completion channel a second time.
#[tokio::test]
async fn fresh_hub_single_authenticate_does_not_panic() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, signing_key) = mint_and_redeem(&state, "solo-body");
    let ws_url = hub.ws_url();

    let mut ws = connect_ws(&ws_url).await;
    send_authenticate(&mut ws, &token_id, &signing_key, "solo-body", &ws_url)
        .await
        .expect("a legitimate authenticate must succeed with no panic");

    // The hub process must still be alive and answering afterward.
    let doc = hub_status_json(&state);
    assert_eq!(doc["role"], "hub", "the hub must still be responsive: {doc}");
}
