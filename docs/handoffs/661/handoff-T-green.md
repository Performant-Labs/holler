# Handoff-T-green: Phase 7 - #661 the hub profile registry (GREEN)

**Date:** 2026-10-09
**Branch:** issue-661-implementation
**Issue:** #661
**Handoff-F reviewed:** docs/handoffs/661/handoff-F.md
**Handoff-T-red:** docs/handoffs/661/handoff-T-red.md

## GREEN confirmation
- `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's own skip): rc 0, 120 binaries, 1297 passed, 0 failed, 5 ignored. Matches F's numbers.
- The five new/amended hub binaries (`profile_registry_test` 12, `profile_persistence_test` 8, `profile_feed_test` 6, `profile_handlers_test` 11, `pane_membership_test` 6) were run 5 times in a row: all 5 runs green, no flake. Both conformance suites pass in full (23 and 19 cases).
- Behavior-removal spot checks (production edited, then restored with `git checkout`; tree clean afterwards):
  - Neutralizing the `refuse_profile_move` call in `panes/store.rs::cas_put`: `pane_membership_test` goes 3 failed / 3 passed (AC 34, 35, 37 fail).
  - Neutralizing `check_name` in `profile/store.rs::cas_put`: `profile_registry_test` goes 3 failed / 9 passed.
  So the tests pin behavior and do not pass regardless of F's change.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo test --workspace` (CI skip) | PASS, 1297/0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `bash scripts/lint.sh` | PASS (rc 0; only pre-existing `warn:` lines for untouched files) |
| `bash scripts/changelog-check.sh` | PASS (ok) |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `cargo machete` | PASS |

F's reported commands were re-run; no discrepancy.

## Tier 2 results
- Test coverage per AC: AC 1-39 backed by the authored tests; AC 41, 44, 45 are S-side greps (F's greps re-checked via `git diff origin/main --stat`: no change to serve.rs, control_server.rs, lib.rs, rename.rs, holler-pane, holler-proto, Cargo files, ADRs, protocol doc, goldens). PASS.
- Test quality: each test names one behavior; tiers are the cheapest (store via trait from threads, handlers via direct dispatch, no process-level). No `thread::sleep`; waits are bounded. Concurrency tests assert invariants (exactly one winner, no lost update). No redundant tests found to prune. PASS.
- File sizes: largest test file 577 lines (<600); largest production file `profile/store.rs` 415. `#[allow]` count: one, pre-existing, with `// #669`. PASS.
- Secrets: `a_stored_secret_value_fails_closed_without_echoing_it` and the over-the-wire env-value test assert absence of the secret. PASS.
- Error handling and data integrity: corrupt/unreadable/unwritable file, stale generation ordering, delete/re-create, restart persistence are all covered. PASS.
- Protocol/goldens: no wire, error-code or golden change, so no golden drift to check. PASS.
- Evidence appendix: `evidence.md` (11 entries) covers the unchanged-code facts the tests rely on (persist loader, feed `select`, params defaults, `Profile` slug check, `EnvVarName`, `next_generation`). No gaps found.
- Playwright/browser: N/A (none in this repo).

## Acceptance criteria status
All criteria PASS, each backed by the test listed in handoff-T-red.md. F reported no tests that look wrong, and I found none to repair. No test file was changed in this phase.

## Blocking issues
None.

## Advisory notes
- AC 6's time windows use the real `now_millis()` and assume the wall clock does not step backwards during a test (already noted in RED).
- Follow-ups for O/operator, not blocking: the A finding 1 generic-registry issue (and `panes/persist.rs:17-21` doc line), A finding 8 cross-story notes on #647/#662/#663/#664, and the README "Debug output" table gap.
