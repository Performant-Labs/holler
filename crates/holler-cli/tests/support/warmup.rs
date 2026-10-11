//! Warm-up readiness (issue #420): [`wait_warm`] and the two pure helpers it is
//! built on, split out of `mod.rs` like `cmds`/`onboard` to keep that file under
//! the 900-line build guard.
//!
//! The per-file `say_ready` shortcut this replaces returned any `say` failure
//! other than `unknown session` as if the session were ready, so a transient
//! `reconnecting` reply surfaced later as a bare assertion with no logs. A
//! `connected` roster row is not enough on its own either: the hub writes the row
//! before it caches the presence `say` resolves against, so `unknown session` can
//! still follow it briefly. [`wait_warm`] therefore waits for the row, then
//! retries **only** `unknown session`, and fails at once on anything else.
//!
//! A failure carries its evidence, the body's own output included. [`Body`]
//! keeps that output in `<state>/body.log` ([`Body::log_path`]), outside the
//! production `hub/` and `body/` subtrees. It can share a dir with a hub's files
//! when a test passes the hub's own `StateDir` to `Body::start`
//! (`roster_sweep_wireup_test` does).

use std::process::Output;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::{try_roster_json, wait_for, Body, Hub, StateDir};

/// True when `roster` (a `holler roster --json` document) has a `connected` row
/// for `session`, named either exactly `session` or `<label>/<session>`.
///
/// The bare-or-`label/session` match is the same idiom as `wait_row_matches` in
/// `wait_test.rs` (a different schema, keyed `session`, so the two are not
/// shared). A bare name matches that session under any label, which is
/// sufficient with one body per test.
pub fn roster_row_connected(roster: &Value, session: &str) -> bool {
    let Some(rows) = roster["rows"].as_array() else {
        return false;
    };
    let qualified = format!("/{session}");
    rows.iter().any(|row| {
        let name = row["name"].as_str().unwrap_or_default();
        (name == session || name.ends_with(&qualified)) && row["conn_state"].as_str() == Some("connected")
    })
}

/// True when a failed warm-up attempt's stderr is `unknown session`, the one
/// transient failure while a body comes up (see the module doc). Every other
/// failure, `reconnecting` and `not connected` included, is real.
pub fn say_failure_is_retryable(stderr: &str) -> bool {
    stderr.contains("unknown session")
}

/// Wait for `session` to be warm and return `attempt`'s successful output.
///
/// One deadline, `timeout`, covers both phases, each polled with [`wait_for`]:
/// 1. [`try_roster_json`] until [`roster_row_connected`]; an `Err` counts as
///    "not yet" and is kept for the report;
/// 2. `attempt` (a `say`, or another first prompt) until it succeeds, retrying
///    only a [`say_failure_is_retryable`] failure.
///
/// Any other failure, and running out of time in either phase, panics with the
/// cause, then `roster:` (the last roster read, JSON or its error), `hub log:`
/// and `body log:`, so a CI failure carries its own evidence. It never returns a
/// failed `Output`.
pub fn wait_warm(
    state: &StateDir,
    session: &str,
    timeout: Duration,
    hub: &Hub,
    body: Option<&Body>,
    attempt: impl FnMut() -> Output,
) -> Output {
    let deadline = Instant::now() + timeout;
    let mut roster = String::from("<no roster read>");
    let connected = wait_for(timeout, || {
        read_roster(state, &mut roster).filter(|doc| roster_row_connected(doc, session))
    });
    if connected.is_none() {
        let cause = format!("`{session}` had no connected roster row within {timeout:?}");
        warm_panic(&cause, &roster, hub, body);
    }
    match attempt_until_ok(deadline, attempt) {
        Ok(out) => out,
        Err(cause) => {
            // The last read showed the row connected; report it as the failure left it.
            read_roster(state, &mut roster);
            warm_panic(&format!("`{session}` {cause}"), &roster, hub, body)
        }
    }
}

/// One roster read, recorded in `last` (the JSON, or why it could not be read)
/// for a failure report.
fn read_roster(state: &StateDir, last: &mut String) -> Option<Value> {
    match try_roster_json(state) {
        Ok(doc) => {
            *last = doc.to_string();
            Some(doc)
        }
        Err(e) => {
            *last = format!("<unreadable> {e}");
            None
        }
    }
}

/// Run `attempt` until it succeeds or `deadline` passes (at least once),
/// retrying only an `unknown session` failure. The `Err` says what ended it.
fn attempt_until_ok(deadline: Instant, mut attempt: impl FnMut() -> Output) -> Result<Output, String> {
    let mut last_stderr = String::new();
    let budget = deadline.saturating_duration_since(Instant::now());
    let settled = wait_for(budget, || {
        let out = attempt();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        if out.status.success() {
            Some(Ok(out))
        } else if say_failure_is_retryable(&stderr) {
            last_stderr = stderr;
            None
        } else {
            Some(Err(format!(
                "attempt failed ({}), not retryable; stderr: {}; stdout: {}",
                out.status,
                stderr.trim_end(),
                String::from_utf8_lossy(&out.stdout).trim_end()
            )))
        }
    });
    settled.unwrap_or_else(|| {
        let stderr = last_stderr.trim_end();
        Err(format!("attempt still failed with `unknown session` at the deadline; stderr: {stderr}"))
    })
}

/// Panic with `cause` and its evidence: `roster:`, `hub log:`, `body log:`.
fn warm_panic(cause: &str, roster: &str, hub: &Hub, body: Option<&Body>) -> ! {
    let body_log = body.map_or_else(|| " <no body handle>".to_string(), |b| format!("\n{}", b.log_text()));
    panic!("wait_warm: {cause}\nroster: {roster}\nhub log:\n{}\nbody log:{body_log}", hub.log_text())
}
