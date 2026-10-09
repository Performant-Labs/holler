# Handoff-F: Phase 6 - #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite

**Date:** 2026-10-09
**Branch:** issue-683-implementation (implemented on top of 4bc8937, T-red PASS; nothing committed by F)
**Issue:** #683 (slice d of #638, epic #633)

## What was done
- `crates/holler-pane-testkit/src/herdr.rs`: the stub is filled (525 lines). It has the constants
  `PROTOCOL_22_VERSION`, `SUPPORTED_VERSIONS`, `UNSUPPORTED_VERSION` and `GRID_UNREACHABLE` (an open code built with
  `RefusalCode::from_static`). It has `HerdrOp` (`impl PortOp`, `"herdr.<method>"`), `Placement`, `HerdrVersion`
  and `Sent`. `FakeHerdr` has `new`, `with_workspace`, `set_placement`, `set_version`, `faults`, `sent`, `vanish`
  and `print`, and implements `HerdrPort`.
- `crates/holler-pane-testkit/src/prober.rs`: the stub is filled (95 lines). It has `ProbeCall` and `FakeProber`
  (`new`, `script`, `calls`, `Default`, `impl Prober`).
- `crates/holler-pane-testkit/src/conformance/herdr.rs`: the stub is filled (424 lines). It has `HerdrFixture`,
  `herdr_cases`, `run_herdr_conformance` and the 11 cases in one `CASES` table, over a crate-private view
  `Scratch { port, session, workspace }`.
- `CHANGELOG.md`: one entry under `## [Unreleased]` / `### Enhancements`, after the #676 entry (AC 6).
- `docs/adr/ADR-0021.md`: AC 8. (a) Section 10's `grid-out-of-range` bullet keeps its text and adds the second
  condition, re-wrapped over three lines to its neighbours' width. (b) Section 9's class-table reason names both
  sources. Nothing else in the ADR changes (`git diff --numstat`: 4 lines added, 2 removed).

## Design decisions
- **`ensure_pane` is split into the brief's helpers.** Step 1 is the lookup (`State::workspace_mut`). Steps 2 to 5
  are in `Workspace::place`, which calls `check_range`, `occupant`, `check_split` (split-only placement only) and
  `mint`, in that order. The trait method itself is seven lines. Every function is well under clippy's limits of
  100 lines and complexity 15.
- **Panes are kept per workspace in a `BTreeMap<(u16, u16), LivePane>`.** The snapshot's order (workspaces in
  declaration order, then row, then column) is then just the map's iteration order, and finding a cell is one
  lookup. `GridPos` has no `Ord`, so the key is a `(row, col)` tuple, and the grid is rebuilt from the key when a
  pane is reported. Finding a pane by id scans every workspace, which is fine for a fake.
  - Alternatives: a `Vec` of panes, sorted on every snapshot, or a key type of my own.
- **The id counter (`minted`, a `u64`) holds the `M` of the last id minted, and only goes up.** The next `M` is
  `checked_add(1)`, and an overflow is `unavailable` and creates nothing. `close` and `vanish` never touch the
  counter, so no id is reused.
  - `base36` uses `char::from_digit(_, 36)` and upper-cases the result, so there is no lookup table and no indexing.
  - `last_lines` counts `str::lines` and skips, so there is no slicing that could panic.
- **`ensure_pane` returns a `HerdrPane` built from the spec's session and workspace.** Step 1 has already checked
  that they are the fake's own, so it equals what `snapshot` reports for that pane.
- **The messages name what the brief asks for.**
  - `grid-out-of-range` names the cell and the size, for example `r1c2 is outside workspace "scratch", which is 2
    rows by 1 column`. A small `count` helper gets "1 column" and "2 rows" right.
  - `grid-unreachable` names the cell and says Herdr places a pane only by a right or down split.
  - `herdr-version-unsupported` names `UNSUPPORTED_VERSION` and `SUPPORTED_VERSIONS`.
  - `unavailable` names the session and the workspace asked for, and the session the fake serves.
- **Suite case 2 ensures `r1c1` first.** It follows the brief's rule that every case ensures `r1c1` before any other
  cell. It then checks that `r1c2` and `r3c1` are each `grid-out-of-range` and that the workspace's panes are
  unchanged after each.
- **Suite case 3 asserts that the workspace's panes equal `[first]`.** That is "exactly one pane, at `r1c1`, with
  that id", read over the whole workspace and not only over the cell. It is equivalent for any port that meets the
  fixture's contract (no pane, or only the root at `r1c1`) and stricter against a broken one.
- **Suite case 4 compares both lists sorted by id.** That gives the brief's "any order".
- **Two small helpers serve cases 5 and 6:** `new_id`, "this id is none of the earlier ones" (a check specific to
  Herdr ids, not a generic `expect_ne`), and `by_id`. Every other check goes through the shared `succeeds`,
  `expect_code` and `expect_eq`. Case 6's failure detail lists the ids it saw.
- **Each fake has its own poison-tolerant `lock()` method,** the idiom of `fault.rs:105-107`. A accepted this in
  round 2, warn 3 (`feed::lock` becomes `pub(crate)` only in #682).
- **ADR, AC 8(a).** The bullet is re-wrapped at 125, 118 and 120 columns against neighbours of up to 126, per A's
  warn 1. It also carries A's optional sentence: "How the adapter learns a workspace's extent is #640's."
- **ASSUMPTION 9.** The comment at suite case 2 also names section 9's `profile apply` row, and the row of any other
  verb that can ask `ensure_pane` for a cell outside the extent, as #640's to update (A's note 2).

## Reuse / extend-vs-new
- **Extended, per the brief's "Extend vs new":**
  - Slice a's three stubs are filled. `lib.rs`, `conformance/mod.rs`, `feed.rs` and slice a's other files are
    unchanged.
  - Every Herdr fault and the call log come from `FaultSwitch<HerdrOp>`, through `PortOp` and `Fault`.
  - The suite runs on `run_cases`, `succeeds`, `expect_code`, `expect_eq` and `Conformance`/`CaseFailure`.
  - Errors are the closed `PaneError` variants (`GridOutOfRange`, `PaneNotFound`, `HerdrVersionUnsupported`,
    `Unavailable`, `Usage`, and `Timeout` through the switch), plus one open code from `RefusalCode::from_static`.
  - Cells are formatted with `GridPos`'s own `Display`, and the prober is keyed on `Argv`'s own `Hash`/`Eq`.
- **New, and named by the brief:** the private `base36` and `last_lines` in `herdr.rs`.
- **Small private helpers that duplicate nothing:**
  - in `herdr.rs`: `count` (the singular or plural noun of a message) and `not_found` (the one `pane-not-found`
    builder);
  - in the suite: `Scratch::{spec, ensure, close, panes}`, `enter`, `by_id` and `new_id`.
- **Searched for existing helpers:** none in the workspace does base 36 or a screen's last lines (A's round 1 and
  round 2 greps).

## Architecture notes for A
- **Layers touched:** `holler-pane-testkit` only, plus the CHANGELOG and two ADR-0021 items.
- **Dependencies:** no new dependency, and no `Cargo.toml` or `Cargo.lock` change. The dependency direction is
  unchanged: `cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'`
  prints nothing.
- **Public API:** exactly the brief's "Public API (exact)" section, with no extra public item. All new items are
  reached by module path, with no flat re-export.
- **Locks:** `FakeHerdr` holds two locks, the switch's and its state's. `faults.enter(op)` always runs before the
  state lock is taken, and never while it is held, so there is no lock-order hazard. `vanish` and `print` take only
  the state lock.
- **Self-report: `archChanged: false`.** No module boundary is new: the stubs were declared by slice a. No existing
  public interface changed, and the dependency direction is unchanged. The new public API is the one A reviewed up
  front, with no deviation.

## Deviations from spec / wireframe
- **API:** none. There is no wireframe (`uiSurface: false`).
- **Size:** larger than the brief's estimate, all under the 600-line warning.
  - `herdr.rs` is 525 lines (estimated ~370) and `conformance/herdr.rs` is 424 (estimated ~340); `prober.rs` is 95
    (estimated ~90).
  - The excess is doc comments, the nine ASSUMPTION comments (thirteen comment blocks with the repeats the brief
    asks for), and rustfmt's vertical layout of struct literals and the case table.
  - `bash scripts/lint.sh` prints no warning for any file of this crate.
- **CHANGELOG:** one sentence beyond AC 6's list, "ADR 0021 now records that `grid-out-of-range` also covers a cell
  outside its workspace", because the PR changes the ADR too. Everything AC 6 lists is in the entry.
- **ADR:** A's optional sentence about #640 and the extent (see Design decisions). The class, `ALL_CODES`,
  `class_of` and its tests are untouched.

## Tier 1 self-check (incl. tests now GREEN)
Run from the root of this run's worktree (`.claude/worktrees/0683-herdr-fake`):
```
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.28s
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.71s          (no warning; T's three test files included)
$ cargo test -p holler-pane-testkit
fake_herdr_test             test result: ok. 22 passed; 0 failed
fake_pane_store_test        test result: ok. 22 passed; 0 failed   (slice a, unchanged)
fake_prober_test            test result: ok. 6 passed; 0 failed
herdr_conformance_test      test result: ok. 15 passed; 0 failed   (both placements, case list, fresh fixture, all 10 mutants)
pane_store_conformance_test test result: ok. 10 passed; 0 failed   (slice a, unchanged)
$ cargo test --workspace
exit 0; 108 test binaries: 1152 passed, 0 failed, 5 ignored
$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!
$ bash scripts/lint.sh
exit 0 (its 600-line warnings are all pre-existing files in other crates; none in holler-pane-testkit)
$ bash scripts/changelog-check.sh
changelog-check: ok
$ bash scripts/test-hooks.sh
exit 0
$ rustfmt --edition 2021 --check <the 3 src files and the 3 new test files>
clean
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane-testkit --no-deps
exit 0 (every intra-doc link resolves)
$ grep -nE '\.unwrap\(\)|\.expect\(|panic!|unreachable!|assert!|todo!|allow\(' <the 3 src files>
one hit only: the `.expect("workspace")` inside the brief's `text`-fence usage example in a doc comment (not compiled)
```

**Mutants checked beyond the pass/fail (informational; T-green owns the formal check).** I copied T's `Mutant`
wrapper into a throwaway crate in the session scratchpad (nothing in the repo) and printed every failure detail.
Each mutant fails its named case, for that case's reason:

| Mutant | Fails |
|---|---|
| `Transposes` | case 1: `ensure_pane(r2c1)` is `grid-out-of-range` because it became `r1c2`. Case 2: `r1c2` succeeded. Also cases 4, 5, 6, 8 and 9, at their `ensure_pane(r2c1)`. |
| `ReadsBackTransposed` | case 1: the snapshot lists `r1c2`. Also case 4. |
| `CreatesOnEveryEnsure` | case 3: the second ensure gave `w1:p2`. Also case 4. |
| `RebuildsTheWorkspace` | case 4: `r1c1` came back as `w1:p2`, renumbered when `r2c1` was made. Also case 2 (a refused ensure renumbered `r1c1`) and case 6 (`a` was renumbered before it was closed). |
| `IdsFromPosition` | case 5: the reused id `scratch:r2c1`. |
| `IdsFromSnapshotOrder` | case 6: `b`'s id `n1` is gone and the closed `n0` is listed. Also case 5. |
| `CloseIsIdempotent` | cases 7 and 8. |
| `ClosedPaneStillAnswers` | case 9. |
| `ReadIgnoresMaxLines` | case 10: 5 lines. |
| `HerdrVersion::Unsupported` | case 11 only. |

The unbroken wrapper and the split-only fake pass every case.

## Evidence appendix
`docs/handoffs/683/evidence.md`. It has fourteen facts from unchanged code, each with its `file:line` and a
verbatim excerpt:
- the fault switch's order (record, sleep, answer) and its wedged `Timeout { op }`;
- `run_cases` dropping the subject before the guard, and `expect_code`'s non-empty details;
- `RefusalCode::from_static`'s const check, and `class_of` making an open code a refusal;
- `GridPos`'s `Display` and its public fields;
- `Argv`'s `Hash`/`Eq` and `PaneId`'s `Ord`;
- the `Key` doc, the `HerdrPort::ensure_pane` doc and the `Prober` trait;
- ADR-0021's I4 row.

## Tests that look wrong (for T)
None. Every authored test passes against the brief's behaviour, unedited.
- The two details T flagged as pinned only implicitly agree with the implementation: `read` keeps a trailing empty
  line from `str::lines` (`"a\nb\nc\nls\n"`), and a vanished id is never reused (`w1:pE` after `w1:pA` vanished).
- T's RED handoff left clippy over the tests unproven. It is now clean under `-D warnings`.

## Known issues
None against the acceptance criteria. Items that are already recorded and are not F's:
- The nine `ASSUMPTION (#640)` comments hand their questions to #640. ASSUMPTION 9 includes the two `holler-pane` doc
  lines (`error.rs:414-416`, `ports.rs:127-128`) and, after A's warn 2, section 9's `profile apply` row.
- O's follow-ups from A's round 2 are still O's to do:
  - add the `profile apply` row to #640's follow-ups;
  - open the combined cleanup issue for the copied `CaseGuard`/`assert_suite_fails_on` after slices b to e merge.

## Files changed
Production and doc files only. The three test files are T's, and F did not edit them.
- `crates/holler-pane-testkit/src/herdr.rs`
- `crates/holler-pane-testkit/src/prober.rs`
- `crates/holler-pane-testkit/src/conformance/herdr.rs`
- `CHANGELOG.md`
- `docs/adr/ADR-0021.md`

Pipeline artifacts: `docs/handoffs/683/handoff-F.md`, `docs/handoffs/683/evidence.md`, and an F entry appended to
`docs/handoffs/683/decisions.md`.
