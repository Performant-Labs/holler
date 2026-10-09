# Brief: #637 pane-control skeleton (`holler-pane` crate, verb and profile stubs, output envelope)

Repo: Performant-Labs/holler. Issue: #637 (story of epic #633). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-637-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633 is
fixed and ADR-0021 (#634) ratifies it later; this story builds against the epic text and does not wait for #634.

**Revision 4.** Revisions 1-3 each drew a BLOCK (6, 2, then 3 blocks; reviews in git at 16c27d7, c33c255, and in
`docs/handoffs/637/handoff-A.md`). The pattern: every signature this brief wrote down and froze was checked against the
compiler or the sibling issues and found short. Revision 4 therefore **freezes less**: only the module and file layout, the CLI
flag names and the error code list are frozen at brief time. The seam signatures (`VerbCtx`, `output.rs`, `ProfileScope`,
`Ports`, `Prober`) are **provisional until compiled** (decision 7): F compiles them together with the in-process stub test and
may correct a signature inside #637, recording each change in decisions.md; they freeze when #637 merges. Revision 4 also fixes
the three revision-3 blocks (the writer seam, the optional pane in `ProfileScope::resolve`, the rustfmt gate) and the cheap warns.
The operator chose this approach (Option 2) on 2026-10-09. Where this brief and the issue body differ, the brief names the
difference under "Blast-radius amendments".

## Problem

Epic #633 splits into 13 stories that run in parallel in wave 3. They can only run in parallel if every shared file
(workspace `Cargo.toml`, `cli.rs`, `lib.rs`, `methods.rs`, `main.rs`, the hub's control dispatch) and every shared type, trait
and signature is created once, up front, by this story. This story creates no behaviour: types with serde, traits with fixed
signatures, the grid parser, the `Argv` and `EnvVarName` guards, one stub file per verb, empty crates, a working-but-minimal
output envelope, and the plumbing that makes `holler pane list` and `pane/list` reach a stub instead of falling through.

## Evidence (verbatim, as of `27da2da`)

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

**The hub control socket answers every non-`control/` method with MethodNotFound**, and hub state reaches handlers only through
four arguments:
```
crates/holler-hub/src/control_server.rs:105-116 (dispatch_control, the tail of the match)
        Some(other) if other.starts_with("control/") => {
            dispatch_session_control(other, &cid, &obj, registry, roster, lockout).await
        }
        Some(other) => encode_error(&cid, Code::MethodNotFound, format!("unknown control method: {other}")),
```
```
crates/holler-hub/src/control_server.rs:24-29
pub async fn handle_control_conn(stream: UnixStream, registry: Registry, roster: Arc<Roster>, lockout: Arc<Lockout>)
crates/holler-hub/src/serve.rs:565   (its only caller)
tokio::spawn(crate::control_server::handle_control_conn(stream, registry.clone(), roster.clone(), lockout.clone()));
```
`control_server.rs` is 829 lines and `serve.rs` 833, so the plumbing must be a few lines and the logic must live in the new
modules; `dispatch_control` is already split to stay under the cognitive-complexity limit (a new arm goes in a helper).

**The v2 wire catalog is closed**, 22 rows, and `find()` is the codec's `method_not_found` source:
```
crates/holler-proto/src/methods.rs:48-49,87-99
/// The complete, closed v2 method catalog (22 rows).
pub const CATALOG: &[Method] = &[ ... ];
pub fn find(name: &str) -> Option<&'static Method> { CATALOG.iter().find(|m| m.name == name) }
```
`holler_proto::vocab::SessionName` (`vocab.rs:70-98`, re-exported at `lib.rs:71-73`) is the ADR 0005 name grammar; `holler-proto`
declares itself async-free (`lib.rs:28-30`). Time in this codebase is `holler_proto::clock::now_millis() -> i64`
(`clock.rs:35`). Timestamps elsewhere are mixed (RFC 3339 in `holds.json` `since` and in `docs.rs`, u64 seconds in the token
store), so there is no dominant pattern and i64 milliseconds is a choice, recorded in decision 3. Exit code 5
(`INVALID_GRANT_EXIT_CODE`, `hold_cmd.rs:141`) exists beside 4; pane and profile verbs use only 0, 1, 2.

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

## Acceptance criteria

All must be observable by command. Names below are the test names T should author (RED first).

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
   `launch`, `relaunch`, `close` only; `--take-over` on `profile apply` only. **Only these shared flags are declared by #637**
   (decision 10): the verb-specific positionals and flags of the sibling issues are not, because each verb's clap struct lives
   in that verb's own file and its owning story adds them (so `pane get hj-c1r1` is a usage error, exit 2, until #643 lands;
   the stubs take what #637 declares). The values stay strings at clap time; the shared
   `SpecFlags::validate()` in `pane/args.rs` turns them into typed values: a bad `--grid` is a refusal (exit 1, code
   `grid-ambiguous` or `grid-out-of-range`), a bad `--env` is a refusal (exit 1, code `env-name-invalid`), a malformed
   `--command-json` or `--check-json` that is not an array of strings is `command-not-argv`. Tests call `validate()` directly.
6. `GridPos` parser table test (`grid_parse_table`): `r2c1`, `c1r2`, `2,1` each give `{row:2,col:1}`; upper case and surrounding
   ASCII whitespace are accepted, interior whitespace is refused (decision 3 below); a bare pair is never read as col,row;
   refusals with stable codes: `r2`, `c1`, `r2c1c3`, `2,1,3`, `21`, `c1r2c3`, and the empty string give `grid-ambiguous`; `r0c1`,
   `c0r1`, `0,1`, `r2c0`, and values above `u16::MAX` give `grid-out-of-range`; `parse(format(x)) == x` for every cell of a
   12 x 12 grid; serde writes `{"row":2,"col":1,"pos":"r2c1"}` with `row`, `col`, `pos` in that order and reads the same back
   (a `pos` that disagrees with `row`/`col` is refused).
7. `Argv` round-trips through serde as a JSON array of strings; a bare JSON string where an `Argv` is expected is refused with
   the code `command-not-argv` (`argv_bare_string_refused`). `EnvVarName` refuses a string containing `=`, an empty string and
   whitespace (`env-name-invalid`); a value cannot be represented in any `ProfileSpec`.
8. Serde round-trip tests: `Pane` (full), `Pane` without `profile` (loads as `None`), `Profile`, `ProfileSpec`. CAS: a `cas_put`
   with a stale `expected_generation` returns `PaneError::Conflict` whose `code()` is `generation-conflict` (test against a trivial
   in-test `PaneStore`).
9. Error codes: `PaneError` and `ALL_CODES` in `holler-pane/src/error.rs` hold every code in decision 2, each a `pub const`
   kebab-case string; a test (`error_codes_unique_and_kebab`) asserts uniqueness and that each matches `^[a-z]+(-[a-z]+)*$`
   using the one validator `error::is_valid_code`; `output::ErrorCode::new` calls that same validator and rejects anything
   else (holler-cli has no second validator); `ErrorCode::from(&PaneError)` is infallible, including for the open `Refused`
   variant. A test covers every variant's `code()` and `Display`.
10. The ports compile against a trivial in-test implementation: `PaneStore`, `ProfileStore`, `ProfileScope`, `HerdrPort` (incl.
    `version()`), `HostPort`, `HarnessPort`, `Prober`; `run_probe` and `ProbeResult` exist with the fixed signature. One test file
    implements every trait (each `Send + Sync`) and calls `ProfileScope::resolve` with and without a pane and `edit_spec` with and without a profile, with a trait-object `act`, so a later
    signature change breaks it (`ports_compile_in_test_impl`). `PaneName`, `ProfileName` and `ProfileName::slug()` have
    table tests (`pane_name_grammar`, `profile_name_slug`: leading and trailing separators are trimmed, a name with no alphanumerics is
    refused when the name is parsed). **Seam test (provisional signatures, decision 7):** one in-process test per verb binary
    (`pane_verbs`, `profile_verbs`) calls a stub's `run` with captured out and err writers in text and JSON mode and asserts the
    routing (text: refusal on err, nothing on out; JSON: one envelope on out, nothing on err), through the shared
    `tests/verb_harness/mod.rs` (Tests section).
11. Hub: a `pane/list` (and one `profile/list`) request over the control socket reaches the stub and returns a JSON-RPC **result**
    that parses back into `holler_pane::PaneReply` as `{ok:false, data:null, error:{code:"not-implemented", message}}` (decision 8;
    no `schema_version`, not the CLI's `output::Envelope`), not MethodNotFound. The test lives in
    `crates/holler-hub/tests/pane_dispatch_test.rs` and drives `handle_control_conn` over `tokio::net::UnixStream::pair()`
    (`Registry::new`, `Roster::with_system_clock`, `Lockout::new` are `pub`); it does not spawn the `holler` binary and does not copy
    `support::Hub`. An unknown `control/x` and an unknown `foo/bar` still return MethodNotFound; `PaneState::load` and
    `ProfileState::load` return empty state; `check_membership` returns `Ok(())` for any pane. `holler-hub` links `holler-pane`,
    and `cargo machete` is clean.
12. `pane_methods_not_in_wire_catalog`: `methods::find("pane/list")` and `find("profile/list")` are `None`, `CATALOG.len()` is
    still 22, and `PANE_METHODS` / `PROFILE_METHODS` list exactly the names in decision 8. No golden file changes.
13. ADR 0003 carries the new rows (decision 6): **one row per verb, in the bare form #637 declares**, so a verb story edits only
    its own row (and its own `cli-surface.txt` line); **rows are grouped by owning story with a separator** (a `# #NNN` comment
    line in the fixture, a blank line in ADR 0003's text block), because adjacent-line edits conflict on rebase. `cargo test -p holler-cli --test docs_cli_test` and `cli_surface_test` pass
    (the first alternative of each `a|b` in an ADR row must parse). Every new leaf verb and every shared flag (the spec flags,
    `--profile`, `--spec-only`, `--take-over`, `--format`, `--pane`) appears on at least one line of
    `crates/holler-cli/tests/fixtures/cli-surface.txt`, plus the regression lines `say io/alpha --parts-file f` and
    `interrupt io/alpha`.
14. `git diff --name-only origin/main...HEAD` lists only paths in the Blast radius below (grep-able check for S).

## Files

Production (new unless noted):
- `crates/holler-pane/` (`Cargo.toml`, `src/lib.rs`, `error.rs`, `pane.rs`, `generation.rs`, `ports.rs`, `grid.rs`, `profile.rs`,
  `argv.rs`, `probe.rs`, and the empty declared stubs `profile_snapshot.rs`, `profile_diff.rs`, `tx_apply.rs`, `tx_launch.rs`,
  `tx_switch.rs`, `reconcile.rs`, `findings.rs`, `import.rs`); `lib.rs` declares every module so no later story edits it.
  Dependencies: `serde`, `serde_json` (consumed by `PaneReply.data`), `holler-proto` (path). No async runtime.
  `reply.rs` (new): `PaneReply` and the shared `pane/*` / `profile/*` params structs (decision 8).
- `crates/holler-adapter-herdr/`, `holler-adapter-host/`, `holler-adapter-opencode/`, `holler-pane-testkit/`: `Cargo.toml` +
  `src/lib.rs` (a crate doc comment only; the testkit may re-export nothing yet).
- `crates/holler-proto/src/methods.rs` (edit): `PANE_METHODS`, `PROFILE_METHODS` (name lists), `is_pane_method()`,
  `is_profile_method()`; outside `CATALOG`. `vocab.rs`: **no edit** (see decision 2 and 8; recorded in decisions.md).
- `crates/holler-cli/src/cli.rs` (edit, small): `Pane(PaneCmd)` and `Profile(ProfileCmd)` variants, global `--format`, the
  `--pane`/`--profile` fields and accessors on `Say`/`Interrupt`/`Answer`, `--profile` on `Roster`; update the namespace comment at
  `cli.rs:7-9`. New `crates/holler-cli/src/prompt_target.rs` (the `--pane`/`--profile` accessors for say/interrupt/answer; keeps `cli.rs`
  near 740 lines). New `crates/holler-cli/src/pane/mod.rs` (`PaneCmd`, the dispatch) + `args.rs` (**only** the shared flag groups:
  `SpecFlags` with `validate()`, `ProfileOpt`, `SpecOnly`; defined once) + one stub per verb, each holding **its own clap `Args` struct** and
  `run` (`list.rs get.rs watch.rs launch.rs relaunch.rs switch.rs reset.rs park.rs unpark.rs close.rs doctor.rs import.rs`) + `wiring.rs` (stub: `pub fn ports() -> OwnedPorts` whose every port returns `PaneError::NotImplemented`; #649 replaces
  the body only) + `profile_scope.rs`; new `crates/holler-cli/src/profile/mod.rs` (`ProfileCmd`; no `args.rs`) + `create.rs delete.rs list.rs show.rs
  apply.rs rename.rs export.rs import.rs` (each with its own `Args` struct; `--take-over` lives in `apply.rs`); new `crates/holler-cli/src/output.rs`; `lib.rs` (edit): module declarations.
- `crates/holler-cli/src/main.rs` (edit): see decision 1.
- `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs` (edit, accessor change and the fail-closed guard only).
- `crates/holler-hub/src/lib.rs` (declarations: `panes`, `profile`, `pane_wiring`), `src/panes/mod.rs` (stub `dispatch` and the
  `PaneState` handle), `src/pane_dispatch.rs` (the forwarding helper), `src/pane_wiring.rs` (empty), `src/profile/rename.rs` (empty module), `src/profile/mod.rs` (stub `dispatch`, the `ProfileState` handle, and
  `check_membership`), `src/control_server.rs` and `src/serve.rs` (forwarding and handle plumbing only), `Cargo.toml` (see decision 4).
- `docs/adr/ADR-0003.md` (edit): decision 6. `CHANGELOG.md`: an `## [Unreleased]` entry linking #637.
Tests: `crates/holler-pane/tests/*.rs` (default autotests there), `crates/holler-cli/tests/pane_verbs/main.rs` and
`profile_verbs/main.rs` (one `[[test]]` each) **plus the shared in-process harness `tests/verb_harness/mod.rs` (included by both binaries via `#[path]`; it builds a `VerbCtx` over given ports, captures out and err, and runs a verb; first caller: the seam test of AC 10, so nine stories do not write near-copies) plus every per-verb file now**: `pane_verbs/<verb>.rs` x12 and
`profile_verbs/<verb>.rs` x8, each holding that verb's stub case and declared in `main.rs`, so a later story edits only its own
file and never `main.rs` or the manifest. Placeholder
`pane_integration/main.rs` (#649) and `profile_apply_scenario/main.rs` (#667) `[[test]]` targets hosted by `holler-cli` (there is no
root package), `crates/holler-cli/tests/fixtures/cli-surface.txt` (edit), `crates/holler-hub/tests/pane_dispatch_test.rs` (drives `handle_control_conn` over `UnixStream::pair()`, AC 11).
Reuse map (extend, do not duplicate): the clap `Cli` and global-flag pattern in `cli.rs`; the `Query::resolve` pattern for the
`--pane` shapes (its variadic tail, but **without** `trailing_var_arg`); `holler_proto::vocab::SessionName` for pane names (do not copy the grammar); `holler_proto::clock::now_millis`
for time; the `control_hold.rs` precedent of a plain-function module the control dispatcher calls; `print_leaf_result_and_exit`
and the existing stub wording; `tempfile`, `assert_cmd`, `predicates`, `rstest` for tests.

## Decisions already made (MO)

Accepted by the operator, 2026-10-08, in this order.

0. **Verbs run in the CLI process**, against the ports; the hub is the **store** only (`pane/get|list|cas_put|watch`, `profile/*`
   handlers). Adapters (Herdr, tmux/host, OpenCode) are constructed CLI-side in `pane/wiring.rs`. The hub's `pane_wiring.rs` wires
   the store, not adapters.
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
2. **Error codes live in `holler-pane/src/error.rs`**, not in `vocab.rs`. `PaneError` has one variant per domain code, a
   `code() -> &'static str`, and `Display`; `pub const ALL_CODES` lists the closed codes; one validator
   `pub fn is_valid_code(&str) -> bool` (kebab-case, `^[a-z]+(-[a-z]+)*$`) is the only one in the workspace. The closed set is
   every code the epic and sibling issues name, plus the infrastructure failures the port implementers must return:
   - domain: `not-implemented`, `usage`, `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`,
     `generation-conflict` (the CAS conflict), `probe-failed`, `profile-conflict`, `profile-not-found`, `profile-exists`,
     `profile-has-live-panes`, `pane-not-in-profile`, `pane-in-other-profile`, `profile-secret-refused`,
     `herdr-version-unsupported`;
   - infrastructure: `timeout` (`Timeout { op }`), `pane-not-found` and `session-not-found` (`NotFound { what }`),
     `store-corrupt` (`StoreCorrupt { .. }`, the fail-closed read), `unavailable` (`Unavailable { what }`: hub, Herdr socket,
     harness unreachable);
   - `profile-drift`: listed for convenience; it is a reconcile **finding kind** (#647/#665 put it in `findings.rs`), not an
     error a port returns.
   - **open variant** `Refused { code: Cow<'static, str>, message: String }` (`code(&self) -> &str`): an adapter crate or a verb
     returns a code it owns without editing the enum (matches #660's "codes are constants in each verb's own file"), and a reply
     code read off the wire that is not in `ALL_CODES` lands here (an older hub can outlive an upgrade, since hub and CLI are one
     binary installed at different times). It is built only through `PaneError::refused(code, message) -> Result<..>`, which
     calls `is_valid_code`, so `ErrorCode::from(&PaneError)` stays infallible and `ErrorCode`'s validation is enforced, not
     conventional. It is not in `ALL_CODES`. **Parse-back rule:** a closed code maps to its variant (structured fields such as
     `Timeout { op }` and `NotFound { what }` travel in an optional `detail` string in the reply); anything else maps to
     `Refused`.
   The owning story of each closed code is a doc comment on its variant (#644/#663: `probe-failed`, `profile-conflict`,
   `profile-not-found`; #643/#663: `pane-not-in-profile`; #661: `pane-in-other-profile`, `profile-secret-refused`,
   `profile-exists`; #662: `profile-has-live-panes`; #640: `herdr-version-unsupported`; #665: `profile-drift`; #638-#642: the
   infrastructure variants). A later story fills behaviour, never the list. `output::ErrorCode` is a validated kebab-case
   newtype (per #660): `ErrorCode::new(&str) -> Result` calls `error::is_valid_code`, and `From<&PaneError>` is infallible, so
   a verb never has dead error handling for a constant. `vocab.rs` is untouched.
3. **`holler-pane` depends on `serde`, `serde_json` and `holler-proto` (path); no async runtime.** `PaneName` is a newtype over
   `holler_proto::vocab::SessionName` (no second copy of the grammar; serde through `SessionName::parse`, because `SessionName`
   has no serde). `ProfileName` is a display name (spaces allowed, trimmed, non-empty, no control characters, at most 64 chars)
   with its own rule and the one `ProfileName::slug() -> String` in `profile.rs` (lower-case, runs of non-alphanumerics become
   one `-`, leading and trailing `-` trimmed; a name with no alphanumerics is refused when the name is parsed, because the slug is
   persisted and unique-checked); #661 enforces unique name and unique slug with `profile-exists`. Every timestamp in the records (`last_observed.at`,
   `Parked.since`, `Profile.created`/`updated`, log entries) is **milliseconds since the Unix epoch as `i64`**, the type
   `holler_proto::clock::now_millis()` returns; the text layer converts to local time for display. `GridPos` accepts upper and lower
   case and surrounding ASCII whitespace, refuses interior whitespace; zero is `grid-out-of-range`; a missing half, a repeated
   label or a mix such as `r2c1c3` is `grid-ambiguous`; bounds are `u16`.
4. **#637 owns the hub plumbing**, in plain functions (no function-pointer registry): in `dispatch_control` one new arm
   `Some(m) if is_pane_method(m) || is_profile_method(m)` calling a helper (to keep cognitive complexity under the limit) that
   forwards to `crate::panes::dispatch` or `crate::profile::dispatch`. Pinned signatures (the precedent is
   `Holds::load(&HubState)` in `build_shared_state`, `serve.rs:351-377`):
   `PaneState::load(&HubState) -> PaneState` and `ProfileState::load(&HubState) -> ProfileState` (stubs return empty state;
   corrupt-file behaviour stays inside `load`, owned by #639 and #661); both are built in `build_shared_state` next to
   `Holds::load`, carried in `SharedState` as **`Arc<PaneState>` and `Arc<ProfileState>`** (like `Arc<Roster>` and `Arc<Lockout>`,
   `serve.rs:346-348`; the types do not derive `Clone`, so a later inline-state change cannot fork a copy per connection and
   defeat the CAS), bundled into one parameter so `accept_loop` and `handle_control_conn` each grow by one, and passed through
   `handle_control_conn` and `dispatch_control`; the forwarding helper lives in the new `crates/holler-hub/src/pane_dispatch.rs`;
   `panes::dispatch(method, &cid, &obj, &PaneState, &ProfileState) -> String` and
   `profile::dispatch(method, &cid, &obj, &ProfileState, &PaneState) -> String` (each receives both handles, so a membership
   check cannot fail open and `profile/rename` can reach the pane store). Each is a `pub async fn` stub answering the
   not-implemented reply (decision 8) as a JSON-RPC **result** (the closed error-code table is untouched). `check_membership(&holler_pane::Pane,
   &ProfileState) -> Result<(), PaneError>` is a plain function returning `Ok(())`; #639's `cas_put` calls it, #661 fills it. The
   `holler-pane` dependency in `holler-hub/Cargo.toml` is consumed by it. `crates/holler-hub/src/profile/rename.rs` is pre-created as an empty module declared in `profile/mod.rs` (#665 fills it and edits no
   #661 file). `holler-pane-testkit` is pre-added as a dev-dependency of
   `holler-hub` and `holler-cli`, each with a one-line consumer test (`testkit_links`), so #638/#639/#661 add no manifest line.
   Later stories declare any new dependency crate-locally with the `# for <consumer>` marker.
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
6. **ADR 0003 is edited by #637**: rows for `holler pane ...` and `holler profile ...`, `--format` on the global-flags line,
   `--pane`/`--profile` on the say/interrupt/answer/roster rows, the amended "only top-level verbs" sentence, citing epic #633 and
   ADR-0021 (#634); `cli.rs:7-9` matches. ADR-0021 stays #634's. Layout: rows grouped by owning story with a separator (AC 13).
   One added sentence says the `pane` and `profile` rows list only the shared flags and each owning story (epic #633) adds its
   positionals and flags. Line 62 ("`--json` prints exactly one JSON object") is amended to cover the envelope and `pane watch`'s
   NDJSON, citing #633 and ADR-0021.
7. **Contracts** (HerdrPort and HarnessPort are **provisional** until spikes #636 and #635 report, under the epic's amend-first
   rule). **Provisional until compiled:** the signatures below for `Ports`, `VerbCtx`, `Sink`, `output.rs`, `ProfileScope` and
   `Prober` are the brief's best design, not a proof. F makes them compile and route together with the seam test (AC 10) and may
   adjust a signature to do so, recording each change in decisions.md; they freeze when #637 merges, after which a change goes
   through amend-first. A reviewer reading this brief should not treat them as verified.
   - Every port is synchronous and blocking, declared `Send + Sync`; each trait carries the doc rule "call from
     `spawn_blocking` (or a thread) in async code; return within I5's bound (default 10 s) or with `PaneError::Timeout`".
   - `PaneStore { get(&self, &PaneName) -> Result<Option<Pane>, PaneError>; list(&self) -> Result<Vec<Pane>, PaneError>;
     cas_put(&self, &Pane, expected_generation: u64) -> Result<Pane, PaneError>; watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> }`
     with `Cursor(u64)` a store-wide change sequence number and `Watch<T> = Box<dyn Iterator<Item = Result<T, PaneError>> + Send>`
     (one watch shape for both stores).
   - `ProfileStore { get; list; cas_put(&Profile, expected_generation); delete(&ProfileName, expected_generation); watch(Cursor);
     log(&ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>; rename(&ProfileName, &ProfileName, expected_generation)
     -> Result<Profile, PaneError> }` (`rename` is PROPOSED, with #665; the stub returns `NotImplemented`).
   - `ProfileScope: Send + Sync { resolve(&self, profile: &ProfileName, pane: Option<&PaneName>) -> Result<ResolvedScope, PaneError>;
     edit_spec(&self, profile: Option<&ProfileName>, &PaneName, edit: &SpecEdit, act: &mut dyn FnMut() -> Result<(), PaneError>) -> Result<Option<Profile>, PaneError> }`.
     `resolve` with no pane means every pane of the profile (#643, #646, #647, #648 need that form); a named pane outside the
     profile is `pane-not-in-profile`, a missing profile `profile-not-found`. `edit_spec` with `None` runs only `act` and touches
     no profile ("no profile is touched without `--profile`", #663's and #638's conformance case). `ResolvedScope { profile:
     Profile, panes: Vec<Pane> }` and `SpecEdit` (set or remove the pane's entry) are minimal types F defines in `profile.rs`;
     `act` is a trait object so `Ports` can hold `&dyn ProfileScope`.
   - `HerdrPort { ensure_pane(&self, &HerdrSpec) -> Result<HerdrPane, _>; send_text(&self, &PaneId, &str); send_keys(&self, &PaneId, &[Key]);
     read(&self, &PaneId, max_lines: usize) -> Result<String, _>; close(&self, &PaneId); snapshot(&self) -> Result<HerdrSnapshot, _>; version(&self) -> Result<String, _> }`;
     `HostPort { ensure_session; run(&self, &PaneName, &Argv) ; stop_owned; ps }`; `HarnessPort { serve; health; create_session; list_sessions;
     abort; attach_tui; select_session; shown_session }`, each taking and returning the minimal data types F defines in `ports.rs`.
   - `trait Prober: Send + Sync { fn run_probe(&self, &Argv, &[String], Duration) -> ProbeResult }`, a `SystemProber` that calls the free
     `run_probe` stub (signature fixed by the epic; #663 builds the body), so #638's fake and AC 10 have something to swap.
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
8. **Methods and replies.** `PANE_METHODS = [pane/get, pane/list, pane/cas_put, pane/watch]`; `PROFILE_METHODS = [profile/get, profile/list,
   profile/cas_put, profile/delete, profile/watch, profile/log, profile/rename]` (`profile/rename` is the PROPOSED one). They are hub
   control methods, outside `CATALOG`, so the v2 wire protocol is unchanged and a body connection still gets `method_not_found`.
   The params structs and the reply type live in `holler-pane` (`reply.rs`), shared by #639/#661 (server) and #649 (client);
   `holler-proto` holds names only, because it cannot depend on `holler-pane`. **Reply encoding:** a JSON-RPC **result**
   `PaneReply { ok: bool, data: Option<serde_json::Value>, error: Option<{code, message}> }` with **no** `schema_version`,
   built from a `PaneError` and parsed back into one; `output::Envelope` stays CLI-only and the CLI maps a `PaneReply` error
   into it. `*/watch` are **long-poll**: the frozen `dispatch(...) -> String` returns one `{events, cursor}` reply per request. This deviates from the existing control methods (JSON-RPC errors with a `data.reason` sub-code,
   `control_server.rs:316,697-713`) because the domain codes do not fit the closed `Code` table; recorded in decisions.md.
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
    other users import `ProfileOpt` from there; no profile verb takes `--profile`, so `profile/args.rs` is not created). #637 declares each verb with what it
    is already known to need (shared groups only), and **does not** guess the positionals or verb-specific flags of the sibling
    stories (`pane get PANE`, `pane switch PANE SESSION`, `pane park` reason and release condition, `pane doctor [--fix]`,
    `pane import --from`, `profile create/delete/show NAME`, `profile rename OLD NEW` and so on; `--take-over` on apply **is**
    declared by #637, in `apply.rs`): each
    owning story adds them in its own file, edits its own ADR 0003 row and its own `cli-surface.txt` line, and no frozen file.
    This follows the epic's "one verb, one file, one owning story" rule and is the reviewer's accepted alternative; recorded in
    decisions.md. `--grid` stays a string at clap time and is typed by `SpecFlags::validate()` (the issue says `cli.rs` declares it; the brief's choice
    gives a stable error code instead of a clap exit 2; recorded in decisions.md). The accessor errors reuse the root-exported
    `cli::Usage` (`Query::resolve`'s type; its `new` becomes `pub(crate)`). Path-qualify `output::Envelope`, never the `holler_proto::Envelope` root re-export. `src/**/mod.rs` is new to
    this codebase and kept because each module root sits inside its owner's blast-radius glob; recorded in decisions.md.
    `Pane.hold` is doc-commented as a pane-record state, not a prompt hold; #646's brief must put any prompt refusal derived from
    pane state at `send_prompt` (the stack's choke-point rule), not in the verb.

## Out of scope

Any behaviour behind a stub; any adapter logic; registry persistence; the hub's real handlers (#639, #661); ADR-0021 and
`docs/protocol/v2.md` (#634); `holler roster` rendering (#648: `roster` gains only the declared flags here); implementing `--pane`
routing in say/interrupt/answer (#646: here they only parse and refuse).

## Test plan

RED first (T): the `holler-pane` tests (they cannot compile until the crate exists: that is the RED), the `pane_verbs` /
`profile_verbs` stub tests, the hub dispatch test, the fixture lines (which make `cli_surface_test` fail until the verbs exist),
and the ADR 0003 rows (which make `docs_cli_test` fail until the clap tree matches). Confirm RED by `cargo test --workspace`
failing to build or failing on named tests, and that no RED is a missing `[[test]]` entry. GREEN: F creates the crates and stubs
until the whole workspace is green. Then clippy `-D warnings`, `rustfmt --check --edition 2021` on each new `.rs` file (not the whole tree, AC 1), `cargo machete`, `bash scripts/lint.sh`,
`bash scripts/changelog-check.sh`, `bash scripts/test-hooks.sh`.

## Risks

- `main.rs` is at the clippy line limit: do the extraction first, in its own commit-sized step, then add the arms.
- `control_server.rs` (829) and `serve.rs` (833) are past the 800-line flag: add only the forwarding arm, two handles and one
  parameter; no logic.
- Changing `handle_control_conn`'s signature touches its one caller (`serve.rs:565`); no test calls it directly.
- `dead_code = "deny"`: stubs and handles must be `pub` or used. `cargo machete`: no unused dependency or dev-dependency.
- `Cli::try_parse()` must keep `--help`, `--version`, bare `holler` and unknown subcommand behaviour byte-identical in text mode
  (the existing `cli_invocation_test` covers this).
- A `--pane` form that parses but is ignored fails open; the guard in the three `*_cmd.rs` files is part of the story, not optional.
- Name collisions: `Pane`/`Profile`, `Envelope`, `List`/`Delete` (decision 10).

## Blast-radius amendments (differences from the issue's list, for the operator and S)

Added to the issue's list: `crates/holler-cli/src/main.rs` (decision 1); `crates/holler-hub/src/control_server.rs`,
`serve.rs`, `Cargo.toml` (decision 4); `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs` (decision 5);
`docs/adr/ADR-0003.md` (decision 6); `crates/holler-cli/Cargo.toml` (`[[test]]` targets, the `holler-pane` dependency and the
testkit dev-dependency); `CHANGELOG.md`; `crates/holler-cli/tests/fixtures/cli-surface.txt`; the new test files and placeholder
`[[test]]` targets under `crates/holler-cli/tests/` and `crates/holler-pane/tests/` and `crates/holler-hub/tests/`;
`crates/holler-cli/src/pane/args.rs` (inside `pane/**`, to keep `cli.rs` under the gate); `crates/holler-hub/src/pane_dispatch.rs`, `profile/rename.rs`;
`crates/holler-cli/src/prompt_target.rs`; `crates/holler-pane/src/reply.rs`.
Removed from the issue's list: `crates/holler-proto/src/vocab.rs` (no edit needed, decision 2). Not changed: workspace
`Cargo.toml` (the `crates/*` glob already covers the new crates); `Cargo.lock` updates for them only. The epic's ownership table
(shared hot spots) needs the same additions; the issue and epic edits are drafted separately and not posted by this run. The draft must also cover
what decision 0 contradicts in sibling text: #647 "also callable by the hub on a timer"; #649 "wire the real adapters into the hub's
`pane/*` handlers" and its `tests/pane_integration/**` and #667's `tests/profile_apply_scenario/**` (both now under
`crates/holler-cli/tests/`); the adapter crates as holler-cli dependencies (add `crates/holler-cli/Cargo.toml` to #649's and
#667's radii); I5 and #644 "an operation id for long work" (nothing executes long work once the CLI exits and the hub runs no
adapters: #634 decides where it runs); #648 "`--json` becomes a deprecated alias" (contradicts decision 9) and its `main.rs` line;
#639's hook "in `control.rs`" (the dispatch is `control_server.rs`); and the per-verb-file ruling of decision 10 for #643-#647, #650,
#662, #664, #665 (each adds its positionals, its ADR row and fixture line, **and its own `tests/pane_verbs/<verb>.rs` or
`tests/profile_verbs/<verb>.rs`** in its radius). Also for the draft: #634 documents `pane/*` and `profile/*` as **hub
control-socket methods**, not wire methods (decision 8 keeps them outside the catalog); #648's `main.rs` work is two edits (pass the
explicit-format bit; remove the `roster --profile` refusal from the extracted helper); the hub's `pane_wiring.rs` (#649) has nothing
left to do under decisions 0 and 4 (drop it from #649's radius or say so); **`send_prompt` has no path to pane state**: if #646 needs
a prompt refusal derived from pane state it follows the holds precedent (`Registry::with_holds`, `live.rs:453`; the Registry carries
a pane-state handle that the `send_prompt` gate checks), so `live.rs` and `circuit/dispatch.rs` join #646's radius, or #646 records
an explicit CLI-side exception; and the epic-wide "`cargo fmt`" rule is read as AC 1 reads it (new files clean, existing files
not reformatted).
