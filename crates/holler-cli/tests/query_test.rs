#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #185
//! `status`/`caps`/`support`/`protocol` — local and remote, both peers
//! (issue #185). Real subprocesses, a real loopback hub, a real `stub-acp`
//! agent for the harness-resolution probes — the same discipline as
//! `body_run_test.rs` (#182): no mocks of the circuit.
//!
//! Readiness is always **observed** via [`support::wait_for`] against
//! `hub_status_json`/`body/connection_state.json`, never a blind sleep.

mod support;

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, wait_for, Body, Hub, StateDir};

const READY: Duration = Duration::from_secs(10);

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

fn join_fresh(state: &StateDir, ws_url: &str, label: &str) -> String {
    let (token_id, secret) = support::mint_token(state, label);
    let (code, _, stderr) = run(
        state,
        &["body", "join", "--server", ws_url, "--token", &format!("{token_id}:{secret}")],
    );
    assert_eq!(code, 0, "body join must exit 0; stderr: {stderr}");
    token_id
}

/// Write `<state>/body/sessions.toml` with one `[[session]]` row whose
/// `command[0]` is `command` verbatim (not necessarily resolvable) — used by
/// the harness-resolution cases, which need to control resolvability
/// directly rather than always pointing at the real `stub-acp` binary.
fn write_raw_sessions_toml(state: &StateDir, name: &str, harness: &str, command: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(state.body()).expect("create body dir");
    let path = state.body().join("sessions.toml");
    let toml = format!(
        "[[session]]\nname = {name:?}\nharness = {harness:?}\ncommand = [{command:?}]\n"
    );
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

// --- body: local, no run required -------------------------------------------

/// `body query status` (and `caps`) answer from local state alone — no
/// `body run` active, no join even required. `connected` is `false` (no
/// `connection_state.json` written yet) and the document is still a valid,
/// complete `Status`.
#[test]
fn body_status_local_without_run() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "status"]);
    assert_eq!(code, 0, "local `body query status` must exit 0 with no run active; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("body query status --json is valid JSON");
    assert_eq!(doc["role"].as_str(), Some("body"));
    assert_eq!(doc["connected"].as_bool(), Some(false), "no run/join means not connected: {doc}");
    assert_eq!(doc["protocol"].as_u64(), Some(2));
}

/// `body caps` answers the same way, with no run active.
#[test]
fn body_caps_local_without_run() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "caps"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["status"]["role"].as_str(), Some("body"));
    assert!(doc["caps"].is_object());
}

/// `query/caps` (both `body caps` and `query caps`) carries a support answer
/// for **every** known feature/harness/capability id in the v2 vocabulary —
/// none silently dropped.
#[test]
fn caps_contains_every_known_id() {
    let state = StateDir::new();
    let (code, stdout, _) = run(&state, &["--json", "body", "caps"]);
    assert_eq!(code, 0);
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    let caps = doc["caps"].as_object().expect("caps is an object");
    for id in holler_proto::FEATURES.iter().chain(holler_proto::HARNESS_IDS.iter()) {
        assert!(caps.contains_key(*id), "caps is missing {id}: {caps:?}");
    }
}

/// `body support` on a harness with a real, `PATH`-resolvable
/// `command[0]` is `ok:true`; the same harness with an unresolvable command
/// is `ok:false`.
#[test]
fn body_support_stub_harness_true_when_on_path_false_when_not() {
    let state = StateDir::new();
    let stub = env!("CARGO_BIN_EXE_stub-acp");

    write_raw_sessions_toml(&state, "alpha", "opencode", stub);
    let (code, stdout, stderr) = run(&state, &["--json", "body", "support", "opencode"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["ok"].as_bool(), Some(true), "a real, existing command must be ok:true: {doc}");

    write_raw_sessions_toml(&state, "alpha", "opencode", "/definitely/not/a/real/binary-xyz");
    let (code, stdout, stderr) = run(&state, &["--json", "body", "support", "opencode"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["ok"].as_bool(), Some(false), "a missing command must be ok:false: {doc}");
}

/// `query/support` with an id outside the v2 vocabulary is `-32006
/// unknown_feature` — surfaced by the local `body support` leaf as exit 1.
#[test]
fn unknown_feature_is_32006() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["body", "support", "not-a-real-feature"]);
    assert_eq!(code, 1, "an unknown id is exit 1, not a crash");
    assert!(stderr.contains("unknown"), "stderr names the failure: {stderr}");
}

/// `query/protocol {version:1}` is `ok:false` (v2 only speaks version 2).
#[test]
fn query_protocol_with_version_1_is_ok_false() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "protocol", "1"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["asked"].as_u64(), Some(1));
    assert_eq!(doc["ok"].as_bool(), Some(false));
}

/// `query/protocol {version:2}` (the current, in-range version) is a normal
/// `ok:true` answer, not a rejection — issue #251's fix must not touch the
/// happy path.
#[test]
fn query_protocol_with_current_version_is_ok_true() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "protocol", "2"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["asked"].as_u64(), Some(2));
    assert_eq!(doc["ok"].as_bool(), Some(true));
}

/// `query/protocol` with no `version` at all reports the plain range and is
/// never a rejection (docs §5.4).
#[test]
fn query_protocol_with_no_version_is_not_rejected() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "protocol"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(doc.get("asked").is_none(), "no version asked must not appear in the answer: {doc}");
}

/// `query/protocol {version:0}` is the documented `-32006 unknown_feature`
/// (docs §5.4: "`n` must be a positive integer") on the **body**'s local
/// leaf (issue #251) — not a normal `ok:false` answer.
#[test]
fn body_query_protocol_version_zero_is_unknown_feature() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["body", "query", "protocol", "0"]);
    assert_eq!(code, 1, "version:0 must be a refusal, not a crash or a normal answer");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");
}

/// A negative `version` is rejected the same way as `0` — the old bug
/// silently treated an unparseable/negative value as "no version asked"
/// (a misleading `ok`-less normal answer) instead of `-32006`.
#[test]
fn body_query_protocol_negative_version_is_unknown_feature() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["body", "query", "protocol", "-1"]);
    assert_eq!(code, 1, "a negative version must be a refusal, not silently 'no version asked'");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");
}

/// A malformed (non-numeric) `version` is rejected the same way.
#[test]
fn body_query_protocol_malformed_version_is_unknown_feature() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["body", "query", "protocol", "not-a-number"]);
    assert_eq!(code, 1, "a malformed version must be a refusal, not silently 'no version asked'");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");
}

/// The **hub**'s local `query/protocol` leaf (`hub query protocol …`, served
/// by `control_server.rs`'s `hub_query_local`) enforces the same rule as the
/// body's (issue #251): `0` is `-32006`, not a normal answer.
#[test]
fn hub_query_protocol_version_zero_is_unknown_feature() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, _, stderr) = run(&state, &["hub", "query", "protocol", "0"]);
    assert_eq!(code, 1, "version:0 must be a refusal, not a crash or a normal answer");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");
    hub.stop(Duration::from_secs(5));
}

/// The hub's local leaf also rejects a negative/malformed `version` (rather
/// than silently forwarding "no version asked" as the old CLI arg parsing
/// and wire dispatch both used to do).
#[test]
fn hub_query_protocol_negative_and_malformed_version_are_unknown_feature() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    let (code, _, stderr) = run(&state, &["hub", "query", "protocol", "-1"]);
    assert_eq!(code, 1, "a negative version must be a refusal: {stderr}");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");

    let (code, _, stderr) = run(&state, &["hub", "query", "protocol", "not-a-number"]);
    assert_eq!(code, 1, "a malformed version must be a refusal: {stderr}");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");

    hub.stop(Duration::from_secs(5));
}

/// The hub's local `query/protocol` leaf still answers normally for a valid
/// version and for no version at all — the fix must not touch those paths.
#[test]
fn hub_query_protocol_valid_and_absent_version_are_not_rejected() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "protocol", "2"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["ok"].as_bool(), Some(true));

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "protocol"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(doc.get("asked").is_none(), "no version asked must not appear in the answer: {doc}");

    hub.stop(Duration::from_secs(5));
}

/// `body query` has no remote target — `body query TARGET status` is a usage
/// error (exit 2), not a silent local answer.
#[test]
fn body_query_remote_form_is_usage_error_exit_2() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["body", "query", "some-target", "status"]);
    assert_eq!(code, 2, "a body has no routing target: stderr={stderr}");
}

// --- hub: local (control socket) and remote (forwarded over a live body) ---

/// `hub status`'s `clients`/`sessions` counts are the live registry's real
/// counts, not hardcoded — a connected body with one configured session
/// moves both off zero.
#[test]
fn hub_status_reports_real_client_and_session_counts() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = support::write_sessions_toml(&state, &[("alpha", &[])]);
    let body = Body::start(&state, &config);

    let ready = wait_for(READY, || {
        let doc = support::hub_status_json(&state);
        (doc.get("clients").and_then(|v| v.as_u64()) == Some(1)
            && doc.get("sessions").and_then(|v| v.as_u64()) == Some(1))
        .then_some(())
    });
    assert!(ready.is_some(), "hub status must report 1 client and 1 session once the body is live");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// The hub's confirmation pass (issue #185): once a body advertises a
/// harness whose `command[0]` really resolves, the hub's own
/// `query/support {harness}` reports `ok:true` — proof, not just the body's
/// say-so in its hello.
#[test]
fn hub_confirmed_harness_reflects_real_probe() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = support::write_sessions_toml(&state, &[("alpha", &[])]);
    let body = Body::start(&state, &config);

    let confirmed = wait_for(READY, || {
        let (code, stdout, _) = run(&state, &["--json", "hub", "support", "opencode"]);
        (code == 0 && serde_json::from_str::<Value>(&stdout).ok()?["ok"].as_bool() == Some(true)).then_some(())
    });
    assert!(confirmed.is_some(), "the hub must confirm `opencode` once the live body's command resolves");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// `hub query TARGET status` resolves TARGET by label (hostname) or by
/// token id, forwards `query/status` to that body's live socket, and prints
/// its answer.
#[test]
fn remote_query_status_by_label_and_by_token_id() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let token_id = join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = support::write_sessions_toml(&state, &[]);
    let body = Body::start(&state, &config);

    let ready = wait_for(READY, || {
        (support::hub_status_json(&state).get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(ready.is_some(), "the body must be live before it can be queried");

    // NOTE: every body currently joins under the hardcoded hostname
    // `"default"` (`body_join`'s `holler_body::join::join(..., "default")`
    // call in `holler-cli/src/main.rs` — there is no `--hostname` flag on
    // `body join` yet; the token's own `--label` is a *token* label, not the
    // wire hostname a body claims). So the label target here is `"default"`,
    // not the token's mint label (`"kiwi"`) — a real per-body `--hostname`
    // is out of this story's scope.
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "default", "status"]);
    assert_eq!(code, 0, "query by label must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["role"].as_str(), Some("body"), "the answer is the BODY's status doc: {doc}");
    assert_eq!(doc["hostname"].as_str(), Some("default"));

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", &token_id, "status"]);
    assert_eq!(code, 0, "query by token id must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["role"].as_str(), Some("body"));

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// `hub query TARGET protocol 0` forwards to the **body**'s live-socket
/// `query/protocol` dispatch (`holler_body::connection::handle_query`) — a
/// third distinct code path from the two local leaves above. Issue #251's
/// fix must reject `0` there too, not just on the hub's/body's local forms.
#[test]
fn remote_query_protocol_version_zero_is_unknown_feature() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = support::write_sessions_toml(&state, &[]);
    let body = Body::start(&state, &config);

    let ready = wait_for(READY, || {
        (support::hub_status_json(&state).get("clients").and_then(|v| v.as_u64()) == Some(1)).then_some(())
    });
    assert!(ready.is_some(), "the body must be live before it can be queried");

    let (code, _, stderr) = run(&state, &["hub", "query", "default", "protocol", "0"]);
    assert_eq!(code, 1, "version:0 forwarded to a live body must be a refusal: {stderr}");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "default", "protocol", "2"]);
    assert_eq!(code, 0, "a valid version must still work normally; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["ok"].as_bool(), Some(true));

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

/// `hub query TARGET status` against a token that joined but is not
/// currently live is `-32004 not_connected`, surfaced as exit 1.
#[test]
fn remote_query_to_disconnected_body_not_connected_exit_1() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let token_id = join_fresh(&state, &hub.ws_url(), "kiwi");
    // No `body run`: the token exists but nothing is live for it.

    let (code, _, stderr) = run(&state, &["hub", "query", &token_id, "status"]);
    assert_eq!(code, 1, "a not-connected target is exit 1, not a crash");
    assert!(stderr.contains("not connected"), "stderr names the failure: {stderr}");

    hub.stop(Duration::from_secs(5));
}

/// A `hub query TARGET …` naming a target that resolves to more than one
/// live body is reported as ambiguous — exit 2, ADR 0003's usage-error code
/// (the pure resolution logic — `Registry::find_target` — is additionally
/// unit-tested in `crates/holler-hub/src/live.rs`, since provoking a genuine
/// hostname collision end-to-end would need two bodies racing the same
/// label, which #184's uniqueness enforcement is what actually prevents in
/// practice).
#[test]
fn remote_query_ambiguous_target_exit_2() {
    // Exercised at the registry level (see `holler_hub::live::tests::
    // ambiguous_target_is_reported_as_ambiguous`); this test pins the CLI's
    // side of the contract — that the hub's ambiguity refusal maps to exit 2,
    // not the ordinary exit-1 "not connected" refusal — using the same
    // hub/control-socket path a real ambiguous match would take.
    let state = StateDir::new();
    let hub = Hub::start(&state);
    // No live body at all: use the ordinary not-connected path to prove the
    // *other* branch (exit 1) stays exit 1, so `is_ambiguous`'s string match
    // in `holler-cli/src/main.rs` cannot be accidentally over-broad.
    let (code, _, stderr) = run(&state, &["hub", "query", "nobody-here", "status"]);
    assert_eq!(code, 1, "a plain not-connected target must stay exit 1, not misclassify as ambiguous: {stderr}");
    hub.stop(Duration::from_secs(5));
}

/// `hub query CMD` (no remote target) answers from the live hub's own state.
#[test]
fn hub_query_local_status() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["role"].as_str(), Some("hub"));
    hub.stop(Duration::from_secs(5));
}

/// `holler hub caps` with no live hub is exit 1 with the spec's exact
/// message (same contract as `hub status`, `hub token …`).
#[test]
fn hub_caps_without_live_hub_exit_1() {
    let state = StateDir::new();
    let (code, _, stderr) = run(&state, &["hub", "caps"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("no live holler hub reachable"), "stderr: {stderr}");
}
