#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #127
//! ADR 0003 invocation tests (story #127, RED first).
//!
//! These pin the normative CLI surface: every leaf in ADR 0003 must parse
//! (and, in the skeleton, report "not implemented" exit 1), `--version`
//! prints the crate version, `--help` prints usage, and a bare or unknown
//! invocation fails closed with exit 2.

use assert_cmd::Command;
use predicates::function::function;
use predicates::prelude::*;
use predicates::str::contains;
use rstest::rstest;

// Story #143's dedicated cases below (`hub_serve_is_recognised`) spawn a real
// hub and tear it down as a process tree, so they reuse the shared harness.
mod support;

fn holler() -> Command {
    Command::cargo_bin("holler").expect("holler binary on PATH")
}

// --version (and -V) print exactly `holler <workspace version>` and exit 0.
#[test]
fn version_flag_prints_crate_version() {
    for flag in ["--version", "-V"] {
        holler()
            .arg(flag)
            .assert()
            .success()
            .stdout(predicate::eq("holler 0.1.0\n"));
    }
}

// --help prints usage that names the top-level verbs and exits 0.
#[test]
fn help_prints_usage_exit_0() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let captured = seen.clone();
    holler()
        .arg("--help")
        .assert()
        .success()
        .stdout(function(move |s: &str| {
            *captured.lock().unwrap() = s.to_string();
            true
        }));
    let help = seen.lock().unwrap().clone();
    for word in ["hub", "body", "roster", "say", "interrupt"] {
        assert!(
            help.contains(word),
            "--help output should mention `{word}`: {help}"
        );
    }
}

// A bare invocation, a role with no subcommand, and an unknown subcommand all
// fail closed: exit 2 with a non-empty usage message on stderr (ADR 0003).
#[rstest]
#[case::bare(&[])]
#[case::hub_bare(&["hub"])]
#[case::body_bare(&["body"])]
#[case::unknown(&["bogus"])]
fn bare_invocation_fails_closed(#[case] args: &[&str]) {
    holler()
        .args(args)
        .assert()
        .failure()
        .code(2)
        .stderr(contains("Usage"));
}

// Every leaf in ADR 0003 parses. In the skeleton each reports "not
// implemented" on stderr and exits 1 (ADR 0003: exit 1 = runtime failure,
// which "not implemented" is until the owning story lands). Proving the verb
// parses is the point: a typo'd or unlisted spelling would exit 2 instead.
// NOTE (story #143): `hub serve` and `hub status` were originally listed here
// as skeleton leaves that print "not implemented" and exit 1. #143 implements
// them: `hub serve` is now a *long-running* server (it never exits on its own,
// so it can't be asserted with `.assert()`, which waits for process exit) and
// `hub status` now exits 1 with a *real* "no live hub reachable" message (ADR
// 0003: exit 1 = unreachable hub) rather than "not implemented". Both are
// therefore removed from this "not implemented" table and pinned by dedicated
// tests below (`hub_serve_is_recognised`, `hub_status_without_live_hub`).
//
// `--json` appears on the leaves **before** the subcommand path (e.g.
// `--json hub caps`): it is a *global* flag on the root (ADR 0003), so clap
// accepts it anywhere in the command line. The `x_json` cases pin that
// placement — the exact regression a per-leaf `--json` would reintroduce (with
// a leaf-local flag, `--json` after the leaf works but before it is a clap
// error / exit 2).
//
// NOTE (story #163): `hub status` (#143) and the eight `hub token …` leaves
// (#163) were originally listed here as skeleton leaves that print "not
// implemented" and exit 1. #143 implemented `hub status` (exit 1 = unreachable
// hub, a *real* diagnostic) and #163 implements the `hub token` leaves (they
// now operate the real file-backed token store and exit 0 / 1 / 3 per ADR 0003,
// never "not implemented"). All nine are therefore removed from this "not
// implemented" table and pinned by dedicated tests: `hub status` by
// `hub_status_without_live_hub` (below), the `hub token` leaves by
// `crates/holler-cli/tests/token_cli_test.rs`. The `cli-surface.txt` fixture
// (which #149 adds `--json` rows for the token leaves) keeps pinning the *parse*
// guarantee — clap accepts every line — independently of their run-time exit.
//
// NOTE (story #176): `body join`, `body detach`, and `body status` (with the
// `--json` arm) were originally listed here as skeleton leaves that print
// "not implemented" and exit 1. #176 implements them: `body join` redeems a
// one-time join secret over WebSocket and exits 0/1/3, `body detach` removes
// the local credential and exits 0, and `body status` reports the local join
// state and exits 0. They are therefore removed from this "not implemented"
// table and pinned by dedicated tests in `crates/holler-cli/tests/body_join_test.rs`.
// The `cli-surface.txt` fixture keeps pinning the *parse* guarantee for these
// rows (clap accepts every line) independently of their run-time exit.
#[rstest]
#[case::hub_caps(&["hub", "caps"])]
#[case::hub_caps_json(&["--json", "hub", "caps"])]
#[case::hub_support(&["hub", "support", "FEATURE"])]
#[case::hub_support_json(&["--json", "hub", "support", "FEATURE"])]
#[case::hub_query_local(&["hub", "query", "CMD"])]
#[case::hub_query_local_json(&["--json", "hub", "query", "CMD"])]
#[case::hub_query_remote(&["hub", "query", "TARGET", "CMD"])]
#[case::hub_query_remote_json(&["--json", "hub", "query", "TARGET", "CMD"])]
#[case::roster(&["roster"])]
#[case::roster_json(&["--json", "roster"])]
#[case::say(&["say", "SESSION", "TEXT"])]
#[case::say_json(&["--json", "say", "SESSION", "TEXT"])]
#[case::interrupt(&["interrupt", "SESSION"])]
#[case::body_run(&["body", "run"])]
#[case::body_caps(&["body", "caps"])]
#[case::body_caps_json(&["--json", "body", "caps"])]
#[case::body_support(&["body", "support", "FEATURE"])]
#[case::body_support_json(&["--json", "body", "support", "FEATURE"])]
#[case::body_query(&["body", "query", "CMD"])]
#[case::body_query_json(&["--json", "body", "query", "CMD"])]
#[case::body_attach_sessions(&["body", "attach", "sessions"])]
#[case::body_attach_init(&["body", "attach", "init"])]
fn every_adr_0003_leaf_parses(#[case] args: &[&str]) {
    holler()
        .args(args)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented"));
}

// Story #143 made `hub serve` a real, *long-running* server. We can't use
// `.assert()` here (it blocks until the child exits, and a server never does);
// the whole serve lifecycle is pinned by `hub_serve_test.rs`. What this case
// still owes ADR 0003 is the *parse* guarantee: `serve` is a recognised `hub`
// leaf, so it must NOT fail closed as an unknown subcommand (exit 2). We prove
// that by giving it a loopback listen target and a *definite* policy refusal —
// another hub already holding the instance lock — so the process exits (code 3)
// instead of running forever, and we assert it did so (i.e. it parsed and ran,
// rather than bailing at exit 2 as "unknown").
//
// It is self-cleaning: it starts one hub (its own process group, like every
// harness spawn) to hold the instance lock, asserts the second `serve` is
// refused with exit 3, then kills the first's whole tree and removes the temp
// state dir.
#[test]
fn hub_serve_is_recognised() {
    use support::{kill_tree, make_own_process_group};
    let state = std::env::temp_dir().join(format!("holler-inv-serve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    let _ = std::fs::create_dir_all(state.join("hub"));
    // Hold the instance lock (the first hub), on a loopback port, as its own
    // process group so we can tear the whole tree down at the end.
    let mut holder_cmd = std::process::Command::new(env!("CARGO_BIN_EXE_holler"));
    holder_cmd
        .args(["hub", "serve", "--listen", "127.0.0.1:0"])
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    make_own_process_group(&mut holder_cmd);
    let mut holder = holder_cmd
        .spawn()
        .expect("start the first (lock-holding) hub");

    // Poll until the first hub has bound (the lock is held before it binds, so
    // `listening.json` existing is a sound proxy for "lock is held").
    let mut locked = false;
    for _ in 0..100 {
        if state.join("hub").join("listening.json").exists() {
            locked = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(locked, "first hub did not come up to hold the lock");

    // The *second* `serve` must parse (not exit 2) and then be refused because
    // the lock is held: exit 3 (fail-closed policy refusal).
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["hub", "serve", "--listen", "127.0.0.1:0"])
        .assert()
        .failure()
        .code(3);

    // Tear the first hub's whole tree down (reap; no zombie, no orphan).
    kill_tree(&mut holder);
    let _ = std::fs::remove_dir_all(&state);
}

// Story #143 implemented `hub status`: with no live hub it exits 1 (ADR 0003:
// "1 runtime failure (unreachable hub…)") with a *real* diagnostic — it no
// longer prints the skeleton's "not implemented". This pins both facts: the
// leaf parses (does not exit 2) and the unreachable-hub path exits 1. The
// `--json` form is asserted in the same test (#147 item 3): it is a root-level
// global flag, so `--json` *before* `hub status` must also parse (not exit 2)
// and take the same unreachable-hub exit-1 path, with a JSON document on
// stdout instead of the human summary.
#[test]
fn hub_status_without_live_hub() {
    let state = std::env::temp_dir().join(format!("holler-inv-status-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&state);
    // Human form: exit 1, real diagnostic (not the skeleton's "not
    // implemented").
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["hub", "status"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not());
    // `--json` global placement: still parses (not exit 2) and still exits 1
    // (unreachable hub) — the JSON arm of the same command.
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["--json", "hub", "status"])
        .assert()
        .failure()
        .code(1);
    let _ = std::fs::remove_dir_all(&state);
}
