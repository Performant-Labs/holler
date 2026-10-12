//! `holler_adapter_host` — the host adapter: [`TmuxHost`] implements
//! `holler_pane::HostPort` (tmux sessions and the processes in them) over a tmux server
//! on the machine the hub runs on (epic #633, decision 3: no ssh; ADR-0021 section 2;
//! story #641).
//!
//! **How it talks to tmux.** Every call is one `tmux` subprocess given an argument
//! vector: never a shell command line, and never keystrokes typed into a pane. Each
//! subprocess has a null stdin and `TMUX` and `TMUX_PANE` removed from its environment,
//! so a hub that runs inside a tmux pane never reaches that pane's server by
//! inheritance. Nothing is added to it: a tmux client's environment reaches every pane
//! started after it. Each port call has one deadline, the configured bound (I5, default
//! 10 s), and every subprocess it spawns gets the time left. A subprocess still running
//! at the deadline is killed and reaped, and the call is `timeout`. Every subprocess, tmux
//! and `kill` alike, is run by the shared bounded runner of `holler-pane` (#696).
//!
//! **Exact targets.** `has-session` takes `-t =NAME`, and every other command that names
//! the session takes `-t =NAME:`. tmux matches a bare name by prefix, and `=NAME` on a
//! window target tries a window of the current session first, so either would let
//! `hj-c1r1` reach `hj-c1r10`.
//!
//! **What `run` starts.** `run` checks that the session exists (one `list-panes`), then
//! runs `new-window -d -P` with `-c <cwd>`, the directory it is given (#802: the pane
//! spec's, which need not be the one the session was made in), and the argv after `-- env
//! --`. The program starts in that directory and never through a shell (`env --` keeps a
//! one-element argv from reaching tmux as a single argument, which tmux would hand to `sh
//! -c`), and `run` returns once tmux reports the new pane's pid: no typing and no sleep. It
//! then tags the window with that pid (the window option `@holler-pid`) and sets the
//! window's own `remain-on-exit off`, in one tmux invocation. The record lives in tmux, not
//! in a file (I6). A window that closed before its tag (the program exited at once; tmux
//! says `no such window` or `can't find window`) is `Ok`. Any other failure of the tag
//! first kills the untagged process's group, so `run` never leaves a process `stop_owned`
//! cannot stop. A pane's harness server is one such program (#802), so `stop_owned` stops
//! it with the rest.
//!
//! **Ownership, and how `stop_owned` stops.** A pane is owned when its `@holler-pid`
//! equals its live `pane_pid`, and that pid is in `2..=i32::MAX`. `stop_owned` sends
//! `TERM` to the process group of every owned pane (the pane's process leads its own
//! group), waits one shared grace (default 2 s), sends `KILL` to every group that still
//! has a member, whether its pane is still listed or not, and waits for the rest of the
//! bound. A group is stopped once its pane is gone (not listed, or listed dead) and
//! `kill -s 0` finds no member. Every signal to a pane's processes goes through the
//! `kill` binary with `LC_ALL=C` (so the crate needs no `libc`), and only to the group of
//! an owned pane or to the pid `run` just started when tagging it failed. Nothing is
//! matched by name. The one other signal is the `KILL` of the adapter's own tmux or
//! `kill` subprocess when it outlives its deadline.
//!
//! **Escaping.** tmux ends a command at every argument that ends in `;`, after `--` too,
//! and it format-expands a start directory, where `#(...)` runs a shell command. So every
//! value the adapter did not write is escaped: an argv element or a directory that ends
//! in `;` gets a `\` before that `;`, and a directory has every `#` doubled. A pid and a
//! window id read back from tmux are parsed strictly before they are reused.
//!
//! **What `foreground_commands` reads (#835).** One `list-panes` of the session, the listing
//! `ps` makes, asking tmux for each pane's `#{pane_current_command}`: the command name of the
//! pane's foreground process, which tmux takes from that process's `argv[0]` (cut at the first
//! space, a path reduced to its basename). The adapter never asks for a pane's start command
//! or a command line, and reads nothing of the process itself (I7). A dead pane is left out,
//! as `ps` leaves it out.
//!
//! **What `attached_clients` reads (#647 part 2).** The same `list-panes` listing of the
//! session, asking tmux for `#{session_attached}`: the number of clients attached to the
//! session, which tmux prints on every pane's line. The listing must be that one count on each
//! line, or the answer is `unavailable` naming the call and quoting none of tmux's output. A
//! count only: the adapter never asks for a client's name, tty or process (I7).
//!
//! **Errors.** Each column is a method; `op` of a `timeout` is `host.<method>`.
//!
//! | What happened | `ensure_session` | `run` | `stop_owned` | `ps` | `foreground_commands` | `attached_clients` |
//! |---|---|---|---|---|---|---|
//! | the session or its server is missing (stderr `can't find session`, `can't find window`, `no such window`, `no server running`, or `error connecting to` with `No such file or directory` or `Connection refused`) | created, or `usage` when its directory is not an absolute path to an existing directory | `pane-not-found` | `Ok` | `pane-not-found` | `pane-not-found` | `pane-not-found` |
//! | the session exists | `Ok`, nothing changes | starts the argv | stops what it owns | its live panes' pids | its live panes' command names | its client count |
//! | an empty argv, or an `argv[0]` containing `=` | | `usage` | | | | |
//! | `cwd` is not an absolute path to an existing directory | | `usage` | | | | |
//! | the bound ran out | `timeout` | `timeout` | `timeout` | `timeout` | `timeout` | `timeout` |
//! | a binary is missing, or tmux or `kill` failed otherwise | `unavailable` | `unavailable` | `unavailable` | `unavailable` | `unavailable` | `unavailable` |
//!
//! `run` checks the session before the argv, as the test kit's fake does, and the argv
//! before `cwd`, which the fake does not check. A `usage` never echoes an argv element or
//! the directory. An `unavailable` carries tmux's or `kill`'s first stderr line, or the
//! path of a binary that could not be run: never an argv element or a directory.
//!
//! **Narrowings.** ADR-0021 is unchanged. Five readings of the port are narrower than
//! the fake's, which does none of them:
//!
//! 1. `stop_owned` stops what `run` started (the process groups of the tagged panes),
//!    not every process of the session: the shell window `ensure_session` creates, and
//!    anything a person opened, is never signalled.
//! 2. `ensure_session` with a directory that does not exist is `usage` when the session
//!    would be created; tmux would create it anyway.
//! 3. The same for a relative directory, `.` included: tmux keeps it as given and
//!    resolves it again against every later client's directory.
//! 4. `run` with an `argv[0]` containing `=` is `usage`: `env` would read it as an
//!    assignment.
//! 5. `run` with a `cwd` that is missing, empty or not absolute is `usage` (#802), before
//!    any window is made: tmux would start the program in `$HOME`, or relative to the
//!    caller's directory, with exit 0.
//!
//! **What a consumer must know.**
//!
//! - `run` creates its window detached, so a client attached to the session keeps its
//!   window.
//! - While the session's shell window exists, the session survives `stop_owned` and `ps`
//!   is not empty. A stop that ends the session's last window ends the session (the
//!   shell may have exited), so a relaunch is `stop_owned`, then `ensure_session`, then
//!   `run`. No port call ends a session: `close` leaves it, and the next launch of that
//!   name reuses it.
//! - A session keeps the directory it was made in (`ensure_session` of an existing
//!   session changes nothing), but `run` does not use it: each program starts in the
//!   `cwd` it is given. Only the session's own shell window stays in the old directory.
//! - A process started without `run` (before the cutover, say) is untagged, and
//!   `stop_owned` never stops it. Neither does it stop a process that left its group
//!   (`setsid`, a daemon).
//! - A program that exits before its tag lands, under a global `remain-on-exit on`,
//!   leaves a dead tagged window. It is never signalled and stays until its session ends.
//! - When `new-window` itself runs out of time, tmux may still start the program: an
//!   untagged process, which `ps` lists and `stop_owned` never stops.
//! - The tmux defaults assumed: `exit-unattached off` (else a server that `new-session -d`
//!   just started would end at once). `remain-on-exit` is set per window.
//! - The stop grace should be well under the bound: the `KILL`s get only the time left
//!   after it.

mod exec;
mod tmux;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::{Argv, HostPort, PaneError, PaneName};

use crate::exec::{Failed, Group, Signal};
use crate::tmux::{Listing, Pid, Refusal, Window};

/// The default bound on every port call (I5).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// The default grace between `TERM` and `KILL` in `stop_owned`.
const DEFAULT_STOP_GRACE: Duration = Duration::from_secs(2);

/// How often `stop_owned` looks again while it waits for its groups (Decision 4).
const STOP_POLL: Duration = Duration::from_millis(50);

/// The least time the cleanup `kill` of a failed tag gets, past the call's deadline if
/// need be (Decision 3).
const CLEANUP_MIN: Duration = Duration::from_millis(250);

/// The longest wait a deadline is computed for; no call comes near it.
const DEADLINE_CAP: Duration = Duration::from_secs(365 * 24 * 60 * 60);

// The `op` of each method's `Timeout`: the names `holler_pane_testkit::host::HostOp`
// gives the port's methods (the default-run timeout test pins them equal).
const OP_ENSURE_SESSION: &str = "host.ensure_session";
const OP_RUN: &str = "host.run";
const OP_STOP_OWNED: &str = "host.stop_owned";
const OP_PS: &str = "host.ps";
const OP_FOREGROUND_COMMANDS: &str = "host.foreground_commands";
const OP_ATTACHED_CLIENTS: &str = "host.attached_clients";

/// The fixed `what` of each refusal of tmux's output; none quotes it.
const BAD_NEW_WINDOW: &str =
    "tmux new-window did not print a pid in 2..=2147483647 and a window id";
const BAD_PANE_LIST: &str =
    "tmux list-panes printed a pane that is not a pid in 2..=2147483647 and a dead flag";
const BAD_COMMAND_LIST: &str =
    "tmux list-panes printed a pane that is not a dead flag and a command name";
const BAD_ATTACHED_LIST: &str =
    "tmux list-panes did not print the session's attached-client count on each pane's line";

/// Which tmux server the adapter talks to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxSocket {
    /// tmux's own default socket (no `-L` or `-S`). `$TMUX` is never used to find it.
    Default,
    /// A named socket in tmux's socket directory (`-L <name>`).
    Name(String),
    /// A socket at a path (`-S <path>`).
    Path(PathBuf),
}

/// The real `HostPort`: a local tmux server, driven by argument vectors, never a shell.
/// See the crate docs for the rules each method keeps.
///
/// Building one does no I/O; every method blocks, as the port says. It is plain data, so
/// it is `Send + Sync`.
#[derive(Debug, Clone)]
pub struct TmuxHost {
    socket: TmuxSocket,
    tmux: PathBuf,
    kill: PathBuf,
    config: Option<PathBuf>,
    timeout: Duration,
    stop_grace: Duration,
}

impl TmuxHost {
    /// A host on `socket`, with `tmux` and `kill` from `PATH`, no config file, a 10 s
    /// bound and a 2 s stop grace. Does no I/O.
    pub fn new(socket: TmuxSocket) -> Self {
        Self {
            socket,
            tmux: PathBuf::from("tmux"),
            kill: PathBuf::from("kill"),
            config: None,
            timeout: DEFAULT_TIMEOUT,
            stop_grace: DEFAULT_STOP_GRACE,
        }
    }

    /// The tmux binary to run (default `tmux` from `PATH`).
    #[must_use]
    pub fn with_tmux_binary(mut self, path: PathBuf) -> Self {
        self.tmux = path;
        self
    }

    /// The `kill` binary every signal goes through (default `kill` from `PATH`).
    #[must_use]
    pub fn with_kill_binary(mut self, path: PathBuf) -> Self {
        self.kill = path;
        self
    }

    /// A tmux config file, passed as `-f <path>` to every call; tmux reads it when the
    /// server starts.
    #[must_use]
    pub fn with_config(mut self, path: PathBuf) -> Self {
        self.config = Some(path);
        self
    }

    /// The bound on every port call (default 10 s).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The grace between `TERM` and `KILL` in `stop_owned` (default 2 s).
    #[must_use]
    pub fn with_stop_grace(mut self, grace: Duration) -> Self {
        self.stop_grace = grace;
        self
    }

    /// Run tmux with `args` after the socket and config flags, with `TMUX` and
    /// `TMUX_PANE` removed and nothing added (Decision 9).
    fn run_tmux(&self, args: &[String], call: Call) -> Result<Reply, PaneError> {
        let mut command = Command::new(&self.tmux);
        command
            .args(tmux::globals(&self.socket, self.config.as_deref()))
            .args(args)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE");
        let ran = exec::run(&mut command, call.deadline).map_err(|failed| call.failed(failed))?;
        if ran.status.success() {
            return Ok(Reply::Done(ran.stdout));
        }
        let subcommand = args.first().map_or("", String::as_str);
        let what = format!("tmux {subcommand}: {}", ran.reason());
        Ok(Reply::Refused(tmux::classify(&ran.stderr), what))
    }

    /// Send `signal` to the process group `pgid`, within the call's deadline.
    fn signal(&self, signal: Signal, pgid: Pid, call: Call) -> Result<Group, PaneError> {
        exec::kill_group(&self.kill, signal, pgid, call.deadline)
            .map_err(|failed| call.failed(failed))
    }

    /// Tag the window `run` started (Decision 3). A window already gone (its program
    /// exited at once) is `Ok`. Any other failure first sends `KILL` to the group of the
    /// pid `new-window` printed, so that no untagged process is left that `stop_owned`
    /// could never stop (W-3), then returns the failure. That `kill` gets the time left
    /// or [`CLEANUP_MIN`], whichever is longer, and is best effort: the error returned is
    /// the tag's.
    fn tag(&self, name: &PaneName, window: Window, pid: Pid, call: Call) -> Result<(), PaneError> {
        let error = match self.run_tmux(&tmux::tag(window, pid), call) {
            Ok(Reply::Done(_) | Reply::Refused(Refusal::WindowGone, _)) => return Ok(()),
            Ok(Reply::Refused(refusal, what)) => refused(name, refusal, what),
            Err(error) => error,
        };
        let cleanup = deadline_after(call.remaining().max(CLEANUP_MIN));
        let _ = exec::kill_group(&self.kill, Signal::Kill, pid, cleanup);
        Err(error)
    }

    /// `stop_owned`'s listing of the session, or `None` when the session is missing.
    fn listing(&self, name: &PaneName, call: Call) -> Result<Option<Listing>, PaneError> {
        match self.run_tmux(&tmux::stop_listing(name), call)? {
            Reply::Done(stdout) => Ok(Some(tmux::parse_listing(&stdout))),
            Reply::Refused(refusal, _) if refusal.is_missing() => Ok(None),
            Reply::Refused(_, what) => Err(PaneError::Unavailable { what }),
        }
    }

    /// Wait for the signalled `groups` (Decision 4): every [`STOP_POLL`], list the
    /// session and probe each group whose pane is gone, until every group is done or
    /// `until` passes. Returns the groups not done.
    ///
    /// A group is done once its pane is gone and `kill -s 0` finds no member; the probe
    /// is asked only once the pane is gone, since a member that ignores `TERM` outlives
    /// its leader's pane (W-14). A session found missing lists no pane: every pane is
    /// gone, and each group is still probed (W-18).
    fn settle(
        &self,
        name: &PaneName,
        mut groups: Vec<Pid>,
        until: Instant,
        call: Call,
    ) -> Result<Vec<Pid>, PaneError> {
        loop {
            let now = Instant::now();
            if groups.is_empty() || now >= until {
                return Ok(groups);
            }
            thread::sleep(STOP_POLL.min(until.saturating_duration_since(now)));
            let live = self
                .listing(name, call)?
                .map(|listing| listing.live)
                .unwrap_or_default();
            let mut left = Vec::with_capacity(groups.len());
            for pgid in groups {
                if live.contains(&pgid) || self.signal(Signal::Probe, pgid, call)? == Group::Member
                {
                    left.push(pgid);
                }
            }
            groups = left;
        }
    }
}

impl HostPort for TmuxHost {
    /// With an absolute path to an existing directory, `new-session -d -s NAME -c <cwd>`,
    /// where `duplicate session` is `Ok` and changes nothing (one call, race-free).
    /// With any other `cwd` the directory matters only if the session would be created:
    /// `has-session`, then `Ok` for an existing session and `usage` for a missing one.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
        let call = Call::start(OP_ENSURE_SESSION, self.timeout);
        if is_usable_dir(cwd) {
            return match self.run_tmux(&tmux::new_session(name, cwd), call)? {
                Reply::Done(_) | Reply::Refused(Refusal::Duplicate, _) => Ok(()),
                Reply::Refused(_, what) => Err(PaneError::Unavailable { what }),
            };
        }
        match self.run_tmux(&tmux::has_session(name), call)? {
            Reply::Done(_) => Ok(()),
            Reply::Refused(refusal, _) if refusal.is_missing() => Err(PaneError::Usage {
                message: format!(
                    "cannot create the tmux session {name}: its directory must be an \
                     absolute path to an existing directory"
                ),
            }),
            Reply::Refused(_, what) => Err(PaneError::Unavailable { what }),
        }
    }

    /// Three tmux calls on success: the read that the session exists, `new-window` in
    /// `cwd`, and the tag (Decision 3). The session is checked first, then the argv, then
    /// `cwd` (narrowing 5), all before any window is made.
    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
        let call = Call::start(OP_RUN, self.timeout);
        if let Reply::Refused(refusal, what) = self.run_tmux(&tmux::panes(name), call)? {
            return Err(refused(name, refusal, what));
        }
        check_argv(argv)?;
        check_cwd(name, cwd)?;
        let (pid, window) = match self.run_tmux(&tmux::new_window(name, cwd, argv), call)? {
            Reply::Done(stdout) => {
                tmux::parse_new_window(&stdout).ok_or_else(|| unavailable(BAD_NEW_WINDOW))?
            }
            Reply::Refused(refusal, what) => return Err(refused(name, refusal, what)),
        };
        self.tag(name, window, pid, call)
    }

    /// `TERM` to every owned group, one shared grace, `KILL` to every group not done, and
    /// the rest of the bound for those (Decision 4). A session missing at the first
    /// listing owns nothing: `Ok`.
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        let call = Call::start(OP_STOP_OWNED, self.timeout);
        let Some(first) = self.listing(name, call)? else {
            return Ok(());
        };
        for &pgid in &first.owned {
            self.signal(Signal::Term, pgid, call)?;
        }
        let grace = deadline_after(self.stop_grace).min(call.deadline);
        let survivors = self.settle(name, first.owned, grace, call)?;
        for &pgid in &survivors {
            self.signal(Signal::Kill, pgid, call)?;
        }
        if self
            .settle(name, survivors, call.deadline, call)?
            .is_empty()
        {
            Ok(())
        } else {
            Err(call.timeout())
        }
    }

    /// The pid of every live pane of the session, the shell included (Decision 5).
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
        let call = Call::start(OP_PS, self.timeout);
        match self.run_tmux(&tmux::panes(name), call)? {
            Reply::Done(stdout) => {
                tmux::parse_ps(&stdout).ok_or_else(|| unavailable(BAD_PANE_LIST))
            }
            Reply::Refused(refusal, what) => Err(refused(name, refusal, what)),
        }
    }

    /// The command name of every live pane of the session, the shell included (#835): one
    /// `list-panes`, read as strictly as `ps`'s, and failing as `ps` fails.
    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
        let call = Call::start(OP_FOREGROUND_COMMANDS, self.timeout);
        match self.run_tmux(&tmux::foreground_listing(name), call)? {
            Reply::Done(stdout) => {
                tmux::parse_foreground(&stdout).ok_or_else(|| unavailable(BAD_COMMAND_LIST))
            }
            Reply::Refused(refusal, what) => Err(refused(name, refusal, what)),
        }
    }

    /// The number of clients attached to the session (#647 part 2): one `list-panes`, the
    /// listing `ps` makes, asking for `#{session_attached}`, read as strictly as `ps`'s and
    /// failing as `ps` fails.
    fn attached_clients(&self, name: &PaneName) -> Result<u32, PaneError> {
        let call = Call::start(OP_ATTACHED_CLIENTS, self.timeout);
        match self.run_tmux(&tmux::attached_listing(name), call)? {
            Reply::Done(stdout) => {
                tmux::parse_attached(&stdout).ok_or_else(|| unavailable(BAD_ATTACHED_LIST))
            }
            Reply::Refused(refusal, what) => Err(refused(name, refusal, what)),
        }
    }
}

/// One port call: the `op` its `Timeout` names, and the one deadline every subprocess it
/// spawns shares (Decision 2).
#[derive(Debug, Clone, Copy)]
struct Call {
    op: &'static str,
    deadline: Instant,
}

impl Call {
    fn start(op: &'static str, timeout: Duration) -> Self {
        Self {
            op,
            deadline: deadline_after(timeout),
        }
    }

    fn remaining(self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    fn timeout(self) -> PaneError {
        PaneError::Timeout {
            op: self.op.to_owned(),
        }
    }

    /// The error of a subprocess that has no exit to read.
    fn failed(self, failed: Failed) -> PaneError {
        match failed {
            Failed::Expired => self.timeout(),
            Failed::Unavailable(what) => PaneError::Unavailable { what },
        }
    }
}

/// How a tmux call ended.
enum Reply {
    /// Exit 0, and its stdout.
    Done(String),
    /// A non-zero exit: its class (Decision 8), and the `what` of an `unavailable`
    /// (`tmux <subcommand>: <first stderr line>`).
    Refused(Refusal, String),
}

/// The error of a refused call of `run`, `ps`, `foreground_commands` or `attached_clients`
/// (Decision 8): a missing session is `pane-not-found`, anything else `unavailable`.
fn refused(name: &PaneName, refusal: Refusal, what: String) -> PaneError {
    if refusal.is_missing() {
        PaneError::PaneNotFound {
            what: name.to_string(),
        }
    } else {
        PaneError::Unavailable { what }
    }
}

/// Refuse an argv `run` cannot start (Decision 7): an empty one, and one whose
/// `argv[0]` contains `=`, which `env` would read as a variable assignment. The message
/// never echoes the element: `NAME=value` is the shape a secret arrives in (W-2).
fn check_argv(argv: &Argv) -> Result<(), PaneError> {
    let message = match argv.as_slice().first() {
        None => "an empty argv has no program to run",
        Some(program) if program.contains('=') => {
            "argv[0] contains '=', which env would read as a variable assignment"
        }
        Some(_) => return Ok(()),
    };
    Err(PaneError::Usage {
        message: message.to_owned(),
    })
}

/// Refuse a `cwd` that `run` cannot start a program in (narrowing 5): one that is not an
/// absolute path to an existing directory, where tmux would start the program in `$HOME` or
/// relative to the caller's directory, with exit 0. The message never echoes the directory.
fn check_cwd(name: &PaneName, cwd: &str) -> Result<(), PaneError> {
    if is_usable_dir(cwd) {
        return Ok(());
    }
    Err(PaneError::Usage {
        message: format!(
            "cannot run a program in the tmux session {name}: its directory must be an \
             absolute path to an existing directory"
        ),
    })
}

/// Whether `cwd` is an absolute path to an existing directory: the one directory rule of
/// `ensure_session` (when it creates a session) and `run`.
fn is_usable_dir(cwd: &str) -> bool {
    let dir = Path::new(cwd);
    dir.is_absolute() && dir.is_dir()
}

fn unavailable(what: &str) -> PaneError {
    PaneError::Unavailable {
        what: what.to_owned(),
    }
}

/// The instant `wait` from now; a wait past [`DEADLINE_CAP`] is capped there.
fn deadline_after(wait: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(wait.min(DEADLINE_CAP)).unwrap_or(now)
}
