#![allow(dead_code)] // #637 (each test binary uses a different subset of these helpers)
//! Shared fixtures for the `holler-pane` tests: the contract's records as JSON (the
//! wire form is what #639/#661/#649 depend on), typed builders over them, and a
//! trivial in-memory `PaneStore`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use holler_pane::{
    Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Profile, ProfileName, ProfileSpec,
    Watch,
};
use serde_json::{json, Value};

pub fn grid(row: u16, col: u16) -> Value {
    json!({"row": row, "col": col, "pos": format!("r{row}c{col}")})
}

/// A fully populated `Pane` in the contract's field names (epic #633, "The contract").
pub fn pane_json() -> Value {
    json!({
        "name": "hj-c1r1",
        "generation": 7,
        "herdr": {"session": "hj", "workspace": "main", "pane_id": "p_12", "grid": grid(1, 1)},
        "host": {
            "name": "kiwi",
            "tmux": "hj-c1r1",
            "cwd": "/work/holler",
            "herdr_api_version": "0.9.1"
        },
        "harness": {"kind": "opencode", "port": 8095, "pid": 4242, "health": "healthy"},
        "session_of_record": "ses_abc",
        "role": "agent",
        "hold": {"parked": {
            "reason": "quota",
            "release_when": "2026-10-10T00:00:00Z",
            "since": 1_760_000_000_000_i64
        }},
        "last_observed": {"shown": "ses_abc", "driven": "ses_abc", "at": 1_760_000_000_123_i64},
        "profile": "Some Profile",
        "model": {"provider": "anthropic", "model_id": "sonnet", "effort": "high"},
        "env": ["ANTHROPIC_API_KEY", "OPENCODE_CONFIG"],
        "context": {"soft": 100_000, "hard": 150_000},
        "command": ["opencode", "--port", "8095"],
        "probe": {
            "check": ["curl", "-s", "http://127.0.0.1:8095/v1/models"],
            "expect": ["qwen38"],
            "last": {"failed": {"missing": ["qwen38"]}}
        }
    })
}

/// A fully populated `ProfileSpec`.
pub fn spec_json() -> Value {
    json!({
        "pane": "hj-c1r1",
        "herdr": {"workspace": "main", "grid": grid(1, 1)},
        "host": {"cwd": "/work/holler"},
        "harness": {"kind": "opencode", "port_policy": "fixed"},
        "model": {"provider": "anthropic", "model_id": "sonnet", "effort": "high"},
        "role": "agent",
        "env": ["ANTHROPIC_API_KEY"],
        "context": {"soft": 100_000, "hard": 150_000},
        "command": ["opencode", "--port", "8095"],
        "check": ["curl", "-s", "http://127.0.0.1:8095/v1/models"],
        "expect": ["qwen38"]
    })
}

/// A `Profile` with two specs. It has no `log` field (the log is read through
/// `ProfileStore::log`, so `cas_put` cannot rewrite history).
pub fn profile_json() -> Value {
    let mut second = spec_json();
    second["pane"] = json!("hj-c2r1");
    second["herdr"]["grid"] = grid(1, 2);
    second["role"] = json!("orchestrator");
    json!({
        "name": "Some Profile",
        "slug": "some-profile",
        "generation": 3,
        "panes": [spec_json(), second],
        "created": 1_760_000_000_000_i64,
        "updated": 1_760_000_100_000_i64
    })
}

pub fn pane() -> Pane {
    serde_json::from_value(pane_json()).expect("the sample pane loads")
}

pub fn spec() -> ProfileSpec {
    serde_json::from_value(spec_json()).expect("the sample spec loads")
}

pub fn profile() -> Profile {
    serde_json::from_value(profile_json()).expect("the sample profile loads")
}

pub fn pane_name(s: &str) -> PaneName {
    PaneName::parse(s).unwrap_or_else(|e| panic!("{s:?} is a valid pane name: {e}"))
}

pub fn profile_name(s: &str) -> ProfileName {
    ProfileName::parse(s).unwrap_or_else(|e| panic!("{s:?} is a valid profile name: {e}"))
}

/// A trivial in-memory `PaneStore`: the CAS rule is "a write names the generation it
/// read; the store bumps it by one; a stale one is `PaneError::Conflict`".
#[derive(Default)]
pub struct MemPaneStore {
    panes: Mutex<BTreeMap<PaneName, Pane>>,
}

impl MemPaneStore {
    pub fn with(panes: Vec<Pane>) -> Self {
        let map = panes.into_iter().map(|p| (p.name.clone(), p)).collect();
        Self {
            panes: Mutex::new(map),
        }
    }
}

fn empty_watch<T: Send + 'static>() -> Watch<T> {
    Box::new(std::iter::empty::<Result<Option<T>, PaneError>>())
}

impl PaneStore for MemPaneStore {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        Ok(self.panes.lock().unwrap().get(name).cloned())
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        Ok(self.panes.lock().unwrap().values().cloned().collect())
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        let mut panes = self.panes.lock().unwrap();
        let current = panes.get(&pane.name).map_or(0, |p| p.generation);
        if current != expected_generation {
            return Err(PaneError::Conflict);
        }
        let mut next = pane.clone();
        next.generation = current + 1;
        panes.insert(next.name.clone(), next.clone());
        Ok(next)
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        let mut panes = self.panes.lock().unwrap();
        let current = panes.get(name).map_or(0, |p| p.generation);
        if current != expected_generation {
            return Err(PaneError::Conflict);
        }
        panes.remove(name);
        Ok(())
    }

    fn watch(&self, _since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        Ok(empty_watch())
    }
}
