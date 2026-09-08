//! ADR 0003 invocation tests (story #327, RED first).
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
#[rstest]
#[case::hub_serve(&["hub", "serve", "--listen", "127.0.0.1:8700"])]
#[case::hub_token_mint(&["hub", "token", "mint", "--label", "x"])]
#[case::hub_token_list(&["hub", "token", "list"])]
#[case::hub_token_delete(&["hub", "token", "delete", "ID"])]
#[case::hub_token_revoke(&["hub", "token", "revoke", "ID"])]
#[case::hub_token_ping(&["hub", "token", "ping", "ID"])]
#[case::hub_status(&["hub", "status"])]
#[case::hub_caps(&["hub", "caps"])]
#[case::hub_support(&["hub", "support", "FEATURE"])]
#[case::hub_query_local(&["hub", "query", "CMD"])]
#[case::hub_query_remote(&["hub", "query", "TARGET", "CMD"])]
#[case::roster(&["roster"])]
#[case::say(&["say", "SESSION", "TEXT"])]
#[case::interrupt(&["interrupt", "SESSION"])]
#[case::body_join(&["body", "join", "--server", "URL", "--token", "ID:SECRET"])]
#[case::body_run(&["body", "run"])]
#[case::body_detach(&["body", "detach"])]
#[case::body_status(&["body", "status"])]
#[case::body_caps(&["body", "caps"])]
#[case::body_support(&["body", "support", "FEATURE"])]
#[case::body_query(&["body", "query", "CMD"])]
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
