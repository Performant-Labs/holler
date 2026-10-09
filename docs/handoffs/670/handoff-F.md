# Handoff-F: Phase 6 - #670 the pane/profile CLI skeleton (skeleton slice c)

**Date:** 2026-10-09
**Branch:** issue-670-implementation (at 0afbb93, F's work uncommitted)
**Issue:** #670

The run is non-interactive, so the confirmation table of the role doc is recorded here instead of waited on:

| Field | Value |
|-------|-------|
| GitHub issue | #670 |
| Working branch | issue-670-implementation (worktree `.claude/worktrees/0670-cli-skeleton`) |
| Build plan phase | pipeline Phase 6 (F, implement against the RED); there is no BUILD_PLAN, the issue is the source of truth |
| Input documents read | docs/handoffs/670-brief.md (Revision 1), docs/handoffs/670/handoff-A.md, handoff-T-red.md, decisions.md, issue #670, epic #633, ADR 0003, the holler-pane crate, the existing `holler-cli` sources and T's tests |
| Acceptance criteria count | 8 (the brief's numbering) |
| Handoff document path | docs/handoffs/670/handoff-F.md |

Nothing was missing. No scope split was proposed: the slice is large in files (27 new `src` files), but 20 of them are one template stub each, which the scope cap exempts as mechanical work.

## What was done

T's three test targets, which were RED, are GREEN with no test edited: `pane_cli_process` 33/33, `pane_verbs` 53/53 and `profile_verbs` 9/9 (the two placeholder targets have no tests). So are `cli_surface_test` 3/3 and `docs_cli_test` 3/3, which the fixture and the ADR rows turn on. The rest of the workspace is unchanged and green.

New files, all in `crates/holler-cli/src/`:
- `output.rs`: `Format` (a clap `ValueEnum`), `FormatChoice`, `resolve_format`, `ErrorCode`, `ErrorBody`, `Envelope<T>`, `Sink`, `VerbCtx`, `emit`, `emit_stream`, `emit_error`, `emit_usage_error`, `not_implemented(_message)`, and the failed-parse helpers `scan_args`, `RawArgs` and `usage_message`.
- `prompt_target.rs`: `PromptTarget`, `PromptArgs`, `Say`/`Interrupt`/`Answer::resolve()`, and `route()` (the `--pane`/`--profile` guard).
- `pane/mod.rs` (`PaneCmd`, `run`), `pane/args.rs` (`SpecFlags` + `validate()` -> `SpecValues`, `ProfileOpt`, `SpecOnly`), `pane/wiring.rs` (`Wiring`, `Unwired`), `pane/profile_scope.rs` (empty), and one stub per verb: `list get watch launch relaunch switch reset park unpark close doctor import`.
- `profile/mod.rs` (`ProfileCmd`, `run`) and one stub per verb: `create delete list show apply rename export import` (`--take-over` is in `apply.rs`).

Edited files:
- `cli.rs`: `Command::Pane` and `Command::Profile`; the global `--format`; `Say`/`Interrupt`/`Answer` take one optional tail (`rest`) plus `--pane` and `--profile`; `Roster` takes `--profile`; `Cli::global_value_flags()`; `Usage::new` is `pub(crate)`; the namespace comment.
- `lib.rs`: the four module declarations.
- `main.rs`: `Cli::try_parse()`, the format resolver, one exhaustive `match` (`dispatch`) with the hub and body chains moved verbatim into `run_hub`/`run_body`, the two new arms, the `roster --profile` refusal.
- `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`: read `route(x.resolve(), &x.profile)` instead of the old fields; nothing else changed.
- `docs/adr/ADR-0003.md`: decision 6 of the brief (rows, global flag, amended sentences). `CHANGELOG.md`: the `## [Unreleased]` entry.

## Design decisions

Each is also in decisions.md. "A W-n" is a warn of handoff-A.md.

1. **Exhaustive dispatch in `main.rs` (A W-9).** One `match` over `Command` with no catch-all replaces the `if let` chain whose last lines were the fall-through to exit 0. The hub and body chains moved into `run_hub`/`run_body` unchanged, `print_leaf_result_and_exit` became `print_leaf_result` returning the code, and `main` exits once. `main()` is now a handful of lines (clippy's 100-line limit is no longer near). The `roster --profile` refusal is a small `run_roster` in `main.rs`, so `roster_cmd.rs` stays untouched for #648 (the brief's decision 1(b)).
2. **A failed parse is read from the raw argv (A W-8).** `Cli::try_parse()` fails before any flag is parsed, so `output::scan_args` finds the first non-flag token and whether JSON was asked for. The global flags that take a value (`--debug`, `--log-format`, `--format`) come from the clap tree (`Cli::global_value_flags()`), so a new one cannot be missed, and the scan lives in `output.rs` without importing `cli.rs`. `resolve_format(json, format)` takes the two flag values, not `&Cli`, for the same reason (the brief had `resolve_format(&Cli)`).
3. **`Format` is the clap value (`Cli.format: Option<Format>`).** A bad `--format` is a clap error, exit 2, naming the value. `Option` keeps an explicit `--format=json` apart from `--json` (`FormatChoice.json_explicit`, for #648).
4. **`--format=json` is `--json` on every verb (decision 9).** `dispatch` passes the legacy verbs `json = (format == Json)`, so `roster --format=json` prints the legacy JSON until #648 decides what it prints.
5. **`emit` takes the exit code from the error's code (A W-1):** `usage` is 2, every other error 1, success 0, in both formats. A `usage` error from a run-time guard (`PaneName::parse`, `Argv::from_json` on text that is not JSON) therefore exits 2.
6. **`emit` guarantees what the contract promises.** In JSON mode the message is put on one line; a result that cannot be encoded is reported on `err` and exits 1 (never half an envelope on `out`); text ends with a newline (added when it is missing) and empty text writes nothing; every line is flushed. A write that fails turns exit 0 into 1, and `emit_stream` stops at the first failed write or error item. Without that last rule `pane watch | head` would loop for ever, because Rust ignores SIGPIPE.
7. **The JSON usage message is flattened (A W-2)** to clap's reason on one line: no `error: ` prefix, no `Usage:` block, no closing hint. A bare namespace (`holler pane --format=json`) gets its own message, because clap answers it with the whole help text.
8. **`VerbCtx` is in `output.rs` and holds `Ports` by value (A W-7).**
9. **`Wiring::connect() -> Result<Wiring, PaneError>` and `Wiring::ports(&self) -> Ports<'_>` (A W-5).** `Unwired` is the one not-implemented port set (all seven traits), `pub` and kept by #649. `main` reports a `connect` error through `emit_error` (exit 1, with the error's code); `pane::run` and `profile::run` never see it.
10. **Numbers and the role are typed at the edge they are cheapest at.** `--ctx-soft`/`--ctx-hard` are `Option<u32>` in clap (a non-number is a clap usage error, exit 2, which is also what a `validate()` refusal would give). `--grid`, `--env` and the JSON flags stay strings and are typed by `validate()` with the guards of `holler-pane`. `--role` is typed through `PaneRole`'s serde names, so there is no second list of roles. `validate()` returns a `SpecValues` (the brief did not name the type) and also refuses `--command-arg` together with `--command-json` (clap already does; this is for a caller that builds `SpecFlags` by hand).
11. **The stubs do not call `validate()`.** T's flag matrix needs `pane launch --grid banana` to reach the stub. #644 calls it.
12. **`prompt_target::route` is the one guard and the one place that knows the exit codes** of the `--pane` forms: a malformed tail is 2, a `--pane` or `--profile` that is not routed yet is 1 with `not implemented (story #646)`. The three verbs call it first, before the timeout, `--parts-file` or any hub. Usage wins over the refusal, as T's notes require.
13. **`--command-arg` and `--check-arg` take no hyphen-leading values** (clap's default), so `--command-arg=--port` is the way to pass `--port`. Allowing hyphen values would let `--command-arg --expect ok` swallow the next flag. T left this to #644, which can change the one attribute.
14. **ADR 0003 gets separate `--pane` rows for `say`, `interrupt` and `answer`** (under #646's block) instead of editing their SESSION rows, because `docs_cli_test` keeps the first alternative of `a|b` and drops `[...]`, so a combined "SESSION or --pane" row cannot parse. `roster` gets `[--profile NAME]` in place. The rows of #665 say "proposed" (A W-11); `docs/protocol/v2.md` §10 is not touched (see Known issues).
15. **`src/**/mod.rs` kept** (A W-10): the operator accepted it so each module root sits inside its owner's blast-radius glob.

## Reuse / extend-vs-new

- **Extended:** the clap tree and the global-flag pattern in `cli.rs`; `Query::resolve`'s variadic-tail pattern (without `trailing_var_arg`) and the root-exported `cli::Usage` for the `--pane` accessors, so there is no second usage-error type (`Usage::new` became `pub(crate)`); `say_cmd.rs`/`interrupt_cmd.rs`/`answer_cmd.rs` and their result types; `print_leaf_result_and_exit` (now `print_leaf_result`) and the stub wording `error: not implemented (story ...)`; ADR 0003 and the CLI fixture.
- **From `holler-pane`, not copied:** `is_valid_code` (`ErrorCode::new`), `PaneError` and its codes (`ErrorCode::from`, the stubs, `usage`), `GridPos::parse`, `EnvVarName::parse`, `Argv::from_json`, `Ports` and the seven port traits, `PaneRole` (through its serde names).
- **New, as the brief justifies:** `output::Envelope` (`PaneReply` is the hub's control-socket reply, not the CLI's output; its own doc says so), `Sink`/`VerbCtx`, `Format`, the failed-parse scan, `PromptTarget`/`PromptArgs`/`route`, `SpecValues`, `Wiring`/`Unwired`. `output.rs` is a deliberate fourth output path, for the envelope only: ADR 0003 forbids changing a legacy verb's `--json` shape.

## Architecture notes for A

- **Dependency direction, no cycle:** `cli.rs` imports `output` (the `Format` type), `pane` (`PaneCmd`, `args::ProfileOpt`) and `profile` (`ProfileCmd`); `pane/*` and `profile/*` import `output` and `holler-pane`, never `cli.rs`; `output.rs` imports neither; `prompt_target.rs` imports `cli`, `output` and `pane::args`; the three `*_cmd.rs` import `prompt_target`; `main.rs` imports all. `holler-cli` gains a runtime dependency on `holler-pane` and a dev-dependency on `holler-pane-testkit` (T's manifest edit); `cargo machete` is clean.
- **Public interface changes:** `Say`, `Interrupt` and `Answer` lose `session`/`text`/`choice` for `rest` (an optional tail) and gain `pane` and `profile`; `Roster` gains `profile`; `Command` gains two variants; `Cli` gains `format`; `Usage::new` is `pub(crate)`. Nothing outside the three `*_cmd.rs` read the old fields (A checked with grep).
- **Behaviour changes for existing verbs, all intended:** `Cli::try_parse()` plus `err.exit()` is `Cli::parse()`; the usage-error output of every legacy verb is unchanged, including with `--json`; `--format` is new on all of them; `say`/`interrupt`/`answer` with a missing SESSION/TEXT/CHOICE is now a `Usage` error from the accessor (exit 2, message names the missing argument) instead of clap's.
- **The surfaces that T's tests and the verb stories rely on (frozen when #670 merges):**

  | Item | Signature as built |
  |---|---|
  | verb entry | `pub fn run(args: &PaneXxx, ctx: &mut VerbCtx<'_>) -> i32`, dispatched by `pane::run(&PaneCmd, &mut VerbCtx) -> i32` and `profile::run(&ProfileCmd, &mut VerbCtx) -> i32` |
  | context | `VerbCtx<'a> { format: Format, ports: Ports<'a>, sink: Sink<'a> }`, `Sink<'a> { out: &'a mut dyn Write, err: &'a mut dyn Write }` |
  | printing | `emit(&mut Sink, Format, Result<T, ErrorBody>, impl FnOnce(&T) -> String) -> i32`, `emit_stream(&mut Sink, Format, impl Iterator<Item = Result<T, ErrorBody>>, impl Fn(&T) -> String) -> i32`, `emit_error(&mut Sink, Format, ErrorBody) -> i32`, `emit_usage_error(&mut Sink, Format, &str) -> i32` |
  | errors | `ErrorCode::new(&str) -> Result<_, InvalidCode>`, `From<&PaneError> for ErrorCode` and `ErrorBody`, `ErrorBody { code, message }`, `not_implemented(story) -> ErrorBody` |
  | wiring | `Wiring::connect() -> Result<Wiring, PaneError>`, `Wiring::ports(&self) -> Ports<'_>`, `Unwired` |
  | flags | `SpecFlags::validate(&self) -> Result<SpecValues, PaneError>`, `ProfileOpt { profile }`, `SpecOnly { spec_only }` |
  | prompt verbs | `Say`/`Interrupt`/`Answer::resolve() -> Result<PromptArgs, Usage>`, `route(Result<PromptArgs, Usage>, &ProfileOpt) -> Result<Routed, Stop>` |

## Deviations from spec / wireframe

No wireframe (no UI). From the brief:
- `resolve_format(&Cli)` became `resolve_format(json: bool, format: Option<Format>)` (A W-8; decision 7 lets F adjust). No test pins it.
- `main.rs` changed more than decision 1 lists: the whole dispatch is one exhaustive `match`, not "an extracted helper plus two arms" (A W-9; the compiler now catches a missing arm). The hub and body code is moved, not rewritten.
- `--ctx-soft`/`--ctx-hard` are `u32` in clap, not strings (decision 10 says values stay strings at clap time; for numbers the outward behaviour is the same: exit 2, code `usage`).
- ADR 0003: the `--pane` forms of `say`/`interrupt`/`answer` are their own rows (decision 14 above), not additions to the SESSION rows.
- `emit` has the write-failure and newline rules of decision 6, which the brief does not mention.

## Tier 1 self-check (incl. tests now GREEN)

Run in the worktree after the last code edit (the full-workspace run is the last block):

```
cargo build --workspace                                   ok
cargo clippy --workspace --all-targets -- -D warnings     ok, no warnings
cargo machete                                             "didn't find any unused dependencies"
bash scripts/lint.sh                                      exit 0 (only the 600-line warns; cli.rs is 780, over its 701 but under 900)
bash scripts/changelog-check.sh                           changelog-check: ok
bash scripts/test-hooks.sh                                all ok
rustfmt --check --edition 2021 <each of the 27 new .rs>   clean
```

Formatting of existing files: no line I added or changed deviates from rustfmt (checked by diffing `rustfmt --check` output against `git diff -U0`); `main.rs`, whose changed hunks are nearly the whole file, is fully rustfmt-clean; no untouched line was reformatted.

The targets that were RED, after the last edit:

```
cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test --test cli_invocation_test
  cli_invocation_test 14 passed   docs_cli_test 3   cli_surface_test 3   pane_cli_process 33   pane_verbs 53   profile_verbs 9   (0 failed)
```

The whole workspace, last, after the final code edit (a `prompt_target::route` refactor that put the resolve-and-guard preamble of the three verbs in one place):

```
cargo test --workspace --no-fail-fast
  exit 0; 98 test binaries with results, 1023 passed, 0 failed, 5 ignored (the ignored tests are in other crates and are unchanged)
```

By hand, on the real binary (state dir empty, no hub): `holler pane list` -> `error: not implemented (story #643)` on stderr, exit 1, stdout empty; `holler pane list --json` -> `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"not implemented (story #643)"}}` and exit 1; `holler pane launch --spec-only --format=json` -> a `usage` envelope naming `--profile`, exit 2; `holler roster --bogus --json` -> clap's plain message, exit 2; `holler say --pane x hello` -> `error: not implemented (story #646)`, exit 1; `holler say --pane x` -> exit 2 naming TEXT; `holler pane list --json >&-` and `| head` do not panic.

## Evidence appendix

`docs/handoffs/670/evidence.md` (nine source facts in unchanged code, each with a verbatim excerpt; every excerpt was checked against its line range).

## Tests that look wrong (for T)

None. Every pinned name and signature in handoff-T-red.md is built as pinned.

Not wrong, but not covered, for T to consider in Phase 7: `emit`'s write-failure and encode-failure paths, one-line JSON messages, and the newline rule; `resolve_format` and `scan_args` as units (today only the binary shows them); `SpecFlags::validate()` for `--role`, for `--command-arg` with `--command-json` built by hand, and for `--ctx-soft`; every method of `Unwired` answering `not-implemented`; `route()` directly. The brief's Test plan puts these with T.

## Known issues

None against the acceptance criteria. Things the next story or the operator should know:
- **`docs/protocol/v2.md` §10** says it reproduces ADR 0003's table "verbatim". It had already drifted (no `--server` forms) and this change widens the gap. v2.md belongs to #634 (A W-11): #634 should resync §10 or drop the word.
- **`roster --format=json`** prints the legacy `--json` document, not the envelope, until #648 reads `FormatChoice.json_explicit` (the one `main.rs` line decision 1(c) reserves for it).
- **`--command-arg`/`--check-arg`** and a hyphen-leading value: use `--command-arg=--flag` (decision 13). #644 owns the choice.
- **`cli.rs` is 780 lines** (warn at 600, fail at 900); the next story to add to it should know.

## Files changed

Production, created (27), all under `crates/holler-cli/src/`:
`output.rs`, `prompt_target.rs`, `pane/mod.rs`, `pane/args.rs`, `pane/wiring.rs`, `pane/profile_scope.rs`, `pane/list.rs`, `pane/get.rs`, `pane/watch.rs`, `pane/launch.rs`, `pane/relaunch.rs`, `pane/switch.rs`, `pane/reset.rs`, `pane/park.rs`, `pane/unpark.rs`, `pane/close.rs`, `pane/doctor.rs`, `pane/import.rs`, `profile/mod.rs`, `profile/create.rs`, `profile/delete.rs`, `profile/list.rs`, `profile/show.rs`, `profile/apply.rs`, `profile/rename.rs`, `profile/export.rs`, `profile/import.rs`.

Production, modified:
`crates/holler-cli/src/cli.rs`, `crates/holler-cli/src/lib.rs`, `crates/holler-cli/src/main.rs`, `crates/holler-cli/src/say_cmd.rs`, `crates/holler-cli/src/interrupt_cmd.rs`, `crates/holler-cli/src/answer_cmd.rs`, `docs/adr/ADR-0003.md`, `CHANGELOG.md`.

Pipeline artifacts: `docs/handoffs/670/handoff-F.md`, `docs/handoffs/670/evidence.md`, `docs/handoffs/670/decisions.md` (F entry appended).

No test file, fixture, manifest or lockfile was edited by F (the manifest, `Cargo.lock`, the fixture and all tests are T's, committed at 0afbb93).
