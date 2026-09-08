//! The clap derive tree — the normative CLI surface from ADR 0003.
//!
//! Rules from ADR 0003 this encodes:
//! - Global flags (`--debug`, `--log-format`, `--json`) are on the root and
//!   therefore reachable on every subcommand. `--debug`/`--log-format` are
//!   parsed but unused until the logging story.
//! - `say`, `interrupt`, `roster` are top-level (hub-only daily verbs);
//!   everything else is namespaced under `hub` or `body`.
//! - No aliases are added beyond the ones ADR 0003 names (`rm`/`remove`).
//!   In particular `body status` is `status` only (the `st` alias is a
//!   rejected spelling).

use clap::{ArgAction, Parser, Subcommand};

/// `holler` — one binary, two roles (hub and body). ADR 0001.
#[derive(Parser, Debug)]
#[command(
    name = "holler",
    about = "One binary, two roles: hub and body (ADR 0001)",
    version,
)]
pub struct Cli {
    /// Set logging verbosity (none|quiet|noisy). Overrides `HOLLER_DEBUG`.
    ///
    /// Captured as an `Option` with an **empty** default (not `None`) so the
    /// resolver can tell "the flag was given" (`Some("noisy")` beats the env)
    /// from "absent" (clap fills `Some("")`, which `resolve` treats as
    /// "fall through to env, then the `none` default"). `resolve` fails closed
    /// (exit 3) on a non-empty value outside none|quiet|noisy.
    #[arg(
        long,
        global = true,
        action = ArgAction::Set,
        num_args = 1,
        default_value = "",
        hide_default_value = true,
    )]
    pub debug: Option<String>,

    /// Output format for logs (text|json). Overrides `HOLLER_LOG_FORMAT`.
    ///
    /// Same empty-default `Option` capture as `debug`: an absent flag arrives
    /// as `Some("")` (fall through to env, then the `text` default); a
    /// non-empty value outside text|json fails closed (exit 3) in `resolve`.
    #[arg(
        long,
        global = true,
        action = ArgAction::Set,
        num_args = 1,
        default_value = "",
        hide_default_value = true,
    )]
    pub log_format: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Operate a hub (serve, tokens, status, caps, support, query).
    Hub(Hub),
    /// Operate a body (join, run, detach, status, caps, support, query, attach).
    Body(Body),
    /// List the roster. (hub-only, top-level daily verb)
    Roster(Roster),
    /// Send a one-shot prompt to a session and print the reply. (hub-only)
    Say(Say),
    /// Interrupt the session's in-flight turn. (hub-only)
    Interrupt(Interrupt),
}

#[derive(Subcommand, Debug)]
pub enum HubCommand {
    /// Run the hub, accepting bodies and routing prompts.
    Serve(Serve),
    /// Manage join tokens.
    Token(Token),
    /// Show hub status.
    Status(Status),
    /// Show hub capabilities.
    Caps(Caps),
    /// Report whether a feature is supported.
    Support(Support),
    /// Run a command against a hub (local, or a remote one by TARGET).
    Query(Query),
}

#[derive(Parser, Debug)]
pub struct Hub {
    #[command(subcommand)]
    pub command: HubCommand,
}

#[derive(Subcommand, Debug)]
pub enum BodyCommand {
    /// Join a hub as a body using a minted token.
    Join(Join),
    /// Run the body (live agent session).
    Run(Run),
    /// Detach the body from the hub.
    Detach(Detach),
    /// Show body status.
    Status(Status),
    /// Show body capabilities.
    Caps(Caps),
    /// Report whether a feature is supported.
    Support(Support),
    /// Run a command against the body.
    Query(Query),
    /// Manage attached sessions.
    Attach(Attach),
}

#[derive(Parser, Debug)]
pub struct Body {
    #[command(subcommand)]
    pub command: BodyCommand,
}

// --- hub ---------------------------------------------------------------

#[derive(Parser, Debug)]
pub struct Serve {
    /// Address (host:port) to listen on. May be given more than once.
    #[arg(long)]
    pub listen: Vec<String>,
    /// Address (host[:port]) to advertise to bodies.
    #[arg(long)]
    pub advertise: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Token {
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
    #[command(subcommand)]
    pub command: TokenCommand,
}

#[derive(Subcommand, Debug)]
pub enum TokenCommand {
    /// Mint a new join token for LABEL.
    Mint(Mint),
    /// List join tokens and their state.
    List(List),
    /// Invalidate an unused token's secret (aliases: rm, remove).
    Delete(Delete),
    /// Cut a bound token's body's access now, keeping its record.
    Revoke(Revoke),
    /// Check whether a token is still valid.
    Ping(Ping),
}

#[derive(Parser, Debug)]
pub struct Mint {
    /// Label the token is minted for.
    #[arg(long)]
    pub label: String,
    /// Token time-to-live (default 24h).
    #[arg(long, default_value = "24h")]
    pub ttl: String,
}

#[derive(Parser, Debug)]
pub struct List {}

#[derive(Parser, Debug)]
#[command(alias = "rm", alias = "remove")]
pub struct Delete {
    /// Token id.
    pub id: String,
}

#[derive(Parser, Debug)]
pub struct Revoke {
    /// Token id.
    pub id: String,
}

#[derive(Parser, Debug)]
pub struct Ping {
    /// Token id.
    pub id: String,
}

#[derive(Parser, Debug)]
pub struct Status {
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct Caps {
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct Support {
    /// Feature name to query.
    pub feature: String,
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

/// `query CMD [ARGS...]` (local) or `query TARGET CMD [ARGS...]` (remote).
///
/// ADR 0003 gives `query` a variadic tail whose first token is either the
/// remote TARGET (remote form) or CMD (local form) — two shapes the clap
/// derive tree can't express as fixed positionals (a `trailing_var_arg` after
/// a required positional is rejected at parse time). So the tail is captured
/// as one variadic and split in code: `target = rest.first()`,
/// `cmd = rest.get(1)`, `args = rest[2..]`.
#[derive(Parser, Debug)]
pub struct Query {
    /// The query tail: CMD [ARGS...] (local) or TARGET CMD [ARGS...] (remote).
    #[arg(required = true, trailing_var_arg = true)]
    pub rest: Vec<String>,
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

impl Query {
    /// The remote target (first token), if present — `None` for the local
    /// form. See the struct doc for the two-shape rationale.
    pub fn target(&self) -> Option<&str> {
        self.rest.first().map(String::as_ref)
    }

    /// The command (second token in the remote form, first in the local form).
    pub fn cmd(&self) -> Option<&str> {
        self.rest.get(1).map(String::as_ref)
    }

    /// Any arguments following the command.
    pub fn args(&self) -> &[String] {
        match self.rest.get(2..) {
            Some(tail) => tail,
            None => &[],
        }
    }
}

// --- top-level (hub-only daily verbs) ----------------------------------

#[derive(Parser, Debug)]
pub struct Roster {
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct Say {
    /// Session address, <label>/<session> (or a bare <session>).
    pub session: String,
    /// Prompt text.
    pub text: String,
    /// Wait up to this long for the reply (default 600s).
    #[arg(long, default_value = "600s")]
    pub timeout: String,
    /// Print machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct Interrupt {
    /// Session address, <label>/<session> (or a bare <session>).
    pub session: String,
}

// --- body ----------------------------------------------------------------

#[derive(Parser, Debug)]
pub struct Join {
    /// Hub WebSocket URL to join.
    #[arg(long)]
    pub server: String,
    /// Join token, as ID:SECRET.
    #[arg(long)]
    pub token: String,
}

#[derive(Parser, Debug)]
pub struct Run {
    /// Path to the body config.
    #[arg(long)]
    pub config: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Detach {}

#[derive(Parser, Debug)]
pub struct Attach {
    #[command(subcommand)]
    pub command: AttachCommand,
}

#[derive(Subcommand, Debug)]
pub enum AttachCommand {
    /// List the attached sessions.
    Sessions(Sessions),
    /// Initialise an attached session.
    Init(Init),
}

#[derive(Parser, Debug)]
pub struct Sessions {
    /// Hub endpoint URL.
    #[arg(long)]
    pub endpoint: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Init {
    /// Hub endpoint URL.
    #[arg(long)]
    pub endpoint: Option<String>,
    /// Session id.
    #[arg(long)]
    pub session: Option<String>,
    /// Session name.
    #[arg(long)]
    pub name: Option<String>,
    /// Write output to PATH.
    #[arg(long)]
    pub out: Option<String>,
    /// Overwrite an existing output file.
    #[arg(long)]
    pub force: bool,
}
