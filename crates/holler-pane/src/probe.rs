//! The health probe of a pane (epic #633, B1): a `check` argv whose output must
//! contain every `expect` string.
//!
//! **Frozen by #637:** [`ProbeResult`] (which `Pane.probe.last` persists) and the
//! signature of [`run_probe`]. The runner is #663's, and it is this crate's one direct
//! side effect (ADR-0021 section 5): it starts a process with the standard library
//! alone, so it takes no new dependency, and the group kill goes through the `kill`
//! program because `std` cannot signal a process group.
//!
//! **The mechanics are kept apart from the verdict**, so #696 can lift them into the one
//! bounded runner the adapters share without a behaviour change: `run_bounded` (with
//! `spawn`, `start_reader`, `wait_for_exit` and `kill_and_reap`) runs the process and
//! answers an `Outcome`, and `verdict` alone turns that into a [`ProbeResult`]. Three
//! points are fixed here on purpose and are #696's to parameterize: **stderr** (discarded
//! here), **the signal and its grace** (`KILL` with no `TERM` grace: a probe is a
//! read-only check with nothing to clean up) and **the kill program** (`kill` from
//! `PATH`, with no seam for a fake).
//!
//! **The pid-reuse rule.** The group's id is the leader's pid, which no other process can
//! take while the leader is unreaped (alive or a zombie). So the runner signals only
//! while `try_wait` has not yet returned the leader's status: it waits for the end of
//! stdout first and for the exit only after that, and once the exit is seen it never
//! signals. A change that reaps the leader before it kills the group (a `try_wait`
//! before the end of stdout, then a kill) reopens the race.

use std::io::{self, Read};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::argv::Argv;

/// The most a probe may write to stdout (1 MiB); one byte more is an `Error`.
const MAX_OUTPUT: usize = 1 << 20;

/// The one budget the group kill and the reap of the leader share, once the runner gives up.
const CLEANUP: Duration = Duration::from_secs(1);

/// How often the runner polls `Child::try_wait`.
const POLL: Duration = Duration::from_millis(10);

/// What one run of a health probe found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeResult {
    /// The probe ran and every expected string was in its output.
    Ok,
    /// The probe ran and these expected strings were not in its output.
    Failed { missing: Vec<String> },
    /// The probe could not be run to a verdict (the program is missing, it timed
    /// out, ...); the reason is plain text.
    Error(String),
}

/// Run the health probe `argv` (never through a shell) and look for every string of
/// `expect` in its output, giving up after `timeout`.
///
/// - **How it runs.** `argv[0]` is the program, looked up on `PATH`, and every other element
///   is one argument, never joined or split. stdin is null, stdout is read, stderr is
///   discarded, and the environment and working directory are the caller's. On Unix the
///   process gets a process group of its own.
/// - **The verdict.** [`ProbeResult::Ok`] only when the program exited 0 and every `expect`
///   string occurs in its stdout (as bytes, case-sensitive, so output that is not UTF-8
///   still matches; an empty string always does). [`ProbeResult::Failed`] when it exited 0
///   without some of them, listed in `expect` order. Everything else is
///   [`ProbeResult::Error`], whatever the output: an empty argv, a zero timeout or one too
///   large for the clock (no process is started), a program that cannot be started, a
///   non-zero exit or an end by a signal, the deadline passing, or more than 1 MiB of
///   stdout.
/// - **The reasons** are fixed texts. None echoes an argv element (a check may carry a
///   token) or a byte of output; a spawn failure is named by its `io::ErrorKind` alone.
/// - **The bound.** One deadline, `timeout` after the call. When it passes, or the cap is
///   hit, the runner kills the probe's whole process group with `KILL`, then the leader,
///   and reaps the leader, within one cleanup budget of 1 s: it returns at the deadline
///   plus at most 1 s of cleanup (normally a few ms). A probe whose background child keeps
///   stdout open after the leader exits is a timeout, never a verdict.
/// - **The group kill is contingent on the `kill` program.** It runs `kill` from `PATH`:
///   if it cannot be spawned, only the leader is ended (`Child::kill`), its children are
///   left, and the runner still returns within the same bound. Off Unix only the leader is
///   ended.
/// - **Long-lived callers.** A probe whose child escapes the group (`setsid`, a double fork)
///   and keeps stdout open leaves one reader thread and its pipe blocked after the runner
///   returns, until that child closes the pipe. A long-lived caller that reuses
///   [`crate::SystemProber`] from `spawn_blocking` or a thread can so leak one thread per
///   such run, without bound; today's callers are short-lived CLI verbs.
pub fn run_probe(argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
    let Some((program, args)) = argv.as_slice().split_first() else {
        return error("the probe argv is empty: there is no program to run");
    };
    if timeout.is_zero() {
        return error("the probe timeout is zero");
    }
    let Some(deadline) = Instant::now().checked_add(timeout) else {
        return error("the probe timeout is too large");
    };
    verdict(run_bounded(program, args, deadline), expect, timeout)
}

/// What the run of one probe process came to: the mechanics' answer, before any verdict.
enum Outcome {
    /// The process exited, after its whole stdout was read.
    Exited { status: ExitStatus, stdout: Vec<u8> },
    /// The deadline passed first.
    TimedOut,
    /// It wrote more than [`MAX_OUTPUT`] bytes to stdout.
    Overflow,
    /// It could not be started.
    SpawnFailed(io::ErrorKind),
    /// The thread that reads its stdout could not be started.
    ReaderFailed,
    /// Reading its stdout failed.
    ReadFailed,
    /// Its exit status could not be read.
    WaitFailed,
}

/// What the reader thread found on stdout.
enum Output {
    /// The end of stdout, and every byte before it.
    Ended(Vec<u8>),
    /// More than [`MAX_OUTPUT`] bytes; reading stopped there.
    Overflow,
    /// A read failed.
    Failed,
}

/// The [`ProbeResult`] of `outcome`, with the fixed reasons of [`run_probe`].
fn verdict(outcome: Outcome, expect: &[String], timeout: Duration) -> ProbeResult {
    match outcome {
        Outcome::Exited { status, stdout } if status.success() => {
            let missing: Vec<String> = expect
                .iter()
                .filter(|wanted| !contains(&stdout, wanted.as_bytes()))
                .cloned()
                .collect();
            if missing.is_empty() {
                ProbeResult::Ok
            } else {
                ProbeResult::Failed { missing }
            }
        }
        Outcome::Exited { status, .. } => match status.code() {
            Some(code) => error(format!("the probe exited with status {code}")),
            None => error("the probe was ended by a signal"),
        },
        Outcome::TimedOut => error(format!(
            "the probe timed out after {} ms",
            timeout.as_millis()
        )),
        Outcome::Overflow => error("the probe wrote more than 1 MiB to stdout"),
        Outcome::SpawnFailed(kind) => {
            error(format!("the probe program could not be started: {kind}"))
        }
        Outcome::ReaderFailed => error("the probe could not start its output reader"),
        Outcome::ReadFailed => error("the probe's output could not be read"),
        Outcome::WaitFailed => error("the probe's exit status could not be read"),
    }
}

/// A [`ProbeResult::Error`] with `reason`.
fn error(reason: impl Into<String>) -> ProbeResult {
    ProbeResult::Error(reason.into())
}

/// Whether `needle` occurs in `haystack`; an empty needle always does.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// Run `program` with `args` until it exits or `deadline` passes, reading its stdout up to the
/// cap. Until the end of stdout the leader is never reaped, so every path here may kill.
fn run_bounded(program: &str, args: &[String], deadline: Instant) -> Outcome {
    let mut child = match spawn(program, args) {
        Ok(child) => child,
        Err(e) => return Outcome::SpawnFailed(e.kind()),
    };
    let Some(output) = start_reader(&mut child) else {
        kill_and_reap(&mut child);
        return Outcome::ReaderFailed;
    };
    let outcome = match output.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Output::Ended(stdout)) => return wait_for_exit(&mut child, stdout, deadline),
        Ok(Output::Overflow) => Outcome::Overflow,
        Ok(Output::Failed) | Err(RecvTimeoutError::Disconnected) => Outcome::ReadFailed,
        Err(RecvTimeoutError::Timeout) => Outcome::TimedOut,
    };
    kill_and_reap(&mut child);
    outcome
}

/// Start `program` itself (no shell) with `args`, stdin null, stdout piped and stderr
/// discarded, in a process group of its own, so the group id is its pid.
fn spawn(program: &str, args: &[String]) -> io::Result<Child> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()
}

/// Start the thread that reads the child's stdout up to the cap and sends what it found. It is
/// detached on purpose: the runner never joins it, so a kill never waits on its pipe.
fn start_reader(child: &mut Child) -> Option<mpsc::Receiver<Output>> {
    let stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::Builder::new()
        .name("holler-probe-stdout".to_owned())
        .spawn(move || {
            // The runner may have stopped listening (it gave up); that is not this thread's error.
            let _ = sender.send(read_capped(stdout));
        });
    reader.ok().map(|_detached| receiver)
}

/// Read `stdout` to its end, or until it passes the cap (one byte past it is enough to know).
fn read_capped(stdout: ChildStdout) -> Output {
    let mut bytes = Vec::new();
    match stdout.take(MAX_OUTPUT as u64 + 1).read_to_end(&mut bytes) {
        Ok(_) if bytes.len() > MAX_OUTPUT => Output::Overflow,
        Ok(_) => Output::Ended(bytes),
        Err(_) => Output::Failed,
    }
}

/// Stdout has ended: wait for the leader's exit until `deadline`. Past it the leader is still
/// unreaped, so the group can be killed; a status that cannot be read is never followed by a
/// signal, since the child may have been reaped elsewhere and its pid reused.
fn wait_for_exit(child: &mut Child, stdout: Vec<u8>, deadline: Instant) -> Outcome {
    match wait_until(child, deadline) {
        Ok(Some(status)) => Outcome::Exited { status, stdout },
        Ok(None) => {
            kill_and_reap(child);
            Outcome::TimedOut
        }
        Err(_) => Outcome::WaitFailed,
    }
}

/// Kill the probe's whole process group, then its leader, and reap the leader, all within the
/// one cleanup budget. Called only while the leader is unreaped (the pid-reuse rule); a leader
/// still unreaped when the budget runs out is left, a zombie, for the caller's exit.
fn kill_and_reap(child: &mut Child) {
    let budget = Instant::now()
        .checked_add(CLEANUP)
        .unwrap_or_else(Instant::now);
    kill_group(child.id(), budget);
    // The leader too, in case the group kill could not run; an error means it is gone already.
    let _ = child.kill();
    let _ = wait_until(child, budget);
}

/// Signal the process group `group` with `KILL` through `kill -s KILL -- -<group>`, waiting for
/// the `kill` program within `budget` and ignoring its outcome (the group may be gone already).
/// A `kill` still running past the budget is killed itself, and reaped if it has ended by then.
fn kill_group(group: u32, budget: Instant) {
    #[cfg(unix)]
    {
        let target = format!("-{group}");
        let killer = Command::new("kill")
            .args(["-s", "KILL", "--", target.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut killer) = killer {
            if let Ok(None) = wait_until(&mut killer, budget) {
                let _ = killer.kill();
                let _ = killer.try_wait();
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (group, budget);
    }
}

/// Poll `child` every [`POLL`] until it exits (its status, and it is reaped) or `until` passes
/// (`None`, and it is not).
fn wait_until(child: &mut Child, until: Instant) -> io::Result<Option<ExitStatus>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(None);
        }
        thread::sleep(left.min(POLL));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #663
mod tests {
    //! The runner's behaviour (#663, AC 8), each test a real child process of a harmless
    //! command. Every path a test creates is under one fresh scratch directory, removed on
    //! `Drop`. The tests signal nothing themselves; `ps` is only read. Inline, a deliberate
    //! departure from this crate's `tests/` pattern: the blast radius has no test file, and
    //! `StateDir`/`wait_for` (holler-cli's test support) are not reachable from here, so a
    //! minimal private guard and a counted poll stand in.

    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use std::{fs, thread};

    use super::{run_probe, ProbeResult};
    use crate::argv::Argv;

    /// A fresh `hlr-probe-663-<pid>-<n>` directory under the temp dir, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let dir =
                std::env::temp_dir().join(format!("hlr-probe-663-{}-{n}", std::process::id()));
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

    fn argv(parts: &[&str]) -> Argv {
        Argv::new(parts.iter().map(|part| (*part).to_owned()).collect())
    }

    fn probe(parts: &[&str], expect: &[&str], timeout: Duration) -> ProbeResult {
        let expect: Vec<String> = expect.iter().map(|e| (*e).to_owned()).collect();
        run_probe(&argv(parts), &expect, timeout)
    }

    fn reason(result: ProbeResult) -> String {
        match result {
            ProbeResult::Error(reason) => reason,
            other => panic!("expected ProbeResult::Error, got {other:?}"),
        }
    }

    fn assert_error_containing(result: ProbeResult, part: &str) -> String {
        let reason = reason(result);
        assert!(
            reason.contains(part),
            "{part:?} missing from the reason {reason:?}"
        );
        reason
    }

    fn path_str(path: &Path) -> &str {
        path.to_str().expect("the scratch path is UTF-8")
    }

    const LONG: Duration = Duration::from_secs(5);

    /// Wait at most 40 × 50 ms for `pid` to be gone or a zombie (`ps -o stat=` empty, a
    /// non-zero exit or a state starting with `Z`); fail naming the pid and the last stat.
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
        panic!("pid {pid} is still alive 2 s after run_probe returned (last stat {last:?})");
    }

    /// AC 8f/8g's order: build `<dir>/pid`, never write it, assert it is absent before the
    /// call; after `run_probe` returns, assert it exists and that its pid dies within 2 s.
    fn assert_group_killed(script: &str, expect: &[&str]) {
        let dir = Scratch::new();
        let pid_file = dir.path("pid");
        assert!(!pid_file.exists());
        let result = probe(
            &["sh", "-c", script, path_str(&pid_file)],
            expect,
            Duration::from_millis(500),
        );
        // Existence first, so a runner that answers without spawning fails here.
        assert!(
            pid_file.exists(),
            "the probe's child never wrote {}",
            pid_file.display()
        );
        assert_error_containing(result, "timed out");
        let pid = fs::read_to_string(&pid_file).unwrap();
        let pid = pid.trim();
        assert!(pid.parse::<u32>().is_ok(), "not a pid: {pid:?}");
        assert_gone_within_2s(pid);
    }

    // 8a
    #[test]
    fn all_expected_strings_present_is_ok() {
        let result = probe(
            &["printf", "%s\n%s\n", "alpha", "beta"],
            &["alpha", "beta"],
            LONG,
        );
        assert_eq!(result, ProbeResult::Ok);
    }

    // 8b
    #[test]
    fn one_missing_string_is_failed_naming_it() {
        let result = probe(
            &["printf", "%s\n%s\n", "alpha", "beta"],
            &["alpha", "gamma", "beta"],
            LONG,
        );
        assert_eq!(
            result,
            ProbeResult::Failed {
                missing: vec!["gamma".to_owned()]
            }
        );
    }

    // 8c
    #[test]
    fn no_expect_and_exit_zero_is_ok() {
        assert_eq!(probe(&["true"], &[], LONG), ProbeResult::Ok);
    }

    // 8d (Decision 14): a non-zero exit is Error whatever the output.
    #[test]
    fn non_zero_exit_is_error_even_with_every_string() {
        let result = probe(&["sh", "-c", "printf qwen38; exit 3"], &["qwen38"], LONG);
        assert_error_containing(result, "exited with status 3");
        assert_error_containing(probe(&["false"], &[], LONG), "exited with status 1");
    }

    // 8e (Decision 15, C7): Error at the deadline, plus at most 2 s of slack.
    #[test]
    fn hung_command_is_error_at_the_timeout() {
        let timeout = Duration::from_millis(300);
        let start = Instant::now();
        let result = probe(&["sleep", "30"], &[], timeout);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= timeout,
            "returned before the timeout: {elapsed:?}"
        );
        assert_error_containing(result, "timed out");
        assert!(
            elapsed < Duration::from_millis(2_300),
            "returned too late: {elapsed:?}"
        );
    }

    // 8f (Decision 15/16): the deadline kills the whole process group.
    #[test]
    fn timeout_kills_the_whole_process_group() {
        assert_group_killed("sleep 30 & echo $! > \"$0\"; wait", &[]);
    }

    // 8g: a background child holding stdout open makes it a timeout, never Ok.
    #[test]
    fn background_child_holding_stdout_is_a_timeout() {
        assert_group_killed("sleep 30 & echo $! > \"$0\"; echo up", &["up"]);
    }

    // 8h (Decision 13): every element arrives literally; nothing splits or interprets it.
    #[test]
    fn argv_is_never_given_to_a_shell() {
        let dir = Scratch::new();
        let (m1, m2) = (dir.path("m1"), dir.path("m2"));
        let sub = format!("$(touch {})", path_str(&m1));
        let chained = format!("x; touch {}", path_str(&m2));
        let result = probe(
            &["printf", "%s|", "a;", &sub, &chained],
            &["a;|$(touch ", "|x; touch "],
            LONG,
        );
        assert_eq!(result, ProbeResult::Ok);
        assert!(!m1.exists() && !m2.exists(), "a shell ran part of the argv");
        assert_error_containing(probe(&["printf hello"], &[], LONG), "could not be started");
    }

    // 8i (Decision 17): more than 1 MiB of stdout is Error, at once.
    #[test]
    fn output_over_the_cap_is_error() {
        let start = Instant::now();
        let result = probe(&["yes"], &[], Duration::from_secs(10));
        assert_error_containing(result, "more than 1 MiB");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "took {:?}",
            start.elapsed()
        );
    }

    // 8j (Decision 19): no reason echoes an argv element or an output byte.
    #[test]
    fn reasons_never_echo_argv_or_output() {
        let missing = probe(
            &["/nonexistent/hlr-663-SENTINELARG", "SECRET663ARG"],
            &[],
            LONG,
        );
        let spawn_reason = assert_error_containing(missing, "could not be started");
        assert!(
            !spawn_reason.contains("SENTINELARG") && !spawn_reason.contains("SECRET663ARG"),
            "{spawn_reason:?}"
        );
        let failed_exit = probe(&["sh", "-c", "printf SECRET663OUT; exit 4"], &[], LONG);
        let exit_reason = reason(failed_exit);
        assert!(!exit_reason.contains("SECRET663OUT"), "{exit_reason:?}");
        let result = probe(&["printf", "SECRET663OUT"], &["absent"], LONG);
        assert_eq!(
            result,
            ProbeResult::Failed {
                missing: vec!["absent".to_owned()]
            }
        );
    }

    // 8k (Decision 14): an empty argv and a zero timeout start no process.
    #[test]
    fn empty_argv_and_zero_timeout_are_errors_without_a_process() {
        let result = run_probe(&Argv::new(vec![]), &[], LONG);
        assert_error_containing(result, "empty");
        let dir = Scratch::new();
        let marker = dir.path("z");
        let result = probe(
            &["sh", "-c", "touch \"$0\"", path_str(&marker)],
            &[],
            Duration::ZERO,
        );
        assert_error_containing(result, "zero");
        assert!(!marker.exists(), "a zero timeout started the probe");
    }

    // 8l (Decision 18): matching is on bytes, so non-UTF-8 output still matches.
    #[test]
    fn non_utf8_output_still_matches() {
        assert_eq!(
            probe(&["printf", "\\377alpha\\376"], &["alpha"], LONG),
            ProbeResult::Ok
        );
    }
}
