# Handoff-T-red: Phase 4 — the #660 conformance suite

**Date:** 2026-10-09, 10:47 PM MDT
**Branch:** `issue-0660-output` (run worktree `0660-output`, at `519947a`)
**Brief:** `docs/handoffs/0660-output/brief.md` · **Survey:** `docs/handoffs/0660-output/survey.md` · **A:** PASS (`handoff-A.md`, findings applied)

VERDICT: GREEN-on-contact

## A precondition

Confirmed: A returned PASS on the plan (up-front review, `handoff-A.md`). Findings 1–3 applied: no new
top-level test target; the expected files were held to, not the wider glob; the #638 checker is wired
as the conformance authority and `verb_harness::one_envelope` was not grown (the one place the
ALL_CODES test used it was **replaced** by the checker; `one_envelope` itself untouched).

## Files touched (all within `crates/holler-cli/tests/**`)

| Path | What was authored |
|---|---|
| `crates/holler-cli/tests/pane_verbs/output_api.rs` (470 → 749 lines) | 8 new tests + 1 extended test + imports/doc; the #660 conformance section at the tail of the file |
| `crates/holler-cli/tests/pane_verbs/process/stub.rs` (214 → 243 lines) | 1 new process-level test + the `check_envelope` import |

No submodule split was needed: 749 lines is under the size gate's 900-line failure (it does cross the
600-line **warn** — advisory note below). `tests/pane_verbs/main.rs`, `verb_harness/mod.rs`, every
`Cargo.toml`, `output.rs`, `holler-pane/**`, `holler-pane-testkit/**` — untouched (`git status` shows
exactly the two files above).

## Tests authored, mapped to the acceptance bullets

1. **Goldens through `check_envelope`, both formats** — `golden_success_in_both_formats_through_the_checker`,
   `golden_refusal_in_both_formats_through_the_checker`,
   `golden_usage_error_in_both_formats_through_the_checker`: each outcome's text leg is an exact raw
   golden (`"3 items\n"` / `"error: …\n"` / `error: <message>\n`), each JSON leg runs the #638
   checker at the outcome's exit code (0 / 3 / 2) **and** asserts the exact compact line
   (`schema_version` first), the checker-returned `data`/`error.code`/`error.message` included.
2. **NDJSON through `check_ndjson`** — `emit_stream_in_json_mode_is_valid_ndjson_through_the_checker`
   (3 ok items at exit 0; every line re-checked on its own) and
   `emit_stream_ending_in_a_refusal_is_valid_ndjson_at_exit_3` (an ok line then a `pane-not-found`
   last line at exit 3 — the `NotLastFailure` semantics held positively).
3. **Forced diagnostic in JSON mode, stdout still ONE envelope** —
   `process/stub.rs::a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope`: the real binary,
   a stub verb under `--debug noisy --format=json` (a *valid* level, so dispatch is reached — the
   `--debug bogus` refusal path at stub.rs:199 never gets this far). Asserts the banner diagnostic
   (`logging_started level=noisy`) on **stderr** and `check_envelope(stdout, 1)` **ok** with the
   `not-implemented` error — the untested banner + envelope combination.
4. **Exit parity, `ALL_CODES`, checker assertions** — the existing
   `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats` extended in place: its
   JSON leg now goes through `check_envelope(&out, json)`, adding rules the local helper never
   checked (framing, exact key set, `schema_version` the integer 1, one-line message, and rule 13's
   class-vs-exit re-derivation), plus the code carried in the returned envelope. The text/JSON parity
   asserts stay.
5. **GridPos golden, row first** — `an_envelope_carrying_a_grid_pos_serializes_row_col_pos_row_first`:
   `emit(Format::Json, Ok(GridPos { row: 2, col: 3 }))`, asserted through `check_envelope` (data is
   any value, rule 7) **paired** with the exact raw line
   `…,"data":{"row":2,"col":3,"pos":"r2c3"},…` and explicit index assertions that `row` < `col` <
   `pos` in the raw text — the compact-golden pattern (`output_api.rs:176-189` of the old file).
6. **#637-fixed signatures, compile-pinned** — `the_fixed_signatures_and_types_compile_unchanged` plus
   three module-level type aliases (`EmitU8`, `EmitStreamU8`, `EmitUsageError`): the declared
   signatures of `emit` / `emit_stream` / `emit_usage_error` with generics instantiated at concrete
   fn-pointer types — any change to a parameter, return type or bound breaks the build. The bindings
   are then called once each (proving they are the module's functions), and the fixed types are used
   as declared: `Format` (two variants, `Copy`), `Envelope<T>` (four public members, both
   constructors, `SCHEMA_VERSION`), `ErrorBody`/`ErrorCode` (`new`/`as_str`), `GridPos` (public
   fields, `rRcC` display).

Tier note: bullets 1–2, 4–6 are unit tier (in-process `Sink`, cheapest sufficient — the module is a
pure function of a `Sink`); bullet 3 is process tier because only the real binary produces the banner
next to an envelope.

## GREEN-on-contact evidence (the honest-RED policy, brief §RED policy / decision 4)

Every new and extended test passed on first contact with current `output.rs` (#670/#676 main). No test
was weakened or reinterpreted after the run; the suite below is what was authored. Commands and
output tails, in the run worktree, one cargo at a time:

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s

$ cargo test -p holler-cli --test pane_cli_process
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

The new/extended tests individually (filtered run, `cargo test -p holler-cli --test pane_verbs --
output_api` → `test result: ok. 28 passed; 0 failed`):

```
test output_api::golden_success_in_both_formats_through_the_checker ... ok
test output_api::golden_refusal_in_both_formats_through_the_checker ... ok
test output_api::golden_usage_error_in_both_formats_through_the_checker ... ok
test output_api::emit_stream_in_json_mode_is_valid_ndjson_through_the_checker ... ok
test output_api::emit_stream_ending_in_a_refusal_is_valid_ndjson_at_exit_3 ... ok
test output_api::an_envelope_carrying_a_grid_pos_serializes_row_col_pos_row_first ... ok
test output_api::the_fixed_signatures_and_types_compile_unchanged ... ok
test output_api::every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats ... ok   (extended)
```

```
$ cargo test -p holler-cli --test pane_cli_process -- a_forced_diagnostic
test stub::a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope ... ok
test result: ok. 1 passed; 0 failed; …
```

Hygiene: `cargo clippy -p holler-cli --test pane_verbs --test pane_cli_process` → `Finished` with
**zero warnings** (the two initial `type_complexity` warnings on the signature pins were fixed by
factoring the fn-pointer types into named aliases, clippy's own suggestion). Both touched files are
clean under `cargo fmt --check` for this package.

Why the suite is not vacuous: the checker's bite is independently proven by its landed self-tests
(`holler-pane-testkit/tests/envelope_test.rs`: a mutation table covering all 17 faults, plus
`every_closed_code_is_accepted_only_at_the_exit_code_of_its_class`); my tests feed it the **real**
`emit`/`emit_stream` output and the real binary's stdout, and every golden also asserts exact raw
text, so a framing, key-set, key-order, class or exit regression faults or fails equality. No
mutation of tracked files was performed at any point.

## Notes for F

**None from faults — there are no faults.** Per the brief's RED policy and the journal's decision 4,
a green-on-contact suite means there is no honest RED and no conformant fix for F to make; the run
stops here for the operator (no sham F stage). `output.rs` as it stands at `519947a` satisfies every
acceptance bullet's tested property.

Advisory (non-blocking, outside the acceptance bullets — recorded so they are not lost):

- **Blank-message edge:** `output.rs`'s `one_line()` collapses whitespace-only text to `""`, which
  would fault checker rule 12 (`MessageNotOneLine`). No real path reaches it (`PaneError` messages
  are never blank), so it is deliberately untested — a speculative RED was not manufactured.
- **Empty NDJSON stream:** `emit_stream` over zero items writes nothing, which `check_ndjson` faults
  as `EmptyStream`. `pane watch` with nothing owed does print nothing (existing
  `watch_with_nothing_owed_prints_nothing`); if that verb's JSON mode is ever checked through
  `check_ndjson`, the two contracts will meet. Watch's story owns that surface; `output_api` does not
  pin the empty case.
- **File size:** `output_api.rs` is now 749 lines — past the 600-line warn, under the 900 fail. If it
  grows again, the #660 conformance tail is a natural `pane_verbs/` submodule split (A's finding 1
  blesses the mechanism).
- **Pre-existing fmt drift (not this run's doing):** `cargo fmt --check -p holler-cli` reports ~2,916
  diff hunks across `src/**` and `tests/**` of the committed tree under this machine's rustfmt
  1.9.0 / edition 2021 — including files this run never touched. An early `cargo fmt -p holler-cli`
  of mine reformatted that drift as collateral; **every file except the two above was reverted
  immediately** (verified: `git status` shows only the two touched files). The drift itself predates
  this run and belongs to whoever owns it (parallel c3r1 / operator) — but be aware a repo-wide
  `cargo fmt --check` gate is red today independent of this story.

## Boundary compliance

Touched files: `crates/holler-cli/tests/pane_verbs/output_api.rs`,
`crates/holler-cli/tests/pane_verbs/process/stub.rs` — both inside `crates/holler-cli/tests/**` and
both in the brief's expected set. Nothing else in the worktree is modified
(`git status`: those two + this handoff directory). No `Cargo.toml`, no `holler-pane/**`, no
`holler-pane-testkit/**`, no `src/**`, no `cli.rs`/`main.rs`/verb files, no `verb_harness/mod.rs`
change (finding 3 honored: `one_envelope` unmodified and still used only by the older tests that
predate the checker wiring). No mutation testing; one cargo invocation at a time; no live fleet, pane
or Herdr session touched; nothing committed (edits left uncommitted in the worktree as instructed).
