#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #451
//! Issue #451: `hub status` shows who is locked out, why, and for how long.
//! Reuses #450's rejected-token reproduction: every body start with a revoked
//! token is one rejected authentication. Real hub, real bodies, no mocks;
//! readiness is observed via the hub's own log, never slept for.

mod support;

use support::{hub_status_json, join, mint_token, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// Start a hub with the given lockout limit, mint+revoke a token, and cause
/// `rejections` rejected authentications; returns once each is logged.
fn reject_n_times(max_failures: &str, rejections: usize) -> (StateDir, Hub, String) {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_LOCKOUT_MAX_FAILURES", max_failures), ("HOLLER_DEBUG", "none")]);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let out = support::holler_cmd(&state).args(["hub", "token", "revoke", &token_id]).output().expect("run hub token revoke");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let config = support::write_sessions_toml(&state, &[("alpha", &[])]);
    for n in 1..=rejections {
        let body = Body::start(&state, &config);
        wait_for(STARTUP_WAIT, || (count(&hub.log_text(), "auth_rejected") >= n).then_some(()))
            .unwrap_or_else(|| panic!("rejection #{n} was never logged:\n{}", hub.log_text()));
        drop(body);
    }
    (state, hub, token_id)
}

#[test]
fn hub_status_reports_a_locked_out_peer_with_reason_cooldown_and_token() {
    let (state, hub, token_id) = reject_n_times("3", 3);
    wait_for(std::time::Duration::from_secs(10), || hub.log_text().contains("lockout_tripped").then_some(()))
        .unwrap_or_else(|| panic!("the lockout trip was never logged:\n{}", hub.log_text()));

    let doc = hub_status_json(&state);
    let peers = doc["lockout"]["peers"].as_array().unwrap_or_else(|| panic!("status has no lockout.peers array: {doc}"));
    assert_eq!(peers.len(), 1, "one locked-out peer: {doc}");
    let p = &peers[0];
    assert_eq!(p["locked_out"], true, "{p}");
    assert_eq!(p["failures"], 3, "{p}");
    assert_eq!(p["reasons"], serde_json::json!({ "token_not_bound": 3 }), "{p}");
    assert!(p["retry_after_secs"].as_u64().unwrap() > 0, "a locked-out peer has time left: {p}");
    assert!(p["peer"].as_str().is_some_and(|s| !s.is_empty()), "{p}");
    assert_eq!(p["token_ids"], serde_json::json!([{"id": token_id, "label": "body-1"}]), "{p}");
    assert!(doc["limits"]["lockout"].is_object() || doc["limits"].get("lockout").is_some(), "pre-existing limits.lockout is kept: {doc}");
}

#[test]
fn hub_status_reports_a_peer_whose_failures_are_still_accumulating() {
    let (state, _hub, token_id) = reject_n_times("5", 2);

    let doc = hub_status_json(&state);
    let peers = doc["lockout"]["peers"].as_array().unwrap_or_else(|| panic!("status has no lockout.peers array: {doc}"));
    assert_eq!(peers.len(), 1, "{doc}");
    let p = &peers[0];
    assert_eq!(p["locked_out"], false, "2 of 5 failures is not a lockout: {p}");
    assert_eq!(p["failures"], 2, "{p}");
    assert_eq!(p["retry_after_secs"], 0, "{p}");
    assert_eq!(p["token_ids"], serde_json::json!([{"id": token_id, "label": "body-1"}]), "{p}");
}

#[test]
fn hub_status_with_no_failures_has_an_empty_lockout_list() {
    let state = StateDir::new();
    let _hub = Hub::start(&state);
    let doc = hub_status_json(&state);
    assert_eq!(doc["lockout"], serde_json::json!({ "peers": [] }), "stable shape when quiet: {doc}");
}

#[test]
fn hub_status_text_lists_the_locked_out_peer_with_reason_and_label() {
    let (state, hub, _token_id) = reject_n_times("3", 3);
    wait_for(std::time::Duration::from_secs(10), || hub.log_text().contains("lockout_tripped").then_some(()))
        .unwrap_or_else(|| panic!("the lockout trip was never logged:\n{}", hub.log_text()));

    let out = support::holler_cmd(&state).args(["hub", "status"]).output().expect("run hub status");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    for needle in ["lockout:", "1 locked out", "locked out, retry in", "token_not_bound x3", "body-1"] {
        assert!(text.contains(needle), "text output lacks {needle:?}:\n{text}");
    }
}
