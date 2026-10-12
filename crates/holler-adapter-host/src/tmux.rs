//! What the adapter says to tmux and how it reads tmux's answers (#641): one pure
//! function per command line, the one escape for values the adapter did not write
//! (Decision 13), the strict parsers for what tmux prints (W-15), and the reading of a
//! failed call's stderr (Decision 8).
//!
//! **Targets** (Decision 10). `has-session` takes `-t =NAME`, a session target. Every
//! other command that names the session takes `-t =NAME:`. A bare name is matched by
//! prefix, and `=NAME` without the colon, on a window target, tries a window of the
//! current session before the session (Evidence round 2, P2-P4). `new-session -s NAME`
//! takes a name, not a target, so it has no `=` (PR4). Pane names are `[a-z0-9-]`, so a
//! name needs no escaping.

use std::ffi::OsString;
use std::fmt;
use std::path::Path;

use holler_pane::{Argv, PaneName};

use crate::TmuxSocket;

/// The largest pid `kill` reads as itself: `-<pid>` goes through `pid_t`, so a larger
/// `u32` would wrap to an unrelated process (W-15, PR3).
const MAX_PID: u32 = i32::MAX.unsigned_abs();

/// What `run`'s `new-window -P` prints: the new pane's pid and its window's id.
const NEW_WINDOW_FORMAT: &str = "#{pane_pid} #{window_id}";

/// What `ps` lists for every pane (and what `run` reads to know that the session exists).
const PS_FORMAT: &str = "#{pane_pid} #{pane_dead}";

/// What `foreground_commands` lists for every pane (#835): whether it is dead, then the
/// command name of its foreground process, last so that a name is the rest of its line.
/// tmux names the process by its `argv[0]`, cut at the first space, a path reduced to its
/// basename: a name only, never a command line (I7).
const FOREGROUND_FORMAT: &str = "#{pane_dead} #{pane_current_command}";

/// What `stop_owned` lists for every pane: also the window's tag (an untagged pane's
/// third field is empty, Q5).
const STOP_FORMAT: &str = "#{pane_pid} #{pane_dead} #{@holler-pid}";

/// The window option that records the pid `run` started (Decision 3).
const TAG_OPTION: &str = "@holler-pid";

/// The flags before every subcommand: the socket, then the config file.
pub(crate) fn globals(socket: &TmuxSocket, config: Option<&Path>) -> Vec<OsString> {
    let mut flags = Vec::new();
    match socket {
        TmuxSocket::Default => {}
        TmuxSocket::Name(name) => flags.extend([OsString::from("-L"), OsString::from(name)]),
        TmuxSocket::Path(path) => flags.extend([OsString::from("-S"), OsString::from(path)]),
    }
    if let Some(config) = config {
        flags.extend([OsString::from("-f"), OsString::from(config)]);
    }
    flags
}

/// Create the session `name`, detached, working in `cwd` (Decision 6). The caller has
/// checked that `cwd` is an absolute path to an existing directory.
pub(crate) fn new_session(name: &PaneName, cwd: &str) -> Vec<String> {
    let cwd = escape_cwd(cwd);
    strings(&["new-session", "-d", "-s", name.as_str(), "-c", &cwd])
}

/// Whether the session `name` exists.
pub(crate) fn has_session(name: &PaneName) -> Vec<String> {
    strings(&["has-session", "-t", &format!("={name}")])
}

/// `run`'s window: detached, printing its pid and id, starting in `cwd` (escaped:
/// tmux format-expands a start directory), with the argv after `env --` and each element
/// escaped (Decisions 3 and 13; #802). The caller has checked that `cwd` is an absolute
/// path to an existing directory. `env --` keeps a one-element argv from reaching tmux as
/// a single argument, which tmux would hand to a shell.
pub(crate) fn new_window(name: &PaneName, cwd: &str, argv: &Argv) -> Vec<String> {
    let target = window_target(name);
    let cwd = escape_cwd(cwd);
    let mut args = strings(&[
        "new-window",
        "-d",
        "-P",
        "-F",
        NEW_WINDOW_FORMAT,
        "-c",
        &cwd,
        "-t",
        &target,
        "--",
        "env",
        "--",
    ]);
    args.extend(argv.as_slice().iter().map(|element| escape(element)));
    args
}

/// The tag of `run`'s window, one invocation of two commands: the pid it started, and
/// its own `remain-on-exit off`, so a stopped run never lingers as a dead pane (W-8).
/// The `;` between them is the adapter's own.
pub(crate) fn tag(window: Window, pid: Pid) -> Vec<String> {
    let (window, pid) = (window.to_string(), pid.get().to_string());
    strings(&[
        "set-option",
        "-w",
        "-t",
        &window,
        TAG_OPTION,
        &pid,
        ";",
        "set-option",
        "-w",
        "-t",
        &window,
        "remain-on-exit",
        "off",
    ])
}

/// `ps`'s listing: every pane of the session, its pid and whether it is dead
/// (Decision 5). `run` asks it first, to know that the session exists: with the colon a
/// missing session fails, where `display-message` would print an empty line (PR6, PR7).
pub(crate) fn panes(name: &PaneName) -> Vec<String> {
    strings(&[
        "list-panes",
        "-s",
        "-t",
        &window_target(name),
        "-F",
        PS_FORMAT,
    ])
}

/// `stop_owned`'s listing, and every poll of it (Decision 4).
pub(crate) fn stop_listing(name: &PaneName) -> Vec<String> {
    strings(&[
        "list-panes",
        "-s",
        "-t",
        &window_target(name),
        "-F",
        STOP_FORMAT,
    ])
}

/// `foreground_commands`' listing (#835): every pane of the session, as `ps` lists them, with
/// the command name of its foreground process.
pub(crate) fn foreground_listing(name: &PaneName) -> Vec<String> {
    strings(&[
        "list-panes",
        "-s",
        "-t",
        &window_target(name),
        "-F",
        FOREGROUND_FORMAT,
    ])
}

/// The session `name` as a window target, exactly.
fn window_target(name: &PaneName) -> String {
    format!("={name}:")
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&part| part.to_owned()).collect()
}

/// A value the adapter did not write, made safe for tmux's command line (Decision 13).
/// tmux ends a command at every argument that ends in `;`, after `--` too, and reads a
/// trailing `\;` as a literal `;` (`man tmux`, PARSING SYNTAX; E1, R1). So a value that
/// ends in `;` gets a `\` before that `;`, and nothing else changes: `x;` is `x\;`, `y\;`
/// is `y\\;`, and `;` is `\;`.
pub(crate) fn escape(value: &str) -> String {
    match value.strip_suffix(';') {
        Some(head) => format!("{head}\\;"),
        None => value.to_owned(),
    }
}

/// A start directory for `-c`, of `new-session` and of `run`'s `new-window` (Decision 13).
/// tmux format-expands it, and `#(...)` there runs a shell command (E3, E4), so every `#`
/// is doubled (`##` is a literal `#`) before [`escape`].
pub(crate) fn escape_cwd(cwd: &str) -> String {
    escape(&cwd.replace('#', "##"))
}

/// A pid read from tmux, in `2..=i32::MAX` (W-15): a larger value wraps through `pid_t`
/// in `kill`, and `0` and `1` are `kill(2)`'s own group and every process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pid(u32);

impl Pid {
    /// `text` as a pid: decimal digits only, in range.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let value = decimal(text)?;
        (2..=MAX_PID).contains(&value).then_some(Self(value))
    }

    pub(crate) fn get(self) -> u32 {
        self.0
    }
}

/// A window id read from tmux: `@` and decimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Window(u32);

impl Window {
    fn parse(text: &str) -> Option<Self> {
        decimal(text.strip_prefix('@')?).map(Self)
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

/// `text` as a decimal number: ASCII digits only (no sign, no space), within `u32`.
fn decimal(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// `new-window -P`'s answer, read strictly: exactly `<pid> @<window>` and its newline.
pub(crate) fn parse_new_window(stdout: &str) -> Option<(Pid, Window)> {
    let line = stdout.strip_suffix('\n').unwrap_or(stdout);
    let (pid, window) = line.split_once(' ')?;
    Some((Pid::parse(pid)?, Window::parse(window)?))
}

/// `ps`'s listing: the pid of every live pane, or `None` when a line is not `<pid>
/// <0|1>` with the pid in range (Decision 5, W-15).
pub(crate) fn parse_ps(stdout: &str) -> Option<Vec<u32>> {
    let mut pids = Vec::new();
    for line in stdout.lines() {
        let (pid, dead) = line.split_once(' ')?;
        let pid = Pid::parse(pid)?;
        match dead {
            "0" => pids.push(pid.get()),
            "1" => {}
            _ => return None,
        }
    }
    Some(pids)
}

/// `foreground_commands`' listing (#835): the command name of every live pane, in the order
/// tmux lists them, or `None` when a line is not `<0|1> <name>`. Read as strictly as `ps`'s:
/// one malformed line fails the whole answer, never a partial list. A name is the rest of
/// its line, as tmux prints it.
pub(crate) fn parse_foreground(stdout: &str) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for line in stdout.lines() {
        let (dead, name) = line.split_once(' ')?;
        match dead {
            "0" => names.push(name.to_owned()),
            "1" => {}
            _ => return None,
        }
    }
    Some(names)
}

/// One reading of `stop_owned`'s listing (Decision 4). A line that is malformed or whose
/// pid is out of range is in neither list.
#[derive(Debug, Default)]
pub(crate) struct Listing {
    /// The panes `run` started: live, and tagged with their own pid.
    pub(crate) owned: Vec<Pid>,
    /// Every pane listed live (`pane_dead` 0), owned or not.
    pub(crate) live: Vec<Pid>,
}

/// Read `stop_owned`'s listing: `<pane_pid> <pane_dead> <@holler-pid>` per pane.
pub(crate) fn parse_listing(stdout: &str) -> Listing {
    let mut listing = Listing::default();
    for line in stdout.lines() {
        let mut fields = line.splitn(3, ' ');
        let (Some(pid_text), Some("0")) = (fields.next(), fields.next()) else {
            continue;
        };
        let Some(pid) = Pid::parse(pid_text) else {
            continue;
        };
        listing.live.push(pid);
        if fields.next() == Some(pid_text) {
            listing.owned.push(pid);
        }
    }
    listing
}

/// A failed tmux call, read from its stderr (Decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// No such session, or no server: `pane-not-found` for `run`, `ps` and
    /// `foreground_commands`, `Ok` for `stop_owned`.
    Missing,
    /// No such window: `can't find window` from a target lookup, and `no such window`,
    /// which `set-option -w -t @<id>` prints for a window that closed (tmux 3.7c). For
    /// `run`, `ps`, `foreground_commands` and `stop_owned` it is [`Refusal::Missing`]; for
    /// `run`'s tag it means the program already exited, and the window with it.
    WindowGone,
    /// `new-session` of a session that exists.
    Duplicate,
    /// Anything else: `unavailable`.
    Other,
}

impl Refusal {
    /// Whether the session is missing, as `run`, `ps`, `foreground_commands` and
    /// `stop_owned` read it.
    pub(crate) fn is_missing(self) -> bool {
        matches!(self, Refusal::Missing | Refusal::WindowGone)
    }
}

/// Classify a failed call by its stderr. tmux's messages are not localized.
pub(crate) fn classify(stderr: &str) -> Refusal {
    let no_socket = stderr.contains("error connecting to")
        && (stderr.contains("No such file or directory") || stderr.contains("Connection refused"));
    if stderr.contains("can't find window") || stderr.contains("no such window") {
        Refusal::WindowGone
    } else if no_socket
        || stderr.contains("can't find session")
        || stderr.contains("no server running")
    {
        Refusal::Missing
    } else if stderr.contains("duplicate session") {
        Refusal::Duplicate
    } else {
        Refusal::Other
    }
}
