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
