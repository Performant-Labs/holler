#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! `OpenCodeHarness::agent_view` (#647 part 2, ADR-0021 section 12, brief D7 and D15) against
//! the hermetic stub: what the server says about the agents of a session's recent user turns,
//! from `GET /session/:id/message`, and which listed agents can dispatch, from `GET /agent`.
//!
//! The two fixtures under `tests/fixtures/agents/` are cut from a running OpenCode 1.19.0
//! (`opencode serve --pure` in a scratch directory laid out as `real_opencode/rig.rs` lays one
//! out, with the project configuration of the brief's "Facts measured outside the repository":
//! `build` denies `task`, a custom primary `orchestrator` allows it). `agents.json` is that
//! server's `GET /agent` with each agent's `prompt` and every `external_directory` rule left
//! out and the scratch paths replaced; `messages.json` is a session's `GET /session/:id/message`
//! after four user messages sent with `noReply` (as `build`, `build`, `orchestrator`, `build`)
//! and one more as `build` that the dead-end model answered with an empty assistant message.
//! The opt-in real test (`real_opencode_test.rs`) proves the same reads on a live server.

#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/stub.rs"]
#[allow(dead_code)] // #642: shared with hermetic_test.rs
mod stub;

use fixtures::*;
use holler_pane::{AgentView, HarnessEndpoint, HarnessPort, PaneError, TurnAgent};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use serde_json::{json, Value};
use stub::{on_refused_port, Stub};

const AGENTS: &str = include_str!("fixtures/agents/agents.json");
const MESSAGES: &str = include_str!("fixtures/agents/messages.json");

/// A text placed in a reply body: an error that quotes the body carries it.
const REPLY_MARKER: &str = "zq647-reply-marker";

fn at(port: u16) -> HarnessEndpoint {
    HarnessEndpoint::Port(port)
}

fn turn(key: Option<&str>, can_dispatch: Option<bool>) -> TurnAgent {
    TurnAgent::new(key.map(agent), can_dispatch)
}

/// A stub that knows the session `ID`, answers its message list with `messages` and the agent
/// list with `agents`.
fn server(messages: &str, agents: &str) -> Stub {
    let stub = Stub::start();
    known_session(&stub);
    stub.json("GET", &format!("/session/{ID}/message"), 200, messages);
    stub.json("GET", "/agent", 200, agents);
    stub
}

fn view(stub: &Stub, max_turns: usize) -> Result<AgentView, PaneError> {
    harness(quick()).agent_view(&at(stub.port), ID, max_turns)
}

/// A user message as `GET /session/:id/message` lists one, run as `agent` (`None`: the
/// message records no agent).
fn user_message(n: usize, agent: Option<&str>) -> Value {
    let mut info = json!({
        "id": format!("msg_{n:04}"),
        "sessionID": ID,
        "role": "user",
        "time": { "created": 1_791_820_000_000_u64 + n as u64 },
    });
    if let Some(agent) = agent {
        info["agent"] = json!(agent);
    }
    json!({ "info": info, "parts": [] })
}

/// A message list of one user message per entry of `agents`, oldest first.
fn messages_as(agents: &[Option<&str>]) -> String {
    let list: Vec<Value> = agents
        .iter()
        .enumerate()
        .map(|(n, agent)| user_message(n, *agent))
        .collect();
    Value::Array(list).to_string()
}

/// An agent of `GET /agent`'s shape: `name`, `mode`, `native`, and `permission` as given.
fn listed(name: &str, mode: &str, native: bool, permission: Value) -> Value {
    json!({
        "name": name,
        "mode": mode,
        "native": native,
        "hidden": null,
        "options": {},
        "permission": permission,
    })
}

/// One permission rule of OpenCode 1.19.0's list.
fn rule(permission: &str, pattern: &str, action: &str) -> Value {
    json!({ "permission": permission, "pattern": pattern, "action": action })
}

// ---- the fixtures of a real 1.19.0 server ----

/// The newest three user turns, oldest first, each with the agent the server recorded and
/// whether that agent can dispatch; the assistant message is not a turn. Of the eight agents
/// the server lists, only the custom primary `orchestrator` is a dispatcher: `build` denies
/// `task`, `plan` is OpenCode's own, `explore` and `general` are subagents, and `compaction`,
/// `summary` and `title` are hidden.
#[test]
fn the_real_servers_answers_give_the_turns_and_the_one_dispatcher() {
    let stub = server(MESSAGES, AGENTS);
    let got = view(&stub, 3).unwrap();
    assert_eq!(
        got.turns,
        vec![
            turn(Some("orchestrator"), Some(true)),
            turn(Some("build"), Some(false)),
            turn(Some("build"), Some(false)),
        ]
    );
    assert_eq!(got.dispatchers, vec![agent("orchestrator")]);

    let all = view(&stub, 10).unwrap();
    assert_eq!(
        all.turns.len(),
        5,
        "the five user messages, not the assistant's: {:?}",
        all.turns
    );
    assert_eq!(
        all.turns[0],
        turn(Some("build"), Some(false)),
        "oldest first"
    );
}

/// Two reads, both `GET`, of the two existing routes, and never a write.
#[test]
fn agent_view_only_reads() {
    let stub = server(MESSAGES, AGENTS);
    view(&stub, 3).unwrap();
    let lines = stub.lines();
    assert!(
        lines.iter().all(|line| line.starts_with("GET ")),
        "reads only: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with(&format!("GET /session/{ID}/message"))),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("GET /agent")),
        "{lines:?}"
    );
}

#[test]
fn max_turns_bounds_the_turns_and_zero_asks_for_none() {
    let stub = server(&messages_as(&[Some("build"), Some("plan")]), AGENTS);
    assert_eq!(
        view(&stub, 1).unwrap().turns,
        vec![turn(Some("plan"), Some(true))],
        "the newest one; plan denies task to `general` only, so it can dispatch"
    );
    assert_eq!(view(&stub, 0).unwrap().turns, vec![]);
}

/// A session with no message has no turn; the dispatchers are still the server's.
#[test]
fn a_session_without_messages_has_no_turn() {
    let stub = server("[]", AGENTS);
    let got = view(&stub, 3).unwrap();
    assert_eq!(got.turns, vec![]);
    assert_eq!(got.dispatchers, vec![agent("orchestrator")]);
}

/// A user message that records no agent is a turn the server cannot say anything about, and so
/// is one whose agent is not an `AgentKey` or is not in the agent list: `agent: None` for the
/// first two, and `can_dispatch: None` for all three (ADR-0021 section 12).
#[test]
fn a_turn_the_server_says_nothing_about_is_none() {
    let stub = server(
        &messages_as(&[None, Some("not a key"), Some("ghost")]),
        AGENTS,
    );
    assert_eq!(
        view(&stub, 3).unwrap().turns,
        vec![
            turn(None, None),
            turn(None, None),
            turn(Some("ghost"), None),
        ]
    );
}

// ---- the rule: can an agent dispatch ----

/// `can_dispatch` of an agent whose permission rules are `rules`: `Some(false)` only when the
/// last rule whose permission is `task` or `*` and whose pattern is `*` is `deny` and no later
/// rule whose permission is `task` or `*` allows another pattern or asks for it (ADR-0021
/// section 12: `ask` lets a person allow it); `None` for rules of a shape the adapter does not
/// know.
#[test]
fn the_permission_rules_decide_whether_an_agent_can_dispatch() {
    let table: Vec<(&str, Value, Option<bool>)> = vec![
        (
            "build's rules: allow everything, then deny task",
            json!([rule("*", "*", "allow"), rule("task", "*", "deny")]),
            Some(false),
        ),
        ("no task rule", json!([rule("*", "*", "allow")]), Some(true)),
        ("no rule at all", json!([]), Some(true)),
        (
            "deny everything",
            json!([rule("*", "*", "deny")]),
            Some(false),
        ),
        (
            "deny then allow of one pattern",
            json!([rule("task", "*", "deny"), rule("task", "*", "allow")]),
            Some(true),
        ),
        (
            "allow then deny of one pattern",
            json!([rule("task", "*", "allow"), rule("task", "*", "deny")]),
            Some(false),
        ),
        (
            "deny all, then allow one subagent",
            json!([rule("task", "*", "deny"), rule("task", "general", "allow")]),
            Some(true),
        ),
        (
            "deny everything, then allow one subagent",
            json!([rule("*", "*", "deny"), rule("task", "general", "allow")]),
            Some(true),
        ),
        (
            "plan's rules: deny one subagent only",
            json!([rule("*", "*", "allow"), rule("task", "general", "deny")]),
            Some(true),
        ),
        (
            "deny task, then a rule of another permission",
            json!([rule("task", "*", "deny"), rule("read", "*", "allow")]),
            Some(false),
        ),
        (
            "deny task, then deny one subagent again",
            json!([rule("task", "*", "deny"), rule("task", "general", "deny")]),
            Some(false),
        ),
        (
            "deny task, then a rule of every permission allows one subagent",
            json!([rule("task", "*", "deny"), rule("*", "general", "allow")]),
            Some(true),
        ),
        (
            "deny task, then ask for one subagent",
            json!([rule("task", "*", "deny"), rule("task", "general", "ask")]),
            Some(true),
        ),
        (
            "ask for every subagent",
            json!([rule("task", "*", "ask")]),
            Some(true),
        ),
        (
            "an action the adapter does not know",
            json!([rule("task", "*", "maybe")]),
            None,
        ),
        (
            "an object where the list is expected (the older shape)",
            json!({ "task": "deny" }),
            None,
        ),
        (
            "a rule with no action",
            json!([rule("*", "*", "allow"), { "permission": "task", "pattern": "*" }]),
            None,
        ),
        ("no rule list", Value::Null, None),
    ];
    for (what, permission, expected) in table {
        let agents = json!([listed("demo", "primary", false, permission)]).to_string();
        let stub = server(&messages_as(&[Some("demo")]), &agents);
        let got = view(&stub, 3).unwrap_or_else(|e| panic!("{what}: {e:?}"));
        assert_eq!(got.turns, vec![turn(Some("demo"), expected)], "{what}");
    }
}

// ---- the rule: which agents are dispatchers ----

/// A dispatcher is an agent a pane could run as its default (not a subagent, not hidden: the
/// rule `verify_serving` applies), not OpenCode's own (`native` false), and able to dispatch;
/// a name that is not an `AgentKey` is left out. Sorted, without duplicates.
#[test]
fn only_custom_default_capable_agents_that_dispatch_are_dispatchers() {
    let allow = || json!([rule("*", "*", "allow"), rule("task", "*", "allow")]);
    let mut hidden = listed("hidden-one", "primary", false, allow());
    hidden["hidden"] = json!(true);
    let agents = json!([
        listed("zeta", "primary", false, allow()),
        listed("alpha", "all", false, allow()),
        listed("plan", "primary", true, allow()),
        listed("helper", "subagent", false, allow()),
        hidden,
        listed(
            "denier",
            "primary",
            false,
            json!([rule("task", "*", "deny")])
        ),
        listed("odd-rules", "primary", false, json!({ "task": "allow" })),
        listed("Not A Key", "primary", false, allow()),
        listed("zeta", "primary", false, allow()),
    ])
    .to_string();
    let stub = server("[]", &agents);
    assert_eq!(
        view(&stub, 3).unwrap().dispatchers,
        vec![agent("alpha"), agent("zeta")]
    );
}

// ---- the errors ----

/// The check `abort` makes: a session the server does not have is `session-not-found`.
#[test]
fn an_unknown_session_is_session_not_found() {
    let stub = Stub::start();
    stub.json("GET", &format!("/session/{ID}"), 404, stub::NOT_FOUND);
    stub.json(
        "GET",
        &format!("/session/{ID}/message"),
        404,
        stub::NOT_FOUND,
    );
    stub.json("GET", "/agent", 200, AGENTS);
    assert_eq!(
        view(&stub, 3),
        Err(PaneError::SessionNotFound {
            what: ID.to_owned()
        })
    );
}

/// A message list that is not a JSON array is `unavailable`, naming the route and quoting
/// nothing of the body: the list is conversation text.
#[test]
fn a_message_list_that_is_not_a_list_is_unavailable_and_quotes_nothing() {
    let body = format!(r#"{{"messages":"{REPLY_MARKER}"}}"#);
    let stub = server(&body, AGENTS);
    let what = unavailable(view(&stub, 3), "a message list that is an object");
    assert!(
        what.contains("/session/:id/message"),
        "names the route: {what:?}"
    );
    assert!(
        !what.contains(REPLY_MARKER),
        "quotes nothing of the body: {what:?}"
    );
    assert!(!what.contains('\n'), "one line: {what:?}");
}

#[test]
fn an_agent_list_that_is_not_a_list_is_unavailable() {
    let stub = server(MESSAGES, &format!(r#"{{"agents":"{REPLY_MARKER}"}}"#));
    let what = unavailable(view(&stub, 3), "an agent list that is an object");
    assert!(what.contains("/agent"), "names the route: {what:?}");
}

/// An agent with no string `mode` is not the JSON the step needs (`usable`'s rule, shared).
#[test]
fn a_listed_agent_without_a_mode_is_unavailable() {
    let agents = json!([{ "name": "orchestrator", "native": false, "permission": [] }]).to_string();
    let stub = server("[]", &agents);
    unavailable(view(&stub, 3), "an agent with no mode");
}

/// A frozen server is `timeout` with the method's op; a port nothing listens on is
/// `unavailable`. The op equals the test kit's, so the fake and the adapter name it alike.
#[test]
fn a_frozen_or_absent_server_fails_as_every_call_does() {
    let stub = server(MESSAGES, AGENTS);
    stub.freeze();
    let op = timeout_op(view(&stub, 3), "a frozen server");
    assert_eq!(op, HarnessOp::AgentView.as_str());
    assert_eq!(op, "harness.agent_view");

    let (port, answer) = on_refused_port(
        |port| harness(quick()).agent_view(&at(port), ID, 3),
        |answer| matches!(answer, Err(PaneError::Unavailable { .. })),
    );
    assert_eq!(
        answer,
        Err(PaneError::Unavailable {
            what: format!("the harness server on port {port}")
        })
    );
}

/// A process endpoint is `process-unsupported`, the one conversion a server harness makes.
#[test]
fn a_process_endpoint_is_process_unsupported() {
    let process = HarnessEndpoint::Process {
        pane: pane_name(),
        pid: 4242,
    };
    match harness(quick()).agent_view(&process, ID, 3) {
        Err(PaneError::Refused { code, .. }) => assert_eq!(code.as_str(), "process-unsupported"),
        other => panic!("expected process-unsupported, got {other:?}"),
    }
}
