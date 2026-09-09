//! The body-side `query/*` wire-dispatch leaf `holler_body::connection`'s
//! live loop calls for an inbound `query/status`/`caps`/`support`/`protocol`
//! request, answering from local state. Split out of `connection.rs` (issue
//! #191) once that file's own priority-path/RTT additions pushed it past the
//! workspace's 900-line-per-file guard (`scripts/lint.sh`) — a pure move, no
//! behavior change. (The `session/prompt`/`session/cancel`/`session/answer`
//! dispatch leaves live in `connection/session_dispatch.rs`, issue #151's own
//! split for the same reason.)

use std::path::Path;

use holler_proto::{Code, CorrelationId, Envelope};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message;

use crate::config::SessionConfig;
use crate::connection::{send, FrameOutcome};
use crate::identity::BodyIdentity;

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
