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
//
// NOTE (issue #182): `body run` was originally listed here too. #182
// implements the live connection loop; an unjoined body now exits 1 with a
// real "not joined" message rather than "not implemented", so it moves to its
// own pinned case (`body_run_unjoined_exits_1`, below) — the full connect/
// authenticate/reconnect/detach lifecycle against a live hub is pinned by
// `crates/holler-cli/tests/body_run_test.rs`.
//
// NOTE (issue #185): `hub caps`/`support`/`query` and `body caps`/`support`/
// `query` were originally listed here too (12 cases, `--json` arms
// included). #185 implements all six leaves: `body caps`/`support`/`query`
// answer from local state and exit 0 (or 1 on an unknown `query/support`
// feature id, 2 on a malformed query tail), `hub caps`/`support`/`query`
// round-trip the live hub's control socket and exit 1 with the real "no live
// holler hub reachable" message when none is up — never "not implemented".
// All twelve are therefore removed from this "not implemented" table and
// pinned by dedicated tests in `crates/holler-cli/tests/query_test.rs`.
//
// NOTE (issue #190): `say`/`say --json` were originally listed here too.
// `say` now does something real — it exits 1 with a genuine "no live holler
// hub reachable" diagnostic rather than "not implemented" when there is no
// hub to route through. Moved to its own pinned case (`say_without_live_hub_exits_1`,
// below), the same treatment `hub_status_without_live_hub` got for the same
// reason. The full route/busy-check/coalesce/TalkLog path against a live
// hub+body is pinned by `crates/holler-cli/tests/talk_test.rs`.
// NOTE (issue #186): `roster`/`roster --json` were originally listed here too.
// `roster` now does something real — it exits 1 with a genuine "no live holler
// hub reachable" diagnostic rather than "not implemented" when there is no
// hub to read the roster from. Moved to its own pinned case
// (`roster_without_live_hub_exits_1`, below), the same treatment `say` got
// for the same reason. The full live view (TTL tri-state, collision, --all)
// is pinned by `crates/holler-cli/tests/roster_cli_test.rs`.
// NOTE (issue #191): `interrupt SESSION` was originally listed here too.
// `interrupt` now does something real — it exits 1 with a genuine "no live
// holler hub reachable" diagnostic rather than "not implemented" when there
// is no hub to route through. Moved to its own pinned case
// (`interrupt_without_live_hub_exits_1`, below), the same treatment `say`/
// `roster` got for the same reason. The full cancel/ack-timeout/redirect path
// against a live hub+body is pinned by
// `crates/holler-cli/tests/interrupt_test.rs`.
// NOTE (issue #196): `body attach sessions`/`body attach init` were
// originally listed here too (the last two rows of what was this file's
// `every_adr_0003_leaf_parses` table — with these gone the table was empty,
// so the whole rstest is retired here rather than left with zero cases).
// They now do something real — a pure HTTP GET against the (default,
// unreachable-in-this-test) `http://127.0.0.1:4096` OpenCode endpoint,
// exiting 1 with a genuine "could not reach" diagnostic rather than "not
// implemented". Pinned by the two cases below, the same treatment
// `say`/`roster`/`interrupt` got for the same reason. The full
// sessions-table/`--json`/`init`/`--force` behaviour against a fake OpenCode
// HTTP server is pinned by `crates/holler-cli/tests/attach_cli_test.rs`.
#[test]
fn body_attach_sessions_without_live_endpoint_exits_1() {
    holler()
        .args(["body", "attach", "sessions"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not())
        .stderr(contains("127.0.0.1:4096"));
}

#[test]
fn body_attach_init_without_live_endpoint_exits_1() {
    holler()
        .args(["body", "attach", "init"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not())
        .stderr(contains("127.0.0.1:4096"));
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

// Issue #190: `say` against a state dir with no live hub exits 1 with the
// spec's exact diagnostic rather than the skeleton's "not implemented" — the
// same "parses, and takes the real unreachable-hub path" pin
// `hub_status_without_live_hub` gives `hub status` above. The full
// route/busy-check/coalesce/TalkLog path against a real hub+body is pinned by
// `crates/holler-cli/tests/talk_test.rs`.
#[test]
fn say_without_live_hub_exits_1() {
    let state = std::env::temp_dir().join(format!("holler-inv-say-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&state);
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["say", "SESSION", "TEXT"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not())
        .stderr(contains("no live holler hub reachable"));
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["--json", "say", "SESSION", "TEXT"])
        .assert()
        .failure()
        .code(1);
    let _ = std::fs::remove_dir_all(&state);
}

// Issue #186: `roster` against a state dir with no live hub exits 1 with the
// spec's exact diagnostic rather than the skeleton's "not implemented" — the
// same "parses, and takes the real unreachable-hub path" pin
// `hub_status_without_live_hub` gives `hub status` above. The full live view
// (TTL tri-state, collision, `--all`) against a real hub is pinned by
// `crates/holler-cli/tests/roster_cli_test.rs`.
#[test]
fn roster_without_live_hub_exits_1() {
    let state = std::env::temp_dir().join(format!("holler-inv-roster-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&state);
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["roster"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not())
        .stderr(contains("no live holler hub reachable"));
    // `--json` is a global flag, so it also takes the same unreachable path.
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["--json", "roster"])
        .assert()
        .failure()
        .code(1);
    let _ = std::fs::remove_dir_all(&state);
}

// Issue #191: `interrupt SESSION` against a state dir with no live hub exits
// 1 with the spec's exact diagnostic rather than the skeleton's "not
// implemented" — the same "parses, and takes the real unreachable-hub path"
// pin `say_without_live_hub_exits_1`/`roster_without_live_hub_exits_1` give
// their own verbs above. The full cancel/ack-timeout/redirect path against a
// real hub+body is pinned by `crates/holler-cli/tests/interrupt_test.rs`.
#[test]
fn interrupt_without_live_hub_exits_1() {
    let state = std::env::temp_dir().join(format!("holler-inv-interrupt-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&state);
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["interrupt", "SESSION"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not implemented").not())
        .stderr(contains("no live holler hub reachable"));
    // `--json` is a global flag, so it also takes the same unreachable path.
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["--json", "interrupt", "SESSION"])
        .assert()
        .failure()
        .code(1);
    let _ = std::fs::remove_dir_all(&state);
}

// Issue #182: `body run` against a state dir that has never joined exits 1
// with a real "not joined" diagnostic (not the skeleton's "not implemented",
// and not a hang — a run with nothing to authenticate returns immediately).
// The live connect/authenticate/reconnect/detach lifecycle is pinned by
// `body_run_test.rs`, which needs a live hub; this case only pins the
// unjoined short-circuit, so it stays in this file's fast, hub-free suite.
#[test]
fn body_run_unjoined_exits_1() {
    let state = std::env::temp_dir().join(format!("holler-inv-run-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&state);
    holler()
        .env("HOLLER_STATE_DIR", &state)
        .env("NO_COLOR", "1")
        .args(["body", "run"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not joined"));
    let _ = std::fs::remove_dir_all(&state);
}
