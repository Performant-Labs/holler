#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #450
//! Issue #450: a rejected `circuit/authenticate` and the lockout it feeds are
//! visible in the hub's **default-level** log, with a stable reason code. The
//! incident this comes from: an expired token made a restarting body trip the
//! failed-auth lockout for every client behind the proxy, and the hub log said
//! only `lockout_refused` — no reason, no trip line — so diagnosis took an hour.
//!
//! The hub runs at `HOLLER_DEBUG=none`, the real default: `Warn` events are
//! never gated by the debug level, so a security-relevant rejection needs no
//! `--debug noisy` to appear. Real hub and real bodies (no mocks of the circuit). Readiness is observed
//! through the hub's own log via [`support::wait_for`], never slept for.

mod support;

use std::time::Duration;

use serde_json::Value;

use support::{join, mint_token, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

/// Make the stored token read as expired: the CLI's ttl units stop at minutes,
/// so rewrite the record's `expires` in the store the hub reads on every
/// authentication.
fn expire_token(state: &StateDir, token_id: &str) {
    fn walk(v: &mut Value, token_id: &str) -> bool {
        match v {
            Value::Object(m) => {
                if m.get("token_id").and_then(Value::as_str) == Some(token_id) {
                    m.insert("expires".into(), Value::from(1u64));
                    return true;
                }
                m.values_mut().any(|c| walk(c, token_id))
            }
            Value::Array(a) => a.iter_mut().any(|c| walk(c, token_id)),
            _ => false,
        }
    }
    let path = state.hub().join("tokens.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read tokens.json")).expect("tokens.json is JSON");
    assert!(walk(&mut doc, token_id), "the token is in the store");
    std::fs::write(&path, serde_json::to_string(&doc).expect("serialize")).expect("write tokens.json");
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

#[test]
fn expired_token_rejection_and_lockout_trip_are_logged_by_default() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_LOCKOUT_MAX_FAILURES", "3"), ("HOLLER_DEBUG", "none")]);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    expire_token(&state, &token_id);
    let config = support::write_sessions_toml(&state, &[("alpha", &[])]);

    // Each body start is one rejected authentication; the third trips the lockout.
    for n in 1..=3 {
        let body = Body::start(&state, &config);
        wait_for(STARTUP_WAIT, || (count(&hub.log_text(), "auth_rejected") >= n).then_some(()))
            .unwrap_or_else(|| panic!("rejection #{n} was never logged:\n{}", hub.log_text()));
        drop(body);
    }
    wait_for(Duration::from_secs(10), || hub.log_text().contains("lockout_tripped").then_some(()))
        .unwrap_or_else(|| panic!("the lockout trip was never logged:\n{}", hub.log_text()));

    let log = hub.log_text();
    assert!(log.contains(r#""reason":"token_expired""#), "rejections carry a stable reason code:\n{log}");
    assert!(log.contains(&format!(r#""token_id":"{token_id}""#)), "rejections name the token id:\n{log}");
    assert!(
        log.contains(r#""failures":"1/3""#) && log.contains(r#""failures":"3/3""#),
        "the failure count in the window is logged:\n{log}"
    );
    assert_eq!(count(&log, r#""type":"lockout_tripped""#), 1, "exactly one trip line:\n{log}");
    assert!(log.contains("token_expiredx3"), "the trip line names the reasons that caused it:\n{log}");
    assert!(!log.contains(&secret), "no secret ever appears in a log line");
}
