//! Selftests for the shared test harness (story #138).
//!
//! These prove the *harness itself* behaves correctly, independent of the hub/
//! body (which are not implemented yet). They cover the three failure classes a
//! broken harness would be blind to:
//!
//! 1. a `StateDir` is actually removed on drop (no leaked temp dirs),
//! 2. `wait_for` returns `None` cleanly and fast on timeout (no hang),
//! 3. `kill_tree` reaps a child *and its grandchildren* (no orphaned agents).
//!
//! The `Hub::start` / `join` / `Body::*` helpers are deliberately *not*
//! exercised here: they drive `holler hub serve` / `body join` / `body run`,
//! which the hub & body stories implement. Those selftests land with those
//! stories.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149

mod support;

use support::{kill_tree, make_own_process_group, wait_for, StateDir};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// `StateDir::new` creates a real directory, and dropping it removes it — so a
/// panicking test does not leak state dirs into the temp area.
#[test]
fn state_dir_is_removed_on_drop() {
    let path = {
        let dir = StateDir::new();
        assert!(dir.path().is_dir(), "fresh StateDir is a real directory");
        // Sanity: the subpaths the harness exposes are derived from it.
        assert!(dir.hub().starts_with(dir.path()));
        assert!(dir.body().starts_with(dir.path()));
        dir.path().to_path_buf()
    }; // `dir` drops here.

    // Give the drop a beat to settle, then assert the dir is gone.
    wait_for(Duration::from_secs(2), || {
        (!path.exists()).then_some(())
    })
    .unwrap_or_else(|| panic!("StateDir was not removed on drop: {path:?}"));
}

/// `wait_for` must stop at the deadline and return `None` promptly. The spec's
/// bound is "well under `timeout + 100 ms`"; `wait_for` sleeps only the
/// *remaining* budget (capped at the poll interval), so the sleep can never
/// push the return past the deadline — only the final `check()` execution can
/// add latency. The hard bound below is the spec's 100 ms of slack plus one
/// poll interval (50 ms) to absorb OS wake-jitter on a loaded CI runner
/// (`thread::sleep` is a minimum: the OS may wake us late). A genuinely hung
/// check would run for *seconds*, so this bound still cleanly separates
/// "timed out" from "hung".
#[test]
fn wait_for_times_out_cleanly() {
    let timeout = Duration::from_millis(150);
    let started = Instant::now();
    let result: Option<i32> = wait_for(timeout, || None);
    let elapsed = started.elapsed();

    assert!(result.is_none(), "a never-satisfying check must time out to None");
    assert!(
        elapsed <= timeout + Duration::from_millis(150),
        "wait_for overshot its budget: took {elapsed:?} for a {timeout:?} timeout"
    );
    assert!(
        elapsed.as_millis() >= 50,
        "wait_for returned before its first poll tick (took {elapsed:?})"
    );
}

/// And the positive half: `wait_for` *does* return a value once the check
/// starts succeeding (it does not only ever time out).
#[test]
fn wait_for_returns_when_check_succeeds() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let started = Instant::now();
    let got = wait_for(Duration::from_secs(2), || {
        // Succeed on the 3rd poll.
        if CALLS.fetch_add(1, Ordering::SeqCst) >= 2 {
            Some(42)
        } else {
            None
        }
    });
    assert_eq!(got, Some(42), "wait_for must surface the first Some");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "wait_for returned only by timeout, not by success"
    );
}

/// `kill_tree` must kill the child *and its descendants*. We spawn a shell that
/// itself spawns a long-lived `sleep` (a grandchild) and waits on it; the shell
/// is in its own process group (as every harness spawn is). After `kill_tree`
/// on the shell, no `sleep` from that group may remain.
///
/// Unix-only (the spec says skip on Windows — `taskkill /F /T` is verified by
/// the Windows CI matrix separately, and `ps` is not portable).
#[cfg(unix)]
#[test]
fn kill_tree_kills_grandchildren() {
    // `sh -c 'sleep 30 & wait'`: the shell forks a `sleep 30` grandchild and
    // waits, so the shell stays alive as long as the sleep does. Both are in
    // the shell's process group because we spawn it with its own pgid (the same
    // setup every harness spawn uses).
    let mut shell_cmd = Command::new("sh");
    make_own_process_group(&mut shell_cmd);
    let mut shell = shell_cmd
        .args(["-c", "sleep 30 & wait"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `sh -c 'sleep 30 & wait'`");

    // Let the grandchild actually start (observed, not guessed): poll `ps`
    // until a `sleep` in the shell's process group is visible.
    let pgid = shell.id() as i64;
    let started = Instant::now();
    wait_for(Duration::from_secs(3), || {
        (grandchildren_of(pgid) > 0).then_some(())
    })
    .unwrap_or_else(|| {
        kill_tree(&mut shell);
        panic!(
            "expected a `sleep` grandchild in pgid {pgid} within 3s; \
             the spawn pattern may not have produced one (took {:?})",
            started.elapsed()
        )
    });

    // Now kill the tree and confirm the grandchild is reaped.
    kill_tree(&mut shell);
    let reaped = wait_for(Duration::from_secs(3), || {
        (grandchildren_of(pgid) == 0).then_some(())
    });
    assert!(
        reaped.is_some(),
        "a grandchild of pgid {pgid} survived kill_tree (orphans leak agents)"
    );
}

// --- helpers (test-local; not part of the public harness API) ---------------

/// The grandchild count of a process group, via `ps` on Unix.
#[cfg(unix)]
fn grandchildren_of(pgid: i64) -> u32 {
    // `ps -eo pgid,comm` lists every process's group + command. Count the rows
    // whose pgid is ours and whose comm is not the shell itself.
    let out = std::process::Command::new("ps")
        .args(["-eo", "pgid,comm"])
        .output()
        .expect("run ps");
    assert!(out.status.success(), "ps failed: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .skip(1) // header
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let row_pgid: i64 = parts.next()?.parse().ok()?;
            let comm = parts.next()?;
            (row_pgid == pgid && !matches!(comm, "sh" | "bash")).then_some(())
        })
        .count() as u32
}
