# Handoff-A-dup: Phase 7 - #645a `pane switch` and `pane reset`  (anti-duplication gate, round 2)

**Date:** 2026-10-10
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`)
**Diff base:** `d9eabbb` (`origin/main` and the merge base; fetched 2026-10-09 23:53 MDT, unchanged since round 1)   **Diff head:** `940e338` (T's round-3 GREEN; F's commits this cycle are `37f2101` and `62aae72`)
**Reuse map:** `docs/handoffs/645-brief.md`, "Reuse map (extend, do not duplicate)", lines 1052-1065 (there is no separate `survey.md`)
**Verdict:** PASS
**Supersedes:** round 1 (BLOCK, commit `da53aba`; its full text is in git history)

## Summary

PASS. Round 1's one block is fixed. F merged `origin/main` (`d9eabbb`). The #645 paragraph of ADR-0021 section 8 now
says that the reconcile step names the pane with `--fix`, even after a run with `--profile`, and why that differs from
the two forms of #663's step 6 (merged `ADR-0021.md:360-366`). #663's generations bullet and step 6 are untouched, so the
merged section 8 no longer gives two rules for one step. This cycle adds no parallel path. F rewrote one reused call (the
remedy fallback, now a `match`) and four doc comments. T added `failed_before_the_act`, the story's own `failed` with one
more assertion, and one test built on the existing helpers. The merge brought in two neighbours the Reuse map could not
name, #643's `list::profile_name` and `get.rs`'s scoped read. Both are recorded as warns below. Neither is a path the map
told F to extend.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:527-528` (section 12), against `:360-361` (the #645 paragraph) | Section 12 says that a verb that times out "prints the reconcile step". The #645 paragraph says that a failure before `select_session` has no step, and AC 5 pins that for a timed-out health check (`tests/pane_verbs/switch.rs:363-373`, `failed_before_the_act(.., 1, "timeout")`). This is not a contradiction. The specific rule comes with its reason, and #646's park, on `main`, reads sections 8 and 12 the same way: they "ask for one after a live act" (`pane/park.rs:11-12`). Section 12's sentence is still literally broader than that reading. AC 24 does not allow an edit to section 12. | The next story that edits section 12 (#644 owns its operation id) narrows the sentence to "...and, once a live act has begun, prints the reconcile step". No change in 645a. |
| 2 | warn | `crates/holler-cli/src/pane/switch.rs:97-101` | Arrived with the merge: #643's `list::profile_name` (`pane/list.rs:140-144`), "the `--profile` a verb is scoped to, typed". `get.rs:81` and `watch.rs:75` call it. `request` spells the same expression inline, as `pane/doctor.rs:49-54` (#647) does, and `park.rs:153` types it its own way. No pattern dominates outside the read verbs. The behaviour is the same (a bad name is `usage`, exit 2), and it is one expression. | At the next edit of `pane/switch.rs` (645b adds `--first` there), call `super::list::profile_name(profile)?`. Doctor's copy can follow with #647 part 2. No change in 645a. |
| 3 | warn | `crates/holler-pane/src/tx_switch.rs:145` and `:304-312` | Carried from round 1, and still unfiled. The lead-in `to reconcile, run ` is spelled here and in `reconcile_step` (`holler-cli/src/pane/profile_scope.rs:49-56`). Once this lands, the doc of `reconcile_step` (`:39-40`, "verbs call it rather than spell a step of their own") is no longer accurate. `screen_text` copies `reconcile/observe.rs:342-348` word for word. The brief justifies both copies (Reuse map line 1057, F-4, O1). The ADR paragraph now states the pane-scoped form, so the standing spec is right and only the code doc lags. No GitHub issue exists for either fold (searched 2026-10-09 23:55 MDT). | O files the F-4 fold: one `findings` function beside `doctor_command` for the lead-in and the screen wording, called by `reconcile_step`, `SwitchFailure::message` and reconcile, with `reconcile_step`'s doc corrected in the same change. The `screen_text` comment then names that issue. |
| 4 | warn | `crates/holler-pane/src/tx_switch.rs:197-213` | Carried from round 1, and widened by the merge. The scoped-read arms (with `--profile`, through `scope.resolve`; else `pane_store.get`, where none is `pane-not-found`) now have four private copies: `reconcile.rs:222-243`, `pane/park.rs:304-319`, `pane/get.rs:96-130` (#643) and this one. The get and switch copies both take the scope's answer by `find` on the name, but they answer a scope that breaks its contract differently: get with `pane-not-in-profile`, switch with `pane-not-found`. That arm cannot be reached with the real scope or the fake (`profile_scope.rs:92-100, 159-174`; F's round-3 evidence). | Follow-up: one `holler-pane` function for "the records that `PANE` and `--profile` name", used by all four, with one answer for that arm. |
| 5 | warn | `crates/holler-cli/tests/pane_verbs/switch.rs:31-230` | Carried from round 1. This is the fourth both-format runner, beside `doctor/surface.rs:131`, `profile_verbs/rig.rs:112` and `park/rig.rs:220`. This cycle adds `failed_before_the_act` (`:218-230`). It extends the story's own `failed` and copies nothing, since no other rig asserts that a reconcile step is absent. F-2's trigger has fired, and F-2 is not filed. | File F-2 with this runner and its new helper in scope, and with `WriterInSelect` (`:433-468`), a delegating `HarnessPort` in the same pattern as doctor's `HealthGate` (`doctor/surface.rs:456-514`); the test kit offers no call hook. No change in 645a. |

## What this cycle changed, and why none of it is a parallel path

| Change | Where | Result |
|---|---|---|
| Round 1's block: the merged section 8 gave two rules for one reconcile step | `origin/main` merged cleanly (`37f2101`). The #645 paragraph (`ADR-0021.md:360-366`) says when the step is printed and that it names the pane even with `--profile`, unlike step 6's two forms. The reason given is that a switched or reset pane has a record, and `--fix` is the remedy table's own form for a fixable mismatch (`findings.rs:119-120, 135-141`). #663's bullet (`:294-299`) and step 6 (`:328-339`) are untouched. `git diff origin/main -- docs/adr/ADR-0021.md` has the four hunks AC 24 allows, and the paragraph is the only `Switch and reset as built (#645)` line, between `### 8.` and `### 9.` | fixed |
| B-1 (the outside diff gate's round 2) | `check_health` (`tx_switch.rs:239-242`): a `match` on `FindingKind::ServerDown.remedy(..)`, with `doctor_command` as the `None` arm. The same reused calls, written another way. | reused, unchanged |
| Doc comments | `tx_switch.rs:30-38, 58-61, 131-135, 229-232`: "once `select_session` has been called", and why the two id limits agree | no code |
| T: no step before the act | `failed_before_the_act` (`tests/pane_verbs/switch.rs:218-230`) wraps `failed`. It is used at 9 sites in `switch.rs` and in AC 17 in `reset.rs`. `reset_create_failure_changes_nothing` (`reset.rs:226-245`) uses `faulted`, `both`, `reset_p` and `assert_unchanged`. | extends the story's own runner |
| T: the module doc of `reset.rs` | `reset.rs:1-3` | doc only |

Round 1's Reuse-map table still holds (git `da53aba`). Every row is extended or reused, and `screen_text` is the one
justified copy. Neither `pane/switch.rs` nor `pane/reset.rs` changed this cycle.

Stack checks on the merged tree:

- **One path.** The engine and reconcile's repair (`reconcile/observe.rs:318-321`) are still the only production callers
  of `select_session`. No prompt is sent, so nothing bypasses `send_prompt`.
- **Contract.** No frozen file differs from `origin/main`, and neither do the closed code table, a golden or wire file, or
  any `Cargo.toml`. The three open codes are still the only ones declared outside the Herdr adapter and its fake.
- **Size and naming.** Every touched file is under 800 lines: `tx_switch.rs` 324, `pane/switch.rs` 160, the test
  `switch.rs` 613 and `reset.rs` 288, and `ADR-0021.md` 633. No personal infrastructure name appears in the diff outside
  `docs/handoffs/`.

## Notes for O (not findings)

- **Open PR #714 (#640 part 3, head `077ea6d`) conflicts textually with this branch in `docs/adr/ADR-0021.md`, in two
  places, both edits on adjacent lines:**
  - Section 9: #714 edits the `pane launch` row, and this branch edits the `pane switch` row below it.
  - "Deferred": #714 rewrites the `HerdrPort`/`HarnessPort` bullet, and this branch rewrites the mismatch-code bullet
    above it and inserts the 645b bullet between them.

  `git merge-tree --write-tree HEAD origin/issue-640-implementation` exits 1, and that file is the only one in conflict.
  The edits are independent, so the resolution keeps both sides. Whichever PR merges second resolves it, as R-1
  anticipated for #644. Open PR #713 (#660) merges cleanly with this branch.
- **Follow-ups still to file:** F-1, F-3, F-4 (finding 3), the scoped-read fold (finding 4) and F-2 (finding 5). Today
  they are recorded only in `docs/handoffs/`.

## Patterns referenced

- `docs/adr/ADR-0021.md:285-370` (section 8 as merged) and `:524-531` (section 12); `crates/holler-cli/src/pane/park.rs:1-30`
  (#646's reading of sections 8 and 12).
- `crates/holler-cli/src/pane/profile_scope.rs:37-56, 92-100, 159-174` (`reconcile_step`, `member`, `resolve`) and
  `crates/holler-pane/src/findings.rs:111-143, 303-316` (the remedy table, `doctor_command`).
- `crates/holler-cli/src/pane/list.rs:140-144`, `get.rs:78-130`, `doctor.rs:46-54` and `park.rs:145-160, 304-319`, with
  `crates/holler-pane/src/reconcile.rs:222-243`: how `--profile` is typed, and the scoped-read arms.
- `crates/holler-pane/src/reconcile/observe.rs:316-348`: select, observe, `screen_text`.
- `crates/holler-cli/tests/pane_verbs/park/rig.rs:186-272`, `tests/profile_verbs/rig.rs:112-190` and
  `tests/pane_verbs/doctor/surface.rs:131, 456-514`: the runners, and `HealthGate`.
