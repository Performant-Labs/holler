//! What the old launcher's files say about one pane (#650), as `holler-cli` read them.
//!
//! This crate knows no file format (ADR-0021 section 5): the CLI parses `pane.env`, the
//! launcher table, `last-session`, `sessions.toml` and `owned-sessions`, and hands the engine
//! these values together with the problems it met reading them (`source-unreadable` and
//! `source-missing` findings). Every value here is plain data; none is a secret, since
//! `pane.env` is read for five values and the names of the rest (I7).

use serde_json::Value;

use crate::pane::{ContextCeilings, HarnessKind, ModelSpec};
use crate::{AgentKey, Argv, EnvVarName, PaneName};

use super::findings::Finding;

/// One pane directory's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneFiles {
    /// The directory's name, which is the record's key and nothing more.
    pub name: PaneName,
    /// What `pane.env` gives; `None` when it is missing, or could not be read whole (a
    /// problem in [`PaneFiles::problems`] says which).
    pub env: Option<EnvValues>,
    /// The pane's entry in the launcher table; `None` when it, or its port or cwd, is
    /// missing, or the table could not be read (a problem says which).
    pub launcher: Option<LauncherEntry>,
    /// The three files that name a session.
    pub sessions: SessionFiles,
    /// The problems met reading the files, each a `source-unreadable` or `source-missing`
    /// finding about this pane.
    pub problems: Vec<Finding>,
}

/// What `pane.env` gives a record: five values, and the names of every other variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvValues {
    /// `OC_MODEL` (`PROVIDER/MODEL_ID`, split at the first `/`) and `OC_EFFORT`.
    pub model: ModelSpec,
    /// `OC_CONTEXT_SOFT` and `OC_CONTEXT_HARD`.
    pub context: ContextCeilings,
    /// `OC_DEFAULT_AGENT`, when the file sets one.
    pub opencode_agent: Option<AgentKey>,
    /// Every other variable, by name only, in file order and de-duplicated.
    pub env: Vec<EnvVarName>,
}

/// One pane's entry in the launcher table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherEntry {
    pub port: u16,
    /// An absolute path.
    pub cwd: String,
    pub command: LauncherCommand,
}

/// The `command` of a launcher entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherCommand {
    /// The entry has none.
    Absent,
    /// An array of strings: the pane's command.
    Argv(Argv),
    /// A string: a shell command, which a record never holds (`command-not-argv`).
    Shell,
}

/// What a file held, when the pane directory may or may not have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState<T> {
    /// The pane directory has no such file (for `last-session`, also an empty one).
    Absent,
    /// The file and what it holds.
    Present(T),
    /// The file exists and could not be read; a problem says why.
    Unreadable,
}

/// The three files that name a pane's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFiles {
    /// `last-session`: its whole content, trimmed.
    pub last: FileState<String>,
    /// `sessions.toml`, read through `holler_body::config`.
    pub table: FileState<SessionsTable>,
    /// `owned-sessions`: one session id per line, blank and `#` lines skipped.
    pub owned: FileState<Vec<String>>,
}

/// What `sessions.toml` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsTable {
    /// Every row's `session_id`, in file order (a row without one adds nothing).
    pub session_ids: Vec<String>,
    /// Every row's `harness`, as written.
    pub harnesses: Vec<String>,
}

impl SessionFiles {
    /// Whether `sessions.toml` marks the pane as not an OpenCode pane: a row whose harness is
    /// not [`HarnessKind::Opencode`], the one harness the import records. A `claude` row, a
    /// harness a record can hold since #831, is not an OpenCode pane either: the import never
    /// records one as OpenCode (plan review W7).
    pub(super) fn not_opencode(&self) -> bool {
        match &self.table {
            FileState::Present(table) => table
                .harnesses
                .iter()
                .any(|h| harness_kind(h) != Some(HarnessKind::Opencode)),
            FileState::Absent | FileState::Unreadable => false,
        }
    }
}

/// The [`HarnessKind`] whose serde name is `harness`, if there is one.
fn harness_kind(harness: &str) -> Option<HarnessKind> {
    serde_json::from_value(Value::String(harness.to_owned())).ok()
}
