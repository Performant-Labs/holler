#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #142
//! RED tests for `holler wait` (issue #142), against real hub + body +
//! `stub-acp` (#130) subprocesses — no mocks of the circuit. Mirrors
//! `answer_cli_test.rs`'s / `talk_test.rs`'s own setup pattern.
//!
//! # Coverage (the issue's own RED list, verbatim in name where practical)
//!
//! - `wait` returns immediately when a session already matches (edge-
//!   triggered, no hub-held watermark).
//! - `--after TURN_ID` does not re-match the turn already reported; a second
//!   call blocks until a *new* turn settles.
//! - The default `--until` set ignores bare `idle` but fires on
//!   `input-required` (a held permission gate).
//! - `wait` fires on `gone` the moment a body's connection drops.
//! - Two named sessions: `wait` returns on the *first* one to match, without
//!   waiting for the other.
//! - A real timeout is exit 2; no live hub is exit 1.
//! - `wait_fires_on_failed_when_stub_crashes` is `#[ignore]`d: it depends on
//!   the exact same upstream `agent-client-protocol` crash-detection defect
//!   `acp_driver_crash_test.rs`'s `crash_mid_turn_is_error_not_hang` and
//!   `session_manager_test.rs`'s `driver_crash_isolated_and_restarts_on_next_
//!   prompt` are already `#[ignore]`d for (see those files' own module docs
//!   for the full investigation) — the crashed child's stdout EOF never
//!   wakes the SDK's own crash watcher, so the session never reaches
//!   `Failed` on its own; it only resolves via a long fixed timeout. Adding a
//!   *third*, un-ignored test with a hard dependency on that same broken
//!   mechanism would introduce a new flake/hang into this suite, not prove
//!   anything the other two don't already document. Left in (not deleted) so
//!   the assertion is ready the moment that upstream defect is fixed.
//!
//! `wait_uses_no_polling` (the issue's own last item) is **not** here: the
//! property it proves (how many times the roster's row table was read during
//! a real wait) is an in-process fact with no wire-level observable, so it
//! lives as a unit test directly against `holler-hub`'s `control_server::wait`
//! (`crates/holler-hub/src/control_server.rs`'s own `wait_tests` module).

mod support;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::{join, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir};

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}
fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `holler wait ARGS...` against a bare state-dir path (not a `&StateDir`) —
/// the shape a background thread needs, since `StateDir`'s `Drop` removes its
/// directory and the owning `StateDir` must stay alive on the test's main
/// thread for the whole test. Mirrors `talk_test.rs`'s own `say_full`.
fn wait_full(state_path: &Path, args: &[&str]) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .arg("wait")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `wait`")
}

/// `say SESSION TEXT` against a bare state-dir path — same reason as
/// [`wait_full`].
fn say_full(state_path: &Path, session: &str, text: &str) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["say", session, text])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

/// A roster row's `name` matches a bare `session` when it is exactly that or
/// nests under a label boundary (`.../<session>`) — same helper
/// `answer_cli_test.rs` uses (roster rows are always qualified `<label>/
/// <session>`, but `wait`/`say`/`answer` all accept the bare name).
fn row_name_matches(row: &serde_json::Value, session: &str) -> bool {
    match row.get("name").and_then(|n| n.as_str()) {
        Some(name) => name == session || name.ends_with(&format!("/{session}")),
        None => false,
    }
}

/// Same matching rule as [`row_name_matches`], but for `wait`'s own result
/// shape (`{"session": "<label>/<session>", ...}` — see `control_server.rs`'s
/// `row_to_json`), which keys the field `session`, not `name`.
fn wait_row_matches(row: &serde_json::Value, session: &str) -> bool {
    match row.get("session").and_then(|n| n.as_str()) {
        Some(name) => name == session || name.ends_with(&format!("/{session}")),
        None => false,
    }
}

/// Same setup `talk_test.rs`/`answer_cli_test.rs` use: mint a token against
/// `hub_state`, join `body_state`, and start `holler body run`.
fn start_body(hub_state: &StateDir, body_state: &StateDir, hub: &Hub, sessions: &[(&str, &[&str])]) -> Body {
    let (token_id, secret) = mint_token(hub_state, "b");
    join(body_state, hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, sessions);
    Body::start(body_state, &config)
}

/// Poll the roster until every name in `sessions` appears (any state) — the
/// same "wait for the body's first presence to land" guard
/// `answer_cli_test.rs::wait_until_session_present` uses, generalised to more
/// than one name so the two-session test can wait on both at once.
fn wait_until_sessions_present(state: &StateDir, sessions: &[&str], timeout: Duration) {
    wait_for(timeout, || {
        let doc = support::roster_json(state);
        let rows = doc.get("rows")?.as_array()?.clone();
        sessions
            .iter()
            .all(|s| rows.iter().any(|r| row_name_matches(r, s)))
            .then_some(())
    })
    .unwrap_or_else(|| panic!("{sessions:?} never all appeared in the roster within {timeout:?}"));
}

/// The qualified `turn_id` currently on `session`'s roster row (or `None`
/// before its first turn).
fn roster_turn_id(state: &StateDir, session: &str) -> Option<String> {
    let doc = support::roster_json(state);
    let rows = doc.get("rows")?.as_array()?.clone();
    let row = rows.into_iter().find(|r| row_name_matches(r, session))?;
    row.get("turn_id").and_then(|v| v.as_str()).map(str::to_string)
}

#[test]
fn wait_returns_immediately_if_already_matching() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    let said = support::say(&hub_state, "alpha", "hi");
    assert!(said.status.success(), "stderr: {}", stderr_of(&said));

    // The turn already settled `completed` before `wait` even starts — this
    // must return right away, not park for anywhere near its own timeout.
    let started = Instant::now();
    let out = support::wait_cmd(&hub_state, &["alpha", "--timeout", "20s", "--json"]);
    let elapsed = started.elapsed();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert!(elapsed < Duration::from_secs(5), "must return immediately (edge-triggered), took {elapsed:?}");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    assert_eq!(doc["matched"], serde_json::json!(true));
    let rows = doc["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["last_turn"]["state"].as_str(), Some("completed"));
}

#[test]
fn wait_with_after_does_not_rematch_same_turn() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    let said = support::say(&hub_state, "alpha", "first");
    assert!(said.status.success(), "stderr: {}", stderr_of(&said));
    let turn1 = roster_turn_id(&hub_state, "alpha").expect("a turn_id after the first say");

    // A second `wait --after turn1` must NOT match on the already-reported
    // turn: run it on a background thread and prove it is still blocked a
    // moment later.
    let path = hub_state.path().to_path_buf();
    let turn1_clone = turn1.clone();
    let handle = std::thread::spawn(move || wait_full(&path, &["alpha", "--after", &turn1_clone, "--timeout", "30s", "--json"]));

    std::thread::sleep(Duration::from_millis(500));
    assert!(!handle.is_finished(), "must not re-match the same turn_id it was watermarked on");

    let said2 = support::say(&hub_state, "alpha", "second");
    assert!(said2.status.success(), "stderr: {}", stderr_of(&said2));

    let ready = wait_for(Duration::from_secs(20), || handle.is_finished().then_some(()));
    assert!(ready.is_some(), "wait --after must return once a NEW turn settles");
    let out = handle.join().expect("wait thread");
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    let turn2 = doc["rows"][0]["turn_id"].as_str().expect("a turn_id on the match");
    assert_ne!(turn2, turn1, "must be the NEW turn, not the watermarked one");
}

#[test]
fn wait_default_set_ignores_bare_idle_but_fires_on_input_required() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-permission", "--chunks", "2"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    // Idle at start; the default --until set must NOT fire on it.
    let out = support::wait_cmd(&hub_state, &["alpha", "--timeout", "1s"]);
    assert_eq!(out.status.code(), Some(2), "bare idle must not match the default set; stderr: {}", stderr_of(&out));

    // Now gate the session (in the background) and prove the SAME default
    // set fires once it reaches input-required.
    let path = hub_state.path().to_path_buf();
    let gate_handle = std::thread::spawn(move || say_full(&path, "alpha", "please run the tool"));

    let out = support::wait_cmd(&hub_state, &["alpha", "--timeout", "10s", "--json"]);
    assert!(out.status.success(), "must fire on input-required; stderr: {}", stderr_of(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    assert_eq!(doc["rows"][0]["state"].as_str(), Some("input-required"));

    let _ = support::answer(&hub_state, "alpha", "allow");
    let _ = gate_handle.join();
}

#[test]
fn wait_on_two_sessions_returns_first_match_only() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(
        &hub_state,
        &body_state,
        &hub,
        &[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])],
    );
    wait_until_sessions_present(&hub_state, &["alpha", "beta"], Duration::from_secs(10));

    let path = hub_state.path().to_path_buf();
    let handle = std::thread::spawn(move || {
        let started = Instant::now();
        let out = wait_full(&path, &["alpha,beta", "--timeout", "20s", "--json"]);
        (out, started.elapsed())
    });

    std::thread::sleep(Duration::from_millis(300));
    assert!(!handle.is_finished(), "neither session has completed a turn yet");

    let said = support::say(&hub_state, "alpha", "only alpha runs");
    assert!(said.status.success(), "stderr: {}", stderr_of(&said));

    let ready = wait_for(Duration::from_secs(15), || handle.is_finished().then_some(()));
    assert!(ready.is_some(), "wait never returned after alpha's turn settled");
    let (out, elapsed) = handle.join().expect("wait thread");

    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert!(elapsed < Duration::from_secs(10), "must return on the first match, not wait out the full timeout: {elapsed:?}");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    let rows = doc["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1, "only the session that actually matched is reported: {rows:?}");
    assert!(wait_row_matches(&rows[0], "alpha"), "the match must be alpha, not beta: {rows:?}");
}

#[test]
fn wait_timeout_exit_2() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    // Idle, never prompted — nothing in the default --until set ever fires.
    let started = Instant::now();
    let out = support::wait_cmd(&hub_state, &["alpha", "--timeout", "1s"]);
    let elapsed = started.elapsed();
    assert_eq!(out.status.code(), Some(2), "a real timeout with no match is exit 2; stderr: {}", stderr_of(&out));
    assert!(elapsed >= Duration::from_millis(900), "must actually wait out the timeout, not return early: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "must not overrun its own timeout by much: {elapsed:?}");
    assert!(stdout_of(&out).is_empty(), "a timeout prints nothing on stdout");
}

#[test]
fn wait_no_hub_exit_1() {
    let state = StateDir::new();
    // No `Hub::start` at all — the control socket does not exist.
    let out = support::wait_cmd(&state, &["alpha"]);
    assert_eq!(out.status.code(), Some(1), "no live hub must be exit 1; stderr: {}", stderr_of(&out));
    assert!(
        stderr_of(&out).contains("no live holler hub reachable"),
        "must carry the spec's exact wording: {}",
        stderr_of(&out)
    );
}

#[test]
fn wait_fires_on_gone_when_body_detaches() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    // Issue #192 changed a hard kill's own immediate effect: an abrupt drop
    // (socket read error/EOF, no clean WS close frame) now marks the row
    // `reconnecting`, not `gone` — only an explicit `body detach` goes
    // straight to `gone`. `gone` is now reached only via the roster's own
    // TTL sweep (issue #255), so this test shortens both thresholds (and the
    // sweep's own interval) the same way `roster_sweep_wireup_test.rs` does,
    // rather than expecting an immediate transition a hard kill no longer
    // produces.
    let hub = Hub::start_with_env(
        &hub_state,
        &[
            ("HOLLER_ROSTER_SWEEP_MS", "200"),
            ("HOLLER_ROSTER_RECONNECT_MS", "1000"),
            ("HOLLER_ROSTER_GONE_MS", "1000"),
        ],
    );
    let mut body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    let path = hub_state.path().to_path_buf();
    let handle = std::thread::spawn(move || wait_full(&path, &["alpha", "--until", "gone", "--timeout", "20s", "--json"]));

    std::thread::sleep(Duration::from_millis(300));
    assert!(!handle.is_finished(), "the body is still live; must not have matched yet");

    // A hard kill (not a graceful `body detach`) drops the socket out from
    // under the hub; the row ages `connected` → `reconnecting` → `gone`
    // purely via the roster's shortened TTL sweep above.
    support::kill_tree(body.child_mut());

    let ready = wait_for(Duration::from_secs(15), || handle.is_finished().then_some(()));
    assert!(ready.is_some(), "wait --until gone never returned after the body was killed");
    let out = handle.join().expect("wait thread");
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    assert_eq!(doc["rows"][0]["conn_state"].as_str(), Some("gone"));
}

/// See this file's module doc: depends on the same upstream `agent-client-
/// protocol` crash-detection defect `acp_driver_crash_test.rs`'s
/// `crash_mid_turn_is_error_not_hang` is already `#[ignore]`d for. Confirmed
/// via that file's own investigation, not re-derived here.
#[test]
#[ignore = "depends on the same upstream agent-client-protocol crash-detection defect as \
    acp_driver_crash_test.rs's crash_mid_turn_is_error_not_hang (issue #188) — the crashed \
    child's stdout EOF never wakes the SDK's own crash watcher, so the session never reaches \
    Failed on its own within a bounded time. See that file's module doc for the full \
    investigation. Re-run with --ignored once that upstream defect is fixed."]
fn wait_fires_on_failed_when_stub_crashes() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--crash-after-prompt", "--chunks", "3"])]);
    wait_until_sessions_present(&hub_state, &["alpha"], Duration::from_secs(10));

    let path = hub_state.path().to_path_buf();
    let handle = std::thread::spawn(move || wait_full(&path, &["alpha", "--until", "failed", "--timeout", "60s", "--json"]));

    // The crashing prompt: expected to fail once the driver observes the
    // crash (blocked upstream — see the `#[ignore]` reason above).
    let _ = support::say(&hub_state, "alpha", "crash please");

    let out = handle.join().expect("wait thread");
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("wait --json is valid JSON");
    assert_eq!(doc["rows"][0]["last_turn"]["state"].as_str(), Some("failed"));
}
