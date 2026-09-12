#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #191
//! RED tests for `interrupt` → `session/cancel` (+ the `interrupt SESSION
//! TEXT` redirect) (issue #191), the issue's own test list, verbatim in
//! name where practical, against real hub + body + `stub-acp` (#130)
//! subprocesses — no mocks of the circuit.
//!
//! Helper split from `talk_test.rs`: this file needs its own
//! `start_body`/`say_ready`/background-thread helpers (rather than `use
//! talk_test::*`, which is not a thing between two independent integration
//! test binaries), so they are duplicated here in the same shape.

mod support;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use support::{join, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir};

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}
fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn say_full(state_path: &Path, args: &[&str]) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .arg("say")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

/// `interrupt` with the full flag surface this file needs (a bare `SESSION`
/// or `SESSION TEXT`) — `support::interrupt` only covers the bare form.
fn interrupt_full(state_path: &Path, args: &[&str]) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .arg("interrupt")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `interrupt`")
}

fn say_full_in_background(
    state: &StateDir,
    args: Vec<String>,
    sleep_before_racing: Duration,
) -> std::thread::JoinHandle<Output> {
    let state_path = state.path().to_path_buf();
    let handle = std::thread::spawn(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        say_full(&state_path, &args)
    });
    std::thread::sleep(sleep_before_racing);
    handle
}

fn owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

fn start_body(hub_state: &StateDir, body_state: &StateDir, hub: &Hub, sessions: &[(&str, &[&str])]) -> Body {
    start_body_labeled(hub_state, body_state, hub, "b", sessions)
}

fn start_body_labeled(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &Hub,
    label: &str,
    sessions: &[(&str, &[&str])],
) -> Body {
    start_body_with_env(hub_state, body_state, hub, label, sessions, &[])
}

fn start_body_with_env(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &Hub,
    label: &str,
    sessions: &[(&str, &[&str])],
    envs: &[(&str, &str)],
) -> Body {
    let (token_id, secret) = mint_token(hub_state, label);
    join(body_state, hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, sessions);
    Body::start_with_env(body_state, &config, envs)
}

/// Poll `say session TEXT` until it stops failing with `unknown session`.
fn say_ready(state: &StateDir, session: &str, text: &str, timeout: Duration) -> Output {
    wait_for(timeout, || {
        let out = support::say(state, session, text);
        if out.status.success() || !stderr_of(&out).contains("unknown session") {
            Some(out)
        } else {
            None
        }
    })
    .unwrap_or_else(|| panic!("`say {session}` never got past unknown_session within {timeout:?}"))
}

#[test]
fn interrupt_cancels_only_target_sibling_completes() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(
        &hub_state,
        &body_state,
        &hub,
        &[("alpha", &["--slow", "--chunks", "10"]), ("beta", &["--chunks", "3"])],
    );

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let alpha_handle =
        say_full_in_background(&hub_state, owned(&["alpha", "a long turn"]), Duration::from_millis(200));
    let interrupted = support::interrupt(&hub_state, "alpha");
    assert_eq!(interrupted.status.code(), Some(0), "stderr: {}", stderr_of(&interrupted));

    let alpha_out = alpha_handle.join().expect("alpha say thread");
    assert_eq!(alpha_out.status.code(), Some(1), "the interrupted say must exit 1; stderr: {}", stderr_of(&alpha_out));

    // Beta, sharing the same connection, is untouched: its own say completes
    // normally.
    let beta_out = support::say(&hub_state, "beta", "hi beta");
    assert!(beta_out.status.success(), "beta's own say must still succeed; stderr: {}", stderr_of(&beta_out));
    assert!(stdout_of(&beta_out).contains("stub chunk"));

    // No reconnect: the hub still shows exactly one live client for this body.
    let status = support::hub_status_json(&hub_state);
    assert_eq!(status["clients"].as_u64(), Some(1), "the connection must survive the interrupt: {status}");
}

#[test]
fn interrupted_say_exits_1_with_clear_message() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(&hub_state, owned(&["alpha", "a long turn"]), Duration::from_millis(200));
    let interrupted = support::interrupt(&hub_state, "alpha");
    assert_eq!(interrupted.status.code(), Some(0), "stderr: {}", stderr_of(&interrupted));

    let out = handle.join().expect("say thread");
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(
        err.contains("prompt was interrupted before it completed"),
        "must carry the spec's exact wording: {err:?}"
    );
}

#[test]
fn session_accepts_new_prompt_immediately_after_interrupt_and_reply_is_fresh() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(&hub_state, owned(&["alpha", "a long turn"]), Duration::from_millis(200));
    let interrupted = support::interrupt(&hub_state, "alpha");
    assert_eq!(interrupted.status.code(), Some(0), "stderr: {}", stderr_of(&interrupted));
    let _ = handle.join();

    // The session must accept a fresh prompt immediately — no lingering
    // session_busy from the cancelled turn.
    let fresh = wait_for(Duration::from_secs(5), || {
        let out = support::say(&hub_state, "alpha", "fresh prompt");
        out.status.success().then_some(out)
    })
    .unwrap_or_else(|| panic!("session did not accept a fresh prompt promptly after interrupt"));
    assert!(stdout_of(&fresh).contains("stub chunk"), "the reply must be a real, fresh one: {:?}", stdout_of(&fresh));
}

#[test]
fn interrupt_with_text_cancels_then_prompts_and_returns_new_reply() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle =
        say_full_in_background(&hub_state, owned(&["alpha", "the turn about to be cancelled"]), Duration::from_millis(200));
    let redirected = interrupt_full(hub_state.path(), &["alpha", "the redirect text"]);
    assert_eq!(redirected.status.code(), Some(0), "stderr: {}", stderr_of(&redirected));
    let reply = stdout_of(&redirected);
    assert!(reply.contains("stub chunk"), "the redirect's own reply must arrive: {reply:?}");

    let cancelled_out = handle.join().expect("cancelled say thread");
    assert_eq!(cancelled_out.status.code(), Some(1), "the original turn must report interrupted; stderr: {}", stderr_of(&cancelled_out));
}

#[test]
fn interrupt_with_text_on_idle_session_just_prompts() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "2"])]);

    let out = wait_for(Duration::from_secs(10), || {
        let out = interrupt_full(hub_state.path(), &["alpha", "hello, idle session"]);
        (out.status.success() || !stderr_of(&out).contains("unknown session")).then_some(out)
    })
    .expect("interrupt with text on an idle session eventually succeeds");
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert!(stdout_of(&out).contains("stub chunk"), "must just prompt and reply: {:?}", stdout_of(&out));
}

#[test]
fn interrupt_idle_session_is_ok_noop() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);

    // Warm the session up once so it is a real, known, idle session (rather
    // than racing the hub's very first presence).
    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let out = support::interrupt(&hub_state, "alpha");
    assert_eq!(out.status.code(), Some(0), "an idle session's cancel must be a no-op success; stderr: {}", stderr_of(&out));
    assert!(stdout_of(&out).contains("interrupted alpha"), "got: {:?}", stdout_of(&out));
}

#[test]
fn interrupt_unknown_session_exit_1() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    let _ = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));

    let out = support::interrupt(&hub_state, "no-such-session");
    assert_eq!(out.status.code(), Some(1), "unknown session must exit 1; stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("unknown session") || err.contains("no-such-session"), "got: {err:?}");
}

/// Issue #191: `--ignore-cancel` (this story's own addition to the stub)
/// makes the agent never resolve a `session/cancel` at all, so
/// `AcpDriver::cancel()`'s own 5s `CANCEL_TIMEOUT` is what eventually fires
/// on the body — but the hub's own RTT-scaled ack timeout (the 2s floor,
/// with no prior measured RTT) fires first, and that is what `interrupt`
/// must report.
#[test]
fn ack_timeout_message_when_body_stalls() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_body(
        &hub_state,
        &body_state,
        &hub,
        &[("alpha", &["--slow", "--chunks", "20", "--ignore-cancel"])],
    );

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "30s", "alpha", "a turn the body will never confirm cancelling"]),
        Duration::from_millis(200),
    );

    let started = Instant::now();
    let out = support::interrupt(&hub_state, "alpha");
    let elapsed = started.elapsed();

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("not confirmed within"), "must carry the spec's own wording: {err:?}");
    assert!(err.contains("still on the roster"), "must carry the spec's own wording: {err:?}");
    assert!(
        elapsed < Duration::from_secs(5),
        "the RTT-scaled ack timeout (2s floor) must fire well before the body's own 5s CANCEL_TIMEOUT; took {elapsed:?}"
    );

    // Cleanup: the body will eventually (≤5s) force-kill the stalled child
    // and settle the original say as some kind of failure; join and stop
    // everything rather than leaving a lingering process behind.
    let _ = handle.join();
    body.stop(&body_state, Duration::from_secs(10));
}

#[test]
fn cancel_not_queued_behind_large_update_flush() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    // Every turn on this session streams 5000 chunks (stub-acp's `--chunks`
    // is a per-session config, applied to *every* prompt) — so even the
    // readiness probe below must never be a real `say` against this session
    // (a full round trip would itself take 5000 × the default 50ms/chunk
    // pacing ≈ 250s, which would trip the body's own *unrelated* 45s
    // liveness timeout long before this story's own interrupt ever runs).
    // Poll the roster instead: it observes presence without running a turn.
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "5000"])]);
    wait_for(Duration::from_secs(10), || {
        let rows = support::roster_json(&hub_state)["rows"].as_array()?.clone();
        rows.iter()
            .any(|r| r["name"].as_str() == Some("b/alpha") && r["conn_state"].as_str() == Some("connected"))
            .then_some(())
    })
    .expect("roster must observe alpha connected");

    let handle = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "600s", "alpha", "a turn streaming thousands of chunks"]),
        Duration::from_millis(150),
    );

    let started = Instant::now();
    let out = support::interrupt(&hub_state, "alpha");
    let elapsed = started.elapsed();

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_of(&out));
    assert!(
        elapsed < Duration::from_secs(1),
        "the cancel's own ack must never queue behind a large update flush; took {elapsed:?}"
    );

    let _ = handle.join();
}

/// Unit test (issue #191's own RED list): [`holler_hub::interrupt::ack_timeout`]
/// never reads the heartbeat interval — setting
/// `HOLLER_HEARTBEAT_INTERVAL_MS` to something absurd must not move its
/// output at all.
#[test]
fn interrupt_ack_timeout_is_independent_of_heartbeat() {
    // SAFETY-of-intent: `std::env::set_var`/`remove_var` on a process-global
    // env var in a `#[test]` is racy against other tests only if something
    // else reads this exact var concurrently; nothing else in this binary
    // reads `HOLLER_HEARTBEAT_INTERVAL_MS` (that var is read only by
    // `holler-hub`'s `circuit::heartbeat_interval`, in a live hub/body
    // subprocess this test does not spawn).
    let baseline = holler_hub::interrupt::ack_timeout(Some(Duration::from_millis(100)));

    std::env::set_var("HOLLER_HEARTBEAT_INTERVAL_MS", "999999999");
    let with_absurd_heartbeat = holler_hub::interrupt::ack_timeout(Some(Duration::from_millis(100)));
    std::env::remove_var("HOLLER_HEARTBEAT_INTERVAL_MS");

    assert_eq!(baseline, with_absurd_heartbeat, "ack_timeout must ignore the heartbeat interval entirely");
    // 10 * 100ms = 1s, below the 2s floor — the floor wins either way.
    assert_eq!(baseline, Duration::from_secs(2));

    // And the RTT-scaled branch (above the floor) is equally unaffected.
    let scaled_baseline = holler_hub::interrupt::ack_timeout(Some(Duration::from_millis(500)));
    std::env::set_var("HOLLER_HEARTBEAT_INTERVAL_MS", "1");
    let scaled_with_heartbeat = holler_hub::interrupt::ack_timeout(Some(Duration::from_millis(500)));
    std::env::remove_var("HOLLER_HEARTBEAT_INTERVAL_MS");
    assert_eq!(scaled_baseline, scaled_with_heartbeat);
    assert_eq!(scaled_baseline, Duration::from_secs(5));
}

/// The automated counterpart of the manual acceptance gate `hlr-1103`
/// (catalogued as `hlr-1107`: lifecycle, auto, smoke, alters-db) — the full
/// operator journey through the real CLI binary against the real stub.
#[test]
fn full_journey_with_stub() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);

    let (token_id, secret) = mint_token(&hub_state, "t");
    join(&body_state, &hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&body_state, &[("alpha", &["--slow", "--chunks", "10"]), ("beta", &["--chunks", "3"])]);
    let mut body = Body::start(&body_state, &config);
    let body_pid = body.child_mut().id();

    // `roster` shows both connected.
    wait_for(Duration::from_secs(10), || {
        let rows = support::roster_json(&hub_state)["rows"].as_array()?.clone();
        let connected: Vec<&str> = rows
            .iter()
            .filter(|r| r["conn_state"].as_str() == Some("connected"))
            .filter_map(|r| r["name"].as_str())
            .collect();
        (connected.contains(&"t/alpha") && connected.contains(&"t/beta")).then_some(())
    })
    .expect("roster must show both t/alpha and t/beta connected");

    // `say alpha` / `say beta` return the stub's text. (Bare names, not
    // `t/alpha`/`t/beta`: `body join` always advertises hostname `default`
    // — the mint `--label` qualifies the *roster's* display name, not the
    // routable `<label>/<session>` address `resolve_session` matches
    // against; with one body live, the bare name is unambiguous either
    // way.)
    let a = say_ready(&hub_state, "alpha", "hi alpha", Duration::from_secs(10));
    assert!(a.status.success(), "stderr: {}", stderr_of(&a));
    assert!(stdout_of(&a).contains("stub chunk"));
    let b = support::say(&hub_state, "beta", "hi beta");
    assert!(b.status.success(), "stderr: {}", stderr_of(&b));
    assert!(stdout_of(&b).contains("stub chunk"));

    // A long `say alpha` in the background + `interrupt alpha` → `beta`
    // unaffected → `say alpha` again returns a fresh reply.
    journey_interrupt_and_fresh_reply(&hub_state);

    // `body detach` → `roster --all` shows both gone, `hub status`
    // `clients:0`, no `stub-acp` process remains.
    body.stop(&body_state, Duration::from_secs(10));
    journey_verify_teardown(&hub_state, body_pid);
}

/// The middle third of [`full_journey_with_stub`]: interrupt the in-flight
/// `alpha` turn, confirm `beta` is untouched, and confirm `alpha` accepts a
/// fresh prompt right after. Split out purely to keep the whole scenario
/// under clippy's cognitive-complexity gate.
fn journey_interrupt_and_fresh_reply(hub_state: &StateDir) {
    let handle = say_full_in_background(hub_state, owned(&["alpha", "a long turn"]), Duration::from_millis(200));
    let interrupted = support::interrupt(hub_state, "alpha");
    assert_eq!(interrupted.status.code(), Some(0), "stderr: {}", stderr_of(&interrupted));
    let cancelled = handle.join().expect("alpha say thread");
    assert_eq!(cancelled.status.code(), Some(1), "stderr: {}", stderr_of(&cancelled));
    assert!(stderr_of(&cancelled).contains("interrupted"));

    let beta_again = support::say(hub_state, "beta", "still there?");
    assert!(beta_again.status.success(), "stderr: {}", stderr_of(&beta_again));

    let mut last_err = String::new();
    let fresh = wait_for(Duration::from_secs(5), || {
        let out = support::say(hub_state, "alpha", "fresh prompt");
        if out.status.success() {
            Some(out)
        } else {
            last_err = stderr_of(&out);
            None
        }
    })
    .unwrap_or_else(|| panic!("alpha must accept a fresh prompt after the interrupt; last err: {last_err}"));
    assert!(stdout_of(&fresh).contains("stub chunk"));
}

/// The final third of [`full_journey_with_stub`]: after `body.stop`, confirm
/// the hub observed the detach, `roster --all` shows both sessions `gone`,
/// and no `stub-acp` process remains. Split out for the same reason as
/// [`journey_interrupt_and_fresh_reply`].
fn journey_verify_teardown(hub_state: &StateDir, body_pid: u32) {
    wait_for(Duration::from_secs(10), || {
        let status = support::hub_status_json(hub_state);
        (status["clients"].as_u64() == Some(0)).then_some(())
    })
    .expect("hub must observe the body detach");

    let all_rows = support::holler_cmd(hub_state)
        .args(["roster", "--all", "--json"])
        .output()
        .expect("run `roster --all`");
    assert!(all_rows.status.success(), "stderr: {}", stderr_of(&all_rows));
    let doc: Value = serde_json::from_slice(&all_rows.stdout).expect("roster --all --json is valid JSON");
    let rows = doc["rows"].as_array().expect("rows array");
    let both: Vec<&Value> = rows
        .iter()
        .filter(|r| r["name"].as_str() == Some("t/alpha") || r["name"].as_str() == Some("t/beta"))
        .collect();
    assert_eq!(both.len(), 2, "both sessions must still be listed (gone) in --all: {rows:?}");
    for row in both {
        assert_eq!(row["conn_state"].as_str(), Some("gone"), "row must be gone: {row:?}");
    }

    // No `stub-acp` process remains — scoped to *this test's own* body
    // (`pgrep -f stub-acp`'s process-wide substring match is unreliable in
    // this suite: it also catches an unrelated `stub-acp` from a completely
    // different worktree/checkout running concurrently, or one *this same
    // binary's own earlier test* leaked by not calling `Body::stop` — both
    // observed while writing this test. Filtering on this body's own pid as
    // parent scopes the check to exactly the process tree this test spawned.)
    let ps = std::process::Command::new("pgrep").args(["-f", "-P", &body_pid.to_string(), "stub-acp"]).output();
    if let Ok(ps) = ps {
        let listing = String::from_utf8_lossy(&ps.stdout);
        assert!(listing.trim().is_empty(), "a stub-acp process must not remain: {listing:?}");
    }
}
