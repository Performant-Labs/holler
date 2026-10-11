//! TRIAL (T's satisfiability check only, never committed).
#![allow(clippy::unwrap_used, clippy::expect_used)] // #649
use std::sync::Arc;
use std::time::Duration;
use holler_hub::control::{ControlCall, ControlError};
use holler_pane::reply::*;
use holler_pane::{Actor, Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileStore, Watch};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

const BOUND: Duration = Duration::from_secs(10);

fn state_dir() -> String {
    holler_hub::state::resolve_state_dir().map(|d| d.display().to_string()).unwrap_or_default()
}

pub fn call(method: &'static str, params: Value, timeout: Duration, write: bool, op: &str) -> Result<Option<Value>, PaneError> {
    let c = ControlCall { method, params: Some(params), timeout };
    match crate::transport::call(None, &c) {
        Ok(v) => serde_json::from_value::<PaneReply>(v)
            .map_err(|e| PaneError::Unavailable { what: format!("{method}: {e}") })?
            .into_result(),
        Err(ControlError::NoLiveHub) => Err(PaneError::Unavailable { what: format!("no live holler hub reachable at {}", state_dir()) }),
        Err(ControlError::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => Err(PaneError::Timeout { op: op.to_owned() }),
        Err(ControlError::Io(_) | ControlError::BadReply(_)) if write => Err(PaneError::Timeout { op: op.to_owned() }),
        Err(other) => Err(PaneError::Unavailable { what: format!("{method}: {other}") }),
    }
}

fn decode<T: DeserializeOwned>(method: &str, v: Option<Value>) -> Result<T, PaneError> {
    serde_json::from_value(v.unwrap_or(Value::Null)).map_err(|e| PaneError::Unavailable { what: format!("{method}: {e}") })
}

#[derive(Debug, Default)]
pub struct HubPaneStore;
impl HubPaneStore { pub fn new() -> Self { Self } }
#[derive(Debug, Default)]
pub struct HubProfileStore;
impl HubProfileStore { pub fn new() -> Self { Self } }

struct Poll<E> { method: &'static str, op: &'static str, since: Cursor, buf: std::collections::VecDeque<E>, done: bool }
impl<E: DeserializeOwned> Iterator for Poll<E> {
    type Item = Result<Option<E>, PaneError>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(e) = self.buf.pop_front() { return Some(Ok(Some(e))); }
        if self.done { return None; }
        let r = call(self.method, json!({"since": self.since}), holler_hub::panes::WATCH_WAIT + Duration::from_secs(5), false, self.op)
            .and_then(|v| decode::<WatchReply<E>>(self.method, v));
        match r {
            Err(e) => { self.done = true; Some(Err(e)) }
            Ok(reply) => { self.since = reply.cursor; self.buf.extend(reply.events); Some(Ok(self.buf.pop_front())) }
        }
    }
}

impl PaneStore for HubPaneStore {
    fn get(&self, n: &PaneName) -> Result<Option<Pane>, PaneError> { decode("pane/get", call("pane/get", json!({"name": n}), BOUND, false, "pane_store.get")?) }
    fn list(&self) -> Result<Vec<Pane>, PaneError> { decode("pane/list", call("pane/list", json!({}), BOUND, false, "pane_store.list")?) }
    fn cas_put(&self, p: &Pane, g: u64) -> Result<Pane, PaneError> { decode("pane/cas_put", call("pane/cas_put", serde_json::to_value(PaneCasPutParams { pane: p.clone(), expected_generation: g }).unwrap(), BOUND, true, "pane_store.cas_put")?) }
    fn delete(&self, n: &PaneName, g: u64) -> Result<(), PaneError> { call("pane/delete", serde_json::to_value(PaneDeleteParams { name: n.clone(), expected_generation: g }).unwrap(), BOUND, true, "pane_store.delete").map(drop) }
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> { Ok(Box::new(Poll::<PaneEvent> { method: "pane/watch", op: "pane_store.watch_next", since, buf: Default::default(), done: false })) }
}
impl ProfileStore for HubProfileStore {
    fn get(&self, n: &ProfileName) -> Result<Option<Profile>, PaneError> { decode("profile/get", call("profile/get", json!({"name": n}), BOUND, false, "profile_store.get")?) }
    fn list(&self) -> Result<Vec<Profile>, PaneError> { decode("profile/list", call("profile/list", json!({}), BOUND, false, "profile_store.list")?) }
    fn cas_put(&self, p: &Profile, g: u64, a: &Actor) -> Result<Profile, PaneError> { decode("profile/cas_put", call("profile/cas_put", serde_json::to_value(ProfileCasPutParams { profile: p.clone(), expected_generation: g, actor: a.clone() }).unwrap(), BOUND, true, "profile_store.cas_put")?) }
    fn delete(&self, n: &ProfileName, g: u64, a: &Actor) -> Result<(), PaneError> { call("profile/delete", serde_json::to_value(ProfileDeleteParams { name: n.clone(), expected_generation: g, actor: a.clone() }).unwrap(), BOUND, true, "profile_store.delete").map(drop) }
    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> { Ok(Box::new(Poll::<ProfileEvent> { method: "profile/watch", op: "profile_store.watch_next", since, buf: Default::default(), done: false })) }
    fn log(&self, n: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> { decode("profile/log", call("profile/log", json!({"name": n}), BOUND, false, "profile_store.log")?) }
    fn rename(&self, f: &ProfileName, t: &ProfileName, g: u64, a: &Actor) -> Result<Profile, PaneError> { decode("profile/rename", call("profile/rename", serde_json::to_value(ProfileRenameParams { from: f.clone(), to: t.clone(), expected_generation: g, actor: a.clone() }).unwrap(), BOUND, true, "profile_store.rename")?) }
}
pub fn stores() -> (Arc<HubPaneStore>, Arc<HubProfileStore>) { (Arc::new(HubPaneStore), Arc::new(HubProfileStore)) }
