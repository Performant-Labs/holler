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

/// Send a `session/prompt {session, message, queue, replace}` request to the
/// body under `request_id`. `replace` (issue #191) is set only by
/// `interrupt SESSION TEXT`, after the matching cancel's own `{applied:true}`
/// — see `crate::interrupt`.
pub(super) async fn send_prompt<Snk>(
    sink: &mut Snk,
    request_id: &str,
    session: &str,
    message: Box<holler_proto::Message>,
    queue: bool,
    replace: bool,
) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let cid = holler_proto::CorrelationId::parse(request_id).map_err(|_| ())?;
    let params = holler_proto::Prompt { session: session.to_string(), message: *message, meta: None, queue, replace };
    let req = Envelope::request(&cid, "session/prompt", Some(serde_json::to_value(params).map_err(|_| ())?));
    let text = holler_proto::encode(&req).map_err(|_| ())?;
    super::log_frame(super::LogDirection::Out, "session/prompt", Some(request_id), &text);
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
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
    registry: &Registry,
    roster: &std::sync::Arc<crate::roster::Roster>,
) {
    let Some(p) = params.and_then(|v| serde_json::from_value::<Presence>(v).ok()) else { return };
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
