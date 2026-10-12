//! The `HarnessPort` conformance suite: [`run_harness_conformance`] runs the cases
//! [`harness_cases`] lists against any `HarnessPort`, each against a fresh harness and
//! the [`HarnessRig`] that comes with it.
//!
//! The cases pin the contract that the OpenCode adapter (#642) provides on top of raw
//! OpenCode as the spike observed it (`docs/research/opencode-pane-spike.md`): a fresh
//! server has no session; servers that share a data directory share their sessions; a
//! TUI shows the session it was attached to, never a guess; and a switch reaches only
//! the pane it names. Raw OpenCode's quirks are what the adapter must hide, so a harness
//! with one on fails the suite. Decided here (#684), and binding on the adapter:
//!
//! - `abort`, `attach_tui` and `select_session` of an id the server does not know are
//!   `session-not-found` (cases 8, 11 and 13). Raw OpenCode acknowledges an abort of an
//!   unknown id, so the adapter checks `GET /session/:id` first.
//! - A call to a port whose server does not answer is `unavailable`, checked before the
//!   session id (case 6).
//! - `select_session` on a pane with no TUI is `unavailable`, checked before the session
//!   id (case 14): the method takes no port, so the TUI is what names the server, and
//!   raw OpenCode acknowledges a switch that no TUI saw. Its class is a failure (exit 1),
//!   as a timeout's is.
//! - `shown_session` of a pane with no TUI is `Ok(None)` (case 9).
//!
//! **How a case starts a server (#802).** `serve_command` is pure: it builds the server's
//! argv, which the host's `run` starts in the pane's tmux session. So the caller passes a
//! launcher, generic over the subject, that starts a server from such an argv, and the
//! suite then waits for the server's health with the launch engine's own wait
//! (`holler_pane::tx_launch::wait_healthy`, every `BOOT_POLL`, within `DEFAULT_BUDGET`).
//! Case 2 checks that `serve_command` starts nothing, and case 16 that it never returns an
//! argv that would start a server without the agent the spec asks for: the argv names the
//! agent, or the harness refuses it with `agent-unsupported`.
//!
//! Every list of sessions is read as a set: the port fixes no order, and OpenCode lists
//! the most recently updated first. A frozen or killed server, a deleted session, a TUI
//! moved by hand and a closed TUI cannot be caused through the port, so the suite does
//! not drive them; the fake's own tests pin them.

use std::time::Instant;

use holler_pane::ports::AGENT_UNSUPPORTED;
use holler_pane::tx_launch::{wait_healthy, BOOT_POLL, DEFAULT_BUDGET};
use holler_pane::{AgentKey, Argv, HarnessPort, PaneError, PaneId, PaneName, ServeSpec};

use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn HarnessPort, &Launcher<'_>, &HarnessRig) -> Result<(), String>;

/// The caller's launcher, bound to the harness of the case: it starts the server an argv of
/// `serve_command` describes, for the pane it names.
struct Launcher<'a>(&'a dyn Fn(&PaneName, &Argv) -> Result<(), PaneError>);

const NOT_FOUND: &str = "session-not-found";
const UNAVAILABLE: &str = "unavailable";

/// The agent key case 16 asks for: a neutral name that no binary path or flag holds.
const AGENT: &str = "conformance-agent";

/// A session id that is well formed (`ses_` and 26 characters) and that no server holds:
/// the fake's suffix is hex.
const UNKNOWN: &str = "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz";

/// The panes whose servers the cases start, on the rig's first and second port: neutral
/// names, never a live session's.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";

/// The Herdr session of the sample rig's panes: a scratch name, as in the fixture.
const SCRATCH: &str = "scratch";

/// What a harness under test gives the suite for one case: two free ports whose servers
/// share ONE data directory, as the live fleet's do, and two panes in which a TUI can be
/// attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRig {
    /// Two ports with no server on them yet.
    pub ports: [u16; 2],
    /// Two panes with no TUI in them yet.
    pub panes: [PaneId; 2],
}

impl HarnessRig {
    /// Ports 48100 and 48101 (the spike's scratch range) and panes `scratch:demo-c1r1`
    /// and `scratch:demo-c2r1` (the fixture's `pane_id` form). Enough for the fake; a
    /// real adapter supplies its own.
    pub fn sample() -> Self {
        Self {
            ports: [48100, 48101],
            panes: [
                PaneId::new(format!("{SCRATCH}:{C1}")),
                PaneId::new(format!("{SCRATCH}:{C2}")),
            ],
        }
    }
}

/// The suite, in order: the one table that the runner iterates and [`harness_cases`]
/// lists.
const CASES: [(&str, Case); 16] = [
    (
        "health-of-unserved-port-is-false",
        health_of_unserved_port_is_false,
    ),
    ("launch-then-healthy", launch_then_healthy),
    ("fresh-server-has-no-sessions", fresh_server_has_no_sessions),
    ("create-session-is-listed", create_session_is_listed),
    (
        "sessions-shared-across-servers",
        sessions_shared_across_servers,
    ),
    (
        "calls-to-unserved-port-are-unavailable",
        calls_to_unserved_port_are_unavailable,
    ),
    ("abort-known-session", abort_known_session),
    (
        "abort-unknown-is-session-not-found",
        abort_unknown_is_session_not_found,
    ),
    ("shown-without-tui-is-none", shown_without_tui_is_none),
    ("attach-shows-the-session", attach_shows_the_session),
    (
        "attach-unknown-is-session-not-found",
        attach_unknown_is_session_not_found,
    ),
    (
        "select-switches-the-shown-session",
        select_switches_the_shown_session,
    ),
    (
        "select-unknown-is-session-not-found",
        select_unknown_is_session_not_found,
    ),
    ("select-without-tui-fails", select_without_tui_fails),
    ("select-reaches-only-its-pane", select_reaches_only_its_pane),
    (
        "serve-command-carries-the-agent",
        serve_command_carries_the_agent,
    ),
];

/// The ids of the cases [`run_harness_conformance`] runs, in the order it runs them.
pub fn harness_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`harness_cases`], in order, each against a fresh harness, and
/// return every case that did not hold.
///
/// `fresh` is called once per case. It returns the harness, the rig the case runs on and
/// a guard that the suite keeps alive for that case only; the harness is dropped before
/// its guard. No server may run on the rig's ports yet, and their servers must share one
/// data directory; no TUI may be in its panes yet.
///
/// `launch` starts a server from an argv the harness's `serve_command` built, for the pane
/// the argv was built for, as the host's `run` would (#802); the suite then waits for the
/// server's health. How each implementation runs the suite (the test kit cannot name an
/// adapter):
///
/// ```text
/// // the fake: its scenario method starts the server the argv describes.
/// let launch = |h: &FakeHarness, _: &PaneName, argv: &Argv| match h.start_server(argv) {
///     Some(started) => started.map(drop),
///     None => Err(PaneError::Usage { message: "not the fake's argv".to_owned() }),
/// };
/// assert_eq!(
///     run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ()), launch),
///     Ok(())
/// );
/// // holler-adapter-opencode (#642): per case, a scratch dir with the spike's isolated
/// //   env and dead-end provider (opencode-pane-spike.md:16-35), two free ports from
/// //   48100-48199 whose servers share that one data directory, and two panes of a
/// //   private tmux server; the scratch dir and the process groups as the guard. Its
/// //   launcher runs the argv itself, in a process group of its own, and keeps the pid
/// //   for the guard: that crate may not depend on the host adapter (ADR-0021 section 5).
/// ```
pub fn run_harness_conformance<S, K, F, L>(mut fresh: F, launch: L) -> Conformance
where
    S: HarnessPort,
    F: FnMut() -> (S, HarnessRig, K),
    L: Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>,
{
    run_cases(
        &CASES,
        || {
            let (harness, rig, guard) = fresh();
            ((harness, rig), guard)
        },
        |case, (harness, rig)| {
            let start = |name: &PaneName, argv: &Argv| launch(harness, name, argv);
            case(harness, &Launcher(&start), rig)
        },
    )
}

// --- the cases ---

/// Case 1: `health` of a port where no server was started is `Ok(false)`.
fn health_of_unserved_port_is_false(
    h: &dyn HarnessPort,
    _: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let healthy = succeeds("health of a port never served", h.health(p0))?;
    expect_eq("health of a port never served", healthy, false)
}

/// Case 2: `serve_command` starts nothing (its port is not healthy after it), and the
/// server its argv starts, once launched, is healthy.
fn launch_then_healthy(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let (name, argv) = command(h, C1, p0)?;
    let early = succeeds("health after serve_command", h.health(p0))?;
    expect_eq(
        "health after serve_command, before the launcher (it starts nothing)",
        early,
        false,
    )?;
    launched(h, l, &name, &argv, p0)?;
    let healthy = succeeds("health after the launch", h.health(p0))?;
    expect_eq("health after the launch", healthy, true)
}

/// Case 3: a fresh server has no session of its own, no "ping" session.
fn fresh_server_has_no_sessions(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    launch(h, l, C1, p0)?;
    expect_eq("the sessions of a fresh server", list(h, p0)?, Vec::new())
}

/// Case 4: two `create_session` calls return two distinct, non-empty ids, and the
/// server lists both.
fn create_session_is_listed(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    launch(h, l, C1, p0)?;
    let a = create(h, p0)?;
    let b = create(h, p0)?;
    if a.is_empty() || b.is_empty() || a == b {
        return Err(format!(
            "two create_session calls must return distinct, non-empty ids; got {a:?} and {b:?}"
        ));
    }
    let listed = list(h, p0)?;
    holds("list_sessions after two creates", &listed, &a)?;
    holds("list_sessions after two creates", &listed, &b)
}

/// Case 5: servers that share a data directory share their sessions: a session created
/// through the second port is listed through the first.
fn sessions_shared_across_servers(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, p1] = rig.ports;
    launch(h, l, C1, p0)?;
    launch(h, l, C2, p1)?;
    let on_p1 = create(h, p1)?;
    holds(
        &format!("list_sessions({p0}) after a create on {p1}"),
        &list(h, p0)?,
        &on_p1,
    )
}

/// Case 6: a call to a port where no server was started is `unavailable`, even with an id
/// that no server holds: reachability is checked before existence.
fn calls_to_unserved_port_are_unavailable(
    h: &dyn HarnessPort,
    _: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    let unserved = "on a port never served";
    expect_code(
        &format!("create_session {unserved}"),
        h.create_session(p0),
        UNAVAILABLE,
    )?;
    expect_code(
        &format!("list_sessions {unserved}"),
        h.list_sessions(p0),
        UNAVAILABLE,
    )?;
    expect_code(
        &format!("abort {unserved}"),
        h.abort(p0, UNKNOWN),
        UNAVAILABLE,
    )?;
    expect_code(
        &format!("attach_tui {unserved}"),
        h.attach_tui(pane0, p0, UNKNOWN),
        UNAVAILABLE,
    )
}

/// Case 7: `abort` of a session the server holds is `Ok`.
fn abort_known_session(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    launch(h, l, C1, p0)?;
    let a = create(h, p0)?;
    succeeds(&format!("abort({p0}, {a})"), h.abort(p0, &a))
}

/// Case 8: `abort` of an id the server does not know is `session-not-found`, where raw
/// OpenCode answers `200 true` (opencode-pane-spike.md:171-172).
fn abort_unknown_is_session_not_found(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    launch(h, l, C1, p0)?;
    expect_code(
        "abort of an id the server does not know",
        h.abort(p0, UNKNOWN),
        NOT_FOUND,
    )
}

/// Case 9: `shown_session` of a pane with no TUI is `Ok(None)`.
fn shown_without_tui_is_none(
    h: &dyn HarnessPort,
    _: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [pane0, _] = &rig.panes;
    expect_eq(
        "shown_session of a pane with no TUI",
        shown(h, pane0)?,
        None,
    )
}

/// Case 10: a TUI attached to a session shows it.
fn attach_shows_the_session(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    launch(h, l, C1, p0)?;
    let a = create(h, p0)?;
    attach(h, pane0, p0, &a)?;
    expect_eq("shown_session after attach_tui", shown(h, pane0)?, Some(a))
}

/// Case 11: `attach_tui` to an id the server does not know is `session-not-found`, and
/// the pane shows nothing: it did not fall back to the session that exists (the guess
/// behind the 2026-10-07 incidents; opencode-pane-spike.md:100-101).
fn attach_unknown_is_session_not_found(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    launch(h, l, C1, p0)?;
    create(h, p0)?;
    let call = "attach_tui to an id the server does not know";
    expect_code(call, h.attach_tui(pane0, p0, UNKNOWN), NOT_FOUND)?;
    expect_eq(
        &format!("shown_session after the refused {call}"),
        shown(h, pane0)?,
        None,
    )
}

/// Case 12: `select_session` switches the pane's TUI to another session.
fn select_switches_the_shown_session(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    switched(h, l, rig).map(|_| ())
}

/// Case 13: `select_session` of an id the server does not know is `session-not-found`,
/// and the TUI keeps the session it showed (opencode-pane-spike.md:117).
fn select_unknown_is_session_not_found(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [pane0, _] = &rig.panes;
    let b = switched(h, l, rig)?;
    let call = "select_session of an id the server does not know";
    expect_code(call, h.select_session(pane0, UNKNOWN), NOT_FOUND)?;
    expect_eq(
        &format!("shown_session after the refused {call}"),
        shown(h, pane0)?,
        Some(b),
    )
}

/// Case 14: `select_session` on a pane with no TUI is `unavailable`, where raw OpenCode
/// answers `200 true` (opencode-pane-spike.md:122-124), and the pane still shows nothing.
fn select_without_tui_fails(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    launch(h, l, C1, p0)?;
    let a = create(h, p0)?;
    let call = "select_session on a pane with no TUI";
    expect_code(call, h.select_session(pane0, &a), UNAVAILABLE)?;
    expect_eq(
        &format!("shown_session after the refused {call}"),
        shown(h, pane0)?,
        None,
    )
}

/// Case 15: `select_session` switches only the TUI of the pane it names, not another
/// pane's TUI on the same session (raw OpenCode broadcasts to every TUI of a server,
/// opencode-pane-spike.md:119-121).
fn select_reaches_only_its_pane(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, p1] = rig.ports;
    let [pane0, pane1] = &rig.panes;
    launch(h, l, C1, p0)?;
    launch(h, l, C2, p1)?;
    let a = create(h, p0)?;
    let b = create(h, p0)?;
    attach(h, pane0, p0, &a)?;
    attach(h, pane1, p1, &a)?;
    select(h, pane0, &b)?;
    expect_eq(
        "shown_session of the pane switched",
        shown(h, pane0)?,
        Some(b),
    )?;
    expect_eq("shown_session of the other pane", shown(h, pane1)?, Some(a))
}

/// Case 16 (#802): `serve_command` of a spec with an agent never returns an argv that would
/// start a server without it: the answer is `agent-unsupported`, or an argv one of whose
/// elements contains the agent's key. Nothing is launched.
fn serve_command_carries_the_agent(
    h: &dyn HarnessPort,
    _: &Launcher<'_>,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let name = succeeds("PaneName::parse", PaneName::parse(C1))?;
    let key = succeeds("AgentKey::parse", AgentKey::parse(AGENT))?;
    let spec = ServeSpec::default().with_agent(key);
    let call = format!("serve_command({name}, {p0}) with the agent {AGENT}");
    match h.serve_command(&name, p0, &spec) {
        Ok(argv)
            if argv
                .as_slice()
                .iter()
                .any(|element| element.contains(AGENT)) =>
        {
            Ok(())
        }
        Ok(argv) => Err(format!(
            "{call}: none of the {} elements of its argv names the agent, so the server would \
             start without it",
            argv.as_slice().len()
        )),
        Err(e) if e.code() == AGENT_UNSUPPORTED.as_str() => Ok(()),
        Err(e) => Err(format!(
            "{call}: expected an argv that names the agent, or `{AGENT_UNSUPPORTED}`; got `{}`: \
             {e}",
            e.code()
        )),
    }
}

// --- helpers ---

/// Start the server of the pane named `text` on `port`, which must succeed: the argv
/// `serve_command` builds for it with no agent, run by the launcher, then the health wait.
fn launch(h: &dyn HarnessPort, l: &Launcher<'_>, text: &str, port: u16) -> Result<(), String> {
    let (name, argv) = command(h, text, port)?;
    launched(h, l, &name, &argv, port)
}

/// The pane named `text`, and the argv `serve_command` builds for its server on `port` with
/// no agent, which must succeed.
fn command(h: &dyn HarnessPort, text: &str, port: u16) -> Result<(PaneName, Argv), String> {
    let name = succeeds("PaneName::parse", PaneName::parse(text))?;
    let call = format!("serve_command({name}, {port})");
    let argv = succeeds(&call, h.serve_command(&name, port, &ServeSpec::default()))?;
    Ok((name, argv))
}

/// Run `argv` through the launcher, which must succeed, then wait for the server on `port` to
/// answer as the launch engine waits: every `BOOT_POLL`, within a whole run's budget.
fn launched(
    h: &dyn HarnessPort,
    l: &Launcher<'_>,
    name: &PaneName,
    argv: &Argv,
    port: u16,
) -> Result<(), String> {
    succeeds(
        &format!("the launch of {name} on {port}"),
        (l.0)(name, argv),
    )?;
    let deadline = Instant::now() + DEFAULT_BUDGET;
    let wait = format!("the health wait on {port}");
    if succeeds(&wait, wait_healthy(h, port, BOOT_POLL, deadline))? {
        Ok(())
    } else {
        Err(format!(
            "the server launched for {name} on {port} did not answer within {DEFAULT_BUDGET:?}"
        ))
    }
}

fn create(h: &dyn HarnessPort, port: u16) -> Result<String, String> {
    succeeds(&format!("create_session({port})"), h.create_session(port))
}

fn list(h: &dyn HarnessPort, port: u16) -> Result<Vec<String>, String> {
    succeeds(&format!("list_sessions({port})"), h.list_sessions(port))
}

fn attach(h: &dyn HarnessPort, pane: &PaneId, port: u16, session: &str) -> Result<(), String> {
    let call = format!("attach_tui({}, {port}, {session})", pane.as_str());
    succeeds(&call, h.attach_tui(pane, port, session))
}

fn select(h: &dyn HarnessPort, pane: &PaneId, session: &str) -> Result<(), String> {
    let call = format!("select_session({}, {session})", pane.as_str());
    succeeds(&call, h.select_session(pane, session))
}

fn shown(h: &dyn HarnessPort, pane: &PaneId) -> Result<Option<String>, String> {
    succeeds(
        &format!("shown_session({})", pane.as_str()),
        h.shown_session(pane),
    )
}

/// `Ok` when `ids`, what `what` read, holds `id`.
fn holds(what: &str, ids: &[String], id: &str) -> Result<(), String> {
    if ids.iter().any(|held| held == id) {
        Ok(())
    } else {
        Err(format!("{what} lacks {id:?}: {ids:?}"))
    }
}

/// Launch the first port's server, create two sessions, attach the first pane to the first
/// session and select the second, which the pane must then show; returns that second
/// session.
fn switched(h: &dyn HarnessPort, l: &Launcher<'_>, rig: &HarnessRig) -> Result<String, String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    launch(h, l, C1, p0)?;
    let a = create(h, p0)?;
    let b = create(h, p0)?;
    attach(h, pane0, p0, &a)?;
    select(h, pane0, &b)?;
    expect_eq(
        "shown_session after select_session",
        shown(h, pane0)?,
        Some(b.clone()),
    )?;
    Ok(b)
}
