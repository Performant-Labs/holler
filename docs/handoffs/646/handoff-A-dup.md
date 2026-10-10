# Handoff-A-dup: Phase 7 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`)
**Diff base:** `ce12cdb`   **Diff head:** `dca7abb`
**Reuse map:** the brief's "Reuse and analogous-feature map" (`docs/handoffs/646-brief.md`, under "Files"; this run has no separate `survey.md`)
**Verdict:** PASS

## Summary

PASS: no block, three warns. F extended what the map named and built no parallel path in production code:

- `PanePark` and `PaneUnpark` grew their own fields in place, with `ProfileOpt` still flattened.
- The verb has doctor's shape: type the arguments, act on `ctx.ports`, then `output::emit` or `emit_error`.
- Membership goes only through `ProfileScope::resolve`.
- Each pane is written by one `cas_put` at the generation it was read at, as reconcile records.
- Quoting is `findings::quoted` and the clock is `now_millis`.
- `unpark.rs` (38 lines) has no loop, scope logic or renderer of its own. It types its target and calls park's engine.

The three warns are duplication or drift that the brief accepted, or that A raised at plan review, and whose follow-ups are
still unfiled:

1. The test rig copies #662's rig helpers.
2. `in_scope` copies reconcile's private scope arms.
3. The ADR paragraph still carries plan-review warns 2 and 3.

None of them is a parallel path that the brief failed to justify.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/park/rig.rs:109-112, 152-158, 186-284` | **The rig copies #662's rig helpers, verbatim in places.**<br>- `assert_failure` (243-259) is a byte-for-byte copy of `profile_verbs/rig.rs:114-130`; only one doc line differs.<br>- `pane` (273-276), `member` (278-284) and `Rig::run` (109-112) are verbatim copies of `profile_verbs/rig.rs:132-143, 69-72`.<br>- `Both`/`run_both` (186-241) are #662's, plus an `err` check and the two rigs.<br>- The four adapter-log asserts (155-158) are #662's `assert_no_adapter_call` (76-79).<br>- `Rig::record` (114-119) repeats doctor's (`doctor/rig.rs:182-189`).<br>- **Why warn, not block:** brief Decision 11 and its Risks section accept a per-story rig. Plan-review warn 5 asked for #662's names, and the rig uses them. The two rigs sit in different test targets, so sharing them means editing #662's rig, which is outside this blast radius.<br>- **What is still open:** the consolidation has no issue and no owner. Two things will make it less than mechanical: `profile()` has the same name and a different signature in the two rigs (`&[&str]` here, `Vec<ProfileSpec>` there), and `Rig.panes` is an `Arc<FakePaneStore>` here but held by value there. | File the consolidation as an issue with an owner.<br>- Target `tests/verb_harness/`: both test targets include it by `#[path]`, and `crates/holler-cli/Cargo.toml:488-494` expects verb stories to change it.<br>- Move `run_both`, `Both`, `assert_failure`, `assert_failure_message`, `pane`, `member` and the adapter-log check there. Each rig keeps only its own stores and scope.<br>- Rename one of the two `profile()` helpers first. |
| 2 | warn | `crates/holler-cli/src/pane/park.rs:300-319` (`in_scope`), `:131-167` (`HoldTarget::parse`) | **The scope arms are a copy, and the shared target is named for holds.**<br>- `in_scope` repeats the two arms of `holler_pane::reconcile::resolve` (`crates/holler-pane/src/reconcile.rs:219-243`) exactly, down to `PaneError::PaneNotFound { what: name.to_string() }`. The module doc (lines 15-20) cites the source, as plan-review warn 4 asked.<br>- **Why warn, not block:** the map sends membership through `ProfileScope::resolve` (done). The source is private, in a frozen crate.<br>- **The cost to come:** `get` and `watch` (#643) and `switch` and `reset` (#645) need the same "PANE or `--profile`" typing and the same two arms. Without a shared helper, each will copy them again or import `HoldTarget`/`in_scope` from `pane::park`. That would be a verb-to-verb dependency beyond the park/unpark pair that Decision 9 approved.<br>- The "panes in scope" follow-up is still unfiled. | File the shared-helper follow-up with an owner.<br>- **Where it can live:** `crates/holler-cli/src/pane/profile_scope.rs` (`pane/mod.rs:10` calls it "the helper every `--profile` verb uses"; #663 owns it and has not merged), or beside `ProfileScope` in `holler-pane` (amend-first).<br>- **What moves there:** `in_scope` and the target typing, renamed so the name no longer says "Hold". Reconcile's arms should call it or match it.<br>- **Until then:** no other verb imports `HoldTarget` or `in_scope` from `park.rs`. |
| 3 | warn | `docs/adr/ADR-0021.md:138-139` and `:471-472`, against `crates/holler-cli/src/pane/park.rs:8-14, 35-38` | **The ADR and the verb's own doc now disagree. These are plan-review warns 2 and 3, still open.**<br>- AC 12 pins the paragraph, so F was right to leave it verbatim. F put the correct facts in the module doc instead:<br>&nbsp;&nbsp;- the record does not enforce the text rule (the hub stores the hold it is given), so a reader still escapes the texts;<br>&nbsp;&nbsp;- no reconcile step follows a failed write, because there is no live act, and a timed-out write may have landed.<br>- The ADR paragraph still states the rule as a property of the values.<br>- Section 12 (line 471-472) still says every verb that times out "prints the reconcile step".<br>- **Risk:** a later reader of the ADR, such as #643 `get`/`list` or the #648 roster, could print the texts unescaped.<br>- F did not introduce this drift in rework; it is the plan warn carried forward. | Carry the two sentences from `handoff-A.md` findings 2 and 3, either in 646b's brief (it edits ADR-0021 too) or in a one-paragraph docs follow-up. File it before this PR merges so it has an owner. |

There is no other duplication. F's rework introduced no drift: the diff touches only the brief's files plus
`tests/pane_verbs/park/failures.rs`, the split that plan-review warn 6 named.

### Also checked (no finding)

- **Frozen and out-of-radius files are untouched** (`git diff ce12cdb..dca7abb`): `pane/mod.rs`, `pane/args.rs`,
  `output.rs`, `cli.rs`, `close.rs`, `prompt_target.rs`, `holler-pane/**`, `holler-pane-testkit/**`, `holler-hub/**`, every
  manifest, golden files and `docs/protocol/`.
- **Sharing between verb files.** `unpark` reaches the engine through `super::park` at `pub(super)`. This is the
  `profile/show.rs` to `profile/list.rs::count` precedent, with narrower visibility (`count` is `pub(crate)`). The crate's
  public API is still `PanePark`, `PaneUnpark` and the two `run`s.
- **Output.** Every answer goes through `output::emit` or `emit_error`, and exit codes come from `class_of`. The
  partial-failure `ErrorBody { code: ErrorCode::from(&error), message }` is built the same way as `output::not_implemented`
  (`output.rs:257-264`), not as a hand-written envelope. The "neither PANE nor `--profile`" refusal is a `PaneError::Usage`,
  as `args.rs:144-150` builds one.
- **The record write.** One `cas_put(&changed, record.generation)` per pane, never retried. This has reconcile's
  `record` shape (`reconcile/observe.rs:349-386`). `generation::next_generation` is not re-implemented.
- **The text guard.** `checked_text` (`park.rs:102-122`) has no public equivalent to call. It follows the pane domain's
  guard shape in `ProfileName::parse` and `Actor::parse` (`holler-pane/src/profile.rs:42-59, 117-134`): trim, a chain of
  refusals, then `PaneError::Usage`. The hub's `sanitize_reason` has the opposite policy and is kept apart, as ADR-0021
  asks. `MAX_TEXT_CHARS` cites it in its doc and does not import it (plan-review warn 1, taken).
- **The new types.** `HoldReport` and `PaneHold` are private. No existing per-pane report fits: `reconcile::PaneSummary`
  is doctor's observation summary. `Hold` has no methods in `holler-pane`, so `HoldChange::next_hold` duplicates none.
  `HoldReport` is also the name of the prompt-hold load report in `holler-load-test`, but that is another crate and
  neither imports the other.
- **The test-local wrappers are not new fakes.**
  - `FailNthCasPut` (`park/failures.rs:22-64`) is the brief's own plan for AC 8c. The test kit's `FaultSwitch` can fail
    only the next call, not the nth (`holler-pane-testkit/src/fault.rs:72-77`).
  - `ReversedScope` (`tests/pane_verbs/park.rs:250-274`, T's repair at GREEN) is the same delegating-wrapper pattern as
    doctor's `HealthGate` (`doctor/surface.rs:458-471`). The fake scope has no switch for answering out of order.
- **No test case is written twice.** Park and unpark run over one rig, take the verb as a `Verb` parameter and share one
  AC 8 check (`park/failures.rs:126`).
- **The overlay's Phase 7 candidates** (the token store, `Lockout`, `Roster`, `log(Severity, ...)`, `Hub`, `Body`,
  `mint_token`, `join`, `wait_for`, `StateDir`) are neither touched nor copied.
- **Upstream.** `origin/main` has three commits past `ce12cdb` (#702, #705, #706), all in adapter crates. #643's
  `list::text_value` and #663's reconcile step have not merged, so park had nothing newer to call.
- **Size.** `park.rs` is 401 lines and `unpark.rs` 38. The test files are 489 (`park.rs`), 407 (`park/rig.rs`), 269
  (`park/failures.rs`) and 41 (`unpark.rs`). All are under lint's 600-line warning.
- **Public repository.** The code, ADR and CHANGELOG diff holds no personal infrastructure name.

## Notes for F

None required: the verdict is PASS and nothing needs folding.

For the orchestrator or S: three follow-ups named at plan review are still unfiled, and findings 1-3 depend on them:

1. The rig consolidation into `tests/verb_harness/`, with an owner.
2. The shared "panes in scope" helper, coordinated with #663's `profile_scope.rs`.
3. The ADR-0021 wording: the text rule is the verb's guard, not the record's, and there is no reconcile step for a
   record-only verb.

## Patterns referenced

- `crates/holler-cli/src/pane/doctor.rs`, `crates/holler-cli/src/profile/{list,show}.rs`, `crates/holler-cli/src/output.rs`:
  the verb shape, sharing between verb files under a frozen `mod.rs`, and building an `ErrorBody`.
- `crates/holler-pane/src/reconcile.rs:219-243` and `reconcile/observe.rs:349-386`: the scope arms and the record-write rule.
- `crates/holler-pane/src/profile.rs:42-59, 117-134`: the pane domain's text guards.
- `crates/holler-cli/tests/profile_verbs/rig.rs`, `tests/pane_verbs/doctor/rig.rs`, `tests/verb_harness/mod.rs`: the rigs and
  the shared harness on main.
- `crates/holler-pane-testkit/src/{fault,profile_scope}.rs`: what the fakes can and cannot do.
