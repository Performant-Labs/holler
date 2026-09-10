#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #190
//! RED tests for `say` → `session/prompt` (issue #190), the issue's own test
//! list, verbatim in name where practical, against real hub + body +
//! `stub-acp` (#130) subprocesses — no mocks of the circuit.
//!
//! # Decisions I made
//!
//! - **Roster/query stand-in.** Neither #185 (query) nor #186 (roster) had
//!   landed when this story was built (`gh pr list`/`gh issue view`: both
//!   open, no branch). `say`'s own name-resolution and busy-check read a
//!   minimal presence cache this story adds to `holler-hub`'s `live`
//!   registry (see that module's own doc) — these tests exercise that
//!   stand-in directly, through the real CLI/wire path.
//! - **"the session has started its turn" observability.** The busy/queue/
//!   connection-loss tests below launch the first `say` in a background
//!   thread via [`say_full_in_background`], which polls `holler roster
//!   --json` (issue #276 — this used to be a fixed sleep, since `holler
//!   roster` wasn't implemented yet when this file was first written) until
//!   the target session's roster row reports `state: "working"` before
//!   returning control, so the racing/killing action that follows always
//!   fires against a session that has genuinely started its turn rather than
//!   one that merely had "enough" wall-clock time to have (hopefully) done so.

mod support;

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use support::{join, kill_tree, mint_token, roster_json, wait_for, write_sessions_toml, Body, Hub, StateDir};

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}
fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `say` with the full flag surface this file needs (`support::say` only
/// covers the bare `say SESSION TEXT` shape, and takes `&StateDir` rather
/// than a bare path — this one takes a `Path` so a background thread can run
/// it against a state dir whose `StateDir` owner lives on the test's main
/// thread, see [`say_full_in_background`]).
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

/// Run one `say` on a background thread against `state`'s directory, then
/// wait (an observable outcome, not a blind sleep — issue #276) for the
/// hub's roster to report `session` as `working` before returning the join
/// handle — the pattern every busy/queue/connection-loss test below shares.
///
/// Polls `holler roster --json` (the same signal
/// `attach_mode_test.rs::wait_for_roster_row` already polls for the
/// equivalent "session actually started" wait) rather than assuming a fixed
/// sleep was long enough for the background process to have spawned,
/// connected, and started its turn. `ready_timeout` bounds the poll; a
/// session that never reaches `working` within it is a genuine test failure,
/// not a race to paper over.
fn say_full_in_background(
    state: &StateDir,
    session: &str,
    args: Vec<String>,
    ready_timeout: Duration,
) -> std::thread::JoinHandle<Output> {
    let state_path = state.path().to_path_buf();
    let handle = std::thread::spawn(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        say_full(&state_path, &args)
    });
    // `state` here is the roster's own enriched view, not the bare
    // `SessionState` the body reports: `holler-hub`'s `talk.rs` (busy-check)
    // and `roster.rs` both synthesize `"stalled"` out of a `working` session
    // once `last_update_age_ms` crosses the hub's stall threshold — so a
    // session that has genuinely started its turn can already read
    // `"stalled"` (never `"working"`) by the first poll when the hub is
    // configured with an aggressive threshold (e.g. `HOLLER_STALL_MS=1`, as
    // `say_to_stalled_session_names_stalled` does). Treat either as "the
    // turn started" — `"idle"`/`"input-required"` are the only states that
    // mean it has not (or, for `input-required`, is not mid-turn at all).
    let suffix = format!("/{session}");
    wait_for(ready_timeout, || {
        let rows = roster_json(state)["rows"].as_array()?.clone();
        rows.iter()
            .any(|r| {
                r["name"].as_str().is_some_and(|n| n.ends_with(&suffix))
                    && matches!(r["state"].as_str(), Some("working") | Some("stalled"))
            })
            .then_some(())
    })
    .unwrap_or_else(|| {
        panic!("background `say {session}` never reached `working` on the roster within {ready_timeout:?}: {:?}", roster_json(state))
    });
    handle
}

fn owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

/// Join `body_state`'s body to `hub` under a freshly minted token and start
/// `holler body run` over `sessions` (`(name, extra stub-acp argv)` pairs —
/// see [`support::write_sessions_toml`]). The token is minted against
/// `hub_state` (`hub token mint` operates on the local token-store file
/// directly, so it must run against the *hub's own* state dir, not the
/// body's — a distinct `body_state` is only for the identity/session-config
/// files the body process itself owns).
fn start_body(hub_state: &StateDir, body_state: &StateDir, hub: &Hub, sessions: &[(&str, &[&str])]) -> Body {
    start_body_labeled(hub_state, body_state, hub, "b", sessions)
}

/// [`start_body`] with an explicit mint label — needed whenever a single
/// test mints more than one token against the same `hub_state` (a duplicate
/// label is refused).
fn start_body_labeled(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &Hub,
    label: &str,
    sessions: &[(&str, &[&str])],
) -> Body {
    start_body_with_env(hub_state, body_state, hub, label, sessions, &[])
}

/// [`start_body_labeled`] with extra environment variables on the body
/// process (see [`support::Body::start_with_env`]'s own doc).
fn start_body_with_env(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &Hub,
    label: &str,
    sessions: &[(&str, &[&str])],
    envs: &[(&str, &str)],
) -> Body {
    let (token_id, secret) = mint_token(hub_state, label);
    join(body_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, sessions);
    Body::start_with_env(body_state, &config, envs)
}

/// Poll `say session TEXT` until it stops failing with `unknown session`
/// (the hub has not yet cached this body's first `session/presence`) or
/// `timeout` elapses — the one legitimate "not ready yet" retry this file
/// does (an observable *outcome*, not a blind sleep): the hub sends its
/// first presence the instant the body's socket authenticates (issue #182),
/// so this converges in well under a second in practice.
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
fn say_routes_and_returns_reply() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "2"])]);

    let out = say_ready(&hub_state, "alpha", "hi", Duration::from_secs(10));
    assert!(out.status.success(), "say must exit 0; stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);
    assert!(text.contains("stub chunk"), "reply must carry the stub's streamed text: {text:?}");
}

#[test]
fn say_two_sessions_independently() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(
        &hub_state,
        &body_state,
        &hub,
        &[("alpha", &["--chunks", "2"]), ("beta", &["--chunks", "2"])],
    );

    let a = say_ready(&hub_state, "alpha", "hi alpha", Duration::from_secs(10));
    assert!(a.status.success(), "say alpha must exit 0; stderr: {}", stderr_of(&a));
    let b = say_full(hub_state.path(), &["beta", "hi beta"]);
    assert!(b.status.success(), "say beta must exit 0; stderr: {}", stderr_of(&b));
    assert!(stdout_of(&a).contains("stub chunk"));
    assert!(stdout_of(&b).contains("stub chunk"));
}

#[test]
fn say_bare_name_when_unambiguous() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);

    // A bare name (no `<label>/` prefix) resolves cleanly when exactly one
    // live session carries it.
    let out = say_ready(&hub_state, "alpha", "hi", Duration::from_secs(10));
    assert!(out.status.success(), "bare unambiguous name must resolve; stderr: {}", stderr_of(&out));
}

#[test]
fn say_ambiguous_exit_2_lists_candidates() {
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    // Two distinct bodies each advertising a session named "alpha" — a bare
    // `say alpha` cannot pick one.
    let body_a = StateDir::new();
    let body_b = StateDir::new();
    let _a = start_body_labeled(&hub_state, &body_a, &hub, "b1", &[("alpha", &["--chunks", "1"])]);
    let _b = start_body_labeled(&hub_state, &body_b, &hub, "b2", &[("alpha", &["--chunks", "1"])]);

    // Wait until *both* bodies' presence has landed: poll until the refusal
    // settles into "ambiguous" (one body registering first would still read
    // as `unknown session`-then-success, never ambiguous). 30s (not the 10s
    // every other `say_ready`-style wait in this file uses): this is the one
    // case that needs *two* full body join+run sequences to complete, not
    // one, and was observed to need more headroom under CI's slower/shared
    // runners than a single-body wait does.
    let out = wait_for(Duration::from_secs(30), || {
        let out = support::say(&hub_state, "alpha", "hi");
        stderr_of(&out).contains("ambiguous").then_some(out)
    })
    .unwrap_or_else(|| panic!("never observed an ambiguous refusal within 30s"));

    assert_eq!(out.status.code(), Some(2), "ambiguous session must exit 2; stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("alpha"), "the refusal must name the ambiguous session: {err:?}");
}

#[test]
fn say_unknown_session_exit_1() {
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body_state = StateDir::new();
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);
    // Give the one real session a moment to register so the refusal below is
    // provably "no such session", not "the hub hasn't heard from anyone yet".
    let _ = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));

    let out = support::say(&hub_state, "no-such-session", "hi");
    assert_eq!(out.status.code(), Some(1), "unknown session must exit 1; stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("unknown session") || err.contains("no-such-session"), "got: {err:?}");
}

#[test]
fn say_json_shape() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "2"])]);

    let out = wait_for(Duration::from_secs(10), || {
        let out = say_full(hub_state.path(), &["--json", "alpha", "hi"]);
        (out.status.success() || !stderr_of(&out).contains("unknown session")).then_some(out)
    })
    .expect("say --json eventually succeeds");
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let doc: Value = serde_json::from_slice(&out.stdout).expect("say --json prints one JSON document");
    for key in ["session", "stop_reason", "state", "updates", "elapsed_ms", "text", "message"] {
        assert!(doc.get(key).is_some(), "say --json result missing `{key}`: {doc}");
    }
    assert_eq!(doc["stop_reason"], "end_turn");
    assert_eq!(doc["state"], "completed");
    assert!(doc["message"]["parts"].is_array());
}

#[test]
fn talklog_has_prompt_updates_done() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "3"])]);

    let out = say_ready(&hub_state, "alpha", "hi", Duration::from_secs(10));
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));

    // The body's advertised hostname is always "default" (no `--hostname`
    // flag exists yet on `body join`), so the log path is deterministic.
    let path = hub_state.hub().join("talklog").join("default__alpha.jsonl");
    let content = wait_for(Duration::from_secs(5), || std::fs::read_to_string(&path).ok())
        .unwrap_or_else(|| panic!("talklog never appeared at {}", path.display()));
    let lines: Vec<Value> = content.lines().map(|l| serde_json::from_str(l).expect("valid jsonl line")).collect();
    assert!(lines.iter().any(|l| l.get("text").is_some()), "must have a prompt line: {lines:?}");
    assert!(
        lines.iter().any(|l| l.get("seq").is_some() && l.get("parts").is_some()),
        "must have an update line: {lines:?}"
    );
    assert!(lines.iter().any(|l| l.get("stopReason").is_some()), "must have a done line: {lines:?}");
    // Every line names the same prompt_id (one turn, one correlation id).
    let ids: std::collections::HashSet<_> =
        lines.iter().filter_map(|l| l.get("prompt_id").and_then(|v| v.as_str())).collect();
    assert_eq!(ids.len(), 1, "every talklog line for one turn shares one prompt_id: {lines:?}");
}

#[test]
fn say_to_working_session_is_session_busy_exit_1_with_hint() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "5"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        "alpha",
        owned(&["alpha", "second, slow turn"]),
        Duration::from_secs(10),
    );
    let racer = support::say(&hub_state, "alpha", "interrupting");
    let _ = handle.join();

    assert_eq!(racer.status.code(), Some(1), "busy session must exit 1; stderr: {}", stderr_of(&racer));
    let err = stderr_of(&racer);
    assert!(err.contains("session_busy"), "must carry the session_busy hint: {err:?}");
    assert!(err.contains("is working") || err.contains("is stalled"), "must name the state: {err:?}");
    assert!(err.contains("--queue"), "must hint --queue: {err:?}");
}

#[test]
fn say_queue_appends_and_runs_after_turn() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "3"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        "alpha",
        owned(&["alpha", "first, slow turn"]),
        Duration::from_secs(10),
    );
    let queued = say_full(hub_state.path(), &["--queue", "alpha", "queued turn"]);
    let first = handle.join().expect("first say thread");

    assert!(first.status.success(), "the first turn must still succeed; stderr: {}", stderr_of(&first));
    assert!(queued.status.success(), "the queued turn must run after the first ends; stderr: {}", stderr_of(&queued));
}

#[test]
fn say_to_stalled_session_names_stalled() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    // `HOLLER_STALL_MS` is the *hub* process's own env (the busy check runs
    // inside the hub, over the control socket) — a 1ms threshold makes the
    // very first busy observation already "stalled" rather than "working".
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_STALL_MS", "1")]);
    // The body pushes `session/presence` immediately on every state change
    // (issue #190's own wiring of #189's `subscribe_presence_changes`), so
    // the hub's cache reflects `Working` within the connection's normal
    // traffic — no heartbeat tuning needed here.
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        "alpha",
        owned(&["alpha", "a long, --slow, 10-chunk turn"]),
        Duration::from_secs(10),
    );
    // Poll (an observable outcome, not a fixed sleep) rather than firing the
    // racer exactly once: the very first busy observation can still land
    // before the state-change → `session/presence` → hub-cache round trip
    // completes, in which case the body's own always-"working" fallback
    // busy response answers first — that is a real, harmless race (the hub
    // forwarded before its cache caught up), not a defect, and a retry a
    // few ms later reliably sees "stalled" instead.
    let racer = wait_for(Duration::from_secs(5), || {
        let out = support::say(&hub_state, "alpha", "interrupting");
        stderr_of(&out).contains("is stalled").then_some(out)
    })
    .unwrap_or_else(|| panic!("never observed a stalled refusal within 5s"));
    let _ = handle.join();

    assert_eq!(racer.status.code(), Some(1), "stderr: {}", stderr_of(&racer));
    let err = stderr_of(&racer);
    assert!(err.contains("is stalled"), "a 1ms stall threshold must name the session stalled: {err:?}");
}

#[test]
fn say_to_disconnected_body_is_not_connected() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    // Detach the body cleanly, then wait for the hub to observe the drop
    // (`hub status`'s `clients` count is the one live observable that story
    // #182 already gives us).
    body.stop(&body_state, Duration::from_secs(5));
    wait_for(Duration::from_secs(5), || {
        let clients = support::hub_status_json(&hub_state)["clients"].as_u64()?;
        (clients == 0).then_some(())
    })
    .expect("hub must observe the body disconnect");

    let out = support::say(&hub_state, "alpha", "hi");
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("not connected") || err.contains("not_connected"), "got: {err:?}");
}

#[test]
fn body_drop_mid_turn_is_connection_lost_not_unreachable() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let mut body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let handle = say_full_in_background(
        &hub_state,
        "alpha",
        owned(&["--timeout", "20s", "alpha", "a long turn, about to be cut off"]),
        Duration::from_secs(10),
    );
    support::kill_tree(body.child_mut());

    let out = handle.join().expect("say thread");
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("connection_lost") || err.contains("disconnected"), "got: {err:?}");
    assert!(!err.contains("no live holler hub reachable"), "must never report the hub itself as unreachable: {err:?}");
}

#[test]
fn say_timeout_message() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    // 10 chunks * 200ms/chunk (--slow) ≈ 2s turn — comfortably longer than a
    // 1s `--timeout`, so the CLI's own wait (not the turn) is what expires.
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--slow", "--chunks", "10"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let out = say_full(hub_state.path(), &["--timeout", "1s", "alpha", "another long turn"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("no reply from") && err.contains("1s"), "got: {err:?}");
}

/// Issue #243: a body that goes silent **without closing the TCP socket**
/// (a sleep/suspend, or a silent network partition — as opposed to
/// `body_drop_mid_turn_is_connection_lost_not_unreachable`'s `kill_tree`,
/// which sends a real FIN/RST the hub's socket read immediately observes)
/// must still have the hub notice within a bounded time and fail any `say`
/// routed to it with `connection_lost`, never hang forever.
///
/// `SIGSTOP` on the body process itself (not its process group / children,
/// and never `kill_tree`) is what produces exactly that: every thread in the
/// body freezes mid-flight, so it stops both reading and writing, but the
/// kernel-level TCP connection stays fully established (no FIN, no RST) —
/// this is the real mechanism issue #243's own repro describes, not a mock
/// of it. `HOLLER_HUB_LIVENESS_TIMEOUT_MS` (this story's own env override,
/// `circuit.rs`'s `liveness_timeout()`) is set far below the CLI's own
/// `--timeout` so a passing run proves the *hub's* liveness check ended the
/// connection, not the `say` client's own wait.
#[test]
fn body_frozen_without_close_is_connection_lost_within_bounded_time() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start_with_env(&hub_state, &[("HOLLER_HUB_LIVENESS_TIMEOUT_MS", "800")]);
    let mut body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);

    let warm = say_ready(&hub_state, "alpha", "warm up", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    // Freeze the body process in place. A positive pid signals only this one
    // process (not `-pid`, which would also freeze the `stub-acp` agent it
    // spawned) — irrelevant to the hub's own socket either way, but keeps
    // this test's intent (freeze the peer holding the connection) exact.
    let pid = body.child_mut().id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGSTOP);
    }

    let started = Instant::now();
    let out = say_full(hub_state.path(), &["--timeout", "20s", "alpha", "are you there"]);
    let elapsed = started.elapsed();

    // Resume then reap the frozen body before any assertion below can panic
    // and skip cleanup (a `SIGSTOP`ped process is otherwise left behind).
    unsafe {
        libc::kill(pid, libc::SIGCONT);
    }
    kill_tree(body.child_mut());

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_of(&out));
    let err = stderr_of(&out);
    assert!(err.contains("connection_lost") || err.contains("disconnected"), "got: {err:?}");
    assert!(
        !err.contains("no reply from"),
        "must be the hub's own liveness check, not the CLI's --timeout firing: {err:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "the hub's 800ms liveness timeout must end the say well under the 20s CLI --timeout; took {elapsed:?}"
    );
}
