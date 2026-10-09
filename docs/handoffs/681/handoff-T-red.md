# Handoff-T-red: #681 the JSON-envelope checker in `holler-pane-testkit` (slice b of #638)

**Date:** 2026-10-09
**Branch:** issue-681-implementation
**Brief / wireframe reviewed:** docs/handoffs/681-brief.md, docs/handoffs/681/handoff-A.md (no wireframe: no UI surface)

## A precondition
Confirmed: A returned PASS on the plan (Phase 3), with six warns and no blocks. I took in W-1 (smallest extra key, not Map order) and W-3 (quote the `String` payloads in `Display`) as test rows. See below.

## Tests authored
Files (staged by explicit path, nothing committed): `crates/holler-pane-testkit/tests/envelope_test.rs` (new), `crates/holler-pane-testkit/src/envelope.rs` (stub kept, doctest and `#[cfg(test)] mod tests` appended; F rewrites the module doc and keeps the doctest), `crates/holler-pane-testkit/Cargo.toml` (+`serde_json = { workspace = true }` with its comment, as the brief allows T to add), `Cargo.lock` (cargo wrote the one `serde_json` line). No `[[test]]` entry needed: the testkit uses default autotests.

All tests call only the public `check_envelope` / `check_ndjson`, so F is free to name private helpers.

Integration (`tests/envelope_test.rs`, the public contract, cheapest tier because the checker is a pure function):
- AC1 `a_success_at_exit_0_is_accepted` (object data and `null` data), `a_failure_is_accepted_at_the_exit_code_of_its_class` (timeout/1, usage/2, pane-not-found/3, open code quota-exceeded/3), `the_adr_examples_are_accepted` (the two ADR lines verbatim with spaces), `a_missing_final_newline_is_accepted`, `every_closed_code_is_accepted_only_at_the_exit_code_of_its_class` (all `ALL_CODES` x exit 1..=3, the expectation computed from `class_of`, not from a second table).
- AC2 `every_mutant_is_rejected_with_its_fault`: 46 rows in four helper tables (framing, keys, failure body, code and message), the assertion message names the row. All of the brief's rows, plus extras that pin rule order and the cases the brief maps to existing variants: missing-schema-version, missing-ok, missing-data-and-error (names `data`, the first in order), schema-float (`1.0`), schema-null, schema-checked-before-ok (schema 2 at exit 3 is `SchemaVersion`), no-code, empty-error-object, code-empty, blank-message, message-not-string, message-checked-before-class (empty message at exit 3 is `MessageNotOneLine`). **A's W-1 rows:** `two-extra-keys-smallest-first` (top level) and `error-two-extra-keys-smallest-first` (inside `error`) are literal strings with `zz` before `aa`, expecting `UnknownKey("aa")` / `UnknownKey("error.aa")`.
- AC2 `the_mutant_tables_cover_every_fault`: `fault_name` is an exhaustive `match` with no `_` arm; the set of names covered by the two tables is compared with the 17 names. (It passes against any compiling stub by design; it is a compile-time and table guard, not a behaviour test.)
- AC3 `a_stream_of_ok_lines_at_exit_0_is_accepted`, `a_stream_ending_in_one_failure_is_accepted_at_its_exit_code` (three shapes), `every_stream_mutant_is_rejected_with_its_fault` (12 rows: the brief's 11 plus a non-boolean `ok` before the last line, which gives `NotLastFailure` per A's note on rule 3).

Unit (`src/envelope.rs` `mod tests`): AC4 `exit_code_is_checked_before_stdout`, `only_one_final_newline_is_stripped`, `broken_json_that_starts_with_a_brace_is_not_text_before`, `every_fault_displays_as_one_line` (the `UnknownKey` and `CodeNotKebab` payloads contain `\n` / `\r\n`: A's W-3).
Doctest: AC5, in the module doc (success at exit 0 is ok, same stdout at exit 3 is `OkDisagreesWithExit`).

Not authored here, by design (they are F's GREEN checklist, not tests): AC6 `cargo tree` shape, AC7 docs wording, AC8 CHANGELOG, AC9 guards, AC10 scope.

## RED confirmation
Command and output (the checker items do not exist yet; the crate and the `serde_json` dependency resolve, so the only errors are the missing items):

```
$ cargo test -p holler-pane-testkit --test envelope_test
error[E0432]: unresolved imports `holler_pane_testkit::envelope::check_envelope`, `...::check_ndjson`, `...::Envelope`, `...::EnvelopeError`, `...::EnvelopeFault`
  --> crates/holler-pane-testkit/tests/envelope_test.rs:10:5
error: could not compile `holler-pane-testkit` (test "envelope_test") due to 1 previous error

$ cargo test -p holler-pane-testkit --lib
error[E0425]: cannot find function `check_envelope` in this scope        (x several)
error[E0433]: cannot find type `EnvelopeFault` in this scope             (x several)
```

The brief names a compile failure on exactly these items as the expected RED for a new public API. To prove the tests would fail on assertions and not on setup, I checked them two more ways in the working tree, then restored the RED state of `envelope.rs` (it holds only the stub doc, the doctest and the unit tests, no production code):
1. Compiling stub (all 17 variants, both functions returning `Err(NotJson)`): 9 of 10 integration tests FAIL on their assertions (`assertion left == right failed` with the row name, for example `mutant ...`, or `a_success_at_exit_0_is_accepted` at `envelope_test.rs:74`). The tenth, `the_mutant_tables_cover_every_fault`, passes by design (above). No failure is a setup, import or fixture error.
2. A throwaway reference implementation written from the brief's rule tables: all 10 integration tests, all 4 unit tests and the doctest pass, so the tests are satisfiable and consistent with the brief. With the extra-key choice changed to the first key in map order and `serde_json/preserve_order` forced on (what `cargo test --workspace` does), `two-extra-keys-smallest-first` FAILS (`mutant ... expected UnknownKey("aa")`), so the W-1 row detects a naive implementation under AC9's workspace run. The reference implementation was discarded; F writes the real one.

`cargo clippy` on the test target with the reference implementation raised nothing in `envelope_test.rs`. `cargo machete` passes. `rustfmt --check --edition 2021` passes on both files. `scripts/lint.sh`: `envelope_test.rs` is 671 lines, a warn (limit 600 warn / 900 fail), not a failure; `#[allow]`s: none.

## Ready for F
Confirmed: RED is valid. F may implement against these tests.

F notes (from the tests, not new requirements):
- Compute the smallest extra key explicitly (`min()`), at the top level and inside `error`. Do not rely on `Map` order.
- `Display` must print the `UnknownKey` and `CodeNotKebab` payloads with `{:?}` (or otherwise escape them) so it stays one line.
- Do not mark `EnvelopeFault` `#[non_exhaustive]` (the test crate's exhaustive `match` would stop compiling).
- Keep the doctest and the `mod tests` already in `envelope.rs` when replacing the stub module doc.
