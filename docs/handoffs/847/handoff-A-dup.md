# Handoff-A-dup: #847 remote-admin last_seen flake  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Diff base:** 6eff9cc   **Diff head:** ca12f3b
**Reuse map:** docs/handoffs/847-brief.md ("Seams the fix reuses" / "Reuse map")
**Verdict:** PASS

## Summary
PASS. The diff is test-only (`remote_admin_test.rs`, `support/remote_admin_rig.rs`, handoffs). It extends the named seams instead of building a parallel path: `live_socket`, `roster_json`, `run`, `wait_for` and `STARTUP_WAIT` are reused unchanged, `rig_with_env` was refactored to call the extracted `joined_rig`, and the presence literal now lives once in `presence_frame`, used by both AC 4 tests. No production file is touched.

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/remote_admin_test.rs` (827 lines) | Grew 814 -> 827, above the ~800 flag, under the 900 lint gate. A's up-front warn 4 asked for the net-neutral test body; the file is 13 lines larger. | Track; next addition to this file should move tests to a sibling target rather than grow it. |
| 2 | warn | `support/remote_admin_rig.rs:76` `wait_past_second` | Reads `holler_proto::now_secs()`, which `token::now_secs` (the hub's `Clock::System`) delegates to, so it is the same clock source and not a clock copy. No existing second-boundary helper exists in `tests/` (grep for past/next second helpers: none). | None. |

Duplication checks against the Phase 7 candidates:
- `Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`: all reused. `joined_rig` is the extracted first half of `rig_with_env` (which now calls it), not a near-copy; the earlier inline `StateDir::new` x2 + `Hub::start_with_env` + `mint_token` + `join` sequence exists once in this module.
- Presence JSON: previously one literal at `remote_admin_test.rs:175`; now one builder (`presence_frame`), no other copy in `tests/` (only comments and non-test golden files mention the method).
- `live_socket` handshake: not re-implemented; called with role `"body"`.
- `token.rs`, `Lockout`, `Roster`, `log(Severity, ..)`: untouched. The `HOLLER_HUB_LIVENESS_TIMEOUT_MS` env use matches the existing pattern in `roster_sweep_wireup_test.rs` and `remote_admin_liveness_test.rs`.
- Drift introduced in rework: none. Layering, naming (`joined_rig`, `presence_frame`, `wait_past_second` match the rig module's cadence), and the helper placement A's plan review asked for are followed.

No duplication; extension is clean.

## Notes for F
Not required (PASS).
