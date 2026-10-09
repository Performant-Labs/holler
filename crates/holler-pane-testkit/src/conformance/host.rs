//! The `HostPort` conformance suite: [`run_host_conformance`] runs the cases
//! [`host_cases`] lists against any `HostPort`, each against a fresh host.
//!
//! The cases pin what the tmux adapter (#641) must keep: `run` starts a process that
//! `ps` then lists, an empty argv is refused, `ensure_session` is idempotent, and
//! `stop_owned` stops every process of its session and none of another session's (no
//! broad kill). Two readings of the port are decided here (#684), and they bind the
//! adapter:
//!
//! - A missing session is `pane-not-found` for `run` and `ps`, which is how this suite
//!   reads #641's "a missing session is a typed error". The tmux session is named by the
//!   pane's name, so the pane's code fits; `session-not-found` stays the code of a
//!   harness session. An empty argv is `usage`.
//! - `stop_owned` of a missing session is `Ok`: nothing is owned, so nothing is
//!   stopped, and a `relaunch` or `close` after a crash does not fail on it (case 7).
//!
//! The cases assert only what a real tmux session can also satisfy. A real session may
//! hold a shell process, so no case asserts that a fresh session's `ps` is empty; and it
//! may end with its last process, so a case ensures the session again before it reads
//! `ps` after `stop_owned`. Every list of pids is read as a set: the port fixes no order.
//! The sessions have neutral names (`demo-c1r1`, `demo-c2r1`) and work in the system's
//! temporary directory, which exists on any machine. The command is `sleep 30`: harmless
//! if a real adapter runs it, and the case's guard stops it.

use std::env;

use holler_pane::{Argv, HostPort, PaneName};

use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn HostPort) -> Result<(), String>;

const NOT_FOUND: &str = "pane-not-found";
const USAGE: &str = "usage";

/// The session names of the cases: neutral ones, never a live session's.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";

/// The suite, in order: the one table that the runner iterates and [`host_cases`]
/// lists.
const CASES: [(&str, Case); 9] = [
    (
        "ps-of-missing-session-is-pane-not-found",
        ps_of_missing_session_is_pane_not_found,
    ),
    (
        "run-in-missing-session-is-pane-not-found",
        run_in_missing_session_is_pane_not_found,
    ),
    ("run-adds-a-process", run_adds_a_process),
    ("ensure-session-is-idempotent", ensure_session_is_idempotent),
    ("run-empty-argv-is-usage", run_empty_argv_is_usage),
    (
        "stop-owned-stops-every-owned-process",
        stop_owned_stops_every_owned_process,
    ),
    (
        "stop-owned-of-missing-session-is-ok",
        stop_owned_of_missing_session_is_ok,
    ),
    (
        "stop-owned-leaves-other-sessions",
        stop_owned_leaves_other_sessions,
    ),
    ("ps-lists-only-its-session", ps_lists_only_its_session),
];

/// The ids of the cases [`run_host_conformance`] runs, in the order it runs them.
pub fn host_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`host_cases`], in order, each against a fresh host with no
/// session, and return every case that did not hold.
///
/// `fresh` is called once per case. It returns the host and a guard that the suite
/// keeps alive for that case only; the host is dropped before its guard. How each
/// implementation runs the suite (the test kit cannot name an adapter):
///
/// ```text
/// // the fake:
/// assert_eq!(run_host_conformance(|| (FakeHost::new(), ())), Ok(()));
/// // holler-adapter-host (#641): a private tmux server per case
/// //   (`tmux -S <tempdir>/tmux.sock`), the adapter on it as the host, and the tempdir
/// //   and the tmux server's handle as the guard, so dropping the guard kills every
/// //   process the case started.
/// ```
pub fn run_host_conformance<S, K, F>(fresh: F) -> Conformance
where
    S: HostPort,
    F: FnMut() -> (S, K),
{
    run_cases(&CASES, fresh, |case, host| case(host))
}

// --- the cases ---

/// Case 1: `ps` of a session never ensured is `pane-not-found`.
fn ps_of_missing_session_is_pane_not_found(host: &dyn HostPort) -> Result<(), String> {
    let missing = succeeds("PaneName::parse", PaneName::parse(C1))?;
    expect_code(
        "ps of a session never ensured",
        host.ps(&missing),
        NOT_FOUND,
    )
}

/// Case 2: `run` in a session never ensured is `pane-not-found`, and it creates
/// nothing: `ps` of the session is still `pane-not-found`.
fn run_in_missing_session_is_pane_not_found(host: &dyn HostPort) -> Result<(), String> {
    let missing = succeeds("PaneName::parse", PaneName::parse(C1))?;
    let call = "run in a session never ensured";
    expect_code(call, host.run(&missing, &sleep()), NOT_FOUND)?;
    expect_code(
        &format!("ps after the refused {call}"),
        host.ps(&missing),
        NOT_FOUND,
    )
}

/// Case 3: `run` starts a process, which `ps` lists.
fn run_adds_a_process(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    started(host, &a).map(|_| ())
}

/// Case 4: a second `ensure_session` of a session that exists is `Ok` and keeps every
/// process of the session.
fn ensure_session_is_idempotent(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    started(host, &a)?;
    let before = ps(host, &a)?;
    succeeds(
        "a second ensure_session",
        host.ensure_session(&a, &temp_dir()),
    )?;
    let after = ps(host, &a)?;
    before
        .into_iter()
        .try_for_each(|pid| holds("ps after a second ensure_session", &after, pid))
}

/// Case 5: `run` of an empty argv is `usage` and starts nothing.
fn run_empty_argv_is_usage(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    let before = ps(host, &a)?;
    let call = "run of an empty argv";
    expect_code(call, host.run(&a, &Argv::new(Vec::new())), USAGE)?;
    expect_eq(
        &format!("ps after the refused {call}"),
        sorted(ps(host, &a)?),
        sorted(before),
    )
}

/// Case 6: `stop_owned` stops every process the session's runs started. The session
/// is ensured again before `ps`, because a real one may end with its last process.
fn stop_owned_stops_every_owned_process(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    let first = started(host, &a)?;
    let second = started(host, &a)?;
    succeeds("stop_owned", host.stop_owned(&a))?;
    succeeds(
        "ensure_session after stop_owned",
        host.ensure_session(&a, &temp_dir()),
    )?;
    let left = ps(host, &a)?;
    lacks("ps after stop_owned", &left, first)?;
    lacks("ps after stop_owned", &left, second)
}

/// Case 7: `stop_owned` of a session never ensured is `Ok`: nothing is owned, so
/// nothing is stopped. This narrows #641's "a missing session is a typed error" to `run`
/// and `ps`, so that a `relaunch` or `close` after a crash does not fail.
fn stop_owned_of_missing_session_is_ok(host: &dyn HostPort) -> Result<(), String> {
    let missing = succeeds("PaneName::parse", PaneName::parse(C1))?;
    succeeds(
        "stop_owned of a session never ensured",
        host.stop_owned(&missing),
    )
}

/// Case 8: `stop_owned` of one session leaves another session's processes running
/// (#641 forbids the broad kill of `pkill` or `killall`).
fn stop_owned_leaves_other_sessions(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    let b = ensured(host, C2)?;
    started(host, &a)?;
    let in_b = started(host, &b)?;
    succeeds("stop_owned(a)", host.stop_owned(&a))?;
    holds("ps(b) after stop_owned(a)", &ps(host, &b)?, in_b)
}

/// Case 9: `ps` of a session does not list another session's processes.
fn ps_lists_only_its_session(host: &dyn HostPort) -> Result<(), String> {
    let a = ensured(host, C1)?;
    let b = ensured(host, C2)?;
    started(host, &a)?;
    let in_b = started(host, &b)?;
    lacks("ps(a)", &ps(host, &a)?, in_b)
}

// --- helpers ---

/// The working directory of every session the cases ensure: the system's temporary
/// directory.
fn temp_dir() -> String {
    env::temp_dir().to_string_lossy().into_owned()
}

/// `sleep 30`, the command every case runs.
fn sleep() -> Argv {
    Argv::new(vec!["sleep".to_owned(), "30".to_owned()])
}

/// The session named `text`, ensured in the temporary directory.
fn ensured(host: &dyn HostPort, text: &str) -> Result<PaneName, String> {
    let name = succeeds("PaneName::parse", PaneName::parse(text))?;
    succeeds(
        &format!("ensure_session({name})"),
        host.ensure_session(&name, &temp_dir()),
    )?;
    Ok(name)
}

/// `ps` of `name`, which must succeed.
fn ps(host: &dyn HostPort, name: &PaneName) -> Result<Vec<u32>, String> {
    succeeds(&format!("ps({name})"), host.ps(name))
}

/// Run `sleep 30` in `name`, which must succeed and add a process to `ps`; returns the
/// pid of that process.
fn started(host: &dyn HostPort, name: &PaneName) -> Result<u32, String> {
    let before = ps(host, name)?;
    let call = format!("run({name}, sleep 30)");
    succeeds(&call, host.run(name, &sleep()))?;
    let after = ps(host, name)?;
    after
        .iter()
        .copied()
        .find(|pid| !before.contains(pid))
        .ok_or_else(|| {
            format!("{call} added no process: ps was {before:?} before and {after:?} after")
        })
}

/// `Ok` when `pids`, what `what` read, holds `pid`.
fn holds(what: &str, pids: &[u32], pid: u32) -> Result<(), String> {
    if pids.contains(&pid) {
        Ok(())
    } else {
        Err(format!("{what} lacks pid {pid}: {pids:?}"))
    }
}

/// `Ok` when `pids`, what `what` read, lacks `pid`.
fn lacks(what: &str, pids: &[u32], pid: u32) -> Result<(), String> {
    if pids.contains(&pid) {
        Err(format!("{what} still holds pid {pid}: {pids:?}"))
    } else {
        Ok(())
    }
}

fn sorted(mut pids: Vec<u32>) -> Vec<u32> {
    pids.sort_unstable();
    pids
}
