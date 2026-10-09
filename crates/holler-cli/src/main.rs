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
//! join|confirm|detach|status|run|caps|support|query`), `attach_cmd.rs` (`body attach
//! sessions|init`, issue #196), plus the pre-existing
//! `say_cmd.rs`/`roster_cmd.rs`/`query_cmd.rs`. Every one of those modules
//! returns a plain ADR 0003 exit code (already having printed its own
//! stdout/stderr output); this file's only job is to call the right one and
//! apply it.
//!
//! `holler pane …` and `holler profile …` (epic #633, story #670) are the
//! `pane` and `profile` modules of the library: they print through
//! `output.rs` over the real stdout and stderr and return an exit code too.
//! Dispatch is one exhaustive `match` over [`Command`] (see [`dispatch`]), so a
//! command with no arm does not compile, instead of falling off the end of
//! `main` and exiting 0.

use clap::Parser;
use holler_cli::output::{self, ErrorBody, Format, FormatChoice, Sink, VerbCtx};
use holler_cli::pane::wiring::Wiring;
use holler_cli::{
    Attach, AttachCommand, Body, BodyCommand, Cli, Command, Hub, HubCommand, Roster, TokenCommand,
};
use holler_proto::log::{emit_banner, init, resolve};

/// The story that renders `roster` (and so `roster --profile`).
const ROSTER_STORY: u32 = 648;

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

/// Print a leaf command's result (a refusal to stderr with the `error: `
/// prefix, a success to stdout) and return its own exit code. Shared by
/// `Roster`/`Say`/`Answer`/`Interrupt` (issue #151 added `Answer`, #191 added
/// `Interrupt` — inlining each one's own copy of this same four-line if/else
/// pushed `main` past the workspace's cognitive-complexity/line-count
/// guards) so each call site is one line. `newline_on_success`: `Roster`'s
/// table already ends every line with its own `\n` (`print!`, no extra one);
/// `Say`/`Answer`/`Interrupt`'s reply text does not (`println!` adds it) —
/// the one difference between these call sites' prior inline copies.
fn print_leaf_result(
    message: &str,
    to_stderr: bool,
    exit_code: i32,
    newline_on_success: bool,
) -> i32 {
    if !message.is_empty() {
        if to_stderr {
            eprintln!("error: {message}");
        } else if newline_on_success {
            println!("{message}");
        } else {
            print!("{message}");
        }
    }
    exit_code
}

/// [`print_leaf_result`] for `hold`/`release`, which can also carry a
/// warning to stderr alongside a success.
fn print_hold_result(result: &holler_cli::hold_cmd::HoldResult) -> i32 {
    if let Some(w) = &result.warning {
        eprintln!("warning: {w}");
    }
    print_leaf_result(&result.message, result.to_stderr, result.exit_code, true)
}

fn main() {
    // Install the rustls crypto provider before any I/O: a `wss://` (TLS)
    // WebSocket needs a process-global provider set up ahead of the first
    // connection, and the body's `join` runs on a throwaway runtime later in
    // this process. (No-op when the build has no `ring` provider; a `wss`
    // join then still fails closed in `holler_body::join`.)
    install_tls_provider();

    let cli = parse_or_exit();
    let choice = format_or_exit(&cli);
    install_logging_or_exit(&cli);
    std::process::exit(dispatch(&cli, choice));
}

/// Parse the command line. A usage error exits 2 with clap's own message, and
/// `--help`/`--version` print and exit 0, exactly as `Cli::parse()` did — with
/// one addition (story #670, decision 1(d)): under the `pane` and `profile`
/// subcommands, in JSON mode, a usage error is an envelope with code `usage`
/// on stdout instead. Every other verb's usage error is untouched (ADR 0003:
/// the `--json` shape is part of the surface).
///
/// ADR 0003: a bare `holler` (or `holler hub`, an unknown subcommand, …)
/// fails closed with exit 2 and a usage message on stderr. That is clap's
/// own behaviour now — the root parser is `subcommand_required = true`, so
/// there is no pre-parse branch to special-case the empty argv (issue #148).
fn parse_or_exit() -> Cli {
    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.use_stderr() {
                if let Some(code) = usage_envelope(&error) {
                    std::process::exit(code);
                }
            }
            error.exit()
        }
    }
}

/// The exit code after printing `error` as a usage envelope, if the command
/// line asked for JSON under `pane` or `profile`. A failed parse has no parsed
/// flags, so this reads the raw arguments.
fn usage_envelope(error: &clap::Error) -> Option<i32> {
    let argv: Vec<String> = std::env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let raw = output::scan_args(&argv, &Cli::global_value_flags());
    let message = output::usage_message(error, raw.envelope_namespace()?);
    let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
    Some(output::emit_usage_error(
        &mut Sink {
            out: &mut out,
            err: &mut err,
        },
        Format::Json,
        &message,
    ))
}

/// Resolve `--json` and `--format` once. `--json` with `--format=text` is a
/// usage error (exit 2): an envelope under `pane` and `profile`, plain text
/// elsewhere.
fn format_or_exit(cli: &Cli) -> FormatChoice {
    match output::resolve_format(cli.json, cli.format) {
        Ok(choice) => choice,
        Err(error) => {
            let code = if matches!(cli.command, Command::Pane(_) | Command::Profile(_)) {
                let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
                output::emit_error(
                    &mut Sink {
                        out: &mut out,
                        err: &mut err,
                    },
                    Format::Json,
                    error,
                )
            } else {
                eprintln!("error: {}", error.message);
                2
            };
            std::process::exit(code)
        }
    }
}

/// Resolve the logging dials: the CLI flag beats the environment, the
/// defaults are none/text, and an unrecognised value fails closed (exit 3)
/// *before* the banner — the banner itself reports the resolved settings.
/// (The env values are read here, not in `resolve`, so the proto core stays
/// free of any OS dependency.)
///
/// Then install the config for any code that emits events later, and emit the
/// banner. The banner is the first stderr write and precedes any blocking
/// I/O; it reports the settings just installed.
fn install_logging_or_exit(cli: &Cli) {
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
    init(config);
    emit_banner();
}

/// Run the command and return its exit code. Every [`Command`] has an arm and
/// there is no catch-all, so a new command that is not dispatched fails to
/// compile. `--format=json` is `--json` for the verbs that predate the
/// envelope: they keep their own `--json` shape, and this passes them the one
/// resolved `json` (#648 takes the explicit-`--format=json` bit for `roster`
/// from `choice.json_explicit`).
fn dispatch(cli: &Cli, choice: FormatChoice) -> i32 {
    let json = choice.format == Format::Json;
    match &cli.command {
        Command::Hub(hub) => run_hub(hub, json),
        Command::Body(body) => run_body(body, json),
        Command::Pane(cmd) => run_with_stdio(choice.format, |ctx| holler_cli::pane::run(cmd, ctx)),
        Command::Profile(cmd) => {
            run_with_stdio(choice.format, |ctx| holler_cli::profile::run(cmd, ctx))
        }
        Command::Roster(roster) => run_roster(roster, json),
        Command::Say(say) => {
            let result = holler_cli::say_cmd::run(say, json);
            print_leaf_result(&result.message, result.to_stderr, result.exit_code, true)
        }
        Command::Interrupt(interrupt) => {
            let result = holler_cli::interrupt_cmd::run(interrupt, json);
            print_leaf_result(&result.message, result.to_stderr, result.exit_code, true)
        }
        Command::Answer(answer) => {
            let result = holler_cli::answer_cmd::run(answer, json);
            print_leaf_result(&result.message, result.to_stderr, result.exit_code, true)
        }
        Command::Hold(hold) => print_hold_result(&holler_cli::hold_cmd::hold(hold, json)),
        Command::Release(release) => {
            print_hold_result(&holler_cli::hold_cmd::release(release, json))
        }
        Command::Wait(wait) => {
            let result = holler_cli::wait_cmd::run(wait, json);
            print_leaf_result(&result.message, result.to_stderr, result.exit_code, false)
        }
    }
}

/// Run a `pane` or `profile` verb over the real wiring, printing to stdout and
/// stderr. A wiring that cannot be built is reported the way a verb reports an
/// error (exit 1, with the error's code).
fn run_with_stdio(format: Format, verb: impl FnOnce(&mut VerbCtx<'_>) -> i32) -> i32 {
    let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
    let mut sink = Sink {
        out: &mut out,
        err: &mut err,
    };
    match Wiring::connect() {
        Ok(wiring) => verb(&mut VerbCtx {
            format,
            ports: wiring.ports(),
            sink,
        }),
        Err(error) => output::emit_error(&mut sink, format, ErrorBody::from(&error)),
    }
}

/// `roster`. `--profile` (epic #633) parses but is refused until story #648
/// renders it, so `roster_cmd.rs` stays untouched for that story.
fn run_roster(roster: &Roster, json: bool) -> i32 {
    if roster.profile.profile.is_some() {
        return print_leaf_result(
            &output::not_implemented_message(ROSTER_STORY),
            true,
            1,
            false,
        );
    }
    let result = holler_cli::roster_cmd::run(roster, json);
    print_leaf_result(&result.message, result.to_stderr, result.exit_code, false)
}

/// Story #143: the hub leaves that are implemented — `hub serve` (the
/// loopback listener + control socket) and `hub status` (read the live hub's
/// status over the control socket). Every other leaf is still the
/// skeleton's "not implemented" (their stories land later).
fn run_hub(hub: &Hub, json: bool) -> i32 {
    match &hub.command {
        // `run` is a lib and returns the exit code (ADR 0003: 0 clean
        // signal shutdown, 1 runtime failure, 3 fail-closed refusal).
        // The bin — the one place a helper's code may be acted on by
        // exiting — applies it.
        HubCommand::Serve(serve) => {
            holler_hub::serve::run(&serve.listen, serve.advertise.as_deref(), &serve.join_held)
        }
        // `--json` is the root-level global flag (issue #147/#155), not
        // a per-leaf field. `--server` (issue #508) is per-leaf.
        HubCommand::Status(status) => holler_cli::hub_cmd::status(status.server.as_deref(), json),
        // Story #163: the `hub token` leaf operates the hub's token store
        // (the file-backed store behind the loopback listener). Each verb
        // runs the store's blocking `flock` operation directly — the CLI is
        // not a tokio runtime, so there is no executor to starve (the
        // store's `spawn_blocking` wrappers are for the serve path).
        HubCommand::Token(token) => match &token.command {
            TokenCommand::Mint(mint) => holler_cli::token_cmd::mint(mint, json),
            TokenCommand::List(_) => holler_cli::token_cmd::list(json),
            TokenCommand::Delete(del) => holler_cli::token_cmd::delete(&del.id, json),
            TokenCommand::Revoke(rev) => holler_cli::token_cmd::revoke(&rev.id, json),
            TokenCommand::Ping(ping) => holler_cli::token_cmd::ping(&ping.id, json),
        },
        HubCommand::Caps(_) => holler_cli::hub_cmd::caps(json),
        HubCommand::Support(support) => holler_cli::hub_cmd::support(&support.feature, json),
        HubCommand::Query(query) => holler_cli::hub_cmd::query(query, json),
    }
}

/// Story #176: the body's identity leaves — `body join` (redeem a minted
/// join secret over the wire and persist the identity), `body detach`
/// (forget it), and `body status` (report this process's own identity).
/// Story #182 adds `body run` (the live connection loop) and makes `body
/// detach` live-aware (write `detach_request` and wait, when a run is
/// active, rather than only deleting the credential). Issue #196 adds
/// `body attach sessions`/`body attach init` (pure HTTP + file write over
/// an OpenCode endpoint, no hub/`SessionManager` involvement).
fn run_body(body: &Body, json: bool) -> i32 {
    match &body.command {
        BodyCommand::Join(join) => {
            holler_cli::body_cmd::join(&join.server, &join.token, &join.hub_key)
        }
        BodyCommand::Confirm(_) => holler_cli::body_cmd::confirm(),
        BodyCommand::Detach(_) => holler_cli::body_cmd::detach(),
        BodyCommand::Status(_) => holler_cli::body_cmd::status(json),
        BodyCommand::Run(run) => holler_cli::body_cmd::run(run.config.as_deref()),
        BodyCommand::Caps(_) => holler_cli::body_cmd::caps(json),
        BodyCommand::Support(support) => holler_cli::body_cmd::support(&support.feature, json),
        BodyCommand::Query(query) => holler_cli::body_cmd::query(query, json),
        BodyCommand::Attach(Attach { command }) => match command {
            AttachCommand::Sessions(sessions) => holler_cli::attach_cmd::sessions(sessions, json),
            AttachCommand::Init(init) => holler_cli::attach_cmd::init(init, json),
        },
    }
}
