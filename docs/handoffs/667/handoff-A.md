# Handoff-A: Phase 3 - #667 profile apply on scratch Herdr, tmux and OpenCode  (up-front plan review)

**Date:** 2026-10-11
**Branch:** issue-667-implementation (worktree `.claude/worktrees/0667-apply-scenario`, head `bc965ae`, base `bda1069` = `origin/main`)
**Brief reviewed:** `docs/handoffs/667-brief.md` at `bc965ae` (sha256 `ed09f85f35d2a8c4...`, 2793 lines, read in full)
**Reuse map:** no `survey.md`. The brief's decisions 2 and 3 and its Evidence (E1 to E76) serve as the map; "Reuse expectations for Phase 7" below restates it for the anti-duplication gate.
**Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

**Verdict: PASS, with no block and nine warns. Five of the warns bind T or F (W-1 to W-5).**

**The plan extends the right object, at the right layer.**
- It drives the real `holler` binary through a real hub (decision 1), so the private `Wiring::from_env` (`crates/holler-cli/src/pane/wiring.rs:132`) stays the only place that assembles the ports.
- It reuses #649's scratch set whole (decision 2) and moves #649's private helpers into `impl Scratch` once (decision 3). That is a spec'd refactor-to-extend, the pipeline's desired path, and it avoids the near-copy #649's own A-dup gate warned about (E67).
- It changes no production code, no manifest, no ADR and no wire format.

**The mechanics the plan depends on hold.** I checked each against the tree; one I compiled.
- **The doubly nested include compiles.** Decision 2 marks it "unverified". I compiled a model of the exact layout in the scratchpad (rustc 1.98.1), and both targets build and run. The included rigs reference nothing at #649's crate root. `scratch/mod.rs` needs only `crate::support::{holler_bin, wait_for, Hub, StateDir}` and `crate::{output_of, Out}` (`scratch/mod.rs:60-61`), and decision 3's `out.rs` provides the second pair in both targets.
- **Step 2's spec-only launch of `B1` is executable**, although `B1` is a live pane in no profile:
  - `tx_launch::launch` returns at `spec_only` before its `pane-exists` check (`crates/holler-pane/src/tx_launch.rs:195-201`);
  - `StoreScope`'s `check_joins` refuses a spec `Set` only for a pane in another profile (`crates/holler-cli/src/pane/profile_scope.rs`).
- **Decision 9's checks can be stored.**
  - `profile-secret-refused` applies only to an environment entry that carries a value (`crates/holler-pane/src/error.rs:449`).
  - `/global/health` is the adapter's own health route (`crates/holler-adapter-opencode/src/server.rs:34`).
- **No other file needs to change with this story.**
  - ADR-0021's only line about #667 (1030-1031) states what the plan does.
  - `docs_pane_control_test.rs` does not pin section 12 (no match for `667`, `Later`, `## 12` or `deferred`).

**What the warns fix.**
- **W-1: the plan's pipeline flow.** It defines RED the way the tester role calls invalid, assumes an F skip the script does not have, and leaves the CHANGELOG entry and the guide's bullet with no owner.
- **W-2:** the new argument guard must also cover `--herdr-session=VALUE`.
- **W-3:** test A is too long for clippy's denied `too_many_lines` unless it is split into step functions.
- **W-4:** further helpers move rather than copy, and `pane doctor` is a write that must stay outside every `Still` window.
- **W-5:** `launch_args` must be pure, so its pin can run in the default run.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn (**binding on T and F**) | Decision 13 (pipeline), decision 14 (CHANGELOG and guide) | pattern consistency (pipeline roles) | Decision 13's RED, "the pins failing to compile ... before the helpers exist", is what `tester.md` rules out ("not a missing import"). The script cannot skip F. The CHANGELOG entry and the section-12 bullet have no owner. | T-red writes all test code and shows each pin failing against the broken variant the Acceptance names. F writes the CHANGELOG entry and deletes the bullet, as #652's F did. Details below. |
| W-2 | warn (**binding on T**) | Decision 3's one added guard; A13 | cross-cutting (the scratch guards) | The guard and A13 name only `--herdr-session X`. clap also accepts `--herdr-session=X`, so `--herdr-session=default` would pass the guard. | Refuse the `=` form unless its value is the scratch session. Add `=default`, `=` and `=holler649-<other>` to A13's refusals and `=<scratch>` to its passes. |
| W-3 | warn (**binding on T**) | Decision 7's two tests | size and structure | The workspace denies `clippy::too_many_lines` (100) and `cognitive_complexity` (15). CI lints `#[ignore]` tests too. Test A inline (steps 1 to 5, 7 and the end) would be several hundred lines. | One function per step, and per phase of test B, as #649 splits its scenario. No allow for either lint: no test target in the repo has one. |
| W-4 | warn (**binding on T**; checked at Phase 7) | Decision 3's list of moves; decision 8's `Still` | reuse; pattern consistency | (a) A helper of `scenario.rs` that #667 later needs (e.g. `delete_strays`) is copied rather than moved. (b) `pane doctor` writes records by compare-and-swap. A `Still` window that contains `doctor` (through `findings()` or `assert_end_state`) measures doctor's write, not apply's. | (a) Move it into `impl Scratch` as decision 3's helpers are moved; a copy is a Phase-7 block. (b) No `doctor` run inside a before/after window. |
| W-5 | warn (**binding on T** for purity; placement recommended) | Decision 3's `launch_args`; A14 | pattern consistency | A14 pins `launch_args` in the default run, where `Scratch::start()` is `None`. A `&self` method cannot be pinned there. | An associated or free function, with the Herdr session and project directory as arguments, returning owned `String`s. Its pin goes in `guards.rs` beside A13's. |
| W-6 | warn (low; not binding) | Decision 3's moves into `scratch/mod.rs` | size | `scratch/mod.rs` is 467 lines. The moves, `launch_args` and the guard add about 140 to 150 lines (my estimate), so about 610. That reaches `scripts/lint.sh`'s 600-line warning, which exits 0. | If it crosses 600, put the moved helpers in a child module `scratch/verbs.rs` (a second `impl Scratch`; I compiled this layout under both includes). Moving `findings()` is optional: no step of #667 calls it. |
| W-7 | warn (not binding) | Decision 2's "small reader of apply's `data.steps`" (`plan.rs`); A14 | reuse | A new reader is justified, but test-side `Action`/`Outcome` enums or string lists would copy the production vocabulary. | Parse each wire name against `holler_pane::tx_apply::{Action, Outcome}`'s own `as_str()`. A name that no variant has is A14's refusal. |
| W-8 | warn (not binding; for O and #839) | The Clashes table's `scenario.rs` row | coordination | "Disjoint hunks" is not quite true. Making `record` and `session_of_record` methods rewrites the calls at `scenario.rs:194` and `:196`, which sit inside `reset_with_a_first_message`, the function #839 edits. | Keep the method form. Tell #839 it rebases over two changed calls. Nothing in flight conflicts today (see "In flight"). |
| W-9 | warn (not binding; for O) | The issue's Scope and Blast radius | spec traceability | The issue still says "its own harness", "in-memory `PaneStore` and `ProfileStore`", "does not depend on #649" and a radius "disjoint from #649's `pane_integration/**`". The brief supersedes all four, and I agree with each reason. S audits against acceptance. | Record the supersession on #667 (a comment linking decisions 1 to 3; outward, so O's or the operator's). |
| nit | nit | Scope 4; the run line | facts; docs | `tests/support/mod.rs` is 857 lines at `bda1069`, not 849 (immaterial: not edited). `docs/testing.md` documents #640's opt-in run but neither #649's nor this one. | None for this story. #666, whose step 0 is this rehearsal, or a docs follow-up can add both run lines there. |

### W-1 in detail: who writes what, and what RED is

**The conflict.**
- **RED.** `tester.md`, Phase 4 step 2, says every new test "must fail for the right reason ... *not* a missing import, a typo, or a setup error". It adds: "Do not implement the feature to make a test pass — that is F's job". Decision 13's RED is a compile failure from helpers that do not exist yet. Yet `feature.md:161` says "F never edits tests", and every helper of this story lives under `crates/*/tests/**`, which is `paths.test` (E70).
- **No F skip.** The Workflow script always runs phase F and commits after it (`coding-pipeline.workflow.mjs:5640-5666`). F answers `F_RESULT_SCHEMA` `{done, archChanged}` (`:224`). There is no skip and no O in this run, so "O records phase F as skipped" cannot happen.
- **No owner for two edits.** The CHANGELOG entry and the deleted guide bullet (decision 14) are neither test nor production paths. The brief names neither T nor F for them.

**The plan already contains a valid RED.** Each Acceptance pin states its own failing variant:
- A12: one `#[ignore]` removed;
- A13: a guard that ignores the flag;
- A14: a missing key, an unknown action or outcome name, and `launch_args` against the pre-move flag list.

The scenario's RED is the production mutants of A16, which only an opted-in run can show.

**The split, binding:**
- **T-red** writes all the test code:
  - decision 3's moves and `out.rs`;
  - the `--herdr-session` guard;
  - `launch_args`;
  - `plan.rs` (the reader and `Still`);
  - `pins.rs`;
  - both scenarios.

  T then shows each pin failing for the right reason against its named broken variant, and restores the code. It also shows #649's default run passing unchanged after the moves. Each RED goes in `handoff-T-red.md` with its failing output.
- **F** adds the CHANGELOG entry under `## [Unreleased]` / `### Enhancements` (decision 14's text) and deletes the one bullet of `docs/pane-control.md` section 12 (line 792). It touches no file under `crates/`. It runs `scripts/changelog-check.sh`, `docs_pane_control_test` and `docs_cli_test`, and reports `archChanged: false`. This is #652's precedent, where F wrote the guide and its CHANGELOG entry (`docs/handoffs/652/decisions.md:120-124`). It gives F a real diff, which answers the brief's Question 2 and its "unverified" note in decision 13.
- **T-green** checks GREEN: the pins, #649's default run, the docs tests, clippy and `lint.sh`. It also runs the opted-in run of both targets twice, with the `ps` and `/tmp` check (A1), and the A16 mutants.

### W-2 in detail

**Why the guard is incomplete.**
- clap accepts both `--herdr-session X` and `--herdr-session=X`.
- `crates/holler-cli/src/` enables no `infer_long_args`, so there is no third spelling.
- A guard that looks only for the two-token form lets `["profile", "apply", "P", "--herdr-session=default"]` through. The adapter's refusal of a session other than its configured one (E75) is a second layer, not this guard.

**Fix, binding.** The pure function, beside `check_session` in `scratch/mod.rs`, refuses all of the following:
- `--herdr-session` with no value, or with a value that starts with `--`;
- `--herdr-session=` with an empty value;
- either form whose value is not `settings.session`.

A13 in `guards.rs` gains the `=` cases.

### W-3 in detail

**Why the tests must be split.**
- `Cargo.toml` `[workspace.lints.clippy]` denies `too_many_lines` and `cognitive_complexity`, and `clippy.toml` sets their thresholds at 100 and 15.
- CI runs `cargo clippy --workspace --all-targets -- -D warnings` (`.github/workflows/ci.yml:348`). That lints the `#[ignore]` scenario tests as well.
- `pane_integration/main.rs` allows only `unwrap_used`, `expect_used`, `panic`, `unreachable` and `dead_code`. No test target in the repo allows either size lint.

**Fix, binding.**
- **Test A** calls one function per step, each taking `&Scratch` and the names and ports it needs: steps 1, 2, 3, 4, 5, 7, then the end state. #649 splits its scenario the same way, into `reset_with_a_first_message`, `break_both` and `assert_end_state`.
- **Test B** gets one function per phase: (i), (ii) and (iii).
- **The crate-level allows** of `profile_apply_scenario/main.rs` are `pane_integration/main.rs`'s five, each with `// #667`. There is none for `too_many_lines` or `cognitive_complexity`.

### W-4 in detail

**(a) Helpers move, never copy.**
- Decision 3 moves the helpers #667's table uses.
- Suppose the end state's `doctor` reports a stray session after apply's relaunches. #649 removes strays with the private `delete_strays` (`scenario.rs:47-52`). That helper then moves into `impl Scratch` the same way.
- The same holds for any other helper of `scenario.rs`. A copy in `profile_apply_scenario/` is a Phase-7 block.

**(b) `pane doctor` is not a read.**
- Reconcile's step 4 records what it observed by one compare-and-swap of the pane's record. It always writes on the first observation (`at == 0`), and writes whenever health or SHOWN changed (`crates/holler-pane/src/reconcile.rs`, module doc). So it moves `generation`, which decision 8 puts in `Still`.
- `Scratch::findings()` and `assert_end_state` both run `doctor`.
- By contrast, `pane get`, `pane list` ("they write nothing", `crates/holler-cli/src/pane/list.rs:16`) and `profile show` are reads, so a `Still` built from them is sound.

**Fix, binding.** No `doctor` runs between a `Still` "before" and its "after". The end state comes after the last window, which is fine.

### W-5 in detail

**Make `launch_args` pure, binding.**
- Make it an associated function, `Scratch::launch_args(session, project, pane, workspace, grid, port, extra) -> Vec<String>`, or a free function beside `binary_env`.
- `Scratch::launch` calls it with `self.herdr.session()` and `oc_root/proj`, and passes the result through `holler`'s `&[&str]`.
- A `&self` method cannot be pinned in the default run, so A14's half on `launch_args` would be opt-in only.

**Placement, recommended.** Put the pin in `pane_integration/guards.rs`. That file is #649's home for the scratch module's pure functions (E5, `guards`), and decision 3 already puts A13 there. The new target's `pins.rs` keeps the pins of its own code: A12 and the `Applied` reader.

## Reuse expectations for Phase 7

These are the analogous objects the brief names. Phase 7 PASSes an extension of them and BLOCKs a parallel path.

1. **`Scratch`** (`pane_integration/scratch/mod.rs`). Every operation #667 performs on a live scratch set is a `Scratch` method, moved or new. No free function in `profile_apply_scenario/` wraps `scratch.json` the way `record` does.
2. **The scratch guards.** The `--herdr-session` guard is a pure function in `scratch/mod.rs`, tested in `guards.rs`. The new target has no guard of its own.
3. **`Out` and `output_of`.** One definition, in `pane_integration/out.rs`, included by both targets. No new `Out` is added. Four exist today: `pane_integration/main.rs` (the one that moves), `pane_verbs/process/main.rs`, `hold_cli_test.rs` and `grant_cli_test.rs`.
4. **Used, never re-implemented:**
   - `support::{Hub, StateDir, wait_for, holler_bin}`;
   - the rig's `free_port`, `refused`, `no_model_can_be_reached`, `signal_group` and `raw`;
   - `check_envelope`, through `Scratch::json`.
5. **`launch_args`** is extracted from `Scratch::launch`, which then calls it. There is no second argv builder for spec-only launches.
6. **New and justified:**
   - `Still`: #649 has no counterpart beyond one `pane list` comparison.
   - the `Applied` reader: production types are `Serialize` only, and the fakes' helpers belong to another target's rig. Its enums are production's (W-7).
   - the A12 source pin: nothing in the repo pins `#[ignore]` by reading source.
7. **No fake host or fake port** is built. #846's three `LinkedHost` copies grow by none.

## In flight (checked 2026-10-11 19:45 MDT)

- I fetched, and `origin/main` is `bda1069`, this branch's base.
- No branch in flight edits any of these files:
  - `pane_integration/main.rs`, `scenario.rs`, `guards.rs` or `scratch/mod.rs`;
  - `profile_apply_scenario/`.
- Some in-flight branches touch nearby files:
  - #831 touches `pane_integration/bind.rs` and `Cargo.toml`;
  - #835 touches `bind.rs`, `observe.rs` and `wired.rs`;
  - #851 touches `Cargo.toml`.

  #667 edits none of these.
- #839 has no commit under `pane_integration/` yet.
- The remote `origin/issue-649-implementation` is #649's own merged branch.

## Notes for O

There is no BLOCK, so no amendment is needed. These go to the operator through the run's result.

1. **W-9.** Comment on #667 that the brief's decisions 1 to 3 supersede the issue's harness, dependency and blast-radius text, and link the brief. S then audits against the reviewed plan.
2. **W-8.** When #839's run starts, tell it that #667 turns `record` and `session_of_record` into `Scratch` methods. `reset_with_a_first_message` then calls `scratch.session_of_record(C2)` and `scratch.record(C2)`.
3. **The brief's Question 2** is answered by W-1: F runs and owns the CHANGELOG entry and the guide's bullet. No journal decision to skip F is needed.
4. **The brief's Questions 1 and 3** stay the operator's:
   - Question 1: the row-before-column hazard of decision 10. I confirmed its reading: `cell_key` sorts row-major (`tx_apply.rs:348-351`), and `below()` refuses a new row under a row of more than one pane (`plan.rs:180-191`).
   - Question 3: naming.

## Patterns referenced

- `crates/holler-cli/tests/pane_integration/scratch/mod.rs` and `scenario.rs`: the analogous object and the helpers to move.
- `crates/holler-cli/tests/pane_integration/main.rs` and `guards.rs`: the crate-root layout, the crate-level allows, and where pure guards are tested.
- `crates/holler-cli/src/pane/wiring.rs`: the one assembly of the ports, which is why the scenario drives the binary.
- `crates/holler-pane/src/tx_apply.rs`, `tx_apply/run.rs`, `tx_launch.rs` and `reconcile.rs`: what apply, launch and doctor read and write.
- `docs/adr/ADR-0021.md`, section 8 ("Apply as built") and lines 1028-1031; `docs/pane-control.md`, sections 6, 8 and 12.
