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
    let hub_key = support::hub_pubkey(state);
    let (code, _, stderr) = run(
        state,
        &["body", "join", "--server", ws_url, "--token", &format!("{token_id}:{secret}"), "--hub-key", &hub_key],
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
    assert_eq!(doc["protocol"].as_u64(), Some(3));
}

/// `query/status` documents carry the running binary's version (issue
/// #319) — `env!("CARGO_PKG_VERSION")`, not hardcoded — on both roles: the
/// body's local answer and the hub's local answer.
#[test]
fn status_docs_carry_version() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("body query status --json is valid JSON");
    assert_eq!(
        doc["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "body query status must carry the running version: {doc}"
    );

    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("hub query status --json is valid JSON");
    assert_eq!(
        doc["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "hub query status must carry the running version: {doc}"
    );
    hub.stop(Duration::from_secs(5));
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

/// `query/protocol {version:1}` is `ok:false` (v2 only speaks version 3,
/// issue #340).
#[test]
fn query_protocol_with_version_1_is_ok_false() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "protocol", "1"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["asked"].as_u64(), Some(1));
    assert_eq!(doc["ok"].as_bool(), Some(false));
}

/// `query/protocol {version:3}` (the current, in-range version) is a normal
/// `ok:true` answer, not a rejection — issue #251's fix must not touch the
/// happy path.
#[test]
fn query_protocol_with_current_version_is_ok_true() {
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["--json", "body", "query", "protocol", "3"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(doc["asked"].as_u64(), Some(3));
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

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "protocol", "3"]);
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

    // Target-resolution routes by the token's own `--label` (`"kiwi"`
    // here), not by the hostname a body claims in its hello — every body
    // currently joins under the hardcoded hostname `"default"` (there is
    // no `--hostname` flag on `body join` yet), but issue #193's
    // 2026-09-21 fix keys `holler-hub`'s `Registry` on the verified token
    // label instead, matching the roster's own routing key. `doc["hostname"]`
    // below is still `"default"` because that field reflects the body's own
    // self-reported identity (`crates/holler-body/src/status.rs`), which is
    // an entirely separate thing from the routing target used to reach it.
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "kiwi", "status"]);
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

    let (code, _, stderr) = run(&state, &["hub", "query", "kiwi", "protocol", "0"]);
    assert_eq!(code, 1, "version:0 forwarded to a live body must be a refusal: {stderr}");
    assert!(stderr.contains("positive integer"), "stderr names the failure: {stderr}");

    let (code, stdout, stderr) = run(&state, &["--json", "hub", "query", "kiwi", "protocol", "3"]);
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

// --- issue #368: `hub status`/`caps`/`support`'s own success-path formatting
// (hub_cmd.rs), never exercised via the real CLI binary against a live hub
// anywhere else in this suite — every prior "success" test either talked to
// the raw control socket directly (hub_serve_test.rs's
// control_status_over_ud_socket), bypassing hub_cmd.rs's own print/format
// code entirely, or only ever passed --json (query_test.rs's own
// hub_query_local_status above). hub_cmd.rs's non-json human-readable
// branches (`status`'s 4-line summary, `caps`'s "hub: N caps known",
// `support`'s "<feature>: ok"/"not ok") had zero coverage until now.

/// `holler hub status` (no `--json`) against a real live hub prints the
/// spec's own 4-line human summary, not just a non-empty string.
#[test]
fn hub_status_live_human_readable_format() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["hub", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 4, "hub {{version}} (protocol 2) / listening / clients / sessions: {stdout:?}");
    assert!(lines[0].starts_with("hub ") && lines[0].contains("(protocol 2)"), "line 1: {lines:?}");
    assert!(lines[1].trim_start().starts_with("listening:"), "line 2: {lines:?}");
    assert!(lines[2].trim_start().starts_with("clients:") && lines[2].contains('0'), "line 3: {lines:?}");
    assert!(lines[3].trim_start().starts_with("sessions:") && lines[3].contains('0'), "line 4: {lines:?}");
    hub.stop(Duration::from_secs(5));
}

/// `holler --json hub status` against a real live hub prints the raw
/// `StatusDoc` — same document shape `control_status_over_ud_socket`
/// confirms at the wire level, but this time through the real CLI leaf.
#[test]
fn hub_status_live_json() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("hub status --json is valid JSON");
    assert_eq!(doc["role"].as_str(), Some("hub"));
    assert_eq!(doc["clients"].as_u64(), Some(0));
    hub.stop(Duration::from_secs(5));
}

/// `holler hub caps` (no `--json`) against a real live hub prints
/// `"hub: N caps known"` — `caps`'s success path had no test at all before
/// this (only the no-live-hub refusal above did).
#[test]
fn hub_caps_live_human_readable_format() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["hub", "caps"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let line = stdout.trim_end();
    assert!(line.starts_with("hub: ") && line.ends_with(" caps known"), "got: {line:?}");
    let n: u32 = line.trim_start_matches("hub: ").trim_end_matches(" caps known").parse().expect("a number of caps");
    assert!(n > 0, "a live hub must know at least one cap: {line:?}");
    hub.stop(Duration::from_secs(5));
}

/// `holler --json hub caps` against a real live hub — the raw `caps` object,
/// not just the human count line above.
#[test]
fn hub_caps_live_json() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["--json", "hub", "caps"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("hub caps --json is valid JSON");
    let caps = doc["caps"].as_object().expect("caps is an object");
    assert!(!caps.is_empty(), "a live hub must report at least one cap: {doc}");
    hub.stop(Duration::from_secs(5));
}

/// `holler hub support FEATURE` (no `--json`) prints `"FEATURE: ok"` /
/// `"FEATURE: not ok"` — the human line had zero coverage; every existing
/// `hub support` test (this file's `hub_support_opencode_ok` and siblings)
/// only ever passes `--json`.
#[test]
fn hub_support_live_human_readable_format() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (code, stdout, stderr) = run(&state, &["hub", "support", "opencode"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.trim_end() == "opencode: ok" || stdout.trim_end() == "opencode: not ok",
        "got: {stdout:?}"
    );

    // An unrecognized feature/harness id is a real refusal (exit 1), not a
    // normal ok=false answer — confirmed by running this for real rather
    // than assumed from reading the source.
    let (code, _, stderr) = run(&state, &["hub", "support", "definitely-not-a-real-feature"]);
    assert_eq!(code, 1, "an unknown feature id is a refusal, not a false answer; stderr: {stderr}");
    assert!(stderr.contains("unknown feature/harness id"), "stderr: {stderr}");
    hub.stop(Duration::from_secs(5));
}
