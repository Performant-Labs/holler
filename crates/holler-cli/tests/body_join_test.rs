#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #176
//! `holler body join` / `body detach` / `body status` e2e (story #176).
//!
//! Each test drives the **real** `holler` binary against an isolated
//! `HOLLER_STATE_DIR` (and, where the spec demands it, a live loopback hub
//! started via the shared harness) and asserts the ADR 0003 exit code plus the
//! stdout/stderr shape and the on-disk artefact each verb owns:
//! - `body join` redeems a minted one-time join secret over the wire and, on success, persists the body's identity (`<state>/body/credential.json`, mode 0600) and prints a success line. The join **secret is never written to disk** (the credential is). A refused redeem is exit 1 with the hub's message; a plaintext `ws://` to a non-loopback host is a fail-closed exit 3 before any connection is made.
//! - `body detach` forgets the body's identity (deletes `credential.json`); it is idempotent and always exits 0.
//! - `body status` reports this process's own identity; with no live hub it is exit 0 with `joined: false`.
//!
//! Readiness is always **observed** (the hub's `listening` event, file
//! existence) — never a blind sleep (ADR 0002).

mod support;

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, Hub, StateDir};

/// The body's persisted identity file (`<state>/body/credential.json`).
fn credential_path(state: &StateDir) -> PathBuf {
    state.body().join("credential.json")
}

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

/// `body join` against a live hub, with a valid minted token: exit 0, the
/// credential is persisted at mode 0600 (and names the token), and `body
/// status` now reports this process joined with the credential's client id.
#[test]
fn join_persists_credential_0600_and_status_reports_joined() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");

    let (code, stdout, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}")],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");

    // The one-time join secret is shown on stdout at mint, and join reports
    // success — but neither the secret nor its hmac is what is persisted.
    let cred_path = credential_path(&state);
    assert!(cred_path.exists(), "body join must persist body/credential.json");
    let mode = file_mode(&cred_path);
    assert_eq!(mode & 0o777, 0o600, "credential.json is mode 0600; got {mode:o}");

    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&cred_path).unwrap()).expect("credential.json is JSON");
    assert_eq!(
        doc["token_id"].as_str(),
        Some(token_id.as_str()),
        "credential.json names the token"
    );
    let client_id = doc["client_id"].as_str().expect("credential.json carries a client_id");
    assert!(
        client_id.starts_with("cli_"),
        "client_id is a cli_ id: {client_id}"
    );
    assert!(
        !doc.to_string().contains(&secret),
        "the join secret must not be persisted: {doc}"
    );

    // …and the process's own view of itself now reports joined, with the same
    // identity (the status document is local — no live hub is consulted here).
    let (sc, sstd, ssterr) = run(&state, &["--json", "body", "status"]);
    assert_eq!(sc, 0, "body status must exit 0; stderr: {ssterr}");
    let sdoc: Value = serde_json::from_str(&sstd).expect("body status --json is valid JSON");
    assert_eq!(sdoc["role"].as_str(), Some("body"), "status document is a body's");
    assert_eq!(sdoc["joined"].as_bool(), Some(true), "status reports joined: {sdoc}");
    assert_eq!(
        sdoc["client_id"].as_str(),
        Some(client_id),
        "status client_id matches the persisted one: {sdoc}"
    );
    assert_eq!(
        sdoc["token_id"].as_str(),
        Some(token_id.as_str()),
        "status token_id matches the minted one: {sdoc}"
    );

    // The success line does not echo the secret back.
    assert!(!stdout.contains(&secret), "the join success line must not print the secret; got: {stdout}");

    hub.stop(Duration::from_secs(5));
}

/// A redeem the hub refuses (an unknown / bad secret) is exit 1 with the hub's
/// one-line message, and persists **no** credential.
#[test]
fn join_with_bad_secret_exit_1_message_from_hub() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    // Mint a real token so the store has a pepper, but present a secret that
    // matches nothing (a well-formed secret with no corresponding token).
    support::mint_token(&state, "kiwi");
    let bogus = "hlr_join_00000000000000000000000000000000000000000000000000000000000000";

    let (code, _, stderr) = run(&state, &["body", "join", "--server", &hub.ws_url(), "--token", &format!("tok_nope:{bogus}")]);
    assert_eq!(code, 1, "a refused redeem must exit 1; stderr: {stderr}");
    assert!(!stderr.is_empty(), "the hub's refusal message reaches stderr");
    assert!(!stderr.contains("not implemented"), "this is a real refusal, not the skeleton: {stderr}");
    assert!(
        !credential_path(&state).exists(),
        "a failed join must not persist a credential"
    );

    hub.stop(Duration::from_secs(5));
}

/// A plaintext `ws://` (not loopback) is a fail-closed policy refusal — exit 3,
/// **before** any connection is attempted (so no live hub is needed, and the
/// credential store is untouched).
#[test]
fn join_plain_ws_to_non_loopback_refused_exit_3() {
    let state = StateDir::new();
    // No hub is started: the refusal must fire at URL-parse time, not on connect.
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", "ws://10.0.0.5:41807", "--token", "tok_7f3a:hlr_join_deadbeef"],
    );
    assert_eq!(code, 3, "a plaintext ws:// to a non-loopback host must exit 3; stderr: {stderr}");
    assert!(!stderr.is_empty(), "the refusal is explained on stderr: {stderr}");
    assert!(
        !credential_path(&state).exists(),
        "a refused join must not persist a credential"
    );
}

/// The one-time join secret is shown to the operator at mint and travels over
/// the wire at join — but it must never be written anywhere under the state dir.
#[test]
fn secret_never_written_to_disk() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let ws_url = hub.ws_url();
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &ws_url, "--token", &format!("{token_id}:{secret}")],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");

    // Scan every file under the state dir for the raw secret. (The at-rest token
    // store keeps only the HMAC, and credential.json keeps only the credential —
    // so the raw secret appears in no file.)
    for path in walk_files(state.path()) {
        let bytes = std::fs::read(&path).unwrap_or_default();
        let hay = String::from_utf8_lossy(&bytes).into_owned();
        assert!(
            !hay.contains(&secret),
            "the join secret is written to {}: …",
            path.display()
        );
    }

    hub.stop(Duration::from_secs(5));
}

/// `body detach` with no live body run simply forgets the identity: it removes
/// `credential.json` and exits 0. Running it again (already detached) is also
/// exit 0 (idempotent).
#[test]
fn detach_without_run_removes_credential() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}")],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");
    assert!(credential_path(&state).exists(), "joined body has a credential");

    // First detach: removes the credential, exits 0.
    let (code, _, stderr) = run(&state, &["body", "detach"]);
    assert_eq!(code, 0, "body detach must exit 0; stderr: {stderr}");
    assert!(
        !credential_path(&state).exists(),
        "detach removes credential.json"
    );

    // Second detach (already detached): still exit 0, no error.
    let (code2, _, stderr2) = run(&state, &["body", "detach"]);
    assert_eq!(code2, 0, "a second detach must still exit 0; stderr: {stderr2}");

    // …and status now reports the body is no longer joined.
    let (sc, sstd, ssterr) = run(&state, &["--json", "body", "status"]);
    assert_eq!(sc, 0, "body status must exit 0; stderr: {ssterr}");
    let sdoc: Value = serde_json::from_str(&sstd).expect("body status --json is valid JSON");
    assert_eq!(sdoc["joined"].as_bool(), Some(false), "after detach, status reports unjoined: {sdoc}");

    hub.stop(Duration::from_secs(5));
}

/// `body status` with no join and no live hub is exit 0 with `joined: false` —
/// it describes *this* process's identity, not the hub's, so it never fails
/// closed because the hub is down.
#[test]
fn status_unjoined_is_exit_0_joined_false() {
    let state = StateDir::new();
    // No hub, no join: the status document is the local, unjoined one.
    let (code, stdout, stderr) = run(&state, &["--json", "body", "status"]);
    assert_eq!(code, 0, "body status must exit 0 even unjoined; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("body status --json is valid JSON");
    assert_eq!(doc["role"].as_str(), Some("body"), "the status document is a body's: {doc}");
    assert_eq!(doc["joined"].as_bool(), Some(false), "an unjoined body reports joined:false: {doc}");
}

// --- small local helpers ------------------------------------------------------

/// The permission bits of a file (Unix `mode & 0o777`); 0 on a non-Unix box.
#[cfg(unix)]
fn file_mode(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path)
        .map(|m| m.mode())
        .unwrap_or(0)
}
#[cfg(not(unix))]
fn file_mode(_path: &std::path::Path) -> u32 {
    0
}

/// Every regular file under `root` (recursive).
fn walk_files(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}

// --- unit tests ----------------------------------------------------------------

#[cfg(test)]
mod unit {
    use holler_body::server_address::parse;

    // The protocol's default body-join port (ADR 0002): a bare `ws://host`
    // dials 41807. (The e2e harness always supplies an explicit port via the
    // hub's bound `127.0.0.1:<port>`, so the default lives here, where it is
    // directly unit-testable.)
    #[test]
    fn url_port_default_is_41807() {
        let addr = parse("ws://loopback.example").expect("a bare ws:// is valid");
        assert_eq!(addr.port, 41807, "a bare ws:// defaults to 41807");
    }

    // An explicit port is honoured.
    #[test]
    fn url_port_explicit_wins() {
        let addr = parse("ws://127.0.0.1:4321").expect("explicit port is valid");
        assert_eq!(addr.port, 4321);
    }

    // `wss://` is accepted (the TLS transport) and defaults to 41807 too.
    #[test]
    fn url_wss_default_port() {
        let addr = parse("wss://loopback.example").expect("a bare wss:// is valid");
        assert_eq!(addr.port, 41807);
    }
}
