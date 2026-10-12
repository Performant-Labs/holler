# Handoff-T-red: Phase 4 - #650 Import today's panes from the launcher's files

**Date:** 2026-10-11
**Branch:** `issue-650-implementation` at `243f910`
**Brief / wireframe reviewed:** `docs/handoffs/650-brief.md` (as amended at `ac17120`), `docs/handoffs/650/handoff-A.md`; no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (second run, on the merged tree), with 0 `block` and 6 `warn` findings. The brief
was not amended after that review, so the tests settle the warns that decide a test (see "Decisions F must know").

## Decisions F must know

The tests pin these. Each is also in `decisions.md`.

1. **`tmux-session-missing`, not `no-tmux-session`** (A finding 1). The brief's own rule is "a condition that reconcile
   already names keeps reconcile's spelling", and a code is stable once merged. A missing tmux session is the problem
   `FindingKind::TmuxSessionMissing.code()`. `session-not-found` is reused from the closed `PaneError` code. That leaves
   ten new codes, not twelve.
2. **Recorded health after a failed list** (A finding 2). Acceptance 6 and join item 4 say `unknown` when `health` or
   `list_sessions` returned an error. The tests pin `unknown` in both cases, as acceptance 6 states. A recommended
   reconcile's `healthy` for `Ok(true)` followed by a failed list, but the brief was not amended, and the field changes
   only what is displayed (the hub's gate refuses `Unhealthy(_)` alone). A healthy server whose list was read and lacks
   the session (`session-not-found`) records `healthy`. An unhealthy server records `Unhealthy("server-down")` or
   `Unhealthy("server-wedged")`.
3. **Two Herdr sessions** (A finding 3). The import's rig holds its own wrapper, `import/seams.rs` `ExtraHerdr`. It appends
   snapshot entries to the fake's own. No launch test module is widened.
4. **Acceptance 2 is an allow-list** (A finding 4):
   - `herdr`: `{Snapshot, Version}`
   - `host`: `{Ps}`
   - `harness`: `{Health, ListSessions}`
   - both stores: `{Get, List}`
   - no probe run
5. **Remedy commands quote the profile name** (A finding 5). Acceptance 10 matches `holler profile delete 'fleet'`,
   `holler profile delete 'fleet' --keep-panes` and `holler profile create 'fleet' --from-current`, built with
   `single_quoted` as in #662 B1.
6. **Where pane-less findings print is not pinned** (A finding 6). The tests pin the form of a finding line, `  <level>
   <code>: <message>`. They do not pin where pane-less findings print, or how a pane-less problem counts in N. The brief
   does not settle either, so F chooses and states it in `handoff-F.md`. Every `import-blocked` message the tests pin has
   exactly one pane with a problem, and no pane-less problem.
7. **Message formats the tests pin.** `import-blocked` text:
   `import blocked: N pane(s) have problems; run with --dry-run for the report`.

   `source-unreadable` messages:

   | File | The message holds |
   |---|---|
   | `pane.env` | `pane.env` and `line N` |
   | launcher table | `launcher.toml`, `line L` and `column C` (from `span()`, 1-based) |
   | `sessions.toml`, a `ConfigError::Toml` | `sessions.toml` and `not valid TOML` |
   | `sessions.toml`, a `ConfigError::Invalid` | `sessions.toml` and the field (`mode`) |

   `observe-failed` messages hold `harness.health`, `harness.list_sessions` or `host.ps`. `server-down` and
   `server-wedged` hold reconcile's sentence, `the harness server on port N ...`.
8. **The API the tests name.**
   - The verb's flags: `--from`, `--herdr-session`, `--launcher`, `--orchestrator` (repeatable) and `--dry-run`.
   - `holler_pane::import::IMPORT_BLOCKED`.
   - `holler_pane::import::findings::ImportFinding`, with `ALL` (iterable), `code(self) -> &'static str`, `level(self)`
     (a `Serialize` value whose JSON is `problem`, `warning`, `skipped` or `info`) and a `Serialize` that writes the
     code. Variant names and declaration order are F's.
9. **`stub.rs`.** The `("pane", "import", 650)` entry is deleted. The `// #650` line stays, as the file's own rule says
   (prior stories kept theirs), where the brief asks to delete it.

   `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` needed a pane stub, and `pane import` was the last one.
   It now takes the first stub of either namespace. The path it tests is the same.

## Tests authored

All in-process on the test kit's fakes, at the cheapest tier that reaches the behaviour: the verb run through
`run_verb_with`, with a temporary `<state-root>`. The pure code table is a `holler-pane` integration test.

### `crates/holler-pane/tests/import_test.rs` (pure, unit tier)

| Test | Pins |
|---|---|
| `import_finding_codes_are_the_pinned_set` | AC 12: the code set, unique, kebab-case, JSON = code; the reused codes come from their one source |
| `import_finding_reuses_reconcile_spellings_and_adds_no_synonym` | AC 12, A finding 1: no new code is a reconcile code, no `no-tmux-session` |
| `herdr_version_unsupported_is_not_an_import_finding` | AC 14: never a finding |
| `each_import_finding_has_its_level` | "Exit codes and what blocks": each code's level; only `problem` blocks |
| `import_blocked_is_an_open_refusal_code` | `import-blocked` is an open code, refusal, exit 3 |

### `crates/holler-cli/tests/pane_verbs/import.rs` and `import/*.rs` (in-process verb tier)

The rig is `import/rig.rs`. It reuses doctor's world with a new unrecorded seed (`doctor/rig.rs`: `Seed::unrecorded`),
labels each Herdr pane with its name and writes agreeing files. The seams are `import/seams.rs`:

- `ExtraHerdr`;
- `NthCasPut`, copied from `profile_verbs/rig.rs`, which A lists as a deliberate copy.

| Test | AC |
|---|---|
| `import::an_import_needs_a_herdr_session` | 11: usage, exit 2, no adapter call, no write |
| `import::an_unknown_orchestrator_is_a_usage_error` | 11: exit 2 before any adapter call |
| `import::adr_0021_lists_the_import_codes` | 12: section 9's row lists the six codes; the open-codes row lists `import-blocked` |
| `join::agreeing_files_import_every_pane_with_its_session_of_record` | 1: every record equals the expected `Pane` field for field (herdr = snapshot entry with label `None`, launcher port/cwd, port not name-derived, `pane.env` model/effort/ceilings/agent/env names, role from the flag, `fleet`, `last_observed.at` 0, Herdr version); name order; JSON `pos` row first |
| `join::the_text_form_lists_each_pane_in_name_order` | 1: `NAME ACTION ROLE POS SESSION`, `r2c1`, name order |
| `join::a_pane_with_no_labelled_herdr_pane_is_unlabelled_and_blocks` | 4: `pane-unlabelled`, text finding line, real run `import-blocked` exit 3 in both formats, nothing written |
| `join::two_herdr_panes_with_the_label_are_ambiguous_and_block` | 4: `pane-label-ambiguous` naming both pane ids |
| `join::the_join_reads_only_the_named_herdr_session` | 13: both halves, through `ExtraHerdr` |
| `join::a_name_that_spells_another_cell_is_information_and_the_snapshot_wins` | 11: `name-cell-differs` (info) naming both cells; the record uses the snapshot's cell |
| `join::files_that_name_different_sessions_disagree_and_block` | 5: the three disagreements; real run refused |
| `join::agreeing_files_naming_a_session_the_server_lacks_record_no_session` | 5 (I6) and 6: `session-not-found`, no session of record, the stale id in no record |
| `join::a_failed_health_check_is_observe_failed_and_health_unknown` | 6 |
| `join::a_failed_session_list_is_observe_failed` | 6 (decision 2) |
| `join::an_unhealthy_server_is_down_or_wedged_as_reconcile_says` | 6: `kill` gives `server-down`, `freeze` gives `server-wedged`, `Unhealthy(code)`, never `observe-failed` |
| `join::no_recorded_session_is_a_warning_and_prints_a_dash` | join item 4: absent or blank `last-session`; text `-` |
| `join::a_missing_tmux_session_is_reconciles_problem_and_blocks` | join item 3 (decision 1) |
| `join::a_failed_ps_is_observe_failed_and_the_pane_imports` | join item 3: a warning naming `host.ps` |
| `join::a_pane_that_is_not_opencode_is_skipped_and_the_others_import` | 8 |
| `join::a_command_array_is_the_argv_and_a_string_is_command_not_argv` | 8 |
| `join::a_directory_without_pane_files_is_ignored_and_blocks_nothing` | 8 |
| `sources::sessions_toml_that_is_not_toml_names_the_file_only` | 7: the secret is first shown in `ConfigError::Toml`'s text, then absent from both formats |
| `sources::sessions_toml_with_an_invalid_row_names_the_row_and_field_only` | 7: the secret is first shown in `Invalid.reason`; neither the row's `name` nor its `reason` is printed |
| `sources::a_launcher_table_that_is_not_toml_names_the_file_line_and_column` | 7: the secret is first shown in the `toml` error text; line and column come from `span()` |
| `sources::a_missing_or_partial_launcher_entry_is_source_missing` | 7: no entry, no port, no cwd |
| `sources::a_pane_env_line_that_does_not_parse_names_the_file_and_line_only` | 7: `export OC_MODEL = x`, no `=`, a leading space, a non-numeric ceiling; `line 3` |
| `sources::a_missing_pane_env_or_required_variable_is_source_missing` | 7: the missing file, and each of the four required variables named |
| `sources::the_orchestrator_needs_its_own_pane_env` | join item 5 |
| `sources::a_missing_state_root_is_a_usage_error` | 7: exit 2, one line |
| `writes::a_dry_run_reads_only_and_reports_the_profile_it_would_create` | 2: allow-list, no write, file bytes unchanged, envelope keys, `data.profile` equal to `profile_from_panes(fleet, expected records)`, `recorded` 0 |
| `writes::a_dry_run_prints_the_profile_as_profile_show_prints_a_spec` | The output: `spec PANE` and `  FIELD: VALUE` lines |
| `writes::a_real_run_creates_fleet_and_every_member_matches_it` | 3: `data.profile` as stored (generation 1, the store's clock), equal to the dry run's except the stamps; `profile show fleet` all `matches`; the secret absent; source files unchanged |
| `writes::a_second_real_run_is_a_no_op` | 9 |
| `writes::an_existing_fleet_with_a_pane_to_record_is_profile_exists` | 9 |
| `writes::a_state_root_with_no_pane_writes_nothing` | "Exit codes": no candidate means exit 0, nothing written |
| `writes::a_failed_write_is_undone_and_answers_its_own_code` | 10 (a): `generation-conflict`, exit 1, store empty, `fleet` gone; undo order pinned on the log |
| `writes::a_write_that_landed_and_timed_out_is_undone_too` | 10 (b): `timeout` |
| `writes::a_failed_profile_delete_is_profile_conflict_with_its_remedy` | 10 (c) |
| `writes::a_failed_record_delete_stops_the_undo_with_its_remedy` | 10 (d) |
| `writes::an_unsupported_herdr_ends_the_import_with_its_error` | 14: both runs, exit 3, no report, no write |

### Fixtures and other test files

- `crates/holler-cli/tests/fixtures/cli-surface.txt`: the two `#650` lines now cover every flag (AC 12).
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: see decision 9.

## RED confirmation

**1. `cargo test -p holler-cli --test pane_verbs`:** 267 passed, 39 failed. The 39 failures are exactly the new import
tests; every other test passes, doctor's included, with the rig change.

- 38 fail where the in-process harness parses the argv, because the verb has none of its flags yet:

  ```
  ["holler", "pane", "import", "--from", "/…/.tmpXXXX", "--herdr-session", "scratch", "--dry-run"] must parse: error: unexpected argument '--from' found
  ```

  The panic comes after each test has built its rig and files, so the setup is valid up to the call. `sessions::*` and
  `launcher` check before the call that their fixture's secret is in the parser's own text, and those checks pass.
- 1 fails on its assertion. The ADR row does not list the codes yet:

  ```
  import::adr_0021_lists_the_import_codes panicked at import.rs:100:9:
  the import row names `profile-conflict`: | `pane import` | `profile-exists` (a profile `fleet` already exists); …
  ```

- **Deeper check (scratch, reverted).** With the five flags declared on the stub only, all 38 reach their behaviour
  assertions and fail on them. For example:
  - `the run succeeds: Outcome { code: 1, … "not-implemented" … }`;
  - `left: (1, "not-implemented")` against `(1, "generation-conflict")`;
  - `left: 1` against exit 2 for the usage cases.

  No test failed on a rig, fixture or harness error. The scratch edit was reverted with `git checkout`, and
  `crates/holler-cli/src/` is unchanged.

**2. `cargo test -p holler-cli --test cli_surface_test`:** `every_surface_line_parses` fails. It names the two new
`pane import` lines (`unexpected argument '--from'`).

**3. `cargo test -p holler-pane --test import_test`:** the build fails on exactly the two items F adds:

```
error[E0432]: unresolved import `holler_pane::import::findings`
error[E0432]: unresolved import `holler_pane::import::IMPORT_BLOCKED`
```

A RED for a type that does not exist yet can only be a compile error.

- **The rest of the file (scratch stub, reverted).** Against a one-variant scratch stub, the file compiles without
  error. `import_finding_codes_are_the_pinned_set` and `import_blocked_is_an_open_refusal_code` fail on their
  assertions.
- **The three property tests** (levels, no synonym, never `herdr-version-unsupported`) range over `ALL`. They pass
  vacuously on a one-variant stub and become meaningful with the full set. They are guards, not REDs of their own.

**4. `cargo test -p holler-cli --test pane_cli_process`:** 36 passed, after the `stub.rs` change.

**Tier 1 on the test code:**

- `cargo clippy -p holler-cli --no-deps --tests -- -D warnings` is clean.
- `bash scripts/lint.sh` exits 0. No import file is over 600 lines: `rig.rs` is 515 and `join.rs` 456.
- Workspace clippy on this machine's stable 1.98 toolchain fails on three `too_many_lines` in `holler-hub`. That code
  is untouched and the failure predates this change; it is not this story's.

## Ready for F

Confirmed: RED is valid, and F may implement against these tests. F should read "Decisions F must know" first.
Decisions 1, 5 and 7 fix strings that F's code must produce.
