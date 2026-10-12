//! The harness server's steps of a launch and a relaunch (#802), the TUI's (#644), and the one
//! health wait: the act of a pane whose harness's shape is `ServerAndTui` (OpenCode).
//!
//! The plan built the server's argv (`HarnessPort::serve_command`, pure). The act runs exactly
//! that argv in the pane's tmux session through `HostPort::run`, the one place that starts and
//! stops a pane's processes, so `HostPort::stop_owned` stops the server too (TERM, a grace,
//! then KILL) and the engine records no pid of it. Each step after a budget check (I5):
//!
//! 1. **The port again.** A server that started answering on the port since the plan checked
//!    it is refused (`port-in-use`), never adopted and never stopped: with the plan's check,
//!    this is what keeps a launch from taking over a server it did not start.
//! 2. **The run**, in the spec's directory.
//! 3. **The health wait** ([`wait_healthy`]): the server is asked every `boot_poll` until it
//!    answers or the run's budget ends. The budget's end is the wait's deadline, checked
//!    before every ask, the first one after the run included, so each ask is a live step
//!    with the budget checked before it and none starts once the budget has run out. A
//!    server that never answers ends the run as the budget's `timeout` (`pane.launch` or
//!    `pane.relaunch`). A server that exits at once is seen the same way: the wait has no
//!    process to watch.
//! 4. **The spec**: the harness says whether the running server honours the plan's
//!    `ServeSpec` (`verify_serving`), before any session is created on it. A refusal is the
//!    run's failure, and the rollback stops the server. It answers for the spec, not for the
//!    server being up: a server gone since the wait fails the next step's call to it (the
//!    session of record's), and that failure rolls back the same way.
//! 5. **The session of record** (A6, B8 for a relaunch): the record's session of record when
//!    the restarted server still lists it, else a new one created through the harness API
//!    (decision 8).
//! 6. **The TUI** (A7, O1): attached to exactly that session, then seen showing it
//!    (`shown_differs`, the one form of that comparison); anything else is `unavailable`.

use std::time::{Duration, Instant};

use crate::findings::quoted;
use crate::reconcile::shown_differs;
use crate::{HarnessEndpoint, HarnessPort, HerdrPane, PaneError, Ports};

use super::{refused, unavailable, wait_until, Budget, Live, Plan, Seen, PORT_IN_USE};

/// Wait for the harness at `at` to answer: ask `harness.health` every `poll` until it answers
/// `true` (`Ok(true)`) or `deadline` passes (`Ok(false)`), through the engine's one wait
/// ([`wait_until`]). A `health` error is returned as it is. So a call takes at most until
/// `deadline` plus one `health` call's own bound.
///
/// The launch engine's wait for a server, and the one the test kit's `HarnessPort` conformance
/// suite and the OpenCode adapter's opt-in real tests wait with.
pub fn wait_healthy(
    harness: &dyn HarnessPort,
    at: &HarnessEndpoint,
    poll: Duration,
    deadline: Instant,
) -> Result<bool, PaneError> {
    let answered = wait_until(poll, deadline, || Ok(harness.health(at)?.then_some(())))?;
    Ok(answered.is_some())
}

/// The endpoint of the server on `port`.
pub(super) fn endpoint(port: u16) -> HarnessEndpoint {
    HarnessEndpoint::Port(port)
}

/// The server's steps of the act, 1 to 6 of the module docs, in the Herdr pane `herdr`: what
/// the server and its TUI brought up and showed.
pub(super) fn bring_up(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    poll: Duration,
    herdr: HerdrPane,
    port: u16,
) -> Result<Live, PaneError> {
    let at = endpoint(port);
    start(ports, plan, seen, budget, poll, &at, port)?;
    budget.check()?;
    let session = session_of_record(ports, plan, &at)?;
    budget.check()?;
    ports.harness.attach_tui(&herdr.pane_id, &at, &session)?;
    budget.check()?;
    let shown = ports.harness.shown_session(&herdr.pane_id, &at)?;
    if shown_differs(Some(&session), shown.as_deref()) {
        let seen = shown
            .as_deref()
            .map_or_else(|| "no session".to_owned(), quoted);
        return Err(unavailable(format!(
            "the TUI of {} shows {seen}, not the session of record {}",
            plan.name,
            quoted(&session)
        )));
    }
    Ok(Live {
        herdr,
        session,
        shown,
        pid: None,
    })
}

/// Steps 1 to 4: the port is checked again, the planned argv runs in the spec's directory, the
/// server's health is waited for within what is left of the budget, and the harness says
/// whether the running server honours the plan's `ServeSpec`.
fn start(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    poll: Duration,
    at: &HarnessEndpoint,
    port: u16,
) -> Result<(), PaneError> {
    budget.check()?;
    if ports.harness.health(at)? {
        return Err(port_in_use(port));
    }
    budget.check()?;
    ports
        .host
        .run(plan.name, &plan.spec.host.cwd, &seen.command)?;
    // No budget check here: the wait checks the budget's end before every ask, its first included.
    if !wait_healthy(ports.harness, at, poll, budget.deadline())? {
        return Err(budget.expired());
    }
    budget.check()?;
    ports.harness.verify_serving(at, &plan.serve)
}

/// Step 5: a new session, created through the harness API. For a relaunch: the record's session
/// of record when the restarted server still lists it, else a new one (decision 8).
fn session_of_record(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    at: &HarnessEndpoint,
) -> Result<String, PaneError> {
    if let Some(kept) = plan.old.and_then(|old| old.session_of_record.as_deref()) {
        let listed = ports.harness.list_sessions(at)?;
        if listed.iter().any(|session| session == kept) {
            return Ok(kept.to_owned());
        }
    }
    ports.harness.create_session(at)
}

/// `port-in-use`: a server answers on `port`, and a running server is never adopted. The plan
/// asks before anything moves, and the act again just before the server's run.
pub(super) fn port_in_use(port: u16) -> PaneError {
    let message = format!(
        "a harness server already answers on port {port}; a running server is never adopted, \
         so stop it or choose another port"
    );
    refused(PORT_IN_USE, message)
}
