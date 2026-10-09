# Decisions — #638 slice a: the fault switch, the fake `PaneStore` and its conformance suite

## A (Phase 3, up-front plan review) — 2026-10-09T11:05:35-06:00
- **Decided:** PASS on docs/handoffs/638-brief.md at 20483ae, with 12 warns (see handoff-A.md).
  - The plan extends the objects it should: the empty testkit crate, the frozen `PaneStore` trait, `next_generation`, the closed `PaneError` set and `ProfileName::slug`.
  - The testkit's only dependency is `holler-pane` (ADR-0021 §5).
  - The new `FaultSwitch` and feed are the only ones in the crate and are justified in writing.
  - T and F follow W-1 (stub wording), W-5, W-6, W-11 and W-12 directly. W-3 and W-4 are re-checked at the anti-duplication gate.
  - W-1 (blast radius of #681 to #684) and W-2 (#682's JSON-decode case needs `serde_json`) are for the run's agent or the operator, before slices b to e start.
- **Assumed:**
  - The 0639 branch (9289d30) is what #639 will merge. Its reply cursor is the head after a snapshot (`store.rs:261-284`, `store.rs:309`), which is the behaviour W-5's reordered case 14 pins.
  - #661 does not fix whether the membership check runs before or after `next_generation` (issue text read 2026-10-09).
- **Hedged:**
  - W-5 tightens case 14 by reading "then every later change" as later than the snapshot. A client that mirrors the state stays correct under either reading, so this is a warn, not a block.
  - W-1 and W-2 are defects in issues #681 to #684, not in this slice's code, so they are warns rather than a block of this run.
  - W-10 is left to O: pin the order in ADR-0021 §8, or document it in the fake.
- **Evidence:**
  - **Read:** the brief; issues #638, #661 and #681 to #684; ADR-0021 in full.
  - **holler-pane:** `ports.rs`, `generation.rs`, `error.rs`, `pane.rs`, `profile.rs`, `reply.rs:115`, `lib.rs`, `tests/common/mod.rs`, `tests/argv_env_test.rs`.
  - **Hub (`issue-639-implementation`):** `panes/{mod,store,feed}.rs` and `tests/pane_support/mod.rs`.
  - **Workspace:** the lints in `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `Cargo.lock:1230-1240`, and `docs/testing.md` (the harness table).
  - **#670 precedent:** its brief and handoff-A at `fa895ac^`.
  - **Greps:** module layout (12 `name.rs` + `name/` directories against 4 `mod.rs`), `#[path]` only in tests, `as_str` against `name` (26 to 1), `kiwi` and `demo-c*r*` fixtures.
  - **Scratch check:** a two-crate cargo workspace for the dev-dependency cycle. Its integration test passes, and its `--lib` unit test fails with E0308.

## T-red (Phase 4, author tests) -- 2026-10-09T12:15:00-06:00
- **Decided:** RED is valid. Two test files, `tests/pane_store_conformance_test.rs` (10 tests: suite, case ids, guard, a no-break control and six mutants) and `tests/fake_pane_store_test.rs` (22 tests: faults, call log, another writer, idle wait, seeding, fixture, bounds).
  - Applied A's W-5 (a direct test that `watch(0)` resumes from the head as of the snapshot), W-6 (`demo-*` names, no `hj*` fixture field), W-3 (`seeded` leaves the call log empty) and W-12 (`PortOp::as_str`).
  - Added a control test (the unbroken mutant wrapper passes) so a mutant cannot fail because of the wrapper.
  - Added `seeding_the_same_name_twice_is_a_conflict` and `a_watch_at_the_head_is_idle_not_usage`, derived from the brief's "each created at expected 0" and "ahead of the head".
- **Assumed:**
  - A compile failure on missing items is the RED the brief prescribes (Test plan), since no production stub may be written by T.
  - Integration tests reach `holler_pane` through the crate's normal dependency, so F adds it to the manifest first.
  - `PaneStoreOp` derives `Debug` and `PartialEq` (the call-log test compares `Vec<PaneStoreOp>`).
- **Hedged:**
  - The six mutants were never run against a real suite (none exists). T-green must run them and repair any that fails on a different case.
  - W-10 (membership versus generation order) is not pinned: A left it to O, and no run has an O.
  - `the_fake_is_send_and_sync_and_its_watch_is_send` cannot be RED at run time (a compile-time check).
- **Evidence:** `cargo test -p holler-pane-testkit` fails with 7 E0432/E0433 errors, all missing items. A throwaway stub crate in the scratchpad (outside the repository) type-checked both files and showed assertion failures; a throwaway working fake passed all 22 fake tests. `rustfmt --check` clean.

## F (Phase 5, implement) -- 2026-10-09T11:40:00-06:00
- **Decided:** implemented the brief's slice a. All 32 of T's tests are GREEN unchanged, and every AC 8 guard passes (see handoff-F.md).
  - **Feed (A's W-4).** `feed.rs` owns everything generic about the change feed, over a crate-private `Change` trait shaped like the hub's `RegistryEntry`: the head, the whole history, the live map that `get` and `list` read, cursor allocation, the `Cursor(0)` snapshot, "after `since`", `usage`, the idle wait and the iterator.
  - **The watch.** Every `next()` passes `faults.enter(WatchNext)` first, and every poll resumes from the head (A's W-5; main's ADR-0021 item 7).
  - **Write paths (A's W-3).** There is one write path per kind, `put` and `remove` with a `Writer`. Every generation goes through `next_generation`, and every cursor comes from `Log::append`. `seeded` and `concurrent_*` bypass the faults and the call log.
  - **Membership (A's W-10, unpinned).** It runs for port writes only, after the generation check, compares slugs, and does not check that the profile exists. Both points are written in `FakePaneStore`'s doc.
  - **Suite (A's W-11).** One `CASES` table, with case 14 reordered (W-5) and neutral `demo-*` names (W-6). The generic helpers (`run_cases`, `succeeds`, `expect_code`, `expect_eq`, `next_item`, `drain`) stay crate-private for slices c to e.
  - **Names.** `PortOp::as_str` (W-12). The stubs name their issue, #681 to #684 (W-1 wording).
  - **CHANGELOG.** The entry is placed before the #670 entry, not after it, because #639 (PR #685) merged its own entry at that spot after this branch was cut. A literal placement conflicts, and the Workflow does not rebase.
- **Assumed:**
  - The run's agent merges this branch without rebasing first. Either way it is clean: `git merge-file` reports 0 conflicts in `CHANGELOG.md`, and no other path overlaps `2a6f349`.
  - `concurrent_put` skipping the membership rule matches the brief's "unconditionally".
- **Hedged:**
  - A's optional public `drain` was not taken (the brief's API is exact); adding it later is additive.
  - Failure details print whole records with `{:?}`, which is verbose but complete.
  - W-1 and W-2 (issues #681 to #684) remain for the run's agent or the operator.
- **Evidence:**
  - **Crate guards:** `cargo test -p holler-pane-testkit` (22 + 10 pass; 30 repeated runs, 0 failed).
  - **Workspace guards:** build, `clippy --workspace --all-targets -D warnings`, `test --workspace` (1073 passed, 0 failed), `cargo machete`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `scripts/test-hooks.sh`, `rustfmt --check`, `cargo tree` (no `holler-cli`, `-hub` or `-adapter` edge), and `rustdoc -D warnings`.
  - **Mutants:** a scratch probe crate (outside the repository) ran each mutant through the suite. Each fails on its named case, and the unbroken wrapper passes.
  - **Hub:** the suite ran against main's real `PaneState` (`2a6f349`) in a throwaway export. Cases 1 to 18 pass and case 19 fails, as the brief predicts until #661.
  - **Unchanged source relied on:** see `evidence.md`.

## T (Phase 7, verify GREEN + Tier 2)

- **Decided:** PASS. The suite is GREEN (22 + 10), Tier 1 matches F's report, and no authored test needed repair.
- **Assumed:** the CHANGELOG placement before the #670 entry is acceptable, since a literal placement would conflict with #639 on main.
- **Hedged:** case 19 against the hub stays red until #661; not a defect of this slice.
- **Evidence:** 20 repeated runs with 0 failures; three production-code breaks (no `usage` check, no delay sleep, non-consumed one-shot) each failed the named tests and were reverted; `cargo test --workspace` 1073 passed, 0 failed; `docs/handoffs/638/handoff-T-green.md`.

## A (Phase 7, anti-duplication gate) -- 2026-10-09T11:55:17-06:00
- **Decided:** PASS on 939d79c..8e38d8f, with 4 warns (see handoff-A-dup.md). No parallel path.
  - **Extended:** the crate, `PaneStore`, `next_generation` (two call sites, no other generation arithmetic), the closed `PaneError` set and `ProfileName::slug`.
  - **New and single:** `FaultSwitch` and the feed.
  - **W-3 holds:** one `put`/`remove` path; every cursor comes from `Log::append`; `seeded` and `concurrent_*` bypass the faults and the call log.
  - **W-4 holds:** everything generic about the watch is in `feed.rs`, over `Change`.
  - **The brief's check holds:** the cases share `expect_code` and `drain`.
  - **Warns:**
    - (1) Move `increasing` to `conformance/mod.rs` before #682 starts, and decide in #682 whether its profile watch cases copy or share cases 14 to 18.
    - (2) A third drain-to-idle helper (a test copy here and the hub's on main); a public `conformance::drain` would fold them.
    - (3) The fake's private `check_membership` makes the other half of the check that the hub's public `check_membership` makes; rename it.
    - (4) The `usage` rule now exists in the hub and in the testkit; if O makes it a port rule, use the `generation.rs` pattern.
- **Assumed:**
  - Brief decision 1 binds #682: it never edits `conformance/mod.rs`. That is why finding 1 matters now.
  - The hub on `origin/main` (`3f9fbf2`) is what the suite will later run against.
- **Hedged:**
  - All four findings are warns. Finding 1 is a forward copy that #682's scope would force, not one in this diff. Findings 2 and 4 are near-copies the brief justified in writing. Finding 3 is a name.
  - The Phase 3 W-1 issue edits (#681 to #684) remain for the run's agent or the operator.
- **Evidence:**
  - **Read in full:** every changed file (`git diff 939d79c 8e38d8f`).
  - **Hub on `origin/main`:** `panes/{mod,feed}.rs`, `profile/mod.rs`, `tests/pane_support/mod.rs`; ADR-0021 lines 538-551.
  - **Ports:** `holler-pane` `ports.rs`, `profile.rs:255-420`, `error.rs` (`PaneCode` is `pub(crate)`), `generation.rs:1-16`.
  - **Issues:** #681 to #684 (their blast radii).
  - **Greps:**
    - arithmetic and `next_generation` call sites, and feed terms by file;
    - lock helpers across the workspace;
    - `fn drain` and fault mechanisms across the workspace;
    - code literals in tests: 27 uses, against 0 `PaneCode::` uses;
    - the overlay's Phase 7 candidates (none hit);
    - personal names in the diff (only the test that asserts their absence).
  - **Merge:** `git merge-tree HEAD origin/main` completes with no conflicts.
