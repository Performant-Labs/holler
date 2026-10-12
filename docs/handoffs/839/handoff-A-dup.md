# Handoff-A-dup: #839 - DRIVEN follows each pane's session of record  (anti-duplication gate)

**Date:** 2026-10-12 (09:55 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`)
**Diff base:** `4e691d9` (the merge-base with `origin/main`)   **Diff head:** `8d3e7fa`
(F's change set is `949115e..d480424`; the rework this cycle, T-green's, is `d480424..8d3e7fa` and touches tests only)
**Reuse map:** `docs/handoffs/839-brief.md`, "Reuse" (line 383) and "Where the code goes" (line 339); this run has no `survey.md`
**Verdict:** PASS

## Summary

PASS: 0 `block`, 6 `warn`. F extended every object the Reuse map named, and built no parallel path to any of them:

- **The hub's prompt path.**
  - `say`'s prelude moved into `talk::admit`; it was not copied. `say` and `check_only` both call it.
  - `LiveHandle::query`'s send-and-wait moved into the one private `forward`, which `rebind` also calls.
  - There is no new `LiveCommand` variant, and `circuit.rs` grew by nothing.
  - `send_prompt` is still the only builder of `session/prompt`.
- **One predicate and one selection.**
  - The hub's and the CLI's pane-state predicates are gone, replaced by `holler_pane::gate::prompt_refusal`.
  - `roster_cmd`'s pair rule now calls `shown_driven_pair_differs`.
  - `first_refusal` is shared by the hub's `pane_gate` and by `FakeHub`.
- **The engine.**
  - `activity_refusal` was extracted from `check_idle` and is shared with the plan's driver check.
  - `tx_switch/plan.rs` is a move of the plan's helpers, which are gone from `tx_switch.rs`, extended to hand back what they read.
  - The plan and reconcile share `driver_of`, `not_driven` and `named`.
- **The edges.**
  - `control/say` moved whole into `control_say.rs`, with one `SayError` mapping.
  - `control_rebind.rs` follows `control_hold.rs`. It reuses `find_by_token`, `presence()`, `replace_session` and `parse_session_id`, and fills one roster row through `roster/rebind.rs`, as `set_turn_id` and `set_last_turn` do.
  - `wiring/hub.rs` reuses `store.rs`'s `Effect`, `timed_out`, `hub_unavailable` and `undecodable`, and the accept rule (`wait_ended`, `QUEUE_ACCEPT_WAIT`).
  - `FakeHub` runs over `FaultSwitch<HubOp>`.
  - `live/tests.rs` is a pure move: `diff -w` against the removed block shows only a three-line doc header.

None of the named Phase 7 candidates was copied:
- `token.rs` is untouched.
- `Lockout` and `Roster` are reused in `driven_support`.
- The body's `log_session` is reused.
- `driven_e2e_test` imports `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir` from `support`.

The warns, most important first:
1. A second copy of the hub's resolve scaffolding.
2. The hub's refusal words copied into `FakeHub`.
3. The fast-path gate order spelled twice.
4. The failed-exchange composition in two files.
5. Test fixtures re-typed, `session_id` among them.
6. A size budget of the brief that the test kit's `hub.rs` exceeds.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/src/live/target.rs:26-53` | **`resolve_prompt_target`'s id branch re-implements `Registry::resolve_session`'s scaffolding** (`live.rs:762-793`). It does the same steps: walk every handle's cached presence; collect `(LiveHandle, SessionAd)` pairs; then none is offline or `Unknown`, one is `Found`, and more are `Ambiguous`, with the same `"{hostname}/{name}"` candidates.<br><br>`offline_advertises` (`:51`) is `matches_offline` (`live.rs:798`) with another predicate.<br><br>**Why a warn.** The brief justified the new function and its file in writing (D1, P1), so this is not a block. But the classification rules now exist twice and can drift apart: the candidate format, or the offline fallback. | One private helper in `live.rs`, e.g. `Registry::resolve_where(live: impl Fn(&LiveHandle, &SessionAd) -> bool, offline: impl Fn(&str, &SessionAd) -> bool) -> ResolveOutcome`, which both call: `resolve_session` with the name rule, and the id branch with `advertises`. `live.rs` is 832 lines, so it has room. |
| 2 | warn | `crates/holler-pane-testkit/src/hub.rs:311-317`, `:217`, `:228` | **`FakeHub::why` is a verbatim copy of the hub's `pane_state_refusal`** (`holler-hub/src/panes/gate.rs:20-26`).<br><br>`judge` also re-spells two sentences:<br>- the hub's `"pane {} {}, so it refuses prompts ({code})"` (`gate.rs:52`);<br>- the real client's `session-held` sentence (`pane/wiring/hub.rs:173`).<br><br>**The selection is shared; the words are not.** `conformance/hub.rs` checks only the code and the pane name. `hold_single_path_test`'s "defined once" scan covers only the three code literals, and only in `holler-hub` and `holler-cli`. So these copies can drift without any test failing. | Put the hub's short phrase on `PromptRefusal` in `holler_pane::gate` (e.g. `PromptRefusal::what(self) -> &'static str`). The hub's `pane_gate` and `FakeHub` then both use it.<br><br>Alternatively, have the conformance compare the fake's whole message with the real hub's. |
| 3 | warn | `crates/holler-hub/src/talk.rs:233-239`; `crates/holler-hub/src/interrupt.rs:117-122` | **The advisory fast path is spelled twice.** It is the hold's `would_refuse`, then `pane_gate` guarded by `(Some(panes), Some(id))`. Each spelling copies `send_prompt`'s order (`circuit/dispatch.rs:113-115`).<br><br>That order is what makes "an operator hold refuses first, with its own kind" (Acceptance 4) hold on the fast paths. With three spellings, one can be reordered alone. | One helper beside `admit`, e.g. `talk::fast_gates(registry, &handle, &ad, grant) -> Result<(), WireError>`. `admit` calls it, and `interrupt` calls it with `grant: None`, so the order is written once next to the authority's. |
| 4 | warn | `crates/holler-cli/src/pane/wiring/hub.rs:227-233` | **`failed()` repeats the closure inside `store.rs::call`** (`store.rs:354-360`): `timed_out`, then `Timeout { op }`, else `hub_unavailable`.<br><br>The policy pieces are shared (made `pub(super)`). The five-line composition is now in two files.<br><br>**Why a warn.** The brief limited `store.rs` to "visibility of four items only", so this was the closest reuse F could make. | A follow-up: move `failed` into `store.rs` as `pub(super) fn exchange_failed(method, effect, op, error)` and have `call` use it too. |
| 5 | warn | `crates/holler-hub/tests/driven_support/mod.rs:54-57` (and the fixture builders listed in the next column) | **`id(n)` is `holler_pane_testkit::fixture::session_id(n)` typed again:** the same signature and the same `ses_{n:026x}`. The file already imports `holler_pane_testkit::fixture::sample_pane` (`:428`), and D8 and Acceptance 15 make `session_id` the one hub-routed fixture. All of this is T-owned test code.<br><br>**A "healthy pane on `sor`, SHOWN `sor`, DRIVEN `d`" builder is written seven times over `sample_pane`:**<br>- `circuit/dispatch.rs` `hold_tests::{gate_pane, healthy}` (`:471-483`);<br>- `panes/gate/tests.rs::{pane, parked, unhealthy}` (`:23-47`);<br>- `driven_support::pane` (`:427`);<br>- `conformance/hub.rs::{pane, parked, unhealthy}`;<br>- `fake_hub_test.rs::pane` (`:53`);<br>- `driven_test.rs::pane` (`:40`);<br>- `gate_test.rs::pane` (`:30`).<br><br>**Smaller copies across test binaries** follow a habit this repo already has (`attach_mode_test::join_fresh` says it is copied on purpose):<br>- `driven_support::Conn` is close to `pane_dispatch_test::Conn`;<br>- `driven_e2e_test::with_state_dir` is close to `pane_integration::with_state_dir`;<br>- `driven_e2e_test`'s inline attach block is close to `attach_mode_test::attach_block`. | Replace `id` with `session_id`, called directly or by `use … as id`.<br><br>Add one builder to the test kit's `fixture` (e.g. `observed_pane(name, sor, driven)`, with `parked` and `unhealthy`) and use it in the seven places. A follow-up is fine. |
| 6 | warn | `crates/holler-pane-testkit/src/hub.rs` (317 lines) | **The brief's "Where the code goes" puts this file under 250 lines**, and handoff-F's list of size deviations does not name it. It is not a lint failure: the limit is 900.<br><br>**Other touched files near the limit**, all under 900:<br>- `circuit.rs` 898 (+0);<br>- `pane_verbs/switch.rs` 895 (+2, against "no growth");<br>- `roster.rs` 893 (+1: `mod rebind;`, the fix A's Phase 3 warn 1 asked for);<br>- `codec_test.rs` 890 (+1, as planned). | Note it at S. Folding finding 2 trims the file a little. |

No parallel path to an object the map named. The rework this cycle introduced no drift: T-green changed six test files and no production file.

## Notes for F

None are required: the verdict is PASS. Each warn above is local, and none is needed for this story to merge.

## Notes for O

- **Findings 1 to 4** are small refactors of production code, each a few lines. Open them as one follow-up issue, or relay them to S. Nothing in them changes behaviour.
- **Finding 5** is T's test hygiene. The `session_id` line is the cheapest to fix now, if a repair pass happens anyway.
- **Out of scope but noticed.** `origin/main` moved to `6eff9cc` (#853) after this branch's merge-base.
  - It touches five of this branch's files: `CHANGELOG.md`, `docs/adr/ADR-0003.md`, `docs/adr/ADR-0021.md`, `docs/pane-control.md` and `crates/holler-cli/tests/profile_verbs/apply/rig.rs`.
  - It adds no `Ports { … }` literal, so bringing the branch up to date is a text merge and needs no new `hub:` field.
- **Still open from earlier phases, not decided here:**
  - Phase 3 warn 2 (the P8 corner) is for the operator.
  - Acceptance 20(d) row (1) needs O's ruling.
  - T-green's advisory 3 (a grant spent before the pane gate refuses, in a race only) is for S.

## Patterns referenced

- `crates/holler-hub/src/live.rs` (`resolve_session`, `matches_offline`, `query`, `forward`, `replace_session`); `crates/holler-hub/src/roster.rs:570-644` (the one-row forward-fills); `crates/holler-hub/src/control_hold.rs`.
- `crates/holler-hub/src/circuit/dispatch.rs` (`send_prompt`); `crates/holler-hub/src/talk.rs` (`admit`, `send_turn`); `crates/holler-body/src/prompt_dispatch.rs` (`handle_answer`) and `connection/session_dispatch.rs` (`spawn_answer_dispatch`).
- `crates/holler-cli/src/pane/wiring/store.rs` (`call`, `timed_out`, `hub_unavailable`); `crates/holler-cli/src/say_cmd.rs` (`run`, `wait_ended`).
- `crates/holler-pane-testkit/src/{fixture.rs, fault.rs, pane_store.rs, harness.rs}`; `crates/holler-cli/tests/support/{mod.rs, hold_rig.rs}`; `crates/holler-cli/tests/{attach_mode_test.rs, pane_integration/main.rs}`; `crates/holler-hub/tests/{pane_support/mod.rs, pane_dispatch_test.rs}`.
