# Handoff-T-green: Phase 7 - #839 DRIVEN follows each pane's session of record

**Date:** 2026-10-12 (09:41 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`, head `d480424`; T's test repairs are in the working tree, uncommitted)
**Issue:** #839
**Handoff-F reviewed:** `docs/handoffs/839/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/839/handoff-T-red.md`

**VERDICT: GREEN, no blocking issue.** F's six "tests that look wrong" (T-1 to T-6) were each checked against the source and the brief, and each was T's mistake. T repaired all six in the tests alone; no production file was edited. With the repairs the whole workspace passes: 2721 passed, 0 failed, 36 ignored.

## Test repairs (T-1 to T-6, test code only)

| # | File | What was wrong | Repair |
|---|---|---|---|
| T-1 | `crates/holler-hub/tests/driven_support/mod.rs` | `TestHub::body` never bound the token's label, so roster rows were `oc`, not `kiwi/oc`. The circuit binds it at authentication (`circuit.rs:306-307`). | `roster.set_token` and `roster.set_label` before `advertise`, as the circuit does. |
| T-2 | `crates/holler-hub/tests/check_only_test.rs` | `error_shape` compared a hold's `data.since` across two hubs, each of which set its own hold. | `error_shape` drops `since` before comparing. |
| T-3 | `crates/holler-cli/tests/hold_single_path_test.rs` | `sources()` scanned out-of-line test modules whole (no `#[cfg(test)]` line to cut at), so `panes/gate/tests.rs`'s expected values counted as a second definition. | `sources()` skips `tests.rs` and `*_tests.rs` files. The other scans in the file stay green. |
| T-4 | `crates/holler-cli/tests/driven_e2e_test.rs` | `Rig::start` passed `hub_args` as the hub's whole serve-args list, so its hubs bound the default port (`127.0.0.1:41807`) and collided with any running hub. | `--listen 127.0.0.1:0` is prepended to `hub_args`. |
| T-5 | `crates/holler-body/tests/rebind_test.rs` | `until_attached` returned when the row was visible, but a row is visible before its task runs and hidden during the first attach, so tests raced the first attach. | `until_attached(sm, server, since, session, id)` waits for the first attach's `GET /session/{id}` after `since`, then for the row. The restart test takes `since` before the second `start`. |
| T-6 | `crates/holler-cli/tests/cli_invocation_test.rs` | `say_without_live_hub_exits_1` and `interrupt_without_live_hub_exits_1` typed `SESSION`, which is not a name. The brief (P3) makes the CLI refuse a non-name as `unknown session` before any hub, so the `no live holler hub reachable` pin no longer held. | The four `args` arrays type `session`, a valid name. A comment cites #839 P3. |

## GREEN confirmation

```text
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
175 test binaries: 2721 passed; 0 failed; 36 ignored            (exit 0)
```

The repaired targets alone:

```text
$ cargo test -p holler-hub --test check_only_test --test driven_resolve_test --test offline_reason_test
4 passed; 11 passed; 3 passed
$ cargo test -p holler-cli --test cli_invocation_test --test driven_e2e_test --test hold_single_path_test
14 passed; 11 passed (13.2 s); 5 passed
```

**Repeats (flake check):**
- `rebind_test` alone, 15 runs: 15 passed, 0 failed. Before the T-5 repair F saw it fail 11 times in 12.
- `check_only_test`, `driven_resolve_test` and `offline_reason_test` together, 10 runs: 0 failures.
- `driven_e2e_test`, 3 more runs: 0 failures.

**Spot-checks: each removal of a behaviour fails its test** (a temporary edit of production code, then restored with `git checkout --`; `git status -- crates/*/src` is clean):

| Mutation | Fails |
|---|---|
| `tx_switch::switch_in_place` records first and rebinds after (order) | `pane_verbs` `driven::order::a_switch_re_points_the_body_between_the_observation_and_the_record`, `a_refused_rebind_is_a_failure_after_the_act_that_records_nothing`, `driven::evidence::a_body_another_pane_records_...` |
| `gate::first_refusal` keys on the session of record only (recorded-DRIVEN key removed) | `gate_test` 2, hub lib 3 (`hold_tests` 2, `panes::gate::tests` 1), `fake_hub_test` 2 (including the conformance suite), `driven_e2e_test` 2 (including the real client's conformance) |
| `driver_of` without the exclusion rule | `driven_test` 2 |
| the body's rebind attach given a deadline 60 s later (late attach) | `rebind_test::a_silent_endpoint_is_refused_in_time_and_a_late_attach_is_never_swapped_in` |
| `circuit::dispatch::send_prompt` without `pane_gate` | `hold_tests` 3 |
| `talk::admit` without `pane_gate` | `check_only_test::say_and_the_check_refuse_the_same_cases_alike_before_anything_moves` |
| `resolve_prompt_target` matching a spawn row by id | `driven_resolve_test::a_name_shaped_id_never_resolves_by_id_and_a_spawn_row_is_never_matched` |
| the CLI's bare-id refusal in `route_target_by` removed | `driven_e2e_test::a_typed_bare_id_is_unknown_with_no_hub` |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0, also after the repairs | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0, with size warnings only, none at 900 | PASS |
| Changelog | `bash scripts/changelog-check.sh origin/main` | ok | `changelog-check: ok` | PASS |
| Workspace tests | as above | 0 failed | 2721 passed, 0 failed | PASS |
| Docs | `cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test` | pass | 3 passed; 20 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Hooks | `bash scripts/test-hooks.sh` | all ok | all ok | PASS |
| Goldens | `git diff --name-status <merge-base> HEAD -- crates/holler-proto/tests/golden` | three new files, none changed | `A` for `Rebind.json`, `RebindResult.json` and `error.not_connected_offline.json`, and no other file | PASS |
| Server start | `driven_e2e_test` (a real hub and a real body, ephemeral port, throwaway state dir) | pass | 11 passed | PASS |

**Cross-check of F's results:** they agree. F's 18 failures were 15 from T-1 to T-6, the five doc commands (fixed before F's commit), and the #665 inode flake. In T's full run all three groups pass: the T fixes are in, `docs_cli_test` is green, and `profile_rename_faults_test` passed in the full suite (F reports it as a flake that predates #839 and fails alone; T did not re-run it alone).

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per acceptance item | The T-red table maps Acceptance 1 to 22 to tests; every target in it is green. The gaps T-red named are unchanged (`admin/query` with `session/rebind` is covered through the shared `hub_query_remote`). | PASS |
| Test quality | Each repair keeps the test's behaviour and fixes only its setup or comparison. The spot-checks above show the key tests fail when the behaviour goes. No test was found redundant. | PASS |
| Type safety | Clippy with `-D warnings` across all targets | PASS |
| Error paths | `offline_reason_test` (offline marked, timeouts bare), the rebind refusals in `rebind_test` and `driven_resolve_test`, `reset --first` refusals in `pane_verbs` `driven::first` | PASS |
| Data integrity | The record is written in one `cas_put` after the rebind (order mutation fails); a refused rebind records nothing; the body swaps only after a successful attach (deadline mutation fails) | PASS |
| API contract | Three new goldens; the catalog count of 23; `codec_test` round trip | PASS |
| Security | `control/rebind` fails closed on its params, the id, the token and the cached session (`driven_resolve_test`); the query allowlist is unchanged, and the source scan (`hold_single_path_test`) pins the one forward | PASS |
| Concurrency | The one reply slot per connection (`driven_resolve_test`, `driven_e2e_test`), repeated without a failure | PASS |
| Migrations | None in this story | N/A |
| Playwright | No browser surface in this repo | N/A |

**Evidence appendix:** T appended two entries to `docs/handoffs/839/evidence.md` for the source facts the repairs rely on: the circuit's label binding at authentication (T-1), and the attach's `GET /session/{id}` existence check (T-5).

## Acceptance criteria status

All 22 pass, each backed by the tests the T-red table names. The items whose tests were repaired in this phase:
- 3 and 12 (`driven_e2e_test`): PASS;
- 5, 9 (hub) and 21 (`driven_resolve_test`): PASS;
- 9 (body, `rebind_test`): PASS;
- 10 (`check_only_test`): PASS;
- 13 and 14 (`hold_single_path_test`): PASS;
- 18 (`driven_e2e_test::a_typed_bare_id_is_unknown_with_no_hub`, `cli_invocation_test`): PASS.

## Blocking issues

None.

## Advisory notes

1. **Acceptance 20(d) row (1) still needs O's ruling.** The code and the test follow D3's order, so a switch onto another pane's session of record is `session-of-other-pane` (T-red note 1).
2. **A regression in the gate makes `driven_e2e_test` slow to fail.** With the recorded-DRIVEN key removed, the run took 613 s: the conformance cases that expect a refusal instead send a real turn and wait up to `SEND_TURN_BUDGET` (600 s). It does fail, but a CI run would sit for about ten minutes first. A follow-up could give the conformance's real-client `Send` a shorter budget.
3. **`send_prompt` spends a grant before the pane gate refuses** (`circuit/dispatch.rs:112-114`: `holds.admit` and then `pane_gate`). This only happens when a pane goes bad between `talk::admit`'s fast path (which checks the pane gate and consumes nothing) and `send_prompt`. The brief orders the hold first. O may want to note it.
4. **`profile_rename_faults_test`** (#665) is a flake that F reports fails alone; it is outside this story and still needs its own issue.
5. **`crates/holler-hub/src/live/tests.rs`** is a pure move of `live.rs`'s inline tests, already in F's commit. The six repaired test files and `evidence.md` are uncommitted in the working tree.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
