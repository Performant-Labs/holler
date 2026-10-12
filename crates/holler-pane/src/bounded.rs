//! The one bounded subprocess runner (#696): run a child under a deadline, kill it on the
//! deadline, and capture its output. The health probe ([`crate::run_probe`]) and the host,
//! OpenCode and Claude adapters all run their children through [`run`]; none keeps a copy.
//!
//! It is this crate's one direct side effect (ADR-0021 section 5): it starts a process with
//! the standard library alone, so it takes no new dependency, and a group kill goes through
//! the `kill` program because `std` cannot signal a process group.
//!
//! **What a consumer chooses**, in a [`Spec`], is exactly where the four former copies
//! differed, so each keeps its behaviour: what is kept of stdout and of stderr (discarded,
//! kept up to a cap with the rest read and dropped, or kept up to a cap with one byte more an
//! error), how far the kill reaches (the child alone, or the child's own process group) and
//! how often the exit is polled. Two things are fixed for every consumer, because they were
//! never a choice: stdin is null, and the program is run as the caller built the [`Command`]
//! (its arguments, environment and directory), never through a shell.
//!
//! **What is not a choice, on purpose (#696, Decision 3).** The kill is `KILL` with no `TERM`
//! grace, and a group kill runs `kill` from `PATH`, with no seam for a fake program: no
//! consumer needs either. The host adapter's own signals to the panes' process groups (`TERM`,
//! the grace, `kill -s 0`, and its fake-`kill` seam) are tmux semantics, not a runner, and stay
//! in the host adapter, which runs its `kill` program through [`run`] like any other child.
//!
//! **The order, and the pid-reuse rule.** Every kept stream is read to its end first, and the
//! exit is awaited only after that. A child's pid (and, under [`KillScope::Group`], its
//! group's id) cannot be taken by another process while the child is unreaped, alive or a
//! zombie. So the runner signals only while `try_wait` has not yet returned the child's
//! status, and once the status is seen it never signals. An exit status that cannot be read
//! (a `try_wait` error: something else in this process reaped the child) is
//! [`RunError::WaitFailed`], and no signal follows it. A change that reaps the child before
//! it may kill (a `try_wait` before the end of the output, then a kill) reopens the race.
//!
//! **The bound.** One deadline for the whole run. A deadline that has already passed starts
//! nothing. When it passes during the run, or a failing cap is passed, the child is killed
//! (its group first, under [`KillScope::Group`]) and reaped, within one cleanup budget of
//! 1 s: [`run`] returns at the deadline plus at most that budget (normally a few ms). A child
//! that exits while something it started still holds an output open is
//! [`RunError::Expired`] once the deadline passes, never a [`Ran`].
//!
//! **Long-lived callers.** The reader threads are detached, so a kill never waits on a pipe.
//! A child whose descendant escapes the kill (any descendant under [`KillScope::Child`];
//! under [`KillScope::Group`], one that left the group: `setsid`, a double fork) and keeps an
//! output open leaves one reader thread and its pipe blocked after [`run`] returns, until that
//! descendant closes the pipe. A long-lived caller can so leak one thread per such run.
//!
//! **No argument is ever echoed** (a check or a tmux argv may carry a token): a [`RunError`]
//! holds at most the `io::Error` of the OS, never the program's arguments or output.

use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// The one budget the kill of the child (its group's too) and its reap share, once the runner
/// gives up.
const CLEANUP: Duration = Duration::from_secs(1);

/// What to do with one of the child's output streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// Not read: the stream goes to the null device and the result holds no bytes of it.
    Discard,
    /// Read to its end, up to `cap` bytes; `over` says what a byte past `cap` means.
    Keep { cap: usize, over: Over },
}

/// What a stream that passes its cap means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// Keep the first `cap` bytes, read and drop the rest; the run goes on. A read error ends
    /// the capture with what was read, and the exit status decides.
    Truncate,
    /// An error ([`RunError::Overflow`]): the child is killed and the run ends at once. A read
    /// error is an error too ([`RunError::OutputLost`]).
    Fail,
}

/// How far the kill at the deadline (or on an overflow) reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillScope {
    /// The child alone, which stays in the caller's process group.
    Child,
    /// The child in a process group of its own (on Unix), and the whole group is killed with
    /// `KILL` through `kill` from `PATH`, then the child. If `kill` cannot be run, only the
    /// child is ended, and the run still returns within the same bound. Off Unix only the
    /// child is ended.
    Group,
}

/// How one run is bounded and what it captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    /// What to do with stdout.
    pub stdout: Capture,
    /// What to do with stderr.
    pub stderr: Capture,
    /// How far the kill reaches.
    pub kill: KillScope,
    /// How often the runner polls `Child::try_wait`.
    pub poll: Duration,
}

/// A child that ended: its status and what was captured of its output.
#[derive(Debug)]
pub struct Ran {
    /// How it exited. A non-zero exit is a [`Ran`], not an error.
    pub status: ExitStatus,
    /// Its stdout, as captured (empty under [`Capture::Discard`]).
    pub stdout: Vec<u8>,
    /// Its stderr, as captured (empty under [`Capture::Discard`]).
    pub stderr: Vec<u8>,
}

/// Why a run produced no [`Ran`]. Each consumer maps it to its own text.
#[derive(Debug)]
pub enum RunError {
    /// The deadline had passed (nothing was started) or passed during the run (the child
    /// was killed).
    Expired,
    /// The program could not be started.
    Spawn(io::Error),
    /// The thread that reads an output stream could not be started (the child was killed).
    ReaderFailed(io::Error),
    /// The exit status could not be read. No signal was sent (the pid-reuse rule).
    WaitFailed(io::Error),
    /// The output could not be read: a read under [`Over::Fail`] failed, or a reader's answer
    /// was lost (the child was killed).
    OutputLost,
    /// A stream with [`Over::Fail`] passed its cap (the child was killed).
    Overflow,
}

/// Which stream a reader read.
#[derive(Clone, Copy)]
enum Stream {
    Out,
    Err,
}

/// How a reader's stream ended, when it did not end with its bytes.
enum Cut {
    /// It passed a failing cap; reading stopped one byte past it.
    Overflow,
    /// A read under a failing cap failed.
    Failed,
}

/// What one reader sends: its stream, and the bytes kept of it or why there are none.
type Reading = (Stream, Result<Vec<u8>, Cut>);

/// Run `command` until it exits or `deadline` passes, under `spec`.
///
/// stdin is null and the streams are set as `spec` says; everything else (the program, its
/// arguments, environment and directory) is the caller's. The kept streams are read to their
/// end first, then the exit is awaited (the module's pid-reuse rule). See the module for the
/// bound and what each [`RunError`] means.
///
/// # Errors
///
/// [`RunError`], when the run has no exit status and complete output to answer with.
pub fn run(command: &mut Command, spec: &Spec, deadline: Instant) -> Result<Ran, RunError> {
    if Instant::now() >= deadline {
        return Err(RunError::Expired);
    }
    let mut child = spawn(command, spec).map_err(RunError::Spawn)?;
    let output = match start_readers(&mut child, spec) {
        Ok(readings) => read_all(&readings, spec, deadline),
        Err(e) => Err(RunError::ReaderFailed(e)),
    };
    match output {
        Ok((stdout, stderr)) => wait_for_exit(&mut child, spec, deadline).map(|status| Ran {
            status,
            stdout,
            stderr,
        }),
        Err(e) => {
            kill_and_reap(&mut child, spec);
            Err(e)
        }
    }
}

/// Start `command` with stdin null and each stream piped or discarded per `spec`, in a
/// process group of its own under [`KillScope::Group`] (on Unix), so the group id is its pid.
fn spawn(command: &mut Command, spec: &Spec) -> io::Result<Child> {
    command
        .stdin(Stdio::null())
        .stdout(stdio(spec.stdout))
        .stderr(stdio(spec.stderr));
    #[cfg(unix)]
    if spec.kill == KillScope::Group {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()
}

/// The `Stdio` of a stream captured as `capture`.
fn stdio(capture: Capture) -> Stdio {
    match capture {
        Capture::Discard => Stdio::null(),
        Capture::Keep { .. } => Stdio::piped(),
    }
}

/// Start one reader per kept stream, all sending on one channel, so a child that floods one
/// stream while the other is quiet never blocks. A discarded stream starts none.
fn start_readers(child: &mut Child, spec: &Spec) -> io::Result<Receiver<Reading>> {
    let (sender, receiver) = mpsc::channel();
    if let Capture::Keep { cap, over } = spec.stdout {
        let pipe = child.stdout.take().ok_or_else(no_pipe)?;
        start_reader(pipe, Stream::Out, cap, over, &sender)?;
    }
    if let Capture::Keep { cap, over } = spec.stderr {
        let pipe = child.stderr.take().ok_or_else(no_pipe)?;
        start_reader(pipe, Stream::Err, cap, over, &sender)?;
    }
    Ok(receiver)
}

/// A piped stream the child does not have: unreachable with a `Stdio::piped()`, and read as a
/// reader that could not start.
fn no_pipe() -> io::Error {
    io::Error::other("the child has no pipe for a kept stream")
}

/// Start the thread that reads `pipe` per `cap` and `over` and sends what it found, tagged
/// `stream`. It is detached on purpose: the runner never joins it, so a kill never waits on
/// its pipe.
fn start_reader(
    pipe: impl Read + Send + 'static,
    stream: Stream,
    cap: usize,
    over: Over,
    sender: &Sender<Reading>,
) -> io::Result<()> {
    let sender = sender.clone();
    let name = match stream {
        Stream::Out => "holler-run-stdout",
        Stream::Err => "holler-run-stderr",
    };
    thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            // The runner may have stopped listening (it gave up); that is not this thread's error.
            let _ = sender.send((stream, read_capped(pipe, cap, over)));
        })
        .map(drop)
}

/// Read `pipe` to its end under `cap`: past it, [`Over::Truncate`] keeps the first `cap`
/// bytes and reads and drops the rest (so the child never waits on a full pipe), and
/// [`Over::Fail`] stops one byte past it, which is enough to know.
fn read_capped(mut pipe: impl Read, cap: usize, over: Over) -> Result<Vec<u8>, Cut> {
    let limit = u64::try_from(cap).unwrap_or(u64::MAX);
    let mut kept = Vec::new();
    match over {
        Over::Truncate => {
            // A read error ends the capture with what was read; the exit status decides.
            let _ = (&mut pipe).take(limit).read_to_end(&mut kept);
            let _ = io::copy(&mut pipe, &mut io::sink());
            Ok(kept)
        }
        Over::Fail => match pipe.take(limit.saturating_add(1)).read_to_end(&mut kept) {
            Ok(_) if kept.len() > cap => Err(Cut::Overflow),
            Ok(_) => Ok(kept),
            Err(_) => Err(Cut::Failed),
        },
    }
}

/// Every kept stream to its end, by `deadline`: `(stdout, stderr)`, a discarded one empty.
/// Until this returns the child is never reaped, so every error here may still kill it.
fn read_all(
    readings: &Receiver<Reading>,
    spec: &Spec,
    deadline: Instant,
) -> Result<(Vec<u8>, Vec<u8>), RunError> {
    let pending = |capture: Capture| match capture {
        Capture::Discard => Some(Vec::new()),
        Capture::Keep { .. } => None,
    };
    let (mut out, mut err) = (pending(spec.stdout), pending(spec.stderr));
    while out.is_none() || err.is_none() {
        let (stream, read) =
            match readings.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(reading) => reading,
                Err(RecvTimeoutError::Timeout) => return Err(RunError::Expired),
                Err(RecvTimeoutError::Disconnected) => return Err(RunError::OutputLost),
            };
        let bytes = read.map_err(|cut| match cut {
            Cut::Overflow => RunError::Overflow,
            Cut::Failed => RunError::OutputLost,
        })?;
        match stream {
            Stream::Out => out = Some(bytes),
            Stream::Err => err = Some(bytes),
        }
    }
    Ok((out.unwrap_or_default(), err.unwrap_or_default()))
}

/// The output has ended: wait for the exit until `deadline`. Past it the child is still
/// unreaped, so it can be killed; a status that cannot be read is never followed by a signal,
/// since the child may have been reaped elsewhere and its pid reused.
fn wait_for_exit(
    child: &mut Child,
    spec: &Spec,
    deadline: Instant,
) -> Result<ExitStatus, RunError> {
    match wait_until(child, deadline, spec.poll) {
        Ok(Some(status)) => Ok(status),
        Ok(None) => {
            kill_and_reap(child, spec);
            Err(RunError::Expired)
        }
        Err(e) => Err(RunError::WaitFailed(e)),
    }
}

/// Kill the child (its whole process group first, under [`KillScope::Group`]) and reap it,
/// all within the one cleanup budget. Called only while the child is unreaped (the pid-reuse
/// rule); a child still unreaped when the budget runs out is left, a zombie, for the caller's
/// exit.
fn kill_and_reap(child: &mut Child, spec: &Spec) {
    let budget = Instant::now()
        .checked_add(CLEANUP)
        .unwrap_or_else(Instant::now);
    if spec.kill == KillScope::Group {
        kill_group(child.id(), budget, spec.poll);
    }
    // The child too, in case the group kill could not run; an error means it is gone already.
    let _ = child.kill();
    let _ = wait_until(child, budget, spec.poll);
}

/// Signal the process group `group` with `KILL` through `kill -s KILL -- -<group>`, waiting for
/// the `kill` program within `budget` and ignoring its outcome (the group may be gone already).
/// A `kill` still running past the budget is killed itself, and reaped if it has ended by then.
fn kill_group(group: u32, budget: Instant, poll: Duration) {
    #[cfg(unix)]
    {
        let target = format!("-{group}");
        let killer = Command::new("kill")
            .args(["-s", "KILL", "--", target.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut killer) = killer {
            if let Ok(None) = wait_until(&mut killer, budget, poll) {
                let _ = killer.kill();
                let _ = killer.try_wait();
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (group, budget, poll);
    }
}

/// Poll `child` every `poll` until it exits (its status, and it is reaped) or `until` passes
/// (`None`, and it is not).
fn wait_until(child: &mut Child, until: Instant, poll: Duration) -> io::Result<Option<ExitStatus>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(None);
        }
        thread::sleep(left.min(poll));
    }
}
