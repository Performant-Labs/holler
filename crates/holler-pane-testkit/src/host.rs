//! `FakeHost`, the in-memory `HostPort`: tmux sessions by pane name, the processes in
//! them and the argv of every command run, with fault injection; and `HostOp`, the
//! port's methods as its faults and its call log name them.
//!
//! The fake passes the `HostPort` conformance suite ([`crate::conformance::host`]).
//! What only the fake has is fault injection ([`FakeHost::faults`]), the log of every
//! argv run and the directory it was run in ([`FakeHost::runs`]), the inspection of its
//! sessions, and two scenarios the port cannot cause: a process that exits by itself
//! ([`FakeHost::exit_process`]) and a session killed outside Holler
//! ([`FakeHost::end_session`]).
//!
//! The host and harness fakes share no state. `run` of a server's argv (#802: the argv
//! `HarnessPort::serve_command` builds) starts no [`crate::harness::FakeHarness`] server,
//! `stop_owned` stops none, and a harness pid is never in `ps`. A test that needs the two
//! to agree drives both with a `HostPort` wrapper: its `run` also calls
//! `FakeHarness::start_server(argv)`, and its `stop_owned` also calls
//! `FakeHarness::kill(port)`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use holler_pane::{Argv, HostPort, PaneError, PaneName};

use crate::fault::{FaultSwitch, PortOp};

/// The first pid the fake mints; later ones count up from it.
const FIRST_PID: u32 = 10_000;

/// A method of the `HostPort` port, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostOp {
    EnsureSession,
    Run,
    StopOwned,
    Ps,
    ForegroundCommands,
}

impl PortOp for HostOp {
    fn as_str(self) -> &'static str {
        match self {
            HostOp::EnsureSession => "host.ensure_session",
            HostOp::Run => "host.run",
            HostOp::StopOwned => "host.stop_owned",
            HostOp::Ps => "host.ps",
            HostOp::ForegroundCommands => "host.foreground_commands",
        }
    }
}

/// An in-memory `HostPort`: tmux sessions by pane name, the processes in them and every
/// argv run.
///
/// It keeps the port's rules as the conformance suite pins them for the tmux adapter
/// (#641):
///
/// - `ensure_session` creates a missing session working in `cwd`; on an existing one it
///   is `Ok` and changes nothing, neither the cwd nor the processes.
/// - `run` in a missing session is `pane-not-found`, and an empty argv is `usage`.
///   Otherwise it starts one process in the session and records the argv exactly as
///   given, with the `cwd` it was given: an element is never joined to another or
///   re-split, and nothing goes through a shell. The session keeps the cwd it was made
///   with; the fake does not check the `cwd` of a run (the tmux adapter refuses one that is
///   not an absolute path to an existing directory).
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
/// - `foreground_commands` (#835) of a missing session is `pane-not-found`; otherwise the
///   command name of each of the session's processes, in the order they started: the
///   basename of the argv's first element, as tmux names a pane's process (`sleep` for
///   `/usr/bin/sleep 30`). A name leaves with its process.
///
/// Two behaviours are the fake's own, and the suite does not assert them because a real
/// tmux session may differ: a fresh session has no process (the fake has no shell), so
/// it names no command, and a session survives `stop_owned`. Pids come from one counter
/// that starts at 10 000, so they are distinct across sessions and never reused, not after
/// `stop_owned`, `exit_process` or `end_session`.
///
/// Every port method first passes [`FakeHost::faults`], and when that fails, it returns
/// the error and changes nothing. The inspection and scenario methods bypass the faults
/// and the call log.
pub struct FakeHost {
    state: Mutex<HostState>,
    faults: FaultSwitch<HostOp>,
}

/// What the fake holds, behind its lock.
#[derive(Default)]
struct HostState {
    /// Sorted by name.
    sessions: BTreeMap<PaneName, Session>,
    /// Every argv a successful `run` started, oldest first, with its session and the
    /// `cwd` it was run in.
    runs: Vec<(PaneName, String, Argv)>,
    /// How many pids were minted: the next one is `FIRST_PID + pids_minted`.
    pids_minted: u32,
}

/// One tmux session.
struct Session {
    cwd: String,
    /// Its processes, in the order they started.
    processes: Vec<Process>,
}

/// One process `run` started.
struct Process {
    pid: u32,
    /// Its command name, as `foreground_commands` reports it.
    command: String,
}

impl FakeHost {
    /// A host with no session and no fault.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HostState::default()),
            faults: FaultSwitch::new(),
        }
    }

    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<HostOp> {
        &self.faults
    }

    /// Every session, sorted by name.
    pub fn sessions(&self) -> Vec<PaneName> {
        self.lock().sessions.keys().cloned().collect()
    }

    /// The cwd the session `name` was created with, or `None` when there is no such
    /// session.
    pub fn cwd(&self, name: &PaneName) -> Option<String> {
        self.lock()
            .sessions
            .get(name)
            .map(|session| session.cwd.clone())
    }

    /// Every argv a successful `run` started, oldest first, as `(session, cwd, argv)`, the
    /// `cwd` being the one `run` was given. The log is never cleared, not by `stop_owned`
    /// and not by `end_session`.
    pub fn runs(&self) -> Vec<(PaneName, String, Argv)> {
        self.lock().runs.clone()
    }

    /// The process `pid` of the session `name` exited by itself (it crashed), so it
    /// leaves the session's `ps` and `foreground_commands`. `pane-not-found` when the
    /// session does not exist or does not hold `pid`.
    pub fn exit_process(&self, name: &PaneName, pid: u32) -> Result<(), PaneError> {
        let mut state = self.lock();
        let processes = &mut state
            .sessions
            .get_mut(name)
            .ok_or_else(|| not_found(name))?
            .processes;
        let at = processes
            .iter()
            .position(|held| held.pid == pid)
            .ok_or_else(|| not_found(name))?;
        processes.remove(at);
        Ok(())
    }

    /// The tmux session `name` ended outside Holler (it was killed): the session and
    /// its processes are gone. `pane-not-found` when there is no such session.
    pub fn end_session(&self, name: &PaneName) -> Result<(), PaneError> {
        self.lock()
            .sessions
            .remove(name)
            .map(drop)
            .ok_or_else(|| not_found(name))
    }

    fn lock(&self) -> MutexGuard<'_, HostState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl HostPort for FakeHost {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        self.faults.enter(HostOp::EnsureSession)?;
        self.lock()
            .sessions
            .entry(name.clone())
            .or_insert_with(|| Session {
                cwd: cwd.to_owned(),
                processes: Vec::new(),
            });
        Ok(())
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        self.faults.enter(HostOp::Run)?;
        self.lock().start(name, cwd, argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.faults.enter(HostOp::StopOwned)?;
        if let Some(session) = self.lock().sessions.get_mut(name) {
            session.processes.clear();
        }
        Ok(())
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.faults.enter(HostOp::Ps)?;
        self.lock().each_process(name, |process| process.pid)
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        self.faults.enter(HostOp::ForegroundCommands)?;
        self.lock()
            .each_process(name, |process| process.command.clone())
    }
}

impl HostState {
    /// `run`'s work: refuse a missing session, then an empty argv; otherwise give the
    /// session a new process, named as its program, and log the argv as given, with its
    /// `cwd`.
    fn start(&mut self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
        let Some(program) = argv.as_slice().first() else {
            return Err(PaneError::Usage {
                message: "an empty argv has no program to run".to_owned(),
            });
        };
        session.processes.push(Process {
            pid: FIRST_PID + self.pids_minted,
            command: command_name(program),
        });
        self.pids_minted += 1;
        self.runs.push((name.clone(), cwd.to_owned(), argv.clone()));
        Ok(())
    }

    /// `field` of each process of the session `name`, in the order they started, or
    /// `pane-not-found` when there is no such session.
    fn each_process<T>(
        &self,
        name: &PaneName,
        field: impl Fn(&Process) -> T,
    ) -> Result<Vec<T>, PaneError> {
        self.sessions
            .get(name)
            .map(|session| session.processes.iter().map(field).collect())
            .ok_or_else(|| not_found(name))
    }
}

/// The command name of a process whose program is `program`: its basename, as tmux names a
/// pane's foreground process (`claude` for `/opt/demo/bin/claude`). A program with no
/// basename (`/`) is named as given.
fn command_name(program: &str) -> String {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
        .to_owned()
}

/// The error for a session `name` that does not exist (or a pid it does not hold).
fn not_found(name: &PaneName) -> PaneError {
    PaneError::PaneNotFound {
        what: name.to_string(),
    }
}
