#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #442
//! Session hold, hub side (issue #442, umbrella #437), against a real hub, a
//! real body and `stub-acp`: no mocks of the circuit, and a real hub restart.
//! Tags: `test-grp-concurrency` (the race matrix) and lifecycle (the rest).
//!
//! The hold is set and cleared through the control socket
//! (`holler_hub::control::hold_at` / `release_at`; the CLI verbs are #443), and
//! prompts are sent the way `holler say` sends them (`control::say_at`).
//!
//! No fixed sleep is used as synchronisation: every wait polls an observable
//! outcome (a roster row, a reply) with a deadline, and the races synchronise
//! on a barrier. The invariant the races assert is never a timing: every
//! `say` is either delivered (a reply came back) or refused `session_held`,
//! never both and never lost, and a `say` started after a `hold` returned is
//! always refused.

mod support;

use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use holler_hub::control::{self, ControlError};
use serde_json::Value;
use support::{holler_cmd, join, kill_tree, make_own_process_group, mint_token, wait_for, write_sessions_toml, Body, StateDir};

const READY: Duration = Duration::from_secs(30);
const SESSION: &str = "b/alpha";

/// A hub on an explicit address (so a restart can rebind the same port), a
/// body joined to it, and helpers over the control socket.
struct Rig {
    hub_state: StateDir,
    body_state: StateDir,
    addr: String,
    hub: Option<Child>,
    body: Option<Body>,
    config: std::path::PathBuf,
    hub_env: Vec<(String, String)>,
}

fn free_addr() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let a = l.local_addr().unwrap();
    drop(l);
    a.to_string()
}

fn start_hub_at(state: &StateDir, addr: &str, env: &[(String, String)]) -> Child {
    let mut cmd = holler_cmd(state);
    cmd.envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    cmd.args(["hub", "serve", "--listen", addr]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
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
    fn start(sessions: &[(&str, &[&str])]) -> Rig {
        Self::start_with(sessions, &[])
    }

    fn start_with(sessions: &[(&str, &[&str])], hub_env: &[(&str, &str)]) -> Rig {
        let hub_state = StateDir::new();
        let body_state = StateDir::new();
        let addr = free_addr();
        let hub_env: Vec<(String, String)> = hub_env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let hub = start_hub_at(&hub_state, &addr, &hub_env);
        let (token_id, secret) = mint_token(&hub_state, "b");
        join(&body_state, &hub_state, &format!("ws://{addr}"), &token_id, &secret);
        let config = write_sessions_toml(&body_state, sessions);
        let mut rig = Rig { hub_state, body_state, addr, hub: Some(hub), body: None, config, hub_env };
        rig.start_body();
        for (name, _) in sessions {
            rig.wait_row(&format!("b/{name}"), |r| r["state"] == "idle");
        }
        rig
    }

    fn start_body(&mut self) {
        self.body = Some(Body::start_with_env(&self.body_state, &self.config, &[("HOLLER_HEARTBEAT_INTERVAL_MS", "300")]));
    }

    fn stop_body(&mut self) {
        if let Some(b) = self.body.take() {
            b.stop(&self.body_state, Duration::from_secs(5));
        }
    }

    /// An abrupt body death (not `body detach`, which un-joins the body): its
    /// state dir keeps the join, so `start_body` brings it back.
    fn kill_body(&mut self) {
        if let Some(mut b) = self.body.take() {
            kill_tree(b.child_mut());
        }
    }

    fn restart_hub(&mut self) {
        if let Some(mut h) = self.hub.take() {
            kill_tree(&mut h);
        }
        self.hub = Some(start_hub_at(&self.hub_state, &self.addr, &self.hub_env));
    }

    fn root(&self) -> &Path {
        self.hub_state.path()
    }

    fn hold(&self, s: &str, reason: Option<&str>) -> Result<Value, ControlError> {
        control::hold_at(self.root(), s, reason)
    }
    fn release(&self, s: &str) -> Result<Value, ControlError> {
        control::release_at(self.root(), s)
    }
    fn say(&self, s: &str, queue: bool) -> Result<Value, ControlError> {
        say_at_retrying(self.root(), s, "hi", queue)
    }
    fn rows(&self) -> Vec<Value> {
        control::roster_at(self.root(), true, None).map(|v| v["rows"].as_array().cloned().unwrap_or_default()).unwrap_or_default()
    }
    fn row(&self, name: &str) -> Option<Value> {
        self.rows().into_iter().find(|r| r["name"] == name)
    }
    fn wait_row(&self, name: &str, pred: impl Fn(&Value) -> bool) -> Value {
        wait_for(READY, || self.row(name).filter(|r| pred(r)))
            .unwrap_or_else(|| panic!("row {name} never matched; roster: {:?}", self.rows()))
    }
    /// Wait until a turn has been dispatched to `name` since `prev` (its
    /// `turn_id` moves the instant the hub sends the prompt, so unlike
    /// `state == working` this cannot be missed by a slow poller).
    fn wait_dispatched(&self, name: &str, prev: Option<Value>) {
        self.wait_row(name, |r| turn_accepted(r, prev.as_ref()));
    }
    fn turn_id(&self, name: &str) -> Option<Value> {
        self.row(name).map(|r| r["turn_id"].clone()).filter(|v| !v.is_null())
    }
    fn hold_file(&self) -> Value {
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
fn say_at_retrying(root: &Path, session: &str, text: &str, queue: bool) -> Result<Value, ControlError> {
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
fn turn_accepted(row: &Value, prev: Option<&Value>) -> bool {
    let moved = !row["turn_id"].is_null() && Some(&row["turn_id"]) != prev;
    let finished = row["last_turn"]["turn_id"] == row["turn_id"];
    moved && (row["state"] == "working" || finished)
}

fn assert_held(res: Result<Value, ControlError>, reason: Option<&str>) {
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

fn is_held(res: &Result<Value, ControlError>) -> bool {
    matches!(res, Err(ControlError::Refused(e)) if e.code == -32011)
}

fn is_delivered(res: &Result<Value, ControlError>) -> bool {
    res.as_ref().is_ok_and(|v| v["text"].as_str().is_some_and(|t| t.contains("stub chunk")))
}

// --- functional ------------------------------------------------------------------

#[test]
fn a_held_session_refuses_every_delivery_variant_and_others_are_unaffected() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    let out = rig.hold(SESSION, Some("deploy freeze")).unwrap();
    assert_eq!(out["hold"], true);
    assert_eq!(out["newly_held"], true);
    assert_eq!(out["persisted"], true);

    assert_held(rig.say(SESSION, false), Some("deploy freeze"));
    assert_held(rig.say(SESSION, true), Some("deploy freeze"));
    // The bare name resolves to the same session.
    assert_held(rig.say("alpha", false), Some("deploy freeze"));
    // `--replace` is the redirect half of `interrupt SESSION TEXT`.
    assert_held(control::interrupt_at(rig.root(), SESSION, Some("do this instead")), Some("deploy freeze"));

    // An unheld session is unaffected, in both variants.
    assert!(is_delivered(&rig.say("b/beta", false)));
    assert!(is_delivered(&rig.say("b/beta", true)));

    // A refusal delivered nothing: releasing makes the session usable at once,
    // with no turn or queue entry left behind by the refused prompts.
    rig.release(SESSION).unwrap();
    assert!(is_delivered(&rig.say(SESSION, false)));
}

#[test]
fn a_prompt_cannot_be_forwarded_around_the_hold() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    rig.hold(SESSION, None).unwrap();
    let prompt = serde_json::json!({
        "session": "alpha",
        "message": { "messageId": "m-x", "role": "ROLE_USER", "parts": [{ "text": "sneak" }] },
    });
    let res = control::query_remote_at(rig.root(), "b", "session/prompt", Some(prompt));
    match res {
        Err(ControlError::Refused(e)) => assert_eq!(e.code, -32601, "a prompt is not a forwardable method: {e:?}"),
        other => panic!("the generic forward must refuse a session/prompt: {other:?}"),
    }
    // And the session saw nothing: the roster row never left idle.
    assert_eq!(rig.row(SESSION).unwrap()["state"], "idle");
}

#[test]
fn the_running_turn_the_accepted_queue_and_interrupt_are_not_touched() {
    let rig = Rig::start(&[("alpha", &["--slow", "--chunks", "8"])]);
    let root = rig.root().to_path_buf();
    let prev = rig.turn_id(SESSION);
    let first = std::thread::spawn({
        let root = root.clone();
        move || say_at_retrying(&root, SESSION, "long turn", false)
    });
    rig.wait_dispatched(SESSION, prev);
    let queued = std::thread::spawn({
        let root = root.clone();
        move || say_at_retrying(&root, SESSION, "queued before the hold", true)
    });
    // The queued prompt was accepted (forwarded to the body) once the body's
    // own presence shows the session still working after a queue call was
    // made; poll the hub's talk log, which records a prompt when it is sent.
    let talklog = rig.hub_state.hub().join("talklog");
    let accepted = wait_for(READY, || {
        let n: usize = std::fs::read_dir(&talklog)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default().lines().filter(|l| l.contains("\"text\"")).count())
            .sum();
        (n >= 2).then_some(())
    });
    assert!(accepted.is_some(), "the queued prompt was never sent");

    rig.hold(SESSION, Some("drain")).unwrap();
    // New work is refused while both accepted turns still run.
    assert_held(rig.say(SESSION, true), Some("drain"));
    // The running turn completes and the queued one still runs.
    assert!(is_delivered(&first.join().unwrap()), "the running turn must complete");
    assert!(is_delivered(&queued.join().unwrap()), "the prompt accepted before the hold must still run");
    // Interrupt (no text) still works on a held session.
    let out = control::interrupt_at(rig.root(), SESSION, None).unwrap();
    assert_eq!(out["applied"], true);
    // And the session is still held afterwards.
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

#[test]
fn hold_and_release_are_idempotent_and_keep_the_first_reason() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let a = rig.hold(SESSION, Some("first")).unwrap();
    let b = rig.hold(SESSION, Some("second")).unwrap();
    assert_eq!(a["newly_held"], true);
    assert_eq!(b["newly_held"], false);
    assert_eq!(b["reason"], "first", "a repeated hold keeps the original reason");
    assert_eq!(b["since"], a["since"], "and the original since-time");
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], true);
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], false);
    assert_eq!(rig.hold(SESSION, None).unwrap()["newly_held"], true);
    assert_eq!(rig.release(SESSION).unwrap()["hold"], false);
    assert!(is_delivered(&rig.say(SESSION, false)));
}

#[test]
fn holding_or_releasing_an_unknown_session_is_refused() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for res in [rig.hold("b/nope", None), rig.release("b/nope"), rig.hold("nope", Some("x"))] {
        match res {
            Err(ControlError::Refused(e)) => assert_eq!(e.code, -32003, "unknown_session: {e:?}"),
            other => panic!("expected unknown_session, got {other:?}"),
        }
    }
    assert!(rig.hold_file().is_null(), "no phantom hold was recorded");
}

#[test]
fn the_roster_shows_the_hold_and_a_held_idle_session_stays_held() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("deploy freeze")).unwrap();
    let held = rig.row(SESSION).unwrap();
    assert_eq!(held["state"], "idle", "the hold is orthogonal to the session's own state");
    assert_eq!(held["hold"], true);
    assert_eq!(held["hold_reason"], "deploy freeze");
    assert!(held["held_since"].as_str().is_some_and(|s| s.ends_with('Z')));
    // A session that is not held carries none of the fields (byte-identical to before).
    let open = rig.row("b/beta").unwrap();
    for f in ["hold", "hold_reason", "held_since"] {
        assert!(open.get(f).is_none(), "{f} must be absent on an unheld row: {open}");
    }
    // Still held (and still idle) after presence beats have refreshed the row.
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

// --- lifecycle: survives disconnect, re-join, prune, restart ---------------------

#[test]
fn a_hold_survives_body_disconnect_and_rejoin() {
    let mut rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("keep")).unwrap();
    rig.kill_body();
    rig.wait_row(SESSION, |r| r["conn_state"] != "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "held while disconnected");
    rig.start_body();
    rig.wait_row(SESSION, |r| r["conn_state"] == "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "a re-joining body is still held");
    assert_held(rig.say(SESSION, false), Some("keep"));
}

#[test]
fn a_hold_survives_a_roster_prune() {
    // A short TTL so the row is pruned within the test.
    let mut rig = Rig::start_with(
        &[("alpha", &["--chunks", "1"])],
        &[("HOLLER_ROSTER_RECONNECT_MS", "200"), ("HOLLER_ROSTER_GONE_MS", "400"), ("HOLLER_ROSTER_PRUNE_MS", "800"), ("HOLLER_ROSTER_SWEEP_MS", "100")],
    );
    rig.hold(SESSION, Some("keep")).unwrap();
    rig.kill_body();
    let pruned = wait_for(READY, || rig.row(SESSION).is_none().then_some(()));
    assert!(pruned.is_some(), "the row was never pruned");
    // The hold outlived its row: it can still be released by name, and the
    // registry still has it.
    assert!(rig.hold_file()["holds"].get(SESSION).is_some());
    rig.start_body();
    rig.wait_row(SESSION, |r| r["conn_state"] == "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "the re-created row is held again");
    assert_held(rig.say(SESSION, false), Some("keep"));
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], true);
}

#[test]
fn holds_survive_a_real_hub_restart_and_a_rejoining_body_stays_held() {
    let mut rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"]), ("gamma", &["--chunks", "1"])]);
    let since = rig.hold("b/alpha", Some("one")).unwrap()["since"].clone();
    rig.hold("b/beta", None).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(rig.hub_state.hub().join("holds.json")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the hold state file is private like the hub's other state");
    }

    rig.restart_hub(); // a real process kill and a fresh `holler hub serve`
    let reconnected = wait_for(Duration::from_secs(40), || {
        rig.row("b/gamma").filter(|r| r["conn_state"] == "connected")
    });
    assert!(reconnected.is_some(), "the body never re-joined the restarted hub");

    let alpha = rig.row("b/alpha").unwrap();
    assert_eq!(alpha["hold"], true);
    assert_eq!(alpha["hold_reason"], "one");
    assert_eq!(alpha["held_since"], since, "the since-time survived the restart");
    assert_eq!(rig.row("b/beta").unwrap()["hold"], true);
    assert!(rig.row("b/gamma").unwrap().get("hold").is_none());
    assert_held(rig.say("b/alpha", false), Some("one"));
    assert_held(rig.say("b/beta", true), None);
    assert!(is_delivered(&rig.say("b/gamma", false)), "an unheld session is unaffected by the restart");
}

// --- isolation -------------------------------------------------------------------

#[test]
fn holding_one_session_never_affects_another() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("only alpha")).unwrap();
    for _ in 0..5 {
        assert!(is_delivered(&rig.say("b/beta", false)));
        assert_held(rig.say(SESSION, false), Some("only alpha"));
    }
    rig.release("b/beta").unwrap(); // releasing an unheld session changes nothing
    assert_held(rig.say(SESSION, false), Some("only alpha"));
    assert!(rig.row("b/beta").unwrap().get("hold").is_none());
}

// --- persistence failure ---------------------------------------------------------

#[test]
fn a_corrupt_hold_file_does_not_stop_the_hub_and_is_kept() {
    let hub_state = StateDir::new();
    std::fs::create_dir_all(hub_state.hub()).unwrap();
    std::fs::write(hub_state.hub().join("holds.json"), b"{ this is not json").unwrap();
    let addr = free_addr();
    let mut hub = start_hub_at(&hub_state, &addr, &[]); // panics if the hub does not come up
    let (token_id, secret) = mint_token(&hub_state, "b");
    let body_state = StateDir::new();
    join(&body_state, &hub_state, &format!("ws://{addr}"), &token_id, &secret);
    let config = write_sessions_toml(&body_state, &[("alpha", &["--chunks", "1"])]);
    let body = Body::start_with_env(&body_state, &config, &[("HOLLER_HEARTBEAT_INTERVAL_MS", "300")]);
    let ready = wait_for(READY, || {
        let r = control::roster_at(hub_state.path(), false, None).ok()?;
        r["rows"].as_array()?.iter().any(|r| r["name"] == SESSION).then_some(())
    });
    assert!(ready.is_some());
    // The hub works, the corrupt file was moved aside intact, and a new hold persists.
    assert!(is_delivered(&say_at_retrying(hub_state.path(), SESSION, "hi", false)));
    let aside: Vec<_> = std::fs::read_dir(hub_state.hub())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("holds.json.corrupt-"))
        .collect();
    assert_eq!(aside.len(), 1, "the corrupt file must be kept, not deleted");
    assert_eq!(std::fs::read(aside[0].path()).unwrap(), b"{ this is not json");
    assert_eq!(control::hold_at(hub_state.path(), SESSION, None).unwrap()["persisted"], true);
    body.stop(&body_state, Duration::from_secs(5));
    kill_tree(&mut hub);
}

#[cfg(unix)]
#[test]
fn an_unwritable_state_directory_keeps_the_hold_in_force() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let dir = rig.hub_state.hub();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let can_write_anyway = std::fs::File::create(dir.join("probe")).is_ok(); // running as root
    let out = rig.hold(SESSION, Some("in memory only")).unwrap();
    let repeat = rig.hold(SESSION, Some("in memory only")).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    if !can_write_anyway {
        assert_eq!(out["persisted"], false, "the caller is told the hold will not survive a restart");
        assert_eq!(repeat["persisted"], false, "and a repeat does not claim otherwise");
    }
    // Once the directory is writable, the next hold or release call saves it.
    assert_eq!(rig.hold(SESSION, None).unwrap()["persisted"], true);
    assert!(rig.hold_file()["holds"].get(SESSION).is_some());
    assert_held(rig.say(SESSION, false), Some("in memory only"));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

// --- concurrency: the race matrix -------------------------------------------------

const ROUNDS: usize = 25;

/// Run `say` (plain) and `hold` at the same instant, many times. Every say is
/// delivered or refused, never lost; a say started after `hold` returned is
/// always refused.
#[test]
fn say_racing_hold_is_delivered_or_refused_never_lost() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let (mut delivered, mut refused) = (0, 0);
    for round in 0..ROUNDS {
        let barrier = Arc::new(Barrier::new(2));
        let racer = std::thread::spawn({
            let (root, b) = (rig.root().to_path_buf(), barrier.clone());
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "race", false)
            }
        });
        barrier.wait();
        rig.hold(SESSION, Some("race")).unwrap();
        let res = racer.join().unwrap();
        if is_delivered(&res) {
            delivered += 1;
        } else if is_held(&res) {
            refused += 1;
        } else {
            panic!("round {round}: a say that raced a hold was neither delivered nor refused: {res:?}");
        }
        // Once hold() has returned, nothing new gets in.
        assert!(is_held(&rig.say(SESSION, false)), "round {round}: a say after hold() returned was not refused");
        assert!(is_held(&rig.say(SESSION, true)), "round {round}: a queued say after hold() returned was not refused");
        rig.release(SESSION).unwrap();
        rig.wait_row(SESSION, |r| r["state"] == "idle" && r.get("hold").is_none());
    }
    assert_eq!(delivered + refused, ROUNDS);
}

#[test]
fn say_racing_release_is_delivered_or_refused_never_lost() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for round in 0..ROUNDS {
        rig.hold(SESSION, Some("race")).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let racer = std::thread::spawn({
            let (root, b) = (rig.root().to_path_buf(), barrier.clone());
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "race", false)
            }
        });
        barrier.wait();
        rig.release(SESSION).unwrap();
        let res = racer.join().unwrap();
        assert!(is_delivered(&res) || is_held(&res), "round {round}: neither delivered nor refused: {res:?}");
        // Once release() has returned, a say goes through.
        rig.wait_row(SESSION, |r| r["state"] == "idle");
        assert!(is_delivered(&rig.say(SESSION, false)), "round {round}: a say after release() returned was refused");
    }
}

/// A `--queue` prompt is in flight behind a running turn while the hold is
/// set: it is either accepted (and runs) or refused, and the running turn is
/// never affected.
#[test]
fn hold_while_a_queued_prompt_is_in_flight() {
    let rig = Rig::start(&[("alpha", &["--slow", "--chunks", "6"])]);
    for round in 0..8 {
        let root = rig.root().to_path_buf();
        let prev = rig.turn_id(SESSION);
        let running = std::thread::spawn({
            let root = root.clone();
            move || say_at_retrying(&root, SESSION, "running", false)
        });
        let dispatched = wait_for(READY, || {
            if running.is_finished() {
                return Some(false);
            }
            rig.row(SESSION).filter(|r| turn_accepted(r, prev.as_ref())).map(|_| true)
        });
        if dispatched != Some(true) {
            let res = if running.is_finished() { format!("{:?}", running.join().unwrap()) } else { "still running".into() };
            panic!("round {round}: the running say never dispatched a new turn; its result: {res}; roster: {:?}", rig.rows());
        }
        let barrier = Arc::new(Barrier::new(2));
        let queued = std::thread::spawn({
            let b = barrier.clone();
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "queued", true)
            }
        });
        barrier.wait();
        rig.hold(SESSION, None).unwrap();
        let q = queued.join().unwrap();
        assert!(is_delivered(&q) || is_held(&q), "round {round}: the queued say was lost: {q:?}");
        let r = running.join().unwrap();
        assert!(is_delivered(&r), "round {round}: the running turn was affected by the hold: {r:?}");
        rig.release(SESSION).unwrap();
        rig.wait_row(SESSION, |r| r["state"] == "idle");
    }
}

/// Two clients hold and release the same session at once: every call
/// succeeds, and the final hub state (roster, refusals and the file on disk)
/// agrees with the last operation.
#[test]
fn two_clients_holding_and_releasing_the_same_session_stay_consistent() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for round in 0..10 {
        let barrier = Arc::new(Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|t| {
                let (root, b) = (rig.root().to_path_buf(), barrier.clone());
                std::thread::spawn(move || {
                    b.wait();
                    for i in 0..20 {
                        let res = if (i + t) % 2 == 0 {
                            control::hold_at(&root, SESSION, Some("x"))
                        } else {
                            control::release_at(&root, SESSION)
                        };
                        assert!(res.is_ok(), "hold/release must always succeed: {res:?}");
                    }
                })
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        // Settle on a known final state, then check every view agrees.
        if round % 2 == 0 {
            rig.hold(SESSION, Some("final")).unwrap();
            let file_has = rig.hold_file()["holds"].get(SESSION).is_some();
            assert!(file_has, "round {round}: the file disagrees with the hub");
            assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
            assert!(is_held(&rig.say(SESSION, false)));
        } else {
            rig.release(SESSION).unwrap();
            assert!(rig.hold_file()["holds"].get(SESSION).is_none(), "round {round}: the file disagrees with the hub");
            assert!(rig.row(SESSION).unwrap().get("hold").is_none());
            assert!(is_delivered(&rig.say(SESSION, false)));
        }
    }
}
