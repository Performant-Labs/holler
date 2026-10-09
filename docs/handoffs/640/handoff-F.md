# Handoff-F: Phase 6 - #640 part 1 of 3, the pure Herdr protocol and grid core

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Issue:** #640 (epic #633). The PR says `Part of #640`, not `Closes #640`.

| Field | Value |
|-------|-------|
| GitHub issue | #640, part 1 of 3 |
| Working branch | `issue-640-implementation` (worktree `.claude/worktrees/0640-herdr-adapter`) |
| Build plan phase | Part 1, the pure core: `layout.rs`, `plan.rs`, `protocol.rs`. No I/O. |
| Input documents read | `docs/handoffs/640-brief.md`; `handoff-A.md`; `handoff-T-red.md`; `decisions.md`; `docs/research/herdr-api-spike.md` (all of it); `scripts/spikes/herdr-grid.sh`; `holler-pane` `lib.rs`, `error.rs`, `grid.rs`, `pane.rs`, `ports.rs`; the testkit's `herdr.rs` and `conformance/herdr.rs`; `holler-hub` `control.rs` (`send_over`); `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `CHANGELOG.md` |
| Acceptance criteria count | 22 (AC 1-20 are tests; AC 21 gates; AC 22 CHANGELOG) |
| Handoff document path | `docs/handoffs/640/handoff-F.md` (this file) |

Scope check: 5 production files in one crate, plus `CHANGELOG.md`. That is inside the cap, so no split was proposed.
This ran under the Workflow script, so the table above is recorded here rather than put to a human.

## What was done

- `crates/holler-adapter-herdr/src/layout.rs`: the stub bodies are filled.
  - `Direction::as_str`.
  - `GridMap` as rows of slots (a private `Slot`: a pane, or unplaced) plus the unplaced panes in tree order.
  - `grid_of` ports the spike's `derive()` walk exactly: the root's `down` chain gives the rows, and each row's `right`
    chain gives its slots.
  - The `_stub` field and the stub comments are gone.
- `crates/holler-adapter-herdr/src/plan.rs`: `plan_splits`. It deduplicates and orders the target, checks the range
  first, answers the cells that exist, refuses a non-grid map, and then simulates rows-first over the width of each
  row. That gives the spike's 2x4 recipe and the `grid-unreachable` refusals of AC 11a to 11d.
- `crates/holler-adapter-herdr/src/protocol.rs`:
  - Requests: a private `Method` table that both `ALLOWED_METHODS` and `Request::method` are built from, `id`,
    `to_line` and `params` with the schema's field names, and a hand-written `Debug` that shows a `SendText`'s length,
    never its text.
  - `decode_reply`, the version gate, and the six parsers. These read through a small private `Object` reader, so every
    message is one line and quotes only what it must.
  - The module docs, which include the grid-tab rule and the way it fails (A, W-13).
- `crates/holler-adapter-herdr/src/lib.rs`: module docs, which state the conversion rule and the split model in prose.
  There are still no flat re-exports.
- `crates/holler-adapter-herdr/Cargo.toml`: the description is updated, since the crate is no longer "empty in the
  workspace skeleton". The dependencies are as T-red left them. `serde` was not added (A, W-6).
- `CHANGELOG.md`: one `[Unreleased]` / `### Enhancements` entry, linking #633 and #640.
- `docs/handoffs/640/evidence.md`: new, with 12 facts from unchanged code.

## Design decisions

- **One conversion, applied in named places.**
  - **The order** is applied in `grid_of`: the root's `down` chain gives rows, and each row's `right` chain gives
    slots. `cell_at` then builds `GridPos { row, col }`.
  - **The base** is applied in `number`, which turns a 0-based walk index into a row or column counted from 1, and in
    its inverse, `index`. `at`, `cols_in`, `cells` and `position_of` all go through these.
  - **For T's AC 2 mutants:** the transposition mutant swaps the two `Direction`s in `grid_of`, and the counting-from-0
    mutant drops the `+ 1` in `number`.
- **Rows of slots, not a flat list.** This mirrors the walk and the brief's description. A nested slot keeps its place
  in the row, so later columns keep their numbers (AC 5), and `at` is a direct index.
- **No pane is invisible.**
  - A pane at a row or column past `u16::MAX` cannot be a `GridPos`, so `grid_of` lists it as unplaced rather than
    dropping it. It cannot happen on a real terminal, but it keeps the function total.
  - `rows` and `cols_in` saturate at `u16::MAX`.
- **`is_empty()` means no pane at all (A, W-4).** The map holds a row exactly when it holds a pane. `plan_splits` emits
  `CreateRoot` only when the grid it builds on has no row. A map whose panes are all unplaced, as in AC 6, is refused
  under 11d before that point, so it can never become `CreateRoot`.
- **The planner never makes a step that could move a pane.**
  - A `right` split is planned only from the last pane of a row: `left.col == width`.
  - A `down` split is planned only from the last row, and only when that row holds one pane.
  - Any other new cell is refused, so a step can only append. An impossible state (a cell that already exists) is
    refused too; it is never turned into a step.
  - `collect::<Result<Vec<_>, _>>()` stops at the first refusal, so there is never a partial plan (AC 11).
- **The order of checks:**
  1. Range, over every target cell (AC 10).
  2. Cells that exist are answered (AC 9).
  3. Any unplaced pane refuses every new cell (11d).
  4. The empty-workspace rule (11a).
  5. The per-cell rules: a gap (11b) or a crowded row above (11c).

  Cells are sorted by `(row, col)` before the checks, so an error never depends on the input order.
- **r2c2 below a row of two is 11c, not a gap (T's open question).** I checked this rather than weaken the test. r2c2
  sits in a row that does not exist yet. A new row can only come from a `down` split under a row of one pane, and that
  row has two, so 11c's reason is the accurate one. The only other way to reach r2c2, a `down` split of r1c2, nests
  for the same reason. So `below()` checks the crowded row before the "needs r2c1" gap. For a row of one pane, r2c2
  without r2c1 is a gap that names r2c1.
- **The ratio denominators use the extent's `rows` and `cols`, as T's note asks.** `share(size, n)` is
  `1 / (size - n + 2)`: the split pane spans `size - n + 2` equal parts and keeps one. Range-checking first guarantees
  a denominator of at least 2.
- **One method table (A, W-3).** `ALLOWED_METHODS` is built in a `const` block from `Method::ALL`, and
  `Request::method` goes through `Request::kind() -> Method`. This is the same single-source pattern as `PaneCode::ALL`
  in `holler-pane/src/error.rs`. A request cannot name a method that is not on the list. The pinned constant keeps its
  type, `[&str; 9]`, and its values.
- **Typed text never reaches a message (A, W-2; AC 16).**
  - `Request`'s `Debug` is hand-written, and `SendText` shows as `text: <N bytes>`.
  - `decode_reply` never quotes Herdr's `message`, or anything from a `result`.
  - Herdr's code, the result `type`, a version string, a label and workspace ids are quoted through a private
    `excerpt` (Rust's `{:?}`, cut to 64 characters). A garbled field therefore cannot add a newline or grow the message
    (the `"odd\ncode"` case).
- **The parsers walk `serde_json::Value` by hand, not through a serde derive (A, W-6).** This keeps the `serde`
  dependency out. It also matters for secrecy: serde's own type errors quote the offending value (`invalid type:
  string "..."`), and for `pane_read` that value would be the pane's screen. Every message comes from the `Object`
  reader, with the form `Herdr's <what> has no usable <key>`.
- **What `decode_reply` accepts and refuses.**
  - The `id` must equal `request.id()`. A reply for another request is garbled.
  - The reply must hold exactly one of `result` (which must be an object) and `error`. Both, neither, or a non-object
    `result` is `unavailable`.
  - `pane_not_found` is `PaneNotFound { what: <the request's pane id> }` only when the request names a pane:
    `Split` (its target), `SendText`, `SendKeys`, `Read` or `Close`. Otherwise it is `unavailable` naming the method
    (A, W-5).
- **The pong.**
  - A missing `protocol`, or one that does not fit a `u32`, is `protocol: None`. The gate refuses that as "an unknown
    protocol" (Decision 8: unsupported, not garbled).
  - A missing `version` is `unavailable`, because the schema requires it and `HerdrPort::version` must return a
    string.
- **Duplicate workspace labels.** `SessionState::workspace` collects every workspace with the label and, when there is
  more than one, names all of their ids, not only two. `parse_snapshot` itself accepts the duplicates (T's reading of
  AC 18).
- **The grid tab is the tab with the lowest `number`.** On a tie (not expected: `number` is a per-workspace ordinal),
  `min_by_key` keeps the first in snapshot order.

## Reuse / extend-vs-new

- **Reused, not paralleled:** `holler_pane`'s `PaneError`, `GridPos`, `PaneId`, `Key`, and `error::RefusalCode`
  through `from_static`. There is no new error, grid or key type. The open code `grid-unreachable` is declared once,
  in `plan.rs`.
- **Vocabulary mirrored from `FakeHerdr`, the closest analogous object:**
  - `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` equal the test kit's (AC 13).
  - The refusal shape is `"<cell> cannot be reached: <why>"`.
  - The range wording is `"<cell> is outside the workspace, which is 2 rows by 1 column"`, `FakeHerdr`'s shape without
    a workspace name, which `plan_splits` does not have.
  - The test kit is a dev-dependency only.
- **Ported exactly:** the spike's `derive()` walk, as `grid_of` plus `chain`.
- **New modules:** the brief's own justification applies. The crate was empty, and it is the only allowed home of the
  conversion (epic decision 7, ADR-0021 section 10). T-red created the module boundaries as stubs. I added no module.
- **Three small private helpers duplicated, each for a stated reason. A should judge these at Phase 7:**
  - `last_lines` (`protocol.rs`): the brief sanctions this. The test kit's copy is private and dev-only, and the
    behaviour is identical (AC 20, `evidence.md`).
  - `excerpt` (`protocol.rs`): `holler_pane::error::excerpt` does the same job but is `pub(crate)`, and `holler-pane`
    is frozen (#637), so making it public is an amend-first change outside this story. This copy differs only in
    style: it compares byte lengths after taking 64 characters.
  - `count` (`plan.rs`): the test kit's `count` is private and dev-only. Mirroring it keeps the "2 rows by 1 column"
    wording equal to `FakeHerdr`'s.

## Architecture notes for A

- **Layers touched:** `holler-adapter-herdr` only, plus `CHANGELOG.md`. No other crate, no ADR, no `holler-pane`
  change, and no change to the test kit.
- **The public surface is exactly the pinned API of the T-red stubs.**
  - Same items, same signatures, and the same derives (`GridMap`: `Debug, Clone, Default, PartialEq`).
  - `Request` keeps `Clone` and still implements `Debug`, now by hand (W-2).
  - Every new type is private: `Slot`, `Method`, `Widths`, `Object` and `TabRef`.
- **Dependencies:** unchanged from T-red. That is `holler-pane` and `serde_json`, with `holler-pane-testkit` as a
  dev-dependency. Inside the crate, `plan` and `protocol` each use `layout`, and there is no cycle.
- **No I/O, no `unsafe`, and no `#[allow]` in `src/`.**
- **Recursion depth:** the parse of a layout and `grid_of` each recurse once per tree level. Part 2 must keep parsing
  wire lines with `serde_json::from_str`, inside `decode_reply`, whose default recursion limit of 128 bounds the depth
  of any tree that comes from Herdr.

## Deviations from spec / wireframe

- **No flat re-exports in `lib.rs`.** The brief says "re-exports of the names below", but A's W-1 found that
  `Direction` and `SessionState` collide with `holler_proto`'s root re-exports. T-red settled on no flat re-exports, as
  in `holler_pane_testkit`, and the tests use module paths. I kept that rather than add a partial set.
- **`ALLOWED_METHODS` is built, not written out.** It is computed in a `const` block from the private method table
  (W-3). Its type and value are the pinned ones.
- There is no wireframe (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

These ran in the worktree at the final source.

`cargo test -p holler-adapter-herdr --no-fail-fast`. At RED this was layout 1/10, planner 1/19 and protocol 3/33:

```
     Running tests/layout_test.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/plan_splits_test.rs
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/protocol_test.rs
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo clippy --workspace --all-targets -- -D warnings`, with the adapter crate re-checked after a `touch`:

```
    Checking holler-adapter-herdr v0.4.0 (.../crates/holler-adapter-herdr)
    Finished `dev` profile [unoptimized + debuginfo] target(s)
exit=0
```

The other gates:

```
rustfmt --check --edition 2021 crates/holler-adapter-herdr/{src,tests}/**/*.rs   -> clean (no output)
cargo machete                       -> "cargo-machete didn't find any unused dependencies in this directory. Good job!"
bash scripts/lint.sh                -> exit 0; the only adapter line is "warn: .../tests/protocol_test.rs is 678 lines" (T's file)
bash scripts/changelog-check.sh     -> changelog-check: ok
grep unsafe / #[allow] in src/      -> none
wc -l src/*.rs                      -> layout 222, lib 39, plan 268, protocol 595 (all under the 600 warning)
```

`cargo test --workspace --no-fail-fast` (no other crate depends on this one, so the run checks only that nothing else
broke):

```
118 test targets; 1317 passed, 0 failed, 5 ignored (the existing #[ignore]s); exit=0
```

## Evidence appendix

`docs/handoffs/640/evidence.md` has 12 facts:

- `GridPos`: it has no `Ord`, and its `Display` form.
- `class_of` for an open code.
- `PaneError`'s `Display` arms.
- `holler-pane`'s `pub(crate)` `excerpt`.
- The test kit's constants, `last_lines` and `count`.
- `Key::as_str` and `PaneId::as_str`.
- The spike's walk and its 2x4 recipe.
- The ratio and nesting rules.

## Tests that look wrong (for T)

None. T's open question about r2c2 below a row of two is answered above: 11c's "one pane" is the accurate reason, and
the test holds as written.

## Known issues

- **A ratio that is not finite** would serialize as JSON `null`, which Herdr's optional `ratio` most likely reads as
  absent. Only a hand-built `Request::Split` can carry one: `plan_splits` always produces a value in `(0, 1/2]`. Not
  reachable in this part.
- **The grid tab depends on tab order (W-13).** It is the tab with the lowest `number`, so reordering a workspace's tabs
  changes it. This is documented in `protocol.rs`. Part 2 decides whether to refuse a multi-tab workspace.
- **A's W-7 to W-14 are not part 1's.** They are forward-compat items for the part 2 and 3 briefs and for #644, #647,
  #649 and #664. Nothing here forecloses them: `GridMap::unplaced()`, the multi-cell `Target`, and
  `parse_pong`/`check_supported` serve every resolution A listed.

## Files changed

- `crates/holler-adapter-herdr/Cargo.toml`
- `crates/holler-adapter-herdr/src/lib.rs`
- `crates/holler-adapter-herdr/src/layout.rs`
- `crates/holler-adapter-herdr/src/plan.rs`
- `crates/holler-adapter-herdr/src/protocol.rs`
- `CHANGELOG.md`

Handoff artifacts (not production): `docs/handoffs/640/handoff-F.md`, `docs/handoffs/640/evidence.md`, and the F entry
in `docs/handoffs/640/decisions.md`.
