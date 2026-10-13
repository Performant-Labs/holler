//! The switch/reset transaction (story #645): the session a pane shows and the hub drives
//! changes through the harness API, the TUI and the record together, and is recorded only
//! once the TUI is seen showing it. `holler pane switch` points a pane at a session its server
//! already has ([`Target::Existing`]); `holler pane reset` starts it on one the run creates
//! ([`Target::Fresh`]). One engine runs both (ADR-0021 section 8, "Switch and reset as built").
//!
//! A run is plan, act, observe, drive, record (I3), and then the first message, in this order:
//!
//! 1. **Plan**, with no write and no live change, so a refusal changes nothing. Re-check an
//!    existing target's id ([`parse_session_id`]). Read the record, through the profile's
//!    scope when the request names one (`pane-not-found`, `profile-not-found`,
//!    `pane-not-in-profile`). Refuse the orchestrator's pane unless the caller acts as the
//!    operator ([`ORCHESTRATOR_PANE`]). Check the pane's harness live (I6), never from the
//!    stored `harness.health` ([`SERVER_UNHEALTHY`]). For a reset of a pane that has a
//!    session of record, ask the harness what that session is doing (`session_activity`) and
//!    refuse one that is running a turn ([`SESSION_BUSY`]), holds a question
//!    ([`SESSION_HOLDS_QUESTION`]) or waits on a permission prompt
//!    ([`SESSION_HOLDS_PERMISSION`]); a pane with none is not asked, since reset is doctor's
//!    remedy for it. For a switch, check that the server lists the target
//!    (`session-not-found`) and that no other record names it as its session of record
//!    ([`SESSION_OF_OTHER_PANE`]). That last check is a read, not the authority: the
//!    registry's compare-and-swap does not enforce it, so two concurrent switches of two panes
//!    to one session can both pass it. Then find the pane's body (#839): `hub.advertised`, and
//!    when a body is advertised the server's list and the other records (a switch has read
//!    both already), joined by `driven::driver_of`, which takes a body to be the pane's only
//!    on evidence. A driver that is ambiguous or unproven is refused `pane-not-driven`, naming
//!    the bodies and the two steps that pair the pane by hand; a switch whose target another
//!    live body advertises is refused [`SESSION_DRIVEN_ELSEWHERE`]; and a driver that runs a
//!    turn, holds a question or waits on a permission is refused as above
//!    ([`activity_refusal`]). For `reset --first`, a pane whose body does not advertise its
//!    session of record (or that no body drives) is refused `pane-not-driven`, and the hub's
//!    prompt path is asked, without sending, whether it would admit the message
//!    (`HubPort::prompt` with `Check`): a parked, held or unhealthy pane, or one whose SHOWN
//!    and DRIVEN differ, is refused before anything moves.
//! 2. **Act**: for a reset, `create_session` on the pane's server; then `select_session` in
//!    the pane's TUI.
//! 3. **Observe**: `shown_session`. A TUI that does not show the target is the closed failure
//!    `unavailable` (exit 1; an open code would be a refusal), and nothing is recorded.
//! 4. **Drive** (#839): `hub.rebind` asks the pane's body to drive the target; the id it
//!    acknowledges must be the target, else `unavailable`. A body already on the target is not
//!    asked (its advertised id is the acknowledgement), and a pane no body drives skips the
//!    step. It comes after the observation, so a failed `select_session` leaves the body where
//!    it was; a failure here is after the act.
//! 5. **Record**: the record read in step 1 with exactly five fields set (the target as
//!    `session_of_record` and `last_observed.shown`, the acknowledged id as
//!    `last_observed.driven`, or `None` when no body drives the pane, the request's clock as
//!    `last_observed.at`, and `harness.health` healthy, which step 1 observed), in one
//!    compare-and-swap at the generation step 1 read, never retried. DRIVEN is what the body
//!    acknowledged or advertised, observed and never inferred, so a stale stored value goes.
//!    Every other field is the read record's, so a field this file does not know is kept.
//! 6. **First message**, only when the request carries one (`pane reset --first`):
//!    `HubPort::prompt` with `Send` queues it to the target, now the session of record, through
//!    the hub's one prompt path (`send_prompt`), accepted and returned. A refusal here (a race:
//!    the pane was parked or held since the check) is the `unprompted` failure.
//!
//! **A single process (#831, ADR-0021 section 15 item 6).** A pane whose harness's shape is
//! `SingleProcess` (Claude Code) has no server to create a session on and no TUI to switch, so
//! its switch and its reset are relaunches. The plan refuses as above, except that a switch also
//! refuses a pane that is not idle (a relaunch drops the turn in flight) and does not ask for a
//! session list: whether the target has a conversation is the relaunch plan's
//! (`process_command` with a resume-only choice: `session-not-found`, before anything is
//! stopped; plan review W1). It asks the hub nothing: no body drives such a pane until #850, so
//! its `reset --first` is refused `pane-not-driven` in the plan. Steps 2 to 5 are then
//! `tx_launch::relaunch_recorded`: the pane's processes stopped, its process run on the target
//! (switch) or on a fresh session (reset), located, seen holding it and recorded with its new
//! pid, its stored DRIVEN kept.
//!
//! No step of an in-place run calls Herdr or the host, so nothing is ever typed into a TUI
//! (I4), and no step serves, attaches, aborts or deletes anything; a relaunch calls them as
//! `tx_launch` does, and types nothing either. A failure once `select_session` has been
//! called (or once a relaunch entered its act) is not compensated: the record still names the
//! previous session, and [`SwitchFailure::message`] ends with the reconcile step (the pane
//! doctor command line for the pane, with `--fix`). A failure before that, a reset's
//! `create_session` included, moved neither the TUI nor the record, and its message has no
//! step. A session a reset created and did not record stays on the server, and the message
//! names it. A failed prompt comes after the record: the TUI and the record agree on the
//! target, so its message has no step, and says that the session was created, shown and
//! recorded but the first message did not land. After a successful reset the previous session
//! stays on the server too: no port deletes a session, so doctor reports it as a stray. If the
//! record's write fails after a rebind, the body is ahead of the record: doctor reports it, and
//! `holler pane switch` to the session of record re-points it; nothing is compensated.
//!
//! An in-place run makes at most thirteen port calls, each bounded by I5, so it needs no budget
//! of its own; a relaunch runs under `tx_launch`'s budget.

mod plan;

/// The live health and idle checks of the plan, which `tx_compact` (#878) calls and does not
/// copy; `plan` itself stays private.
pub(crate) use plan::{check_health, check_idle};

use crate::error::RefusalCode;
use crate::findings::{doctor_command, quoted};
use crate::hub::{Advertised, PromptMode};
use crate::pane::{Health, Shape};
use crate::ports::Activity;
use crate::reconcile::shown_differs;
use crate::tx_launch::{self, ProcessSession, TxFailure, TxOptions};
use crate::{HarnessEndpoint, Pane, PaneError, PaneName, Ports, ProfileName};

use plan::{plan, Plan};

pub use crate::findings::{parse_session_id, SESSION_ID_MAX};

/// `orchestrator-pane`: the pane's role is `orchestrator` and the caller did not pass
/// `--as-operator`. Refusal, exit 3.
pub const ORCHESTRATOR_PANE: RefusalCode = RefusalCode::from_static("orchestrator-pane");
/// `server-unhealthy`: the pane's harness server does not answer `health`. Refusal, exit 3.
pub const SERVER_UNHEALTHY: RefusalCode = RefusalCode::from_static("server-unhealthy");
/// `session-of-other-pane`: the switch target is another pane's session of record.
/// Refusal, exit 3.
pub const SESSION_OF_OTHER_PANE: RefusalCode = RefusalCode::from_static("session-of-other-pane");
/// `session-busy`: a reset's pane has a session of record that is running a turn.
/// Refusal, exit 3.
pub const SESSION_BUSY: RefusalCode = RefusalCode::from_static("session-busy");
/// `session-holds-question`: a reset's pane has a session of record waiting for an answer
/// to a question it asked. Refusal, exit 3.
pub const SESSION_HOLDS_QUESTION: RefusalCode = RefusalCode::from_static("session-holds-question");
/// `session-holds-permission`: the pane's session of record waits for a person to answer a
/// permission prompt (#831, `Activity::WaitingPermission`), so a reset, or a single process's
/// switch, which relaunches it, would drop it. Refusal, exit 3.
pub const SESSION_HOLDS_PERMISSION: RefusalCode =
    RefusalCode::from_static("session-holds-permission");
/// `session-driven-elsewhere`: a live body other than the pane's own already advertises the
/// switch's target (#839, P6), so the switch would put two bodies on one harness session.
/// Refusal, exit 3.
pub const SESSION_DRIVEN_ELSEWHERE: RefusalCode =
    RefusalCode::from_static("session-driven-elsewhere");

/// The one mapping of what a session is doing to its refusal (#839): the code, and the words
/// that finish "session S of P ...". `check_idle` (the harness's answer) and the plan's check
/// of the pane's body (the hub's advertisement) both call it. `None` for an idle session.
pub fn activity_refusal(activity: Activity) -> Option<(RefusalCode, &'static str)> {
    match activity {
        Activity::Idle => None,
        Activity::Busy => Some((SESSION_BUSY, "is running a turn; try again once it ends")),
        Activity::HoldingQuestion => {
            Some((SESSION_HOLDS_QUESTION, "holds a question; answer it first"))
        }
        Activity::WaitingPermission => Some((
            SESSION_HOLDS_PERMISSION,
            "waits on a permission prompt; answer it in the pane first",
        )),
    }
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
    /// The first message to queue to the target once it is recorded (`reset --first`).
    pub first: Option<String>,
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
    /// The session the run created, showed and recorded, whose first message (`HubPort::prompt`
    /// with `Send`) then failed or was refused. The record is correct, so such a failure is
    /// never `acted` and never `created`.
    pub unprompted: Option<String>,
}

impl From<PaneError> for SwitchFailure {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            acted: false,
            created: None,
            unprompted: None,
        }
    }
}

impl SwitchFailure {
    /// The one-line message of the failure for `pane`: `error`'s text, then `; session <id>
    /// was created and is not recorded` when a reset left a session behind, or `; session
    /// <id> was created, shown and recorded, but the first message did not land` when only
    /// the prompt failed, then `; to reconcile, run <the pane doctor command line for pane,
    /// with --fix>` once `select_session` has been called.
    pub fn message(&self, pane: &PaneName) -> String {
        let mut message = self.error.to_string();
        if let Some(created) = &self.created {
            message.push_str(&format!(
                "; session {} was created and is not recorded",
                quoted(created)
            ));
        }
        if let Some(recorded) = &self.unprompted {
            message.push_str(&format!(
                "; session {} was created, shown and recorded, but the first message did not land",
                quoted(recorded)
            ));
        }
        if self.acted {
            message.push_str("; to reconcile, run ");
            message.push_str(&doctor_command(Some(pane), true));
        }
        message
    }
}

/// Run one switch or reset over `ports` (see the module docs). `Ok` once the target was seen
/// shown (by the TUI, or held by the relaunched process) and the record names it.
pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
    let plan = plan(ports, request)?;
    let pane = match plan.record.harness.kind.shape() {
        Shape::ServerAndTui => switch_in_place(ports, request, &plan)?,
        Shape::SingleProcess => relaunched(ports, request, &plan.record)?,
    };
    if let Some(text) = &request.first {
        prompt(ports, &pane, text)?;
    }
    Ok(Switched {
        pane,
        previous: plan.record.session_of_record,
    })
}

/// Steps 2 to 5 for a server and its TUI: a reset's `create_session`, `select_session`, the
/// observation, the rebind of the pane's body and the record. Returns the record as stored.
fn switch_in_place(
    ports: Ports<'_>,
    request: &SwitchRequest,
    plan: &Plan,
) -> Result<Pane, SwitchFailure> {
    let record = &plan.record;
    let at = record.harness.endpoint(&record.name)?;
    let (target, created) = match &request.target {
        Target::Existing(id) => (id.clone(), None),
        Target::Fresh => {
            let id = ports.harness.create_session(&at)?;
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
        unprompted: None,
    };
    ports
        .harness
        .select_session(&record.herdr.pane_id, &target)
        .map_err(acted)?;
    observe(ports, record, &at, &target).map_err(acted)?;
    let driven = drive(ports, plan.driver.as_ref(), &target).map_err(acted)?;
    let next = recorded(record, &target, request.now_ms, driven);
    ports
        .pane_store
        .cas_put(&next, record.generation)
        .map_err(acted)
}

/// Step 4: re-point the pane's body to `target` (#839). Answers the DRIVEN the record gets:
/// the id the body acknowledged, or `None` when no body drives the pane. A body that already
/// advertises the target is not asked, and an acknowledgement of another id is `unavailable`.
fn drive(
    ports: Ports<'_>,
    driver: Option<&Advertised>,
    target: &str,
) -> Result<Option<String>, PaneError> {
    let Some(body) = driver else {
        return Ok(None);
    };
    if body.harness_session_id == target {
        return Ok(Some(target.to_owned()));
    }
    let acknowledged = ports.hub.rebind(body, target)?;
    if acknowledged != target {
        return Err(PaneError::Unavailable {
            what: format!(
                "the body {} acknowledged session {}, not {}",
                quoted(&body.name),
                quoted(&acknowledged),
                quoted(target)
            ),
        });
    }
    Ok(Some(acknowledged))
}

/// Steps 2 to 4 for a single process (#831): a relaunch of the pane as recorded, on the target
/// (switch, which must have a conversation) or on a fresh session (reset), through
/// `tx_launch::relaunch_recorded`, the one stop, run, wait, locate and record sequence. Returns
/// the record the relaunch stored.
fn relaunched(
    ports: Ports<'_>,
    request: &SwitchRequest,
    record: &Pane,
) -> Result<Pane, SwitchFailure> {
    let session = match &request.target {
        Target::Existing(id) => ProcessSession::Resume(id.clone()),
        Target::Fresh => ProcessSession::Fresh,
    };
    let launched = tx_launch::relaunch_recorded(ports, record, &session, &TxOptions::default())
        .map_err(|TxFailure { error, acted }| SwitchFailure {
            error,
            acted,
            created: None,
            unprompted: None,
        })?;
    let stored = launched.pane.ok_or_else(|| PaneError::Unavailable {
        what: format!("the relaunch of {} stored no record", record.name),
    })?;
    Ok(stored)
}

/// Step 6: queue `text` to the session of record of `pane`, the record the run stored, through
/// the hub's one prompt path (#839: `HubPort::prompt` with `Send`, accepted and returned). The
/// prompt goes only to the session just recorded (I2), so a failure here, a refusal in a race
/// included, leaves the TUI and the record in agreement: no reconcile step, nothing unrecorded.
fn prompt(ports: Ports<'_>, pane: &Pane, text: &str) -> Result<(), SwitchFailure> {
    let target = pane.session_of_record.clone().unwrap_or_default();
    ports
        .hub
        .prompt(&target, text, PromptMode::Send)
        .map_err(|error| SwitchFailure {
            error,
            acted: false,
            created: None,
            unprompted: Some(target.clone()),
        })
}

/// The observation, once `target` has been selected: what the pane's TUI shows. Anything
/// but `target`, its home screen included, is `unavailable` (I3).
fn observe(
    ports: Ports<'_>,
    record: &Pane,
    at: &HarnessEndpoint,
    target: &str,
) -> Result<(), PaneError> {
    let shown = ports.harness.shown_session(&record.herdr.pane_id, at)?;
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
/// its TUI was seen showing at `now_ms`, `driven` as what its body acknowledged (`None`: no
/// body drives it), and its server healthy. Every other field is `record`'s.
fn recorded(record: &Pane, target: &str, now_ms: i64, driven: Option<String>) -> Pane {
    let mut next = record.clone();
    next.session_of_record = Some(target.to_owned());
    next.last_observed.shown = Some(target.to_owned());
    next.last_observed.driven = driven;
    next.last_observed.at = now_ms;
    next.harness.health = Health::Healthy;
    next
}
