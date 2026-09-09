//! Dispatch one inbound `session/prompt`/`session/cancel`/`session/answer`
//! request: parse its typed params and spawn the matching task in
//! [`crate::prompt_dispatch`], or answer `-32602 invalid_params`. Split out
//! of `connection.rs` proper (issue #151) — inlining `session/answer`'s own
//! parse-or-spawn arm alongside `session/prompt`/`session/cancel`'s pushed
//! that file past the workspace's 900-line file-size guard
//! (`scripts/lint.sh` check 4); this whole three-way dispatch moved here as
//! one unit rather than picking an arbitrary single function to relocate.
//!
//! Issue #191's priority path: `session/cancel`'s own dispatch task writes
//! its `{applied:true}` response via `priority_tx`, a second channel
//! [`crate::connection::LiveConnection`] drains with priority over the
//! normal `outbound_tx` — so a cancel's own ack can never queue behind a
//! large `session/update` flush or another prompt's own outbound frames.
//! `session/prompt`/`session/answer` are unaffected and still use
//! `outbound_tx`.

use std::sync::Arc;

use futures_util::Sink;
use holler_proto::{Answer, Cancel, Code, CorrelationId, Envelope, Prompt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use super::{send, FrameOutcome};
use crate::session_manager::SessionManager;

/// Dispatch one inbound `session/prompt`/`session/cancel`/`session/answer`
/// request: parse its typed params and spawn the matching dispatch task, or
/// answer `-32602 invalid_params`. The caller (`LiveConnection::handle_text`)
/// only reaches this for one of these three method names — this single
/// three-way `match` replaces what would otherwise be three near-identical
/// arms inlined there.
pub(super) async fn dispatch_session_request<Snk>(
    sink: &mut Snk,
    session_manager: &Arc<SessionManager>,
    outbound_tx: &mpsc::UnboundedSender<Message>,
    priority_tx: &mpsc::UnboundedSender<Message>,
    id: String,
    method: String,
    params: Option<serde_json::Value>,
) -> FrameOutcome
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let env = Envelope::Request { id: id.clone(), method: method.clone(), params };
    match method.as_str() {
        "session/prompt" => match holler_proto::typed_params::<Prompt>(&env) {
            Ok(p) => spawn_prompt_dispatch(session_manager, outbound_tx, id, p),
            Err(e) => send_invalid_params(sink, &id, &e).await,
        },
        // Issue #191: dispatched onto `priority_tx`, not `outbound_tx` — see
        // the module doc.
        "session/cancel" => match holler_proto::typed_params::<Cancel>(&env) {
            Ok(p) => spawn_cancel_dispatch(session_manager, priority_tx, id, p),
            Err(e) => send_invalid_params(sink, &id, &e).await,
        },
        // The caller's own guard (`handle_text`) only reaches here for one
        // of these three method names.
        _ => match holler_proto::typed_params::<Answer>(&env) {
            Ok(p) => spawn_answer_dispatch(session_manager, outbound_tx, id, p),
            Err(e) => send_invalid_params(sink, &id, &e).await,
        },
    }
    FrameOutcome::Continue
}

/// Spawn a `session/prompt` dispatch task. Split out (alongside
/// [`spawn_cancel_dispatch`]/[`spawn_answer_dispatch`]/
/// [`send_invalid_params`]) to keep [`dispatch_session_request`]'s own
/// cognitive complexity flat.
fn spawn_prompt_dispatch(
    session_manager: &Arc<SessionManager>,
    outbound_tx: &mpsc::UnboundedSender<Message>,
    id: String,
    params: Prompt,
) {
    let sm = Arc::clone(session_manager);
    let ob = outbound_tx.clone();
    tokio::spawn(crate::prompt_dispatch::handle_prompt(sm, id, params, ob));
}

/// Spawn a `session/cancel` dispatch task. See [`spawn_prompt_dispatch`]'s
/// own doc.
fn spawn_cancel_dispatch(
    session_manager: &Arc<SessionManager>,
    outbound_tx: &mpsc::UnboundedSender<Message>,
    id: String,
    params: Cancel,
) {
    let sm = Arc::clone(session_manager);
    let ob = outbound_tx.clone();
    tokio::spawn(crate::prompt_dispatch::handle_cancel(sm, id, params, ob));
}

/// Spawn a `session/answer` dispatch task (issue #151). See
/// [`spawn_prompt_dispatch`]'s own doc.
fn spawn_answer_dispatch(
    session_manager: &Arc<SessionManager>,
    outbound_tx: &mpsc::UnboundedSender<Message>,
    id: String,
    params: Answer,
) {
    let sm = Arc::clone(session_manager);
    let ob = outbound_tx.clone();
    tokio::spawn(crate::prompt_dispatch::handle_answer(sm, id, params, ob));
}

/// Answer a `session/prompt`/`session/cancel`/`session/answer` whose params
/// failed to parse with `-32602 invalid_params`. See
/// [`spawn_prompt_dispatch`]'s own doc.
async fn send_invalid_params<Snk>(sink: &mut Snk, id: &str, e: &holler_proto::WireError)
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if let Ok(cid) = CorrelationId::parse(id) {
        let err = holler_proto::WireError::new(Code::InvalidParams, &e.message, None);
        let _ = send(sink, &Envelope::error_frame(&cid, &err)).await;
    }
}
