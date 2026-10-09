# Handoff-A-dup: Phase 7 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-682-implementation
**Diff base:** e410e9d (origin/main)   **Diff head:** ad84b41
**Reuse map:** docs/handoffs/682-brief.md, sections "Evidence", "Reuse refactors" and "Extend vs new" (no separate survey.md)
**Verdict:** PASS

## Summary

PASS. F extended every object the brief named and built no parallel path in `src/`. `FakeProfileStore` uses the shared
`FaultSwitch` (every port method, `rename` included, enters it first), the generic `Feed`/`Log`/`Change`/`Watch` (one
`impl Change for ProfileEvent`, keyed by slug, and `Feed::watch` with `WatchNext`), and the `Writer` and `lock` moved into
`feed.rs`. `pane_store.rs` keeps no copy of either. `next_generation` is the only compare-and-swap rule in both fakes,
`ProfileName::slug` the only slug rule, and `EnvVarName` the only env guard. The profile suite calls `run_cases`,
`succeeds`, `expect_code`, `expect_eq`, `next_item` and `drain` from `conformance/mod.rs`, and `profile_name` and the four
watch helpers (now generic) from the pane suite. It defines no local copy of any of them. The four warns are about
consolidating later, not about drift: a follow-up that Phase 3 asked for and that has not been filed (W-1), test-side copies that
AC5 forced (W-2), a small part of W-5 still left in the fixtures (W-3), and a module name (W-4).

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn | `src/conformance/pane_store.rs:423-534`; `src/conformance/profile_store.rs:50,64,85-87`; `profile_store/watch.rs:15`; `profile_store/log.rs:10`; `src/lib.rs:14-16,27-28` | Phase 3's W-2 is now real. The five pane-suite helpers are `pub(super)`, and three sibling files import them. The two suites also each define the same literals: `CONFLICT` (`pane_store.rs:32`, `profile_store.rs:64`) and `C1`/`C2`/`C3` (`pane_store.rs:37-39`, `profile_store.rs:85-87`). The brief justifies this in writing (no slice edits `conformance/mod.rs`), so it is not drift. But **no follow-up issue exists** (I searched the repo's issues), and #688 (part 2) says it reuses the same helpers. Separately, `lib.rs`'s summaries of `feed` and `fixture`, and "slices b to e are empty stubs", are now stale, and AC8 forbids editing them here. | Open the follow-up now. After #681, #683, #684 and #688 merge, move `profile_name`, `expect_change`, `changes`, `cursors`, `increasing` and the shared code and pane-name constants into `conformance/mod.rs`. Refresh `lib.rs:14-16,27-28` in the same change. |
| W-2 | warn | `tests/fake_profile_store_test.rs:29-78` vs `tests/fake_pane_store_test.rs:20-65`; `tests/profile_store_conformance_test.rs:176-188` vs `tests/pane_store_conformance_test.rs:251-263` | The test helpers are copies of slice a's with only the types changed (`name`, `timeout`, `corrupt`, `unavailable`, `open_watch`, `watch_error`, `drain`). `assert_suite_fails_on` differs only in the runner it calls. The brief sanctions both: AC2 prescribes the mirror, AC5 forbids editing slice a's test files, and each Rust integration test is its own crate with no `tests/common/` yet. Part 2 (#688) and slices d and e (#683, #684) will each add another copy. | Fold this into W-1's follow-up: a `tests/common/mod.rs` holding a generic `drain<T: Debug>(&mut Watch<T>) -> Vec<T>`, the error constructors, and `assert_fails_on(result: Conformance, case, label: impl Debug)`, with all four existing test files switched to it. If #688 lands first, its brief should say whether it starts `tests/common/` or adds a third copy on purpose. |
| W-3 | warn | `src/fixture.rs:58,103` and `:64,107`; `tests/fake_profile_store_test.rs:576-584` | F did everything Phase 3's W-5 asked: `SCRATCH`, `SAMPLE_GRID`, `SAMPLE_CWD`, `sample_model()` and `sample_context()` are shared. Two values are still written inline in both fixtures: `HarnessKind::Opencode` and `PaneRole::Agent`. Yet the module doc (`fixture.rs:5-7`) says a sample pane and its sample spec agree on the harness. `sample_spec_agrees_with_sample_pane` checks only the model, the context and the cwd, so a change to one fixture's harness or role would pass unnoticed. That matters for #662's `profile_snapshot`, which maps a pane to a spec. | Add `SAMPLE_HARNESS` and `SAMPLE_ROLE` consts beside `SAMPLE_GRID`, and extend the agreement test with `harness.kind`, `role`, `herdr.workspace` and `herdr.grid`. It is cheap, and #688 is the natural carrier because it adds to `fixture.rs` anyway. |
| W-4 | warn | `src/conformance/profile_store/log.rs:1-4,63,83` | The module `log` also holds case 17 (`rename-is-not-implemented`) and case 18 (`env-is-names-only`), which are not log cases. The brief's Risks section prescribed this exact split ("cases 15 to 18" into `profile_store/log.rs`) to stay under 600 lines, and the module doc names all four cases, so this is accepted. | No action now. When #665 replaces case 17 with the real `rename` cases, put them in their own child module (`profile_store/rename.rs`) instead of growing `log.rs`. |

**Checked and accepted (no finding):**
- **AC7's greps** print nothing. There is no `'='` scan, `enum Writer` appears only in `feed.rs:220`, and `Condvar` only in
  `feed.rs`. `fn lock` exists only at `feed.rs:243` (now `pub(crate)`, used by `profile_store.rs:234,287`) and as
  `FaultSwitch`'s private method `fault.rs:105`, which predates this change and was accepted at Phase 3.
- **The two fakes' `put`/`remove` write paths and impl blocks** have the same shape. The brief accepts this in writing
  ("Accepted near-duplicate"). The rule order differs on purpose (the name rule before the generation for profiles, the
  generation before membership for panes), so a shared skeleton would need a hook at each step.
- **New, as the brief justified:** `change_logs`, the clock, `check_name`, `rename`'s refusal, `summary` and the `entry`
  constructor in the fake. In the suite: `actor`, `stored_as`, `listed`, `entries`, `history` and `step`, plus the typed
  `sample`, `revised`, `put`, `shown`, `unchanged`, `open` and `last_cursor` over `ProfileStore`. Nothing in `holler-pane`
  or `holler-hub` builds a `ProfileLogEntry`, an `Updated` summary or a same-slug check today, so there was nothing to reuse.
  `log.rs`'s `non_decreasing` (non-strict, over `at: i64`) has the same `windows(2)` idiom as `increasing` (strict, over
  `Cursor`), but it is a different predicate. Making `increasing` generic to cover both would be over-abstraction.
- **The generic `expect_change`** prints the key with `{:?}`, so a pane-suite failure now reads `PaneName("demo-c1r1")`
  where it used to read `demo-c1r1`. The brief specified the `Debug` bound, the change is text-only, and no test pins the
  detail text.
- **No architectural drift crept in during rework.** T's GREEN repair split one test and touched no production code.
- **Layout and dependencies:** `lib.rs`, `conformance/mod.rs`, both `profile_scope` stubs, slice a's two test files, every
  manifest and `Cargo.lock` are unchanged, and the diff touches no crate but the test kit. The largest `.rs` file in the
  diff is 597 lines, and no personal names appear anywhere in it.
- **ADR-0021:61-62** ("Two names with the same slug are the same profile for uniqueness") grounds the name rule. The other
  rules the suite fixes cover ground the ADR leaves open: the name rule comes before the generation, a `Deleted` entry
  carries g + 1, and the log is never cut. They are stated in the suite's module doc, as Phase 3's W-1 asked.

## Notes for F

None: PASS, with nothing to fold.

## Notes for O and the run's agent

- **Carried forward from Phase 3 (W-6), and still open:** the script opens the PR with `Closes #682.` Change it to
  `Part of #682.` with `gh pr edit`, in the same edit that adds the `CONTRIBUTING.md` AI disclosure. After the merge,
  check that #682 is still open. Part 2 is now filed as #688.
- **W-1 and W-2** are one follow-up issue (helpers into `conformance/mod.rs`, a `tests/common/mod.rs`, and the `lib.rs`
  summaries), due once #681, #683, #684 and #688 have merged. **W-3** fits into #688.

## Patterns referenced

- `crates/holler-pane-testkit/src/{feed.rs, fault.rs, pane_store.rs, fixture.rs}` and
  `src/conformance/{mod.rs, pane_store.rs}` (slice a, as refactored)
- `crates/holler-pane-testkit/tests/{pane_store_conformance_test.rs, fake_pane_store_test.rs}` (slice a's test pattern)
- `crates/holler-pane/src/{profile.rs, error.rs}` and `crates/holler-hub/src/profile/mod.rs` (no reusable name-rule,
  log-entry or summary helper)
- `docs/adr/ADR-0021.md:53-62` (the `Profile` record, the log, and slug uniqueness)
- `docs/handoffs/682/handoff-A.md` (the Phase 3 warns this gate checks)
