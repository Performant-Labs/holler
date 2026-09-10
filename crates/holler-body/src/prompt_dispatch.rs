//! The body-side async glue for inbound `session/prompt` / `session/cancel`
//! (issue #190): resolve the request against the live
//! [`crate::session_manager::SessionManager`], drive mid-turn text chunks
//! through a [`crate::reply_coalescer::Coalescer`] and emit `session/update`
//! notifications as it flushes, then answer the original request once the
//! turn ends.
//!
//! Each inbound `session/prompt`/`session/cancel` is spawned as its own
//! task by [`crate::connection`] so one session's long-running turn never
//! blocks another session's, or the connection's own heartbeat/ping
//! handling. Every task writes to the wire exclusively through the
//! `outbound` channel — never a `&mut` of the socket sink directly — since
//! several of these tasks (plus the connection loop's own heartbeat) can be
//! live at once and only one owner may ever call `sink.send`.
//!
//! # Decisions I made
//!
//! - **`Prompt.message`'s text is the concatenation of its text parts.**
//!   [`crate::acp_driver::AcpDriver::prompt`] takes a plain `&str` (the ACP
//!   v2 prompt content is itself a list of content blocks, but this driver's
//!   own surface, issue #188, only ever sends one text block) — a
//!   non-text part in an inbound `Prompt.message` is silently dropped rather
//!   than refused, since nothing in the issue's RED list exercises a
//!   multi-part or non-text `say`.
//! - **A driver-level failure (`PromptOutcome::Error`) is answered as a
//!   normal, completed `session/prompt` response** with
//!   `stop_reason:"error"`/`state:"failed"` (via the same
//!   `state_for_stop_reason` table [`crate::session_manager::task`] uses),
//!   not a JSON-RPC error frame — the closed v2 error table (docs §8) has no
//!   code for "the harness could not be brought up"; a failed *turn* is A2A
//!   vocabulary, not a protocol error.
//! - **`session/cancel` against a driver-level failure** (the driver
//!   reported an error cancelling, or the session task is gone) is answered
//!   `-32004 not_connected` — the closest existing code to "this session's
//!   driver could not be reached right now"; there is no dedicated
//!   "cancel failed" code in the v2 table either.
//! - **`session/answer` (issue #151)** maps `SessionManager::answer`'s three
//!   failure shapes onto the closed v2 table: nothing pending →
//!   `-32010 nothing_pending` (the code's whole reason for existing); the
//!   `choice` not resolving, or an unsupported elicitation shape → `-32602
//!   invalid_params` (malformed *content*, not a protocol violation); every
//!   other failure (a dropped connection, a driver that could not be
//!   reached) → `-32004 not_connected`, the same fallback `session/cancel`
//!   uses just above.

use std::sync::Arc;
use std::time::Instant;

use holler_proto::log::{Component, Direction as LogDirection, Event as LogEvent, Severity};
use holler_proto::{
    Answer, AnswerResult, Cancel, CancelResult, Code, CorrelationId, Envelope, Message, Part,
    Prompt, PromptResult, Role, SessionName, WireError,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::reply_coalescer::{Coalescer, PushOutcome};
use crate::session_manager::{PromptOutcome, SessionManager, SessionManagerError};

/// Handle one inbound `session/prompt` request. Spawned by the connection
/// loop; writes only to `outbound`.
pub async fn handle_prompt(
    session_manager: Arc<SessionManager>,
    id: String,
    params: Prompt,
    outbound: mpsc::UnboundedSender<WsMessage>,
) {
    let Ok(cid) = CorrelationId::parse(&id) else { return };
    let name = match SessionName::parse(&params.session) {
        Ok(n) => n,
        Err(e) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("bad session name: {e}"));
            return;
        }
    };

    let text = message_text(&params.message);
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<String>();
    let updates_task = tokio::spawn(drive_updates(
        params.session.clone(),
        id.clone(),
        chunk_rx,
        outbound.clone(),
    ));

    // `replace` (issue #191, `interrupt SESSION TEXT`): cancel whatever is
    // in flight and run this prompt ahead of the queue instead of the normal
    // busy-check dispatch — `SessionManager::replace_with_updates` is exactly
    // `SessionCommand::Replace`'s own entry point. The hub only ever sets
    // `replace` after it has itself confirmed the cancel (`control/interrupt`
    // waits for `{applied:true}` before sending this request), so by the time
    // this arrives the session is expected to already be idle — `replace`
    // still cancels defensively rather than assuming that.
    let outcome = if params.replace {
        session_manager.replace_with_updates(&name, text, Some(chunk_tx)).await
    } else {
        session_manager.prompt_with_updates(&name, id.clone(), text, params.queue, Some(chunk_tx)).await
    };
    // The updates task ends on its own once the session task drops its
    // `current_updates` sender (turn-end, before the reply fires — see
    // `session_manager::task::finish_turn_no_dispatch`), so by the time
    // `outcome` resolves this join is already done or a moment away.
    let final_parts = updates_task.await.unwrap_or_default();

    match outcome {
        Ok(PromptOutcome::Result { stop_reason, state, .. }) => {
            let message = agent_message(&id, final_parts);
            let result = PromptResult { stop_reason, state: state.as_str().to_string(), message };
            send_response(&outbound, &cid, &result);
        }
        Ok(PromptOutcome::Busy { state, turn_age_ms, last_update_age_ms }) => {
            let err = WireError::session_busy(state, turn_age_ms, last_update_age_ms);
            send_error_frame(&outbound, &cid, &err);
        }
        Ok(PromptOutcome::QueueFull) => {
            send_error(&outbound, &cid, Code::LimitExceeded, "the queue for this session is full");
        }
        Ok(PromptOutcome::Error(msg)) => {
            // A driver-level failure is a completed (failed) turn, not a
            // protocol error — see the module doc's "Decisions I made".
            let stop_reason = "error".to_string();
            let state = holler_proto::state_for_stop_reason(&stop_reason)
                .unwrap_or(holler_proto::SessionState::Failed);
            let message = agent_message(&id, vec![Part::text_part(msg)]);
            let result = PromptResult { stop_reason, state: state.as_str().to_string(), message };
            send_response(&outbound, &cid, &result);
        }
        Err(SessionManagerError::UnknownSession) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("no such session: {}", params.session));
        }
        Err(SessionManagerError::Gone) => {
            send_error(&outbound, &cid, Code::NotConnected, "the session task is gone");
        }
    }
}

/// Handle one inbound `session/cancel` request.
pub async fn handle_cancel(
    session_manager: Arc<SessionManager>,
    id: String,
    params: Cancel,
    outbound: mpsc::UnboundedSender<WsMessage>,
) {
    let Ok(cid) = CorrelationId::parse(&id) else { return };
    let name = match SessionName::parse(&params.session) {
        Ok(n) => n,
        Err(e) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("bad session name: {e}"));
            return;
        }
    };
    match session_manager.cancel(&name).await {
        Ok(Ok(())) => send_response(&outbound, &cid, &CancelResult { applied: true }),
        Ok(Err(msg)) => send_error(&outbound, &cid, Code::NotConnected, &msg),
        Err(SessionManagerError::UnknownSession) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("no such session: {}", params.session));
        }
        Err(SessionManagerError::Gone) => {
            send_error(&outbound, &cid, Code::NotConnected, "the session task is gone");
        }
    }
}

/// Handle one inbound `session/answer` request (issue #151). Resolves a held
/// permission/elicitation via [`SessionManager::answer`] and answers
/// `{applied:true}` once the driver reports the turn resumed (or ended
/// outright) — never before, per that method's own contract. A session with
/// nothing pending answers `-32010 nothing_pending`; a `choice` that did not
/// resolve, or an elicitation shape this driver does not resolve, answers
/// `-32602 invalid_params` (the closest existing code to "your request was
/// well-formed but its content was wrong"); every other failure (the
/// connection ended, or the driver could not be reached) answers `-32004
/// not_connected`, mirroring [`handle_cancel`]'s own mapping.
pub async fn handle_answer(
    session_manager: Arc<SessionManager>,
    id: String,
    params: Answer,
    outbound: mpsc::UnboundedSender<WsMessage>,
) {
    let Ok(cid) = CorrelationId::parse(&id) else { return };
    let name = match SessionName::parse(&params.session) {
        Ok(n) => n,
        Err(e) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("bad session name: {e}"));
            return;
        }
    };
    match session_manager.answer(&name, params.choice.clone()).await {
        Ok(Ok(())) => send_response(&outbound, &cid, &AnswerResult { applied: true }),
        Ok(Err(msg)) => send_answer_refusal(&outbound, &cid, &msg),
        Err(SessionManagerError::UnknownSession) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("no such session: {}", params.session));
        }
        Err(SessionManagerError::Gone) => {
            send_error(&outbound, &cid, Code::NotConnected, "the session task is gone");
        }
    }
}

/// Map [`crate::session_manager::task::handle_answer`]'s one-line failure
/// strings to the closed v2 error table. Split out of [`handle_answer`] to
/// keep that function's own match flat.
fn send_answer_refusal(outbound: &mpsc::UnboundedSender<WsMessage>, cid: &CorrelationId, msg: &str) {
    if msg == "nothing pending to answer" {
        send_error_frame(outbound, cid, &WireError::nothing_pending());
    } else if msg.contains("did not resolve") || msg.contains("unsupported elicitation") {
        send_error(outbound, cid, Code::InvalidParams, msg);
    } else {
        send_error(outbound, cid, Code::NotConnected, msg);
    }
}

/// The concatenation of `message`'s text parts (see the module doc's
/// "Decisions I made" — a non-text part is dropped, not refused).
fn message_text(message: &Message) -> String {
    message.parts.iter().filter_map(|p| p.text()).collect::<Vec<_>>().join("")
}

/// Build the outward A2A reply message: `role:"agent"`, one merged text part
/// per the coalescer's own "consecutive text parts merged" contract.
fn agent_message(turn_id: &str, parts: Vec<Part>) -> Message {
    Message {
        message_id: format!("m-{turn_id}"),
        context_id: None,
        task_id: None,
        role: Role::RoleAgent,
        parts,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

/// Drain `chunk_rx` through a [`Coalescer`], sending one `session/update`
/// notification per window/cap flush, until the sender side closes (the
/// session task drops its updates channel at turn end) — then flush any
/// stragglers and return the **full** reply as one merged text part (empty
/// `Vec` if nothing ever streamed).
async fn drive_updates(
    session: String,
    prompt_id: String,
    mut chunk_rx: mpsc::UnboundedReceiver<String>,
    outbound: mpsc::UnboundedSender<WsMessage>,
) -> Vec<Part> {
    let mut coalescer = Coalescer::new();
    let mut total = String::new();
    let mut seq: u64 = 0;
    loop {
        let deadline = coalescer.next_deadline();
        tokio::select! {
            biased;
            chunk = chunk_rx.recv() => {
                match chunk {
                    Some(text) => {
                        total.push_str(&text);
                        if coalescer.push(&text, Instant::now()) == PushOutcome::CapHit {
                            flush_and_send(&mut coalescer, &outbound, &session, &prompt_id, &mut seq);
                        }
                    }
                    None => {
                        // Turn end: flush any stragglers before returning.
                        flush_and_send(&mut coalescer, &outbound, &session, &prompt_id, &mut seq);
                        break;
                    }
                }
            }
            () = sleep_until_deadline(deadline) => {
                if coalescer.due(Instant::now()) {
                    flush_and_send(&mut coalescer, &outbound, &session, &prompt_id, &mut seq);
                }
            }
        }
    }
    if total.is_empty() {
        Vec::new()
    } else {
        vec![Part::text_part(total)]
    }
}

fn flush_and_send(
    coalescer: &mut Coalescer,
    outbound: &mpsc::UnboundedSender<WsMessage>,
    session: &str,
    prompt_id: &str,
    seq: &mut u64,
) {
    let Some(parts) = coalescer.flush() else { return };
    *seq += 1;
    let update = holler_proto::Update {
        session: session.to_string(),
        prompt_id: prompt_id.to_string(),
        seq: *seq,
        parts,
    };
    let env = Envelope::notification("session/update", serde_json::to_value(update).ok());
    send_frame(outbound, &env);
}

/// Wait until `deadline`, or pend forever when there is none (an idle
/// coalescer has no window to wait out) — the same "absent branch parks
/// inert in `select!`" trick [`crate::session_manager::task::poll_stream`]
/// uses for its own driver-event stream.
async fn sleep_until_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(d) => tokio::time::sleep_until(tokio::time::Instant::from_std(d)).await,
        None => std::future::pending().await,
    }
}

/// The single choke point every outbound frame this module produces goes
/// through — `session/update` notifications and every `session/prompt`/
/// `session/cancel`/`session/answer` *response* — none of which touch
/// `crate::connection::send` at all (they go via `outbound`/`priority_tx`,
/// relayed to the socket later by `connection::relay_outbound`). Before
/// issue #197 this had **no wire logging whatsoever**: this is exactly the
/// holler-server#207 regression ("noisy never showed prompt/reply") —
/// `session/update` in particular never appeared in a trace at any debug
/// level, quiet or noisy, because nothing on this path ever logged a frame.
fn send_frame(outbound: &mpsc::UnboundedSender<WsMessage>, env: &Envelope) {
    if let Ok(text) = holler_proto::encode(env) {
        let method: &'static str = match env.method() {
            Some("session/update") => "session/update",
            _ => match env {
                Envelope::Response { .. } | Envelope::Error { .. } => "session/prompt",
                _ => "other",
            },
        };
        holler_proto::log::emit(&LogEvent {
            component: Component::Wire,
            severity: Severity::Debug,
            direction: LogDirection::Out,
            method,
            id: None,
            peer: None,
            fields: env.id().map(|i| vec![("id", i.to_string())]).unwrap_or_default(),
            frame: holler_proto::log::frame_at_noisy(&text),
        });
        let _ = outbound.send(WsMessage::text(text));
    }
}

fn send_response(outbound: &mpsc::UnboundedSender<WsMessage>, cid: &CorrelationId, result: &impl serde::Serialize) {
    let env = Envelope::response(cid, serde_json::to_value(result).ok());
    send_frame(outbound, &env);
}

fn send_error(outbound: &mpsc::UnboundedSender<WsMessage>, cid: &CorrelationId, code: Code, message: &str) {
    send_error_frame(outbound, cid, &WireError::new(code, message, None));
}

fn send_error_frame(outbound: &mpsc::UnboundedSender<WsMessage>, cid: &CorrelationId, error: &WireError) {
    let env = Envelope::error_frame(cid, error);
    send_frame(outbound, &env);
}
