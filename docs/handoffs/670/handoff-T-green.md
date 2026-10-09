# Handoff-T-green: #670 the pane/profile CLI skeleton (skeleton slice c) - Phase 7 (verify GREEN + Tier 2)

**Date:** 2026-10-09
**Branch:** issue-670-implementation (at 016eeaa; T's one test addition uncommitted)
**Issue:** #670
**Handoff-F reviewed:** docs/handoffs/670/handoff-F.md
**Handoff-T-red:** docs/handoffs/670/handoff-T-red.md

## GREEN confirmation

```
cargo test -p holler-cli --test pane_cli_process --test pane_verbs --test profile_verbs   (run 3 times, no flake)
  pane_cli_process 33 passed, pane_verbs 53 passed (58 after T's additions below), profile_verbs 9 passed
cargo test --workspace --no-fail-fast
  exit 0; 98 test binaries with results, 1023 passed, 0 failed, 5 ignored (other crates', unchanged)
```

These are the numbers F reported; no discrepancy.

Spot-check that the tests pin behavior: with the production code temporarily mutated (reverted with `git checkout`, tree confirmed clean), (a) changing `exit_code`'s usage branch from 2 to 1 made `spec_only_requires_profile`, `json_conflicts_text`, `command_arg_and_command_json_are_mutually_exclusive` and others fail; (b) changing the `not implemented (story #N)` wording made `format_is_a_global_flag`, `roster_profile_is_refused_with_the_roster_story` and others fail.

**Test change in this phase.** F flagged no wrong test. F did list code paths no test reaches. T added five in-process tests to `crates/holler-cli/tests/pane_verbs/output_api.rs` (282 -> 389 lines, well under 900), no production code touched:
- `emit_text_ends_with_exactly_one_newline_and_empty_text_writes_nothing`
- `emit_json_error_message_is_put_on_one_line`
- `a_failed_write_is_never_exit_0` (a writer that fails with BrokenPipe: exit 0 becomes 1 in both formats, a usage error stays 2)
- `emit_stream_stops_at_the_first_failed_write` (an infinite iterator returning is the proof; it is the `pane watch | head` guard)
- `an_unencodable_result_is_reported_on_err_and_leaves_out_empty`

All pass on F's code (pane_verbs now 58/58). They pin the rules of F's decision 6, which the brief did not state; they are invariants, not timings.

## Tier 1 results

| Command | Result |
|---|---|
| `bash scripts/lint.sh` | exit 0 (only 600-line warns; `cli.rs` 780) PASS |
| `bash scripts/changelog-check.sh` | ok PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean PASS (re-run on holler-cli after T's addition: clean) |
| `cargo machete` | no unused dependencies PASS |
| `cargo test --workspace --no-fail-fast` | 1023 passed, 0 failed PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 3 passed PASS |
| `cargo test -p holler-cli --test wire_selftest` | 3 passed PASS |

## Tier 2 results

- Coverage per acceptance criterion: every AC has a backing test (below). PASS.
- Test quality: each test names one behavior; the process tests use the real binary with no sleeps; no fixed-sleep or timing assertions in the new targets. The suite is proportionate (20 stub verbs are table-driven). PASS.
- RED validity carried through: the three RED targets and `cli_surface_test` went RED to GREEN with no test edited by F (T's commit 0afbb93 is unchanged by F's commit). PASS.
- `#[allow(...)]` without `// #NNN`: none in the new src or test files. PASS.
- Files under 900 lines: lint passes. PASS.
- Secrets: the new surface handles no tokens; no test or log excerpt carries one. PASS.
- Protocol-visible change: none to the wire (no goldens, `docs/protocol/v2.md` untouched, F records the §10 drift for #634). N/A.
- Docs: ADR 0003 rows parse (`docs_cli_test` green). PASS.
- Evidence appendix: `evidence.md` (nine facts, verbatim) covers the unchanged-code facts the tests rely on (e.g. `usage` code, `is_valid_code`); T found nothing further to add. PASS.
- Browser/UI: none in this repo; not run.

## Acceptance criteria status

Mapped to the brief's numbering as T-red's table records: AC 2, 3, 4 (refusals), 5 (flag matrix), 7 (ADR rows, fixture) by `pane_cli_process` (33) and `cli_surface_test`/`docs_cli_test`; AC 4 (accessors), 5 (`validate()`), 6 (seam, output API) by `pane_verbs`; AC 6 profile half and 8 (per-verb stubs) by `profile_verbs`. All PASS.

## Blocking issues

None.

## Advisory notes

- Still unit-uncovered (reachable only through the binary): `resolve_format`, `scan_args`, `route()` called directly, `SpecFlags::validate()` for `--role`/`--ctx-soft`, and every `Unwired` method answering `not-implemented`. The binary tests pin their observable behavior; #644 (which calls `validate()`) is the natural place for the `validate()` cases.
- `Unwired::run_probe` answers `ProbeResult::Error("not implemented")` with no story number (F's hedge); #649 replaces it.
- `cli.rs` is 780 lines (fail at 900).
- T's test addition is uncommitted in the worktree; O/the script commits it.

## Test-only rework (S: REWORK, test-only, #678)

S asked that the shared test scaffolding stop pinning sibling stubs and `Wiring::connect`'s body, so the wave-3 stories (#643-#647, #649, #650, #662, #664, #665) edit only their own files. No `src/` change. What changed:

1. **Harness** (`tests/verb_harness/mod.rs`): added `run_verb_with(argv, format, ports: Ports<'_>)`; `run_verb` delegates with `unwired_ports()` (a `static UNWIRED: Unwired`). The `Wiring` import is gone, so #649 replacing `connect()` cannot break any in-process verb test, and #643/#644/#662 can pass the fakes of #638. New `tests/verb_harness/parse.rs` (included by the two in-process targets through `verb_harness`, and by `pane_cli_process` through `#[path]`): `try_parse`, `accepted` (parses, or only a required positional is missing, so sibling positionals do not break flag checks), `unknown_argument` (`ErrorKind::UnknownArgument`), `resolved_format` (through `output::resolve_format`), and the `SPEC_FLAG_SETS` matrix.
2. **Frozen roots**: deleted `seam_pane_stub_routes_text_to_err_and_json_to_out` and `seam_profile_stub_routes_text_to_err_and_json_to_out` (exact duplicates of `pane_verbs/list.rs` and `profile_verbs/list.rs`); moved the JSON-with-shared-flags case to `pane_verbs/launch.rs`; rewrote the ports test as `unwired_ports_answer_not_implemented` over `unwired_ports()`. AC 6 stays covered by the per-verb harness cases.
3. **`pane_cli_process`**:
   - `flags.rs` now parses in-process (`Cli::try_parse_from`) and asserts refused flags as `UnknownArgument`; no subprocess, no stub line. `format_is_a_global_flag` checks `Cli.format` and `resolve_format`.
   - The positive launch/relaunch spec-flag matrix moved to `pane_verbs/launch.rs` and `relaunch.rs` (`pane_*_accepts_every_spec_flag`, one shared `SPEC_FLAG_SETS`), which also covers every flag on `relaunch`.
   - `usage.rs`: the three cases that pinned #643/#644 stub lines now check the parse (`accepted`) or the resolved format, not a stub line.
   - `stub.rs`: `STUBS` is the only story-numbered table, grouped under `// #NNN` lines (rustfmt removes blank separators); `stub_verb_not_implemented` and its JSON twin iterate it, so a story deletes only its group. The two legacy-refusal lines (#646, #648) moved there as named constants that `legacy_verbs.rs` imports. `PANE_VERBS`/`PROFILE_VERBS` are now permanent name lists; `docs_rows.rs` has its own permanent `STORY_GROUPS` (used by the ADR-adjacency and fixture-header tests, and checked against the verb lists).

Verification (worktree, after the rework): `cargo test --workspace` 1029 passed, 0 failed (was 1023; net +6 from the new parse, matrix and consistency tests, minus the two deleted duplicates); `pane_verbs` 59, `profile_verbs` 8, `pane_cli_process` 34, `cli_surface_test` 3, `docs_cli_test` 3, `wire_selftest` 3 all pass; `cargo clippy --workspace --all-targets -- -D warnings` clean; `scripts/lint.sh` and `changelog-check.sh` clean; `cargo machete` clean; the touched test files are rustfmt-clean. Grep over `crates/holler-cli/tests` finds `not implemented (story #` only in `stub.rs` (the stub table and the two legacy-refusal constants) and the story-parameterised `assert_stub_routes` format string, and no `Wiring` anywhere. Largest touched test file is `docs_rows.rs` at 256 lines.

Not done (S advisory, outside T's remit): rebase onto `origin/main` (#669; CHANGELOG conflict), PR-body AI disclosure, and the #649/#634/#644/#648 notes.

## Rework 2: S pass 2 (REWORK: one production item, three test items)

**Date:** 2026-10-09. **Branch:** issue-670-implementation at 02a3655 (F's `prompt_target.rs` fix is committed; T's edits below are uncommitted). **Handoff-F reviewed:** the "Rework 1" section of `handoff-F.md`. F listed no test as wrong.

### What T changed (tests and manifest comments only; no `src/` file)

1. **S item 1, the proving tests** (F's fix: `resolve_tail` refuses more than two positionals in the SESSION form):
   - `crates/holler-cli/tests/pane_verbs/target_flags.rs`: `an_extra_positional_split_off_the_session_form_by_a_flag_is_refused` (the four argv forms S named; asserts `parse` is Ok, the premise, then that `resolve` is an `Err` containing `positionals`, so it is the accessor's `Usage` and not a clap error) and `a_flag_between_session_and_text_still_resolves_to_both` (`say`, `interrupt`, `answer` valid split forms resolve by value; the guard).
   - `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs`: two rows added to the table of `a_malformed_pane_form_is_a_usage_error_not_a_refusal` (9 to 11 cases): `say io/alpha hello --queue extra` and `say --profile demo io/alpha hello --queue extra` exit 2, stdout empty, stderr names `positionals`, is not the #646 refusal and has no `no live holler hub` text (the second row pins that usage wins over the refusal). The loop now also fails on a `no live holler hub` line for every row.
2. **S item 2** (`process/stub.rs`): the `STUBS` doc now says a story deletes its own entries and keeps its `// #NNN` line, with the reason (adjacent deletions conflict); the doc of `PANE_FORM_REFUSAL` says #646 keeps its `// #646` line and the blank line below. Comments only.
3. **S item 3** (`process/legacy_verbs.rs`): the one `use` line importing both refusal constants is two lines, `PANE_FORM_REFUSAL as PANE_REFUSAL` and `ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL`, so #646 and #648 each touch only their own `use` line (rustfmt keeps them separate; they are adjacent lines, but each story edits its own line only).
4. **S item 4, stale comments:** `process/main.rs` module doc no longer lists the flag matrix among what only the binary can show (it points to `flags.rs` and `pane_verbs/{launch,relaunch}.rs`); `Cargo.toml` comments now say the in-process targets run over the ports a test gives them (`Unwired` by default), and that `pane_cli_process` is its own target so a change to the in-process harness cannot break it, and that it builds only `verb_harness/parse.rs`.

### RED proof (the new tests against the pass-1 `resolve_tail`)

With `prompt_target.rs` temporarily replaced by `git show 51f3bed:crates/holler-cli/src/prompt_target.rs` (restored with `git checkout`; tree confirmed free of any `src/` change):

```
cargo test -p holler-cli --test pane_verbs target_flags
  an_extra_positional_split_off_the_session_form_by_a_flag_is_refused ... FAILED   (resolve returned Ok for the first argv: the assertion about the missing refusal)
  a_flag_between_session_and_text_still_resolves_to_both ... ok                    (guard: passes before and after)
  17 passed; 1 failed
cargo test -p holler-cli --test pane_cli_process
  a_malformed_pane_form_is_a_usage_error_not_a_refusal ... FAILED
    holler ["say","io/alpha","hello","--queue","extra"]: want exit 2 naming "positionals"; got code 1, stderr "error: no live holler hub reachable at /tmp/holler-test-..."
    holler ["say","--profile","demo","io/alpha","hello","--queue","extra"]: want exit 2 naming "positionals"; got code 1, stderr "error: not implemented (story #646)"
  33 passed; 1 failed
```

Both fail on the missing behaviour (not a compile or setup error), exactly the "before" F recorded.

### GREEN (the fixed code, 02a3655)

```
cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_cli_process --test docs_cli_test --test cli_surface_test
  pane_verbs 61, profile_verbs 8, pane_cli_process 34, docs_cli_test 3, cli_surface_test 3: all passed
cargo test --workspace --no-fail-fast      (run 2, nothing else running)
  exit 0; 98 binaries, 1031 passed, 0 failed, 5 ignored   (T-green's 1029 + the 2 new in-process tests; the process test is a table row)
cargo clippy --workspace --all-targets -- -D warnings    clean
bash scripts/lint.sh                                      exit 0 (600-line warns only; no file near 900)
bash scripts/changelog-check.sh                           ok
cargo machete                                             no unused dependencies
rustfmt --check --edition 2021 on the touched test files  clean (only the pre-existing diffs in tests/support/{cmds,hold_rig}.rs, which this branch does not touch)
```

F's Tier 1 numbers reproduce (`pane_verbs` 59 became 61 with my two tests). The earlier targets F listed (`talk_test` 18, `interrupt_test` 11, `answer_cli_test` 11, `cli_invocation_test` 14) pass inside the workspace run.

### Flake observed (not #670)

Workspace run 1 (right after a full rebuild) failed 7 of the 11 tests in `interrupt_test`, each in `wait_warm` with the body `reconnecting` / `io disconnected mid-turn` (`support/warmup.rs:136`). No quota error (`/tmp` had 7.5 GB free). `interrupt_test` alone passed 3 of 3 (11/11 each), and the second full workspace run was fully green. The failing path is hub/body reconnection under load, which this change does not touch (`interrupt_cmd.rs` runs after the warm-up, which is where it failed). Worth knowing for the run's CI check: this target has a load-sensitive warm-up; I did not diagnose it.

### Blocking issues

None. Production code is unchanged by T.

### Advisory notes

- S re-audits AC 4 against the two new tests plus the table rows.
- The earlier advisory list above stands. The three T edits are uncommitted; O/the script commits them.
