# Handoff-A-dup: Phase 7 (role doc; 8 in the script) - #676 pane and profile verbs exit 3 on a refusal, 1 on a failure  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-676-implementation
**Diff base:** 55dba00 (merge-base with origin/main)   **Diff head:** 38eac7a
**Reuse map:** docs/handoffs/676-brief.md §Files "Reuse map"
**Verdict:** PASS

## Summary

PASS. F extended the object the Reuse map named and built no parallel path. `ErrorClass`, `ErrorClass::exit_code` and
`class_of` are pure additions to `holler-pane/src/error.rs`: the diff removes no line there. `class_of` finds a code with
`PaneCode::parse` and `is_valid_code`, so there is no second code list and no second validator. `output.rs::exit_code` is
now the one line `class_of(error.code.as_str()).exit_code()`. The new tests reuse `error_body`, `with_sink` and
`one_envelope`, and they iterate `ALL_CODES`.

All four findings are `warn`. W-1 and W-2 are two sides of one gap. The classification is also written down in the ADR
table and in four CLI tests, and the ADR's instruction for moving a close call names neither. One sentence at
`ADR-0021.md:397` fixes that instruction. W-3 and W-4 are carried from Phase 3 and need no change in #676.

## Phase 3 checklist, verified on the diff

- **One exhaustive `match`.** `class_of` has exactly one `match` over `PaneCode` (`error.rs:274-302`) and no `_` arm. Its
  22 variants split into 1 usage, 14 refusal and 7 failure, which matches the brief's table and the ADR's table row for
  row. The open-or-malformed split is a `let ... else` before the match (`:267-273`), not a second match.
- **Lookup reuse.** The lookup uses `PaneCode::parse` (`:126`) and `is_valid_code` (`:163`). `class_of` does not copy
  `is_closed_code`/`str_eq` into a const lookup just to be `const`, which is the right trade.
- **The CLI keeps no table.** `output.rs:338-340` has no table and no comparison with `usage`. `emit_text` (`:283`) and
  `emit_json` (`:292`) still call it. `emit_stream`, `emit_error` and `emit_usage_error` are unchanged and reach it
  through `emit`.
- **Nothing outside the plan.** There is no new module and no `lib.rs` re-export. `main.rs`, every `Cargo.toml` and
  `Cargo.lock` are unchanged (`git diff --quiet`). `error.rs` loses no line, so the frozen rule (`lib.rs:31`) holds.

## Duplication sweep

- `ErrorClass` and `class_of` are defined once and used only by `output.rs` and the tests. No other code-to-class or
  code-to-exit mapping exists for pane codes:
  - `holler-proto`'s refusals are wire types with no pane-code mapping.
  - `hold_cmd.rs:19,141` (4, 5) and `prompt_target.rs:58-61` (2, 1) are legacy, verb-local constants that predate this
    change (see W-4).
  - The test kit (`holler-pane-testkit/src/lib.rs`) is still an empty skeleton, so #638 calls `class_of` and has
    nothing to fold in.
- The diff adds three production items (`ErrorClass`, `ErrorClass::exit_code`, `class_of`), plus test functions,
  `DECIDED` and `QUOTA`. It adds no test helper. The stack's Phase 7 candidates are untouched: the `token.rs`
  operations, `Lockout`, `Roster`, `log(Severity, ...)`, `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`.
- `DECIDED` (`error_class_test.rs:12`) and `CLOSED` (`error_test.rs:19`) are both literal 22-code lists, but they are
  different objects: one maps code to class, the other is the epic's closed set. Both are pinned to `ALL_CODES`
  (`error_class_test.rs:44`, `error_test.rs:81-84`), so they cannot drift apart silently. The brief justified the
  separate file, and Phase 3 accepted it. This is not duplication.
- Sizes: `error.rs` 710, `output.rs` 467, `output_api.rs` 470 and `error_class_test.rs` 80, all under the ~800 line.
  `process::exit` still appears only in `main.rs`.
- `origin/main` has moved two commits ahead (the #635 and #636 research spikes). They touch only `docs/research/` and
  `scripts/spikes/` and overlap nothing in this branch.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:370-395` (table), `:397` | The 24-row table is a third written form of the classification, after `class_of` and `DECIDED`, and the only one no test checks. A closed-code table in `docs/` usually has a conformance test: `holler-proto/tests/docs_errors_test.rs:1-6` ("A code added to `Code::ALL` without a docs row fails here"), `hold_cli_test.rs:248-255` and `grant_cli_test.rs:212-218` (v2.md §10 exit numbers), and `pane_verbs/process/docs_rows.rs` (ADR 0003 rows). The pattern does not cover every ADR table, so this is a warn. The ADR's own instruction for moving a close call (`:397`, "one arm of `class_of` and one row of its test") leaves out this table, so anyone who follows it leaves the ADR stale. | Amend `:397` to name this table's row (see Notes for O). Optionally, as a T-owned addition, write a test shaped like `docs_errors_test.rs`: read `docs/adr/ADR-0021.md`, slice §9, and assert that every `ALL_CODES` entry has a row whose class and exit agree with `class_of(code)` and `exit_code()`. |
| 2 | warn | `crates/holler-cli/tests/pane_verbs/output_api.rs:141,145`, `:156,160`, `:287,291` (`pane-not-found`); `:193,208,212` (`probe-failed`) | Four routing tests (err versus out, envelope shape, stream end) also assert the close-call classes as literal `3`s. Moving the three not-found codes, the first close call `:397` lists, breaks three of these tests. Moving `probe-failed` breaks the fourth and its doc line. So the cost that brief decision 1 (`676-brief.md:246`) and the ADR (`:397`) both state, "one arm and one test row", is understated. AC 2 required three of the four (`:144`, `:159` and `:211` at base), so the claim is the brief's, not drift by T or F. `a_refusal_exits_3_and_a_failure_exits_1_in_both_formats` (`:256-279`) already does it the cleaner way: it pins numbers with settled codes only. | Make the same `:397` sentence name "any `output_api.rs` test that uses the code". Optionally, as a T-owned follow-up, use a settled refusal such as `pane-in-other-profile` as those tests' example code. A close-call move is then one arm, one `DECIDED` row and one ADR row. |
| 3 | warn (carried from Phase 3 W-2) | `crates/holler-cli/src/main.rs:253-255` | The `run_with_stdio` doc still says a wiring error is reported "(exit 1, with the error's code)". After #676 such an error exits by its class. The line is accurate today only because `Wiring::connect` always returns `Ok` (`pane/wiring.rs:31-32`). The brief kept `main.rs` out (decision 7, AC 7), and F followed the brief. | No change in #676. The PR body carries the note to #649: when `connect` can fail, the doc becomes "(with the error's code, and the exit code of its class)". |
| 4 | warn (carried from Phase 3 W-5) | `crates/holler-cli/src/prompt_target.rs:58-61` | The legacy `--pane`/`--profile` stop keeps its own `USAGE_EXIT = 2` and `REFUSED_EXIT = 1`, and it calls exit 1 a "refusal". Under ADR-0021 §9 a refusal exits 3. The numbers agree with `class_of`, because `not-implemented` is a Failure (1). The constants predate this change, and the brief puts legacy exit codes out of scope. | No change in #676. When #646 replaces these stops, it should go through `ErrorBody`/`class_of`, or at least rename `REFUSED_EXIT` to match the wording of §9. |

There is no duplication, and the extension is clean. Rework introduced no drift: T-green changed only the `//!` doc at
`spec_flags.rs:1-5`.

## Notes for F

None. The verdict is PASS.

## Notes for O (non-blocking)

- **One sentence closes W-1 and W-2 at the source.** At `ADR-0021.md:397`, replace "(one arm of `class_of` and one row
  of its test)" with, for example, "(one arm of `class_of`, one row of its test, this table's row, and any
  `output_api.rs` test that uses the code as its example)". The edit is doc-only and in a blast-radius file, so it can go
  in before merge. The conformance test (W-1) and the swap of the tests' example code (W-2) are optional T-owned
  follow-ups.
- **The PR body must carry W-3 to #649 and W-4 to #646.** These handoffs are deleted when the run ends.
