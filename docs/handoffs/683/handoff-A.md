# Handoff-A: Phase 3 - #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-683-implementation (at 6cd60be; base e410e9d = origin/main)
**Brief reviewed:** docs/handoffs/683-brief.md   **Reuse map:** docs/handoffs/683-brief.md, the "Extend vs new" section and the "shared mechanism this slice reuses" evidence block (this run has no survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, because of one ADR finding. Otherwise the plan's structure is sound and reuses what already exists:

- It fills slice a's stubs and does not touch `lib.rs` or `conformance/mod.rs`.
- Every fake method goes through the shared `FaultSwitch<HerdrOp>`, named the way `PaneStoreOp` is.
- The suite reuses `run_cases`, `succeeds`, `expect_code`, `expect_eq` and the single `CASES` table.
- The mutants are wrappers local to the test file, as in slice a.
- Errors are the closed variants plus one `RefusalCode::from_static` constant.
- The test kit still depends on `holler-pane` alone.
- Nothing is duplicated. The only other `HerdrPort` and `Prober` impls are the CLI's production placeholder `Unwired` (wiring.rs:121, 223) and local stubs in `holler-pane`'s own `ports_test.rs`. Neither is a test kit.

**The block:** suite case 2 makes `grid-out-of-range` the answer of `HerdrPort::ensure_pane` for a cell outside the workspace, and #640's real adapter must pass that case. ADR-0021 section 10 defines the code only as "a zero ... or a number above 65535", raised by the `GridPos` guard, and the brief rules out any ADR edit. The decision itself already exists in the #638 amendment of 2026-10-08 and in #683. The fix is a short ADR amendment added to the brief, as #639 and #676 did.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | "Fake behaviour" step 2; suite case 2 `ensure-r1c2-is-grid-out-of-range`; AC 3 `a_workspace_bounds_ensure_pane`; Blast radius "Not changed: ... ADR" | ADRs (cross-cutting: error taxonomy) | ADR-0021 section 10 (line 421) defines `grid-out-of-range` as "a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535". Section 9 (line 377) gives the reason as "The `GridPos` guard declined a row or column outside its bounds". The plan also makes the code `ensure_pane`'s answer for a valid cell that lies outside the declared workspace (`r1c2` in a 2x1 workspace). Case 2 then holds #640's adapter to that. This extends what a closed code means, yet the brief keeps the ADR unchanged and does not mention line 421. The stack rule requires the ADR to be updated in the same change. This repo did exactly that in #639 (section 6 and "Decisions taken" item 7, in 2a6f349) and in #676 (section 9, in 3f9fbf2). | Add `docs/adr/ADR-0021.md` to Files and Blast radius, with an AC that requires two edits. (a) Section 10's `grid-out-of-range` bullet gains "or, from `HerdrPort::ensure_pane`, a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column); the HerdrPort conformance suite pins it (#638 amendment 2026-10-08, grid; #683)". (b) The reason in section 9's class table names both sources. The class (Refusal, exit 3) and the closed list do not change, so `class_of` and its tests are untouched. |
| 2 | warn | Out of scope: "any change to `holler-pane`" | ADRs / contract docs | Two `holler-pane` docs would describe less than the suite enforces. `PaneError::GridOutOfRange` (error.rs:414-416) says "a row or column of zero, or above `u16::MAX` ... (#637, `GridPos`.)". `HerdrPort::ensure_pane` (ports.rs:127-128) says only "or fail loudly". Keeping `holler-pane` out of this slice is right: it is frozen, `HerdrPort` follows the amend-first rule (ADR-0021 section 2), and the port's final form is deferred to #640. | Add a ninth ASSUMPTION at case 2 in `conformance/herdr.rs`: `ensure_pane` outside the workspace is `grid-out-of-range`, and #640 updates both doc lines when it finalizes `HerdrPort`. Put the same item in #640's follow-ups. |
| 3 | warn | `GRID_UNREACHABLE`, `SUPPORTED_VERSIONS`; ASSUMPTION 7 / Decision 4 ("#640 adopts or renames it") | cross-cutting (error codes); dependency direction | Both are test-side copies of values the adapter will own. ADR-0021 section 9 (lines 323-326) says a verb or adapter declares its open code in its own file, and that a merged code is "never renamed". The test kit cannot depend on the adapter, so some copy is unavoidable. But "adopts or renames" invites drift once a verb test pins the literal. Neither value binds #640 today (the suite tests neither), so this is not a block. | Reword ASSUMPTION 7: these are the fake's provisional test vocabulary, raised by no verb or adapter yet. Verb tests (#644) compare against `herdr::GRID_UNREACHABLE.as_str()` and `herdr::SUPPORTED_VERSIONS`, never the literal. #640 then either declares the same values and asserts in its dev-tests that they equal the test kit's, or the fake takes #640's names before any verb story pins them. |
| 4 | warn | Suite case 10 `read-returns-at-most-max-lines` presses `Key::new("Enter")` | pattern consistency (port contract) | The rule on `Key` is "by the name Herdr uses" (ports.rs:100-101); `Enter`/`C-c` there is a stale example. The spike verified Herdr's names as `enter`, `esc` and `ctrl+c` (herdr-api-spike.md:130). #640 runs this suite against a real Herdr. With `Enter`, case 10 would either fail for a reason unrelated to `max_lines` or force #640 into case-folding the port does not ask for. ASSUMPTION 5 is placed only at the fake's `send_keys`. | In case 10, press `Key::new("enter")`. The fake treats `enter` in any case alike, so the fake's results do not change. Repeat ASSUMPTION 5 at case 10. The fake's own tests may keep `Enter` to cover the case-insensitivity. |
| 5 | warn | `FakeHerdr::sent()` / `Sent` ("so a verb test can assert I4") | cross-cutting (I4 guard, ADR-0021 section 4 line 163) | `sent()` holds only sends that reached a pane. An attempt on an unknown or closed pane, or one stopped by a fault, appears only in `faults().calls()`. So a #645 I4 test asserting `sent().is_empty()` would pass a verb that tried to type and failed. ADR-0021's I4 test idea says the fake "records every `send_text`/`send_keys`". | No new mechanism. Add one sentence to the docs of `sent()` and `Sent`: every attempt is in `faults().calls()` as `HerdrOp::SendText`/`SendKeys`, and that is the check for "no keystroke"; `sent()` carries the payloads of the sends that landed. |
| 6 | warn | `tests/herdr_conformance_test.rs`: the guard-lifetime `CaseGuard`, `assert_suite_fails_on`, the `Break`/`Mutant` wrapper | anti-duplication (test scaffolding) | This is the second copy of slice a's scaffolding (pane_store_conformance_test.rs:53-64 and 251-263). Each integration-test file is its own crate and the brief names the pattern, so this copy is accepted and **will not be a Phase 7 rejection**. Slices c (#682) and e (#684) would bring it to three or four copies. | Nothing to change in this slice. If slice c or e copies it again, move `CaseGuard` and an `assert_fails_on(result: Conformance, case)` into `crates/holler-pane-testkit/tests/support/mod.rs`. That follows the `tests/<dir>/mod.rs` layout already used in holler-cli, holler-hub, holler-pane and holler-proto. |

Apart from these findings, the plan matches existing patterns. Checked against the code:

- The trait signatures, `Key`, `PaneId`, `HerdrPane`, `GridPos` (public fields, `Display` as `rRcC`, no `Ord`), `Argv` (`Hash + Eq`), `ProbeResult` (`Clone`) and the cited error variants all match the brief's evidence.
- `RefusalCode::from_static("grid-unreachable")` is valid for a `const`: kebab-case and not a closed code.
- `class_of` treats an open code as a refusal.
- Nothing in the workspace already provides a base-36 or tail-of-screen helper, so the two new private helpers in `herdr.rs` duplicate nothing.
- `FakeProber` keeps its own call log instead of a `FaultSwitch`, and the brief justifies that in writing (Decision 9): the switch logs only a `Copy` op and answers a `PaneError`, while a `Prober` answers a `ProbeResult` and its test needs the arguments.
- `HerdrOp`, `FakeHerdr`, the `herdr.<method>` strings and the test-file names follow the names slice a and `lib.rs` already fixed.
- No `.rs` file is near the 600-line warning or the 900-line limit of `scripts/lint.sh`, and the fallback `tests/herdr_mutants/mod.rs` matches the repo's test-module layout.
- `cargo tree` in AC 5 guards the dependency direction (ADR-0021 section 5: the test kit depends on `holler-pane` only).

## Notes for O

1. **Amend the brief, then start a fresh run** (never `resumeFromRunId`, which replays this verdict):
   - **Files:** "Changed: `CHANGELOG.md`, `docs/adr/ADR-0021.md`".
   - **Blast radius:** add `docs/adr/ADR-0021.md`, and remove "ADR" from the "Not changed" list.
   - **New AC (the ADR):**
     - (a) Section 10's `grid-out-of-range` bullet (line 421) adds the `ensure_pane` workspace-extent condition and cites the #638 amendment of 2026-10-08 (grid) and #683.
     - (b) Section 9's class-table reason (line 377) names both sources: the `GridPos` guard and the Herdr port, for a cell outside the workspace.
     - (c) No change to the class, the closed list, `class_of` or its tests.
     - Optionally, a "Decisions taken" line citing the #638 amendment, as #639 added item 7.
   - **"Decisions already made", bullet 2:** say that it extends ADR-0021 section 10 and that this change records it there.
   - This records a decision the operator already made in #638 and #683. It needs no new ruling. F makes the doc edit in this PR and S checks it against the AC. That follows the repo's practice (#639 and #676 amended ADR-0021 in the same PR) and the stack rule ("updated in the same change").
   - If you read ADR-0021 section 2's amend-first rule as covering any `HerdrPort` semantics, the alternative is a separate `docs(adr)` PR merged before the fresh run. Either way, the ADR must state the rule before the suite holds #640 to it.
2. **Fold warns 2 to 5 into the same amendment.** Each is one sentence in the brief, and the run restarts anyway:
   - ASSUMPTION 9 at case 2.
   - The new wording of ASSUMPTION 7.
   - `Key::new("enter")` and ASSUMPTION 5 at case 10.
   - The note on `sent()` and `faults().calls()`.
3. **Warn 6 is for later.** Record it for slices c and e (#682, #684); this slice needs nothing.

## Patterns referenced

- `crates/holler-pane-testkit/src/fault.rs:17-134`, `src/conformance/mod.rs:43-100`, `src/conformance/pane_store.rs:23-116`, `src/pane_store.rs:20-44 and 128-146`, `tests/pane_store_conformance_test.rs:53-149 and 251-263`: the shared fault switch, runner, assertions, case table and mutation pattern that this slice extends.
- `crates/holler-pane/src/ports.rs:100-148 and 202-221`, `src/error.rs:262-337 and 414-416`, `src/grid.rs:29-73`: the port, `Key`, `Prober`, `RefusalCode`, `class_of`, `GridOutOfRange` and `GridPos`.
- `docs/adr/ADR-0021.md`: section 2 (84-103), section 4 (I4, 163), section 5 (176-186), section 9 (318-326, 377), section 10 (411-430), "Deferred to named stories" and "Decisions taken" (516-553).
- `docs/research/herdr-api-spike.md:44-53, 122-134, 158-163, 226-232, 450-454`.
- `git show 2a6f349 3f9fbf2 -- docs/adr/ADR-0021.md`: #639 and #676 amended ADR-0021 in the same PR as the code.
