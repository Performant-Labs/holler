# Handoff-F: Phase 6 (implement) - #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

**Date:** 2026-10-09
**Branch:** issue-676-implementation (on 679fae2, T-red)
**Issue:** #676

## What was done

- `crates/holler-pane/src/error.rs` (extend): new `pub enum ErrorClass { Usage, Refusal, Failure }` (derives `Debug, Clone,
  Copy, PartialEq, Eq`), `pub const fn ErrorClass::exit_code(self) -> i32` (Usage 2, Refusal 3, Failure 1) and
  `pub fn class_of(code: &str) -> ErrorClass` (lines 216-303). `class_of` looks the code up with `PaneCode::parse`. A closed
  code goes to one exhaustive `match closed { ... }` over `PaneCode`, with arms grouped by `|` and no `_` arm. Any other
  well-formed code (`is_valid_code`) is a Refusal, and a malformed code is a Failure. The module doc also gets one bullet
  naming the three items. The file grows from 617 to 710 lines.
- `crates/holler-cli/src/output.rs` (edit): `exit_code` is now `class_of(error.code.as_str()).exit_code()`. It no longer
  compares with `usage` and holds no table. Also changed: the import, the module doc (lines 10-16), the `emit` doc, the
  `exit_code` doc, and the `not_implemented` doc, which no longer calls a stub's error a "refusal" (it is a failure, exit 1).
- `crates/holler-cli/src/pane/mod.rs:51`, `crates/holler-cli/src/profile/mod.rs:39`: doc comments only. They now read
  "0 ok, 1 failure, 2 usage, 3 refusal".
- `crates/holler-cli/src/pane/args.rs:6`: doc comment only. A bad `--grid` is now "(exit 3)".
- `docs/adr/ADR-0021.md`, three changes:
  - §9: the exit-codes bullet names `holler_pane::error::class_of` and links #676. It now states that `ok` is false for
    exits 1, 2 and 3, and that an open code is always a refusal (A W-1). The 24-row classification table (22 closed codes,
    open, malformed) follows inside the bullet, with a short close-calls paragraph.
  - "Decisions taken" item 5 gains "(done in #676)".
  - "Deferred to named stories" gains one bullet for the I3 code (A W-1).
- `docs/adr/ADR-0003.md`: a `**Clarified by:** #676` header line (A W-3) and one sentence on the exit-codes rule.
- `CHANGELOG.md`: an `[Unreleased]` / Enhancements entry after the #670 entry. It covers 3 for a refusal and 1 for a
  failure in both modes, `ok` false, the table in ADR-0021 §9, stubs still exit 1, and links #676.
- `docs/handoffs/676/evidence.md`: nine source facts from unchanged code.

## Design decisions

- **`let Some(closed) = PaneCode::parse(code) else { ... }` then `match closed`.** A's Phase-8 check is "exactly one
  exhaustive `match` over `PaneCode`, with no `_` arm, inside `class_of`", so the open/malformed split happens before the
  match. I considered a single `match` over `Option<PaneCode>` with nested or-patterns and rejected it: it is also exhaustive,
  but it is not literally a match over `PaneCode`.
- **Arms follow the ADR table's order** (usage, then refusals, then failures), and each group has one comment line. The
  per-code reasons live once, in ADR-0021 §9, and are not repeated in code.
- **`exit_code` is a `const fn`**, a pure mapping like `is_valid_code`. `class_of` cannot be `const` because `PaneCode::parse`
  is not.
- **A W-4:** the `class_of` doc says it is not `PaneError::classify` and shows `class_of(error.code())` for a caller that holds
  a `PaneError`. It uses a plain code span, not an intra-doc link, because `classify` is crate-private.
- **A W-1:**
  - §9 says "An open code is always a refusal (exit 3), so a verb that reports a runtime failure uses a closed failure code;
    a malformed code is a failure."
  - I3's post-act mismatch wants exit 1, and no closed failure code describes it. It is recorded under "Deferred to named
    stories" for #644 and #645.
  - A asked O to put this in the brief's Risks. The brief was not amended, and `docs/handoffs/676*` is deleted when the run
    ends, so the ADR is the record that lasts.
- **A W-3:** ADR 0003's header gets a second `**Clarified by:**` line, as ADR 0007 already has two.
- **The table sits inside the exit-codes bullet** (indented two spaces), "right after" it as AC 4 says. A GFM parser
  (markdown-it) renders it as one 25-row table inside the list item.
- **The close calls are written once, under the table.** They name the three not-found codes, `probe-failed`,
  `herdr-version-unsupported`, `not-implemented` and `profile-drift`, with the alternative for each. They are not repeated in
  each row, and the ADR is the only lasting record of them.
- **"(done in #676)" placement.** In item 5 it follows "so a change follows it", so it does not claim that the epic, #638 and
  the verb stories were amended here.
- **The `not_implemented` doc in `output.rs`.** That file now defines "refusal" as exit 3, so "The refusal of a stub verb"
  would tell a reader that stubs exit 3. It now reads "The error of a stub verb: code `not-implemented` (a runtime failure,
  exit 1)". This is a comment in a blast-radius file.
- No inline code span in `docs/` starts with `holler ` (`docs_cli_test` parses those). `holler_pane::...` is safe.

## Reuse / extend-vs-new

Extended `crates/holler-pane/src/error.rs`, the object the brief's Reuse map names:

- The lookup reuses `PaneCode::parse` and `is_valid_code`. There is no second code list and no second validator.
- The CLI calls `class_of` and keeps no table.
- No new module, no `lib.rs` re-export, no manifest change.

## Architecture notes for A

- **New public API, additive only:** `holler_pane::error::{ErrorClass, class_of}` and `ErrorClass::exit_code`. No existing
  signature or behaviour in `holler-pane` changed (the frozen rule, brief decision 8).
- **Dependencies:** direction is unchanged. `holler-cli` already depends on `holler-pane`, and the test kit (#638) can call
  `class_of` without `holler-cli`.
- **`output.rs`:** `exit_code` stays private and is one line. `emit_text` and `emit_json` call it as before (`:283`, `:292`).
  `emit_stream`, `emit_error` and `emit_usage_error` are unchanged and reach it through `emit`.
- **Not touched:** `main.rs`, `Cargo.toml`/`Cargo.lock`, `holler-pane/src/lib.rs`, `holler-hub`, `holler-proto`, golden
  files and `cli-surface` fixtures.

## Deviations from spec / wireframe

These are beyond the brief's AC lists, all doc text in blast-radius files, and all on A's written warns:

- **ADR-0021:** one "Deferred to named stories" bullet (A W-1) and the W-1 sentence in §9.
- **ADR 0003:** one header line (A W-3).
- **`output.rs`:** the `not_implemented` doc line (vocabulary residue in the module this story changes).

`main.rs` is **not** edited, as the brief's decision 7, Blast radius and AC 7 say. See Known issues for A W-2.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-pane --test error_class_test
test a_malformed_code_is_a_failure ... ok
test an_open_code_is_a_refusal ... ok
test exit_codes_by_class ... ok
test every_closed_code_has_the_decided_class ... ok
test result: ok. 4 passed; 0 failed

$ cargo test -p holler-cli --test pane_verbs
test result: ok. 64 passed; 0 failed
  (output_api: 21 passed, including every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats,
   a_refusal_exits_3_and_a_failure_exits_1_in_both_formats, emit_stream_exits_3_on_a_refusal_item, the two
   pane-not-found tests now at 3, emit_exits_2_..._and_3_for_a_refusal; the unavailable / failed-write / stub
   exit-1 tests unchanged and passing)

$ cargo build --workspace                                    Finished (0 warnings)
$ cargo clippy --workspace --all-targets -- -D warnings      Finished (clean)
$ cargo test --workspace                                     exit 0: 1048 passed, 0 failed, 5 ignored, 100 targets (doctests incl.)
$ cargo machete                                              didn't find any unused dependencies
$ bash scripts/lint.sh                                       exit 0 (600-line warnings only; error.rs 710, under 900)
$ bash scripts/changelog-check.sh                            changelog-check: ok
$ bash scripts/test-hooks.sh                                 exit 0
$ cargo test -p holler-cli --test docs_cli_test              3 passed
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps   clean (intra-doc links resolve)
$ rustfmt --check --edition 2021 <file>                      clean: error_class_test.rs, error.rs, output.rs, args.rs,
                                                             pane/mod.rs, profile/mod.rs (each was clean at origin/main)
$ grep -rn "refused or failed" crates/ docs/adr/             no output (exit 1)
```

## Evidence appendix

`docs/handoffs/676/evidence.md` has nine facts:

- `PaneCode::parse` returns `None` for any non-closed text.
- `is_valid_code` refuses `""`.
- `PaneError::code()` returns a `Refused`'s open code.
- `from_wire` turns a malformed code into `unavailable`.
- Both formats call the one `exit_code`.
- `settle` only turns a failed write on a success into 1.
- `emit_stream` returns the first non-zero code.
- `ErrorCode::new` validates with `is_valid_code`, so the malformed branch cannot be reached from the CLI.
- `main.rs:112` applies the returned code unchanged.

## Tests that look wrong (for T)

None of T's tests is wrong; all pass as authored. One AC-3 item is in a test file, which F does not edit:

- `crates/holler-cli/tests/pane_verbs/spec_flags.rs:3-4` (the module `//!` doc, no test code) still says "A refusal is exit 1
  in the verb". A suggested replacement for lines 3-4: "...`holler-pane` guards and their stable codes. A guard's refusal is
  exit 3 in the verb (one coded `usage` exits 2); neither is a clap usage error, so the code is stable and the message is
  one line." AC 3 is fully met once T changes it at T-green.

## Known issues

- **A W-2, `main.rs`, not changed by design.** The `run_with_stdio` doc at `crates/holler-cli/src/main.rs:253-255` still says
  a wiring error is reported "(exit 1, with the error's code)".
  - The brief keeps `main.rs` out of this story (decision 7, Blast radius, AC 7).
  - The line is accurate today, because `Wiring::connect` always returns `Ok` (`crates/holler-cli/src/pane/wiring.rs`).
  - It becomes wrong only when #649 fills `connect` and it can return, for example, `herdr-version-unsupported` (exit 3).
    #649 should change it to "(with the error's code, and the exit code of its class)".
  - This handoff is deleted when the run ends, so the note has to reach #649 through the PR body or an issue comment.
- **The spec_flags.rs doc line** above, which is T's.
- **The stub verb docs.** The 17 stub verb files say "refuse, naming the story that owns it" about a `not-implemented`
  error, which is a failure with exit 1. Each verb's story replaces its stub, so these are out of scope, like A's W-5 on the
  legacy verbs' wording (left for #646).

## Files changed

- `crates/holler-pane/src/error.rs`
- `crates/holler-cli/src/output.rs`
- `crates/holler-cli/src/pane/mod.rs`
- `crates/holler-cli/src/profile/mod.rs`
- `crates/holler-cli/src/pane/args.rs`
- `docs/adr/ADR-0021.md`
- `docs/adr/ADR-0003.md`
- `CHANGELOG.md`
- `docs/handoffs/676/handoff-F.md`, `docs/handoffs/676/evidence.md`, `docs/handoffs/676/decisions.md` (pipeline artifacts)
