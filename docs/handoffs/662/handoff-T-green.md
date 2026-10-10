# Handoff-T-green: Phase 6 - #662b profile write verbs (`holler profile create`, `holler profile delete`)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `22cf75f`)
**Issue:** #662 (662b; closes the issue)
**Handoff-F reviewed:** `docs/handoffs/662/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/662/handoff-T-red.md`

## What T changed in this phase

| File | Change |
|---|---|
| `crates/holler-cli/tests/profile_verbs/create.rs` | **One test added:** `create_from_current_undoes_the_joined_panes_newest_first`. It covers a gap that the mutation spot-check found (M6 below). B2 step (2) says the joined panes leave "in reverse join order". No test failed when the undo ran oldest first, because every earlier undo test either failed the first undo step or let the whole undo complete. The new test has three panes. The 3rd join fails and the 5th `cas_put` fails, which is the second undo step. Newest first, that step is `demo-c1r1`'s undo, so only `demo-c1r1` stays a member and `demo-c1r2` is cleared. Oldest first would leave the opposite state. The file now has 564 lines. It is staged by path. |
| `docs/handoffs/662/evidence.md` | A `## T (Phase 6)` section with three test-kit facts that the tests rely on and that F did not list: the fake's `list` is sorted by name, `fail_next` fires once, and the call log includes failed calls. Each fact has its `file:line` and a verbatim excerpt that T copied from source. |

F flagged no tests as wrong ("Tests that look wrong (for T)": None), so no existing test was repaired. No production code
was changed. Each mutation below was reverted with `git checkout -- <file>`, and `git status` confirms the production
files are clean.

## GREEN confirmation

`cargo test -p holler-cli --test profile_verbs`:

```
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

That is the 43 tests F reported plus T's new one. The suite was repeated 10 times in a row and passed 44 of 44 every
time, so there is no flake. Every test runs in-process over the fakes, with no sleep and no subprocess.

Surface targets, in one run (`--test profile_verbs --test cli_surface_test --test docs_cli_test --test pane_cli_process`):
`cli_surface_test` 3 passed, `docs_cli_test` 3 passed (the RED on ADR-0003:65-66 is gone), `pane_cli_process` 34 passed,
`profile_verbs` 43 passed (before T's addition).

**Spot-check (mutation): the tests fail when the behaviour is removed.** For each mutation, T changed one line of
production code, ran `profile_verbs`, and reverted the change.

| # | Mutation | Result | Killed by |
|---|---|---|---|
| M1 | undo skips B2's re-read of the failed pane | killed | `create_from_current_undoes_a_join_that_landed_but_timed_out` |
| M2 | `insert_profile` does not map `Conflict` to `profile-exists` | killed | `create_reports_a_create_race_as_profile_exists` |
| M3 | delete's `Conflict` after a detach is not mapped to `profile-conflict` | killed | `delete_conflict_after_a_detach_is_profile_conflict` |
| M4 | `single_quoted` does not escape `'` | killed | `delete_quotes_a_name_with_a_quote_in_its_suggested_command` |
| M5 | plan check ignores `is_member` (a pane naming NAME's slug is refused) | killed | `create_from_current_joins_a_pane_that_already_names_the_profile` |
| M6 | undo clears joined panes oldest first | **survived**, then killed after T's addition | `create_from_current_undoes_the_joined_panes_newest_first` |
| M7 | delete ignores live members without `--keep-panes` | killed (3) | `delete_refuses_while_panes_are_live`, `delete_counts_members_by_slug`, `delete_quotes_...` |
| M8 | undo never deletes the profile | killed (2) | `create_from_current_undoes_everything_when_a_join_fails`, `..._landed_but_timed_out` |
| M9 | `--from-current` skips the join write | killed (5) | every `--from-current` join test |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. The only output is the existing size warnings for untouched files; the largest touched file is 564 lines. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok` | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (re-run as `-p holler-cli --all-targets` after T's test: clean) | PASS |
| Workspace tests | `HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's skip) | all pass | 128 targets, **1464 passed, 0 failed, 5 ignored**, exit 0 | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | "didn't find any unused dependencies" | PASS |
| Hooks | `bash scripts/test-hooks.sh` | pass | all `ok` | PASS |
| Format | `rustfmt --check --edition 2021` on all 7 touched `.rs` files | exit 0 | exit 0 | PASS |

**Cross-check against F's report.**

- F's isolated workspace run had 2 `interrupt_test` failures, which F called a liveness-check flake under the parallel
  run. T's isolated run of the same command had **0 failures**. That fits F's diagnosis: the failures are flaky, and no
  profile verb reaches that code.
- F's first, plain run failed `logging_test` because of the machine's live hub. T ran only isolated, as F advised, so
  that difference is from the environment.
- Every other number matches F's report: 43 passed in `profile_verbs`, and the surface counts, clippy, lint, fmt and
  changelog results are the same.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC 2a-2m and 3a-3h each have the test the brief names. AC 4 (stub cases gone, `662),` count 0, `// #662` kept) and AC 5 (ADR-0003 rows, fixture group exact) were checked by grep and by the surface targets. | PASS |
| Test quality | Each test names one behaviour and asserts outcomes: exit code, envelope code, message parts, and store state on **both** rigs. It never asserts internal calls beyond the brief's "no write" call-log checks. 8 of 9 mutations were killed, and the 9th is now killed by T's one added test. No two tests are redundant. The env-decode half of 2j overlaps `holler-pane/tests/argv_env_test.rs:96`, but AC 2j asks for it on the created spec, which was journalled at RED. The suite is proportionate: 27 verb tests for two verbs with 5 partial-write branches. | PASS |
| Type safety | No `unsafe`, no `#[allow]` added (`git diff ce12cdb -- '*.rs'`); `JoinFailed` sized under clippy's `result_large_err` | PASS |
| Error paths | Every B4 row has a test: exists (by name and by slug), create race, not-found (both verbs), other-profile (several panes, `; `-joined), join failure with the undo complete, undo failure, a join that timed out after landing, live panes, detach failure at the 1st and 2nd write, delete `Conflict` with and without detaches, and usage. | PASS |
| Data integrity | A refused create writes nothing (call-log checks). A failed create leaves no profile and no member, or else answers `profile-conflict` with the profile kept. The undo order is now pinned. A failed delete is never re-attached. Concurrency is modelled as invariants via the seam, not timing. | PASS |
| API contract | `Created {profile, members}` / `Deleted {name, slug, detached}`; JSON key order is checked in 3a; every JSON output passes `check_envelope`; text and JSON exit codes are equal (in `run_both`) | PASS |
| Security / I7 | AC 2j: env holds names only, and a decode with a value fails as `profile-secret-refused`, with the value absent from the error text. Suggested commands are single-quoted for the shell (M4). AC 1 grep: no adapter, `scope`, prober or env access. | PASS |
| Migration safety | N/A: no schema or storage change | N/A |
| Protocol / goldens | N/A: no wire change; `docs/protocol/v2.md` untouched | N/A |
| Evidence appendix | F's 16 facts plus T's 3 test-kit facts (`evidence.md`, `## T (Phase 6)`) | PASS |
| Playwright / browser | N/A: no UI surface in this repo | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 No adapter/probe/env | PASS | `run_both`'s `assert_no_adapter_call` in every verb test; grep prints nothing |
| 2a-2m create | PASS | the 15 create tests named in handoff-T-red, plus `..._undoes_the_joined_panes_newest_first` (B2 order) |
| 3a-3h delete | PASS | the 11 delete tests named in handoff-T-red |
| 4 Stub cases gone | PASS | grep: no `assert_stub_routes`; `grep -c '662),' stub.rs` = 0; `// #662` at stub.rs:35 |
| 5 Surface | PASS | ADR-0003:65-66 exact, `#662` in column 67; fixture group exact; three surface targets green |
| 6 ADR-0021 | PASS | each grep prints one line (346, 347); the diff is only those two rows |
| 7 Crate tests | PASS | `profile_verbs` 44/44; workspace 1464/0 |
| 8 Lints | PASS | clippy clean; lint.sh exit 0; no `#[allow]` added; every file under 900 lines |
| 9 No unsafe / deps | PASS | no `+...unsafe` line in the diff; no manifest or lock change |
| 10 Formatting | PASS | `rustfmt --check --edition 2021` exits 0 on every touched `.rs` file |
| 11 CHANGELOG | PASS | one `[Unreleased]` entry naming both verbs, `not-implemented` until #649, linking #662; check ok; no host or account name |
| 12 Blast radius | PASS | `git diff --name-only ce12cdb` is the brief's Files plus `docs/handoffs/662*` |

## Blocking issues

None.

## Advisory notes

- **The `CHANGELOG.md` rebase conflict with `origin/main`** (F's Known issues) is still open. Keep both entries and re-run
  `changelog-check.sh` after the rebase.
- **The unmapped errors F listed in Known issues still stand, as the brief specifies.** Both pass through with no
  detached list, as B4 maps it, and re-running the verb converges.
  - A `timeout` from `insert_profile` after its write landed.
  - A non-`Conflict` error from delete's final profile delete after some detaches.
- **F's `interrupt_test` flake did not reproduce here.** If it recurs in CI, it belongs to the hub liveness code, not to
  this story.
- **A's W-2 is still open.** The `profile-conflict` definition and the `--keep-panes` I1 exception are left for a
  follow-up, as F recorded.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
