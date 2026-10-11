# Handoff-T-green: Phase 7 - #650 Import today's panes from the launcher's files

**Date:** 2026-10-11
**Branch:** `issue-650-implementation` at `660efe8` (plus T's test repair below, uncommitted; T does not commit)
**Issue:** #650
**Handoff-F reviewed:** `docs/handoffs/650/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/650/handoff-T-red.md`

## Test repair (F's "Tests that look wrong")

F was right about both tests. Each read the pane store's call log after `rig.records()`, whose own `PaneStore::list` is
logged (the rig's doc says to read `records()` last), so the 3-call tail ended in that `List`.

- **The change:** `writes::a_failed_write_is_undone_and_answers_its_own_code` and
  `writes::a_write_that_landed_and_timed_out_is_undone_too` now take `pane_ops_tail(&rig, &mark, 3)` right after the
  run, before `records()`/`fleet()`.
- **What is unchanged:** the expected undo orders (`[CasPut, Get, Delete]` and `[Get, Delete, Delete]`) and every other
  assertion.
- **The file:** `crates/holler-cli/tests/pane_verbs/import/writes.rs` (only these two tests). No production code was
  touched.

**The gitleaks finding F reported is already resolved.** The sentinel at `tests/pane_verbs/import/sources.rs:170` is
now `zq-env-secret-c3c3`, and it was committed in `660efe8`.

- `gitleaks detect --log-opts HEAD~1..HEAD` reports no leaks.
- `gitleaks detect --no-git` over `tests/pane_verbs/import` also reports no leaks.

## GREEN confirmation

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs` | 306 passed, 0 failed (all 39 import tests included) |
| `cargo test -p holler-pane --test import_test` | 5 passed |
| `cargo test -p holler-cli --test cli_surface_test` | 3 passed |

**Mutation spot-check (scratch edits to production code, each reverted with `git checkout`; `git status` was clean
apart from T's test file afterwards):**

| Mutation | Caught by |
|---|---|
| `writes.rs` undo: never delete a record whose write landed (`landed.filter(\|_\| false)`) | `writes::a_write_that_landed_and_timed_out_is_undone_too` FAILED (1 of 39) |
| `assess.rs`: drop the `sources-disagree` finding (still no record) | `join::files_that_name_different_sessions_disagree_and_block` FAILED (1 of 39) |

The tests pin behaviour: removing either behaviour fails exactly the test that names it.

## Tier 1 results

| Check | Expected | Actual | |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0. The only warnings are for pre-existing files outside the diff; the largest file in the diff is `import/rig.rs` at 515 lines | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (rustc 1.98.1). The hub `too_many_lines` failure noted at RED no longer appears | PASS |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the pipeline's `test.unit.command`, plus `--no-fail-fast`) | all pass | 149 targets: 2160 passed, 0 failed, 25 ignored | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| `cargo machete` | no unused deps | none found | PASS |
| `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| `rustfmt --edition 2021 --check` on the repaired test file | clean | clean | PASS |
| Server start / API smoke | n/a | The verb's ports are `Unwired` until #649. F's real-binary runs (usage paths exit 2, then `not-implemented`) match the verb's contract | n/a |

**Cross-check of F's numbers:**

- F reported 304 passed and 2 failed in `pane_verbs`, and 2158 passed and 2 failed over the workspace.
- After the repair: 306 passed and 2160 passed, with no failure.
- The difference is exactly the two repaired tests. There is no other discrepancy.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per acceptance criterion | RED handoff's table mapped against the brief's AC 1 to 14; every AC has at least one passing test (below) | PASS |
| Test quality and proportion | Each test names one behaviour. The mutations above fail the named test alone. No test was found redundant: the 3 property tests in `import_test.rs` range over `ALL` and guard the set, they are not duplicates | PASS |
| Type safety | `#[allow`, `unsafe`, `todo!` and `unimplemented!` grep over every `.rs` in the diff: none | PASS |
| Error handling | Usage (exit 2), `import-blocked` (3), `herdr-version-unsupported` (3), the undo's `generation-conflict` / `timeout` / `profile-conflict` (1), observe failures and source errors are all tested | PASS |
| Data integrity | The undo order is pinned on the call log. A landed `timeout` write is deleted, and a failed undo step names what remains. A second run is a no-op, and an existing `fleet` refuses | PASS |
| API contract | The JSON envelope keys, `data.profile` against `profile_from_panes` and the store, and the text line form are all pinned; `cli-surface.txt` lists every flag | PASS |
| Secrets | Every source-error test first proves its secret is in the parser's text, then asserts it is absent from both formats. gitleaks is clean on the commit | PASS |
| File limits | Every test and support file is under 900 lines (max 515) | PASS |
| Protocol / goldens | No protocol, hub or golden change (`git diff 017cc6c --stat`) | N/A |
| Migration safety | No schema change; the store record shape is unchanged | N/A |
| Browser / Playwright | None in this repo | N/A |
| Evidence appendix | T added three entries under "T, Phase 7" in `evidence.md`: reconcile's `tmux-session-missing` spelling, `PaneCode`'s `session-not-found`, and the fake `health`'s `Ok(false)` for killed and frozen alike, each excerpt copied from source | PASS |

## Acceptance criteria status

| AC | Status | Backing tests |
|---|---|---|
| 1 Agreement | PASS | `join::agreeing_files_import_every_pane_with_its_session_of_record`, `join::the_text_form_lists_each_pane_in_name_order` |
| 2 Dry run writes nothing | PASS | `writes::a_dry_run_reads_only_and_reports_the_profile_it_would_create`, `writes::a_dry_run_prints_the_profile_as_profile_show_prints_a_spec` |
| 3 Real run | PASS | `writes::a_real_run_creates_fleet_and_every_member_matches_it` |
| 4 Join failures | PASS | `join::a_pane_with_no_labelled_herdr_pane_is_unlabelled_and_blocks`, `join::two_herdr_panes_with_the_label_are_ambiguous_and_block` |
| 5 Disagreement | PASS | `join::files_that_name_different_sessions_disagree_and_block`, `join::agreeing_files_naming_a_session_the_server_lacks_record_no_session` |
| 6 Server lacks the session / observe failures | PASS | `join::agreeing_files_naming_a_session_the_server_lacks_record_no_session`, `join::a_failed_health_check_is_observe_failed_and_health_unknown`, `join::a_failed_session_list_is_observe_failed`, `join::an_unhealthy_server_is_down_or_wedged_as_reconcile_says` |
| 7 Hand-edited and missing files | PASS | the 8 `sources::*` tests |
| 8 Not OpenCode and command shapes | PASS | `join::a_pane_that_is_not_opencode_is_skipped_and_the_others_import`, `join::a_command_array_is_the_argv_and_a_string_is_command_not_argv`, `join::a_directory_without_pane_files_is_ignored_and_blocks_nothing` |
| 9 Idempotence and `fleet` | PASS | `writes::a_second_real_run_is_a_no_op`, `writes::an_existing_fleet_with_a_pane_to_record_is_profile_exists` |
| 10 Failure records nothing | PASS (tests); see the advisory note | `writes::a_failed_write_is_undone_and_answers_its_own_code`, `writes::a_write_that_landed_and_timed_out_is_undone_too`, `writes::a_failed_profile_delete_is_profile_conflict_with_its_remedy`, `writes::a_failed_record_delete_stops_the_undo_with_its_remedy` |
| 11 Roles and flags | PASS | `import::an_import_needs_a_herdr_session`, `import::an_unknown_orchestrator_is_a_usage_error`, `sources::the_orchestrator_needs_its_own_pane_env`, `join::a_name_that_spells_another_cell_is_information_and_the_snapshot_wins` |
| 12 Vocabulary and docs | PASS | `import_test.rs` (5), `import::adr_0021_lists_the_import_codes`, `cli_surface_test`, `docs_cli_test` |
| 13 Herdr session | PASS | `join::the_join_reads_only_the_named_herdr_session` |
| 14 Herdr's version | PASS | `writes::an_unsupported_herdr_ends_the_import_with_its_error`, `import_test::herdr_version_unsupported_is_not_an_import_finding` |

## Blocking issues

None.

## Advisory notes

- **AC 10's follow-up issue is not filed.** AC 10 says "A follow-up issue is filed" (a verb that removes only a record).
  F takes no outward action, and neither does T, so O or S must file it before the merge.
- **T's test repair is uncommitted** in the worktree (`writes.rs`, plus `evidence.md`, `handoff-T-green.md` and
  `decisions.md`), for the run to commit.
- **Behaviour T could not test:** the real fleet's file shapes, the launcher table above all. Nothing a user runs changes
  until #649 wires the ports; the cutover's (#654) dry run covers both, as the brief says.
