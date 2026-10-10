# Handoff-A-dup: Phase 7 - #645 part 2, `--first` on reset and the activity refusals  (anti-duplication gate)

**Date:** 2026-10-10
**Branch:** `issue-0645-switch-reset` (T-red at `560a6c0`; F's implementation uncommitted on top)
**Diff base:** `560a6c0` (F's implementation + T's fixture row) · full run `3d95aec` adds T's RED suite (already reviewed as the t-red contract)
**Reuse map:** `docs/handoffs/0645-switch-reset/survey.md` (Reuse & Analogous-Feature map) · plan review `handoff-A.md` (the a-dup checkpoint, item 6)
**Verdict:** PASS

## Summary

F extended exactly the objects the Reuse map names — the shared `execute` path, `SwitchRequest`/`plan()` in the
engine, reset's own clap args, and the two default-armed `HarnessPort` seam methods of the operator's carve-out —
and built no parallel path anywhere. The diff is the carve-out and nothing more (9 files, all in boundary); the
seam wrappers stayed `WriterInSelect`-shaped record-and-replay doubles, not a second `FakeHarness`; and the one
place a second prompt path could have grown (the hub's `send_prompt` choke point) is explicitly fenced off by the
new ADR-0021 seam paragraph, which points #646/#649 at the seam. Zero blocks; one warn (a stale half-clause in
ADR-0021 §9 :534 left for O at merge).

## The checklist, verified

1. **The carve-out's extent held.** `git diff 560a6c0 --stat` = exactly the allowed set:
   `ports.rs`, `tx_switch.rs`, `reset.rs`, `switch.rs` (CLI), `ADR-0003.md`, `ADR-0021.md`, `cli-surface.txt`
   (one row), plus the run's own handoff docs (`brief.md` boundary amendment, `decisions.md` journal; handoff-F /
   handoff-T-green untracked). No other file. `ports.rs` holds exactly the two default-armed methods
   (`ports.rs:213-229`), the const `PROMPT_UNSUPPORTED` (`ports.rs:234`), the closed `Activity` (`ports.rs:238`,
   minimal derives, no Serde, no root re-export), the `RefusalCode` import the const requires, and the one
   module-doc amendment sentence — the carve-out's whole extent as amended (N-1).
2. **The wrappers did not grow into a second FakeHarness.** `SeamHarness`
   (`tests/pane_verbs/switch.rs:713-789`) delegates the eight existing methods to `inner: &FakeHarness` verbatim
   and overrides only the two seam methods; `SeamLog` (:669) is two mutex vectors; `QueuedPrompt` (:657) is a
   plain record. Answers come from a table set per case (`activity: Activity`, `prompt: Option<PaneError>`) — no
   session or TUI state of their own, no checker/rule logic duplicated (the engine owns the
   Busy/HoldingQuestion→code mapping at `tx_switch.rs:320-324`; the wrapper only replays a configured answer).
   The one extra read — the pane-store snapshot at prompt time (`then_of_record`) — observes the rig's shared
   store; it models nothing. The a-dup constraint is written on the type's own doc comment.
3. **No parallel path.** The prompt and the activity query both flow through the one trait
   (`tx_switch.rs:218` `ports.harness.send_prompt`, `:319` `ports.harness.session_activity`); the CLI layer only
   threads `Option<String>` (`reset.rs:37,51` → `execute`/`request` → `SwitchRequest.first`; switch passes `None`
   at `switch.rs:57`). `Activity` is defined once (`ports.rs:238`); tests import
   `holler_pane::ports::Activity` — no local duplicate. Codes are consts in the owning files per ruling 3
   (`SESSION_BUSY`/`SESSION_HOLDS_QUESTION` at `tx_switch.rs:69-72`; `PROMPT_UNSUPPORTED` at `ports.rs:234`),
   all travelling as `PaneError::Refused`; `error.rs` is absent from the diff — the closed set untouched. The
   pre-existing hub choke point (`holler-hub/src/circuit/dispatch.rs:81`) and the new seam coexist as the
   carve-out's recorded departure: ADR-0021's seam paragraph (`ADR-0021.md:399-410`) states #646 and #649 "build
   on them rather than beside them," names both 645b departures (verb→`HarnessPort::send_prompt`, not the hub's
   prompt path; activity read on the session of record, not DRIVEN), and the deferred item (:686-691) leaves any
   later merge into the hub's path to #646/#649 — the fence my plan review demanded (W-1's condition) is built.
4. **No drift vs the plan.** N-1 enum+const in `ports.rs` ✓. N-2's third decoration `unprompted`
   (`tx_switch.rs:140`) is set only with `acted: false, created: None` (:219-224), and `message()` appends the
   brief's phrase (:168-172) with the not-recorded clause gated on `created` and the reconcile step on `acted` —
   neither fires ✓. N-3's `prompt-unsupported`-vs-`not-implemented` rationale is in the ADR (:403-405) ✓. The
   gate sits after `refuse_orchestrator` (:240) and `check_health` (:241), on the `None` arm of `existing` —
   exactly `Target::Fresh` (:235-237), switch's arm unchanged (:243-246); `check_idle` returns before any port
   call when `session_of_record` is `None` (:313-317) ✓. The prompt runs strictly after `cas_put` (:211 →
   :216-218), target already the session of record ✓. Scope stayed put: no park check (the gap is recorded in
   the ADR for #646/#649), no empty-`--first` validation (deferred to #642 part 3), no drive-by edit anywhere.
5. **Sequencing and test honesty.** I reproduced the counts independently: `#[test]` under `tests/pane_verbs/`
   is 249 at base `3d95aec` and 256 in the tree; the `process/` subtree holds 35 of them (the separate
   `pane_cli_process` binary — not a module of `pane_verbs`' `main.rs`), leaving **214 → 221 = 214 + 7**, T-green's
   numbers exactly. The fixture row is one line under `# #645` after `--as-operator`, parse-proven
   (`cli_surface_test` 3/0). The ordering pin (`then_of_record` == the new session) is asserted in the incident
   case. The test files are unmodified since T's RED commit `560a6c0` (F wrote nothing under `tests/**` — the
   attempt-1 refusal was handled by the boundary amendment, not routed around).

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:534` | §9's class-table row still says the open codes are "#640's `grid-unreachable`, merged; #645's and #646's, planned" — half-stale the moment this PR merges (#645's three codes are as-built). Outside W-1's three spots, so F correctly left it; already flagged by F and T-green and journaled in decisions.md as an accepted residual. | O applies the one-clause edit at merge ("#645's, merged; #646's, planned") or explicitly drops it — do not let it reach the PR unnoted. |

No duplication findings. Extension is clean.

## Notes for F

None — no rework. For the record, F's own notes ("Notes for A-dup / the operator") were each verified against the
tree: the park-state gap is in the ADR seam paragraph, `--first ""` is genuinely unpinned (the text travels as
typed), the success output is unchanged with `--first`, and `SwitchRequest.first`'s engine-level generality (any
recorded target) is the map's named extension — only reset sets it, and the docs scope it to reset.

## Notes for S / O

- Finding 1 is the only merge-time action; everything else T-green hedged (park-state, empty `--first`,
  flag-coverage unenforced) I confirm as deliberate, recorded deferrals — not defects.
- The working tree is uncommitted (implementation + fixture row + this handoff's siblings); per the standing
  rules the commit is O's, then the rebase on `origin/main` before the PR. Placeholder rule holds in every new
  handoff file (checked).
- The ADR seam paragraph is the contract #646/#649 must read; if either story proposes a second prompt or
  activity vantage, this run's record is the thing to cite against it.

## Patterns referenced

- `crates/holler-cli/tests/pane_verbs/switch.rs:713-789` — `SeamHarness`/`SeamLog`/`QueuedPrompt`, judged against
  the `WriterInSelect` precedent (:648 area, part 1) my plan review set as the a-dup baseline.
- `crates/holler-pane/src/tx_switch.rs:69-72,140,216-224,235-247,313-324` — the codes, the third decoration, the
  prompt-after-record step and the `check_idle` gate.
- `crates/holler-pane/src/ports.rs:208-245` — the carve-out's whole extent.
- `docs/adr/ADR-0021.md:399-410,473,686-691` — the seam paragraph, the as-built codes row, the decided deferred
  item (the #646/#649 fence).
- `docs/handoffs/0645-switch-reset/handoff-A.md` (plan review, item 6 — the a-dup checkpoint) and
  `handoff-T-green.md` (the counts and hunk-by-hunk spot-check I re-verified).
