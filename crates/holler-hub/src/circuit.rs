//! The hub side of a **re-authenticated** circuit (issue #182): the
//! `circuit/authenticate` → `circuit/hello` handshake, then the live session
//! loop that answers presence notifications and lets the control socket
//! (`hub token ping`) reach the body over this exact connection.
//!
//! `circuit/join` (story #176, [`crate::join`]) is the one-shot bootstrap and
//! stays completely separate: it never leads into talk on the same socket.
//! This module is what a **returning** body's socket runs through instead.

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::{
    log::{Component, Direction as LogDirection, Event, Severity},
    Authenticate, Code, Envelope, Hello, HelloRole, PingAck, Presence,
};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::{LiveCommand, Registry};
use crate::serve::{close, send_error};
use crate::state::HubState;

/// How long the hub waits for the body's half of the hello exchange, and for
/// the body's answer to the hub's own hello, before giving up on the socket.
const HELLO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

fn log(severity: Severity, method: &'static str, fields: Vec<(&'static str, String)>) {
    holler_proto::log::emit(&Event {
        component: Component::Wire,
        severity,
        direction: LogDirection::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

/// Handle a freshly-accepted socket whose first frame was `circuit/
/// authenticate`: verify the credential, run the hello exchange, and — on
/// success — hold the session loop until the body disconnects. Every failure
/// path replies with the matching error and closes the socket; a bad
/// credential is always `-32002 unauthenticated` (the body's connection loop
/// treats that code, and only that code, as "do not retry").
pub async fn handle_authenticated<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    id: Option<&str>,
    params: Authenticate,
    state: &HubState,
    registry: &Registry,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let record = match crate::token::verify_credential_async(&params.token_id, &params.credential, state).await {
        Ok(r) => r,
        Err(e) => {
            send_error(sink, id, Code::Unauthenticated, &format!("authentication failed: {e}")).await;
            close(sink).await;
            return;
        }
    };
    let Some(client_id) = record.client_id.clone() else {
        // Defensive: `verify_credential` only returns `Bound` records, which
        // always carry a `client_id`. Treated the same as a bad credential.
        send_error(sink, id, Code::Unauthenticated, "authentication failed: no client id on record").await;
        close(sink).await;
        return;
    };

    let ok = serde_json::to_value(holler_proto::AuthOk { ok: true }).unwrap_or_default();
    if reply(sink, id, ok).await.is_err() {
        return;
    }

    if hello_exchange(sink, stream, &params.hostname).await.is_err() {
        return;
    }

    log(
        Severity::Info,
        "conn_connected",
        vec![("client_id", client_id.clone()), ("hostname", params.hostname.clone())],
    );

    let mut cmd_rx = registry.insert(&client_id, &params.hostname, &params.token_id).await;
    session_loop(sink, stream, &mut cmd_rx, &client_id).await;
    registry.remove(&client_id).await;
    log(Severity::Warn, "conn_dropped", vec![("client_id", client_id)]);
}

/// Send a response envelope; `Err` means the socket is already gone (the
/// caller should stop, not close-twice).
async fn reply<Snk>(sink: &mut Snk, id: Option<&str>, result: serde_json::Value) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let Some(cid) = id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) else {
        return Err(());
    };
    let env = Envelope::response(&cid, Some(result));
    let text = holler_proto::encode(&env).unwrap_or_default();
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
}

/// The bidirectional hello exchange (issue #182 step 2): the body sends its
/// own `circuit/hello` request (answered with the hub's hello document), then
/// the hub sends its own `circuit/hello` request, which the body must answer
/// with `{}`. Either half timing out or failing to parse ends the socket.
async fn hello_exchange<Snk, St>(sink: &mut Snk, stream: &mut St, hostname: &str) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let env = tokio::time::timeout(HELLO_TIMEOUT, next_envelope(stream))
        .await
        .map_err(|_| ())?
        .ok_or(())?;
    let Envelope::Request { id, method, .. } = &env else {
        send_error(sink, env.id(), Code::Unauthenticated, "expected circuit/hello").await;
        close(sink).await;
        return Err(());
    };
    if method != "circuit/hello" {
        send_error(sink, Some(id), Code::MethodNotFound, "expected circuit/hello first").await;
        close(sink).await;
        return Err(());
    }
    let hub_hello = hub_hello_doc(hostname);
    let result = serde_json::to_value(hub_hello).unwrap_or_default();
    reply(sink, Some(id), result).await?;

    // The hub's own half: a request the body must answer with `{}`.
    let hub_cid = holler_proto::CorrelationId::mint_hub();
    let hello_req = Envelope::request(&hub_cid, "circuit/hello", Some(serde_json::json!({
        "protocol": holler_proto::PROTOCOL_VERSION,
        "protocol_min": holler_proto::PROTOCOL_MIN,
        "protocol_max": holler_proto::PROTOCOL_MAX,
        "role": "hub",
        "hostname": hub_hostname(),
    })));
    let text = holler_proto::encode(&hello_req).unwrap_or_default();
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())?;

    let ack = tokio::time::timeout(HELLO_TIMEOUT, next_envelope(stream))
        .await
        .map_err(|_| ())?
        .ok_or(())?;
    match ack {
        Envelope::Response { id, .. } if id == hub_cid.as_str() => Ok(()),
        _ => Err(()),
    }
}

/// The hub's own hostname (best-effort; `"unknown"` if unresolvable — never a
/// reason to refuse a connection).
fn hub_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// The hub's `circuit/hello` document answering the body's own hello.
fn hub_hello_doc(_body_hostname: &str) -> Hello {
    Hello {
        protocol: holler_proto::PROTOCOL_VERSION,
        protocol_min: holler_proto::PROTOCOL_MIN,
        protocol_max: holler_proto::PROTOCOL_MAX,
        role: HelloRole::Hub,
        hostname: hub_hostname(),
        token_id: None,
        client_id: None,
        features: Vec::new(),
        harnesses: None,
        harnesses_known: Some(Vec::new()),
        harnesses_confirmed: Some(Vec::new()),
        sessions: None,
    }
}

/// The live session loop: answer presence heartbeats and `circuit/ping`
/// requests from the body, and service [`LiveCommand`]s from the registry
/// (currently only `hub token ping`'s probe). Returns when the socket closes,
/// errors, or a decode failure ends the connection.
async fn session_loop<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    cmd_rx: &mut tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
    client_id: &str,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    // At most one outstanding hub-initiated ping at a time: this connection
    // serves exactly one `hub token ping` caller per round trip.
    let mut pending_ping: Option<(String, tokio::sync::oneshot::Sender<PingAck>)> = None;

    loop {
        tokio::select! {
            frame = stream.next() => {
                match frame {
                    Some(Ok(Message::Text(t))) => {
                        if handle_inbound(sink, &t, client_id, &mut pending_ping).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Binary(_))) | Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(LiveCommand::Ping { reply: reply_tx }) => {
                        match send_ping_probe(sink).await {
                            Ok(cid) => pending_ping = Some((cid, reply_tx)),
                            Err(()) => return,
                        }
                    }
                    None => return, // the registry entry was dropped/replaced.
                }
            }
        }
    }
}

/// Send a `circuit/ping` request to the body, returning its correlation id so
/// the caller can recognise the matching reply.
async fn send_ping_probe<Snk>(sink: &mut Snk) -> Result<String, ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let cid = holler_proto::CorrelationId::mint_hub();
    let req = Envelope::request(&cid, "circuit/ping", None);
    let text = holler_proto::encode(&req).unwrap_or_default();
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())?;
    Ok(cid.as_str().to_string())
}

/// Handle one inbound text frame in the live session loop. `Err` means the
/// socket should be torn down (a decode failure or a send failure).
async fn handle_inbound<Snk>(
    sink: &mut Snk,
    text: &str,
    client_id: &str,
    pending_ping: &mut Option<(String, tokio::sync::oneshot::Sender<PingAck>)>,
) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let env = match holler_proto::decode(text) {
        Ok(e) => e,
        Err(e) => {
            send_error(sink, None, e.code(), &e.to_string()).await;
            return Err(());
        }
    };
    match &env {
        Envelope::Notification { method, params } if method == "session/presence" => {
            if let Some(p) = params.clone().and_then(|v| serde_json::from_value::<Presence>(v).ok()) {
                log(Severity::Debug, "presence", vec![("client_id", client_id.to_string()), ("hostname", p.hostname)]);
            }
            Ok(())
        }
        Envelope::Request { id, method, .. } if method == "circuit/ping" => {
            let ack = PingAck { hostname: client_id.to_string(), ts: now_millis() };
            reply(sink, Some(id), serde_json::to_value(ack).unwrap_or_default()).await
        }
        Envelope::Response { id, result } => {
            if let Some((want, _)) = pending_ping.as_ref() {
                if id == want {
                    if let Some((_, tx)) = pending_ping.take() {
                        if let Some(ack) = result.clone().and_then(|v| serde_json::from_value::<PingAck>(v).ok()) {
                            let _ = tx.send(ack);
                        }
                    }
                }
            }
            Ok(())
        }
        Envelope::Request { id, .. } => {
            send_error(sink, Some(id), Code::MethodNotFound, "unknown method").await;
            Ok(())
        }
        Envelope::Notification { .. } | Envelope::Error { .. } => Ok(()),
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Read the next text frame off `stream` as a decoded [`Envelope`], skipping
/// ping/pong/raw frames. `None` on close, error, or EOF.
async fn next_envelope<St>(stream: &mut St) -> Option<Envelope>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    loop {
        match stream.next().await {
            Some(Ok(Message::Text(t))) => return holler_proto::decode(&t).ok(),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => continue,
            _ => return None,
        }
    }
}
