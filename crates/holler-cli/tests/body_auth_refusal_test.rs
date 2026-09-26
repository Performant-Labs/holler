#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #486
//! What `holler body run` does with each `-32002` refusal (issue #486, and
//! the permanent-rejection slice of #452): a `hub_unavailable` refusal is the
//! hub's own fault, so the body backs off and keeps retrying without ever
//! being counted toward its lockout; a credential refusal stops the body
//! with exit 1 and says in plain words why.
//!
//! Real hub, real body, no mocks. Readiness is observed via the hub's log,
//! the body's log, `connection_state.json` and `try_wait`, never slept for.
//! Kept out of `body_run_test.rs`, which is already over the 600-line warning.

mod support;

use std::process::ExitStatus;
use std::time::Duration;

use serde_json::{json, Value};
use support::{holler_cmd, hub_status_json, join, mint_token, wait_for, Body, Hub, StateDir};

/// Backoff jitter can reach 30 s (full jitter, 1 s to a 30 s cap), so every
/// wait that spans a retry gets this budget.
const RETRY_BUDGET: Duration = Duration::from_secs(45);

/// The JSON log lines in `text` whose `type` is `ty`.
fn events(text: &str, ty: &str) -> Vec<Value> {
    text.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).filter(|v| v["type"] == ty).collect()
}

/// `body/connection_state.json`'s `state`, if the file exists and parses.
fn conn_state(state: &StateDir) -> Option<String> {
    let bytes = std::fs::read(state.body().join("connection_state.json")).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v["state"].as_str().map(str::to_owned)
}

fn wait_conn_state(state: &StateDir, want: &str, budget: Duration, body: &Body) {
    wait_for(budget, || (conn_state(state).as_deref() == Some(want)).then_some(()))
        .unwrap_or_else(|| panic!("connection_state.json never read {want:?} (now {:?}); body log:\n{}", conn_state(state), body.log_text()));
}

fn tokens_path(state: &StateDir) -> std::path::PathBuf {
    state.hub().join("tokens.json")
}

fn assert_secret_absent(what: &str, text: &str, secret: &str) {
    assert!(!text.contains(secret), "the join secret must never appear in the {what}");
}

fn body_config(state: &StateDir) -> std::path::PathBuf {
    support::write_sessions_toml(state, &[])
}

/// #486, acceptance criterion 1: a hub whose token store is corrupt refuses
/// every attempt as `hub_unavailable`. The body keeps retrying (it neither
/// exits nor counts against a lockout of one), reports `reconnecting`, and
/// connects once the store is repaired.
#[test]
fn a_hub_side_fault_keeps_the_body_retrying_until_the_hub_recovers() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_LOCKOUT_MAX_FAILURES", "1"), ("HOLLER_DEBUG", "none")]);
    let (token_id, secret) = mint_token(&state, "fault-body");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let good = std::fs::read(tokens_path(&state)).expect("read the valid store");
    std::fs::write(tokens_path(&state), b"").expect("corrupt the store");

    let mut body = Body::start_with_env(&state, &body_config(&state), &[("HOLLER_LOG_FORMAT", "json")]);

    // (a) Two hub faults while the body is still running: it retried, it did not stop.
    let seen: Result<usize, ExitStatus> = wait_for(RETRY_BUDGET, || {
        if let Some(status) = body.child_mut().try_wait().expect("poll the body") {
            return Some(Err(status));
        }
        let n = events(&hub.log_text(), "auth_unavailable").len();
        (n >= 2).then_some(Ok(n))
    })
    .unwrap_or_else(|| panic!("fewer than 2 auth_unavailable events in {RETRY_BUDGET:?}:\n{}", hub.log_text()));
    let n = seen.unwrap_or_else(|status| {
        panic!("the body stopped ({status}) on a hub_unavailable refusal instead of retrying; body log:\n{}", body.log_text())
    });
    assert!(body.child_mut().try_wait().expect("poll the body").is_none(), "the body is still running after {n} hub faults");

    // (b) It says it is reconnecting.
    wait_conn_state(&state, "reconnecting", RETRY_BUDGET, &body);

    // (c) The drop names the hub's reason.
    let drops = events(&body.log_text(), "conn_dropped");
    assert!(
        drops.iter().any(|e| e["reason"].as_str().is_some_and(|r| r.contains("hub_unavailable"))),
        "no conn_dropped event names hub_unavailable; body log:\n{}",
        body.log_text()
    );

    // (d) Repair the store: the same body connects on a later retry.
    std::fs::write(tokens_path(&state), &good).expect("repair the store");
    wait_conn_state(&state, "connected", RETRY_BUDGET, &body);

    // (e) None of the refusals was counted: a limit of 1 would show any.
    let doc = hub_status_json(&state);
    assert_eq!(doc["lockout"], json!({ "peers": [] }), "a hub fault must never count toward the lockout: {doc}");
    assert!(events(&hub.log_text(), "auth_rejected").is_empty(), "a hub fault is not an auth rejection:\n{}", hub.log_text());
    assert_secret_absent("hub log", &hub.log_text(), &secret);
    assert_secret_absent("body log", &body.log_text(), &secret);

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// #452 slice, acceptance criterion 2: a revoked token stops the body with
/// exit 1 after exactly one counted attempt, and the body says in plain words
/// that the token was revoked, naming the stable code.
#[test]
fn a_revoked_token_stops_the_body_and_says_why() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_DEBUG", "none")]);
    let (token_id, secret) = mint_token(&state, "revoked-body");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let out = holler_cmd(&state).args(["hub", "token", "revoke", &token_id]).output().expect("run hub token revoke");
    assert!(out.status.success(), "revoke must exit 0: {}", String::from_utf8_lossy(&out.stderr));

    let mut body = Body::start_with_env(&state, &body_config(&state), &[("HOLLER_LOG_FORMAT", "json")]);
    let status = wait_for(Duration::from_secs(20), || body.child_mut().try_wait().expect("poll the body"))
        .unwrap_or_else(|| panic!("the body never stopped on a revoked token; body log:\n{}", body.log_text()));
    let log = body.log_text();
    assert_eq!(status.code(), Some(1), "a revoked token is exit 1; body log:\n{log}");
    assert!(log.contains("authentication failed"), "{log}");
    assert!(log.contains("token_not_bound"), "the body names the stable code; body log:\n{log}");
    assert!(log.contains("revoked"), "the body says in plain words that the token was revoked; body log:\n{log}");
    assert_secret_absent("body log", &log, &secret);

    let for_token = |text: &str| events(text, "auth_rejected").into_iter().filter(|e| e["token_id"] == token_id.as_str()).count();
    wait_for(Duration::from_secs(10), || (for_token(&hub.log_text()) >= 1).then_some(()))
        .unwrap_or_else(|| panic!("the rejection was never logged:\n{}", hub.log_text()));
    // The body has exited, so no later attempt can arrive: one is final.
    assert_eq!(for_token(&hub.log_text()), 1, "a stopped body makes exactly one counted attempt:\n{}", hub.log_text());
    assert_ne!(conn_state(&state).as_deref(), Some("connected"), "a refused body never reported connected");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}
