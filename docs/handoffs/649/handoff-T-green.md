# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario  (D33 cycle: #832's observe methods)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `736f547` plus this pass)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md` (fourth cycle, D33)
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md` (D33 cycle)
**Handoff-S:** `docs/handoffs/649/handoff-S.md` (second pass, ADVISORY-HOLD; step 4 and advisory 4 are this pass's)

This handoff replaces the fifth pass's (`98280ad`), which git history keeps. Its results (M1 to M10, the 6 s pin, the
fake-hub rework) stand. **This pass changes no production code.** It adds one end-state check to the opt-in scenario
(`scenario.rs`, S's advisory 4) and one evidence entry for it.

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration` (default run, inside the workspace run below) | `ok. 66 passed; 0 failed; 4 ignored; ... finished in 119.14s`. At T-red this was 59 passed and 7 failed: all seven D33 rows are now green |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` (merged tree, with the new end-state check) | `ok. 4 passed; 0 failed; ... finished in 59.05s`. No new `/tmp/hlr649-*` directory and no `hlr649`/`holler649` process left (the two older dirs F named are unchanged) |

**The tests fail when the behaviour is removed.** Three mutants of F's `bind.rs`, each applied by a script, run with
`-- observe:: bind::`, and restored from a scratchpad copy (`git status` shows no change under `src/` after each):

| Mutant | Failing rows |
|---|---|
| M11: `BindingHerdr::observe_panes` loops over the inner `observe_pane` (correct values, wrong call shape) | `observe::the_binding_herdr_delegates_…`, `observe::the_wirings_herdr_chain_…`: `left: ["observe_pane", "observe_pane", "observe_pane"]`, `right: ["observe_panes"]` |
| M12: `LazyHerdr::observe_pane` answers `Ok(default)` when the build failed | `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method`, `observe::the_lazy_herdr_passes_the_herdr_conformance_suite` |
| M13: `LazyHerdr::observe_panes` answers `[]` on a failed build | `observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane` |

T-red already showed the no-forward state fails all seven rows. So each line F added is pinned.

## What this pass changed (test-only)

- **S's advisory 4.** `scenario.rs`'s `assert_end_state` now also reads an unscoped `pane list --format=json` through the
  real wiring after the repairs, and asserts:
  - no `unmanaged_unavailable`: the real `LazyHerdr` chain reads Herdr;
  - no pane id a record names (C1's or C2's Herdr pane) is in the `unmanaged` table.
  It passed in the opt-in run above. The file is 394 lines; the added lines are rustfmt-clean (the file's other 23
  rustfmt hunks predate this pass).
- **Evidence.** A new top section in `evidence.md`: the `"unmanaged"` array's shape (`list.rs:83-84`), copied from
  source. The `unmanaged_unavailable` line is already quoted by T-red's section.

## Tier 1 results

| Check | Command | Actual | |
|---|---|---|---|
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | 151 binaries: 2344 passed, **1 failed**, 32 ignored. The failure is `remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen` (`last_seen` moved 1791759181 → 1791759182). See below | PASS (flake, not this story) |
| The failed test, re-run | `cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never_moves_the_bodys_last_seen`, three times; then the whole target | 1/1 passed each time; the target 21/21 | PASS |
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0; size warnings only, for files this story does not touch | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | PASS |
| Unused deps | `cargo machete` | none | PASS |
| Docs CLI | `docs_cli_test` (in the workspace run) | 3 passed | PASS |
| Wire canary | `wire_selftest` (in the workspace run) | 3 passed | PASS |

**The one workspace failure is not this story's.** `remote_admin_test.rs` and `holler-hub` are untouched by the branch
(`git diff origin/main...HEAD` lists neither). The test sleeps 2 s and asserts a whole-second `last_seen` did not move;
under the full workspace's parallel load it moved by one second, and it passes in isolation every time. It is a
timing flake of #508's test, for O to file. It is not a blocker for F.

**Cross-check against F's handoff.** F reported 66/0/4 for the default run, 4/4 for the opt-in run, and 2345/0/32 for
the workspace. Mine are 66/0/4, 4/4, and 2344/1/32: the same total, with the one difference the flake above. The
rename scan passes, as F reported.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | D33's four pins (conformance over both wrappers, `observe_pane` in the every-method row, one recording check per wrapper and the chain, the failed-build row) and its `store_only` sentence are each a test; S's step 4 and advisory 4 are now the opt-in run plus the new end-state check | PASS |
| Test quality | Each D33 row names its behaviour and fails on its own assertion (T-red's RED, M11 to M13). The new end-state check sits in the existing scenario, the only tier that sees a real Herdr through the real wiring, and does not duplicate `store_only`'s no-Herdr row | PASS |
| Behaviour, not implementation | The recording rows assert answers and the call shape ADR-0021 section 14 requires (one `observe_panes`), not F's code. The end-state check asserts the JSON contract, not wording | PASS |
| Type safety | Clippy `-D warnings` clean; F's change adds no cast | PASS |
| Error handling | Failed build: `observe_pane` answers the stored `usage`, `observe_panes` one empty observation per pane, no retry. With no Herdr, `pane list` exits 0 with `unmanaged_unavailable: usage` (`store_only`) | PASS |
| API contract | No protocol, golden or `docs/protocol/v2.md` change; the port trait is #832's, unchanged | PASS |
| Security | No secret enters the new assertions' messages (pane ids and the list's own JSON) | PASS |
| Data integrity, migration safety, Playwright | N/A: no schema, no store change in this cycle, no browser surface | PASS |

## Acceptance criteria status

All PASS. D33 is backed by:
- `observe::the_lazy_herdr_passes_the_herdr_conformance_suite` and `bind::the_binding_herdr_passes_the_herdr_conformance_suite`;
- `observe::the_binding_herdr_delegates_observe_pane_and_observe_panes`, `observe::the_lazy_herdr_delegates_…`,
  `observe::the_wirings_herdr_chain_delegates_observe_panes_end_to_end`;
- `observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane` and the every-method row in `bind.rs`;
- `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode` (D33's last sentence);
- the opt-in `scenario::the_scenario_runs_in_order_on_scratch_instances`, now with the unmanaged-table check.

The other criteria keep the tests named in the fifth pass: `no_hub::*`, `store_only::*`, `env::*`, `wired::*`,
`guards::*`, `watch::*`, `fake_hub::*`, `bind::*`, the opt-in `scenario::*`, and the D28 sentinel checks.

## Blocking issues

None.

## Advisory notes

- **For O: a flake to file.** `remote_admin_traffic_never_moves_the_bodys_last_seen` (#508) failed once under the full
  workspace's load and passes alone. `gh issue list --search` finds no issue for it. CI may hit it too; a re-run clears it.
- `wiring.rs`'s #832 import line is not rustfmt-clean, on `main` as here (F's Design decision 18). CI runs no rustfmt.
- S's advisories 1 to 3 and 5 to 8 are unchanged by this pass; 6 F took, and 4 is now pinned.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
