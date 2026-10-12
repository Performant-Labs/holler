#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #696
//! The real `claude` seams, run against a fake `claude` that is a `/bin/sh` script (#696).
//!
//! `hermetic_test.rs` fakes the three seams, so nothing there runs `ClaudeAgents` or
//! `ClaudeLogin`, the two seams that start a program. This file runs them over the shared
//! bounded runner of `holler-pane` through the adapter's public port (`locate_process` is
//! `claude agents --json`, `login_status` is `claude auth status`), and pins what the
//! runner's `Spec` for `claude` means to a caller: its bound, its program-only error text,
//! its discarded stderr and its 1 MiB stdout cap. The fake is a script, not `claude`, and
//! nothing here logs in or reaches the network.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use holler_adapter_claude::{ClaudeConfig, ClaudeHarness, Timeouts};
use holler_pane::ports::LoginStatus;
use holler_pane::{HarnessPort, PaneError, PaneName};

/// What a bound may overrun by on a loaded runner.
const SLACK: Duration = Duration::from_secs(2);
/// The cap on `claude`'s stdout: 1 MiB.
const CAP: usize = 1 << 20;
const SECRET: &str = "HLR_SECRET_STDERR_696";

/// A fake `claude` in a scratch directory, and an adapter over it with `timeouts`.
struct Rig {
    dir: tempfile::TempDir,
    harness: ClaudeHarness,
}

impl Rig {
    /// A `claude` that runs `body` for every call.
    fn new(body: &str, timeouts: Timeouts) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("claude");
        script(&bin, body);
        Self::over(dir, bin, timeouts)
    }

    fn over(dir: tempfile::TempDir, bin: PathBuf, timeouts: Timeouts) -> Self {
        let mut config = ClaudeConfig::new(bin, dir.path().join("config"), dir.path().join("s"));
        config.timeouts = timeouts;
        Self {
            dir,
            harness: ClaudeHarness::new(config),
        }
    }

    fn agents(&self) -> Result<Option<u32>, PaneError> {
        let name = PaneName::parse("demo-c1r1").unwrap();
        self.harness.locate_process(&name, "any-session")
    }

    fn login(&self) -> Result<LoginStatus, PaneError> {
        self.harness.login_status()
    }

    fn pid_file(&self) -> PathBuf {
        self.dir.path().join("pid")
    }
}

/// Bounds short enough to wait for, for the two reads that start `claude`.
fn quick() -> Timeouts {
    Timeouts {
        agents: Duration::from_millis(300),
        login: Duration::from_millis(300),
        ..Timeouts::default()
    }
}

/// Write an executable script, then run it once until it executes: a script just written
/// can fail with `ETXTBSY` while a parallel test's fork still holds the write descriptor,
/// and the adapter must never meet that.
fn script(path: &Path, body: &str) {
    fs::write(
        path,
        format!("#!/bin/sh\n[ \"$1\" = --hlr-warmup ] && exit 0\n{body}\n"),
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(path).arg("--hlr-warmup").status() {
            Ok(status) => {
                assert!(status.success(), "{} warm-up: {status}", path.display());
                return;
            }
            Err(e) if e.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("{} warm-up: {e}", path.display()),
        }
    }
    panic!("{} stayed busy", path.display());
}

/// Whether `pid` is a process that exists (a zombie counts), via `kill -s 0`.
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", "--", &pid.to_string()])
        .env("LC_ALL", "C")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>) -> String {
    match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[test]
fn a_claude_that_hangs_times_out_the_method_that_asked_and_is_killed() {
    for (read, op) in [(0, "harness.agents"), (1, "harness.login_status")] {
        let rig = Rig::new(
            "echo $$ > \"$(dirname \"$0\")/pid\"\nexec sleep 30",
            quick(),
        );
        let started = Instant::now();
        let result = if read == 0 {
            rig.agents().map(|_| ())
        } else {
            rig.login().map(|_| ())
        };
        let took = started.elapsed();
        match result {
            Err(PaneError::Timeout { op: got }) => assert_eq!(got, op),
            other => panic!("{op}: expected Timeout, got {other:?}"),
        }
        assert!(
            took >= Duration::from_millis(250) && took < Duration::from_millis(300) + SLACK,
            "{op}: took {took:?}"
        );
        let pid: u32 = fs::read_to_string(rig.pid_file())
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(!alive(pid), "{op}: the hung claude {pid} was not reaped");
    }
}

#[test]
fn a_missing_claude_is_unavailable_naming_the_program_and_no_argument() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("hlr-696-no-such-claude");
    for what in [
        unavailable(Rig::over(tempfile::tempdir().unwrap(), bin.clone(), quick()).agents()),
        unavailable(Rig::over(tempfile::tempdir().unwrap(), bin.clone(), quick()).login()),
    ] {
        assert!(what.contains("hlr-696-no-such-claude"), "{what:?}");
        assert!(
            !what.contains("agents") && !what.contains("auth"),
            "{what:?}"
        );
    }
}

#[test]
fn what_claude_prints_on_stderr_is_never_quoted() {
    let rig = Rig::new(&format!("echo {SECRET} >&2\nexit 1"), quick());
    // `agents --json` failing is reported by its status alone.
    let what = unavailable(rig.agents());
    assert!(what.contains("claude agents --json failed"), "{what:?}");
    assert!(!what.contains(SECRET), "{what:?}");
    // `auth status` answering nothing on stdout is unavailable, again without the stderr.
    let what = unavailable(rig.login());
    assert!(what.contains("answered nothing"), "{what:?}");
    assert!(!what.contains(SECRET), "{what:?}");
}

/// The cap is 1 MiB of stdout, and what is past it is read and dropped: an answer that is a
/// whole JSON document in exactly its first MiB, followed by 2 MiB of noise, is the
/// document. Without the cap the noise follows the document and the answer does not parse;
/// with a failing cap the call is an error; and a reader that stopped at the cap would let
/// the child block on a full pipe and time out.
#[test]
fn claudes_stdout_is_cut_at_one_mib_and_the_rest_is_drained_not_waited_on() {
    let head = r#"{"authMethod":"claude.ai","pad":""#;
    let pad = CAP - head.len() - 2;
    let body = format!(
        "printf '%s' '{head}'\nhead -c {pad} /dev/zero | tr '\\0' A\nprintf '\"}}'\n\
         head -c {} /dev/zero | tr '\\0' x\nexit 0",
        2 * CAP
    );
    let rig = Rig::new(
        &body,
        Timeouts {
            login: Duration::from_secs(10),
            ..quick()
        },
    );
    let started = Instant::now();
    let status = rig.login();
    assert!(
        matches!(status, Ok(LoginStatus::Ok)),
        "expected the first MiB to be the whole answer, got {status:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "{:?}",
        started.elapsed()
    );
}
