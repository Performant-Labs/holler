#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #696
//! What the OpenCode adapter keeps of a `tmux` call's output (#696): at most the first 64 KiB
//! of each stream, with the rest read and dropped. `attach_test.rs` runs the committed fake
//! tmux, whose answers are short; this file runs a `tmux` that is a `/bin/sh` script printing
//! far more than the cap, and reads the cap off the one place the adapter quotes `tmux`'s
//! output: the first line of its stderr in an `unavailable`.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName};

/// The cap on each stream of a `tmux` call: 64 KiB.
const CAP: usize = 64 << 10;

/// An adapter whose `tmux` is `tmux`, resolving every pane to the session `demo-c1r1`.
fn harness(tmux: PathBuf, dir: &Path) -> OpenCodeHarness {
    OpenCodeHarness::new(OpenCodeConfig {
        opencode_bin: PathBuf::from("/bin/oc"),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: tmux,
            socket: TmuxSocket::Path(dir.join("s")),
        },
        tui_session: Arc::new(|_: &PaneId| Ok(PaneName::parse("demo-c1r1").unwrap())),
        timeouts: Timeouts {
            call: Duration::from_secs(10),
            request: Duration::from_secs(1),
            health: Duration::from_secs(1),
            settle: Duration::from_secs(1),
            compact: Duration::from_secs(20),
        },
    })
}

/// Write an executable script, then run it once until it executes: a script just written
/// can fail with `ETXTBSY` while a parallel test's fork still holds the write descriptor.
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

/// `tmux` stderr over the cap is cut at the cap, and the call still finishes: a `tmux` that
/// writes 3 MiB of one marker byte on one line, then fails, is `unavailable` quoting exactly
/// 64 KiB of it. A cap of any other size, none, or a reader that stopped at the cap (the
/// child would block on a full pipe and time out) each change that.
#[test]
fn tmux_stderr_is_cut_at_64_kib_and_the_rest_is_drained_not_waited_on() {
    let dir = tempfile::tempdir().unwrap();
    let tmux = dir.path().join("tmux");
    script(
        &tmux,
        "head -c 3145728 /dev/zero | tr '\\0' Z >&2\necho >&2\nexit 1",
    );
    let harness = harness(tmux, dir.path());
    let started = Instant::now();
    let result = harness.shown_session(&PaneId::new("w9:p1"), &HarnessEndpoint::Port(4096));
    let what = match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("expected Unavailable, got {other:?}"),
    };
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(what.matches('Z').count(), CAP, "kept {} bytes", what.len());
    assert!(what.len() < CAP + 200, "{} bytes", what.len());
}
