# Handoff-T-green: #683 the pane test kit, slice d (`FakeHerdr`, `FakeProber`, `HerdrPort` conformance suite)

**Date:** 2026-10-09
**Branch:** issue-683-implementation
**Issue:** #683
**Handoff-F reviewed:** `docs/handoffs/683/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/683/handoff-T-red.md`

## GREEN confirmation
`cargo test -p holler-pane-testkit`: `fake_herdr_test` 22 passed, `fake_prober_test` 6, `herdr_conformance_test` 15
(both placements, the case list, a fresh fixture per case, the unbroken wrapper and all 10 mutants), and slice a's
`fake_pane_store_test` 22 and `pane_store_conformance_test` 10 unchanged. Eight further repeats of the crate's suite: no failure.

Spot-check that the tests pin behavior (a production line mutated in `src/herdr.rs`, tests run, file restored by
`git checkout`; the tree is clean):
- `check_range` upper bound `..=` to `..`: eight `fake_herdr_test` tests fail.
- split-only check disabled: `split_only_mode_starts_an_empty_workspace_at_r1c1` and `..._refuses_an_absolute_placement` fail.
- base-36 digits lower-cased: `ids_are_minted_per_workspace_in_base_36_and_never_reused` fails.
- `last_lines` skip set to 0: `read_returns_the_last_lines_of_the_screen` fails.
- (A fifth mutant, a self-assignment of the counter, does not compile under clippy's `-D`-level lint; not run.)

The 10 mutants each fail their named case(s) in `herdr_conformance_test` (the assertion in `assert_suite_fails_on`
passes). No test needed repair; F flagged none.

## Tier 1 results
| Check | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace --no-fail-fast` | PASS: 1152 passed, 0 failed, 5 ignored (matches F) |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `cargo machete` | PASS |
| `bash scripts/lint.sh` | PASS (600-line warnings are other crates' files only) |
| `bash scripts/changelog-check.sh` | PASS |
| `bash scripts/test-hooks.sh` | PASS |
| `rustfmt --edition 2021 --check` on the 6 changed `.rs` files | PASS |

Discrepancy with F, noted: on my first `cargo test --workspace` (fail-fast) and on three direct reruns,
`holler-cli` `body_run_test::fresh_hello_and_presence_on_every_reconnect` failed with "hub did not report listening
within 10s: Disconnected". It fails identically on the main checkout (`4c890d4`, no #683 code), and passed in the
following `--no-fail-fast` run. The diff touches no `holler-cli` code; the test starts a hub on a fixed explicit port
(`start_hub_at`), and this host has many listeners, so it is environmental (a port collision on this machine). Not
caused by #683, not blocking.

## Tier 2 results
- Coverage per criterion: PASS (see below).
- Test quality: PASS. Each test names one behavior; the 10 mutants are the proportionate check on the suite, one per
  break, each against a named case; the three extra tests in `fake_herdr_test` (base-36 carry, split-only occupant,
  print to an unknown pane) pin distinct rules and do not duplicate another. No redundant test found. Timing: the one
  slow-call test asserts a lower bound only; no fixed sleeps are used to wait for readiness.
- Error paths tested: PASS (`grid-out-of-range`, `grid-unreachable`, `unavailable`, `usage`, `pane-not-found`,
  `herdr-version-unsupported`, `Timeout` on every method, the one-shot failure creating nothing).
- Type safety / panics: PASS. `src/herdr.rs` and `src/prober.rs` and `conformance/herdr.rs` have no `allow(`; the
  three test files each carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #683`.
- Secrets in logs/errors: N/A (no credential in this kit).
- File sizes: PASS (largest 538 lines, under the 600 warning and the 900 limit).
- Protocol/goldens/`[[test]]`: N/A (test kit only; `holler-pane-testkit` has no `autotests = false`).
- AC 5 blast radius: PASS. `git diff --name-only origin/main...HEAD` is the three `src` files, three new tests,
  `CHANGELOG.md`, `docs/adr/ADR-0021.md` and `docs/handoffs/683*`. No `Cargo.toml` or `Cargo.lock`.
- Evidence appendix: PASS. I mechanically checked all 14 entries in `docs/handoffs/683/evidence.md`: every excerpt
  line is verbatim in its source file. None needed adding.

## Acceptance criteria status
1. Fake passes its suite in both placements, case list, fresh fixture: PASS (`herdr_conformance_test`, 4 tests).
2. Each mutant fails on its named case: PASS (10 mutant tests plus the unbroken wrapper).
3. The fake's own mechanisms: PASS (`fake_herdr_test`, 22 tests, every AC 3 name present).
4. The fake probe: PASS (`fake_prober_test`, 5 named tests plus the unscripted-message test).
5. No other crate changes; slice a's tests pass unchanged: PASS.
6. CHANGELOG entry under Unreleased / Enhancements, after the #676 entry, links #683: PASS (`changelog-check` ok).
7. Guards: PASS (table above).
8. ADR-0021: PASS. Section 10's bullet keeps its text and adds the workspace-extent condition (plus one clause that
   hands the extent question to #640); section 9's reason names both sources; nothing else changed (4 added, 2
   removed lines).

## Blocking issues
None.

## Advisory notes
- The section 10 ADR bullet carries one clause beyond the brief's example ("How the adapter learns a workspace's
  extent is #640's."). Harmless, consistent with the ASSUMPTION comments; S may want to confirm it is wanted.
- `body_run_test::fresh_hello_and_presence_on_every_reconnect` fails on this host on main too; worth an issue if it
  recurs for the operator.
