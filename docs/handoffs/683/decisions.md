# Decisions — #683 the pane test kit, slice d: FakeHerdr, FakeProber and the HerdrPort conformance suite

## A (Phase 3, up-front plan review) — 2026-10-09T12:38:24-06:00
- **Decided:**
  - BLOCK on docs/handoffs/683-brief.md at 6cd60be: 1 block and 5 warns (see handoff-A.md).
  - The plan's structure reuses what exists and passes every shared dimension:
    - slice a's stubs are filled, and `lib.rs` and `conformance/mod.rs` are untouched;
    - `FaultSwitch<HerdrOp>` gives every fault and the call log;
    - the suite runs on `run_cases`, `succeeds`, `expect_code`, `expect_eq` and a single `CASES` table;
    - the mutants are wrappers local to the test file;
    - errors are the closed variants plus one `RefusalCode::from_static` constant;
    - the test kit still depends on `holler-pane` only.
  - The block is ADR-0021 sections 10 and 9. Suite case 2 extends `grid-out-of-range` to a cell outside the declared workspace and binds #640's adapter to it. The brief keeps the ADR unchanged. The stack rule and the repo's precedent (#639 and #676 amended ADR-0021 in the same PR) require the ADR to be amended in the same change.
- **Assumed:**
  - The #638 amendment (2026-10-08, grid and review) and #683's scope text are the operator's decision on what `grid-out-of-range` means for a workspace. The ADR edit only records that decision and needs no new operator ruling.
  - This run has no survey.md, so the brief's "Extend vs new" section is the Reuse map.
- **Hedged:**
  - Section 10 can be read as describing only the parser's refusals, not every source of the code. That reading would make finding 1 a warn.
  - I blocked anyway, for three reasons. Section 10 lists the code's conditions as a closed list. Section 9 names the `GridPos` guard as its source. And the suite would bind #640 to the new meaning through test code alone.
  - In #508, A kept an ADR refinement at warn only because the ADR's wording was ambiguous. This wording is not.
  - Warns 2 to 5 can be folded into the amended brief at little cost. Warn 6 (copied mutation scaffolding) is a follow-up for slices c and e, and is explicitly not a Phase 7 rejection for this slice.
- **Evidence:**
  - Read the brief in full.
  - Read `holler-pane-testkit`: `fault.rs`, `conformance/mod.rs`, `conformance/pane_store.rs`, `pane_store.rs`, `lib.rs`, `Cargo.toml` and `tests/pane_store_conformance_test.rs`.
  - Read `holler-pane`: `ports.rs`, `pane.rs:74-100`, `probe.rs`, `argv.rs:1-60`, `grid.rs:1-90`, and `error.rs:1-120` and `240-520`.
  - Read ADR-0021 sections 2, 4, 5, 9 and 10, plus "Deferred to named stories" and "Decisions taken".
  - Read `herdr-api-spike.md` at the brief's cited lines.
  - Ran `gh issue view` on 683, 638 and 633, and `git show` on 2a6f349 and 3f9fbf2 for ADR-0021.
  - Checked `scripts/lint.sh:44-53` (warn at 600, fail at 900) and the CHANGELOG head (#676 entry ends at line 89).
  - Grepped for other `impl HerdrPort` and `impl Prober` (only the CLI's `Unwired` and `holler-pane`'s local test stubs), other `from_static` uses (`holler-pane` tests only), base-36 and tail-of-screen helpers (none), and the `tests/*/mod.rs` layout.

## A (Phase 3, up-front plan review, round 2 after the brief amendment) — 2026-10-09T12:52:39-06:00
- **Decided:**
  - PASS on docs/handoffs/683-brief.md at 9d79d0e, with 0 blocks and 3 new warns (see handoff-A.md, which this round
    overwrites; round 1 stays in git at a3e47a4).
  - AC 8 resolves round 1's block. ADR-0021 records `ensure_pane`'s workspace-extent `grid-out-of-range` in section 10
    (line 421) and in section 9's class reason (line 377). The class, `ALL_CODES` and `class_of` stay untouched.
  - Warns 2 to 5 are folded in as suggested. Warn 6 is accepted (Decision 11).
  - I checked the whole plan again at e410e9d (`origin/main` has not moved). It reuses the same shared pieces as in round 1:
    - `FaultSwitch<HerdrOp>`;
    - `run_cases` and the `expect_*` helpers, over one `CASES` table;
    - test-local mutants;
    - the closed variants plus one `from_static` code.
  - The test kit still depends on `holler-pane` only.
  - The three new warns:
    - AC 8(c)'s "exactly two lines" conflicts with section 10's wrap at about 125 columns.
    - Section 9's `profile apply` row should go into #640's hand-off.
    - Decision 11's follow-up names slices c and e, which run in parallel and copy the scaffolding too. It becomes one
      cleanup issue after slices b to e merge, combined with #682's W-2.
- **Assumed:**
  - S reads AC 8(c) as two items changed. The section 10 bullet may wrap to the file's width.
  - A re-review overwrites handoff-A.md and adds a round entry here, as the rounds of #508 did.
- **Hedged:**
  - Warn 2 (the `profile apply` row) could be read as part of round 1's ADR block. I kept it at warn. Whether `apply` can
    ask for a cell outside the extent depends on where #640 gets the extent, which is undecided, so editing the row now
    would be a guess.
  - Warn 3 retargets Decision 11's follow-up. It is not a Phase 7 rejection for this slice. That includes a local
    poison-tolerant lock helper in `herdr.rs` or `prober.rs`, since `feed::lock` becomes `pub(crate)` only in #682.
- **Evidence:**
  - Diffed the brief from 6cd60be to 9d79d0e, and read the amended brief in full.
  - Read again in `holler-pane-testkit`:
    - `fault.rs`, `conformance/mod.rs`, `conformance/pane_store.rs:1-130`, `pane_store.rs`, `lib.rs`, `fixture.rs`
      and `Cargo.toml`;
    - the three stubs;
    - `tests/pane_store_conformance_test.rs`.
  - Read again in `holler-pane`: `ports.rs:84-235`, `error.rs:255-345 and 400-500`, `grid.rs:1-80`, `probe.rs`,
    `argv.rs:1-40`, `pane.rs:70-102` and the `lib.rs` re-exports.
  - In ADR-0021, read the header, sections 2-3, 9 and 10, and "Deferred to named stories". Ran `git show 2a6f349 3f9fbf2`
    for the ADR hunks, and measured the line widths of section 10.
  - Ran `gh issue view` on 683, unchanged since 10:48 MDT, and on 638, which has the grid amendment of 2026-10-08.
  - `git ls-remote origin main` is e410e9d, and the only open PR is from dependabot.
  - The sibling worktrees (0681, 0682, 0684) have no edits to ADR-0021 or to the shared files.
  - Grepped their briefs for the copied scaffolding and for `feed::lock`.
  - No test or lint parses ADR-0021's prose. `docs_rows.rs` reads ADR-0003, `docs_cli_test` parses only `holler …`
    commands, and there is no markdown line-length lint.
  - Spike lines 413-454: support is decided by `protocol`, and the version string is what gets recorded.

## T (Phase 4, author the RED tests) — 2026-10-09T12:57:06-06:00
- **Decided:**
  - Wrote three test files (`herdr_conformance_test.rs`, `fake_herdr_test.rs`, `fake_prober_test.rs`) for AC 1 to 4; RED is valid.
  - RED is a build failure of only unresolved imports (E0432), as the brief's test plan fixes it, since the stubs hold no items.
  - Type-checked the tests against throwaway signature-only stubs, then reverted them, so no compile error in the tests hides behind the E0432s.
  - Added four tests beyond the brief's list, each pinning a stated behaviour: the 36th-pane carry, an occupied cell in split-only mode, `print` to an unknown pane, and the unscripted-argv message.
  - Every mutant test names a list of cases (all must appear), so the two-case mutants of AC 2 assert both.
- **Assumed:**
  - `str::lines` semantics for `read` (a trailing empty line survives a double `enter`), and `vanish` never reusing an id, follow from the brief's counter rule; F may read them otherwise and T-green would then repair the test.
  - The crate has no `autotests = false`, so no `[[test]]` entry (the stack note applies to `holler-cli`).
- **Hedged:**
  - The mutants cannot run before the suite exists, so whether each fails on its named case is unproven until T-green.
- **Evidence:**
  - `cargo test -p holler-pane-testkit --no-run`: four E0432 groups, nothing else.
  - With stubs: build clean; 22 `fake_herdr_test` tests failed in the stubs; clippy over the tests did not complete (a stub-only lint), left to T-green.
  - Slice a's two test files: 22 and 10 passed. `bash scripts/lint.sh` exit 0.
