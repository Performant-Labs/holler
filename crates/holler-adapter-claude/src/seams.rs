//! The three real seams (ADR-0021 section 15 item 15): `claude agents --json`, a line to a unix
//! socket, and `claude auth status`. Each returns the raw I/O; the adapter parses it, so the
//! hermetic tests fake exactly these.
//!
//! Both `claude` runs go through [`capture`], over the shared bounded runner of `holler-pane`
//! (#696), never a shell, each within its own bound, with every variable that changes the login removed from their environment
//! (`HarnessKind::login_variables`), so `auth status` answers for the login the panes' processes
//! use, which their argv's `env -u` gives them. Nothing either prints is logged; only the
//! adapter's parse reads it, and only what it needs.

use std::io::{Read as _, Write as _};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

use holler_pane::bounded::{self, Capture, KillScope, Over, RunError, Spec};
use holler_pane::pane::HarnessKind;
use holler_pane::PaneError;

use crate::{AgentsList, InboxWrite, LoginCheck};

/// The `op` of a `claude agents --json` run that timed out.
const OP_AGENTS: &str = "harness.agents";
/// The `op` of a `claude auth status` run that timed out: the one method that reads it.
const OP_LOGIN_STATUS: &str = "harness.login_status";
/// The `op` of a socket write that timed out.
const OP_SEND_PROMPT: &str = "harness.send_prompt";

/// How long a written socket is kept open for the process to read the line, as the spike's
/// writer did, unless the process closes it first.
const LINGER: Duration = Duration::from_millis(400);

/// `claude agents --json`, run within `bound`.
pub(crate) struct ClaudeAgents {
    pub(crate) bin: PathBuf,
    pub(crate) bound: Duration,
}

impl AgentsList for ClaudeAgents {
    fn agents_json(&self) -> Result<String, PaneError> {
        let mut command = claude(&self.bin);
        command.args(["agents", "--json"]);
        let ran = capture(command, OP_AGENTS, Instant::now() + self.bound)?;
        if ran.status.success() {
            return Ok(ran.stdout);
        }
        Err(PaneError::Unavailable {
            what: format!("claude agents --json failed ({})", ran.status),
        })
    }
}

/// `claude auth status`, run within `bound`. Its exit status is 1 when nobody is logged in,
/// which its answer says too, so its stdout is returned whatever the status, unless it is
/// empty.
pub(crate) struct ClaudeLogin {
    pub(crate) bin: PathBuf,
    pub(crate) bound: Duration,
}

impl LoginCheck for ClaudeLogin {
    fn auth_status_json(&self) -> Result<String, PaneError> {
        let mut command = claude(&self.bin);
        command.args(["auth", "status"]);
        let ran = capture(command, OP_LOGIN_STATUS, Instant::now() + self.bound)?;
        if ran.stdout.trim().is_empty() {
            return Err(PaneError::Unavailable {
                what: format!("claude auth status answered nothing ({})", ran.status),
            });
        }
        Ok(ran.stdout)
    }
}

/// How `claude` is run: stdin null, stdout kept up to 1 MiB (a fleet's `agents --json` is far
/// below it) with the rest read and dropped, stderr discarded and never quoted (it could carry
/// anything the program prints), the child alone killed and reaped at the deadline, and its
/// exit polled every 5 ms.
const CLAUDE_RUN: Spec = Spec {
    stdout: Capture::Keep {
        cap: 1 << 20,
        over: Over::Truncate,
    },
    stderr: Capture::Discard,
    kill: KillScope::Child,
    poll: Duration::from_millis(5),
};

/// A `claude` run that ended: its status, and its stdout, decoded lossily.
struct Ran {
    status: ExitStatus,
    stdout: String,
}

/// Run `command` to its exit before `deadline`, under [`CLAUDE_RUN`]. Nothing is started once
/// `deadline` has passed; that, a child still running at `deadline`, and a stdout still open at
/// `deadline` (something the child started holds it) are `timeout` with `op`. A program that
/// cannot be run is `unavailable`, naming it and none of its arguments.
fn capture(mut command: Command, op: &str, deadline: Instant) -> Result<Ran, PaneError> {
    let program = Path::new(command.get_program()).display().to_string();
    let ran = bounded::run(&mut command, &CLAUDE_RUN, deadline).map_err(|error| match error {
        RunError::Expired => PaneError::Timeout { op: op.to_owned() },
        RunError::Spawn(error) => cannot("run", &program, &error),
        RunError::ReaderFailed(error) => cannot("read the output of", &program, &error),
        RunError::WaitFailed(error) => cannot("wait for", &program, &error),
        // `Overflow` cannot happen: stdout is truncated, never failed.
        RunError::OutputLost | RunError::Overflow => PaneError::Unavailable {
            what: format!("cannot read the output of {program}: it was lost"),
        },
    })?;
    Ok(Ran {
        status: ran.status,
        stdout: String::from_utf8_lossy(&ran.stdout).into_owned(),
    })
}

/// `unavailable`: the adapter cannot do `doing` with `program`, for `error` (its kind alone).
fn cannot(doing: &str, program: &str, error: &std::io::Error) -> PaneError {
    PaneError::Unavailable {
        what: format!("cannot {doing} {program}: {}", error.kind()),
    }
}

/// One line to a unix socket, written within `bound`.
pub(crate) struct SocketLine {
    pub(crate) bound: Duration,
}

impl InboxWrite for SocketLine {
    /// Connect, write `line` and its newline, end the write side, then keep the connection open
    /// for at most [`LINGER`] while the process reads, as the spike's writer did. A socket that
    /// cannot be reached is `unavailable`, naming no part of the line.
    fn write_line(&self, socket: &Path, line: &str) -> Result<(), PaneError> {
        let failed = |doing: &str, error: std::io::Error| PaneError::Unavailable {
            what: format!("cannot {doing} the inbox socket: {}", error.kind()),
        };
        let mut stream = UnixStream::connect(socket).map_err(|e| failed("connect to", e))?;
        stream
            .set_write_timeout(Some(self.bound))
            .map_err(|e| failed("bound the write to", e))?;
        let mut framed = line.to_owned();
        framed.push('\n');
        stream.write_all(framed.as_bytes()).map_err(|error| {
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) {
                PaneError::Timeout {
                    op: OP_SEND_PROMPT.to_owned(),
                }
            } else {
                failed("write to", error)
            }
        })?;
        // The line is written: what follows only lets the process read it.
        let _ = stream.shutdown(Shutdown::Write);
        let until = Instant::now() + LINGER;
        let mut drained = [0_u8; 256];
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            let open = !left.is_zero()
                && stream.set_read_timeout(Some(left)).is_ok()
                && matches!(stream.read(&mut drained), Ok(read) if read > 0);
            if !open {
                break;
            }
        }
        Ok(())
    }
}

/// A command that runs `bin` with every variable that changes a Claude Code login removed from
/// its environment.
fn claude(bin: &Path) -> Command {
    let mut command = Command::new(bin);
    for variable in HarnessKind::Claude.login_variables() {
        command.env_remove(variable);
    }
    command
}
