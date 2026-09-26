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

/// The hub's JSON log lines whose `type` is `ty`.
pub fn log_events(hub: &Hub, ty: &str) -> Vec<serde_json::Value> {
    hub.log_text().lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).filter(|v| v["type"] == ty).collect()
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
