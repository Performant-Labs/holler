# Handoff-A-dup: Phase 7 - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-684-implementation
**Diff base:** e410e9d (origin/main and the merge base; main has not moved)   **Diff head:** d6f5850
**Reuse map:** docs/handoffs/684-brief.md, "Extend vs new" and the "What slice a left to reuse" evidence block (this run has no survey.md)
**Verdict:** PASS

## Summary

PASS. F extended what slice a built and added no parallel path:

- Both fakes hold a `FaultSwitch<Op>` and call `enter` first in every port method (`host.rs:171, 183, 188, 196`;
  `harness.rs:283, 288, 293, 300, 309, 320, 337, 352`).
- `HostOp` and `HarnessOp` implement the merged `PortOp::as_str`.
- Both suites run on `run_cases`, `succeeds`, `expect_code` and `expect_eq` from `conformance/mod.rs`, unchanged. The harness
  suite folds its rig into the subject rather than changing the runner (`conformance/harness.rs:147-154`).
- Every error is a closed `PaneError` variant. The frozen `timeout` has the fault switch's shape (`harness.rs:477-481`
  against `fault.rs:121-125`).
- The public surface is the brief's API, item for item.

A workspace search found no other in-memory `HostPort` or `HarnessPort` that these fakes could have extended.

All three findings are `warn`. Each is a small copy that this slice's no-edit rule forces, or a doc item that sits between
slices. None is new drift. All three belong in one cleanup issue, and no such issue exists yet.

## Findings

No block. Nothing in the diff builds a parallel path to an object the map named.

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane-testkit/src/conformance/harness.rs:53-74, 142-155` | Carried forward from Phase 3 W-1. Both sides are now implemented. This slice has `HarnessRig { ports, panes }` with a 3-tuple `fresh: FnMut() -> (S, HarnessRig, K)`. Sibling #683 has `HerdrFixture<H> { port, session, workspace }` with `fresh: FnMut() -> (HerdrFixture<H>, K)` (`.claude/worktrees/0683-herdr-fake/crates/holler-pane-testkit/src/conformance/herdr.rs:57-64, 140-143`). Once both merge, the crate has two shapes and two words, "Rig" and "Fixture", for one idea: the data the implementation under test supplies to each case. The ASSUMPTION prefixes also differ: `(#640):` there, `(#642 to confirm):` here. This is not a block. The brief names `HarnessRig` and justifies it (Decision 8), neither shape is on main, so there is no dominant pattern, and F correctly built what the brief specified. | Settle it in the cleanup issue (see Notes for O), **before #640 or #642 writes its suite runner**. After that, a change breaks the adapters' tests. Either align on one shape (#683's bundle needs no re-tupling closure, which keeps it closest to slice a's call), or record the two shapes as deliberate in `conformance/mod.rs`'s module doc. |
| 2 | warn | `src/host.rs:158-160` and `src/harness.rs:270-272` (`lock`); `src/conformance/host.rs:249-270` and `src/conformance/harness.rs:400-407` (`holds`, `lacks`, `sorted`); `src/conformance/harness.rs:48, 67-70` (`SCRATCH`, 48100, the `scratch:<name>` id); `tests/host_conformance_test.rs:149-160` and `tests/harness_conformance_test.rs:64-75` (`assert_fails_on`) | Each is a small copy. The brief justifies each in writing, or Phase 3 W-5 accepted it, because this slice may not edit `lib.rs`, `conformance/mod.rs`, `fixture.rs` or `feed.rs`. One fact is new since Phase 3. #682, not yet merged, makes `feed::lock<T>` `pub(crate)` (`.claude/worktrees/0682-profile-fakes/crates/holler-pane-testkit/src/feed.rs:243`). #683's plan review already sends its own `herdr.rs` and `prober.rs` locks there (its handoff-A row 3). So once slices b to e merge, the crate has one shared lock and five per-struct copies of the same line (fault, herdr, prober, host, harness). It also has five copies of the mutation-test assertion (pane store, profile store, herdr, host, harness). The rig repeats `fixture.rs:11, 15, 31` (`SAMPLE_PORT`, `SCRATCH`, the `pane_id` form). Nothing keeps those in step except two tests that pin the same literal. The two `holds` helpers differ only in element type (pids, session ids), and `conformance/mod.rs` has no containment helper they could have used. | No change in this slice. Put the scope in the cleanup issue: one `tests/support/mod.rs` with `assert_fails_on(result: Conformance, case: &str)` (this slice's shape is the one A proposed) and #683's `CaseGuard`; generic `holds` and `lacks` in `conformance/mod.rs`, next to #682's five `pub(super)` helpers (#682 handoff-A W-2); every fake on one crate-private lock helper; `fixture.rs` exposing its `SCRATCH` and `SAMPLE_PORT` (or a `pane_id` helper) for `HarnessRig::sample`. |
| 3 | warn | `crates/holler-pane-testkit/src/lib.rs:27-28` (not in this diff) | "The modules of slices b to e are empty stubs" becomes false for `host` and `harness` when this merges. #681's Phase 3 W-6 and its Phase 7 row 2 gave the rewrite to #684 as "the last slice". This brief's AC 9 forbids any `lib.rs` edit, though, and the four slices merge in no fixed order, so nobody picked the note up. F was right to leave `lib.rs` alone, because S audits against AC 9. | Add the sentence to the cleanup issue, which edits `conformance/mod.rs` anyway, so that `lib.rs` is fair game there. Do not widen this slice: AC 9 stands. |

Checked and found clean (the evidence for the PASS):

- **Placement and scope.** `git diff --name-only e410e9d...d6f5850` lists only the brief's blast radius:
  - the four stub files, filled;
  - four new test files;
  - `CHANGELOG.md`;
  - `docs/handoffs/684*`.

  Not changed: `lib.rs`, `conformance/mod.rs`, `fixture.rs`, `feed.rs`, any manifest, `Cargo.lock`, any other crate, any ADR
  and any golden file. There is no `harness/world.rs`, because `harness.rs` is 502 lines.
- **No parallel implementation of either port.** The workspace has two other implementations, and neither is a fake:
  - `Unwired` (`holler-cli/src/pane/wiring.rs:151, 169`) is the CLI's not-yet-wired placeholder. Every method answers
    `not-implemented`.
  - `TestHost` and `TestHarness` (`holler-pane/tests/ports_test.rs:197, 214`) return constants to pin the frozen
    signatures.

  The adapter crates are still empty skeletons (#641, #642). The HTTP-level fake OpenCode servers in the `holler-body` and
  `holler-cli` tests sit at attach mode's wire seam, as Phase 3 noted, and the test kit cannot depend on them.
- **One rule per check, within the diff.** Shared helpers carry the repeated logic:
  - `navigate` and `select_session` share `tui_port`, `known` and `show`.
  - `freeze`, `thaw` and `kill` share `World::signal`.
  - Every port call that needs a running server goes through `World::reach`.
  - `run`, `ps`, `exit_process` and `end_session` share `not_found`.
  - Each error shape has one constructor (`frozen`, `unreachable_server`, `no_tui`, `unknown_session`).
  - Each quirk is a single branch at the spot where the contract would refuse (`harness.rs:312, 339`), not a second code
    path.
- **Slice a's patterns kept.**
  - Scenario and inspection methods bypass the faults and the call log, as `FakePaneStore::concurrent_*` do.
  - Each fake has `new()` and `Default`, and its `faults()` returns `&FaultSwitch<Op>`.
  - Each suite keeps one `CASES` table, which `*_cases()` lists.
  - The tests follow the `Break`/`Mutant` pattern.
  - `ServerState` (the fake's ground truth: running, frozen, killed) does not duplicate `holler_pane::Health` (a stored
    observation: healthy, unhealthy with a reason, unknown).
- **Overlay candidates.** No added line names or copies the token store, `Lockout`, `Roster`, `log(Severity, ..)` or the
  hub's test-harness helpers. `src/` adds no blocking I/O, no sleep and no logging.
- **Dependency direction.** The added `use` lines name only `std`, `holler_pane`, `crate::fault` and `super` (the
  conformance runner and its assertions). They do not name `crate::feed` or `crate::fixture`.
- **Size and repository hygiene.** The largest source file is `harness.rs` at 502 lines, and the largest test file is
  `fake_harness_test.rs` at 576. Both are under `lint.sh`'s 600-line warning and well under the overlay's flag at about 800.
  The added lines name no personal infrastructure and contain no absolute home path. All five commits use the GitHub no-reply
  address.

## Notes for F

None. The verdict is PASS, so there is nothing to fold.

## Notes for O

Open **one** test-kit cleanup issue now, so that it is not lost, and schedule it for after slices b to e (#681 to #684)
merge and before #640 or #642 starts. A search of the repository's issues found none. Its scope gathers what four plan
reviews deferred:

- finding 1 (one suite-fixture shape, or the two recorded as deliberate);
- finding 2 (`tests/support/mod.rs`, `holds` and `lacks` in `conformance/mod.rs`, one lock helper, the fixture constants);
- finding 3 (the stale `lib.rs:27-28` sentence);
- #682's W-2 (its five `pub(super)` suite helpers move into `conformance/mod.rs`);
- #683's W-3 (`CaseGuard`, and `herdr.rs` and `prober.rs` onto the shared lock).

Optionally, a default `PortOp::timeout(self) -> PaneError` in `fault.rs` would give the wedged and frozen `timeout`s one
constructor. Today they are two three-line sites, `fault.rs:122-124` and `harness.rs:477-481`.

For the PR body (Phase 3 W-3, unchanged): list what binds the adapters.

- **#641:** `run` and `ps` of a missing session are `pane-not-found`; `stop_owned` of a missing session is `Ok`; an empty
  argv is `usage`.
- **#642:** reachability is checked before existence; an unknown id is `session-not-found` for `abort`, `attach_tui` and
  `select_session`; `select_session` on a pane with no TUI is `unavailable`, checked before the id; `shown_session` with
  no TUI is `Ok(None)`.

## Patterns referenced

- `crates/holler-pane-testkit/src/fault.rs:17-134`, `src/conformance/mod.rs:31-100`, `src/pane_store.rs:20-222`,
  `src/conformance/pane_store.rs:24-116, 408-417`, `src/fixture.rs:10-31`, `src/feed.rs:216-220`,
  `tests/pane_store_conformance_test.rs:151-311` (the switch, runner, assertions, case table, scaffolding and mutation
  pattern this slice extends).
- `crates/holler-pane/src/ports.rs:150-200`, `src/pane.rs:74-89, 132-139` (`PaneId::new` is the only constructor; `Health`).
- `crates/holler-cli/src/pane/wiring.rs:151-201`, `crates/holler-pane/tests/ports_test.rs:197-241` (the other two port
  implementations).
- Sibling slices, all unmerged: `.claude/worktrees/0683-herdr-fake/crates/holler-pane-testkit/src/conformance/herdr.rs:48-153`
  and its `docs/handoffs/683/handoff-A.md` row 3; `.claude/worktrees/0682-profile-fakes` (the `feed.rs`, `fixture.rs` and
  `conformance/pane_store.rs` diff, and `docs/handoffs/682/handoff-A.md` W-2); `.claude/worktrees/0681-envelope/docs/handoffs/681/handoff-A-dup.md`
  row 2.
