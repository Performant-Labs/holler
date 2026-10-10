//! The switch/reset transaction (story #645): the session a pane shows and the hub drives
//! changes through the harness API, the TUI and the record together, and is recorded only
//! once the TUI is seen showing it. `holler pane switch` points a pane at a session its server
//! already has ([`Target::Existing`]); `holler pane reset` starts it on one the run creates
//! ([`Target::Fresh`]). One engine runs both (ADR-0021 section 8, "Switch and reset as built").
//!
//! A run is plan, act, observe, record (I3), in this order:
//!
//! 1. **Plan**, with no write and no live change, so a refusal changes nothing. Re-check an
//!    existing target's id ([`parse_session_id`]). Read the record, through the profile's
//!    scope when the request names one (`pane-not-found`, `profile-not-found`,
//!    `pane-not-in-profile`). Refuse the orchestrator's pane unless the caller acts as the
//!    operator ([`ORCHESTRATOR_PANE`]). Check the pane's harness server live (I6), never from
//!    the stored `harness.health` ([`SERVER_UNHEALTHY`]). For a switch, check that the server
//!    lists the target (`session-not-found`) and that no other record names it as its session
//!    of record ([`SESSION_OF_OTHER_PANE`]). That last check is a read, not the authority: the
//!    registry's compare-and-swap does not enforce it, so two concurrent switches of two panes
//!    to one session can both pass it.
//! 2. **Act**: for a reset, `create_session` on the pane's server; then `select_session` in
//!    the pane's TUI.
//! 3. **Observe**: `shown_session`. A TUI that does not show the target is the closed failure
//!    `unavailable` (exit 1; an open code would be a refusal), and nothing is recorded.
//! 4. **Record**: the record read in step 1 with exactly four fields set (the target as
//!    `session_of_record` and `last_observed.shown`, the request's clock as
//!    `last_observed.at`, and `harness.health` healthy, which step 1 observed), in one
//!    compare-and-swap at the generation step 1 read, never retried. `last_observed.driven`
//!    is left as stored (no port observes DRIVEN before #649), and every other field is the
//!    read record's, so a field this file does not know is kept.
//!
//! No step calls Herdr or the host, so nothing is ever typed into a TUI (I4), and no step
//! serves, attaches, aborts or deletes anything. A failure once `select_session` has been
//! called is not compensated: the record still names the previous session, and
//! [`SwitchFailure::message`] ends with the reconcile step (the pane doctor command line for
//! the pane, with `--fix`), which selects that session again. A failure before that, a
//! reset's `create_session` included, moved neither the TUI nor the record, and its message
//! has no step. A session a reset created and did not record stays on the server, and the
//! message names it. After a successful reset the previous session stays on the server
//! too: no port deletes a session, so doctor reports it as a stray.
//!
//! A run makes at most seven port calls, each bounded by I5, so it needs no budget of its own.
//! Refusing a pane that is busy or holds a question, and queueing a first message after a
//! reset, are not here: no port reports a session's activity or sends it a prompt.

use crate::error::RefusalCode;
use crate::findings::{doctor_command, quoted, FindingKind, FixState};
use crate::pane::{Health, PaneRole};
use crate::reconcile::shown_differs;
use crate::{Pane, PaneError, PaneName, Ports, ProfileName};

/// `orchestrator-pane`: the pane's role is `orchestrator` and the caller did not pass
/// `--as-operator`. Refusal, exit 3.
pub const ORCHESTRATOR_PANE: RefusalCode = RefusalCode::from_static("orchestrator-pane");
/// `server-unhealthy`: the pane's harness server does not answer `health`. Refusal, exit 3.
pub const SERVER_UNHEALTHY: RefusalCode = RefusalCode::from_static("server-unhealthy");
/// `session-of-other-pane`: the switch target is another pane's session of record.
/// Refusal, exit 3.
pub const SESSION_OF_OTHER_PANE: RefusalCode = RefusalCode::from_static("session-of-other-pane");
/// The longest session id [`parse_session_id`] accepts. It is the length `findings::quoted`
/// cuts at. `quoted` counts characters and [`parse_session_id`] counts bytes, which agree
/// because an accepted id is ASCII, so a typed id is never cut short in a message.
pub const SESSION_ID_MAX: usize = 64;

/// A harness session id typed by a person: 1 to [`SESSION_ID_MAX`] ASCII letters, digits,
/// `_` or `-` (OpenCode's ids are `ses_` and 26 letters and digits). Anything else is
/// `usage`, and the message quotes the text with [`quoted`], so no whitespace, control
/// character or shell metacharacter reaches a port or a message.
pub fn parse_session_id(text: &str) -> Result<String, PaneError> {
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    if (1..=SESSION_ID_MAX).contains(&text.len()) && text.bytes().all(allowed) {
        return Ok(text.to_owned());
    }
    Err(PaneError::Usage {
        message: format!(
            "invalid session id {}: a session id is 1 to {SESSION_ID_MAX} ASCII letters, \
             digits, '_' or '-'",
            quoted(text)
        ),
    })
}

/// The session a run moves the pane to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `switch`: a session the pane's server already has.
    Existing(String),
    /// `reset`: a session this run creates.
    Fresh,
}

/// One switch or reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchRequest {
    pub pane: PaneName,
    pub profile: Option<ProfileName>,
    pub target: Target,
    pub as_operator: bool,
    /// Milliseconds since the Unix epoch, written as `last_observed.at`.
    pub now_ms: i64,
}

/// A run that recorded its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switched {
    /// The record as `cas_put` stored it.
    pub pane: Pane,
    /// The session of record before the run.
    pub previous: Option<String>,
}

/// A run that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchFailure {
    pub error: PaneError,
    /// `select_session` was called (the TUI may have moved): the message carries the
    /// reconcile step. It is set on that call's own failure, whose effect on the screen is
    /// unknown, and on every failure after the call; never on one before it.
    pub acted: bool,
    /// The session `reset` created and did not record (it stays on the server).
    pub created: Option<String>,
}

impl From<PaneError> for SwitchFailure {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            acted: false,
            created: None,
        }
    }
}

impl SwitchFailure {
    /// The one-line message of the failure for `pane`: `error`'s text, then `; session <id>
    /// was created and is not recorded` when a reset left a session behind, then `; to
    /// reconcile, run <the pane doctor command line for pane, with --fix>` once
    /// `select_session` has been called.
    pub fn message(&self, pane: &PaneName) -> String {
        let mut message = self.error.to_string();
        if let Some(created) = &self.created {
            message.push_str(&format!(
                "; session {} was created and is not recorded",
                quoted(created)
            ));
        }
        if self.acted {
            message.push_str("; to reconcile, run ");
            message.push_str(&doctor_command(Some(pane), true));
        }
        message
    }
}

/// Run one switch or reset over `ports` (see the module docs). `Ok` once the TUI was seen
/// showing the target and the record names it.
pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
    let record = plan(ports, request)?;
    let (target, created) = match &request.target {
        Target::Existing(id) => (id.clone(), None),
        Target::Fresh => {
            let id = ports.harness.create_session(record.harness.port)?;
            (id.clone(), Some(id))
        }
    };
    // Nothing above has moved the TUI or the record (a reset's new session is neither shown
    // nor recorded yet), so each failure so far converts with `acted: false`. From the
    // `select_session` call below on, every failure is `acted`, the call's own included:
    // the engine cannot tell how far a failed call got, so the TUI may have moved.
    let acted = |error| SwitchFailure {
        error,
        acted: true,
        created: created.clone(),
    };
    ports
        .harness
        .select_session(&record.herdr.pane_id, &target)
        .map_err(acted)?;
    observe(ports, &record, &target).map_err(acted)?;
    let next = recorded(&record, &target, request.now_ms);
    let pane = ports
        .pane_store
        .cas_put(&next, record.generation)
        .map_err(acted)?;
    Ok(Switched {
        pane,
        previous: record.session_of_record,
    })
}

/// The plan: every refusal, before any write or live change. Returns the record the run
/// starts from, whose generation the record write names.
fn plan(ports: Ports<'_>, request: &SwitchRequest) -> Result<Pane, PaneError> {
    let existing = match &request.target {
        Target::Existing(id) => Some(parse_session_id(id)?),
        Target::Fresh => None,
    };
    let record = read(ports, request)?;
    refuse_orchestrator(&record, request.as_operator)?;
    check_health(ports, &record)?;
    if let Some(target) = &existing {
        check_listed(ports, &record, target)?;
        check_unclaimed(ports, &record.name, target)?;
    }
    Ok(record)
}

/// The record of the request's pane: through the profile's scope when the request names a
/// profile (`profile-not-found`, `pane-not-in-profile`), else from the pane store. No
/// record is `pane-not-found`, and so is a scope that answers with another pane. The
/// scope's answer is searched by the pane's name, as `pane get` does, not taken as its
/// first pane. While `resolve` keeps its contract (a named pane is answered alone) the two
/// are the same, and a scope that breaks it cannot make the run act on another pane.
fn read(ports: Ports<'_>, request: &SwitchRequest) -> Result<Pane, PaneError> {
    let record = match &request.profile {
        Some(profile) => ports
            .scope
            .resolve(profile, Some(&request.pane))?
            .panes
            .into_iter()
            .find(|pane| pane.name == request.pane),
        None => ports.pane_store.get(&request.pane)?,
    };
    record.ok_or_else(|| PaneError::PaneNotFound {
        what: request.pane.to_string(),
    })
}

/// The orchestrator's pane changes only as the operator's deliberate act (`--as-operator`).
fn refuse_orchestrator(record: &Pane, as_operator: bool) -> Result<(), PaneError> {
    if record.role == PaneRole::Orchestrator && !as_operator {
        return Err(PaneError::Refused {
            code: ORCHESTRATOR_PANE,
            message: format!(
                "{} is the orchestrator's pane; pass --as-operator to change it",
                record.name
            ),
        });
    }
    Ok(())
}

/// The pane's harness server answers its health check now (I6). One that does not is
/// refused with the remedy doctor gives a down server, taken from the one remedy table,
/// which has one for every named pane; were it ever `None`, the pane doctor command line
/// stands in. A check that fails is passed on as it is.
fn check_health(ports: Ports<'_>, record: &Pane) -> Result<(), PaneError> {
    let port = record.harness.port;
    if ports.harness.health(port)? {
        return Ok(());
    }
    let pane = Some(&record.name);
    let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
        Some(remedy) => remedy,
        None => doctor_command(pane, false),
    };
    Err(PaneError::Refused {
        code: SERVER_UNHEALTHY,
        message: format!(
            "the harness server of {} on port {port} does not answer; run {remedy}",
            record.name
        ),
    })
}

/// A switch's target is listed by the pane's own server, so a deleted session, or one the
/// server leaves out (a child session), is refused before anything moves.
fn check_listed(ports: Ports<'_>, record: &Pane, target: &str) -> Result<(), PaneError> {
    let sessions = ports.harness.list_sessions(record.harness.port)?;
    if sessions.iter().any(|id| id == target) {
        return Ok(());
    }
    Err(PaneError::SessionNotFound {
        what: quoted(target),
    })
}

/// A switch's target is no other pane's session of record. Servers that share a data
/// directory list every pane's sessions, so the server's list alone would let a switch show
/// one pane another's conversation. The first such record in list order is named.
fn check_unclaimed(ports: Ports<'_>, pane: &PaneName, target: &str) -> Result<(), PaneError> {
    let records = ports.pane_store.list()?;
    let claimed = records
        .iter()
        .find(|other| other.name != *pane && other.session_of_record.as_deref() == Some(target));
    match claimed {
        Some(other) => Err(PaneError::Refused {
            code: SESSION_OF_OTHER_PANE,
            message: format!(
                "{} is the session of record of {}",
                quoted(target),
                other.name
            ),
        }),
        None => Ok(()),
    }
}

/// The observation, once `target` has been selected: what the pane's TUI shows. Anything
/// but `target`, its home screen included, is `unavailable` (I3).
fn observe(ports: Ports<'_>, record: &Pane, target: &str) -> Result<(), PaneError> {
    let shown = ports.harness.shown_session(&record.herdr.pane_id)?;
    if !shown_differs(Some(target), shown.as_deref()) {
        return Ok(());
    }
    Err(PaneError::Unavailable {
        what: format!(
            "the TUI of {} shows {}, not session {}",
            record.name,
            screen_text(shown.as_deref()),
            quoted(target)
        ),
    })
}

/// How a message names what the TUI shows. A word-for-word copy of reconcile's private
/// `screen_text` (`reconcile/observe.rs`), so doctor and switch describe a screen alike;
/// folding the two into one function in `findings` is a follow-up of #645.
fn screen_text(shown: Option<&str>) -> String {
    shown.map_or_else(
        || "its home screen".to_owned(),
        |session| format!("session {}", quoted(session)),
    )
}

/// The record a run writes: `record` with `target` as its session of record and as what
/// its TUI was seen showing at `now_ms`, and its server healthy. Every other field, and
/// `last_observed.driven`, is `record`'s.
fn recorded(record: &Pane, target: &str, now_ms: i64) -> Pane {
    let mut next = record.clone();
    next.session_of_record = Some(target.to_owned());
    next.last_observed.shown = Some(target.to_owned());
    next.last_observed.at = now_ms;
    next.harness.health = Health::Healthy;
    next
}
