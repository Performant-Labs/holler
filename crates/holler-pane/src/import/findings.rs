//! The findings of `holler pane import` (#650): what the import found about a pane directory,
//! how much each weighs, and the one table of their codes.
//!
//! - [`ImportFinding`]: eighteen kinds, each with a stable kebab-case code that is also its
//!   JSON form ([`ImportFinding::code`] is the one source of both), copying
//!   [`FindingKind`]'s pattern. A condition reconcile already names keeps reconcile's
//!   spelling, taken from its one source and never retyped: `observe-failed`,
//!   `tmux-session-missing`, `server-wedged` and `server-down` from [`FindingKind::code`],
//!   and `session-not-found` and `command-not-argv` from the closed error codes. The other
//!   twelve are the import's own, two of them #835's (`harness-claude-code` and
//!   `harness-unreadable`). A code is stable once merged (ADR-0021 section 9).
//! - [`Level`]: a `problem` gives the pane no record and blocks a real run; a `warning` is
//!   recorded without what it names; `skipped` and `info` block nothing.
//! - [`Finding`]: one finding, about a pane or about no pane, with a one-line message.
//!
//! `herdr-version-unsupported` is not a finding of the import: it is the error
//! `herdr.version()` answers, and it ends the run with no report.
//!
//! **Messages.** Untrusted text (an id Herdr or a harness reports, an adapter's message, a
//! directory's name) reaches a message only through `crate::findings::quoted` or
//! `embedded`. A `source-unreadable` message never carries text read from a file or from a
//! parser's error: it names the file and a position only (`holler-cli` builds those).

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::error::PaneCode;
use crate::findings::FindingKind;
use crate::PaneName;

/// How much a finding weighs. Only a [`Level::Problem`] blocks a real run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// The pane gets no record, and a real run writes nothing.
    Problem,
    /// The pane is recorded without what the finding names.
    Warning,
    /// The pane is left out and blocks nothing (`harness-not-opencode`).
    Skipped,
    /// Information only.
    Info,
}

impl Level {
    /// The level's name, which is also its JSON form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Problem => "problem",
            Level::Warning => "warning",
            Level::Skipped => "skipped",
            Level::Info => "info",
        }
    }
}

impl Serialize for Level {
    /// The level's [`as_str`](Level::as_str).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What the import found about a pane directory, in the order a pane's findings are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImportFinding {
    /// The pane already has a record; it is skipped before any other check.
    AlreadyImported,
    /// Its `sessions.toml` names a harness that is not OpenCode: no record, never blocking.
    HarnessNotOpencode,
    /// Its tmux session runs Claude Code (#835): the pane is left alone, with no record and no
    /// other finding, and blocks nothing. Its level is `info`, not `skipped` as
    /// `harness-not-opencode`'s is: a switch adopts these panes once Claude Code can be driven
    /// (#831), while nothing adopts a pane that is not OpenCode.
    HarnessClaudeCode,
    /// What the pane's tmux session runs could not be read (#835): a read failed, or the
    /// pane's check did not finish. The pane is never recorded as OpenCode on a guess.
    HarnessUnreadable,
    /// A file could not be read or parsed; the message names the file and a position only.
    SourceUnreadable,
    /// A file, a required variable or a launcher entry (or its port or cwd) is missing.
    SourceMissing,
    /// No Herdr pane in the run's Herdr session carries the pane's name as its label.
    PaneUnlabelled,
    /// Two or more Herdr panes in the run's Herdr session carry the pane's name as their label.
    PaneLabelAmbiguous,
    /// The name spells a cell other than the one Herdr shows the pane at; the record takes
    /// Herdr's.
    NameCellDiffers,
    /// The pane's tmux session does not exist (reconcile's code).
    TmuxSessionMissing,
    /// A port call returned an error, so nothing was concluded from it (reconcile's code).
    ObserveFailed,
    /// The pane's harness server accepts a connection and never answers (reconcile's code).
    ServerWedged,
    /// The pane's harness server fails its health check, and is not wedged (reconcile's code).
    ServerDown,
    /// `last-session` names a session that `sessions.toml` or `owned-sessions` does not.
    SourcesDisagree,
    /// `last-session` is absent or empty.
    NoSessionRecorded,
    /// The files agree on a session that the pane's healthy server does not have (the closed
    /// error code).
    SessionNotFound,
    /// The launcher table gives the command as a string (the closed error code).
    CommandNotArgv,
    /// A directory of the state root that holds none of the pane files.
    IgnoredEntry,
}

impl ImportFinding {
    /// Every kind, in declaration order.
    pub const ALL: &'static [ImportFinding] = &[
        ImportFinding::AlreadyImported,
        ImportFinding::HarnessNotOpencode,
        ImportFinding::HarnessClaudeCode,
        ImportFinding::HarnessUnreadable,
        ImportFinding::SourceUnreadable,
        ImportFinding::SourceMissing,
        ImportFinding::PaneUnlabelled,
        ImportFinding::PaneLabelAmbiguous,
        ImportFinding::NameCellDiffers,
        ImportFinding::TmuxSessionMissing,
        ImportFinding::ObserveFailed,
        ImportFinding::ServerWedged,
        ImportFinding::ServerDown,
        ImportFinding::SourcesDisagree,
        ImportFinding::NoSessionRecorded,
        ImportFinding::SessionNotFound,
        ImportFinding::CommandNotArgv,
        ImportFinding::IgnoredEntry,
    ];

    /// The kind's stable kebab-case code, which is also its JSON form.
    pub const fn code(self) -> &'static str {
        match self {
            ImportFinding::AlreadyImported => "already-imported",
            ImportFinding::HarnessNotOpencode => "harness-not-opencode",
            ImportFinding::HarnessClaudeCode => "harness-claude-code",
            ImportFinding::HarnessUnreadable => "harness-unreadable",
            ImportFinding::SourceUnreadable => "source-unreadable",
            ImportFinding::SourceMissing => "source-missing",
            ImportFinding::PaneUnlabelled => "pane-unlabelled",
            ImportFinding::PaneLabelAmbiguous => "pane-label-ambiguous",
            ImportFinding::NameCellDiffers => "name-cell-differs",
            ImportFinding::TmuxSessionMissing => FindingKind::TmuxSessionMissing.code(),
            ImportFinding::ObserveFailed => FindingKind::ObserveFailed.code(),
            ImportFinding::ServerWedged => FindingKind::ServerWedged.code(),
            ImportFinding::ServerDown => FindingKind::ServerDown.code(),
            ImportFinding::SourcesDisagree => "sources-disagree",
            ImportFinding::NoSessionRecorded => "no-session-recorded",
            ImportFinding::SessionNotFound => PaneCode::SessionNotFound.as_str(),
            ImportFinding::CommandNotArgv => PaneCode::CommandNotArgv.as_str(),
            ImportFinding::IgnoredEntry => "ignored-entry",
        }
    }

    /// How much a finding of this kind weighs (only a problem blocks).
    pub const fn level(self) -> Level {
        match self {
            ImportFinding::HarnessUnreadable
            | ImportFinding::SourceUnreadable
            | ImportFinding::SourceMissing
            | ImportFinding::PaneUnlabelled
            | ImportFinding::PaneLabelAmbiguous
            | ImportFinding::TmuxSessionMissing
            | ImportFinding::SourcesDisagree => Level::Problem,
            ImportFinding::ObserveFailed
            | ImportFinding::ServerWedged
            | ImportFinding::ServerDown
            | ImportFinding::NoSessionRecorded
            | ImportFinding::SessionNotFound
            | ImportFinding::CommandNotArgv => Level::Warning,
            ImportFinding::HarnessNotOpencode => Level::Skipped,
            ImportFinding::AlreadyImported
            | ImportFinding::HarnessClaudeCode
            | ImportFinding::NameCellDiffers
            | ImportFinding::IgnoredEntry => Level::Info,
        }
    }
}

impl Serialize for ImportFinding {
    /// The kind's [`code`](ImportFinding::code), so the JSON form cannot drift from it.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

/// One finding of an import's report.
///
/// Its JSON is `{"pane", "level", "kind", "message"}`, in that order: `pane` is null for a
/// finding about no pane, and `level` is always its kind's, so the two cannot disagree. The
/// key is `kind`, as in `holler pane doctor`'s report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The pane it is about; `None` for a directory that is not a pane directory, or whose
    /// name is not a pane name.
    pub pane: Option<PaneName>,
    pub kind: ImportFinding,
    /// One line for a person; untrusted values in it are quoted.
    pub message: String,
}

impl Finding {
    /// A finding of `kind` about the pane `pane`.
    pub fn about(pane: &PaneName, kind: ImportFinding, message: String) -> Self {
        Self {
            pane: Some(pane.clone()),
            kind,
            message,
        }
    }

    /// A finding of `kind` about no pane.
    pub fn unattached(kind: ImportFinding, message: String) -> Self {
        Self {
            pane: None,
            kind,
            message,
        }
    }

    /// The finding's level: its kind's.
    pub fn level(&self) -> Level {
        self.kind.level()
    }
}

impl Serialize for Finding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("Finding", 4)?;
        out.serialize_field("pane", &self.pane)?;
        out.serialize_field("level", &self.level())?;
        out.serialize_field("kind", &self.kind)?;
        out.serialize_field("message", &self.message)?;
        out.end()
    }
}
