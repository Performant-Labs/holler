//! The plan of a switch or reset (#645; the body that drives the pane, #839): every refusal,
//! before any write or live change, so a refusal changes nothing (step 1 of the parent module's
//! order). Split out of `tx_switch.rs` when #839's driver checks grew it past the size the brief
//! set; the parent keeps the act, the observation, the drive step, the record and the first
//! message.

use super::{
    activity_refusal, parse_session_id, SwitchRequest, Target, ORCHESTRATOR_PANE, SERVER_UNHEALTHY,
    SESSION_DRIVEN_ELSEWHERE, SESSION_OF_OTHER_PANE,
};
use crate::driven::{self, driver_of, elsewhere, Driver};
use crate::findings::{doctor_command, quoted, switch_command, FindingKind, FixState};
use crate::hub::{Advertised, PromptMode, PANE_NOT_DRIVEN};
use crate::pane::{PaneRole, Shape};
use crate::{HarnessEndpoint, Pane, PaneError, PaneName, Ports};

/// What the plan found: the record the run starts from (its generation is the one the record
/// write names), and the live body that drives the pane, if any (#839).
pub(super) struct Plan {
    pub(super) record: Pane,
    pub(super) driver: Option<Advertised>,
}

/// What a switch's plan has already read for its own checks, so the search for the pane's body
/// does not read it again: the server's list and every record.
#[derive(Default)]
struct Reads {
    listed: Option<Vec<String>>,
    records: Option<Vec<Pane>>,
}

/// The plan: every refusal, before any write or live change. Returns the record the run
/// starts from, whose generation the record write names, and the body that drives the pane.
pub(super) fn plan(ports: Ports<'_>, request: &SwitchRequest) -> Result<Plan, PaneError> {
    let existing = match &request.target {
        Target::Existing(id) => Some(parse_session_id(id)?),
        Target::Fresh => None,
    };
    let record = read(ports, request)?;
    refuse_orchestrator(&record, request.as_operator)?;
    let at = record.harness.endpoint(&record.name)?;
    check_health(ports, &record, &at)?;
    if record.harness.kind.shape() == Shape::SingleProcess {
        check_idle(ports, &record, &at)?;
        if let Some(target) = &existing {
            check_unclaimed(ports, &record.name, target)?;
        }
        // No body drives a single process until #850, so the hub is not asked (D3 item 8).
        check_first(ports, request, &record, None)?;
        return Ok(Plan {
            record,
            driver: None,
        });
    }
    let reads = match &existing {
        Some(target) => Reads {
            listed: Some(check_listed(ports, &at, target)?),
            records: Some(check_unclaimed(ports, &record.name, target)?),
        },
        None => {
            check_idle(ports, &record, &at)?;
            Reads::default()
        }
    };
    let driver = find_driver(ports, &record, &at, existing.as_deref(), reads)?;
    check_first(ports, request, &record, driver.as_ref())?;
    Ok(Plan { record, driver })
}

/// The live body that drives `record`'s pane (#839, D3 step 1): `hub.advertised`, then, when a
/// body is advertised, `driver_of` over the server's list and every record (read here unless
/// the switch's checks already read them). An ambiguous or unproven driver is refused
/// `pane-not-driven`; a switch whose `target` another live body advertises is refused
/// [`SESSION_DRIVEN_ELSEWHERE`]; a driver that is not idle is refused by
/// [`activity_refusal`], as the body would refuse the rebind.
fn find_driver(
    ports: Ports<'_>,
    record: &Pane,
    at: &HarnessEndpoint,
    target: Option<&str>,
    reads: Reads,
) -> Result<Option<Advertised>, PaneError> {
    let advertised = ports.hub.advertised()?;
    if advertised.is_empty() {
        return Ok(None);
    }
    let listed = match reads.listed {
        Some(listed) => listed,
        None => ports.harness.list_sessions(at)?,
    };
    let records = match reads.records {
        Some(records) => records,
        None => ports.pane_store.list()?,
    };
    let found = driver_of(record, &advertised, &listed, &records, target);
    if let Some(refusal) = driven::not_driven(&record.name, &found) {
        return Err(refusal);
    }
    let driver = match found {
        Driver::One(body) => Some(body),
        Driver::None | Driver::Ambiguous(_) | Driver::Unproven(_) => None,
    };
    if let Some(target) = target {
        refuse_elsewhere(&advertised, driver, target)?;
    }
    if let Some(body) = driver {
        if let Some((code, doing)) = activity_refusal(body.state) {
            let (id, name) = (quoted(&body.harness_session_id), quoted(&body.name));
            let message = format!("session {id} of {} (its body {name}) {doing}", record.name);
            return Err(PaneError::Refused { code, message });
        }
    }
    Ok(driver.cloned())
}

/// A switch onto `target` is refused when a live body other than the pane's `driver` advertises
/// it (P6): two bodies would drive one harness session.
fn refuse_elsewhere(
    advertised: &[Advertised],
    driver: Option<&Advertised>,
    target: &str,
) -> Result<(), PaneError> {
    let others = elsewhere(advertised, driver, target);
    if others.is_empty() {
        return Ok(());
    }
    let named: Vec<String> = others.iter().map(|body| driven::named(body)).collect();
    Err(PaneError::Refused {
        code: SESSION_DRIVEN_ELSEWHERE,
        message: format!(
            "session {} is driven by another body: {}; one body drives a session, so choose \
             another session, or stop that body first",
            quoted(target),
            named.join(", ")
        ),
    })
}

/// `reset --first` (#839, D5): the message can go only through the hub's gate, which judges a
/// session by the id its body advertises, so the pane's body must advertise the pane's session
/// of record (else `pane-not-driven`), and the gate is asked without sending
/// (`HubPort::prompt` with `Check`): its refusal, before anything moves, is the run's.
fn check_first(
    ports: Ports<'_>,
    request: &SwitchRequest,
    record: &Pane,
    driver: Option<&Advertised>,
) -> Result<(), PaneError> {
    let Some(text) = &request.first else {
        return Ok(());
    };
    let pane = &record.name;
    let message = match (driver, record.session_of_record.as_deref()) {
        (Some(body), Some(sor)) if body.harness_session_id == sor => {
            return ports.hub.prompt(sor, text, PromptMode::Check);
        }
        (Some(body), Some(sor)) => {
            let remedy = switch_command(pane, sor)
                .map_or_else(String::new, |command| format!("; run {command} first"));
            format!(
                "the body {} that drives {pane} is on session {}, not its session of record {}, \
                 so the hub's gate cannot judge the first message{remedy}",
                quoted(&body.name),
                quoted(&body.harness_session_id),
                quoted(sor)
            )
        }
        (Some(_), None) => format!(
            "{pane} has no session of record, so the hub's gate cannot judge the first message; \
             reset it without --first"
        ),
        (None, _) => format!(
            "no body drives {pane}, so its first message cannot go through the hub's gate; \
             reset it without --first, or start a body for it first"
        ),
    };
    Err(PaneError::Refused {
        code: PANE_NOT_DRIVEN,
        message,
    })
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
    record.ok_or_else(|| crate::unmanaged::missing(ports, request.pane.as_str()))
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

/// The pane's harness answers its health check now (I6), at the record's endpoint. One that
/// does not is refused with the remedy doctor gives a down server, taken from the one remedy
/// table, which has one for every named pane; were it ever `None`, the pane doctor command
/// line stands in. A check that fails is passed on as it is. `tx_compact` calls it too (#878).
pub(crate) fn check_health(
    ports: Ports<'_>,
    record: &Pane,
    at: &HarnessEndpoint,
) -> Result<(), PaneError> {
    if ports.harness.health(at)? {
        return Ok(());
    }
    let pane = Some(&record.name);
    let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
        Some(remedy) => remedy,
        None => doctor_command(pane, false),
    };
    let state = match at {
        HarnessEndpoint::Port(port) => {
            format!(
                "the harness server of {} on port {port} does not answer",
                record.name
            )
        }
        HarnessEndpoint::Process { pid, .. } => {
            format!("the harness process {pid} of {} does not run", record.name)
        }
    };
    Err(PaneError::Refused {
        code: SERVER_UNHEALTHY,
        message: format!("{state}; run {remedy}"),
    })
}

/// A reset, and a single process's switch (a relaunch), does not drop a conversation that is
/// mid-turn or waiting on an answer: the pane's session of record must be idle. A pane with no
/// session of record has no conversation to drop, and is not asked. A query that fails is
/// passed on as it is. `tx_compact` calls it too (#878): a compaction is a model call, so it
/// does not run on top of a turn in flight.
pub(crate) fn check_idle(
    ports: Ports<'_>,
    record: &Pane,
    at: &HarnessEndpoint,
) -> Result<(), PaneError> {
    let Some(session) = &record.session_of_record else {
        return Ok(());
    };
    let activity = ports.harness.session_activity(at, session)?;
    let Some((code, doing)) = activity_refusal(activity) else {
        return Ok(());
    };
    Err(PaneError::Refused {
        code,
        message: format!("session {} of {} {doing}", quoted(session), record.name),
    })
}

/// A switch's target is listed by the pane's own server, so a deleted session, or one the
/// server leaves out (a child session), is refused before anything moves. Returns the list,
/// which the search for the pane's body reuses.
fn check_listed(
    ports: Ports<'_>,
    at: &HarnessEndpoint,
    target: &str,
) -> Result<Vec<String>, PaneError> {
    let sessions = ports.harness.list_sessions(at)?;
    if sessions.iter().any(|id| id == target) {
        return Ok(sessions);
    }
    Err(PaneError::SessionNotFound {
        what: quoted(target),
    })
}

/// A switch's target is no other pane's session of record. Servers that share a data
/// directory list every pane's sessions, so the server's list alone would let a switch show
/// one pane another's conversation. The first such record in list order is named. Returns
/// every record, which the search for the pane's body reuses.
fn check_unclaimed(
    ports: Ports<'_>,
    pane: &PaneName,
    target: &str,
) -> Result<Vec<Pane>, PaneError> {
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
        None => Ok(records),
    }
}
