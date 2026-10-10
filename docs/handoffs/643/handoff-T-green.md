# Handoff-T-green: Phase 7 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (amendment 1)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 901007d, F's amendment-1 commit)
**Issue:** #643
**Handoff-F reviewed:** docs/handoffs/643/handoff-F.md (amendment 1)
**Handoff-T-red:** docs/handoffs/643/handoff-T-red.md (amendment 1)

Round 2 of this handoff is in git: `git show 279a1fb:docs/handoffs/643/handoff-T-green.md`. This file replaces it.
Round 1 (`50bcc83`) and round 2 still hold for everything amendment 1 did not touch: the AC table for AC 1 to 18, the
Tier 2 analysis of escaping and errors, and the mutations M1 to M11, Ma to Mc. The production diff since T-red
(`git diff 350858d..901007d -- crates CHANGELOG.md`) is `list.rs`, `get.rs`, `watch.rs` and `CHANGELOG.md` only.

## GREEN confirmation

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` | 36 passed, 0 failed (T-red: 31 passed, 5 failed) |
| the same, 3 more runs after the mutations were restored | 36, 36, 36 passed |
| `cargo test -p holler-cli --test pane_verbs` | 127 passed, 0 failed |

All five of T-red's failing tests now pass:
- `list_flags_a_pane_whose_shown_differs_from_its_session_of_record`
- `get_flags_a_mismatch`
- `watch_flags_a_mismatch`
- `list_help_documents_the_columns_and_the_json_shape`
- `watch_help_documents_the_stream`

The W-13 guard `a_negative_observed_at_is_never_observed` still passes.

F listed no tests under "Tests that look wrong (for T)", and T found none. No test was changed this phase.

**Spot-check (behaviour removed, test fails).** T applied each mutation to the current tree with `sed`, ran the
read-verb filter, and restored the file with `git checkout`. `git status --short` was empty after each one.

| Mutation | What it removes | Result |
|---|---|---|
| Mc1: guard `at == 0` | W-13 (a negative `at` is never observed) | 1 failed: `a_negative_observed_at_is_never_observed` |
| Mc2: drop `\|\| record.is_none()` | "no session of record" reads `-` (c5) | 3 failed: the three AC 3 tests |
| Mc3: `shown.is_some() && shown_differs(..)` | the home screen is a MISMATCH (c3) | 3 failed: the three AC 3 tests |
| Mc4: drop the `at` guard | "never observed" reads `-` (c4) | 4 failed: the three AC 3 tests and the W-13 test |
| Mc5: `watch --profile` admits every record (no `is_member`) | W-12 membership | 2 failed: `watch_profile_prints_only_member_changes`, `watch_profile_prints_a_pane_leaving_the_profile_once` |

T-red's RED run already showed that the old SHOWN-against-DRIVEN rule fails c1, c2, c3 and c6. So the comparison is
pinned in both directions.

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| `cargo test --workspace` (as `HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, CI's skip) | 0 failed | 128 result lines: 1472 passed, 0 failed, 5 ignored; exit 0 | PASS |
| `cargo test -p holler-cli --test pane_cli_process` | 0 failed | 34 passed | PASS |
| `cargo test -p holler-cli --test cli_surface_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | 0 failed | 3 passed | PASS |
| `cargo machete` | no unused deps | none found | PASS |
| `rustfmt --check --edition 2021` on the three verb files and three test files | exit 0 | exit 0 | PASS |

Server start and API smoke do not apply. Until #649 the binary wires `Unwired`, and the verbs run in-process over the
fakes.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage: a test per AC | AC 3 is backed by its three tests over the one six-row `sync_rig` (c1 to c6). AC 19 is backed by the help tests with the amendment's needles. W-13 is backed by its guard test, and W-12 by the existing `watch_profile_*` tests (Mc5). AC 1, 2 and 4 to 18 are unchanged from round 2 | PASS |
| Test quality | Each new or changed test names one behaviour. Each asserts T-written literals (`SYNC_WANT`, the `shown_driven` table), not values derived from the code. They run at the in-process tier. `SYNC_WANT` is one table shared by three verbs, so it is not three copies. Mc1 to Mc5 show each test fails when its behaviour is removed. The suite is proportionate: 36 read-verb tests for three verbs | PASS |
| Type safety | `SessionSync::of(&Pane)` has two callers, and `grep -rn SessionSync crates` finds no other. Clippy is clean under `-D warnings` | PASS |
| Reuse (no second comparison) | SYNC calls `holler_pane::reconcile::shown_differs`, and `watch` calls `profile_diff::is_member`. `last_observed.driven` is only printed (c6: DRIVEN `ses-b` differs, and SYNC is `ok`) | PASS |
| Error paths, data integrity, API contract | Serde names `ok`, `mismatch` and `unobserved` are unchanged (JSON asserted in all three AC 3 tests). No error path changed | PASS |
| Security | No secret-shaped value in tests or output (`demo-*`, `ses-a`, `ses-b`, `/srv/demo` only). Text escaping is unchanged | PASS |
| Migration safety, protocol and goldens | No schema, store, `holler-proto` or golden change | N/A |
| AC 17, 21, 22 greps | `ports\.(herdr\|host\|harness\|prober)`, `not_implemented\|const STORY`, and `unsafe` (on the seven files) | nothing printed. PASS |
| Manifests (AC 22) | `git diff --stat $(git merge-base HEAD origin/main) -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` | empty. PASS |
| File sizes and ASCII | `wc -l`: src 335, 277 and 223; tests 587, 389 and 320, all under 900. All six files are ASCII-only, with no new `#[allow]` | PASS |
| Blast radius (AC 24) | `git diff --name-only <merge-base>...HEAD`, outside `docs/handoffs/643*`, lists exactly the brief's ten "Files" | PASS |
| Playwright / browser | No such surface in this repo | N/A |

**Evidence appendix:** T added nothing this phase. The new tests rely on two unchanged-code facts, and `evidence.md`
already covers both:
- `shown_differs` and its `at > 0` proviso. The brief quotes them (`reconcile.rs:171-181`), and evidence.md line 8
  points there. T checked that `reconcile.rs:179-180` still matches.
- `is_member`. F's W-12 entry quotes `profile_diff.rs:261-265` verbatim.

`evidence.md` is 11,901 bytes, under the 12,000-byte cap.

**F's commands cross-checked:** T re-ran every command in handoff-F.md's Tier 1 self-check. The results match F's:
- the test counts: 36, then 3, 3, 34 and 127, and the workspace run's 1472 passed, 0 failed and 5 ignored;
- clippy, lint, changelog, machete and rustfmt all exit 0;
- the greps print nothing, and the manifest diff is empty;
- the file sizes are 335, 277 and 223 lines.

There is no discrepancy.

## Acceptance criteria status

All 24 PASS. AC 1, 2 and 4 to 18 are backed as in round 2's table, and their tests pass in the 127. Amendment 1 changes
the backing of these:

| AC | Status | Backing |
|---|---|---|
| 3 SYNC is `shown_differs`'s answer, and DRIVEN is printed, not compared | PASS | `list_flags_a_pane_whose_shown_differs_from_its_session_of_record`, `get_flags_a_mismatch`, `watch_flags_a_mismatch` (c1 to c6); Mc2 to Mc4 |
| W-13 (`at <= 0` is never observed) | PASS | `a_negative_observed_at_is_never_observed`; Mc1 |
| W-12 (`watch --profile` uses `is_member`) | PASS | `watch_profile_prints_only_member_changes`, `watch_profile_prints_a_pane_leaving_the_profile_once`; Mc5 |
| 19 `--help` documents it | PASS | `list_help_documents_the_columns_and_the_json_shape` (`session of record`, `home screen`, `#649`), `get_help_documents_the_json_shape`, `watch_help_documents_the_stream` (`pane get`) |
| 20 Surface rows | PASS | `cli_surface_test`, `docs_cli_test`, `pane_cli_process`. `// #643` is on `stub.rs:18`, and no `643` STUBS row is left |
| 21 No stub | PASS | grep prints nothing |
| 22 Quality gates | PASS | Tier 1 above |
| 23 CHANGELOG | PASS | Both sentences are replaced as specified, and the entry ends with the `[#643]` link. `changelog-check: ok` |
| 24 Blast radius | PASS | The brief's ten files plus `docs/handoffs/643*` |

## Blocking issues

None.

## Advisory notes

- **Doc wording (cosmetic).** `list.rs`'s module doc and `SessionSync`'s doc call `shown_differs` "the one SHOWN/DRIVEN
  rule". That name is accurate in the ADR's sense (the session of record is the session the hub drives). But it sits
  next to "DRIVEN is ... never compared", where DRIVEN means the `last_observed.driven` column. A reader could trip on
  the two meanings. Not blocking: the help text users see names the session of record.
- **The branch is 2 commits behind `origin/main`** (17 ahead, 2 behind after `git fetch`). F reports that it merges
  cleanly. The run's agent brings `main` in before merging.
- **evidence.md has 99 bytes of headroom.** A later entry has to replace older text, or the gate drops the tail.
- #647's unseen first observation (a false MISMATCH is possible), D-2 to D-6, and A's open warns are unchanged. They
  are the MO's to file.
