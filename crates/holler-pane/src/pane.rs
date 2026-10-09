//! The pane record (epic #633, "The contract"): one record per pane, owned by the
//! hub, and its serde form.
//!
//! The JSON field names are the contract: #639 persists these forms, #649 reads them
//! over the control socket. The records refuse unknown fields (`deny_unknown_fields`):
//! a record that is read, changed and written back by a hub or a client that does not
//! know a field would otherwise drop that field silently; refusing is loud and
//! fails closed. Optional fields default to absent on read and are left out when
//! they are `None`. Every timestamp is milliseconds since the Unix epoch as `i64`.

use std::fmt;

use holler_proto::vocab::SessionName;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::argv::{Argv, EnvVarName};
use crate::error::{deserialize_parsed, excerpt, PaneError};
use crate::grid::GridPos;
use crate::ports::Cursor;
use crate::probe::ProbeResult;
use crate::profile::ProfileName;

/// The name of a pane, e.g. `hj-c1r1` (the tmux session name). It is a
/// `holler_proto::vocab::SessionName`: the ADR 0005 name grammar is reused, not
/// copied, so a pane name is exactly a valid session name. Serde goes through
/// [`PaneName::parse`]; an invalid name is `usage`.
///
/// Pane names such as `hj-c1r2` are names, not positions, and are not renamed.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneName(SessionName);

impl PaneName {
    /// Parse a pane name; the grammar is `SessionName::parse`'s.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        SessionName::parse(text)
            .map(Self)
            .map_err(|e| PaneError::Usage {
                message: format!("invalid pane name {}: {e}", excerpt(text)),
            })
    }

    /// The name, verbatim.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PaneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for PaneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PaneName({:?})", self.as_str())
    }
}

impl Serialize for PaneName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PaneName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_parsed(deserializer, PaneName::parse)
    }
}

/// Herdr's own identifier of a pane (opaque to Holler, e.g. `p_12`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneId(String);

impl PaneId {
    /// A pane id from Herdr's text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id, verbatim.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
}

/// The machine side of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInfo {
    /// The host's name.
    pub name: String,
    /// The tmux session name (equal to the pane's name).
    pub tmux: String,
    /// The project directory or worktree the pane works in.
    pub cwd: String,
    /// The Herdr API version, recorded by the Herdr adapter (#640) on connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub herdr_api_version: Option<String>,
}

/// The harness a pane runs. Only OpenCode exists; the `HarnessPort` is the seam for
/// another.
///
/// This is a closed enum on purpose (epic #633: no new harness; a stored record with
/// an unknown harness fails to load rather than being driven blindly). It is a second
/// list beside `holler_proto::vocab::HARNESS_IDS`; every serde name of this enum must
/// be in that list, which a test pins. Adding a harness to panes adds a variant here
/// and goes through the epic's amend-first rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessKind {
    Opencode,
}

/// What the harness server last reported about its health.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    /// Unhealthy, with the reason.
    Unhealthy(String),
    Unknown,
}

/// The harness server of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessInfo {
    pub kind: HarnessKind,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub health: Health,
}

/// The role of a pane (and of a profile spec): an agent or the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneRole {
    Agent,
    Orchestrator,
}

/// A pane's hold state **as a field of the pane record**: whether `park`/`unpark`
/// has parked it, or it is drained. It is **not** the prompt hold of `holler hold`
/// (`holler_proto::SessionHold`). Any refusal of a prompt that is derived from pane
/// state belongs at `send_prompt`, the one choke point every prompt passes through,
/// not in a verb (#646's brief states this).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Hold {
    None,
    Parked {
        reason: String,
        /// When the park ends, in the verb's own words (a time or a condition).
        release_when: String,
        /// When the pane was parked (milliseconds since the Unix epoch).
        since: i64,
    },
    Drained,
}

/// What reconcile last observed about which session the pane shows and which the hub
/// drives. Written by reconcile, never inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastObserved {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driven: Option<String>,
    /// When it was observed (milliseconds since the Unix epoch).
    pub at: i64,
}

/// The model a pane runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    pub provider: String,
    pub model_id: String,
    pub effort: String,
}

/// The context ceilings a watchdog reads (a local model's window is smaller than a
/// hosted one's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCeilings {
    pub soft: u32,
    pub hard: u32,
}

/// The health probe of a pane (B1): the stored `check` argv, the strings its output
/// must contain, and the last result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneProbe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Argv>,
    #[serde(default)]
    pub expect: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<ProbeResult>,
}

/// One record per pane, owned by the hub. The key is [`Pane::name`]. Every write is
/// a compare-and-swap on [`Pane::generation`] (see [`crate::generation`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub name: PaneName,
    /// Bumped on every change.
    pub generation: u64,
    pub herdr: HerdrPane,
    pub host: HostInfo,
    pub harness: HarnessInfo,
    /// THE session: the TUI is attached to it and the hub drives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_of_record: Option<String>,
    pub role: PaneRole,
    pub hold: Hold,
    pub last_observed: LastObserved,
    /// The profile the pane belongs to; a pane belongs to at most one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Recorded by launch/relaunch (#644) and import (#650), so
    /// `profile create --from-current` copies them from the store.
    pub model: ModelSpec,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
    pub env: Vec<EnvVarName>,
    pub context: ContextCeilings,
    /// The launch command: an argv array, never a shell string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Argv>,
    pub probe: PaneProbe,
}

/// One change to the pane store, as `PaneStore::watch` yields it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneEvent {
    /// The store-wide sequence number of this change; hand it back as the `since`
    /// of the next watch to resume without a gap or a repeat.
    pub cursor: Cursor,
    /// The pane the change concerns.
    pub name: PaneName,
    /// The record after the change; `None` when the record was deleted.
    #[serde(default)]
    pub pane: Option<Box<Pane>>,
}
