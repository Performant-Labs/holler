//! The body-side wire-dispatch leaves `holler_body::connection`'s live loop
//! spawns/calls for inbound `session/prompt`/`session/cancel`/`query/*`
//! requests: spawning the actual (long-running) `prompt_dispatch` tasks,
//! answering a bad-params request with `-32602`, and answering `query/*`
//! from local state. Split out of `connection.rs` (issue #191) once that
//! file's own priority-path/RTT additions pushed it past the workspace's
//! 900-line-per-file guard (`scripts/lint.sh`) — a pure move, no behavior
//! change: every function here is exactly what `connection.rs` used to
//! define inline, just relocated (and `pub(crate)` where a caller there
//! still needs it).

use std::path::Path;
use std::sync::Arc;

use holler_proto::{Cancel, Code, CorrelationId, Envelope, Prompt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::config::SessionConfig;
use crate::connection::{send, FrameOutcome};
use crate::identity::BodyIdentity;
use crate::session_manager::SessionManager;

/// Spawn a `session/prompt` dispatch task. Split out of
/// [`crate::connection::LiveConnection::handle_text`] (used once, but
/// alongside [`spawn_cancel_dispatch`]/[`send_invalid_params`], to keep that
/// dispatch's cognitive complexity under the workspace threshold).
pub(crate) fn spawn_prompt_dispatch(
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
pub(crate) fn spawn_cancel_dispatch(
    session_manager: &Arc<SessionManager>,
    outbound_tx: &mpsc::UnboundedSender<Message>,
    id: String,
    params: Cancel,
) {
    let sm = Arc::clone(session_manager);
    let ob = outbound_tx.clone();
    tokio::spawn(crate::prompt_dispatch::handle_cancel(sm, id, params, ob));
}

/// Answer a `session/prompt`/`session/cancel` whose params failed to parse
/// with `-32602 invalid_params`. See [`spawn_prompt_dispatch`]'s own doc.
pub(crate) async fn send_invalid_params<Snk>(sink: &mut Snk, id: &str, e: &holler_proto::WireError)
where
    Snk: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    if let Ok(cid) = CorrelationId::parse(id) {
        let err = holler_proto::WireError::new(Code::InvalidParams, &e.message, None);
        let _ = send(sink, &Envelope::error_frame(&cid, &err)).await;
    }
}

/// Answer one `query/*` request (issue #185) from local state — see
/// [`crate::query`] for the document builders. `query/support` with an
/// unknown id answers `-32006`; every other case answers `Ok`.
pub(crate) async fn handle_query<Snk>(
    sink: &mut Snk,
    id: &str,
    method: &str,
    params: Option<serde_json::Value>,
    identity: &BodyIdentity,
    state_root: &Path,
    configs: &[SessionConfig],
) -> FrameOutcome
where
    Snk: futures_util::Sink<Message, Error = WsError> + Unpin,
{
    let Ok(cid) = CorrelationId::parse(id) else {
        return FrameOutcome::Continue;
    };
    let result: Result<serde_json::Value, holler_proto::WireError> = match method {
        "query/status" => Ok(serde_json::to_value(crate::query::local_status(state_root, Some(identity), configs))
            .unwrap_or_default()),
        "query/caps" => Ok(serde_json::to_value(crate::query::local_caps(state_root, Some(identity), configs))
            .unwrap_or_default()),
        "query/support" => {
            let feature = params
                .as_ref()
                .and_then(|p| p.get("feature"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            crate::query::local_support(feature, configs).map(|s| serde_json::to_value(s).unwrap_or_default())
        }
        "query/protocol" => holler_proto::ProtocolParams::parse_version(params.as_ref())
            .map(|version| serde_json::to_value(crate::query::local_protocol(version)).unwrap_or_default()),
        _ => Err(holler_proto::WireError::new(Code::MethodNotFound, "unknown query method", None)),
    };
    let send_result = match result {
        Ok(value) => send(sink, &Envelope::response(&cid, Some(value))).await,
        Err(e) => send(sink, &Envelope::error_frame(&cid, &e)).await,
    };
    match send_result {
        Ok(()) => FrameOutcome::Continue,
        Err(()) => FrameOutcome::Dropped("send query answer: socket closed".to_string()),
    }
}
