//! Entry point for the `holler` binary.
//!
//! Parses the full ADR 0003 CLI surface (see `cli.rs`), then routes each
//! leaf command to a "not implemented" handler. Behaviour is intentionally
//! inert in the workspace skeleton (story #327): every leaf prints
//! `error: not implemented (story <name>)` to stderr and exits 1, while
//! `--version`/`--help`/bare/unknown follow the ADR 0003 exit-code rules.

use clap::Parser;
use holler_cli::{
    Attach, AttachCommand, BodyCommand, Cli, Command, HubCommand, TokenCommand,
};

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
