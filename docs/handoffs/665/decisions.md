# Decision journal: #665 `holler profile rename`, `export`, `import` and `profile-drift`

## A (Phase 3, up-front plan review): 2026-10-11 20:18 MDT
- **Decided:** PASS, with 0 block and 6 warn findings (`docs/handoffs/665/handoff-A.md`).
  - **The layering is right.**
    - Rename is a hub store transaction, under the lock order ADR-0021 section 7 reserves for it.
    - The verbs run CLI-side, with no change to any port.
    - The file format is in the CLI (ADR-0021 section 5).
    - Drift is one kind computed by `diff_profile`, the comparison `profile show` uses.
  - **The warns:**
    1. Two test files outside the Blast radius fail CI: `pane_integration/no_hub.rs`'s bare rows and `profile_registry_test.rs`'s 23-case pin. A third conformance runner over the wire (`pane_integration/clients.rs`) and acceptance 16's scope also need naming.
    2. D2's shape. The owned `Arc<PaneState>` seam. ADR section 7's "never calls into the profile registry" sentence and the order of the checks, both of which the closure shape breaks (a guard-holding handle is preferred). Twin-symmetric multi-change save and commit, with the restore as a re-save. Stale module docs and operating limits, since `send_prompt`'s gate waits on the pane lock.
    3. D3's port doc is false for `ProfileState` on a hub; one `Store::rename` should serve both halves.
    4. D7's key order relies on `preserve_order` that holler-cli gets only by unification through holler-body. Declare it, or drop the guarantee; never mirror `ProfileSpec`.
    5. D8 and D9 make every file I/O error `usage`, against `pane import`'s split of `usage` and `unavailable`.
    6. The placement of the conformance rename cases, the import tests and the hub rename test is not named.
- **Assumed:**
  - The brief's operator decisions (rigor, one story, D4's log reading, D10's "once per pass") are final and outside A's remit.
  - No `survey.md` exists, so "Reuse expectations for Phase 7" in the handoff stands in for the Reuse map.
  - The automated run has no O to amend the brief, so T and F read the warns as amendments.
- **Hedged:**
  - **Warn 2(b) offers two acceptable shapes.** A guard-holding handle, or the closure plus an ADR amendment and a pre-check. A-dup should accept either if the ADR text matches the code.
  - **Warn 4.** The feature unification comes from `cargo tree -e features -i serde_json`, which shows `agent-client-protocol` enabling `preserve_order`. I did not build an export to observe the key order.
  - **Warn 5 is a consistency call.** The brief's `usage` is defensible. I ask for alignment or a stated reason, not a specific class.
  - **Concurrency.** That the rename transaction cannot deadlock is reasoned from every hub path that takes the two locks (`check_membership` releases the profile lock before the pane lock; `poll` waits without the lock), not tested.
- **Evidence:**
  - **The brief.** Read in full (2495 lines). A script checked every quoted source line of E3 to E72 and E77 against `bda1069`: 1,599 lines, 0 mismatches.
  - **Hub source:** `holler-hub/src/profile/{mod,store,entry,handlers,rename}.rs`, `panes/{mod,store,handlers,persist}.rs`, `pane_dispatch.rs`, `circuit/dispatch.rs` (the pane gate).
  - **holler-pane source:** `reconcile.rs`, `findings.rs`, `profile_diff.rs`, `profile.rs`, `reply.rs`, `grid.rs`, `argv.rs`, `profile_snapshot.rs`, `import/files.rs`, `error.rs` (`from_decode`).
  - **holler-cli source:** `profile/{mod,create,show,delete,rename,export,import}.rs`, `output.rs`, `pane/wiring/store.rs`, `pane/wiring.rs` (`Unwired`), `pane/import.rs`, `pane/import/sources.rs`, `main.rs` (`run_with_stdio`).
  - **Test kit:** `profile_store.rs`, `feed.rs`, `profile_scope.rs`, `fault.rs`, `conformance/profile_store.rs`.
  - **Tests:** `pane_integration/{main,no_hub,clients}.rs`, `hub/tests/{profile_registry,pane_dispatch,profile_handlers}_test.rs`, `pane_support/mod.rs`, `doctor/{rig,read_only}.rs`, `process/{stub,flags}.rs`, `docs_pane_control_test.rs`, `findings_test.rs`.
  - **Docs and config:** ADR-0021 sections 1-3 and 5-9 and "Decisions taken"; `Cargo.toml`; `scripts/lint.sh`; `.aftersight/pipeline.config.json`.
  - **Commands:** `cargo tree -e features -i serde_json --offline`; `git fetch` and `git diff HEAD...origin/main` (#851 merged at 20:07 MDT, in different hunks); and greps for `PROPOSED`, `#665`, `rename`, `not-implemented` and the conformance case-count pins.
