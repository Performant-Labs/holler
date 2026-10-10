//! `holler_adapter_host` — the host adapter: it implements `holler_pane::HostPort`
//! (tmux sessions and the processes in them) for the machine the hub runs on
//! (epic #633).
//!
//! **RED stub (#641, Phase 4).** The tester landed the public signatures of the
//! brief's Decision 1 with no logic, so the tests compile and fail on their
//! assertions. Every port method answers `not-implemented`. The Feature implementor
//! replaces this file.

use std::path::PathBuf;
use std::time::Duration;

use holler_pane::{Argv, HostPort, PaneError, PaneName};

/// The default bound on every port call (I5).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// The default grace between `TERM` and `KILL` in `stop_owned`.
const DEFAULT_STOP_GRACE: Duration = Duration::from_secs(2);

/// Which tmux server the adapter talks to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxSocket {
    /// tmux's own default socket (no `-L` or `-S`).
    Default,
    /// A named socket in tmux's socket directory (`-L <name>`).
    Name(String),
    /// A socket at a path (`-S <path>`).
    Path(PathBuf),
}

/// The real `HostPort`: a local tmux server, driven by argument vectors, never a
/// shell.
#[derive(Debug, Clone)]
pub struct TmuxHost {
    socket: TmuxSocket,
    tmux: PathBuf,
    kill: PathBuf,
    config: Option<PathBuf>,
    timeout: Duration,
    stop_grace: Duration,
}

impl TmuxHost {
    /// A host on `socket`, with `tmux` and `kill` from `PATH`, no config file, a 10 s
    /// bound and a 2 s stop grace. Does no I/O.
    pub fn new(socket: TmuxSocket) -> Self {
        Self {
            socket,
            tmux: PathBuf::from("tmux"),
            kill: PathBuf::from("kill"),
            config: None,
            timeout: DEFAULT_TIMEOUT,
            stop_grace: DEFAULT_STOP_GRACE,
        }
    }

    /// The tmux binary to run.
    #[must_use]
    pub fn with_tmux_binary(mut self, path: PathBuf) -> Self {
        self.tmux = path;
        self
    }

    /// The `kill` binary every signal goes through.
    #[must_use]
    pub fn with_kill_binary(mut self, path: PathBuf) -> Self {
        self.kill = path;
        self
    }

    /// A tmux config file (`-f <path>`).
    #[must_use]
    pub fn with_config(mut self, path: PathBuf) -> Self {
        self.config = Some(path);
        self
    }

    /// The bound on every port call.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The grace between `TERM` and `KILL` in `stop_owned`.
    #[must_use]
    pub fn with_stop_grace(mut self, grace: Duration) -> Self {
        self.stop_grace = grace;
        self
    }

    /// The RED stub's one answer; it reads every field so the stub has no dead code.
    fn stub(&self) -> PaneError {
        let _configured = (
            &self.socket,
            &self.tmux,
            &self.kill,
            &self.config,
            self.timeout,
            self.stop_grace,
        );
        PaneError::NotImplemented
    }
}

impl HostPort for TmuxHost {
    fn ensure_session(&self, _name: &PaneName, _cwd: &str) -> Result<(), PaneError> {
        Err(self.stub())
    }

    fn run(&self, _name: &PaneName, _argv: &Argv) -> Result<(), PaneError> {
        Err(self.stub())
    }

    fn stop_owned(&self, _name: &PaneName) -> Result<(), PaneError> {
        Err(self.stub())
    }

    fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
        Err(self.stub())
    }
}
