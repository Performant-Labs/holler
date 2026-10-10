# Handoff-T-green: Phase 7 - #662a profile verbs: the pure core and the read verbs

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `03ed19d`; run 662a only)
**Issue:** #662 (run 662a)
**Handoff-F reviewed:** `docs/handoffs/662/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/662/handoff-T-red.md`

## GREEN confirmation

F's commit `03ed19d` touches no test file (`git show --stat`: only `src/`, ADRs, CHANGELOG and handoffs).

```text
$ cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test --no-fail-fast
profile_diff_test:     ok. 9 passed; 0 failed
profile_snapshot_test: ok. 5 passed; 0 failed
$ cargo test -p holler-cli --test profile_verbs --no-fail-fast
ok. 18 passed; 0 failed        (19 after T's additions below)
```

Every AC 1-2 and AC 4-5 test name from the brief is in the run and passes. 2c (`snapshot_round_trips_to_no_difference`),
which held vacuously at RED, now carries its force over the real `diff_spec`.

**Spot-check: each test fails when the behaviour is removed.** Each mutation was applied to F's code, run, and reverted
with `git checkout` (the tree is clean of them):

| Mutation of production code | Result |
|---|---|
| `env`/`expect` compared in order, not as sets | `diff_compares_env_and_expect_as_sets_and_argv_in_order` FAILS |
| `is_member` compares raw names, not slugs | `is_member_compares_slugs` and `list_counts_members_by_slug` FAIL |
| `FieldValue::Text` writes control characters raw | `field_text_escapes_control_characters` and `show_lists_missing_and_extra_panes` FAIL |
| `show` never reads `probe.last` | `show_reports_the_last_probe_result_without_running_one` FAILS |
| snapshot writes the bare `fixed` policy | `fixed_port_policy_*`, `snapshot_copies_*` and 5 `profile_verbs` cases FAIL |
| (after T's additions) the JSON-array pass leaves DEL/C1 raw | `field_text_escapes_control_characters` FAILS |
| (after T's additions) a count of 1 prints as plural | `list_counts_members_by_slug` FAILS |
| (after T's additions) the probe `error` reason is not `{:?}`-quoted | `show_reports_the_last_probe_result_without_running_one` FAILS |
| `list` drops its `sort_by` | **survives**: see Advisory 1 |

## Test changes in this phase (T's, test code only)

F reported no test as wrong. T added the cases F offered that pin behaviour the brief's Risks care about, by extending
existing tests where one already owned the behaviour:

1. `profile_diff_test::field_text_escapes_control_characters`: DEL and a C1 CSI (U+009B) in `Text`, `List` and `Argv` print
   escaped (`\u{9b}` / `\u009b`), no output holds a control character, and the array form still parses back to the same
   strings (F's decision 3).
2. `profile_verbs::show::show_reports_the_last_probe_result_without_running_one`: a third member with
   `ProbeResult::Error("refused\u{1b}[2J")` prints `  probe: error ("refused\u{1b}[2J")`, no raw ESC reaches `out`, and the
   JSON is `{"error":"refused\u001b[2J"}` (the brief's `error ("reason")` form).
3. `profile_verbs::list::list_counts_members_by_slug`: the weak `contains(", 1 live, ")` became the exact line
   `Some Profile (some-profile): 1 pane, 1 live, generation 1` (singular count, F's decision 8).
4. New `profile_verbs::list::list_passes_a_store_failure_through`: a wedged pane store, after the profile store answered,
   gives `timeout`, exit 1, nothing on `out`, both formats (the `list` analogue of AC 5g).
5. `two_profiles`' doc comment now says the fake lists in slug order, so the test pins the output order, not the verb's sort.

Four facts in unchanged code that these tests rely on were appended to `evidence.md` ("Added by T").

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 (re-run after T's edits for holler-pane and holler-cli: clean) | PASS |
| `bash scripts/lint.sh` | exit 0 | exit 0 (size warnings only, for files outside this diff); touched tests at 122, 325, 336 lines | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo machete` | nothing | no unused dependencies | PASS |
| `cargo test --workspace --no-fail-fast` | pass | 1404 passed, 4 failed, all 4 in `logging_test` (identical to F's report) | PASS (env) |
| `HOLLER_STATE_DIR=<empty dir> cargo test -p holler-cli --test logging_test` | pass | 11 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed (RED's ADR-0003:68 failure is fixed) | PASS |
| `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| `cli_surface_test` / `pane_cli_process` | pass | 3 / 34 passed | PASS |
| `rustfmt --check --edition 2021` on every `.rs` changed vs `origin/main` | exit 0 | exit 0 | PASS |

The `logging_test` failures are `Unexpected success` on `holler roster`, because a hub is reachable from this machine; the
diff does not touch the roster path, and the same 4 failed at RED. CI, with no hub running, is the clean re-check.

Cross-check with F: every command F reported reproduces F's numbers exactly. No discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC 1a-c, 2a-h, 4a-c, 5a-g each map to a named passing test; AC 3, 8, 9, 10, 13, 14, 15 by grep/command (below) | PASS |
| Test quality | Mutations above; each test names one behaviour; pure tests in holler-pane, verb tests in-process over the fakes (no process tier needed); no duplicate pair found | PASS |
| AC 2e grep (no hand-written serde name) | `grep -nE '^[^/]*"(opencode\|agent\|orchestrator)"' profile_diff.rs` | no output, PASS |
| AC 3 (no adapter/probe/env) | `run_both` asserts empty call logs on every verb test; the AC 3 grep over the six files | no output, PASS |
| AC 8 | `grep -n assert_stub_routes profile_verbs/{list,show}.rs` empty; `grep -c '662),' stub.rs` = 2 | PASS |
| AC 9 | ADR-0003:68 `holler profile show NAME`, `#662` at column 67; `cli-surface.txt` 662a lines as listed | PASS |
| AC 10 | `Deferred to #662` absent; `fixed:<port>` at ADR-0021:154-155 (section 3) | PASS |
| AC 13 | no `+unsafe`; no `Cargo.toml`/`Cargo.lock` diff | PASS |
| Type safety | typed `FieldValue`/`SpecField`; JSON only from derived structs; no `unwrap` in production (`unwrap_or_default` on an infallible encode) | PASS |
| Error handling | usage (2), profile-not-found (3), timeout and unavailable (1) for `show`; timeout (1) for `list`; store errors never read as not-found | PASS |
| Security (terminal output) | every stored string in text mode is escaped: `Text`, pane names, argv/list (incl. DEL/C1), probe reasons; asserted with raw-character absence | PASS |
| Data integrity | read-only verbs: no write, no store mutation; membership by slug on both sides | PASS |
| Migrations / goldens / protocol | none: no protocol-visible change, no golden | N/A |
| Browser / Playwright | none in this repo | N/A |

## Acceptance criteria status (662a)

| AC | Status | Backed by |
|---|---|---|
| 1a-c snapshot | PASS | `profile_snapshot_test` (5 tests) |
| 2a-h diff | PASS | `profile_diff_test` (9 tests) + AC 2e grep |
| 3 no adapter/probe/env | PASS | `run_both`'s `assert_no_adapter_call` + grep |
| 4a-c list | PASS | `list_reports_*`, `list_counts_members_by_slug`, `list_of_no_profiles` (+ `list_passes_a_store_failure_through`) |
| 5a-g show | PASS | the nine `show::*` tests |
| 8 stubs gone | PASS | grep |
| 9 surface | PASS | ADR-0003 row, `cli_surface_test`, `docs_cli_test`, `pane_cli_process` |
| 10 ADR-0021 | PASS | greps |
| 11 crate tests | PASS | `profile_verbs` 19/19, `holler-pane` all targets; workspace green but for the environmental `logging_test` |
| 12 lints | PASS | clippy, lint.sh |
| 13 no unsafe/deps | PASS | greps, machete |
| 14 formatting | PASS | rustfmt --check |
| 15 CHANGELOG | PASS | one Enhancements entry linking #662; changelog-check ok |

## Blocking issues

None.

## Advisory notes

1. **`list`'s sort is not pinned.** Both the kit's fake and the hub's store list profiles in slug order (`evidence.md`,
   "Added by T"), so removing the verb's `sort_by` passes every test. The port pins no order, so the sort is right to keep.
   Pinning it needs a store whose `list` answers out of order: a wrapper implementing all nine `ProfileStore` methods in
   the rig, or a test-kit option to shuffle `list`. That costs more than the defensive line it would guard, so it is left
   out; the test comment no longer implies it is pinned.
2. **`logging_test`** fails 4/11 on a machine with a reachable hub, independent of this diff; check it in CI.
3. F's architecture notes (the bare-array `list` data vs #643's object form; the text-form divergence from #643) are for
   the brief's Follow-up and do not affect these tests.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
