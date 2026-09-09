//! The clap derive tree — the normative CLI surface from ADR 0003.
//!
//! Rules from ADR 0003 this encodes:
//! - Global flags (`--debug`, `--log-format`, `--json`) are on the root and
//!   therefore reachable on every subcommand. `--debug`/`--log-format` are
//!   parsed but unused until the logging story.
//! - `say`, `interrupt`, `answer`, `roster` are top-level (hub-only daily
//!   verbs);
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
    subcommand_required = true
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

    /// Print machine-readable output.
    ///
    /// Global (issue #147/#155): reachable after any leaf, including
    /// `hub token mint --label x --json` — which the per-struct copies this
    /// replaced could not parse (the flag sat on `Token`, before the leaf).
    #[arg(long, global = true)]
    pub json: bool,

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
    /// Resolve a held permission/elicitation. (hub-only)
    Answer(Answer),
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

// NOTE: every leaf's `--json` field was removed here — `--json` is now a
// single global flag on `Cli` (ADR 0003), so `status`/`caps`/`support`/`query`
// under `body` share the root's `--json` rather than each declaring their own.

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
pub struct Status {}

#[derive(Parser, Debug)]
pub struct Caps {}

#[derive(Parser, Debug)]
pub struct Support {
    /// Feature name to query.
    pub feature: String,
}

/// `query CMD [ARGS...]` (local) or `query TARGET CMD [ARGS...]` (remote).
///
/// ADR 0003 gives `query` a variadic tail whose first token is either the
/// remote TARGET (remote form) or CMD (local form) — two shapes the clap
/// derive tree can't express as fixed positionals (a `trailing_var_arg` after
/// a required positional is rejected at parse time). So the tail is captured
/// as one variadic and split in code by [`Query::resolve`].
#[derive(Parser, Debug)]
pub struct Query {
    /// The query tail: CMD [ARGS...] (local) or TARGET CMD [ARGS...] (remote).
    #[arg(required = true, trailing_var_arg = true)]
    pub rest: Vec<String>,
}

impl Query {
    /// Disambiguate the variadic tail into its target, command verb, and args.
    ///
    /// The first token is a **CMD** iff it is one of `status | caps |
    /// support | protocol`; otherwise it is a **TARGET**. That rule is owned
    /// here (issue #148): the old `target()`/`cmd()`/`args()` accessors
    /// treated the first token as the target unconditionally, so for the local
    /// form `query status` `target()` wrongly returned `status` and `cmd()`
    /// returned `None`.
    ///
    /// A local form with no trailing command (`query`, or a bare non-CMD token)
    /// is a usage error (exit 2) — `rest` is a non-empty variadic, so an empty
    /// tail is unreachable (clap rejects it) and only the second shape errors.
    pub fn resolve(&self) -> Result<QueryResolution, Usage> {
        // `rest` is a required variadic, so clap guarantees at least one token;
        // `let-else` (not `expect`) keeps the borrow checker happy.
        let first = match self.rest.first() {
            Some(first) => first,
            None => return Err(Usage::new("a query needs a CMD [ARGS...] or TARGET CMD".to_string())),
        };
        if is_cmd(first) {
            // Local form: `query CMD [ARGS...]` — no target.
            Ok(QueryResolution::Local {
                cmd: cmd_of(first),
                args: self.rest.iter().skip(1).cloned().collect(),
            })
        } else if let Some(second) = self.rest.get(1) {
            // Remote form: `query TARGET CMD [ARGS...]`.
            Ok(QueryResolution::Remote {
                target: Target::new(first.clone()),
                cmd: cmd_of(second),
                args: self.rest.iter().skip(2).cloned().collect(),
            })
        } else {
            // Remote target named, but no command after it: a usage error.
            Err(Usage::new(format!(
                "a command is required after the target: `query {first} CMD [ARGS...]`"
            )))
        }
    }
}

/// The result of [`Query::resolve`]: the query tail disambiguated into its
/// TARGET (remote form) or CMD (local form), the command verb, and the
/// remaining args.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResolution {
    /// Local form (`query CMD [ARGS...]`): no remote target.
    Local { cmd: Cmd, args: Vec<String> },
    /// Remote form (`query TARGET CMD [ARGS...]`): a target is named.
    Remote {
        target: Target,
        cmd: Cmd,
        args: Vec<String>,
    },
}

impl QueryResolution {
    /// The remote target, if this is the remote form.
    pub fn target(&self) -> Option<&Target> {
        match self {
            QueryResolution::Local { .. } => None,
            QueryResolution::Remote { target, .. } => Some(target),
        }
    }

    /// The command's verb.
    pub fn cmd(&self) -> &Cmd {
        match self {
            QueryResolution::Local { cmd, .. } | QueryResolution::Remote { cmd, .. } => cmd,
        }
    }

    /// The arguments following the command.
    pub fn args(&self) -> &[String] {
        match self {
            QueryResolution::Local { args, .. } | QueryResolution::Remote { args, .. } => args,
        }
    }
}

/// A remote target for `query`: the first token of the remote form, kept
/// verbatim. ADR 0003 says `TARGET` is `token id | client id | label |
/// label/session` and is resolved *at the hub* against the roster; the CLI
/// only classifies the shape, it never resolves the name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target(String);

impl Target {
    /// Wrap a raw target token. (The CLI does not validate the grammar here —
    /// ADR 0005's resolution and any ambiguity are the hub's job at call time.)
    pub fn new(raw: String) -> Self {
        Self(raw)
    }

    /// The target exactly as the user typed it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The command verb of a `query`, disambiguated by `resolve`. The four names
/// are the `query/…` method catalog from protocol v2 (`v2.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    /// `query/status`
    Status,
    /// `query/caps`
    Caps,
    /// `query/support`
    Support,
    /// `query/protocol`
    Protocol,
}

impl Cmd {
    /// The wire method name (`query/status`, …) for [`v2.md`](../../../../docs/protocol/v2.md) §4.
    pub fn method(self) -> &'static str {
        match self {
            Cmd::Status => "query/status",
            Cmd::Caps => "query/caps",
            Cmd::Support => "query/support",
            Cmd::Protocol => "query/protocol",
        }
    }
}

/// A usage error from [`Query::resolve`] — the tail names a target but no
/// command (ADR 0003: exit 2). The message is what `main` prints to stderr.
#[derive(Debug)]
pub struct Usage {
    message: String,
}

impl Usage {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// True iff `token` is a query command verb (the CMD/TARGET disambiguator).
fn is_cmd(token: &str) -> bool {
    matches!(token, "status" | "caps" | "support" | "protocol")
}

/// Map a command-verb string (already known to be a CMD) to its enum.
fn cmd_of(token: &str) -> Cmd {
    match token {
        "caps" => Cmd::Caps,
        "support" => Cmd::Support,
        "protocol" => Cmd::Protocol,
        // is_cmd() is the gate; `status` and nothing else fall through.
        _ => Cmd::Status,
    }
}

// --- top-level (hub-only daily verbs) ----------------------------------

#[derive(Parser, Debug)]
pub struct Roster {
    /// Include `gone` rows (bodies that have dropped off) in the listing.
    /// Without this the roster shows the live-only view (connected +
    /// reconnecting).
    #[arg(long)]
    pub all: bool,
    /// List only sessions named exactly PREFIX or nested under it
    /// (`PREFIX/…`) — e.g. `--prefix io` (or `io/`) matches `io` and every
    /// `io/<session>` (ADR 0005 §4).
    #[arg(long)]
    pub prefix: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Say {
    /// Session address, <label>/<session> (or a bare <session>).
    pub session: String,
    /// Prompt text. Omit when `--parts-file` supplies the full A2A message.
    #[arg(required_unless_present = "parts_file")]
    pub text: Option<String>,
    /// A full A2A `Message` (non-text parts) instead of a plain-text `TEXT`.
    #[arg(long)]
    pub parts_file: Option<String>,
    /// Wait up to this long for the reply (default 600s).
    #[arg(long, default_value = "600s")]
    pub timeout: String,
    /// Append behind a busy session's current turn instead of refusing with
    /// `session_busy`.
    #[arg(long)]
    pub queue: bool,
}

#[derive(Parser, Debug)]
pub struct Interrupt {
    /// Session address, <label>/<session> (or a bare <session>).
    pub session: String,
    /// Redirect text (issue #191): cancel, then run this prompt ahead of
    /// the queue, streaming its reply exactly like `say`.
    pub text: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Answer {
    /// Session address, <label>/<session> (or a bare <session>).
    pub session: String,
    /// The choice: a 0-based index, an option's label/key (case-insensitive),
    /// a comma-separated list (one segment per question, for a multi-field
    /// elicitation), or one of `once`/`always`/`reject` for a permission
    /// prompt that offers them.
    pub choice: String,
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

#[cfg(test)]
mod tests {
    // Unit tests legitimately use unwrap/expect/panic (the integration test
    // files do the same at their crate top). An in-crate `mod` can't use a
    // file-level `#![allow]`, so the module opts out here instead.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #149

    use super::*;

    fn query(rest: &[&str]) -> Query {
        Query { rest: rest.iter().map(|s| s.to_string()).collect() }
    }

    /// The four shapes the old accessors got wrong (issue #148): `status`,
    /// `TARGET status`, `support opencode`, `TARGET support opencode`.
    #[test]
    fn resolves_the_four_query_shapes() {
        // (a) Local, no target: `query status` — cmd is the first token.
        let r = query(&["status"]).resolve().expect("local `query status` must resolve");
        assert!(r.target().is_none(), "`query status` has no target");
        assert_eq!(r.cmd(), &Cmd::Status);
        assert!(r.args().is_empty());

        // (b) Remote, same verb: `query TARGET status`.
        let r = query(&["io", "status"]).resolve().expect("remote `query io status` must resolve");
        assert_eq!(r.target(), Some(&Target::new("io".into())));
        assert_eq!(r.cmd(), &Cmd::Status);
        assert!(r.args().is_empty());

        // (c) Local, command taking an arg: `query support opencode`.
        let r = query(&["support", "opencode"])
            .resolve()
            .expect("local `query support opencode` must resolve");
        assert!(r.target().is_none(), "`query support opencode` has no target");
        assert_eq!(r.cmd(), &Cmd::Support);
        assert_eq!(r.args(), &["opencode".to_string()]);

        // (d) Remote, command taking an arg: `query TARGET support opencode`.
        let r = query(&["io/alpha", "support", "opencode"])
            .resolve()
            .expect("remote `query io/alpha support opencode` must resolve");
        assert_eq!(r.target(), Some(&Target::new("io/alpha".into())));
        assert_eq!(r.cmd(), &Cmd::Support);
        assert_eq!(r.args(), &["opencode".to_string()]);
    }

    /// A target with no following command is a usage error (ADR 0003: exit 2).
    #[test]
    fn target_without_command_is_a_usage_error() {
        // A bare, non-CMD token is a remote target with no command after it.
        let err = query(&["io"]).resolve().expect_err("`query io` (no cmd) must be a usage error");
        let Usage { .. } = err; // just assert it is the usage error type
        assert!(err.to_string().contains("io"));
    }

    /// The four verbs map onto the v2 §4 method catalog names.
    #[test]
    fn cmd_maps_to_the_wire_method_name() {
        assert_eq!(Cmd::Status.method(), "query/status");
        assert_eq!(Cmd::Caps.method(), "query/caps");
        assert_eq!(Cmd::Support.method(), "query/support");
        assert_eq!(Cmd::Protocol.method(), "query/protocol");
    }

    /// `caps` and `protocol` also disambiguate local-vs-remote by the same rule.
    #[test]
    fn caps_and_protocol_resolve_like_the_other_verbs() {
        assert_eq!(query(&["caps"]).resolve().unwrap().cmd(), &Cmd::Caps);
        assert_eq!(
            query(&["beta", "caps"]).resolve().unwrap().cmd(),
            &Cmd::Caps
        );
        // `protocol <n>` keeps the version in the args (query/protocol {version}).
        let r = query(&["protocol", "2"]).resolve().unwrap();
        assert_eq!(r.cmd(), &Cmd::Protocol);
        assert_eq!(r.args(), &["2".to_string()]);
    }
}
