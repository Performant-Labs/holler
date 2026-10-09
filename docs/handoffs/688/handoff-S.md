# Handoff-S: Phase 8 - #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite  (spec audit)

**Date:** 2026-10-09 (audited at 15:25 MDT)
**Branch:** issue-688-implementation (at 7114c33, on origin/main 9d61c9f, which is also the merge base; working tree clean)
**Issue:** #688 (part of #638, epic #633; the second half of #682). Rigor: in-session. UI surface: none.
**Handoffs reviewed:** docs/handoffs/688-brief.md (as amended at 98c250e), docs/handoffs/688/{handoff-A.md,
handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, decisions.md, evidence.md}, and the appendix the
issue points to ("Fixed for part 2", `95c44d2^:docs/handoffs/682-brief.md`, lines 489-565).
**Diff audited:** `git diff origin/main...HEAD`: ten code, test, ADR and changelog files, plus pipeline artifacts. Nothing
under `crates/`, `CHANGELOG.md` or `docs/adr/` changed after F (628bcab), and no test changed after T-red (b0bf1ee). So F
edited no test, and T-green's run at 05d7262 covers the code at HEAD.

## A precondition

Met. handoff-A.md (Phase 3, the re-review after the amendment) is **PASS**, with warns W-7 and W-8 and no block.
handoff-A-dup.md (Phase 7, at 05d7262) is **PASS**, with warns W-1 and W-2 and no block.

## T precondition

Met. handoff-T-green.md lists **no blocking issues**.

- **RED** (handoff-T-red.md): both new test targets failed to build with E0432. The only causes were the brief's three
  missing public items (`FakeProfileScope`, `profile_scope_cases`, `run_profile_scope_conformance`). T then compiled the
  tests against three throwaway versions and reverted each:
  - a reference fake, which passed all 18 fake tests (so each one can be satisfied);
  - a variant that held the hook's lock, which failed the W-8 deadlock test after 10 s;
  - a stub suite, which failed the id-list test and the three mutant tests on their assertions.
- **GREEN** (handoff-T-green.md): the 18 + 6 new tests pass, and passed 5 more runs without a flake. The four earlier
  targets pass unedited, and the workspace run has 1279 passed and 0 failed. A throwaway no-op restore failed 6 of the 18
  fake tests.
- The test counts in the files match: 18 and 6 `#[test]`s.

## Acceptance criteria

**The issue's scope.** The issue body predates the plan review; see Scope check and advisory N-1.

| Issue clause | Proving test or evidence | Status |
|---|---|---|
| `FakeProfileScope` in `holler-pane-testkit`, writing the profile first (ADR-0021 I8) | `src/profile_scope.rs:209-231`. Case 8 `the-act-sees-the-edit`: inside the act, `get(Alpha)` already holds the edit at g + 1. `the_calls_follow_the_i8_order`: profile calls are `[Get, CasPut]`, or `[Get, CasPut, CasPut]` after a failed act | Met |
| A failed act restores the specs, and the generation moves by 2 | Case 9 `failed-act-restores-the-specs`: it returns exactly the act's error, the specs are equal to before, the generation is g + 2, and the log gained `(g+1, updated)` and `(g+2, updated)` by one actor. The pane list is unchanged and the act ran once. `the_log_carries_the_scopes_actor` checks the two entries' actor | Met |
| `run_profile_scope_conformance`: the 14 cases fixed in the #682 appendix | All 14 appendix case ids are present as cases 1 to 14, in the appendix's order (checked against `95c44d2^:docs/handoffs/682-brief.md:541-556`). The amended brief sharpens three of them: cases 13 and 14 now name a `Set`, and case 9 also checks that one actor wrote both entries | Met. The suite is a superset: case 15 is added (see Scope check) |
| The 2 scope mutants of the appendix | `a_scope_that_writes_the_profile_after_the_act_fails` fails on `the-act-sees-the-edit` and `failed-act-restores-the-specs`. `a_scope_that_does_not_restore_on_a_failed_act_fails` fails on `failed-act-restores-the-specs` | Met. The mutants are a superset: `MembershipOnRemove` is added |
| The appendix's own fake tests | `a_failed_restore_returns_its_own_error` (with `store-corrupt`, as the amended brief says), `resolve_of_a_pane_with_no_record_is_pane_not_in_profile` and `the_scope_is_send_and_sync` | Met |
| Needed by #663 | The suite's module docs (`conformance/profile_scope.rs:58-64`) map #663's acceptance bullets to case ids. I checked the mapping against #663's acceptance text. Its "a stale generation gives `profile-conflict`" is case 11, a stale restore; a stale first write is case 10's `generation-conflict`, under ADR-0021 section 8 step 2 | Met |

**The brief's AC1 to AC10:**

| AC | Proving test or evidence | Status |
|---|---|---|
| AC1 the fake passes its suite, which runs the 15 documented ids | `the_fake_passes_the_profile_scope_conformance_suite`. `the_suite_runs_the_documented_cases`: `DOCUMENTED_CASES` equals the brief's 15 ids in order, which I compared one by one, and `CASES` (`conformance/profile_scope.rs:103-155`) lists the same 15 | Met |
| AC2 the mutation check | `enum Break { Nothing, WritesAfterAct, NoRestore, MembershipOnRemove }` and `struct Mutant { inner: FakeProfileScope, broken }`. `resolve` delegates, and `assert_suite_fails_on` follows part 1's harness. I traced each mutant through its named case (below). The mutants only reorder the act around a delegated `edit_spec`, or add one delegated refusal, so the test file holds no copy of the edit logic | Met |
| AC3 faults reach the scope through its stores | `a_wedged_profile_store_times_out_resolve_and_edit_spec`, `a_wedged_pane_store_times_out_resolve_and_edit_spec`, `a_corrupt_profile_store_fails_closed`, `a_failed_restore_returns_its_own_error`, `without_a_profile_a_wedged_profile_store_is_not_called` and `the_calls_follow_the_i8_order`. Each asserts the exact error (`Timeout { op: "profile_store.get" }`, `"pane_store.list"`, `"pane_store.get"`, `StoreCorrupt`), how often the act ran, and, where the brief asks, that the profile store saw no `CasPut` | Met (advisory N-4) |
| AC4 the fake's own rules | All nine tests the brief names are present. The brief lists three clauses under `a_hook_before_the_restore_makes_it_profile_conflict`. The third (a hook armed before a succeeding act is still armed afterwards) is asserted in T's sibling test `a_hook_stays_armed_until_a_failed_act_of_an_edit_with_a_profile`. That test also covers a failed act without a profile and a conflicting first write. T added `arming_the_hook_again_replaces_an_unused_one` and `a_hook_may_arm_the_hook_again_without_deadlocking` (A's W-8) | Met |
| AC5 earlier slices still hold | `git diff --name-only origin/main...HEAD -- crates/holler-pane-testkit/tests/` lists only the two new files. The four earlier targets pass (T-green: 10, 22, 9 and 23 tests) | Met |
| AC6 dependencies | `git diff --quiet origin/main -- crates/holler-pane-testkit/Cargo.toml Cargo.toml Cargo.lock` exits 0 (I ran it). F's `cargo tree` grep printed nothing. The manifests and lock are unchanged, so the dependency tree is too | Met |
| AC7 one rule, one place | Both greps print nothing (I ran them). Reading the code confirms the rule. `check_joins` is the scope's only caller of `check_membership` (`profile_scope.rs:270`). `plan` reads the pane record for every edit (`:139`) and calls `check_joins` only under `if let (SpecEdit::Set(_), Some(record))` (`:140-142`). The suite imports part 1's and slice a's helpers and defines no copy of them (`conformance/profile_scope.rs:79-81`, `act.rs:8-16`) | Met |
| AC8 layout | `git diff --name-only` lists exactly the blast radius: `src/lib.rs` and `src/conformance/mod.rs` are absent, and nothing outside `crates/holler-pane-testkit/`, the ADR, `CHANGELOG.md` and `docs/handoffs/688*` is touched. The ADR diff is one sentence (3 lines) in section 8 step 1 (lines 289-291) and nothing else. Its wording goes beyond the brief's quote: it adds A's W-7 `--spec-only` clause and `(#688)` (F's deviation 2) | Met. The wording change was sanctioned by A |
| AC9 CHANGELOG | One entry, last under `## [Unreleased]` / `### Enhancements` (the only subsection there), after the #681 entry. It covers the I8 order, the restore and the generation moving by two, `profile-conflict`, the hook, the 15 cases and the three broken scopes, the ADR line, and "test code only". It links #688 and the epic #633. T-green: `changelog-check: ok` | Met |
| AC10 guards | **T-green's Tier 1:** clippy `-D warnings --all-targets`, `cargo test --workspace`, `lint.sh`, `changelog-check` and `machete` all PASS. The test kit inherits the workspace lints (`[lints] workspace = true`), so clippy denies `too_many_lines`, `cognitive_complexity` and `dead_code`. **F:** the build, `rustfmt --check --edition 2021` on the 8 `.rs` files and `test-hooks.sh` all PASS. **`wc -l` (my run):** the largest touched `.rs` file has 535 lines, and the two suite files have 492 and 219, under the 600-line warn | Met |

**AC2, traced by reading.** `WritesAfterAct` runs the act first. In case 8 the act therefore sees g = 1. In case 9 the
failing act returns before any write, so the generation stays at 1, not g + 2. `NoRestore` leaves `s'` at g + 1 in
case 9. `MembershipOnRemove` asks `resolve(Alpha, c3)`, gets `pane-not-in-profile` and turns it into
`pane-in-other-profile`, which fails case 15. Case 7, the suite's other `Remove`, removes c2, a member, and passes. F's
throwaway probe printed each mutant's full failure list and agrees: `WritesAfterAct` also fails cases 10, 11, 13 and 14,
`NoRestore` also fails case 11, and `MembershipOnRemove` fails case 15 alone.

## Spec compliance

- **Public API: exactly as the brief pins it.**
  - `FakeProfileScope` has the four private fields. Its hook field is a `Mutex<Option<RestoreHook>>`, where `RestoreHook`
    is an alias for `Box<dyn FnOnce() + Send>`.
  - `new(Arc<dyn ProfileStore>, Arc<dyn PaneStore>, Actor)` and `before_next_restore(&self, impl FnOnce() + Send + 'static)`
    have the brief's signatures.
  - `profile_scope_cases() -> Vec<&'static str>` and `run_profile_scope_conformance<S, F>(build: F) -> Conformance` have
    the brief's bounds. The doc's `text` fence is the brief's, verbatim.
  - There are no flat re-exports, and `lib.rs` and `conformance/mod.rs` are untouched.
- **Fake behaviour: each step of the brief, in order.**
  - **`resolve`** checks `profile-not-found` first. With no pane it lists the panes whose record names P (slug compared),
    sorted by name. With a named pane, that pane's record must name P, or else it is `pane-not-in-profile`, which also
    covers a pane with no record. It returns the stored P.
  - **`edit_spec(Some(P))`**, step by step:
    1. Get P (`profile-not-found` before any other call).
    2. A `Set` whose spec names another pane is `usage`.
    3. Get the pane record, for every edit. A `Set` is checked for membership through `check_membership`; a `Remove` is
       never checked.
    4. Apply the edit: a `Set` replaces the entry in place or appends one, and a `Remove` uses `retain`.
    5. `cas_put` at g, with no retry.
    6. Run the act.
    7. If the act fails, take the hook in a statement of its own, run it, then `cas_put` the old specs at
       `written.generation`. A `Conflict` there becomes `ProfileConflict` naming P. Any other error is returned as it is.
  - **`edit_spec(None)`** runs only the act, through an early return (`:217-219`).
  - No lock is held across the act or the hook. The mutex is read through `crate::feed::lock`, which is the same
    `unwrap_or_else(PoisonError::into_inner)` expression the brief names, reused rather than copied.
- **The suite: the 15 cases follow the brief's table.** Cases 1 to 8 are in `conformance/profile_scope.rs` and cases 9
  to 15 in `conformance/profile_scope/act.rs`. Each case asserts what its row says. Specifically:
  - Case 1 compares the panes with the store's records. Cases 5 and 9 compare `panes.list()` before and after, which pins
    that the scope writes no pane record.
  - Cases 10 and 14 use part 1's `shown` and `unchanged`.
  - Case 11 checks that the error's `Display` contains `Demo Alpha`.
  - Cases 13 and 14 assert that the act ran 0 times and there was no `CasPut`.
  - Case 15 checks Beta and the pane list.

  No case passes against a do-nothing scope. I traced all 15 against one. The suite pins only what the brief allows:
  - no `what` text beyond case 11's profile name;
  - no actor value, only that the edit and its reversal share one (through `gained`);
  - no order of pane-store calls.

  Case 1's `[c1, c2]` order is the one `FakePaneStore::list` returns anyway (its records come from a `BTreeMap`), so it
  adds no unlisted constraint for #663.
- **The `ASSUMPTION (#663)` and `ASSUMPTION (#661/#663)` comments.** Every point in the brief's list has its comment at
  the case or helper concerned, and the fake carries the same comment at its code. The module docs summarise them, give
  the open restore-failure point, and give the #663 case mapping. The comments follow the repo's existing
  `// ASSUMPTION (#640): ...` form.
- **Decisions already made: each one is implemented.**
  - The profile is written first, the restore is a second write at g + 1, and a conflict on the restore is
    `profile-conflict`.
  - The scope checks membership for a `Set` only, in addition to the registry, and the ADR sentence records it.
  - The fake is in the test kit, which names neither `holler-cli` nor `holler-hub`.
  - No slice edits `lib.rs` or `conformance/mod.rs`.
- **Decisions made in the brief: each one is implemented.** The fake has no fault switch of its own; the module doc gives
  the scope's own reason. `profile-not-found` comes before `pane-in-other-profile`. The fake's own guards are in place.
  The verb records the pane inside its act. Reuse goes through visibility changes only.
- **F's four deviations.** Each is documented in handoff-F.md, none is silent, and none changes behaviour that a test or
  the issue pins.
  1. `CREATED` stays private. The scope suite never reads it, so a "reused" doc line on it would be false.
  2. The ADR sentence carries A's W-7 clause and `(#688)`. A's Phase 3 Notes for O item 1 asked for exactly this, and it
     is still one sentence.
  3. `pane_name` gains a doc line, matching `profile_name`.
  4. The suite's internal shape is `Seeded<S>` plus `Bench<'a>`. The brief left that shape open, and it mirrors the
     herdr suite.

## Quality audit

- **Correctness and failure handling.**
  - The scope fails closed. A wedged or corrupt profile store refuses before the pane store is read and before the act.
    A wedged pane store refuses before any write. A restoring write that fails returns its error rather than claiming
    success.
  - The one lossy path is a non-conflict failure of the restoring write: the act's error is lost and P keeps the edit.
    That is the brief's open point, documented at the code (`profile_scope.rs:180-184`) and in the suite's module docs
    for #663 to decide. No case pins it.
  - Concurrency is fenced by the profile store's compare-and-swap. No lock is held across the act. The hook is taken out
    of its mutex in its own `let` statement (`:158`), so the guard drops before the hook runs, and a replaced hook is
    dropped after the lock is released (`:90-93`). A two-thread test pins this, bounded by `recv_timeout`, with no sleep.
- **Build guards.**
  - **Panics:** the added `src/` lines contain no `unwrap`, `expect`, `panic!`, `assert!`, `unreachable!` or `todo!`.
    The two hits of my scan are doc prose: a `text` fence and an inline code span in `before_next_restore`'s doc.
  - **`#[allow]`:** none was added in `src/`. Both test files open with
    `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #688`.
  - **Size and dead code:** no file is near 900 lines. There is no dead code; clippy denies it, and T-green is clean.
- **Protocol:** none. No protocol doc, golden file, `holler-proto` change or new error code. The scope returns only
  closed `PaneError` variants.
- **Tests.**
  - This is in-process composition over in-memory fakes, so no hub or body harness is needed.
  - No fixed sleeps. RED-first evidence is in handoff-T-red.md.
  - Each test pins a behaviour rather than the implementation. The suite reads results back through the ports and the
    fakes' public inspectors, and the mutants prove the suite discriminates.
- **Documentation.**
  - The CHANGELOG has the entry (AC9), and the ADR gains the one sentence in section 8 step 1.
  - No other `docs/` page or README names the test kit's fakes (I grepped), and there is no new log event, CLI surface
    or protocol field.
  - `lib.rs:28-29` will be stale once this merges (advisory N-2). AC8 bars this run from editing it.
- **Public-repository privacy.** I grepped the added lines of the whole diff, handoffs included, for personal names,
  home paths, hostnames, tailnet names, IPs, account names, private domains and secrets: no hits. The fixtures use
  `Demo *` and `demo-c*r1`, and the actors are `conformance`, `seeder`, `scope-actor`, `other-writer` and
  `scope-under-test`.
- **Commit and PR hygiene.**
  - All eight commit subjects pass `.githooks/commit-msg`'s Conventional Commit regex.
  - Each commit carries the GitHub no-reply identity and the `Co-Authored-By: Claude <noreply@anthropic.com>` trailer
    that `.githooks/prepare-commit-msg` adds.
  - No PR exists yet, and the branch is not pushed, so the PR body's AI disclosure cannot be checked at this phase
    (advisory N-3).

## Scope check

**Relative to the brief: exactly its scope.** The diff touches the brief's blast radius and nothing else, with no
unrelated refactor. The three visibility changes are the brief's reuse refactors, and the ADR edit is the one sentence.

**Relative to the issue body: a documented superset.** The issue asks for "the 14 cases and 2 scope mutants" of the #682
appendix and estimates about 1,020 lines. The run delivers those 14 case ids and both mutants, plus additions the plan
review required:

- **Case 15 and the `MembershipOnRemove` mutant.** A's BLOCK B-1 found that the appendix's step 2 refused a `Remove` of a
  detached spec. That contradicts ADR-0021:282-283 and the `pane close` row of section 9 (now line 341).
- **The `before_next_restore` hook (W-4).** Without it a verb test cannot reach `profile-conflict`.
- **The ADR sentence (W-1, plus W-7's clause).**

The brief records each addition ("Amended after the plan review", "Where the appendix is stale"). None is silent, and
none is unrelated work. The changed lines in code, tests, the ADR and the changelog total about 1,760; the brief
re-estimated about 1,750. The issue body itself was never amended to match (advisory N-1).

## Verdict

**PASS.**

- Every acceptance item of the issue and AC1 to AC10 of the brief has a proving test or a check that I verified.
- The decisions are implemented as stated, and F's four deviations are documented and sanctioned.
- The quality audit finds nothing blocking. Nothing in `src/` or the tests needs a change.

The notes below are for O; none blocks the merge. N-1 should be done before the merge, because the script's handoff
cleanup removes `docs/handoffs/688*` before the push.

## Advisory notes

- **N-1 (O, before merge): amend issue #688's body.** It still says "the 14 cases and 2 scope mutants fixed in the
  appendix of `docs/handoffs/682-brief.md` (on the `issue-682-implementation` branch until part 1 merges)" and "~1,020
  lines". That file is on neither `main` nor the tip of `origin/issue-682-implementation`; it exists only at
  `95c44d2^:docs/handoffs/682-brief.md`.
  - **Why it matters:** this run's brief is removed by the handoff cleanup too. Once that happens, the issue is the
    planning record left besides the suite's module docs, which do carry the substance.
  - **Who flagged it:** A (Phase 3 Notes for O, item 2), F, T-green and A-dup each asked for a dated note. It was still
    absent when I re-read the issue at 15:27 MDT; the issue was last updated at 13:28 MDT, before the plan review.
  - **Suggested text:** "(amended 2026-10-09, plan review) 15 cases and 3 scope mutants: case 15
    `remove-of-a-detached-spec-is-not-refused` and the `MembershipOnRemove` mutant pin that a `Remove` is never refused
    for membership (ADR-0021 section 8 and the `pane close` row of section 9). The fake also has a one-shot
    `before_next_restore` hook, so a verb test can reach `profile-conflict`. ADR-0021 section 8 step 1 gains one sentence
    recording the scope's `Set`-only `pane-in-other-profile` check. The #682 appendix is at
    `95c44d2^:docs/handoffs/682-brief.md`. About 1,760 lines."
- **N-2 (O): give the stale `lib.rs` line an owner.** `lib.rs:28-29` ("The modules of slices b to e are empty stubs")
  becomes false with this merge, because no stub is left. The `(slice c, #682)` labels on `profile_scope` in
  `lib.rs:19-20` and `:23-24` and in `conformance/mod.rs:12-13` name only part 1. AC8 bars this run from editing those
  files. #694's body, read now, still does not list them, as A (Phase 3, note 3) and A-dup (W-1) asked. Add one line to
  #694's scope.
- **N-3 (the run's agent, after the PR opens): add the AI disclosure.**
  - The script's PR body does not carry the disclosure. Add it with `gh pr edit`, per `CONTRIBUTING.md` ("Disclosure is
    required") and this repo's CLAUDE.md.
  - Separately: `CONTRIBUTING.md:20-21` says agent commits carry a session link. No recent squash commit on `main` has
    one (9d61c9f, 316b8e3, 434a1b5, 90997a2), and the repo's hook adds only the trailer. That mismatch between the doc
    and practice is repo-wide, not this run's.
- **N-4 (test strength, no change needed): `a_failed_restore_returns_its_own_error`** (`tests/fake_profile_scope_test.rs:226-245`).
  - **The gap:** the test asserts generation 2 but not "with the edit". Its edit `set(C1)` equals the seeded c1 entry,
    because `sample_profile` builds every entry with `sample_spec` (`fixture.rs:126`), so the specs cannot show the edit.
  - **Why no change is needed:** the test still tells the cases apart. Generation 2 means the restore did not land,
    against 3 if it did and 1 if nothing was written. The exact `store-corrupt` error rules out a fake that returns the
    act's error instead.
  - **For #663:** when #663 settles this open point, use a changed spec, so that "P keeps the edit" is visible.
- **N-5 (for #644): ADR wording near the new sentence.**
  - ADR-0021:283 ("a detached spec is not refused") speaks of the registries. The new sentence at :289-291 says the scope
    refuses a `Set` that would create or update a detached spec, with or without `--spec-only`. Read in context they
    agree; A's Phase 3 reading is the same. When #644 settles `--spec-only`, adding "by the registries" to line 283
    would remove the apparent tension.
  - #644 inherits the refusal of `launch --profile P --spec-only X` for an X in another profile. The brief's Risks rule
    keeps it reversible: amend case 14 first.
- **N-6 (optional): the fake's "calls neither store" without a profile has no test.** The brief's "Fake behaviour" says
  `edit_spec(None, ..)` makes no pane store call either. The early return (`profile_scope.rs:217-219`) implements it, but
  no test pins it:
  - case 12 pins only the profile store, as the brief's table decides, so #663 is not bound further;
  - `without_a_profile_a_wedged_profile_store_is_not_called` wedges only the profile store.

  One more assertion there, `r.panes.faults().calls().is_empty()`, would pin it if wanted. It is not an AC.
- **N-7 (carried from A's Phase 3 note 4, for #646).** `close --profile Alpha c3`, where c3 belongs to Beta, removes
  Alpha's detached spec (case 15). The verb's act would then close Beta's live pane. Whether it should is #646's
  question. Case 15 pins only the scope and holds either way.
