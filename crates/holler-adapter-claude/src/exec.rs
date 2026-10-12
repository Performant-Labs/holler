//! Running `claude` to its end within a bound (private): `claude agents --json` and `claude
//! auth status`, the two reads the real seams make, never through a shell.
//!
//! [`capture`] runs a program to its exit and returns its status and what it printed on stdout.
//! It knows nothing of `claude`: what its answers mean is the adapter's. Each output is drained
//! on a thread of its own, so a child that writes more than a pipe holds cannot block, and at
//! most [`MAX_OUTPUT`] bytes of stdout are kept; stderr is read and dropped, never quoted (it
//! could carry anything the program prints). The deadline loop polls the child's exit with
//! `try_wait`; on the deadline the child is killed and reaped, and the answer is `timeout` with
//! the caller's `op`. A program that cannot be started is `unavailable`, naming it and none of
//! its arguments. The runner is shaped like the OpenCode and host adapters' (an adapter may not
//! depend on another), so that one runner for the adapters (#696) is a move.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use holler_pane::PaneError;

/// How often a running program is checked for its exit.
const POLL: Duration = Duration::from_millis(5);

/// The most of stdout [`capture`] keeps (1 MiB): a fleet's `agents --json` is far below it.
const MAX_OUTPUT: u64 = 1 << 20;

/// A program that ran to its exit: its status, and its stdout, decoded lossily.
pub(crate) struct Ran {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
}

/// Run `command` to its exit before `deadline`, with a null stdin, its stdout captured and its
/// stderr dropped. Nothing is started once `deadline` has passed. A child still running at
/// `deadline` is killed and reaped; that, a deadline that has passed, and a stdout still open
/// at `deadline` (something the child started holds it) are `timeout` with `op`.
pub(crate) fn capture(mut command: Command, op: &str, deadline: Instant) -> Result<Ran, PaneError> {
    let program = Path::new(command.get_program()).display().to_string();
    if Instant::now() >= deadline {
        return Err(timeout(op));
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| cannot("run", &program, &error))?;
    let stdout = match drain(child.stdout.take()) {
        Ok(stdout) => stdout,
        Err(error) => {
            reap(&mut child);
            return Err(cannot("read the output of", &program, &error));
        }
    };
    let status = wait(&mut child, &program, op, deadline)?;
    let stdout = collect(&stdout, &program, op, deadline)?;
    Ok(Ran { status, stdout })
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

/// Kill `child` and wait for it, so no zombie is left. Either may fail only when the child is
/// already gone.
fn reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read `pipe` to its end on a thread of its own; the receiver gets what was kept of it. A
/// missing pipe sends an empty output at once.
fn drain(pipe: Option<impl Read + Send + 'static>) -> io::Result<Receiver<Vec<u8>>> {
    let (tx, rx) = mpsc::channel();
    let Some(mut pipe) = pipe else {
        // The receiver is alive: the caller holds it.
        let _ = tx.send(Vec::new());
        return Ok(rx);
    };
    std::thread::Builder::new()
        .name("claude-adapter-drain".to_owned())
        .spawn(move || {
            let mut kept = Vec::new();
            // A read error ends the drain with what was read; the exit status decides.
            let _ = (&mut pipe).take(MAX_OUTPUT).read_to_end(&mut kept);
            // The rest is read and dropped, so the child never waits on a full pipe.
            let _ = io::copy(&mut pipe, &mut io::sink());
            // The receiver is gone only when `capture` gave up.
            let _ = tx.send(kept);
        })
        .map(|_| rx)
}

/// The output, once its pipe closed, by `deadline`.
fn collect(
    rx: &Receiver<Vec<u8>>,
    program: &str,
    op: &str,
    deadline: Instant,
) -> Result<String, PaneError> {
    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(RecvTimeoutError::Timeout) => Err(timeout(op)),
        Err(RecvTimeoutError::Disconnected) => Err(PaneError::Unavailable {
            what: format!("cannot read the output of {program}: it was lost"),
        }),
    }
}

/// `unavailable`: the adapter cannot do `doing` with `program`, for `error`.
fn cannot(doing: &str, program: &str, error: &io::Error) -> PaneError {
    PaneError::Unavailable {
        what: format!("cannot {doing} {program}: {}", error.kind()),
    }
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}
