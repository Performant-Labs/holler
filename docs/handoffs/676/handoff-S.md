# Handoff-S: Phase 9 (spec audit) - #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

**Date:** 2026-10-09 (11:11 MDT)
**Branch:** issue-676-implementation (at d5734af; merge-base 55dba00)
**Issue:** #676 (epic #633, ADR-0021 "Decisions taken" item 5)
**Brief:** `docs/handoffs/676-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md`, `evidence.md`. Issue #676 read with `gh issue view`, and #638, #645 and #646 read to check the ADR's claims
about them.
**Diff audited:** `git diff origin/main...HEAD` in full: all production lines, both test files, both ADRs and the CHANGELOG.
I ran read-only commands only (`git diff`, `grep`, `wc -l`, `gh issue view`) and did not re-run Tier 1 or Tier 2.

## A precondition

Met. `handoff-A.md` is **PASS** (0 blocks, 5 warns). `handoff-A-dup.md` is **PASS** (0 blocks, 4 warns). Neither has a block.

## T precondition

Met. `handoff-T-red.md` confirms RED. Both targets fail to build on the missing API (E0432). A throwaway skeleton showed the
assertions fail for the right reason: 3 failed in `error_class_test` and 6 in `output_api`, each about the missing exit 3 or
the decided class. `handoff-T-green.md` confirms GREEN: `cargo test --workspace` gives 1048 passed, 0 failed, 5 ignored, and
**Blocking issues: None**. Two mutations prove the tests are not vacuous. Restoring the old `exit_code` rule fails 6
`pane_verbs` tests, and moving `PaneInOtherProfile` to the Failure arm fails `every_closed_code_has_the_decided_class`.

## Acceptance criteria

The issue's own acceptance list comes first (it is the source of truth), then the brief's ACs.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| Issue 1 | Every closed code is classified, and a test fails when a new code is added without one | `crates/holler-pane/tests/error_class_test.rs::every_closed_code_has_the_decided_class` asserts that `DECIDED`'s key set equals `ALL_CODES`, then checks `class_of` row by row. At compile time, `class_of`'s `match closed` (`error.rs:274-302`) names all 22 `PaneCode` variants with no `_` arm, so a new variant does not build. | MET |
| Issue 2 | The refusal examples exit 3 and the failure examples (timeout, generation-conflict, an unreachable hub) exit 1, with the same code in text and JSON mode | `crates/holler-cli/tests/pane_verbs/output_api.rs::a_refusal_exits_3_and_a_failure_exits_1_in_both_formats` (`:257`) checks literal numbers for all four named refusals, an open code, `timeout`, `generation-conflict` and `unavailable` (the unreachable-hub code, per ADR-0021's table row), in both formats, without calling `class_of`. `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats` (`:221`) asserts text equals JSON for all 22 closed codes. | MET |
| Issue 3 | The envelope's `ok` is false for exit 1, 2 and 3 alike | `every_closed_code_exits_by_its_class_...` asserts `ok == false`, `data == null` and `error.code == code` on the JSON envelope of every closed code. The 22 codes cover all three exits: 1 usage, 14 refusals and 7 failures. `emit_usage_error_returns_2_and_routes_by_format` (`:299`) covers the clap usage path. | MET |
| Issue scope | `emit_stream` ends with 3 on a refusal and 1 on a failure | `emit_stream_exits_3_on_a_refusal_item` (`:284`, `pane-not-found`, keeps the earlier line). `emit_stream_exits_1_on_an_error_item_and_keeps_the_earlier_lines` (`:353`, `unavailable`) is unchanged. | MET (text mode; see Advisory 3) |
| Issue scope | `docs/` or ADR 0003 text that says these verbs never exit 3 | Repo-wide grep: only ADR-0003 and ADR-0021 describe pane/profile exit codes, and both are updated. `README.md` and the rest of `docs/` say nothing about them. | MET |
| Brief 1 | `class_of` and `ErrorClass` exist, `ErrorClass` derives `Debug, Clone, Copy, PartialEq, Eq`, one exhaustive match, four tests | `error.rs:220-303`: `ErrorClass` (`:221`), `exit_code` (`:239`, `const`), `class_of` (`:266`, `let ... else` on `PaneCode::parse`, then one `match`). `error_class_test.rs` holds the four tests the brief names: `:39`, `:56`, `:69`, `:76`. | MET |
| Brief 2 | `exit_code` is `class_of(..).exit_code()`, with no comparison against `usage` and no table; three new tests, three updated, header fixed | `output.rs:338-340` is that one line. New tests: `output_api.rs:221`, `:257`, `:284`. Updated: `:145` and `:160` (`pane-not-found` now gives 3), and `:195` is renamed to `..._and_3_for_a_refusal`, with `probe-failed` giving 3 at `:212`. Header: `:4-9`. The unchanged exit-1 tests (`:353`, `:398`, `:423`, `:444`, `:458`) and the stub and legacy-verb tests still pass (64/64 `pane_verbs`). | MET |
| Brief 3 | Text residue fixed | I re-ran `grep -rn "refused or failed"` across the whole repo: the only hits are in this run's own handoffs. `output.rs:10-16`, `:201-202` and `:336-337`, `pane/mod.rs:51`, `profile/mod.rs:39`, `pane/args.rs:6` and `spec_flags.rs:3-5` all state the new codes. The `args.rs` claim (a bad `--grid` exits 3) is correct: `GridPos::parse` returns only `grid-ambiguous` or `grid-out-of-range` (`grid.rs:46-60`), and both are refusals. | MET |
| Brief 4 | ADR-0021 §9 has the table, names `class_of`, links #676; item 5 says "(done in #676)"; ADR 0003 gets its sentence; no `` `holler `` code span | ADR-0021 `:358-400`: the bullet names `holler_pane::error::class_of` and links #676, and "in the change that moves..." is gone. The table has 24 rows: the 22 closed codes, each with the brief's class, plus the open and malformed rows. `:543-546` adds "(done in #676)". ADR-0003 `:98` adds the sentence, and `:10` adds a `Clarified by` line (A W-3). The ADR diffs add no `` `holler `` span. The only two in the diff are in `CHANGELOG.md`, which `docs_cli_test` does not scan (`README.md` and `docs/**` only), and both are real namespaces. | MET |
| Brief 5 | CHANGELOG entry | `CHANGELOG.md:58-65`, under `## [Unreleased]` / `### Enhancements`, directly after the #670 entry. It covers 3 for a refusal and 1 for a failure in both modes, `ok` false, the table in ADR-0021 §9, stubs still exit 1, and the link to #676. `changelog-check.sh` passes (T-green). | MET |
| Brief 6 | Guards | T-green Tier 1: `cargo test --workspace`, clippy with `-D warnings`, `machete`, `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `wire_selftest` and `rustfmt --check` all pass. F also ran `cargo build`, `test-hooks.sh` and `cargo doc -D warnings`. `wc -l`: `error.rs` 710, `output.rs` 467, `output_api.rs` 470, `error_class_test.rs` 80, all under 900. | MET |
| Brief 7 | Blast radius | `git diff --name-only origin/main...HEAD` lists 11 code and doc paths, all on the brief's list, plus `docs/handoffs/676*`. `main.rs`, `holler-pane/src/lib.rs`, every `Cargo.toml`, `Cargo.lock`, the golden files and the `cli-surface` fixtures are unchanged (`git diff --quiet`). | MET |

## Spec compliance

Every "Decision already made" in the brief is implemented as stated:

1. **The table.** `class_of`'s arms match the brief row for row: 1 Usage, 14 Refusal, 7 Failure. ADR-0021's table matches as
   well; I checked each row's class. Its reasons are the brief's, lightly reworded. The brief's per-row "Close call." marks
   became one paragraph under the table that lists the same seven codes, which is a presentation choice only.
2. **`class_of(code: &str)`, not a method on `PaneError`.** Done, and no `PaneError::class` was added. The doc comment shows
   `class_of(error.code())` and tells it apart from the crate-private `classify` (A W-4).
3. **Three classes, `Usage` included.** Done. The CLI has no special case for `usage`.
4. **The exit number lives on `ErrorClass::exit_code` in `holler-pane`.** Done. Exit 0 stays a literal in `output.rs`
   (`:279`, `:290`).
5. **An open code is a refusal and a malformed code is a failure.** Done (`error.rs:267-273`), and the ADR states it. Production
   code builds an `ErrorBody` only from a `PaneError`, from `usage` or from `not-implemented`. No open failure code exists
   anywhere, so this rule changes no current exit code.
6. **`not-implemented` is a failure (exit 1).** Done. Every stub test and legacy-verb test is unchanged and passes.
7. **Doc-comment lines in `pane/mod.rs`, `profile/mod.rs` and `pane/args.rs`; `main.rs` not edited.** Done.
8. **The holler-pane frozen rule.** Kept. The `error.rs` diff removes 0 lines, and `lib.rs` is unchanged.

The brief is not self-contradictory. One understatement it carries into the ADR is in Advisory 1.

## Quality audit

- **Correctness and failure handling.** `exit_code` is the only place that picks an error's exit code. `emit_text`, `emit_json`,
  `emit_stream`, `emit_error`, `emit_usage_error` and `main.rs`'s wiring-error path all reach it, and `main.rs:112` passes the
  code through unchanged. `settle` changes only a failed write on a success (to 1), so a refusal whose write fails still
  exits 3. Malformed codes fail closed as a Failure, the same rule `from_wire` applies to a garbled reply. There is no
  concurrency surface.
- **Build guards.** The added production lines have no `unwrap`, `expect`, `panic!`, `todo!` or `#[allow]`, and the added test
  lines have no `#[allow]`, `unwrap` or `expect`. No touched file is at or above 900 lines. There is no dead code: both new
  `pub` items are used by `output.rs` and the tests. `let ... else` is already used across the workspace, and the toolchain
  is `stable`.
- **Protocol.** No change. Protocol v2, the 22-row wire catalog, `holler-proto` and every golden file are untouched, as the
  brief puts them out of scope.
- **Tests.** The tests are pure in-memory checks at the cheapest tier that reaches the code, with no sleeps or timing. A
  process-level refusal test cannot exist yet, because every stub returns `not-implemented` before it reads its arguments,
  and the brief puts that test out of scope. The numbers are pinned twice: through `class_of` over `ALL_CODES`, and by
  literal spot checks that do not call `class_of`. A wrong table and a matching wrong test therefore cannot both pass. RED
  first is evidenced in T-red, and non-vacuity by T-green's two mutations.
- **Documentation.** The CHANGELOG entry links the issue. ADR-0021 and ADR-0003 are updated consistently. Every other exit
  number in ADR-0021 still agrees with the table: I3 (`:162`), `generation-conflict` and `profile-conflict` (`:271`, `:287`,
  `:294`) and `timeout` (`:451`). The ADR's claim that the open codes #645 and #646 plan are all refusals matches both
  issues' text ("Refuses a pane that is not idle...", "refuse when ... the shown and driven sessions differ"). The new
  "Deferred to named stories" bullet for the I3 post-act mismatch (A W-1) is accurate and owned by #644 and #645. There is
  no new log event, CLI surface or protocol field, so the README needs nothing.
- **Public-repository privacy.** I grepped all 1,321 added lines, handoffs included, for home paths, account and machine names,
  e-mail addresses, tailnet or Tailscale names, IP addresses and API keys. There were no hits.
- **Commit and PR hygiene.** All six commit subjects are Conventional Commits, and every commit has a `Co-Authored-By`
  trailer. They take the same shape as the merged #670 squash, which the repo's `prepare-commit-msg` hook writes. The PR is
  not open yet. Checking its AI disclosure against `CONTRIBUTING.md` is the post-PR step (see Advisory 2).
- **Branch freshness.** `origin/main` is 2 commits ahead (#635 and #636). Both touch only `docs/research/` and
  `scripts/spikes/` and overlap nothing here, as A-dup checked.

## Scope check

F delivered exactly the brief's scope. There is no unrelated refactor and no extra code. Four doc-only additions go beyond the
AC lists, all in blast-radius files and all explained in `handoff-F.md` "Deviations":

- ADR-0021 §9's open-code sentence and the I3 "Deferred to named stories" bullet (A W-1).
- ADR-0003's `Clarified by: #676` header line (A W-3).
- The `class_of` doc's note on `classify` (A W-4).
- `output.rs`'s `not_implemented` doc, which no longer calls a stub's exit-1 error a "refusal".

Each one keeps the vocabulary consistent with the decision, and none changes behaviour. Nothing was left undone. T's one
repair, the `spec_flags.rs` `//!` line, was an AC 3 item, and T made it at T-green.

## Verdict

**PASS.** All three of the issue's acceptance criteria and the brief's seven are met. Each has a named test or concrete
evidence, and the tests are shown not to be vacuous. All eight MO decisions are implemented as stated, quality is acceptable,
and the scope is exact. Ready for O.

## Advisory notes (non-blocking)

1. **ADR-0021 `:397` understates what moving a close call costs.** This is A-dup W-1 and W-2, which were not applied before
   this audit. The sentence says "(one arm of `class_of` and one row of its test)". In fact a move also changes this table's
   own row, and moving `pane-not-found` breaks three `output_api.rs` tests that use it as their example refusal: `:145`,
   `:160` and `:291`. Moving `probe-failed` breaks `:212`. The tests would catch each of those, so nothing breaks silently.
   O can apply the fix before merge without another gate, since it is doc-only and in a blast-radius file. Replace the
   parenthetical with: "(one arm of `class_of`, one row of its test, this table's row, and any `output_api.rs` test that uses
   the code as its example)".
2. **What the PR body must carry, since these handoffs are deleted before push:**
   - The AI disclosure under `CONTRIBUTING.md`. The script's PR body has none; add it with `gh pr edit`, as the repo's
     `CLAUDE.md` says.
   - **For #649:** the `run_with_stdio` doc at `crates/holler-cli/src/main.rs:253-255` says a wiring error exits 1. Once
     `Wiring::connect` can fail, change it to "(with the error's code, and the exit code of its class)". The line is accurate
     today because `connect` always returns `Ok`.
   - **For #646:** when #646 replaces the legacy `--pane`/`--profile` stops, it should align `prompt_target.rs:58-61`
     (`REFUSED_EXIT = 1`) and the "refused with exit 1" docs in `say_cmd.rs:113`, `interrupt_cmd.rs:36` and
     `answer_cmd.rs:36` with ADR-0021 §9's vocabulary, or route them through `ErrorBody` and `class_of`.
   - **For #645:** its acceptance says "a switch to a deleted session fails". Under the table, `session-not-found` is a
     refusal (exit 3), which the ADR records as a close call. #645's brief should assert the code and exit 3 as the table
     says, and should not read "fails" as exit 1.
3. **Small test-only fixes, optional, for T when a later story next touches these files:**
   - In `output_api.rs:350-353`, the name and doc of `emit_stream_exits_1_on_an_error_item_and_keeps_the_earlier_lines` still
     say that any error item ends at 1. Only a failure item does now. Its code is `unavailable`, so the assertion is right; a
     better name is `..._on_a_failure_item_...`. The brief kept this test unchanged on purpose.
   - In `error_class_test.rs`, the doc at `:10-11` cites "the brief's table", which is deleted before push; ADR-0021 §9 is the
     lasting copy. Line `:3` says the test kit already calls `class_of`, but that comes with #638.
   - `emit_stream_exits_3_on_a_refusal_item` runs in text mode only. `emit_stream` passes the format straight to `emit`, and
     `emit` is covered in both formats, so the gap is theoretical.
4. **Out of scope, noted only.** The stub verb files under `crates/holler-cli/src/pane/` and `src/profile/` still say a stub
   "refuses" with `not-implemented`, which is now a failure (exit 1). Each verb's story replaces its stub.
