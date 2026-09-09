//! `roster`'s pure logic (issue #186), split out of `main.rs` to keep that
//! file under the workspace's 900-line build guard (`scripts/lint.sh` check
//! 4) — the same split `say_cmd.rs` (issue #190) made for `say` and
//! `query_cmd.rs` (issue #185) made for `hub query`. This module returns a
//! plain [`RosterResult`]; only the bin (`main.rs`, the one file allowed to
//! exit the process) turns that into an actual exit.
//!
//! The dispatch: `main` routes `Command::Roster` here (issue #186 made
//! `roster` a live verb; #190 did the same for `say`; `interrupt`, #191,
//! doesn't yet).

use crate::Roster;

/// What `roster_command` (in `main.rs`) should print and exit with.
pub struct RosterResult {
    /// The output to print — the table (or `--json`'s raw document) on
    /// success, the refusal's message on every error.
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout (the roster table or JSON document).
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> RosterResult {
    RosterResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> RosterResult {
    RosterResult { message, to_stderr: true, exit_code }
}

/// `holler roster [--all] [--prefix PREFIX] [--json]` (issue #186; `--prefix`
/// added by issue #236, ADR 0005 §4): read the live hub's roster over the
/// control socket and report it — the table (or `--json`'s raw document) on
/// success, a refusal on every error. Exit codes: `0` a roster came back
/// (live-only view by default, `--all` adds `gone`, `--prefix` narrows to a
/// label or a deeper prefix), `1` every runtime refusal (no live hub
/// reachable, a control-socket I/O error, or a bad reply).
pub fn run(roster: &Roster, json: bool) -> RosterResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::roster(roster.all, roster.prefix.as_deref()) {
        Ok(doc) => {
            if json {
                ok(doc.to_string())
            } else {
                ok(render_table(&doc))
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(e) => err(e.to_string(), 1),
    }
}

/// Render the roster reply (`{rows: [...]}`) as the spec's table. Columns are
/// the row's display fields; a held permission is the `PENDING` column (the
/// human view shows just its count — `--json` carries the full array). An
/// empty roster prints a friendly one-liner instead of a bare header.
fn render_table(doc: &serde_json::Value) -> String {
    let rows = doc.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    if rows.is_empty() {
        return "(no sessions on the hub)\n".to_string();
    }
    let mut out = String::new();
    out.push_str(&format!(
        "{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} LAST TURN\n",
        "SESSION", "HARNESS", "MODE", "STATE", "CONN", "HOSTNAME", "PENDING"
    ));
    for row in &rows {
        let pending = row
            .get("pending")
            .and_then(|p| p.as_array())
            .map(|items| items.len().to_string())
            .unwrap_or_else(|| "-".into());
        out.push_str(&format!(
            "{:<20} {:<10} {:<8} {:<10} {:<10} {:<8} {:<8} {}\n",
            row.get("name").and_then(|v| v.as_str()).unwrap_or("-"),
            row.get("harness").and_then(|v| v.as_str()).unwrap_or("-"),
            row.get("mode").and_then(|v| v.as_str()).unwrap_or("-"),
            row.get("state").and_then(|v| v.as_str()).unwrap_or("-"),
            row.get("conn_state").and_then(|v| v.as_str()).unwrap_or("-"),
            row.get("hostname").and_then(|v| v.as_str()).unwrap_or("-"),
            pending,
            last_turn_display(row.get("last_turn"))
        ));
    }
    out
}

/// The `LAST TURN` column (issue #142): `<state> <stop_reason> <age> ago`
/// (e.g. `completed end_turn 2m ago`) once a turn has ended, or `-` before
/// the session's first turn / for a row that predates this field. `age` is
/// computed from `ended_at` via a plain epoch-seconds subtraction (the same
/// RFC 3339 shape the hub always emits — see `holler_hub::roster::
/// parse_rfc3339`'s own doc for why there is only one parser for it).
fn last_turn_display(last_turn: Option<&serde_json::Value>) -> String {
    let Some(lt) = last_turn.filter(|v| !v.is_null()) else {
        return "-".to_string();
    };
    let state = lt.get("state").and_then(|v| v.as_str()).unwrap_or("-");
    let stop_reason = lt.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("-");
    let ended_at = lt.get("ended_at").and_then(|v| v.as_str());
    let age = ended_at
        .and_then(holler_hub::roster::parse_rfc3339_secs_since)
        .map(|secs| if secs < 60 { format!("{secs}s ago") } else { format!("{}m ago", secs / 60) })
        .unwrap_or_else(|| "-".to_string());
    format!("{state} {stop_reason} {age}")
}
