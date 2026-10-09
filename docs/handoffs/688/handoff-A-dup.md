# Handoff-A-dup: Phase 7 - #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-688-implementation
**Diff base:** 9d61c9f (origin/main, also the merge base)   **Diff head:** 05d7262
**Reuse map:** docs/handoffs/688-brief.md, sections "Evidence", "Reuse refactors" and "Reuse map" (this run has no separate survey.md)
**Verdict:** PASS

## Summary

PASS. F extended the two stubs that #638 declared and composed the existing fakes through their ports. It reused every
object the Reuse map named and built no parallel path:

- `check_membership` is still the only producer of `pane-in-other-profile` in `src/`, and the scope calls it only for a
  `Set`.
- The scope reads its mutex through `feed::lock`.
- The suite imports part 1's helpers, constants and `Step`, and slice a's name helpers, and keeps no local copy of any of
  them.
- The three mutants delegate to the fake.

Every new object (`with_edit`, `belongs`, the restore step, and the suite's small helpers) is one the brief justifies as
new. I found no equivalent anywhere in the workspace. There are two warns, neither of them drift. The first is a stale
`lib.rs` doc line that no issue owns yet. The second is a one-line near-copy of part 1's `revised` that #694 could fold.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn | `crates/holler-pane-testkit/src/lib.rs:28-29` (not in the diff) | "The modules of slices b to e are empty stubs" is false once this diff merges, because it fills the last two stubs. This run may not edit `lib.rs` (AC8). Phase 3's Notes for O item 3 asked for #694's scope to cover the line, but #694's body (read 2026-10-09) does not mention it. So no issue owns it yet. The `(slice c, #682)` labels on `profile_scope` in `lib.rs:19-20, 23-24` and `conformance/mod.rs:12-13` also name only part 1. | Add one line to #694's scope: refresh `lib.rs:28-29`, and optionally add #688 to the two slice c labels. |
| W-2 | warn | `crates/holler-pane-testkit/src/conformance/profile_scope.rs:401-407` | `changed_spec` makes the same change as part 1's private `revised` (`conformance/profile_store.rs:448-456`) for the same purpose ("so that a write of it shows"). Both raise a spec's `context.soft` with `saturating_add`. `revised` works on a profile's first spec and `changed_spec` on a bare spec, which a `Set` needs. The brief planned this helper (its `spec_of`), so it passes. Nothing couples the two, so they cannot drift into a wrong result. | Optional: when #694 moves the shared helpers into `conformance/mod.rs`, write `revised` in terms of a spec-level helper, or keep both. No change in this run. |

No duplication; the extension is clean. I found no drift from rework, since this is the first Phase 7 pass and F's four
deviations are each consistent with neighbouring code (see below).

### What was checked, and the result

- **AC7, re-run by me.** Both greps print nothing:
  - `grep -rn "PaneInOtherProfile" crates/holler-pane-testkit/src | grep -v pane_store.rs`;
  - `grep -rnE "^(pub\(super\) )?fn (actor|sample|history|shown|unchanged|pane_name|profile_name)\b" .../conformance/profile_scope*`.
- **`check_membership`.**
  - It is `pub(crate)` (`pane_store.rs:225`), and its doc names the scope as its second caller.
  - In the scope, `check_joins` is its only caller (`profile_scope.rs:265-271`), and `plan` calls `check_joins` only
    inside `if let (SpecEdit::Set(_), Some(record))` (`:140-142`).
  - The pane record is read unconditionally, for every edit (`:139`).
  - A workspace grep shows that the only other `PaneInOtherProfile` producers are in `holler-pane/src/error.rs` (the
    enum and its decode) and in the two test files (a mutant and an assertion), which AC7 excludes.
- **The other errors the scope builds.**
  - `pane-not-in-profile` (`member`, `:119-126`) and `profile-conflict` (`restore`, `:172-179`) are built only in the
    scope.
  - `holler-pane` has no constructor for them to reuse. The other fakes build their errors inline too
    (`profile_store.rs:216-218, 290-292`).
- **`with_edit` (`:277-293`) has no counterpart.**
  - A workspace grep for spec mutators and for `panes.retain` and `iter_mut().find` finds only `MemScope` in
    `holler-pane/tests/ports_test.rs:152`. That is a test-local stand-in, which the brief excludes because it moves a
    `Set` entry to the end and never restores.
  - `holler-pane`'s `Profile`, `ProfileName` and `Pane` offer only `parse`, `as_str` and `slug`.
- **`belongs` (`:235-239`) is not a second `check_membership`.**
  - The two answer different questions. `belongs` asks whether a record names P, which `resolve` and
    `pane-not-in-profile` need. `check_membership` asks whether a stored record may be rewritten with Q, which the
    `pane-in-other-profile` refusal needs.
  - `check_membership` passes a pane that is in no profile, so it cannot stand in for `belongs`.
  - Both compare `ProfileName::slug`, as the map requires.
  - The hub's `check_membership` (`holler-hub/src/profile/mod.rs:67`) is a stub until #661, and the test kit may not
    depend on the hub (AC6).
- **The stores are reached only through their ports** (`Arc<dyn ProfileStore>` and `Arc<dyn PaneStore>`). The scope
  never reaches into a fake's internals. Its only crate-internal edges are `check_membership` and `feed::lock`, which is
  already `pub(crate)` and already used by `FakeProfileStore` (`profile_store.rs:22`).
- **The suite reuses rather than copies.**
  - `conformance/profile_scope.rs:79-84` and `act.rs:8-18` import part 1's `actor`, `sample`, `history`, `shown`,
    `unchanged`, `Step`, `UPDATED` and `ALPHA` to `C3` and `GAMMA`, slice a's `pane_name` and `profile_name`, and
    `run_cases`, `succeeds`, `expect_code` and `expect_eq`.
  - The fixture is built from `sample_pane`, `sample_spec`, `FakeProfileStore::seeded` and `FakePaneStore::seeded`.
  - Faults and calls go through `faults().fail_next` and `faults().calls()`, and the other writer through
    `concurrent_put`.
  - The new helpers are each new behaviour or a thin layer over a reused one. `gained` is built on `history`, and
    `specs` and `changed_spec` on `sample_spec`. `stored`, `records`, `edit_with`, `edit`, `expect_edited`, `act_failed`
    and `no_write` have no equivalent in another suite.
  - The per-suite code constants (`CONFLICT`, `PROFILE_NOT_FOUND`, `NOT_IN_PROFILE`, `PROFILE_CONFLICT`,
    `IN_OTHER_PROFILE`) follow the dominant pattern: the pane, profile and harness suites each keep their own.
  - `C4` and `SEEDED` are new and named in the cadence of `C1` to `C3`.
- **The suite's shape matches its neighbours.**
  - `Seeded<S>` (owned per case) plus `Bench<'a>` (the borrowed view a case gets) mirrors slice d's `HerdrFixture<H>`
    plus `Scratch<'a>` (`conformance/herdr.rs:57-75, 140-153`).
  - `act.rs` mirrors part 1's `profile_store/log.rs`: a child module with `pub(super)` cases that imports from `super`
    and `crate::conformance::pane_store::profile_name`.
- **The mutants hold no copy of the edit logic** (`tests/profile_scope_conformance_test.rs:88-130`):
  - `WritesAfterAct` and `NoRestore` only reorder the call to `act()` around a delegated `edit_spec` given a no-op act;
  - `MembershipOnRemove` delegates to `resolve` and maps one error;
  - `Break::Nothing` passes.
  - The `Break`/`Mutant`/`assert_suite_fails_on` scaffold is the accepted per-file copy of
    `tests/profile_store_conformance_test.rs:59-188`, which #694 is to move into `tests/support`.
- **The test-local helpers are accepted.** These are `Rig`, `member`, `edit_with`, `stored`, `code_of`, `timeout`,
  `corrupt` and `unavailable` in `tests/fake_profile_scope_test.rs`.
  - There is no `tests/common`, and integration tests cannot reach `pub(super)` suite helpers.
  - The sibling `fake_pane_store_test.rs` and `fake_profile_store_test.rs` each keep the same `timeout`, `corrupt` and
    `unavailable`.
  - The deadlock test's `mpsc` plus `recv_timeout` is not a copy of a wait helper. The test kit has none, and a
    deadlocked thread cannot be joined, which the two existing threaded tests do.
- **W-8 holds.**
  - The hook is taken out of its mutex in a `let` statement of its own (`profile_scope.rs:158`), so the temporary guard
    drops at the `;` before the hook is called (`:159-161`).
  - `before_next_restore` drops a replaced hook after the lock is released (`:90-93`).
  - T pins this with `a_hook_may_arm_the_hook_again_without_deadlocking`.
- **This stack's checks.**
  - **ADR:** the ADR diff is one sentence in section 8, step 1, carrying W-7's clause and `(#688)`.
  - **No out-of-radius changes:** the diff touches no protocol doc, golden file, `holler-proto` or `holler-pane`, and
    adds no new error code. `lib.rs`, `conformance/mod.rs` and every manifest are unchanged (AC6 and AC8, re-run).
  - **Size:** every touched `.rs` file is at 535 lines or fewer (the suite file 492, `act.rs` 219).
  - **Panic-free `src/`:** there is no `unwrap`, `expect`, `panic` or `#[allow]` in the new `src/` code. The one
    `.unwrap()` hit is inside doc prose.
  - **Public repository:** the diff names no personal infrastructure, and all seven branch commits carry the GitHub
    no-reply identity.
- **F's four deviations, checked against neighbouring code.** None of them adds drift.
  - `CREATED` stays private. The scope suite never reads it, and the AC7 grep does not cover constants.
  - The ADR sentence carries W-7's clause, as Phase 3 asked.
  - `pane_name` gains a doc line, matching `profile_name`.
  - `Seeded` plus `Bench` follows the herdr suite's shape.

## Notes for F

None: PASS.

## Notes for O

- W-1 is a single sentence to add to #694's body. S audits against the issues, and #694 is where the stale `lib.rs` line
  belongs.
- Issue #688's body is still the text from before the plan review (14 cases, 2 mutants, about 1,020 lines). A, F and T
  have each flagged this before S runs. I am not repeating it as a finding.
