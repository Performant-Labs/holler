//! The one bounded subprocess runner of the adapters and the health probe (#696).
//!
//! **STUB (T-red, #696).** This file lands the public types and the signature of [`run`]
//! so that `tests/bounded_runner_test.rs` fails on an assertion rather than on a missing
//! symbol (the #637 stub pattern). [`run`] answers [`RunError::OutputLost`] whatever it is
//! given. F replaces this file with the probe's `run_bounded` family, generalised: the
//! types below are the contract the tests were written against.
//!
//! The runner never echoes an argument of the command (a check or a tmux argv may carry a
//! token): a [`RunError`] names at most the `io::ErrorKind` or the `io::Error` of the OS.

use std::io;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

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
    /// Keep the first `cap` bytes, read and drop the rest; the run goes on.
    Truncate,
    /// An error ([`RunError::Overflow`]): the child is killed and the run ends at once.
    Fail,
}

/// How far the kill at the deadline (or on an overflow) reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillScope {
    /// The child alone, in the caller's process group.
    Child,
    /// The child in a process group of its own (on Unix), and the whole group is killed.
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

/// Why a run produced no [`Ran`].
#[derive(Debug)]
pub enum RunError {
    /// The deadline had passed (nothing was started) or passed during the run (the child
    /// was killed).
    Expired,
    /// The program could not be started.
    Spawn(io::Error),
    /// The thread that reads an output stream could not be started.
    ReaderFailed(io::Error),
    /// The exit status could not be read.
    WaitFailed(io::Error),
    /// The reader's answer was lost.
    OutputLost,
    /// A stream with [`Over::Fail`] passed its cap.
    Overflow,
}

/// Run `command` until it exits or `deadline` passes, under `spec`.
///
/// The stub answers [`RunError::OutputLost`] and starts nothing.
pub fn run(command: &mut Command, spec: &Spec, deadline: Instant) -> Result<Ran, RunError> {
    let _ = (command, spec, deadline);
    Err(RunError::OutputLost)
}
