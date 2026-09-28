//! One dispatch point for every hub-only verb (issue #508/#509, epic #506,
//! MO 8): [`call`] sends a [`ControlCall`] over the local Unix control
//! socket exactly as every leaf did before this story when no `--server`
//! was given, or — with one — rewrites its `control/x` method to `admin/x`
//! and sends it over [`holler_body::admin_client`] instead.
//!
//! `holler-hub` must not depend on `holler-body` (ADR 0001's role split);
//! this crate is the one place already allowed to depend on both (`main.rs`
//! calls into each directly), so the local/remote glue lives here rather
//! than in either crate. The remote path is a short-lived, one-shot dial —
//! the CLI itself is not a tokio runtime, so it builds a throwaway one and
//! blocks on it, the same pattern `holler_body::join::join` already uses.

use holler_hub::control::{ControlCall, ControlError};

/// Run `call` locally (over the Unix control socket, as every leaf did
/// before this story) when `server` is `None`, or against the named remote
/// hub (over `holler_body::admin_client`, rewriting `call.method`'s
/// `control/x` to `admin/x`) when it is `Some`.
///
/// A `--server` that fails the loopback policy check (ADR 0002, the same
/// fail-closed rule `body join` applies) is refused *before any dial* as
/// [`ControlError::RemotePolicyRefused`], which every `*_cmd.rs` maps to
/// exit 3 (AC 13). Every other remote failure — not joined, an unreachable
/// hub, a hub-key mismatch, … — is [`ControlError::RemoteUnavailable`]
/// (MO 9), which the existing catch-all refusal arm in every `*_cmd.rs`
/// already renders as exit 1 with the plain message, unchanged. A real
/// hub-side JSON-RPC error (an unknown session, `session_busy`, an
/// ambiguous target, …) still comes back as the ordinary
/// [`ControlError::Refused`] every existing rendering already handles —
/// this is what keeps the local and the remote path sharing one rendering
/// (MO 9's whole point).
pub fn call(server: Option<&str>, call: &ControlCall) -> Result<serde_json::Value, ControlError> {
    match server {
        None => holler_hub::control::run(call),
        Some(server) => remote(server, call),
    }
}

fn remote(server: &str, call: &ControlCall) -> Result<serde_json::Value, ControlError> {
    let addr = holler_body::server_address::parse(server)
        .map_err(|e| ControlError::RemotePolicyRefused(format!("bad --server address: {e}")))?;
    if let Some(reason) = holler_body::server_address::loopback_only_check(&addr) {
        return Err(ControlError::RemotePolicyRefused(reason.to_string()));
    }

    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    let method = admin_method(call.method);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ControlError::RemoteUnavailable(format!("could not start the local runtime: {e}")))?;
    rt.block_on(holler_body::admin_client::call(&state_root, server, &method, call.params.clone(), call.timeout))
        .map_err(map_admin_error)
}

/// `control/x` → `admin/x` (MO 8's whole transport switch). The two local
/// query forms collapse onto the one wire method `admin/query` (MO 2): the
/// hub's admin loop tells them apart by `params.target`'s presence, which
/// `ControlCall::query_local`/`query_remote` already set (or omit)
/// correctly — the rewrite only ever touches the method name.
fn admin_method(control_method: &str) -> String {
    match control_method {
        "control/query_local" | "control/query_remote" => "admin/query".to_string(),
        other => format!("admin/{}", other.trim_start_matches("control/")),
    }
}

/// Map an [`holler_body::admin_client::AdminClientError`] onto the shared
/// [`ControlError`] (MO 9): a real hub-side JSON-RPC error folds into
/// [`ControlError::Refused`] unchanged; everything else (not joined, an
/// unreachable hub, a hub-key mismatch, a dropped socket, …) becomes
/// [`ControlError::RemoteUnavailable`] with its own plain message.
fn map_admin_error(e: holler_body::admin_client::AdminClientError) -> ControlError {
    match e {
        holler_body::admin_client::AdminClientError::Wire(err) => ControlError::Refused(err),
        other => ControlError::RemoteUnavailable(other.to_string()),
    }
}
