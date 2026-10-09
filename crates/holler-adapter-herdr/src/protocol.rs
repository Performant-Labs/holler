//! Herdr's wire, with no I/O: requests, reply decoding, the parsers and the version gate.
//!
//! stub (#640 part 1): T-red holds only the pinned signatures and constants. F fills the
//! bodies.

use holler_pane::{Key, PaneError, PaneId};
use serde_json::Value;

use crate::layout::{Direction, LayoutNode};

/// The protocol versions the adapter supports.
pub const SUPPORTED_PROTOCOLS: [u32; 1] = [22];
/// What a refusal names as supported.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
/// Every method the adapter may call, and no other.
pub const ALLOWED_METHODS: [&str; 9] = [
    "ping",
    "session.snapshot",
    "layout.export",
    "workspace.create",
    "pane.split",
    "pane.send_text",
    "pane.send_keys",
    "pane.read",
    "pane.close",
];

/// One request to Herdr.
// stub (#640 part 1): F replaces the derived `Debug` with a hand-written one that never
// prints `SendText`'s text (A, W-2).
#[derive(Debug, Clone)]
pub enum Request {
    Ping,
    SessionSnapshot,
    LayoutExport {
        tab_id: String,
    },
    WorkspaceCreate {
        label: String,
    },
    Split {
        target: PaneId,
        direction: Direction,
        ratio: f64,
    },
    SendText {
        pane: PaneId,
        text: String,
    },
    SendKeys {
        pane: PaneId,
        keys: Vec<Key>,
    },
    Read {
        pane: PaneId,
        lines: u32,
    },
    Close {
        pane: PaneId,
    },
}

impl Request {
    /// Herdr's method name.
    pub fn method(&self) -> &'static str {
        "" // stub (#640 part 1): F fills
    }

    /// The request id: `holler:<method>`.
    pub fn id(&self) -> String {
        String::new() // stub (#640 part 1): F fills
    }

    /// One JSON object and exactly one trailing newline.
    pub fn to_line(&self) -> String {
        String::new() // stub (#640 part 1): F fills
    }
}

/// The `result` object of a reply to `request`, or the error it maps to.
pub fn decode_reply(_request: &Request, _line: &str) -> Result<Value, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// What `ping` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerVersion {
    pub version: String,
    pub protocol: Option<u32>,
}

/// Read a `pong` result.
pub fn parse_pong(_result: &Value) -> Result<ServerVersion, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Accept exactly the supported protocols; refuse the rest with `herdr-version-unsupported`.
pub fn check_supported(_server: &ServerVersion) -> Result<(), PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// A Herdr workspace, by label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRef {
    pub workspace_id: String,
    pub label: String,
    pub grid_tab: Option<String>,
}

/// A pane of a Herdr snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneRef {
    pub pane_id: PaneId,
    pub workspace_id: String,
    pub tab_id: String,
}

/// What `session.snapshot` reports, reduced to what the adapter reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub workspaces: Vec<WorkspaceRef>,
    pub panes: Vec<PaneRef>,
}

impl SessionState {
    /// The workspace labelled `label`; two with one label are `unavailable`.
    pub fn workspace(&self, _label: &str) -> Result<Option<&WorkspaceRef>, PaneError> {
        Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
    }
}

/// Read a `session_snapshot` result.
pub fn parse_snapshot(_result: &Value) -> Result<SessionState, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Read a `layout_export` result into its split tree.
pub fn parse_layout_export(_result: &Value) -> Result<LayoutNode, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Read a `workspace_created` result: the workspace and its root pane.
pub fn parse_workspace_created(_result: &Value) -> Result<(WorkspaceRef, PaneId), PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Read a `pane_info` result (what `pane.split` returns).
pub fn parse_pane_info(_result: &Value) -> Result<PaneId, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Read a `pane_read` result: its last `max_lines` lines.
pub fn parse_read(_result: &Value, _max_lines: usize) -> Result<String, PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}

/// Accept only `type: "ok"`.
pub fn expect_ok(_result: &Value) -> Result<(), PaneError> {
    Err(PaneError::NotImplemented) // stub (#640 part 1): F fills
}
