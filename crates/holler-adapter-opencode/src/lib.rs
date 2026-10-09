//! `holler_adapter_opencode` — the OpenCode adapter: it implements
//! `holler_pane::HarnessPort` (serve, health, sessions, abort, attach) for the
//! OpenCode harness (epic #633).
//!
//! stub (#642a): T's pinned signatures (docs/handoffs/642-brief.md, "The public API");
//! F fills every body and writes the crate docs.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};

pub mod http;
pub mod tui;

pub use tui::{TmuxConfig, TmuxSocket};

/// Maps a key to what the adapter needs; an `Err` is returned to the caller as is.
pub type Resolver<K, V> = Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

/// The environment a child process is started with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEnv {
    /// The child inherits the environment it is started with.
    Inherit,
    /// The child gets exactly these variables (`env -i` semantics).
    Isolated(Vec<(String, String)>),
}

/// The adapter's time bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub call: Duration,
    pub request: Duration,
    pub health: Duration,
    pub boot_try: Duration,
    pub settle: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        // stub (#642a): F fills
        Self {
            call: Duration::ZERO,
            request: Duration::ZERO,
            health: Duration::ZERO,
            boot_try: Duration::ZERO,
            settle: Duration::ZERO,
        }
    }
}

/// How the adapter reaches OpenCode and tmux.
#[derive(Clone)]
pub struct OpenCodeConfig {
    pub opencode_bin: PathBuf,
    pub serve_args: Vec<String>,
    pub env: ProcessEnv,
    pub tmux: TmuxConfig,
    pub workdir: Resolver<PaneName, PathBuf>,
    pub tui_session: Resolver<PaneId, PaneName>,
    pub timeouts: Timeouts,
}

/// The `HarnessPort` over OpenCode.
pub struct OpenCodeHarness {
    _config: OpenCodeConfig, // stub (#642a): F fills
}

impl OpenCodeHarness {
    pub fn new(config: OpenCodeConfig) -> Self {
        Self { _config: config }
    }
}

impl HarnessPort for OpenCodeHarness {
    fn serve(&self, _name: &PaneName, _port: u16) -> Result<u32, PaneError> {
        Err(PaneError::NotImplemented) // stub (#642a): F fills
    }
    fn health(&self, _port: u16) -> Result<bool, PaneError> {
        Err(PaneError::NotImplemented) // stub (#642a): F fills
    }
    fn create_session(&self, _port: u16) -> Result<String, PaneError> {
        Err(PaneError::NotImplemented) // stub (#642a): F fills
    }
    fn list_sessions(&self, _port: u16) -> Result<Vec<String>, PaneError> {
        Err(PaneError::NotImplemented) // stub (#642a): F fills
    }
    fn abort(&self, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented) // stub (#642a): F fills
    }
    fn attach_tui(&self, _pane: &PaneId, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }
    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented)
    }
    fn shown_session(&self, _pane: &PaneId) -> Result<Option<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }
}
