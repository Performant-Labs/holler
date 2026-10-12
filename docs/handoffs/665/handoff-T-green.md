# Handoff-T-green: Phase 6 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (21:40 MDT)
**Branch:** `issue-665-implementation`, on `d391f1d` (F)
**Issue:** #665
**Handoff-F reviewed:** `docs/handoffs/665/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/665/handoff-T-red.md`

## Re-verification (Phase 7 pass, 2026-10-11, 21:48 to 21:58 MDT, on `3b20fc0`)

The script ran this phase again on `3b20fc0` (this handoff's first round). Since then:
- no production or test code has changed;
- the tree was clean at the start;
- no "TEST-ONLY REWORK" note came with the call.

Every check was re-run independently:
- **Workspace.** `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 154 targets, **2,434 passed, 0 failed, 32 ignored**, exit 0. This includes:
  - `pane_integration`, the wire conformance runner, with repaired cases 26 and 27;
  - `profile_rename_test` and `profile_rename_faults_test`;
  - `docs_cli_test` and the `wire_selftest` canary.

  #693 did not recur in this run.
- **Gates.** All clean:
  - `bash scripts/lint.sh` exits 0, with size warnings only and no `.rs` file at 900 lines;
  - `bash scripts/changelog-check.sh` prints `ok`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo machete` finds no unused dependency.
- **A new spot check: the drift finding.** In `crates/holler-pane/src/reconcile/drift.rs`, `drifted` was changed to never match a `Differs` row (`false && ...`). `cargo test -p holler-cli --test pane_verbs drift` then failed 4 of 7 tests:
  - `drift_agrees_with_profile_show_on_one_fixture`;
  - `drift_is_one_finding_per_pane_per_pass_for_as_long_as_it_stands`;
  - `drift_follows_the_scope_of_the_pass`;
  - `a_drift_finding_has_its_shape_and_never_quotes_a_value`.

  The 3 tests that still passed check the number of profile calls and the failed-read case, not the finding, so they should be unaffected. The change was reverted, and `git status` is clean.

  (My first try at this mutation did not apply, because the pattern did not match. I saw that before relying on the result, and redid it.)
- **The verdict is unchanged: PASS.** There is no blocking issue. The advisory notes below still apply.

## Test repair: the two cases F flagged

F flagged two conformance cases under "Tests that look wrong (for T)": case 26 `rename-of-one-slug-is-one-put` and case 27 `rename-feed-is-deletion-then-put`. They failed over the wire only (`pane_integration` :: `clients::the_profile_client_passes_the_profile_store_conformance_suite`), with `next() after ...: expected the change of "demo-alpha" ..., got None`.

- **The diagnosis is right.**
  - `expect_change` reads exactly one `next()`.
  - `HubProfileStore::watch` makes its first long-poll inside `watch()` and keeps that reply for the first `next()` (#649 D4; `evidence.md`, the last entry). So a watch opened at the head first yields the idle `Ok(None)` of a window that ended before the rename.
  - The suite's own case 19 reads that idle item before writing, and case 21 does too. My cases 26 and 27 did not, so the defect is in the tests.
- **The fix** is F's option (a), as in case 19, and is in `crates/holler-pane-testkit/src/conformance/profile_store/rename.rs`.
  - Cases 26 and 27 now read the first `next()` after `open(...)` and expect `None`, then rename.
  - The doc comment of each case says why.
  - Nothing else in the cases changed. The "and nothing else" check (a final `next()` of `None`) still holds.
  - The file was formatted with `rustfmt --edition 2021` on the file alone.
- **Production code is unchanged** by this phase.

## GREEN confirmation

**The three runners of the conformance suite pass after the repair:**

| Runner | Command | Result |
|---|---|---|
| The fake, plus the mutants | `cargo test -p holler-pane-testkit --test profile_store_conformance_test` | 12 passed |
| `ProfileState` | `cargo test -p holler-hub --test profile_registry_test` | 12 passed |
| The wire (`HubProfileStore`) | `cargo test -p holler-cli --test pane_integration` | 66 passed, 0 failed, 4 ignored |

**The workspace run.** The command is `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, finished about 21:31 MDT. Across 154 targets: **2,433 passed, 1 failed, 32 ignored.**
- Every test authored or edited in Phase 4 passes. That is all 70 RED failures in 14 targets: the hub, conformance, CLI verbs, doctor drift, findings, docs, surface and process tests.
- **The one failure is the known flake #693.** It is `remote_admin_test` :: `remote_say_wait_and_hub_query_json_match_local_after_masking`, and it failed with `"updates": 1` vs `2`.
  - #693 is open, titled "Flaky ... (updates 2 vs 1)".
  - The run shared the machine with T's mutation builds below.
  - In isolation (`cargo test -p holler-cli --test remote_admin_test`, 5 runs) it passed 4 of 5, and the failed run was the same `updates` flake.
  - The diff touches no `say`, roster, body or circuit code.
  - F's run hit a different `remote_admin_test` timing test (`last_seen`), which passed here.

**Spot checks: the tests still fail when the behaviour is removed.** Each mutation was built in a separate target dir, so the running suite was not touched, and each was reverted. `git diff` shows only the test repair.
1. **The fake publishes no deletion event on a rename.** The repaired case 27 fails: `next() after the rename (the old name): expected the change of "demo-alpha" to None, got Some(ProfileEvent { ... "Demo Beta" ... })`. Case 23 fails too. So reading the idle item first did not stop the case from catching a wrong feed.
2. **The hub's `Locked::moves` moves no member** (`if true || ...`). `profile_rename_test` fails 4 of 10: H2 `every_member_follows_and_nothing_else_of_a_pane_changes`, H3 `membership_is_by_slug`, and H5's two feed tests. This is the brief's control for acceptance 2: a rename of the profile alone fails.

**Concurrency repeated.** `cargo test -p holler-hub --test profile_rename_faults_test` passed 5 runs of 5. That is 100 rounds of each 20-round race test (H14) with no flake and no timeout.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all pass | 2,433 passed, 1 failed (#693, the known flake; see above) | PASS (the flake is not this story's) |
| Docs CLI | `docs_cli_test`, in the workspace run | pass | pass (3) | PASS |
| Canary | `wire_selftest`, in the workspace run | pass | pass | PASS |
| Unused deps | `cargo machete` | none | none | PASS |

**Cross-check with F.** F's targeted counts match where the targets are the same:
- `pane_integration` was 65 passed and 1 failed for F, and is 66 passed here once the test is repaired.
- F's workspace run had 2 failures: the same wire case, and a `remote_admin_test` timing test.
- This run has 1 failure: a different `remote_admin_test` timing test (#693).
- There is no other discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each acceptance criterion | The table below, against the brief's "Acceptance" | PASS |
| Test quality | Each test names a behaviour. The two repaired cases now fail only for a wrong feed (spot check 1), not for an idle read. Tiers: the hub transaction is at hub integration (the cheapest tier with both registries), the record half is in the conformance suite, the verbs run in-process over the fakes, and one process test covers the real binary. No duplicate was found: the process test covers only the wire path the in-process tests cannot. | PASS |
| Type safety | Clippy is clean. No `unsafe` and no new `#[allow]` in the diff. | PASS |
| Error handling | Every refusal code of D2, D8 and D9 is pinned in both formats, and each writes nothing. Faults are covered: store-corrupt, a failed pane save, a failed profile save with the restore. The I/O `unavailable` split (F's deviation 1, A warn 5) is not pinned by a test; see the advisory notes. | PASS |
| Data integrity | Covered: the restore is a real re-save (H11 checks the inode), the retry completes an interrupted rename (H13), a restart (H12), races (H14, repeated), and member writes during a rename are moved or refused, never lost. | PASS |
| API contract | The reply shapes are pinned: rename `{profile, from}`, export `{name, slug, specs, out, document}`, import `{profile}`. D7's document is pinned byte for byte, and the wire code of each refusal is pinned. | PASS |
| Security | Export never carries an environment value (the canary test). Import never echoes the file (the canary secret is absent from stdout, stderr and the envelope). The read is bounded (`take(1 MiB + 1)`). `--out` is written at 0600. A control character in a path is escaped. | PASS |
| Migration safety | No schema change. The tombstone passes the existing load check (H12). | PASS |
| File size | `lint.sh`'s 900-line check passes. | PASS |
| Browser / Playwright | N/A (no UI surface) | N/A |

**Evidence appendix.** The repaired cases rely on one fact from unchanged code: the wire client's eager first long-poll. `evidence.md` already lists it, with its verbatim excerpt (F's last entry). No entry is missing.

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| 1. The record | PASS | H1 |
| 2. Members follow, attached (control) | PASS | H2, plus spot check 2 |
| 3. Membership by slug | PASS | H3 |
| 4. Log intact, one new entry | PASS | H4 (2 tests), conformance cases 24 and 25 |
| 5. Feeds | PASS | H5 (2 tests), conformance cases 26 and 27 (repaired) |
| 6. Refusals change nothing, in order | PASS | H6, conformance case 28 |
| 7. Fail closed | PASS | H9 |
| 8. Failed saves (control) | PASS | H10, H11 |
| 9. Restart | PASS | H12 |
| 10. Retry completes | PASS | H13 |
| 11. Concurrency | PASS | H14 (x5 runs) |
| 12. Membership hook after a rename | PASS | H7 |
| 13. `not-implemented` pins replaced | PASS | H8, H15 |
| Conformance and fake (with mutants) | PASS | C1, C2, on all three runners |
| CLI rename | PASS | R1 |
| CLI export | PASS | E1 |
| CLI import | PASS | I1 |
| 14 to 19. Drift: agreement, once per pass, read-only, scope, failed read, shape | PASS | D1 (`doctor/drift.rs`, 7 tests) |
| 20. Registry of kinds | PASS | F1 |
| 21. Existing doctor tests unchanged | PASS | D2, plus the whole `pane_verbs` target |
| Process level | PASS | X3 |
| Docs and surface | PASS | X1, X2, `docs_cli_test` |

## Blocking issues

None.

## Advisory notes

- **The `unavailable` half of the file-error split is not pinned by a test** (F's deviation 1, A warn 5). An example is a permission-denied path for import, or an unwritable `--out`. Only the `usage` half is pinned. I left it unpinned:
  - The split is documented in ADR-0021 section 9 and the guide.
  - A permission test is unreliable when the suite runs as root (CI containers).
- **#693** (`remote_say_wait_and_hub_query_json_match_local_after_masking`) is still flaky and can fail this PR's CI on an unrelated test. A rerun of the job is the remedy, per #762.
- **The README's "Debug output" table** does not list `profile_rename_restore_failed`, like the other registry fault events. This is F's known issue, and it is a docs follow-up if wanted.
