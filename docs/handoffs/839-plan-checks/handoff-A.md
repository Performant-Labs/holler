# Handoff-A: #839 - DRIVEN follows each pane's session of record  (up-front plan review, third pass)

**Date:** 2026-10-11
**Branch:** `issue-839-implementation` at `bf9d1c2` (`origin/main` at `d6ae539` merged in; local `origin/main` is now `0f5835e`, #667's merge)
**Brief reviewed:** `docs/handoffs/839-brief.md` (4502 lines, as amended at `bf9d1c2`)   **Reuse map:** the brief's "Reuse" section (this run has no `survey.md`)   **Wireframe:** N/A (no UI surface)
**Previous pass:** BLOCK, 1 `block` and 9 `warn` (this file at `8b021d5`; the first pass's is at `6211c86`). The numbers in the brief's "Changes since the second plan check" refer to that version.
**Verdict:** BLOCK

## Summary

BLOCK: 2 `block`, 6 `warn`.

**The amendment answers all ten findings of the second pass** (the table below), and I checked each fix against the code. P4's evidence rule closes both halves of the last block:
- the relaunch case is found by rule (ii), from the stored DRIVEN;
- the one-body case is `Unproven`, never `One`.

**Two new blocks:**
1. **The brief states P4's exclusion two ways.** D3, P4 and the verbatim section 11 remove only a body that advertises another pane's *session of record*. Acceptance 20(b)'s last case, D2's restart text and question 5 all assume that a body tied to another pane by that pane's *recorded DRIVEN* is removed too.
   - Under the rule as written, that body is `Unproven` for the other pane, so the switch is refused instead of proceeding.
   - The refusal's remedy then leads the operator to pair it with the wrong pane.
   - T's test and F's code cannot both follow the brief.
2. **`HubRefusal::NotDriven` cannot be told apart by its code.** The hub answers `-32004 not_connected`, with no `data`, both when the pane's body is offline (resolve time) and after delivery: `say`'s turn timeout and cancel, and `interrupt`'s ack timeout and failed redirect.
   - As planned, a `say --pane` whose turn outlives `--timeout` prints `pane-not-driven` with exit 3 ("no body drives the pane") while the body is still running the turn. Today it exits 1 with the hub's text.
   - No acceptance item tests this.

**The warns must go into this amendment.** On this path they reach no later phase: the script stops only on BLOCK, and neither `feature.md` nor `tester.md` reads `handoff-A.md`. Two of them fail the verify command whatever the code does:
- the brief's own line 444 trips the wizard's rename scan;
- #831's router adds a second `.send_prompt(` forward, which D5's scan forbids.

## The second pass's findings

| # | Second pass | Now |
|---|---|---|
| 1 | block: `driver_of`'s only evidence is membership in the server's list | **Resolved** (P4).<br>- The relaunch case is found by rule (ii), listed or not.<br>- The one-body case is `Unproven`.<br>- Rules (iv) and (v) reuse `check_unclaimed`'s shared-listing signal (`tx_switch.rs:341-360`).<br>- The remedy `pane switch PANE <record>` runs the whole path: `tx_switch` has no early return for the current record.<br>- One follow-on point: finding 1. |
| 2 | warn: the routed verbs' hold exit and the shape of `hub_refusal` | **Resolved** (P5).<br>- The new `--pane` arm goes ahead of today's arms in `say_cmd::control_refusal` (`:179-211`) and in `interrupt_cmd::run` (`:65-79`).<br>- `answer_cmd` has no held arm, as before.<br>- One follow-on point: finding 2. |
| 3 | warn: two bodies on one harness session | **Resolved** (P6, `elsewhere`). Where its code is declared is finding 8. |
| 4 | warn: `admit` would copy `say`'s prelude | **Resolved.**<br>- The prelude (`talk.rs:202-242`) moves as it is.<br>- `admit` needs `roster` for `not_connected_detail` (`:424`), and the planned signature has it.<br>- `holds().would_refuse` consumes no grant (`holds.rs:359-365`), so `check_only` keeps its promise. |
| 5 | warn: the slot, two send-and-waits, the 10 s tie, no pin | **Resolved.**<br>- The circuit matches each reply to the pending query by correlation id (`circuit/dispatch.rs:360`, `:388`). A reply that arrives after a forward timed out is therefore dropped, never misdelivered.<br>- The bounds nest: 5, 6, 8 and 10 s.<br>- The new scan's placement rule is stated.<br>- Where the moved tests' `mod` line goes is finding 4. |
| 6 | warn: the remedy set by hand | **Resolved.** `with_fix` recomputes `remedy` from the table (`findings.rs:299`), so setting it afterwards is right. Quoting is finding 8. |
| 7 | warn: the presence mirror before the task runs | **Resolved.**<br>- `SessionManager::start` builds the presence at `session_manager.rs:210`, before the spawn at `:218`.<br>- `omitted_pending_attach` is set only after a failed first attach (`task.rs:47-55`), so leaving it alone keeps today's behaviour. |
| 8 | warn: reuse of `store.rs`'s mapping and of `FaultSwitch` | **Resolved.**<br>- `call` decodes a `PaneReply` envelope (`store.rs:342`), so a separate send over the same three helpers (`:368`, `:382`, `:393`) is the right split.<br>- `FaultSwitch<Op: PortOp>` fits `HubOp`.<br>- `FakeHub`'s gate is finding 5. |
| 9 | warn: section 11's "stays in force, unchanged." | **Resolved.** ADR-0021:868 is the phrase's only occurrence, so Acceptance 17's grep holds. |
| 10 | warn: the clashes table was not re-read | **Re-read at the amendment, and stale again** (finding 7). |

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | **P4:** "A body that advertises another pane's session of record is that pane's".<br>**D3:** "The bodies that advertise another pane's session of record are first removed"; `driver_of(…, other_records, …)`, "the other panes' sessions of record".<br>**Section 11, point 3 (verbatim):** the same rule.<br>**Against them:**<br>- Acceptance 20(b)'s last sentence;<br>- D2 (line 89): "nobody's record and nobody's stored DRIVEN, so it is `Unproven`";<br>- question 5: "a listed attach body that nothing ties to it". | cross-cutting (fail closed); contract consistency | **The rule as written removes only bodies that advertise another pane's session of record.**<br><br>**Walk 20(b)'s last case through it:**<br>- P1 has no body. P2's body B advertises P2's recorded DRIVEN `d2`, which is not P2's record, and the shared server lists `d2`.<br>- For P1, B is not removed (`d2` is no pane's record). Rules (i) to (iii) miss. The listing is shared, so rule (v) gives `Unproven([B])`.<br>- The acceptance says the result is `None` and that P1's switch proceeds without a drive step. Under the rule, the switch is refused `pane-not-driven`.<br><br>**The refusal's remedy then pairs the wrong body.** The `Unproven` message tells the operator to run `holler pane switch P1 d2`, then to switch P1 to its record.<br>- `check_unclaimed` refuses only another pane's session of record (`tx_switch.rs:344-359`), so `d2` passes.<br>- Rule (iii) then makes B P1's driver, and the second switch re-points it.<br>- B is P2's by P4's own rule (ii): the system holds that evidence and still offers B to P1.<br><br>**The effect on the run.** T writes 20(b) as stated, and F implements the verbatim rule. The test cannot go GREEN unless F changes contract text it must copy verbatim. | Notes for O, §1.<br><br>**Recommended:** a body that another pane's recorded DRIVEN proves is removed, with this pane's own evidence checked first.<br><br>**Alternative:** keep the rule and correct the three texts. |
| 2 | block | **D5 (a):** `HubRefusal::NotDriven(String)` for "`unknown session`, `not_connected` or `ambiguous` for the pane's id", from `classify(&WireError)`.<br>**P5:** the verbs gain `pane-not-driven` "for `unknown session`, `not_connected` and `ambiguous` of the pane's id".<br>**D7:** the same row. | CLI contract (exit codes, section 9); error-mapping consistency | **`-32004` does not mean "the pane's id did not resolve".** The hub encodes all of these as `-32004` with no `data`, through `encode_error` (`control_server.rs:710-713`):<br>- `say`'s offline body, `NotConnected(detail)` (`:321`), the only resolve-time case;<br>- `say`'s `Timeout`, "no reply from {session} within {secs}s" (`:347-350`), sent after the prompt was delivered;<br>- `say`'s `Cancelled` (`:351-353`);<br>- `interrupt`'s `AckTimeout` and `PromptFailed` (`:395`, `:408`).<br><br>The CLI waits `timeout + 5 s` (`control.rs:93`), so the hub's `Timeout` arrives as `ControlError::Refused`.<br><br>**As planned:**<br>- `holler say --pane P "…"` whose turn outlives `--timeout` (600 s by default) prints `pane-not-driven`, exit 3, while the body runs the turn on. Today it is exit 1 with the hub's text.<br>- `interrupt --pane` with an unacknowledged cancel gets the same misreport.<br>- Neither case is in the acceptance.<br><br>**A related trap in `reset --first`'s `Send`.**<br>- `ControlCall::say_with` derives `timeout_ms` and the read timeout from one argument (`control.rs:83-95`).<br>- Building `Send` with the 2 s accept wait would make the hub time the turn out at 2 s, racing the CLI's own 2 s deadline, and a delivered first message could be reported as failed.<br>- `say_cmd::run` builds with the turn budget and overrides only `call.timeout` (`say_cmd.rs:157-167`). | Notes for O, §2.<br><br>**Recommended:**<br>- the hub marks the resolve-time `not_connected` with an additive `data.reason`;<br>- `classify` maps `-32004` to `NotDriven` only with that reason;<br>- pin `Send`'s `timeout_ms` to the turn budget;<br>- add the timeout and ack cases to Acceptance 18. |
| 3 | warn | **D5's scan:** "the only `.send_prompt(` call is the one in the pass-through of `impl HarnessPort for Wired<dyn HarnessPort>` in `wiring/bind.rs` … the test asserts that file has exactly one; any other call fails".<br>**The merge order** lands #831 before #839. | choke point; blast radius | **#831 adds a second wrapper forward inside the scan's scope:**<br>- `holler-pane/src/ports/router.rs:118`, `self.by(at).send_prompt(at, session, text)`, inside `fn send_prompt` of `impl HarnessPort for` the router;<br>- #831 is at `d137a1d`, T-green PASS.<br><br>Once #831 is merged, the scan fails.<br><br>**The strip rule is also ambiguous.** "The text of `fn send_prompt` definitions stripped": if that strips the bodies, `bind.rs` has zero calls. | **Define the scan** as: a `.send_prompt(` call is allowed only inside the body of a `fn send_prompt`, which is a wrapper's pass-through:<br>- `wiring/bind.rs`;<br>- `ports/router.rs`, once #831 is merged.<br><br>Any other call fails.<br><br>**Name #831's router** in the clashes row and the Blast radius note. |
| 4 | warn | **The move:** "the pane-gate tests (E81, E82 and their neighbours) move with them to … `panes/gate_tests.rs`".<br>**Acceptance 4's file list:** "in `panes/gate_tests.rs` …: … an operator hold refuses first with its own kind".<br>**`live.rs`:** "`#[cfg(test)] mod tests;`" (no position given). | pattern consistency; test placement | **The tests to move drive `send_prompt`, which `panes/` cannot reach.**<br>- Every pane test in `circuit/dispatch.rs`'s `hold_tests` (`:513-819`) calls `send_prompt` with a `HoldGate` and a `Recording` sink. All three are private to `circuit` (`dispatch.rs:62`, `:163`).<br>- What these tests pin is the enforcement point: every variant, nothing on the socket, the hold refusing first. Rewritten against `pane_gate` alone, those pins would be lost.<br><br>**Where the `mod` line goes matters.**<br>- `code_of` cuts each file at its first `#[cfg(test)]` (`hold_single_path_test.rs:33-40`).<br>- If `live.rs`'s `#[cfg(test)] mod tests;` sits at the top beside `mod target;`, the existing scan (`:71`) loses `pub async fn query(` and panics.<br>- The hub's precedent puts it on the last line (`holds.rs:576`, `lockout.rs:590`).<br><br>**Out-of-line test files are scanned unstripped.** `rust_files` reads them in full, so the new forward-shape scan must not count them. | **Placement:**<br>- the tests that drive `send_prompt` stay in `circuit/dispatch.rs`'s `hold_tests`, rewritten on `ses_` ids and the new key;<br>- `panes/gate_tests.rs` holds the selection tests (record or recorded DRIVEN, fail-closed name order);<br>- correct Acceptance 4's files.<br><br>**Say:** each `#[cfg(test)] mod …;` line is the last line of its file. |
| 5 | warn | **D10:** `FakeHub`'s `prompt(Check\|Send)` "finds the pane whose `session_of_record` equals the id".<br>**D1:** the gate selects every pane whose record **or recorded DRIVEN** is the id, fail-closed, naming the first bad one in name order.<br>**Acceptance 19:** "Fakes and a real hub". | duplication (one predicate); test-harness drift | **The fake's selection is a second copy of the rule, specified differently.** It matches the record only and finds one pane, so it admits prompts the hub refuses:<br>- a prompt to a pane's recorded DRIVEN;<br>- a prompt to an id that two panes share.<br><br>The conformance that D10 runs against both implementations will either fail or, if it skips these cases, let the fake drift.<br><br>D1's "one predicate, not three" covers `prompt_refusal`, but not the selection, which is what this story changes. | **Put the selection in `holler_pane::gate`**, beside `prompt_refusal` (e.g. `first_refusal(panes, id) -> Option<(&Pane, PromptRefusal)>`).<br>- `panes/gate.rs` and `FakeHub` both call it.<br>- `conformance/hub.rs` pins the DRIVEN key and the two-pane order. |
| 6 | warn | **The brief, line 444** (#803's row): "Files #839 does not edit (`agent-skills/<the wizard skill's old directory name>`, …)". | blast radius (a tracked file) | **The brief fails a repo-wide test.**<br>- `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` (`crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:64`) scans every `git ls-files` path for that name.<br>- Reproduced on `bf9d1c2`: line 444 is the only hit.<br>- `cargo test --workspace` (F's verify command and CI) therefore fails, whatever the code.<br>- #803's brief (`9af2ad7`, its clashes table) flags the same line. | Reword line 444 (e.g. "the wizard's skill directory"). |
| 7 | warn | "Clashes with stories in flight" and the merge order, as of 20:58 MDT. | blast radius | **Moved since (local refs, not fetched):**<br><br>**#667 merged** (`0f5835e`, #855, 21:11 MDT), so the branch is behind `origin/main`.<br>- D9's target landed exactly as E155 has it (`reset_with_a_first_message` at `scenario.rs:144`).<br>- #667's new `profile_apply_scenario/**` uses none of `--first`, `send_prompt`, `Ports { }` or DRIVEN.<br><br>**#831 is built** (`d137a1d`, T-green PASS), not "A and T-red only":<br>- `HarnessPort` takes `at: &HarnessEndpoint` instead of `port: u16`;<br>- a 16th `Ports` literal (`tests/pane_claude/rig.rs:199`, so 14 files);<br>- the router forward (finding 3);<br>- `tx_switch.rs` has 230 lines changed, and its `send_prompt` step is at `:295`;<br>- `tx_launch` is split into submodules, so E129's citation moves.<br><br>**#835** has a second S REWORK (`d71b091`). `ports.rs` is now +96, where the brief says +74. `bind.rs` still has one forward.<br><br>**#648** is at S PASS (`8c5b861`), with `origin/main` merged in. The pair rule is still at `roster_cmd.rs:341`, which the brief's conditional edit covers.<br><br>**#803's brief now exists** (`9af2ad7`).<br>- Its D13 builds one local body with one attach row per OpenCode pane, `session_id` read from the registry. P2 and question 2 hold, and the dependency deferred to this pass is resolved.<br>- With one body for the whole fleet, a restart reverts every pane at once, and the per-connection slot is fleet-wide. | **Update the table and the counts.**<br><br>**Merge `origin/main`** into the branch before T.<br><br>**Note #803's D13** under question 2 and in D2's restart text. |
| 8 | warn | **D7:** the three codes are "declared … in `crates/holler-pane/src/hub.rs`".<br>**D2:** `control/rebind {token, session, harness_session_id}`.<br>**D3:** `Advertised { pub token: String, … }`.<br>**D3's messages** (`label1/oc (ses_…a)`, …) and `switch_command(pane, session)`. | naming; pattern consistency; untrusted input | **Three small drifts.**<br><br>**(a) Where `session-driven-elsewhere` is declared.**<br>- Only the switch's plan raises it (P6).<br>- "A verb … declares the code it owns" (`error.rs:10-11`), and every switch code lives in `tx_switch.rs`, including `SESSION_OF_OTHER_PANE` (`:66`), the code P6 compares it with.<br>- `pane-not-driven` and `session-held` are the port's answers, so `hub.rs` is right for those two.<br><br>**(b) The name of the token parameter.**<br>- An exact token id is `token_id` in the control methods (`control/token_ping`, `control/revoke`; `control.rs:167`, `:179`) and in the roster row (`roster.rs:152`).<br>- `token` is `control/test_drop`'s loose `find_target` lookup.<br><br>**(c) Untrusted text.**<br>- `SessionAd.harness_session_id` is an unvalidated `Option<String>` on the wire (`docs.rs:530-532`).<br>- The messages print advertised ids and names, and the remedy prints a session id into a command line.<br>- The existing builders interpolate only a validated `PaneName`, and `check_unclaimed` quotes its target (`tx_switch.rs:354`). | **(a)** Declare `SESSION_DRIVEN_ELSEWHERE` in `tx_switch.rs`.<br><br>**(b)** Use `token_id` for both the parameter and the field.<br><br>**(c)** Quote advertised values with `findings::quoted`. Build `switch_command` only for a session that passes `parse_session_id`; otherwise set `remedy: None`. |

## Notes for O

There is no O on the automated path, so this BLOCK stops the run (`needs-amendment`). Amend the brief and start a **fresh** run. Do not use `resumeFromRunId`, which would replay this verdict.

**Nothing below reopens the following:**
- P1 to P6. Finding 1 applies P4's own rule (ii) to the exclusion, and finding 2 implements P5's "of the pane's id".
- D1's join and gate keys.
- D2's request, refusals, swap, deadline, slot and bounds.
- D3's step order, `HubPort` and its file, the "Ambiguous drivers" remedy and reconcile's finding.
- D4, D6, D8, D9 and D11.

### 1. Say which bodies another pane's evidence removes (finding 1)

**(a) Recommended: apply P4's rule (ii) to the exclusion.**
- **The rule.** A body that advertises another pane's session of record, or another pane's recorded DRIVEN, is that pane's and is never a candidate. The exception is a body that also advertises this pane's own session of record or recorded DRIVEN: this pane's evidence is checked first, so a stale DRIVEN on another pane never takes a pane's own body.
- **`driver_of`'s input.** It takes the other panes' records and recorded DRIVEN, as read at the start of the plan or of the reconcile pass. Not the values the pass writes, so the result does not depend on pane order.
- **The consequence to state.** `pane switch P1 <that body's id>` is refused `session-driven-elsewhere` by P6, because the body advertises the target and is not P1's driver. The `Unproven` remedy can then never pair a body that another pane's evidence proves.
- **The texts to update:** P4's sentence (as an elaboration), D3's first step, section 11 point 3 (verbatim), Acceptance 20(d) and `driver_of`'s table.
- **The rows to add to Acceptance 20:**
  - (b)'s last case is `None`, and P1's switch to that body's id is refused `session-driven-elsewhere`;
  - a body that advertises P1's record, while P2's stale recorded DRIVEN names the same id, is P1's driver.

**(b) Alternative: keep the rule and correct the three texts.**
- Acceptance 20(b)'s last case becomes `Unproven([P2's body])`, and P1's switch is refused.
- D2's "and nobody's stored DRIVEN" goes.
- Question 5 reads "that no pane's record ties to it".
- Section 11 states that the `Unproven` list can name a body that another pane's recorded DRIVEN points at.

### 2. Tell a resolve-time `not_connected` from one after delivery (finding 2)

**(a) Recommended: the hub marks the resolve-time case.**
- The resolve-time `NotConnected` arm of `control_say.rs`'s one mapping adds a `data.reason` (e.g. `"offline"`). So do the same arms of `control/interrupt` and `control/answer`.
- That is additive, optional `data` on an existing code. Document it in `docs/protocol/v2.md`'s `-32004` row. No golden file changes.
- `classify` maps `-32004` to `NotDriven` only when that reason is present. A bare `-32004` falls through to today's arms, exit 1 with the hub's text. That covers `say`'s timeout and cancel, and `interrupt`'s ack timeout and failed redirect.
- This implements P5's "of the pane's id" as written.

**(b) Alternative: `NotDriven` is `-32003` only** (`unknown session` and `ambiguous`). A `--pane` to an offline body keeps today's text and exit 1. This needs the main session to amend P5's sentence.

**Either way:**
- **Pin `Send`'s `timeout_ms`.** It is the turn budget, never the 2 s accept wait: build the call as `say_cmd::run` does, and override only `call.timeout` to `QUEUE_ACCEPT_WAIT`.
- **Add three cases to Acceptance 18:**
  - a `say --pane` whose turn outlives `--timeout` exits 1 with "no reply from … within Ns";
  - an `interrupt --pane` whose cancel is not acknowledged exits 1, as today;
  - a `reset --first` whose `Send` is accepted is never reported as a failure.

### 3. Fold in the warns while amending

- **Finding 3:** the `.send_prompt(` scan's rule (only inside the body of a `fn send_prompt`), and #831's router.
- **Finding 4:** which tests move to `panes/gate_tests.rs`; each `mod` line last in its file; Acceptance 4's files.
- **Finding 5:** one selection function in `holler_pane::gate`, used by the hub and by `FakeHub`, pinned by the conformance.
- **Finding 6:** reword line 444, or the workspace tests fail.
- **Finding 7:** the clashes table and its counts, and a merge of `origin/main` before T.
- **Finding 8:** `SESSION_DRIVEN_ELSEWHERE` in `tx_switch.rs`; `token_id`; quoted messages and a validated remedy id.

## Patterns referenced

- **`holler-pane`:**
  - `crates/holler-pane/src/tx_switch.rs` (`check_unclaimed`, `check_listed`, `check_idle`, the declared switch codes, `parse_session_id`);
  - `crates/holler-pane/src/error.rs` (who declares an open code);
  - `crates/holler-pane/src/findings.rs` (`remedy`, `with_fix`, the command builders, `quoted`).
- **Hub:**
  - `crates/holler-hub/src/live.rs` (`query`, `insert`, `replace_session`, `resolve_session`);
  - `crates/holler-hub/src/circuit/dispatch.rs` (`pane_gate`, `send_prompt`, `handle_response`, `hold_tests`);
  - `crates/holler-hub/src/circuit.rs` (the `Say` arm);
  - `crates/holler-hub/src/talk.rs` (`say`'s prelude, `send_turn`);
  - `crates/holler-hub/src/control_server.rs` (the `say`, `interrupt` and `answer` error mappings, `encode_error`);
  - `crates/holler-hub/src/control.rs` (`say_with`, `CLIENT_TIMEOUT`, `token_id` params);
  - `crates/holler-hub/src/holds.rs` (`would_refuse`);
  - `crates/holler-hub/src/control_hold.rs`;
  - `crates/holler-hub/src/roster.rs` (`advertise`, `Row`).
- **Body:** `crates/holler-body/src/session_manager.rs` (`start`, `presence_doc`), `session_manager/task.rs` (`SessionPresence`, the `select!` loop).
- **CLI and the test kit:**
  - `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`;
  - `crates/holler-cli/src/pane/wiring/store.rs`;
  - `crates/holler-cli/tests/hold_single_path_test.rs`;
  - `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs`;
  - `crates/holler-pane-testkit/src/fault.rs`.
- **Siblings (local refs):**
  - #831 `ports/router.rs` and `ports.rs` at `d137a1d`;
  - #667's `scenario.rs` on `origin/main` (`0f5835e`);
  - #835 at `d71b091`;
  - #648 at `823e0a3`;
  - #803's brief at `9af2ad7` (D13).
