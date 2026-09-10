#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #192
//! RED tests for issue #192's reconnect contract, against a real hub + body +
//! `stub-acp` (#130) — no mocks of the circuit.
//!
//! # Decisions I made
//!
//! - **The drop mechanism is `control/test_drop`, not `kill_tree`.** Issue
//!   #190's own `body_drop_mid_turn_is_connection_lost_not_unreachable`
//!   (`talk_test.rs`) already proves the hub fails a pending `say` with
//!   `connection_lost` when the whole body *process* dies. This story's own
//!   spec is about the **hub's own connection-level teardown path**
//!   (reconnecting-vs-gone, the body reconnecting and resuming), which needs
//!   the body process to stay alive and actually reconnect — `kill_tree`
//!   cannot produce that. `control/test_drop {token}` (this issue's own test
//!   hook, gated behind `HOLLER_TEST_HOOKS=1` so it is unreachable in a
//!   production hub) forcibly ends the hub's own half of the connection
//!   while the body process keeps running, which is what lets it notice the
//!   drop and reconnect on its own backoff.
//! - **"reconnecting" is held open with `SIGSTOP`.** A real body reconnects
//!   within ~1s of noticing a drop (`backoff::delay_random(0)`), which is far
//!   too narrow a window to reliably `say` against a `reconnecting` row.
//!   `say_during_reconnecting_is_not_connected_with_age` freezes the body
//!   (the same real `SIGSTOP` mechanism `talk_test.rs`'s issue #243 coverage
//!   uses) *before* dropping its hub-side connection, so the frozen body
//!   never notices and the row stays `reconnecting` until the test resumes
//!   it.
//! - **"reply is discarded" is proven by absence, not by content.** The
//!   stub's own reply text (`"stub chunk N"`) is not distinguishable per-turn
//!   without threading prompt text through the stub, so
//!   `queued_prompt_runs_after_drop_and_reply_is_discarded` proves the
//!   discard by construction: both the running and queued `say` CLI
//!   processes already exited with `connection_lost` (there is no live
//!   process left for a stale reply to reach), the session still returns to
//!   `idle` once the body finishes both turns locally (proving they ran to
//!   completion, not stuck `working` forever), and a fresh `say` afterward
//!   succeeds normally (the session was never left wedged by the discarded
//!   replies).

mod support;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use support::{join, kill_tree, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir, STARTUP_WAIT};

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}
fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `say` with the full flag surface this file needs (mirrors
/// `talk_test.rs`'s own copy — see that file's doc for why this takes a bare
/// `Path` rather than `&StateDir`).
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

/// Run one `say` on a background thread, giving it `sleep_before_racing` to
/// get underway before returning the join handle (mirrors `talk_test.rs`'s
/// own copy).
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

/// Poll `say session TEXT` until it stops failing with `unknown session` or
/// `timeout` elapses (mirrors `talk_test.rs`'s own copy — the hub caches a
/// body's first presence asynchronously, so the very first `say` after
/// `body run` starts can race it).
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

/// `holler roster --all --json` (needed throughout this file: a
/// `reconnecting`/`gone` row is hidden from the default listing).
fn roster_all_json(state: &StateDir) -> Value {
    let out = support::holler_cmd(state)
        .args(["roster", "--all", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `holler roster --all --json`");
    assert!(out.status.success(), "`holler roster --all --json` failed: {}", stderr_of(&out));
    serde_json::from_slice(&out.stdout).expect("roster --all --json is valid JSON")
}

fn row_of<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    v.get("rows")?.as_array()?.iter().find(|r| r.get("name").and_then(Value::as_str) == Some(name))
}

fn conn_state_of<'a>(v: &'a Value, name: &str) -> Option<&'a str> {
    row_of(v, name)?.get("conn_state")?.as_str()
}

fn state_of<'a>(v: &'a Value, name: &str) -> Option<&'a str> {
    row_of(v, name)?.get("state")?.as_str()
}

/// Mint a token, join a body under it, and start `holler body run` — the
/// common setup every test below shares. Returns `(token_id, body)` (the
/// token id is `control/test_drop`'s own `token` target).
fn start_body_with_token(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &Hub,
    label: &str,
    sessions: &[(&str, &[&str])],
) -> (String, Body) {
    let (token_id, secret) = mint_token(hub_state, label);
    join(body_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, sessions);
    (token_id, Body::start(body_state, &config))
}

/// Force-drop `token`'s live connection on `hub_state`'s hub (issue #192's
/// `control/test_drop` test hook — only answered when the hub was started
/// with `HOLLER_TEST_HOOKS=1`, which every test below does).
fn test_drop(hub_state: &StateDir, token_id: &str) {
    let result = holler_hub::control::test_drop_at(hub_state.path(), token_id);
    assert!(result.is_ok(), "control/test_drop must succeed: {result:?}");
}

#[test]
fn say_fails_connection_lost_when_body_drops_mid_turn() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--slow", "--chunks", "20"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "30s", "alpha", "a long turn, about to be dropped"]),
        Duration::from_millis(300),
    );

    let started = Instant::now();
    test_drop(&hub_state, &token_id);

    let out = handle.join().expect("say thread");
    let elapsed = started.elapsed();

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("connection_lost") || err.contains("disconnected"), "got: {err:?}");
    assert!(!err.contains("no live holler hub reachable"), "must never report the hub itself as unreachable: {err:?}");
    // The whole point of #192 rule 3: the hub fails the pending `say`
    // *immediately* on the drop, never after the CLI's own 30s `--timeout`.
    assert!(elapsed < Duration::from_secs(10), "must fail immediately on the drop, not wait out a timeout: {elapsed:?}");

    kill_tree(body.child_mut());
}

#[test]
fn orphan_updates_after_reconnect_are_ignored_not_delivered_twice() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "30s", "alpha", "a turn about to be orphaned"]),
        Duration::from_millis(300),
    );
    test_drop(&hub_state, &token_id);

    let first = handle.join().expect("say thread");
    assert_eq!(first.status.code(), Some(1), "stderr: {}", stderr_of(&first));
    assert!(stderr_of(&first).contains("connection_lost") || stderr_of(&first).contains("disconnected"));

    // The body reconnects on its own backoff (never a blind sleep, ADR
    // 0002): once the roster shows the session `connected` again, a fresh
    // `say` must succeed cleanly — exactly-once for the operator, never a
    // stale/duplicate reply from the dropped turn's own updates (which the
    // hub — already having failed that `say` — ignores as `orphan_update`,
    // `docs/protocol/v2.md`'s "Reconnect contract" §6).
    wait_for(Duration::from_secs(15), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("connected")).then_some(())
    })
    .expect("the body must reconnect on its own backoff");

    let fresh = wait_for(Duration::from_secs(10), || {
        let out = support::say(&hub_state, "alpha", "fresh prompt after reconnect");
        out.status.success().then_some(out)
    })
    .unwrap_or_else(|| panic!("a fresh `say` must succeed once the body reconnects"));
    assert!(stdout_of(&fresh).contains("stub chunk"), "got: {:?}", stdout_of(&fresh));

    kill_tree(body.child_mut());
}

#[test]
fn say_during_reconnecting_is_not_connected_with_age() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--chunks", "1"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    // Freeze the body *before* dropping its hub-side connection so it never
    // notices and never reconnects — the row stays `reconnecting` (not a
    // fleeting ~1s window) until this test resumes it. Same real mechanism
    // as `talk_test.rs`'s `body_frozen_without_close_is_connection_lost_
    // within_bounded_time`.
    let pid = body.child_mut().id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGSTOP);
    }
    test_drop(&hub_state, &token_id);

    let reconnecting = wait_for(Duration::from_secs(10), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("reconnecting")).then_some(())
    });

    // A short, bounded wait (never a substitute for the `wait_for` above,
    // which already confirmed `reconnecting`) so the age hint below is a
    // real, non-zero "Ns ago" rather than always reading "0s ago".
    std::thread::sleep(Duration::from_millis(1_200));

    let started = Instant::now();
    let out = say_full(hub_state.path(), &["--timeout", "30s", "alpha", "are you there"]);
    let elapsed = started.elapsed();

    // Resume then reap the frozen body before any assertion below can panic
    // and skip cleanup (a `SIGSTOP`ped process is otherwise left behind).
    unsafe {
        libc::kill(pid, libc::SIGCONT);
    }
    kill_tree(body.child_mut());

    reconnecting.expect("the roster must show `alpha` reconnecting after the drop, while the body stays frozen");

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    // Rule 4's exact example shape: `"io/alpha is reconnecting (last seen
    // 32s ago)"` — the wire code is `-32004 not_connected` (the CLI's own
    // exit-1 mapping, asserted above); the message itself names the row's
    // own `conn_state` and age rather than a bare "not connected".
    assert!(err.contains("reconnecting"), "must name the row's own conn_state: {err:?}");
    assert!(err.contains("last seen") && err.contains("s ago"), "must carry the age hint: {err:?}");
    assert!(!err.contains("0s ago"), "the age hint must reflect real elapsed time, not always read zero: {err:?}");
    // No queuing at the hub in v1 (rule 4): the refusal is immediate, never a
    // wait for the (frozen, unreachable) body to come back.
    assert!(elapsed < Duration::from_secs(10), "must refuse immediately, never queue: {elapsed:?}");
}

#[test]
fn queued_prompt_runs_after_drop_and_reply_is_discarded() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--slow", "--chunks", "6"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let running = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "30s", "alpha", "first, slow turn"]),
        Duration::from_millis(200),
    );
    let queued = say_full_in_background(
        &hub_state,
        owned(&["--timeout", "30s", "--queue", "alpha", "queued turn, about to be dropped too"]),
        Duration::from_millis(200),
    );

    test_drop(&hub_state, &token_id);

    let running_out = running.join().expect("running say thread");
    let queued_out = queued.join().expect("queued say thread");
    for (label, out) in [("running", &running_out), ("queued", &queued_out)] {
        assert_eq!(out.status.code(), Some(1), "{label} say must fail; stderr: {}", stderr_of(out));
        let err = stderr_of(out);
        assert!(
            err.contains("connection_lost") || err.contains("disconnected"),
            "{label} say must report connection_lost, not a stale reply: {err:?}"
        );
    }

    // Rule 5: the queued prompt (already in the session's mailbox before the
    // drop) still runs to completion locally — the session returns to `idle`
    // once both turns finish, never stuck `working` forever even though
    // neither turn's reply ever reached an operator.
    wait_for(Duration::from_secs(20), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("connected") && state_of(&v, "b/alpha") == Some("idle")).then_some(())
    })
    .expect("the session must return to idle once both dropped turns finish locally");

    // The session was never left wedged by the discarded replies: a fresh
    // `say` afterward succeeds normally.
    let fresh = support::say(&hub_state, "alpha", "fresh prompt after the queue drained");
    assert!(fresh.status.success(), "stderr: {}", stderr_of(&fresh));
    assert!(stdout_of(&fresh).contains("stub chunk"));

    kill_tree(body.child_mut());
}

#[test]
fn presence_after_reconnect_is_authoritative() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let (_token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);

    let warm_a = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm_a.status.success(), "stderr: {}", stderr_of(&warm_a));
    let warm_b = say_ready(&hub_state, "beta", "warm up", STARTUP_WAIT);
    assert!(warm_b.status.success(), "stderr: {}", stderr_of(&warm_b));

    let both_up = roster_all_json(&hub_state);
    assert_eq!(conn_state_of(&both_up, "b/alpha"), Some("connected"));
    assert_eq!(conn_state_of(&both_up, "b/beta"), Some("connected"));

    // Abruptly end the body (not a clean `body detach`, which would clear its
    // identity) and restart it with `beta` removed from its own config — the
    // same identity/credential on disk, so the *same* token/client id
    // reconnects.
    kill_tree(body.child_mut());
    let config = write_sessions_toml(&body_state, &[("alpha", &["--chunks", "1"])]);
    let mut body2 = Body::start(&body_state, &config);

    // Rule 1: the fresh reconnect's `session/presence` carries only `alpha`
    // now, and the hub **replaces** (never merges) that token's rows —
    // `beta`'s row goes `gone` immediately, not lingering as `connected`.
    wait_for(Duration::from_secs(25), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("connected") && conn_state_of(&v, "b/beta") == Some("gone")).then_some(())
    })
    .unwrap_or_else(|| {
        let v = roster_all_json(&hub_state);
        panic!("presence after reconnect must be authoritative (alpha connected, beta gone): {v}")
    });

    kill_tree(body2.child_mut());
}

#[test]
fn roster_returns_to_connected_after_successful_traffic() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--chunks", "1"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    // Freeze the body, drop its connection (→ `reconnecting`), then resume it
    // so it reconnects on its own backoff and sends fresh traffic
    // (`circuit/hello` + `session/presence`) — issue holler-server#203's own
    // fix: any real traffic from the token flips a `reconnecting` row back to
    // `connected`, proven here end to end (not just `roster_test.rs`'s own
    // unit coverage of `Roster::touch`).
    let pid = body.child_mut().id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGSTOP);
    }
    test_drop(&hub_state, &token_id);
    wait_for(Duration::from_secs(10), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("reconnecting")).then_some(())
    })
    .expect("the roster must show `alpha` reconnecting after the drop");

    unsafe {
        libc::kill(pid, libc::SIGCONT);
    }

    let reconnected = wait_for(Duration::from_secs(15), || {
        let v = roster_all_json(&hub_state);
        (conn_state_of(&v, "b/alpha") == Some("connected")).then_some(())
    });

    kill_tree(body.child_mut());

    reconnected.expect("the roster must return `alpha` to connected once the body's own traffic resumes");
}

/// Issue #299: connection churn at volume — dozens of consecutive
/// disconnect/reconnect cycles against a real hub, using the exact same
/// `control/test_drop` mechanism the tests above already use. Each cycle: a
/// successful `test_drop` call is itself proof a live connection just ended
/// (its own RPC hard-errors with no live match), then the roster must return
/// to `connected` on the body's fresh reconnect traffic and a fresh `say`
/// must work — repeated [`CHURN_CYCLES`] times back to back. After the
/// churn, it asserts no state leak accumulated: exactly one roster row for
/// the session (no stale duplicate rows) and no leaked processes hanging off
/// the body (no accumulated dead/orphaned agent children across all those
/// reconnects).
const CHURN_CYCLES: usize = 25;

#[test]
fn connection_churn_survives_dozens_of_reconnect_cycles() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_TEST_HOOKS", "1")]);
    let (token_id, mut body) =
        start_body_with_token(&hub_state, &body_state, &hub, "b", &[("alpha", &["--chunks", "1"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", STARTUP_WAIT);
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    for cycle in 0..CHURN_CYCLES {
        // The body must still be alive going into this cycle — a crash mid-churn
        // is a real bug, not something later cycles should paper over.
        assert!(
            body.child_mut().try_wait().expect("try_wait the body").is_none(),
            "the body process must survive churn cycle {cycle} (it died)"
        );

        // `test_drop`'s own RPC only succeeds by finding and force-ending a
        // *live* connection for this token (`control_server.rs`'s
        // `test_drop`: `NotConnected` is a hard error) — so a successful call
        // here is itself proof a real drop happened this cycle. This
        // deliberately does *not* also poll for the roster to transiently
        // show non-`connected` first: a real body reconnects within ~1s
        // (`say_during_reconnecting_is_not_connected_with_age` above notes
        // this exact window is too narrow to reliably observe without
        // freezing the body), so at churn volume — many consecutive
        // back-to-back cycles, no freeze — that whole transient can complete
        // between two 50ms polls and be legitimately missed. What must hold
        // is the outcome below: the row is back on fresh `connected` traffic.
        test_drop(&hub_state, &token_id);

        // The body reconnects on its own backoff and sends fresh presence,
        // which is what flips the row back to `connected` (issue
        // holler-server#203's fix, proven end to end by
        // `roster_returns_to_connected_after_successful_traffic` above for a
        // single cycle — here across many consecutive ones).
        wait_for(Duration::from_secs(8), || {
            let v = roster_all_json(&hub_state);
            (conn_state_of(&v, "b/alpha") == Some("connected")).then_some(())
        })
        .unwrap_or_else(|| panic!("cycle {cycle}: the roster must return `alpha` to connected on the body's fresh reconnect"));

        // A fresh `say` must work cleanly on the reconnected session — proves
        // the reconnect is not just a roster-label flip but a genuinely live
        // connection each cycle.
        let out = say_ready(&hub_state, "alpha", &format!("cycle {cycle}"), Duration::from_secs(8));
        assert!(out.status.success(), "cycle {cycle}: `say` after reconnect failed: {}", stderr_of(&out));
        assert!(
            stdout_of(&out).contains("stub chunk"),
            "cycle {cycle}: unexpected reply: {:?}",
            stdout_of(&out)
        );
    }

    // No crash across the whole churn run.
    assert!(
        body.child_mut().try_wait().expect("try_wait the body").is_none(),
        "the body process must still be alive after {CHURN_CYCLES} churn cycles"
    );

    // No stale roster rows: exactly one row for this session, not one per
    // cycle (the hub upserts a token's rows on presence — it must never grow
    // duplicates across repeated reconnects).
    let final_roster = roster_all_json(&hub_state);
    let rows_named_alpha = final_roster["rows"]
        .as_array()
        .expect("roster --all --json carries `rows`")
        .iter()
        .filter(|r| r.get("name").and_then(Value::as_str) == Some("b/alpha"))
        .count();
    assert_eq!(
        rows_named_alpha, 1,
        "expected exactly one roster row for `b/alpha` after {CHURN_CYCLES} churn cycles, found {rows_named_alpha}: {final_roster}"
    );

    // No leaked processes/tasks: the body should still be driving exactly one
    // live agent child (the current session's `stub-acp`), never an
    // accumulation of dead/orphaned children from earlier cycles' reconnects.
    #[cfg(unix)]
    {
        let body_pid = body.child_mut().id();
        let children = Command::new("pgrep")
            .args(["-P", &body_pid.to_string()])
            .output()
            .expect("run pgrep to count the body's live children");
        let child_count = stdout_of(&children).lines().filter(|l| !l.trim().is_empty()).count();
        assert!(
            child_count <= 1,
            "expected the body to hold at most one live agent child after {CHURN_CYCLES} churn cycles, found {child_count} (a leak): {:?}",
            stdout_of(&children)
        );
    }

    kill_tree(body.child_mut());
}
