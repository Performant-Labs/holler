# Brief: #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

Repo: Performant-Labs/holler. Issue: #676 (epic #633, "Decisions 2026-10-09" item 5). Rigor: in-session. UI surface: no.
Kind: feature (small).

**Branch:** `issue-676-implementation`, based on `55dba00` (#637, #669, #670 and ADR-0021 merged). **Design (D):** N/A.
**Decision record:** ADR-0021 section 9 and "Decisions taken" item 5 (operator, 2026-10-09). The issue is the source of
truth; where this brief differs from it, the issue wins.

**Size check:** one function and one enum in `holler-pane`, a three-line change to one function in `output.rs`, about six
updated assertions, two new tests, doc and ADR text. Expected to fit one run; no split.

## Problem

ADR-0021 section 9 decided that `holler pane` and `holler profile` exit 0 ok, 1 runtime failure, 2 usage, 3 refusal, in text
and JSON mode alike, and that one function in `holler-pane` decides which `PaneError` code is a refusal and which a failure.
#670 merged before that function existed, so its output module maps every non-usage error to exit 1. This story writes the
function, routes the output module's exit code through it, and fixes the tests and text that still say "1 refused or failed".
It must merge before the wave 3 verb stories (#643 onward) so they assert the final exit codes.

## The decision (restated exactly)

- Exit **0** ok, **1** runtime failure, **2** usage, **3** refusal. Same code in text mode and JSON mode.
- In JSON mode the envelope's `ok` is `false` for exit 1, 2 and 3 alike (and `data` is `null`, `error` is set).
- A **refusal** is a request the system understood and declined, working as designed: nothing was wrong with the system.
  A **failure** is something that went wrong while doing the work (a timeout, an unreachable hub or adapter, a conflict
  between writers). `usage` stays its own class and exit 2.
- One function in `holler-pane` classifies; the CLI and (later) the test kit (#638) both call it.

## Evidence (verbatim, as of `55dba00`)

The exit code is chosen in exactly one place, by comparing the code string with `usage`:
```
crates/holler-cli/src/output.rs:330-341
/// The exit code of an error: 2 for `usage`, 1 for every other.
fn exit_code(error: &ErrorBody) -> i32 {
    if error.code
        == ErrorCode::from(&PaneError::Usage {
            message: String::new(),
        })
    {
        2
    } else {
        1
    }
}
```
Both formats call it, so changing it changes both alike (`emit_text` at `output.rs:277`, `emit_json` at `output.rs:286`):
```
crates/holler-cli/src/output.rs:275-277
        Err(error) => {
            let written = write_line(sink.err, &format!("error: {}", error.message));
            settle(written, exit_code(&error))
crates/holler-cli/src/output.rs:285-291
        Err(error) => {
            let code = exit_code(&error);
            ...
            settle(write_envelope(sink, &Envelope::<()>::failure(error)), code)
```
`emit_stream` returns the first non-zero code `emit` returns (`output.rs:220-233`), `emit_error` and `emit_usage_error` go
through `emit` (`output.rs:236-244`), and `settle` (`output.rs:322-328`) only turns a failed write on a success into 1, so
a refusal whose write fails stays 3. `ErrorBody` carries the code as a string, not a `PaneError`, and tests build it from
any valid code:
```
crates/holler-cli/src/output.rs:122-128
pub struct ErrorBody {
    pub code: ErrorCode,      // ErrorCode(String), validated by holler_pane::error::is_valid_code
    pub message: String,
}
crates/holler-cli/tests/pane_verbs/output_api.rs:31-36
fn error_body(code: &str, message: &str) -> ErrorBody {
    ErrorBody { code: ErrorCode::new(code).expect("a valid code"), message: message.to_string() }
```
So the classifier must take the **code string** (`&str`), not a `PaneError`.

The exit code reaches the process unchanged; `main.rs` needs no edit:
```
crates/holler-cli/src/main.rs:112      std::process::exit(dispatch(&cli, choice));
crates/holler-cli/src/main.rs:225-227  Command::Pane(cmd) => run_with_stdio(choice.format, |ctx| holler_cli::pane::run(cmd, ctx)),
                                       Command::Profile(cmd) => run_with_stdio(choice.format, |ctx| holler_cli::profile::run(cmd, ctx))
crates/holler-cli/src/main.rs:268      Err(error) => output::emit_error(&mut sink, format, ErrorBody::from(&error)),
```
The closed codes are one private enum with an exhaustive `as_str` and a `parse` (the single source `ALL_CODES` is built from):
```
crates/holler-pane/src/error.rs:33-34    const CODE_COUNT: usize = 22;
crates/holler-pane/src/error.rs:40-63    pub(crate) enum PaneCode { NotImplemented, Usage, GridAmbiguous, GridOutOfRange, CommandNotArgv,
                                         EnvNameInvalid, GenerationConflict, ProbeFailed, ProfileConflict, ProfileNotFound,
                                         ProfileExists, ProfileHasLivePanes, PaneNotInProfile, PaneInOtherProfile,
                                         ProfileSecretRefused, HerdrVersionUnsupported, Timeout, PaneNotFound, SessionNotFound,
                                         StoreCorrupt, Unavailable, ProfileDrift }
crates/holler-pane/src/error.rs:121-123  pub(crate) fn parse(text: &str) -> Option<PaneCode> { PaneCode::ALL.iter().copied().find(|c| c.as_str() == text) }
crates/holler-pane/src/error.rs:141      pub const ALL_CODES: &[&str] = &CLOSED_CODES;
crates/holler-pane/src/error.rs:158      pub const fn is_valid_code(code: &str) -> bool {
```
Any other well-formed code is an open code, carried by `PaneError::Refused` (`error.rs:398`); a malformed code read off the
wire becomes `unavailable` (`error.rs:506-519`, `from_wire`). The crate keeps its error vocabulary under `error::`
(`crates/holler-pane/src/lib.rs:61`), and the test kit may not depend on `holler-cli`
(`crates/holler-pane-testkit/src/lib.rs:5-6`).

The rule as ADR-0021 states it, and the table it says this change adds:
```
docs/adr/ADR-0021.md:358-365
- **Exit codes are the same in both formats (operator, 2026-10-09):** 0 ok, 1 runtime failure, 2 usage, 3 refusal. A
  refusal is a request the system understood and declined, working as designed, ... A failure is something that went
  wrong while doing the work (a timeout, an unreachable hub or adapter, a conflict between writers). ... Which `PaneError`
  code is a refusal and which a failure is decided once, in one function in `holler-pane`, in the change that moves the
  output module to exit 3 (see "Decisions taken", item 5). This **amends the epic's output contract**, which said refusals exit 1.
docs/adr/ADR-0021.md:506-508   5. **Refusals exit 3 (section 9), not 1.** ... its output module maps every error to exit 1, so a change follows it; ...
docs/adr/ADR-0003.md:97        - **Exit codes:** `0` ok; `1` runtime failure (unreachable hub, wire error, interrupted turn); `2` usage/ambiguity; `3` fail-closed policy refusal (e.g. non-loopback bind, invalid `--debug`).
```
ADR-0021 already says generation-conflict and profile-conflict exit 1 (`ADR-0021.md:271`, `:287`, `:294`), and that a
legacy verb's `--pane`/`--profile` "not implemented" is exit 1 (`ADR-0021.md:196`, `ADR-0003.md:93`); both stay true below.

Text that says "1 refused or failed" today (the residue to fix):
```
crates/holler-cli/src/output.rs:10-12           The exit code is the same in both modes: `0` ok, `1` refused or failed, `2` usage. ...
crates/holler-cli/src/output.rs:197             /// Print one result and return the exit code: 0 ok, 1 error, 2 for an error coded `usage`.
crates/holler-cli/src/output.rs:330             /// The exit code of an error: 2 for `usage`, 1 for every other.
crates/holler-cli/src/pane/mod.rs:51            /// Run a `holler pane` verb and return its exit code (0 ok, 1 refused or failed, 2 usage).
crates/holler-cli/src/profile/mod.rs:39         /// Run a `holler profile` verb and return its exit code (0 ok, 1 refused or failed, 2 usage).
crates/holler-cli/src/pane/args.rs:5-6          ... so a bad `--grid` is a refusal with a code a script can match (exit 1) and not a clap usage error.
crates/holler-cli/tests/pane_verbs/spec_flags.rs:3   `holler-pane` guards and their stable codes. A refusal is exit 1 in the verb ...
crates/holler-cli/tests/pane_verbs/output_api.rs:6-8 ... Exit codes are the same in both formats: 0 ok, 1 refused or failed, 2 usage ...
```
#670's tests that assert exit 1 for a code this brief makes a refusal:
```
crates/holler-cli/tests/pane_verbs/output_api.rs:135-145  emit_error_in_text_mode_...   error_body("pane-not-found", ...)  assert_eq!(code, 1);  (line 144)
crates/holler-cli/tests/pane_verbs/output_api.rs:150-159  emit_error_in_json_mode_...   error_body("pane-not-found", ...)  assert_eq!(code, 1);  (line 159)
crates/holler-cli/tests/pane_verbs/output_api.rs:190-213  emit_exits_2_for_a_usage_coded_error_in_both_formats_and_1_for_any_other
                                                          error_body("probe-failed", "x") ... assert_eq!(code, 1, "...any other error exits 1");  (line 211)
```
Exit-1 assertions that stay as they are (their codes are failures or a failed write): `output_api.rs:279` (`unavailable`),
`:326` (`unavailable`), `:371` and `:386` (failed write / unencodable data); every stub test, which asserts
`not-implemented` exits 1 (`tests/verb_harness/mod.rs:109`, `:121`; `tests/pane_verbs/launch.rs:22`;
`tests/pane_verbs/process/stub.rs:84`, `:126`); `tests/pane_verbs/process/legacy_verbs.rs:29` (plain-text legacy refusal,
not the output module). No stub validates its flags, so no process-level test can reach a refusal code yet
(`crates/holler-cli/src/pane/launch.rs:25-27` returns `not_implemented(STORY)` before reading its args).

Precedent relevant to one close call: the legacy `hold`/`release` verbs exit 1 for an unknown session
(`docs/protocol/v2.md:757`, "An unknown session is exit 1 (`unknown session`)").

## The classification (decided here)

One public function, `holler_pane::error::class_of(code: &str) -> ErrorClass`, with one public enum
`ErrorClass { Usage, Refusal, Failure }` and `ErrorClass::exit_code(self) -> i32` (Usage 2, Refusal 3, Failure 1). Inside
`class_of`: a closed code (`PaneCode::parse`) is classified by **one exhaustive `match` over `PaneCode` with no `_` arm**
(grouped with `|`), so a new `PaneCode` variant without an arm does not compile; any other well-formed code
(`is_valid_code`) is an open `Refused` code and is a **Refusal**; a malformed code is a **Failure** (matching `from_wire`,
which turns a garbled code into `unavailable`).

| Code | Class (exit) | Reason |
|---|---|---|
| `usage` | Usage (2) | The request is malformed; ADR 0003's exit 2. |
| `grid-ambiguous` | Refusal (3) | The `GridPos` guard declined text it cannot read one way. |
| `grid-out-of-range` | Refusal (3) | The `GridPos` guard declined a row or column outside its bounds. |
| `command-not-argv` | Refusal (3) | Policy: a stored command is an argv array, never a shell string. |
| `env-name-invalid` | Refusal (3) | The `EnvVarName` guard declined a blank or whitespace name. |
| `profile-secret-refused` | Refusal (3) | Policy I7: a profile holds names, never values. |
| `profile-exists` | Refusal (3) | The name is taken; nothing went wrong. |
| `profile-has-live-panes` | Refusal (3) | Delete is declined while panes of the profile are live. |
| `pane-not-in-profile` | Refusal (3) | The `--profile` scope check declined a pane outside the profile. |
| `pane-in-other-profile` | Refusal (3) | Ownership policy: a pane belongs to at most one profile (C3). |
| `probe-failed` | Refusal (3) | The health gate declined to launch or relaunch, working as designed. **Close call.** |
| `herdr-version-unsupported` | Refusal (3) | Fail-closed version gate, like ADR 0003's exit 3. **Close call.** |
| `profile-not-found` | Refusal (3) | The named profile does not exist; the request was understood and declined. **Close call.** |
| `pane-not-found` | Refusal (3) | The named pane does not exist. **Close call.** |
| `session-not-found` | Refusal (3) | The named harness session does not exist. **Close call.** |
| `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
| `profile-conflict` | Failure (1) | The profile moved after the live change (I8 write order); a race. |
| `timeout` | Failure (1) | The I5 bound ran out while doing the work. |
| `unavailable` | Failure (1) | The hub, Herdr socket or harness cannot be reached (also a garbled reply). |
| `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
| `not-implemented` | Failure (1) | The verb cannot do the work yet; keeps every stub at exit 1. **Close call.** |
| `profile-drift` | Failure (1) | A finding kind, not an error a port returns; if one ever surfaces, live state disagrees with its spec. **Close call.** |
| open `Refused` code (any well-formed non-closed code) | Refusal (3) | `Refused` is the declared channel for codes a verb or adapter owns; the open codes already planned (#645 pane not idle, holds a question, unhealthy server, orchestrator's own pane; #646 unhealthy pane, SHOWN differs from DRIVEN) are all refusals. |
| malformed code | Failure (1) | Cannot be trusted; the same rule as `from_wire`. Unreachable from `ErrorBody`, whose code is always valid. |

All 22 rows are the codes of `ALL_CODES` (`error.rs:67-90`); no code is invented.

## Acceptance criteria

Test names are what T should author (RED first).

1. **Classifier exists and is exhaustive.** `holler_pane::error::{class_of, ErrorClass}` exist, `ErrorClass` derives
   `Debug, Clone, Copy, PartialEq, Eq`, and `class_of` contains one `match` over `PaneCode` with no wildcard arm (A checks
   this in the diff review). New test file `crates/holler-pane/tests/error_class_test.rs`:
   - `every_closed_code_has_the_decided_class`: a literal table of the 22 rows above (code string -> `ErrorClass`);
     asserts the table's key set equals `ALL_CODES` (so a code added to `ALL_CODES` fails here until the test names its
     class) and `class_of(code)` equals the table's class for every row.
   - `an_open_code_is_a_refusal`: `class_of("quota-exceeded") == ErrorClass::Refusal`, and the code of a
     `PaneError::Refused` built from `RefusalCode::from_static` classifies as `Refusal`.
   - `a_malformed_code_is_a_failure`: `class_of("Not A Code")` and `class_of("")` are `Failure`.
   - `exit_codes_by_class`: `Usage.exit_code() == 2`, `Refusal.exit_code() == 3`, `Failure.exit_code() == 1`.
2. **Output module uses it.** `output.rs` `exit_code` returns `class_of(error.code.as_str()).exit_code()`; it no longer
   compares with `usage` itself, and holds no second table. Extend `crates/holler-cli/tests/pane_verbs/output_api.rs`:
   - `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats`: for every code in `ALL_CODES` and for
     `Format::Text` and `Format::Json`, `emit` with `Err(error_body(code, "m"))` returns `class_of(code).exit_code()`; the
     text and JSON codes are equal; in JSON mode the one envelope has `ok == false`, `data == null`, `error.code == code`.
   - `a_refusal_exits_3_and_a_failure_exits_1_in_both_formats`: literal spot checks, both formats:
     `pane-in-other-profile`, `profile-secret-refused`, `command-not-argv`, `grid-ambiguous` and an open code
     (`quota-exceeded`) exit 3; `timeout`, `generation-conflict`, `unavailable` exit 1; `usage` exits 2.
   - `emit_stream_exits_3_on_a_refusal_item`: a stream `[Ok, Err(refusal)]` returns 3 and keeps the earlier line.
   - Update the three tests listed in Evidence: `pane-not-found` now exits 3 (lines 144, 159); rename
     `emit_exits_2_for_a_usage_coded_error_in_both_formats_and_1_for_any_other` to say 3 for a refusal and assert
     `probe-failed` exits 3 (line 211); fix the header (lines 6-8). The stub, legacy-verb and failed-write exit-1 tests are
     unchanged and still pass.
3. **Text residue fixed.** `grep -rn "refused or failed" crates/ docs/adr/` returns nothing; `output.rs:10-12`, `:197`,
   `:330`, `pane/mod.rs:51`, `profile/mod.rs:39`, `pane/args.rs:5-6` and `tests/pane_verbs/spec_flags.rs:3` state 0 ok,
   1 failure, 2 usage, 3 refusal (doc comments only in the three `src/pane|profile` files).
4. **ADR-0021 section 9** carries the classification table (code, class, one-line reason; the rows above) right after the
   "Exit codes are the same in both formats" bullet, names `holler_pane::error::class_of`, and replaces "in the change that
   moves the output module to exit 3" with a link to #676. "Decisions taken" item 5 (`ADR-0021.md:506-508`) gains "(done in
   #676)". **ADR 0003** line 97 gains one sentence: the `pane` and `profile` verbs (epic #633, ADR-0021 section 9) use the
   same four codes in text and JSON mode, 3 for a refusal and 1 for a runtime failure, and which error code is which is
   decided by `holler_pane::error::class_of` (#676). Write no inline code span that starts with `holler ` (docs_cli_test
   parses those, `crates/holler-cli/tests/docs_cli_test.rs:5-10`).
5. **CHANGELOG.** An entry under `## [Unreleased]` / `### Enhancements`, after the #670 entry: `holler pane` and
   `holler profile` now exit 3 for a refusal and 1 for a runtime failure, the same in text and JSON mode, and the envelope's
   `ok` is false for both; the code-to-class table is in ADR-0021 section 9; stubs still exit 1 (`not-implemented` is a
   failure); link [#676](https://github.com/Performant-Labs/holler/issues/676).
6. **Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` and `bash scripts/test-hooks.sh` pass. The new
   `error_class_test.rs` passes `rustfmt --check --edition 2021`; existing files are not reformatted (their changed hunks add
   no rustfmt deviation). `error.rs` stays under 900 lines (617 today; lint already warns at 600, which is accepted).
7. `git diff --name-only origin/main...HEAD` lists only Blast-radius paths.

## Files

- `crates/holler-pane/src/error.rs` (extend): `ErrorClass`, `ErrorClass::exit_code`, `class_of`; one bullet in the module doc
  (lines 1-25) naming them. No new module (that would edit `lib.rs`, which #637 keeps closed to later stories), no
  re-export from `lib.rs` (the vocabulary beyond `PaneError` stays under `error::`, `lib.rs:61`).
- `crates/holler-pane/tests/error_class_test.rs` (new; holler-pane has default `autotests`, so no manifest entry). If it needs
  `expect`/`unwrap`, copy the allow line of `error_test.rs:1` with `// #676`; prefer `assert_eq!` and need none.
- `crates/holler-cli/src/output.rs` (edit): `exit_code` body and the three doc comments; import `holler_pane::error::class_of`.
- `crates/holler-cli/src/pane/mod.rs`, `profile/mod.rs`, `pane/args.rs` (doc comment lines only).
- `crates/holler-cli/tests/pane_verbs/output_api.rs` (extend), `tests/pane_verbs/spec_flags.rs` (doc line only).
- `docs/adr/ADR-0021.md`, `docs/adr/ADR-0003.md`, `CHANGELOG.md`.

Reuse map (extend, do not duplicate): `PaneCode::parse` and `is_valid_code` for the lookup (no second code list, no second
validator); `error_body`, `with_sink` and `one_envelope` in `output_api.rs` for the new CLI tests; `ALL_CODES` as the
iteration source in both test files.

## Decisions already made (MO)

1. **The table above**, every row decided here. The close calls for the operator are listed under Risks; the run does not
   wait on them (changing a row later is one arm and one test row).
2. **Signature: `class_of(code: &str)`, not a method on `PaneError`.** The CLI has only `ErrorBody`'s code string, and the
   test kit (#638) will read codes out of envelopes. No `PaneError::class` is added (no caller).
3. **Three classes, `Usage` included**, so the one `match` covers all 22 closed codes and the CLI maps a class to a number
   with no special case for `usage`.
4. **The exit number lives on `ErrorClass` (`exit_code`) in `holler-pane`.** The issue says the CLI and the test kit both use
   the function, and the test kit cannot depend on `holler-cli`, so the class-to-number mapping is written once there.
   Exit 0 stays a literal in `output.rs`.
5. **Open `Refused` codes are refusals; a malformed code is a failure** (see the table).
6. **`not-implemented` is a failure (exit 1)**, so every stub and the plain-text legacy `--pane`/`--profile` refusal keep
   exit 1, and #670's stub tests do not change.
7. **Blast radius includes three doc-comment lines in `pane/mod.rs`, `profile/mod.rs` and `pane/args.rs`** (outside the
   issue's list): they state the old rule; comment-only, no code. `main.rs` is **not** edited: it applies whatever the verb
   returns (`main.rs:112`).
8. **holler-pane "frozen" rule (`lib.rs:31`)**: this story adds items and changes no existing signature or code; the
   addition is the epic's own 2026-10-09 decision (ADR-0021 section 9), so no further amendment is needed.

## Out of scope

Any verb behaviour; process-level refusal tests (no verb returns a refusal until wave 3); the test kit's envelope helper
(#638 calls `class_of`); legacy verbs' exit codes (`say`/`interrupt`/`answer`/`roster` `--pane`/`--profile` stay plain-text
exit 1; `hold`/`release` unchanged); `holler-proto`'s wire `Code` table, protocol v2, the closed 22-row wire catalog and every
golden file (unchanged); `cli-surface` fixtures (no surface change); the epic body (already amended, line "exit codes
identical in both formats (0 ok, 1 failure, 2 usage, 3 refusal)").

## Test plan

RED (T): add `error_class_test.rs` (fails to build: `class_of`/`ErrorClass` do not exist) and the new and updated
`output_api.rs` cases (the refusal assertions get 1 today, and the build fails on the `class_of` import). Confirm RED with
`cargo test -p holler-pane --test error_class_test` and `cargo test -p holler-cli --test pane_verbs`, and that no RED is a
typo or a missing target. GREEN (F): add the enum and function, change `exit_code`, fix the doc comments, ADRs and
CHANGELOG. Then the full guard list of AC 6.

## Risks and close calls (for the operator)

- **Not-found codes (`profile-not-found`, `pane-not-found`, `session-not-found`) as refusals.** They fit the rule (nothing
  went wrong; the request named something that does not exist), but the legacy `hold`/`release` exit 1 for an unknown
  session (`v2.md:757`). If the operator prefers parity with the legacy verbs, the three move to Failure.
- **`probe-failed` as a refusal.** The probe gate declining is working as designed (ADR-0021 lists it as refusing launch),
  but its cause is often the environment (a binary missing). Failure is the alternative.
- **`herdr-version-unsupported` as a refusal.** A fail-closed gate, like ADR 0003's exit 3; the alternative is to treat an
  unsupported installed Herdr as an environment failure.
- **`not-implemented` as a failure.** Arguably "understood and declined", but Refusal would move every stub and #670's stub
  tests to exit 3 while the legacy plain-text `not implemented` stays 1.
- **`profile-drift` as a failure.** Never returned as an error today; the class only matters if a later story surfaces it.
- `error.rs` is past the 600-line warning; the addition (about 50 lines) keeps it well under 900.
- `dead_code = "deny"`: `ErrorClass::exit_code` and `class_of` are `pub` and used by `output.rs`; no `pub(crate)` helper
  without a caller.

## Blast radius

`crates/holler-pane/src/error.rs`, `crates/holler-pane/tests/error_class_test.rs` (new), `crates/holler-cli/src/output.rs`,
`crates/holler-cli/src/pane/mod.rs`, `crates/holler-cli/src/profile/mod.rs`, `crates/holler-cli/src/pane/args.rs` (doc
comments only), `crates/holler-cli/tests/pane_verbs/output_api.rs`, `crates/holler-cli/tests/pane_verbs/spec_flags.rs` (doc
line only), `docs/adr/ADR-0021.md`, `docs/adr/ADR-0003.md`, `CHANGELOG.md`, `docs/handoffs/676*` (pipeline artifacts). Not
changed: `main.rs`, any `Cargo.toml` or `Cargo.lock`, `holler-pane/src/lib.rs`, `holler-hub`, `holler-proto`, any golden file
or `cli-surface` fixture.
