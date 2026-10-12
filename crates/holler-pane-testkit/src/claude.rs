//! `FakeClaude`, the in-memory single-process `HarnessPort` modelled on the Claude Code spike
//! (`docs/research/claude-code-pane-spike.md`, #830), and `ClaudeOp`, the port's methods as its
//! faults and its call log name them (#831, ADR-0021 section 15 item 15).
//!
//! The fake passes the Claude Code conformance suite ([`crate::conformance::claude`]). What
//! only the fake has is fault injection ([`FakeClaude::faults`]), the scenarios the port cannot
//! cause (a process started from the argv `process_command` built, a process run by hand, a
//! person's `/clear`, a SIGINT, a stall, a permission prompt, a question, a login that goes
//! missing) and the inspection of processes, conversations and the specs asked for.
//!
//! **What it models.** A process holds one session. A session's conversation (its transcript)
//! exists only once the session was first prompted, and outlives its process. Starting a
//! process on `--session-id` of a session that a live process holds, or that has a
//! conversation, is `session-in-use`; on `--resume` of a held session it is `session-in-use`,
//! and of a session with no conversation `no-conversation`. A process started from the fake's
//! own argv accepts cross-session messages (Holler's launch carries `crossSessionInbound:
//! accept`); one run by hand does not, so a message to it is held behind a dialog
//! (`inbound-held`) and it waits on a permission prompt. A person's `/clear` moves the process
//! to a new session and keeps its pid; a SIGINT ends it (it is no abort).
//!
//! **The port**, each method after [`FakeClaude::faults`]:
//!
//! - `process_command(name, spec, choice)`: pure; it records `spec` ([`FakeClaude::process_specs`])
//!   and returns `holler-fake-claude --pane <name> --session-id|--resume <id> --settings
//!   <accept>`: `Fresh` takes `--session-id`, `Keep` `--resume` when the session has a
//!   conversation and `--session-id` otherwise, `Resume` `--resume`, or `session-not-found` when
//!   it has none. An agent is `agent-unsupported`, and a spec with no directory `usage`.
//! - `new_session_id`: a fresh UUID-shaped id, counted.
//! - `locate_process(name, session)`: the live process of `name` holding `session`.
//! - `health(at)`: whether the process `at` names (that pane, that pid) still runs.
//! - `shown_session(_, at)`: the session that process holds now, `None` when it is gone; read
//!   through the endpoint, never a TUI.
//! - `session_activity(at, _)`: the process's status: `idle`, `busy`, `waiting` on `input
//!   needed` (`HoldingQuestion`) or on anything else (`WaitingPermission`).
//! - `send_prompt(at, session, text)`: queued to the process's inbox, which starts the
//!   session's conversation; `session-mismatch` when the process holds another session, and
//!   `inbound-held` for a process that does not accept a cross-session message.
//! - `abort`: `abort-unsupported`. `login_status`: the login [`FakeClaude::set_login`] set (`Ok`
//!   at first).
//! - `agent_view(at, _, _)` (#647 part 2): the empty view for a process endpoint, whatever the
//!   process is doing (a single process has no server to list agents or turns, so it cannot
//!   tell), and `server-unsupported` for a port, as the real adapter answers.
//! - A server's or a TUI's step (`serve_command`, `verify_serving`, `create_session`,
//!   `list_sessions`, `attach_tui`, `select_session`) is `server-unsupported`, and so is an
//!   endpoint that names a port. A call to a process that does not run is `unavailable`.
//!
//! The host and this fake share no state: `FakeHost::run` of the fake's argv starts no process
//! here. A rig links the two through `FakeHarnesses::start` and `stop` (the one linker,
//! [`crate::harnesses`]).

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use holler_pane::error::RefusalCode;
use holler_pane::ports::{
    Activity, LoginStatus, SessionChoice, ABORT_UNSUPPORTED, AGENT_UNSUPPORTED, INBOUND_HELD,
    SERVER_UNSUPPORTED, SESSION_MISMATCH,
};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
};

use crate::fault::{FaultSwitch, PortOp};

/// The first pid the fake mints; later ones count up from it (distinct from `FakeHost`'s and
/// `FakeHarness`'s).
const FIRST_PID: u32 = 30_000;

/// The program of every argv [`FakeClaude::process_command`] builds, which no real program
/// is, and its flags, each followed by its value: [`FakeClaude::start`] reads them.
const PROGRAM: &str = "holler-fake-claude";
const PANE_FLAG: &str = "--pane";
const SESSION_ID_FLAG: &str = "--session-id";
const RESUME_FLAG: &str = "--resume";
const SETTINGS_FLAG: &str = "--settings";
/// The settings of the fake's own argv: Holler's launch accepts cross-session messages.
const ACCEPT_SETTINGS: &str = r#"{"crossSessionInbound":"accept"}"#;

/// The `waitingFor` of a permission prompt, and of a question (the spike's two values).
const PERMISSION_PROMPT: &str = "permission prompt";
const INPUT_NEEDED: &str = "input needed";

/// `session-in-use`: the fake's start of a process on a session that a live process holds, or
/// on `--session-id` of a session that has a conversation (the binary's own refusal; the fake's
/// open code). A refusal, exit 3.
pub const SESSION_IN_USE: RefusalCode = RefusalCode::from_static("session-in-use");

/// `no-conversation`: the fake's start of a process on `--resume` of a session that has no
/// conversation (the binary's "No conversation found"; the fake's open code). A refusal, exit 3.
pub const NO_CONVERSATION: RefusalCode = RefusalCode::from_static("no-conversation");

/// A method of the `HarnessPort` port, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaudeOp {
    ServeCommand,
    VerifyServing,
    Health,
    CreateSession,
    ListSessions,
    Abort,
    AttachTui,
    SelectSession,
    ShownSession,
    SendPrompt,
    SessionActivity,
    ProcessCommand,
    NewSessionId,
    LocateProcess,
    LoginStatus,
    AgentView,
}

impl PortOp for ClaudeOp {
    fn as_str(self) -> &'static str {
        match self {
            ClaudeOp::ServeCommand => "harness.serve_command",
            ClaudeOp::VerifyServing => "harness.verify_serving",
            ClaudeOp::Health => "harness.health",
            ClaudeOp::CreateSession => "harness.create_session",
            ClaudeOp::ListSessions => "harness.list_sessions",
            ClaudeOp::Abort => "harness.abort",
            ClaudeOp::AttachTui => "harness.attach_tui",
            ClaudeOp::SelectSession => "harness.select_session",
            ClaudeOp::ShownSession => "harness.shown_session",
            ClaudeOp::SendPrompt => "harness.send_prompt",
            ClaudeOp::SessionActivity => "harness.session_activity",
            ClaudeOp::ProcessCommand => "harness.process_command",
            ClaudeOp::NewSessionId => "harness.new_session_id",
            ClaudeOp::LocateProcess => "harness.locate_process",
            ClaudeOp::LoginStatus => "harness.login_status",
            ClaudeOp::AgentView => "harness.agent_view",
        }
    }
}

/// A process's status, as `claude agents --json` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeStatus {
    Idle,
    Busy,
    /// Waiting for a person; [`ProcessView::waiting_for`] says on what.
    Waiting,
}

/// What [`FakeClaude::process`] reports about one live process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessView {
    /// The pane whose tmux session runs it.
    pub pane: PaneName,
    pub pid: u32,
    /// The session it holds now.
    pub session_id: String,
    pub status: ClaudeStatus,
    /// What it waits on (`permission prompt`, `input needed`), while it waits.
    pub waiting_for: Option<String>,
    /// Every message delivered to it, oldest first.
    pub inbox: Vec<String>,
    /// Whether it accepts a cross-session message (Holler's launch does; a process run by
    /// hand does not).
    pub accept_inbound: bool,
}

/// An in-memory single-process `HarnessPort` modelled on the Claude Code spike (see the module
/// docs).
pub struct FakeClaude {
    world: Mutex<World>,
    faults: FaultSwitch<ClaudeOp>,
}

/// What the fake holds, behind its lock.
struct World {
    /// The live processes, in the order they started.
    processes: Vec<ProcessView>,
    /// The sessions that have a conversation (a transcript).
    conversations: HashSet<String>,
    /// Every spec a successful `process_command` was asked for, oldest first.
    process_specs: Vec<ServeSpec>,
    login: LoginStatus,
    /// How many session ids were minted.
    sessions_minted: u64,
    /// How many pids were minted: the next one is `FIRST_PID + pids_minted`.
    pids_minted: u32,
}

impl FakeClaude {
    /// A harness with no process, no conversation, no fault, and the subscription login.
    pub fn new() -> Self {
        Self {
            world: Mutex::new(World {
                processes: Vec::new(),
                conversations: HashSet::new(),
                process_specs: Vec::new(),
                login: LoginStatus::Ok,
                sessions_minted: 0,
                pids_minted: 0,
            }),
            faults: FaultSwitch::new(),
        }
    }

    /// The fault switch of every port method and the log of the calls made through the port.
    pub fn faults(&self) -> &FaultSwitch<ClaudeOp> {
        &self.faults
    }

    /// Start the process `argv` describes, as a host's `run` of it would; `None`, starting
    /// nothing, for any argv `process_command` does not build. Its refusals are the binary's:
    /// `session-in-use` and `no-conversation` (see the module docs). Returns the new pid.
    pub fn start(&self, argv: &Argv) -> Option<Result<u32, PaneError>> {
        let start = described(argv)?;
        Some(self.lock().start(start))
    }

    /// Stop every live process of the pane `name`, as `HostPort::stop_owned` of its tmux
    /// session would. Their conversations stay.
    pub fn stop(&self, name: &PaneName) {
        self.lock()
            .processes
            .retain(|process| process.pane != *name);
    }

    /// Every live process, in the order they started.
    pub fn processes(&self) -> Vec<ProcessView> {
        self.lock().processes.clone()
    }

    /// The live process `pid`, if it runs.
    pub fn process(&self, pid: u32) -> Option<ProcessView> {
        self.lock()
            .processes
            .iter()
            .find(|process| process.pid == pid)
            .cloned()
    }

    /// Every spec a successful `process_command` was asked for, oldest first.
    pub fn process_specs(&self) -> Vec<ServeSpec> {
        self.lock().process_specs.clone()
    }

    /// The session `session` has had a conversation (it was prompted elsewhere, or earlier).
    pub fn seed_conversation(&self, session: &str) {
        self.lock().conversations.insert(session.to_owned());
    }

    /// Whether `session` has a conversation.
    pub fn has_conversation(&self, session: &str) -> bool {
        self.lock().conversations.contains(session)
    }

    /// A person started a Claude Code process in the pane `name` by hand, holding `session`,
    /// without Holler's settings: it does not accept a cross-session message. Returns its pid.
    pub fn run_by_hand(&self, name: &PaneName, session: &str) -> u32 {
        self.lock().spawn(name.clone(), session.to_owned(), false)
    }

    /// A person typed `/clear` in the process `pid`: it now holds a new session (returned),
    /// with the same pid. `unavailable` when no such process runs.
    pub fn clear(&self, pid: u32) -> Result<String, PaneError> {
        let mut world = self.lock();
        let session = world.mint_session();
        world.running(pid)?.session_id.clone_from(&session);
        Ok(session)
    }

    /// The process `pid` got a SIGINT, which ends it (a signal is not an abort). Its
    /// conversation stays. `unavailable` when no such process runs.
    pub fn sigint(&self, pid: u32) -> Result<(), PaneError> {
        let mut world = self.lock();
        world.running(pid)?;
        world.processes.retain(|process| process.pid != pid);
        Ok(())
    }

    /// The process `pid` is running a turn (or is stuck in one): `busy`.
    pub fn stall(&self, pid: u32) -> Result<(), PaneError> {
        self.lock().set_status(pid, ClaudeStatus::Busy, None)
    }

    /// The process `pid` waits on a permission prompt.
    pub fn hold_permission(&self, pid: u32) -> Result<(), PaneError> {
        self.lock()
            .set_status(pid, ClaudeStatus::Waiting, Some(PERMISSION_PROMPT))
    }

    /// The process `pid` waits for the answer to a question.
    pub fn hold_question(&self, pid: u32) -> Result<(), PaneError> {
        self.lock()
            .set_status(pid, ClaudeStatus::Waiting, Some(INPUT_NEEDED))
    }

    /// The process `pid` is idle again.
    pub fn set_idle(&self, pid: u32) -> Result<(), PaneError> {
        self.lock().set_status(pid, ClaudeStatus::Idle, None)
    }

    /// What `login_status` answers from now on.
    pub fn set_login(&self, login: LoginStatus) {
        self.lock().login = login;
    }

    fn lock(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A server's or a TUI's step, which a single process does not have: `server-unsupported`,
    /// after the fault switch.
    fn no_server(&self, op: ClaudeOp) -> PaneError {
        if let Err(fault) = self.faults.enter(op) {
            return fault;
        }
        PaneError::Refused {
            code: SERVER_UNSUPPORTED,
            message: format!(
                "{} is a server's or a TUI's step, and a Claude Code process has neither",
                op.as_str()
            ),
        }
    }
}

impl Default for FakeClaude {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessPort for FakeClaude {
    fn serve_command(&self, _: &PaneName, _: u16, _: &ServeSpec) -> Result<Argv, PaneError> {
        Err(self.no_server(ClaudeOp::ServeCommand))
    }

    fn verify_serving(&self, _: &HarnessEndpoint, _: &ServeSpec) -> Result<(), PaneError> {
        Err(self.no_server(ClaudeOp::VerifyServing))
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        self.faults.enter(ClaudeOp::Health)?;
        let (pane, pid) = at.process()?;
        Ok(self.lock().find(pane, pid).is_some())
    }

    fn create_session(&self, _: &HarnessEndpoint) -> Result<String, PaneError> {
        Err(self.no_server(ClaudeOp::CreateSession))
    }

    fn list_sessions(&self, _: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        Err(self.no_server(ClaudeOp::ListSessions))
    }

    fn abort(&self, at: &HarnessEndpoint, _session: &str) -> Result<(), PaneError> {
        self.faults.enter(ClaudeOp::Abort)?;
        let (pane, pid) = at.process()?;
        Err(PaneError::Refused {
            code: ABORT_UNSUPPORTED,
            message: format!(
                "the harness process {pid} of {pane} has no in-place abort; interrupt relaunches \
                 the pane on its session of record"
            ),
        })
    }

    fn attach_tui(&self, _: &PaneId, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        Err(self.no_server(ClaudeOp::AttachTui))
    }

    fn select_session(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
        Err(self.no_server(ClaudeOp::SelectSession))
    }

    fn shown_session(&self, _: &PaneId, at: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
        self.faults.enter(ClaudeOp::ShownSession)?;
        let (pane, pid) = at.process()?;
        let world = self.lock();
        Ok(world
            .find(pane, pid)
            .map(|process| process.session_id.clone()))
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.faults.enter(ClaudeOp::SendPrompt)?;
        let (pane, pid) = at.process()?;
        self.lock().deliver(pane, pid, session, text)
    }

    fn session_activity(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
    ) -> Result<Activity, PaneError> {
        self.faults.enter(ClaudeOp::SessionActivity)?;
        let (pane, pid) = at.process()?;
        let world = self.lock();
        let process = world.find(pane, pid).ok_or_else(|| gone(pane, pid))?;
        Ok(match (process.status, process.waiting_for.as_deref()) {
            (ClaudeStatus::Idle, _) => Activity::Idle,
            (ClaudeStatus::Busy, _) => Activity::Busy,
            (ClaudeStatus::Waiting, Some(INPUT_NEEDED)) => Activity::HoldingQuestion,
            (ClaudeStatus::Waiting, _) => Activity::WaitingPermission,
        })
    }

    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        self.faults.enter(ClaudeOp::ProcessCommand)?;
        if spec.agent.is_some() {
            return Err(PaneError::Refused {
                code: AGENT_UNSUPPORTED,
                message: format!("{name}: a Claude Code pane takes no agent in this story"),
            });
        }
        if spec.cwd.is_none() {
            return Err(PaneError::Usage {
                message: format!("{name}: a Claude Code process needs the pane's directory"),
            });
        }
        let mut world = self.lock();
        let flag = world.flag_for(session)?;
        world.process_specs.push(spec.clone());
        Ok(description(name, flag, session.session()))
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        self.faults.enter(ClaudeOp::NewSessionId)?;
        Ok(self.lock().mint_session())
    }

    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        self.faults.enter(ClaudeOp::LocateProcess)?;
        let world = self.lock();
        Ok(world
            .processes
            .iter()
            .find(|process| process.pane == *name && process.session_id == session)
            .map(|process| process.pid))
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        self.faults.enter(ClaudeOp::LoginStatus)?;
        Ok(self.lock().login)
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
        _max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        self.faults.enter(ClaudeOp::AgentView)?;
        at.process()?;
        Ok(AgentView::default())
    }
}

/// What an argv the fake built asks to start.
struct Start {
    pane: PaneName,
    /// `--resume` (true) or `--session-id` (false).
    resume: bool,
    session: String,
    accept_inbound: bool,
}

impl World {
    /// `start`'s work, once the argv was read as the fake's.
    fn start(&mut self, start: Start) -> Result<u32, PaneError> {
        let held = self
            .processes
            .iter()
            .any(|process| process.session_id == start.session);
        let talked = self.conversations.contains(&start.session);
        if held || (!start.resume && talked) {
            return Err(PaneError::Refused {
                code: SESSION_IN_USE,
                message: format!("session {} is already in use", start.session),
            });
        }
        if start.resume && !talked {
            return Err(PaneError::Refused {
                code: NO_CONVERSATION,
                message: format!("no conversation found with session {}", start.session),
            });
        }
        Ok(self.spawn(start.pane, start.session, start.accept_inbound))
    }

    /// A new live process of `pane` holding `session`, idle; its pid.
    fn spawn(&mut self, pane: PaneName, session: String, accept_inbound: bool) -> u32 {
        let pid = FIRST_PID + self.pids_minted;
        self.pids_minted += 1;
        self.processes.push(ProcessView {
            pane,
            pid,
            session_id: session,
            status: ClaudeStatus::Idle,
            waiting_for: None,
            inbox: Vec::new(),
            accept_inbound,
        });
        pid
    }

    /// The live process `pid` of the pane `pane`.
    fn find(&self, pane: &PaneName, pid: u32) -> Option<&ProcessView> {
        self.processes
            .iter()
            .find(|process| process.pid == pid && process.pane == *pane)
    }

    /// The live process `pid`, to change; `unavailable` when it does not run.
    fn running(&mut self, pid: u32) -> Result<&mut ProcessView, PaneError> {
        self.processes
            .iter_mut()
            .find(|process| process.pid == pid)
            .ok_or_else(|| PaneError::Unavailable {
                what: format!("no Claude Code process {pid} runs"),
            })
    }

    /// Set the status of the live process `pid`.
    fn set_status(
        &mut self,
        pid: u32,
        status: ClaudeStatus,
        waiting_for: Option<&str>,
    ) -> Result<(), PaneError> {
        let process = self.running(pid)?;
        process.status = status;
        process.waiting_for = waiting_for.map(str::to_owned);
        Ok(())
    }

    /// `send_prompt`'s work: deliver `text` to `session`, held by the process `pid` of `pane`.
    fn deliver(
        &mut self,
        pane: &PaneName,
        pid: u32,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let process = self
            .processes
            .iter_mut()
            .find(|process| process.pid == pid && process.pane == *pane)
            .ok_or_else(|| gone(pane, pid))?;
        if process.session_id != session {
            return Err(PaneError::Refused {
                code: SESSION_MISMATCH,
                message: format!(
                    "the harness process {pid} of {pane} holds another session than the one named"
                ),
            });
        }
        if !process.accept_inbound {
            process.status = ClaudeStatus::Waiting;
            process.waiting_for = Some(PERMISSION_PROMPT.to_owned());
            return Err(PaneError::Refused {
                code: INBOUND_HELD,
                message: format!(
                    "the harness process {pid} of {pane} held the message behind a dialog: it was \
                     not started by Holler, so it does not accept a cross-session message"
                ),
            });
        }
        process.inbox.push(text.to_owned());
        self.conversations.insert(session.to_owned());
        Ok(())
    }

    /// The flag `process_command` builds for `choice`: `--resume` or `--session-id`, or
    /// `session-not-found` for a resume of a session with no conversation.
    fn flag_for(&self, choice: &SessionChoice) -> Result<&'static str, PaneError> {
        let talked = self.conversations.contains(choice.session());
        match choice {
            SessionChoice::Fresh(_) => Ok(SESSION_ID_FLAG),
            SessionChoice::Keep(_) if talked => Ok(RESUME_FLAG),
            SessionChoice::Keep(_) => Ok(SESSION_ID_FLAG),
            SessionChoice::Resume(_) if talked => Ok(RESUME_FLAG),
            SessionChoice::Resume(session) => Err(PaneError::SessionNotFound {
                what: format!("{session} (it has no conversation to resume)"),
            }),
        }
    }

    /// A new session id, UUID-shaped, counted.
    fn mint_session(&mut self) -> String {
        self.sessions_minted += 1;
        format!("00000000-0000-4000-8000-{:012x}", self.sessions_minted)
    }
}

/// The argv `process_command` builds: the fake's program, the pane, the session flag and its
/// id, and the settings that accept cross-session messages, each value an element of its own.
fn description(name: &PaneName, flag: &str, session: &str) -> Argv {
    Argv::new(vec![
        PROGRAM.to_owned(),
        PANE_FLAG.to_owned(),
        name.as_str().to_owned(),
        flag.to_owned(),
        session.to_owned(),
        SETTINGS_FLAG.to_owned(),
        ACCEPT_SETTINGS.to_owned(),
    ])
}

/// What an argv [`description`] built asks to start; `None` for any other argv.
fn described(argv: &Argv) -> Option<Start> {
    let [program, pane_flag, pane, flag, session, settings_flag, settings] = argv.as_slice() else {
        return None;
    };
    if program != PROGRAM || pane_flag != PANE_FLAG || settings_flag != SETTINGS_FLAG {
        return None;
    }
    let resume = match flag.as_str() {
        RESUME_FLAG => true,
        SESSION_ID_FLAG => false,
        _ => return None,
    };
    Some(Start {
        pane: PaneName::parse(pane).ok()?,
        resume,
        session: session.clone(),
        accept_inbound: settings == ACCEPT_SETTINGS,
    })
}

/// What a call to the process `pid` of `pane` answers when it does not run.
fn gone(pane: &PaneName, pid: u32) -> PaneError {
    PaneError::Unavailable {
        what: format!("the harness process {pid} of {pane} does not run"),
    }
}
