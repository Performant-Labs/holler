# Handoff-A: Phase 3 - #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite  (up-front plan review, round 2)

**Date:** 2026-10-09
**Branch:** issue-683-implementation (at 9d79d0e; base e410e9d = origin/main, which has not moved)
**Brief reviewed:** docs/handoffs/683-brief.md, as amended in 9d79d0e   **Reuse map:** the brief's "Extend vs new" section and its "shared mechanism this slice reuses" evidence block (this run has no survey.md)   **Wireframe:** N/A (no UI surface)
**Round 1:** BLOCK on the brief at 6cd60be (commit a3e47a4): one block (ADR-0021 sections 9 and 10) and five warns.
**Verdict:** PASS

## Summary

PASS. The amended brief resolves round 1's block. AC 8 brings `docs/adr/ADR-0021.md` into the change and records
`ensure_pane`'s workspace-extent `grid-out-of-range` in two places: section 10's bullet (line 421) and section 9's class
reason (line 377). The class, the closed list and `class_of` stay unchanged.

Warns 2 to 5 are folded in as round 1 suggested, and warn 6 is accepted (Decision 11). Checked again against the code at
`e410e9d`, the plan still extends slice a's mechanisms and builds no parallel path.

This round adds three warns, none of them blocking:

- AC 8(c) says "exactly two lines", but the ADR wraps its bullets.
- The `profile apply` row of section 9 should be handed to #640.
- Decision 11's follow-up names two slices that are running in parallel and copying the same code.

## Round 1 findings, as resolved

| # | Round 1 finding | Where the amended brief resolves it | Status |
|---|---|---|---|
| 1 | block: ADR-0021 sections 9 and 10 not amended | AC 8 (a) to (c); Files; Blast radius; "Decisions already made" bullet 2; Decision 10 | resolved |
| 2 | warn: two `holler-pane` docs describe less than the suite enforces | ASSUMPTION 9 at suite case 2, handed to #640 | resolved |
| 3 | warn: `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` are test-side copies | ASSUMPTION 7 reworded; doc comments on both constants; Decision 4 | resolved |
| 4 | warn: case 10 pressed `Enter` | cases 9 and 10 press `Key::new("enter")`; ASSUMPTION 5 repeated at case 10; the note under the case table | resolved |
| 5 | warn: `sent()` cannot prove I4 | `Sent` and `sent()` docs; "Fake behaviour"; AC 3 adds a send stopped by `fail_next` | resolved |
| 6 | warn: copied mutation scaffolding | Decision 11 | accepted; see warn 3 below |

## Findings (round 2)

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | AC 8 (a) and (c) | pattern consistency (ADR format) | **Line wrap.** AC 8(c) requires `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` to show "exactly those two lines changed". Section 10's bullets are wrapped at about 125 columns, with two-space continuation lines (lines 415-429). The example wording in (a) is 306 characters. F must therefore either leave one 306-character line in a wrapped section, or re-wrap the bullet over three lines, which fails a literal reading of (c). The table row of (b) is one line either way. **Undefined extent.** The new bullet states a rule over "its workspace's rows and columns", but Herdr has no grid (spike 47-50). Where a workspace's extent comes from is #640's to decide (ASSUMPTION 1, and "Deferred to named stories": `HerdrPort` in its final form, #640). | F re-wraps the section 10 bullet to the width of its neighbours. S reads (c) as two items changed: the section 10 `grid-out-of-range` bullet, continuation lines included, and the section 9 table row. Optionally, the same bullet adds "(how the adapter learns a workspace's extent is #640's)". No other ADR line changes. |
| 2 | warn | AC 8 (c) "no other ADR line is edited"; ASSUMPTION 9's hand-off to #640 | ADRs (section 9, failure modes by verb) | After AC 8, a verb that reaches `ensure_pane` can answer `grid-out-of-range` for a cell outside the workspace. Section 9's `pane launch`/`relaunch` row (line 335) already lists the code. The `profile apply` row (line 346, #664) does not, although `apply` ensures each spec's cell and the row already lists two codes that ports raise (`probe-failed`, `herdr-version-unsupported`). Whether `apply` can ask for a cell outside the extent depends on where #640 gets a workspace's extent (ASSUMPTION 1). If the extent comes from the profile, it never can. If it comes from configuration, it can. Editing the row now would guess #640's answer, so keeping AC 8 to two items is right. | Hand the row to #640 together with the two `holler-pane` doc lines. The ASSUMPTION 9 comment at case 2 also names section 9's `profile apply` row, and any other verb that reaches `ensure_pane`, as something #640 updates when it decides where the extent comes from. O adds the same item to #640's follow-ups. |
| 3 | warn | Decision 11: "A later slice (#682, #684) that would copy them again moves `CaseGuard` and an `assert_fails_on` ... into `tests/support/mod.rs` instead" | anti-duplication (test scaffolding, crate-private helpers) | **Copied scaffolding.** Slices c and e are not "later". They are running now from the same base, `e410e9d` (worktrees `0682-profile-fakes` and `0684-host-harness`), and both plans have passed review (9e661ec, ccf0a28). Both briefs copy `assert_suite_fails_on` and the `Break`/`Mutant` pattern (682-brief.md:305, 684-brief.md:487), and neither plans `tests/support/mod.rs`. So Decision 11's move will not happen in either slice, and once slices b to e merge there will be four copies. **Lock helper.** #682 makes `feed::lock` `pub(crate)` (682-brief.md:213). This slice is on the same base and `feed.rs` is outside its blast radius, so it writes its own poison-tolerant lock in `herdr.rs` and `prober.rs`, using the idiom the brief cites (`fault.rs:105-107`). Neither is a Phase 7 rejection for this slice. | No change to this slice's code. Replace Decision 11's "later slice" with one cleanup issue that O opens, to run after slices b to e have merged. Combine it with #682's plan-review follow-up (its W-2: move the shared suite helpers into `conformance/mod.rs`). Scope: move `CaseGuard` and `assert_fails_on(result: Conformance, case: &str)` into `crates/holler-pane-testkit/tests/support/mod.rs`, used by every conformance test file; `herdr.rs` and `prober.rs` call `crate::feed::lock`. |

Apart from these, the plan matches existing patterns. Checked again this round:

- `origin/main` is still `e410e9d`, and the code the brief cites is unchanged.
- Every signature in the brief's evidence block matches the code: `HerdrPort`, `HerdrSpec`, `HerdrSnapshot`, `Key`, `Prober`,
  `ProbeResult`, `Argv` (`Hash + Eq`), `PaneId`, `HerdrPane`, `GridPos`, the cited `PaneError` variants,
  `RefusalCode::from_static` and `class_of`. Each is reachable at the path the brief uses (`holler_pane::*` re-exports,
  `holler_pane::error::*`).
- The suite fits the shared runner. `run_cases(&CASES, fresh, |case, fixture| case(&view))` takes `HerdrFixture<H>` as the
  subject and drops it before the guard. `Case` is a `Copy` fn pointer over the crate-private view.
- No sibling slice changes ADR-0021 or any file of this slice. Slices b, c and e share only `CHANGELOG.md` with it, so
  AC 8 cannot collide with them.
- No test or lint parses ADR-0021's prose. `docs_rows.rs` reads ADR-0003, `docs_cli_test` parses only `holler …`
  commands, and there is no markdown line-length lint.
- `version()` answering the build string (`PROTOCOL_22_VERSION`) matches the spike. Support is decided by the integer
  `protocol`, and the version string is what gets recorded as `host.herdr_api_version` (herdr-api-spike.md:413-426, 450-454).
- Round 1's other checks still hold:
  - Every fake method goes through `FaultSwitch<HerdrOp>`, and `herdr.<method>` mirrors `pane_store.<method>`.
  - The suite reuses `run_cases`, `succeeds`, `expect_code` and `expect_eq` from a single `CASES` table.
  - The mutants are wrappers local to the test file.
  - Errors are closed variants plus one `from_static` code.
  - `FakeProber` keeps its own call log, as Decision 9 justifies.
  - No new helper duplicates an existing one: nothing in the workspace already does base 36 or a screen's last lines.
  - No file comes near 600 lines.
  - AC 5's `cargo tree` guards the rule that the test kit depends on `holler-pane` only.

## Notes for O

The verdict is PASS, so nothing here blocks the run. Notes for F, S and O:

1. F re-wraps the section 10 bullet to the width of its neighbours. S checks AC 8(c) as two items (warn 1).
2. F may add the `profile apply` row to the ASSUMPTION 9 comment it writes at case 2. Either way, O adds this to #640's
   follow-ups: update ADR-0021 section 9's `profile apply` row if `apply` can ask for a cell outside the extent (warn 2).
3. O opens the combined cleanup issue to run after slices b to e merge, and records in decisions.md that this issue
   replaces Decision 11's "later slice" (warn 3).
4. At Phase 7, A will not reject the copied `CaseGuard` and `assert_suite_fails_on`, or a local poison-tolerant lock
   helper in `herdr.rs` or `prober.rs`.

## Patterns referenced

- The shared mechanisms this slice extends, in `crates/holler-pane-testkit/`:
  - `src/fault.rs:17-134`
  - `src/conformance/mod.rs:43-100`
  - `src/conformance/pane_store.rs:23-116`
  - `src/pane_store.rs:20-44 and 128-146`
  - `tests/pane_store_conformance_test.rs:53-149 and 251-263`
- The port and its types, in `crates/holler-pane/`: `src/ports.rs:84-235`, `src/error.rs:255-345 and 400-500`,
  `src/grid.rs:29-73`, `src/lib.rs:62-76`.
- `docs/adr/ADR-0021.md`:
  - sections 2-3 (84-155)
  - section 9 (304-410; rows 335, 346, 377)
  - section 10 (411-430)
  - "Deferred to named stories" (516-528)
  - `git show 2a6f349 3f9fbf2 -- docs/adr/ADR-0021.md`
- `docs/research/herdr-api-spike.md:413-454`.
- The sibling plans, read from their worktrees:
  - `docs/handoffs/682-brief.md:213, 305` and `682/handoff-A.md` (W-2)
  - `docs/handoffs/684-brief.md:487`
