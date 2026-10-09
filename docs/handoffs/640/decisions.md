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
