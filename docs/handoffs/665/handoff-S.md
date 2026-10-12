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
