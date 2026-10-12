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
#[derive(Debug)]
pub(super) enum SendPromptError {
    /// The session hold, or the pane-state gate, refused the prompt (issue
    /// #442, #460, #646): nothing was sent. Carries the wire error to answer
    /// with (`session_held`, or `invalid_grant` for a grant that was not
    /// honoured).
    Refused(holler_proto::WireError),
    /// The frame could not be built or the socket is gone.
    Io,
}

/// The hold registry and the key [`send_prompt`] checks it with
/// (`<label>/<session>`, built from the connection's own authenticated label),
/// and the pane registry it checks the session against (story #646).
#[derive(Clone)]
pub(super) struct HoldGate<'a> {
    pub(super) holds: &'a crate::holds::Holds,
    pub(super) key: &'a str,
    /// The one-time release grant the sender presented, if any (issue #460).
    pub(super) grant: Option<&'a str>,
    /// The pane registry ([`Registry::panes`]), `None` when the registry was
    /// built without one: then no prompt is refused for a pane's state.
    pub(super) panes: Option<std::sync::Arc<crate::panes::PaneState>>,
}

/// The pane-state refusals of [`send_prompt`] (story #646 part 3), as `data.reason`.
const PANE_PARKED: &str = "pane-parked";
const PANE_UNHEALTHY: &str = "pane-unhealthy";
const PANE_SHOWN_DRIVEN_MISMATCH: &str = "pane-shown-driven-mismatch";

/// What a pane's record says against prompting its session of record: its code and
/// a few words of why, or `None` when nothing does. The record is the source (the
/// hold `park` set, the health and the last observation reconcile wrote), never a
/// live probe: parked; unhealthy; or SHOWN and DRIVEN both observed and differing
/// (one side absent is not a mismatch). Checked in that order.
///
/// Its twin is the CLI's routing engine, `holler_cli::say_cmd::resolve_pane_target`,
/// which refuses the same three conditions before contacting the hub. The predicate
/// exists twice only because `holler-pane` is frozen and the hub cannot depend on the
/// CLI crate; keep the two equal.
fn pane_state_refusal(pane: &holler_pane::Pane) -> Option<(&'static str, &'static str)> {
    use holler_pane::pane::{Health, Hold};
    if matches!(pane.hold, Hold::Parked { .. }) {
        return Some((PANE_PARKED, "is parked"));
    }
    if matches!(pane.harness.health, Health::Unhealthy(_)) {
        return Some((PANE_UNHEALTHY, "is unhealthy"));
    }
    let seen = &pane.last_observed;
    match (seen.shown.as_deref(), seen.driven.as_deref()) {
        (Some(shown), Some(driven)) if shown != driven => Some((
            PANE_SHOWN_DRIVEN_MISMATCH,
            "shows a session other than the one it drives",
        )),
        _ => None,
    }
}

/// The pane-state gate: refuse a prompt to `session` (the bare session name being
/// sent) while any pane whose session of record it is has a refusal
/// ([`pane_state_refusal`]). Fail-closed across panes on different bodies that
/// record the same session name: any bad one refuses, and the first bad one in name
/// order (`list` is name-sorted) is the one named. One synchronous read with no
/// `await`, so it is totally ordered against a pane write, as `holds.admit` is
/// against `hold`/`release`.
///
/// The refusal is `-32011 session_held` with `data.hold_kind = "pane"` and
/// `data.reason` the pane code (no `data.since`: the park's is epoch milliseconds,
/// not the RFC 3339 the field documents). `holler release` does not lift it; the
/// remedy is `holler pane unpark` or `holler pane doctor`.
///
/// A registry that cannot be read (`store-corrupt`, logged by the registry when it
/// failed to load) refuses nothing, as a hold file that cannot be read leaves the
/// hub with no holds (`crate::holds`): a broken pane file must not stop every
/// prompt to every session.
fn pane_gate(
    panes: &crate::panes::PaneState,
    session: &str,
) -> Result<(), holler_proto::WireError> {
    use holler_pane::PaneStore as _;
    let Ok(records) = panes.list() else {
        return Ok(());
    };
    let refused = records
        .iter()
        .filter(|pane| pane.session_of_record.as_deref() == Some(session))
        .find_map(|pane| pane_state_refusal(pane).map(|refusal| (pane, refusal)));
    match refused {
        Some((pane, (code, why))) => Err(holler_proto::WireError::new(
            holler_proto::Code::SessionHeld,
            format!("pane {} {why}, so it refuses prompts ({code})", pane.name),
            Some(code),
        )
        .with_hold_kind("pane")),
        None => Ok(()),
    }
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
///
/// The pane-state gate (story #646, [`pane_gate`]) is checked here too, after
/// the hold: an operator or default hold refuses first, with its own kind.
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
    if let Some(panes) = gate.panes.as_deref() {
        pane_gate(panes, session).map_err(SendPromptError::Refused)?;
    }
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
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442, #646
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
            let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None, panes: None, harness_session: None };
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
        let other = HoldGate { holds: &holds, key: "io/beta", grant: None, panes: None, harness_session: None };
        assert!(send_prompt(&mut sink, other, "h-01HTESTHOLD00000000000000", "beta", message(), false, false).await.is_ok());
        holds.release("io/alpha");
        let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None, panes: None, harness_session: None };
        assert!(send_prompt(&mut sink, gate, "h-01HTESTHOLD00000000000001", "alpha", message(), true, false).await.is_ok());
        assert_eq!(sink.0.len(), 2);
    }

    // --- the pane-state gate (story #646 part 3; keyed on the advertised id since #839) ------
    //
    // `HoldGate` carries the pane registry and, since #839 (D1), the advertised
    // `harness_session_id` of the session being prompted: a prompt is refused while a pane whose
    // session of record or recorded DRIVEN is that id is parked, unhealthy, or has SHOWN != DRIVEN
    // (both observed). The session's name is no key. The selection is
    // `holler_pane::gate::first_refusal`; this is the enforcement point. Ids are `ses_`-shaped
    // (D8); session and pane names are neutral (`oc`, `demo-c*`).

    use std::sync::Arc as PaneArc;

    use holler_pane::pane::{Health as PaneHealth, Hold as PaneHold, LastObserved};
    use holler_pane::{Pane, PaneStore as _};

    const A: &str = "ses_000000000000000000000000a1";
    const B: &str = "ses_000000000000000000000000b2";
    const OLD: &str = "ses_000000000000000000000000c3";

    fn parked_hold() -> PaneHold {
        PaneHold::Parked { reason: "disk full".to_owned(), release_when: "later".to_owned(), since: 5 }
    }

    /// A pane record for the gate: the session of record, hold, health and observation vary.
    fn gate_pane(name: &str, sor: &str, hold: PaneHold, health: PaneHealth, shown: Option<&str>, driven: Option<&str>) -> Pane {
        let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
        pane.session_of_record = Some(sor.to_owned());
        pane.hold = hold;
        pane.harness.health = health;
        pane.last_observed = LastObserved { shown: shown.map(str::to_owned), driven: driven.map(str::to_owned), at: 1 };
        pane
    }

    /// A healthy, unparked pane on `sor` whose SHOWN is `sor` and whose DRIVEN is `driven`.
    fn healthy(name: &str, sor: &str, driven: Option<&str>) -> Pane {
        gate_pane(name, sor, PaneHold::None, PaneHealth::Healthy, Some(sor), driven)
    }

    /// A pane registry seeded with `panes` in a throwaway state dir, which must outlive it.
    fn pane_registry(panes: &[Pane]) -> (PaneArc<crate::panes::PaneState>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::state::HubState::from_root(dir.path().to_path_buf());
        std::fs::create_dir_all(&state.hub_dir).unwrap();
        let registry = PaneArc::new(crate::panes::PaneState::load(&state));
        for pane in panes {
            registry.cas_put(pane, 0).unwrap();
        }
        (registry, dir)
    }

    /// `send_prompt` to the body session `io/oc`, which advertises `harness` (`None`: a spawn
    /// session), in the variant `(queue, replace)`; `Ok` when it was sent, else the refusal.
    async fn send_to(
        holds: &crate::holds::Holds,
        panes: &PaneArc<crate::panes::PaneState>,
        harness: Option<&str>,
        variant: (bool, bool),
    ) -> Result<(), holler_proto::WireError> {
        let gate = HoldGate { holds, key: "io/oc", grant: None, panes: Some(PaneArc::clone(panes)), harness_session: harness };
        let mut sink = Recording::default();
        let res = send_prompt(&mut sink, gate, "h-01HTESTPANE00000000000000", "oc", message(), variant.0, variant.1).await;
        match res {
            Ok(()) => {
                assert_eq!(sink.0.len(), 1, "a sent prompt is one frame");
                Ok(())
            }
            Err(SendPromptError::Refused(e)) => {
                assert!(sink.0.is_empty(), "a refused prompt puts nothing on the socket");
                Err(e)
            }
            Err(SendPromptError::Io) => panic!("the recording sink cannot fail"),
        }
    }

    const VARIANTS: [(bool, bool); 4] = [(false, false), (true, false), (false, true), (true, true)];

    /// `-32011`, `hold_kind == "pane"`, the pane code as `data.reason`, no `data.since`, one line
    /// naming the pane and the code.
    fn assert_pane_refusal(error: &holler_proto::WireError, pane: &str, code: &str) {
        assert_eq!(error.code, -32011, "{error:?}");
        let data = error.data.as_deref().expect("the refusal carries data");
        assert_eq!(data.hold_kind.as_deref(), Some("pane"), "{error:?}");
        assert_eq!(data.reason.as_deref(), Some(code), "{error:?}");
        assert!(data.since.is_none(), "no data.since: the park's epoch-ms since has no RFC 3339 form");
        assert!(!error.message.contains('\n'), "one line: {}", error.message);
        assert!(error.message.contains(pane) && error.message.contains(code), "{}", error.message);
    }

    /// Parked, unhealthy and SHOWN != DRIVEN refuse in every variant (plain, queued, replace), for
    /// the body session whose advertised id is the pane's session of record.
    #[tokio::test]
    async fn a_gated_pane_s_session_of_record_is_refused_in_every_variant() {
        let holds = crate::holds::Holds::in_memory();
        let cases = [
            (gate_pane("demo-c1r1", A, parked_hold(), PaneHealth::Healthy, Some(A), Some(A)), "pane-parked"),
            (gate_pane("demo-c1r1", A, PaneHold::None, PaneHealth::Unhealthy("wedged".to_owned()), Some(A), Some(A)), "pane-unhealthy"),
            (healthy("demo-c1r1", A, Some(OLD)), "pane-shown-driven-mismatch"),
        ];
        for (record, code) in cases {
            let (panes, _dir) = pane_registry(&[record]);
            for variant in VARIANTS {
                let error = send_to(&holds, &panes, Some(A), variant).await.expect_err(code);
                assert_pane_refusal(&error, "demo-c1r1", code);
            }
        }
    }

    /// The recorded DRIVEN is a key (D1): a body that is behind advertises `OLD`, which no record
    /// names as a session of record; once `OLD` is the pane's recorded DRIVEN the pair refuses
    /// it. With no DRIVEN recorded it is admitted: the window ADR-0021 section 11 states.
    #[tokio::test]
    async fn a_body_on_a_pane_s_recorded_driven_is_gated_once_it_is_recorded() {
        let holds = crate::holds::Holds::in_memory();
        let (panes, _dir) = pane_registry(&[healthy("demo-c1r1", A, Some(OLD))]);
        let error = send_to(&holds, &panes, Some(OLD), (false, false)).await.expect_err("gated");
        assert_pane_refusal(&error, "demo-c1r1", "pane-shown-driven-mismatch");
        let (panes, _dir) = pane_registry(&[healthy("demo-c1r1", A, None)]);
        assert!(send_to(&holds, &panes, Some(OLD), (false, false)).await.is_ok(), "the stated window");
    }

    /// The session's name is no key: a parked pane whose session of record is the text `oc` (the
    /// body session's name) does not gate a body that advertises another id, and a spawn session
    /// (no advertised id) is never pane-gated.
    #[tokio::test]
    async fn the_name_is_no_key_and_a_spawn_session_is_never_pane_gated() {
        let holds = crate::holds::Holds::in_memory();
        let (panes, _dir) = pane_registry(&[gate_pane("demo-c1r1", "oc", parked_hold(), PaneHealth::Healthy, Some("oc"), Some("oc"))]);
        assert!(send_to(&holds, &panes, Some(B), (false, false)).await.is_ok(), "named oc, advertising B");
        assert!(send_to(&holds, &panes, None, (false, false)).await.is_ok(), "a spawn session");
    }

    /// A healthy pane on the id (one observation side absent included) and an id no pane names go
    /// through.
    #[tokio::test]
    async fn a_healthy_pane_s_id_and_an_unnamed_id_go_through() {
        let holds = crate::holds::Holds::in_memory();
        let (panes, _dir) = pane_registry(&[healthy("demo-c1r1", A, None)]);
        assert!(send_to(&holds, &panes, Some(A), (false, false)).await.is_ok());
        assert!(send_to(&holds, &panes, Some(B), (true, false)).await.is_ok());
    }

    /// Fail closed across panes that record one id: any bad one refuses, the first bad one in name
    /// order is named, before or after the good one, by either key.
    #[tokio::test]
    async fn any_bad_pane_on_the_id_refuses_fail_closed() {
        let holds = crate::holds::Holds::in_memory();
        let bad_after = gate_pane("demo-c2r1", A, parked_hold(), PaneHealth::Healthy, Some(A), Some(A));
        let (panes, _dir) = pane_registry(&[healthy("demo-c1r1", A, Some(A)), bad_after]);
        let error = send_to(&holds, &panes, Some(A), (false, false)).await.expect_err("refused");
        assert_pane_refusal(&error, "demo-c2r1", "pane-parked");
        let bad_first = gate_pane("demo-c1r1", A, PaneHold::None, PaneHealth::Unhealthy("wedged".to_owned()), Some(A), Some(A));
        let parked_too = gate_pane("demo-c2r1", A, parked_hold(), PaneHealth::Healthy, Some(A), Some(A));
        let (panes, _dir) = pane_registry(&[bad_first, parked_too]);
        let error = send_to(&holds, &panes, Some(A), (false, false)).await.expect_err("refused");
        assert_pane_refusal(&error, "demo-c1r1", "pane-unhealthy");
        let (panes, _dir) = pane_registry(&[healthy("demo-c1r1", B, Some(A)), healthy("demo-c2r1", A, Some(A))]);
        let error = send_to(&holds, &panes, Some(A), (false, false)).await.expect_err("refused");
        assert_pane_refusal(&error, "demo-c1r1", "pane-shown-driven-mismatch");
    }

    /// The pane gate runs after `holds.admit`: an operator hold refuses first, with its own kind.
    #[tokio::test]
    async fn an_operator_hold_still_refuses_first_with_its_own_kind() {
        let holds = crate::holds::Holds::in_memory();
        holds.hold("io/oc", Some("freeze"));
        let (panes, _dir) = pane_registry(&[gate_pane("demo-c1r1", A, parked_hold(), PaneHealth::Healthy, Some(A), Some(A))]);
        let error = send_to(&holds, &panes, Some(A), (false, false)).await.expect_err("held");
        assert_eq!(error.code, -32011);
        let data = error.data.as_deref().expect("data");
        assert_eq!(data.hold_kind.as_deref(), Some("operator"), "the hold's own kind");
        assert_ne!(data.reason.as_deref(), Some("pane-parked"));
        assert!(error.message.contains("freeze"), "{}", error.message);
    }

    /// The registry carries the pane-state handle through `with_panes`, which `circuit.rs`'s one
    /// `HoldGate` construction reads back out.
    #[test]
    fn the_registry_carries_the_pane_state_through_with_panes() {
        let (panes, _dir) = pane_registry(&[]);
        let registry = crate::live::Registry::new().with_panes(PaneArc::clone(&panes));
        assert!(registry.panes().is_some(), "the handle is carried");
    }
}
