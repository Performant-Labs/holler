//! `HarnessPort::agent_view` (#647 part 2, ADR-0021 section 12): what the server says about the
//! agents of a session's recent user turns, and which of its agents can dispatch a subagent.
//! It reports observations, never a finding: reconcile owns the rule.
//!
//! One call, within [`Timeouts::call`](crate::Timeouts::call), of three `GET`s of existing
//! routes, and nothing that changes the server: the existence check `abort` makes
//! (`GET /session/:id`; an unknown session is `session-not-found`), the session's messages
//! (`GET /session/:id/message`, through the crate's one reader, which quotes no body: the list
//! is conversation text) and the server's agents (`GET /agent`, through the reader
//! `verify_serving` uses). It reads no file (I6).
//!
//! - **The turns** are the messages whose `info.role` is `user`, the newest `max_turns`, oldest
//!   first. A turn's agent is its `info.agent` (OpenCode 1.19.0; the opt-in real test proves
//!   it), `None` when it has none or a name that is not an `AgentKey`. An assistant message
//!   carries an agent too, and is not a turn.
//! - **Whether an agent can dispatch** ([`can_dispatch`]) is read from its permission rules.
//! - **The dispatchers** are the agents the server lists that OpenCode takes as a default agent
//!   (`server::as_default`, the rule `verify_serving` applies: not a subagent, not hidden), that
//!   are not OpenCode's own (`native` is `false`), and that can dispatch (`Some(true)`). A name
//!   that is not an `AgentKey` is left out; `AgentView::with_dispatchers` sorts them and drops
//!   duplicates.
//! - **A reply of the wrong shape is `unavailable`**, never a skipped rule: a message list or an
//!   agent list that is not a JSON array, a message with no `info` object and string
//!   `info.role`, an agent with no string `name` and `mode`. An agent list's rules of a shape
//!   the adapter does not know are not an error: that agent's `can_dispatch` is `None`, which
//!   stops reconcile's rule.
//!
//! **The bound of the message read** (plan review W-7). OpenCode 1.19.0's route takes a `limit`
//! (`?limit=N` answers the newest N messages, oldest first; measured on a scratch server,
//! 2026-10-12), but it counts every message, and one user turn can carry many assistant
//! messages, so no limit promises the last `max_turns` user turns. The adapter asks for none and
//! reads the whole list, which grows with a long-lived session's conversation. A reply past the
//! transport's cap (64 MiB) or one that is not read within the call's bound is `unavailable` or
//! `timeout`: reconcile then reports `observe-failed` naming `harness.agent_view` and concludes
//! nothing, on every pass while it lasts.

use holler_pane::{AgentKey, AgentView, PaneError, TurnAgent};
use serde_json::Value;

use crate::prompt::messages;
use crate::server::{agent_list, as_default};
use crate::OP_AGENT_VIEW;
use crate::{known, session_path, string_at, OpenCodeHarness, GET_AGENT, GET_MESSAGES};

/// The permission an agent's rules name for dispatching a subagent (OpenCode's `task` tool),
/// and the pattern and the permission that match everything.
const TASK: &str = "task";
const EVERY: &str = "*";

/// `HarnessPort::agent_view` of the server on `port` (see the module docs).
pub(crate) fn agent_view(
    harness: &OpenCodeHarness,
    port: u16,
    session: &str,
    max_turns: usize,
) -> Result<AgentView, PaneError> {
    let call = harness.call(port, OP_AGENT_VIEW);
    known(&call, &session_path(session), session)?;
    let (reply, messages) = messages(&call, session)?;
    let users = user_turns(&messages).ok_or_else(|| {
        call.unexpected_status(GET_MESSAGES, &reply, "messages that each have an info.role")
    })?;
    let (reply, agents) = agent_list(&call)?;
    let listed = listed(&agents)
        .ok_or_else(|| call.unexpected(GET_AGENT, &reply, "agents with a string name and mode"))?;
    let newest = users.get(users.len().saturating_sub(max_turns)..);
    let turns = newest.unwrap_or_default().iter().map(|info| {
        let agent = string_at(Some(info), "agent").and_then(|name| AgentKey::parse(name).ok());
        let can = agent.as_ref().and_then(|key| dispatches(&listed, key));
        TurnAgent::new(agent, can)
    });
    let dispatchers = listed
        .iter()
        .filter(|agent| agent.default && agent.custom && agent.can_dispatch == Some(true))
        .filter_map(|agent| AgentKey::parse(agent.name).ok());
    Ok(AgentView::default()
        .with_turns(turns.collect::<Vec<_>>())
        .with_dispatchers(dispatchers.collect::<Vec<_>>()))
}

/// The `info` of each user message of `messages`, oldest first; `None` when a message has no
/// `info` object with a string `role`, which is not the JSON the step needs.
fn user_turns(messages: &[Value]) -> Option<Vec<&Value>> {
    let mut users = Vec::new();
    for message in messages {
        let info = message.get("info").filter(|info| info.is_object())?;
        if string_at(Some(info), "role")? == "user" {
            users.push(info);
        }
    }
    Some(users)
}

/// What the server says of one agent it lists.
struct Listed<'a> {
    name: &'a str,
    /// OpenCode takes it as a default agent (`server::as_default`).
    default: bool,
    /// `native` is `false`: the project's own, not one OpenCode ships.
    custom: bool,
    can_dispatch: Option<bool>,
}

/// Every agent of the `GET /agent` list `agents`, in order; `None` when one has no string `name`
/// or no string `mode`, which is not the JSON the step needs.
fn listed(agents: &[Value]) -> Option<Vec<Listed<'_>>> {
    agents
        .iter()
        .map(|agent| {
            Some(Listed {
                name: string_at(Some(agent), "name")?,
                default: as_default(agent)?.is_ok(),
                custom: agent.get("native") == Some(&Value::Bool(false)),
                can_dispatch: can_dispatch(agent.get("permission")),
            })
        })
        .collect()
}

/// Whether the agent `key` can dispatch, as the first agent of that name in `listed` says;
/// `None` for an agent the server does not list.
fn dispatches(listed: &[Listed<'_>], key: &AgentKey) -> Option<bool> {
    listed
        .iter()
        .find(|agent| agent.name == key.as_str())
        .and_then(|agent| agent.can_dispatch)
}

/// Whether an agent whose permission rules are `rules` can dispatch a subagent (ADR-0021 section
/// 12). OpenCode 1.19.0 lists the rules as `{permission, pattern, action}`, and the last rule
/// that matches a request decides it; a `task` request's pattern is the subagent's name.
///
/// `Some(false)` only when the rules deny `task` for every subagent: the last rule whose
/// permission is `task` or `*` and whose pattern is `*` is `deny`, and no later rule whose
/// permission is `task` or `*` lets another pattern through (`allow`, or `ask`, which lets a
/// person allow it). `Some(true)` otherwise, a list with no such rule included. `None` for rules
/// of a shape the adapter does not know: not a list, a rule without a string `permission`,
/// `pattern` or `action`, or an action other than `allow`, `ask` and `deny`.
fn can_dispatch(rules: Option<&Value>) -> Option<bool> {
    let rules = rules?
        .as_array()?
        .iter()
        .map(Rule::read)
        .collect::<Option<Vec<_>>>()?;
    let mut reopened = false;
    for rule in rules.iter().rev() {
        if rule.permission != TASK && rule.permission != EVERY {
            continue;
        }
        if rule.pattern == EVERY {
            return Some(!rule.deny || reopened);
        }
        reopened |= !rule.deny;
    }
    Some(true)
}

/// One permission rule of an agent, read strictly.
struct Rule<'a> {
    permission: &'a str,
    pattern: &'a str,
    /// The action is `deny`; `allow` and `ask` are not.
    deny: bool,
}

impl<'a> Rule<'a> {
    /// `rule` as a rule; `None` when it is not an object of three strings with a known action.
    fn read(rule: &'a Value) -> Option<Self> {
        let deny = match string_at(Some(rule), "action")? {
            "deny" => true,
            "allow" | "ask" => false,
            _ => return None,
        };
        Some(Self {
            permission: string_at(Some(rule), "permission")?,
            pattern: string_at(Some(rule), "pattern")?,
            deny,
        })
    }
}
