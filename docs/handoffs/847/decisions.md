# Decisions: #847 remote-admin last_seen flake

## A (Phase 3, up-front plan review) - 2026-10-12
- **Decided:** PASS on `docs/handoffs/847-brief.md`, no blocks, four warns. Detail is in `docs/handoffs/847/handoff-A.md`.
  - **Layer and object are right.** The fix is test-only and reuses `live_socket`, `rig_with_env` internals, `roster_json`, `run` and `hub_status_json`. Production is untouched and `admin.rs` has no roster touch (verified).
  - **W-1:** wait for the second boundary BEFORE the admin calls, not after (D2 reads as a loop of admin calls).
  - **W-2:** keep the hand-rolled body's `WsClient` bound until the last assertion; it is never polled, so the test must stay well under the 45 s liveness timeout.
  - **W-3:** the presence literal is currently in one place only (`remote_admin_test.rs:175`); extract one helper into `support/remote_admin_rig.rs` (the D3 clause), reused by both tests.
  - **W-4:** `remote_admin_test.rs` is 814 lines; new helpers go in `remote_admin_rig.rs`, and T-green records the final line count.
- **Ran (read-only):** read the brief, the cited test and rig code, `raw_ws.rs`, `circuit.rs` ping and liveness, `roster.rs` touch and clear, `admin.rs` touch grep, a presence-literal grep across `tests/`, and `git diff origin/main --stat`.

## T-red (Phase 4) - 2026-10-12
- **Decided:** RED valid. Old test fails 3/3 under `HOLLER_HEARTBEAT_INTERVAL_MS=300`; rewritten test is green 15/15 under it and red 3/3 against the `roster.touch` mutant in `admin::run` (restored, hub diff empty). Detail in `docs/handoffs/847/handoff-T-red.md`.
  - Applied A's W-1 (boundary wait before the admin calls), W-2 (named `body_ws` kept to the end), W-3 (`presence_frame` helper in `remote_admin_rig.rs`, used by both AC 4 tests).
  - Observable for the second boundary is the test process's own whole-second clock (`hub status` has no `now` field).
- **Assumed:** the test process and the hub share `Clock::System` (`token::now_secs`), so crossing a second in the test process means a later hub write stamps a later second.
- **Hedged:** the brief's claim that the new test stays green with `HOLLER_HEARTBEAT_INTERVAL_MS=300` was false as stated: the hub derives its liveness timeout from that var (3x), tearing down the silent body in 900 ms (6/12 failures). The test now sets `HOLLER_HUB_LIVENESS_TIMEOUT_MS=120000` on its own hub (existing env, no new one).
- **Evidence:** a pre-existing flake in `remote_say_wait_and_hub_query_json_match_local_after_masking` (1 of 14 on unmodified files, passes alone): follow-up, out of scope (D5).

## F (Phase 5/6) - 2026-10-12
- **Decided:** no production change. The story is test-only (brief "Production: none", AC 5), and T-red's rewritten test is already GREEN on the unmodified hub. Detail in `docs/handoffs/847/handoff-F.md`.
- **Assumed:** a test-only flake fix needs no `CHANGELOG.md` entry, because AC 5 limits the diff to the test files and `docs/handoffs/`. Flagged for O/S.
- **Hedged:** none.
- **Evidence:** target test ok (1.32 s) and ok with `HOLLER_HEARTBEAT_INTERVAL_MS=300` (0.90 s); `remote_admin_test` 21/21; clippy on `remote_admin_test` + `remote_admin_liveness_test` with `-D warnings` clean; `git diff origin/main --stat -- crates/` shows only the two test files.

## T (Phase 7, green) - 2026-10-12
- **Decided:** GREEN, no blocking Tier 2 issue. `remote_admin_test` 21/21, `remote_admin_liveness_test` 7/7, 30/30 loaded runs, 10/10 at `HOLLER_HEARTBEAT_INTERVAL_MS=300`, mutant red 3/3 and restored. Detail in `docs/handoffs/847/handoff-T-green.md`.
- **Assumed:** the workspace-wide clippy failure at `token_load_test.rs:60` is pre-existing, because that file is absent from the diff.
- **Hedged:** AC 6's `--all-targets` clippy clause is unmet for a reason outside this story; reported, not fixed here.
- **Evidence:** loaded runs used the compiled test binary directly; mutant failures at `remote_admin_test.rs:392`.

## A (Phase 7, anti-duplication) - 2026-10-12
- **Decided:** PASS. The diff extends `rig_with_env` (via `joined_rig`) and centralises the presence literal in `presence_frame`; `live_socket`, `roster_json`, `run`, `wait_for` reused as-is. No parallel path, no production change.
- **Assumed:** `holler_proto::now_secs` is the hub's roster clock source (`token::now_secs` delegates to it).
- **Hedged:** `remote_admin_test.rs` is 827 lines (above the ~800 flag, under the 900 gate); warn only.
- **Evidence:** `docs/handoffs/847/handoff-A-dup.md`.
