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
