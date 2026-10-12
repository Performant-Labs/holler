# Handoff-T-red: Phase 4 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (20:40 MDT)
**Branch:** `issue-665-implementation` (head `1185e7b` before this phase)
**Brief / wireframe reviewed:** `docs/handoffs/665-brief.md`, `docs/handoffs/665/handoff-A.md`; wireframe N/A (`uiSurface: false`)

## A precondition

Confirmed: A returned PASS on the plan (`handoff-A.md`, 0 block, 6 warns). T reads the warns as amendments, as A's notes ask.

## The contract amendment (D1), done first

A's notes say that D1's "amend first" had no owner in the script, and named T as the simplest one. T made the amendment before writing any test:

- **`docs/adr/ADR-0003.md`.** The three rows are now `holler profile rename OLD NEW`, `holler profile export NAME [--out FILE]` and `holler profile import FILE [--name NAME]` (`#665`). The "proposed" sentence is gone.
- **`docs/adr/ADR-0021.md`.** Every edit of the brief's "The contract amendment" is in:
  - the status line;
  - the section 2 row;
  - section 3 and its "Profile files as built" paragraph;
  - section 6 and its table row;
  - section 7;
  - section 8, with "except `profile/rename`" and "Rename as built";
  - the three section 9 rows, and the `profile-drift` class row;
  - "Decisions taken" item 22.

  The "Rename as built" sentence about the in-process port uses A's warn-3 wording: "`ProfileState` called as a port, which no hub path does". Item 4 still says PROPOSED. It stays as the record of 2026-10-09, as the brief says.
- **Left for F.** The section 7 lock-order sentence is F's to amend if F keeps the closure shape (A warn 2b). The doc comments in `src/` are F's too: `holler-pane/src/profile.rs`, `reply.rs`, `holler-proto/src/methods.rs`, and the hub's and CLI's module docs.
- **Docs that make the docs tests RED, and that F writes.** `docs/pane-control.md` and `CHANGELOG.md`. Their edits are listed exactly in the brief, under "The guide and its test: the exact edits".

## Tests authored

| # | Test (file :: name) | Pins (brief acceptance) | Tier, and why |
|---|---|---|---|
| H1 | `holler-hub/tests/profile_rename_test.rs` :: `the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record` | 1: the record | hub integration: the real handler over real registries. This is the cheapest tier that has both registries. |
| H2 | :: `every_member_follows_and_nothing_else_of_a_pane_changes` | 2 (control: a profile-only rename fails it) | same |
| H3 | :: `membership_is_by_slug` | 3 | same |
| H4 | :: `the_old_log_is_closed_and_the_new_log_has_exactly_one_renamed_entry`, `a_deleted_profiles_log_under_the_new_slug_goes_on` | 4 (D4), including `from` = the stored display name when the request spells OLD differently | same |
| H5 | :: `the_feeds_carry_the_deletion_then_the_put_and_one_put_per_member`, `a_rename_of_one_slug_is_one_put_one_entry_and_skips_a_member_naming_it_already` | 5 (D5) | same |
| H6 | :: `a_refused_rename_changes_nothing_and_answers_its_code_in_order` | 6: 13 refusals. Each leaves both files byte-identical, both memories, both logs and both heads unchanged. The order: not-found whatever the generation, then usage, then exists whatever the generation, then conflict. | same |
| H7 | :: `after_a_rename_a_pane_write_naming_the_old_name_is_refused_and_the_new_accepted` | 12 | same |
| H8 | :: `profile_rename_without_params_is_a_usage_error_from_the_real_handler` | 13 | same |
| H9 | `holler-hub/tests/profile_rename_faults_test.rs` :: `a_pane_registry_that_failed_to_load_is_store_corrupt_and_nothing_is_written`, `a_profile_registry_that_failed_to_load_...` | 7. A corrupt `panes.json` with a missing OLD is still `store-corrupt` (A warn 2b, the order of the checks). | same |
| H10 | :: `a_failed_pane_save_is_unavailable_and_changes_nothing` | 8a | same; the save fails because the file was replaced by a directory |
| H11 | :: `a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds` | 8b (control: committing memory early fails it). Also **panes saved first**: `panes.json` has its old bytes but a new inode. | same |
| H12 | :: `a_rename_survives_a_restart_and_a_resumed_watcher_learns_of_the_deletion` | 9 | same |
| H13 | :: `running_the_same_rename_again_completes_one_interrupted_between_the_saves` | 10 (the crash state is built by hand, then reloaded) | same |
| H14 | :: `two_renames_of_one_profile_racing_exactly_one_wins_and_every_member_names_it`, `a_member_written_during_a_rename_is_moved_or_refused_never_lost` | 11. Each runs 20 rounds on its own runtime and thread, bounded at 60 s, so a deadlock fails the test and never hangs it. | same |
| H15 | `pane_dispatch_test.rs` (two pins), `profile_handlers_test.rs` (the corrupt registry now covers `profile/rename` with valid params), `profile_registry_test.rs` (the case count is 28) | 13, and A warn 1 | edits to existing hub tests |
| C1 | `holler-pane-testkit/src/conformance/profile_store/rename.rs`: cases 23 to 28: `rename-moves-the-record`, `-logs-renamed-and-closes-the-old-log`, `-onto-a-deleted-name-continues-its-log`, `-of-one-slug-is-one-put`, `-feed-is-deletion-then-put`, `-refusals-change-nothing`. Case 22 (`failed-write-changes-nothing`) now uses a stale rename (`generation-conflict`) as its fifth refused write. `rename-is-not-implemented` is gone. | the conformance block (the record half, D3) | the conformance suite. It runs on the fake, on `ProfileState`, and over the wire through `HubProfileStore` (`pane_integration/clients.rs`, A warn 1). |
| C2 | `holler-pane-testkit/tests/profile_store_conformance_test.rs`: `DOCUMENTED_CASES` (28), and the mutants `RenameAsCreateAndDelete`, `RenameSkipsCas`, `RenameOverwritesTaken` | the conformance controls | mutation check |
| R1 | `holler-cli/tests/profile_verbs/rename.rs` (9 tests) | CLI rename: the text and JSON of Scope A, exactly. The verb sends the generation it read: the profile is seeded at 2, so any other value conflicts. The actor (from the log) is `holler profile rename`. Also: spaces; a spelling-only rename; `profile-not-found` and the same-name `usage`, each with no `Rename` call; `profile-exists` (2 spellings); a bad name; `generation-conflict` exit 1 through a seam that runs `concurrent_put` before the rename; the positional count. The verb writes no pane. | in-process verb over the fakes |
| E1 | `profile_verbs/export.rs` (8 tests) | CLI export: the D7 document byte for byte, for a full spec (with `opencode_agent`) and a bare one; `rRcC` at four cells; no record fields; **no env value**: the canary variable is set in the process, and appears in no output or file (control); `--out` writes 0600, replaces an existing file and leaves no temporary file, with JSON `out` and `document: null`; a missing directory is `usage` and writes nothing; `profile-not-found`; a control character in the path is escaped in the text | same |
| I1 | `profile_verbs/import.rs` (6 tests) and `import/refusals.rs` (7 tests) | CLI import: detached at generation 1, actor `holler profile import`, no pane call. A spec naming another profile's pane, or no pane, is accepted. `r2c1`, `c1r2`, `2,1` and `R2C1`, and both object forms, load as r2c1. `--name` overrides the file's name and is parsed. The refusals and their codes: secret (a list entry and a bare string), `env-name-invalid`, `command-not-argv` (command and check), both grid codes, `profile-exists` (3 ways), and `usage` (7 shapes plus 4 file cases). Each refusal is checked in both formats, writes nothing, and never prints the canary. The round trip: export, then import `--name Copy`, gives equal specs. | same |
| D1 | `holler-cli/tests/pane_verbs/doctor/drift.rs` (7 tests) | 14 to 19: drift agrees with `profile show` on one fixture (env compared as a set; missing and extra rows are not drift); one finding per pane per pass and no memory (control: one finding per field); one `List` and no profile write (whole-fleet and `--pane` passes, A warn 1); no profile call with no profiled pane; the scope; a failed `list` is `observe-failed`, concludes no drift, leaves the rest of the pass unchanged and exits 0; the shape (the `quoted` name, no value, `remedy` null, `not-fixable`, the text line) | the doctor rig: a reconcile pass and the verb over the fakes |
| D2 | `doctor/rig.rs`: `profiles_of` builds each profile from the stored records (`profile_from_panes`) | 21. Every existing doctor test passes unchanged. | rig |
| F1 | `holler-pane/tests/findings_test.rs`: `CODES` gains `profile-drift` last; `profile_drift_is_still_a_failure_class_code` | 20 | unit |
| X1 | `docs_pane_control_test.rs`: deletes `PROPOSED_PROFILE_VERBS`, `PROPOSED_FINDING` and the `heading_forms` skip; renames the test to `every_verb_is_named_in_a_span_or_heading`; adds `profile_io_and_drift_are_built_not_proposed` and the negative control `the_built_not_proposed_rule_fails_a_proposed_line` | D11 | docs test |
| X2 | `fixtures/cli-surface.txt` (the six #665 lines), `process/stub.rs` (the #665 `STUBS` entries removed, `// #665` kept), `pane_integration/no_hub.rs` (rows with their positionals, A warn 1) | the surface | fixture and process tests |
| X3 | `process/profile_io.rs` (new, declared in `process/main.rs`) | the process-level block: `rename` on an empty hub exits 3 with `profile-not-found`; import, then export to `--out` (equal documents), then rename, then show, all through a `LiveHub` | real binary, real hub |

Supporting edits:

- `pane_support/mod.rs` gains `request`, `rename_params` and `REPLY_WITHIN`, and names the two new files in its includer list.
- `process/stub.rs`'s `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` used `STUBS.first()`. That table is now empty, so the test uses `profile show` of a missing profile on a live hub (a refusal envelope, exit 3).
- `holler-pane/tests/ports_test.rs` has two stale "PROPOSED" comments reworded. Its own double still answers `not-implemented`.

## RED confirmation

The command is `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`. The workspace builds, clippy (`--all-targets -D warnings`) is clean, and `scripts/lint.sh` exits 0. The full run's list of failures is in "Workspace RED run" below.

Each failure is the feature's absence, never a setup error:

- **Hub (H1-H14):** `profile/rename` answers `not-implemented`. Examples:
  - `left: "not-implemented" right: "profile-not-found"` (H6);
  - `left: "not-implemented" right: "store-corrupt"` (H9);
  - `left: "not-implemented" right: "unavailable"` (H10, H11);
  - `called Result::unwrap() on an Err value: NotImplemented` (the success cases).
- **H15:** `profile/rename without params must be a usage error, got Err(NotImplemented)`. In `profile_handlers_test`, the corrupt-registry case gets `not-implemented` where `store-corrupt` is expected.
- **Conformance (C1):** the fake, the hub registry and the wire client each fail the 6 rename cases and case 22, with `rename(p, q, 2) failed with not-implemented` and `expected generation-conflict, got not-implemented`. `profile_registry_test`'s count is 28, so its suite run fails on the same cases.
- **CLI verbs (R1, E1, I1):** `["holler", "profile", "rename", "Demo", "Renamed"] must parse: error: unexpected argument 'Demo' found / Usage: holler profile rename [OPTIONS]`. The verbs have no positionals and no flags yet. Clap is part of the verb, so this is the missing behaviour. It is not a harness fault: once the arguments exist, every assertion below the parse runs. The positional tests fail on their own assertion (`FILE is required: Cli {... Profile(Import(ProfileImport)) }`, and `expected MissingRequiredArgument`).
- **Doctor (D1):** every assertion on the pass fails with no drift found:
  - `doctor and show agree: [] vs [(demo-one, [model.effort]), (demo-three, [herdr.grid, model.model_id, context.hard])]`. The show half of that test **passed**, so the fixture is what the test says.
  - `calls.profiles: [] vs [List]`.
  - `observe-failed: 0 vs 1`.
- **F1:** `FindingKind::ALL` lacks `profile-drift` (`left: [... "unmanaged-stalled"] right: [... "unmanaged-stalled", "profile-drift"]`).
- **X1:** the guide still says PROPOSED on 9 lines, and section 3 has no heading `holler profile rename OLD NEW`. `audit_content_and_section_3_headings_are_present` fails because the `heading_forms` skip was removed.
- **X2:**
  - `cli_surface_test`: 6 of 149 lines fail to parse (`Usage: holler profile rename [OPTIONS]`).
  - `docs_cli_test`: the new ADR-0003 rows and ADR-0021 examples give `unexpected argument found`.
  - `no_hub`: `want exit 1 ... unavailable; got code 2 ... unexpected argument 'hlr649'`.
- **X3:** `rename` exits 2 where 3 is expected, and `import` exits 2 where 0 is expected (`unexpected argument`).

These tests pass before F, by design:

- the three conformance mutant tests (the suite already fails on any store whose rename is not implemented);
- `a_pass_over_unprofiled_panes_makes_no_profile_call`, a guard on today's read-only rule;
- `profile_drift_is_still_a_failure_class_code` (`class_of` already says failure);
- `the_built_not_proposed_rule_fails_a_proposed_line`, the negative control of the docs rule;
- `every_verb_is_named_in_a_span_or_heading`;
- the repointed forced-diagnostic test.

Each is a control or a guard, not a test of the new behaviour.

## Notes for F (read before writing code)

1. **Do not run `cargo fmt -p <crate>`.** The repository is not rustfmt-clean, and there is no `rustfmt.toml` or fmt check in CI. `cargo fmt -p holler-cli` (and `-p holler-hub`) reformatted about 130 untouched files here, which T reverted. Format a **new** file with `rustfmt --edition 2021 <file>`. A file with `mod` children also formats those children. Leave edited files as they are.
2. **The D7 key order is pinned byte for byte** (`FULL_DOCUMENT`, `BARE_DOCUMENT`). A warn 4 option (a): declare `serde_json`'s `preserve_order` for holler-cli, with lint #149's marker, or otherwise guarantee the order. Never hand-copy `ProfileSpec`'s fields.
3. **The order of the port's checks**, in the conformance suite and the hub test: `store-corrupt` (hub), then `profile-not-found` whatever the generation, then `usage` (NEW equals OLD's stored name), then `profile-exists` whatever the generation, then `generation-conflict`. The fake and `ProfileState` must pass cases 23 to 28. `HubProfileStore` runs them over the wire too, through the full transaction.
4. **Panes first** is observable only by the inode of `panes.json` in H11. The restore has to be a real save (`write_atomic`) of the committed table, as A warn 2c says.
5. **File errors (A warn 5).** The tests pin `usage` only for a missing parent directory (export), and for a missing file, a directory, more than 1 MiB, not JSON and a wrong shape (import). A permission or I/O error on a usable path is not pinned, so F may follow A's `unavailable` alignment.
6. **Output shapes the tests pin:**
   - rename: `data: {"profile", "from": {"name", "slug"}}`;
   - export: `data: {"name", "slug", "specs", "out", "document"}`, where `document` is an object when there is no `--out`, else `null`;
   - import: `data: {"profile"}`;
   - the text lines exactly as Scope A and B give them;
   - with `--out`, the export path is printed through `show::text` escaping, so a raw ESC never reaches stdout.
7. **The drift message is pinned exactly:** `differs from profile <quoted(name)> on <a, b> (see holler profile show)`. The fields are in `SpecField::ALL` order. The doctor text line is `<pane> <pos> profile-drift: <message>`.
8. **Docs.** The guide edits of "The guide and its test: the exact edits" and the CHANGELOG bullet are F's. `docs_pane_control_test` stays RED until they land. The section 5 "N name none" count must become five on this base.

## Ready for F

Confirmed: RED is valid, and F may implement against these tests.

## Workspace RED run

The run finished at about 20:55 MDT. `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` gives **70 failed tests in 14 targets**. Every one of them is a test this phase authored or edited, and no other test failed.

The 14 targets:

- holler-cli: `cli_surface_test`, `docs_cli_test`, `docs_pane_control_test`, `pane_cli_process`, `pane_integration` (the `no_hub` rows and the wire conformance client), `pane_verbs` (doctor drift), `profile_verbs`;
- holler-hub: `pane_dispatch_test`, `profile_handlers_test`, `profile_registry_test`, `profile_rename_faults_test`, `profile_rename_test`;
- holler-pane: `findings_test`;
- holler-pane-testkit: `profile_store_conformance_test`.

Every existing doctor test passes with the new rig, which is acceptance 21.
