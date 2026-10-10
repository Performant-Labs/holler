//! The Herdr display plugin's library (issue #651, epic #633): read the hub's
//! pane registry and show it in Herdr's sidebar. **Display only** — the crate's
//! sources and its manifest may name no Herdr method beyond
//! `pane.report_metadata`, `workspace.report_metadata` and the read-only set
//! (`ping`, `session.snapshot`, `pane.get`, `pane.list`, `pane.read`,
//! `events.subscribe`); `tests/display_only_allowlist.rs` fails the crate if
//! they do. Every action the manifest offers runs a `holler pane` verb; the
//! plugin itself never changes a pane, a session or a layout.
//!
//! The T-red skeleton carries only the item signatures the tests compile
//! against; the bodies are stubs that refuse, and the F phase fills them in.

use std::path::PathBuf;

use holler_pane::{Pane, PaneName};

/// The two peers one refresh talks to, resolved from the environment a Herdr
/// plugin process receives.
#[derive(Debug, Clone)]
pub struct Endpoints {
    /// Herdr's session socket: the `HERDR_SOCKET_PATH` the plugin process is
    /// given.
    pub herdr_socket: PathBuf,
    /// The hub's control socket: `<HOLLER_STATE_DIR>/hub/control.sock`, the
    /// same resolution `holler_hub::control::run` makes (the hub read honours
    /// the ambient `HOLLER_STATE_DIR`).
    pub hub_control_socket: PathBuf,
}

impl Endpoints {
    /// Resolve both endpoints from the environment. `Err(PlugError::Env)`
    /// naming the missing variable when either is absent.
    pub fn from_env() -> Result<Self, PlugError> {
        // TODO(#651): read `HERDR_SOCKET_PATH` and `HOLLER_STATE_DIR`.
        Err(PlugError::Env("endpoints are not resolved yet".to_owned()))
    }
}

/// Runs refreshes, remembering which panes it last reported so a failed hub
/// read re-reports exactly those as `unknown` — never the previous facts again.
pub struct Reporter {
    endpoints: Endpoints,
}

impl Reporter {
    /// A reporter talking to `endpoints`.
    pub fn new(endpoints: Endpoints) -> Self {
        Self { endpoints }
    }

    /// One refresh: read `pane/list` from the hub's control socket, then send
    /// one `pane.report_metadata` per registry pane addressed by its
    /// `herdr.pane_id` (tokens `pos`, `project`, `shown`, `driven`, `sync`,
    /// `hold`; `ttl_ms` set) and one `workspace.report_metadata` per mapped
    /// workspace (token `profile`; **no** `ttl_ms`), all under the source id
    /// `holler`. When the hub read fails, send for every pane shown last time
    /// a `pane.report_metadata` with `state_labels: ["unknown"]`, every token
    /// `unknown`, and `ttl_ms` — and nothing else.
    pub fn refresh(&mut self) -> Result<Refreshed, PlugError> {
        // TODO(#651): the refresh loop — hub read, derive, report, remember.
        let _ = &self.endpoints;
        Err(PlugError::Herdr(
            "the refresh loop is not implemented yet".to_owned(),
        ))
    }
}

/// What one refresh did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refreshed {
    /// `pane.report_metadata` reports sent.
    pub pane_reports: usize,
    /// `workspace.report_metadata` reports sent.
    pub workspace_reports: usize,
    /// The hub read failed; the panes shown last time were re-reported
    /// `unknown`.
    pub unknown: bool,
}

/// Why a refresh could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlugError {
    /// An endpoint could not be resolved from the environment.
    Env(String),
    /// The Herdr report exchange failed (socket, framing or reply).
    Herdr(String),
}

/// The registry pane a Herdr action's `HERDR_PANE_ID` addresses: the record
/// whose `herdr.pane_id` equals `herdr_pane_id`, returned by pane name so the
/// action runs a `holler pane` verb on it. `None` when no record matches.
pub fn resolve_action_pane(panes: &[Pane], herdr_pane_id: &str) -> Option<PaneName> {
    // TODO(#651): resolve through the registry's herdr pane ids.
    let _ = (panes, herdr_pane_id);
    None
}
