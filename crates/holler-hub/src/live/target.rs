//! A body session's harness id as an address, and the rebind's own forward (#839, ADR-0021
//! section 11 points 1 and 2).
//!
//! - [`Registry::resolve_prompt_target`] resolves a `say`, `interrupt` or `answer` target: a
//!   valid name (ADR 0005) by name, as before, and any other string as the `harness_session_id`
//!   a live attach-mode session advertises. The `holler` CLI sends an id only for `--pane`.
//! - [`LiveHandle::rebind`] asks a body to re-point one of its sessions (`session/rebind`). It
//!   has its own forward, never `LiveHandle::query`'s, whose allowlist stays `query/*` and
//!   `session/answer`: `control/query_remote` and `admin/query` hand any method to `query`, so
//!   an allowlisted rebind would reach a body without `control/rebind`'s cache and roster update.

use std::time::Duration;

use holler_proto::{Mode, SessionAd, WireError};
use serde_json::Value;

use super::{LiveHandle, Registry, ResolveOutcome};

impl Registry {
    /// Resolve a prompt target (#839): a target that parses as a routable name is resolved by
    /// name ([`Registry::resolve_session`], exactly as before); any other string is matched
    /// against the live attach-mode sessions by the `harness_session_id` each advertises. No
    /// match is `Unknown`, or `NotConnected` when a body that is now offline last advertised it;
    /// two are `Ambiguous`. A name is never an id: the name grammar refuses `_`, which every
    /// OpenCode id has, and an id that is also a valid name is resolved as a name.
    pub async fn resolve_prompt_target(&self, target: &str) -> ResolveOutcome {
        if let Ok(name) = holler_proto::RoutableName::parse(target) {
            return self.resolve_session(&name).await;
        }
        let inner = self.inner.lock().await;
        let mut matches: Vec<(LiveHandle, SessionAd)> = Vec::new();
        for handle in inner.values() {
            let Some(sessions) = handle.presence.lock().await.clone() else { continue };
            let advertising = sessions.into_iter().filter(|ad| advertises(ad, target));
            matches.extend(advertising.map(|ad| (handle.clone(), ad)));
        }
        drop(inner);
        if matches.len() > 1 {
            let candidates = matches.into_iter().map(|(h, ad)| format!("{}/{}", h.hostname, ad.name)).collect();
            return ResolveOutcome::Ambiguous(candidates);
        }
        match matches.pop() {
            Some((handle, ad)) => ResolveOutcome::Found(handle, Box::new(ad)),
            None if self.offline_advertises(target).await => ResolveOutcome::NotConnected,
            None => ResolveOutcome::Unknown,
        }
    }

    /// `true` iff a now-disconnected body's last-known presence advertised `id` on an attach
    /// session (the `not_connected` case, as `matches_offline` is for a name).
    async fn offline_advertises(&self, id: &str) -> bool {
        self.offline.lock().await.values().any(|(_, sessions)| sessions.iter().any(|ad| advertises(ad, id)))
    }
}

/// Whether `ad` is an attach-mode session advertising the harness session `id`. A spawn
/// session's row is never matched by an id.
fn advertises(ad: &SessionAd, id: &str) -> bool {
    ad.mode == Mode::Attach && ad.harness_session_id.as_deref() == Some(id)
}

impl LiveHandle {
    /// Ask this body to re-point one of its attach sessions (`params` is `session/rebind`'s
    /// `{session, harness_session_id}`) and await its answer, bounded by `timeout`, the wait for
    /// the connection's one reply slot included. `None` when no answer came in time: a rebind
    /// that waited behind a slow forward may be left with less than the body's own 5 s, and the
    /// body may then still re-point after the caller gave up (`pane doctor` reports such a body,
    /// and `pane switch` re-points it). The inner `Result` is the body's answer, a refusal
    /// carried verbatim.
    pub async fn rebind(&self, params: Value, timeout: Duration) -> Option<Result<Value, WireError>> {
        self.forward("session/rebind", Some(params), timeout).await
    }
}
