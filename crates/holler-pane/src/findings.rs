//! The finding kinds reconcile and `holler pane doctor` report. Empty stub declared by
//! #637 so that no two stories edit `lib.rs`; story #647 fills it, and #665 adds only
//! the `ProfileDrift` kind after #647 has merged.
//!
//! RED scaffold (#647 test plan): the public types of the brief's Decision 10, landed by
//! the tester so the RED suite compiles. F fills the remedy builder and the sanitizer.

use serde::Serialize;

use crate::{GridPos, PaneId, PaneName};

/// What reconcile found about a pane, a server or Herdr. A finding is a report, not an
/// error: its code is never a `RefusalCode` (two of them equal closed error codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    HerdrVersionUnsupported,
    ObserveFailed,
    UnregisteredHerdrPane,
    HerdrPaneMissing,
    TmuxSessionMissing,
    ServerWedged,
    ServerDown,
    NoSessionOfRecord,
    SessionOfRecordMissing,
    TuiForeignSession,
    ShownDrivenMismatch,
    StraySession,
}

impl FindingKind {
    /// Every kind, in declaration order.
    pub const ALL: &'static [FindingKind] = &[
        FindingKind::HerdrVersionUnsupported,
        FindingKind::ObserveFailed,
        FindingKind::UnregisteredHerdrPane,
        FindingKind::HerdrPaneMissing,
        FindingKind::TmuxSessionMissing,
        FindingKind::ServerWedged,
        FindingKind::ServerDown,
        FindingKind::NoSessionOfRecord,
        FindingKind::SessionOfRecordMissing,
        FindingKind::TuiForeignSession,
        FindingKind::ShownDrivenMismatch,
        FindingKind::StraySession,
    ];

    /// The kind's stable kebab-case code.
    pub const fn code(self) -> &'static str {
        match self {
            FindingKind::HerdrVersionUnsupported => "herdr-version-unsupported",
            FindingKind::ObserveFailed => "observe-failed",
            FindingKind::UnregisteredHerdrPane => "unregistered-herdr-pane",
            FindingKind::HerdrPaneMissing => "herdr-pane-missing",
            FindingKind::TmuxSessionMissing => "tmux-session-missing",
            FindingKind::ServerWedged => "server-wedged",
            FindingKind::ServerDown => "server-down",
            FindingKind::NoSessionOfRecord => "no-session-of-record",
            FindingKind::SessionOfRecordMissing => "session-of-record-missing",
            FindingKind::TuiForeignSession => "tui-foreign-session",
            FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
            FindingKind::StraySession => "stray-session",
        }
    }
}

/// What `--fix` did, or could do, about a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixState {
    NotFixable,
    Fixable,
    Skipped,
    Fixed,
    Failed,
}

/// Why a fix failed: the error's code and message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixError {
    pub code: String,
    pub message: String,
}

/// One finding of a reconcile pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub pane: Option<PaneName>,
    pub grid: Option<GridPos>,
    pub herdr_pane: Option<PaneId>,
    pub session: Option<String>,
    pub ports: Vec<u16>,
    pub message: String,
    pub remedy: Option<String>,
    pub fix: FixState,
    pub fix_error: Option<FixError>,
}
