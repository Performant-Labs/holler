//! `interrupt` (issue #191): the hub-side orchestration behind
//! `control/interrupt`. `holler interrupt SESSION` cancels the target
//! session's in-flight turn and confirms the cancel was *applied*, not
//! merely received; `holler interrupt SESSION TEXT` (the redirect form)
//! additionally sends a fresh prompt right after, streaming its reply
//! exactly like `say` — one atomic operator action, so a redirect can never
//! be lost behind a stuck turn.
//!
//! # Decisions I made
//!
//! - **The ack wait is RTT-scaled, not the connection's liveness timeout.**
//!   [`ack_timeout`] is `max(2s, 10 × last measured circuit/ping RTT)` — a
//!   *round-trip* budget, deliberately blind to the heartbeat/presence
//!   cadence (`crate::circuit::heartbeat_interval`) and to the liveness
//!   timeout (`crate::circuit::liveness_timeout`), which answer a different
//!   question ("has this body gone silent") than "how long should one
//!   request/response pair reasonably take". The unit test proving that
//!   independence lives in `crates/holler-cli/tests/interrupt_test.rs`
//!   (`interrupt_ack_timeout_is_independent_of_heartbeat`), calling this
//!   function directly with `HOLLER_HEARTBEAT_INTERVAL_MS` set to a wildly
//!   different value and confirming the result does not move.
//! - **No prior RTT measurement floors the timeout at 2s.** A body that has
//!   never been `hub token ping`ed and has never had a prior `interrupt`
//!   confirmed has no RTT sample; `ack_timeout(None)` is the 2s floor, not a
//!   division-by-zero special case.
//! - **The redirect's own prompt phase reuses `say`'s reply-collection tail**
//!   (`crate::talk::send_replace_turn`) rather than duplicating it — the two
//!   differ only in whether a busy check runs first (`say` does; the
//!   redirect skips it because the cancel just confirmed the session is
//!   idle) and in the wire `Prompt.replace` flag.
//! - **The redirect prompt's own timeout is a fixed 600s** (mirroring
//!   `say`'s own CLI default) — ADR 0003's `interrupt SESSION [TEXT]` has no
//!   `--timeout` flag of its own to thread through.

use std::time::{Duration, Instant};

use crate::live::{CancelReply, LiveHandle, Registry, ResolveOutcome};
use crate::state::HubState;
use crate::talk::{self, SayError, SayOutcome};

/// The redirect prompt's own wait (no `--timeout` flag exists on `interrupt`
/// — see the module doc's "Decisions I made").
const REPLACE_TIMEOUT: Duration = Duration::from_secs(600);

/// Why [`interrupt`] could not complete.
pub enum InterruptError {
    /// No live session matched `session` at all.
    UnknownSession,
    /// A session by this name exists but its body is not currently
    /// connected.
    NotConnected,
    /// More than one live session matched a bare name.
    Ambiguous(Vec<String>),
    /// The `{applied:true}` ack did not arrive within [`ack_timeout`]'s
    /// window — the spec's own "body may be stalled; the session is still
    /// on the roster" case, distinct from `ConnectionLost` (the connection
    /// itself may still be perfectly healthy).
    AckTimeout { secs: u64 },
    /// The body answered the cancel with some other JSON-RPC refusal.
    Refused(holler_proto::WireError),
    /// The socket dropped before the cancel's response arrived.
    ConnectionLost,
    /// The cancel was applied, but the redirect's own prompt phase failed —
    /// carries `say`'s own error shape (unknown/busy/refused/… can't
    /// actually recur here since the session was just confirmed idle, but
    /// `ConnectionLost`/`Timeout`/`Cancelled` are all real possibilities for
    /// a fresh turn on a live socket).
    PromptFailed(SayError),
}

/// A successful [`interrupt`] outcome. `reply` is `Some` only for the
/// `interrupt SESSION TEXT` redirect form.
pub struct InterruptOutcome {
    pub session: String,
    pub reply: Option<SayOutcome>,
}

/// The `{applied:true}` ack's own wait: `max(2s, 10 × last measured circuit/
/// ping RTT)`. Distinct from `crate::circuit::liveness_timeout` (which
/// answers "has this body gone silent", not "how long should one round trip
/// take") and deliberately does not read `crate::circuit::heartbeat_interval`
/// or its `HOLLER_HEARTBEAT_INTERVAL_MS` override at all — see the module
/// doc's "Decisions I made".
pub fn ack_timeout(last_measured_rtt: Option<Duration>) -> Duration {
    let scaled = last_measured_rtt.and_then(|d| d.checked_mul(10)).unwrap_or(Duration::ZERO);
    std::cmp::max(Duration::from_secs(2), scaled)
}

/// `holler interrupt SESSION [TEXT]` (issue #191): resolve `session` against
/// the live hub, send `session/cancel` on its priority channel, wait for the
/// body's `{applied:true}` (bounded by [`ack_timeout`]), then — only when
/// `text` is given — send a fresh `session/prompt {replace:true}` and
/// collect its reply exactly like `say`.
pub async fn interrupt(
    registry: &Registry,
    state: &HubState,
    session: &str,
    text: Option<&str>,
) -> Result<InterruptOutcome, InterruptError> {
    let name = holler_proto::RoutableName::parse(session).map_err(|_| InterruptError::UnknownSession)?;
    let (handle, ad) = match registry.resolve_session(&name).await {
        ResolveOutcome::Found(h, ad) => (h, ad),
        ResolveOutcome::Unknown => return Err(InterruptError::UnknownSession),
        ResolveOutcome::NotConnected => return Err(InterruptError::NotConnected),
        ResolveOutcome::Ambiguous(candidates) => return Err(InterruptError::Ambiguous(candidates)),
    };

    apply_cancel(&handle, ad.name.as_str()).await?;

    let session_full = format!("{}/{}", handle.hostname, ad.name);
    let Some(text) = text else {
        return Ok(InterruptOutcome { session: session_full, reply: None });
    };

    match talk::send_replace_turn(registry, state, &handle, &ad, text, REPLACE_TIMEOUT).await {
        Ok(outcome) => Ok(InterruptOutcome { session: session_full, reply: Some(outcome) }),
        Err(e) => Err(InterruptError::PromptFailed(e)),
    }
}

/// Send `session/cancel` for `session_name` on `handle`'s priority channel
/// and await `{applied:true}`, recording the round trip's own RTT for the
/// *next* interrupt's [`ack_timeout`] scale on success. Split out of
/// [`interrupt`] to keep that function's own control flow (resolve → cancel
/// → optional redirect) flat and readable.
async fn apply_cancel(handle: &LiveHandle, session_name: &str) -> Result<(), InterruptError> {
    let last_rtt = handle.last_rtt().await;
    let timeout = ack_timeout(last_rtt);
    let request_id = holler_proto::CorrelationId::mint_hub().as_str().to_string();
    let started = Instant::now();

    match handle.cancel(request_id, session_name.to_string(), timeout).await {
        None => Err(InterruptError::AckTimeout { secs: timeout.as_secs() }),
        Some(CancelReply::Refused(e)) => Err(InterruptError::Refused(e)),
        Some(CancelReply::ConnectionLost) => Err(InterruptError::ConnectionLost),
        Some(CancelReply::Applied) => {
            handle.record_rtt(started.elapsed()).await;
            Ok(())
        }
    }
}
