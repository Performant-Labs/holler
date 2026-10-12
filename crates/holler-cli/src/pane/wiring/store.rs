//! The registries' clients (#649, D3, D4): [`HubPaneStore`] and [`HubProfileStore`] implement
//! `PaneStore` and `ProfileStore` by the hub's `pane/*` and `profile/*` control methods (E9), one
//! request per call. They send through `crate::transport::call(None, ..)`, the CLI's one dispatch
//! point (MO 8), which runs the exchange over the local control socket of the state dir that
//! `HOLLER_STATE_DIR` names when the call is made. So a client holds nothing: building one does
//! no I/O, and every call goes to the hub of that moment.
//!
//! **Requests and replies.** The params are `holler_pane::reply`'s structs, which the hub decodes
//! (E12, E13); the reply is a `PaneReply` (E10), whose error comes back as its `PaneError`
//! (`into_result`), and whose `data` decodes as ADR-0021 section 6 gives it (E14, E15). A reply or a
//! `data` that does not decode is `unavailable` naming the method, never `usage`, which is for the
//! caller's own input (W-9).
//!
//! **Bounds.** Every request carries a 10 s `ControlCall.timeout` (I5, D9). A `*/watch` request
//! carries the hub's long-poll window `WATCH_WAIT` plus the control precedent's 5 s (E57), so an
//! idle window is never mistaken for a wedged hub. `reach_hub`, `connect`'s exchange, keeps
//! `ControlCall::status()`'s own 5 s (E60).
//!
//! **A failed exchange (D3, `call`).** One `ControlError` (E59) becomes one `PaneError`, by
//! whether the method reads or writes:
//!
//! | The exchange | A read (`get`, `list`, `log`, each `watch` request) | A write (`cas_put`, `delete`, `rename`) |
//! |---|---|---|
//! | no hub (`NoLiveHub`) | `unavailable`: `no live holler hub reachable at <state dir>` | the same: the request never left |
//! | no reply within the bound (`Io`, `WouldBlock` or `TimedOut`) | `timeout`, naming the store method | `timeout` |
//! | any other `Io`, or `BadReply` (a lost reply) | `unavailable`, naming the hub | `timeout`: it may have landed (E40) |
//! | a JSON-RPC error (`Refused`), or a remote variant | `unavailable`, naming the hub | `unavailable`: nothing was written |
//!
//! The `op` of a `timeout` is the test kit's name for the store method (`pane_store.cas_put`,
//! `profile_store.watch_next`), as in every other implementation and fake (ADR-0021 section 9).
//! `send_over` cannot tell a write that failed to leave from one whose reply was lost, so a write
//! that fails in flight is always the `timeout` that says it may have landed.
//!
//! **Watch (D4).** `watch(since)` makes its first request at once and keeps the reply for the
//! first `next()`, so a refusal of the cursor (a cursor ahead of the head is the hub's `usage`)
//! and a missing hub are `watch()`'s own answer, before any item: `pane watch` promises that every
//! refusal comes before its first line. Each later batch is one request, resumed from the
//! cursor of the reply before it (E16). A batch yields its events in order, and a batch with
//! none is the idle item `Ok(None)`. An error is one `Err`, and then the stream ends.
//!
//! `profile/rename` (#665) is the hub's one transaction that renames a profile and moves its member
//! panes; the client sends it like any other write.

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Duration;

use holler_hub::control::{ControlCall, ControlError};
use holler_hub::panes::WATCH_WAIT;
use holler_pane::reply::{
    PaneCasPutParams, PaneDeleteParams, PaneGetParams, PaneReply, ProfileCasPutParams,
    ProfileDeleteParams, ProfileGetParams, ProfileLogParams, ProfileRenameParams, WatchParams,
    WatchReply,
};
use holler_pane::{
    Actor, Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Profile, ProfileEvent,
    ProfileLogEntry, ProfileName, ProfileStore, Watch,
};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::Serialize;
use serde_json::{json, Value};

/// The bound of one store request (I5).
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// The bound of one `*/watch` request: the hub's long-poll window and the control precedent's
/// margin (E57).
const WATCH_TIMEOUT: Duration = WATCH_WAIT.saturating_add(Duration::from_secs(5));

const PANE_GET: Method = Method::read("pane/get", "pane_store.get");
const PANE_LIST: Method = Method::read("pane/list", "pane_store.list");
const PANE_CAS_PUT: Method = Method::write("pane/cas_put", "pane_store.cas_put");
const PANE_DELETE: Method = Method::write("pane/delete", "pane_store.delete");
const PANE_WATCH: Method = Method::long_poll("pane/watch", "pane_store.watch_next");
const PROFILE_GET: Method = Method::read("profile/get", "profile_store.get");
const PROFILE_LIST: Method = Method::read("profile/list", "profile_store.list");
const PROFILE_CAS_PUT: Method = Method::write("profile/cas_put", "profile_store.cas_put");
const PROFILE_DELETE: Method = Method::write("profile/delete", "profile_store.delete");
const PROFILE_WATCH: Method = Method::long_poll("profile/watch", "profile_store.watch_next");
const PROFILE_LOG: Method = Method::read("profile/log", "profile_store.log");
const PROFILE_RENAME: Method = Method::write("profile/rename", "profile_store.rename");

/// The pane registry's client: `PaneStore` over the hub's `pane/*` methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct HubPaneStore;

impl HubPaneStore {
    /// A client of the hub that `HOLLER_STATE_DIR` names when each call is made. No I/O.
    pub fn new() -> Self {
        Self
    }
}

/// The profile registry's client: `ProfileStore` over the hub's `profile/*` methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct HubProfileStore;

impl HubProfileStore {
    /// A client of the hub that `HOLLER_STATE_DIR` names when each call is made. No I/O.
    pub fn new() -> Self {
        Self
    }
}

/// The two clients a wiring shares: the verbs, the profile scope and the `tui_session` lookup
/// all read the registries through them.
pub fn stores() -> (Arc<dyn PaneStore>, Arc<dyn ProfileStore>) {
    (
        Arc::new(HubPaneStore::new()),
        Arc::new(HubProfileStore::new()),
    )
}

/// `connect`'s one exchange (D2): `control/status`, within the control client's own 5 s (E60).
/// Every failure, a hub that does not answer in time included, is `unavailable` naming the hub:
/// a verb can do nothing without it.
pub(crate) fn reach_hub() -> Result<(), PaneError> {
    let status = ControlCall::status();
    crate::transport::call(None, &status)
        .map(drop)
        .map_err(|error| hub_unavailable(status.method, &error))
}

impl PaneStore for HubPaneStore {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        PANE_GET.request(&PaneGetParams { name: name.clone() })
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        PANE_LIST.request(&json!({}))
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        PANE_CAS_PUT.request(&PaneCasPutParams {
            pane: pane.clone(),
            expected_generation,
        })
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        let params = PaneDeleteParams {
            name: name.clone(),
            expected_generation,
        };
        PANE_DELETE.request::<IgnoredAny>(&params).map(drop)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        PANE_WATCH.watch(since)
    }
}

impl ProfileStore for HubProfileStore {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        PROFILE_GET.request(&ProfileGetParams { name: name.clone() })
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        PROFILE_LIST.request(&json!({}))
    }

    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        PROFILE_CAS_PUT.request(&ProfileCasPutParams {
            profile: profile.clone(),
            expected_generation,
            actor: actor.clone(),
        })
    }

    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError> {
        let params = ProfileDeleteParams {
            name: name.clone(),
            expected_generation,
            actor: actor.clone(),
        };
        PROFILE_DELETE.request::<IgnoredAny>(&params).map(drop)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        PROFILE_WATCH.watch(since)
    }

    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        PROFILE_LOG.request(&ProfileLogParams { name: name.clone() })
    }

    fn rename(
        &self,
        from: &ProfileName,
        to: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        PROFILE_RENAME.request(&ProfileRenameParams {
            from: from.clone(),
            to: to.clone(),
            expected_generation,
            actor: actor.clone(),
        })
    }
}

/// What a request does to its registry, which decides what a lost reply means (D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    /// It reads: a request that failed changed nothing.
    Read,
    /// It writes: a request whose reply was lost may have landed.
    Write,
}

/// One store method: its wire name (E9), the `op` its `timeout` names, what it does, and the
/// bound of one request.
#[derive(Debug, Clone, Copy)]
struct Method {
    name: &'static str,
    op: &'static str,
    effect: Effect,
    timeout: Duration,
}

impl Method {
    /// A method that reads, within the store's bound.
    const fn read(name: &'static str, op: &'static str) -> Self {
        Self {
            name,
            op,
            effect: Effect::Read,
            timeout: CALL_TIMEOUT,
        }
    }

    /// A method that writes, within the store's bound.
    const fn write(name: &'static str, op: &'static str) -> Self {
        Self {
            name,
            op,
            effect: Effect::Write,
            timeout: CALL_TIMEOUT,
        }
    }

    /// A `*/watch` method: it reads, within the long-poll's own bound.
    const fn long_poll(name: &'static str, op: &'static str) -> Self {
        Self {
            name,
            op,
            effect: Effect::Read,
            timeout: WATCH_TIMEOUT,
        }
    }

    /// One request of this method with `params`, and its reply's `data` decoded as `T`.
    fn request<T: DeserializeOwned>(self, params: &impl Serialize) -> Result<T, PaneError> {
        let params = serde_json::to_value(params).map_err(|error| PaneError::Unavailable {
            what: format!("{}: the request does not encode: {error}", self.name),
        })?;
        let data = call(self.name, params, self.timeout, self.effect, self.op)?;
        serde_json::from_value(data.unwrap_or(Value::Null))
            .map_err(|error| undecodable(self.name, &error))
    }

    /// The watch of this `*/watch` method from `since`, its first request made now (D4).
    fn watch<E>(self, since: Cursor) -> Result<Watch<E>, PaneError>
    where
        E: DeserializeOwned + Send + 'static,
    {
        let mut feed = Feed {
            method: self,
            since,
            batch: None,
            ended: false,
        };
        feed.batch = Some(feed.fetch()?);
        Ok(Box::new(feed))
    }
}

/// A `Watch` over a `*/watch` method (D4): one long-poll request per batch.
struct Feed<E> {
    method: Method,
    /// Where the next request resumes: the cursor of the last reply.
    since: Cursor,
    /// What is left of the last reply's batch, or `None` when the next item needs a request. A
    /// batch with no events stands for one idle item.
    batch: Option<VecDeque<E>>,
    /// An error ended the stream.
    ended: bool,
}

impl<E: DeserializeOwned> Feed<E> {
    /// One request from `since`: its batch, and its cursor as the next request's `since`.
    fn fetch(&mut self) -> Result<VecDeque<E>, PaneError> {
        let reply: WatchReply<E> = self.method.request(&WatchParams { since: self.since })?;
        self.since = reply.cursor;
        Ok(reply.events.into())
    }
}

impl<E: DeserializeOwned> Iterator for Feed<E> {
    type Item = Result<Option<E>, PaneError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let mut batch = match self.batch.take() {
            Some(batch) => batch,
            None => match self.fetch() {
                Ok(batch) => batch,
                Err(error) => {
                    self.ended = true;
                    return Some(Err(error));
                }
            },
        };
        let Some(event) = batch.pop_front() else {
            return Some(Ok(None));
        };
        if !batch.is_empty() {
            self.batch = Some(batch);
        }
        Some(Ok(Some(event)))
    }
}

/// Send `method` with `params` to the hub, within `timeout`, through the CLI's one dispatch
/// point, and answer the reply's `data` or its error. The one mapping of a failed exchange (D3,
/// the module docs' table): `effect` says whether a lost reply may have landed, and `op` is what
/// a `timeout` names.
fn call(
    method: &'static str,
    params: Value,
    timeout: Duration,
    effect: Effect,
    op: &'static str,
) -> Result<Option<Value>, PaneError> {
    let request = ControlCall {
        method,
        params: Some(params),
        timeout,
    };
    let result = crate::transport::call(None, &request).map_err(|error| {
        if timed_out(&error, effect) {
            PaneError::Timeout { op: op.to_owned() }
        } else {
            hub_unavailable(method, &error)
        }
    })?;
    let reply: PaneReply =
        serde_json::from_value(result).map_err(|error| undecodable(method, &error))?;
    reply.into_result()
}

/// Whether a failed exchange is a `timeout` (D3): no reply within the bound, for any method, and
/// for a write also a reply lost in flight, since the write may have landed.
fn timed_out(error: &ControlError, effect: Effect) -> bool {
    match (error, effect) {
        (ControlError::Io(io), _)
            if matches!(io.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
        {
            true
        }
        (ControlError::Io(_) | ControlError::BadReply(_), Effect::Write) => true,
        _ => false,
    }
}

/// `unavailable`, naming the hub: `no live holler hub reachable at <state dir>`, the CLI's own
/// text, when there is none, else which hub did not answer `method`, and how.
fn hub_unavailable(method: &str, error: &ControlError) -> PaneError {
    let state = holler_hub::state::resolve_state_dir().unwrap_or_default();
    let state = state.display();
    let what = match error {
        ControlError::NoLiveHub => format!("no live holler hub reachable at {state}"),
        other => format!("the holler hub at {state} did not answer {method}: {other}"),
    };
    PaneError::Unavailable { what }
}

/// `unavailable` naming `method`: its reply, or the reply's `data`, does not decode (W-9).
fn undecodable(method: &str, error: &serde_json::Error) -> PaneError {
    PaneError::Unavailable {
        what: format!("{method}: the hub's reply does not decode: {error}"),
    }
}
