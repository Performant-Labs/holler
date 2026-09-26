#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #450
//! Issue #450: a rejected `circuit/authenticate` and the lockout it feeds are
//! visible in the hub's **default-level** log, with a stable reason code. The
//! incident this comes from: an expired token made a restarting body trip the
//! failed-auth lockout for every client behind the proxy, and the hub log said
//! only `lockout_refused` — no reason, no trip line — so diagnosis took an hour.
//! Since #453 a bound token outlives `expires`, so the reproduction below
//! uses a revoked token instead.
//!
//! The hub runs at `HOLLER_DEBUG=none`, the real default: `Warn` events are
//! never gated by the debug level, so a security-relevant rejection needs no
//! `--debug noisy` to appear. Real hub and real bodies (no mocks of the circuit). Readiness is observed
//! through the hub's own log via [`support::wait_for`], never slept for.

mod support;

use std::time::Duration;

use support::{join, mint_token, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

#[test]
fn rejected_token_and_lockout_trip_are_logged_by_default() {
    let state = StateDir::new();
    let hub = Hub::start_with_env(&state, &[("HOLLER_LOCKOUT_MAX_FAILURES", "3"), ("HOLLER_DEBUG", "none")]);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let out = support::holler_cmd(&state).args(["hub", "token", "revoke", &token_id]).output().expect("run hub token revoke");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
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
    assert!(log.contains(r#""reason":"token_not_bound""#), "rejections carry a stable reason code:\n{log}");
    assert!(log.contains(&format!(r#""token_id":"{token_id}""#)), "rejections name the token id:\n{log}");
    assert!(
        log.contains(r#""failures":"1/3""#) && log.contains(r#""failures":"3/3""#),
        "the failure count in the window is logged:\n{log}"
    );
    assert_eq!(count(&log, r#""type":"lockout_tripped""#), 1, "exactly one trip line:\n{log}");
    assert!(log.contains("token_not_boundx3"), "the trip line names the reasons that caused it:\n{log}");
    assert!(!log.contains(&secret), "no secret ever appears in a log line");
}
