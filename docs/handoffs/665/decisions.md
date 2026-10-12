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

## S (Phase 10, spec audit): 2026-10-11 22:17 MDT
- **Decided:** REWORK (`production`), with four items (`docs/handoffs/665/handoff-S.md`).
  - **The transaction, the verbs and the drift finding are spec-compliant.** A's and T's preconditions hold. The deviations (A warns 2b, 3, 4a and 5, and the override of the issue's "#661 files" rule) are recorded and justified.
  - **1. Production:** `profile import` and `profile export` print `1 specs`. Use `super::list::count`, as every sibling verb does. This resolves A-dup warn 2, which no one had resolved. T updates the 3 pinned strings.
  - **2. Production docs:** four statements, the `profile import` help among them, say no refusal prints the file's content. A refused position is quoted by `GridPos`'s `excerpt`. Narrow the statements, or make them true in import and pin that.
  - **3. Test-only:** no spec-level case makes serde quote a value, so `spec_refused`'s redaction can be removed with every test green. Add wrong-typed `CANARY` members.
  - **4. Test-only:** acceptance 1's "the stamp as `updated`" is checked only as `>=`. Assert equality with the rename's log `at`, in H1 and in conformance case 23.
- **Assumed:**
  - The automated run has no O, so A's warns are amendments to the brief, as T and F read them.
  - The brief's `<N> specs` is a template rendered through `count()`, as `create`'s identical doc template is. It is not a requirement to print `1 specs`.
  - A position is not secret by type. Item 2 is about the accuracy of the claim, so either side may move.
- **Hedged:**
  - I ran no test, as S does not run Tier 1 or Tier 2. That each REWORK test gap survives its mutation is reasoned from serde's messages: an unknown key names the key, not the value. It was not run.
  - Lock-order safety is verified by reading every hub path to the profile registry. `check_membership` runs before the pane lock. It is not proven by a test beyond T's bounded races.
- **Evidence:**
  - **Read:** the full diff `git diff origin/main...HEAD`; every production file in full; every new or changed test; the brief's decisions, acceptance, contract and guide sections; all six handoffs, `evidence.md` and the outside gate's `665-diff-result-r1.md` (PASS).
  - **Commands:** `git merge-tree` against `origin/main` (5 conflicting docs and test files, advisory); `git diff --stat` of the untouched files; `wc -l` of every touched file; a privacy grep of the added lines (no hits); greps for `unwrap`, `expect`, `panic` and `#[allow]`; `target/debug/holler profile import --help`; and source reads of `GridPos::parse`, `excerpt`, `from_decode` and `split_prefix`.

## F (rework 1, after S REWORK items 1 and 2): 2026-10-11 22:40 MDT
- **Decided:**
  - **Item 1: `profile import` and `profile export` print the spec count through `super::list::count`,** as every sibling verb does. A one-spec profile now prints `1 spec`, which resolves A-dup warn 2. T updates the three test strings that pin `1 specs` (`handoff-F.md`).
  - **Item 2: S's option (b). The code is made to match the claim, rather than the claim narrowed to the code.**
    - In import, the two grid codes keep their codes. Their messages now give `herdr.grid of spec N` and `GridPos`'s hint or bounds, in place of the text `GridPos` quotes.
    - This covers a text position (`read_position`) and the object form's `pos` label and zero cell (the spec's decode). Both now go through `spec_refused`.
  - **Why (b) and not (a):**
    - The brief's import acceptance says "the refused value is never printed" right after it lists `grid-ambiguous` (`hj-c1r2`) and `grid-out-of-range` (`r0c1`). Option (a) would document an exception to the brief.
    - It is the rule the repository's file importers already keep: `pane import`'s launcher table names the file and the position "and never the error's text", and `spec_refused` already redacted `usage`.
    - The spec's number locates the fault in a file with several specs, which the excerpt did not.
  - **The claim is scoped to what a spec holds,** in the `--help`, both module docs, the guide and the CHANGELOG. The new wording is "no refusal quotes what a spec holds: a bad position or member is named by the spec's number".
    - "No refusal prints what the file holds" cannot be made true. The document's name is printed as a name: on success, by `profile-exists` (create's `taken`, the reuse A asked for), and quoted by `ProfileName::parse` when it is invalid, as `--name` is.
    - The `version` refusal prints the number.
    - Hiding the name would make `profile-exists` useless.
  - **`GRID_FORMS` and `GRID_BOUNDS` repeat holler-pane's private `FORMS` and `BOUNDS`.**
    - Making them public would edit `grid.rs`, which is outside the Blast radius.
    - Cutting the hint out of the original message could leak part of a `pos` label.
    - The member's path comes from `SpecField::Grid.as_str()`, not a literal.
  - **S's advisory 8 is taken, because it is in F's own files:**
    - the 132-character doc line in `profile/store.rs` is rewrapped;
    - the guide's rename sentence now says the new name's log gets one entry, after the log of a profile deleted earlier under that name;
    - the CHANGELOG bullet names `profile_rename_restore_failed`, as the registries' earlier entries name their events.
- **Assumed:**
  - **Not taken:**
    - advisory 2 (`docs/protocol/v2.md`), which the brief's "Not touched" list puts out of reach;
    - A-dup warn 3 (an opaque `Moves`), which is optional and can be a follow-up;
    - a README "Debug output" row, since the table lists none of the registries' events.
  - **The order of refusals across specs is unchanged.** Every text position is read before any spec decodes (D8 steps 4 then 5).
- **Hedged:**
  - **`spec_refused`'s `other => other` arm relies on holler-pane's guards keeping their no-echo rule** (`argv.rs`'s module docs). A future closed code with a message, answered by a spec's decode, would pass through unchanged. Today only the two grid codes carry text (`evidence.md`).
  - **Correction to round 1's handoff (S advisory 4).** `agent-key-invalid` does not pass through import. It is an open code, so `from_decode` folds it into `usage`, and the spec's generic message replaces it.
- **Evidence:**
  - **Gates:** clippy `-D warnings` (workspace, all targets) is clean, `lint.sh` exits 0, `changelog-check` prints `ok`, `cargo machete` is clean, and `rustfmt --check` passes on each touched file.
  - **`profile_verbs`:** 109 passed and 3 failed. The failures are exactly the three `1 specs` pins, each now `1 spec`.
  - **A scratch hub on its own temporary state dir:** four files with a canary in spec 2's position (text ambiguous, text `r0c1`, an object `pos` label, an object zero cell). Each was refused with its grid code, exit 3, as `herdr.grid of spec 2` with the hint, in both formats. The canary was printed 0 times, and `profile list` said `no profiles`.
  - **The control:** with the two new arms removed, the text position and the `pos` label both printed the canary. The arms were then restored.
  - **The workspace run:** 154 targets, 2,431 passed, 3 failed (the three pins), 32 ignored.
  - **The evidence check:** a script checked the eight new `evidence.md` entries against their cited lines: 0 mismatches.

## T (Phase 7, rework 1, after S REWORK and F rework 1): 2026-10-11 22:55 MDT
- **Decided:**
  - Updated the three `1 specs` pins to `1 spec` (S item 1, F's `count`).
  - Pinned F's option (b) with `CANARY` in a text position, a `pos` label and a zero cell, plus one new two-spec test that the message names `herdr.grid of spec 2`.
  - Pinned spec-level `usage` redaction with two wrong-type members holding `CANARY` (S item 3).
  - Pinned `updated` as the rename's one stamp in H1 and conformance case 23 (S item 4). H1 waits on `now_millis() > old.updated` first, so the check is deterministic.
  - Narrowed `refusals.rs`'s module doc to the rule now stated (F's optional item 5).
- **Assumed:** a condition wait on the hub's own clock function (at most about 1 ms) is within the no-blind-sleeps rule, since it observes a condition rather than sleeping for a duration.
- **Hedged:** case 23's equality cannot catch a kept `updated` on the fake (fixed clock) or reliably on `ProfileState`; H1 is the deterministic control, and `evidence.md` records why.
- **Evidence:** `profile_verbs` 113 passed; workspace 2,435 passed, 0 failed, 32 ignored; lint, changelog-check, clippy `-D warnings` and machete clean; three mutation spot checks each fail the new assertions and were restored from saved copies.

## F (rework 2, after the outside diff gate's round 2): 2026-10-11 23:10 MDT
- **Decided:**
  - **No production change.** Round 2's verdict is BLOCK, but its own line says "0 blocking findings", and each of its three BLOCK items concludes "no block". The pipeline's rule makes the verdict line depend on BLOCK findings only. The six needs-verification findings are answered by the diff or by evidence, and the three WARNs and three NITs are answered in `handoff-F.md`.
  - **`evidence.md` is rewritten to fit the gate**, at 11,118 bytes against its 12,000-byte cap; round 2's 22,769 bytes reached the reviewer as fragments.
    - It opens with where each claim can be checked, including why the excerpts of the changed hub files look cut: the brief's base citations consume the excerpt budget.
    - It adds NV-4 (`LiveHub` is the real `holler hub serve`; `forward` sends `profile/*` to `profile::dispatch`) and NV-6 (`write_atomic`).
    - It drops what the brief already quotes (E7, E8, E13, E20, E23, E26, E32) or a diff test pins.
    - Every citation names its path, and T's three entries are kept word for word.
  - **The WARNs:**
    - W-1 (`Option<&PaneState>`) stays, because it is A's warn 3 design.
    - W-2 (a path quoted with `{:?}`) stays: `Debug` escapes control characters, and the brief's `show::text` rule is for the success line, which export follows.
    - W-3 (the restore re-saves memory) stays: memory is the committed state, and a test pins the previous bytes.
- **Assumed:**
  - Round 3 of the gate reads the same inputs: the brief, the diff, 12,000 bytes of `evidence.md`, and excerpts in order of first citation. The handoff is not among them.
  - The workspace run of T-green on `5dbdbd9` still stands, because no compiled file changed.
- **Hedged:**
  - The reviewer may still write a contradictory verdict, which no change here can prevent. Three consecutive diff-gate BLOCKs escalate the run, and that is the right stop.
  - Three pipeline issues are listed for the main session to file in Aftersight: the verdict parse, the excerpt budget, and the gate getting no handoff.
- **Evidence:**
  - `docs/handoffs/665-diff-result-r2.md` and its prompt.
  - `dual-review.sh` lines 900-1245: the excerpt budget and the evidence cap.
  - `coding-pipeline.workflow.mjs`: the Phase 7 routing and `buildDiffGateCaptureTaskPrompt`.
  - A local `dual-review.sh --mode diff --dump-only`: the appendix is attached whole (11,687 bytes with its header), and 44 references were omitted at the excerpt cap.
  - The check script: 13 entries, 0 mismatches.
  - The gates: clippy `-D warnings` clean, `lint.sh` exit 0, `changelog-check` ok.
  - The story's targets: `profile_rename_test` 10, `profile_rename_faults_test` 8, `profile_registry_test` 12, conformance 12, `findings_test` 2, `profile_verbs` 113, drift 7, `profile_io` 2, docs 18 + 3 + 3, and `pane_integration` 66. All passed, 0 failed.

## T (Phase 7, rework 2, after F rework 2): 2026-10-11 23:17 MDT
- **Decided:**
  - GREEN on `2ed6ac0` with no test repair. F changed only handoff files, flagged no test, and no round 2 gate finding asks for a test change.
  - Re-ran the whole workspace rather than relying on F's assumption that `5dbdbd9`'s run still stands.
- **Assumed:** the earlier rounds' mutation spot checks still hold, since no `.rs` file changed between `5dbdbd9` and `2ed6ac0`.
- **Hedged:** the rebase onto `origin/main` (S advisory 1) will change compiled code. The suite must be re-run after it, and this GREEN does not cover the rebased tree.
- **Evidence:**
  - The workspace: 154 targets, 2,435 passed, 0 failed, 32 ignored (23:09 to 23:15 MDT).
  - Clippy `-D warnings`, `lint.sh`, `changelog-check` and `machete` are clean.
  - T's own check of the rewritten `evidence.md`: 13 entries, 0 mismatches, 11,118 bytes.
  - The quoted ranges of the dropped E-numbers are unchanged since `bda1069`.

## T (Phase 7, test-only rework, diff gate round 3): 2026-10-11 23:27 MDT
- **Decided:**
  - B-1 is answered in the tests and the appendix, not in production code. `ProfileSpec`'s `env` and `expect` carry no `skip_serializing_if` (`profile.rs:192-193`, `:202-203`), so both are always written, and `BARE_DOCUMENT` already pinned `"env": []` and `"expect": []` byte for byte. The gap the reviewer named is real in one place: no test fed a file that leaves the two members out. The new import test `a_spec_without_env_or_expect_imports_and_exports_them_as_empty_arrays` does that, then exports the result and asserts both are `[]` and the whole document equals `BARE_DOCUMENT`.
  - One evidence entry quotes `ProfileSpec`'s `deny_unknown_fields`, `env` and `expect` attributes, because the gate cannot see unchanged lines of `profile.rs`. It also answers B-2's "`deny_unknown_fields` not shown". B-2 is marked "Not a block" and asks for no test change; its `error.rs:632-634` excerpt is already in the appendix.
  - `BARE_DOCUMENT` is now `pub(crate)`, so the new test reuses it instead of a second copy.
- **Assumed:** the gate's "absent (not just empty)" case can only arise from a file, since a `Vec` in memory has no absent state. So an import, then an export, is the cheapest test that reaches it.
- **Hedged:** none.
- **Evidence:**
  - Mutation (restored from a copy, `git status` clean in `crates/*/src`): `skip_serializing_if = "Vec::is_empty"` on `env` and `expect` fails the new test (`left: (Null, Null)`) and `export_leaves_out_absent_optional_members`.
  - The workspace: 154 targets, 2,436 passed, 0 failed, 32 ignored, 23:19 to 23:25 MDT. That is 2,435 plus the new test.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check` and `machete` are clean. `rustfmt --check` passes on both touched files.
  - `evidence.md`: 14 entries, 0 excerpt mismatches, 11,799 bytes (under 12,000).

## A (Phase 7, anti-duplication gate, re-run after the rework): 2026-10-11 23:32 MDT
- **Decided:** PASS, with 0 block and 3 warn findings (`docs/handoffs/665/handoff-A-dup.md`, top section).
  - **The cycle's diff (`fa1677c..c63908e`) extends existing objects only:**
    - `super::list::count` in both `render` functions, which closes the last pass's warn 2;
    - two new arms on `spec_refused`, with the position step routed through it (one redaction path);
    - `SpecField::Grid.as_str()` for the member's path;
    - `BARE_DOCUMENT` shared, not copied;
    - the suite's `entries` in case 23, and the hub's `now_millis` in H1.
  - **Warn 1 (new).** `GRID_FORMS` and `GRID_BOUNDS` (`export.rs:54-60`) copy holler-pane's private `FORMS` and `BOUNDS` (`grid.rs:23-27`).
    - The copy is deliberate and recorded, and it is not a parallel path. But nothing pins it to the originals.
    - The fix is to expose the originals from holler-pane as a follow-up, or to have T pin the copy against `GridPos::parse`'s message.
  - **Warns 2 and 3** are the last pass's warns 1 and 3, unchanged and optional.
- **Assumed:**
  - "The diff produced this cycle" means `fa1677c..c63908e`, the code changed since the last A-dup pass. I re-checked the Phase 3 expectations only where that diff touches them (`export.rs`, `import.rs`).
  - Message text copied across a crate boundary because of the Blast radius is a warn, not a parallel path, under the Phase 7 verdict rule.
- **Hedged:**
  - I ran no test. T's last GREEN (the workspace on these sources: 2,436 passed, 0 failed) covers behavior.
  - `spec_refused`'s `other => other` arm is noted, not raised. It predates the cycle and matches its neighbours in `create.rs` and `delete.rs`. `with_context` is the precedent if the operator wants the redaction enforced by the compiler.
- **Evidence:**
  - **Read:** `git diff fa1677c..c63908e` (10 code and docs files); `src/profile/export.rs` and `import.rs` in full; every changed test; `grid.rs`; `error.rs` (`from_decode`, `from_closed`, `deserialize_parsed`); `pane/profile_scope.rs::with_context`; the gate's round 4 result (PASS, no duplication finding).
  - **Greps:**
    - the hint text across crates and docs;
    - `fn wait*` and spin-waits on the clock in the hub tests;
    - `fn count` and the users of `list::count`;
    - where `assert_message_contains` and the conformance `entries` were defined;
    - `REPLY_WITHIN` and the request helpers;
    - personal infrastructure names in the added lines and the commit metadata.
  - **Untouched files:** `git diff --stat bda1069..HEAD` is empty for `grid.rs`, `bind.rs`, `ports.rs`, `lib.rs`, `cli.rs`, `docs/protocol/v2.md` and `docs/orchestrating.md`.

## S (Phase 10, spec audit, round 2): 2026-10-11 23:43 MDT
- **Decided:** PASS (`docs/handoffs/665/handoff-S.md`, top section).
  - **Round 1's four REWORK items are closed:**
    1. Both counts go through `count()`.
    2. The grid redaction is option (b): `spec_refused` replaces the grid codes' messages, and the claim is scoped to "what a spec holds".
    3. The `usage` redaction is pinned by wrong-type canary members.
    4. `updated` equals the rename's stamp, in H1 and in case 23.
  - Every acceptance criterion maps to a named test. The deviations are the ones round 1 accepted.
  - **Six advisories, none blocking:**
    1. the rebase conflicts in 5 files;
    2. `docs/protocol/v2.md:817`;
    3. "a bad position or member is named by the spec's number" overstates the three unit-variant guard codes;
    4. A-dup's new warn: the grid hint text is copied from holler-pane, and nothing pins the copy;
    5. five new hunks are not rustfmt-clean, in two test files;
    6. round 1's remaining advisories.
- **Assumed:**
  - T-green's last workspace run (2,436 passed, on `c63908e`'s sources) covers `HEAD`, because `c63908e..cd2e06e` changes only `docs/handoffs/665/`.
  - The pipeline's commit format (no session link) satisfies CONTRIBUTING.md, as it did for #648's merged run. The PR's AI disclosure is added after the script opens the PR.
- **Hedged:**
  - I ran no test, since S does not re-run Tier 1 or Tier 2. Behaviour rests on T's recorded runs and mutation checks, plus my reading of the code.
  - Advisory 2 is left as an advisory, not an ADVISORY-HOLD. It is a one-word, non-normative edit in a file the brief puts out of reach, and no criterion depends on it.
- **Evidence:**
  - **Read in full:** the brief; issue #665 and its confirming comment; every handoff and both A-dup passes; `evidence.md`; the outside gate's rounds 3 and 4.
  - **Diffs:** `git diff origin/main...HEAD` and `git diff bde41cf HEAD`.
  - **Source read in full or in part:**
    - the three verbs (`rename.rs`, `export.rs`, `import.rs`);
    - the hub's `profile/store.rs` and `panes/store.rs` (rename paths), `profile/{mod,rename}.rs` and `panes/mod.rs`;
    - `reconcile/drift.rs`;
    - the fake's `move_record`;
    - `error.rs` (`from_decode`, `from_closed`, `Display`);
    - `output.rs` (`emit`, `write_line`);
    - `atomic_file.rs` (`temp_path`);
    - every new and changed test.
  - **Commands:**
    - a grep of added `src` lines for `unwrap`, `expect`, `panic` and `unsafe`;
    - a grep for `#[allow]`;
    - `wc -l` of every touched file;
    - a privacy grep of all 8,718 added lines, and `git log` for the authors;
    - `git diff --stat` of the files D6 says are untouched;
    - `git merge-tree` against `origin/main` `037405f`;
    - `rustfmt --check` on the touched files, at the base and at the head;
    - a grep of the touched tests for sleeps.
