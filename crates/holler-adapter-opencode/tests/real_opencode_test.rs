#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The OpenCode adapter against a real `opencode serve` and a real tmux (#642b, AC 12-19a).
//! Opt-in: every test is `#[ignore]` and returns at once, saying why, unless
//! `HOLLER_TEST_OPENCODE=1`; opted in, a missing `opencode` or `tmux` is a failure. Run:
//!
//! ```text
//! HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test \
//!   -- --ignored --test-threads=1
//! ```
//!
//! Every test builds its own rig (`real_opencode/rig.rs`): a scratch `HOME`, the dead-end
//! provider (no model can be reached; a prompt is sent only by the agent and prompt tests of
//! #642 part 3, to that provider, and their turns are aborted), ports from 48100-48199 only
//! and a private tmux server. No test prints a raw `#{pane_title}`: assertions use the parsed
//! value.

#[path = "real_opencode/rig.rs"]
mod rig;

use std::time::{Duration, Instant};

use holler_adapter_opencode::tui::attach_port;
use holler_adapter_opencode::{OpenCodeHarness, AGENT_UNUSABLE};
use holler_pane::ports::Activity;
use holler_pane::{AgentKey, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec};
use holler_pane_testkit::conformance::harness::run_harness_conformance;
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use rig::{opted_in, raw, refused, sessions_for, signal_group, Recording, Rig, PANES, SESSIONS};
use serde_json::{json, Value};

/// A well-formed id that no server holds.
const UNKNOWN: &str = "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz";
/// What a bound may overrun by (AC 32).
const SLACK: Duration = Duration::from_millis(500);

fn json_of(reply: &holler_adapter_opencode::http::Reply) -> Value {
    serde_json::from_slice(&reply.body).unwrap_or(Value::Null)
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let answer = f();
    (answer, start.elapsed())
}

fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) {
    assert!(
        matches!(result, Err(PaneError::Unavailable { .. })),
        "{what}: expected unavailable, got {result:?}"
    );
}

// ---- AC 12: the conformance suite ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac12_the_adapter_passes_the_harness_conformance_suite() {
    if !opted_in("ac12") {
        return;
    }
    let result = run_harness_conformance(|| Rig::start().into_case(), Recording::launch);
    assert_eq!(result, Ok(()), "every case holds against real OpenCode");
}

// ---- AC 13: create, list, switch and report ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac13_create_list_attach_switch_report_and_abort() {
    if !opted_in("ac13") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let (a, b) = (rig.create(0), rig.create(0));
    let p0 = rig.ports[0];
    let listed = rig
        .harness
        .list_sessions(&HarnessEndpoint::Port(p0))
        .unwrap();
    assert!(listed.contains(&a) && listed.contains(&b), "{listed:?}");
    let pane0 = &rig.panes[0];
    assert_eq!(
        rig.harness
            .attach_tui(pane0, &HarnessEndpoint::Port(p0), &a),
        Ok(())
    );
    assert_eq!(
        rig.harness
            .shown_session(pane0, &HarnessEndpoint::Port(rig.ports[0])),
        Ok(Some(a.clone()))
    );
    assert_eq!(rig.harness.select_session(pane0, &b), Ok(()));
    assert_eq!(
        rig.harness
            .shown_session(pane0, &HarnessEndpoint::Port(rig.ports[0])),
        Ok(Some(b))
    );
    assert_eq!(rig.harness.abort(&HarnessEndpoint::Port(p0), &a), Ok(()));
}

// ---- AC 14: a deleted session under a TUI ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac14_a_deleted_session_under_a_tui_is_reported_not_hidden() {
    if !opted_in("ac14") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let p0 = rig.ports[0];
    let a = rig.create(0);
    let pane0 = rig.panes[0].clone();
    assert_eq!(
        rig.harness
            .attach_tui(&pane0, &HarnessEndpoint::Port(p0), &a),
        Ok(())
    );
    let before = rig
        .harness
        .list_sessions(&HarnessEndpoint::Port(p0))
        .unwrap();
    let deleted = raw(p0, "DELETE", &format!("/session/{a}"), None);
    assert_eq!(deleted.status, 200, "raw DELETE /session/:id");
    assert_eq!(
        rig.shown_within(&pane0, &None, Duration::from_secs(2)),
        Ok(None),
        "the TUI left the deleted session within 2 s"
    );
    assert_eq!(
        rig.pane_format(SESSIONS[0], "#{pane_dead}"),
        "0",
        "the TUI still runs"
    );
    let after = rig
        .harness
        .list_sessions(&HarnessEndpoint::Port(p0))
        .unwrap();
    assert!(!after.contains(&a), "{after:?}");
    assert!(
        after.iter().all(|id| before.contains(id)),
        "no session appeared: before {before:?}, after {after:?}"
    );
}

// ---- AC 15 and 16: a frozen or killed server only affects calls to it ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac15_a_frozen_server_only_affects_calls_to_that_server() {
    if !opted_in("ac15") {
        return;
    }
    let mut rig = Rig::start();
    let pid0 = rig.serve(0);
    rig.serve(1);
    let [p0, p1] = rig.ports;
    let a = rig.create(0);
    let pane0 = rig.panes[0].clone();
    assert_eq!(
        rig.harness
            .attach_tui(&pane0, &HarnessEndpoint::Port(p0), &a),
        Ok(())
    );
    assert!(signal_group(pid0, "STOP"), "SIGSTOP the server's group");
    let timeouts = rig.config.timeouts;
    let (health, took) = timed(|| rig.harness.health(&HarnessEndpoint::Port(p0)));
    assert_eq!(health, Ok(false), "health of a frozen server");
    assert!(took < timeouts.health + SLACK, "health took {took:?}");
    let (created, took) = timed(|| rig.harness.create_session(&HarnessEndpoint::Port(p0)));
    assert_eq!(
        created,
        Err(PaneError::Timeout {
            op: HarnessOp::CreateSession.as_str().to_owned()
        })
    );
    assert!(took < timeouts.call + SLACK, "create_session took {took:?}");
    assert_eq!(
        rig.harness.health(&HarnessEndpoint::Port(p1)),
        Ok(true),
        "the other server"
    );
    assert!(
        rig.harness
            .create_session(&HarnessEndpoint::Port(p1))
            .is_ok(),
        "the other server creates"
    );
    assert_eq!(
        rig.harness
            .shown_session(&pane0, &HarnessEndpoint::Port(rig.ports[0])),
        Ok(Some(a)),
        "the TUI keeps its screen"
    );
    signal_group(pid0, "CONT");
}

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac16_a_killed_server_only_affects_calls_to_that_server() {
    if !opted_in("ac16") {
        return;
    }
    let mut rig = Rig::start();
    let pid0 = rig.serve(0);
    rig.serve(1);
    let [p0, p1] = rig.ports;
    let a = rig.create(0);
    assert!(signal_group(pid0, "KILL"), "SIGKILL the server's group");
    let (health, took) = timed(|| {
        let until = Instant::now() + Duration::from_secs(1);
        loop {
            let health = rig.harness.health(&HarnessEndpoint::Port(p0));
            if health == Ok(false) || Instant::now() >= until {
                return health;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    assert_eq!(health, Ok(false), "health of a killed server");
    assert!(
        took < Duration::from_secs(1) + SLACK,
        "health took {took:?}"
    );
    let (created, took) = timed(|| rig.harness.create_session(&HarnessEndpoint::Port(p0)));
    unavailable(created, "create_session on a killed server");
    assert!(
        took < Duration::from_secs(1),
        "create_session took {took:?}"
    );
    let listed = rig
        .harness
        .list_sessions(&HarnessEndpoint::Port(p1))
        .unwrap();
    assert!(listed.contains(&a), "one shared data directory: {listed:?}");
}

// ---- AC 17: abort of a busy session ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac17_abort_of_a_busy_session_stops_it() {
    if !opted_in("ac17") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let p0 = rig.ports[0];
    let a = rig.create(0);
    let busy = |status: &Value| status[&a]["type"] == "busy";
    // A shell command makes the session busy with no model call (opencode-api.sh:76-78).
    let command = format!("sleep 37.{}", std::process::id() % 900 + 100);
    let shell = json!({ "agent": "build", "command": command });
    let path = format!("/session/{a}/shell");
    let _shell = std::thread::spawn(move || {
        let _ = holler_adapter_opencode::http::request(
            p0,
            "POST",
            &path,
            Some(&shell),
            Duration::from_secs(60),
        );
    });
    let until = Instant::now() + Duration::from_secs(10);
    while !busy(&json_of(&raw(p0, "GET", "/session/status", None))) {
        assert!(
            Instant::now() < until,
            "the shell command made the session busy"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let (aborted, took) = timed(|| rig.harness.abort(&HarnessEndpoint::Port(p0), &a));
    assert_eq!(aborted, Ok(()));
    assert!(took < Duration::from_secs(1), "abort took {took:?}");
    assert!(
        !busy(&json_of(&raw(p0, "GET", "/session/status", None))),
        "the session is no longer busy"
    );
}

// ---- AC 18: raw OpenCode pins ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac18_raw_opencode_pins_what_the_adapter_hides() {
    if !opted_in("ac18") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let p0 = rig.ports[0];
    let s = rig.create(0);
    // (2) No TUI is attached to this fresh server.
    for (pane, session) in PANES.iter().zip(SESSIONS) {
        assert_eq!(
            rig.harness
                .shown_session(&PaneId::new(*pane), &HarnessEndpoint::Port(rig.ports[0])),
            Ok(None),
            "{pane}"
        );
        let start = rig.pane_format(session, "#{pane_start_command}");
        assert_eq!(
            attach_port(&start),
            None,
            "{pane} still runs its placeholder"
        );
    }
    // (3) The pins. A failure here means OpenCode changed: re-run the spike (#635).
    let select = raw(
        p0,
        "POST",
        "/tui/select-session",
        Some(&json!({ "sessionID": s })),
    );
    assert_eq!(
        (select.status, json_of(&select)),
        (200, Value::Bool(true)),
        "pin: select-session with no TUI attached answers 200 true"
    );
    let doc = json_of(&raw(p0, "GET", "/doc", None));
    let ids: Vec<&str> = doc["paths"]
        .as_object()
        .into_iter()
        .flat_map(|paths| paths.values())
        .filter_map(Value::as_object)
        .flat_map(|ops| ops.values())
        .filter_map(|op| op["operationId"].as_str())
        .collect();
    for id in [
        "global.health",
        "session.create",
        "session.list",
        "session.get",
        "session.update",
        "session.delete",
        "session.status",
        "session.abort",
        "tui.selectSession",
        "tui.showToast",
    ] {
        assert!(ids.contains(&id), "pin: GET /doc lists the operation {id}");
    }
    let abort = raw(p0, "POST", &format!("/session/{UNKNOWN}/abort"), None);
    assert_eq!(
        (abort.status, json_of(&abort)),
        (200, Value::Bool(true)),
        "pin: an abort of an unknown id answers 200 true"
    );
}

// ---- AC 19 and 19a ----

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac19_serve_command_starts_nothing_and_applies_an_agent() {
    if !opted_in("ac19") {
        return;
    }
    let rig = Rig::start();
    let p0 = rig.ports[0];
    let name = PaneName::parse(SESSIONS[0]).unwrap();
    let argv = rig
        .harness
        .serve_command(&name, p0, &ServeSpec::default())
        .unwrap();
    assert!(!argv.as_slice().is_empty());
    std::thread::sleep(Duration::from_millis(500));
    assert!(refused(p0), "#802: serve_command started nothing on {p0}");
    let key = AgentKey::parse("feature-implementor").unwrap();
    let with = ServeSpec::default().with_agent(key);
    let applied = rig.harness.serve_command(&name, p0, &with).unwrap();
    assert!(
        applied
            .as_slice()
            .iter()
            .any(|e| e.contains("feature-implementor")),
        "#642 part 3: the argv carries the key: {applied:?}"
    );
    std::thread::sleep(Duration::from_millis(500));
    assert!(refused(p0), "serve_command started nothing on {p0}");
}

#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn ac19a_a_prefix_of_a_session_name_never_reaches_its_pane() {
    if !opted_in("ac19a") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let p0 = rig.ports[0];
    let (a, b) = (rig.create(0), rig.create(0));
    let pane0 = rig.panes[0].clone();
    assert_eq!(
        rig.harness
            .attach_tui(&pane0, &HarnessEndpoint::Port(p0), &a),
        Ok(())
    );
    let pid = rig.pane_format(SESSIONS[0], "#{pane_pid}");
    // `demo` is a prefix of `demo-c1r1`; no session `demo` exists.
    let mut config = rig.config.clone();
    config.tui_session = sessions_for(&[
        (PANES[0], SESSIONS[0]),
        (PANES[1], SESSIONS[1]),
        ("w9:p3", "demo"),
    ]);
    let prefixed = OpenCodeHarness::new(config);
    let p3 = PaneId::new("w9:p3");
    assert_eq!(
        prefixed.shown_session(&p3, &HarnessEndpoint::Port(rig.ports[0])),
        Ok(None)
    );
    unavailable(prefixed.select_session(&p3, &b), "select_session(w9:p3)");
    unavailable(
        prefixed.attach_tui(&p3, &HarnessEndpoint::Port(p0), &b),
        "attach_tui(w9:p3)",
    );
    assert_eq!(
        rig.harness
            .shown_session(&pane0, &HarnessEndpoint::Port(rig.ports[0])),
        Ok(Some(a)),
        "pane 0 still shows A"
    );
    assert_eq!(
        rig.pane_format(SESSIONS[0], "#{pane_pid}"),
        pid,
        "pane 0 was not respawned"
    );
}

// ---- #642 part 3, acceptance 11: the agent, the prompt and the activity ----

/// How long a raw read of the server's own state may take to show what a test waits for.
const OBSERVE: Duration = Duration::from_secs(15);

fn spec(key: &str) -> ServeSpec {
    ServeSpec::default().with_agent(AgentKey::parse(key).unwrap())
}

/// Poll `seen` on a raw read until it answers, within [`OBSERVE`].
fn observed<T>(what: &str, mut seen: impl FnMut() -> Option<T>) -> T {
    let until = Instant::now() + OBSERVE;
    loop {
        if let Some(answer) = seen() {
            return answer;
        }
        assert!(
            Instant::now() < until,
            "{what}: not seen within {OBSERVE:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The raw message list of `session`, read with no adapter in between.
fn messages(port: u16, session: &str) -> Vec<Value> {
    let reply = raw(port, "GET", &format!("/session/{session}/message"), None);
    json_of(&reply).as_array().cloned().unwrap_or_default()
}

/// The `info` of the first assistant message of `session`, once there is one.
fn first_assistant(port: u16, session: &str) -> Value {
    observed("an assistant message", || {
        messages(port, session)
            .into_iter()
            .map(|m| m["info"].clone())
            .find(|info| info["role"] == "assistant")
    })
}

/// The raw prompt holler-body sends (E16), on the bare route.
fn raw_prompt(port: u16, session: &str) {
    let body = json!({ "parts": [{ "type": "text", "text": "hello" }] });
    let reply = raw(
        port,
        "POST",
        &format!("/session/{session}/prompt_async"),
        Some(&body),
    );
    assert_eq!(reply.status, 204, "raw prompt_async");
}

/// End `session`'s turn with a raw abort: the dead-end provider is retried until then.
fn raw_abort(port: u16, session: &str) {
    let reply = raw(port, "POST", &format!("/session/{session}/abort"), None);
    assert_eq!(json_of(&reply), Value::Bool(true), "raw abort");
}

fn refusal(result: Result<(), PaneError>, what: &str) -> (String, String) {
    match result {
        Err(PaneError::Refused { code, message }) => (code.as_str().to_owned(), message),
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

/// Acceptance 11 (D1-D4): a server started with `plan` runs its turns as `plan`, and
/// `verify_serving` says so; a server started with no agent runs as its own default, and
/// `verify_serving` asked for `plan` refuses it: a real server the variable did not reach.
#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn agent_default_reaches_the_first_message() {
    if !opted_in("agent_default_reaches_the_first_message") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve_with(0, &spec("plan"));
    rig.serve(1);
    let [p0, p1] = rig.ports;
    assert_eq!(
        rig.harness
            .verify_serving(&HarnessEndpoint::Port(p0), &spec("plan")),
        Ok(())
    );
    let (code, message) = refusal(
        rig.harness
            .verify_serving(&HarnessEndpoint::Port(p1), &spec("plan")),
        "a server with no agent",
    );
    assert_eq!(code, AGENT_UNUSABLE.as_str());
    assert!(
        message.contains("the server's default agent is not plan"),
        "{message:?}"
    );
    // The server's own default: the first agent a default may be (E33).
    let agents = json_of(&raw(p1, "GET", "/agent", None));
    let default = agents
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["mode"] != "subagent" && a["hidden"] != true)
        .and_then(|a| a["name"].as_str())
        .unwrap()
        .to_owned();
    assert_ne!(
        default, "plan",
        "the second server must not run plan anyway"
    );
    for (i, want) in [(0, "plan"), (1, default.as_str())] {
        let port = rig.ports[i];
        let session = rig.create(i);
        raw_prompt(port, &session);
        let info = first_assistant(port, &session);
        assert_eq!(
            (info["agent"].as_str(), info["mode"].as_str()),
            (Some(want), Some(want)),
            "server {i}: {info}"
        );
        raw_abort(port, &session);
    }
}

/// Acceptance 11 (D4(b)): a server started with a subagent, a hidden agent or an unknown key
/// comes up healthy (E21), and `verify_serving` refuses each `agent-unusable`.
#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key() {
    if !opted_in("verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key") {
        return;
    }
    for (key, reason) in [
        ("explore", "agent explore is a subagent"),
        ("title", "agent title is hidden"),
        ("zq642-no-such-agent", "no agent named zq642-no-such-agent"),
    ] {
        let mut rig = Rig::start();
        rig.serve_with(0, &spec(key));
        let (code, message) = refusal(
            rig.harness
                .verify_serving(&HarnessEndpoint::Port(rig.ports[0]), &spec(key)),
            key,
        );
        assert_eq!(code, AGENT_UNUSABLE.as_str(), "{key}");
        assert!(message.contains(reason), "{key}: {message:?}");
    }
}

/// Acceptance 11 (D6, D7): `send_prompt` queues a turn, which reads `Busy` while OpenCode
/// retries the dead-end provider (`retry`, D7(b)); once a raw abort has ended it, as the raw
/// message list shows, `session_activity` reads `Idle` within `Timeouts.settle`.
#[test]
#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]
fn send_prompt_queues_a_turn_and_session_activity_follows_it() {
    if !opted_in("send_prompt_queues_a_turn_and_session_activity_follows_it") {
        return;
    }
    let mut rig = Rig::start();
    rig.serve(0);
    let p0 = rig.ports[0];
    let s = rig.create(0);
    assert_eq!(
        rig.harness
            .send_prompt(&HarnessEndpoint::Port(p0), &s, "hello"),
        Ok(())
    );
    observed("the turn in /session/status", || {
        let status = json_of(&raw(p0, "GET", "/session/status", None));
        status.get(&s).is_some().then_some(())
    });
    assert_eq!(
        rig.harness.session_activity(&HarnessEndpoint::Port(p0), &s),
        Ok(Activity::Busy)
    );
    raw_abort(p0, &s);
    observed("the turn's assistant message completed", || {
        messages(p0, &s)
            .iter()
            .any(|m| m["info"]["role"] == "assistant" && m["info"]["time"]["completed"].is_number())
            .then_some(())
    });
    let until = Instant::now() + rig.config.timeouts.settle;
    let mut activity = rig.harness.session_activity(&HarnessEndpoint::Port(p0), &s);
    while activity != Ok(Activity::Idle) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
        activity = rig.harness.session_activity(&HarnessEndpoint::Port(p0), &s);
    }
    assert_eq!(activity, Ok(Activity::Idle), "once the turn has ended");
}
