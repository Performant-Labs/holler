# Handoff-F: Phase 6 - #839 DRIVEN follows each pane's session of record

**Date:** 2026-10-12 (09:20 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`, head `949115e`; F committed nothing, and every change is in the working tree)
**Issue:** #839

**F verdict: done.** The contract T pinned is implemented as the brief decides. Clippy, `scripts/lint.sh` and `scripts/changelog-check.sh` are clean. Every test T authored passes, except six tests that are wrong (T-1 to T-6 below, all test-only). I proved each one with a temporary local edit of the test alone: with the edit applied, every affected target is green. I then restored each test file with `git checkout --`. The source facts these diagnoses rely on are in `evidence.md`.

## What was done

**Contract docs, written first and verbatim where the brief gives the text:**
- `docs/adr/ADR-0021.md`:
  - the Amends line and the `HubPort` row in section 2;
  - section 5's I2 and its wiring;
  - section 6, now 23 rows;
  - the section 8 replacements and the section 9 rows;
  - section 11, as the brief words it;
  - section 13, item 18, the "Deferred" list, and "Decisions taken" items 32 to 45.
- `docs/adr/ADR-0014.md`, `ADR-0005.md` and `ADR-0003.md`: the passages P3 names, and one header line each.
- `docs/protocol/v2.md`:
  - the `session/rebind` row;
  - `admin/say`'s `check_only`;
  - the presence sentence and the pane hold;
  - a paragraph on re-pointing;
  - the `-32004` and `-32009` rows and their `data.reason`;
  - the note in section 11.
- `docs/pane-control.md`:
  - `reset --first` through the gate;
  - `park`;
  - the DRIVEN column, and SHOWN against DRIVEN;
  - the body's evidence and the two-step pairing;
  - a body restart reverts its re-point;
  - a `ses_` id is not a `SESSION`;
  - the cutover steps, "Later", and section 9's three new codes.
- `CHANGELOG.md`: an `[Unreleased]` Enhancements entry that links #839, records the behaviour change (`reset --first` on a pane no body drives) and says it absorbs #828; also the one clause of #649 the brief names.
- `README.md`: the `session` row of the debug-output table names the body's new `rebind` event.

**`holler-proto`:**
- `src/rebind.rs` (new): `Rebind` and `RebindResult`, each `{session, harness_session_id}`, with `deny_unknown_fields`.
- `src/lib.rs`: the module, and a line of its doc.
- `src/methods.rs`: the catalog row `session/rebind` (`Request`, `HubToBody`); the docs say 23 rows.
- `src/error.rs`: `OFFLINE_REASON` (`"offline"`), and `Error::offline(message)` beside `Error::invalid_grant`.

**`holler-body`:**
- `src/session_manager.rs`:
  - `SessionCommand::Rebind {harness_session_id, deadline, reply_tx}` and `SessionManager::rebind`;
  - `live_configs`, the configurations as started, with each attach session's live id;
  - `start` initialises the presence mirror with the configured id;
  - `presence_doc` reads the mirror.
- `src/session_manager/task.rs`:
  - `SessionPresence.harness_session_id`;
  - the `rebind` handler: the deadline, the mode, `rebind_busy`, then an attach bounded by `timeout_at`, then the swap;
  - `REBIND_TIMED_OUT`;
  - `mirror_session_id`, called by the first attach, by `start_turn`'s re-attach and by the rebind;
  - the dispatch arm.
- `src/prompt_dispatch.rs`: `REBIND_DEADLINE` (5 s), `REBIND_REPLY_WAIT` (6 s), and `handle_rebind`, shaped as `handle_answer` is: it writes one frame.
- `src/connection/session_dispatch.rs`: the `session/rebind` arm and `spawn_rebind_dispatch`.
- `src/connection.rs`: the `matches!` list and `static_wire_method` gain the name; `handle_query` answers from `live_configs(..)`. The file grows by 1 line, to 874.
- `src/registry.rs`: a doc paragraph (the configured baseline, not the live id).

**`holler-hub`:**
- `src/live.rs`:
  - `mod target;`;
  - `SayOpts.harness_session` and `LiveCommand::Say.harness_session`;
  - `LiveHandle.query_slot`;
  - the private `forward`, the one place a `LiveCommand::Query` is built. `query` calls it after its allowlist, which is unchanged.
  - The inline tests moved whole to `src/live/tests.rs`; see Deviations.
  - The file goes from 884 to 832 lines.
- `src/live/target.rs` (new): `Registry::resolve_prompt_target`, `offline_advertises`, and `LiveHandle::rebind`, its own forward with the literal `"session/rebind"`.
- `src/circuit.rs`: the `Say` arm, edited in place to set `HoldGate.harness_session`. The file stays at 898 lines.
- `src/circuit/dispatch.rs`: `HoldGate.harness_session`. `pane_gate` and `pane_state_refusal` move out to `panes/gate.rs`. `send_prompt` calls `crate::panes::gate::pane_gate` only when the prompted session advertises an id.
- `src/panes/gate.rs` (new): `pane_gate`, which maps `holler_pane::gate::first_refusal`'s answer to the `-32011` frame (`hold_kind: "pane"`, the code as `reason`, the hub's message). It ends with `#[cfg(test)] mod tests;`, which is T's file.
- `src/panes/mod.rs`: `pub(crate) mod gate;`, and one doc fix.
- `src/talk.rs`:
  - `admit`: resolve, the hold's fast path, the pane gate, then the busy check. `say` is `admit` then `send_turn`, and `check_only` is `admit` alone.
  - `busy_check`;
  - `SayOpts.harness_session`;
  - `answer` resolves through `resolve_prompt_target`.
- `src/interrupt.rs`: `resolve_prompt_target`. With text, the pane gate runs before the cancel (A's warn 3).
- `src/control_say.rs` (new): the whole `control/say` handler, with `check_only` answering `{admitted: true}`. Its one `refusal()` mapping gives `NotConnected` as `WireError::offline` and leaves `Timeout` and `Cancelled` as a bare `-32004`.
- `src/control_rebind.rs` (new):
  - `REBIND_FORWARD` (8 s);
  - `control/rebind {token_id, session, harness_session_id}`. It fails closed on its params and on an id `parse_session_id` refuses, then on the token, then on a session the body's cache does not hold. It forwards, passes a body's refusal through verbatim, and on success records before the reply.
- `src/roster/rebind.rs` (new): `Roster::set_harness_session_id`, a one-row forward-fill (A's warn 1). `src/roster.rs` gains `mod rebind;` and is 893 lines.
- `src/control_server.rs`: the `"say"` arm calls `control_say::say`; there is a `control/rebind` arm; the offline arms of `interrupt` and `answer` build `WireError::offline(..)`. The file goes from 837 to 772 lines.
- `src/control.rs`: `ControlCall::rebind` (10 s) and `ControlCall::say_check`.
- `src/lib.rs`: `pub mod control_rebind; pub mod control_say;`.

**`holler-pane`:**
- `src/gate.rs` (new): `PANE_PARKED`, `PANE_UNHEALTHY`, `PANE_SHOWN_DRIVEN_MISMATCH`, `PromptRefusal`, `prompt_refusal`, `shown_driven_pair_differs` and `first_refusal`.
- `src/hub.rs` (new): `Advertised`, `PromptMode`, `trait HubPort` (`advertised`, `rebind`, `prompt`, with no defaults), `PANE_NOT_DRIVEN` and `SESSION_HELD`.
- `src/driven.rs` (new): `Driver`, `driver_of`, `elsewhere`, `not_driven`, and the crate-private `named`.
- `src/lib.rs`: the three modules.
- `src/ports.rs`: `Ports.hub: &'a dyn HubPort`, and the amendments list.
- `src/findings.rs`:
  - `SESSION_ID_MAX` and `parse_session_id`, moved here from `tx_switch` (A's warn 5);
  - `switch_command`;
  - `Finding::with_switch_remedy` and `Finding::without_remedy`;
  - the doc of `remedy`.
- `src/tx_switch.rs`:
  - a new module doc, in step order;
  - `SESSION_DRIVEN_ELSEWHERE` and `activity_refusal`, which `check_idle` shares;
  - `drive()`, the rebind;
  - `recorded(.., driven)`;
  - `prompt()`, through `hub.prompt(Send)`;
  - re-exports of `parse_session_id` and `SESSION_ID_MAX`;
  - `mod plan;`. The file is 399 lines.
- `src/tx_switch/plan.rs` (new):
  - `Plan` and `Reads`;
  - `plan`, `find_driver`, `refuse_elsewhere` and `check_first`;
  - the plan's existing helpers, moved here: `read`, `refuse_orchestrator`, `check_health`, `check_idle`, `check_listed` and `check_unclaimed`.
- `src/reconcile.rs`: `read_bodies`, with one `hub.advertised` per pass, and only when a server pane is in scope. A failed read is an `observe-failed` finding naming `hub.advertised`.
- `src/reconcile/observe.rs`: `Bodies`, `Driven`, `check_driven`, and `record_with`, which records DRIVEN in the same `cas_put` as SHOWN. `record` stays as a wrapper, so `observe/process.rs` is not edited.

**`holler-pane-testkit` (`src/` only):**
- `src/hub.rs` (new): `FakeHub`, `HubOp`, `HoldKind`, `HubCall` and `SentPrompt`, over `FaultSwitch<HubOp>` and `holler_pane::gate::first_refusal`. It judges in this order: resolve, hold, gate, then question or permission.
- `src/conformance/hub.rs` (new): `HubFixture`, `run_hub_conformance`, and `hub_cases`, the seven documented cases.
- `src/fixture.rs`: `session_id(n)`, which is `ses_{n:026x}`.
- `src/pane_store.rs`: `pub(crate) fn records()`, a read that is not logged, for `FakeHub`.
- `src/lib.rs` and `src/conformance/mod.rs`: the modules.

**`holler-cli`:**
- `src/pane/hub_refusal.rs` (new): `classify` and `HubRefusal`, which has `NotDriven`, `PaneGate`, `Held {kind, reason}`, `Busy` and `Other`.
- `src/pane/wiring/hub.rs` (new): `HubClient` (holds no state and does no I/O at build), `send_call`, and `SEND_TURN_BUDGET` (600 s).
  - `advertised` reads `control/roster`.
  - `rebind` is `control/rebind`.
  - `prompt` is `control/say` with `check_only`, or `send_call` read through `wait_ended`.
  - Refusals map to the reset's codes. A failed exchange goes through `store.rs`'s mapping.
- `src/pane/wiring.rs`: `pub mod hub;`, the `hub` field, and `Unwired`'s `HubPort` (every method `not-implemented`).
- `src/pane/wiring/store.rs`: `Effect`, `timed_out`, `hub_unavailable` and `undecodable` become `pub(super)`. Only their visibility changes.
- `src/pane/mod.rs`: `pub mod hub_refusal;`.
- `src/prompt_target.rs`: `Routed.pane`.
- `src/say_cmd.rs`:
  - the pane codes are re-exported from `holler_pane::gate`;
  - `pane_state_refusal` decides by `prompt_refusal` and keeps only its own messages;
  - `route_target_by` refuses a `SESSION` that is not a valid name (`unknown session: X`, exit 1, before any hub) and sets `pane`;
  - `pane_not_driven`;
  - `control_refusal` takes `pane`, and its `NotDriven` arm comes first;
  - `wait_ended` is `pub(crate)`.
- `src/interrupt_cmd.rs` and `src/answer_cmd.rs`: the `NotDriven` arm comes first, and `Routed.pane` is set and read.
- `src/roster_cmd.rs`: `shown_driven_mismatch` calls `shown_driven_pair_differs`, and its doc stops naming three copies.
- `src/pane/reset.rs`: the module doc.

**Adapter:** `crates/holler-adapter-opencode/src/prompt.rs`, the module doc only.

## Design decisions

- **The body re-points by building first and swapping after (D2, I5).**
  - `rebind` refuses, before anything else, a command dequeued after its deadline, a spawn session and a busy session. Busy means a turn in flight, a queued prompt, or a state other than idle; it gets the prompt's `-32009` with the same timing fields.
  - It then attaches the new id with `HttpAttachDriver::attach` under `tokio::time::timeout_at(deadline)`.
  - Only on success does it write `config.session_id` (the one source every later attach reads), swap the driver, mirror the id into presence and shut the old driver down.
  - On a timeout the attach future is dropped. Its one await before it spawns anything is the existence check (`evidence.md`), so no late attach is ever swapped in.
  - Alternative rejected: mutate first and roll back, which needs a compensating write on every failure path.
- **Four nested bounds, each above the one below:**
  - the task's deadline, 5 s;
  - `handle_rebind`'s reply wait, 6 s;
  - the hub's `REBIND_FORWARD`, 8 s, which includes the wait for the slot;
  - `ControlCall::rebind`, 10 s.

  So each layer reports the answer of the layer below it, never its own timeout first.
- **One forward, one slot.**
  - `LiveHandle::forward` is the only place a `LiveCommand::Query` is built.
  - The per-connection `query_slot` (an `Arc<tokio::sync::Mutex<()>>`) is taken inside the caller's own `timeout`, so a caller that waits behind a stalled forward returns at its own bound (Acceptance 21).
  - `query` keeps its allowlist, unchanged, ahead of the call. `rebind` passes its own literal method.
- **An id is an address only when it is not a name.** `resolve_prompt_target` parses the target as a `RoutableName` first and resolves it by name, exactly as before. Anything else is matched against live attach ads by `harness_session_id`. An id that an offline body last advertised is `NotConnected`. Two matching bodies are `Ambiguous`. A spawn row is never matched by id.
- **The gate key is the advertised id, and the name is no key.**
  - `HoldGate.harness_session` carries the advertised id of the session being prompted, and `send_prompt` calls `pane_gate` only when there is one, so a spawn session is never pane-gated.
  - `pane_gate` reads the store once and asks `holler_pane::gate::first_refusal`, the selection `FakeHub` shares. The selection keys on the session of record or the recorded DRIVEN; two panes recording one id fail closed; the first bad pane in name order is the one named.
- **`check_only` is `admit` and nothing else.**
  - `talk::admit` is `say`'s old prelude, moved and not copied, plus the pane gate.
  - The check answers `{admitted: true}` and writes no TalkLog line, moves no turn id, and consumes no grant (`would_refuse` is the fast path).
  - `say` runs the same `admit`, so the two refuse alike by construction.
- **The engine's order is: plan, act, observe, drive, record, then the first message.**
  - `rebind` runs after `shown_session` and before `cas_put`. DRIVEN is recorded in the same `cas_put` as the session of record and SHOWN. `prompt(Send)` runs after the record.
  - A single-process pane asks the hub nothing and takes `Driver::None` (D3 item 8), so its `--first` is refused `pane-not-driven` in the plan.
  - `driver_of` applies the exclusion rule first, and lets this pane's own evidence override it, as P8 states once. Then rules (i) session of record, (ii) recorded DRIVEN and (iii) the switch's target. Then the listed fallback, which on a shared listing becomes `Unproven`.
- **A refusal after the act is `unavailable`.** When `HubPort::rebind` is refused, the TUI has already moved, so the client reports `unavailable` carrying the body's reason. The engine's message then ends with the reconcile step, never a refusal code (Acceptance 8).
- **`session-held` names its `holler release`.** The `-32011` data carries no hold key, so the client makes one more roster read, on that path only, to name the body session that advertises the id. If that read fails, the message falls back to the id.
- **The `--pane` mismatch message says "its body drives".** It used to say "the hub drives", which no longer matches what DRIVEN is: the session the pane's body advertises.

## Reuse / extend-vs-new

Everything follows the brief's Reuse map.

**Reused or extended:**
- `talk::send_turn` stays the one place `SayOpts` is built, and the gate key reaches every variant through it.
- `say`'s prelude moved into `talk::admit`; it was not copied.
- `LiveHandle::query`'s send-and-wait moved into the one `forward`.
- `Registry::replace_session` and `find_by_token`.
- `control_hold.rs` is the precedent for `control_rebind.rs` (one method, one file).
- `wiring/store.rs` is the precedent for `wiring/hub.rs`, and its `Effect`, `timed_out`, `hub_unavailable` and `undecodable` are reused, made `pub(super)`.
- `say_cmd::{QUEUE_ACCEPT_WAIT, wait_ended}` and `say_with` with the turn budget (E168).
- `tx_switch::check_idle`, extracted into `activity_refusal`.
- `findings.rs`'s builders and `with_fix`, with `switch_command` beside them.
- `fault::{FaultSwitch, PortOp}`, used for `FakeHub` with `HubOp`.
- `Error::invalid_grant` is the precedent for `Error::offline`.
- `ports/router.rs` and `wiring/bind.rs` are not edited.

**New, as "Where the code goes" lists:**
- `holler_pane::{gate, driven, hub}`;
- `live/target.rs`, `panes/gate.rs`, `control_rebind.rs` and `control_say.rs`;
- `pane/hub_refusal.rs` and `pane/wiring/hub.rs`;
- the test kit's `hub.rs` and `conformance/hub.rs`;
- `holler-proto`'s `rebind.rs`.

**New beyond that list:**
- `roster/rebind.rs`, for A's warn 1;
- `tx_switch/plan.rs`, the brief's own split at its 650-line trigger.

**Removed:**
- the hub's `pane_state_refusal` and the CLI's twin pane-state predicate, both replaced by `holler_pane::gate::prompt_refusal`;
- the twin pair rule in `roster_cmd`, replaced by `shown_driven_pair_differs`.

## Architecture notes for A

- **New modules and a port:**
  - `holler_pane::{gate, hub, driven}`;
  - `HubPort` is a new port and `Ports` has a new required field, `hub`. Every `Ports` literal changed; T migrated the test ones, and F changed `wiring.rs`.
  - The test kit has `FakeHub` and a hub conformance suite.
- **Dependency direction:** unchanged. `holler-hub` already depends on `holler-pane` (A confirmed this), and `holler-cli` on all three. There are no new crates and no new external dependencies.
- **Protocol (additive):**
  - a new hub-to-body request, `session/rebind` (the catalog has 23 rows), with three new goldens and no existing golden changed;
  - the optional `data.reason: "offline"` on the resolve-time `-32004` of `say`, `interrupt` and `answer`;
  - a new local-only control method, `control/rebind`, with no `admin/` form;
  - a new optional `check_only` param on `control/say` and `admin/say`.
- **One enforcement point is kept:** `send_prompt` in `circuit/dispatch.rs` is still the only builder of `session/prompt`. The engine's `HarnessPort::send_prompt` call is gone, so the only `.send_prompt(` calls left in pane and CLI code are the two pass-throughs.
- **Local patterns followed:**
  - the hub's out-of-line test modules end their file (`holds.rs`, `lockout.rs`);
  - a child module reaches a full file's private table (`roster/rebind.rs`, as `roster/rfc3339.rs` does);
  - new pane, test kit and CLI leaf files are rustfmt-clean, and the hub and body files keep their long-line style (no `cargo fmt --all`).

## Deviations from spec / wireframe

- **A's warns were all taken where the brief left a choice:**
  - **Warn 1:** `control/rebind` fills one roster row. The brief's whole-set `Roster::advertise` could race the presence handler and put a stale frame on the body's other rows. This adds `roster/rebind.rs` and 1 line to `roster.rs`, which the brief said would not grow and is not in its Blast radius. It is now 893 lines, under the 900 limit.
  - **Warn 3:** `interrupt` with text checks the pane gate before the cancel.
  - **Warn 5:** `parse_session_id` and `SESSION_ID_MAX` live in `findings.rs`, and `tx_switch` re-exports them, so no caller changed.
  - **Warn 7:** `control/say` lives whole in `control_say.rs`, rather than split between two files.
- **`tx_switch/plan.rs`:** `tx_switch.rs` would have been 683 lines, over the brief's 650 trigger, so the plan's helpers moved out. `tx_switch.rs` is 399 lines and `plan.rs` 305.
- **`live/tests.rs` is a pure move of `live.rs`'s inline tests**, done first as the brief says. A `diff` shows only the indentation changed. It is a test file that F did not author, so T owns and stages it.
- **`HubRefusal::Held` is `{kind, reason}`, without T's `key`:** the refusal's data carries no key. The tests match it with `..`.
- **`FakePaneStore::records()`** (`holler-pane-testkit/src/pane_store.rs`, not in the Blast radius) is a crate-private read that is not logged. It keeps `FakeHub`'s reads out of the store's call log, as T's contract requires.
- **`control/rebind` checks the id with `parse_session_id`** before any body is asked. The brief did not state this check; it fails closed with `-32602`.
- **`README.md`** (not in the Blast radius) has its debug-table row, as the stack rule requires for a new log event.
- **The pane gate's tests are `panes/gate/tests.rs`**, following T's contract, and `gate.rs` ends with `#[cfg(test)] mod tests;`. The brief named the file `panes/gate_tests.rs`.
- **Sizes:**
  - `say_cmd.rs` is 588 lines; the brief estimated "under 560".
  - `talk.rs` grew by 28 lines, under its +30 trigger for `talk/admit.rs`.
  - `session_manager/task.rs` is 794 lines, under 850, so the handler stays in the file.
  - `http_attach_driver*` needed no edit: the swap uses `attach` and `shutdown` as they are.
- **The new `--pane` mismatch wording,** "its body drives" (see Design decisions).

## Tier 1 self-check (incl. tests now GREEN)

```text
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.96s        (exit 0)

$ bash scripts/lint.sh                                                         (exit 0; size warnings only, none at 900)
lint: golden files with real value changes: 3 — reorder-only: 0
lint:     crates/holler-proto/tests/golden/docs/Rebind.json (new file)
lint:     crates/holler-proto/tests/golden/docs/RebindResult.json (new file)
lint:     crates/holler-proto/tests/golden/envelope/error.not_connected_offline.json (new file)

$ bash scripts/changelog-check.sh origin/main
changelog-check: ok

$ rustfmt --check --edition 2021 <the nine new pane, test kit, CLI and proto files>   all clean

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
2703 passed; 18 failed; 36 ignored, where the 18 are:
  15  T-1 to T-6 (below): cli_invocation 2, driven_e2e 9, hold_single_path 1, check_only 2, driven_resolve 2
   1  docs_cli_test::every_documented_holler_command_parses: five commands in F's doc text; fixed since:
      $ cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test   3 passed; 20 passed
   1  profile_rename_faults_test: an inode-reuse flake that predates #839 (Known issues)
```

**T's targets in that run:**
- **holler-proto:** the lib's `pane_methods_not_in_wire_catalog` passes, as do `golden_test` (14) and `codec_test` (62).
- **holler-pane:** `gate_test` 8, `driven_test` 14, `ports_test` 8.
- **holler-pane-testkit:** `fake_hub_test` 13.
- **holler-body:** the lib (107, `rebind_tests` included) and `rebind_test` 8. `rebind_test` is flaky; see T-5.
- **holler-hub:** the lib (89, with `hold_tests` 9 and `panes::gate::tests` 5) and `offline_reason_test` 3.
- **holler-cli:** `pane_verbs` 381, `pane_claude` 45, `pane_integration` 71 and `docs_pane_control_test` 20.

**The rest, with each wrong test temporarily fixed in the test only and then restored** (`git status -- crates/*/tests` is clean):
- `check_only_test` 4, `driven_resolve_test` 11, `offline_reason_test` 3;
- `cli_invocation_test` 14, `driven_e2e_test` 11, `hold_single_path_test` 5.

`driven_e2e_test` starts a real hub and a real body; this is the server-start check. `rebind_test` with T-5's fix applied ran 12 times with no failure.

## Evidence appendix

`docs/handoffs/839/evidence.md` has:
- the four facts in unchanged code that the implementation relies on (`Roster::advertise` replaces a whole set; the presence handler's order; `attach`'s one await before it spawns; `would_refuse` consumes nothing);
- the facts behind each wrong test and the #665 flake.

## Tests that look wrong (for T)

Each fix below was applied locally to the test alone, run green, and reverted. None needs a production change.

- **T-1: `driven_support::TestHub::body` never binds the token's label.** Its roster rows are named `oc`, not `kiwi/oc` (`roster.rs:853-858`), so `find_by_name("kiwi/oc")` and `row(hub, "kiwi/oc")` find nothing.
  - **Fails:**
    - `check_only_test::an_admitted_check_sends_logs_and_moves_nothing` (`:56`) and `say_and_the_check_refuse_the_same_cases_alike_before_anything_moves` (`:205`);
    - `driven_resolve_test::control_rebind_re_points_one_row_and_updates_the_cache_and_roster_first` and `a_body_s_refusal_is_passed_through_and_changes_nothing` (`:169`, "no row kiwi/oc").
  - **Fix:** in `crates/holler-hub/tests/driven_support/mod.rs`, after `self.registry.update_presence(&client_id, ads.clone()).await;`, add `self.roster.set_label(&token_id, label);`, as the circuit does at authentication.
- **T-2: `check_only_test::error_shape` compares a hold's `data.since` across two hubs.** Each `check_only` iteration builds its own hub and sets the hold again, so the two timestamps differ by microseconds. With T-1 applied it fails as `"an operator hold: say and the check refuse alike"`.
  - **Fix:** drop `since` before comparing:
    ```rust
    let mut data = error["data"].clone();
    if let Some(d) = data.as_object_mut() { d.remove("since"); }
    json!({"code": error["code"], "data": data})
    ```
- **T-3: `hold_single_path_test::the_pane_codes_and_the_pair_rule_are_defined_once` scans T's own `panes/gate/tests.rs`.**
  - **Why:** `sources()` says it reads "every non-test `.rs` file", but it cuts a file only at its first `#[cfg(test)]`. An out-of-line test module has no such line, so the expected values `"pane-unhealthy"` and `"pane-shown-driven-mismatch"` in `assert_frame` count as a second definition.
  - **Fix:** in `sources()`, after `rust_files(&root, &mut files);`, add `files.retain(|f| f.file_name().is_some_and(|n| n != "tests.rs"));`. This covers `holds/tests.rs`, `lockout/tests.rs`, `live/tests.rs` and `panes/gate/tests.rs`, and the other scans stay green with it. The alternative is to write those two expected values as `PromptRefusal::Unhealthy.code().as_str()` and `PromptRefusal::ShownDrivenMismatch.code().as_str()`.
- **T-4: `driven_e2e_test`'s `Rig::start` passes `hub_args` as the hub's whole serve-args list.** Its hubs start without `--listen 127.0.0.1:0` and bind the default `127.0.0.1:41807`. Nine of its eleven tests fail with `bind_failed ... Address already in use` on any machine where a hub runs; on this one, that is the operator's live hub, which was never contacted. Two rigs alive at once would collide the same way.
  - **Fix:** at `driven_e2e_test.rs:66`:
    ```rust
    let args: Vec<&str> = ["--listen", "127.0.0.1:0"].iter().copied().chain(hub_args.iter().copied()).collect();
    let hub = Hub::start_with_args(&state, &[], &args);
    ```
- **T-5: `rebind_test::until_attached` returns before the first attach.**
  - **Why:** an attach row is visible in presence before its task runs (T's own guard, `presence_advertises_the_configured_id_before_the_task_runs`), then hidden while the first attach runs (`task.rs:168-172`). So "visible" does not mean "attached", and the next step races the first attach.
  - **Three tests fail by that race, with the same cause:**
    - `the_refusals_keep_the_old_binding` (`:216`, `None`): it sets `exists` to false before the first attach has checked;
    - `a_restart_reverts_to_the_configured_id` (`:391`, `None`);
    - `a_silent_endpoint_is_refused_in_time_and_a_late_attach_is_never_swapped_in` (`:130`): the stall catches the first attach, so the rebind never sends its `GET`.
  - **Rate:** it failed 11 times in 12 as a single binary from the `-p` build and 4 times in 6 from the workspace build, and it passed in the full run. With the fix it failed 0 times in 12.
  - **Fix:** wait for the first attach's existence check, then for the row:
    ```rust
    async fn until_attached(sm: &SessionManager, server: &FakeServer, since: usize, session: &str, id: &str) {
        until_request(server, since, "GET", &format!("/session/{id}")).await;
        let deadline = Instant::now() + WAIT;
        while advertised(sm, session).is_none() { /* as today */ }
    }
    ```
    The seven callers pass `&server, 0, "alpha", OLD`. The restart takes `let since = server.requests().len();` before `start(&configs)` and passes it.
- **T-6: `cli_invocation_test::say_without_live_hub_exits_1` and `interrupt_without_live_hub_exits_1` (these predate #839) type `SESSION`.**
  - **Why:** uppercase is not a name (`vocab.rs:209-223`). Per the brief ("The CLI keeps the human form name-only (P3)": `route_target_by` checks a `PromptTarget::Session` with `RoutableName::parse` "and stops ... before any hub is contacted (with no hub running it now says `unknown session` rather than `no live holler hub reachable`)"), the CLI now prints `unknown session: SESSION`. The exit stays 1, but the stderr pin fails.
  - **Fix:** type a valid name (`session`) in the four `args` arrays (lines 328, 337, 387 and 397). With that, all 14 pass.

## Known issues

- **A behaviour change, as the brief decides (P3):** any typed `SESSION` that is not a valid name, not only a `ses_` id, is refused by the CLI as `unknown session: X`, exit 1, with no hub contacted. With no hub running, `holler say BAD TEXT` used to say `no live holler hub reachable`. T-6 is the pin that recorded the old text.
- **Until T-5 lands, `rebind_test` will fail CI intermittently.** The production code cannot remove the race, because T's guard requires the attach row to be visible before its task runs.
- **`profile_rename_faults_test::a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds` is a flake from #665 that predates this story.** It asserts the pane file's inode changed, and a filesystem may hand the same inode back.
  - Run alone, it fails 40 times in 40 on this branch and 20 in 20 on the tree before #839 (`9ecfc34`, built in a scratch copy). In the full suite, it usually passes.
  - This diff does not touch the rename or the store: `panes/mod.rs` changed by a doc line and `mod gate;`.
  - It needs an issue of its own; F files none.
- **Acceptance 20(d) row (1) still needs O's ruling** (T's note 1): the code follows D3's order, so a switch onto another pane's session of record is `session-of-other-pane`.
- **The live hub was not touched.** The unfixed e2e rig only failed to bind its port; every e2e run used an ephemeral port and a throwaway state directory.

## Files changed

**Production code:**
- `crates/holler-proto/src/{rebind.rs (new),lib.rs,methods.rs,error.rs}`
- `crates/holler-body/src/{session_manager.rs,session_manager/task.rs,prompt_dispatch.rs,connection.rs,connection/session_dispatch.rs,registry.rs}`
- `crates/holler-hub/src/{live.rs,live/target.rs (new),circuit.rs,circuit/dispatch.rs,panes/gate.rs (new),panes/mod.rs,talk.rs,interrupt.rs,control_say.rs (new),control_rebind.rs (new),control_server.rs,control.rs,roster.rs,roster/rebind.rs (new),lib.rs}`
- `crates/holler-pane/src/{gate.rs (new),hub.rs (new),driven.rs (new),lib.rs,ports.rs,findings.rs,tx_switch.rs,tx_switch/plan.rs (new),reconcile.rs,reconcile/observe.rs}`
- `crates/holler-pane-testkit/src/{hub.rs (new),conformance/hub.rs (new),conformance/mod.rs,fixture.rs,pane_store.rs,lib.rs}`
- `crates/holler-cli/src/{pane/hub_refusal.rs (new),pane/wiring/hub.rs (new),pane/wiring.rs,pane/wiring/store.rs,pane/mod.rs,pane/reset.rs,prompt_target.rs,say_cmd.rs,interrupt_cmd.rs,answer_cmd.rs,roster_cmd.rs}`
- `crates/holler-adapter-opencode/src/prompt.rs`

**Docs:**
- `docs/adr/{ADR-0021,ADR-0014,ADR-0005,ADR-0003}.md`
- `docs/protocol/v2.md`
- `docs/pane-control.md`
- `CHANGELOG.md`
- `README.md`

**Moved, not authored (T stages it):** `crates/holler-hub/src/live/tests.rs`

**Pipeline scratch:** `docs/handoffs/839/{handoff-F.md,evidence.md (new),decisions.md (appended)}`
