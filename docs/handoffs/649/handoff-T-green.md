# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `8b70771` plus this phase's one test edit)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md`

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration` (F's tree, before this phase's edit) | `ok. 58 passed; 0 failed; 4 ignored; finished in 107.61s` |
| Same, after this phase's two added rows | `ok. 59 passed; 0 failed; 4 ignored; finished in 107.45s` |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` | `ok. 4 passed; 0 failed; finished in 54.67s` (scenario, profile scenario, grid, server argv under a real tmux pty) |

All 8 RED rows of Phase 4 now pass: `env::herdrs_own_session_variable_is_no_fallback`,
`env::no_herdr_session_is_usage_naming_holler_herdr_session`, `env::malformed_workspaces_are_usage_naming_the_variable`,
`store_only::a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session`, the three `fake_hub` eager-watch
rows, and `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server`.

After the opt-in run there was no `/tmp/hlr649-*` directory and no `hlr649`/`holler649` process. The one `/tmp/hlr649-scenario.bak`
file there dates from 11:24 MDT, an earlier phase's file, not this run's.

### Spot-check: the tests fail when the behaviour is removed

Each mutation was applied to F's production code, the named filter run, and the file restored with `git checkout`. The
tree was then rebuilt and the full target re-run green (above).

| Mutation | Caught by |
|---|---|
| M1 `watch()` makes no first request (lazy, as the trial was) (D4) | 3 failed: `fake_hub::watch_makes_its_first_request…`, `…a_cursor_ahead_of_the_head_is_refused_by_watch_itself`, `…watch_with_no_hub_is_unavailable_from_watch_itself` |
| M2 a write whose reply is lost maps to `unavailable` (D3) | `fake_hub::a_write_whose_reply_is_lost_is_timeout` |
| M3 a zero extent is accepted (D5) | `env::malformed_workspaces_are_usage_naming_the_variable` |
| M4 a relative path is accepted for every path variable | `env::a_path_variable_that_is_not_absolute_is_usage_naming_it` (new) and `env::a_relative_tmux_socket_is_usage_never_the_default_server` |
| M5 the `HERDR_SESSION` fallback restored (D5) | `env::herdrs_own_session_variable_is_no_fallback` |
| M6 `BindingHerdr` offers no pane id (D8 (a)) | `bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists` |
| M7 the misconfigured host falls back to `Unwired` (W-E) | `wired::…`: `host.ensure_session` and `host.ps` got `Err(NotImplemented)` |

## Test repairs and additions this phase

F listed no test as wrong. F named two untested edges of its own Design decision 6, which the wiring's module-doc table
documents to the operator. Each is now pinned by a compact row in `crates/holler-cli/tests/pane_integration/env.rs`:

- **New `env::a_path_variable_that_is_not_absolute_is_usage_naming_it`.** `""`, `herdr.sock` and `./h/herdr.sock` in
  `HOLLER_HERDR_SOCKET` (with a valid `HERDR_SOCKET_PATH` present, so it also pins that the next fallback is not used),
  `HERDR_SOCKET_PATH` and `HOLLER_OPENCODE_BIN` are each `usage` naming that variable. `HOLLER_TMUX_SOCKET` keeps its own
  D6 row.
- **`env::no_herdr_session_is_usage_naming_holler_herdr_session`** now also covers an empty `HOLLER_HERDR_SESSION`.
- **`env::the_workspaces_are_label_rows_by_columns`** now also covers the empty `HOLLER_HERDR_WORKSPACES` as no workspace.

`env.rs` is 260 lines.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (before and after the test edit) | PASS |
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all pass | 2309 passed, **1 failed**, 32 ignored: `wizard_scripts`' `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` | PASS for this story (see below) |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; no warning for any `pane_integration` or `wiring` file | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | ok | PASS |
| Unused deps | `cargo machete` | none | none | PASS |

**The one workspace failure is pipeline scratch, as F reported.** It is the rename scan over git-tracked files, and its only
hits are `docs/handoffs/649-brief.md:1197` and `docs/handoffs/649/decisions.md:66`. `git grep agent-skills/setup-wizard --
':!docs/handoffs'` finds nothing else. `docs/handoffs/` is removed before the push, so CI will not see it. It is not F's
code, and no test of this story is involved.

**Cross-check of F's figures.** F's 58/4 default and 4/4 opt-in results reproduce. F's workspace run stopped at the
first failing binary; this run used `--no-fail-fast` and reached every crate (2309 passed), so F's two split figures
(1607 + 702) are consistent with it. No discrepancy.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | Each acceptance row maps to a test (table below). | PASS |
| Test quality | Every Phase-4 test fails when its behaviour is removed (M1 to M7). The new rows are table rows inside existing tests, or one test for one documented rule, with no duplicate. The suite is proportionate: 59 default rows across 11 files, each under 600 lines. | PASS |
| Type safety | Clippy `-D warnings` clean; no `unsafe`, no `as` casts in the four wiring files; params are `holler_pane::reply`'s typed structs. | PASS |
| Error handling | D3's mapping table is one function (`store::call`/`timed_out`), and every row has a `fake_hub` test; config errors are kept and answered at first use (`env::malformed_adapter_settings_do_not_stop_the_store_only_verbs`, `wired::…`). | PASS |
| Data integrity | The pane and profile clients pass the test kit's conformance suites against a real hub (`clients::*`); a restarted hub serves the same registry (`store_only::a_restarted_hub_…`); two records on one Herdr pane are `unavailable`, never a pick (`bind::two_records_on_one_pane_are_unavailable`). | PASS |
| API contract | No protocol, golden-file or port change (the diff touches none of `holler-proto`, `holler-pane` or the goldens). | PASS |
| Security | D28's sentinel keys are grepped in every verb's output, every scratch file and `ps -eww` argv (scenario, passed). `serve_args`'s error quotes none of the value. `hub_unavailable` names the state dir and the method only. | PASS |
| Blast radius | `git diff --name-only origin/main...HEAD` (outside `docs/handoffs/`): every file is in the brief's Blast radius; `crates/holler-hub/src/pane_wiring.rs` is untouched (D16). | PASS |
| Evidence appendix | The new rows rely only on `wiring/env.rs`, which is in the diff, so `evidence.md` needs no new entry. | PASS |
| Migration safety, Playwright | N/A (no schema; no browser surface in this repo). | N/A |

## Acceptance criteria status

| Criterion | Status | Backed by |
|---|---|---|
| No hub: every verb exits 1 `unavailable` naming the hub, touches no adapter; the process stub tests use a started hub | PASS | `no_hub::every_verb_with_no_hub_…_in_json` / `_in_text`; `pane_verbs/process` (workspace run) |
| Store-only verbs work with no Herdr, tmux or OpenCode | PASS | `store_only::the_store_only_verbs_work_…`, `env::malformed_adapter_settings_do_not_stop_…` |
| A read never answered is `timeout` within 10 s; other read failures `unavailable`; JSON-RPC error `unavailable` | PASS | `fake_hub::a_read_the_hub_never_answers_…`, `…a_read_whose_reply_is_lost_is_unavailable`, `…a_json_rpc_error_is_unavailable_…` |
| Malformed `HOLLER_TMUX_SOCKET` is `usage` from host and harness, never the default socket | PASS | `wired::a_relative_tmux_socket_…` (M7), `env::a_relative_tmux_socket_…` |
| A write whose reply is lost is `timeout` | PASS | `fake_hub::a_write_whose_reply_is_lost_is_timeout` (M2) |
| Scenario on scratch instances only; real session names refused; guards in the default run | PASS | `scenario::the_scenario_runs_in_order_…` (opt-in), `guards::*` (7, default) |
| Scenario steps in order, ending with D21's repairs | PASS | `scenario::the_scenario_runs_in_order_on_scratch_instances` |
| No typing into a TUI; registry equals observed state at the end | PASS | same |
| Profile scenario | PASS | `scenario::the_profile_scenario_runs_on_scratch_instances` |
| Grid spellings at row 2, column 1 in Herdr's snapshot | PASS | `scenario::the_two_grid_spellings_…` |
| Every `--format=json` output and `watch` line is an envelope | PASS | scenario; `watch::pane_watch_prints_…` |
| The scenario's launch resolves `attach_tui` before the record exists | PASS | scenario launch steps |
| Server argv under a real tmux pty | PASS | `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops` |
| `attach_tui` resolves before the record, decorators over fakes (default run) | PASS | `bind::launch_resolves_attach_tuis_pane_…` (M6) |
| No credential printed, written or on an argv | PASS | scenario's D28 sentinel checks |

## Blocking issues

None.

## Advisory notes

- **`wizard_scripts`' rename scan fails on this branch while `docs/handoffs/` is tracked.** That is expected until Phase 11
  removes the directory. Whoever runs the workspace suite before then should read the failure as the scratch hit, not a
  regression.
- **`with_env` mutates the process environment** of a multi-threaded test binary (under the state-dir lock). Other tests
  in the target that spawn the binary while it holds the lock could inherit those variables. None does so today without
  that lock, and the run is green, but a future test spawning outside the lock could inherit a stray `HOLLER_TMUX_SOCKET`.
- **T-red's findings 1 to 3** (the TUI query without a UTF-8 locale, `pane switch` right after `pane relaunch`, the shared
  session store) stay O's to file. The scenario still carries its bounded `pane switch` retry for finding 2; it passed in
  this run, and its try count is printed only on failure, since cargo captures a passing test's output.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
