//! The switch/reset transaction (the session of record changes, the TUI follows), story
//! #645. **RED stub** (the pipeline's test-first phase): the public API of the brief
//! (`docs/handoffs/645-brief.md`, "The public API") with bodies that answer
//! `not-implemented`, so the verbs' tests compile and fail on their assertions. F fills it.

use crate::error::RefusalCode;
use crate::{Pane, PaneError, PaneName, Ports, ProfileName};

/// `orchestrator-pane`: the pane's role is `orchestrator` and the caller did not pass
/// `--as-operator`. Refusal, exit 3.
pub const ORCHESTRATOR_PANE: RefusalCode = RefusalCode::from_static("orchestrator-pane");
/// `server-unhealthy`: the pane's harness server does not answer `health`. Refusal, exit 3.
pub const SERVER_UNHEALTHY: RefusalCode = RefusalCode::from_static("server-unhealthy");
/// `session-of-other-pane`: the switch target is another pane's session of record.
/// Refusal, exit 3.
pub const SESSION_OF_OTHER_PANE: RefusalCode = RefusalCode::from_static("session-of-other-pane");
/// The longest session id [`parse_session_id`] accepts.
pub const SESSION_ID_MAX: usize = 64;

/// A harness session id typed by a person: 1..=[`SESSION_ID_MAX`] ASCII letters, digits,
/// `_` or `-`. Anything else is `usage`.
pub fn parse_session_id(text: &str) -> Result<String, PaneError> {
    let _ = text;
    Err(PaneError::NotImplemented)
}

/// The session a run moves the pane to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `switch`: a session the pane's server already has.
    Existing(String),
    /// `reset`: a session this run creates.
    Fresh,
}

/// One switch or reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchRequest {
    pub pane: PaneName,
    pub profile: Option<ProfileName>,
    pub target: Target,
    pub as_operator: bool,
    /// Milliseconds since the Unix epoch, written as `last_observed.at`.
    pub now_ms: i64,
}

/// A run that recorded its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switched {
    /// The record as `cas_put` stored it.
    pub pane: Pane,
    /// The session of record before the run.
    pub previous: Option<String>,
}

/// A run that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchFailure {
    pub error: PaneError,
    /// `select_session` was called (the TUI may have moved): the message carries the
    /// reconcile step.
    pub acted: bool,
    /// The session `reset` created and did not record (it stays on the server).
    pub created: Option<String>,
}

impl From<PaneError> for SwitchFailure {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            acted: false,
            created: None,
        }
    }
}

impl SwitchFailure {
    /// The one-line message of the failure for `pane`.
    pub fn message(&self, pane: &PaneName) -> String {
        let _ = pane;
        self.error.to_string()
    }
}

/// Run one switch or reset over `ports`.
pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
    let _ = (ports, request);
    Err(PaneError::NotImplemented.into())
}
