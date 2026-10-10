# Handoff-T-green: Phase 7 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `2d9c7a0`, base `3bdd129`)
**Issue:** #663
**Handoff-F reviewed:** `docs/handoffs/663/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/663/handoff-T-red.md`

## GREEN confirmation

The tests were not edited. Both test modules, from `#[cfg(test)]` to the end of the file, are byte-identical to the RED
commit `8fe683b`:
- `probe.rs`: sha256 `3512b7ca...`;
- `profile_scope.rs`: sha256 `adea55d8...`.

F flagged no test as wrong ("Tests that look wrong (for T)": none), and I found none, so no test was repaired.

```
$ cargo test -p holler-cli --lib pane::profile_scope
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out
$ cargo test -p holler-pane --lib probe::tests
test result: ok. 12 passed; 0 failed; ... finished in 0.53s
$ cargo test -p holler-pane --lib probe::tests -- --test-threads=1
test result: ok. 12 passed; 0 failed; ... finished in 1.39s           (AC 8's serial budget is 30 s)
$ cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success
test result: ok. 1 passed                                               (8m)
```

**Flake check.** I ran `probe::tests` 20 times in parallel and 5 times serially: 25 passes, 0 failures. Afterwards there
was no `hlr-probe-663-*` directory under the temp dir and no orphaned `sleep 30`. The one `sleep 30` on the host belongs
to another session's `gh pr checks` loop, whose parent is a zsh `until` loop.

**The tests pin behaviour, not implementation.** I made ten mutations of F's code, one at a time, and restored each one
from git. Each is the smallest change that removes one behaviour and still compiles; `dead_code = "deny"` makes a deleted
call a compile error, not a test result, so calls were disabled with `if false`. Every mutation turns at least one
authored test red:

| Mutation | Behaviour removed | Killed by |
|---|---|---|
| M1 | the group kill (`kill_group` is never called) | 8f `timeout_kills_the_whole_process_group`, 8g `background_child_holding_stdout_is_a_timeout` |
| M2 | the exit-status check (a non-zero exit is judged on its output) | 8d, 8j |
| M3 | `contains` always true | 8b, 8j |
| M4 | the 1 MiB cap | 8i `output_over_the_cap_is_error` |
| M5 | the restore after a failed act | AC 1 (conformance), AC 2, AC 3 |
| M6 | the first-write `timeout` message (passed through bare) | AC 4 |
| M7 | the `'\''` escape in `single_quoted` | AC 5 |
| M8 | the pane-record read before a `Remove` | AC 7 |
| M9 | the `Set`-filed-under-another-pane guard | AC 6 |
| M10 | the context on a restore that fails other than by a conflict | AC 2 |

The script is in the session scratchpad, not the repo. Afterwards `git status --short` was empty and the suites were
GREEN again.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Format (ruling 4) | `rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs crates/holler-cli/src/pane/profile_scope.rs` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. Its warnings are pre-existing files only; neither changed file is listed | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Workspace tests | `HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast` | 0 failed | 125 suites: 1403 passed, 0 failed, 5 ignored; exit 0 | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Test kit | `cargo test -p holler-pane-testkit` | pass | 202 passed, 0 failed | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Server / API smoke | n/a | n/a | No binary behaviour changes: no verb uses either item until #649 wires them | N/A |

**Cross-check of F's commands.** Every command F reported was re-run, and the results agree with two differences, both
benign:
1. **The workspace run.** I did not pass F's `--skip roster_stays_accurate_under_concurrent_body_load`, so my run has one
   more test (1403 against F's 1402). That test passed.
2. **`logging_test.rs`.** F reports four `logging_test.rs` cases failing on this host without `HOLLER_STATE_DIR`,
   because a hub was on the default socket. In my run, `env -u HOLLER_STATE_DIR cargo test -p holler-cli --test
   logging_test` gave 11 passed. So the failure depends on host state at the time, not on code. The file is not in this
   diff.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | Each test-backed AC (1-8) has its named test, and AC 9-14 are gates run below. | PASS |
| Test quality (§7) | Each test names one AC behaviour. Each failed in isolation for the right reason at RED and is killed by a targeted mutation (above). All sit at the unit tier. None repeats a conformance case: AC 6 and AC 7 pin points the suite leaves open, and AC 2-4 pin the message extensions that only the real scope has. The suite is proportionate: 19 tests for about 300 production lines. Nothing to delete or merge. | PASS |
| Type safety | No `unsafe` (AC 11 grep). No new `#[allow]` beyond the two test-module ones, each with `// #663`. `with_context` has no `_` arm, so a new `PaneError` variant fails to compile. | PASS |
| Error handling | Every error path is pinned: the restore failure (3 codes), the restore conflict, the first-write timeout and pass-through, usage, the pane-store fault, and the probe's spawn failure, empty argv, zero timeout, non-zero exit, timeout, cap and non-UTF-8 output. | PASS |
| Data integrity | The CAS order is pinned by the call log (`[Get, CasPut, CasPut]`, no retry). The no-write-before-guard rule is pinned by AC 6 and AC 7 (no `CasPut`, still at generation 1). The concurrent writer is pinned by AC 3 and conformance case 11. | PASS |
| API contract | `StoreScope::new`, `reconcile_step` and `RECONCILE_STEP_UNSCOPED` match Decisions 1 and 8. `ProbeResult` is unchanged. `probe.rs`'s only `pub` items are `ProbeResult` and `run_probe`. | PASS |
| Security | AC 9's three greps print nothing; `Command::new` count is 2 (`argv[0]` and `kill`); `#[cfg(test)]` appears once in each file, as the last item. 8h and 8j prove direct exec and that no reason echoes argv or output. The reconcile step is POSIX-quoted (AC 5 uses `$(id)` and `'`). The diff adds no personal host or account name (grep). | PASS |
| Migration safety | No schema, wire or golden change. | N/A |
| Size | `probe.rs` 573 lines, `profile_scope.rs` 597 lines: both under 600. The longest function is 39 lines (limit 100). | PASS (see advisory 1) |
| Evidence appendix | I checked F's 10 excerpts against the source at the cited lines; all are verbatim. I appended 3 facts the tests rely on that F did not list, each copied from source myself: `seeded` stores at generation 1 and bypasses the call log; `fail_next` is one-shot; `concurrent_put` writes at generation + 1 and bypasses the call log. | PASS |
| Playwright / UI | No UI surface in this repo. | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 Conformance | PASS | `store_scope_passes_the_profile_scope_conformance_suite` |
| 2 Restore failure keeps code, names the edit | PASS | `restore_failure_keeps_its_code_and_names_the_unrestored_edit` (`timeout`, `unavailable`, `store-corrupt`) |
| 3 Restore conflict carries the step | PASS | `restore_conflict_names_the_act_error_and_the_reconcile_step` |
| 4 First-write timeout | PASS | `first_write_timeout_says_the_edit_may_have_landed` |
| 5 Reconcile step quoting | PASS | `reconcile_step_single_quotes_the_profile_name` |
| 6 Spec filed under another pane | PASS | `set_of_a_spec_for_another_pane_is_usage_before_any_write` |
| 7 Pane-store fault before the write | PASS | `pane_store_fault_fails_a_remove_before_the_profile_write` |
| 8a-8l Probe runner | PASS | the 12 `probe::tests`, 0.53 s parallel and 1.39 s serial |
| 8m Stub regression guard | PASS | `ports_test::run_probe_stub_never_reports_success` |
| 9 No shell, no broad kill, two spawns | PASS | greps above, with 8h and 8j |
| 10 Quality gates | PASS | Tier 1 table |
| 11 No `unsafe`, no new dependency | PASS | `git diff 3bdd129...HEAD -- crates` shows no added `unsafe`; no `Cargo.toml` or `Cargo.lock` in the diff |
| 12 Blast radius | PASS | `git diff --name-only 3bdd129...HEAD`: `CHANGELOG.md`, the two `.rs` files, `docs/adr/ADR-0021.md`, and `docs/handoffs/663*` |
| 13 CHANGELOG | PASS | one entry under `## [Unreleased]` / `### Enhancements`, linking #663; it names no host |
| 14 ADR-0021 in place | PASS | 4 hunks, at section 1 `ProbeResult`, section 2's bound, the record fence bullet, and section 8 steps 2, 5 and 6. No row or heading lines changed, `(#663)` appears in every edit, and the exact step text, `RECONCILE_STEP_UNSCOPED` and `1 MiB` are present. |

AC 11 and AC 12 were measured against the merge base `3bdd129`, not `origin/main`, because `origin/main` has moved to
`e612878` (#701). Against `origin/main...HEAD` the three-dot form is the same thing.

## Blocking issues

None.

## Advisory notes

1. **`profile_scope.rs` is at 597 lines**, 3 under the 600-line warning (the failure is at 900). The next story to
   touch it (F1, F2 or #649) will need a trim first.
2. **A rebase is needed before merge.** `origin/main` moved to `e612878`, and F's merge dry run shows a `CHANGELOG.md`
   conflict: keep both entries. The PR step owns this.
3. **A restoring write that fails with a variant that has no payload** (for example `NotImplemented`) comes back
   without the context, because `with_context` leaves payload-less variants unchanged. No `ProfileStore` answers those
   from `cas_put` today, and the brief does not pin it. Worth a line in F2 if the helper is hoisted.
4. **The 8e-8g timing bounds and BSD `kill -s KILL -- -<pgid>` are first exercised by CI's macOS job.** On Linux they
   passed 25 of 25 runs.
5. **Follow-up for O (F's known issue 2).** #701 gave `pane doctor` a `[PANE]` positional, so naming the pane in the
   reconcile step is now possible. It is a follow-up beside F5, not this story's AC.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
