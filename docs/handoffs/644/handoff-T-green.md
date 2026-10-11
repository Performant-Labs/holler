# Handoff-T-green: #644 part 2 - the OpenCode agent through `launch` and `relaunch`

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` at `9c76881` (F's commit; merge base `origin/main` `7284001`)
**Issue:** #644
**Handoff-F reviewed:** `docs/handoffs/644/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/644/handoff-T-red.md`

## GREEN confirmation

All 13 tests that were RED now pass, along with the two refusal-table regression pins (green before F, as recorded in
the RED handoff). No test was repaired in this phase: F flagged none under "Tests that look wrong", and T found none.

```
cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   -> exit 0
  pane_verbs:          275 passed; 0 failed   (RED: 266 passed; 9 failed)
  profile_verbs:        45 passed; 0 failed   (RED: 44 passed; 1 failed)
  profile_diff_test:    11 passed; 0 failed   (RED: 8 passed; 3 failed)
```

Named results, all `ok`: `launch::agent::{the_text_line_names_the_agent_and_is_unchanged_without_one,
a_key_that_reads_as_the_empty_value_is_quoted, data_carries_the_recorded_agent_after_spec_only, spec_only_reports_no_agent,
the_help_of_both_verbs_names_the_agent, a_launch_the_server_refuses_records_nothing,
a_relaunch_the_server_refuses_leaves_the_record_unchanged}`, `launch::launch_with_agent_stores_the_key_in_the_record`,
`relaunch::relaunch_{without,with}_agent_..._stored_key`, `launch::profiles::launch_with_profile_adds_the_spec_and_bumps_once`,
`show::show_lists_the_agent_and_reports_an_agent_difference`, `spec_field_values_serialize_like_the_spec`,
`diff_reports_an_agent_change`, `an_absent_agent_prints_default_and_a_key_named_none_prints_none`.

**The tests pin behavior: removal spot-checks.** Each mutation was applied to F's production code, the suite was run,
and the file was restored with `git checkout` (the tree was clean afterwards).

| Mutation | Result |
|---|---|
| M1: drop the `, agent KEY` clause from `Report::text` | `launch::agent`: 2 failed (the text line, the `-` quoting) |
| M2: `Report.agent` always `None` (not read from the record) | `pane_verbs`: 7 failed (the two text tests, which read the same field, the JSON key test, and the four extended launch, relaunch and profile tests) |
| M3: `FieldValue::Key(None)` prints `none` instead of `(default)` | `profile_diff_test`: 1 failed; `profile_verbs`: 1 failed |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0, no warnings | PASS |
| Workspace tests (the configured `test.unit.command`) | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0, 0 failed, 25 ignored (opt-in) | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | exit 0 | exit 0 | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (warn-only lines are pre-existing files over 600) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Format | `rustfmt --edition 2021 --check` on the 4 changed `.rs` files and the 2 new or changed test files most touched | clean | clean | PASS |

The whole log (clippy, workspace, docs and canary runs) has 150 `test result: ok` lines, 2133 passed, 0 failed; it
includes the two separate reruns of `docs_cli_test` and `wire_selftest`, which F's 148/2127 count does not.

**Cross-check against F's handoff.** Every command F reported was re-run and agrees: `profile_diff_test` 11/0,
`pane_verbs` 275/0, `profile_verbs` 45/0, clippy clean, lint and changelog-check pass, workspace test exit 0. No
discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each acceptance criterion | Mapped below, each to a passing test | PASS |
| Test quality (`test-quality.md` §7) | Each test names a behavior; the mutations above show they fail for the right reason; existing tests were extended, not copied (A warn 5); the refusal table is one table per verb, not one test per code | PASS |
| Proportion | 7 tests in the new module, 2 new unit tests, 1 new `show` test, 5 extended; no redundant test found to prune | PASS |
| Type safety | `Report.agent: Option<AgentKey>` is the record's own type; `FieldValue::Key(Option<String>)` per the brief; no casts, no `#[allow]` added | PASS |
| Error paths | `verify_serving` refusal on launch (stand-in open code, fake `agent-mismatch`, with `--profile`) and relaunch (both codes): exit 3, code and message plus the reconcile step once, no `CreateSession`, no `CasPut`, server killed, record/profile unchanged or compensated | PASS |
| Data integrity | `data.agent == data.pane.opencode_agent` asserted; `--spec-only` gives `null` while P's spec holds the key; a refused relaunch leaves the record equal to before | PASS |
| API contract | JSON `data` keys in order `verb, pane, profile, spec_only, agent`, read on raw stdout; `schema_version` stays 1 (additive, ADR-0021:693, evidence) | PASS |
| Security | `AgentKey` guard is #700's and unchanged; the key prints through `text_value` (no control sequence reaches the terminal); no secret in any asserted output | PASS |
| Migration safety | No persisted or wire format changes: neither `SpecField` nor `FieldValue` derives `Deserialize` | PASS (N/A) |
| Golden files / protocol | No protocol-visible change; no golden blessed | PASS (N/A) |
| File sizes under 900 | `launch.rs` 748, `agent.rs` 299, `relaunch.rs` 633, `profiles.rs` 212, `show.rs` 365, `profile_diff_test.rs` 400 | PASS |
| Browser / Playwright | Not applicable in this repo | N/A |

**Evidence appendix.** F's 18 entries cover the record, serde, `text_value`, `profile show` and engine facts. T appended
two entries under "T (Phase 7 verify)" in `docs/handoffs/644/evidence.md` for unchanged test-kit facts the refusal table
relies on: the fake `verify_serving`'s `agent-mismatch` message (`holler-pane-testkit/src/harness.rs:332-344`) and
`fail_next` (`holler-pane-testkit/src/fault.rs:72-77`), each excerpt copied from source by T.

## Acceptance criteria status

| # | Criterion | Status | Backing test |
|---|---|---|---|
| 1 | Output, text and JSON | PASS | `launch::agent::the_text_line_names_the_agent_and_is_unchanged_without_one`, `a_key_that_reads_as_the_empty_value_is_quoted`, `data_carries_the_recorded_agent_after_spec_only`, `spec_only_reports_no_agent` |
| 2 | Launch records the agent | PASS | `launch::launch_with_agent_stores_the_key_in_the_record` (extended); `ServeSpec` pinned by #802's `launch::serve` tests |
| 3 | Relaunch with and without `--agent` | PASS | `relaunch::relaunch_without_agent_keeps_the_stored_key`, `relaunch_with_agent_replaces_the_stored_key` (extended) |
| 4 | An undefined key is refused, nothing recorded | PASS | `launch::agent::a_launch_the_server_refuses_records_nothing`, `a_relaunch_the_server_refuses_leaves_the_record_unchanged` |
| 5 | `--agent` with `--profile P` | PASS | `launch::profiles::launch_with_profile_adds_the_spec_and_bumps_once` (extended); failure half is the `--profile` row of the launch refusal table (log `[1, 2, 3]`) |
| 6 | The field table | PASS | `spec_field_values_serialize_like_the_spec` (16 `PATHS`), `diff_reports_an_agent_change`, `an_absent_agent_prints_default_and_a_key_named_none_prints_none` |
| 7 | `profile show` sees it | PASS | `show::show_lists_the_agent_and_reports_an_agent_difference` |
| Scope 4 | Help text | PASS | `launch::agent::the_help_of_both_verbs_names_the_agent` |
| 8 | Docs | PASS (reviewed) | ADR-0021 (a) section 8, (b) section 9 row, (c) deferred bullet, (d) items 15-20, (e) section 3 all present in F's diff; `CHANGELOG.md` Unreleased entry links #644; `docs_cli_test` passes |

## Blocking issues

None.

## Advisory notes

- **F's Deviation 1 (`docs/adr/ADR-0003.md`, outside the brief's Blast radius).** One token, `--agent KEY`, in the SPEC
  FLAGS bullet, as A's warn 1 recommended. Not a test concern; O decides whether it stays (it is accurate and
  `docs_cli_test` passes with it).
- **The `(pid P)` clause in `Report::text` can no longer print** since #802 records no pid (F's known issue). The text
  tests do not depend on it. Removing it is O's call.
- **Merge order with #664.** #664 also edits ADR-0021, ADR-0003 and `CHANGELOG.md` in other hunks; whichever merges
  second keeps both texts. #664's `profile apply` report will now see the 16th field.
- Bare `cargo fmt --all` in this worktree reformats ~180 unrelated files (no rustfmt config); only per-file `rustfmt`
  was used.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
