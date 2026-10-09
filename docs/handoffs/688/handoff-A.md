# Handoff-A: Phase 3 - #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-688-implementation (at 98c250e, on origin/main 9d61c9f)
**Brief reviewed:** docs/handoffs/688-brief.md, as amended at 98c250e   **Reuse map:** the same brief, sections "Evidence", "Reuse refactors" and "Reuse map" (this run has no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This is the re-review after the amendment. It replaces the BLOCK pass at db4cba8, whose handoff stays in git history.
Finding numbers continue from that pass.

## Summary

PASS. The amendment fixes B-1 as asked. The `pane-in-other-profile` check now runs for a `Set` only, and the pane record is
still read for every edit. Case 15 pins that removing a detached spec is not refused. The new `MembershipOnRemove` mutant
shows that this case catches the old behaviour. Every warn from the first pass is addressed, and the W-6 follow-up is filed
as #694. Two new warns concern what the amendment added. Neither changes the API that T writes tests against. W-7: the
`Set` check also refuses a `--spec-only` edit, and the new ADR sentence should say so. W-8: the restore hook must run after
its mutex has been released.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-7 | warn | "Fake behaviour" step 3; the `ASSUMPTION (#661/#663)` bullet; "ADR edit"; case 14 | ADRs; forward-compat | #663 says "`--spec-only` makes `edit_spec` skip the act", and `edit_spec` has no flag for it, so the verb passes an act that does nothing. The scope can't tell that call from a live one. So the `Set` check also refuses `launch --profile P --spec-only X` (and the same `relaunch`) when X belongs to Q. Nothing live would move there, and the result would only be a detached spec. ADR-0021:66-68 and :282-283 treat a detached spec as a normal state, and `profile create --from` makes them. The check also means a detached spec that already exists, such as the fixture's Alpha c3 entry, can be removed through `edit_spec` but never updated. This is not drift. Section 9 (line 335) lists `pane-in-other-profile` for `launch` and `relaunch` with no `--spec-only` exception, the choice fails closed, and `profile apply --take-over` remains the way to adopt a pane. But the new ADR sentence is where this rule gets written down, two paragraphs after "a detached spec is not refused", and as quoted it does not mention `--spec-only`. #644, whose acceptance includes "`--spec-only` changes P and nothing live", would have to work this out on its own. | Extend the same sentence, for example "...before anything is written or moved, with or without `--spec-only`, since the scope cannot see that an act is empty; a `Remove` is not...". Mirror it in the `ASSUMPTION (#661/#663)` bullet and in the forward-compat row for the spec-editing verbs. The ADR edit then stays the one sentence AC8 allows. If the operator would rather let `--spec-only` write a detached spec, the check cannot stay in the scope, because the scope cannot see the flag. It would move to #644's verb, and case 14 would be amended there under the brief's Risks rule. That decision belongs to a later story, not this run. |
| W-8 | warn | `before_next_restore`; "Fake behaviour" step 7 ("take and run the hook"); Risks | concurrency | (a) The workspace is on edition 2021 (`Cargo.toml:10`). There, the `MutexGuard` in `if let Some(hook) = self.restore_hook.lock()....take() { hook() }` lives to the end of the block, so the hook would run while the scope's own mutex is held. The doc invites re-arming ("Arming it again replaces an unused hook"). A hook that re-arms, or that calls back into the scope, would then deadlock, and in a test a deadlock hangs instead of failing. Risks says "no lock is held across `act()`" but says nothing about the hook. (b) The public doc says "after the next act that fails", but step 7 is on the `Some(P)` path only. It does not say whether a failing act under `edit_spec(None, ..)` uses up the hook. | Take the hook out in a `let` statement of its own, so the guard drops at the `;`, then call it. Extend the Risks line to "nor across the hook". Make the doc say "the next failed act of an `edit_spec` with a profile; a failing act without a profile, and a first write that fails, leave it armed". Optionally, add the `None` half to AC4's hook test (one line). |

### The first pass's findings, after the amendment

- **B-1 (block): resolved.**
  - Step 3 reads the pane record for every edit, so AC3's `[Get]` shape stays, and calls `check_membership` only for a
    `Set`.
  - Case 15, `remove-of-a-detached-spec-is-not-refused`, pins the other half. It cites ADR-0021:282-283 and :338, and I
    re-verified both.
  - Case 13 now uses a `Set` for c1, so it still tells the two check orders apart. AC4 `membership_compares_slugs` uses a
    `Set`.
  - `MembershipOnRemove` fails on case 15 alone, because the suite's only other `Remove`, case 7, removes a member. The
    harness asks only that the named case be among the failures (`profile_store_conformance_test.rs:176-188`). The mutant
    delegates through `resolve`, so the test file holds no copy of the edit logic. Building the `PaneError` in the test
    file matches part 1's `ForgetsLogOnDelete`.
- **W-1: resolved.**
  - The one-sentence edit goes in section 8 step 1 (line 288, verified), and the blast radius, AC8 and AC9 now list it.
  - The precedent claim holds. #683's issue radius did not list ADR-0021, yet 316b8e3 edited it, under the stack's rule
    that an ADR extension lands in the same change.
  - The sentence agrees with section 9: line 335 lists `pane-in-other-profile` for launch and relaunch, and line 338 leaves
    it out for close. It keeps "Decisions taken" item 2 as the authority. W-7 suggests one clause to add.
- **W-2: resolved.** `act.rs` is planned from the start, mirroring `profile_store/log.rs`, whose child `pub(super)` cases
  import from `super::`. AC10 keeps both suite files under 600 lines. Even part 1's 30% overrun leaves the estimates of
  ~420 and ~330 lines under that.
- **W-3: resolved.** The mapping goes in the module doc and in the forward-compat row. I checked it against #663's
  acceptance text.
- **W-4: resolved** by `before_next_restore`, the first pass's Option 1.
  - `FaultSwitch` holds only `Fault` and `PaneError` values (`fault.rs:26-52`). It cannot place another writer's version
    between the act and the restore, so the hook is the narrowest addition and not a second fault path. It is local to the
    fake, and an AC4 test pins it.
  - It keeps `FakeProfileScope: Send + Sync`. `ProfileStore`, `PaneStore` and `ProfileScope` are all `Send + Sync`
    (`profile.rs:328, 380`; `ports.rs:62`), and the hook is a `Box<dyn FnOnce() + Send>` behind a `Mutex`.
  - The mutex is read with `unwrap_or_else(PoisonError::into_inner)`, as the brief says `FakeProber` does
    (`prober.rs:71`). W-8 covers the hook's lock.
- **W-5: resolved.** A restore that fails with anything but a conflict is listed as open for #663.
- **W-6: resolved.** Filed as #694.
- **Minor, Decision 1's claim about `FakeProber`: resolved.** The decision now gives the scope's own reason and cites
  `prober.rs:7-9`, which I verified.

### Re-verified, unchanged since the first pass

- `origin/main` is still 9d61c9f (`git ls-remote`). The only open PR is dependabot #673, which touches nothing in the
  blast radius.
- The Unreleased `### Enhancements` section of `CHANGELOG.md` still ends with the #681 entry, so AC9's placement holds.
- `expect_code` compares code strings (`conformance/mod.rs:75-90`), so AC7's first grep can pass without contortion.

Apart from W-7 and W-8, the amended plan is consistent with existing patterns.

## Notes for O

Nothing blocks. These are cheap, and none of them needs another plan review.

1. **W-7.** F can add the clause while writing the ADR sentence. It stays one sentence, so AC8 still holds. Optionally,
   end the sentence with "(#688)", as slice d's ADR edit carried "(#638 amendment 2026-10-08, grid; #683)". That lets
   #663 find the suite that pins it.
2. **The issue body.** Issue #688 still says "the 14 cases and 2 scope mutants" and "~1,020 lines", and it mentions
   neither the hook nor the ADR line. The issue is this repo's source of truth and S audits against it. Add a dated
   "(amended 2026-10-09, plan review)" note, as #644, #646 and #663 carry.
3. **#694's scope.** It should also refresh `lib.rs:28-29`, "The modules of slices b to e are empty stubs". After #688,
   no stub is left.
4. **Out of scope, noticed for #646.** ADR-0021:338 gives `pane close` no membership code. As written,
   `close --profile Alpha c3` without `--spec-only` removes Alpha's detached c3 spec and then closes c3's live pane, which
   belongs to Beta. Case 15 holds whichever way #646 decides, because it pins only the scope. Whether the verb's act should
   close another profile's live pane is #646's question, and perhaps one for the ADR.

**What Phase 7 will check:**
- AC7's two greps. The first is literal, so the scope files must not name `PaneInOtherProfile` even in a doc link. The suite
  checks the code string through `expect_code`.
- `check_membership` is `pub(crate)`, remains the only producer of `PaneInOtherProfile` in `src/`, and is called only on
  the `SpecEdit::Set` path. The pane record is read for every edit.
- The scope suite imports `actor`, `sample`, `history`, `shown`, `unchanged`, `pane_name`, `profile_name`, `run_cases`,
  `succeeds`, `expect_code` and `expect_eq`, and keeps no local copy of any of them.
- The three mutants delegate to `FakeProfileScope` and hold no copy of the edit logic.
- The restore hook is taken out of its mutex before it runs (W-8).
- `lib.rs` and `conformance/mod.rs` are untouched, and the ADR diff is the one sentence in section 8 step 1.

The accepted near-duplicates are the per-file mutant harness and the test-local helpers of the two new test files.

## Patterns referenced

- The brief's amendment (`git diff 9fd9c1b 98c250e -- docs/handoffs/688-brief.md`) and the first pass (db4cba8)
- `docs/adr/ADR-0021.md`, lines 66-68, 122-130, 167, 190-192, 281-302, 331-346, 384 and 536-541, and slice d's ADR and
  CHANGELOG edit (316b8e3)
- `crates/holler-pane-testkit/src/{fault.rs, prober.rs, profile_store.rs:100-210, pane_store.rs:218-238, lib.rs}` and
  `src/conformance/{mod.rs, profile_store.rs, profile_store/log.rs}`, plus `tests/profile_store_conformance_test.rs:58-239`
  (the mutant harness)
- `crates/holler-pane/src/profile.rs:328, 380`, `ports.rs:62`, `Cargo.toml:10, 19-30`, `scripts/lint.sh:42-52`
- Issues #663, #644, #646, #683, #638, #688 and #694
