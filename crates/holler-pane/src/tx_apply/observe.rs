//! The up-front observation of `profile apply` (#664), and what it says of each recorded spec
//! pane's health. The parent module has the rules; this is how they are observed.
//!
//! Every call runs at once, one scoped thread each, the shape of `reconcile`'s `observe_all`:
//! Herdr's snapshot, and each recorded spec pane's `health` and `shown_session`. So the
//! observation takes as long as its slowest call, one port call's own bound (I5), whatever the
//! number of panes. It is apply's own fan-out and not `observe_all`, which is private to
//! `reconcile`, also calls `host.ps` and `list_sessions` (which a dry run must not), and turns a
//! failed call into a finding, where a failed `health` call here is a `failed` step. A thread that
//! cannot start or panics is a call that failed: `unavailable`, naming the call.

use std::thread::{self, Scope, ScopedJoinHandle};

use crate::error::{class_of, ErrorClass};
use crate::findings::{embedded, quoted};
use crate::reconcile::shown_differs;
use crate::{HerdrSnapshot, Pane, PaneError, Ports};

/// The calls of the observation, named as `<port>.<method>` in a message.
const HERDR_SNAPSHOT: &str = "herdr.snapshot";
const HARNESS_HEALTH: &str = "harness.health";
const HARNESS_SHOWN_SESSION: &str = "harness.shown_session";

/// What the observation saw.
pub(super) struct Observed {
    /// Herdr's snapshot, or why it could not be read.
    pub(super) snapshot: Result<HerdrSnapshot, PaneError>,
    /// One per record, in the order of the records observed.
    pub(super) panes: Vec<Seen>,
}

/// What one recorded pane's two calls answered.
pub(super) struct Seen {
    health: Result<bool, PaneError>,
    shown: Result<Option<String>, PaneError>,
}

/// A recorded pane's health, as §2 observes it.
pub(super) enum Judged {
    /// Healthy, with what could not be told (an unreadable shown session), if anything.
    Healthy { note: Option<String> },
    /// Not healthy, and why.
    Unhealthy(String),
}

/// Observe Herdr and every one of `records` at once (see the module docs). Each call is joined
/// before this returns, so no step runs while an observation is outstanding (C1).
pub(super) fn observe(ports: Ports<'_>, records: &[Pane]) -> Observed {
    thread::scope(|threads| {
        let snapshot = spawn(threads, move || ports.herdr.snapshot());
        let health: Vec<_> = records
            .iter()
            .map(|pane| spawn(threads, move || ports.harness.health(pane.harness.port)))
            .collect();
        let shown: Vec<_> = records
            .iter()
            .map(|pane| {
                spawn(threads, move || {
                    ports.harness.shown_session(&pane.herdr.pane_id)
                })
            })
            .collect();
        let panes = records
            .iter()
            .zip(health.into_iter().zip(shown))
            .map(|(pane, (health, shown))| Seen {
                health: joined(health).unwrap_or_else(|| Err(lost(HARNESS_HEALTH, Some(pane)))),
                shown: joined(shown)
                    .unwrap_or_else(|| Err(lost(HARNESS_SHOWN_SESSION, Some(pane)))),
            })
            .collect();
        Observed {
            snapshot: joined(snapshot).unwrap_or_else(|| Err(lost(HERDR_SNAPSHOT, None))),
            panes,
        }
    })
}

/// Start `call` on a thread of the scope.
fn spawn<'scope, T: Send + 'scope>(
    threads: &'scope Scope<'scope, '_>,
    call: impl FnOnce() -> T + Send + 'scope,
) -> std::io::Result<ScopedJoinHandle<'scope, T>> {
    thread::Builder::new().spawn_scoped(threads, call)
}

/// What a thread returned, or `None` when it could not start or panicked.
fn joined<T>(thread: std::io::Result<ScopedJoinHandle<'_, T>>) -> Option<T> {
    thread.ok()?.join().ok()
}

/// A call whose thread did not finish: `unavailable`, naming the call and its pane.
fn lost(call: &str, pane: Option<&Pane>) -> PaneError {
    let of = pane.map_or_else(String::new, |pane| format!(" of {}", pane.name));
    PaneError::Unavailable {
        what: format!("{call}{of} was not observed: its check did not finish"),
    }
}

/// What `seen` and the snapshot say of `record`'s health (§2, C9, C12). `Err` when a call its
/// row needs failed (a `failed` step, C2 and C3): its `health`, or Herdr's snapshot, which every
/// recorded pane that row a does not refuse needs.
pub(super) fn judge(
    record: &Pane,
    seen: &Seen,
    snapshot: &Result<HerdrSnapshot, PaneError>,
) -> Result<Judged, PaneError> {
    let answers = seen
        .health
        .clone()
        .map_err(|error| as_failure(HARNESS_HEALTH, record, error))?;
    let snapshot = snapshot
        .as_ref()
        .map_err(|error| as_failure(HERDR_SNAPSHOT, record, error.clone()))?;
    if !answers {
        let port = record.harness.port;
        return Ok(Judged::Unhealthy(format!(
            "its harness server on port {port} did not answer"
        )));
    }
    if !listed(snapshot, record) {
        return Ok(Judged::Unhealthy(format!(
            "Herdr did not list its pane {} in session {}",
            quoted(record.herdr.pane_id.as_str()),
            quoted(&record.herdr.session)
        )));
    }
    Ok(shown(record, &seen.shown))
}

/// §2's third condition, from the shown session alone (C12, the decision on O2): only a session
/// other than the session of record makes the pane unhealthy; `Ok(None)` and an error cannot
/// tell, which the step's detail says. Every reason is in the past tense: it is what the
/// observation saw, before the step ran.
fn shown(record: &Pane, shown: &Result<Option<String>, PaneError>) -> Judged {
    let of_record = record.session_of_record.as_deref();
    let untold = match shown {
        Ok(Some(session)) if shown_differs(of_record, Some(session.as_str())) => {
            let record_text = of_record.map_or_else(|| "none".to_owned(), quoted);
            return Judged::Unhealthy(format!(
                "its TUI showed session {}, not its session of record {record_text}",
                quoted(session)
            ));
        }
        Ok(Some(_)) => None,
        Ok(None) => Some("the TUI showed no session: its home screen, or no TUI".to_owned()),
        Err(error) => Some(format!("{HARNESS_SHOWN_SESSION} failed ({})", error.code())),
    };
    let note = untold.zip(of_record).map(|(why, of_record)| {
        format!(
            "could not tell whether its TUI shows its session of record {} ({why}); not acted on",
            quoted(of_record)
        )
    });
    Judged::Healthy { note }
}

/// Whether Herdr's snapshot lists `record`'s Herdr pane: the pair of its Herdr session and pane
/// id, the join key `tx_launch` and `reconcile` use. Never the cell, nor the label Herdr reports
/// (observed, never recorded: ADR-0021 section 2, #808).
fn listed(snapshot: &HerdrSnapshot, record: &Pane) -> bool {
    snapshot
        .panes
        .iter()
        .any(|herdr| herdr.session == record.herdr.session && herdr.pane_id == record.herdr.pane_id)
}

/// A failed call of `record`'s observation, as its `failed` step's error. §2 makes every error of
/// `health` a failed step, whatever its class (C9), and a failed step exits 1 (§5), so an error
/// that is not failure class becomes `unavailable` naming the call: an envelope's code has the
/// class of its exit code. A failure-class error is kept as it is.
fn as_failure(call: &str, record: &Pane, error: PaneError) -> PaneError {
    if class_of(error.code()) == ErrorClass::Failure {
        return error;
    }
    PaneError::Unavailable {
        what: format!(
            "{call} for {} answered {}: {}",
            record.name,
            error.code(),
            embedded(&error.to_string())
        ),
    }
}
