//! Running a helper program to its end within a bound (private). The adapter runs two
//! helpers, never through a shell: `kill`, and tmux (the `attach` module's calls).
//!
//! - [`kill_group`] sends SIGKILL to a process group through the `kill` program, not a raw
//!   system call: when `serve`'s deadline passes before the server it started answers, it
//!   stops that server's group.
//! - [`capture`] runs a program to its exit and returns its status and what it printed,
//!   which the tmux calls read. It knows nothing of tmux: what tmux's answers mean is the
//!   `tui` module's.
//!
//! Both share one deadline loop: a child's exit is polled with `try_wait`, and on the
//! deadline the child is killed and reaped, and the answer is `timeout` with the caller's
//! `op`. A program that cannot be started is `unavailable`, naming it and none of its
//! arguments. The runner is shaped like the host adapter's (#641), so that one runner for
//! the adapters (#696) is a move.

use std::fmt::Display;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use holler_pane::PaneError;

/// The program that signals a process group, found on the adapter's own `PATH`.
const KILL: &str = "kill";

/// How often a running helper is checked for its exit.
const POLL: Duration = Duration::from_millis(5);

/// The most of each output [`capture`] keeps (64 KiB); the rest is read and dropped.
const MAX_OUTPUT: u64 = 64 << 10;

/// A helper that ran to its exit: its status, and what it printed, decoded lossily.
pub(crate) struct Ran {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Ran {
    /// Why a helper that failed failed, on one line: its first non-empty stderr line, or
    /// its exit status when it printed nothing there.
    pub(crate) fn reason(&self) -> String {
        self.stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(|| self.status.to_string(), crate::one_line)
    }
}

/// Which output a drained pipe was.
#[derive(Clone, Copy)]
enum Pipe {
    Out,
    Err,
}

/// SIGKILL every process in the group `pgid` (`kill -s KILL -- -<pgid>`), within `bound`.
///
/// A `pgid` of 0 or 1 is refused: `kill` reads `-0` as its caller's own group and `-1` as
/// every process the caller may signal.
pub(crate) fn kill_group(pgid: u32, op: &str, bound: Duration) -> Result<(), PaneError> {
    if pgid <= 1 {
        return Err(PaneError::Unavailable {
            what: format!("refusing to signal the process group {pgid}"),
        });
    }
    let group = format!("-{pgid}");
    let mut kill = Command::new(KILL);
    kill.args(["-s", "KILL", "--", group.as_str()]);
    let status = run(kill, op, bound)?;
    if status.success() {
        Ok(())
    } else {
        Err(PaneError::Unavailable {
            what: format!("kill -s KILL -- {group} failed ({status})"),
        })
    }
}

/// Run `command` to its exit before `deadline`, with a null stdin and its stdout and
/// stderr captured. Each output is drained on a thread of its own, so a child that writes
/// more than a pipe holds cannot block, and at most [`MAX_OUTPUT`] bytes of each are kept.
/// Nothing is started once `deadline` has passed. A child still running at `deadline` is
/// killed and reaped; that, a deadline that has passed, and outputs still open at
/// `deadline` (something the child started holds them) are `timeout` with `op`.
pub(crate) fn capture(mut command: Command, op: &str, deadline: Instant) -> Result<Ran, PaneError> {
    let program = program_of(&command);
    if Instant::now() >= deadline {
        return Err(timeout(op));
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| cannot("run", &program, &error))?;
    let (tx, rx) = mpsc::channel();
    let drained = drain(child.stdout.take(), Pipe::Out, &tx)
        .and_then(|()| drain(child.stderr.take(), Pipe::Err, &tx));
    drop(tx);
    if let Err(error) = drained {
        reap(&mut child);
        return Err(cannot("read the output of", &program, &error));
    }
    let status = wait(&mut child, &program, op, deadline)?;
    let (stdout, stderr) = collect(&rx, &program, op, deadline)?;
    Ok(Ran {
        status,
        stdout,
        stderr,
    })
}

/// Run `command`, with stdin, stdout and stderr null, until it exits or `bound` passes.
fn run(mut command: Command, op: &str, bound: Duration) -> Result<ExitStatus, PaneError> {
    let program = program_of(&command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| cannot("run", &program, &error))?;
    wait(&mut child, &program, op, crate::deadline_after(bound))
}

/// The deadline loop: poll `child` until it exits or `deadline` passes. On the deadline, or
/// when its exit cannot be read, it is killed and reaped: `timeout` with `op`, or
/// `unavailable`.
fn wait(
    child: &mut Child,
    program: &str,
    op: &str,
    deadline: Instant,
) -> Result<ExitStatus, PaneError> {
    loop {
        let waited = child.try_wait();
        if let Ok(Some(status)) = waited {
            return Ok(status);
        }
        let now = Instant::now();
        if waited.is_ok() && now < deadline {
            std::thread::sleep(POLL.min(deadline.saturating_duration_since(now)));
            continue;
        }
        reap(child);
        return Err(match waited {
            Err(error) => cannot("wait for", program, &error),
            Ok(_) => timeout(op),
        });
    }
}

/// Kill `child` and wait for it, so no zombie is left. Either may fail only when the child
/// is already gone.
fn reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read `pipe` to its end on a thread of its own and send what was kept of it, tagged
/// `which`. A missing pipe sends an empty output at once.
fn drain(
    pipe: Option<impl Read + Send + 'static>,
    which: Pipe,
    tx: &Sender<(Pipe, Vec<u8>)>,
) -> io::Result<()> {
    let tx = tx.clone();
    let Some(mut pipe) = pipe else {
        // The receiver is alive: `capture` holds it until both outputs are in.
        let _ = tx.send((which, Vec::new()));
        return Ok(());
    };
    std::thread::Builder::new()
        .name("opencode-adapter-drain".to_owned())
        .spawn(move || {
            let mut kept = Vec::new();
            // A read error ends the drain with what was read; the exit status decides.
            let _ = (&mut pipe).take(MAX_OUTPUT).read_to_end(&mut kept);
            // The rest is read and dropped, so the child never waits on a full pipe.
            let _ = io::copy(&mut pipe, &mut io::sink());
            // The receiver is gone only when `capture` gave up.
            let _ = tx.send((which, kept));
        })
        .map(drop)
}

/// Both outputs, each once its pipe closed, by `deadline`.
fn collect(
    rx: &Receiver<(Pipe, Vec<u8>)>,
    program: &str,
    op: &str,
    deadline: Instant,
) -> Result<(String, String), PaneError> {
    let (mut out, mut err) = (None, None);
    while out.is_none() || err.is_none() {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok((Pipe::Out, bytes)) => out = Some(bytes),
            Ok((Pipe::Err, bytes)) => err = Some(bytes),
            Err(RecvTimeoutError::Timeout) => return Err(timeout(op)),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(PaneError::Unavailable {
                    what: format!("cannot read the output of {program}: it was lost"),
                })
            }
        }
    }
    let text =
        |bytes: Option<Vec<u8>>| String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned();
    Ok((text(out), text(err)))
}

/// The program of `command`, as a message names it.
fn program_of(command: &Command) -> String {
    Path::new(command.get_program()).display().to_string()
}

/// `unavailable`: the adapter cannot do `doing` with `program`, for `error`.
fn cannot(doing: &str, program: &str, error: &dyn Display) -> PaneError {
    PaneError::Unavailable {
        what: crate::one_line(&format!("cannot {doing} {program}: {error}")),
    }
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}
