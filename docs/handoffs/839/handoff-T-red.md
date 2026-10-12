# Handoff-T-red: Phase 4 - #839 DRIVEN follows each pane's session of record

**Date:** 2026-10-12 (07:58 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`, head `0be85be`; nothing committed by T)
**Brief / wireframe reviewed:** `docs/handoffs/839-brief.md` (decisions, ADR text, placement, Reuse, Acceptance 1 to 22, Blast radius, and the Evidence entries the tests touch); `docs/handoffs/839/handoff-A.md` (PASS, 7 warns). No wireframe (no UI).

**VERDICT: RED is valid.** It is compile-level for the contract, as #831's and #802's were, plus assertion-level for every test that compiles today. Every compile error names a contract item of the brief (a module, type, field, variant, method or constant that does not exist yet). None is a typo, a missing `[[test]]`, or a wrong path. To be sure of that, T type-checked the whole suite against a throwaway stub of the contract (signatures only, `todo!()` bodies): every test target compiled and `cargo clippy --workspace --all-targets -- -D warnings` was clean. The stubs were then removed and `src/` restored. `bash scripts/lint.sh` exits 0.

## A precondition

Confirmed: A returned PASS on the plan (fourth pass), 0 `block`, 7 `warn`. What the suite does with the warns:

- **Warn 4 (named for T): taken.** `switch_moves_the_tui_and_the_record_together` now expects DRIVEN `None` (no body, so the stale value goes) and `[HubOp::Advertised]` on the hub. `assert_recorded` expects DRIVEN `None` and is renamed "exactly those fields". `reset_creates_a_fresh_session_and_switches_to_it` keeps its harness `[Health, CreateSession, SelectSession, ShownSession]` and store `[Get, CasPut]` pins and adds `[HubOp::Advertised]`. That pins A's suggestion: **a reset reads `list_sessions` and `pane_store.list` only when `advertised()` returns a body** (a switch already reads both).
- **Warn 3: taken.** `check_only_test.rs::a_redirect_the_pane_gate_refuses_cancels_nothing` sends `control/interrupt` with text to a body that drives a parked pane. It must be refused `-32011 hold_kind: pane`, and the body sees no cancel. This is about +3 lines in `interrupt.rs`, beside `would_refuse`. **O: say whether to keep it.** It is a pin beyond the brief's 22 acceptance items, and it comes from A's warn.
- **Warn 1:** `control_rebind_re_points_one_row_and_updates_the_cache_and_roster_first` asserts that the body's other row keeps its `state` (`working`), its id and `connected`. Both A's one-row forward-fill and the brief's whole-set `advertise` pass it in a quiet test. The race itself has no deterministic test.
- **Warn 6:** the forward scan counts `self.forward(` in `live.rs` and `live/target.rs` only, which is A's first option. It never meets `pane_dispatch::forward` or `send_query_forward`.
- **Warn 2** is the operator's (P8 row (4)). `driven_test.rs` pins row (4) as `One`, as P8 decided.
- **Warns 5 and 7** are F's: where `switch_command` validates, and the `control_say.rs` split. The tests call `holler_pane::findings::switch_command`, and nothing else depends on either choice.

## The contract the tests pin (F implements exactly these names)

### `holler-proto`
- `holler_proto::rebind::{Rebind, RebindResult}`, each `{ session: String, harness_session_id: String }`. Serde.
- The catalog has 23 rows: `session/rebind` is `Request`, `HubToBody`.
- `holler_proto::error::OFFLINE_REASON` is `"offline"`. `WireError::offline(message)` builds `-32004` with `data.reason: "offline"`.
- Golden files, written by T and checked against the stub's encoder:
  - `tests/golden/docs/Rebind.json`
  - `tests/golden/docs/RebindResult.json`
  - `tests/golden/envelope/error.not_connected_offline.json` (its message is `alpha's body is not connected`)

### `holler-pane`

**`gate`:**
- `PANE_PARKED`, `PANE_UNHEALTHY`, `PANE_SHOWN_DRIVEN_MISMATCH` (`RefusalCode`).
- `PromptRefusal { Parked, Unhealthy, ShownDrivenMismatch }`. It is `Debug + Copy + Eq` and has `.code() -> RefusalCode`.
- `prompt_refusal(&Pane) -> Option<PromptRefusal>`
- `shown_driven_pair_differs(&LastObserved) -> bool`
- `first_refusal<'a>(&'a [Pane], id: &str) -> Option<(&'a Pane, PromptRefusal)>`

**`hub`:**
- `Advertised { token, name, harness_session_id: String, state: Activity }`, which is `Debug + Clone + Eq`.
- `PromptMode { Check, Send }`, which is `Debug + Copy + Eq`.
- `trait HubPort: Send + Sync`, with the three methods of D3 and no defaults.
- `PANE_NOT_DRIVEN` and `SESSION_HELD`.

**`driven`:**
- `Driver<'a> { None, One(&'a Advertised), Ambiguous(Vec<&'a Advertised>), Unproven(Vec<&'a Advertised>) }`, which is `Debug + Eq`.
- `driver_of(pane, advertised, server_sessions: &[String], others: &[Pane], target: Option<&str>)`. `others` excludes the pane itself.
- `elsewhere(advertised, driver: Option<&Advertised>, target) -> Vec<&Advertised>`

**`Ports.hub`:** `&'a dyn HubPort`.

**`tx_switch`:**
- `activity_refusal(Activity) -> Option<(RefusalCode, &'static str)>`
- `SESSION_DRIVEN_ELSEWHERE`

**`findings`:** `switch_command(&PaneName, &str) -> Option<String>`, which is `None` for an id that fails `parse_session_id`.

### `holler-pane-testkit` (`src/` is F's)

**`fixture::session_id(n: u32) -> String`:** `ses_{n:026x}`.

**`hub`:**
- `HubOp { Advertised, Rebind, Prompt }`. It implements `PortOp`, as `hub.advertised`, `hub.rebind` and `hub.prompt`.
- `HoldKind { Operator, Default }`, which is **`Copy`**.
- `HubCall { op, id, mode: Option<PromptMode>, of_record: bool }`
- `SentPrompt { id, text, of_record }`
- `FakeHub` has:
  - `new(Arc<FakePaneStore>)`;
  - the `&self` scenario methods `with_body(token, name, id, Activity)`, `set_state(name, Activity)`, `hold(name, HoldKind)`, `release(name)` and `fail_send(PaneError)`;
  - the readers `faults()`, `bodies()`, `log()` (its `Rebind` and `Prompt` entries only) and `prompts()`.

**What `FakeHub` must do** (the behaviour `fake_hub_test.rs` pins):
- **Its order of judgement:** resolve the id (none or two bodies is `pane-not-driven`), then a hold (`session-held`, the message naming `holler release <name>`), then `first_refusal` over the store (the pane's own code), then a held question or permission (`session-busy`). A busy body is admitted, because the wiring sends `queue: true`.
- **`fail_send`** refuses the next `Send` only.
- **A hold** stops prompts, not a rebind.
- **Its reads of the store stay out of the store's call log.**

**`conformance::hub`:**
- `HubFixture { panes: Arc<dyn PaneStore>, ids: [String; 2] }`. Each id is advertised by one live, idle attach body with no hold, and the store starts empty.
- `run_hub_conformance(fresh: FnMut() -> (S: HubPort, HubFixture, K)) -> Conformance`
- `hub_cases()`, the 7 ids in `fake_hub_test.rs::DOCUMENTED_CASES`. The cases are F's, one per id:
  1. `advertised` names both ids.
  2. A healthy pane on `ids[0]` is admitted by `Check`.
  3. A parked pane on its session of record is refused `pane-parked`, naming the pane.
  4. A pane whose session of record is an id no body advertises, and whose recorded DRIVEN is `ids[0]`, is refused `pane-shown-driven-mismatch`.
  5. A healthy pane and an unhealthy pane both on `ids[0]`: `pane-unhealthy`, naming the bad one.
  6. Two bad panes: the first in name order is named.
  7. An id no body advertises is `pane-not-driven`.

### `holler-hub`
- `Registry::resolve_prompt_target(&self, &str) -> ResolveOutcome` (async).
- `LiveHandle::rebind(&self, params: Value, timeout) -> Option<Result<Value, WireError>>`
- `holler_hub::control_rebind::REBIND_FORWARD` is 8 s.
- `ControlCall::rebind(token_id, session, harness_session_id)`: method `control/rebind`, params exactly `{token_id, session, harness_session_id}`, timeout 10 s.
- `ControlCall::say_check(session, text)`.
- `HoldGate.harness_session: Option<&'a str>`.
- `pub(crate) fn pane_gate(&PaneState, id: &str) -> Result<(), WireError>` in `panes/gate.rs`. Its tests are `panes/gate/tests.rs`, and `gate.rs` ends with `#[cfg(test)] mod tests;`. The brief says `panes/gate_tests.rs`. A child module of `gate.rs` lives in `gate/`, as `holds.rs` keeps `holds/tests.rs`, so the brief's path would need a `#[path]`.

**Wire answers:**

| Request | Answer |
|---|---|
| `control/say` with `check_only: true`, admitted | `{"admitted": true}` |
| `control/rebind` with `token` instead of `token_id` | `-32602` |
| `control/rebind` for a session not in the body's cached presence | `-32003` |
| `control/rebind` with an unknown `token_id` | any error, with no body asked |

### `holler-body`
- `prompt_dispatch::handle_rebind(Arc<SessionManager>, id: String, Rebind, UnboundedSender<WsMessage>)`. It is the same shape as `handle_answer` and writes one frame.
- `prompt_dispatch::REBIND_DEADLINE` (5 s) and `REBIND_REPLY_WAIT` (6 s).
- `SessionManager::live_configs(&self, &[SessionConfig]) -> Vec<SessionConfig>`
- `SessionCommand::Rebind { harness_session_id: String, deadline: tokio::time::Instant, reply_tx: oneshot::Sender<Result<String, WireError>> }`, handled through `dispatch_command`. A command dequeued after its deadline answers an error and changes nothing.
- `SessionPresence.harness_session_id: Option<String>`
- `rebind_tests.rs` is declared by T as the last line of `session_manager/task.rs`.

**Refusal frames:**

| Case | Frame |
|---|---|
| An unknown name | `-32003` |
| A spawn session | `-32602` with `data.reason: "not an attach session"` |
| An id the endpoint does not have | `-32602` with `"session not found on the endpoint"` |
| A turn in flight | `-32009` with `data.state` |
| A silent endpoint | `-32004` with `"endpoint did not answer"`, in 4 to 7 s |

### `holler-cli`
- `holler_cli::pane::hub_refusal::{classify, HubRefusal}`. `HubRefusal` is `Debug` and has the variants:
  - `NotDriven(String)`
  - `PaneGate { code: String, message: String }`
  - `Held { kind: String, reason, key }`
  - `Busy(String)`
  - `Other`
- `holler_cli::pane::wiring::hub::{HubClient, send_call, SEND_TURN_BUDGET}`:
  - `HubClient::new()` resolves the socket from the environment, as `HubPaneStore::new()` does;
  - `send_call(id, text) -> ControlCall` gives `timeout_ms` 600000, `queue: true`, no `check_only`, and `call.timeout == QUEUE_ACCEPT_WAIT`.
- `Routed.pane: Option<String>`, which is `None` for a SESSION.
- `Unwired` implements `HubPort`, every method answering `not-implemented`.
- `route_target` refuses a typed bare id: `Stop { message: "unknown session: <id>", exit_code: 1 }`, before any hub is contacted.

## Tests authored

| File | Tests | Pins (Acceptance) | Tier | Why this tier |
|---|---|---|---|---|
| `holler-pane/tests/gate_test.rs` (new) | 8 | 4, 14: the predicate, the pair rule, `first_refusal`'s two keys, fail closed, name order | unit | pure functions |
| `holler-pane/tests/driven_test.rs` (new) | 14 | 6, 20 (a to e), P6; `activity_refusal` (8); `switch_command` | unit | `driver_of` and `elsewhere` are pure: the table belongs here |
| `holler-pane-testkit/tests/fake_hub_test.rs` (new) | 13 | D10; 19 (the conformance suite over `FakeHub`) | unit (fake) + conformance | the verbs' tests rely on the fake meaning what the hub does |
| `holler-proto/src/methods.rs` (count test) | 1, changed | 11: 23 rows | unit | the existing pin |
| `holler-proto/tests/golden_test.rs` | +1 test, +2 instances | 11: the three new goldens | golden | wire shape |
| `holler-proto/tests/codec_test.rs` | +1 arm | 11 (`every_method_round_trips` needs it once the row exists) | codec | existing table |
| `holler-hub/src/circuit/dispatch.rs` `hold_tests` | 9 (6 rewritten) | 4: the enforcement point, on `ses_` ids and the new key; the name is not a key; spawn not gated; the DRIVEN key and its window | unit | `HoldGate` and `Recording` are private to `circuit` |
| `holler-hub/src/panes/gate/tests.rs` (new) | 5 | 4, 14: `pane_gate`'s frame, the DRIVEN key, first bad of two, unreadable store | unit | `pane_gate` is `pub(crate)` |
| `holler-hub/tests/driven_resolve_test.rs` (new) + `driven_support/` | 11 | 5, 9 (hub), 13 (hub), 21 | integration (real control socket, fake bodies on the registry's channels) | the slot and the cache are hub state; no process needed |
| `holler-hub/tests/check_only_test.rs` (new) | 4 | 10, A's warn 3 | integration | as above |
| `holler-hub/tests/offline_reason_test.rs` (new) | 3 | 18 (wire half, P7) | integration | as above |
| `holler-body/tests/rebind_test.rs` (new) + fake stall mode | 8 | 9 (body): the happy path, the refusals, I5, a late attach dropped, one source after `session.error`, `query/support`, restart reverts, the first moment | integration (real `SessionManager` and fake OpenCode server) | the driver and the session task are real |
| `holler-body/src/session_manager/task/rebind_tests.rs` (new) | 2 | 9: the 30 s retry path re-attaches the new id; past-deadline command changes nothing | unit | the retry timer is 30 s, so `attempt_initial_attach` is called directly |
| `holler-cli/tests/pane_verbs/driven.rs` + `driven/{order,first,evidence,doctor}.rs` (new) | 34 | 1, 2, 6, 7, 8, 12, 14, 15, 18 (the classification, `Routed`, the `Send` call), 19, 20 | integration (verbs over the fakes and `FakeHub`) | the engine's order, plan refusals and records through the verbs |
| `holler-cli/tests/pane_verbs/{switch,reset}.rs` | 2 rewritten (E83, E84), 2 pins updated | 1, 2, 7 and A's warn 4 | integration | existing tests whose pins this story changes |
| `holler-cli/tests/pane_claude/driven.rs` (new) + `lifecycle.rs` (E91 changed) | 3 | 22 | integration | a Claude pane's rig is in this target |
| `holler-cli/tests/driven_e2e_test.rs` (new `[[test]]`) | 11 | 3, 12, 13, 18 (i to iii, a typed id with no hub), 19 (the conformance suite over the real client, the gate on a recorded DRIVEN), 21, the bounds nesting | e2e (real hub, real body, fake OpenCode server) | the circuit's one reply slot and the process-level exits |
| `holler-cli/tests/hold_single_path_test.rs` | +3 | 13 (the forward's shape, the two `.send_prompt(` pass-throughs), 14 (no second copy of the codes, `roster_cmd` calls the rule) | source scan | as E77 |
| `holler-cli/tests/docs_pane_control_test.rs` | +1 | 17 | docs | as its neighbours |
| `holler-cli/tests/pane_integration/scenario.rs` | step 3 rewritten | 16 (D9) | opt-in scenario | compiled by CI, run by hand |

**Mechanical test edits** (no assertion changed):
- `Ports.hub` is added to every test `Ports` literal and rig:
  - `pane_verbs/{park/rig,close/rig,list,launch/rig}.rs`, `profile_verbs/{rig,apply/rig}.rs`, `roster_panes_test.rs` and `pane_integration/{observe,bind}.rs` use `&Unwired`, because none of them reaches the hub;
  - `doctor/rig.rs` and `pane_claude/rig.rs` get a `FakeHub` field and log `hub` calls;
  - `verb_harness::unwired_ports` gets `hub`;
  - `holler-pane/tests/ports_test.rs` gets a local `TestHub`.
- `pane_verbs/main.rs`'s `Unwired` test also checks the hub port.
- The fake OpenCode server gains a stall mode (`set_stall_exists`, with a `struct_excessive_bools` allow linked to #839).

**F's mechanical step:** `src/pane/wiring.rs`'s `Ports` literal.

## RED confirmation

**Assertion-level** (compiles today):

```text
$ cargo test -p holler-proto --lib methods::
pane_methods_not_in_wire_catalog FAILED: the v2 wire catalog is closed  left: 22  right: 23

$ cargo test -p holler-hub --test check_only_test
an_admitted_check_sends_logs_and_moves_nothing FAILED   left: {elapsed_ms, message, session: "kiwi/oc", ...}  right: {"admitted": true}
a_check_consumes_no_grant FAILED                        (the same: the check was sent as a real say)
say_and_the_check_refuse_the_same_cases_alike_... FAILED  "parked (check_only false) is refused": say delivered it
a_redirect_the_pane_gate_refuses_cancels_nothing FAILED  (the redirect was cancelled and delivered, not refused)

$ cargo test -p holler-hub --test offline_reason_test
say_interrupt_and_answer_to_an_offline_body_are_marked_offline FAILED  left: (-32004, None)  right: (-32004, Some("offline"))
say_s_timeout_and_cancelled_turn_stay_bare ... ok                (guard: must stay bare)
interrupt_s_unconfirmed_cancel_and_failed_redirect_stay_bare ... ok (guard)

$ cargo test -p holler-cli --test hold_single_path_test
the_query_forward_is_built_once_and_called_by_query_and_rebind_only FAILED  left: "query" right: "forward"
send_prompt_is_called_only_by_the_two_pass_throughs FAILED  left: [..router.rs, ("holler-pane/tx_switch.rs", "prompt"), ..bind.rs]
the_pane_codes_and_the_pair_rule_are_defined_once FAILED   (codes spelled in circuit/dispatch.rs and say_cmd.rs)
(the two existing scans stay green)

$ cargo test -p holler-cli --test docs_pane_control_test
the_sentences_driven_replaced_are_gone_and_the_entry_is_written FAILED on the 11 old sentences
  (ADR-0021 x5, pane-control.md x2, CHANGELOG.md, tx_switch.rs, reconcile.rs, methods.rs);
  the section-9 codes and the CHANGELOG entry are checked after them
```

**Unchanged, and green:**
- `holler-proto --test codec_test` (62 passed);
- `holler-body --test http_attach_driver_test` (14 passed);
- `attach_mode_test` and `debug_trace_test` build (they include the changed fake).

**Compile-level** (`cargo test -p <crate> --test <target> --no-run`). The unique errors, all contract items:

**`holler-proto` `golden_test`:**
- no `holler_proto::rebind`;
- no `error::OFFLINE_REASON`;
- no `WireError::offline`.

**`holler-pane`:**
- `gate_test`: no `holler_pane::gate`.
- `driven_test`: no `holler_pane::{driven, hub}`, `tx_switch::activity_refusal` or `findings::switch_command`.
- `ports_test`: no `holler_pane::hub`.

**`holler-pane-testkit` `fake_hub_test`:**
- no `holler_pane::{hub, gate}`;
- no `holler_pane_testkit::{hub, conformance::hub}`;
- no `fixture::session_id`.

**`holler-hub`:**
- lib test: `HoldGate` has no field `harness_session` (4).
- `driven_resolve_test`:
  - no `Registry::resolve_prompt_target` (12);
  - no `LiveHandle::rebind` (2);
  - no `ControlCall::rebind`;
  - no `holler_hub::control_rebind`.

**`holler-body`:**
- lib test:
  - no `SessionCommand::Rebind` (2);
  - no `SessionPresence.harness_session_id` (2).
- `rebind_test`:
  - no `holler_proto::rebind`;
  - no `prompt_dispatch::{handle_rebind, REBIND_DEADLINE, REBIND_REPLY_WAIT}`;
  - no `SessionManager::live_configs`.

**`holler-cli`:**
- `pane_verbs`:
  - `Ports` has no field `hub` (10 rigs and literals, and `verb_harness`);
  - no `holler_pane::{hub, gate}`;
  - no `holler_pane_testkit::{hub, fixture::session_id}`;
  - no `holler_cli::pane::{hub_refusal, wiring::hub}`;
  - no `Routed.pane`;
  - no `WireError::offline`.
- `pane_claude`: `Ports.hub`, `holler_pane_testkit::hub`.
- `driven_e2e_test`:
  - no `ControlCall::rebind`;
  - no `prompt_dispatch::REBIND_*`;
  - no `fixture::session_id`;
  - no `conformance::hub`;
  - no `holler_pane::hub`;
  - no `control_rebind`;
  - no `wiring::hub`.
- `pane_integration`, `profile_verbs` and `roster_panes_test`: `Ports.hub` only.

**What the RED does not prove:**
- **The stub type-check proves the suite compiles against the contract as written. It does not prove that each behavioural assertion fails before F's code exists.** Most of them sit behind the compile errors.
- **Guards.** These tests pass on today's behaviour and must stay green:
  - `presence_advertises_the_configured_id_before_the_task_runs` (the mirror must be initialised at `start`);
  - `the_routed_verbs_refuse_by_the_one_predicate` (the CLI and the predicate agree);
  - the two bare-`-32004` offline cases;
  - `the_hub_s_rebind_bound_is_below_the_cli_s` (the constants).
- **T-green spot-checks** that removing a behaviour fails its test. The order (rebind before `shown_session`, `Send` before `cas_put`), the exclusion rule, the recorded-DRIVEN key and the late-attach drop come first.

## Not authored, and why

- **`admin/query` carrying `session/rebind`** from an admin-role connection, Acceptance 13. The hub maps it to the same `hub_query_remote` (E99), which `control/query_remote` is tested through, in process and end to end. A remote-admin rig would add a second process pair for the same function.
- **`scripts/golden-diff-summary.sh`'s output**, Acceptance 11. It is informational and not edited. S reads its output: three `(new file)` lines.
- **The ADR, `v2.md`, guide and CHANGELOG edits themselves.** The contract amendment (scope 1, "one commit, before any test") is **not on the branch yet**, so `docs_pane_control_test` fails on today's docs. It passes once F (or O) lands the amendment the brief gives verbatim.
- **The race of A's warn 1** (a presence frame between the cache read and the roster write) is not deterministic in a test.

## Notes for F and O

1. **Acceptance 20(d) row (1) is inconsistent within the brief.** It says P1's switch onto the id of a body on P2's **session of record** is `session-driven-elsewhere`. D3 step 1 runs "the existing refusals; then" the driver, and `check_unclaimed` refuses that target `session-of-other-pane` first (E137). The suite follows D3's order:
   - `a_switch_onto_another_pane_s_record_is_refused_by_the_existing_rule_first` expects `session-of-other-pane`;
   - row (2), on P2's recorded DRIVEN, expects `session-driven-elsewhere`.

   If O wants row (1) to be `session-driven-elsewhere`, the plan order changes and T changes that one test.
2. **`panes/gate/tests.rs`, not `panes/gate_tests.rs`** (see the contract above).
3. **The reset's lazy reads (A's warn 4).** With no body advertised, a reset's plan reads neither `list_sessions` nor `pane_store.list`.
4. **`check_only` is read with `queue` as sent**, and the tests send `queue: true` for the admitted case, as the wiring does.

## Ready for F

RED is valid: at compile level for the contract, and at assertion level for the hub's `check_only`, the offline reason, the catalog count, the three source scans and the docs. F may implement against these tests in the brief's order: contract, proto, body, hub, pane, test kit, then CLI.

`T-red complete, RED is valid. F may implement against the authored tests.`
