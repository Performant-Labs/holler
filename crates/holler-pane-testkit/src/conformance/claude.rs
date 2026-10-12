//! The Claude Code conformance suite (#831, ADR-0021 section 15 item 15; plan review W1 and
//! W12): [`run_claude_conformance`] runs the cases [`claude_cases`] lists against any
//! single-process `HarnessPort`, each against a fresh harness and the [`ClaudeRig`] that comes
//! with it. `FakeClaude` passes it, the router of the two fakes passes it, and the Claude Code
//! adapter passes it against a scratch Claude Code (opt-in): the suite is the guard of what the
//! spike verified, the delivery frame included, which is read from the client and not
//! documented.
//!
//! The cases pin, in order:
//!
//! 1. `process_command` starts nothing: the session it names is not located after it;
//! 2. the process its argv starts, once launched, is located holding the session, within
//!    [`LOCATE_WITHIN`] (`holler_pane::tx_launch::wait_located`, every `BOOT_POLL`);
//! 3. two fresh session ids differ;
//! 4. a located process is healthy, and idle within [`SETTLE_WITHIN`];
//! 5. the session it is seen holding (`shown_session`, through its endpoint) is its session;
//! 6. a resume of a session with no conversation is `session-not-found`, so a switch to an
//!    unknown id starts nothing (W1);
//! 7. a keep of a session with no conversation is an argv that starts the process on it (the
//!    resume-only refusal of case 6 is not a keep's);
//! 8. a prompt to a session the process does not hold is `session-mismatch`;
//! 9. a prompt delivered at `send_prompt` starts the session's conversation, so the process,
//!    stopped and resumed, holds the same session again (W12): the frame reached the model,
//!    since only a prompted session has a transcript to resume;
//! 10. a stopped process is not healthy;
//! 11. `abort` is `abort-unsupported` (there is no in-place abort, and a signal is not one);
//! 12. `login_status` answers.
//! 13. `agent_view` of a process endpoint is the empty view (#647 part 2, ADR-0021 section 12):
//!     a single process has no server to list agents or turns, so it cannot tell, and says so
//!     with an empty view, never a guess. Run over the router of the two harnesses, it also
//!     pins that a process endpoint's `agent_view` reaches the process harness.
//!
//! **The bounds.** A launched process must be located within [`LOCATE_WITHIN`], half the launch
//! engine's `DEFAULT_BUDGET`, which must also hold the Herdr and tmux steps of a launch; what a
//! case waits for after that (an idle status, a conversation after a prompt, a process gone after
//! a stop) within [`SETTLE_WITHIN`]. A harness that never answers one of them fails its case
//! when the bound passes, so a broken harness costs the suite at most those bounds.
//!
//! **How a case starts and stops a process.** `process_command` is pure: it builds the argv,
//! which the host's `run` starts in the pane's tmux session and its `stop_owned` stops. So the
//! caller passes a launcher and a stopper, generic over the subject, as the server suite takes a
//! launcher (#802). A real adapter's case sends at most one model turn (case 9).

use std::time::{Duration, Instant};

use holler_pane::ports::{Activity, SessionChoice, ABORT_UNSUPPORTED, SESSION_MISMATCH};
use holler_pane::tx_launch::{wait_located, wait_until, BOOT_POLL};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
};

use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn HarnessPort, &Hands<'_>, &ClaudeRig) -> Result<(), String>;

/// The caller's launcher and stopper, bound to the harness of the case: the launcher starts the
/// process an argv of `process_command` describes, for the pane it names; the stopper stops
/// every process of a pane.
struct Hands<'a> {
    launch: &'a dyn Fn(&PaneName, &Argv) -> Result<(), PaneError>,
    stop: &'a dyn Fn(&PaneName) -> Result<(), PaneError>,
}

const NOT_FOUND: &str = "session-not-found";

/// How long a launched process may take to be located: half the launch engine's budget
/// (`holler_pane::tx_launch::DEFAULT_BUDGET`, 10 s), which also holds a launch's Herdr and tmux
/// steps.
pub const LOCATE_WITHIN: Duration = Duration::from_secs(5);

/// How long a located process may take to settle into what a case waits for next: idle, a
/// conversation once prompted, gone once stopped.
pub const SETTLE_WITHIN: Duration = Duration::from_secs(3);

/// The one prompt a case sends (case 9): short, so a real model's turn is short.
const PROMPT: &str = "Reply with the single word: ok";

/// The pid case 11 names for a process it does not launch: abort is refused before any
/// process is looked for.
const ABORTED_PID: u32 = 1;

/// The panes the cases launch in, and their Herdr session: neutral names, never a live
/// session's.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";
const SCRATCH: &str = "scratch";

/// The directory of the sample rig's processes.
const SAMPLE_CWD: &str = "/srv/demo";

/// What a single-process harness under test gives the suite for one case: two panes with no
/// process yet, their Herdr pane ids, and the directory every process works in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeRig {
    /// Two panes whose tmux sessions run no process yet.
    pub panes: [PaneName; 2],
    /// Their Herdr pane ids, in the same order.
    pub pane_ids: [PaneId; 2],
    /// The directory every process works in (a real Claude Code must already trust it).
    pub cwd: String,
}

impl ClaudeRig {
    /// Panes `demo-c1r1` and `demo-c2r1`, their pane ids `scratch:<name>` (the fixture's
    /// form), and the directory `/srv/demo`. Enough for the fake; a real adapter supplies its
    /// own directory.
    pub fn sample() -> Self {
        Self {
            panes: [constant_name(C1), constant_name(C2)],
            pane_ids: [
                PaneId::new(format!("{SCRATCH}:{C1}")),
                PaneId::new(format!("{SCRATCH}:{C2}")),
            ],
            cwd: SAMPLE_CWD.to_owned(),
        }
    }
}

/// `text`, one of this module's constant pane names, as a [`PaneName`].
// Each constant is a well-formed pane name, so the parse is infallible.
#[allow(clippy::expect_used)] // #831
fn constant_name(text: &str) -> PaneName {
    PaneName::parse(text).expect("a constant pane name of the suite")
}

/// The suite, in order: the one table that the runner iterates and [`claude_cases`] lists.
const CASES: [(&str, Case); 13] = [
    (
        "process-command-starts-nothing",
        process_command_starts_nothing,
    ),
    ("launch-then-located", launch_then_located),
    ("new-session-ids-are-distinct", new_session_ids_are_distinct),
    (
        "located-process-is-healthy-and-idle",
        located_process_is_healthy_and_idle,
    ),
    ("shown-is-the-held-session", shown_is_the_held_session),
    (
        "resume-without-conversation-is-session-not-found",
        resume_without_conversation_is_session_not_found,
    ),
    (
        "keep-without-conversation-starts-it",
        keep_without_conversation_starts_it,
    ),
    (
        "prompt-to-another-session-is-session-mismatch",
        prompt_to_another_session_is_session_mismatch,
    ),
    (
        "resume-after-a-prompt-keeps-the-session",
        resume_after_a_prompt_keeps_the_session,
    ),
    (
        "stopped-process-is-not-healthy",
        stopped_process_is_not_healthy,
    ),
    ("abort-is-unsupported", abort_is_unsupported),
    ("login-status-answers", login_status_answers),
    (
        "agent-view-of-a-process-is-empty",
        agent_view_of_a_process_is_empty,
    ),
];

/// The ids of the cases [`run_claude_conformance`] runs, in the order it runs them.
pub fn claude_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`claude_cases`], in order, each against a fresh harness, and return
/// every case that did not hold.
///
/// `fresh` is called once per case. It returns the harness, the rig the case runs on and a
/// guard that the suite keeps alive for that case only (a real adapter's guard ends the case's
/// tmux sessions); the harness is dropped before its guard. No process may run in the rig's
/// panes yet.
///
/// `launch` starts the process an argv the harness's `process_command` built, for the pane the
/// argv was built for, as the host's `run` would; `stop` stops every process of a pane, as the
/// host's `stop_owned` would. How each implementation runs the suite (the test kit cannot name
/// an adapter):
///
/// ```text
/// // the fake: its scenario methods start and stop the process.
/// let launch = |h: &FakeClaude, _: &PaneName, argv: &Argv| match h.start(argv) {
///     Some(started) => started.map(drop),
///     None => Err(PaneError::Usage { message: "not the fake's argv".to_owned() }),
/// };
/// let stop = |h: &FakeClaude, pane: &PaneName| { h.stop(pane); Ok(()) };
/// assert_eq!(
///     run_claude_conformance(|| (FakeClaude::new(), ClaudeRig::sample(), ()), launch, stop),
///     Ok(())
/// );
/// // holler-adapter-claude (opt-in): a scratch tmux server, the argv run as each pane's
/// //   session, and kill-session as the stopper; a directory Claude Code already trusts.
/// ```
pub fn run_claude_conformance<S, K, F, L, T>(mut fresh: F, launch: L, stop: T) -> Conformance
where
    S: HarnessPort,
    F: FnMut() -> (S, ClaudeRig, K),
    L: Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>,
    T: Fn(&S, &PaneName) -> Result<(), PaneError>,
{
    run_cases(
        &CASES,
        || {
            let (harness, rig, guard) = fresh();
            ((harness, rig), guard)
        },
        |case, (harness, rig)| {
            let start = |name: &PaneName, argv: &Argv| launch(harness, name, argv);
            let end = |name: &PaneName| stop(harness, name);
            let hands = Hands {
                launch: &start,
                stop: &end,
            };
            case(harness, &hands, rig)
        },
    )
}

// --- the cases ---

/// Case 1: `process_command` starts nothing: the session it names is not located after it.
fn process_command_starts_nothing(
    h: &dyn HarnessPort,
    _: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let pane = &rig.panes[0];
    let id = new_id(h)?;
    command(h, rig, pane, &SessionChoice::Fresh(id.clone()))?;
    let located = succeeds("locate_process", h.locate_process(pane, &id))?;
    expect_eq(
        "locate_process after process_command, before the launcher (it starts nothing)",
        located,
        None,
    )
}

/// Case 2: the process an argv starts is located holding its session, within the budget.
fn launch_then_located(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    launched(h, hands, rig, &SessionChoice::Fresh(id)).map(drop)
}

/// Case 3: two `new_session_id` calls return two distinct, non-empty ids.
fn new_session_ids_are_distinct(
    h: &dyn HarnessPort,
    _: &Hands<'_>,
    _: &ClaudeRig,
) -> Result<(), String> {
    let (a, b) = (new_id(h)?, new_id(h)?);
    if a.is_empty() || b.is_empty() || a == b {
        return Err(format!(
            "two new_session_id calls must return distinct, non-empty ids; got {a:?} and {b:?}"
        ));
    }
    Ok(())
}

/// Case 4: a located process is healthy, and idle within the budget.
fn located_process_is_healthy_and_idle(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let at = launched(h, hands, rig, &SessionChoice::Fresh(id.clone()))?;
    let healthy = succeeds(&format!("health of {at}"), h.health(&at))?;
    expect_eq(&format!("health of the located {at}"), healthy, true)?;
    let idle = wait_until(BOOT_POLL, Instant::now() + SETTLE_WITHIN, || {
        let activity = h.session_activity(&at, &id)?;
        Ok((activity == Activity::Idle).then_some(()))
    });
    match succeeds(&format!("session_activity of {at}"), idle)? {
        Some(()) => Ok(()),
        None => Err(format!(
            "the located {at} did not report itself idle within {SETTLE_WITHIN:?}"
        )),
    }
}

/// Case 5: the session a located process is seen holding, through its endpoint, is its session.
fn shown_is_the_held_session(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let at = launched(h, hands, rig, &SessionChoice::Fresh(id.clone()))?;
    let shown = succeeds(
        &format!("shown_session of {at}"),
        h.shown_session(&rig.pane_ids[0], &at),
    )?;
    expect_eq(
        &format!("the session {at} is seen holding"),
        shown,
        Some(id),
    )
}

/// Case 6 (W1): a resume of a session that has no conversation is `session-not-found`, so a
/// switch to an id that exists nowhere starts nothing.
fn resume_without_conversation_is_session_not_found(
    h: &dyn HarnessPort,
    _: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let call = "process_command of a resume of a session with no conversation";
    let answer = h.process_command(&rig.panes[0], &spec(rig), &SessionChoice::Resume(id));
    expect_code(call, answer, NOT_FOUND)
}

/// Case 7: a keep of a session that has no conversation is an argv that starts the process on
/// it: built, and naming the session (case 2 shows that such an argv, a fresh session's, starts
/// a process that is located holding it).
fn keep_without_conversation_starts_it(
    h: &dyn HarnessPort,
    _: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let argv = command(h, rig, &rig.panes[0], &SessionChoice::Keep(id.clone()))?;
    if argv.as_slice().contains(&id) {
        return Ok(());
    }
    Err(format!(
        "the argv of a keep of {id} names no session: none of its {} elements is the id",
        argv.as_slice().len()
    ))
}

/// Case 8: a prompt to a session the process does not hold is `session-mismatch`.
fn prompt_to_another_session_is_session_mismatch(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let at = launched(h, hands, rig, &SessionChoice::Fresh(id))?;
    let other = new_id(h)?;
    let call = format!("send_prompt to a session {at} does not hold");
    expect_code(
        &call,
        h.send_prompt(&at, &other, PROMPT),
        SESSION_MISMATCH.as_str(),
    )
}

/// Case 9 (W12): a prompt delivered at `send_prompt` starts the session's conversation, so the
/// process, stopped and resumed, holds the same session again, in a new process.
fn resume_after_a_prompt_keeps_the_session(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let pane = &rig.panes[0];
    let id = new_id(h)?;
    let first = launched(h, hands, rig, &SessionChoice::Fresh(id.clone()))?;
    succeeds(
        &format!("send_prompt to {first}"),
        h.send_prompt(&first, &id, PROMPT),
    )?;
    let resume = SessionChoice::Resume(id.clone());
    let argv = wait_until(BOOT_POLL, Instant::now() + SETTLE_WITHIN, || {
        Ok(h.process_command(pane, &spec(rig), &resume).ok())
    });
    let Some(argv) = succeeds("the wait for the conversation", argv)? else {
        return Err(format!(
            "the session {id} had no conversation to resume within {SETTLE_WITHIN:?} of a \
             prompt that send_prompt acknowledged: the prompt did not reach the model"
        ));
    };
    stopped(h, hands, pane, &first)?;
    succeeds(
        &format!("the launch of {pane}"),
        (hands.launch)(pane, &argv),
    )?;
    let again = located(h, pane, &id)?;
    if again == first {
        return Err(format!(
            "the resumed process is {again}, the one that was stopped"
        ));
    }
    Ok(())
}

/// Case 10: a stopped process is not healthy.
fn stopped_process_is_not_healthy(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let at = launched(h, hands, rig, &SessionChoice::Fresh(id))?;
    stopped(h, hands, &rig.panes[0], &at)
}

/// Case 11: `abort` is `abort-unsupported`: there is no in-place abort for a single process,
/// whatever it is doing, and a signal is not one. Nothing is launched: the refusal is the
/// harness's, not a process's.
fn abort_is_unsupported(h: &dyn HarnessPort, _: &Hands<'_>, rig: &ClaudeRig) -> Result<(), String> {
    let id = new_id(h)?;
    let at = HarnessEndpoint::Process {
        pane: rig.panes[0].clone(),
        pid: ABORTED_PID,
    };
    expect_code(
        &format!("abort of {at}"),
        h.abort(&at, &id),
        ABORT_UNSUPPORTED.as_str(),
    )
}

/// Case 12: `login_status` answers.
fn login_status_answers(h: &dyn HarnessPort, _: &Hands<'_>, _: &ClaudeRig) -> Result<(), String> {
    succeeds("login_status", h.login_status()).map(drop)
}

/// Case 13 (#647 part 2): `agent_view` of a process endpoint is `Ok` with the empty view.
/// Nothing is launched: a single process cannot tell, whatever it is doing.
fn agent_view_of_a_process_is_empty(
    h: &dyn HarnessPort,
    _: &Hands<'_>,
    rig: &ClaudeRig,
) -> Result<(), String> {
    let id = new_id(h)?;
    let at = HarnessEndpoint::Process {
        pane: rig.panes[0].clone(),
        pid: ABORTED_PID,
    };
    let view = succeeds(&format!("agent_view of {at}"), h.agent_view(&at, &id, 3))?;
    expect_eq(&format!("agent_view of {at}"), view, AgentView::default())
}

// --- helpers ---

/// The spec every process of the rig runs with: its directory.
fn spec(rig: &ClaudeRig) -> ServeSpec {
    ServeSpec::default().with_cwd(rig.cwd.clone())
}

/// A fresh session id, which must be given.
fn new_id(h: &dyn HarnessPort) -> Result<String, String> {
    succeeds("new_session_id", h.new_session_id())
}

/// The argv `process_command` builds for `pane` on `choice`, which must succeed.
fn command(
    h: &dyn HarnessPort,
    rig: &ClaudeRig,
    pane: &PaneName,
    choice: &SessionChoice,
) -> Result<Argv, String> {
    let call = format!("process_command({pane}, {choice:?})");
    succeeds(&call, h.process_command(pane, &spec(rig), choice))
}

/// Launch the rig's first pane on `choice`, which must succeed, and wait until its process is
/// located holding the session: its endpoint.
fn launched(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    rig: &ClaudeRig,
    choice: &SessionChoice,
) -> Result<HarnessEndpoint, String> {
    let pane = &rig.panes[0];
    let argv = command(h, rig, pane, choice)?;
    succeeds(
        &format!("the launch of {pane}"),
        (hands.launch)(pane, &argv),
    )?;
    located(h, pane, choice.session())
}

/// The endpoint of the process of `pane` holding `session`, once located within
/// [`LOCATE_WITHIN`].
fn located(h: &dyn HarnessPort, pane: &PaneName, session: &str) -> Result<HarnessEndpoint, String> {
    let deadline = Instant::now() + LOCATE_WITHIN;
    let wait = format!("the wait for the process of {pane} holding {session}");
    match succeeds(&wait, wait_located(h, pane, session, BOOT_POLL, deadline))? {
        Some(pid) => Ok(HarnessEndpoint::Process {
            pane: pane.clone(),
            pid,
        }),
        None => Err(format!(
            "no process of {pane} holding {session} was located within {LOCATE_WITHIN:?} of \
             its launch"
        )),
    }
}

/// Stop the processes of `pane`, which must succeed, and wait until `at` is no longer healthy,
/// within [`SETTLE_WITHIN`].
fn stopped(
    h: &dyn HarnessPort,
    hands: &Hands<'_>,
    pane: &PaneName,
    at: &HarnessEndpoint,
) -> Result<(), String> {
    succeeds(&format!("the stop of {pane}"), (hands.stop)(pane))?;
    let gone = wait_until(BOOT_POLL, Instant::now() + SETTLE_WITHIN, || {
        Ok((!h.health(at)?).then_some(()))
    });
    match succeeds(&format!("health of {at} after the stop"), gone)? {
        Some(()) => Ok(()),
        None => Err(format!(
            "{at} was still healthy {SETTLE_WITHIN:?} after its pane was stopped"
        )),
    }
}
