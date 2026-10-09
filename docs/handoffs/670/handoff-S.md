# Handoff-S: #670 the pane/profile CLI skeleton (skeleton slice c of epic #633), spec audit, pass 3

**Date:** 2026-10-09 (09:16 MDT)
**Branch:** issue-670-implementation (at b160a40; merge base f2602ba; `origin/main` is now 094ebfa: #669 at af3d8df, then #668, then #634's ADR-0021)
**Issue:** #670 (epic #633)
**Brief:** `docs/handoffs/670-brief.md` (Revision 1)
**Pass:** 3. Pass 2 (REWORK, production) is this file at 7939b54, and pass 1 (REWORK, test-only) is this file at 4776239. Since pass 2:
- F's rework 1 (02a3655): `src/prompt_target.rs`, +15 / -2.
- T's rework 2 (4876a4c): four test files and two manifest comments.
- A's anti-duplication gate, pass 3 (b160a40): PASS, 1 warn.

Nothing else under `src/`, nor the fixture, ADR 0003, `Cargo.lock` or `CHANGELOG.md`, has changed since pass 2.

**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`;
- `handoff-F.md` ("Rework 1" and pass 1);
- `handoff-T-green.md` (with "Test-only rework" and "Rework 2");
- `handoff-A-dup.md` (pass 3), pass 2 of this file, `decisions.md`, `evidence.md` (with "Rework 1");
- the body of issue #670, epic #633, issue #676, and ADR-0021 as merged on `origin/main`.

**Diff audited:** `git diff origin/main...HEAD` (82 paths).
- Read in full: every new or changed `src` file.
- Read in full, tests: `verb_harness/{mod,parse}.rs`, `pane_verbs/{main,list,launch,relaunch,get,close,spec_flags,target_flags,output_api}.rs`, `profile_verbs/{main,list,apply}.rs`, `process/{main,stub,usage,flags,legacy_verbs,docs_rows}.rs` and the two placeholder targets.
- Read as diffs: the manifest, the fixture, ADR 0003, `CHANGELOG.md` and `Cargo.lock`.

**Read-only fact checks (this pass):**
- `git diff --name-only` against the blast radius, and `wc -l` on the touched files.
- Greps of the added lines for `unwrap`/`expect`/`panic`/`unreachable`, `#[allow`, `process::exit`, private names, IPs and secrets.
- `rustfmt --check --edition 2021` on all 62 new `.rs` files. A second check compares the lines rustfmt would change in the 6 edited `src` files with the lines this branch added there.
- The built binary over an empty, isolated state dir, with no hub. It was built at 08:51 MDT, after the last `src` commit (02a3655, 08:50 MDT); `git diff 02a3655 HEAD -- crates/holler-cli/src` is empty.
- A differential probe, in a scratch crate outside the repo on the workspace's clap 4.6.6. It compares `origin/main`'s `Say`/`Interrupt`/`Answer` with this branch's shapes plus a copy of `resolve_tail`, over 50 argv forms.
- Two scratch git repos for the `use`-line layout of `legacy_verbs.rs`.
- `git merge-tree` against `origin/main` (094ebfa).

## A precondition

Met.
- `handoff-A.md` (plan review): **PASS**, 13 warns, no block.
- `handoff-A-dup.md`, pass 3 (f2602ba...4876a4c): **PASS**, 1 warn (W-1: pass-2 item 3 below is not settled), no block.

## T precondition

Met.
- **RED** (`handoff-T-red.md`):
  - `pane_cli_process`: 28 of 33 tests failed on missing behaviour; the 5 that passed are deliberate guards.
  - `pane_verbs` and `profile_verbs` failed to build only on the API the brief commits F to create.
  - `cli_surface_test` failed on the 20 new leaves.
- **RED for the rework tests** (`handoff-T-green.md`, "Rework 2"), against `git show 51f3bed:crates/holler-cli/src/prompt_target.rs`:
  - `an_extra_positional_split_off_the_session_form_by_a_flag_is_refused` failed: `resolve` returned `Ok`.
  - `a_malformed_pane_form_is_a_usage_error_not_a_refusal` failed on its two new rows: exit 1 on the hub line, and exit 1 on the #646 refusal.
  - The guard test `a_flag_between_session_and_text_still_resolves_to_both` passed both ways, by design.
- **GREEN** (on 02a3655): no blocking issues.
  - Workspace: 1031 passed, 0 failed, 5 ignored.
  - `pane_verbs` 61, `profile_verbs` 8, `pane_cli_process` 34, `docs_cli_test` 3, `cli_surface_test` 3.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check.sh` and `cargo machete` are clean.
- T's first workspace run failed 7 `interrupt_test` cases in `wait_warm` (`reconnecting`); the second run was green. This is open issue #420, a known flaky `interrupt_test` (advisory 4).

## Pass-2 REWORK: status

1. **F, `prompt_target.rs`: resolved.**
   - `resolve_tail`'s SESSION arm (`prompt_target.rs:170-180`) returns the existing `Usage` when the tail holds more than two positionals. The `--pane` arm is unchanged. The doc at lines 149-155 now states the clap limit correctly, so the module doc's "a missing or surplus positional is the existing [`Usage`] error" (lines 8-9) holds.
   - **The binary:**
     - `say io/alpha hello --queue extra`, `interrupt io/alpha --server ws://127.0.0.1:1 stop now` and `answer io/alpha --server ws://127.0.0.1:1 1 2` exit 2. The message is `only SESSION and TEXT|CHOICE may be given, got 3 positionals: ...`, stdout is empty, and no hub line appears.
     - `say io/alpha hello extra` is still clap's exit 2.
     - `say io/alpha --queue hello` still reaches the hub path (`no live holler hub reachable`, exit 1).
   - **The differential probe: 50 forms, 0 differences.**
     - Every form that `origin/main` accepts resolves to the same session, text and flags. The forms include `--`, hyphen values, `--parts-file` before or after the tail, flags between the positionals, and `--server` in every position.
     - Every form that `origin/main` refused is still refused. The split forms are now refused by the accessor's `Usage` and the contiguous ones by clap, all with exit 2.
2. **T, `process/stub.rs`: resolved.**
   - The `STUBS` doc (lines 12-16) tells a story to keep its `// #NNN` line.
   - The refusal constants keep `// #646`, `// #648` and the blank line between them (lines 49-57). A-dup pass 3 merged that layout cleanly.
3. **T, `process/legacy_verbs.rs:10-11`: not resolved as asked.**
   - **What pass 2 asked:** "its own `use` line, **with an unchanged line between them**, or path-qualify each constant".
   - **What T did:** the import is now two lines, but the two lines are adjacent.
   - **Reproduced:** I copied the file into a scratch repo. Deleting line 10 on one branch (#646) and line 11 on another (#648) gives `CONFLICT (content)` on merge.
   - **The fix works:** with `// #646` above line 10 and `// #648` above line 11, the file is `rustfmt --check --edition 2021` clean and the same two deletions merge cleanly.
   - **T's journal entry is wrong on both counts.** The "Assumed" of T's rework-2 entry in `decisions.md` says adjacent lines do not conflict and rustfmt would not keep a separating line. Both are false.
   - **Not carried as a REWORK.** The reasons are under Verdict, and the fix is advisory 1.
4. **T, comments: resolved.**
   - `process/main.rs:9-16` no longer lists the flag matrix among what only the binary can show.
   - `Cargo.toml:477-479` now says the in-process targets run over the ports a test gives them, `Unwired` by default. `Cargo.toml:488-494` now gives the reason that `pane_cli_process` is its own target.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | build, clippy, test, machete, `lint.sh`, `changelog-check.sh` pass; no file at or over 900 lines; `main()` ≤ 100 code lines; new `.rs` files rustfmt-clean; existing files not reformatted | **T-green, Rework 2:** 1031 passed, 0 failed; clippy, lint, changelog and machete clean.<br>**My checks:**<br>- the largest touched source file is `cli.rs`, at 780 lines;<br>- `main()` is `main.rs:101-113`, 6 code lines;<br>- all 62 new `.rs` files are rustfmt-clean. The only diffs are in the pre-existing `tests/support/*`, which `process/main.rs` reaches through `#[path]`; this branch does not touch them;<br>- in the 6 edited `src` files, no line that rustfmt would change is a line this branch added. | MET |
| 2 | 20 stub verbs: text is `error: not implemented (story #NNN)` on stderr, empty stdout, exit 1; JSON is one `not-implemented` envelope, exit 1; a bare namespace exits 2; `--debug bogus` exits 3 | **`process/stub.rs`:**<br>- `stub_verb_not_implemented`: 20 verbs, from `STUBS`;<br>- `stub_verb_not_implemented_json_is_one_envelope`: 20 verbs × 4 spellings, with the exact compact prefix;<br>- `a_bare_namespace_is_a_usage_error`;<br>- `a_bad_debug_value_is_still_a_policy_refusal_before_dispatch`.<br>**In process:** the 20 per-verb `*_stub_routes_text_to_err_and_json_to_out` cases.<br>**Story numbers:** they match the epic's wave table.<br>**Binary:** I checked `pane list` and `pane list --json` (#643) and `profile apply --take-over --format json` (#664). | MET |
| 3 | Usage errors exit 2, and under `pane`/`profile` in JSON mode they are a `usage` envelope; legacy verbs untouched; `--help`/`--version` exit 0 | **`process/usage.rs`:**<br>- `spec_only_requires_profile`;<br>- `a_bad_format_value_is_a_usage_error`;<br>- `json_conflicts_text`;<br>- `command_arg_and_command_json_are_mutually_exclusive`, which covers both pairs;<br>- `an_unknown_flag_under_pane_or_profile_is_an_envelope_in_json_mode`;<br>- `the_envelope_message_keeps_the_reason_clap_gave`;<br>- `an_unknown_flag_in_text_mode_is_clap_s_own_message`;<br>- `a_legacy_verbs_json_usage_error_is_untouched`.<br>**`process/stub.rs`:** `help_lists_every_verb_and_exits_0`, `top_level_help_names_pane_and_profile`.<br>**Existing:** `cli_invocation_test`.<br>**Binary:**<br>- bare `holler`, `holler --json` and `holler bogus --json` give clap's message, exit 2, empty stdout;<br>- `pane bogus --json` and `pane get demo-c1r1 --json` give a `usage` envelope, exit 2. | MET |
| 4 | `--pane`/`--profile` shapes parse and resolve by value; existing forms resolve as before; #646/#648 refusals in plain text before any hub; missing or extra positionals are `cli::Usage`, exit 2 | **`pane_verbs/target_flags.rs`** (18 tests):<br>- resolution by value for the SESSION forms, the `--pane` forms and `--pane --profile`;<br>- `Usage` for a missing SESSION, TEXT or CHOICE, for a SESSION with `--pane`, and for a surplus after `--pane NAME TEXT`;<br>- **new:** `an_extra_positional_split_off_the_session_form_by_a_flag_is_refused`, with RED shown against 51f3bed;<br>- **new:** `a_flag_between_session_and_text_still_resolves_to_both`.<br>**`process/legacy_verbs.rs`** (5 tests):<br>- the refusals under 4 formats;<br>- `roster --profile` refused for #648;<br>- `a_malformed_pane_form_is_a_usage_error_not_a_refusal`, now 11 rows, with the 2 split-form rows (RED shown), and it fails on any `no live holler hub` line.<br>**Unchanged and green:** `talk_test`, `interrupt_test`, `answer_cli_test`.<br>**My differential probe:** 50 forms, 0 differences. | MET |
| 5 | Spec flags parse on `launch`/`relaunch`; `--profile`, `--spec-only` and `--take-over` only where specified; `validate()` types the values with the guards' codes | **Flag placement:**<br>- `pane_verbs/{launch,relaunch}.rs`: `pane_*_accepts_every_spec_flag`, over `SPEC_FLAG_SETS`;<br>- `process/flags.rs` (5 tests: in-process, `UnknownArgument` for a refused flag);<br>- `usage.rs`: `spec_only_with_a_profile_parses`, `each_argv_form_alone_parses`.<br>**Validation:** `pane_verbs/spec_flags.rs` (8 tests):<br>- `grid-ambiguous`, `grid-out-of-range`;<br>- `profile-secret-refused` and `env-name-invalid` (as the issue words it);<br>- `command-not-argv` for both JSON flags, and `usage` for text that is not JSON;<br>- the secret is never echoed. | MET |
| 6 | One seam test per verb binary through `verb_harness`; `emit`/`emit_stream`/`emit_usage_error` routing; `ErrorCode::new` uses `is_valid_code`; `ErrorCode::from(&PaneError)` | - 20 per-verb `assert_stub_routes` cases through `run_verb` over `unwired_ports()`;<br>- `run_verb_with` takes any `Ports`;<br>- `pane_launch_json_mode_with_shared_flags_is_still_one_envelope`;<br>- `pane_verbs/output_api.rs` (18 tests), including `error_code_new_agrees_with_the_one_validator`;<br>- `unwired_ports_answer_not_implemented`. | MET |
| 7 | ADR 0003 rows (one per verb, bare, grouped by story with a blank line) and fixture lines (grouped under `# #NNN`, every leaf and shared flag, the two regression lines); `docs_cli_test` and `cli_surface_test` pass | - `process/docs_rows.rs` (6 tests): `adr_0003_has_one_row_per_pane_and_profile_verb`, `adr_0003_rows_of_different_stories_are_never_adjacent`, `adr_0003_mentions_the_new_flags_and_the_amended_sentences`, `the_fixture_covers_every_new_leaf_and_every_shared_flag`, `the_fixture_groups_new_lines_by_owning_story`, `every_verb_has_exactly_one_owning_story`;<br>- `docs_cli_test` (3) and `cli_surface_test` (3) are green per T-green;<br>- I read the ADR and fixture diffs. | MET |
| 8 | `git diff --name-only origin/main...HEAD` lists only blast-radius paths | My grep: 82 paths, 0 outside the radius. | MET |

## Spec compliance

The brief's decisions are implemented as stated. The production code is unchanged since pass 2 except `resolve_tail`.
- **Decision 1** (`main.rs`):
  - `Cli::try_parse()` is used, the format resolver is called once, and `output.rs` never exits.
  - The usage envelope appears only under `pane`/`profile` in JSON mode. The value-taking global flags come from the clap tree (`Cli::global_value_flags`).
  - The `roster --profile` refusal is in `main.rs::run_roster`, so `roster_cmd.rs` is untouched.
  - Dispatch is one exhaustive `match`; `main()` is 6 code lines.
- **Decision 5** (the `--pane` shapes) and the issue's Scope ("Missing or extra positionals return the existing `cli::Usage` (exit 2)"):
  - These now hold for both arms, and for a tail that a flag splits.
  - Clap still refuses the contiguous `say io/alpha hello extra` itself.
  - There is no `trailing_var_arg`, so `--parts-file`, `--queue`, `--grant` and `--server` after SESSION still parse.
  - The three `*_cmd.rs` files changed only to read the accessors and to call `route` first.
- **Decision 6:** ADR 0003 has:
  - one bare row per verb, grouped by story with blank lines, and the #665 rows marked proposed;
  - `--format` on the global-flags line;
  - the sentence that says each owning story adds its own flags;
  - the amended "only top-level verbs" sentence and the amended `--json` line;
  - #633 and ADR-0021 cited.
- **Decision 7:** the signatures as built are in `handoff-F.md` ("Architecture notes for A").
- **Decision 9:** `--format=json` is `--json` on every verb, `--json --format=text` exits 2, and `FormatChoice.json_explicit` exists for #648.
- **Decision 10:**
  - one `Args` struct per verb file;
  - only `SpecFlags`, `ProfileOpt` and `SpecOnly` in `pane/args.rs`, and `--take-over` in `apply.rs`;
  - no sibling positional guessed (`pane get demo-c1r1` is a usage error, as the brief expects);
  - `output::Envelope` is never re-exported at the root.

Deviations recorded in `decisions.md`, accepted as in pass 2:
- exhaustive `match` dispatch (A W-9);
- `resolve_format(json, format)` instead of `resolve_format(&Cli)` (A W-8);
- `VerbCtx` in `output.rs`, holding `Ports` by value (A W-7);
- `Wiring::connect()` and `Unwired` (A W-5);
- `--ctx-soft`/`--ctx-hard` as `u32`;
- separate `--pane` rows in ADR 0003 under #646;
- an envelope message without the `error: ` prefix (this matches ADR-0021's example);
- the `mod.rs` layout (A W-10).

The documents that landed on `origin/main` after this branch was cut agree with it:
- ADR-0021 (094ebfa) says #670 creates `output.rs` with every error mapped to exit 1, and that a follow-up moves refusals to exit 3.
- The epic says the same ("Decisions taken", item 5).
- #676 is that follow-up, and #670's tests are in its scope.

So #670's exit 1 for a refusal is what the specification asks for now.

## Quality audit

- **Correctness and failure handling:**
  - The pass-2 regression is fixed and proven at both levels.
  - A malformed tail is a usage error before the #646 refusal, `--timeout`, `--parts-file` or any hub. `route()` is the first statement of each verb.
  - `--pane`/`--profile` are refused in plain text under every format.
  - Dispatch is exhaustive, and `--debug bogus` still exits 3 before dispatch.
  - `emit` never writes half an envelope, keeps the JSON message on one line, and turns a failed write into a non-zero exit. `emit_stream` stops at the first failed write.
  - There is no state, so there is no concurrency concern.
- **Build guards:**
  - No `unwrap`, `expect`, `panic`, `unreachable` or `unsafe` in the added `src` lines.
  - The 15 added `#[allow]` lines are all on the three test-crate roots, and each carries `// #670`.
  - `process::exit` appears only in `main.rs`.
  - The largest touched source file is `cli.rs`, at 780 lines (warn at 600, fail at 900).
  - `dead_code` and `cargo machete` are clean per T-green.
- **Formatting:** see AC 1. No existing file was reformatted.
- **Protocol:** no wire, golden-file, `holler-proto`, `holler-hub` or `holler-pane` change, and no error-code table change. `docs/protocol/v2.md` is out of scope (advisory 5).
- **Tests:**
  - The real binary runs over an isolated `StateDir`, with no sleeps.
  - RED-first evidence exists for the original tests and for the rework tests.
  - The new tests assert behaviour: the accessor's `positionals` message, exit 2, no hub line. They are not tautologies: they fail against 51f3bed.
- **Documentation:**
  - The `CHANGELOG.md` `## [Unreleased]` entry (Enhancements) links #670.
  - ADR 0003 carries decision 6.
  - The README needs nothing: the verbs are stubs, and the operator guide is #652's.
- **Public-repository privacy:** clean.
  - The new code, tests, fixture and ADR rows use `demo`, `demo-c1r1`, `/srv/demo`, `127.0.0.1` and the established `io/alpha` placeholder, which `origin/main` already uses dozens of times.
  - `hj-` appears only as quotations in the handoff docs; those names are already on `main`.
  - No secret, hostname, tailnet name, private IP, home path or account name appears in an added line. I also checked the committed handoffs.
- **Commit and PR hygiene:**
  - Every subject is a Conventional Commit (`docs(#670)`, `chore(#670)`).
  - Every commit has a `Co-Authored-By` trailer without a session link. That is the script's convention (#669 merged the same way).
  - The PR is not open yet, and the branch is not pushed (advisory 3).

## Scope check

**Production matches the brief's scope.**
- All 82 paths are inside the blast radius.
- Nothing in `holler-hub`, `holler-pane`, `holler-proto`, any golden file or any ADR other than 0003 changed.
- F's rework touched one file. T's rework touched test files and manifest comments only.

**Additions beyond the brief's wording, each recorded:**
- the `pane_cli_process` target and `tests/verb_harness/parse.rs`;
- `emit_error` and `not_implemented(_message)`;
- `emit`'s write rules;
- `Cli::global_value_flags`;
- `validate()` refusing an arg flag together with a JSON flag for a hand-built `SpecFlags`.

**Under-delivery:** none. The AC 4 gap of pass 2 is closed.

## Verdict

**PASS.** All eight acceptance criteria are met, and the brief's decisions are implemented, with the deviations recorded and accepted. The pass-2 production item is fixed and proven.

Pass-2 item 3 is the one item left open (above). I do not carry it as a REWORK, for three reasons:
- **It is not a requirement.** No acceptance criterion or decision of the brief or issue requires it. AC 7's separator rule covers the ADR rows and the fixture lines, and both are correct.
- **Pass 2 said so.** It said items 2-4 "would not have forced a REWORK on their own".
- **Its cost is small.** At worst, whichever of #646 and #648 lands second resolves one trivial rebase conflict, and #646 has to rewrite the four refusal tests around those lines in any case.

The fix is two comment lines (advisory 1), and the record is corrected in `decisions.md`.

Context, not the reason for the verdict: a third consecutive S REWORK would reach the script's per-gate threshold (`PER_GATE_BLOCK_THRESHOLD = 3`) and stop the run.

## Advisory notes

None of these block the verdict.

1. **Pass-2 item 3, the fix.** In `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs`:
   - put `// #646` above line 10 (`use crate::stub::PANE_FORM_REFUSAL as PANE_REFUSAL;`);
   - put `// #648` above line 11 (`use crate::stub::ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL;`);
   - each story then deletes its own `use` line and keeps its comment line.

   This layout is rustfmt-clean and the two parallel deletions merge cleanly (both verified). It can go into this PR before the merge, or #646 can make it first.
2. **Rebase before the PR.**
   - `git merge-tree` against `origin/main` (094ebfa) conflicts in `CHANGELOG.md` only: #669 added its own `[Unreleased]` bullet after #637's. Keep both bullets. `Cargo.lock` auto-merges.
   - After the rebase, `docs/adr/ADR-0021.md` exists, so ADR 0003's "ADR-0021 (#634)" can become a link (optional).
   - Re-run the workspace tests after the rebase.
3. **PR disclosure.**
   - Once the PR is open, add the AI disclosure to its body with `gh pr edit`, as `CLAUDE.md` asks (`CONTRIBUTING.md`, "AI-assisted contributions").
   - The squash commit can carry the `Co-Authored-By` trailer with a session link, which the script's commits do not.
4. **CI flake.** T saw 7 `interrupt_test` failures in `wait_warm` (`reconnecting`) in one workspace run and none in the next. This is the open issue #420. `interrupt_cmd.rs` runs after the warm-up, so a failure there is not this change.
5. **`docs/protocol/v2.md` §10.** #634 merged (094ebfa) without resyncing it. Line 710 still says §10 "reproduces [ADR 0003's] table verbatim". The table already had no `--server` forms, and now it has no `pane`/`profile` rows either. No open story owns this. File a docs follow-up that resyncs §10 or drops "verbatim".
6. **For #649:** `main.rs:262` calls `Wiring::connect()` before every pane/profile verb. If the real `connect` dials eagerly, then while no hub runs, every verb that is still a stub answers `unavailable` instead of `not implemented (story #N)`, and `process/stub.rs` breaks. A lazy connect avoids that.
7. **For #644:**
   - one PROVIDER/ID parser and one `SpecValues`-to-`ProfileSpec` merge, not one per verb file;
   - `parse_role`'s message and `--role`'s doc list `agent or orchestrator` by hand;
   - `--command-arg`/`--check-arg` take no hyphen-leading value (`--command-arg=--port` works).
8. **For #646:**
   - `holler say --help` renders the tail as `[SESSION] [TEXT]...`, and the help does not show the limit of two positionals that `resolve_tail` now enforces. A `help` line on `rest` could state it.
   - The limit is written twice: in clap's `num_args` and in `resolve_tail`. F recorded the coupling.
9. **For #648 and #676:**
   - **#648:** until it reads `FormatChoice.json_explicit`, `roster --format=json` prints the legacy `--json` document.
   - **#676:** it changes `emit`'s exit codes and the #670 tests that assert exit 1 for a refusal.
10. **Optional cleanups** (A-dup pass-2 W-2 and pass-1 W-5, unchanged):
    - "prepend `holler`, then `try_parse_from`" is written three ways;
    - `assert_spec_flags_accepted` repeats `assert_no_failures`;
    - `usage.rs::spec_only_with_a_profile_parses` repeats a `flags.rs` check;
    - the one-envelope check exists in three places;
    - exit codes 1 and 2 are spelled out in `output.rs`, `prompt_target.rs` and `main.rs`;
    - `flatten` repeats `one_line`;
    - the stdio `Sink` is built three times in `main.rs`;
    - `interrupt | io/alpha` is in the fixture twice (lines 55 and 178).
11. **Small gaps:**
    - `unwired_ports_answer_not_implemented` (`pane_verbs/main.rs:41-52`) checks 3 of the 7 ports, although its doc says every port.
    - `Unwired::run_probe` answers `ProbeResult::Error("not implemented")` with no story number.
    - `Cargo.toml:488-494` lists "which flag parses where" among the binary's tests, but `flags.rs` asks clap in-process.
    - `cli.rs` is at 780 lines.
