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

## F (implement): 2026-10-11 21:14 MDT
- **Decided:**
  - **The transaction's shape is A's preferred option (warn 2b): a guard-holding handle, no closure.** `PaneState::locked()` returns `panes::Locked`, which holds the pane lock. `profile::store::Store::rename` takes the profile lock, then the handle, so ADR-0021 section 7's lock-order sentence stays true unamended, and a `panes.json` that failed to load is `store-corrupt` before any profile check (the order the ADR states).
  - **The twins are generalized in place (warn 2c).** In both stores, `entries_with`, `save` and `commit` take a slice of changes, `apply` (memory and the wake-up) is split out of `commit`, and `after(cursor)` gives consecutive cursors. The single writes pass one change. The restore after a failed profile save is the pane store's unlogged `write` of the committed table, and its failure is logged as `profile_rename_restore_failed` through `log_fault`.
  - **One `Store::rename(.., panes: Option<&PaneState>)` (warn 3).** The port passes `None` (the record half), and `profile/rename` passes the pane registry. `ProfileState::rename`'s doc and the port doc in `holler-pane/src/profile.rs` say exactly what each does, without the sentence A found false.
  - **A rename with no member writes no pane file and wakes no pane watcher.** The handle's `save`, `restore` and `apply` do nothing for an empty list of moves.
  - **`profile::dispatch` and `rename::dispatch` take `panes: &Arc<PaneState>` (warn 2a).** The handler is `panes::handlers::run` with a closure, like the other `profile/*` methods.
  - **D7's key order: warn 4 option (a).** holler-cli declares `serde_json`'s `preserve_order`, with lint #149's marker, and the workspace comment now says holler-cli relies on it. `ProfileSpec`'s fields are not mirrored: each pane is `to_value(spec)` with `herdr.grid` replaced in place.
  - **One `Document` type in `export.rs`, both directions.** Reading decodes it with `deny_unknown_fields`, and maps any failure to `usage` from serde's category and position alone. The specs then go through `GridPos::parse` (text positions) and `decode_params`.
  - **A spec's `usage` replaces serde's message** with one naming the spec's position, because serde's text can quote the file (the acceptance's "never echo serde's message"). A typed guard's own code passes unchanged.
  - **File errors follow A's warn 5:** the split `pane import` makes. `usage` for a path that names nothing usable (missing, a missing or non-directory parent, a directory, no file name, not a regular file, over 1 MiB, not a document). `unavailable` for any other I/O failure. ADR-0021 section 9's two rows and the guide's "Ends with" lines say so. This deviates from D8 and D9, which made every file error `usage`.
  - **The text forms print "N specs" as the tests pin them** (`1 specs`), not through `count()`.
  - **Drift compares a profile with its members in scope.** For a pane in scope, the row is the one `profile show` computes over all members. The profiles are read by one `list`, only when a pane in scope names a profile. A pane naming a missing profile is not reported (brief: out of scope).
  - **`profile_rename_restore_failed` is recorded in ADR-0021's "Rename as built".** The README "Debug output" table is unchanged, since the registries' other fault events (`*_registry_corrupt`, `*_registry_write_failed`) are not listed there either.
- **Assumed:**
  - **The brief overrides the issue's "edit none of #661's files" (A warn 2d).** `Store`'s lock, save and commit are private to `profile::store`, so the rename has to edit them. This is recorded here and in the handoff, because the brief is O's and is not edited after the plan check.
  - **The fake's rename can leave half a write.** It appends two events in one `feed.write`, so a cursor overflow on the second append would leave the first in place. A `u64` cursor cannot overflow in a test, and the fake's `Log.head` is private to `feed.rs`, so there is no pre-check.
- **Hedged:**
  - **Two of T's new conformance cases fail over the wire only:** `rename-of-one-slug-is-one-put` and `rename-feed-is-deletion-then-put`, through `HubProfileStore` in `pane_integration`. `HubProfileStore::watch` makes its first long-poll at once (#649, D4), so the first `next()` after `open(...)` at the head is the idle batch of a window that ended before the rename. The suite's own cases 19 and 21 consume that first idle item before writing. Both cases pass on the fake and on `ProfileState`. This is a test sequencing defect, flagged for T in `handoff-F.md`. F did not edit the cases.
  - **Lock-order safety is reasoned, not proven.** It rests on `check_membership` releasing the profile lock before the pane write takes the pane lock (`evidence.md`), plus T's two 20-round race tests, both green.
- **Evidence:**
  - **Gates:** `cargo clippy --workspace --all-targets -- -D warnings` is clean, and `bash scripts/lint.sh` exits 0 (every touched file is under 600 lines).
  - **Formatting:** rustfmt was run per file on the leaf files F touched. Every touched module tree passes `rustfmt --check`, and no other file changed.
  - **Tests:** the targeted and workspace runs are in `handoff-F.md`. `evidence.md` holds the unchanged-code facts the diff relies on.

## T (Phase 6, verify / GREEN): 2026-10-11 21:40 MDT
- **Decided:**
  - **Conformance cases 26 and 27 repaired (test-only), with F's option (a).** Each now reads the first `next()` of the watch opened at the head (expect `None`) before the rename, as case 19 does. F's diagnosis holds: `HubProfileStore::watch` makes its first long-poll inside `watch()` (#649 D4), and `expect_change` reads exactly one `next()`. Production code is unchanged.
  - **PASS.** The suite is GREEN on all three conformance runners. Every Phase-4 test passes. Tier 1 and Tier 2 are clean.
- **Assumed:**
  - **The one workspace failure is the known flake #693, not a regression.** It is `remote_say_wait_and_hub_query_json_match_local_after_masking`, `updates` 1 vs 2.
    - #693 is open with that exact signature.
    - The test passed 4 of 5 runs in isolation.
    - The diff touches no `say`, roster, body or circuit code.
- **Hedged:**
  - **The `unavailable` half of the file-error split is unpinned** (F's deviation 1). A permission test is unreliable as root. Recorded as advisory.
- **Evidence:**
  - **The workspace run:** `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`. 154 targets: 2,433 passed, 1 failed (#693), 32 ignored.
  - **The gates:** `lint.sh`, `changelog-check.sh`, clippy `-D warnings` and `cargo machete` are all clean.
  - **The mutation spot checks** were built in a separate target dir and reverted:
    - with no deletion event in the fake's rename, case 27 fails;
    - with no member moves in `Locked::moves`, H2, H3 and H5 fail.
  - **Concurrency:** `profile_rename_faults_test` passed 5 runs of 5.

## T (Phase 7, verify / GREEN, re-run): 2026-10-11 21:58 MDT
- **Decided:**
  - **PASS again, on `3b20fc0`.** Nothing changed since the first GREEN round, and no rework note came with the call, so no test was edited.
  - **The re-run was full and independent:** the workspace suite, lint, the changelog check, clippy and machete.
  - **One new mutation spot check, for drift.** The earlier round's mutations covered the feed and the member moves.
- **Assumed:** this is a re-entry of the same phase, for example a resumed script, and not a request for a rework.
- **Hedged:** #693 did not recur in this run. It is still open and can still fail CI on a test this diff does not touch.
- **Evidence:**
  - **The workspace run:** 154 targets, 2,434 passed, 0 failed, 32 ignored, exit 0.
  - **The gates:** `lint.sh` exits 0, `changelog-check` prints `ok`, and clippy `-D warnings` and `cargo machete` are clean.
  - **The drift mutation:** `drifted` was changed to match no `Differs` row. Four `doctor::drift` tests failed: agreement, one-per-pass, scope and shape. The change was reverted, and `git status` is clean.

## A (Phase 7, anti-duplication gate): 2026-10-11 22:02 MDT
- **Decided:** PASS, with 0 block and 3 warn findings (`docs/handoffs/665/handoff-A-dup.md`).
  - **No parallel production path.** All 11 of Phase 3's "Reuse expectations for Phase 7" hold:
    - one `Store::rename` for the port and the transaction;
    - the twins extended in place under the same names;
    - the restore as a save of the committed table;
    - the `run` handler pipeline;
    - the CLI reuse of `insert_profile`, `taken`, `profile_from_panes`, `decode_params`, `GridPos::parse`, `write_atomic` and `show::text`;
    - drift as `diff_profile` over `is_member`;
    - the fake in one `feed.write`;
    - D6.
  - **Warn 1 (test-only).** `pane_support::request` is a near-copy of `profile_handlers_test.rs`'s private `line`, and `REPLY_WITHIN` is declared three times. Fold `line` into `request`.
  - **Warn 2.** `profile import` and `profile export` print `N specs` without `super::list::count`, so a one-spec profile prints `1 specs`. Every sibling verb prints `1 spec`, and three test strings pin the difference. Use `count()`, or record the literal form as intended.
  - **Warn 3 (low).** `panes::Locked`'s `save` and `apply` accept any `PaneEvent`s, a crate-visible bypass of `cas_put`'s compare-and-swap and `refuse_profile_move`. An opaque `Moves` type would close it. Optional.
- **Assumed:**
  - Test code is in A-dup's scope, since a Phase 7 block sends T to repair the tests.
  - The hub tests' convention of one `Rig` per file is the established pattern, so the two new rename `Rig`s are not drift.
  - The brief's `<N> specs` was a template, like `create`'s doc comment (`N specs`), not a requirement to skip pluralization.
- **Hedged:**
  - **Warn 1 is a warn, not a block.** Before #665, the hub tests kept one copy of the dispatch-with-timeout helper per file, so there was no dominant pattern. The new copy is in the shared module.
  - **Warn 2 is a consistency call.** Either outcome is acceptable if it is recorded.
  - **I did not run the tests.** T's GREEN rounds (2,434 passed, 0 failed, on `3b20fc0`) cover behavior. The diff since then is the handoffs only.
- **Evidence:**
  - **The diff:** `git diff bda1069..fa1677c`, with every changed production file read in full:
    - hub: `profile/{store,mod,rename,handlers}.rs` and `panes/{store,mod}.rs`;
    - CLI: `profile/{rename,export,import,create,show}.rs`;
    - holler-pane: `reconcile.rs`, `reconcile/drift.rs`, `findings.rs` and `profile_diff.rs`;
    - the testkit's `profile_store.rs`.
  - **The tests read:** the new hub, conformance, CLI-verb, doctor and process tests and their support modules.
  - **Greps:**
    - `is_member`, `diff_profile` and `ProfileNotFound` construction sites;
    - `count(` across the profile verbs;
    - the request helpers and `REPLY_WITHIN` in the hub tests;
    - bounded file readers;
    - personal infrastructure names in the diff and the commit metadata.
  - **D6:** `git diff --stat` is empty for the files D6 says are untouched.
