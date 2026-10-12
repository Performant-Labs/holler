//! `FakeHarnesses`, the router of the two harness fakes (#831, ADR-0021 section 15 item 4; plan
//! review W5): one `HarnessPort` over `FakeHarness` (OpenCode) and `FakeClaude` (Claude Code),
//! built on `holler_pane::HarnessRouter`, the one router the wiring builds too, so a grid that
//! mixes the two harnesses needs no new field in `Ports` and the routing rule exists once.
//!
//! It is also the linker a rig's host calls ([`FakeHarnesses::start`] and
//! [`FakeHarnesses::stop`]): `HostPort::run` of an argv either fake built starts it, and
//! `HostPort::stop_owned` of a pane ends both fakes' processes of that pane, as tmux runs and
//! stops a pane's processes (#802). Every rig that runs a `FakeClaude` links it through these
//! two, and none writes a Claude linker of its own. The three rigs that run only `FakeHarness`
//! and predate #831 (in `pane_verbs`, `profile_verbs` and `pane_integration`) keep their own
//! linking, unchanged.

use holler_pane::ports::{Activity, LoginStatus, SessionChoice};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, HarnessRouter, PaneError, PaneId, PaneName,
    ServeSpec,
};

use crate::claude::FakeClaude;
use crate::harness::FakeHarness;

/// The router of a [`FakeHarness`] (every `Port` endpoint, a server's argv and a TUI's switch)
/// and a [`FakeClaude`] (every `Process` endpoint and the four process methods). Every port
/// method forwards to the router, which routes it as `holler_pane::HarnessRouter`'s table says.
pub struct FakeHarnesses(HarnessRouter<FakeHarness, FakeClaude>);

impl FakeHarnesses {
    /// The two fakes, each new.
    pub fn new() -> Self {
        Self(HarnessRouter::new(FakeHarness::new(), FakeClaude::new()))
    }

    /// The OpenCode fake.
    pub fn opencode(&self) -> &FakeHarness {
        self.0.server()
    }

    /// The Claude Code fake.
    pub fn claude(&self) -> &FakeClaude {
        self.0.process()
    }

    /// Start what `argv` describes, as a host's `run` of it would: the server of an argv
    /// `serve_command` built, or the process of one `process_command` built. `None`, starting
    /// nothing, for any other argv.
    pub fn start(&self, argv: &Argv) -> Option<Result<u32, PaneError>> {
        self.opencode()
            .start_server(argv)
            .or_else(|| self.claude().start(argv))
    }

    /// End every process of the pane `name` in both fakes, and no other pane's: its server is
    /// killed (its sessions stay) and its Claude Code process stopped (its conversation stays).
    pub fn stop(&self, name: &PaneName) {
        self.opencode().kill_servers_of(name);
        self.claude().stop(name);
    }
}

impl Default for FakeHarnesses {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessPort for FakeHarnesses {
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError> {
        self.0.serve_command(name, port, spec)
    }

    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
        self.0.verify_serving(at, spec)
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.0.health(at)
    }

    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
        self.0.create_session(at)
    }

    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        self.0.list_sessions(at)
    }

    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
        self.0.abort(at, session)
    }

    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError> {
        self.0.attach_tui(pane, at, session)
    }

    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.0.select_session(pane, session)
    }

    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError> {
        self.0.shown_session(pane, at)
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.0.send_prompt(at, session, text)
    }

    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        self.0.session_activity(at, session)
    }

    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        self.0.process_command(name, spec, session)
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        self.0.new_session_id()
    }

    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        self.0.locate_process(name, session)
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        self.0.login_status()
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        self.0.agent_view(at, session, max_turns)
    }
}
