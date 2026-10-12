//! `roster_cmd::run`, the hub call before the pure half (acceptance 11, A's W-1 and W-4). The
//! only cases of this target that reach the hub call: each runs under one lock with
//! `HOLLER_STATE_DIR` on an empty scratch dir, so no case can reach the default state dir's
//! live hub (A's W-6). None reaches a real hub: the dir has none, and a non-loopback
//! `ws://` is refused before any dial.

use std::cell::Cell;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use holler_cli::roster_cmd;
use holler_pane::{PaneError, Ports};
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fault::Fault;

use crate::{base_panes, capture, parse, Mode, Rig, Run};

static STATE_DIR_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with `HOLLER_STATE_DIR` on a fresh empty dir, under the lock.
fn with_empty_state_dir<R>(f: impl FnOnce(&Path) -> R) -> R {
    let _held = STATE_DIR_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    // Edition 2021: `set_var` is safe, and the lock serializes it within this target.
    std::env::set_var("HOLLER_STATE_DIR", dir.path());
    f(dir.path())
}

/// `holler roster <args>` through `run`, with a source of ports that records whether it was
/// called.
fn run(args: &[&str], mode: Mode, ports: Ports<'_>) -> (Run, bool) {
    let (cli, choice) = parse(args, mode);
    let holler_cli::Command::Roster(roster) = &cli.command else {
        unreachable!("parsed `holler roster`")
    };
    let called = Cell::new(false);
    let source = || -> Result<Ports<'_>, PaneError> {
        called.set(true);
        Ok(ports)
    };
    let run = capture(|sink| roster_cmd::run(roster, choice, source, sink));
    (run, called.get())
}

/// Acceptance 11 (round 6): with no live hub the roster exits 1 exactly as today, whatever
/// the registry would answer, and never asks for the registry. Text and `--json` print
/// today's one line; `--format=json` prints the failure envelope coded `unavailable`. A
/// seeded store and one whose `list` fails `unavailable` give the same bytes.
#[test]
fn with_no_hub_the_result_is_todays_whatever_the_store() {
    let seeded = Rig::new(base_panes());
    let failing = Rig::new(base_panes());
    failing
        .panes
        .faults()
        .set(Some(Fault::Fail(PaneError::Unavailable {
            what: "the hub".to_owned(),
        })));
    with_empty_state_dir(|dir| {
        let today = format!("no live holler hub reachable at {}", dir.display());
        for mode in Mode::ALL {
            let (first, asked) = run(&[], mode, seeded.ports());
            assert!(
                !asked,
                "{mode:?}: the registry is not asked for before a reply"
            );
            let (second, asked) = run(&[], mode, failing.ports());
            assert!(
                !asked,
                "{mode:?}: the registry is not asked for before a reply"
            );
            assert_eq!(
                first, second,
                "{mode:?}: the store does not change the result"
            );
            assert_eq!(first.code, 1, "{mode:?}: {first:?}");
            if mode == Mode::Envelope {
                let envelope =
                    check_envelope(&first.out, 1).unwrap_or_else(|f| panic!("{f:?}: {first:?}"));
                let error = envelope.error.unwrap();
                assert_eq!(
                    (error.code.as_str(), error.message.as_str()),
                    ("unavailable", today.as_str())
                );
                assert_eq!(first.err, "", "{first:?}");
            } else {
                assert_eq!(
                    (first.out.as_str(), first.err.clone()),
                    ("", format!("error: {today}\n")),
                    "{mode:?}"
                );
            }
        }
    });
}

/// A's W-4: a `--server` refused by policy (a plaintext `ws://` to a non-loopback host,
/// #508's AC 13) stays exit 3 in every mode: today's line in text and under `--json`, a
/// failure envelope under `--format=json`, whose code is a refusal's (exit 3).
#[test]
fn a_server_refused_by_policy_exits_3_in_every_mode() {
    let rig = Rig::new(base_panes());
    with_empty_state_dir(|_| {
        let today = "a plaintext ws:// to a non-loopback host is refused; off-loopback pairing is wss:// (ADR 0002)";
        for mode in Mode::ALL {
            let (run, asked) = run(&["--server", "ws://example.com:1"], mode, rig.ports());
            assert!(!asked, "{mode:?}: --server never reads the registry");
            assert_eq!(run.code, 3, "{mode:?}: {run:?}");
            if mode == Mode::Envelope {
                let envelope =
                    check_envelope(&run.out, 3).unwrap_or_else(|f| panic!("{f:?}: {run:?}"));
                assert_eq!(envelope.error.unwrap().message, today);
                assert_eq!(run.err, "", "{run:?}");
            } else {
                assert_eq!(
                    (run.out.as_str(), run.err.clone()),
                    ("", format!("error: {today}\n")),
                    "{mode:?}"
                );
            }
        }
    });
}

/// R3 (acceptance 13 (a), in-process): `--profile` with `--server` is a usage error found
/// before any dial: one line on err, nothing on out, exit 2, in every mode.
#[test]
fn profile_with_server_is_a_usage_line_before_any_dial() {
    let rig = Rig::new(base_panes());
    with_empty_state_dir(|_| {
        for mode in Mode::ALL {
            let args = ["--profile", "demo", "--server", "ws://example.com:1"];
            let (run, asked) = run(&args, mode, rig.ports());
            assert!(!asked, "{mode:?}");
            let expected = (
                "",
                "error: --profile cannot be combined with --server\n".to_owned(),
            );
            assert_eq!(
                (run.code, (run.out.as_str(), run.err.clone())),
                (2, expected),
                "{mode:?}"
            );
        }
    });
}
