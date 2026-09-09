//! Pure helpers behind `hub`/`body` `caps`/`support`/`query` (issue #185).
//!
//! Kept in the lib crate (not `main.rs`) purely to keep `main.rs` — which
//! `scripts/lint.sh`'s file-size gate caps at 900 lines, and which is the
//! only place a process exit call may appear (issue #147 §2) — under that
//! limit. None of these helpers exit the process; every exit decision for a
//! `hub`/`body` `caps`/`support`/`query` leaf still lives in `main.rs`, which
//! calls these for their pure return value and applies the ADR 0003 exit
//! code itself.

use crate::cli::Cmd;

/// Build a `query/*` request's `params` from the CLI's already-parsed
/// command verb and its trailing args. Only `support` (`{feature}`) and
/// `protocol` (`{version?}`) take any; `status`/`caps` take none.
pub fn query_cmd_params(cmd: &Cmd, args: &[String]) -> Option<serde_json::Value> {
    match cmd {
        Cmd::Support => args.first().map(|f| serde_json::json!({ "feature": f })),
        Cmd::Protocol => args
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .map(|v| serde_json::json!({ "version": v })),
        Cmd::Status | Cmd::Caps => None,
    }
}

/// `true` iff a control refusal is the hub's ambiguous-target marker (see
/// `holler_hub::control_server`'s `hub_query_remote` — an ambiguous TARGET is
/// reported as an `invalid_request` whose message names the ambiguity, since
/// the wire's error table has no dedicated code for it and this is purely a
/// CLI-target-resolution concern, never sent peer-to-peer).
pub fn is_ambiguous(e: &holler_proto::WireError) -> bool {
    e.message.contains("ambiguous")
}

/// Resolve this body's session configs for a local `caps`/`support`/`query`
/// leaf. Uses the same discovery order `body run` does (`HOLLER_CONFIG` >
/// `./sessions.toml` > `./session.toml`), plus one more fallback these
/// leaves add: `<state>/body/sessions.toml` / `session.toml` — the body's
/// own state dir, so `caps`/`support`/`query` find the same config a `body
/// run` in that state dir was started with even when invoked from a
/// different cwd (these leaves have no `--config` flag of their own; ADR
/// 0003 gives that flag only to `run`). Unlike `body run`'s fail-closed
/// "every session is explicit", a missing config file is **not** a refusal
/// here: these leaves must still answer (with an empty session/harness
/// list) when no config exists yet.
pub fn body_local_configs(state_root: &std::path::Path) -> Vec<holler_body::config::SessionConfig> {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Ok(p) = holler_body::config::discover_and_load(None, &cwd) {
        return p.sessions;
    }
    let body_dir = state_root.join("body");
    for name in ["sessions.toml", "session.toml"] {
        let path = body_dir.join(name);
        if path.is_file() {
            if let Ok(p) = holler_body::config::load(&path) {
                return p.sessions;
            }
        }
    }
    Vec::new()
}

// --- shared `caps`/`support`/`query` print+exit tail (issue #229) ----------
//
// `hub_cmd.rs`'s and `body_cmd.rs`'s six leaves (`caps`, `support`, `query`
// on each side) all end the same way: on success, print the raw JSON to
// stdout with `--json`, or a short human summary otherwise, and exit 0; on
// a refusal, print `error: <message>` to stderr and exit with the refusal's
// code. That tail — previously two near-identical helpers,
// `print_control_doc`/`control_error_exit` for the hub side and
// `print_query_doc` for the body side — is expressed once here. What
// legitimately differs between the two sides (a control-socket round trip
// with `ControlError` vs a local, always-fallible-only-on-serialize lookup)
// stays in each side's own module, which builds a [`FetchOutcome`] from its
// own result type and hands it to [`print_and_exit_code`].
//
// The already-serialized JSON text (not a re-parsed `serde_json::Value`) is
// carried in `Doc` on purpose: `holler_body`'s query documents are plain
// structs, and round-tripping one through `Value` would risk reordering its
// object keys (a `Value` map is unordered without the `preserve_order`
// feature) and changing the exact bytes `--json` prints.
pub enum FetchOutcome {
    /// A document to report: `json_text` verbatim on `--json`, else `human`.
    Doc { json_text: String, human: String },
    /// A refusal: `message` to stderr (prefixed `error: `), then `exit_code`.
    Err { message: String, exit_code: i32 },
}

/// Print a [`FetchOutcome`] per ADR 0003 and return the exit code the caller
/// (`main.rs`, the one file allowed to end the process) should apply.
pub fn print_and_exit_code(json: bool, outcome: FetchOutcome) -> i32 {
    match outcome {
        FetchOutcome::Doc { json_text, human } => {
            if json {
                println!("{json_text}");
            } else {
                println!("{human}");
            }
            0
        }
        FetchOutcome::Err { message, exit_code } => {
            eprintln!("error: {message}");
            exit_code
        }
    }
}
