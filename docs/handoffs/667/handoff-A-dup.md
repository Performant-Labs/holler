# Handoff-A-dup: Phase 7 (the script's Phase 8) - #667 profile apply on scratch Herdr, tmux and OpenCode  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** issue-667-implementation (worktree `.claude/worktrees/0667-apply-scenario`)
**Diff base:** `bda1069`   **Diff head:** `367e921`
**Reuse map:** `docs/handoffs/667-brief.md` decisions 2 and 3, restated as the seven "Reuse expectations for Phase 7" in `docs/handoffs/667/handoff-A.md`
**Verdict:** PASS

## Summary

**PASS: no block, two low warns and one nit, none binding.** The diff extends the analogous objects the map named and builds no parallel path.

Who wrote what: T wrote all the code (Phase 3's W-1), and F wrote documentation only (one CHANGELOG entry and one deleted line of `docs/pane-control.md`).

What the diff extends:
- **`Scratch`.** The six helpers of decision 3 moved into a second `impl Scratch` (`scratch/verbs.rs`), and `launch_argv` is new.
- **The scratch guards.** `check_herdr_session_args` sits beside `check_session`, and `Scratch::holler` calls it.
- **`Scratch::launch`.** Its argv is now the one pure builder, `launch_args`.
- **`Out` and `output_of`.** They have one home, `pane_integration/out.rs`, which both targets include.

What the new target builds is what decision 2 lists as "Built new": the `Applied` reader, `Still`, the plan-against-show comparison and the A12 pin.

The warns are follow-up material:
- W-1: a fourth PATH lookup, placed outside the shared module;
- W-2: a test-side copy of production's `Action`/`Outcome` variant sets, forced by the no-production-code scope;
- W-3 (nit): two thin wrappers written twice inside the new target.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn (low; not binding) | `crates/holler-cli/tests/profile_apply_scenario/plan.rs:27-37` | **`assert_on_binary_path` is a fourth "program on a PATH" loop.** The other three are `rig::on_path` (`holler-adapter-opencode/tests/real_opencode/rig.rs:96`), `tmux_on_path` (`holler-adapter-host/tests/real_tmux/mod.rs:130`, #649's carried A-dup W-2) and `find_on_path` (`holler-cli/tests/wizard/proof.rs:179`).<ul><li>**Its meaning is right.** It searches the binary's `PATH` (`Settings.path`, which `Scratch::start` builds). The others search this process's `PATH`.</li><li>**Generalising `rig::on_path` was not an option.** It would edit an adapter crate's tests, which the brief's blast radius excludes.</li><li>**The problem is placement.** It checks the shared scratch set's `Settings` but lives in this target's `plan.rs`. The next scratch scenario (#831's, #839's) would not find it and could write a fifth.</li></ul> | Move it into `scratch/mod.rs`, beside `binary_env` (a free function or a `Settings` method). Use `std::env::split_paths`, as its three siblings do. List it with #649's unfiled test-glue consolidation (its W-O). |
| W-2 | warn (low; follow-up) | `plan.rs:39-59` | **`ACTIONS` and `OUTCOMES` copy by hand the variant sets of production's `tx_apply::Action` and `Outcome`.**<ul><li>**The repo's pattern** for a closed vocabulary is a production table that tests read: `FindingKind::ALL` (`holler-pane/src/findings.rs:80`), `SpecField::ALL` and `ImportFinding::ALL`. `docs_pane_control_test.rs:3-4` says it reads "from the code's own tables".</li><li>**`tx_apply` has no such table** (`tx_apply.rs:130-205`), and this story may not edit production. So the copy is the right call here, and it does take each name from production's own `as_str` (W-7).</li><li>**The cost.** A new production variant is refused only in the opted-in run. CI never notices.</li></ul> | **Follow-up**, for the next story that edits `tx_apply.rs`: add `Action::ALL` and `Outcome::ALL` in `FindingKind::ALL`'s shape, and point the reader at them. **Optional now**, inside this blast radius: an exhaustive `match` over each enum in `plan.rs`, so a new variant fails CI's compile. |
| W-3 | nit (not binding) | `profile_apply_scenario/scenario.rs:34-40`, `:72-80`; `probe.rs:28-34`, `:60-76` | **Local glue is written twice in the new target.**<ul><li>`Run::port` is byte-identical in `scenario.rs` and `probe.rs`.</li><li>A spec-only `pane launch` is wrapped twice: `scenario::Run::spec_only` and `probe::Run::spec_y`. Each prepends `--profile P --spec-only`, and each converts `launch_argv`'s `Vec<String>`, as `Scratch::launch` does.</li><li>Both go through the shared `launch_argv`, so there is still one argv builder (Phase 3's W-5 holds).</li></ul> | <ul><li>A `Scratch::launch_spec_only(pane, workspace, grid, port, profile, extra)` in `scratch/verbs.rs`, beside `launch`. It is the spec-only counterpart a later scenario will also want.</li><li>One port lookup in `plan.rs`.</li></ul> |

The rework introduced no architectural drift: no production code, manifest, ADR, wire format or `tests/support/` file changed.

## Checked and clean (the seven reuse expectations of `handoff-A.md`)

1. **`Scratch` is extended, not paralleled.**
   - **The moves are faithful.** I compared each moved helper hunk by hunk with the code deleted from #649's `scenario.rs`. The bodies are the same; only the formatting changed.
     - `assert_end_state`: `C1`, `C2` and `"hlr649"` became parameters.
     - `kill_server` is the server kill of `break_both`.
     - #649's call sites became method calls, and their assertions are unchanged.
   - **Nothing was copied.** `delete_strays` was not needed, and it was not copied (W-4 (a)).
   - **The new target's `Run` methods belong to the target.**
     - What they are: `apply`, `apply_in_session`, `show`, `still` and the spec-only edits.
     - What they bind: each test's ports and profiles. `apply` returns the target's own `Applied`.
     - Why not on `Scratch`: it is shared and verb-agnostic, so it would then depend on this target.
     - No free function re-wraps a `Scratch` operation (W-3 is the one thin exception, and both copies go through `launch_argv`).
2. **The guards are extended, not paralleled.**
   - `check_herdr_session_args` (`scratch/mod.rs:127`) sits beside `check_session`.
   - `Scratch::holler` calls it (`:349`), so it guards both targets.
   - It is pinned in `guards.rs:152`, with both spellings (W-2 of Phase 3).
   - It shares one constant with `launch_args`: `HERDR_SESSION_FLAG` (`:121`).
   - The new target has no guard of its own.
3. **`Out` and `output_of` have one definition.** It is `pane_integration/out.rs`, included by path at `profile_apply_scenario/main.rs:32-33`. There are four `struct Out`s before the diff and four after: `out.rs` (moved), `pane_verbs/process/main.rs`, `hold_cli_test.rs` and `grant_cli_test.rs`.
4. **Reused, never re-implemented.**
   - `support::{Hub, StateDir, wait_for, holler_bin}`: neither touched nor copied.
   - The rig's `free_port`, `refused`, `no_model_can_be_reached` and `signal_group`.
   - `check_envelope`, through `Scratch::json`.
   - None of the overlay's Phase-7 candidates has a copy in the diff: the `token.rs` operations, `Lockout`, `Roster`, `log(Severity, ...)`, `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`.
   - No wait is hand-rolled. The only poll is `kill_server`'s `wait_for`.
5. **`launch_args` (`scratch/mod.rs:157`) is the one argv builder.** The chain is `Scratch::launch` → `launch_argv` (`:462`) → `launch_args`. Both spec-only launches of the new target use `launch_argv`.
6. **The new objects are justified.** Each is listed under decision 2's "Built new" and in Phase 3's item 6.
   - **`Still`.** #649 has no counterpart.
   - **The `Applied` reader.** Production's `Applied` and `Step` derive only `Serialize` (`tx_apply.rs:96`, `:106`), so there is no deserializer to reuse. Names are parsed through production's `as_str`.
   - **`assert_plan_agrees_with_show`.** It has the shape of the fakes' `a_dry_run_and_profile_show_agree` (`profile_verbs/apply/dry_run.rs:143-178`), but that loop is inline in a test of another target and works on raw JSON. There was nothing to call.
   - **The A12 source pin's parser (`pins.rs:20-50`).** It has no counterpart: the repo's other source scans (`hold_single_path_test.rs`, `logging_guard_test.rs`) match lines, not attributes and function bodies.
7. **No fake host and no fake port.** No `LinkedHost` was added, so #846's three stay three.

**Also checked:**
- **The crate-level allows.** `profile_apply_scenario/main.rs` has the same five as `pane_integration/main.rs`, each ending `// #667`.
- **Size.** The largest touched file is `scenario.rs`, at 551 lines: under lint's 600 warning and the overlay's line of about 800.
- **Public repository.** The diff names no personal infrastructure.

## Notes for O

- **A CHANGELOG conflict with #851 on merge.**
  - `origin/main` gained `d6ae539` (#851) after this branch's base.
  - A trial `git merge-tree HEAD origin/main` conflicts in `CHANGELOG.md` only: both changes add the first entry under `### Enhancements`. Keep both entries.
  - `docs/pane-control.md` merges cleanly, since #851 edits a different hunk.
- **For #831 and #839.** Their scenario work should use the shared pieces this story made:
  - `Scratch::launch_argv`;
  - `kill_server`;
  - `assert_end_state`;
  - `assert_on_binary_path`, once W-1 moves it into `scratch/mod.rs`.

  Phase 3's W-8 held as predicted: `reset_with_a_first_message` now calls `scratch.session_of_record(C2)` and `scratch.record(C2)` (`pane_integration/scenario.rs:145`, `:147`).
- **In flight.** No branch edits the shared scratch files or this target. The only branch that touches them is `origin/issue-649-implementation`, which is #649's merged branch.

## Notes for F

None: PASS.
