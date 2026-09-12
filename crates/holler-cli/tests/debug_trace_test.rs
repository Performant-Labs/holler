#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #197
//! RED/GREEN tests for `--debug noisy`/`--debug quiet` per-component
//! instrumentation (issue #197) — the issue's own test list, verbatim in
//! name — against a **real** hub + body + `stub-acp` pair (real
//! subprocesses, no mocks of the circuit), and a **fake** OpenCode HTTP
//! server for the attach-mode leaf (reusing the exact fake server issue
//! #194/#195 built, per this crate's own established `#[path]`-include
//! convention — see `attach_mode_test.rs`'s own module doc).
//!
//! # Why this file needs its own hub/body spawn helpers
//!
//! `support::Hub`/`support::Body` deliberately discard (or never even pipe)
//! their child's stderr once past their own readiness signal — the harness
//! was built for tests that only care about *behavior*, not the log lines
//! themselves. This file is the first one that needs to read the captured
//! trace back, so it spawns the hub/body processes itself (mirroring
//! `support::Hub::start_with_env`/`Body::start_with_env`'s own plumbing —
//! the same duplication precedent `interrupt_test.rs`'s module doc already
//! establishes for this crate's test files) with stderr piped to a
//! background drainer thread that appends every line to a shared buffer
//! instead of dropping it.
//!
//! # "for the body"/"for the hub" component lists
//!
//! The issue's own RED list says the combined trace shows `control, wire,
//! session, acp` for the body and `control, wire, talklog` for the hub. The
//! body has no control socket of its own (only the hub does — `component=
//! control` is entirely a hub-side concept, see `holler-hub/src/control_
//! server.rs`), so this reads as one causal chain across *both* processes:
//! the CLI's one-shot `say`/`interrupt` reaches the hub's control socket
//! first (`control`), the hub relays `session/prompt` over the wire
//! (`wire`), the body's session task picks it up (`session`), and its ACP
//! driver dispatches to the child (`acp`) — with `wire`/`talklog` also
//! showing up again on the hub's own side (the body's `session/update`
//! reply, and the hub appending it to the talklog). This file asserts:
//! hub trace contains `control`, `wire`, `talklog` in that causal order;
//! body trace contains `wire`, `session`, `acp` in that causal order.

#[path = "../../holler-body/tests/http_attach_driver_test/fake_server.rs"]
mod fake_server;
mod support;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fake_server::{message_updated_assistant, part_updated_text, session_idle, FakeServer};
use serde_json::Value;

use support::{holler_bin, make_own_process_group, mint_token, wait_for, StateDir, STARTUP_WAIT};

/// Lines captured off a subprocess's stderr, shared with the background
/// drainer thread that keeps appending to it for the process's whole life.
type Captured = Arc<Mutex<Vec<String>>>;

fn spawn_capturing(mut cmd: Command) -> (Child, Captured) {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn subprocess");
    let stderr = child.stderr.take().expect("stderr is piped");
    let lines: Captured = Arc::new(Mutex::new(Vec::new()));
    let lines_writer = lines.clone();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim_end().to_string();
                    if !trimmed.is_empty() {
                        lines_writer.lock().unwrap().push(trimmed);
                    }
                }
            }
        }
    });
    (child, lines)
}

/// Spawn `holler hub serve` with `--debug`/`--log-format` overridden by
/// `debug_level`, capturing its stderr; blocks until the `listening` event
/// is observed in the captured lines (mirrors `support::Hub::start`'s own
/// readiness wait, but this file needs the captured lines themselves rather
/// than just the parsed port).
fn start_hub_capturing(state: &StateDir, debug_level: &str) -> (Child, u16, Captured) {
    let mut cmd = Command::new(holler_bin());
    cmd.env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_DEBUG", debug_level)
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["hub", "serve", "--listen", "127.0.0.1:0"]);
    let (child, lines) = spawn_capturing(cmd);
    let port = wait_for(Duration::from_secs(10), || {
        let guard = lines.lock().unwrap();
        guard.iter().find_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            if v.get("event").and_then(|e| e.as_str()) == Some("listening") {
                v.get("addr").and_then(|a| a.as_str()).and_then(|a| a.rsplit(':').next()).and_then(|p| p.parse().ok())
            } else {
                None
            }
        })
    })
    .unwrap_or_else(|| panic!("hub never reported a listening port"));
    (child, port, lines)
}

/// Spawn `holler body run` against `config`, capturing its stderr.
fn start_body_capturing(state: &StateDir, config: &Path, debug_level: &str) -> (Child, Captured) {
    let mut cmd = Command::new(holler_bin());
    cmd.env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_DEBUG", debug_level)
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["body", "run", "--config"])
        .arg(config);
    spawn_capturing(cmd)
}

/// Join `state`'s body to the hub at `ws_url`, minting a fresh token labeled
/// `label` (mirrors `attach_mode_test.rs`'s own `join_fresh`).
fn join_fresh(state: &StateDir, ws_url: &str, label: &str) {
    let (token_id, secret) = mint_token(state, label);
    let token = format!("{token_id}:{secret}");
    let hub_key = support::hub_pubkey(state);
    let out = Command::new(holler_bin())
        .env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["body", "join", "--server", ws_url, "--token", &token, "--hub-key", &hub_key])
        .output()
        .expect("run `body join`");
    assert!(out.status.success(), "body join failed: {}", String::from_utf8_lossy(&out.stderr));
}

fn write_sessions_toml(state: &StateDir, sessions: &[(&str, &[&str])]) -> PathBuf {
    let stub = support::stub_acp_bin();
    let mut toml = String::new();
    for (name, extra) in sessions {
        let argv = std::iter::once(stub)
            .chain(extra.iter().copied())
            .map(|a| serde_json::to_string(a).expect("json-encode argv element"))
            .collect::<Vec<_>>()
            .join(", ");
        toml.push_str(&format!(
            "[[session]]\nname = {name_q}\nharness = \"opencode\"\ncommand = [{argv}]\n",
            name_q = serde_json::to_string(name).expect("json-encode name"),
        ));
    }
    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).expect("create body dir");
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

fn attach_sessions_toml(state: &StateDir, name: &str, endpoint: &str, session_id: &str) -> PathBuf {
    let toml = format!(
        "[[session]]\nname = {name_q}\nharness = \"opencode\"\nmode = \"attach\"\nendpoint = {endpoint_q}\nsession_id = {session_id_q}\n",
        name_q = serde_json::to_string(name).unwrap(),
        endpoint_q = serde_json::to_string(endpoint).unwrap(),
        session_id_q = serde_json::to_string(session_id).unwrap(),
    );
    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).expect("create body dir");
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

fn stop_capturing(mut child: Child) {
    let pid = child.id() as i32;
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGINT);
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.wait();
}

/// Parse every captured line as a JSON log event and collect the
/// `component` values, in the order they appeared — `None` for a line that
/// is not a JSON object with a `component` field (defensive; every log line
/// this suite cares about is one).
fn components_in_order(lines: &Captured) -> Vec<String> {
    lines
        .lock()
        .unwrap()
        .iter()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            v.get("component").and_then(|c| c.as_str()).map(str::to_string)
        })
        .collect()
}

fn first_index(components: &[String], name: &str) -> Option<usize> {
    components.iter().position(|c| c == name)
}

fn wait_for_component(lines: &Captured, component: &str, timeout: Duration) {
    let found = wait_for(timeout, || {
        if components_in_order(lines).iter().any(|c| c == component) {
            Some(())
        } else {
            None
        }
    });
    assert!(
        found.is_some(),
        "component {component:?} never appeared in trace: {:?}",
        lines.lock().unwrap().clone()
    );
}

#[test]
fn noisy_trace_of_say_contains_all_components_in_order() {
    let state = StateDir::new();
    let (hub_child, port, hub_lines) = start_hub_capturing(&state, "noisy");
    let ws_url = format!("ws://127.0.0.1:{port}");
    join_fresh(&state, &ws_url, "alpha");
    let config = write_sessions_toml(&state, &[("alpha", &[])]);
    let (body_child, body_lines) = start_body_capturing(&state, &config, "noisy");

    // Wait for the session to actually register on the roster before `say`
    // (a body that hasn't heartbeated yet reports `unknown_session`).
    let ready = wait_for(STARTUP_WAIT, || {
        let out = support::say(&state, "alpha", "hello");
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() {
            Some(out)
        } else if stderr.contains("unknown session") || stderr.contains("not_connected") {
            None
        } else {
            Some(out)
        }
    });
    let out = ready.unwrap_or_else(|| panic!("`say alpha` never became reachable"));
    assert!(out.status.success(), "say failed: {}", String::from_utf8_lossy(&out.stderr));

    wait_for_component(&hub_lines, "talklog", Duration::from_secs(5));
    wait_for_component(&body_lines, "acp", Duration::from_secs(5));

    let hub_components = components_in_order(&hub_lines);
    let body_components = components_in_order(&body_lines);

    for want in ["control", "wire", "talklog"] {
        assert!(hub_components.contains(&want.to_string()), "hub trace missing {want:?}: {hub_components:?}");
    }
    for want in ["wire", "session", "acp"] {
        assert!(body_components.contains(&want.to_string()), "body trace missing {want:?}: {body_components:?}");
    }

    // Causal order (issue #197's own RED wording: "control received first,
    // wire frame logged, session state change, acp dispatch") — checked
    // relative to the FIRST `control` event, not the trace's very first
    // `wire` line: the hub's own connection handshake (the body's initial
    // `circuit/join`/heartbeats) legitimately logs `wire` frames before any
    // `say` ever runs, so "control precedes wire" means "a wire frame
    // follows this say's own control request", not "no wire frame ever
    // precedes it".
    let hub_control = first_index(&hub_components, "control").unwrap();
    assert!(
        hub_components.iter().skip(hub_control + 1).any(|c| c == "wire"),
        "hub: no wire frame followed the first control event: {hub_components:?}"
    );

    let body_wire = first_index(&body_components, "wire").unwrap();
    let body_session = body_components.iter().skip(body_wire + 1).position(|c| c == "session");
    let body_session = body_session.map(|i| i + body_wire + 1);
    assert!(body_session.is_some(), "body: no session event followed the first wire event: {body_components:?}");
    let body_session = body_session.unwrap();
    assert!(
        body_components.iter().skip(body_session + 1).any(|c| c == "acp"),
        "body: no acp event followed the first session event: {body_components:?}"
    );

    stop_capturing(body_child);
    stop_capturing(hub_child);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn noisy_trace_of_attach_say_contains_http_attach() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let (hub_child, port, _hub_lines) = start_hub_capturing(&state, "noisy");
    let ws_url = format!("ws://127.0.0.1:{port}");
    join_fresh(&state, &ws_url, "beta");
    let config = attach_sessions_toml(&state, "beta", &server.endpoint(), "ses_trace");
    let (body_child, body_lines) = start_body_capturing(&state, &config, "noisy");

    // Wait for the attach session to come up (its own initial attach
    // succeeding against the fake server) before `say`.
    let _ = wait_for(STARTUP_WAIT, || {
        let doc = support::roster_json(&state);
        doc.get("rows")
            .and_then(|r| r.as_array())
            .and_then(|rows| rows.iter().find(|r| r.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.ends_with("/beta"))))
            .cloned()
    });

    let out = std::thread::scope(|scope| {
        scope.spawn(|| {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while std::time::Instant::now() < deadline {
                if server.requests().iter().any(|r| r.method == "POST" && r.path.ends_with("/prompt_async")) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            server.push_event(message_updated_assistant("m1", "ses_trace"));
            server.push_event(part_updated_text("m1", "ses_trace", "hi from attach"));
            server.push_event(session_idle("ses_trace"));
        });
        support::say(&state, "beta", "hi")
    });
    assert!(out.status.success(), "say to attach session failed: {}", String::from_utf8_lossy(&out.stderr));

    wait_for_component(&body_lines, "http_attach", Duration::from_secs(5));

    stop_capturing(body_child);
    stop_capturing(hub_child);
}

#[test]
fn quiet_has_no_frames_noisy_has_frames() {
    let state = StateDir::new();
    let (hub_child, port, hub_lines) = start_hub_capturing(&state, "quiet");
    let ws_url = format!("ws://127.0.0.1:{port}");
    join_fresh(&state, &ws_url, "gamma");
    let config = write_sessions_toml(&state, &[("gamma", &[])]);
    let (body_child, _body_lines) = start_body_capturing(&state, &config, "quiet");

    let ready = wait_for(STARTUP_WAIT, || {
        let out = support::say(&state, "gamma", "hello quiet");
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() {
            Some(out)
        } else if stderr.contains("unknown session") || stderr.contains("not_connected") {
            None
        } else {
            Some(out)
        }
    });
    let out = ready.unwrap_or_else(|| panic!("`say gamma` never became reachable"));
    assert!(out.status.success(), "say failed: {}", String::from_utf8_lossy(&out.stderr));

    wait_for_component(&hub_lines, "wire", Duration::from_secs(5));

    // `quiet`: every wire-shaped debug line must carry no `frame` field at
    // all (issue #197's own contract: quiet is shape-only).
    let quiet_had_frame = hub_lines.lock().unwrap().iter().any(|l| {
        serde_json::from_str::<Value>(l).ok().is_some_and(|v| v.get("frame").is_some())
    });
    assert!(!quiet_had_frame, "quiet must never carry a `frame` field: {:?}", hub_lines.lock().unwrap());

    stop_capturing(body_child);
    stop_capturing(hub_child);

    // `noisy`: the same round trip must carry at least one `frame` field.
    let state2 = StateDir::new();
    let (hub_child2, port2, hub_lines2) = start_hub_capturing(&state2, "noisy");
    let ws_url2 = format!("ws://127.0.0.1:{port2}");
    join_fresh(&state2, &ws_url2, "delta");
    let config2 = write_sessions_toml(&state2, &[("delta", &[])]);
    let (body_child2, _body_lines2) = start_body_capturing(&state2, &config2, "noisy");

    let ready2 = wait_for(STARTUP_WAIT, || {
        let out = support::say(&state2, "delta", "hello noisy");
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() {
            Some(out)
        } else if stderr.contains("unknown session") || stderr.contains("not_connected") {
            None
        } else {
            Some(out)
        }
    });
    let out2 = ready2.unwrap_or_else(|| panic!("`say delta` never became reachable"));
    assert!(out2.status.success(), "say failed: {}", String::from_utf8_lossy(&out2.stderr));

    wait_for_component(&hub_lines2, "wire", Duration::from_secs(5));
    let noisy_had_frame = hub_lines2.lock().unwrap().iter().any(|l| {
        serde_json::from_str::<Value>(l).ok().is_some_and(|v| v.get("frame").is_some())
    });
    assert!(noisy_had_frame, "noisy must carry at least one `frame` field: {:?}", hub_lines2.lock().unwrap());

    stop_capturing(body_child2);
    stop_capturing(hub_child2);
}
