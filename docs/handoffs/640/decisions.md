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
