# Handoff-A-dup: Phase 7 - #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-683-implementation
**Diff base:** e410e9d (origin/main and the merge base; main has not moved)   **Diff head:** bbe4d89
**Reuse map:** docs/handoffs/683-brief.md, "Extend vs new" and the "shared mechanism this slice reuses" evidence block (this run has no survey.md)
**Verdict:** PASS

## Summary

PASS. F extended slice a's mechanisms and built no parallel path:

- **Faults.** `FakeHerdr` holds one `FaultSwitch<HerdrOp>`, and all seven port methods call `faults.enter(op)` first
  (`herdr.rs:247, 260, 271, 289, 294, 303, 314`). `HerdrOp` implements the merged `PortOp` with `herdr.<method>` names,
  mirroring `PaneStoreOp`. Neither fake builds a `Timeout` of its own: the wedged one comes from the switch alone.
- **Suite.** The suite runs on the shared `run_cases` (`conformance/herdr.rs:145`) over one `CASES` table that
  `herdr_cases()` lists. Every case checks through `succeeds`, `expect_code` and `expect_eq` from `conformance/mod.rs`, which
  is unchanged.
- **Errors.** Every error is a closed `PaneError` variant. The one open code, `GRID_UNREACHABLE`, is built with
  `RefusalCode::from_static`, the mechanism ADR-0021 section 9 prescribes.
- **Formatting and keys.** Cells are written with `GridPos`'s own `Display`, and `FakeProber` is keyed on `Argv`'s own
  `Hash`/`Eq`.
- **Scope.** The three stubs were filled in place. `lib.rs`, `conformance/mod.rs`, `feed.rs` and `fixture.rs` are untouched.

Each new object or record that might look like a second path was justified in writing in the brief and reviewed at Phase 3:

- the `BTreeMap` pane store instead of `feed.rs`, because `HerdrPort` has no `watch`;
- `FakeProber`'s own call log instead of a `FaultSwitch` (Decision 9);
- `Sent` beside `faults().calls()`, because it records payloads where the log records attempts (round 1 warn 5);
- `HerdrFixture` (Decision 2).

A search of the workspace found no other in-memory `HerdrPort` or `Prober` that these fakes could have extended.

All three findings are `warn`. None is new drift. Two are follow-ups that Phase 3 handed to O, and neither has been filed:
no test-kit cleanup issue exists, and #640 carries none of this slice's hand-off. The third is a cross-slice shape question
that #684's gate raised.

## Findings

No block. Nothing in the diff builds a parallel path to an object the map named.

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `tests/herdr_conformance_test.rs:77-87, 385-398`; `tests/fake_herdr_test.rs:56-58`; `src/herdr.rs:238-242`; `src/prober.rs:68-72`; `src/conformance/herdr.rs:40` | **Copied scaffolding, accepted before the code existed** (Phase 3 round 2 warn 3 and Note 4; brief Decision 11). It is now in the code: (a) `CaseGuard` copies `pane_store_conformance_test.rs:53-64` verbatim; (b) `assert_suite_fails_on` copies `:251-263` and widens `case: &str` to `cases: &[&str]`; (c) `timeout(op)` repeats `fake_pane_store_test.rs:24-26`; (d) each fake has its own poison-tolerant `lock()`, while #682 makes `feed::lock` `pub(crate)` (its `feed.rs:243`); (e) `NOT_FOUND` repeats `conformance/pane_store.rs:30`. The brief's no-edit rule on `lib.rs`, `conformance/mod.rs` and `feed.rs` forces each copy, and none is longer than 12 lines. A search of the issues found no cleanup issue (#684's gate reports the same). After slices b to e merge, the crate holds five copies of the mutation assertion and five per-struct `lock()` helpers (fault, herdr, prober, host, harness) beside the shared `feed::lock`. | No change in this slice. Add these items to the **one** test-kit cleanup issue that #684's gate scoped (its handoff-A-dup, "Notes for O"), not to a new issue. That issue also needs one requirement from this slice: the shared `assert_fails_on` must handle a mutant that fails two named cases (#683's `Transposes` and `CloseIsIdempotent`). It can take `&[&str]`, as here, or take `&Conformance` so that a test calls it once per case. |
| 2 | warn | `src/conformance/herdr.rs:57-64, 140-143` (against #684's `src/conformance/harness.rs:53-74, 142-155`) | **Two shapes for one idea, across slices.** A suite's per-case fixture is `HerdrFixture<H> { port, session, workspace }` with a 2-tuple `fresh: FnMut() -> (HerdrFixture<H>, K)` here, and `HarnessRig` with a 3-tuple `fresh` in #684. The ASSUMPTION prefixes also differ: `ASSUMPTION (#640):` here, `(#642 to confirm):` there. #684's gate raised this as its finding 1, and it is recorded here so that this PR carries it too. It is not drift in this diff: the brief's exact API names `HerdrFixture` and justifies it (Decision 2), neither shape is on main, and this shape keeps slice a's 2-tuple `fresh` and its `run_cases` call unchanged. | No change in this slice. Settle it in the same cleanup issue **before #640 or #642 writes its suite runner**, because after that a change breaks an adapter's tests. Either pick one shape (#684's gate notes that this slice's is closest to slice a's call), or record both as deliberate in `conformance/mod.rs`'s module doc. |
| 3 | warn | `src/herdr.rs:26, 32, 42, 187, 190, 272, 295, 318, 358`; `src/conformance/herdr.rs:49, 52, 189, 337` | **The hand-off to #640 exists only in code comments.** There are thirteen `ASSUMPTION (#640)` blocks, covering the brief's nine assumptions. `gh issue view 640` (its body, no comments) mentions neither #683 nor any of these items: (a) the two `holler-pane` doc lines that ASSUMPTION 9 hands over (`error.rs:414-416`, the `GridOutOfRange` doc; `ports.rs:127-128`, the `ensure_pane` doc); (b) ADR-0021 section 9's `profile apply` row (Phase 3 round 2 warn 2, Note 2: "O adds this to #640's follow-ups"); (c) ASSUMPTION 7's choice for `grid-unreachable` and `SUPPORTED_VERSIONS`. #640's implementer meets the suite's cases by running them, but no failing test surfaces (a) to (c). | When the PR is opened, list what binds #640 in the PR body, as #684's gate asks for #641 and #642, and post the same list as one comment on #640: the suite to pass (`run_herdr_conformance`, 11 cases, an opt-in scratch session); the fixture contract (a workspace of 2 rows by 1 column whose extent the adapter knows, empty or holding only its root at `r1c1`); the thirteen ASSUMPTION comments by file and line; and items (a) to (c). |

No architectural drift was introduced during rework. There was no rework: T-green passed on F's first implementation, and
this is the first Phase 7 pass.

Checked and found clean (the evidence for the PASS):

- **Placement and scope.** `git diff --name-only e410e9d...bbe4d89` lists only the brief's blast radius:
  - the three stub files, filled;
  - three new test files;
  - `CHANGELOG.md`;
  - `docs/adr/ADR-0021.md`;
  - `docs/handoffs/683*`.

  Not changed: `lib.rs`, `conformance/mod.rs`, `feed.rs`, `fixture.rs`, `pane_store.rs`, any manifest, `Cargo.lock`, any
  other crate, any golden or protocol file. `cargo tree -p holler-pane-testkit -e normal` names no `holler-cli`,
  `holler-hub` or adapter crate.
- **No other fake to extend.** The workspace has three other `HerdrPort`/`Prober` implementations, and none is an analogue:
  - `Unwired` (`holler-cli/src/pane/wiring.rs:121, 223`) answers `not-implemented`. Its own doc says the test kit's fakes
    replace it in the verb harness later.
  - `TestHerdr` and `FailingProber` (`holler-pane/tests/ports_test.rs:167, 246`) return constants to pin #637's
    signatures. That crate cannot use the test kit, because the test kit depends on it.
  - `SystemProber` (`holler-pane/src/ports.rs:218`) is the real runner's stub.

  No sibling worktree (0681, 0682, 0684) touches `herdr.rs`, `prober.rs` or `conformance/herdr.rs`, and none builds a probe
  fake or a scripted-result table of its own (#684's `prober.rs` is still the stub). The only path shared with a sibling is
  `CHANGELOG.md`.
- **One path per rule, within the diff.**
  - `close` and `vanish` share `State::remove`, as `FakePaneStore::delete` and `concurrent_delete` share `remove`.
  - `send_text`, `send_keys`, `read` and `print` share `State::pane_mut`.
  - `ensure_pane` is a lookup (`State::workspace_mut`) plus `Workspace::place`, which calls `check_range`, `occupant`,
    `check_split` and `mint` once each.
  - `pane-not-found` has one builder (`not_found`).
  - `vanish` and `print` bypass the faults and the call log, as `FakePaneStore::concurrent_*` do.
- **New helpers duplicate nothing reachable.**
  - The brief names `base36` and `last_lines`. A search found no radix-36 or tail-of-screen helper anywhere.
  - `count` and `not_found` are private message builders. `holler-pane` exposes no per-code constant (`PaneCode` is
    `pub(crate)`) and no error builder. The `count(hay, needle)` helpers in `holler-cli` tests are unrelated and out of
    reach by dependency direction.
  - `GridPos` offers only `parse` and `Display`, so the fake's range check copies no guard.
- **Suite helpers match slice a's layout.** The cases come first, then port-specific private helpers that wrap `succeeds`
  (`Scratch::{spec, ensure, close, panes}`, `enter`, `by_id`, `new_id`). A bespoke `Err(format!(..))` appears only for
  checks that are not equality (cases 6, 10 and 11, and `new_id`), as in slice a's `increasing` and `expect_change`.
- **ADR rule.** The two ADR-0021 items are exactly AC 8: the section 9 class reason, and the section 10 bullet, re-wrapped
  to its neighbours' width and carrying A's optional #640 sentence. Nothing else in the ADR changed. `GRID_UNREACHABLE`
  follows section 9's open-code mechanism (a `from_static` constant raised as `Refused`, so class Refusal) and needs no
  verb-table row until a verb raises it (ASSUMPTION 7).
- **Overlay candidates.** No added line names or copies the token store, `Lockout`, `Roster`, `log(Severity, ..)` or the
  hub's test-harness helpers (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`). `src/` adds no sleep, loop,
  blocking I/O or logging. The state lock is taken only after `faults.enter` returns, so there is no lock-order hazard
  with the switch.
- **Size and repository hygiene.** The largest files are `fake_herdr_test.rs` at 538 lines and `herdr.rs` at 525. Both are
  under `lint.sh`'s 600-line warning and well under the overlay's flag at about 800. The added lines name no personal
  infrastructure; the only hits are the repository's own issue links in the CHANGELOG, the form every entry uses. All
  seven commits use the GitHub no-reply address.

## Notes for F

None. The verdict is PASS, so there is nothing to fold.

## Notes for O (in the automated run: the run's own agent, at PR time)

1. Do not open a second cleanup issue. Open the **one** test-kit cleanup issue that #684's gate scoped, if no slice has
   opened it by then, and add this slice's items: finding 1's (a) to (e), the two-case requirement on `assert_fails_on`,
   and finding 2. Schedule it after slices b to e merge and before #640 or #642 starts.
2. Put finding 3's list in this PR's body and in one comment on #640. This closes Phase 3 round 2's Note 2 (the
   `profile apply` row) and Note 3 (the cleanup issue), which nobody has picked up. A search of the issues on 2026-10-09
   found neither.

## Patterns referenced

- `crates/holler-pane-testkit/src/fault.rs:17-134`, `src/conformance/mod.rs:43-100`, `src/conformance/pane_store.rs:23-116,
  408-517`, `src/pane_store.rs:20-222`, `src/feed.rs:216-220`, `tests/pane_store_conformance_test.rs:51-263`,
  `tests/fake_pane_store_test.rs:16-60`: the switch, runner, assertions, case table, scaffolding and mutation pattern this
  slice extends.
- `crates/holler-pane/src/error.rs:36-60, 136-150, 316-345` (`PaneCode` is `pub(crate)`; `ALL_CODES`; `from_static`),
  `src/grid.rs:29-73`, `src/pane.rs:74-100`, `src/ports.rs:100-235`.
- `crates/holler-cli/src/pane/wiring.rs:1-60, 121-226`, `crates/holler-pane/tests/ports_test.rs:163-258`: the other port
  implementations.
- `docs/adr/ADR-0021.md:304-410` (section 9, the open-code rule and the verb table), `:411-430` (section 10).
- Sibling slices, all unmerged: `.claude/worktrees/0684-host-harness/docs/handoffs/684/handoff-A-dup.md` (findings 1 and
  2, Notes for O); `.claude/worktrees/0682-profile-fakes/crates/holler-pane-testkit/src/feed.rs:243`; the test files of
  0682 and 0684, for the copies of `assert_suite_fails_on`.
