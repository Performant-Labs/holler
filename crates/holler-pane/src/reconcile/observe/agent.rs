//! The agent check of a server's chain (#647 part 2, ADR-0021 section 12; plan review W-8):
//! `agent-cannot-dispatch`, an OpenCode pane whose recent turns all ran as an agent that cannot
//! dispatch a subagent while the project ships an orchestrator, an agent that can.
//!
//! It runs after the parent's `check_session_of_record`, on the server the parent observed, and
//! asks [`HarnessPort::agent_view`] only once rules 1 to 3 hold, so a pane whose record names an
//! agent costs no call. The finding is raised when all five hold:
//!
//! 1. the record names no agent (`opencode_agent` is absent): the only opt-out a record has, so a
//!    pane whose record names one chose it, `build` included;
//! 2. the pane's harness is a server (OpenCode): the parent calls this on the server path only;
//! 3. the server is healthy and its trusted list holds the session of record (`Server::has`):
//!    not a down, wedged or unknown server, a list that failed, or a list without it;
//! 4. the view of the session of record's last [`RECENT_TURNS`] user turns has a turn, and every
//!    turn's agent cannot dispatch (`Some(false)`; a turn the server says nothing about stops it);
//! 5. the view lists a dispatcher: the project ships an orchestrator.
//!
//! "Recent hub-delivered turns" are the recent turns of the session of record, which I2 makes the
//! session the hub drives: OpenCode records a turn's agent, not who sent it. The finding is about
//! the pane, its session of record and its server's port. Its remedy is the relaunch with the one
//! dispatcher; with several there is none, and the message lists up to three. A failed view is
//! `observe-failed`, and nothing is concluded. The check writes nothing, and `--fix` never acts
//! on it: a relaunch changes a pane's agent only when the operator names one.
//!
//! [`HarnessPort::agent_view`]: crate::HarnessPort::agent_view

use crate::findings::{quoted, single_quoted, Finding, FindingKind};
use crate::{AgentKey, AgentView, HarnessEndpoint, Pane, Ports};

use super::{observe_failed, own_ports, Server};

/// How many of the session of record's last user turns the rule reads (rule 4).
pub(super) const RECENT_TURNS: usize = 3;

/// The call of the check, named as `<port>.<method>` in an `observe-failed` message.
const HARNESS_AGENT_VIEW: &str = "harness.agent_view";

/// How many dispatchers a message names when no one of them is the remedy.
const LISTED: usize = 3;

/// Run the check of the server pane `pane`, whose harness is at `at` and was observed as
/// `server`, after the parent's session-of-record check.
pub(super) fn check(
    ports: Ports<'_>,
    pane: &Pane,
    at: &HarnessEndpoint,
    server: &Server,
    findings: &mut Vec<Finding>,
) {
    if pane.opencode_agent.is_some() {
        return;
    }
    let Some(record) = pane.session_of_record.as_deref() else {
        return;
    };
    if !server.has(record) {
        return;
    }
    match ports.harness.agent_view(at, record, RECENT_TURNS) {
        Ok(view) => findings.extend(cannot_dispatch(pane, record, &view)),
        Err(error) => findings.push(observe_failed(pane, HARNESS_AGENT_VIEW, &error)),
    }
}

/// `agent-cannot-dispatch` for `pane` when the view of its session of record `record` holds
/// rules 4 and 5, with the relaunch remedy when the view lists exactly one dispatcher.
fn cannot_dispatch(pane: &Pane, record: &str, view: &AgentView) -> Option<Finding> {
    let stuck = !view.turns.is_empty()
        && view
            .turns
            .iter()
            .all(|turn| turn.can_dispatch == Some(false));
    if !stuck || view.dispatchers.is_empty() {
        return None;
    }
    let finding = Finding::about(
        FindingKind::AgentCannotDispatch,
        pane,
        message(pane, record, view),
    )
    .with_session(Some(record))
    .with_ports(own_ports(pane));
    Some(match view.dispatchers.as_slice() {
        [only] => finding.with_agent(only),
        _ => finding,
    })
}

/// The finding's message: the agents the turns ran as, the orchestrator to relaunch with (or up
/// to three to choose from), and for a pane in a profile that the relaunch needs `--profile`, or
/// the profile keeps the old agent (plan review W-3). Every value the server gave is quoted, and
/// so is the profile name: [`quoted`] where the message names it, and POSIX-single-quoted
/// (`single_quoted`, #662 B1) as the `--profile` argument it suggests, so that argument pastes
/// into a shell as one word.
fn message(pane: &Pane, record: &str, view: &AgentView) -> String {
    let turns = match view.turns.len() {
        1 => "the last user turn".to_owned(),
        n => format!("the last {n} user turns"),
    };
    let mut ran_as: Vec<&AgentKey> = Vec::new();
    for agent in view.turns.iter().filter_map(|turn| turn.agent.as_ref()) {
        if !ran_as.contains(&agent) {
            ran_as.push(agent);
        }
    }
    let ran_as = if ran_as.is_empty() {
        "an agent the server does not name".to_owned()
    } else {
        listed(&ran_as)
    };
    let ships = match view.dispatchers.as_slice() {
        [only] => format!("the orchestrator {}", quoted(only.as_str())),
        many => {
            let named: Vec<&AgentKey> = many.iter().take(LISTED).collect();
            let more = match many.len().saturating_sub(LISTED) {
                0 => String::new(),
                rest => format!(" and {rest} more"),
            };
            format!(
                "{} agents that can ({}{more}): relaunch the pane with one of them as its agent \
                 (--agent=KEY)",
                many.len(),
                listed(&named)
            )
        }
    };
    let mut message = format!(
        "{turns} of the session of record {} ran as {ran_as}, which cannot dispatch a subagent, \
         while the project ships {ships}",
        quoted(record)
    );
    if let Some(profile) = &pane.profile {
        message.push_str(&format!(
            "; the pane is in the profile {}, so relaunch it with --profile {} too, or the \
             profile keeps the old agent and the next pass reports profile-drift",
            quoted(profile.as_str()),
            single_quoted(profile)
        ));
    }
    message
}

/// `keys` quoted and joined with commas.
fn listed(keys: &[&AgentKey]) -> String {
    keys.iter()
        .map(|key| quoted(key.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}
