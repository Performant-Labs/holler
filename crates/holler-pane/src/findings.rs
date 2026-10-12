//! The findings of a reconcile pass and `holler pane doctor` (#647): what a pass found, the
//! one table of what each kind tells the operator to run, and the one sanitizer for the
//! untrusted text a finding quotes. #647 part 2 added the `Unattached` and
//! `AgentCannotDispatch` kinds, last.
//!
//! - [`FindingKind`]: eighteen kinds, each with a stable kebab-case code that is also its JSON
//!   form ([`FindingKind::code`] is the one source of both). #832 added `unmanaged-stalled`,
//!   #831 `pane-waiting` and `claude-login-missing`, #665 `profile-drift`, and #647 part 2
//!   `unattached` and `agent-cannot-dispatch`. A finding is a report, not an error: its code is
//!   never a `RefusalCode`, and two codes (`herdr-version-unsupported`, and #665's
//!   `profile-drift`) equal closed error codes.
//! - [`Finding`]: one finding, with what it is about (a pane, a position, a Herdr pane, a
//!   session, harness servers), a one-line message, its remedy and what `--fix` did.
//! - **Remedies** ([`FindingKind::remedy_for`]; [`FindingKind::remedy`] is it with no agent)
//!   name only the verb that owns each repair (`pane relaunch`, `pane reset`, `pane doctor`),
//!   built from constant words and a [`PaneName`], whose grammar has no space, shell
//!   metacharacter or leading `-`. One kind (`agent-cannot-dispatch`, #647 part 2) also names
//!   the agent to relaunch with: an [`AgentKey`] from the harness's list, which reaches a remedy
//!   only parsed (ASCII letters, digits, `-` and `_`), joined to its flag as `--agent=KEY`
//!   because a key may start with `-`. No remedy carries a session id, a Herdr id, a port, a
//!   profile or any other adapter text, and none is a raw OpenCode, Herdr or tmux command: only
//!   Holler reaches those (epic #633). A finding no Holler verb repairs (a stray session, an
//!   unregistered Herdr pane, an unsupported Herdr, a session no client is attached to) has no
//!   remedy, and its message says why.
//! - **Untrusted text** (session and Herdr ids, the record's host names, adapter messages)
//!   reaches a message only through [`quoted`] (one value) or `embedded` (an adapter's
//!   message), so no control sequence reaches a terminal.
//!
//! **Serde.** A report is output and never read back, so unlike the records (`lib.rs`) it
//! leaves nothing out: every absent value is `null`, and the JSON shape of `holler pane doctor`
//! is fixed (it is part of ADR 0003's versioned `--json` surface).

use serde::{Serialize, Serializer};

use crate::error::excerpt;
use crate::{AgentKey, GridPos, HerdrPane, Pane, PaneError, PaneId, PaneName};

/// The longest an adapter's message may be inside a finding's message, in characters.
const EMBED_LIMIT: usize = 200;

/// The command lines the remedies name, one per verb that owns a repair.
const DOCTOR: &str = "holler pane doctor";
const RELAUNCH: &str = "holler pane relaunch";
const RESET: &str = "holler pane reset";

/// What a reconcile pass found about a pane, a harness server, a session or Herdr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingKind {
    /// Herdr reports an API version the adapter does not know; the message names the
    /// supported ones. No Holler verb upgrades Herdr.
    HerdrVersionUnsupported,
    /// A port call of the pass failed, so nothing was concluded from what it would have
    /// observed. The message names the call as `<port>.<method>` and the error's code. It is
    /// a read (`herdr.snapshot`, `host.ps`, `harness.health`, ...) or the record write
    /// (`pane_store.cas_put`), a write that lost a race with another writer included.
    ObserveFailed,
    /// A Herdr pane that no record names: a process with no record. Whole-fleet runs only.
    UnregisteredHerdrPane,
    /// Herdr has no pane where the record says the pane is.
    HerdrPaneMissing,
    /// The pane's tmux session does not exist.
    TmuxSessionMissing,
    /// The pane's harness server accepts a connection and never answers.
    ServerWedged,
    /// The pane's harness server fails its health check, and is not wedged.
    ServerDown,
    /// The record names no session of record. Doctor never picks one.
    NoSessionOfRecord,
    /// The pane's server answered, and it does not have the session of record.
    SessionOfRecordMissing,
    /// The TUI shows a session its pane's own server does not have: a harness the record
    /// does not know is in the pane.
    TuiForeignSession,
    /// The TUI shows another session than the session of record, or its home screen. The
    /// one kind `--fix` repairs: it selects the session of record.
    ShownDrivenMismatch,
    /// A session on a pane's server that is the session of record of no pane.
    StraySession,
    /// A Herdr pane no record names has shown the same screen for longer than the
    /// operator's threshold (#832). Only a whole-fleet pass with a threshold looks.
    UnmanagedStalled,
    /// A pane's single process (Claude Code) waits for a person: a permission prompt or a
    /// question; the message names which (#831). No Holler verb answers one.
    PaneWaiting,
    /// The processes of the Claude Code panes do not use the subscription login (#831): once
    /// per pass that holds such a pane. The message names the login, never a command that
    /// reads one, and no Holler verb sets a login (I7).
    ClaudeLoginMissing,
    /// The pane's record differs from its profile's spec on at least one field of
    /// `profile_diff`'s table, the comparison `profile show` makes (#665). The message names
    /// the fields and no value. Which side is right is the operator's choice, so no remedy.
    ProfileDrift,
    /// The pane's tmux session exists and no tmux client is attached to it, so no Herdr pane
    /// shows it (#647 part 2, ADR-0021 section 12). A client attached in any terminal counts:
    /// which Herdr pane shows a session is not observable. No Holler verb attaches a client,
    /// so no remedy, and the message says so. (Not the import's finding about no pane.)
    Unattached,
    /// An OpenCode pane whose record names no agent, whose recent turns (the last three of the
    /// session of record) all ran as an agent that cannot dispatch a subagent, while its server
    /// lists an agent that can, the project's orchestrator (#647 part 2, ADR-0021 section 12).
    /// The remedy is the relaunch with that agent when the server lists exactly one; `--fix`
    /// never changes a pane's agent.
    AgentCannotDispatch,
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
        FindingKind::UnmanagedStalled,
        FindingKind::PaneWaiting,
        FindingKind::ClaudeLoginMissing,
        FindingKind::ProfileDrift,
        FindingKind::Unattached,
        FindingKind::AgentCannotDispatch,
    ];

    /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
    /// once merged (ADR-0021 section 9).
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
            FindingKind::UnmanagedStalled => "unmanaged-stalled",
            FindingKind::PaneWaiting => "pane-waiting",
            FindingKind::ClaudeLoginMissing => "claude-login-missing",
            FindingKind::ProfileDrift => "profile-drift",
            FindingKind::Unattached => "unattached",
            FindingKind::AgentCannotDispatch => "agent-cannot-dispatch",
        }
    }

    /// The command a finding of this kind tells the operator to run, for the pane `pane`
    /// (`None` when it is about no pane) once `--fix` left it at `fix`:
    /// [`FindingKind::remedy_for`] with no agent, the form every kind but
    /// `agent-cannot-dispatch` needs.
    pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
        self.remedy_for(pane, fix, None)
    }

    /// The command a finding of this kind tells the operator to run, for the pane `pane`
    /// (`None` when it is about no pane) once `--fix` left it at `fix`, and for `agent`, the
    /// agent it names (#647 part 2; only `agent-cannot-dispatch` reads it); `None` when no
    /// Holler verb repairs it. The one table of remedies:
    ///
    /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
    /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
    /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
    ///   one (choosing an existing session is the operator's, never a guess);
    /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
    ///   it cannot or failed, nothing once it is fixed;
    /// - a pane that waits for a person, and a login that is not the subscription's (#831):
    ///   none, since only a person answers the one and logs in for the other;
    /// - a tmux session no client is attached to (#647 part 2): none, since no Holler verb
    ///   attaches one;
    /// - a pane whose recent turns cannot dispatch while the project ships an orchestrator (#647
    ///   part 2): `holler pane relaunch <pane> --agent=<agent>` when the caller names the one
    ///   orchestrator, and none without it (no agent, or several to choose from). The key is
    ///   joined to the flag with `=`: a key may start with `-`, which would read as a flag.
    pub fn remedy_for(
        self,
        pane: Option<&PaneName>,
        fix: FixState,
        agent: Option<&AgentKey>,
    ) -> Option<String> {
        match self {
            FindingKind::HerdrVersionUnsupported
            | FindingKind::UnregisteredHerdrPane
            | FindingKind::UnmanagedStalled
            | FindingKind::StraySession
            | FindingKind::PaneWaiting
            | FindingKind::ClaudeLoginMissing
            | FindingKind::ProfileDrift
            | FindingKind::Unattached => None,
            FindingKind::AgentCannotDispatch => pane
                .zip(agent)
                .map(|(pane, agent)| relaunch_with_agent(pane, agent)),
            FindingKind::ObserveFailed => Some(doctor_command(pane, false)),
            FindingKind::HerdrPaneMissing
            | FindingKind::TmuxSessionMissing
            | FindingKind::ServerWedged
            | FindingKind::ServerDown
            | FindingKind::TuiForeignSession => pane.map(relaunch_command),
            FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
                pane.map(reset_command)
            }
            FindingKind::ShownDrivenMismatch => match fix {
                FixState::Fixable | FixState::Skipped => {
                    pane.map(|pane| doctor_command(Some(pane), true))
                }
                FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
                FixState::Fixed => None,
            },
        }
    }
}

impl Serialize for FindingKind {
    /// The kind's [`code`](FindingKind::code), so the JSON form cannot drift from it.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

/// What `--fix` did, or could do, about a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixState {
    /// `--fix` does not repair it: every kind but the mismatch, and a mismatch whose server
    /// is not healthy or does not have the session of record.
    NotFixable,
    /// `--fix` would repair it, and this run did not ask to.
    Fixable,
    /// `--fix` was asked for and left it alone: the orchestrator's pane is switched only by a
    /// run that names it.
    Skipped,
    /// `--fix` repaired it: the TUI was seen showing the session of record afterwards.
    Fixed,
    /// `--fix` tried and the TUI was not seen showing the session of record; the finding's
    /// `fix_error` says why.
    Failed,
}

impl FixState {
    /// The state's kebab-case name, which is also its JSON form.
    pub const fn as_str(self) -> &'static str {
        match self {
            FixState::NotFixable => "not-fixable",
            FixState::Fixable => "fixable",
            FixState::Skipped => "skipped",
            FixState::Fixed => "fixed",
            FixState::Failed => "failed",
        }
    }
}

impl Serialize for FixState {
    /// The state's [`as_str`](FixState::as_str).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Why a fix failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixError {
    /// The code of the error the act or the observation after it answered, or
    /// `shown-driven-mismatch` when the switch was acknowledged and the TUI still showed
    /// another session.
    pub code: String,
    /// One line, sanitized as a finding's message is.
    pub message: String,
}

impl From<&PaneError> for FixError {
    fn from(error: &PaneError) -> Self {
        Self {
            code: error.code().to_owned(),
            message: embedded(&error.to_string()),
        }
    }
}

/// One finding of a reconcile pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub kind: FindingKind,
    /// The pane it is about; `None` for Herdr itself, a stray session or an unregistered
    /// Herdr pane.
    pub pane: Option<PaneName>,
    /// Where that pane is, as its record places it, or where the unregistered Herdr pane
    /// is.
    pub grid: Option<GridPos>,
    /// The Herdr pane it is about: the one Herdr lacks (`herdr-pane-missing`) or the one no
    /// record names (`unregistered-herdr-pane`).
    pub herdr_pane: Option<PaneId>,
    /// The session it is about: the stray, the missing session of record, the foreign one,
    /// the one a mismatched TUI shows (`None` there is its home screen), or the session of
    /// record whose recent turns cannot dispatch.
    pub session: Option<String>,
    /// The harness servers, by port and sorted, whose state or sessions it reports: every
    /// in-scope server a stray session was listed on, and the pane's own server for a
    /// wedged or down server, a missing session of record, a foreign session and turns that
    /// cannot dispatch.
    pub ports: Vec<u16>,
    /// One line for a person; untrusted values in it are quoted.
    pub message: String,
    /// The command to run (see [`FindingKind::remedy`]); `None` when no Holler verb repairs
    /// it, or it was fixed.
    pub remedy: Option<String>,
    pub fix: FixState,
    /// Why the fix failed, when it did.
    pub fix_error: Option<FixError>,
}

impl Finding {
    /// A finding of `kind` about no pane, not fixable, with the kind's remedy.
    pub(crate) fn new(kind: FindingKind, message: String) -> Self {
        Self {
            kind,
            pane: None,
            grid: None,
            herdr_pane: None,
            session: None,
            ports: Vec::new(),
            message,
            remedy: kind.remedy(None, FixState::NotFixable),
            fix: FixState::NotFixable,
            fix_error: None,
        }
    }

    /// A finding of `kind` about `pane`, at its record's position, not fixable, with the
    /// kind's remedy for that pane.
    pub(crate) fn about(kind: FindingKind, pane: &Pane, message: String) -> Self {
        Self {
            pane: Some(pane.name.clone()),
            grid: Some(pane.herdr.grid),
            remedy: kind.remedy(Some(&pane.name), FixState::NotFixable),
            ..Self::new(kind, message)
        }
    }

    /// The same finding, about the Herdr pane `herdr` and at its position.
    pub(crate) fn with_herdr_pane(self, herdr: &HerdrPane) -> Self {
        Self {
            herdr_pane: Some(herdr.pane_id.clone()),
            grid: Some(herdr.grid),
            ..self
        }
    }

    /// The same finding, about `session`.
    pub(crate) fn with_session(self, session: Option<&str>) -> Self {
        Self {
            session: session.map(str::to_owned),
            ..self
        }
    }

    /// The same finding, observed on the servers `ports` (sorted).
    pub(crate) fn with_ports(self, ports: Vec<u16>) -> Self {
        Self { ports, ..self }
    }

    /// The same finding, left at `fix` by `--fix` (with `error` when it failed), with the
    /// remedy for that state.
    pub(crate) fn with_fix(self, fix: FixState, error: Option<FixError>) -> Self {
        Self {
            remedy: self.kind.remedy(self.pane.as_ref(), fix),
            fix,
            fix_error: error,
            ..self
        }
    }

    /// The same finding, with the remedy its kind names for `agent` (#647 part 2: the one
    /// orchestrator of `agent-cannot-dispatch`, from the harness's list), from the one table.
    /// Set after [`Finding::with_fix`], which computes the remedy with no agent.
    pub(crate) fn with_agent(self, agent: &AgentKey) -> Self {
        Self {
            remedy: self
                .kind
                .remedy_for(self.pane.as_ref(), self.fix, Some(agent)),
            ..self
        }
    }
}

/// The `holler pane doctor` command line for `pane` (every pane when `None`), with `--fix`
/// when `fix`. It is the reconcile step another verb prints after a failure (ADR-0021
/// sections 8 and 12), so a verb builds it here rather than spelling it again.
pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String {
    let mut line = DOCTOR.to_owned();
    if let Some(pane) = pane {
        line.push(' ');
        line.push_str(pane.as_str());
    }
    if fix {
        line.push_str(" --fix");
    }
    line
}

/// `holler pane relaunch <pane>`.
fn relaunch_command(pane: &PaneName) -> String {
    format!("{RELAUNCH} {pane}")
}

/// `holler pane relaunch <pane> --agent=<agent>`: the key joined to its flag, since a key may
/// start with `-` (`AgentKey::parse` allows it), which as a separate word would read as a flag.
fn relaunch_with_agent(pane: &PaneName, agent: &AgentKey) -> String {
    format!("{RELAUNCH} {pane} --agent={}", agent.as_str())
}

/// `holler pane reset <pane>`.
fn reset_command(pane: &PaneName) -> String {
    format!("{RESET} {pane}")
}

/// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
/// control character and quote in it is escaped, and cut to 64 characters
/// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
/// included.
pub fn quoted(text: &str) -> String {
    excerpt(text)
}

/// An adapter's `message`, to embed in a finding's message: one line, with every control
/// character escaped (`\n`, `\u{1b}`), and cut to 200 characters.
pub(crate) fn embedded(message: &str) -> String {
    let mut out = String::with_capacity(message.len().min(EMBED_LIMIT));
    for c in message.chars().take(EMBED_LIMIT) {
        if c.is_control() {
            out.extend(c.escape_debug());
        } else {
            out.push(c);
        }
    }
    if message.chars().nth(EMBED_LIMIT).is_some() {
        out.push_str("...");
    }
    out
}
