//! What [`HarnessPort::agent_view`] reports (#647 part 2, ADR-0021 section 12): the agents of a
//! session's recent user turns, and the agents of the harness that can dispatch.
//!
//! An observation (I6), read fresh and never stored: reconcile's `agent-cannot-dispatch` rule
//! reads it, and the judgments (an opt-out, "the project ships an orchestrator") stay there, so
//! a harness reports what its server says and never a finding. "Cannot tell" is the empty view or
//! a `None`, never a guess.
//!
//! [`HarnessPort::agent_view`]: super::HarnessPort::agent_view

use crate::argv::AgentKey;

/// What a harness server says about the agents of one session, as
/// [`HarnessPort::agent_view`](super::HarnessPort::agent_view) answers it.
///
/// `#[non_exhaustive]`, as [`PaneObservation`](super::PaneObservation) is: outside this crate a
/// view is built only from its empty default and its builders, so a struct literal does not
/// compile there, and the order of `turns` and the sorting of `dispatchers` are each kept in one
/// place, whatever an implementation reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AgentView {
    /// The last user turns of the session the call names (at most the `max_turns` it asked
    /// for), oldest first.
    pub turns: Vec<TurnAgent>,
    /// The agents the server lists that a pane could run as its default (not a subagent, not
    /// hidden), that are not the harness's own, and that can dispatch: sorted by key, without
    /// duplicates. A name that is not an [`AgentKey`] is left out.
    pub dispatchers: Vec<AgentKey>,
}

impl AgentView {
    /// This view, with `turns` as its turns, kept in the order given (oldest first).
    #[must_use]
    pub fn with_turns(mut self, turns: impl IntoIterator<Item = TurnAgent>) -> Self {
        self.turns = turns.into_iter().collect();
        self
    }

    /// This view, with `dispatchers` as its dispatchers, sorted by key and without duplicates:
    /// the one place that rule is kept.
    #[must_use]
    pub fn with_dispatchers(mut self, dispatchers: impl IntoIterator<Item = AgentKey>) -> Self {
        let mut keys: Vec<AgentKey> = dispatchers.into_iter().collect();
        keys.sort();
        keys.dedup();
        self.dispatchers = keys;
        self
    }
}

/// One user turn of a session, as a harness server recorded it.
///
/// `#[non_exhaustive]`: built with [`TurnAgent::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TurnAgent {
    /// The agent the server recorded for the turn; `None` when it recorded none, or a name that
    /// is not an [`AgentKey`].
    pub agent: Option<AgentKey>,
    /// Whether that agent can dispatch a subagent: `Some(false)` only when the server's
    /// permission rules for it deny dispatching to every subagent, `Some(true)` when they do
    /// not, and `None` when the server does not say (no agent, an agent it does not list, or
    /// rules of a shape the implementation does not know).
    pub can_dispatch: Option<bool>,
}

impl TurnAgent {
    /// A turn run as `agent`, which `can_dispatch` or not, as the server says.
    pub fn new(agent: Option<AgentKey>, can_dispatch: Option<bool>) -> Self {
        Self {
            agent,
            can_dispatch,
        }
    }
}
