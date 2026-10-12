# Handoff-A: #878 pane compact  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Brief reviewed:** docs/handoffs/878-brief.md   **Reuse map:** brief section A5 ("Reuse map") and E1-E7   **Wireframe:** N/A (no UI)
**Verdict:** BLOCK

## Summary

The plan extends the right objects almost everywhere: a default-armed `HarnessPort::compact` beside `send_prompt`
and `session_activity`, an engine beside `tx_switch`, the existing `session-busy` constant, `check_idle` and
`check_health` called rather than copied, a verb file shaped like `switch.rs`, and no change to the hub or the
wire. One thing blocks: the engine is placed in `holler-pane` and told to reuse `resolve_pane_record`, which lives
in `holler-cli`. As written F can only duplicate the pane-state predicate and its messages (a third copy) or
invert the crate dependency; T also has no engine signature to write RED tests against. Everything else is a warn.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | A2 "The engine `holler_pane::tx_compact` ... plan (the record and the pane-state refusals ...)" vs A5 reuse map "`resolve_pane_record`" | layering / dependency direction | `resolve_pane_record` and `pane_state_refusal` (the messages around `holler_pane::gate::prompt_refusal`) are in `crates/holler-cli/src/say_cmd.rs:384,434`. `holler-pane` cannot import them. `tx_switch::plan::read` is a separate record read with its own scope handling and `unmanaged::missing`, and it has no pane-state check at all. So the plan's engine either re-implements the parked/unhealthy/mismatch refusals with new messages (a third copy; the brief itself says "no second pane-state predicate") or moves `say_cmd` code into `holler-pane` (a drive-by edit of a file the brief does not list). The engine's signature is also unstated, so T cannot author RED. | Follow the `interrupt_pane` precedent (`crates/holler-cli/src/interrupt_cmd.rs:61-67`): the CLI verb calls `resolve_pane_record(ports, pane, profile)` (this also guarantees a session of record, which `check_idle` alone does not: it returns Ok for none), then calls `holler_pane::tx_compact::compact(ports, &record)`. The engine does `endpoint`, `check_health`, `check_idle`, `ports.harness.compact(.., None)`, and the shrink rule. State that signature and the return type in A2. |
| 2 | warn | A2 "`check_idle` and `check_health` become `pub(crate)` in `tx_switch::plan`" | pattern consistency | `mod plan;` in `crates/holler-pane/src/tx_switch.rs:86` is private, so `pub(crate)` on the functions is not reachable from `tx_compact`. `tx_switch.rs` is not in A5. | Add `tx_switch.rs` to A5 with `pub(crate) use plan::{check_health, check_idle};` (or make `plan` `pub(crate)`). |
| 3 | warn | A2 "`Timeouts` gets one field `compact`" | pattern consistency | `Timeouts` is built by struct literal in `tests/support/fixtures.rs:30`, `hermetic_test.rs:487,679`, `attach_test.rs:48,398`, `tmux_capture_test.rs:36`. A new field breaks each, and `hermetic_test.rs:679` likely asserts the defaults. A5 lists none of them. | List these test files in A5 as mechanical edits; T owns them in RED. |
| 4 | warn | A2 adapter order, "GET /config" | identity and secrets | `GET /config` is a whole server config that can hold a provider `apiKey` or other credential. `verify_serving` (`server.rs`) reports a wrong reply with `Call::unexpected`, which quotes up to 60 bytes of the body. Copying that shape would put config text in a message. The brief asks "quotes no reply" only for the summarize reply. | Say that every reply in `compact` (config, messages, summarize) is reported with `unexpected_status`, and that only the `provider/model` name pair is ever carried into a message. Add a test that a config reply holding a marker string never reaches the error text. |
| 5 | warn | A2 `Timeouts.compact` replaces `call` and `request` for the whole `Call` | abstraction level | The cheap reads (`known`, the activity reads, `GET /config`, the token reads) then each get 120 s. A server that hangs on the first `known` GET holds the caller for 120 s with nothing sent. The brief accepts this (stated in the ADR). Also, `prompt::session_activity` builds its own 10 s `Call` (`harness.call(port, OP_SESSION_ACTIVITY)`), so "in one Call" is not true for the activity check. | Prefer: ordinary `harness.call` (10 s) for `known`, activity and `GET /config`, a second `Call` with the compact bound for the `summarize` POST and the token reads after it. If kept as written, correct the "one Call" wording and name the 120 s hang in the ADR. |
| 6 | warn | A5 `crates/holler-adapter-opencode/src/lib.rs` | size and structure | `lib.rs` is 778 lines. The field, default, impl method, `mod compact;` and a route constant take it to about 800, past the point where the plan must name where new code goes. | Put the `SUMMARIZE` route const in `compact.rs` (a child module sees the parent's private `Route`); keep only the field, the default, `mod compact;` and the 3-line method in `lib.rs`. |
| 7 | warn | A2 "model ... not a `provider/model` pair is `compact-model-unpinned`" | pattern consistency | Model ids can themselves contain `/` (routed providers). A rule that rejects more than one `/` would refuse a valid pin. | Split at the first `/`; provider and model must each be non-empty. Add a test with a model id that contains `/`. |
| 8 | warn | A2 engine order: health, idle, then `compact` | cross-cutting | A Claude pane that is busy answers `session-busy` before `compact-unsupported`, and a stopped one answers `server-unhealthy`. The refusal that never changes should not hide behind one that can. | Acceptable as is; if kept, say so in the ADR paragraph, since acceptance 3 expects `compact-unsupported` from the Claude fake (seed it idle and healthy). |
| 9 | warn | A1 ADR edits | ADRs | ADR-0021 section 2 (line 97-100) states the list of methods bounded otherwise and "returns within I5's bound (default 10 s)". A1 names the new paragraph and the tables, not that sentence. | Add the section 2 sentence to A1's edit list. |

The remaining checks hold. `compact` is not a prompt, so the `send_prompt` choke point is untouched; the hub
roster, token store and wire are unchanged, with `--json` data additive and `schema_version` 1. The new codes are
open codes (`RefusalCode::from_static`, as `PROMPT_UNSUPPORTED`), not entries in the closed `holler-proto` table.
Nothing persists. No blocking I/O enters a hub path (the verb is CLI-side and synchronous). The wrapper list
(router, `FakeHarnesses`, `Wired`, `Unwired`) is complete for production code; the test wrappers `Mutant` (both
conformance tests) must forward `compact` for the new cases to reach `FakeHarness`, which the plan's "each new
case gets a mutant" already implies. The token source is unobserved (stated in the brief); the plan handles it by
refusing `unavailable`, never zero.

## Notes for O

Amend the brief, then start a fresh run (do not `resumeFromRunId`). Required:

1. A2/A3/A5: fix the engine and CLI split (finding 1). State `tx_compact::compact(ports, &Pane) -> Result<Compacted, PaneError>` or equivalent, say that the CLI resolves the record through `resolve_pane_record` (profile included), and say that the engine owns health, idle, the act and the shrink rule only. Adjust A7 and the "engine tests" wording so T knows what to call. Add `crates/holler-cli/src/say_cmd.rs` to "not touched".

Recommended in the same edit, so F and T do not rediscover them: findings 2 to 5 (A5 file list, the secrets rule for `GET /config` replies, the bound's scope), and 6 to 9 as one-line notes.

## Patterns referenced

- `crates/holler-cli/src/interrupt_cmd.rs` (`interrupt_pane`), `crates/holler-cli/src/say_cmd.rs` (`resolve_pane_record`, `pane_state_refusal`), `crates/holler-pane/src/gate.rs` (`prompt_refusal`)
- `crates/holler-pane/src/tx_switch.rs`, `tx_switch/plan.rs` (`read`, `check_health`, `check_idle`)
- `crates/holler-adapter-opencode/src/lib.rs` (`Timeouts`, `Call`), `prompt.rs`, `server.rs` (`verify_serving`)
- `crates/holler-pane/src/ports.rs`, `ports/router.rs`; `crates/holler-cli/src/pane/mod.rs`
