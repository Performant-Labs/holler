#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #696
//! The shared bounded runner (#696): what the four private copies did, pinned through the
//! one public API. Each test is a real child of a harmless command (`sh -c`, `sleep`,
//! `printf`, `head`, `yes`) under one fresh scratch directory removed on `Drop`. The tests
//! signal nothing themselves; `ps` is only read. Timing is asserted as an invariant (a
//! generous ceiling, and "not before the deadline" where the deadline is the point), never
//! as a tight duration.
//!
//! A behaviour-preserving refactor has no behaviour to turn green, so these are
//! characterisation tests: they describe the table of the brief row by row (stdout and
//! stderr capture, caps that truncate or fail, kill scope, deadline, spawn failure). The
//! probe's own texts are pinned by the unchanged inline tests of `probe.rs`, and the
//! adapters' by their own suites.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::bounded::{run, Capture, KillScope, Over, Ran, RunError, Spec};

const LONG: Duration = Duration::from_secs(10);
const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;

/// A fresh `hlr-bounded-696-<pid>-<n>` directory under the temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("hlr-bounded-696-{}-{n}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("the scratch path is UTF-8")
}

/// stdout kept up to `cap`, truncating; stderr discarded (what the Claude adapter asks for).
fn keep_stdout(cap: usize, over: Over, kill: KillScope) -> Spec {
    Spec {
        stdout: Capture::Keep { cap, over },
        stderr: Capture::Discard,
        kill,
        poll: Duration::from_millis(5),
    }
}

/// Both streams kept, truncating at 64 KiB (what the OpenCode adapter asks for).
fn keep_both(cap: usize) -> Spec {
    let keep = Capture::Keep {
        cap,
        over: Over::Truncate,
    };
    Spec {
        stdout: keep,
        stderr: keep,
        kill: KillScope::Child,
        poll: Duration::from_millis(2),
    }
}

fn sh(script: &str, args: &[&str]) -> Command {
    let mut command = Command::new("sh");
    command.arg("-c").arg(script).args(args);
    command
}

fn ran(result: Result<Ran, RunError>) -> Ran {
    match result {
        Ok(ran) => ran,
        Err(e) => panic!("expected a finished run, got {e:?}"),
    }
}

fn expired(result: Result<Ran, RunError>) {
    match result {
        Err(RunError::Expired) => {}
        other => panic!("expected RunError::Expired, got {other:?}"),
    }
}

/// Wait at most 40 x 50 ms for `pid` to be gone or a zombie; fail naming the pid.
fn assert_gone_within_2s(pid: &str) {
    let mut last = String::new();
    for _ in 0..40 {
        let out = Command::new("ps")
            .args(["-o", "stat=", "-p", pid])
            .output()
            .unwrap();
        let stat = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if !out.status.success() || stat.is_empty() || stat.starts_with('Z') {
            return;
        }
        last = stat;
        thread::sleep(Duration::from_millis(50));
    }
    panic!("pid {pid} is still alive 2 s after the run returned (last stat {last:?})");
}

/// The pid a script wrote to `file`, which must exist (so a runner that answers without
/// spawning fails on the existence, not later).
fn written_pid(file: &Path) -> String {
    assert!(
        file.exists(),
        "the child never wrote {}: it was not started",
        file.display()
    );
    let pid = fs::read_to_string(file).unwrap().trim().to_owned();
    assert!(pid.parse::<u32>().is_ok(), "not a pid: {pid:?}");
    pid
}

// --- the basic shape ---

#[test]
fn a_child_that_exits_returns_its_status_and_stdout_at_once_not_at_the_deadline() {
    let start = Instant::now();
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let ran = ran(run(
        &mut sh("printf hello; exit 3", &[]),
        &spec,
        start + Duration::from_secs(30),
    ));
    assert_eq!(ran.status.code(), Some(3), "a non-zero exit is a Ran");
    assert_eq!(ran.stdout, b"hello");
    assert!(ran.stderr.is_empty());
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn the_command_runs_as_the_caller_built_it() {
    let dir = Scratch::new();
    let mut command = Command::new("sh");
    command
        .args(["-c", "printf '%s|%s|' \"$HLR_696\" \"$0\"; pwd", "a;b"])
        .env("HLR_696", "from-the-caller")
        .current_dir(&dir.0);
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let out = ran(run(&mut command, &spec, Instant::now() + LONG)).stdout;
    let out = String::from_utf8(out).unwrap();
    assert!(
        out.starts_with("from-the-caller|a;b|"),
        "env or argv changed: {out:?}"
    );
    // `pwd` may print a canonical path; the last component is ours either way.
    let name = dir.0.file_name().unwrap().to_str().unwrap();
    assert!(out.trim_end().ends_with(name), "cwd changed: {out:?}");
}

// --- the deadline ---

#[test]
fn a_hung_child_is_expired_at_the_deadline_and_not_before() {
    let deadline_in = Duration::from_millis(300);
    let start = Instant::now();
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let result = run(Command::new("sleep").arg("30"), &spec, start + deadline_in);
    let elapsed = start.elapsed();
    expired(result);
    assert!(
        elapsed >= deadline_in,
        "returned before the deadline: {elapsed:?}"
    );
    assert!(
        elapsed < deadline_in + Duration::from_secs(2),
        "returned too late: {elapsed:?}"
    );
}

#[test]
fn a_deadline_already_passed_starts_nothing() {
    let dir = Scratch::new();
    let marker = dir.path("ran");
    let past = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let result = run(&mut sh("touch \"$0\"", &[path_str(&marker)]), &spec, past);
    expired(result);
    // The child would write the marker within milliseconds of a start; give it that.
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "a spent deadline still started the command"
    );
}

#[test]
fn the_deadline_kills_the_child() {
    let dir = Scratch::new();
    let pid_file = dir.path("pid");
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let result = run(
        &mut sh("echo $$ > \"$0\"; exec sleep 30", &[path_str(&pid_file)]),
        &spec,
        Instant::now() + Duration::from_millis(500),
    );
    let pid = written_pid(&pid_file);
    expired(result);
    assert_gone_within_2s(&pid);
}

#[test]
fn group_scope_kills_the_whole_process_group_at_the_deadline() {
    let dir = Scratch::new();
    let pid_file = dir.path("pid");
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Group);
    let result = run(
        &mut sh("sleep 30 & echo $! > \"$0\"; wait", &[path_str(&pid_file)]),
        &spec,
        Instant::now() + Duration::from_millis(500),
    );
    let grandchild = written_pid(&pid_file);
    expired(result);
    assert_gone_within_2s(&grandchild);
}

#[test]
fn a_background_child_holding_stdout_open_is_expired_never_a_result() {
    let dir = Scratch::new();
    let pid_file = dir.path("pid");
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Group);
    let result = run(
        &mut sh(
            "sleep 30 & echo $! > \"$0\"; echo up",
            &[path_str(&pid_file)],
        ),
        &spec,
        Instant::now() + Duration::from_millis(500),
    );
    let grandchild = written_pid(&pid_file);
    // The leader exited at once, but the pipe was still open: no verdict, a timeout.
    expired(result);
    assert_gone_within_2s(&grandchild);
}

/// The process-group id `ps` reports for `pid`.
fn pgid_of(pid: &str) -> String {
    let out = Command::new("ps")
        .args(["-o", "pgid=", "-p", pid])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Run a child that writes `<pid> <pgid>` of itself, and return that pair.
fn child_pid_and_pgid(kill: KillScope) -> (String, String) {
    let dir = Scratch::new();
    let out_file = dir.path("ids");
    let spec = keep_stdout(MIB, Over::Fail, kill);
    let script = "echo \"$$ $(ps -o pgid= -p $$)\" > \"$0\"";
    let _ = ran(run(
        &mut sh(script, &[path_str(&out_file)]),
        &spec,
        Instant::now() + LONG,
    ));
    assert!(out_file.exists(), "the child never wrote its ids");
    let ids = fs::read_to_string(&out_file).unwrap();
    let mut parts = ids.split_whitespace().map(str::to_owned);
    (parts.next().unwrap(), parts.next().unwrap())
}

#[test]
fn child_scope_keeps_the_callers_process_group() {
    let (_, child_group) = child_pid_and_pgid(KillScope::Child);
    assert_eq!(child_group, pgid_of(&std::process::id().to_string()));
}

#[test]
fn group_scope_gives_the_child_a_process_group_of_its_own() {
    let (pid, child_group) = child_pid_and_pgid(KillScope::Group);
    assert_eq!(child_group, pid, "the group id is the leader's pid");
    assert_ne!(child_group, pgid_of(&std::process::id().to_string()));
}

// --- stderr ---

#[test]
fn kept_stderr_is_returned_beside_the_status() {
    let ran = ran(run(
        &mut sh("echo err >&2; printf out; exit 3", &[]),
        &keep_both(64 * KIB),
        Instant::now() + LONG,
    ));
    assert_eq!(ran.status.code(), Some(3));
    assert_eq!(ran.stderr, b"err\n");
    assert_eq!(ran.stdout, b"out");
}

#[test]
fn discarded_stderr_is_empty_and_never_blocks_a_chatty_child() {
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let ran = ran(run(
        &mut sh(
            "echo err >&2; head -c 204800 /dev/zero >&2; printf out",
            &[],
        ),
        &spec,
        Instant::now() + LONG,
    ));
    assert!(ran.status.success());
    assert!(
        ran.stderr.is_empty(),
        "{} bytes of stderr",
        ran.stderr.len()
    );
    assert_eq!(ran.stdout, b"out");
}

#[test]
fn a_flood_on_stderr_with_a_quiet_stdout_still_finishes_and_is_truncated() {
    // Larger than the pipe buffer: a one-stream engine that waits for stdout first blocks.
    let ran = ran(run(
        &mut sh("head -c 204800 /dev/zero >&2", &[]),
        &keep_both(64 * KIB),
        Instant::now() + LONG,
    ));
    assert!(ran.status.success(), "the child must not block on stderr");
    assert!(ran.stdout.is_empty());
    assert_eq!(ran.stderr.len(), 64 * KIB);
}

// --- caps ---

#[test]
fn a_truncating_cap_keeps_the_first_bytes_and_lets_the_child_finish() {
    let ran = ran(run(
        &mut sh("head -c 204800 /dev/zero | tr '\\0' 'x'; exit 0", &[]),
        &keep_both(64 * KIB),
        Instant::now() + LONG,
    ));
    assert!(
        ran.status.success(),
        "the rest is read and dropped, not left to block"
    );
    assert_eq!(ran.stdout.len(), 64 * KIB);
    assert!(ran.stdout.iter().all(|b| *b == b'x'));
}

#[test]
fn a_failing_cap_is_an_overflow_and_ends_the_run_at_once() {
    let start = Instant::now();
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Group);
    let result = run(&mut Command::new("yes"), &spec, start + LONG);
    assert!(matches!(result, Err(RunError::Overflow)), "{result:?}");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn the_failing_cap_is_one_byte_past_the_cap_exactly() {
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let at_cap = ran(run(
        &mut sh("head -c 1048576 /dev/zero", &[]),
        &spec,
        Instant::now() + LONG,
    ));
    assert_eq!(
        at_cap.stdout.len(),
        MIB,
        "exactly the cap is not an overflow"
    );
    let over = run(
        &mut sh("head -c 1048577 /dev/zero", &[]),
        &spec,
        Instant::now() + LONG,
    );
    assert!(matches!(over, Err(RunError::Overflow)), "{over:?}");
}

#[test]
fn an_overflow_kills_the_child() {
    let dir = Scratch::new();
    let pid_file = dir.path("pid");
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Child);
    let result = run(
        &mut sh("echo $$ > \"$0\"; exec yes", &[path_str(&pid_file)]),
        &spec,
        Instant::now() + LONG,
    );
    let pid = written_pid(&pid_file);
    assert!(matches!(result, Err(RunError::Overflow)), "{result:?}");
    assert_gone_within_2s(&pid);
}

#[test]
fn a_failing_cap_on_stderr_is_an_overflow_too() {
    let fail = Capture::Keep {
        cap: 64 * KIB,
        over: Over::Fail,
    };
    let spec = Spec {
        stdout: fail,
        stderr: fail,
        kill: KillScope::Child,
        poll: Duration::from_millis(5),
    };
    let result = run(&mut sh("exec yes >&2", &[]), &spec, Instant::now() + LONG);
    assert!(matches!(result, Err(RunError::Overflow)), "{result:?}");
}

// --- spawn failure ---

#[test]
fn a_missing_program_is_a_spawn_error_that_echoes_no_argument() {
    let spec = keep_stdout(MIB, Over::Fail, KillScope::Group);
    let result = run(
        Command::new("/nonexistent/hlr-696-SENTINELPROG").arg("SECRET696ARG"),
        &spec,
        Instant::now() + LONG,
    );
    match &result {
        Err(RunError::Spawn(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected RunError::Spawn(NotFound), got {other:?}"),
    }
    let text = format!("{result:?}");
    assert!(
        !text.contains("SECRET696ARG"),
        "an argument was echoed: {text}"
    );
}
