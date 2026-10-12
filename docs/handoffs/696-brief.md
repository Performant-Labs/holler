# Brief: #696 one bounded subprocess runner shared by the adapters and run_probe

Repo: Performant-Labs/holler. Issue: #696 (part of epic #633). Rigor: in-session. UI surface: no. Kind: rework (behaviour-preserving refactor).

Written against `origin/main` at `05471b9`. All `file:line` references are to that tree. The GitHub issue and its one comment are the source of truth; ADR-0021 is the standing spec.

## Problem
Four copies of "run a child under a deadline, kill it on timeout, capture its output" exist: `holler-pane`'s probe runner and one private copy in each of the host, OpenCode and Claude adapters. The adapters may not depend on one another, so each copied the others by design and said so, to make this issue "a move". Four copies drift (they already differ in poll interval, output caps, stderr handling, kill scope and error texts, listed below). The refactor exposes one runner from `holler-pane`, switches the three adapters (and `run_probe`) to it, and deletes the private copies, with no change to any observable behaviour.

Two things in the issue text are stale and the brief corrects them:
- "`#641` has `exec.rs` and `tmux.rs`": in the OpenCode adapter there is no `tmux.rs`; the runner is `crates/holler-adapter-opencode/src/exec.rs` and its only call site is `attach.rs:217`. In the host adapter `tmux.rs` is parsing only (no process code); the runner is `exec.rs`. Verified: `grep -n "Command::new" crates/holler-adapter-*/src` finds `Command::new` only at `host/src/exec.rs:209`, `host/src/lib.rs:239`, `opencode/src/tui.rs:118`, `claude/src/seams.rs:127`, and `holler-pane/src/probe.rs:204,276`.
- "#641, #642, #663 merged": true. `run_probe` is real (`holler-pane/src/probe.rs:93`), not a stub.
- The comment on #696 adds `crates/holler-adapter-claude/src/exec.rs` (143 lines) to the scope.

## Evidence (verbatim excerpts)

**1. The pane runner, already split mechanics from verdict, and says #696 is its next step.**
```
crates/holler-pane/src/probe.rs:10-17
//! **The mechanics are kept apart from the verdict**, so #696 can lift them into the one
//! bounded runner the adapters share without a behaviour change: `run_bounded` (with
//! `spawn`, `start_reader`, `wait_for_exit` and `kill_and_reap`) runs the process and
//! answers an `Outcome`, and `verdict` alone turns that into a [`ProbeResult`]. Three
//! points are fixed here on purpose and are #696's to parameterize: **stderr** (discarded
//! here), **the signal and its grace** (`KILL` with no `TERM` grace: a probe is a
//! read-only check with nothing to clean up) and **the kill program** (`kill` from
//! `PATH`, with no seam for a fake).
```
```
crates/holler-pane/src/probe.rs:182-199
fn run_bounded(program: &str, args: &[String], deadline: Instant) -> Outcome {
    let mut child = match spawn(program, args) { Ok(child) => child, Err(e) => return Outcome::SpawnFailed(e.kind()), };
    let Some(output) = start_reader(&mut child) else { kill_and_reap(&mut child); return Outcome::ReaderFailed; };
    let outcome = match output.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Output::Ended(stdout)) => return wait_for_exit(&mut child, stdout, deadline),
        Ok(Output::Overflow) => Outcome::Overflow,
        Ok(Output::Failed) | Err(RecvTimeoutError::Disconnected) => Outcome::ReadFailed,
        Err(RecvTimeoutError::Timeout) => Outcome::TimedOut,
    };
    kill_and_reap(&mut child);
    outcome
}
```
(abridged to fit; the file is the reference.) Fixed behaviours: stdout piped and stderr `Stdio::null()` (`probe.rs:209`); `command.process_group(0)` on Unix (`:213`); stdout read to end FIRST, exit awaited after (the pid-reuse rule, `:19-24`); over-cap stdout is `Overflow`, an error, not truncation (`MAX_OUTPUT` 1 MiB, `:37`, `:235-237`); kill = group `KILL` through `kill` from `PATH` (`:276-281`) then `Child::kill`, reap within a 1 s `CLEANUP` budget (`:40`, `:259-267`); poll 10 ms (`:43`); no signal after a `try_wait` error (`:252`).

**2. Host adapter runner.**
```
crates/holler-adapter-host/src/exec.rs:59-90   pub(crate) fn run(command: &mut Command, deadline: Instant) -> Result<Ran, Failed>
```
Stdin null, stdout and stderr piped, both drained on two threads with NO output cap (`:105-113`); polls `try_wait` every 2 ms (`:21`, `:119-134`); on the deadline or a `try_wait` error `reap` = `child.kill()` then blocking `child.wait()` (`:139-142`); no process group; deadline already passed spawns nothing (`:63-65`). Result type `Failed::{Expired, Unavailable(String)}` (`:44-50`). Error texts: `"{program}: {e}"` (`:72`, `:125`), `"{program}: no thread to read its output: {e}"` (`:80`), `"{program}: its output was lost"` (`:158`). `Ran::reason()` = first non-empty stderr line else status (`:33-39`). The adapter's own `kill_group` (`:202-225`) builds `kill -s <SIG> -- -<pgid>` with `LC_ALL=C` and calls `run`; it classifies `No such process` from stderr. Callers: `host/src/lib.rs:245` (`run_tmux`), `:256` and `:273` (`kill_group`), mapping `Failed` at `:443-448`. The fake-`kill` seam is `TmuxHost::with_kill_binary` (`host/src/lib.rs:207-211`; field `kill: PathBuf` at `:180`, default `"kill"` at `:193`), exercised by `holler-adapter-host/tests/fake_tmux_test.rs` (`a_kill_failure_other_than_no_such_process_is_unavailable` `:648`, `a_failed_tag_kills_the_untagged_process` `:711`, `one_grace_is_shared_by_every_owned_group`, `stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor` `:580`) through `Fake::kills()` (`tests/common/mod.rs:116`).

**3. OpenCode adapter runner.**
```
crates/holler-adapter-opencode/src/exec.rs:61  pub(crate) fn capture(mut command: Command, op: &str, deadline: Instant) -> Result<Ran, PaneError>
```
Same shape as the host's but: poll 5 ms (`:24`), each stream capped at 64 KiB and the rest read and discarded (`:27`, `:141-143`), returns `PaneError` directly: `Timeout{op}` (`:187-189`) and `Unavailable` with texts `"cannot run {program}: {error}"`, `"cannot read the output of {program}: {error}"`, `"cannot wait for ..."` all passed through `crate::one_line` (`:183`, `:71`, `:78`, `:110`), `"cannot read the output of {program}: it was lost"` (`:165`, not one_lined). `Ran::reason()` uses `crate::one_line` (`:44`). Only call site: `attach.rs:217` (`exec::capture(tui::tmux_command(self.tmux, args), self.op, until)?`), import at `attach.rs:25`, module at `lib.rs:144`.

**4. Claude adapter runner.**
```
crates/holler-adapter-claude/src/exec.rs:38  pub(crate) fn capture(mut command: Command, op: &str, deadline: Instant) -> Result<Ran, PaneError>
```
Stdout piped and capped at 1 MiB with the rest discarded (`:26`, `:109-111`), stderr `Stdio::null()` (`:46`), single drain thread (`:97-116`), poll 5 ms (`:23`), `PaneError` texts `"cannot {doing} {program}: {error.kind()}"` (`:137`; the kind only, not the message), `"... it was lost"` (`:129`). Call sites: `seams.rs:45` (`OP_AGENTS`) and `:67` (`OP_LOGIN_STATUS`); import `seams.rs:21`; module `lib.rs:51`.

**5. Dependency direction.** All three adapters already depend on `holler-pane` (`host/Cargo.toml:17`, `opencode/Cargo.toml:17`, `claude/Cargo.toml:18`) and on nothing else of ours except as dev-dependencies. `holler-pane` depends on `serde`, `serde_json`, `holler-proto` only (`Cargo.toml:16-21`) and its probe runner uses `std` alone (`probe.rs:5-8`). So the refactor adds NO crate edge and NO dependency; the shared runner lives in `holler-pane` as the issue says. `holler-pane` lint set denies `dead_code`, `unwrap`, `expect`, `panic` (workspace `Cargo.toml`), so the runner must have no unused parameter or `pub(crate)`-only item.

**6. The behaviour differences a "shared" runner must absorb (the whole difficulty):**

| | pane probe | host | OpenCode | Claude |
|---|---|---|---|---|
| stdout | piped, cap 1 MiB, over = error | piped, no cap | piped, cap 64 KiB truncate | piped, cap 1 MiB truncate |
| stderr | null | piped, no cap | piped, cap 64 KiB | null |
| process group | own group | none | none | none |
| kill on deadline | group `KILL` via `kill`, then leader, bounded 1 s | `Child::kill` + `wait` | same | same |
| order | read stdout to end, then wait | wait, then collect | same | same |
| poll | 10 ms | 2 ms | 5 ms | 5 ms |
| failure type | `Outcome` -> `ProbeResult::Error(text)` | `Failed` | `PaneError`, one_lined | `PaneError`, `kind()` only |
| after `try_wait` error | no signal (pid reuse) | kill + wait | kill + wait | kill + wait |

## Acceptance criteria
1. `holler-pane` exposes one bounded runner (module and names: see Open decision 1). `run_probe` is built on it; its signature, its `ProbeResult` texts and the `Prober` trait are unchanged. Observable: `cargo test -p holler-pane` passes, every existing `probe.rs` test (`probe.rs:429-571`, e.g. `hung_command_is_error_at_the_timeout`, `timeout_kills_the_whole_process_group`, `output_over_the_cap_is_error`) unchanged and green.
2. The host, OpenCode and Claude adapters call the shared runner. `crates/holler-adapter-host/src/exec.rs` no longer contains `try_wait`, `recv_timeout`, `thread::Builder`, `read_to_end` or `child.kill`; `crates/holler-adapter-opencode/src/exec.rs` and `crates/holler-adapter-claude/src/exec.rs` are deleted. Observable: `grep -rn "try_wait" crates/*/src` lists only the shared runner's file(s) in `holler-pane`; `ls` shows the two files gone; `grep -rn "mod exec" crates/holler-adapter-{opencode,claude}/src/lib.rs` is empty. The host's `kill_group`/`Signal`/`Group` stay in the host (they are tmux/`kill` semantics, not a runner) and call the shared runner.
3. No behaviour change. Every existing test of the four crates passes unmodified: `cargo test -p holler-pane -p holler-adapter-host -p holler-adapter-opencode -p holler-adapter-claude`, including the fake-`kill` tests of `fake_tmux_test.rs` (`with_kill_binary` unchanged) and the OpenCode `ac30e_each_method_past_its_deadline_times_out_with_its_own_op` (`attach_test.rs:574`). Each adapter keeps its own error texts, caps, stderr handling, kill scope and poll (the table above), carried as parameters of the shared runner or mapped in the adapter. No existing test file is edited, except to follow a moved item with the same assertions (state any such edit in the handoff).
4. New tests (Test plan) pass: a source guard that exactly one bounded runner exists, and characterisation tests of the shared runner covering timeout, kill-on-timeout, captured stderr, output caps (truncate vs error), spawn failure, and a deadline already passed.
5. `cargo clippy --workspace --all-targets -- -D warnings` clean (the workspace denies `dead_code`; a parameter only one consumer uses must still be used). `cargo fmt` only on the touched crates, never the workspace.
6. `docs/pane-control.md:1007` ("the shared subprocess runner of the adapters and the probe (#696);") is removed from the deferred list, and the doc comments in `probe.rs:10-17`, `host/src/lib.rs` header and `host/src/exec.rs:1-9`, now stale, are updated. ADR-0021: see Open decision 5.

## Files
Production:
- `crates/holler-pane/src/probe.rs` (mechanics move out; `verdict`, `run_probe`, `ProbeResult` stay), new `crates/holler-pane/src/bounded.rs` (name: Open decision 1), `crates/holler-pane/src/lib.rs` (module and re-exports, around `lib.rs:52`, `:76`).
- `crates/holler-adapter-host/src/exec.rs` (keeps `Signal`, `Group`, `kill_group`; `run`/`Ran`/`Failed` become thin wrappers or are replaced), `crates/holler-adapter-host/src/lib.rs` (call sites `:245`, `:256`, `:273`, `Failed` mapping `:443-448`).
- `crates/holler-adapter-opencode/src/exec.rs` (delete), `src/lib.rs:144` (drop `mod exec`), `src/attach.rs:25,217` (call site).
- `crates/holler-adapter-claude/src/exec.rs` (delete), `src/lib.rs:51`, `src/seams.rs:5,21,45,67`.
- `docs/pane-control.md:1007`.

Tests: new `crates/holler-pane/tests/bounded_runner_test.rs` (characterisation) and `crates/holler-pane/tests/one_runner_guard_test.rs` (source guard). Existing tests untouched.

Reuse map (extend, do not duplicate): the shared runner IS `probe.rs`'s `run_bounded`/`spawn`/`start_reader`/`wait_for_exit`/`kill_and_reap`/`kill_group`/`wait_until`, generalised; the adapters' `drain`/`collect`/`wait`/`reap` are deleted, not ported. The source-guard idiom is `fake_tmux_test.rs:761` (`no_broad_kill_in_source`, with `collect_rs`). The scratch-dir and polling helpers in `probe.rs`'s inline tests (`:329-427`) are the model for the characterisation tests (they cannot be imported across `tests/`; copy the minimal guard, as #663 did).

## Decisions already made (MO)
From the issue and the repo's standing rules (not re-opened here):
- The shared runner lives in `holler-pane` (issue scope). No crate edge is added.
- No behaviour change (issue scope). Where the four copies differ, the difference is kept as a parameter or an adapter-side mapping.
- The host's fake-`kill` seam (`with_kill_binary`) keeps working.
- `run_probe`'s signature, `ProbeResult` and its texts are frozen (ADR-0021 section 1).
- Recommended by A/F reading, flagged for confirmation below where they involve a choice.

## Open decisions (NOT decided here; recommendation given)
1. **Public API of `holler-pane`.** This adds public surface to a crate the ADR calls the contract crate. Recommendation: one new `pub mod bounded` (file `src/bounded.rs`) exporting a spec/builder plus one function, e.g. `run(command: Command, spec: &Spec, deadline: Instant) -> Result<Ran, RunError>`, where `Spec` carries: stdout `Capture` (`Discard` | `Keep{cap, over: Truncate|Fail}`), stderr `Capture` (same enum), `kill: Child | Group`, `poll: Duration`. `Ran { status, stdout: Vec<u8>, stderr: Vec<u8> }` (bytes, because the probe matches bytes; adapters do `String::from_utf8_lossy`). Re-export from `lib.rs` next to `run_probe`. Keep `Spec`'s fields minimal: only what a consumer sets today, so no `dead_code`.
2. **Error type.** The three adapters use three different message formats (table row "failure type"), and tests may pin them. Recommendation: the runner returns a typed `RunError` (`Expired`, `Spawn(io::Error)`, `ReaderFailed`, `WaitFailed(io::Error)`, `OutputLost`, `Overflow`) and each adapter (and `verdict`) maps it to its existing text, so no string changes. Returning `PaneError` directly from the shared runner would unify the texts and is a behaviour change; reject unless the operator wants it.
3. **Kill program seam and `Group` scope.** `probe.rs:14-17` calls the kill program "#696's to parameterize". No adapter uses the runner's group kill (host signals through its own `kill_group`; OpenCode and Claude kill the child only), and switching them to process-group isolation would change behaviour (children would leave the terminal's foreground group). Recommendation: do NOT add a kill-program field now (no consumer; YAGNI; `dead_code` is denied); adapters keep `KillScope::Child`; the probe keeps `Group` with `kill` from `PATH`. Say so in the doc comment so the "#696's to parameterize" note is closed, not left dangling.
4. **Unify the wait order and the post-`try_wait`-error rule.** The probe reads stdout to the end first and never signals after a `try_wait` error (pid-reuse safety, `probe.rs:19-24`); the adapters wait first and `kill`+`wait` after an error. Recommendation: one engine on the probe's order and rule. Outcomes are the same for every reachable case (hung child, held pipe, early close); the only change is the unreachable "something else reaped our child" path, where the adapters would no longer call `Child::kill` (which std treats as a no-op on a reaped child). That is a deliberate, tiny, safer deviation from "no change"; confirm, or the engine needs a per-spec flag (not recommended).
5. **ADR-0021 amendment.** Section 2 (the `Prober` paragraph) and section 5 describe the runner as `run_probe`'s. Recommendation: a short amend-first note in ADR-0021 (section 5, crate layout) that `holler-pane::bounded` is the one runner for the probe and the adapters, no contract change; if the operator prefers no ADR edit for a pure refactor, doc comments plus `pane-control.md:1007` suffice.
6. **Poll interval.** 2/5/5/10 ms differ. Recommendation: keep it as a `Spec` field in this PR (strictly no change); collapsing to one value is a later, separate choice.

## Out of scope
- Any behaviour change (list in the table stays as is), including capping the host's uncapped output, putting adapter children in a process group, adding a `TERM` grace to the probe, or localising error texts.
- `holler-adapter-herdr` (`transport.rs:111` uses `recv_timeout` for a socket exchange, not a subprocess) and the hub/body/load-test crates.
- Moving the host's `kill_group` into `holler-pane`, or merging it with the probe's best-effort `kill_group` (`probe.rs:272`): they differ in purpose (the host's classifies `No such process` and honours a fake binary). Record as a possible follow-up, not this PR.
- Bumping versions, the ADR numbering, any live Herdr/tmux state, and `cargo fmt` outside the touched crates.

## Test plan
A behaviour-preserving refactor has no failing behaviour to turn green, so T's RED is honestly structural and gets two forms, both authored before F:

**RED-1, source guard (fails on an assertion today).** `one_runner_guard_test.rs` in `holler-pane/tests` walks `crates/*/src/**/*.rs` (idiom of `fake_tmux_test.rs:761`, comment text stripped as there) and asserts that `try_wait(` occurs only in `holler-pane/src/` files of the shared runner, and that `crates/holler-adapter-{host,opencode,claude}/src/exec.rs` contain no `fn wait`, `fn drain`, `fn collect` or `fn reap`. Today it fails and names the three adapter files (plus `probe.rs` if the runner module is a separate file). It goes GREEN only when the private copies are gone. Note its limit honestly in the handoff: it proves "only one runner", not "same behaviour".

**RED-2, characterisation through the new public API.** `bounded_runner_test.rs` pins what each current copy does, using real children of harmless commands (`sh -c`, `sleep`, `printf`), each in a scratch dir. For RED by assertion rather than compile failure, T lands the public signatures first as a stub (the repo's own #637 pattern: a `run` that answers `Err(RunError::Unavailable)` with the real types), so the tests fail on assertions, not on missing symbols. Cases, one per adapter behaviour in the table:
- timeout: a `sleep 5` with a 200 ms deadline returns `Expired` within deadline + 1 s;
- kill on timeout: the child's pid is gone afterwards (`ps` read only, as `probe.rs:386-403` does), for `KillScope::Child` and for `Group` (grandchild also gone);
- captured stderr: `sh -c 'echo err >&2; exit 3'` with stderr `Keep` returns status 3 and `err\n`; with `Discard` returns empty stderr and no stderr drain thread is needed;
- caps: output of 200 KiB with `Truncate(64 KiB)` returns exactly 64 KiB and the child still exits 0 (does not block); with `Fail(1 MiB)` over-cap is `Overflow`;
- held pipe: a child that exits while a background process holds stdout is `Expired` at the deadline, never a verdict;
- spawn failure: a missing program is `Spawn` with `io::ErrorKind::NotFound` and no argument in any text;
- a deadline already passed spawns nothing (assert with a marker file that the command never ran).

**GREEN (F).** Move the mechanics, switch `run_probe` and the three adapters, delete the copies. T(verify) runs the four crates' full suites and the guard, and runs `cargo test --workspace` once. The existing adapter tests (fake `kill`, `ac30e`, hermetic frozen-server timeouts such as `hermetic_test.rs:123`) are the regression net for AC 3; T lists in the handoff any adapter behaviour with NO existing test (the Claude runner has no direct timeout test: only the seams' hermetic tests exercise it) and adds one characterisation case per such gap through the adapter's public seam, not through `exec`.

## Risks
- The table is long and the unifying engine is easy to get subtly wrong (read-then-wait vs wait-then-collect, who drains after the cap). Mitigation: Open decisions 4 and 2, and the characterisation tests above mirror the table row by row.
- Error text drift: some adapter test or the S audit may pin a text (e.g. `"cannot run"`, the `kind()`-only Claude form). F must grep tests for the old strings before choosing Open decision 2's mapping; A reviews the diff for any string that changed.
- The probe's documented limitation (a child escaping the group leaks one reader thread, `probe.rs:88-92`) now also applies, unchanged, to adapters that already detached their drain threads; no new leak is introduced, but the doc comment must say it once in the shared runner.
- Public API creep in the contract crate (Open decision 1).
- Time: the `try_wait` polling tests are timing-based; keep the deadlines generous (existing tests use 2 s grace).

## Clashes (siblings, files only)
- **#844 (in flight, OpenCode adapter locale):** `tmux_command` `-u` in `crates/holler-adapter-opencode/src/tui.rs` (its definition is at `tui.rs:117-134`), the fake-tmux fixture, `pane_integration` `scratch/mod.rs` and `guards.rs`. Its call site is `attach.rs:217`, the very line this brief rewrites (`exec::capture(tui::tmux_command(self.tmux, args), ...)`). Expected: a one-line textual conflict at `attach.rs:217` if #844 also touches that call; no overlap in `tui.rs`, which this brief does not edit (`tmux_command` still returns a `Command`, which the shared runner takes, so #844's change composes). Land #844 first if it is close; otherwise rebase this one.
- **#846 (in flight):** `holler-cli` tests `verb_harness` / `linked_host.rs` and the pane rigs. This brief touches no `holler-cli` file. No overlap. (If a rig spawns `exec` via a real adapter, its tests still pass by AC 3.)
- **#845 (parked):** OpenCode `attach.rs` `select_session` (`attach.rs:63`). Same file as the `use crate::exec;` (`:25`) and `:217` edits; different hunks, so a clean merge is expected, but a later #845 rebase will touch the same file.
- #831 / PR #857 (merged) created the Claude `exec.rs`; nothing in flight edits `crates/holler-adapter-claude/src/exec.rs` or `seams.rs` that I could find (check `git log origin/main -- crates/holler-adapter-claude` before F starts).
