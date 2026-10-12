//! The single process's steps of a launch and a relaunch (#831, ADR-0021 section 15 item 6): the
//! act of a pane whose harness's shape is `SingleProcess` (Claude Code), which has no server, no
//! session to create and no TUI to attach.
//!
//! The plan built the process's argv (`HarnessPort::process_command`, pure), holding the session
//! the run's `ProcessSession` chose, and refused a launch into a tmux session that already runs
//! Claude Code ([`refuse_in_use`], `process-in-use`, plan review W11): a process a crash left
//! behind keeps running, and spending, and is never adopted. The act, each step after a budget
//! check (I5):
//!
//! 1. **The run**: `HostPort::run` starts exactly the planned argv in the pane's tmux session,
//!    in the spec's directory, so `HostPort::stop_owned` stops it (a relaunch's B1 already did).
//! 2. **The wait**: through the engine's one wait (`wait_until`), every `boot_poll` until the
//!    budget ends, the harness must locate the process holding the session
//!    (`locate_process`) and report it idle (`session_activity`). The pid it is located by is
//!    the process's identity from then on, and the record's. A harness that answers the wait
//!    with an error (Claude Code's `harness-dialog-open` for a process held by a dialog a person
//!    must answer) fails the run, and one never located or never idle ends it as the budget's
//!    `timeout`; the rollback stops the process either way.
//! 3. **The observation**: the session the process is seen holding (`shown_session` through
//!    its endpoint) must be the planned session; anything else is `unavailable`.

use std::time::{Duration, Instant};

use crate::error::RefusalCode;
use crate::findings::quoted;
use crate::pane::is_claude_code_command;
use crate::ports::Activity;
use crate::reconcile::shown_differs;
use crate::{HarnessEndpoint, HarnessPort, HerdrPane, PaneError, PaneName, Ports};

use super::{refused, unavailable, wait_until, Budget, Live, Plan, Seen};

/// `process-in-use`: a launch's tmux session already runs Claude Code, which no record names
/// (a crash can leave one behind); a running process is never adopted (#831, plan review W11),
/// as `port-in-use` refuses a server left behind. A refusal, exit 3.
pub const PROCESS_IN_USE: RefusalCode = RefusalCode::from_static("process-in-use");

/// Wait for the single process of `name` holding `session` to be located: ask
/// `harness.locate_process` every `poll`, through the engine's one wait ([`wait_until`]), until
/// it answers a pid (`Ok(Some(pid))`) or `deadline` passes (`Ok(None)`). A `locate_process`
/// error is returned as it is.
///
/// The test kit's Claude Code conformance suite and the adapter's opt-in real test wait with
/// it; the act waits with the same loop for the process to be located and idle.
pub fn wait_located(
    harness: &dyn HarnessPort,
    name: &PaneName,
    session: &str,
    poll: Duration,
    deadline: Instant,
) -> Result<Option<u32>, PaneError> {
    wait_until(poll, deadline, || harness.locate_process(name, session))
}

/// The plan's refusal of a launch into a tmux session whose foreground processes include one
/// that `is_claude_code_command` names (#835's signal). A session that does not exist yet runs
/// nothing.
pub(super) fn refuse_in_use(ports: Ports<'_>, name: &PaneName) -> Result<(), PaneError> {
    let names = match ports.host.foreground_commands(name) {
        Ok(names) => names,
        Err(PaneError::PaneNotFound { .. }) => return Ok(()),
        Err(error) => return Err(error),
    };
    if !names.iter().any(|command| is_claude_code_command(command)) {
        return Ok(());
    }
    Err(refused(
        PROCESS_IN_USE,
        format!(
            "the tmux session {name} already runs Claude Code, which no record names; a running \
             process is never adopted, so stop it, or close the session, and launch again"
        ),
    ))
}

/// The single process's steps of the act, 1 to 3 of the module docs, in the Herdr pane
/// `herdr`: what the process brought up and showed, with its pid.
pub(super) fn bring_up(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    poll: Duration,
    herdr: HerdrPane,
) -> Result<Live, PaneError> {
    let session = seen.held.clone().ok_or_else(|| {
        unavailable(format!(
            "the plan of {} built no session for its process",
            plan.name
        ))
    })?;
    budget.check()?;
    ports
        .host
        .run(plan.name, &plan.spec.host.cwd, &seen.command)?;
    // No budget check here: the wait checks the budget's end before every ask, its first included.
    let pid = wait_idle(ports.harness, plan.name, &session, poll, budget.deadline())?
        .ok_or_else(|| budget.expired())?;
    let at = HarnessEndpoint::Process {
        pane: plan.name.clone(),
        pid,
    };
    budget.check()?;
    let shown = ports.harness.shown_session(&herdr.pane_id, &at)?;
    if shown_differs(Some(&session), shown.as_deref()) {
        let held = shown
            .as_deref()
            .map_or_else(|| "no session".to_owned(), quoted);
        return Err(unavailable(format!(
            "the harness {at} holds {held}, not the session {}",
            quoted(&session)
        )));
    }
    Ok(Live {
        herdr,
        session,
        shown,
        pid: Some(pid),
    })
}

/// Step 2: the pid of the process of `name` holding `session` once it is located and idle, or
/// `None` when `deadline` passed first.
fn wait_idle(
    harness: &dyn HarnessPort,
    name: &PaneName,
    session: &str,
    poll: Duration,
    deadline: Instant,
) -> Result<Option<u32>, PaneError> {
    wait_until(poll, deadline, || {
        let Some(pid) = harness.locate_process(name, session)? else {
            return Ok(None);
        };
        let at = HarnessEndpoint::Process {
            pane: name.clone(),
            pid,
        };
        let idle = harness.session_activity(&at, session)? == Activity::Idle;
        Ok(idle.then_some(pid))
    })
}
