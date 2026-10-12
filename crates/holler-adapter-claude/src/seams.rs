//! The three real seams (ADR-0021 section 15 item 15): `claude agents --json`, a line to a unix
//! socket, and `claude auth status`. Each returns the raw I/O; the adapter parses it, so the
//! hermetic tests fake exactly these.
//!
//! Both `claude` runs go through [`crate::exec::capture`], never a shell, each within its own
//! bound, with every variable that changes the login removed from their environment
//! (`HarnessKind::login_variables`), so `auth status` answers for the login the panes' processes
//! use, which their argv's `env -u` gives them. Nothing either prints is logged; only the
//! adapter's parse reads it, and only what it needs.

use std::io::{Read as _, Write as _};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use holler_pane::pane::HarnessKind;
use holler_pane::PaneError;

use crate::exec::capture;
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
