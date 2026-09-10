#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #196
//! `holler body attach sessions` / `holler body attach init` e2e (issue
//! #196) — a genuine float over the HTTP attach driver (#194): list an
//! already-running OpenCode endpoint's sessions and write a ready `mode =
//! "attach"` config row. Pure HTTP + file write; never touches
//! `SessionManager`/`session_manager.rs`/the hub at all, so these tests drive
//! the real `holler` binary against a fake OpenCode HTTP server
//! (`fake_server.rs`, hand-rolled — see its own doc comment for why it is
//! not a straight reuse of `holler-body`'s own fake server) rather than a
//! real hub/body pair.

mod support;
#[path = "attach_cli_test/fake_server.rs"]
mod fake_server;

use std::process::Stdio;

use fake_server::{unreachable_endpoint, FakeServer};
use serde_json::{json, Value};
use support::{holler_cmd, StateDir};

/// One OpenCode session list entry, real shape (`id`, `title`,
/// `time.created`/`time.updated`, epoch milliseconds).
fn session_json(id: &str, title: &str, updated_ms: i64) -> Value {
    json!({
        "id": id,
        "title": title,
        "time": {"created": updated_ms - 1000, "updated": updated_ms},
    })
}

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

/// `body attach sessions`: newest first (by `time.updated`), a
/// `SESSION_ID TITLE UPDATED` table.
#[test]
fn sessions_lists_newest_first() {
    let server = FakeServer::start();
    server.set_bare(
        200,
        json!([
            session_json("ses_old", "first chat", 1_700_000_000_000),
            session_json("ses_new", "latest chat", 1_700_000_050_000),
            session_json("ses_mid", "middle chat", 1_700_000_020_000),
        ])
        .to_string(),
    );

    let state = StateDir::new();
    let (code, stdout, stderr) = run(
        &state,
        &["body", "attach", "sessions", "--endpoint", &server.endpoint()],
    );
    assert_eq!(code, 0, "sessions must exit 0; stderr: {stderr}");
    assert!(
        stdout.contains("SESSION_ID") && stdout.contains("TITLE") && stdout.contains("UPDATED"),
        "table header names all three columns; got: {stdout}"
    );

    let pos_new = stdout.find("ses_new").expect("ses_new listed");
    let pos_mid = stdout.find("ses_mid").expect("ses_mid listed");
    let pos_old = stdout.find("ses_old").expect("ses_old listed");
    assert!(
        pos_new < pos_mid && pos_mid < pos_old,
        "rows must be newest-updated first; got:\n{stdout}"
    );
    assert!(stdout.contains("latest chat"), "title column is populated: {stdout}");

    // The bare form answered, so no `/api/session` fallback was needed.
    assert_eq!(server.requests(), vec!["/session".to_string()]);
}

/// `--json body attach sessions`: a `{"sessions": [...]}` document, each
/// entry carrying `session_id`/`title`/`updated`, newest first.
#[test]
fn sessions_json_shape() {
    let server = FakeServer::start();
    server.set_bare(
        200,
        json!([
            session_json("ses_a", "alpha chat", 1_700_000_000_000),
            session_json("ses_b", "beta chat", 1_700_000_099_000),
        ])
        .to_string(),
    );

    let state = StateDir::new();
    let (code, stdout, stderr) = run(
        &state,
        &["--json", "body", "attach", "sessions", "--endpoint", &server.endpoint()],
    );
    assert_eq!(code, 0, "sessions --json must exit 0; stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("sessions --json stdout is valid JSON");
    let sessions = doc["sessions"].as_array().expect("sessions is an array");
    assert_eq!(sessions.len(), 2, "both sessions are present: {doc:?}");
    assert_eq!(sessions[0]["session_id"], "ses_b", "newest first: {doc:?}");
    assert_eq!(sessions[0]["title"], "beta chat");
    assert!(sessions[0]["updated"].is_number(), "updated is a number: {doc:?}");
    assert_eq!(sessions[1]["session_id"], "ses_a");
}

/// A non-2xx from both `/session` and `/api/session` (or an unreachable
/// endpoint) is exit 1, naming the URL(s) tried on stderr.
#[test]
fn sessions_unreachable_exit_1() {
    let endpoint = unreachable_endpoint();
    let state = StateDir::new();
    let (code, stdout, stderr) = run(&state, &["body", "attach", "sessions", "--endpoint", &endpoint]);
    assert_eq!(code, 1, "an unreachable endpoint must exit 1; stdout: {stdout}, stderr: {stderr}");
    assert!(
        stderr.contains(&endpoint),
        "stderr names the URL that was tried; got: {stderr}"
    );
}

/// `body attach init` with every default: endpoint 127.0.0.1:4096 (here
/// pointed at the fake server via `--endpoint` only — the other defaults,
/// `--session`=newest / `--name`=alpha / `--out`=./attach.toml, are left
/// unset), run with the fake server's endpoint from a fresh cwd.
#[test]
fn init_writes_valid_config_default_newest_alpha() {
    let server = FakeServer::start();
    server.set_bare(
        200,
        json!([
            session_json("ses_old", "first chat", 1_700_000_000_000),
            session_json("ses_new", "latest chat", 1_700_000_050_000),
        ])
        .to_string(),
    );

    let state = StateDir::new();
    let out = holler_cmd(&state)
        .current_dir(state.path())
        .args(["body", "attach", "init", "--endpoint", &server.endpoint()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "init must exit 0; stderr: {stderr}");

    let written = state.path().join("attach.toml");
    assert!(written.exists(), "./attach.toml was written; stdout: {stdout}");
    assert!(
        stdout.contains("attach.toml"),
        "stdout names the file written: {stdout}"
    );
    assert!(
        stdout.contains("holler body run --config"),
        "stdout prints the next command: {stdout}"
    );

    let contents = std::fs::read_to_string(&written).expect("read attach.toml");
    assert!(contents.contains("name = \"alpha\""), "default name is alpha: {contents}");
    assert!(
        contents.contains("session_id = \"ses_new\""),
        "default session is the newest one: {contents}"
    );
    assert!(contents.contains("mode = \"attach\""), "mode is attach: {contents}");
    assert!(
        contents.contains(&format!("endpoint = \"{}\"", server.endpoint())),
        "endpoint is recorded: {contents}"
    );
}

/// `--force` is required to overwrite an existing `--out` file (exit 3
/// otherwise, file left untouched).
#[test]
fn init_refuses_overwrite_without_force() {
    let server = FakeServer::start();
    server.set_bare(200, json!([session_json("ses_1", "chat", 1_700_000_000_000)]).to_string());

    let state = StateDir::new();
    let out_path = state.path().join("attach.toml");
    std::fs::write(&out_path, "# pre-existing\n").expect("seed an existing file");

    let (code, _stdout, stderr) = run(
        &state,
        &[
            "body",
            "attach",
            "init",
            "--endpoint",
            &server.endpoint(),
            "--out",
            out_path.to_str().expect("utf8 path"),
        ],
    );
    assert_eq!(code, 3, "an existing --out without --force is exit 3; stderr: {stderr}");
    let contents = std::fs::read_to_string(&out_path).expect("read attach.toml");
    assert_eq!(contents, "# pre-existing\n", "the existing file must be left untouched");

    // Now with --force: it overwrites.
    let (code, _stdout, stderr) = run(
        &state,
        &[
            "body",
            "attach",
            "init",
            "--endpoint",
            &server.endpoint(),
            "--out",
            out_path.to_str().expect("utf8 path"),
            "--force",
        ],
    );
    assert_eq!(code, 0, "--force must allow the overwrite; stderr: {stderr}");
    let contents = std::fs::read_to_string(&out_path).expect("read attach.toml");
    assert!(contents.contains("session_id = \"ses_1\""), "the file was actually overwritten: {contents}");
}

/// The file `init` writes is not just hand-checked strings — it must
/// actually load through `holler-body`'s own real config loader
/// (`holler_body::config::load`) as a valid single `mode = "attach"`
/// session row.
#[test]
fn init_output_parses_with_body_config_loader() {
    let server = FakeServer::start();
    server.set_bare(200, json!([session_json("ses_xyz", "chat", 1_700_000_000_000)]).to_string());

    let state = StateDir::new();
    let out_path = state.path().join("attach.toml");
    let (code, _stdout, stderr) = run(
        &state,
        &[
            "body",
            "attach",
            "init",
            "--endpoint",
            &server.endpoint(),
            "--session",
            "ses_xyz",
            "--name",
            "beta",
            "--out",
            out_path.to_str().expect("utf8 path"),
        ],
    );
    assert_eq!(code, 0, "init must exit 0; stderr: {stderr}");

    let parsed = holler_body::config::load(&out_path).expect("the written file loads through the real config loader");
    assert_eq!(parsed.sessions.len(), 1, "exactly one session row: {parsed:?}");
    let row = &parsed.sessions[0];
    assert_eq!(row.name.as_str(), "beta");
    assert_eq!(row.mode, holler_body::config::SessionMode::Attach);
    assert_eq!(row.endpoint.as_deref(), Some(server.endpoint().as_str()));
    assert_eq!(row.session_id.as_deref(), Some("ses_xyz"));
    assert!(row.command.is_none(), "attach mode never carries a command");
}
