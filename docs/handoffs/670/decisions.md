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
