//! `HerdrAdapter`: the `HerdrPort` over a [`Transport`] (Herdr's local socket in
//! production).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use holler_pane::{HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId};

use crate::plan::Extent;
use crate::transport::{Transport, UnixSocketTransport};

/// I5's bound for one `HerdrPort` call.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// What the adapter is told (Herdr has no grid and no discoverable socket of Holler's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrConfig {
    /// The one Herdr session served.
    pub session: String,
    /// That session's socket, absolute.
    pub socket: PathBuf,
    /// Workspace label to rows by columns.
    pub workspaces: BTreeMap<String, Extent>,
    /// The bound of one `HerdrPort` call.
    pub timeout: Duration,
}

impl HerdrConfig {
    /// A config with no workspace and [`DEFAULT_TIMEOUT`].
    pub fn new(session: impl Into<String>, socket: impl Into<PathBuf>) -> Self {
        Self {
            session: session.into(),
            socket: socket.into(),
            workspaces: BTreeMap::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// This config, with the workspace `label` of `extent`.
    pub fn with_workspace(mut self, label: impl Into<String>, extent: Extent) -> Self {
        self.workspaces.insert(label.into(), extent);
        self
    }
}

/// The `HerdrPort` over `T`.
pub struct HerdrAdapter<T = UnixSocketTransport> {
    config: HerdrConfig,
    #[allow(dead_code)] // #640 stub (#640 part 2): F fills, and removes this allow
    transport: T,
}

impl HerdrAdapter<UnixSocketTransport> {
    /// Connect to the socket of `config`.
    pub fn connect(_config: HerdrConfig) -> Result<Self, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }
}

impl<T: Transport> HerdrAdapter<T> {
    /// Connect over `transport`.
    pub fn connect_with(_config: HerdrConfig, _transport: T) -> Result<Self, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    /// The config it was connected with.
    pub fn config(&self) -> &HerdrConfig {
        &self.config
    }
}

impl<T: Transport> HerdrPort for HerdrAdapter<T> {
    fn ensure_pane(&self, _spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn send_text(&self, _pane: &PaneId, _text: &str) -> Result<(), PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn send_keys(&self, _pane: &PaneId, _keys: &[Key]) -> Result<(), PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn read(&self, _pane: &PaneId, _max_lines: usize) -> Result<String, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn close(&self, _pane: &PaneId) -> Result<(), PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }

    fn version(&self) -> Result<String, PaneError> {
        // stub (#640 part 2): F fills
        Err(PaneError::NotImplemented)
    }
}
