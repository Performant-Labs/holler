# Brief: #681 the JSON-envelope checker in `holler-pane-testkit` (slice b of #638)

Repo: Performant-Labs/holler. Issue: #681 (slice b of #638, epic #633). Rigor: in-session. UI surface: no. Kind: test-only
(a test-support library; nothing a user runs changes).

**Branch:** `issue-681-implementation`, based on `e410e9d` (#637, #669, #670, ADR-0021, #639, #676 and slice a of #638
merged). **Design (D):** N/A. **Decision record:** ADR-0021 section 9 (the `--format=json` envelope, exit codes 0/1/2/3).
The issue is the source of truth; where this brief differs from it, the issue wins, except where "Decisions made in this
brief" says why.

**Size check:** one module filled (`src/envelope.rs`, about 330 lines with its unit tests), one integration test file
(about 280 lines), a three-line `Cargo.toml` change, the matching `Cargo.lock` line, one doc line in `lib.rs` and a
CHANGELOG entry. About 620 lines in all. That fits one run, so no split.

## Problem

Every `holler pane` and `holler profile` verb story in wave 3 has to show that its `--format=json` output is the one
envelope of ADR-0021 section 9, with the exit code that matches the error's class. Slice a of #638 put an empty stub at
`crates/holler-pane-testkit/src/envelope.rs` for this. This slice fills the stub with a checker that reads a verb's stdout
and exit code and returns either the parsed envelope or the first rule the output breaks, for one envelope or for an NDJSON
stream (`pane watch`). The testkit is a dev-dependency of `holler-cli` and `holler-hub`, so it cannot use the CLI's own
`Envelope` type. It parses with `serde_json` alone and classifies codes with `holler_pane::error::class_of`.

## Evidence (verbatim, as of `e410e9d`)

The stub this slice fills (the whole file):
```
crates/holler-pane-testkit/src/envelope.rs:1-4
//! The JSON-envelope checker every verb's `--format=json` tests reuse: stdout is one
//! envelope (or NDJSON lines of them) with `schema_version` 1, `ok` agreeing with the
//! exit code, `error` null exactly when `ok`, and nothing else. Empty stub declared by
//! #638 so that no two slices edit `lib.rs`; slice b (#681) fills it.
```
It is already declared, and the crate does no flat re-exports:
```
crates/holler-pane-testkit/src/lib.rs:5-8
//! It depends on `holler-pane` alone and must not depend on `holler-cli`: the hub and
//! the CLI take it as a dev-dependency, so a normal dependency back would make a cycle
//! (ADR-0021 section 5). Every item is reached by its module path. There are no flat
//! re-exports, so a later slice never edits this file.
crates/holler-pane-testkit/src/lib.rs:23     //! - [`envelope`] — the JSON-envelope checker (slice b, #681).
crates/holler-pane-testkit/src/lib.rs:32     pub mod envelope;
```
The testkit manifest as it is now (no `serde_json`, no dev-dependencies, default `autotests`):
```
crates/holler-pane-testkit/Cargo.toml:13-21
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise).
[dependencies]
# The ports the fakes implement and the suites drive (`PaneStore`, `Pane`, `PaneError`,
# `next_generation`); for every fake and suite in this crate.
holler-pane = { path = "../holler-pane" }

# Workspace lints (issue #149).
[lints]
workspace = true
```
The workspace `serde_json` has no feature list and no `preserve_order`, so `serde_json::Map` iterates keys in sorted order:
```
Cargo.toml:45-48
# serde_json without `preserve_order`: holler-proto's canonical round-trips
# sort keys explicitly (tests/common) and the hub's `--json` status printer is
# hand-built (insertion order), so nothing relies on serde_json::Map's ordering.
serde_json = { version = "1" } # for holler-proto, holler-cli
```
`Cargo.lock` resolves it to `serde_json 1.0.151`. In that version `Value` derives `Clone, Eq, PartialEq, Hash`
(`serde_json-1.0.151/src/value/mod.rs:115`), so an `Envelope` holding a `Value` can derive `Eq`, and
`StreamDeserializer::byte_offset` exists (`src/de.rs:2424`).

The classifier and the validator this slice calls (#676), in `holler-pane`:
```
crates/holler-pane/src/error.rs:163   pub const fn is_valid_code(code: &str) -> bool {      // ^[a-z]+(-[a-z]+)*$
crates/holler-pane/src/error.rs:221   pub enum ErrorClass {  Usage, Refusal, Failure }        // Debug, Clone, Copy, PartialEq, Eq
crates/holler-pane/src/error.rs:239-245
    pub const fn exit_code(self) -> i32 {
        match self {
            ErrorClass::Usage => 2,
            ErrorClass::Refusal => 3,
            ErrorClass::Failure => 1,
        }
    }
crates/holler-pane/src/error.rs:266-273
pub fn class_of(code: &str) -> ErrorClass {
    let Some(closed) = PaneCode::parse(code) else {
        return if is_valid_code(code) {
            ErrorClass::Refusal
        } else {
            ErrorClass::Failure
        };
    };
crates/holler-pane/src/error.rs:146   pub const ALL_CODES: &[&str] = &CLOSED_CODES;           // the 22 closed codes
```
The signature matches the issue's assumption: `class_of` takes the code string and `exit_code` is on `ErrorClass`. No
adjustment is needed. `usage` is `Usage` (exit 2). An open well-formed code (for example `quota-exceeded`) is `Refusal`
(exit 3). `timeout`, `unavailable`, `generation-conflict` and `not-implemented` are `Failure` (exit 1). `pane-not-found` is
`Refusal` (exit 3).

What the real CLI writes, which the checker must accept (`holler-cli` is read only, never depended on):
```
crates/holler-cli/src/output.rs:34        pub const SCHEMA_VERSION: u32 = 1;
crates/holler-cli/src/output.rs:150-160
/// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
/// `ok` is false and `error` is `null` when it is true.
#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
}
crates/holler-cli/src/output.rs:125-132   pub struct ErrorBody { pub code: ErrorCode, pub message: String }   // serializes as {"code","message"}
crates/holler-cli/src/output.rs:225-238   emit_stream: one emit per item; returns the first non-zero code, so the failure is the last line
crates/holler-cli/src/output.rs:288-299   emit_json: a failure is Envelope::<()>::failure(error) with message one_line(...), exit exit_code(&error)
crates/holler-cli/src/output.rs:304-315   write_envelope: serde_json::to_string(envelope) then write_line (one compact line)
crates/holler-cli/src/output.rs:318-324   write_line: writes the line, adds "\n" unless it ends with one, flushes
crates/holler-cli/src/output.rs:338-340
fn exit_code(error: &ErrorBody) -> i32 {
    class_of(error.code.as_str()).exit_code()
}
```
So one envelope on stdout is `{"schema_version":1,"ok":...,"data":...,"error":...}\n`. A success's `data` may be any
JSON value, including `null` (a verb whose data is `()`).

The contract (ADR-0021 section 9):
```
docs/adr/ADR-0021.md:353-354
{"schema_version": 1, "ok": true, "data": {}, "error": null}
{"schema_version": 1, "ok": false, "data": null, "error": {"code": "pane-not-found", "message": "pane not found: hj-c1r1"}}
docs/adr/ADR-0021.md:357-358   `error` is `null` exactly when `ok` is true. ... the envelope has no `detail`.
docs/adr/ADR-0021.md:360       `pane watch` in JSON mode writes NDJSON: one envelope per line, flushed per line.
docs/adr/ADR-0021.md:361,366   Exit codes are the same in both formats ...: 0 ok, 1 runtime failure, 2 usage, 3 refusal.
                               ... In JSON mode `ok` is false for exit 1, 2 and 3 alike.
docs/adr/ADR-0021.md:405       `schema_version` is an integer and is 1 now.
docs/adr/ADR-0021.md:326       Every message is one line and never echoes a secret.
```
(Line numbers were checked with `grep -n` on `e410e9d`. Cite text, not numbers, if they drift.)

The house pattern for a testkit test file, which this slice copies:
```
crates/holler-pane-testkit/tests/pane_store_conformance_test.rs:1
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
```
`scripts/lint.sh` check 1 requires a trailing `// #NNN` on every `#[allow]`. Check 4 warns at 600 lines and fails at 900.
Check 5 requires a `# for <consumer>` comment on any dependency line that has a `features =` list.

## Public API (exact; `holler_pane_testkit::envelope`)

```rust
/// One envelope that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub schema_version: u64,          // always 1 once checked
    pub ok: bool,
    pub data: serde_json::Value,      // any value when ok; Value::Null on failure
    pub error: Option<EnvelopeError>, // None exactly when ok
}

/// The `error` member of a failed envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeError {
    pub code: String,
    pub message: String,
}

/// The first rule the output breaks. Exactly these 17 variants (the issue's list; none added).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeFault {
    NotJson,
    TextBefore,
    TextAfter,
    NotAnObject,
    MissingKey(&'static str),
    UnknownKey(String),
    SchemaVersion,
    ExitCodeUnknown(i32),
    OkDisagreesWithExit,
    ErrorWhileOk,
    NoErrorWhenFailed,
    DataOnFailure,
    CodeNotKebab(String),
    MessageNotOneLine,
    ClassDisagreesWithExit,
    EmptyStream,
    NotLastFailure,
}
impl std::fmt::Display for EnvelopeFault { /* one non-empty line per variant, naming the rule */ }
impl std::error::Error for EnvelopeFault {}

pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault>;
pub fn check_ndjson(stdout: &str, exit_code: i32) -> Result<Vec<Envelope>, EnvelopeFault>;
```
Every variant gets a `///` doc line saying what triggers it. Imports: `holler_pane::error::{class_of, is_valid_code}`. No
other public item. Private helpers are free (for example `frame`, `exact_keys`, `check_failure`), but each must have a
caller (`dead_code` is denied).

## The rules, in check order (`check_envelope`)

The first rule broken is the fault returned. The order is part of the contract, because a mutant that breaks two rules
must give a predictable fault.

| # | Rule | Fault |
|---|---|---|
| 0 | `exit_code` is 0, 1, 2 or 3. This is checked before stdout is read. | `ExitCodeUnknown(exit_code)` |
| 1 | **Framing.** Let `body` be `stdout` with one final `\n` removed if present (one only, and not `\r\n`). (a) If `body` is non-empty, its first byte is not whitespace, and a JSON value parses starting at byte 0 (`serde_json::Deserializer::from_str(body).into_iter::<Value>()`, first item `Ok`), then anything at all after that value (`byte_offset()` < `body.len()`: whitespace, a second envelope, a blank line, text) gives the fault. | `TextAfter` |
| 1 | (b) Otherwise, if `body` does not start with `{` and some `{` at a byte index `i > 0` starts a JSON value that parses (the first item of the stream deserializer over `&body[i..]` is `Ok`), the fault is this. Covers a log line or leading spaces before the envelope. | `TextBefore` |
| 1 | (c) Otherwise (empty, whitespace only, truncated, not JSON, or broken JSON that starts with `{`). | `NotJson` |
| 2 | The value is a JSON object. | `NotAnObject` |
| 3 | The keys `schema_version`, `ok`, `data`, `error` are all present. The first missing one, in that order, is named. | `MissingKey("schema_version" \| "ok" \| "data" \| "error")` |
| 4 | There is no other key. The first extra key in `Map` order (sorted) is named. | `UnknownKey(key)` |
| 5 | `schema_version` is the JSON integer `1` (`as_u64() == Some(1)`). `2`, `"1"`, `1.0` and `null` all fail. | `SchemaVersion` |
| 6 | `ok` is the JSON boolean `exit_code == 0`. `ok: true` with exit 1/2/3, `ok: false` with exit 0, and a non-boolean `ok` all fail. | `OkDisagreesWithExit` |
| 7 | When ok: `error` is `null`. On success, return `Envelope { schema_version: 1, ok: true, data, error: None }`; `data` may be any value, `null` included. | `ErrorWhileOk` |
| 8 | When failed: `error` is a JSON object (`null`, a string or any non-object fails). | `NoErrorWhenFailed` |
| 9 | When failed: `data` is `null`. | `DataOnFailure` |
| 10 | When failed: the `error` object has exactly the keys `code` and `message`. A missing key is checked first, `code` then `message`; then an extra key. | `MissingKey("error.code" \| "error.message")`, `UnknownKey("error.<key>")` |
| 11 | `code` is a string that passes `is_valid_code`. A non-string `code` is named by its JSON text (`5` gives `"5"`). | `CodeNotKebab(code)` |
| 12 | `message` is a string that is not blank (`trim()` non-empty) and contains no `\n` and no `\r`. A non-string `message` fails too. | `MessageNotOneLine` |
| 13 | `class_of(code).exit_code() == exit_code` (`usage` pairs with 2, a refusal with 3, a failure with 1). | `ClassDisagreesWithExit` |
| — | All checks passed: return `Envelope { schema_version: 1, ok: false, data: Value::Null, error: Some(EnvelopeError { code, message }) }`. | — |

## The rules for a stream (`check_ndjson`)

| # | Rule | Fault |
|---|---|---|
| 0 | `exit_code` is 0, 1, 2 or 3 (checked first). | `ExitCodeUnknown(exit_code)` |
| 1 | Remove one final `\n` if present. If nothing is left (`""` or `"\n"`), there is no envelope. | `EmptyStream` |
| 2 | Split the rest on `\n`. Each line is checked **alone** with `check_envelope`, in order, and the first fault wins. A blank line inside the stream is a line, so it gives `NotJson`. | (that line's fault) |
| 3 | Every line but the last is checked at exit 0. An `OkDisagreesWithExit` from such a line (it has `ok: false`) is reported as `NotLastFailure`. Its other faults are reported as they are. | `NotLastFailure` |
| 4 | The last line is checked at `exit_code`. With exit 0 it must be `ok`. Otherwise it must be the failure, which matches `emit_stream`. | (its fault, e.g. `OkDisagreesWithExit`) |
| — | Return every line's `Envelope`, in order. | — |

Consequences, pinned by the tests: `[ok, ok]` at exit 0 is accepted. `[ok, fail(timeout)]` at exit 1 is accepted.
`[ok, fail, ok]` at exit 3 gives `NotLastFailure`, and so does `[fail, ok]` at exit 0. `[ok, fail]` at exit 0 gives
`OkDisagreesWithExit` (it comes from the last line), and so does `[ok, ok]` at exit 3.

## Acceptance criteria

Test names are what T authors (RED first). Integration tests go in the new file
`crates/holler-pane-testkit/tests/envelope_test.rs` (default `autotests`, so the manifest needs no `[[test]]` entry). Unit
tests go in a `#[cfg(test)] mod tests` at the end of `src/envelope.rs`.

- [ ] **AC1, the good envelopes are accepted** (`tests/envelope_test.rs`):
  - `a_success_at_exit_0_is_accepted`: `{"schema_version":1,"ok":true,"data":{"name":"hj-c1r1"},"error":null}\n` at exit 0
    gives `Ok(Envelope { schema_version: 1, ok: true, data: json!({"name":"hj-c1r1"}), error: None })`. The same with
    `"data":null` is accepted with `data == Value::Null`.
  - `a_failure_is_accepted_at_the_exit_code_of_its_class`: compact failure envelopes are accepted at `timeout` exit 1,
    `usage` exit 2, `pane-not-found` exit 3 and `quota-exceeded` (an open code) exit 3. Each returns `ok: false`,
    `data: Value::Null` and `error: Some(EnvelopeError { code, message })` with the same text.
  - `the_adr_examples_are_accepted`: the two ADR-0021 example lines, verbatim with their spaces, are accepted at exit 0 and
    exit 3.
  - `a_missing_final_newline_is_accepted`: the success envelope without its `\n` is accepted.
  - `every_closed_code_is_accepted_only_at_the_exit_code_of_its_class`: for every `code` in `holler_pane::error::ALL_CODES`
    and every exit in `1..=3`, a failure envelope with that code gives `Ok` exactly when
    `class_of(code).exit_code() == exit`, and `Err(ClassDisagreesWithExit)` otherwise.
- [ ] **AC2, mutation check: every mutant is rejected with its fault** (`tests/envelope_test.rs`):
  - `every_mutant_is_rejected_with_its_fault`: a table of `(name, stdout, exit, expected fault)`. Most rows are a good
    envelope with one change. The test asserts `check_envelope(stdout, exit) == Err(expected)` row by row, and the
    assertion message names the row. The table has at least these rows (the issue's named cases are marked *):
    | Row | stdout (`G` = a good success, `F` = a good failure coded `timeout`) | exit | Fault |
    |---|---|---|---|
    | broken* | `{` | 0 | `NotJson` |
    | empty | `` (empty) | 0 | `NotJson` |
    | prose | `not json\n` | 1 | `NotJson` |
    | text-before* | `warning: hub slow\n` + G | 0 | `TextBefore` |
    | leading-space | ` ` + G | 0 | `TextBefore` |
    | text-after | G + `trailing\n` (G with its `\n`) | 0 | `TextAfter` |
    | two-envelopes | G + G | 0 | `TextAfter` |
    | blank-line-after | G + `\n` (G with its `\n`, so `}\n\n`) | 0 | `TextAfter` |
    | array | `[1]` | 0 | `NotAnObject` |
    | missing-error | G without `"error"` | 0 | `MissingKey("error")` |
    | missing-data | G without `"data"` | 0 | `MissingKey("data")` |
    | detail-key* | G plus `"detail":"x"` | 0 | `UnknownKey("detail".into())` |
    | schema-2* | G with `"schema_version":2` | 0 | `SchemaVersion` |
    | schema-string | G with `"schema_version":"1"` | 0 | `SchemaVersion` |
    | exit-4 | G | 4 | `ExitCodeUnknown(4)` |
    | exit-negative | G | -1 | `ExitCodeUnknown(-1)` |
    | ok-true-exit-3* | G (ok true) | 3 | `OkDisagreesWithExit` |
    | ok-false-exit-0* | F | 0 | `OkDisagreesWithExit` |
    | ok-not-bool | G with `"ok":"true"` | 0 | `OkDisagreesWithExit` |
    | error-while-ok* | G with `"error":{"code":"timeout","message":"x"}` | 0 | `ErrorWhileOk` |
    | no-error | F with `"error":null` | 1 | `NoErrorWhenFailed` |
    | error-not-object | F with `"error":"boom"` | 1 | `NoErrorWhenFailed` |
    | data-on-failure | F with `"data":{}` | 1 | `DataOnFailure` |
    | no-message | F with `"error":{"code":"timeout"}` | 1 | `MissingKey("error.message")` |
    | error-detail | F with an extra `"detail":"x"` inside `error` | 1 | `UnknownKey("error.detail".into())` |
    | code-not-kebab | F with `"code":"Pane_Not_Found"` | 1 | `CodeNotKebab("Pane_Not_Found".into())` |
    | code-not-string | F with `"code":5` | 1 | `CodeNotKebab("5".into())` |
    | two-line-message* | F with `"message":"line one\nline two"` (a JSON `\n` escape) | 1 | `MessageNotOneLine` |
    | cr-message | F with `"message":"a\rb"` | 1 | `MessageNotOneLine` |
    | empty-message | F with `"message":""` | 1 | `MessageNotOneLine` |
    | class-mismatch | F (`timeout`) | 3 | `ClassDisagreesWithExit` |
    | usage-at-1 | F with `"code":"usage"` | 1 | `ClassDisagreesWithExit` |
  - `the_mutant_tables_cover_every_fault`: the expected faults of this table and of the AC3 stream table, taken together,
    include every `EnvelopeFault` variant. The test maps each fault to its name through a helper with an **exhaustive
    `match` and no `_` arm**, so a variant added later fails to compile until a row covers it. It then compares the set of
    names with all 17.
- [ ] **AC3, NDJSON** (`tests/envelope_test.rs`):
  - `a_stream_of_ok_lines_at_exit_0_is_accepted`: three success lines at exit 0 give a `Vec` of 3 envelopes, all `ok`, in
    order.
  - `a_stream_ending_in_one_failure_is_accepted_at_its_exit_code`: `[ok, ok, fail(timeout)]` at exit 1,
    `[ok, fail(pane-not-found)]` at exit 3 and `[fail(usage)]` at exit 2 are accepted. The last envelope is the failure.
  - `every_stream_mutant_is_rejected_with_its_fault`: a table asserting `check_ndjson(stdout, exit) == Err(expected)`:
    `""` exit 0 gives `EmptyStream`; `"\n"` exit 0 gives `EmptyStream`; `[ok, fail, ok]` exit 3 gives `NotLastFailure`;
    `[fail, ok]` exit 0 gives `NotLastFailure`; `[ok, fail]` exit 0 gives `OkDisagreesWithExit`; `[ok, ok]` exit 3 gives
    `OkDisagreesWithExit`; `ok\n{\nok\n` exit 0 gives `NotJson`; `ok\n\nok\n` (a blank line) exit 0 gives `NotJson`; a
    line with `"detail"` gives `UnknownKey("detail".into())`; `[ok, fail(timeout)]` exit 3 gives `ClassDisagreesWithExit`;
    `[ok]` exit 5 gives `ExitCodeUnknown(5)`.
- [ ] **AC4, unit tests in the module** (`src/envelope.rs`, `mod tests`):
  - `exit_code_is_checked_before_stdout`: `check_envelope("{", 7) == Err(ExitCodeUnknown(7))`, and the same for
    `check_ndjson("", 7)`.
  - `only_one_final_newline_is_stripped`: G with `\n` is accepted; G with `\r\n` gives `TextAfter`; G with `\n\n` gives
    `TextAfter`.
  - `broken_json_that_starts_with_a_brace_is_not_text_before`: `{"a": {"b":1}` gives `NotJson`; `{partial\n` + G gives
    `NotJson`; `x` + G gives `TextBefore`; `"   "` gives `NotJson`; `null` gives `NotAnObject`.
  - `every_fault_displays_as_one_line`: for one value of every variant, `to_string()` is non-empty and contains no `\n`.
- [ ] **AC5, a doctest** in the module doc of `envelope.rs`: it checks one compact success envelope at exit 0 (`is_ok()`)
  and asserts the same stdout at exit 3 gives `Err(EnvelopeFault::OkDisagreesWithExit)`. It runs under `cargo test --doc`.
- [ ] **AC6, dependencies.** `crates/holler-pane-testkit/Cargo.toml` `[dependencies]` gains
  `serde_json = { workspace = true }`, with a comment line above it saying it parses the envelopes for `envelope.rs`
  (`check_envelope`, `check_ndjson`). There is no feature list, so no `# for` marker is needed, but the comment names the
  consumer anyway, as the `holler-pane` line does. `cargo tree -p holler-pane-testkit -e normal --depth 1` lists only
  `holler-pane` and `serde_json` (today it lists only `holler-pane`).
  `cargo tree -p holler-pane-testkit -e normal | grep -cE "holler-(cli|hub)"` prints `0`. `cargo machete` passes.
- [ ] **AC7, docs.** The module doc of `envelope.rs` replaces the stub text. It describes the two functions, the check
  order, and that the checker parses with `serde_json` alone and classifies with `holler_pane::error::class_of`, with no
  table of its own. `lib.rs:5` changes from "It depends on `holler-pane` alone" to say it depends on `holler-pane` and
  `serde_json` (for the envelope checker). That is a doc-only edit; nothing else in `lib.rs` changes.
- [ ] **AC8, CHANGELOG.** An entry under `## [Unreleased]` / `### Enhancements`, after the #676 entry (the last bullet,
  ending at `CHANGELOG.md:89`). In plain words: the test kit can now check a verb's `--format=json` output (one envelope,
  or an NDJSON stream where only the last line may be a failure) against its exit code, and it names the first rule the
  output breaks; it is test code only; link [#681](https://github.com/Performant-Labs/holler/issues/681). No personal
  names.
- [ ] **AC9, guards.** These all pass: `cargo build --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` (workspace lints deny `unwrap_used`, `expect_used`, `panic`,
  `unreachable`, `dead_code`, `too_many_lines` over 100 and `cognitive_complexity` over 15, also in test code; split long
  checks and tables into helpers), `cargo test -p holler-pane-testkit` (unit, integration and doc tests),
  `cargo test --workspace`, `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` and
  `bash scripts/test-hooks.sh`. `rustfmt --check --edition 2021` passes on `src/envelope.rs`, `tests/envelope_test.rs` and
  `src/lib.rs`. `src/envelope.rs` stays under 600 lines. Library code has no `unwrap`, `expect`, `panic!`, `unreachable!`
  or indexing that can panic (use `get`, `strip_suffix`, `split_at` behind a char-boundary-safe index from
  `char_indices`/`match_indices`). Prefer no `#[allow]` in the test file: use `assert_eq!` and `Value::to_string()`, which
  cannot fail. If one is needed, copy the house line with `// #681`.
- [ ] **AC10, scope.** `git diff --name-only origin/main...HEAD` lists only the Blast-radius paths below.

## Files

- `crates/holler-pane-testkit/src/envelope.rs` (**extend the stub**): the API, the private helpers and `mod tests`.
- `crates/holler-pane-testkit/tests/envelope_test.rs` (new): AC1 to AC3. Build test stdout with `serde_json::json!` and
  `Value::to_string()` plus `"\n"`, or with literal strings for the framing rows. Use one `good_success()` and one
  `good_failure(code)` builder, plus a `with(base, key, value)` / `without(base, key)` mutator, so each row is one line.
- `crates/holler-pane-testkit/Cargo.toml`: add `serde_json`. **`Cargo.lock`**: the one added `"serde_json"` line under
  `holler-pane-testkit` (cargo writes it).
- `crates/holler-pane-testkit/src/lib.rs`: line 5, doc only (AC7).
- `CHANGELOG.md`.

**Reuse map (extend, do not duplicate):** `holler_pane::error::is_valid_code` is the only code validator, and `class_of`
plus `ErrorClass::exit_code` is the only code-to-exit table (no list of codes or exits in the testkit). `ALL_CODES` drives
AC1's exhaustive loop. `serde_json`'s `StreamDeserializer` does the framing (no hand-written JSON scanner). The pattern of
the `conformance` module (one `(id, case)` table, results that name the failing row) is the model for the mutant tables,
but `conformance::run_cases` is **not** used: it takes a fresh subject per case and returns `CaseFailure`s, which does not
fit a pure function of `(stdout, exit)`. Do not add a `conformance/envelope.rs`: the issue puts everything in
`envelope.rs`, and slice a's `conformance/mod.rs` is closed to later slices.

## Decisions made in this brief

1. **No new fault variant.** The issue's 17 variants are kept exactly. The cases it does not name are mapped to the closest
   existing variant: a non-boolean `ok` gives `OkDisagreesWithExit` (it is not the boolean the exit code requires); a
   non-object `error` on failure gives `NoErrorWhenFailed`; a missing or extra member of `error` gives
   `MissingKey("error.code" | "error.message")` / `UnknownKey("error.<key>")`; a non-string `code` gives
   `CodeNotKebab(<its JSON text>)`; a non-string, empty or blank `message` gives `MessageNotOneLine`; a `schema_version`
   of the wrong type gives `SchemaVersion`.
2. **The check order is fixed** (the tables above), with the exit code first, so a mutant breaking two rules has one
   predictable fault and the NDJSON rule can reuse `check_envelope`.
3. **Framing.** Only one final `\n` is optional. Any other byte after the value (`\r`, spaces, a blank line) is `TextAfter`.
   Leading whitespace or text is `TextBefore` only when a later `{` begins valid JSON and the output does not itself start
   with `{`. Otherwise it is `NotJson`, so broken JSON is never reported as text before.
4. **`data` on success is unconstrained** (any JSON value, `null` included), because the CLI serializes `Some(())` as
   `null`. `schema_version` is `u64` (the issue's type; the CLI's `u32` serializes the same).
5. **NDJSON.** Lines before the last are checked at exit 0, and their `ok: false` is renamed `NotLastFailure`. The last
   line is checked at the process's exit code. `""` and `"\n"` are `EmptyStream`. A blank line inside the stream is
   `NotJson`. The fault does not carry a line number, because the issue fixes the signature.
6. **`Display` and `std::error::Error` for `EnvelopeFault`** are added (beyond the issue's sketch), so a caller's test can
   use `?` or print the fault. These are additions, not changes to a fixed signature.
7. **Blast radius adds `Cargo.lock` and `lib.rs` line 5** (doc only), which are outside the issue's list. Cargo rewrites the
   lock for the new dependency. The `lib.rs` sentence "depends on `holler-pane` alone" would become false. No other
   `lib.rs` line changes, so slices c to e (which do not edit `lib.rs`) cannot conflict with it.
8. **Unit tests in the module plus an integration test.** The integration file is the public-API contract and holds the
   mutation tables. The unit tests pin framing edge cases and `Display`. Both call only the public functions, so T can
   write them before F picks the private helper names.

## Out of scope

Using the checker from `holler-cli` or `holler-hub` tests (the wave-3 verb stories and #649 do that). A cross-check that
the CLI's `emit` output passes the checker, which needs `holler-cli` tests and is outside this blast radius. Any change to
`holler-cli/src/output.rs`, `holler-pane` or ADR-0021. Slices c to e (#682, #683, #684). The roster's legacy `--json`
shape (#648). Line numbers in faults. Checking `data`'s shape per verb (each verb's own tests do that).

## Test plan

RED (T): write `tests/envelope_test.rs` and the `mod tests` and doctest in `envelope.rs`. They fail to compile because
`check_envelope`, `check_ndjson`, `Envelope`, `EnvelopeError` and `EnvelopeFault` do not exist, and the test file's
`serde_json` import needs the dependency. T adds the `Cargo.toml` line, or confirms that the only RED errors are the
missing items and the missing crate. Run `cargo test -p holler-pane-testkit --test envelope_test` and
`cargo test -p holler-pane-testkit --lib`. Make sure no RED comes from a typo. GREEN (F): fill `envelope.rs`, add the
dependency, fix `lib.rs:5`, add the CHANGELOG entry. Then run the full guard list of AC9.

## Risks

- **Framing details are the likeliest first-pass failure.** `StreamDeserializer` skips leading whitespace, so rule 1(a)
  must check the first byte itself. `byte_offset()` must be read after the first `next()`. Slicing `&body[i..]` must use
  an index that comes from `char_indices` or `match_indices('{')`, which is always a char boundary.
- **Cognitive complexity 15 and 100 lines per function** apply to the checker and to the test tables. Keep the rules in
  small helpers (framing, exact keys, failure body) and the tables in their own functions.
- The CLI's `one_line` can turn an all-whitespace message into `""`, which this checker rejects (`MessageNotOneLine`). That
  is intended (ADR-0021: every message is one line), and the CLI tests own that path.
- If a slice c to e also edits `lib.rs` line 5, the merge conflict is one doc line.

## Blast radius

`crates/holler-pane-testkit/src/envelope.rs`, `crates/holler-pane-testkit/tests/envelope_test.rs` (new),
`crates/holler-pane-testkit/Cargo.toml`, `Cargo.lock`, `crates/holler-pane-testkit/src/lib.rs` (line 5, doc only),
`CHANGELOG.md`, `docs/handoffs/681*` (pipeline artifacts). Not changed: `holler-pane`, `holler-cli`, `holler-hub`, the root
`Cargo.toml`, `conformance/`, any other testkit module, any ADR or golden file.
