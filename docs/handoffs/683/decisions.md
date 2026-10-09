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
