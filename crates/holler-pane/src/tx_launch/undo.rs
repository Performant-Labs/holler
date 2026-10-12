//! What a relaunch stops before it brings the pane up again (B1 and B2), and what a run undoes
//! when a step fails: the rollback of what the act made, and a relaunch's close of its old
//! Herdr pane after a move (B10). Nothing here starts anything.

use crate::findings::{embedded, quoted};
use crate::{HerdrPane, Pane, PaneError, PaneId, PaneName, Ports};

use super::{cell, refused, same_pane, unavailable, Budget, Plan, Seen};

/// B1 and B2 of a relaunch: stop what the pane owns, its old harness among them, then check
/// that the old harness no longer answers at its recorded endpoint: a server on its port, or
/// the process by its pid. One that still answers fails the run, loudly, and is never adopted.
pub(super) fn stop_old(
    ports: Ports<'_>,
    name: &PaneName,
    old: &Pane,
    budget: &Budget,
) -> Result<(), PaneError> {
    budget.check()?;
    ports.host.stop_owned(name)?;
    budget.check()?;
    let at = old.harness.endpoint(name)?;
    if ports.harness.health(&at)? {
        let what = match at.port() {
            Ok(port) => format!(
                "the harness server on port {port} still answers after the processes of {name} \
                 were stopped; a running server is never adopted"
            ),
            Err(_) => format!(
                "the harness {at} still runs after the processes of {name} were stopped; a \
                 running process is never adopted"
            ),
        };
        return Err(unavailable(what));
    }
    Ok(())
}

/// What the act made, so that a rollback undoes that and nothing else.
#[derive(Default)]
pub(super) struct Made {
    /// The tmux session's step ran, so its processes, the harness among them, are stopped.
    pub(super) session: bool,
    /// The Herdr pane this run created (the plan's snapshot did not list it), so it is closed.
    pub(super) pane: Option<PaneId>,
}

/// Undo what `made` says this run made, best effort and in order: stop the tmux session's
/// processes, then close the Herdr pane it created. A rollback call that fails does not hide
/// `error`: its message gains `; rollback failed: <call> (<code>)`.
pub(super) fn roll_back(
    ports: Ports<'_>,
    name: &PaneName,
    made: &Made,
    error: PaneError,
) -> PaneError {
    let mut failed = Vec::new();
    if made.session {
        if let Err(stop) = ports.host.stop_owned(name) {
            failed.push(format!("host.stop_owned ({})", stop.code()));
        }
    }
    if let Some(pane) = &made.pane {
        if let Err(close) = closed(ports.herdr.close(pane)) {
            failed.push(format!("herdr.close ({})", close.code()));
        }
    }
    if failed.is_empty() {
        return error;
    }
    with_note(error, &format!("rollback failed: {}", failed.join(", ")))
}

/// B10, after R and only on a move: close the old Herdr pane, when the plan's snapshot listed it
/// and it is not the pane the record now names. A close that fails never fails the act (the
/// record names the pane that holds the TUI, decision 23); its error is the run's failure.
pub(super) fn close_old(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    old: &Pane,
    now: &HerdrPane,
) -> Option<PaneError> {
    let moved = plan.target.workspace != old.herdr.workspace || plan.target.grid != old.herdr.grid;
    let listed = seen.listed.iter().any(|p| same_pane(p, &old.herdr));
    if !moved || !listed || same_pane(&old.herdr, now) {
        return None;
    }
    let error = closed(ports.herdr.close(&old.herdr.pane_id)).err()?;
    Some(unavailable(format!(
        "relaunched {} at {}, but its old Herdr pane {} was not closed ({}: {})",
        plan.name,
        cell(&now.workspace, now.grid),
        quoted(old.herdr.pane_id.as_str()),
        error.code(),
        embedded(&error.to_string())
    )))
}

/// A close's answer, with `pane-not-found` counted as closed: a vanished pane is already gone.
fn closed(answer: Result<(), PaneError>) -> Result<(), PaneError> {
    match answer {
        Err(PaneError::PaneNotFound { .. }) => Ok(()),
        other => other,
    }
}

/// `error` with `; <note>` added to its message, its code kept: through the one payload that
/// travels beside the code (`PaneError::detail`, which the crate's own `from_wire` turns back
/// into the variant), or a `Refused`'s message. A variant with no payload is returned as it is.
fn with_note(error: PaneError, note: &str) -> PaneError {
    let noted = match &error {
        PaneError::Refused { code, message } => {
            Some(refused(code.clone(), format!("{message}; {note}")))
        }
        other => other.detail().map(|detail| {
            let detail = Some(format!("{detail}; {note}"));
            PaneError::from_wire(other.code().to_owned(), String::new(), detail)
        }),
    };
    noted.unwrap_or(error)
}
