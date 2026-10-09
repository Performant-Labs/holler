# Handoff-T-green: Phase 7 (Workflow verify; Phase 6 in the role doc) - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites

**Date:** 2026-10-09
**Branch:** issue-684-implementation
**Issue:** #684
**Handoff-F reviewed:** docs/handoffs/684/handoff-F.md
**Handoff-T-red:** docs/handoffs/684/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-pane-testkit`: every binary passes. New: `fake_harness_test` 19, `fake_host_test` 12,
`harness_conformance_test` 13, `host_conformance_test` 12 (56 total). Slice a unchanged: `fake_pane_store_test` 22,
`pane_store_conformance_test` 10. No test file was edited in this phase ("Tests that look wrong" in handoff-F is "None").

Spot-check that the tests pin behavior (each mutation applied to F's production code, suite re-run, then reverted with
`git checkout`; the tree is clean afterwards):

| Mutation | Result |
|---|---|
| `harness.rs` `reach`: frozen server answers `Ok` instead of `timeout` | `a_frozen_server_answers_health_false_and_times_out_its_calls` FAILED |
| `host.rs` `stop_owned`: no longer clears the pids | `a_fresh_session_has_no_process_and_survives_stop_owned`, `pids_are_distinct_and_never_reused` FAILED |
| `harness.rs` `reach`: killed server answers `Ok` | `a_killed_server_answers_health_false_and_is_unavailable` FAILED |

## Tier 1 results

| Command | Expected | Actual | |
|---|---|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| `bash scripts/lint.sh` | exit 0 | exit 0 (600-line warnings are other crates' existing files) | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo machete` | no unused deps | none | PASS |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's command, incl. `docs_cli_test`, `wire_selftest`) | exit 0 | 1164 passed, 0 failed, 5 ignored (identical to F's report) | PASS |
| `cargo tree -p holler-pane-testkit -e normal --prefix none \| grep -E '^holler-(cli\|hub\|adapter)'` | no output | no output | PASS |
| `grep -c 'ASSUMPTION (#642 to confirm)' crates/holler-pane-testkit/src/harness.rs` | 3 | 3 | PASS |

Cross-check of F's results: no discrepancy.

## Tier 2 results

- **Coverage per acceptance criterion:** PASS, see below.
- **Test quality:** PASS. Each test names a behavior and runs at the one tier the crate exposes (integration against the
  public API, as slice a). The conformance-suite tests and the fake tests are not duplicates: the former drive the suite
  against the fake and against broken wrappers, the latter pin the fake's own behavior. No redundant test found.
- **Type safety / error handling:** PASS. Clippy clean; no `unwrap`/`expect`/`panic` in the four src files outside `text`
  doc fences; error paths (frozen, killed, unknown id, no TUI, empty argv, missing session) each have a test.
- **Data integrity / contract:** PASS. Pids never reused, session ids unique, the closed code strings used unchanged.
- **Security:** N/A. Test-kit crate, no input boundary, no secrets handled.
- **Migration / protocol:** N/A. No schema, wire or golden change; `docs/protocol/v2.md` untouched, none needed.
- **Repo rules:** PASS. All files under 900 lines (largest: `fake_harness_test.rs` 576, `harness.rs` 502); no `#[allow]` without
  a `// #NNN` link; no sleeps (the only delay is the fault switch's injected one, slice a's); no browser surface.
- **Evidence appendix:** `evidence.md` checked against source (`fault.rs:93-107`, `119-125`, `conformance/mod.rs:47-52`);
  the excerpts are verbatim. Nothing to append.

## Acceptance criteria status

All criteria in the brief are backed by the Phase-4 suite, now GREEN: AC 1-2 by `host_conformance_test.rs`
(fake passes; each broken wrapper fails its named case); the harness equivalents by `harness_conformance_test.rs`;
the fakes' behavior by `fake_host_test.rs` and `fake_harness_test.rs`; AC 7 (3 ASSUMPTION comments) and AC 8
(dependency rule) by the commands above; the changelog criterion by `changelog-check.sh`. PASS for each.

## Blocking issues

None.

## Advisory notes

- The ASSUMPTION comments are wrapped over two lines rather than literally "one line" (F's hedge). The AC `grep -c` is
  satisfied; S may decide whether the literal reading matters.
- Two six-line `holds` helpers (pids, session ids) are duplicated across the suites; F flagged them for the W-5 follow-up.
