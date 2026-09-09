//! `say`'s pure logic (issue #190), split out of `main.rs` to keep that file
//! under the workspace's 900-line build guard (`scripts/lint.sh` check 4) —
//! the same split `query_cmd.rs` (issue #185) already made for `hub query`'s
//! own resolve/param logic. This module returns a plain [`SayResult`]; only
//! the bin (`main.rs`, the one file allowed to exit the process) turns that
//! into an actual exit.

use std::time::Duration;

use crate::Say;

/// What `say_command` (in `main.rs`) should print and exit with.
pub struct SayResult {
    /// The line to print — to stdout on success, stderr on every refusal.
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout (a reply).
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> SayResult {
    SayResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> SayResult {
    SayResult { message, to_stderr: true, exit_code }
}

/// Parse a `<number><s|m|h>` duration (the `say --timeout` grammar). `None`
/// on anything else — the call site fails closed (exit 3) on `None`, same
/// discipline as `main.rs`'s own `parse_ttl`.
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

/// `say`'s prompt text (issue #190): `TEXT` normally, or the concatenated
/// text parts of `--parts-file`'s A2A `Message` when given instead. Clap's
/// own `required_unless_present` on `Say::text` guarantees exactly one of
/// the two is present by the time this runs.
fn resolve_say_text(say: &Say) -> Result<String, String> {
    if let Some(path) = &say.parts_file {
        let content = std::fs::read_to_string(path).map_err(|e| format!("cannot read --parts-file {path}: {e}"))?;
        let message: holler_proto::Message = serde_json::from_str(&content)
            .map_err(|e| format!("--parts-file {path} is not a valid A2A Message: {e}"))?;
        Ok(message.parts.iter().filter_map(|p| p.text()).collect::<Vec<_>>().join(""))
    } else {
        Ok(say.text.clone().unwrap_or_default())
    }
}

/// The `-32009 session_busy` refusal's human hint (issue #190's exact
/// wording): `"<session> is <state> (turn <age>, last update <age> ago); use
/// 'interrupt <session> \"…\"' to replace it, or 'say --queue' to append"`.
///
/// A held permission/elicitation (issue #151, `data.state ==
/// "input-required"`) is a distinct hint: neither `interrupt` nor `--queue`
/// resolves it — only `answer` does, so the wording points there instead,
/// and drops the `--queue` suggestion entirely (`--queue` is refused on
/// `input-required`, never silently accepted — see `talk::say`'s own gate).
fn busy_hint(session: &str, e: &holler_proto::WireError) -> String {
    let data = e.data.as_ref();
    let state = data.and_then(|d| d.state.clone()).unwrap_or_else(|| "working".to_string());
    let turn_age = fmt_age(data.and_then(|d| d.turn_age_ms).unwrap_or(0));
    let last_update_age = fmt_age(data.and_then(|d| d.last_update_age_ms).unwrap_or(0));
    if state == "input-required" {
        format!(
            "session_busy: {session} is input-required (turn {turn_age}, last update {last_update_age} ago); \
             use 'answer {session} <choice>' to resolve it"
        )
    } else {
        format!(
            "session_busy: {session} is {state} (turn {turn_age}, last update {last_update_age} ago); \
             use 'interrupt {session} \"…\"' to replace it, or 'say --queue' to append"
        )
    }
}

/// Render a millisecond age as `<m>m<s>s` (or bare `<s>s` under a minute) —
/// the busy hint's own age formatting.
fn fmt_age(ms: u64) -> String {
    let total_secs = ms / 1000;
    let (m, s) = (total_secs / 60, total_secs % 60);
    if m > 0 {
        format!("{m}m{s}s")
    } else {
        format!("{s}s")
    }
}

/// `holler say SESSION TEXT [--timeout 600s] [--queue] [--json]` (issue
/// #190): resolve `SESSION` against the live hub, run one prompt turn, and
/// report the reply (or `--json`'s full result document) — or a refusal.
/// Exit codes: `0` a reply arrived, `1` every runtime refusal (no live hub,
/// not connected, unknown/ambiguous session, busy, connection lost, timeout,
/// cancelled — each with the spec's own wording; `2` an ambiguous session,
/// `3` a malformed `--timeout`/`--parts-file`.
pub fn run(say: &Say, json: bool) -> SayResult {
    let timeout = match parse_duration(&say.timeout) {
        Some(d) => d,
        None => return err(format!("invalid --timeout {:?} (use e.g. 30s, 5m, 1h)", say.timeout), 3),
    };
    let text = match resolve_say_text(say) {
        Ok(t) => t,
        Err(msg) => return err(msg, 3),
    };
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::say(&say.session, &text, say.queue, timeout) {
        Ok(doc) => {
            if json {
                ok(doc.to_string())
            } else {
                let text = doc.get("text").and_then(|v| v.as_str()).unwrap_or("");
                ok(text.to_string())
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::Refused(e)) => {
            // `ambiguous` (issue #190 spec: "lists candidates") is the one
            // `say` refusal that exits 2, not 1 — every other refusal
            // (unknown session, session_busy, not_connected, connection_lost,
            // cancelled) is a runtime failure (exit 1).
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            let message = if e.code == holler_proto::Code::SessionBusy.jsonrpc() {
                busy_hint(&say.session, &e)
            } else {
                e.message.clone()
            };
            err(message, if is_ambiguous { 2 } else { 1 })
        }
        Err(e) => err(e.to_string(), 1),
    }
}
