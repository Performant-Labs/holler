//! The session-hold test rig (issues #442, #460): a real hub on an explicit
//! address (so a restart can rebind the same port), a real body joined to it
//! and hosting `stub-acp` sessions, and helpers over the hub's control socket.
//! Shared by `hold_hub_test.rs` and `join_held_test.rs`.

#![allow(dead_code)] // #442: each test binary uses a subset

use std::path::Path;
use std::process::{Child, Stdio};
use std::time::Duration;

use holler_hub::control::{self, ControlError};
use serde_json::Value;

use super::{holler_cmd, join, kill_tree, make_own_process_group, mint_token, wait_for, write_sessions_toml, Body, StateDir};

pub const READY: Duration = Duration::from_secs(30);
pub const SESSION: &str = "b/alpha";

/// A hub on an explicit address (so a restart can rebind the same port), a
/// body joined to it, and helpers over the control socket.
pub struct Rig {
    pub hub_state: StateDir,
    pub body_state: StateDir,
    addr: String,
    hub: Option<Child>,
    body: Option<Body>,
    config: std::path::PathBuf,
    hub_env: Vec<(String, String)>,
    hub_args: Vec<String>,
}

/// A loopback address for a hub that will be **restarted on the same port**.
/// An OS-assigned ephemeral port would do for a hub that never restarts, but
/// once the old hub is killed its port is free, and a parallel test asking the
/// OS for "any free port" can be handed exactly that one. So candidates come
/// from a range the OS does not hand out as ephemeral (Linux starts at 32768,
/// macOS at 49152), spread by pid and a counter and probed before use.
pub fn free_addr() -> String {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    loop {
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let port = 20_000 + (std::process::id().wrapping_mul(7919).wrapping_add(n.wrapping_mul(131))) % 10_000;
        let addr = format!("127.0.0.1:{port}");
        if std::net::TcpListener::bind(&addr).is_ok() {
            return addr;
        }
    }
}

pub fn start_hub_at(state: &StateDir, addr: &str, env: &[(String, String)], args: &[String]) -> Child {
    let mut cmd = holler_cmd(state);
    cmd.envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    cmd.args(["hub", "serve", "--listen", addr]).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn `holler hub serve`");
    let stderr = child.stderr.take().expect("hub stderr is piped");
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        let mut sent = false;
        // Keep draining for the hub's whole life so its stderr pipe never fills.
        while reader.read_line(&mut line).is_ok_and(|n| n > 0) {
            if !sent && serde_json::from_str::<Value>(&line).is_ok_and(|v| v["event"] == "listening") {
                sent = true;
                let _ = tx.send(());
            }
            line.clear();
        }
    });
    rx.recv_timeout(Duration::from_secs(10)).expect("hub did not report listening within 10s");
    child
}

impl Rig {
    pub fn start(sessions: &[(&str, &[&str])]) -> Rig {
        Self::start_with(sessions, &[])
    }

    pub fn start_with(sessions: &[(&str, &[&str])], hub_env: &[(&str, &str)]) -> Rig {
        Self::start_with_args(sessions, hub_env, &[])
    }

    /// [`Rig::start_with`], also passing `hub_args` to `holler hub serve`
    /// (e.g. `--join-held`).
    pub fn start_with_args(sessions: &[(&str, &[&str])], hub_env: &[(&str, &str)], hub_args: &[&str]) -> Rig {
        let hub_state = StateDir::new();
        let body_state = StateDir::new();
        let addr = free_addr();
        let hub_env: Vec<(String, String)> = hub_env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let hub_args: Vec<String> = hub_args.iter().map(|s| s.to_string()).collect();
        let hub = start_hub_at(&hub_state, &addr, &hub_env, &hub_args);
        let (token_id, secret) = mint_token(&hub_state, "b");
        join(&body_state, &hub_state, &format!("ws://{addr}"), &token_id, &secret);
        let config = write_sessions_toml(&body_state, sessions);
        let mut rig = Rig { hub_state, body_state, addr, hub: Some(hub), body: None, config, hub_env, hub_args };
        rig.start_body();
        for (name, _) in sessions {
            rig.wait_row(&format!("b/{name}"), |r| r["state"] == "idle");
        }
        rig
    }

    pub fn start_body(&mut self) {
        self.body = Some(Body::start_with_env(&self.body_state, &self.config, &[("HOLLER_HEARTBEAT_INTERVAL_MS", "300")]));
    }

    pub fn stop_body(&mut self) {
        if let Some(b) = self.body.take() {
            b.stop(&self.body_state, Duration::from_secs(5));
        }
    }

    /// An abrupt body death (not `body detach`, which un-joins the body): its
    /// state dir keeps the join, so `start_body` brings it back.
    pub fn kill_body(&mut self) {
        if let Some(mut b) = self.body.take() {
            kill_tree(b.child_mut());
        }
    }

    pub fn restart_hub(&mut self) {
        if let Some(mut h) = self.hub.take() {
            kill_tree(&mut h);
        }
        self.hub = Some(start_hub_at(&self.hub_state, &self.addr, &self.hub_env, &self.hub_args));
    }

    pub fn root(&self) -> &Path {
        self.hub_state.path()
    }

    pub fn hold(&self, s: &str, reason: Option<&str>) -> Result<Value, ControlError> {
        control::hold_at(self.root(), s, reason)
    }
    pub fn release(&self, s: &str) -> Result<Value, ControlError> {
        control::release_at(self.root(), s)
    }
    pub fn say(&self, s: &str, queue: bool) -> Result<Value, ControlError> {
        say_at_retrying(self.root(), s, "hi", queue)
    }
    pub fn rows(&self) -> Vec<Value> {
        control::roster_at(self.root(), true, None).map(|v| v["rows"].as_array().cloned().unwrap_or_default()).unwrap_or_default()
    }
    pub fn row(&self, name: &str) -> Option<Value> {
        self.rows().into_iter().find(|r| r["name"] == name)
    }
    pub fn wait_row(&self, name: &str, pred: impl Fn(&Value) -> bool) -> Value {
        wait_for(READY, || self.row(name).filter(|r| pred(r)))
            .unwrap_or_else(|| panic!("row {name} never matched; roster: {:?}", self.rows()))
    }
    /// Wait until a turn has been dispatched to `name` since `prev` (its
    /// `turn_id` moves the instant the hub sends the prompt, so unlike
    /// `state == working` this cannot be missed by a slow poller).
    pub fn wait_dispatched(&self, name: &str, prev: Option<Value>) {
        self.wait_row(name, |r| turn_accepted(r, prev.as_ref()));
    }
    pub fn turn_id(&self, name: &str) -> Option<Value> {
        self.row(name).map(|r| r["turn_id"].clone()).filter(|v| !v.is_null())
    }
    pub fn hold_file(&self) -> Value {
        let text = std::fs::read_to_string(self.hub_state.hub().join("holds.json")).unwrap_or_default();
        serde_json::from_str(&text).unwrap_or(Value::Null)
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.stop_body();
        if let Some(mut h) = self.hub.take() {
            kill_tree(&mut h);
        }
    }
}

/// `say`, retried while the hub answers `session_busy`. A busy refusal means
/// nothing was delivered, so a retry can never deliver twice; it is needed
/// because the hub's own presence cache can trail the roster row by a moment
/// after a turn ends, so a session the roster shows `idle` may still be
/// refused as `working` for that moment.
pub fn say_at_retrying(root: &Path, session: &str, text: &str, queue: bool) -> Result<Value, ControlError> {
    let deadline = std::time::Instant::now() + READY;
    loop {
        let res = control::say_at(root, session, text, queue, Duration::from_secs(60));
        let busy = matches!(&res, Err(ControlError::Refused(e)) if e.code == -32009);
        if !busy || std::time::Instant::now() >= deadline {
            return res;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A turn newer than `prev` has been *accepted* by the body: the roster shows
/// it `working` (or already finished). Seeing only that the hub moved the row's
/// `turn_id` is not enough: that happens just before the prompt is sent, and a
/// hold set in that instant still refuses it.
pub fn turn_accepted(row: &Value, prev: Option<&Value>) -> bool {
    let moved = !row["turn_id"].is_null() && Some(&row["turn_id"]) != prev;
    let finished = row["last_turn"]["turn_id"] == row["turn_id"];
    moved && (row["state"] == "working" || finished)
}

pub fn assert_held(res: Result<Value, ControlError>, reason: Option<&str>) {
    match res {
        Err(ControlError::Refused(e)) => {
            assert_eq!(e.code, -32011, "expected session_held, got {e:?}");
            let data = e.data.expect("session_held carries data");
            assert_eq!(data.code, "session_held");
            assert_eq!(data.reason.as_deref(), reason);
            assert!(data.since.is_some(), "since is set");
            if let Some(r) = reason {
                assert!(e.message.contains(r), "the message names the reason: {}", e.message);
            }
        }
        Ok(v) => panic!("expected session_held, but the prompt was delivered: {v}"),
        Err(e) => panic!("expected session_held, got {e}"),
    }
}

pub fn is_held(res: &Result<Value, ControlError>) -> bool {
    matches!(res, Err(ControlError::Refused(e)) if e.code == -32011)
}

pub fn is_delivered(res: &Result<Value, ControlError>) -> bool {
    res.as_ref().is_ok_and(|v| v["text"].as_str().is_some_and(|t| t.contains("stub chunk")))
}
