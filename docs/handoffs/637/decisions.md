# Decisions — #637 pane-control skeleton

## A (Phase 3, up-front plan review) — 2026-10-08T21:15:21-06:00
- **Decided:** BLOCK on docs/handoffs/637-brief.md at 8fd92f8, with 6 blocks and 11 warns (see handoff-A.md).
  - The overall shape is right: a new `holler-pane` domain crate, stubs per verb, `CATALOG` kept at 22, clap structs moved out of `cli.rs`, and verbs run CLI-side against ports.
  - The six blocks are plan defects that T and F cannot fix inside the brief's blast radius:
    1. The `main.rs` dispatch and exit point is unlisted, and `main()` is at 99 of clippy's 100 deny-level lines.
    2. Decision 7 puts error codes in the frozen `vocab.rs` but names only 5, while the epic and its siblings name 8+ more, and #660's fixed text puts codes in each verb's file.
    3. Decisions 5 and 6 contradict each other on `holler-pane`'s dependencies, because `SessionName` lives in holler-proto.
    4. The hub hook is never reached: `dispatch_control` refuses non-`control/` methods, no state plumbing exists from serve.rs, and the hub `Cargo.toml` lacks `holler-pane`, which neither #639 nor #661 may add.
    5. AC 4's `say --pane X TEXT` cannot parse without a positional redesign whose consumers are out of radius.
    6. ADR 0003 is left contradicted, and #634's radius excludes it.
- **Decided:** `mod.rs` under `src/` deviates from the codebase's `foo.rs` + `foo/` convention. Recorded as a justified deviation (warn 14), because it keeps each module root inside its owner's glob. Phase 7 and S should not flag it.
- **Assumed:**
  - Verbs execute in the CLI process, so `pane/*` holds only the four `PaneStore` operations. This is inferred from #644's blast radius (`launch.rs`, `relaunch.rs`, `holler-pane/src/tx_launch.rs`). #649's "wire the real adapters into the hub's `pane/*` handlers" reads the other way; O/the operator should confirm.
  - The epic's contract section and the sibling issues' fixed text (#639, #643, #644, #646, #649, #660, #661, #663) are as fetched from GitHub on 2026-10-08.
- **Hedged:**
  - Row 6 (ADR 0003) is a block under the stack's ADR rule and the established practice, where every verb-adding story updated ADR 0003. The operator can instead move it to #634 by amending #634's radius, accepting that the ADR lags the code.
  - Rows 8-10 (port signatures, `Prober`, verb entry and `Ports` bundle) are warns, not blocks, because the brief can let F settle them. But #637 freezes them, and #638 implements them in parallel.
- **Evidence:**
  - Read: main.rs (full), cli.rs (full), lib.rs, say_cmd.rs, interrupt_cmd.rs, answer_cmd.rs, hold_cmd.rs; control_server.rs:1-200; serve.rs:565; holler-proto vocab.rs, methods.rs, lib.rs, hold.rs and error.rs header; scripts/lint.sh; clippy.toml; ci.yml machete step; holler-cli and holler-hub `Cargo.toml`; cli_surface_test.rs, docs_cli_test.rs and both fixtures; ADR-0003.md table and its `git log`.
  - Fetched with `gh issue view`: #633, #634, #637, #638, #639, #643, #644, #646, #649, #660, #661, #663.
  - Counted `main()` code lines the way clippy's `too_many_lines` does (99). Ran a scratch crate in the scratchpad pinned to the workspace's clap 4.6.6 to test the `say`/`interrupt`/`answer` `--pane` shapes; no worktree files were touched.
