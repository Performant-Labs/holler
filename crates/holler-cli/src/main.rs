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
use holler_cli::{
    Attach, AttachCommand, BodyCommand, Cli, Cmd, Command, HubCommand, Mint, Query,
    QueryResolution, Roster, Say, TokenCommand,
};
use holler_cli::query_cmd::{body_local_configs, is_ambiguous, query_cmd_params};
use holler_cli::time_fmt::format_epoch;
use holler_proto::log::{emit_banner, init, resolve};
use holler_proto::TokenError;

/// Install rustls's process-global crypto provider for `wss://` (TLS)
/// transport, before any `ClientConfig`/`ServerConfig` is built.
///
/// rustls 0.23 picks its crypto from a process-global that must be installed
/// before the first use; with none set, a `wss://` connect fails ("no
/// process-level CryptoProvider available"). The workspace builds rustls with
/// its default `aws-lc-rs` provider (the same one `tokio-tungstenite`'s
/// `rustls-tls-*` features select), so the binary installs
/// `rustls::crypto::aws_lc_rs::default_provider` once, here, at start — before
/// the body's throwaway runtime in `join` connects, and before the hub's
/// listener. Installing twice is a no-op: `install_default` returns the
/// already-installed provider, which we discard. A `wss://` join with no
/// provider still fails closed (exit 1) in `holler_body::join`, so a
/// provider-less build can never downgrade to plaintext.
#[allow(clippy::let_and_return)] // #176
fn install_tls_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

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

// --- `holler hub caps|support|query` (issue #185) ---------------------------
//
// All three are control-socket round trips (like `hub status`): no live hub
// reachable is exit 1 with the spec's exact "no live holler hub reachable"
// message. `query TARGET …` additionally forwards over the target body's own
// live socket; a target that resolves to zero or more-than-one live body is
// distinguished from an ordinary refusal (see `hub_query`'s own doc).

/// `holler hub caps [--json]` (issue #185): the live hub's `query/caps`
/// document (status + a support answer for every known id).
fn hub_caps(json: bool) -> ! {
    match holler_hub::control::caps() {
        Ok(doc) => {
            let n = doc.get("caps").and_then(|c| c.as_object()).map(|o| o.len()).unwrap_or(0);
            print_control_doc(json, doc, || format!("hub: {n} caps known"));
            std::process::exit(0);
        }
        Err(e) => control_error_exit(e),
    }
}

/// `holler hub support FEATURE [--json]` (issue #185): a single
/// `query/support` answer from the live hub.
fn hub_support(feature: &str, json: bool) -> ! {
    match holler_hub::control::support(feature) {
        Ok(doc) => {
            let ok = doc.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            print_control_doc(json, doc, || format!("{feature}: {}", if ok { "ok" } else { "not ok" }));
            std::process::exit(0);
        }
        Err(e) => control_error_exit(e),
    }
}

/// `holler hub query CMD [ARGS...]` (local) or `holler hub query TARGET CMD
/// [ARGS...]` (remote, forwarded to that body's live socket) — issue #185.
/// ADR 0003's exit codes: 0 on an answer, 1 when no live hub (or, for the
/// remote form, no live *body* matching TARGET) is reachable, 2 when TARGET
/// is ambiguous or the query tail itself is malformed.
fn hub_query(query: &Query, json: bool) -> ! {
    let resolved = match query.resolve() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let method = resolved.cmd().method();
    let params = query_cmd_params(resolved.cmd(), resolved.args());

    let result = match resolved.target() {
        None => holler_hub::control::query_local(method, params),
        Some(target) => holler_hub::control::query_remote(target.as_str(), method, params),
    };
    match result {
        Ok(doc) => {
            print_control_doc(json, doc, || format!("{method}: ok"));
            std::process::exit(0);
        }
        Err(holler_hub::control::ControlError::Refused(e)) if is_ambiguous(&e) => {
            eprintln!("error: {}", e.message);
            std::process::exit(2);
        }
        Err(e) => control_error_exit(e),
    }
}

/// Print a control-socket document per ADR 0003: `--json` prints only the raw
/// JSON to stdout; without it, `human()`'s short summary does.
fn print_control_doc(json: bool, doc: serde_json::Value, human: impl FnOnce() -> String) {
    if json {
        println!("{doc}");
    } else {
        println!("{}", human());
    }
}

/// The shared exit code for a [`holler_hub::control::ControlError`]: no live
/// hub reachable is exit 1 (the spec's exact message); every other refusal
/// (not connected, `-32004`) is exit 1 — only an ambiguous TARGET is exit 2.
fn control_error_exit(e: holler_hub::control::ControlError) -> ! {
    match e {
        holler_hub::control::ControlError::NoLiveHub => {
            let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
            eprintln!("error: no live holler hub reachable at {}", state_root.display());
        }
        other => eprintln!("error: {other}"),
    }
    std::process::exit(1);
}

/// `holler say` (issue #190) — thin wrapper: [`holler_cli::say_cmd::run`]
/// carries the logic (kept out of `main.rs`'s 900-line budget, same split as
/// `query_cmd.rs`/#185); this is the one place allowed to exit on it.
fn say_command(say: &Say, json: bool) -> ! {
    let result = holler_cli::say_cmd::run(say, json);
    if !result.message.is_empty() {
        if result.to_stderr {
            eprintln!("error: {}", result.message);
        } else {
            println!("{}", result.message);
        }
    }
    std::process::exit(result.exit_code);
}

/// `holler roster` (issue #186) — thin wrapper: [`holler_cli::roster_cmd::run`]
/// carries the logic (kept out of `main.rs`'s 900-line budget, like
/// `say_cmd.rs`); this is the one place allowed to exit on it.
fn roster_command(roster: &Roster, json: bool) -> ! {
    let result = holler_cli::roster_cmd::run(roster, json);
    if !result.message.is_empty() {
        if result.to_stderr {
            eprintln!("error: {}", result.message);
        } else {
            print!("{}", result.message);
        }
    }
    std::process::exit(result.exit_code);
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

/// `holler hub token ping ID` (issue #182): first the store's own record
/// check — a `revoked`/`expired`/`not_found` token can never be live, so
/// those stay a fail-closed policy refusal (exit 3) without ever touching the
/// live hub. An **unused** (not-yet-redeemed) token has never had a socket to
/// begin with, so it keeps the pre-#182 behaviour: `valid`, exit 0 (there is
/// nothing live to probe). Only a **bound** token is actually probed: the
/// control socket asks the live hub to send a `circuit/ping` over that
/// token's socket and report `{hostname, rtt_ms}`. No live socket for it —
/// including no live hub at all — is the spec's `-32004 not_connected`
/// (exit 1); a genuine answer is exit 0.
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
    let refusal = match record.state {
        holler_hub::token::TokenState::Revoked => Some("revoked"),
        holler_hub::token::TokenState::Unused if record.expires < now => Some("expired"),
        holler_hub::token::TokenState::Bound if record.expires < now => Some("expired"),
        holler_hub::token::TokenState::Unused => None, // never redeemed: nothing to probe, `valid`.
        holler_hub::token::TokenState::Bound => None,  // worth probing live.
    };
    if let Some(status) = refusal {
        if json {
            println!("{}", serde_json::json!({ "token_id": id, "label": record.label, "state": status }));
        } else {
            println!("{status} ({})", record.label);
        }
        std::process::exit(3);
    }
    if record.state == holler_hub::token::TokenState::Bound {
        token_ping_live(id, &record.label, json);
    }
    if json {
        println!("{}", serde_json::json!({ "token_id": id, "label": record.label, "state": "valid" }));
    } else {
        println!("valid ({})", record.label);
    }
    std::process::exit(0);
}

/// The live half of `hub token ping`: probe the hub's control socket. Split
/// out of [`token_ping`] to keep that dispatch's cognitive complexity under
/// the workspace threshold.
fn token_ping_live(id: &str, label: &str, json: bool) -> ! {
    match holler_hub::control::token_ping(id) {
        Ok(doc) => {
            let hostname = doc.get("hostname").and_then(|v| v.as_str()).unwrap_or("?");
            let rtt_ms = doc.get("rtt_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            if json {
                println!("{}", serde_json::json!({ "token_id": id, "label": label, "state": "valid", "hostname": hostname, "rtt_ms": rtt_ms }));
            } else {
                println!("valid ({label}, {hostname}) rtt={rtt_ms}ms");
            }
            std::process::exit(0);
        }
        Err(e) => {
            if json {
                println!("{}", serde_json::json!({ "token_id": id, "label": label, "state": "not_connected" }));
            }
            eprintln!("not_connected {id} ({e})");
            std::process::exit(1);
        }
    }
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
    // Install the rustls crypto provider before any I/O: a `wss://` (TLS)
    // WebSocket needs a process-global provider set up ahead of the first
    // connection, and the body's `join` runs on a throwaway runtime later in
    // this process. (No-op when the build has no `ring` provider; a `wss`
    // join then still fails closed in `holler_body::join`.)
    install_tls_provider();

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

    // Story #143: the hub leaves that are implemented — `hub serve` (the
    // loopback listener + control socket) and `hub status` (read the live hub's
    // status over the control socket). Every other leaf is still the
    // skeleton's "not implemented" (their stories land later).
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
            HubCommand::Caps(_) => hub_caps(cli.json),
            HubCommand::Support(support) => hub_support(&support.feature, cli.json),
            HubCommand::Query(query) => hub_query(query, cli.json),
        }
    }

    // Story #176: the body's identity leaves — `body join` (redeem a minted
    // join secret over the wire and persist the identity), `body detach`
    // (forget it), and `body status` (report this process's own identity).
    // Story #182 adds `body run` (the live connection loop) and makes `body
    // detach` live-aware (write `detach_request` and wait, when a run is
    // active, rather than only deleting the credential). The other body
    // leaves (caps, support, query, attach) are still inert.
    if let Command::Body(body) = &cli.command {
        match &body.command {
            BodyCommand::Join(join) => body_join(&join.server, &join.token),
            BodyCommand::Detach(_) => body_detach(),
            BodyCommand::Status(_) => body_status(cli.json),
            BodyCommand::Run(run) => body_run(run.config.as_deref()),
            BodyCommand::Caps(_) => body_caps(cli.json),
            BodyCommand::Support(support) => body_support(&support.feature, cli.json),
            BodyCommand::Query(query) => body_query(query, cli.json),
            BodyCommand::Attach(Attach { command }) => match command {
                AttachCommand::Sessions(_) => not_implemented("BodyAttachSessions"),
                AttachCommand::Init(_) => not_implemented("BodyAttachInit"),
            },
        }
    }

    if let Command::Roster(roster) = &cli.command {
        roster_command(roster, cli.json);
    }
    if let Command::Say(say) = &cli.command {
        say_command(say, cli.json);
    }

    let story = match &cli.command {
        // the implemented leaves were handled above and returned; these arms
        // are defensive — every path above exits.
        Command::Hub(_) => not_implemented("Hub"),
        Command::Roster(_) => not_implemented("Roster"),
        Command::Say(_) => not_implemented("Say"),
        Command::Interrupt(_) => "Interrupt",
        Command::Body(_) => not_implemented("Body"),
    };

    not_implemented(story);
}

// --- `holler body …` (story #176) ------------------------------------------
//
// The body's identity leaves. Each resolves the state dir (the body's
// credential lives under `<state>/body/`), drives the `holler_body` helpers,
// and applies the ADR 0003 exit code the helper returns (0 ok, 1 a runtime
// failure, 3 a fail-closed policy refusal). The CLI is not a tokio runtime —
// `holler_body::join` builds its own throwaway runtime for the one-shot wire
// handshake — so these run straight-line like the token verbs.

/// Resolve the state dir, or fail closed (exit 1) if there is nowhere for the
/// body's credential to live. (The e2e harness always sets `HOLLER_STATE_DIR`.)
fn body_state() -> holler_hub::state::HubState {
    match holler_hub::state::resolve_state_dir() {
        Some(root) => holler_hub::state::HubState::from_root(root),
        None => {
            // `resolve_state_dir` already printed the refusal to stderr.
            std::process::exit(1);
        }
    }
}

/// `holler body join --server <url> --token <ID:SECRET>` — redeem a minted
/// one-time join secret over the wire and persist the body's identity. The
/// helper prints its own one-line result (a stderr reason on a refusal; a
/// success line naming the `client_id`, never the secret) and returns the exit
/// code, which the bin applies here (ADR 0003: 0 joined, 1 a runtime failure
/// incl. a refused redeem, 3 a plaintext non-loopback `ws://`).
fn body_join(server: &str, token: &str) -> ! {
    let state = body_state();
    let exit = holler_body::join::join(&state.root, server, token, "default");
    let code = match exit {
        holler_body::join::JoinExit::Ok => 0,
        holler_body::join::JoinExit::Refused(_) => 1,
        holler_body::join::JoinExit::Policy => 3,
    };
    std::process::exit(code);
}

/// `holler body detach` — forget the body's identity: the no-run form
/// deletes `<state>/body/credential.json` outright; when a `body run` is
/// live (issue #182) it is asked to end the circuit first (see
/// `holler_body::detach`'s module doc). Idempotent: always exit 0 except an
/// I/O failure (exit 1).
fn body_detach() -> ! {
    let state = body_state();
    let code = match holler_body::detach::detach(&state.root) {
        holler_body::detach::DetachExit::Ok => 0,
        holler_body::detach::DetachExit::Io => 1,
    };
    std::process::exit(code);
}

/// `holler body run` (issue #182 connection loop; issue #187 wires the
/// session config in) — resolves the session config (`--config PATH` >
/// `HOLLER_CONFIG` > `./sessions.toml` > `./session.toml`; none found is a
/// fail-closed exit 3, per "every session is explicit"), builds the
/// in-process [`holler_body::registry::SessionRegistry`], and drives the live
/// connection loop: authenticate, exchange hellos, heartbeat (now carrying
/// every registered session, `idle`), survive drops with backoff, and honour
/// `detach`. Runs until a clean end (detach or a signal) or an unretryable
/// authentication failure.
fn body_run(config_flag: Option<&str>) -> ! {
    let state = body_state();
    // Not-joined (exit 1, `connection::run`'s own check) outranks a missing
    // config (exit 3): an unjoined body has nothing to run regardless of its
    // session list, and `body_run_unjoined_exits_1` pins that ordering — so
    // this is checked *before* config discovery, even though `connection::
    // run` re-checks it a moment later once it actually needs the identity.
    match holler_body::identity::load(&state.root) {
        None => {
            eprintln!("error: not joined; run `holler body join` first");
            std::process::exit(1);
        }
        Some(Err(e)) => {
            eprintln!("error: state dir: {e}");
            std::process::exit(1);
        }
        Some(Ok(_)) => {}
    }

    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot resolve the current directory: {e}");
            std::process::exit(1);
        }
    };
    let parsed = match holler_body::config::discover_and_load(config_flag, &cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.message());
            std::process::exit(3);
        }
    };
    for w in &parsed.warnings {
        eprintln!("warn: {w}");
    }
    let registry = holler_body::registry::SessionRegistry::from_sessions(parsed.sessions);

    // Issue #190: `connection::run` now owns the registry outright — it
    // starts a `SessionManager` over it (one task per session, for this
    // process's whole live lifetime), derives issue #185's `query/*`
    // config rows from the same registry internally, and drives every
    // `session/presence` from the manager's *live* state, not the
    // registry's own always-`idle` snapshot.
    let code = match holler_body::connection::run(&state.root, registry) {
        holler_body::connection::RunExit::Ok => 0,
        holler_body::connection::RunExit::NotJoined
        | holler_body::connection::RunExit::AuthFailed(_)
        | holler_body::connection::RunExit::Io(_) => 1,
        holler_body::connection::RunExit::LockHeld(_) => 3,
    };
    std::process::exit(code);
}

/// `holler body status [--json]` — report this process's own identity (local:
/// it reads the credential file, never a live hub). With `--json`, only the
/// status document goes to stdout; without, a short human summary does. An
/// unjoined body is a valid document (`joined: false`, exit 0), not an error;
/// only a state-dir I/O failure is exit 1.
fn body_status(json: bool) -> ! {
    let state = body_state();
    let (doc, exit) = holler_body::status::status(&state.root);
    let Some(doc) = doc else {
        // A state-dir I/O failure: the helper already printed the reason.
        std::process::exit(1);
    };
    if json {
        match serde_json::to_string(&doc) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("{}", holler_body::status::render_human(&doc));
    }
    std::process::exit(match exit {
        holler_body::status::StatusExit::Ok => 0,
        holler_body::status::StatusExit::Io => 1,
    });
}

// --- `holler body caps|support|query` (issue #185) --------------------------
//
// All three are **local**: they read this body's own persisted identity, the
// connection-state file, and the session config — the exact same local
// probes `crate::query`'s document builders answer a hub-forwarded
// `query/*` request with (see `holler_body::connection::handle_query`), so a
// live `body run` and a one-shot `body caps`/`support`/`query` agree by
// construction. `--json` prints only the raw document to stdout (ADR 0003);
// without it, a short human summary does.

/// Print a query document per ADR 0003: with `--json`, only the raw JSON
/// (already serialized by the caller) goes to stdout; without it, `human()`'s
/// short summary does. A serialization failure is defensive (every document
/// here derives `Serialize` over plain, always-encodable fields) but still
/// exits 1 rather than panicking.
fn print_query_doc(json: bool, encoded: Result<String, serde_json::Error>, human: impl FnOnce() -> String) {
    if json {
        match encoded {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("{}", human());
    }
}

/// `holler body caps [--json]` (issue #185): the local status doc plus a
/// support answer for every known feature/harness/capability id (docs §5.2).
fn body_caps(json: bool) -> ! {
    let state = body_state();
    let identity = holler_body::identity::load(&state.root).and_then(Result::ok);
    let configs = body_local_configs(&state.root);
    let doc = holler_body::query::local_caps(&state.root, identity.as_ref(), &configs);
    print_query_doc(json, serde_json::to_string(&doc), || {
        format!("body: {} caps known", doc.caps.len())
    });
    std::process::exit(0);
}

/// `holler body support FEATURE [--json]` (issue #185): a single
/// `query/support` answer. An id outside the v2 vocabulary is exit 1 (the
/// wire's `-32006 unknown_feature`, surfaced as a plain stderr reason here —
/// this leaf never touches the wire).
fn body_support(feature: &str, json: bool) -> ! {
    let state = body_state();
    let configs = body_local_configs(&state.root);
    match holler_body::query::local_support(feature, &configs) {
        Ok(doc) => {
            print_query_doc(json, serde_json::to_string(&doc), || {
                let reason = doc.reason.as_deref().map(|r| format!(" ({r})")).unwrap_or_default();
                format!("{feature}: {}{reason}", if doc.ok { "ok" } else { "not ok" })
            });
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {}", e.message);
            std::process::exit(1);
        }
    }
}

/// `holler body query CMD [ARGS...] [--json]` (issue #185): local only — a
/// body never forwards `query/*` to another peer (that is `hub query
/// TARGET …`'s job). A remote-form tail (`query TARGET CMD…`) is a usage
/// error (exit 2): a body has no routing table to resolve TARGET against.
fn body_query(query: &Query, json: bool) -> ! {
    let resolved = match query.resolve() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let QueryResolution::Local { cmd, args } = &resolved else {
        eprintln!("error: `body query` has no remote target — did you mean `hub query TARGET …`?");
        std::process::exit(2);
    };
    let state = body_state();
    let identity = holler_body::identity::load(&state.root).and_then(Result::ok);
    let configs = body_local_configs(&state.root);
    match cmd {
        Cmd::Status => {
            let doc = holler_body::query::local_status(&state.root, identity.as_ref(), &configs);
            print_query_doc(json, serde_json::to_string(&doc), || format!("body: role={:?}", doc.role));
        }
        Cmd::Caps => {
            let doc = holler_body::query::local_caps(&state.root, identity.as_ref(), &configs);
            print_query_doc(json, serde_json::to_string(&doc), || {
                format!("body: {} caps known", doc.caps.len())
            });
        }
        Cmd::Support => {
            let Some(feature) = args.first() else {
                eprintln!("error: `query support` needs a FEATURE argument");
                std::process::exit(2);
            };
            match holler_body::query::local_support(feature, &configs) {
                Ok(doc) => print_query_doc(json, serde_json::to_string(&doc), || format!("{feature}: {}", doc.ok)),
                Err(e) => {
                    eprintln!("error: {}", e.message);
                    std::process::exit(1);
                }
            }
        }
        Cmd::Protocol => {
            let version = args.first().and_then(|s| s.parse::<u32>().ok());
            let doc = holler_body::query::local_protocol(version);
            print_query_doc(json, serde_json::to_string(&doc), || {
                format!("protocol: session={} min={} max={}", doc.session, doc.min, doc.max)
            });
        }
    }
    std::process::exit(0);
}
