//! The health probe of a pane (epic #633, B1): a `check` argv whose output must
//! contain every `expect` string.
//!
//! **Frozen by #637:** [`ProbeResult`] (which `Pane.probe.last` persists) and the
//! signature of [`run_probe`]. The verdict is #663's; the process is run by the crate's one
//! bounded runner, [`crate::bounded`] (#696), which the adapters share: this module only says
//! how a probe is run (`PROBE`) and turns what the runner answers into a [`ProbeResult`]
//! (`verdict`), so the runner's mechanics and the pid-reuse rule are documented there.
//!
//! A probe's run is fixed, not a choice: stdout kept up to 1 MiB with one byte more an
//! error, stderr discarded, and on the deadline its whole process group killed with `KILL`
//! (no `TERM` grace: a probe is a read-only check with nothing to clean up) through `kill`
//! from `PATH`, with no seam for a fake.

use std::process::Command;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::argv::Argv;
use crate::bounded::{self, Capture, KillScope, Over, Ran, RunError, Spec};

/// The most a probe may write to stdout (1 MiB); one byte more is an `Error`.
const MAX_OUTPUT: usize = 1 << 20;

/// How a probe is run: stdout read up to [`MAX_OUTPUT`] (one byte more fails the run), stderr
/// discarded, the probe in a process group of its own that the deadline kills whole, and its
/// exit polled every 10 ms.
const PROBE: Spec = Spec {
    stdout: Capture::Keep {
        cap: MAX_OUTPUT,
        over: Over::Fail,
    },
    stderr: Capture::Discard,
    kill: KillScope::Group,
    poll: Duration::from_millis(10),
};

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
/// - **No signal once the leader may have been reaped.** The runner signals only while the
///   leader is unreaped, so its pid cannot belong to another process. An exit status that
///   cannot be read (a `try_wait` error: something else in this process reaped the leader)
///   is `Error`, no signal is sent, and any child still in the group is left.
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
    let mut command = Command::new(program);
    command.args(args);
    verdict(
        bounded::run(&mut command, &PROBE, deadline),
        expect,
        timeout,
    )
}

/// The [`ProbeResult`] of what the runner answered, with the fixed reasons of [`run_probe`].
fn verdict(outcome: Result<Ran, RunError>, expect: &[String], timeout: Duration) -> ProbeResult {
    match outcome {
        Ok(Ran { status, stdout, .. }) if status.success() => {
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
        Ok(Ran { status, .. }) => match status.code() {
            Some(code) => error(format!("the probe exited with status {code}")),
            None => error("the probe was ended by a signal"),
        },
        Err(RunError::Expired) => error(format!(
            "the probe timed out after {} ms",
            timeout.as_millis()
        )),
        Err(RunError::Overflow) => error("the probe wrote more than 1 MiB to stdout"),
        Err(RunError::Spawn(e)) => error(format!(
            "the probe program could not be started: {}",
            e.kind()
        )),
        Err(RunError::ReaderFailed(_)) => error("the probe could not start its output reader"),
        Err(RunError::OutputLost) => error("the probe's output could not be read"),
        Err(RunError::WaitFailed(_)) => error("the probe's exit status could not be read"),
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
