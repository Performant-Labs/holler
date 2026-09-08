//! Entry point for the `holler` binary.
//!
//! Parses the full ADR 0003 CLI surface (see `cli.rs`), resolves the logging
//! dials (`--debug`/`HOLLER_DEBUG`, `--log-format`/`HOLLER_LOG_FORMAT`), and
//! routes each leaf command to a handler. Behaviour is intentionally inert in
//! the workspace skeleton (story #127): every leaf prints
//! `error: not implemented (story <name>)` to stderr and exits 1, while
//! `--version`/`--help`/bare/unknown follow the ADR 0003 exit-code rules.
//!
//! Logging (story #144): the resolved settings are installed once and the
//! startup banner (`logging_started`) is emitted **before any blocking I/O**,
//! so the level/format are visible even when later work hangs. An
//! unrecognised value is a fail-closed policy refusal (exit 3), never a
//! silent fallback. All logging goes to stderr; stdout is command output.

use clap::Parser;
use holler_cli::{Attach, AttachCommand, BodyCommand, Cli, Command, HubCommand, Mint, TokenCommand};
use holler_proto::log::{emit_banner, init, resolve};
use holler_proto::TokenError;

/// ADR 0003: every unimplemented leaf exits 1 with this shape on stderr.
fn not_implemented(story: &str) -> ! {
    eprintln!("error: not implemented (story {story})");
    std::process::exit(1);
}

/// `holler hub status [--json]` — read the live hub's status document over the
/// control socket and print it (story #143). With `--json`, the raw
/// `StatusDoc` JSON goes to stdout (machine-readable). Without, a short
/// human-readable summary goes to stdout. If no live hub is reachable, the
/// spec's exact message goes to stderr and the exit code is 1.
fn hub_status(json: bool) -> ! {
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
            std::process::exit(0);
        }
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            eprintln!(
                "error: no live holler hub reachable at {}",
                state_root.display()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

// --- `holler hub token …` (story #163) --------------------------------------
//
// Each verb runs the token store's blocking `flock` operation. The CLI is not
// a tokio runtime, so these call the store's sync fns directly (the
// `spawn_blocking` wrappers exist for the serve path, not here). Every verb
// exits: `0` on success (stdout), `1` on an I/O failure, `3` on a fail-closed
// policy refusal (ADR 0003). The state dir is resolved from the environment;
// an unresolvable dir is the store's own `TokenError` (printed, exit 1).

/// Resolve this invocation's state dir, or fail closed (exit 1) if there is
/// nowhere to put the token store.
fn token_state() -> holler_hub::state::HubState {
    match holler_hub::state::resolve_state_dir() {
        Some(root) => holler_hub::state::HubState::from_root(root),
        None => {
            // `resolve_state_dir` already printed the refusal to stderr.
            std::process::exit(1);
        }
    }
}

/// The one shared error-exit for the token verbs: print the store's message to
/// stderr and exit 1 (a store refusal such as an I/O failure). The fail-closed
/// *policy* refusals (duplicate label, unknown id) are mapped to exit 3 at the
/// call sites — see `token_delete`/`token_mint` — because those are the spec's
/// "exit 3" cases.
fn token_exit(e: TokenError) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}

/// Format a unix epoch second as a local-time `YYYY-MM-DD HH:MM:SS` string
/// (the `list`/`ping` EXPIRES column). No chrono dependency: a manual civil
/// calendar conversion (Howard Hinnant's algorithm) is enough for epoch seconds.
fn format_epoch(secs: u64) -> String {
    let days = (secs as i64) / 86400;
    let rem = secs as i64 % 86400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Days from 1970-01-01 → year, month, day (Hinnant's civil-from-days).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 3) / 153;
    let day = doy - (153 * mp + 2) / 5;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
    )
}

/// `holler hub token mint --label L [--ttl 24h] [--json]` — mint a join token
/// and hand the operator the one-time secret. Human output prints `token_id`,
/// `secret`, `expires`, and a ready-to-paste `holler body join` line; `--json`
/// prints `{token_id, secret, expires, join_command}`. A duplicate label or an
/// invalid label is a fail-closed refusal (exit 3); an I/O failure is exit 1.
fn token_mint(mint: &Mint, json: bool) -> ! {
    let state = token_state();
    let ttl_secs = match parse_ttl(&mint.ttl) {
        Some(t) => t,
        None => {
            eprintln!("error: invalid --ttl {:?} (use e.g. 15m, 2h, 24h)", mint.ttl);
            std::process::exit(3);
        }
    };
    let minted = match holler_hub::token::mint(&mint.label, ttl_secs, &state) {
        Ok(m) => m,
        Err(e) => {
            // A duplicate label or a malformed label is a policy refusal
            // (exit 3); any other store error (I/O) is a runtime failure (1).
            if e.message.contains("label") {
                eprintln!("error: {e}");
                std::process::exit(3);
            }
            token_exit(e);
        }
    };
    let join_command = join_command(&state, &minted.record.token_id, &minted.secret);
    if json {
        let doc = serde_json::json!({
            "token_id": minted.record.token_id,
            "secret": minted.secret,
            "expires": minted.record.expires,
            "join_command": join_command,
        });
        println!("{}", doc);
    } else {
        println!("token_id:    {}", minted.record.token_id);
        println!("secret:      {}   (shown once; not stored)", minted.secret);
        println!("expires:     {}", format_epoch(minted.record.expires));
        println!();
        println!("  {join_command}");
    }
    std::process::exit(0);
}

/// `holler hub token list [--json]` — list every token. Human output prints a
/// `TOKEN_ID LABEL STATE MACHINE LAST_SEEN EXPIRES` table; secrets are never
/// printed. `--json` prints a `{"tokens":[{token_id,label,state,machine,
/// last_seen,expires}...]}` document (machine = hostname, last_seen/expires as
/// epoch seconds).
fn token_list(json: bool) -> ! {
    let state = token_state();
    let rows = match holler_hub::token::list(&state) {
        Ok(r) => r,
        Err(e) => token_exit(e),
    };
    if json {
        let toks: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "token_id": r.token_id,
                    "label": r.label,
                    "state": holler_hub::token::state_str(r.state),
                    "machine": r.hostname,
                    "last_seen": r.last_seen,
                    "expires": r.expires,
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "tokens": toks }));
    } else {
        println!(
            "{:<14} {:<10} {:<8} {:<10} {:<10} EXPIRES",
            "TOKEN_ID", "LABEL", "STATE", "MACHINE", "LAST_SEEN"
        );
        for r in &rows {
            let machine = r.hostname.as_deref().unwrap_or("-");
            let last_seen = r.last_seen.map(format_epoch).unwrap_or_else(|| "-".into());
            println!(
                "{:<14} {:<10} {:<8} {:<10} {:<10} {}",
                r.token_id,
                r.label,
                holler_hub::token::state_str(r.state),
                machine,
                last_seen,
                format_epoch(r.expires)
            );
        }
    }
    std::process::exit(0);
}

/// `holler hub token delete ID` (aliases `rm`/`remove`) and
/// `holler hub token revoke ID` share one operation (the store's
/// `delete`): an `unused` token is invalidated (secret void), a `bound` token
/// is revoked (credential void, **row kept**). Both print the spec's shape:
/// `revoked <id> (<label>, <hostname>)` or `invalidated <id> (<label>, unused)`.
/// An unknown id is a fail-closed refusal (exit 3); an I/O failure is exit 1.
fn token_delete(id: &str, json: bool) -> ! {
    run_token_inactivate(id, "delete", json);
}

fn token_revoke(id: &str, json: bool) -> ! {
    run_token_inactivate(id, "revoke", json);
}

/// The shared delete/revoke body (the store treats them identically). `verb`
/// is only used in the `--json` document (so a machine can tell which spelling
/// was used); the human line's verb word comes from the record's pre-state via
/// `word` (unused→"invalidated", bound→"revoked").
fn run_token_inactivate(id: &str, verb: &str, json: bool) -> ! {
    let state = token_state();
    // A bound token is `revoked`; an unused one is `invalidated`. Peek the
    // pre-state to pick the word (the store flips both to `revoked`).
    let was_bound = matches!(
        holler_hub::token::list(&state),
        Ok(rows) if rows.iter().any(|r| r.token_id == id && r.state == holler_hub::token::TokenState::Bound)
    );
    let record = match holler_hub::token::delete(id, &state) {
        Ok(r) => r,
        Err(e) => {
            // An unknown id is a policy refusal (exit 3); an I/O failure is 1.
            if e.message.contains("no such token") {
                eprintln!("error: {e}");
                std::process::exit(3);
            }
            token_exit(e);
        }
    };
    let word = if was_bound { "revoked" } else { "invalidated" };
    let suffix = match &record.hostname {
        Some(host) => format!("{}, {host}", record.label),
        None => format!("{}, unused", record.label),
    };
    if json {
        let doc = serde_json::json!({
            "token_id": record.token_id,
            "label": record.label,
            "state": "revoked",
            "hostname": record.hostname,
            "verb": verb,
        });
        println!("{}", doc);
    } else {
        println!("{word} {} ({suffix})", record.token_id);
    }
    std::process::exit(0);
}

/// `holler hub token ping ID` — the spec leaves `ping` to the **registry**
/// story (it needs a live socket). Here it checks the store's record and
/// reports `valid` / `expired` / `revoked` / `not_found` cleanly; a
/// `revoked` or `not_found` token is a fail-closed refusal (exit 3), and a
/// missing hub is exit 1.
fn token_ping(id: &str, json: bool) -> ! {
    let state = token_state();
    let rows = match holler_hub::token::list(&state) {
        Ok(r) => r,
        Err(e) => token_exit(e),
    };
    let Some(record) = rows.iter().find(|r| r.token_id == id) else {
        if json {
            println!("{}", serde_json::json!({ "token_id": id, "state": "not_found" }));
        }
        eprintln!("not_found {id} (no such token)");
        std::process::exit(3);
    };
    let now = holler_hub::token::now_secs();
    let status = match record.state {
        holler_hub::token::TokenState::Revoked => "revoked",
        holler_hub::token::TokenState::Unused if record.expires < now => "expired",
        holler_hub::token::TokenState::Bound if record.expires < now => "expired",
        _ => "valid",
    };
    if json {
        println!("{}", serde_json::json!({ "token_id": id, "label": record.label, "state": status }));
    } else {
        let suffix = match (&record.hostname, record.state) {
            (Some(host), holler_hub::token::TokenState::Bound) => format!(" ({}, {})", record.label, host),
            _ => format!(" ({})", record.label),
        };
        println!("{status}{suffix}");
    }
    if status == "valid" {
        std::process::exit(0);
    }
    std::process::exit(3);
}

/// Parse a `--ttl` like `15m` / `2h` / `24h` into seconds, or `None` if the
/// spelling is not a `<number><m|h|d>` duration (the spec's TTL grammar). The
/// number is the leading run of digits; the unit is the single trailing
/// character. A leading `+`/`-`, an empty number, or a missing/unknown unit
/// all refuse (fail closed at the call site with exit 3).
fn parse_ttl(ttl: &str) -> Option<u64> {
    let digits: &str = ttl.split(|c: char| !c.is_ascii_digit()).next()?;
    if digits.is_empty() {
        return None;
    }
    let rest = &ttl[digits.len()..];
    if rest.len() != 1 {
        return None;
    }
    let num: u64 = digits.parse().ok()?;
    match rest {
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 86400),
        _ => None,
    }
}

/// The ready-to-paste `holler body join --server <adv> --token <id>:<secret>`
/// line `mint` prints (and `--json`'s `join_command`). The server is the
/// hub's persisted advertise address if `hub serve --advertise` set one, else
/// the spec's loopback default `ws://127.0.0.1:41807`.
fn join_command(state: &holler_hub::state::HubState, token_id: &str, secret: &str) -> String {
    let server = std::fs::read_to_string(holler_hub::state::advertise_path(state))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("advertise").and_then(|a| a.as_str()).map(String::from))
        .unwrap_or_else(|| "ws://127.0.0.1:41807".into());
    format!("holler body join --server {server} --token {token_id}:{secret}")
}

fn main() {
    // ADR 0003: a bare `holler` (or `holler hub`, an unknown subcommand, …)
    // fails closed with exit 2 and a usage message on stderr. That is clap's
    // own behaviour now — the root parser is `subcommand_required = true`, so
    // there is no pre-parse branch to special-case the empty argv (issue #148).
    let cli = Cli::parse();

    // Resolve the logging dials: the CLI flag beats the environment, the
    // defaults are none/text, and an unrecognised value fails closed (exit 3)
    // *before* the banner — the banner itself reports the resolved settings.
    // (The env values are read here, not in `resolve`, so the proto core stays
    // free of any OS dependency.)
    let env_debug = std::env::var("HOLLER_DEBUG").ok();
    let env_format = std::env::var("HOLLER_LOG_FORMAT").ok();
    let config = match resolve(
        cli.debug.as_deref(),
        cli.log_format.as_deref(),
        env_debug.as_deref(),
        env_format.as_deref(),
    ) {
        Ok(c) => c,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(3);
        }
    };

    // Install the config for any code that emits events later, then emit the
    // banner. The banner is the first stderr write and precedes any blocking
    // I/O; it reports the settings just installed.
    init(config);
    emit_banner();

    // Story #143: the two hub leaves that are implemented — `hub serve`
    // (the loopback listener + control socket) and `hub status` (read the
    // live hub's status over the control socket). Every other leaf is still
    // the skeleton's "not implemented" (their stories land later).
    if let Command::Hub(hub) = &cli.command {
        match &hub.command {
            HubCommand::Serve(serve) => {
                // `run` is a lib and returns the exit code (ADR 0003: 0 clean
                // signal shutdown, 1 runtime failure, 3 fail-closed refusal).
                // The bin — the one place a helper's code may be acted on by
                // exiting — applies it. A clean 0 would otherwise fall through
                // to the "not implemented" arm below, so we exit in all cases.
                std::process::exit(holler_hub::serve::run(
                    &serve.listen,
                    serve.advertise.as_deref(),
                ));
            }
            HubCommand::Status(_) => {
                // `--json` is the root-level global flag (issue #147/#155), not
                // a per-leaf field: `Status` is empty, so read `cli.json`.
                hub_status(cli.json);
            }
            // Story #163: the `hub token` leaf operates the hub's token store
            // (the file-backed store behind the loopback listener). Each verb
            // runs the store's blocking `flock` operation directly — the CLI is
            // not a tokio runtime, so there is no executor to starve (the
            // store's `spawn_blocking` wrappers are for the serve path).
            HubCommand::Token(token) => match &token.command {
                TokenCommand::Mint(mint) => token_mint(mint, cli.json),
                TokenCommand::List(_) => token_list(cli.json),
                TokenCommand::Delete(del) => token_delete(&del.id, cli.json),
                TokenCommand::Revoke(rev) => token_revoke(&rev.id, cli.json),
                TokenCommand::Ping(ping) => token_ping(&ping.id, cli.json),
            },
            HubCommand::Caps(_) => not_implemented("HubCaps"),
            HubCommand::Support(_) => not_implemented("HubSupport"),
            HubCommand::Query(_) => not_implemented("HubQuery"),
        }
    }

    let story = match &cli.command {
        // hub (the implemented Serve/Status were handled above and returned;
        // this arm is defensive — every `Command::Hub` path above exits)
        Command::Hub(_) => not_implemented("Hub"),
        // top-level (hub-only daily verbs)
        Command::Roster(_) => "Roster",
        Command::Say(_) => "Say",
        Command::Interrupt(_) => "Interrupt",
        // body
        Command::Body(body) => match &body.command {
            BodyCommand::Join(_) => "BodyJoin",
            BodyCommand::Run(_) => "BodyRun",
            BodyCommand::Detach(_) => "BodyDetach",
            BodyCommand::Status(_) => "BodyStatus",
            BodyCommand::Caps(_) => "BodyCaps",
            BodyCommand::Support(_) => "BodySupport",
            BodyCommand::Query(_) => "BodyQuery",
            BodyCommand::Attach(Attach { command }) => match command {
                AttachCommand::Sessions(_) => "BodyAttachSessions",
                AttachCommand::Init(_) => "BodyAttachInit",
            },
        },
    };

    not_implemented(story);
}
