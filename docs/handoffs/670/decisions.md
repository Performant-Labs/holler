# Decisions — #670 the pane/profile CLI skeleton (skeleton slice c of epic #633)

## A (Phase 3, up-front plan review) — 2026-10-09T06:19:31-06:00
- **Decided:** PASS on docs/handoffs/670-brief.md (Revision 1) at a41ca0a, with 13 warns and no blocks (see handoff-A.md). The plan reuses `cli::Usage` and the `Query::resolve` tail pattern, and takes every code, guard and port type from `holler-pane`, so there is no second usage-error type, validator or grammar. The `output::Envelope` is a justified new type: `PaneReply`'s own doc says it is not the CLI envelope. Exits stay in `main.rs`, and no hub, proto or golden file changes. Size is not a reason to split (W-13): about 63 paths, but 40 are template stubs and per-verb tests, and slice a (#637, 39 files) merged through this pipeline.
- **Decided (for T/F, warns that shape frozen contracts):**
  - W-1: `emit` takes the exit code from the error's code (`usage` gives 2), so decision 9's "2 usage" holds for run-time `PaneError::Usage` from `PaneName`/`ProfileName`/`Actor`/`Argv::from_json`.
  - W-2: the JSON usage envelope flattens clap's multi-line error into one line.
  - W-3: `spec_only_requires_profile` uses `pane launch --spec-only`, not `pane launch X --spec-only`, which fails on `X`.
  - W-4: subprocess tests never assert an empty stderr; the `logging_started` banner is always there.
  - W-5: `wiring.rs` gets an owning, fallible API, and one not-implemented port set that `verb_harness` reuses.
- **Assumed:**
  - The brief's inline "Reuse map" paragraph is the Reuse map of record; this run has no survey.md.
  - `hj-*` pane names are the live fleet's names. I inferred this from the epic ("are not renamed") and did not check it against the fleet.
  - T and F read handoff-A and settle W-1 to W-5 within decision 7's "provisional until compiled" allowance. In the #508 run, T and F acted on A's warns.
- **Hedged:**
  - W-1 is a warn, not a block, because decision 9 already says "2 usage" and decision 7 lets F adjust. If F keeps "0 ok, 1 error", all 13 verb stories inherit an exit-1 `usage` envelope.
  - W-10 (`src/**/mod.rs` against the workspace's 12 of 12 `name.rs` + `name/` modules) was accepted by the operator, so I flagged it with evidence only.
  - W-13: if F's scope cap trips anyway, the more separable seam is the legacy-verb `--pane`/`--profile` reshaping, not the issue's c1/c2. c1's pane/profile-only usage envelope cannot be tested end to end without c2's subcommands.
- **Evidence:**
  - **Issues:** the brief; issue #670; epic #633 (contract, Skeleton split, hot spots, rules); #638 (the envelope helper parses with `serde_json` alone); #665 (still PROPOSED).
  - **holler-cli source:** `main.rs`, `cli.rs`, `lib.rs`, `Cargo.toml`, `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`, `roster_cmd.rs`, `query_cmd.rs`, `hub_cmd.rs`.
  - **holler-cli tests:** `cli_surface_test.rs`, `docs_cli_test.rs`, `fixture/mod.rs`, both `cli-surface*.txt`, and a scan of `cli_invocation_test.rs`.
  - **Docs:** ADR-0003.md, v2.md §10, `docs/testing.md`.
  - **holler-pane:** `lib.rs`, `error.rs`, `ports.rs`, `argv.rs`, `grid.rs`, `profile.rs`, `pane.rs`, `reply.rs`, and `holler-pane-testkit`.
  - **Build and logging:** `scripts/lint.sh`, `clippy.toml`, holler-proto `log.rs` `emit_banner`.
  - **Clap probe:** clap 4.6.6 in a scratch crate outside the repo (`cargo run --offline`). It covered the `rest`/`num_args` shapes for say/interrupt/answer, `pane launch [X] --spec-only`, the `ignore_errors` lenient re-parse, and the full text of multi-line usage errors.
  - **Checks:** `find` for `src/**/mod.rs` (none) and module directories (12, all `name.rs` + `name/`); grep for outside builders of `Say`/`Interrupt`/`Answer` (none); `git show --stat f2602ba` (39 files, 4,567 insertions); `main()` code-line count, about 99.

## T-red (Phase 4, author tests, RED) — 2026-10-09T06:36:00-06:00
- **Decided:** RED is valid (handoff-T-red.md). Four test targets (`pane_cli_process` on the real binary, `pane_verbs` and `profile_verbs` in-process, two placeholders) plus `tests/verb_harness/mod.rs`, all declared in `holler-cli/Cargo.toml`. `pane_cli_process`: 28 of 33 fail on assertions about missing behaviour. The in-process targets fail to build only on the API the brief commits F to create. Process-level tests got their own target so a valid assertion-level RED exists even before the in-process API compiles. T pinned: `VerbCtx` in `output.rs` holding `Ports` by value (A W-7), `Wiring::connect()` plus `Wiring::ports()` (W-5), `emit` exit 2 for a `usage` code (W-1), a one-line flattened JSON usage message (W-2), `Say`/`Interrupt`/`Answer::resolve() -> Result<PromptArgs, Usage>`, and `SpecFlags` as a `clap::Args` group with `validate()`.
- **Assumed:**
  - The verb-to-story table (unpark #646, get/watch #643, import #650, apply #664, rename/export/import #665) comes from the epic's wave table and the stories' own file lists.
  - A JSON usage error for `--json --format=text` is an envelope under pane/profile (AC 3 lists it among the JSON cases).
  - The `error: ` prefix is not part of the JSON message; the tests check the message contains `not implemented (story #NNN)`.
  - `--check-arg`/`--command-arg` values that begin with `-` need `allow_hyphen_values`; the tests do not use them.
- **Hedged:**
  - The pinned names are provisional (decision 7). If F changes one, T fixes the test in Phase 6 and F records it.
  - ADR 0003 is not edited by T; `docs_rows.rs` pins its rows instead (the three ADR tests stay red until F edits it).
  - A verb-specific positional (`pane get PANE`) is deliberately not asserted either way, so #643 does not have to delete a test.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_cli_process`: 5 passed, 28 failed.
  - `--test pane_verbs --test profile_verbs`: unresolved imports of `holler_cli::{output, pane, profile, prompt_target}`, missing `Command::Pane`/`Profile`, missing `resolve`.
  - `--test cli_surface_test`: 2 failed, the 20 new leaves unknown to clap.
  - Throwaway prototype in the scratchpad (not in the repo): `pane_verbs` 53/53, `profile_verbs` 9/9, `cli_surface_test` 3/3, `pane_cli_process` 30/33.
  - `rustfmt --check --edition 2021` clean on every new `.rs` file; no existing file touched.

## F (Phase 6, implement) — 2026-10-09T07:20:18-06:00
- **Decided:** #670 is built against T's RED and is GREEN (handoff-F.md). `pane_cli_process` 33/33, `pane_verbs` 53/53, `profile_verbs` 9/9, `cli_surface_test` and `docs_cli_test` pass; `cargo test --workspace` is green (1023 passed, 0 failed). No test, fixture, manifest or lockfile was edited, and no name or signature that T pinned changed.
- **Decided (A's warns, settled as handoff-A.md asked):**
  - W-1: `emit` takes the exit code from the error's code (`usage` gives 2, any other error 1, ok 0), in both formats.
  - W-2: the JSON usage message is clap's reason on one line (no `error: `, no `Usage:` block, no closing hint); a bare namespace gets its own message because clap answers it with the whole help.
  - W-5: `Wiring::connect() -> Result<Wiring, PaneError>` and `Wiring::ports(&self) -> Ports<'_>`; `Unwired` is the one not-implemented port set (all seven traits), `pub`, and kept by #649.
  - W-7: `VerbCtx` is in `output.rs` and holds `Ports` by value.
  - W-8: a failed parse is read by `output::scan_args` over the raw argv, with the value-taking global flags taken from the clap tree (`Cli::global_value_flags()`), so no second copy of the global-flag grammar; `resolve_format(json, format)` takes the flag values, not `&Cli`, so `output.rs` does not import `cli.rs`. This changes the brief's `resolve_format(&Cli)`.
  - W-9: `main.rs` dispatch is one exhaustive `match` with no catch-all; the hub and body chains moved verbatim into `run_hub`/`run_body`; `print_leaf_result_and_exit` became `print_leaf_result` returning the code. The `roster --profile` refusal stays in `main.rs` (`run_roster`), so `roster_cmd.rs` is untouched for #648.
  - W-10: `pane/mod.rs` and `profile/mod.rs` kept (the operator accepted it; each module root sits inside its owner's blast-radius glob).
  - W-11: the #665 verbs are marked proposed in their clap docs (so in `--help`), in ADR 0003 and in the fixture. `docs/protocol/v2.md` §10 ("reproduces the ADR 0003 table verbatim") is not edited: #634 owns v2.md and should resync §10 or drop the word.
  - W-3, W-4, W-6, W-12 were T's and are followed. W-13: no split was needed.
- **Decided (F's own choices):**
  - `--format=json` is `--json` on every verb (decision 9): `dispatch` passes the legacy verbs `json = (format == Json)`; `FormatChoice.json_explicit` is for #648 and unused until then.
  - `Format` derives clap's `ValueEnum` and `Cli.format` is `Option<Format>`, so a bad `--format` is a clap error (exit 2).
  - `emit` also puts the JSON message on one line, never writes half an envelope (an encode failure goes to `err`, exit 1), flushes each line, ends text with a newline, writes nothing for empty text, and turns a failed write into exit 1; `emit_stream` stops at the first failed write or error item (Rust ignores SIGPIPE, so without this `pane watch | head` would never end).
  - `SpecFlags::validate()` returns a `SpecValues` (my type; the brief names none). `--ctx-soft`/`--ctx-hard` are `u32` in clap (the brief says values stay strings at clap time; for numbers the outward result is the same, exit 2 and code `usage`). `--role` is typed through `PaneRole`'s serde names (no second list). `--command-arg` with `--command-json` is refused in `validate()` too, for a caller that builds the flags by hand. The stubs do not call `validate()`, because T's flag matrix needs a bad `--grid` to reach the stub.
  - `prompt_target::route` is the one guard and the one place that knows the exit codes of the `--pane` forms (malformed tail 2, unrouted `--pane`/`--profile` 1, `not implemented (story #646)`). The three verbs call it first, before `--timeout`, `--parts-file` or any hub, and usage wins over the refusal.
  - `--command-arg`/`--check-arg` keep clap's default of no hyphen-leading values (`--command-arg=--port` works); #644 owns the choice.
  - ADR 0003: the `--pane` forms of `say`/`interrupt`/`answer` are their own rows under #646's block, because `docs_cli_test` keeps the first alternative of `a|b` and drops `[...]`, so one "SESSION or --pane" row cannot parse; `roster` gets `[--profile NAME]` in place.
  - Formatting: every new `.rs` file is `rustfmt --check` clean; in existing files no line I added or changed deviates, and `main.rs` (nearly all changed) is clean as a whole. Doc placeholders in my docs are in backticks, so I add no rustdoc warning (the 9 that exist were there).
- **Assumed:**
  - The clap 4.6.6 behaviours the design leans on: `#[command(subcommand)]` on a tuple variant nests the group with `subcommand_required` and `arg_required_else_help`; `num_args = 0..=2` without `trailing_var_arg` keeps the flags after SESSION parsing and a third positional a clap error; `Error`'s `Display` is plain text (checked with `CLICOLOR_FORCE=1`); `Arg::is_global_set` and `get_action().takes_values()` are public. All checked by compiling, by T's process tests and by hand, not from the clap docs.
  - `std::process::exit` flushes stdout (the roster table is printed with `print!`); it is the same path as before.
  - The operator wants `roster --format=json` to print the legacy `--json` document until #648 (decision 9), and not the envelope.
- **Hedged:**
  - `SpecValues` and `validate()` freeze with `pane/args.rs`. `model` and `effort` stay raw strings (splitting PROVIDER/ID is #644's call, in its own file); if #644 needs another shape it goes through the amend rule.
  - `emit`'s write-failure and encode-failure paths, `resolve_format`, `scan_args`, `route` and `Unwired` are exercised only through the binary or not at all: F writes no tests, so T may want unit cases in Phase 7 (listed in handoff-F.md).
  - `Unwired::run_probe` answers `ProbeResult::Error("not implemented")` without a story number, because `ProbeResult` is not a `PaneError`.
  - `cli.rs` is 780 lines (warn at 600, fail at 900): the next story that adds to it should know.
- **Evidence:**
  - `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` (no warnings); `cargo machete` (clean); `bash scripts/lint.sh` (exit 0); `bash scripts/changelog-check.sh` (ok); `bash scripts/test-hooks.sh` (all ok).
  - `cargo test --workspace --no-fail-fast`: exit 0, 98 binaries, 1023 passed, 0 failed, 5 ignored (other crates', unchanged). Run twice; the second run is after the last code edit.
  - By hand on the real binary with an empty state dir: the stub text and JSON forms, the `usage` envelope under `pane`/`profile` and plain clap output under `roster`/`say`, `--pane` usage errors against the refusal, `hub token list --json` against `--format=json` (byte-identical), closed stdout/stderr and a pipe to `head`.
  - Read: the brief, handoff-A.md, handoff-T-red.md, decisions.md, issue #670, epic #633, ADR 0003, `holler-pane` (`lib`, `error`, `ports`, `argv`, `grid`, `profile`, `pane`, `probe`), the testkit crate, `holler-cli` (`main`, `cli`, `lib`, `say_cmd`, `interrupt_cmd`, `answer_cmd`, `roster_cmd`) and T's tests and fixture.
  - Source facts F relies on: docs/handoffs/670/evidence.md.

## T (Phase 7, verify GREEN + Tier 2) - #670

- **Decided:**
  - GREEN confirmed: workspace 1023 passed / 0 failed; the three RED targets 33/53/9 pass, three repeat runs; `lint.sh`, `changelog-check.sh`, clippy `-D warnings`, `cargo machete`, `docs_cli_test`, `wire_selftest` all clean. Verdict PASS.
  - Added five in-process tests to `pane_verbs/output_api.rs` for `emit`'s write rules (newline, one-line JSON message, failed write never exit 0, stream stops on failed write, unencodable result) that F listed as uncovered. No production code touched.
- **Assumed:**
  - Leaving `resolve_format`, `scan_args`, `route`, `validate()` for role/ctx and `Unwired` without direct unit tests is proportionate for a skeleton slice: the binary tests pin them observably, and #644/#649 own the real behavior.
- **Hedged:**
  - A broken-pipe `Write` stub stands in for a closed pipe; the real `| head` case was only checked by F by hand.
- **Evidence:**
  - Mutation spot-checks (exit code 2 to 1; stub wording changed) each failed several tests, reverted by `git checkout`/restore, tree clean afterward. Full log in the scratchpad `full.log`. See handoff-T-green.md.

## A (Phase 7, anti-duplication gate) — 2026-10-09T07:41:26-06:00
- **Decided:** PASS on f2602ba...85444f1, with 5 warns and no blocks (handoff-A-dup.md). F extended what the Reuse map named and built no parallel path:
  - the `--pane` accessors use `cli::Usage` and the `Query::resolve` tail pattern;
  - every code, guard and port comes from `holler-pane`;
  - `print_leaf_result_and_exit` was changed in place.

  The new types copy no existing object. The brief justifies `output::Envelope`, `ErrorBody`, `ErrorCode`, `Format`, `Wiring` and `Unwired` in writing, and `SpecValues` is a partial type with no counterpart in holler-pane. None of them copies `PaneReply`, `RefusalCode`, `LogFormat` or `ProfileSpec`.
- **Decided (warns):**
  - W-1: shared test files pin verbs that sibling stories will implement, so the nine verb stories (and #649) must edit #670's shared test files. The files are both in-process `main.rs` files, the `pane_cli_process` verb tables, `flags.rs` and three `usage.rs` cases. Two of the seam tests are exact duplicates of per-verb tests, and the verb tables put lines of different stories next to each other.
  - W-2: `verb_harness` gets its ports from `Wiring::connect()`, which #649 replaces, instead of from `Unwired`. It also takes no ports, although the brief says it builds a `VerbCtx` "over given ports".
  - W-3: the one-envelope check is copied (`one_envelope` and `Out::envelope`, plus inline in `stub.rs`).
  - W-4: `SpecValues` is a flat, partial copy of `ProfileSpec`'s fields. #644 owns one merge and one PROVIDER/ID parser.
  - W-5: minor rules stated twice: exit codes, the envelope namespaces, `flatten`/`one_line`, the stdio `Sink`, and a duplicate fixture line.
- **Assumed:**
  - A verb story replaces its stub's output, so any test that pins `not implemented (story #N)` for that verb fails when the story lands.
  - #649's `connect` builds real adapters, as `wiring.rs`'s doc says.
  - Phase 7 blocks only a parallel path (the role's verdict rules), so W-1 and W-2 are warns.
- **Hedged:**
  - W-1 is the costliest warn. It does not break this story, but the nine verb stories hit it in parallel. I rated it warn because it is not a parallel path. If S reads the brief's "a later story edits only its own file and never `main.rs` or the manifest" as a requirement, it should be fixed before merge.
  - W-2's hazard to #649 is inferred from `wiring.rs`'s doc and `main.rs`, not run.
- **Evidence:**
  - **Diff and handoffs:** `git diff origin/main...HEAD` (79 files, all inside the blast radius); the brief, handoff-A, handoff-T-red, handoff-F, handoff-T-green and this journal.
  - **Read in full:** every new or changed `src` file in holler-cli and every new test file.
  - **Existing tests:** the `tests/support/mod.rs` API, `tests/fixture/mod.rs`, `cli_surface_test.rs` and `docs_cli_test.rs`.
  - **holler-pane:** `error.rs`, `profile.rs`, `pane.rs`, `ports.rs`, `reply.rs`, `lib.rs`, and the testkit.
  - **Greps:**
    - exit-code constants and "not implemented" wording;
    - argv scanning, `ValueEnum` and `LogFormat`;
    - PROVIDER/ID parsers (none) and message flattening;
    - users of `Wiring::connect` and `Unwired`;
    - `hj-` names in the diff (none).

## T (test-only rework after S, #678)

- **Decided:**
  - `run_verb_with(ports)` is the harness's one entry point and `run_verb` is the unwired case.
  - Flag acceptance is asked of clap in-process; a flag is accepted unless clap says `UnknownArgument` or the verb is unknown, so a sibling story adding a required positional does not break these tests.
  - The positive spec-flag matrix lives in `launch.rs`/`relaunch.rs`.
  - `STUBS` (in `stub.rs`) is the only table of stub owners. The two legacy-refusal lines sit beside it.
  - The ADR/fixture layout tests use their own permanent `STORY_GROUPS`.
- **Assumed:** the `parse.rs` helper shared by three targets through `#[path]` is acceptable (it needs only `holler_cli::Cli`, which `cli_surface_test` already uses). The `pane_cli_process` comment in `Cargo.toml` stays true.
- **Hedged:** rustfmt strips blank lines between `STUBS` groups, so the `// #NNN` comments are the separators (S allowed either). `STORY_GROUPS` repeats the story numbers the ADR/fixture checks need, so a new story must add itself there; `every_verb_has_exactly_one_owning_story` fails if it does not.
- **Evidence:** `cargo test --workspace` (1029/0), clippy `-D warnings`, `scripts/lint.sh`, grep for `not implemented (story #` and `Wiring` under `crates/holler-cli/tests`.

## A (Phase 7, anti-duplication gate, pass 2) — 2026-10-09T08:19:44-06:00
- **Decided:** PASS on f2602ba...56883d6, with 3 warns and no blocks (handoff-A-dup.md, rewritten for this pass; pass 1 is at e4365c4).
  - This cycle is T's test-only rework (56883d6). It extends the shared harness and does not build a second one beside it.
  - `run_verb_with` takes the ports. `run_verb` runs over the existing `Unwired`, so there is still one not-implemented port set.
  - No test refers to `Wiring` any more, and no frozen root or shared process file pins another story's stub.
  - `parse.rs` uses `output::resolve_format`, and `STORY_GROUPS` replaces the inline `groups` copy.
  - Pass-1 W-1 and W-2 are settled.
- **Decided (warns):**
  - W-1: two places where parallel wave-3 stories still conflict.
    - Deleting a whole `STUBS` group, `// #NNN` header included, makes the deletions of #643 and #644 adjacent.
    - `legacy_verbs.rs:10` imports the #646 and #648 refusal constants on one `use` line, which both stories must edit.
    - The fix: delete only the entries, and give each constant its own `use`.
  - W-2: small test-helper copies:
    - three spellings of "prepend `holler` + `try_parse_from`";
    - `assert_no_failures` copied into `parse.rs`;
    - `usage.rs::spec_only_with_a_profile_parses` repeats a `flags.rs` check;
    - the one-envelope check, still in three places (pass-1 W-3).
  - W-3: comments in frozen files are now wrong: `process/main.rs` says "only the binary can show" the flag matrix, and the manifest's comments on the in-process and `pane_cli_process` targets are out of date.
- **Assumed:**
  - #643 and #644 (and #646 and #648) can land in parallel, so an edit on an adjacent line is a real conflict.
  - A story reads "deletes exactly its own group" as including its `// #NNN` header.
  - Phase 7 blocks only a parallel path. These findings are layout and comment issues, so they are warns.
- **Hedged:**
  - W-1(b) depends on `unused_imports` failing clippy `-D warnings` once a story deletes its refusal test. I worked this out from the lint setup (no `rustfmt.toml`; nothing allows `unused_imports`) and did not run it.
  - I did not run the test suite. GREEN rests on T's run (1029 passed, 0 failed).
- **Evidence:**
  - `git diff e4365c4..56883d6`. It touches test files and handoffs only, and is empty for `src/`, the manifest, the fixture, ADR 0003, `Cargo.lock` and `CHANGELOG.md`.
  - Read in full:
    - `verb_harness/{mod,parse}.rs`;
    - `pane_verbs/{main,launch,relaunch}.rs`, `profile_verbs/main.rs`;
    - `process/{main,stub,flags,usage,docs_rows,legacy_verbs}.rs`;
    - `src/pane/wiring.rs`;
    - handoff-S, handoff-T-green and this journal.
  - Read in part: the head of `target_flags.rs`, `holler-pane` `Ports`, and the parse helpers in `cli_surface_test.rs` and `docs_cli_test.rs`.
  - Greps:
    - `not implemented (story #` and `Wiring` under `tests/`;
    - `try_parse_from` / `ErrorKind` helpers;
    - `Ports {` construction sites.
  - Scratch git repo (in the scratchpad, deleted afterwards): deleting two adjacent whole `// #NNN` groups conflicts; deleting only the entries merges cleanly.
  - `git show af3d8df` (#669): `pane_wiring.rs`, `pane_dispatch.rs`, `panes/mod.rs`, `profile/mod.rs`. No overlap with the CLI.
  - Blast-radius grep over `git diff --name-only f2602ba..HEAD`: 82 paths, none outside the radius.

## S (spec audit, pass 2) — 2026-10-09T08:32:19-06:00
- **Pass 1, journalled late:** pass 1 of S (4776239) returned REWORK, test-only, but wrote no entry here. The findings were the shared test scaffolding pinning sibling stubs and `Wiring::connect()`. T's rework (56883d6) resolved all three items, and A-dup pass 2 (51f3bed) confirms it.
- **Decided:** REWORK, production (handoff-S.md, pass 2).
  - **1 (F, `src`):** `prompt_target.rs` `resolve_tail` must refuse an extra positional in the SESSION form.
    - The issue says "Missing or extra positionals return the existing `cli::Usage` (exit 2)".
    - `num_args = 0..=2` bounds one occurrence. A flag between positionals starts another, so `say io/alpha hello --queue extra`, `say io/alpha --timeout 5m fix it`, `interrupt io/alpha --server URL stop now` and `answer io/alpha --server URL 1 2` parse.
    - The accessor drops everything after the second value. `origin/main` refused these with clap's `UnknownArgument` (exit 2).
    - T adds the proving tests in `target_flags.rs` and `legacy_verbs.rs`.
  - **2-3 (T, tests):** two places where parallel wave-3 stories still collide, as A-dup W-1 found:
    - the `STUBS` doc says to delete the whole group (header included);
    - `legacy_verbs.rs:10` imports the #646 and #648 refusal constants on one `use` line.
  - **4 (T):** comments that the rework made wrong (A-dup W-3), in `process/main.rs:9-13` and `Cargo.toml:477-478, 487-492`.
  - Everything else meets the issue and the brief. AC 1-3 and 5-8 are MET; AC 4 is NOT MET (item 1).
- **Assumed:**
  - `interrupt` and `answer` on `origin/main` refuse a third positional the way `say` does. They have the same fixed `session` + `text`/`choice` positionals. I probed only `say`'s shape in the scratch crate.
  - The built `target/debug/holler` (07:29 MDT) is the audited code: no `src` change since 016eeaa (07:21 MDT).
- **Hedged:**
  - Items 2-4 are small and would not have forced a REWORK on their own. They ride along because item 1 forces a loop, and the files freeze when this story merges.
  - I did not re-run Tier 1 or Tier 2. GREEN rests on T's run: 1029 passed, 0 failed.
- **Evidence:**
  - **Diff and docs:** `git diff origin/main...HEAD` (82 paths) and the body of issue #670 (`gh issue view 670`). Read in full: every new or changed `src` file and the reworked test files.
  - **Built binary:** run over an empty, isolated state dir, with no hub started. The split-tail forms reach the hub path (exit 1), while `say io/alpha hello extra` is exit 2.
  - **Clap probe:** a scratch crate on clap 4.6.6, outside the repo. The old `Say` shape gives `UnknownArgument` for `say io/alpha hello --queue extra` and `say io/alpha --timeout 5m fix it`; the new shape parses them as `rest = [io/alpha, hello, extra]` and `[io/alpha, fix, it]`.
  - **Merge probe:** a scratch git repo. Whole `// #NNN` groups deleted in parallel conflict; entries-only deletions merge cleanly.
  - **Formatting:** `rustfmt --check --edition 2021` on the new and reworked files is clean. Its only diffs are in the pre-existing `tests/support/*`, which this branch does not touch.
  - **Merge with main:** `git merge-tree` against `origin/main` (1c48c30) conflicts in `CHANGELOG.md` only.
  - **Greps:** banned calls, `#[allow`, `process::exit`, privacy, `not implemented (story #`, `Wiring`.

## F (Phase 6, rework 1 after S pass 2) — 2026-10-09T08:41:02-06:00
- **Decided:** S pass 2 item 1 is fixed (handoff-F.md, "Rework 1"). `resolve_tail`'s SESSION arm in `prompt_target.rs` returns the existing `Usage` (exit 2) when the tail holds more than two positionals: `only SESSION and TEXT may be given, got 3 positionals: <forms>` (`CHOICE` for `answer`). It is one file, +15 / -2 lines, and no signature changed. This addresses AC 4, the one S marked NOT MET; S re-audits it once T's proving tests are in. `archChanged` is false: one private function's error condition changed, and no module boundary, public interface or dependency direction.
  - **The count stays in code.** Clap's `num_args` bounds the values of one occurrence of the tail, and a flag between positionals starts another, so `say io/alpha hello --queue extra`, `say io/alpha --timeout 5m fix it`, `interrupt io/alpha --server URL stop now` and `answer io/alpha --server URL 1 2` parsed with a third value that the accessor dropped. The alternatives are the shapes the brief ruled out: fixed positionals cannot parse `say --pane NAME TEXT`, and `trailing_var_arg` swallows `--parts-file`, `--queue`, `--grant` and `--server`.
  - **The `--pane` arm is unchanged** (it already checks `rest.len() > 1`). The doc comment of `resolve_tail` no longer says that clap refused a third positional.
  - **The message gives the count and the forms, not the surplus values**, as the `--pane` arm does. The surplus words are the user's prompt text, and a usage error should not copy them into a log.
  - **The order is unchanged:** a usage error comes before the #646 refusal, `--timeout`, `--parts-file` and any hub, because `route()` is the first statement of each verb's `run`.
  - **Items 2-4 of S's verdict were left alone.** They are test files (the `STUBS` doc, the `use` line in `legacy_verbs.rs`, the comment in `process/main.rs`) and manifest comments, which S assigned to T. The proving tests for item 1 are named in handoff-F.md for T to author in T-green.
- **Corrects an earlier entry:** the F entry above (Assumed, the clap behaviours) says `num_args = 0..=2` "keeps ... a third positional a clap error". That held for the contiguous form only. The split form is what this rework closes.
- **Assumed:**
  - `interrupt` and `answer` on `origin/main` refused a third positional the way `say` did. S assumed it. I checked it by reading `origin/main`'s `cli.rs`: both declare fixed `session` + `text`/`choice` positionals, so a third is clap's `UnknownArgument`. I did not build and run `origin/main`.
  - T adds the proving tests in T-green and shows them failing against the pass-1 `resolve_tail` (51f3bed), as S asked.
  - The wording of the message is not pinned by anything in S's list. I named `positionals` as the substring T can assert on.
- **Hedged:**
  - F writes no tests, so the proof of the fix is the built binary, before and after, not a test. Before (the 07:29 MDT build of the pass-1 sources), the four split forms exited 1 on the hub or the remote path. After, they exit 2 with the message, an empty stdout and no hub line.
  - `rest.len() > 2` counts the values clap gives it. A later story that gives these verbs another positional must change this bound together with `num_args`.
  - The quota is not mine to size. `/tmp` is a tmpfs with `usrquota`, the session scratchpad already holds about 8 GB, and a heavy build beside `cargo test --workspace` pushed the hub tests into `Disk quota exceeded`. I inferred the per-user quota from the mount options and errno 122; I do not know its limit. T's next workspace run should start with nothing else running.
  - `holler say --help` still renders the tail as `[SESSION] [TEXT]...` (clap's rendering of `num_args`; S's note for #646), so the help does not show the two-positional limit that this rework enforces in code.
- **Evidence:**
  - **Before and after, on the built binary** with an isolated empty state dir and no hub: the four forms above, the contiguous `say io/alpha hello extra` (clap, exit 2, unchanged), the valid split form `say io/alpha --queue hello` (still reaches the hub path), the `--pane` arm (unchanged), the valid `interrupt`/`answer` forms with `--server`, and the order probes (`--profile demo`, `--timeout bogus`, `--parts-file /nonexistent.json` each lose to the usage error).
  - **By value, in process:** a scratch test kept outside the repo (compiled with `rustc --test` against the built `holler_cli` and `clap` rlibs) shows that clap parses the four split forms, that the accessor refuses each with the `positionals` message, that `say io/alpha --queue hello`, `interrupt io/alpha --server URL stop` and `answer io/alpha --server URL 1` resolve to both values, and that the `--pane` arm is unchanged (3 of 3 pass). It is not a repo test and is not committed.
  - **Tier 1, after the last edit:**
    - `cargo test --workspace --no-fail-fast`, with nothing else running: exit 0, 98 binaries, 1029 passed, 0 failed, 5 ignored. These are T-green's totals; no test was added.
    - The targets that touch these verbs (`pane_verbs` 59, `profile_verbs` 8, `pane_cli_process` 34, `talk_test` 18, `interrupt_test` 11, `answer_cli_test` 11, `cli_invocation_test` 14, `cli_surface_test` 3, `docs_cli_test` 3) pass.
    - `cargo build -p holler-cli`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo machete`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `scripts/test-hooks.sh` and `rustfmt --check --edition 2021 prompt_target.rs` are clean.
    - Two earlier full runs: one green before a comment-only edit, and one with 14 failures that I caused by compiling a scratch probe into the quota-limited `/tmp` while it ran (12 print `Disk quota exceeded`, one is a knock-on `NotFound`, one is a `connection_lost` race). Its three targets (`hold_hub_test`, `hub_hygiene_test`, `interrupt_test`) pass alone (16, 14, 11), and so did the clean run above. Logs: the scratchpad's `workspace-test*.log`.
  - **Read:** handoff-S.md (pass 2) and the other handoffs of this run, the brief, issue #670, `prompt_target.rs`, `cli.rs`, the three `*_cmd.rs`, `target_flags.rs`, `legacy_verbs.rs`, `origin/main`'s `cli.rs`.
  - Source facts F relies on: `docs/handoffs/670/evidence.md`, section "Rework 1".

## T (phase 7, rework 2): S pass 2 items 1-4

- **Decided:**
  - **Item 1 proof at the cheapest tier, plus one binary row.** Two in-process tests in `target_flags.rs` (refusal of the four split forms, asserting clap accepts them first; the valid split form as a guard), and two rows in the existing malformed-form table of `legacy_verbs.rs`, not a new process test (cheaper, no duplication). The second row (`--profile demo ... --queue extra`) pins usage-before-refusal.
  - **Assert on `positionals`** in the message, the substring F named, so the test tells the accessor's `Usage` from a clap parse error without pinning the full wording.
  - **Items 2-4 done as S worded them:** the doc/comment rewrites for `STUBS`, the two refusal constants, `process/main.rs` and the `Cargo.toml` comments; one `use` line per refusal constant.
- **Assumed:** adjacent `use` lines are acceptable for item 3: each story edits only its own line, and git conflicts only on overlapping or adjacent *changed* hunks of the same line pair; S asked for "its own `use` line", not a separating unchanged line, and rustfmt would not keep a blank line. Hedge: if S meant a separating line, the `use` lines can be separated by a comment line.
- **Hedged:**
  - The `interrupt_test` warm-up failures in workspace run 1 are a load flake I did not diagnose; evidence is run 1 vs run 2 and the three solo passes.
  - The Cargo.toml reason "a change to the in-process harness cannot break it" is true of `verb_harness/mod.rs`; `pane_cli_process` still compiles `verb_harness/parse.rs`, which the in-process targets share.
- **Evidence:** RED against `git show 51f3bed:crates/holler-cli/src/prompt_target.rs` (1 failing in-process test, 1 failing process test with the two new rows), GREEN on 02a3655: 1031 passed, 0 failed, 5 ignored; clippy, lint, changelog-check, machete clean. Details in `handoff-T-green.md`, "Rework 2".

## A (Phase 7, anti-duplication gate, pass 3) — 2026-10-09T09:06:17-06:00
- **Decided:** PASS on f2602ba...4876a4c, with 1 warn and no blocks. handoff-A-dup.md is rewritten for this pass; pass 2 is that file at 51f3bed. This cycle is 51f3bed..4876a4c.
  - **F extended what the map named.** `resolve_tail` in `prompt_target.rs` refuses a third positional in the SESSION form with the existing `cli::Usage`. The check was added in place, beside the `--pane` arm's check, and reuses `Tail::forms()`. There is no new function, type or error type, and no signature changed.
  - **T's proving tests reuse what was there.** The in-process tests use `target_flags.rs`'s own `parse`/`resolve`/`expect`/`session`. The binary cases are two rows in the existing table of `legacy_verbs.rs`.
  - **Settled from pass 2:** W-1(a) (keep the `// #NNN` line in `STUBS`) and W-3 (out-of-date comments).
- **Decided (warn):**
  - **W-1: S pass 2's item 3 (pass-2 W-1(b)) is not settled.** The two refusal imports at `legacy_verbs.rs:10-11` are separate lines, but adjacent ones. #646 and #648 must each delete theirs, and git reports deletions on adjacent lines as a conflict.
  - **The fix:** a `// #646` line above the first `use` and a `// #648` line above the second, the convention `stub.rs` uses. Each story keeps its comment line. Owner: T.
  - **This corrects the "Assumed" of T's rework-2 entry above.** That entry says adjacent `use` lines do not conflict and rustfmt would not keep a separating line. Both are wrong (Evidence).
- **Assumed:**
  - #646 and #648 can land in parallel. Each deletes its refusal constant (the `stub.rs` doc says so for #646, and "until story #648" for #648) or its last use, so each must delete its own `use` line.
  - Phase 7 blocks only a parallel path. This finding is a layout issue in a test file, so it is a warn, as in pass 2.
- **Hedged:**
  - I did not run the test suite. GREEN rests on T's rework-2 run: 1031 passed, 0 failed.
  - The two-positional limit is stated twice, in clap's `num_args` and in `resolve_tail`. I did not flag it. Both are needed (clap refuses the contiguous form, the accessor the split form), F recorded the coupling, and changing `num_args` alone still refuses the extra positional.
- **Evidence:**
  - **Diff:** `git diff 51f3bed..4876a4c`. The only `src/` file is `prompt_target.rs`; the rest is four test files, two manifest comments and handoffs.
  - **Read in full:** `prompt_target.rs`, `target_flags.rs`, `process/{legacy_verbs,main,stub}.rs`, the manifest's test-target comments, `cli.rs:280-600` (`Query::resolve`, `Usage`, `Say`, `Interrupt`, `Answer`), the head of `verb_harness/parse.rs`, and handoff-S, handoff-F and the T-green "Rework 2" section.
  - **Scratch git repo** (in the session scratchpad, deleted afterwards), on copies of this branch's files:
    - deleting `legacy_verbs.rs` line 10 on one side and line 11 on the other gives `CONFLICT (content)`;
    - the same deletions with a blank line, or a `// #NNN` line above each `use`, merge cleanly, and both layouts pass `rustfmt --check --edition 2021`. The `// #NNN` layout also passes inside the repo, which has no rustfmt config;
    - `STUBS` entries-only deletions (#643 against #644) merge cleanly;
    - the two refusal constants deleted as the new `stub.rs` doc says merge cleanly.
  - **Greps:**
    - `no live holler hub` (11 existing CLI tests match the literal; there is no shared constant);
    - `positionals` and `num_args` in `src`;
    - the blast radius over `git diff --name-only f2602ba...HEAD` (82 paths, all inside).
  - **Merge with main:** `git merge-tree` against `origin/main` (1c48c30) conflicts in `CHANGELOG.md` only, and `origin/main` changes nothing in `holler-cli`.

## S (spec audit, pass 3) — 2026-10-09T09:16:08-06:00
- **Decided:** PASS. handoff-S.md is rewritten for this pass; pass 2 is that file at 7939b54.
  - **AC 1-8 are MET.** The brief's decisions are implemented, and the deviations recorded above are accepted.
  - **Pass-2 item 1 (F, `src`) is resolved.**
    - `resolve_tail` refuses a third positional in the SESSION form with the existing `Usage` (exit 2), and the doc comments now hold.
    - T's two in-process tests and two binary rows fail against 51f3bed and pass on 02a3655.
  - **Pass-2 items 2 and 4 (T) are resolved:** the `STUBS` doc and the refusal-constant layout, and the comments in `process/main.rs` and the manifest.
  - **Pass-2 item 3 (T) is not resolved as asked.** The two refusal imports at `process/legacy_verbs.rs:10-11` are separate lines, but adjacent ones; pass 2 asked for "an unchanged line between them". It is not carried as a REWORK:
    - no acceptance criterion or decision of the brief requires it (AC 7's separator rule covers the ADR rows and fixture lines, which are correct);
    - pass 2 said items 2-4 would not force a REWORK on their own;
    - the cost is one trivial rebase conflict for whichever of #646 and #648 lands second.

    The fix, a `// #646` line and a `// #648` line above the two `use` lines, is advisory 1 of handoff-S.
- **Corrects an earlier entry.** The "Assumed" of T's rework-2 entry above is wrong on both counts:
  - Adjacent `use` lines do conflict. In a scratch repo, #646 deleting line 10 and #648 deleting line 11 in parallel gives `CONFLICT (content)`.
  - rustfmt does keep a separating line. With a `// #NNN` line above each `use`, the file passes `rustfmt --check --edition 2021`, and the same deletions merge cleanly.
  - A-dup pass 3 found the same.
- **Assumed:**
  - The built `target/debug/holler` (08:51 MDT) is the audited code: `git diff 02a3655 HEAD -- crates/holler-cli/src` is empty, and the last `src` commit is from 08:50 MDT.
  - The `interrupt_test` failures in T's first workspace run are the known flake of open issue #420 (`reconnecting` in the warm-up), not this change: `interrupt_cmd.rs` runs after the warm-up.
- **Hedged:**
  - I did not re-run Tier 1 or Tier 2. GREEN rests on T's rework-2 run: 1031 passed, 0 failed.
  - A third consecutive S REWORK would reach the script's `PER_GATE_BLOCK_THRESHOLD` (3) and stop the run. I knew this. The verdict rests on the item's weight, as above, not on the threshold.
  - Advisory 1 is unowned unless O folds it into this PR or #646 picks it up.
  - Since #634 merged (094ebfa), the `docs/protocol/v2.md` §10 drift (advisory 5) has no owning story.
- **Evidence:**
  - **Diff and documents:** `git diff origin/main...HEAD` (82 paths, all inside the blast radius). Also issue #670, epic #633 ("Decisions taken", item 5), issue #676, and ADR-0021 and v2.md on `origin/main`.
  - **Read in full:** every new or changed `src` file and the test files listed in handoff-S.
  - **Built binary:** run over an empty, isolated state dir, with no hub started.
    - The split forms of `say`, `interrupt` and `answer` exit 2 with the `positionals` message, an empty stdout and no hub line.
    - `say io/alpha --queue hello` still reaches the hub path.
    - Also checked: the stub text and JSON forms, `usage` envelopes under `pane`/`profile` only, `--debug bogus` (exit 3), and bare and unknown subcommands.
  - **Differential probe:** a scratch crate on clap 4.6.6, outside the repo. It compares `origin/main`'s `Say`/`Interrupt`/`Answer` with this branch's shapes plus a copy of `resolve_tail`, over 50 argv forms. Result: 0 differences. Every accepted form resolves to the same values, and every refused form is still refused.
  - **Formatting:**
    - `rustfmt --check --edition 2021` on all 62 new `.rs` files: the only diffs are in the pre-existing `tests/support/*`.
    - In the 6 edited `src` files, no line that rustfmt would change is a line this branch added.
  - **Greps:** banned calls, `#[allow`, `process::exit`, privacy (including the committed handoffs), and `Envelope` outside `output.rs`.
  - **Merge with main:** `git merge-tree` against `origin/main` (094ebfa) conflicts in `CHANGELOG.md` only.
