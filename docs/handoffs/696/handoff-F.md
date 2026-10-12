# Handoff-F: #696 - one bounded subprocess runner shared by the adapters and run_probe

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Issue:** #696

## What was done
- `crates/holler-pane/src/bounded.rs`: T's stub replaced with the runner: the probe's `run_bounded` family (`spawn`,
  readers, `wait_for_exit`, `kill_and_reap`, `kill_group`, `wait_until`) generalised over `Spec`. Public types unchanged from
  the stub (`run(&mut Command, &Spec, Instant) -> Result<Ran, RunError>`, `Spec`, `Capture`, `Over`, `KillScope`, `Ran`,
  `RunError`). Module doc carries the pid-reuse rule, the bound, the thread-leak limit and Decision 3.
- `crates/holler-pane/src/probe.rs`: mechanics removed; `run_probe` builds the `Command` and calls `bounded::run` with a
  `PROBE` spec (stdout `Keep{1 MiB, Fail}`, stderr `Discard`, `Group`, 10 ms); `verdict` maps `Result<Ran, RunError>` to the
  same `ProbeResult` texts. Header doc updated (the "#696's to parameterize" note closed). Inline tests untouched.
- `crates/holler-adapter-host/src/exec.rs`: `run` is a thin wrapper (spec: both streams `Keep{usize::MAX, Truncate}`,
  `Child`, 2 ms) mapping `RunError` to `Failed` with the old texts; `drain`/`wait`/`reap`/`collect` deleted. `Ran`, `Failed`,
  `Signal`, `Group`, `kill_group` kept. Header doc updated.
- `crates/holler-adapter-host/src/lib.rs`: one sentence in the header ("run by the shared bounded runner"). No code change.
- `crates/holler-adapter-opencode/src/exec.rs`: deleted. `src/lib.rs`: `mod exec;` dropped. `src/attach.rs`: private
  `TMUX_RUN` spec (both `Keep{64 KiB, Truncate}`, `Child`, 5 ms), `Ran` + `reason()` (one_line), `capture` and `cannot`
  moved here with the same texts; module doc bullet added.
- `crates/holler-adapter-claude/src/exec.rs`: deleted. `src/lib.rs`: `mod exec;` dropped. `src/seams.rs`: private
  `CLAUDE_RUN` spec (stdout `Keep{1 MiB, Truncate}`, stderr `Discard`, `Child`, 5 ms), `Ran`, `capture`, `cannot`
  (`kind()` only) with the same texts; module doc updated.
- `docs/adr/ADR-0021.md`: section 1 (`ProbeResult` paragraph) one sentence; section 5 (crate dependency rules) the
  "Amended by #696, with no contract change" note (Decision 5, also A's note on lines 80-86 and 398-399).
- `docs/pane-control.md`: the #696 line removed from section 12's deferred list.
- `CHANGELOG.md`: one `[Unreleased]` Enhancements entry linking #696.

## Design decisions
- **One engine, every kept stream then wait (Decision 4, A warn 3).** One channel; one reader thread per `Keep` stream, none
  for `Discard` (`Stdio::null()`). `read_all` waits for every kept stream by the deadline, and only then `wait_until`. Any
  error before the exit is seen kills; a `try_wait` error never does.
- **Read errors per `over` (A warn 4).** `Truncate`: a read error ends the capture with what was read, then the rest is
  drained to the sink (the adapters' rule). `Fail`: a read error is `RunError::OutputLost`, which `verdict` maps to the
  probe's old `ReadFailed` text ("the probe's output could not be read"), as the old probe already joined a failed read and a
  lost channel in one arm. No `ReadFailed` variant was added (the stub's `RunError` is T's contract).
- **"No cap" for the host** is `Keep { cap: usize::MAX, over: Truncate }` (a cap no output reaches); `take` gets
  `u64::try_from(cap).unwrap_or(u64::MAX)` and `Fail`'s one-past is `saturating_add`.
- **A missing pipe** (unreachable with `Stdio::piped()`) is `ReaderFailed(io::Error::other(..))`, the probe's fail-closed
  rule, not the adapters' "empty output".
- **Kill and reap** is the probe's bounded `kill_and_reap` for both scopes: `Group` adds the `kill -s KILL -- -<pid>` first;
  both then `Child::kill` and reap within the 1 s `CLEANUP` budget.
- **Unreachable `Overflow`** in the three adapters (all `Truncate`) is mapped with `OutputLost` to the "output was lost" text,
  with a comment; no `unreachable!`/panic (denied).
- **Mapping homes (A warn 5):** OpenCode in `attach.rs`, Claude in `seams.rs`, host in its kept `exec.rs`. Each adapter keeps
  its own `Ran` with lossily decoded `String`s, so call sites are unchanged apart from the function name.
- **No flat re-export** from `holler-pane/src/lib.rs` (A warn 2): consumers use `holler_pane::bounded::{..}`; `lib.rs` is
  unchanged from T's one line.

## Reuse / extend-vs-new
Extended the brief's named object: the probe's `run_bounded`/`spawn`/`start_reader`/`wait_for_exit`/`kill_and_reap`/
`kill_group`/`wait_until` moved into `bounded.rs` and generalised (stderr reader, `Truncate`, `KillScope::Child`, per-spec
poll). The adapters' `drain`/`wait`/`reap`/`collect` were deleted, not ported. The host's `kill_group` stays in the host
(brief, Out of scope) and runs `kill` through the shared runner via `exec::run`.

## Architecture notes for A
- New public surface in the contract crate: `pub mod bounded` (Decision 1; already declared by T's stub). No crate edge, no
  dependency (all three adapters already depend on `holler-pane`).
- `holler-pane` remains std-only for process work. The ADR note records `bounded` as the crate's one direct side effect.
- Two near-identical `capture`/`cannot` mappings now live in `attach.rs` and `seams.rs` (different texts: OpenCode one_lines
  the full `io::Error`, Claude prints `kind()` only). That is Decision 2's intended outcome (adapters may not depend on each
  other), not a third runner.
- File sizes: `bounded.rs` 376, `probe.rs` 420 (from 577), host `exec.rs` 149 (from 225), `attach.rs` 312, `seams.rs` 181.

## Deviations from spec / wireframe
- **Decision 4 (decided, recorded here as required):** the adapters no longer `Child::kill` after a `try_wait` error (the
  "something else reaped our child" path, unreachable in practice); they return the same `WaitFailed` text with no signal.
- **The adapters' reap is now bounded:** on the deadline they get `Child::kill` then a 1 s polled reap (the probe's
  `kill_and_reap`) instead of `Child::kill` then a blocking `Child::wait`. Outcome and text are identical for every reachable
  case (a `KILL`ed child is reaped in ms); only a child that survives `KILL` for over 1 s (uninterruptible sleep) differs:
  before, the call blocked past its bound; now it returns at the bound and leaves the zombie. Not named in the brief's
  decisions; it follows from "one engine" and the probe's bound. A should confirm.
- **Order for a held pipe in the adapters:** before, the child's exit was reaped first and the pipe then timed out (no kill);
  now the child is unreaped until the deadline and then `KILL`ed as a zombie (harmless) and reaped. Same `Expired` outcome.
- **ADR-0021 is not the first commit on the branch** (Decision 5 "amend-first"): T flagged that O had not landed it before
  T's tests, and F does not commit. The ADR edit is in this phase's working tree; whoever commits may split it into its own
  commit, but it cannot precede T's commit without a history rewrite.
- **CHANGELOG entry added** although the brief made it conditional on `docs/releasing.md` (which says nothing about
  refactors): the role's stack rule asks for one, and recent refactors (#802, #808) carried one. Drop it if O disagrees.

## Tier 1 self-check (incl. tests now GREEN)
```
cargo test -p holler-pane -p holler-adapter-host -p holler-adapter-opencode -p holler-adapter-claude
  holler-pane: unittests (probe.rs inline) 18 passed; bounded_runner_test 18 passed; one_runner_guard_test 4 passed;
               every other holler-pane test file green
  holler-adapter-host: fake_tmux_test 29 passed; foreground_test 4 passed; real_tmux_test 9 ignored
  holler-adapter-opencode: agent 11, attach 23, hermetic 33, prompt 12, tui 13 passed; real_opencode 12 ignored
  holler-adapter-claude: hermetic_test 16 passed; real_claude_test 1 ignored
  0 failed anywhere
cargo clippy -p holler-pane -p holler-adapter-host -p holler-adapter-opencode -p holler-adapter-claude --all-targets -- -D warnings: clean
cargo clippy --workspace --all-targets --exclude holler-cli -- -D warnings: clean
cargo clippy -p holler-cli --lib --bins -- -D warnings: clean
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps on the four crates: clean
bash scripts/lint.sh: exit 0 (warnings only, all pre-existing files)
grep -rn "try_wait" crates/*/src: code only in crates/holler-pane/src/bounded.rs (plus doc-comment mentions in bounded.rs and probe.rs)
ls crates/holler-adapter-{opencode,claude}/src/exec.rs: both gone; grep "mod exec" in their lib.rs: empty
```
`cargo clippy --workspace --all-targets -- -D warnings` fails on `crates/holler-cli/tests/token_load_test.rs:60`
(`unneeded cloning of iterator items`): a file this branch does not touch, flagged by this host's newer clippy (rustc
1.93.1). Pre-existing; not fixed (out of scope, and a test file).
`cargo fmt` ran on the four touched crates only; it also reflowed two untouched test files
(`holler-pane/tests/claude_code_rule_test.rs`, `holler-adapter-host/tests/foreground_test.rs`), which I reverted.
`cargo test --workspace` was not run (T-verify's).

## Evidence appendix
`docs/handoffs/696/evidence.md`

## Tests that look wrong (for T)
None. Note for T-verify: the guard's `try_wait(` / `recv_timeout(` checks strip `//` comments, so the doc-comment mentions
in `probe.rs` (`a `try_wait` error`) do not trip it (they contain no `(` after the word anyway).

## Known issues
- The brief's T-green gaps remain T's: the Claude runner has no direct timeout test through its seam, the OpenCode 64 KiB
  truncation is untested at the adapter level, and the host's uncapped output is untested.
- The ADR amend-first ordering (above).

## Files changed
- crates/holler-pane/src/bounded.rs
- crates/holler-pane/src/probe.rs
- crates/holler-adapter-host/src/exec.rs
- crates/holler-adapter-host/src/lib.rs
- crates/holler-adapter-opencode/src/exec.rs (deleted)
- crates/holler-adapter-opencode/src/lib.rs
- crates/holler-adapter-opencode/src/attach.rs
- crates/holler-adapter-claude/src/exec.rs (deleted)
- crates/holler-adapter-claude/src/lib.rs
- crates/holler-adapter-claude/src/seams.rs
- docs/adr/ADR-0021.md
- docs/pane-control.md
- CHANGELOG.md
