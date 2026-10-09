//! `FakeHost`, the in-memory `HostPort`: tmux sessions by pane name, the processes in
//! them and the argv of every command run, with fault injection; and `HostOp`, the
//! port's methods as its faults and its call log name them.
//!
//! The fake passes the `HostPort` conformance suite ([`crate::conformance::host`]).
//! What only the fake has is fault injection ([`FakeHost::faults`]), the argv log
//! ([`FakeHost::runs`]), the inspection of its sessions, and two scenarios the port
//! cannot cause: a process that exits by itself ([`FakeHost::exit_process`]) and a
//! session killed outside Holler ([`FakeHost::end_session`]).
//!
//! The host and harness fakes share no state. `stop_owned` does not stop a
//! [`crate::harness::FakeHarness`] server, and a harness pid is never in `ps`. A test
//! that needs the two to agree drives both, e.g. with a `HostPort` wrapper whose
//! `stop_owned` also calls `FakeHarness::kill(port)`.

use std::collections::BTreeMap;
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
}

impl PortOp for HostOp {
    fn as_str(self) -> &'static str {
        match self {
            HostOp::EnsureSession => "host.ensure_session",
            HostOp::Run => "host.run",
            HostOp::StopOwned => "host.stop_owned",
            HostOp::Ps => "host.ps",
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
///   given: an element is never joined to another or re-split, and nothing goes
///   through a shell.
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
///
/// Two behaviours are the fake's own, and the suite does not assert them because a real
/// tmux session may differ: a fresh session has no process (the fake has no shell), and
/// a session survives `stop_owned`. Pids come from one counter that starts at 10 000, so
/// they are distinct across sessions and never reused, not after `stop_owned`,
/// `exit_process` or `end_session`.
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
    /// Every argv a successful `run` started, oldest first, with its session.
    runs: Vec<(PaneName, Argv)>,
    /// How many pids were minted: the next one is `FIRST_PID + pids_minted`.
    pids_minted: u32,
}

/// One tmux session.
struct Session {
    cwd: String,
    /// Its processes, in the order they started.
    pids: Vec<u32>,
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

    /// Every argv a successful `run` started, oldest first, with its session. The log
    /// is never cleared, not by `stop_owned` and not by `end_session`.
    pub fn runs(&self) -> Vec<(PaneName, Argv)> {
        self.lock().runs.clone()
    }

    /// The process `pid` of the session `name` exited by itself (it crashed), so it
    /// leaves the session's `ps`. `pane-not-found` when the session does not exist or
    /// does not hold `pid`.
    pub fn exit_process(&self, name: &PaneName, pid: u32) -> Result<(), PaneError> {
        let mut state = self.lock();
        let pids = &mut state
            .sessions
            .get_mut(name)
            .ok_or_else(|| not_found(name))?
            .pids;
        let at = pids
            .iter()
            .position(|&held| held == pid)
            .ok_or_else(|| not_found(name))?;
        pids.remove(at);
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
                pids: Vec::new(),
            });
        Ok(())
    }

    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
        self.faults.enter(HostOp::Run)?;
        self.lock().start(name, argv)
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.faults.enter(HostOp::StopOwned)?;
        if let Some(session) = self.lock().sessions.get_mut(name) {
            session.pids.clear();
        }
        Ok(())
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        self.faults.enter(HostOp::Ps)?;
        self.lock()
            .sessions
            .get(name)
            .map(|session| session.pids.clone())
            .ok_or_else(|| not_found(name))
    }
}

impl HostState {
    /// `run`'s work: refuse a missing session, then an empty argv; otherwise give the
    /// session a new pid and log the argv as given.
    fn start(&mut self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
        let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
        if argv.as_slice().is_empty() {
            return Err(PaneError::Usage {
                message: "an empty argv has no program to run".to_owned(),
            });
        }
        session.pids.push(FIRST_PID + self.pids_minted);
        self.pids_minted += 1;
        self.runs.push((name.clone(), argv.clone()));
        Ok(())
    }
}

/// The error for a session `name` that does not exist (or a pid it does not hold).
fn not_found(name: &PaneName) -> PaneError {
    PaneError::PaneNotFound {
        what: name.to_string(),
    }
}
