# Handoff-A-dup: #845 switch-after-relaunch  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-845-implementation
**Diff base:** da16b23e60387d648a387c908f31f60ceff73d6b   **Diff head:** b38df22
**Reuse map:** docs/handoffs/845-brief.md ("Reuse map")
**Verdict:** PASS

## Summary
F extended the analogous objects the brief named. The re-send runs inside the crate's one `poll` through `Tui::watch`
(which gained a per-poll `unseen` action; `attach_tui` passes a no-op), reuses `Tui::query`, `known`, `deadline_after`
and the existing `Call::send_by` (lib.rs:513, not added by this diff), and the reply rules moved into one private
`select` used by the first send and every re-send. No second poll, bound, subprocess kind or type. Production diff is
`attach.rs` only; `lib.rs`, `tx_switch`, the port, the fakes and `holler-proto` are untouched.

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-adapter-opencode/tests/attach_test.rs` (`Rig::on_select`) | Re-derives the fake tmux data-file path (`sock.display-message.out`) and writes it by write-then-rename, a near-copy of `Fake::reply` plus its private `file()`. Deliberate: the brief bars editing `fake_tmux.rs` (clash with #844 PR #870) and a rename is atomic where `Fake::reply` is not. The path literal couples the test to the fixture's naming. | After #844 merges, fold an atomic `reply` into `Fake` and have `on_select` call it. No change needed now. |
| 2 | warn | `attach_test.rs` (841 lines) | Above the ~800 soft limit (lint fails at 900). The new tests added 135 lines. | The next change to this file moves the #845 block (`ac29f`-`ac29i`, `patient`, `Rig::on_select`/`selects`) into its own test file. |
| 3 | info | `tests/support/stub.rs` (`hang`, `on_nth`) | New helpers are additive and distinct from `freeze` (whole server) and `json_once` (reply for next request). `hang` is per-route and `on_nth` is a callback, neither of which an existing helper covers. Not duplicates. | None. |

No duplication; extension is clean. No architectural drift: the choke-point, wire, persistence and concurrency
dimensions are not touched. Docs (ADR-0021, `pane-control.md`, CHANGELOG, module doc) each state the rule once.

## Notes for F
None (PASS).
