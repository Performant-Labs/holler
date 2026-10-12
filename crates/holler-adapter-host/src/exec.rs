//! Running one child process under a deadline (Decision 2 of #641), and the one `kill`
//! call every signal of the adapter goes through (Decision 4).
//!
//! A child is run by the shared bounded runner, [`holler_pane::bounded`] (#696), as this
//! adapter always ran it: a null stdin, stdout and stderr both kept whole (no cap), each read
//! on a thread of its own so a child that writes more than a pipe holds cannot block, the
//! child left in the adapter's process group, and its exit polled every 2 ms until the
//! deadline. On the deadline the child is killed and reaped, and the call is
//! [`Failed::Expired`]: a hung call is reported, never waited on. A child that exits while
//! something it started still holds its pipes is `Expired` too once the deadline passes, so
//! no call outlives its bound. This module only maps the runner's answer to the adapter's
//! own [`Ran`] and texts.

use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

use holler_pane::bounded::{self, Capture, KillScope, Over, RunError, Spec};

use crate::tmux::Pid;

/// How the adapter runs tmux and `kill`: both outputs kept whole, the child alone killed on
/// the deadline, and its exit polled every 2 ms.
const RUN: Spec = Spec {
    stdout: WHOLE,
    stderr: WHOLE,
    kill: KillScope::Child,
    poll: Duration::from_millis(2),
};

/// An output kept whole: a cap no output reaches, so nothing is ever cut.
const WHOLE: Capture = Capture::Keep {
    cap: usize::MAX,
    over: Over::Truncate,
};

/// A child that ran to its exit: its status and what it printed, decoded lossily.
pub(crate) struct Ran {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Ran {
    /// Why a child that failed failed: its first non-empty stderr line, or its exit
    /// status when it printed nothing there.
    pub(crate) fn reason(&self) -> String {
        self.stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(|| self.status.to_string(), str::to_owned)
    }
}

/// Why a call has no exit to read.
#[derive(Debug)]
pub(crate) enum Failed {
    /// The deadline passed. A child that had started was killed and reaped.
    Expired,
    /// The program could not be run, or its output was lost. The text names the program,
    /// never one of its arguments (W-2).
    Unavailable(String),
}

/// Run `command`, whose program and arguments are set, to its exit or to `deadline`.
/// Nothing is spawned once `deadline` has passed.
pub(crate) fn run(command: &mut Command, deadline: Instant) -> Result<Ran, Failed> {
    let program = Path::new(command.get_program()).display().to_string();
    let ran = bounded::run(command, &RUN, deadline).map_err(|e| failed(&program, e))?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    Ok(Ran {
        status: ran.status,
        stdout: text(&ran.stdout),
        stderr: text(&ran.stderr),
    })
}

/// The adapter's [`Failed`] for the runner's `error`, naming `program` and none of its
/// arguments.
fn failed(program: &str, error: RunError) -> Failed {
    let what = match error {
        RunError::Expired => return Failed::Expired,
        RunError::Spawn(e) | RunError::WaitFailed(e) => format!("{program}: {e}"),
        RunError::ReaderFailed(e) => format!("{program}: no thread to read its output: {e}"),
        // `Overflow` cannot happen: no output is capped.
        RunError::OutputLost | RunError::Overflow => format!("{program}: its output was lost"),
    };
    Failed::Unavailable(what)
}

/// A signal of the adapter: `TERM` and `KILL` stop a process group, and `0` asks
/// whether the group still has a member (Decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signal {
    Term,
    Kill,
    Probe,
}

impl Signal {
    /// The signal as `kill -s` takes it.
    fn as_arg(self) -> &'static str {
        match self {
            Signal::Term => "TERM",
            Signal::Kill => "KILL",
            Signal::Probe => "0",
        }
    }
}

/// What `kill` found in a process group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    /// The signal reached a member (`kill` exited 0).
    Member,
    /// The group has no member (`No such process`).
    Empty,
}

/// Send `signal` to the process group `pgid` through the `kill` binary at `kill`:
/// `kill -s <SIG> -- -<pgid>`, with `LC_ALL=C` so that its `No such process` is not
/// translated (procps-ng `kill` is localized). Only a stderr holding `No such process`
/// means an empty group; any other failure is `Unavailable` with `kill`'s first stderr
/// line.
pub(crate) fn kill_group(
    kill: &Path,
    signal: Signal,
    pgid: Pid,
    deadline: Instant,
) -> Result<Group, Failed> {
    let group = format!("-{}", pgid.get());
    let mut command = Command::new(kill);
    command
        .args(["-s", signal.as_arg(), "--", &group])
        .env("LC_ALL", "C");
    let ran = run(&mut command, deadline)?;
    if ran.status.success() {
        Ok(Group::Member)
    } else if ran.stderr.contains("No such process") {
        Ok(Group::Empty)
    } else {
        Err(Failed::Unavailable(format!(
            "kill -s {}: {}",
            signal.as_arg(),
            ran.reason()
        )))
    }
}
