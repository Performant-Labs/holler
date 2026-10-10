//! Running one child process under a deadline (Decision 2 of #641), and the one `kill`
//! call every signal of the adapter goes through (Decision 4).
//!
//! A child gets a null stdin. Its stdout and stderr are drained on two threads, so a
//! child that writes more than a pipe holds cannot block, and its exit is polled with
//! `try_wait` until the deadline. On the deadline the child is killed and reaped, and the
//! call is [`Failed::Expired`]: a hung call is reported, never waited on. A child that
//! exits while something it started still holds its pipes is `Expired` too once the
//! deadline passes, so no call outlives its bound.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::tmux::Pid;

/// How long a running child sleeps between two polls of its exit.
const POLL: Duration = Duration::from_millis(2);

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

/// Which pipe a drained output came from.
#[derive(Clone, Copy)]
enum Stream {
    Out,
    Err,
}

/// Run `command`, whose program and arguments are set, to its exit or to `deadline`.
/// Nothing is spawned once `deadline` has passed.
pub(crate) fn run(command: &mut Command, deadline: Instant) -> Result<Ran, Failed> {
    let program = Path::new(command.get_program()).display().to_string();
    if Instant::now() >= deadline {
        return Err(Failed::Expired);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| Failed::Unavailable(format!("{program}: {e}")))?;
    let (tx, rx) = mpsc::channel();
    let drained = drain(child.stdout.take(), Stream::Out, &tx)
        .and_then(|()| drain(child.stderr.take(), Stream::Err, &tx));
    drop(tx);
    if let Err(e) = drained {
        reap(&mut child);
        return Err(Failed::Unavailable(format!(
            "{program}: no thread to read its output: {e}"
        )));
    }
    let status = wait(&mut child, deadline, &program)?;
    let (stdout, stderr) = collect(&rx, deadline, &program)?;
    Ok(Ran {
        status,
        stdout,
        stderr,
    })
}

/// Read `pipe` to its end on a thread of its own and send what it held, tagged
/// `stream`. A missing pipe sends an empty output at once.
fn drain(
    pipe: Option<impl Read + Send + 'static>,
    stream: Stream,
    tx: &Sender<(Stream, Vec<u8>)>,
) -> std::io::Result<()> {
    let tx = tx.clone();
    let Some(mut pipe) = pipe else {
        // The receiver is alive: `run` holds it until both outputs are in.
        let _ = tx.send((stream, Vec::new()));
        return Ok(());
    };
    thread::Builder::new()
        .name("holler-host-drain".to_owned())
        .spawn(move || {
            let mut bytes = Vec::new();
            // A read error ends the drain with what was read; the exit status decides.
            let _ = pipe.read_to_end(&mut bytes);
            // The receiver is gone only when `run` gave up at the deadline.
            let _ = tx.send((stream, bytes));
        })
        .map(drop)
}

/// Poll `child` until it exits or `deadline` passes; on the deadline kill and reap it.
fn wait(child: &mut Child, deadline: Instant, program: &str) -> Result<ExitStatus, Failed> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(e) => {
                reap(child);
                return Err(Failed::Unavailable(format!("{program}: {e}")));
            }
        }
        let now = Instant::now();
        if now >= deadline {
            reap(child);
            return Err(Failed::Expired);
        }
        thread::sleep(POLL.min(deadline.saturating_duration_since(now)));
    }
}

/// Kill `child` and wait for it, so no zombie is left. Either may fail only when the
/// child is already gone.
fn reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Both outputs, each once its pipe closed, by `deadline`.
fn collect(
    rx: &Receiver<(Stream, Vec<u8>)>,
    deadline: Instant,
    program: &str,
) -> Result<(String, String), Failed> {
    let (mut out, mut err) = (None, None);
    while out.is_none() || err.is_none() {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok((Stream::Out, bytes)) => out = Some(bytes),
            Ok((Stream::Err, bytes)) => err = Some(bytes),
            Err(RecvTimeoutError::Timeout) => return Err(Failed::Expired),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Failed::Unavailable(format!(
                    "{program}: its output was lost"
                )))
            }
        }
    }
    let text =
        |bytes: Option<Vec<u8>>| String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned();
    Ok((text(out), text(err)))
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
