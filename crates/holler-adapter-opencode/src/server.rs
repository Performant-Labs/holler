//! A pane's OpenCode server: the argv that starts it (`serve_command`), whether the running
//! one honours its spec (`verify_serving`), and whether a server is healthy (`health`).
//!
//! The adapter starts no server (#802). The host adapter's `run` starts the argv in the
//! pane's tmux session, in the pane spec's directory, and its `stop_owned` stops it; the
//! launch engine waits for its health. So nothing here spawns, waits on or signals a
//! process.
//!
//! The pane's OpenCode agent is the server's `default_agent`, set once, when it starts
//! (#642 part 3). `opencode serve` has no agent option, so the argv sets it through
//! `OPENCODE_CONFIG_CONTENT`, which OpenCode merges over every config file. OpenCode does not
//! refuse an unusable default agent at start (a server started with a subagent, a hidden agent
//! or an unknown key answers healthy), so the running server is asked before any session is
//! made on it.
//!
//! [`agent_list`] is the crate's one reader of `GET /agent`, and [`as_default`] the one
//! statement of OpenCode's rule for a default agent: `verify_serving` asks both of the pane's
//! agent, and `agent_view` (#647 part 2, `agents.rs`) of every agent the server lists.

use std::time::Duration;

use holler_pane::error::RefusalCode;
use holler_pane::{AgentKey, Argv, PaneError, ServeSpec};
use serde_json::{json, Value};

use crate::http::{self, Reply};
use crate::{json_of, string_at, Call, OpenCodeConfig, OpenCodeHarness};
use crate::{GET_AGENT, GET_CONFIG, OP_VERIFY_SERVING};

/// `agent-unusable`: `verify_serving` of a spec whose agent the running server does not run
/// as its default: the variable did not reach the server, or OpenCode has no agent of that
/// name, or the agent is a subagent or hidden, which OpenCode refuses as a default agent.
/// The adapter's own open code (#642 part 3), distinct from `agent-unsupported`, which stays
/// the answer of a harness that cannot apply an agent at all. Refusal, exit 3.
pub const AGENT_UNUSABLE: RefusalCode = RefusalCode::from_static("agent-unusable");

/// The route a health check asks.
const HEALTH: &str = "/global/health";

/// The variable whose JSON OpenCode merges over every config file: the server's
/// `default_agent` travels in it.
const CONFIG_CONTENT: &str = "OPENCODE_CONFIG_CONTENT";

/// `HarnessPort::serve_command`: `<env prefix> <opencode_bin> serve --port P --hostname
/// 127.0.0.1 <serve_args>`. The prefix is the TUI's ([`crate::ProcessEnv::prefix`]) for a
/// spec without an agent, and the TUI's plus the one pair [`default_agent`] for a spec with
/// one ([`crate::ProcessEnv::prefix_with`]), after the prefix's own words, so the pair
/// replaces a same-named configured pair and an inherited value. The server listens on the
/// loopback address only, and the pane's name is not on the argv: the host runs it in the
/// pane's own tmux session and directory. Pure: it reads the configuration and the spec and
/// nothing else.
pub(crate) fn serve_command(
    config: &OpenCodeConfig,
    port: u16,
    spec: &ServeSpec,
) -> Result<Argv, PaneError> {
    let pairs: Vec<(String, String)> = spec.agent.iter().map(default_agent).collect();
    let mut argv = config.env.prefix_with(&pairs);
    argv.extend([
        config.bin()?.to_owned(),
        "serve".to_owned(),
        "--port".to_owned(),
        port.to_string(),
        "--hostname".to_owned(),
        "127.0.0.1".to_owned(),
    ]);
    argv.extend(config.serve_args.iter().cloned());
    Ok(Argv::new(argv))
}

/// The pair that makes `key` the server's default agent: `OPENCODE_CONFIG_CONTENT` holding the
/// compact JSON `{"default_agent":KEY}`, and nothing else: never an agent's definition, since
/// tmux refuses a command of about 20,000 bytes.
fn default_agent(key: &AgentKey) -> (String, String) {
    let content = json!({ "default_agent": key.as_str() });
    (CONFIG_CONTENT.to_owned(), content.to_string())
}

/// `HarnessPort::verify_serving`. A spec without an agent is `Ok` with nothing sent: its argv
/// applies none, so the server keeps its own default. For a spec with `key`, within one
/// call's bound, in this order: `GET /config` must answer an object whose `default_agent` is
/// `key`, which proves the variable reached the server (a server it did not reach runs as
/// `build`, which cannot dispatch subagents); then `GET /agent` must list `key` as an agent
/// OpenCode takes as a default ([`usable`]). A server that does not is [`AGENT_UNUSABLE`], in
/// one line naming the key and the reason and quoting no reply. A reply that is not the JSON
/// a step needs (the web app's page answers an unserved route with a `200`) is `unavailable`.
pub(crate) fn verify_serving(
    harness: &OpenCodeHarness,
    port: u16,
    spec: &ServeSpec,
) -> Result<(), PaneError> {
    let Some(key) = &spec.agent else {
        return Ok(());
    };
    let key = key.as_str();
    let call = harness.call(port, OP_VERIFY_SERVING);
    let reply = call.send(GET_CONFIG, GET_CONFIG.label, None)?;
    let config = json_of(&reply)
        .filter(Value::is_object)
        .ok_or_else(|| call.unexpected(GET_CONFIG, &reply, "an object"))?;
    if string_at(Some(&config), "default_agent") != Some(key) {
        return Err(unusable(format!("the server's default agent is not {key}")));
    }
    let (reply, agents) = agent_list(&call)?;
    usable(&call, &reply, &agents, key)
}

/// `GET /agent`: the server's agents, the JSON array of a `200`, with the reply, for a step that
/// finds an agent of the wrong shape to name the route. Any other answer is `unavailable`,
/// naming the route. The crate's one reader of the route.
pub(crate) fn agent_list(call: &Call) -> Result<(Reply, Vec<Value>), PaneError> {
    let reply = call.send(GET_AGENT, GET_AGENT.label, None)?;
    match json_of(&reply) {
        Some(Value::Array(agents)) => Ok((reply, agents)),
        _ => Err(call.unexpected(GET_AGENT, &reply, "a list of agents")),
    }
}

/// Whether the `GET /agent` list `agents` holds `key` as an agent OpenCode takes as a default
/// agent ([`as_default`]); a missing agent is refused too. An element named `key` with no string
/// `mode` is not the JSON the step needs: `unavailable`.
fn usable(call: &Call, reply: &Reply, agents: &[Value], key: &str) -> Result<(), PaneError> {
    let Some(agent) = agents
        .iter()
        .find(|agent| string_at(Some(agent), "name") == Some(key))
    else {
        return Err(unusable(format!("no agent named {key}")));
    };
    match as_default(agent) {
        None => Err(call.unexpected(GET_AGENT, reply, "agents with a string mode")),
        Some(Err(why)) => Err(unusable(format!("agent {key} is {why}"))),
        Some(Ok(())) => Ok(()),
    }
}

/// OpenCode's own rule for an agent it takes as a default agent, for one element `agent` of the
/// `GET /agent` list: it refuses one whose `mode` is `subagent` and a hidden one, and nothing
/// else, so `primary` and `all` (what an agent defined in a config without a `mode` gets) both
/// pass. `hidden` counts only when it is `true`, since OpenCode leaves it out otherwise.
/// `Some(Ok(()))` for an agent it takes, `Some(Err(why))` for one it refuses (`why` completes
/// "agent KEY is ..."), and `None` for an element with no string `mode`, which is not the JSON a
/// step needs.
pub(crate) fn as_default(agent: &Value) -> Option<Result<(), &'static str>> {
    match string_at(Some(agent), "mode")? {
        "subagent" => Some(Err("a subagent")),
        _ if agent.get("hidden") == Some(&Value::Bool(true)) => Some(Err("hidden")),
        _ => Some(Ok(())),
    }
}

/// [`AGENT_UNUSABLE`] with `message`, which names the key (a name, never a secret: I7) and the
/// reason, and quotes no reply.
fn unusable(message: String) -> PaneError {
    PaneError::Refused {
        code: AGENT_UNUSABLE,
        message,
    }
}

/// Whether the server on `port` answers `GET /global/health` with `{"healthy": true}`
/// within `within`. Refused, timed out, garbled or anything else is `false`.
pub(crate) fn healthy(port: u16, within: Duration) -> bool {
    matches!(
        http::request(port, "GET", HEALTH, None, within),
        Ok(reply) if is_healthy(&reply)
    )
}

/// A `200` whose JSON is an object with `"healthy": true`.
fn is_healthy(reply: &Reply) -> bool {
    json_of(reply)
        .as_ref()
        .and_then(|health| health.get("healthy"))
        .and_then(Value::as_bool)
        == Some(true)
}
