# Handoff-T-green: Phase 6 - #647 part 2: `unattached` and `agent-cannot-dispatch`

**Date:** 2026-10-12 (11:20 MDT; re-verified at about 11:55 MDT, see "Re-entry" below)
**Branch:** `issue-647-implementation`, at F's `8b6030e` plus T's two test edits below (committed as `4bd4e75`), and in the re-entry a whitespace-only fix to three test files (uncommitted; the driver commits it)
**Issue:** #647 (brief `docs/handoffs/647-part2-brief.md`)
**Handoff-F reviewed:** `docs/handoffs/647/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/647/handoff-T-red.md`

## Re-entry (Phase 7 call, at `4bd4e75`)

The driver called this phase again with no new F commit. `git diff 8b6030e HEAD -- 'crates/*/src' docs/adr docs/pane-control.md CHANGELOG.md` is empty, so the production code is the code T verified below, and the mutation results still hold. T re-ran everything from scratch at `4bd4e75`:

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the `test.unit.command`) | **171 binaries: 2688 passed, 0 failed, 39 ignored; exit 0.** 2688 is F's 2687 plus T's new `attached_test` case. The #847 flake did not recur |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 (re-run again after the fix below) |
| `bash scripts/lint.sh` | exit 0, warnings only. The largest touched test file is `pane_verbs/launch/rig.rs` at 865 lines, under 900 |
| `bash scripts/changelog-check.sh` | `changelog-check: ok` |
| `docs_cli_test`, `wire_selftest` | 3 passed each |
| `cargo machete` | no unused dependency |

**Correction to the rustfmt row below.** The first pass said "rustfmt clean on the 63 files". That was wrong. `rustfmt --check` flags 12 of the branch's files, and `cargo fmt --all --check` also fails on `main` across many untouched files: CI does not run rustfmt (no `fmt` in `.github/workflows/` or `scripts/lint.sh`), and the repo has no `rustfmt.toml`. To separate this branch's drift from what was already there, T ran `cargo fmt --all` in two throwaway detached worktrees, one at `origin/main` and one at HEAD, and compared the formatting diff file by file (both worktrees were removed afterwards):
- **Fixed (whitespace only):** `holler-adapter-opencode/tests/real_opencode_test.rs` (+15), `holler-cli/tests/pane_verbs/doctor.rs` (+4) and `holler-pane/tests/ports_test.rs` (+6). Each was rustfmt-clean on `main`, and only this branch's lines broke it. T copied rustfmt's output for those three files only. Each now passes `rustfmt --check`. After the fix, `ports_test` passes 8 of 8, `pane_verbs -- doctor` 71 of 71, and `real_opencode_test` compiles (14 opt-in tests, ignored by default). Clippy and lint are clean.
- **Left as is:** `docs_pane_control_test.rs`, `pane_integration/{scenario,wired}.rs` and `fake_host_test.rs`. On `main` these files already use long lines that rustfmt would wrap (298, 157, 48 and 13 lines of drift). The branch's new lines follow that style, so wrapping only them would be inconsistent churn. `pane_integration/bind.rs` has less drift than on `main`. CI does not check any of it.

No production code is touched. Verdict unchanged: PASS.

## GREEN confirmation

F changed production files only. `git diff --name-only a3a9e2b 8b6030e` lists no test file, and F flagged no test as wrong.

```
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
  171 test binaries: 2686 passed, 1 failed, 39 ignored (the opt-in real tests)
  failed: remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen
```

That one failure is the known open flake **#847** ("fails under the loaded workspace run"). The body's `last_seen` moved by one second, from 1791825139 to 1791825140. The branch changes nothing in `holler-hub`, `holler-body` or that test. Run on its own five times, it passed five times. It does not block this story.

Every test authored at RED passes. That covers `agent_view_test` and `findings_test` in `holler-pane`, the test kit's conformance cases 11, 17 and 13 with their mutants, `fake_harness_agent_test` and the fake tests, the adapters' `attached_test`, `fake_tmux_test`, OpenCode's `agent_view_test` and Claude's `hermetic_test`, `pane_verbs`' `doctor/attach.rs`, `doctor/agent.rs` and `read_only`, `pane_integration`'s forwarding, bind and wired tests, and `docs_pane_control_test`. All of them ran in the workspace run above.

**Spot-check: do the tests fail if the behaviour is removed?** Yes. `scratchpad/mutate.sh` changed one production file at a time, ran the tests that pin that behaviour, and restored the file with `git checkout`. `git status` shows no production file changed afterwards.

| Mutation | Result |
|---|---|
| M1 `can_dispatch`: `ask` counts as deny | killed by `the_permission_rules_decide_...`, but only by the rows T added in this phase (see below) |
| M2 `can_dispatch`: a rule whose permission is `*` is ignored | killed (`the_permission_rules_decide_...`) |
| M3 `RECENT_TURNS = 4` | killed (`every_recent_turn_unable_to_dispatch_...`, `fix_changes_nothing_for_it`, `a_second_pass_...`) |
| M4 `RECENT_TURNS = 2` | killed (`one_recent_turn_that_could_dispatch_raises_nothing`, `one_recent_turn_the_server_says_nothing_about_...`) |
| M5 `check_attached` never raises `unattached` | killed (6 tests in `doctor::attach`) |
| M6 the agent check ignores `opencode_agent` | killed (`a_record_that_names_build_...`, `..._the_orchestrator_...`) |
| M7 a turn the server says nothing about counts as stuck | killed (`one_recent_turn_the_server_says_nothing_about_raises_nothing`) |
| M8 no dispatcher check | killed (`no_dispatcher_raises_nothing`) |
| P1 `parse_attached` accepts counts that disagree | **survived** at first; killed by the test T added in this phase |
| P2 `parse_attached` reads an empty listing as 0 | **survived** at first; killed by the test T added in this phase |
| W7 `BindingHost::attached_clients` returns `Ok(0)` | killed (`forwarding::the_binding_host_forwards_attached_clients`, `..._over_the_wired_host_...`) |
| W7 `Wired<dyn HarnessPort>::agent_view` returns the default view | killed (`bind::the_wired_harness_forwards_every_method_...`, `bind::a_misconfigured_wired_harness_...`) |

The two W7 rows reproduce F's own W7 mutation results.

### Test changes in this phase (T's own tests; no production code touched)

1. **`crates/holler-adapter-opencode/tests/agent_view_test.rs`: four rows added to the `can_dispatch` table.** F sharpened ADR-0021 section 12 (Deviation 1). A later rule whose permission is `task` *or `*`* and that allows or *asks for* another pattern now reopens dispatch, but no test pinned the new wording. The new rows:
   - `[deny task *, allow * general]` is `Some(true)`;
   - `[deny task *, ask task general]` is `Some(true)`;
   - `[ask task *]` is `Some(true)`;
   - an unknown action (`maybe`) is `None`.

   The test's doc comment now uses the ADR's wording. File length: 414 lines.
2. **`crates/holler-adapter-host/tests/attached_test.rs`: a new test, `an_empty_listing_or_counts_that_disagree_are_unavailable`.** It covers the two strictness branches of `parse_attached` that no test pinned: a listing with no line, and two lines whose counts disagree. Both must be `unavailable`, never `0`. File length: 145 lines.
3. **`docs/handoffs/647/evidence.md`: one entry appended.** `assert_end_state` (acceptance 18) relies on the scratch rig's dead-end configuration naming no agent. The entry gives `scratch/mod.rs:314` and `real_opencode/rig.rs:69-86` and `:526-531`, with verbatim excerpts copied from source.

Both edited test files are rustfmt-clean and pass: OpenCode `agent_view_test` 13 of 13, and `attached_test` 6 of 6.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_...` | all pass | 2686 passed and 1 failed; the failure is #847's known flake, unrelated, and passes 5 of 5 on its own | PASS (flake noted) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean; re-run on the two adapter crates after T's edits, also clean | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0, before and after T's edits; warnings only, no file at 900 lines | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| rustfmt (not run by CI) | `rustfmt --edition 2021 --check`, one file at a time, on the 63 `.rs` files changed since `4973f9a` | no drift added by the branch | **corrected in the re-entry:** 12 files flagged. 3 had branch-only drift and were fixed; 4 follow their files' existing long-line style on `main` | PASS (after the re-entry fix) |
| Real tmux (opt-in) | `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored` (tmux 3.7c) | pass | 9 passed, conformance case 11 included | PASS |
| Real OpenCode (opt-in) | `HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1` (1.19.0) | pass | 14 passed, both `ac647` tests included | PASS |

**Cross-check against F's report.** F reported 2687 passed and 0 failed. T's run gave 2686 passed and 1 failed. The difference is exactly the #847 flake. Every other number matches F's: clippy, lint, changelog, real tmux 9 of 9, real OpenCode 14 of 14, and the W7 mutation outcomes.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per acceptance criterion | mapped below | PASS |
| Test quality | Each test names one behaviour. The negatives in `doctor/agent.rs` are guards that M6, M7 and M8 now prove they catch. The suite is proportionate: T added 1 test and 4 table rows, and deleted nothing (no test was redundant) | PASS |
| Type safety | No `unsafe` in the added lines. Every new `#[allow]` carries its issue link. `AgentView` and `TurnAgent` are `#[non_exhaustive]` (E0639 doc tests). No `any`-style escape | PASS |
| Error handling | Each error path has a test: `observe-failed` for a failed count and for a failed view, tmux `pane-not-found` and `unavailable`, a malformed count, OpenCode `session-not-found`, a message or agent list of the wrong shape, `timeout` and `process-unsupported` | PASS |
| Data integrity / read-only | `read_only.rs` uses the shared allowlist with exactly the two new reads. Both rules write nothing, and `--fix` does nothing for them (`fix_does_nothing_for_unattached`, `fix_changes_nothing_for_it`) | PASS |
| API contract | Two finding codes are added and no field is added or changed. `cli-surface.txt` and ADR-0003 are unchanged against `origin/main`. The remedy's `--agent=KEY` parses through clap, including a key with a leading `-` | PASS |
| Security / privacy | The message list is never quoted (`a_message_list_that_is_not_a_list_is_unavailable_and_quotes_nothing`), and the tmux output is never quoted (the `SECRETVALUE` check). The privacy grep of the added lines (personal host, user or path names) found nothing | PASS |
| Migration safety | No schema or persisted-state change | N/A |
| Commit order (acc 20) | `45bff85`, docs only (a `ports.rs` module doc), comes before every test or code commit on the branch | PASS |
| Module docs (acc 21) | `attached_clients` appears in `reconcile.rs` step 1 and its single-process paragraph, in `observe.rs`'s bound and in `process.rs`'s Observe bullet. `agent_view` appears in `reconcile.rs` step 1 and in `observe.rs`'s bound, which reads "at most nine port calls" | PASS |
| Playwright / browser | none in this repo | N/A |

## Acceptance criteria status

| # | Status | Backed by |
|---|---|---|
| 1 | PASS | `ports.rs` doc tests (two `compile_fail,E0046`, plus `PidsOnly`) |
| 2 | PASS | `agent_view_test.rs` (5 tests) and the two `E0639` doc tests |
| 3 | PASS | `findings_test::finding_kind_codes_are_stable` (18 codes) |
| 4 | PASS | the `findings_test` remedy tests and `doctor::agent::a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent` |
| 5 | PASS | the `fake_host_test` attached-client tests |
| 6 | PASS | host conformance case 11 and its two mutants (fake, wrappers, real tmux) |
| 7 | PASS | harness conformance case 17, Claude case 13, `fake_harness_agent_test`, `fake_claude_test` and `fake_harnesses_test` |
| 8 | PASS | `attached_test.rs` (now 6 tests) and `fake_tmux_test` `METHODS` |
| 9 | PASS | OpenCode `agent_view_test.rs` (rule table now 18 rows) and the real `ac647_*` tests |
| 10, 11 | PASS | `doctor/attach.rs` (8 tests) |
| 12, 13 | PASS | `doctor/agent.rs` (18 tests) |
| 14 | PASS | the once-per-pass and `--fix` tests in both files |
| 15 | PASS | `doctor/read_only.rs` and `doctor.rs`, through the shared `HOST_READS` and `HARNESS_READS` |
| 16 | PASS | every existing doctor test passes; `a_world_built_without_a_client_raises_unattached_for_every_pane` |
| 17 | PASS | `docs_pane_control_test::section_5_rows_of_part_2_name_their_exact_commands` |
| 18 | PASS (compiles in the default run; opt-in scratch run not executed here) | `scratch/verbs.rs::assert_end_state` and `scenario.rs::break_both`, with the evidence entry T added |
| 19 | PASS | the cli-surface diff is empty; lint, clippy and rustfmt are clean; no `unsafe`; the privacy grep is clean |
| 20 | PASS | the commit order above |
| 21 | PASS | the greps above |

## Blocking issues

None.

## Advisory notes

- **#847 flake.** `remote_admin_traffic_never_moves_the_bodys_last_seen` failed once under the loaded workspace run. It is unrelated to this diff and already tracked. CI may hit it too, and a re-run clears it.
- **Acceptance 18's scratch scenarios** need real Herdr, tmux and OpenCode, and T did not run them in this phase. They compile in the default run, and the claim they rely on is proved by `ac647_a_scratch_server_without_project_agents_has_no_dispatcher` (opt-in, passed).
- **F's known issues stand as F recorded them:** the unbounded message read (W-7), and W-3's remedy-with-`--profile` question. Neither is a test defect.
