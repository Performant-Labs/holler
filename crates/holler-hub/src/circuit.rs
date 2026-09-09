//! The hub side of a **re-authenticated** circuit (issue #182): the
//! `circuit/authenticate` → `circuit/hello` handshake, then the live session
//! loop that answers presence notifications and lets the control socket
//! (`hub token ping`) reach the body over this exact connection.
//!
//! `circuit/join` (story #176, [`crate::join`]) is the one-shot bootstrap and
//! stays completely separate: it never leads into talk on the same socket.
//! This module is what a **returning** body's socket runs through instead.
//!
//! Issue #230 folded the session loop's connection-scoped mutable state (the
//! sink/stream, client id, registry, roster, and the four pending-request
//! trackers) into [`SessionConnection`], turning `session_loop`/`handle_inbound`
//! into methods on it instead of functions threading 9 loose positional
//! parameters (which is what forced their
//! clippy "too many arguments" `allow` escapes) — a pure refactor, no
//! behavior change on its own.
//!
//! Issue #243 is a real gap this module closes: unlike the body's own
//! `live_loop` (`holler-body`'s `connection.rs`), which has always raced a
//! `last_frame_at`/heartbeat-interval liveness timeout in its main
//! `tokio::select!`, this hub-side loop used to have **none** — it only
//! reacted to `stream.next()` or `cmd_rx.recv()`. A body that goes
//! unreachable without a clean TCP close (sleep/suspend, a silent network
//! partition) used to park this task indefinitely, and any `say` routed to
//! that session would hang forever with no `connection_lost` error.
//! [`SessionConnection`] now tracks its own `last_frame_at` (refreshed on
//! every inbound WS frame, not just a decoded one — the same discipline as
//! the body side) and races it against [`liveness_timeout`] in the same
//! `select!` that already watches the socket and the command channel; on
//! expiry the connection is torn down exactly like every other teardown path
//! here (`fail_pending_says` + clearing the roster row immediately).

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::{
    log::{Component, Direction as LogDirection, Event, Severity},
    Authenticate, Code, Envelope, Hello, HelloRole, PingAck, Presence, PromptResult, Update, WireError,
};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::{LiveCommand, Registry, SayReply, SeenUpdate};
use crate::serve::{close, send_error};
use crate::state::HubState;

/// One `say` still awaiting its `session/prompt` response on this
/// connection (issue #190). Keyed by `request_id` in a map (not the single
/// `Option` `pending_ping` uses) — unlike a probe ping, `say`s to different
/// (or even the same, via `--queue`) sessions are routinely concurrent on
/// one connection: the hub forwards a `--queue`d `session/prompt` to the
/// body immediately, without itself waiting for the turn ahead of it to
/// finish, so two (or more) requests can be in flight on this socket at
/// once. A single `Option` here would have the second `say` silently drop
/// the first's reply channel (`Sender` overwritten and dropped) the moment
/// it arrived — confirmed the hard way while building this story's own
/// `say_queue_appends_and_runs_after_turn` test, which failed with a
/// spurious "no reply … within 600s" the instant the queued call's request
/// went out, not after any real 600s wait.
struct PendingSay {
    reply: tokio::sync::oneshot::Sender<SayReply>,
    updates: Vec<SeenUpdate>,
}

/// How long the hub waits for the body's half of the hello exchange, and for
/// the body's answer to the hub's own hello, before giving up on the socket.
const HELLO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// The heartbeat/presence interval a body is expected to keep to (mirrors
/// `holler-body`'s own `heartbeat_interval()`): 15s, or
/// `HOLLER_HEARTBEAT_INTERVAL_MS` for tests that cannot wait 15s for real.
fn heartbeat_interval() -> std::time::Duration {
    std::env::var("HOLLER_HEARTBEAT_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(std::time::Duration::from_millis)
        .unwrap_or(std::time::Duration::from_secs(15))
}

/// How long with no frame of any kind from the body before this connection is
/// treated as dead (issue #243) — 3 × the heartbeat interval by default,
/// matching the body's own `live_loop` liveness bound and the roster's
/// `reconnect_secs` default (45s), so a body that has actually gone silent
/// (not just mid-reconnect) is torn down at the same horizon the roster
/// display already assumes. Independently overridable via
/// `HOLLER_HUB_LIVENESS_TIMEOUT_MS` (tests cannot wait 45s for real).
fn liveness_timeout() -> std::time::Duration {
    std::env::var("HOLLER_HUB_LIVENESS_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(std::time::Duration::from_millis)
        .unwrap_or(heartbeat_interval() * 3)
}

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
    roster: &std::sync::Arc<crate::roster::Roster>,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let record = match crate::token::verify_credential_async(&params.token_id, &params.credential, state).await {
        Ok(r) => r,
        Err(e) => {
            send_error(sink, id, Code::Unauthenticated, &format!("authentication failed: {e}")).await;
            close(sink).await;
            // The credentials are no longer valid, so the body is permanently
            // gone — drop it from the roster outright (it can never re-auth
            // with the same token, so the TTL would only keep a stale row
            // around). Issue #186.
            roster.clear(&params.token_id);
            return;
        }
    };
    let Some(client_id) = record.client_id.clone() else {
        // Defensive: `verify_credential` only returns `Bound` records, which
        // always carry a `client_id`. Treated the same as a bad credential.
        send_error(sink, id, Code::Unauthenticated, "authentication failed: no client id on record").await;
        close(sink).await;
        roster.clear(&params.token_id);
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
    // Issue #236 (ADR 0005 §2): the label travels with the authenticated
    // token, never on the wire, so this is the one place the hub can read it
    // — `record` is the verified token record from just above. Binding it
    // (and the client id) here, before any `session/presence` arrives, means
    // every roster row this body advertises is qualified `<label>/<session>`
    // from its very first row, not just from the second presence beat.
    roster.set_token(&params.token_id, &client_id);
    roster.set_label(&params.token_id, &record.label);

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

    let mut conn =
        SessionConnection::new(sink, stream, &mut cmd_rx, &client_id, registry, roster, pending_confirms).await;
    conn.run().await;
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

/// A pending hub-initiated `circuit/ping`/`query/*` forward — at most one at
/// a time, same discipline as before #190 (unlike `say`, neither is ever
/// concurrent on one connection today).
type PendingPing = Option<(String, tokio::sync::oneshot::Sender<PingAck>)>;
type PendingQuery = Option<(String, tokio::sync::oneshot::Sender<Result<serde_json::Value, WireError>>)>;

/// Issue #230: the session loop's connection-scoped mutable state, bundled
/// into one owned struct instead of threaded as 9 loose positional
/// parameters across `session_loop`/`handle_inbound` (which is what forced
/// their clippy "too many arguments" `allow` escapes). `sink`/`stream` are
/// the socket halves; `client_id`/`registry`/`roster` are this connection's
/// read-mostly identity and shared state; `pending_ping`/`pending_query`/
/// `pending_confirms`/`pending_says` are the four independent
/// pending-request trackers `handle_inbound` resolves against; `roster_token`
/// is captured once (issue #186: the roster row is keyed by *token*, and
/// `handle_authenticated` removes this client from the registry the moment
/// [`Self::run`] returns, so the token must be captured while the registry
/// entry is still live) so teardown can clear the roster row unconditionally.
/// `last_frame_at` (issue #243) is this connection's own liveness clock,
/// refreshed on every inbound WS frame — mirroring the body-side
/// `LiveConnection`'s own field — and raced against [`liveness_timeout`] in
/// [`Self::run`]'s `select!` so a body that goes silent without closing the
/// socket is torn down instead of parking this task forever.
struct SessionConnection<'a, Snk, St> {
    sink: &'a mut Snk,
    stream: &'a mut St,
    cmd_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
    client_id: &'a str,
    registry: &'a Registry,
    roster: &'a std::sync::Arc<crate::roster::Roster>,
    roster_token: Option<String>,
    pending_ping: PendingPing,
    pending_query: PendingQuery,
    pending_confirms: std::collections::HashMap<String, String>,
    pending_says: std::collections::HashMap<String, PendingSay>,
    last_frame_at: tokio::time::Instant,
}

impl<'a, Snk, St> SessionConnection<'a, Snk, St>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    async fn new(
        sink: &'a mut Snk,
        stream: &'a mut St,
        cmd_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
        client_id: &'a str,
        registry: &'a Registry,
        roster: &'a std::sync::Arc<crate::roster::Roster>,
        pending_confirms: std::collections::HashMap<String, String>,
    ) -> Self {
        let roster_token = registry.token_id_for_client(client_id).await;
        Self {
            sink,
            stream,
            cmd_rx,
            client_id,
            registry,
            roster,
            roster_token,
            pending_ping: None,
            pending_query: None,
            pending_confirms,
            pending_says: std::collections::HashMap::new(),
            last_frame_at: tokio::time::Instant::now(),
        }
    }

    /// Issue #186: the roster row is keyed by *token*, so an explicit close
    /// is `gone` immediately, not "reconnecting" — used on every teardown
    /// path below, including the issue #243 liveness timeout.
    fn clear_from_roster(&self) {
        if let Some(token) = &self.roster_token {
            self.roster.clear(token);
        }
    }

    /// The live session loop: answer presence heartbeats and `circuit/ping`
    /// requests from the body, service [`LiveCommand`]s from the registry
    /// (`hub token ping`'s probe, issue #185's `hub query TARGET …` forward,
    /// and issue #190's `say`), and (issue #243) tear the connection down if
    /// no frame of any kind arrives from the body within [`liveness_timeout`]
    /// — the same liveness discipline `holler-body`'s own `live_loop` has
    /// always applied in the other direction. Returns when the socket
    /// closes, errors, decodes fail, or the liveness timeout fires.
    async fn run(&mut self) {
        loop {
            tokio::select! {
                frame = self.stream.next() => {
                    match frame {
                        Some(Ok(Message::Text(t))) => {
                            self.last_frame_at = tokio::time::Instant::now();
                            if self.handle_inbound(&t).await.is_err() {
                                self.fail_pending_says();
                                self.clear_from_roster();
                                return;
                            }
                        }
                        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {
                            self.last_frame_at = tokio::time::Instant::now();
                        }
                        Some(Ok(Message::Binary(_))) | Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                            self.fail_pending_says();
                            self.clear_from_roster();
                            return;
                        }
                    }
                }
                _ = tokio::time::sleep_until(self.last_frame_at + liveness_timeout()) => {
                    log(
                        Severity::Warn,
                        "conn_liveness_expired",
                        vec![("client_id", self.client_id.to_string())],
                    );
                    self.fail_pending_says();
                    self.clear_from_roster();
                    return;
                }
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(LiveCommand::Ping { reply: reply_tx }) => {
                            match send_ping_probe(self.sink).await {
                                Ok(cid) => self.pending_ping = Some((cid, reply_tx)),
                                Err(()) => return,
                            }
                        }
                        Some(LiveCommand::Query { method, params, reply: reply_tx }) => {
                            match send_query_forward(self.sink, &method, params).await {
                                Ok(cid) => self.pending_query = Some((cid, reply_tx)),
                                Err(()) => return,
                            }
                        }
                        Some(LiveCommand::Say { request_id, session, message, queue, reply }) => {
                            match send_prompt(self.sink, &request_id, &session, message, queue).await {
                                Ok(()) => {
                                    self.pending_says.insert(request_id, PendingSay { reply, updates: Vec::new() });
                                }
                                Err(()) => {
                                    let _ = reply.send(SayReply::ConnectionLost);
                                    return;
                                }
                            }
                        }
                        None => return, // the registry entry was dropped/replaced.
                    }
                }
            }
        }
    }

    /// The socket ended (or the liveness timeout fired) while one or more
    /// `say`s were still in flight: report every one of them as
    /// `connection_lost`, never a silent drop (the spec's own wording: "body
    /// io disconnected mid-turn; ask again" — never "hub unreachable").
    fn fail_pending_says(&mut self) {
        for (_, p) in self.pending_says.drain() {
            let _ = p.reply.send(SayReply::ConnectionLost);
        }
    }

    /// Handle one inbound text frame in the live session loop. `Err` means
    /// the socket should be torn down (a decode failure or a send failure).
    async fn handle_inbound(&mut self, text: &str) -> Result<(), ()> {
        let env = match holler_proto::decode(text) {
            Ok(e) => e,
            Err(e) => {
                send_error(self.sink, None, e.code(), &e.to_string()).await;
                return Err(());
            }
        };
        // Every inbound frame on an authenticated socket is proof the body is
        // still talking to us (issue #186), so refresh the roster row's
        // `last_heard_ms` on whatever the frame is. The roster is keyed by
        // token, not client id — resolve the token through the registry (a
        // body that has already been unregistered mid-frame just has no
        // token to attribute, and the loop's teardown `clear_from_roster`
        // then finalises it). `None` is a defensive no-op: the frame is still
        // serviced normally below.
        if let Some(token_id) = self.registry.token_id_for_client(self.client_id).await {
            self.roster.touch(&token_id, env.method().unwrap_or(""));
        }
        match &env {
            Envelope::Notification { method, params } if method == "session/presence" => {
                handle_presence_notification(self.client_id, params.clone(), self.registry, self.roster).await;
                Ok(())
            }
            Envelope::Notification { method, params } if method == "session/update" => {
                handle_update_notification(params.clone(), &mut self.pending_says);
                Ok(())
            }
            Envelope::Request { id, method, .. } if method == "circuit/ping" => {
                let ack = PingAck { hostname: self.client_id.to_string(), ts: now_millis() };
                reply(self.sink, Some(id), serde_json::to_value(ack).unwrap_or_default()).await
            }
            Envelope::Response { id, result } if self.pending_confirms.contains_key(id) => {
                handle_confirm_response(id, result.clone(), self.client_id, &mut self.pending_confirms, self.registry)
                    .await;
                Ok(())
            }
            Envelope::Response { id, result } => {
                handle_response(id, result.clone(), &mut self.pending_ping, &mut self.pending_query, &mut self.pending_says);
                Ok(())
            }
            Envelope::Error { id, error } => {
                handle_error_response(id.as_deref(), error, &mut self.pending_query, &mut self.pending_says);
                Ok(())
            }
            Envelope::Request { id, .. } => {
                send_error(self.sink, Some(id), Code::MethodNotFound, "unknown method").await;
                Ok(())
            }
            Envelope::Notification { .. } => Ok(()),
        }
    }
}

/// Send a `session/prompt {session, message, queue}` request to the body
/// under `request_id`.
async fn send_prompt<Snk>(
    sink: &mut Snk,
    request_id: &str,
    session: &str,
    message: Box<holler_proto::Message>,
    queue: bool,
) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let cid = holler_proto::CorrelationId::parse(request_id).map_err(|_| ())?;
    let params = holler_proto::Prompt { session: session.to_string(), message: *message, meta: None, queue };
    let req = Envelope::request(&cid, "session/prompt", Some(serde_json::to_value(params).map_err(|_| ())?));
    let text = holler_proto::encode(&req).map_err(|_| ())?;
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
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

/// A `session/presence` notification: record the body's current session
/// count (`hub status`'s `sessions`, issue #185) and cache its live session
/// state (issue #190's roster stand-in — see the `live` module doc) so
/// `say`'s busy check and name resolution have something real to read. Split
/// out of [`SessionConnection::handle_inbound`] to keep that dispatch's
/// cognitive complexity under the workspace threshold.
async fn handle_presence_notification(
    client_id: &str,
    params: Option<serde_json::Value>,
    registry: &Registry,
    roster: &std::sync::Arc<crate::roster::Roster>,
) {
    let Some(p) = params.and_then(|v| serde_json::from_value::<Presence>(v).ok()) else { return };
    // Clone `p.hostname` into the log line (issue #186): `roster.advertise`
    // below borrows the whole `Presence`, so the log must not move a field out.
    log(Severity::Debug, "presence", vec![("client_id", client_id.to_string()), ("hostname", p.hostname.clone())]);
    // Advertise the roster row *before* `registry.update_presence` moves
    // `p.sessions` out (issue #186). The roster row is keyed by token, so
    // resolve the client id to its token first. A body that lost its registry
    // entry (a duplicate that was just replaced) has no token to attribute to;
    // the roster row for the token the *new* socket owns is what `rows()`
    // returns, and this stale socket's own teardown clears its (old) token.
    if let Some(token_id) = registry.token_id_for_client(client_id).await {
        roster.advertise(&token_id, &p);
    }
    registry.set_session_count(client_id, p.sessions.len() as u32).await;
    registry.update_presence(client_id, p.sessions).await;
}

/// A `session/update` notification: append it to the matching in-flight
/// `say`'s own record (by `prompt_id`), if any is still pending. Split out
/// of [`SessionConnection::handle_inbound`] for the same reason as
/// [`handle_presence_notification`].
fn handle_update_notification(
    params: Option<serde_json::Value>,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
) {
    let Some(p) = params.and_then(|v| serde_json::from_value::<Update>(v).ok()) else { return };
    if let Some(pending) = pending_says.get_mut(&p.prompt_id) {
        pending.updates.push(SeenUpdate { ts: holler_proto::log::timestamp(), seq: p.seq, parts: p.parts });
    }
}

/// A confirmation probe's answer (issue #185) — matched by id against every
/// harness's outstanding probe at once, independent of `pending_ping`/
/// `pending_query`'s single-slot discipline and of whatever else (presence, a
/// `hub query` forward, a `say`) interleaves on the wire around it.
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

/// Route a plain `Response` to whichever of `pending_says`/`pending_ping`/
/// `pending_query` is waiting on this `id` (at most one is, by construction —
/// `pending_says` keys by `id` directly; the other two are each a single
/// outstanding slot). Split out of [`SessionConnection::handle_inbound`] for
/// the same reason as [`handle_presence_notification`].
fn handle_response(
    id: &str,
    result: Option<serde_json::Value>,
    pending_ping: &mut PendingPing,
    pending_query: &mut PendingQuery,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
) {
    if let Some(pending) = pending_says.remove(id) {
        let outcome = match result.and_then(|v| serde_json::from_value::<PromptResult>(v).ok()) {
            Some(r) => SayReply::Result {
                message: Box::new(r.message),
                stop_reason: r.stop_reason,
                state: r.state,
                updates: pending.updates,
            },
            None => SayReply::ConnectionLost,
        };
        let _ = pending.reply.send(outcome);
        return;
    }
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

/// Route an `Error` envelope to whichever of `pending_says`/`pending_query`
/// matches (issue #190's own body-refused `say`, or issue #185's a body's
/// `query/*` error — e.g. `query/support`'s `-32006` — forwarded verbatim to
/// the `hub query` caller). `circuit/ping`/confirmation probes never error on
/// the wire (a body always answers with a result, `ok:false` included).
fn handle_error_response(
    id: Option<&str>,
    error: &WireError,
    pending_query: &mut PendingQuery,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
) {
    let Some(want_id) = id else { return };
    if let Some(pending) = pending_says.remove(want_id) {
        let _ = pending.reply.send(SayReply::Refused(error.clone()));
        return;
    }
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
