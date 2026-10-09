# Handoff-A-dup: Phase 7 - #640 part 1 of 3, the pure Herdr protocol and grid core  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Diff base:** 9d61c9f (origin/main)   **Diff head:** 22cef15 (F's code is 78f5274, on T-red's stubs at 09ada91)
**Reuse map:** docs/handoffs/640-brief.md §"Reuse map (extend, do not duplicate)"
**Verdict:** PASS

## Summary

PASS. F extended or mirrored every object the Reuse map named and built no parallel path:

- **Shared types.** Every error is a `holler_pane::PaneError` variant, and every cell, id and key is `holler_pane`'s
  `GridPos`, `PaneId` or `Key`. The one open code, `grid-unreachable`, goes through `RefusalCode::from_static`.
- **Testkit vocabulary.** `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` are declared once each in production. Tests pin
  both equal to the test kit's (AC 13). The message shapes match `FakeHerdr`'s.
- **The conversion.** `grid_of` is the spike's `derive()` walk. No second conversion rule exists.
- **The parallel copies.** The copies outside this crate are `last_lines` (the brief sanctions it), `count` and
  `excerpt`. Each has its original in a private item of a frozen or dev-only crate, and F declared each one in writing.

The three findings are all `warn`. They cover one helper copied from another crate and two small rules repeated within
this crate. None of them changes the pinned API.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-adapter-herdr/src/protocol.rs:580` | **`excerpt` is a second copy of `holler_pane::error::excerpt`** (`error.rs:688`). That function is `pub(crate)`, and `holler-pane` is frozen (#637) and outside this brief's blast radius, so making it public would be an amend-first API change. The copy is justified for part 1. It behaves the same: 64 characters, `{:?}` quoting and a trailing `...`. Its byte-length test after `take(64)` is equivalent to the original's character count. The risk: the host adapter (#641) and the OpenCode adapter (#642) will also quote what a peer sends in one-line messages, so the number of copies is likely to reach three. No test here pins the 64-character limit either: T-green's `EXCERPT_LIMIT` mutant survived. | No change in part 1. Record a follow-up: the next amend-first change to `holler-pane` (for example the Phase 3 W-9 `HerdrSnapshot` amendment) makes `error::excerpt` public, and each adapter then uses it instead of its own copy. Until then, #641 and #642 copy this one rather than write a third rule. |
| 2 | warn | `crates/holler-adapter-herdr/src/plan.rs:122-124, 209, 214` | **`Widths` repeats two of `layout.rs`'s private helpers.** It writes the row-to-index rule `usize::from(row).checked_sub(1)` twice, and the length saturation `u16::try_from(len).unwrap_or(u16::MAX)` once. These are `layout::index` (`layout.rs:215`) and `layout::saturate` (`layout.rs:220`). The layout module doc (`layout.rs:21-23`) says the base is applied in "exactly two places", which is now true of `layout.rs` alone. `Widths` itself is not a parallel path. It is built from `GridMap`'s public `rows()` and `cols_in()`, it never walks the tree again, and it stays in `GridPos` terms, with no Herdr conversion. | Make `layout::index` and `layout::saturate` `pub(crate)`, and call them from `Widths::add`, `Widths::width` and `Widths::rows`. Then the 1-based rule has one home in the whole crate, which is what the brief's "off-by-one impossible to merge" asks for. Part 2 edits this crate anyway, so it can make the change. |
| 3 | warn (low) | `crates/holler-adapter-herdr/src/protocol.rs:473-483` | **`direction()` keeps its own list of `Direction`'s variants**, `[Direction::Right, Direction::Down]`. That list is separate from `Direction::as_str` (`layout.rs:36-44`), so the two halves of the wire-name mapping sit in two modules. This is the same kind of problem as Phase 3 W-3, which F fixed for methods with `Method::ALL`. The risk is low, because Herdr has exactly two directions (spike §7). | Optional: add a `Direction::ALL` const next to `as_str` (the `PaneCode::ALL` and `Method::ALL` pattern) and have `direction()` use it. Part 2 can do this if it touches either file. |

The copies were checked and need no change. So was the drift that could have come in during F's work:

- **Reuse map row 1 (the port, its types, every error).** There is no new error, grid or key type.
  - The new public types are all pinned by the brief, and the workspace has nothing equivalent: `Direction`, `LayoutNode`,
    `GridMap`, `Extent`, `Target`, `Step`, `Request`, `ServerVersion`, `WorkspaceRef`, `PaneRef` and `SessionState`. I
    searched for each name.
  - `holler_proto` also has a `Direction`, a `log::Direction` and a `SessionState`, but they mean other things. This
    crate does not re-export them at its root (Phase 3 W-1).
  - `PaneRef` and `SessionState` are Herdr's own records: Herdr's ids, with no grid. They do not duplicate `HerdrPane`
    or `HerdrSnapshot`. Part 2 builds those from them.
- **Row 2 (`FakeHerdr`'s vocabulary).** The adapter's messages take the fake's shapes:
  - a refusal: `"<cell> cannot be reached: <why>"`;
  - a cell out of range: `"<cell> is outside the workspace, which is N rows by M columns"`;
  - an unsupported version: `"Herdr reports version …; the supported one is …"`.

  The test kit is a dev-dependency only (`Cargo.toml:22-25`).
- **Row 4 (the tree walk).** `grid_of` and `chain` follow `herdr-grid.sh:32-39` exactly: the same chain rule, and rows
  then slots, counted from 1. The one addition, which puts the panes of a nested slot in `unplaced`, is what AC 5 and
  AC 6 require. Nothing reads positions from the rectangles.
- **Row 5 (`last_lines`, `protocol.rs:592`).** The body matches the test kit's (`herdr.rs:522-525`) line for line. The
  brief sanctions this copy.
- **`count` (`plan.rs:262`).** It is a five-line copy of the test kit's private, dev-only `count` (`herdr.rs:498`). It
  keeps the range message's wording equal to `FakeHerdr`'s, and no production code has one to reuse. Justified.
- **Row 6 (the socket client).** There is no import of `holler-hub` or `holler-proto`, and no I/O. `decode_reply`
  reads Herdr's own wire: string error codes and `holler:<method>` ids. `holler_proto::Envelope` and `send_over` do not
  model that wire, and ADR-0021 §5 does not let an adapter depend on them.
- **Pattern consistency.**
  - `Method::ALL` and the `const`-built `ALLOWED_METHODS` copy the `PaneCode::ALL` and `CLOSED_CODES` pattern
    (`error.rs:72`, `error.rs:148`).
  - `Request`'s hand-written `Debug` follows `PaneName`'s (`pane.rs:56`).
  - The parsers read `Value` by hand, not through serde derives. The test kit's envelope checker does the same
    (`envelope.rs:42`). handoff-F gives the reason: a serde type error quotes the value, which for `pane_read` is a
    pane's screen.
- **No drift from F's work.**
  - F's commit changes only the five production files, `CHANGELOG.md` and the handoff documents. It changes no test.
  - The public items are the T-red stubs' items. Only two things changed: the `_` prefixes on the stubs' parameter
    names are gone, and `ALLOWED_METHODS` is built by a `const` block, still as `[&str; 9]`.
  - The new derives are on private types only.
  - The dependencies are the same as at T-red.
  - There is no `unsafe` and no `#[allow]` in `src/`.
  - The files are 39, 222, 268 and 595 lines long.
  - No other crate, ADR, golden file or protocol doc is touched.
  - There are no personal infrastructure names.
- **The Phase 7 checklist for this stack.** None of the listed candidates is touched or copied: the `token.rs`
  operations, `Lockout`, `Roster`, `log(Severity, …)`, and the test helpers `Hub`, `Body`, `mint_token`, `join`,
  `wait_for` and `StateDir`. The builders in `tests/common/mod.rs` build Herdr trees only, and nothing in the workspace
  does the same.

## Notes for F

PASS, so nothing is required. W-2 and W-3 are small edits within this crate that part 2 can make, since it edits this
crate anyway. W-1 is a follow-up for O and the MO, tied to the next amend-first change to `holler-pane`.
