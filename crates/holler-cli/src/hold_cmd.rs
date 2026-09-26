//! `hold` and `release` (issue #443, umbrella #437): the CLI half of the
//! session hold, plus the one piece of shared refusal formatting `say` and
//! `interrupt` use when they meet a held session. Like `say_cmd.rs`, this
//! module returns a plain [`HoldResult`]; only the bin turns it into an exit.
//!
//! Exit codes (the same conventions as `interrupt`): `0` success (including a
//! repeated `hold` or `release`), `1` a runtime refusal (no live hub, unknown
//! session), `2` an ambiguous session (candidates listed). `say` and
//! `interrupt SESSION TEXT` exit [`HELD_EXIT_CODE`] when the session is held,
//! so a script can tell "held" from "failed".

use crate::{Hold, Release};

/// The exit code of a `say` (or `interrupt SESSION TEXT`) refused because the
/// session is held (`-32011 session_held`). Distinct from `1` (failed), `2`
/// (ambiguous) and `3` (malformed argument). Documented in `docs/protocol/
/// v2.md` §10 and pinned by `hold_cli_test`.
pub const HELD_EXIT_CODE: i32 = 4;

/// What `main.rs` should print and exit with.
pub struct HoldResult {
    pub message: String,
    /// `true` prints to stderr (a refusal), `false` to stdout.
    pub to_stderr: bool,
    pub exit_code: i32,
}

fn ok(message: String) -> HoldResult {
    HoldResult { message, to_stderr: false, exit_code: 0 }
}
fn err(message: String, exit_code: i32) -> HoldResult {
    HoldResult { message, to_stderr: true, exit_code }
}

/// The lines every successful hold/release adds when the hub could not save
/// its state file: the hold is in force but will not survive a hub restart.
const NOT_SAVED: &str = "warning: the hub could not save this change to disk; it is in force now but will not survive a hub restart";

fn run(
    json: bool,
    call: impl FnOnce() -> Result<serde_json::Value, holler_hub::control::ControlError>,
    text: impl FnOnce(&serde_json::Value) -> String,
) -> HoldResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match call() {
        Ok(doc) => {
            if json {
                return ok(doc.to_string());
            }
            let mut out = text(&doc);
            if doc.get("persisted").and_then(|v| v.as_bool()) == Some(false) {
                out.push('\n');
                out.push_str(NOT_SAVED);
            }
            ok(out)
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::Refused(e)) => {
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
            err(e.message.clone(), if is_ambiguous { 2 } else { 1 })
        }
        Err(e) => err(e.to_string(), 1),
    }
}

/// `holler hold SESSION [--reason TEXT] [--json]`.
pub fn hold(hold: &Hold, json: bool) -> HoldResult {
    run(
        json,
        || holler_hub::control::hold(&hold.session, hold.reason.as_deref()),
        |doc| {
            let session = doc.get("session").and_then(|v| v.as_str()).unwrap_or(&hold.session);
            let since = doc.get("since").and_then(|v| v.as_str()).unwrap_or("-");
            let reason = doc.get("reason").and_then(|v| v.as_str()).map(|r| format!(": {r}")).unwrap_or_default();
            let verb = if doc.get("newly_held").and_then(|v| v.as_bool()) == Some(false) { "already held" } else { "held" };
            format!("{verb} {session} since {since}{reason}")
        },
    )
}

/// `holler release SESSION [--json]`.
pub fn release(release: &Release, json: bool) -> HoldResult {
    run(
        json,
        || holler_hub::control::release(&release.session),
        |doc| {
            let session = doc.get("session").and_then(|v| v.as_str()).unwrap_or(&release.session);
            if doc.get("was_held").and_then(|v| v.as_bool()) == Some(false) {
                format!("{session} was not held")
            } else {
                format!("released {session}")
            }
        },
    )
}

/// `true` for a `-32011 session_held` wire error.
pub fn is_held(e: &holler_proto::WireError) -> bool {
    e.code == holler_proto::Code::SessionHeld.jsonrpc()
}

/// What `say` / `interrupt SESSION TEXT` print for a held session: the
/// session, the reason and the since-time, and what to do about it. With
/// `--json`, the same facts as one object on stdout.
pub fn held_refusal(session: &str, e: &holler_proto::WireError, json: bool) -> (String, bool) {
    let data = e.data.as_ref();
    let reason = data.and_then(|d| d.reason.as_deref());
    let since = data.and_then(|d| d.since.as_deref());
    if json {
        let doc = serde_json::json!({ "error": "session_held", "session": session, "reason": reason, "since": since });
        return (doc.to_string(), false);
    }
    let mut msg = format!("session_held: {session} is held");
    if let Some(s) = since {
        msg.push_str(&format!(" (since {s})"));
    }
    if let Some(r) = reason {
        msg.push_str(&format!(": {r}"));
    }
    msg.push_str("; ask whoever holds it to release it");
    (msg, true)
}
