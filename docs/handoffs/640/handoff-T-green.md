# Handoff-T-green: Phase 7 - #640 part 1 of 3, the pure Herdr protocol and grid core

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Issue:** #640 (part 1 of 3; the PR says `Part of #640`)
**Handoff-F reviewed:** docs/handoffs/640/handoff-F.md
**Handoff-T-red:** docs/handoffs/640/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-adapter-herdr --no-fail-fast`: layout_test 10/10, plan_splits_test 19/19, protocol_test 33/33. These
are the same counts F reported. F's "Tests that look wrong (for T)" section says "None", so no test was repaired.

Mutation spot-checks on production code (each reverted with `git checkout`; the tree is clean):

| Mutant | Result |
|--------|--------|
| Swap the `down`/`right` directions in `grid_of` (transposition) | layout 8 fail, planner 6 fail. Caught |
| Drop the `+ 1` in `number` (count from 0) | layout 8 fail. Caught |
| `SendText`'s `Debug` prints the text, not its length | protocol 1 fail (AC 16). Caught |
| Ratio denominator `size - n + 2` becomes `+ 1` | planner 7 fail. Caught |
| `EXCERPT_LIMIT` 64 becomes 1000 | **Survives** (see Advisory notes) |

## Tier 1 results

| Command | Expected | Actual | |
|---------|----------|--------|-|
| `cargo test -p holler-adapter-herdr --no-fail-fast` | green | 62 passed, 0 failed | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` (adapter re-checked after `touch`) | exit 0 | exit 0 | PASS |
| `cargo test --workspace --no-fail-fast` | green | 118 targets, 1317 passed, 0 failed, 5 ignored (existing) | PASS |
| `bash scripts/lint.sh` | exit 0 | exit 0; only warnings, adapter's is `protocol_test.rs` at 678 lines (limit 900) | PASS |
| `bash scripts/changelog-check.sh` | ok | ok | PASS |
| `cargo machete` | none unused | none unused | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | green | 3 passed | PASS |
| `cargo test -p holler-cli --test wire_selftest` | green | 3 passed | PASS |

## Tier 2 results

- **Test coverage per acceptance criterion:** AC 1-20 each have a named test (`acN_...`). AC 21 is the gates above. AC 22 is
  the CHANGELOG entry, checked by `changelog-check`. PASS.
- **Test quality:** the pure, deterministic tests have no sleeps, no I/O and no timing. Each sits at the integration-test
  layer of a pure crate, the cheapest sufficient tier. The suite is proportionate (62 tests for 3 modules); I found nothing
  to delete or merge. PASS.
- **Timing and flake risk:** none. There is no concurrency or clock. PASS.
- **Type safety and unsafe:** no `unsafe` and no `#[allow]` in `src/`. The `#![allow(...)]` in the test file carries `// #640`. PASS.
- **Error handling:** the garbled-reply, pane-not-found, version-gate and refusal paths are tested. PASS.
- **Secrets:** AC 16 asserts that typed text and Herdr's `message` never appear in a message, in `Debug`, or in `Display`. The
  `SendText` mutant above confirms the test bites. PASS.
- **Protocol-visible change:** this crate adds no holler wire message, so no golden files or `docs/protocol/v2.md` change
  are needed. N/A.
- **File sizes:** all under 900 (src: layout 222, plan 268, protocol 595, lib 39). PASS.
- **Evidence appendix:** `evidence.md` has 12 entries from F, in the required format. I spot-checked three against source:
  the `excerpt` visibility in `holler-pane`, the `class_of` entry and the spike's recipe. Each excerpt is verbatim. PASS.
- **Browser, Playwright and visual surface:** none in this repo. N/A.

## Acceptance criteria status

AC 1-20: PASS, each backed by its `acN_...` test in `layout_test.rs`, `plan_splits_test.rs` or `protocol_test.rs`.
AC 21 (gates): PASS. AC 22 (CHANGELOG): PASS.

## Blocking issues

None.

## Advisory notes

- The 64-character cut in the private `excerpt` helper (`protocol.rs`) is not pinned by any test. The mutant that raised the
  limit to 1000 survived. This is not an acceptance criterion: AC 16 pins one line and no leak, which hold. A has
  already been asked to judge the duplicated helper. If a later part wants the bound pinned, it needs one test that feeds a
  long Herdr code or label and asserts the message length. I did not add one, because the bound is not in the brief.
- `protocol_test.rs` is 678 lines, which draws a `lint.sh` warning at 600 but is well below the 900 failure line.
