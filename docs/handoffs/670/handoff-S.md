# Handoff-S: #670 the pane/profile CLI skeleton (skeleton slice c of epic #633), spec audit, pass 2

**Date:** 2026-10-09
**Branch:** issue-670-implementation (at 51f3bed; merge base f2602ba; `origin/main` is now 1c48c30: #669 at af3d8df, then #668)
**Issue:** #670 (epic #633)
**Brief:** `docs/handoffs/670-brief.md` (Revision 1)
**Pass:** 2. Pass 1 (REWORK, test-only) is this file at 4776239. Since then: T's test-only rework (56883d6) and A's anti-duplication gate, pass 2 (51f3bed). Nothing under `src/`, the manifest, the fixture, ADR 0003, `Cargo.lock` or `CHANGELOG.md` has changed since F's commit (016eeaa).
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` (with its "Test-only rework" section), `handoff-A-dup.md` (pass 2), pass 1 of this file, `decisions.md`, `evidence.md`, and the body of issue #670.
**Diff audited:** `git diff origin/main...HEAD` (82 paths).
- Read in full: every new or changed `src` file.
- Read in full, tests: `verb_harness/{mod,parse}.rs`, `pane_verbs/{main,launch,relaunch,list,get,target_flags}.rs`, `profile_verbs/{main,list,apply}.rs` and `process/{main,stub,usage,flags,legacy_verbs,docs_rows}.rs`.
- Read as diffs: the manifest, the fixture, ADR 0003 and the CHANGELOG.

**Read-only fact checks:**
- `git diff --name-only` against the blast radius, and `wc -l` on the touched files;
- greps of the added lines for `unwrap`/`expect`/`panic`, `#[allow`, `process::exit`, private names and secrets;
- `rustfmt --check --edition 2021` on the 27 new `src` files and on every new or reworked test file;
- `git merge-tree` against `origin/main`;
- the built binary over an empty, isolated state dir, with no hub. It was built at 07:29 MDT, after F's last `src` commit at 07:21 MDT;
- a scratch crate outside the repo, on the workspace's clap 4.6.6, comparing `origin/main`'s `Say` positionals with this branch's tail;
- a scratch git repo for the adjacent-deletion conflict that A-dup W-1 describes.

## A precondition

Met.
- `handoff-A.md` (plan review): **PASS**, 13 warns, no block.
- `handoff-A-dup.md`, pass 2 (f2602ba...56883d6): **PASS**, 3 warns, no block.

## T precondition

Met.
- **RED** (`handoff-T-red.md`):
  - `pane_cli_process`: 28 of 33 tests failed on missing behaviour. The 5 that passed are deliberate guards.
  - `pane_verbs` and `profile_verbs` failed to build only on the API the brief commits F to create.
  - `cli_surface_test` failed on the 20 new leaves.
  - Every target was declared in the same commit.
- **GREEN** (`handoff-T-green.md`): no blocking issues. After the test-only rework:
  - workspace: 1029 passed, 0 failed;
  - `pane_verbs` 59, `profile_verbs` 8, `pane_cli_process` 34;
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh` and `cargo machete` are clean;
  - mutation spot-checks make the tests fail.

## Pass-1 REWORK: resolved

A-dup pass 2 confirms all three items, and so do my reads and greps:

1. **Harness.** `verb_harness::run_verb_with(argv, format, ports)` takes the ports. `run_verb` runs over `unwired_ports()`, which is a `static Unwired`. No test names `Wiring`.
2. **Frozen roots.**
   - The two duplicate seam tests are gone.
   - The JSON case with the shared flags is now in `pane_verbs/launch.rs`.
   - `unwired_ports_answer_not_implemented` replaces the test of `connect()`.
3. **Shared process files.**
   - `process/flags.rs` asks clap in-process and checks for `UnknownArgument`.
   - The positive spec-flag matrix is in `pane_verbs/{launch,relaunch}.rs`, and it now covers `relaunch`.
   - The three `usage.rs` cases no longer read a stub line.
   - `STUBS` is the one story-numbered stub table, grouped under `// #NNN` lines.
   - `PANE_VERBS`, `PROFILE_VERBS` and `STORY_GROUPS` are permanent lists.
   - `not implemented (story #` appears only in `process/stub.rs` and in the parameterised `assert_stub_routes`.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | build, clippy, test, machete, lint and changelog-check pass; no file over 900 lines; `main()` ≤ 100 lines; new `.rs` files rustfmt-clean; existing files not reformatted | **T-green:** Tier 1 table and the rework verification (1029/0; clippy, lint, changelog and machete clean).<br>**My checks:**<br>- the largest touched file is `cli.rs`, at 780 lines;<br>- `main()` is `main.rs:101-113`, 6 code lines;<br>- `rustfmt --check` is clean on the 27 new `src` files and the new or reworked test files. Its only diffs are in the pre-existing `tests/support/{cmds,hold_rig}.rs`, which `process/main.rs` reaches through `#[path]`; this branch does not touch them. | MET |
| 2 | 20 stub verbs: text `error: not implemented (story #NNN)` on stderr, empty stdout, exit 1; JSON is one envelope, exit 1; a bare namespace exits 2; `--debug bogus` exits 3 | **`process/stub.rs`:**<br>- `stub_verb_not_implemented` (20 verbs, from `STUBS`);<br>- `stub_verb_not_implemented_json_is_one_envelope` (20 verbs × 4 spellings, exact compact prefix);<br>- `a_bare_namespace_is_a_usage_error`;<br>- `a_bad_debug_value_is_still_a_policy_refusal_before_dispatch`.<br>**In process:** 20 per-verb `*_stub_routes_text_to_err_and_json_to_out`. The story numbers match the epic's wave table. | MET |
| 3 | Usage errors exit 2, and are a `usage` envelope in JSON mode under `pane`/`profile` only; `--help`/`--version` unchanged | **`process/usage.rs`:**<br>- `spec_only_requires_profile`;<br>- `a_bad_format_value_is_a_usage_error`;<br>- `json_conflicts_text`;<br>- `command_arg_and_command_json_are_mutually_exclusive` (both pairs);<br>- `an_unknown_flag_under_pane_or_profile_is_an_envelope_in_json_mode`;<br>- `the_envelope_message_keeps_the_reason_clap_gave`;<br>- `an_unknown_flag_in_text_mode_is_clap_s_own_message`;<br>- `a_legacy_verbs_json_usage_error_is_untouched`.<br>**`process/stub.rs`:** `help_lists_every_verb_and_exits_0`, `top_level_help_names_pane_and_profile`.<br>**Existing:** `cli_invocation_test`. | MET |
| 4 | `--pane`/`--profile` shapes parse and resolve by value; existing forms unchanged; #646/#648 refusals in plain text before any hub; **missing or extra positionals are `cli::Usage`, exit 2** (issue, Scope) | **Covered:**<br>- `pane_verbs/target_flags.rs` (16 tests): resolution by value, `Usage` for missing positionals, a SESSION with `--pane`, and a surplus after `--pane NAME TEXT`;<br>- `process/legacy_verbs.rs` (5 tests): the refusals under 4 formats, `roster --profile` to #648, malformed forms exit 2.<br>**Not met:** an extra positional that a flag splits off the SESSION form is accepted and silently dropped (REWORK 1). `target_flags.rs:88-91` tests only the contiguous form. | **NOT MET** |
| 5 | Spec flags parse on launch/relaunch; placement of `--profile`, `--spec-only`, `--take-over`; `validate()` types values with the guards' codes | **Flag placement:**<br>- `pane_verbs/{launch,relaunch}.rs`: `pane_*_accepts_every_spec_flag` (`SPEC_FLAG_SETS`);<br>- `process/flags.rs` (5 tests, in-process `UnknownArgument`);<br>- `usage.rs`: `spec_only_with_a_profile_parses`, `each_argv_form_alone_parses`.<br>**Validation:** `pane_verbs/spec_flags.rs` (8 tests): `grid-ambiguous`, `grid-out-of-range`, `profile-secret-refused`, `env-name-invalid`, `command-not-argv` for both JSON flags, `usage` for text that is not JSON, and the secret is never echoed. | MET |
| 6 | Seam test per verb binary through `verb_harness`; `emit`/`emit_stream`/`emit_usage_error` routing; `ErrorCode::new` uses `is_valid_code`; `ErrorCode::from(&PaneError)` | - 20 per-verb `assert_stub_routes` cases through `run_verb`, now over `unwired_ports()`;<br>- `run_verb_with` takes the given ports;<br>- `pane_verbs/output_api.rs` (18 tests);<br>- `unwired_ports_answer_not_implemented`. | MET |
| 7 | ADR 0003 rows (one per verb, bare, grouped by story) and fixture lines (grouped under `# #NNN`, every leaf and shared flag, the two regression lines); `docs_cli_test` and `cli_surface_test` pass | - `process/docs_rows.rs` (6 tests, including `every_verb_has_exactly_one_owning_story`);<br>- `docs_cli_test` (3) and `cli_surface_test` (3) green per T-green;<br>- I read the fixture and ADR diffs. | MET |
| 8 | `git diff --name-only origin/main...HEAD` lists only blast-radius paths | My grep: 82 paths, none outside the radius. | MET |

## Spec compliance

The production code is unchanged since pass 1 (016eeaa). The brief's decisions are implemented as pass 1 recorded:
- **Decision 1** (`main.rs`):
  - `Cli::try_parse()` is used;
  - the usage envelope appears only under `pane`/`profile`, and the value-taking global flags come from the clap tree;
  - the resolver is called once;
  - `output.rs` never exits;
  - the `roster --profile` refusal is in `main.rs::run_roster`.
- **Decision 6:** ADR 0003 has rows grouped by story, `--format` on the global-flags line, the amended sentences, #633 and ADR-0021 cited, and the #665 rows marked proposed.
- **Decision 7:** the signatures as built are listed in `handoff-F.md`.
- **Decision 9:** `--format=json` is `--json` on every verb; `--json --format=text` exits 2; `FormatChoice.json_explicit` exists for #648.
- **Decision 10:** one `Args` struct per verb file, only the shared groups in `pane/args.rs`, `--take-over` in `apply.rs`, and no sibling positionals guessed.

These deviations are recorded in `decisions.md`, and I accept them:
- exhaustive `match` dispatch (A W-9);
- `resolve_format(json, format)` (A W-8);
- `VerbCtx` in `output.rs` holding `Ports` by value (A W-7);
- `Wiring::connect()` and `Unwired` (A W-5);
- `--ctx-soft`/`--ctx-hard` as `u32`;
- separate `--pane` rows in ADR 0003 under #646;
- the envelope message without `error: `;
- the `mod.rs` layout (A W-10).

**Not implemented as stated: the issue's "extra positionals" rule (decision 5; REWORK 1).**

The requirement:
- The issue's Scope says of the new tail: "Missing or extra positionals return the existing `cli::Usage` (exit 2)."
- Its Acceptance says "the existing forms resolve to the same values as before".
- Decision 5 expects a third positional to stay exit 2.

What the code does instead:
- `num_args = 0..=2` bounds the values of **one occurrence** only. A flag between positionals starts a new occurrence, so clap accepts a third positional there.
- `resolve_tail`'s SESSION arm (`prompt_target.rs:167-175`) reads the first two values and drops the rest.
- Its doc says "clap has already refused a third" (line 152), and the module doc says "a missing or surplus positional is the existing [`Usage`] error (exit 2)" (lines 8-9). Neither statement holds.

| Command | `origin/main` shape | This branch |
|---|---|---|
| `say io/alpha hello --queue extra` | clap `UnknownArgument`: "unexpected argument 'extra' found", exit 2 (probe) | parses as `rest = [io/alpha, hello, extra]` (probe). The binary goes on to the hub (`no live holler hub reachable`, exit 1); with a live hub it sends `hello` |
| `say io/alpha --timeout 5m fix it` | `UnknownArgument` for 'it' (probe) | parses as `rest = [io/alpha, fix, it]` and would send `fix` |
| `interrupt io/alpha --server ws://127.0.0.1:1 stop now` | same fixed-positional shape (`session` + `text`) | parses and reaches the remote path (`credential.json not found`, exit 1); it would redirect with `stop` |
| `answer io/alpha --server ws://127.0.0.1:1 1 2` | same fixed-positional shape (`session` + `choice`) | parses, reaches the remote path, and would answer `1` |
| `say io/alpha hello extra` (no flag in between) | `UnknownArgument` | `TooManyValues`, exit 2 (correct) |

The `--pane` arm is correct: `say --pane demo-c1r1 hello --queue extra` exits 2, because it checks `rest.len() > 1`.

## Quality audit

- **Correctness and failure handling:**
  - **The regression above (REWORK 1).** A malformed `say`/`interrupt`/`answer` that `origin/main` refused now runs against a live session, with part of the user's input dropped. That is the failure the brief's Risks section names for `--pane` ("a form that parses but is ignored fails open"), reached through a positional instead.
  - **Otherwise sound, as in pass 1:**
    - `--pane`/`--profile` are refused before any hub, and a usage error wins over that refusal;
    - dispatch is exhaustive, and `--debug bogus` still exits 3 before dispatch;
    - `emit` never writes half an envelope, keeps the JSON message on one line, and turns a failed write into a non-zero exit;
    - `emit_stream` stops on a broken pipe;
    - there is no state, so there is no concurrency concern.
- **Build guards:**
  - no `unwrap`/`expect`/`panic`/`unreachable` in the added `src` lines;
  - every `#[allow]` (test-crate roots only) carries `// #670`;
  - `process::exit` appears only in `main.rs`;
  - the largest touched file is `cli.rs` at 780 lines (warn at 600, fail at 900);
  - `main()` is 6 code lines;
  - `dead_code` and `cargo machete` are clean per T-green.
- **Formatting:** see AC 1. No existing file was reformatted.
- **Protocol:** no wire, golden-file, `holler-proto`, `holler-hub` or `holler-pane` change.
- **Tests:**
  - **Good:** the real binary runs over an isolated `StateDir`; no sleeps; RED-first evidence is in `handoff-T-red.md`.
  - **Gap (REWORK 1):** no test covers a positional split off by a flag.
  - **Gap (REWORK 2, 3):** two edits in shared files still collide when wave-3 stories land in parallel.
  - **Gap (REWORK 4):** the rework made comments wrong in files that no sibling story edits.
- **Documentation:** the CHANGELOG `## [Unreleased]` entry links #670, and ADR 0003 carries decision 6. The README needs nothing: the verbs are stubs, and the operator guide is #652's.
- **Public-repository privacy:** clean.
  - New code, tests, the fixture and the ADR rows use `demo`, `demo-c1r1`, `/srv/demo` and `127.0.0.1`.
  - `hj-` appears only as quotations inside handoff docs. Those names are already on `main`, in the `holler-pane` and `holler-hub` tests.
  - No secret, hostname, tailnet name, private IP or home path appears in an added line.
- **Commit and PR hygiene:**
  - Subjects are Conventional (`docs(#670)`, `chore(#670)`).
  - Every commit has a `Co-Authored-By` trailer without a session link. That is the script's convention; #669 merged the same way.
  - The PR is not open yet.

## Scope check

Production matches the brief's scope. All 82 paths are in the blast radius, and nothing in `holler-hub`, `holler-pane` or `holler-proto` changed. The rework added `tests/verb_harness/parse.rs`, which is inside `verb_harness/**` and shared by three targets.

Additions beyond the brief's wording, each recorded:
- the `pane_cli_process` target;
- `emit_error` and `not_implemented(_message)`;
- `emit`'s write rules;
- `Cli::global_value_flags`;
- `validate()` refusing an arg flag together with a JSON flag for hand-built `SpecFlags`.

Under-delivery: the issue's "extra positionals" rule (REWORK 1).

## Verdict

**REWORK** (production). Item 1 needs a `src/` change. Items 2-4 touch only test files and manifest comments, which are T's.

1. **F, `crates/holler-cli/src/prompt_target.rs:149-176`: refuse an extra positional in the SESSION form.**
   - **The rule.** In `resolve_tail`'s SESSION arm (lines 167-175), more than two positionals (SESSION, then TEXT or CHOICE) is the existing `Usage` (exit 2), naming what was given. For example: `only SESSION and TEXT may be given, got 3 positionals: ...`. The `--pane` arm stays as it is.
   - **The doc.** Correct lines 151-152. Clap's `num_args` bounds one occurrence, and a flag between positionals starts another, so clap does not refuse a third positional in that case.
   - **The order.** Keep it: the usage error comes before the #646 refusal and before `--timeout`, `--parts-file` or any hub.
   - **Do not break the valid split form.** `say io/alpha --queue hello` must still be session `io/alpha`, text `hello`, as it is on `origin/main` (probe).
   - **Proving tests (T, in T-green; F names them in handoff-F):**
     - **`crates/holler-cli/tests/pane_verbs/target_flags.rs`:**
       - `resolve()` is an error for `say io/alpha hello --queue extra`, `say io/alpha --timeout 5m fix it`, `interrupt io/alpha --server ws://127.0.0.1:1 stop now` and `answer io/alpha --server ws://127.0.0.1:1 1 2`;
       - `say io/alpha --queue hello` still resolves to (`io/alpha`, `hello`).
     - **`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs`** (in `a_malformed_pane_form_is_a_usage_error_not_a_refusal` or beside it): `say io/alpha hello --queue extra` exits 2, with an empty stdout and no `no live holler hub` line.
     - **Proof the test proves something:** show that the in-process test fails against the current `resolve_tail` (51f3bed).
2. **T, `crates/holler-cli/tests/pane_verbs/process/stub.rs:12-14` (and the two refusal constants at 47-53): tell a story to keep its separator.**
   - **Problem.** The doc says a story "deletes exactly its own group". If #643 and #644 each delete their group together with its `// #NNN` line, the two deletions are adjacent and git reports a conflict.
   - **Reproduced.** In a scratch repo, deleting whole groups with their headers conflicted; deleting only the entries merged cleanly.
   - **Change.** Reword the doc: a story deletes its own entries and keeps its `// #NNN` line. Likewise, the `// #646` and `// #648` lines and the blank line between the two constants stay.
3. **T, `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:10`: one `use` line per story.**
   - **Problem.** One line imports both #646's `PANE_FORM_REFUSAL` and #648's `ROSTER_PROFILE_REFUSAL`. The test root allows `dead_code` but not `unused_imports`. So whichever story removes its refusal test must also edit this line, or clippy `-D warnings` fails, and #646 and #648 both edit the same line.
   - **Change.** Give each constant its own `use` line, with an unchanged line between them, or path-qualify each constant where it is used.
4. **T, comments the rework made wrong, in files no sibling story edits:**
   - **`crates/holler-cli/tests/pane_verbs/process/main.rs:9-13`** lists "that the flag matrix parses" among what "only the binary can show". `flags.rs` now asks clap in-process, and the positive matrix is in `pane_verbs/{launch,relaunch}.rs`.
   - **`crates/holler-cli/Cargo.toml:477-478`** says the in-process targets run over "the stub wiring's `Ports`". They now run over the ports they are given, `Unwired` by default.
   - **`crates/holler-cli/Cargo.toml:487-492`** says `pane_cli_process` tests "the flag matrix" and is "its own target so that it builds without the in-process API". It now builds `tests/verb_harness/parse.rs`, which imports `holler_cli::Cli` and `holler_cli::output`. State the reason that still holds, or drop the clause.

After the rework, T runs:
- `cargo test --workspace`, which includes `talk_test`, `interrupt_test`, `answer_cli_test`, `cli_invocation_test`, `cli_surface_test`, `docs_cli_test` and the three #670 targets;
- clippy `-D warnings`;
- `rustfmt --check --edition 2021` on the touched new files.

## Advisory notes

None of these block the verdict.

- **Rebase before the PR.** `git merge-tree` against `origin/main` (1c48c30) still conflicts in `CHANGELOG.md`: #669 added its own `[Unreleased]` bullet after #637's. `Cargo.lock` auto-merges. Keep both bullets and re-run the workspace tests after the rebase.
- **PR disclosure.** The script's commit trailers carry no session link. Per `CLAUDE.md`, add the AI disclosure to the PR body with `gh pr edit` once the PR is open (`CONTRIBUTING.md`, "AI-assisted contributions").
- **For #649:** `main.rs:262` calls `Wiring::connect()` before every verb. If the real `connect` dials eagerly, then while no hub runs, every verb that is still a stub answers `unavailable` instead of `not implemented (story #N)`, and `process/stub.rs` breaks. A lazy connect avoids that.
- **For #634:** `docs/protocol/v2.md:710` says §10 "reproduces [ADR 0003's] table verbatim". It had already drifted, and this change widens the gap. Resync §10 or drop "verbatim".
- **For #644:**
  - one PROVIDER/ID parser and one `SpecValues`-to-`ProfileSpec` merge, not one per verb file;
  - `parse_role`'s message and `--role`'s doc list `agent or orchestrator` by hand;
  - `--command-arg`/`--check-arg` take no hyphen-leading value; `--command-arg=--port` works.
- **For #646:** `holler say --help` renders the tail as `[SESSION] [TEXT]...`, and the `...` reads as a repeatable TEXT. This is clap's rendering of `num_args`. A `help` line on `rest` could say that TEXT is one argument (quote it).
- **For #648:** until #648 reads `FormatChoice.json_explicit`, `roster --format=json` prints the legacy `--json` document.
- **Optional cleanups** (A-dup pass-2 W-2, pass-1 W-5):
  - "prepend `holler`, then `try_parse_from`" is written three ways;
  - `assert_spec_flags_accepted` repeats `assert_no_failures`;
  - `usage.rs::spec_only_with_a_profile_parses` repeats a `flags.rs` check;
  - the one-envelope check exists in three places;
  - exit codes 1 and 2 are spelled out in `output.rs`, `prompt_target.rs` and `main.rs`;
  - `flatten` repeats `one_line`;
  - the stdio `Sink` is built three times in `main.rs`;
  - `interrupt | io/alpha` is in the fixture twice (lines 55 and 178).
- **Small gaps:** `unwired_ports_answer_not_implemented` (`pane_verbs/main.rs:41-52`) checks 3 of the 7 ports, although its doc says every port. `Unwired::run_probe` answers `ProbeResult::Error("not implemented")` with no story number. `cli.rs` is at 780 lines.
