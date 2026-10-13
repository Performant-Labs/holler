# Handoff-A: #878 pane compact  (up-front plan review, round two)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Brief reviewed:** docs/handoffs/878-brief.md (as amended after round one, commit 54596b3)   **Reuse map:** brief section A5 ("Reuse map") and E1-E7   **Wireframe:** N/A (no UI)
**Verdict:** PASS

## Summary

Round one's block is fixed. The CLI verb resolves the record through `resolve_pane_record` (the `interrupt_pane`
precedent) and calls `holler_pane::tx_compact::compact(ports, &Pane) -> Result<CompactOutcome, PaneError>`. The engine
owns only health, idle, the act and the shrink rule, reuses `check_health` / `check_idle` through a `pub(crate)`
re-export, and does not import from `holler-cli`. All eight round-one warns are taken. No block remains. Three new
warns are about wording that F would otherwise act on wrongly; none changes a decision.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | A2 CLI: "shaped like `switch.rs` ... emits through `emit_outcome`" | pattern consistency / reuse | `switch::emit_outcome` (`crates/holler-cli/src/pane/switch.rs:127`) takes `Result<Switched, SwitchFailure>` and its `Outcome` / `Verb::{Switch, Reset}` / `render` are switch-specific; `launch::emit_outcome` is a different function with another signature. A `compact` verb cannot call either. F has two bad options: edit `switch.rs` (not in A5) or copy it. The tree's pattern for a verb whose engine returns `Result<_, PaneError>` is `close.rs:54` / `park.rs:225`: a local `Serialize` data struct and `output::emit(&mut ctx.sink, ctx.format, result, render)`, with `ErrorBody::from(&PaneError)` for the failure. | Reword A2/A5: `compact.rs` defines its own `Compacted { pane, session, tokens_before, tokens_after }` and a text renderer, and prints with `output::emit` / `emit_error` as `close.rs` does. Say `switch.rs` is not edited. Add `crates/holler-cli/src/pane/switch.rs` to "not touched". |
| 2 | warn | A2 bound: "`call_until(port, op, deadline_after(timeouts.compact))` with `request: timeouts.compact`" | pattern consistency | `OpenCodeHarness::call_until` (`lib.rs:330`) takes no request argument; it copies `timeouts.request` (5 s) into `Call.request`. The sentence implies a parameter that does not exist. `Call`'s fields are private to `lib.rs` but visible to its child module `compact.rs`, so a struct-update works without touching `lib.rs`. | In A2 say: in `compact.rs`, `Call { request: timeouts.compact, ..harness.call_until(port, OP_COMPACT, deadline_after(timeouts.compact)) }`. Do not add a parameter to `call_until`, which would touch every caller and push `lib.rs` toward 800 lines. T's "reply held longer than `request`" test pins it. |
| 3 | warn | A2 engine: "no CAS write, a compaction records nothing" vs the verb's `last_observed` | state and persistence | None found wrong. Noted for completeness: the verb writes no record, so a compacted session's new context size is not stored; `holler pane get` will not show it. This is acceptable and stated. | None. |
| 4 | warn | A2 adapter re-check of activity after the engine's `check_idle` | cross-cutting | The adapter calls `prompt::session_activity` again (a second `GET /question`, `/session/status`). Existing `send_prompt` does not re-check, and the engine already did. The brief justifies it (a turn can start in between; the port contract says a busy session is `session-busy` and nothing is sent, so the conformance case needs it at the port). Two reads are the cost. | Keep. Say in one line in the ADR paragraph that the double read is deliberate (the port contract holds without the engine). |
| 5 | warn | A3.1 `HarnessOp::Compact` and a settable `session_activity` on `FakeHarness` | pattern consistency | `HarnessOp` (`harness.rs:51-79`) may be listed or iterated by existing tests (fault-injection tables, "every op" checks). A new variant can break them silently or fail to be covered. | T greps `HarnessOp::` in `crates/holler-pane-testkit/{src,tests}` and extends any exhaustive list or table in RED. |

The round-one checks hold on the amended brief. `compact` is not a prompt, so the `send_prompt` choke point is
untouched; the hub, roster, token store and wire are unchanged, with `--json` data additive and `schema_version` 1. The
new codes are open codes (`RefusalCode::from_static`, as `PROMPT_UNSUPPORTED`), not entries in the closed `holler-proto`
table. Nothing persists. No blocking I/O enters a hub path (the verb is CLI-side). The wrapper list (router,
`FakeHarnesses`, `Wired`, `Unwired`, the test `Mutant`s) is complete. `tx_switch.rs` (399 lines) gets the re-export;
`lib.rs` of the OpenCode adapter stays under 800 with `SUMMARIZE` in `compact.rs`; `ports.rs` (750) gets only a
re-export; `conformance/harness.rs` (655) and `harness.rs` (692) have room. ADR edits now include section 2 lines 96-100.
`Timeouts` literal sites (five break, two use `..`) are named. Every reply in `compact` is reported with
`unexpected_status`, with a test. The model pin is split at the first `/`. Public-repo rule respected.

## Notes for F

Not a BLOCK. Apply findings 1 and 2 when writing; they correct sentences in the brief that cannot be followed literally.

## Patterns referenced

- `crates/holler-cli/src/interrupt_cmd.rs` (`interrupt_pane`), `crates/holler-cli/src/say_cmd.rs` (`resolve_pane_record`, `pane_state_refusal`)
- `crates/holler-pane/src/tx_switch.rs`, `tx_switch/plan.rs` (`check_health`, `check_idle`)
- `crates/holler-cli/src/pane/close.rs`, `park.rs`, `switch.rs`, `launch.rs` (`emit`, `emit_outcome`), `pane/mod.rs`
- `crates/holler-adapter-opencode/src/lib.rs` (`Timeouts`, `Call`, `call_until`), `prompt.rs` (`session_activity`)
- `crates/holler-pane/src/ports.rs` (`Ports`, defaults)
