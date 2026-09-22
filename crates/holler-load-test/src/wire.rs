//! A real circuit client, in-process (issue #369 harness bullet 2's
//! connection half).
//!
//! **This is not a mock of a body.** It dials the hub's real WebSocket
//! listener and runs the real, shipping circuit handshake — the Noise XK
//! `circuit/authenticate` → `circuit/prove` exchange (issue #338) followed by
//! the bidirectional `circuit/hello` (issue #182) — then holds the connection
//! open with the same `session/presence` + WS-Ping heartbeat a live `holler
//! body run` emits. From the hub's side it is indistinguishable from a body:
//! it occupies a registry slot, it counts in `hub status --json`'s `clients`,
//! and it is torn down through the same paths.
//!
//! What it deliberately is *not* is a `holler body run` **process**. Scenario
//! 1 (#370) ramps to 200 concurrent connections and measures the handshake
//! itself; 200 body processes (plus their 200 `stub-acp` children) would both
//! swamp the machine and, more importantly, make the per-connection handshake
//! latency #370 asks for unmeasurable — a subprocess cannot report the timing
//! of its own internal handshake steps. Scenarios that need real agent
//! sessions use the process fleet in `fleet.rs` instead.

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use holler_proto::noise::{build_prologue, HandshakeXk};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::MaybeTlsStream;

use crate::Res;

type WsClient = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// One client's long-lived key material.
pub struct ClientIdentity {
    ed25519: ed25519_dalek::SigningKey,
    x25519_secret: [u8; 32],
}

impl ClientIdentity {
    /// A fresh identity derived deterministically from `seed`, so a rerun of
    /// the same ramp uses the same keys and a diff between two runs is a diff
    /// in the hub's behaviour, not in the harness's randomness.
    pub fn derive(seed: u64) -> Self {
        let mut bytes = [0u8; 32];
        for (i, slot) in bytes.iter_mut().enumerate() {
            *slot = ((seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i as u64)) >> 11) as u8;
        }
        // Distinct derivation per algorithm so the two keypairs are unrelated.
        let mut ed_bytes = bytes;
        ed_bytes[0] ^= 0xA5;
        Self {
            ed25519: ed25519_dalek::SigningKey::from_bytes(&ed_bytes),
            x25519_secret: bytes,
        }
    }

    pub fn ed25519_pubkey_hex(&self) -> String {
        hex::encode(self.ed25519.verifying_key().to_bytes())
    }

    pub fn x25519_pubkey_hex(&self) -> String {
        let secret = x25519_dalek::StaticSecret::from(self.x25519_secret);
        hex::encode(x25519_dalek::PublicKey::from(&secret).to_bytes())
    }
}

/// The per-connection timings #370 asks for.
#[derive(Debug, Clone, Copy)]
pub struct ConnectTimings {
    /// TCP connect + WebSocket upgrade.
    pub ws_connect: Duration,
    /// `circuit/authenticate` → `circuit/prove` → `circuit/hello`.
    pub handshake: Duration,
}

impl ConnectTimings {
    pub fn total(&self) -> Duration {
        self.ws_connect + self.handshake
    }
}

/// Dial the hub and run the full handshake, timing each half.
pub async fn connect_and_handshake(
    ws_url: &str,
    token_id: &str,
    identity: &ClientIdentity,
    hub_pubkey: &[u8; 32],
    hostname: &str,
) -> Res<(WsClient, ConnectTimings)> {
    let dial_started = Instant::now();
    let (mut ws, _) = tokio_tungstenite::connect_async(ws_url).await?;
    let ws_connect = dial_started.elapsed();

    let handshake_started = Instant::now();
    authenticate(&mut ws, token_id, identity, hub_pubkey, hostname, ws_url).await?;
    hello(&mut ws, hostname).await?;
    let handshake = handshake_started.elapsed();

    Ok((ws, ConnectTimings { ws_connect, handshake }))
}

/// The Noise XK half: `circuit/authenticate` (message 1), read the hub's
/// message 2, then `circuit/prove` (message 3).
async fn authenticate(
    ws: &mut WsClient,
    token_id: &str,
    identity: &ClientIdentity,
    hub_pubkey: &[u8; 32],
    hostname: &str,
    advertised_url: &str,
) -> Res<()> {
    let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, advertised_url);
    let mut handshake = HandshakeXk::initiator(&identity.x25519_secret, hub_pubkey, &prologue)?;
    let msg1 = handshake.write_message()?;

    send(
        ws,
        &json!({
            "jsonrpc": "2.0",
            "id": "b-auth1",
            "method": "circuit/authenticate",
            "params": {
                "protocol": holler_proto::PROTOCOL_VERSION,
                "token_id": token_id,
                "hostname": hostname,
                "advertised_url": advertised_url,
                "message": hex::encode(msg1),
            },
        }),
    )
    .await?;

    let reply = next_frame(ws).await?;
    if let Some(err) = reply.get("error") {
        return Err(format!("circuit/authenticate refused: {err}").into());
    }
    let msg2_hex = reply
        .get("result")
        .and_then(|r| r.get("message"))
        .and_then(Value::as_str)
        .ok_or("circuit/authenticate result carried no handshake message")?;
    handshake.read_message(&hex::decode(msg2_hex)?)?;
    let msg3 = handshake.write_message()?;

    send(
        ws,
        &json!({
            "jsonrpc": "2.0",
            "id": "b-prove1",
            "method": "circuit/prove",
            "params": { "token_id": token_id, "message": hex::encode(msg3) },
        }),
    )
    .await?;
    let reply = next_frame(ws).await?;
    if let Some(err) = reply.get("error") {
        return Err(format!("circuit/prove refused: {err}").into());
    }
    Ok(())
}

/// The bidirectional `circuit/hello`: ours, and the hub's own request back,
/// which must be answered before the connection is considered live.
///
/// The two can arrive in either order, so this drains frames until both halves
/// are accounted for rather than assuming a sequence.
async fn hello(ws: &mut WsClient, hostname: &str) -> Res<()> {
    send(
        ws,
        &json!({
            "jsonrpc": "2.0",
            "id": "b-hello1",
            "method": "circuit/hello",
            "params": {
                "protocol": holler_proto::PROTOCOL_VERSION,
                "protocol_min": holler_proto::PROTOCOL_MIN,
                "protocol_max": holler_proto::PROTOCOL_MAX,
                "role": "body",
                "hostname": hostname,
                // Empty: this client hosts no harnesses, so the hub's
                // confirmation pass (issue #185) has nothing to probe. A
                // harness it cannot confirm would be a lie on the wire.
                "harnesses": [],
            },
        }),
    )
    .await?;

    let mut ours_answered = false;
    let mut theirs_answered = false;
    while !(ours_answered && theirs_answered) {
        let frame = next_frame(ws).await?;
        if let Some(err) = frame.get("error") {
            return Err(format!("circuit/hello refused: {err}").into());
        }
        let id = frame.get("id").cloned();
        if frame.get("method").and_then(Value::as_str) == Some("circuit/hello") {
            let Some(id) = id else { continue };
            send(ws, &json!({ "jsonrpc": "2.0", "id": id, "result": {} })).await?;
            theirs_answered = true;
        } else if id.as_ref().and_then(Value::as_str) == Some("b-hello1") {
            ours_answered = true;
        }
    }
    Ok(())
}

/// Hold the connection live until `shutdown` flips, heartbeating exactly the
/// way `holler_body::connection` does: a `session/presence` notification plus
/// a WS-level Ping on every beat, and an answer to every request the hub
/// sends (`circuit/ping`, the liveness probe `hub token ping` drives).
///
/// Returns `Err` if the hub dropped the connection before the shutdown signal
/// — a silent drop is exactly what #370's `clients` check exists to catch, so
/// it must be reported, never swallowed.
pub async fn hold_live(
    mut ws: WsClient,
    hostname: &str,
    heartbeat: Duration,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
) -> Res<()> {
    let presence = json!({
        "jsonrpc": "2.0",
        "method": "session/presence",
        "params": { "hostname": hostname, "sessions": [] },
    });
    send(&mut ws, &presence).await?;

    let mut beat = tokio::time::interval(heartbeat);
    beat.tick().await; // The first tick is immediate; presence just went out.
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    let _ = ws.close(None).await;
                    return Ok(());
                }
            }
            _ = beat.tick() => {
                send(&mut ws, &presence).await?;
                ws.send(Message::Ping(Vec::new().into())).await?;
            }
            frame = ws.next() => {
                match frame {
                    None => return Err("hub closed the connection".into()),
                    Some(Err(e)) => return Err(format!("websocket error: {e}").into()),
                    Some(Ok(Message::Close(_))) => return Err("hub sent a close frame".into()),
                    Some(Ok(Message::Text(text))) => {
                        // Answer any request the hub makes; ignore everything
                        // else (responses to our own presence, notifications).
                        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
                        if v.get("method").is_some() {
                            if let Some(id) = v.get("id").cloned() {
                                send(&mut ws, &json!({ "jsonrpc": "2.0", "id": id, "result": {} })).await?;
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

async fn send(ws: &mut WsClient, frame: &Value) -> Res<()> {
    ws.send(Message::text(frame.to_string())).await?;
    Ok(())
}

/// The next application frame, skipping WS control frames. Bounded: a hub that
/// answers nothing must fail this client, not hang the whole ramp.
async fn next_frame(ws: &mut WsClient) -> Res<Value> {
    const FRAME_TIMEOUT: Duration = Duration::from_secs(30);
    let read = async {
        loop {
            match ws.next().await {
                None => return Err::<Value, crate::Error>("connection closed mid-handshake".into()),
                Some(Err(e)) => return Err(format!("websocket error: {e}").into()),
                Some(Ok(Message::Text(text))) => return Ok(serde_json::from_str::<Value>(&text)?),
                Some(Ok(Message::Close(_))) => return Err("hub closed the connection mid-handshake".into()),
                Some(Ok(_)) => continue,
            }
        }
    };
    match tokio::time::timeout(FRAME_TIMEOUT, read).await {
        Ok(v) => v,
        Err(_) => Err(format!("no frame from the hub within {FRAME_TIMEOUT:?}").into()),
    }
}
