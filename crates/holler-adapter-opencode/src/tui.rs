//! The tmux side of the adapter: which tmux server a pane's TUI runs on.
//!
//! In part 1 of #642 this module holds only [`TmuxSocket`] and [`TmuxConfig`], the type of
//! [`crate::OpenCodeConfig`]'s `tmux` field. Part 2 adds the tmux calls of `attach_tui`,
//! `select_session` and `shown_session` here, with their builders and parsers. This adapter
//! never starts a tmux server (it only addresses sessions that exist), so a tmux config
//! file has no effect and there is no `-f`.

use std::path::PathBuf;

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

// ---- RED stubs (T, #642b Phase 4) -------------------------------------------------------
//
// The minimum public surface the brief's API names, so that `tests/tui_test.rs` and
// `tests/attach_test.rs` compile and fail on their assertions rather than on missing items.
// Every body is a placeholder answer (empty, `None`, `NoPane`, `Unrecognised`); F replaces
// each one with the real builder or parser (docs/handoffs/642-brief.md, "The public API").

use std::process::Command;

use holler_pane::PaneName;

use crate::ProcessEnv;

/// The five fields of the TUI query, in order, joined by one TAB each (Decision 23).
pub const QUERY_FORMAT: &str = "";

/// What a TUI's terminal title shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleShows {
    /// `OC | <id>` with a whole session id.
    Session(String),
    /// Exactly `OpenCode`: the home screen.
    Home,
    /// Anything else.
    Unrecognised,
}

/// What one `query_args` reply says about the pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// No tmux pane of that exact session.
    NoPane,
    /// A dead pane, with its exit status when it parses.
    Dead(Option<i32>),
    /// A live pane: its start command and its title.
    Live {
        start_command: String,
        title: String,
    },
}

/// A `Command` for one tmux call (RED stub).
pub fn tmux_command(tmux: &TmuxConfig, _args: &[String]) -> Command {
    Command::new(&tmux.tmux_bin)
}

/// `=<session>:` (RED stub).
pub fn exact_target(_session: &PaneName) -> String {
    String::new()
}

/// #641's Decision 13 escape (RED stub).
pub fn escape_arg(_value: &str) -> String {
    String::new()
}

/// A start directory for `-c` (RED stub).
pub fn escape_dir(_dir: &str) -> String {
    String::new()
}

/// The TUI's argv, unescaped (RED stub).
pub fn tui_argv(
    _opencode_bin: &str,
    _env: &ProcessEnv,
    _port: u16,
    _dir: &str,
    _session_id: &str,
) -> Vec<String> {
    Vec::new()
}

/// `respawn-pane -k -t =<session>: -c <dir> -- <argv>` (RED stub).
pub fn respawn_args(_session: &PaneName, _dir: &str, _tui_argv: &[String]) -> Vec<String> {
    Vec::new()
}

/// `set-option -p -t =<session>: remain-on-exit on` (RED stub).
pub fn remain_on_exit_args(_session: &PaneName) -> Vec<String> {
    Vec::new()
}

/// `display-message -p -t =<session>: QUERY_FORMAT` (RED stub).
pub fn query_args(_session: &PaneName) -> Vec<String> {
    Vec::new()
}

/// Read a query reply (RED stub).
pub fn parse_query(_session: &PaneName, _stdout: &str) -> Query {
    Query::NoPane
}

/// Read a TUI title (RED stub).
pub fn parse_title(_title: &str) -> TitleShows {
    TitleShows::Unrecognised
}

/// The loopback port of an `opencode attach` command line (RED stub).
pub fn attach_port(_command_line: &str) -> Option<u16> {
    None
}
