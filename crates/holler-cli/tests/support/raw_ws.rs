//! Raw WebSocket clients for driving a real hub without a body process
//! (issue #455): moved verbatim out of `hub_hygiene_test.rs` so
//! `hub_status_lockout_test.rs` can reuse them, plus
//! [`connect_ws_with_headers`] and [`authenticate_on`].

#![allow(dead_code)] // #455: each test binary uses a subset

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use holler_proto::noise::{build_prologue, HandshakeXk};
use holler_proto::{decode, Envelope};
use serde_json::json;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream};

use super::{Hub, StateDir};

pub type WsClient = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub async fn connect_ws(url: &str) -> WsClient {
    connect_ws_with_headers(url, &[]).await
}

/// Dial the hub with extra HTTP upgrade-request headers (issue #455: a
/// client spoofing `X-Forwarded-For` / `Forwarded`).
pub async fn connect_ws_with_headers(url: &str, headers: &[(&str, &str)]) -> WsClient {
    let mut request = url.into_client_request().expect("a valid ws:// URL");
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).expect("a valid header name");
        request.headers_mut().insert(name, HeaderValue::from_str(value).expect("a valid header value"));
    }
    tokio_tungstenite::connect_async(request).await.expect("dial the hub's WebSocket listener").0
}

/// This test binary's own resolved copy of the hub's X25519 static public
/// key (issue #322) — a raw test client needs it in advance to build a
/// Noise XK initiator (the `K` in XK), the same way a real body gets it from
/// `body join`'s out-of-band `--hub-key` pin. Safe to call after
/// `Hub::start`/`Hub::start_with_env` return (readiness implies the hub's
/// own startup, including any identity generation, has already run) —
/// `holler_hub::identity::ensure` is idempotent and loads back whatever key
/// is already persisted rather than generating a second, different one.
pub fn hub_x25519_pubkey(state: &StateDir) -> [u8; 32] {
    let hub_state = holler_hub::state::HubState::from_root(state.path().to_path_buf());
    let identity = holler_hub::identity::ensure(&hub_state).expect("resolve the hub's X25519 identity");
    let bytes = hex::decode(identity.public_hex()).expect("hub pubkey is valid hex");
    bytes.try_into().expect("hub pubkey is 32 bytes")
}

/// A `circuit/authenticate` request frame carrying Noise message 1.
pub fn authenticate_request(token_id: &str, hostname: &str, advertised_url: &str, msg1: &[u8]) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": "b-auth1",
        "method": "circuit/authenticate",
        "params": {
            "protocol": holler_proto::PROTOCOL_VERSION,
            "token_id": token_id, "hostname": hostname, "advertised_url": advertised_url, "message": hex::encode(msg1),
        },
    })
}

/// Read the next frame and decode it as a v2 envelope, skipping ping/pong.
/// `None` on close or EOF.
pub async fn decode_next(ws: &mut WsClient) -> Option<Envelope> {
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
pub async fn wait_for_close(ws: &mut WsClient) -> Option<u16> {
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

/// Send one `circuit/authenticate` on a fresh socket and read one frame. A
/// refusal is `Ok(error)`; anything else (a 1008 lockout close, a challenge)
/// is `Err(description)`, so a caller can collect every attempt and assert on
/// them together instead of dying on the first unexpected shape.
pub async fn authenticate_once(ws_url: &str, token_id: &str, key: &[u8; 32], hub_pubkey: &[u8; 32]) -> Result<holler_proto::WireError, String> {
    let mut ws = connect_ws(ws_url).await;
    authenticate_on(&mut ws, ws_url, token_id, key, hub_pubkey).await
}

/// [`authenticate_once`] on a socket the caller already dialled (for example
/// with [`connect_ws_with_headers`], issue #455).
pub async fn authenticate_on(ws: &mut WsClient, ws_url: &str, token_id: &str, key: &[u8; 32], hub_pubkey: &[u8; 32]) -> Result<holler_proto::WireError, String> {
    let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, ws_url);
    let msg1 = HandshakeXk::initiator(key, hub_pubkey, &prologue).and_then(|mut h| h.write_message()).expect("write handshake message 1");
    if let Err(e) = ws.send(Message::text(authenticate_request(token_id, "fault-body", ws_url, &msg1).to_string())).await {
        return Err(format!("send failed: {e}"));
    }
    match tokio::time::timeout(Duration::from_secs(10), ws.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => match decode(t.as_str()) {
            Ok(Envelope::Error { error, .. }) => Ok(error),
            other => Err(format!("not a refusal: {other:?}")),
        },
        other => Err(format!("no refusal frame: {other:?}")),
    }
}

/// Run the `circuit/authenticate` → `circuit/prove` Noise XK handshake
/// (issue #338) and read back the final answer. `Ok(())` is `{ok:true}`;
/// `Err(WireError)` is the hub's refusal at either step (still followed by a
/// close in every case — the caller decides whether to keep draining).
/// `advertised_url` is bound into the handshake's prologue — real callers
/// here always use the hub's own `ws://` URL (there is no MITM/relay in this
/// test's topology, only good/bad-key scenarios).
///
/// Moved out of `hub_hygiene_test.rs` (issue #508's clarifications: "do not
/// write a third copy of the initiator") so `remote_admin_test.rs` can drive
/// the same full handshake with a caller-chosen `circuit/hello` role.
pub async fn send_authenticate(
    ws: &mut WsClient,
    token_id: &str,
    body_x25519_secret: &[u8; 32],
    hub_x25519_pubkey: &[u8; 32],
    hostname: &str,
    advertised_url: &str,
) -> Result<(), holler_proto::WireError> {
    let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, advertised_url);
    let mut handshake = HandshakeXk::initiator(body_x25519_secret, hub_x25519_pubkey, &prologue).expect("build noise initiator");
    let msg1 = handshake.write_message().expect("write handshake message 1");

    let req = authenticate_request(token_id, hostname, advertised_url, &msg1);
    ws.send(Message::text(req.to_string())).await.expect("send circuit/authenticate");
    let env = decode_next(ws).await.expect("an answer to circuit/authenticate");
    let msg2_hex = match env {
        Envelope::Response { result, .. } => result
            .and_then(|v| v.get("message").and_then(|n| n.as_str()).map(String::from))
            .expect("circuit/authenticate result carries a noise handshake message"),
        Envelope::Error { error, .. } => return Err(error),
        other => panic!("unexpected reply to circuit/authenticate: {other:?}"),
    };
    let msg2 = hex::decode(&msg2_hex).expect("hex-decode handshake message 2");
    handshake.read_message(&msg2).expect("process handshake message 2");
    let msg3 = handshake.write_message().expect("write handshake message 3");

    let prove = json!({
        "jsonrpc": "2.0",
        "id": "b-prove1",
        "method": "circuit/prove",
        "params": { "token_id": token_id, "message": hex::encode(msg3) },
    });
    ws.send(Message::text(prove.to_string())).await.expect("send circuit/prove");
    let env = decode_next(ws).await.expect("an answer to circuit/prove");
    match env.error() {
        Some(e) => Err(e.clone()),
        None => Ok(()),
    }
}

/// The bidirectional `circuit/hello` exchange a live body (or, issue #508, an
/// admin client) runs right after a successful `circuit/authenticate` (see
/// `circuit::hello_exchange`): send ours with the given `role` (`"body"` or
/// `"admin"`), read the hub's answer, then answer the hub's own hello
/// request with `{}`. Panics (test failure) if either half does not arrive.
///
/// **Issue #508:** on current `main`, `hello_exchange` ignores `role`
/// entirely — any string (including `"admin"`, or one that fails to parse at
/// all) still completes the handshake as an ordinary body hello (see
/// `circuit.rs`'s `hello_exchange`, which falls through unrefused when
/// `serde_json::from_value::<Hello>` fails). This is exactly the hazard
/// `remote_admin_test.rs`'s RED tests pin: `role: "admin"` here does **not**
/// yet mean "no supersede, no roster write".
pub async fn run_hello_as(ws: &mut WsClient, hostname: &str, role: &str) {
    let hello = json!({
        "jsonrpc": "2.0",
        "id": "b-hello1",
        "method": "circuit/hello",
        "params": {
            "protocol": holler_proto::PROTOCOL_VERSION,
            "protocol_min": holler_proto::PROTOCOL_MIN,
            "protocol_max": holler_proto::PROTOCOL_MAX,
            "role": role, "hostname": hostname, "harnesses": [],
        },
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

/// [`run_hello_as`] with role `"body"` — today's ordinary body hello.
pub async fn run_hello(ws: &mut WsClient, hostname: &str) {
    run_hello_as(ws, hostname, "body").await;
}

/// Authenticate + hello (with a caller-chosen `circuit/hello` role) in one
/// call — the full handshake a real body (`role: "body"`) or, issue #508, an
/// admin client (`role: "admin"`) runs to reach the live session loop.
pub async fn go_live_as(
    ws: &mut WsClient,
    token_id: &str,
    body_x25519_secret: &[u8; 32],
    hub_x25519_pubkey: &[u8; 32],
    hostname: &str,
    advertised_url: &str,
    role: &str,
) {
    send_authenticate(ws, token_id, body_x25519_secret, hub_x25519_pubkey, hostname, advertised_url)
        .await
        .expect("authenticate must succeed");
    run_hello_as(ws, hostname, role).await;
}

/// [`go_live_as`] with role `"body"` — today's ordinary body handshake.
pub async fn go_live(ws: &mut WsClient, token_id: &str, body_x25519_secret: &[u8; 32], hub_x25519_pubkey: &[u8; 32], hostname: &str, advertised_url: &str) {
    go_live_as(ws, token_id, body_x25519_secret, hub_x25519_pubkey, hostname, advertised_url, "body").await;
}

/// The hub's JSON log lines whose `type` is `ty`.
pub fn log_events(hub: &Hub, ty: &str) -> Vec<serde_json::Value> {
    hub.log_text().lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).filter(|v| v["type"] == ty).collect()
}

/// One full-handshake admin (or body) socket already past the
/// `circuit/hello` exchange (issue #508): reads the joined credential's
/// x25519 identity off `state` (the body's own dir — `state/body/
/// x25519_identity.key`), reads the hub's pubkey from that **same joined
/// credential's own pinned `hub_pubkey`** (never `hub_x25519_pubkey`, which
/// resolves a hub identity from `state`'s `hub/` subtree — wrong, and
/// silently *generates a fresh, different* hub identity, once `state` is a
/// body-only dir with no hub state of its own, AC 11's whole point), dials
/// and runs [`go_live_as`] with `role`. Shared by `remote_admin_test.rs` and
/// `remote_admin_liveness_test.rs` (handoff-S round-2, A-dup W-2: previously
/// duplicated five times in one file, and the same shape a sixth/seventh
/// time would need in the other).
pub async fn live_socket(state: &StateDir, hub: &Hub, token_id: &str, hostname: &str, role: &str) -> WsClient {
    let identity = holler_body::identity::load(state.path())
        .expect("this state dir has a joined body/credential.json")
        .expect("credential.json is readable");
    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the joined credential's x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey: [u8; 32] = hex::decode(&identity.hub_pubkey)
        .ok()
        .and_then(|b| b.try_into().ok())
        .expect("the joined credential's pinned hub_pubkey is 32 bytes of hex");
    let ws_url = hub.ws_url();
    let mut ws = connect_ws(&ws_url).await;
    go_live_as(&mut ws, token_id, &secret_bytes, &hub_pubkey, hostname, &ws_url, role).await;
    ws
}

/// Send `circuit/authenticate` for `token_id` on `ws` and read what comes
/// back, skipping ping/pong. `Ok(code)` is a close that arrived **before any
/// application frame** (issue #455: the token-scoped lockout answers with a
/// bare close 1008, no JSON-RPC error); `Err` describes the first
/// application frame, or an EOF with no close frame. A send error is
/// ignored: a peer-wide refusal may already have closed the socket.
pub async fn authenticate_expecting_bare_close(ws: &mut WsClient, ws_url: &str, token_id: &str, key: &[u8; 32], hub_pubkey: &[u8; 32]) -> Result<u16, String> {
    let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, ws_url);
    let msg1 = HandshakeXk::initiator(key, hub_pubkey, &prologue).and_then(|mut h| h.write_message()).expect("write handshake message 1");
    let _ = ws.send(Message::text(authenticate_request(token_id, "lockout-body", ws_url, &msg1).to_string())).await;
    loop {
        match tokio::time::timeout(Duration::from_secs(10), ws.next()).await {
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)))) => continue,
            Ok(Some(Ok(Message::Close(Some(frame))))) => return Ok(frame.code.into()),
            other => return Err(format!("expected a bare close, got {other:?}")),
        }
    }
}
