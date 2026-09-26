//! `hold` and `release` (issue #443, umbrella #437): the CLI half of the
//! session hold, plus the one piece of shared refusal formatting `say` and
//! `interrupt` use when they meet a held session. Like `say_cmd.rs`, this
//! module returns a plain [`HoldResult`]; only the bin turns it into an exit.
//!
//! Exit codes (the same conventions as `interrupt`): `0` success (including a
//! repeated `hold` or `release`), `1` a runtime refusal (no live hub, unknown
//! session), `2` an ambiguous session (candidates listed). `say` and
//! `interrupt SESSION TEXT` exit [`HELD_EXIT_CODE`] when the session is held,
//! so a script can tell "held" from "failed"; `say --grant` with a grant the
//! hub will not honour exits [`INVALID_GRANT_EXIT_CODE`].

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
    /// Printed to stderr as `warning: …` alongside a success (e.g. the hub
    /// could not save the change to disk).
    pub warning: Option<String>,
}

fn ok(message: String, warning: Option<String>) -> HoldResult {
    HoldResult { message, to_stderr: false, exit_code: 0, warning }
}
fn err(message: String, exit_code: i32) -> HoldResult {
    HoldResult { message, to_stderr: true, exit_code, warning: None }
}

const NOT_SAVED: &str = "the hub could not save this change to disk; it is in force now but will not survive a hub restart";

fn run(
    json: bool,
    call: impl FnOnce() -> Result<serde_json::Value, holler_hub::control::ControlError>,
    text: impl FnOnce(&serde_json::Value) -> (String, Option<String>),
) -> HoldResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match call() {
        Ok(doc) => {
            let (out, mut warning) = text(&doc);
            if doc.get("persisted").and_then(|v| v.as_bool()) == Some(false) {
                warning = Some(NOT_SAVED.to_string());
            }
            ok(if json { doc.to_string() } else { out }, warning)
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
            (format!("{verb} {session} since {since}{reason}"), None)
        },
    )
}

/// `holler release SESSION [--once [--ttl DURATION]] [--json]`.
pub fn release(release: &Release, json: bool) -> HoldResult {
    if release.once {
        return release_once(release, json);
    }
    run(
        json,
        || holler_hub::control::release(&release.session),
        |doc| {
            let session = doc.get("session").and_then(|v| v.as_str()).unwrap_or(&release.session);
            let text = match doc.get("lifted").and_then(|v| v.as_str()) {
                None => format!("{session} was not held"),
                Some(kind) if doc.get("still_held").and_then(|v| v.as_bool()) == Some(true) => {
                    format!("released the {kind} hold on {session}; it is still held by default (release again to lift that)")
                }
                Some("default") => format!("released the default hold on {session}"),
                Some(_) => format!("released {session}"),
            };
            (text, None)
        },
    )
}

/// `holler release SESSION --once [--ttl DURATION]` (issue #460): prints the
/// grant id (alone on stdout, so `GRANT=$(holler release S --once)` works).
fn release_once(release: &Release, json: bool) -> HoldResult {
    let ttl = match release.ttl.as_deref().map(crate::say_cmd::parse_duration) {
        None => None,
        Some(Some(d)) => Some(d),
        Some(None) => return err(format!("invalid --ttl {:?} (use e.g. 30s, 5m, 1h)", release.ttl.as_deref().unwrap_or("")), 3),
    };
    run(
        json,
        || holler_hub::control::release_once(&release.session, ttl),
        |doc| {
            let grant = doc.get("grant").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let warning = if doc.get("operator_held").and_then(|v| v.as_bool()) == Some(true) {
                Some("the session also has an operator hold, which a grant does not lift; release that first".to_string())
            } else if doc.get("default_held").and_then(|v| v.as_bool()) == Some(false) {
                Some("the session has no default hold, so it is not refusing prompts and needs no grant".to_string())
            } else {
                None
            };
            (grant, warning)
        },
    )
}

/// `true` for a `-32011 session_held` wire error.
pub fn is_held(e: &holler_proto::WireError) -> bool {
    e.code == holler_proto::Code::SessionHeld.jsonrpc()
}

/// `true` for a `-32012 invalid_grant` wire error.
pub fn is_invalid_grant(e: &holler_proto::WireError) -> bool {
    e.code == holler_proto::Code::InvalidGrant.jsonrpc()
}

/// The exit code of a `say --grant` refused because the grant is unknown,
/// expired, used or for another session (`-32012 invalid_grant`).
pub const INVALID_GRANT_EXIT_CODE: i32 = 5;

/// What `say` / `interrupt SESSION TEXT` print for a held session: the
/// session, which hold it is (an `operator` hold or a `default` one from
/// joining held), the reason and the since-time. With `--json`, the same
/// facts as one object on stdout.
pub fn held_refusal(session: &str, e: &holler_proto::WireError, json: bool) -> (String, bool) {
    let data = e.data.as_ref();
    let reason = data.and_then(|d| d.reason.as_deref());
    let since = data.and_then(|d| d.since.as_deref());
    let kind = data.and_then(|d| d.hold_kind.as_deref()).unwrap_or("operator");
    if json {
        let doc = serde_json::json!({ "error": "session_held", "session": session, "reason": reason, "since": since, "hold_kind": kind });
        return (doc.to_string(), false);
    }
    let mut msg = format!("session_held: {session} is held");
    if let Some(s) = since {
        msg.push_str(&format!(" (since {s})"));
    }
    if let Some(r) = reason {
        msg.push_str(&format!(": {r}"));
    }
    msg.push_str(if kind == "default" {
        "; it joined held, so a one-time grant (release --once, then say --grant) or a release lifts it"
    } else {
        "; ask whoever holds it to release it"
    });
    (msg, true)
}

/// What `say --grant` prints for a grant the hub did not honour.
pub fn invalid_grant_refusal(session: &str, e: &holler_proto::WireError, json: bool) -> (String, bool) {
    let reason = e.data.as_ref().and_then(|d| d.reason.as_deref()).unwrap_or("unknown");
    if json {
        let doc = serde_json::json!({ "error": "invalid_grant", "session": session, "reason": reason });
        return (doc.to_string(), false);
    }
    (format!("invalid_grant: {}", e.message), true)
}
