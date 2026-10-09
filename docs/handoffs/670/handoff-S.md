# Handoff-S: #670 the pane/profile CLI skeleton (skeleton slice c of epic #633), spec audit

**Date:** 2026-10-09
**Branch:** issue-670-implementation (at e4365c4; merge base f2602ba; `origin/main` is now af3d8df, #669)
**Issue:** #670 (epic #633)
**Brief:** `docs/handoffs/670-brief.md` (Revision 1)
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md`
**Diff audited:** all of `git diff origin/main...HEAD` (80 files). I read every new or changed `src` file in full: `cli.rs`, `main.rs`, `output.rs`, `prompt_target.rs`, `pane/**`, `profile/**`, the three `*_cmd.rs` and `lib.rs`. I read every new test file in full, plus the fixture, the ADR 0003 and CHANGELOG diffs, and the manifest.
**Read-only fact checks:**
- `git diff --name-only` against the blast radius;
- `wc -l` on touched files;
- greps for `unwrap`/`expect`/`panic`, `#[allow`, `process::exit` and private names;
- `rustfmt --check` on `output_api.rs`, the one new file edited after F's rustfmt run;
- a few stub, help and usage runs of the built binary over an empty, isolated state dir. No hub was started and no `say`/`interrupt`/`answer` SESSION form was run.

## A precondition

Met. `handoff-A.md` (plan review) is **PASS** with 13 warns. `handoff-A-dup.md` (anti-duplication) is **PASS** with 5 warns. Neither has a block.

## T precondition

Met. `handoff-T-red.md` records a valid RED:
- `pane_cli_process`: 28 of 33 fail on missing behaviour. The 5 that pass are deliberate guards.
- `pane_verbs` and `profile_verbs` fail to build only on the API the brief commits F to create.
- `cli_surface_test` fails on the 20 new leaves.
- Every target is declared in the manifest in the same commit (0afbb93), so no RED is a missing `[[test]]`.

`handoff-T-green.md` records GREEN with **no blocking issues**:
- workspace 1023 passed, 0 failed;
- clippy `-D warnings`, `cargo machete`, `lint.sh` and `changelog-check.sh` all clean;
- mutation spot-checks fail the tests.

F's commit (016eeaa) touches no test, fixture or manifest file. T's Phase 7 commit (85444f1) adds five tests to `output_api.rs` and touches no production code.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | build, clippy, test, machete, lint, changelog-check pass; no file over 900 lines; `main()` ≤ 100 lines; new `.rs` files rustfmt-clean, existing files not reformatted | `handoff-T-green.md` Tier 1 table (clippy, machete, lint, changelog, `cargo test --workspace` 1023/0); `handoff-F.md` (build, test-hooks, rustfmt on the 27 new `src` files); `handoff-T-red.md` (rustfmt on new test files). My checks: the largest touched file is `cli.rs` at 780 lines; `main()` is `main.rs:101-113` (6 code lines); `rustfmt --check --edition 2021` passes on `output_api.rs` after T's Phase 7 additions. | MET |
| 2 | 20 stub verbs: text `error: not implemented (story #NNN)` on stderr, empty stdout, exit 1; JSON is one envelope, exit 1; bare namespace exit 2; `--debug bogus` exit 3 | `process/stub.rs`:<br>- `stub_verb_not_implemented` (20 verbs);<br>- `stub_verb_not_implemented_json_is_one_envelope` (20 verbs × 4 spellings, exact compact prefix);<br>- `a_bare_namespace_is_a_usage_error`;<br>- `a_bad_debug_value_is_still_a_policy_refusal_before_dispatch`.<br>In process: the 20 per-verb `*_stub_routes_text_to_err_and_json_to_out`. Story numbers match the epic's wave table and the sibling issues' titles (#643 list/get/watch, #644 launch/relaunch, #645 switch/reset, #646 park/unpark/close, #647 doctor, #650 import, #662 create/delete/list/show, #664 apply, #665 rename/export/import). | MET |
| 3 | Usage errors exit 2, as a `usage` envelope in JSON mode under `pane`/`profile` only; `--help`/`--version` unchanged | `process/usage.rs`:<br>- `spec_only_requires_profile` (uses `pane launch --spec-only` per A W-3);<br>- `a_bad_format_value_is_a_usage_error`;<br>- `json_conflicts_text`;<br>- `command_arg_and_command_json_are_mutually_exclusive` (both pairs);<br>- `an_unknown_flag_under_pane_or_profile_is_an_envelope_in_json_mode`;<br>- `a_legacy_verbs_json_usage_error_is_untouched`;<br>- `an_unknown_flag_in_text_mode_is_clap_s_own_message`.<br>`process/stub.rs::help_lists_every_verb_and_exits_0` and `top_level_help_names_pane_and_profile`; existing `cli_invocation_test` (14) green. | MET |
| 4 | `--pane`/`--profile` shapes parse and resolve by value; existing forms unchanged; refusals #646/#648 in plain text before any hub; `Usage` for missing or extra positionals | `pane_verbs/target_flags.rs` (16 tests, by value: SESSION forms, `--pane` forms, `--pane --profile`, flags after the tail, clap's third-positional refusal, `Usage` for missing, extra and SESSION-with-`--pane`). `process/legacy_verbs.rs`: 10 refused forms under 4 formats, `roster --profile` to #648, malformed forms exit 2 (not the refusal), SESSION forms not refused. The existing say/interrupt/answer tests are unchanged and green. | MET |
| 5 | Spec flags parse on launch/relaunch; `--profile`, `--spec-only`, `--take-over` placement; `validate()` types values with the guards' codes | `process/flags.rs` (6 tests, the flag matrix). `pane_verbs/spec_flags.rs` calls `validate()` directly and covers: `grid-ambiguous`, `grid-out-of-range`, `profile-secret-refused` (the issue's `NAME=value` case), `env-name-invalid`, `command-not-argv` for both JSON flags, `usage` for non-JSON text, and no secret echoed. | MET (ctx deviation recorded, see Spec compliance) |
| 6 | Seam test per verb binary through `verb_harness`; `emit`/`emit_stream`/`emit_usage_error` routing; `ErrorCode::new` uses `is_valid_code`; `ErrorCode::from(&PaneError)` | Per-verb `assert_stub_routes` cases (12 + 8) through `tests/verb_harness/mod.rs`, both formats (text to `err` only; JSON one envelope on `out`, nothing on `err`). `pane_verbs/output_api.rs` (18 tests). | MET for the behaviour; **the harness misses the brief's shape, see REWORK 1-2** |
| 7 | ADR 0003 rows (one per verb, bare, grouped by story) and fixture lines (grouped under `# #NNN`, every leaf and shared flag, the two regression lines); `docs_cli_test` and `cli_surface_test` pass | `process/docs_rows.rs` (5 tests); `docs_cli_test` (3) and `cli_surface_test` (3) green per T-green; fixture diff read. | MET |
| 8 | `git diff --name-only origin/main...HEAD` lists only blast-radius paths | My check: all 80 paths match the brief's Blast radius (`grep -v` of the radius returns nothing). | MET |

## Spec compliance

The production code implements the brief's decisions. Each deviation from them is recorded in `decisions.md` and `handoff-F.md`. None is silent:

- **Decision 1 (`main.rs`)**:
  - `Cli::try_parse()`; `--help` and `--version` go through `error.exit()` as before.
  - The JSON usage envelope appears only when the first non-flag token is `pane` or `profile`. The value-taking global flags come from the clap tree (`Cli::global_value_flags`).
  - The resolver is called once.
  - `output.rs` never exits (grep).
  - Dispatch is one exhaustive `match` instead of "a helper plus two arms" (A W-9, recorded), which removes the old fall-through to exit 0.
  - The `roster --profile` refusal sits in `main.rs::run_roster`, so `roster_cmd.rs` is untouched for #648.
- **Decision 5**:
  - `rest: Vec<String>` with `value_names` and `num_args = 0..=2` (`1..=2` for `answer`), without `trailing_var_arg`.
  - The accessors live in `prompt_target.rs` and reuse the root `cli::Usage`, whose `new` is now `pub(crate)`.
  - The three `*_cmd.rs` change only to call `route(x.resolve(), &x.profile)` first, before `--timeout`, `--parts-file` or any hub.
  - No accessor name collides with a root export.
- **Decision 6 (ADR 0003)**:
  - rows grouped by story and separated by blank lines;
  - `--format` on the global-flags line;
  - `[--profile NAME]` on `roster`;
  - the "only top-level verbs" sentence amended;
  - the shared-flags sentence;
  - line 62 amended for the envelope and NDJSON;
  - #633 and ADR-0021 cited;
  - the #665 rows marked proposed (A W-11).

  Recorded deviation: the `--pane` forms of say/interrupt/answer are separate rows under #646's block, because `docs_cli_test` cannot parse a combined `SESSION|--pane` row.
- **Decision 7**: the signatures are as built and listed in `handoff-F.md`. Recorded adjustments: `VerbCtx` lives in `output.rs` and holds `Ports` by value (A W-7); `resolve_format(json, format)` instead of `&Cli` (A W-8); `emit` maps `usage` to exit 2 (A W-1); `Wiring::connect()`/`ports()` with `Unwired` (A W-5).
- **Decision 9**: `--format=json` is `--json` on every verb, and `--json --format=text` exits 2. `FormatChoice.json_explicit` exists for #648.
- **Decision 10**: one `Args` struct per verb file (`PaneList` … `ProfileImport`); only the shared groups are in `pane/args.rs`; no `profile/args.rs`; `--take-over` in `apply.rs`; no sibling positionals guessed; `--grid` stays a string; `mod.rs` layout kept (A W-10, recorded).

  Recorded deviation: `--ctx-soft`/`--ctx-hard` are `u32` at clap time, not strings. They match `holler_pane::ContextCeilings { soft: u32, hard: u32 }`, and a bad value is still exit 2 with code `usage`, the same outcome `validate()` would give. Accepted.
- **AC 2 envelope message**: it is `not implemented (story #NNN)`, without text mode's `error: ` prefix. T recorded this reading in `decisions.md`, and it is consistent with the epic's envelope ("one line for a person"). Accepted.
- **Unrecorded deviation (REWORK 1)**: the brief's Files section says the harness "builds a `VerbCtx` over given ports … so nine stories do not write near-copies", and AC 6 says the seam runs over ports "built from in-test stubs". `run_verb(argv, format)` takes no ports and builds them from `Wiring::connect()`. A W-5 (`decisions.md`) settled that the harness reuses `Unwired`, the not-implemented set #649 keeps. It does not reuse `Unwired` directly: it goes through `connect()`, the constructor #649 replaces. `src/pane/wiring.rs:10-13` states the harness builds from `Unwired`, which is not what the test code does.
- **Unrecorded deviation (REWORK 2-3)**: the brief's Files section says the per-verb test files exist "so a later story edits only its own file and never `main.rs` or the manifest". The issue says the slice is "created once so each verb story adds a file and edits none of these". `pane_verbs/main.rs` and `profile_verbs/main.rs` say the same in their own docs (lines 8-9), yet they hold tests that pin #643's, #644's and #662's stubs and #649's `connect()` body. The shared `pane_cli_process` files pin every sibling's stub line. A-dup W-1 raised this and left the call to S. I read the brief's sentence as a requirement, because it is the stated purpose of this slice ("Gates the CLI-verb stories").

## Quality audit

- **Correctness and failure handling:**
  - The `--pane`/`--profile` forms fail closed before any hub contact. The process tests run with no hub and assert the refusal line, so a contacted hub would have shown a different error.
  - Usage wins over the refusal for malformed tails.
  - Exhaustive dispatch removes the exit-0 fall-through.
  - `--debug bogus` still exits 3 before dispatch.
  - `emit` never writes half an envelope, keeps JSON messages on one line, flushes each line, and turns a failed write into a non-zero exit. `emit_stream` stops on a broken pipe (`pane watch | head`).
  - Output was checked by hand on the built binary: stubs, the bare namespace, `--spec-only` without `--profile`, `--ctx-soft abc` and a missing `--profile` value all give the specified envelope or plain message.
  - No state, so no concurrency concern.
- **Build guards:**
  - No `unwrap`/`expect`/`panic`/`unreachable` in the added `src` lines.
  - Every `#[allow]` (only the test-crate roots) carries `// #670`.
  - No touched file is near 900 lines (`cli.rs` 780, `output.rs` 468).
  - `main()` is 6 code lines.
  - `dead_code` and machete are clean per T-green.
- **Protocol:** no wire change, no golden file, no `holler-proto`/`holler-hub`/`holler-pane` edit. `docs/protocol/v2.md` is out of scope (#634); see Advisory notes.
- **Tests:**
  - The process tests use the real binary over an isolated `StateDir`.
  - No sleeps in the new targets.
  - RED-first evidence is in `handoff-T-red.md`.
  - The in-process tests assert behaviour by value.
  - The **test layout** fails the brief's forward-compatibility requirement (REWORK 1-3).
- **Documentation:** the `CHANGELOG.md` `## [Unreleased]` entry links #670. ADR 0003 is updated. README needs nothing, because the verbs are stubs and the operator guide is #652's.
- **Public-repository privacy:**
  - New code, tests, the fixture and ADR rows use `demo`/`demo-c1r1`, `/srv/demo` and `127.0.0.1`.
  - `hj-` appears only as quotations inside the handoff docs, and those names are already public on `main` (holler-pane tests, the epic).
  - `io/alpha` is the existing test label, and the brief mandates the regression lines.
  - No secrets, hostnames, tailnet names, private IPs or local paths.
- **Commit and PR hygiene:**
  - Subjects are Conventional (`docs(#670)`, `chore(#670)`) and pass the hook.
  - Each commit has a `Co-Authored-By` trailer but no session link. That is the workflow script's own convention: #669 merged the same way. See Advisory notes.
  - The PR does not exist yet.

## Scope check

Production matches the brief's scope. Every path is in the blast radius, and nothing in `holler-hub`, `holler-pane` or `holler-proto` changed.

Additions beyond the brief's wording, each justified and recorded:
- the `pane_cli_process` `[[test]]` target (T, so a RED existed before the in-process API compiled);
- `emit_error` and `not_implemented(_message)`;
- the write-failure, newline and one-line rules of `emit`;
- `Cli::global_value_flags`;
- `validate()` also refusing arg-plus-JSON for hand-built flags.

Under-delivery: the harness has no "given ports" entry point (REWORK 1).

## Verdict

**REWORK** (test-only)

The production code is compliant: every AC's behaviour is implemented and proven. The required changes concern the shared test scaffolding, which is this slice's deliverable for the 13 wave-3 stories and freezes when it merges. As built, #643, #644, #645, #646, #647, #650, #662, #664, #665 and #649 would all have to edit the same frozen test files in parallel, and some of those edits sit on adjacent lines. That is the rebase conflict the brief designs out of ADR 0003 and the fixture.

**TEST-ONLY: no `src/` change required.** The fields of `holler_pane::Ports` and the type `holler_cli::pane::wiring::Unwired` are already `pub`.

1. **`crates/holler-cli/tests/verb_harness/mod.rs:13, 27-31`: build over given ports, never over `Wiring::connect()`.**
   - **Problem:** `run_verb` takes no ports and calls `Wiring::connect()`. #649 replaces the body of `connect()` (`src/pane/wiring.rs:10-13`). After #649, every in-process verb test would run against the real wiring, and the `.expect` at line 31 would panic with no hub. #643, #644 and #662 must run their verbs over #638's fakes ("Tests use only #638's fakes"). They could not do that without editing this shared file or copying `run_verb`.
   - **Change:**
     - Add `pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome`, which builds the `VerbCtx` over the given ports.
     - Make `run_verb` call it with ports the harness builds from `Unwired`, for example a `static UNWIRED: Unwired` and `pub fn unwired_ports() -> Ports<'static>` filling all seven fields.
     - Remove the `Wiring` import.
2. **`crates/holler-cli/tests/pane_verbs/main.rs:45-78` and `crates/holler-cli/tests/profile_verbs/main.rs:27-30`: no test in these frozen roots may pin a sibling story's stub or `connect()`'s body.** Their own docs (lines 8-9) say a verb story never edits them.
   - Delete `seam_pane_stub_routes_text_to_err_and_json_to_out` (`pane_verbs/main.rs:45-48`). It is an exact duplicate of `pane_verbs/list.rs:14-17`.
   - Delete `seam_profile_stub_routes_text_to_err_and_json_to_out` (`profile_verbs/main.rs:27-30`). It is an exact duplicate of `profile_verbs/list.rs:6-9`.
   - Move `seam_json_mode_with_shared_flags_is_still_one_envelope` (`pane_verbs/main.rs:67-78`, which pins #644's `pane launch` stub) into `pane_verbs/launch.rs`.
   - Rewrite `stub_wiring_ports_answer_not_implemented` (`pane_verbs/main.rs:50-65`) to assert on `Unwired`, for example through item 1's `unwired_ports()`, instead of `Wiring::connect()`.

   AC 6 stays covered by the 20 per-verb stub cases. Each runs a stub through the shared harness in both formats.
3. **`crates/holler-cli/tests/pane_verbs/process/`: no shared-file test may use a sibling's stub line as its proof, and the stub list must be grouped by story.**
   - **`flags.rs:24-40` (`parses()`, "exit 1 with the owning story's `not implemented` line"), used by all six tests at lines 56-208:**
     - Check parsing in-process with `Cli::try_parse_from`, following `cli_surface_test.rs:25-29`.
     - Assert each refused flag (`--profile` on `pane import` and the profile verbs, `--spec-only` off launch/relaunch/close, `--take-over` off `profile apply`) as clap `ErrorKind::UnknownArgument`. That also proves the verb exists and survives a story adding positionals.
     - For the positive "every spec flag parses on launch and relaunch" matrix, either put it in `pane_verbs/launch.rs` and `relaunch.rs` (#644's files) or extend #644's fixture block. Today the fixture covers every spec flag on `launch` but not on `relaunch`.
   - **`usage.rs`:** `:69-77` `spec_only_with_a_profile_parses_and_reaches_the_stub` (#644), `:109-118` `format_text_alone_is_the_default_text_mode` (#643) and `:146-168` `each_argv_form_alone_parses` (#644) all assert a sibling's stub line.
     - Check the parse without the stub line, or move the case to the owning verb's file.
     - Check `--format=text` through `holler_cli::output::resolve_format` or the parsed `Cli.format`.
   - **`main.rs:32-59` (`PANE_VERBS`/`PROFILE_VERBS`) with `stub.rs:19-74`:** AC 2's `stub_verb_not_implemented` and its JSON twin legitimately list every stub, so every story must delete its own rows.
     - Give those two tests their own stub table, grouped by owning story with a blank or `// #NNN` line between groups, so each story deletes only its own group and touches no line next to another story's.
     - Keep a permanent verb list for `help_lists_every_verb_and_exits_0` and `docs_rows.rs`, so removing a stub does not drop the verb from the help, ADR and fixture checks.
     - Optional: derive `docs_rows.rs:56-71`'s `groups` from the same story table instead of a second copy.

After the rework, T re-runs `pane_verbs`, `profile_verbs`, `pane_cli_process`, `cli_surface_test`, clippy `-D warnings` and `rustfmt --check --edition 2021` on the touched test files. Then T confirms, by grep, that:
- no test in `pane_verbs/main.rs`, `profile_verbs/main.rs`, `verb_harness/mod.rs` or `process/{flags,usage}.rs` contains `not implemented (story #` or `Wiring::connect`;
- outside each verb's own file, only the grouped stub table in `process/` names a sibling's story.

## Advisory notes

None of these block the verdict.

- **Rebase before the PR.** `origin/main` gained #669 (af3d8df) after this branch's base. `git merge-tree` shows a textual conflict in `CHANGELOG.md`: both stories add an `[Unreleased]` bullet after #637's. `Cargo.lock` auto-merges. #669 changed only `holler-hub` internals; `serve::run`'s signature is unchanged. Keep both bullets and re-run the workspace tests after the rebase.
- **PR disclosure.** The script's commit trailers carry no session link. Per `CLAUDE.md`, add the AI disclosure to the PR body with `gh pr edit` once the PR is open (`CONTRIBUTING.md`, "AI-assisted contributions").
- **For #649:** `main.rs:262` calls `Wiring::connect()` before every verb. If the real `connect` dials eagerly, every verb that is still a stub answers `unavailable` instead of `not implemented (story #N)` when no hub runs, and `process/stub.rs` breaks. A lazy connect avoids that (A-dup W-2 note).
- **For #634:** `docs/protocol/v2.md` §10 (line 710) says it reproduces ADR 0003's table verbatim. It had already drifted, and this change widens the gap. Resync §10 or drop "verbatim".
- **For #644:**
  - one PROVIDER/ID parser and one `SpecValues`-to-`ProfileSpec` merge, not one per verb file (A-dup W-4);
  - `parse_role`'s message and `--role`'s doc list `agent or orchestrator` by hand;
  - `--command-arg`/`--check-arg` take no hyphen-leading value (`--command-arg=--port` works).
- **For #648:** `roster --format=json` prints the legacy `--json` document until #648 reads `FormatChoice.json_explicit`.
- **Optional cleanups (A-dup W-3/W-5):**
  - the one-envelope check exists in `verb_harness::one_envelope`, `process::Out::envelope` and inline in `stub.rs`;
  - exit codes 1 and 2 are spelled in `output::exit_code`, `prompt_target` and `main.rs`;
  - `flatten` repeats `one_line`;
  - the stdio `Sink` is built three times in `main.rs`;
  - `interrupt | io/alpha` now appears twice in the fixture (lines 55 and 178).
- **Headroom:** `cli.rs` is at 780 lines (warn 600, fail 900). `Unwired::run_probe` answers `ProbeResult::Error("not implemented")` with no story number, because `ProbeResult` is not a `PaneError`.
