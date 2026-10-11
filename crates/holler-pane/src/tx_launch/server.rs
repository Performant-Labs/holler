//! The harness server's steps of a launch and a relaunch, and the one health wait (#802).
//!
//! The plan built the server's argv (`HarnessPort::serve_command`, pure). The act runs exactly
//! that argv in the pane's tmux session through `HostPort::run`, the one place that starts and
//! stops a pane's processes, so `HostPort::stop_owned` stops the server too (TERM, a grace,
//! then KILL) and the engine records no pid of it. Each step after a budget check (I5):
//!
//! - **The old server** ([`stop_old`], a relaunch's B1 and B2): what the pane owns is stopped,
//!   and a server that still answers on the old port fails the run, loudly, never adopted.
//! - **The new one** ([`start`]), after the tmux session exists and the spec's command runs:
//!   1. **The port again.** A server that started answering on the port since the plan checked
//!      it is refused (`port-in-use`), never adopted and never stopped: with the plan's check,
//!      this is what keeps a launch from taking over a server it did not start.
//!   2. **The run**, in the spec's directory.
//!   3. **The health wait** ([`wait_healthy`]): the server is asked every `boot_poll` until it
//!      answers or the run's budget ends. The budget's end is the wait's deadline, checked
//!      before every ask, the first one after the run included, so each ask is a live step
//!      with the budget checked before it and none starts once the budget has run out. A
//!      server that never answers ends the run as the budget's `timeout` (`pane.launch` or
//!      `pane.relaunch`). A server that exits at once is seen the same way: the wait has no
//!      process to watch.
//!   4. **The spec**: `verify_serving` asks the running server whether it honours the plan's
//!      `ServeSpec`, before any session is created on it. A refusal is the run's failure, and
//!      the rollback stops the server.

use std::thread;
use std::time::{Duration, Instant};

use crate::{HarnessPort, Pane, PaneError, PaneName, Ports};

use super::{port_in_use, unavailable, Budget, Plan, Seen};

/// Wait for the harness server on `port` to answer: ask `harness.health` every `poll` until it
/// answers `true` (`Ok(true)`) or `deadline` passes (`Ok(false)`). A `health` error is returned
/// as it is. The deadline is checked before every ask, the first one included, and no ask
/// starts once it has passed: a deadline already past asks nothing, and one that comes during
/// a sleep ends the sleep and the wait. So a call takes at most until `deadline` plus one
/// `health` call's own bound.
///
/// The launch engine's wait, and the one the test kit's `HarnessPort` conformance suite and the
/// OpenCode adapter's opt-in real tests wait with.
pub fn wait_healthy(
    harness: &dyn HarnessPort,
    port: u16,
    poll: Duration,
    deadline: Instant,
) -> Result<bool, PaneError> {
    loop {
        if Instant::now() >= deadline {
            return Ok(false);
        }
        if harness.health(port)? {
            return Ok(true);
        }
        thread::sleep(deadline.saturating_duration_since(Instant::now()).min(poll));
    }
}

/// B1 and B2 of a relaunch: stop what the pane owns, its old server among them, then check
/// that the old server no longer answers. One that still answers fails the run, loudly, and
/// is never adopted.
pub(super) fn stop_old(
    ports: Ports<'_>,
    name: &PaneName,
    old: &Pane,
    budget: &Budget,
) -> Result<(), PaneError> {
    budget.check()?;
    ports.host.stop_owned(name)?;
    budget.check()?;
    let port = old.harness.port;
    if ports.harness.health(port)? {
        return Err(unavailable(format!(
            "the harness server on port {port} still answers after the processes of {name} \
             were stopped; a running server is never adopted"
        )));
    }
    Ok(())
}

/// The new server's step of the act (see the module docs): the port is checked again, the
/// planned argv runs in the spec's directory, the server's health is waited for within what is
/// left of the budget, and the running server is asked whether it honours the plan's
/// `ServeSpec`.
pub(super) fn start(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    poll: Duration,
) -> Result<(), PaneError> {
    budget.check()?;
    if ports.harness.health(plan.port)? {
        return Err(port_in_use(plan.port));
    }
    budget.check()?;
    ports
        .host
        .run(plan.name, &plan.spec.host.cwd, &seen.server)?;
    // No budget check here: the wait checks the budget's end before every ask, its first included.
    if !wait_healthy(ports.harness, plan.port, poll, budget.deadline())? {
        return Err(budget.expired());
    }
    budget.check()?;
    ports.harness.verify_serving(plan.port, &plan.serve)
}
