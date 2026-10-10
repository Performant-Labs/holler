//! Running a helper program to its end within a bound (private; decision 11 of the brief).
//!
//! In part 1 of #642 the one helper is `kill`: when `serve`'s deadline passes before the
//! server it started answers, [`kill_group`] sends SIGKILL to that server's process group
//! through the `kill` program, not a raw system call. Part 2 adds what its tmux calls read
//! (their output, and the classification of tmux's stderr) with those calls.
//! The runner is shaped like the host adapter's (#641), so that one runner for the adapters
//! (#696) is a move.

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use holler_pane::PaneError;

/// The program that signals a process group, found on the adapter's own `PATH`.
const KILL: &str = "kill";

/// How often a running helper is checked for its exit.
const POLL: Duration = Duration::from_millis(5);

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

/// Run `command`, with stdin, stdout and stderr null, until it exits or `bound` passes. A
/// program that cannot be started is `unavailable`, naming it. Past `bound` the child is
/// killed and reaped, and the answer is `timeout` with `op`.
fn run(mut command: Command, op: &str, bound: Duration) -> Result<ExitStatus, PaneError> {
    let program = Path::new(command.get_program()).display().to_string();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| PaneError::Unavailable {
            what: crate::one_line(&format!("cannot run {program}: {error}")),
        })?;
    let deadline = crate::deadline_after(bound);
    loop {
        let waited = child.try_wait();
        if let Ok(Some(status)) = waited {
            return Ok(status);
        }
        if waited.is_ok() && Instant::now() < deadline {
            std::thread::sleep(POLL);
            continue;
        }
        let _ = child.kill();
        let _ = child.wait();
        return Err(match waited {
            Err(error) => PaneError::Unavailable {
                what: crate::one_line(&format!("cannot wait for {program}: {error}")),
            },
            Ok(_) => PaneError::Timeout { op: op.to_owned() },
        });
    }
}
