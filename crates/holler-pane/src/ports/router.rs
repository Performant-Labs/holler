//! The one router of the two harness shapes (#831, ADR-0021 section 15 item 4; plan review W5).
//!
//! `Ports` keeps one `harness`. A grid that mixes OpenCode and Claude Code panes reaches both
//! through a [`HarnessRouter`] that holds the server harness and the single-process harness and
//! sends each call to one of them. The wiring builds it over the two adapters, and the test
//! kit's `FakeHarnesses` over the two fakes: the routing rule exists once, here, because the
//! test kit cannot depend on the wiring nor the wiring on the test kit.
//!
//! **The routing table.**
//!
//! | Call | Goes to |
//! |---|---|
//! | every method keyed by an endpoint (`verify_serving`, `health`, `create_session`, `list_sessions`, `abort`, `attach_tui`, `shown_session`, `send_prompt`, `session_activity`, `agent_view`, `compact`) | the server harness for [`HarnessEndpoint::Port`], the process harness for [`HarnessEndpoint::Process`] |
//! | `serve_command`, `select_session` (a server's argv; a TUI's switch) | the server harness |
//! | `process_command`, `new_session_id`, `locate_process`, `login_status` | the process harness |
//!
//! A call is forwarded as it is, the methods with a default included, so a default of the
//! router never answers in place of the harness a call is routed to.

use crate::argv::Argv;
use crate::error::PaneError;
use crate::pane::{PaneId, PaneName};

use super::{
    Activity, AgentView, CompactModel, CompactOutcome, HarnessEndpoint, HarnessPort, LoginStatus,
    ServeSpec, SessionChoice,
};

/// A `HarnessPort` over a server harness `S` (OpenCode) and a single-process harness `P`
/// (Claude Code), routing each call as the module docs' table says.
pub struct HarnessRouter<S, P> {
    server: S,
    process: P,
}

impl<S, P> HarnessRouter<S, P> {
    /// The router of `server`, the harness of every `Port` endpoint, and `process`, the
    /// harness of every `Process` endpoint.
    pub fn new(server: S, process: P) -> Self {
        Self { server, process }
    }

    /// The server harness.
    pub fn server(&self) -> &S {
        &self.server
    }

    /// The single-process harness.
    pub fn process(&self) -> &P {
        &self.process
    }
}

impl<S: HarnessPort, P: HarnessPort> HarnessRouter<S, P> {
    /// The harness `endpoint` names.
    fn by(&self, endpoint: &HarnessEndpoint) -> &dyn HarnessPort {
        match endpoint {
            HarnessEndpoint::Port(_) => &self.server,
            HarnessEndpoint::Process { .. } => &self.process,
        }
    }
}

impl<S: HarnessPort, P: HarnessPort> HarnessPort for HarnessRouter<S, P> {
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        self.server.serve_command(name, port, spec)
    }

    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.by(at).verify_serving(at, spec)
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.by(at).health(at)
    }

    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.by(at).create_session(at)
    }

    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.by(at).list_sessions(at)
    }

    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.by(at).abort(at, session)
    }

    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        self.by(at).attach_tui(pane, at, session)
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.server.select_session(pane, session)
    }

    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.by(at).shown_session(pane, at)
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.by(at).send_prompt(at, session, text)
    }

    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        self.by(at).session_activity(at, session)
    }

    fn compact(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        model: Option<&CompactModel>,
    ) -> Result<CompactOutcome, PaneError> {
        self.by(at).compact(at, session, model)
    }

    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        self.process.process_command(name, spec, session)
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        self.process.new_session_id()
    }

    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        self.process.locate_process(name, session)
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        self.process.login_status()
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        self.by(at).agent_view(at, session, max_turns)
    }
}
