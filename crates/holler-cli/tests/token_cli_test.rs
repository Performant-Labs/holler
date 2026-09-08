#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #163
//! `holler hub token …` e2e (story #163).
//!
//! #143 removed `hub status` (and #163 removes the eight `hub token` leaves)
//! from `cli_invocation_test`'s "not implemented" table because they are now
//! real commands with ADR 0003 exit codes. This file pins the run-time
//! behaviour the parse-only fixture (`cli-surface.txt`, via #149) cannot: each
//! verb drives the **real** `holler` binary against an isolated
//! `HOLLER_STATE_DIR` and asserts its exit code and stdout/stderr shape.
//!
//! The tokens these tests operate live in the store file
//! (`<state>/hub/tokens.json`) — the same file the `token_store_test.rs`
//! integration test exercises in-process. Here we exercise the CLI's thin
//! layer over that store (its exit codes and human/JSON output).

mod support;

use support::{holler_cmd, StateDir};
use std::process::Stdio;

use serde_json::Value;

/// Run one `holler` invocation (piped stdio, the given state dir) and return
/// its exit code plus the decoded stdout/stderr strings.
fn run(state: &StateDir, args: &[&str]) -> (i32, String, String) {
    let out = holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `hub token mint --label L` (no `--json`): exit 0, stdout carries the token
/// id, the one-time secret (shown once, not stored), the expiry, and a
/// ready-to-paste `holler body join` line; the `hub/` dir exists.
#[test]
fn mint_human_exit_0_prints_secret_and_join_line() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code, 0, "mint must exit 0; stderr: {stderr}");
    assert!(stdout.contains("token_id:"), "stdout names the token_id; got: {stdout}");
    assert!(stdout.contains("hlr_join_"), "the one-time secret is printed; got: {stdout}");
    assert!(stdout.contains("holler body join"), "a join line is printed; got: {stdout}");
    assert!(state.hub().join("tokens.json").exists(), "tokens.json was created");
}

/// `--json hub token mint --label L`: exit 0, stdout is a JSON object carrying
/// `token_id`, `secret` (a `hlr_join_`-prefixed one-time secret), `expires`
/// (a positive epoch), and a `join_command` naming the token id.
#[test]
fn mint_json_exit_0_document_shape() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(
        &state,
        &["--json", "hub", "token", "mint", "--label", "alice", "--ttl", "2h"],
    );
    assert_eq!(code, 0, "mint --json must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("mint --json stdout is valid JSON");
    let token_id = doc["token_id"].as_str().expect("token_id present");
    assert!(token_id.starts_with("tok_"), "token_id is a tok_ id: {token_id}");
    let secret = doc["secret"].as_str().expect("secret present");
    assert!(
        secret.starts_with("hlr_join_"),
        "secret is a one-time join secret: {secret}"
    );
    let expires = doc["expires"].as_u64().unwrap_or(0);
    assert!(expires > 0, "expires is a positive epoch: {doc:?}");
    let join = doc["join_command"].as_str().unwrap_or("");
    assert!(
        join.contains(token_id),
        "join_command names the token id: {doc:?}"
    );
}

/// A duplicate label (and a malformed one) is a fail-closed policy refusal:
/// exit 3 with a non-empty stderr, and no token is added.
#[test]
fn mint_duplicate_label_exit_3() {
    let state = StateDir::new();
    let (code, _, _) = run(&state, &["hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code, 0, "first mint must exit 0");
    let (code2, _, stderr2) = run(&state, &["hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code2, 3, "a duplicate label must exit 3; stderr: {stderr2}");
    assert!(
        stderr2.contains("already in use"),
        "the duplicate is reported as already-in-use; stderr: {stderr2}"
    );
    // An invalid label (uppercase, outside the ADR 0005 grammar) is the same
    // fail-closed policy class: exit 3, and no second token is added.
    let (code3, _, stderr3) = run(&state, &["hub", "token", "mint", "--label", "Bad"]);
    assert_eq!(code3, 3, "an invalid label must exit 3; stderr: {stderr3}");
    assert!(
        stderr3.contains("invalid label"),
        "the bad grammar is reported as invalid; stderr: {stderr3}"
    );
}

/// `hub token list` (no `--json`): exit 0, a human table with the pinned
/// column headers and one row for the minted label.
#[test]
fn list_human_exit_0() {
    let state = StateDir::new();
    run(&state, &["hub", "token", "mint", "--label", "alice"]);
    let (code, stdout, stderr) = run(&state, &["hub", "token", "list"]);
    assert_eq!(code, 0, "list must exit 0; stderr: {stderr}");
    for header in ["TOKEN_ID", "LABEL", "STATE", "EXPIRES"] {
        assert!(
            stdout.contains(header),
            "the table has a {header} column; got: {stdout}"
        );
    }
    assert!(stdout.contains("alice"), "the minted label is listed; got: {stdout}");
    assert!(
        stdout.contains("unused"),
        "a freshly minted token is unused; got: {stdout}"
    );
}

/// `--json hub token list`: exit 0, stdout is `{"tokens":[…]}` and the one
/// minted token appears with its id and label.
#[test]
fn list_json_exit_0() {
    let state = StateDir::new();
    run(&state, &["--json", "hub", "token", "mint", "--label", "alice"]);
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "token", "list"]);
    assert_eq!(code, 0, "list --json must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("list --json is valid JSON");
    let arr = doc["tokens"].as_array().expect("a tokens array");
    assert_eq!(arr.len(), 1, "exactly one token; got: {doc:?}");
    assert!(
        arr[0]["label"].as_str() == Some("alice"),
        "the row's label is alice; got: {doc:?}"
    );
    let row_id = arr[0]["token_id"].as_str().unwrap_or("");
    assert!(
        row_id.starts_with("tok_"),
        "the row's token_id is a tok_ id; got: {doc:?}"
    );
}

/// `hub token delete <id>` on an **unused** token: exit 0, prints
/// `invalidated <id> (alice, unused)`.
#[test]
fn delete_unused_exit_0() {
    let state = StateDir::new();
    let (code, stdout, _) = run(&state, &["--json", "hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code, 0);
    let id = serde_json::from_str::<Value>(&stdout)
        .expect("mint --json is JSON")["token_id"]
        .as_str()
        .expect("token_id")
        .to_string();
    let (code2, stdout2, stderr2) = run(&state, &["hub", "token", "delete", &id]);
    assert_eq!(code2, 0, "delete of an unused token must exit 0; stderr: {stderr2}");
    assert!(
        stdout2.contains(&format!("invalidated {id} (alice, unused)")),
        "the human line is 'invalidated {id} (alice, unused)'; got: {stdout2}"
    );
}

/// `hub token revoke <id>` on an **unused** token: exit 0. The store flips both
/// `delete` and `revoke` to `revoked`, and the human line names the *pre*-state
/// word: an unused token is `invalidated` (a `revoked` verb on an already-unused
/// token reads "invalidated … (alice, unused)").
#[test]
fn revoke_unused_exit_0() {
    let state = StateDir::new();
    let (code, stdout, _) = run(&state, &["--json", "hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code, 0);
    let id = serde_json::from_str::<Value>(&stdout)
        .expect("mint --json is JSON")["token_id"]
        .as_str()
        .expect("token_id")
        .to_string();
    let (code2, stdout2, stderr2) = run(&state, &["hub", "token", "revoke", &id]);
    assert_eq!(code2, 0, "revoke of an unused token must exit 0; stderr: {stderr2}");
    assert!(
        stdout2.contains(&format!("invalidated {id} (alice, unused)")),
        "an unused token's pre-state word is 'invalidated'; got: {stdout2}"
    );
    // After the flip the store records it as revoked.
    let (code3, stdout3, _) = run(&state, &["hub", "token", "ping", &id]);
    assert_eq!(code3, 3, "a revoked token is a fail-closed refusal on ping");
    assert!(
        stdout3.contains("revoked"),
        "post-inactivation the record is revoked; got: {stdout3}"
    );
}

/// A token the store does not know is a fail-closed refusal: exit 3 for both
/// `delete` and `revoke`.
#[test]
fn delete_and_revoke_unknown_id_exit_3() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["hub", "token", "delete", "tok_doesnotexist"]);
    assert_eq!(code, 3, "delete of an unknown id must exit 3; stderr: {stderr}");
    let (code2, stdout2, stderr2) = run(&state, &["hub", "token", "revoke", "tok_doesnotexist"]);
    assert_eq!(code2, 3, "revoke of an unknown id must exit 3; stderr: {stderr2}");
    let _ = (stdout, stdout2);
}

/// `hub token ping <id>` on a freshly minted (unused, not expired) token:
/// exit 0, prints `valid`.
#[test]
fn ping_valid_exit_0() {
    let state = StateDir::new();
    let (code, stdout, _) = run(&state, &["--json", "hub", "token", "mint", "--label", "alice"]);
    assert_eq!(code, 0);
    let id = serde_json::from_str::<Value>(&stdout)
        .expect("mint --json is JSON")["token_id"]
        .as_str()
        .expect("token_id")
        .to_string();
    let (code2, stdout2, stderr2) = run(&state, &["hub", "token", "ping", &id]);
    assert_eq!(code2, 0, "ping of a valid token must exit 0; stderr: {stderr2}");
    assert!(
        stdout2.trim_start().starts_with("valid"),
        "ping prints 'valid …'; got: {stdout2}"
    );
}

/// `hub token ping <id>` on an unknown id: exit 3, prints `not_found`.
#[test]
fn ping_unknown_id_exit_3() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["hub", "token", "ping", "tok_doesnotexist"]);
    assert_eq!(code, 3, "ping of an unknown id must exit 3; stderr: {stderr}");
    assert!(
        stdout.trim().starts_with("not_found") || stderr.contains("not_found"),
        "ping reports not_found; stdout: {stdout}, stderr: {stderr}"
    );
}
