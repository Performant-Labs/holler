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

## A (Phase 3 re-review, brief revision 2) — 2026-10-08T21:49:01-06:00
- **Decided:** BLOCK on docs/handoffs/637-brief.md at d63d278 (revision 2), with 2 blocks and 9 warns. See handoff-A.md, which replaces the revision-1 review; that review is kept in git at 16c27d7.
  - All six revision-1 blocks are answered, and each was re-verified against the code.
  - Block 1: `pane/args.rs` and `profile/args.rs` are frozen, but they declare no positionals and none of the verb-specific flags that #643-#647, #650 and #662-#665 need. Each of those stories' radius holds only its own verb files.
  - Block 2: the closed `PaneError` list misses `profile-exists` and `profile-has-live-panes`. It also has no variant for timeouts, missing records, corrupt stores or unreachable services, all of which the port implementers (#638-#642, #649, #661) must return.
- **Decided:** the hub stubs answering with the CLI envelope is a warn, not a block (row 3). Its sharers (#639, then #661, then #649) run in sequence, not in parallel. It is still a layering and duplication risk.
- **Assumed:**
  - The sibling issue texts are as fetched with `gh issue view` on 2026-10-08. The latest edit to any of them was at 19:06 MDT, before revision 2 was committed at 21:26 MDT.
  - Decision 0's knock-on issue amendments (row 9) are not posted yet; the brief says they are drafted separately.
- **Hedged:**
  - Row 6 (the output API) is a warn because F can settle it. But #660 cannot change the signatures later, and an `emit()` that writes straight to stdout leaves every verb story's in-process JSON test without a seam.
  - Row 1 offers two fixes: enumerate the whole surface now, or move each verb's `Args` struct into its own verb file. The operator picks one.
- **Evidence:**
  - Read:
    - the brief (both revisions);
    - in holler-cli: main.rs, cli.rs, lib.rs, say_cmd.rs, and hold_cmd.rs (parts);
    - in holler-hub: control_server.rs (:1-420 and :697-760), serve.rs (:340-610), and the `load` path in holds.rs;
    - manifests and config: the holler-hub, holler-cli and holler-proto `Cargo.toml`, the workspace `Cargo.toml`, clippy.toml, and lint.sh;
    - in holler-proto: methods.rs, vocab.rs, clock.rs, and the error.rs table;
    - CLI tests: docs_cli_test.rs, cli_invocation_test.rs, the cli-surface fixtures, the support/mod.rs API, and hub_serve_test.rs;
    - ADR-0003.md.
  - Fetched #633-#667 with `gh issue view`, and grepped every kebab-case code they name.
  - Counted `main()`'s lines the way clippy does (99).
  - Ran a scratch crate on clap 4.6.6 to test the `--pane` tail with and without `trailing_var_arg`. No worktree files were touched.
