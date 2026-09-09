//! The control-socket **server** side: newline-delimited JSON-RPC handling
//! for the one-shot CLI commands (`hub status`, `hub token ping`, …) that
//! reach the live hub process over the control Unix socket (ADR 0006). The
//! client side lives in [`crate::control`]; the accept loop that spawns
//! [`handle_control_conn`] per connection lives in [`crate::serve`].
//!
//! Moved out of `serve.rs` (issue #182) once that file's authenticated
//! live-session path pushed it toward the 900-line build guard — this module
//! is the whole control-socket **server**, `serve.rs` is the WS accept loop
//! and handshake.

use holler_proto::{Code, Envelope, WireError};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::live::Registry;
use crate::state::{advertise_path, resolve_state_dir, HubState};

/// Handle one control-socket connection: newline-delimited JSON-RPC. A frame
/// that does not decode is `-32700`/`-32600`; an unknown `control/…` method is
/// `-32601 method_not_found`.
pub async fn handle_control_conn(stream: UnixStream, registry: Registry) {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut write_half = write_half;
    let mut lines = tokio::io::BufReader::new(read_half).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    continue;
                }
                let reply = dispatch_control(&line, &registry).await;
                let bytes = format!("{reply}\n");
                if write_half.write_all(bytes.as_bytes()).await.is_err() {
                    return; // client went away.
                }
            }
            Ok(None) => return, // client closed.
            Err(_) => return,
        }
    }
}

/// Parse one control frame, dispatch it, and return the reply as a single
/// line (no trailing newline; the caller adds it).
async fn dispatch_control(line: &str, registry: &Registry) -> String {
    // The control socket is **internal, non-wire**: it is not validated
    // against the v2 wire catalog (those are the `control/…` methods, which
    // live only here). We still parse the frame as a JSON-RPC object so we can
    // echo the request's id and answer with the right envelope shape.
    let obj: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return unkeyed_error_line(Code::ParseError, "the control frame is not JSON"),
    };
    // A batch (array) on the control socket is also a parse-shape error.
    if obj.is_array() {
        return unkeyed_error_line(Code::InvalidRequest, "a batch is not a control frame");
    }
    let id = obj.get("id").and_then(|v| v.as_str()).map(str::to_owned);
    let method = obj.get("method").and_then(|v| v.as_str());
    let cid = resolve_cid(id.as_deref());

    match method {
        Some("control/status") => {
            let doc = status_doc(registry).await;
            encode_response(&cid, doc)
        }
        Some("control/token_ping") => token_ping(&cid, &obj, registry).await,
        Some(other) => encode_error(&cid, Code::MethodNotFound, format!("unknown control method: {other}")),
        // No method: not a call (a stray response/notification or empty frame).
        None => unkeyed_error_line(Code::InvalidRequest, "a control frame must be a request with a method"),
    }
}

/// `control/token_ping {token_id}` (issue #182): find the live circuit bound
/// to `token_id` in the registry and ask it to answer a `circuit/ping`,
/// returning `{hostname, rtt_ms}`. No live socket for that token is
/// `-32004 not_connected` — the spec's exact fail-closed shape for `hub token
/// ping` against a body that is not currently connected.
async fn token_ping(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(token_id) = obj.get("params").and_then(|p| p.get("token_id")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/token_ping needs params.token_id".to_string());
    };
    let Some(handle) = registry.find_by_token(token_id).await else {
        return encode_error(cid, Code::NotConnected, format!("{token_id} has no live connection"));
    };
    let started = std::time::Instant::now();
    match handle.ping(std::time::Duration::from_secs(5)).await {
        Some(ack) => {
            let rtt_ms = started.elapsed().as_millis() as u64;
            encode_response(cid, serde_json::json!({ "hostname": ack.hostname, "rtt_ms": rtt_ms }))
        }
        None => encode_error(cid, Code::NotConnected, format!("{token_id} did not answer the ping")),
    }
}

fn encode_response(cid: &holler_proto::CorrelationId, result: serde_json::Value) -> String {
    let env = Envelope::response(cid, Some(result));
    holler_proto::encode(&env).unwrap_or_default()
}

fn encode_error(cid: &holler_proto::CorrelationId, code: Code, message: String) -> String {
    let env = Envelope::error_frame(cid, &WireError::new(code, message, None));
    holler_proto::encode(&env).unwrap_or_default()
}

/// Build a hub-minted unkeyed error line (`id: null`) carrying `code`.
fn unkeyed_error_line(code: Code, message: &str) -> String {
    let env = Envelope::Error {
        id: None,
        error: WireError::new(code, message, None),
    };
    holler_proto::encode(&env).unwrap_or_default()
}

/// A synthetic hub id for control replies whose request id is missing or
/// unparsable (defensive; the control protocol expects a request id).
// The literal is a well-formed `h-` id, so the parse is infallible.
#[allow(clippy::expect_used)] // #143
fn fallback_id() -> holler_proto::CorrelationId {
    holler_proto::CorrelationId::parse("h-000000000000000000000000")
        .expect("a well-formed synthetic hub id")
}

/// Resolve a request id (a wire string, possibly absent) to a
/// [`CorrelationId`], falling back to a synthetic hub id when it is missing or
/// does not carry the `h-`/`b-` prefix.
fn resolve_cid(id: Option<&str>) -> holler_proto::CorrelationId {
    match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
        Some(cid) => cid,
        None => fallback_id(),
    }
}

/// Build the hub's status document for a `control/status` answer. Per the
/// story spec the doc has `role:"hub"`, a `listening` **array** of bound
/// addresses, an optional `advertise`, `clients` (bodies), `sessions`,
/// `harnesses_known`, `harnesses_confirmed`, `protocol:2`, and `version`.
/// `clients` is now the live registry's size (issue #182) — previously always
/// `0` (no body could authenticate yet); `sessions` stays `0` until a session
/// story lands.
async fn status_doc(registry: &Registry) -> serde_json::Value {
    // Only ever called by the live hub's own control dispatch, where the state
    // dir is always resolvable; `unwrap_or_default` is a defensive no-op.
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    let listening = read_listening(&state);
    let advertise = std::fs::read_to_string(advertise_path(&state))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("advertise")
                .and_then(|a| a.as_str())
                .map(str::to_owned)
        });
    let version = env!("CARGO_PKG_VERSION");
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());

    serde_json::json!({
        "role": "hub",
        "protocol": 2,
        "version": version,
        "hostname": hostname,
        "listening": listening,
        "advertise": advertise,
        "clients": registry.len().await,
        "sessions": 0,
        "harnesses_known": [],
        "harnesses_confirmed": [],
    })
}

/// The bound listen addresses for the live hub, read from the listening event
/// we already emitted on stderr at startup. We re-derive them from the live
/// listeners' state: there is no persisted listener list, so a re-bind is
/// wrong. Instead the hub records its bound addrs in memory; `status_doc` runs
/// on the same process, so we read them from a file the start path writes.
fn read_listening(state: &HubState) -> Vec<String> {
    // The start path writes the bound addresses to `hub/listening.json` so a
    // `control/status` (running in the same process) can report them.
    let path = state.hub_dir.join("listening.json");
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
