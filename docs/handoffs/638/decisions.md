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
