//! The ACP v1 fallback connection (issue #362): the same background-task
//! shape as `connection.rs`'s v2 path, speaking the plain (v1) typed API
//! instead of `V2ConnectionTo`.
//!
//! Why this exists: `AcpDriver` targets ACP protocol v2 (ADR 0013), but as
//! of 2026-09-21 no real ACP implementation in the wild — not the newest
//! `opencode acp`, not the newest published `@agentclientprotocol/sdk` (the
//! Claude Code bridge's own dependency) — negotiates past v1 (issue #362's
//! writeup has the evidence). Every real harness this driver spawns will
//! negotiate v1 until the ecosystem catches up, so `AcpDriver::spawn` tries
//! v2 first and falls back to this module's `run` only on the specific
//! "peer negotiated v1" failure (see `is_v1_negotiation_failure` in the
//! parent module) — never as the default path, and never silently: every
//! fallback is logged.
//!
//! # Fidelity lost in this mode
//!
//! ACP v1 has no `requires_action` session state at all — that concept (and
//! Holler's own `input-required` wire state built on it, ADR 0013) is a v2
//! addition. A v1 agent's permission request arrives as a plain
//! `session/request_permission` RPC call mid-turn, with no separate
//! state_update notification framing it. This module still surfaces it
//! through the exact same `PendingBlock`/`answer()` machinery `connection.rs`
//! uses (`store_pending`, reused directly — it is already version-neutral),
//! so a caller sees the same `InputRequired` status and the same `answer()`
//! contract either way; only the *wire* shape of what triggered it differs.
//!
//! The other v1/v2 difference this module works around: v1's
//! `session/prompt` response carries the turn's `stop_reason` directly (no
//! separate `idle` state_update to wait for, unlike v2 — see `acp_driver.rs`'s
//! module doc on why v2 needed that split). `AcpDriver::prompt`'s v1 branch
//! therefore awaits that response in a background task rather than firing
//! the request and returning immediately the way the v2 branch does.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1;
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, ConnectionTo, Responder};
use holler_proto::docs::PendingKind;
use tokio::sync::oneshot;

use holler_proto::log::Direction as LogDirection;

use super::connection::{lock, store_pending, Shared};
use super::pending::{chunk_text_v1, permission_fields_v1, PendingBlock, PendingResponder};
use super::{log_debug, log_warn, DriverEvent, Status, StopReason};

/// What the v1 background task hands back once `initialize` + `session/new`
/// succeed (or the reason they didn't) — the v1-typed twin of
/// `connection::Ready`.
pub(super) struct ReadyV1 {
    pub(super) session_id: v1::SessionId,
    pub(super) connection: ConnectionTo<Agent>,
}

fn handle_notification(shared: &Arc<Mutex<Shared>>, note: v1::SessionNotification) {
    if let v1::SessionUpdate::AgentMessageChunk(chunk) = note.update {
        if let Some(text) = chunk_text_v1(&chunk.content) {
            log_debug(
                LogDirection::In,
                "session/update",
                vec![("chunk_len", text.len().to_string())],
                holler_proto::log::frame_at_noisy(&serde_json::json!({ "chunk": text }).to_string()),
            );
            let guard = lock(shared);
            if let Some(tx) = &guard.current_events {
                let _ = tx.send(DriverEvent::Chunk(text));
            }
        }
    }
    // Every other v1 `SessionUpdate` variant (tool calls, plans, usage, …) is
    // outside this driver's scope in either protocol mode — see
    // `connection.rs::handle_notification`'s identical `_ => {}`.
}

fn handle_permission_request(
    shared: &Arc<Mutex<Shared>>,
    request: v1::RequestPermissionRequest,
    responder: Responder<v1::RequestPermissionResponse>,
) {
    let (fields, unsupported) = permission_fields_v1(&request);
    // v1 has no direct `title` field on the request itself (see the module
    // doc); the closest human-readable prompt is the tool-call update's own
    // optional title.
    let prompt = request.tool_call.fields.title.clone().unwrap_or_else(|| "permission request".to_string());
    store_pending(
        shared,
        PendingBlock {
            fields,
            responder: PendingResponder::PermissionV1(responder),
            unsupported,
            kind: PendingKind::Permission,
            prompt,
        },
    );
}

async fn do_handshake(
    connection: &ConnectionTo<Agent>,
    cwd: &Path,
    shared: &Arc<Mutex<Shared>>,
) -> Result<v1::SessionId, agent_client_protocol::Error> {
    log_debug(LogDirection::Out, "initialize", vec![("protocol", "v1".to_string())], None);
    let _initialize = connection
        .send_request(v1::InitializeRequest::new(
            agent_client_protocol::schema::ProtocolVersion::V1,
        ))
        .block_task()
        .await?;

    log_debug(LogDirection::Out, "session/new", vec![], None);
    let new_session = connection
        .send_request(v1::NewSessionRequest::new(cwd.to_path_buf()))
        .block_task()
        .await?;
    let session_id = new_session.session_id;

    // Crash watcher: identical shape to `connection.rs::do_handshake`'s own —
    // see that function's comment for why this reports `Done(Error)` rather
    // than hanging when the transport closes unexpectedly.
    let watch_shared = shared.clone();
    let watch_conn = connection.clone();
    connection.spawn(async move {
        watch_conn.incoming_closed().await;
        log_warn("spawn", vec![("event", "child_exit: connection closed (v1)".to_string())]);
        let mut guard = lock(&watch_shared);
        guard.last_stop_reason = Some(StopReason::Error);
        if let Some(tx) = guard.current_events.take() {
            let _ = tx.send(DriverEvent::Done(StopReason::Error));
        }
        if let Some(done_tx) = guard.awaiting_done.take() {
            let _ = done_tx.send(StopReason::Error);
        }
        guard.status = Status::Idle;
        Ok(())
    })?;

    Ok(session_id)
}

/// The v1 fallback connection's whole lifetime — the same shape as
/// `connection::run`, speaking the plain (v1) typed client API.
pub(super) async fn run(
    agent_config: AcpAgentConfig,
    cwd: PathBuf,
    shared: Arc<Mutex<Shared>>,
    ready_tx: oneshot::Sender<Result<ReadyV1, String>>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    let agent = AcpAgent::new(agent_config);
    let notif_shared = shared.clone();
    let perm_shared = shared.clone();
    let handshake_shared = shared.clone();
    let mut shutdown_rx = shutdown_rx;

    let _ = Client
        .builder()
        .name("holler-body-v1")
        .on_receive_notification(
            move |note: v1::SessionNotification, _cx: ConnectionTo<Agent>| {
                let shared = notif_shared.clone();
                async move {
                    handle_notification(&shared, note);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            move |request: v1::RequestPermissionRequest,
                  responder: Responder<v1::RequestPermissionResponse>,
                  _cx: ConnectionTo<Agent>| {
                let shared = perm_shared.clone();
                async move {
                    handle_permission_request(&shared, request, responder);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, move |connection: ConnectionTo<Agent>| async move {
            match do_handshake(&connection, &cwd, &handshake_shared).await {
                Ok(session_id) => {
                    let _ = ready_tx.send(Ok(ReadyV1 {
                        session_id,
                        connection: connection.clone(),
                    }));
                    tokio::select! {
                        _ = &mut shutdown_rx => {}
                        _ = connection.incoming_closed() => {}
                    }
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                }
            }
            Ok(())
        })
        .await;
}
