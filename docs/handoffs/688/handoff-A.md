# Handoff-A: Phase 3 - #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-688-implementation (at 9fd9c1b, on origin/main 9d61c9f)
**Brief reviewed:** docs/handoffs/688-brief.md   **Reuse map:** docs/handoffs/688-brief.md, sections "Evidence", "Reuse refactors" and "Reuse map" (this run has no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on one finding. The rest of the plan is sound. It fills the two stubs over the existing fakes, which it holds as
`Arc<dyn …>` ports. It reuses `check_membership`, `run_cases` and part 1's suite helpers by visibility changes only, keeps
the test kit on `holler-pane` alone, and follows ADR-0021 section 8's I8 order step for step. The blocking problem is in
the fake's `edit_spec` step 3: it runs the `pane-in-other-profile` check for every edit, `Remove` included. `Remove` is
`pane close`, and ADR-0021 section 9 says `pane close` never answers that code. It also means a detached spec, which
section 8 allows, could never be removed through `edit_spec`. The fix also changes which edit case 13 and AC4 must use, so
it has to go into the brief before T writes tests. There are six warns besides.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-1 | block | "Fake behaviour", `edit_spec` step 3 (brief line 224): `check_membership` for every edit; the `ASSUMPTION (#661/#663)` bullet (line 282) | ADRs; cross-cutting (failure taxonomy) | Step 3 applies the `pane-in-other-profile` check to `SpecEdit::Remove` as well as to `Set`. `Remove` is what `pane close --profile P` sends, and ADR-0021:338 gives `pane close` only `pane-not-found`, `generation-conflict`, `profile-not-found` and `profile-conflict`. ADR-0021:282-283 says a spec naming another profile's pane (a detached spec) "is not refused", and the brief's own fixture has one: Alpha's c3 entry, where c3 belongs to Beta. As written, `edit_spec(Some(Alpha), c3, Remove, ..)` answers `pane-in-other-profile`, so a detached spec can never be removed through `edit_spec`. Every verb story that tests `close --profile` against this fake would get a code the ADR rules out for that verb. The brief's own reason for the check, "nothing is written to P and nothing live moves for a pane that cannot join P", applies only to a `Set`. No case or AC exercises the `Remove` path, so T, A-dup and S would not catch it, and #663 will read the fake as its reference implementation. | Amend the brief. Step 3 still calls `panes.get(pane)?` for every edit, because ADR-0021 section 8 step 1 reads the pane record and AC3's `[Get]` call shape should stay. It calls `check_membership` only for `SpecEdit::Set`. Reword the `ASSUMPTION (#661/#663)` bullet to "a `Set` for a pane of another profile". Pin the other half. The better option is a 15th suite case, for example `remove-of-a-detached-spec-is-not-refused`: `Remove` for c3 on Alpha drops Alpha's c3 entry at g + 1, and the act runs once. A suite case is the better choice because ADR-0021:338 binds #663 too. The minimum is an AC4 test on the fake. Name `Set(sample_spec(c1))` explicitly for case 13's c1 half and use a `Set` in AC4 `membership_compares_slugs`. Once the check is `Set`-only, a `Remove` there no longer tells the two check orders apart. |
| W-1 | warn | Case 14; the `ASSUMPTION (#661/#663)` bullet; "Decisions already made" ("the scope's earlier check is an addition", line 422); "**No ADR change**" (line 420) | ADRs | Case 14 binds #663 to a refusal the ADR does not state: `edit_spec` answers `pane-in-other-profile` before the profile write. ADR-0021 "Decisions taken" item 2 (lines 540-541) puts this check in the pane registry's compare-and-swap, and section 8's I8 order (lines 285-302) has no refusal before step 2. The check itself is right. Without it, a pane of Beta is relaunched with Alpha's spec before the registry refuses its record, which goes against I3. Precedent is mixed. Part 1's A (W-1) kept an ADR-silent rule in the suite's module doc without an ADR edit. Slice d amended ADR-0021 in its own PR (316b8e3) when its suite pinned a rule the ADR lacked, under the same `crates/holler-pane-testkit/**` blast radius. This repo's rule is that an extension updates the ADR in the same change. | Fold this into B-1's amendment. Add one sentence to ADR-0021 section 8 step 1: "a `Set` for a pane whose record belongs to another profile is refused here with `pane-in-other-profile`, before anything is written or moved; the registry's check (Decisions taken, item 2) stays the authority". Add `docs/adr/ADR-0021.md` to the blast radius and to AC8, and mention the ADR line in the CHANGELOG entry, as #683's entry did. If the operator prefers no ADR edit, state the rule and its reason in the suite's module doc and in decisions.md, as part 1 did with its W-1. |
| W-2 | warn | Files: `src/conformance/profile_scope.rs` ~650, split into `profile_scope/act.rs` "**If it nears 800**" (line 362) | size and structure | The estimate is already past the 600-line lint warn, and the trigger is looser than part 1's practice. Part 1's one-file suite reached 640 lines, and F split it into 521 + 129 + 144 lines. `profile_store/log.rs:1-2` gives the reason: "so that no file of the suite nears the 600-line lint". Part 1's tests ran about 30% over their estimate, so ~650 could land past 800. No `src/` file in the crate is over 600 today. | Plan the split up front: cases 9 to 14 (the act cases) go in `conformance/profile_scope/act.rs` from the start. Alternatively, set the trigger at the 600-line warn. Either way, list the file in the blast radius unconditionally. |
| W-3 | warn | Forward-compat row for #663 (line 412); cases 10 and 11 | ADRs; forward-compat | #663's acceptance says "a stale generation gives `profile-conflict`". Under ADR-0021 section 8, steps 2 and 6, a stale first write is `generation-conflict` (case 10) and only a stale restore is `profile-conflict` (case 11). The suite follows the ADR, which is correct. But the forward-compat row says "yes" without mapping #663's wording onto the cases, and #663 may read case 10 as a contradiction. | In the suite's module doc and the forward-compat row, map #663's acceptance bullets to case ids. Say that "a stale generation gives `profile-conflict`" is case 11, and that a stale first write is `generation-conflict` (case 10) under section 8 step 2. |
| W-4 | warn | Decision 1 (no fault switch on the scope, line 430); forward-compat row for the spec-editing verb stories (line 413) | forward-compat | Using the stores alone, a verb story cannot make `edit_spec` answer `profile-conflict`. That is the outcome after which ADR-0021 section 8 step 6 makes the verb print its reconcile step, which is something a launch, relaunch or close story will want to test. Case 11 gets it by writing the store from inside the act, but in a verb test the act is the verb's own code. `fail_next(CasPut, Conflict)` fails the first write, which gives `generation-conflict`. So the row's "yes" holds for every scope outcome except this one. | Pick one of two fixes. Option 1: add a one-shot hook on `FakeProfileScope`, where another writer stores P (through `concurrent_put`) between the next act and its restore, so the restore really conflicts, plus one AC4 test. Option 2: describe the technique in the forward-compat row, which is a test-local port wrapper whose call moves P and then fails. |
| W-5 | warn | "Fake behaviour" step 7 and decision 3: a restore that fails with anything but `Conflict` is returned as it is | cross-cutting (error boundaries) | When the restore fails with `timeout`, `store-corrupt` or `unavailable`, P keeps an edit that nothing live matches. Section 8 step 6 says that state must fail loudly with a reconcile step. Here the answer is a bare store error that does not name P, and the act's error is lost. The ADR decides only the `Conflict` case, and the codebase has no earlier compensating write to follow. Leaving it out of the suite is right, but #663 will copy the fake by default. | List it among the points #663 decides, in the suite's module doc. Optionally, the fake's error could name P and say that its specs were not restored. If W-1's ADR edit is made, section 8 step 6 can take one more sentence on it. |
| W-6 | warn | "Reuse refactors": part 1's helpers, `Shown`, `Step` and the name constants become `pub(super)`, and so does `pane_name` | dependency direction | This is right under the rule that no slice edits `conformance/mod.rs`, and part 1 set the precedent. But the scope suite will import from both sibling suites, `pane_store` and `profile_store`. Part 1's A (W-2) warned about exactly this fan-in. The follow-up it asked for was to move shared suite helpers into `conformance/mod.rs` once the stub slices had merged, and it was never filed (`gh issue list` shows none). #688 is the last stub slice, so the no-edit rule has done its job once it merges. | Build as planned. O files the follow-up now. After #688, it moves `profile_name`, `pane_name`, `actor`, `history`, `shown`, `unchanged`, the watch helpers and the shared `Demo …`/`demo-c*r1` names into `conformance/mod.rs` or a new `conformance/common.rs`. It also refreshes the "empty stubs" line in `lib.rs:28`, which is stale now that slices b, d and e have merged. |

Apart from these, the plan is consistent with existing patterns. I verified:

- **Every line the brief quotes is correct** as of 9d61c9f. This covers the port (`profile.rs:259-273, 373-404`), ADR-0021
  section 8 and "Decisions taken" items 1 and 2, the error variants and their `Display`, which carries `what` and so makes
  case 11's and AC4's message checks sound, and every `testkit` line reference.
- **The `build(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S` shape fits #663.** `Wiring` owns its ports and lends
  `Ports<'_>` (`holler-cli/src/pane/wiring.rs:25-46`). A real scope kept there cannot borrow its sibling stores, so it must
  own them, which is what `build` assumes. The herdr and harness suites already hand cases more than the bare subject, and
  no guard `K` is needed over two in-memory fakes.
- **No missed reuse candidate.** `MemScope` (`holler-pane/tests/ports_test.rs:103-158`) is a test-local stand-in in another
  crate. It writes after the act and moves a `Set` entry to the end, and the brief says so. `Unwired` is a placeholder. The
  hub has no `pane-in-other-profile` rule yet (#661 adds it), so `check_membership` is the only copy in the workspace, and
  making it `pub(crate)` keeps it that way.
- **Naming and layout mirror part 1:** `FakeProfileScope`, `run_profile_scope_conformance`, `profile_scope_cases`, kebab
  case ids, `tests/{profile_scope_conformance_test.rs, fake_profile_scope_test.rs}`, and the `foo.rs` + `foo/` child
  module for the split.
- **Lints:** widening visibility trips nothing. `dead_code` is the only workspace rust lint, and there is no
  `unreachable_pub`.
- **Concurrency:** the scope holds no lock across `act()`, and every fake method takes `&self`, so cases 8 and 11, whose
  acts read and write the store, are re-entrant without deadlock.
- **The `ASSUMPTION (#N)` form** is the crate's convention (`herdr.rs`, `harness.rs`, `conformance/herdr.rs`).
- **Smaller points.** "The scope writes no pane record" is the only reading the frozen `act: FnMut() -> Result<(), _>`
  allows, and it is flagged for #663. Exact `spec.pane == pane.as_str()` matching is correct because `PaneName` is
  verbatim (`pane.rs:44-47`). No open PR touches the test kit.

## Notes for O

**What to amend before a fresh run.** In the automated path, amend the brief and start a new run. Do not use
`resumeFromRunId`, which replays this verdict.

1. **B-1 (required).** Make these four changes to the brief:
   - In "Fake behaviour" step 3, keep `panes.get(pane)?` for every edit and run `check_membership` for `SpecEdit::Set` only.
   - Reword the `ASSUMPTION (#661/#663)` bullet to "a `Set` for a pane of another profile".
   - Add the `Remove`-of-a-detached-spec rule. A 15th suite case, `remove-of-a-detached-spec-is-not-refused`, is the
     better place, because the ADR binds #663 too. If it becomes a case, update AC1's id list, the "14" counts and AC9's
     wording. The minimum is an AC4 test on the fake.
   - Spell out `Set(sample_spec(c1))` for case 13's second call and a `Set` for AC4 `membership_compares_slugs`.

   An optional third mutant, `MembershipOnRemove`, would prove that the new case discriminates.
2. **W-1 (recommended, same amendment).** Add the one-sentence ADR-0021 section 8 step 1 note quoted in the table. Add
   `docs/adr/ADR-0021.md` to the blast radius and AC8, and replace "**No ADR change**" with what changed.
3. **W-2 to W-5** can go into the same amendment cheaply: an unconditional `act.rs`, the #663 case mapping, the
   `profile-conflict` route for verb stories, and the restore-failure point added to #663's list.
4. **W-6** is a follow-up issue, not a brief change.
5. **Minor.** Decision 1 gives the reason "Every other fake has its own switch because it *is* a port's boundary". That
   is not quite true: `FakeProber` has no switch (`prober.rs:7-9`). It is the precedent for a fake without one, so the
   `FakeProfileScope` module doc should give the scope's own reason, as `FakeProber`'s does, and drop the general claim.

**What Phase 7 will check:**
- AC7's two greps.
- `check_membership` is the only producer of `PaneInOtherProfile` in `src/` and is called only for a `Set`.
- The scope suite imports `actor`, `sample`, `history`, `shown`, `unchanged`, `pane_name`, `profile_name`, `run_cases`,
  `succeeds`, `expect_code` and `expect_eq` and keeps no local copy of any of them.
- The test files' mutants delegate to `FakeProfileScope` and hold no copy of the edit logic.
- `lib.rs` and `conformance/mod.rs` are untouched.

The accepted near-duplicates are the per-file mutant harness and the test-local helpers of the two new test files
(`actor`, `name`, `timeout`, …), because integration test files share nothing without `tests/common`.

## Patterns referenced

- `crates/holler-pane-testkit/src/{profile_store.rs, pane_store.rs, fault.rs, fixture.rs, prober.rs}` and
  `src/conformance/{mod.rs, pane_store.rs, profile_store.rs, profile_store/log.rs, profile_store/watch.rs, herdr.rs,
  harness.rs}`; `tests/profile_store_conformance_test.rs` (the mutant harness)
- `crates/holler-pane/src/profile.rs:175-177, 259-273, 373-404`, `src/error.rs:400-470, 640-690`
- `docs/adr/ADR-0021.md` sections 4 (I8, line 167), 5 (lines 190-192), 8 (lines 264-302), 9 (lines 326-346, the
  per-verb codes), and "Decisions taken" items 1 and 2
- `crates/holler-cli/src/pane/wiring.rs:25-46, 203-220`, `crates/holler-pane/src/ports.rs:227-235`,
  `crates/holler-pane/tests/ports_test.rs:103-158`
- Issues #688, #663 and #638. Part 1's Phase 3 review and decision journal (`git show 95c44d2^:docs/handoffs/682/`) and
  slice d's ADR edit (316b8e3)
