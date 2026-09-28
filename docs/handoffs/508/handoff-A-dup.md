# Handoff-A-dup: Phase 7 - #508/#509 remote hub-admin client  (anti-duplication gate, round 2 after S REWORK)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Diff base:** daf6633 (full feature); rework delta ac4e002..a8f39d9   **Diff head:** a8f39d9
**Reuse map:** docs/handoffs/506-brief.md §Files "Reuse map"
**Verdict:** PASS

Supersedes the round-1 gate (PASS on daf6633..9760ba4, committed at ac4e002; still in git history). The four round-1 warns stand unchanged: `read_reply` compared with `next_envelope`, `send_ping` and the `circuit/ping` answer, two `ControlError` variants, and the 898/899-line headroom. The rework touched none of them. This pass reviews the rework delta: F-rework's Q-1/Q-2/Q-3 fixes and T's four new tests.

## Summary

PASS. The rework stays inside the one new object the brief called for (`circuit/admin.rs`) and adds no module and no public surface. `RequestOutcome` and `FrameOutcome::Rejected` follow the file's existing small-enum-per-decision-point style. The `admin_dropped` fix makes that line match `admin_connected`'s four-field vec and uses the existing `log(Severity, ...)` helper. The `send_prompt` choke point, `dispatch_allowlisted` and `circuit.rs` are all untouched, and circuit.rs is still 898 lines. There are two new near-copies. Both are small, local and warn-only: `reply_decode_error` rebuilds `wire.rs::send_error_with_reason`'s frame-building logic, and the new tests repeat the identity-file read boilerplate a fifth time.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/src/circuit/admin.rs:276-287` (`reply_decode_error`) | The frame-building half (parse `id` as `CorrelationId`, then `Envelope::error_frame` if it parses, else an unkeyed `Envelope::Error { id, error }`, then `encode`) is a line-for-line copy of `wire.rs:33-40` (`send_error_with_reason`). F's handoff calls this "reusing a proven pattern", but the code copies it. It does not call it. The copy is justified in part: `wire.rs` writes to a WS `Sink`, while the admin loop replies through an `mpsc::UnboundedSender<String>`, so the admin loop cannot call `send_error` itself. The raw-JSON id recovery is new and belongs in admin.rs. `control_server.rs:716` `unkeyed_error_line` is a third, older variant of the same idea. This is not in this stack's reject list (token store, Lockout, Roster, `log`, test harness), and it is about 6 lines. | No change required for this issue. Optional follow-up: add `pub(crate) fn error_frame_text(id: Option<&str>, code, message, reason) -> String` to `wire.rs`. `send_error_with_reason` and `reply_decode_error` would both call it, and `unkeyed_error_line` could fold in later. |
| 2 | warn | `crates/holler-cli/tests/remote_admin_test.rs:236, 279, 321` (plus the existing 108, 172) | Each new test repeats the same five-line "read `identity_path` → 32-byte array → `hub_x25519_pubkey` → `connect_ws` → `go_live_as(..., role)`" setup. The file now has five copies. No shared helper existed for it before, so this is local repetition, not a near-copy of `support::{Hub, StateDir, join, mint_token, wait_for}`. Those are reused correctly, and so are `raw_ws::{connect_ws, go_live_as, hub_x25519_pubkey, decode_next}`. | When T adds the remaining AC 6/7/8/13/16 tests (still open per S item 4), first extract a local `async fn live_socket(state, hub, token_id, label, role) -> WsClient` in `remote_admin_test.rs`, or put it in `support::raw_ws` if a second test file needs it. The copy count will roughly double otherwise. |

No parallel path. The extension is clean. The rework introduced no architectural drift. F correctly left `circuit.rs::handle_inbound`'s decode-failure path and the Unix-socket dispatch alone, because they are separate code paths. That is not a partial fix of a shared bug. The new `#[cfg(test)]` allowlist unit test calls the real `allowlisted_verb` and does not re-implement the map. (Its name says "six" and its doc says "seven". That wording mismatch is T/S's to settle, not a duplication issue.) The v2.md change moves and adds prose only. There are no catalog, error-code or golden-file changes.

## Notes for F

None (PASS).

## Patterns referenced

- `crates/holler-hub/src/wire.rs:13-45` (`send_error`, `send_error_with_reason`)
- `crates/holler-hub/src/control_server.rs:704-721` (`encode_error_frame`, `unkeyed_error_line`)
- `crates/holler-hub/src/circuit.rs:810-817` (`handle_inbound` decode-failure reply)
- `crates/holler-cli/tests/support/raw_ws.rs` (`connect_ws`, `hub_x25519_pubkey`, `decode_next`, `go_live_as`)
- Round-1 baseline: this file at ac4e002
