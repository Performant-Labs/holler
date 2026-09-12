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

/// `body join` against a live hub, with a valid minted token: exit 0, and the
/// success line never echoes the join secret back.
#[test]
fn join_exits_0_and_does_not_echo_the_secret() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let hub_key = support::hub_pubkey(&state);

    let (code, stdout, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");
    assert!(!stdout.contains(&secret), "the join success line must not print the secret; got: {stdout}");

    hub.stop(Duration::from_secs(5));
}

/// `body join` persists `body/credential.json` at mode 0600, naming the
/// token — carrying issue #322's pinned hub key and issue #323's own signing
/// key, never the join secret nor a `credential` field (there is none any
/// more).
#[test]
fn join_persists_credential_0600_and_pins_hub_key() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let hub_key = support::hub_pubkey(&state);

    let (code, _stdout, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");

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
    // Issue #322: the pinned hub key is persisted verbatim.
    assert_eq!(
        doc["hub_pubkey"].as_str(),
        Some(hub_key.as_str()),
        "credential.json pins the hub's public key: {doc}"
    );
    // Issue #323: this body's own signing private key is persisted (never
    // sent to the hub), never a `credential` field (there is none any more).
    assert!(doc.get("signing_key").and_then(|v| v.as_str()).is_some_and(|s| s.len() == 64), "credential.json carries this body's own signing_key: {doc}");
    assert!(doc.get("credential").is_none(), "there is no `credential` field any more (issue #323): {doc}");

    hub.stop(Duration::from_secs(5));
}

/// After a successful `body join`, `body status` reports this process
/// joined with the same identity persisted in `credential.json` — the status
/// document is local, so no live hub is consulted here.
#[test]
fn status_reports_joined_with_same_identity_after_join() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let hub_key = support::hub_pubkey(&state);

    let (code, _stdout, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(credential_path(&state)).unwrap())
        .expect("credential.json is JSON");
    let client_id = doc["client_id"].as_str().expect("credential.json carries a client_id");

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
    let hub_key = support::hub_pubkey(&state);
    let bogus = "hlr_join_00000000000000000000000000000000000000000000000000000000000000";

    let (code, _, stderr) = run(&state, &["body", "join", "--server", &hub.ws_url(), "--token", &format!("tok_nope:{bogus}"), "--hub-key", &hub_key]);
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
    // No hub is started: the refusal must fire at URL-parse time, not on
    // connect — `--hub-key` is a placeholder here, never even inspected.
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", "ws://10.0.0.5:41807", "--token", "tok_7f3a:hlr_join_deadbeef", "--hub-key", &"a".repeat(64)],
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
    let hub_key = support::hub_pubkey(&state);
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &ws_url, "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");

    // Scan every file under the state dir for the raw secret. (The at-rest
    // token store keeps only the HMAC, and credential.json keeps only this
    // body's own signing key and the pinned hub key — so the raw join secret
    // appears in no file.)
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
    let hub_key = support::hub_pubkey(&state);
    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
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

// --- issue #322: hub identity / join-line pinning -----------------------------

/// Issue #322's `join_line_carries_hub_pubkey`: `hub token mint`'s human
/// output and its `--json` document both carry the hub's real X25519 public
/// key, and the ready-to-paste join line embeds it as `--hub-key`.
#[test]
fn join_line_carries_hub_pubkey() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let real_key = support::hub_pubkey(&state);

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "token", "mint", "--label", "kiwi"]);
    assert_eq!(code, 0, "hub token mint must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("mint --json is valid JSON");
    assert_eq!(doc["hub_pubkey"].as_str(), Some(real_key.as_str()), "mint --json carries the real hub_pubkey: {doc}");
    let join_command = doc["join_command"].as_str().expect("mint --json carries join_command");
    assert!(
        join_command.contains(&format!("--hub-key {real_key}")),
        "the join line embeds --hub-key <the real key>: {join_command}"
    );

    // Human output: the same key appears both as its own line and inside the
    // ready-to-paste join line.
    let (code2, stdout2, stderr2) = run(&state, &["hub", "token", "mint", "--label", "web"]);
    assert_eq!(code2, 0, "hub token mint (human) must exit 0; stderr: {stderr2}");
    assert!(stdout2.contains(&format!("hub_pubkey:  {real_key}")), "human output names hub_pubkey: {stdout2}");
    assert!(stdout2.contains(&format!("--hub-key {real_key}")), "human output's join line embeds --hub-key: {stdout2}");

    hub.stop(Duration::from_secs(5));
}

/// Issue #322's `body_join_pins_hub_key_and_persists_it`: `body join
/// --hub-key HEX` persists exactly that key in `credential.json`, verbatim —
/// this is the pin a later `body run` compares every hub hello against
/// (`holler_body::connection::hello_exchange`), and it is taken purely from
/// the CLI argument, never negotiated or confirmed over the wire during join
/// itself (the whole point of an out-of-band pin).
#[test]
fn body_join_pins_hub_key_and_persists_it() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    let real_key = support::hub_pubkey(&state);

    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", &real_key],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");

    let doc: Value = serde_json::from_str(&std::fs::read_to_string(credential_path(&state)).unwrap())
        .expect("credential.json is JSON");
    assert_eq!(doc["hub_pubkey"].as_str(), Some(real_key.as_str()), "the pinned hub key is persisted verbatim: {doc}");

    hub.stop(Duration::from_secs(5));
}

/// A malformed `--hub-key` (not 64 lowercase hex chars) is a fail-closed
/// policy refusal — exit 3, before any connection is attempted, the same
/// class of refusal as a plaintext non-loopback `ws://`.
#[test]
fn body_join_malformed_hub_key_refused_exit_3() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");

    let (code, _, stderr) = run(
        &state,
        &["body", "join", "--server", &hub.ws_url(), "--token", &format!("{token_id}:{secret}"), "--hub-key", "not-hex"],
    );
    assert_eq!(code, 3, "a malformed --hub-key must exit 3; stderr: {stderr}");
    assert!(!credential_path(&state).exists(), "a refused join must not persist an identity");

    hub.stop(Duration::from_secs(5));
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
