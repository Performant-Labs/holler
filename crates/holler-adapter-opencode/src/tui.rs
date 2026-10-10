//! The tmux side of the adapter: which tmux server a pane's TUI runs on ([`TmuxSocket`],
//! [`TmuxConfig`]), and the pure builders and parsers of the tmux calls that `attach_tui`,
//! `select_session` and `shown_session` make (#642). The calls themselves are made by the
//! crate's private `attach` module.
//!
//! The builders and parsers are public for this crate's tests and are not an interface:
//! wiring (#649) uses only [`TmuxConfig`] and [`TmuxSocket`], and `holler-cli` never writes
//! tmux syntax. The rules every tmux call keeps:
//!
//! - **One tmux server.** Every tmux process is built by [`tmux_command`], which names the
//!   configured socket and removes `TMUX` and `TMUX_PANE` from the child, so
//!   [`TmuxSocket::Default`] is tmux's own default socket, never the server an inherited
//!   `$TMUX` names. The adapter never starts a tmux server (it only addresses sessions that
//!   exist), so a tmux config file has no effect and there is no `-f`.
//! - **Exact targets.** tmux matches a bare target by prefix (`-t demo` reaches
//!   `demo-c1r1`), so a session name reaches tmux only as [`exact_target`], `=<name>:`, the
//!   form the host adapter (#641) calls `window_target`. tmux resolves it to the active pane
//!   of the session's current window, and that pane is the TUI's.
//! - **Escaped values.** A value the adapter did not write goes through [`escape_arg`], or,
//!   for `-c`, through [`escape_dir`]: the host adapter's `escape` and `escape_cwd`, by the
//!   same rule. A pane name needs no escape: it is `[a-z0-9-]`.
//! - **A query counts only for its own session.** `display-message -p` on a missing exact
//!   target exits 0 with every field empty, so [`parse_query`] reads a reply as a pane only
//!   when its first field is the session's name.

use std::path::PathBuf;
use std::process::Command;

use holler_pane::PaneName;

use crate::ProcessEnv;

/// The one query of a pane, five fields in this order, joined by one TAB (U+0009) each:
/// the session's name, whether the pane is dead (`0` or `1`), a dead pane's exit status,
/// the command the pane was started with, and the pane's title. tmux prints a TAB inside
/// an argument of the start command as `\t` (tmux 3.7c), so only the title, which is last
/// and kept whole, can hold one.
pub const QUERY_FORMAT: &str =
    "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}";

/// The title of a TUI that shows a session: this, then the session's title, which
/// `create_session` sets to the session's id (opencode-pane-spike.md:137-151).
const TITLE_PREFIX: &str = "OC | ";

/// The title of a TUI on its home screen, and of one whose session still has a default
/// title.
const HOME_TITLE: &str = "OpenCode";

/// The URL of a server on the loopback address, up to its port.
const LOOPBACK_URL: &str = "http://127.0.0.1:";

/// The variable that turns the TUI's title off, which the adapter reads: under
/// [`ProcessEnv::Inherit`] the TUI's `env` removes it.
const TITLE_OFF: &str = "OPENCODE_DISABLE_TERMINAL_TITLE";

/// Which tmux server to address: the same variants as the host adapter's (#641)
/// `TmuxSocket`, so wiring (#649) configures one value and hands it to both adapters. An
/// adapter cannot depend on another adapter crate (ADR-0021 section 5), hence a mirror and
/// not a re-export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxSocket {
    /// tmux's own default socket: no flag. Never the server an inherited `$TMUX` names.
    Default,
    /// A named socket: `-L <name>`.
    Name(String),
    /// A socket path: `-S <path>`.
    Path(PathBuf),
}

/// How the adapter runs tmux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxConfig {
    /// The `tmux` binary; production: `"tmux"`, found on `PATH`.
    pub tmux_bin: PathBuf,
    /// Production: [`TmuxSocket::Default`]; tests: always [`TmuxSocket::Path`], a private
    /// server.
    pub socket: TmuxSocket,
}

/// What a TUI's terminal title says it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleShows {
    /// `OC | <id>` with a whole session id.
    Session(String),
    /// Exactly `OpenCode`: the home screen, or a session whose title is still the default
    /// one, which cannot be told apart from it.
    Home,
    /// Anything else: tmux's default title (the host name), a title cut to fit, a title
    /// that is not an id, or none.
    Unrecognised,
}

/// What one [`query_args`] reply says about the pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// No tmux pane of that exact session: the reply's first field is not the session's
    /// name (a missing exact target expands every field to empty), or it is malformed.
    NoPane,
    /// A dead pane (kept by `remain-on-exit on`), with its exit status when that parses as
    /// a number.
    Dead(Option<i32>),
    /// A live pane: the command it was started with, as tmux prints it, and its title.
    Live {
        start_command: String,
        title: String,
    },
}

/// A failed tmux call, read from its stderr (tmux's messages are not localized).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// No such session or pane, or no tmux server on the socket.
    Missing,
    /// Anything else.
    Other,
}

/// A `Command` for one tmux call: `<tmux_bin> [-S <path> | -L <name>] <args...>`, with
/// `TMUX` and `TMUX_PANE` removed from the child's environment. Every tmux process the
/// adapter starts is built here.
pub fn tmux_command(tmux: &TmuxConfig, args: &[String]) -> Command {
    let mut command = Command::new(&tmux.tmux_bin);
    match &tmux.socket {
        TmuxSocket::Default => {}
        TmuxSocket::Name(name) => {
            command.arg("-L").arg(name);
        }
        TmuxSocket::Path(path) => {
            command.arg("-S").arg(path);
        }
    }
    command
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE");
    command
}

/// `=<session>:`, the only form in which a session reaches tmux.
pub fn exact_target(session: &PaneName) -> String {
    format!("={session}:")
}

/// A value the adapter did not write, made safe for tmux's command line. tmux ends a
/// command at every argument that ends in `;`, after `--` too, and reads a trailing `\;` as
/// a literal `;`. So a value that ends in `;` gets a `\` before that `;`, and nothing else
/// changes: `x;` is `x\;`, `y\;` is `y\\;`, and `;` is `\;`.
pub fn escape_arg(value: &str) -> String {
    match value.strip_suffix(';') {
        Some(head) => format!("{head}\\;"),
        None => value.to_owned(),
    }
}

/// A start directory for `-c`. tmux format-expands it, and `#(...)` there would run a
/// shell command, so every `#` is doubled (`##` is a literal `#`) before [`escape_arg`].
pub fn escape_dir(dir: &str) -> String {
    escape_arg(&dir.replace('#', "##"))
}

/// The TUI's argv, unescaped. Under [`ProcessEnv::Isolated`] it is
///
/// ```text
/// env -i K=V ... <bin> attach http://127.0.0.1:<port> --dir <dir> --session <id>
/// ```
///
/// and under [`ProcessEnv::Inherit`] it starts `env -u OPENCODE_DISABLE_TERMINAL_TITLE`
/// instead of `env -i K=V ...`, so the title the adapter reads stays on.
pub fn tui_argv(
    opencode_bin: &str,
    env: &ProcessEnv,
    port: u16,
    dir: &str,
    session_id: &str,
) -> Vec<String> {
    let mut argv = vec!["env".to_owned()];
    match env {
        ProcessEnv::Inherit => argv.extend(["-u".to_owned(), TITLE_OFF.to_owned()]),
        ProcessEnv::Isolated(vars) => {
            argv.push("-i".to_owned());
            argv.extend(vars.iter().map(|(key, value)| format!("{key}={value}")));
        }
    }
    argv.extend([
        opencode_bin.to_owned(),
        "attach".to_owned(),
        format!("{LOOPBACK_URL}{port}"),
        "--dir".to_owned(),
        dir.to_owned(),
        "--session".to_owned(),
        session_id.to_owned(),
    ]);
    argv
}

/// `respawn-pane -k -t =<session>: -c <escape_dir(dir)> -- <escape_arg(e) for each e>`:
/// the pane's program is replaced by `tui_argv`, which tmux runs as an argv of several
/// arguments, without a shell.
pub fn respawn_args(session: &PaneName, dir: &str, tui_argv: &[String]) -> Vec<String> {
    let mut args = strings(&[
        "respawn-pane",
        "-k",
        "-t",
        &exact_target(session),
        "-c",
        &escape_dir(dir),
        "--",
    ]);
    args.extend(tui_argv.iter().map(|element| escape_arg(element)));
    args
}

/// `set-option -p -t =<session>: remain-on-exit on`: a TUI that exits stays a dead pane,
/// whose exit status can be read, instead of closing the tmux session.
pub fn remain_on_exit_args(session: &PaneName) -> Vec<String> {
    strings(&[
        "set-option",
        "-p",
        "-t",
        &exact_target(session),
        "remain-on-exit",
        "on",
    ])
}

/// `display-message -p -t =<session>: QUERY_FORMAT`: the one query of a pane.
pub fn query_args(session: &PaneName) -> Vec<String> {
    strings(&[
        "display-message",
        "-p",
        "-t",
        &exact_target(session),
        QUERY_FORMAT,
    ])
}

/// Read `query_args(session)`'s stdout, one trailing newline removed. Fewer than five
/// fields, a first field that is not `session`'s name, or a `#{pane_dead}` other than `0`
/// or `1` is [`Query::NoPane`]: no TUI is assumed.
pub fn parse_query(session: &PaneName, stdout: &str) -> Query {
    let reply = stdout.strip_suffix('\n').unwrap_or(stdout);
    let fields: Vec<&str> = reply.splitn(5, '\t').collect();
    let &[name, dead, status, start_command, title] = fields.as_slice() else {
        return Query::NoPane;
    };
    if name != session.as_str() {
        return Query::NoPane;
    }
    match dead {
        "0" => Query::Live {
            start_command: start_command.to_owned(),
            title: title.to_owned(),
        },
        "1" => Query::Dead(status.parse().ok()),
        _ => Query::NoPane,
    }
}

/// Read a TUI's title. `OC | <id>` with a whole session id (`ses_`, then one or more of
/// `[0-9A-Za-z]`; a title cut to 40 characters ends in `…` and is no id) is
/// [`TitleShows::Session`]; exactly `OpenCode` is [`TitleShows::Home`]; anything else is
/// [`TitleShows::Unrecognised`]. It never guesses an id from a title.
pub fn parse_title(title: &str) -> TitleShows {
    if title == HOME_TITLE {
        return TitleShows::Home;
    }
    match title.strip_prefix(TITLE_PREFIX) {
        Some(id) if is_session_id(id) => TitleShows::Session(id.to_owned()),
        _ => TitleShows::Unrecognised,
    }
}

/// The loopback port of an `opencode attach http://127.0.0.1:<port> ...` command line, as
/// tmux prints a pane's start command (an argument that holds a space is quoted, which
/// neither `attach` nor the URL ever needs), or `None` when the line is no such attach:
/// another program, `opencode serve`, or a server that is not on the loopback address. The
/// flags after the URL are not read.
pub fn attach_port(command_line: &str) -> Option<u16> {
    let words: Vec<&str> = command_line.split_whitespace().collect();
    words.windows(2).find_map(|pair| match pair {
        ["attach", url] => loopback_port(url),
        _ => None,
    })
}

/// Classify a failed tmux call by its stderr: no such session or pane (`can't find
/// session`, `can't find window`, `can't find pane`, `no such pane`), or no server on the
/// socket (`no server running`, or `error connecting to` with `No such file or directory`
/// or `Connection refused`), is [`Refusal::Missing`]; anything else is [`Refusal::Other`].
pub(crate) fn classify(stderr: &str) -> Refusal {
    let no_socket = stderr.contains("error connecting to")
        && (stderr.contains("No such file or directory") || stderr.contains("Connection refused"));
    let missing = [
        "can't find session",
        "can't find window",
        "can't find pane",
        "no such pane",
        "no server running",
    ]
    .iter()
    .any(|message| stderr.contains(message));
    if no_socket || missing {
        Refusal::Missing
    } else {
        Refusal::Other
    }
}

/// `ses_` and one or more of `[0-9A-Za-z]`: a whole session id.
fn is_session_id(text: &str) -> bool {
    text.strip_prefix("ses_").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}

/// The port of `http://127.0.0.1:<port>`: decimal digits only, and not 0.
fn loopback_port(url: &str) -> Option<u16> {
    let digits = url.strip_prefix(LOOPBACK_URL)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|&port| port != 0)
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&part| part.to_owned()).collect()
}
