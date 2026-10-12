# Handoff-T-green: #847 remote-admin last_seen flake

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Issue:** #847
**Handoff-F reviewed:** docs/handoffs/847/handoff-F.md (no production change; no "tests that look wrong")
**Handoff-T-red:** docs/handoffs/847/handoff-T-red.md

## GREEN confirmation
- `cargo test -p holler-cli --test remote_admin_test`: 21 passed, 0 failed (3.21 s).
- `cargo test -p holler-cli --test remote_admin_liveness_test`: 7 passed.
- Target test, 30 consecutive runs of the compiled test binary while a background loop ran the whole `remote_admin_test` target with `--test-threads=16` (12 iterations) plus a CPU burn on every core: 30 pass, 0 fail.
- Target test under the flake's trigger, `HOLLER_HEARTBEAT_INTERVAL_MS=300`, 10 runs: 10 pass, 0 fail.
- Mutation spot-check (re-run by T): `deps.roster.touch(&record.token_id, "admin/mutant");` as the first statement of `circuit::admin::run`. The test fails 3 of 3 at `remote_admin_test.rs:392` (`left: ...128, right: ...127`, `...129 vs ...128`, `...130 vs ...129`). Restored with `git checkout -- crates/holler-hub`; `git diff -- crates/holler-hub` is empty.

## Tier 1 results
| Check | Result |
|---|---|
| `cargo clippy -p holler-cli --test remote_admin_test --test remote_admin_liveness_test -- -D warnings` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | FAIL at `crates/holler-cli/tests/token_load_test.rs:60` (`unneeded cloning of iterator items`). This file is not in the diff (`git diff origin/main --stat -- crates/holler-cli/tests/token_load_test.rs` is empty), so the failure is on main too. Not caused by #847; AC 6's `--all-targets` clause cannot be met by this story without touching an unrelated file. |
| `bash scripts/lint.sh` | PASS (warnings only, none for the touched files); `remote_admin_test.rs` 827 lines, `remote_admin_rig.rs` 114, limit 900 |
| `git diff origin/main --stat -- crates/` | only `tests/remote_admin_test.rs` and `tests/support/remote_admin_rig.rs` (AC 5) |

## Tier 2 results
- AC 1 grep: no `thread::sleep` in `remote_admin_test.rs`; the target test does not call `Body::start` (the remaining `Body::start` hits, lines 98, 802, 808, belong to other tests). PASS.
- Test quality: the one rewritten test names one behavior (admin traffic does not write the body's row), sits at process level as D4 requires, is not duplicated, and is proven by the mutant. The support helpers are each used (`presence_frame` by two tests, `wait_past_second` and `joined_rig` by the target test and `rig_with_env`). PASS.
- Timing asserted as invariant (a `wait_for` on the second boundary, no fixed sleep); `STARTUP_WAIT` bounds it. PASS.
- `grep last_seen remote_admin_test.rs`: only the target test and the masking helper (line 643). No other time-window assertion on a live body. PASS (D5).
- No secret in assertions; no `#[allow]` added. PASS.
- Protocol, golden, docs-CLI, migration, security checks: N/A (test-only change).

## Acceptance criteria status
1. No sleep, no real body: PASS (grep above).
2. Invariant asserted (`last_seen` equal, `conn_state == connected`, no clock-based move): PASS.
3. 30 consecutive loaded runs, zero failures: PASS (30/30).
4. Mutation proof: PASS (3/3 red, restored).
5. No production changes: PASS.
6. `remote_admin_test` 21/21: PASS. Clippy clean for the touched targets; workspace `--all-targets` fails on pre-existing `token_load_test.rs:60`: NOT MET, unrelated to this diff.

## Blocking issues
None.

## Advisory notes
- Pre-existing, unrelated: clippy failure at `token_load_test.rs:60`. Worth its own issue.
- Pre-existing flake in `remote_say_wait_and_hub_query_json_match_local_after_masking` (T-red saw 1 of 14 on unmodified files; passed in all my runs). Out of scope under D5; follow-up.
- No `CHANGELOG.md` entry (F's flag): O/S to decide; a test-only flake fix plausibly needs none.
- `rustfmt --check` was not applied; the touched files carry pre-existing long-line style.
- Evidence appendix: no `evidence.md` needed beyond T-red's citations; the tests rely on nothing outside the diff other than `Clock::System` = `token::now_secs` and the liveness-timeout derivation, both cited in T-red.
