#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! The Claude Code adapter against a real, scratch Claude Code in a private tmux server (#831,
//! ADR-0021 section 15 item 15). It is the guard of the delivery frame, which is **Read** and not
//! documented: run it on every Claude Code upgrade.
//!
//! Opt-in: the test is `#[ignore]` and returns at once, saying why, unless `HOLLER_TEST_CLAUDE=1`;
//! opted in, a missing `claude` or `tmux` is a failure. Run:
//!
//! ```text
//! HOLLER_TEST_CLAUDE=1 cargo test -p holler-adapter-claude --test real_claude_test \
//!   -- --ignored --test-threads=1
//! ```
//!
//! - `HOLLER_TEST_CLAUDE_BIN`: the `claude` program, an absolute path (default: `claude` on
//!   `PATH`, resolved by the adapter's configuration);
//! - `HOLLER_TEST_CLAUDE_CWD`: a directory Claude Code already trusts (default: a fresh scratch
//!   directory, which Claude Code may first ask to trust: the launch then waits on `dialog open`
//!   and the suite fails, which is the adapter's `harness-dialog-open`, not a test bug).
//!
//! What it touches: a private tmux server on a scratch socket, the panes `demo-c1r1` and
//! `demo-c2r1` in it, and one model turn (case `resume-after-a-prompt-keeps-the-session` sends one
//! short prompt, which is what proves the frame reached the model: only a prompted session has a
//! transcript to resume). It reads `claude agents --json` (every process of the user, matched by
//! a fresh session id) and `claude auth status`'s `authMethod`; it never reads, copies or prints
//! the login, and never touches a pane or a session it did not start.

use std::path::{Path, PathBuf};
use std::process::Command;

use holler_adapter_claude::{ClaudeConfig, ClaudeHarness};
use holler_pane::{Argv, PaneError, PaneId, PaneName};
use holler_pane_testkit::conformance::claude::{run_claude_conformance, ClaudeRig};

const OPT_IN: &str = "HOLLER_TEST_CLAUDE";

fn opted_in(test: &str) -> bool {
    if std::env::var(OPT_IN).as_deref() == Ok("1") {
        return true;
    }
    eprintln!("{test}: skipped; set {OPT_IN}=1 to run it against a scratch Claude Code");
    false
}

/// A scratch tmux server and directories, kept for one case and killed with it.
struct Scratch {
    dir: tempfile::TempDir,
    socket: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("tmux.sock");
        Self { dir, socket }
    }

    fn cwd(&self) -> String {
        std::env::var("HOLLER_TEST_CLAUDE_CWD")
            .unwrap_or_else(|_| self.dir.path().to_string_lossy().into_owned())
    }

    fn tmux(socket: &Path) -> Command {
        let mut command = Command::new("tmux");
        command.arg("-S").arg(socket);
        command
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = Self::tmux(&self.socket).arg("kill-server").status();
    }
}

/// The adapter with its real seams, over the user's own Claude Code configuration directory (a
/// transcript's existence is all it checks there) and a scratch socket directory.
fn harness(scratch: &Scratch) -> ClaudeHarness {
    let bin = std::env::var("HOLLER_TEST_CLAUDE_BIN").unwrap_or_else(|_| "claude".to_owned());
    let home = std::env::var("HOME").expect("HOME: the login travels with it");
    let config_dir = Path::new(&home).join(".claude");
    let socket_dir = scratch.dir.path().join("holler-state").join("claude");
    ClaudeHarness::new(ClaudeConfig::new(bin, config_dir, socket_dir))
}

/// Start `argv` (never through a shell) as the only process of a new tmux session `name`.
fn launch(socket: &Path, cwd: &str, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
    let status = Scratch::tmux(socket)
        .args(["new-session", "-d", "-s", name.as_str(), "-c", cwd])
        .args(argv.as_slice())
        .status()
        .map_err(|e| PaneError::Unavailable {
            what: format!("tmux: {e}"),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(PaneError::Unavailable {
            what: format!("tmux new-session for {name}"),
        })
    }
}

/// End the tmux session `name` and the process in it.
fn stop(socket: &Path, name: &PaneName) -> Result<(), PaneError> {
    let _ = Scratch::tmux(socket)
        .args(["kill-session", "-t", name.as_str()])
        .status();
    Ok(())
}

#[test]
#[ignore = "needs claude and tmux: HOLLER_TEST_CLAUDE=1 cargo test -p holler-adapter-claude --test real_claude_test -- --ignored --test-threads=1"]
fn the_adapter_passes_the_claude_conformance_suite_against_a_scratch_claude_code() {
    if !opted_in("real claude conformance") {
        return;
    }
    // One scratch tmux server for the run; each case's guard ends its panes' sessions.
    let scratch = Scratch::new();
    let cwd = scratch.cwd();
    let panes = [
        PaneName::parse("demo-c1r1").unwrap(),
        PaneName::parse("demo-c2r1").unwrap(),
    ];
    let result = run_claude_conformance(
        || {
            let rig = ClaudeRig {
                panes: panes.clone(),
                pane_ids: [
                    PaneId::new("scratch:demo-c1r1"),
                    PaneId::new("scratch:demo-c2r1"),
                ],
                cwd: cwd.clone(),
            };
            let guard = CaseGuard {
                socket: scratch.socket.clone(),
                panes: panes.clone(),
            };
            (harness(&scratch), rig, guard)
        },
        |_: &ClaudeHarness, name: &PaneName, argv: &Argv| launch(&scratch.socket, &cwd, name, argv),
        |_: &ClaudeHarness, name: &PaneName| stop(&scratch.socket, name),
    );
    assert_eq!(
        result,
        Ok(()),
        "every case holds against a real Claude Code"
    );
}

/// Ends a case's panes' tmux sessions when the case is over, whatever it left running.
struct CaseGuard {
    socket: PathBuf,
    panes: [PaneName; 2],
}

impl Drop for CaseGuard {
    fn drop(&mut self) {
        for pane in &self.panes {
            let _ = stop(&self.socket, pane);
        }
    }
}
