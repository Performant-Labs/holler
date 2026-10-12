# Handoff-A: #847 remote-admin last_seen flake  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Brief reviewed:** docs/handoffs/847-brief.md   **Reuse map:** the brief's "Seams the fix reuses" and "Reuse map" sections (no separate survey.md)   **Wireframe:** N/A (no UI)
**Verdict:** PASS

## Summary
PASS, no blocks, four warns. The plan is test-only, keeps the hub untouched, and reuses the right seams (`live_socket`, `rig_with_env` internals, `roster_json`, `run`, `hub_status_json`). I checked each seam against the worktree and the cited production lines. The hub is correct and the test is wrong, so a test-only fix is the right layer. None of the stack checks (choke point, wire/protocol, ADRs, state, concurrency, secrets) is touched.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | D2 "loop the admin calls until the hub's clock has crossed a second boundary" | pattern consistency / test soundness | The order matters. A mutant's `touch` writes `now` at call time, so it is only distinguishable from the baseline if that call lands in a later second than the baseline's `last_seen`. Looping the admin calls and then checking is weaker than crossing the boundary first. The sibling tests use `wait_for` on a condition, so this should follow the same pattern. | Read the baseline, then `wait_for` until the test process's `SystemTime` seconds exceed the baseline `last_seen`. Only then run one `roster --server` and one `hub status --server`, then the "second of each" (AC 2), then read the row. Do not run the admin calls before the boundary. The brief has found no `now` or uptime-seconds field in `hub status`; T should confirm, and fall back to `SystemTime` as D2 allows. |
| 2 | warn | Hand-rolled silent body (AC 1) | pattern consistency | The `WsClient` returned by `live_socket(.., "body")` must stay bound (e.g. `let _body_ws = ...`) until the last assertion, and the test must not drop it early or write `let _ =`. Dropping it closes the socket and the row goes `gone` (`Roster::clear` writes `last_seen`), which would break the test for the wrong reason. The client is also never polled, so it never answers the hub's WS Ping (`HOLLER_WS_PING_INTERVAL_MS`, default 10 s). Hub liveness is 45 s by default, so this is fine only while the test stays short, as the Risks section already says. | Name the binding, assert `conn_state == "connected"` in the final check (AC 2 already requires it), and record the longest observed run as the brief says. No hub env change. |
| 3 | warn | "Do not add a second presence-JSON builder if one extraction serves both tests" | abstraction level / duplication | The brief says "the AC 4 first-half test already builds the same literal", but the literal is currently at `remote_admin_test.rs:175-183` and nowhere else in `tests/` or `tests/support/`. The new test is the second consumer, so a helper is justified. Placing it inline in `remote_admin_test.rs` would leave a second copy in the same file. | Extract one `presence_frame(hostname, sessions)` (or equivalent) into `support/remote_admin_rig.rs`, which both `remote_admin_test.rs` and `remote_admin_liveness_test.rs` already share. Use it in both tests. This is the D3 clause, now taken, so AC 5 allows exactly that file. The existing AC 4 first-half literal must keep the same JSON. |
| 4 | warn | File size | size and structure | `remote_admin_test.rs` is 814 lines, above the 800 flag and under the 900 `scripts/lint.sh` gate. The rewrite plus the boundary wait could grow it. | The plan names where new code goes: the boundary-wait helper and the presence helper go in `support/remote_admin_rig.rs` (78 lines, plenty of room), not in the test file. The test body should end net-neutral or smaller. T-green records the final `wc -l`. |

All four are non-binding to the verdict. Findings 1 and 3 are worth O or T treating as binding in the test authoring step.

## Notes for O
Not required (PASS). Suggested, not blocking: copy findings 1 and 3 into the T-red instructions, so the boundary wait comes before the admin calls and the presence literal is extracted once.

## Checks that came back clean
- **Production blast radius.** `grep touch` in `crates/holler-hub/src/circuit/admin.rs` shows only doc comments (lines 6, 65, 232). `Roster::touch` (`roster.rs:390`) is reached from `circuit.rs:831` for body-socket text frames only. The claim "the hub is correct" holds.
- **Choke points.** Not touched (no prompt path).
- **Wire, protocol, goldens.** No change, so no golden or `docs/protocol/v2.md` update is needed.
- **ADRs.** None contradicted; D3 keeps `Clock::Manual` unit-only.
- **State and concurrency.** No hub changes, no persistence, no new writes.
- **Secrets.** The test uses `live_socket`, which reads the joined credential from the test's own temp `StateDir`; nothing is logged.
- **Public repository.** No infrastructure names in the brief.
- **Siblings.** #844-#848 do not touch `remote_admin_test.rs`, `remote_admin_rig.rs`, `raw_ws.rs` or the hub. #846 may reshape `tests/support/` generally. The only shared file here is `remote_admin_rig.rs`, which the pane rigs do not use. A rebase conflict is unlikely.
- **Anti-duplication candidates for Phase 7.** `live_socket`, `roster_json`, `hub_status_json`, `wait_for`, `mint_token` and `join` must be reused as they are. A new second presence builder, or a second copy of the `live_socket` handshake, is a rejection.

## Patterns referenced
- `crates/holler-cli/tests/remote_admin_test.rs` (the target test at 364-387, the AC 4 first-half test at 159-199, and AC 3 at 330-360)
- `crates/holler-cli/tests/support/remote_admin_rig.rs` (`rig_with_env`, `run`)
- `crates/holler-cli/tests/support/raw_ws.rs` (`live_socket`)
- `crates/holler-hub/src/circuit/admin.rs` and `crates/holler-hub/src/roster.rs:390-420`
- `crates/holler-hub/src/circuit.rs:100-125` (liveness and WS ping intervals)
