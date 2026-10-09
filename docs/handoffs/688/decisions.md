# Decisions — #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite

## A (Phase 3, up-front plan review) — 2026-10-09T14:25:44-06:00
- **Decided:** BLOCK on docs/handoffs/688-brief.md at 9fd9c1b, with one block and six warns (see handoff-A.md).
  - **The block (B-1):** the fake's `edit_spec` step 3 runs the `pane-in-other-profile` check for every edit, `Remove`
    included. `Remove` is `pane close`, and ADR-0021:338 gives `pane close` no `pane-in-other-profile`. ADR-0021:282-283
    says a detached spec is not refused, yet the brief's own fixture holds one (Alpha's c3 entry) that `edit_spec` could
    then never remove. The fix is to run the check for `Set` only, and it changes which edit case 13 and AC4 must use. So it
    belongs in the brief before T writes tests, not in a warn for F.
  - **The rest of the plan extends the right objects.** It composes the two existing fakes as `Arc<dyn …>` ports, reuses
    `check_membership`, `run_cases` and part 1's suite helpers by visibility changes only, and leaves `lib.rs`,
    `conformance/mod.rs` and every manifest untouched. It follows ADR-0021 section 8's I8 order, including "Decisions
    taken" item 1.
- **Assumed:**
  - The brief's "Evidence", "Reuse refactors" and "Reuse map" sections are the Reuse map, because this run has no separate
    survey.md.
  - ADR-0021 section 9's per-verb table, line 338, is binding on what `pane close --profile` may answer, because the
    section says every verb's codes beyond the common ones are listed there.
  - The I8 order's "Record" step is the verb's, inside its act. That is the only reading the frozen
    `act: FnMut() -> Result<(), _>` allows, and the brief flags it for #663.
- **Hedged:**
  - **W-1, the case 14 pre-check, is a warn and not a second block.** It extends ADR-0021 section 8, which places the
    check in the registry's compare-and-swap. Precedent is mixed: part 1's A let an ADR-silent rule live in the suite's
    module doc, while slice d amended ADR-0021 in its own PR (316b8e3). I recommend O fold the one-sentence ADR edit into
    B-1's amendment.
  - **W-4:** `profile-conflict` cannot be reached from a verb test through the stores alone. That is a forward-compat gap,
    not drift, so it is a warn.
  - **W-5:** what the fake does when a restore fails with something other than a conflict is left for #663 to decide.
- **Evidence:**
  - Read the brief in full, issues #688, #663 and #638, and the whole of `crates/holler-pane-testkit` that the plan
    touches or reuses:
    - `src/profile_store.rs`, `pane_store.rs`, `fault.rs`, `fixture.rs` and `prober.rs`;
    - `src/conformance/mod.rs`, `pane_store.rs` (lines 1-60 and 400-534), `profile_store.rs`, `profile_store/log.rs`,
      `watch.rs` (lines 1-60), and the herdr and harness runners;
    - `tests/profile_store_conformance_test.rs` and the headers of every test file.
  - Read `holler-pane/src/profile.rs`, `error.rs:395-470` and `540-690`, `pane.rs:34-47`, `ports.rs:215-235`, and
    `holler-cli/src/pane/wiring.rs`.
  - Read ADR-0021 sections 4, 5, 8 and 9 and "Decisions taken".
  - Ran `wc -l` on every crate file, grepped for every `ProfileScope` impl and every membership rule in the workspace, and
    read the workspace lints and `scripts/lint.sh`.
  - Checked the git history of part 1's Phase 3 review and journal (`95c44d2^:docs/handoffs/682/`) and of slice d's ADR
    edit (316b8e3).
  - Ran `gh issue list` (no follow-up for part 1's W-2) and `gh pr list` (no open PR touches the test kit).

## A (Phase 3, up-front plan review, re-review after the amendment) — 2026-10-09T14:41:53-06:00
- **Decided:** PASS on docs/handoffs/688-brief.md as amended at 98c250e, with no block and two new warns, W-7 and W-8
  (see handoff-A.md, which replaces the BLOCK pass of db4cba8).
  - **B-1 is fixed as asked.** The membership check is `Set`-only, case 15 pins a `Remove` of a detached spec, and case
    13 and AC4 name a `Set`. The `MembershipOnRemove` mutant shows the new case catches the old behaviour.
  - **W-1 to W-6 and the minor point are all addressed.** W-6 is #694. The `before_next_restore` hook is the narrowest way
    to reach `profile-conflict` from a verb test, because `FaultSwitch` can only fail calls. It is not a second fault path.
  - **W-7:** the scope cannot see that a `--spec-only` act is empty, so the `Set` check refuses `--spec-only` too. The new
    ADR sentence should say so.
  - **W-8:** the restore hook must run after its mutex is released (edition 2021 keeps an `if let` guard alive through
    the block). Its doc should say it fires only on the restore path of `edit_spec` with a profile.
- **Assumed:**
  - #663's "`--spec-only` makes `edit_spec` skip the act" means the verb passes an empty act, since `edit_spec` has no flag
    for it.
  - ADR-0021:282-283's "a detached spec is not refused" is about the stores, because the paragraph is about `pane/cas_put`
    and the two registries. So the `Set` check does not contradict it.
- **Hedged:**
  - **W-7 is a warn, not a block.** Section 9 line 335 allows `pane-in-other-profile` for launch and relaunch with no
    `--spec-only` exception, the choice fails closed, and the ADR edit writes it down. Whether `--spec-only` may write a
    detached spec is an open product point for #644 and the operator, and the brief's Risks rule keeps it reversible.
  - **W-8 is a warn.** The deadlock needs a hook that re-arms or calls back into the scope, and no planned test does that.
  - **Noticed for #646, not this run's:** ADR-0021:338 gives `pane close` no membership code. So `close --profile P X`
    for X in another profile would close that profile's live pane.
- **Evidence:**
  - Ran `git diff 9fd9c1b HEAD -- docs/handoffs/688-brief.md`, and read the amended brief in full and the first pass's
    handoff and journal.
  - Re-read ADR-0021 lines 60-195, 255-350, 366-400 and 520-555, and 316b8e3's ADR and CHANGELOG diff.
  - Read these testkit files:
    - `fault.rs` in full, `prober.rs:1-60`, `profile_store.rs:100-210` and `pane_store.rs:205-238`;
    - `conformance/mod.rs:25-137`, `conformance/profile_store.rs` (the header and the helpers) and
      `profile_store/log.rs:1-20`;
    - `tests/profile_store_conformance_test.rs:50-239`.
  - Grepped the testkit sources for hooks, closures, mutex patterns and `PaneInOtherProfile`. Grepped the three port
    traits for `Send + Sync`. Checked `Cargo.toml` for the edition and the lints, and `scripts/lint.sh` for the size gates.
  - Ran `gh issue view` for 663, 644, 646, 683, 638, 688 and 694, and searched `gh issue list` for spec-only and for
    recent issues.
  - Ran `gh pr list`: the only open PR is dependabot #673, and nothing in it is in the radius. Ran `git ls-remote origin`:
    main is still 9d61c9f.

## T (phase 4, author/RED)

- **Decided:**
  - The RED is the unresolved import of `FakeProfileScope`, `profile_scope_cases` and `run_profile_scope_conformance`;
    nothing else could fail first for a test kit whose API does not exist.
  - Three tests beyond the brief's AC4 list (hook stays armed across a profile-less failed act, a conflicting first write
    and a succeeding act; re-arming replaces; re-arming from inside the hook does not deadlock), all following from the
    brief's Public API doc and A's W-8.
  - The `MembershipOnRemove` mutant test asserts only case 15, as the brief says.
- **Assumed:**
  - "Armed until a failed act of an edit with a profile" (A's W-8 wording) is the intended hook contract. The brief's step 7
    is under the `Some(P)` path only, and `edit_spec(None)` "calls only act()".
  - `sample_profile("Demo Alpha", &[C4])` is a valid "other writer's version" (the spec set differs from every state the
    scope writes).
- **Hedged:**
  - The mutants' targeted cases are checked by reading, not by running, until F's suite exists. T-green re-runs them.
  - The deadlock test waits 10 s only when it fails; it adds no time when it passes.
- **Evidence:** `cargo test -p holler-pane-testkit --no-fail-fast` (two E0432 build failures); the reverted probe runs
  recorded in handoff-T-red.md.

## F (phase 6 of the script, implementation) — 2026-10-09T15:04:34-06:00
- **Decided:**
  - `FakeProfileScope` follows the brief's "Fake behaviour" in order: get P, then the `usage` guard for a `Set`, then
    the pane-record `get` for every edit, then `check_membership` for a `Set` only, then `cas_put` at g, then the act.
    A failed act runs the one-shot hook and then the restore `cas_put` at g + 1. `Conflict` there becomes
    `profile-conflict`, and any other error is returned as it is.
  - The hook is taken out of its mutex in a `let` statement of its own before it runs (W-8). A replaced hook is dropped
    after the lock is released. The mutex is read through `crate::feed::lock`, reused rather than restated.
  - The suite is `Seeded<S>` (owned per case) plus `Bench<'a>` (the `&dyn ProfileScope` view a case gets). Cases 1 to 8
    are in `conformance/profile_scope.rs` and cases 9 to 15 in `conformance/profile_scope/act.rs`. The log checks read
    only what the log gained (`gained`), because the suite pins no actor.
  - The ADR sentence includes A's W-7 clause (with or without `--spec-only`) and `(#688)`. It is one sentence in
    section 8, step 1, mirrored in the suite's `ASSUMPTION (#661/#663)` paragraph and in the fake's comment.
  - `CREATED` stays private, because the scope suite does not read it. Every other helper and constant the brief listed
    is `pub(super)` and reused.
- **Assumed:**
  - A `Remove` that drops every entry naming the pane (`retain`) equals the brief's "drops it" for a well-formed
    profile, which has one entry per pane. No case or test can tell the two apart.
  - Checking `profile-conflict`'s `Display` for the profile name is the general form of "its message contains
    `Demo Alpha`", since `Display` is `profile conflict: {what}`.
- **Hedged:**
  - Case 12 pins only the profile store's calls, as the brief's table does, though the fake also makes no pane call.
    Tightening it to the pane store would bind #663 beyond the decided assumption, so I left that to #663.
  - Issue #688's body is still the pre-review text (14 cases, 2 mutants, about 1,020 lines). Amending it is O's job (A's
    note 2), and I flagged it for S.
- **Evidence:**
  - Every new target is GREEN (18 + 6), and `cargo test -p holler-pane-testkit` has every target ok. Clippy
    (`--workspace --all-targets -D warnings`) is clean, and `rustfmt --check`, `lint.sh`, `changelog-check.sh`,
    `cargo machete` and `test-hooks.sh` pass. The AC6 and AC7 commands print nothing.
  - A throwaway mutant probe, deleted afterwards, showed each of T's three mutants failing on its named case for the
    right reason. Two extra mutants failed too: retrying the first write fails case 10, and swallowing the act's error
    fails case 9.
  - Source facts are in `docs/handoffs/688/evidence.md`.

## T (phase 7, verify GREEN + Tier 2)
- **Decided:** PASS. Suite GREEN, Tier 1 matches F's report, no test repaired, no production change requested.
- **Assumed:** F's AC6-AC8 git/grep outputs are accurate; I re-ran the workspace-level guards (clippy, test, lint, changelog, machete, docs_cli, wire_selftest) rather than each grep.
- **Hedged:** Ran the two new targets 5 extra times to look for flakes; none.
- **Evidence:** a no-op restore mutation failed 6 of 18 `fake_profile_scope_test` tests; reverted, tree clean. Details in handoff-T-green.md.

## A (Phase 7, anti-duplication gate) — 2026-10-09T15:15:38-06:00
- **Decided:** PASS on the diff 9d61c9f..05d7262, with no block and two warns (see handoff-A-dup.md).
  - **F extended the stubs and built no parallel path.** It composes the two fakes through their ports and reuses
    `check_membership`, which stays the only `PaneInOtherProfile` producer in `src/` and is called on the `Set` path
    only. It also reuses `feed::lock`, part 1's suite helpers, constants and `Step`, slice a's name helpers, and the
    shared runner and expectation helpers. The three mutants delegate to the fake.
  - **The new objects are the ones the brief justifies.** `with_edit`, `belongs` and the restore step have no equivalent
    in the workspace. The only other spec-edit applier is `holler-pane`'s test-local `MemScope`, which the brief
    excludes. `belongs` answers a different question from `check_membership`.
  - **W-1:** `lib.rs:28-29` still calls the slices' modules empty stubs, and #694, which Phase 3 asked to own it, does
    not list it.
  - **W-2:** `changed_spec` makes the same change as part 1's `revised` (a spec's `context.soft`). The brief planned
    it, so it is a candidate for #694, not this run's work.
- **Assumed:**
  - The brief's "Evidence", "Reuse refactors" and "Reuse map" sections are the Reuse map, as at Phase 3.
  - Test-local helpers in the two new integration test files are the accepted near-duplicates of Phase 3. There is no
    `tests/common`, and integration tests cannot reach `pub(super)` items.
- **Hedged:**
  - **W-2 is a warn, not a block.** The brief named the helper as new, the two helpers act at different levels (a bare
    spec, a profile's first spec), and nothing couples them.
  - **The per-suite code constants are not flagged.** `CONFLICT` and `PROFILE_NOT_FOUND` repeat part 1's private ones,
    but the pane, profile and harness suites each keep their own, so that is the dominant pattern.
  - **Runtime is T's.** I did not re-run the build or the tests, since T-green verified 05d7262 and the tree is clean.
- **Evidence:**
  - Read `git diff origin/main...HEAD`, every changed file in full, and F's, T's and Phase 3's handoffs.
  - Read the whole of these, for comparison:
    - the profile and pane store suites (with `log.rs` and `watch.rs`) and both fakes;
    - `fixture.rs`, `fault.rs`, `prober.rs`, `feed::lock` and `conformance/mod.rs`;
    - the herdr and harness suites' runners;
    - `tests/profile_store_conformance_test.rs`, and the threaded tests in the two fake-store test files.
  - Grepped the workspace for `SpecEdit`, `PaneNotInProfile`, `ProfileConflict` and `PaneInOtherProfile`, for slug
    comparisons and membership predicates, and for spec mutators and `panes.retain`. Read `holler-pane`'s `Profile` and
    `Pane` methods, `MemScope`, the CLI's `Unwired` scope and the hub's `check_membership` stub.
  - Re-ran the AC6, AC7 and AC8 checks (each clean) and a scan for panics and `#[allow]` in the new `src/` code.
  - Scanned the added lines for personal or infrastructure names (none) and checked the seven branch commits' identities
    (all GitHub no-reply).
  - Ran `gh issue view 694`.

## S (Phase 8, spec audit) — 2026-10-09T15:27:37-06:00
- **Decided:** PASS on 9d61c9f..7114c33 (see handoff-S.md).
  - Every clause of the issue and AC1 to AC10 of the brief has a proving test or a check I verified. The 15 case ids
    match the brief one for one, and I traced the three mutants through their named cases by reading.
  - The fake follows the brief's "Fake behaviour" step for step. The suite asserts each row of the case table. No case
    passes against a do-nothing scope.
  - F's four deviations are documented and sanctioned. The ADR sentence carries the W-7 clause that A asked for.
  - Seven advisory notes, none blocking. N-1 (amend #688's stale body) should be done before the merge.
- **Assumed:**
  - T-green's Tier 1 output (clippy, workspace tests, lint, changelog-check, machete) and F's (build, rustfmt,
    test-hooks, `cargo tree`) are accurate. By role I did not re-run them. No `crates/` file changed after F (628bcab) and
    no test after T-red (b0bf1ee), so T-green's run at 05d7262 covers HEAD.
  - The script's handoff cleanup removes `docs/handoffs/688*` before the push, as it did for #682 (95c44d2). That is why
    N-1 matters.
- **Hedged:**
  - **The over-delivery against the issue body is not a REWORK or a HOLD.** The issue body still says 14 cases, 2
    mutants and about 1,020 lines. The run delivers those 14 case ids and both mutants, plus case 15, a third mutant, the
    hook and the ADR sentence. Each addition is a documented plan-review fix (B-1, W-1, W-4, W-7), and none is unrelated
    work. Only the issue's text is stale, and amending it is O's job (N-1).
  - **The `--spec-only` clause in the ADR is accepted as descriptive.** The scope cannot see an empty act, so case 14
    already implies the refusal. #644 inherits it, and the brief's Risks rule keeps it reversible (N-5).
  - **N-4 and N-6 are test-strength notes, not REWORK.** In N-4 the generation and the exact error already tell the
    behaviours apart. In N-6 the brief's table deliberately leaves the pane store out of case 12.
- **Evidence:**
  - Read the issue, #663, #694, the amended brief, all six handoffs and `evidence.md`, the whole diff (src, tests, ADR and
    CHANGELOG), and the #682 appendix at `95c44d2^:docs/handoffs/682-brief.md:489-565`.
  - Ran these checks:
    - `git diff --quiet` on the manifests and `Cargo.lock`;
    - AC7's two greps;
    - `git diff --name-only` (AC5 and AC8);
    - `wc -l` on the eight touched `.rs` files (max 535);
    - a scan of the added `src/` lines for panics and `#[allow]`;
    - a privacy grep of every added line;
    - `git log` for the subjects, trailers and identities, against `.githooks/commit-msg` and `prepare-commit-msg`;
    - `gh pr list` (no PR yet).
  - Read these to check facts the suite relies on:
    - ADR-0021 section 8 and section 9's table;
    - `profile.rs:255-275`;
    - `ports.rs` (`list`);
    - `feed.rs` (the `BTreeMap` records);
    - `fixture.rs`;
    - `conformance/mod.rs`;
    - part 1's `shown` and `unchanged`;
    - the `lints` table in the test kit's `Cargo.toml`;
    - `lib.rs:15-29`;
    - CONTRIBUTING.md.
