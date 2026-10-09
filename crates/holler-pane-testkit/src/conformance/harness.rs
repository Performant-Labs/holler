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
//! Every list of sessions is read as a set: the port fixes no order, and OpenCode lists
//! the most recently updated first. A frozen or killed server, a deleted session, a TUI
//! moved by hand and a closed TUI cannot be caused through the port, so the suite does
//! not drive them; the fake's own tests pin them.

use holler_pane::{HarnessPort, PaneId, PaneName};

use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn HarnessPort, &HarnessRig) -> Result<(), String>;

const NOT_FOUND: &str = "session-not-found";
const UNAVAILABLE: &str = "unavailable";

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
const CASES: [(&str, Case); 15] = [
    (
        "health-of-unserved-port-is-false",
        health_of_unserved_port_is_false,
    ),
    ("serve-then-healthy", serve_then_healthy),
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
/// data directory; no TUI may be in its panes yet. How each implementation runs the
/// suite (the test kit cannot name an adapter):
///
/// ```text
/// // the fake:
/// assert_eq!(
///     run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ())),
///     Ok(())
/// );
/// // holler-adapter-opencode (#642): per case, a scratch dir with the spike's isolated
/// //   env and dead-end provider (opencode-pane-spike.md:16-35), two free ports from
/// //   48100-48199 whose servers share that one data directory, and two panes of a
/// //   private tmux server; the scratch dir and the process groups as the guard.
/// ```
pub fn run_harness_conformance<S, K, F>(mut fresh: F) -> Conformance
where
    S: HarnessPort,
    F: FnMut() -> (S, HarnessRig, K),
{
    run_cases(
        &CASES,
        || {
            let (harness, rig, guard) = fresh();
            ((harness, rig), guard)
        },
        |case, (harness, rig)| case(harness, rig),
    )
}

// --- the cases ---

/// Case 1: `health` of a port where no server was served is `Ok(false)`.
fn health_of_unserved_port_is_false(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let healthy = succeeds("health of a port never served", h.health(p0))?;
    expect_eq("health of a port never served", healthy, false)
}

/// Case 2: `serve` returns the server's pid, and the server is then healthy.
fn serve_then_healthy(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let pid = serve(h, C1, p0)?;
    if pid == 0 {
        return Err(format!(
            "serve({C1}, {p0}) returned pid 0, which is no process"
        ));
    }
    let healthy = succeeds("health after serve", h.health(p0))?;
    expect_eq("health after serve", healthy, true)
}

/// Case 3: a fresh server has no session of its own, no "ping" session.
fn fresh_server_has_no_sessions(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    serve(h, C1, p0)?;
    expect_eq("the sessions of a fresh server", list(h, p0)?, Vec::new())
}

/// Case 4: two `create_session` calls return two distinct, non-empty ids, and the
/// server lists both.
fn create_session_is_listed(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    serve(h, C1, p0)?;
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
fn sessions_shared_across_servers(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, p1] = rig.ports;
    serve(h, C1, p0)?;
    serve(h, C2, p1)?;
    let on_p1 = create(h, p1)?;
    holds(
        &format!("list_sessions({p0}) after a create on {p1}"),
        &list(h, p0)?,
        &on_p1,
    )
}

/// Case 6: a call to a port where no server was served is `unavailable`, even with an id
/// that no server holds: reachability is checked before existence.
fn calls_to_unserved_port_are_unavailable(
    h: &dyn HarnessPort,
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
fn abort_known_session(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    serve(h, C1, p0)?;
    let a = create(h, p0)?;
    succeeds(&format!("abort({p0}, {a})"), h.abort(p0, &a))
}

/// Case 8: `abort` of an id the server does not know is `session-not-found`, where raw
/// OpenCode answers `200 true` (opencode-pane-spike.md:171-172).
fn abort_unknown_is_session_not_found(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    serve(h, C1, p0)?;
    expect_code(
        "abort of an id the server does not know",
        h.abort(p0, UNKNOWN),
        NOT_FOUND,
    )
}

/// Case 9: `shown_session` of a pane with no TUI is `Ok(None)`.
fn shown_without_tui_is_none(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [pane0, _] = &rig.panes;
    expect_eq(
        "shown_session of a pane with no TUI",
        shown(h, pane0)?,
        None,
    )
}

/// Case 10: a TUI attached to a session shows it.
fn attach_shows_the_session(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    serve(h, C1, p0)?;
    let a = create(h, p0)?;
    attach(h, pane0, p0, &a)?;
    expect_eq("shown_session after attach_tui", shown(h, pane0)?, Some(a))
}

/// Case 11: `attach_tui` to an id the server does not know is `session-not-found`, and
/// the pane shows nothing: it did not fall back to the session that exists (the guess
/// behind the 2026-10-07 incidents; opencode-pane-spike.md:100-101).
fn attach_unknown_is_session_not_found(
    h: &dyn HarnessPort,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    serve(h, C1, p0)?;
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
fn select_switches_the_shown_session(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    switched(h, rig).map(|_| ())
}

/// Case 13: `select_session` of an id the server does not know is `session-not-found`,
/// and the TUI keeps the session it showed (opencode-pane-spike.md:117).
fn select_unknown_is_session_not_found(
    h: &dyn HarnessPort,
    rig: &HarnessRig,
) -> Result<(), String> {
    let [pane0, _] = &rig.panes;
    let b = switched(h, rig)?;
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
fn select_without_tui_fails(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    serve(h, C1, p0)?;
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
fn select_reaches_only_its_pane(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<(), String> {
    let [p0, p1] = rig.ports;
    let [pane0, pane1] = &rig.panes;
    serve(h, C1, p0)?;
    serve(h, C2, p1)?;
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

// --- helpers ---

/// Serve the pane named `text` on `port`, which must succeed; returns the pid.
fn serve(h: &dyn HarnessPort, text: &str, port: u16) -> Result<u32, String> {
    let name = succeeds("PaneName::parse", PaneName::parse(text))?;
    succeeds(&format!("serve({name}, {port})"), h.serve(&name, port))
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

/// Serve the first port, create two sessions, attach the first pane to the first
/// session and select the second, which the pane must then show; returns that second
/// session.
fn switched(h: &dyn HarnessPort, rig: &HarnessRig) -> Result<String, String> {
    let [p0, _] = rig.ports;
    let [pane0, _] = &rig.panes;
    serve(h, C1, p0)?;
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
