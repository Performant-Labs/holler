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
    Authenticate, Code, Envelope, Hello, HelloRole, PingAck, Presence, WireError,
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

    let body_harnesses = match hello_exchange(sink, stream, &params.hostname).await {
        Ok(harnesses) => harnesses,
        Err(()) => return,
    };

    log(
        Severity::Info,
        "conn_connected",
        vec![("client_id", client_id.clone()), ("hostname", params.hostname.clone())],
    );

    let mut cmd_rx = registry.insert(&client_id, &params.hostname, &params.token_id).await;
    registry.set_harnesses_advertised(&client_id, body_harnesses.clone()).await;

    // The confirmation pass (issue #185): for each harness the body just
    // advertised, send a real `query/support` probe and record `confirmed`
    // only on `ok:true`. The probes are fired here (all at once, fire-and-
    // forget) but their **answers** are matched inside `session_loop`'s own
    // frame dispatch, alongside `circuit/ping`/presence/`hub query` traffic —
    // not read by a separate blocking loop beforehand, which would race the
    // body's own immediate `session/presence` send (a real defect this
    // story's own e2e test caught: a pre-loop read can just as easily pick up
    // that presence notification as the probe's response, silently losing
    // the confirmation). A body that lies in its hello (or whose harness
    // stops resolving) simply never gets confirmed — `hub status`'s
    // `harnesses_confirmed` stays the trustworthy subset of `harnesses_known`.
    let pending_confirms = send_confirm_probes(sink, &body_harnesses).await;

    session_loop(sink, stream, &mut cmd_rx, &client_id, registry, pending_confirms).await;
    registry.remove(&client_id).await;
    log(Severity::Warn, "conn_dropped", vec![("client_id", client_id)]);
}

/// Send one `query/support {feature: harness}` request per harness, returning
/// the correlation id → harness map `session_loop` matches answers against.
/// A send failure (the socket is already gone) just stops early — whatever
/// was sent still gets a chance to be answered before the socket is
/// discovered dead in the loop proper.
async fn send_confirm_probes<Snk>(sink: &mut Snk, harnesses: &[String]) -> std::collections::HashMap<String, String>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let mut pending = std::collections::HashMap::new();
    for harness in harnesses {
        let cid = holler_proto::CorrelationId::mint_hub();
        let params = serde_json::json!({ "feature": harness });
        let req = Envelope::request(&cid, "query/support", Some(params));
        let text = holler_proto::encode(&req).unwrap_or_default();
        if sink.send(Message::text(text)).await.is_err() || sink.flush().await.is_err() {
            break;
        }
        pending.insert(cid.as_str().to_string(), harness.clone());
    }
    pending
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
///
/// Returns the body's advertised harness ids (issue #185's `harnesses_known`
/// input, and the confirmation pass's probe list) — an empty vec if the
/// body's hello carried none or failed to parse as a [`Hello`] (a body that
/// sends a malformed `harnesses` field just gets nothing confirmed, never a
/// reason to refuse the whole handshake here).
async fn hello_exchange<Snk, St>(sink: &mut Snk, stream: &mut St, hostname: &str) -> Result<Vec<String>, ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let env = tokio::time::timeout(HELLO_TIMEOUT, next_envelope(stream))
        .await
        .map_err(|_| ())?
        .ok_or(())?;
    let Envelope::Request { id, method, params } = &env else {
        send_error(sink, env.id(), Code::Unauthenticated, "expected circuit/hello").await;
        close(sink).await;
        return Err(());
    };
    if method != "circuit/hello" {
        send_error(sink, Some(id), Code::MethodNotFound, "expected circuit/hello first").await;
        close(sink).await;
        return Err(());
    }
    let body_harnesses = params
        .clone()
        .and_then(|v| serde_json::from_value::<Hello>(v).ok())
        .and_then(|h| h.harnesses)
        .unwrap_or_default();

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
        Envelope::Response { id, .. } if id == hub_cid.as_str() => Ok(body_harnesses),
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

/// A pending hub-initiated `query/*` forward (issue #185: `hub query TARGET
/// …`) awaiting its answer — at most one at a time, same discipline as
/// [`PendingPing`].
type PendingPing = Option<(String, tokio::sync::oneshot::Sender<PingAck>)>;
type PendingQuery = Option<(String, tokio::sync::oneshot::Sender<Result<serde_json::Value, WireError>>)>;

/// The live session loop: answer presence heartbeats and `circuit/ping`
/// requests from the body, and service [`LiveCommand`]s from the registry
/// (`hub token ping`'s probe, and issue #185's `hub query TARGET …`
/// forward). Returns when the socket closes, errors, or a decode failure
/// ends the connection.
async fn session_loop<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    cmd_rx: &mut tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
    client_id: &str,
    registry: &Registry,
    mut pending_confirms: std::collections::HashMap<String, String>,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    // At most one outstanding hub-initiated ping/query at a time: this
    // connection serves exactly one `hub token ping` or `hub query` caller
    // per round trip. `pending_confirms` (issue #185) may hold several at
    // once (one per harness the body advertised) — each is independent, so
    // it stays a map rather than a single slot.
    let mut pending_ping: PendingPing = None;
    let mut pending_query: PendingQuery = None;

    loop {
        tokio::select! {
            frame = stream.next() => {
                match frame {
                    Some(Ok(Message::Text(t))) => {
                        if handle_inbound(sink, &t, client_id, &mut pending_ping, &mut pending_query, &mut pending_confirms, registry).await.is_err() {
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
                    Some(LiveCommand::Query { method, params, reply: reply_tx }) => {
                        match send_query_forward(sink, &method, params).await {
                            Ok(cid) => pending_query = Some((cid, reply_tx)),
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

/// Send a `hub query TARGET …` forward (issue #185) to the body over this
/// connection, returning its correlation id.
async fn send_query_forward<Snk>(sink: &mut Snk, method: &str, params: Option<serde_json::Value>) -> Result<String, ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let cid = holler_proto::CorrelationId::mint_hub();
    let req = Envelope::request(&cid, method, params);
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
    pending_ping: &mut PendingPing,
    pending_query: &mut PendingQuery,
    pending_confirms: &mut std::collections::HashMap<String, String>,
    registry: &Registry,
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
            handle_presence(client_id, params.clone(), registry).await;
            Ok(())
        }
        Envelope::Request { id, method, .. } if method == "circuit/ping" => {
            let ack = PingAck { hostname: client_id.to_string(), ts: now_millis() };
            reply(sink, Some(id), serde_json::to_value(ack).unwrap_or_default()).await
        }
        Envelope::Response { id, result } if pending_confirms.contains_key(id) => {
            handle_confirm_response(id, result.clone(), client_id, pending_confirms, registry).await;
            Ok(())
        }
        Envelope::Response { id, result } => {
            handle_response(id, result.clone(), pending_ping, pending_query);
            Ok(())
        }
        Envelope::Error { id, error } => {
            handle_error_response(id.as_deref(), error, pending_query);
            Ok(())
        }
        Envelope::Request { id, .. } => {
            send_error(sink, Some(id), Code::MethodNotFound, "unknown method").await;
            Ok(())
        }
        Envelope::Notification { .. } => Ok(()),
    }
}

/// A `session/presence` notification's side effect: record the body's
/// current session count (`hub status`'s `sessions`, issue #185).
async fn handle_presence(client_id: &str, params: Option<serde_json::Value>, registry: &Registry) {
    if let Some(p) = params.and_then(|v| serde_json::from_value::<Presence>(v).ok()) {
        registry.set_session_count(client_id, p.sessions.len() as u32).await;
        log(Severity::Debug, "presence", vec![("client_id", client_id.to_string()), ("hostname", p.hostname)]);
    }
}

/// A confirmation probe's answer (issue #185) — matched by id against every
/// harness's outstanding probe at once, independent of `pending_ping`/
/// `pending_query`'s single-slot discipline and of whatever else (presence, a
/// `hub query` forward) interleaves on the wire around it.
async fn handle_confirm_response(
    id: &str,
    result: Option<serde_json::Value>,
    client_id: &str,
    pending_confirms: &mut std::collections::HashMap<String, String>,
    registry: &Registry,
) {
    let Some(harness) = pending_confirms.remove(id) else {
        return;
    };
    let ok = result.and_then(|v| v.get("ok").and_then(|o| o.as_bool())).unwrap_or(false);
    if ok {
        registry.confirm_harness(client_id, &harness).await;
    }
}

/// Route a plain `Response` to whichever of `pending_ping`/`pending_query` is
/// waiting on this `id` (at most one is, by construction — each is a single
/// outstanding slot).
fn handle_response(id: &str, result: Option<serde_json::Value>, pending_ping: &mut PendingPing, pending_query: &mut PendingQuery) {
    if pending_ping.as_ref().is_some_and(|(want, _)| id == want) {
        if let Some((_, tx)) = pending_ping.take() {
            if let Some(ack) = result.and_then(|v| serde_json::from_value::<PingAck>(v).ok()) {
                let _ = tx.send(ack);
            }
        }
    } else if pending_query.as_ref().is_some_and(|(want, _)| id == want) {
        if let Some((_, tx)) = pending_query.take() {
            let _ = tx.send(Ok(result.unwrap_or(serde_json::Value::Null)));
        }
    }
}

/// Route an `Error` envelope to `pending_query` when it matches (issue #185:
/// a body's `query/*` error, e.g. `query/support`'s `-32006`, forwarded
/// verbatim to the `hub query` caller). `circuit/ping`/confirmation probes
/// never error on the wire (a body always answers `query/support` with a
/// result, `ok:false` included), so only `pending_query` is checked here.
fn handle_error_response(id: Option<&str>, error: &WireError, pending_query: &mut PendingQuery) {
    let Some(want_id) = id else { return };
    if pending_query.as_ref().is_some_and(|(want, _)| want_id == want) {
        if let Some((_, tx)) = pending_query.take() {
            let _ = tx.send(Err(error.clone()));
        }
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
