//! `FakeHarness`, the in-memory `HarnessPort` modelled on the OpenCode spike
//! (`docs/research/opencode-pane-spike.md`), and `HarnessOp`, the port's methods as its
//! faults and its call log name them.
//!
//! The fake passes the `HarnessPort` conformance suite
//! ([`crate::conformance::harness`]). What only the fake has is fault injection
//! ([`FakeHarness::faults`]), switches for two quirks of raw OpenCode ([`Quirk`]), a data
//! directory per port, the scenarios the port cannot cause (a frozen or killed server, a
//! session another client creates or deletes, a person moving the TUI, a TUI that exits)
//! and the inspection of servers, TUIs and aborts.
//!
//! The host and harness fakes share no state. `FakeHost::stop_owned` does not stop a
//! server of this fake, and a harness pid is never in `FakeHost::ps`. A test that needs
//! the two to agree drives both, e.g. with a `HostPort` wrapper whose `stop_owned` also
//! calls [`FakeHarness::kill`].

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};

use crate::fault::{FaultSwitch, PortOp};

/// The data directory of every port that was not given one of its own.
const DEFAULT_DIR: &str = "default";

/// The first pid the fake mints; later ones count up from it.
const FIRST_PID: u32 = 20_000;

/// A method of the `HarnessPort` port, as a fault targets it and the call log records
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarnessOp {
    Serve,
    Health,
    CreateSession,
    ListSessions,
    Abort,
    AttachTui,
    SelectSession,
    ShownSession,
}

impl PortOp for HarnessOp {
    fn as_str(self) -> &'static str {
        match self {
            HarnessOp::Serve => "harness.serve",
            HarnessOp::Health => "harness.health",
            HarnessOp::CreateSession => "harness.create_session",
            HarnessOp::ListSessions => "harness.list_sessions",
            HarnessOp::Abort => "harness.abort",
            HarnessOp::AttachTui => "harness.attach_tui",
            HarnessOp::SelectSession => "harness.select_session",
            HarnessOp::ShownSession => "harness.shown_session",
        }
    }
}

/// A behaviour of raw OpenCode that an adapter must hide. Each is off by default, and
/// the conformance suite fails a harness that has one on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quirk {
    /// `select_session` on a pane with no TUI answers `Ok(())` and changes nothing
    /// (opencode-pane-spike.md:122-124).
    SelectAckedWithoutTui,
    /// `abort` of an id the server does not know answers `Ok(())`, and the abort is
    /// logged (opencode-pane-spike.md:171-172).
    AbortUnknownAcked,
}

/// The state of the harness server on a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerState {
    /// It answers.
    Running,
    /// SIGSTOPped: it accepts a connection and never answers
    /// (opencode-pane-spike.md:189-191).
    Frozen,
    /// Dead: it refuses the connection (opencode-pane-spike.md:192). Its sessions stay
    /// in the data directory.
    Killed,
}

/// What [`FakeHarness::server`] reports about the server on a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerView {
    /// The pane it was served for.
    pub name: PaneName,
    pub pid: u32,
    pub state: ServerState,
}

/// What [`FakeHarness::tui`] reports about the TUI in a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiView {
    /// The port of the server it is attached to.
    pub port: u16,
    /// The session it shows; `None` on its home screen.
    pub shown: Option<String>,
}

/// An in-memory `HarnessPort` modelled on the OpenCode spike.
///
/// It holds the servers by port, a data directory per port, the sessions of each data
/// directory in creation order, the TUI of each pane, the quirks that are on and the
/// aborts the servers acknowledged. Every port shares the data directory `"default"`,
/// as the live fleet's servers share one (opencode-pane-spike.md:75-77), until
/// [`FakeHarness::set_data_dir`] gives a port its own.
///
/// A call that *reaches* a port goes on when the server there runs. When the server is
/// frozen, the call answers `timeout` with the method's [`HarnessOp`] as its `op`, the
/// shape a wedged port answers; when it was killed or never served, the call answers
/// `unavailable` ("the harness server on port N"). Each method, in check order:
///
/// - `serve(name, port)`: on a frozen server, `timeout`; on one running for `name`, its
///   pid (idempotent); on one running for another pane, `unavailable` (the port is in
///   use). Otherwise it starts a server with a new pid, counted from 20 000 and never
///   reused. It creates no session: a fresh server has none
///   (opencode-pane-spike.md:69-70).
/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
///   never served, as a timed request that times out or is refused would tell.
/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
///   directory, or list that directory's sessions in creation order.
/// - `abort(port, session)` reaches the port. A session outside its data directory is
///   `session-not-found`; otherwise the abort is logged ([`FakeHarness::aborts`]).
/// - `attach_tui(pane, port, session)` reaches the port. A session outside its data
///   directory is `session-not-found`, and the pane keeps the TUI it had; otherwise the
///   pane's TUI, replacing any earlier one, shows the session.
/// - `select_session(pane, session)`: a pane with no TUI is `unavailable` ("no TUI in
///   pane P"), checked first because the TUI is what names the server. It then reaches
///   the TUI's port. A session outside that port's data directory is
///   `session-not-found`, and the screen stays; otherwise that TUI, and no other, shows
///   the session.
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
///
/// Every port method first passes [`FakeHarness::faults`], which models a wedged
/// *adapter*: under `Fault::Wedged` every method answers `timeout`, `health` and
/// `shown_session` included. [`FakeHarness::freeze`] models a wedged *server* behind a
/// working adapter. A method that fails changes nothing. The configuration, scenario
/// and inspection methods bypass the faults and the call log.
pub struct FakeHarness {
    world: Mutex<World>,
    faults: FaultSwitch<HarnessOp>,
}

/// What the fake holds, behind its lock.
#[derive(Default)]
struct World {
    servers: HashMap<u16, ServerView>,
    /// The ports given a data directory of their own.
    dirs: HashMap<u16, String>,
    /// The session ids of each data directory, in creation order.
    sessions: HashMap<String, Vec<String>>,
    tuis: HashMap<PaneId, TuiView>,
    quirks: HashSet<Quirk>,
    /// Every acknowledged abort, oldest first.
    aborts: Vec<(u16, String)>,
    /// How many session ids were minted: the next one is number `sessions_minted + 1`.
    sessions_minted: u64,
    /// How many pids were minted: the next one is `FIRST_PID + pids_minted`.
    pids_minted: u32,
}

impl FakeHarness {
    /// A harness with no server, session, TUI, quirk or fault, every port on the data
    /// directory `"default"`.
    pub fn new() -> Self {
        Self {
            world: Mutex::new(World::default()),
            faults: FaultSwitch::new(),
        }
    }

    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<HarnessOp> {
        &self.faults
    }

    /// Turn `quirk` on or off.
    pub fn set_quirk(&self, quirk: Quirk, on: bool) {
        let mut world = self.lock();
        if on {
            world.quirks.insert(quirk);
        } else {
            world.quirks.remove(&quirk);
        }
    }

    /// Give the server on `port` the data directory `dir` (by default `"default"`,
    /// which every port shares). The sessions already created stay where they are.
    pub fn set_data_dir(&self, port: u16, dir: &str) {
        self.lock().dirs.insert(port, dir.to_owned());
    }

    /// SIGSTOP the server on `port`: it is frozen. `unavailable` when no server was
    /// ever served on `port`, or it was killed.
    pub fn freeze(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Frozen)
    }

    /// SIGCONT the server on `port`: it runs again. `unavailable` when no server was
    /// ever served on `port`, or it was killed.
    pub fn thaw(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Running)
    }

    /// SIGKILL the server on `port`: it is dead, its sessions stay in the data
    /// directory, and a TUI attached to it keeps its screen. `Ok` on a server already
    /// killed; `unavailable` when no server was ever served on `port`.
    pub fn kill(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Killed)
    }

    /// Another client created a session in the data directory of `port`: a stray
    /// "ping" session, or one a person started by hand on a TUI's home screen. It needs
    /// no running server. Returns its id.
    pub fn seed_session(&self, port: u16) -> String {
        self.lock().mint_session(port)
    }

    /// Another client deleted `session` (`DELETE /session/:id`): it leaves its data
    /// directory, and every TUI showing it goes to its home screen and stays attached
    /// (opencode-pane-spike.md:222). `session-not-found` when no data directory holds
    /// it.
    pub fn delete_session(&self, session: &str) -> Result<(), PaneError> {
        self.lock().delete(session)
    }

    /// A person moved the TUI in `pane` by hand: `Some(id)` shows that session, `None`
    /// goes to the home screen. `unavailable` when there is no TUI in `pane`;
    /// `session-not-found` when the data directory of the TUI's port lacks the id.
    pub fn navigate(&self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError> {
        let mut world = self.lock();
        let port = world.tui_port(pane)?;
        if let Some(session) = session {
            world.known(port, session)?;
        }
        world.show(pane, session)
    }

    /// The TUI process in `pane` exited. `unavailable` when there is no TUI in `pane`.
    pub fn close_tui(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock()
            .tuis
            .remove(pane)
            .map(drop)
            .ok_or_else(|| no_tui(pane))
    }

    /// The server last served on `port`, or `None` when none ever was. A launch test
    /// (#644) reads it to see which pane's server runs on a port, with which pid.
    pub fn server(&self, port: u16) -> Option<ServerView> {
        self.lock().servers.get(&port).cloned()
    }

    /// The TUI in `pane`, or `None` when there is none.
    pub fn tui(&self, pane: &PaneId) -> Option<TuiView> {
        self.lock().tuis.get(pane).cloned()
    }

    /// Every abort a server acknowledged, oldest first, as `(port, session)`.
    pub fn aborts(&self) -> Vec<(u16, String)> {
        self.lock().aborts.clone()
    }

    fn lock(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for FakeHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessPort for FakeHarness {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        self.faults.enter(HarnessOp::Serve)?;
        self.lock().serve(name, port)
    }

    fn health(&self, port: u16) -> Result<bool, PaneError> {
        self.faults.enter(HarnessOp::Health)?;
        Ok(self.lock().state(port) == Some(ServerState::Running))
    }

    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        self.faults.enter(HarnessOp::CreateSession)?;
        let mut world = self.lock();
        world.reach(port, HarnessOp::CreateSession)?;
        Ok(world.mint_session(port))
    }

    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        self.faults.enter(HarnessOp::ListSessions)?;
        let world = self.lock();
        world.reach(port, HarnessOp::ListSessions)?;
        Ok(world.sessions_of(port).to_vec())
    }

    // ASSUMPTION (#642 to confirm): abort stops a model turn as it stops a shell command;
    // the spike verified only a shell command (opencode-pane-spike.md:179, 267).
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        self.faults.enter(HarnessOp::Abort)?;
        let mut world = self.lock();
        world.reach(port, HarnessOp::Abort)?;
        if !world.quirks.contains(&Quirk::AbortUnknownAcked) {
            world.known(port, session)?;
        }
        world.aborts.push((port, session.to_owned()));
        Ok(())
    }

    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        self.faults.enter(HarnessOp::AttachTui)?;
        let mut world = self.lock();
        world.reach(port, HarnessOp::AttachTui)?;
        world.known(port, session)?;
        let tui = TuiView {
            port,
            shown: Some(session.to_owned()),
        };
        world.tuis.insert(pane.clone(), tui);
        Ok(())
    }

    // ASSUMPTION (#642 to confirm): select-session switches a TUI whose --dir is another
    // project directory than the session's; the spike did not try it
    // (opencode-pane-spike.md:272-273). The fake has no project directories and always
    // switches.
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.faults.enter(HarnessOp::SelectSession)?;
        let mut world = self.lock();
        if !world.tuis.contains_key(pane) && world.quirks.contains(&Quirk::SelectAckedWithoutTui) {
            return Ok(());
        }
        let port = world.tui_port(pane)?;
        world.reach(port, HarnessOp::SelectSession)?;
        world.known(port, session)?;
        world.show(pane, Some(session))
    }

    // ASSUMPTION (#642 to confirm): the shown session is read from the pane's terminal
    // title; the spike read it through tmux, and whether Herdr exposes a pane's terminal
    // title is unverified (opencode-pane-spike.md:151-152, 269).
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.faults.enter(HarnessOp::ShownSession)?;
        Ok(self.lock().tuis.get(pane).and_then(|tui| tui.shown.clone()))
    }
}

impl World {
    /// The state of the server on `port`; `None` when none was ever served there.
    fn state(&self, port: u16) -> Option<ServerState> {
        self.servers.get(&port).map(|server| server.state)
    }

    /// Reach the server on `port` for a call of `op`: `Ok` when it runs, otherwise the
    /// frozen `timeout` or the unreachable `unavailable`.
    fn reach(&self, port: u16, op: HarnessOp) -> Result<(), PaneError> {
        match self.state(port) {
            Some(ServerState::Running) => Ok(()),
            Some(ServerState::Frozen) => Err(frozen(op)),
            Some(ServerState::Killed) | None => Err(unreachable_server(port)),
        }
    }

    /// `serve`'s work, past the fault switch.
    fn serve(&mut self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        if let Some(server) = self.servers.get(&port) {
            match server.state {
                ServerState::Running if server.name == *name => return Ok(server.pid),
                ServerState::Running => {
                    return Err(PaneError::Unavailable {
                        what: format!("port {port} is in use by the server of {}", server.name),
                    })
                }
                ServerState::Frozen => return Err(frozen(HarnessOp::Serve)),
                ServerState::Killed => {}
            }
        }
        let pid = FIRST_PID + self.pids_minted;
        self.pids_minted += 1;
        let server = ServerView {
            name: name.clone(),
            pid,
            state: ServerState::Running,
        };
        self.servers.insert(port, server);
        Ok(pid)
    }

    /// Send the server on `port` the signal that leaves it `to`. A port never served is
    /// unreachable, and so is a killed server, except that killing it again is `Ok`.
    fn signal(&mut self, port: u16, to: ServerState) -> Result<(), PaneError> {
        match self.servers.get_mut(&port) {
            Some(server) if server.state != ServerState::Killed || to == ServerState::Killed => {
                server.state = to;
                Ok(())
            }
            _ => Err(unreachable_server(port)),
        }
    }

    /// The data directory of `port`.
    fn dir(&self, port: u16) -> &str {
        self.dirs.get(&port).map_or(DEFAULT_DIR, String::as_str)
    }

    /// The sessions of the data directory of `port`, in creation order.
    fn sessions_of(&self, port: u16) -> &[String] {
        self.sessions
            .get(self.dir(port))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// `Ok` when the data directory of `port` holds `session`; otherwise
    /// `session-not-found`.
    fn known(&self, port: u16, session: &str) -> Result<(), PaneError> {
        if self.sessions_of(port).iter().any(|id| id == session) {
            Ok(())
        } else {
            Err(unknown_session(session))
        }
    }

    /// Mint a new session id in the data directory of `port`.
    fn mint_session(&mut self, port: u16) -> String {
        self.sessions_minted += 1;
        let id = format!("ses_{:026x}", self.sessions_minted);
        let dir = self.dir(port).to_owned();
        self.sessions.entry(dir).or_default().push(id.clone());
        id
    }

    /// `delete_session`'s work: the session leaves its data directory, and every TUI
    /// that shows it goes home.
    fn delete(&mut self, session: &str) -> Result<(), PaneError> {
        let ids = self
            .sessions
            .values_mut()
            .find(|ids| ids.iter().any(|id| id == session))
            .ok_or_else(|| unknown_session(session))?;
        ids.retain(|id| id != session);
        for tui in self.tuis.values_mut() {
            if tui.shown.as_deref() == Some(session) {
                tui.shown = None;
            }
        }
        Ok(())
    }

    /// The port of the TUI in `pane`; the no-TUI `unavailable` when there is none.
    fn tui_port(&self, pane: &PaneId) -> Result<u16, PaneError> {
        self.tuis
            .get(pane)
            .map(|tui| tui.port)
            .ok_or_else(|| no_tui(pane))
    }

    /// The TUI in `pane` shows `session` (`None`: its home screen).
    fn show(&mut self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError> {
        let tui = self.tuis.get_mut(pane).ok_or_else(|| no_tui(pane))?;
        tui.shown = session.map(str::to_owned);
        Ok(())
    }
}

/// What a call of `op` answers when the server it reaches is frozen: the `timeout` a
/// wedged port answers, `op` naming the method.
fn frozen(op: HarnessOp) -> PaneError {
    PaneError::Timeout {
        op: op.as_str().to_owned(),
    }
}

/// What a call answers when the server on `port` was killed or never served.
fn unreachable_server(port: u16) -> PaneError {
    PaneError::Unavailable {
        what: format!("the harness server on port {port}"),
    }
}

/// What a call that needs the TUI in `pane` answers when there is none.
fn no_tui(pane: &PaneId) -> PaneError {
    PaneError::Unavailable {
        what: format!("no TUI in pane {}", pane.as_str()),
    }
}

/// What a call answers for a session id the data directory does not hold.
fn unknown_session(session: &str) -> PaneError {
    PaneError::SessionNotFound {
        what: session.to_owned(),
    }
}
