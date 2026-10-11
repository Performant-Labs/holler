#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #453
//! Issue #453: a token's `expires` bounds only the unredeemed join secret. A
//! body that has already joined keeps re-authenticating after that deadline;
//! `hub token revoke` is what ends a bound credential. `hub token ping` and
//! `hub token list` follow the same rule (`--json` output is unchanged).
//!
//! Real hub, real bodies (stub-acp agents), no mocks of the circuit. Each test
//! has its own `StateDir` and hub, so the hub log and the store it asserts on
//! hold only that test's events and records. Readiness is observed through
//! the roster, the hub log or a body's exit via [`support::wait_for`], never
//! slept for.

mod support;

use std::process::Output;
use std::time::Duration;

use serde_json::Value;

use support::{join, mint_token, roster_json, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

const SESSION: &str = "alpha";

/// Unlocked edit of tokens.json: set `key` on the store's only record (the
/// CLI cannot set a past `expires` or an `unused` state). Valid only while no
/// body for the token is connected: the hub's own writes load and save the
/// whole file under a flock this edit does not take.
fn edit_record(state: &StateDir, token_id: &str, key: &str, value: Value) {
    let path = state.hub().join("tokens.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read tokens.json")).expect("tokens.json is JSON");
    let rows = doc.as_array_mut().expect("tokens.json is a JSON array");
    assert_eq!(rows.len(), 1, "the edit helper expects exactly one record: {rows:?}");
    assert_eq!(rows[0]["token_id"].as_str(), Some(token_id), "the one record is this token");
    rows[0][key] = value;
    std::fs::write(&path, serde_json::to_string(&doc).expect("serialize")).expect("write tokens.json");
}

/// The stored record of `token_id`, read straight from tokens.json.
fn stored(state: &StateDir, token_id: &str) -> Value {
    let raw = std::fs::read_to_string(state.hub().join("tokens.json")).expect("read tokens.json");
    let doc: Value = serde_json::from_str(&raw).expect("tokens.json is JSON");
    doc.as_array()
        .and_then(|a| a.iter().find(|r| r["token_id"].as_str() == Some(token_id)).cloned())
        .unwrap_or_else(|| panic!("{token_id} is in the store: {doc}"))
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// Whether the default roster listing shows the session as `connected`.
/// A row that is hidden (`gone` rows are not listed) or `gone` counts as not.
fn session_connected(state: &StateDir) -> bool {
    roster_json(state)["rows"].as_array().is_some_and(|rows| {
        rows.iter().any(|r| {
            r["name"].as_str().is_some_and(|n| n.ends_with(&format!("/{SESSION}"))) && r["conn_state"].as_str() == Some("connected")
        })
    })
}

/// Wait until the session is `connected`; fail fast (with the hub log) if the
/// hub refuses the authentication instead.
fn wait_connected(state: &StateDir, hub: &Hub, what: &str) {
    let rejected_before = count(&hub.log_text(), "auth_rejected");
    let outcome = wait_for(STARTUP_WAIT, || {
        if session_connected(state) {
            Some(true)
        } else {
            (count(&hub.log_text(), "auth_rejected") > rejected_before).then_some(false)
        }
    });
    assert_eq!(outcome, Some(true), "{what}: the body never reached `connected`:\n{}", hub.log_text());
}

fn wait_not_connected(state: &StateDir, timeout: Duration, what: &str) {
    wait_for(timeout, || (!session_connected(state)).then_some(()))
        .unwrap_or_else(|| panic!("{what}: the session is still `connected` after {timeout:?}"));
}

fn holler(state: &StateDir, args: &[&str]) -> Output {
    support::holler_cmd(state).args(args).output().expect("run holler")
}

/// `hub token ping <id> --json`: (the JSON document without `rtt_ms`, exit code).
fn ping_json(state: &StateDir, token_id: &str) -> (Value, Option<i32>) {
    let out = holler(state, &["--json", "hub", "token", "ping", token_id]);
    let mut doc: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("ping --json prints JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    if let Some(m) = doc.as_object_mut() {
        m.remove("rtt_ms"); // a live measurement, not part of the rule under test
    }
    (doc, out.status.code())
}

/// A hub, one minted-and-joined token (label `body-1`), and a sessions config.
fn bound_setup(hub_env: &[(&str, &str)]) -> (StateDir, Hub, String, std::path::PathBuf) {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, hub_env);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let config = support::write_sessions_toml(&state, &[(SESSION, &[])]);
    (state, hub, token_id, config)
}

/// Criterion 1: a bound token whose `expires` has passed authenticates, again
/// and again; heartbeat writes never normalise the stored `expires`.
#[test]
fn bound_token_past_expires_still_authenticates() {
    let (state, hub, token_id, config) = bound_setup(&[]);
    edit_record(&state, &token_id, "expires", Value::from(1u64));

    for n in 1..=3 {
        let body = Body::start(&state, &config);
        wait_connected(&state, &hub, &format!("start #{n} with expires=1"));
        drop(body); // kill the process tree: `Body::stop` would detach (delete the credential)
        wait_not_connected(&state, STARTUP_WAIT, &format!("after stop #{n}"));
    }

    let log = hub.log_text();
    assert_eq!(count(&log, "auth_rejected"), 0, "no authentication was refused:\n{log}");
    assert_eq!(stored(&state, &token_id)["expires"], Value::from(1u64), "the stored expires is left as it was");
}

/// Criterion 2 (integration): a token whose stored state is `unused` cannot
/// authenticate, and the hub logs `token_not_bound`. Characterization: passes
/// before and after #453.
#[test]
fn unused_token_authenticate_is_refused_token_not_bound() {
    let (state, hub, token_id, config) = bound_setup(&[]);
    edit_record(&state, &token_id, "state", Value::from("unused"));

    let mut body = Body::start(&state, &config);
    wait_for(STARTUP_WAIT, || hub.log_text().contains(r#""reason":"token_not_bound""#).then_some(()))
        .unwrap_or_else(|| panic!("an unused token's authenticate was never refused token_not_bound:\n{}", hub.log_text()));
    // The refusal is final: the body exits non-zero instead of running. A body
    // that authenticated would stay up, so this fails if the state check goes.
    let exited = wait_for(STARTUP_WAIT, || body.child_mut().try_wait().expect("try_wait on the body"));
    assert!(exited.is_some_and(|s| !s.success()), "a refused body exits non-zero: {exited:?}\n{}", hub.log_text());
}

/// Criterion 3: revoking a connected body (whose `expires` has passed) takes
/// it off the roster, and a fresh start of that body is refused.
#[test]
fn revoke_closes_connected_body_and_reconnect_is_refused() {
    let (state, hub, token_id, config) = bound_setup(&[("HOLLER_LOCKOUT_MAX_FAILURES", "100")]);
    edit_record(&state, &token_id, "expires", Value::from(1u64));
    let mut body = Body::start(&state, &config);
    wait_connected(&state, &hub, "before revoke");

    let out = holler(&state, &["hub", "token", "revoke", &token_id]);
    assert!(out.status.success(), "hub token revoke must succeed: {}", String::from_utf8_lossy(&out.stderr));
    wait_not_connected(&state, Duration::from_secs(10), "after revoke");

    // The old body gives up on its own: its reconnect is refused (-32002) and
    // it exits without retrying. Waiting for that exit, rather than killing
    // it, fixes the baseline below: the hub logs a refusal before sending it,
    // so once the process is gone none of its refusals can still be in flight.
    let exited = wait_for(STARTUP_WAIT, || body.child_mut().try_wait().expect("try_wait on the old body"));
    assert!(exited.is_some(), "a revoked body must stop retrying and exit:\n{}", hub.log_text());
    drop(body);
    let not_bound = || count(&hub.log_text(), r#""reason":"token_not_bound""#);
    let n0 = not_bound();

    let _fresh = Body::start(&state, &config);
    wait_for(STARTUP_WAIT, || (not_bound() > n0).then_some(()))
        .unwrap_or_else(|| panic!("the fresh start of a revoked body was never refused token_not_bound:\n{}", hub.log_text()));
    let got_in = wait_for(Duration::from_secs(5), || session_connected(&state).then_some(()));
    assert!(got_in.is_none(), "a revoked body must never show `connected`:\n{}", hub.log_text());
    let log = hub.log_text();
    assert_eq!(count(&log, "lockout_tripped"), 0, "the high lockout limit is never reached:\n{log}");
}

/// Criterion 4: `hub token ping` treats a bound token past `expires` exactly
/// like the same token with a future `expires`, with no body and with one.
#[test]
fn ping_bound_token_past_expires_is_not_expired() {
    let (state, hub, token_id, config) = bound_setup(&[]);
    let future = stored(&state, &token_id)["expires"].clone();

    // No body connected: future expires first, then the same token past it.
    let idle_future = ping_json(&state, &token_id);
    edit_record(&state, &token_id, "expires", Value::from(1u64));
    let idle_past = ping_json(&state, &token_id);
    assert_eq!(idle_past, idle_future, "no body: a past expires must ping like a future one");

    // Body connected, past expires.
    let body = Body::start(&state, &config);
    wait_connected(&state, &hub, "ping with expires=1");
    let live_past = ping_json(&state, &token_id);
    drop(body); // kill the process tree: `Body::stop` would detach (delete the credential)
    wait_not_connected(&state, STARTUP_WAIT, "after stop");

    // Body connected, the original future expires restored.
    edit_record(&state, &token_id, "expires", future);
    let body = Body::start(&state, &config);
    wait_connected(&state, &hub, "ping with future expires");
    let live_future = ping_json(&state, &token_id);
    drop(body); // kill the process tree: `Body::stop` would detach (delete the credential)
    assert_eq!(live_past, live_future, "connected: a past expires must ping like a future one");
}

/// Criterion 4, unchanged half: an `unused` token past its `expires` still
/// pings `expired` with exit 3 (no hub needed: it is refused before any live
/// probe). Characterization: passes before and after #453.
#[test]
fn ping_unused_token_past_expires_is_still_expired() {
    let state = StateDir::new();
    let (token_id, _secret) = mint_token(&state, "body-1");
    edit_record(&state, &token_id, "expires", Value::from(1u64));
    let (doc, code) = ping_json(&state, &token_id);
    assert_eq!(doc["state"], "expired", "{doc}");
    assert_eq!(code, Some(3), "{doc}");
}

/// Criterion 5: in text output EXPIRES is `-` for `bound` and `revoked` rows
/// and a date for `unused` rows; `--json` keeps its shape and `expires` is the
/// stored number for every row.
#[test]
fn list_expires_dash_for_non_unused_and_json_unchanged() {
    let (state, _hub, bound_id, _config) = bound_setup(&[]);
    let (unused_id, _) = mint_token(&state, "body-2");
    let (revoked_id, _) = mint_token(&state, "body-3");
    let out = holler(&state, &["hub", "token", "revoke", &revoked_id]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let out = holler(&state, &["hub", "token", "list"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let row = |id: &str| -> Vec<String> {
        let line = text.lines().find(|l| l.starts_with(id)).unwrap_or_else(|| panic!("{id} is listed:\n{text}"));
        line.split_whitespace().map(str::to_owned).collect()
    };
    let is_date = |d: &str, t: &str| d.len() == 10 && d.as_bytes()[4] == b'-' && t.len() == 8 && t.as_bytes()[2] == b':';
    for (id, state_word) in [(&bound_id, "bound"), (&revoked_id, "revoked")] {
        let cols = row(id);
        assert!(cols.contains(&state_word.to_string()), "{id} is {state_word}:\n{text}");
        assert_eq!(cols.last().map(String::as_str), Some("-"), "a {state_word} row's EXPIRES is `-`:\n{text}");
    }
    let cols = row(&unused_id);
    assert!(cols.contains(&"unused".to_string()), "{text}");
    let n = cols.len();
    assert!(n >= 2 && is_date(&cols[n - 2], &cols[n - 1]), "an unused row's EXPIRES is a date:\n{text}");

    let out = holler(&state, &["--json", "hub", "token", "list"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let doc: Value = serde_json::from_slice(&out.stdout).expect("list --json is JSON");
    let tokens = doc["tokens"].as_array().unwrap_or_else(|| panic!("{doc}"));
    assert_eq!(tokens.len(), 3, "{doc}");
    for t in tokens {
        let mut keys: Vec<&str> = t.as_object().expect("row object").keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["expires", "label", "last_seen", "machine", "state", "token_id"], "row shape: {t}");
        let id = t["token_id"].as_str().expect("token_id");
        assert!(t["expires"].is_u64(), "expires stays numeric: {t}");
        assert_eq!(t["expires"], stored(&state, id)["expires"], "expires is the stored value: {t}");
    }
}
