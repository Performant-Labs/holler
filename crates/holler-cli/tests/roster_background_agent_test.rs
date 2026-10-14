#![allow(clippy::unwrap_used)] // #914
#![allow(clippy::expect_used)] // #914
#![allow(clippy::panic)] // #914
//! `backgroundAgentLastOutput` (issue #914, for pfleet#506): a Claude row's roster JSON carries
//! the epoch seconds of the newest write under its session's `subagents/` directory, or null.
//!
//! Real temp directories with fabricated sidechain files and mtimes set by `set_modified`; the
//! roster's own `render_in` over a literal hub reply. No session of this machine is read, and no
//! hub type is faked.

use std::fs::{self, File, FileTimes};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use clap::Parser;
use holler_cli::output::{resolve_format, Sink};
use holler_cli::roster_agent::last_output;
use holler_cli::roster_cmd::{self, Registry};
use holler_cli::{Cli, Command};
use holler_pane::PaneError;
use serde_json::{json, Value};

const SESSION: &str = "a2ed9dcf-8ee4-4cf1-9ec6-54de111215cf";

/// Write `projects/<project>/<session>/subagents/<file>` with its mtime at `secs`.
fn sidechain(projects: &Path, project: &str, session: &str, file: &str, secs: u64) {
    let dir = projects.join(project).join(session).join("subagents");
    fs::create_dir_all(&dir).unwrap();
    let handle = File::create(dir.join(file)).unwrap();
    handle
        .set_times(FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(secs)))
        .unwrap();
}

#[test]
fn newest_agent_file_of_the_session_wins() {
    let root = tempfile::tempdir().unwrap();
    sidechain(root.path(), "proj-a", SESSION, "agent-aaa.jsonl", 1_000);
    sidechain(root.path(), "proj-a", SESSION, "agent-bbb.jsonl", 3_000);
    sidechain(root.path(), "proj-a", SESSION, "agent-ccc.jsonl", 2_000);
    assert_eq!(last_output(root.path(), SESSION), Some(3_000));
}

#[test]
fn the_session_is_found_under_any_project_directory() {
    let root = tempfile::tempdir().unwrap();
    sidechain(
        root.path(),
        "proj-a",
        "other-session",
        "agent-x.jsonl",
        9_000,
    );
    sidechain(root.path(), "proj-b", SESSION, "agent-y.jsonl", 4_000);
    assert_eq!(last_output(root.path(), SESSION), Some(4_000));
}

#[test]
fn only_agent_jsonl_files_count() {
    let root = tempfile::tempdir().unwrap();
    sidechain(root.path(), "p", SESSION, "agent-aaa.jsonl", 1_000);
    sidechain(root.path(), "p", SESSION, "agent-aaa.meta.json", 8_000);
    sidechain(root.path(), "p", SESSION, "notes.txt", 8_000);
    assert_eq!(last_output(root.path(), SESSION), Some(1_000));
}

#[test]
fn a_symlink_reports_its_targets_write_time_and_a_directory_is_not_a_transcript() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("p").join(SESSION).join("subagents");
    fs::create_dir_all(&dir).unwrap();
    let target = root.path().join("elsewhere.jsonl");
    File::create(&target)
        .unwrap()
        .set_times(FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(6_000)))
        .unwrap();
    std::os::unix::fs::symlink(&target, dir.join("agent-link.jsonl")).unwrap();
    // A dangling link and a directory named like a transcript count for nothing.
    std::os::unix::fs::symlink(root.path().join("gone"), dir.join("agent-dangling.jsonl")).unwrap();
    fs::create_dir(dir.join("agent-dir.jsonl")).unwrap();
    assert_eq!(last_output(root.path(), SESSION), Some(6_000));
}

#[test]
fn a_stray_file_among_the_projects_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    File::create(root.path().join("stray.txt")).unwrap();
    sidechain(root.path(), "p", SESSION, "agent-a.jsonl", 7_000);
    assert_eq!(last_output(root.path(), SESSION), Some(7_000));
}

#[test]
fn null_cases_are_none_and_never_zero() {
    let root = tempfile::tempdir().unwrap();
    // No projects directory at all.
    assert_eq!(last_output(&root.path().join("absent"), SESSION), None);
    // A projects directory with no such session.
    fs::create_dir_all(root.path().join("p")).unwrap();
    assert_eq!(last_output(root.path(), SESSION), None);
    // A session with no subagents directory.
    fs::create_dir_all(root.path().join("p").join(SESSION)).unwrap();
    assert_eq!(last_output(root.path(), SESSION), None);
    // An empty subagents directory.
    fs::create_dir_all(root.path().join("p").join(SESSION).join("subagents")).unwrap();
    assert_eq!(last_output(root.path(), SESSION), None);
    // A file stamped at the epoch is "cannot tell", not 0.
    sidechain(root.path(), "p", SESSION, "agent-zero.jsonl", 0);
    assert_eq!(last_output(root.path(), SESSION), None);
}

#[test]
fn a_session_id_that_is_not_a_plain_name_is_never_looked_up() {
    let root = tempfile::tempdir().unwrap();
    sidechain(root.path(), "p", "real", "agent-a.jsonl", 5_000);
    for id in ["", ".", "..", "../p/real", "p/real", "real/..", "a\\b"] {
        assert_eq!(last_output(root.path(), id), None, "{id:?}");
    }
}

fn hub_row(name: &str, harness: &str, session: Option<&str>) -> Value {
    json!({
        "name": name, "harness": harness, "mode": "attach", "state": "working",
        "conn_state": "connected", "token_id": "t", "client_id": "c", "hostname": "h",
        "harness_session_id": session, "first_seen": 1, "last_seen": 2,
        "last_update_at": null, "pending": null, "turn_id": null, "last_turn": null
    })
}

/// The `rows` of `roster <flags>` over `rows`, with `projects` as the Claude projects directory.
fn rows_of(rows: Vec<Value>, flags: &[&str], projects: Option<&Path>) -> Vec<Value> {
    let mut argv = vec!["holler", "roster"];
    argv.extend_from_slice(flags);
    let cli = Cli::try_parse_from(&argv).unwrap();
    let choice = resolve_format(cli.json, cli.format).unwrap();
    let Command::Roster(roster) = &cli.command else {
        panic!("parsed `holler roster`")
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = roster_cmd::render_in(
        &json!({ "rows": rows }),
        Registry::Failed(PaneError::NotImplemented),
        roster,
        choice,
        &mut Sink {
            out: &mut out,
            err: &mut err,
        },
        projects,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    let doc: Value = serde_json::from_slice(&out).unwrap();
    let rows = doc.get("data").unwrap_or(&doc).get("rows").unwrap();
    rows.as_array().unwrap().clone()
}

fn field(row: &Value) -> &Value {
    row.get("backgroundAgentLastOutput")
        .expect("the key is never left out")
}

#[test]
fn a_claude_row_carries_the_epoch_seconds_in_both_json_modes() {
    let root = tempfile::tempdir().unwrap();
    sidechain(root.path(), "p", SESSION, "agent-a.jsonl", 1_790_000_000);
    for flags in [&["--json"][..], &["--format=json"][..]] {
        let rows = rows_of(
            vec![hub_row("r1c3/work", "claude", Some(SESSION))],
            flags,
            Some(root.path()),
        );
        assert_eq!(field(&rows[0]), &json!(1_790_000_000_u64), "{flags:?}");
    }
}

#[test]
fn every_other_row_reads_null() {
    let root = tempfile::tempdir().unwrap();
    sidechain(root.path(), "p", SESSION, "agent-a.jsonl", 1_790_000_000);
    let rows = rows_of(
        vec![
            // not a Claude pane, though the session id matches a directory
            hub_row("o1/work", "opencode", Some(SESSION)),
            // a Claude row with no session id yet
            hub_row("r1c1/work", "claude", None),
            // a Claude row whose session id key is absent
            {
                let mut row = hub_row("r1c4/work", "claude", None);
                row.as_object_mut().unwrap().remove("harness_session_id");
                row
            },
            // a Claude row whose session has no sidechain files
            hub_row("r1c2/work", "claude", Some("no-such-session")),
        ],
        &["--json"],
        Some(root.path()),
    );
    assert_eq!(rows.len(), 4);
    for row in &rows {
        assert_eq!(field(row), &Value::Null, "{row}");
    }
}

#[test]
fn with_no_projects_directory_every_row_reads_null() {
    let rows = rows_of(
        vec![hub_row("r1c3/work", "claude", Some(SESSION))],
        &["--json"],
        None,
    );
    assert_eq!(field(&rows[0]), &Value::Null);
}
