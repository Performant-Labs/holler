//! The chain of a pane whose harness is one process (#831, ADR-0021 section 15 items 6 and 13;
//! plan review W2 and W3): Claude Code, which has no server list and no TUI.
//!
//! - **Observe**: the parent's `host.ps`, then `harness.health` at the process's endpoint
//!   (whether it still runs; a gone process is `server-down`, and there is no wedge to tell
//!   apart), `harness.shown_session` (the session it holds now; a person's `/clear` or
//!   `/resume` moves it) and, while it runs, `harness.session_activity`: a process that waits
//!   for a person is `pane-waiting`, with the reason.
//! - **Compare**: `no-session-of-record`, and the one mismatch, the process holding another
//!   session than the session of record (`shown-driven-mismatch`). The rules that need a
//!   server's list are skipped: `session-of-record-missing`, `tui-foreign-session`,
//!   `stray-session` and `server-wedged`. A process that is gone shows nothing, so its screen
//!   is not read and no mismatch is reported beside its `server-down`.
//! - **Repair**, with `--fix`: a relaunch on the session of record, through the caller's
//!   [`Relaunch`], and only while the process is idle, so a repair never drops a running turn
//!   (the rule `reset` and `switch` keep); one that works or waits is `not-fixable`, with the
//!   relaunch remedy. The orchestrator's pane is repaired only by a pass that names it.
//! - **Record**: the parent's, unless a relaunch was attempted. The relaunch stores its own
//!   record (a new pid, the session of record held), and a failed one has stopped and rolled
//!   back the process, so what the chain saw before it is no longer the pane's state.

use crate::findings::{quoted, Finding, FindingKind, FixError, FixState};
use crate::pane::PaneRole;
use crate::ports::Activity;
use crate::{HarnessEndpoint, Pane, Ports};

use super::{
    check_session_of_record, observe_failed, observe_screen, record, screen_text, shown_differs,
    summary, Checked, ReconcileRequest, Relaunch, Screen, Server, HARNESS_HEALTH,
};

/// The activity call of a single process's chain, named as `<port>.<method>`.
const HARNESS_SESSION_ACTIVITY: &str = "harness.session_activity";

/// Run the chain of the single-process pane `pane`, whose harness is at `at`, after the
/// parent's tmux check (its findings are `findings`).
pub(super) fn check(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    relaunch: Option<Relaunch<'_>>,
    pane: &Pane,
    at: &HarnessEndpoint,
    mut findings: Vec<Finding>,
) -> Checked {
    let server = observe_process(ports, pane, at, &mut findings);
    let screen = match server {
        Server::Down => Screen::Unseen,
        _ => observe_screen(ports, pane, at, &mut findings),
    };
    check_session_of_record(pane, &server, &mut findings);
    let activity = match server {
        Server::Healthy { .. } => activity(ports, pane, at, &mut findings),
        _ => None,
    };
    let fixer = Fixer {
        request,
        relaunch,
        activity,
    };
    let (screen, relaunched) = check_held(&fixer, pane, screen, &mut findings);
    if !relaunched {
        findings.extend(record(ports, request, pane, &server, &screen));
    }
    Checked {
        findings,
        summary: summary(pane, screen.shown(), server.health()),
        listed: None,
    }
}

/// The process's health check: running (a server with no list), gone (`server-down`, with the
/// relaunch remedy), or not known (the check failed: `observe-failed`).
fn observe_process(
    ports: Ports<'_>,
    pane: &Pane,
    at: &HarnessEndpoint,
    findings: &mut Vec<Finding>,
) -> Server {
    match ports.harness.health(at) {
        Ok(true) => Server::Healthy { sessions: None },
        Ok(false) => {
            let message = format!("the harness {at} does not run");
            findings.push(Finding::about(FindingKind::ServerDown, pane, message));
            Server::Down
        }
        Err(error) => {
            findings.push(observe_failed(pane, HARNESS_HEALTH, &error));
            Server::Unknown
        }
    }
}

/// What the running process is doing, for the repair's idle gate, and `pane-waiting` when it
/// waits for a person, named by the reason. `None`, with an `observe-failed`, when the call
/// failed. A process holds one session at a time, so the session named is the session of
/// record, or none.
fn activity(
    ports: Ports<'_>,
    pane: &Pane,
    at: &HarnessEndpoint,
    findings: &mut Vec<Finding>,
) -> Option<Activity> {
    let session = pane.session_of_record.as_deref().unwrap_or_default();
    match ports.harness.session_activity(at, session) {
        Ok(activity) => {
            if let Some(reason) = waits_for(activity) {
                let message = format!(
                    "the harness {at} waits on a {reason}; a person answers it in the pane (no \
                     holler verb does)"
                );
                findings.push(Finding::about(FindingKind::PaneWaiting, pane, message));
            }
            Some(activity)
        }
        Err(error) => {
            findings.push(observe_failed(pane, HARNESS_SESSION_ACTIVITY, &error));
            None
        }
    }
}

/// The reason a process waits for a person, as `pane-waiting` names it; `None` for a process
/// that does not.
fn waits_for(activity: Activity) -> Option<&'static str> {
    match activity {
        Activity::WaitingPermission => Some("permission prompt"),
        Activity::HoldingQuestion => Some("question (input needed)"),
        Activity::Idle | Activity::Busy => None,
    }
}

/// What a repair needs besides the pane: the request, the caller's relaunch and the process's
/// activity.
struct Fixer<'r> {
    request: &'r ReconcileRequest<'r>,
    relaunch: Option<Relaunch<'r>>,
    activity: Option<Activity>,
}

/// The process's one mismatch: `shown-driven-mismatch` when it holds another session than the
/// session of record, repaired by [`repair`] when the record decides it. Returns the screen at
/// the end of the chain (after a relaunch, the session the relaunched process holds) and
/// whether a relaunch was attempted.
fn check_held(
    fixer: &Fixer<'_>,
    pane: &Pane,
    screen: Screen,
    findings: &mut Vec<Finding>,
) -> (Screen, bool) {
    let (Screen::Seen(shown), Some(record)) = (&screen, pane.session_of_record.as_deref()) else {
        return (screen, false);
    };
    let shown = shown.as_deref();
    if !shown_differs(Some(record), shown) {
        return (screen, false);
    }
    let repair = repair(fixer, pane, record);
    let tense = if repair.state == FixState::Fixed {
        "held"
    } else {
        "holds"
    };
    let message = format!(
        "the pane's process {tense} {}, not the session of record {}",
        screen_text(shown),
        quoted(record)
    );
    let finding = Finding::about(FindingKind::ShownDrivenMismatch, pane, message)
        .with_session(shown)
        .with_fix(repair.state, repair.error);
    findings.push(finding);
    let relaunched = repair.relaunched;
    (repair.screen.unwrap_or(screen), relaunched)
}

/// What `--fix` did about the process's mismatch.
struct Repair {
    state: FixState,
    error: Option<FixError>,
    /// The session the relaunched process holds; `None` when nothing was relaunched.
    screen: Option<Screen>,
    /// A relaunch was attempted, so the chain writes no record of its own.
    relaunched: bool,
}

/// Plan the repair of a mismatch: fixable only while the process is idle; done only with
/// `--fix` and a relaunch to do it with, and on the orchestrator's pane only by a pass that
/// names it.
fn repair(fixer: &Fixer<'_>, pane: &Pane, record: &str) -> Repair {
    let request = fixer.request;
    let state = if fixer.activity != Some(Activity::Idle) {
        FixState::NotFixable
    } else if !request.fix {
        FixState::Fixable
    } else if pane.role == PaneRole::Orchestrator && request.pane != Some(&pane.name) {
        FixState::Skipped
    } else if let Some(relaunch) = fixer.relaunch {
        return relaunched(relaunch, pane, record);
    } else {
        FixState::NotFixable
    };
    Repair {
        state,
        error: None,
        screen: None,
        relaunched: false,
    }
}

/// Act and observe (I3): relaunch the pane on its session of record, then read what the record
/// the relaunch stored says it holds. Fixed only when the relaunch succeeded and its process
/// was seen holding `record`.
fn relaunched(relaunch: Relaunch<'_>, pane: &Pane, record: &str) -> Repair {
    let (state, error, screen) = match relaunch(pane) {
        Ok(stored) if stored.last_observed.shown.as_deref() == Some(record) => (
            FixState::Fixed,
            None,
            Some(Screen::Seen(stored.last_observed.shown)),
        ),
        Ok(stored) => {
            let held = stored.last_observed.shown;
            let message = format!(
                "after the relaunch on {}, the pane's process holds {}",
                quoted(record),
                screen_text(held.as_deref())
            );
            let code = FindingKind::ShownDrivenMismatch.code().to_owned();
            let error = FixError { code, message };
            (FixState::Failed, Some(error), Some(Screen::Seen(held)))
        }
        Err(error) => (FixState::Failed, Some(FixError::from(&error)), None),
    };
    Repair {
        state,
        error,
        screen,
        relaunched: true,
    }
}
