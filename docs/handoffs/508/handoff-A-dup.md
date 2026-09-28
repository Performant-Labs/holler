# Handoff-A-dup: Phase 7 - #508/#509 remote hub-admin client  (anti-duplication gate)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Diff base:** daf6633   **Diff head:** 9760ba4
**Reuse map:** docs/handoffs/506-brief.md §Files "Reuse map"
**Verdict:** PASS

## Summary

PASS. F extended every object the Reuse map named and did not build a parallel path. The admin loop calls the existing `control_server` handlers through one new `dispatch_allowlisted` entry, and the Unix socket's own `control/*` arms now go through that same entry, so each verb still has one implementation. `send_prompt` is still the only prompt path, and `hold_single_path_test` is unmodified. The hub-key pinning has one copy: the body's `hello_exchange` gained a `role` parameter. `authenticate`/`authenticate_and_hello` are unchanged, the address policy reuses `server_address::{parse, loopback_only_check}`, and the `*_cmd.rs` rendering is shared. The test initiator was moved into `raw_ws.rs` rather than copied (hub_hygiene_test.rs lost 90 lines). What remains are three small local helpers that look like existing ones. Each is `warn` only.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-body/src/admin_client.rs` (`read_reply`) | Frame-skipping read loop resembles `connection::next_envelope` (connection.rs:432). It is not a copy: it matches the reply by correlation id, turns an `Error` envelope into `Wire`, and gives each close/error a distinct message, which `next_envelope`'s `Option` return cannot express. Its doc says so. | None required. If a third one-shot client appears, have `read_reply` call `next_envelope` in a loop (widen it to `pub(crate)`) and keep only the cid match local. |
| 2 | warn | `crates/holler-hub/src/circuit/admin.rs` (`send_ping`, `circuit/ping` arm) | `send_ping` repeats `SessionConnection::send_ws_ping`'s two sink calls without its reconnect side effects. The `circuit/ping` answer re-creates circuit.rs:845's `PingAck` build with a fixed `hostname:"hub-admin"`. Both are 2–3 lines, and the body-side versions are tied to `SessionConnection` state, so the admin loop cannot call them. | None required. Optional: a free `ws_ping(sink)` helper in circuit.rs that both call. |
| 3 | warn | `crates/holler-hub/src/control.rs` (`ControlError::{RemotePolicyRefused, RemoteUnavailable}`) | MO 9 said to add **one** variant. F added two, because the exit-3 policy refusal needs its own arm in every `*_cmd.rs`. This is a new variant on the existing enum, not a parallel error type, and exit 3 matches `body join`. It departs from the brief's wording, so I'm recording it here instead of passing over it. | O/S decide whether to amend MO 9's wording or accept. No code change needed for duplication. |
| 4 | warn | `crates/holler-hub/src/circuit.rs` (898), `crates/holler-proto/src/docs.rs` (899) | Both are within 1–2 lines of the 900-line `lint.sh` guard. That is inside the budget, but there is no headroom. `control_server.rs` fell to 829 because `status_doc`/`read_listening` moved verbatim to `control_status.rs`, the fallback the brief had pre-agreed. | The next story to touch either file should do the pre-agreed move first (`hello_exchange`/`hub_hello_doc` → `circuit/hello.rs`; the enums section of docs.rs → a sibling module). |

No duplication; extension is clean. I found no architectural drift introduced during rework. T's GREEN pass only edited test/fixture files.

## Notes for F

None (PASS).

## Patterns referenced

- `crates/holler-hub/src/control_server.rs` (`dispatch_control`, `dispatch_session_control`, new `dispatch_allowlisted`)
- `crates/holler-hub/src/circuit.rs:97-149, 625-632, 845-848` (liveness/ping helpers, `circuit/ping` answer)
- `crates/holler-body/src/connection.rs:432-458` (`next_envelope`, `send`), `connection/handshake.rs` (`hello_exchange`)
- `crates/holler-body/src/server_address.rs` (`parse`, `loopback_only_check`)
- `crates/holler-cli/tests/support/raw_ws.rs`, `crates/holler-cli/tests/hub_hygiene_test.rs` (moved initiator)
