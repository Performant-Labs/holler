//! The five `pane/*` handlers, and the one pipeline they share.
//!
//! Each handler is [`run`] with a closure of its own. `run` goes through these steps:
//!
//! 1. It takes the request's `params`, reading an absent member or `null` as `{}`.
//! 2. It decodes them with `holler_pane::decode_params`. A guard's own code survives the
//!    decode (a shell string is `command-not-argv`), and anything else that does not
//!    decode is `usage`.
//! 3. It runs the closure on the blocking pool (`spawn_blocking`), because the store is
//!    synchronous: it waits on its lock, on its file, and, for `pane/watch`, on the
//!    long-poll window. A task that does not finish (it panicked) fails closed as
//!    `unavailable`.
//! 4. It answers a `PaneReply` (the closure's data, or its error) as the reply line
//!    (`crate::pane_dispatch::reply_line`).
//!
//! #661's `profile/*` handlers call [`run`] too, from `profile/`.
//!
//! The `data` of each method (D7) is its closure's value: the `Pane` or `null` for
//! `pane/get`, an array of `Pane` sorted by name for `pane/list`, the stored `Pane` for
//! `pane/cas_put`, `null` for `pane/delete`, and a `WatchReply<PaneEvent>` for
//! `pane/watch`.

use std::sync::Arc;

use holler_pane::reply::{PaneCasPutParams, PaneDeleteParams, PaneGetParams, WatchParams};
use holler_pane::{decode_params, PaneError, PaneReply};
use holler_proto::CorrelationId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::task::JoinError;

use super::store::Store;
use crate::pane_dispatch::reply_line;
use crate::profile::{check_membership, ProfileState};

/// The params of a method that takes none (`pane/list`): only `{}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoParams {}

/// Answer one request: decode its params into `P`, run `work` on them on the blocking
/// pool, and turn the outcome into the reply line (see the module docs).
pub(crate) async fn run<P, T, F>(cid: &CorrelationId, obj: &Value, work: F) -> String
where
    P: DeserializeOwned + Send + 'static,
    T: Serialize,
    F: FnOnce(P) -> Result<T, PaneError> + Send + 'static,
{
    let reply = match decode_params::<P>(params_of(obj)) {
        Ok(params) => {
            let task = move || work(params).and_then(|data| to_data(&data));
            answer(tokio::task::spawn_blocking(task).await)
        }
        Err(err) => PaneReply::failure(&err),
    };
    reply_line(cid, &reply)
}

/// The request's `params`, with an absent member or `null` read as `{}` (see
/// `holler_pane::decode_params`).
fn params_of(obj: &Value) -> Value {
    match obj.get("params") {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(params) => params.clone(),
    }
}

/// A closure's value as the `data` of the reply.
fn to_data<T: Serialize>(data: &T) -> Result<Value, PaneError> {
    serde_json::to_value(data).map_err(|_| PaneError::Unavailable {
        what: "the reply's data could not be encoded".to_owned(),
    })
}

/// The reply for what the blocking task returned.
fn answer(joined: Result<Result<Value, PaneError>, JoinError>) -> PaneReply {
    match joined {
        Ok(Ok(data)) => PaneReply::success(data),
        Ok(Err(err)) => PaneReply::failure(&err),
        Err(join) => PaneReply::failure(&PaneError::Unavailable {
            what: format!("the registry task did not finish ({join})"),
        }),
    }
}

/// `pane/get`: the record, or `null` when there is none.
pub(crate) async fn get(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: PaneGetParams| {
        store.get(&params.name)
    })
    .await
}

/// `pane/list`: every record, sorted by name.
pub(crate) async fn list(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |_: NoParams| store.list()).await
}

/// `pane/cas_put`: the stored record. The membership check runs first, on the blocking
/// pool and outside the pane lock (D1, D9). It runs off the executor because #661's check
/// reads the profile registry through its blocking port. A refusal is the reply, and
/// nothing is written.
pub(crate) async fn cas_put(
    cid: &CorrelationId,
    obj: &Value,
    store: Arc<Store>,
    profiles: Arc<ProfileState>,
) -> String {
    run(cid, obj, move |params: PaneCasPutParams| {
        check_membership(&params.pane, &profiles)?;
        store.cas_put(&params.pane, params.expected_generation)
    })
    .await
}

/// `pane/delete`: `null`.
pub(crate) async fn delete(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: PaneDeleteParams| {
        store.delete(&params.name, params.expected_generation)
    })
    .await
}

/// `pane/watch`: one long-poll batch and the cursor to resume from.
pub(crate) async fn watch(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
    run(cid, obj, move |params: WatchParams| {
        store.poll(params.since)
    })
    .await
}
