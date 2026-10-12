# Handoff-T-green (test-only rework, diff gate round 3) - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (23:27 MDT)
**Branch:** `issue-665-implementation`, on `bac1513` plus this phase's uncommitted test changes
**Issue:** #665
**Handoff-F reviewed:** `docs/handoffs/665/handoff-F.md` (no new F round; this is a test-only re-entry)
**Handoff-T-red:** `docs/handoffs/665/handoff-T-red.md`
**Findings answered:** `docs/handoffs/665-diff-result-r3.md`, B-1 and B-2

The earlier rounds' handoffs follow this section, unchanged.

## Test repair (B-1)

**What B-1 claimed.** It claimed `env` and `expect` may be left out of an exported pane when a spec has none, and that no test proves otherwise.

**What the code does.** `ProfileSpec` gives both members `#[serde(default)]` and no `skip_serializing_if` (`crates/holler-pane/src/profile.rs:192-193`, `:202-203`). Only the three `Option` members skip. So every exported pane writes `env` and `expect`, as `[]` when empty, and the production code meets D7.
- `export_leaves_out_absent_optional_members` already pinned this byte for byte, through `BARE_DOCUMENT`.
- The gate could not see `ProfileSpec`'s attributes, because those lines are unchanged.

**The real gap.** A spec with "absent" (not empty) `env` or `expect` can exist only in a file, and no test fed one.

**The changes, all test-side:**
1. **New test** `a_spec_without_env_or_expect_imports_and_exports_them_as_empty_arrays`, in `crates/holler-cli/tests/profile_verbs/import.rs`. It takes `BARE_DOCUMENT`, removes `env` and `expect` from the pane, and imports it in both formats. Then:
   - the stored spec's `env` and `expect` are empty;
   - `profile export Bare` on the same rig writes `"env": []` and `"expect": []`;
   - the whole document equals `BARE_DOCUMENT` byte for byte.

   This pins the full serialized shape for an absent-env spec, which is the gate's own remedy. It runs at the in-process verb tier, the cheapest tier that has both the import and the export path.
2. `BARE_DOCUMENT` (`crates/holler-cli/tests/profile_verbs/export.rs`) is now `pub(crate)`, with a doc line saying `import.rs` uses it. The existing round-trip test now uses the imported `Format` in place of a full path.
3. **New evidence entry** in `docs/handoffs/665/evidence.md`, under "The verbs (CLI)". It quotes `ProfileSpec`'s `#[serde(deny_unknown_fields)]` (line 179) and its `env` and `expect` attributes, verbatim from source. This also settles B-2's "`deny_unknown_fields` ... not shown".

**B-2** is marked "Not a block" by the gate, and it asks for no test change. Its one open point is now in the appendix, and so is the `error.rs:632-634` excerpt. The "never quoted" behaviour is already pinned by a mutation-checked canary test, `a_document_of_another_shape_is_usage_and_never_quoted` (rework 1, spot check 2).

No production file was changed in this phase.

## GREEN confirmation

- `cargo test -p holler-cli --test profile_verbs import`: 15 passed, including the new test.
- **The workspace.** `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, 23:19 to 23:25 MDT: **154 targets, 2,436 passed, 0 failed, 32 ignored**, exit 0. That is 2,435 plus the new test. The story's targets: `profile_verbs` 114, `docs_cli_test` 3, `wire_selftest` 3.
- **Spot check.** `skip_serializing_if = "Vec::is_empty"` was added to `env` and `expect` in `profile.rs`, from a saved copy. Two tests then fail:
  - the new test, with `left: (Null, Null)` against `right: (Array [], Array [])`;
  - `export_leaves_out_absent_optional_members`, where the document lacks both members.

  The file was restored from the copy, and `git status` shows no change under `crates/*/src`.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| Workspace tests | as above | all pass | 2,436 passed, 0 failed, 32 ignored | PASS |
| Docs CLI / canary | `docs_cli_test`, `wire_selftest`, in the workspace run | pass | 3 and 3 passed | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Format | `rustfmt --edition 2021 --check` on both touched files | clean | clean | PASS |

**Cross-check with F.** There is no new F round to cross-check. The counts match T-green rework 2, plus one test.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| D7 "always present" pinned | The new test (absent in the file) and `BARE_DOCUMENT` (empty in memory). Both fail under the skip mutation. | PASS |
| Test quality | One test, one behaviour. It does not duplicate `export_leaves_out_absent_optional_members`, which starts from an in-memory spec, not a file without the members. It reuses `BARE_DOCUMENT`, with no second copy of the shape. | PASS |
| Evidence appendix | 14 entries, every excerpt line matches its cited source line, and the file is 11,799 bytes, under the 12,000-byte cap. The new entry sits before the drift section, so it is well inside the cap. | PASS |
| File size | `import.rs` 231 lines, `export.rs` 364 | PASS |
| Security, error handling, data integrity | Unchanged from rework 1 | PASS |
| Browser / Playwright | N/A (no UI surface) | N/A |

## Acceptance criteria status

Unchanged from rework 1. **CLI `profile export`** is now also backed by the new test, for D7's "`env` and `expect` always present".

## Blocking issues

None. Nothing F must change: B-1's premise does not hold for the code as written, and it is now proven in the tests and the appendix.

## Advisory notes

- The earlier advisories still apply: the rebase onto `origin/main` needs a fresh workspace run, the `unavailable` file-error half is unpinned, and #693 can flake CI.

---

# Handoff-T-green (rework 2): after F rework 2, the outside diff gate's round 2 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (23:17 MDT)
**Branch:** `issue-665-implementation`, on `2ed6ac0` (F rework 2)
**Issue:** #665
**Handoff-F reviewed:** `docs/handoffs/665/handoff-F.md`, section "Handoff-F (rework 2)"
**Handoff-T-red:** `docs/handoffs/665/handoff-T-red.md`

The earlier rounds' handoffs follow this section, unchanged.

## What changed since the last T-green

- **No compiled file.** `git diff 5dbdbd9 2ed6ac0` touches only `docs/handoffs/665/`: `evidence.md` (rewritten), `handoff-F.md` and `decisions.md`. No `.rs` file, manifest, guide, ADR or CHANGELOG changed.
- **No test repair.** F lists no "Tests that look wrong (for T)", and no finding in `docs/handoffs/665-diff-result-r2.md` asks for a test change: NIT-2 (`process/profile_io.rs`) asks for none, and NV-4 is answered by the new `LiveHub` evidence entry. T changed no file but this handoff and `decisions.md`.

## GREEN confirmation

The full workspace suite was re-run from scratch on `2ed6ac0`, 23:09 to 23:15 MDT, with the configured command: `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`.
- **154 targets, 2,435 passed, 0 failed, 32 ignored, exit 0.** This is the same as T-green rework 1 on `5dbdbd9`, as it must be with no compiled change.
- **The story's targets, from that run, match F's counts exactly:**

| Target | Passed | Failed | Ignored |
|---|---|---|---|
| holler-hub `profile_rename_test` | 10 | 0 | 0 |
| holler-hub `profile_rename_faults_test` | 8 | 0 | 0 |
| holler-hub `profile_registry_test` (conformance on `ProfileState`) | 12 | 0 | 0 |
| holler-pane-testkit `profile_store_conformance_test` (on the fake) | 12 | 0 | 0 |
| holler-pane `findings_test` | 2 | 0 | 0 |
| holler-cli `profile_verbs` | 113 | 0 | 0 |
| holler-cli `pane_verbs` (includes the 7 drift tests) | 340 | 0 | 0 |
| holler-cli `pane_cli_process` (includes the 2 `profile_io` tests) | 38 | 0 | 0 |
| holler-cli `pane_integration` (includes the wire conformance runner) | 66 | 0 | 4 |
| holler-cli `docs_pane_control_test`, `docs_cli_test`, `cli_surface_test` | 18, 3, 3 | 0 | 0 |
| holler-cli `wire_selftest` (canary) | 3 | 0 | 0 |

**Spot checks.** Not repeated: no production or test code changed, so the mutation spot checks of the earlier rounds still hold. They are spot checks 1 to 3 of rework 1 and the drift check of round 1's re-verification.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. Size warnings only, none on a file this story touches, none at 900 lines | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| Workspace tests | as above | all pass | 2,435 passed, 0 failed, 32 ignored | PASS |
| Docs CLI | `docs_cli_test`, in the workspace run | pass | 3 passed | PASS |
| Canary | `wire_selftest`, in the workspace run | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | none | PASS |

**Cross-check with F.** F re-ran the gates and the story's targets, not the workspace. Every count F reports (10, 8, 12, 12, 2, 113, 7, 2, 18 + 3 + 3, 66) agrees with this run. F assumed the 2,435 of `5dbdbd9` still stood. This run confirms it. No discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Evidence appendix | T's own script compared every excerpt line of the rewritten `evidence.md` with the lines its `Source:` cites. The result was 13 entries, 0 mismatches. The file is 11,118 bytes, under the gate's 12,000-byte cap. | PASS |
| T's entries kept | The "T rework" section matches `5dbdbd9`'s text word for word: `now_millis`, `stamp` and the fake's clock. | PASS |
| Dropped facts still sourced | F dropped the entries that the brief quotes: E7 and E8 (`entry.rs`), E13 (`pane_dispatch.rs`), E20 (`reply.rs`), E23 (`argv.rs`), E26 (`create.rs`) and E32 (`profile_diff.rs`). `git diff bda1069 HEAD` leaves those quoted ranges unchanged. `create.rs` changes only `taken`'s visibility and doc, and `reply.rs` only a doc comment at line 192, both outside the quotes. | PASS |
| Coverage, test quality, type safety, error handling, security, data integrity | Unchanged from rework 1, since no code changed | PASS |
| Browser / Playwright | N/A (no UI surface) | N/A |

## Acceptance criteria status

Unchanged from rework 1 (below): every criterion PASS, each backed by the test named there.

## Blocking issues

None.

## Advisory notes

- **F's three gate-pipeline issues (the verdict parse, the excerpt budget, no handoff at the diff gate) are for the main session.** T confirms the round 2 verdict line: it reads "BLOCK — 0 blocking findings", and every B item concludes "no block".
- **S advisory 1:** after the rebase onto `origin/main`, re-run the workspace suite. `origin/main` has moved a long way (roster panes, build version), so expect conflicts in `cli.rs`, `main.rs` and the test manifests.
- Earlier advisories still apply: the `unavailable` file-error half is unpinned, and #693 can flake CI. #693 did not recur in this run.

---

# Handoff-T-green (rework 1): after S REWORK and F rework 1 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)

**Date:** 2026-10-11 (22:55 MDT)
**Branch:** `issue-665-implementation`, on `ccbcbc5` (F rework 1)
**Issue:** #665
**Handoff-F reviewed:** `docs/handoffs/665/handoff-F.md`, section "Handoff-F (rework 1)"
**Handoff-S:** `docs/handoffs/665/handoff-S.md` (REWORK: items 1 and 2 production, items 3 and 4 test-only)
**Handoff-T-red:** `docs/handoffs/665/handoff-T-red.md`

The first round's handoff follows this section, unchanged.

## Test repair (T's items)

F changed no test. T made these changes; no production file was changed in this phase.

1. **S item 1 (three pinned strings).** `1 specs` is now `1 spec`, matching F's `count(n, "spec")`:
   - `crates/holler-cli/tests/profile_verbs/import.rs`, in two tests: `import_creates_a_detached_profile_at_generation_one_in_both_formats` and `import_name_overrides_the_files_name_and_is_parsed`;
   - `crates/holler-cli/tests/profile_verbs/export.rs`: `export_out_writes_the_file_at_0600_and_replaces_one_that_exists`.
2. **S item 2(b) pinned** in `crates/holler-cli/tests/profile_verbs/import/refusals.rs`:
   - `a_bad_position_is_its_grid_code` gains three cases, each refused with its code at exit 3, writing nothing, with `CANARY` never printed: a text position `r1<CANARY>` (`grid-ambiguous`), an object form whose `pos` label is `CANARY` (`grid-ambiguous`), and an object zero cell (`grid-out-of-range`).
   - New test `a_refused_position_is_named_by_its_spec_number_not_quoted`: a two-spec document with the canary position in spec 2. It is `grid-ambiguous`, never prints the canary, and both formats' message contains `herdr.grid of spec 2` (the shared `assert_message_contains` helper).
3. **S item 3 (spec-level `usage` redaction).** `a_document_of_another_shape_is_usage_and_never_quoted` gains two members of the wrong type holding `CANARY`, which serde's own message would quote: `context.soft = CANARY` (`invalid type: string ..., expected u32`) and `role = CANARY` (`unknown variant ...`). Each is `usage`, exit 2, never printed.
4. **S item 4 (`updated` is the rename's stamp).**
   - H1 (`crates/holler-hub/tests/profile_rename_test.rs`, `the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record`) now asserts `stored.updated == the at of "Day Shift"'s one log entry`. Before the rename it waits, on the observed condition `now_millis() > old.updated` (the hub's own clock function, reused from `holler_proto::clock`), so the check is deterministic: a rename that kept the old `updated` can never equal its new stamp. This is a condition wait, not a fixed sleep, and lasts at most about one millisecond.
   - Conformance case 23 (`crates/holler-pane-testkit/src/conformance/profile_store/rename.rs`) replaces its `>=` check with equality: `renamed.updated` equals the `at` of the new name's one log entry. The case's doc comment says so.
5. **Optional item 5, taken:** `refusals.rs`'s module doc now states the narrower rule, "never quotes what a spec holds (a bad position or member is named by the spec's number)". S advisory 3 (the process test's name) is not taken; it is a naming nit and coverage holds.

All five touched files pass `rustfmt --edition 2021 --check`. Sizes: `refusals.rs` 183 lines, `profile_rename_test.rs` 478, `rename.rs` 292.

## GREEN confirmation

| Target | Command | Result |
|---|---|---|
| CLI verbs | `cargo test -p holler-cli --test profile_verbs` | **113 passed**, 0 failed (F's 109 + the 3 repaired pins + 1 new test) |
| Hub rename | `cargo test -p holler-hub --test profile_rename_test` | 10 passed |
| Conformance on the fake | `cargo test -p holler-pane-testkit --test profile_store_conformance_test` | 12 passed |
| Conformance on `ProfileState` | `cargo test -p holler-hub --test profile_registry_test` | 12 passed |
| Conformance on the wire | `pane_integration` :: `clients::the_profile_client_passes_the_profile_store_conformance_suite`, in the workspace run | ok |

**Spot checks: each new assertion fails when the behaviour is removed.** Each mutation was applied to a copy-saved source file, run, and restored from the copy; `git diff -- crates/*/src` shows only T's conformance case edit afterwards.
1. **No grid redaction** (`spec_refused`'s two grid arms disabled with `if false`): `a_bad_position_is_its_grid_code` and `a_refused_position_is_named_by_its_spec_number_not_quoted` fail with `the file's content was printed: error: ambiguous grid position: "r1t665-import-canary-..."`.
2. **No `usage` redaction** (`spec_refused`'s `Usage` arm disabled): `a_document_of_another_shape_is_usage_and_never_quoted` fails with `invalid type: string "t665-import-canary-...", expected u32`. Before this round, removing the arm left every test green (S item 3).
3. **A rename that keeps the old `updated`** (both the hub's `renamed` and the fake's `move_record` set `updated: stored.updated`): H1 fails, 3 runs of 3, with "`updated` is the rename's one stamp". Case 23 still passes on the fake (its clock is fixed at 0, so a kept `updated` equals the stamp) and on `ProfileState` (create and rename in the same millisecond). Case 23 pins the invariant on every runner; H1 is the deterministic control. `evidence.md` records why.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (size warnings on other crates only) | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (finished 22:49 MDT) | all pass | 154 targets, **2,435 passed, 0 failed, 32 ignored**, exit 0 | PASS |
| Docs CLI | `docs_cli_test`, in the workspace run | pass | pass | PASS |
| Canary | `wire_selftest`, in the workspace run | pass | pass | PASS |
| Unused deps | `cargo machete` | none | none | PASS |

**Cross-check with F.** F reported `profile_verbs` 109 passed and 3 failed (the three `1 specs` pins), and the workspace 2,431 passed, 3 failed. Here the same three pass after the repair and one test is added: 2,431 + 3 + 1 = 2,435. No discrepancy. #693 did not recur.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of S's REWORK | Items 1 to 4 each have a test that fails without the behaviour (spot checks 1 to 3; item 1 by the three exact strings) | PASS |
| Test quality | The new cases sit in the existing refusal tests at the in-process verb tier, the cheapest with the real import path. One new test, because spec numbering needs a two-spec document and a message check that `refused` does not make. H1's wait is a condition wait on the hub's own clock function, not a sleep. No duplicate. | PASS |
| Type safety | Clippy clean; no new `#[allow]` in this round | PASS |
| Error handling | Every redaction path of `spec_refused` (two grid codes, `usage`) is now pinned with a canary | PASS |
| Security | The canary is asserted absent from text out/err and JSON out/err in every new case | PASS |
| Data integrity / migration | Unchanged from round 1 | PASS |
| File size | Largest touched test file 478 lines | PASS |
| Evidence appendix | Three entries added under "T rework" in `evidence.md` (the hub's clock, the one-stamp rule, the fake's fixed clock), each excerpt diffed against source: 0 mismatches | PASS |
| Browser / Playwright | N/A (no UI surface) | N/A |

## Acceptance criteria status

Unchanged from round 1 (below), with S's two PARTIAL rows now met:
- **1. The record:** PASS. H1 now pins `updated` as the stamp (spot check 3); case 23 pins the same invariant on all three runners.
- **CLI `profile import`:** PASS. The refused value is never printed, now pinned for the grid codes (text and `pos` label) and for spec-level `usage` (spot checks 1 and 2), and the count is `1 spec`.
- **CLI `profile export`:** PASS. The count is `1 spec`.

## Blocking issues

None.

## Advisory notes

- Round 1's advisories still apply: the `unavailable` file-error half is unpinned (root in CI), and #693 can flake CI.
- S advisory 1 (the conflict with `origin/main`) is for the rebase before merge; re-run the workspace suite after it.
- S advisory 3 (`rename_on_an_empty_hub_reaches_the_real_handler` overstates its name) is left; coverage holds through the round-trip test.

---

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
