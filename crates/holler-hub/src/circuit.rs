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
//! expiry the connection is torn down exactly like every other *abrupt*
//! teardown path here (`fail_pending` + marking the roster's rows
//! `reconnecting` — issue #192 revised this from an immediate `gone`: the
//! body is expected to reconnect, and only ages all the way out to `gone` if
//! it does not, on the same TTL the roster's own sweep already uses).
//!
//! Issue #192 pins the reconnect contract end to end: every abrupt ending
//! (a decode/send failure, a socket error, EOF, this liveness timeout, or the
//! `control/test_drop` test hook) marks the token's rows `reconnecting`
//! rather than `gone` ([`SessionConnection::mark_reconnecting`]); only an
//! explicit WS close frame (`body detach`'s own clean teardown) still goes
//! straight to `gone` ([`SessionConnection::clear_from_roster`],
//! holler-server#80). Every pending `say`/`cancel` is still failed
//! immediately either way (`fail_pending`) — the operator never waits out a
//! timeout to learn the body dropped mid-turn. See `docs/protocol/v2.md`'s
//! "Reconnect contract" section for the full six-point contract this
//! implements.

mod dispatch;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::{
    log::{Component, Direction as LogDirection, Event, Severity},
    Authenticate, Code, Envelope, Hello, HelloRole, PingAck, WireError,
};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::{CancelCommand, CancelReply, LiveCommand, Registry, SayReply};
use crate::serve::{close, send_error};
use crate::state::HubState;
use dispatch::PendingSay;

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

    let (mut cmd_rx, mut cancel_rx) = registry.insert(&client_id, &params.hostname, &params.token_id).await;
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
        SessionConnection::new(
            sink,
            stream,
            CommandChannels { cmd_rx: &mut cmd_rx, cancel_rx: &mut cancel_rx },
            &client_id,
            registry,
            roster,
            pending_confirms,
        )
        .await;
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
    /// The priority channel `LiveHandle::cancel` sends on (issue #191) — see
    /// [`CancelCommand`]'s own doc for why this is drained separately from
    /// (and with priority over) `cmd_rx`.
    cancel_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<CancelCommand>,
    client_id: &'a str,
    registry: &'a Registry,
    roster: &'a std::sync::Arc<crate::roster::Roster>,
    roster_token: Option<String>,
    pending_ping: PendingPing,
    pending_query: PendingQuery,
    pending_confirms: std::collections::HashMap<String, String>,
    pending_says: std::collections::HashMap<String, PendingSay>,
    /// One `session/cancel` still awaiting its `{applied:true}` response on
    /// this connection (issue #191), keyed by `request_id` — same "map, not
    /// a single `Option`" discipline as `pending_says` (issue #202's
    /// regression: a sibling's own `say` and this cancel can be concurrent).
    pending_cancels: std::collections::HashMap<String, tokio::sync::oneshot::Sender<CancelReply>>,
    last_frame_at: tokio::time::Instant,
}

/// The two command channels a live connection's task drains (issue #191):
/// bundled together purely to keep [`SessionConnection::new`] under clippy's
/// too-many-arguments gate — `cmd_rx` (the normal `LiveCommand`s) and
/// `cancel_rx` (the priority `CancelCommand`s) are otherwise unrelated to
/// each other.
struct CommandChannels<'a> {
    cmd_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
    cancel_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<CancelCommand>,
}

impl<'a, Snk, St> SessionConnection<'a, Snk, St>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    async fn new(
        sink: &'a mut Snk,
        stream: &'a mut St,
        channels: CommandChannels<'a>,
        client_id: &'a str,
        registry: &'a Registry,
        roster: &'a std::sync::Arc<crate::roster::Roster>,
        pending_confirms: std::collections::HashMap<String, String>,
    ) -> Self {
        let roster_token = registry.token_id_for_client(client_id).await;
        Self {
            sink,
            stream,
            cmd_rx: channels.cmd_rx,
            cancel_rx: channels.cancel_rx,
            client_id,
            registry,
            roster,
            roster_token,
            pending_ping: None,
            pending_query: None,
            pending_confirms,
            pending_says: std::collections::HashMap::new(),
            pending_cancels: std::collections::HashMap::new(),
            last_frame_at: tokio::time::Instant::now(),
        }
    }

    /// Issue #186: the roster row is keyed by *token*, so an explicit close
    /// (a WS close frame — `body detach`'s own clean teardown) is `gone`
    /// immediately, never "reconnecting" — the body told us on its way out
    /// that it is not coming back.
    fn clear_from_roster(&self) {
        if let Some(token) = &self.roster_token {
            self.roster.clear(token);
        }
    }

    /// Issue #192: every **abrupt** teardown path below (a decode/send
    /// failure, a socket error, EOF, the issue #243 liveness timeout, or the
    /// `control/test_drop` test hook) marks the token's rows `reconnecting`
    /// instead of `clear`-ing them straight to `gone` — the body is expected
    /// to reconnect (see [`crate::roster::Roster::mark_reconnecting`]'s own
    /// doc for why this is not the same as an explicit close).
    fn mark_reconnecting(&self) {
        if let Some(token) = &self.roster_token {
            self.roster.mark_reconnecting(token);
        }
    }

    /// Handle one [`CancelCommand`] received from the priority channel (issue
    /// #191): send `session/cancel` and register the pending reply, or (a
    /// send failure, or the registry entry itself being gone) report
    /// `Err(())` so the caller ([`Self::run`]) tears the connection down —
    /// split out purely to keep that `select!`'s own cognitive complexity
    /// under the workspace threshold.
    async fn handle_cancel_command(&mut self, cancel_cmd: Option<CancelCommand>) -> Result<(), ()> {
        let CancelCommand { request_id, session, reply } = cancel_cmd.ok_or(())?;
        match dispatch::send_cancel(self.sink, &request_id, &session).await {
            Ok(()) => {
                self.pending_cancels.insert(request_id, reply);
                Ok(())
            }
            Err(()) => {
                let _ = reply.send(CancelReply::ConnectionLost);
                Err(())
            }
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
                // `biased` (issue #191): `cancel_rx` is listed ahead of
                // `cmd_rx` so a queued `session/cancel` always wins the tie
                // when both channels have a command ready — the hub-side
                // half of the priority path (`LiveHandle::cancel` sends here
                // instead of the normal `tx`/`cmd_rx`, which can carry a
                // queued `say`/`query`/`ping` ahead of it).
                biased;
                frame = self.stream.next() => {
                    match frame {
                        Some(Ok(Message::Text(t))) => {
                            self.last_frame_at = tokio::time::Instant::now();
                            if self.handle_inbound(&t).await.is_err() {
                                self.fail_pending();
                                self.mark_reconnecting();
                                return;
                            }
                        }
                        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {
                            self.last_frame_at = tokio::time::Instant::now();
                        }
                        // A clean WS close frame (`body detach`'s own
                        // teardown, issue #182 step 6): the body told us on
                        // its way out — `gone` immediately (holler-server#80).
                        Some(Ok(Message::Close(_))) => {
                            self.fail_pending();
                            self.clear_from_roster();
                            return;
                        }
                        // Every other ending is *abrupt* (issue #192): no
                        // clean close handshake, so the body is expected to
                        // reconnect on its own backoff — `reconnecting`, not
                        // `gone`.
                        Some(Ok(Message::Binary(_))) | Some(Err(_)) | None => {
                            self.fail_pending();
                            self.mark_reconnecting();
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
                    self.fail_pending();
                    self.mark_reconnecting();
                    return;
                }
                cancel_cmd = self.cancel_rx.recv() => {
                    if self.handle_cancel_command(cancel_cmd).await.is_err() {
                        return;
                    }
                }
                cmd = self.cmd_rx.recv() => {
                    if self.handle_live_command(cmd).await.is_err() {
                        return;
                    }
                }
            }
        }
    }

    /// Handle one [`LiveCommand`] received from the normal channel: `Err(())`
    /// (a send failure, or the registry entry itself being gone) tells
    /// [`Self::run`] to tear the connection down. Split out for the same
    /// reason as [`Self::handle_cancel_command`] — keeping that `select!`'s
    /// own cognitive complexity under the workspace threshold.
    async fn handle_live_command(&mut self, cmd: Option<LiveCommand>) -> Result<(), ()> {
        match cmd.ok_or(())? {
            LiveCommand::Ping { reply: reply_tx } => match dispatch::send_ping_probe(self.sink).await {
                Ok(cid) => {
                    self.pending_ping = Some((cid, reply_tx));
                    Ok(())
                }
                Err(()) => Err(()),
            },
            LiveCommand::Query { method, params, reply: reply_tx } => {
                match dispatch::send_query_forward(self.sink, &method, params).await {
                    Ok(cid) => {
                        self.pending_query = Some((cid, reply_tx));
                        Ok(())
                    }
                    Err(()) => Err(()),
                }
            }
            LiveCommand::Say { request_id, session, message, queue, replace, reply } => {
                match dispatch::send_prompt(self.sink, &request_id, &session, message, queue, replace).await {
                    Ok(()) => {
                        self.pending_says.insert(request_id, PendingSay { reply, updates: Vec::new() });
                        Ok(())
                    }
                    Err(()) => {
                        let _ = reply.send(SayReply::ConnectionLost);
                        Err(())
                    }
                }
            }
            // Issue #192's `control/test_drop` test hook: fail every
            // pending say/cancel with `connection_lost`, mark the token's
            // rows `reconnecting` (an abrupt drop, not an explicit close),
            // ack the caller, then tear this connection down — `run`'s own
            // `Err(())` handling for this arm does *not* re-run cleanup, so
            // it happens here, once.
            LiveCommand::Drop { reply } => {
                self.fail_pending();
                self.mark_reconnecting();
                let _ = reply.send(());
                Err(())
            }
        }
    }

    /// The socket ended (or the liveness timeout fired) while one or more
    /// `say`s/cancels were still in flight: report every one of them as
    /// `connection_lost`, never a silent drop (the spec's own wording: "body
    /// io disconnected mid-turn; ask again" — never "hub unreachable").
    fn fail_pending(&mut self) {
        for (_, p) in self.pending_says.drain() {
            let _ = p.reply.send(SayReply::ConnectionLost);
        }
        for (_, reply) in self.pending_cancels.drain() {
            let _ = reply.send(CancelReply::ConnectionLost);
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
                dispatch::handle_presence_notification(self.client_id, params.clone(), self.registry, self.roster).await;
                Ok(())
            }
            Envelope::Notification { method, params } if method == "session/update" => {
                dispatch::handle_update_notification(params.clone(), &mut self.pending_says);
                Ok(())
            }
            Envelope::Request { id, method, .. } if method == "circuit/ping" => {
                let ack = PingAck { hostname: self.client_id.to_string(), ts: now_millis() };
                reply(self.sink, Some(id), serde_json::to_value(ack).unwrap_or_default()).await
            }
            Envelope::Response { id, result } if self.pending_confirms.contains_key(id) => {
                dispatch::handle_confirm_response(id, result.clone(), self.client_id, &mut self.pending_confirms, self.registry)
                    .await;
                Ok(())
            }
            Envelope::Response { id, result } => {
                dispatch::handle_response(
                    id,
                    result.clone(),
                    &mut self.pending_ping,
                    &mut self.pending_query,
                    &mut self.pending_says,
                    &mut self.pending_cancels,
                );
                Ok(())
            }
            Envelope::Error { id, error } => {
                dispatch::handle_error_response(id.as_deref(), error, &mut self.pending_query, &mut self.pending_says, &mut self.pending_cancels);
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
