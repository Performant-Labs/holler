# Decision journal: #684 (test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites)

## A (Phase 3, up-front plan review) — 2026-10-09T12:49:55-06:00
- **Decided:**
  - PASS with six `warn`s (W-1 to W-6 in `handoff-A.md`); nothing blocks T.
  - The plan extends slice a: it reuses `FaultSwitch` and `enter`, `PortOp::as_str`, `run_cases`, `succeeds`, `expect_code` and `expect_eq`, the `CASES` table, and the `Break`/`Mutant` pattern and naming. It returns closed `PaneError`s only and adds no dependency.
  - Every closed code the suites hold #641 and #642 to is inside its stated meaning in ADR-0021 section 9 and `error.rs` (`pane-not-found`, `session-not-found` and `unavailable` are tagged "(#638-#642.)"; `usage` covers a missing argument). So, unlike #683, no ADR amendment is needed.
  - The two fakes sharing no state is accepted, but must be documented (W-2).
  - The copies of `assert_suite_fails_on` in this slice are accepted. They are consolidated into `tests/support/mod.rs` by one follow-up after #638's last slice merges (W-5). Phase 7 will not reject them.
- **Assumed:**
  - #683 merges the `HerdrFixture<H>` suite shape its brief has at 9d79d0e.
  - The real adapters can meet every suite case. I reasoned through each case against the OpenCode spike and tmux behaviour, but nothing here runs them.
- **Hedged:**
  - W-1 (one suite-fixture shape for slices d and e) is a decision across slices, for O or the operator; my lean is #683's bundle.
  - `unavailable` for "no TUI in the pane" is the closest reading of a closed code in this slice. I judged it inside "a harness cannot be reached".
  - Host case 7 (`stop_owned` of a missing session is `Ok`) narrows #641's "a missing session is a typed error" to `run` and `ps`. I judged that sound, but it binds #641, so it belongs in the PR body (W-3).
- **Evidence:**
  - Read: the brief; issues #684, #638, #633, #641 and #642; ADR-0021 sections 2, 4, 5, 7 and 9 and its "Deferred" and "Decisions taken" lists; `docs/research/opencode-pane-spike.md`; the `title` rows of `herdr-api-spike.md`.
  - Code read: all of `crates/holler-pane-testkit/src` and its slice a tests; `crates/holler-pane/src/{ports,error,argv,pane,probe}.rs`; `scripts/lint.sh`; the workspace lints and `clippy.toml`.
  - Sibling slices: #683's brief and its `handoff-A.md` (its W-6 on scaffolding copies), and #682's brief (its test plan).
  - A standalone `rustc --edition 2021` compile of the brief's `run_cases` fold (`S = (H, HarnessRig)`) built and ran, so the fold needs no change to `run_cases`.
