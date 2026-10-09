# Handoff-F: Phase 5 - #638 slice a: the fault switch, the fake `PaneStore` and its conformance suite

**Date:** 2026-10-09
**Branch:** issue-638-implementation
**Issue:** #638 (slice a; this PR is "Part of #638", not "Closes")

| Field | Value |
|-------|-------|
| GitHub issue | #638 (amended 2026-10-09: #638 is slice a; slices b to e are #681 to #684) |
| Working branch | `issue-638-implementation` (base `939d79c`) |
| Build plan phase | Epic #633, wave 3 prerequisite (the test kit) |
| Input documents read | the brief, `handoff-A.md`, `handoff-T-red.md`, `decisions.md`, issue #638, ADR-0021 (sections 2, 5, 6, 7, 8, Decisions taken; and main's amendment, item 7), `holler-pane` (`ports.rs`, `generation.rs`, `pane.rs`, `profile.rs`, `error.rs`, `argv.rs`, `grid.rs`, `lib.rs`, `tests/common/mod.rs`), the hub registry (`panes/{mod,store,feed}.rs`) |
| Acceptance criteria count | 8 |
| Handoff document path | `docs/handoffs/638/handoff-F.md` |

This was an automated Workflow run, so no one confirmed the table; it is recorded here instead. The scope cap does not apply: the brief already split the issue, and this slice is about 1,400 production lines in 6 modules plus 12 stubs.

## What was done

- `crates/holler-pane-testkit/Cargo.toml`: the new description and the one dependency, `holler-pane`, with its comment. No `[dev-dependencies]`.
- `src/lib.rs`: declares exactly the 12 modules of AC 2 (`mod feed;` is crate-private), with a doc that lists each one and the slice that fills it. No `pub use`.
- `src/fault.rs` (134 lines):
  - `PortOp` (`as_str`, per A's W-12), `Fault` (`Wedged`, `Fail`) and `FaultSwitch<Op>`.
  - The switch offers a standing fault, one-shot errors (FIFO per op), a delay and the call log.
  - `pub(crate) enter` records the call, sleeps without holding the lock, then answers the standing fault, then the oldest one-shot for that op.
- `src/feed.rs` (260 lines, crate-private), the generic change feed:
  - the `Change` trait over the event type;
  - `Log<E>`: the head, the whole history, and each live record's last change;
  - `Feed<E>`: the lock, the condition variable and the idle wait;
  - the `Stream` `Watch` iterator.
- `src/fixture.rs` (73 lines): `sample_pane`.
- `src/pane_store.rs` (259 lines):
  - `PaneStoreOp` and its `PortOp` impl.
  - `FakePaneStore`: `new`, `seeded`, `set_idle_wait`, `faults`, `concurrent_put`, `concurrent_delete`, `Default` and `PaneStore`.
  - `impl Change for PaneEvent`, and `check_membership`.
- `src/conformance/mod.rs` (137 lines):
  - public: `CaseFailure` and `Conformance`;
  - crate-private and generic, for the later suites: `run_cases`, `succeeds`, `expect_code`, `expect_eq`, `next_item` and `drain`;
  - the 6 suite modules.
- `src/conformance/pane_store.rs` (518 lines):
  - the one `CASES` table of 19 `(id, case)` rows, and `pane_store_cases()`;
  - `run_pane_store_conformance`, whose doc carries the `text` fence showing how the fake, the hub and the CLI client run it, and the note that case 19 waits for #661;
  - one function per case, and the case helpers.
- 12 stubs, each a `//!` doc naming what it will hold and its slice and issue (A's W-1 wording):
  - `src/{envelope,profile_store,profile_scope,herdr,prober,host,harness}.rs`;
  - `src/conformance/{profile_store,profile_scope,herdr,host,harness}.rs`.
- `CHANGELOG.md`: one `### Enhancements` entry under `## [Unreleased]`, linking #638 (see "Deviations" for its position).
- `Cargo.lock`: the testkit's one new dependency line (`holler-pane`), exactly as A's W-9 expected.

## Design decisions

1. **The feed owns the records (A's W-4).**
   - **What `Log<E>` holds:** the head, the whole history (`Vec<E>`) and `live: BTreeMap<Key, E>`, the last change of each live record.
   - **How the store uses it:** `get` and `list` read the live map, so the store has no second map that could disagree with the log, and `list` is in name order because the key is the `PaneName`.
   - **Generic parts, all in `feed.rs`:** cursor allocation (`append`, where overflow is `store-corrupt`), the `Cursor(0)` snapshot, "events after `since`", the `usage` check, the idle wait on the condition variable, and the iterator.
   - **The `Change` trait** (`key`, `cursor`, `record`) has the shape of the hub's `RegistryEntry`. `key()` returns an owned key, so slice c can key profiles by slug.
   - **Rejected:** a records map in the store beside a log in the feed (two sources of truth), and an iterator that polls one event at a time. With one event per poll, resuming "from the head as of the snapshot" (A's W-5) needs extra state.
2. **The watch iterator.**
   - Every `next()` calls `faults.enter(WatchNext)` first, even with changes still buffered, as the brief says. A fault set on an open stream therefore reaches it.
   - When nothing is buffered, it polls. A poll returns the owed changes and the head, and the stream resumes from that head every time. That is the hub's behaviour, and main's ADR-0021 amendment (Decisions taken, item 7: "the idle `pane/watch` cursor is the head") makes it the rule.
   - The first error is yielded once, and the stream then returns `None`.
3. **One write path per kind (A's W-3).**
   - `put(pane, Writer)` and `remove(name, Writer)` are the only writes. `Writer::Port(expected)` covers `cas_put`, `delete` and `seeded`; `Writer::Other` covers `concurrent_*`.
   - Every generation goes through `next_generation`, and `Writer::Other` calls it as `next_generation(current, current)`. Every cursor comes from `Log::append`.
   - `seeded` and `concurrent_*` bypass the faults and the call log, because only the port methods call `faults.enter`.
4. **Membership (brief decision 6; A's W-10 left unpinned).**
   - The rule runs for `Writer::Port` writes only, after the generation check, and compares slugs. It does not check that the profile exists.
   - Both of those points are written in `FakePaneStore`'s doc, which is the "leave it unpinned" option A offered.
   - `concurrent_put` skips the rule, as the brief's "unconditionally" says, so a test can stage any state another writer could leave.
5. **The fault switch.**
   - The one-shot queue is a `Vec<(Op, PaneError)>` searched by `==`, because `PortOp` has no `Hash` bound.
   - A standing fault answers before the queue and leaves it queued.
   - `Wedged` answers `Timeout { op: as_str() }` at once (brief decision 5).
6. **The suite (A's W-11).**
   - There is one `CASES` table. The runner iterates it and `pane_store_cases()` maps it, so the two cannot drift.
   - The cases take `&dyn PaneStore`, and the runner coerces `&S` at the call.
   - The generic `run_cases` calls `fresh` once per case and drops the store before its guard, which `the_guard_lives_for_the_case` checks.
   - The case functions share `expect_code` and `drain` (the A-dup check named in the brief).
   - Case 14 uses W-5's order (create a, create b, update a, create c, delete b). Names are `demo-c1r1`, `demo-c2r1` and `demo-c3r1` (W-6), and profiles are `Alpha` and `Beta`.
7. **Helpers stay crate-private.** A suggested a public, generic `drain` as optional. I did not take it, because the brief's API is "exact". Making it public later is additive.
8. **`sample_pane` values.**
   - Herdr session and workspace are `scratch`, and the pane id is `scratch:<name>`, so two sample panes never claim one Herdr pane.
   - The host is `localhost` and the cwd is `/srv/demo` (the CLI tests' neutral placeholder).
   - `host.tmux` is the pane's name (`pane.rs:108`).
   - The model is `demo-provider`/`demo-model`/`medium`, and the context is 100,000/150,000.

## Reuse / extend-vs-new

Extended, per the brief's "Extend vs new":

- the empty `holler-pane-testkit` crate and its manifest;
- the frozen `PaneStore` trait;
- `holler_pane::next_generation`, for every compare-and-swap (two call sites, `put` and `remove`, with no other generation arithmetic);
- the closed `PaneError` variants (`Conflict`, `PaneNotFound`, `PaneInOtherProfile`, `Timeout`, `StoreCorrupt`, `Usage`), with no new code;
- `ProfileName::slug`.

New, as the brief justified in writing: the generic `FaultSwitch` and the generic feed. Each is the only one in the crate, and slices c to e reuse them. Nothing was copied from the hub, whose `feed.rs` and `store.rs` are `pub(crate)` in a crate the testkit must not depend on. `MemPaneStore` (`holler-pane/tests/common`) is the accepted near-duplicate and is untouched. A's W-8 corrects the reason: it is outside this blast radius and is a trivial "the trait can be implemented" double.

## Architecture notes for A

- **Dependency:** a new normal edge, `holler-pane-testkit -> holler-pane`, and nothing else (`cargo tree` below). No other manifest changes.
- **Inside the crate:**
  - `pane_store -> feed -> fault`, and `pane_store -> fault`;
  - `conformance -> fixture` and `holler_pane` only. The suites never name a fake, and the fakes never name a suite.
- **Crate-private surface for slices c to e:**
  - `feed::{Change, Log, Feed}` (`Feed::new`, `set_idle_wait`, `read`, `write`, `watch`) and `FaultSwitch::enter`;
  - `conformance::{run_cases, succeeds, expect_code, expect_eq, next_item, drain}`.
  - Slice c adds `impl Change for ProfileEvent` in its own `profile_store.rs` (a local trait on a foreign type).
- **Public surface:** exactly the brief's, with A's W-12 rename (`PortOp::as_str`).
- **Concurrency:** one `Mutex` per feed and one per switch, taken with `unwrap_or_else(PoisonError::into_inner)`. Watchers wait with `Condvar::wait_timeout`, which releases the lock. Writers notify after releasing it. The check runs under the lock, so no wakeup is lost.

## Deviations from spec / wireframe

1. **The CHANGELOG entry sits just before the #670 entry, not after it (AC 7).**
   - **Why:** #639 merged to `main` at 11:18 MDT (PR #685, `2a6f349`), after this branch was cut, and added its own entry right after #670's, which is the spot AC 7 names. Our entry there would conflict with main for certain. The Workflow's PR step only runs `gh pr create` and never rebases, so the PR would be unmergeable and its CI would not run.
   - **Checked:** a three-way merge of `CHANGELOG.md` (base `939d79c`, this tree, `origin/main`) with `git merge-file` reports 0 conflicts.
   - **Order:** `docs/releasing.md` step 7 reorganizes the section by hand at release time.
2. None else. Every other AC is met as written.

## Tier 1 self-check (incl. tests now GREEN)

All run in the worktree, at 11:30–11:37 MDT:

```
$ cargo test -p holler-pane-testkit
     Running tests/fake_pane_store_test.rs
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/pane_store_conformance_test.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   (30 repeated runs: 0 failed, which covers the timing tests)

$ cargo build --workspace                                   -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings     -> exit 0
$ cargo test --workspace                                    -> exit 0; 101 test binaries, 1073 passed, 0 failed, 5 ignored
     (holler-hub `testkit_links ... ok`, holler-cli `list::testkit_links ... ok`, both unchanged)
$ cargo machete                                             -> "didn't find any unused dependencies"
$ bash scripts/lint.sh                                      -> exit 0 (600-line warnings only for pre-existing files; none in this crate)
$ bash scripts/changelog-check.sh                           -> changelog-check: ok
$ bash scripts/test-hooks.sh                                -> exit 0
$ rustfmt --check --edition 2021 src/*.rs src/conformance/*.rs tests/*.rs   -> exit 0
$ cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'   -> prints nothing
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane-testkit --no-deps      -> exit 0
```

File sizes: the largest file is `conformance/pane_store.rs` at 518 lines, under 600. Clippy enforces the function limits (100 lines, cognitive complexity 15).

**The mutants fail where they should.** A throwaway probe crate in the scratchpad, never committed, ran T's six mutants and the no-break wrapper through the suite and printed every failure:

| Mutant | Cases it fails |
|---|---|
| none (`Break::Nothing`) | none; it passes |
| `NoCas` | create-over-existing-conflicts, **stale-generation-conflicts**, expected-ahead-conflicts, failed-write-changes-nothing |
| `KeepsSubmittedGeneration` | 10 cases, including **submitted-generation-ignored** |
| `GenerationBeforeExistence` | only **delete-missing-is-pane-not-found** |
| `RepeatsOnResume` | **watch-resumes-without-gap-or-repeat**, failed-write-changes-nothing |
| `EndsStreamWhenIdle` | the five watch cases, 14 to 18, including **watch-idle-is-ok-none-and-stays-usable** |
| `LetsPaneChangeProfile` | only **pane-in-other-profile** |

**The suite runs against the real hub registry.**

- **Setup:** `origin/main` (`2a6f349`, #639 merged) was exported to a throwaway tree outside the repository, this crate was laid over it, and the suite ran against `PaneState::load_with(&HubState::from_root(<tempdir>), PaneStoreOptions { watch_wait: 50 ms, feed_retained: 1024 })`, exactly as the doc fence shows.
- **Result:** cases 1 to 18 pass. `pane-in-other-profile` fails with "cas_put moving a pane of Alpha to Beta: expected `pane-in-other-profile`, but it succeeded". That is the gap the brief's Risk 1 expects until #661 merges.
- **Cleanup:** the tree was deleted afterwards.

## Evidence appendix

`docs/handoffs/638/evidence.md` (7 facts from unchanged `holler-pane` source: `next_generation`, `delete`'s check order, the `Watch` contract, `Cursor: Ord`, `ProfileName::slug`, the three error codes, `HostInfo.tmux`).

## Tests that look wrong (for T)

None. All 32 authored tests pass without change. The six mutants, which T-red could not run, fail on their named cases (table above).

## Known issues

1. **The hub fails case 19 until #661 merges.** This is expected, documented in `run_pane_store_conformance`'s doc, and verified above.
2. **The branch is one commit behind `origin/main`** (`2a6f349`, #639). No file overlaps except `CHANGELOG.md`, which merges cleanly (see "Deviations").
3. **Issue fixes before slices b to e start (not F's):** A's W-1 (widen the blast radius of #681 to #684 to their `conformance/*.rs` stubs and tests) and W-2 (#682's JSON-decode case needs `serde_json`) are still open, for the run's agent or the operator.
4. **Long failure details.** A case that compares records prints both whole `Pane`s with `{:?}`. The detail is complete and non-empty, as the mutation tests require, but long.

## Files changed

- `Cargo.lock`
- `CHANGELOG.md`
- `crates/holler-pane-testkit/Cargo.toml`
- `crates/holler-pane-testkit/src/lib.rs`
- `crates/holler-pane-testkit/src/fault.rs`
- `crates/holler-pane-testkit/src/feed.rs`
- `crates/holler-pane-testkit/src/fixture.rs`
- `crates/holler-pane-testkit/src/pane_store.rs`
- `crates/holler-pane-testkit/src/conformance/mod.rs`
- `crates/holler-pane-testkit/src/conformance/pane_store.rs`
- `crates/holler-pane-testkit/src/envelope.rs` (stub)
- `crates/holler-pane-testkit/src/profile_store.rs` (stub)
- `crates/holler-pane-testkit/src/profile_scope.rs` (stub)
- `crates/holler-pane-testkit/src/herdr.rs` (stub)
- `crates/holler-pane-testkit/src/prober.rs` (stub)
- `crates/holler-pane-testkit/src/host.rs` (stub)
- `crates/holler-pane-testkit/src/harness.rs` (stub)
- `crates/holler-pane-testkit/src/conformance/profile_store.rs` (stub)
- `crates/holler-pane-testkit/src/conformance/profile_scope.rs` (stub)
- `crates/holler-pane-testkit/src/conformance/herdr.rs` (stub)
- `crates/holler-pane-testkit/src/conformance/host.rs` (stub)
- `crates/holler-pane-testkit/src/conformance/harness.rs` (stub)

Pipeline artifacts (not production): `docs/handoffs/638/handoff-F.md`, `docs/handoffs/638/evidence.md`, `docs/handoffs/638/decisions.md` (appended).
