# Handoff-T-green: #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

**Date:** 2026-10-09
**Branch:** issue-676-implementation
**Issue:** #676
**Handoff-F reviewed:** docs/handoffs/676/handoff-F.md
**Handoff-T-red:** docs/handoffs/676/handoff-T-red.md

## GREEN confirmation
- `cargo test --workspace`: exit 0, 1048 passed, 0 failed, 5 ignored (the same total F reported).
- `cargo test -p holler-pane --test error_class_test`: 4/4. `cargo test -p holler-cli --test pane_verbs`: 64/64.
- Non-vacuity, by temporary mutation of production code (each reverted with `git checkout`; `git status` then showed only the
  test-file doc repair below):
  - `output.rs::exit_code` replaced by "usage -> 2, else 1": `pane_verbs` goes 6 failed / 58 passed (the two `pane-not-found`
    tests, `emit_stream_exits_3_on_a_refusal_item`, `a_refusal_exits_3_...`, `every_closed_code_exits_by_its_class_...`,
    `emit_exits_2_..._and_3_for_a_refusal`).
  - `PaneInOtherProfile` moved from the Refusal arm to the Failure arm in `class_of`: `every_closed_code_has_the_decided_class`
    fails (1 failed / 3 passed).

## Test repair (T-owned)
F flagged one item in a test file, and no test was wrong. `crates/holler-cli/tests/pane_verbs/spec_flags.rs:3-5`, a `//!` doc
line, said "A refusal is exit 1 in the verb". It now says a guard's refusal is exit 3 and one coded `usage` exits 2. No test
code changed; rustfmt-clean (as before); `pane_verbs` still 64/64.

## Tier 1 results
| Check | Result |
|---|---|
| `cargo test --workspace` | PASS (1048 / 0 failed / 5 ignored) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo machete` | PASS |
| `bash scripts/lint.sh` | PASS (exit 0; 600-line warnings only, `error.rs` 710 < 900) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `rustfmt --check --edition 2021` on `error_class_test.rs`, `error.rs`, `output.rs`, `spec_flags.rs` | PASS |
| `grep -rn "refused or failed" crates/ docs/adr/` | PASS (no match) |
| Blast radius (`git diff --name-only origin/main...HEAD`) | PASS (only brief-listed paths; no `main.rs`, manifests, goldens) |

F's other reported commands (`test-hooks.sh`, `cargo doc -D warnings`) were not re-run: nothing they cover changed after F, and
the only edit in this phase is a test-file doc comment.

## Tier 2 results
- **Coverage per criterion:** PASS. AC 1 `error_class_test.rs` (4); AC 2 `output_api.rs` (3 new, 3 updated); AC 3-5 are text,
  verified by grep, `docs_cli_test` and `changelog-check.sh`.
- **Test quality:** PASS. Pure in-memory tests at the cheapest tier, no sleeps or timing. The numbers are pinned twice (through
  `class_of` over `ALL_CODES`, and by literal spot checks that do not call it). `a_malformed_code_is_a_failure` is a regression
  guard only (noted in T-red). No redundant test found.
- **Exhaustiveness:** PASS. `class_of` has one `match` over `PaneCode`, no `_` arm; the 22-row test table is tied to `ALL_CODES`.
- **Error handling / security:** PASS. A malformed code is a Failure, an open code a Refusal; the tests assert no secrets (none are
  involved).
- **Protocol / goldens:** N/A. No wire change; no golden file or `cli-surface` fixture touched.
- **Playwright / UI:** N/A (no UI surface in this repo).

## Acceptance criteria status
1. Classifier exists and is exhaustive: PASS (`error_class_test.rs`).
2. Output module uses it: PASS (`output_api.rs`; `exit_code` is the one-line `class_of(...).exit_code()`).
3. Text residue fixed: PASS (grep clean; `spec_flags.rs` doc repaired this phase).
4. ADR-0021 section 9 table, ADR 0003 sentence: present (22 closed-code rows in the diff); `docs_cli_test` passes. Content is S's audit.
5. CHANGELOG entry: PASS (`changelog-check.sh`).
6. Guards: PASS (table above).
7. Blast radius: PASS.

## Blocking issues
None.

## Advisory notes
- `main.rs:253-255` still says a wiring error is "(exit 1, ...)". Accurate today (`Wiring::connect` cannot fail); #649 must
  update it. The note must reach #649 through the PR body, since this handoff is deleted at the end of the run.
- The 17 stub verb files still say "refuse" about a `not-implemented` error (exit 1); each verb's story replaces its stub.
- Close calls for the operator (three not-found codes, `probe-failed`, `herdr-version-unsupported`, `not-implemented`,
  `profile-drift`) are recorded once in ADR-0021 section 9; changing one is a one-arm, one-test-row edit.
