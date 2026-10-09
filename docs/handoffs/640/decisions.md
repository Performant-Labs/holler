# Decisions — #640 herdr-adapter, part 1 of 3 (the pure Herdr protocol and grid core)

## A (Phase 3, up-front plan review) — 2026-10-09T14:52:10-06:00
- **Decided:**
  - **Verdict.** PASS on docs/handoffs/640-brief.md at 1571c6d (sha256 81fe0e9dd864e8c9…), with 14 warns and no block. The findings are in handoff-A.md.
  - **Shared types.** The plan extends `holler_pane`'s `PaneError`, `GridPos`, `PaneId` and `Key`, and adds no parallel error, grid or key type.
  - **Open code.** `grid-unreachable` is declared in its own file through `RefusalCode::from_static`, per ADR-0021 §9.
  - **Testkit vocabulary.** Mirrored and pinned equal by test (AC 13), the option the testkit's own `ASSUMPTION (#640)` offers.
  - **The conversion.** The spike's `derive()` walk is ported as the only conversion.
  - **Placement and dependencies.** The new modules are justified, because the crate is empty and ADR-0021 §10 and epic decision 7 name it the only home of the conversion. Dependencies point the right way: `holler-pane` only, the testkit as a dev-dependency, no `holler-hub` (ADR-0021 §5).
- **Assumed:**
  - **Run order.** Parts 2 and 3 run as outlined and #640 closes with part 3.
  - **Deferred ADR rows.** The ADR-0021 §9 and §10 rows can therefore wait for part 3: no verb can raise `grid-unreachable` until #649 wires the adapter, and #649 depends on #640.
  - **Decision 4.** "The extent is adapter configuration" is within the latitude ADR-0021 §10 gives #640.
- **Hedged:**
  - **Why warns, not blocks.** None of W-7 to W-14 changes part 1's pinned API. `GridMap::unplaced()`, `parse_pong`/`check_supported` and the multi-cell `Target` serve every resolution of them, so I did not block.
  - **W-7 (highest risk).** `HerdrPort::ensure_pane` takes one cell per call, so #664 cannot get a multi-cell plan. A row-major order passes on FakeHerdr's split-only mode and fails on real Herdr.
  - **W-9 (highest risk).** `HerdrSnapshot` cannot carry an unplaced pane. ADR-0021 §2's amend-first rule points at a contract PR before part 2, not only before #647.
  - **W-1 (naming only).** The root names `Direction` and `SessionState` collide with `holler_proto`'s root re-exports. Keeping them module-qualified fixes it without renaming a pinned item. Renaming `SessionState` is O's call and needs an amendment before T starts.
  - **W-11 (ownership).** Decision 8 is correct layering (I1), but it leaves `host.herdr_api_version` with no owner. ADR-0021 §1 line 40, the epic's contract, #640's Scope and pane.rs:112 all still say the adapter records it, and #644's issue does not take it on.
- **Evidence:**
  - **Brief.** Read in full.
  - **holler-pane.** src/{lib, ports, grid, error, pane, profile}.rs.
  - **holler-pane-testkit.** src/{lib, herdr}.rs, src/conformance/herdr.rs, tests/fake_herdr_test.rs.
  - **Other crates.** holler-proto src/lib.rs (root re-exports) and src/docs.rs:60; holler-hub src/control.rs:477-518 (`send_over`); holler-cli src/pane/wiring.rs.
  - **Docs and scripts.** ADR-0021 §1, §2, §5, §9, §10; docs/research/herdr-api-spike.md (all of it); scripts/spikes/herdr-grid.sh.
  - **Build config.** Cargo.toml `[workspace.lints]`, clippy.toml, scripts/lint.sh.
  - **Issues (via `gh`).** #640, #633 (epic), #638, #644, #647, #649, #650, #664, #683.
  - **Duplicate search.** grep for existing `Direction`, `Extent`, `GridMap`, `LayoutNode`, `last_lines`, `plan_splits`, `excerpt`/`one_line` helpers. The only overlaps are the testkit-sanctioned constants and the private `last_lines`, both justified in the brief.

## T (Phase 4, author / RED) — 2026-10-09T15:20:00-06:00
- **Decided:**
  - **Suite shape.** Three files cover AC 1-20: `layout_test.rs` (10 tests), `plan_splits_test.rs` (19 tests) and `protocol_test.rs` (33 tests). A shared `tests/common/mod.rs` holds the tree builders. All are unit-level with no I/O.
  - **Stubs and Cargo.toml.** The stubs (`layout.rs`, `plan.rs`, `protocol.rs`, `lib.rs`) hold only pinned signatures and constants, per Decision 12. `Cargo.toml` gets `holler-pane` and `serde_json` as dependencies and the testkit as a dev-dependency. `serde` is left out until F's code uses a derive (A, W-6).
  - **A's warns.** W-1 to W-6 are settled as listed in handoff-T-red.md: no flat re-exports; a no-wildcard variant match for the allow-list; `is_empty()` false when any pane exists; `pane_not_found` without a pane is `Unavailable`; `Request` never prints typed text through `Debug`; test headers end `// #640`.
  - **Message wording.** Only the substrings the brief states are pinned: the cell name, "2 rows by 1 column", "one pane" for 11c, "unknown" for a missing protocol, the version and `SUPPORTED_VERSIONS`.
- **Assumed:**
  - **Ratio denominator.** `1/(cols - c + 2)` uses the extent's column count. The tests pin only cases where the extent and the row agree.
  - **Where the duplicate-label error comes from.** The brief's AC 18 sentence puts it on `SessionState::workspace`, not on `parse_snapshot`, so the test expects `parse_snapshot` to succeed.
  - **Split's `pane_not_found`.** It names the split target, the only pane id in that request.
- **Hedged:**
  - **Constant-only passes.** Five tests pass at RED: the two constant checks, the allow-list's forbidden-names check, the protocol/versions agreement, and `GridMap::default()`. They pin values the stubs already hold, as the brief expects for AC 13.
  - **Planner tests and `grid_of`.** The planner tests build existing maps with `grid_of`, so a `grid_of` bug fails them too. That is by necessity (`GridMap` has no other constructor), and the layout tests catch `grid_of` first.
  - **Fixture strictness.** The layout export fixtures carry every field the schema lists, so a strict F parser is not penalized for the fixture. The `root_pane` and `pane` fixtures carry only `pane_id`, `workspace_id` and `tab_id`, since the brief names those alone.
- **Evidence:**
  - `cargo test -p holler-adapter-herdr --no-fail-fast`: layout 1 passed and 9 failed; plan_splits 1 passed and 18 failed; protocol 3 passed and 30 failed.
  - Clippy, machete, lint.sh and changelog-check are green on the RED tree.
  - All files read: brief, handoff-A, holler-pane `error.rs`, `ports.rs`, `pane.rs`, `grid.rs`, the testkit's `herdr.rs`, the spike §5-§7 and §13 passages, and `Cargo.toml` and `clippy.toml`.

## F (Phase 6, implement) — 2026-10-09T15:21:37-06:00
- **Decided:**
  - **The conversion.** `grid_of` ports the spike's `derive()` walk exactly, into rows of slots. The order is applied in
    `grid_of`'s two `chain` calls and in `cell_at`. The base is applied in `number` and `index`. A pane past `u16::MAX`
    is unplaced, never dropped.
  - **The planner.** Range first, then the cells that exist, then 11d, then a rows-first simulation over row widths. A
    step only ever appends: to the end of a row, or below the last row when it holds one pane. A refusal returns no
    plan. The ratios use the extent's `rows` and `cols`.
  - **r2c2 below a row of two is 11c ("one pane"), not a gap.** It is in a new row, and a new row can only come from a
    `down` split under a row of one pane. I checked it and kept T's test as written.
  - **A's warns in code:**
    - W-2: a hand-written `Debug` for `Request`, which shows `SendText` as `<N bytes>`.
    - W-3: one private `Method` table builds both `ALLOWED_METHODS` and `Request::method`.
    - W-4: `is_empty()` means no row, which means no pane, and `CreateRoot` comes only from an empty grid.
    - W-5: `pane_not_found` for a request with no pane is `unavailable`.
    - W-6: the parsers walk `Value` by hand, with no `serde` dependency. That also keeps serde's value-quoting type
      errors, which could quote a pane's screen, out of every message.
    - W-13: the grid-tab rule and the way it fails are documented in `protocol.rs`.
  - **Messages** are one line. Herdr's code, the result type, the version, the label and workspace ids are quoted by a
    private `excerpt` (`{:?}`, at most 64 characters). Herdr's `message` is never echoed.
- **Assumed:**
  - **Version and protocol.** A pong `protocol` that is not a `u32` is unknown (`None`, refused by the gate), not
    garbled. A missing `version` is garbled (`unavailable`).
  - **Ties.** On a tie for the lowest tab `number`, the first tab in snapshot order wins.
  - **Wire depth.** Part 2 parses wire lines only through `decode_reply`, whose `serde_json::from_str` limits recursion
    to 128, so the recursive tree parse and walk are bounded.
- **Hedged:**
  - **Three private helpers are duplicated.** `last_lines` is sanctioned by the brief. `excerpt` copies
    `holler_pane::error::excerpt`, which is `pub(crate)` in a frozen crate. `count` copies the test kit's private,
    dev-only helper and keeps the range wording equal. Each is small and cannot be imported. A judges them at Phase 7.
  - **No flat re-exports.** The brief asks for re-exports, but A's W-1 and T-red settled on none.
  - **`archChanged` is reported false.** The public surface is exactly T-red's pinned stubs (no new public item or
    derive), I added no module, and no dependency changed.
- **Evidence:**
  - `cargo test -p holler-adapter-herdr`: layout 10/10, planner 19/19, protocol 33/33 (RED was 1/10, 1/19, 3/33).
  - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
  - `cargo test --workspace --no-fail-fast`: 118 targets, 1317 passed, 0 failed, 5 ignored (the existing ones).
  - `rustfmt --check --edition 2021`: clean. `cargo machete`: clean. `lint.sh`: exit 0. `changelog-check`: ok.
  - Read: the brief, handoff-A, handoff-T-red, the spike (all of it), `herdr-grid.sh`, holler-pane
    `lib`/`error`/`grid`/`pane`/`ports`, the testkit's `herdr.rs` and `conformance/herdr.rs`, and
    `control.rs` `send_over`.
  - The facts are in `evidence.md` (12 entries).

## T (Phase 7, verify GREEN + Tier 2)

- **Decided:** PASS. The suite is GREEN, there is no blocking Tier 2 issue, and no test needed repair (F reported none).
- **Assumed:** the 64-character excerpt cut is an implementation detail, not an acceptance criterion, so a surviving mutant
  there is advisory and not a reason to add a test or to block.
- **Hedged:** none.
- **Evidence:**
  - The adapter tests 62/62; clippy exit 0; `cargo test --workspace` 118 targets, 1317 passed, 0 failed, 5 ignored;
    lint, changelog-check, machete, `docs_cli_test` and `wire_selftest` all pass.
  - Four of five mutants were caught: transposition, count-from-0, the `SendText` `Debug` leak and the ratio denominator.
    The `EXCERPT_LIMIT` mutant survived.

## A (Phase 7, anti-duplication) — 2026-10-09T15:34:15-06:00
- **Decided:**
  - **Verdict.** PASS on 9d61c9f..22cef15, with 3 warns and no block. The findings are in handoff-A-dup.md.
  - **No parallel path.** F used `holler_pane`'s `PaneError`, `GridPos`, `PaneId`, `Key` and `RefusalCode::from_static`.
    It mirrored `FakeHerdr`'s constants and message shapes, ported the spike's `derive()` walk as the only conversion,
    and imported nothing from `holler-hub` or `holler-proto`.
  - **The copied helpers.** `last_lines` is sanctioned by the brief. `count` copies a private, dev-only test kit helper.
    `excerpt` copies `holler_pane::error::excerpt`, which is `pub(crate)` in a frozen crate. All three are justified
    and declared in handoff-F. `excerpt` is W-1, a follow-up for the next amend-first change to `holler-pane`.
  - **Within the crate.** W-2: `Widths` in `plan.rs` repeats `layout::index` and `layout::saturate`. W-3:
    `protocol::direction` keeps its own list of `Direction`'s variants. Both are small, and part 2 can fix them.
- **Assumed:**
  - **Amend-first.** Making `holler_pane::error::excerpt` public counts as an API change to the frozen contract crate.
    So it is amend-first and out of part 1's scope, not a drive-by edit.
- **Hedged:**
  - **W-1 and later adapters.** I expect #641 and #642 to need the same quoting helper. That is a forecast, not
    something in this diff, so W-1 is a warn.
- **Evidence:**
  - **Read in full:** `src/{lib,layout,plan,protocol}.rs`, `Cargo.toml`, `tests/common/mod.rs` and the CHANGELOG and
    Cargo.lock diffs.
  - **The analogous objects:** the test kit's `herdr.rs` (all of it), `envelope.rs` (header), and `holler-pane`'s
    `error.rs`, `grid.rs`, `lib.rs`, `ports.rs` and `reply.rs`. Also `holler-hub`'s `control.rs` (`send_over`) and
    `herdr-grid.sh:32-39`.
  - **The diff checks:** the stubs and the final code have the same public lines and the same public derives. F's commit
    touches no test.
  - **The duplicate searches:** `excerpt`/`one_line`, `count`/`plural`, `last_lines`, every new type name, and JSON
    field readers across `crates/`.
  - **Hygiene:** a grep for personal infrastructure names, and for `unsafe` and `#[allow]` in `src/`.
