# Handoff-T-green: Phase 7 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark`

**Date:** 2026-10-09
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`, head `693b29c`, merge base
`ce12cdb`)
**Issue:** #646 (part 1 of 3)
**Handoff-F reviewed:** `docs/handoffs/646/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/646/handoff-T-red.md`

The Workflow script calls this step "Phase 7". In the role doc's numbering it is Phase 6 (verify GREEN).

## GREEN confirmation

F changed no test file (`git diff --stat b996a8f 693b29c` lists only `park.rs`, `unpark.rs`, ADR-0021, CHANGELOG and
handoff files). F listed no test under "Tests that look wrong".

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All 10 tests authored at RED pass, and so do the 93 that were already there.

**Mutation spot-check on F's real code** (one mutation at a time in `src/pane/park.rs`, `pane_verbs -- park` run each
time, then the file restored; the scratch script was never staged):

| Mutation | Result |
|---|---|
| Park overwrites any hold (drained or already parked) | caught: AC 3, AC 8 |
| Unpark also clears a drained pane | caught: AC 3 |
| No name-order sort in `in_scope` | **survived at first**, caught after the repair below (AC 4) |
| Continue after a failed write | caught: AC 8 (park and unpark) |
| Progress suffix on a one-pane scope | caught: AC 8 (park and unpark) |
| "parked by this run before it" also lists panes left as they are | caught: AC 8 (park and unpark) |
| Store the untrimmed text | caught: AC 7 |
| No control-character guard | caught: AC 7 |
| Cap off by one (201 characters accepted) | caught: AC 7 |
| A named member with `--profile` skips the membership check | caught: AC 5 |
| `since` taken per pane, not once per run | caught: AC 4 |
| Text line drops `already` | caught: AC 3, AC 8 |

### Test repaired in this phase (T's own hole, not F's)

- **The hole.** `ProfileScope::resolve` promises no order (`crates/holler-pane/src/profile.rs:380-389`). AC 4 says the
  members are taken "in name order". The fake scope sorts its answer itself (`profile_scope.rs:105-115`), so the AC 4 test
  passed with the verb's sort deleted: it pinned the fake, not the verb.
- **The repair** is in `crates/holler-cli/tests/pane_verbs/park.rs` (now 489 lines):
  - a test-local `ReversedScope`, which delegates to the rig's `FakeProfileScope` and reverses `resolve`'s panes, with
    `edit_spec` delegated unchanged;
  - `assert_name_order_is_the_verbs`, which first checks that the wrapper really answers `demo-c2r1, demo-c1r1`, then
    runs park and then unpark over `Ports { scope: &reversed, ..rig.ports() }`. It asserts exit 0, the two text lines in
    name order, and the AC 9 check.
  - It is called from `park_and_unpark_with_a_profile_take_every_member_in_name_order`, on a fresh `profile_world()`.
- **Results.** With the sort removed, that test fails. With F's code it passes, so no production change is needed. No
  production file was touched.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests (CI's form) | `HOLLER_STATE_DIR=<empty dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all pass | 128 targets, 1448 passed, 0 failed, 5 ignored, exit 0 | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (also after the test repair, `-p holler-cli`) | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. The only warning is the pre-existing `holler-pane/src/error.rs is 710 lines` | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Hook tests | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| rustfmt on changed `.rs` | `git diff --name-only ce12cdb HEAD -- '*.rs' \| xargs rustfmt --check --edition 2021` | clean | clean | PASS |

Cross-check against F's reported commands:

- Same results for `pane_verbs` (103), `pane_cli_process` (34, inside the workspace run), `cli_surface_test`, `docs_cli_test`,
  clippy, lint and changelog-check.
- No discrepancy in behaviour. One environment note follows.

Environment note (not a branch fault):

- The first `cargo test --workspace` run failed 4 cases of `logging_test`. The cause is this session's own environment:
  the session exports `HOLLER_STATE_DIR` pointing at a live hub, so `holler roster` succeeded where the test expects
  exit 1.
- With `HOLLER_STATE_DIR` set to an empty directory, as on CI, the whole workspace is green.
- The branch does not touch `logging_test.rs` or the logging code.

`cargo fmt --all -- --check` reports diffs across many untouched crates (`holler-body`, `holler-proto`,
`holler-load-test`, ...), a pre-existing repo-wide state. CI does not run it, and epic ruling 4 forbids reformatting
existing files. Not this story's concern.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC 1-10 each map to a named test (table below); AC 11 to the surface tests; AC 12 and 13 to grep and script checks | PASS |
| Test quality | Each test names a behaviour. 12 of 12 mutations are caught. The tests sit at the in-process tier, which is the cheapest that reaches the verb's envelope and exit code, and none duplicates a `pane_cli_process` case. One hole (AC 4 order) was found and closed above. The suite is proportionate: 10 tests across 4 files, all under 600 lines | PASS |
| Type safety | `HoldTarget` makes "neither PANE nor `--profile`" unrepresentable. There are no casts and no `unsafe`; `grep` of the diff found none | PASS |
| Error handling | Every refusal path (usage a-e, `pane-not-found`, `pane-not-in-profile`, `profile-not-found`) and every store failure (conflict, unavailable, the nth write) is asserted with the exact message and exit code in both formats | PASS |
| Data integrity | One compare-and-swap per pane at the generation it read, with no retry. The run stops at the first failure, and the wrapper's count proves no write follows it. Rerun after a partial failure is idempotent. Untouched records are asserted equal to the seed | PASS |
| API contract | JSON `data` `{"panes":[{name, changed, generation, hold}]}` is asserted exactly. `check_envelope` passes on every JSON output. Exit codes are equal across formats | PASS |
| Security and input validation | The text guards refuse blank text, control characters and more than 200 characters. The usage message never holds the value (asserted). Text output quotes stored text through `findings::quoted`. No secret is involved | PASS |
| Migration safety | No schema, record or protocol change. No golden files are affected | N/A |
| Manifests | `git diff --stat ce12cdb HEAD -- '*Cargo.toml'` is empty | PASS (see advisory) |
| No live act (AC 9) | `assert_no_live_call_and_no_profile_write` in every test, the new order check included | PASS |
| Repo rules | Test and support files are under 900 lines (largest is `park.rs` at 489). No new `#[allow]`. No fixed sleeps. No `[[test]]` entry is needed (modules of the existing `pane_verbs` target) | PASS |
| Browser / Playwright | Not applicable in this repo | N/A |

Evidence appendix: one T entry appended to `docs/handoffs/646/evidence.md`. It covers `ProfileScope`'s missing order
promise, which `ReversedScope` relies on, with the excerpt copied from `profile.rs:380-404`.

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 Park then unpark one pane (text) | PASS | `park::park_then_unpark_round_trips_one_pane` |
| 2 The same in JSON | PASS | `park::park_and_unpark_json_pass_the_envelope_helper` |
| 3 Idempotent, nothing written | PASS | `park::park_and_unpark_leave_a_pane_already_in_that_state` |
| 4 Every pane of a profile, name order, one `since` | PASS | `park::park_and_unpark_with_a_profile_take_every_member_in_name_order` (with T's order repair) |
| 5 Membership and missing profile refused | PASS | `park::park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile`, `unpark::unpark_of_a_member_named_with_its_profile_changes_only_that_pane` |
| 6 A missing pane | PASS | `park::park_and_unpark_refuse_a_pane_with_no_record` |
| 7 Usage before any store call | PASS | `park::park_and_unpark_usage_errors_touch_no_store` |
| 8 Store failures name the pane and stop | PASS | `park::failures::park_failures_name_the_pane_and_stop`, `unpark::unpark_failures_name_the_pane_and_stop` |
| 9 No live act, no profile write | PASS | the rig's AC 9 check, in every test above |
| 10 Exit codes equal across formats; envelope helper | PASS | `run_both`, in every AC 5-8 case |
| 11 The surface | PASS | ADR-0003 rows 54-55 match the brief exactly; fixture lines 132-135 match; `stub.rs` entries removed; `pane_cli_process` 34, `cli_surface_test` 3, `docs_cli_test` 3 and `pane_verbs` all pass |
| 12 ADR-0021 edited | PASS | `grep -c '^\*\*Park and unpark as built (#646).\*\*' docs/adr/ADR-0021.md` is 1 (wording is S's to audit) |
| 13 Hygiene | PASS | clippy clean; rustfmt clean on changed files; lint exit 0; no manifest change against the merge base; no `unsafe`; one CHANGELOG entry under `[Unreleased]` |

## Blocking issues

None.

## Advisory notes

- **AC 13's literal manifest check.** `git diff --stat origin/main -- '*Cargo.toml'` is not empty (adapter-herdr, -host,
  -opencode). That is upstream drift since `ce12cdb`, not this branch. Against the merge base it is empty. A rebase
  before the PR clears it.
- **Running the workspace suite in this session** needs `HOLLER_STATE_DIR` pointed at an empty directory. Otherwise
  `logging_test` sees the operator's live hub (see Tier 1).
- **A's follow-ups** (ADR wording warns 2 and 3, the shared "panes in scope" helper, the rig consolidation) are as F
  listed them, for O.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
