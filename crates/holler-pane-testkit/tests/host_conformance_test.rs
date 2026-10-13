#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
//! The fake host passes its own conformance suite, the suite lists the cases it
//! documents, and a host that breaks one rule fails the case that pins that rule (the
//! mutation check; #638, slice e).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The suite is
//! the unit under test here: a suite that cannot tell a broken host from a good one
//! proves nothing about the tmux adapter later.

use holler_pane::{Argv, HostPort, PaneError, PaneName};
use holler_pane_testkit::conformance::host::{host_cases, run_host_conformance};
use holler_pane_testkit::conformance::Conformance;
use holler_pane_testkit::host::FakeHost;

/// The 9 case ids of the brief's table, in order, #835's `foreground_commands` case, #647
/// part 2's `attached_clients` case and #831's `foreground_pids` case.
const DOCUMENTED_CASES: [&str; 12] = [
    "ps-of-missing-session-is-pane-not-found",
    "run-in-missing-session-is-pane-not-found",
    "run-adds-a-process",
    "ensure-session-is-idempotent",
    "run-empty-argv-is-usage",
    "stop-owned-stops-every-owned-process",
    "stop-owned-of-missing-session-is-ok",
    "stop-owned-leaves-other-sessions",
    "ps-lists-only-its-session",
    "foreground-commands-names-what-run-started",
    "attached-clients-of-fresh-session-is-zero-and-of-missing-is-pane-not-found",
    "foreground-pids-hold-what-run-started-and-of-missing-is-pane-not-found",
];

#[test]
fn the_fake_passes_the_host_conformance_suite() {
    let result = run_host_conformance(|| (FakeHost::new(), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_host_suite_runs_the_documented_cases() {
    assert_eq!(host_cases(), DOCUMENTED_CASES.to_vec());
}

// --- mutation check: a broken host fails, on the named case ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct host: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `ps` of a missing session answers an empty list.
    PsOfMissingIsEmpty,
    /// `run` creates the missing session first.
    RunCreatesMissingSession,
    /// `run` starts nothing.
    RunIsNoop,
    /// A second `ensure_session` recreates the session (its processes are gone).
    EnsureRecreates,
    /// An empty argv runs something.
    RunsEmptyArgv,
    /// `stop_owned` stops nothing.
    StopIsNoop,
    /// `stop_owned` of a missing session fails.
    StopMissingFails,
    /// `stop_owned` stops every session, the broad kill the adapter must not do.
    StopsEverySession,
    /// `ps` lists the processes of every session.
    PsListsEverySession,
    /// `foreground_commands` names nothing, as a default that guesses would (#835).
    ForegroundNamesNothing,
    /// `foreground_commands` of a missing session answers an empty list.
    ForegroundOfMissingIsEmpty,
    /// `foreground_pids` names nothing, as a default that guesses would (#831).
    ForegroundPidsNothing,
    /// `foreground_pids` of a missing session answers an empty list.
    ForegroundPidsOfMissingIsEmpty,
    /// `foreground_pids` answers its pids descending (the order is part of the port).
    ForegroundPidsDescending,
    /// `foreground_pids` repeats every pid.
    ForegroundPidsRepeat,
    /// `attached_clients` always answers 1, as a wrapper that answers a constant would
    /// (#647 part 2).
    AttachedAlwaysOne,
    /// `attached_clients` of a missing session answers 0.
    AttachedOfMissingIsZero,
}

struct Mutant {
    inner: FakeHost,
    broken: Break,
}

impl Mutant {
    fn new(broken: Break) -> Self {
        Self {
            inner: FakeHost::new(),
            broken,
        }
    }

    fn exists(&self, name: &PaneName) -> bool {
        self.inner.sessions().contains(name)
    }
}

fn not_found(name: &PaneName) -> PaneError {
    PaneError::PaneNotFound {
        what: name.to_string(),
    }
}

impl HostPort for Mutant {
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        if self.broken == Break::EnsureRecreates && self.exists(name) {
            self.inner.end_session(name)?;
        }
        self.inner.ensure_session(name, cwd)
    }

    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        match self.broken {
            Break::RunCreatesMissingSession if !self.exists(name) => {
                self.inner.ensure_session(name, "/")?;
                self.inner.run(name, cwd, argv)
            }
            Break::RunIsNoop => {
                if self.exists(name) {
                    Ok(())
                } else {
                    Err(not_found(name))
                }
            }
            Break::RunsEmptyArgv if argv.as_slice().is_empty() => {
                self.inner
                    .run(name, cwd, &Argv::new(vec!["true".to_owned()]))
            }
            _ => self.inner.run(name, cwd, argv),
        }
    }

    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        match self.broken {
            Break::StopIsNoop => Ok(()),
            Break::StopMissingFails if !self.exists(name) => Err(not_found(name)),
            Break::StopsEverySession => {
                for each in self.inner.sessions() {
                    self.inner.stop_owned(&each)?;
                }
                self.inner.stop_owned(name)
            }
            _ => self.inner.stop_owned(name),
        }
    }

    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        match self.broken {
            Break::PsOfMissingIsEmpty if !self.exists(name) => Ok(vec![]),
            Break::PsListsEverySession => {
                let own = self.inner.ps(name)?;
                let mut all = own;
                for each in self.inner.sessions() {
                    if &each != name {
                        all.extend(self.inner.ps(&each)?);
                    }
                }
                Ok(all)
            }
            _ => self.inner.ps(name),
        }
    }

    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        match self.broken {
            Break::ForegroundNamesNothing => self.inner.foreground_commands(name).map(|_| vec![]),
            Break::ForegroundOfMissingIsEmpty if !self.exists(name) => Ok(vec![]),
            _ => self.inner.foreground_commands(name),
        }
    }

    fn foreground_pids(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        match self.broken {
            Break::ForegroundPidsNothing => self.inner.foreground_pids(name).map(|_| vec![]),
            Break::ForegroundPidsOfMissingIsEmpty if !self.exists(name) => Ok(vec![]),
            Break::ForegroundPidsDescending => self.inner.foreground_pids(name).map(|mut pids| {
                pids.reverse();
                pids
            }),
            Break::ForegroundPidsRepeat => self
                .inner
                .foreground_pids(name)
                .map(|pids| pids.iter().chain(pids.iter()).copied().collect()),
            _ => self.inner.foreground_pids(name),
        }
    }

    fn attached_clients(&self, name: &PaneName) -> Result<u32, PaneError> {
        match self.broken {
            Break::AttachedAlwaysOne => Ok(1),
            Break::AttachedOfMissingIsZero if !self.exists(name) => Ok(0),
            _ => self.inner.attached_clients(name),
        }
    }
}

/// The suite must reject the run and name `case` among its failures.
fn assert_fails_on(result: Conformance, case: &str) {
    let failures = result.expect_err("a broken host must not pass the suite");
    assert!(
        failures.iter().any(|f| f.case == case),
        "expected case `{case}` among the failures, got: {failures:?}"
    );
    assert!(
        failures.iter().all(|f| !f.detail.is_empty()),
        "every failure says why: {failures:?}"
    );
}

fn assert_suite_fails_on(broken: Break, case: &str) {
    assert_fails_on(run_host_conformance(|| (Mutant::new(broken), ())), case);
}

#[test]
fn the_unbroken_host_wrapper_passes() {
    let result = run_host_conformance(|| (Mutant::new(Break::Nothing), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn a_host_whose_ps_of_a_missing_session_is_empty_fails() {
    assert_suite_fails_on(
        Break::PsOfMissingIsEmpty,
        "ps-of-missing-session-is-pane-not-found",
    );
}

#[test]
fn a_host_whose_run_creates_the_missing_session_fails() {
    assert_suite_fails_on(
        Break::RunCreatesMissingSession,
        "run-in-missing-session-is-pane-not-found",
    );
}

#[test]
fn a_host_whose_run_starts_nothing_fails() {
    assert_suite_fails_on(Break::RunIsNoop, "run-adds-a-process");
}

#[test]
fn a_host_whose_second_ensure_recreates_the_session_fails() {
    assert_suite_fails_on(Break::EnsureRecreates, "ensure-session-is-idempotent");
}

#[test]
fn a_host_that_runs_an_empty_argv_fails() {
    assert_suite_fails_on(Break::RunsEmptyArgv, "run-empty-argv-is-usage");
}

#[test]
fn a_host_whose_stop_owned_stops_nothing_fails() {
    assert_suite_fails_on(Break::StopIsNoop, "stop-owned-stops-every-owned-process");
}

#[test]
fn a_host_whose_stop_owned_of_a_missing_session_is_an_error_fails() {
    assert_suite_fails_on(
        Break::StopMissingFails,
        "stop-owned-of-missing-session-is-ok",
    );
}

#[test]
fn a_host_whose_stop_owned_stops_every_session_fails() {
    assert_suite_fails_on(Break::StopsEverySession, "stop-owned-leaves-other-sessions");
}

#[test]
fn a_host_whose_ps_lists_every_session_fails() {
    assert_suite_fails_on(Break::PsListsEverySession, "ps-lists-only-its-session");
}

#[test]
fn a_host_whose_foreground_commands_names_nothing_fails() {
    assert_suite_fails_on(
        Break::ForegroundNamesNothing,
        "foreground-commands-names-what-run-started",
    );
}

#[test]
fn a_host_whose_foreground_commands_of_a_missing_session_is_empty_fails() {
    assert_suite_fails_on(
        Break::ForegroundOfMissingIsEmpty,
        "foreground-commands-names-what-run-started",
    );
}

#[test]
fn a_host_whose_attached_clients_is_always_one_fails() {
    assert_suite_fails_on(
        Break::AttachedAlwaysOne,
        "attached-clients-of-fresh-session-is-zero-and-of-missing-is-pane-not-found",
    );
}

#[test]
fn a_host_whose_attached_clients_of_a_missing_session_is_zero_fails() {
    assert_suite_fails_on(
        Break::AttachedOfMissingIsZero,
        "attached-clients-of-fresh-session-is-zero-and-of-missing-is-pane-not-found",
    );
}

const FOREGROUND_PIDS_CASE: &str =
    "foreground-pids-hold-what-run-started-and-of-missing-is-pane-not-found";

#[test]
fn a_host_whose_foreground_pids_names_nothing_fails() {
    assert_suite_fails_on(Break::ForegroundPidsNothing, FOREGROUND_PIDS_CASE);
}

#[test]
fn a_host_whose_foreground_pids_of_a_missing_session_is_empty_fails() {
    assert_suite_fails_on(Break::ForegroundPidsOfMissingIsEmpty, FOREGROUND_PIDS_CASE);
}

/// The case starts two processes, so that the order of the pids is observable.
#[test]
fn a_host_whose_foreground_pids_are_not_ascending_fails() {
    assert_suite_fails_on(Break::ForegroundPidsDescending, FOREGROUND_PIDS_CASE);
}

#[test]
fn a_host_whose_foreground_pids_repeat_a_pid_fails() {
    assert_suite_fails_on(Break::ForegroundPidsRepeat, FOREGROUND_PIDS_CASE);
}
