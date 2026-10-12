# Handoff-A: #839 - DRIVEN follows each pane's session of record  (up-front plan review, second pass)

**Date:** 2026-10-11
**Branch:** `issue-839-implementation` at `e7df33e` (`origin/main` at `d6ae539` merged in at `7f1df46`)
**Brief reviewed:** `docs/handoffs/839-brief.md` (3906 lines, as amended at `e7df33e`)   **Reuse map:** the brief's "Reuse" section (this run has no `survey.md`)   **Wireframe:** N/A (no UI surface)
**Previous pass:** BLOCK, 3 `block` and 10 `warn` (this file at `6211c86`). The numbers in the brief's "Changes since the plan check" refer to that version.
**Verdict:** BLOCK

## Summary

BLOCK: 1 `block`, 9 `warn`.

**The amendment answers all 13 findings of the first pass** (the table below). These now hold:
- P1's closed forward;
- P2's one source for the attach id;
- P3's four ADR passages and the CLI's name-only `SESSION`;
- `Activity` reuse, `control_say.rs`, the scan rule, no new `FixState`, the gate keyed on a recorded DRIVEN, the per-connection slot, `resolve_prompt_target`, and the blast-radius additions.

I checked each against the code:
- `LiveCommand` is `pub` and a child module reaches `LiveHandle`'s private `tx`;
- `Inner.config` is an owned `SessionConfig`, read by every attach path (`task.rs:96`, `:172`, `:259`, `:493`);
- the four body readers of `session_id` are the ones F7 names;
- the CLI's new refusal of a bare id is the hub's own `RoutableName::parse` refusal (`talk.rs:146`, `:203`, `interrupt.rs:102`), moved earlier.

**One new `block`, from the amendment itself.** P2 accepts that every body restart reverts its binding. That makes a body that is behind routine: "a reboot or a deploy does this to the whole fleet at once". `driver_of`'s fallback is now therefore the main way a pane finds its body again. Its evidence, that the pane's own server lists the advertised id, fails both ways:
- **It picks the wrong body.** Servers that list each other's sessions (the case `check_unclaimed` documents) make another pane's behind body "the only candidate" for a pane with no connected body. A `pane switch` or `pane reset` then re-points that body silently.
- **It misses the right one.** A body whose session a relaunched server no longer lists is never found. That makes the section 11 text the brief gives F verbatim untrue.

## The first pass's findings

| # | First pass | Now |
|---|---|---|
| 1 | block: the rebind leaks onto the generic forward | **Resolved** (P1).<br>- `LiveHandle::rebind` sits in `live/target.rs` and the allowlist is untouched.<br>- The existing scan `the_generic_request_forward_allows_only_query_and_answer` (`hold_single_path_test.rs:71`) still holds.<br>- Acceptance 13 follows `hold_hub_test:56-70`.<br>- One follow-on point is finding 5. |
| 2 | block: the body silently returns to the old session | **Resolved** (P2: one source, the presence mirror, `live_configs`, and the registry baseline documented as such).<br>- The restart decision is made.<br>- Its remedy depends on `driver_of` (finding 1).<br>- One follow-on point is finding 7. |
| 3 | block: three ADRs not amended | **Resolved** (P3). Each passage exists with the quoted words. One sentence of ADR-0021 section 11 is left (finding 9). |
| 4 | warn: `advertise` replaces the whole set | Resolved: the whole cached list, with a two-session test. |
| 5 | warn: `DriverState` duplicates `Activity` | Resolved: `Advertised.state: Activity` and one `activity_refusal`. |
| 6 | warn: one accept rule, one refusal mapping, one `SayError` mapping | **Resolved for the accept rule** (`wait_ended`) and **for the hub** (`control_say.rs`).<br>**Open for the three CLI verbs** (finding 2). |
| 7 | warn: the `.send_prompt(` scan | Resolved: one allowed call, in `wiring/bind.rs`, which is not edited. |
| 8 | warn: a new `FixState` is JSON-visible | Resolved: no new state.<br>How the remedy is set is finding 6. |
| 9 | warn: a body that is behind is not gated | Resolved: the recorded DRIVEN is a key, and the remaining window is stated. |
| 10 | warn: the I2 sentence over-claims | Narrowed to switch and reset.<br>The relaunch sentence it added is false (finding 1). |
| 11 | warn: one `pending_query` slot | Resolved: `query_slot`.<br>The bound and the plumbing are finding 5. |
| 12 | warn: resolver name, ADR file list | Resolved: `resolve_prompt_target`, and `src/hub.rs` added to section 2's list. |
| 13 | warn: missing files | Resolved: `codec_test.rs`, the fake server's stall mode, the three CLI files.<br>D8's test moves to `holler-cli`. |

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | **D3 `driver_of`:** "Candidates are the advertised ids that the pane's own server lists and that are not another pane's session of record … else the only candidate".<br>**D3's residue:** a relaunch "until `pane switch` …, which `doctor` names".<br>**Section 11, point 3 (verbatim) and the I2 row:** the same relaunch sentence.<br>**P2:** "the remedy is `holler pane switch`" | cross-cutting (fail closed); pattern consistency (`check_unclaimed`); ADR consistency | **Membership in the pane's server list is the plan's only evidence that a body belongs to a pane, and it fails both ways.**<br><br>**Too wide.** `check_unclaimed`'s doc (`tx_switch.rs:341-343`) says servers that share a data directory list every pane's sessions, "so the server's list alone would let a switch show one pane another's conversation". The plan excludes other panes' sessions of record, but not another pane's behind body. Example:<br>- P1 and P2 share a listing. P2's body B restarted and advertises its configured `ses_b0`, which is no pane's record.<br>- P1 has no connected body: the cutover is in progress, or its body is still starting after the same reboot.<br>- For P1, B is "the only candidate", and `doctor` records P1's DRIVEN as `ses_b0` with the remedy `pane switch P1 <record>`. Doctor names B for P2 too.<br>- Following P1's remedy, or `pane reset P1`, re-points B to P1's session through P2's server.<br>- P2 loses its body without a message. A bare `say` to B's name passes the gate (P1 is healthy) and lands in P1's session.<br><br>The brief itself rejects this ("choosing among the candidates by order or by guess (a wrong pairing, silently)") and says "nothing in the roster or the records says which body belongs to which pane".<br><br>**Too narrow.**<br>- A relaunch creates a new session when the restarted server no longer lists the old one (`tx_launch.rs:680-688`). The body is then on a session its pane's server does not list, so it is never a candidate.<br>- `doctor` records DRIVEN `None` over the stored value and reports no mismatch.<br>- `pane switch PANE <record>` finds no driver and succeeds without a rebind, leaving the body where it was.<br>- The section 11 text F must copy verbatim ("leaves the body on the old session until `pane switch`"), the I2 row and D3's residue say the opposite. | Notes for O, §1: give `driver_of` positive evidence and fail closed without it. Add the one-body and relaunch cases to Acceptance 20 and 6, and correct the three texts. |
| 2 | warn | **D5 (a):** `hub_refusal` is "used … by the `--pane` arms of `say_cmd`, `interrupt_cmd` and `answer_cmd` … each calls it when `Routed.pane` is `Some`".<br>**D7:** operator or default hold → `session-held` (3). | CLI contract | **Unsettled: whether the routed verbs now print `session-held`, exit 3, for an operator or default hold.**<br>- Today they print `hold_cmd::held_refusal` and exit 4. `HELD_EXIT_CODE` (`hold_cmd.rs:19`) exists "so a script can tell held from failed".<br>- #646's `every_other_held_error_falls_through_to_the_generic_arm` (`target_flags.rs:608`) pins the fall-through.<br>- A pane hold's `--json` form is the `PaneHeld` object of `pane_hold_refusal` (`say_cmd.rs:460`), which a `PaneError` cannot produce.<br>- Section 9's routed row, the CHANGELOG and Acceptance 18 name neither change. | **Decide it.** The smaller choice:<br>- for the three verbs, `hub_refusal` maps only `unknown session`, `not_connected` and `ambiguous` to `pane-not-driven`;<br>- pane holds keep `pane_hold_refusal`;<br>- operator and default holds keep the held arm (exit 4);<br>- `session-held` is the code of `reset --first` alone.<br><br>Give `hub_refusal` a return type both callers can use, e.g. an enum that `wiring/hub.rs` turns into a `PaneError` and the verbs render.<br><br>If exit 3 is wanted instead, list it in section 9 and the CHANGELOG, and add a test. |
| 3 | warn | **D3:** "the one that advertises the pane's session of record is the driver; else, for a switch, the one that advertises `target`".<br>**Step 4:** "Drive". | ADR consistency (ADR 0014) | **Two bodies can end up on one harness session.**<br>- The pane's own body advertises the record, and a second live attach session already advertises the switch target. That second session can be another pane's behind body, or an operator's own attach row for a session Holler does not own.<br>- The record rule wins, and step 4 re-points the pane's body onto the target. Two bodies then drive one harness session, which ADR 0014's Consequences call undefined.<br>- `resolve_prompt_target` then answers `Ambiguous` for that id, so `--pane` reports `pane-not-driven` after a switch that succeeded. | Refuse in the plan, before anything moves, a switch whose target a live attach session other than the driver already advertises. Use a sibling of `session-of-other-pane`, or `pane-not-driven` naming that session. Add a row to Acceptance 20. |
| 4 | warn | D5: "the `check_only` branch calls a new `talk::admit`" (`talk.rs` +30) | duplication | **`admit` would copy what `talk::say` already runs before `send_turn`:**<br>- the parse and resolve (now `resolve_prompt_target`);<br>- the hold fast path (`talk.rs:215`);<br>- the busy check (`:222-243`).<br><br>Written beside `say`, it is a second copy of that prelude, which Phase 7 rejects. | **Extract `say`'s prelude into `admit`.**<br>- `admit` adds the pane gate and returns the resolved `(LiveHandle, SessionAd)`.<br>- `say` calls `admit`, then `send_turn`; `check_only` calls `admit` alone.<br>- As a result, `say` refuses a pane-gated prompt before it writes a TalkLog line or moves the turn id.<br>- Say whether `check_only` runs the busy check. The plan has already refused a busy driver. |
| 5 | warn | **D2, the slot:** "both `query` (after its allowlist check) and `rebind` hold it …; `rebind` takes it inside its own `timeout`".<br>**D2, the bounds:** "The hub's forward (10 s) and the CLI's call (10 s) are then never the first to expire". | concurrency; I5; choke point | **`query`'s bound.** `query` waits for the slot outside its own `timeout`. `hub query` (10 s at the hub, `control_server.rs:269`; the CLI waits 5 s, `control.rs:16`) and `answer` can therefore wait one rebind past their bound.<br><br>**The plumbing.** `rebind` repeats `query`'s send and wait (`live.rs:290-298`).<br><br>**The tie.** The hub's forward and the CLI's `control/rebind` call are both 10 s. Once the slot wait is inside the forward, the CLI can expire first, report `timeout`, and never see the hub's answer.<br><br>**No pin.** Nothing pins that `LiveCommand::Query` is built only in `query` (after the allowlist) and in `rebind` (with the literal method). | **Slot:** take it inside the caller's own `timeout` in both.<br><br>**One send and wait:** a single private helper in `live.rs`, placed after `query`, serves both. The scan at `hold_single_path_test.rs:71` looks for `LiveCommand::Query` after `pub async fn query(` in `live.rs`, so a helper in `target.rs` would break it.<br><br>**Bounds:** keep the forward below the CLI's call (e.g. 8 s against 10 s).<br><br>**Scan:** extend the forward scan to the two constructors. |
| 6 | warn | D3: "reconcile sets the finding's own `remedy` field (E121) to `holler pane switch PANE <session of record>`" | pattern consistency | **The hand-set remedy collides with the remedy table.**<br>- `FindingKind::remedy` is "the one table of remedies" (`findings.rs:118`).<br>- `Finding::with_fix` (`:299`) recomputes `remedy` from that table. A remedy set before `with_fix` is overwritten with the table's `relaunch` for `not-fixable`.<br>- A remedy set by hand spells the command outside the builders (`doctor_command`, `relaunch_command`, `reset_command`, `:312-333`). | **Use the builders.**<br>- Add `switch_command(pane, session)` beside them.<br>- Add a `Finding` method that sets it after `with_fix`.<br>- Amend `remedy`'s doc to name this one case that reconcile sets. |
| 7 | warn | D2: `mirror_session_id(inner)` "at task start from the configuration …"; `presence_doc` reads the mirror | state | **Until the task first runs, an attach row is advertised without its id.**<br>- `SessionManager::start` builds `SessionPresence::default()` and then spawns the task (`session_manager.rs:210`, `:218`). Until the task runs, `presence_doc` reads the mirror's `None` while `omitted_pending_attach` is still `false`.<br>- Today the row carries the configured id (`:367`).<br>- A `HoldGate` with no id skips the pane gate (D1).<br>- In production the window is short. In a current-thread test that calls `presence_doc` right after `start`, it lasts the whole test, and the restart case of Acceptance 9 can read exactly that. | Set the mirror (and `omitted_pending_attach` for an attach row) where the presence is built, from the configuration (`SessionManager::start` or `Inner::new`), before the task is spawned. The task stays the only writer after that. |
| 8 | warn | D7's last row: "as `ControlError` maps today"; D10: `FakeHub` keeps "a call log (`calls()`)" | duplication | **Two of the plan's objects have precedents to extend, and the Reuse list names neither.**<br>- `wiring/store.rs` already maps a failed exchange to a `PaneError` once: `call`, `timed_out` and `hub_unavailable` (`:342-390`), by read or write.<br>- Every fake in the kit takes its faults and call log from `fault::FaultSwitch` with an op enum (`fault.rs:41`).<br><br>A second mapping in `wiring/hub.rs`, or a hand-rolled log in `FakeHub`, is a near-copy. The overlay names test-harness helpers as Phase 7 candidates. | **Name both in Reuse.**<br><br>**`wiring/hub.rs`:** route only `ControlError::Refused` to `hub_refusal`, and the rest through `store.rs`'s helpers (made `pub(super)`). `rebind` and `prompt(Send)` are writes; `advertised` and `prompt(Check)` are reads.<br><br>**`FakeHub`:**<br>- give it a `HubOp` (`hub.advertised`, `hub.rebind`, `hub.prompt`) and `faults()`;<br>- for Acceptance 2's order, record the pane's record at each call, as `SeamHarness` does (`then_of_record`, `switch.rs:695-699`). |
| 9 | warn | "Amended text of section 11": "Keep the first two paragraphs (from 'ADR 0014 stays in force, unchanged.' …)" | ADR consistency | P3 amends ADR 0014's point 5 and Consequences and adds a bullet. Section 11 would still open "ADR 0014 stays in force, unchanged." (`ADR-0021.md:868`), and Acceptance 17's grep would not catch it. | **Change the sentence:**<br>- either "ADR 0014 stays in force, extended by points 1 and 2 below (#839)";<br>- or keep "unchanged" for the teardown and PTY rules, and say what is extended. |
| 10 | warn | **"Clashes with stories in flight":** "not re-read at this amendment … to be re-read at the plan check's second pass".<br>**The merge order.** | blast radius | **Re-read at this pass, from local refs (not fetched):**<br>- **#667** is no longer "no commits yet". It is at `4bf1524` (F done, T-green and A-dup PASS). It rewrites `pane_integration/scenario.rs`, which moves D9's `reset_with_a_first_message` from line 193 to 144. It also edits `scratch/*`, `docs/pane-control.md` and `CHANGELOG.md`, and adds no `Ports` literal. The stated reason for #839 before #667 ("#667 has no code yet") no longer holds.<br>- **#835** is at `ccc53a0` (S REWORK). It edits `wiring/bind.rs` (+17), the file whose `send_prompt` forward D5's scan allows, and ADR-0003.<br>- **#648** is at `4bc2088` (T-red PASS, tests only).<br>- **#665** (A PASS, brief only) adds a `FindingKind` in `findings.rs` and already orders itself after #839.<br>- **#647 part 2** (being prepared) appends finding kinds and adds port methods that `bind.rs` must forward.<br>- **#831** is unchanged at `b3f0295`. | **Update the table and the order:**<br>- #667 likely lands first, and D9 is applied to its `scenario.rs`;<br>- add #665 and #647 part 2 (`findings.rs` and `reconcile*`, mechanical). |

## Notes for O

There is no O on the automated path, so this BLOCK stops the run (`needs-amendment`). Amend the brief and start a **fresh** run. Do not use `resumeFromRunId`, which would replay this verdict.

**Nothing below reopens the following:**
- P1 to P3;
- D1's id join and the gate keyed in `send_turn` and on the recorded DRIVEN;
- D2's wire request, its body-side refusals, the build-then-swap with the deadline carried into the task, and `control/rebind` with the whole-list `advertise`;
- D3's step order, the `HubPort` port and its file, and the "Ambiguous drivers" two-step remedy;
- D4, D7's code table (apart from finding 2), D8, D9's intent, D10 and D11.

### 1. Give `driver_of` positive evidence, and fail closed without it (finding 1)

**Choose one:**

- **(a) Rules over what the record and the hub already hold.** A candidate is the driver when it advertises:
  - (i) the pane's session of record;
  - (ii) the pane's stored `last_observed.driven`. That is the record's own memory, so a body whose session the server no longer lists is still found. Another pane's session of record stays excluded;
  - (iii) for a switch, the target (today's target rule).

  Keep the fallback "listed by the pane's server, and the only candidate" only when that server lists no other pane's session of record. That is the shared-listing signal `check_unclaimed` already relies on. Otherwise:
  - the verbs refuse `pane-not-driven`, naming the listed candidates and the two-step remedy;
  - reconcile records DRIVEN `None` and reports them with `remedy: None`, as for `Ambiguous`.

  A body that reverted on a pane whose server lists only its own sessions is still found and re-pointed in one `pane switch`.

- **(b) Join on the endpoint.** Each attach session's endpoint is already in the body's `query/status` (`session_list[].endpoint`, `query.rs:54-61`, #195), so no wire change is needed. A candidate must attach through the pane's own server (its port). This costs one `query/status` per body per plan or pass, through `control/query_remote`.

**Either way:**
- **Acceptance 20** gains the one-body case: a pane with no connected body, and another pane's behind body on a shared listing. Neither `driver_of` nor `doctor` makes that body the pane's driver, and `pane switch` or `pane reset` of the pane does not rebind it.
- **Acceptance 6** gains a relaunch case: stored DRIVEN `ses_old`, not listed, and a body advertising it.
  - Under (a), `doctor` reports `shown-driven-mismatch` with the switch remedy, and `pane switch` re-points the body.
  - Under (b), or if neither rule finds it: the section 11 text, the I2 row, D3's residue and the guide say the body must be restarted with the new session in its configuration, and `doctor` says so.
- **Rewrite the relaunch sentence** in the verbatim section 11 point 3, the I2 row, D3's residue and P2's "the remedy works" to match what the chosen rule does.

### 2. Fold in the warns while amending

- **Finding 2:** decide the routed verbs' hold exit and the shape of `hub_refusal`.
- **Finding 3:** refuse a switch onto a session another live body drives.
- **Finding 4:** `admit` is `say`'s extracted prelude.
- **Finding 5:**
  - the slot inside each caller's timeout;
  - one send and wait;
  - the forward bounded below the CLI's call;
  - the constructor scan.
- **Finding 6:** `switch_command`, set after `with_fix`.
- **Finding 7:** the mirror set at construction.
- **Finding 8:** add `store.rs`'s failure mapping and `FaultSwitch` to Reuse.
- **Finding 9:** section 11's first sentence.
- **Finding 10:** the clashes table and the merge order.

## Patterns referenced

- **`holler-pane`:**
  - `crates/holler-pane/src/tx_switch.rs` (`check_unclaimed`'s shared-listing rule, `check_idle`);
  - `crates/holler-pane/src/tx_launch.rs` (a relaunch's session of record);
  - `crates/holler-pane/src/findings.rs` (`remedy`, `with_fix`, the command builders);
  - `crates/holler-pane/src/reconcile/observe.rs` (`check_screen`).
- **Hub:**
  - `crates/holler-hub/src/live.rs` (`LiveCommand`, `query`, `insert`, `resolve_session`);
  - `crates/holler-hub/src/talk.rs` (`say`'s prelude, `send_turn`);
  - `crates/holler-hub/src/control_server.rs` and `control.rs` (`CLIENT_TIMEOUT`);
  - `crates/holler-cli/tests/hold_single_path_test.rs`.
- **Body:** `crates/holler-body/src/session_manager.rs`, `session_manager/task.rs`, `http_attach_driver.rs`, `query.rs` (`StatusSession.endpoint`).
- **CLI and test kit:**
  - `crates/holler-cli/src/say_cmd.rs` and `hold_cmd.rs`;
  - `crates/holler-cli/src/pane/wiring/store.rs`;
  - `crates/holler-cli/tests/pane_verbs/target_flags.rs` and `pane_verbs/switch.rs` (`SeamHarness`);
  - `crates/holler-pane-testkit/src/fault.rs`.
- **ADRs:** `docs/adr/ADR-0014.md` (point 5, Consequences), `ADR-0005.md` section 3, `ADR-0003.md` ("Addressing"), `ADR-0021.md` sections 2, 8 and 11.
