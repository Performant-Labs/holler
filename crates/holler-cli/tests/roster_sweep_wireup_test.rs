#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #255
//! Issue #255: `Roster::sweep()` must actually run **inside the production
//! hub process**, not just against the unit tests' injected clock
//! (`crates/holler-hub/tests/roster_test.rs`). This drives a real `holler hub
//! serve` + a real `holler body run`, freezes the body with `SIGSTOP` (the
//! same real mechanism `talk_test.rs`'s issue #243 coverage uses — the TCP
//! connection stays fully established, no FIN/RST, so only the *absence of
//! traffic* can move the roster), and asserts the row ages
//! `connected → reconnecting → gone` purely by watching
//! `holler roster --json` over time. The test never calls `.sweep()` itself —
//! that would only prove the algorithm (already covered by the unit tests),
//! not that the hub actually wired the periodic task up.
//!
//! The hub's own connection-level liveness timeout (issue #243,
//! `circuit.rs`'s `liveness_timeout()`, default 3× the heartbeat) would
//! *also* eventually tear the connection down and mark the row `gone`
//! immediately (`Roster::clear`, issue #80) — a different code path than the
//! TTL sweep this story wires up. `HOLLER_HUB_LIVENESS_TIMEOUT_MS` is set far
//! longer than this test's whole run so that path never fires, and only the
//! roster's own (heavily shortened via `HOLLER_ROSTER_*_MS`) TTL thresholds,
//! swept on a short `HOLLER_ROSTER_SWEEP_MS` interval, can be what moves the
//! row.

mod support;

use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, join, kill_tree, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir};

/// `holler roster --all --json` (the `--all` the default `roster_json` helper
/// omits — needed here because a `gone` row is hidden from the default
/// listing).
fn roster_all_json(state: &StateDir) -> Value {
    let out = holler_cmd(state)
        .args(["roster", "--all", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `holler roster --all --json`");
    assert!(
        out.status.success(),
        "`holler roster --all --json` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("roster --all --json is valid JSON")
}

fn conn_state_of<'a>(v: &'a Value, name: &str) -> Option<&'a str> {
    v.get("rows")?
        .as_array()?
        .iter()
        .find(|r| r.get("name").and_then(Value::as_str) == Some(name))?
        .get("conn_state")?
        .as_str()
}

/// A roster row's `name` is `"<token label>/<session name>"`, not the bare
/// session name — `roster_all_json`'s `rows[].name` field for the session
/// `"alpha"` minted under the `"body-1"` label below is `"body-1/alpha"`.
const ALPHA_ROW: &str = "body-1/alpha";

#[test]
fn roster_sweep_runs_in_production_hub_without_being_called_directly() {
    let hub_state = StateDir::new();

    // Shorten every roster TTL threshold and the sweep's own interval so the
    // test observes the full connected → reconnecting → gone journey in a
    // few seconds of wall time; keep `prune` far enough out that the row is
    // still present (just `gone`) when we assert on it. `Config::from_env`
    // truncates each `HOLLER_ROSTER_*_MS` down to *whole seconds*
    // (`env_ms(..) / 1000`), so every threshold below is a multiple of
    // 1000ms — a sub-second override (e.g. 400ms) truncates to 0s, which
    // would make the row `reconnecting` instantly and defeat the point of
    // observing a journey. Set the hub's *connection-level* liveness timeout
    // (issue #243) far longer than this test's run so that separate
    // teardown path never fires and only the TTL sweep this story wires up
    // can move the row.
    let hub = Hub::start_with_env(
        &hub_state,
        &[
            ("HOLLER_HUB_LIVENESS_TIMEOUT_MS", "60000"),
            ("HOLLER_ROSTER_SWEEP_MS", "200"),
            ("HOLLER_ROSTER_RECONNECT_MS", "1000"),
            ("HOLLER_ROSTER_GONE_MS", "3000"),
            ("HOLLER_ROSTER_PRUNE_MS", "30000"),
        ],
    );
    let (token_id, secret) = mint_token(&hub_state, "body-1");
    join(&hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&hub_state, &[("alpha", &[])]);
    let mut body = Body::start(&hub_state, &config);

    // Observe the row appear `connected` via the body's own presence
    // heartbeat (never a blind sleep, ADR 0002).
    wait_for(Duration::from_secs(15), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, ALPHA_ROW) == Some("connected")).then_some(())
    })
    .expect("the roster shows `alpha` connected before the freeze");

    // Freeze the body's own process (SIGSTOP, not `kill_tree`): every thread
    // stops, including the presence heartbeat, but the kernel-level TCP
    // connection stays fully established — the same real mechanism
    // `talk_test.rs`'s `body_frozen_without_close_is_connection_lost_within_
    // bounded_time` uses for issue #243. From here, only the TTL sweep
    // (driven purely by wall-clock time since `last_seen`, never by any
    // frame the frozen body could send) can move the row.
    let pid = body.child_mut().id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGSTOP);
    }

    let sweep_result = (|| {
        wait_for(Duration::from_secs(10), || {
            let v = roster_all_json(&hub_state);
            (conn_state_of(&v, ALPHA_ROW) == Some("reconnecting")).then_some(())
        })?;
        wait_for(Duration::from_secs(10), || {
            let v = roster_all_json(&hub_state);
            (conn_state_of(&v, ALPHA_ROW) == Some("gone")).then_some(())
        })
    })();

    // Resume then reap the frozen body before any assertion below can panic
    // and skip cleanup (a `SIGSTOP`ped process is otherwise left behind).
    unsafe {
        libc::kill(pid, libc::SIGCONT);
    }
    kill_tree(body.child_mut());

    assert!(
        sweep_result.is_some(),
        "the roster row for `alpha` must age connected → reconnecting → gone \
         purely from the production hub's own background sweep task, within \
         the shortened TTL thresholds — it never did, so the sweep is not \
         actually wired into the running hub"
    );

    hub.stop(Duration::from_secs(10));
}
