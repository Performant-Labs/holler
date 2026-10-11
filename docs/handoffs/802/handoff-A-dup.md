# Handoff-A-dup: Phase 7 - #802 `HarnessPort::serve_command` and `ServeSpec`; the server runs in the pane's tmux session  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`)
**Diff base:** `92fbc6e` (merge base with `origin/main`)   **Diff head:** `9861930`
**Reuse map:** no `survey.md` in this automated run. The map is the brief's Scope (`docs/handoffs/802-brief.md`, as amended in `0dc4e8b`) plus the Phase 8 lists in `handoff-A.md` (round 2, "Notes for O") and round 1 (`git show 0421ec1:docs/handoffs/802/handoff-A.md`).
**Verdict:** PASS

## Summary

**Verdict: PASS, with no blocks.** F extended every object the map named. No production code builds a parallel path.

**How each object was extended:**

- **The ports.** Both ports are amended in place: `serve` became `serve_command` plus `verify_serving`, and `run` gained `cwd`. `ServeSpec` sits beside `HerdrSpec`, and `AGENT_UNSUPPORTED` beside `PROMPT_UNSUPPORTED`.
- **The OpenCode adapter.**
  - The env prefix is extracted once from `tui_argv` into `ProcessEnv::prefix`, and `TITLE_OFF` moved with it.
  - The UTF-8 check of the binary path is extracted from `attach.rs` into `OpenCodeConfig::bin()`.
  - One `refuse_an_agent` answers for both `serve_command` and `verify_serving`.
  - The old spawn, boot poll, reaper and group kill are deleted, not kept alongside.
- **The host adapter.**
  - It reuses `escape_cwd` for `new-window -c` and `tmux::panes` for the existence read.
  - `ensure_session` and `run` share one directory rule, `is_usable_dir`.
- **The engine.**
  - One `ServeSpec` per run, built in `Plan::new`; the argv is kept in `Seen`.
  - `port_in_use` is shared by the plan's check and the act's re-check.
  - `stop_old` moved into `tx_launch/server.rs`, not copied. The child-module layout follows `reconcile/observe.rs`.
  - `wait_healthy` is the only health-wait loop. The engine, the conformance suite and the real rig all call it.
- **The test kit.**
  - `World::serve` became `World::start`, which `start_server` reaches.
  - `FakeHost::runs()` gained the `cwd`, so there is still one argv log, and the two fakes still share no state.
  - `LinkedHost` and the existing `CASES` table and runner were extended, not copied.

**What is left.** The duplication is small and lives in test code only. It is recorded as W-1 and W-2.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn (low; test code; follow-up, optional) | `crates/holler-cli/tests/pane_verbs/doctor/rig.rs:388`, `crates/holler-cli/tests/pane_verbs/launch/rig.rs:453` (`Rig::serve`), `:347-364` (`seed_live`), `crates/holler-pane-testkit/tests/fake_harness_test.rs:85`, `crates/holler-cli/tests/pane_verbs/launch/serve.rs:509-517` | **What repeats.** Starting a fake server for setup takes two calls: `serve_command(name, port, &ServeSpec::default())`, then `start_server(&argv)`. Three private helpers and two inline copies repeat that pair. Before #802 each site was one call, `FakeHarness::serve`. **Why it is a warn.** The brief's Scope 4 told setup code to call the scenario method, so this follows the map. No existing object was bypassed. **Pattern point.** The kit's own pattern for a setup step is one method on the fake, as `seed_session` is. **Stale doc.** `doctor/rig.rs:5` still says the rig starts each server with `serve`. | Add one setup method on `FakeHarness`, for example `start_for(name, port) -> Result<u32, PaneError>`: `serve_command` with the default spec (logged, as setup through the port is today), then `start_server`. Use it at all five sites. Fix the doctor rig's module doc. If no one takes this now, do it the next time a rig needs to start a server. |
| W-2 | warn (low; test code) | `crates/holler-cli/tests/pane_verbs/launch/serve.rs:116-134` (`Seen`, `snapshot_into`), `:32` (`STEP_PHRASE`); `crates/holler-cli/tests/pane_verbs/launch/rig.rs:465-475` (`Rig::mark`); `crates/holler-cli/tests/pane_verbs/launch/profiles.rs:17` | **`Seen`.** `Seen` and `snapshot_into` rebuild part of the rig's `Calls` from the same fake logs that `Rig::mark` reads. A hook gets `&Fakes` and not `&Rig`, so it cannot call `mark`. This is a snapshot of the existing logs, not a second log, so W-8 holds. **`STEP_PHRASE`.** The constant is now declared twice, in two sibling modules of one rig. | Move `mark`'s body into `pub(crate) fn calls_of(f: &Fakes) -> Calls` in `launch/rig.rs`. Have both `Rig::mark` and the hook call it, and drop `Seen`. Move one `pub(crate) const STEP_PHRASE` into `launch/rig.rs` beside `STEP`. |
| W-3 | warn (watch; size) | `crates/holler-pane/src/tx_launch.rs` (820 lines), `crates/holler-cli/tests/pane_verbs/switch.rs` (833), `crates/holler-adapter-host/tests/fake_tmux_test.rs` (818), `crates/holler-adapter-opencode/tests/hermetic_test.rs` (813), `crates/holler-cli/tests/pane_verbs/launch/rig.rs` (793) | **The lint.** All five files are under the 900-line fail, and `scripts/lint.sh` passes. **`switch.rs`.** It grew from 827 to 833 lines. The brief asked for a line-neutral edit there, or for its two doubles to move to a sibling file. **`tx_launch.rs`.** It is at 820. #802 put its own new code in `tx_launch/server.rs` as planned. #644 part 2 and #664 also edit this file. | **The next story that touches `tx_launch.rs`** names a child module for its code, as #802 did. **`switch.rs`.** Moving `WriterInSelect` and `SeamHarness` to a sibling file is a cheap follow-up. **#642 part 3.** Its brief (D8) already keeps its code out of `hermetic_test.rs` and the OpenCode `lib.rs`. |
| W-4 | warn (reading note for #642 part 3; no change in #802) | `crates/holler-pane/src/ports.rs` (the `verify_serving` doc, round 3) | **The sentence.** Round 3 added "A spec that the implementation's own argv already settles is answered `Ok(())` with nothing sent." **The reading that holds.** "Settles" covers only a field that the argv fixes by itself, such as a `None` agent. **Why an agent key is different.** The argv never settles an agent key: the server resolves the key from its own config, and checking that is the point of decision 5. **What already binds it.** The trait doc's first rule, "`Ok` only when the server honours every field of `spec`", already binds this reading. #642 part 3's D4 checks `GET /config` and `GET /agent`, and its D9 allows the doc edit. This is not drift. It matches decision 5 and A's W-3. | None in #802. When #642 part 3 edits this doc (its D9), it should say that an agent key is always checked on the server. |

**The Phase 8 checks named in round 2.**

- **One health-wait loop.** There is exactly one, `wait_healthy` (`tx_launch/server.rs:44-59`). It checks its deadline at the top of the loop, before every ask. The only other poll loop in `holler-pane` is `probe.rs:297` (`wait_until`, over `Child::try_wait`). It predates #802 and asks in a different order on purpose (round 2, W-4), so it is not a copy to merge.
- **No duplicate budget check.** No `budget.check()` repeats the wait's own check: `server.rs:102` has none between `run` and the wait. The check at `:106` guards `verify_serving`, which is a separate live step.
- **`BOOT_POLL`.** The real rig imports `BOOT_POLL` (`real_opencode/rig.rs:39`) instead of copying it.

**Round 1's rejection list.** Nothing on it is present.

- **The argv log.** No second argv log or recording wrapper: `FakeHost::runs()` is extended, and `serve_specs` is required by the brief.
- **The suite.** No second conformance runner or table: there is one `CASES` (`conformance/harness.rs:98`) and one `run_harness_conformance(fresh, launch)` (`:176`).
- **The escape.** No new escape for `-c`: `tmux.rs:71` calls the existing `escape_cwd`.
- **The env prefix.** No second env-prefix builder: `ProcessEnv::prefix` (`holler-adapter-opencode/src/lib.rs:176`) is called by `tui.rs:173` and `server.rs:33`.

**The overlay's Phase 7 candidates** are the token store, `Lockout`, `Roster`, `log(Severity, ...)`, `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`. The diff copies none of them. Its only hub change, `pane_registry_test.rs`, reuses `temp_state`, `write_registry`, `v1_file`, `load` and `registry_file`.

**Considered, not findings.**

- **The real rig's own spawn.** `Recording::launch_on` (`real_opencode/rig.rs:208-245`) starts a process group of its own in test code. The brief requires it: ADR-0021 section 5 forbids that crate a dependency on the host adapter. It does not bring the deleted adapter code back into production. Its wait is the shared `wait_healthy`.
- **The re-executed credential test.** `hermetic_test.rs`'s credential test re-runs the test binary as a child. That is the idiom `fake_tmux_test.rs` already uses (`env_probe_child`), and each test binary needs its own copy.
- **The delegating doubles.** `HookedHarness`, `HealthGate`, `WriterInSelect`, `SeamHarness`, `Recording`, `Mutant`, `TestHarness` and `Unwired` all existed before. #802 updates them in place and adds no new wrapper.
- **The fake's code.** `AGENT_MISMATCH` (`agent-mismatch`) is the fake's own open code, which the trait doc allows ("an open code the implementation owns"). It does not copy `AGENT_UNSUPPORTED`: the two mean different things.

**Drift introduced during rework (rounds 2 and 3).** None.

- **Round 2.** It changed `wait_healthy`'s check order, computed the budget's end once, imported `BOOT_POLL` and added a CHANGELOG clause. Each matches round 2's W-4, W-14 and W-16.
- **Round 3.** It changed docs only: `git diff 2a32fc8 7ee7d8e -- crates/` holds only comment lines. The ADR, the trait doc, the engine's module doc and the CHANGELOG now agree on decision 5 (see W-4).
- **Tests at head.** `cargo clippy --workspace --all-targets -- -D warnings` is clean. Four test targets pass:
  - `pane_verbs`: 262;
  - the test kit's targets, including `harness_conformance_test` (15) and `fake_harness_test` (23);
  - `hermetic_test`: 31;
  - `fake_tmux_test`: 29.

## Notes for F

PASS: there is no parallel path to fold. W-1 and W-2 concern test code, which is T's. They do not need a rework round.

## Notes for O

- **Optional test cleanup.** W-1 and W-2 together are about 30 lines across four test files. O can open one follow-up for them, or hand them to the next story that edits `launch/rig.rs` or `doctor/rig.rs`.
- **File size.** W-3 is a watch for #644 part 2 and #664, which also edit `tx_launch.rs`, now at 820 lines.
- **#642 part 3.** W-4 is a reading note to carry into that story.
- **Carried from rounds 1 and 2, unchanged.** W-11(a) (with #649), W-13 (the merge order with #664) and W-15 (the plan's reads under I5).

## Patterns referenced

- `crates/holler-pane/src/tx_launch.rs` and `tx_launch/server.rs`, after the child-module pattern of `reconcile.rs` and `reconcile/observe.rs`; `probe.rs:297` (`wait_until`).
- `crates/holler-adapter-opencode/src/{lib,server,tui,attach,exec}.rs` (`ProcessEnv::prefix`, `OpenCodeConfig::bin`, `refuse_an_agent`, and the deletions).
- `crates/holler-adapter-host/src/{lib,tmux}.rs` (`check_cwd` / `is_usable_dir`, `escape_cwd`, `tmux::panes`).
- `crates/holler-pane-testkit/src/{harness,host}.rs` and `src/conformance/harness.rs` (`World::start`, `start_server`, `runs()`, `CASES`, the launcher).
- `crates/holler-cli/tests/pane_verbs/launch/rig.rs` (`LinkedHost`, `fire`, `Rig::mark`, `Rig::serve`), `doctor/rig.rs`, `launch/serve.rs`; `crates/holler-adapter-host/tests/fake_tmux_test.rs` (`env_probe_child`).
