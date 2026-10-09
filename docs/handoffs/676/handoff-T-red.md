# Handoff-T-red: #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

**Date:** 2026-10-09
**Branch:** issue-676-implementation
**Brief / wireframe reviewed:** docs/handoffs/676-brief.md (no wireframe: no UI surface)

## A precondition
Confirmed: A returned PASS on the plan (Phase 3), see handoff-A.md (0 blocks, 5 warns; none changes the tests).

## Tests authored
New file `crates/holler-pane/tests/error_class_test.rs` (unit-level on the types crate, the cheapest tier; holler-pane has
default autotests, so no manifest entry):
- `every_closed_code_has_the_decided_class` (AC 1): the brief's 22-row table written out literally; asserts its key set
  equals `ALL_CODES` and `class_of(code)` equals the row's class.
- `an_open_code_is_a_refusal` (AC 1): `class_of("quota-exceeded")` and the code of a `PaneError::Refused` built from
  `RefusalCode::from_static` are `Refusal`.
- `a_malformed_code_is_a_failure` (AC 1): `"Not A Code"` and `""` are `Failure`. Regression guard: it passes against a
  skeleton that returns `Failure` for everything, and fails only if malformed codes ever fall into the open-code arm.
- `exit_codes_by_class` (AC 1): Usage 2, Refusal 3, Failure 1.

Extended `crates/holler-cli/tests/pane_verbs/output_api.rs` (integration over the real `emit`/`emit_stream`, the cheapest
tier that reaches the output module; 470 lines, under 900; rustfmt-clean, as it was before):
- `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats` (AC 2): for every `ALL_CODES` entry and both
  formats, the exit equals `class_of(code).exit_code()`, text equals JSON, and the JSON envelope has `ok == false`,
  `data == null`, `error.code == code`, nothing on err.
- `a_refusal_exits_3_and_a_failure_exits_1_in_both_formats` (AC 2): literal spot checks, independent of `class_of`
  (`pane-in-other-profile`, `profile-secret-refused`, `command-not-argv`, `grid-ambiguous`, open `quota-exceeded` -> 3;
  `timeout`, `generation-conflict`, `unavailable` -> 1; `usage` -> 2).
- `emit_stream_exits_3_on_a_refusal_item` (AC 2): `[Ok, Err(pane-not-found)]` returns 3, keeps the earlier line.
- Updated: `emit_error_in_text_mode_...` and `emit_error_in_json_mode_...` now assert 3 for `pane-not-found`;
  `emit_exits_2_for_a_usage_coded_error_in_both_formats_and_1_for_any_other` renamed to `..._and_3_for_a_refusal` and
  asserts `probe-failed` exits 3; module header lines 6-8 now state 0/1/2/3 and point at `class_of`.
- Unchanged and still expected to pass: the `unavailable` (exit 1) tests, failed-write tests, every stub test, legacy-verb tests.

Not authored (out of scope per the brief): process-level refusal tests (no verb returns a refusal yet); AC 3-5 (doc/ADR/
CHANGELOG text) are checked by `grep`, `docs_cli_test` and `changelog-check.sh` in the guards, not by a new test.

## RED confirmation
As authored, both targets fail to build on the missing API only (no typo, no missing target; `error_class_test` is a real
target and `pane_verbs` is declared at Cargo.toml:481):
```
$ cargo test -p holler-pane --test error_class_test
error[E0432]: unresolved imports `holler_pane::error::class_of`, `holler_pane::error::ErrorClass`
$ cargo test -p holler-cli --test pane_verbs
error[E0432]: unresolved import `holler_pane::error::class_of`
```
Because the compile error hides the assertions, I proved they fail for the right reason with a throwaway skeleton appended
to `holler-pane/src/error.rs` (`ErrorClass` enum, `exit_code() -> 1`, `class_of -> Failure` for every code), then reverted
it (`git checkout`; `error.rs` is unmodified). Against that skeleton, with today's `output.rs` (everything non-usage exits 1):
```
holler-pane error_class_test: 3 failed, 1 passed
  exit_codes_by_class                      left: 1 right: 2   (Usage.exit_code())
  an_open_code_is_a_refusal                left: Failure right: Refusal
  every_closed_code_has_the_decided_class  "the class of `usage`" left: Failure right: Usage
  (a_malformed_code_is_a_failure passes: see note above)
holler-cli pane_verbs output_api: 6 failed, 15 passed
  emit_error_in_text_mode_...              output_api.rs:145  left: 1 right: 3
  emit_error_in_json_mode_...              output_api.rs:160  left: 1 right: 3
  emit_exits_2_..._and_3_for_a_refusal     output_api.rs:212  left: 1 right: 3
  emit_stream_exits_3_on_a_refusal_item    output_api.rs:291  left: 1 right: 3
  a_refusal_exits_3_and_a_failure_...      output_api.rs:276  left: 1 right: 3
  every_closed_code_exits_by_its_class_... output_api.rs:240  left: 2 right: 1   (usage: class_of skeleton says 1)
```
Every failure is an assertion about the missing behavior (exit 3 for a refusal, the decided class). Note for F: the CLI
test `every_closed_code_..._both_formats` is only meaningful once `class_of` is real and `exit_code` in `output.rs` calls it;
the literal spot-check test is the independent pin on the numbers.

Staged by explicit path: `crates/holler-pane/tests/error_class_test.rs`, `crates/holler-cli/tests/pane_verbs/output_api.rs`,
and this handoff and decisions.md (the driver commits).

## Ready for F
Confirmed: RED is valid; F may implement against these tests. F must not edit them (T owns tests).
