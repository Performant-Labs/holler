# Brief: #637 pane-control skeleton (`holler-pane` crate, verb and profile stubs, output envelope)

Repo: Performant-Labs/holler. Issue: #637 (story of epic #633). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-637-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633 is
fixed and ADR-0021 (#634) ratifies it later; this story builds against the epic text and does not wait for #634.

The issue body (#637) and the epic's "The contract" section are the spec. This brief adds the survey findings, the
decisions the issue leaves open, and the existing-test constraints the issue does not mention. Where this brief and the
issue differ, the brief names the difference under "Blast-radius amendments".

## Problem

Epic #633 splits into 13 stories that run in parallel in wave 3. They can only run in parallel if every shared file
(workspace `Cargo.toml`, `cli.rs`, `lib.rs`, `methods.rs`, `vocab.rs`) and every shared type, trait and signature is created
once, up front, by this story. This story creates no behaviour: types with serde, traits, the grid parser, the `Argv`
and `EnvVarName` guards, one stub file per verb, empty crates, and a working-but-minimal output envelope. Nothing may be
left for a later story to add to a shared file.

## Evidence (verbatim, as of `27da2da`)

The workspace takes every crate under `crates/` and each crate opts in to the deny lints:
```
Cargo.toml:1-3
[workspace]
members = ["crates/*"]
resolver = "2"
```
```
Cargo.toml (workspace.lints)
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
cognitive_complexity = "deny"
too_many_lines = "deny"
struct_excessive_bools = "deny"
large_enum_variant = "warn"

[workspace.lints.rust]
dead_code = "deny"
```
So new crates are picked up by the `crates/*` glob (no `members` edit is needed), must carry `[lints] workspace = true`, and
must use `version.workspace = true` etc. Dependencies must be declared only if consumed (`cargo machete` fails CI;
`crates/holler-hub/Cargo.toml`: "Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI otherwise)").
Every dependency with a feature list needs a `# for <consumer>` marker (lint check 5, `scripts/lint.sh`). File-size gate
(lint check 4): **warn at 600 lines, fail at 900**.

The global flags today (`--json` exists and is global):
```
crates/holler-cli/src/cli.rs:57-63
    /// Print machine-readable output.
    ///
    /// Global (issue #147/#155): reachable after any leaf, including
    /// `hub token mint --label x --json` — which the per-struct copies this
    /// replaced could not parse (the flag sat on `Token`, before the leaf).
    #[arg(long, global = true)]
    pub json: bool,
```
The top-level command enum, which gains `Pane(Pane)` and `Profile(Profile)`:
```
crates/holler-cli/src/cli.rs:69-89
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Operate a hub (serve, tokens, status, caps, support, query).
    Hub(Hub),
    /// Operate a body (join, run, detach, status, caps, support, query, attach).
    Body(Body),
    /// List the roster. (hub-only, top-level daily verb)
    Roster(Roster),
    Say(Say), Interrupt(Interrupt), Answer(Answer), Wait(Wait), Hold(Hold), Release(Release),
}
```
(`cli.rs` is 701 lines today. It must stay under 900, so the large clap structs for pane and profile live in new files, see Files.)

The CLI surface is pinned by a fixture, in both directions:
```
crates/holler-cli/tests/cli_surface_test.rs:1-15 (header)
//! `tests/fixtures/cli-surface.txt` is the normative, machine-readable form
//! of ADR 0003's CLI table. Three properties are pinned:
//! 1. every line in it parses (`Cli::try_parse_from`),
//! 2. every line in `cli-surface.pending.txt` does **not** parse yet ...
//! 3. the set of leaf verbs in the fixture equals the set clap knows — a verb
//!    added to the tree without a fixture line (or vice versa) fails here.
```
So every new leaf verb (`pane list` ... `pane import`, `profile create` ... `profile import`) needs a line in
`crates/holler-cli/tests/fixtures/cli-surface.txt`. That file is not in the issue's Blast radius (see amendments).

The v2 wire catalog is closed, and documents itself as 22 rows:
```
crates/holler-proto/src/methods.rs:48-49
/// The complete, closed v2 method catalog (22 rows).
#[rustfmt::skip]
pub const CATALOG: &[Method] = &[
```
```
crates/holler-proto/src/methods.rs:87-99
pub fn find(name: &str) -> Option<&'static Method> { CATALOG.iter().find(|m| m.name == name) }
```
`find()` is the codec's `method_not_found` source, so a `pane/*` name added to `CATALOG` would become a legal body-to-hub
wire method. The pane and profile methods are hub **control** methods (the hub's pane registry is reached over the
existing control channel, like `control/*`), so they must not enter `CATALOG`.

`vocab.rs` holds the name and feature vocabulary (`FEATURES`, `HARNESS_IDS`, `crates/holler-proto/src/vocab.rs:258-275`).
Hub modules are declared in `crates/holler-hub/src/lib.rs:8-27` (`pub mod circuit; ... pub mod token;`).
`crates/holler-cli/src/lib.rs:7-20` declares the CLI modules and re-exports the clap types.

## Acceptance criteria

All must be observable by command. Names below are the test names T should author (RED first).

1. `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and
   `cargo test --workspace` pass; `bash scripts/lint.sh` passes (no file over 900 lines; no `#[allow]` without an issue link).
2. `holler pane list` prints `not implemented` and exits **1** (not a panic). `holler profile list` and every other stub
   verb (`pane`: list get watch launch relaunch switch reset park unpark close doctor import; `profile`: create delete list
   show apply rename export import) does the same. `holler pane list --format=json` prints exactly one valid envelope
   `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"<one line>"}}` on stdout and
   nothing else on stdout; exit 1.
3. Usage errors exit 2: `holler pane launch X --spec-only` without `--profile` (test `spec_only_requires_profile`), and a
   bad `--format` value.
4. `holler say --pane X --profile P`, `interrupt` and `answer` with the same flags parse (the existing positional/`--session`
   forms keep parsing); every spec flag parses on `pane launch` and `pane relaunch`:
   `--project --workspace --grid --model --effort --role --env (repeatable) --ctx-soft --ctx-hard --port-policy
   --command-arg (repeatable) --command-json --check-arg (repeatable) --check-json --expect (repeatable)`.
   `--profile` parses on every `pane` verb except `import`, and on `roster`. `--spec-only` is accepted on `launch`,
   `relaunch`, `close` only. `--take-over` on `profile apply` only.
5. `GridPos` parser table test (`grid_parse_table`): `r2c1`, `c1r2`, `2,1` each give `{row:2,col:1}`, with upper case
   and surrounding spaces accepted (the ADR has not ruled otherwise; the test pins what is accepted and what is refused so
   #640 and #644 inherit one behaviour); a bare pair is never read as col,row; refusals with stable codes:
   `r2`, `c1`, `r2c1c3`, `2,1,3`, `21`, `r0c1`, `c0r1`, `0,1`, `r2c0`, values above `u16::MAX`, and the empty string give
   `grid-ambiguous` or `grid-out-of-range` as the table says; `parse(format(x)) == x` for every cell of a 12 x 12 grid;
   serde writes `{"row":2,"col":1,"pos":"r2c1"}` with `row` before `col` before `pos` and reads the same back (a `pos`
   that disagrees with `row`/`col` is refused).
6. `Argv` round-trips through serde as a JSON array of strings; a bare JSON string where an `Argv` is expected is refused
   with the code `command-not-argv` (test `argv_bare_string_refused`). `EnvVarName` refuses a string containing `=`
   (and empty, and whitespace); a value cannot be represented in any `ProfileSpec`.
7. Serde round-trip tests: `Pane` (full), `Pane` without `profile` (a record without the field loads as `None`),
   `Profile`, `ProfileSpec`. CAS: a `cas_put` with a stale `expected_generation` returns the typed `Conflict` error (test
   against a trivial in-test `PaneStore`).
8. The four ports (`PaneStore`, `HerdrPort` incl. `version()`, `HostPort`, `HarnessPort`), `ProfileStore`, `ProfileScope`,
   and `run_probe`/`ProbeResult` each compile against a trivial in-test implementation (one test file that implements every
   trait, so a later signature change breaks it).
9. `crates/holler-cli/tests/fixtures/cli-surface.txt` has a line for every new leaf verb and the existing
   `cli_surface_test` still passes in both directions.
10. `git diff --name-only origin/main...HEAD` lists only paths in the Blast radius below (grep-able check for S).

## Files

Production (new unless noted):
- `crates/holler-pane/` (`Cargo.toml`, `src/lib.rs`, `error.rs`, `pane.rs`, `generation.rs`, `ports.rs`, `grid.rs`,
  `profile.rs`, `argv.rs`, `probe.rs`, and the empty declared stubs `profile_snapshot.rs`, `profile_diff.rs`, `tx_apply.rs`,
  `tx_launch.rs`, `tx_switch.rs`, `reconcile.rs`, `findings.rs`, `import.rs`); `lib.rs` declares every module so no later
  story edits it.
- `crates/holler-adapter-herdr/`, `holler-adapter-host/`, `holler-adapter-opencode/`, `holler-pane-testkit/`: `Cargo.toml` +
  `src/lib.rs` only (a crate doc comment; nothing else).
- `crates/holler-proto/src/methods.rs` (edit): `PANE_METHODS` and `PROFILE_METHODS` consts (names + a params type name per
  method), **outside** `CATALOG`; `crates/holler-proto/src/vocab.rs` (edit): the string constants for the stable error codes
  the skeleton defines (`not-implemented`, `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`).
- `crates/holler-cli/src/cli.rs` (edit, small): `Pane(Pane)`, `Profile(Profile)` variants, global `--format`; new
  `crates/holler-cli/src/pane/mod.rs` + `args.rs` (clap structs, incl. the spec flags declared once) + one stub per verb
  (`list.rs get.rs watch.rs launch.rs relaunch.rs switch.rs reset.rs park.rs unpark.rs close.rs doctor.rs import.rs`) +
  `wiring.rs` + `profile_scope.rs`; new `crates/holler-cli/src/profile/mod.rs` + `args.rs` + `create.rs delete.rs list.rs
  show.rs apply.rs rename.rs export.rs import.rs`; new `crates/holler-cli/src/output.rs`; `lib.rs` (edit): module
  declarations and re-exports.
- `crates/holler-hub/src/lib.rs` (edit, declarations only: `panes`, `profile`, `pane_wiring`), `src/panes/mod.rs`,
  `src/pane_wiring.rs` (empty stubs), `src/profile/mod.rs` (stub plus the registration hook for `profile/*` handlers and the
  membership-check hook on the `pane/cas_put` path, each a function-pointer/trait seam with a no-op default).
Tests: `crates/holler-pane/tests/*.rs` (grid, argv/env, serde round-trips, CAS, trait-compile), `crates/holler-cli/tests/
pane_profile_stub_test.rs` (acceptance 2-4), `crates/holler-cli/tests/fixtures/cli-surface.txt` (edit).
Reuse map (extend, do not duplicate): clap `Cli`/global-flag pattern in `cli.rs`; the existing `--json` global; `tempfile`,
`assert_cmd`, `predicates`, `rstest` workspace dependencies for tests; `holler_proto::vocab` name grammar for pane names
(a pane name is a `SessionName`-shaped segment, e.g. `hj-c1r1`); the exit-code convention (`process::exit` only in
`main.rs`, lint check 2).

## Decisions already made (MO)

1. **New method names stay out of `CATALOG`.** `pane/*` and `profile/*` are hub control methods; `CATALOG` stays 22 rows so
   the v2 wire protocol does not change and `find()` still rejects them as `method_not_found` on a body connection. A test
   (`pane_methods_not_in_wire_catalog`) pins this.
2. **`--json` and `--format`.** The existing global `--json` stays for every existing verb. `--format=text|json`
   is added as a global option (default `text`). On `pane`/`profile`/roster verbs the effective format is `json` if either
   `--json` or `--format=json` is given; `--json --format=text` is a usage error (exit 2). The single resolver is in
   `output.rs`. (A's up-front review may challenge this; the alternative, replacing `--json`, is a breaking change to
   existing verbs and is out of scope.)
3. **`GridPos` accepts** upper and lower case and surrounding ASCII whitespace; it refuses interior whitespace. Zero is
   `grid-out-of-range`; a labelled form with a missing half, a repeated label, or a mix such as `r2c1c3` is `grid-ambiguous`.
4. **Exit codes:** 0 ok, 1 refused/failed (including `not-implemented`), 2 usage (the `output.rs` contract).
5. **`Pane` name** is validated as a `SessionName` (ADR 0005 grammar), reusing the existing type; `GridPos` bounds are `u16`.
6. **Trait style:** the ports are synchronous traits returning `Result<_, PaneError>` (no async runtime in `holler-pane`);
   adapters that need async own their runtime. `holler-pane` depends only on `serde`, `serde_json` (declare only these).
7. **Error taxonomy** is one `PaneError` enum with a stable kebab-case `code()`; `Conflict { expected, actual }` is the CAS
   error. Every code string lives once, in `holler-proto::vocab`.

## Out of scope

Any behaviour behind a stub; any adapter logic; registry persistence; the hub handlers (#639, #661); editing ADRs or
`docs/protocol/v2.md` (ADR-0021 is #634); `holler roster` rendering (#648). `roster` gains only the declared `--profile` and
`--format` flags, and its output is untouched.

## Test plan

RED first (T): write the `holler-pane` tests (they cannot compile until the crate exists: that is the RED), then the CLI
stub test and the fixture lines (the fixture lines make `cli_surface_test` fail until the verbs exist). Confirm RED by
`cargo test --workspace` failing to build or failing on named tests. GREEN: F creates the crates and stubs until the
whole workspace is green. Then clippy `-D warnings`, `cargo fmt --check`, `bash scripts/lint.sh`, `bash scripts/test-hooks.sh`.

## Risks

- `cli.rs` is 701 lines and the file-size gate fails at 900: keep the new clap structs in `pane/args.rs` and
  `profile/args.rs` and add only the two variants and `--format` to `cli.rs`.
- `dead_code = "deny"`: a private helper with no caller fails the build; stubs must be `pub` or used.
- `cargo machete`: no unused dependency in the new crates; `Cargo.lock` changes only by the new workspace crates.
- Existing `--json` global and the new `--format` can disagree (decision 2).
- `clap` `global = true` flags on a derive tree can clash with a same-named per-verb option; run the full existing CLI tests.
- The empty adapter crates must still compile with `[lints] workspace = true` (no unused items).

## Blast-radius amendments (differences from the issue's list, for the operator and S)

- Adds `crates/holler-cli/tests/fixtures/cli-surface.txt` and the new test files under `crates/holler-cli/tests/` and
  `crates/holler-pane/tests/`; without them `cli_surface_test` (leaf set equality) fails. Tests are an expected part of every
  story's footprint, but the fixture is a shared file and is named here so S does not flag it.
- Adds `crates/holler-cli/src/pane/args.rs` and `profile/args.rs` (inside `pane/**` and `profile/**`) to keep `cli.rs` under
  the 900-line gate.
- Adds `crates/holler-cli/Cargo.toml`: `holler-cli` sets `autotests = false`, so every new `crates/holler-cli/tests/*.rs`
  file (here `pane_profile_stub_test.rs`) must be declared as a `[[test]]` or it silently never builds (a RED that says "no
  test target named X" is this mistake, not a real RED). The same edit adds the `holler-pane` path dependency the CLI uses.
- Adds `CHANGELOG.md`: an `## [Unreleased]` entry (Enhancements) linking #637, required by `scripts/changelog-check.sh`.
- No `Cargo.toml` (workspace) edit is needed: `members = ["crates/*"]` already globs the new crates; `Cargo.lock` is updated.
