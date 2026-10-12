# [feat(pane): DRIVEN follows each pane's session of record: the hub joins a pane to the body that drives it, the gate keys on the join, switch and reset move it, and `reset --first` goes through the hub's gate](https://github.com/Performant-Labs/holler/issues/839)

Part of epic #633. This is the complete statement of story #839, prepared on 2026-10-11 against `main` at `bda1069` (every line number below was re-checked there; "Line numbers re-checked" says which of the issue's moved). It **absorbs #828** (closed): the issue text is the source of truth where this brief is silent, and this brief is the decision where they differ (the differences are listed in "Where this brief is the decision"). The story is **contract amendment first** (ADR-0021 section 11 and the passages listed in "The ADR and the dependent docs", then `docs/protocol/v2.md`), then tests (RED), then code. Rigor is `in-session`: no outside model reviews it, so the Evidence section quotes every source fact a decision rests on, and a reviewer needs no other source. Cite an entry as E<n>.

**Status of the decisions.** The five questions the issue asks (the address, how a body follows, who records DRIVEN and when, hold keys, the order of `reset --first`) and the decisions that follow from them (D1 to D12) were **decided by the preparer under the operator's standing rules ("decide, don't ask"), to be confirmed at the plan check**. Each lists what it rejected and why. "Questions only the operator can answer" lists the three the preparer did not decide.

## Where this brief is the decision

**The facts that force the design** (each is in the Evidence section):

- F1. The hub's prompt path addresses a body session by its ADR 0005 name and by nothing else: `talk::say`, `talk::answer` and `interrupt` each parse the target as a `RoutableName` and match it against `ad.name` (E9, E11, E12, E7). An OpenCode session id never parses (E39: lower-case letters, digits and `-`; the id has `_`).
- F2. `say --pane` sends the pane's `session_of_record` as that target (E21), so for a real pane the hub answers `unknown session` today.
- F3. The pane gate matches `pane.session_of_record == the name being sent` (E2, E4). For a real pane that never matches; it fires only for #646's synthetic `ses-demo-*` fixtures (E81, E82).
- F4. A body's harness session is configuration, fixed when the body starts (E44, E45), advertised as `harness_session_id` (E40, E46). The closed v2 catalog has no request that changes it (E42, E43). The hub runs no adapters, and a prompt reaches a body only through `send_prompt` (E3, E77).
- F5. Nothing writes `last_observed.driven` (E32, E34). The engine sends `reset --first` straight to `HarnessPort::send_prompt` (E31), and `Ports` has no port that faces the hub's live sessions (E29).
- F6. Holds are keyed `<label>/<session>` (E15), and a hub started with `--join-held` holds a name on first sight (E17).

**Decided, and what each rejected.** (All "decided by the preparer, to be confirmed at the plan check".)

**D1. The address: the advertised `harness_session_id`, joined to `Pane.session_of_record` by string equality.**
- The hub resolves a prompt target that is not a valid name as a harness session id, against the live attach sessions, for `say`, `interrupt` and `answer` alike: one new function `Registry::resolve_target(&str) -> ResolveOutcome` (a target that parses as a `RoutableName` is resolved by `resolve_session`, exactly as before; any other string matches live ads with `mode == attach` and `harness_session_id == Some(target)`; no match is `Unknown`, or `NotConnected` when an offline ad had it; two matches are `Ambiguous`: the outcomes of E8). The three callers replace their two lines (parse, then `resolve_session`) with one call. Names and ids cannot collide where the grammar separates them: an OpenCode id is `ses_` and 26 characters (E96, the doc of `parse_session_id`), and `_` is not a name character (E39). An id that is also a valid name (the v2 document's example `openc-abc`, E67) is resolved as a name; it is not routable by id, and the ADR says so.
- The pane gate is keyed on the **advertised id of the session being prompted**, however the prompt addressed it: `SayOpts` and `LiveCommand::Say` carry `harness_session: Option<String>` (set in `send_turn` from `ad.harness_session_id`, E10), `HoldGate` carries it, and `pane_gate` matches `pane.session_of_record.as_deref() == Some(id)`. The name-keyed match of E2 is **removed**, not kept beside the new one. A bare `say alpha` to a body session that drives a parked pane's session of record is therefore refused too, which is what the pane hold's doc already claims ("whatever route the prompt takes", E68).
- One predicate, not three. `holler_pane::gate::prompt_refusal(&Pane) -> Option<PromptRefusal>` (`Parked`, `Unhealthy`, `ShownDrivenMismatch`, checked in that order, one side absent is not a mismatch) replaces the hub's copy (E1) and the CLI's (E22); each keeps its own message text, and the pair test it uses, `holler_pane::gate::shown_driven_pair_differs(&LastObserved) -> bool`, is public (#648's roster would otherwise write it a third time, E93). The three code constants move to `holler_pane::gate` and `say_cmd` re-exports them (E20). `holler-pane` may change because this brief amends the contract first (the comments in E1 and E22 say it is frozen "except through the amend-first rule").
- Rejected: **(a) join on the pane's name** (a pane's name equal to its body session's name). Nothing on `main` establishes it: the wizard names attach sessions `alpha` and `beta`, independent of any pane (E88), and a pane's name is its tmux session's name (E94). It would make the join a naming rule the cutover must impose on a live fleet, and a miss would be silent. **(b) a new `Pane` field naming the body session**: a second source of truth beside the advertisement, a change of the stored record (`deny_unknown_fields`, the byte-compatibility test of E78), and `import` would have to fill it. **(c) a new wire request to ask a body which session it drives**: presence already carries it (ADR 0014 point 6, E51). **(d) keep the name match beside the id match**: two keys with different meanings, and the only inputs that match the old one are test fixtures.

**D2. How a body ends up on the pane's new session: the hub asks it to re-point, with a new additive request `session/rebind`. I5 bound: 5 s in the body, 10 s at the hub and at the CLI.**
- Wire: `session/rebind`, a request, hub to body. Params `{session, harness_session_id}` (`session` is the body's bare session name); result `{session, harness_session_id}`. It is **additive**: the catalog goes from 22 to 23 rows, and `Rebind` and `RebindResult` get golden files that are new files (no existing golden moves).
- The body re-points **one attach-mode session of its own** to another harness session **on the same endpoint**, fail closed: a name that is not a session of this body is `-32003 unknown_session`; a spawn-mode session is `-32602 invalid_params` (`data.reason: "not an attach session"`); a session with a turn in flight, a queued prompt or a pending question is `-32009 session_busy` (with the existing `state`, `turn_age_ms` and `last_update_age_ms`); an id the endpoint does not have (the same `GET {endpoint}/session/{id}` existence check as `attach`, bare route then `/api`, E50; never `POST /session`) is `-32602 invalid_params` (`data.reason: "session not found on the endpoint"`) and the old binding is kept. On success the driver targets the new id, the session's presence advertises it, and the result returns it. The body's `hello` row also carries `harness_session_id` (E41), but it is sent at connect and not again; presence is what the roster and the resolver read (E7, E40), so the rebind changes presence and leaves the hello alone. The session's **name does not change** (ADR 0005 section 3, E52: the name is the identity, the locator changes; ADR 0014 point 5, E51).
- Bound (I5). The attach driver builds its HTTP client with `reqwest::Client::new()`, which sets no timeout (E97), so the body wraps the whole handler in `tokio::time::timeout(5 s)`. On expiry the body answers `-32004 not_connected` with `data.reason: "endpoint did not answer"` (no catalog code means "the endpoint hung"; this is the one the v2 table already uses for "the party asked about cannot be reached", E70's neighbouring row) and keeps the old binding. The hub's forward (10 s) and the CLI's call (10 s) are then never the first to expire.
- How the body swaps is the implementor's, if the observable behaviour above holds. The expected way: build the new driver first with `HttpAttachDriver::attach` on a copy of the session's config with the new id (that is the existence check, E97), swap it into the session task, then shut the old driver down (`shutdown` ends only the body's own loop and touches nothing on the harness, ADR 0014 point 3, E51); `presence_doc` must read the id from the swapped state, not from the immutable `managed.config.session_id` (E46).
- Hub: the forward rides `LiveHandle::query` (E6, the generic request forward `answer` already uses; E11): its allowlist (`query/*` and `session/answer`) adds `session/rebind`, one condition on the same line, and its comment says why a rebind may pass (it starts no turn). A new hub control method `control/rebind {token, session, harness_session_id}` in a **new file** `crates/holler-hub/src/control_rebind.rs` (the precedent is `control_hold.rs`) resolves the body by `token_id` (a roster field, E18) and the bare session name, forwards with a 10 s bound, passes a body refusal through verbatim, and **on success overwrites the cached ad (`Registry::replace_session`) and re-advertises the roster row (`Roster::advertise`) before it replies**, so a `control/roster` or a `resolve_target` made right after the reply already sees the new id (nothing waits for the next presence beat). The dispatch arm is one line in `dispatch_session_control` (E13's file). On the body, `dispatch_session_request` (E49) gets a fourth arm and its caller's `matches!` list (E48) a fourth name; the session task's mailbox (E47) gets a `Rebind` command beside `Answer`.
- Scope limit, stated in the ADR: only the HTTP attach driver implements it. ACP and spawn sessions refuse. A body for a Claude Code pane (#850) implements the same request in its own driver kind (a relaunch of the process); nothing here assumes OpenCode beyond the existence check inside the HTTP attach driver.
- Rejected: **(a) restart the body with a new config**: not bounded by I5, needs the CLI to supervise another process and rewrite a file the wizard owns (E88), and ADR 0014 point 3 says a body must be able to leave without touching the harness (E51). **(b) a new body session per `ses_`**: a new name is a new hold key, so `--join-held` would default-hold every reset (E17) and operator holds would not carry over; the roster would grow by a row per reset; and ADR 0005 section 3 says the name is the stable identity (E52). **(c) the hub or the CLI posts the prompt to the pane's OpenCode server itself**: that is an adapter in the hub (ruling 1, section 5 of the ADR, E56), it skips the body's turn tracking, and it is a second way to deliver a prompt, which the single-sender guard exists to prevent (E77). **(d) leave it to the cutover**: the story's own acceptance (a healthy pane's `reset --first` is delivered) cannot hold in production, and after every switch the body would still drive the old session, so the gate would refuse everything as `pane-shown-driven-mismatch`. **(e) hot reload of the body's config file**: the body has no reload path and the file would become a control channel.

**D3. Who records `last_observed.driven`, and where in the step order: the verb and reconcile record it from what the body acknowledged or advertised; the hub records nothing.**
- A new port, `HubPort` (`holler-pane`, new file `crates/holler-pane/src/hub.rs`), with three **required** methods (no defaults: #835 made `observe_pane` and `foreground_commands` required for exactly the reason that a default answers silently for a wrapper that forgot to forward, E92) and `Ports` gains `pub hub: &'a dyn HubPort` (E29; `Wiring::ports` lends the real ports, E25, and `Unwired` is the not-implemented set, E26; `PaneStore`, E27, gets no new method):
  ```rust
  pub struct Advertised { pub token: String, pub name: String /* "label/session" */, pub harness_session_id: String, pub state: DriverState }
  pub enum DriverState { Idle, Working, InputRequired }
  pub enum PromptMode { Check, Send }
  pub trait HubPort: Send + Sync {
      /// The live attach-mode sessions the hub knows, with the id each advertises.
      fn advertised(&self) -> Result<Vec<Advertised>, PaneError>;
      /// Ask the body behind `driver` to drive `harness_session_id`; returns the id it acknowledged.
      fn rebind(&self, driver: &Advertised, harness_session_id: &str) -> Result<String, PaneError>;
      /// The hub's prompt path: `Check` runs the gate and sends nothing; `Send` queues the prompt through `send_prompt`.
      fn prompt(&self, harness_session_id: &str, text: &str, mode: PromptMode) -> Result<(), PaneError>;
  }
  ```
- The join the engine and reconcile use, a pure function in a new file `crates/holler-pane/src/driven.rs`: `driver_of(pane, advertised, server_sessions, other_records) -> Driver<'_>` (`None`, `One(&Advertised)`, `Ambiguous(Vec<&Advertised>)`). Candidates are the advertised ids that the pane's **own server lists** and that are **not another pane's session of record** (servers that share a data directory list every pane's sessions; `check_unclaimed` says so, E31's plan). The pane's record wins among the candidates; otherwise exactly one candidate is the driver; none is `None`; two or more is `Ambiguous`. Why membership in the server's list and not equality with the record: a body still attached to an **earlier** session must be both **found** (to re-point it) and **seen** (DRIVEN differs from SHOWN); equality with the new record would find nothing and report nothing.
- The real `HubPort` is `crates/holler-cli/src/pane/wiring/hub.rs`: `advertised` reads `control/roster` (E19; keep rows whose `mode` is `attach`, `conn_state` is `connected` and `harness_session_id` is present); `rebind` calls `control/rebind` (D2); `prompt` calls `control/say` (D5). `Unwired` answers `not-implemented`; the test kit gets `FakeHub` (D10).
- **Switch and reset, the new order** (the engine, `tx_switch`; `Check` and `Send` are `PromptMode`):
  1. Plan (nothing written or moved): the existing refusals; then `hub.advertised()`, `harness.list_sessions` and `pane_store.list` give `driver_of`. `Ambiguous` is refused `pane-not-driven` (the message names the candidates). A driver whose `state` is `Working` is refused `session-busy`, `InputRequired` `session-holds-question` (both existing open codes) for switch and reset alike: the body would refuse the rebind, and the refusal must come before the TUI moves. With `--first`: unless a driver exists **and the id it advertises is the pane's session of record** (the gate joins by that id, so it can judge no other), the run is refused `pane-not-driven`; otherwise `hub.prompt(session of record, text, Check)` (D5).
  2. Act: `create_session` (reset), `select_session`.
  3. Observe: `shown_session` (unchanged: a mismatch is `unavailable`, nothing recorded).
  4. **Drive**: with a driver, `hub.rebind(driver, target)`; the acknowledged id must equal the target, else `unavailable`. A failure here is a failure after the act: `acted: true`, the message ends with the reconcile step (`doctor --fix` selects the old session again, which is the state the unmoved body still matches).
  5. Record, one compare-and-swap at the generation the plan read: the four fields of `recorded` (E32) **and `last_observed.driven`** = the acknowledged id, or `None` when no body drives the pane (observed by step 1, not inferred; this overwrites a stale stored value).
  6. First message, `--first` only: `hub.prompt(target, text, Send)`. A refusal or failure here is the existing `unprompted` failure (the record stands; E31's tail; the message says the session was created, shown and recorded, but the first message did not land), with the refusal's code.
  7. A run now makes at most **thirteen** port calls, each bounded by I5 (E30 said eight).
- Reconcile (`pane doctor`): reads `hub.advertised()` **once per pass**, as it reads `herdr.snapshot` (E34), computes `driver_of` for each pane from the server list it already observed, and records `last_observed.driven` with `shown` in the same write. A failed `advertised()` is an `observe-failed` finding for `hub.advertised` and leaves `driven` as stored (the rule for every failed read, E34). `shown-driven-mismatch` keeps its test, SHOWN against the session of record (E35, E36), and **also** reports a pane whose observed DRIVEN is `Some(d)` with `d` equal to neither SHOWN nor the session of record. (The kind's doc, E37, says it is the one kind `--fix` repairs; that stays true of the TUI side.) `--fix` repairs only the TUI side (it selects the session of record); a pane whose TUI is right and whose body is behind is `not-fixable` by `--fix`, and its remedy is `holler pane switch PANE SESSION` for its session of record, which is idempotent and re-points the body (E38's table gets that one case; if it needs a new internal `FixState`, that is allowed, it is not on the wire).
- Rejected: **(a) the hub writes or reports DRIVEN into the record**: the hub is the store only (ruling 1) and `last_observed` is "written by reconcile, never inferred" (I6, E33). **(b) record `driven = session_of_record` without observing it**: that is inference. **(c) drive before `select_session`**: a failed select would leave the body moved and nothing in `doctor --fix` could move it back; driving after the observation leaves the state `--fix` already restores. **(d) a `driven` field on the pane store's wire or a new `pane/*` method**: `PANE_METHODS` is frozen (E58) and the roster already carries every fact (E18).
- Known residue, documented: if the compare-and-swap fails after a successful rebind (a lost race or a `timeout`), the body is ahead of the record. `doctor` reports it as `shown-driven-mismatch`, and `holler pane switch PANE <session of record>` re-points the body back. No step is compensated (the engine's rule, E30).

**D4. Hold keys: the key `<label>/<session>` is the body session's and does not change when it is re-pointed.**
- An operator hold or a default hold on a pane's body session therefore **carries across a switch or reset**, and `--join-held` (which holds a key on first sight, E17) does not fire again after a rebind, because no new name appears. `reset --first` onto a held pane is refused before anything moves (D5) with the new open code `session-held` (exit 3; message: the hold kind, the reason, and `holler release <label>/<session>`); `release` lifts it as it lifts any hold (the pane hold of `hold_kind: "pane"` is not lifted by `release`, E68, and keeps its own remedy). A `rebind` is not refused by a hold: a hold stops prompts, not re-pointing.
- Pinned by tests (Acceptance 12): an operator hold set before a reset still refuses `--first` after a previous reset; a hub started with `--join-held` that held the body's name on join refuses `--first`, and after `release` the next `reset --first` is delivered; a rebind makes no `session_held` event.
- Rejected: a hold keyed on the harness session id (a new key space; an operator holds a session by the name `holler roster` shows, E18).

**D5. `reset --first`: checked before the switch, sent after it; the hub's `send_prompt` stays the only authority.**
- The plan step 1 above sends `hub.prompt(id, text, Check)`: the hub resolves the id, runs `holds.would_refuse` and the pane gate (the same functions `send_prompt` calls, E16, E2) and **sends nothing** (no frame to the body, no TalkLog line, no turn id moved, no grant consumed). A refusal changes nothing: no session is created, SHOWN, DRIVEN and the session of record are unchanged. This is the advisory fast path `talk::say` already has for holds (E9: "a fast path only"), not a second enforcement site: it calls the same functions, and the authority that cannot be raced is still `send_prompt`.
- `Check` is the new optional parameter `check_only: bool` of `control/say` (and so of `admin/say`, which shares the handler, E13): additive, absent means a normal send; documented in `docs/protocol/v2.md`.
- `check_only` is read where `control/say`'s other params are (E14). `Send` is `control/say` with `queue: true`, no grant, and the CLI's accept-and-return rule: the reply wait is `QUEUE_ACCEPT_WAIT` (2 s) and a read deadline means accepted (E23), never the turn's reply, so the verb is bounded (I5).
- If `Check` admitted and `Send` is refused (a race: a park, hold or health change in between), the reset has completed: the existing `unprompted` failure applies (the record stands, exit 3 with the refusal's code, the message says the first message did not land). That residue is accepted and documented.
- Rejected: **(a) roll back the switch on a refusal**: no port deletes a session, so a reset's new session would stay behind as a stray and "unchanged" would be false; undoing the TUI is a second act with its own failure modes; and the engine compensates nothing after the act (E30). **(b) no pre-check, refuse only after the record**: the issue's acceptance says SHOWN, DRIVEN and the session of record stay unchanged. **(c) a plan-time predicate in the engine without the hub** (the A review's finding 3, option iii): a third copy of the pane-state predicate and no view of holds.
- `HarnessPort::send_prompt` stays in the trait with its permissive default (E28; the OpenCode adapter implements it, #649's pass-through test and #831's fake use it) but **no verb and no engine calls it any more**. A source-scan test (like E77 and E95) fails if `holler-pane/src` or `holler-cli/src` calls `.send_prompt(` outside the adapter's own conformance. Retiring the method is a follow-up (finding 8's other option).

**D6. A pane no body drives, and a harness that is not OpenCode.**
- DRIVEN means "a live attach-mode body advertises a session the pane's server lists". On `main` only an OpenCode attach body can advertise one. For any pane with no such body (`Driver::None`): `last_observed.driven` is `None`; switch and reset work as before without the drive step; `say --pane` and `interrupt --pane` and `answer --pane` answer `pane-not-driven` (exit 3) instead of today's `unknown session` (exit 1); and **`reset --first` is refused `pane-not-driven` before anything moves**, as it is for a pane whose body is behind its session of record or whose record names no session (`pane reset PANE` without `--first` heals either, and `--first` can follow as a `say --pane`). Today `reset --first` works through the adapter; this is the behaviour change the one-gate rule costs, recorded in the CHANGELOG, and it is the operator's question 3.
- A Claude Code pane (#831) has no OpenCode `ses_` session and, on `main`, no body (#850 builds the driver). The join is a string comparison of the advertised id with the session of record, so a Claude UUID joins the same way once a body advertises it, and `session/rebind` is that driver's own request to implement (a relaunch). **Nothing in this story builds, fakes or assumes a Claude body.** Until #850 a Claude pane is `Driver::None`: `reset --first` on it is refused `pane-not-driven`, which changes #831's test `reset_first_reaches_the_new_process` (E91) and its ADR item 18 (E89); "Clashes with stories in flight" says how.

**D7. The new and changed refusal codes (ADR-0021 section 9), one mapping.** Declared with `RefusalCode::from_static` in `crates/holler-pane/src/hub.rs` (the owning file; exit 3, class refusal):

| What the hub or the plan answers | Pane code (exit) |
|---|---|
| `-32011 session_held`, `hold_kind: "pane"`, `reason` = `pane-parked`, `pane-unhealthy` or `pane-shown-driven-mismatch` | the same code (3), the message of `say_cmd::pane_hold_refusal` (E24) |
| `-32011 session_held`, `hold_kind` `operator` or `default` | `session-held` (3), **new** |
| `unknown session`, `not_connected` or `ambiguous` for the pane's id; no driver in the plan; `Ambiguous` drivers | `pane-not-driven` (3), **new** |
| `-32009 session_busy`; a driver whose state is `Working` or `InputRequired` | `session-busy`, `session-holds-question` (3, existing, #645) |
| a refused `rebind`: `-32003 unknown_session`, `-32004 not_connected`, `-32602 invalid_params` | `unavailable` (1), carrying the body's reason (the rebind comes after the act, so it is a failure, not a refusal) |
| `-32012 invalid_grant` (cannot happen: no grant is sent) | `unavailable` (1) |
| the accept wait's read deadline on `Send` | accepted (not an error) |
| any other transport or wire failure | `unavailable` (1) or `timeout` (1), as `ControlError` maps today |

`prompt-unsupported` leaves the `pane reset` row of section 9 (only the `HarnessPort` default raised it). No code goes into `holler_proto::Code`, and no pane verb exits 4.

**D8. Fixtures are `ses_`-shaped, and a test proves the shape.** Every fixture of a hub-routed case uses `ses_` and 26 characters (E78, E79, E80): the test kit gains `holler_pane_testkit::fixture::session_id(n: u32) -> String` = `format!("ses_{n:026x}")` (the format `FakeHarness` mints, E80), and one test asserts that it has 30 characters and that `RoutableName::parse` refuses it, so a fixture cannot pass by being a valid name. The #646 gate tests (E81, E82) are rewritten onto it. A test that passes only on `ses-demo-*` names does not satisfy the story.

**D9. Where the opt-in scenario changes (#649's `reset_with_a_first_message`, E86).** The scenario has a hub and no body, so a `reset --first` there is now refused. Step 3 becomes: `reset C2 --first "…"` exits 3 with `pane-not-driven` and the record is byte-for-byte unchanged (no session created); then `reset C2` without `--first` makes the fresh session the later steps use. The "its session reads Busy" assertion moves to the default-run end-to-end test (Acceptance 3), which has a real body. The scenario starts no body (that would need a hub join and a token flow in the scratch rig; it is a possible follow-up, not this story).

**D10. The test kit.** `FakeHub` (new file `crates/holler-pane-testkit/src/hub.rs`) implements `HubPort`: scripted bodies (`with_body(token, name, harness_session_id, state)`), `rebind` updates the matching body's id (or answers a scripted fault), `prompt(Check|Send)` finds the pane whose `session_of_record` equals the id **in the `FakePaneStore` it was built over** and applies `holler_pane::gate::prompt_refusal` plus scripted holds (`hold(name, kind)`), a call log (`calls()`), and the `Send` log (`prompts()`). A conformance function (new file `conformance/hub.rs`) runs against `FakeHub` and, in the default-run end-to-end test, against the real client over a real hub.

**D11. Size rules.** `scripts/lint.sh` fails at 900. "Where the code goes" lists the files and what each may grow by; `circuit.rs` takes **no** growth, `docs.rs` (899) and `roster.rs` (892) take none, `live.rs` at most +6, and no file crosses about 850 by this story. New tests go in new files.

**D12. What this story does not do** (named so a test can pin the edge): it builds no Claude Code body (#850); it adds no `doctor --fix` that re-points a body (the remedy is `pane switch`); it does not retire `HarnessPort::send_prompt`; it adds no CLI flag (`cli-surface.txt` is unchanged); it starts no body in the opt-in scenario; it does not change `pane list`'s columns (DRIVEN prints what the record holds, which is now real); it does not touch the roster command (#648).

## Scope

In this order:

1. **Contract.** ADR-0021 (the edit list below, section 11 verbatim), `docs/protocol/v2.md` (`session/rebind`, `check_only`, the pane hold paragraph, the error rows), a one-line extension note in ADR-0014. One commit, before any test.
2. **`holler-proto`:** `Rebind`, `RebindResult` (new file `rebind.rs`), the catalog row, the count test, the two golden files.
3. **`holler-body`:** the `session/rebind` handler (dispatch, session manager command, HTTP attach driver re-point, presence).
4. **`holler-hub`:** `resolve_target`, the re-keyed gate, `LiveHandle::query`'s allowlist, `control/rebind`, `check_only`, the `SayOpts`/`LiveCommand::Say`/`HoldGate` field.
5. **`holler-pane`:** `gate`, `driven`, `hub` (the port), `Ports.hub`, the engine's new order, reconcile's observation and finding, the module docs.
6. **`holler-pane-testkit`:** `session_id`, `FakeHub`, its conformance.
7. **`holler-cli`:** `wiring/hub.rs`, `Unwired`, `say_cmd` (the shared predicate, `pane-not-driven` for routed verbs), `ControlCall` constructors, the 15 `Ports { … }` literals (13 files) (one line each).
8. **Docs:** `docs/pane-control.md`, `CHANGELOG.md`, the module docs.

## The ADR and the dependent docs

**ADR-0021** (locate each by its words; line numbers are those of `main` at `bda1069` and will have moved when #831 and #835 merge). Numbers for the new "Decisions taken" items are the next free numbers at merge time; do not renumber others.

| Where (by words) | Change |
|---|---|
| Header, the **Amends:** line (E53) | Append: "[ADR 0014](ADR-0014.md) is extended by section 11 (#839): a hub may ask an attach-mode body to re-point one of its sessions to another harness session on the same endpoint (`session/rebind`); the session's name, and so its address, does not change." |
| Section 2, the port table (E54 and the table above it) | Add the row: `\| HubPort \| advertised, rebind(driver, harness_session_id), prompt(harness_session_id, text, mode) (#839) \| holler-cli/src/pane/wiring/hub.rs, a client of the hub's control socket \|`. In the sentence "Both `HarnessPort` and `HostPort` are amended since, under that rule", append: "and `Ports` by #839, which gains `HubPort` (section 11)". |
| Section 4, invariant I2's test idea (E55) | Append: "The driven session is the one the fake hub's body advertises (#839)." |
| Section 5, "Wiring as built (#649)", bullet "What the wiring makes live" (E57), the sentences from "And since #642 part 3 the first message of `pane reset --first` is a real prompt through `HarnessPort::send_prompt`" through "Until #828 lands, the operator does not use `reset --first` on a parked or held pane." | Replace with: "Since #839 the first message of `pane reset --first` goes through the hub's prompt path, so a parked, held or unhealthy pane refuses it (section 8, 'Switch and reset as built')." |
| Same section, bullet "Not done here", the sentence "The hub's DRIVEN session is not pointed at `session_of_record` (section 11, #654)." | Delete it (done: section 11, #839). |
| Section 5, the "Wiring as built (#649)" bullets (E56) | Add a bullet: "**The hub port (#839).** `connect` also builds `HubPort`, a client of the hub's control socket; it needs only the hub, as the registries' clients do." |
| Section 6 (E58), "the closed 22-row catalog" | "the closed 23-row catalog (22 until #839 added `session/rebind`, section 11)". |
| Section 8, "Switch and reset as built (#645)" (E59) | See "Replacement text for section 8" below. |
| Section 8, "The prompt and activity seam" (E60), from "Two choices differ from what this ADR had proposed for 645b" through "whether it moves to the hub's one prompt path is #828's (section 5, "Wiring as built (#649)")." | Replace with: "One choice differs from what this ADR had proposed for 645b, and #646 and #649 build on it: the activity read is of the pane's session of record, which section 11 (#839) makes the session the hub drives. The first message does **not** go through `HarnessPort::send_prompt` since #839: it goes through the hub's one prompt path (section 11, point 5). `HarnessPort::send_prompt` remains the adapter's transport, with no verb or engine caller; a test fails if one appears." Keep the last sentence ("The adapter's `send_prompt` stays pure transport…"). |
| Section 9, the `pane switch`, `pane reset` row (E61) | Replace "and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default)" with "and, with `--first`: open (#839) `pane-not-driven` (no body drives the pane, or more than one could), `session-held` (an operator or default hold on the body's session), and the pane gate's `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; for both verbs, `session-busy` and `session-holds-question` also when the body driving the pane reports a turn or a question". |
| Section 9, the `say`, `interrupt`, `answer` with `--pane` row (E61) | Append: "open (#839) `pane-not-driven`: no body advertises the pane's session of record (it was `unknown session`, exit 1)". And in the sentence "The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"`" add "for the session whose advertised id is the pane's session of record". |
| Section 9, the "any other well-formed code (open, raised as `Refused`)" row (E62), the list of open codes | Add "#839's `pane-not-driven` and `session-held`". |
| Section 11 (E63) | Replace as given in "Amended text of section 11" below. |
| Section 13 (E64), "is replaced by `session_of_record` once the cutover (#654) points the hub's DRIVEN session at it (section 11)" | "is replaced by `session_of_record` once a body is re-pointed to it (#839, section 11); the cutover (#654) starts one attach body for each pane." |
| "Deferred to named stories" (E65), the bullet "How the hub's DRIVEN session follows `session_of_record`: #654 …" | Replace with: "A body for a Claude Code pane, so that DRIVEN exists for it: #850. A `doctor --fix` that re-points a body, and retiring `HarnessPort::send_prompt`: unscheduled." |
| Same section, the bullet "The refusal of a pane that is not idle or holds a question, and `reset --first`", from "and the wiring (#649) makes the first message's path live, and whether it moves to the hub's one prompt path is #828's." | Replace with "and since #839 the first message goes through the hub's one prompt path." |
| "Decisions taken" (end of the ADR) | Append the decisions D1 to D6 of this brief, each as one numbered item marked "(2026-10-11, #839, under the operator's standing rules)". |

**Replacement text for section 8, "Switch and reset as built (#645)"** (E59). Keep the paragraph up to "…in one compare-and-swap at the generation the plan read, never retried;" and replace everything from "`last_observed.driven` is left as stored (section 11). Then, only for `reset --first TEXT`," through "it has no reconcile step, because the record is correct." with:

> `last_observed.driven` is written in the same compare-and-swap, from the id the body acknowledged (section 11), or `None` when no body drives the pane. Before the act, the plan reads the live attach sessions through `HubPort::advertised` and finds the pane's driver (`holler_pane::driven::driver_of`); a driver that reports a turn or a question is refused (`session-busy`, `session-holds-question`), and for `reset --first TEXT` a pane no body drives, or whose body does not advertise its session of record, is refused `pane-not-driven` and the hub's gate is asked, without sending, whether it would admit the prompt (`HubPort::prompt` with `Check`), so a parked, held or unhealthy pane, or one whose SHOWN and DRIVEN differ, is refused before anything moves: no session is created and the record is unchanged. After `shown_session` shows the target, `HubPort::rebind` asks the body to drive it; a failure there is a failure after the act (the message ends with the reconcile step). Then, only for `reset --first TEXT`, `HubPort::prompt` with `Send` queues `TEXT` through the hub's one prompt path (`send_prompt`), accepted and returned (never waiting for the turn). A prompt that fails after the record, a refusal in a race included, is that call's own error (a refusal is exit 3 with its code), and its message says the session was created, shown and recorded but the first message did not land; it has no reconcile step, because the record is correct.

Also in that paragraph: "Neither verb checks the pane's park state; a refusal derived from pane state belongs to the prompt gate of #646, which `reset --first` does not pass through (see the seam below)." becomes "Neither verb checks the pane's park state itself; `reset --first` is refused for it by the prompt gate of #646, asked before the act and applied at the send." And "a run makes at most eight port calls" becomes "at most thirteen".

**Amended text of section 11** (E63). Keep the first two paragraphs (from "ADR 0014 stays in force, unchanged." through "…(a session with no pane record).") and the paragraph on `HerdrPort::send_text` and `send_keys`. **Replace the paragraphs "Deferred to #654 (cutover)…" and "A switch or reset (#645) likewise leaves…" (the last five lines of the section) with this text, verbatim:**

```text
**DRIVEN as built (#839).** The session the hub drives for a pane is the harness session that a live attach-mode body
advertises (`SessionAd.harness_session_id`, ADR 0014 point 6), and the hub joins it to the pane by that id. Decided under
the operator's standing rules (2026-10-11). Each point names what it rejected.

1. **The address is the advertised `harness_session_id`.** `say`, `interrupt` and `answer` resolve a target that is a valid
   name (ADR 0005) by name, as before, and any other target as a harness session id, against the live attach sessions: no
   match is unknown, a match on an offline body is not connected, two matches are ambiguous. A name is never an id (the name
   grammar refuses `_`, which every OpenCode id has); an id that happens to be a valid name is resolved as a name and is not
   routable by id. The pane gate (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) is keyed on the advertised id
   of the session being prompted, joined to `Pane.session_of_record`, however the prompt addressed that session, and no longer
   on the session's name; so a bare `say` to a body session that drives a parked pane's session of record is refused too.
   One predicate, `holler_pane::gate::prompt_refusal`, decides the three refusals for the hub's gate and for `--pane`; their
   messages stay where they are. Rejected: joining on the pane's name (nothing makes a body's session name its pane's name,
   and the join would be a naming rule imposed on a fleet); a new field of `Pane` naming the body session (a second source of
   truth beside the advertisement, and a change of the stored record); a new request to ask a body which session it drives
   (presence already carries it); keeping the name match beside the id match (two keys, and only test fixtures match the old).
2. **A body is re-pointed, not restarted: `session/rebind`.** A new additive request, hub to body, `{session,
   harness_session_id}` answered `{session, harness_session_id}` (protocol v2 section 4; the catalog has 23 rows). The body
   re-points one of its own attach sessions to another harness session on the same endpoint, after the existence check
   `attach` makes (never `session/new`), and only when the session is idle: a session that is not an attach session is
   `invalid_params`, an unknown name `unknown_session`, a busy session or one holding a question `session_busy`, an id the
   endpoint does not have `invalid_params` with the old binding kept, an endpoint that does not answer within 5 s
   `not_connected` with the old binding kept. The session's name does not change, so its address and its hold key do not
   (point 4); ADR 0005 section 3 says the name is the identity and the locator changes. The hub forwards it over the generic
   request forward (`control/rebind`, bounded by 10 s) and, before it replies, updates its cached presence and the roster row,
   so a read after the reply sees the new id. Only the HTTP attach driver implements it; a body for another harness
   implements it in its own driver. Rejected: restarting the body with a new configuration (not bounded by I5, and the CLI
   would supervise a process and rewrite a file the wizard owns); a new body session for each harness session (a new name is a
   new hold key, so `--join-held` would default-hold every reset); the hub or the CLI posting the prompt to the pane's server
   (an adapter in the hub, and a second way to deliver a prompt); leaving it to the cutover (the first message of `reset
   --first` could not be delivered, and the gate would refuse everything as `pane-shown-driven-mismatch`).
3. **DRIVEN is observed, never inferred, and the hub does not write it (I6).** The pure function
   `holler_pane::driven::driver_of` finds the body that drives a pane: among the live attach sessions whose advertised id the
   pane's own server lists and that are not another pane's session of record, the pane's session of record if it is one,
   else the only one; none is not driven, more than one is ambiguous. The port `HubPort` (`advertised`, `rebind`, `prompt`)
   reads the hub through its control socket. A switch or reset writes `last_observed.driven` in the same compare-and-swap as
   the record, from the id the body acknowledged to `rebind` (`None` when no body drives the pane); reconcile reads
   `advertised` once per pass and records what it found with `shown`. `shown-driven-mismatch` is also reported when an
   observed DRIVEN is neither SHOWN nor the session of record; `--fix` repairs only the TUI side, and the remedy for a body
   that is behind is `holler pane switch PANE SESSION` for the session of record, which is idempotent. The order of a switch
   or reset is: plan (the refusals of section 8, then the driver, then for `--first` the gate's answer), act, observe, rebind,
   record, first message. The rebind comes after the observation so that a failed `select_session` leaves the body where it
   was and `doctor --fix` restores the rest. If the record's write fails after the rebind, the body is ahead of the record:
   `doctor` reports it and `pane switch` re-points it back; nothing is compensated (section 8). Rejected: the hub writing
   DRIVEN (the hub is the store only, ruling 1); recording the session of record as DRIVEN without observing it (inference);
   driving before the act (a failed select would leave the body moved, with no repair).
4. **Hold keys.** A hold is keyed `<label>/<session>` by the body session's name, which a rebind does not change, so an
   operator or default hold carries across a switch or reset, and `--join-held` (which holds a name on first sight) does not
   fire again. A hold stops prompts, not a rebind.
5. **`reset --first` goes through the hub's gate: asked before the act, sent after the record.** Before anything moves, the
   plan asks the hub whether it would admit the prompt (`control/say` with `check_only`, which resolves the id and runs the hold
   and the pane gate, the same functions `send_prompt` calls, and sends and logs nothing), so a refusal creates no session and
   leaves the session of record, SHOWN and DRIVEN as they were. After the record, the prompt is queued through
   `send_prompt`, accepted and returned within the accept wait (2 s) and never waiting for the turn (I5). `send_prompt` stays the
   only authority: a pane parked, held or made unhealthy between the question and the send is refused there, and then the
   reset has completed and the first message did not land (section 8). Rejected: rolling the switch back on a refusal (no port
   deletes a session, so the created session would stay as a stray and nothing would be "unchanged", and undoing the TUI is a
   second act that can fail); no question before the act (the record would change and only the message be refused); a plan-time
   check without the hub (a third copy of the predicate, blind to holds). `HarnessPort::send_prompt` stays as the adapter's
   transport and no verb or engine calls it.
6. **A pane no body drives.** DRIVEN is then `None`; a switch or reset works as before without the rebind; `say`,
   `interrupt` and `answer` with `--pane` answer `pane-not-driven` (they were `unknown session`); `reset --first` is refused
   `pane-not-driven` before anything moves, as it is when the body does not advertise the pane's session of record. Only an OpenCode attach body exists today. A Claude Code pane (#831) is in this
   case until a body can drive it (#850); the join is a string comparison of the advertised id with the session of record, so
   its session id joins the same way once a body advertises it, and that body implements `session/rebind` in its own driver.
```

**`docs/protocol/v2.md`** (E66 to E70):
- Section 4 table (E66): a row after `session/answer`: `session/rebind | request | hub→body | {session, harness_session_id} | {session, harness_session_id} — sent only after the driver targets the new session and presence advertises it (#839)`.
- Section "Attach-mode sessions" (E69): a paragraph "**Re-pointing (#839).**" stating the body's refusals and the 5 s bound of D2, and that the session name does not change.
- Section 7, presence (E67): after the sentence "`harness_session_id` is present for `attach`": "A `session/rebind` changes it; the hub's cache and roster row are updated when the rebind is acknowledged."
- The `control/say` description (wherever `admin/say`'s "identical to control/say's `{session, text, queue?, grant?, timeout_ms?}`" is, E66): add `check_only?`, and say that `session` may be a harness session id (D1).
- The pane hold paragraph (E68): "the session of record of a pane" → "a session whose advertised `harness_session_id` is the session of record of a pane"; keep the rest.
- Error rows (E70): `-32009` and `-32004` rows gain the rebind cases ("or a `session/rebind` of a session with a turn in flight or a question pending"; "or a `session/rebind` whose endpoint did not answer").
- The non-normative note at the end about `pane/*` (v2.md line 817): leave, and add one sentence that `control/rebind` is a control-socket method as `pane/*` are, outside the catalog.

**`docs/adr/ADR-0014.md`:** one bullet at the end of "Consequences": "- Extended by [ADR 0021](ADR-0021.md) section 11 (#839): a hub may ask an attach-mode body to re-point one of its sessions to another harness session on the same endpoint (`session/rebind`). The session's name, and so its address, does not change."

**`docs/pane-control.md`** (E71 to E75; `docs_pane_control_test` pins the guide to the code's tables, so it must still pass): section for `reset` (E71): replace the paragraph "**Do not use `--first` on a parked or held pane.** …" with the new rule (the first message goes through the hub's gate; a parked, held or unhealthy pane, one whose SHOWN and DRIVEN differ, and one no body drives are refused before anything moves, `pane-parked`, `session-held`, `pane-unhealthy`, `pane-shown-driven-mismatch`, `pane-not-driven`), and add that a pane needs a body attached to its server for `--first`; section for `park` (E71): delete "; the first message of `reset --first` is not (see `reset`)"; DRIVEN column (E72) and "SHOWN against DRIVEN" (E73): DRIVEN is the session the body that drives the pane advertises, recorded by `switch`, `reset` and `doctor`, `-` when no body drives the pane; SYNC and `doctor` compare as ADR section 11 says; cutover steps (E74): delete "and `reset --first` is not used on a parked or held pane (section 3)", and add that each pane needs an attach body before the cutover step that uses `reset --first`; "Later" (E75): delete the bullet "DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839)". Section 9's error table gains `pane-not-driven` and `session-held`.

**Module docs that repeat the old sentences** (edit by words): `crates/holler-pane/src/tx_switch.rs` lines 27-35 and 50 (E30), `crates/holler-pane/src/reconcile.rs` lines 33-35 (E34), `crates/holler-cli/src/pane/reset.rs` lines 1-15, `crates/holler-pane/src/ports.rs` line 345 ("`pane reset --first` calls it") and the amendments list at its top, `crates/holler-adapter-opencode/src/prompt.rs` line 7 ("`pane reset --first` sends through `send_prompt` outside that path"), `crates/holler-hub/src/circuit/dispatch.rs` (the doc of `pane_state_refusal` and of the tests that name the CLI twin), `crates/holler-cli/src/say_cmd.rs` (the doc of `resolve_pane_target` that names the twin), `crates/holler-proto/src/methods.rs` (the "22-row" doc and the count test, E42, E43), and `crates/holler-hub/src/live.rs` (the allowlist comment, E6).

**`CHANGELOG.md`**, under `## [Unreleased]`, `### Enhancements`, one new item (E76 shows the shape), linking `/issues/839` and `/issues/828`:

> - Pane control, DRIVEN (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): the session the hub drives now follows each pane's session of record ([#839](https://github.com/Performant-Labs/holler/issues/839), which absorbs [#828](https://github.com/Performant-Labs/holler/issues/828)). The hub joins a pane to the attach-mode body that advertises its session of record, and `say`, `interrupt` and `answer` accept that `ses_…` id as their target, so `--pane` now reaches a real pane; the pane gate (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) is keyed on the same id instead of the body's session name. `holler pane switch` and `holler pane reset` ask the body to follow the new session (a new additive hub-to-body request, `session/rebind`; the protocol catalog has 23 rows) and record `last_observed.driven` from its answer, and `holler pane doctor` records it too and reports `shown-driven-mismatch` when it differs. The first message of `holler pane reset --first` now goes through the hub's prompt path: a parked, held or unhealthy pane refuses it before anything moves (`pane-parked`, `session-held`, `pane-unhealthy`, `pane-shown-driven-mismatch`), and a pane no body drives refuses it (`pane-not-driven`; it worked through the harness before). `control/say` gains an optional `check_only`. This replaces the caution about `reset --first` on a parked or held pane in the entry above. [ADR 0021](docs/adr/ADR-0021.md) section 11 records the decisions.

and, in the existing #649 item (E76's neighbour, "so until [#828] lands, do not use `reset --first` on a parked or held pane"), replace that clause with "and, since [#839](https://github.com/Performant-Labs/holler/issues/839), through the hub's gate".

## Where the code goes

Sizes measured with `wc -l` at `bda1069`; `scripts/lint.sh` fails at 900. The issue's numbers for `switch.rs` (~833) and `support/mod.rs` (~849) have grown to 868 and 857.

| File | Now | What | Rule |
|---|---|---|---|
| `crates/holler-hub/src/circuit.rs` | 898 | The `LiveCommand::Say` arm (E4): destructure `harness_session` and set `HoldGate.harness_session` | **+0 lines**: edit the existing lines in place (they already exceed 100 columns); do not reformat the file |
| `crates/holler-hub/src/live.rs` | 884 | `SayOpts` and `LiveCommand::Say` gain one field each (E5); `query`'s allowlist condition (E6); `mod target;` | at most **+6**; `resolve_target` itself goes in the new `crates/holler-hub/src/live/target.rs` |
| `crates/holler-hub/src/circuit/dispatch.rs` | 820 | `HoldGate.harness_session`; `pane_gate` re-keyed; `pane_state_refusal` deleted (E1) | net **negative**; the pane-gate tests (E81, E82 and their neighbours) move to a new `crates/holler-hub/src/circuit/dispatch/gate_tests.rs` (`#[cfg(test)] mod gate_tests;`), rewritten on `ses_` ids |
| `crates/holler-hub/src/talk.rs` | 540 | the two resolve lines of `say` and `answer`; `SayOpts` gets `harness_session`; the `check_only` branch calls a new `talk::admit` | ok (+30); if more, `crates/holler-hub/src/talk/admit.rs` |
| `crates/holler-hub/src/interrupt.rs` | 155 | the two resolve lines | ok |
| `crates/holler-hub/src/control_server.rs` | 837 | one arm for `control/rebind` in `dispatch_session_control`; `check_only` read in `say` | at most **+8**; everything else in `control_rebind.rs` (new) |
| `crates/holler-hub/src/control.rs` | 518 | `ControlCall::rebind`, `ControlCall::say_check` | ok |
| `crates/holler-hub/src/roster.rs` | 892 | nothing: `Roster::advertise` is reused | **no growth** |
| `crates/holler-proto/src/docs.rs` | 899 | nothing | **no growth**; `Rebind`, `RebindResult` in new `crates/holler-proto/src/rebind.rs` |
| `crates/holler-proto/src/methods.rs` | 219 | one catalog row, the count and doc | ok |
| `crates/holler-body/src/connection.rs` | 873 | the `matches!` list (E48) and `static_wire_method` gain the name | at most **+2**; the handler is in `prompt_dispatch.rs` (355) and `session_dispatch.rs` (120) |
| `crates/holler-body/src/session_manager/task.rs` | 705 | the command `Rebind` | ok; the driver swap in `http_attach_driver/` (`http_attach_driver.rs` 435, `connection.rs` 618) |
| `crates/holler-pane/src/ports.rs` | 423 | `Ports.hub` | +1 |
| `crates/holler-pane/src/tx_switch.rs` | 399 | the new order | ~+70, ok; if it passes 600, the plan helpers go to `tx_switch/plan.rs` |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs` | 450, 391 | the observation and the finding | ok |
| new `crates/holler-pane/src/{gate,driven,hub}.rs` | 0 | D1, D3 | each under 200 |
| `crates/holler-cli/src/say_cmd.rs` | 494 | the shared predicate; `pane-not-driven` | net negative |
| `crates/holler-cli/src/pane/wiring.rs` | 396 | `mod hub;`, the field, `Unwired`'s impl | ok; the client is `wiring/hub.rs` (new) |
| `crates/holler-cli/tests/pane_verbs/switch.rs` | 868 | **no growth** (it already holds `SeamHarness`) | new tests in new files |
| `crates/holler-cli/tests/support/mod.rs` | 857 | at most a `mod driven;` line | helpers in new `crates/holler-cli/tests/support/driven.rs` |
| `crates/holler-cli/tests/pane_verbs/reset.rs` | 530 | E83 and E84 are rewritten in place; the rest of the new tests are in new `crates/holler-cli/tests/pane_verbs/driven.rs` (add `mod driven;` to `main.rs`, 66 lines) | |
| the 15 `Ports { … }` literals (13 files) | | one line each (`hub: &hub,`): `verb_harness/mod.rs`, `pane_integration/bind.rs`, `pane_verbs/{park/rig,list,doctor/rig,park,close/rig,launch/rig,import/rig}.rs`, `profile_verbs/{rig,apply/rig}.rs`, `holler-pane/tests/ports_test.rs` (two), `pane/wiring.rs` | `launch/rig.rs` is 835: +1 is fine |

## Acceptance (each is a test)

Fixtures use `ses_`-shaped ids (D8). "On the fakes" means the test kit's fakes with `FakeHub`.

1. **I2 extended to DRIVEN.** After a switch and after a reset on the fakes, with a body advertising the old session, `session_of_record`, SHOWN and the fake hub's advertised id are equal, `last_observed.driven` equals them, and the record was written by one `cas_put` (new file `pane_verbs/driven.rs`).
2. **The step order.** The fakes' call logs show: reads, `advertised`, [`prompt(Check)`], `create_session` (reset), `select_session`, `shown_session`, `rebind`, `cas_put`, [`prompt(Send)`]; `rebind` is after `shown_session` and before `cas_put`; `prompt(Send)` is after `cas_put`; `HarnessPort::send_prompt` is never called; no keystroke (I4).
3. **A message through the hub's prompt path reaches a pane's real session of record.** Default-run end-to-end test (new file `crates/holler-cli/tests/driven_e2e_test.rs`, pattern E87): a real hub, a real body in attach mode against the fake OpenCode server with a `ses_`-shaped id, the pane registered through the pane store; `holler say --pane P "hi"` is delivered to `POST /session/<id>/prompt_async` and the reply returns. `say --pane` on a pane with no body exits 3 with `pane-not-driven`. `interrupt --pane` and `answer --pane` resolve the same id.
4. **The gate refuses a real-shaped pane**, in `gate_tests.rs` and in the end-to-end test: parked gives `pane-parked`, unhealthy `pane-unhealthy`, SHOWN differing from DRIVEN `pane-shown-driven-mismatch`; one side absent is no mismatch; two panes recording one session are fail-closed with the first bad one named; an operator hold refuses first with its own kind; a body session with no `harness_session_id` (spawn) is never pane-gated; **a session whose NAME equals a pane's session of record is not gated** (the old key is gone); a bare `say <name>` to the body that drives a parked pane is refused.
5. **Resolution.** `resolve_target`: an id matches the one live attach ad; a name still resolves by name; an unknown id is `Unknown`; an id on an offline body is `NotConnected`; two bodies advertising one id are `Ambiguous`; a valid-name-shaped id string never resolves by id. `say`, `interrupt` and `answer` accept an id (new file `crates/holler-hub/tests/driven_resolve_test.rs`).
6. **`doctor`.** `shown-driven-mismatch` is reported for a pane whose observed DRIVEN differs from SHOWN (TUI right, body behind: `not-fixable`, remedy `holler pane switch PANE SESSION`; TUI wrong: fixable as today), and not for one that agrees; `last_observed.driven` is recorded in the same write as `shown`; `hub.advertised` failing is an `observe-failed` finding and leaves `driven` as stored; `driver_of`'s table (record preferred, a stale body found, another pane's record excluded, none, ambiguous) in `crates/holler-pane/tests/driven_test.rs`.
7. **`reset --first` obeys the gate.** On a parked pane, an unhealthy pane, a pane whose SHOWN and DRIVEN differ, a held pane (operator hold and default hold) and a pane no body drives: exit 3 with `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`, `session-held` and `pane-not-driven`; in each, `create_session` and `select_session` were never called, the record is byte-for-byte unchanged (session of record, SHOWN, DRIVEN, generation), and no prompt was sent. On a healthy pane it is delivered through `HubPort::prompt(Send)` and the harness fake records no prompt. A race (`Check` admits, `Send` is refused) gives exit 3 with the refusal's code, the record stands, and the message says the first message did not land.
8. **Rebind failures.** A refused rebind (body busy, id not on the endpoint, endpoint silent) after the act: exit 1 or 3 by the code's class, `acted: true`, the message ends with the reconcile step, the record unchanged. A busy or questioning driver in the plan: `session-busy` or `session-holds-question` with nothing moved, for switch and reset. `switch` still queries no `session_activity` (E85 stays green).
9. **The body.** `crates/holler-body/tests/rebind_test.rs` (fake OpenCode server; toggle `exists_v1` and `exists_v2`, since the fake answers one verdict for every id): the happy path (presence advertises the new id; the next prompt goes to the new id's `prompt_async`; the name is unchanged), the five refusals of D2, the old binding kept on every refusal, and the 5 s bound with a server that never answers. The hub's `control/rebind` (in `driven_resolve_test.rs` or its own file): forwards, passes a body refusal through verbatim, updates `control/roster` and `resolve_target` before the reply.
10. **`check_only`.** `control/say` with `check_only` and a body that would refuse answers the refusal and one that would admit answers `{admitted: true}`; in both no frame reaches the body, no TalkLog line is written, no turn id moves, no grant is consumed.
11. **The protocol.** The catalog has 23 rows (`session/rebind`: request, `HubToBody`); the goldens `Rebind.json` and `RebindResult.json` are new files and `scripts/golden-diff-summary.sh` shows no value drift in any existing golden; `docs/protocol/v2.md` has the row.
12. **Hold behaviour across a reset (D4).** The three tests of D4, with the key `<label>/<session>` asserted equal before and after.
13. **No bypass.** A source-scan test (like E77) fails if `holler-pane/src` or `holler-cli/src` calls `.send_prompt(` and if a second `session/prompt` builder appears in the hub (E77 itself stays green); `LiveHandle::query`'s allowlist is exactly `query/*`, `session/answer`, `session/rebind`.
14. **One predicate.** The three pane-state cases are decided by `holler_pane::gate::prompt_refusal` for the hub and for `--pane`: a test builds the same records for both and asserts the same code; no other copy of the rule exists in `holler-hub` or `holler-cli` (a source scan for `PANE_SHOWN_DRIVEN_MISMATCH` definitions).
15. **Fixtures.** `session_id(n)` has 30 characters and `RoutableName::parse` refuses it; every hub-routed fixture uses it.
16. **Scenario.** `reset_with_a_first_message` is rewritten as D9 says (it is `#[ignore]`d behind `HOLLER_TEST_PANE_INTEGRATION=1`, so CI compiles it and does not run it; the build run compiles it).
17. **Docs.** `docs_pane_control_test` passes with the amended guide; the ADR and the module docs carry none of the old sentences listed above (a grep for "#828", "until the cutover wires DRIVEN", "No port observes DRIVEN" finds nothing outside `docs/handoffs/`); the CHANGELOG has the entry.

## Blast radius

- `docs/adr/ADR-0021.md`, `docs/adr/ADR-0014.md`, `docs/protocol/v2.md`, `docs/pane-control.md`, `CHANGELOG.md`, `docs/handoffs/839-brief.md`.
- `crates/holler-proto/src/{rebind.rs (new),lib.rs,methods.rs}`, `crates/holler-proto/tests/golden/docs/{Rebind,RebindResult}.json` (new), `crates/holler-proto/tests/golden_test.rs`.
- `crates/holler-body/src/{connection.rs,connection/session_dispatch.rs,prompt_dispatch.rs,session_manager.rs,session_manager/task.rs,http_attach_driver.rs,http_attach_driver/connection.rs}`, `crates/holler-body/tests/rebind_test.rs` (new).
- `crates/holler-hub/src/{circuit.rs,circuit/dispatch.rs,circuit/dispatch/gate_tests.rs (new),live.rs,live/target.rs (new),talk.rs,interrupt.rs,control_server.rs,control_rebind.rs (new),control.rs,lib.rs}`, `crates/holler-hub/tests/driven_resolve_test.rs` (new).
- `crates/holler-pane/src/{lib.rs,ports.rs,gate.rs,driven.rs,hub.rs (the last three new),tx_switch.rs,reconcile.rs,reconcile/observe.rs,findings.rs}`, `crates/holler-pane/tests/{driven_test.rs (new),ports_test.rs}`.
- `crates/holler-pane-testkit/src/{lib.rs,fixture.rs,hub.rs (new),conformance/hub.rs (new),conformance/mod.rs}`, and its tests.
- `crates/holler-cli/src/{say_cmd.rs,pane/wiring.rs,pane/wiring/hub.rs (new),pane/reset.rs (doc)}`, `crates/holler-cli/Cargo.toml` (the new `[[test]]` `driven_e2e_test`), the 15 `Ports` literals listed above, `crates/holler-cli/tests/{driven_e2e_test.rs (new),pane_verbs/driven.rs (new),pane_verbs/reset.rs,pane_verbs/main.rs,support/driven.rs (new),support/mod.rs (one line),pane_integration/scenario.rs}`.
- `crates/holler-adapter-opencode/src/prompt.rs` (the module doc line only).
- Not touched: `crates/holler-cli/tests/fixtures/cli-surface.txt`, `docs/adr/ADR-0003.md` (no CLI flag, no `--json` shape change: `last_observed.driven` is an existing field that now holds a value).

## Clashes with stories in flight

State on 2026-10-11 (read only; branches and shas as found). The shared hot spots are `ports.rs`/`Ports` literals, `pane_integration/bind.rs`, the ADR's "Deferred" bullet, the ADR's section 8 and 11, and `CHANGELOG.md`.

| Story | Shared files and hunks | Does the DRIVEN decision change what it assumes? |
|---|---|---|
| **#831** Claude Code harness (branch `issue-831-implementation`, `b3f0295`, A and T-red passed; brief `docs/handoffs/831-brief.md`) | ADR-0021: its section 15 (a new section, no conflict), the "Deferred" DRIVEN bullet (E90: it already reads "…: #839 (it was #654's…)", the same bullet #839 rewrites), the "Decisions taken" list (it adds 22 to 25: #839 takes the next free numbers). `crates/holler-pane/src/tx_switch.rs`, `tx_launch*`, `reconcile*` (its endpoint change is wide and mechanical, per its brief). `crates/holler-cli/tests/pane_integration/bind.rs` (+195 lines, a `Ports` literal at line 537). `Cargo.toml`/`Cargo.lock`, `CHANGELOG.md`. | **Yes, in two places.** (1) Its test `pane_claude/lifecycle.rs::reset_first_reaches_the_new_process` (E91) asserts a Claude pane's `reset --first` reaches the new process through the engine's `send_prompt`. After #839 the engine does not call `send_prompt`, and a Claude pane has no body (#850), so the same command is refused `pane-not-driven` before the pane moves: that test becomes "refused, nothing moved", and its scenario step "deliver at `HarnessPort::send_prompt`" (`mixed.rs`, which calls the port directly) stays valid as an adapter-level check. (2) Its ADR item 18 (E89) already says `say --pane` needs #839 and a body (#850); #839 makes that exact: the join works for any harness whose body advertises its id, and #850 supplies the body. #831's item 9 (`interrupt --pane` for a Claude pane is a relaunch "after the same pane-state gates `resolve_pane_target` applies") calls `say_cmd`'s predicate: after #839 it calls `holler_pane::gate::prompt_refusal` through the unchanged `resolve_pane_target`, so nothing to change. |
| **#835** import leaves panes it does not drive alone (branch `issue-835-implementation`, `526bbd3`, at T-green BLOCK) | `crates/holler-pane/src/ports.rs` (it makes `HerdrPort::observe_pane` required and adds `HostPort::foreground_commands`, E92: the compiler must find every implementation, a wrapper included), `pane_integration/bind.rs` (+14), the `Ports` literals of `launch/rig.rs`, `import/rig.rs`, `profile_verbs/apply/rig.rs`, `crates/holler-cli/src/pane/wiring.rs` and `wiring/bind.rs`, ADR-0021 (sections 2, 3, 9, 13; none is DRIVEN), `docs_pane_control_test.rs`, `CHANGELOG.md`. | **No change to what it assumes.** It adds no DRIVEN logic. The `HubPort` of D3 follows its rule (required methods, no defaults) so a wrapper cannot forget to forward. Mechanical conflicts only: both add a field or a method to the same traits and literals. |
| **#648** roster SHOWN and DRIVEN (branch `issue-648-implementation`, `f5f7013`, brief only, round 6) | `crates/holler-cli/src/roster_cmd.rs`, `main.rs`, `crates/holler-hub/src/roster.rs` (additive fields; 892 lines: it must stay under 900, so #839 adds none), `CHANGELOG.md`, `docs/pane-control.md` (the roster sections). | **Compatible, and it is better after #839.** It reads `last_observed.driven` from the record (E93: "the existing pair rule"), and its own brief says every record's DRIVEN is `None` "until #839 builds the writer". It will implement the pair rule a third time unless it calls `holler_pane::gate`; recommend it does (`prompt_refusal`'s pair test is public). Its fixtures set `driven` by hand, so they are unaffected. |
| **#667** `profile apply` scenario (branch `issue-667-implementation`, no commits yet) | `crates/holler-cli/Cargo.toml`, `crates/holler-cli/tests/profile_apply_scenario/**` (new), and a `Ports { … }` literal built from the real adapters and the kit's in-memory stores. | **Small.** Its harness must give `Ports` a `hub`: `Unwired` is enough (apply never prompts or drives), or the kit's `FakeHub`. Nothing else. |

**Shared ADR-0021 and CHANGELOG hunks, so the conflicts are expected and mechanical.** (1) The "Deferred to named stories" DRIVEN bullet: #831 rewrote it (E90) and #839 rewrites it again: take #839's text. (2) The end of "Decisions taken": #831 appends items 22 to 25, #835 appends a few lines, and #839 appends its own: keep all, renumber #839's to the next free numbers. (3) Section 9's table: #835 edits the `pane import` row, which is the line above the `say`, `interrupt`, `answer` row that #839 edits (adjacent, not overlapping). (4) Section 2 and the wiring bullets of section 5: #835 and #839 each add to the port tables and lists. (5) Section 8 and 11: only #839 (and #831's new section 15 sits after section 14). (6) `CHANGELOG.md`: every story adds a bullet at the top of `### Enhancements`; whichever merges later rebases its bullet above the others, and #839 also edits one clause of the #649 bullet. (7) `docs/pane-control.md`: #839 edits `reset` and `park` (section 3), the DRIVEN column and "SHOWN against DRIVEN" (section 4), cutover steps 4 to 6 (section 10) and the "Later" list (section 12); no other branch edits it yet, but #648's brief names the roster paragraphs of section 4, next to the DRIVEN paragraph.

**Recommended merge order:** #835, then #831, then #648 (any order among these three, as each is nearly or fully built and none depends on #839), then **#839**, then #667. Reasons: #839 adds a field to `Ports` (15 literals) and rewrites the engine's tail, and it has no code yet, while #831 and #835 have code and tests written against the current `Ports` and the current `send_prompt` step; rebasing #839 onto them is smaller than rebasing both onto it. #831's ADR "Deferred" bullet edit lands first and #839 supersedes it with the text above. #667 has no code yet and should be written against `Ports` with `hub`. If #831 must land before #839 with its `reset_first_reaches_the_new_process` as it is, #839's run changes that one test and the sentence of its ADR item 18 that says the story proves delivery "through `reset --first`" (E89, line 1176) in the same change; the plan check should confirm this with the operator because it edits another story's contract text.

## Line numbers re-checked

The issue and the #828 review cite lines "true at 4366fb8". Against `main` at `bda1069` (`git diff 4366fb8 HEAD` touches only `tx_switch.rs` among the hub and CLI files named; the six commits since are #832's and docs):

| Citation | Now |
|---|---|
| `circuit.rs` 898, `dispatch.rs` ~820, `control_server.rs` ~837, `live.rs` ~884 | true (898, 820, 837, 884) |
| `tests/pane_verbs/switch.rs` ~833, `tests/support/mod.rs` ~849 | **grew**: 868 and 857 |
| `control_server.rs:291-305` (`say`), `talk.rs:203-209`, `live.rs:744-775`, `circuit.rs:737-740` | true |
| `dispatch.rs:83-103` (`pane_state_refusal`) | moved to 87-103 |
| `dispatch.rs:130-133` (the gate's compare) | moved to 134-136 |
| `say_cmd.rs:341-372` | 344-372; `resolve_pane_target` is 301-318 |
| `holds.rs:429-450` (`note_joined`) | 435-460 |
| `vocab.rs:202-223` | `check_segment` is 202-218 |
| `docs.rs:530-532` (`harness_session_id`) | true |
| `tx_switch.rs:78-81`, `record_bytes.rs:20` | true |
| ADR-0021 `:823-828` (section 11 deferral), `:926` ("Deferred"), `:480-486`, `:525-526`, `:694`, `:931-934` | moved down by 60 to 130 lines (section 11 is 866-888; the DRIVEN bullet 1040; section 8's switch and reset paragraph starts at 523; the row at 753; the bullets at 1043-1048). **Locate by words.** |

## Depends on

Merged: #645, #646, #647, #649, #652, #802, #832. Related and not dependencies: #835, #831, #648, #667 (see "Clashes"). **#654** (the cutover) loses DRIVEN as its own work: its step "run `doctor` until it reports no `shown-driven-mismatch`" becomes a meaningful check, and it must start one attach body for each pane and use `holler pane switch` or `reset` afterwards (the guide edit above says so). **#850** (a body for a Claude Code pane) is the follow-up that gives a Claude pane a driver. The issue #654's body is edited by the operator, not by this story.

## Pipeline

`rigor: in-session` (operator decision, 2026-10-11). Phase order (the repo's pipeline, `uiSurface: false`, so D and U never run): O, A (this brief), T (author, RED), F, T (verify, GREEN), A (anti-duplication), S. Contract amendment first (ADR-0021 and `docs/protocol/v2.md`, one commit), then tests, then code. A fresh worktree, branch `issue-839-implementation`, F on `claude-opus-5-5` at `max` (the repo's table). Needs no real port, live fleet, or real Herdr session; verification is the test kit and scratch instances (a real hub and body against the fake OpenCode server are scratch instances the test starts).

## Rules for this story

See the epic (#633): public repository (no personal host, tailnet or account names), the amend-first rule for the contract, no story touches a live fleet, a real Herdr session, a running pane or a real OpenCode session. A prompt reaches a body only through `send_prompt` in `crates/holler-hub/src/circuit/dispatch.rs`; put control-plane behaviour there, not at call sites. Never run workspace `cargo fmt --all` (it reformats hundreds of files); keep each edit in the style of its file. Edit only the files in the Blast radius. Conventional Commits; no new `unsafe`; clippy clean. Declare every new open code with `RefusalCode::from_static` in the owning file and list it in section 9.

## Questions only the operator can answer

The preparer did not guess these. Each has a default, which the brief above already follows.

1. **Scope: one story, or two?** D2 adds a protocol request, a body change, a hub control method, a new port and a CLI client, and rewrites the engine's tail, in six crates. The brief keeps it one story, in the order contract, tests, code, because the issue says so and the acceptance ("a healthy pane's `reset --first` is delivered") cannot hold without the re-point. The alternative is to split `holler-proto` + `holler-body` + the hub control method (`session/rebind` and `control/rebind`, with the body tests) into a first story, and leave the join, the gate, the port and the verbs for the second. Default: one story.
2. **A fleet fact the repository cannot show.** The design assumes the production fleet runs **one attach-mode body session per registry pane, whose endpoint is that pane's OpenCode server** (the wizard's attach rows, E88, do). If a body hosts several panes' sessions, or a pane is driven in spawn mode, `driver_of` and `session/rebind` still work per session, but the cutover (#654) must start one attach session per pane, and the guide says so. Please confirm the shape before the cutover.
3. **The behaviour change.** `reset --first` on a pane no body drives is **refused** (`pane-not-driven`); today it works through the adapter. That is the price of one gate, and it also removes the only way to send a first message to a Claude pane until #850. Default: accept it and say so in the CHANGELOG. The alternative (keep the adapter path for panes with no body) is a second enforcement site, which the A review rejected for #828.

## Not verified

- The production fleet's body shape (question 2).
- The attach driver's request timeout: the code builds `reqwest::Client::new()` with no timeout (E97), so the 5 s wrapper in D2 is mine; a measured rebind time on a real server was not taken.
- `Roster::advertise` re-advertising a row for a token with a changed `harness_session_id`: `handle_presence_notification` calls it for every beat, so it should, but `control_rebind` is the one new caller and its test (Acceptance 9) pins it.
- That the fake OpenCode server (E87's pattern) can serve two session ids: it answers one verdict for every id (`exists_v1`), so the rebind tests toggle it.
- The branches of #831, #835 and #648 are moving (their runs are in progress): their shas above are the ones read, and "Clashes" should be re-read at the plan check.

## AI assistance

Drafted by Claude (Sonnet 5.5) for story #839, from the issue, `docs/handoffs/828/handoff-A.md` (on the #828 branch), `main` at `bda1069` and the read-only state of the sibling branches named above, at the operator's direction, per `CONTRIBUTING.md`. The decisions are the preparer's, to be confirmed at the plan check.

## Evidence (verbatim)

Cut on 2026-10-11 from `main` at `bda1069` (a few excerpts come from a named branch commit, shown in their command). Every excerpt below was produced by a command (shown under its heading) that pipes the source through `sed -n 'A,Bp'`; the line numbers on the left were added by `awk` and are those of the file at that commit. Nothing is retyped.

**E1: `crates/holler-hub/src/circuit/dispatch.rs:58-103`** — The hub's gate input and the pane-state predicate (copy 1 of 2): `HoldGate`, the three codes, `pane_state_refusal`. It keys on nothing but the record.

Command: `sed -n '58,103p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=58 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
58	/// The hold registry and the key [`send_prompt`] checks it with
59	/// (`<label>/<session>`, built from the connection's own authenticated label),
60	/// and the pane registry it checks the session against (story #646).
61	#[derive(Clone)]
62	pub(super) struct HoldGate<'a> {
63	    pub(super) holds: &'a crate::holds::Holds,
64	    pub(super) key: &'a str,
65	    /// The one-time release grant the sender presented, if any (issue #460).
66	    pub(super) grant: Option<&'a str>,
67	    /// The pane registry ([`Registry::panes`]), `None` when the registry was
68	    /// built without one: then no prompt is refused for a pane's state.
69	    pub(super) panes: Option<std::sync::Arc<crate::panes::PaneState>>,
70	}
71	
72	/// The pane-state refusals of [`send_prompt`] (story #646 part 3), as `data.reason`.
73	const PANE_PARKED: &str = "pane-parked";
74	const PANE_UNHEALTHY: &str = "pane-unhealthy";
75	const PANE_SHOWN_DRIVEN_MISMATCH: &str = "pane-shown-driven-mismatch";
76	
77	/// What a pane's record says against prompting its session of record: its code and
78	/// a few words of why, or `None` when nothing does. The record is the source (the
79	/// hold `park` set, the health and the last observation reconcile wrote), never a
80	/// live probe: parked; unhealthy; or SHOWN and DRIVEN both observed and differing
81	/// (one side absent is not a mismatch). Checked in that order.
82	///
83	/// Its twin is the CLI's routing engine, `holler_cli::say_cmd::resolve_pane_target`,
84	/// which refuses the same three conditions before contacting the hub. The predicate
85	/// exists twice only because `holler-pane` is frozen and the hub cannot depend on the
86	/// CLI crate; keep the two equal.
87	fn pane_state_refusal(pane: &holler_pane::Pane) -> Option<(&'static str, &'static str)> {
88	    use holler_pane::pane::{Health, Hold};
89	    if matches!(pane.hold, Hold::Parked { .. }) {
90	        return Some((PANE_PARKED, "is parked"));
91	    }
92	    if matches!(pane.harness.health, Health::Unhealthy(_)) {
93	        return Some((PANE_UNHEALTHY, "is unhealthy"));
94	    }
95	    let seen = &pane.last_observed;
96	    match (seen.shown.as_deref(), seen.driven.as_deref()) {
97	        (Some(shown), Some(driven)) if shown != driven => Some((
98	            PANE_SHOWN_DRIVEN_MISMATCH,
99	            "shows a session other than the one it drives",
100	        )),
101	        _ => None,
102	    }
103	}
~~~~

**E2: `crates/holler-hub/src/circuit/dispatch.rs:105-143`** — `pane_gate`: the join today is `pane.session_of_record == Some(session)`, where `session` is the name being sent.

Command: `sed -n '105,143p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=105 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
105	/// The pane-state gate: refuse a prompt to `session` (the bare session name being
106	/// sent) while any pane whose session of record it is has a refusal
107	/// ([`pane_state_refusal`]). Fail-closed across panes on different bodies that
108	/// record the same session name: any bad one refuses, and the first bad one in name
109	/// order (`list` is name-sorted) is the one named. One synchronous read with no
110	/// `await`, so it is totally ordered against a pane write, as `holds.admit` is
111	/// against `hold`/`release`.
112	///
113	/// The refusal is `-32011 session_held` with `data.hold_kind = "pane"` and
114	/// `data.reason` the pane code (no `data.since`: the park's is epoch milliseconds,
115	/// not the RFC 3339 the field documents). `holler release` does not lift it; the
116	/// remedy is `holler pane unpark` or `holler pane doctor`.
117	///
118	/// A registry that cannot be read (`store-corrupt`, logged by the registry when it
119	/// failed to load) refuses nothing, as a hold file that cannot be read leaves the
120	/// hub with no holds (`crate::holds`): a broken pane file must not stop every
121	/// prompt to every session.
122	fn pane_gate(
123	    panes: &crate::panes::PaneState,
124	    session: &str,
125	) -> Result<(), holler_proto::WireError> {
126	    use holler_pane::PaneStore as _;
127	    let Ok(records) = panes.list() else {
128	        return Ok(());
129	    };
130	    let refused = records
131	        .iter()
132	        .filter(|pane| pane.session_of_record.as_deref() == Some(session))
133	        .find_map(|pane| pane_state_refusal(pane).map(|refusal| (pane, refusal)));
134	    match refused {
135	        Some((pane, (code, why))) => Err(holler_proto::WireError::new(
136	            holler_proto::Code::SessionHeld,
137	            format!("pane {} {why}, so it refuses prompts ({code})", pane.name),
138	            Some(code),
139	        )
140	        .with_hold_kind("pane")),
141	        None => Ok(()),
142	    }
143	}
~~~~

**E3: `crates/holler-hub/src/circuit/dispatch.rs:145-188`** — `send_prompt`, the one place a `session/prompt` is put on a socket: hold first, then `pane_gate`.

Command: `sed -n '145,188p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=145 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
145	/// Send a `session/prompt {session, message, queue, replace}` request to the
146	/// body under `request_id`. `replace` (issue #191) is set only by
147	/// `interrupt SESSION TEXT`, after the matching cancel's own `{applied:true}`
148	/// — see `crate::interrupt`.
149	///
150	/// **This is the hub's one enforcement point for the session hold** (issue
151	/// #442). It is the only function in the hub that puts a `session/prompt` on
152	/// a socket, so checking here, before anything is built or sent, covers
153	/// `say`, `say --queue` and `say --replace` alike; a hold checked anywhere
154	/// else would be a bypass. `gate.key` is `<label>/<session>` built by the
155	/// caller from the connection's own authenticated label and the session name
156	/// it is about to send. The check is one synchronous map lookup with no
157	/// `await`, so it is totally ordered against `hold`/`release` (see
158	/// `crate::holds`). `crates/holler-cli/tests/hold_single_path_test.rs` fails
159	/// if a second sender of `session/prompt` appears.
160	///
161	/// The pane-state gate (story #646, [`pane_gate`]) is checked here too, after
162	/// the hold: an operator or default hold refuses first, with its own kind.
163	pub(super) async fn send_prompt<Snk>(
164	    sink: &mut Snk,
165	    gate: HoldGate<'_>,
166	    request_id: &str,
167	    session: &str,
168	    message: Box<holler_proto::Message>,
169	    queue: bool,
170	    replace: bool,
171	) -> Result<(), SendPromptError>
172	where
173	    Snk: Sink<Message, Error = WsError> + Unpin,
174	{
175	    // `admit` decides and, for a valid grant, consumes it in one critical
176	    // section: the prompt is accepted here, so the grant is spent here.
177	    gate.holds.admit(gate.key, gate.grant).map_err(SendPromptError::Refused)?;
178	    if let Some(panes) = gate.panes.as_deref() {
179	        pane_gate(panes, session).map_err(SendPromptError::Refused)?;
180	    }
181	    let cid = holler_proto::CorrelationId::parse(request_id).map_err(|_| SendPromptError::Io)?;
182	    let params = holler_proto::Prompt { session: session.to_string(), message: *message, meta: None, queue, replace };
183	    let req = Envelope::request(&cid, "session/prompt", Some(serde_json::to_value(params).map_err(|_| SendPromptError::Io)?));
184	    let text = holler_proto::encode(&req).map_err(|_| SendPromptError::Io)?;
185	    super::log_frame(super::LogDirection::Out, "session/prompt", Some(request_id), &text);
186	    sink.send(Message::text(text)).await.map_err(|_| SendPromptError::Io)?;
187	    sink.flush().await.map_err(|_| SendPromptError::Io)
188	}
~~~~

**E4: `crates/holler-hub/src/circuit.rs:737-756`** — The connection's `LiveCommand::Say` arm: the key is `<label>/<session>`, the gate is built here, `send_prompt` is called with the body's session name. This file is 898 lines.

Command: `sed -n '737,756p' crates/holler-hub/src/circuit.rs | awk -v s=737 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
737	            LiveCommand::Say { request_id, session, message, queue, replace, grant, reply } => {
738	                let key = crate::holds::session_key(self.label, &session);
739	                let gate = HoldGate { holds: self.registry.holds(), key: &key, grant: grant.as_deref(), panes: self.registry.panes().cloned() };
740	                match dispatch::send_prompt(self.sink, gate, &request_id, &session, message, queue, replace).await {
741	                    Ok(()) => {
742	                        self.pending_says.insert(request_id, PendingSay { reply, updates: Vec::new() });
743	                        Ok(())
744	                    }
745	                    // Issue #442: a held session refuses the prompt (nothing
746	                    // was sent); the connection itself is fine.
747	                    Err(SendPromptError::Refused(err)) => {
748	                        let _ = reply.send(SayReply::Refused(err));
749	                        Ok(())
750	                    }
751	                    Err(SendPromptError::Io) => {
752	                        let _ = reply.send(SayReply::ConnectionLost);
753	                        Err(())
754	                    }
755	                }
756	            }
~~~~

**E5: `crates/holler-hub/src/live.rs:195-205`** — `SayOpts`, the delivery options `talk` builds (live.rs is 884 lines).

Command: `sed -n '195,205p' crates/holler-hub/src/live.rs | awk -v s=195 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
195	/// The delivery options of one [`LiveHandle::say`] (bundled to keep its
196	/// argument count under the workspace's clippy limit).
197	#[derive(Default)]
198	pub struct SayOpts {
199	    /// `say --queue`.
200	    pub queue: bool,
201	    /// `interrupt SESSION TEXT`'s redirect.
202	    pub replace: bool,
203	    /// A one-time release grant id (issue #460).
204	    pub grant: Option<String>,
205	}
~~~~

**E6: `crates/holler-hub/src/live.rs:265-321`** — `LiveHandle::query` (the allowlist: `query/*` and `session/answer`) and `LiveHandle::say`.

Command: `sed -n '265,321p' crates/holler-hub/src/live.rs | awk -v s=265 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
265	    /// Forward one `query/*` request to this body and await its answer,
266	    /// bounded by `timeout` (issue #185: `hub query TARGET CMD…`). The outer
267	    /// `Option` is `None` when the body never answered in time (or the
268	    /// connection task is gone) — the caller reports that as `-32004
269	    /// not_connected`, same as `ping`. The inner `Result` is the body's own
270	    /// answer: `Ok` a result document, `Err` a wire error it raised (e.g.
271	    /// `query/support`'s `-32006 unknown_feature`), forwarded verbatim.
272	    pub async fn query(
273	        &self,
274	        method: &str,
275	        params: Option<Value>,
276	        timeout: std::time::Duration,
277	    ) -> Option<Result<Value, WireError>> {
278	        // Issue #442: this is a generic request forward, so it must not be a
279	        // second way to deliver a prompt. Only the `query/*` family and the
280	        // hub's own `session/answer` (which resolves a held question, it does
281	        // not start a turn) may pass; `session/prompt` goes through `say`,
282	        // where the session hold is enforced.
283	        if !(method.starts_with("query/") || method == "session/answer") {
284	            return Some(Err(WireError::new(
285	                holler_proto::Code::MethodNotFound,
286	                format!("{method} cannot be forwarded to a body"),
287	                None,
288	            )));
289	        }
290	        let (reply_tx, reply_rx) = oneshot::channel();
291	        if self
292	            .tx
293	            .send(LiveCommand::Query { method: method.to_string(), params, reply: reply_tx })
294	            .is_err()
295	        {
296	            return None;
297	        }
298	        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
299	    }
300	
301	    /// Ask the live connection to run one `say` turn and await its outcome,
302	    /// bounded by `timeout` (the CLI's own `--timeout`, default 600s).
303	    /// `None` means the connection task is gone (the socket dropped) or did
304	    /// not answer within `timeout` at all — the caller maps that to the
305	    /// spec's `no reply from … within Ns` / `connection_lost` wording.
306	    pub async fn say(
307	        &self,
308	        request_id: String,
309	        session: String,
310	        message: Box<Message>,
311	        opts: SayOpts,
312	        timeout: std::time::Duration,
313	    ) -> Option<SayReply> {
314	        let (reply_tx, reply_rx) = oneshot::channel();
315	        let SayOpts { queue, replace, grant } = opts;
316	        let cmd = LiveCommand::Say { request_id, session, message, queue, replace, grant, reply: reply_tx };
317	        if self.tx.send(cmd).is_err() {
318	            return None;
319	        }
320	        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
321	    }
~~~~

**E7: `crates/holler-hub/src/live.rs:740-788`** — `Registry::resolve_session`: a name is matched against `ad.name` and nothing else; `matches_offline` likewise.

Command: `sed -n '740,788p' crates/holler-hub/src/live.rs | awk -v s=740 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
740	    /// Resolve a `say`/`interrupt` target name against every live body's
741	    /// cached presence (ADR 0003's bare/label rules, the minimal slice this
742	    /// story needs): a `<label>/<session>` name matches only that body's
743	    /// session; a bare name matches by session name across every live body.
744	    pub async fn resolve_session(&self, name: &holler_proto::RoutableName) -> ResolveOutcome {
745	        let inner = self.inner.lock().await;
746	        let mut matches: Vec<(LiveHandle, SessionAd)> = Vec::new();
747	        for handle in inner.values() {
748	            if name.has_label() && handle.hostname != name.label() {
749	                continue;
750	            }
751	            let Some(sessions) = handle.presence.lock().await.clone() else { continue };
752	            for ad in sessions {
753	                if ad.name == name.session() {
754	                    matches.push((handle.clone(), ad));
755	                }
756	            }
757	        }
758	        drop(inner);
759	        if matches.is_empty() {
760	            return if self.matches_offline(name).await {
761	                ResolveOutcome::NotConnected
762	            } else {
763	                ResolveOutcome::Unknown
764	            };
765	        }
766	        if matches.len() == 1 {
767	            // Guarded by the length check just above, so this never panics —
768	            // `Vec::remove` at a valid index, not an `.expect()`/`.unwrap()`
769	            // escape hatch on a fallible value.
770	            let (handle, ad) = matches.remove(0);
771	            return ResolveOutcome::Found(handle, Box::new(ad));
772	        }
773	        let candidates = matches.into_iter().map(|(h, ad)| format!("{}/{}", h.hostname, ad.name)).collect();
774	        ResolveOutcome::Ambiguous(candidates)
775	    }
776	
777	    /// `true` iff a now-disconnected body's last-known presence had a
778	    /// session matching `name` (the `not_connected` case — see the
779	    /// `offline` field's own doc).
780	    async fn matches_offline(&self, name: &holler_proto::RoutableName) -> bool {
781	        self.offline.lock().await.values().any(|(hostname, sessions)| {
782	            if name.has_label() && hostname != name.label() {
783	                return false;
784	            }
785	            sessions.iter().any(|ad| ad.name == name.session())
786	        })
787	    }
788	}
~~~~

**E8: `crates/holler-hub/src/live.rs:798-810`** — `ResolveOutcome`.

Command: `sed -n '798,810p' crates/holler-hub/src/live.rs | awk -v s=798 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
798	/// The outcome of [`Registry::resolve_session`].
799	pub enum ResolveOutcome {
800	    /// Exactly one live session matched.
801	    Found(LiveHandle, Box<SessionAd>),
802	    /// No live session matched, live or offline.
803	    Unknown,
804	    /// A session by this name was last seen on a body that is not currently
805	    /// connected (issue #190's own stand-in — see the `offline` field doc).
806	    NotConnected,
807	    /// More than one live session matched a bare name — the candidates are
808	    /// `<label>/<session>` strings for the spec's "lists candidates" wording.
809	    Ambiguous(Vec<String>),
810	}
~~~~

**E9: `crates/holler-hub/src/talk.rs:196-218`** — `talk::say`: the target must parse as a `RoutableName`, else `UnknownSession`; then the hold fast path.

Command: `sed -n '196,218p' crates/holler-hub/src/talk.rs | awk -v s=196 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
196	pub async fn say(
197	    registry: &Registry,
198	    roster: &Roster,
199	    state: &HubState,
200	    args: SayArgs<'_>,
201	) -> Result<SayOutcome, SayError> {
202	    let SayArgs { session, text, queue, grant, timeout } = args;
203	    let name = holler_proto::RoutableName::parse(session).map_err(|_| SayError::UnknownSession)?;
204	    let (handle, ad) = match registry.resolve_session(&name).await {
205	        ResolveOutcome::Found(h, ad) => (h, ad),
206	        ResolveOutcome::Unknown => return Err(SayError::UnknownSession),
207	        ResolveOutcome::NotConnected => return Err(SayError::NotConnected(not_connected_detail(roster, session))),
208	        ResolveOutcome::Ambiguous(candidates) => return Err(SayError::Ambiguous(candidates)),
209	    };
210	
211	    // Issue #442: a held session refuses new work before anything else is
212	    // done for it (no turn id moved, no TalkLog line). This is a fast path
213	    // only: the enforcement that cannot be raced is the check in
214	    // `circuit::dispatch::send_prompt`, which every prompt passes through.
215	    if let Some(refusal) = registry.holds().would_refuse(&crate::holds::session_key(&handle.hostname, ad.name.as_str()), grant) {
216	        return Err(SayError::Refused(refusal));
217	    }
218	
~~~~

**E10: `crates/holler-hub/src/talk.rs:326-340`** — `send_turn` builds `SayOpts` and calls `LiveHandle::say` with `ad.name`.

Command: `sed -n '326,340p' crates/holler-hub/src/talk.rs | awk -v s=326 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
326	
327	    append_talklog(state, &handle.hostname, ad.name.as_str(), &TalkLine::Prompt {
328	        prompt_id: request_id.clone(),
329	        text: text.to_string(),
330	    });
331	
332	    let reply = handle
333	        .say(
334	            request_id.clone(),
335	            ad.name.clone(),
336	            Box::new(message),
337	            SayOpts { queue: kind.queue(), replace: kind.replace(), grant: grant.map(str::to_owned) },
338	            timeout,
339	        )
340	        .await;
~~~~

**E11: `crates/holler-hub/src/talk.rs:140-162`** — `talk::answer`: the same parse and resolve, and the use of `LiveHandle::query` for `session/answer` ("rather than adding a dedicated `LiveCommand` variant").

Command: `sed -n '140,162p' crates/holler-hub/src/talk.rs | awk -v s=140 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
140	pub async fn answer(
141	    registry: &Registry,
142	    session: &str,
143	    choice: &str,
144	    timeout: Duration,
145	) -> Result<AnswerOutcome, AnswerError> {
146	    let name = holler_proto::RoutableName::parse(session).map_err(|_| AnswerError::UnknownSession)?;
147	    let (handle, ad) = match registry.resolve_session(&name).await {
148	        ResolveOutcome::Found(h, ad) => (h, ad),
149	        ResolveOutcome::Unknown => return Err(AnswerError::UnknownSession),
150	        ResolveOutcome::NotConnected => return Err(AnswerError::NotConnected),
151	        ResolveOutcome::Ambiguous(candidates) => return Err(AnswerError::Ambiguous(candidates)),
152	    };
153	    let params = serde_json::json!({ "session": ad.name, "choice": choice });
154	    match handle.query("session/answer", Some(params), timeout).await {
155	        None => Err(AnswerError::ConnectionLost),
156	        Some(Err(e)) => Err(AnswerError::Refused(e)),
157	        Some(Ok(value)) => {
158	            let applied = value.get("applied").and_then(|v| v.as_bool()).unwrap_or(false);
159	            Ok(AnswerOutcome { session: format!("{}/{}", handle.hostname, ad.name), applied })
160	        }
161	    }
162	}
~~~~

**E12: `crates/holler-hub/src/interrupt.rs:95-120`** — `interrupt`: the same parse and resolve.

Command: `sed -n '95,120p' crates/holler-hub/src/interrupt.rs | awk -v s=95 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
95	pub async fn interrupt(
96	    registry: &Registry,
97	    roster: &Roster,
98	    state: &HubState,
99	    session: &str,
100	    text: Option<&str>,
101	) -> Result<InterruptOutcome, InterruptError> {
102	    let name = holler_proto::RoutableName::parse(session).map_err(|_| InterruptError::UnknownSession)?;
103	    let (handle, ad) = match registry.resolve_session(&name).await {
104	        ResolveOutcome::Found(h, ad) => (h, ad),
105	        ResolveOutcome::Unknown => return Err(InterruptError::UnknownSession),
106	        ResolveOutcome::NotConnected => return Err(InterruptError::NotConnected),
107	        ResolveOutcome::Ambiguous(candidates) => return Err(InterruptError::Ambiguous(candidates)),
108	    };
109	
110	    // Issue #442: `interrupt` keeps working on a held session, but its
111	    // redirect half delivers a new prompt, which a hold refuses. Refuse the
112	    // whole redirect up front rather than cancel the turn and then fail to
113	    // deliver what was meant to replace it. (The refusal that cannot be raced
114	    // is still `send_prompt`'s; a hold set between this check and the redirect
115	    // surfaces as `PromptFailed`.)
116	    if text.is_some() {
117	        if let Some(refusal) = registry.holds().would_refuse(&crate::holds::session_key(&handle.hostname, ad.name.as_str()), None) {
118	            return Err(InterruptError::Refused(refusal));
119	        }
120	    }
~~~~

**E13: `crates/holler-hub/src/control_server.rs:183-202`** — `dispatch_allowlisted`: one implementation per verb, shared by `control/*` and `admin/*`.

Command: `sed -n '183,202p' crates/holler-hub/src/control_server.rs | awk -v s=183 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
183	pub(crate) async fn dispatch_allowlisted(
184	    verb: &str,
185	    cid: &holler_proto::CorrelationId,
186	    obj: &serde_json::Value,
187	    registry: &Registry,
188	    roster: &Roster,
189	    lockout: &Lockout,
190	) -> Option<String> {
191	    Some(match verb {
192	        "status" => encode_response(cid, crate::control_status::status_doc(registry, lockout).await),
193	        "roster" => roster_control(cid, obj, roster, registry).await,
194	        "say" => say(cid, obj, registry, roster).await,
195	        "interrupt" => interrupt(cid, obj, registry, roster).await,
196	        "answer" => answer(cid, obj, registry).await,
197	        "wait" => wait(cid, obj, roster).await,
198	        "query_local" => hub_query_local(cid, obj, registry).await,
199	        "query_remote" => hub_query_remote(cid, obj, registry).await,
200	        _ => return None,
201	    })
202	}
~~~~

**E14: `crates/holler-hub/src/control_server.rs:287-312`** — `control/say`: the param mapping (this file is 837 lines).

Command: `sed -n '287,312p' crates/holler-hub/src/control_server.rs | awk -v s=287 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
287	/// `control/say {session, text, timeout_ms, queue}` (issue #190): the CLI's
288	/// `say` verb, relayed to [`crate::talk::say`]. See that module for the full
289	/// resolve → busy-check → send → collect → TalkLog flow; this is only the
290	/// control-socket param/result mapping.
291	async fn say(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
292	    let params = obj.get("params");
293	    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
294	        return encode_error(cid, Code::InvalidParams, "control/say needs params.session".to_string());
295	    };
296	    let Some(text) = params.and_then(|p| p.get("text")).and_then(|v| v.as_str()) else {
297	        return encode_error(cid, Code::InvalidParams, "control/say needs params.text".to_string());
298	    };
299	    let queue = params.and_then(|p| p.get("queue")).and_then(|v| v.as_bool()).unwrap_or(false);
300	    let timeout_ms = params.and_then(|p| p.get("timeout_ms")).and_then(|v| v.as_u64()).unwrap_or(600_000);
301	    let timeout = std::time::Duration::from_millis(timeout_ms);
302	
303	    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
304	    let grant = params.and_then(|p| p.get("grant")).and_then(|v| v.as_str());
305	    match crate::talk::say(registry, roster, &state, crate::talk::SayArgs { session, text, queue, grant, timeout }).await {
306	        Ok(outcome) => encode_response(cid, serde_json::json!({
307	            "session": outcome.session,
308	            "stop_reason": outcome.stop_reason,
309	            "state": outcome.state,
310	            "updates": outcome.updates,
311	            "elapsed_ms": outcome.elapsed_ms,
312	            "text": crate::talk::reply_text(&outcome.message),
~~~~

**E15: `crates/holler-hub/src/holds.rs:74-79`** — The hold key, built by one function.

Command: `sed -n '74,79p' crates/holler-hub/src/holds.rs | awk -v s=74 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
74	/// The registry key for a session: `<label>/<session>`, where `label` is the
75	/// token label its body authenticated as. Every place that checks or sets a
76	/// hold builds the key with this one function.
77	pub fn session_key(label: &str, session: &str) -> String {
78	    format!("{label}/{session}")
79	}
~~~~

**E16: `crates/holler-hub/src/holds.rs:350-365`** — `admit` (the authority) and `would_refuse` (the fast path, "never the authority").

Command: `sed -n '350,365p' crates/holler-hub/src/holds.rs | awk -v s=350 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
350	    /// **The enforcement decision** (issue #442, #460), called from
351	    /// `circuit::dispatch::send_prompt` and nowhere else that delivers: `Ok` to
352	    /// deliver the prompt, or the refusal to send instead. A valid `grant` for
353	    /// this session is consumed here, in the same critical section that
354	    /// decides, so of two racing senders only one can be admitted with it.
355	    pub fn admit(&self, key: &str, grant: Option<&str>) -> Result<(), holler_proto::WireError> {
356	        decide(&mut self.state(), key, grant, true)
357	    }
358	
359	    /// What [`Holds::admit`] would answer right now, without consuming
360	    /// anything: the fast path that keeps a refused prompt from leaving
361	    /// side effects behind. Never the authority (a grant can be consumed
362	    /// between this and `admit`).
363	    pub fn would_refuse(&self, key: &str, grant: Option<&str>) -> Option<holler_proto::WireError> {
364	        decide(&mut self.state(), key, grant, false).err()
365	    }
~~~~

**E17: `crates/holler-hub/src/holds.rs:428-460`** — `note_joined`: a default hold is set on the first sight of a key that matches `--join-held`.

Command: `sed -n '428,460p' crates/holler-hub/src/holds.rs | awk -v s=428 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
428	
429	    /// A body advertised these sessions (issue #460). A session *joins* when it
430	    /// matches a `--join-held` pattern and has no default hold, unless one was
431	    /// released in this process: it then gets a default hold (reason
432	    /// [`JOIN_REASON`]). Nothing is remembered about sessions that do not
433	    /// match, so memory grows only with the holds themselves. Returns how many
434	    /// were held. Cheap when `--join-held` is off: no lock is taken.
435	    pub fn note_joined(&self, keys: impl IntoIterator<Item = String>) -> usize {
436	        let patterns = self.shared.join_held.lock().unwrap_or_else(PoisonError::into_inner).clone();
437	        if patterns.is_empty() {
438	            return 0;
439	        }
440	        let mut newly = Vec::new();
441	        {
442	            let mut st = self.state();
443	            for key in keys {
444	                if st.default.contains_key(&key) || st.released.contains(&key) {
445	                    continue;
446	                }
447	                if patterns.iter().any(|pat| glob_match(pat, &key)) {
448	                    st.default.insert(key.clone(), HoldInfo { reason: Some(JOIN_REASON.to_owned()), since: log::timestamp() });
449	                    newly.push(key);
450	                }
451	            }
452	        }
453	        if !newly.is_empty() {
454	            self.sync(true);
455	            for key in &newly {
456	                emit(Severity::Info, "session_held", vec![("session", key.clone()), ("kind", "default".to_owned())]);
457	            }
458	        }
459	        newly.len()
460	    }
~~~~

**E18: `crates/holler-hub/src/roster.rs:131-180`** — The roster row: `name`, `mode`, `state`, `conn_state`, `token_id`, `hostname`, `harness_session_id`, and the flattened hold fields. roster.rs is 892 lines.

Command: `sed -n '131,180p' crates/holler-hub/src/roster.rs | awk -v s=131 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
131	/// One roster row: one routable session name a body advertises.
132	///
133	/// `name` keys the row (a `RoutableName`'s string form, `label/session`). The
134	/// transport's liveness (`conn_state`) is separate from the A2A session state
135	/// (`state`) — a body can be `connected` but `idle`, or `gone` while still
136	/// `working` in the last presence.
137	#[derive(Debug, Clone, PartialEq, serde::Serialize)]
138	pub struct Row {
139	    /// The routable name (`label/session`).
140	    pub name: String,
141	    /// The harness hosting this session.
142	    pub harness: String,
143	    /// How the session was established: `spawn` / `attach`.
144	    pub mode: String,
145	    /// The A2A session state (`idle` / `working` / `input-required`) — the
146	    /// *stored* value. [`Roster::rows`] displays `stalled` in its place when a
147	    /// `working` row's `last_update_at` has aged past the stall threshold.
148	    pub state: String,
149	    /// The transport's liveness: `connected` / `reconnecting` / `gone`.
150	    pub conn_state: &'static str,
151	    /// The token that currently holds the row.
152	    pub token_id: String,
153	    /// The body's client id.
154	    pub client_id: String,
155	    /// The body's hostname.
156	    pub hostname: String,
157	    /// The attached harness session id (attach mode only).
158	    pub harness_session_id: Option<String>,
159	    /// When the row was first seen (epoch seconds).
160	    pub first_seen: u64,
161	    /// When the token was last heard from (epoch seconds) — the TTL's input.
162	    pub last_seen: u64,
163	    /// When the in-flight turn last produced a `session/update` (epoch
164	    /// seconds), or `None` for a non-`working` row — the `stalled` input.
165	    pub last_update_at: Option<u64>,
166	    /// The held permission(s)/elicitation(s) (issue #151) — the `PENDING`
167	    /// column; `--json` carries the full array.
168	    pub pending: Option<Vec<PendingItem>>,
169	    /// The JSON-RPC id of the current / most recent `session/prompt` (issue
170	    /// #142) — the `holler wait --after` basis.
171	    pub turn_id: Option<String>,
172	    /// The most recently completed turn (issue #142) — the `LAST TURN` column.
173	    pub last_turn: Option<LastTurn>,
174	    /// The session hold (issue #442), filled in from the hub's hold registry
175	    /// when the row is read by `control/roster`; the roster itself never holds
176	    /// it (a hold belongs to the session name, not to a row). Flattened, and
177	    /// absent for a session that is not held.
178	    #[serde(flatten)]
179	    pub hold: holler_proto::SessionHold,
180	}
~~~~

**E19: `crates/holler-hub/src/control.rs:60-95`** — `ControlCall` and its `roster` and `say_with` constructors; `query_remote` follows at line 135.

Command: `sed -n '60,95p' crates/holler-hub/src/control.rs | awk -v s=60 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
60	/// from `control/x` to `admin/x` and handing `call.params`/`call.timeout` to
61	/// `holler_body::admin_client` is the whole transport switch — no verb
62	/// re-builds its own params/timeout a second time for the remote path. The
63	/// public fns below (`roster`, `say_with`, the `_at` variants, …) keep their
64	/// existing signatures and behaviour; each is now a thin wrapper around one
65	/// of these constructors.
66	pub struct ControlCall {
67	    pub method: &'static str,
68	    pub params: Option<serde_json::Value>,
69	    pub timeout: std::time::Duration,
70	}
71	
72	impl ControlCall {
73	    pub fn status() -> Self {
74	        Self { method: "control/status", params: None, timeout: CLIENT_TIMEOUT }
75	    }
76	    pub fn roster(all: bool, prefix: Option<&str>) -> Self {
77	        Self {
78	            method: "control/roster",
79	            params: Some(serde_json::json!({ "all": all, "prefix": prefix })),
80	            timeout: CLIENT_TIMEOUT,
81	        }
82	    }
83	    pub fn say_with(session: &str, text: &str, queue: bool, grant: Option<&str>, timeout: std::time::Duration) -> Self {
84	        Self {
85	            method: "control/say",
86	            params: Some(serde_json::json!({
87	                "session": session,
88	                "text": text,
89	                "queue": queue,
90	                "grant": grant,
91	                "timeout_ms": u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
92	            })),
93	            timeout: timeout + std::time::Duration::from_secs(5),
94	        }
95	    }
~~~~

**E20: `crates/holler-cli/src/say_cmd.rs:213-223`** — The three pane codes, declared in the CLI.

Command: `sed -n '213,223p' crates/holler-cli/src/say_cmd.rs | awk -v s=213 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
213	// --- routing by pane (story #646, part 3) ----------------------------------------
214	
215	/// A pane that is parked (`holler pane park`): its prompts are refused until it is
216	/// unparked. The three open codes of the routed verbs are declared here, the file
217	/// that owns the routing (ADR-0021 section 9); each is a refusal, exit 3.
218	pub const PANE_PARKED: RefusalCode = RefusalCode::from_static("pane-parked");
219	/// A pane whose record says its harness server is unhealthy.
220	pub const PANE_UNHEALTHY: RefusalCode = RefusalCode::from_static("pane-unhealthy");
221	/// A pane whose record says it shows one session while the hub drives another.
222	pub const PANE_SHOWN_DRIVEN_MISMATCH: RefusalCode =
223	    RefusalCode::from_static("pane-shown-driven-mismatch");
~~~~

**E21: `crates/holler-cli/src/say_cmd.rs:288-318`** — `resolve_pane_target`: the CLI sends the pane's `session_of_record` as the hub target; the twin predicate is named in its doc.

Command: `sed -n '288,318p' crates/holler-cli/src/say_cmd.rs | awk -v s=288 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
288	/// Resolve `--pane NAME [--profile P]` to the pane's session of record, checking in
289	/// this order: the pane name (`usage`); with `--profile`, the profile scope
290	/// (`ProfileScope::resolve`, so `profile-not-found` and `pane-not-in-profile` come
291	/// from the one helper, as for `pane park`); the record (`pane-not-found`); its session
292	/// of record (`session-not-found`, naming the pane); then what the record says about
293	/// the pane: parked ([`PANE_PARKED`]), unhealthy ([`PANE_UNHEALTHY`], with the reason)
294	/// or SHOWN and DRIVEN both observed and differing ([`PANE_SHOWN_DRIVEN_MISMATCH`]).
295	/// The record is the source; nothing live is probed.
296	///
297	/// The pane-state predicate has a twin at the hub's `send_prompt` gate
298	/// (`holler-hub/src/circuit/dispatch.rs`, `pane_state_refusal`), which also refuses a
299	/// bare `say` to the session; it exists twice only because `holler-pane` is frozen and
300	/// the hub cannot depend on this crate. Keep the two equal.
301	pub fn resolve_pane_target(
302	    ports: Ports<'_>,
303	    pane: &str,
304	    profile: Option<&str>,
305	) -> Result<String, PaneError> {
306	    let name = PaneName::parse(pane)?;
307	    let profile = profile.map(ProfileName::parse).transpose()?;
308	    let record = routed_record(ports, &name, profile.as_ref())?;
309	    let Some(session) = record.session_of_record.clone() else {
310	        return Err(PaneError::SessionNotFound {
311	            what: format!("{name} has no session of record"),
312	        });
313	    };
314	    match pane_state_refusal(&record) {
315	        Some(refusal) => Err(refusal),
316	        None => Ok(session),
317	    }
318	}
~~~~

**E22: `crates/holler-cli/src/say_cmd.rs:341-372`** — `pane_state_refusal` (copy 2 of 2).

Command: `sed -n '341,372p' crates/holler-cli/src/say_cmd.rs | awk -v s=341 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
341	/// What the record says against prompting the pane, in the order parked, unhealthy,
342	/// SHOWN differing from DRIVEN (one side absent is not a mismatch); each message names
343	/// the pane, the code and the remedy on one line (stored text is `quoted`).
344	fn pane_state_refusal(record: &Pane) -> Option<PaneError> {
345	    let name = &record.name;
346	    let doctor = doctor_command(Some(name), false);
347	    let (code, message) = if let Hold::Parked { reason, .. } = &record.hold {
348	        let reason = quoted(reason);
349	        let message = format!("{name} is parked ({PANE_PARKED}, reason {reason}); run holler pane unpark {name} first");
350	        (PANE_PARKED, message)
351	    } else if let Health::Unhealthy(reason) = &record.harness.health {
352	        let reason = quoted(reason);
353	        (
354	            PANE_UNHEALTHY,
355	            format!("{name} is unhealthy ({PANE_UNHEALTHY}): {reason}; run {doctor}"),
356	        )
357	    } else {
358	        let seen = &record.last_observed;
359	        let (Some(shown), Some(driven)) = (seen.shown.as_deref(), seen.driven.as_deref()) else {
360	            return None;
361	        };
362	        if shown == driven {
363	            return None;
364	        }
365	        let (shown, driven) = (quoted(shown), quoted(driven));
366	        let message = format!(
367	            "{name} shows session {shown} but the hub drives {driven} ({PANE_SHOWN_DRIVEN_MISMATCH}); run {doctor}"
368	        );
369	        (PANE_SHOWN_DRIVEN_MISMATCH, message)
370	    };
371	    Some(PaneError::Refused { code, message })
372	}
~~~~

**E23: `crates/holler-cli/src/say_cmd.rs:376-419`** — `QUEUE_ACCEPT_WAIT`, `QueueWait` and `wait_ended`: a read deadline of the accept wait is reported as accepted.

Command: `sed -n '376,419p' crates/holler-cli/src/say_cmd.rs | awk -v s=376 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
376	/// How long the routed `say --pane NAME --queue` waits for the hub before it reports
377	/// the prompt queued. It replaces only the control call's own wait: the prompt's
378	/// `timeout_ms` stays `--timeout`, so the hub keeps waiting for the turn after the CLI
379	/// has returned. A refusal (busy is not one with `--queue`; a hold or the pane gate is)
380	/// answers well inside it.
381	pub const QUEUE_ACCEPT_WAIT: Duration = Duration::from_secs(2);
382	
383	/// What the wait of a `say` ended with.
384	#[derive(Debug)]
385	pub enum QueueWait {
386	    /// The turn finished inside the wait; the control call answered this document.
387	    Reply(serde_json::Value),
388	    /// The CLI's own acceptance deadline ([`QUEUE_ACCEPT_WAIT`]) expired after the
389	    /// prompt was accepted.
390	    Deadline,
391	}
392	
393	/// Whether this `say` returns once its prompt is accepted: the routed (`--pane`)
394	/// `--queue` form over the local control socket. A remote hub (`--server`) cannot tell
395	/// a read deadline from a dropped connection, so it keeps the reply wait; the bare
396	/// `say --queue SESSION` keeps it too (#190's contract).
397	fn returns_on_acceptance(say: &Say) -> bool {
398	    say.queue && say.pane.is_some() && say.server.is_none()
399	}
400	
401	/// The control call's result as a [`QueueWait`]: a reply, or, when the call waited
402	/// only [`QUEUE_ACCEPT_WAIT`] (`returns_on_acceptance`), its read deadline expiring
403	/// with no answer, which means the hub accepted the prompt. Any other error is passed
404	/// on to be printed as a refusal.
405	fn wait_ended(
406	    reply: Result<serde_json::Value, ControlError>,
407	    returns_on_acceptance: bool,
408	) -> Result<QueueWait, ControlError> {
409	    match reply {
410	        Ok(doc) => Ok(QueueWait::Reply(doc)),
411	        Err(ControlError::Io(e))
412	            if returns_on_acceptance
413	                && matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
414	        {
415	            Ok(QueueWait::Deadline)
416	        }
417	        Err(other) => Err(other),
418	    }
419	}
~~~~

**E24: `crates/holler-cli/src/say_cmd.rs:440-494`** — `pane_hold_refusal`: the hub's pane arm of `session_held`, rendered with the pane's remedy, exit 3.

Command: `sed -n '440,494p' crates/holler-cli/src/say_cmd.rs | awk -v s=440 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
440	// --- the pane arm of a held refusal (story #646, part 3) -------------------------
441	
442	/// The `-32011 session_held` of the hub's pane-state gate, as `--json` prints it: the
443	/// held form's object (`hold_cmd::held_refusal`) with `hold_kind: "pane"` and no
444	/// `since`, in this key order.
445	#[derive(Serialize)]
446	struct PaneHeld<'a> {
447	    error: &'static str,
448	    session: &'a str,
449	    reason: &'a str,
450	    hold_kind: &'static str,
451	}
452	
453	/// Render a `-32011 session_held` refusal of `session` whose `data.hold_kind` is
454	/// `"pane"`: the hub's pane-state gate refused the prompt (`data.reason` is the pane
455	/// code). One line naming the code with its remedy (`holler pane unpark` for
456	/// `pane-parked`, `holler pane doctor` otherwise), exit 3 like every pane-routed
457	/// refusal; never the generic held tail nor `HELD_EXIT_CODE`, because `holler release`
458	/// does not lift it. JSON: the held form's object with `hold_kind: "pane"`, on stdout.
459	/// `None` for every other error, which the caller's generic arms then render.
460	pub fn pane_hold_refusal(
461	    session: &str,
462	    e: &holler_proto::WireError,
463	    json: bool,
464	) -> Option<SayResult> {
465	    let data = e.data.as_deref()?;
466	    if !crate::hold_cmd::is_held(e) || data.hold_kind.as_deref() != Some("pane") {
467	        return None;
468	    }
469	    let code = data.reason.as_deref().unwrap_or_default();
470	    let exit_code = ErrorClass::Refusal.exit_code();
471	    if json {
472	        let doc = PaneHeld {
473	            error: "session_held",
474	            session,
475	            reason: code,
476	            hold_kind: "pane",
477	        };
478	        return Some(match serde_json::to_string(&doc) {
479	            Ok(line) => SayResult {
480	                message: line,
481	                to_stderr: false,
482	                exit_code,
483	            },
484	            Err(cause) => err(format!("cannot encode the refusal: {cause}"), exit_code),
485	        });
486	    }
487	    let remedy = if code == PANE_PARKED.as_str() {
488	        "unpark the pane with holler pane unpark"
489	    } else {
490	        "check the pane with holler pane doctor"
491	    };
492	    let message = coded(code, e.message.clone());
493	    Some(err(format!("{message}; {remedy}"), exit_code))
494	}
~~~~

**E25: `crates/holler-cli/src/pane/wiring.rs:108-128`** — `Wiring::ports`: the seven ports.

Command: `sed -n '108,128p' crates/holler-cli/src/pane/wiring.rs | awk -v s=108 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
108	    pub fn connect() -> Result<Self, PaneError> {
109	        store::reach_hub()?;
110	        let actor = Actor::parse(ACTOR)?;
111	        let lookup = |name: &str| {
112	            std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
113	        };
114	        Ok(Self::from_env(&lookup, actor))
115	    }
116	
117	    /// The ports, lent for one verb.
118	    pub fn ports(&self) -> Ports<'_> {
119	        Ports {
120	            pane_store: self.panes.as_ref(),
121	            profile_store: self.profiles.as_ref(),
122	            herdr: &self.herdr,
123	            host: &self.host,
124	            harness: &self.harness,
125	            scope: &self.scope,
126	            prober: &self.prober,
127	        }
128	    }
~~~~

**E26: `crates/holler-cli/src/pane/wiring.rs:205-216`** — `Unwired`, and its `PaneStore` impl (every method `NotImplemented`).

Command: `sed -n '205,216p' crates/holler-cli/src/pane/wiring.rs | awk -v s=205 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
205	/// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
206	#[derive(Debug, Clone, Copy, Default)]
207	pub struct Unwired;
208	
209	impl PaneStore for Unwired {
210	    fn get(&self, _name: &PaneName) -> Result<Option<Pane>, PaneError> {
211	        Err(PaneError::NotImplemented)
212	    }
213	
214	    fn list(&self) -> Result<Vec<Pane>, PaneError> {
215	        Err(PaneError::NotImplemented)
216	    }
~~~~

**E27: `crates/holler-pane/src/ports.rs:63-91`** — `PaneStore`.

Command: `sed -n '63,91p' crates/holler-pane/src/ports.rs | awk -v s=63 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
63	/// The registry of panes, kept by the hub (#639) and faked by the test kit (#638).
64	///
65	/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
66	/// thread) in async code. Every method returns within I5's bound (default 10 s) or
67	/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
68	///
69	/// Every write is a compare-and-swap on the pane's `generation` (see
70	/// [`crate::generation`]); a stale one is `generation-conflict`.
71	pub trait PaneStore: Send + Sync {
72	    /// The pane named `name`, or `None`.
73	    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;
74	
75	    /// Every pane.
76	    fn list(&self) -> Result<Vec<Pane>, PaneError>;
77	
78	    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
79	    /// new pane); returns the stored record with its bumped generation.
80	    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
81	
82	    /// Remove a pane's record if it is still at `expected_generation`. `close`
83	    /// uses it, so a closed pane's record does not outlive the pane. A stale
84	    /// generation is `generation-conflict`; a record that does not exist is
85	    /// `pane-not-found`, whatever `expected_generation` is (a missing record is
86	    /// checked first, so no store has to guess which of the two to answer).
87	    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;
88	
89	    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
90	    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
91	}
~~~~

**E28: `crates/holler-pane/src/ports.rs:344-366`** — `HarnessPort::send_prompt` and `session_activity` and their defaults.

Command: `sed -n '344,366p' crates/holler-pane/src/ports.rs | awk -v s=344 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
344	    /// Queue `text` as a prompt to `session` on the server at `port`, through the
345	    /// harness API, never by typing into a TUI (I4). `pane reset --first` calls it (#645).
346	    ///
347	    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
348	    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
349	    fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
350	        let _ = (port, session, text);
351	        Err(PaneError::Refused {
352	            code: PROMPT_UNSUPPORTED,
353	            message: "this harness does not queue a prompt".to_owned(),
354	        })
355	    }
356	
357	    /// What `session` on the server at `port` is doing now (#645: `pane reset` refuses a
358	    /// conversation that is not [`Activity::Idle`]).
359	    ///
360	    /// The default answers [`Activity::Idle`], so an impl that does not report activity
361	    /// refuses nothing.
362	    fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
363	        let _ = (port, session);
364	        Ok(Activity::Idle)
365	    }
366	}
~~~~

**E29: `crates/holler-pane/src/ports.rs:412-423`** — `Ports`: the struct every verb holds.

Command: `sed -n '412,423p' crates/holler-pane/src/ports.rs | awk -v s=412 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
412	/// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
413	/// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
414	#[derive(Clone, Copy)]
415	pub struct Ports<'a> {
416	    pub pane_store: &'a dyn PaneStore,
417	    pub profile_store: &'a dyn ProfileStore,
418	    pub herdr: &'a dyn HerdrPort,
419	    pub host: &'a dyn HostPort,
420	    pub harness: &'a dyn HarnessPort,
421	    pub scope: &'a dyn ProfileScope,
422	    pub prober: &'a dyn Prober,
423	}
~~~~

**E30: `crates/holler-pane/src/tx_switch.rs:1-50`** — The switch/reset engine's module doc: the step order, "left as stored (no port observes DRIVEN before #649)", "at most eight port calls".

Command: `sed -n '1,50p' crates/holler-pane/src/tx_switch.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The switch/reset transaction (story #645): the session a pane shows and the hub drives
2	//! changes through the harness API, the TUI and the record together, and is recorded only
3	//! once the TUI is seen showing it. `holler pane switch` points a pane at a session its server
4	//! already has ([`Target::Existing`]); `holler pane reset` starts it on one the run creates
5	//! ([`Target::Fresh`]). One engine runs both (ADR-0021 section 8, "Switch and reset as built").
6	//!
7	//! A run is plan, act, observe, record (I3), in this order:
8	//!
9	//! 1. **Plan**, with no write and no live change, so a refusal changes nothing. Re-check an
10	//!    existing target's id ([`parse_session_id`]). Read the record, through the profile's
11	//!    scope when the request names one (`pane-not-found`, `profile-not-found`,
12	//!    `pane-not-in-profile`). Refuse the orchestrator's pane unless the caller acts as the
13	//!    operator ([`ORCHESTRATOR_PANE`]). Check the pane's harness server live (I6), never from
14	//!    the stored `harness.health` ([`SERVER_UNHEALTHY`]). For a reset of a pane that has a
15	//!    session of record, ask the server what that session is doing (`session_activity`) and
16	//!    refuse one that is running a turn ([`SESSION_BUSY`]) or holds a question
17	//!    ([`SESSION_HOLDS_QUESTION`]); a pane with none is not asked, since reset is doctor's
18	//!    remedy for it. For a switch, check that the server lists the target
19	//!    (`session-not-found`) and that no other record names it as its session of record
20	//!    ([`SESSION_OF_OTHER_PANE`]). That last check is a read, not the authority: the
21	//!    registry's compare-and-swap does not enforce it, so two concurrent switches of two panes
22	//!    to one session can both pass it.
23	//! 2. **Act**: for a reset, `create_session` on the pane's server; then `select_session` in
24	//!    the pane's TUI.
25	//! 3. **Observe**: `shown_session`. A TUI that does not show the target is the closed failure
26	//!    `unavailable` (exit 1; an open code would be a refusal), and nothing is recorded.
27	//! 4. **Record**: the record read in step 1 with exactly four fields set (the target as
28	//!    `session_of_record` and `last_observed.shown`, the request's clock as
29	//!    `last_observed.at`, and `harness.health` healthy, which step 1 observed), in one
30	//!    compare-and-swap at the generation step 1 read, never retried. `last_observed.driven`
31	//!    is left as stored (no port observes DRIVEN before #649), and every other field is the
32	//!    read record's, so a field this file does not know is kept.
33	//! 5. **Prompt**, only when the request carries a first message (`pane reset --first`):
34	//!    `send_prompt` queues it to the target, now the session of record, through the harness
35	//!    API.
36	//!
37	//! No step calls Herdr or the host, so nothing is ever typed into a TUI (I4), and no step
38	//! serves, attaches, aborts or deletes anything. A failure once `select_session` has been
39	//! called is not compensated: the record still names the previous session, and
40	//! [`SwitchFailure::message`] ends with the reconcile step (the pane doctor command line for
41	//! the pane, with `--fix`), which selects that session again. A failure before that, a
42	//! reset's `create_session` included, moved neither the TUI nor the record, and its message
43	//! has no step. A session a reset created and did not record stays on the server, and the
44	//! message names it. A failed prompt comes after the record: the TUI and the record agree on
45	//! the target, so its message has no step, and says that the session was created, shown and
46	//! recorded but the first message did not land. After a successful reset the previous
47	//! session stays on the server too: no port deletes a session, so doctor reports it as a
48	//! stray.
49	//!
50	//! A run makes at most eight port calls, each bounded by I5, so it needs no budget of its own.
~~~~

**E31: `crates/holler-pane/src/tx_switch.rs:184-230`** — The engine: plan, act, observe, record, then `harness.send_prompt`.

Command: `sed -n '184,230p' crates/holler-pane/src/tx_switch.rs | awk -v s=184 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
184	pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
185	    let record = plan(ports, request)?;
186	    let (target, created) = match &request.target {
187	        Target::Existing(id) => (id.clone(), None),
188	        Target::Fresh => {
189	            let id = ports.harness.create_session(record.harness.port)?;
190	            (id.clone(), Some(id))
191	        }
192	    };
193	    // Nothing above has moved the TUI or the record (a reset's new session is neither shown
194	    // nor recorded yet), so each failure so far converts with `acted: false`. From the
195	    // `select_session` call below on, every failure is `acted`, the call's own included:
196	    // the engine cannot tell how far a failed call got, so the TUI may have moved.
197	    let acted = |error| SwitchFailure {
198	        error,
199	        acted: true,
200	        created: created.clone(),
201	        unprompted: None,
202	    };
203	    ports
204	        .harness
205	        .select_session(&record.herdr.pane_id, &target)
206	        .map_err(acted)?;
207	    observe(ports, &record, &target).map_err(acted)?;
208	    let next = recorded(&record, &target, request.now_ms);
209	    let pane = ports
210	        .pane_store
211	        .cas_put(&next, record.generation)
212	        .map_err(acted)?;
213	    // The prompt goes only to the session just recorded (I2), so a failure here leaves the
214	    // TUI and the record in agreement: no reconcile step, nothing unrecorded.
215	    if let Some(text) = &request.first {
216	        ports
217	            .harness
218	            .send_prompt(record.harness.port, &target, text)
219	            .map_err(|error| SwitchFailure {
220	                error,
221	                acted: false,
222	                created: None,
223	                unprompted: Some(target.clone()),
224	            })?;
225	    }
226	    Ok(Switched {
227	        pane,
228	        previous: record.session_of_record,
229	    })
230	}
~~~~

**E32: `crates/holler-pane/src/tx_switch.rs:389-399`** — `recorded`: four fields set; `last_observed.driven` is the read record's.

Command: `sed -n '389,399p' crates/holler-pane/src/tx_switch.rs | awk -v s=389 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
389	/// The record a run writes: `record` with `target` as its session of record and as what
390	/// its TUI was seen showing at `now_ms`, and its server healthy. Every other field, and
391	/// `last_observed.driven`, is `record`'s.
392	fn recorded(record: &Pane, target: &str, now_ms: i64) -> Pane {
393	    let mut next = record.clone();
394	    next.session_of_record = Some(target.to_owned());
395	    next.last_observed.shown = Some(target.to_owned());
396	    next.last_observed.at = now_ms;
397	    next.harness.health = Health::Healthy;
398	    next
399	}
~~~~

**E33: `crates/holler-pane/src/pane.rs:188-199`** — `LastObserved { shown, driven, at }`.

Command: `sed -n '188,199p' crates/holler-pane/src/pane.rs | awk -v s=188 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
188	/// What reconcile last observed about which session the pane shows and which the hub
189	/// drives. Written by reconcile, never inferred.
190	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
191	#[serde(deny_unknown_fields)]
192	pub struct LastObserved {
193	    #[serde(default, skip_serializing_if = "Option::is_none")]
194	    pub shown: Option<String>,
195	    #[serde(default, skip_serializing_if = "Option::is_none")]
196	    pub driven: Option<String>,
197	    /// When it was observed (milliseconds since the Unix epoch).
198	    pub at: i64,
199	}
~~~~

**E34: `crates/holler-pane/src/reconcile.rs:26-40`** — reconcile's module doc: "`last_observed.driven` is left as stored. No port observes DRIVEN before #649".

Command: `sed -n '26,40p' crates/holler-pane/src/reconcile.rs | awk -v s=26 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
26	//! **What the record write means.** Other verbs gate on these fields (#645, #646, #648):
27	//!
28	//! - `harness.health` is written from this pass's health check: `healthy`,
29	//!   `{"unhealthy": "server-wedged"}` or `{"unhealthy": "server-down"}`. A failed check
30	//!   leaves it as stored.
31	//! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
32	//!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
33	//! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
34	//!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
35	//!   the session the hub drives.
36	//! - A record is written only when one of those values changed, or on its first
37	//!   observation (`at == 0`): the hub's rule for a periodic writer (`panes/mod.rs`). So
38	//!   `at` is when the current values were first observed, not a freshness stamp. A write
39	//!   that loses a race with another writer is reported and never retried: that writer's
40	//!   record wins.
~~~~

**E35: `crates/holler-pane/src/reconcile.rs:176-186`** — `shown_differs`, the one comparison of SHOWN with the session of record.

Command: `sed -n '176,186p' crates/holler-pane/src/reconcile.rs | awk -v s=176 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
176	/// Whether a pane whose session of record is `session_of_record` shows something else: the
177	/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
178	/// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
179	/// no mismatch (it is `no-session-of-record`).
180	///
181	/// The one form of this comparison: a pass calls it with what the TUI shows now, and a
182	/// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
183	/// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
184	pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
185	    session_of_record.is_some_and(|record| shown != Some(record))
186	}
~~~~

**E36: `crates/holler-pane/src/reconcile/observe.rs:220-266`** — `check_screen`: `shown-driven-mismatch` is SHOWN against the session of record today.

Command: `sed -n '220,266p' crates/holler-pane/src/reconcile/observe.rs | awk -v s=220 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
220	/// The TUI's rules: `tui-foreign-session` when it shows a session its pane's own healthy
221	/// server does not have, or else `shown-driven-mismatch` when it shows another than the
222	/// session of record, repaired by [`repair`] when the record decides it. Returns the screen
223	/// at the end of the chain: after an attempted fix, what the TUI showed then.
224	fn check_screen(
225	    ports: Ports<'_>,
226	    request: &ReconcileRequest<'_>,
227	    pane: &Pane,
228	    server: &Server,
229	    screen: Screen,
230	    findings: &mut Vec<Finding>,
231	) -> Screen {
232	    let Screen::Seen(shown) = &screen else {
233	        return screen;
234	    };
235	    let shown = shown.as_deref();
236	    if let (Some(session), Some(ids)) = (shown, server.sessions()) {
237	        if !ids.iter().any(|id| id == session) {
238	            findings.push(foreign_session(pane, session));
239	            return screen;
240	        }
241	    }
242	    let Some(record) = pane.session_of_record.as_deref() else {
243	        return screen;
244	    };
245	    if !shown_differs(Some(record), shown) {
246	        return screen;
247	    }
248	    let repair = repair(ports, request, pane, server, record);
249	    let tense = if repair.state == FixState::Fixed {
250	        "showed"
251	    } else {
252	        "shows"
253	    };
254	    let message = format!(
255	        "the TUI {tense} {}, not the session of record {}",
256	        screen_text(shown),
257	        quoted(record)
258	    );
259	    let finding = Finding::about(FindingKind::ShownDrivenMismatch, pane, message);
260	    findings.push(
261	        finding
262	            .with_session(shown)
263	            .with_fix(repair.state, repair.error),
264	    );
265	    repair.screen.unwrap_or(screen)
266	}
~~~~

**E37: `crates/holler-pane/src/findings.rs:68-70`** — The finding kind's doc.

Command: `sed -n '68,70p' crates/holler-pane/src/findings.rs | awk -v s=68 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
68	    /// The TUI shows another session than the session of record, or its home screen. The
69	    /// one kind `--fix` repairs: it selects the session of record.
70	    ShownDrivenMismatch,
~~~~

**E38: `crates/holler-pane/src/findings.rs:141-147`** — The mismatch's remedy table.

Command: `sed -n '141,147p' crates/holler-pane/src/findings.rs | awk -v s=141 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
141	            FindingKind::ShownDrivenMismatch => match fix {
142	                FixState::Fixable | FixState::Skipped => {
143	                    pane.map(|pane| doctor_command(Some(pane), true))
144	                }
145	                FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
146	                FixState::Fixed => None,
147	            },
~~~~

**E39: `crates/holler-proto/src/vocab.rs:200-218`** — The name grammar: lower-case letters, digits and `-`, so no id with `_` is a name.

Command: `sed -n '200,218p' crates/holler-proto/src/vocab.rs | awk -v s=200 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
200	/// The per-segment checks shared by all three name kinds: length limit,
201	/// word-char first/last, and no disallowed characters.
202	fn check_segment(seg: &[u8]) -> Result<(), NameError> {
203	    if seg.is_empty() {
204	        return Err(NameError::Empty);
205	    }
206	    if seg.len() > MAX_SEGMENT_LEN {
207	        return Err(NameError::TooLong);
208	    }
209	    if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
210	        return Err(NameError::BadCharacter);
211	    }
212	    for &c in seg {
213	        if !(is_word(c) || c == b'-') {
214	            return Err(NameError::BadCharacter);
215	        }
216	    }
217	    Ok(())
218	}
~~~~

**E40: `crates/holler-proto/src/docs.rs:517-532`** — `SessionAd`, the presence row, with `harness_session_id` (docs.rs is 899 lines: it takes no growth).

Command: `sed -n '517,532p' crates/holler-proto/src/docs.rs | awk -v s=517 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
517	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
518	#[serde(deny_unknown_fields)]
519	pub struct SessionAd {
520	    /// The routable name (`<label>/<session>`).
521	    pub name: String,
522	    /// The harness hosting this session.
523	    pub harness: String,
524	    /// The A2A **session** state — one of `idle`, `working`, `input-required`
525	    /// (`idle` is Holler's addition, ADR 0005; it has no A2A task-state
526	    /// constant).
527	    pub state: SessionState,
528	    /// How the session was established: `spawn` or `attach`.
529	    pub mode: Mode,
530	    /// The existing harness session being attached to (attach mode only).
531	    #[serde(skip_serializing_if = "Option::is_none")]
532	    pub harness_session_id: Option<String>,
~~~~

**E41: `crates/holler-proto/src/docs.rs:212-224`** — `HelloSession`, the hello row, with `harness_session_id`.

Command: `sed -n '212,224p' crates/holler-proto/src/docs.rs | awk -v s=212 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
212	/// One session advertised in a **body**'s hello.
213	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
214	pub struct HelloSession {
215	    /// The session name (`<session>` or `<label>/<session>`).
216	    pub name: String,
217	    /// The harness that hosts this session.
218	    pub harness: String,
219	    /// How the session was established: `spawn` or `attach`.
220	    pub mode: Mode,
221	    /// The existing harness session being attached to (attach mode only).
222	    #[serde(skip_serializing_if = "Option::is_none")]
223	    pub harness_session_id: Option<String>,
224	}
~~~~

**E42: `crates/holler-proto/src/methods.rs:57-80`** — The closed catalog: 22 rows, `session/answer` the last hub-to-body request.

Command: `sed -n '57,80p' crates/holler-proto/src/methods.rs | awk -v s=57 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
57	/// The complete, closed v2 method catalog (22 rows).
58	#[rustfmt::skip]
59	pub const CATALOG: &[Method] = &[
60	    // name                 kind          direction   (docs §4)
61	    Method { name: "circuit/join",        kind: MethodKind::Request,      dir: Direction::BodyToHub },
62	    Method { name: "circuit/authenticate",kind: MethodKind::Request,      dir: Direction::BodyToHub },
63	    // `circuit/prove` (issue #323): the second frame of the authenticate
64	    // challenge-response — the body signs the hub's nonce and answers here.
65	    Method { name: "circuit/prove",       kind: MethodKind::Request,      dir: Direction::BodyToHub },
66	    Method { name: "circuit/hello",       kind: MethodKind::Request,      dir: Direction::Both      },
67	    Method { name: "circuit/ping",        kind: MethodKind::Request,      dir: Direction::Both      },
68	    Method { name: "circuit/superseded",  kind: MethodKind::Notification, dir: Direction::HubToBody },
69	    Method { name: "query/status",        kind: MethodKind::Request,      dir: Direction::Both      },
70	    Method { name: "query/caps",          kind: MethodKind::Request,      dir: Direction::Both      },
71	    Method { name: "query/support",       kind: MethodKind::Request,      dir: Direction::Both      },
72	    Method { name: "query/protocol",      kind: MethodKind::Request,      dir: Direction::Both      },
73	    Method { name: "session/presence",    kind: MethodKind::Notification, dir: Direction::BodyToHub },
74	    Method { name: "session/prompt",      kind: MethodKind::Request,      dir: Direction::HubToBody },
75	    Method { name: "session/update",      kind: MethodKind::Notification, dir: Direction::BodyToHub },
76	    Method { name: "session/cancel",      kind: MethodKind::Request,      dir: Direction::HubToBody },
77	    // `session/answer` (issue #151): resolve a held `input-required`
78	    // permission/elicitation — same direction/kind as `session/cancel`, the
79	    // request it's modeled on.
80	    Method { name: "session/answer",      kind: MethodKind::Request,      dir: Direction::HubToBody },
~~~~

**E43: `crates/holler-proto/src/methods.rs:152-156`** — The test that pins 22.

Command: `sed -n '152,156p' crates/holler-proto/src/methods.rs | awk -v s=152 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
152	    #[test]
153	    fn pane_methods_not_in_wire_catalog() {
154	        assert_eq!(CATALOG.len(), 22, "the v2 wire catalog is closed");
155	        assert!(find("pane/list").is_none());
156	        assert!(find("profile/list").is_none());
~~~~

**E44: `crates/holler-body/src/config.rs:1-21`** — The body config grammar: an attach session names `endpoint` and `session_id`.

Command: `sed -n '1,21p' crates/holler-body/src/config.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The body's session config file (issue #187): named sessions, each a
2	//! harness + transport (`spawn` a fresh process or `attach` to one already
3	//! running), validated fail-closed at parse time.
4	//!
5	//! Grammar (the issue's own spec):
6	//!
7	//! ```text
8	//! [[session]]
9	//! name = "alpha"                       # SessionName grammar (ADR 0005); unique in file
10	//! harness = "opencode"                 # any holler_proto::HARNESS_IDS id; unknown ids are
11	//!                                       # legal on the wire but `support` says ok:false
12	//! mode = "spawn"                       # "spawn" (default) | "attach"
13	//! command = ["opencode", "acp"]        # spawn only; argv, no shell
14	//! cwd = "/path"                        # optional, spawn only
15	//! env = { KEY = "value" }              # optional, spawn only
16	//! interrupt = "acp"                    # optional: "acp" (default) | "http"; http needs endpoint
17	//! endpoint = "http://127.0.0.1:4096"   # attach: required; spawn: optional (HTTP interrupt fallback)
18	//! session_id = "ses_..."               # attach: required
19	//! auth_method = "api-key"              # optional, spawn only: ACP auth method id sent in `authenticate`
20	//!                                       # when session/new answers auth-required (ACP v1 only, #439)
21	//! ```
~~~~

**E45: `crates/holler-body/src/config.rs:95-118`** — `SessionConfig`.

Command: `sed -n '95,118p' crates/holler-body/src/config.rs | awk -v s=95 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
95	/// One validated `[[session]]` entry.
96	#[derive(Debug, Clone, PartialEq, Eq)]
97	pub struct SessionConfig {
98	    /// The session's name (unique within the file; [`SessionName`] grammar).
99	    pub name: SessionName,
100	    /// The harness id (`holler_proto::HARNESS_IDS`; unknown ids are legal on
101	    /// the wire, so this is not validated against the vocabulary here).
102	    pub harness: String,
103	    /// `spawn` or `attach`.
104	    pub mode: SessionMode,
105	    /// The spawn argv (`command[0]` resolved on `PATH` or as a path). `None`
106	    /// for `attach` (or dropped with a warning if the file set one anyway).
107	    pub command: Option<Vec<String>>,
108	    /// The spawned process's working directory (spawn only).
109	    pub cwd: Option<String>,
110	    /// Extra environment variables for the spawned process (spawn only).
111	    pub env: Option<BTreeMap<String, String>>,
112	    /// How to interrupt an in-flight turn.
113	    pub interrupt: Interrupt,
114	    /// The attach endpoint (required for `attach`; an optional HTTP-interrupt
115	    /// fallback for `spawn`).
116	    pub endpoint: Option<String>,
117	    /// The existing harness session id being attached to (attach only).
118	    pub session_id: Option<String>,
~~~~

**E46: `crates/holler-body/src/session_manager.rs:338-377`** — `presence_doc`: `harness_session_id: managed.config.session_id.clone()`.

Command: `sed -n '338,377p' crates/holler-body/src/session_manager.rs | awk -v s=338 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
338	    /// Build this body's `session/presence` params from every session's live
339	    /// state — the dynamic replacement for
340	    /// [`crate::registry::SessionRegistry::presence_doc`]'s always-`idle`
341	    /// snapshot (that one has no driver behind it yet; this one does).
342	    ///
343	    /// An attach-mode session that has never yet completed its initial
344	    /// [`crate::http_attach_driver::HttpAttachDriver::attach`] (its very
345	    /// first attempt failed, and `task`'s own 30s retry loop has not
346	    /// succeeded yet) is **omitted entirely** from this document (issue
347	    /// #195's own spec: "presence omits the session until it attaches on a
348	    /// 30s retry") — never advertised as a fabricated `idle` row for a
349	    /// harness session this body cannot actually reach yet.
350	    pub fn presence_doc(&self, hostname: String) -> Presence {
351	        let sessions = self
352	            .sessions
353	            .iter()
354	            .filter_map(|(name, managed)| {
355	                let p = managed.presence.lock().unwrap_or_else(PoisonError::into_inner);
356	                if p.omitted_pending_attach {
357	                    return None;
358	                }
359	                Some(SessionAd {
360	                    name: name.as_str().to_string(),
361	                    harness: managed.config.harness.clone(),
362	                    state: p.state,
363	                    mode: match managed.config.mode {
364	                        SessionMode::Spawn => Mode::Spawn,
365	                        SessionMode::Attach => Mode::Attach,
366	                    },
367	                    harness_session_id: managed.config.session_id.clone(),
368	                    turn_started_at: p.turn_started_at.clone(),
369	                    last_update_at: p.last_update_at.clone(),
370	                    pending: p.pending.clone(),
371	                    turn_id: p.turn_id.clone(),
372	                    last_turn: p.last_turn.clone(),
373	                })
374	            })
375	            .collect();
376	        Presence { hostname, sessions }
377	    }
~~~~

**E47: `crates/holler-body/src/session_manager.rs:95-134`** — `SessionCommand`: the per-session task's mailbox.

Command: `sed -n '95,134p' crates/holler-body/src/session_manager.rs | awk -v s=95 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
95	pub enum SessionCommand {
96	    /// A plain or `--queue` prompt. `id` is the JSON-RPC id of the
97	    /// `session/prompt` call this belongs to (surfaced back on
98	    /// `presence_doc()`'s `turn_id`/`last_turn` once it resolves).
99	    Prompt {
100	        id: String,
101	        text: String,
102	        queue: bool,
103	        reply_tx: oneshot::Sender<PromptOutcome>,
104	        /// Mid-turn text chunks, forwarded as the driver emits them (issue
105	        /// #190's coalescer consumes this to build `session/update`
106	        /// notifications). `None` for a caller with no streaming consumer
107	        /// (e.g. the existing #189 unit tests) — chunks are then simply
108	        /// dropped, exactly like before this story.
109	        updates: Option<mpsc::UnboundedSender<String>>,
110	    },
111	    /// Cancel the in-flight turn only; queued prompts are untouched.
112	    Cancel { reply_tx: oneshot::Sender<Result<(), String>> },
113	    /// Resolve a held permission/elicitation. Replies `Ok(())` only once the
114	    /// driver reports `Working` again (or the turn ends outright).
115	    Answer {
116	        choice: String,
117	        reply_tx: oneshot::Sender<Result<(), String>>,
118	    },
119	    /// Cancel the in-flight turn (if any), then run `text` **ahead of** the
120	    /// queue — what `interrupt SESSION TEXT` maps to.
121	    Replace {
122	        text: String,
123	        reply_tx: oneshot::Sender<PromptOutcome>,
124	        /// Mid-turn text chunks for this redirect turn (issue #191: `interrupt
125	        /// SESSION TEXT` streams its reply exactly like `say`). `None` for a
126	        /// caller with no streaming consumer (the pre-#191 unit tests).
127	        updates: Option<mpsc::UnboundedSender<String>>,
128	    },
129	    /// End this session's task: gracefully shut down its driver (if any) and
130	    /// return. Queued prompts are dropped without a reply (their `reply_tx`
131	    /// is simply never fired) — a shutting-down body has nowhere to deliver
132	    /// that answer anyway.
133	    Shutdown,
134	}
~~~~

**E48: `crates/holler-body/src/connection.rs:710-734`** — The body's inbound dispatch: three `session/*` methods reach `dispatch_session_request` (connection.rs is 873 lines).

Command: `sed -n '710,734p' crates/holler-body/src/connection.rs | awk -v s=710 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
710	            Envelope::Request { id, method, params }
711	                if matches!(method.as_str(), "session/prompt" | "session/cancel" | "session/answer") =>
712	            {
713	                // Issue #191's priority path: `session/cancel`'s own
714	                // response frame goes out via `priority_tx`, not the normal
715	                // `outbound_tx` — see `priority_tx`'s own doc comment above.
716	                // `dispatch_session_request` picks between the two channels
717	                // itself, by method name.
718	                session_dispatch::dispatch_session_request(
719	                    self.sink,
720	                    self.session_manager,
721	                    &self.outbound_tx,
722	                    &self.priority_tx,
723	                    id,
724	                    method,
725	                    params,
726	                )
727	                .await
728	            }
729	            Envelope::Request { id, .. } => {
730	                let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
731	                let err = holler_proto::WireError::new(Code::MethodNotFound, "not implemented on this body yet", None);
732	                let _ = send(self.sink, &Envelope::error_frame(&cid, &err)).await;
733	                FrameOutcome::Continue
734	            }
~~~~

**E49: `crates/holler-body/src/connection/session_dispatch.rs:28-66`** — `dispatch_session_request`.

Command: `sed -n '28,66p' crates/holler-body/src/connection/session_dispatch.rs | awk -v s=28 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
28	/// Dispatch one inbound `session/prompt`/`session/cancel`/`session/answer`
29	/// request: parse its typed params and spawn the matching dispatch task, or
30	/// answer `-32602 invalid_params`. The caller (`LiveConnection::handle_text`)
31	/// only reaches this for one of these three method names — this single
32	/// three-way `match` replaces what would otherwise be three near-identical
33	/// arms inlined there.
34	pub(super) async fn dispatch_session_request<Snk>(
35	    sink: &mut Snk,
36	    session_manager: &Arc<SessionManager>,
37	    outbound_tx: &mpsc::UnboundedSender<Message>,
38	    priority_tx: &mpsc::UnboundedSender<Message>,
39	    id: String,
40	    method: String,
41	    params: Option<serde_json::Value>,
42	) -> FrameOutcome
43	where
44	    Snk: Sink<Message, Error = WsError> + Unpin,
45	{
46	    let env = Envelope::Request { id: id.clone(), method: method.clone(), params };
47	    match method.as_str() {
48	        "session/prompt" => match holler_proto::typed_params::<Prompt>(&env) {
49	            Ok(p) => spawn_prompt_dispatch(session_manager, outbound_tx, id, p),
50	            Err(e) => send_invalid_params(sink, &id, &e).await,
51	        },
52	        // Issue #191: dispatched onto `priority_tx`, not `outbound_tx` — see
53	        // the module doc.
54	        "session/cancel" => match holler_proto::typed_params::<Cancel>(&env) {
55	            Ok(p) => spawn_cancel_dispatch(session_manager, priority_tx, id, p),
56	            Err(e) => send_invalid_params(sink, &id, &e).await,
57	        },
58	        // The caller's own guard (`handle_text`) only reaches here for one
59	        // of these three method names.
60	        _ => match holler_proto::typed_params::<Answer>(&env) {
61	            Ok(p) => spawn_answer_dispatch(session_manager, outbound_tx, id, p),
62	            Err(e) => send_invalid_params(sink, &id, &e).await,
63	        },
64	    }
65	    FrameOutcome::Continue
66	}
~~~~

**E50: `crates/holler-body/src/http_attach_driver.rs:29-37`** — The attach driver's existence check: `GET {endpoint}/session/{session_id}`, fail closed, never `session/new`.

Command: `sed -n '29,37p' crates/holler-body/src/http_attach_driver.rs | awk -v s=29 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
29	//! - **Existence check**: `GET {endpoint}/session/{session_id}` — confirmed
30	//!   `200` for a real session, `404` for a bogus id. `GET
31	//!   {endpoint}/api/session/{session_id}` also confirmed `200` for the same
32	//!   real session — **both forms genuinely work** as an existence probe, so
33	//!   [`HttpAttachDriver::attach`] tries the bare form first and falls back to
34	//!   `/api` on a `404` of the first, same as this issue's own spec. Any
35	//!   non-2xx from *both* attempts is [`crate::acp_driver::DriverError::NotFound`]
36	//!   — fail closed, never fall through to `POST /session` (verified this is
37	//!   never called by [`HttpAttachDriver::attach`]).
~~~~

**E51: `docs/adr/ADR-0014.md:11-19`** — ADR 0014 points 2 to 6: `session_id` is config, a harness id is "a body-side locator", presence carries `harness_session_id`.

Command: `sed -n '11,19p' docs/adr/ADR-0014.md | awk -v s=11 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
11	A body config row may **attach** to a harness session that is already running under something else — a terminal-multiplexer pane, an operator's own `opencode serve` — instead of **spawning** and owning it. The two legacy repos wrote this as a matched pair of ADRs (one per side of the old split); this repo folds them into one, because in the single-binary architecture there is only one process making the choice.
12	
13	1. **Spawn remains the default.** A config row with `mode = "spawn"` or no `mode` at all behaves exactly as [ADR 0012](ADR-0012.md) describes: the body execs `command` as an ACP v2 subprocess it fully owns, and `detach` or process exit tears that child down. Nothing about spawn changes.
14	2. **Attach is a first-class second transport, not a fallback.** A config row with `mode = "attach"` names an `endpoint` and a `session_id`; the body drives that session over its existing HTTP/ACP control surface and is **not its parent**. `session/new` is never called in attach mode — if `session_id` is missing, or the endpoint 404s on a lookup for it, the body fails closed with a precise error rather than creating a consolation session.
15	3. **Teardown is mode-aware, and attach's teardown does the least, not the most.** `detach`, a dropped hub connection, or this process exiting must never kill or replace an attached session — only the local control channel (the body's own connection into the endpoint) closes. This is the entire point of the mode: two things cannot both parent one process, so whichever one the operator is already watching (a Herdr pane, a terminal) must be the side that owns it, and Holler's body must be able to leave without taking it down.
16	4. **Holler still does not become a terminal multiplexer.** Attach never types into a PTY and never streams pane bytes on the Holler wire (`herdr pane send-keys` or equivalent stuffing is explicitly out of scope). Prompt and interrupt go through the same ACP/HTTP control surface an attached endpoint already exposes — the same surface a spawn-mode body uses as its interrupt fallback — never through the terminal the human is watching.
17	5. **Addressing is unaffected — this is the point of [ADR 0005](ADR-0005.md)/[ADR 0011](ADR-0011.md).** The Holler session name (e.g. `io/alpha`) is the circuit address regardless of mode; a harness-side session id (`ses_…`) is a **body-side locator**, carried on presence so the roster can distinguish attach from spawn, but it never changes who `say io/alpha` talks to.
18	6. **Presence carries `mode` and `harness_session_id` as optional fields**, not a protocol version bump — ADR 0004's envelope already ignores unknown keys, and these two are additive to the presence document the roster (ADR 0010) already consumes.
19	
~~~~

**E52: `docs/adr/ADR-0005.md:28-30`** — ADR 0005 section 3: the name is stable, the locator may change.

Command: `sed -n '28,30p' docs/adr/ADR-0005.md | awk -v s=28 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
28	### 3. Name vs locator
29	
30	The **roster row is keyed by routable name**. The `client_id`, peer address, and connection handle are **locator fields on that row**, and any of them **may change across a reconnect without changing the name**. Consequently every `say` / `interrupt` / `query` `TARGET` resolves a **name → the current locator at call time**; a command is never issued to a locator it was handed earlier. A body that drops and re-joins keeps its `io/alpha` name and simply gets a new row locator under it.
~~~~

**E53: `docs/adr/ADR-0021.md:1-8`** — The ADR header: Status, Depends on, Amends.

Command: `sed -n '1,8p' docs/adr/ADR-0021.md | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	# ADR 0021 — The pane control plane: one registry on the hub, verbs on the CLI, Herdr, tmux and OpenCode behind ports
2	
3	**Status:** accepted (the items marked **PROPOSED** below are not; they wait for the operator, see "Decisions taken")
4	**Issue:** [#22](https://github.com/Performant-Labs/holler/issues/22) (claims this reserved slot for [#634](https://github.com/Performant-Labs/holler/issues/634), part of epic [#633](https://github.com/Performant-Labs/holler/issues/633))
5	**Date:** 2026-10-09
6	**Depends on:** [ADR 0003](ADR-0003.md) (CLI surface, exit codes), [ADR 0004](ADR-0004.md) (JSON-RPC 2.0), [ADR 0005](ADR-0005.md) (names), [ADR 0012](ADR-0012.md) (config, not plugins), [ADR 0014](ADR-0014.md) (attach mode)
7	**Ratifies:** the contract section of epic #633 (decisions 1-7, the confirmed decisions C1-C5, the approved features B1, B2, B4, B5, invariants I1-I8) and the nine rulings of its "Skeleton split" section, as merged in [#637](https://github.com/Performant-Labs/holler/issues/637) (the `holler-pane` crate) and [#669](https://github.com/Performant-Labs/holler/issues/669) (the hub plumbing)
8	**Amends:** nothing directly. [ADR 0003](ADR-0003.md)'s `--json` line is amended by [#670](https://github.com/Performant-Labs/holler/issues/670) for the envelope and the NDJSON of `pane watch`, citing this ADR.
~~~~

**E54: `docs/adr/ADR-0021.md:92-101`** — Section 2, the paragraph that ends "`Ports` bundles one `&dyn` of each".

Command: `sed -n '92,101p' docs/adr/ADR-0021.md | awk -v s=92 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
92	### 2. Ratified: the ports
93	
94	All ports are traits in `holler-pane` (`src/ports.rs`, `src/profile.rs`), **synchronous, blocking and `Send + Sync`**: async
95	code calls them from `spawn_blocking` or a thread, and every method returns within I5's bound (default 10 s) or with
96	`timeout`. Two methods are bounded otherwise, which also qualifies section 12's per-call bound (#663):
97	`ProfileScope::edit_spec` returns within the sum of its port calls (at most four) plus the verb's own act, which it can
98	neither bound nor cancel, and `Prober::run_probe` returns at its deadline plus at most 1 s to kill and reap the probe; a
99	follow-up amends the frozen trait docs to match. Through the wiring a third case can run past its bound: an OpenCode TUI
100	method whose pane the `tui_session` lookup resolves through the registry or Herdr (section 5, "Wiring as built
101	(#649)"). `Ports` bundles one `&dyn` of each, and a verb holds it.
~~~~

**E55: `docs/adr/ADR-0021.md:301-310`** — The invariants table: I2 and I5.

Command: `sed -n '301,310p' docs/adr/ADR-0021.md | awk -v s=301 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
301	### 4. Ratified: the invariants, each with a test idea
302	
303	| | Invariant | One-line test idea |
304	|---|---|---|
305	| I1 | Only pane verbs change a pane or its registration. | Every adapter call in a scenario is made from a verb's code path: the fakes record their caller, and the hub's `pane/*` handlers hold no adapter (no adapter crate in `holler-hub`'s manifest). |
306	| I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
307	| I3 | Every verb is plan, act, observe, record; a mismatch fails loudly and records nothing. | Make the fake TUI show a different session after `act`: the verb exits 1 with a code, and the stored record and its generation are unchanged. |
308	| I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the switch and reset tests fail if any keystroke was sent (#645). |
309	| I5 | A verb returns in bounded time (default 10 s); long work returns an operation id. | A wedged fake port makes the verb exit with `timeout` within the bound plus a margin; the operation-id half is not built (section 12, decided 2026-10-11). |
310	| I6 | State is observed from Herdr, tmux and the harness, never inferred from files. | Give the importer and reconcile a stale registration file that names session A while the fake harness shows B: no record gets A as observed state. |
~~~~

**E56: `docs/adr/ADR-0021.md:321-334`** — Section 5, "Wiring as built (#649)": the first bullets.

Command: `sed -n '321,334p' docs/adr/ADR-0021.md | awk -v s=321 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
321	**Wiring as built (#649).** `Wiring::connect` builds the ports of a run, once per run, and every pane and profile verb runs
322	over them, as do `say`, `interrupt` and `answer` with `--pane`, whose pane resolution calls `connect` (`say_cmd.rs`).
323	
324	- **`connect` needs the hub and nothing else.** It makes one `control/status` exchange (the control client's own 5 s), and
325	  any failure of it is `unavailable` naming the hub and its state dir, so with no hub every verb exits 1 having touched no
326	  adapter. It builds the two registries' clients, the Herdr adapter, the tmux host, the OpenCode harness, `SystemProber`, and
327	  `StoreScope` over the two clients as the `ProfileScope`. The Herdr adapter pings Herdr when it is built, so it is built on
328	  its first use and kept, a failed build included; building the host or the harness does no I/O. So `pane list`, `get` and
329	  `watch` and `profile list` and `show` run with no Herdr, tmux or OpenCode present.
330	- **The registries' clients** (`wiring/store.rs`) send each call as one control-socket request through the CLI's one dispatch
331	  point (`transport::call`), with the params of `holler_pane::reply` and a `PaneReply` back, within 10 s, or within the
332	  long-poll window plus 5 s for a `*/watch` request. A failed exchange of a read is `timeout` naming the store method when
333	  the hub took the request and did not answer within the bound, which is section 2's own rule, and `unavailable` naming the
334	  hub otherwise. A write whose reply is lost is `timeout`, since it may have landed (section 8, step 2). A JSON-RPC error is
~~~~

**E57: `docs/adr/ADR-0021.md:360-371`** — Section 5: the bullets "What the wiring makes live" and "Not done here" (the #828 and #654 sentences).

Command: `sed -n '360,371p' docs/adr/ADR-0021.md | awk -v s=360 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
360	- **No new bound.** Each adapter and each registry request keeps its own 10 s bound (I5), and each verb its budget; there is
361	  no flag, option or variable for a bound.
362	- **What the wiring makes live.** Every profile write through the scope is logged under the one actor `holler pane`: a
363	  `--profile` edit of `launch`, `relaunch` or `close`, and the creates and relaunches of `profile apply`, so the profile log
364	  cannot tell those verbs apart. A per-verb actor needs a seam other than the argument-less `connect` (#825). And since #642
365	  part 3 the first message of `pane reset --first` is a real prompt through `HarnessPort::send_prompt`, outside the hub's
366	  prompt path and so outside its park and hold gate (section 8, "The prompt and activity seam"): this wiring makes that path
367	  live, and closing it is #828's. Until #828 lands, the operator does not use `reset --first` on a parked or held pane.
368	- **Not done here.** No `HostPort` amendment and no credential handling: a provider credential reaches a harness server only
369	  through its tmux server's environment, and how production sets that is the cutover's (#654). The hub's DRIVEN session
370	  is not pointed at `session_of_record` (section 11, #654). `--herdr-session` is not defaulted from the configured session
371	  (section 8, "Launch and relaunch as built (#644)"). The pane's model, effort and environment still do not reach its
~~~~

**E58: `docs/adr/ADR-0021.md:393-399`** — Section 6: the pane and profile methods are control-socket methods, "the closed 22-row catalog".

Command: `sed -n '393,399p' docs/adr/ADR-0021.md | awk -v s=393 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
393	### 6. The hub control-socket methods and their replies
394	
395	**Decided (ruling 6): `pane/*` and `profile/*` are hub control-socket methods, not v2 wire methods.** They sit outside the
396	closed 22-row catalog (`holler_proto::methods::CATALOG`), are not reserved in [protocol v2](../protocol/v2.md) §4, and a body
397	connection that sends one still gets `method_not_found`. The names are:
398	
399	- `PANE_METHODS`: `pane/get`, `pane/list`, `pane/cas_put`, `pane/delete`, `pane/watch`.
~~~~

**E59: `docs/adr/ADR-0021.md:523-556`** — Section 8, "Switch and reset as built (#645)".

Command: `sed -n '523,556p' docs/adr/ADR-0021.md | awk -v s=523 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
523	**Switch and reset as built (#645).** One engine, `holler_pane::tx_switch`, runs both verbs: switch moves a pane to a
524	session its harness server already has, and reset to one the run creates. Its plan reads the record (through the profile's
525	scope with `--profile`) and refuses before anything is written or moved: the orchestrator's pane unless the operator passes
526	`--as-operator`, a statement of intent and not authentication (`orchestrator-pane`); a harness server that does not answer
527	its health check, observed live and never read from `harness.health` (`server-unhealthy`, naming the relaunch remedy); for
528	reset of a pane that has a session of record, a conversation that `HarnessPort::session_activity` reports running a turn
529	(`session-busy`) or holding a question (`session-holds-question`), while a pane with none is not asked, since reset is
530	doctor's remedy for it; and, for switch, a target the pane's own server does not list (`session-not-found`) or that is
531	another pane's session of record (`session-of-other-pane`). The act is `select_session` in the pane's TUI, after
532	`create_session` for reset, and the observation is `shown_session`. **A mismatch observed after the act is the closed
533	failure `unavailable`, exit 1:** I3 needs a failure, and an open code is a refusal (section 9). A mismatch records nothing.
534	Otherwise the record the plan read is written with exactly four fields set (`session_of_record` and `last_observed.shown` to
535	the target, `last_observed.at` to the run's clock, `harness.health` to healthy) in one compare-and-swap at the generation the
536	plan read, never retried; `last_observed.driven` is left as stored (section 11). Then, only for `reset --first TEXT`,
537	`HarnessPort::send_prompt` queues `TEXT` to the new session, now the session of record (I2), so the first message can land
538	nowhere else. A prompt that fails after the record is the port's own error (its class decides the exit), and its message
539	says the session was created, shown and recorded but the first message did not land; it has no reconcile step, because the
540	record is correct. No step calls Herdr or the host (I4), and a run makes at most eight port calls, each bounded by I5
541	(section 12). Neither verb checks the pane's park state; a refusal derived from pane state belongs to the prompt gate of
542	#646, which `reset --first` does not pass through (see the seam below). A failure after the act and before the record is
543	not compensated: the record still names the previous session, and the message ends with the reconcile step, the pane
544	doctor command line for the pane with `--fix`, which selects that session again. That step follows every such failure once
545	`select_session` has been called, a record conflict or a `timeout` included; a failure before then has moved neither the TUI
546	nor the record, and its message has no step.
547	Whether or not the verb ran with `--profile`, the step names the pane, unlike the two forms of step 6 that the generations
548	rule above gives. Those name no pane because a failed launch can leave a pane with no record, which a pane-scoped doctor
549	refuses. A switch or reset reads the pane's record before it acts, so a doctor run for that pane is not refused, and
550	`--fix` is the pane doctor's own remedy for a TUI that does not show the session of record, while a fix can repair it.
551	Neither verb edits P's specs, so no `profile show` is needed. A session that reset created and did not record stays on the
552	server, and the message names it. After a successful reset the previous session also stays on the server, and the pane
553	doctor reports it as a stray session: no port deletes a session. The session-of-other-pane check is a read in the verb, not
554	a rule of the registry, whose compare-and-swap enforces only the generation and the membership rule, so two concurrent
555	switches of two panes to one session can both pass it and both record that session.
556	
~~~~

**E60: `docs/adr/ADR-0021.md:557-583`** — Section 8, "The prompt and activity seam".

Command: `sed -n '557,583p' docs/adr/ADR-0021.md | awk -v s=557 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
557	**The prompt and activity seam (#645 part 2, the operator's carve-out).** `HarnessPort` grows two default-armed methods,
558	the first amendment since #637 froze it (the second is #802's, section 2): `send_prompt(port, session, text)` and
559	`session_activity(port, session)`, which
560	answers the closed `Activity` (`Idle`, `Busy`, `HoldingQuestion`; in `ports.rs`, never on the wire). A default keeps every
561	impl that does not override it as it was: `session_activity` answers `Idle`, so nothing is refused where no port reports
562	activity, and `send_prompt` refuses with the open code `prompt-unsupported` (exit 3). That code departs from the
563	`not-implemented` the skeleton answers for unlanded methods on purpose: by the time `reset --first` sends the prompt the
564	reset itself has succeeded, so "understood and declined" (a refusal) is the honest class, not a runtime failure. The OpenCode
565	adapter replaces both defaults (#642 part 3), and the test kit's fake records them (#684). Its `send_prompt` asks `GET
566	/session/:id` first, as its `abort` does (an unknown session is `session-not-found` and nothing is posted), then posts
567	`{"parts":[{"type":"text","text":TEXT}]}`, naming no agent, to the bare `POST /session/:id/prompt_async`, the route and body
568	`holler-body` sends; it is `Ok` exactly on that route's `204`, and its message for any other reply names the route, the port
569	and the status and quotes no part of the reply, which could echo the prompt. Its `session_activity` is `HoldingQuestion`
570	while `GET /question` or `GET /permission` lists an item of the session or of a direct child (subagent) session of it, the
571	scope `holler-body` reads: a pending permission request, the session's own or a direct child's, reads as `HoldingQuestion`,
572	since the turn waits on an answer either way. A list that cannot be read holds nothing (a turn that waits on an answer still
573	runs, so its status reads `Busy`). Otherwise it is `Busy` while `GET /session/status` has an entry for the session whose
574	`type` is not `idle` (`busy`, `retry`, or none), else `Idle`; only a server whose status route answers no JSON object is read
575	from its last assistant message instead (`Busy` while it has no `time.completed`). Its `abort` reads that route the same
576	way, so an abort of a session OpenCode is retrying is not settled until the session's entry is gone or `idle`. Two choices
577	differ from what this ADR had proposed for 645b, and #646 and #649 build on them rather than beside them: the first message
578	goes from the verb straight to `HarnessPort::send_prompt`, not through the hub's one prompt path (so the prompt gate of #646,
579	park state included, does not see it); and the activity read is of the pane's session of record, not of its DRIVEN session,
580	which is not kept in step with it until the cutover wires DRIVEN (#654, section 11). Since #642 part 3 the adapter sends
581	that first message for real, and the gate still does not see this path; the wiring (#649) makes it live, and whether it
582	moves to the hub's one prompt path is #828's (section 5, "Wiring as built (#649)"). The adapter's `send_prompt` stays pure transport: a
583	hold, a limit or a park belongs at the hub's choke point, not in an adapter.
~~~~

**E61: `docs/adr/ADR-0021.md:753-758`** — Section 9: the verb rows for switch, reset and the routed prompt verbs.

Command: `sed -n '753,758p' docs/adr/ADR-0021.md | awk -v s=753 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
753	| `pane switch`, `pane reset` | `pane-not-found`, open (#832) `pane-unmanaged`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645): `orchestrator-pane`, `server-unhealthy`, for `switch` `session-of-other-pane`, and for `reset` a session of record that is running a turn or holds a question (`session-busy`, `session-holds-question`) and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default) |
754	| `pane park`, `pane unpark` | `pane-not-found`, open (#832) `pane-unmanaged`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
755	| `pane close` | `pane-not-found`, open (#832) `pane-unmanaged`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-not-in-profile` (with `--profile`, P holds no spec for the pane) |
756	| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` and, with `--unmanaged-stalled-after SECONDS`, `unmanaged-stalled` (#832, section 14) |
757	| `pane import` | `usage` (no `--herdr-session`, a `--from` that is not a directory, an `--orchestrator` that names no pane directory), `herdr-version-unsupported` (from `herdr.version()`, which ends the run with no report), `profile-exists` (a real run with a pane to record while a profile `fleet` exists), the code of a failed write whose undo completed (`generation-conflict`, `timeout`, ...), `profile-conflict` (an undo step failed; the message names the commands that reconcile); open (#650): `import-blocked` (a real run while a pane has a problem; nothing is written). `command-not-argv`, `session-not-found` and every disagreement are findings of the report, not errors (#650) |
758	| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found` (no session of record); open (#646), as the record says: `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; all exit 3, the code in the message. The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"` |
~~~~

**E62: `docs/adr/ADR-0021.md:810-816`** — Section 9: the closed and open code rows.

Command: `sed -n '810,816p' docs/adr/ADR-0021.md | awk -v s=810 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
810	  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply), or the live state a verb needs is not there after its act (#644). |
811	  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
812	  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
813	  | `profile-drift` | Failure (1) | A finding kind, not an error a port returns; if one surfaces, live state disagrees with its spec. |
814	  | any other well-formed code (open, raised as `Refused`) | Refusal (3) | `Refused` is the channel for the codes a verb or adapter owns; the open codes so far (#640's `grid-unreachable`, #645's `session-busy`, `session-holds-question` and `prompt-unsupported`, #646's `pane-parked`, `pane-unhealthy` and `pane-shown-driven-mismatch`, all merged, #802's `agent-unsupported`, #642's `agent-unusable`, the OpenCode adapter's, and #650's `import-blocked`) are all refusals. |
815	  | a malformed code | Failure (1) | It cannot be trusted: the same rule turns a garbled wire reply into `unavailable`. |
816	
~~~~

**E63: `docs/adr/ADR-0021.md:866-888`** — Section 11, in full.

Command: `sed -n '866,888p' docs/adr/ADR-0021.md | awk -v s=866 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
866	### 11. How attach mode (ADR 0014) relates
867	
868	[ADR 0014](ADR-0014.md) stays in force, unchanged. What changes is **who chooses the session**. For a pane in the registry,
869	Holler creates the session of record: the pane verbs start the harness server (the argv `HarnessPort::serve_command`
870	builds, which `HostPort::run` starts in the pane's tmux session, #802), create the session over its API (`create_session`)
871	and attach the TUI to it (`attach_tui`). The body that drives that session is still not its
872	parent, so ADR 0014's teardown rule (point 3) and its rule against typing into a PTY (point 4, which I4 restates for pane
873	verbs) apply as they are. The difference is that the session id comes from `session_of_record`, which a verb writes after it
874	observes the session. It is no longer pasted by an operator or guessed by a repair. That answers ADR 0014's open "automatic
875	discovery of attachable sessions (e.g. from a Herdr pane)" for registered panes: there is no discovery; the registry names
876	the session. Attach mode with an operator-written `endpoint` and `session_id` stays for sessions Holler does not own (a
877	session with no pane record).
878	
879	`HerdrPort::send_text` and `send_keys` exist in the provisional port. Under I4 no verb may use them to change which
880	session a pane shows or what it is doing; #640 uses them only in its tests, which type into panes they created in a scratch
881	Herdr session, #646 names the permitted uses, and #645's tests fail on any keystroke.
882	
883	**Deferred to #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each registered pane,
884	and when it moves after a switch or reset. The wiring (#649) wires the ports and leaves DRIVEN to the cutover.
885	Until the cutover wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the
886	hub drives, and leaves `last_observed.driven` as stored.
887	A switch or reset (#645) likewise leaves `last_observed.driven` as stored; the hub's DRIVEN session follows the new
888	`session_of_record` once the cutover wires it.
~~~~

**E64: `docs/adr/ADR-0021.md:935-936`** — Section 13: the sentence about the hub's own registration.

Command: `sed -n '935,936p' docs/adr/ADR-0021.md | awk -v s=935 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
935	The hub's own registration, the fifth record of the incident, is replaced by `session_of_record` once the cutover (#654)
936	points the hub's DRIVEN session at it (section 11).
~~~~

**E65: `docs/adr/ADR-0021.md:1037-1049`** — "Deferred to named stories": the DRIVEN bullet and the `reset --first` bullet.

Command: `sed -n '1037,1049p' docs/adr/ADR-0021.md | awk -v s=1037 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1037	## Deferred to named stories
1038	
1039	- The roster's `--json` shape beside the envelope: #648.
1040	- How the hub's DRIVEN session follows `session_of_record`: #654 (the wiring, #649, leaves it to the cutover).
1041	- The exact file layouts of `panes.json` and `profiles.json`, the long-poll window, and the feed's retained window: #639 and
1042	  #661.
1043	- Which closed failure code a mismatch observed after `act` carries: decided, `unavailable` (exit 1), section 8: #644 for
1044	  launch and relaunch; #645 for switch and reset.
1045	- The refusal of a pane that is not idle or holds a question, and `reset --first`: decided, #645 part 2, through two
1046	  default-armed `HarnessPort` methods (section 8, "The prompt and activity seam"). Their real implementation is built, in
1047	  the OpenCode adapter (#642 part 3); the fake's recording is #684's; the wiring (#649) makes the first message's path live,
1048	  and whether it moves to the hub's one prompt path is #828's.
1049	- `HerdrPort`: #640 implements it as merged; the label is added by #808; a pane with no cell stays out of the snapshot (the
~~~~

**E66: `docs/protocol/v2.md:276-300`** — Section 4, the method table (the rows `session/prompt`, `session/cancel`, `session/answer`).

Command: `sed -n '276,300p' docs/protocol/v2.md | awk -v s=276 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
276	| Method | Kind | Direction | params | result |
277	| --- | --- | --- | --- | --- |
278	| `circuit/join` | request | body→hub, first frame on a fresh socket | `{secret, hostname, body_pubkey, body_x25519_pubkey}` | `{client_id}`; hub **closes the socket** after replying |
279	| `circuit/authenticate` | request | body→hub, first frame | `{protocol, token_id, hostname, advertised_url, message}` — `protocol` is this body's version claim, checked before Noise (issue #340); `message` is Noise XK message 1 (`e, es`), hex | `{message}` — Noise XK message 2 (`e, ee`), hex; not an ack |
280	| `circuit/prove` | request | body→hub, second frame | `{token_id, message}` — Noise XK message 3 (`s, se`), hex | `{ok: true}` |
281	| `circuit/hello` | request | both, after `circuit/prove` | the hello document (§5) | `{}` |
282	| `circuit/ping` | request | both | `{}` | `{hostname, ts}` |
283	| `circuit/superseded` | notification | hub→old socket | `{}` | — (then the hub **closes** it) |
284	| `query/status` | request | both | `{}` | the status document (§5.1) |
285	| `query/caps` | request | both | `{}` | the caps document (§5.2) |
286	| `query/support` | request | both | `{feature}` | the support answer (§5.3) |
287	| `query/protocol` | request | both | `{version?}` | `{session:3, min, max, asked?, ok?}` |
288	| `session/presence` | notification | body→hub | `{hostname, sessions:[{name, harness, state, mode, harness_session_id?}]}` — `state` names are A2A's | — |
289	| `session/prompt` | request | hub→body | `{session, message, meta?}` (`message` is an A2A `Message`, `role:"user"`) | `{stopReason, state, message}` (§6) |
290	| `session/update` | notification | body→hub | `{session, prompt_id, seq, parts:[Part]}` (a streamed text chunk is a text `Part`; the coalescer merges consecutive text parts) | — |
291	| `session/cancel` | request | hub→body | `{session}` | `{applied: true}` — sent **only after** the cancel was applied |
292	| `session/answer` | request | hub→body | `{session, choice}` | `{applied: true}` — sent **only after** the driver resumed the turn (issue #151) |
293	| `admin/status` | request | body→hub, admin-role only (§3.2) | identical to `control/status` (none) | identical to `control/status`'s reply |
294	| `admin/roster` | request | body→hub, admin-role only | identical to `control/roster`'s `{all?, prefix?}` | identical to `control/roster`'s `{rows}` |
295	| `admin/say` | request | body→hub, admin-role only | identical to `control/say`'s `{session, text, queue?, grant?, timeout_ms?}` | identical to `control/say`'s reply |
296	| `admin/interrupt` | request | body→hub, admin-role only | identical to `control/interrupt`'s `{session, text?}` | identical to `control/interrupt`'s reply |
297	| `admin/answer` | request | body→hub, admin-role only | identical to `control/answer`'s `{session, choice}` | identical to `control/answer`'s `{session, applied}` |
298	| `admin/wait` | request | body→hub, admin-role only | identical to `control/wait`'s `{sessions?, prefix?, until?, after?, timeout_ms?}` | identical to `control/wait`'s `{matched, rows}` |
299	| `admin/query` | request | body→hub, admin-role only | `{target?, method, params?}` — `target` present maps to `control/query_remote`, absent to `control/query_local` | identical to whichever local/remote `hub query` form it maps to |
300	
~~~~

**E67: `docs/protocol/v2.md:507-527`** — Section 7, presence: `harness_session_id` "present for `attach`".

Command: `sed -n '507,527p' docs/protocol/v2.md | awk -v s=507 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
507	## 7. Presence
508	
509	`session/presence` is a **notification** (body→hub), sent periodically as
510	a heartbeat and on every (re)connect:
511	
512	```json
513	{"jsonrpc":"2.0","method":"session/presence","params":{"hostname":"kiwi","sessions":[
514	  {"name":"alpha","harness":"opencode","state":"idle","mode":"spawn"},
515	  {"name":"beta","harness":"opencode","state":"working","mode":"spawn","harness_session_id":"openc-abc",
516	   "turn_started_at":"2026-09-08T12:00:00Z","last_update_at":"2026-09-08T12:00:03Z",
517	   "turn_id":"h-01H...",
518	   "last_turn":{"turn_id":"h-01H...prev","state":"completed","stop_reason":"end_turn","ended_at":"2026-09-08T11:59:50Z"}},
519	  {"name":"gamma","harness":"opencode","state":"input-required","mode":"spawn",
520	   "pending":[{"id":"perm-1","kind":"permission","prompt":"Run `rm -rf /`?","options":["allow","reject"]},
521	               {"id":"el-1","kind":"elicitation","prompt":"Which branch to merge?","options":["main","release/1.0"]}],
522	   "turn_id":"h-01H..."}
523	]}}
524	```
525	
526	- `state` is an **A2A session state**: `idle` | `working` | `input-required` (ADR 0005 adds `idle` — A2A states are per *task*, Holler presence is per *session*).
527	- `mode` is `spawn` | `attach`. `harness_session_id` is present for `attach` (the existing harness session being attached to).
~~~~

**E68: `docs/protocol/v2.md:540-540`** — The pane hold paragraph.

Command: `sed -n '540,540p' docs/protocol/v2.md | awk -v s=540 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
540	  - **A third kind, sourced from a pane record (issue #646).** A **pane** hold is not operator state at all: the same enforcement point refuses a prompt to the session of record of a pane whose record says it is parked, unhealthy, or showing a session other than the one it drives, whatever route the prompt takes (a bare `say SESSION` included). It never appears on a roster row (`hold`/`hold_reason`/`held_since` stay operator fields), `release` does not lift it (the pane record owns it: `pane unpark` or `pane doctor` is the remedy), and the refusal carries `data.hold_kind: "pane"` with `data.reason` set to the stable pane code (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) instead of a free-text reason, and no `data.since` ([ADR 0021](../adr/ADR-0021.md) records the predicate).
~~~~

**E69: `docs/protocol/v2.md:545-551`** — "Attach-mode sessions": where the rebind paragraph goes.

Command: `sed -n '545,551p' docs/protocol/v2.md | awk -v s=545 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
545	### Attach-mode sessions (issue #195)
546	
547	`mode:"attach"` is dispatched to `HttpAttachDriver` instead of `AcpDriver`
548	(§`crates/holler-body/src/http_attach_driver.rs`) — it never spawns anything;
549	it attaches to a harness session another process already owns.
550	
551	- **Eager, not lazy.** Unlike a `spawn`-mode session (whose driver comes up
~~~~

**E70: `docs/protocol/v2.md:656-658`** — The error table rows `-32009`, `-32010`, `-32011`.

Command: `sed -n '656,658p' docs/protocol/v2.md | awk -v s=656 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
656	| `-32009` | `session_busy` | `say` to a `working`/`stalled` session without `--queue` (issue #150), **or** to an `input-required` session (issue #151) — `--queue` never overrides the latter; `data.state:"input-required"` distinguishes it from the `working`/`stalled` case, and the CLI hint points at `answer`, not `interrupt`/`--queue`. |
657	| `-32010` | `nothing_pending` | `answer SESSION CHOICE` to a session that is not `input-required` — nothing to resolve (issue #151). |
658	| `-32011` | `session_held` | `say` (plain, `--queue` or `--replace`, including the redirect half of `interrupt SESSION TEXT`) to a session an operator has **held** (`holler hold SESSION`, issue #437). New work is refused; the turn already running, turns already accepted into the body's queue, `interrupt` and `roster` are not affected. `data.reason` is the operator's text (absent when none was given), `data.since` is when the hold was set (RFC 3339) and `data.hold_kind` (issue #460) is `operator` or `default` (absent means `operator`). With `data.hold_kind: "pane"` (issue #646) the refusal comes from a pane record and not an operator hold: `data.reason` is the stable pane code (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`), there is no `data.since`, and the `release` verb does not lift it. |
~~~~

**E71: `docs/pane-control.md:212-236`** — The guide: `reset`, "Do not use `--first` on a parked or held pane", `park`.

Command: `sed -n '212,236p' docs/pane-control.md | awk -v s=212 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
212	### `holler pane reset PANE`
213	
214	Start a pane on a fresh session: create it, show it, record it. `--first TEXT` queues a first message to the new session
215	once it is recorded. The previous session stays on the server, and `doctor` then reports it as a `stray-session`.
216	
217	```bash
218	holler pane reset demo-alpha --first "Read the brief and start."
219	```
220	
221	Refused (3) with `session-busy` or `session-holds-question` while the session of record is running a turn or waits on
222	an answer, and with `orchestrator-pane` and `server-unhealthy` as `switch` is.
223	
224	**Do not use `--first` on a parked or held pane.** The first message goes to the harness directly, outside the hub's
225	prompt path, so the gate that refuses a prompt to a parked or held pane does not see it (ADR-0021 section 8, "The prompt
226	and activity seam"). Sending it through that gate is issue [#839](https://github.com/Performant-Labs/holler/issues/839),
227	not built.
228	
229	### `holler pane park --reason TEXT --release-when WHEN`
230	
231	Take a pane out of service until it is unparked: the record's hold becomes parked, with the reason and the release
232	condition (one line each, at most 200 characters). It takes one pane, or with `--profile` and no pane every pane of the
233	profile. It changes the record only: nothing moves in Herdr and no process is touched. A prompt
234	through the hub to a parked pane's session of record is refused (`pane-parked`); the first message of `reset --first` is
235	not (see `reset`).
236	
~~~~

**E72: `docs/pane-control.md:396-398`** — The guide's DRIVEN column.

Command: `sed -n '396,398p' docs/pane-control.md | awk -v s=396 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
396	| HEALTH | `healthy`, `unhealthy` or `unknown`, as the harness server last reported it (`pane get` shows the reason). |
397	| SHOWN | The session the pane's TUI showed when reconcile last observed it. |
398	| DRIVEN | The session the hub drives, printed as the record holds it and never compared. |
~~~~

**E73: `docs/pane-control.md:420-424`** — The guide: "SHOWN against DRIVEN".

Command: `sed -n '420,424p' docs/pane-control.md | awk -v s=420 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
420	**SHOWN against DRIVEN.** The session of record is the session the hub drives (I2), so SYNC compares SHOWN with it. It
421	is `MISMATCH` when the TUI showed another session or its home screen, and `-` when reconcile has never observed the pane
422	or the pane has no session of record. DRIVEN is printed as the record holds it and is not compared: nothing records it
423	yet (the hub wiring, #649, leaves it as stored; making it follow each pane's session of record is
424	[#839](https://github.com/Performant-Labs/holler/issues/839), not built), so it prints `-`. `pane doctor` uses the same rule for `shown-driven-mismatch` (section 5).
~~~~

**E74: `docs/pane-control.md:733-737`** — The guide's cutover steps 4 to 6.

Command: `sed -n '733,737p' docs/pane-control.md | awk -v s=733 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
733	   `shown-driven-mismatch`, and `holler pane list` shows SYNC `ok` for every pane (section 4).
734	5. **Cut one pane over** to `holler pane launch PANE` and `holler pane reset PANE`, watch it **through a full overnight**,
735	   then move the rest. A launch records the pane's model and does not apply it yet (#829, section 2), and
736	   `reset --first` is not used on a parked or held pane (section 3).
737	6. **Seven days with no invisible session.** Every day `doctor` reports no `shown-driven-mismatch` and no stray session
~~~~

**E75: `docs/pane-control.md:784-794`** — The guide's "Later" list.

Command: `sed -n '784,794p' docs/pane-control.md | awk -v s=784 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
784	
785	Deferred, so not documented here as existing; none of them is available:
786	
787	- the roster extension: the pane columns on `holler roster` (#648);
788	- applying a pane's model, effort and environment to its harness server (#829);
789	- DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
790	- an import that leaves the panes Holler does not drive alone (#835);
791	- the shared subprocess runner of the adapters and the probe (#696);
792	- the real-adapter scenario for `profile apply` (#667);
793	- recreating the Herdr workspace from the stored profiles (#666);
794	- the PROPOSED profile verbs and the PROPOSED doctor finding `profile-drift` (#665).
~~~~

**E76: `CHANGELOG.md:8-11`** — The head of `## [Unreleased]`: the shape of an entry.

Command: `sed -n '8,11p' CHANGELOG.md | awk -v s=8 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
8	## [Unreleased]
9	
10	### Enhancements
11	- Pane control, the operator guide (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): [`docs/pane-control.md`](docs/pane-control.md) documents every `holler pane` and `holler profile` verb ([#652](https://github.com/Performant-Labs/holler/issues/652)).
~~~~

**E77: `crates/holler-cli/tests/hold_single_path_test.rs:1-12`** — The single-sender guard: a source-level scan of the hub.

Command: `sed -n '1,12p' crates/holler-cli/tests/hold_single_path_test.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442
2	//! The session hold has one enforcement point (issue #442, umbrella decision
3	//! 3): every prompt reaches a body through `send_prompt` in
4	//! `crates/holler-hub/src/circuit/dispatch.rs`, and the hold is checked there.
5	//! A second way to put a `session/prompt` on a body's socket would be a way
6	//! around every hold, so this test fails when one appears.
7	//!
8	//! It is a source-level guard, deliberately: it reads the hub's non-test
9	//! source and asserts that the `session/prompt` method name is *built* in
10	//! exactly one place, and that place checks the hold first. The behavioral
11	//! half (the generic request forward cannot carry a prompt either) is
12	//! `hold_hub_test::a_prompt_cannot_be_forwarded_around_the_hold`.
~~~~

**E78: `crates/holler-cli/tests/pane_verbs/launch/record_bytes.rs:20-20`** — The `ses_` fixture: `ses_` and 26 characters.

Command: `sed -n '20,20p' crates/holler-cli/tests/pane_verbs/launch/record_bytes.rs | awk -v s=20 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
20	const PRE_808_RECORD: &str = r#"{"name":"demo-c1r1","generation":1,"herdr":{"session":"scratch","workspace":"main","pane_id":"w1:p1","grid":{"row":2,"col":1,"pos":"r2c1"}},"host":{"name":"localhost","tmux":"demo-c1r1","cwd":"/srv/demo","herdr_api_version":"0.9.1-preview.2026-09-21-0ff0f27e2226"},"harness":{"kind":"opencode","port":48100,"pid":20000,"health":"healthy"},"session_of_record":"ses_00000000000000000000000001","role":"agent","hold":"none","last_observed":{"shown":"ses_00000000000000000000000001","at":1760000000000},"model":{"provider":"demo-provider","model_id":"demo-model","effort":"medium"},"env":[],"context":{"soft":100000,"hard":150000},"probe":{"expect":[]}}"#;
~~~~

**E79: `crates/holler-pane-testkit/src/harness.rs:140-146`** — The fake harness mints `ses_` and 26 hex digits.

Command: `sed -n '140,146p' crates/holler-pane-testkit/src/harness.rs | awk -v s=140 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
140	///   running server was started with; otherwise the fake's refusal [`AGENT_MISMATCH`].
141	/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
142	///   never started, as a timed request that times out or is refused would tell.
143	/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
144	///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
145	///   directory, or list that directory's sessions in creation order.
146	/// - `abort(port, session)` reaches the port. A session outside its data directory is
~~~~

**E80: `crates/holler-pane-testkit/src/harness.rs:498-505`** — `mint_session`.

Command: `sed -n '498,505p' crates/holler-pane-testkit/src/harness.rs | awk -v s=498 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
498	    /// Mint a new session id in the data directory of `port`.
499	    fn mint_session(&mut self, port: u16) -> String {
500	        self.sessions_minted += 1;
501	        let id = format!("ses_{:026x}", self.sessions_minted);
502	        let dir = self.dir(port).to_owned();
503	        self.sessions.entry(dir).or_default().push(id.clone());
504	        id
505	    }
~~~~

**E81: `crates/holler-hub/src/circuit/dispatch.rs:515-548`** — The #646 gate tests' fixtures: synthetic names (`ses-demo-*`), "no name here can name a real pane or session".

Command: `sed -n '515,548p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=515 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
515	    // The same HoldGate now carries the pane registry (`panes`, `Registry::with_panes`):
516	    // a prompt to any session that is a pane's session of record is refused while that
517	    // pane is parked, unhealthy, or has SHOWN != DRIVEN (both observed, differing) —
518	    // the hub's one enforcement point, closing the bare `say <session-of-record>`
519	    // bypass. The pane-state predicate's CLI-side twin is `say_cmd::resolve_pane_target`
520	    // (`holler-cli`); it exists twice only because `holler-pane/**` is frozen (binding 6).
521	    // Seeded names are synthetic (`demo-c*`, `ses-demo-*`): no name here can name a real
522	    // pane or session.
523	
524	    use std::sync::Arc as PaneArc;
525	
526	    use holler_pane::pane::{Health as PaneHealth, Hold as PaneHold, LastObserved};
527	    use holler_pane::{Pane, PaneStore as _};
528	
529	    /// A parked hold for the gate's seeded panes.
530	    fn parked_hold() -> PaneHold {
531	        PaneHold::Parked {
532	            reason: "disk full".to_owned(),
533	            release_when: "later".to_owned(),
534	            since: 5,
535	        }
536	    }
537	
538	    /// A demo pane record for the gate: synthetic name, the session of record, hold,
539	    /// health and last observation the case varies.
540	    fn gate_pane(
541	        name: &str,
542	        session_of_record: Option<&str>,
543	        hold: PaneHold,
544	        health: PaneHealth,
545	        shown: Option<&str>,
546	        driven: Option<&str>,
547	    ) -> Pane {
548	        let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
~~~~

**E82: `crates/holler-hub/src/circuit/dispatch.rs:620-650`** — One gate test: the key and the session are `ses-demo-a`.

Command: `sed -n '620,650p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=620 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
620	    /// every variant — plain, queued and replace alike (the gate is the one
621	    /// enforcement point; `say --queue` passes through it too).
622	    #[tokio::test]
623	    async fn a_prompt_to_a_parked_pane_s_session_of_record_is_refused_in_every_variant() {
624	        let holds = crate::holds::Holds::in_memory();
625	        let (panes, _dir) = pane_registry(&[gate_pane(
626	            "demo-c1r1",
627	            Some("ses-demo-a"),
628	            parked_hold(),
629	            PaneHealth::Healthy,
630	            Some("ses-demo-a"),
631	            Some("ses-demo-a"),
632	        )]);
633	        for (queue, replace) in [(false, false), (true, false), (false, true), (true, true)] {
634	            let key = "io/ses-demo-a".to_owned();
635	            let mut sink = Recording::default();
636	            let gate = HoldGate {
637	                holds: &holds,
638	                key: &key,
639	                grant: None,
640	                panes: Some(PaneArc::clone(&panes)),
641	            };
642	            let res =
643	                send_prompt(&mut sink, gate, "h-01HTESTPANE00000000000000", "ses-demo-a", message(), queue, replace)
644	                    .await;
645	            assert!(
646	                matches!(res, Err(SendPromptError::Refused(ref e)) if e.code == -32011),
647	                "queue={queue} replace={replace}"
648	            );
649	            assert!(sink.0.is_empty(), "a refused prompt puts nothing on the socket");
650	            let Err(SendPromptError::Refused(error)) = res else { unreachable!() };
~~~~

**E83: `crates/holler-cli/tests/pane_verbs/reset.rs:327-378`** — The #645 test that asserts the first message through `HarnessPort::send_prompt` (to be rewritten).

Command: `sed -n '327,378p' crates/holler-cli/tests/pane_verbs/reset.rs | awk -v s=327 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
327	/// lists both), `--first` queues the text to the NEW session only — never the old
328	/// one, never another pane's session or port — and only after the record write. The
329	/// run is part 1's reset otherwise (same fake-log op sequence; no keystroke, by the
330	/// runner's I4 check).
331	#[test]
332	fn reset_with_first_queues_the_message_to_the_new_session_only() {
333	    let cases = reset_over_seam(
334	        || Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]),
335	        &["--first", "ship the fix"],
336	        Activity::Idle,
337	        None,
338	    );
339	    data(&cases);
340	    for case in &cases {
341	        let s1 = case.rig.live(P).session.clone();
342	        let q = case.rig.live(Q).session.clone();
343	        let new = case
344	            .record(P)
345	            .session_of_record
346	            .expect("the new session of record");
347	        assert_ne!(
348	            &new, &s1,
349	            "{:?}: a new session, not the old one",
350	            case.format
351	        );
352	        assert_ne!(&new, &q, "{:?}: not the other pane's session", case.format);
353	        case.assert_recorded(&new);
354	        let ops = [
355	            H::Health,
356	            H::CreateSession,
357	            H::SelectSession,
358	            H::ShownSession,
359	        ];
360	        assert_eq!(case.calls.harness, ops, "{:?}", case.format);
361	        let prompts = case.setup.prompts();
362	        assert_eq!(
363	            prompts.len(),
364	            1,
365	            "{:?}: one prompt, to the new session only: {prompts:?} (old {s1:?}, {Q} {q:?})",
366	            case.format
367	        );
368	        let prompt = &prompts[0];
369	        assert_eq!(prompt.port, case.rig.live(P).port, "{:?}", case.format);
370	        assert_eq!(prompt.session, new, "{:?}: the new session", case.format);
371	        assert_eq!(prompt.text, "ship the fix", "{:?}", case.format);
372	        assert_eq!(
373	            prompt.then_of_record.as_deref(),
374	            Some(new.as_str()),
375	            "{:?}: the prompt follows the record write",
376	            case.format
377	        );
378	    }
~~~~

**E84: `crates/holler-cli/tests/pane_verbs/reset.rs:417-445`** — The #645 test that a failing `send_prompt` after the record is reported honestly.

Command: `sed -n '417,445p' crates/holler-cli/tests/pane_verbs/reset.rs | awk -v s=417 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
417	/// honestly: the run fails with the port's own code, the message says the first
418	/// message did not land, and nothing is appended — no reconcile step (the record is
419	/// correct) and no "is not recorded" clause (the session IS recorded). The record
420	/// and the TUI stand exactly as a successful reset left them.
421	#[test]
422	fn reset_first_prompt_failure_after_the_record_is_honest() {
423	    let fault = PaneError::Unavailable {
424	        what: "the harness".to_owned(),
425	    };
426	    let cases = reset_over_seam(
427	        || Rig::new(&[Seed::new(P, 1, 1)]),
428	        &["--first", "ship the fix"],
429	        Activity::Idle,
430	        Some(fault),
431	    );
432	    for message in failed(&cases, 1, "unavailable") {
433	        assert!(
434	            message.contains("first message did not land"),
435	            "names what failed: {message:?}"
436	        );
437	        assert!(
438	            !message.contains("to reconcile"),
439	            "no reconcile step: {message:?}"
440	        );
441	        assert!(
442	            !message.contains("is not recorded"),
443	            "the session is recorded: {message:?}"
444	        );
445	    }
~~~~

**E85: `crates/holler-cli/tests/pane_verbs/switch.rs:846-866`** — "A switch is unaffected by the activity seam: a switch queries no activity" (switch.rs is 868 lines).

Command: `sed -n '846,866p' crates/holler-cli/tests/pane_verbs/switch.rs | awk -v s=846 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
846	/// AC 4 (part 2): a switch is unaffected by the activity seam: a wrapper that would
847	/// answer busy for any query is never asked (the gate is reset's), and a switch
848	/// queues no prompt.
849	#[test]
850	fn switch_is_unaffected_by_the_activity_seam() {
851	    let build = || {
852	        let (rig, s2) = one_pane();
853	        (rig, (s2, Arc::new(SeamLog::default())))
854	    };
855	    let words = |_: &Rig, (s2, _): &(String, Arc<SeamLog>)| argv(&["pane", "switch", P, s2]);
856	    let exec = |rig: &Rig, (_, log): &(String, Arc<SeamLog>), argv: &[&str], format| {
857	        let seam = SeamHarness::new(rig, P, log, Activity::Busy, None);
858	        run_verb_with(argv, format, rig.ports_with(&seam))
859	    };
860	    let cases = both_with(build, words, exec);
861	    data(&cases);
862	    for case in &cases {
863	        case.assert_recorded(&case.setup.0);
864	        let log = &case.setup.1;
865	        assert!(log.activity().is_empty(), "a switch queries no activity");
866	        assert!(log.prompts().is_empty(), "a switch queues no prompt");
~~~~

**E86: `crates/holler-cli/tests/pane_integration/scenario.rs:190-205`** — The opt-in scenario step `reset_with_a_first_message`.

Command: `sed -n '190,205p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=190 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
190	/// Step 3: a reset of C2 with a first message, through `send_prompt` (#642 part 3). The
191	/// dead-end provider is retried until the turn is aborted, so the session reads `Busy`, as D12
192	/// expects; no model is reached.
193	fn reset_with_a_first_message(scratch: &Scratch, p2: u16) {
194	    let before = session_of_record(scratch, C2);
195	    scratch.json(&["pane", "reset", C2, "--first", "hlr649 first message"], 0);
196	    let after = record(scratch, C2);
197	    let fresh = after["session_of_record"].as_str().unwrap().to_owned();
198	    assert_ne!(fresh, before, "a fresh session");
199	    assert_eq!(after["last_observed"]["shown"], fresh.as_str(), "SHOWN follows");
200	    let busy = wait_for(Duration::from_secs(15), || {
201	        (scratch.harness().session_activity(p2, &fresh) == Ok(Activity::Busy)).then_some(())
202	    });
203	    assert!(busy.is_some(), "the first message reached the server: its session reads Busy");
204	    no_model_can_be_reached(p2);
205	}
~~~~

**E87: `crates/holler-cli/tests/attach_mode_test.rs:224-240`** — A real hub, a real body and a fake OpenCode server: the pattern of the default-run end-to-end test.

Command: `sed -n '224,240p' crates/holler-cli/tests/attach_mode_test.rs | awk -v s=224 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
224	async fn attach_session_appears_on_roster_with_mode_attach_and_harness_session_id() {
225	    let _guard = SERIAL.lock().await;
226	    let server = FakeServer::start().await;
227	    let state = StateDir::new();
228	    let hub = Hub::start(&state);
229	    join_fresh(&state, &hub.ws_url(), "kiwi");
230	    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_1")]);
231	    let body = Body::start(&state, &config);
232	
233	    let row = wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);
234	    assert_eq!(row["mode"].as_str(), Some("attach"), "row: {row}");
235	    assert_eq!(row["harness_session_id"].as_str(), Some("ses_1"), "row: {row}");
236	    assert_eq!(row["state"].as_str(), Some("idle"), "an attached session with nothing in flight is idle: {row}");
237	
238	    body.stop(&state, Duration::from_secs(5));
239	    hub.stop(Duration::from_secs(5));
240	}
~~~~

**E88: `docs/setup-wizard.md:88-104`** — The wizard names attach sessions `alpha` and `beta`, independent of pane names.

Command: `sed -n '88,104p' docs/setup-wizard.md | awk -v s=88 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
88	# sessions can live on different remote machines, so nothing here assumes a single shared host.
89	[[session]]
90	name = "alpha"
91	harness = "opencode"
92	mode = "attach"
93	endpoint = "http://127.0.0.1:47001"
94	remote_host = "remote-a"                              # for ssh — an entry in ~/.ssh/config
95	remote_tailnet_host = "remote-a.example.ts.net"     # for the attach pane's URL
96	# session_id filled in by the wizard once a real session exists — leave absent in a template
97	
98	[[session]]
99	name = "beta"
100	harness = "opencode"
101	mode = "attach"
102	endpoint = "http://127.0.0.1:47002"
103	remote_host = "remote-a"
104	remote_tailnet_host = "remote-a.example.ts.net"
~~~~

**E89: `b3f0295:docs/adr/ADR-0021.md:1165-1182`** — #831 branch at `b3f0295`: item 18, `say` needs #839 and a body (#850).

Command: `git show b3f0295:docs/adr/ADR-0021.md | sed -n '1165,1182p' | awk -v s=1165 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1165	18. **`holler say` and the acceptance step: it needs #839, and more (checked 2026-10-11).** The scratch scenario's step "deliver
1166	    a message to each through `holler say`" cannot be met by this story alone, for either harness:
1167	    - The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body that drives
1168	      its harness, addressed by the body's advertised name (ADR 0014 point 5; `send_prompt` and `pane_gate` in
1169	      `holler-hub/src/circuit/dispatch.rs`). `holler say --pane NAME` resolves the pane to its `session_of_record` and uses
1170	      that as the address (`resolve_pane_target`), which matches no body's name: the id is an OpenCode `ses_…` or a Claude
1171	      UUID, and the join from a body's `harness_session_id` to `Pane.session_of_record` is **#839** (DRIVEN, not built). So
1172	      `say --pane`, its holds and its pane gate reach no real pane of either harness until #839.
1173	    - A Claude Code pane additionally has **no body at all**: the hub constructs no adapter (decision 1 of section 5), and the
1174	      adapter this story builds is called by the pane engine, not by a body. A body driver for it is the follow-up story **#850**.
1175	    So this story proves delivery where it can, at `HarnessPort::send_prompt` (item 7) against `FakeClaude` and the opt-in
1176	    real-binary test, and through `reset --first`; the `say` step of the scenario is written down, marked blocked by #839 and
1177	    #850, and **#831 does not close on it**. Nothing here routes a prompt around the hub's gate: the Claude pane's `say` is not
1178	    given a CLI-side path, because that would be a second enforcement site for holds and parks.
1179	
1180	**Not built here, and where it goes.** The body driver for `say` (#850); the hub's join (#839); `pane-stalled` (with #837's
1181	time source); the `PreToolUse` hook gate; a permission-mode, allowed-tools and questions spec field; `--agent` for Claude;
1182	answering a permission prompt by hook; interrupt with replacement text; the wizard's use of the flag (#803); a Windows or
~~~~

**E90: `b3f0295:docs/adr/ADR-0021.md:1208-1213`** — #831 branch at `b3f0295`: its edit of the DRIVEN "Deferred" bullet.

Command: `git show b3f0295:docs/adr/ADR-0021.md | sed -n '1208,1213p' | awk -v s=1208 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1208	## Deferred to named stories
1209	
1210	- The roster's `--json` shape beside the envelope: #648.
1211	- How the hub's DRIVEN session follows `session_of_record`: #839 (it was #654's; the wiring, #649, leaves it to the cutover).
1212	- A body that lets `holler say --pane` reach a Claude Code pane: #850 (section 15, item 18). `pane-stalled` waits with #837's time
1213	  source, and the `PreToolUse` gate, a permission-mode, allowed-tools and questions spec field and `--agent` for Claude are later
~~~~

**E91: `b3f0295:crates/holler-cli/tests/pane_claude/lifecycle.rs:205-220`** — #831's test that `reset --first` reaches the new Claude process (through the engine's `send_prompt`).

Command: `git show b3f0295:crates/holler-cli/tests/pane_claude/lifecycle.rs | sed -n '205,220p' | awk -v s=205 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
205	/// W4: the first message goes to the process the reset started, never to the stopped one (the
206	/// endpoint of the record the relaunch stored, not of the record the plan read).
207	#[test]
208	fn reset_first_reaches_the_new_process() {
209	    let rig = one_claude_pane();
210	    let old_pid = rig.record(CLAUDE).harness.pid.unwrap();
211	
212	    let run = rig.run(
213	        &["pane", "reset", CLAUDE, "--first", "start on the review"],
214	        Format::Json,
215	    );
216	
217	    assert_eq!(run.code, 0, "{run:?}");
218	    let process = rig.process_of(CLAUDE).unwrap();
219	    assert_ne!(process.pid, old_pid);
220	    assert_eq!(process.inbox, vec!["start on the review"]);
~~~~

**E92: `526bbd3:crates/holler-pane/src/ports.rs:11-30`** — #835 branch at `526bbd3`: the amendments list of `ports.rs`, "Both are required so that the compiler finds every implementation".

Command: `git show 526bbd3:crates/holler-pane/src/ports.rs | sed -n '11,30p' | awk -v s=11 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
11	//!
12	//! **Frozen when #637 merges**, after which a change goes through the epic's
13	//! amend-first rule. [`HerdrPort`], and the minimal data types it takes and returns,
14	//! stay provisional until the spike #636 (Herdr) reports. [`HarnessPort`] is final:
15	//! the spike #635 (OpenCode) confirmed it, and #642 built it. Four amendments since:
16	//!
17	//! - #645's two default-armed seam methods, [`HarnessPort::send_prompt`] and
18	//!   [`HarnessPort::session_activity`], whose defaults keep every impl that does not
19	//!   override them as it was;
20	//! - #802's: `serve` became the pure [`HarnessPort::serve_command`], which takes a
21	//!   [`ServeSpec`] and returns the server's argv, beside the required
22	//!   [`HarnessPort::verify_serving`]; and [`HostPort::run`] takes the directory it starts
23	//!   its program in. The host starts and stops a pane's processes, its harness server
24	//!   among them;
25	//! - #832's: [`HerdrPort::observe_pane`] and its batch form [`HerdrPort::observe_panes`],
26	//!   what Herdr says about a pane that no record names;
27	//! - #835's: the required [`HostPort::foreground_commands`], the command name of each pane
28	//!   process of a tmux session (how the import tells a pane that runs Claude Code), and
29	//!   [`HerdrPort::observe_pane`] lost its default. Both are required so that the compiler
30	//!   finds every implementation, a wrapper port included: a default answers silently in place
~~~~

**E93: `f5f7013:docs/handoffs/648-brief.md:87-87`** — #648 branch at `f5f7013`: its mismatch rule is "the existing pair rule".

Command: `git show f5f7013:docs/handoffs/648-brief.md | sed -n '87,87p' | awk -v s=87 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
87	- **The mismatch rule is the existing pair rule:** a pane is a mismatch only when both `last_observed.shown` and `last_observed.driven` are `Some` and unequal; one side absent is never a mismatch (`say_cmd.rs` and the hub's `dispatch.rs` apply the same rule to a recorded pair, E11). `reconcile::shown_differs` answers a different question (shown against `session_of_record`) and is not used here.
~~~~

**E94: `docs/adr/ADR-0021.md:37-37`** — The `Pane.name` row: a pane's name is a `SessionName` newtype, "also the tmux session name".

Command: `sed -n '37,37p' docs/adr/ADR-0021.md | awk -v s=37 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
37	| `name` | `PaneName` | A newtype over `holler_proto::vocab::SessionName`, so a pane name obeys the [ADR 0005](ADR-0005.md) grammar. It is also the tmux session name. Names such as `hj-c1r2` are names, not positions. |
~~~~

**E95: `crates/holler-cli/tests/hold_single_path_test.rs:43-68`** — The single-sender guard's scan: exactly one `session/prompt` builder, in `circuit/dispatch.rs`, the hold checked before it.

Command: `sed -n '43,68p' crates/holler-cli/tests/hold_single_path_test.rs | awk -v s=43 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
43	fn session_prompt_is_built_in_exactly_one_place_and_the_hold_is_checked_there() {
44	    let mut files = Vec::new();
45	    rust_files(&hub_src(), &mut files);
46	    let mut builders = Vec::new();
47	    for f in &files {
48	        let code = code_of(f);
49	        // Constructing the request: the method name as a request's method.
50	        // (`static_wire_method` in circuit.rs only maps an inbound name to a
51	        // log label and never sends.)
52	        for (i, line) in code.lines().enumerate() {
53	            if line.contains("\"session/prompt\"") && line.contains("Envelope::request") {
54	                builders.push(format!("{}:{}", f.strip_prefix(hub_src()).unwrap().display(), i + 1));
55	            }
56	        }
57	    }
58	    assert_eq!(builders.len(), 1, "exactly one place may build a session/prompt request: {builders:?}");
59	    assert!(builders[0].starts_with("circuit/dispatch.rs"), "the one builder must be send_prompt: {builders:?}");
60	
61	    // And the hold is checked in that function before the request is built.
62	    let dispatch = code_of(&hub_src().join("circuit/dispatch.rs"));
63	    let start = dispatch.find("async fn send_prompt").expect("send_prompt exists");
64	    let body = &dispatch[start..];
65	    let check = body.find("gate.holds.admit(").expect("send_prompt checks the hold");
66	    let build = body.find("Envelope::request").expect("send_prompt builds the request");
67	    assert!(check < build, "the hold must be checked before the prompt is built or sent");
68	}
~~~~

**E96: `crates/holler-pane/src/tx_switch.rs:78-94`** — `parse_session_id`: "OpenCode's ids are `ses_` and 26 letters and digits".

Command: `sed -n '78,94p' crates/holler-pane/src/tx_switch.rs | awk -v s=78 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
78	/// A harness session id typed by a person: 1 to [`SESSION_ID_MAX`] ASCII letters, digits,
79	/// `_` or `-` (OpenCode's ids are `ses_` and 26 letters and digits). Anything else is
80	/// `usage`, and the message quotes the text with [`quoted`], so no whitespace, control
81	/// character or shell metacharacter reaches a port or a message.
82	pub fn parse_session_id(text: &str) -> Result<String, PaneError> {
83	    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
84	    if (1..=SESSION_ID_MAX).contains(&text.len()) && text.bytes().all(allowed) {
85	        return Ok(text.to_owned());
86	    }
87	    Err(PaneError::Usage {
88	        message: format!(
89	            "invalid session id {}: a session id is 1 to {SESSION_ID_MAX} ASCII letters, \
90	             digits, '_' or '-'",
91	            quoted(text)
92	        ),
93	    })
94	}
~~~~

**E97: `crates/holler-body/src/http_attach_driver.rs:174-216`** — `HttpAttachDriver`: the session id is a field baked into its background task; `attach` builds `reqwest::Client::new()` (no timeout) and makes the existence check.

Command: `sed -n '174,216p' crates/holler-body/src/http_attach_driver.rs | awk -v s=174 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
174	pub struct HttpAttachDriver {
175	    command_tx: mpsc::UnboundedSender<Command>,
176	    connection_task: tokio::task::JoinHandle<()>,
177	    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
178	    client: reqwest::Client,
179	    endpoint: String,
180	    session_id: String,
181	    shared: Arc<Mutex<Shared>>,
182	}
183	
184	impl HttpAttachDriver {
185	    /// Attaches to `config`'s already-existing OpenCode session.
186	    ///
187	    /// Fails closed — [`DriverError::NotFound`], never `session/new`, never
188	    /// exec `command` — if `endpoint`/`session_id` are missing (defensive;
189	    /// [`crate::config`] already validates this at load time) or if neither
190	    /// path prefix's existence check returns success.
191	    pub async fn attach(config: &SessionConfig) -> Result<Self, DriverError> {
192	        let endpoint = config
193	            .endpoint
194	            .clone()
195	            .filter(|s| !s.is_empty())
196	            .ok_or_else(|| DriverError::NotFound("no endpoint configured".to_string()))?;
197	        let session_id = config
198	            .session_id
199	            .clone()
200	            .filter(|s| !s.is_empty())
201	            .ok_or_else(|| DriverError::NotFound("no session_id configured".to_string()))?;
202	
203	        let client = reqwest::Client::new();
204	        match check_exists(&client, &endpoint, &session_id).await {
205	            Ok(()) => log_debug(
206	                "attach",
207	                vec![("event", "attached".to_string()), ("session_id", session_id.clone())],
208	            ),
209	            Err(e) => {
210	                log_warn(
211	                    "attach",
212	                    vec![("event", format!("attach_failed: {}", e.message()))],
213	                );
214	                return Err(e);
215	            }
216	        }
~~~~

