#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #351
//! `holler body confirm` (issue #351): the one-time, interactive operator
//! confirmation gate for a pairing's Short Authentication String — and the
//! two behaviors the issue's acceptance criteria pin:
//!
//! 1. A first-time pairing (no `sas_confirmed` on record) still lets `body
//!    run`'s automatic reconnect loop connect fully non-interactively (the
//!    design decision documented in `holler_body::connection::
//!    connect_and_serve`: a soft `sas_unconfirmed` warning log line, never a
//!    blocking prompt or a hard refusal) — no regression to #339/#350.
//! 2. `body confirm` is the only thing that ever sets `sas_confirmed`, it is
//!    interactive (reads a real y/n off stdin), and once set, every later
//!    reconnect (including across a hub restart) proceeds without ever
//!    touching stdin again.
//!
//! Real subprocesses, real loopback sockets — no mocks of the circuit (the
//! same discipline `body_run_test.rs`/`body_join_test.rs` use).

mod support;

use std::io::Write;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use support::{body_status_json, holler_cmd, kill_tree, make_own_process_group, wait_for, Hub, StateDir};

const READY: Duration = Duration::from_secs(10);

fn connection_state_path(state: &StateDir) -> std::path::PathBuf {
    state.body().join("connection_state.json")
}

fn conn_state(state: &StateDir) -> Option<String> {
    let bytes = std::fs::read(connection_state_path(state)).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get("state").and_then(|s| s.as_str()).map(str::to_owned)
}

/// Join `state`'s body to `ws_url` with a freshly-minted token, pinning the
/// hub's real key (mirrors `body_run_test.rs`'s own `join_fresh`).
fn join_fresh(state: &StateDir, ws_url: &str, label: &str) -> String {
    let (token_id, secret) = support::mint_token(state, label);
    let hub_key = support::hub_pubkey(state);
    let out = holler_cmd(state)
        .args(["body", "join", "--server", ws_url, "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn body join")
        .wait_with_output()
        .expect("wait body join");
    assert!(out.status.success(), "body join must exit 0: {}", String::from_utf8_lossy(&out.stderr));
    token_id
}

/// Spawn `holler body run` with an empty session config, in its own process
/// group, with stdin **null** (issue #351's whole point: the automatic
/// reconnect loop must never read stdin regardless of `sas_confirmed`) and
/// stderr piped so tests can watch its log lines.
fn spawn_run_with_piped_stderr(state: &StateDir) -> std::process::Child {
    let config = support::write_sessions_toml(state, &[]);
    let mut cmd = holler_cmd(state);
    cmd.env("HOLLER_DEBUG", "noisy")
        .arg("body")
        .arg("run")
        .arg("--config")
        .arg(&config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    cmd.spawn().expect("spawn `holler body run`")
}

/// Drain `stderr` on a background thread, incrementing `counter` once per
/// line containing `needle`. Mirrors `body_run_test.rs`'s
/// `fresh_hello_and_presence_on_every_reconnect` drainer.
fn watch_for(stderr: std::process::ChildStderr, needle: &'static str) -> std::sync::Arc<std::sync::atomic::AtomicUsize> {
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c = counter.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if line.contains(needle) {
                        c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        }
    });
    counter
}

/// Run `holler body confirm` to completion with `answer` written to its
/// stdin (e.g. `"y\n"`, `"n\n"`), returning (exit code, stdout, stderr).
fn run_confirm(state: &StateDir, answer: &str) -> (i32, String, String) {
    let mut child = holler_cmd(state)
        .args(["body", "confirm"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `holler body confirm`");
    child
        .stdin
        .take()
        .expect("confirm stdin piped")
        .write_all(answer.as_bytes())
        .expect("write the operator's answer");
    let out = child.wait_with_output().expect("wait on `holler body confirm`");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Acceptance criterion: "every automatic reconnect … remains fully
/// non-interactive and non-blocking". A fresh pairing (never confirmed) must
/// still reach `connected` when `body run` is spawned with stdin **null** —
/// if the reconnect loop ever tried to read a confirmation off stdin, it
/// would block forever on a closed pipe and this would time out. It also
/// logs the documented `sas_unconfirmed` warning exactly once per connect.
#[test]
fn unconfirmed_first_pairing_connects_without_blocking_and_warns() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");

    let mut body = spawn_run_with_piped_stderr(&state);
    let stderr = body.stderr.take().expect("body stderr is piped");
    let warnings = watch_for(stderr, "sas_unconfirmed");

    let connected = wait_for(READY, || (conn_state(&state).as_deref() == Some("connected")).then_some(()));
    assert!(connected.is_some(), "an unconfirmed pairing must still reach `connected` (non-blocking)");

    let warned = wait_for(READY, || (warnings.load(std::sync::atomic::Ordering::SeqCst) >= 1).then_some(()));
    assert!(warned.is_some(), "an unconfirmed connect must log the sas_unconfirmed warning");

    // And `body status` agrees: joined, but not yet confirmed.
    let doc = body_status_json(&state);
    assert_eq!(doc["joined"].as_bool(), Some(true));
    assert_eq!(doc["sas_confirmed"].as_bool(), Some(false), "a never-confirmed pairing reports sas_confirmed:false: {doc}");

    kill_tree(&mut body);
    hub.stop(Duration::from_secs(5));
}

/// Acceptance criteria 1+2: a first-time pairing requires an explicit
/// operator confirmation step, and it persists (asked at most once). This
/// drives `body confirm` over a real piped stdin/stdout against a live hub
/// (the SAS only exists once a real Noise handshake completes), answers
/// "y", and checks the flag lands in `body status`.
#[test]
fn confirm_with_yes_persists_sas_confirmed() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");

    let (code, stdout, stderr) = run_confirm(&state, "y\n");
    assert_eq!(code, 0, "confirm with y must exit 0; stdout={stdout} stderr={stderr}");
    assert!(stdout.contains("confirmed"), "confirm must tell the operator it succeeded: {stdout}");

    let doc = body_status_json(&state);
    assert_eq!(doc["sas_confirmed"].as_bool(), Some(true), "confirm must persist sas_confirmed:true: {doc}");

    // Idempotent: confirming again short-circuits (no connection attempted —
    // stopping the hub first proves it) and still exits 0.
    hub.stop(Duration::from_secs(5));
    let child = holler_cmd(&state)
        .args(["body", "confirm"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `holler body confirm` again");
    let out = child.wait_with_output().expect("wait on second confirm");
    assert!(out.status.success(), "an already-confirmed pairing's re-confirm must still exit 0 (idempotent), with no hub reachable: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("already confirmed"),
        "a second confirm must say it is already confirmed: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// The operator can decline: nothing is persisted, and the pairing remains
/// unconfirmed (so it keeps warning, and a later `body confirm` can still
/// run).
#[test]
fn confirm_with_no_does_not_persist() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");

    let (code, stdout, stderr) = run_confirm(&state, "n\n");
    assert_eq!(code, 1, "declining confirmation must exit 1; stdout={stdout} stderr={stderr}");

    let doc = body_status_json(&state);
    assert_eq!(doc["sas_confirmed"].as_bool(), Some(false), "a declined confirmation must not persist: {doc}");

    hub.stop(Duration::from_secs(5));
}

/// Acceptance criterion 3: once confirmed, every later automatic reconnect —
/// including across a real hub restart — proceeds without ever touching
/// stdin again (the running `body run` process's stdin was `Stdio::null()`
/// from the moment it was spawned, *before* confirmation happened, so a
/// successful reconnect after restart is direct proof the reconnect path
/// never blocks on the now-set flag either).
#[test]
fn confirmed_pairing_reconnects_freely_after_hub_restart() {
    let state = StateDir::new();
    let addr = "127.0.0.1:41919";
    let mut hub = start_hub_at(&state, addr);
    join_fresh(&state, &format!("ws://{addr}"), "kiwi");

    let (code, _stdout, stderr) = run_confirm(&state, "y\n");
    assert_eq!(code, 0, "confirm must succeed before the reconnect exercise: {stderr}");

    let mut body = spawn_run_with_piped_stderr(&state);
    let stderr_pipe = body.stderr.take().expect("body stderr is piped");
    let unconfirmed_warnings = watch_for(stderr_pipe, "sas_unconfirmed");

    let first = wait_for(READY, || (conn_state(&state).as_deref() == Some("connected")).then_some(()));
    assert!(first.is_some(), "must connect once before the restart");

    kill_tree(&mut hub);
    let mut hub2 = start_hub_at(&state, addr);
    let reconnected = wait_for(Duration::from_secs(35), || {
        (conn_state(&state).as_deref() == Some("connected")).then_some(())
    });
    assert!(reconnected.is_some(), "an already-confirmed pairing must reconnect after a hub restart without blocking");

    assert_eq!(
        unconfirmed_warnings.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a confirmed pairing must never log sas_unconfirmed, on the first connect or any reconnect"
    );

    kill_tree(&mut body);
    kill_tree(&mut hub2);
}

/// Spawn a hub bound to a fixed address (not port 0) so a later restart can
/// rebind the same address — mirrors `body_run_test.rs`'s private helper of
/// the same name (kept local rather than shared, per that file's own note
/// that its harness split is for its own line-count budget, not a shared
/// API).
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
