# Handoff-T-red: #670 the pane/profile CLI skeleton (skeleton slice c) - Phase 4 (author tests, RED)

**Date:** 2026-10-09
**Branch:** issue-670-implementation (at 2b8368d, tests uncommitted)
**Brief / wireframe reviewed:** docs/handoffs/670-brief.md (Revision 1); wireframe N/A (no UI); docs/handoffs/670/handoff-A.md; issue #670; epic #633 (verb-to-story table)

## A precondition

Confirmed: A returned PASS on the plan (Phase 3, handoff-A.md, 13 warns, no blocks). T settled the shape-freezing warns W-1 to W-5 as below.

## Tests authored

Four test targets plus a shared in-process harness, all inside the blast radius. Every target is declared in `crates/holler-cli/Cargo.toml` (`autotests = false`), so none is a missing-`[[test]]` RED. T also added the `holler-pane` dependency and the `holler-pane-testkit` dev-dependency there (the tests import both); F uses `holler-pane` in `src`, so `cargo machete` is clean once it does.

| Target (path) | Tier | What it pins |
|---|---|---|
| `pane_cli_process` (`tests/pane_verbs/process/`) | process: the real `holler` binary, no hub | AC 2, 3, 4 (refusals), 5 (flag matrix), 7 (ADR rows, fixture layout) |
| `pane_verbs` (`tests/pane_verbs/`) | in-process | AC 4 (accessor values), 5 (`validate()`), 6 (seam, output API) |
| `profile_verbs` (`tests/profile_verbs/`) | in-process | AC 6, profile half of the seam; 8 per-verb stub cases |
| `pane_integration`, `profile_apply_scenario` | placeholders (no tests) | declared for #649 and #667 |
| `tests/verb_harness/mod.rs` | harness | `run_verb` over `Wiring` ports and captured writers; `assert_stub_routes` |

Process-level (`pane_cli_process`, 33 tests), in `stub.rs`, `usage.rs`, `flags.rs`, `legacy_verbs.rs`, `docs_rows.rs`:
- `stub_verb_not_implemented`: all 20 verbs, text mode: exit 1, empty stdout, stderr line `error: not implemented (story #NNN)` with the owning story from the epic. `stub_verb_not_implemented_json_is_one_envelope`: the same 20 verbs in the four spellings (`--format=json`, `--format json`, `--json`, the flag before the namespace): exactly one compact line starting `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":`, message one line naming the story.
- Bare `pane` / `profile` exit 2 (a known namespace, not an unknown subcommand); `--help` lists the verbs and exits 0 even with `--json`; `pane list --debug bogus` (and with `--format=json`) exits 3, no stub runs.
- Usage errors: `spec_only_requires_profile` (`launch`, `relaunch`, `close`; text names `--profile`, JSON is a one-line `usage` envelope), `json_conflicts_text` (envelope under pane/profile, plain exit 2 on `roster`), bad `--format` value, `--command-arg`/`--command-json` and `--check-arg`/`--check-json` exclusion, unknown flag under pane/profile in every global-flag spelling (`--debug v`, `--debug=v`, `--log-format`, `--format json` before the namespace), bare namespace with `--json`. `a_legacy_verbs_json_usage_error_is_untouched` pins that `roster`, `hub status`, `say`, `body status` and the "flag value looks like `pane`" traps (`--debug pane roster`, `roster --prefix pane`, `say pane`) keep plain stderr output.
- Flag matrix (`flags.rs`): every spec flag on `launch` and `relaunch`; `--profile` on every pane verb except `import`, on no profile verb; `--spec-only` on `launch`/`relaunch`/`close` only; `--take-over` on `profile apply` only; `--format` global. Each negative first proves the verb exists, so none passes because `pane` is unknown.
- Legacy verbs (`legacy_verbs.rs`): ten `--pane`/`--profile` forms refuse with exit 1 and `error: not implemented (story #646)`, empty stdout, in plain text under `--json`, `--format=json`, `--format json`, `--format=text`; `roster --profile` refuses with `#648`; malformed `--pane` forms are exit 2 and name the missing argument, not the refusal; the SESSION forms are not refused.
- `docs_rows.rs`: ADR 0003 has exactly one bare row per verb (20), rows of different owning stories are never on adjacent lines, the ADR mentions `--format`, `--pane`, `--profile`, `#633`, `ADR-0021`, lists `--format` on the global-flags line and no longer says `say`/`interrupt`/`roster` are the only top-level verbs; the fixture has every new leaf, every shared flag, one `# #NNN` header per owning story with each leaf under its own story, and the two regression lines.

In-process (`pane_verbs`, 53 tests; `profile_verbs`, 9):
- Seam (`main.rs` of each): `pane list` and `profile list` through `pane::run` / `profile::run` over the stub `Wiring` ports: text writes only to `err`, JSON writes exactly one envelope to `out` and nothing to `err`. `stub_wiring_ports_answer_not_implemented`. One stub case per verb file (12 + 8), each naming its owning story.
- `output_api.rs`: `ErrorCode::new` agrees with `holler_pane::error::is_valid_code` on every closed code and a set of malformed ones (no second validator); `ErrorCode::from(&PaneError)`; `emit` ok and error in both formats (routing, exact envelope, compact key order); `emit` exit codes (0 ok, 1 error, 2 for a `usage`-coded error in both formats, settling W-1); `emit_usage_error`; `emit_stream` (one line per item in text, one envelope per line in JSON, exit 1 on an error item with earlier lines kept).
- `spec_flags.rs`: `SpecFlags::validate()` returns `grid-ambiguous`, `grid-out-of-range`, `profile-secret-refused` (`NAME=value`), `env-name-invalid` (blank or whitespace), `command-not-argv` (for both JSON flags), `usage` (not JSON), and the secret never appears in the refusal.
- `target_flags.rs`: `Say`/`Interrupt`/`Answer::resolve()` asserted by value for the SESSION forms (unchanged), the `--pane` forms, `--pane --profile`, flags after the tail, clap's exit 2 on a third positional, and the accessor's `Usage` for missing SESSION/TEXT/CHOICE, `--pane` with a SESSION, and a surplus positional.
- `testkit_links` in `pane_verbs/list.rs` (#643's file) consumes the testkit dev-dependency.

### API pinned by the tests (provisional, as decision 7 allows; F records any change in decisions.md)

- `holler_cli::output`: `Format {Text, Json}`, `Sink { out, err }` (pub fields, `&mut dyn Write`), `VerbCtx { format, ports: Ports, sink }` (pub fields; **`ports` by value and `VerbCtx` in `output.rs`**, settling W-7), `ErrorCode::{new, From<&PaneError>}` (Serialize), `ErrorBody { code, message: String }` with `From<&PaneError>`, `emit`, `emit_stream`, `emit_usage_error`, with the signatures of decision 7.
- `holler_cli::pane::run(&PaneCmd, &mut VerbCtx) -> i32`, `holler_cli::profile::run(&ProfileCmd, &mut VerbCtx) -> i32`; `Command::Pane(PaneCmd)`, `Command::Profile(ProfileCmd)`.
- `holler_cli::pane::wiring::Wiring::connect() -> Result<Wiring, PaneError>` and `Wiring::ports(&self) -> Ports<'_>` (settling W-5: an owning, fallible API; one not-implemented port set, reused by the harness).
- `holler_cli::pane::args::SpecFlags` (`clap::Args`, `validate(&self) -> Result<_, PaneError>`).
- `holler_cli::prompt_target::{PromptTarget::{Session, Pane}(String), PromptArgs { target, arg: Option<String> }}` and `Say`/`Interrupt`/`Answer::resolve(&self) -> Result<PromptArgs, holler_cli::Usage>`. `arg` is TEXT for `say`, the optional redirect text for `interrupt`, CHOICE for `answer`.
- A stub writes exactly `error: not implemented (story #NNN)` to `err` in text mode; the JSON message contains `not implemented (story #NNN)` (the `error: ` prefix is not pinned inside the envelope).
- The JSON usage envelope message is one line with no `error:` prefix and no `Usage:` block (W-2).

### Decisions on A's warns

- W-1: `emit` maps a `usage` code to exit 2 (tested in-process).
- W-2: JSON usage message flattened to one line (tested on the binary).
- W-3: `spec_only_requires_profile` uses `pane launch --spec-only`, never `pane launch X --spec-only`.
- W-4: no subprocess test asserts an empty stderr; they assert the refusal line, and stdout exactly.
- W-5: `Wiring` as above. W-6: `verb_harness` is in-process only, its envelope checks are inline and minimal, and `testkit_links` lives in `pane_verbs/list.rs`. W-12: the tests, fixture and ADR checks use `demo-c1r1`, never `hj-*`.
- Not pinned by T (left to F and O): W-8 (how the raw-argv scan is built; the tests only fix its observable behaviour), W-9 (exhaustive `match` in `main.rs`), W-10 (`mod.rs` layout: tests reach the modules by path only), W-11 (marking #665 rows "proposed"; the fixture carries a comment line for it).

### Fixture lines (T-authored)

`crates/holler-cli/tests/fixtures/cli-surface.txt` gains 82 lines in blocks under `# #643` ... `# #665` (every new leaf, every shared flag, the `--pane` forms, `roster --profile`) and the regression lines `say | io/alpha --parts-file f` and `interrupt | io/alpha` (the second already existed in the file; the new copy sits in the regression block). They make `cli_surface_test` fail until the clap tree has the verbs. ADR 0003 is not edited by T (documentation is F's file in the brief); `docs_rows.rs` pins its rows.

## RED confirmation

Commands run in the worktree:

```
cargo test -p holler-cli --test pane_verbs --test profile_verbs
cargo test -p holler-cli --test pane_cli_process
cargo test -p holler-cli --test cli_surface_test
```

1. `pane_verbs` and `profile_verbs` do not build, and the cause is the missing feature, not a typo or a missing target: `E0432 unresolved import holler_cli::output`, `E0432 unresolved import holler_cli::prompt_target`, `E0432 unresolved imports holler_cli::pane, holler_cli::profile`, `E0599 no variant named Pane/Profile found for enum Command`, `E0599 no method named resolve found for struct Say/Interrupt/Answer`. The brief names "failing to build" as an accepted RED for the in-process API; `holler-pane` and the testkit resolve, the targets are declared, and nothing else is missing.
2. `pane_cli_process`: 33 tests, 28 fail, 5 pass. Each failure is the assertion on the missing behaviour. Examples: `stub_verb_not_implemented` gets `error: unrecognized subcommand 'pane'` and exit 2 where it wants exit 1; `roster_profile_is_refused_with_the_roster_story` gets `error: unexpected argument '--profile' found` exit 2, wants exit 1 and `error: not implemented (story #648)`; `a_bad_debug_value_is_still_a_policy_refusal_before_dispatch` gets exit 2, wants 3.
3. The 5 that pass today are guards, deliberate: `a_legacy_verbs_json_usage_error_is_untouched`, `the_session_forms_are_not_refused` and `a_malformed_pane_form_is_a_usage_error_not_a_refusal` (existing behaviour the `--pane` redesign must keep), and `the_fixture_covers_every_new_leaf_and_every_shared_flag` and `the_fixture_groups_new_lines_by_owning_story` (T has already written the fixture; they stop a later edit from dropping lines).
4. `cli_surface_test`: `every_surface_line_parses` and `fixture_leaf_set_equals_clap_leaf_set` fail (the 20 new leaves are "in cli-surface.txt but unknown to clap"); `every_pending_line_does_not_parse_yet` still passes.

**Satisfiability check (not committed).** To prove the RED is reachable, and the tests compile and agree with the brief's clap shapes, T built a throwaway prototype of the API in a scratch copy of the workspace outside the repo (the scratchpad directory), with every verb a stub, the output module, the wiring, the accessors and a hack `main.rs`. All of `pane_verbs` (53), `profile_verbs` (9) and `cli_surface_test` pass against it, and `pane_cli_process` passes 30 of 33 (the 3 ADR tests need F's ADR edit). The prototype caught two test mistakes, now fixed: `--check-arg -f` is read by clap as a flag (use a non-hyphen value; whether `--command-arg`/`--check-arg` should set `allow_hyphen_values` is a decision for #644, see advisory), and the usage-before-refusal order for `say --pane X` with no text. It is not part of the repo and not a design.

## Ready for F

Confirmed: RED is valid. `pane_cli_process` fails on assertions about behaviour that does not exist, and the two in-process targets fail to build only on API the brief commits F to create. F may implement against these tests.

Tell O: `T-red complete, RED is valid. F may implement against the authored tests.`

Notes for F:
- Resolve in the accessor **before** refusing: `say --pane X` with no text, `say --pane X a b`, and `answer --pane X` are exit 2, not the story-646 refusal.
- `emit` text mode writes `error: <message>` to `err`; a stub's whole `err` is that one line.
- Stage nothing under `tests/support/`: `rustfmt` on a test file that `#[path]`-includes `support` reformats those files. T reverted that; run `rustfmt --check` on new files only.
- If F changes a pinned name or signature, record it in decisions.md and tell T, who owns the test change in Phase 6.
