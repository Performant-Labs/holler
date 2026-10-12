# Handoff-T-green: #696 - one bounded subprocess runner

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Issue:** #696
**Handoff-F reviewed:** docs/handoffs/696/handoff-F.md
**Handoff-T-red:** docs/handoffs/696/handoff-T-red.md

## GREEN confirmation
- `cargo test -p holler-pane -p holler-adapter-host -p holler-adapter-opencode -p holler-adapter-claude`: 0 failed.
  `bounded_runner_test` 18/18, `one_runner_guard_test` 4/4, `probe.rs` inline 18/18, `fake_tmux_test` 29/29,
  `foreground_test` 4/4, OpenCode `attach` 23, `hermetic` 33, `agent` 11, `prompt` 12, `tui` 13, Claude `hermetic` 16.
  Real-tmux/opencode/claude suites stay `ignored` (opt-in).
- `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the CI command):
  exit 0, 169 `test result` lines, none failed. See the flake note under Advisory.
- Tests still fail when behaviour is removed (each mutation applied to production, then reverted by `git checkout`):
  - runner: no group kill -> 2 of 18 fail; no process group of its own -> 3 fail; Fail cap `>=` for `>` -> 1 fails.
  - Claude stdout cap 1 MiB -> 512 KiB, OpenCode 64 -> 32 KiB, host `usize::MAX` -> 1 MiB: each fails its new test.
  - the guard's RED output (handoff-T-red) shows it fails on a private copy.
- Flake check: the new and the 18+4 tests, 12 repeats, then 3 more under 8 busy-loop processes: no failure.

## Tier 1 results
| Check | Result |
|---|---|
| `cargo test --workspace` (CI form) | PASS (see above) |
| `bash scripts/lint.sh` | PASS, exit 0 (warnings only, pre-existing files; no new file near 900) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo clippy -p <four crates> --all-targets -- -D warnings` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | FAIL on untouched `holler-cli/tests/token_load_test.rs:60` (`unneeded cloning of iterator items`, this host's newer clippy). Pre-existing, not in the diff. Workspace excluding `holler-cli`, and `holler-cli --lib --bins`, are clean. |
| `cargo test -p holler-cli --test docs_cli_test`, `--test wire_selftest` | PASS (3 and 3) |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` on the four crates | PASS |
| `cargo fmt --check` | new files clean; `foreground_test.rs` reports a diff, an untouched pre-existing file |
| `cargo machete` | NOT RUN: not installed here. No `Cargo.toml` is in the diff, so no dependency changed. |

## Tier 2 results
- **Coverage per AC.** PASS. The brief's three named gaps are closed here (below).
- **Test quality (test-quality.md section 7).** PASS. Each of the 18+4 names a behaviour, fails for the right reason (RED recorded),
  sits at the cheapest tier (the runner is a `holler-pane` integration test with real children, no hub). The three new files are
  one adapter-seam test each, no duplicates of the runner tests: they pin an adapter's chosen `Spec` (cap size and mode), which the
  runner tests cannot see. No test was found redundant; none deleted.
- **Concurrency/timing.** PASS. Readiness is by exit or by pid; the bounds are asserted as ranges (`>= bound`, `< bound + 2 s`),
  not durations; no fixed sleep except the existing `ETXTBSY` retry idiom for a freshly written script.
- **Secrets.** PASS. `what_claude_prints_on_stderr_is_never_quoted` asserts the stderr marker is absent from both Claude seams' errors.
- **AC 2 greps.** PASS. `try_wait` and `recv_timeout` in `crates/*/src` code: only `holler-pane/src/bounded.rs`, plus the
  herdr socket exchange and two load-test files (not subprocess runners, outside the walked crates). Both adapter `exec.rs` are
  gone; `mod exec` absent from their `lib.rs`.
- **AC 3 (no behaviour change).** PASS. No existing test file is in the diff (`git diff origin/main --name-only -- '*/tests/*'`
  lists only T's two). Every existing adapter and probe test passes unmodified.
- **Protocol/goldens/docs.** N/A (no wire change). `docs_cli_test` PASS.
- **`#[allow]` links, file sizes.** PASS (`// #696`; the new tests are 210, 95 and 35 lines).
- **Evidence appendix.** Completed: three entries added under `## T (green)` in `evidence.md`, with verbatim excerpts.

## Tests added in this phase (the brief's adapter gaps, through public seams)
- `crates/holler-adapter-claude/tests/seams_test.rs` (4; the Claude runner had no direct test, only faked seams):
  `a_claude_that_hangs_times_out_the_method_that_asked_and_is_killed` (op names, bound, child reaped),
  `a_missing_claude_is_unavailable_naming_the_program_and_no_argument`,
  `what_claude_prints_on_stderr_is_never_quoted`,
  `claudes_stdout_is_cut_at_one_mib_and_the_rest_is_drained_not_waited_on` (exactly the first MiB parses; noise after is dropped).
- `crates/holler-adapter-opencode/tests/tmux_capture_test.rs` (1): a 3 MiB tmux stderr is quoted as exactly 64 KiB and the call does not block.
- `crates/holler-adapter-host/tests/uncapped_output_test.rs` (1): a 3 MiB tmux stderr line is quoted whole.
No `[[test]]` entries are needed (those three crates auto-discover `tests/*.rs`; checked by running each).

## Acceptance criteria status
1. `pub mod bounded`; `run_probe` on it; probe tests unchanged and green: PASS (inline 18/18, `bounded_runner_test`).
2. Adapters on the shared runner; copies deleted: PASS (`one_runner_guard_test` 4/4, greps above).
3. No behaviour change: PASS (all existing tests unmodified and green; new cap tests pin Claude 1 MiB, OpenCode 64 KiB, host none).
4. New tests pass: PASS (18 + 4 + 6).
5. Clippy clean: PASS for the touched crates; the workspace form fails only on the pre-existing `token_load_test.rs` finding.
6. Docs: `pane-control.md` line removed, header docs updated, ADR-0021 note present: PASS on inspection (A/S own the wording). ADR "first commit" ordering is not met, as F and T-red already flagged.

## Blocking issues
None.

## Advisory notes
- **A flake outside this diff.** The first workspace run failed `interrupt_test::interrupt_unknown_session_exit_1`:
  `wait_warm: alpha had no connected roster row within 10s`; the hub log shows `conn_liveness_expired` 6 ms after
  `conn_connected`, so the body stayed `reconnecting`. It passed 3/3 alone and the full `--no-fail-fast` run was green.
  That path is wire liveness, not a subprocess runner. Worth its own issue if it recurs.
- **F's bounded reap for the adapters** (a child surviving `KILL` over 1 s now returns at the bound, was blocking) has no test: it
  needs a child in uninterruptible sleep. A should confirm, as F asked.
- The Claude `Spec` keeps stderr `Discard`; switching it to `Keep` would not change any observable behaviour (the seam never reads it), so no test pins `Discard` there.
