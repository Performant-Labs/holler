//! Shared rig for the remote-admin test files (issue #508/#509, epic #506):
//! a hub in one `StateDir` and a body **joined from a separate one** (AC 11:
//! the remote form needs no hub state at all — the body dir has no `hub/`
//! subtree, only `body/credential.json`), hosting sessions. Split out so
//! both `remote_admin_test.rs` (AC 3-13) and `remote_admin_liveness_test.rs`
//! (AC 16, the admin-loop liveness cases, which need their own low
//! `HOLLER_WS_PING_INTERVAL_MS`/`HOLLER_HUB_LIVENESS_TIMEOUT_MS` hub) can use
//! the same rig without a second copy (handoff-S round-2, A-dup W-2's own
//! "do not duplicate" rule, and this crate's 900-line-per-file guard, which
//! the full AC 3-17 suite in one file would have blown past).

#![allow(dead_code)] // #508: each test binary uses a subset

use std::process::Stdio;

use super::{join, mint_token, roster_json, wait_for, write_sessions_toml, Body, Hub, StateDir, STARTUP_WAIT};

/// [`rig_with_env`] with no extra hub environment — the ordinary AC 3-13 rig.
pub fn two_dir_rig(sessions: &[(&str, &[&str])]) -> (StateDir, StateDir, Hub, Body, String, String) {
    rig_with_env(sessions, &[])
}

/// [`two_dir_rig`], also passing `hub_env` to the hub process (AC 16: a low
/// ping interval and liveness timeout so the admin-loop liveness cases run
/// in seconds, not minutes).
pub fn rig_with_env(sessions: &[(&str, &[&str])], hub_env: &[(&str, &str)]) -> (StateDir, StateDir, Hub, Body, String, String) {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, hub_env);
    let (token_id, secret) = mint_token(&hub_state, "b");
    join(&body_state, &hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&body_state, sessions);
    let body = Body::start(&body_state, &config);
    for (name, _) in sessions {
        let want = format!("b/{name}");
        wait_for(STARTUP_WAIT, || {
            roster_json(&hub_state)["rows"].as_array()?.iter().any(|r| r["name"] == want.as_str() && r["state"] == "idle").then_some(())
        })
        .unwrap_or_else(|| panic!("session {want} never came up idle"));
    }
    let ws_url = hub.ws_url();
    (hub_state, body_state, hub, body, token_id, ws_url)
}

/// Run `holler ARGS...` against `state` and return the raw `Output`.
pub fn run(state: &StateDir, args: &[&str]) -> std::process::Output {
    super::holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler")
}

/// Spawn `holler ARGS...` against `state` without waiting — for tests that
/// need several CLI invocations racing each other (AC 8).
pub fn spawn_cmd(state: &StateDir, args: &[&str]) -> std::process::Child {
    super::holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
}

/// Read `body_state`'s persisted `credential.json`, let `f` mutate it, and
/// write it back — how AC 7/13's bad-credential cases (an unknown token, a
/// revoked one) corrupt a real joined identity without a second, hand-rolled
/// join path.
pub fn mutate_credential(body_state: &StateDir, f: impl FnOnce(&mut serde_json::Value)) {
    let path = holler_body::identity::BodyIdentity::path(body_state.path());
    let mut v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read credential.json")).expect("credential.json is valid JSON");
    f(&mut v);
    std::fs::write(&path, serde_json::to_vec_pretty(&v).expect("serialize credential.json")).expect("write credential.json");
}
