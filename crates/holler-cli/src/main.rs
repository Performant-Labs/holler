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
    Attach, AttachCommand, BodyCommand, Cli, Command, HubCommand, TokenCommand,
};
use holler_proto::log::{emit_banner, init, resolve};

/// ADR 0003: every unimplemented leaf exits 1 with this shape on stderr.
fn not_implemented(story: &str) -> ! {
    eprintln!("error: not implemented (story {story})");
    std::process::exit(1);
}

fn main() {
    // ADR 0003: a bare `holler` (no args) fails closed — exit 2, non-empty
    // usage on stderr. Detected up front, before parsing, so a bare
    // invocation never reaches the subcommand dispatch below.
    if std::env::args().count() == 1 {
        eprintln!(
            "error: a subcommand is required (hub | body | roster | say | interrupt)\n\nUsage: holler [OPTIONS] <COMMAND>"
        );
        std::process::exit(2);
    }

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

    let story = match &cli.command {
        // hub
        Command::Hub(hub) => match &hub.command {
            HubCommand::Serve(_) => "HubServing",
            HubCommand::Token(token) => match &token.command {
                TokenCommand::Mint(_) => "TokenMint",
                TokenCommand::List(_) => "TokenList",
                TokenCommand::Delete(_) => "TokenDelete",
                TokenCommand::Revoke(_) => "TokenRevoke",
                TokenCommand::Ping(_) => "TokenPing",
            },
            HubCommand::Status(_) => "HubStatus",
            HubCommand::Caps(_) => "HubCaps",
            HubCommand::Support(_) => "HubSupport",
            HubCommand::Query(_) => "HubQuery",
        },
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
