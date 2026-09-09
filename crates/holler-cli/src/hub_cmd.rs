//! `holler hub status|caps|support|query` (stories #143, #185), split out
//! of `main.rs` to keep that file under the workspace's 900-line build
//! guard (`scripts/lint.sh` check 4) — issue #228, the same split
//! `say_cmd.rs`/`roster_cmd.rs`/`token_cmd.rs` made for their own leaves.
//!
//! All four are control-socket round trips: no live hub reachable is exit 1
//! with the spec's exact "no live holler hub reachable" message. `query
//! TARGET …` additionally forwards over the target body's own live socket;
//! a target that resolves to zero or more-than-one live body is
//! distinguished from an ordinary refusal (see [`query`]'s own doc).
//!
//! `caps`/`support`/`query` share their print+exit tail with `body_cmd.rs`'s
//! own versions via [`crate::query_cmd::print_and_exit_code`] (issue #229) —
//! see that function's doc for why the fetch itself (a control-socket round
//! trip here, a local lookup there) stays separate.

use crate::query_cmd::{query_cmd_params, is_ambiguous, print_and_exit_code, FetchOutcome};
use crate::Query;

/// `holler hub status [--json]` — read the live hub's status document over
/// the control socket and print it (story #143). With `--json`, the raw
/// `StatusDoc` JSON goes to stdout (machine-readable). Without, a short
/// human-readable summary goes to stdout. If no live hub is reachable, the
/// spec's exact message goes to stderr and the exit code is 1.
pub fn status(json: bool) -> i32 {
    // The state dir may be unresolvable (no `HOLLER_STATE_DIR`/`$HOME`); then
    // there is no live hub and the error message below just has no path to name.
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::status() {
        Ok(doc) => {
            if json {
                println!("{}", doc);
            } else {
                let listening = doc
                    .get("listening")
                    .and_then(|l| l.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "(none)".to_string());
                let clients = doc.get("clients").and_then(|v| v.as_u64()).unwrap_or(0);
                let sessions = doc.get("sessions").and_then(|v| v.as_u64()).unwrap_or(0);
                let version = doc.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                println!("hub {version} (protocol 2)");
                println!("  listening: {listening}");
                println!("  clients:   {clients}");
                println!("  sessions:  {sessions}");
            }
            0
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            eprintln!(
                "error: no live holler hub reachable at {}",
                state_root.display()
            );
            1
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

// --- `holler hub caps|support|query` (issue #185) ---------------------------
//
// All three are control-socket round trips (like `hub status`): no live hub
// reachable is exit 1 with the spec's exact "no live holler hub reachable"
// message. `query TARGET …` additionally forwards over the target body's own
// live socket; a target that resolves to zero or more-than-one live body is
// distinguished from an ordinary refusal (see `query`'s own doc).

/// `holler hub caps [--json]` (issue #185): the live hub's `query/caps`
/// document (status + a support answer for every known id).
pub fn caps(json: bool) -> i32 {
    let outcome = match holler_hub::control::caps() {
        Ok(doc) => {
            let n = doc.get("caps").and_then(|c| c.as_object()).map(|o| o.len()).unwrap_or(0);
            FetchOutcome::Doc { json_text: doc.to_string(), human: format!("hub: {n} caps known") }
        }
        Err(e) => control_error(e),
    };
    print_and_exit_code(json, outcome)
}

/// `holler hub support FEATURE [--json]` (issue #185): a single
/// `query/support` answer from the live hub.
pub fn support(feature: &str, json: bool) -> i32 {
    let outcome = match holler_hub::control::support(feature) {
        Ok(doc) => {
            let ok = doc.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            FetchOutcome::Doc {
                json_text: doc.to_string(),
                human: format!("{feature}: {}", if ok { "ok" } else { "not ok" }),
            }
        }
        Err(e) => control_error(e),
    };
    print_and_exit_code(json, outcome)
}

/// `holler hub query CMD [ARGS...]` (local) or `holler hub query TARGET CMD
/// [ARGS...]` (remote, forwarded to that body's live socket) — issue #185.
/// ADR 0003's exit codes: 0 on an answer, 1 when no live hub (or, for the
/// remote form, no live *body* matching TARGET) is reachable, 2 when TARGET
/// is ambiguous or the query tail itself is malformed.
pub fn query(q: &Query, json: bool) -> i32 {
    let resolved = match q.resolve() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let method = resolved.cmd().method();
    let params = query_cmd_params(resolved.cmd(), resolved.args());

    let result = match resolved.target() {
        None => holler_hub::control::query_local(method, params),
        Some(target) => holler_hub::control::query_remote(target.as_str(), method, params),
    };
    let outcome = match result {
        Ok(doc) => FetchOutcome::Doc { json_text: doc.to_string(), human: format!("{method}: ok") },
        Err(holler_hub::control::ControlError::Refused(e)) if is_ambiguous(&e) => {
            eprintln!("error: {}", e.message);
            return 2;
        }
        Err(e) => control_error(e),
    };
    print_and_exit_code(json, outcome)
}

/// The shared refusal mapping for a [`holler_hub::control::ControlError`]:
/// no live hub reachable is exit 1 (the spec's exact message); every other
/// refusal (not connected, `-32004`) is exit 1 too — only an ambiguous
/// TARGET (handled at the `query` call site, before this is reached) is exit
/// 2.
fn control_error(e: holler_hub::control::ControlError) -> FetchOutcome {
    let message = match &e {
        holler_hub::control::ControlError::NoLiveHub => {
            let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
            format!("no live holler hub reachable at {}", state_root.display())
        }
        other => other.to_string(),
    };
    FetchOutcome::Err { message, exit_code: 1 }
}
