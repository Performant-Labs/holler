//! `SessionConnection`'s own outbound-send helpers and inbound-frame
//! dispatch routines (issue #192): split out of `circuit.rs` proper once
//! this file's own growth (the reconnect contract's `mark_reconnecting`/
//! `Drop`-command/`orphan_update` additions) pushed it past the workspace's
//! 900-line build guard (`scripts/lint.sh` check 4) — a pure relocation, no
//! behavior change of its own. Mirrors `holler-body`'s own
//! `connection/session_dispatch.rs` split for the same reason.
//!
//! [`PendingSay`] lives here (not `circuit.rs`) because every function that
//! reads or resolves it — [`handle_update_notification`], [`handle_response`],
//! [`handle_error_response`] — moved here too; `circuit.rs`'s
//! `SessionConnection` still owns the `pending_says` map itself and
//! constructs/drains `PendingSay` values directly, so its fields stay
//! `pub(super)` rather than fully private.

use futures_util::{Sink, SinkExt};
use holler_proto::{
    log::Severity, Envelope, PingAck, Presence, PromptResult, Update, WireError,
};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::{CancelReply, Registry, SayReply, SeenUpdate};

use crate::state::HubState;

use super::{PendingPing, PendingQuery};

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
pub(super) struct PendingSay {
    pub(super) reply: tokio::sync::oneshot::Sender<SayReply>,
    pub(super) updates: Vec<SeenUpdate>,
}

/// Why [`send_prompt`] did not send.
pub(super) enum SendPromptError {
    /// The session hold refused the prompt (issue #442, #460): nothing was
    /// sent. Carries the wire error to answer with (`session_held`, or
    /// `invalid_grant` for a grant that was not honoured).
    Refused(holler_proto::WireError),
    /// The frame could not be built or the socket is gone.
    Io,
}

/// The hold registry and the key [`send_prompt`] checks it with
/// (`<label>/<session>`, built from the connection's own authenticated label).
#[derive(Clone, Copy)]
pub(super) struct HoldGate<'a> {
    pub(super) holds: &'a crate::holds::Holds,
    pub(super) key: &'a str,
    /// The one-time release grant the sender presented, if any (issue #460).
    pub(super) grant: Option<&'a str>,
}

/// Send a `session/prompt {session, message, queue, replace}` request to the
/// body under `request_id`. `replace` (issue #191) is set only by
/// `interrupt SESSION TEXT`, after the matching cancel's own `{applied:true}`
/// — see `crate::interrupt`.
///
/// **This is the hub's one enforcement point for the session hold** (issue
/// #442). It is the only function in the hub that puts a `session/prompt` on
/// a socket, so checking here, before anything is built or sent, covers
/// `say`, `say --queue` and `say --replace` alike; a hold checked anywhere
/// else would be a bypass. `gate.key` is `<label>/<session>` built by the
/// caller from the connection's own authenticated label and the session name
/// it is about to send. The check is one synchronous map lookup with no
/// `await`, so it is totally ordered against `hold`/`release` (see
/// `crate::holds`). `crates/holler-cli/tests/hold_single_path_test.rs` fails
/// if a second sender of `session/prompt` appears.
pub(super) async fn send_prompt<Snk>(
    sink: &mut Snk,
    gate: HoldGate<'_>,
    request_id: &str,
    session: &str,
    message: Box<holler_proto::Message>,
    queue: bool,
    replace: bool,
) -> Result<(), SendPromptError>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    // `admit` decides and, for a valid grant, consumes it in one critical
    // section: the prompt is accepted here, so the grant is spent here.
    gate.holds.admit(gate.key, gate.grant).map_err(SendPromptError::Refused)?;
    let cid = holler_proto::CorrelationId::parse(request_id).map_err(|_| SendPromptError::Io)?;
    let params = holler_proto::Prompt { session: session.to_string(), message: *message, meta: None, queue, replace };
    let req = Envelope::request(&cid, "session/prompt", Some(serde_json::to_value(params).map_err(|_| SendPromptError::Io)?));
    let text = holler_proto::encode(&req).map_err(|_| SendPromptError::Io)?;
    super::log_frame(super::LogDirection::Out, "session/prompt", Some(request_id), &text);
    sink.send(Message::text(text)).await.map_err(|_| SendPromptError::Io)?;
    sink.flush().await.map_err(|_| SendPromptError::Io)
}

/// Send a `session/cancel {session}` request to the body under `request_id`
/// (issue #191) — the priority-path counterpart of [`send_prompt`].
pub(super) async fn send_cancel<Snk>(sink: &mut Snk, request_id: &str, session: &str) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let cid = holler_proto::CorrelationId::parse(request_id).map_err(|_| ())?;
    let params = holler_proto::Cancel { session: session.to_string() };
    let req = Envelope::request(&cid, "session/cancel", Some(serde_json::to_value(params).map_err(|_| ())?));
    let text = holler_proto::encode(&req).map_err(|_| ())?;
    super::log_frame(super::LogDirection::Out, "session/cancel", Some(request_id), &text);
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
}

/// Send a `circuit/ping` request to the body, returning its correlation id so
/// the caller can recognise the matching reply.
pub(super) async fn send_ping_probe<Snk>(sink: &mut Snk) -> Result<String, ()>
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
pub(super) async fn send_query_forward<Snk>(sink: &mut Snk, method: &str, params: Option<serde_json::Value>) -> Result<String, ()>
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
/// out of [`super::SessionConnection::handle_inbound`] to keep that
/// dispatch's cognitive complexity under the workspace threshold.
pub(super) async fn handle_presence_notification(
    client_id: &str,
    params: Option<serde_json::Value>,
    label: &str,
    registry: &Registry,
    roster: &std::sync::Arc<crate::roster::Roster>,
    last_seen: &mut LastSeenFlusher,
) {
    let Some(p) = params.and_then(|v| serde_json::from_value::<Presence>(v).ok()) else { return };
    // Issue #460: a session seen for the first time joins held when the hub
    // was started with `--join-held`. Done before the session becomes
    // resolvable (`update_presence` below), so there is no moment at which it
    // can be sent to but is not yet held.
    registry.holds().note_joined(p.sessions.iter().map(|s| crate::holds::session_key(label, &s.name)));
    // Clone `p.hostname` into the log line (issue #186): `roster.advertise`
    // below borrows the whole `Presence`, so the log must not move a field out.
    super::log(Severity::Debug, "presence", vec![("client_id", client_id.to_string()), ("hostname", p.hostname.clone())]);
    // Advertise the roster row *before* `registry.update_presence` moves
    // `p.sessions` out (issue #186). The roster row is keyed by token, so
    // resolve the client id to its token first. A body that lost its registry
    // entry (a duplicate that was just replaced) has no token to attribute to;
    // the roster row for the token the *new* socket owns is what `rows()`
    // returns, and this stale socket's own teardown clears its (old) token.
    if let Some(token_id) = registry.token_id_for_client(client_id).await {
        roster.advertise(&token_id, &p);
        last_seen.beat(&token_id);
    }
    registry.set_session_count(client_id, p.sessions.len() as u32).await;
    registry.update_presence(client_id, p.sessions).await;
}

/// A `session/update` notification: append it to the matching in-flight
/// `say`'s own record (by `prompt_id`), if any is still pending. Split out
/// of [`super::SessionConnection::handle_inbound`] for the same reason as
/// [`handle_presence_notification`].
///
/// Issue #192 rule 6: a `session/update` for a `prompt_id` this connection no
/// longer has pending — the hub already failed that `say` with
/// `connection_lost` on the earlier drop, and this is a stray update that
/// outraced the reconnect (or, architecturally, never can under the body's
/// own per-connection outbound channel — see `docs/protocol/v2.md`'s
/// "Reconnect contract" §6) — is silently ignored, exactly as it already was
/// before this issue (a `HashMap::get_mut` miss is a no-op); the only change
/// here is logging it at `debug` (`orphan_update`) so it is observable rather
/// than invisible. Never delivered to the operator twice: the failed `say` is
/// still the only outcome that reaches them.
pub(super) fn handle_update_notification(
    params: Option<serde_json::Value>,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
) {
    let Some(p) = params.and_then(|v| serde_json::from_value::<Update>(v).ok()) else { return };
    match pending_says.get_mut(&p.prompt_id) {
        Some(pending) => {
            pending.updates.push(SeenUpdate { ts: holler_proto::log::timestamp(), seq: p.seq, parts: p.parts });
        }
        None => {
            super::log(Severity::Debug, "orphan_update", vec![("prompt_id", p.prompt_id)]);
        }
    }
}

/// A confirmation probe's answer (issue #185) — matched by id against every
/// harness's outstanding probe at once, independent of `pending_ping`/
/// `pending_query`'s single-slot discipline and of whatever else (presence, a
/// `hub query` forward, a `say`) interleaves on the wire around it.
pub(super) async fn handle_confirm_response(
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
/// outstanding slot). Split out of [`super::SessionConnection::handle_inbound`]
/// for the same reason as [`handle_presence_notification`].
pub(super) fn handle_response(
    id: &str,
    result: Option<serde_json::Value>,
    pending_ping: &mut PendingPing,
    pending_query: &mut PendingQuery,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
    pending_cancels: &mut std::collections::HashMap<String, tokio::sync::oneshot::Sender<CancelReply>>,
) {
    if let Some(reply) = pending_cancels.remove(id) {
        let outcome = match result.and_then(|v| serde_json::from_value::<holler_proto::CancelResult>(v).ok()) {
            Some(r) if r.applied => CancelReply::Applied,
            _ => CancelReply::ConnectionLost,
        };
        let _ = reply.send(outcome);
        return;
    }
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
pub(super) fn handle_error_response(
    id: Option<&str>,
    error: &WireError,
    pending_query: &mut PendingQuery,
    pending_says: &mut std::collections::HashMap<String, PendingSay>,
    pending_cancels: &mut std::collections::HashMap<String, tokio::sync::oneshot::Sender<CancelReply>>,
) {
    let Some(want_id) = id else { return };
    if let Some(reply) = pending_cancels.remove(want_id) {
        let _ = reply.send(CancelReply::Refused(error.clone()));
        return;
    }
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
/// Send one `query/support {feature: harness}` request per harness, returning
/// the correlation id → harness map `session_loop` matches answers against.
/// A send failure (the socket is already gone) just stops early — whatever
/// was sent still gets a chance to be answered before the socket is
/// discovered dead in the loop proper.
pub(super) async fn send_confirm_probes<Snk>(sink: &mut Snk, harnesses: &[String]) -> std::collections::HashMap<String, String>
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

/// Issue #419: how often a connection persists its token's `last_seen` (the
/// column `hub token list` prints). The presence heartbeat repeats for the
/// connection's whole life and every bump rewrites the whole token store under
/// its lock, so persisting on every beat would make every connected body
/// contend with every other one and with `mint`/`redeem`. Minute-level
/// freshness is all an operator reading `token list` needs.
const LAST_SEEN_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Per-connection throttle for persisting a bound token's `last_seen` on the
/// presence heartbeat (issue #419).
pub(super) struct LastSeenFlusher {
    state: HubState,
    flushed_at: Option<tokio::time::Instant>,
}

impl LastSeenFlusher {
    pub(super) fn new(state: &HubState) -> Self {
        Self { state: state.clone(), flushed_at: None }
    }

    /// Persist `token_id`'s `last_seen` unless this connection already did so
    /// within [`LAST_SEEN_FLUSH_INTERVAL`]. Detached, so a contended token store
    /// never stalls the connection's frame loop; a failure is logged and dropped
    /// (the roster, not the token store, is the liveness source).
    fn beat(&mut self, token_id: &str) {
        let now = tokio::time::Instant::now();
        if self.flushed_at.is_some_and(|t| now.duration_since(t) < LAST_SEEN_FLUSH_INTERVAL) {
            return;
        }
        self.flushed_at = Some(now);
        let (token_id, state) = (token_id.to_string(), self.state.clone());
        tokio::spawn(async move {
            if let Err(e) = crate::token::touch_last_seen_async(&token_id, &state).await {
                super::log(Severity::Warn, "last_seen_touch_failed", vec![("error", e.to_string())]);
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #442
mod hold_tests {
    use super::*;

    /// A sink that records what is written to it and cannot fail.
    #[derive(Default)]
    struct Recording(Vec<Message>);

    impl Sink<Message> for Recording {
        type Error = WsError;
        fn poll_ready(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
            std::task::Poll::Ready(Ok(()))
        }
        fn start_send(mut self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), WsError> {
            self.0.push(item);
            Ok(())
        }
        fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
            std::task::Poll::Ready(Ok(()))
        }
        fn poll_close(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
            std::task::Poll::Ready(Ok(()))
        }
    }

    fn message() -> Box<holler_proto::Message> {
        Box::new(crate::talk::test_user_message("h-01HTESTHOLD00000000000000", "hi"))
    }

    #[tokio::test]
    async fn a_held_session_is_refused_in_every_variant_and_nothing_is_sent() {
        let holds = crate::holds::Holds::in_memory();
        holds.hold("io/alpha", Some("freeze"));
        for (queue, replace) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut sink = Recording::default();
            let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None };
            let res = send_prompt(&mut sink, gate, "h-01HTESTHOLD00000000000000", "alpha", message(), queue, replace).await;
            assert!(matches!(res, Err(SendPromptError::Refused(ref e)) if e.code == -32011 && e.message.contains("freeze")), "queue={queue} replace={replace}");
            assert!(sink.0.is_empty(), "a refused prompt must put nothing on the socket");
        }
    }

    #[tokio::test]
    async fn an_unheld_session_and_a_released_one_are_sent() {
        let holds = crate::holds::Holds::in_memory();
        holds.hold("io/alpha", None);
        let mut sink = Recording::default();
        // A different session is never affected.
        let other = HoldGate { holds: &holds, key: "io/beta", grant: None };
        assert!(send_prompt(&mut sink, other, "h-01HTESTHOLD00000000000000", "beta", message(), false, false).await.is_ok());
        holds.release("io/alpha");
        let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None };
        assert!(send_prompt(&mut sink, gate, "h-01HTESTHOLD00000000000001", "alpha", message(), true, false).await.is_ok());
        assert_eq!(sink.0.len(), 2);
    }
}
