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

use crate::argv::{AgentKey, Argv, EnvVarName};
use crate::error::{deserialize_parsed, excerpt, PaneError};
use crate::grid::GridPos;
use crate::ports::{Cursor, HarnessEndpoint};
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
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists, and
/// what `HerdrPort::snapshot` lists; a pane Herdr reports also carries its label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
    /// The pane's label as Herdr reports it, read by the Herdr adapter through its
    /// `label_of`: an observation (I6), and the join key from a pane name to a live Herdr
    /// pane (#808). `launch` and `relaunch` never write one, so a record's `herdr.label`
    /// is `None` and its JSON has no `label` key. `None` stands both for a pane Herdr
    /// sent no label for and for one it sent an empty label for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
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
    /// The Herdr API version, as `HerdrPort::version()` reports it, recorded by a verb
    /// (the Herdr adapter writes no record; ADR-0021 section 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub herdr_api_version: Option<String>,
}

/// The harness a pane runs: OpenCode, or Claude Code (#831, ADR-0021 section 15). The
/// `HarnessPort` is the seam both are reached through.
///
/// This is a closed enum on purpose (epic #633: a stored record with an unknown harness
/// fails to load rather than being driven blindly). It is a second list beside
/// `holler_proto::vocab::HARNESS_IDS`; every serde name of this enum must be in that list,
/// which a test pins. Adding a harness to panes adds a variant here and goes through the
/// epic's amend-first rule.
///
/// The facts about a kind are its methods, the one place they live (decision 25):
/// [`HarnessKind::shape`], [`HarnessKind::abort_style`] and
/// [`HarnessKind::login_variables`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessKind {
    /// OpenCode: a harness server on a port, and a TUI attached to one of its sessions.
    Opencode,
    /// Claude Code's interactive CLI (#831): one process that is the harness, the TUI and
    /// the session holder at once. Not the ACP body harness of the same id (ADR 0012),
    /// which only `holler-body` reads.
    Claude,
}

/// How a harness of a kind is built ([`HarnessKind::shape`], #831).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A server on a port, and a TUI that attaches to and switches between its sessions
    /// (OpenCode).
    ServerAndTui,
    /// One process that holds one session; a switch, a reset or an interrupt is a
    /// relaunch of it (Claude Code).
    SingleProcess,
}

/// How the running turn of a harness of a kind is stopped ([`HarnessKind::abort_style`],
/// #831).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortStyle {
    /// Through the harness API, in place, the session and its process kept (OpenCode).
    InPlace,
    /// By stopping the process and relaunching it on the session of record: the turn in
    /// flight is lost and the session kept (Claude Code; the operator's decision of
    /// 2026-10-11: no keypress, no in-place interrupt).
    Relaunch,
}

/// The environment variables that silently change a Claude Code login (ADR-0021 section 15
/// item 10): each is unset on every Claude launch argv, and a spec that names one is
/// refused (`claude-login-env-refused`). Names only, never a value (I7).
pub const CLAUDE_LOGIN_VARIABLES: [&str; 7] = [
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

impl HarnessKind {
    /// The kind's name: its serde name (`opencode`, `claude`), as a message or a line of text
    /// output names it.
    pub const fn as_str(self) -> &'static str {
        match self {
            HarnessKind::Opencode => "opencode",
            HarnessKind::Claude => "claude",
        }
    }

    /// How a harness of this kind is built: [`Shape::ServerAndTui`] for `opencode`,
    /// [`Shape::SingleProcess`] for `claude`. The engines choose their steps by it.
    pub const fn shape(self) -> Shape {
        match self {
            HarnessKind::Opencode => Shape::ServerAndTui,
            HarnessKind::Claude => Shape::SingleProcess,
        }
    }

    /// How the running turn of a harness of this kind is stopped:
    /// [`AbortStyle::InPlace`] for `opencode`, [`AbortStyle::Relaunch`] for `claude`.
    pub const fn abort_style(self) -> AbortStyle {
        match self {
            HarnessKind::Opencode => AbortStyle::InPlace,
            HarnessKind::Claude => AbortStyle::Relaunch,
        }
    }

    /// The environment variable names that change the login of a harness of this kind,
    /// which its pane's spec may not name: [`CLAUDE_LOGIN_VARIABLES`] for `claude`, none for
    /// `opencode` (its provider credential reaches the server through the tmux server's
    /// environment, ADR-0021 section 2).
    pub const fn login_variables(self) -> &'static [&'static str] {
        match self {
            HarnessKind::Opencode => &[],
            HarnessKind::Claude => &CLAUDE_LOGIN_VARIABLES,
        }
    }
}

/// The command name of a Claude Code CLI process, as tmux reports a pane's foreground
/// process (`HostPort::foreground_commands`, #835). It is a process name, not the protocol's
/// harness id: it is not taken from `holler_proto::vocab::HARNESS_IDS`, though the two are
/// spelled alike.
///
/// The one place the name is written: [`is_claude_code_command`] is the one rule that uses it,
/// for the import (#835) and the stories that adopt or show such a pane (#831, #832).
pub const CLAUDE_CODE_COMMAND: &str = "claude";

/// Whether `name`, a command name as `HostPort::foreground_commands` reports it, is Claude
/// Code's: exactly [`CLAUDE_CODE_COMMAND`], with no case folding, trimming or path. No other
/// name is classified.
///
/// The rule's limits (#835, ADR-0021 section 13): tmux names a pane's foreground process by
/// its `argv[0]`, so a Claude Code pane whose `claude` has exited shows its shell, and one
/// started under another `argv[0]` (a wrapper, or `node` running its script) is not
/// recognised. It is a default for an import, not a proof of what a pane runs.
pub fn is_claude_code_command(name: &str) -> bool {
    name == CLAUDE_CODE_COMMAND
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

/// The harness of a pane: its kind, where it is reached, and its last observed health.
///
/// An `opencode` harness is reached on its server's `port`; a `claude` harness is one process,
/// identified by its `pid`, and has no port (#831, ADR-0021 section 15 item 3). A record is
/// valid only as one of the two, so a stored record that is neither does not load
/// (`deny_unknown_fields` and the fail-closed registry, section 7). An `opencode` record may
/// still carry the `pid` the launcher wrote before #802, and reads and writes the same bytes
/// as before the port became optional. [`HarnessInfo::endpoint`] is the one place a record
/// becomes a [`HarnessEndpoint`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawHarnessInfo")]
pub struct HarnessInfo {
    pub kind: HarnessKind,
    /// The harness server's port: present exactly for an `opencode` harness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// The harness process's pid: required for a `claude` harness, whose identity it is (it
    /// survives a person's `/clear`); an `opencode` harness records none since #802.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub health: Health,
}

/// The serde form of a [`HarnessInfo`] before its fields are checked against its kind.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHarnessInfo {
    kind: HarnessKind,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    pid: Option<u32>,
    health: Health,
}

impl TryFrom<RawHarnessInfo> for HarnessInfo {
    type Error = String;

    fn try_from(raw: RawHarnessInfo) -> Result<Self, String> {
        let info = HarnessInfo {
            kind: raw.kind,
            port: raw.port,
            pid: raw.pid,
            health: raw.health,
        };
        match info.misfit() {
            Some(why) => Err(why.to_owned()),
            None => Ok(info),
        }
    }
}

impl HarnessInfo {
    /// Where the harness of the pane `pane` is reached: the server's port for an `opencode`
    /// harness, the pane and the process's pid for a `claude` one. A value that does not fit
    /// its kind (built in code; a stored one does not load) is `store-corrupt`, and no
    /// endpoint is guessed.
    pub fn endpoint(&self, pane: &PaneName) -> Result<HarnessEndpoint, PaneError> {
        match (self.kind.shape(), self.port, self.pid) {
            (Shape::ServerAndTui, Some(port), _) => Ok(HarnessEndpoint::Port(port)),
            (Shape::SingleProcess, None, Some(pid)) => Ok(HarnessEndpoint::Process {
                pane: pane.clone(),
                pid,
            }),
            _ => {
                let why = self.misfit().unwrap_or("it does not fit its kind");
                Err(PaneError::StoreCorrupt {
                    what: format!("the harness of {pane}: {why}"),
                })
            }
        }
    }

    /// Why this harness does not fit its kind, or `None` when it does: an `opencode` harness
    /// has a port; a `claude` harness has a pid and no port.
    fn misfit(&self) -> Option<&'static str> {
        match (self.kind.shape(), self.port, self.pid) {
            (Shape::ServerAndTui, Some(_), _) | (Shape::SingleProcess, None, Some(_)) => None,
            (Shape::ServerAndTui, None, _) => Some("an opencode harness needs its server's port"),
            (Shape::SingleProcess, Some(_), _) => {
                Some("a claude harness is one process and has no port")
            }
            (Shape::SingleProcess, None, None) => Some("a claude harness needs its process's pid"),
        }
    }
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
    /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
    /// server's default agent (#700). A name, never a secret (I7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_agent: Option<AgentKey>,
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
