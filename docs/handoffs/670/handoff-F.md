# Handoff-F: Phase 6 - #670 the pane/profile CLI skeleton (skeleton slice c)

**Date:** 2026-10-09
**Branch:** issue-670-implementation (pass 2 is uncommitted on top of 7939b54; pass 1 is commit 016eeaa)
**Issue:** #670
**Pass:** 2, the rework after S pass 2 (`handoff-S.md`, REWORK, item 1). "Rework 1" below is this pass. "Pass 1" after it is the original handoff, kept as written: its files, decisions and Tier 1 numbers still stand, except for `prompt_target.rs`, which this pass edits.

## Rework 1: refuse an extra positional in the SESSION form (S pass 2, item 1)

The run is non-interactive, so the confirmation table of the role doc is recorded here instead of waited on:

| Field | Value |
|-------|-------|
| GitHub issue | #670 |
| Working branch | issue-670-implementation (worktree `.claude/worktrees/0670-cli-skeleton`), HEAD 7939b54 |
| Build plan phase | pipeline Phase 6 (F), rework after S pass 2; the issue is the source of truth, there is no BUILD_PLAN |
| Input documents read | issue #670 (Scope: "Missing or extra positionals return the existing `cli::Usage` (exit 2)"), `docs/handoffs/670-brief.md`, `handoff-S.md` (pass 2) and the other handoffs of this run, `decisions.md`, `evidence.md`, `prompt_target.rs`, `cli.rs`, the three `*_cmd.rs`, `target_flags.rs`, `legacy_verbs.rs`, and `origin/main`'s `cli.rs` |
| Acceptance criteria count | 8 (the brief's numbering); this pass addresses AC 4, the one S marked NOT MET (S re-audits it once T's tests are in) |
| Handoff document path | `docs/handoffs/670/handoff-F.md` |

Scope: item 1 is the only production item of S's verdict. Items 2-4 are test files and manifest comments, which S assigned to T; F does not touch them.

### What was done

- `crates/holler-cli/src/prompt_target.rs`, `resolve_tail`:
  - The SESSION arm now returns `Usage` when the tail holds more than two positionals: `only SESSION and TEXT may be given, got 3 positionals: <forms>` (`CHOICE` in place of `TEXT` for `answer`). The `--pane` arm is unchanged.
  - The doc comment no longer says that "clap has already refused a third". It says that `num_args` bounds one occurrence, that a flag between positionals starts another, and that the check therefore lives here. The module doc's sentence ("a missing or surplus positional is the existing `Usage` error (exit 2)") is now true and is unchanged.
- That is the whole production change: one file, +15 / -2 lines, no signature changed.

### Proof (the built binary, an isolated empty state dir, no hub)

"Before" is the binary built at 07:29 MDT from the pass-1 sources (F's last `src` commit is from 07:21 MDT); "after" is the rebuild at 08:35 MDT (the key forms were run again on the final build at 08:41 MDT, after the last edit, with the same results). The `origin/main` column is read from `origin/main`'s `cli.rs` (`Say`, `Interrupt` and `Answer` declare fixed `session` + `text`/`choice` positionals there, so a third positional is clap's `UnknownArgument`) and from S's scratch-crate probe of `say`; I did not build and run `origin/main`.

| argv | `origin/main` | before | after |
|---|---|---|---|
| `say io/alpha hello --queue extra` | exit 2 (clap) | exit 1, `no live holler hub reachable` (it parsed `rest = [io/alpha, hello, extra]` and went on to the hub; S notes that a live hub would be sent `hello`) | exit 2, `only SESSION and TEXT may be given, got 3 positionals: ...`, stdout empty, no hub line |
| `say io/alpha --timeout 5m fix it` | exit 2 | exit 1, hub path | exit 2, same message |
| `interrupt io/alpha --server ws://127.0.0.1:1 stop now` | exit 2 | exit 1, `credential.json not found` (the remote path) | exit 2, same message with `holler interrupt SESSION [TEXT]` forms |
| `answer io/alpha --server ws://127.0.0.1:1 1 2` | exit 2 | exit 1, remote path | exit 2, `only SESSION and CHOICE may be given, got 3 positionals: ...` |
| `say io/alpha hello extra` (contiguous) | exit 2 | exit 2 (clap `TooManyValues`) | exit 2 (clap), unchanged |
| `say io/alpha --queue hello` (the valid split form) | session `io/alpha`, text `hello` | exit 1, `no live holler hub reachable` | the same, unchanged |
| `say --pane demo-c1r1 hello --queue extra` | n/a (new form) | exit 2, `--pane` arm | exit 2, unchanged |

The valid forms still reach the hub or the remote path (`interrupt io/alpha --server ws://127.0.0.1:1 stop`, `answer io/alpha --server ws://127.0.0.1:1 1`, `say io/alpha hello --queue`), and `say --pane demo-c1r1 hello` is still the #646 refusal (exit 1).

Order, after: each of these is exit 2 with the positional message, and none is the #646 refusal or an exit-3 error. So the usage error comes first, as S asked.
- `say --profile demo io/alpha hello --queue extra`
- `say io/alpha hello --queue extra --timeout bogus`
- `say io/alpha --parts-file /nonexistent.json hello extra`

`answer io/alpha 1 --server ws://127.0.0.1:1 2 3` reports `got 4 positionals`.

### Design decisions

1. **The count stays in code, in the SESSION arm of `resolve_tail`, as S prescribed.** Clap cannot bound the tail as a whole: `num_args` limits one occurrence. The alternatives are the shapes the brief ruled out: fixed positionals (`say --pane NAME TEXT` cannot parse) and `trailing_var_arg` (it swallows `--parts-file`, `--queue`, `--grant` and `--server`).
2. **One check per arm, not one before the `match`.** The two arms say different things: the `--pane` arm explains that `--pane` takes the place of SESSION, the SESSION arm names the two arguments that are allowed.
3. **The message gives the count and the forms, not the surplus values**, as the `--pane` arm does. The surplus words are the user's prompt text, and a usage error should not copy them into a log.
4. **The argument is named from `Tail::arg`:** `TEXT` for `say` and `interrupt`, `CHOICE` for `answer`. For `interrupt` the TEXT is optional, and the forms after the colon show the brackets.
5. **Order unchanged.** `route()` is the first statement of each verb's `run`, and a `Usage` from `resolve()` becomes `Stop { exit_code: 2 }` before the #646 refusal, `--timeout`, `--parts-file` or any hub (evidence.md, and the probes above).
6. **Not touched:** items 2-4 of S's list (the `STUBS` doc and the two refusal constants in `process/stub.rs`, the `use` line in `process/legacy_verbs.rs`, the comments in `process/main.rs` and `Cargo.toml`).

### Reuse / extend-vs-new

Extended `resolve_tail` and the existing `cli::Usage`. No new function, type or error, and nothing is duplicated.

### Architecture notes for A

None. One private function's error condition changed. `Say`/`Interrupt`/`Answer::resolve()` keep their signatures; they return `Err` where they returned `Ok` for a third positional, which is what `origin/main` did through clap. No module boundary, public interface or dependency direction changed (`archChanged: false`).

### Deviations from spec / wireframe

None. Pass 1's assumption in `decisions.md` ("a third positional a clap error") held only for the contiguous form. The correction is in the new F entry there.

### Tests T adds in T-green (S item 1; F writes none)

Names are suggestions. The argv forms and the expected results are S's.

1. **`crates/holler-cli/tests/pane_verbs/target_flags.rs`** (in process), for example `an_extra_positional_split_off_the_session_form_by_a_flag_is_refused`:
   - For each of `say io/alpha hello --queue extra`, `say io/alpha --timeout 5m fix it`, `interrupt io/alpha --server ws://127.0.0.1:1 stop now` and `answer io/alpha --server ws://127.0.0.1:1 1 2`: assert `parse(args).is_ok()` (clap accepts it; that is the premise), and that `resolve(args)` is an `Err` whose message contains `positionals` (so it is the accessor's `Usage`, not a parse error).
2. **The same file**, for example `a_flag_between_session_and_text_still_resolves_to_both`:
   - `resolve(&["say", "io/alpha", "--queue", "hello"])` is `Ok(expect(session("io/alpha"), Some("hello")))`, as it is on `origin/main`. The same for `interrupt io/alpha --server ws://127.0.0.1:1 stop` and `answer io/alpha --server ws://127.0.0.1:1 1`.
3. **`crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs`**, in `a_malformed_pane_form_is_a_usage_error_not_a_refusal` or beside it (the binary):
   - `say io/alpha hello --queue extra` exits 2, stdout empty, stderr names `positionals`, stderr is not the #646 refusal, and stderr has no `no live holler hub` line.
   - Optional, it pins the order S asked to keep: `say --profile demo io/alpha hello --queue extra` is the same (exit 2, not the refusal).

F checked the in-process shape of tests 1 and 2 by value with a scratch test kept outside the repo (compiled with `rustc --test` against the built `holler_cli` and `clap` rlibs, on the final build). The four split forms parse in clap and are refused by the accessor with the `positionals` message; the valid split forms for `say`, `interrupt` and `answer` resolve to both values; the `--pane` arm is unchanged. So the tests are right against the fix. The red side is T's to show.

Proof that the tests prove something (T): run them against the pass-1 `resolve_tail` (51f3bed; `git show 51f3bed:crates/holler-cli/src/prompt_target.rs`, or take the new `rest.len() > 2` check out). Tests 1 and 3 must fail there (`resolve` returns the session and the text; the binary exits 1 on the hub line). Test 2 passes before and after: it guards the valid split form.

### Tier 1 self-check (incl. tests now GREEN)

Run in the worktree after the last code edit:

```
rustfmt --check --edition 2021 crates/holler-cli/src/prompt_target.rs   clean (a new file, so it must be)
cargo build -p holler-cli                                               ok
cargo clippy --workspace --all-targets -- -D warnings                   ok, no warnings
cargo machete                                                           "didn't find any unused dependencies"
bash scripts/lint.sh                                                    exit 0 (only the 600-line warns; cli.rs is 780)
bash scripts/changelog-check.sh                                         changelog-check: ok
bash scripts/test-hooks.sh                                              all ok
cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_cli_process --test talk_test --test interrupt_test --test answer_cli_test --test cli_invocation_test --test cli_surface_test --test docs_cli_test
  pane_verbs 59, profile_verbs 8, pane_cli_process 34, talk_test 18, interrupt_test 11, answer_cli_test 11,
  cli_invocation_test 14, cli_surface_test 3, docs_cli_test 3: all passed, 0 failed
  (pane_verbs, profile_verbs, pane_cli_process, cli_surface_test and docs_cli_test match T-green's counts)
```

The whole workspace, last, after the final edit, with nothing else running alongside it:

```
cargo test --workspace --no-fail-fast
  exit 0; 98 test binaries with results, 1029 passed, 0 failed, 5 ignored (the ignored tests are in other crates and are unchanged)
```

That is T-green's total (1029 passed, 0 failed); this rework adds no test. Two earlier full runs, for the record:
- Before the last edit (a rewording of the doc comment of `resolve_tail`): exit 0, 98 binaries, 1029 passed, 0 failed, 5 ignored.
- A run after that edit failed 14 tests in `hold_hub_test` (1), `hub_hygiene_test` (4) and `interrupt_test` (9). I caused it: while that run was in its hub-test stretch I compiled a scratch probe (`rustc --test`, a 188 MB binary) into the session scratchpad on `/tmp`, which is a tmpfs mounted with `usrquota`. In 12 of the 14 failures the output says `Disk quota exceeded (os error 122)` (creating a `holler-test-*` state file, a body identity key, `sessions.toml`). One is a knock-on `NotFound` in `hub_hygiene_test`. One is a `connection_lost` race in `hold_hub_test` (`say_racing_release_is_delivered_or_refused_never_lost`). All of them fall within 20 seconds of the compile. I deleted the probe binary; the three targets then passed alone (16, 14 and 11 tests), and the full run above passed. None of the failures involves the #670 code.

### Evidence appendix

`docs/handoffs/670/evidence.md`, section "Rework 1": two facts in unchanged code, each with a verbatim excerpt (`cli.rs:499-500`, `say_cmd.rs:116-119`). The clap behaviour behind the defect is the crate's, so it is shown by the before/after run above and not quoted.

### Tests that look wrong (for T)

None. `target_flags.rs` and `legacy_verbs.rs` pass unchanged; they did not cover the split form, which is the gap S found.

### Known issues

None for item 1. Items 2-4 of S's verdict are open for T. S's advisory notes (rebase onto `origin/main` before the PR, the PR-body disclosure, and the notes for #649, #634, #644, #646 and #648) are unchanged and are not F's.

Environment, for T's workspace run: `/tmp` here is a 30 GB tmpfs mounted with `usrquota`, and this session's scratchpad on it already holds about 8 GB from earlier phases (one throwaway cargo target dir alone is 4 GB). A full `cargo test --workspace` writes its `holler-test-*` state dirs to `/tmp`, so with a heavy build running next to it the hub tests fail with `Disk quota exceeded`, as in the disturbed run above. Running nothing else during the workspace run, or clearing disposable scratch from earlier phases first, avoids it. I do not know the size of the quota.

### Files changed

- Production, modified: `crates/holler-cli/src/prompt_target.rs`.
- Pipeline artifacts: `docs/handoffs/670/handoff-F.md`, `docs/handoffs/670/evidence.md` (the "Rework 1" section), `docs/handoffs/670/decisions.md` (a new F entry).
- No test file, manifest, fixture or lockfile was edited.

## Pass 1: the original handoff (kept as written)

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
