//! The reconcile engine: observe Herdr, tmux and the harness, compare with the
//! registry and report; it never infers state from files (I6). Empty stub declared by
//! #637 so that no two stories edit `lib.rs`; story #647 fills it.
//!
//! RED scaffold (#647 test plan): the public API of the brief's Decision 10, landed by
//! the tester so the RED suite compiles. `reconcile` answers `not-implemented` until F
//! fills it.

use serde::Serialize;

use crate::findings::Finding;
use crate::{GridPos, PaneError, PaneName, Ports, ProfileName};

/// What one reconcile pass is asked to do.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileRequest<'a> {
    /// Scope the pass to this profile's panes.
    pub profile: Option<&'a ProfileName>,
    /// Scope the pass to this one pane.
    pub pane: Option<&'a PaneName>,
    /// Repair what the record decides.
    pub fix: bool,
    /// The clock: milliseconds since the Unix epoch.
    pub now_ms: i64,
}

/// What one reconcile pass found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub scope: ScopeSummary,
    pub fix_requested: bool,
    pub herdr: HerdrSummary,
    pub hosts: Vec<HostSummary>,
    pub panes: Vec<PaneSummary>,
    pub findings: Vec<Finding>,
}

/// The scope the pass ran over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScopeSummary {
    pub profile: Option<ProfileName>,
    pub pane: Option<PaneName>,
}

/// What Herdr reported about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HerdrSummary {
    pub version: Option<String>,
}

/// One host of the panes in scope, as their records name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostSummary {
    pub name: String,
    pub herdr_api_version: Option<String>,
}

/// One pane in scope, as the pass observed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneSummary {
    pub name: PaneName,
    pub grid: GridPos,
    pub session_of_record: Option<String>,
    pub shown: Option<String>,
    pub health: ObservedHealth,
}

/// The harness server's health as this pass observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservedHealth {
    Healthy,
    Wedged,
    Down,
    Unknown,
}

/// Run one reconcile pass. RED scaffold: answers `not-implemented`.
pub fn reconcile(_ports: Ports<'_>, _request: &ReconcileRequest<'_>) -> Result<Report, PaneError> {
    Err(PaneError::NotImplemented)
}
