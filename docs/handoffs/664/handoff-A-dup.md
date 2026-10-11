# Handoff-A-dup: Phase 7 - `holler profile apply` (#664)  (anti-duplication gate)

**Date:** 2026-10-11 (11:36 MDT)
**Branch:** `issue-664-implementation`
**Diff base:** `7284001` (the merge base with `origin/main`, #802)   **Diff head:** `8ed5f8c` (T-green, fresh run after A round 4)
**Reuse map:** none (no `survey.md`). The "Reuse expectations for Phase 7" in `docs/handoffs/664/handoff-A.md` (round 4) stand in for one.
**Verdict:** PASS

## Summary

PASS, with no block.

F extended the objects the plan named and built no parallel path:

- `tx_launch` is called unchanged for create and relaunch.
- The plan is `diff_profile`'s own rows.
- Every outcome goes through `class_of`.
- The verb reuses `reconcile_step`, `list::count` and `show`'s two escape helpers.
- The leave and join writes are the operator's justified twins of `detach` and `join`. They match line for line and #818 tracks them.

All nine round-4 expectations are met. This cycle's rework adds `reconciled` to the verb and edits doc comments, with no new object and no drift.

Two warns remain. Both are small, untracked copies, and both are left for follow-up work:

1. Three private helpers are copied from `reconcile`.
2. The test rig copies `launch/rig.rs`.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/tx_apply/observe.rs:94` (also `:26-28` and `:167`) | **Small private helpers are copied, and nothing tracks them.**<ul><li>`joined` (`observe.rs:94-96`) is a verbatim copy of `reconcile.rs:281-283`.</li><li>The three `<port>.<method>` call-name constants (`observe.rs:26-28`) repeat `reconcile.rs:60` and `reconcile/observe.rs:18,20`.</li><li>`listed` (`observe.rs:167-172`) is the third spelling of the `(session, pane_id)` join key. The first two are `tx_launch::same_pane` (`tx_launch.rs:779`) and reconcile's pair sets (`reconcile.rs:328-336` and `:350-358`).</li></ul>Each copy has a reason. C21 gives apply its own fan-out, and C9 has `tx_apply` compare the pair itself. Sharing any of them would mean editing `reconcile.rs` or `tx_launch.rs`, which are outside the Blast radius. But unlike the twins (#818), no follow-up names them. | **No change in this story.** O adds them to a follow-up, either a rider on #818 or a small refactor issue. That follow-up should:<ul><li>give `joined` and the call names one `pub(crate)` home, shared by `reconcile` and `tx_apply`;</li><li>give the join key one home, for example a method in `pane.rs`, called by `same_pane`, reconcile's sets and `listed`.</li></ul> |
| 2 | warn | `crates/holler-cli/tests/profile_verbs/apply/rig.rs:146-414` and `:634-759` | **The apply rig re-creates its model.**<ul><li>176 of its 467 non-trivial code lines appear verbatim in `pane_verbs/launch/rig.rs`. They include `Calls`, `Run` with `Deref`, `mark` and `calls_since`, `LinkedHost`, the nine forwarding methods of `HookedHarness`, `seed_live`, `place_herdr_pane`, and `name`, `profile_name` and `argv`.</li><li>Many more lines differ only in `self.x` against `self.fakes.x`.</li></ul>**Why it is not a block.**<ul><li>It is the house pattern. `park/rig.rs:4-9` and `close/rig.rs:3-10` copy by name "so a later consolidation is mechanical".</li><li>A round 1 warn 6 directed this rig.</li><li>The two rigs are in different test binaries, which share only `verb_harness`, by `#[path]`.</li></ul>**Why it is a warn.**<ul><li>The copy already differs from its model. `seed_live` records `pid: Some(pid)` (`apply/rig.rs:308`). Launch's has recorded `pid: None` since #802 (`launch/rig.rs:398`). The apply tests use that pid to tell a new server apart, so this is not a defect, but it makes the consolidation less mechanical.</li><li>No issue tracks the consolidation, and the pane and profile verbs now have six rig files.</li><li>At 759 lines, `apply/rig.rs` is close to the 800-line mark. New helpers belong in the case files.</li></ul> | **No change in this story.** O files one test-infrastructure issue: move the shared live-rig pieces into one module that `pane_verbs` and `profile_verbs` both include by `#[path]`, as they already share `verb_harness`. |

### Round 4's reuse expectations, checked against the diff

| # | Expectation | Status | Evidence |
|---|---|---|---|
| 1 | "Listed" compares only the `(session, pane_id)` pair | Met | `observe.rs:167-172` compares exactly `session` and `pane_id`. No `HerdrPane` `==`, `contains` or `same_place` appears in `tx_apply/`. |
| 2 | One thread per call; the doc no longer claims `observe_all`'s shape | Met | <ul><li>`observe.rs:54-83` starts one scoped `thread::Builder` thread per call: the snapshot, each `health` and each `shown_session`.</li><li>Each thread is joined. A lost thread is `unavailable` naming its call (`:98-104`).</li><li>The doc (`:4-16`) names the mechanism and the grain.</li><li>There are no channels and no `thread::spawn`. No `Cargo.toml` or `Cargo.lock` changes.</li></ul> |
| 3 | Each row's `PaneDiff` is `diff_profile`'s row | Met | <ul><li>`plan` (`tx_apply.rs:332-345`) zips P's specs with `diff_profile`.</li><li>`spec_records` (`:246-255`) keeps only the records that a spec names. So `diff_profile` emits no `Extra` row (`profile_diff.rs:292-309`), and the zip pairs one to one.</li><li>`Step.diff` is that row, cloned unchanged (`run.rs:95-98`, `tx_apply.rs:499-503`).</li><li>No `PaneDiff` is built in `tx_apply`.</li></ul> |
| 4 | The twins are exact private copies with no undo, and `run.rs` cites #818 | Met | <ul><li>`leave` (`run.rs:219-225`) matches `detach` (`profile/delete.rs:149-155`) line for line.</li><li>`join` (`run.rs:230-240`) matches the write at `profile/create.rs:278-282` for one pane, without `undo` and without any profile delete.</li><li>Both are private `fn`s.</li><li>`run.rs:5-12` cites #818. `gh issue view 818` shows it is open, and its body carries C10's clause that the twins stay semantically identical.</li></ul> |
| 5 | Every step is classified through `class_of` | Met | <ul><li>`Planned::halted` (`tx_apply.rs:513-534`) is the one place where a step's error becomes an outcome. It is reached from live steps (`run.rs:46`) and from planning refusals and failed observations (`dry`).</li><li>`observe::as_failure` (`observe.rs:178-190`) uses `class_of`.</li><li>There is no local code list.</li></ul> |
| 6 | The reconcile step goes on every acted step, `refused` included, with one guard | Met | <ul><li>`reconciled` (`profile/apply.rs:94-108`) uses `reconcile_step(Some(P))`, the `"; "` join, and the same "already contains" guard as `failure_body` in `pane/launch.rs:435-442` and `pane/close.rs:205-220`.</li><li>`stopped` (`:114-134`) appends nothing.</li><li>The doc of `Step.acted` (`tx_apply.rs:122-126`) covers `refused`.</li><li>Each verb carrying its own guard is the house pattern, and the text comes from the one builder.</li></ul> |
| 7 | The rigs are additive, and the #802 changes stay in the story's rig files | Met | <ul><li>`profile_verbs/rig.rs` gains only `serve_server` (`:240-255`). Its `Rig`, its `run_both*` helpers and `assert_no_adapter_call` are unchanged.</li><li>Under `pane_verbs/`, only the one `STUBS` line in `process/stub.rs` changes.</li></ul> |
| 8 | The open codes are `RefusalCode` constants listed in the ADR | Met | <ul><li>`tx_apply.rs:71-78` uses `RefusalCode::from_static`, `tx_switch.rs:59-72`'s pattern. Both codes are in ADR-0021's apply row (`:657`).</li><li>`orchestrator-not-relaunched` does not duplicate `tx_switch`'s `orchestrator-pane`. That code is a gate that `--as-operator` lifts, while apply's refusal has no override, and §3 row e names it.</li></ul> |
| 9 | Only the 27 paths, and lint exits 0 | Met | <ul><li>`git diff --name-only 7284001..8ed5f8c` lists exactly the 27 paths.</li><li>`bash scripts/lint.sh` exits 0. The story's only warning is `apply/rig.rs` at 759 lines.</li><li>This handoff adds a 28th path, under `docs/handoffs/664/`, which expectation 9 allows.</li></ul> |

### Drift re-check (verified, no action)

- **This cycle's rework (`3e09a1f..8ed5f8c`).**
  - Production code changes only in `profile/apply.rs`: the new `reconciled` pass, and `stopped`, which no longer appends.
  - `tx_apply.rs`, `observe.rs` and `run.rs` change only in doc comments.
  - No interface, module or dependency changes.
  - The engine only marks `acted`, and the verb owns the text, because `reconcile_step` belongs to `holler-cli`. `pane launch` splits the work the same way.
- **Shared code is untouched.** The diff is empty for `tx_launch.rs`, `tx_launch/`, `reconcile.rs`, `reconcile/`, `holler-pane/src/lib.rs`, `profile/mod.rs`, every `Cargo.toml` and `Cargo.lock`. The engine and the verb have no direct I/O, no `unsafe` and no `#[allow]`.
- **Other near-copies checked, with no finding.**
  - `stored_profile` (`tx_apply.rs:238-243`) is the only get-or-`profile-not-found` helper in `holler-pane`. Its siblings, such as `StoreScope::stored` and the verbs, are in `holler-cli`, which `holler-pane` cannot call.
  - `Halt` wraps `TxFailure` to add a note, because `tx_launch` may not be edited.
  - `cell_key` is new because `GridPos` has no `Ord`.
  - The messages for `pane-in-other-profile` and `pane-not-in-profile` are built where they are raised, as everywhere else.
  - The verb reuses `list::count`, `show`'s `difference_line` and `text` (now `pub(crate)`), and `emit` with `ErrorBody`.
- **The stack's Phase 7 candidates.** The story copies none of these and does not reach the hub:
  - the operations in `token.rs`;
  - `Lockout` and `Roster`;
  - `log(Severity, ...)`;
  - the hub test helpers `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`.

  The twins' `join` is the profile join, not the hub harness's.
- **Public repository.** The diff names no personal infrastructure.

## Notes for F

None. The verdict is PASS.

## Notes for O

- **Warns 1 and 2 are follow-up work.** Neither needs a change in this story. Decide whether to file them, or fold warn 1 into #818.
- **`origin/main` is now `dc1398c` (#820).**
  - `git merge-tree` against it is clean.
  - The only overlap is `CHANGELOG.md`.
  - The PR step merges `main` and then re-runs `changelog-check.sh`, as T-green advised.
- **Round 4's warns 1 and 3 are still unaddressed.** Warn 1 is E40's empty excerpt; warn 3 is the 27 paths missing from the Blast radius. Both are brief edits, not duplication. Expectation 9 still pins the path set.

## Patterns referenced

- **The fan-out and the join key:** `crates/holler-pane/src/reconcile.rs:251-283` (`observe_all`, `joined`) and `:326-369`, and `crates/holler-pane/src/tx_launch.rs:778-787` (`same_pane`, `same_place`).
- **The twins' originals:** `crates/holler-cli/src/profile/delete.rs:145-155` (`detach`) and `crates/holler-cli/src/profile/create.rs:266-290` (`join`).
- **The reconcile step:** `crates/holler-cli/src/pane/launch.rs:434-442` and `crates/holler-cli/src/pane/close.rs:202-220` (each `failure_body`), and `crates/holler-cli/src/pane/profile_scope.rs:38-56` (`reconcile_step`).
- **The comparison:** `crates/holler-pane/src/profile_diff.rs:287-309` (`diff_profile`).
- **The rigs:** `crates/holler-cli/tests/pane_verbs/launch/rig.rs`, `park/rig.rs:1-9`, `close/rig.rs:1-10` and `crates/holler-cli/tests/profile_verbs/rig.rs`.
