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
//!
//! # Authentication (issue #439)
//!
//! The handshake keeps the `initialize` response's advertised auth methods
//! (ids and kinds only). Only when `session/new` fails with auth-required
//! (`-32000`) does it send one `authenticate` for the session's configured
//! `auth_method`, if [`super::auth::select`] accepts it, and retry
//! `session/new` exactly once. That sequence is [`super::auth::open_session`],
//! shared with the v2 path's `auth/login` (#459). The credential never
//! crosses the wire: the request carries only the method id, and the adapter
//! reads its credential from its own environment.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1;
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, ConnectionTo, Responder};
use holler_proto::docs::PendingKind;
use tokio::sync::oneshot;

use holler_proto::log::Direction as LogDirection;

use super::auth::{self, AdvertisedMethod, AuthProgress, MethodKind};
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

/// The `initialize` response's advertised auth methods, reduced to ids and
/// kinds (issue #439): descriptions and `_meta` are dropped here, unread.
fn advertised_methods(methods: &[v1::AuthMethod]) -> Vec<AdvertisedMethod> {
    methods
        .iter()
        .map(|method| AdvertisedMethod {
            id: method.id().0.to_string(),
            kind: match method {
                v1::AuthMethod::Agent(_) => MethodKind::Agent,
                v1::AuthMethod::Terminal(_) => MethodKind::Terminal,
                // `v1::AuthMethod` is #[non_exhaustive]: a variant a newer SDK adds.
                // (`Agent` is untagged, so an unknown `type` lands in `Agent`
                // above; see `MethodKind::Other`.)
                _ => MethodKind::Other,
            },
        })
        .collect()
}

async fn new_session(
    connection: &ConnectionTo<Agent>,
    cwd: &Path,
) -> Result<v1::SessionId, agent_client_protocol::Error> {
    log_debug(LogDirection::Out, "session/new", vec![], None);
    connection
        .send_request(v1::NewSessionRequest::new(cwd.to_path_buf()))
        .block_task()
        .await
        .map(|response| response.session_id)
}

/// The one `authenticate` request (issue #439), [`auth::open_session`]'s
/// login step on v1. Only the method id is sent or logged; the caller builds
/// the failure reason from the error's code and message.
async fn authenticate(connection: &ConnectionTo<Agent>, method_id: String) -> Result<(), agent_client_protocol::Error> {
    log_debug(LogDirection::Out, "authenticate", vec![("method_id", auth::quote_id(&method_id))], None);
    connection
        .send_request(v1::AuthenticateRequest::new(method_id))
        .block_task()
        .await
        .map(|_response| ())
}

/// `initialize`, `session/new` (with the issue #439 auth flow), then the
/// crash watcher. Every failure is already the startup reason `run` hands
/// back; a non-auth failure's text is today's `Error` display.
async fn do_handshake(
    connection: &ConnectionTo<Agent>,
    cwd: &Path,
    shared: &Arc<Mutex<Shared>>,
    auth_method: Option<&str>,
    progress: &AuthProgress,
) -> Result<v1::SessionId, String> {
    log_debug(LogDirection::Out, "initialize", vec![("protocol", "v1".to_string())], None);
    let initialize = connection
        .send_request(v1::InitializeRequest::new(
            agent_client_protocol::schema::ProtocolVersion::V1,
        ))
        .block_task()
        .await
        .map_err(|e| auth::startup_error_text(&e, auth_method))?;
    let advertised = advertised_methods(&initialize.auth_methods);

    let session_id = auth::open_session(
        &advertised,
        auth_method,
        progress,
        || new_session(connection, cwd),
        |method_id| authenticate(connection, method_id),
    )
    .await?;

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
    })
    .map_err(|e| auth::startup_error_text(&e, auth_method))?;

    Ok(session_id)
}

/// The v1 fallback connection's whole lifetime — the same shape as
/// `connection::run`, speaking the plain (v1) typed client API.
/// `auth_method` is the session's configured ACP auth method id (issue #439).
pub(super) async fn run(
    agent_config: AcpAgentConfig,
    cwd: PathBuf,
    auth_method: Option<String>,
    progress: Arc<AuthProgress>,
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
            match do_handshake(&connection, &cwd, &handshake_shared, auth_method.as_deref(), &progress).await {
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
                Err(reason) => {
                    let _ = ready_tx.send(Err(reason));
                }
            }
            Ok(())
        })
        .await;
}
