# [feat(pane): DRIVEN follows each pane's session of record: the hub joins a pane to the body that drives it, the gate keys on the join, switch and reset move it, and `reset --first` goes through the hub's gate](https://github.com/Performant-Labs/holler/issues/839)

Part of epic #633. This is the complete statement of story #839, prepared on 2026-10-11 and **re-cut once on 2026-10-12 from `main` at `b76f4d2`** (which held #831, PR #857, #665, #648 and #667) and **refreshed once on 2026-10-12 for #835** (PR #860), on `main` at `4e691d9`, which holds all five. It **absorbs #828** (closed): the issue text is the source of truth where this brief is silent, and this brief is the decision where they differ (the differences are listed in "Where this brief is the decision"). The story is **contract amendment first** (ADR-0021 section 11 and the passages listed in "The ADR and the dependent docs", then `docs/protocol/v2.md`), then tests (RED), then code. Rigor is `in-session`, as "Pipeline" declares it: no outside model reviews the brief, so the Evidence section quotes every source fact a decision rests on, and a reviewer needs no other source. Cite an entry as E<n>. "Plan-check history" says what the three plan checks blocked and where this cut answers each. Every Evidence command was re-run on the tree this cut was written on, and again at the #835 refresh, and "Line numbers re-checked" says what moved.

**Status of the decisions.** The five questions the issue asks (the address, how a body follows, who records DRIVEN and when, hold keys, the order of `reset --first`) and the decisions that follow from them (D1 to D12) were **decided by the preparer under the operator's standing rules ("decide, don't ask"), to be confirmed at the plan check**. Each lists what it rejected and why. "Questions only the operator can answer" lists the five the preparer did not decide. Three further decisions were taken by the main session at the plan check (P1 to P3 below: the generic forward stays closed, one source for the body's attach id, the ADR amendments and the bare-id CLI form); they are recorded as decided, with what each rejected, and are not to be re-opened. Three more were taken at the second plan check (P4 to P6 below: `driver_of` accepts a body only on positive evidence, the three `--pane` verbs keep their hold exit, a switch onto a session another body drives is refused in the plan); same rule. Two more were taken by the operator after the third (P7 and P8 below: the hub marks an offline body's `-32004` with an additive `data.reason`, and the exclusion rule has one statement, in D3), recorded on 2026-10-12; same rule.

## Plan-check history

Three plan checks have run on this story (their records are in `docs/handoffs/839-plan-checks/`; the first pass's handoff is at `6211c86` and the second's at `8b021d5` in git history). None of their answered findings is re-opened.

1. **First pass (2026-10-11), BLOCK, 3 blocks and 10 warns.** The rebind leaked onto the generic forward; the body silently returned to the old session; ADRs 0005, 0003 and 0014 were not amended. Answered by P1 (the rebind has its own forward), P2 (one source for the attach id, mirrored; a restart reverts) and P3 (the three ADRs amended; a typed bare id refused), and the warns by D1 to D5, D8, D10, D11 and the blast radius.
2. **Second pass (2026-10-11), BLOCK, 1 block and 9 warns.** `driver_of`'s only evidence was membership in the pane's server list, which fails both ways. Answered by P4 (positive evidence, D3's rules (i) to (v), `Unproven`), P5 (the routed verbs keep exit 4), P6 (`session-driven-elsewhere`), and the `talk::admit`, forward-helper, remedy-builder, presence-mirror and fake-reuse decisions of D2, D3 and D5.
3. **Third pass (night of 2026-10-11), BLOCK, 2 blocks and 6 warns.** (1) The exclusion of another pane's body was stated two ways: answered by P8 and by D3's single statement, "the exclusion rule", which every other place points at. (2) `NotDriven` could not be told by code, because the hub sends `-32004` both for an offline body and after delivery: answered by P7 (the additive `data.reason: "offline"`), D5's `classify`, the D7 table, Acceptance 18 (three turn-outlives-the-wait cases) and Acceptance 11 (a new golden file). The warns: the `.send_prompt(` scan now allows the two wrapper forwards (D5, Acceptance 13); the tests that drive `send_prompt` stay in `circuit/dispatch.rs` and every `mod …;` line is last in its file ("Where the code goes", Acceptance 4); one selection, `holler_pane::gate::first_refusal`, serves the hub and `FakeHub` (D1, D10); the #803 row no longer spells the wizard skill's old directory; the clashes table, counts and merge order were re-read on 2026-10-12; `SESSION_DRIVEN_ELSEWHERE` is declared in `tx_switch.rs`, the control parameter is `token_id`, and advertised values are quoted and the remedy id validated (D2, D3, D7).
4. **What moved under the brief since (2026-10-12).** #831, #665, #648, #667 and then #835 merged. Every excerpt was re-cut from `main`; D3 says what a single-process pane (#831) does in the new order; the ADR edit list gains #831's passages (section 8's single-process paragraph, section 9's rows, item 18); the `Ports` literals are 20 in 17 files (#835 added two); `HarnessPort` takes an endpoint; `HerdrPort::observe_pane` and `HostPort::foreground_commands` are required (#835).

## Where this brief is the decision

**The facts that force the design** (each is in the Evidence section):

- F1. The hub's prompt path addresses a body session by its ADR 0005 name and by nothing else: `talk::say`, `talk::answer` and `interrupt` each parse the target as a `RoutableName` and match it against `ad.name` (E9, E11, E12, E7). An OpenCode session id never parses (E39: lower-case letters, digits and `-`; the id has `_`).
- F2. `say --pane` sends the pane's `session_of_record` as that target (E21), so for a real pane the hub answers `unknown session` today.
- F3. The pane gate matches `pane.session_of_record == the name being sent` (E2, E4). For a real pane that never matches; it fires only for #646's synthetic `ses-demo-*` fixtures (E81, E82).
- F4. A body's harness session is configuration, fixed when the body starts (E44, E45), advertised as `harness_session_id` (E40, E46). The closed v2 catalog has no request that changes it (E42, E43). The hub runs no adapters, and a prompt reaches a body only through `send_prompt` (E3, E77).
- F5. Nothing writes `last_observed.driven` (E32, E34). The engine sends `reset --first` straight to `HarnessPort::send_prompt` (E31, E158), and `Ports` has no port that faces the hub's live sessions (E29).
- F6. Holds are keyed `<label>/<session>` (E15), and a hub started with `--join-held` holds a name on first sight (E17).
- F7. A body's attach id is read from the session task's own configuration every time the task attaches: at start, on the 30 s retry while an attach session has no driver, and at the next prompt after a turn that ended in error (OpenCode's `session.error` ends the turn with `StopReason::Error`, and the task then drops its driver) (E101, E102, E103, E104, E105). Presence, `query/support` and the registry's baseline read the as-started `SessionConfig` (E108, E109, E46). A re-point that changes only some of these readers is undone silently by the others.
- F8. `LiveHandle::query` is the hub's generic request forward: `control/query_remote` hands any `params.method` to it, and `admin/query` with a target maps to `query_remote` (E98, E99); its allowlist (E6) is their only filter. The circuit keeps one `pending_query` slot per connection, overwritten by each forwarded request (E116).
- F9. Servers that share a data directory list each other's sessions (`check_unclaimed`: "the server's list alone would let a switch show one pane another's conversation", E137), so "the pane's own server lists this id" is no proof that a body belongs to the pane.
- F10. A relaunch keeps the session of record only if the restarted server still lists it, else creates a new one (E129), and keeps the stored DRIVEN (E128): the body then advertises a session that its pane's server no longer lists.
- F11. The hub's `-32004 not_connected` carries no `data` and is sent for five different things. At resolve time: `say`'s offline body (`SayError::NotConnected`), `interrupt`'s and `answer`'s. After the prompt was delivered: `say`'s turn `Timeout` ("no reply from {session} within {secs}s") and `Cancelled`, `interrupt`'s `AckTimeout` and the failed redirect (`PromptFailed`) (E160, E161, E162; E9 for the resolve-time raise). The CLI waits `timeout + 5 s` for a `say`, so the hub's `Timeout` reaches it as a refusal. A classification by code alone would print `pane-not-driven` (exit 3) for a `say --pane` whose turn outlives `--timeout` while the body runs the turn on; today that is exit 1 with the hub's text. `ControlCall::say_with` derives the hub's `timeout_ms` and the call's read timeout from one argument (E19), and `say_cmd::run` overrides only `call.timeout` for the accept-and-return form (E168), so a `Send` built with the 2 s accept wait as its budget would make the hub time the turn out at 2 s.
- F12. #831 (merged) gave a pane's harness one of two shapes, `Shape::ServerAndTui` and `Shape::SingleProcess` (E177): a single process's switch and reset are a relaunch (`tx_launch::relaunch_recorded`, which keeps the stored DRIVEN, E128, E178), `HarnessPort` is keyed by `at: &HarnessEndpoint` (E28), `HarnessRouter` forwards each call to one of two harnesses (E159), `Activity` has a fourth variant, `WaitingPermission` (E118, E119), and reconcile skips the server-list rules for a single process (E176).

**Decided at the plan check (2026-10-11, the main session; final, not to be re-opened).**

- **P1. The generic forward stays closed; the rebind has its own forward.** `LiveHandle::query`'s allowlist (`query/*` and `session/answer`) and its comment are **not changed** (E6). `session/rebind` is sent by a dedicated `LiveHandle::rebind(params, timeout)` in `crates/holler-hub/src/live/target.rs` (a child module of `live`, so its `impl LiveHandle` block reaches the private `tx` and `LiveCommand`), as a `LiveCommand::Query` with `method: "session/rebind"`; `circuit.rs` grows by nothing. Why: an allowlisted `session/rebind` would be carried by `control/query_remote` and by `admin/query` from an admin-role remote connection (E98, E99), so a local control client or a remote admin could re-point any body without `control/rebind`'s cache and roster update, and it would contradict D2's choice to keep the rebind off the admin surface. Rejected: **(a) widen the allowlist** (that exposure; Acceptance 13 would have pinned it); **(b) a new `LiveCommand::Rebind` variant** (grows `circuit.rs`, 898 lines); **(c) an `admin/rebind`** through `dispatch_allowlisted` (a remote re-point is not asked for: question 4).
- **P2. One source for the body's attach id: `Inner.config.session_id`, mirrored into `SessionPresence`.** Every reader named in F7 reads that one source (D2 says how). A body **restart** reverts to the id in its configuration: accepted, **not persisted** (persistence would add state under the state-directory rules, a path, an atomic write, mode `0600`, a corrupt file failing closed, for a window the cutover does not need). The remedy is `holler pane switch` to the session of record (or `holler pane reset`, which re-points the body to the new session); `doctor` reports the reverted body as `shown-driven-mismatch` once `driver_of` can find it, and P4 says it finds a body by positive evidence only: on a pane whose server lists only its own sessions the listed-candidate fallback finds a reverted body; on servers that share a data directory a reverted body's configured id is stale for every pane and nothing proves whose it is, so the verbs refuse and the operator pairs the body with the two-step switch of D3. Rejected: **(a) a "swapped state" value beside the configuration** (two sources; the re-attach paths read the old one); **(b) persisting the binding** (above); **(c) rewriting the body's configuration file** (the wizard owns it, E88).
- **P3. ADR 0014 (point 5 and Consequences), ADR 0005 section 3, ADR 0003's Addressing line and ADR-0021's Amends line are all amended**, and a bare harness id as the `SESSION` positional of `say`, `interrupt` and `answer` **stays refused** (`unknown session`, as before): the id is an address only for `--pane` resolution and the hub's gate. Pinned by a test that `holler say ses_… TEXT` is refused (Acceptance 18). Rejected: **(a) let the bare form work** (a second typed address that the guide, ADR 0003 and every script would owe, and a stale id would reach a body); **(b) a hub-side flag that says who typed a target** (a new param on three methods for one caller); **(c) amend only ADR 0014's rebind bullet** (ADR 0005 section 3, ADR 0003 and ADR 0014's own point 5 would contradict D1).

**Decided at the second plan check (2026-10-11, the main session; final, not to be re-opened).**

- **P4. `driver_of` accepts a body as a pane's body only on positive evidence (option (a) of the handoff).** The exact rule is in D3. A live attach body belongs to a pane only if it advertises, in this order, (i) the pane's session of record, (ii) the pane's stored `last_observed.driven` (the record's own memory: a body whose session the pane's server no longer lists is still found, E129), or (iii) for a switch, the switch's target. Bodies that other panes' evidence ties to those panes are never candidates (the exclusion rule, D3). The old fallback, "the only candidate that the pane's own server lists", stays only when that server lists **no other pane's session of record**, the shared-listing signal that `check_unclaimed` already uses (E137). With a shared listing and no evidence a listed body is `Unproven`: the verbs refuse `pane-not-driven` naming it and the two-step remedy, and `doctor` records DRIVEN `None` with no remedy. Why: a pane with no connected body (the cutover, a fleet booting after a reboot) would otherwise claim another pane's behind body, and `pane switch` or `pane reset` would re-point it silently; and a relaunch onto a new session leaves a body on an id the restarted server no longer lists, which membership in the server's list never finds. Rejected: **(b) join on the attach endpoint** (each body's `query/status` carries `session_list[].endpoint`, E152, so a candidate would have to attach through the pane's own port): it needs no wire change, but it costs one `query/status` per body per plan or pass through `control/query_remote` (which shares the per-connection slot of D2), makes `driver_of` depend on a reply that a busy or silent body may not give (failing closed would then refuse the very pane that needs repairing), and compares endpoint strings (host spelling, a trailing slash) that nothing normalises; (a) uses only facts that the record and the roster already hold. (The reasons for rejecting (b) are the preparer's; the main session recorded the choice only.)
- **P5. The three `--pane` verbs keep today's hold behaviour; only `reset --first` reports a hold as `session-held`.** `say`, `interrupt` and `answer` with `--pane`: an operator or default hold is still the generic held refusal with exit 4 (`HELD_EXIT_CODE`, "so a script can tell held from failed", E151), a pane hold (`hold_kind: "pane"`) is still `pane_hold_refusal` with its `PaneHeld` `--json` form (E149, E24), and #646's `every_other_held_error_falls_through_to_the_generic_arm` (E150) stays green and unchanged. What those verbs gain is `pane-not-driven` (exit 3) for `unknown session` and `ambiguous` of the pane's id and for the offline-body `not_connected` (P7); every other `-32004` keeps today's exit 1 and text. `pane reset --first` has no exit 4 (the pane verbs exit 0, 1 or 3), so it maps the hub's hold refusal (`hold_kind` `operator` or `default`) to the new open code `session-held` (exit 3). `hub_refusal` returns a classification that both callers use (D5 says which caller maps what). Rejected: **making the three verbs print `session-held` with exit 3**: it changes a documented exit code that scripts rely on, contradicts #646's pin, and the `PaneHeld` JSON cannot be produced from a `PaneError`.
- **P6. A switch onto a session that another live body already drives is refused in the plan.** After `driver_of` (D3) finds the pane's own body, the plan also asks whether a *different* live attach body advertises the switch's target (another pane's behind body, or an operator's own attach row for a session that Holler does not own). If one does, nothing moves and the verb is refused with the new open code **`session-driven-elsewhere`** (exit 3), naming that body as `label/session` with the id. Two bodies on one harness session is what ADR 0014's Consequences call undefined, and it would make `resolve_prompt_target` answer `Ambiguous` for that id after a switch that succeeded. A body that is the pane's own driver and already advertises the target is not "another" (the idempotent re-run). A reset needs no such check: its target is created by the act, so no body can advertise it at plan time. No existing code fits: `session-of-other-pane` means the target is another pane's session of record, read from the pane store (E153), and its remedy is to choose another session; here the holder may be no pane at all and the remedy concerns a body. Pinned by Acceptance 20. Rejected: refusing with `pane-not-driven` (its meaning is "no body drives the pane, or more than one could"); reusing `session-of-other-pane` with another message (one code, two causes, two remedies); letting the switch proceed and re-pointing the second body (two bodies on one session).

**Decided by the operator after the third plan check (2026-10-12; final, not to be re-opened).**

- **P7. The hub marks the resolve-time `not_connected` with an additive `data.reason`; `NotDriven` stays.** `-32004 not_connected` has no `data` today and means five things (F11). The hub adds an optional `data.reason: "offline"` (the constant `OFFLINE_REASON` and the constructor `Error::offline(message)` in `crates/holler-proto/src/error.rs`, beside `Error::invalid_grant`, E163, E164) to the offline-body arm of `say`, `interrupt` and `answer` and to those arms alone; `hub_refusal::classify(&WireError)` (D5) maps `-32004` to `HubRefusal::NotDriven` only when that reason is present, and a bare `-32004` (`say`'s `Timeout` and `Cancelled`, `interrupt`'s `AckTimeout` and `PromptFailed`) falls through to today's arms: exit 1 with the hub's text. The field is additive and optional on an existing code, so the protocol version does not change; `docs/protocol/v2.md`'s `-32004` row says so, and one new golden file pins the frame (D5, Acceptance 11). An old hub that sends no reason makes a new CLI print today's text for an offline body, which is safe. Rejected: **dropping the offline case from `NotDriven`** (`NotDriven` would be `-32003` alone and a `--pane` to an offline body would keep today's text and exit 1, with P5's sentence amended); the operator chose the marked code, and the reasons for rejecting the other are not recorded.
- **P8. The exclusion rule has one statement, in D3.** The third plan check found the rule that removes a body tied to another pane stated two ways: the narrower one (by that pane's session of record alone) in D3, P4 and the verbatim section 11, and the wider one (also by its recorded DRIVEN) in Acceptance 20(b), D2 and question 5. The operator chose the wider rule, from the brief's own acceptance. D3 states it once; P4, D2, question 5, Acceptance 6 and 20 and the `driver_of` table refer to it as "the exclusion rule", and section 11 point 3 (the contract text F copies) carries the same words. Rejected: keeping the narrower rule and correcting Acceptance 20(b)'s last case, D2 and question 5 (that body would be `Unproven` for every other pane, and the `Unproven` remedy would lead the operator to pair it with the wrong pane).

**Decided, and what each rejected.** (All "decided by the preparer, to be confirmed at the plan check".)

**D1. The address: the advertised `harness_session_id`, joined to `Pane.session_of_record` by string equality.**
- The hub resolves a prompt target that is not a valid name as a harness session id, against the live attach sessions, for `say`, `interrupt` and `answer` alike: one new function `Registry::resolve_prompt_target(&str) -> ResolveOutcome` in `live/target.rs` (the name says what it resolves: `Registry::find_target` beside it is `hub query`'s lookup by token, client or label, E117, a different grammar). A target that parses as a `RoutableName` is resolved by `resolve_session`, exactly as before; any other string matches live ads with `mode == attach` and `harness_session_id == Some(target)`; no match is `Unknown`, or `NotConnected` when an offline ad had it; two matches are `Ambiguous`: the outcomes of E8. The three callers replace their two lines (parse, then `resolve_session`) with one call. Names and ids cannot collide where the grammar separates them: an OpenCode id is `ses_` and 26 characters (E96, the doc of `parse_session_id`), and `_` is not a name character (E39). An id that is also a valid name (the v2 document's example `openc-abc`, E67) is resolved as a name; it is not routable by id, and the ADR says so.
- **The CLI keeps the human form name-only (P3).** The id is an address at the hub, for `--pane` and the hub's gate, and not something a person types as a SESSION. `holler say ses_… TEXT`, `holler interrupt ses_…` and `holler answer ses_… CHOICE` stay refused as before (`unknown session: ses_…`, exit 1): `route_target_by` (E123; `route_target` calls it) checks a `PromptTarget::Session` with `RoutableName::parse` (the CLI already depends on `holler-proto`; E136) and stops with the hub's own text and exit code before any hub is contacted (with no hub running it now says `unknown session` rather than `no live holler hub reachable`; the input was never one the grammar allowed). Only `--pane` sends an id: `Routed` gains `pane: Option<String>` (the `--pane` name, `None` for a SESSION), which E126 lacks, so that the one refusal mapping of D5 can tell a `--pane` target from a typed one. `Routed` (E126) is built in three places (`prompt_target::route`, at line 74; `route_target_by`, E123; and `interrupt_cmd::route`, which builds its own after `interrupt_pane` answers `PaneInterrupt::Hub`, E169) and destructured in the three `run` functions; each sets `pane`. A client that speaks to the hub's control socket directly (`control/say` with an id; `admin/say` from an admin-role connection) is resolved like any other, because the id is an address at the hub; the human surface of the CLI does not change.
- The pane gate is keyed on the **advertised id of the session being prompted**, however the prompt addressed it: `SayOpts` and `LiveCommand::Say` carry `harness_session: Option<String>` (set in `send_turn` from `ad.harness_session_id`, E10; `send_turn` is the only place `SayOpts` is built, so plain, `--queue` and interrupt-redirect prompts are all keyed), `HoldGate` carries it, and `pane_gate(panes, id)` selects the panes whose `session_of_record == Some(id)` **or whose `last_observed.driven == Some(id)`** and judges each by `holler_pane::gate::prompt_refusal`; **the selection is one function of `holler_pane::gate`**, `first_refusal(panes: &[Pane], id: &str) -> Option<(&Pane, PromptRefusal)>` (the first bad pane in the slice's order, which is name order because the store lists name-sorted), which the hub's `pane_gate` and the test kit's `FakeHub` (D10) both call, so the selection exists once and `holler-pane/tests/gate_test.rs` and `conformance/hub.rs` pin its two keys and the order. The recorded-DRIVEN key closes the case the record key cannot see: a body that is **behind** (it still drives an earlier session of a pane, after a failed record write or after a restart reverted it, P2) advertises an id that no record names as a session of record, so a bare `say <name>` to it would be admitted and land in a session the pane's TUI does not show; once a pass of `doctor` or a verb has recorded that id as the pane's DRIVEN, the pair (SHOWN differs from DRIVEN) refuses it as `pane-shown-driven-mismatch`. The window that remains is stated in the ADR (section 11, point 3): a body that is behind and that no pass or verb has observed yet is not gated. A pane whose recorded DRIVEN is stale (the body has moved on) can refuse a session it no longer concerns; that is fail-closed, as the gate already is across panes that record one session. The name-keyed match of E2 is **removed**, not kept beside the new one. A bare `say alpha` to a body session that drives a parked pane's session of record is therefore refused too, which is what the pane hold's doc already claims ("whatever route the prompt takes", E68). `pane_gate` and `pane_state_refusal` leave `circuit/dispatch.rs` for a new `crates/holler-hub/src/panes/gate.rs` (`pub(crate) fn pane_gate`, which maps `first_refusal`'s answer to the `-32011` refusal and keeps the hub's own message text), because `talk::admit` (D5) must call the very function `send_prompt` calls and `dispatch` is private to `circuit`. **The tests that drive `send_prompt` do not move.** `dispatch.rs`'s `hold_tests` (E81, E82 and their neighbours) call `send_prompt` with the `HoldGate` and the `Recording` sink, which are private to `circuit` (E1, E170), and what they pin is the enforcement point: every variant, nothing on the socket, the hold refusing first. They stay in `circuit/dispatch.rs`, rewritten on `ses_` ids and the new key; `panes/gate_tests.rs` holds only the tests of `pane_gate` itself (Acceptance 4).
- One predicate, not three. `holler_pane::gate::prompt_refusal(&Pane) -> Option<PromptRefusal>` (`Parked`, `Unhealthy`, `ShownDrivenMismatch`, checked in that order, one side absent is not a mismatch) replaces the hub's copy (E1) and the CLI's (E22); each keeps its own message text, and the pair test it uses, `holler_pane::gate::shown_driven_pair_differs(&LastObserved) -> bool`, is public (#648 has merged and wrote that pair rule a third time in `roster_cmd.rs`, E93, E154; its body becomes a call to this function). The three code constants move to `holler_pane::gate` and `say_cmd` re-exports them (E20). `holler-pane` may change because this brief amends the contract first (the comments in E1 and E22 say it is frozen "except through the amend-first rule").
- Rejected: **(a) join on the pane's name** (a pane's name equal to its body session's name). Nothing on `main` establishes it: the wizard names attach sessions `alpha` and `beta`, independent of any pane (E88), and a pane's name is its tmux session's name (E94). It would make the join a naming rule the cutover must impose on a live fleet, and a miss would be silent. **(b) a new `Pane` field naming the body session**: a second source of truth beside the advertisement, a change of the stored record (`deny_unknown_fields`, the byte-compatibility test of E78), and `import` would have to fill it. **(c) a new wire request to ask a body which session it drives**: presence already carries it (ADR 0014 point 6, E51). **(d) keep the name match beside the id match**: two keys with different meanings, and the only inputs that match the old one are test fixtures. **(e) key the gate on the session of record alone**: a body that is behind is then not gated at all.

**D2. How a body ends up on the pane's new session: the hub asks it to re-point, with a new additive request `session/rebind`. I5 bound: 5 s in the task, 8 s at the hub, 10 s at the CLI.**
- Wire: `session/rebind`, a request, hub to body. Params `{session, harness_session_id}` (`session` is the body's bare session name); result `{session, harness_session_id}`. It is **additive**: the catalog goes from 22 to 23 rows, and `Rebind` and `RebindResult` get golden files that are new files (no existing golden moves).
- The body re-points **one attach-mode session of its own** to another harness session **on the same endpoint**, fail closed: a name that is not a session of this body is `-32003 unknown_session`; a spawn-mode session is `-32602 invalid_params` (`data.reason: "not an attach session"`); a session with a turn in flight, a queued prompt or a pending question is `-32009 session_busy` (with the existing `state`, `turn_age_ms` and `last_update_age_ms`); an id the endpoint does not have (the same `GET {endpoint}/session/{id}` existence check as `attach`, bare route then `/api`, E50; never `POST /session`) is `-32602 invalid_params` (`data.reason: "session not found on the endpoint"`) and the old binding is kept. On success the driver targets the new id, the session's presence advertises it, and the result returns it. The `hello` row type carries `harness_session_id` (E41), but the body's hello is sent with an empty `sessions` list (E110), so no hello names an id; presence is what the roster and the resolver read (E7, E40), so the rebind changes presence. The session's **name does not change** (ADR 0005 section 3, E52: the name is the identity, the locator changes; ADR 0014 point 5, E51).
- Bound (I5). The attach driver builds its HTTP client with `reqwest::Client::new()`, which sets no timeout (E97). The 5 s lives in the session task, not around the handler: the `Rebind` command carries a `deadline` (now + 5 s, set by the handler), the task does nothing with a command it dequeues after it, and wraps its `HttpAttachDriver::attach` in `tokio::time::timeout` for the time left, so a swap can only happen before the deadline; the handler awaits the reply for the deadline plus 1 s. On expiry the body answers `-32004 not_connected` with `data.reason: "endpoint did not answer"` (no catalog code means "the endpoint hung"; this is the one the v2 table already uses for "the party asked about cannot be reached", E70's neighbouring row) and **the old binding is kept, for certain**: an attach that finishes after the deadline is dropped, never swapped in (a 5 s timer around the handler alone would answer "kept" while the task went on to swap). The bounds nest strictly, so each layer sees the answer of the one below it: the task's deadline is 5 s; the body's handler waits the deadline plus 1 s (6 s); the hub's forward of the rebind, the wait for the slot included (below), is 8 s (`REBIND_FORWARD`); the CLI's `control/rebind` call (`ControlCall::rebind`) is 10 s (the one-shot `CLIENT_TIMEOUT` is 5 s, E140, and the hub's `query_remote` forward is 10 s, E98). With the hub's bound strictly below the CLI's, the CLI never expires first and reports `timeout` without having seen the hub's answer.
- **How the body swaps, and the one source of the attach id (P2).** `Inner.config.session_id` (the session task's own configuration, `session_manager/task.rs`) is the one source of the id a body attaches to. The `Rebind` command builds the new driver first with `HttpAttachDriver::attach` on a copy of that configuration with the new id (that is the existence check, E97) and, only when it has attached: writes the new id to `Inner.config.session_id`, swaps the driver in, mirrors the id into `SessionPresence`, notifies presence, and shuts the old driver down (`shutdown` ends only the body's own loop and touches nothing on the harness, ADR 0014 point 3, E51). A failed rebind writes nothing. Every later reader follows that one source:
  - the attach paths read `inner.config` and need no change of their own: `start_turn`'s re-attach at the next prompt after a turn that ended in error (E103; the driver is dropped when OpenCode sends `session.error`, E105, E104) and the attach retry, which runs while the driver is `None` (E101, E102);
  - presence reads a mirror: `SessionPresence` (E106) gains `harness_session_id: Option<String>`, initialised where the presence is built, in `SessionManager::start`, from the configuration (`harness_session_id: entry.config.session_id.clone()` in the `SessionPresence` the manager creates before it spawns the task, E143, E144), so an attach row advertises its id from the first moment, also in a current-thread test that reads `presence_doc` right after `start` before the task has run (without that the mirror's `None` would be read, and a `HoldGate` with no id skips the pane gate, D1); after `start` it is written only by the task, through one helper (`mirror_session_id(inner)`: in `attempt_initial_attach`, in `start_turn`'s re-attach and in the rebind swap; `omitted_pending_attach` stays as it is, set by `run` before the first attach attempt, because moving it would change when a row first appears and existing tests may pin that), and `SessionManager::presence_doc` reads it from there, never from `managed.config.session_id` (E46; `ManagedSession.config` stays the as-started configuration, E107);
  - `query/support`'s live probe (E108) probes the live id: `SessionManager::live_configs(&self, configs) -> Vec<SessionConfig>` returns the as-started configurations with each attach session's `session_id` taken from the mirror, and the body's `handle_query` call passes its result in place of `self.configs` (E111; the same number of lines);
  - `SessionRegistry::presence_doc` (E109) is the as-started baseline, built before any task exists; production presence is `SessionManager::presence_doc` (`connection.rs:869`) and the registry's version is called only by `config_test`. It keeps reading the configuration, and its doc says it is the configured baseline and not the live id.
- **A body restart reverts the binding (P2).** The rebind is not persisted: a restarted body attaches the `session_id` of its configuration again, so a pane whose body was re-pointed and then restarted has a body that drives an earlier session (a reboot or a deploy does this to the whole fleet at once, and #803's wizard runs one local body for every OpenCode pane, one attach row each, so one restart reverts every pane that body drives, E181). The consequences are specified and tested, none left to F: (1) `doctor` finds the reverted body when `driver_of` has positive evidence for it (D3): on a pane whose server lists only its own sessions the fallback finds it (its configured id is listed; no port deletes a session); on servers that share a data directory a reverted body's configured id is named by no pane's record or recorded DRIVEN, so the exclusion rule (D3) removes nothing, nothing proves whose it is, and it is `Unproven` and `doctor` reports it with no remedy and the operator pairs it with the two-step switch; once found, `doctor` records its id as the pane's DRIVEN and reports `shown-driven-mismatch`, `fix: not-fixable`, with the remedy `holler pane switch PANE <session of record>` (D3); (2) the gate refuses prompts to that body once its id is recorded as the pane's DRIVEN (D1: it is keyed on the recorded DRIVEN too), until the switch re-points it; (3) the remedy works when `driver_of` cannot tell (D3: the target rule pairs the body the operator names, then the second switch re-points it). The cutover guide says to run `pane doctor` after every body restart.
- Hub: the rebind rides its **own forward**, not `LiveHandle::query`'s (P1). The allowlist of `LiveHandle::query` and its comment are unchanged: `control/query_remote` and `admin/query` with a target hand any `params.method` to it, and the existing test `hold_hub_test::a_prompt_cannot_be_forwarded_around_the_hold` (E100) is the pattern that pins that a method outside the allowlist is refused `-32601`. `LiveHandle::rebind(params, timeout)` in the new `crates/holler-hub/src/live/target.rs` hands the literal method `"session/rebind"` to the private `forward` helper below, which builds a `LiveCommand::Query` on the same plumbing (so `circuit.rs` is not touched). **One request at a time per connection:** the circuit keeps a single `pending_query` slot and overwrites it (E116), shared by `session/answer`, `query/*` and now the rebind, so two overlapping requests drop one reply, which is then reported as not connected, and a rebind whose reply was dropped may still have re-pointed the body. `LiveHandle` therefore gains `query_slot: Arc<tokio::sync::Mutex<()>>` (E113, created where the handle is, E115). **One helper, one constructor, the slot inside the caller's timeout.** A private `LiveHandle::forward(&self, method, params, timeout)` in `live.rs`, placed immediately after `query`, is the only place a `LiveCommand::Query` is built: one `tokio::time::timeout(timeout, ..)` wraps the slot's lock, the send and the wait for the reply. `query` (after its allowlist check) and `rebind` (in `live/target.rs`, with the literal method) both call it, so each caller's own bound includes the wait for the slot: `query` returns `None` at its own `timeout` (`hub query`, `answer`), not one rebind later, and the send-and-wait code that `query` has today is moved into the helper, not copied. The helper sits after `query` in `live.rs` because the scan at `hold_single_path_test.rs:71` looks for the first `LiveCommand::Query` after `pub async fn query(` in that file and requires the allowlist check before it (E139): a helper in `target.rs` would leave the scan nothing to find. A new scan beside it pins the shape (Acceptance 13). The hub's rebind forward is 8 s, strictly below the CLI's 10 s call, and the body's reply wait is 6 s, below both (D2 "Bound"); a rebind that waited behind a slow query may be left with less than the body's 5 s, and then the hub answers "did not answer" while the body may still re-point (the residue below). The doc of `LiveHandle::rebind` says so. With #803's one body for the whole fleet the slot is fleet-wide: a rebind waits behind a query to any pane of that body, inside the same 8 s bound (E181). The residue that stays: a rebind that times out at the hub while the body is still re-pointing; `doctor` reports it and `pane switch` re-points the body. A new hub control method `control/rebind {token_id, session, harness_session_id}` (`token_id`, as `control/token_ping` and `control/revoke` name an exact token and as the roster row does, E182, E18; `token` alone is `control/test_drop`'s loose lookup) in a **new file** `crates/holler-hub/src/control_rebind.rs` (the precedent is `control_hold.rs`) resolves the body by `token_id` (a roster field, E18) with `Registry::find_by_token` and checks that the bare session name is in that handle's cached presence (E114), calls `LiveHandle::rebind` with an 8 s bound (`REBIND_FORWARD`; the CLI's `ControlCall::rebind` waits 10 s), passes a body refusal through verbatim, and **on success updates the cache and the roster before it replies**: `Registry::replace_session` for the one row, then `Roster::advertise(token, &Presence { hostname, sessions })` with the handle's **whole** cached session list after that replacement, because `advertise` replaces the token's whole set of rows and a row missing from the frame goes `gone` at once (E112; a one-row frame would drop the body's other sessions). So a `control/roster` or a `resolve_prompt_target` made right after the reply already sees the new id (nothing waits for the next presence beat). `control/rebind` is **not** in `dispatch_allowlisted`: it is a local control-socket method only, and there is no `admin/rebind` (question 4). The dispatch arm is one line in `dispatch_session_control` (E13's file). On the body, `dispatch_session_request` (E49) gets a fourth arm and its caller's `matches!` list (E48) a fourth name; the session task's mailbox (E47) gets a `Rebind` command beside `Answer`.
- Scope limit, stated in the ADR: only the HTTP attach driver implements it. ACP and spawn sessions refuse. A body for a Claude Code pane (#850) implements the same request in its own driver kind (a relaunch of the process); nothing here assumes OpenCode beyond the existence check inside the HTTP attach driver.
- Rejected: **(a) restart the body with a new config**: not bounded by I5, needs the CLI to supervise another process and rewrite a file the wizard owns (E88), and ADR 0014 point 3 says a body must be able to leave without touching the harness (E51). **(b) a new body session per `ses_`**: a new name is a new hold key, so `--join-held` would default-hold every reset (E17) and operator holds would not carry over; the roster would grow by a row per reset; and ADR 0005 section 3 says the name is the stable identity (E52). **(c) the hub or the CLI posts the prompt to the pane's OpenCode server itself**: that is an adapter in the hub (ruling 1, section 5 of the ADR, E56), it skips the body's turn tracking, and it is a second way to deliver a prompt, which the single-sender guard exists to prevent (E77). **(d) leave it to the cutover**: the story's own acceptance (a healthy pane's `reset --first` is delivered) cannot hold in production, and after every switch the body would still drive the old session, so the gate would refuse everything as `pane-shown-driven-mismatch`. **(e) hot reload of the body's config file**: the body has no reload path and the file would become a control channel. **(f) allowlist `session/rebind` on `LiveHandle::query`**, **(g) a body-side "swapped state" separate from the configuration**, **(h) persist the binding across a restart**: see P1 and P2.

**D3. Who records `last_observed.driven`, and where in the step order: the verb and reconcile record it from what the body acknowledged or advertised; the hub records nothing.**
- A new port, `HubPort` (`holler-pane`, new file `crates/holler-pane/src/hub.rs`), with three **required** methods (no defaults: `HostPort::foreground_commands` and, since #835, `HerdrPort::observe_pane` are required on `main` for exactly the reason that a default answers silently for a wrapper that forgot to forward, E92, E185) and `Ports` gains `pub hub: &'a dyn HubPort` (E29; `Wiring::ports` lends the real ports, E25, and `Unwired` is the not-implemented set, E26; `PaneStore`, E27, gets no new method):
  ```rust
  pub struct Advertised { pub token: String, pub name: String /* "label/session" */, pub harness_session_id: String, pub state: Activity /* holler_pane::ports::Activity */ }
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
- `Advertised.state` is the existing `holler_pane::ports::Activity { Idle, Busy, HoldingQuestion, WaitingPermission }` (E118; four variants since #831), not a new enum: the wiring maps the roster's `working` and `stalled` to `Busy`, `input-required` to `HoldingQuestion`, `idle` to `Idle`, and any other string to `Busy` (fail closed). One function turns an `Activity` into its refusal, `activity_refusal(Activity) -> Option<(RefusalCode, &'static str)>` in `tx_switch.rs` (a total match over the four variants: `Idle` none, `Busy` `session-busy`, `HoldingQuestion` `session-holds-question`, `WaitingPermission` `session-holds-permission`; the wiring never produces the last, because the roster's `input-required` covers a question or a permission), and both `check_idle` (E119, where the match sits today) and the new driver check call it.
- The join the engine and reconcile use, a pure function in a new file `crates/holler-pane/src/driven.rs`: `driver_of(pane, advertised, server_sessions, others, target) -> Driver<'_>` (`None`, `One(&Advertised)`, `Ambiguous(Vec<&Advertised>)`, `Unproven(Vec<&Advertised>)`; `target: Option<&str>` is the session a switch is moving to, `None` for a reset and for reconcile; `others` are the other panes' records, as read at the start of the plan or of the reconcile pass and never the values that pass writes, so the result does not depend on pane order). **A body is accepted only on positive evidence (P4).**

  **The exclusion rule.** A body that advertises another pane's session of record, or another pane's recorded DRIVEN, is that pane's and is never a candidate for any rule below, the fallback included. The exception is a body that also advertises this pane's own session of record or recorded DRIVEN: this pane's evidence is checked first, so a stale DRIVEN on another pane never takes a pane's own body. One consequence the verbs state: `pane switch P1 <the id of a body removed for P1>` is refused `session-driven-elsewhere` (P6), because that body advertises the target and is not P1's driver, so the `Unproven` remedy can never pair a body that another pane's evidence proves. These words are copied into section 11 point 3 below, which is the contract text F copies; everywhere else this brief says "the exclusion rule, D3" and does not restate it.

  Of the bodies that remain, the first of these rules that matches decides: **(i)** the bodies that advertise the pane's **session of record**; **(ii)** those that advertise the pane's stored **`last_observed.driven`** (the record's own memory: it finds a body whose session the pane's server no longer lists, as after a relaunch onto a new session, E129, and a body that is behind); **(iii)** for a switch, those that advertise the **`target`** (it already drives where the pane is going, so it is the driver and needs no rebind). A rule that matches exactly one body gives `One`; two or more give `Ambiguous` (two bodies on one session of record, one stored DRIVEN or one target). When no rule matches: **(iv)** if the pane's server lists **no other pane's session of record** (`server_sessions` and the sessions of record of `others` do not intersect: the shared-listing signal of `check_unclaimed`, E137), the advertised ids that the server lists are the candidates: exactly one is `One` (a body that reverted on a pane that shares nothing is still found), two or more are `Ambiguous`, none is `None`; **(v)** if the listing is shared, the listed bodies are `Unproven` (none listed: `None`): they may or may not be this pane's and nothing says which. `Ambiguous` and `Unproven` are refused alike in the plan (next bullet). Why evidence and not membership in the server's list: servers that share a data directory list every pane's sessions (E137), so a pane with no connected body would otherwise claim another pane's behind body and re-point it silently, and a relaunch can leave a body on a session that the restarted server no longer lists (E129), which membership never finds. Why (ii) after (i): after a switch or a reset the record equals what the body advertises and (i) decides; (ii) is for the bodies that are behind. A second pure function in the same file, `elsewhere(advertised, driver, target) -> Vec<&Advertised>`, returns the bodies other than the driver that advertise `target` (P6); it looks at every live attach body, the ones the exclusion rule removed included.
- **Ambiguous drivers (P2: the remedy must work).** `Ambiguous` and `Unproven` are refused `pane-not-driven` in the plan, before anything moves, for switch and reset alike. The `Ambiguous` message names every candidate as `label/session` with the id it advertises, and gives the manual step: `two bodies could drive PANE: label1/oc (ses_…a), label2/oc (ses_…b); switch PANE to the session its own body drives (holler pane switch PANE ses_…), then holler pane switch PANE <session of record>`. The two steps work because of the target rule: `holler pane switch PANE <the id one candidate advertises>` makes that body the driver (no rebind; the record, SHOWN and DRIVEN become that id in one write), after which the record is an advertised id and the plain rule picks the body; the second switch then re-points it to the session of record. A reset has no such target, so a reset on an ambiguous pane is refused the same way, and the message names the switch. The `Unproven` message reads `no body is known to drive PANE; a listed body might be its own: label1/oc (ses_…a), … (switch PANE to the session its own body drives (holler pane switch PANE ses_…), then holler pane switch PANE <session of record>; if PANE has no body, start one first)` and lists the candidates the same way; the first switch pairs the body by rule (iii), and the second finds it by rule (i). **Advertised text is untrusted.** `SessionAd.harness_session_id` is an unvalidated `Option<String>` on the wire (E40), so every advertised name and id a message prints goes through `findings::quoted` (as `check_unclaimed` quotes its target, E137), and the command in these messages is built, never spelled: `switch_command(pane, session)` (below) is called only for an id that passes `parse_session_id` (E96); for any other id the message names the body and says its id is not a valid session id, and the finding's `remedy` is `None`. A pane that truly has no body, on a listing shared with a behind body that no pane's record or recorded DRIVEN names (the exclusion rule, D3, removes the others), gets the same text until that body is paired or restarted (question 5). Choosing the id of the wrong body pairs the pane with that body, visibly (the record, SHOWN and DRIVEN all show the id) and undoably (switch again); nothing else about the pane changes. Pinned by `driver_of`'s table and by the verbs' tests (Acceptance 20). Rejected: choosing among the candidates by order or by guess (a wrong pairing, silently); a flag to name the body (D12: no flag, `cli-surface.txt` stays); trying `rebind` on each candidate (servers that share a data directory all have the target, so every rebind would succeed).
- The real `HubPort` is `crates/holler-cli/src/pane/wiring/hub.rs`: `advertised` reads `control/roster` (E19; keep rows whose `mode` is `attach`, `conn_state` is `connected` and `harness_session_id` is present; the row's `state` maps to `Activity`); `rebind` calls `control/rebind` (D2); `prompt` calls `control/say` (D5). A refused exchange (`ControlError::Refused`) is classified by `hub_refusal` (D5, one function) and mapped by the method that made the call; every other failure goes through the mapping that `wiring/store.rs` already holds, not a second one: `Effect`, `timed_out`, `hub_unavailable` and `undecodable` are made `pub(super)` (E145, E146; `call` itself decodes the `pane/*` reply envelope, which `control/*` replies do not use, so `wiring/hub.rs` has a short send of its own that calls `crate::transport::call` and maps its error through those three). `rebind` and `prompt(Send)` are writes (`Effect::Write`: a reply lost in flight may have landed, so it is a `timeout`); `advertised` and `prompt(Check)` are reads. The `op` a `timeout` names is `hub.advertised`, `hub.rebind` or `hub.prompt`. `Unwired` answers `not-implemented`; the test kit gets `FakeHub` (D10).
- **Switch and reset, the new order** (the engine, `tx_switch`; `Check` and `Send` are `PromptMode`):
  1. Plan (nothing written or moved; for a single-process pane see item 8): the existing refusals; then `hub.advertised()`, `harness.list_sessions` and `pane_store.list` give `driver_of`. `Ambiguous` and `Unproven` are refused `pane-not-driven` (the message names the candidates and the manual step; see the "Ambiguous drivers" bullet). **For a switch**, a live attach body other than the driver that advertises the target is refused `session-driven-elsewhere` (P6), naming it as `label/session` with its id, both quoted with `findings::quoted` (advertised text is untrusted, see "Ambiguous drivers"): nothing moves. A driver whose `state` is `Busy` is refused `session-busy`, `HoldingQuestion` `session-holds-question` (existing open codes; `WaitingPermission`, which the wiring never produces, `session-holds-permission`; all from the one `activity_refusal` function that `check_idle` also calls) for switch and reset alike: the body would refuse the rebind, and the refusal must come before the TUI moves. With `--first`: unless a driver exists **and the id it advertises is the pane's session of record** (the gate joins by that id, so it can judge no other), the run is refused `pane-not-driven`; otherwise `hub.prompt(session of record, text, Check)` (D5).
  2. Act: `create_session` (reset), `select_session`.
  3. Observe: `shown_session` (unchanged: a mismatch is `unavailable`, nothing recorded).
  4. **Drive**: with a driver, `hub.rebind(driver, target)`; the acknowledged id must equal the target, else `unavailable`. A driver that already advertises the target (the idempotent re-run of the remedy, and the target rule of `driver_of`) is not asked: its advertised id is the acknowledgement. A failure here is a failure after the act: `acted: true`, the message ends with the reconcile step (`doctor --fix` selects the old session again, which is the state the unmoved body still matches).
  5. Record, one compare-and-swap at the generation the plan read (`switch_in_place`'s `cas_put`, E31): the four fields of `recorded` (E32) **and `last_observed.driven`** = the acknowledged id, or `None` when no body drives the pane (observed by step 1, not inferred; this overwrites a stale stored value).
  6. First message, `--first` only: `hub.prompt(target, text, Send)`, in place of the `send_prompt` call of `tx_switch::prompt` (E158). A refusal or failure here is the existing `unprompted` failure (the record stands; E31's tail; the message says the session was created, shown and recorded, but the first message did not land), with the refusal's code.
  7. An in-place run now makes at most **thirteen** port calls, each bounded by I5 (E30 said eight).
  8. **A single-process pane** (`Shape::SingleProcess`, a Claude Code pane, #831; E177) takes its act, observation and record from `tx_launch::relaunch_recorded`, not from steps 2 to 5 (E31, E178). The plan asks the hub nothing for it and takes the driver as `Driver::None`: no Claude body exists until #850 (D6) and such a pane has no server list. The relaunch stores the record itself and keeps its stored DRIVEN (E128); there is no drive step and no DRIVEN write. With `--first` the plan therefore refuses `pane-not-driven` before anything is stopped (D6). Steps 1 to 7 above describe the in-place path (`switch_in_place`); only its record write gains `last_observed.driven`.
- Reconcile (`pane doctor`): reads `hub.advertised()` **once per pass**, as it reads `herdr.snapshot` (E34), computes `driver_of` for each pane from the server list it already observed and the other panes' records as the pass read them (never the DRIVEN values it writes), and records `last_observed.driven` with `shown` in the same write. A single-process pane (#831) is skipped by this step: it has no server list and no body until #850, and its `last_observed.driven` stays as stored (E176). A failed `advertised()` is an `observe-failed` finding for `hub.advertised` and leaves `driven` as stored (the rule for every failed read, E34). `shown-driven-mismatch` keeps its test, SHOWN against the session of record (E35, E36), and **also** reports a pane whose observed DRIVEN is `Some(d)` with `d` equal to neither SHOWN nor the session of record. (The kind's doc, E37, says it is the one kind `--fix` repairs; that stays true of the TUI side.) `--fix` repairs only the TUI side (it selects the session of record). A pane whose TUI is right and whose body is behind is `fix: not-fixable` and gets **no new `FixState`** (a state's name is its JSON form, E120, and `--json` does not change): reconcile sets the finding's own `remedy` field (E121) to `holler pane switch PANE <session of record>`, which is idempotent and re-points the body. That command is **built, not spelled, and set after `with_fix`**: `findings.rs` gains `switch_command(pane, session)` beside the other remedy builders (`doctor_command`, `relaunch_command`, `reset_command`, E142), and two `Finding` methods, `with_switch_remedy(self, session)` and `without_remedy(self)`, which reconcile calls **after** `with_fix` (`switch_command` answers `Option<String>`: `None` for an id that fails `parse_session_id`, E96, and `with_switch_remedy` then sets no remedy), because `with_fix` recomputes `remedy` from the one table (`FindingKind::remedy`, E141) and would overwrite a remedy set earlier with the table's `relaunch` for a not-fixable mismatch. The doc of `remedy` is amended to say that the table answers by kind and fix alone and that reconcile sets these two cases itself (a body that is behind: the switch command; a body that cannot be told: none). E38's table is unchanged: it answers every other not-fixable mismatch. A pane whose driver is `Ambiguous` or `Unproven` records DRIVEN `None` (nothing is observed) and is reported as `shown-driven-mismatch`, `not-fixable`, `remedy: None` through `without_remedy` (no one verb repairs it unattended), with the message of the refusal.
- Rejected: **(a) the hub writes or reports DRIVEN into the record**: the hub is the store only (ruling 1) and `last_observed` is "written by reconcile, never inferred" (I6, E33). **(b) record `driven = session_of_record` without observing it**: that is inference. **(c) drive before `select_session`**: a failed select would leave the body moved and nothing in `doctor --fix` could move it back; driving after the observation leaves the state `--fix` already restores. **(d) a `driven` field on the pane store's wire or a new `pane/*` method**: `PANE_METHODS` is frozen (E58) and the roster already carries every fact (E18).
- Known residue, documented: if the compare-and-swap fails after a successful rebind (a lost race or a `timeout`), the body is ahead of the record. `doctor` reports it as `shown-driven-mismatch`, and `holler pane switch PANE <session of record>` re-points the body back. No step is compensated (the engine's rule, E30). A **body restart** (P2) and a **rebind whose reply the hub never saw** (the slot residue of D2) reach the same state and have the same report and remedy. A **relaunch** onto a new session (the restarted server no longer lists the old one, E129) leaves the body on the old session, and `launch` and `relaunch` keep the stored DRIVEN and never infer it (E128). The body is found by rule (ii) of `driver_of`: it advertises the stored DRIVEN, listed or not. `doctor` reports `shown-driven-mismatch` (DRIVEN the old session, SHOWN the new one) with the remedy `holler pane switch PANE <session of record>`, and the switch re-points it (Acceptance 6). The window: a pane whose DRIVEN was never recorded (no verb and no `doctor` pass observed its body before the relaunch) has no evidence, and its server no longer lists the old id, so its body is not found: nothing is reported for it and the switch re-points nothing; the body must be restarted with the new session in its configuration (the guide says to run `pane doctor` before any relaunch and after every body restart). And the window the gate cannot close: a body that is behind and that no `doctor` pass or verb has observed yet is not gated (D1).

**D4. Hold keys: the key `<label>/<session>` is the body session's and does not change when it is re-pointed.**
- An operator hold or a default hold on a pane's body session therefore **carries across a switch or reset**, and `--join-held` (which holds a key on first sight, E17) does not fire again after a rebind, because no new name appears. `reset --first` onto a held pane is refused before anything moves (D5) with the new open code `session-held` (exit 3; message: the hold kind, the reason, and `holler release <label>/<session>`); `release` lifts it as it lifts any hold (the pane hold of `hold_kind: "pane"` is not lifted by `release`, E68, and keeps its own remedy). That mapping is `reset --first`'s alone (P5): `say`, `interrupt` and `answer` with `--pane` keep the held refusal and exit 4 for the same hold. A `rebind` is not refused by a hold: a hold stops prompts, not re-pointing.
- Pinned by tests (Acceptance 12): an operator hold set before a reset still refuses `--first` after a previous reset; a hub started with `--join-held` that held the body's name on join refuses `--first`, and after `release` the next `reset --first` is delivered; a rebind makes no `session_held` event.
- Rejected: a hold keyed on the harness session id (a new key space; an operator holds a session by the name `holler roster` shows, E18).

**D5. `reset --first`: checked before the switch, sent after it; the hub's `send_prompt` stays the only authority.**
- The plan step 1 above sends `hub.prompt(id, text, Check)`: the hub resolves the id, runs `holds.would_refuse` and the pane gate (the same functions `send_prompt` calls, E16, E2; `pane_gate` is reachable from `talk::admit` because D1 moves it to `panes/gate.rs`) and **sends nothing** (no frame to the body, no TalkLog line, no turn id moved, no grant consumed). A refusal changes nothing: no session is created, SHOWN, DRIVEN and the session of record are unchanged. This is the advisory fast path `talk::say` already has for holds (E9: "a fast path only"), not a second enforcement site: it calls the same functions, and the authority that cannot be raced is still `send_prompt`.
- **`talk::admit` is `say`'s prelude, extracted, not a second copy (E138).** `admit(registry, roster, session, queue, grant) -> Result<(LiveHandle, SessionAd), SayError>` is what `say` runs before `send_turn` today: the parse and resolve (now `resolve_prompt_target`), the hold fast path (`holds().would_refuse`) and the busy check, moved as they are; it adds the pane gate (`panes::gate::pane_gate` on the ad's id, the very function `send_prompt` calls) and returns the resolved pair. `say` calls `admit` and then `send_turn`; the `check_only` branch calls `admit` and stops. So `say` also refuses a pane-gated prompt before it writes a TalkLog line or moves the turn id (a fast path, as the hold's is; `send_prompt` stays the authority). `check_only` runs the whole of `admit`, the busy check included, with the `queue` it was given; the wiring passes `queue: true` for `Check` and `Send` alike, so only a held question refuses it (the plan has already refused a busy or questioning driver, so the check catches a race). Acceptance 10 pins that `say` and `check_only` refuse the same cases.
- `Check` is the new optional parameter `check_only: bool` of `control/say` (and so of `admin/say`, which shares the handler, E13): additive, absent means a normal send; documented in `docs/protocol/v2.md`.
- `check_only` is read where `control/say`'s other params are (E14). `Send` is `control/say` with `queue: true`, no grant, and the CLI's accept-and-return rule, which exists **once**: `wiring/hub.rs` builds the call as `say_cmd::run` builds one (E168): `ControlCall::say_with(id, text, true, None, SEND_TURN_BUDGET)`, where `SEND_TURN_BUDGET` is the turn budget, 600 s, the default of `say --timeout`, so that the hub's `timeout_ms` is the turn's; it then overrides **only `call.timeout`** with `say_cmd::QUEUE_ACCEPT_WAIT` (2 s) and passes the reply through `say_cmd::wait_ended` (made `pub(crate)`), whose read deadline means accepted (E23), never the turn's reply, so the verb is bounded (I5). Building the call with the 2 s wait as its argument would make the hub time the turn out at 2 s, racing the CLI's own deadline, and a delivered first message could be reported as failed (F11); built as above, the hub keeps waiting for the turn after the CLI has returned, as it does for `say --pane --queue`. `wiring/hub.rs` writes no accept rule of its own.
- **One mapping each, reused.** (a) The hub's refusal is **classified once**, in a new file `crates/holler-cli/src/pane/hub_refusal.rs`: `hub_refusal::classify(&WireError) -> HubRefusal`, where `HubRefusal` is `NotDriven(String)` (`-32003` `unknown session` or `ambiguous` for the pane's id, or `-32004 not_connected` **with `data.reason: "offline"`** (P7); the hub's text; a `-32004` without that reason is `Other`), `PaneGate { code: String, message: String }` (`session_held` with `hold_kind: "pane"`: `pane-parked`, `pane-unhealthy` or `pane-shown-driven-mismatch`; the code is a well-formed one, as `PaneError::Refused` requires), `Held { kind, reason, key }` (`session_held` with `hold_kind` `operator` or `default`), `Busy(String)` (`session_busy`) and `Other` (anything else, `invalid_grant` included). It decides no code and no exit; **each caller maps what it needs (P5)**. `wiring/hub.rs`'s `prompt` (`Check` and `Send`, reached by `reset --first` alone) maps `NotDriven` to `pane-not-driven`, `PaneGate` to `Refused` with the same code and the hub's message, `Held` to `session-held`, `Busy` to `session-busy`, and `Other` through the `store.rs` mapping. `wiring/hub.rs`'s `rebind` treats every refusal as `unavailable` carrying the body's reason (it comes after the act). The `--pane` arms of `say_cmd`, `interrupt_cmd` and `answer_cmd` (E124, E125; each when `Routed.pane` is `Some`) act on `NotDriven` alone, as `pane-not-driven` (exit 3), and leave every other variant to the arms they have today: `pane_hold_refusal` for a pane hold, the held arm with exit 4 for an operator or default hold, then the generic arms (E149). The `NotDriven` arm goes first in `say_cmd::control_refusal` (E149) and in `interrupt_cmd::run` (E124), ahead of today's `is_ambiguous` test, which exits 2 for an ambiguous session; `answer_cmd` has no held arm (E125). A bare `-32004` reaches the generic arms and prints the hub's text with exit 1 (`say`'s "no reply from … within Ns", `interrupt`'s "interrupt sent but not confirmed within Ns"). A typed SESSION keeps today's `unknown session`, exit 1. (b) On the hub, `control/say`'s `SayError` arms are extracted from `control_server::say` (E122) into one function in a new file `crates/holler-hub/src/control_say.rs`, and the `check_only` branch maps the `talk::admit` error through the same function. That function's `NotConnected` arm (the resolve-time one) builds `WireError::offline(detail)` (P7, E163) and its `Timeout` and `Cancelled` arms stay bare `-32004`; the offline arms of `control/interrupt` and `control/answer` stay in `control_server.rs` and build `WireError::offline(..)` the same way, while `AckTimeout` and `PromptFailed` stay bare (E160, E161, E162).
- If `Check` admitted and `Send` is refused (a race: a park, hold or health change in between), the reset has completed: the existing `unprompted` failure applies (the record stands, exit 3 with the refusal's code, the message says the first message did not land). That residue is accepted and documented.
- Rejected: **(a) roll back the switch on a refusal**: no port deletes a session, so a reset's new session would stay behind as a stray and "unchanged" would be false; undoing the TUI is a second act with its own failure modes; and the engine compensates nothing after the act (E30). **(b) no pre-check, refuse only after the record**: the issue's acceptance says SHOWN, DRIVEN and the session of record stay unchanged. **(c) a plan-time predicate in the engine without the hub** (the A review's finding 3, option iii): a third copy of the pane-state predicate and no view of holds.
- `HarnessPort::send_prompt` stays in the trait with its default (E28; the OpenCode and Claude adapters implement it, #649's pass-through test and the test kit's fakes use it) but **no verb and no engine calls it any more**. A source-scan test (like E77 and E95) enforces it, defined precisely: in `holler-pane/src` and `holler-cli/src`, with comments stripped and each file cut at its first `#[cfg(test)]` as `code_of` does (E171), a `.send_prompt(` call is allowed only inside the body of a `fn send_prompt` that is a wrapper's pass-through, and there are exactly two such calls: in `crates/holler-cli/src/pane/wiring/bind.rs` (`impl HarnessPort for Wired<dyn HarnessPort>`, E127) and in `crates/holler-pane/src/ports/router.rs` (`HarnessRouter`, #831: `self.by(at).send_prompt(at, session, text)`, E159). #649's rule is that a wrapper forwards every method, so F cannot remove either. The test finds the enclosing `fn` of every call, fails on a call inside any other function or file, and asserts that each of the two files has exactly one; the test kit's fakes are outside the two scanned directories. `tx_switch.rs:295` (E158, in `fn prompt`) is the call F removes. Retiring the method is a follow-up (finding 8's other option).

**D6. A pane no body drives, and a harness that is not OpenCode.**
- DRIVEN means "a live attach-mode body advertises a session the pane's server lists". On `main` only an OpenCode attach body can advertise one. For any pane with no such body (`Driver::None`): `last_observed.driven` is `None`; switch and reset work as before without the drive step; `say --pane` and `interrupt --pane` and `answer --pane` answer `pane-not-driven` (exit 3) instead of today's `unknown session` (exit 1); and **`reset --first` is refused `pane-not-driven` before anything moves**, as it is for a pane whose body is behind its session of record or whose record names no session (`pane reset PANE` without `--first` heals either, and `--first` can follow as a `say --pane`). Today `reset --first` works through the adapter; this is the behaviour change the one-gate rule costs, recorded in the CHANGELOG, and it is the operator's question 3.
- A Claude Code pane (#831, merged) has the `SingleProcess` shape, no OpenCode `ses_` session and, on `main`, no body (#850 builds the driver). The join is a string comparison of the advertised id with the session of record, so a Claude UUID joins the same way once a body advertises it, and `session/rebind` is that driver's own request to implement (a relaunch). **Nothing in this story builds, fakes or assumes a Claude body.** Until #850 a Claude pane is `Driver::None`: `reset --first` on it is refused `pane-not-driven`, which changes #831's merged test `reset_first_reaches_the_new_process` (E91, it becomes "refused, nothing moved") and two passages of its ADR text (section 8's single-process paragraph, E178, and item 18, E89); the ADR table says how.

**D7. The new and changed refusal codes (ADR-0021 section 9), one mapping.** Declared with `RefusalCode::from_static` in the file that owns each (a verb declares the codes it owns, `error.rs`): `pane-not-driven` and `session-held`, the port's answers, in `crates/holler-pane/src/hub.rs`; `session-driven-elsewhere`, which only the switch's plan raises (P6), in `crates/holler-pane/src/tx_switch.rs` beside `SESSION_OF_OTHER_PANE` (E153); all exit 3, class refusal:

| What the hub or the plan answers | Pane code (exit) |
|---|---|
| `-32011 session_held`, `hold_kind: "pane"`, `reason` = `pane-parked`, `pane-unhealthy` or `pane-shown-driven-mismatch` | `reset --first`: the same code (3), the hub's message. The three `--pane` verbs: unchanged, `say_cmd::pane_hold_refusal` (E24) (P5) |
| `-32011 session_held`, `hold_kind` `operator` or `default` | `reset --first`: `session-held` (3), **new**. The three `--pane` verbs: unchanged, the held refusal, exit 4 (P5) |
| `-32003` `unknown session` or `ambiguous` for the pane's id, or `-32004 not_connected` with `data.reason: "offline"` (P7) (the three verbs and `reset --first`); no driver in the plan; `Ambiguous` or `Unproven` drivers | `pane-not-driven` (3), **new** |
| `-32004 not_connected` without the reason: `say`'s "no reply from … within Ns" and "prompt was interrupted", `interrupt`'s "interrupt sent but not confirmed" and its failed redirect | the three `--pane` verbs: unchanged, exit 1 with the hub's text (P7); `reset --first`: `Other`, so `unavailable` (1) |
| `-32009 session_busy`; a driver whose `Activity` is `Busy`, `HoldingQuestion` or `WaitingPermission` | `session-busy`, `session-holds-question`, `session-holds-permission` (3, existing, #645 and #831) |
| a switch whose target a live body other than the driver advertises (the plan, P6) | `session-driven-elsewhere` (3), **new** |
| a refused `rebind`: `-32003 unknown_session`, `-32004 not_connected`, `-32602 invalid_params` | `unavailable` (1), carrying the body's reason (the rebind comes after the act, so it is a failure, not a refusal) |
| `-32012 invalid_grant` (cannot happen: no grant is sent) | `unavailable` (1) |
| the accept wait's read deadline on `Send` | accepted (not an error) |
| any other transport or wire failure | `unavailable` (1) or `timeout` (1), as `ControlError` maps today |

`prompt-unsupported` leaves the `pane reset` row of section 9 (only the `HarnessPort` default raised it). No code goes into `holler_proto::Code`, and no `pane` verb exits 4 (the routed `say`, `interrupt` and `answer` keep their exit 4 for an operator or default hold, P5).

**D8. Fixtures are `ses_`-shaped, and a test proves the shape.** Every fixture of a hub-routed case uses `ses_` and 26 characters (E78, E79, E80): the test kit gains `holler_pane_testkit::fixture::session_id(n: u32) -> String` = `format!("ses_{n:026x}")` (the format `FakeHarness` mints, E80), and one test asserts that it has 30 characters and that `RoutableName::parse` refuses it, so a fixture cannot pass by being a valid name. The test lives in `crates/holler-cli/tests/pane_verbs/driven.rs`, which already links both `holler-pane-testkit` and `holler-proto`: the testkit depends only on `holler-pane` and `serde_json` (E135), and a `holler-proto` dev-dependency there would be a manifest change for one assertion. The #646 gate tests (E81, E82) are rewritten onto it. A test that passes only on `ses-demo-*` names does not satisfy the story.

**D9. Where the opt-in scenario changes (#649's `reset_with_a_first_message`, E86).** The scenario has a hub and no body, so a `reset --first` there is now refused. Step 3 becomes: `reset C2 --first "…"` exits 3 with `pane-not-driven` and the record is byte-for-byte unchanged (no session created); then `reset C2` without `--first` makes the fresh session the later steps use. The "its session reads Busy" assertion moves to the default-run end-to-end test (Acceptance 3), which has a real body. The scenario starts no body (that would need a hub join and a token flow in the scratch rig; it is a possible follow-up, not this story). #667 has merged: the function is at line 144, reads `scratch.session_of_record(C2)` and `scratch.record(C2)` (E86) and is called at line 98 (E155), and D9's step is applied to that text.

**D10. The test kit.** `FakeHub` (new file `crates/holler-pane-testkit/src/hub.rs`) implements `HubPort`: scripted bodies (`with_body(token, name, harness_session_id, state)`), `rebind` updates the matching body's id (or answers a scripted fault), `prompt(Check|Send)` calls `holler_pane::gate::first_refusal` (D1: the very selection the hub's `pane_gate` calls, so a pane's recorded DRIVEN and two panes that record one id are judged as the hub judges them) over the panes of the `FakePaneStore` it was built over, plus scripted holds (`hold(name, kind)`), its faults and its call log from `fault::FaultSwitch<HubOp>` (E147; `HubOp` is `Advertised`, `Rebind`, `Prompt`, with `as_str` `hub.advertised`, `hub.rebind`, `hub.prompt`, and `faults()`), not a hand-rolled log, and the `Send` log (`prompts()`). For Acceptance 2's order across two fakes each `Rebind` and `Prompt` entry also records whether a pane in the store has the call's id as its session of record at that moment, as `SeamHarness` records `then_of_record` (E148). A conformance function (new file `conformance/hub.rs`) runs against `FakeHub` and, in the default-run end-to-end test, against the real client over a real hub; it pins the selection's two keys (the session of record, the recorded DRIVEN) and its order (the first bad pane in name order is the one named).

**D11. Size rules.** `scripts/lint.sh` fails at 900. "Where the code goes" lists the files and what each may grow by (sizes re-measured on `main` at `4e691d9`; only `ports.rs` differs from `b76f4d2`); `circuit.rs` takes **no** growth, `docs.rs` (899) and `roster.rs` (892) take none, `live.rs` about +13 (`mod target;`, the `query_slot` field, its initialiser, the `forward` helper, `query` shorter by its moved send-and-wait, the two `Say` fields: 897 at most), and so first loses its inline `mod tests` (lines 812-884) to `live/tests.rs`, a pure move, `control_server.rs` and `circuit/dispatch.rs` end smaller than they are, `codec_test.rs` (889) gains one arm (+1), and no file crosses about 850 by this story. The test files that are already near the limit (`pane_verbs/switch.rs` 893, `support/mod.rs` 857, `launch/rig.rs` 852) take at most one line each. New tests go in new files, and each `#[cfg(test)] mod …;` line this story adds is the last line of its file (the hub's precedent, E172, E173; `code_of` cuts a file at its first `#[cfg(test)]`, E171).

**D12. What this story does not do** (named so a test can pin the edge): it builds no Claude Code body (#850); it adds no `doctor --fix` that re-points a body (the remedy is `pane switch`); it does not retire `HarnessPort::send_prompt`; it adds no CLI flag (`cli-surface.txt` is unchanged); it starts no body in the opt-in scenario; it does not change `pane list`'s columns (DRIVEN prints what the record holds, which is now real); it does not touch the roster command (#648, merged); it adds no `admin/rebind` (question 4); it does not persist a rebind across a body restart (P2); it does not change how a single-process pane's switch and reset record DRIVEN (`tx_launch` keeps the stored value, D3 item 8).

## Scope

In this order:

1. **Contract.** ADR-0021 (the edit list below, section 11 verbatim), `docs/protocol/v2.md` (`session/rebind`, `check_only`, the pane hold paragraph, the error rows), and the passages D1 changes in ADR-0014 (point 5, Consequences), ADR-0005 (section 3) and ADR-0003 (the Addressing line and a header note). One commit, before any test.
2. **`holler-proto`:** `Rebind`, `RebindResult` (new file `rebind.rs`), the catalog row, the count test, the two golden files; `error.rs` (`OFFLINE_REASON`, `Error::offline`) and a third new golden file, the offline `-32004` frame (P7).
3. **`holler-body`:** the `session/rebind` handler (dispatch, session manager command with its deadline, HTTP attach driver re-point), the one source of the attach id (`Inner.config.session_id`, the `SessionPresence` mirror, `live_configs`), the fake server's stall mode.
4. **`holler-hub`:** `resolve_prompt_target`, the re-keyed gate (moved to `panes/gate.rs`), `LiveHandle::rebind` and the per-connection query slot (the allowlist of `LiveHandle::query` is **unchanged**), `control/rebind`, `check_only` (`talk::admit`, extracted from `say`, and `control_say.rs`), the offline `data.reason` on the offline-body arms of `say`, `interrupt` and `answer` (P7), the one `forward` helper under `query` and `rebind`, the `SayOpts`/`LiveCommand::Say`/`HoldGate` field.
5. **`holler-pane`:** `gate` (with `first_refusal`, the one selection), `driven`, `hub` (the port and the codes `pane-not-driven` and `session-held`), `Ports.hub`, the engine's new order (`tx_switch`, which also declares `session-driven-elsewhere`), reconcile's observation and finding, `findings.rs` (`switch_command`, `with_switch_remedy`, `without_remedy`), the module docs.
6. **`holler-pane-testkit`:** `session_id`, `FakeHub`, its conformance.
7. **`holler-cli`:** `wiring/hub.rs` (with `wiring/store.rs`'s failure mapping made `pub(super)`), `pane/hub_refusal.rs` (the `HubRefusal` classification), `Unwired`, `say_cmd` (the shared predicate, a bare id refused as a SESSION, `pane-not-driven` for routed verbs), `interrupt_cmd`, `answer_cmd` and `prompt_target` (`Routed.pane`), `ControlCall` constructors, the 20 `Ports { … }` literals (17 files) (one line each).
8. **Docs:** `docs/pane-control.md`, `CHANGELOG.md`, the module docs.

## The ADR and the dependent docs

**ADR-0021** (locate each by its words; the Evidence line numbers are those of `main` at `4e691d9`, which holds #831, #665 and #835; #835 edited the intro of section 2 and its `HerdrPort` and `HostPort` rows, the `pane import` row of section 9, section 13 (a new "Decided (#835)" block), section 14's `observe_pane` bullet, "Deferred to named stories" (the adoption sentence left item 18's bullet and became a last bullet of its own) and "Decisions taken" (items 30 and 31), so E53 to E65, E89, E90, E178 and E183 are cut at their new lines). Numbers for the new "Decisions taken" items are the next free numbers at merge time (item 31 is the last on `main`, E183, so 32 today; #853's branch also appends 30 and 31 and takes the next free numbers when it rebases, so the numbers depend on which of the two merges first); do not renumber others.

| Where (by words) | Change |
|---|---|
| Header, the **Amends:** line (E53) | Append: "[ADR 0014](ADR-0014.md) (point 5 and its Consequences bullet on `roster`/`status`), [ADR 0005](ADR-0005.md) section 3 and [ADR 0003](ADR-0003.md)'s Addressing line are extended by section 11 (#839): the hub also resolves a live attach-mode session's `harness_session_id` as an address (the `holler` CLI sends one only for `--pane`; a person's `SESSION` is still a name), and may ask an attach-mode body to re-point one of its sessions to another harness session on the same endpoint (`session/rebind`); the session's name, and so its identity, does not change." |
| Section 2, the port table (E54 and the table above it) | Add the row: `\| HubPort \| advertised, rebind(driver, harness_session_id), prompt(harness_session_id, text, mode) (#839) \| holler-cli/src/pane/wiring/hub.rs, a client of the hub's control socket \|`. In the sentence "Both `HarnessPort` and `HostPort` are amended since, under that rule" (section 2, below its port table; #835 lengthened it: it now ends at "(section 13, \"Decided (#835)\")", and the next sentence, on `observe_pane`, is #835's), append before that sentence's full stop: ", and `Ports` by #839, which gains `HubPort` (section 11)". In the sentence "All ports are traits in `holler-pane` (`src/ports.rs`, `src/profile.rs`)" (E54) add `src/hub.rs`. |
| Section 4, invariant I2's test idea (E55) | Append: "After a switch or a reset the driven session is the one the fake hub's body advertises (#839). A launch and a relaunch keep the stored DRIVEN and never infer it; a relaunch onto a new session leaves the body on the old one, which `pane doctor` reports from the stored DRIVEN and `pane switch` re-points (section 11, point 3)." |
| Section 5, "Wiring as built (#649)", bullet "What the wiring makes live" (E57), the sentences from "And since #642 part 3 the first message of `pane reset --first` is a real prompt through `HarnessPort::send_prompt`" through "Until #828 lands, the operator does not use `reset --first` on a parked or held pane." | Replace with: "Since #839 the first message of `pane reset --first` goes through the hub's prompt path, so a parked, held or unhealthy pane refuses it (section 8, 'Switch and reset as built')." |
| Same section, bullet "Not done here", the sentence "The hub's DRIVEN session is not pointed at `session_of_record` (section 11, #654)." | Delete it (done: section 11, #839). |
| Section 5, the "Wiring as built (#649)" bullets (E56) | Add a bullet: "**The hub port (#839).** `connect` also builds `HubPort`, a client of the hub's control socket; it needs only the hub, as the registries' clients do." |
| Section 6 (E58), "the closed 22-row catalog" | "the closed 23-row catalog (22 until #839 added `session/rebind`, section 11)". |
| Section 8, "Switch and reset as built (#645)" (E59) | See "Replacement text for section 8" below. |
| Section 8, "The prompt and activity seam" (E60), from "Two choices differ from what this ADR had proposed for 645b" through "whether it moves to the hub's one prompt path is #828's (section 5, "Wiring as built (#649)")." | Replace with: "One choice differs from what this ADR had proposed for 645b, and #646 and #649 build on it: the activity read is of the pane's session of record, which section 11 (#839) makes the session the hub drives. The first message does **not** go through `HarnessPort::send_prompt` since #839: it goes through the hub's one prompt path (section 11, point 5). `HarnessPort::send_prompt` remains the adapter's transport, with no verb or engine caller; a test fails if one appears." Keep the last sentence ("The adapter's `send_prompt` stays pure transport…"). |
| Section 9, the `pane switch`, `pane reset` row (E61) | Replace "and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default)" with "and, with `--first`: open (#839) `pane-not-driven` (no body drives the pane, or more than one could, or a listed body cannot be shown to be its own), `session-held` (an operator or default hold on the body's session), for `switch` `session-driven-elsewhere` (a live body other than the pane's own already advertises the target), and the pane gate's `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; for both verbs, `session-busy` and `session-holds-question` also when the body driving the pane reports a turn or a question". And, in the same row's clause for a Claude Code pane (#831, section 8), delete ", and with `--first` the port's `session-mismatch` and `inbound-held`": a Claude Code pane's `--first` is refused `pane-not-driven` in the plan (no body until #850), and those two are `send_prompt`'s own answers, which no verb reaches any more. |
| Section 9, the `say`, `interrupt`, `answer` with `--pane` row (E61) | Append: "open (#839) `pane-not-driven`: no body advertises the pane's session of record, or the body that does is offline (it was `unknown session` or `not connected`, exit 1); a turn that outlives `--timeout` or a cancel the body does not acknowledge is still exit 1 with the hub's text, and `say` to a Claude Code pane is `pane-not-driven` until a body drives it (#850). An operator or default hold on the body's session is still the held refusal with exit 4, and a pane hold keeps its `PaneHeld` form; only `pane reset --first` reports a hold as `session-held` (exit 3)". And in the sentence "The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"`" add "for the session whose advertised id is the pane's session of record or recorded DRIVEN". |
| Section 9, the "any other well-formed code (open, raised as `Refused`)" row (E62), the list of open codes | Add "#839's `pane-not-driven`, `session-held` and `session-driven-elsewhere`". |
| Section 11 (E63) | Replace as given in "Amended text of section 11" below. |
| Section 13 (E64), "is replaced by `session_of_record` once the cutover (#654) points the hub's DRIVEN session at it (section 11)" | "is replaced by `session_of_record` once a body is re-pointed to it (#839, section 11); the cutover (#654) starts one attach body for each pane." |
| "Deferred to named stories" (E65, E90), the bullet "How the hub's DRIVEN session follows `session_of_record`: #839 (it was #654's; …)" | Replace with: "A `doctor --fix` that re-points a body, and retiring `HarnessPort::send_prompt`: unscheduled." And in #831's next bullet replace "A body that lets `holler say --pane NAME TEXT` reach a Claude Code pane: #850 (section 15, item 18)." with "A body for a Claude Code pane, so that `holler say --pane NAME TEXT` and `reset --first` reach it and DRIVEN exists for it: #850 (section 15, item 18)." |
| Same section, the bullet "The refusal of a pane that is not idle or holds a question, and `reset --first`", from "and the wiring (#649) makes the first message's path live, and whether it moves to the hub's one prompt path is #828's." | Replace with "and since #839 the first message goes through the hub's one prompt path." |
| Section 8, "Switch and reset of a single process as built (#831, section 15 item 6)" (E178) | Replace the sentence "`reset --first` sends its message to the endpoint of the record the relaunch stored, the new process, never the stopped one." with: "Since #839, `reset --first` on a single process is refused `pane-not-driven` in the plan, before anything is stopped: no body drives a Claude Code pane until #850, and the hub's gate is the only path a first message takes (section 11, point 6)." |
| Section 15, item 18 (E89) | Replace "and through `reset --first`; " with "and, until #839, through `reset --first`; "; replace "`reset --first` reaches the relaunched process (the endpoint of the record the relaunch stored), but it is the existing carve-out" with "`reset --first` reached the relaunched process (the endpoint of the record the relaunch stored) through the existing carve-out"; and replace "once #839 moves it, a Claude pane's `reset --first` waits for #850 too." with "since #839 moved it, a Claude pane's `reset --first` is refused `pane-not-driven` until #850 (section 11, point 6)." This edits #831's merged text, and item 18 itself says #839 would. |
| "Decisions taken" (end of the ADR) | Append the decisions D1 to D6 of this brief and P1 to P6 of the first two plan checks, each as one numbered item marked "(2026-10-11, #839, under the operator's standing rules)", and P7 and P8, each marked "(operator, 2026-10-12, #839)". |

**Replacement text for section 8, "Switch and reset as built (#645)"** (E59). Keep the paragraph up to "…in one compare-and-swap at the generation the plan read, never retried;" and replace everything from "`last_observed.driven` is left as stored (section 11). Then, only for `reset --first TEXT`," through "it has no reconcile step, because the record is correct." with:

> `last_observed.driven` is written in the same compare-and-swap, from the id the body acknowledged (section 11), or `None` when no body drives the pane. Before the act, the plan reads the live attach sessions through `HubPort::advertised` and finds the pane's driver (`holler_pane::driven::driver_of`); a driver that reports a turn or a question is refused (`session-busy`, `session-holds-question`), and a driver that cannot be shown to be the pane's own (ambiguous, or unproven on a listing shared with other panes) is refused `pane-not-driven` unless the pane's session of record, its recorded DRIVEN or a switch's target names exactly one body, and a switch whose target another body already advertises is refused `session-driven-elsewhere`, and for `reset --first TEXT` a pane no body drives, or whose body does not advertise its session of record, is refused `pane-not-driven` and the hub's gate is asked, without sending, whether it would admit the prompt (`HubPort::prompt` with `Check`), so a parked, held or unhealthy pane, or one whose SHOWN and DRIVEN differ, is refused before anything moves: no session is created and the record is unchanged. After `shown_session` shows the target, `HubPort::rebind` asks the body to drive it; a failure there is a failure after the act (the message ends with the reconcile step). Then, only for `reset --first TEXT`, `HubPort::prompt` with `Send` queues `TEXT` through the hub's one prompt path (`send_prompt`), accepted and returned (never waiting for the turn). A prompt that fails after the record, a refusal in a race included, is that call's own error (a refusal is exit 3 with its code), and its message says the session was created, shown and recorded but the first message did not land; it has no reconcile step, because the record is correct. A single process's switch and reset (the next paragraph of the ADR) ask the hub nothing.

Also in that paragraph: "Neither verb checks the pane's park state; a refusal derived from pane state belongs to the prompt gate of #646, which `reset --first` does not pass through (see the seam below)." becomes "Neither verb checks the pane's park state itself; `reset --first` is refused for it by the prompt gate of #646, asked before the act and applied at the send." And "a run makes at most eight port calls" becomes "at most thirteen".

**Amended text of section 11** (E63). Keep the first two paragraphs (from "ADR 0014 stays in force, unchanged." through "…(a session with no pane record).") and the paragraph on `HerdrPort::send_text` and `send_keys`, **with one change to the opening sentence**: "[ADR 0014](ADR-0014.md) stays in force, unchanged." becomes "[ADR 0014](ADR-0014.md) stays in force, extended by points 1 and 2 below (#839: the hub also resolves a session's harness id as an address, and may ask the body to re-point); its teardown rule (point 3) and its rule against typing into a PTY (point 4) apply as they are." (ADR 0014 point 5 and its Consequences are amended, so "unchanged" would be false.) **Replace the paragraphs "Deferred to #654 (cutover)…" and "A switch or reset (#645) likewise leaves…" (the last five lines of the section) with this text, verbatim:**

```text
**DRIVEN as built (#839).** The session the hub drives for a pane is the harness session that a live attach-mode body
advertises (`SessionAd.harness_session_id`, ADR 0014 point 6), and the hub joins it to the pane by that id. Decided under
the operator's standing rules (2026-10-11). Each point names what it rejected.

1. **The address is the advertised `harness_session_id`.** At the hub, `say`, `interrupt` and `answer` resolve a target
   that is a valid name (ADR 0005) by name, as before, and any other target as a harness session id, against the live
   attach sessions: no match is unknown, a match on an offline body is not connected, two matches are ambiguous. The hub
   marks the offline answer, `-32004 not_connected`, with an optional `data.reason` of `offline`; that is how a `--pane`
   verb tells a pane that no body drives from a prompt the body did not answer in time, which is the same code without the
   reason and keeps its text and its exit 1. A name is never an id (the name grammar refuses `_`, which every OpenCode id
   has); an id that happens to be a valid name is resolved as a name and is not routable by id. The id is an address for `--pane` resolution and the pane gate, not a form
   a person types: the `holler` CLI refuses a `SESSION` positional that is not a valid name (`holler say ses_… TEXT` is
   `unknown session`, as before) and sends an id only for `--pane`. This extends ADR 0005 section 3, ADR 0003's Addressing
   line and ADR 0014 point 5 and its Consequences; the name is still the identity. The pane gate (`pane-parked`,
   `pane-unhealthy`, `pane-shown-driven-mismatch`) is keyed on the advertised id of the session being prompted, however the
   prompt addressed that session, and selects the panes whose session of record or recorded DRIVEN it is; it is no longer
   keyed on the session's name, so a bare `say` to a body session that drives a parked pane's session of record is refused
   too. One predicate, `holler_pane::gate::prompt_refusal`, decides the three refusals for the hub's gate and for `--pane`;
   their messages stay where they are. Rejected: joining on the pane's name (nothing makes a body's session name its
   pane's name, and the join would be a naming rule imposed on a fleet); a new field of `Pane` naming the body session (a
   second source of truth beside the advertisement, and a change of the stored record); a new request to ask a body which
   session it drives (presence already carries it); keeping the name match beside the id match (two keys, and only test
   fixtures match the old); keying the gate on the session of record alone (a body that is behind would not be gated);
   letting a bare id work as a typed SESSION (a second typed address, and a stale id would reach a body).
2. **A body is re-pointed, not restarted: `session/rebind`.** A new additive request, hub to body, `{session,
   harness_session_id}` answered `{session, harness_session_id}` (protocol v2 section 4; the catalog has 23 rows). The body
   re-points one of its own attach sessions to another harness session on the same endpoint, after the existence check
   `attach` makes (never `session/new`), and only when the session is idle: a session that is not an attach session is
   `invalid_params`, an unknown name `unknown_session`, a busy session or one holding a question `session_busy`, an id the
   endpoint does not have `invalid_params` with the old binding kept, an endpoint that does not answer within 5 s
   `not_connected` with the old binding kept (an attach that finishes after the deadline is dropped, never swapped in). The
   session's name does not change, so its address and its hold key do not (point 4); ADR 0005 section 3 says the name is
   the identity and the locator changes. The session task's own configuration is the one source of the id a body attaches
   to: the rebind writes the new id there once the new driver has attached, so the re-attach after a turn that ended in
   error and the attach retry follow it, and presence and `query/support` read it from there. The hub sends the request
   through its own `LiveHandle::rebind`, not through the generic request forward, whose allowlist is unchanged, so
   `control/query_remote` and `admin/query` cannot carry it. `control/rebind`, a method of the local control socket only
   (there is no `admin/rebind`), is bounded by 8 s (the CLI waits 10 s) and, before it replies, updates the hub's cached presence and the
   body's roster rows, so a read after the reply sees the new id. One forwarded request is in flight on a connection at a
   time: the circuit has one reply slot, shared by `session/answer`, `query/*` and the rebind, and the hub serialises the
   three. Only the HTTP attach driver implements it; a body for another harness implements it in its own driver. Rejected:
   restarting the body with a new configuration (not bounded by I5, and the CLI would supervise a process and rewrite a
   file the wizard owns); a new body session for each harness session (a new name is a new hold key, so `--join-held`
   would default-hold every reset); the hub or the CLI posting the prompt to the pane's server (an adapter in the hub, and
   a second way to deliver a prompt); leaving it to the cutover (the first message of `reset --first` could not be
   delivered, and the gate would refuse everything as `pane-shown-driven-mismatch`); allowlisting the request on the
   generic forward (`control/query_remote` and `admin/query` would carry it without the cache update); persisting the
   binding across a body restart (state for a window the cutover does not need).
3. **DRIVEN is observed, never inferred, and the hub does not write it (I6).** The pure function
   `holler_pane::driven::driver_of` finds the body that drives a pane, and accepts a body only on positive evidence. A
   body that advertises another pane's session of record, or another pane's recorded DRIVEN, is that pane's and is never
   a candidate for any rule below, the fallback included. The exception is a body that also advertises this pane's own
   session of record or recorded DRIVEN: this pane's evidence is checked first, so a stale DRIVEN on another pane never
   takes a pane's own body. Of the bodies that remain, those that advertise the pane's session of record are its drivers; else those that advertise its recorded
   DRIVEN (the record's own memory, which finds a body whose session the pane's server no longer lists); else, for a
   switch, those that advertise the session being switched to. One match is the driver and more than one is ambiguous.
   With no evidence a body counts
   only if the pane's server lists no other pane's session of record (servers that share a data directory list each
   other's sessions, so a listed id proves nothing there), lists the body's id, and the body is the only such one;
   where the listing is shared, a listed body is unproven. Ambiguous and unproven bodies are refused alike. The port
   `HubPort` (`advertised`, `rebind`, `prompt`) reads the hub through its control socket. A switch or reset writes
   `last_observed.driven` in the same compare-and-swap as the record, from the id the body acknowledged to `rebind`
   (`None` when no body drives the pane; a body already on the target is not asked); reconcile reads `advertised` once
   per pass and records what it found with `shown`. `shown-driven-mismatch` is also reported when an observed DRIVEN
   is neither SHOWN nor the session of record; `--fix` repairs only the TUI side, and a body that is behind is
   `not-fixable` with the finding's own remedy `holler pane switch PANE SESSION` for the session of record, which is
   idempotent. The order of a switch or reset is: plan (the refusals of section 8, then the driver, then for `--first`
   the gate's answer), act, observe, rebind, record, first message. A switch whose target another body, not the pane's
   driver, already advertises is refused in the plan, before anything moves (two bodies would drive one harness
   session, which ADR 0014 leaves undefined; a body the rule above removed counts as one that advertises it). The rebind comes after the observation so that a failed `select_session`
   leaves the body where it was and `doctor --fix` restores the rest. If the record's write fails after the rebind,
   the body is ahead of the record: `doctor` reports it and `pane switch` re-points it back; nothing is compensated
   (section 8). A body restart reverts a re-pointed body to the session in its configuration (the binding is not
   persisted). When the pane's server lists only its own sessions, `doctor` finds the body, reports it the same way,
   the same remedy re-points it, and the gate refuses prompts to it once its id is recorded as the pane's DRIVEN. When
   servers share a data directory, a reverted body's configured id is nobody's session of record and nobody's recorded
   DRIVEN, so nothing proves whose body it is: the verbs refuse `pane-not-driven`, naming each listed body, and the
   operator makes the pairing explicit by switching the pane to the session its own body drives, then to the session
   of record. `launch` and `relaunch` keep the stored DRIVEN and never infer it, so a relaunch onto a new session (the
   restarted server no longer lists the old one) leaves the body on the old session; the body advertises the stored
   DRIVEN, so `doctor` finds it, reports the mismatch and `pane switch` re-points it. If no DRIVEN was ever recorded
   for the pane, nothing finds that body and it must be restarted with the new session in its configuration. The
   window that remains: a body that is behind and that no pass or verb has yet observed is not refused by the gate.
   Rejected: the hub writing DRIVEN (the hub is the store only, ruling 1); recording the session of record as DRIVEN
   without observing it (inference); driving before the act (a failed select would leave the body moved, with no
   repair); choosing among ambiguous bodies by order (a wrong pairing, silently); taking a body to be a pane's because
   the pane's server lists its id (shared listings); joining on the body's attach endpoint (a `query/status` per body
   per pass, and endpoint strings to compare).
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
   `pane-not-driven` before anything moves, as it is when the body does not advertise the pane's session of record, and when
   more than one body could drive the pane, or none can be shown to be its own, and the call names none of them (point 3). Only an OpenCode attach body exists
   today. A Claude Code pane (#831) is in this case until a body can drive it (#850), and its switch and reset, which are a relaunch, ask the hub nothing; the join is a string comparison of the
   advertised id with the session of record, so its session id joins the same way once a body advertises it, and that body
   implements `session/rebind` in its own driver.
```

**`docs/protocol/v2.md`** (E66 to E70):
- Section 4 table (E66): a row after `session/answer`: `session/rebind | request | hub→body | {session, harness_session_id} | {session, harness_session_id} — sent only after the driver targets the new session and presence advertises it (#839)`.
- Section "Attach-mode sessions" (E69): a paragraph "**Re-pointing (#839).**" stating the body's refusals and the 5 s bound of D2 (an attach that finishes after the deadline is not swapped in), that the session name does not change, and that the request is not forwardable through `query/*`: `control/query_remote` and `admin/query` refuse `session/rebind` with `-32601`.
- Section 7, presence (E67): after the sentence "`harness_session_id` is present for `attach`": "A `session/rebind` changes it; the hub's cache and roster row are updated when the rebind is acknowledged."
- The `control/say` description (wherever `admin/say`'s "identical to control/say's `{session, text, queue?, grant?, timeout_ms?}`" is, E66): add `check_only?`, and say that `session` may be a harness session id at the hub (D1; the `holler` CLI sends one only for `--pane`, and refuses one typed as a SESSION).
- The pane hold paragraph (E68): "the session of record of a pane" → "a session whose advertised `harness_session_id` is the session of record of a pane"; keep the rest.
- Error rows (E70, E184): `-32009` and `-32004` rows gain the rebind cases ("or a `session/rebind` of a session with a turn in flight or a question pending"; "or a `session/rebind` whose endpoint did not answer"). The `-32004` row (E184) also gains the offline reason (P7): "or a `say`, `interrupt` or `answer` whose target resolves to an offline body, which carries `data.reason: "offline"`; the field is optional, and a `-32004` without it is a prompt the body did not answer within the wait, a prompt cancelled before it completed, a cancel the body did not acknowledge, or a failed redirect". The sentence under the table (`data` is `{"code", "reason"?}`) already allows it; the reasons on `-32004` are `"offline"` (the hub, P7) and `"endpoint did not answer"` (the body, for a rebind).
- The non-normative note at the end about `pane/*` (v2.md line 824, 817 before #851): leave, and add one sentence that `control/rebind` is a control-socket method as `pane/*` are, outside the catalog.

**`docs/adr/ADR-0014.md`, `docs/adr/ADR-0005.md`, `docs/adr/ADR-0003.md`** (P3; E51, E52, E131, E130). Each passage D1 changes is amended in the contract commit:
- **ADR 0014 point 5** ("Addressing is unaffected"): append "Since [ADR 0021](ADR-0021.md) section 11 (#839) the hub also accepts the `ses_…` id as an address, for its own `--pane` resolution and pane gate (the `holler` CLI's `SESSION` stays a name), and a `session/rebind` can change it; the name is still the identity."
- **ADR 0014 Consequences**, the bullet "`roster`/`status` surface `mode` and the harness session id so an operator can tell attach apart from spawn without it changing how they address the session": append "(Extended by [ADR 0021](ADR-0021.md) section 11 (#839): the hub also resolves that id as an address for `--pane`; a person's `SESSION` is still a name.)". And one new bullet at the end: "- Extended by [ADR 0021](ADR-0021.md) section 11 (#839): a hub may ask an attach-mode body to re-point one of its sessions to another harness session on the same endpoint (`session/rebind`). The session's name, and so its address, does not change."
- **ADR 0005 section 3** (E52), after "a command is never issued to a locator it was handed earlier.": "Since [ADR 0021](ADR-0021.md) section 11 (#839), a `say`, `interrupt` or `answer` target that is not a valid name is resolved at call time as the `harness_session_id` that a live attach-mode session advertises (the `holler` CLI sends one only for `--pane`; a person's `SESSION` is still a name); the name stays the identity, and the locator may change (`session/rebind`)."
- **ADR 0003**, "Not decided here", the Addressing line (E130): append "A bare harness session id (`ses_…`) is not a `SESSION`: `holler say ses_… TEXT`, `interrupt` and `answer` refuse it as `unknown session` (exit 1); only `--pane` resolves to one, and the hub accepts it as an address ([ADR 0021](ADR-0021.md) section 11, #839)." And one `**Clarified by:**` line in the header, after the last one, in the form of the others: "[#839](https://github.com/Performant-Labs/holler/issues/839) (2026-10-11) — a harness session id is an address at the hub, for `--pane` only; the CLI's `SESSION` positional stays a name."
- **ADR-0021's Amends line** names all of the above (the first row of the table).

**`docs/pane-control.md`** (E71 to E75; `docs_pane_control_test` pins the guide to the code's tables, so it must still pass): section for `reset` (E71): replace the paragraph "**Do not use `--first` on a parked or held pane.** …" with the new rule (the first message goes through the hub's gate; a parked, held or unhealthy pane, one whose SHOWN and DRIVEN differ, and one no body drives are refused before anything moves, `pane-parked`, `session-held`, `pane-unhealthy`, `pane-shown-driven-mismatch`, `pane-not-driven`), and add that a pane needs a body attached to its server for `--first`; in the same section replace "`--first` goes to the relaunched process, through the same direct path as below" (a Claude Code pane's reset, #831) with "`--first` on a Claude Code pane is refused `pane-not-driven` until a body drives it (#850)"; section for `park` (E71): delete "; the first message of `reset --first` is not (see `reset`)"; DRIVEN column (E72) and "SHOWN against DRIVEN" (E73): DRIVEN is the session the body that drives the pane advertises, recorded by `switch`, `reset` and `doctor`, `-` when no body drives the pane; SYNC and `doctor` compare as ADR section 11 says; cutover steps (E74): delete "and `reset --first` is not used on a parked or held pane (section 3)", and add that each pane needs an attach body before the cutover step that uses `reset --first`; "Later" (E75): delete the bullet "DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839)", and in the bullet "a body that lets `holler say --pane NAME TEXT` reach a Claude Code pane (#850, after #839)" drop ", after #839" ("Later" is at E75; #835 reworded the bullet between the two this story edits, on adopting the Claude Code panes the import leaves alone, and that bullet stays). Section 9's error table gains `pane-not-driven`, `session-held` and `session-driven-elsewhere`. Also: a short paragraph that a body restart reverts a re-pointed body to its configured session, that `pane doctor` is run before any relaunch and after restarting bodies, and reports it as `shown-driven-mismatch` with the remedy `holler pane switch PANE SESSION` (on servers that share a data directory a reverted body is reported without a remedy, and the two-step switch pairs it); the message of the ambiguous-body refusal and its two steps (switch to the session the pane's own body drives, then to the session of record); and that a `ses_…` id is typed only as `--pane` resolution, never as a `SESSION` (`holler say ses_… TEXT` is `unknown session`).

**Module docs that repeat the old sentences** (edit by words): `crates/holler-pane/src/tx_switch.rs` lines 29-36 and 61 (E30), `crates/holler-pane/src/reconcile.rs` lines 52-54 (E34), `crates/holler-cli/src/pane/reset.rs` lines 1-15, `crates/holler-pane/src/ports.rs` line 491 ("`pane reset --first` calls it", E28) and the amendments list at its top (lines 15-41, E92), `crates/holler-adapter-opencode/src/prompt.rs` line 7 ("`pane reset --first` sends through `send_prompt` outside that path"), `crates/holler-hub/src/panes/gate.rs` (the moved docs of `pane_state_refusal` and of the tests that name the CLI twin; `circuit/dispatch.rs` loses them), `crates/holler-cli/src/say_cmd.rs` (the doc of `resolve_pane_target` that names the twin), `crates/holler-cli/src/roster_cmd.rs` (the doc of `shown_driven_mismatch` that says "keep the three equal", E154), `crates/holler-proto/src/methods.rs` (the "22-row" doc and the count test, E42, E43), and `crates/holler-body/src/registry.rs` (the doc of `SessionRegistry::presence_doc`: the as-started baseline, not the live id). `crates/holler-hub/src/live.rs`'s allowlist comment (E6) is **not** edited (P1).

**`CHANGELOG.md`**, under `## [Unreleased]`, `### Enhancements`, one new item (E76 shows the shape), linking `/issues/839` and `/issues/828`:

> - Pane control, DRIVEN (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): the session the hub drives now follows each pane's session of record ([#839](https://github.com/Performant-Labs/holler/issues/839), which absorbs [#828](https://github.com/Performant-Labs/holler/issues/828)). The hub joins a pane to the attach-mode body that advertises its session of record, and the hub resolves that `ses_…` id as an address for `say`, `interrupt` and `answer`, so `--pane` now reaches a real pane (a `ses_…` typed as the session argument is still refused, as before); the pane gate (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) is keyed on the same id, and on a pane's recorded DRIVEN, instead of the body's session name. `holler pane switch` and `holler pane reset` ask the body to follow the new session (a new additive hub-to-body request, `session/rebind`; the protocol catalog has 23 rows) and record `last_observed.driven` from its answer, and `holler pane doctor` records it too and reports `shown-driven-mismatch` when it differs; a body is taken to be a pane's only on evidence (its session of record, its recorded DRIVEN or the session being switched to), and a switch onto a session that another body already drives is refused (`session-driven-elsewhere`). A restarted body attaches its configured session again; `holler pane doctor` reports that and `holler pane switch PANE SESSION` re-points it. The first message of `holler pane reset --first` now goes through the hub's prompt path: a parked, held or unhealthy pane refuses it before anything moves (`pane-parked`, `session-held`, `pane-unhealthy`, `pane-shown-driven-mismatch`), and a pane no body drives refuses it (`pane-not-driven`; it worked through the harness before), a Claude Code pane included until [#850](https://github.com/Performant-Labs/holler/issues/850) gives it a body. `control/say` gains an optional `check_only`, and the hub's `not_connected` refusal of an offline body gains an optional `data.reason` of `offline`, which is how `say --pane` tells a pane no body drives from a turn that outlived its wait. This replaces the caution about `reset --first` on a parked or held pane in the entry above. [ADR 0021](docs/adr/ADR-0021.md) section 11 records the decisions.

and, in the existing #649 item (E179: "so until [#828] lands, do not use `reset --first` on a parked or held pane"), replace that clause with "and, since [#839](https://github.com/Performant-Labs/holler/issues/839), through the hub's gate".

## Where the code goes

Sizes measured with `wc -l` on 2026-10-12 on `main` at `b76f4d2` and re-counted at `4e691d9` after #835 (only `ports.rs` differs); `scripts/lint.sh` fails at 900. The issue's numbers for `switch.rs` (~833) and `support/mod.rs` (~849) have grown to 893 and 857. **Test modules:** each `#[cfg(test)] mod …;` line this story adds (or moves) is the last line of its file, as in the hub's precedent (`holds.rs:575-576`, `lockout.rs:589-590`, E172, E173): `hold_single_path_test.rs`'s `code_of` cuts a file at its first `#[cfg(test)]` (E171), so a `mod` line near the top would hide the code below it from the scans. A test file that is out of line is read unstripped by `rust_files`; the moved hub tests contain no `"session/prompt"` + `Envelope::request` line and no `LiveCommand`, so the two scans do not count them.

| File | Now | What | Rule |
|---|---|---|---|
| `crates/holler-hub/src/circuit.rs` | 898 | The `LiveCommand::Say` arm (E4): destructure `harness_session` and set `HoldGate.harness_session` | **+0 lines**: edit the existing lines in place (they already exceed 100 columns); do not reformat the file |
| `crates/holler-hub/src/live.rs` | 884 | `SayOpts` and `LiveCommand::Say` gain one field each (E5); `mod target;`; `LiveHandle.query_slot` (field, its initialiser in `insert`); the private `forward` helper right after `query` (the one place a `LiveCommand::Query` is built: slot, send and wait inside one `timeout`; `query` calls it after its allowlist check, E139); **the allowlist and its comment are unchanged** (P1) | about **+13** (884 to 897: `mod target;` 1, field 2, initialiser 1, `forward` 12 with its doc, `query` 8 shorter, two `Say` fields 2). **Do first, in the same commit as `mod target;`:** move the inline `mod tests` (lines 812-884) to `crates/holler-hub/src/live/tests.rs`, a pure move, which leaves the file near 825; the file then ends with `#[cfg(test)]`, its `#[allow(...)] // #185` line and `mod tests;`, as its last lines. `resolve_prompt_target` and `LiveHandle::rebind` go in the new `crates/holler-hub/src/live/target.rs` |
| `crates/holler-hub/src/circuit/dispatch.rs` | 820 | `HoldGate.harness_session`; `pane_gate` and `pane_state_refusal` leave for `panes/gate.rs` (E1, E2); `send_prompt` calls the moved `pane_gate`; the tests of `hold_tests` that drive `send_prompt` (E81, E82 and their neighbours) **stay here** and are rewritten on `ses_` ids and the new key (the `HoldGate` and the `Recording` sink are private to `circuit`, E170) | about 85 lines leave with the gate and up to about 50 are added by the new cases (the DRIVEN key, a session whose name equals a record, a spawn session): it ends near 780 to 830. If it would pass 850, first move `hold_tests` out whole to `crates/holler-hub/src/circuit/dispatch/hold_tests.rs` (a pure move; `#[cfg(test)] mod hold_tests;` last in `dispatch.rs`; the file is a child of `dispatch`, so the private items stay reachable) |
| `crates/holler-hub/src/talk.rs` | 540 | `say`'s prelude (parse and resolve, hold fast path, busy check, E138) moves into `talk::admit`, which adds the pane gate; `say` calls it and then `send_turn`; the `check_only` branch calls `admit` alone; `answer`'s two resolve lines; `SayOpts` gets `harness_session` | ok (net about +10: the prelude moves, it is not copied); if more than +30, `crates/holler-hub/src/talk/admit.rs` |
| `crates/holler-hub/src/interrupt.rs` | 155 | the two resolve lines | ok |
| `crates/holler-hub/src/control_server.rs` | 837 | one arm for `control/rebind` in `dispatch_session_control`; `check_only` read in `say`; the `SayError` arms of `say` (E122) move to `control_say.rs`; the offline arms of `interrupt` and `answer` build `WireError::offline(..)` (E161, E162) | net **negative**; everything else in `control_rebind.rs` and `control_say.rs` (both new, each under 200) |
| new `crates/holler-hub/src/panes/gate.rs`, `panes/gate_tests.rs`; `panes/mod.rs` | 0; 226 | `pane_gate` (a mapping of `holler_pane::gate::first_refusal`'s answer to the `-32011` refusal, with the hub's message text), `pane_state_refusal` and their tests of the frame (Acceptance 4); `mod gate;` | each under 200; `mod.rs` +1; `#[cfg(test)] mod gate_tests;` is the last line of `gate.rs` |
| `crates/holler-hub/src/lib.rs` | 32 | `pub mod control_rebind;`, `pub mod control_say;` | +2 |
| `crates/holler-hub/src/control.rs` | 518 | `ControlCall::rebind` (timeout 10 s, above the hub's 8 s forward), `ControlCall::say_check` | ok |
| `crates/holler-hub/src/roster.rs` | 892 | nothing: `Roster::advertise` is reused | **no growth** |
| `crates/holler-proto/src/docs.rs` | 899 | nothing | **no growth**; `Rebind`, `RebindResult` in new `crates/holler-proto/src/rebind.rs` |
| `crates/holler-proto/src/methods.rs` | 219 | one catalog row, the count and doc | ok |
| `crates/holler-proto/src/error.rs` | 303 | `OFFLINE_REASON` and `Error::offline(message)` beside `Error::invalid_grant` (E163, E164) | about +14 |
| `crates/holler-proto/tests/golden_test.rs` | 356 | one test beside `unauthenticated_error_frames_match_golden` (E165) for the offline `-32004` frame; the new golden file `crates/holler-proto/tests/golden/envelope/error.not_connected_offline.json` | about +10; no existing golden changes |
| `crates/holler-body/src/connection.rs` | 873 | the `matches!` list (E48) and `static_wire_method` gain the name; the `handle_query` call passes `live_configs(...)` in place of `self.configs` (E111) | at most **+2**; the handler is in `prompt_dispatch.rs` (355) and `connection/session_dispatch.rs` (120) |
| `crates/holler-body/src/session_manager/task.rs` | 705 | the `Rebind` handler (deadline, build-then-swap), `mirror_session_id`, the `SessionPresence` field | ~+90 (about 795); if it would pass 850, the handler goes to `session_manager/task/rebind.rs`; its unit tests are in a new `session_manager/task/rebind_tests.rs` (`#[cfg(test)] mod rebind_tests;`, +1, last line); the driver swap in `http_attach_driver/` (`http_attach_driver.rs` 435, `connection.rs` 618) |
| `crates/holler-body/src/session_manager.rs` | 389 | the `Rebind` command and `SessionManager::rebind`, `start` builds the presence with the configured id (E143), `presence_doc` reads the mirror, `live_configs` | ~+47 |
| `crates/holler-body/tests/http_attach_driver_test/fake_server.rs` | 347 | a stall mode for the existence check (E133, E134) | ~+25; the CLI's `attach_mode_test` includes this file by `#[path]` (E87), so it must keep compiling there |
| `crates/holler-proto/tests/codec_test.rs` | 889 | one `canonical_frame` arm for `session/rebind` (E132) | **+1** (890) |
| `crates/holler-pane/src/ports.rs` | 621 (563 before #835) | `Ports.hub`; the amendments list at its top | +1 field, +6 doc |
| `crates/holler-pane/src/tx_switch.rs` | 487 | the new order in `plan` and `switch_in_place` (E31, E175); `prompt` calls `hub.prompt` (E158); `activity_refusal` shared with `check_idle`; `SESSION_DRIVEN_ELSEWHERE`; a single-process pane takes the driver as `None` without asking the hub | ~+80 (about 570), ok; if it passes 650, the plan helpers go to `tx_switch/plan.rs` |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs` | 533, 449 | the observation of `hub.advertised` once per pass and the finding | ok; if `reconcile.rs` would pass 700, the observation goes to a new `reconcile/driven.rs` |
| new `crates/holler-pane/src/{gate,driven,hub}.rs`; `lib.rs` | 0; 81 | D1 (`gate.rs`: `prompt_refusal`, `shown_driven_pair_differs`, `first_refusal` and the three pane codes), D3 (`driven.rs`: `driver_of`, `elsewhere`; `hub.rs`: the port, `pane-not-driven` and `session-held`) | each under 250; `lib.rs` +3 |
| `crates/holler-pane/src/findings.rs` | 379 | `switch_command` beside the builders, `Finding::with_switch_remedy`, `Finding::without_remedy`, the amended doc of `remedy` (E141, E142) | ~+25, ok |
| `crates/holler-cli/src/say_cmd.rs` | 535 | the shared predicate; a bare id refused in `route_target_by` (E123); `Routed.pane` set; `wait_ended` made `pub(crate)`; the `NotDriven` arm in `control_refusal` (E149) | net about zero (the twin predicate leaves, the arm comes); under 560 |
| `crates/holler-cli/src/{interrupt_cmd,answer_cmd,prompt_target}.rs` | 196, 99, 202 | the `--pane` arms call `hub_refusal` (E124, E125); `Routed.pane` (E126) is set in `prompt_target::route`, in `interrupt_cmd::route` (E169) and in `route_target_by` | ok (+14, +10, +3) |
| new `crates/holler-cli/src/pane/hub_refusal.rs` | 0 | D5's one classification of a hub refusal (`HubRefusal`); each caller maps what it needs | under 200 |
| new `crates/holler-cli/src/pane/wiring/hub.rs`; `pane/wiring/store.rs` | 0; 397 | the real `HubPort`; `Effect`, `timed_out`, `hub_unavailable`, `undecodable` made `pub(super)` (visibility only, E145, E146) | under 250; +0 to +4 |
| `crates/holler-cli/tests/hold_single_path_test.rs` | 81 | the forward-shape scan (Acceptance 13) | +30 |
| new `crates/holler-pane-testkit/src/hub.rs`, `conformance/hub.rs`; `lib.rs`, `conformance/mod.rs` | 0; 46, 139 | `FakeHub` over `fault::FaultSwitch<HubOp>` (E147) and `holler_pane::gate::first_refusal`, its conformance | each under 250; `lib.rs` +2, `mod.rs` +2 |
| `crates/holler-cli/src/pane/wiring/bind.rs` and `crates/holler-pane/src/ports/router.rs` | 489; 145 | **not edited**: their `send_prompt` pass-throughs (E127, E159) are the two calls the scan allows | 0 |
| `crates/holler-cli/src/pane/wiring.rs` | 468 | `mod hub;`, the field, `Unwired`'s impl | ok; the client is `wiring/hub.rs` (new) |
| `crates/holler-cli/src/roster_cmd.rs` | 578 | `shown_driven_mismatch`'s body becomes a call to `holler_pane::gate::shown_driven_pair_differs`; its doc stops naming three copies (E154) | about -2 |
| `crates/holler-cli/tests/pane_verbs/switch.rs` | 893 | **no growth** (it already holds `SeamHarness`) | new tests in new files |
| `crates/holler-cli/tests/support/mod.rs` | 857 | at most a `mod driven;` line | helpers in new `crates/holler-cli/tests/support/driven.rs` |
| `crates/holler-cli/tests/pane_verbs/reset.rs` | 532 | E83 and E84 are rewritten in place; the rest of the new tests are in new `crates/holler-cli/tests/pane_verbs/driven.rs` (add `mod driven;` to `main.rs`, 81 lines; it also holds D8's fixture test) | |
| the 20 `Ports { … }` literals (17 files) | | one line each (`hub: &hub,`), E180: `verb_harness/mod.rs`, `pane_integration/{bind,observe}.rs`, `pane_claude/rig.rs`, `roster_panes_test.rs`, `pane_verbs/{park/rig,list,doctor/rig,park,close/rig,launch/rig,import/rig (two)}.rs`, `profile_verbs/{rig (two),rename,apply/rig}.rs`, `holler-pane/tests/ports_test.rs` (two), `pane/wiring.rs` | `launch/rig.rs` is 852: +1 is fine (`import/rig.rs` is 535, `profile_verbs/apply/rig.rs` 775); #835 added the `observe.rs` literal and the second `import/rig.rs` one |

## Reuse (the survey for T and F)

Extend these; write a new one only where "Where the code goes" says so. `talk::send_turn` (the one place `SayOpts` is built, E10); `LiveCommand::Query` and `LiveHandle::query`'s plumbing (E6, the rebind's transport); `Registry::replace_session` and `Registry::find_by_token`; `control_hold.rs` and `holds.rs` (the precedents for `control_rebind.rs`, the hold keys); `wiring/store.rs` (the precedent for `wiring/hub.rs`); `say_cmd::{QUEUE_ACCEPT_WAIT, wait_ended}` (the accept rule); `holler_pane::ports::Activity` and `tx_switch::check_idle` (the activity mapping); `Finding.remedy` (E121); `holler_pane::gate` (new: the one predicate, `prompt_refusal`, that replaces the hub's and the CLI's copies, and the one selection, `first_refusal`, that the hub's gate and `FakeHub` share); the body's fake OpenCode server (`http_attach_driver_test/fake_server.rs`, with its `exists_v1`/`exists_v2` toggles and `push_event`); `hold_hub_test::a_prompt_cannot_be_forwarded_around_the_hold` (the pattern for the closed forward); `talk::say`'s prelude (moved into `talk::admit`, not copied, E138); `LiveHandle::query`'s send-and-wait (moved into the one `forward` helper); `wiring/store.rs`'s `Effect`, `timed_out`, `hub_unavailable` and `undecodable` (the one mapping of a failed exchange: `wiring/hub.rs` routes every `ControlError` that is not a refusal through them, E145, E146); `findings.rs`'s command builders and `with_fix` (E141, E142: `switch_command` goes beside them, and the remedy is set after `with_fix`); `holler_pane_testkit::fault::{FaultSwitch, PortOp}` (the faults and call log of `FakeHub`, with a `HubOp`, E147) and `SeamHarness`'s `then_of_record` (the order across two fakes, E148); `say_cmd::run`'s construction of the accept-and-return call (E168: `say_with` with the turn budget, then only `call.timeout` overridden); `Error::invalid_grant` and `unauthenticated_error_frames_match_golden` (E164, E165: the precedent for `Error::offline` and its golden file); `ports/router.rs` and `wiring/bind.rs` (E159, E127: the two pass-throughs the `.send_prompt(` scan allows).

## Acceptance (each is a test)

Fixtures use `ses_`-shaped ids (D8). "On the fakes" means the test kit's fakes with `FakeHub`.

1. **I2 extended to DRIVEN.** After a switch and after a reset on the fakes, with a body advertising the old session, `session_of_record`, SHOWN and the fake hub's advertised id are equal, `last_observed.driven` equals them, and the record was written by one `cas_put` (new file `pane_verbs/driven.rs`).
2. **The step order.** The fakes' call logs show: reads, `advertised`, [`prompt(Check)`], `create_session` (reset), `select_session`, `shown_session`, `rebind`, `cas_put`, [`prompt(Send)`]; `rebind` is after `shown_session` and before `cas_put`; `prompt(Send)` is after `cas_put`; `HarnessPort::send_prompt` is never called; no keystroke (I4). This is the in-place path; a single-process pane is Acceptance 22.
3. **A message through the hub's prompt path reaches a pane's real session of record.** Default-run end-to-end test (new file `crates/holler-cli/tests/driven_e2e_test.rs`, pattern E87): a real hub, a real body in attach mode against the fake OpenCode server with a `ses_`-shaped id, the pane registered through the pane store; `holler say --pane P "hi"` is delivered to `POST /session/<id>/prompt_async` and the reply returns. `say --pane` on a pane with no body exits 3 with `pane-not-driven`. `interrupt --pane` and `answer --pane` resolve the same id.
4. **The gate refuses a real-shaped pane.** The selection (`holler_pane::gate::first_refusal`: by session of record, by recorded DRIVEN, two panes recording one id fail closed, the first bad one in name order is the one named) is pinned in `crates/holler-pane/tests/gate_test.rs`. The enforcement point, which is every case below that sends a prompt (every variant, nothing on the socket, the hold refusing first), is pinned in `circuit/dispatch.rs`'s `hold_tests`: they drive `send_prompt` with the private `HoldGate` and `Recording` sink, so they stay there, rewritten on `ses_` ids and the new key. `pane_gate` alone (the `-32011` frame with `hold_kind: "pane"`, the pane code as `reason` and the hub's message, for one refused pane and for the first bad of two; an unreadable store refuses nothing) is pinned in `panes/gate_tests.rs`. The end-to-end test covers the whole. The cases: parked gives `pane-parked`, unhealthy `pane-unhealthy`, SHOWN differing from DRIVEN `pane-shown-driven-mismatch`; one side absent is no mismatch; two panes recording one session are fail-closed with the first bad one named; an operator hold refuses first with its own kind; a body session with no `harness_session_id` (spawn) is never pane-gated; **a session whose NAME equals a pane's session of record is not gated** (the old key is gone); a bare `say <name>` to the body that drives a parked pane is refused; **a pane's recorded DRIVEN is a key too**: a body advertising `ses_old` while the pane's session of record is `ses_new` and its recorded DRIVEN is `ses_old` is refused `pane-shown-driven-mismatch` for a bare `say`, and is admitted when no DRIVEN was recorded (the window the ADR states).
5. **Resolution.** `resolve_prompt_target`: an id matches the one live attach ad; a name still resolves by name; an unknown id is `Unknown`; an id on an offline body is `NotConnected`; two bodies advertising one id are `Ambiguous`; a valid-name-shaped id string never resolves by id. `say`, `interrupt` and `answer` accept an id (new file `crates/holler-hub/tests/driven_resolve_test.rs`). A typed bare id is refused at the CLI (Acceptance 18).
6. **`doctor`.** `shown-driven-mismatch` is reported for a pane whose observed DRIVEN differs from SHOWN (TUI right, body behind: `not-fixable`, remedy `holler pane switch PANE SESSION`; TUI wrong: fixable as today), and not for one that agrees; `last_observed.driven` is recorded in the same write as `shown`; `hub.advertised` failing is an `observe-failed` finding and leaves `driven` as stored; `driver_of`'s table (record preferred, then stored DRIVEN, then the target; a stale body found; the exclusion rule (D3) with its exception; none; ambiguous; unproven; the listed-candidate fallback alone on an unshared listing; Acceptance 20) in `crates/holler-pane/tests/driven_test.rs`. **A reverted body:** one body advertising its configured `ses_old` (listed by the pane's server), the record and SHOWN `ses_new`: exactly one `shown-driven-mismatch` finding, `fix: not-fixable`, `remedy` = `holler pane switch PANE ses_new`, DRIVEN recorded `ses_old`; the five `FixState` names and their JSON are unchanged (no new state); `pane switch PANE ses_new` then re-points that body and records `ses_new`. **A relaunch onto a new session** (`tx_launch/server.rs:126-140`, E129: the restarted server no longer lists `ses_old`, so the relaunch created `ses_new` as the record and SHOWN): stored DRIVEN `ses_old`, not listed, and a body advertising `ses_old`: `driver_of` finds the body by the stored DRIVEN (rule (ii)); `doctor` reports exactly one `shown-driven-mismatch`, `not-fixable`, `remedy` = `holler pane switch PANE ses_new`, DRIVEN `ses_old`; `pane switch PANE ses_new` re-points the body and records `ses_new`. The same case with no stored DRIVEN finds no body and reports no mismatch (the stated window, pinned so that it cannot change silently). The fake server, hub and records are the kit's; no relaunch is run.
7. **`reset --first` obeys the gate.** On a parked pane, an unhealthy pane, a pane whose SHOWN and DRIVEN differ, a held pane (operator hold and default hold) and a pane no body drives: exit 3 with `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`, `session-held` and `pane-not-driven`; in each, `create_session` and `select_session` were never called, the record is byte-for-byte unchanged (session of record, SHOWN, DRIVEN, generation), and no prompt was sent. On a healthy pane it is delivered through `HubPort::prompt(Send)` and the harness fake records no prompt. A race (`Check` admits, `Send` is refused) gives exit 3 with the refusal's code, the record stands, and the message says the first message did not land. The same operator hold on `say --pane`, `interrupt --pane` and `answer --pane` keeps exit 4 (Acceptance 18).
8. **Rebind failures.** A refused rebind (body busy, id not on the endpoint, endpoint silent) after the act: exit 1 or 3 by the code's class, `acted: true`, the message ends with the reconcile step, the record unchanged. A busy or questioning driver in the plan: `session-busy` or `session-holds-question` with nothing moved, for switch and reset. `switch` still queries no `session_activity` (E85 stays green). A driver whose `Activity` is `Busy`, `HoldingQuestion` or `WaitingPermission` maps through the same `activity_refusal` as `check_idle` (one test table of the four variants drives both).
9. **The body.** `crates/holler-body/tests/rebind_test.rs` (fake OpenCode server; toggle `exists_v1` and `exists_v2`, since the fake answers one verdict for every id; the fake gains a **stall mode** that never answers the existence check, because the 5 s test needs only that check to hang on the same endpoint): the happy path (presence advertises the new id; the next prompt goes to the new id's `prompt_async`; the name is unchanged), the five refusals of D2, the old binding kept on every refusal, and the 5 s bound with a server that never answers, **including that an attach that finishes after the deadline is not swapped in** (the next prompt still goes to the old id). **One source of the id:** after a rebind, the fake pushes `session.error` (`session_error(id)` through `push_event`) during a turn on the new id; the turn ends in error and the driver is dropped; the **next prompt goes to the new id's `prompt_async`**, the re-attach's existence check is for the new id (the request log has no `GET` of the old id after the rebind), and presence advertises the new id throughout. A unit test in `session_manager/task/rebind_tests.rs` calls `attempt_initial_attach` (the function the 30 s retry timer calls) on an `Inner` whose driver was dropped after a rebind and asserts the same; `query/support` probes the new id (the fake's request log). **A restart reverts:** a second `SessionManager` started over the same configuration advertises the configured id again (it pins the decision of P2). **The first moment:** in a current-thread test, `presence_doc` read right after `SessionManager::start`, before the task has run, already advertises the configured id for an attach row (the mirror is initialised where the presence is built, E143, E144). The hub's `control/rebind` (in `driven_resolve_test.rs` or its own file): takes `token_id` (not `token`), forwards, passes a body refusal through verbatim, refuses a session that is not in the handle's cached presence, and updates `control/roster` and `resolve_prompt_target` before the reply **with a two-session body whose other session's row stays `connected` and keeps its id** (`advertise` replaces the whole set).
10. **`check_only`.** `control/say` with `check_only` and a body that would refuse answers the refusal and one that would admit answers `{admitted: true}`; in both no frame reaches the body, no TalkLog line is written, no turn id moves, no grant is consumed. `talk::say` and the `check_only` branch call the one `admit`: a table of (parked, unhealthy, SHOWN differs from DRIVEN, operator hold, default hold, a held question, a busy session without `queue`) runs through both and they refuse the same cases with the same errors, and `say` refuses a pane-gated prompt before any TalkLog line is written or the turn id moves.
11. **The protocol.** The catalog has 23 rows (`session/rebind`: request, `HubToBody`); the goldens `Rebind.json` and `RebindResult.json` are new files, and so is `envelope/error.not_connected_offline.json` (the `-32004` frame that `Error::offline` builds, `data.reason: "offline"`, from a test beside the `unauthenticated` frames, E165); no existing golden file changes. `scripts/golden-diff-summary.sh` is **not edited**: it is generic and informational, it compares each changed golden file with the base and prints a new file as `(new file)` (E166, E167); its output for this change lists exactly those three files as new and none as a real value change or a reorder. `docs/protocol/v2.md` has the `session/rebind` row and the `-32004` sentence (E184).
12. **Hold behaviour across a reset (D4).** The three tests of D4, with the key `<label>/<session>` asserted equal before and after.
13. **No bypass.** `LiveHandle::query`'s allowlist is exactly `query/*` and `session/answer` (its text and comment unchanged), and `control/query_remote` and `admin/query` with a target, carrying `method: session/rebind`, are refused `-32601` with the body's binding unchanged: a `hold_hub_test`-style test on a real hub and a real body in attach mode against the fake OpenCode server (E100 is the pattern), keeping the fixture realistic, not a stub that refuses everything. A source-scan test (like E77) fails if a second `session/prompt` builder appears in the hub (E77 itself stays green), and fails if `.send_prompt(` is called in `holler-pane/src` or `holler-cli/src` other than inside the body of a `fn send_prompt` that is one of the two pass-throughs D5 names (`wiring/bind.rs`, E127, and `ports/router.rs`, E159), each file having exactly one. **The forward keeps its shape** (a scan in `hold_single_path_test.rs`, beside the one at line 71, E139): `LiveCommand::Query {` is built in one place in `holler-hub/src/live.rs` (cut by `code_of`) and `live/target.rs` (not `live/tests.rs`), the private `forward`, which lies after `pub async fn query(`; `forward(` is called exactly twice, by `query` after its allowlist check and by `rebind` with the literal `"session/rebind"`, and nowhere else in the hub; the existing scan (the allowlist check before the first `LiveCommand::Query` after `pub async fn query(`) stays green unchanged.
14. **One predicate.** The three pane-state cases are decided by `holler_pane::gate::prompt_refusal` for the hub and for `--pane`: a test builds the same records for both and asserts the same code; no other copy of the rule exists in `holler-hub` or `holler-cli` (a source scan for `PANE_SHOWN_DRIVEN_MISMATCH` definitions). The scan also fails on a pair rule written in `roster_cmd.rs` (E154; #648 has merged): it calls `holler_pane::gate::shown_driven_pair_differs`.
15. **Fixtures.** `session_id(n)` has 30 characters and `RoutableName::parse` refuses it; every hub-routed fixture uses it.
16. **Scenario.** `reset_with_a_first_message` is rewritten as D9 says (it is `#[ignore]`d behind `HOLLER_TEST_PANE_INTEGRATION=1`, so CI compiles it and does not run it; the build run compiles it).
17. **Docs.** `docs_pane_control_test` passes with the amended guide; the ADR, the guide, the CHANGELOG and the module docs carry none of the old sentences listed above: a test that reads those files with whitespace normalised finds none of "not use `reset --first` on a parked or held pane" (the ADR's and the CHANGELOG's form), "not use `--first` on a parked or held pane" (the guide's, case-insensitive), "whether it moves to the hub's one prompt path is #828's", "until the cutover wires DRIVEN", "port observes DRIVEN before #649" (case-insensitive), "`reset --first` is not used on a parked or held pane" (the guide's cutover step) or, in `docs/adr/ADR-0021.md`, "stays in force, unchanged". The mentions of #828 that remain are history and are named: ADR item 18's "was #828's, which was closed … as folded into #839" and the new CHANGELOG entry's "absorbs #828". The CHANGELOG has the entry.
18. **A bare id is not a SESSION.** `holler say ses_… TEXT`, `holler interrupt ses_…` and `holler answer ses_… 0` (a `ses_`-shaped id of D8) exit 1 with `unknown session: ses_…` and open no socket (run them with no hub), while `holler say --pane P …` resolves the same id through the hub; `control/say` sent straight to the control socket with the id is resolved (the id is an address at the hub). `Routed.pane` is `Some` only for `--pane`, and a `--pane` hub refusal of `unknown session` or `ambiguous` (`-32003`), or of `not_connected` **with `data.reason: "offline"`**, is `pane-not-driven` (exit 3) in all three verbs, from the one mapping (the `NotDriven` arm is first, ahead of the `is_ambiguous` exit 2). **A turn that outlives its wait is not `pane-not-driven` (P7, three cases):** (i) a `say --pane P --timeout 1s "…"` whose turn does not end in 1 s (a real body against the fake OpenCode server that never ends the turn) exits 1 with the hub's "no reply from … within 1s" and prints no `pane-not-driven`; (ii) an `interrupt --pane P` whose cancel the body does not acknowledge (a body that drops `session/cancel`; the wait is the 2 s floor of `ack_timeout`) exits 1 with the hub's "interrupt sent but not confirmed within Ns", as today; (iii) a `reset --first` whose `Send` is accepted while the turn runs on for more than the 2 s accept wait is never reported as a failure (exit 0), and the call `wiring/hub.rs` builds for `Send` has `params.timeout_ms` equal to the turn budget (600000) and `call.timeout` equal to `QUEUE_ACCEPT_WAIT`, asserted on the call itself. **The wire reason** is pinned at the hub in a new `crates/holler-hub/tests/offline_reason_test.rs` on a real hub: `control/say`, `control/interrupt` and `control/answer` to an offline body answer `-32004` with `data.reason: "offline"`; `say`'s turn timeout and cancel and `interrupt`'s unacknowledged cancel and failed redirect answer `-32004` with no `data.reason`. `hub_refusal::classify` has a table test: `-32003` plain and `ambiguous` and `-32004` with the reason give `NotDriven`; a bare `-32004` gives `Other`; `-32011` pane gives `PaneGate`, operator and default give `Held`; `-32009` gives `Busy`. **The routed verbs keep their holds (P5):** with an operator hold, then a default hold, on the body that drives the pane, `say --pane`, `interrupt --pane` and `answer --pane` still print the held refusal and exit 4 (`HELD_EXIT_CODE`); a pane hold still prints the `PaneHeld` object under `--json`; #646's `every_other_held_error_falls_through_to_the_generic_arm` (E150) is unchanged and green; and `reset --first` on the same operator hold exits 3 with `session-held` (Acceptance 7). (Files: `driven_e2e_test.rs`, `pane_verbs/driven.rs`, `offline_reason_test.rs`, and `hub_refusal.rs`'s own tests.)
19. **The gate sees a body that is behind.** Fakes and a real hub: a body advertising `ses_old`, a pane with session of record and SHOWN `ses_new`: with no recorded DRIVEN a bare `say <name>` is admitted (the stated window); after `pane doctor` records DRIVEN `ses_old` it is refused `pane-shown-driven-mismatch`; after `pane switch PANE ses_new` re-points the body it is delivered. The conformance function (`conformance/hub.rs`, D10) asserts, against `FakeHub` and against the real client over a real hub, that a prompt to a pane's recorded DRIVEN is refused as the hub refuses it, that two panes recording one id are fail-closed, and that the first bad pane in name order is the one named.
20. **Drivers need evidence.** (`driver_of`'s table in `driven_test.rs`; the verbs' tests in `pane_verbs/driven.rs`.) **(a) Unproven, two bodies:** two bodies advertising `ses_a` and `ses_b`, both listed by one server that two panes share (its listing includes the other pane's session of record), neither any pane's record nor any pane's recorded DRIVEN (so the exclusion rule, D3, removes neither): `driver_of` is `Unproven([a, b])`; `pane switch P1 <new>` and `pane reset P1` exit 3 `pane-not-driven`, the message names both `label/session` with their ids and the two steps, and nothing moved (record bytes and generation unchanged, no `create_session`, no `select_session`, no `rebind`); `pane switch P1 ses_a` succeeds without a `rebind` call (rule (iii); record, SHOWN and DRIVEN are `ses_a`); then `pane switch P1 <new>` finds the body by rule (i), calls `rebind` on that body only and records. A target advertised by two bodies is `Ambiguous`, so are two bodies advertising the pane's session of record, and so are two listed bodies on a listing that is not shared; a reset has no target. **(b) The one-body case (the second plan check's block):** P1 has no connected body; P2's body is behind and advertises its configured `ses_cfg2`, which the shared server lists and which no pane's record or recorded DRIVEN names: `driver_of(P1, ..)` is `Unproven([P2's body])`, never `One`; `pane doctor` records P1's DRIVEN `None` and reports it with no remedy; `pane switch P1 <new>` and `pane reset P1` exit 3 `pane-not-driven`, and P2's body is never asked to `rebind` (the fake hub's log has no `rebind`). The same state with P2's body advertising P2's recorded DRIVEN `ses_d2` (the exclusion rule, D3) gives `None` for P1, P1's switch proceeds without a drive step, and `pane switch P1 ses_d2` is refused `session-driven-elsewhere` naming that body (it advertises the target and is not P1's driver); for P2 the same body is found by rule (ii). **(c) The fallback is kept where it is safe:** a pane whose server lists only its own sessions, and one body advertising a listed id that is neither its record nor its stored DRIVEN, is `One` (a reverted body is found and re-pointed by one `pane switch`). **(d) The exclusion rule (D3), as a table in `driven_test.rs`, one row each:** (1) a body on P2's session of record; (2) a body on P2's recorded DRIVEN; (3) a body on P1's own session of record that P2's stale recorded DRIVEN also names; (4) a body on P1's own recorded DRIVEN that P2's record also names. For each row, P1's `driver_of` under every rule (i) to (v): rows (1) and (2) give `None` (nothing remains), rows (3) and (4) give `One`; and for rows (1) and (2) the verbs' outcome for P1's switch onto the body's id is `session-driven-elsewhere`. **(e) Two bodies on one session are refused (P6):** P1's body advertises its record and a second live attach body (another pane's behind body, or an operator's attach row) advertises the switch's target: `pane switch P1 <target>` exits 3 `session-driven-elsewhere` naming that body, and nothing moved; when the body that advertises the target is P1's own driver it is the idempotent re-run (no refusal, no `rebind`). `pane doctor` on (a) gives one `shown-driven-mismatch` finding per pane involved, `not-fixable`, `remedy: None`, DRIVEN recorded `None`.
21. **One request at a time per connection, inside each caller's bound.** On a real hub and a real body: a `control/rebind` and several `query/status` requests (`hub query`) issued at the same moment on one connection each get their own reply (none is reported as not connected) and the rebind applies; `LiveHandle::rebind` times out inside its 8 s bound even when the slot is held by a query that never answers; a `LiveHandle::query` that waits for the slot returns `None` at **its own** `timeout`, not one rebind later (a hub-level test with a short `timeout` on a stalled slot); and the nesting is asserted from the constants: the body's reply wait (6 s) is below the hub's `REBIND_FORWARD` (8 s), which is below `ControlCall::rebind`'s timeout (10 s).
22. **A single-process pane asks the hub nothing (D3 item 8).** On the fakes with a Claude Code pane record (`Shape::SingleProcess`, from the test kit's `FakeHarnesses` and `FakeClaude`): `pane reset PANE` and `pane switch PANE SESSION` run `relaunch_recorded`, make no `hub.*` call, and leave the stored `last_observed.driven` as it was; `pane reset PANE --first` exits 3 `pane-not-driven` with nothing stopped and the record byte-for-byte unchanged (#831's `reset_first_reaches_the_new_process`, E91, becomes this case); `pane doctor` makes no `hub.advertised` observation for the pane and leaves its DRIVEN as stored. (File: `pane_verbs/driven.rs`, and `pane_claude/lifecycle.rs` for the changed test.)

## Blast radius

- `docs/adr/ADR-0021.md`, `docs/adr/ADR-0014.md`, `docs/adr/ADR-0005.md`, `docs/adr/ADR-0003.md`, `docs/protocol/v2.md`, `docs/pane-control.md`, `CHANGELOG.md`, `docs/handoffs/839-brief.md`.
- `crates/holler-proto/src/{rebind.rs (new),lib.rs,methods.rs,error.rs}`, `crates/holler-proto/tests/golden/docs/{Rebind,RebindResult}.json` (new), `crates/holler-proto/tests/golden/envelope/error.not_connected_offline.json` (new), `crates/holler-proto/tests/{golden_test.rs,codec_test.rs}`. Not edited: `scripts/golden-diff-summary.sh` (Acceptance 11).
- `crates/holler-body/src/{connection.rs,connection/session_dispatch.rs,prompt_dispatch.rs,session_manager.rs,session_manager/task.rs,session_manager/task/rebind_tests.rs (new),http_attach_driver.rs,http_attach_driver/connection.rs,registry.rs (doc only)}`, `crates/holler-body/tests/{rebind_test.rs (new),http_attach_driver_test/fake_server.rs (stall mode)}`.
- `crates/holler-hub/src/{circuit.rs (the `Say` arm only),circuit/dispatch.rs,live.rs,live/target.rs (new),live/tests.rs (new: the inline tests of `live.rs`, moved),panes/gate.rs (new),panes/gate_tests.rs (new),panes/mod.rs,talk.rs,interrupt.rs,control_server.rs,control_rebind.rs (new),control_say.rs (new),control.rs,lib.rs}`, `crates/holler-hub/tests/{driven_resolve_test.rs,offline_reason_test.rs}` (new).
- `crates/holler-pane/src/{lib.rs,ports.rs,gate.rs,driven.rs,hub.rs (the last three new),tx_switch.rs,reconcile.rs,reconcile/observe.rs,findings.rs}`, `crates/holler-pane/tests/{driven_test.rs (new),gate_test.rs (new),ports_test.rs}`. Not edited: `ports/router.rs` (its `send_prompt` forward is the second call the scan allows).
- `crates/holler-pane-testkit/src/{lib.rs,fixture.rs,hub.rs (new),conformance/hub.rs (new),conformance/mod.rs}`, and its tests.
- `crates/holler-cli/src/{say_cmd.rs,interrupt_cmd.rs,answer_cmd.rs,prompt_target.rs,pane/wiring.rs,pane/wiring/hub.rs (new),pane/wiring/store.rs (visibility of four items only),pane/hub_refusal.rs (new),pane/reset.rs (doc)}`, `crates/holler-cli/src/roster_cmd.rs` (#648 has merged: its `shown_driven_mismatch` body becomes a call to `holler_pane::gate::shown_driven_pair_differs`, and its doc stops naming the three copies, E154), `crates/holler-cli/Cargo.toml` (the new `[[test]]` `driven_e2e_test`), `crates/holler-cli/tests/hold_single_path_test.rs` (the forward-shape scan), the 20 `Ports` literals listed above (17 files, among them `pane_claude/rig.rs`, `roster_panes_test.rs` and `profile_verbs/rename.rs`), `crates/holler-cli/tests/{driven_e2e_test.rs (new),pane_verbs/driven.rs (new),pane_verbs/reset.rs,pane_verbs/main.rs,support/driven.rs (new),support/mod.rs (one line),pane_integration/scenario.rs,pane_claude/lifecycle.rs (E91: the changed test)}`. `pane/wiring/bind.rs` is **not edited** (its `send_prompt` forward is one of the two calls the scan allows).
- `crates/holler-adapter-opencode/src/prompt.rs` (the module doc line only).
- Not touched: `crates/holler-cli/tests/fixtures/cli-surface.txt` (no CLI flag; no `--json` shape change: `last_observed.driven` is an existing field that now holds a value, and no `FixState` or `FindingKind` is added), `crates/holler-pane-testkit/Cargo.toml` (D8's test lives in `holler-cli`), `crates/holler-hub/src/circuit.rs` beyond the `Say` arm, and the allowlist of `LiveHandle::query`.

## Clashes with stories in flight

State on 2026-10-12 at about 04:00 MDT, re-read for the cut, and again at about 06:40 MDT for the #835 refresh (read only; branches and shas as found; `git log origin/main` and each sibling worktree's own `git log`). **Merged since the last pass:** #835 (`4e691d9`, PR #860, 2026-10-12 06:29 MDT) and, before it, #667 (`0f5835e`, PR #855, 2026-10-11 21:11 MDT), #648 (`037405f`, PR #856, 21:25 MDT), #665 (`8462c9e`, PR #858, 2026-10-12 00:02 MDT) and #831 (`b76f4d2`, PR #857, 00:50 MDT); this branch holds all five. The shared hot spots are `ports.rs`/`Ports` literals, `wiring/bind.rs`, `findings.rs`/`reconcile*`, the ADR's "Deferred" bullets and "Decisions taken", sections 2, 8, 9, 11 and 15 of the ADR, `docs/pane-control.md` and `CHANGELOG.md`.

| Story | Shared files and hunks | Does the DRIVEN decision change what it assumes? |
|---|---|---|
| **#667** `profile apply` scenario (**merged**, `0f5835e`) | `crates/holler-cli/tests/pane_integration/{scenario.rs,guards.rs,main.rs,out.rs,scratch/mod.rs,scratch/verbs.rs}`, `crates/holler-cli/tests/profile_apply_scenario/**`. It added no `Ports { }` literal and uses none of `--first`, `send_prompt` or DRIVEN there. | **Yes, one place, now a fact.** D9's step edits `reset_with_a_first_message`, which is at line 144 of `scenario.rs` on `main` and reads `scratch.session_of_record(C2)` and `scratch.record(C2)` (E86, E155). |
| **#648** roster SHOWN and DRIVEN (**merged**, `037405f`) | `crates/holler-cli/src/roster_cmd.rs` (578 lines), `cli.rs`, `main.rs`, its tests, ADR-0003, ADR-0021, `docs/pane-control.md`, `CHANGELOG.md`. It does not touch `crates/holler-hub/src/roster.rs` (892 lines). | **Yes, one place, now unconditional.** `roster_cmd::shown_driven_mismatch` is the third copy of the pair rule that D1's public `holler_pane::gate::shown_driven_pair_differs` exists to prevent (E154: its doc says "keep the three equal"); #839 replaces its body by a call and corrects the doc (Blast radius; Acceptance 14 scans for it). Its join of a roster row to a pane is by the **label** of the row's name (`pane_label`, E154): that is #648's display rule, not the gate's join (D1). With #803's one local body for the fleet (below) a row is named `<token label>/<pane name>`, so `pane_label` pairs no row with a pane; that display is #648's and #803's to settle and this story does not change it (D12). |
| **#665** `profile rename/export/import`, `profile-drift` (**merged**, `8462c9e`) | `crates/holler-pane/src/findings.rs` (`FindingKind::ProfileDrift` appended, with its `code` and `remedy` arm), `reconcile.rs` (`mod drift;` and one call), `reconcile/drift.rs` (new), `tests/findings_test.rs`, ADR-0003, ADR-0021, `docs/pane-control.md`, `CHANGELOG.md`; also the hub's `profile/*` and `panes/store.rs`. It added a `Ports { }` literal (`profile_verbs/rename.rs`, counted in the 18). | **No change.** Hunks are adjacent, not overlapping: #839 adds `switch_command`, two `Finding` methods and the doc of `remedy` in `findings.rs` (379 lines now), and an observation of `hub.advertised` in `reconcile*`; #665 added an enum arm and a call. `CODES` in `findings_test.rs` was recomputed by #665; #839 adds no kind. |
| **#831** Claude Code harness (**merged**, `b76f4d2`) | `ports.rs` (563 lines at `b76f4d2`, 621 after #835; `HarnessPort` keyed by `at: &HarnessEndpoint`, `Activity::WaitingPermission`, four default-armed process methods), **`ports/router.rs` (new: `HarnessRouter`, whose `send_prompt` forward is `router.rs:118`, E159)**, `tx_switch.rs` (487 lines; the plan and the act split into `switch_in_place` and `relaunched`, E31, E175), `tx_launch.rs` split into submodules (`tx_launch/server.rs` now holds E129), `reconcile*` (a single-process chain, E176), `pane.rs` (`Shape`, E177), `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs` (the `--pane` routes of a Claude Code pane), `wiring.rs`, `wiring/bind.rs`, ADR-0021 (section 8's single-process paragraph E178, section 9's rows, section 15, "Deferred", "Decisions taken" 22 to 28), `docs/pane-control.md`, `CHANGELOG.md`, a `Ports` literal in `pane_claude/rig.rs` (E180). | **Yes, in four places, now facts.** (1) `ports/router.rs` is a second `.send_prompt(` forward, so D5's scan allows it beside `wiring/bind.rs` (Acceptance 13). (2) A single-process pane's switch and reset are a relaunch: D3 item 8 says the plan asks the hub nothing and `--first` is refused `pane-not-driven`; `pane_claude/lifecycle.rs::reset_first_reaches_the_new_process` (E91) becomes "refused, nothing moved" (Acceptance 22) and two passages of ADR text change (the ADR table). (3) `Activity` has a fourth variant, so `activity_refusal` is a total match over four (E118, E119). (4) Its `interrupt --pane` and `answer --pane` for a Claude Code pane never reach the hub (`interrupt_pane`, `answer_pane_target`), so the `NotDriven` arm is reached only for a server pane; `Routed.pane` is set in `interrupt_cmd::route` (E169). Its ADR item 18 already says `say --pane` needs #839 and a body (#850). |
| **#835** import leaves panes it does not drive alone (**merged**, `4e691d9`, PR #860, 2026-10-12 06:29 MDT) | `ports.rs` (621 lines: `HerdrPort::observe_pane` made required, with a `compile_fail` doctest, and `observe_panes` documented as forwarded by a wrapper, E185; the amendments list, E92), `HostPort::foreground_commands` (required since #831's merge, now with its own `compile_fail` doctest), the import (`crates/holler-pane/src/import.rs`, `import/{assess,findings,observe}.rs`, `crates/holler-cli/src/pane/import.rs`: the info finding `harness-claude-code`, the problem `harness-unreadable`, and `--adopt-claude-code`, which is `usage` before any file or port is read), `pane.rs` (a doc on `is_claude_code_command`), `wiring/bind.rs` (a doc comment only: the forwards of `observe_pane` and `observe_panes` were already there; `wiring.rs` is untouched), ADR-0021 (section 2's intro and its `HerdrPort` and `HostPort` rows, section 9's `pane import` row, a "Decided (#835)" block in section 13, section 14's `observe_pane` bullet, "Deferred to named stories", "Decisions taken" items 30 and 31), ADR-0003, `docs/pane-control.md`, `CHANGELOG.md`, `docs_pane_control_test.rs`, `cli-surface.txt`, and two new `Ports` literals (`pane_integration/observe.rs` and a second one in `pane_verbs/import/rig.rs`). It left `launch/rig.rs` and `profile_verbs/apply/rig.rs` alone. | **No change to what it assumes.** It adds no DRIVEN logic and no doctor kind (`harness-claude-code` is a finding of the import's report, not of `findings.rs`); `HubPort` follows its rule (required methods, no defaults), and the compiler now holds a wrapper to it for `observe_pane` too. A pane the import leaves alone has no record, so there is no DRIVEN, gate or driver for this story to look up. **Re-verified on `4e691d9`:** every Evidence command was re-run ("Line numbers re-checked" lists the 28 entries that moved or changed); `wiring/bind.rs` still has exactly one `.send_prompt(` (`bind.rs:352`, inside its `fn send_prompt`, E127) and `ports/router.rs` one (`router.rs:118`, E159), and the only other call in `holler-pane/src` and `holler-cli/src` is the engine's own (`tx_switch.rs:295`, E158), which this story removes; the `Ports` literals are 20 in 17 files (E180). |
| **#647 part 2** `agent-cannot-dispatch`, `unattached` (worktree `0647-agent-cannot-dispatch`, branch `issue-647-implementation`, brief `docs/handoffs/647-part2-brief.md` at `f8d2624`; no code) | `findings.rs` (two kinds, `remedy_for`, `Finding::with_agent`: it may rename or extend `remedy`, beside which #839's doc amendment and `with_switch_remedy` sit, E156), `reconcile.rs`, `reconcile/observe.rs`, `ports.rs` (new harness methods, no `Ports` field), `wiring.rs`, `wiring/bind.rs` (forwarding arms for its new methods; `send_prompt`'s forward is untouched), `pane_integration/{bind,wired,scenario}.rs`, ADR-0021, `docs/pane-control.md` section 5, `CHANGELOG.md`. | **No change to what it assumes** (its brief says it does not depend on DRIVEN). Mechanical; whichever merges second rebases `findings.rs` (the call to `with_fix` then `with_switch_remedy` must survive a rename of `remedy`) and recomputes its own call bound (its chain bound, and #839's thirteen port calls for switch and reset, are different bounds). Its brief lands it after #839. |
| **#803** the setup wizard builds panes through `holler pane` and `holler profile apply` (parked on #853; worktree `0803-wizard-panes`, its brief `docs/handoffs/803-brief.md` at `ccd25df`, 2026-10-12 04:26 MDT, which names #853 and #835 as pending; the shape E181 quotes, at `9af2ad7`, is unchanged in D13 at `ccd25df`) | Files #839 does not edit (the wizard's skill directory, `agent-skills/herdr-setup`, `docs/setup-wizard.md`, the wizard's tests; those tests scan every tracked file for the skill's old directory name, so this brief never spells it, E174). No source-file clash found. Its brief has REFRESH rows for #839 (its D13 and D14). | **One semantic dependency, resolved by its brief.** Its D13 builds **one local body for the whole fleet, with one attach row per OpenCode pane** (`name` = the pane's name, `session_id` = the pane's session of record read from the registry; E181), which is the shape question 2 assumes. Consequences for this story, stated where they bite: a body restart reverts every pane that body drives at once (D2's restart text); the per-connection query slot is fleet-wide (D2 "Hub"); a Claude Code pane gets no row (D6). Its brief also notes that its own text carries a REFRESH row for DRIVEN printing `-`, which it refreshes after #839 merges. |
| **#850** a body driver for a Claude Code pane (worktree `0850-claude-body`, its brief `docs/handoffs/850-brief.md` at `6adf3f9`, 2026-10-12 05:09 MDT; not merged, and it builds on this story) | The body's `session/rebind` handler for a relaunch driver; the roster row of a Claude Code pane. Its D5 (accepted by the operator): the engine's drive step is extended to a single process once a Claude body exists, which #850 does after #839. | Nothing here changes: D6 says a Claude pane is `Driver::None` until #850 lands, D3 item 8, Acceptance 22 and the ADR text say that the refusal holds only until #850, and nothing in this story builds, fakes or assumes a Claude body. |
| **#853** `profile apply` creation order and its exit 3 on a refused step (worktree `0853-apply-grid-order`, branch `issue-853-implementation` at `c1f7deb`, 2026-10-12 06:28 MDT, its cycle 4 round 1; **not merged**) | `crates/holler-pane/src/tx_apply.rs`, `crates/holler-cli/src/profile/apply.rs`, the adapter's `plan_splits_test.rs`, the testkit's `herdr.rs`, the `profile_verbs/apply/*` and `profile_apply_scenario/*` tests (`apply/rig.rs` holds a `Ports` literal: only comments and a constant change there), ADR-0021 (its own contract amendment; it appends "Decisions taken" items 30 and 31, which #835 now holds), ADR-0003, `docs/pane-control.md`, `CHANGELOG.md`. | **No change to what it assumes**: no DRIVEN, no `Ports` field, no `tx_switch.rs`, `reconcile*` or `ports.rs` edit. Mechanical: the numbers of its two "Decisions taken" items are retaken (the next free numbers) by whichever of the two merges second, and the `CHANGELOG.md` and `pane-control.md` hunks are distinct. It gates #803 (parked on it). |

**Shared ADR-0021 and CHANGELOG hunks, so the conflicts are expected and mechanical.** (1) The "Deferred to named stories" DRIVEN bullet: #831 rewrote it (E90) and #839 rewrites it again: take #839's text. (2) The end of "Decisions taken": #831 appended 22 to 28, #665 added 29, #835 added 30 and 31, and #839 appends its own: keep all, number #839's from the next free number (32 on `main` today, E183); #853's branch also appends 30 and 31, so whichever of #839 and #853 merges second renumbers. (3) Section 9's table: #835 edited the `pane import` row (merged), which is the line above the `say`, `interrupt`, `answer` row that #839 edits; #831 edited both rows #839 edits (E61), and the ADR table gives the words to keep. (4) Section 2 and the wiring bullets of section 5: #835 edited section 2's intro sentence and the `HerdrPort` and `HostPort` rows (merged); #839 appends to the same intro sentence (see the ADR table) and adds its own port row. (5) Sections 8, 11 and 15: only #839 edits section 11; in section 8 it edits one sentence of #831's single-process paragraph (E178), and in section 15 three clauses of item 18 (E89). (6) `CHANGELOG.md`: every story adds a bullet at the top of `### Enhancements`; whichever merges later rebases its bullet above the others, and #839 also edits one clause of the #649 item (E179). (7) `docs/pane-control.md`: #839 edits `reset` (with the Claude Code sentence at line 276) and `park` (section 3), the DRIVEN column and "SHOWN against DRIVEN" (section 4), cutover steps 4 to 6 (section 10) and the "Later" list (section 12); #835 edited the same file in other hunks (merged): in section 12 its reworded adoption bullet sits between the two "Later" bullets #839 edits. (8) `findings.rs`, `reconcile.rs` and `reconcile/observe.rs`: #839, #647 part 2 and (merged) #831 and #665 add to them (kinds are appended last; #839 adds no kind, only a command builder, two `Finding` methods and a doc, and the observation of `hub.advertised`); the hunks are distinct.

**Recommended merge order:** #667, #648, #665, #831 and #835 are merged; then **#839**, then #647 part 2 and #850 (each says in its own brief that it lands after #839); #853 is independent of #839 and may merge on either side of it (only the numbers of the "Decisions taken" items and the `CHANGELOG.md` order differ), and #803, parked on #853, merges last. Reasons: #839 adds a field to `Ports` (20 literals) and rewrites the engine's tail, and #835 is now on `main`, so the operator's plan is one fresh run for this story on a tree that holds it. #647 part 2 appends finding kinds and a reconcile call and says in its own brief that it lands after #839; #850 extends the drive step to a single process, which needs #839's drive step to exist. #803 needs a body per pane and reads #839's `pane-not-driven`; it merges last, and its D13 and D14 REFRESH rows are then written once against a DRIVEN that is wired.


## Line numbers re-checked

**At the #835 refresh (2026-10-12, `main` at `4e691d9`, which holds #835, PR #860).** All 185 Evidence commands were re-run by the same script. 157 matched as they were. 20 moved without changing and were re-cut at their new lines (E27 to E29, E33, E55 to E60, E62 to E64, E71 to E74, E89, E118 and E178). 8 changed and were re-cut: E61 (section 9's `pane import` row is #835's), E65 and E90 ("Deferred": the adoption sentence left item 18's bullet for a bullet of its own at the end of the list), E75 (the "Later" bullet on adopting Claude Code panes), E92 (the amendments list now says `observe_pane` lost its default), E180 (20 `Ports` literals in 17 files, from 18 in 16), E183 (items 30 and 31 are #835's, so the next free number is 32) and E185 (`observe_pane` is required now: its excerpt is lines 284-308 and shows `observe_panes` keeping its default). Every size in "Where the code goes" was re-counted: only `ports.rs` changed (563 to 621); `bind.rs` (489) and `wiring.rs` (468) are the same lengths, and `bind.rs` lines 346-353 (E127) match. Sizes the story's `Ports` literals sit beside (`launch/rig.rs` 852, `import/rig.rs` 535, `profile_verbs/apply/rig.rs` 775) were re-counted too.

**At the earlier cut (2026-10-12, `main` at `b76f4d2`).** The 157 Evidence commands of the previous version were re-run on this tree by a script that compares each command's output with its excerpt. 97 matched as they were. 38 moved without changing and were re-cut at their new lines (E20, E22 to E27, E29, E32 to E36, E38, E55, E57, E58, E63, E64, E73, E74, E79, E80, E83 to E85, E96, E120, E121, E124, E125, E128, E130, E137, E142, E148, E149, E153). 22 changed and were re-cut by reading the code: E21, E28, E30, E31, E53, E56, E59 to E62, E65, E71, E72, E75, E76, E86, E118, E119, E123, E127, E129 (it moved to `tx_launch/server.rs`) and E141. Eleven entries that cited a sibling branch's commit now cite `main`, because the story merged: E89 to E91 (#831's ADR text and test), E92 (the amendments list of `ports.rs`), E93 and E154 (#648's roster), E155 (#667's scenario) and E157 (#665's reconcile doc); E156 stays cut from #647 part 2's brief. E158 onward are new. After the re-cut all 157 old entries and the new ones match their commands. What #835 would move was left for one refresh after it merged; the next paragraph is that refresh.

The citations in the prose, re-read on this tree:

| Citation | Now |
|---|---|
| `circuit.rs` 898, `dispatch.rs` 820, `control_server.rs` 837, `live.rs` 884 | true (898, 820, 837, 884; none of the four changed since the last cut) |
| `tests/pane_verbs/switch.rs` ~833, `tests/support/mod.rs` ~849 | **grew**: 893 and 857 |
| `control_server.rs:291-305` (`say`), `talk.rs:203-209`, `live.rs:744-775`, `circuit.rs:737-740` | true |
| `dispatch.rs:83-103` (`pane_state_refusal`), `dispatch.rs:130-133` (the gate's compare) | 87-103 and 134-136 |
| `tx_switch.rs:218` (the engine's `send_prompt` call) | **moved to `tx_switch.rs:295`** (`fn prompt`, E158) |
| `tx_launch.rs:680-688` (E129) | **moved to `tx_launch/server.rs:126-140`**; E128 is `tx_launch.rs:473-484` |
| `ports.rs` line 345 (`send_prompt`'s doc) | **line 491** (E28; 433 before #835 added 58 lines above it); the amendments list is lines 15-41 (E92) |
| `say_cmd.rs:341-372` (`pane_state_refusal`) | 382-413 (E22); `resolve_pane_target` is 325-332 and `route_target_by` 259-284 |
| `bind.rs` `send_prompt` forward | `bind.rs:346-353` (E127), the call itself `bind.rs:352`; the router's forward is `router.rs:118` (E159); no other `.send_prompt(` in `holler-pane/src` or `holler-cli/src` but the engine's own, `tx_switch.rs:295` (E158) |
| `docs/pane-control.md` line 274 (the Claude Code sentence of `reset`) | 276 (E71 is 265-290) |
| `pane_integration/bind.rs:537` (a `Ports` literal) | 650 |
| `holds.rs:429-450` (`note_joined`), `vocab.rs:202-223`, `docs.rs:530-532` (`harness_session_id`), `record_bytes.rs:20` | 435-460, `check_segment` 202-218, true, true |
| ADR-0021 `:823-828`, `:926`, `:480-486`, `:525-526`, `:694`, `:931-934` | moved again at the #835 refresh: section 11 is 925-947; the DRIVEN "Deferred" bullet 1432; section 8's switch and reset paragraph 543-575 and #831's single-process paragraph 577-591; the section 9 rows 810 and 815. **Locate by words.** |
| `docs/protocol/v2.md` line 824 (the `pane/*` note), `-32004` row | 824, 658 (E184) |


## Depends on

Merged: #645, #646, #647 (part 1), #649, #652, #802, #832, #648, #665, #667, #831, #835. Related and not dependencies: #647 part 2, #803, #850, #853 (see "Clashes"). **#654** (the cutover) loses DRIVEN as its own work: its step "run `doctor` until it reports no `shown-driven-mismatch`" becomes a meaningful check, and it must start one attach body for each pane and use `holler pane switch` or `reset` afterwards (the guide edit above says so). **#850** (a body for a Claude Code pane) is the follow-up that gives a Claude pane a driver. The issue #654's body is edited by the operator, not by this story.

## Pipeline

`rigor: in-session` (operator decision, 2026-10-11). Phase order (the repo's pipeline, `uiSurface: false`, so D and U never run): O, A (this brief), T (author, RED), F, T (verify, GREEN), A (anti-duplication), S. Contract amendment first (ADR-0021 and `docs/protocol/v2.md`, one commit), then tests, then code. A fresh worktree, branch `issue-839-implementation`, F on `claude-opus-5-5` at `max` (the repo's table). The branch is merged with `origin/main` at `4e691d9` (which holds #835). One fresh run (operator, 2026-10-12), launched now that #835 has merged; if its plan check blocks a fourth time, #839 is parked again. Needs no real port, live fleet, or real Herdr session; verification is the test kit and scratch instances (a real hub and body against the fake OpenCode server are scratch instances the test starts).

## Rules for this story

See the epic (#633): public repository (no personal host, tailnet or account names), the amend-first rule for the contract, no story touches a live fleet, a real Herdr session, a running pane or a real OpenCode session. A prompt reaches a body only through `send_prompt` in `crates/holler-hub/src/circuit/dispatch.rs`; put control-plane behaviour there, not at call sites. Never run workspace `cargo fmt --all` (it reformats hundreds of files); keep each edit in the style of its file. Edit only the files in the Blast radius. Conventional Commits; no new `unsafe`; clippy clean. Declare every new open code with `RefusalCode::from_static` in the owning file and list it in section 9.

## Questions only the operator can answer

The preparer did not guess these. Each has a default, which the brief above already follows.

1. **Scope: one story, or two?** D2 adds a protocol request, a body change, a hub control method, a new port and a CLI client, and rewrites the engine's tail, in six crates. The brief keeps it one story, in the order contract, tests, code, because the issue says so and the acceptance ("a healthy pane's `reset --first` is delivered") cannot hold without the re-point. The alternative is to split `holler-proto` + `holler-body` + the hub control method (`session/rebind` and `control/rebind`, with the body tests) into a first story, and leave the join, the gate, the port and the verbs for the second. Default: one story.
2. **A fleet fact the repository cannot show.** The design assumes the production fleet runs **one attach-mode body session per registry pane, whose endpoint is that pane's OpenCode server** (the wizard's attach rows, E88, do). If a body hosts several panes' sessions, or a pane is driven in spawn mode, `driver_of` and `session/rebind` still work per session, but the cutover (#654) must start one attach session per pane, and the guide says so. Please confirm the shape before the cutover. #803's brief (D13, E181) builds **one local body for every OpenCode pane, one attach row per pane** with `session_id` read from the registry, which is a body that hosts several panes' sessions: `driver_of` and `session/rebind` work per session, and the consequences are stated in D2 (a restart reverts every pane that body drives at once; the query slot is fleet-wide).
3. **The behaviour change.** `reset --first` on a pane no body drives is **refused** (`pane-not-driven`); today it works through the adapter. That is the price of one gate, and it also removes the only way to send a first message to a Claude pane (#831, merged) until #850. Default: accept it and say so in the CHANGELOG. The alternative (keep the adapter path for panes with no body) is a second enforcement site, which the A review rejected for #828.
4. **A remote rebind.** The rebind is a local control-socket method only (P1); there is no `admin/rebind`, so an admin-role connection (remote, less trusted than the local socket) cannot re-point a body. If remote admins should ever be able to, decide it as an `admin/rebind` through `dispatch_allowlisted`, not as a side effect of an allowlist. Default: no remote rebind.
5. **A pane with no body, on a listing shared with a pane whose body is behind.** After P4 a pane that has no connected body, on a harness server whose listing includes another pane's session of record, is `Unproven` while a listed attach body that no pane's record or recorded DRIVEN names exists (the exclusion rule, D3, removes the bodies that some pane's evidence names): `pane switch` and `pane reset` of that pane are refused `pane-not-driven` (naming the body and the two-step switch) and `doctor` reports it without a remedy, until that body is paired or restarted. This is the cost of never claiming another pane's body; it needs overlapping listings, which question 2's fleet may not have. Default: fail closed, as P4 says. The alternative (proceed without a drive step when no body advertises the pane's own evidence) is the silent wrong pairing the second plan check blocked.

**Answered by the operator (2026-10-12), after the third plan check.** (a) The wire contract: the hub adds an additive, optional `data.reason` to the offline-body `-32004` and `NotDriven` stays (P7, option 1; the alternative was `NotDriven` for `-32003` alone). (b) The exclusion rule: the wider rule, with this pane's evidence first (P8, stated once, in D3). The "golden diff summary" that the operator's decision says is updated is read as Acceptance 11 reads it (item (2) in the next paragraph).

**Taken in this re-cut by the preparer, not by the operator, for the plan check to confirm** (each follows from a decision above; none reopens one): (1) a single-process pane (#831) asks the hub nothing and takes the driver as `None`, so its `reset --first` is refused `pane-not-driven` in the plan and its record keeps its stored DRIVEN (D3 item 8, from D6 and E128); (2) `scripts/golden-diff-summary.sh` is not edited, the new golden file is `envelope/error.not_connected_offline.json` and the script's output is the check (Acceptance 11); (3) the ADR edits reach two passages of #831's merged text (section 8's single-process paragraph and section 15 item 18), which item 18 itself anticipates; (4) the `Send` turn budget is 600 s, the default of `say --timeout`, as `SEND_TURN_BUDGET` (D5); (5) the reason constant and constructor live in `holler-proto`'s `error.rs` (`OFFLINE_REASON`, `Error::offline`), beside `Error::invalid_grant`.

**Decided at the launch (2026-10-11, the main session, under the standing rule to decide and report; the operator can overrule at the plan check).** (1) One story, as the brief keeps it: the issue says so, and a split would leave the first half with nothing to test its re-point against. If the plan check judges the F phase too large, the split line is the one the question names. (2) The default stands. The live fleet's bodies are addressed as `<label>/oc` (one attach session per pane, seen in the operator's own `say` commands), which matches the assumed shape; the cutover (#654) re-confirms it against `holler roster` before relying on it. (3) Accepted: `reset --first` on a pane no body drives is refused `pane-not-driven`, and the CHANGELOG says so.

## Not verified

- The production fleet's body shape (question 2).
- The attach driver's request timeout: the code builds `reqwest::Client::new()` with no timeout (E97), so the 5 s wrapper in D2 is mine; a measured rebind time on a real server was not taken.
- `Roster::advertise` re-advertising a row for a token with a changed `harness_session_id`, with the token's other rows left alone: `handle_presence_notification` calls it for every beat with the whole list, so it should, but `control_rebind` is the one new caller and its two-session test (Acceptance 9) pins it.
- That a `tokio::sync::Mutex` held across `query` and `rebind` (`query_slot`) costs no latency that matters: the longest wait for it is one forward (8 s for a rebind, 10 s for a `hub query`); nothing was measured. Acceptance 21 pins the behaviour, not the timing.
- That a real OpenCode server sends `session.error` the way the fake does (E105 reads the code, not a recording), and that a real restart of a body leaves the pane's server listing the old session (no port deletes sessions; assumed, not observed).
- The ambiguity rule (the target rule of D3) was reasoned from the code, not run: the production fleet may not share a data directory at all (question 2), in which case `Ambiguous` never arises.
- That the fake OpenCode server (E87's pattern) can serve two session ids: it answers one verdict for every id (`exists_v1`), so the rebind tests toggle it.
- #803's brief (`ccd25df`), #850's brief (`6adf3f9`), #853's branch (`c1f7deb`) and #647 part 2's brief (`f8d2624`) were read at the shas named, and none is merged: the clashes table says what each touches. E181 stays cut from #803's brief at `9af2ad7`; D13 is unchanged in substance at `ccd25df`.
- That the optional `data.reason` of `Error::offline` survives every route an error takes to the CLI: `ErrorData.reason` is an `Option<String>` and `Error::new` already sets it for `-32003` (`ambiguous`), so the local control socket carries it, but the remote route (`admin/say` through `holler_body::admin_client`) was not read for a reshaping of `data`. `hub_refusal::classify` fails safe there: a missing reason is `Other` (exit 1, the hub's text).
- The single-process rule (D3 item 8) was reasoned from `tx_switch::switch`, `plan` and `relaunched` (E31, E175) and from D6, not run: no build exists yet.
- That a `say --pane` against a real body whose turn never ends exits at `--timeout` plus the hub's own handling (Acceptance 18 (i)) in about a second: read from `talk::say`'s timeout, not timed.
- The shared-listing signal (rules (iv) and (v) of `driver_of`) was reasoned from `check_unclaimed`'s doc (E137), not observed: whether the production fleet's servers share a data directory is question 2's open fact. If they never do, `Unproven` never arises and the fallback decides every case.
- That no existing body test pins the moment an attach row first appears in `presence_doc` (the reason `omitted_pending_attach` is left where `run` sets it, D2): not searched for.
- That moving `live.rs`'s inline tests to `live/tests.rs` needs nothing but the `mod` line (they use `super::*`): read, not compiled.
- That `session-driven-elsewhere` and the other two new codes pass `RefusalCode`'s well-formedness check as `from_static` constants: they follow the pattern of E96's neighbours (`session-of-other-pane`, E153), and no build was run.

## AI assistance

Drafted by Claude (Sonnet 5.5) for story #839, from the issue, `docs/handoffs/828/handoff-A.md` (on the #828 branch), `main` at `bda1069` and the read-only state of the sibling branches named above, at the operator's direction, per `CONTRIBUTING.md`. The decisions are the preparer's, to be confirmed at the plan check. Amended on 2026-10-11 by Claude (Sonnet 5.5) after the plan check's BLOCK, from `docs/handoffs/839/handoff-A.md` and the main session's decisions P1 to P3, on the tree merged with `origin/main` at `d6ae539`. Amended a second time on 2026-10-11 by Claude (Sonnet 5.5) after the plan check's second pass, from `docs/handoffs/839/handoff-A.md` and the main session's decisions P4 to P6. Re-cut once on 2026-10-12 by Claude (Sonnet 5.5) after the third pass, from `docs/handoffs/839-plan-checks/handoff-A.md`, the operator's decisions of 2026-10-12 (P7, P8) and `main` at `b76f4d2`; every Evidence command was re-run and the new ones were cut by `sed -n` from the files named. Refreshed once on 2026-10-12 by Claude (Sonnet 5.5) for the merge of #835 (`main` at `4e691d9`): the markers were resolved against the merged tree, every Evidence command was re-run again, and no decision was changed.

## Evidence (verbatim)

Cut on 2026-10-11 from `main` at `bda1069`, re-run and re-cut on 2026-10-12 on the tree at `b76f4d2` (`main`, which held #831, #665, #648 and #667) with this branch's docs, and re-run and re-cut once more on 2026-10-12 at the #835 refresh, on the tree at `4e691d9` (`main`, which also holds #835); "Line numbers re-checked" says which entries moved. E158 onward are new on that tree (E181 is a sibling branch's brief at a named commit, as E156 is). Every excerpt below was produced by a command (shown under its heading) that pipes the source through `sed -n 'A,Bp'` (E180 is a `grep`, E181 a `git show`); the line numbers on the left were added by `awk` and are those of the file on that tree (E180's are `grep`'s own). Nothing is retyped.

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

**E20: `crates/holler-cli/src/say_cmd.rs:219-229`** — The three pane codes, declared in the CLI.

Command: `sed -n '219,229p' crates/holler-cli/src/say_cmd.rs | awk -v s=219 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
219	// --- routing by pane (story #646, part 3) ----------------------------------------
220	
221	/// A pane that is parked (`holler pane park`): its prompts are refused until it is
222	/// unparked. The three open codes of the routed verbs are declared here, the file
223	/// that owns the routing (ADR-0021 section 9); each is a refusal, exit 3.
224	pub const PANE_PARKED: RefusalCode = RefusalCode::from_static("pane-parked");
225	/// A pane whose record says its harness server is unhealthy.
226	pub const PANE_UNHEALTHY: RefusalCode = RefusalCode::from_static("pane-unhealthy");
227	/// A pane whose record says it shows one session while the hub drives another.
228	pub const PANE_SHOWN_DRIVEN_MISMATCH: RefusalCode =
229	    RefusalCode::from_static("pane-shown-driven-mismatch");
~~~~

**E21: `crates/holler-cli/src/say_cmd.rs:312-359`** — `resolve_pane_target`: the CLI sends the pane's `session_of_record` as the hub target; the twin predicate is named in its doc.

Command: `sed -n '312,359p' crates/holler-cli/src/say_cmd.rs | awk -v s=312 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
312	/// Resolve `--pane NAME [--profile P]` to the pane's session of record, checking in
313	/// this order: the pane name (`usage`); with `--profile`, the profile scope
314	/// (`ProfileScope::resolve`, so `profile-not-found` and `pane-not-in-profile` come
315	/// from the one helper, as for `pane park`); the record (`pane-not-found`); its session
316	/// of record (`session-not-found`, naming the pane); then what the record says about
317	/// the pane: parked ([`PANE_PARKED`]), unhealthy ([`PANE_UNHEALTHY`], with the reason)
318	/// or SHOWN and DRIVEN both observed and differing ([`PANE_SHOWN_DRIVEN_MISMATCH`]).
319	/// The record is the source; nothing live is probed.
320	///
321	/// The pane-state predicate has a twin at the hub's `send_prompt` gate
322	/// (`holler-hub/src/circuit/dispatch.rs`, `pane_state_refusal`), which also refuses a
323	/// bare `say` to the session; it exists twice only because `holler-pane` is frozen and
324	/// the hub cannot depend on this crate. Keep the two equal.
325	pub fn resolve_pane_target(
326	    ports: Ports<'_>,
327	    pane: &str,
328	    profile: Option<&str>,
329	) -> Result<String, PaneError> {
330	    let record = resolve_pane_record(ports, pane, profile)?;
331	    session_of_record(&record)
332	}
333	
334	/// [`resolve_pane_target`]'s checks, answering the record they passed (#831: `interrupt` and
335	/// `answer` act on the pane's harness by its kind). The record has a session of record.
336	pub fn resolve_pane_record(
337	    ports: Ports<'_>,
338	    pane: &str,
339	    profile: Option<&str>,
340	) -> Result<Pane, PaneError> {
341	    let name = PaneName::parse(pane)?;
342	    let profile = profile.map(ProfileName::parse).transpose()?;
343	    let record = routed_record(ports, &name, profile.as_ref())?;
344	    session_of_record(&record)?;
345	    match pane_state_refusal(&record) {
346	        Some(refusal) => Err(refusal),
347	        None => Ok(record),
348	    }
349	}
350	
351	/// The session of record of `record`: `session-not-found`, naming the pane, when it has none.
352	pub(crate) fn session_of_record(record: &Pane) -> Result<String, PaneError> {
353	    record
354	        .session_of_record
355	        .clone()
356	        .ok_or_else(|| PaneError::SessionNotFound {
357	            what: format!("{} has no session of record", record.name),
358	        })
359	}
~~~~

**E22: `crates/holler-cli/src/say_cmd.rs:382-413`** — `pane_state_refusal` (copy 2 of 2).

Command: `sed -n '382,413p' crates/holler-cli/src/say_cmd.rs | awk -v s=382 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
382	/// What the record says against prompting the pane, in the order parked, unhealthy,
383	/// SHOWN differing from DRIVEN (one side absent is not a mismatch); each message names
384	/// the pane, the code and the remedy on one line (stored text is `quoted`).
385	fn pane_state_refusal(record: &Pane) -> Option<PaneError> {
386	    let name = &record.name;
387	    let doctor = doctor_command(Some(name), false);
388	    let (code, message) = if let Hold::Parked { reason, .. } = &record.hold {
389	        let reason = quoted(reason);
390	        let message = format!("{name} is parked ({PANE_PARKED}, reason {reason}); run holler pane unpark {name} first");
391	        (PANE_PARKED, message)
392	    } else if let Health::Unhealthy(reason) = &record.harness.health {
393	        let reason = quoted(reason);
394	        (
395	            PANE_UNHEALTHY,
396	            format!("{name} is unhealthy ({PANE_UNHEALTHY}): {reason}; run {doctor}"),
397	        )
398	    } else {
399	        let seen = &record.last_observed;
400	        let (Some(shown), Some(driven)) = (seen.shown.as_deref(), seen.driven.as_deref()) else {
401	            return None;
402	        };
403	        if shown == driven {
404	            return None;
405	        }
406	        let (shown, driven) = (quoted(shown), quoted(driven));
407	        let message = format!(
408	            "{name} shows session {shown} but the hub drives {driven} ({PANE_SHOWN_DRIVEN_MISMATCH}); run {doctor}"
409	        );
410	        (PANE_SHOWN_DRIVEN_MISMATCH, message)
411	    };
412	    Some(PaneError::Refused { code, message })
413	}
~~~~

**E23: `crates/holler-cli/src/say_cmd.rs:417-460`** — `QUEUE_ACCEPT_WAIT`, `QueueWait` and `wait_ended`: a read deadline of the accept wait is reported as accepted.

Command: `sed -n '417,460p' crates/holler-cli/src/say_cmd.rs | awk -v s=417 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
417	/// How long the routed `say --pane NAME --queue` waits for the hub before it reports
418	/// the prompt queued. It replaces only the control call's own wait: the prompt's
419	/// `timeout_ms` stays `--timeout`, so the hub keeps waiting for the turn after the CLI
420	/// has returned. A refusal (busy is not one with `--queue`; a hold or the pane gate is)
421	/// answers well inside it.
422	pub const QUEUE_ACCEPT_WAIT: Duration = Duration::from_secs(2);
423	
424	/// What the wait of a `say` ended with.
425	#[derive(Debug)]
426	pub enum QueueWait {
427	    /// The turn finished inside the wait; the control call answered this document.
428	    Reply(serde_json::Value),
429	    /// The CLI's own acceptance deadline ([`QUEUE_ACCEPT_WAIT`]) expired after the
430	    /// prompt was accepted.
431	    Deadline,
432	}
433	
434	/// Whether this `say` returns once its prompt is accepted: the routed (`--pane`)
435	/// `--queue` form over the local control socket. A remote hub (`--server`) cannot tell
436	/// a read deadline from a dropped connection, so it keeps the reply wait; the bare
437	/// `say --queue SESSION` keeps it too (#190's contract).
438	fn returns_on_acceptance(say: &Say) -> bool {
439	    say.queue && say.pane.is_some() && say.server.is_none()
440	}
441	
442	/// The control call's result as a [`QueueWait`]: a reply, or, when the call waited
443	/// only [`QUEUE_ACCEPT_WAIT`] (`returns_on_acceptance`), its read deadline expiring
444	/// with no answer, which means the hub accepted the prompt. Any other error is passed
445	/// on to be printed as a refusal.
446	fn wait_ended(
447	    reply: Result<serde_json::Value, ControlError>,
448	    returns_on_acceptance: bool,
449	) -> Result<QueueWait, ControlError> {
450	    match reply {
451	        Ok(doc) => Ok(QueueWait::Reply(doc)),
452	        Err(ControlError::Io(e))
453	            if returns_on_acceptance
454	                && matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
455	        {
456	            Ok(QueueWait::Deadline)
457	        }
458	        Err(other) => Err(other),
459	    }
460	}
~~~~

**E24: `crates/holler-cli/src/say_cmd.rs:481-535`** — `pane_hold_refusal`: the hub's pane arm of `session_held`, rendered with the pane's remedy, exit 3.

Command: `sed -n '481,535p' crates/holler-cli/src/say_cmd.rs | awk -v s=481 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
481	// --- the pane arm of a held refusal (story #646, part 3) -------------------------
482	
483	/// The `-32011 session_held` of the hub's pane-state gate, as `--json` prints it: the
484	/// held form's object (`hold_cmd::held_refusal`) with `hold_kind: "pane"` and no
485	/// `since`, in this key order.
486	#[derive(Serialize)]
487	struct PaneHeld<'a> {
488	    error: &'static str,
489	    session: &'a str,
490	    reason: &'a str,
491	    hold_kind: &'static str,
492	}
493	
494	/// Render a `-32011 session_held` refusal of `session` whose `data.hold_kind` is
495	/// `"pane"`: the hub's pane-state gate refused the prompt (`data.reason` is the pane
496	/// code). One line naming the code with its remedy (`holler pane unpark` for
497	/// `pane-parked`, `holler pane doctor` otherwise), exit 3 like every pane-routed
498	/// refusal; never the generic held tail nor `HELD_EXIT_CODE`, because `holler release`
499	/// does not lift it. JSON: the held form's object with `hold_kind: "pane"`, on stdout.
500	/// `None` for every other error, which the caller's generic arms then render.
501	pub fn pane_hold_refusal(
502	    session: &str,
503	    e: &holler_proto::WireError,
504	    json: bool,
505	) -> Option<SayResult> {
506	    let data = e.data.as_deref()?;
507	    if !crate::hold_cmd::is_held(e) || data.hold_kind.as_deref() != Some("pane") {
508	        return None;
509	    }
510	    let code = data.reason.as_deref().unwrap_or_default();
511	    let exit_code = ErrorClass::Refusal.exit_code();
512	    if json {
513	        let doc = PaneHeld {
514	            error: "session_held",
515	            session,
516	            reason: code,
517	            hold_kind: "pane",
518	        };
519	        return Some(match serde_json::to_string(&doc) {
520	            Ok(line) => SayResult {
521	                message: line,
522	                to_stderr: false,
523	                exit_code,
524	            },
525	            Err(cause) => err(format!("cannot encode the refusal: {cause}"), exit_code),
526	        });
527	    }
528	    let remedy = if code == PANE_PARKED.as_str() {
529	        "unpark the pane with holler pane unpark"
530	    } else {
531	        "check the pane with holler pane doctor"
532	    };
533	    let message = coded(code, e.message.clone());
534	    Some(err(format!("{message}; {remedy}"), exit_code))
535	}
~~~~

**E25: `crates/holler-cli/src/pane/wiring.rs:126-146`** — `Wiring::ports`: the seven ports.

Command: `sed -n '126,146p' crates/holler-cli/src/pane/wiring.rs | awk -v s=126 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
126	    pub fn connect() -> Result<Self, PaneError> {
127	        store::reach_hub()?;
128	        let actor = Actor::parse(ACTOR)?;
129	        let lookup = |name: &str| {
130	            std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
131	        };
132	        Ok(Self::from_env(&lookup, actor))
133	    }
134	
135	    /// The ports, lent for one verb.
136	    pub fn ports(&self) -> Ports<'_> {
137	        Ports {
138	            pane_store: self.panes.as_ref(),
139	            profile_store: self.profiles.as_ref(),
140	            herdr: &self.herdr,
141	            host: &self.host,
142	            harness: &self.harness,
143	            scope: &self.scope,
144	            prober: &self.prober,
145	        }
146	    }
~~~~

**E26: `crates/holler-cli/src/pane/wiring.rs:241-252`** — `Unwired`, and its `PaneStore` impl (every method `NotImplemented`).

Command: `sed -n '241,252p' crates/holler-cli/src/pane/wiring.rs | awk -v s=241 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
241	/// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
242	#[derive(Debug, Clone, Copy, Default)]
243	pub struct Unwired;
244	
245	impl PaneStore for Unwired {
246	    fn get(&self, _name: &PaneName) -> Result<Option<Pane>, PaneError> {
247	        Err(PaneError::NotImplemented)
248	    }
249	
250	    fn list(&self) -> Result<Vec<Pane>, PaneError> {
251	        Err(PaneError::NotImplemented)
252	    }
~~~~

**E27: `crates/holler-pane/src/ports.rs:90-118`** — `PaneStore`.

Command: `sed -n '90,118p' crates/holler-pane/src/ports.rs | awk -v s=90 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
90	/// The registry of panes, kept by the hub (#639) and faked by the test kit (#638).
91	///
92	/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
93	/// thread) in async code. Every method returns within I5's bound (default 10 s) or
94	/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
95	///
96	/// Every write is a compare-and-swap on the pane's `generation` (see
97	/// [`crate::generation`]); a stale one is `generation-conflict`.
98	pub trait PaneStore: Send + Sync {
99	    /// The pane named `name`, or `None`.
100	    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;
101	
102	    /// Every pane.
103	    fn list(&self) -> Result<Vec<Pane>, PaneError>;
104	
105	    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
106	    /// new pane); returns the stored record with its bumped generation.
107	    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
108	
109	    /// Remove a pane's record if it is still at `expected_generation`. `close`
110	    /// uses it, so a closed pane's record does not outlive the pane. A stale
111	    /// generation is `generation-conflict`; a record that does not exist is
112	    /// `pane-not-found`, whatever `expected_generation` is (a missing record is
113	    /// checked first, so no store has to guess which of the two to answer).
114	    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;
115	
116	    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
117	    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
118	}
~~~~

**E28: `crates/holler-pane/src/ports.rs:490-517`** — `HarnessPort::send_prompt` and `session_activity` and their defaults.

Command: `sed -n '490,517p' crates/holler-pane/src/ports.rs | awk -v s=490 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
490	    /// Queue `text` as a prompt to `session` at `at`, through the harness API, never by
491	    /// typing into a TUI (I4). `pane reset --first` calls it (#645).
492	    ///
493	    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
494	    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
495	    fn send_prompt(
496	        &self,
497	        at: &HarnessEndpoint,
498	        session: &str,
499	        text: &str,
500	    ) -> Result<(), PaneError> {
501	        let _ = (at, session, text);
502	        Err(PaneError::Refused {
503	            code: PROMPT_UNSUPPORTED,
504	            message: "this harness does not queue a prompt".to_owned(),
505	        })
506	    }
507	
508	    /// What `session` at `at` is doing now (#645: `pane reset` refuses a conversation that
509	    /// is not [`Activity::Idle`]). A single process holds one session at a time, so its
510	    /// activity is the process's.
511	    ///
512	    /// The default answers [`Activity::Idle`], so an impl that does not report activity
513	    /// refuses nothing.
514	    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
515	        let _ = (at, session);
516	        Ok(Activity::Idle)
517	    }
~~~~

**E29: `crates/holler-pane/src/ports.rs:610-621`** — `Ports`: the struct every verb holds.

Command: `sed -n '610,621p' crates/holler-pane/src/ports.rs | awk -v s=610 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
610	/// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
611	/// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
612	#[derive(Clone, Copy)]
613	pub struct Ports<'a> {
614	    pub pane_store: &'a dyn PaneStore,
615	    pub profile_store: &'a dyn ProfileStore,
616	    pub herdr: &'a dyn HerdrPort,
617	    pub host: &'a dyn HostPort,
618	    pub harness: &'a dyn HarnessPort,
619	    pub scope: &'a dyn ProfileScope,
620	    pub prober: &'a dyn Prober,
621	}
~~~~

**E30: `crates/holler-pane/src/tx_switch.rs:1-62`** — The module doc of `tx_switch.rs` on `main` (with #831's single-process paragraph): the steps, "`last_observed.driven` is left as stored" and "at most eight port calls".

Command: `sed -n '1,62p' crates/holler-pane/src/tx_switch.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

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
13	//!    operator ([`ORCHESTRATOR_PANE`]). Check the pane's harness live (I6), never from the
14	//!    stored `harness.health` ([`SERVER_UNHEALTHY`]). For a reset of a pane that has a
15	//!    session of record, ask the harness what that session is doing (`session_activity`) and
16	//!    refuse one that is running a turn ([`SESSION_BUSY`]), holds a question
17	//!    ([`SESSION_HOLDS_QUESTION`]) or waits on a permission prompt
18	//!    ([`SESSION_HOLDS_PERMISSION`]); a pane with none is not asked, since reset is doctor's
19	//!    remedy for it. For a switch, check that the server lists the target
20	//!    (`session-not-found`) and that no other record names it as its session of record
21	//!    ([`SESSION_OF_OTHER_PANE`]). That last check is a read, not the authority: the
22	//!    registry's compare-and-swap does not enforce it, so two concurrent switches of two panes
23	//!    to one session can both pass it.
24	//! 2. **Act**: for a reset, `create_session` on the pane's server; then `select_session` in
25	//!    the pane's TUI.
26	//! 3. **Observe**: `shown_session`. A TUI that does not show the target is the closed failure
27	//!    `unavailable` (exit 1; an open code would be a refusal), and nothing is recorded.
28	//! 4. **Record**: the record read in step 1 with exactly four fields set (the target as
29	//!    `session_of_record` and `last_observed.shown`, the request's clock as
30	//!    `last_observed.at`, and `harness.health` healthy, which step 1 observed), in one
31	//!    compare-and-swap at the generation step 1 read, never retried. `last_observed.driven`
32	//!    is left as stored (no port observes DRIVEN before #649), and every other field is the
33	//!    read record's, so a field this file does not know is kept.
34	//! 5. **Prompt**, only when the request carries a first message (`pane reset --first`):
35	//!    `send_prompt` queues it to the target, now the session of record, through the harness
36	//!    API, at the endpoint of the record the run stored.
37	//!
38	//! **A single process (#831, ADR-0021 section 15 item 6).** A pane whose harness's shape is
39	//! `SingleProcess` (Claude Code) has no server to create a session on and no TUI to switch, so
40	//! its switch and its reset are relaunches. The plan refuses as above, except that a switch also
41	//! refuses a pane that is not idle (a relaunch drops the turn in flight) and does not ask for a
42	//! session list: whether the target has a conversation is the relaunch plan's
43	//! (`process_command` with a resume-only choice: `session-not-found`, before anything is
44	//! stopped; plan review W1). Steps 2 to 4 are then `tx_launch::relaunch_recorded`: the pane's
45	//! processes stopped, its process run on the target (switch) or on a fresh session (reset),
46	//! located, seen holding it and recorded with its new pid. Step 5 sends to the new process.
47	//!
48	//! No step of an in-place run calls Herdr or the host, so nothing is ever typed into a TUI
49	//! (I4), and no step serves, attaches, aborts or deletes anything; a relaunch calls them as
50	//! `tx_launch` does, and types nothing either. A failure once `select_session` has been
51	//! called (or once a relaunch entered its act) is not compensated: the record still names the
52	//! previous session, and [`SwitchFailure::message`] ends with the reconcile step (the pane
53	//! doctor command line for the pane, with `--fix`). A failure before that, a reset's
54	//! `create_session` included, moved neither the TUI nor the record, and its message has no
55	//! step. A session a reset created and did not record stays on the server, and the message
56	//! names it. A failed prompt comes after the record: the TUI and the record agree on the
57	//! target, so its message has no step, and says that the session was created, shown and
58	//! recorded but the first message did not land. After a successful reset the previous session
59	//! stays on the server too: no port deletes a session, so doctor reports it as a stray.
60	//!
61	//! An in-place run makes at most eight port calls, each bounded by I5, so it needs no budget of
62	//! its own; a relaunch runs under `tx_launch`'s budget.
~~~~

**E31: `crates/holler-pane/src/tx_switch.rs:199-215`** — `switch`: the engine's tail on `main`: plan, then the act by the pane's harness shape, then `prompt` for `reset --first` (the call that goes straight to `HarnessPort::send_prompt` is E158).

Command: `sed -n '199,215p' crates/holler-pane/src/tx_switch.rs | awk -v s=199 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
199	
200	/// Run one switch or reset over `ports` (see the module docs). `Ok` once the target was seen
201	/// shown (by the TUI, or held by the relaunched process) and the record names it.
202	pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
203	    let record = plan(ports, request)?;
204	    let pane = match record.harness.kind.shape() {
205	        Shape::ServerAndTui => switch_in_place(ports, request, &record)?,
206	        Shape::SingleProcess => relaunched(ports, request, &record)?,
207	    };
208	    if let Some(text) = &request.first {
209	        prompt(ports, &pane, text)?;
210	    }
211	    Ok(Switched {
212	        pane,
213	        previous: record.session_of_record,
214	    })
215	}
~~~~

**E32: `crates/holler-pane/src/tx_switch.rs:477-487`** — `recorded`: four fields set; `last_observed.driven` is the read record's.

Command: `sed -n '477,487p' crates/holler-pane/src/tx_switch.rs | awk -v s=477 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
477	/// The record a run writes: `record` with `target` as its session of record and as what
478	/// its TUI was seen showing at `now_ms`, and its server healthy. Every other field, and
479	/// `last_observed.driven`, is `record`'s.
480	fn recorded(record: &Pane, target: &str, now_ms: i64) -> Pane {
481	    let mut next = record.clone();
482	    next.session_of_record = Some(target.to_owned());
483	    next.last_observed.shown = Some(target.to_owned());
484	    next.last_observed.at = now_ms;
485	    next.harness.health = Health::Healthy;
486	    next
487	}
~~~~

**E33: `crates/holler-pane/src/pane.rs:372-383`** — `LastObserved { shown, driven, at }`.

Command: `sed -n '372,383p' crates/holler-pane/src/pane.rs | awk -v s=372 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
372	/// What reconcile last observed about which session the pane shows and which the hub
373	/// drives. Written by reconcile, never inferred.
374	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
375	#[serde(deny_unknown_fields)]
376	pub struct LastObserved {
377	    #[serde(default, skip_serializing_if = "Option::is_none")]
378	    pub shown: Option<String>,
379	    #[serde(default, skip_serializing_if = "Option::is_none")]
380	    pub driven: Option<String>,
381	    /// When it was observed (milliseconds since the Unix epoch).
382	    pub at: i64,
383	}
~~~~

**E34: `crates/holler-pane/src/reconcile.rs:45-59`** — reconcile's module doc: "`last_observed.driven` is left as stored. No port observes DRIVEN before #649".

Command: `sed -n '45,59p' crates/holler-pane/src/reconcile.rs | awk -v s=45 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
45	//! **What the record write means.** Other verbs gate on these fields (#645, #646, #648):
46	//!
47	//! - `harness.health` is written from this pass's health check: `healthy`,
48	//!   `{"unhealthy": "server-wedged"}` or `{"unhealthy": "server-down"}`. A failed check
49	//!   leaves it as stored.
50	//! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
51	//!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
52	//! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
53	//!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
54	//!   the session the hub drives.
55	//! - A record is written only when one of those values changed, or on its first
56	//!   observation (`at == 0`): the hub's rule for a periodic writer (`panes/mod.rs`). So
57	//!   `at` is when the current values were first observed, not a freshness stamp. A write
58	//!   that loses a race with another writer is reported and never retried: that writer's
59	//!   record wins.
~~~~

**E35: `crates/holler-pane/src/reconcile.rs:205-215`** — `shown_differs`, the one comparison of SHOWN with the session of record.

Command: `sed -n '205,215p' crates/holler-pane/src/reconcile.rs | awk -v s=205 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
205	/// Whether a pane whose session of record is `session_of_record` shows something else: the
206	/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
207	/// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
208	/// no mismatch (it is `no-session-of-record`).
209	///
210	/// The one form of this comparison: a pass calls it with what the TUI shows now, and a
211	/// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
212	/// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
213	pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
214	    session_of_record.is_some_and(|record| shown != Some(record))
215	}
~~~~

**E36: `crates/holler-pane/src/reconcile/observe.rs:262-308`** — `check_screen`: `shown-driven-mismatch` is SHOWN against the session of record today.

Command: `sed -n '262,308p' crates/holler-pane/src/reconcile/observe.rs | awk -v s=262 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
262	/// The TUI's rules: `tui-foreign-session` when it shows a session its pane's own healthy
263	/// server does not have, or else `shown-driven-mismatch` when it shows another than the
264	/// session of record, repaired by [`repair`] when the record decides it. Returns the screen
265	/// at the end of the chain: after an attempted fix, what the TUI showed then.
266	fn check_screen(
267	    ports: Ports<'_>,
268	    request: &ReconcileRequest<'_>,
269	    pane: &Pane,
270	    server: &Server,
271	    screen: Screen,
272	    findings: &mut Vec<Finding>,
273	) -> Screen {
274	    let Screen::Seen(shown) = &screen else {
275	        return screen;
276	    };
277	    let shown = shown.as_deref();
278	    if let (Some(session), Some(ids)) = (shown, server.sessions()) {
279	        if !ids.iter().any(|id| id == session) {
280	            findings.push(foreign_session(pane, session));
281	            return screen;
282	        }
283	    }
284	    let Some(record) = pane.session_of_record.as_deref() else {
285	        return screen;
286	    };
287	    if !shown_differs(Some(record), shown) {
288	        return screen;
289	    }
290	    let repair = repair(ports, request, pane, server, record);
291	    let tense = if repair.state == FixState::Fixed {
292	        "showed"
293	    } else {
294	        "shows"
295	    };
296	    let message = format!(
297	        "the TUI {tense} {}, not the session of record {}",
298	        screen_text(shown),
299	        quoted(record)
300	    );
301	    let finding = Finding::about(FindingKind::ShownDrivenMismatch, pane, message);
302	    findings.push(
303	        finding
304	            .with_session(shown)
305	            .with_fix(repair.state, repair.error),
306	    );
307	    repair.screen.unwrap_or(screen)
308	}
~~~~

**E37: `crates/holler-pane/src/findings.rs:68-70`** — The finding kind's doc.

Command: `sed -n '68,70p' crates/holler-pane/src/findings.rs | awk -v s=68 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
68	    /// The TUI shows another session than the session of record, or its home screen. The
69	    /// one kind `--fix` repairs: it selects the session of record.
70	    ShownDrivenMismatch,
~~~~

**E38: `crates/holler-pane/src/findings.rs:163-169`** — The mismatch's remedy table.

Command: `sed -n '163,169p' crates/holler-pane/src/findings.rs | awk -v s=163 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
163	            FindingKind::ShownDrivenMismatch => match fix {
164	                FixState::Fixable | FixState::Skipped => {
165	                    pane.map(|pane| doctor_command(Some(pane), true))
166	                }
167	                FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
168	                FixState::Fixed => None,
169	            },
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
3	**Status:** accepted
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

**E55: `docs/adr/ADR-0021.md:319-328`** — The invariants table: I2 and I5.

Command: `sed -n '319,328p' docs/adr/ADR-0021.md | awk -v s=319 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
319	### 4. Ratified: the invariants, each with a test idea
320	
321	| | Invariant | One-line test idea |
322	|---|---|---|
323	| I1 | Only pane verbs change a pane or its registration. | Every adapter call in a scenario is made from a verb's code path: the fakes record their caller, and the hub's `pane/*` handlers hold no adapter (no adapter crate in `holler-hub`'s manifest). |
324	| I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
325	| I3 | Every verb is plan, act, observe, record; a mismatch fails loudly and records nothing. | Make the fake TUI show a different session after `act`: the verb exits 1 with a code, and the stored record and its generation are unchanged. |
326	| I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the switch and reset tests fail if any keystroke was sent (#645). |
327	| I5 | A verb returns in bounded time (default 10 s); long work returns an operation id. | A wedged fake port makes the verb exit with `timeout` within the bound plus a margin; the operation-id half is not built (section 12, decided 2026-10-11). |
328	| I6 | State is observed from Herdr, tmux and the harness, never inferred from files. | Give the importer and reconcile a stale registration file that names session A while the fake harness shows B: no record gets A as observed state. |
~~~~

**E56: `docs/adr/ADR-0021.md:339-354`** — Section 5, "Wiring as built (#649)": the first bullets.

Command: `sed -n '339,354p' docs/adr/ADR-0021.md | awk -v s=339 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
339	**Wiring as built (#649).** `Wiring::connect` builds the ports of a run, once per run, and every pane and profile verb runs
340	over them, as do `say`, `interrupt` and `answer` with `--pane`, whose pane resolution calls `connect` (`say_cmd.rs`), and
341	the roster (#648), which calls `connect` only after its hub call has answered and never with `--server`, and reads the
342	pane store or, with `--profile`, the profile scope, and no adapter.
343	
344	- **`connect` needs the hub and nothing else.** It makes one `control/status` exchange (the control client's own 5 s), and
345	  any failure of it is `unavailable` naming the hub and its state dir, so with no hub every verb exits 1 having touched no
346	  adapter. It builds the two registries' clients, the Herdr adapter, the tmux host, the OpenCode harness, `SystemProber`, and
347	  `StoreScope` over the two clients as the `ProfileScope`. The Herdr adapter pings Herdr when it is built, so it is built on
348	  its first use and kept, a failed build included; building the host or the harness does no I/O. So `pane list`, `get` and
349	  `watch` and `profile list` and `show` run with no Herdr, tmux or OpenCode present.
350	- **The registries' clients** (`wiring/store.rs`) send each call as one control-socket request through the CLI's one dispatch
351	  point (`transport::call`), with the params of `holler_pane::reply` and a `PaneReply` back, within 10 s, or within the
352	  long-poll window plus 5 s for a `*/watch` request. A failed exchange of a read is `timeout` naming the store method when
353	  the hub took the request and did not answer within the bound, which is section 2's own rule, and `unavailable` naming the
354	  hub otherwise. A write whose reply is lost is `timeout`, since it may have landed (section 8, step 2). A JSON-RPC error is
~~~~

**E57: `docs/adr/ADR-0021.md:380-391`** — Section 5: the bullets "What the wiring makes live" and "Not done here" (the #828 and #654 sentences).

Command: `sed -n '380,391p' docs/adr/ADR-0021.md | awk -v s=380 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
380	- **No new bound.** Each adapter and each registry request keeps its own 10 s bound (I5), and each verb its budget; there is
381	  no flag, option or variable for a bound.
382	- **What the wiring makes live.** Every profile write through the scope is logged under the one actor `holler pane`: a
383	  `--profile` edit of `launch`, `relaunch` or `close`, and the creates and relaunches of `profile apply`, so the profile log
384	  cannot tell those verbs apart. A per-verb actor needs a seam other than the argument-less `connect` (#825). And since #642
385	  part 3 the first message of `pane reset --first` is a real prompt through `HarnessPort::send_prompt`, outside the hub's
386	  prompt path and so outside its park and hold gate (section 8, "The prompt and activity seam"): this wiring makes that path
387	  live, and closing it is #828's. Until #828 lands, the operator does not use `reset --first` on a parked or held pane.
388	- **Not done here.** No `HostPort` amendment and no credential handling: a provider credential reaches a harness server only
389	  through its tmux server's environment, and how production sets that is the cutover's (#654). The hub's DRIVEN session
390	  is not pointed at `session_of_record` (section 11, #654). `--herdr-session` is not defaulted from the configured session
391	  (section 8, "Launch and relaunch as built (#644)"). The pane's model, effort and environment still do not reach its
~~~~

**E58: `docs/adr/ADR-0021.md:413-419`** — Section 6: the pane and profile methods are control-socket methods, "the closed 22-row catalog".

Command: `sed -n '413,419p' docs/adr/ADR-0021.md | awk -v s=413 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
413	### 6. The hub control-socket methods and their replies
414	
415	**Decided (ruling 6): `pane/*` and `profile/*` are hub control-socket methods, not v2 wire methods.** They sit outside the
416	closed 22-row catalog (`holler_proto::methods::CATALOG`), are not reserved in [protocol v2](../protocol/v2.md) §4, and a body
417	connection that sends one still gets `method_not_found`. The names are:
418	
419	- `PANE_METHODS`: `pane/get`, `pane/list`, `pane/cas_put`, `pane/delete`, `pane/watch`.
~~~~

**E59: `docs/adr/ADR-0021.md:543-575`** — Section 8, "Switch and reset as built (#645)".

Command: `sed -n '543,575p' docs/adr/ADR-0021.md | awk -v s=543 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
543	**Switch and reset as built (#645).** One engine, `holler_pane::tx_switch`, runs both verbs: switch moves a pane to a
544	session its harness server already has, and reset to one the run creates. Its plan reads the record (through the profile's
545	scope with `--profile`) and refuses before anything is written or moved: the orchestrator's pane unless the operator passes
546	`--as-operator`, a statement of intent and not authentication (`orchestrator-pane`); a harness server that does not answer
547	its health check, observed live and never read from `harness.health` (`server-unhealthy`, naming the relaunch remedy); for
548	reset of a pane that has a session of record, a conversation that `HarnessPort::session_activity` reports running a turn
549	(`session-busy`) or holding a question (`session-holds-question`), while a pane with none is not asked, since reset is
550	doctor's remedy for it; and, for switch, a target the pane's own server does not list (`session-not-found`) or that is
551	another pane's session of record (`session-of-other-pane`). The act is `select_session` in the pane's TUI, after
552	`create_session` for reset, and the observation is `shown_session`. **A mismatch observed after the act is the closed
553	failure `unavailable`, exit 1:** I3 needs a failure, and an open code is a refusal (section 9). A mismatch records nothing.
554	Otherwise the record the plan read is written with exactly four fields set (`session_of_record` and `last_observed.shown` to
555	the target, `last_observed.at` to the run's clock, `harness.health` to healthy) in one compare-and-swap at the generation the
556	plan read, never retried; `last_observed.driven` is left as stored (section 11). Then, only for `reset --first TEXT`,
557	`HarnessPort::send_prompt` queues `TEXT` to the new session, now the session of record (I2), so the first message can land
558	nowhere else. A prompt that fails after the record is the port's own error (its class decides the exit), and its message
559	says the session was created, shown and recorded but the first message did not land; it has no reconcile step, because the
560	record is correct. No step calls Herdr or the host (I4; a single process's switch and reset are a relaunch, below), and a
561	run makes at most eight port calls, each bounded by I5 (section 12). Neither verb checks the pane's park state; a refusal
562	derived from pane state belongs to the prompt gate of #646, which `reset --first` does not pass through (see the seam
563	below). A failure after the act and before the record is not compensated: the record still names the previous session,
564	and the message ends with the reconcile step, the pane doctor command line for the pane with `--fix`, which selects that
565	session again. That step follows every such failure once `select_session` has been called, a record conflict or a
566	`timeout` included; a failure before then has moved neither the TUI nor the record, and its message has no step.
567	Whether or not the verb ran with `--profile`, the step names the pane, unlike the two forms of step 6 that the generations
568	rule above gives. Those name no pane because a failed launch can leave a pane with no record, which a pane-scoped doctor
569	refuses. A switch or reset reads the pane's record before it acts, so a doctor run for that pane is not refused, and
570	`--fix` is the pane doctor's own remedy for a TUI that does not show the session of record, while a fix can repair it.
571	Neither verb edits P's specs, so no `profile show` is needed. A session that reset created and did not record stays on the
572	server, and the message names it. After a successful reset the previous session also stays on the server, and the pane
573	doctor reports it as a stray session: no port deletes a session. The session-of-other-pane check is a read in the verb, not
574	a rule of the registry, whose compare-and-swap enforces only the generation and the membership rule, so two concurrent
575	switches of two panes to one session can both pass it and both record that session.
~~~~

**E60: `docs/adr/ADR-0021.md:593-620`** — Section 8, "The prompt and activity seam".

Command: `sed -n '593,620p' docs/adr/ADR-0021.md | awk -v s=593 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
593	**The prompt and activity seam (#645 part 2, the operator's carve-out).** `HarnessPort` grows two default-armed methods,
594	the first amendment since #637 froze it (the second is #802's, section 2): `send_prompt(port, session, text)` and
595	`session_activity(port, session)`, which
596	answers the closed `Activity` (`Idle`, `Busy`, `HoldingQuestion`, and since #831 `WaitingPermission`; in `ports.rs`, never on
597	the wire). A default keeps every
598	impl that does not override it as it was: `session_activity` answers `Idle`, so nothing is refused where no port reports
599	activity, and `send_prompt` refuses with the open code `prompt-unsupported` (exit 3). That code departs from the
600	`not-implemented` the skeleton answers for unlanded methods on purpose: by the time `reset --first` sends the prompt the
601	reset itself has succeeded, so "understood and declined" (a refusal) is the honest class, not a runtime failure. The OpenCode
602	adapter replaces both defaults (#642 part 3), and the test kit's fake records them (#684). Its `send_prompt` asks `GET
603	/session/:id` first, as its `abort` does (an unknown session is `session-not-found` and nothing is posted), then posts
604	`{"parts":[{"type":"text","text":TEXT}]}`, naming no agent, to the bare `POST /session/:id/prompt_async`, the route and body
605	`holler-body` sends; it is `Ok` exactly on that route's `204`, and its message for any other reply names the route, the port
606	and the status and quotes no part of the reply, which could echo the prompt. Its `session_activity` is `HoldingQuestion`
607	while `GET /question` or `GET /permission` lists an item of the session or of a direct child (subagent) session of it, the
608	scope `holler-body` reads: a pending permission request, the session's own or a direct child's, reads as `HoldingQuestion`,
609	since the turn waits on an answer either way. A list that cannot be read holds nothing (a turn that waits on an answer still
610	runs, so its status reads `Busy`). Otherwise it is `Busy` while `GET /session/status` has an entry for the session whose
611	`type` is not `idle` (`busy`, `retry`, or none), else `Idle`; only a server whose status route answers no JSON object is read
612	from its last assistant message instead (`Busy` while it has no `time.completed`). Its `abort` reads that route the same
613	way, so an abort of a session OpenCode is retrying is not settled until the session's entry is gone or `idle`. Two choices
614	differ from what this ADR had proposed for 645b, and #646 and #649 build on them rather than beside them: the first message
615	goes from the verb straight to `HarnessPort::send_prompt`, not through the hub's one prompt path (so the prompt gate of #646,
616	park state included, does not see it); and the activity read is of the pane's session of record, not of its DRIVEN session,
617	which is not kept in step with it until the cutover wires DRIVEN (#654, section 11). Since #642 part 3 the adapter sends
618	that first message for real, and the gate still does not see this path; the wiring (#649) makes it live, and whether it
619	moves to the hub's one prompt path is #828's (section 5, "Wiring as built (#649)"). The adapter's `send_prompt` stays pure transport: a
620	hold, a limit or a park belongs at the hub's choke point, not in an adapter.
~~~~

**E61: `docs/adr/ADR-0021.md:810-815`** — Section 9: the verb rows for switch, reset and the routed prompt verbs (the `pane import` row above the routed verbs' row is #835's; this story does not edit it).

Command: `sed -n '810,815p' docs/adr/ADR-0021.md | awk -v s=810 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
810	| `pane switch`, `pane reset` | `pane-not-found`, open (#832) `pane-unmanaged`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645): `orchestrator-pane`, `server-unhealthy`, for `switch` `session-of-other-pane`, and for `reset` a session of record that is running a turn or holds a question (`session-busy`, `session-holds-question`) and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default); open (#831, section 8) for a Claude Code pane, which is relaunched: `session-busy` and `session-holds-question` for `switch` too, `session-holds-permission` for both, the relaunch's codes (`harness-dialog-open`, `timeout`, ...), and with `--first` the port's `session-mismatch` and `inbound-held` |
811	| `pane park`, `pane unpark` | `pane-not-found`, open (#832) `pane-unmanaged`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
812	| `pane close` | `pane-not-found`, open (#832) `pane-unmanaged`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-not-in-profile` (with `--profile`, P holds no spec for the pane) |
813	| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` and, with `--unmanaged-stalled-after SECONDS`, `unmanaged-stalled` (#832, section 14), and for a Claude Code pane `pane-waiting` and, about no pane, `claude-login-missing` (#831, section 15 item 13); `--fix` of a Claude Code pane's mismatch is a relaunch, with the relaunch's codes |
814	| `pane import` | `usage` (`--adopt-claude-code` until #831's adoption step lands (section 15, item 17), before any file or port is read (#835); no `--herdr-session`, a `--from` that is not a directory, an `--orchestrator` that names no pane directory), `herdr-version-unsupported` (from `herdr.version()`, which ends the run with no report), `profile-exists` (a real run with a pane to record while a profile `fleet` exists), the code of a failed write whose undo completed (`generation-conflict`, `timeout`, ...), `profile-conflict` (an undo step failed; the message names the commands that reconcile); open (#650): `import-blocked` (a real run while a pane has a problem, `harness-unreadable` included; nothing is written). `command-not-argv`, `session-not-found`, every disagreement, and a pane left alone because its tmux session runs Claude Code (`harness-claude-code`, #835, section 13) are findings of the report, not errors (#650) |
815	| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found` (no session of record); open (#646), as the record says: `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; all exit 3, the code in the message. The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"`. For a Claude Code pane (#831, section 15 item 9): `interrupt` is a relaunch on the session of record, with the relaunch's codes, and with text `interrupt-text-unsupported`; `answer` is `answer-unsupported` |
~~~~

**E62: `docs/adr/ADR-0021.md:871-874`** — Section 9: the closed and open code rows.

Command: `sed -n '871,874p' docs/adr/ADR-0021.md | awk -v s=871 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
871	  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
872	  | `profile-drift` | Failure (1) | A finding kind, not an error a port returns; if one surfaces, live state disagrees with its spec. Raised by reconcile since #665. |
873	  | any other well-formed code (open, raised as `Refused`) | Refusal (3) | `Refused` is the channel for the codes a verb or adapter owns; the open codes so far (#640's `grid-unreachable`, #645's `session-busy`, `session-holds-question` and `prompt-unsupported`, #646's `pane-parked`, `pane-unhealthy` and `pane-shown-driven-mismatch`, all merged, #802's `agent-unsupported`, #642's `agent-unusable`, the OpenCode adapter's, #650's `import-blocked`, and #648's `remote-policy-refused`, the roster's `--format=json` code for a `--server` the transport refuses before any dial, ADR 0003's exit 3) are all refusals. |
874	  | a malformed code | Failure (1) | It cannot be trusted: the same rule turns a garbled wire reply into `unavailable`. |
~~~~

**E63: `docs/adr/ADR-0021.md:925-947`** — Section 11, in full.

Command: `sed -n '925,947p' docs/adr/ADR-0021.md | awk -v s=925 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
925	### 11. How attach mode (ADR 0014) relates
926	
927	[ADR 0014](ADR-0014.md) stays in force, unchanged. What changes is **who chooses the session**. For a pane in the registry,
928	Holler creates the session of record: the pane verbs start the harness server (the argv `HarnessPort::serve_command`
929	builds, which `HostPort::run` starts in the pane's tmux session, #802), create the session over its API (`create_session`)
930	and attach the TUI to it (`attach_tui`). The body that drives that session is still not its
931	parent, so ADR 0014's teardown rule (point 3) and its rule against typing into a PTY (point 4, which I4 restates for pane
932	verbs) apply as they are. The difference is that the session id comes from `session_of_record`, which a verb writes after it
933	observes the session. It is no longer pasted by an operator or guessed by a repair. That answers ADR 0014's open "automatic
934	discovery of attachable sessions (e.g. from a Herdr pane)" for registered panes: there is no discovery; the registry names
935	the session. Attach mode with an operator-written `endpoint` and `session_id` stays for sessions Holler does not own (a
936	session with no pane record).
937	
938	`HerdrPort::send_text` and `send_keys` exist in the provisional port. Under I4 no verb may use them to change which
939	session a pane shows or what it is doing; #640 uses them only in its tests, which type into panes they created in a scratch
940	Herdr session, #646 names the permitted uses, and #645's tests fail on any keystroke.
941	
942	**Deferred to #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each registered pane,
943	and when it moves after a switch or reset. The wiring (#649) wires the ports and leaves DRIVEN to the cutover.
944	Until the cutover wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the
945	hub drives, and leaves `last_observed.driven` as stored.
946	A switch or reset (#645) likewise leaves `last_observed.driven` as stored; the hub's DRIVEN session follows the new
947	`session_of_record` once the cutover wires it.
~~~~

**E64: `docs/adr/ADR-0021.md:994-995`** — Section 13: the sentence about the hub's own registration.

Command: `sed -n '994,995p' docs/adr/ADR-0021.md | awk -v s=994 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
994	The hub's own registration, the fifth record of the incident, is replaced by `session_of_record` once the cutover (#654)
995	points the hub's DRIVEN session at it (section 11).
~~~~

**E65: `docs/adr/ADR-0021.md:1430-1443`** — "Deferred to named stories": the DRIVEN bullet and the `reset --first` bullet. #835 took the sentence on adopting Claude Code panes out of the second bullet and made it a bullet of its own at the end of the list (out of this range).

Command: `sed -n '1430,1443p' docs/adr/ADR-0021.md | awk -v s=1430 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1430	## Deferred to named stories
1431	
1432	- How the hub's DRIVEN session follows `session_of_record`: #839 (it was #654's; the wiring, #649, leaves it to the cutover).
1433	- A body that lets `holler say --pane NAME TEXT` reach a Claude Code pane: #850 (section 15, item 18). `pane-stalled` waits
1434	  with #837's time source, and the `PreToolUse` gate, a permission-mode, allowed-tools and questions spec field and `--agent`
1435	  for Claude are later stories (section 15).
1436	- The exact file layouts of `panes.json` and `profiles.json`, the long-poll window, and the feed's retained window: #639 and
1437	  #661.
1438	- Which closed failure code a mismatch observed after `act` carries: decided, `unavailable` (exit 1), section 8: #644 for
1439	  launch and relaunch; #645 for switch and reset.
1440	- The refusal of a pane that is not idle or holds a question, and `reset --first`: decided, #645 part 2, through two
1441	  default-armed `HarnessPort` methods (section 8, "The prompt and activity seam"). Their real implementation is built, in
1442	  the OpenCode adapter (#642 part 3); the fake's recording is #684's; the wiring (#649) makes the first message's path live,
1443	  and whether it moves to the hub's one prompt path is #828's.
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

**E67: `docs/protocol/v2.md:514-534`** — Section 7, presence: `harness_session_id` "present for `attach`".

Command: `sed -n '514,534p' docs/protocol/v2.md | awk -v s=514 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
514	## 7. Presence
515	
516	`session/presence` is a **notification** (body→hub), sent periodically as
517	a heartbeat and on every (re)connect:
518	
519	```json
520	{"jsonrpc":"2.0","method":"session/presence","params":{"hostname":"kiwi","sessions":[
521	  {"name":"alpha","harness":"opencode","state":"idle","mode":"spawn"},
522	  {"name":"beta","harness":"opencode","state":"working","mode":"spawn","harness_session_id":"openc-abc",
523	   "turn_started_at":"2026-09-08T12:00:00Z","last_update_at":"2026-09-08T12:00:03Z",
524	   "turn_id":"h-01H...",
525	   "last_turn":{"turn_id":"h-01H...prev","state":"completed","stop_reason":"end_turn","ended_at":"2026-09-08T11:59:50Z"}},
526	  {"name":"gamma","harness":"opencode","state":"input-required","mode":"spawn",
527	   "pending":[{"id":"perm-1","kind":"permission","prompt":"Run `rm -rf /`?","options":["allow","reject"]},
528	               {"id":"el-1","kind":"elicitation","prompt":"Which branch to merge?","options":["main","release/1.0"]}],
529	   "turn_id":"h-01H..."}
530	]}}
531	```
532	
533	- `state` is an **A2A session state**: `idle` | `working` | `input-required` (ADR 0005 adds `idle` — A2A states are per *task*, Holler presence is per *session*).
534	- `mode` is `spawn` | `attach`. `harness_session_id` is present for `attach` (the existing harness session being attached to).
~~~~

**E68: `docs/protocol/v2.md:547-547`** — The pane hold paragraph.

Command: `sed -n '547,547p' docs/protocol/v2.md | awk -v s=547 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
547	  - **A third kind, sourced from a pane record (issue #646).** A **pane** hold is not operator state at all: the same enforcement point refuses a prompt to the session of record of a pane whose record says it is parked, unhealthy, or showing a session other than the one it drives, whatever route the prompt takes (a bare `say SESSION` included). It never appears on a roster row (`hold`/`hold_reason`/`held_since` stay operator fields), `release` does not lift it (the pane record owns it: `pane unpark` or `pane doctor` is the remedy), and the refusal carries `data.hold_kind: "pane"` with `data.reason` set to the stable pane code (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`) instead of a free-text reason, and no `data.since` ([ADR 0021](../adr/ADR-0021.md) records the predicate).
~~~~

**E69: `docs/protocol/v2.md:552-558`** — "Attach-mode sessions": where the rebind paragraph goes.

Command: `sed -n '552,558p' docs/protocol/v2.md | awk -v s=552 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
552	### Attach-mode sessions (issue #195)
553	
554	`mode:"attach"` is dispatched to `HttpAttachDriver` instead of `AcpDriver`
555	(§`crates/holler-body/src/http_attach_driver.rs`) — it never spawns anything;
556	it attaches to a harness session another process already owns.
557	
558	- **Eager, not lazy.** Unlike a `spawn`-mode session (whose driver comes up
~~~~

**E70: `docs/protocol/v2.md:663-665`** — The error table rows `-32009`, `-32010`, `-32011`.

Command: `sed -n '663,665p' docs/protocol/v2.md | awk -v s=663 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
663	| `-32009` | `session_busy` | `say` to a `working`/`stalled` session without `--queue` (issue #150), **or** to an `input-required` session (issue #151) — `--queue` never overrides the latter; `data.state:"input-required"` distinguishes it from the `working`/`stalled` case, and the CLI hint points at `answer`, not `interrupt`/`--queue`. |
664	| `-32010` | `nothing_pending` | `answer SESSION CHOICE` to a session that is not `input-required` — nothing to resolve (issue #151). |
665	| `-32011` | `session_held` | `say` (plain, `--queue` or `--replace`, including the redirect half of `interrupt SESSION TEXT`) to a session an operator has **held** (`holler hold SESSION`, issue #437). New work is refused; the turn already running, turns already accepted into the body's queue, `interrupt` and `roster` are not affected. `data.reason` is the operator's text (absent when none was given), `data.since` is when the hold was set (RFC 3339) and `data.hold_kind` (issue #460) is `operator` or `default` (absent means `operator`). With `data.hold_kind: "pane"` (issue #646) the refusal comes from a pane record and not an operator hold: `data.reason` is the stable pane code (`pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`), there is no `data.since`, and the `release` verb does not lift it. |
~~~~

**E71: `docs/pane-control.md:265-290`** — The guide: `reset`, "Do not use `--first` on a parked or held pane", `park`.

Command: `sed -n '265,290p' docs/pane-control.md | awk -v s=265 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
265	### `holler pane reset PANE`
266	
267	Start a pane on a fresh session: create it, show it, record it. `--first TEXT` queues a first message to the new session
268	once it is recorded. The previous session stays on the server, and `doctor` then reports it as a `stray-session`.
269	
270	```bash
271	holler pane reset demo-alpha --first "Read the brief and start."
272	```
273	
274	Refused (3) with `session-busy` or `session-holds-question` while the session of record is running a turn or waits on
275	an answer, and with `orchestrator-pane` and `server-unhealthy` as `switch` is. A Claude Code pane is reset by a relaunch on
276	a fresh session id, refused as `switch` is (`session-holds-permission` included); `--first` goes to the relaunched
277	process, through the same direct path as below.
278	
279	**Do not use `--first` on a parked or held pane.** The first message goes to the harness directly, outside the hub's
280	prompt path, so the gate that refuses a prompt to a parked or held pane does not see it (ADR-0021 section 8, "The prompt
281	and activity seam"). Sending it through that gate is issue [#839](https://github.com/Performant-Labs/holler/issues/839),
282	not built.
283	
284	### `holler pane park --reason TEXT --release-when WHEN`
285	
286	Take a pane out of service until it is unparked: the record's hold becomes parked, with the reason and the release
287	condition (one line each, at most 200 characters). It takes one pane, or with `--profile` and no pane every pane of the
288	profile. It changes the record only: nothing moves in Herdr and no process is touched. A prompt
289	through the hub to a parked pane's session of record is refused (`pane-parked`); the first message of `reset --first` is
290	not (see `reset`).
~~~~

**E72: `docs/pane-control.md:519-521`** — The guide's DRIVEN column.

Command: `sed -n '519,521p' docs/pane-control.md | awk -v s=519 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
519	| HEALTH | `healthy`, `unhealthy` or `unknown`, as the harness server last reported it (`pane get` shows the reason); for a Claude Code pane, whether its process still ran. |
520	| SHOWN | The session the pane's TUI showed when reconcile last observed it; for a Claude Code pane, the session its process held. |
521	| DRIVEN | The session the hub drives, printed as the record holds it and never compared. |
~~~~

**E73: `docs/pane-control.md:543-547`** — The guide: "SHOWN against DRIVEN".

Command: `sed -n '543,547p' docs/pane-control.md | awk -v s=543 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
543	**SHOWN against DRIVEN.** The session of record is the session the hub drives (I2), so SYNC compares SHOWN with it. It
544	is `MISMATCH` when the TUI showed another session or its home screen, and `-` when reconcile has never observed the pane
545	or the pane has no session of record. DRIVEN is printed as the record holds it and is not compared: nothing records it
546	yet (the hub wiring, #649, leaves it as stored; making it follow each pane's session of record is
547	[#839](https://github.com/Performant-Labs/holler/issues/839), not built), so it prints `-`. `pane doctor` uses the same rule for `shown-driven-mismatch` (section 5).
~~~~

**E74: `docs/pane-control.md:920-924`** — The guide's cutover steps 4 to 6.

Command: `sed -n '920,924p' docs/pane-control.md | awk -v s=920 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
920	   `shown-driven-mismatch`, and `holler pane list` shows SYNC `ok` for every pane (section 4).
921	5. **Cut one pane over** to `holler pane launch PANE` and `holler pane reset PANE`, watch it **through a full overnight**,
922	   then move the rest. A launch records the pane's model and does not apply it yet (#829, section 2), and
923	   `reset --first` is not used on a parked or held pane (section 3).
924	6. **Seven days with no invisible session.** Every day `doctor` reports no `shown-driven-mismatch` and no stray session
~~~~

**E75: `docs/pane-control.md:975-984`** — The guide's "Later" list (#835 reworded the bullet that named the import and `--adopt-claude-code`; the DRIVEN bullet this story deletes is unchanged).

Command: `sed -n '975,984p' docs/pane-control.md | awk -v s=975 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
975	## 12. Later
976	
977	Deferred, so not documented here as existing; none of them is available:
978	
979	- applying a pane's model, effort and environment to its harness server (#829);
980	- DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
981	- adopting the Claude Code panes the import leaves alone (#831's last step, ADR-0021 section 15, item 17; until it lands,
982	  `--adopt-claude-code` is refused);
983	- a body that lets `holler say --pane NAME TEXT` reach a Claude Code pane (#850, after #839);
984	- answering a Claude Code pane's permission prompt, and its `--agent` (later stories of ADR-0021 section 15);
~~~~

**E76: `CHANGELOG.md:8-12`** — The head of `## [Unreleased]`: the shape of an entry.

Command: `sed -n '8,12p' CHANGELOG.md | awk -v s=8 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
8	## [Unreleased]
9	
10	### Enhancements
11	- Pane control, a Claude Code harness (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
12	  `holler pane launch PANE --harness claude --port-policy none` launches a pane whose harness is one interactive Claude
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

**E79: `crates/holler-pane-testkit/src/harness.rs:142-148`** — The fake harness mints `ses_` and 26 hex digits.

Command: `sed -n '142,148p' crates/holler-pane-testkit/src/harness.rs | awk -v s=142 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
142	///   running server was started with; otherwise the fake's refusal [`AGENT_MISMATCH`].
143	/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
144	///   never started, as a timed request that times out or is refused would tell.
145	/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
146	///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
147	///   directory, or list that directory's sessions in creation order.
148	/// - `abort(port, session)` reaches the port. A session outside its data directory is
~~~~

**E80: `crates/holler-pane-testkit/src/harness.rs:530-537`** — `mint_session`.

Command: `sed -n '530,537p' crates/holler-pane-testkit/src/harness.rs | awk -v s=530 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
530	    /// Mint a new session id in the data directory of `port`.
531	    fn mint_session(&mut self, port: u16) -> String {
532	        self.sessions_minted += 1;
533	        let id = format!("ses_{:026x}", self.sessions_minted);
534	        let dir = self.dir(port).to_owned();
535	        self.sessions.entry(dir).or_default().push(id.clone());
536	        id
537	    }
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

**E83: `crates/holler-cli/tests/pane_verbs/reset.rs:329-380`** — The #645 test that asserts the first message through `HarnessPort::send_prompt` (to be rewritten).

Command: `sed -n '329,380p' crates/holler-cli/tests/pane_verbs/reset.rs | awk -v s=329 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
329	/// lists both), `--first` queues the text to the NEW session only — never the old
330	/// one, never another pane's session or port — and only after the record write. The
331	/// run is part 1's reset otherwise (same fake-log op sequence; no keystroke, by the
332	/// runner's I4 check).
333	#[test]
334	fn reset_with_first_queues_the_message_to_the_new_session_only() {
335	    let cases = reset_over_seam(
336	        || Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]),
337	        &["--first", "ship the fix"],
338	        Activity::Idle,
339	        None,
340	    );
341	    data(&cases);
342	    for case in &cases {
343	        let s1 = case.rig.live(P).session.clone();
344	        let q = case.rig.live(Q).session.clone();
345	        let new = case
346	            .record(P)
347	            .session_of_record
348	            .expect("the new session of record");
349	        assert_ne!(
350	            &new, &s1,
351	            "{:?}: a new session, not the old one",
352	            case.format
353	        );
354	        assert_ne!(&new, &q, "{:?}: not the other pane's session", case.format);
355	        case.assert_recorded(&new);
356	        let ops = [
357	            H::Health,
358	            H::CreateSession,
359	            H::SelectSession,
360	            H::ShownSession,
361	        ];
362	        assert_eq!(case.calls.harness, ops, "{:?}", case.format);
363	        let prompts = case.setup.prompts();
364	        assert_eq!(
365	            prompts.len(),
366	            1,
367	            "{:?}: one prompt, to the new session only: {prompts:?} (old {s1:?}, {Q} {q:?})",
368	            case.format
369	        );
370	        let prompt = &prompts[0];
371	        assert_eq!(prompt.port, case.rig.live(P).port, "{:?}", case.format);
372	        assert_eq!(prompt.session, new, "{:?}: the new session", case.format);
373	        assert_eq!(prompt.text, "ship the fix", "{:?}", case.format);
374	        assert_eq!(
375	            prompt.then_of_record.as_deref(),
376	            Some(new.as_str()),
377	            "{:?}: the prompt follows the record write",
378	            case.format
379	        );
380	    }
~~~~

**E84: `crates/holler-cli/tests/pane_verbs/reset.rs:419-447`** — The #645 test that a failing `send_prompt` after the record is reported honestly.

Command: `sed -n '419,447p' crates/holler-cli/tests/pane_verbs/reset.rs | awk -v s=419 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
419	/// honestly: the run fails with the port's own code, the message says the first
420	/// message did not land, and nothing is appended — no reconcile step (the record is
421	/// correct) and no "is not recorded" clause (the session IS recorded). The record
422	/// and the TUI stand exactly as a successful reset left them.
423	#[test]
424	fn reset_first_prompt_failure_after_the_record_is_honest() {
425	    let fault = PaneError::Unavailable {
426	        what: "the harness".to_owned(),
427	    };
428	    let cases = reset_over_seam(
429	        || Rig::new(&[Seed::new(P, 1, 1)]),
430	        &["--first", "ship the fix"],
431	        Activity::Idle,
432	        Some(fault),
433	    );
434	    for message in failed(&cases, 1, "unavailable") {
435	        assert!(
436	            message.contains("first message did not land"),
437	            "names what failed: {message:?}"
438	        );
439	        assert!(
440	            !message.contains("to reconcile"),
441	            "no reconcile step: {message:?}"
442	        );
443	        assert!(
444	            !message.contains("is not recorded"),
445	            "the session is recorded: {message:?}"
446	        );
447	    }
~~~~

**E85: `crates/holler-cli/tests/pane_verbs/switch.rs:871-891`** — "A switch is unaffected by the activity seam: a switch queries no activity" (switch.rs is 868 lines).

Command: `sed -n '871,891p' crates/holler-cli/tests/pane_verbs/switch.rs | awk -v s=871 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
871	/// AC 4 (part 2): a switch is unaffected by the activity seam: a wrapper that would
872	/// answer busy for any query is never asked (the gate is reset's), and a switch
873	/// queues no prompt.
874	#[test]
875	fn switch_is_unaffected_by_the_activity_seam() {
876	    let build = || {
877	        let (rig, s2) = one_pane();
878	        (rig, (s2, Arc::new(SeamLog::default())))
879	    };
880	    let words = |_: &Rig, (s2, _): &(String, Arc<SeamLog>)| argv(&["pane", "switch", P, s2]);
881	    let exec = |rig: &Rig, (_, log): &(String, Arc<SeamLog>), argv: &[&str], format| {
882	        let seam = SeamHarness::new(rig, P, log, Activity::Busy, None);
883	        run_verb_with(argv, format, rig.ports_with(&seam))
884	    };
885	    let cases = both_with(build, words, exec);
886	    data(&cases);
887	    for case in &cases {
888	        case.assert_recorded(&case.setup.0);
889	        let log = &case.setup.1;
890	        assert!(log.activity().is_empty(), "a switch queries no activity");
891	        assert!(log.prompts().is_empty(), "a switch queues no prompt");
~~~~

**E86: `crates/holler-cli/tests/pane_integration/scenario.rs:141-156`** — The opt-in scenario step `reset_with_a_first_message`, as #667 left it on `main` (D9's step is applied to this text).

Command: `sed -n '141,156p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=141 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
141	/// Step 3: a reset of C2 with a first message, through `send_prompt` (#642 part 3). The
142	/// dead-end provider is retried until the turn is aborted, so the session reads `Busy`, as D12
143	/// expects; no model is reached.
144	fn reset_with_a_first_message(scratch: &Scratch, p2: u16) {
145	    let before = scratch.session_of_record(C2);
146	    scratch.json(&["pane", "reset", C2, "--first", "hlr649 first message"], 0);
147	    let after = scratch.record(C2);
148	    let fresh = after["session_of_record"].as_str().unwrap().to_owned();
149	    assert_ne!(fresh, before, "a fresh session");
150	    assert_eq!(after["last_observed"]["shown"], fresh.as_str(), "SHOWN follows");
151	    let busy = wait_for(Duration::from_secs(15), || {
152	        (scratch.harness().session_activity(&HarnessEndpoint::Port(p2), &fresh) == Ok(Activity::Busy)).then_some(())
153	    });
154	    assert!(busy.is_some(), "the first message reached the server: its session reads Busy");
155	    no_model_can_be_reached(p2);
156	}
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

**E89: `docs/adr/ADR-0021.md:1380-1400`** — #831 on `main`: item 18, `say` needs #839 and a body (#850); its last sentences say `reset --first` "waits for #850 too" once #839 moves it, and name the edit this story makes to them.

Command: `sed -n '1380,1400p' docs/adr/ADR-0021.md | awk -v s=1380 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1380	18. **`holler say` and the acceptance step: it needs #839, and more (checked 2026-10-11).** The scratch scenario's step "deliver
1381	    a message to each through `holler say`" cannot be met by this story alone, for either harness:
1382	    - The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body that drives
1383	      its harness, addressed by the body's advertised name (ADR 0014 point 5; `send_prompt` and `pane_gate` in
1384	      `holler-hub/src/circuit/dispatch.rs`). `holler say --pane NAME` resolves the pane to its `session_of_record` and uses
1385	      that as the address (`resolve_pane_target`), which matches no body's name: the id is an OpenCode `ses_…` or a Claude
1386	      UUID, and the join from a body's `harness_session_id` to `Pane.session_of_record` is **#839** (DRIVEN, not built). So
1387	      `say --pane`, its holds and its pane gate reach no real pane of either harness until #839.
1388	    - A Claude Code pane additionally has **no body at all**: the hub constructs no adapter (decision 1 of section 5), and the
1389	      adapter this story builds is called by the pane engine, not by a body. A body driver for it is the follow-up story **#850**.
1390	    So this story proves delivery where it can, at `HarnessPort::send_prompt` (item 7) against `FakeClaude` and the opt-in
1391	    real-binary test, and through `reset --first`; the `say` step of the scenario is written down, marked blocked by #839 and
1392	    #850, and **#831 does not close on it**. Nothing here routes a prompt around the hub's gate: the Claude pane's `say` is not
1393	    given a CLI-side path, because that would be a second enforcement site for holds and parks. **As built (plan review W4 and
1394	    W12):** delivery is proven at `HarnessPort::send_prompt`, on `FakeClaude`, in the conformance suite's case 9 and in the
1395	    opt-in real test. `reset --first` reaches the relaunched process (the endpoint of the record the relaunch stored), but it is
1396	    the existing carve-out around the hub's prompt gate (section 8, the seam). Moving it to the hub's prompt path was #828's,
1397	    which was closed on 2026-10-11 (not planned) as folded into #839: once #839 moves it, a Claude pane's `reset --first` waits
1398	    for #850 too. **The roster, as built:** #648 merged before this story's PR, so `holler roster` reads the registry and
1399	    prints a pane's cells on the hub row joined to it. Its rows are the sessions that bodies serve, so a Claude Code pane has
1400	    no roster row until a body drives its session (#850); `holler pane list` and `holler pane doctor` show it now.
~~~~

**E90: `docs/adr/ADR-0021.md:1432-1435`** — #831 on `main`: the "Deferred" DRIVEN bullet (#839) and the Claude body bullet (#850) that this story rewrites (#835 shortened the second bullet: the adoption sentence moved to a bullet of its own at the end of the list).

Command: `sed -n '1432,1435p' docs/adr/ADR-0021.md | awk -v s=1432 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1432	- How the hub's DRIVEN session follows `session_of_record`: #839 (it was #654's; the wiring, #649, leaves it to the cutover).
1433	- A body that lets `holler say --pane NAME TEXT` reach a Claude Code pane: #850 (section 15, item 18). `pane-stalled` waits
1434	  with #837's time source, and the `PreToolUse` gate, a permission-mode, allowed-tools and questions spec field and `--agent`
1435	  for Claude are later stories (section 15).
~~~~

**E91: `crates/holler-cli/tests/pane_claude/lifecycle.rs:207-223`** — #831 on `main`: its test that `reset --first` reaches the new Claude process (through the engine's `send_prompt`); this story changes it to "refused, nothing moved".

Command: `sed -n '207,223p' crates/holler-cli/tests/pane_claude/lifecycle.rs | awk -v s=207 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
207	/// W4: the first message goes to the process the reset started, never to the stopped one (the
208	/// endpoint of the record the relaunch stored, not of the record the plan read).
209	#[test]
210	fn reset_first_reaches_the_new_process() {
211	    let rig = one_claude_pane();
212	    let old_pid = rig.record(CLAUDE).harness.pid.unwrap();
213	
214	    let run = rig.run(
215	        &["pane", "reset", CLAUDE, "--first", "start on the review"],
216	        Format::Json,
217	    );
218	
219	    assert_eq!(run.code, 0, "{run:?}");
220	    let process = rig.process_of(CLAUDE).unwrap();
221	    assert_ne!(process.pid, old_pid);
222	    assert_eq!(process.inbox, vec!["start on the review"]);
223	}
~~~~

**E92: `crates/holler-pane/src/ports.rs:25-41`** — `ports.rs` on `main`: the amendments list, with #835's required `HostPort::foreground_commands` and `HerdrPort::observe_pane` (which lost its default) and #831's endpoint-keyed `HarnessPort`, "so that the compiler finds every implementation".

Command: `sed -n '25,41p' crates/holler-pane/src/ports.rs | awk -v s=25 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
25	//! - #832's: [`HerdrPort::observe_pane`] and its batch form [`HerdrPort::observe_panes`],
26	//!   what Herdr says about a pane that no record names;
27	//! - #835's: the required [`HostPort::foreground_commands`], the command name of the
28	//!   foreground process of each pane of a tmux session (how the import tells a pane that
29	//!   runs Claude Code), and [`HerdrPort::observe_pane`] lost its default. Both are required
30	//!   so that the compiler finds every implementation, a wrapper port included: a default
31	//!   answers silently in place of a port a wrapper forgot to forward;
32	//! - #831's, a second harness (ADR-0021 section 15): every [`HarnessPort`] method keyed by a
33	//!   server's `port: u16` is keyed by a [`HarnessEndpoint`] instead (a port, or a single
34	//!   process), the change of the port that #802 decision 2 set as the pattern, so a call
35	//!   site that forgets the endpoint does not compile; `shown_session` and `attach_tui` take
36	//!   one too. Four default-armed methods for a single-process harness, each defaulting to
37	//!   the refusal [`PROCESS_UNSUPPORTED`] ([`HarnessPort::process_command`],
38	//!   [`HarnessPort::new_session_id`], [`HarnessPort::locate_process`],
39	//!   [`HarnessPort::login_status`]); [`ServeSpec`] gains the model and the directory; and
40	//!   [`Activity::WaitingPermission`]. [`HarnessRouter`] is the one router of the two
41	//!   shapes, so [`Ports`] keeps one harness.
~~~~

**E93: `crates/holler-cli/src/roster_cmd.rs:10-16`** — #648 on `main`: the roster's module doc: a pane is a mismatch when SHOWN and DRIVEN are both known and differ.

Command: `sed -n '10,16p' crates/holler-cli/src/roster_cmd.rs | awk -v s=10 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
10	//! **The pane columns (story #648, epic #633).** A hub row is joined to a pane record when the part
11	//! of its name before the first `/` is the pane's name (`pane_label`); the row then carries six of
12	//! the cells `holler pane list` prints for that record ([`PANE_COLUMNS`]). They are the registry's
13	//! values, read on each call, and the roster calls no adapter. When a pane's recorded SHOWN and
14	//! DRIVEN are both known and differ (`shown_driven_mismatch`), the text output's first line names
15	//! it. The registry is the local hub's: it is read only after the hub call answered, and never
16	//! with `--server`, which names another hub.
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

**E96: `crates/holler-pane/src/tx_switch.rs:96-112`** — `parse_session_id`: "OpenCode's ids are `ses_` and 26 letters and digits".

Command: `sed -n '96,112p' crates/holler-pane/src/tx_switch.rs | awk -v s=96 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
96	/// A harness session id typed by a person: 1 to [`SESSION_ID_MAX`] ASCII letters, digits,
97	/// `_` or `-` (OpenCode's ids are `ses_` and 26 letters and digits). Anything else is
98	/// `usage`, and the message quotes the text with [`quoted`], so no whitespace, control
99	/// character or shell metacharacter reaches a port or a message.
100	pub fn parse_session_id(text: &str) -> Result<String, PaneError> {
101	    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
102	    if (1..=SESSION_ID_MAX).contains(&text.len()) && text.bytes().all(allowed) {
103	        return Ok(text.to_owned());
104	    }
105	    Err(PaneError::Usage {
106	        message: format!(
107	            "invalid session id {}: a session id is 1 to {SESSION_ID_MAX} ASCII letters, \
108	             digits, '_' or '-'",
109	            quoted(text)
110	        ),
111	    })
112	}
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

**E98: `crates/holler-hub/src/control_server.rs:251-284`** — `hub_query_remote`: `control/query_remote` hands any `params.method` to `LiveHandle::query`; the allowlist (E6) is its only filter.

Command: `sed -n '251,284p' crates/holler-hub/src/control_server.rs | awk -v s=251 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
251	/// `control/query_remote {target, method, params?}` (issue #185): `holler hub
252	/// query TARGET CMD [ARGS...]` — resolve `target` against the live registry
253	/// and forward the `query/*` request over that body's socket. No live body
254	/// for `target` is `-32004 not_connected` (exit 1 at the CLI); more than one
255	/// match is reported as an ambiguous refusal (the CLI maps that to exit 2,
256	/// ADR 0003's usage-error code for an unresolvable target).
257	async fn hub_query_remote(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
258	    let params = obj.get("params");
259	    let (Some(target), Some(method)) = (
260	        params.and_then(|p| p.get("target")).and_then(|v| v.as_str()),
261	        params.and_then(|p| p.get("method")).and_then(|v| v.as_str()),
262	    ) else {
263	        return encode_error(cid, Code::InvalidParams, "control/query_remote needs params.target and params.method".to_string());
264	    };
265	    let inner_params = params.and_then(|p| p.get("params")).cloned();
266	
267	    match registry.find_target(target).await {
268	        crate::live::TargetLookup::Found(handle) => {
269	            match handle.query(method, inner_params, std::time::Duration::from_secs(10)).await {
270	                Some(Ok(value)) => encode_response(cid, value),
271	                // The body itself raised a wire error (e.g. `query/support`'s
272	                // `-32006 unknown_feature`) — forward it verbatim rather than
273	                // re-wrapping it in a fresh `Code`.
274	                Some(Err(e)) => holler_proto::encode(&Envelope::error_frame(cid, &e)).unwrap_or_default(),
275	                None => encode_error(cid, Code::NotConnected, format!("{target} did not answer")),
276	            }
277	        }
278	        crate::live::TargetLookup::NotConnected => {
279	            encode_error(cid, Code::NotConnected, format!("{target} is not connected"))
280	        }
281	        crate::live::TargetLookup::Ambiguous => {
282	            encode_error(cid, Code::InvalidRequest, format!("{target} is ambiguous — matches more than one live body"))
283	        }
284	    }
~~~~

**E99: `crates/holler-hub/src/circuit/admin.rs:256-262`** — `admin/query` with a `params.target` maps to `query_remote`, so an admin-role remote connection reaches the same forward.

Command: `sed -n '256,262p' crates/holler-hub/src/circuit/admin.rs | awk -v s=256 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
256	    let Ok(obj) = serde_json::from_str::<serde_json::Value>(text) else { return RequestOutcome::Inline };
257	    let verb = if method == "admin/query" {
258	        // MO 2: maps to `control/query_remote` when `params.target` is
259	        // present, `control/query_local` when it is absent — the same
260	        // shape `hub_query_remote`/`hub_query_local` already read `obj`
261	        // for, unchanged.
262	        let has_target = obj.get("params").and_then(|p| p.get("target")).and_then(|v| v.as_str()).is_some();
~~~~

**E100: `crates/holler-cli/tests/hold_hub_test.rs:55-70`** — The existing pattern: a raw `session/prompt` sent through `query_remote` is refused `-32601` and the session saw nothing.

Command: `sed -n '55,70p' crates/holler-cli/tests/hold_hub_test.rs | awk -v s=55 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
55	#[test]
56	fn a_prompt_cannot_be_forwarded_around_the_hold() {
57	    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
58	    rig.hold(SESSION, None).unwrap();
59	    let prompt = serde_json::json!({
60	        "session": "alpha",
61	        "message": { "messageId": "m-x", "role": "ROLE_USER", "parts": [{ "text": "sneak" }] },
62	    });
63	    let res = control::query_remote_at(rig.root(), "b", "session/prompt", Some(prompt));
64	    match res {
65	        Err(ControlError::Refused(e)) => assert_eq!(e.code, -32601, "a prompt is not a forwardable method: {e:?}"),
66	        other => panic!("the generic forward must refuse a session/prompt: {other:?}"),
67	    }
68	    // And the session saw nothing: the roster row never left idle.
69	    assert_eq!(rig.row(SESSION).unwrap()["state"], "idle");
70	}
~~~~

**E101: `crates/holler-body/src/session_manager/task.rs:171-186`** — The session task's loop: the attach retry runs while an attach session has no driver (`awaiting_attach`).

Command: `sed -n '171,186p' crates/holler-body/src/session_manager/task.rs | awk -v s=171 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
171	    loop {
172	        let awaiting_attach = inner.config.mode == SessionMode::Attach && inner.driver.is_none();
173	        tokio::select! {
174	            biased;
175	            cmd = mailbox.recv() => {
176	                if !dispatch_command(&mut inner, cmd).await {
177	                    return;
178	                }
179	            }
180	            event = poll_stream(&mut inner.current_stream) => {
181	                handle_stream_event(&mut inner, event).await;
182	            }
183	            _ = retry.tick(), if awaiting_attach => {
184	                attempt_initial_attach(&mut inner).await;
185	            }
186	        }
~~~~

**E102: `crates/holler-body/src/session_manager/task.rs:258-262`** — `attempt_initial_attach` (what the retry calls) attaches from `inner.config`.

Command: `sed -n '258,262p' crates/holler-body/src/session_manager/task.rs | awk -v s=258 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
258	async fn attempt_initial_attach(inner: &mut Inner) {
259	    match Driver::spawn_or_attach(&inner.config).await {
260	        Ok(driver) => {
261	            inner.driver = Some(driver);
262	            let mut p = inner.presence.lock().unwrap_or_else(PoisonError::into_inner);
~~~~

**E103: `crates/holler-body/src/session_manager/task.rs:492-498`** — `start_turn` re-attaches from `inner.config` when the driver is `None`.

Command: `sed -n '492,498p' crates/holler-body/src/session_manager/task.rs | awk -v s=492 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
492	    if inner.driver.is_none() {
493	        match Driver::spawn_or_attach(&inner.config).await {
494	            Ok(d) => inner.driver = Some(d),
495	            Err(e) => {
496	                let _ = reply_tx.send(PromptOutcome::Error(e.message()));
497	                return false;
498	            }
~~~~

**E104: `crates/holler-body/src/session_manager/task.rs:659-665`** — A turn that ends in `StopReason::Error` drops the driver, so the next prompt (or the retry) re-attaches.

Command: `sed -n '659,665p' crates/holler-body/src/session_manager/task.rs | awk -v s=659 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
659	    inner.last_update_at_ms = None;
660	    // Crash isolation (issue #189's own acceptance item): drop the driver so
661	    // the *next* prompt respawns a fresh one, never retried automatically.
662	    if matches!(reason, StopReason::Error) {
663	        inner.driver = None;
664	    }
665	    if publish_idle {
~~~~

**E105: `crates/holler-body/src/http_attach_driver/connection.rs:480-481`** — OpenCode's `session.error` event ends the turn with `StopReason::Error`.

Command: `sed -n '480,481p' crates/holler-body/src/http_attach_driver/connection.rs | awk -v s=480 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
480	        "session.idle" => finish_turn(shared, StopReason::EndTurn),
481	        "session.error" => finish_turn(shared, StopReason::Error),
~~~~

**E106: `crates/holler-body/src/session_manager/task.rs:31-56`** — `SessionPresence`: the task's presence snapshot, where the mirror of the attach id goes.

Command: `sed -n '31,56p' crates/holler-body/src/session_manager/task.rs | awk -v s=31 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
31	/// The live, presence-facing snapshot of one session — written only by that
32	/// session's own task, read by [`super::SessionManager::presence_doc`] (and
33	/// any number of other outside readers) through the shared `Arc<Mutex<_>>`.
34	#[derive(Debug, Clone)]
35	pub(super) struct SessionPresence {
36	    pub(super) state: SessionState,
37	    pub(super) turn_started_at: Option<String>,
38	    pub(super) last_update_at: Option<String>,
39	    pub(super) turn_id: Option<String>,
40	    pub(super) last_turn: Option<LastTurn>,
41	    /// The held permission(s)/elicitation(s) (issue #151) — `Some` only
42	    /// while `state` is `InputRequired`, mirroring `SessionAd.pending`'s own
43	    /// wire contract. Populated from [`AcpDriver::pending`] by [`set_state`]
44	    /// every time this session transitions to (or stays in)
45	    /// `InputRequired`, and cleared the moment it leaves that state.
46	    pub(super) pending: Option<Vec<PendingItem>>,
47	    /// `true` while an attach-mode session's *initial*
48	    /// [`crate::http_attach_driver::HttpAttachDriver::attach`] has not yet
49	    /// succeeded (its first attempt failed, and the [`ATTACH_RETRY_INTERVAL`]
50	    /// retry loop has not landed one yet) — [`super::SessionManager::
51	    /// presence_doc`] omits the whole row while this is `true` (issue #195).
52	    /// Always `false` for a spawn-mode session (never gated — spawning is
53	    /// lazy, on the first prompt, not eager at task startup the way an
54	    /// attach is).
55	    pub(super) omitted_pending_attach: bool,
56	}
~~~~

**E107: `crates/holler-body/src/session_manager.rs:181-189`** — `ManagedSession`: the manager keeps the as-started `config`.

Command: `sed -n '181,189p' crates/holler-body/src/session_manager.rs | awk -v s=181 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
181	/// One registered session's manager-side handle: its (immutable) config, the
182	/// mailbox to its task, and the read-only presence snapshot that task keeps
183	/// current.
184	struct ManagedSession {
185	    config: SessionConfig,
186	    tx: mpsc::Sender<SessionCommand>,
187	    presence: Arc<Mutex<SessionPresence>>,
188	    join: JoinHandle<()>,
189	}
~~~~

**E108: `crates/holler-body/src/query.rs:151-158`** — `query/support`'s live probe reads the configured `session_id`.

Command: `sed -n '151,158p' crates/holler-body/src/query.rs | awk -v s=151 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
151	    for s in &sessions {
152	        match s.mode {
153	            SessionMode::Attach => {
154	                let (endpoint, session_id) = (s.endpoint.as_deref(), s.session_id.as_deref());
155	                let ok = match (endpoint, session_id) {
156	                    (Some(endpoint), Some(session_id)) => {
157	                        probe_attach_session_live(endpoint, session_id)
158	                    }
~~~~

**E109: `crates/holler-body/src/registry.rs:74-90`** — `SessionRegistry::presence_doc`: the as-started baseline reads `entry.config.session_id` (called only by `config_test`; production uses `SessionManager::presence_doc`, E46).

Command: `sed -n '74,90p' crates/holler-body/src/registry.rs | awk -v s=74 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
74	    /// Build this body's `session/presence` params (the notification issue
75	    /// #182 sends on every heartbeat and reconnect): one [`SessionAd`] per
76	    /// registered session, reflecting its current state. `hostname` is the
77	    /// body's own hostname/label (`Presence::hostname`).
78	    pub fn presence_doc(&self, hostname: String) -> Presence {
79	        let sessions = self
80	            .sessions
81	            .values()
82	            .map(|entry| SessionAd {
83	                name: entry.config.name.as_str().to_string(),
84	                harness: entry.config.harness.clone(),
85	                state: entry.state,
86	                mode: match entry.config.mode {
87	                    SessionMode::Spawn => Mode::Spawn,
88	                    SessionMode::Attach => Mode::Attach,
89	                },
90	                harness_session_id: entry.config.session_id.clone(),
~~~~

**E110: `crates/holler-body/src/connection/handshake.rs:230-238`** — The body's `hello` sends an empty `sessions` list (`.then(Vec::new)`).

Command: `sed -n '230,238p' crates/holler-body/src/connection/handshake.rs | awk -v s=230 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
230	    let harnesses = if role == HelloRole::Body {
231	        let mut h: Vec<String> = configs.iter().map(|c| c.harness.clone()).collect();
232	        h.sort();
233	        h.dedup();
234	        Some(h)
235	    } else {
236	        None
237	    };
238	    let sessions = (role == HelloRole::Body).then(Vec::new);
~~~~

**E111: `crates/holler-body/src/connection.rs:703-705`** — The body's `handle_query` call passes `self.configs`.

Command: `sed -n '703,705p' crates/holler-body/src/connection.rs | awk -v s=703 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
703	            Envelope::Request { id, method, params } if method.starts_with("query/") => {
704	                crate::dispatch::handle_query(self.sink, &id, &method, params, self.identity, self.state_root, self.configs).await
705	            }
~~~~

**E112: `crates/holler-hub/src/roster.rs:343-376`** — `Roster::advertise` replaces a token's whole set of rows; a row absent from the frame goes `gone` at once.

Command: `sed -n '343,376p' crates/holler-hub/src/roster.rs | awk -v s=343 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
343	    /// <session>` (see [`qualified_name`]); idempotent, like [`Roster::
344	    /// set_token`]/[`Roster::set_hostname`].
345	    pub fn set_label(&self, token_id: &str, label: &str) {
346	        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).bind_label(token_id, label);
347	    }
348	
349	    /// A `session/presence` from a body: replace (not merge) that token's set
350	    /// of rows. A row in the frame is upserted (`connected`, `last_seen` =
351	    /// now); a row the token previously held but which is absent from the frame
352	    /// goes `gone` **immediately**. Returns the names **rejected** by the
353	    /// collision policy (a connected different-token holder blocks them) — the
354	    /// rest of the frame was applied.
355	    pub fn advertise(&self, token_id: &str, p: &Presence) -> Vec<String> {
356	        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
357	        inner.bind_token(token_id, None, Some(&p.hostname));
358	        let info = inner.tokens.get(token_id).cloned().unwrap_or_default();
359	        let incoming: HashSet<String> = p.sessions.iter().map(|s| qualified_name(&info, s)).collect();
360	        // The newest ad timestamp in this frame (the `last_update_at` base):
361	        // the body emits one presence per body, so its sessions share one
362	        // clock and this is the frame's "now" on the body's wall.
363	        let base = p
364	            .sessions
365	            .iter()
366	            .filter_map(|s| parse_rfc3339_secs(s.last_update_at.as_deref()))
367	            .max();
368	        let mut rejected = Vec::new();
369	        let mut changed = 0usize;
370	        for s in &p.sessions {
371	            if let Some(name) = inner.apply_ad(token_id, &info, s, base) {
372	                rejected.push(name);
373	            } else {
374	                changed += 1;
375	            }
376	        }
~~~~

**E113: `crates/holler-hub/src/live.rs:222-250`** — `LiveHandle`: its fields (`tx`, `presence`), where `query_slot` goes.

Command: `sed -n '222,250p' crates/holler-hub/src/live.rs | awk -v s=222 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
222	/// A live, authenticated circuit's command channel — how another task (the
223	/// control socket) reaches this specific connection.
224	#[derive(Clone)]
225	pub struct LiveHandle {
226	    pub client_id: String,
227	    pub hostname: String,
228	    pub token_id: String,
229	    /// The transport peer address (`ip:port`) this connection was accepted
230	    /// from (issue #184: captured at accept, reported in every connection
231	    /// log line and in `hub status --json`'s `clients_detail[].peer`).
232	    pub peer: String,
233	    /// The moment this circuit was registered (issue #184's `connected_at`,
234	    /// epoch milliseconds).
235	    pub connected_at: u64,
236	    /// This registry entry's monotonic generation stamp (issue #184). Not
237	    /// exposed on the wire; used by [`Registry::remove_if_current`] so a
238	    /// superseded connection's own teardown never evicts the entry that
239	    /// replaced it (see that fn's doc for the race it closes).
240	    seq: u64,
241	    tx: tokio::sync::mpsc::UnboundedSender<LiveCommand>,
242	    /// The priority channel a `session/cancel` travels instead of `tx`
243	    /// (issue #191) — see [`CancelCommand`]'s own doc.
244	    cancel_tx: tokio::sync::mpsc::UnboundedSender<CancelCommand>,
245	    state: Arc<Mutex<LiveState>>,
246	    /// The last `session/presence` this body sent (issue #190's roster
247	    /// stand-in — see the module doc). `None` until its first presence.
248	    presence: Arc<Mutex<Option<Vec<SessionAd>>>>,
249	}
250	
~~~~

**E114: `crates/holler-hub/src/live.rs:357-360`** — `LiveHandle::presence`: the cached session list.

Command: `sed -n '357,360p' crates/holler-hub/src/live.rs | awk -v s=357 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
357	    /// The last presence this body reported, if any yet.
358	    pub async fn presence(&self) -> Option<Vec<SessionAd>> {
359	        self.presence.lock().await.clone()
360	    }
~~~~

**E115: `crates/holler-hub/src/live.rs:504-515`** — `Registry::insert` builds a `LiveHandle`.

Command: `sed -n '504,515p' crates/holler-hub/src/live.rs | awk -v s=504 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
504	        let handle = LiveHandle {
505	            client_id: client_id.to_string(),
506	            hostname: hostname.to_string(),
507	            token_id: token_id.to_string(),
508	            peer: peer.to_string(),
509	            connected_at: now_millis_u64(),
510	            seq,
511	            tx,
512	            cancel_tx,
513	            state: Arc::new(Mutex::new(LiveState::default())),
514	            presence: Arc::new(Mutex::new(None)),
515	        };
~~~~

**E116: `crates/holler-hub/src/circuit.rs:728-735`** — The circuit's single `pending_query` slot, overwritten by each `LiveCommand::Query`.

Command: `sed -n '728,735p' crates/holler-hub/src/circuit.rs | awk -v s=728 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
728	            LiveCommand::Query { method, params, reply: reply_tx } => {
729	                match dispatch::send_query_forward(self.sink, &method, params).await {
730	                    Ok(cid) => {
731	                        self.pending_query = Some((cid, reply_tx));
732	                        Ok(())
733	                    }
734	                    Err(()) => Err(()),
735	                }
~~~~

**E117: `crates/holler-hub/src/live.rs:626-631`** — `Registry::find_target`: `hub query`'s lookup by token, client or label.

Command: `sed -n '626,631p' crates/holler-hub/src/live.rs | awk -v s=626 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
626	    /// Resolve a `hub query TARGET …` target against the live registry
627	    /// (issue #185): `target` may name a token id, a client id, or a
628	    /// hostname/label (a `label/session` form's label part). Exactly one
629	    /// match is [`TargetLookup::Found`]; zero is `NotConnected`; more than
630	    /// one is `Ambiguous`.
631	    pub async fn find_target(&self, target: &str) -> TargetLookup {
~~~~

**E118: `crates/holler-pane/src/ports.rs:573-586`** — `Activity`, the closed answer of `session_activity` (four variants since #831; in `ports.rs`, never on the wire).

Command: `sed -n '573,586p' crates/holler-pane/src/ports.rs | awk -v s=573 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
573	/// What a harness session is doing, as [`HarnessPort::session_activity`] reports it.
574	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
575	pub enum Activity {
576	    /// Waiting for a prompt.
577	    Idle,
578	    /// Running a turn.
579	    Busy,
580	    /// Waiting for an answer to a question it asked.
581	    HoldingQuestion,
582	    /// Waiting for a person to answer a permission prompt (#831; Claude Code's `waiting`
583	    /// with `permission prompt`, `sandbox request` or `worker request`). Holler does not
584	    /// answer one in this story.
585	    WaitingPermission,
586	}
~~~~

**E119: `crates/holler-pane/src/tx_switch.rs:388-410`** — `check_idle`: the one mapping today from an `Activity` (four variants since #831) to `session-busy`, `session-holds-question` and `session-holds-permission`.

Command: `sed -n '388,410p' crates/holler-pane/src/tx_switch.rs | awk -v s=388 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
388	/// A reset, and a single process's switch (a relaunch), does not drop a conversation that is
389	/// mid-turn or waiting on an answer: the pane's session of record must be idle. A pane with no
390	/// session of record has no conversation to drop, and is not asked. A query that fails is
391	/// passed on as it is.
392	fn check_idle(ports: Ports<'_>, record: &Pane, at: &HarnessEndpoint) -> Result<(), PaneError> {
393	    let Some(session) = &record.session_of_record else {
394	        return Ok(());
395	    };
396	    let activity = ports.harness.session_activity(at, session)?;
397	    let (code, doing) = match activity {
398	        Activity::Idle => return Ok(()),
399	        Activity::Busy => (SESSION_BUSY, "is running a turn; try again once it ends"),
400	        Activity::HoldingQuestion => (SESSION_HOLDS_QUESTION, "holds a question; answer it first"),
401	        Activity::WaitingPermission => (
402	            SESSION_HOLDS_PERMISSION,
403	            "waits on a permission prompt; answer it in the pane first",
404	        ),
405	    };
406	    Err(PaneError::Refused {
407	        code,
408	        message: format!("session {} of {} {doing}", quoted(session), record.name),
409	    })
410	}
~~~~

**E120: `crates/holler-pane/src/findings.rs:199-201`** — `FixState::as_str`: a state's name is its JSON form.

Command: `sed -n '199,201p' crates/holler-pane/src/findings.rs | awk -v s=199 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
199	impl FixState {
200	    /// The state's kebab-case name, which is also its JSON form.
201	    pub const fn as_str(self) -> &'static str {
~~~~

**E121: `crates/holler-pane/src/findings.rs:259-264`** — `Finding.remedy`: the finding's own command.

Command: `sed -n '259,264p' crates/holler-pane/src/findings.rs | awk -v s=259 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
259	    /// One line for a person; untrusted values in it are quoted.
260	    pub message: String,
261	    /// The command to run (see [`FindingKind::remedy`]); `None` when no Holler verb repairs
262	    /// it, or it was fixed.
263	    pub remedy: Option<String>,
264	    pub fix: FixState,
~~~~

**E122: `crates/holler-hub/src/control_server.rs:315-353`** — `control/say`'s `SayError` arms, inline.

Command: `sed -n '315,353p' crates/holler-hub/src/control_server.rs | awk -v s=315 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
315	        Err(crate::talk::SayError::UnknownSession) => {
316	            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
317	        }
318	        // Issue #192, rule 4: `detail` already carries the roster's own
319	        // `conn_state` (and, for `reconnecting`, the last-seen age) when the
320	        // roster still remembers this name — see `talk::not_connected_detail`.
321	        Err(crate::talk::SayError::NotConnected(detail)) => encode_error(cid, Code::NotConnected, detail),
322	        Err(crate::talk::SayError::Ambiguous(candidates)) => {
323	            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
324	            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
325	            encode_error_frame(cid, &err)
326	        }
327	        Err(crate::talk::SayError::Busy { state, turn_age_ms, last_update_age_ms }) => {
328	            let err = WireError::session_busy(state, turn_age_ms, last_update_age_ms);
329	            encode_error_frame(cid, &err)
330	        }
331	        // Issue #151: reported as `-32009 session_busy` with `data.state
332	        // = "input-required"` (distinct from `working`/`stalled`) so the
333	        // CLI's own busy hint (`say_cmd::busy_hint`) can point at `answer`
334	        // instead of `interrupt`/`--queue` — neither of which resolves a
335	        // held permission/elicitation.
336	        Err(crate::talk::SayError::InputRequired { question, turn_age_ms, last_update_age_ms }) => {
337	            let mut err = WireError::session_busy("input-required", turn_age_ms, last_update_age_ms);
338	            if let Some(q) = question {
339	                err.message = q;
340	            }
341	            encode_error_frame(cid, &err)
342	        }
343	        Err(crate::talk::SayError::Refused(err)) => encode_error_frame(cid, &err),
344	        Err(crate::talk::SayError::ConnectionLost) => {
345	            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"))
346	        }
347	        Err(crate::talk::SayError::Timeout) => {
348	            let secs = timeout.as_secs();
349	            encode_error(cid, Code::NotConnected, format!("no reply from {session} within {secs}s"))
350	        }
351	        Err(crate::talk::SayError::Cancelled) => {
352	            encode_error(cid, Code::NotConnected, "prompt was interrupted before it completed".to_string())
353	        }
~~~~

**E123: `crates/holler-cli/src/say_cmd.rs:244-284`** — `route_target` and `route_target_by`: a SESSION is sent as it is; a `--pane` is resolved over the wiring (`answer` passes its own resolver, #831).

Command: `sed -n '244,284p' crates/holler-cli/src/say_cmd.rs | awk -v s=244 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
244	/// Clear a resolved prompt verb to run, or say why it stops; `say`, `interrupt` and
245	/// `answer` all call it. A malformed tail is `usage` (exit 2), as before. A SESSION is
246	/// sent to as it is, and a SESSION with `--profile` is `usage`, naming `--pane`. A
247	/// `--pane` target is resolved by [`resolve_pane_target`] over the run's wiring, before
248	/// any hub is contacted: its error prints with the exit of its code's class, a refusal
249	/// (exit 3) naming its stable code.
250	pub fn route_target(
251	    resolved: Result<PromptArgs, Usage>,
252	    profile: &ProfileOpt,
253	) -> Result<Routed, Stop> {
254	    route_target_by(resolved, profile, resolve_pane_target)
255	}
256	
257	/// [`route_target`], resolving a `--pane` target with `resolve` (`answer` resolves it with
258	/// `answer_cmd::answer_pane_target`, #831).
259	pub(crate) fn route_target_by(
260	    resolved: Result<PromptArgs, Usage>,
261	    profile: &ProfileOpt,
262	    resolve: fn(Ports<'_>, &str, Option<&str>) -> Result<String, PaneError>,
263	) -> Result<Routed, Stop> {
264	    let args = resolved.map_err(|usage| Stop {
265	        message: usage.to_string(),
266	        exit_code: ErrorClass::Usage.exit_code(),
267	    })?;
268	    let session = match (args.target, profile.profile.as_deref()) {
269	        (PromptTarget::Session(session), None) => session,
270	        (PromptTarget::Session(_), Some(_)) => {
271	            return Err(Stop {
272	                message: PROFILE_NEEDS_PANE.to_owned(),
273	                exit_code: ErrorClass::Usage.exit_code(),
274	            })
275	        }
276	        (PromptTarget::Pane(pane), profile) => Wiring::connect()
277	            .and_then(|wiring| resolve(wiring.ports(), &pane, profile))
278	            .map_err(|error| routing_stop(&error))?,
279	    };
280	    Ok(Routed {
281	        session,
282	        arg: args.arg,
283	    })
284	}
~~~~

**E124: `crates/holler-cli/src/interrupt_cmd.rs:177-186`** — `interrupt`'s refusal arm: the `--pane` handling is inline.

Command: `sed -n '177,186p' crates/holler-cli/src/interrupt_cmd.rs | awk -v s=177 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
177	        Err(holler_hub::control::ControlError::Refused(e)) => {
178	            // The pane arm comes first (story #646): the redirect passes
179	            // `send_prompt`, whose pane-state gate is no hold to release.
180	            if let Some(refused) = pane_hold_refusal(&session, &e, json) {
181	                return InterruptResult {
182	                    message: refused.message,
183	                    to_stderr: refused.to_stderr,
184	                    exit_code: refused.exit_code,
185	                };
186	            }
~~~~

**E125: `crates/holler-cli/src/answer_cmd.rs:88-97`** — `answer`'s refusal arm.

Command: `sed -n '88,97p' crates/holler-cli/src/answer_cmd.rs | awk -v s=88 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
88	        Err(holler_hub::control::ControlError::RemotePolicyRefused(msg)) => err(msg, 3),
89	        Err(holler_hub::control::ControlError::Refused(e)) => {
90	            // `ambiguous` (matching `say`'s own convention: "lists
91	            // candidates") is the one refusal that exits 2, not 1 — every
92	            // other refusal (unknown session, nothing_pending, not_connected,
93	            // invalid_params, connection_lost) is a runtime failure (exit 1).
94	            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
95	            err(e.message.clone(), if is_ambiguous { 2 } else { 1 })
96	        }
97	        Err(e) => err(e.to_string(), 1),
~~~~

**E126: `crates/holler-cli/src/prompt_target.rs:41-46`** — `Routed { session, arg }`: it does not record that a target came from `--pane`.

Command: `sed -n '41,46p' crates/holler-cli/src/prompt_target.rs | awk -v s=41 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
41	/// A prompt verb that is cleared to contact a hub: the session to prompt and the argument after it.
42	#[derive(Debug, Clone, PartialEq, Eq)]
43	pub struct Routed {
44	    pub session: String,
45	    pub arg: Option<String>,
46	}
~~~~

**E127: `crates/holler-cli/src/pane/wiring/bind.rs:346-353`** — `wiring/bind.rs`'s pass-through of `send_prompt` (`impl HarnessPort for Wired<dyn HarnessPort>`): one of the two `.send_prompt(` calls the scan allows; the other is the router's (E159).

Command: `sed -n '346,353p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v s=346 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
346	    fn send_prompt(
347	        &self,
348	        at: &HarnessEndpoint,
349	        session: &str,
350	        text: &str,
351	    ) -> Result<(), PaneError> {
352	        self.port()?.send_prompt(at, session, text)
353	    }
~~~~

**E128: `crates/holler-pane/src/tx_launch.rs:473-484`** — `launch` and `relaunch` keep the record's stored DRIVEN.

Command: `sed -n '473,484p' crates/holler-pane/src/tx_launch.rs | awk -v s=473 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
473	    /// The record R writes: what the act brought up and observed, and the spec's values. A
474	    /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
475	    /// profile, its profile.
476	    fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
477	        let (hold, driven, profile) = match self.old {
478	            Some(old) => (
479	                old.hold.clone(),
480	                old.last_observed.driven.clone(),
481	                self.profile.or(old.profile.as_ref()).cloned(),
482	            ),
483	            None => (Hold::None, None, self.profile.cloned()),
484	        };
~~~~

**E129: `crates/holler-pane/src/tx_launch/server.rs:126-140`** — A relaunch keeps the session of record only if the restarted server lists it, else creates a new one.

Command: `sed -n '126,140p' crates/holler-pane/src/tx_launch/server.rs | awk -v s=126 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
126	/// Step 5: a new session, created through the harness API. For a relaunch: the record's session
127	/// of record when the restarted server still lists it, else a new one (decision 8).
128	fn session_of_record(
129	    ports: Ports<'_>,
130	    plan: &Plan<'_>,
131	    at: &HarnessEndpoint,
132	) -> Result<String, PaneError> {
133	    if let Some(kept) = plan.old.and_then(|old| old.session_of_record.as_deref()) {
134	        let listed = ports.harness.list_sessions(at)?;
135	        if listed.iter().any(|session| session == kept) {
136	            return Ok(kept.to_owned());
137	        }
138	    }
139	    ports.harness.create_session(at)
140	}
~~~~

**E130: `docs/adr/ADR-0003.md:117-120`** — ADR 0003 "Not decided here", the Addressing line (line 119 on `main`).

Command: `sed -n '117,120p' docs/adr/ADR-0003.md | awk -v s=117 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
117	## Not decided here
118	
119	- **Addressing** — `SESSION`/`TARGET` expand under ADR 0005 (`<label>/<session>`).
120	- **Wire** — the `--json` document shapes follow ADR 0004 (protocol v2).
~~~~

**E131: `docs/adr/ADR-0014.md:24-30`** — ADR 0014 Consequences, with the bullet "without it changing how they address the session".

Command: `sed -n '24,30p' docs/adr/ADR-0014.md | awk -v s=24 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
24	## Consequences
25	
26	- The body's config schema gains `mode`, `endpoint`, and `session_id` fields, meaningful only under attach; `command` is ignored (never used as a spawn fallback) when `mode = "attach"`.
27	- Session-manager teardown branches on mode: spawn tears down its ACP child; attach drops only its own control channel and returns, leaving the harness process running exactly as it found it.
28	- Single-instance locking on `holler body run` still applies per body process; two bodies attaching the *same* `session_id` concurrently is undefined and the second one's lock attempt fails, same as today.
29	- `roster`/`status` surface `mode` and the harness session id so an operator can tell attach apart from spawn without it changing how they address the session.
30	- Discovering an attachable session automatically (rather than an operator pasting `endpoint` + `session_id`) is explicitly out of scope for v1 attach and remains a follow-on.
~~~~

**E132: `crates/holler-proto/tests/codec_test.rs:82-87`** — `every_method_round_trips` panics `no canonical frame` for a catalog row without an arm.

Command: `sed -n '82,87p' crates/holler-proto/tests/codec_test.rs | awk -v s=82 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
82	        "admin/answer" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/answer","params":{{"session":"io/alpha","choice":"0"}}}}"#),
83	        "admin/wait" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/wait","params":{{"sessions":"io/alpha","prefix":null,"until":null,"after":null,"timeout_ms":600000}}}}"#),
84	        "admin/query" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/query","params":{{"target":null,"method":"query/status","params":null}}}}"#),
85	        _ => panic!("no canonical frame for {method:?}"),
86	    }
87	}
~~~~

**E133: `crates/holler-body/tests/http_attach_driver_test/fake_server.rs:32-36`** — The fake OpenCode server's state: `exists_v1` and `exists_v2`, no stall mode.

Command: `sed -n '32,36p' crates/holler-body/tests/http_attach_driver_test/fake_server.rs | awk -v s=32 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
32	pub struct FakeState {
33	    pub exists_v1: bool,
34	    pub exists_v2: bool,
35	    pub questions: Vec<Value>,
36	    pub permissions: Vec<Value>,
~~~~

**E134: `crates/holler-body/tests/http_attach_driver_test/fake_server.rs:276-292`** — The fake's existence check answers at once, one verdict for every id.

Command: `sed -n '276,292p' crates/holler-body/tests/http_attach_driver_test/fake_server.rs | awk -v s=276 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
276	    let mut state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
277	    state.requests.push(RecordedRequest {
278	        method: method.to_string(),
279	        path: path.to_string(),
280	        body: body.to_string(),
281	    });
282	
283	    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
284	
285	    match (method, segments.as_slice()) {
286	        ("GET", ["session", id]) if !id.is_empty() => {
287	            if state.exists_v1 {
288	                ok_json("{}")
289	            } else {
290	                not_found()
291	            }
292	        }
~~~~

**E135: `crates/holler-pane-testkit/Cargo.toml:12-24`** — `holler-pane-testkit`'s manifest: it depends only on `holler-pane` and `serde_json`.

Command: `sed -n '12,24p' crates/holler-pane-testkit/Cargo.toml | awk -v s=12 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
12	# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
13	# otherwise).
14	[dependencies]
15	# The ports the fakes implement and the suites drive (`PaneStore`, `Pane`, `PaneError`,
16	# `next_generation`); for every fake and suite in this crate.
17	holler-pane = { path = "../holler-pane" }
18	# Parses the verbs' `--format=json` output for `envelope.rs` (`check_envelope`,
19	# `check_ndjson`); the testkit cannot use the CLI's own `Envelope` type.
20	serde_json = { workspace = true }
21	
22	# Workspace lints (issue #149).
23	[lints]
24	workspace = true
~~~~

**E136: `crates/holler-proto/src/vocab.rs:122-149`** — `RoutableName::parse`: `label/session` or a bare session; `_` is refused.

Command: `sed -n '122,149p' crates/holler-proto/src/vocab.rs | awk -v s=122 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
122	impl RoutableName {
123	    /// Parse a wire string into a `RoutableName`.
124	    ///
125	    /// Succeeds iff the string is one or more `/`-separated segments, each
126	    /// 1–32 chars, each matching the segment grammar (lowercase letters and
127	    /// digits; `-` only between word chars; no leading/trailing `-`).
128	    pub fn parse(s: &str) -> Result<Self, NameError> {
129	        if s.is_empty() {
130	            return Err(NameError::Empty);
131	        }
132	        let bytes = s.as_bytes();
133	        let mut start = 0usize;
134	        for (i, &b) in bytes.iter().enumerate() {
135	            if b == b'/' {
136	                if i == start {
137	                    return Err(NameError::EmptySegment);
138	                }
139	                check_segment(&bytes[start..i])?;
140	                start = i + 1;
141	            }
142	        }
143	        if start == bytes.len() {
144	            // Trailing `/`.
145	            return Err(NameError::EmptySegment);
146	        }
147	        check_segment(&bytes[start..])?;
148	        Ok(Self(s.to_owned()))
149	    }
~~~~

**E137: `crates/holler-pane/src/tx_switch.rs:424-443`** — `check_unclaimed`: "Servers that share a data directory list every pane's sessions, so the server's list alone would let a switch show one pane another's conversation". The shared-listing signal, and the reason a listed id is no proof that a body belongs to a pane.

Command: `sed -n '424,443p' crates/holler-pane/src/tx_switch.rs | awk -v s=424 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
424	/// A switch's target is no other pane's session of record. Servers that share a data
425	/// directory list every pane's sessions, so the server's list alone would let a switch show
426	/// one pane another's conversation. The first such record in list order is named.
427	fn check_unclaimed(ports: Ports<'_>, pane: &PaneName, target: &str) -> Result<(), PaneError> {
428	    let records = ports.pane_store.list()?;
429	    let claimed = records
430	        .iter()
431	        .find(|other| other.name != *pane && other.session_of_record.as_deref() == Some(target));
432	    match claimed {
433	        Some(other) => Err(PaneError::Refused {
434	            code: SESSION_OF_OTHER_PANE,
435	            message: format!(
436	                "{} is the session of record of {}",
437	                quoted(target),
438	                other.name
439	            ),
440	        }),
441	        None => Ok(()),
442	    }
443	}
~~~~

**E138: `crates/holler-hub/src/talk.rs:196-246`** — `talk::say` in full up to `send_turn`: the prelude that `talk::admit` is extracted from (parse and resolve, the hold fast path, the busy check).

Command: `sed -n '196,246p' crates/holler-hub/src/talk.rs | awk -v s=196 '{printf "%d\t%s\n", s+NR-1, $0}'`

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
219	    // The busy check (issue #150's policy, enforced here as the hub's own
220	    // fail-closed gate — the body's `SessionManager` enforces it again,
221	    // independently, so a race still fails closed).
222	    match ad.state {
223	        SessionState::InputRequired => {
224	            let question = ad.pending.as_ref().and_then(|items| items.first()).map(|item| item.prompt.clone());
225	            let turn_age_ms = age_ms(ad.turn_started_at.as_deref());
226	            let last_update_age_ms = age_ms(ad.last_update_at.as_deref());
227	            return Err(SayError::InputRequired { question, turn_age_ms, last_update_age_ms });
228	        }
229	        SessionState::Working if !queue => {
230	            let turn_age_ms = age_ms(ad.turn_started_at.as_deref());
231	            let last_update_age_ms = age_ms(ad.last_update_at.as_deref());
232	            // `stalled` is a roster-derived display state (docs.rs's own
233	            // comment on `SessionAd.last_update_at`: "working with no update
234	            // for HOLLER_STALL_MS displays as roster's derived stalled
235	            // state") — #186 (roster) owns the real derivation; this is the
236	            // minimal stand-in `say`'s own busy hint needs (see the module
237	            // doc's "Decisions I made").
238	            let state = if last_update_age_ms >= stall_threshold_ms() { "stalled" } else { "working" };
239	            return Err(SayError::Busy { state: state.to_string(), turn_age_ms, last_update_age_ms });
240	        }
241	        _ => {}
242	    }
243	
244	    let kind = if queue { TurnKind::Queue } else { TurnKind::Plain };
245	    send_turn(&TurnCtx { registry, roster, state, grant }, &handle, &ad, text, kind, timeout).await
246	}
~~~~

**E139: `crates/holler-cli/tests/hold_single_path_test.rs:70-81`** — The scan of the generic forward: the first `LiveCommand::Query` after `pub async fn query(` in `live.rs` must come after the allowlist check, so the send-and-wait helper has to sit after `query` in `live.rs`.

Command: `sed -n '70,81p' crates/holler-cli/tests/hold_single_path_test.rs | awk -v s=70 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
70	#[test]
71	fn the_generic_request_forward_allows_only_query_and_answer() {
72	    // `LiveHandle::query` is the hub's generic "forward this method to the
73	    // body" path (`hub query TARGET ...`, `answer`). It must not carry a prompt.
74	    let live = code_of(&hub_src().join("live.rs"));
75	    let start = live.find("pub async fn query(").expect("LiveHandle::query exists");
76	    let body = &live[start..];
77	    assert!(
78	        body.find("method.starts_with(\"query/\")").is_some_and(|i| i < body.find("LiveCommand::Query").unwrap()),
79	        "LiveHandle::query must reject anything but query/* and session/answer before forwarding"
80	    );
81	}
~~~~

**E140: `crates/holler-hub/src/control.rs:15-16`** — `CLIENT_TIMEOUT`, the CLI's bound for a one-shot control call (5 s); `hub query` and `answer` use it, and the hub forwards a `query_remote` for 10 s (E98).

Command: `sed -n '15,16p' crates/holler-hub/src/control.rs | awk -v s=15 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
15	/// The timeout for a one-shot control exchange (the spec: 5 s).
16	const CLIENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
~~~~

**E141: `crates/holler-pane/src/findings.rs:133-145`** — `FindingKind::remedy` and its doc: "The one table of remedies".

Command: `sed -n '133,145p' crates/holler-pane/src/findings.rs | awk -v s=133 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
133	    /// The command a finding of this kind tells the operator to run, for the pane `pane`
134	    /// (`None` when it is about no pane) once `--fix` left it at `fix`; `None` when no Holler
135	    /// verb repairs it. The one table of remedies:
136	    ///
137	    /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
138	    /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
139	    /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
140	    ///   one (choosing an existing session is the operator's, never a guess);
141	    /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
142	    ///   it cannot or failed, nothing once it is fixed;
143	    /// - a pane that waits for a person, and a login that is not the subscription's (#831):
144	    ///   none, since only a person answers the one and logs in for the other.
145	    pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
~~~~

**E142: `crates/holler-pane/src/findings.rs:319-355`** — `Finding::with_fix`, which recomputes `remedy` from the table, and the command builders `doctor_command`, `relaunch_command` and `reset_command` beside which `switch_command` goes.

Command: `sed -n '319,355p' crates/holler-pane/src/findings.rs | awk -v s=319 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
319	    /// The same finding, left at `fix` by `--fix` (with `error` when it failed), with the
320	    /// remedy for that state.
321	    pub(crate) fn with_fix(self, fix: FixState, error: Option<FixError>) -> Self {
322	        Self {
323	            remedy: self.kind.remedy(self.pane.as_ref(), fix),
324	            fix,
325	            fix_error: error,
326	            ..self
327	        }
328	    }
329	}
330	
331	/// The `holler pane doctor` command line for `pane` (every pane when `None`), with `--fix`
332	/// when `fix`. It is the reconcile step another verb prints after a failure (ADR-0021
333	/// sections 8 and 12), so a verb builds it here rather than spelling it again.
334	pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String {
335	    let mut line = DOCTOR.to_owned();
336	    if let Some(pane) = pane {
337	        line.push(' ');
338	        line.push_str(pane.as_str());
339	    }
340	    if fix {
341	        line.push_str(" --fix");
342	    }
343	    line
344	}
345	
346	/// `holler pane relaunch <pane>`.
347	fn relaunch_command(pane: &PaneName) -> String {
348	    format!("{RELAUNCH} {pane}")
349	}
350	
351	/// `holler pane reset <pane>`.
352	fn reset_command(pane: &PaneName) -> String {
353	    format!("{RESET} {pane}")
354	}
355	
~~~~

**E143: `crates/holler-body/src/session_manager.rs:202-225`** — `SessionManager::start`: the presence is built with `SessionPresence::default()` before the task is spawned.

Command: `sed -n '202,225p' crates/holler-body/src/session_manager.rs | awk -v s=202 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
202	    pub fn start(registry: &SessionRegistry) -> Self {
203	        // Capacity is advisory (lossy) for a broadcast channel: a lagging
204	        // subscriber misses old presence-changed notices, never blocks a
205	        // session task's own send. 64 headroom for however many
206	        // sessions/changes land between two drains of a slow subscriber.
207	        let (presence_tx, _first_rx) = broadcast::channel(64);
208	        let mut sessions = BTreeMap::new();
209	        for (name, entry) in registry.iter() {
210	            let presence = Arc::new(Mutex::new(SessionPresence::default()));
211	            let (tx, rx) = mpsc::channel(MAILBOX_CAP);
212	            let inner = task::Inner::new(
213	                entry.config.clone(),
214	                presence.clone(),
215	                presence_tx.clone(),
216	                name.clone(),
217	            );
218	            let join = tokio::spawn(task::run(rx, inner));
219	            sessions.insert(
220	                name.clone(),
221	                ManagedSession { config: entry.config.clone(), tx, presence, join },
222	            );
223	        }
224	        Self { sessions, presence_tx }
225	    }
~~~~

**E144: `crates/holler-body/src/session_manager/task.rs:58-78`** — `SessionPresence::default` and its doc: `run` sets `omitted_pending_attach` when the task first runs, not when the presence is built.

Command: `sed -n '58,78p' crates/holler-body/src/session_manager/task.rs | awk -v s=58 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
58	impl Default for SessionPresence {
59	    /// Every session starts `idle` — no turn has run yet (mirrors
60	    /// [`crate::registry::SessionRegistry::from_sessions`]'s own baseline).
61	    /// Hand-written rather than `#[derive(Default)]` because `SessionState`
62	    /// (a `holler_proto` type) has no `Default` impl of its own to derive
63	    /// against — implementing one here would be an orphan-rule violation
64	    /// (neither this crate's trait nor this crate's type). `omitted_pending_
65	    /// attach` defaults `false`; [`run`] flips it `true` right away for an
66	    /// attach-mode session, before its own first attach attempt, so there is
67	    /// never a window where a not-yet-attached session is visible.
68	    fn default() -> Self {
69	        Self {
70	            state: SessionState::Idle,
71	            turn_started_at: None,
72	            last_update_at: None,
73	            turn_id: None,
74	            last_turn: None,
75	            pending: None,
76	            omitted_pending_attach: false,
77	        }
78	    }
~~~~

**E145: `crates/holler-cli/src/pane/wiring/store.rs:214-224`** — `Effect`: what a request does to its registry, which decides what a lost reply means.

Command: `sed -n '214,224p' crates/holler-cli/src/pane/wiring/store.rs | awk -v s=214 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
214	/// What a request does to its registry, which decides what a lost reply means (D3).
215	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
216	enum Effect {
217	    /// It reads: a request that failed changed nothing.
218	    Read,
219	    /// It writes: a request whose reply was lost may have landed.
220	    Write,
221	}
222	
223	/// One store method: its wire name (E9), the `op` its `timeout` names, what it does, and the
224	/// bound of one request.
~~~~

**E146: `crates/holler-cli/src/pane/wiring/store.rs:337-397`** — `call`, `timed_out`, `hub_unavailable` and `undecodable`: the one mapping of a failed exchange to a `PaneError` that `wiring/hub.rs` reuses (`call` itself decodes the `pane/*` reply envelope).

Command: `sed -n '337,397p' crates/holler-cli/src/pane/wiring/store.rs | awk -v s=337 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
337	
338	/// Send `method` with `params` to the hub, within `timeout`, through the CLI's one dispatch
339	/// point, and answer the reply's `data` or its error. The one mapping of a failed exchange (D3,
340	/// the module docs' table): `effect` says whether a lost reply may have landed, and `op` is what
341	/// a `timeout` names.
342	fn call(
343	    method: &'static str,
344	    params: Value,
345	    timeout: Duration,
346	    effect: Effect,
347	    op: &'static str,
348	) -> Result<Option<Value>, PaneError> {
349	    let request = ControlCall {
350	        method,
351	        params: Some(params),
352	        timeout,
353	    };
354	    let result = crate::transport::call(None, &request).map_err(|error| {
355	        if timed_out(&error, effect) {
356	            PaneError::Timeout { op: op.to_owned() }
357	        } else {
358	            hub_unavailable(method, &error)
359	        }
360	    })?;
361	    let reply: PaneReply =
362	        serde_json::from_value(result).map_err(|error| undecodable(method, &error))?;
363	    reply.into_result()
364	}
365	
366	/// Whether a failed exchange is a `timeout` (D3): no reply within the bound, for any method, and
367	/// for a write also a reply lost in flight, since the write may have landed.
368	fn timed_out(error: &ControlError, effect: Effect) -> bool {
369	    match (error, effect) {
370	        (ControlError::Io(io), _)
371	            if matches!(io.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
372	        {
373	            true
374	        }
375	        (ControlError::Io(_) | ControlError::BadReply(_), Effect::Write) => true,
376	        _ => false,
377	    }
378	}
379	
380	/// `unavailable`, naming the hub: `no live holler hub reachable at <state dir>`, the CLI's own
381	/// text, when there is none, else which hub did not answer `method`, and how.
382	fn hub_unavailable(method: &str, error: &ControlError) -> PaneError {
383	    let state = holler_hub::state::resolve_state_dir().unwrap_or_default();
384	    let state = state.display();
385	    let what = match error {
386	        ControlError::NoLiveHub => format!("no live holler hub reachable at {state}"),
387	        other => format!("the holler hub at {state} did not answer {method}: {other}"),
388	    };
389	    PaneError::Unavailable { what }
390	}
391	
392	/// `unavailable` naming `method`: its reply, or the reply's `data`, does not decode (W-9).
393	fn undecodable(method: &str, error: &serde_json::Error) -> PaneError {
394	    PaneError::Unavailable {
395	        what: format!("{method}: the hub's reply does not decode: {error}"),
396	    }
397	}
~~~~

**E147: `crates/holler-pane-testkit/src/fault.rs:14-60`** — `PortOp`, `Fault` and `FaultSwitch<Op>`: the faults and the call log every fake of the kit takes, which `FakeHub` takes too with a `HubOp`.

Command: `sed -n '14,60p' crates/holler-pane-testkit/src/fault.rs | awk -v s=14 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
14	
15	use holler_pane::PaneError;
16	
17	/// A port method that a fault can target and the call log records. Each fake has its
18	/// own enum of them, e.g. [`crate::pane_store::PaneStoreOp`].
19	pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static {
20	    /// `"<port>.<method>"`, e.g. `"pane_store.cas_put"`. It is also the `op` of the
21	    /// `timeout` a wedged call answers.
22	    fn as_str(self) -> &'static str;
23	}
24	
25	/// A standing fault: it applies to every call until it is cleared.
26	#[derive(Debug, Clone, PartialEq, Eq)]
27	pub enum Fault {
28	    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
29	    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
30	    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
31	    /// timer fire.
32	    Wedged,
33	    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
34	    Fail(PaneError),
35	}
36	
37	/// The faults of one fake port and the log of the calls made through it.
38	///
39	/// Every method takes `&self`, so a test can share the fake behind an `Arc` and still
40	/// drive its faults.
41	pub struct FaultSwitch<Op: PortOp> {
42	    state: Mutex<State<Op>>,
43	}
44	
45	/// What the switch holds, behind its lock.
46	struct State<Op> {
47	    standing: Option<Fault>,
48	    /// The one-shot errors, oldest first. Each one fails the next call of its method.
49	    queued: Vec<(Op, PaneError)>,
50	    delay: Option<Duration>,
51	    calls: Vec<Op>,
52	}
53	
54	impl<Op: PortOp> FaultSwitch<Op> {
55	    /// A switch with no fault, no delay and an empty call log.
56	    pub fn new() -> Self {
57	        Self {
58	            state: Mutex::new(State {
59	                standing: None,
60	                queued: Vec::new(),
~~~~

**E148: `crates/holler-cli/tests/pane_verbs/switch.rs:699-710`** — `SeamHarness`'s `QueuedPrompt.then_of_record`: how a test sees the order of a call on one fake against a write on another.

Command: `sed -n '699,710p' crates/holler-cli/tests/pane_verbs/switch.rs | awk -v s=699 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
699	    }
700	}
701	
702	/// One prompt a [`SeamHarness`] queued: the port and session it went to, the text,
703	/// and the pane's session of record at prompt time — the prompt follows the record
704	/// write only if that is the prompted session (#645 part 2's ordering pin).
705	#[derive(Debug, Clone, PartialEq, Eq)]
706	pub(crate) struct QueuedPrompt {
707	    pub port: u16,
708	    pub session: String,
709	    pub text: String,
710	    pub then_of_record: Option<String>,
~~~~

**E149: `crates/holler-cli/src/say_cmd.rs:184-206`** — The routed verbs' refusal arm: the pane hold first (`pane_hold_refusal`), then the generic held arm with `HELD_EXIT_CODE` (4) for an operator or default hold.

Command: `sed -n '184,206p' crates/holler-cli/src/say_cmd.rs | awk -v s=184 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
184	/// A control call that did not answer with a reply, as `say` prints it.
185	fn control_refusal(session: &str, e: ControlError, json: bool) -> SayResult {
186	    match e {
187	        ControlError::RemotePolicyRefused(msg) => err(msg, 3),
188	        ControlError::Refused(e) => {
189	            // The pane arm comes first (story #646): a pane-state refusal is
190	            // `session_held` on the wire but no hold to release.
191	            if let Some(result) = pane_hold_refusal(session, &e, json) {
192	                return result;
193	            }
194	            // `ambiguous` (issue #190 spec: "lists candidates") is the one
195	            // `say` refusal that exits 2, not 1 — every other refusal
196	            // (unknown session, session_busy, not_connected, connection_lost,
197	            // cancelled) is a runtime failure (exit 1).
198	            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
199	            if crate::hold_cmd::is_held(&e) {
200	                let (message, to_stderr) = crate::hold_cmd::held_refusal(session, &e, json);
201	                return SayResult { message, to_stderr, exit_code: crate::hold_cmd::HELD_EXIT_CODE };
202	            }
203	            if crate::hold_cmd::is_invalid_grant(&e) {
204	                let (message, to_stderr) =
205	                    crate::hold_cmd::invalid_grant_refusal(session, &e, json);
206	                return SayResult { message, to_stderr, exit_code: crate::hold_cmd::INVALID_GRANT_EXIT_CODE };
~~~~

**E150: `crates/holler-cli/tests/pane_verbs/target_flags.rs:605-621`** — #646's pin: every `session_held` that is not `hold_kind: "pane"` falls through to the generic held arm.

Command: `sed -n '605,621p' crates/holler-cli/tests/pane_verbs/target_flags.rs | awk -v s=605 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
605	/// Every other `-32011` (an operator hold, a default one, or one with no kind) falls
606	/// through to the generic held arm: the pane arm claims only `hold_kind == "pane"`.
607	#[test]
608	fn every_other_held_error_falls_through_to_the_generic_arm() {
609	    let mut operator = pane_held("freeze");
610	    if let Some(data) = operator.data.as_mut() {
611	        data.hold_kind = Some("operator".into());
612	    }
613	    assert!(pane_hold_refusal("ses-demo-c1r1", &operator, false).is_none());
614	
615	    let mut unkinded = pane_held("freeze");
616	    if let Some(data) = unkinded.data.as_mut() {
617	        data.hold_kind = None;
618	    }
619	    assert!(pane_hold_refusal("ses-demo-c1r1", &unkinded, false).is_none());
620	}
~~~~

**E151: `crates/holler-cli/src/hold_cmd.rs:15-19`** — `HELD_EXIT_CODE`: "so a script can tell held from failed".

Command: `sed -n '15,19p' crates/holler-cli/src/hold_cmd.rs | awk -v s=15 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
15	/// The exit code of a `say` (or `interrupt SESSION TEXT`) refused because the
16	/// session is held (`-32011 session_held`). Distinct from `1` (failed), `2`
17	/// (ambiguous) and `3` (malformed argument). Documented in `docs/protocol/
18	/// v2.md` §10 and pinned by `hold_cli_test`.
19	pub const HELD_EXIT_CODE: i32 = 4;
~~~~

**E152: `crates/holler-body/src/query.rs:54-61`** — `query/status`'s `session_list`: name, harness, state and `endpoint`, and no harness session id (the rejected join of P4's alternative (b) would read the endpoint).

Command: `sed -n '54,61p' crates/holler-body/src/query.rs | awk -v s=54 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
54	    let session_list = configs
55	        .iter()
56	        .map(|s| StatusSession {
57	            name: s.name.as_str().to_string(),
58	            harness: s.harness.clone(),
59	            state: WireSessionState::Idle,
60	            endpoint: s.endpoint.clone(),
61	        })
~~~~

**E153: `crates/holler-pane/src/tx_switch.rs:75-79`** — `session-of-other-pane`: "the switch target is another pane's session of record".

Command: `sed -n '75,79p' crates/holler-pane/src/tx_switch.rs | awk -v s=75 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
75	/// `server-unhealthy`: the pane's harness server does not answer `health`. Refusal, exit 3.
76	pub const SERVER_UNHEALTHY: RefusalCode = RefusalCode::from_static("server-unhealthy");
77	/// `session-of-other-pane`: the switch target is another pane's session of record.
78	/// Refusal, exit 3.
79	pub const SESSION_OF_OTHER_PANE: RefusalCode = RefusalCode::from_static("session-of-other-pane");
~~~~

**E154: `crates/holler-cli/src/roster_cmd.rs:328-343`** — #648 on `main`: `shown_driven_mismatch`, the third copy of the pair rule (its doc says "keep the three equal"), and `pane_label`'s doc, which names #839 as the place to change if the join moves.

Command: `sed -n '328,343p' crates/holler-cli/src/roster_cmd.rs | awk -v s=328 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
328	/// This is the roster's relation between a row and a pane. The hub's prompt gate and `say --pane`
329	/// relate them by the pane's session of record instead (ADR-0021 section 8); how the hub's DRIVEN
330	/// session follows that session is #839's, and this is the one place to change if it moves.
331	fn pane_label(name: &str) -> &str {
332	    name.split_once('/').map_or(name, |(label, _)| label)
333	}
334	
335	/// Whether a pane's recorded SHOWN and DRIVEN are a mismatch: both known and different. One side
336	/// absent is never one. It is the pair rule of `say --pane`'s refusal
337	/// (`say_cmd::pane_state_refusal`, `pane-shown-driven-mismatch`) and of its twin at the hub's
338	/// prompt gate (`holler-hub/src/circuit/dispatch.rs`, `pane_state_refusal`): keep the three equal
339	/// (a test pins this one to `say_cmd::resolve_pane_target`). It is not `reconcile::shown_differs`,
340	/// which compares SHOWN with the session of record.
341	fn shown_driven_mismatch(shown: Option<&str>, driven: Option<&str>) -> bool {
342	    matches!((shown, driven), (Some(shown), Some(driven)) if shown != driven)
343	}
~~~~

**E155: `crates/holler-cli/tests/pane_integration/scenario.rs:91-99`** — #667 on `main`: the scenario's steps 2 and 3, where `reset_with_a_first_message(&scratch, p2)` is called (the function is E86).

Command: `sed -n '91,99p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=91 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
91	    // 2. A relaunch of one: the same port, its session of record kept.
92	    let kept = scratch.session_of_record(C1);
93	    scratch.json(&["pane", "relaunch", C1], 0);
94	    no_model_can_be_reached(p1);
95	    assert_eq!(scratch.session_of_record(C1), kept, "relaunch keeps the session");
96	
97	    // 3. A reset of one, with a first message.
98	    reset_with_a_first_message(&scratch, p2);
99	
~~~~

**E156: `f8d2624:docs/handoffs/647-part2-brief.md:166-166`** — #647 part 2's brief at `f8d2624`: its table row for `findings.rs` ("two kinds, `remedy_for`, `Finding::with_agent`").

Command: `git show f8d2624:docs/handoffs/647-part2-brief.md | sed -n '166,166p' | awk -v s=166 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
166	| `crates/holler-pane/src/findings.rs` | 357 | 357 | 373 | two kinds, `remedy_for`, `Finding::with_agent` | yes |
~~~~

**E157: `crates/holler-pane/src/reconcile.rs:17-21`** — #665 on `main`: reconcile's module doc: the `profile-drift` rule and its file `reconcile/drift.rs`; with it `findings.rs` has the appended `FindingKind::ProfileDrift` (E38's neighbours).

Command: `sed -n '17,21p' crates/holler-pane/src/reconcile.rs | awk -v s=17 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
17	//!    against every record, not only the scope, so another profile's session of record is
18	//!    never a stray; unregistered Herdr panes are reported by whole-fleet passes only. A pane
19	//!    that differs from its profile's spec is `profile-drift` (#665, `drift.rs`), for which
20	//!    the pass reads `profile_store.list` once, and only when a pane in scope names a profile.
21	//! 3. **Repair**, with `--fix`, one thing: a shown/driven mismatch, by selecting the session
~~~~

**E158: `crates/holler-pane/src/tx_switch.rs:280-297`** — `tx_switch::prompt` (step 5 of a switch or reset): the engine's one `send_prompt` call, at line 295. It is the call F removes; `hub.prompt(.., Send)` takes its place.

Command: `sed -n '280,297p' crates/holler-pane/src/tx_switch.rs | awk -v s=280 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
280	/// Step 5: queue `text` to the session of record of `pane`, the record the run stored, at its
281	/// endpoint: the process a relaunch started, never the stopped one (plan review W4). The prompt
282	/// goes only to the session just recorded (I2), so a failure here leaves the TUI and the record
283	/// in agreement: no reconcile step, nothing unrecorded.
284	fn prompt(ports: Ports<'_>, pane: &Pane, text: &str) -> Result<(), SwitchFailure> {
285	    let target = pane.session_of_record.clone().unwrap_or_default();
286	    let unprompted = |error| SwitchFailure {
287	        error,
288	        acted: false,
289	        created: None,
290	        unprompted: Some(target.clone()),
291	    };
292	    let at = pane.harness.endpoint(&pane.name).map_err(unprompted)?;
293	    ports
294	        .harness
295	        .send_prompt(&at, &target, text)
296	        .map_err(unprompted)
297	}
~~~~

**E159: `crates/holler-pane/src/ports/router.rs:112-119`** — `HarnessRouter::send_prompt` (#831): the second wrapper pass-through, with `.send_prompt(` at line 118. D5's scan allows it, beside `wiring/bind.rs` (E127).

Command: `sed -n '112,119p' crates/holler-pane/src/ports/router.rs | awk -v s=112 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
112	    fn send_prompt(
113	        &self,
114	        at: &HarnessEndpoint,
115	        session: &str,
116	        text: &str,
117	    ) -> Result<(), PaneError> {
118	        self.by(at).send_prompt(at, session, text)
119	    }
~~~~

**E160: `crates/holler-hub/src/control_server.rs:318-354`** — `control/say`'s `SayError` arms: the resolve-time `NotConnected` (line 321) and, after delivery, `Timeout` (347-350) and `Cancelled` (351-353), all `Code::NotConnected` (`-32004`) with no `data`.

Command: `sed -n '318,354p' crates/holler-hub/src/control_server.rs | awk -v s=318 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
318	        // Issue #192, rule 4: `detail` already carries the roster's own
319	        // `conn_state` (and, for `reconnecting`, the last-seen age) when the
320	        // roster still remembers this name — see `talk::not_connected_detail`.
321	        Err(crate::talk::SayError::NotConnected(detail)) => encode_error(cid, Code::NotConnected, detail),
322	        Err(crate::talk::SayError::Ambiguous(candidates)) => {
323	            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
324	            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
325	            encode_error_frame(cid, &err)
326	        }
327	        Err(crate::talk::SayError::Busy { state, turn_age_ms, last_update_age_ms }) => {
328	            let err = WireError::session_busy(state, turn_age_ms, last_update_age_ms);
329	            encode_error_frame(cid, &err)
330	        }
331	        // Issue #151: reported as `-32009 session_busy` with `data.state
332	        // = "input-required"` (distinct from `working`/`stalled`) so the
333	        // CLI's own busy hint (`say_cmd::busy_hint`) can point at `answer`
334	        // instead of `interrupt`/`--queue` — neither of which resolves a
335	        // held permission/elicitation.
336	        Err(crate::talk::SayError::InputRequired { question, turn_age_ms, last_update_age_ms }) => {
337	            let mut err = WireError::session_busy("input-required", turn_age_ms, last_update_age_ms);
338	            if let Some(q) = question {
339	                err.message = q;
340	            }
341	            encode_error_frame(cid, &err)
342	        }
343	        Err(crate::talk::SayError::Refused(err)) => encode_error_frame(cid, &err),
344	        Err(crate::talk::SayError::ConnectionLost) => {
345	            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"))
346	        }
347	        Err(crate::talk::SayError::Timeout) => {
348	            let secs = timeout.as_secs();
349	            encode_error(cid, Code::NotConnected, format!("no reply from {session} within {secs}s"))
350	        }
351	        Err(crate::talk::SayError::Cancelled) => {
352	            encode_error(cid, Code::NotConnected, "prompt was interrupted before it completed".to_string())
353	        }
354	    }
~~~~

**E161: `crates/holler-hub/src/control_server.rs:384-409`** — `control/interrupt`'s arms: the resolve-time `NotConnected` (387-389) and, after the cancel was sent, `AckTimeout` (395-402) and the redirect's `PromptFailed` (408), all `-32004` with no `data`.

Command: `sed -n '384,409p' crates/holler-hub/src/control_server.rs | awk -v s=384 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
384	        Err(crate::interrupt::InterruptError::UnknownSession) => {
385	            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
386	        }
387	        Err(crate::interrupt::InterruptError::NotConnected) => {
388	            encode_error(cid, Code::NotConnected, format!("{session}'s body is not connected"))
389	        }
390	        Err(crate::interrupt::InterruptError::Ambiguous(candidates)) => {
391	            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
392	            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
393	            encode_error_frame(cid, &err)
394	        }
395	        Err(crate::interrupt::InterruptError::AckTimeout { secs }) => encode_error(
396	            cid,
397	            Code::NotConnected,
398	            format!(
399	                "interrupt sent but not confirmed within {secs}s (body may be stalled); \
400	                 the session is still on the roster"
401	            ),
402	        ),
403	        Err(crate::interrupt::InterruptError::Refused(err)) => encode_error_frame(cid, &err),
404	        Err(crate::interrupt::InterruptError::ConnectionLost) => {
405	            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"))
406	        }
407	        Err(crate::interrupt::InterruptError::PromptFailed(crate::talk::SayError::Refused(err))) => encode_error_frame(cid, &err),
408	        Err(crate::interrupt::InterruptError::PromptFailed(e)) => encode_error(cid, Code::NotConnected, e.message()),
409	    }
~~~~

**E162: `crates/holler-hub/src/control_server.rs:430-445`** — `control/answer`'s arms: the resolve-time `NotConnected` (433-435) is its only `-32004`.

Command: `sed -n '430,445p' crates/holler-hub/src/control_server.rs | awk -v s=430 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
430	        Err(crate::talk::AnswerError::UnknownSession) => {
431	            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
432	        }
433	        Err(crate::talk::AnswerError::NotConnected) => {
434	            encode_error(cid, Code::NotConnected, format!("{session}'s body is not connected"))
435	        }
436	        Err(crate::talk::AnswerError::Ambiguous(candidates)) => {
437	            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
438	            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
439	            encode_error_frame(cid, &err)
440	        }
441	        Err(crate::talk::AnswerError::Refused(err)) => encode_error_frame(cid, &err),
442	        Err(crate::talk::AnswerError::ConnectionLost) => {
443	            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected; ask again"))
444	        }
445	    }
~~~~

**E163: `crates/holler-proto/src/error.rs:186-204`** — `Error::new(code, message, reason)`: the constructor that sets `data.reason`; `Error::offline` is this call with `Code::NotConnected` and the reason `"offline"` (P7).

Command: `sed -n '186,204p' crates/holler-proto/src/error.rs | awk -v s=186 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
186	impl Error {
187	    /// Build a conformant error from a known [`Code`]: the numeric JSON-RPC
188	    /// code plus the conventional `data` object (`{"code": <data_code>}`, and
189	    /// `{"reason": <reason>}` when `reason` is `Some`).
190	    pub fn new(code: Code, message: impl Into<String>, reason: Option<&'static str>) -> Self {
191	        Self {
192	            code: code.jsonrpc(),
193	            message: message.into(),
194	            data: Some(Box::new(ErrorData {
195	                code: code.data_code().to_owned(),
196	                reason: reason.map(str::to_owned),
197	                state: None,
198	                turn_age_ms: None,
199	                last_update_age_ms: None,
200	                since: None,
201	                hold_kind: None,
202	            })),
203	        }
204	    }
~~~~

**E164: `crates/holler-proto/src/error.rs:271-282`** — `Error::invalid_grant(reason: &'static str)`: the precedent for a constructor that names one `data.reason` of an existing code.

Command: `sed -n '271,282p' crates/holler-proto/src/error.rs | awk -v s=271 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
271	    /// Build a `-32012 invalid_grant` refusal (issue #460): `say --grant ID`
272	    /// carried a grant the hub will not honour. `reason` is one of `unknown`,
273	    /// `expired`, `used` or `other_session`.
274	    pub fn invalid_grant(reason: &'static str) -> Self {
275	        let message = match reason {
276	            "expired" => "grant has expired",
277	            "used" => "grant has already been used",
278	            "other_session" => "grant was issued for a different session",
279	            _ => "unknown grant",
280	        };
281	        Self::new(Code::InvalidGrant, message, Some(reason))
282	    }
~~~~

**E165: `crates/holler-proto/tests/golden_test.rs:137-151`** — `unauthenticated_error_frames_match_golden`: the precedent for a golden file per `data.reason` of an error frame; the offline `-32004` frame gets one beside it.

Command: `sed -n '137,151p' crates/holler-proto/tests/golden_test.rs | awk -v s=137 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
137	/// `-32002 unauthenticated` with the #486 `data.reason` a body acts on: the
138	/// hub-fault code it retries, and the two credential codes it names in plain
139	/// words before it stops.
140	#[test]
141	fn unauthenticated_error_frames_match_golden() {
142	    use holler_proto::noise::{HUB_UNAVAILABLE_REASON, TOKEN_NOT_BOUND_REASON, TOKEN_UNKNOWN_REASON};
143	    for (file, message, reason) in [
144	        ("hub_unavailable", "authentication failed: the token store could not be read", HUB_UNAVAILABLE_REASON),
145	        ("token_not_bound", "authentication failed: token tok_x is not bound", TOKEN_NOT_BOUND_REASON),
146	        ("token_unknown", "authentication failed: no such token tok_x", TOKEN_UNKNOWN_REASON),
147	    ] {
148	        let env = Envelope::Error { id: Some(id()), error: WireError::new(Code::Unauthenticated, message, Some(reason)) };
149	        assert_golden(&format!("envelope/error.unauthenticated_{file}.json"), &encode(&env).unwrap());
150	    }
151	}
~~~~

**E166: `scripts/golden-diff-summary.sh:2-24`** — `scripts/golden-diff-summary.sh`, its header: informational, canonicalizes each changed golden file's old and new blob; no per-file list is kept in it.

Command: `sed -n '2,24p' scripts/golden-diff-summary.sh | awk -v s=2 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
2	# Golden-file diff summary (issue #246): `BLESS=1 cargo test -p holler-proto
3	# --test golden_test` re-serializes every golden JSON file with
4	# `serde_json::to_string_pretty`, which (via the crate's `preserve_order`
5	# feature having been dropped) reorders every file's keys alphabetically as a
6	# side effect — even files whose *value* didn't change. That cosmetic churn
7	# can bury a real, unrelated value-drift regression inside a huge "reorder"
8	# diff (the drift-laundering failure mode in
9	# docs/reviews/overlays/tests-as-evidence.md).
10	#
11	# This canonicalizes each changed golden file's old and new git blob (sorted
12	# keys, compact — jq -Sc is equivalent to golden_test.rs's own
13	# tests/common::canonical for this purpose) and compares them: identical
14	# canonical forms means a pure key-reorder (expected, harmless fallout of
15	# BLESS=1); different canonical forms means a real value change that deserves
16	# reviewer attention, called out by name instead of getting lost in the
17	# reorder noise.
18	#
19	# Informational only — never fails the build (exit 0 always). Called from
20	# scripts/lint.sh.
21	set -uo pipefail
22	
23	golden_glob='crates/holler-proto/tests/golden/**/*.json'
24	
~~~~

**E167: `scripts/golden-diff-summary.sh:60-76`** — `scripts/golden-diff-summary.sh`: a golden file with no old blob is listed as `(new file)`; a changed one is a real value change or a reorder.

Command: `sed -n '60,76p' scripts/golden-diff-summary.sh | awk -v s=60 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
60	  new_raw=$(cat "$gf" 2>/dev/null) || new_raw=""
61	
62	  if [ -z "$old_raw" ]; then
63	    # New golden file (no prior blob) — nothing to compare against; not a
64	    # reorder, and not flagged as a "value change" either. Note it plainly.
65	    real_change+=("$gf (new file)")
66	    continue
67	  fi
68	
69	  old_canon=$(printf '%s' "$old_raw" | jq -Sc . 2>/dev/null)
70	  new_canon=$(printf '%s' "$new_raw" | jq -Sc . 2>/dev/null)
71	
72	  if [ -z "$old_canon" ] || [ -z "$new_canon" ]; then
73	    unreadable+=("$gf")
74	  elif [ "$old_canon" = "$new_canon" ]; then
75	    reorder_only+=("$gf")
76	  else
~~~~

**E168: `crates/holler-cli/src/say_cmd.rs:149-176`** — `say_cmd::run`: the call is built with the turn budget (`say_with(.., timeout)`), and only `call.timeout` is overridden to `QUEUE_ACCEPT_WAIT` for the accept-and-return form; `wiring/hub.rs` builds `Send` the same way.

Command: `sed -n '149,176p' crates/holler-cli/src/say_cmd.rs | awk -v s=149 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
149	pub fn run(say: &Say, json: bool) -> SayResult {
150	    let Routed { session, arg } = match route_target(say.resolve(), &say.profile) {
151	        Ok(routed) => routed,
152	        Err(stop) => return err(stop.message, stop.exit_code),
153	    };
154	    let timeout = match parse_duration(&say.timeout) {
155	        Some(d) => d,
156	        None => return err(format!("invalid --timeout {:?} (use e.g. 30s, 5m, 1h)", say.timeout), 3),
157	    };
158	    let text = match resolve_say_text(say, arg.as_deref()) {
159	        Ok(t) => t,
160	        Err(msg) => return err(msg, 3),
161	    };
162	    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
163	    let mut call = holler_hub::control::ControlCall::say_with(
164	        &session,
165	        &text,
166	        say.queue,
167	        say.grant.as_deref(),
168	        timeout,
169	    );
170	    let early = returns_on_acceptance(say);
171	    if early {
172	        call.timeout = QUEUE_ACCEPT_WAIT;
173	    }
174	    let reply = crate::transport::call(say.server.as_deref(), &call);
175	    match wait_ended(reply, early) {
176	        Ok(wait) => queue_outcome(&session, wait, json),
~~~~

**E169: `crates/holler-cli/src/interrupt_cmd.rs:97-115`** — `interrupt_cmd::route`: builds its own `Routed { session, arg }` for a `--pane` target the pane's harness aborts in place (`PaneInterrupt::Hub`); `Routed.pane` is set here too.

Command: `sed -n '97,115p' crates/holler-cli/src/interrupt_cmd.rs | awk -v s=97 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
97	/// Route `interrupt`: a SESSION, and a malformed tail, through `say`'s routing unchanged; a
98	/// `--pane` target through [`interrupt_pane`] over the run's wiring.
99	fn route(interrupt: &Interrupt) -> Result<Next, Stop> {
100	    let Ok(PromptArgs {
101	        target: PromptTarget::Pane(pane),
102	        arg,
103	    }) = interrupt.resolve()
104	    else {
105	        return route_target(interrupt.resolve(), &interrupt.profile).map(Next::Hub);
106	    };
107	    let profile = interrupt.profile.profile.as_deref();
108	    let answer = Wiring::connect()
109	        .and_then(|wiring| interrupt_pane(wiring.ports(), &pane, profile, arg.as_deref()))
110	        .map_err(|error| routing_stop(&error))?;
111	    Ok(match answer {
112	        PaneInterrupt::Hub { session } => Next::Hub(Routed { session, arg }),
113	        PaneInterrupt::Relaunched { session } => Next::Relaunched { pane, session },
114	    })
115	}
~~~~

**E170: `crates/holler-hub/src/circuit/dispatch.rs:456-482`** — `dispatch.rs`'s inline `mod hold_tests`: the `Recording` sink and the helpers are private to the module, which drives `send_prompt` with the private `HoldGate`; the pane-gate tests (E81, E82) are in it.

Command: `sed -n '456,482p' crates/holler-hub/src/circuit/dispatch.rs | awk -v s=456 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
456	#[cfg(test)]
457	#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442, #646
458	mod hold_tests {
459	    use super::*;
460	
461	    /// A sink that records what is written to it and cannot fail.
462	    #[derive(Default)]
463	    struct Recording(Vec<Message>);
464	
465	    impl Sink<Message> for Recording {
466	        type Error = WsError;
467	        fn poll_ready(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
468	            std::task::Poll::Ready(Ok(()))
469	        }
470	        fn start_send(mut self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), WsError> {
471	            self.0.push(item);
472	            Ok(())
473	        }
474	        fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
475	            std::task::Poll::Ready(Ok(()))
476	        }
477	        fn poll_close(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), WsError>> {
478	            std::task::Poll::Ready(Ok(()))
479	        }
480	    }
481	
482	    fn message() -> Box<holler_proto::Message> {
~~~~

**E171: `crates/holler-cli/tests/hold_single_path_test.rs:20-40`** — `rust_files` reads every `.rs` file of the hub, and `code_of` cuts each at its first `#[cfg(test)]`: an out-of-line test file is read unstripped, and a `mod` line near the top would hide the code below it.

Command: `sed -n '20,40p' crates/holler-cli/tests/hold_single_path_test.rs | awk -v s=20 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
20	fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
21	    for e in std::fs::read_dir(dir).unwrap() {
22	        let p = e.unwrap().path();
23	        if p.is_dir() {
24	            rust_files(&p, out);
25	        } else if p.extension().is_some_and(|x| x == "rs") {
26	            out.push(p);
27	        }
28	    }
29	}
30	
31	/// The file's code with `//` comments and everything from the first
32	/// `#[cfg(test)]` on removed (test modules are last in every hub file).
33	fn code_of(path: &Path) -> String {
34	    let text = std::fs::read_to_string(path).unwrap();
35	    let text = text.split("#[cfg(test)]").next().unwrap();
36	    text.lines()
37	        .map(|l| l.split("//").next().unwrap_or(""))
38	        .collect::<Vec<_>>()
39	        .join("\n")
40	}
~~~~

**E172: `crates/holler-hub/src/holds.rs:573-576`** — `holds.rs`: the hub's precedent for an out-of-line test module, `#[cfg(test)] mod tests;` as the last two lines of the file.

Command: `sed -n '573,576p' crates/holler-hub/src/holds.rs | awk -v s=573 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
573	}
574	
575	#[cfg(test)]
576	mod tests;
~~~~

**E173: `crates/holler-hub/src/lockout.rs:587-590`** — `lockout.rs`: the same precedent.

Command: `sed -n '587,590p' crates/holler-hub/src/lockout.rs | awk -v s=587 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
587	}
588	
589	#[cfg(test)]
590	mod tests;
~~~~

**E174: `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:41-75`** — `skilltext_w5_rename`: reads every path of `git ls-files` and fails on the wizard skill's old directory name outside the rename note; the brief and every file this story adds or edits must never spell it.

Command: `sed -n '41,75p' crates/holler-cli/tests/wizard/skilltext_w5_rename.rs | awk -v s=41 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
41	        .args(["ls-files", "-z"])
42	        .current_dir(root())
43	        .output()
44	        .unwrap();
45	    assert!(listing.status.success());
46	    let mut hits = Vec::new();
47	    for rel in String::from_utf8(listing.stdout).unwrap().split('\0') {
48	        if rel.is_empty() || rel.ends_with("skilltext_w5_rename.rs") {
49	            continue;
50	        }
51	        let Ok(text) = std::fs::read_to_string(root().join(rel)) else {
52	            continue; // deleted in the working tree, or not UTF-8
53	        };
54	        for (i, line) in text.lines().enumerate() {
55	            if line.contains(needle) {
56	                hits.push(format!("{rel}:{}: {line}", i + 1));
57	            }
58	        }
59	    }
60	    hits
61	}
62	
63	#[test]
64	fn no_old_command_or_directory_outside_the_rename_note() {
65	    let old = format!("{}-{}", "setup", "wizard");
66	    let note_marker = format!("~/.claude/{}", old_skill_path());
67	    for needle in [format!("agent-skills/{old}"), format!("/{old}")] {
68	        let bad: Vec<String> = scan_hits(&needle)
69	            .into_iter()
70	            .filter(|h| {
71	                // `docs/<old>.md` is the doc and keeps its name.
72	                let stripped = h.replace(&format!("docs/{old}"), "").replace(&format!("../{old}"), "");
73	                stripped.contains(&needle) && !h.contains(&note_marker)
74	            })
75	            .filter(|h| !h.contains(&format!(".config/{old}")))
~~~~

**E175: `crates/holler-pane/src/tx_switch.rs:299-327`** — `tx_switch::plan`: the plan, with the single-process branch (`Shape::SingleProcess`: `check_idle` and no session list) and the server branch (`check_listed`, `check_unclaimed`); the hub reads join here.

Command: `sed -n '299,327p' crates/holler-pane/src/tx_switch.rs | awk -v s=299 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
299	/// The plan: every refusal, before any write or live change. Returns the record the run
300	/// starts from, whose generation the record write names.
301	fn plan(ports: Ports<'_>, request: &SwitchRequest) -> Result<Pane, PaneError> {
302	    let existing = match &request.target {
303	        Target::Existing(id) => Some(parse_session_id(id)?),
304	        Target::Fresh => None,
305	    };
306	    let record = read(ports, request)?;
307	    refuse_orchestrator(&record, request.as_operator)?;
308	    let at = record.harness.endpoint(&record.name)?;
309	    check_health(ports, &record, &at)?;
310	    let single = record.harness.kind.shape() == Shape::SingleProcess;
311	    match &existing {
312	        Some(target) if single => {
313	            check_idle(ports, &record, &at)?;
314	            check_unclaimed(ports, &record.name, target)?;
315	        }
316	        Some(target) => {
317	            check_listed(ports, &at, target)?;
318	            check_unclaimed(ports, &record.name, target)?;
319	        }
320	        None => check_idle(ports, &record, &at)?,
321	    }
322	    Ok(record)
323	}
324	
325	/// The record of the request's pane: through the profile's scope when the request names a
326	/// profile (`profile-not-found`, `pane-not-in-profile`), else from the pane store. No
327	/// record is `pane-not-found`, and so is a scope that answers with another pane. The
~~~~

**E176: `crates/holler-pane/src/reconcile.rs:28-43`** — `reconcile.rs`'s module doc, #831's single-process paragraph: no server list, the rules that need one are skipped, and the relaunch stores the record.

Command: `sed -n '28,43p' crates/holler-pane/src/reconcile.rs | awk -v s=28 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
28	//! **A single process (#831, ADR-0021 section 15 items 6 and 13; plan review W2 and W3).** A
29	//! pane whose harness's shape is `SingleProcess` (Claude Code) has no server list and no TUI,
30	//! so its chain is `host.ps`, `harness.health` at its endpoint (whether its process still
31	//! runs: `server-down` when it does not, with no wedge check), `harness.shown_session` (the
32	//! session the process holds, which a person's `/clear` or `/resume` moves) and
33	//! `harness.session_activity` (a wait is `pane-waiting`, with its reason). The rules that
34	//! need a server's list are skipped: `session-of-record-missing`, `tui-foreign-session`,
35	//! `stray-session` and `server-wedged`. Its one mismatch, the process holding another session
36	//! than the session of record, is repaired by relaunching it on the session of record, and
37	//! only while it is idle, so a repair never drops a running turn: a pane that works or waits
38	//! is `not-fixable`, with the relaunch remedy. The relaunch is the caller's
39	//! ([`reconcile_with`]'s `relaunch`: `holler pane doctor` passes `tx_launch`'s), so this
40	//! engine never imports the launch engine, which reads its [`shown_differs`]. After a
41	//! relaunch the chain writes no record of its own: the relaunch stored it. And once per pass,
42	//! when a pane in scope is a single process, `harness.login_status` is asked: a login that is
43	//! not the subscription's is one `claude-login-missing`.
~~~~

**E177: `crates/holler-pane/src/pane.rs:149-158`** — `Shape` (#831): `ServerAndTui` for OpenCode and `SingleProcess` for Claude Code; `HarnessKind::shape()` chooses the engines' steps.

Command: `sed -n '149,158p' crates/holler-pane/src/pane.rs | awk -v s=149 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
149	/// How a harness of a kind is built ([`HarnessKind::shape`], #831).
150	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
151	pub enum Shape {
152	    /// A server on a port, and a TUI that attaches to and switches between its sessions
153	    /// (OpenCode).
154	    ServerAndTui,
155	    /// One process that holds one session; a switch, a reset or an interrupt is a
156	    /// relaunch of it (Claude Code).
157	    SingleProcess,
158	}
~~~~

**E178: `docs/adr/ADR-0021.md:577-591`** — ADR-0021 section 8, "Switch and reset of a single process as built (#831, section 15 item 6)": the last sentence says `reset --first` sends to the endpoint of the record the relaunch stored; this story replaces it.

Command: `sed -n '577,591p' docs/adr/ADR-0021.md | awk -v s=577 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
577	**Switch and reset of a single process as built (#831, section 15 item 6).** A pane whose harness's shape is
578	`SingleProcess` (Claude Code) has no server to create a session on and no TUI to switch, so `tx_switch` keeps its plan and
579	replaces its act with a relaunch. The plan refuses as above (`orchestrator-pane`, `server-unhealthy` for a process that no
580	longer runs, and for switch `session-of-other-pane`), except that a switch also refuses a pane that is not idle, since a
581	relaunch drops the turn in flight (`session-busy`, `session-holds-question`, and `session-holds-permission` for a
582	permission prompt, which reset refuses too), and asks for no session list. Whether the target has a conversation is the
583	relaunch's plan's: `process_command` with the resume-only `SessionChoice::Resume` answers `session-not-found` for an id
584	with no transcript, before anything is stopped, so a switch to an unknown id starts and records nothing, as a server
585	pane's switch does. The act is then
586	`tx_launch::relaunch_recorded`, the one stop, run, wait, locate and record sequence (section 15 item 6): the pane's
587	processes stopped, its process run on the target (switch) or on a fresh session (reset), located idle, seen holding it, and
588	recorded with its new pid by the launch engine's own compare-and-swap. A relaunch calls Herdr and the host as a launch does,
589	and types nothing (I4). `reset --first` sends its message to the endpoint of the record the relaunch stored, the new process,
590	never the stopped one. A failure once the relaunch entered its act ends with the reconcile step, as a failure after
591	`select_session` does.
~~~~

**E179: `CHANGELOG.md:94-99`** — `CHANGELOG.md`, the #649 item: the clause "so until #828 lands, do not use `reset --first` on a parked or held pane" that this story replaces.

Command: `sed -n '94,99p' CHANGELOG.md | awk -v s=94 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
94	  write whose reply is lost answers `timeout`, since it may have landed. No credential is read, printed or put on a
95	  command line: an OpenCode server inherits its tmux server's environment. Two things go live with it: every profile
96	  write a `--profile` pane verb or `profile apply` makes is logged as `holler pane`
97	  ([#825](https://github.com/Performant-Labs/holler/issues/825)), and the first message of `pane reset --first` reaches
98	  the pane's server outside the hub's park and hold gate, so until
99	  [#828](https://github.com/Performant-Labs/holler/issues/828) lands, do not use `reset --first` on a parked or held pane.
~~~~

**E180: `grep -rn "Ports {" crates --include='*.rs'`** — Every `Ports { … }` literal on `main` (the struct itself and doc lines excluded): 20 literals in 17 files.

Command: `grep -rn "Ports {" crates --include='*.rs' | grep -v 'crates/holler-pane/src/ports.rs' | grep -v 'pub struct\|impl\|///\|//!' | sort`

~~~~text
crates/holler-cli/src/pane/wiring.rs:137:        Ports {
crates/holler-cli/tests/pane_claude/rig.rs:199:        f(Ports {
crates/holler-cli/tests/pane_integration/bind.rs:650:    let ports = Ports {
crates/holler-cli/tests/pane_integration/observe.rs:258:    let ports = Ports {
crates/holler-cli/tests/pane_verbs/close/rig.rs:143:        Ports {
crates/holler-cli/tests/pane_verbs/doctor/rig.rs:176:        Ports {
crates/holler-cli/tests/pane_verbs/import/rig.rs:247:        let ports = Ports {
crates/holler-cli/tests/pane_verbs/import/rig.rs:263:        let ports = Ports {
crates/holler-cli/tests/pane_verbs/launch/rig.rs:324:        f(Ports {
crates/holler-cli/tests/pane_verbs/list.rs:72:        Ports {
crates/holler-cli/tests/pane_verbs/park/rig.rs:98:        Ports {
crates/holler-cli/tests/pane_verbs/park.rs:294:    let ports = Ports {
crates/holler-cli/tests/profile_verbs/apply/rig.rs:226:        f(Ports {
crates/holler-cli/tests/profile_verbs/rename.rs:254:            Ports {
crates/holler-cli/tests/profile_verbs/rig.rs:63:        Ports {
crates/holler-cli/tests/profile_verbs/rig.rs:85:            Ports {
crates/holler-cli/tests/roster_panes_test.rs:147:        Ports {
crates/holler-cli/tests/verb_harness/mod.rs:27:    Ports {
crates/holler-pane/tests/ports_test.rs:504:    let ports = Ports {
crates/holler-pane/tests/ports_test.rs:523:    let ports = Ports {
~~~~

**E181: `9af2ad7:docs/handoffs/803-brief.md:63-63`** — #803's brief at `9af2ad7` (a sibling branch, parked): D13, one local body for every OpenCode pane, one attach row per pane, with `session_id` read from the registry.

Command: `git show 9af2ad7:docs/handoffs/803-brief.md | sed -n '63,63p' | awk -v s=63 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
63	**D13. One local body drives the OpenCode panes; a Claude Code pane has none yet.** *(decided by the preparer, to be confirmed at the plan check)* A body is how the hub drives a session (R4, item 18; R6). Stage 8 derives one body config from the registry: for each OpenCode pane, a `[[session]]` row with `name` = the pane's name, `harness = "opencode"`, `mode = "attach"`, `endpoint = "http://127.0.0.1:<port>"`, `session_id` = the pane's session of record, both read from `holler pane get NAME --format=json` (`data.pane.harness.port` and `data.pane.session_of_record`, E78), never from the config; it drops what the body denies, exactly as the derived config does today (`[instance]`, `backend_port`, `workspace`, and now every `[[pane]]` key; E68). The config is written to `<body_config>` = `<state_dir>/herdr/setup/<prefix>-sessions.toml`, the wizard's own directory (the rule that every wizard file lives there), replacing `~/sessions.toml` and the remote path of E68. It mints one token labelled `<prefix>-local` (the label rule of E13 minus the remote host), joins with `--server ws://127.0.0.1:<hub_port>` (F8), runs one `holler body run`, and records it `--role body --stage 8`. #839's design assumes exactly this ("one attach-mode body session per registry pane whose endpoint is that pane's server", R6), and its brief names the one semantic dependency between the two stories (its clash table's row for this story, line 444 of `docs/handoffs/839-brief.md` at `bf9d1c2`, not excerpted because that line carries the old spelling F12 forbids): a body's attach `session_id` is configuration for #839, while the issue's item 3 takes the per-session `session_id` out of the wizard's config. It stays configuration where the body reads it: the master config has no `session_id`, and the *derived body config* has one, read from the registry. No body is started without one. **REFRESH AFTER MERGE of #831**: a Claude pane gets no row (no body driver until #850, R9, R4); the wizard says so in Stage 3, and Stage 10's `holler say` round trip covers OpenCode panes only. Rejected: write the `session_id` from the config (the config no longer has one, E1 item 3); one body per pane (many processes, one roster label each, for no gain).
~~~~

**E182: `crates/holler-hub/src/control.rs:162-181`** — `control/token_ping` and `control/revoke` name an exact token `token_id`; `control/rebind` does the same.

Command: `sed -n '162,181p' crates/holler-hub/src/control.rs | awk -v s=162 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
162	/// `hub token ping ID` (issue #182): ask the live hub to send a `circuit/
163	/// ping` over the token's live socket (if any) and report `{hostname,
164	/// rtt_ms}`. No live socket for that token surfaces as
165	/// [`ControlError::Refused`] carrying the hub's `-32004 not_connected`.
166	pub fn token_ping(token_id: &str) -> Result<serde_json::Value, ControlError> {
167	    let params = serde_json::json!({ "token_id": token_id });
168	    exchange("b-token-ping", "control/token_ping", Some(params))
169	}
170	
171	/// `hub token revoke ID` (issue #184): ask the live hub to force-close
172	/// `token_id`'s socket, if it currently has one. Best-effort —
173	/// [`ControlError::NoLiveHub`] (no hub running) is expected and harmless
174	/// here: the token store side of the revoke already took effect (the caller
175	/// runs this only *after* `token::delete` succeeds), so there is simply
176	/// nothing live to close. `hub token delete` never calls this (#454): it
177	/// refuses a bound token, and an unused or revoked one has no socket.
178	pub fn revoke_live(token_id: &str) -> Result<serde_json::Value, ControlError> {
179	    let params = serde_json::json!({ "token_id": token_id });
180	    exchange("b-revoke", "control/revoke", Some(params))
181	}
~~~~

**E183: `docs/adr/ADR-0021.md:1537-1552`** — ADR-0021, the end of "Decisions taken": item 31 is the last on `main` (#831 appended 22 to 28, #665 item 29, #835 items 30 and 31); this story appends from item 32, or from the next free number at merge time.

Command: `sed -n '1537,1552p' docs/adr/ADR-0021.md | awk -v s=1537 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1537	28. **The login variables are a fact of the kind, refused by the engine's plan** (2026-10-11, #831, section 15 item 10;
1538	    decided under the operator's standing rules in phase F): `HarnessKind::login_variables()`, so the adapter's `env -u` list
1539	    and the plan's `claude-login-env-refused` cannot drift, and the refusal comes before any port call.
1540	29. **`profile rename`, `export`, `import`, `profile/rename`, `ProfileStore::rename` and the `profile-drift` finding (#665) are
1541	    confirmed and built** (operator, 2026-10-11; decided in the plan check under the operator's standing rules, not operator
1542	    rulings, so the operator may overturn them: the log of a rename (a `deleted` entry on the old name's log, one `renamed`
1543	    entry on the new), the file format, and 'once per episode' read as once per pass).
1544	30. **`pane import` leaves a pane whose tmux session runs Claude Code alone, and `--adopt-claude-code` will adopt it**
1545	    (operator, 2026-10-11, #835, section 13): #654's option (b), changed to a default plus a switch. The switch is refused
1546	    until #831's adoption step lands (section 15, item 17).
1547	31. **`HerdrPort::observe_pane` is required** (operator, 2026-10-11, #835, section 14): the operator folded this change to
1548	    #832's method into #835, which makes `HostPort::foreground_commands` required for the same reason. A wrapper port that
1549	    does not forward a defaulted method answers the default silently over the real wiring, where a required method is a
1550	    compile error.
1551	
1552	Items still open and owned by a named story are listed under "Deferred to named stories" above.
~~~~

**E184: `docs/protocol/v2.md:658-658`** — `docs/protocol/v2.md` section 8, the `-32004` row: today it names only a remote `query`/`ping`; this story adds the rebind cases and the offline reason.

Command: `sed -n '658,658p' docs/protocol/v2.md | awk -v s=658 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
658	| `-32004` | `not_connected` | A remote `query`/`ping` for a bound token with no live socket. |
~~~~

**E185: `crates/holler-pane/src/ports.rs:284-308`** — `HerdrPort::observe_pane` on `main` after #835: required, no default (the compiler finds every implementation, a wrapper port included); `observe_panes` keeps its default and a wrapper port forwards it.

Command: `sed -n '284,308p' crates/holler-pane/src/ports.rs | awk -v s=284 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
284	    /// What Herdr says about the pane `pane` that a record does not hold: its working
285	    /// directory and when its screen last changed (#832). It is an observation (I6), read
286	    /// fresh and never stored; `pane list` and `pane get` show it for a pane no record
287	    /// names.
288	    ///
289	    /// An implementation that cannot tell says so with `None` fields (an empty
290	    /// [`PaneObservation`]) and never guesses. `pane-not-found` when Herdr has no such pane.
291	    ///
292	    /// Required since #835, with no default. The default answered an empty observation, so a
293	    /// wrapper port that did not forward it reported nothing, silently, over the real wiring;
294	    /// now the compiler finds every implementation.
295	    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError>;
296	
297	    /// [`HerdrPort::observe_pane`] for several panes at once: one observation for each of
298	    /// `panes`, in the same order, where a pane that cannot be observed (Herdr does not have
299	    /// it, or the call failed) reports an empty one. A list of many unmanaged panes costs one
300	    /// call to Herdr when the implementation can read them together; the default asks about
301	    /// each pane in turn. A wrapper port overrides it to forward the batch, so that a list
302	    /// stays one call to the port it wraps.
303	    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
304	        panes
305	            .iter()
306	            .map(|pane| self.observe_pane(pane).unwrap_or_default())
307	            .collect()
308	    }
~~~~
