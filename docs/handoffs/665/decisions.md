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

## T (Phase 4, author tests / RED): 2026-10-11 20:45 MDT
- **Decided:**
  - **The contract amendment (D1).** T made it first, as A's notes propose: ADR-0003's three rows, and every ADR-0021 edit of the brief. The "on its own" sentence uses A's warn-3 wording.
  - **What stays F's.** The guide, the CHANGELOG and the `src/` doc comments. `docs_pane_control_test` is RED until the guide edits land.
  - **The hub's tests are split in two targets**, `profile_rename_test.rs` (the outcome) and `profile_rename_faults_test.rs` (faults, restart, retry, races), each under 600 lines (A warn 6). They share `pane_support::{request, rename_params}`.
  - **The conformance rename cases are a new child**, `conformance/profile_store/rename.rs`: six cases, 23 to 28, at the end of the table, so the cases after them did not move. The env case is now 17 and the watch cases are 18 to 22.
  - **`rename-is-not-implemented` is gone.** Case 22's fifth refused write is a stale rename. The suite has 28 cases (`profile_registry_test` and `DOCUMENTED_CASES`).
  - **Three rename mutants** were added to the conformance test.
  - **The import tests are split** into `import.rs` and `import/refusals.rs` (A warn 6).
  - **The D7 document is pinned byte for byte**, the brief's own acceptance. This forces A warn 4 option (a), or an equivalent guarantee of the order.
  - **Panes-first is pinned** by the inode of `panes.json` after a failed profile save: the old bytes, in a new file. It is the only observable of the order.
  - **The member-write race uses an update of an existing member** (compare-and-swap at generation 1), so its outcome is one of exactly three, never the documented check-then-write gap of a new join. Moved with the edit, refused `generation-conflict`, or refused `profile-not-found`.
  - **Acceptance 16's allowlist is in `drift.rs`, not `read_only.rs`.** It is scoped to whole-fleet and `--pane` passes, per A warn 1. The every-incident rig still has no profile, so `read_only.rs` is unchanged.
  - **`STUBS` is now empty**, so `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` uses `profile show` of a missing profile on a live hub.
- **Assumed:**
  - The CLI-verb REDs fail at clap's parse (`unexpected argument`). The verbs' positionals are part of the feature, and this is not a harness fault.
  - A spelling-only rename of OLD to its own stored name is `usage` even when OLD is spelled differently in the request (`demo` to `Demo`).
  - `--name` taken in another spelling (`copy` against `Copy`) is `profile-exists`.
- **Hedged:**
  - The races run 20 rounds each. They prove no interleaving exhaustively.
  - The canary for export sets a process environment variable with a unique name, `HOLLER_T665_EXPORT_CANARY`, as `interrupt_test.rs` does.
  - **A tooling slip, fixed.** T's own `cargo fmt -p holler-cli` and `-p holler-hub` reformatted about 130 files that are not this story's. They were reverted with `git checkout`, and T's edits to `no_hub.rs` and `docs_pane_control_test.rs` were reapplied by hand. F is told not to run `cargo fmt -p`.
- **Evidence:**
  - **Gates:** `cargo clippy --workspace --all-targets -- -D warnings` is clean, and `bash scripts/lint.sh` exits 0.
  - **The RED run:** `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`. The targeted runs are recorded in `handoff-T-red.md`.
