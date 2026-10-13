# Handoff-A-dup: #878 pane compact  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Diff base:** ef881fc5b08c10f7ee1216e06c6f2c74edff2bf5   **Diff head:** e43c74696f8f78fd61d68e7176c63015dc58daef
**Reuse map:** docs/handoffs/878-brief.md section A5 ("Reuse map") and E1-E7
**Verdict:** PASS

## Summary

F extended what the map named and built no parallel path. The engine `tx_compact::compact` calls `check_health` and
`check_idle` through a `pub(crate)` re-export (visibility only, no copy). The fake, the adapter and the engine all take
the refusal code from the one `activity_refusal` mapping. The adapter reuses `known`, `Call::send`,
`unexpected_status`, `prompt::session_activity`, `prompt::messages`, `string_at`, `json_of` and `GET_CONFIG`. The verb
reuses `resolve_pane_record`, `unmanaged::missing`, `output::emit` (the `close.rs` shape) and `HarnessInfo::endpoint`.
The new objects (`ports/compact.rs`, `tx_compact.rs`, the adapter's `compact.rs`, the verb file) are the ones the brief
names. None of the Phase 7 candidates (token store, `Lockout`/`Roster`, `log(Severity, ...)`, `Hub`/`Body`/`mint_token`/
`join`/`wait_for`/`StateDir`) is touched or copied.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane-testkit/src/harness.rs` (830 lines) | The brief and A's round-two review recorded the file at 692 lines with room; it is now 830, past the ~800 flag and under the 900 lint failure. F's handoff reports it. The next change to this file has no room. | Open a follow-up to split the fake's compaction/activity state (`contexts`, `activities`, `pins`, `compact_runs` and their seeders) into a child module before the next edit. Not needed for this PR. |
| 2 | warn | `crates/holler-adapter-opencode/src/compact.rs:80`, `crates/holler-pane-testkit/src/harness.rs` (`compact`) | The activity refusal `PaneError::Refused { code, message: format!("session {..} {doing}") }` is built at three sites (`check_idle`, the adapter, the fake), each from the shared `activity_refusal` mapping. The mapping is single; only the 3-line wrapper repeats, and the message wording differs per site (the engine names the pane, the adapter and fake name the port or nothing). Not a parallel path. | Optional: if a fourth site appears, add a helper beside `activity_refusal` that returns the `PaneError`. |
| 3 | warn | `crates/holler-adapter-opencode/tests/compact_test.rs:63-103` | `assistant()`/`user()` message fixtures and the stub setup (`server_listing`) are local to the test file. No equivalent exists in `tests/support` (`prompt_test.rs:418` and `agent_view_test.rs:63` build ad hoc JSON strings with other fields), so this is not a near-copy. A second consumer would justify moving them to `support/fixtures.rs`. | None now. |

No duplication; extension is clean. No architectural drift re-flagged: `holler-pane` imports nothing from `holler-cli`;
the new refusal codes are open codes (`RefusalCode::from_static`), not entries in the closed `holler-proto` table; no
hub, wire, registry or log-event change; nothing persists; the Claude adapter answers the trait default
(`compact-unsupported`) and `Unwired` answers `not-implemented`; every wrapper (router, `FakeHarnesses`, `Wired`,
`Unwired`) forwards `compact` explicitly. The adapter's 120 s bound is a struct update of `call_until` as A's round-two
warn 2 asked, not a new `call_until` parameter. `opencode/src/lib.rs` is 793 lines (under 800).

## Notes for F

None (PASS).
