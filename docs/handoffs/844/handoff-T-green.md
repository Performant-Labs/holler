# Handoff-T-green: #844 oc-tmux-utf8

**Date:** 2026-10-12
**Branch:** issue-844-implementation
**Issue:** #844
**Handoff-F reviewed:** docs/handoffs/844/handoff-F.md
**Handoff-T-red:** docs/handoffs/844/handoff-T-red.md

## GREEN confirmation
- `cargo test -p holler-adapter-opencode --no-fail-fast`: agent 11, attach 23, hermetic 33, prompt 12, tui 13, utf8_locale 1 passed; real_opencode 12 ignored (opt-in). `the_query_reads_a_live_pane_in_a_process_with_no_utf8_locale` ran against real tmux (not skipped).
- `cargo test --workspace --no-fail-fast`: exit 0, 168 `test result: ok`, no FAILED.
- `cargo test -p holler-cli --test pane_integration`: 71 passed, 4 ignored (opt-in #649 real-OpenCode scenario not run).
- Spot-check: with the `.arg("-u")` line deleted from `tui.rs`, `tui_test::ac11c_...` fails again (12 pass, 1 fail). The real-tmux test and the 12 `attach_test` cases fail without `-u` per the T-red record; the production file was restored with `git checkout` (tree clean).

## Tier 1 results
| Check | Result |
|---|---|
| `bash scripts/lint.sh` | exit 0, size warns only for untouched files |
| `bash scripts/changelog-check.sh` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | FAIL in `crates/holler-cli/tests/token_load_test.rs:60` (`unneeded cloning of iterator items`). File is not in the diff vs origin/main: pre-existing, unrelated. Clippy on `holler-adapter-opencode` is clean (F). |
| `cargo test -p holler-cli --test docs_cli_test` | 3 passed |
| `cargo test -p holler-cli --test wire_selftest` | 3 passed |
| `cargo machete` | not installed here; not run |

## Tier 2 results
- Coverage per AC: PASS (see below).
- Test quality: PASS. Argv pin is a pure unit test; the locale behavior needs a real tmux and has exactly one test with three cases; the fixture change pins every hermetic call site without new tests. No redundant tests found.
- Error/secret handling: N/A (no new error path, no secret in any assertion).
- Protocol/golden: N/A (no wire change).
- File sizes: new test file 162 lines, under 900. No new `#[allow]`.
- No Playwright/browser surface in this repo.

## Acceptance criteria status
- AC 1 (`tmux_command` argv `flag ++ ["-u"] ++ call`, all three sockets): PASS, `tui_test::ac11c_...`.
- AC 2 (live pane readable under `LANG=C`, `LC_ALL=C`, no locale): PASS, `utf8_locale_test`.
- AC 3 (rig binary env has no locale): PASS, `pane_integration` guards test + `scratch::binary_env`.
- AC 4 (`-u` in exactly one place in `tui.rs`): PASS, `tui.rs:135`; `lib.rs:234` is an unrelated `env -u`.
- AC 5 (fixture requires `-u`): PASS, `attach_test` 23 passed.
- Docs (CHANGELOG, ADR-0021, module doc): present.

## Blocking issues
None. The workspace clippy failure is pre-existing in an untouched file and outside this issue.

## Advisory notes
- A's warn 1: `docs/testing.md` does not say why the real-tmux test is not `#[ignore]`; the reason is in decisions.md. Optional follow-up.
- Pre-existing clippy lint in `token_load_test.rs` will fail CI's clippy step on the toolchain used here; worth a separate issue if CI uses the same toolchain.
