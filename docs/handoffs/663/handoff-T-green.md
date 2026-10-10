# Handoff-T-green: Phase 7 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (re-entry)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `4bbe607`, merge base `0ad2d8a`)
**Issue:** #663
**Handoff-F reviewed:** `docs/handoffs/663/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/663/handoff-T-red.md`

This replaces the previous run's T-green (at `93fb653`, in git history). That handoff passed the scope and the probe
runner. This run verifies only what the amended brief (`9762a97`) changed: Decision 8 (`reconcile_step` built on
`doctor_command`), AC 5's new assertions and greps, and AC 14's ADR wording (a, e and the new f).

## GREEN confirmation

No test was edited. Both test modules, from `#[cfg(test)]` to the end of the file, hash the same as at the RED commit
`f79cd05`:
- `profile_scope.rs`: sha256 `d21d0961...`;
- `probe.rs`: sha256 `3512b7ca...`.

F flagged no test as wrong ("Tests that look wrong (for T)": none), and I found none, so no test was repaired.

```
$ cargo test -p holler-cli --lib pane::profile_scope
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out     (RED: 3 passed, 4 failed)
$ cargo test -p holler-pane --lib probe::tests
test result: ok. 12 passed; 0 failed; ... finished in 0.53s
$ cargo test -p holler-pane --lib probe::tests -- --test-threads=1
test result: ok. 12 passed; 0 failed; ... finished in 1.39s           (AC 8's serial budget is 30 s)
$ cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success
test result: ok. 1 passed                                               (8m)
```

**Flake check.** `probe::tests` 10 more times: 10 of 10 passed. Afterwards there was no `hlr-probe-663-*` directory
under `$TMPDIR` and no `sleep 30` process.

**The tests pin behaviour, not implementation.** Four mutations of F's new `reconcile_step` body, one at a time, each
restored with `git checkout` (`git status --short` empty afterwards):

| Mutation | Behaviour removed | Killed by |
|---|---|---|
| M11 | the bare form (`None` gives `... doctor --all`) | AC 5 |
| M12 | the POSIX quoting (the name is used raw; `single_quoted` kept live behind `if true` so `dead_code = "deny"` does not turn it into a compile error) | AC 2, 3, 4, 5 |
| M13 | the `and then holler profile show '<P>'` half | AC 2, 3, 5 |
| M14 | the builder's arguments (`doctor_command(None, true)`, so `--fix` leaks in) | AC 2, 3, 4, 5 |

A mutation that swaps `doctor_command(None, false)` for the literal `"holler pane doctor"` keeps every test green, by
design: the output is the same. AC 5's source grep is what pins the reuse (below), as the brief says.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Format (ruling 4) | `rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs crates/holler-cli/src/pane/profile_scope.rs` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. One warn for this story: `warn: crates/holler-cli/src/pane/profile_scope.rs is 605 lines` (accepted by AC 10, journalled here) | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Workspace tests | `HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast` | 0 failed | exit 0; 131 suites: 1495 passed, 0 failed, 5 ignored | PASS |
| Lib tests | `cargo test -p holler-cli --lib` | pass | 16 passed | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Test kit | `cargo test -p holler-pane-testkit` | pass | 202 passed, 0 failed | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Rustdoc | `cargo doc -p holler-cli -p holler-pane --no-deps` | no warning in a changed file | 7 warnings, all in `crates/holler-cli/src/cli.rs` (pre-existing, not in the diff) | PASS |
| Server / API smoke | n/a | n/a | No binary behaviour changes: no verb calls either item until #649 wires them | N/A |

**Cross-check of F's commands.** Every command F reported was re-run, and the results agree. One difference, benign: I
did not pass F's `--skip roster_stays_accurate_under_concurrent_body_load`, so my workspace run has one more test (1495
against F's 1494). That test passed.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | Each test-backed AC (1-8) has its named test; AC 5's source greps and AC 9-14 are gates run below. | PASS |
| Test quality (§7) | The one amended test (`reconcile_step_single_quotes_the_profile_name`) names AC 5's behaviour, failed at RED on its first equality (`left: ""`), and is killed by M11-M14. Its `doctor_command` equality ties the bare form to #701's builder, so a drift in `findings.rs` fails here, not in a verb. No test duplicates another; nothing to delete or merge. | PASS |
| Type safety | No `unsafe` (AC 11). No new `#[allow]`. `reconcile_step` uses `let ... else`, no unwrap. | PASS |
| Error handling | Unchanged since the previous T-green; AC 2-4 now also pin the step text inside each error that carries it. | PASS |
| Data integrity | Unchanged since the previous T-green (call logs, no write before the guards). | PASS |
| API contract | `reconcile_step(Option<&ProfileName>) -> String` matches Decision 8; `RECONCILE_STEP_UNSCOPED` is gone (AC 5c: 0). `ProbeResult` and `run_probe` unchanged; `probe.rs` changed only in rustdoc. | PASS |
| Security | AC 9's greps print nothing; `Command::new` count 2; `#[cfg(test)]` once per file. The step stays POSIX-quoted (AC 5 uses `$(id)` and `'`, killed by M12). | PASS |
| AC 5 source greps | production lines of `profile_scope.rs`, comments excluded: `holler pane doctor` 0; `doctor_command(None, false)` 1; `RECONCILE_STEP_UNSCOPED` in the file 0 | PASS |
| Size | `profile_scope.rs` 605 lines (warn at 600 accepted by AC 10; gate is 900); `probe.rs` 577 (under 600). | PASS (advisory 1) |
| Evidence appendix | I checked F's 7 new excerpts against the source: `findings.rs:36, 306-316`, `lib.rs:42`, `reconcile.rs:226-235` on the branch, and `exec.rs:17, 32-34`, `server.rs:106, 169-172` at `dc300ab`. All verbatim. The amended test relies on nothing else outside the diff, so I added no entry. | PASS |
| Migration safety | No schema, wire or golden change. | N/A |
| Playwright / UI | No UI surface in this repo. | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 Conformance | PASS | `store_scope_passes_the_profile_scope_conformance_suite` |
| 2 Restore failure keeps code, names the edit | PASS | `restore_failure_keeps_its_code_and_names_the_unrestored_edit` |
| 3 Restore conflict carries the step | PASS | `restore_conflict_names_the_act_error_and_the_reconcile_step` |
| 4 First-write timeout | PASS | `first_write_timeout_says_the_edit_may_have_landed` |
| 5 Reconcile step quoting, built on `doctor_command` | PASS | `reconcile_step_single_quotes_the_profile_name`, plus the three source greps (0 / 1 / 0) |
| 6 Spec filed under another pane | PASS | `set_of_a_spec_for_another_pane_is_usage_before_any_write` |
| 7 Pane-store fault before the write | PASS | `pane_store_fault_fails_a_remove_before_the_profile_write` |
| 8a-8l Probe runner | PASS | the 12 `probe::tests`, 0.53 s parallel and 1.39 s serial |
| 8m Stub regression guard | PASS | `ports_test::run_probe_stub_never_reports_success` |
| 9 No shell, no broad kill, two spawns | PASS | greps above, with 8h and 8j |
| 10 Quality gates | PASS | Tier 1 table |
| 11 No `unsafe`, no new dependency | PASS | `git diff 0ad2d8a...HEAD -- crates`: no added `unsafe`; no `Cargo.toml` or `Cargo.lock` |
| 12 Blast radius | PASS | `git diff --name-only 0ad2d8a...HEAD`: `CHANGELOG.md`, the two `.rs` files, `docs/adr/ADR-0021.md`, `docs/handoffs/663*` only |
| 13 CHANGELOG | PASS | unchanged from the previous run; `changelog-check: ok` |
| 14 ADR-0021 in place | PASS | 5 hunks (section 1, section 2, the fence bullet, section 8 steps 2-6, section 12); no `^[-+]\|` and no `^[-+]#` line; `(#663)` in every hunk; the added text holds the exact step text, `doctor_command`, `try_wait` and `1 MiB`; `grep -cE 'RECONCILE_STEP_UNSCOPED\|takes one \(#647\)\|for now the'` gives 0; section 12's first paragraph holds `No verb leaves work running after it exits`, `section 1` and `(#663)`; no added line over 124 columns |

AC 11, 12 and 14 were measured against the merge base `0ad2d8a`, which is what `origin/main...HEAD` resolves to;
`origin/main` itself is now at `e327569`.

## Blocking issues

None.

## Advisory notes

1. **`profile_scope.rs` is at 605 lines,** past the 600-line warning (295 lines of room under the 900 gate). AC 10
   accepts it. The next story to touch the file (F1, F2 or #649) should trim first.
2. **A merge is needed before the PR merges.** `git merge-tree --write-tree HEAD origin/main` conflicts in `CHANGELOG.md`
   only (keep both entries). Neither `dc300ab` nor `e327569` touches the two `.rs` files or the ADR.
3. **`probe.rs` gained one rustdoc bullet** (F's deviation 1, 4 lines, doc only). The test module is unchanged and the
   file stays under 600 lines.
4. **W-17 (a profile name with a leading `-`) is documented, not fixed;** the printed step would not parse. That is O's
   follow-up in `ProfileName::parse`. No test pins it, since the brief makes it out of scope.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
