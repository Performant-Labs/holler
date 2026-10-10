# Handoff-T-green: Phase 6 - Verify GREEN + Tier 2

**Date:** 2026-10-10
**Branch:** `issue-0700-implementation` (run worktree `<run-worktree>`)
**Issue:** #700
**Handoff-F reviewed:** `docs/handoffs/0700-implementation/handoff-F.md` (commit 34ca660)
**Handoff-T-red:** `docs/handoffs/0700-implementation/handoff-T-red.md` (commit e49c5aa)

## GREEN confirmation

**Command (the configured suite):**
`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`

**Result: every test binary `ok`, exit 0** (the one filtered-out test is the configured skip).
Key RED targets, now passing: `pane_verbs` 219/219, `argv_env_test` 8/8, `records_test` 14/14,
`profile_snapshot_test` 5/5, `cli_surface_test` 3/3.

**Tests pin behavior, not implementation:** the RED→GREEN transition is the evidence — at e49c5aa
the suite failed 3 targets on the missing symbols (`AgentKey`, `opencode_agent`) and 2 at runtime
(`unknown field opencode_agent`, `unexpected argument '--agent'`); F's implementation against that
contract turned exactly those green. (Mutation testing barred for this run by the operator, so the
spot-check rides the recorded RED.)

**Two environmental flakes met on the way (NOT F's diff, documented here for the record):**

1. First full run: 6 failures in `interrupt_test`. All 6 pass 11/11 in isolation and in the final
   full run — websocket-timing sensitivity under full-workspace parallel load.
2. Second full run: `body_run_test::fresh_hello_and_presence_on_every_reconnect` failed
   `hub did not report listening within 10s: Disconnected`. Its fixed port (127.0.0.1:41918) was
   in TIME_WAIT from the prior run's teardown — the exact #242/#259 rebind-race class named in the
   test's own doc comment. After the ~60 s TIME_WAIT expired it passed 10/10 alone and in the final
   full run. The machine also carries live sessions and hubs from sibling runs (out of bounds,
   untouched). 34ca660 touches no hub, body or interrupt code path.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Configured suite | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0, all `ok` — PASS |
| Build/compile | (the suite run compiles every target) | no errors, no warnings — PASS |
| F's numbers cross-checked | F: 140 binaries, 1761 passed / 1 failed (the test-literal defect) | matches after my repair: that 1 now passes; no discrepancy — PASS |

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| F touched no tests | `git show 34ca660 --stat` | 12 files: `crates/*/src/**` + `docs/adr` + handoffs only; zero paths under `crates/*/tests/**` — PASS |
| Test repair 1 (F's flag upheld) | verified delimiter counting myself: `r#"""#`'s closing `"#` matches after one `"` of content → serde_json lexes EOF before `Deserialize` runs, so it can never yield `agent-key-invalid`; RED never saw it (E0432 compile-RED) | F's analysis correct; repaired to `r#""""#` (JSON `""`), the intended empty-key decode — PASS |
| Test repair 2 (mine, found by Tier 2) | `cargo fmt --all --check` at base vs HEAD | my two RED lines in `pane_verbs/{launch.rs:124, relaunch.rs:567}` exceeded the width (absent at base); rewrapped to rustfmt's exact form by hand — PASS |
| fmt | `cargo fmt --all --check`, file-set diffed against base `3d95aec` (temp worktree, removed after) | pre-existing workspace-wide diffs (~180 hunks) unchanged; **the set of flagged files at HEAD is byte-identical to base** — zero fmt delta from this run — PASS |
| clippy | `cargo clippy --workspace --all-targets` | exit 0, no warnings, no errors — PASS |
| unsafe | `git grep -n "unsafe" crates/holler-pane/src crates/holler-cli/src` at HEAD and at base | none at either — no new unsafe from 34ca660 — PASS |
| Surface fixture | `grep -c -- '--agent' crates/holler-cli/tests/fixtures/cli-surface.txt` | exactly `1` — PASS |
| ADR | `git show 34ca660 -- docs/adr/ADR-0021.md` | one inserted dated row in §1's Pane table (after `model`), `1 insertion(+)`, no deletions — PASS |

## Acceptance criteria status

| AC | Test backing it | Status |
|---|---|---|
| 1 AgentKey guard: grammar, `agent-key-invalid`, no echo | `argv_env_test::agent_key_accepts_names_and_refuses_malformed_keys` | PASS |
| 2 serde plain string, fail closed on decode | `argv_env_test::agent_key_serde_is_a_plain_string_failing_closed` (after literal repair) | PASS |
| 3 `Pane.opencode_agent` after `model`, omitted when None, old records load | `records_test::pane_opencode_agent_round_trips_set_and_absent` | PASS |
| 4 same on `ProfileSpec` | `records_test::profile_spec_opencode_agent_round_trips_set_and_absent` | PASS |
| 5 launch writes the spec's key (DECISION 1) | `pane_verbs::launch_with_agent_stores_the_key_in_the_record` | PASS |
| 6 relaunch overlay: given replaces, absent keeps | `relaunch_with_agent_replaces_the_stored_key`, `relaunch_without_agent_keeps_the_stored_key` | PASS |
| 7 `--agent` refused by `validate` before anything runs | `guards::an_invalid_agent_key_is_refused_before_anything`, `relaunch_refuses_an_invalid_agent_key_before_anything` | PASS |
| 8 ADR row | spot-check above | PASS |
| 9 existing suite unchanged (all 1762 pass) | configured suite | PASS |
| 10 `--from-current` copies the field | `profile_snapshot_test::snapshot_copies_every_spec_field_from_the_pane_record` | PASS |

## Blocking issues

None.

## Advisory notes

- F's deviation 1 (`holler-pane/src/lib.rs` root re-export) is inside the issue's blast radius and
  was named in T-red's "Ready for F" — fine as implemented.
- F's optional test-side items (`verb_harness/parse.rs` `SPEC_FLAG_SETS`, `docs_rows.rs` static
  flag list) remain undone — declined again by T: A ruled W6 optional, `--agent` is pinned by the
  verb tests and the surface fixture, and widening into those files now would exceed this run's
  test boundary without adding behavior coverage.
- ADR-0021's older deferred bullet (~line 677) still words the `Pane.opencode_agent` recording as
  #644's; the one-row constraint kept it and the new row states the launch record writes it — S
  may inspect (echoing F).
- The `interrupt_test` under-load sensitivity and the fixed-port TIME_WAIT race in
  `body_run_test` are pre-existing flake classes worth a follow-up issue; they can gate a green
  suite on a busy machine.

GREEN: CONFIRMED
