# Handoff-T-red: #696 - one bounded subprocess runner

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Brief / wireframe reviewed:** docs/handoffs/696-brief.md (no wireframe: no UI); docs/handoffs/696/handoff-A.md

## A precondition
Confirmed: A returned PASS on the plan (handoff-A.md, six warns). Warns 1, 2 and 6 change the public types, so they are
applied in the stub below (see "The stub"). Not done on the branch: the ADR-0021 amend-first commit (brief, Decision 5:
"the FIRST commit on the branch, before T authors tests"). `git log origin/main..HEAD` shows no ADR commit and
`git diff origin/main -- docs/adr` is empty. T does not write it (it is not a test); O or F should land it before F's code.

## The stub (production file, authorised by the brief's Test plan)
`crates/holler-pane/src/bounded.rs` (new) and one line in `crates/holler-pane/src/lib.rs` (`pub mod bounded;`).
It declares the contract the tests are written against and a `run` that answers `Err(RunError::OutputLost)` and starts
nothing (the #637 stub pattern), so RED is an assertion, not a missing symbol. F REPLACES this file.

- `run(command: &mut Command, spec: &Spec, deadline: Instant) -> Result<Ran, RunError>` (A finding 6: `&mut Command`).
- `Spec { stdout: Capture, stderr: Capture, kill: KillScope, poll: Duration }` (Copy).
- `Capture::{Discard, Keep { cap: usize, over: Over }}`, `Over::{Truncate, Fail}`, `KillScope::{Child, Group}`.
- `Ran { status: ExitStatus, stdout: Vec<u8>, stderr: Vec<u8> }`.
- `RunError::{Expired, Spawn(io::Error), ReaderFailed(io::Error), WaitFailed(io::Error), OutputLost, Overflow}` (A finding 1:
  `ReaderFailed` carries the `io::Error`). `Debug` only is relied on; the tests never call `Display`.
- No flat re-export from `lib.rs` (A finding 2); the tests import `holler_pane::bounded::{...}`.

F may rename or reshape these only with a matching change to the two test files, which T owns; prefer to keep them.
Doc comments in the stub state the semantics (truncate vs fail, kill scope); they are provisional.

## Tests authored

### `crates/holler-pane/tests/bounded_runner_test.rs` (18 tests, RED-2, integration tier in `holler-pane/tests/`)
The cheapest sufficient tier: the runner is a `holler-pane` function over real children, so a `holler-pane` integration test
with `sh`/`sleep`/`head`/`yes` and no hub or body. Each pins one row of the brief's behaviour table.

| Test | Pins |
|---|---|
| `a_child_that_exits_returns_its_status_and_stdout_at_once_not_at_the_deadline` | a non-zero exit is a `Ran`, not an error; returns at exit, not at the deadline; stdout captured |
| `the_command_runs_as_the_caller_built_it` | the caller's env, cwd and literal argv are honoured (no shell, no rebuild) |
| `a_hung_child_is_expired_at_the_deadline_and_not_before` | `Expired`, not before the deadline, within deadline + 2 s |
| `a_deadline_already_passed_starts_nothing` | spent deadline: `Expired` and a marker file proves the command never ran |
| `the_deadline_kills_the_child` | `Child` scope: the leader's pid is gone afterwards |
| `group_scope_kills_the_whole_process_group_at_the_deadline` | `Group` scope: the grandchild is gone too |
| `a_background_child_holding_stdout_open_is_expired_never_a_result` | leader exits, pipe held: `Expired`, never a verdict; group killed |
| `child_scope_keeps_the_callers_process_group` | adapters stay in the caller's group (no behaviour change) |
| `group_scope_gives_the_child_a_process_group_of_its_own` | probe: group id == leader pid, not the caller's |
| `kept_stderr_is_returned_beside_the_status` | stderr `Keep`: status 3, `err\n`, stdout too |
| `discarded_stderr_is_empty_and_never_blocks_a_chatty_child` | stderr `Discard`: empty, and 200 KiB on stderr does not block |
| `a_flood_on_stderr_with_a_quiet_stdout_still_finishes_and_is_truncated` | A finding 3: both streams drained at once; a one-stream engine deadlocks |
| `a_truncating_cap_keeps_the_first_bytes_and_lets_the_child_finish` | `Truncate(64 KiB)`: exactly 64 KiB, child exits 0 |
| `a_failing_cap_is_an_overflow_and_ends_the_run_at_once` | `Fail(1 MiB)` on `yes`: `Overflow`, fast |
| `the_failing_cap_is_one_byte_past_the_cap_exactly` | exactly 1 MiB is `Ok`; 1 MiB + 1 is `Overflow` |
| `an_overflow_kills_the_child` | overflow ends the leader (pid gone) |
| `a_failing_cap_on_stderr_is_an_overflow_too` | A finding 3: an overflow on either stream fails the run |
| `a_missing_program_is_a_spawn_error_that_echoes_no_argument` | `Spawn` with `NotFound`; no argument in the error's `Debug` |

### `crates/holler-pane/tests/one_runner_guard_test.rs` (4 tests, RED-1, unit/source tier)
Walks the `src/` of `holler-pane` and the three adapters, comment text stripped (`fake_tmux_test.rs` idiom).
- `only_the_shared_runner_polls_a_child`: `try_wait(` occurs only in `crates/holler-pane/src/bounded.rs` (A: exactly that file; `probe.rs` must have none afterwards), and that file does poll.
- `only_the_shared_runner_waits_on_a_reader_with_a_timeout`: same for `recv_timeout(` (the herdr adapter's socket use is outside the walked crates).
- `no_adapter_keeps_its_own_drain_collect_wait_or_reap`: each adapter's `exec.rs`, if it still exists, has no `fn wait*`, `fn drain*`, `fn collect*`, `fn reap*`, `thread::Builder`, `read_to_end` or `child.kill` (AC 2). A deleted file passes. The match is by prefix, so `fn wait_for_exit` in a kept host `exec.rs` would also trip it: that is intended (the host keeps only `kill_group`, `Signal`, `Group`).
- `the_opencode_and_claude_adapters_have_no_exec_module_left`: both `exec.rs` are absent and `lib.rs` has no `mod exec;` (AC 2).

Limit, stated in the file too: this proves "only one runner", not "the same behaviour".

## RED confirmation
Command: `cargo test -p holler-pane --test bounded_runner_test` and `--test one_runner_guard_test`.

`bounded_runner_test`: `0 passed; 18 failed`. Every failure is on an assertion about the missing behaviour, none a compile,
setup or timeout error. Representative output:
```
expected a finished run, got OutputLost                      (exit/stderr/cap/process-group tests)
expected RunError::Expired, got Err(OutputLost)              (deadline tests)
expected RunError::Spawn(NotFound), got Err(OutputLost)      (spawn failure)
Err(OutputLost)                                              (the Overflow tests)
the child never wrote /tmp/hlr-bounded-696-.../pid: it was not started   (kill tests: the existence check precedes the pid check)
```
`one_runner_guard_test`: `0 passed; 4 failed`, naming the offenders:
```
try_wait( outside crates/holler-pane/src/bounded.rs: a private runner copy remains in [
    "crates/holler-pane/src/probe.rs", "crates/holler-adapter-host/src/exec.rs",
    "crates/holler-adapter-opencode/src/exec.rs", "crates/holler-adapter-claude/src/exec.rs" ]
recv_timeout( outside ...: the same four files
holler-adapter-opencode/src/exec.rs still exists: its runner moved to crates/holler-pane/src/bounded.rs
a private runner copy remains: [ ...host/opencode/claude exec.rs: fn wait, fn drain, fn collect, fn reap, thread::Builder, read_to_end, child.kill ]
```

**Validity of the tests themselves (not only that they fail).** A failing test proves little if the test is wrong, so I put a
throwaway reference runner (threads for both streams, group kill through `kill`, 60 lines, not kept) in `bounded.rs`,
ran `bounded_runner_test` three times: `18 passed` each time, about 0.55 s. Then restored the stub. This shows the 18 are
satisfiable and not flaky on this host, and that the `ps`/`sh`/`head` fixtures work. The guard's GREEN side was not run
(it needs F's deletions); its RED output above shows the walk finds the right files.

Existing tests are untouched and still green: `cargo test -p holler-pane` runs the 18 inline `probe.rs` tests (`18 passed`) and
every other `holler-pane` test green; only the two new files fail. `cargo clippy -p holler-pane --all-targets -- -D warnings`
clean; `bash scripts/lint.sh` exit 0 (the new files are 445, 154 and 91 lines; no `#[allow]` without `// #696`).

## Not covered here, and why
- **Poll interval** (2/5/5/10 ms) and the 1 s cleanup budget: not testable as invariants without a clock seam; `poll` is exercised
  as a parameter only. The `hung_child` test bounds the cleanup (deadline + 2 s).
- **The pid-reuse rule and "no signal after a `try_wait` error"** (Decision 4): the error path is unreachable without reaping the
  child behind the runner's back. Left to A's diff review.
- **Read-error semantics per `over` mode** (A finding 4): unreachable with real children.
- **Adapter-side error texts** (`cannot run ...`, the Claude `kind()`-only form, `Failed::Unavailable`): pinned by the unchanged
  adapter suites; F must grep them before writing the `RunError` mapping (brief, Risks).
- **Adapter gaps with no direct test** (to be filled at T-green through the adapters' public seams, per the brief): the
  Claude runner has no direct timeout test (only the seams' hermetic tests); the OpenCode `Truncate` cap at 64 KiB is untested
  at the adapter level; the host's uncapped output is untested. T-green lists and closes these.
- `holler-cli` is not touched, so no `[[test]]` entry is needed (`holler-pane` auto-discovers `tests/*.rs`).

## Ready for F
Confirmed: RED is valid. F may implement against these tests. F replaces `crates/holler-pane/src/bounded.rs`, switches `run_probe`
and the three adapters, deletes the two adapter `exec.rs` and the host's loops, and lands the ADR-0021 amendment first.
