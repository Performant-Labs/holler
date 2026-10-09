#![allow(dead_code)] // #639 (not every test binary uses every helper)
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #639
//! Shared helpers of the pane tests (issue #639): the one sample pane, a throwaway state
//! dir, the short store options, and the parse-back of a `pane/*` reply line.
//!
//! Included with `mod pane_support;` by `pane_registry_test.rs`, `pane_handlers_test.rs`
//! and `pane_dispatch_test.rs`.

use std::path::PathBuf;
use std::time::Duration;

use holler_hub::panes::{PaneState, PaneStoreOptions};
use holler_hub::state::HubState;
use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneReply, PaneStore, Watch};
use serde_json::{json, Value};

/// A fully populated pane named `name`, at generation 7, in `profile` when given.
pub fn sample_pane(name: &str, profile: Option<&str>) -> Pane {
    let mut pane = json!({
        "name": name,
        "generation": 7,
        "herdr": {"session": "hj", "workspace": "main", "pane_id": "p_12",
                  "grid": {"row": 1, "col": 1, "pos": "r1c1"}},
        "host": {"name": "kiwi", "tmux": name, "cwd": "/work/holler", "herdr_api_version": "0.9.1"},
        "harness": {"kind": "opencode", "port": 8095, "pid": 4242, "health": "healthy"},
        "role": "agent",
        "hold": {"parked": {"reason": "quota", "release_when": "2026-10-10T00:00:00Z", "since": 1_760_000_000_000_i64}},
        "last_observed": {"shown": "ses_abc", "driven": "ses_abc", "at": 1_760_000_000_123_i64},
        "model": {"provider": "anthropic", "model_id": "sonnet", "effort": "high"},
        "env": ["ANTHROPIC_API_KEY"],
        "context": {"soft": 100_000, "hard": 150_000},
        "command": ["opencode", "--port", "8095"],
        "probe": {"check": ["curl", "-s", "http://127.0.0.1:8095/v1/models"], "expect": ["qwen38"]}
    });
    if let Some(profile) = profile {
        pane["profile"] = json!(profile);
    }
    serde_json::from_value(pane).unwrap()
}

/// A valid pane name.
pub fn name(text: &str) -> holler_pane::PaneName {
    holler_pane::PaneName::parse(text).unwrap()
}

/// A throwaway state dir. `<root>/hub` is deliberately not created: a real hub creates
/// it before the first connection, and the registry must also cope without.
pub fn temp_state() -> (tempfile::TempDir, HubState) {
    let dir = tempfile::tempdir().unwrap();
    let state = HubState::from_root(dir.path().to_path_buf());
    (dir, state)
}

/// Options with a short long-poll window, so an idle `next()` returns quickly. The ring
/// keeps the default length.
pub fn short_opts() -> PaneStoreOptions {
    PaneStoreOptions {
        watch_wait: Duration::from_millis(100),
        ..PaneStoreOptions::default()
    }
}

/// Load the registry of `state` with the short options.
pub fn load(state: &HubState) -> PaneState {
    PaneState::load_with(state, short_opts())
}

/// `<root>/hub/panes.json`.
pub fn registry_file(state: &HubState) -> PathBuf {
    state.hub_dir.join("panes.json")
}

/// Create `name` (sample pane, no profile) at generation 1.
pub fn create(store: &PaneState, name: &str) -> Pane {
    store.cas_put(&sample_pane(name, None), 0).unwrap()
}

/// Everything the iterator yields until it reports idle. Any error fails the test.
pub fn drain(watch: &mut Watch<PaneEvent>) -> Vec<PaneEvent> {
    let mut events = Vec::new();
    loop {
        match watch.next() {
            Some(Ok(Some(event))) => events.push(event),
            Some(Ok(None)) => return events,
            other => panic!("the watch ended or failed while draining: {other:?}"),
        }
    }
}

/// The head cursor of the feed, found through the public port alone: `watch(n)` is
/// accepted exactly when `n <= head` (a cursor ahead of the store is `usage`).
pub fn head(store: &PaneState) -> Cursor {
    let (mut lo, mut hi) = (0_u64, 1_u64 << 24);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        match store.watch(Cursor(mid)) {
            Ok(_) => lo = mid,
            Err(PaneError::Usage { .. }) => hi = mid - 1,
            Err(other) => panic!("watch({mid}) failed unexpectedly: {other}"),
        }
    }
    Cursor(lo)
}

/// The names in `dir`, sorted: used to prove a refused write created or moved nothing.
pub fn dir_listing(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The reply's `result` parsed as a `PaneReply` and read back as a `Result`: the
/// parse-back a client does, so it also pins the code on the wire. A JSON-RPC error
/// frame fails the test.
pub fn pane_outcome(reply: &Value, method: &str) -> Result<Option<Value>, PaneError> {
    assert!(
        reply.get("error").is_none(),
        "{method} must be a JSON-RPC result carrying a PaneReply, not a JSON-RPC error: {reply}"
    );
    let result = reply
        .get("result")
        .unwrap_or_else(|| panic!("{method} has no result: {reply}"));
    let pane_reply: PaneReply = serde_json::from_value(result.clone())
        .unwrap_or_else(|e| panic!("the result of {method} is not a PaneReply ({e}): {result}"));
    pane_reply.into_result()
}

/// [`pane_outcome`] for a reply line, as `panes::dispatch` returns it.
pub fn outcome(line: &str, method: &str) -> Result<Option<Value>, PaneError> {
    let reply: Value = serde_json::from_str(line)
        .unwrap_or_else(|e| panic!("the reply to {method} is not JSON ({e}): {line:?}"));
    pane_outcome(&reply, method)
}
