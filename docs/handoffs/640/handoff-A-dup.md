# Handoff-A-dup: Phase 7 - #640 part 1 of 3, the pure Herdr protocol and grid core  (anti-duplication gate, round 2 after S REWORK)

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Diff base:** 9d61c9f (origin/main, the full feature); rework delta a5e0fef..df48467   **Diff head:** df48467
**Reuse map:** docs/handoffs/640-brief.md §"Reuse map (extend, do not duplicate)"
**Verdict:** PASS

This round replaces the round-1 gate. Round 1 passed at 22cef15, was committed as d9e19e3, and stays in git history.
Its three warns still stand, because this rework touched none of the code they cite. They are restated below as W-1 to
W-3.

This pass reviews the delta from S's test-only REWORK. That delta changes one file of code,
`crates/holler-adapter-herdr/tests/protocol_test.rs` (+2/-1), plus the T-green handoff and the journal. The production
code is still exactly F's: `git diff 78f5274 HEAD -- crates/holler-adapter-herdr/src crates/holler-adapter-herdr/Cargo.toml
Cargo.lock CHANGELOG.md` is empty.

## Summary

PASS. The rework adds no production code, helper, type or dependency.

- **What changed.** One assertion could not fail: `contains("99")` was already implied by `contains("99.0.0-fake")`. It
  is now `contains("protocol 99")`, which pins the wording that `check_supported` already produces (`protocol.rs:323`
  and `:328`). That narrows an existing assertion and adds no structure, so it cannot create a parallel path.
- **What I re-checked.** I ran the round-1 duplicate checks again against HEAD, and against the two newer commits on
  origin/main (c76bbed and cd635c0). Neither changes the picture. Those commits touch neither `holler-pane`, the test
  kit's Herdr fake nor this crate. They also add no public helper that could replace the crate's three private copies.

There is no new finding. The three warns are carried over from round 1.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn (carried, round 1 W-1) | `crates/holler-adapter-herdr/src/protocol.rs:580` | **`excerpt` is a copy of `holler_pane::error::excerpt`** (`error.rs:688`). The original is `pub(crate)` in the frozen contract crate, so the copy is justified for part 1. They behave the same: 64 characters, `{:?}` quoting and a trailing `...`. The copy's byte-length test after `take(64)` is equivalent to the original's character count. Two risks remain. No test pins the 64-character limit (T-green's `EXCERPT_LIMIT` mutant survived). And the host adapter (#641) and the OpenCode adapter (#642) are likely to make a third and fourth copy. No issue tracks this yet (`gh issue list --search excerpt`: none). | No change in part 1. The next amend-first change to `holler-pane` makes `error::excerpt` public, and each adapter calls it instead of keeping a copy. That change could be the Phase 3 W-9 `HerdrSnapshot` amendment. Until then, #641 and #642 copy this one rather than write a third rule. O or the MO files the follow-up or adds it to that amendment's scope. |
| 2 | warn (carried, round 1 W-2) | `crates/holler-adapter-herdr/src/plan.rs:122-124, 209, 214` | **`Widths` repeats two of `layout.rs`'s private helpers.** The row-to-index rule `usize::from(row).checked_sub(1)` appears twice, and the length saturation `u16::try_from(len).unwrap_or(u16::MAX)` once. These are `layout::index` (`layout.rs:215`) and `layout::saturate` (`layout.rs:220`). The module doc at `layout.rs:21-23` says the base is applied in "exactly two places", which is true of `layout.rs` alone. `Widths` is not a parallel path. It is built from `GridMap`'s public `rows()` and `cols_in()`, does not walk the tree again, and does no Herdr conversion. | Make `layout::index` and `layout::saturate` `pub(crate)`, and call them from `Widths::add`, `Widths::width` and `Widths::rows`. Then the 1-based rule has one home in the crate. Part 2 edits this crate anyway, so it can make the change. |
| 3 | warn (low, carried, round 1 W-3) | `crates/holler-adapter-herdr/src/protocol.rs:473-483` | **`direction()` keeps its own list of `Direction`'s variants**, `[Direction::Right, Direction::Down]`, apart from `Direction::as_str` (`layout.rs:36-44`). The two halves of the wire-name mapping therefore sit in two modules. The risk is low, because Herdr has exactly two directions (spike §7). | Optional: add a `Direction::ALL` const next to `as_str`, following the `Method::ALL` and `PaneCode::ALL` pattern, and have `direction()` use it. Part 2 can do this if it touches either file. |

No duplication, and the extension is clean. The rework introduced no architectural drift. Evidence:

- **The rework delta (`protocol_test.rs:397-400`).**
  - **The change.** It adds the assertion `message.contains("protocol 99")` and a one-line comment.
  - **Why it cannot pass by accident.** The assertion can match only `check_supported`'s protocol clause. The version
    is quoted (`"99.0.0-fake"`) and follows the word "version", and `SUPPORTED_VERSIONS` reads "protocol 22".
  - **What it adds.** It checks the production wording and does not re-implement the gate. That is the same shape as
    the test-only rework in the #508 run.
  - **What it leaves alone.** No helper, fixture or builder is added. `tests/common/mod.rs` is unchanged since T-red
    (`git diff --stat 09ada91 HEAD -- crates/holler-adapter-herdr/tests` lists only this one file).
  - **Size.** The file is 679 lines, up from 678, so it stays under the stack's ~800-line flag.
- **Reuse map row 2, the vocabulary mirror.**
  - **The frame.** The adapter's refusal uses `FakeHerdr`'s frame (`holler-pane-testkit/src/herdr.rs:321-324`): "Herdr
    reports version …; the supported one is {SUPPORTED_VERSIONS}". It adds the one clause that AC 17 requires, "with
    protocol N".
  - **The difference.** The rework pins that clause. It is the only place the two messages differ, and the difference
    is older than this round.
  - **The constants.** No shared constant changes, and AC 13 still pins `SUPPORTED_VERSIONS` and `GRID_UNREACHABLE`
    equal to the test kit's.
  - **For verb tests.** They should assert the code `herdr-version-unsupported`, not the wording.
- **Re-verified at HEAD (unchanged since round 1).**
  - **Shared types.** Every error is a `holler_pane::PaneError` variant. Every cell, id and key is `GridPos`, `PaneId` or
    `Key`. The one open code goes through `RefusalCode::from_static` (`plan.rs:39`).
  - **The conversion.** `grid_of` and `chain` (`layout.rs:145-185`) are the spike's `derive()` walk. They match the
    brief's verbatim copy of `herdr-grid.sh:32-39`. Nothing reads the rectangles.
  - **New public items.** None of them exists elsewhere in `crates/`: `LayoutNode`, `GridMap`, `Extent`, `Request`,
    `ServerVersion`, `WorkspaceRef`, `PaneRef`, `grid_of`, `plan_splits`, `decode_reply` and every `parse_*`.
  - **Name-only collisions.** These carry unrelated meanings:
    - `holler_proto::methods::Direction` is a circuit side;
    - `holler_proto::log::Direction` is a wire-event direction;
    - `holler_proto::docs::SessionState` is an A2A session state;
    - `holler_cli::cli::Target` is a CLI target string;
    - a `Step` type alias in the test kit's `conformance/profile_store.rs`.

    The crate re-exports nothing at its root (`lib.rs:32-35`), so none of them clashes.
  - **The three private copies.** Their only originals are `holler_pane::error::excerpt` (`pub(crate)`), and the test
    kit's private, dev-only `count` (`herdr.rs:498`) and `last_lines` (`herdr.rs:522`).
    - The bodies of the two `last_lines` are equal, and the brief sanctions that copy.
    - `count` takes a `usize` where the original takes a `u16`, with the same wording.
    - A workspace-wide search for quote-and-cut, plural-count and last-N-lines helpers finds no public original. The one
      other quote-and-cut site, `holler-hub/src/holds.rs:222`, trims a reason without quoting it, and it lives in a crate
      that an adapter may not depend on (ADR-0021 §5).
  - **The stack's Phase 7 candidates.** None is touched or copied in `src/` or `tests/`: the `token.rs` operations,
    `Lockout`, `Roster`, `log(Severity, …)`, `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`. The
    builders in `tests/common/mod.rs` have no public equivalent in `holler-pane` or `holler-pane-testkit`.
  - **Hygiene.**
    - There is no `unsafe` or `#[allow]` in `src/`.
    - No test sleeps, opens a socket or spawns a process or thread.
    - The production files are 39, 222, 268 and 595 lines long.
    - The dependencies are `holler-pane` and `serde_json`, with the test kit as a dev-dependency only. There is no
      `holler-hub` or `holler-proto`.
- **origin/main since the merge base, 9d61c9f.**
  - **What the newer commits change.** c76bbed and cd635c0 change the hub's profile registry, the test kit's profile
    scope, ADR-0021 and the CHANGELOG. The ADR-0021 change is one `pane-in-other-profile` sentence in the spec-editing
    flow.
  - **What they leave alone.** They do not touch `crates/holler-pane/`, `holler-pane-testkit/src/herdr.rs`,
    `conformance/herdr.rs` or this crate.
  - **Helpers.** `git grep` on origin/main finds no new public `excerpt`, `count` or `last_lines`.
  - **Result.** Merging this branch creates no new duplicate.

## Notes for F

None, since the verdict is PASS. W-2 and W-3 are small edits within this crate, and part 2 can make them. W-1 is a
follow-up for O and the MO, tied to the next amend-first change to `holler-pane`.

## Patterns referenced

- `crates/holler-pane/src/error.rs:688`: `excerpt`, `pub(crate)`.
- `crates/holler-pane-testkit/src/herdr.rs:36, 51, 321-324, 498, 522`: the shared constants, the version-refusal
  wording, `count` and `last_lines`.
- `docs/handoffs/640-brief.md` §Evidence: the verbatim `herdr-grid.sh:32-39` walk, and the Reuse map.
- `docs/handoffs/508/handoff-A-dup.md`: this repo's precedent for a round-N gate after a test-only S REWORK.
