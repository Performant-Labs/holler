# Handoff-A: #652 - The operator guide for `holler pane`  (up-front plan review, round 2)

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (base `origin/main` at `5d366b9`, which has #650, #664, #644 part 2 and #642 part 3; head `05d7804`; the branch touches only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/652-brief.md` at `05d7804` (amended after round 1's BLOCK, commit `060d328`)   **Reuse map:** the brief's evidence (E, R, C excerpts), its Blast radius and "Files named by this brief" (no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: 0 `block`, 5 `warn`. Round 1's block (which panes `holler pane` manages) is resolved by operator decision (a):
the brief now scopes I1 and every verb to registry (OpenCode) panes, keeps the script's Claude Code path after the
sign-off, and carries that boundary into sections 1, 3, 5, 10, 11 and the `docs/orchestrating.md` rewrite. Round 1's
warns 2 to 9 are all folded in (rollback paragraph, import synopsis and "replaced by", `docs_cli_test` writing rules,
numbered headings and one PROPOSED line rule, `class_of`-checked class column, `docs_<subject>_test` name, the #651
plugin sentence, `scripts/pane-restart:3` in the Blast radius). The test plan extends the existing docs-test pattern
(`docs_cli_test.rs`, `docs_errors_test.rs`) and reads every list from a public table on the base; nothing is a
parallel path. The warns are two heading forms that disagree with the clap tree, an unqualified sentence in section
10, the #649 ordering, and the test file's size.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Section 3 heading list; acceptance 17 | pattern consistency (clap tree vs. brief) | Two heading forms in section 3 disagree with the clap tree on `5d366b9`. (a) `holler pane switch PANE`: `PaneSwitch` has a second required positional, `session` (`value_name = "SESSION"`, `switch.rs:38-39`), so the span fails `docs_cli_test` with `MissingRequiredArgument` and acceptance 17 computes `holler pane switch PANE SESSION`. (b) `holler pane park PANE --reason TEXT --release-when WHEN`: `park`'s `PANE` is `Option<String>` (`park.rs:68-69`), so acceptance 17's computed string is `holler pane park --reason TEXT --release-when WHEN`, which is not a substring of the brief's heading. Built as written, the brief's own list fails its own test. | F writes `### \`holler pane switch PANE SESSION\`` and `### \`holler pane park --reason TEXT --release-when WHEN\`` (the computed forms; the prose may say the optional `PANE` comes first). The brief already says "the heading text is the form that parses"; T and F take the computed form as authoritative over the listed one. |
| 2 | warn | Section 10's closing paragraph vs. "Known facts" (1) | cross-cutting (source of truth) | The boundary bullet is correctly qualified ("a pane whose `sessions.toml` names another harness"), but section 10's last paragraph says unqualified "A pane that does not run OpenCode gets no record from the import; ... `unregistered-herdr-pane` in every pass". Known fact (1) says the live fleet's Claude Code panes have files that name `opencode`, so the import adopts them with a record, and they will **not** show as `unregistered-herdr-pane`. The guide would state both. | Section 10 says "a pane whose files name another harness" and points to the known fact stated before step 1. Separately (outside this story): no exclusion mechanism exists, so #654's real import would record Claude Code panes as OpenCode panes, after which `holler pane relaunch` on one would start an OpenCode server in it. O should file a follow-up issue (an import exclusion, or a step of #654) that gates #654's real run; the guide's section 12 can name it once it has a number. |
| 3 | warn | "Depends on", the REFRESH lines (R1 to R3), sections 2 and 4 | ADR / ordering | #649 is still OPEN at review time, and `wiring.rs` on the base is the #670 stub (every port answers `not-implemented`). Sections 2 and 4 (no hub gives `unavailable`; read verbs need no Herdr; the environment list lives in `wiring.rs`'s module doc; DRIVEN prints `-`) describe its expected contract. If this run reaches F before #649 merges, F writes them against code that does not exist, and the "re-cut and patch" step cannot run. | Merge `origin/main` after #649 lands and re-cut R1 to R3 before F writes sections 2 and 4, or hold the PR until then; S checks each REFRESH line against merged `wiring.rs`. None of acceptances 1 to 17 reads #649's code, so T can proceed. |
| 4 | warn | Acceptance 15 vs. operator decision 3 | single source | Decision 3 says "the one hand-kept list is the PROPOSED set", but acceptance 15 adds a second hand-kept list (apply's action and outcome words), because `tx_apply` exposes only `as_str`. The reason is sound (no source change is in scope), but the brief contradicts itself. | Keep acceptance 15 as written; its constant's doc names ADR-0021 section 9's apply row and says it is hand-kept because `tx_apply` has no `ALL` table. Optional follow-up: an `Action::ALL`/`Outcome::ALL` in `tx_apply.rs` would let the test read it. |
| 5 | warn | `crates/holler-cli/tests/docs_pane_control_test.rs` (new) | size and structure | Seventeen tests, a section slicer, a clap-tree walker with a placeholder rule, a grid-block parser and an envelope check is likely 600 to 900 lines; `scripts/lint.sh` warns at 600 and fails at 900, and the brief names no split. | If the file passes about 600 lines, make the target a directory (`tests/docs_pane_control/main.rs` plus modules), as `pane_verbs/` and `pane_integration/main.rs` do, with `path` in the `[[test]]` entry pointing at `main.rs`. |

No duplication or layering drift: the test is a new target beside `docs_cli_test` (`Cargo.toml:88-90`, `autotests = false`), the
guide is a new page indexed under the existing "Guidance" heading of `docs/README.md`, the `orchestrating.md` change rewrites the
one section whose own deletion condition is obsolete, and no source file outside tests changes.

### Verified on the base (`5d366b9`)

- `HarnessKind` is `{ Opencode }` (`holler-pane/src/pane.rs:136-138`); `ImportFinding::ALL` has sixteen kinds with
  `harness-not-opencode` at `Level::Skipped` and the six cross-spelled codes exactly as section 3 says
  (`import/findings.rs:103-166`); `IMPORT_BLOCKED` is public (`holler-pane/src/import.rs:45`).
- `pane import`: `--from` (`STATE_ROOT`) required in clap, `--herdr-session` (`NAME`) optional in clap and `usage` at
  run time (`pane/import.rs:82-95`, `:157`); `already-imported` and `profile-exists` behave as section 10 says
  (`pane/import/writes.rs:6-8`, `:75-83`).
- `ALL_CODES`, `class_of`, `ErrorClass::exit_code` are public (`error.rs:146`, `:221-266`); `profile-exists` is a closed
  code.
- Open codes exist as named: `AGENT_UNUSABLE` (`holler-adapter-opencode/src/server.rs:31`), `AGENT_KEY_INVALID`
  (`argv.rs:125`), `orchestrator-not-relaunched` and `SPEC_NOT_LAUNCHABLE` (`tx_apply.rs:74-78`).
- `COLUMNS` is public with nine names (`pane/list.rs:30`); `, agent KEY` is printed by `launch.rs:408`;
  `check_envelope(stdout, exit_code)` is public in the test kit and `holler-cli` already dev-depends on it
  (`Cargo.toml:435`).
- `GridPos::parse` refuses `hj-c1r2` (no comma; `take_cell` requires a leading `r` or `c`), as section 7 says.
- `docs/orchestrating.md:108` is the only `(interim)` heading and no page links its anchor; `scripts/pane-restart:3`
  is the only other carrier of the stop-gap sentence, and `scripts/tests/pane-restart-test.sh` does not read it.

## Notes for O

PASS, so none required. For the build: take findings 1 and 2 into F's instructions (two heading forms, one qualified
sentence); file the follow-up of finding 2 before #654's real import; and do not let F write sections 2 and 4 until
#649 has merged into the branch (finding 3).

## Patterns referenced

1. `crates/holler-cli/tests/docs_cli_test.rs` (whole): fences and spans scanned, normalisation, tolerated parse errors.
2. `crates/holler-proto/tests/docs_errors_test.rs`: a docs section sliced by its numbered heading and checked row by row against the code's table.
3. `crates/holler-cli/tests/cli_surface_test.rs`: the clap-tree walk that filters `help`.
4. `crates/holler-cli/src/pane/{switch,park,import}.rs` and `crates/holler-pane/src/import/findings.rs`: the argument and finding tables the guide must match.
5. ADR-0021 section 4 (I1, line 302), sections 9 and 13, read against `docs/orchestrating.md:108-145` and `scripts/pane-restart:1-12`.
