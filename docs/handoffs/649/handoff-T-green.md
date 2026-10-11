# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario  (second cycle: re-entry after the diff gate's round-1 BLOCK)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `22aa539` plus this phase's one test edit)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md` (second cycle)
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md` (second cycle)

This handoff replaces the first cycle's (`1c05f75`), which git history keeps. Everything that one confirmed (the 8 first-cycle
RED rows, mutations M1 to M7, the full acceptance table) still holds at this tree. The suite that pins it is unchanged
and still green. This cycle adds the W-T row and the 6 s pin.

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration` (F's tree, `22aa539`) | `ok. 61 passed; 0 failed; 4 ignored; finished in 119.06s` |
| `cargo test -p holler-cli --test pane_integration env::` (after this phase's edit) | `ok. 13 passed; 0 failed` |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` | `ok. 4 passed; 0 failed; finished in 67.75s`: the scenario, the profile scenario, the grid, and the server argv under a real tmux pty |

This cycle's RED row `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server` now passes, beside
its D6 sibling `wired::a_relative_tmux_socket_…`. The pin `fake_hub::a_read_and_a_write_answered_after_6_s_succeed` passes.
After the opt-in run there is no `/tmp/hlr649-*/` directory and no `hlr649` or `holler649` process. The loose
`/tmp/hlr649-*.txt` and `.bak` files are earlier phases' notes.

### Spot-check: the W-T row fails when the behaviour is removed

- **Reverting to `std::env::var(name).ok()`:** this is HEAD before F's change, and T-red's RED output shows the row failing
  on it.
- **M8, a lossy conversion:** `connect`'s lookup was set to `var_os(name).map(|v| v.to_string_lossy().into_owned())`. That is
  the form T-red and F rejected, because `/tmp/hlr649-\u{FFFD}/tmux.sock` is absolute.
  - Result: `wired::a_tmux_socket_that_is_not_utf8_…` **FAILED**, with `host.ensure_session: want usage naming
    HOLLER_TMUX_SOCKET; got Ok(())`, and the same for `host.ps`, `harness.health` and `harness.shown_session`.
  - So the row pins "malformed, refused", not merely "not the default socket".
  - The file was restored with `git checkout`, and `git status` was clean afterwards. No scratch directory or process was
    left by the mutated run.

## Test repairs and additions this phase

F listed no test as wrong. One table value was added:

- **`env::the_serve_args_are_a_json_array_of_strings_default_none`** now also refuses `""` as `usage` naming
  `HOLLER_OPENCODE_SERVE_ARGS`.
  - F's new docs (`connect`, `Lookup`, the module doc and the ADR sentence) state that a non-UTF-8 value reads as the
    empty value, and that the empty value is `usage` for every variable but `HOLLER_HERDR_WORKSPACES`.
  - Every path variable's empty value was already pinned (first cycle's `env::a_path_variable_that_is_not_absolute_…`,
    plus `HOLLER_TMUX_SOCKET`'s own rows), and so was the session's (`env::no_herdr_session_…`). The serve arguments'
    empty value was the one documented case with no test.
  - It passes against F's code, because `serde_json::from_str("")` is an error.
- I did not add a second process-level non-UTF-8 row. I agree with F's note: every variable goes through the one closure in
  `connect`, which the W-T row pins, and `env.rs`'s empty-value rows pin each reader.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean, exit 0 (also `-p holler-cli` after the test edit) | PASS |
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all pass | 151 binaries: 2312 passed, **1 failed**, 32 ignored. The failure is `wizard_scripts`' `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` | PASS for this story (see below) |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. The only warning is `crates/holler-pane/src/error.rs is 713 lines`, a file this story does not touch | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | ok | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Format | `rustfmt --edition 2021 --check crates/holler-cli/src/pane/wiring.rs` | clean | clean | PASS |

**The one workspace failure is pipeline scratch, as F reported.**
- Its hits are `docs/handoffs/649-brief.md:1197`, `docs/handoffs/649/decisions.md:66`, and this handoff's previous
  version, which quoted the name.
- `git grep -l agent-skills/setup-wizard -- ':!docs/handoffs'` finds nothing.
- `docs/handoffs/` is removed before the push, so this is not a regression.

**Cross-check of F's figures.** F's results reproduce:
- 61/0/4 for the default run;
- 4/4 for the opt-in run (67.75 s here, 82.48 s for F);
- 2312 passed, 1 failed and 32 ignored for the workspace;
- 3/3 for `docs_cli_test` and 3/3 for `wire_selftest`;
- clean clippy, lint, changelog and machete.

There is no discrepancy.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | The one new behaviour of this cycle (W-T) has its row. Every first-cycle criterion keeps its test (table below). | PASS |
| Test quality | The W-T row fails on both the unset read (T-red) and the lossy read (M8), so it pins the behaviour, not the call. It is a second value through the D6 row's shared check, not a copy. The 6 s pin fails at a 5 s bound (T-red's mutation). The added `""` is one table value in an existing row. The suite is proportionate: 61 default rows across 11 files, the largest `bind.rs` at 577 lines. | PASS |
| Type safety | Clippy `-D warnings` clean. F's change adds no cast, no `unsafe` and no new type. The lookup still satisfies D24's `Lookup` signature. | PASS |
| Error handling | `absolute` now answers the empty value before `PathBuf::from` with `<VAR> is empty or not UTF-8; it must be an absolute path`. It is still `usage` naming the variable, which every path row asserts. The no-session message still names only `HOLLER_HERDR_SESSION`, as `env::no_herdr_session_…` asserts through its helper `names_only_hollers_session_variable`. `serve_args` refuses `""` (added row). | PASS |
| Docs match the code | `Lookup`'s doc says `PATH`, `HOME` and `XDG_CONFIG_HOME` take the empty value as unset. That matches `env.rs`: `PATH` is `unwrap_or_default` with relative entries skipped, and the config home filters to absolute directories. `HOLLER_HERDR_WORKSPACES`' empty value is no workspace, as pinned by the first cycle's `env::the_workspaces_are_label_rows_by_columns`. | PASS |
| Data integrity, API contract | No protocol, golden-file, port or store change this cycle. | PASS |
| Security | No message quotes a value. The new texts name only the variable, and `serve_args` still quotes none of its input. A non-UTF-8 `HOLLER_TMUX_SOCKET` can no longer reach the operator's default tmux server (W-T row, trap untouched). | PASS |
| Blast radius | This cycle's production diff is `wiring.rs`, `wiring/env.rs` and `docs/adr/ADR-0021.md`, all in the brief's Blast radius. | PASS |
| Evidence appendix | The W-T row and the added `""` rely only on `connect`, `absolute`, `tmux_socket` and `serve_args`, all in the diff. `evidence.md` needs no new entry from T. F's eight W-S entries are present. | PASS |
| Migration safety, Playwright | N/A: no schema, and no browser surface in this repo. | N/A |

## Acceptance criteria status

| Criterion | Status | Backed by |
|---|---|---|
| No hub: every verb exits 1 `unavailable` naming the hub and touches no adapter; the process stub tests use a started hub | PASS | `no_hub::every_verb_with_no_hub_…_in_json` / `_in_text`; `pane_verbs/process` (workspace run) |
| Store-only verbs work with no Herdr, tmux or OpenCode | PASS | `store_only::the_store_only_verbs_work_…`, `env::malformed_adapter_settings_do_not_stop_…` |
| A read never answered is `timeout` within 10 s; other read failures are `unavailable`; a JSON-RPC error is `unavailable` | PASS | `fake_hub::a_read_the_hub_never_answers_…`, `…a_read_and_a_write_answered_after_6_s_succeed` (lower bound), `…a_read_whose_reply_is_lost_is_unavailable`, `…a_json_rpc_error_is_unavailable_…` |
| A malformed `HOLLER_TMUX_SOCKET`, **including a non-UTF-8 one**, is `usage` from the host and the harness, never the default socket | PASS | `wired::a_relative_tmux_socket_…`, `wired::a_tmux_socket_that_is_not_utf8_…` (M8), `env::a_relative_tmux_socket_…` |
| A write whose reply is lost is `timeout` | PASS | `fake_hub::a_write_whose_reply_is_lost_is_timeout` |
| The scenario runs on scratch instances only; real session names are refused; the guards are in the default run | PASS | `scenario::the_scenario_runs_in_order_…` (opt-in), `guards::*` (default) |
| The scenario's steps run in order, ending with D21's repairs | PASS | `scenario::the_scenario_runs_in_order_on_scratch_instances` |
| No typing into a TUI; the registry equals the observed state at the end | PASS | same |
| The profile scenario | PASS | `scenario::the_profile_scenario_runs_on_scratch_instances` |
| Both grid spellings land at row 2, column 1 in Herdr's snapshot | PASS | `scenario::the_two_grid_spellings_…` |
| Every `--format=json` output and `watch` line is an envelope | PASS | the scenario; `watch::pane_watch_prints_…` |
| `attach_tui` resolves before the record exists | PASS | `bind::launch_resolves_attach_tuis_pane_…`, and the scenario's launch steps |
| The server argv runs under a real tmux pty | PASS | `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops` |
| No credential is printed, written or put on an argv | PASS | the scenario's D28 sentinel checks |

## Blocking issues

None.

## Advisory notes

- **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace, not as `usage`.** This is F's known issue and Design
  decision 9. It fails closed: the adapter refuses every launch into an unlisted workspace, per `evidence.md`. It is not
  pinned at process level, which is consistent with the brief's scope. The operator gets the adapter's "workspace … is not
  configured", which does not name the variable.
- **The test tree is not bare-`rustfmt` clean.** The shared `support/` modules have long lines, and CI runs no `fmt`.
  This predates the story, and the one line added here follows the file's existing style.
- **The rename scan fails while `docs/handoffs/` is tracked**, as in both earlier cycles. Read that failure as the scratch
  hit.
- The first cycle's notes still stand:
  - `with_env` mutates the process environment under the state-dir lock;
  - T-red's findings 1 to 3 are O's to file.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
