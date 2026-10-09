# Handoff-T-green: Phase 7 - #638 slice a: the fault switch, the fake `PaneStore` and its conformance suite

**Date:** 2026-10-09
**Branch:** issue-638-implementation
**Issue:** #638 (slice a; "Part of #638")
**Handoff-F reviewed:** docs/handoffs/638/handoff-F.md
**Handoff-T-red:** docs/handoffs/638/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-pane-testkit`: `fake_pane_store_test` 22 passed, `pane_store_conformance_test` 10 passed, 0 failed. No test needed repair
(F flagged none; none of mine changed). Repeated 20 times in a loop: 0 failures (the timing tests `a_slow_call_takes_at_least_the_delay`,
`the_idle_wait_wakes_on_a_write` and `the_default_idle_wait_is_zero` assert bounds, not durations).

Behavior-removed spot checks on production code (each reverted; tree clean afterwards):

| Break | Test that failed |
|---|---|
| `check_since` never answers `usage` | `a_watch_ahead_of_the_head_is_usage` |
| the fault switch no longer sleeps for the delay | `a_slow_call_takes_at_least_the_delay` |
| a queued one-shot error is not consumed | `fail_next_is_one_shot_and_per_op`, `queued_one_shot_errors_come_out_in_order` |

F's six suite mutants (`NoCas`, `KeepsSubmittedGeneration`, `GenerationBeforeExistence`, `RepeatsOnResume`, `EndsStreamWhenIdle`,
`LetsPaneChangeProfile`) are the mutant tests in `pane_store_conformance_test.rs`; all pass, each on its named case, and the unbroken wrapper passes.

## Tier 1 results

| Command | Result |
|---|---|
| `cargo build --workspace` | exit 0, PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, PASS |
| `cargo test --workspace` | 1073 passed, 0 failed (matches F); `testkit_links` in holler-hub and holler-cli ok, PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| `cargo machete` | no unused dependencies, PASS |
| `bash scripts/lint.sh` | exit 0 (600-line warnings only for pre-existing files), PASS |
| `bash scripts/changelog-check.sh` | ok, PASS |
| `bash scripts/test-hooks.sh` | exit 0, PASS |
| `rustfmt --check --edition 2021` on the crate's src and tests | exit 0, PASS |
| `cargo tree -p holler-pane-testkit -e normal --prefix none \| grep -E '^holler-(cli\|hub\|adapter)'` | prints nothing, PASS |

Results match F's handoff; no discrepancy.

## Tier 2 results

- **Coverage per criterion:** every behavioral AC (3, 4, 5) has a named test; AC 1, 2, 6, 7, 8 are structural and checked below. PASS.
- **Test quality:** each test names one behavior; the suite is proportionate (32 tests, two files, 426 and 311 lines, no duplicate of another
  test; `MemPaneStore` in holler-pane is untouched). The mutants share one unbroken-wrapper check. PASS.
- **Timing:** invariants and lower/upper bounds only, no fixed sleeps used for readiness; 20 repeats clean. PASS.
- **Lint rules:** all new test files under 900 lines (largest source file 518); every `#[allow(...)]` carries `// #638`. PASS.
- **Layout (AC 2):** 12 modules, `mod feed;` crate-private, no `pub use`; stubs are doc-only. PASS.
- **Blast radius (AC 6):** `git diff --name-only 939d79c HEAD` outside the testkit crate and `docs/handoffs`: `CHANGELOG.md`, `Cargo.lock` only. PASS.
- **Secrets in logs/errors:** not applicable (no secrets handled by the fake). N/A.
- **Protocol change / goldens:** none. N/A.
- **Evidence appendix:** `evidence.md` has seven entries with verbatim excerpts; the two I spot-checked (`next_generation`, `delete` contract) match
  source. PASS.
- **No browser/Playwright surface in this repo.** N/A.

## Acceptance criteria status

1. Manifest: PASS (machete, cargo tree).
2. Layout: PASS (read against `lib.rs` and `conformance/mod.rs`).
3. Fake passes its suite: PASS (`the_fake_passes_the_pane_store_conformance_suite`, `the_suite_runs_the_documented_cases`, `the_guard_lives_for_the_case`).
4. Mutation check: PASS (six mutant tests plus the unbroken wrapper).
5. Faults injected and observed: PASS (all `fake_pane_store_test.rs` tests).
6. No other crate changes: PASS (only `CHANGELOG.md` and `Cargo.lock` outside the crate; `testkit_links` unchanged and green).
7. CHANGELOG: PASS with a documented deviation. The entry sits before the #670 entry rather than after it, because #639 merged to main at that spot;
   F's three-way `git merge-file` reports 0 conflicts. Acceptable.
8. Guards: PASS (table above).

## Blocking issues

None.

## Advisory notes

- The hub's registry fails suite case 19 (`pane-in-other-profile`) until #661 merges. It is documented in `run_pane_store_conformance`'s doc and is
  not this slice's defect.
- Failure details print whole `Pane`s with `{:?}`, which is long but non-empty.
- Open items outside this slice: A's W-1 and W-2 (widen the blast radius of #681 to #684, add `serde_json` to #682).
