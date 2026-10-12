//! `profile/rename` (#665): the one hub-side transaction that renames a profile and moves
//! every member pane's `Pane.profile` to the new name (ADR-0021 section 8, "Rename as built").
//!
//! The handler is #639's `panes::handlers::run` with a closure, as every other `profile/*`
//! method is (`handlers.rs`): the params decode as `ProfileRenameParams`, a request that does
//! not decode is `usage`, and the closure runs on the blocking pool. The transaction is
//! `Store::rename` given the pane registry, which takes the profile lock and then the pane lock
//! and saves both documents, the panes' first (`store.rs`, "Renames"). The reply's `data` is the
//! renamed record (ADR-0021 section 6).

use std::sync::Arc;

use holler_pane::reply::ProfileRenameParams;
use holler_proto::CorrelationId;
use serde_json::Value;

use super::ProfileState;
use crate::panes::handlers::run;
use crate::panes::PaneState;

/// Answer `profile/rename`: the renamed record. `panes` is the pane registry whose member
/// panes the transaction moves; the closure owns a handle on each registry, since the
/// blocking pool needs `'static` work.
pub async fn dispatch(
    cid: &CorrelationId,
    obj: &Value,
    profiles: &ProfileState,
    panes: &Arc<PaneState>,
) -> String {
    let store = Arc::clone(&profiles.store);
    let panes = Arc::clone(panes);
    run(cid, obj, move |params: ProfileRenameParams| {
        store.rename(
            &params.from,
            &params.to,
            params.expected_generation,
            &params.actor,
            Some(&panes),
        )
    })
    .await
}
