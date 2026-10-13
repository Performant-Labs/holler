# Handoff-A-dup: Phase 7 - #850: a body driver for a Claude Code pane  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-850-implementation
**Diff base:** ef881fc5b08c10f7ee1216e06c6f2c74edff2bf5   **Diff head:** 5f015c0fb8ef6c61a2eba23aa23e38f86d9964e5
**Reuse map:** `docs/handoffs/850-brief.md` ("Reuse", "Where the code goes", F1-F12)
**Verdict:** PASS

## Summary

PASS. F extended the objects the Reuse map named and built no parallel path. The new objects (`claude_attach_driver` with its `turn` child, `config/claude_row.rs`) are the brief's own, justified in writing. Delivery and observation stay the adapter's, reached through the one `HarnessPort` that `holler-cli` builds with the wiring's `claude_harness`. The hub, `holler-proto` and the adapter are untouched.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/tx_switch.rs:351` | `relaunched_and_driven` repeats the tail of `switch_in_place` (`drive` into `last_observed.driven`, then `cas_put` at the stored generation). It reuses `drive` as written; only the two-line tail is shared by shape, and the record being written differs (the relaunch's stored record, not the planned one). | None needed. If a third caller appears, fold the tail into one `record_driven(ports, stored, body, held)` helper. |
| 2 | warn | `crates/holler-body/src/config.rs` (`attaches_to_claude_code`) and `crates/holler-body/src/claude_attach_driver.rs` | The method and the free function it delegates to are one comparison written once (the brief's source-scan test pins it). The free function exists only because validation runs before a `SessionConfig` exists. | None. Noted so the pair is not later read as two copies. |
| 3 | warn | `crates/holler-body/src/config/claude_row.rs:45` | `is_uuid` is a new UUID-shape check. The only other 8-4-4-4-12 check in the tree is a test assertion in `holler-adapter-claude/tests/hermetic_test.rs:370`, so there is no production validator to reuse, and the body cannot depend on the adapter (A1). | None. |
| 4 | warn | `crates/holler-cli/tests/claude_rig/mod.rs` | A new fake `claude` script plus inbox listener. The nearest neighbours are the adapter's `seams_test.rs` script (a different crate, different job: it tests the adapter's own bounds) and `FakeClaude` in the testkit (a in-process `HarnessPort`, which cannot stand in for a real `holler body run` child). Not a near-copy of `Hub`, `Body`, `mint_token`, `join`, `wait_for` or `StateDir`: the rig uses `support::{wait_for, StateDir}` and the `fake_server.rs` OpenCode fake by `#[path]`, unedited. | None. |

Confirmed against the candidate list and the Reuse map:

- **Driver.** `Driver` stays the one enum with the one choice point (`spawn_or_attach`); the `ClaudeAttach` variant adds one arm per method. `task.rs` gains wiring only (queue, presence, turn id and crash isolation are not copied), and `rebind` now goes through `Driver::spawn_or_attach` instead of a second attach path. `ClaudeAttachDriver` returns `DriverEventStream::new`, `DriverEvent`, `DriverError`, `Status` and `StopReason`, copies no SSE or poll loop of `HttpAttachDriver`, and mirrors only its public shape.
- **One implementation of delivery.** The body calls `send_prompt`, `session_activity` and `locate_process` of the injected port and implements no frame, socket or `claude` command. `wiring::claude_harness` is made `pub(crate)` and `process_env` is the lookup `Wiring::connect` used inline, now named, so there is one construction and one environment read. `body_cmd` builds the harness only when a row attaches to Claude Code.
- **Choke point.** The one body `.send_prompt(` call is in `claude_attach_driver/turn.rs`, and `hold_single_path_test` gains `the_body_calls_send_prompt_from_exactly_one_place` (the plan review's W-2). Holds and the pane gate stay in the hub's `send_prompt`; nothing is enforced at the driver.
- **Support probe.** `claude_code_support` reuses `query::resolvable` and `holler_pane::pane::CLAUDE_CODE_COMMAND`, in the shape of the neighbouring `harness_support` arms.
- **Pane engine.** A single process's plan reuses `find_driver`, `check_first` and the existing `Reads`; the switch reuses `drive`; reconcile reuses `check_driven` and `record_with`, and the one-use `record` wrapper was removed. No second drive path exists.
- **Candidates.** No new copy of the token store operations, `Lockout`, `Roster`, the `log(Severity, ...)` helper or the test harness helpers.
- **Wire and hub.** `git diff --stat` lists no file under `crates/holler-hub/src` or `crates/holler-proto/src` and no golden file. ADR 0014, ADR-0021 and `docs/protocol/v2.md` are amended in the same change.
- **Size.** `task.rs` is 802 and `connection.rs` is 875, both wiring-only growth with the brief's named overflow locations. They are under the 900 limit. New files are 59, 158 and 222 lines.

No duplication; extension is clean.

## Notes for F

None (PASS).
