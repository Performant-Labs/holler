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
