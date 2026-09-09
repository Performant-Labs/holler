#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #182
//! `holler body run` — the connection loop (issue #182): authenticate,
//! hello, presence heartbeat, liveness/reconnect, detach, and the hub-side
//! `hub token ping` companion. Real subprocesses, real loopback sockets — no
//! mocks of the circuit (the same discipline as `body_join_test.rs`, #176).
//!
//! Readiness is always **observed** — `hub_status_json`'s `clients` count, or
//! `body/connection_state.json`'s `state` field — via [`support::wait_for`],
//! never a blind sleep (ADR 0002).
//!
//! Two RED-list cases (`hub_restart_is_recovered_by_reconnect`,
//! `fresh_hello_and_presence_on_every_reconnect`) kill and restart a real hub
//! process and time a real reconnect; both are `#[ignore]` (run with `--
//! --ignored`) per the same "test-tag-interop" treatment the issue calls out
//! for the hub-restart case — real cross-process timing, not suited to the
//! default fast suite. `backoff_caps_at_30s` (the schedule's own RED test) is
//! a pure-function unit test and lives beside the function it tests,
//! `crates/holler-body/src/backoff.rs`, per normal Rust convention rather
//! than this integration-test file.

mod support;

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, kill_tree, make_own_process_group, wait_for, Body, Hub, StateDir};

/// Run one `holler` invocation to completion (piped stdio) and return its
/// exit code plus decoded stdout/stderr.
fn run(state: &StateDir, args: &[&str]) -> (i32, String, String) {
    let out = holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn connection_state_path(state: &StateDir) -> PathBuf {
    state.body().join("connection_state.json")
}

/// Read `body/connection_state.json`'s `state` field, if the file exists and
/// parses (`None` otherwise — the file may not exist yet, or a write may be
/// mid-rename; both are transient and `wait_for` retries).
fn conn_state(state: &StateDir) -> Option<String> {
    let bytes = std::fs::read(connection_state_path(state)).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get("state").and_then(|s| s.as_str()).map(str::to_owned)
}

/// Join `state`'s body to the hub at `ws_url` with a freshly-minted token,
/// returning the token id (tests that revoke it, or want to name it in `hub
/// token ping`, need it back).
fn join_fresh(state: &StateDir, ws_url: &str, label: &str) -> String {
    let (token_id, secret) = support::mint_token(state, label);
    let (code, _, stderr) = run(
        state,
        &["body", "join", "--server", ws_url, "--token", &format!("{token_id}:{secret}")],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");
    token_id
}

/// Spawn `holler body run` in its own process group (so [`kill_tree`] reaps
/// it), with an empty session config (no sessions exist in this story — every
/// presence carries `sessions:[]`).
fn spawn_run(state: &StateDir) -> Body {
    let config = support::write_sessions_toml(state, &[]);
    Body::start(state, &config)
}

const READY: Duration = Duration::from_secs(10);

/// `body run` against a live hub: authenticates, hellos, and reaches
/// `connected` — and the hub's own `hub status` then counts it as one client.
#[test]
fn run_authenticates_and_hub_status_shows_one_client() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");

    let body = spawn_run(&state);
    let connected = wait_for(READY, || (conn_state(&state).as_deref() == Some("connected")).then_some(()));
    assert!(connected.is_some(), "body run must reach `connected` within {READY:?}");

    let clients = wait_for(READY, || {
        let doc = support::hub_status_json(&state);
        (doc.get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(clients.is_some(), "hub status must show 1 client once the body is live");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// `hub token ping ID` against a live, authenticated body: the hub sends a
/// real `circuit/ping` over that token's socket and reports `{hostname,
/// rtt_ms}` — exit 0.
#[test]
fn token_ping_returns_rtt() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let token_id = join_fresh(&state, &hub.ws_url(), "kiwi");
    let body = spawn_run(&state);

    let ready = wait_for(READY, || {
        let doc = support::hub_status_json(&state);
        (doc.get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(ready.is_some(), "the body must be live before pinging it");

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "token", "ping", &token_id]);
    assert_eq!(code, 0, "ping of a live token must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("ping --json is JSON");
    assert_eq!(doc["state"].as_str(), Some("valid"));
    assert!(doc["hostname"].as_str().is_some(), "ping reports the body's hostname: {doc}");
    assert!(doc["rtt_ms"].as_u64().is_some(), "ping reports an rtt_ms: {doc}");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// A second `body run` against the same state dir while the first is live is
/// refused: exit 3, "another holler body run is active".
#[test]
fn second_run_refused_exit_3() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let body = spawn_run(&state);

    let connected = wait_for(READY, || (conn_state(&state).as_deref() == Some("connected")).then_some(()));
    assert!(connected.is_some(), "the first run must connect before the second is attempted");

    let config = support::write_sessions_toml(&state, &[]);
    let out = holler_cmd(&state)
        .arg("body")
        .arg("run")
        .arg("--config")
        .arg(&config)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn second body run")
        .wait_with_output()
        .expect("wait on second body run");
    assert_eq!(out.status.code(), Some(3), "a second live run must exit 3");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("another holler body run is active"),
        "stderr names the held lock: {stderr}"
    );

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// A `body run` that is hard-killed (simulating a crash — no graceful
/// shutdown) leaves its `run.lock` reclaimable: a fresh `body run` right
/// after it must NOT be refused (the flock releases on process death, exactly
/// like the hub's own instance lock, story #143).
#[test]
fn crashed_run_lock_reclaimed() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");

    let mut first = spawn_run_raw(&state);
    // Assert on the hub's own client count, not the body's
    // `connection_state.json` — that file is a **stale leftover** the instant
    // `first` is SIGKILLed (no graceful writer left to update it), so a check
    // against it after the crash could pass on `first`'s last write rather
    // than proving `second` ever connected. `hub status`'s `clients` count is
    // hub-side truth: it can only read 1 while a socket is actually live.
    let clients_1 = wait_for(READY, || {
        (support::hub_status_json(&state).get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(clients_1.is_some(), "the first run must connect before it is crashed");

    // SIGKILL, not SIGTERM: a crash gets no graceful-shutdown path at all —
    // the flock's release is entirely the OS's doing on process death.
    kill_tree(&mut first);
    let clients_0 = wait_for(READY, || {
        (support::hub_status_json(&state).get("clients").and_then(|v| v.as_u64()) == Some(0)).then_some(())
    });
    assert!(clients_0.is_some(), "the hub must notice the crashed socket");

    let mut second = spawn_run_raw(&state);
    let clients_1_again = wait_for(READY, || {
        (support::hub_status_json(&state).get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(
        clients_1_again.is_some(),
        "a fresh run after a crash must reclaim the lock and connect, not be refused"
    );
    kill_tree(&mut second);
    hub.stop(Duration::from_secs(5));
}

/// Spawn `body run` as a raw `Child` (not wrapped in [`Body`]) — used by
/// tests that need to kill it themselves rather than go through `body
/// detach`'s graceful path.
fn spawn_run_raw(state: &StateDir) -> std::process::Child {
    let config = support::write_sessions_toml(state, &[]);
    let mut cmd = holler_cmd(state);
    cmd.arg("body")
        .arg("run")
        .arg("--config")
        .arg(&config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    make_own_process_group(&mut cmd);
    cmd.spawn().expect("spawn `holler body run`")
}

/// `body detach` against a **live** run: it signals the run to close its
/// circuit (rather than only deleting the credential), and once it has, the
/// hub's own client count drops back to 0.
#[test]
fn detach_closes_live_run_and_hub_client_count_drops_to_0() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let body = spawn_run(&state);

    let ready = wait_for(READY, || {
        let doc = support::hub_status_json(&state);
        (doc.get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(ready.is_some(), "the body must be live before it is detached");

    // `Body::stop` runs `holler body detach` then waits for the child to
    // exit — exactly the live-detach path issue #182 adds to `body detach`.
    body.stop(&state, Duration::from_secs(10));

    let dropped = wait_for(READY, || {
        let doc = support::hub_status_json(&state);
        (doc.get("clients").and_then(|v| v.as_u64()) == Some(0)).then_some(())
    });
    assert!(dropped.is_some(), "hub status must show 0 clients once the body detaches");

    hub.stop(Duration::from_secs(5));
}

/// A credential the hub has since revoked is refused with `-32002` on the
/// very first `circuit/authenticate` — the body's connection loop treats that
/// (and only that) code as unretryable: exit 1, immediately, no reconnect
/// loop spun up first.
#[test]
fn revoked_credential_exit_1_no_retry() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let token_id = join_fresh(&state, &hub.ws_url(), "kiwi");

    let (code, _, stderr) = run(&state, &["hub", "token", "revoke", &token_id]);
    assert_eq!(code, 0, "revoke must exit 0; stderr: {stderr}");

    let config = support::write_sessions_toml(&state, &[]);
    let out = holler_cmd(&state)
        .arg("body")
        .arg("run")
        .arg("--config")
        .arg(&config)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn body run against a revoked credential")
        .wait_with_output()
        .expect("wait on body run");
    assert_eq!(out.status.code(), Some(1), "a revoked credential is exit 1, not a retry loop");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("authentication failed"),
        "stderr names the auth failure: {stderr}"
    );

    hub.stop(Duration::from_secs(5));
}

// --- interop-tagged: real hub restart / real reconnect timing ---------------
//
// Both cases below kill and restart a real hub process bound to the SAME
// port and time how long a live body takes to notice and reconnect. That is
// exactly the class of case the issue calls "test-tag-interop": a real
// cross-process timing test, excluded from the default fast suite and run
// deliberately (`cargo test -- --ignored`). A short
// `HOLLER_HEARTBEAT_INTERVAL_MS` keeps the liveness window (3× the interval)
// small enough that even the ignored run finishes in seconds, not minutes.

/// Spawn a hub bound to an **explicit** address (unlike [`Hub::start`], which
/// always asks for port 0) so a restart can rebind the exact same port. Reuses
/// `Hub::start`'s own "wait for the `listening` event on stderr" contract.
fn start_hub_at(state: &StateDir, addr: &str) -> std::process::Child {
    let mut cmd = holler_cmd(state);
    cmd.args(["hub", "serve", "--listen", addr])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn `holler hub serve`");
    let stderr = child.stderr.take().expect("hub stderr is piped");
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if let Ok(v) = serde_json::from_str::<Value>(&line) {
                        if v.get("event").and_then(|e| e.as_str()) == Some("listening") {
                            let _ = tx.send(());
                        }
                    }
                }
            }
        }
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("hub did not report listening within 10s");
    child
}

/// Issue #182: "stop hub, start again on the same port, body reports
/// `connected` again <= 35s".
#[test]
#[ignore] // test-tag-interop: real cross-process hub restart, excluded from the default suite.
fn hub_restart_is_recovered_by_reconnect() {
    let state = StateDir::new();
    let addr = "127.0.0.1:41917"; // a fixed, unusual port (unlikely to collide locally).
    let mut hub = start_hub_at(&state, addr);
    join_fresh(&state, &format!("ws://{addr}"), "kiwi");

    let mut cmd = holler_cmd(&state);
    cmd.env("HOLLER_HEARTBEAT_INTERVAL_MS", "300");
    let config = support::write_sessions_toml(&state, &[]);
    cmd.arg("body").arg("run").arg("--config").arg(&config).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    make_own_process_group(&mut cmd);
    let mut body = cmd.spawn().expect("spawn body run");

    let connected = wait_for(READY, || (conn_state(&state).as_deref() == Some("connected")).then_some(()));
    assert!(connected.is_some(), "must connect to the first hub instance");

    kill_tree(&mut hub);
    let mut hub2 = start_hub_at(&state, addr);

    let reconnected = wait_for(Duration::from_secs(35), || {
        (conn_state(&state).as_deref() == Some("connected")).then_some(())
    });
    assert!(reconnected.is_some(), "must reconnect to the restarted hub within 35s");

    kill_tree(&mut body);
    kill_tree(&mut hub2);
}

/// Issue #182: "count hellos seen by hub across 2 reconnects == 3" — using the
/// body's own `conn_connected` log line (emitted only once a full connect →
/// authenticate → hello exchange succeeds) as the observable proxy, since the
/// hub exposes no hello counter on the wire.
#[test]
#[ignore] // test-tag-interop: real cross-process hub restarts, excluded from the default suite.
fn fresh_hello_and_presence_on_every_reconnect() {
    let state = StateDir::new();
    let addr = "127.0.0.1:41918";
    let mut hub = start_hub_at(&state, addr);
    join_fresh(&state, &format!("ws://{addr}"), "kiwi");

    let config = support::write_sessions_toml(&state, &[]);
    let mut cmd = holler_cmd(&state);
    cmd.env("HOLLER_HEARTBEAT_INTERVAL_MS", "300")
        .env("HOLLER_DEBUG", "noisy")
        .arg("body")
        .arg("run")
        .arg("--config")
        .arg(&config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    let mut body = cmd.spawn().expect("spawn body run");
    let stderr = body.stderr.take().expect("body stderr is piped");
    let connected_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = connected_count.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if line.contains("conn_connected") {
                        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        }
    });

    let first = wait_for(READY, || {
        (connected_count.load(std::sync::atomic::Ordering::SeqCst) >= 1).then_some(())
    });
    assert!(first.is_some(), "must connect once before any restart");

    for _ in 0..2 {
        kill_tree(&mut hub);
        hub = start_hub_at(&state, addr);
        let n_before = connected_count.load(std::sync::atomic::Ordering::SeqCst);
        let reconnected = wait_for(Duration::from_secs(35), || {
            (connected_count.load(std::sync::atomic::Ordering::SeqCst) > n_before).then_some(())
        });
        assert!(reconnected.is_some(), "must reconnect (and re-hello) after each restart");
    }

    assert_eq!(
        connected_count.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "one initial connect + one fresh hello per reconnect == 3"
    );

    kill_tree(&mut body);
    kill_tree(&mut hub);
}
