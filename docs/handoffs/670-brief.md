# Brief: #670 the pane/profile CLI skeleton (clap tree, output module, `main.rs` dispatch, verb stubs, ADR 0003)

Repo: Performant-Labs/holler. Issue: #670 (slice c of epic #633's skeleton; slice a #637 is merged at `f2602ba`, slice b is
#669). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-670-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633 and the
epic's "Skeleton split" section are fixed; ADR-0021 (#634) ratifies them later. The issue text of #670 is the source of truth with
the epic; where they differ from this brief, the issue wins.

**Revision 1.** Written from the merged `holler-pane` crate (not from the earlier design), after the single-story brief drew three
architecture BLOCKs and was split (2026-10-09). Its CLI half is the design basis; decision numbers below keep the earlier
numbering (0, 2, 3, 4 and 8 belong to #637 and #669 and are omitted). This slice is large (about 25 new files plus `main.rs`): if
its architecture review blocks on size, the issue says to split it again into c1 (`main.rs`, `output.rs`, format resolver, usage
errors) and c2 (clap verbs, ADR 0003, fixture).

## Problem

Thirteen wave-3 stories add `holler pane` and `holler profile` verbs in parallel. They can only do so if everything they share in
`holler-cli` is created once, up front: the clap tree, the global `--format`, the `--pane`/`--profile` shapes on `say`,
`interrupt`, `answer` and `roster`, the output envelope and its `emit` functions, the `main.rs` dispatch, one stub file per verb
(each with its own clap `Args` struct), the ADR 0003 rows and the CLI fixture lines. This slice creates no behaviour: every verb
prints "not implemented (story #NNN)" and exits 1. It uses the merged `holler-pane` crate for error codes, the `Ports` bundle,
`SpecFlags` validation types and the grid/argv/env guards; it touches no hub code (that is #669).

## Evidence (verbatim, as of `f2602ba`; the CLI sources are unchanged since `27da2da`)
The workspace takes every crate under `crates/`; new crates carry `[lints] workspace = true`:
```
Cargo.toml:1-3
[workspace]
members = ["crates/*"]
resolver = "2"
```
```
Cargo.toml (workspace.lints)
unwrap_used = "deny"  expect_used = "deny"  panic = "deny"  unreachable = "deny"
cognitive_complexity = "deny"  too_many_lines = "deny"  struct_excessive_bools = "deny"
dead_code = "deny" (rustc)
clippy.toml: cognitive-complexity-threshold = 15 ; too-many-lines-threshold = 100
```
`cargo machete` fails CI on an unused dependency (`crates/holler-hub/Cargo.toml`: "Declare only what is consumed"); a
dependency with a feature list needs a `# for <consumer>` marker (lint check 5); file-size gate (lint check 4): warn at 600,
**fail at 900** lines.

**The CLI dispatches, and may exit, only in `main.rs`** (`process::exit` is allowed only in a bin's `main.rs`, lint check 2).
Parsing is `Cli::parse()`, which exits inside clap:
```
crates/holler-cli/src/main.rs:96
    let cli = Cli::parse();
```
Dispatch is an `if let` chain, not an exhaustive `match`; a variant with no arm falls off the end of `main` and exits 0:
```
crates/holler-cli/src/main.rs:203-238
    if let Command::Roster(roster) = &cli.command {
        let result = holler_cli::roster_cmd::run(roster, cli.json);
        print_leaf_result_and_exit(&result.message, result.to_stderr, result.exit_code, false);
    }
    if let Command::Say(say) = &cli.command { ... say_cmd::run(say, cli.json) ... }
    (same shape for Interrupt, Answer, Hold, Release, Wait)
    // Every `Command` variant is handled by one of the arms above (each
    // exits before falling through), so this is never actually reached ...
}
```
`main()` (84-238) is at the clippy `too_many_lines` limit (the reviewer counted 99 of 100 code lines), so any new arm needs the
existing chain extracted into a helper first. The established stub wording is `error: not implemented (story <name>)` on
stderr, exit 1 (`main.rs:5-8`), and exit 3 is the fail-closed policy refusal (`holler pane list --debug bogus` exits 3 before
dispatch).

The global flags and the top-level command enum (`--json` exists and is global; the enum gains two variants):
```
crates/holler-cli/src/cli.rs:57-63
    #[arg(long, global = true)]
    pub json: bool,
```
```
crates/holler-cli/src/cli.rs:69-89
pub enum Command { Hub(Hub), Body(Body), Roster(Roster), Say(Say), Interrupt(Interrupt), Answer(Answer), Wait(Wait), Hold(Hold), Release(Release) }
```
`say`, `interrupt` and `answer` take a required positional SESSION; this is why `say --pane X TEXT` cannot parse today:
```
crates/holler-cli/src/cli.rs:442-460 (Say), 479-489 (Interrupt), 512-525 (Answer)
pub struct Say       { pub session: String, #[arg(required_unless_present = "parts_file")] pub text: Option<String>,
                       #[arg(long)] pub parts_file: Option<String>, timeout, queue, grant, server }
pub struct Interrupt { pub session: String, pub text: Option<String>, #[arg(long)] pub server: Option<String> }
pub struct Answer    { pub session: String, pub choice: String, #[arg(long)] pub server: Option<String> }
```
The reviewer checked this against the workspace's clap 4.6.6 in a scratch crate: with the new flags added, or with SESSION made
`Option` (with or without `conflicts_with = "pane"`), `say --pane hj-c1r1 hello` is refused because `hello` binds to SESSION;
`allow_missing_positional` parses the `--pane` forms but breaks the existing `say io/alpha --parts-file F`
(`crates/holler-cli/tests/talk_test.rs:620-665`) and `interrupt io/alpha` (a `cli-surface.txt` line). The three verbs consume the
fields as `&str` at `say_cmd.rs:121`, `interrupt_cmd.rs:35`, `answer_cmd.rs:36`. The in-repo precedent for a tail that clap
cannot express as fixed positionals, resolved in code behind accessors:
```
crates/holler-cli/src/cli.rs:242-262
/// `query CMD [ARGS...]` (local) or `query TARGET CMD [ARGS...]` (remote).
/// ... two shapes the clap derive tree can't express as fixed positionals ... the tail is captured
/// as one variadic and split in code by [`Query::resolve`].
pub struct Query { pub server: Option<String>, #[arg(required = true, trailing_var_arg = true)] pub rest: Vec<String> }
```
**ADR 0003 fixes the complete CLI surface** and must move with it:
```
docs/adr/ADR-0003.md:12
This ADR fixes the **complete CLI surface** and the **versioning policy** ... Later stories implement exactly this surface
docs/adr/ADR-0003.md:21     global flags on every subcommand:  --debug none|quiet|noisy   --log-format text|json   --json
docs/adr/ADR-0003.md:61     **Exit codes:** `0` ok; `1` runtime failure ...; `2` usage/ambiguity; `3` fail-closed policy refusal
docs/adr/ADR-0003.md:62     `--json` prints **exactly one JSON object** to stdout; **all diagnostics go to stderr.**
docs/adr/ADR-0003.md:69     "Breaking" = the CLI surface, the `--json` shape, the on-disk formats ...
docs/adr/ADR-0003.md:75     `say`/`interrupt`/`roster` are the only top-level verbs
```
Earlier verb-adding stories (2bbc96c, db76a6b, d8b2d62, 7e2f595) each updated ADR 0003 in the same change. `docs_cli_test` parses
every `holler ...` line shown in `docs/**` against the clap tree.

The CLI surface fixture is checked in both directions, and `holler-cli` declares every test target by hand:
```
crates/holler-cli/tests/cli_surface_test.rs:1-15 (header)
//! 3. the set of leaf verbs in the fixture equals the set clap knows — a verb
//!    added to the tree without a fixture line (or vice versa) fails here.
crates/holler-cli/Cargo.toml:14  autotests = false     (48 [[test]] entries today)
```

**The merged `holler-pane` API this slice builds on** (read the crate, do not copy it): `holler_pane::{Ports<'a> (Copy; one &dyn
each of pane_store, profile_store, herdr, host, harness, scope, prober), PaneError (code(), Display, ::NotImplemented,
::Refused via RefusalCode), error::is_valid_code (a const fn), GridPos, Argv, EnvVarName, PaneName, ProfileName, Actor,
ProfileScope, ProfileStore, PaneStore, ...}`. `PaneReply` is the hub's control-socket reply, not the CLI's output envelope.

## Acceptance criteria

All must be observable by command. Names below are the test names T should author (RED first). Numbering is this brief's own.
1. `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo machete`, `bash scripts/lint.sh` and `bash scripts/changelog-check.sh` pass; no file over 900 lines; `main()` stays at
   or under 100 code lines (clippy `too_many_lines`). **Formatting:** the tree is not rustfmt-clean today (176 of 195 `.rs`
   files fail `cargo fmt --check` on origin/main, CI has no fmt step, and formatting `control_server.rs`/`serve.rs` would take
   them past 900 lines), so the gate is: every **new** `.rs` file passes `rustfmt --check --edition 2021`, and existing files are
   **not reformatted** (their changed hunks add no rustfmt deviation; repo practice, cb7fe39). `cargo fmt --check` on the whole
   workspace is not an acceptance criterion.
2. Every stub verb (`holler pane`: list get watch launch relaunch switch reset park unpark close doctor import; `holler profile`:
   create delete list show apply rename export import) in text mode prints `error: not implemented (story #NNN)` to **stderr**,
   names its owning story, prints nothing on stdout, and exits **1** (test `stub_verb_not_implemented`). With `--format=json` (or
   `--json`) it prints exactly one envelope on stdout and nothing else:
   `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"<the same one line>"}}`; exit 1.
   A bare `holler pane` / `holler profile` is a clap usage error, exit 2. `holler pane list --debug bogus` still exits 3.
3. Usage errors exit 2, and in JSON mode **under the `pane` and `profile` subcommands only** are an envelope with code `usage`
   (`emit_usage_error`; decision 1(d) and 9 keep every legacy verb's `--json` usage-error output as it is today): `pane launch X --spec-only`
   without `--profile` (`spec_only_requires_profile`); a bad `--format` value; `--json --format=text` (`json_conflicts_text`);
   `--command-arg` with `--command-json` and `--check-arg` with `--check-json` (mutually exclusive). `--help` and `--version` still
   exit 0 and print what they print today; a text-mode clap usage error still prints clap's own message to stderr.
4. The `--pane`/`--profile` shapes on the existing verbs (decision 5). These forms parse and populate fields, asserted by value:
   `say SESSION TEXT`, `say SESSION --parts-file F` (both unchanged), `say --pane NAME TEXT`, `say --pane NAME --parts-file F`,
   `say --pane NAME --profile P TEXT`; `interrupt SESSION [TEXT]`, `interrupt --pane NAME [TEXT]`; `answer SESSION CHOICE`,
   `answer --pane NAME CHOICE`; `roster --profile P`. Tests assert the resolved target (session or pane), the prompt text, and
   that the existing forms resolve to the same values as before. Given `--pane` or `--profile`, each of the three verbs refuses
   with exit 1 and `error: not implemented (story #646)` on stderr, **plain text under any format** (they are legacy verbs;
   no envelope, so #646 inherits one shape), before contacting any hub (a `--pane` that parses and is ignored would deliver a
   prompt unchecked); `roster --profile P` likewise refuses with `error: not implemented (story #648)`, exit 1. The accessor
   returns the existing `cli::Usage` error (exit 2, a message naming the missing argument; no new usage-error type) when SESSION, TEXT or CHOICE is missing, when `--pane` is given together
   with a SESSION, and when an extra positional follows `--pane NAME TEXT` (`say`, `answer --pane NAME`, bare `say`,
   `say --pane X a b`). Flags after SESSION still parse (`say io/alpha hello --queue`, `say io/alpha --parts-file f`).
   The existing say/interrupt/answer tests pass unchanged.
5. Every spec flag parses on `pane launch` and `pane relaunch`: `--project --workspace --grid --model --effort --role --env
   (repeatable) --ctx-soft --ctx-hard --port-policy --command-arg (repeatable) --command-json --check-arg (repeatable)
   --check-json --expect (repeatable)`. `--profile` parses on every `pane` verb except `import`, and on `roster`; `--spec-only` on
   `launch`, `relaunch`, `close` only; `--take-over` on `profile apply` only. **Only these shared flags are declared by #670**
   (decision 10): the verb-specific positionals and flags of the sibling issues are not, because each verb's clap struct lives
   in that verb's own file and its owning story adds them (so `pane get hj-c1r1` is a usage error, exit 2, until #643 lands;
   the stubs take what #670 declares). The values stay strings at clap time; the shared
   `SpecFlags::validate()` in `pane/args.rs` turns them into typed values: a bad `--grid` is a refusal (exit 1, code
   `grid-ambiguous` or `grid-out-of-range`), a bad `--env` is a refusal (exit 1, code `env-name-invalid`), a malformed
   `--command-json` or `--check-json` that is not an array of strings is `command-not-argv`. Tests call `validate()` directly.
6. **Seam test (the signatures of decision 7 are provisional until this compiles).** One in-process test per verb binary
   (`pane_verbs`, `profile_verbs`), through the shared `tests/verb_harness/mod.rs`, calls a stub's `run` with captured out and err
   writers in text and JSON mode over a `holler_pane::Ports` built from in-test stubs, and asserts the routing: text mode writes the
   refusal to err and nothing to out; JSON mode writes exactly one envelope to out and nothing to err. A stub that calls `emit`
   with an error and with a success is covered, and `emit_stream` writes one line per item. `ErrorCode::new` rejects an invalid
   code via `holler_pane::error::is_valid_code` (no second validator in holler-cli); `ErrorCode::from(&PaneError)` is infallible.
7. ADR 0003 carries the new rows (decision 6): **one row per verb, in the bare form this slice declares**, so a verb story edits
   only its own row (and its own `cli-surface.txt` line); **rows are grouped by owning story with a separator** (a `# #NNN` comment
   line in the fixture, a blank line in ADR 0003's text block), because adjacent-line edits conflict on rebase.
   `cargo test -p holler-cli --test docs_cli_test` and `cli_surface_test` pass (the first alternative of each `a|b` in an ADR row
   must parse). Every new leaf verb and every shared flag (the spec flags, `--profile`, `--spec-only`, `--take-over`, `--format`,
   `--pane`) appears on at least one line of `crates/holler-cli/tests/fixtures/cli-surface.txt`, plus the regression lines
   `say io/alpha --parts-file f` and `interrupt io/alpha`.
8. `git diff --name-only origin/main...HEAD` lists only paths in the Blast radius below (grep-able check for S).

## Files

- `crates/holler-cli/src/cli.rs` (edit, small): `Pane(PaneCmd)` and `Profile(ProfileCmd)` variants, global `--format`, the
  `--pane`/`--profile` fields on `Say`/`Interrupt`/`Answer`, `--profile` on `Roster`; update the namespace comment at `cli.rs:7-9`.
- New `crates/holler-cli/src/prompt_target.rs`: the `--pane`/`--profile` accessors for say/interrupt/answer (keeps `cli.rs` near 740
  lines).
- New `crates/holler-cli/src/pane/mod.rs` (`PaneCmd`, the dispatch) + `args.rs` (**only** the shared flag groups: `SpecFlags` with
  `validate()`, `ProfileOpt`, `SpecOnly`; defined once) + one stub per verb, each holding **its own clap `Args` struct** and `run`
  (`list.rs get.rs watch.rs launch.rs relaunch.rs switch.rs reset.rs park.rs unpark.rs close.rs doctor.rs import.rs`) +
  `wiring.rs` (stub: builds a `Ports` whose every port returns `PaneError::NotImplemented`; #649 replaces the body only) +
  `profile_scope.rs` (empty; #663 fills it).
- New `crates/holler-cli/src/profile/mod.rs` (`ProfileCmd`; no `args.rs`) + `create.rs delete.rs list.rs show.rs apply.rs rename.rs
  export.rs import.rs` (each with its own `Args` struct; `--take-over` lives in `apply.rs`).
- New `crates/holler-cli/src/output.rs`; `lib.rs` (edit): module declarations.
- `crates/holler-cli/src/main.rs` (edit): see decision 1.
- `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs` (edit, accessor change and the fail-closed guard only).
- `docs/adr/ADR-0003.md` (edit): decision 6. `CHANGELOG.md`: an `## [Unreleased]` entry linking #670.
- `crates/holler-cli/Cargo.toml`: `[[test]]` entries for `pane_verbs`, `profile_verbs`, and the placeholder `pane_integration` (#649) and
  `profile_apply_scenario` (#667); the `holler-pane` dependency and the `holler-pane-testkit` dev-dependency with a one-line
  consumer test (`testkit_links`), so #638, #643-#647 add no manifest line.
- Tests: `crates/holler-cli/tests/pane_verbs/main.rs` and `profile_verbs/main.rs` (one `[[test]]` each) **plus the shared
  in-process harness `tests/verb_harness/mod.rs`** (included by both binaries via `#[path]`; it builds a `VerbCtx` over given
  ports, captures out and err, and runs a verb; first caller: the seam test, so nine stories do not write near-copies) **plus every
  per-verb file now**: `pane_verbs/<verb>.rs` x12 and `profile_verbs/<verb>.rs` x8, each holding that verb's stub case and declared
  in `main.rs`, so a later story edits only its own file and never `main.rs` or the manifest. Placeholder
  `pane_integration/main.rs` (#649) and `profile_apply_scenario/main.rs` (#667) `[[test]]` targets hosted by `holler-cli`,
  `crates/holler-cli/tests/fixtures/cli-surface.txt` (edit).

Reuse map (extend, do not duplicate): the clap `Cli` and global-flag pattern in `cli.rs`; the `Query::resolve` pattern and the
root-exported `cli::Usage` type for the `--pane` accessors (its variadic tail, but **without** `trailing_var_arg`; its `new` becomes
`pub(crate)`); `holler_pane` for every code, guard and port type (do not copy a grammar or a validator);
`print_leaf_result_and_exit` and the existing stub wording; `tempfile`, `assert_cmd`, `predicates`, `rstest` for tests. Do not add a
second usage-error type.

## Decisions already made (MO)

Accepted by the operator 2026-10-08 and 2026-10-09; numbering follows the earlier combined brief.

1. **`main.rs` is in the blast radius**, scoped to: (a) one dispatch arm each for `Pane` and `Profile`, delegating to
   `holler_cli::pane::run` / `profile::run`, which return an exit code; (b) extract the existing top-level-verb chain
   (`main.rs:203-238`) into one helper so `main()` is at or under 100 code lines (the helper also holds the `roster --profile`
   refusal, `not implemented (story #648)`); (c) call the single format resolver once after parsing and pass `roster_cmd::run`
   the same `json` bool as today (#648 takes the one-line `main.rs` edit it needs for the explicit-`--format=json` bit); (d)
   switch `Cli::parse()` to `Cli::try_parse()`: `--help` and `--version` exit 0 as today, a text-mode usage error prints clap's
   own message and exits 2 as today, and in JSON mode **only when the first non-flag argv token is `pane` or `profile`**, skipping the values of `--debug`, `--log-format` and `--format` in both the `--x v` and `--x=v` forms (a pure
   helper in `output.rs` scans the raw argv for `--format=json`, `--format json` or `--json`) the usage error is
   `output::emit_usage_error` (an envelope with code `usage`, exit 2). Every legacy verb's usage error is untouched (no change
   to its `--json` output, ADR-0003:69). `output.rs` never calls `process::exit`; it returns the code. #660 therefore never
   needs `main.rs`.

5. **`--pane` shapes by redesign now.** `Say`, `Interrupt` and `Answer` keep their existing behaviour for `SESSION` forms and gain
   `--pane NAME` and `--profile P`. SESSION/TEXT/CHOICE become one variadic tail resolved in code behind accessors in the
   `Query::resolve` style, so `say --pane NAME TEXT` parses. Verified by the reviewer in a scratch crate on clap 4.6.6: use
   `#[arg(value_names = ["SESSION","TEXT"], num_args = 0..=2)] rest: Vec<String>` (`1..=2` for `answer`) **without**
   `trailing_var_arg` (`Query`'s `trailing_var_arg = true` would swallow `--parts-file`, `--queue`, `--grant`, `--server` into the
   tail); flags after SESSION still parse and `say io/alpha hello extra` stays clap's exit 2. Because the tail is optional the
   accessor returns a usage error (exit 2) for a missing SESSION/TEXT/CHOICE and for an extra positional with `--pane` (AC 4).
   The accessors live in the new `crates/holler-cli/src/prompt_target.rs`. `say_cmd.rs`, `interrupt_cmd.rs` and
   `answer_cmd.rs` change only to read the accessors and to refuse `--pane`/`--profile` with exit 1
   `error: not implemented (story #646)`, plain stderr text under any format. Accessor names must not collide with existing root
   exports (`Target`, `List`, `Delete`). (The earlier brief's "existing positional/`--session` forms" was inaccurate: these verbs
   have no `--session`.)

6. **ADR 0003 is edited by #670**: rows for `holler pane ...` and `holler profile ...`, `--format` on the global-flags line,
   `--pane`/`--profile` on the say/interrupt/answer/roster rows, the amended "only top-level verbs" sentence, citing epic #633 and
   ADR-0021 (#634); `cli.rs:7-9` matches. ADR-0021 stays #634's. Layout: rows grouped by owning story with a separator (AC 7).
   One added sentence says the `pane` and `profile` rows list only the shared flags and each owning story (epic #633) adds its
   positionals and flags. Line 62 ("`--json` prints exactly one JSON object") is amended to cover the envelope and `pane watch`'s
   NDJSON, citing #633 and ADR-0021.

7. **Verb entry, writers and the output API (provisional until compiled).** The signatures below are the brief's best design, not
   a proof. The implementer makes them compile together with the seam test (AC 6) and may adjust them, recording each change in
   decisions.md; they freeze when #670 merges. `ProfileScope`, `Ports` and `Prober` belong to #637 and are already frozen: a
   change to them goes through the epic's amend-first rule.
   - **Verb entry and writers.** `pub fn run(args: &XArgs, ctx: &mut VerbCtx) -> i32`, with
     `VerbCtx<'a> { format: output::Format, ports: &'a Ports<'a>, sink: output::Sink<'a> }` in **holler-cli** (it holds a
     holler-cli type) and `Ports { pane_store, profile_store, herdr, host, harness, scope, prober }` (one `&dyn` each) in
     **holler-pane**. `output::Sink<'a> { out: &'a mut dyn Write, err: &'a mut dyn Write }`; the real `main` builds it over
     stdout/stderr, tests over buffers, and `roster_cmd` can build one. `&mut VerbCtx` is required: a verb cannot write through `&`.
     `wiring.rs` builds the real bundle, #638 the fake, and `pane/mod.rs` and `profile/mod.rs` never change after this story.
   - **`output.rs` API:** `Format { Text, Json }`; `Envelope<T>`; `ErrorBody { code: ErrorCode, message }`;
     `emit<T: Serialize>(sink: &mut Sink, format: Format, result: Result<T, ErrorBody>, text: impl FnOnce(&T) -> String) -> i32`
     (0 ok, 1 error; the per-verb `text` closure renders text mode); `emit_stream<T: Serialize>(sink: &mut Sink, format: Format,
     items: impl Iterator<Item = Result<T, ErrorBody>>, text: impl Fn(&T) -> String) -> i32` (one line per item);
     `emit_usage_error(sink: &mut Sink, format: Format, message: &str) -> i32` (returns 2); `resolve_format(&Cli) ->
     Result<FormatChoice, ErrorBody>` and the raw-argv scan. **Routing:** text mode writes ok data to `out` and any error or
     diagnostic to `err`, nothing on `out` for an error; JSON mode writes exactly one envelope to `out` (error envelopes
     included) and nothing to `err` unless a verb logs a diagnostic. A stub writes `error: not implemented (story #NNN)` to `err`
     in text mode.

9. **`--format` and `--json`.** One resolver in `output.rs` applies to every verb. `--format=json` is equivalent to `--json` on every
   verb; `--json --format=text` is a usage error (exit 2). Legacy verbs keep their legacy JSON shape and their legacy usage-error output (decision 1(d)). The resolver also
   reports whether `--format=json` was explicit (`FormatChoice { format, json_explicit }`), so #648 can emit the envelope for
   `roster` only on explicit `--format=json` and keep `roster --json` byte-compatible (no break under ADR-0003:69); #648 takes
   the one `main.rs` line that passes the bit (decision 1(c)). Pane and profile verbs treat either flag as the envelope. Exit
   codes: pane/profile verbs return 0 ok, 1 refused or failed, 2 usage; 3 (policy refusal) stays in `main.rs`; 4
   (`HELD_EXIT_CODE`) belongs to `hold`/`release` and is not touched.

10. **Smaller rulings.** **Each verb's clap `Args` struct lives in its own verb file** (`pane/<verb>.rs`, `profile/<verb>.rs`),
    named `PaneList`, `ProfileDelete` and so on, and is referenced from the frozen `PaneCmd`/`ProfileCmd` enums in `mod.rs`. Only
    the shared groups (`SpecFlags`, `ProfileOpt`, `SpecOnly`) live in the frozen `pane/args.rs`, each defined once (`roster` and
    other users import `ProfileOpt` from there; no profile verb takes `--profile`, so `profile/args.rs` is not created). #670 declares each verb with what it
    is already known to need (shared groups only), and **does not** guess the positionals or verb-specific flags of the sibling
    stories (`pane get PANE`, `pane switch PANE SESSION`, `pane park` reason and release condition, `pane doctor [--fix]`,
    `pane import --from`, `profile create/delete/show NAME`, `profile rename OLD NEW` and so on; `--take-over` on apply **is**
    declared by #670, in `apply.rs`): each
    owning story adds them in its own file, edits its own ADR 0003 row and its own `cli-surface.txt` line, and no frozen file.
    This follows the epic's "one verb, one file, one owning story" rule and is the reviewer's accepted alternative; recorded in
    decisions.md. `--grid` stays a string at clap time and is typed by `SpecFlags::validate()` (the issue says `cli.rs` declares it; the brief's choice
    gives a stable error code instead of a clap exit 2; recorded in decisions.md). The accessor errors reuse the root-exported
    `cli::Usage` (`Query::resolve`'s type; its `new` becomes `pub(crate)`). Path-qualify `output::Envelope`, never the `holler_proto::Envelope` root re-export. `src/**/mod.rs` is new to
    this codebase and kept because each module root sits inside its owner's blast-radius glob; recorded in decisions.md.
    `Pane.hold` is doc-commented as a pane-record state, not a prompt hold; #646's brief must put any prompt refusal derived from
    pane state at `send_prompt` (the stack's choke-point rule), not in the verb.

## Out of scope

Any behaviour behind a stub; the real verbs (#643-#647, #650, #662, #664, #665); `--pane` routing in say/interrupt/answer (#646: here
they only parse and refuse); `roster` rendering (#648: `roster` gains only `--profile`, refused with "not implemented (story
#648)"); anything in `holler-hub` (#669) or `holler-pane` (#637, merged); `docs/protocol/v2.md` and ADR-0021 (#634).

## Test plan

RED first (T): the `pane_verbs`/`profile_verbs` stub tests, the seam test, the `--pane` shape tests, the usage-error and
`--format` tests, the fixture lines (which make `cli_surface_test` fail until the verbs exist), and the ADR 0003 rows (which make
`docs_cli_test` fail until the clap tree matches). Confirm RED by `cargo test -p holler-cli` failing to build or on named tests,
and that no RED is a missing `[[test]]` entry. GREEN: F creates the stubs and plumbing until the whole workspace is green. Then
`cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check --edition 2021` on each new `.rs` file (not the whole
tree), `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`, `bash scripts/test-hooks.sh`.

## Risks

- `main.rs` is at the clippy line limit (99 of 100): extract the existing chain into a helper first, in its own step, then add the
  arms.
- `Cli::try_parse()` must keep `--help`, `--version`, bare `holler` and unknown-subcommand behaviour byte-identical in text mode
  (the existing `cli_invocation_test` covers this); exit 3 on `--debug bogus` is unchanged.
- A `--pane` form that parses but is ignored fails open: the guard in the three `*_cmd.rs` files is part of the story.
- `dead_code = "deny"`: stubs must be `pub` or used; `cargo machete`: no unused dependency or dev-dependency.
- `holler-cli` has `autotests = false`: every new test target needs a `[[test]]` entry.
- Name collisions: path-qualify `output::Envelope`, never the `holler_proto::Envelope` root re-export; `Pane`/`Profile`,
  `List`/`Delete` already exist as root exports (decision 10).
- `cli.rs` is 701 lines: the new fields stay small; the accessors live in `prompt_target.rs`.

## Blast radius

`crates/holler-cli/src/cli.rs`, `lib.rs`, `main.rs`, `output.rs`, `prompt_target.rs`, `say_cmd.rs`, `interrupt_cmd.rs`,
`answer_cmd.rs`, `pane/**`, `profile/**`; `crates/holler-cli/Cargo.toml`; `crates/holler-cli/tests/pane_verbs/**`,
`profile_verbs/**`, `verb_harness/**`, `pane_integration/**`, `profile_apply_scenario/**`, `tests/fixtures/cli-surface.txt`;
`docs/adr/ADR-0003.md`; `Cargo.lock`; `CHANGELOG.md`; `docs/handoffs/670*` (pipeline artifacts). Not changed: `holler-hub`,
`holler-pane`, `holler-proto`, any golden file, ADRs other than 0003.
