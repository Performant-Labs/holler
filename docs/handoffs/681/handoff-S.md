# Handoff-S: Phase 10 (spec audit) - #681 the JSON-envelope checker in `holler-pane-testkit` (slice b of #638)

**Date:** 2026-10-09
**Branch:** issue-681-implementation (at d6dafe7; merge base e410e9d = origin/main)
**Issue:** #681 (slice b of #638, epic #633). Rigor: in-session. UI surface: none.
**Brief:** `docs/handoffs/681-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md` (all in `docs/handoffs/681/`)
**Diff audited:** all of `git diff origin/main...HEAD`. I read in full the production code (`src/envelope.rs`, 457 lines), the `lib.rs` paragraph, the testkit `Cargo.toml`, the one `Cargo.lock` line and the `CHANGELOG.md` entry. I also read in full the tests: `tests/envelope_test.rs` (696 lines) and the `mod tests` and doctest in `envelope.rs`.

## A precondition

Met. `handoff-A.md` (plan review) is **PASS**, with 6 warns and no block. `handoff-A-dup.md` (anti-duplication) is **PASS**, with no block. Its two warns carry forward Phase 3's W-5 and W-6, which fall outside this blast radius.

The diff settles each Phase 3 warn that was meant for this run:
- W-1: the extra key is named by `keys().min()`, not by map order (`exact_members`, `envelope.rs:313-327`).
- W-2: the module doc has the key-set sentence (`envelope.rs:44-47`).
- W-3: `Display` quotes the string payloads with `{:?}` (`envelope.rs:162-179`), and a unit test with newline payloads pins it.

## T precondition

Met. `handoff-T-green.md` reports no blocking issues.

- **RED.** `handoff-T-red.md` records a compile failure (E0432, E0425 and E0433) on exactly the five missing public items. With a compiling stub in place, 9 of the 10 integration tests failed on their assertions. The tenth is the table-coverage guard, which passes by design.
- **GREEN.** `cargo test -p holler-pane-testkit` passes with both `serde_json::Map` backends. `cargo test --workspace` gives 1124 passed, 0 failed and 5 ignored, the same counts F reported.
- **Mutation spot-checks.** T-green made seven hand mutants: drop the class check, skip data-on-failure, widen the exit range, drop the blank-message check, accept any `schema_version`, and replace `min()` with `next_back()` or `next()`. The suite kills all of them. The `next()` mutant first survived under `preserve_order`. T repaired the suite with test rows only.
- **Checked with git:**
  - F's commit `8daf2ef` touches no file under `tests/`.
  - The `mod tests` and the doctest in `envelope.rs` are byte-identical between the RED commit `75ea14e` and HEAD.
  - T-green's commit `b2c9f21` touches only `tests/envelope_test.rs` (+25 lines) and T's own handoff, journal and evidence entries.

## Acceptance criteria

IT = `crates/holler-pane-testkit/tests/envelope_test.rs`. UT = the `#[cfg(test)] mod tests` in `crates/holler-pane-testkit/src/envelope.rs`.

**Brief (AC1 to AC10):**

| AC | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Good envelopes are accepted | IT `a_success_at_exit_0_is_accepted`: compares the whole `Envelope`, with object `data` and with `null` `data`. IT `a_failure_is_accepted_at_the_exit_code_of_its_class`: timeout at exit 1, usage at 2, pane-not-found at 3 and quota-exceeded at 3, each compared as a whole `Envelope` with its `EnvelopeError`. IT `the_adr_examples_are_accepted`: both strings match `ADR-0021.md:353-354` exactly (`grep -cF` = 1 each). IT `a_missing_final_newline_is_accepted`. IT `every_closed_code_is_accepted_only_at_the_exit_code_of_its_class`: every code in `ALL_CODES` at exits 1 to 3. `usage` is a closed code, so exit 2 is covered too. | MET |
| 2 | Every mutant is rejected with its fault, and the tables cover all 17 faults | IT `every_mutant_is_rejected_with_its_fault`: 48 rows. They include all 32 of the brief's rows under the same names and all 8 mutants the issue names. The assertion message names the row. IT `the_mutant_tables_cover_every_fault`: `fault_name` is an exhaustive `match` with no `_` arm, and the set of covered names equals the 17. | MET |
| 3 | NDJSON | IT `a_stream_of_ok_lines_at_exit_0_is_accepted`: 3 envelopes, `data` in order. IT `a_stream_ending_in_one_failure_is_accepted_at_its_exit_code`: the brief's three shapes, with the failure last. IT `every_stream_mutant_is_rejected_with_its_fault`: 12 rows, the brief's 11 plus a non-boolean `ok` before the last line. | MET |
| 4 | Unit tests in the module | UT `exit_code_is_checked_before_stdout`, `only_one_final_newline_is_stripped`, `broken_json_that_starts_with_a_brace_is_not_text_before` (all 5 of the brief's cases) and `every_fault_displays_as_one_line` (all 17 variants; the `UnknownKey` and `CodeNotKebab` payloads contain `\n` and `\r\n`). | MET |
| 5 | Doctest | In the module doc, `envelope.rs:49-58`. It asserts `is_ok()` at exit 0 and `Err(EnvelopeFault::OkDisagreesWithExit)` at exit 3. F and T-green record it passing. | MET |
| 6 | Dependencies | `Cargo.toml` adds `serde_json = { workspace = true }`. The comment above it names `envelope.rs` (`check_envelope`, `check_ndjson`). It has no feature list, and the crate has no `[dev-dependencies]`. Re-checked read-only: `cargo tree --offline -p holler-pane-testkit -e normal --depth 1` lists only `holler-pane` and `serde_json v1.0.151`, and the `holler-(cli\|hub)` count is 0. `cargo machete` passes (F, T-green). | MET |
| 7 | Docs | The stub text is gone. The module doc covers both functions, the 14 rules in order and the NDJSON rules. It says the checker parses with `serde_json` alone and classifies with `class_of`, with no table of its own. The `lib.rs` paragraph at line 5 now names `serde_json` (for the envelope checker). The change is doc only, and no item changes. The re-wrap is covered under Spec compliance. | MET |
| 8 | CHANGELOG | The entry is under `## [Unreleased]` / `### Enhancements`, right after the #676 bullet. In plain words it says the kit checks one envelope or an NDJSON stream in which only the last line may be a failure, and that it names the first rule broken. It says "Test code only", links #681 and names no person. `changelog-check: ok` (F, T-green). | MET |
| 9 | Guards | F and T-green recorded exit 0 for the build, `clippy --workspace --all-targets -D warnings`, the testkit tests (unit, integration and doc), the workspace tests, `machete`, `lint.sh`, `changelog-check.sh`, `test-hooks.sh` and `rustfmt --check` on the three files. My own read-only checks: `envelope.rs` is 457 lines (the limit is 600). Library code (lines 1 to 377) has no `unwrap()`, `expect(`, `panic!`, `unreachable!` or indexing. The diff adds no `#[allow]`. | MET |
| 10 | Scope | `git diff --name-only origin/main...HEAD` lists the six blast-radius files plus `docs/handoffs/681-brief.md` and `docs/handoffs/681/*`, and nothing else. | MET |

**Issue #681 acceptance:**

| Criterion | Proving test or evidence | Status |
|---|---|---|
| The suite rejects the named mutants (a mutation check), and a good envelope is accepted at each exit code | The named mutants are the rows `broken`, `text-before`, `ok-true-exit-3`, `ok-false-exit-0`, `error-while-ok`, `two-line-message`, `schema-2` and `detail-key`. The AC1 tests accept a good envelope at exits 0, 1, 2 and 3. Slice b has no fake, so "their own conformance suites" means the checker's own suite. | MET |
| Nothing depends on `holler-cli` or `holler-hub`; only `holler-pane` and `serde_json` | AC6 evidence. | MET |
| Workspace gates pass, the CHANGELOG `[Unreleased]` entry links this issue, and library code has no unwrap, expect or panic | AC8 and AC9 evidence. | MET |

## Spec compliance

- **Public API.** The API is exactly the brief's:
  - `Envelope`, `EnvelopeError` and `EnvelopeFault`, each deriving `Debug, Clone, PartialEq, Eq`.
  - `EnvelopeFault` has the 17 variants in the issue's order, each with a `///` line. It is not `#[non_exhaustive]`.
  - `Display` and `std::error::Error` are implemented.
  - `check_envelope` and `check_ndjson` have the brief's signatures.

  The imports are `holler_pane::error::{class_of, is_valid_code}`. The three consts and eight helpers are private, and each has a caller.
- **Check order (decision 2).** I read `check_envelope` and `check_failure` against the brief's table. The checks run in this order:
  - the exit code;
  - framing;
  - an object;
  - a missing key, then an extra key;
  - `schema_version`;
  - `ok`;
  - on success, `error` is `null`;
  - on failure: `error` is an object, `data` is `null`, then the `error` keys, `code`, `message` and the class.

  That is rules 0 to 13 in the brief's order.
- **Framing (decision 3).**
  - Only one final `\n` is removed (`strip_suffix('\n')`).
  - The first byte must not be whitespace, because `serde_json` skips leading whitespace.
  - `byte_offset()` is read after the first `next()`.
  - Text before is only considered when the body does not start with `{`.
  - `match_indices('{')` plus `str::get` keeps slices on char boundaries, with no indexing.
  - `is_ascii_whitespace` and JSON whitespace differ only in form feed. I checked that both reach rule 1(b) when a form feed comes first, so the fault is the same.
- **Mapping the cases the issue does not name (decision 1).** All as the brief maps them:
  - a non-boolean `ok` is `OkDisagreesWithExit`;
  - a non-object `error` is `NoErrorWhenFailed`;
  - a missing or extra member of `error` is `MissingKey("error.code" | "error.message")` or `UnknownKey("error.<key>")`, through the same `exact_members` the envelope uses;
  - a non-string `code` is `CodeNotKebab(<JSON text>)`;
  - a non-string or blank `message` is `MessageNotOneLine`.

  The issue says a message must be "non-empty". The brief also rejects a blank one, and records that choice in decision 1, so it is not a silent change. It changes nothing for real CLI output: the CLI's `one_line` turns an all-whitespace message into `""`, which the issue's own rule rejects.
- **Data and version (decision 4).** `data` on success is unconstrained. `schema_version` is a `u64`, accepted only when `as_u64() == Some(1)`, so `1.0`, `"1"` and `null` are rejected.
- **NDJSON (decision 5).** Lines before the last are checked at exit 0, and their `OkDisagreesWithExit` becomes `NotLastFailure`. The last line is checked at the process's exit code. `""` and `"\n"` are `EmptyStream`, and a blank line is `NotJson`. All six of the brief's "consequences" are rows in the stream table.
- **Display and Error (decision 6).** `Display` prints one line per variant, and `std::error::Error` is implemented.
- **Cargo.lock and lib.rs (decision 7).** `Cargo.lock` gains one line. In `lib.rs` only the doc paragraph that starts at line 5 changes: 4 old lines become 5. AC7 says "nothing else in lib.rs changes". The re-wrap stays inside that one doc paragraph, changes no item, and leaves the words after the edited sentence as they were. A's W-6 allowed exactly this ("any re-wrap kept inside that one doc paragraph"), and F declared it under "Deviations". The change is doc only, and slices c to e do not edit `lib.rs`, so it cannot conflict with them. Accepted. It is not a silent deviation.
- **ADR-0021 section 9.** The checker matches it:
  - stdout is one envelope;
  - `error` is `null` exactly when `ok`;
  - there is no `detail`;
  - NDJSON has one envelope per line;
  - exit codes are 0, 1, 2 and 3, and `ok` is false for 1, 2 and 3;
  - the class comes from `class_of`;
  - `schema_version` is the integer 1.

  The module doc says how a field added to the envelope later is handled (W-2).

## Quality audit

- **Correctness and failure handling.** The checker is a pure function that fails closed. Every input that breaks a rule gets a fault, and no input can panic. Library code has no `unwrap`, `expect`, `panic!`, `unreachable!` or indexing. In `exact_members`, `unwrap_or_default()` runs right after the presence check, so its `null` default is never used, and a comment says so. I traced these edge cases by hand:
  - `"1 {..}"` and `"[{..}"`;
  - a leading form feed;
  - `G\r\n`;
  - `"\n\n"` as a stream;
  - an `ok: true` line that carries an `error` before the last line;
  - a pretty-printed envelope.

  Each gives the fault the brief's rules imply. The checker has no concurrency and no persistence.
- **Build guards.**
  - The diff adds no `#[allow]`.
  - File sizes (`wc -l`): `envelope.rs` 457, `tests/envelope_test.rs` 696, `lib.rs` 42, `Cargo.toml` 24, `CHANGELOG.md` 562. All are under 900. Only the test file is over `lint.sh`'s 600-line warning, which is a warning, not a failure.
  - `dead_code` is denied and clippy is clean (F, T-green).
  - The `too_many_lines` and `cognitive_complexity` limits hold: the mutant table is split into five row functions.
- **Determinism across builds.** `cargo tree --offline -e features -i serde_json` shows `preserve_order` off for `-p holler-pane-testkit` and on for `--workspace`, so the testkit tests run under both `Map` backends in CI. `keys().min()` names the same key under both. Literal-string rows pin it: `two-extra-keys-smallest-first`, `extra-keys-among-known-keys-smallest-first` and their `error.` twins. T-green showed that the suite kills a `next()` mutant under `preserve_order` and a `next_back()` mutant under `BTreeMap`.
- **Protocol.** There is no wire, golden-file or `docs/protocol/v2.md` change, and none is needed. The checker reads the CLI's output contract and does not change it.
- **Tests.**
  - The tests run at the cheapest tier: a pure function, with no process, no sleep and no I/O. Nothing crosses a process, so no hub or body harness is needed.
  - Assertions compare whole values (`Envelope` or `Err(fault)`), not internals.
  - The `ALL_CODES` loop takes its expectation from `class_of`. The fixed anchors keep it from being a tautology: timeout at 1, usage at 2, pane-not-found and quota-exceeded at 3, and the `class-mismatch` and `usage-at-1` rows.
  - The RED-first evidence is in `handoff-T-red.md`.
  - `the_mutant_tables_cover_every_fault` passes against any implementation that compiles. As the brief intends, it is a compile-time guard over the tables, not a behaviour test. T and F both say so.
- **Documentation.**
  - The CHANGELOG entry is present and links #681 and the epic #633.
  - No new log event, CLI surface or protocol field, so README and `docs/` need nothing.
  - The intra-doc links resolve: F ran `cargo doc` with `-D warnings`.
- **Public-repository privacy.** I grepped every added line of the diff, including the brief and the handoffs, for:
  - home paths;
  - account, host and tailnet names;
  - IP addresses;
  - e-mail addresses and handles;
  - scratch and worktree paths;
  - secret, token and key strings.

  Nothing matched apart from prose about secrets. `evidence.md` cites the `serde_json` source by a registry-relative path, not an absolute one.
- **Commit and PR hygiene.**
  - All six branch commit subjects are Conventional (`docs(handoffs): ...` and `chore(#681): ...`), and the repo's `commit-msg` regex accepts them.
  - Each commit has a `Co-Authored-By:` trailer. None has a session link, but neither do recent squash commits on `main` such as `e410e9d`, so this run is not the cause, and the commits are squashed at merge anyway.
  - No PR exists yet (`gh pr list --head issue-681-implementation` is empty). Per CLAUDE.md, the AI disclosure goes into the PR body with `gh pr edit` after the script opens the PR. See Advisory note 5.

  None of this is F's work to fix.

## Scope check

- **Delivered:** exactly what the brief scoped:
  - the five public items and the module doc;
  - the `lib.rs` paragraph;
  - the dependency line;
  - the CHANGELOG entry.

  Nothing changes in `holler-pane`, `holler-cli`, `holler-hub`, the root `Cargo.toml`, `conformance/` or any other testkit module. No ADR or golden file changes.
- **Over-delivery:** small, and all of it declared:
  - 16 mutant rows beyond the brief's 32, and 1 stream row beyond its 11. They pin rule order and the decision-1 mappings, and the brief asks for "at least these rows".
  - `MissingKey` is quoted in `Display` too.
  - The struct fields have `///` docs.

  None of it is a refactor or a feature.
- **Under-delivery:** none.
- **Size:** the test file is 696 lines. The brief estimated about 280, and T explains the difference in its RED and GREEN handoffs. It is under the 900-line gate. `handoff-T-green.md` gives 690 lines, but the file is 696 (671 at RED plus 25). That is a miscount in the handoff and changes nothing.

## Verdict

**PASS.** Every acceptance criterion of the brief (AC1 to AC10) and of issue #681 is proved by a named test or by evidence I checked. The implementation follows the brief's API, its rule order and all seven of its decisions. Quality, privacy and scope are clean. Ready for O.

## Advisory notes (non-blocking)

1. **Follow-ups recorded only in pipeline files.** The handoffs and `decisions.md` are removed before the push, so O should record these on the issues or they will be lost:
   - **W-6, for #684.** `lib.rs:28-29` still says "The modules of slices b to e are empty stubs", which is no longer true of `envelope`. #684's issue text does not mention this, and its blast radius leaves out `lib.rs`.
   - **W-4, for #660.** The issue text still says "0 ok, 1 refused or failed, 2 usage", which predates ADR-0021 section 9 and #676 (a refusal exits 3). It also does not say that the CLI's `one_line` keeps a lone `\r` and turns an all-whitespace message into `""`. This checker rejects both, as it should.
   - **W-5.** The three placeholder envelope parsers in the `holler-cli` tests should call `holler_pane_testkit::envelope` or be removed.
2. **Rule order is only partly pinned by tests.** The code checks the rules in the brief's order; I verified that by reading it.
   - Rows that break two rules pin these orders: 0 before 1, 5 before 6, 6 before 7, 6 before 8, 12 before 13, and the order of the missing keys inside rules 3 and 10.
   - No row pins these: 3 before 4, 4 before 5, 8 before 9, 9 before 10, 10 before 11, 11 before 12, and 11 before 13.

   If a later change reordered one of those pairs, the output would still be rejected, only under a different fault name. A follow-up could add rows. For example, `error: null` with `data: {}` at exit 1 pins 8 before 9, and `Pane_Not_Found` at exit 3 pins 11 before 13.
3. **Duplicate keys are not detected** (F's known issue). `serde_json::Value` keeps the last value of a repeated key. Neither the issue nor the brief has a rule for this. The CLI serializes a struct, so it cannot write a duplicate key. If this matters later, an 18th variant would be the fix.
4. **What `check_envelope` does not pin.**
   - It accepts a pretty-printed, multi-line single envelope. That fits the issue's "exactly one JSON object". Only `check_ndjson` requires one line per envelope. Whether single-envelope output is compact belongs to the CLI's own tests (#660).
   - Rule 1(b) can be quadratic, but only on output that is already faulty.
5. **PR step.** Once the script opens the PR, add the AI disclosure that `CONTRIBUTING.md` requires with `gh pr edit`, as CLAUDE.md directs. Squash-merge with the house `Co-authored-by` trailers.
