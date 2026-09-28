//! `control/status`'s document builder and the bound-listen-address reader:
//! split out of `control_server.rs` (issue #508's pre-agreed W-4 fallback,
//! `docs/handoffs/506-brief.md`'s Clarifications) once that file's own
//! growth — the new `dispatch_allowlisted` entry point the admin loop and
//! the Unix control socket now share — pushed it past the workspace's
//! 900-line build guard (`scripts/lint.sh` check 4). A pure relocation, no
//! behaviour change of its own, following the `control_hold.rs` precedent.

use crate::live::Registry;
use crate::lockout::Lockout;
use crate::state::{advertise_path, resolve_state_dir, HubState};

/// Build the hub's status document for a `control/status` answer. Per the
/// story spec the doc has `role:"hub"`, a `listening` **array** of bound
/// addresses, an optional `advertise`, `clients` (bodies), `sessions`,
/// `harnesses_known`, `harnesses_confirmed`, `protocol` (the running
/// build's `holler_proto::PROTOCOL_VERSION`, issue #340 — not a hardcoded
/// literal, so this doc never lags a version bump), and `version`.
/// `clients`/`sessions`/`harnesses_known`/`harnesses_confirmed` are now the
/// live registry's real counts (issue #182 landed `clients`; issue #185 adds
/// the rest — previously always `0`/`[]`, since no body could yet report a
/// session count or a confirmed harness).
pub(crate) async fn status_doc(registry: &Registry, lockout: &Lockout) -> serde_json::Value {
    // Only ever called by the live hub's own control dispatch, where the state
    // dir is always resolvable; `unwrap_or_default` is a defensive no-op.
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    // Issue #451: the live lockout state. The labels of the token ids it names
    // are looked up here, never on the authentication path, and only when a
    // peer is listed; a store that cannot be read costs the labels, not the status.
    let lockout_now = lockout.snapshot();
    let mut labels = std::collections::HashMap::new();
    if !lockout_now.is_empty() {
        let records = crate::token::list_async(&state).await.unwrap_or_default();
        labels.extend(records.into_iter().map(|r| (r.token_id, r.label)));
    }
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
    // Issue #322: the hub's X25519 public key, so an operator can compare it
    // out-of-band without re-minting a token. `identity::ensure` is
    // idempotent (loads the existing key once `hub serve`/`hub token mint`
    // generated one); `None` only if it cannot even be resolved (an
    // unwritable state dir), which `hub status` should still answer despite.
    let hub_pubkey = crate::identity::ensure(&state).ok().map(|i| i.public_hex());

    serde_json::json!({
        "role": "hub",
        "protocol": holler_proto::PROTOCOL_VERSION,
        "version": version,
        "hostname": hostname,
        "listening": listening,
        "advertise": advertise,
        "hub_pubkey": hub_pubkey,
        "clients": registry.len().await,
        // Issue #184: the per-connection detail (`token_id`/`client_id`/
        // `hostname`/`peer`/`connected_at`) `hub status --json` now also
        // reports. Additive — `clients` itself stays the plain live count
        // the existing test suite already asserts `as_u64()` against.
        "clients_detail": registry.clients_detail().await,
        "sessions": registry.total_sessions().await,
        "harnesses_known": registry.harnesses_known().await,
        "harnesses_confirmed": registry.harnesses_confirmed().await,
        // Issue #184's acceptance: the hygiene/lockout limits documented in
        // `hub status --json`'s `limits{}`.
        "limits": crate::hygiene::HygieneLimits::resolve().to_json(crate::lockout::LockoutLimits::resolve()),
        "lockout": lockout_now.to_json(&labels),
    })
}

/// The bound listen addresses for the live hub, read from the listening event
/// we already emitted on stderr at startup. We re-derive them from the live
/// listeners' state: there is no persisted listener list, so a re-bind is
/// wrong. Instead the hub records its bound addrs in memory; `status_doc` runs
/// on the same process, so we read them from a file the start path writes.
pub(crate) fn read_listening(state: &HubState) -> Vec<String> {
    // The start path writes the bound addresses to `hub/listening.json` so a
    // `control/status` (running in the same process) can report them.
    let path = state.hub_dir.join("listening.json");
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
