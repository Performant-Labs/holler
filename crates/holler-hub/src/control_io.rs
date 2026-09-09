//! The control-socket **server** side (split out of `serve.rs` so that file
//! stays under the 900-line lint gate): newline-delimited JSON-RPC on the hub's
//! internal Unix control socket. [`crate::control`] is the *client* side; this
//! module is the *server* side. ADR 0006.

use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::net::UnixStream;

use holler_proto::{Envelope, WireError};
use holler_proto::Code;

use crate::state::{advertise_path, resolve_state_dir, HubState};
use crate::registry::Registry;
use crate::connection::{Limits, MAX_MESSAGE_BYTES, MAX_PREAUTH_CONNECTIONS, PRE_AUTH_TIMEOUT_MS};
use crate::lockout::{
    DEFAULT_DURATION_MS, DEFAULT_MAX_FAILURES, DEFAULT_WINDOW_MS, LockoutLimits,
};

/// How often the revoke watcher polls the token store. A short interval keeps
/// a revoke's force-close "immediate" (well under a second) without busy-spinning.
const REVOKE_POLL_MS: u64 = 100;

/// Background watcher (story #184): a `hub token revoke`/`delete` runs in a
/// **separate** CLI process that edits the token store's file behind its own
/// flock, so the live hub is never directly told a token was cut. This watcher
/// polls the store (every [`REVOKE_POLL_MS`] ms) and, for each token that is
/// still live in the registry but is now `revoked` in the store (or no longer
/// present), calls [`Registry::remove`] — which closes that socket with
/// **1008**. Revocation thus cuts the socket off at once (the spec: "revocation
/// closes immediately"), and a later reconnect fails `-32002` (the store
/// rejects a revoked credential). Each `Registry` entry's own [`ConnectionHandle`]
/// Drop also drops the socket when the peer goes away, so this watcher only ever
/// closes sockets whose token was revoked; it never removes a token that merely
/// disconnected. The hub's own store writes (join redeem / last_seen) hold the
/// flock only briefly and release it, so this poller's flock never deadlocks.
/// Spawned by `serve_forever`; it parks on a `sleep` and never contends with
/// the accept loop.
pub async fn registry_revoke_watcher(registry: Arc<Registry>, state: HubState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(REVOKE_POLL_MS)).await;
        // Peek the store (its own flock, released on drop). A read error (e.g.
        // a save in flight) just skips this tick.
        let rows = match crate::token::list(&state) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // A token must be force-closed iff it is live in the registry but the
        // store says it is `revoked` (or the row is gone).
        for token_id in registry.live_token_ids() {
            let revoked = rows.iter().any(|r| r.token_id == token_id && matches!(r.state, crate::token::TokenState::Revoked));
            let present = rows.iter().any(|r| r.token_id == token_id);
            if revoked || !present {
                registry.remove(&token_id);
            }
        }
    }
}

/// Handle one control-socket connection: newline-delimited JSON-RPC. Only
/// `control/status` is answered on this story; every other `control/…` method
/// is `-32601 method_not_found` (they land in later stories), and a frame that
/// does not decode is `-32700`/`-32600`.
pub async fn handle_control_conn(stream: UnixStream, registry: Arc<Registry>) {
    use tokio::io::AsyncWriteExt;

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
                let reply = dispatch_control(&line, &registry);
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

/// Parse one control frame, dispatch it, and return the reply as a single line
/// (no trailing newline; the caller adds it). Only `control/status` is
/// implemented on this story.
fn dispatch_control(line: &str, registry: &Arc<Registry>) -> String {
    // The control socket is **internal, non-wire**: it is not validated
    // against the v2 wire catalog (those are the `control/…` methods, which
    // live only here). We still parse the frame as a JSON-RPC object so we can
    // echo the request's id and answer with the right envelope shape.
    let obj: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return unkeyed_error_line(Code::ParseError, "the control frame is not JSON");
        }
    };
    // A batch (array) on the control socket is also a parse-shape error.
    if obj.is_array() {
        return unkeyed_error_line(Code::InvalidRequest, "a batch is not a control frame");
    }
    let id = obj.get("id").and_then(|v| v.as_str()).map(str::to_owned);
    let method = obj.get("method").and_then(|v| v.as_str());

    match method {
        Some("control/status") => {
            let doc = status_doc(registry);
            let cid = resolve_cid(id.as_deref());
            let env = Envelope::response(&cid, Some(doc));
            holler_proto::encode(&env).unwrap_or_default()
        }
        Some(other) => {
            let cid = resolve_cid(id.as_deref());
            let env = Envelope::error_frame(
                &cid,
                &WireError::new(
                    Code::MethodNotFound,
                    format!("unknown control method: {other}"),
                    None,
                ),
            );
            holler_proto::encode(&env).unwrap_or_default()
        }
        // No method: not a call (a stray response/notification or empty frame).
        None => unkeyed_error_line(
            Code::InvalidRequest,
            "a control frame must be a request with a method",
        ),
    }
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
/// `clients`/`clients_detail` are reported from the live registry (#184), and
/// `limits` (the resolved hygiene + lockout tunables, #184 acceptance) reports
/// the values in force so an operator can see the frame cap, pre-auth timeout,
/// pre-auth cap, and lockout window/duration the hub is actually running with.
fn status_doc(registry: &Arc<Registry>) -> serde_json::Value {
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

    // The connection-hygiene and failed-auth-lockout tunables in force (#184
    // acceptance: documented in `hub status --json` `limits{}`). Resolved from
    // the environment (defaults when unset), matching what the live hub
    // enforces — `status` and `serve` share the same process/env here.
    let limits = Limits::resolve();
    let lockout = LockoutLimits::resolve();
    let default_overrides = limits != Limits::default()
        || lockout != LockoutLimits::default();

    serde_json::json!({
        "role": "hub",
        "protocol": 2,
        "version": version,
        "hostname": hostname,
        "listening": listening,
        "advertise": advertise,
        "clients": registry.client_count(),
        "clients_detail": registry.clients_json(),
        "limits": {
            "max_message_bytes": limits.max_message_bytes,
            "pre_auth_timeout_ms": limits.pre_auth_timeout_ms,
            "max_preauth_connections": limits.max_preauth_connections,
            "lockout": {
                "max_failures": lockout.max_failures,
                "window_ms": lockout.window_ms,
                "duration_ms": lockout.duration_ms,
            },
            "defaults": {
                "max_message_bytes": MAX_MESSAGE_BYTES,
                "pre_auth_timeout_ms": PRE_AUTH_TIMEOUT_MS,
                "max_preauth_connections": MAX_PREAUTH_CONNECTIONS,
                "lockout": {
                    "max_failures": DEFAULT_MAX_FAILURES,
                    "window_ms": DEFAULT_WINDOW_MS,
                    "duration_ms": DEFAULT_DURATION_MS,
                },
            },
            "overridden": default_overrides,
        },
        "sessions": 0,
        "harnesses_known": [],
        "harnesses_confirmed": [],
    })
}

/// The bound listen addresses for the live hub, read from the listening event
/// we already emitted on stderr at startup. We re-derive them from the live
/// listeners' state: there is no persisted listener list, so we re-bind is
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
