# Handoff-S (round 2): Phase 10 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)  (spec audit)

**Date:** 2026-10-11 (23:43 MDT)
**Branch:** `issue-665-implementation`, head `cd2e06e`, merge base `bda1069`
**Issue:** [#665](https://github.com/Performant-Labs/holler/issues/665). The operator confirmed it on 2026-10-11 at 19:40 MDT, in the issue's comment.
**Source of truth:** `docs/handoffs/665-brief.md`. The run has no O, so A's six warns (`handoff-A.md`) count as amendments.
**Handoffs reviewed:**
- `665/handoff-A.md`
- `665/handoff-T-red.md`
- `665/handoff-F.md`: round 1, rework 1 and rework 2
- `665/handoff-T-green.md`: round 1, reworks 1 and 2, and the test-only round
- `665/handoff-A-dup.md`: both passes
- `665/decisions.md` and `665/evidence.md`
- round 1 of this file
- the outside diff gate's rounds 1 to 4 (`docs/handoffs/665-diff-result-r{1..4}.md`, gitignored). Round 4 is PASS.

**Diff audited:**
- `git diff origin/main...HEAD`: 59 files. I read every production file in full.
- `git diff bde41cf HEAD`: the rework cycle since round 1, 10 code and docs files.

**Verdict:** PASS

Round 1 of this audit follows this section, unchanged.

## A precondition

Met.
- A is PASS, with 0 block and 6 warns.
- A-dup was re-run on `c63908e` and is PASS, with 0 block and 3 warns. Warn 1 is new (`GRID_FORMS` and `GRID_BOUNDS` copy holler-pane's text). Warns 2 and 3 carry over from the first pass.

## T precondition

Met.
- **RED.** `handoff-T-red.md` records 70 failures in 14 targets. Each one is the missing feature, not a setup fault.
- **GREEN.** T-green's last round ran on the sources of `c63908e`:
  - the workspace run: 154 targets, 2,436 passed, 0 failed, 32 ignored;
  - `lint.sh`, `changelog-check`, clippy `-D warnings` and `cargo machete` are all clean;
  - `git diff --stat c63908e HEAD` shows changes only under `docs/handoffs/665/`, so that run covers every line of code audited here.
- **Blocking issues:** none.
- **Mutation spot checks on this cycle's repairs:**
  - With `spec_refused`'s two grid arms disabled, both grid canary tests fail, and the output contains the canary.
  - With the `usage` arm disabled, the wrong-type test fails on serde's `invalid type: string "t665-import-canary-..."`.
  - A rename that keeps the old `updated` fails H1, 3 runs of 3.
  - With `skip_serializing_if` added to `env` and `expect`, the new import test fails, and so does `export_leaves_out_absent_optional_members`.

## Round 1's REWORK, item by item

| # | What round 1 required | What was done | Proof |
|---|---|---|---|
| 1 | Pluralize the spec count with `count(n, "spec")` | Both `render` functions use `super::list::count` (`src/profile/export.rs:302`, `src/profile/import.rs:126`). | The three pins now read `1 spec`: `tests/profile_verbs/import.rs:81` and `:160`, and `tests/profile_verbs/export.rs:319`. |
| 2 | Narrow the "never printed" claim, or make it true | **Option (b), make it true.** <ul><li>`spec_refused` keeps both grid codes but replaces their messages with `herdr.grid of spec N` plus the hint or the bounds (`export.rs:193-209`).</li><li>`Document::specs` sends the text-position step through `spec_refused` too (`:147-149`).</li><li>The claim is scoped to "what a spec holds" in `--help`, both module docs, the guide and the CHANGELOG.</li></ul> | <ul><li>`import/refusals.rs::a_bad_position_is_its_grid_code`: the canary in a text position, in a `pos` label, and a zero cell.</li><li>`a_refused_position_is_named_by_its_spec_number_not_quoted`: the spec number appears and the canary does not.</li></ul> |
| 3 | TEST-ONLY: pin the spec-level `usage` redaction | Two wrong-type members hold the canary: `context.soft` and `role`. | `refusals.rs::a_document_of_another_shape_is_usage_and_never_quoted` (T's spot check 2) |
| 4 | TEST-ONLY: pin that `updated` is the rename's stamp | <ul><li>**H1** waits until `now_millis() > old.updated` (a condition wait, not a sleep), then asserts `updated` equals the new log's `at`.</li><li>**Case 23** asserts the same equality on all three runners.</li></ul> | `crates/holler-hub/tests/profile_rename_test.rs:140-159`; `crates/holler-pane-testkit/src/conformance/profile_store/rename.rs:56-61` |

F also took round 1's advisory 8 items:
- the module-doc line is rewrapped;
- the guide's log wording is corrected for a reused name (D4);
- the CHANGELOG names `profile_rename_restore_failed`.

**I checked the redaction claim against the code myself.**
- By `from_decode` and `from_closed` (`crates/holler-pane/src/error.rs:568-596` and `:630-636`), a spec's decode can answer only:
  - `usage`, whose message is replaced;
  - the two grid codes, whose messages are replaced;
  - the unit variants `command-not-argv`, `env-name-invalid` and `profile-secret-refused`, whose `Display` is a fixed sentence.
- `Document::read` names only serde's category and the line and column.
- A wrong `format` names the expected format, and a wrong `version` names the number.

So no refusal quotes what a spec holds. See advisory 3 for one phrase that still says more than this.

## Acceptance criteria

The abbreviations used in this table:
- **H:** `crates/holler-hub/tests/profile_rename_test.rs`.
- **HF:** `profile_rename_faults_test.rs`, in the same directory.
- **C:** `crates/holler-pane-testkit/src/conformance/profile_store/rename.rs`, cases 23 to 28. It runs against three stores: the fake (`profile_store_conformance_test`), `ProfileState` (`profile_registry_test`, 28 cases), and the wire (`pane_integration` `clients::the_profile_client_passes_the_profile_store_conformance_suite`).
- **CLI:** `crates/holler-cli/tests/`.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The record: found by NEW and by its slug, gone under OLD, generation + 1, old `created`, the stamp as `updated`, equal specs, the reply equals the stored record | H `the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record`; C23 `rename-moves-the-record` | MET. Round 1's PARTIAL is closed: `updated` now equals the stamp. |
| 2 | Every member follows and stays attached (control) | H `every_member_follows_and_nothing_else_of_a_pane_changes`. Control: with T's mutation (`Locked::moves` moves no member), H2, H3 and H5 fail. | MET |
| 3 | Membership is by slug | H `membership_is_by_slug` | MET |
| 4 | The log stays intact and gains one entry; `at` never decreases; a deleted profile's log under NEW goes on | H `the_old_log_is_closed_and_the_new_log_has_exactly_one_renamed_entry` and `a_deleted_profiles_log_under_the_new_slug_goes_on`; C24, C25 | MET |
| 5 | The feeds: the deletion, then the put, then one pane put per member in name order, at consecutive cursors; a same-slug rename is one put | H `the_feeds_carry_the_deletion_then_the_put_and_one_put_per_member` and `a_rename_of_one_slug_is_one_put_one_entry_and_skips_a_member_naming_it_already`; C26, C27 | MET |
| 6 | Refusals change nothing, come in order, and carry their code on the wire | H `a_refused_rename_changes_nothing_and_answers_its_code_in_order`: 13 cases, comparing the files' bytes, both memories, the logs and both heads. C28. | MET |
| 7 | Fail closed | HF `a_pane_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written` (a missing OLD is still `store-corrupt`), and `a_profile_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written` | MET |
| 8 | Failed saves (control) | HF `a_failed_pane_save_is_unavailable_and_changes_nothing`, and `a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds`. The second checks the old bytes, a new inode (so the panes were saved first) and unchanged memory. | MET |
| 9 | Restart | HF `a_rename_survives_a_restart_and_a_resumed_watcher_learns_of_the_deletion` | MET |
| 10 | A retry completes an interrupted rename | HF `running_the_same_rename_again_completes_one_interrupted_between_the_saves` | MET |
| 11 | Concurrency, with a time bound | HF `two_renames_of_one_profile_racing_exactly_one_wins_and_every_member_names_it` and `a_member_written_during_a_rename_is_moved_or_refused_never_lost`. Each runs 20 rounds inside a 60 s bound. | MET |
| 12 | The membership hook after a rename | H `after_a_rename_a_pane_write_naming_the_old_name_is_refused_and_the_new_accepted` | MET |
| 13 | The `not-implemented` pins are replaced | H `profile_rename_without_params_is_a_usage_error_from_the_real_handler`; `pane_dispatch_test.rs` (`assert_answers_an_empty_profile_request`, `profile_dispatch_takes_both_handles_and_routes_rename_to_its_handler`); `profile_handlers_test.rs::a_corrupt_registry_answers_store_corrupt_on_every_profile_method` | MET |
| - | The conformance suite and the fake | C23 to C28 pass against all three stores. The mutants each fail their case: `a_rename_that_is_a_create_and_a_delete_fails`, `a_rename_that_skips_the_generation_check_fails` and `a_rename_that_overwrites_a_taken_name_fails`. Case 22's fifth refused write is now a stale rename. The fake still routes `rename` through `faults.enter(ProfileStoreOp::Rename)`. | MET |
| - | CLI `profile rename` | CLI `profile_verbs/rename.rs`, 9 tests: <ul><li>the exact text and JSON;</li><li>the generation read (seeded at 2) and the actor;</li><li>names with spaces, and a spelling-only rename;</li><li>`profile-not-found` 3, `profile-exists` 3 (two spellings), the same name `usage` 2 with no `Rename` call;</li><li>`generation-conflict` 1, through a seam that runs `concurrent_put`;</li><li>the positional count, through clap; a clap error under `profile` in JSON mode is a `usage` envelope (`process/usage.rs`);</li><li>no adapter call (`run_both_with`).</li></ul> | MET |
| - | CLI `profile export` | CLI `profile_verbs/export.rs`, 8 tests: <ul><li>the D7 document byte for byte, full and bare;</li><li>`rRcC` at four cells, and no record fields;</li><li>the environment canary appears in no output and no file;</li><li>`--out` writes at mode 0600, replaces an existing file and leaves no temporary file;</li><li>a missing directory is `usage`, and a missing profile is `profile-not-found`;</li><li>a control character in the path is escaped;</li><li>the count reads `1 spec`.</li></ul> | MET |
| - | CLI `profile import` | CLI `profile_verbs/import.rs` (7 tests) and `import/refusals.rs` (8 tests): <ul><li>detached, at generation 1, with the actor;</li><li>six position forms;</li><li>`--name` overrides the file's name and is parsed;</li><li>every refusal, in both formats, writes nothing and never prints the canary;</li><li>a file that leaves `env` and `expect` out exports them back as `[]`;</li><li>the round trip gives equal specs.</li></ul> | MET. Round 1's PARTIAL is closed. |
| 14-19 | Drift: it agrees with `show` (control: `env` compared as a set), once per pass, a pass stays a read, scope, a failed read, the finding's shape | CLI `pane_verbs/doctor/drift.rs`, 7 tests | MET |
| 20 | The registry of finding kinds | `crates/holler-pane/tests/findings_test.rs`: `CODES` has 14 entries with `profile-drift` last, plus `profile_drift_is_still_a_failure_class_code` | MET |
| 21 | Existing doctor tests unchanged | `doctor/rig.rs::profiles_of` builds each profile with `profile_from_panes`. The whole `pane_verbs` target passes. | MET |
| - | Process level | CLI `pane_verbs/process/profile_io.rs`, 2 tests; `STUBS` in `stub.rs` has no #665 entry | MET (advisory 6) |
| - | Docs and surface | <ul><li>`docs_pane_control_test.rs`: `profile_io_and_drift_are_built_not_proposed`, its negative control `the_built_not_proposed_rule_fails_a_proposed_line`, and `every_verb_is_named_in_a_span_or_heading`;</li><li>`docs_cli_test`, `cli_surface_test` and the ADR rows test.</li></ul> | MET |
| - | Lint | <ul><li>T-green's run is clean.</li><li>No touched `.rs` file nears 900 lines: the largest are 596 (a test) and 562 (`src/profile/store.rs`).</li><li>No new `unsafe`.</li></ul> | MET |

## Spec compliance

**The decisions are implemented as stated.** For D1 to D13, round 1's findings hold, and the cycle changed none of them:
- **D2 and D3:** one `Store::rename` takes the profile lock, then `PaneState::locked()`, and runs the checks in the brief's order. It saves the panes, then the profiles, then restores the panes if the profile save failed. It commits memory only after both saves.
- **D4 and D5:** OLD's log gets one `deleted` entry and NEW's one `renamed` entry, under one stamp, and the feeds are in the stated order.
- **D6:** `git diff --stat` is empty for `bind.rs`, `ports.rs`, `lib.rs`, `cli.rs`, `docs/protocol/v2.md`, `docs/orchestrating.md`, `entry.rs`, `grid.rs`, `read_only.rs` and `Cargo.lock`.
- **D7:** each pane is `to_value(spec)` with the grid replaced, and holler-cli declares `preserve_order`.
- **D8 and D9:** the steps run in order. Every position is read before any spec decodes, so the rework's routing does not change the order. Nothing is written before the last step.
- **D10:** drift is `diff_profile` over each profile's members in scope, with one `list` per pass, and only when a pane in scope names a profile.
- **D11:** every guide and test edit is in.
- **D12 and D13:** both hold.

**Recorded deviations.** Each one is sanctioned by A or forced by the code, and each is journalled:
- A warn 2b: a guard-holding handle, not a closure.
- A warn 3: the port doc's wording.
- A warn 4a: holler-cli declares `preserve_order`.
- A warn 5: file errors split into `usage` and `unavailable`, as `pane import` does.
- A warn 6: where the large new test suites and cases are placed.
- The issue's "edit none of #661's files" is overridden. The rename cannot be built without editing them.
- The guide uses placeholders such as `NAME` and `PANE`, so that `docs_cli_test` can parse each inline span.
- T put acceptance 16's allowlist in `drift.rs`. `read_only.rs` is unchanged, and its rig still makes no profile call, as acceptance 21 asks.
- Formatting uses `rustfmt` file by file. `cargo fmt -p` reformats about 130 files this story does not own (advisory 5).

## Quality audit

- **Correctness and failure handling.** I re-read the full `Store::rename`, `renamed` and `Locked`, and the restore path.
  - **Lock order.** The only hub path from the pane side to the profile registry is `check_membership`, which runs before the pane lock (`panes/handlers.rs:111-114`). No path takes the pane lock and then the profile lock.
  - **No member is lost.** `moves` is computed under the pane lock, so a racing `cas_put` is either moved or refused.
  - **NEW's log never goes back in time.** A tombstone at NEW's slug feeds the stamp through `entry::stamp(target, now)`, so `at` never decreases in NEW's log.
  - **The restore is a real save.** It writes the committed table (`write(table, &[])`), and a failure is logged as `profile_rename_restore_failed`.
  - **A bare relative `--out` works,** such as the guide's `fleet.json`. `write_atomic`'s `temp_path` uses `with_file_name`, so the temporary file lands in the current directory.
- **Build guards.**
  - No added line under `crates/*/src` contains `unwrap()`, `expect(`, `panic!`, `unreachable!`, `todo!`, `dbg!` or `unsafe`.
  - The only new `#[allow]`s are four lines in the two new hub test files, and each carries `// #665`.
  - Clippy's `dead_code = "deny"` is clean (T-green).
- **Protocol.**
  - There is no wire change: `ProfileRenameParams` and the reply already existed, and no golden file changed.
  - `docs/protocol/v2.md` is untouched, as the brief requires (advisory 2).
- **Tests.**
  - The hub tests use real registries on temporary state dirs, and the process tests use `LiveHub`.
  - There are no fixed sleeps. A grep of the touched tests finds only H1's yield loop, which waits on a condition.
  - Every race is bounded, and each test was RED before F's code.
- **Documentation.**
  - The CHANGELOG has an `[Unreleased]` / Enhancements bullet that links #665 and names `profile_rename_restore_failed`. The README lists no registry fault event, and the earlier registries named theirs the same way.
  - ADR-0003, ADR-0021 and the guide are updated.
  - In ADR-0021, `PROPOSED` remains only in "Decisions taken" item 4, which the brief keeps as the record of 2026-10-09.
- **Public-repository privacy.**
  - I grepped all 8,718 added lines for personal hosts, tailnet names, non-loopback IPs, account names, home paths, private domains and key patterns. The only hits are the brief's own rule sentence and round 1's description of the same grep.
  - Every commit's author is the GitHub no-reply address.
- **Commit and PR hygiene.**
  - Each subject is a Conventional Commit, and each commit carries `Co-Authored-By`.
  - No commit has a session link. That is the pipeline's standard format, and #648's merged run (`037405f`) is the same.
  - The PR does not exist yet. After the script opens it, the run's agent must add the AI disclosure with `gh pr edit` (CONTRIBUTING.md, "Disclosure is required").

## Scope check

**Delivered: the brief's scope, all three parts.** Round 1's accounting of the files outside the Blast radius still holds: each one comes from one of A's warns or from reuse expectation 8.

The rework cycle added only what round 1's REWORK asked for:
- two arms on `spec_refused`, and two message constants;
- `count()` in both `render` functions;
- the canary tests and the `updated` assertions;
- one import test for the gate's B-1;
- doc wording.

There is no unrelated refactor. Nothing is under-delivered.

## Verdict

**PASS.** Every acceptance criterion has a test that proves it, and the tests that guard the security and stamp rules are checked by mutation. Every decision is implemented as stated or as a recorded, sanctioned deviation, and round 1's four REWORK items are closed. Nothing below blocks.

## Advisory notes (non-blocking)

1. **Rebase before the merge.** `git merge-tree` against `origin/main` (`037405f`) still conflicts in `CHANGELOG.md`, `docs_pane_control_test.rs`, `ADR-0003.md`, `ADR-0021.md` and `docs/pane-control.md`. The brief's clash table expects this.
   - On `main`, "Decisions taken" item 22 is still free.
   - `UnmanagedStalled` is still the last finding kind, so `profile-drift` stays appended last.
   - The guide still says "four name none", so "five" is still right.
   - Re-run the workspace suite after the rebase.
2. **`docs/protocol/v2.md:817` still says "the proposed `profile/rename`".** The brief puts v2.md out of reach, so this is the brief's miss, not F's. Make the one-word fix while resolving the rebase's doc conflicts, or file a follow-up.
3. **One phrase still says more than the code does:** "a bad position or member is named by the spec's number". It appears in `src/profile/import.rs:50-52` (`--help`), `docs/pane-control.md:413` and `CHANGELOG.md:21`.
   - The problem: a member refused by a typed guard (`command-not-argv`, `env-name-invalid`, `profile-secret-refused`) prints the guard's fixed sentence. These codes are unit variants, so they cannot carry a spec number.
   - The safety half of the sentence ("no refusal quotes what a spec holds") is true.
   - Suggested wording: "a bad position, or a member that does not decode, is named by the spec's number".
4. **A-dup warn 1 (new).** `GRID_FORMS` and `GRID_BOUNDS` (`src/profile/export.rs:57,60`) copy holler-pane's private `FORMS` and `BOUNDS` (`crates/holler-pane/src/grid.rs:24,27`), and no test ties the copies to the originals. The follow-up is to expose the originals from holler-pane and delete the copies.
5. **Formatting.** Four new hunks in `docs_pane_control_test.rs` and one in `process/stub.rs:213` are not rustfmt-clean:
   - in `docs_pane_control_test.rs`: `BUILT_BY_665`, `PROFILE_IO_HEADINGS` and two `format!` calls;
   - the base already had 31 such hunks in `docs_pane_control_test.rs`, and CI has no fmt gate;
   - they can be tidied at the rebase.
6. **Carried over from round 1:**
   - advisory 3: `rename_on_an_empty_hub_reaches_the_real_handler` is answered by the CLI's own `get`. The round-trip test does reach the handler, so coverage holds.
   - advisory 4: `agent-key-invalid` folds into `usage`, so the key rule's text is lost.
   - advisory 5: case 28's final read is vacuous on the wire runner.
   - advisory 6: A-dup's warns. `pane_support::request` duplicates a helper, and `Locked` accepts any `PaneEvent`s.
   - advisory 7: the `unavailable` half of the file-error split is unpinned.
   - advisory 9: #693 is a known flake.

---

# Handoff-S: Phase 10 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)  (spec audit)

**Date:** 2026-10-11 (22:17 MDT)
**Branch:** `issue-665-implementation`, head `75d5ef4`, merge base `bda1069`
**Issue:** [#665](https://github.com/Performant-Labs/holler/issues/665) (confirmed by the operator 2026-10-11, 19:40 MDT)
**Source of truth:** `docs/handoffs/665-brief.md`, read with A's six warns as amendments (`handoff-A.md`, the run has no O)
**Handoffs reviewed:** `665/handoff-A.md`, `665/handoff-T-red.md`, `665/handoff-F.md`, `665/handoff-T-green.md`, `665/handoff-A-dup.md`, `665/decisions.md`, `665/evidence.md`, and the outside diff gate's round 1 (`docs/handoffs/665-diff-result-r1.md`, gitignored: PASS)
**Diff audited:** `git diff origin/main...HEAD` (58 files), every production file read in full
**Verdict:** REWORK (production; items 3 and 4 are T's)

## A precondition

Met. `handoff-A.md` is PASS, with 0 block and 6 warns. A-dup (`handoff-A-dup.md`) is PASS too, with 0 block and 3 warns.

## T precondition

Met.
- **RED.** `handoff-T-red.md` shows 70 failed tests in 14 targets. Each failure is the missing feature (`not-implemented`, clap's `unexpected argument`, no drift found), not a setup fault.
- **GREEN.** `handoff-T-green.md` shows the full workspace run on `3b20fc0`: 2,434 passed, 0 failed, 32 ignored. Lint, the changelog check, clippy `-D warnings` and `cargo machete` are clean.
- **Blocking issues:** none.
- **Mutation spot checks.** With the fake's deletion event removed, case 27 fails. With `Locked::moves` moving no member, H2, H3 and H5 fail. With `drifted` matching nothing, 4 of the 7 drift tests fail.

## Acceptance criteria

H = `crates/holler-hub/tests/profile_rename_test.rs`, HF = `profile_rename_faults_test.rs` (same directory), C = the conformance cases in `crates/holler-pane-testkit/src/conformance/profile_store/rename.rs`.

| # | Criterion (brief, "Acceptance") | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The record: found by NEW and its slug, gone under OLD, generation + 1, NEW's name and slug, old `created`, **the stamp as `updated`**, equal specs, reply = stored | H `the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record`; C23 `rename-moves-the-record` | **PARTIAL.** `updated` is checked only as `>=` (REWORK 4) |
| 2 | Every member follows and stays attached (control) | H `every_member_follows_and_nothing_else_of_a_pane_changes`; the control is T's `Locked::moves` mutation | MET |
| 3 | Membership by slug | H `membership_is_by_slug` | MET |
| 4 | The log is intact, with one new entry, `at` never decreasing, and a deleted profile's log goes on | H `the_old_log_is_closed_and_the_new_log_has_exactly_one_renamed_entry`, `a_deleted_profiles_log_under_the_new_slug_goes_on`; C24, C25 | MET |
| 5 | The feeds: the deletion then the put, one pane put per member in name order, consecutive cursors; a same-slug rename is one put | H `the_feeds_carry_the_deletion_then_the_put_and_one_put_per_member`, `a_rename_of_one_slug_is_one_put_one_entry_and_skips_a_member_naming_it_already`; C26, C27 | MET |
| 6 | Refusals change nothing, in order, each with its code on the wire | H `a_refused_rename_changes_nothing_and_answers_its_code_in_order` (13 cases: files, memory, logs and heads); C28 | MET |
| 7 | Fail closed | HF `a_pane_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written`, `a_profile_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written` | MET |
| 8 | Failed saves (control: a memory commit before the second save fails it) | HF `a_failed_pane_save_is_unavailable_and_changes_nothing`, `a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds` (the new inode shows panes are saved first) | MET |
| 9 | Restart | HF `a_rename_survives_a_restart_and_a_resumed_watcher_learns_of_the_deletion` | MET |
| 10 | A retry completes an interrupted rename | HF `running_the_same_rename_again_completes_one_interrupted_between_the_saves` | MET |
| 11 | Concurrency, bounded | HF `two_renames_of_one_profile_racing_exactly_one_wins_and_every_member_names_it`, `a_member_written_during_a_rename_is_moved_or_refused_never_lost` (60 s bound; 5 x 20 rounds in T-green) | MET |
| 12 | The membership hook after a rename | H `after_a_rename_a_pane_write_naming_the_old_name_is_refused_and_the_new_accepted` | MET |
| 13 | The `not-implemented` pins are replaced | H `profile_rename_without_params_is_a_usage_error_from_the_real_handler`; `pane_dispatch_test.rs` (two functions); `profile_handlers_test.rs::a_corrupt_registry_answers_store_corrupt_on_every_profile_method` | MET |
| - | Conformance and fake | Cases 23 to 28 pass on all three runners: the fake (`profile_store_conformance_test`), `ProfileState` (`profile_registry_test`, 28 cases) and the wire (`pane_integration` `clients`). The three rename mutants fail. Case 22's fifth refused write is a stale rename. The fake's `rename` still enters `ProfileStoreOp::Rename` | MET |
| - | CLI `profile rename` | `crates/holler-cli/tests/profile_verbs/rename.rs` (9 tests). The text and JSON are exact. The verb sends the generation it read (seeded at 2) and its actor. Codes and exits: not-found 3, exists 3 (two spellings), same name 2 (no `Rename` call), conflict 1 through a `concurrent_put` seam. The positional count is checked through clap. `run_both_with` asserts there was no adapter call | MET |
| - | CLI `profile export` | `profile_verbs/export.rs` (8 tests). D7 byte for byte (full and bare); `rRcC` at four cells; no record fields; the env canary in no output and no file; `--out` at 0600, replacing a file and leaving no temporary file; a missing directory is `usage`; not-found; a control character is escaped | MET, but the text is `1 specs` (REWORK 1) |
| - | CLI `profile import` | `profile_verbs/import.rs` (6 tests) and `import/refusals.rs` (7 tests). Detached at generation 1 with the actor; six position forms; `--name`; every refusal in both formats with no write; the round trip gives equal specs | **PARTIAL.** "The refused value is never printed" is unproven at the spec level (REWORK 3), and a refused position is quoted (REWORK 2). The text is `1 specs` (REWORK 1) |
| 14-19 | Drift: agrees with `show`, once per pass, read-only, scope, a failed read, the shape | `crates/holler-cli/tests/pane_verbs/doctor/drift.rs` (7 tests) | MET |
| 20 | The registry of kinds | `crates/holler-pane/tests/findings_test.rs` (`CODES` has 14 entries, `profile-drift` last; `profile_drift_is_still_a_failure_class_code`) | MET |
| 21 | Existing doctor tests unchanged | `doctor/rig.rs::profiles_of` uses `profile_from_panes`; `pane_verbs` passes (340) | MET |
| - | Process level | `process/profile_io.rs` (2 tests); `stub.rs` (`STUBS` has no #665 entry; the forced-diagnostic test is repointed) | MET (advisory 3) |
| - | Docs and surface | `docs_pane_control_test.rs` (`profile_io_and_drift_are_built_not_proposed`, its negative control, the renamed test); `docs_cli_test`; `cli_surface_test`; the ADR rows test | MET |
| - | Lint | `lint.sh` exits 0, clippy is clean, and the largest touched files are 596 lines (test) and 562 (src) | MET |

## Spec compliance

The brief's decisions are implemented as stated, apart from the deviations below. Each one is recorded and justified.

- **D2 and D3: the transaction.**
  - One `profile::store::Store::rename(.., panes: Option<&PaneState>)` takes the profile lock, then `PaneState::locked()`.
  - The order holds: both loads (`store-corrupt`), then `profile-not-found`, then `usage`, then `profile-exists`, then `generation-conflict`.
  - Saves run panes first, then profiles, then a restore through `Store::write` of the committed table. Memory changes only after both saves.
  - **I checked the lock order myself.** The only hub path from the pane side to the profile registry is `check_membership` in `pane/cas_put`, and it runs before the pane lock (`panes/handlers.rs:111-114`). So nothing takes the pane lock and then the profile lock.
  - The shape is A's warn 2b handle, not the brief's closure, and is recorded. The port is the record half (`None`), with the doc wording from A's warn 3.
- **D4 and D5.** The tombstone gets `deleted`, NEW gets one `renamed {from: stored name}`, one stamp is folded over both entries, and the feed order is as specified. A same-slug rename is one entry.
- **D6.** None of the listed files changed: `bind.rs`, `ports.rs`, `lib.rs`, `cli.rs`, `docs/protocol/v2.md`, `docs/orchestrating.md`, `read_only.rs`, `entry.rs` and `Cargo.lock` all have an empty `git diff --stat`.
- **D7.** The `Document` keeps `to_value(spec)` with the grid replaced. holler-cli declares `preserve_order` (A warn 4a), and the D7 document is pinned byte for byte.
- **D8 and D9.** The steps run in order and nothing is written before the last one.
  - *Deviation (A warn 5, recorded):* every file error is no longer `usage`. A path that names nothing usable is `usage`, and any other I/O failure is `unavailable`, as `pane import` does. ADR-0021 section 9 and the guide say so.
- **D10.** The drift step runs `diff_profile` over each profile's members in scope. For a pane in scope this is the same row `profile show` computes, because a row depends only on its spec and the pane of that name. One `list` is made per pass, and only when a pane in scope names a profile.
- **D11.** All seven guide edits and all four test edits are in. F added placeholders so `docs_cli_test` parses each inline `holler ...` span (F deviation 3).
- **D1.** The amendment landed in T's phase commit together with the tests, not alone (A's note: the script has no owner for it). The PR is squash-merged, so this does not matter.
- **Issue vs brief.** The issue's "edit none of #661's files" cannot hold (A warn 2d). This override is recorded in `decisions.md`.

## Quality audit

- **Correctness and failure handling.**
  - A failed load fails closed before any check.
  - A failed second save restores the pane file, and the failed restore is logged as `profile_rename_restore_failed`, documented in ADR-0021 "Rename as built".
  - A member moved under the pane lock cannot be lost: a racing `cas_put` is either moved or refused (HF, 100 rounds).
  - The outside gate's needs-verification items NV-1 to NV-5 are settled by reading the full `Store::rename`, `renamed`, the fake's `move_record` and the `reconcile` call site. Each is as the diff shows.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!` or `unsafe` was added in `src/`.
  - The only new `#[allow]`s are in the two new hub test files, and each carries `// #665`.
  - No touched file is near 900 lines.
  - Clippy's `dead_code = "deny"` is clean.
- **Protocol.** There is no wire change. `ProfileRenameParams` and the reply already existed, and no golden file changed. `docs/protocol/v2.md` stays as the brief requires (but see advisory 2).
- **Tests.**
  - The hub tests use real registries on temporary state dirs.
  - The new tests have no fixed sleeps, and the races are bounded.
  - There is one process-wide `set_var` with a unique name, following the precedent of `interrupt_test.rs`.
  - Gaps: REWORK 3 and 4.
- **Documentation.**
  - The CHANGELOG has an `[Unreleased]` / Enhancements entry linking #665.
  - The guide and both ADRs are updated.
  - The new log event is named in ADR-0021.
  - Three statements, plus the verb's `--help`, claim that nothing from a refused file is printed, which is broader than the behaviour (REWORK 2).
- **Public-repository privacy.** I grepped every added line for personal hosts, tailnet names, IPs (other than loopback), account names, private domains and key material. The only hit is the brief's own rule sentence ("no personal host, tailnet, ...").
- **Commit and PR hygiene.**
  - The subjects are Conventional Commits, and each carries `Co-Authored-By`.
  - There is no session link, which is the pipeline's standard commit format, the same as #648's merged run.
  - The PR does not exist yet. After the script opens it, the run's agent adds the AI disclosure with `gh pr edit` (CLAUDE.md, CONTRIBUTING.md).

## Scope check

**Delivered: the brief's scope, all three parts.** The files outside the brief's Blast radius each come from A's warns:
- `pane_integration/no_hub.rs`, `profile_registry_test.rs` and `ports_test.rs` (warn 1);
- both manifests (warn 4);
- the split test files, `pane_support` and the conformance `rename.rs` child (warn 6);
- create's `taken` made `pub(crate)` (reuse expectation 8).

The twin-store generalization (slices, `apply`, `after`) is required by the two-change save, not a drive-by refactor. Nothing is under-delivered except the test gaps below.

## Verdict

**REWORK** (`reworkKind: production`). F does items 1 and 2. T updates the strings that item 1 changes, and does items 3 and 4, which need no `src/` change.

1. **Spec counts are not pluralized.**
   - **Where:** `crates/holler-cli/src/profile/import.rs:116` and `crates/holler-cli/src/profile/export.rs:272`.
   - **The problem:** both print `"{} specs"` from a raw count, so a one-spec profile prints `1 specs`. `profile create` and `profile show` print `1 spec` for the same profile: every sibling verb renders this through `super::list::count`, at `create.rs:339`, `show.rs:126`, `list.rs:82`, `delete.rs:163` and `apply.rs:146`. The brief's `<N> specs` is the same template as `create`'s doc comment (`N specs`), which `create` renders with `count()`.
   - **Why now:** A-dup warn 2 asked for `count()` or a recorded decision that `1 specs` is intended. Neither exists: F's journal entry only defers to the pinned strings.
   - **Fix:** render with `count(n, "spec")` in both `render` functions.
   - **For T:** change `1 specs` to `1 spec` in `crates/holler-cli/tests/profile_verbs/import.rs:80` and `:159`, and in `crates/holler-cli/tests/profile_verbs/export.rs:318`.
2. **The never-printed claim is broader than what the code does.**
   - **The claims:**
     - `crates/holler-cli/src/profile/import.rs:45-46`, which is `holler profile import --help`: "no refusal prints what the file holds";
     - `import.rs:19-20`: "No message quotes what the file holds";
     - `crates/holler-cli/src/profile/export.rs:20-21`: "nothing from the file is echoed";
     - `docs/pane-control.md:411`: "no refusal prints what the file holds".
   - **What the code does:** a refused position is quoted.
     - `GridPos::parse` puts `excerpt(text)`, up to 64 characters of the file's text, into `grid-ambiguous` and `grid-out-of-range`. Import calls it in `read_position` (`export.rs:167`).
     - An object-form position whose `pos` label disagrees is quoted by `GridPos`'s deserializer (`crates/holler-pane/src/grid.rs:159-166`). As a closed code, it passes `spec_refused` unchanged.
   - **Fix, either of:**
     - **(a)** narrow the four statements to what holds: an environment value is never printed, and a spec member that does not decode is reported by the spec's number, never quoted. Say that a refused position is quoted the way the grid codes quote it.
     - **(b)** make the statements true. In import only, keep the two grid codes but replace their messages with the spec's number and the accepted forms or bounds, as `spec_refused` does for `usage`. T then pins this with `CANARY` in a text position and in a `pos` label.
3. **TEST-ONLY: the spec-level redaction is unpinned.**
   - **Where:** `crates/holler-cli/tests/profile_verbs/import/refusals.rs:111-134` (`a_document_of_another_shape_is_usage_and_never_quoted`).
   - **The gap:** neither spec-level `usage` case makes serde quote a value. `colour` and `generation` are unknown keys, and serde names a key, not its value. Removing `spec_refused`'s message replacement (`export.rs:175-184`) therefore leaves every test green. That replacement is what keeps a misplaced secret off the terminal.
   - **Fix:** add specs whose value holds `CANARY` with the wrong type, each refused `usage` (exit 2) and never printed. For example:
     - `spec["context"]["soft"] = json!(CANARY)` (serde: `invalid type: string "...", expected u32`);
     - `spec["role"] = json!(CANARY)` (serde: `unknown variant ...`).
4. **TEST-ONLY: acceptance 1's "the stamp as `updated`" is unpinned.**
   - **Where:** `crates/holler-hub/tests/profile_rename_test.rs:148` asserts only `stored.updated >= old.updated`, which still holds if the rename kept the old `updated`. Conformance case 23 (`crates/holler-pane-testkit/src/conformance/profile_store/rename.rs:43-61`) sets `updated` aside before it compares.
   - **Fix:** assert that `updated` equals the rename's one stamp:
     - in H1, `stored.updated == rig.log("Day Shift")[0].at`;
     - in case 23, `renamed.updated` equals the `at` of the new name's one log entry.

   Both the hub and the fake stamp the record and the entry with the same value, so this holds on all three runners.

## Advisory notes (non-blocking)

1. **The branch conflicts with `origin/main`** (`037405f`, after #648, #667 and #851) in `CHANGELOG.md`, `crates/holler-cli/tests/docs_pane_control_test.rs`, `docs/adr/ADR-0003.md`, `docs/adr/ADR-0021.md` and `docs/pane-control.md`.
   - The brief's clash table predicted this; `git merge-tree` confirms it.
   - "Decisions taken" item 22 is still free on `main`.
   - Re-run the workspace suite after the rebase.
2. **`docs/protocol/v2.md:817` still says "the proposed `profile/rename`".** The brief puts v2.md out of reach, so this is the brief's miss, not F's. Make the one-word fix at the rebase, or as a follow-up.
3. **`process/profile_io.rs::rename_on_an_empty_hub_reaches_the_real_handler` never reaches the hub's rename handler.** The CLI's own `get` answers `profile-not-found` first. `import_export_and_rename_round_trip_through_the_hub` does reach it (a stub would exit 1), so coverage holds. Only the name and doc overstate.
4. **F's handoff says `agent-key-invalid` passes through import. It does not.**
   - `decode_params` folds open codes into `usage`, because `from_decode` matches closed codes only. `spec_refused` then gives the generic spec message.
   - The behaviour matches the docs ("any other spec that does not decode is `usage`"), but the operator loses the key rule's text.
   - This is an optional follow-up. ADR-0021 section 9 is right not to list the code.
5. **Case 28's final "no event" read is vacuous on the wire runner,** because the client's eager first long-poll answers idle. H6's head check covers the hub.
6. **A-dup's warns 1 and 3 remain optional:**
   - warn 1: `pane_support::request` duplicates `profile_handlers_test.rs`'s `line` and `REPLY_WITHIN`;
   - warn 3: `Locked` accepts any `PaneEvent`s.
7. **The `unavailable` half of the file-error split is unpinned,** as T noted. A permission test is unreliable when CI runs as root.
8. **Smaller items:**
   - `crates/holler-hub/src/profile/store.rs:41` is an unwrapped 132-character doc line.
   - The guide's rename subsection says "the new name's log begins with it". That is inexact when a deleted profile's log under the new slug goes on (D4).
   - The CHANGELOG bullet does not name `profile_rename_restore_failed`, although the earlier registry entries named their events.
   - T-green's "no new `#[allow]`" is inaccurate, though the two new allows are compliant.
9. **#693** is still a known flake that can fail CI on a test this diff does not touch.
