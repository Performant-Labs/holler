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
//!
//! This file is the *only* place allowed to call `std::process::exit`
//! (issue #147 §2) and is kept under `scripts/lint.sh`'s 900-line file-size
//! gate (issue #228) by pushing every leaf's actual logic into a sibling
//! module — `token_cmd.rs` (`hub token …`), `hub_cmd.rs` (`hub
//! status|caps|support|query`), `body_cmd.rs` (`body
//! join|detach|status|run|caps|support|query`), plus the pre-existing
//! `say_cmd.rs`/`roster_cmd.rs`/`query_cmd.rs`. Every one of those modules
//! returns a plain ADR 0003 exit code (already having printed its own
//! stdout/stderr output); this file's only job is to call the right one and
//! apply it.

use clap::Parser;
use holler_cli::{
    Attach, AttachCommand, BodyCommand, Cli, Command, HubCommand, TokenCommand,
};
use holler_proto::log::{emit_banner, init, resolve};

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
                std::process::exit(holler_cli::hub_cmd::status(cli.json));
            }
            // Story #163: the `hub token` leaf operates the hub's token store
            // (the file-backed store behind the loopback listener). Each verb
            // runs the store's blocking `flock` operation directly — the CLI is
            // not a tokio runtime, so there is no executor to starve (the
            // store's `spawn_blocking` wrappers are for the serve path).
            HubCommand::Token(token) => {
                let code = match &token.command {
                    TokenCommand::Mint(mint) => holler_cli::token_cmd::mint(mint, cli.json),
                    TokenCommand::List(_) => holler_cli::token_cmd::list(cli.json),
                    TokenCommand::Delete(del) => holler_cli::token_cmd::delete(&del.id, cli.json),
                    TokenCommand::Revoke(rev) => holler_cli::token_cmd::revoke(&rev.id, cli.json),
                    TokenCommand::Ping(ping) => holler_cli::token_cmd::ping(&ping.id, cli.json),
                };
                std::process::exit(code);
            }
            HubCommand::Caps(_) => std::process::exit(holler_cli::hub_cmd::caps(cli.json)),
            HubCommand::Support(support) => {
                std::process::exit(holler_cli::hub_cmd::support(&support.feature, cli.json));
            }
            HubCommand::Query(query) => std::process::exit(holler_cli::hub_cmd::query(query, cli.json)),
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
            BodyCommand::Join(join) => {
                std::process::exit(holler_cli::body_cmd::join(&join.server, &join.token));
            }
            BodyCommand::Detach(_) => std::process::exit(holler_cli::body_cmd::detach()),
            BodyCommand::Status(_) => std::process::exit(holler_cli::body_cmd::status(cli.json)),
            BodyCommand::Run(run) => std::process::exit(holler_cli::body_cmd::run(run.config.as_deref())),
            BodyCommand::Caps(_) => std::process::exit(holler_cli::body_cmd::caps(cli.json)),
            BodyCommand::Support(support) => {
                std::process::exit(holler_cli::body_cmd::support(&support.feature, cli.json));
            }
            BodyCommand::Query(query) => std::process::exit(holler_cli::body_cmd::query(query, cli.json)),
            BodyCommand::Attach(Attach { command }) => match command {
                AttachCommand::Sessions(_) => not_implemented("BodyAttachSessions"),
                AttachCommand::Init(_) => not_implemented("BodyAttachInit"),
            },
        }
    }

    if let Command::Roster(roster) = &cli.command {
        let result = holler_cli::roster_cmd::run(roster, cli.json);
        if !result.message.is_empty() {
            if result.to_stderr {
                eprintln!("error: {}", result.message);
            } else {
                print!("{}", result.message);
            }
        }
        std::process::exit(result.exit_code);
    }
    if let Command::Say(say) = &cli.command {
        let result = holler_cli::say_cmd::run(say, cli.json);
        if !result.message.is_empty() {
            if result.to_stderr {
                eprintln!("error: {}", result.message);
            } else {
                println!("{}", result.message);
            }
        }
        std::process::exit(result.exit_code);
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
