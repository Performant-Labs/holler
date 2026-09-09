//! `answer`'s pure logic (issue #151), split out of `main.rs` to keep that
//! file under the workspace's 900-line build guard (`scripts/lint.sh` check
//! 4) — the same split `say_cmd.rs` (issue #190) made for `say`. This module
//! returns a plain [`AnswerResult`]; only the bin (`main.rs`, the one file
//! allowed to exit the process) turns that into an actual exit.

use crate::Answer;

/// What `answer_command` (in `main.rs`) should print and exit with.
pub struct AnswerResult {
    /// The line to print — to stdout on success, stderr on every refusal.
    pub message: String,
    /// `true` prints `message` to stderr (a refusal); `false` prints to
    /// stdout (a confirmation).
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> AnswerResult {
    AnswerResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> AnswerResult {
    AnswerResult { message, to_stderr: true, exit_code }
}

/// `holler answer SESSION CHOICE [--json]` (issue #151): resolve `SESSION`
/// against the live hub and resolve its held permission/elicitation with
/// `CHOICE` — an index, a label/key, a comma-separated list (one segment per
/// question), or `once`/`always`/`reject`. Exit codes: `0` the answer was
/// applied; `1` every runtime refusal (no live hub, not connected, unknown
/// session, nothing pending, the choice did not resolve, connection lost);
/// `2` an ambiguous session (lists candidates, matching `say`'s own
/// convention).
pub fn run(answer: &Answer, json: bool) -> AnswerResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::answer(&answer.session, &answer.choice) {
        Ok(doc) => {
            if json {
                ok(doc.to_string())
            } else {
                let applied = doc.get("applied").and_then(|v| v.as_bool()).unwrap_or(false);
                ok(if applied { "applied".to_string() } else { "not applied".to_string() })
            }
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::Refused(e)) => {
            // `ambiguous` (matching `say`'s own convention: "lists
            // candidates") is the one refusal that exits 2, not 1 — every
            // other refusal (unknown session, nothing_pending, not_connected,
            // invalid_params, connection_lost) is a runtime failure (exit 1).
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            err(e.message.clone(), if is_ambiguous { 2 } else { 1 })
        }
        Err(e) => err(e.to_string(), 1),
    }
}
