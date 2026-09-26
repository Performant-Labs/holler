#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #451
//! Issue #451: `hub status` shows who is locked out, why, and for how long.
//! Reuses #450's rejected-token reproduction: every body start with a revoked
//! token is one rejected authentication. Real hub, real bodies, no mocks;
//! readiness is observed via the hub's own log, never slept for.

mod support;

use serde_json::{json, Value};
use support::raw_ws::{authenticate_on, authenticate_once, connect_ws, connect_ws_with_headers, hub_x25519_pubkey, log_events, wait_for_close};
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
    assert_eq!(p["scope"], "token", "one failing token is a token-scope entry (#455): {p}");
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
    assert_eq!(p["scope"], "token", "{p}");
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

/// The `lockout.peers` array of `hub status --json`.
fn lockout_peers(doc: &Value) -> Vec<Value> {
    doc["lockout"]["peers"].as_array().unwrap_or_else(|| panic!("status has no lockout.peers array: {doc}")).clone()
}

/// Issue #455, criterion 5: behind a proxy every body shares one address. A
/// token that trips the lockout locks out only itself: a second body with its
/// own token, from the same address, still joins and goes live.
#[test]
fn a_locked_out_token_does_not_lock_out_another_body_on_the_same_address() {
    let (state, hub, token_a) = reject_n_times("3", 3);
    wait_for(std::time::Duration::from_secs(10), || hub.log_text().contains("lockout_tripped").then_some(()))
        .unwrap_or_else(|| panic!("token A's lockout trip was never logged:\n{}", hub.log_text()));

    let (token_b, secret_b) = mint_token(&state, "body-2");
    let body_b_state = StateDir::new();
    join(&body_b_state, &state, &hub.ws_url(), &token_b, &secret_b);
    let config_b = support::write_sessions_toml(&body_b_state, &[("alpha", &[])]);
    let _body_b = Body::start(&body_b_state, &config_b);
    wait_for(STARTUP_WAIT, || {
        let doc = hub_status_json(&state);
        let live = doc["clients_detail"].as_array().is_some_and(|c| c.iter().any(|r| r["token_id"] == token_b.as_str()));
        live.then_some(())
    })
    .unwrap_or_else(|| panic!("body B never went live beside locked-out token A:\n{}", hub.log_text()));

    let doc = hub_status_json(&state);
    let peers = lockout_peers(&doc);
    assert_eq!(peers.len(), 1, "only token A is tracked: {doc}");
    let p = &peers[0];
    assert_eq!(p["scope"], "token", "{p}");
    assert_eq!(p["locked_out"], true, "{p}");
    assert_eq!(p["token_ids"], json!([{"id": token_a, "label": "body-1"}]), "{p}");
}

/// Issue #455, criterion 7 (the #184 flood guard): a client cycling through
/// distinct token ids is still locked out as a whole. Seven distinct unknown
/// ids are each answered `-32002` and each gets its own token-scope entry; the
/// eighth trips the whole address, after which a socket that sends nothing is
/// refused 1008 before any frame is read. Default limits.
#[tokio::test]
async fn cycling_eight_distinct_token_ids_locks_out_the_whole_address() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_DEBUG", "none")]);
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();
    let key = [7u8; 32];

    let mut outcomes = Vec::new();
    for n in 0..7 {
        outcomes.push(authenticate_once(&ws_url, &format!("tok_flood_{n}"), &key, &hub_pubkey).await);
    }
    for (n, o) in outcomes.iter().enumerate() {
        let err = o.as_ref().unwrap_or_else(|e| panic!("distinct id {n} must be answered -32002, not locked out: {e}"));
        assert_eq!(err.code, -32002, "distinct id {n}: {err:?}");
    }
    let doc = hub_status_json(&state);
    let peers = lockout_peers(&doc);
    assert_eq!(peers.len(), 7, "seven distinct ids are seven token-scope entries: {doc}");
    assert!(peers.iter().all(|p| p["scope"] == "token" && p["locked_out"] == false), "no entry is peer scope yet: {doc}");

    let eighth = authenticate_once(&ws_url, "tok_flood_7", &key, &hub_pubkey).await;
    let err = eighth.unwrap_or_else(|e| panic!("the eighth distinct id is refused -32002 (it is counted, and trips the address): {e}"));
    assert_eq!(err.code, -32002, "{err:?}");

    let mut silent = connect_ws(&ws_url).await;
    assert_eq!(wait_for_close(&mut silent).await, Some(1008), "a peer-wide lockout refuses a new socket before any frame");

    let doc = hub_status_json(&state);
    let peers = lockout_peers(&doc);
    assert_eq!(peers.len(), 1, "the eight buckets fold into one peer-wide entry: {doc}");
    let p = &peers[0];
    assert_eq!((p["scope"].clone(), p["locked_out"].clone()), (json!("peer"), json!(true)), "{p}");
    assert_eq!(p["peer"], "127.0.0.1", "{p}");
    assert_eq!(p["failures"], 8, "the folded counts are summed: {p}");
    let tripped = wait_for(STARTUP_WAIT, || log_events(&hub, "lockout_tripped").into_iter().find(|e| e["scope"] == "peer"))
        .unwrap_or_else(|| panic!("no peer-scope lockout_tripped was logged:\n{}", hub.log_text()));
    assert!(tripped["peer"].as_str().is_some_and(|s| s.starts_with("127.0.0.1")), "{tripped}");

    let out = support::holler_cmd(&state).args(["hub", "status"]).output().expect("run hub status");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("all tokens"), "the text view marks a peer-wide entry:\n{text}");
}

/// Issue #455, criterion 8: forwarded-address headers are never read. A client
/// that claims another address in `X-Forwarded-For`, `Forwarded` and
/// `X-Real-IP` is still keyed by its transport address, and the claimed
/// address appears in neither `hub status` nor the hub log (at `noisy`, where
/// the most is logged).
#[tokio::test]
async fn forwarded_headers_never_choose_the_lockout_key() {
    const SPOOFED: &str = "198.51.100.7";
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_DEBUG", "noisy")]);
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();
    let headers = [("X-Forwarded-For", SPOOFED), ("Forwarded", "for=198.51.100.7"), ("X-Real-IP", SPOOFED)];

    for n in 0..2 {
        let mut ws = connect_ws_with_headers(&ws_url, &headers).await;
        let outcome = authenticate_on(&mut ws, &ws_url, "tok_spoofed", &[9u8; 32], &hub_pubkey).await;
        let err = outcome.unwrap_or_else(|e| panic!("attempt {n} must be refused -32002: {e}"));
        assert_eq!(err.code, -32002, "attempt {n}: {err:?}");
    }
    wait_for(STARTUP_WAIT, || (log_events(&hub, "auth_rejected").len() >= 2).then_some(()))
        .unwrap_or_else(|| panic!("two rejections were never logged:\n{}", hub.log_text()));

    let doc = hub_status_json(&state);
    let peers = lockout_peers(&doc);
    assert_eq!(peers.len(), 1, "{doc}");
    assert_eq!(peers[0]["peer"], "127.0.0.1", "keyed by the transport address, not a header: {doc}");
    assert_eq!(peers[0]["failures"], 2, "{doc}");
    assert!(!doc.to_string().contains(SPOOFED), "the spoofed address must not reach hub status: {doc}");
    assert!(!hub.log_text().contains(SPOOFED), "the spoofed address must not reach the hub log:\n{}", hub.log_text());
}
