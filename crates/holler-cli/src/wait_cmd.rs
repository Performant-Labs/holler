//! `wait`'s pure logic (issue #142), split out of `main.rs` to keep that file
//! under the workspace's 900-line build guard (`scripts/lint.sh` check 4) —
//! the same split `say_cmd.rs`/`roster_cmd.rs`/`query_cmd.rs` already made
//! for their own leaves. This module returns a plain [`WaitResult`]; only the
//! bin (`main.rs`) turns that into an actual process exit.

use std::time::Duration;

use crate::Wait;

/// What `wait_command` (in `main.rs`) should print and exit with.
pub struct WaitResult {
    /// The line(s) to print — the matched rows (or `--json`'s raw document)
    /// on a match, the refusal's message on every error, nothing on a
    /// timeout (ADR 0003: a timeout is a quiet exit 2, not a printed error).
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout (a match, or `--json`'s document either way).
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> WaitResult {
    WaitResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> WaitResult {
    WaitResult { message, to_stderr: true, exit_code }
}

/// Parse a `<number><s|m|h>` duration (the `wait --timeout` grammar, shared
/// with `say --timeout`'s own copy — each leaf module owns its copy rather
/// than a shared dependency for four lines of parsing).
fn parse_duration(s: &str) -> Option<Duration> {
    let digits: &str = s.split(|c: char| !c.is_ascii_digit()).next()?;
    if digits.is_empty() {
        return None;
    }
    let rest = &s[digits.len()..];
    if rest.len() != 1 {
        return None;
    }
    let num: u64 = digits.parse().ok()?;
    match rest {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 3600)),
        _ => None,
    }
}

/// `holler wait SESSION[,SESSION…] [--prefix P] [--until STATES] [--after
/// TURN_ID] [--timeout 600s] [--json]` (issue #142): block on the live hub's
/// roster until a named session (or every row under `--prefix`) matches one
/// of `--until`'s target states, or `--timeout` elapses. Exit codes: `0` a
/// match landed (rows printed — one line per match, or `--json`'s array),
/// `1` no live hub reachable / a control-socket error, `2` the timeout
/// elapsed with no match, `3` a malformed `--timeout`.
pub fn run(wait: &Wait, json: bool) -> WaitResult {
    let timeout = match parse_duration(&wait.timeout) {
        Some(d) => d,
        None => return err(format!("invalid --timeout {:?} (use e.g. 30s, 5m, 1h)", wait.timeout), 3),
    };
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::wait(
        wait.sessions.as_deref(),
        wait.prefix.as_deref(),
        wait.until.as_deref(),
        wait.after.as_deref(),
        timeout,
    ) {
        Ok(doc) => {
            let matched = doc.get("matched").and_then(serde_json::Value::as_bool).unwrap_or(false);
            if json {
                ok(doc.to_string())
            } else if matched {
                ok(render_matches(&doc))
            } else {
                // A timeout is a quiet exit 2 (ADR 0003) — nothing printed.
                WaitResult { message: String::new(), to_stderr: false, exit_code: 2 }
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(e) => err(e.to_string(), 1),
    }
}

/// Render `{matched:true, rows:[...]}` as one `SESSION STATE TURN_ID
/// STOP_REASON AGE` line per matched row (the spec's own human-readable
/// shape). `AGE` is how long ago the matched turn ended (`age_secs`, computed
/// hub-side — see `control_server.rs`'s `row_to_json`) or `-` for a
/// live-state match (`input-required`, `gone`), which has no single
/// "since when" instant of its own here.
fn render_matches(doc: &serde_json::Value) -> String {
    let rows = doc.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let mut out = String::new();
    for row in &rows {
        let session = row.get("session").and_then(|v| v.as_str()).unwrap_or("-");
        let state = row.get("state").and_then(|v| v.as_str()).unwrap_or("-");
        let turn_id = row.get("turn_id").and_then(|v| v.as_str()).unwrap_or("-");
        let stop_reason = row.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("-");
        let age = row
            .get("age_secs")
            .and_then(serde_json::Value::as_u64)
            .map(fmt_age_secs)
            .unwrap_or_else(|| "-".to_string());
        out.push_str(&format!("{session} {state} {turn_id} {stop_reason} {age}\n"));
    }
    out
}

/// `<n>s ago` under a minute, `<n>m ago` at or past it.
fn fmt_age_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else {
        format!("{}m ago", secs / 60)
    }
}
