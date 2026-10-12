# Handoff-A: #839 - DRIVEN follows each pane's session of record  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-839-implementation` at `1e1f203` (base `origin/main` at `bda1069`; the branch's two commits add the brief)
**Brief reviewed:** `docs/handoffs/839-brief.md`   **Reuse map:** none. This run has no `survey.md`; the brief's decisions, "Where the code goes" and the precedents it names stand in for one.   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 3 `block`, 10 `warn`.

**Most of the plan extends what already exists:**
- **The predicate:** one pane-state predicate moves into `holler-pane`, which the hub and the CLI both already depend on.
- **The gate key** is set in `talk::send_turn`. That is the only place `SayOpts` is built, so the plain, `--queue` and interrupt-redirect variants are all covered (`talk.rs:255-265`, `:337`).
- **New files follow their precedents:**
  - `control_rebind.rs` sits beside `control_hold.rs`, and its arm is local only, in `dispatch_session_control`;
  - `rebind.rs` sits beside `hold.rs`;
  - `HubPort` is a new `Ports` member, the option the #828 review named;
  - `wiring/hub.rs` sits beside `wiring/store.rs`;
  - `FakeHub` comes with its own conformance file.
- **Reuse:** `Registry::replace_session` and `Registry::find_by_token` are reused.
- **Hold keys (D4)** match the presence handler: `note_joined` keys `<label>/<name>`, which a rebind does not change.
- **Size:** `circuit.rs` needs no growth. The hub's outbound forward is not logged by method name, and the body's answer is logged as `response_or_error`.

**Three things must change in the brief before T writes tests:**

1. **The rebind leaks onto the generic request forward.** Adding `session/rebind` to `LiveHandle::query`'s allowlist exposes it through `control/query_remote` and, from an admin-role remote connection, `admin/query`. That second route skips `control/rebind`'s cache and roster update. It also contradicts the brief's own choice to keep the rebind off the admin surface. Acceptance 13 would pin it in a test.
2. **The body silently returns to the old session.** The rebind turns the body's attach id into runtime state, but the plan names only one reader of it, `presence_doc`. The session task re-attaches from its immutable config in two cases:
   - after any turn that ends in error (OpenCode's `session.error`);
   - in its 30 s retry loop.

   Either way the body goes back to the pre-rebind session, which the pane's TUI no longer shows. Presence may still advertise the new id, and then neither the gate nor `doctor` can see it. A body restart reverts the same way, and the brief does not decide what happens then.
3. **D1 changes the addressing rules of three ADRs without amending any of them.**
   - ADR 0005 section 3 says every TARGET resolves a name.
   - ADR 0003 says SESSION expands under ADR 0005.
   - ADR 0014's Consequences say the harness id is surfaced "without it changing how they address the session".

   D1 also makes a bare `holler say ses_…` work, and nothing decides or tests that.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | D2, "Hub": "its allowlist (`query/*` and `session/answer`) adds `session/rebind`"; Acceptance 13 | choke point; cross-cutting (who may re-point a body) | **`LiveHandle::query` is the generic forward.**<br>- `control/query_remote` sends any `params.method` to `handle.query` (`control_server.rs:257-285`, `:269`).<br>- `admin/query` with a target maps to it (`circuit/admin.rs:257-262`).<br>- The allowlist (`live.rs:283`) is that route's only filter. The repo already guards it: `hold_hub_test.rs:56-70` forwards a raw `session/prompt` through `query_remote` and expects `-32601`.<br><br>**With `session/rebind` added, two clients can re-point any body's attach session without going through `control/rebind`:** a local control client, and an admin-role remote connection.<br>- The hub then makes no cache and roster update before replying, and checks no token or session.<br>- This contradicts the brief's decision to keep `control/rebind` out of `dispatch_allowlisted` (there is no `admin/rebind`).<br><br>**The gate cannot catch a mistaken rebind.** A rebind to a session the TUI does not show is invisible to the gate, because the gate keys on the session of record. | Keep the allowlist and its comment exactly as they are.<br><br>**Give the rebind its own forward:** `LiveHandle::rebind(params, timeout)` in the new `live/target.rs`.<br>- An `impl LiveHandle` block in that child module can reach the private `tx` and `LiveCommand`.<br>- It sends `LiveCommand::Query { method: "session/rebind", .. }`. That is the same plumbing, so `circuit.rs` still needs no growth.<br><br>**Acceptance 13 then reads:** the allowlist is exactly `query/*` and `session/answer`, and `query_remote` or `admin/query` with `session/rebind` is refused `-32601` with the body unchanged. |
| 2 | block | D2: "build the new driver … on a copy of the session's config with the new id …; `presence_doc` must read the id from the swapped state"; Acceptance 9; no decision on a body restart | state and persistence | **The plan makes the attach id mutable but names only one reader of it. Other code reads the immutable config.**<br>- OpenCode's `session.error` event (`http_attach_driver/connection.rs:481`) ends the turn with `StopReason::Error`.<br>- `finish_turn_no_dispatch` then drops the driver (`session_manager/task.rs:662-663`).<br>- The next `start_turn` re-attaches with `Driver::spawn_or_attach(&inner.config)` (`task.rs:493`).<br>- The 30 s retry does the same while the driver is `None` (`:172`, `:184`, `:259`).<br><br>**After a rebind, the first errored turn puts the body back on the pre-rebind session**, the one the pane's TUI no longer shows. If presence keeps advertising the new id, which the brief's "swapped state" invites, the gate and `doctor` see a healthy pane while prompts land in an invisible session. That is the incident class this epic exists to end.<br><br>**Two more readers** use the config: `query.rs:154-157` (the `query/support` live probe) and `registry.rs:90`.<br><br>**A restarted body reads its configured `session_id` again.** D2 rejects rewriting the config file, so every rebound session reverts, and the brief does not say so. When servers share a data directory, a restart of two or more bodies leaves each pane with two stale candidates. `driver_of` then answers `Ambiguous`, which refuses `pane switch`, the stated remedy. | Amend D2 and Acceptance 9 as in Notes for O, §2:<br>- one source for the id, which every attach path and presence read;<br>- a test that the next prompt still reaches the new id after an errored turn;<br>- a written decision on a body restart, with a remedy that still works when `driver_of` is ambiguous. |
| 3 | block | D1: "The hub resolves a prompt target that is not a valid name as a harness session id"; the ADR edit list (ADR-0014 gets a rebind bullet only) | ADR consistency | **D1 makes a locator an address. Three ADRs say otherwise:**<br>- ADR 0014 point 5 calls `ses_…` "a body-side locator". Its Consequences say the id is surfaced "without it changing how they address the session" (`ADR-0014.md:17`, `:29`).<br>- ADR 0005 section 3 says every `say`/`interrupt`/`query` TARGET "resolves a **name** → the current locator" (`ADR-0005.md:28-30`).<br>- ADR 0003 says `SESSION`/`TARGET` "expand under ADR 0005" (`ADR-0003.md:117`).<br><br>The brief amends none of them, and ADR-0021's "Amends:" edit names only ADR 0014, and only for the rebind.<br><br>**A CLI change comes with it.** `route_target` sends a SESSION as it is (`say_cmd.rs:231-232`). After D1, `holler say ses_… TEXT`, `interrupt ses_…` and `answer ses_…` therefore work without `--pane`. No decision and no test covers that. | Notes for O, §3: amend the three ADRs and ADR-0021's "Amends:" line, and decide the bare-id CLI form. |
| 4 | warn | D2: "re-advertises the roster row (`Roster::advertise`)" | reuse; pattern | **`advertise` replaces the token's whole set of rows.** A row missing from the frame goes `gone` at once (`roster.rs:349-383`). A one-row `Presence` built for the rebound session would drop the body's other sessions. | **Either:**<br>- pass the handle's whole cached session list after `replace_session` (`live.rs:719`), with a two-session body in the test;<br>- or have the body's rebind call `notify_presence`, so that the existing presence handler updates the roster, and keep only `replace_session` for read-after-reply. |
| 5 | warn | D3: `pub enum DriverState { Idle, Working, InputRequired }` on `Advertised` | duplication | **This enum duplicates one that exists.** `holler_pane::ports::Activity { Idle, Busy, HoldingQuestion }` (`ports.rs:379-388`) already names these three states. `check_idle` maps it to `session-busy` and `session-holds-question` (`tx_switch.rs:311-326`). | **Use `Advertised.state: Activity`.**<br>- The wiring maps the roster's `working` and `stalled` to `Busy`, and `input-required` to `HoldingQuestion`.<br>- One function maps an `Activity` to its refusal, and both the activity check and the new driver check use it. |
| 6 | warn | D5 (`Send` "with the CLI's accept-and-return rule"), D6 (`pane-not-driven` for the `--pane` verbs), `check_only` in `control/say` | duplication; blast radius | **(a) The accept rule already exists.** `QUEUE_ACCEPT_WAIT` and `wait_ended` (`say_cmd.rs:381`, `:405`; `wait_ended` is private) are that rule.<br><br>**(b) The refusal mapping has four users.** `wiring/hub.rs` and the `--pane` arms of `say_cmd`, `interrupt_cmd.rs:65-68` and `answer_cmd.rs:62-65` all map a hub refusal to D7's codes. `Routed { session, arg }` (`prompt_target.rs:43-46`) does not record that a target came from `--pane`. `interrupt_cmd.rs`, `answer_cmd.rs` and `prompt_target.rs` are not in the blast radius.<br><br>**(c) `control/say` already maps `SayError`.** Its arms are inline (`control_server.rs:315-352`), and the `check_only` branch needs the same mapping. | **One function for each, reused:**<br>- `wiring/hub.rs` calls the existing accept rule;<br>- one `WireError`-to-pane-code mapping serves all four users;<br>- the `SayError` arms are extracted to one function shared with `check_only`.<br><br>Add the three CLI files to the blast radius. |
| 7 | warn | Acceptance 13 / D5: the scan "fails if `holler-pane/src` or `holler-cli/src` calls `.send_prompt(`" | test design against existing code | **The scan would fail on code that must stay.** `wiring/bind.rs:326-327` forwards `HarnessPort::send_prompt`. That is #649's pass-through: a wrapper forwards every method.<br>- The scan still fails after F removes the engine's call.<br>- F cannot remove the forward without breaking that rule. | Scan for calls outside a forwarding `fn send_prompt` in an `impl HarnessPort`. Or name `wiring/bind.rs`'s forward as the one allowed site. |
| 8 | warn | D3: "if it needs a new internal `FixState`, that is allowed, it is not on the wire"; Blast radius: "no `--json` shape change" | CLI contract | **A new `FixState` changes `pane doctor --json`.** A `FixState`'s kebab-case name "is also its JSON form" (`findings.rs:178`). | **Add no new state.**<br>- Keep `fix: not-fixable`.<br>- For a body that is behind, set the finding's own `remedy` field (`findings.rs:241`) to `holler pane switch PANE <session of record>`. |
| 9 | warn | D1: "`pane_gate` matches `pane.session_of_record.as_deref() == Some(id)`" | choke-point coverage | **A body that is behind is not gated.**<br>- Such a body still drives an earlier session of pane P: D3's residue after a failed record write, or the reverts of finding 2. It advertises an id that no record names as its session of record.<br>- A bare `say <name>` to it is therefore admitted, and lands in a session P's TUI does not show.<br>- `doctor` records that DRIVEN on P, but the gate never reads it. | **Either:**<br>- also match `pane.last_observed.driven == Some(id)`, so the recorded pair refuses as `pane-shown-driven-mismatch`;<br>- or state this residue in ADR section 11, point 3. |
| 10 | warn | "Section 4, invariant I2's test idea … Append: the driven session is the one the fake hub's body advertises" | ADR consistency | **The appended sentence promises more than the story builds.** I2's test idea covers launch, relaunch, switch and reset.<br>- Launch and relaunch keep the stored DRIVEN (`tx_launch.rs:361-370`).<br>- A relaunch can create a new session (`tx_launch.rs:680-688`), with no rebind. | **Either:**<br>- limit the appended sentence to switch and reset, and say what launch and relaunch do with DRIVEN (a relaunch onto a new session leaves the body behind until `pane switch`);<br>- or bring relaunch into scope. |
| 11 | warn | D2: the forward over `LiveHandle::query` | concurrency | **Queries share one slot per connection.** The circuit keeps a single `pending_query` (`circuit.rs:731` overwrites it), shared by `session/answer`, `query/*` and now the rebind.<br>- A concurrent query drops the other's reply, which is then reported as not connected.<br>- A rebind whose reply was dropped may still have re-pointed the body. | Name this in D2's residue: `doctor` reports it and `pane switch` re-points the body. Say it in the doc of `LiveHandle::rebind` too (finding 1). |
| 12 | warn | `Registry::resolve_target`; the ADR-0021 section 2 edit | naming; ADR consistency | **Naming:** `resolve_target` would sit beside `Registry::find_target` and `TargetLookup` (`live.rs:631`). That is `hub query`'s lookup by token, client or label, so there would be two "target" resolvers with different grammars.<br><br>**ADR file list:** ADR-0021 section 2 says "All ports are traits in `holler-pane` (`src/ports.rs`, `src/profile.rs`)" (`ADR-0021.md:94`). | Give the resolver a prompt-specific name, e.g. `resolve_prompt_target`.<br><br>Add `src/hub.rs` to that sentence in the edit list. |
| 13 | warn | Blast radius | completeness | **Files the plan needs but does not list:**<br>- `crates/holler-proto/tests/codec_test.rs` (889 lines). `every_method_round_trips` walks `CATALOG` and panics with `no canonical frame` on a new row (`:85`).<br>- `crates/holler-body/tests/http_attach_driver_test/fake_server.rs`. It has no stall mode, and the 5 s test needs only the existence check to hang, on the same endpoint.<br>- `holler-pane-testkit` depends only on `holler-pane`. D8's `RoutableName::parse` test needs `holler-proto`.<br>- Finding 6's three CLI files. | **Add each file, or move the test:**<br>- add one `session/rebind` frame to `codec_test.rs`;<br>- add a stall mode to the fake server, or build the hang another way;<br>- put D8's test in a crate that has `holler-proto`, or add the testkit's manifest to the blast radius. |

## Notes for O

There is no O on the automated path, so this BLOCK stops the run (`needs-amendment`). Amend the brief and start a **fresh** run. Do not use `resumeFromRunId`, which would replay this verdict.

**The rest of the plan stands**, and nothing below reopens it:
- D1's id join, and the gate keyed in `send_turn`;
- D3's step order (plan, act, observe, rebind, record, first message) and `driver_of`;
- D4's hold keys;
- D5's check-then-send through `send_prompt`;
- D7's code table, D8's fixtures and D11's size rules.

### 1. Keep the generic forward closed (finding 1)

- **D2, "Hub" bullet:** leave `LiveHandle::query`'s allowlist and its comment unchanged (`live.rs:283`).
  - Add `LiveHandle::rebind(params, timeout)` in the new `crates/holler-hub/src/live/target.rs`, beside `resolve_target`.
  - It sends `LiveCommand::Query { method: "session/rebind", .. }`.
  - `control_rebind.rs` calls it, after `Registry::find_by_token` (`live.rs:603`) and a check that the session is in that handle's cached presence.
  - `live.rs` stays within +6 lines, and `circuit.rs` within +0.
- **Acceptance 13:** replace the allowlist sentence with: "`LiveHandle::query`'s allowlist is exactly `query/*` and `session/answer`. `control/query_remote` (and so `admin/query`) with `method: session/rebind` is refused `-32601`, and the body's binding is unchanged." `hold_hub_test::a_prompt_cannot_be_forwarded_around_the_hold` is the pattern.
- **A remote rebind:** if admin-role connections should ever be able to rebind, decide it as an `admin/rebind` through `dispatch_allowlisted`, not as a side effect of the allowlist.

### 2. One source for the body's binding, and a decision on restarts (finding 2)

**a. D2, "How the body swaps".** Replace "on a copy of the session's config" with this rule:

- **The rebind writes the new id to the session task's own configuration** (`Inner.config.session_id` in `session_manager/task.rs`) once the new driver has attached. That makes it the one source every later attach reads:
  - `start_turn`'s re-attach after a turn that ended in error (`task.rs:493`; the driver is dropped at `:662-663` when OpenCode sends `session.error`);
  - the attach retry (`:172`, `:184`, `:259`).
- **Presence reads the same value.** The task mirrors the id into the shared `SessionPresence` on every attach, and `presence_doc` reads it from there, never from `managed.config.session_id` (`session_manager.rs:367`).
- **The other readers:** say whether `query.rs:154-157` (the `query/support` live probe) and `registry.rs:90` follow the new id or stay with the configured one.

**b. Acceptance 9 gains a case.**
- After a rebind, the fake pushes a `session.error` (its `push_event`).
- The next prompt goes to the **new** id's `prompt_async`, and presence still advertises the new id.

**c. Decide what a body restart does, and write it into ADR-0021 section 11 point 2, the guide and the cutover steps.** The rebind is not persisted, so a restarted body attaches its configured `session_id` again. Choose one:

- **(i) Accept the revert.** This is the smaller change.
  - `doctor` reports the pane as `shown-driven-mismatch`, `not-fixable`, with the remedy `holler pane switch PANE <session of record>` (finding 8). It does so whenever the pane's server still lists the reverted session, and no port deletes sessions.
  - Add an acceptance test for that report.
  - **Give the remedy a rule for `Ambiguous`.** When servers share a data directory, a restart of two or more bodies gives each pane two stale candidates, and switch is then refused `pane-not-driven`. For example, a switch to the pane's own session of record could pick the candidate the pane's recorded `last_observed.driven` names. Alternatively, have question 2's fleet confirmation rule out shared data directories, and say so.
- **(ii) Persist the binding in the body's state directory**, under the repository's state rules:
  - a stated path;
  - an atomic write;
  - mode `0600`;
  - a corrupt or unwritable file fails closed to the configured id, with a warning.

  This removes both the revert and the ambiguity trap, but it is a larger change.

**d. Finding 9 makes (i) safe.** Keying the gate also on a recorded DRIVEN refuses prompts to a reverted body once `doctor` has seen it. Take it, or state the window in ADR section 11 point 3.

### 3. Amend the ADRs D1 changes, and decide the bare-id CLI form (finding 3)

**ADR edits:**
- **ADR-0021's "Amends:" line** also names ADR 0005 section 3, ADR 0014 point 5 and its Consequences bullet on `roster`/`status`, and ADR 0003's "Addressing" line, all as extended by section 11 point 1.
- **ADR 0005 section 3** gains one sentence: since ADR 0021 section 11 (#839), a `say`, `interrupt` or `answer` TARGET that is not a valid name is resolved at call time as the `harness_session_id` that a live attach-mode session advertises; the name stays the identity.
- **ADR 0014** gets the same exception in point 5 and in the Consequences bullet "without it changing how they address the session". The bullet the brief already adds covers only the rebind.

**Decide whether `holler say ses_… TEXT`, `interrupt ses_…` and `answer ses_…` are supported.** They will work, because `route_target` sends a SESSION as it is (`say_cmd.rs:231-232`).
- **If supported:** ADR 0003's Addressing line (`:117`) and the guide say so, and one acceptance test pins the bare form.
- **If not:** the CLI refuses a SESSION that is not a valid name, and only `--pane` sends an id.

### 4. Fold in the warns while amending

- **Finding 4:** the full-presence rule for `advertise`, or a `notify_presence` on the body's side.
- **Finding 5:** `Activity` in place of `DriverState`, with one refusal mapping.
- **Finding 6:** one accept rule, one refusal mapping and one `SayError` mapping; add `interrupt_cmd.rs`, `answer_cmd.rs` and `prompt_target.rs` to the blast radius.
- **Finding 7:** define the `.send_prompt(` scan precisely.
- **Finding 8:** no new `FixState`.
- **Finding 10:** narrow the I2 sentence.
- **Finding 11:** the shared query slot as a named residue.
- **Finding 12:** rename `resolve_target`, and add `src/hub.rs` to ADR section 2's file list.
- **Finding 13:** blast-radius additions (`codec_test.rs`, the body's fake server, the testkit's crate for D8's test).

A short "Reuse" section naming the objects to extend would serve as this run's survey for T and F. Those objects are: `send_turn`, `LiveCommand::Query`, `replace_session`, `find_by_token`, `control_hold.rs`, `hold.rs`, `wiring/store.rs`, `say_cmd`'s accept rule, `Activity` and `check_idle`, `Finding.remedy`, and the body's fake OpenCode server.

## Patterns referenced

- `crates/holler-hub/src/live.rs` (`query`'s allowlist, `replace_session`, `find_by_token`, `find_target`), `crates/holler-hub/src/control_server.rs` (`hub_query_remote`, `dispatch_session_control`, `control/say`'s error arms) and `crates/holler-hub/src/circuit/admin.rs` (`admin/query`)
- `crates/holler-hub/src/circuit/dispatch.rs` (`send_prompt`, `pane_gate`, `handle_presence_notification`) and `crates/holler-hub/src/roster.rs` (`advertise`)
- `crates/holler-body/src/session_manager/task.rs` (crash isolation, re-attach from `inner.config`) and `crates/holler-body/src/http_attach_driver/connection.rs` (`session.error`)
- `docs/adr/ADR-0005.md` section 3, `docs/adr/ADR-0014.md` points 5-6 and Consequences, `docs/adr/ADR-0003.md` "Addressing"
- `crates/holler-pane/src/{ports.rs,tx_switch.rs,findings.rs,tx_launch.rs}`, `crates/holler-cli/src/{say_cmd.rs,pane/wiring/bind.rs}`, `docs/handoffs/828/handoff-A.md` (branch `issue-828-implementation`)
