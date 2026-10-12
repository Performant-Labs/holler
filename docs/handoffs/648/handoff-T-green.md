# Handoff-T-green: Phase 7 - #648 the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ

**Date:** 2026-10-11
**Branch:** issue-648-implementation (worktree `.claude/worktrees/0648b-roster-shown-driven`, head `f7f2bcf`)
**Issue:** #648
**Handoff-F reviewed:** `docs/handoffs/648/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/648/handoff-T-red.md`

## GREEN confirmation

```text
$ cargo test -p holler-cli --test roster_panes_test --test roster_cli_test --test docs_pane_control_test \
    --test docs_cli_test --test cli_surface_test --test pane_cli_process --test wire_selftest
cli_surface_test        ok. 3 passed; 0 failed
docs_cli_test           ok. 3 passed; 0 failed
docs_pane_control_test  ok. 16 passed; 0 failed
pane_cli_process        ok. 35 passed; 0 failed
roster_cli_test         ok. 9 passed; 0 failed
roster_panes_test       ok. 25 passed; 0 failed
wire_selftest           ok. 3 passed; 0 failed
```

All 29 tests from Phase 4 pass: the 25 in `roster_panes_test`, the 4 new ones in `roster_cli_test`, and the
replacement `section_4_documents_the_roster_columns`.

**Mutation spot-checks.** Each mutation below was made to `src/roster_cmd.rs` and reverted with `git checkout`. Every
one made at least one test fail, so the tests pin the behaviour and not the implementation:

| Mutation | Tests that failed |
|---|---|
| The pair rule is never true | 6: `two_mismatches_...`, `one_mismatch_...`, `format_json_...`, `profile::profile_narrows_...`, `registry::a_record_rewritten_...`, `registry::the_mismatch_rule_is_say_pane_targets_rule` |
| Join on the whole row name, not the label | 11, among them the base golden, the JSON test, the profile test, the cells test and the no-adapter test |
| The `--server` `NotRead` note is dropped | `registry::a_registry_not_read_under_server_says_so`, `roster_server_does_not_read_the_local_registry` |
| `--profile` reads the whole pane store, not the scope | 3 `profile::` tests, plus `roster_reads_the_real_registry_of_the_local_hub` (no `profile-not-found`) |
| The `--profile` row filter is removed | `profile::profile_lists_only_the_rows_of_its_panes_in_roster_order` |
| A policy refusal becomes `unavailable` (exit 1) | `hubless::a_server_refused_by_policy_exits_3_in_every_mode` |
| The summary line no longer de-duplicates a pane | `one_mismatch_...`, `format_json_...` |

After the spot-checks, `git status` shows no change under `src/`.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Full workspace suite (the pipeline's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0; across 153 result lines, 2402 passed, 0 failed and 32 ignored | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | rc 0, no warning | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. `roster_cmd.rs` is 578 lines, under the 600-line warning. `cli.rs` is 784, an existing warning. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Docs commands parse | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused dependencies | `cargo machete` | none | "didn't find any unused dependencies" | PASS |
| Help smoke | `HOLLER_STATE_DIR=<scratch> target/debug/holler roster --help` | legend after the options | Prints the legend: six columns, `hj-c1r2` / `r2c1`, the summary line, `--profile`, `--server` and both JSON shapes. rc 0. | PASS |

**Cross-check of F's results.** They agree with F's handoff: the same pass counts (25, 9, 16, 3 and 3), clippy
clean, lint 0, changelog ok, and the same workspace totals (2402, 0 and 32). This run had no #847 flake.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each AC | Mapped AC 1 to 14 to the tests below | PASS |
| Test quality (§7) | Each test names one behaviour. The mutations above show that each fails for its own reason. The tier is in-process over the test kit's fakes, with process tests only where `main.rs`, clap or `Wiring::connect` must be seen (AC 12, AC 13). No test duplicates another: the process tests check the real wiring, and the in-process ones check the bytes. None is redundant enough to delete. | PASS |
| Type safety | No `unwrap`, `expect`, `panic` or `#[allow]` added in `src/`. `REMOTE_POLICY_REFUSED` is checked at compile time (`RefusalCode::from_static`). | PASS |
| Error handling | Every error class is tested in every mode: usage (2), unavailable (1), refusal (3), profile-not-found (3), and store errors as a note with exit 0 | PASS |
| Data integrity | The registry is read on each call, and a record rewritten between calls shows its new values (`concurrent_put`). The roster writes nothing. | PASS |
| API contract | `--json` is additive: every hub key is kept. `--format=json` passes `check_envelope`, and `data` has exactly the keys `rows` and `mismatches`. ADR 0003 and ADR-0021 record both shapes. No protocol or wire change, so no golden or `v2.md` drift. | PASS |
| Security | `--server` never joins another hub's rows to the local registry (`NotRead`). The policy refusal still comes before any dial. No secret is in the output: rows carry token ids only, as before. | PASS |
| Migration safety | N/A (no schema or stored state) | N/A |
| Playwright / browser | N/A (no UI surface in this repo) | N/A |
| Evidence appendix | I appended a T section to `evidence.md`, with excerpts copied from source: the hub `Row`'s serde shape and flattened hold, `check_envelope`'s class-agrees-with-exit rule, `FakeProfileScope::members` and `resolve` (name order), the loopback-policy message, and `say_cmd::resolve_pane_target`'s signature. Today's widths are in the diff's removed lines, so they need no entry. | PASS |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 Old columns unchanged | PASS | `old_columns_are_unchanged_and_the_six_new_ones_follow_hold` (golden `base.txt`) |
| 2 Text mismatch | PASS | `two_mismatches_lead_with_one_summary_line_in_roster_order` (golden), `base.txt` (none) |
| 3 `--json` additive | PASS | `json_keeps_every_key_and_adds_the_six` |
| 4 Envelope | PASS | `format_json_is_one_envelope_of_rows_and_mismatches`, `profile::an_unknown_profile_is_refused_in_every_mode`, `profile::a_malformed_profile_name_is_a_usage_line_in_every_mode` |
| 5 POS | PASS | `old_columns_...` (no `c1r2` outside the name), `json_keeps_...` (`GridPos` form), `help_names_the_columns_and_that_a_pane_name_is_not_its_position` |
| 6 PROFILE / `--profile` | PASS | `profile::profile_lists_only_...`, `profile::an_unknown_profile_...` |
| 7 Same values as `pane list`, read fresh, no adapter | PASS | `registry::the_cells_are_pane_lists_cells_...`, `registry::a_record_rewritten_...`, `registry::the_roster_reads_the_registry_and_calls_no_adapter` |
| 8 Registry unavailable | PASS | `registry::a_not_implemented_store_...` (golden `no_registry.txt`), `registry::any_other_store_error_...`, `registry::a_failed_connect_is_a_registry_error` |
| 9 One mismatch, `pane(s)` | PASS | `one_mismatch_is_named_once_however_many_rows_carry_it` (golden) |
| 10 Join and `--profile` over rows | PASS | `join::a_row_with_no_label_...`, `join::a_labelled_row_is_never_joined_by_its_session`, `profile::profile_lists_only_...`, `profile::profile_narrows_the_summary_line_to_its_panes` |
| 11 Exit codes per class and format | PASS | `hubless::with_no_hub_the_result_is_todays_whatever_the_store`, `roster_usage_errors_are_clap_text_in_every_format`, `profile::an_unknown_profile_...` |
| 12 Real wiring end to end | PASS | `roster_reads_the_real_registry_of_the_local_hub` |
| 13 `--server` | PASS | `roster_profile_with_server_is_a_usage_error`, `roster_server_does_not_read_the_local_registry`, `hubless::profile_with_server_is_a_usage_line_before_any_dial`, `registry::a_registry_not_read_under_server_says_so` |
| 14 `--profile` over a registry that cannot answer | PASS | `profile::profile_over_a_registry_that_cannot_answer_is_its_error`, `profile::profile_after_a_failed_connect_is_its_error` |

## Blocking issues

None.

## Advisory notes

- **W-12 is not pinned.** The empty `--profile` line, `(no sessions of profile P on the hub)`, is F's choice, and no
  test asks for it, as the brief leaves it open. It is a cheap follow-up test if S or O wants it fixed.
- **`roster_cmd.rs` is not rustfmt-clean.** It has two small diffs: the `HUB_COLUMNS` array layout and a comment
  line. Neither CI nor `lint.sh` runs rustfmt, and the file at `4bc2088` already had 7 rustfmt diffs, so this is not a
  gate.
- **The branch is behind `origin/main`** by `d6ae539` (#851), which touches `cli.rs`, `main.rs`, ADR 0003, the guide
  and CHANGELOG. F placed its hunks away from it. The merge before the PR should re-run `docs_cli_test`,
  `docs_pane_control_test` and `changelog-check.sh`.
- **A's W-8 doc drift** (`reconcile.rs:181-183`, `pane/list.rs:239-241`) stays for O's follow-up, as F reports.
- **F's "Tests that look wrong (for T)" is "None".** No test was repaired in this phase. No test file changed.
