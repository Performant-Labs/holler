# Decisions: #847 remote-admin last_seen flake

## A (Phase 3, up-front plan review) - 2026-10-12
- **Decided:** PASS on `docs/handoffs/847-brief.md`, no blocks, four warns. Detail is in `docs/handoffs/847/handoff-A.md`.
  - **Layer and object are right.** The fix is test-only and reuses `live_socket`, `rig_with_env` internals, `roster_json`, `run` and `hub_status_json`. Production is untouched and `admin.rs` has no roster touch (verified).
  - **W-1:** wait for the second boundary BEFORE the admin calls, not after (D2 reads as a loop of admin calls).
  - **W-2:** keep the hand-rolled body's `WsClient` bound until the last assertion; it is never polled, so the test must stay well under the 45 s liveness timeout.
  - **W-3:** the presence literal is currently in one place only (`remote_admin_test.rs:175`); extract one helper into `support/remote_admin_rig.rs` (the D3 clause), reused by both tests.
  - **W-4:** `remote_admin_test.rs` is 814 lines; new helpers go in `remote_admin_rig.rs`, and T-green records the final line count.
- **Ran (read-only):** read the brief, the cited test and rig code, `raw_ws.rs`, `circuit.rs` ping and liveness, `roster.rs` touch and clear, `admin.rs` touch grep, a presence-literal grep across `tests/`, and `git diff origin/main --stat`.
