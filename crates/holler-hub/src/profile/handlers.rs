//! The six `profile/*` handlers. Each is #639's `panes::handlers::run` with a closure of
//! its own over the profile store, so the params, their decode, the blocking pool and the
//! reply line are the pane handlers' pipeline (see `panes/handlers.rs`), not a copy of it.
//!
//! The `data` of each method (ADR-0021 §6) is its closure's value: the `Profile` or `null`
//! for `profile/get`, an array of `Profile` for `profile/list` (in slug order, which is not
//! pinned), the stored `Profile` for `profile/cas_put`, `null` for `profile/delete`, a
//! `WatchReply<ProfileEvent>` for `profile/watch`, and an array of `ProfileLogEntry`, oldest
//! first, for `profile/log`.
//!
//! No method takes a log. `ProfileCasPutParams` and the `Profile` inside it refuse an
//! unknown member, so a request that carries a log anywhere is `usage` and writes nothing.
//! An env entry that carries a value (`NAME=value`) is refused by `EnvVarName`'s own decode
//! inside `decode_params` (`profile-secret-refused`), so the value is never echoed and
//! nothing here reads the raw request.

use std::sync::Arc;

use holler_pane::reply::{
    ProfileCasPutParams, ProfileDeleteParams, ProfileGetParams, ProfileLogParams, WatchParams,
};
use holler_proto::CorrelationId;
use serde_json::Value;

use super::store::Store;
use crate::panes::handlers::{run, NoParams};

/// `profile/get`: the record, or `null` when there is none.
pub(crate) async fn get(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: ProfileGetParams| {
        store.get(&params.name)
    })
    .await
}

/// `profile/list`: every record.
pub(crate) async fn list(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |_: NoParams| store.list()).await
}

/// `profile/cas_put`: the stored record.
pub(crate) async fn cas_put(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: ProfileCasPutParams| {
        store.cas_put(&params.profile, params.expected_generation, &params.actor)
    })
    .await
}

/// `profile/delete`: `null`.
pub(crate) async fn delete(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: ProfileDeleteParams| {
        store.delete(&params.name, params.expected_generation, &params.actor)
    })
    .await
}

/// `profile/watch`: one long-poll batch and the cursor to resume from.
pub(crate) async fn watch(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: WatchParams| {
        store.poll(params.since)
    })
    .await
}

/// `profile/log`: the change log, oldest first.
pub(crate) async fn log(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: ProfileLogParams| {
        store.log(&params.name)
    })
    .await
}
