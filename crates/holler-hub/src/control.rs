//! The control-socket **client** side: the one-shot CLI commands
//! (`holler hub status` here; `roster`/`say`/… in later stories) reach the
//! live hub process over the control Unix socket rather than the WebSocket
//! circuit (ADR 0006). This module is the client; the server side lives in
//! [`crate::serve`].

use std::io::{BufRead, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use holler_proto::CorrelationId;

use crate::state::{control_sock_path, resolve_state_dir, HubState};

/// The timeout for a one-shot control exchange (the spec: 5 s).
const CLIENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Why a control exchange could not reach the live hub.
#[derive(Debug)]
pub enum ControlError {
    /// No socket at the expected path (the hub is not running, or the state
    /// dir is wrong) — the spec's `no live holler hub reachable at <dir>`.
    NoLiveHub,
    /// The socket was present but the exchange failed (I/O or a bad reply).
    Io(std::io::Error),
    /// The reply did not parse as a v2 envelope.
    BadReply(String),
    /// The hub answered with a JSON-RPC error (issue #182: `control/
    /// token_ping`'s `-32004 not_connected` is reported this way).
    Refused(holler_proto::WireError),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::NoLiveHub => write!(f, "no live holler hub reachable"),
            ControlError::Io(e) => write!(f, "control socket I/O error: {e}"),
            ControlError::BadReply(s) => write!(f, "bad reply from the hub: {s}"),
            ControlError::Refused(e) => write!(f, "{}", e.message),
        }
    }
}

/// Resolve the control socket path for the current state dir.
pub fn sock_path() -> PathBuf {
    // A missing state dir has no socket; `unwrap_or_default` is a defensive
    // no-op (this helper is only used where the state dir is resolvable).
    control_sock_path(&HubState::from_root(
        resolve_state_dir().unwrap_or_default(),
    ))
}

/// Ask the live hub for its status document over the control socket.
///
/// Returns the reply envelope's `result` (the `StatusDoc` JSON value). Errors
/// map to [`ControlError::NoLiveHub`] when the socket is absent (hub not
/// running) — the CLI prints the spec's exact message and exits 1.
pub fn status() -> Result<serde_json::Value, ControlError> {
    exchange("b-status", "control/status", None)
}

/// `hub token ping ID` (issue #182): ask the live hub to send a `circuit/
/// ping` over the token's live socket (if any) and report `{hostname,
/// rtt_ms}`. No live socket for that token surfaces as
/// [`ControlError::Refused`] carrying the hub's `-32004 not_connected`.
pub fn token_ping(token_id: &str) -> Result<serde_json::Value, ControlError> {
    let params = serde_json::json!({ "token_id": token_id });
    exchange("b-token-ping", "control/token_ping", Some(params))
}

/// `hub caps` (issue #185): the live hub's `query/caps` document.
pub fn caps() -> Result<serde_json::Value, ControlError> {
    exchange("b-caps", "control/caps", None)
}

/// `hub support FEATURE` (issue #185): a single `query/support` answer.
pub fn support(feature: &str) -> Result<serde_json::Value, ControlError> {
    let params = serde_json::json!({ "feature": feature });
    exchange("b-support", "control/support", Some(params))
}

/// `hub query CMD [ARGS...]` (issue #185, local form): answer `method`
/// (`query/status`|`caps`|`support`|`protocol`) from the live hub's own
/// state, with `params` as that method's own params (e.g.
/// `query/support`'s `{feature}`).
pub fn query_local(method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value, ControlError> {
    let outer = serde_json::json!({ "method": method, "params": params });
    exchange("b-query-local", "control/query_local", Some(outer))
}

/// `hub query TARGET CMD [ARGS...]` (issue #185, remote form): forward
/// `method`/`params` to the live body resolved from `target` (token id,
/// client id, or hostname/label). No live match is [`ControlError::Refused`]
/// carrying `-32004 not_connected`; more than one match is a `-32600`-coded
/// refusal whose message names the ambiguity (the CLI maps that to exit 2).
pub fn query_remote(
    target: &str,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, ControlError> {
    let outer = serde_json::json!({ "target": target, "method": method, "params": params });
    exchange("b-query-remote", "control/query_remote", Some(outer))
}

/// `say SESSION TEXT` (issue #190): ask the live hub to resolve `session`,
/// run one `session/prompt` turn, and report `{session, stop_reason, state,
/// updates, elapsed_ms, text, message}`. `timeout` is the caller's own
/// `--timeout` (default 600s) — this call waits up to `timeout` **plus** a
/// small fixed margin for the control-socket round trip itself, since the
/// hub's own exchange already applies `timeout` to the live socket wait.
pub fn say(session: &str, text: &str, queue: bool, timeout: std::time::Duration) -> Result<serde_json::Value, ControlError> {
    let params = serde_json::json!({
        "session": session,
        "text": text,
        "queue": queue,
        "timeout_ms": u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
    });
    exchange_with_timeout("b-say", "control/say", Some(params), timeout + std::time::Duration::from_secs(5))
}

/// Send one `method`/`params` request over the control socket and return its
/// `result` — the shared body of every one-shot control exchange (`status`,
/// `token_ping`, …). `id_literal` is a fixed, well-formed `b-` id (each
/// caller's own; the control socket does not correlate concurrent calls, so a
/// literal per call site is enough).
fn exchange(id_literal: &str, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value, ControlError> {
    exchange_with_timeout(id_literal, method, params, CLIENT_TIMEOUT)
}

/// [`exchange`] with a caller-chosen read timeout (issue #190: `say` waits
/// far longer than the 5s default one-shot exchanges use).
// The `id_literal` callers pass are always well-formed `b-` ids, and the
// request envelope encodes infallibly, so the `.expect`s here are unreachable.
#[allow(clippy::expect_used)] // #143
fn exchange_with_timeout(
    id_literal: &str,
    method: &str,
    params: Option<serde_json::Value>,
    timeout: std::time::Duration,
) -> Result<serde_json::Value, ControlError> {
    // No resolvable state dir means there is no control socket to connect to —
    // the same "no live hub" condition as an absent socket.
    let path = match resolve_state_dir() {
        Some(dir) => control_sock_path(&HubState::from_root(dir)),
        None => return Err(ControlError::NoLiveHub),
    };
    let stream = UnixStream::connect(&path).map_err(|_| ControlError::NoLiveHub)?;
    stream.set_read_timeout(Some(timeout)).map_err(ControlError::Io)?;

    let cid = CorrelationId::parse(id_literal).expect("a well-formed literal control id");
    let req = holler_proto::Envelope::request(&cid, method, params);
    let bytes = format!("{}\n", holler_proto::encode(&req).expect("encode the control request"));

    let mut stream = stream;
    stream.write_all(bytes.as_bytes()).map_err(ControlError::Io)?;
    stream.flush().map_err(ControlError::Io)?;

    let mut line = String::new();
    let n = std::io::BufReader::new(stream)
        .read_line(&mut line)
        .map_err(ControlError::Io)?;
    if n == 0 {
        return Err(ControlError::BadReply("the hub closed the socket before replying".into()));
    }

    let env = holler_proto::decode(&line).map_err(|e| ControlError::BadReply(e.to_string()))?;
    if let Some(error) = env.error() {
        return Err(ControlError::Refused(error.clone()));
    }
    env.result()
        .cloned()
        .ok_or_else(|| ControlError::BadReply("the hub reply carried no result".into()))
}
