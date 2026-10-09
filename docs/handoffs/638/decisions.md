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
