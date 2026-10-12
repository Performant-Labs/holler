# [feat(body): a body driver for a Claude Code pane, so holler say --pane reaches it](https://github.com/Performant-Labs/holler/issues/850)

Part of epic #633 (decision 9), a follow-up to #831. This is the complete statement of story #850, prepared on 2026-10-12 against `main` at `b76f4d2` and refreshed the same day against `main` at `46b3549`, which holds #831 (PR #857), #835 (PR #860), #839 (PR #863), #853 (PR #861), #665, #648 and #667, and refreshed again on 2026-10-12 against `main` at `ef881fc`, which also holds #803 (PR #866), #647 part 2 (PR #872), #846 (PR #868), #696 (PR #874), #844 (PR #870), #845 (PR #873) and #871. No other story is in flight. The operator decided on 2026-10-12 that the story **is in the #633 release and gates it**, that it is **amend-first** (the ADR 0014 and ADR-0021 amendment comes before any test), that it runs at `rigor: in-session`, and that it **launches after #839 has merged** (it has), because it needs #839's join of a body to its pane. It is a **preparation**: nothing was launched. The GitHub issue (#850) is the source of truth where this brief is silent; this brief is the decision where they differ.

## Needs operator decision

- **N1. The merged wizard says a Claude Code pane has no body row, and this story does not edit it.** #803 (PR #866) is on `main`; its `agent-skills/herdr-setup/SKILL.md` (lines 77, 96, 843, 1327, 1464) and `docs/setup-wizard.md` (line 172) say a Claude pane has no body row and no roster row "until #850", and its Stage 8 derives attach rows for OpenCode panes only (E46). The earlier brief left that to "#803's refresh after #850", which no longer exists because #803 has merged. After #850 those sentences are false, and A4's item 19 says the row is derived "by the setup wizard (#803)". Not decided here (Scope and Q3 stand: #850 edits no wizard file). Choose: (a) a follow-up story edits the wizard to derive Claude rows and fix its text (the path this brief assumes), or (b) #850 also edits those six wizard places (text only, with their `skilltext` tests) and adds nothing to Stage 8. Either way, A4's item 19 wording "or by the setup wizard (#803)" is accurate only after (a) or (b).

**Refreshed after #839 and #835.** #839 (PR #863) and #835 (PR #860) are on `main`. Every line of this brief that rested on what they would change was re-checked against the merged code, ADRs and docs and now states the fact; no marker is left. "What was re-checked after #839 and #835 merged" lists what each re-check found. The excerpts E41 to E45, E52 and E53 are cut from the merged code and docs; E46 is cut from the merged wizard (#803); everything else is checked against `main` at `ef881fc` (every excerpt was re-run on it, line numbers moved where #803, #647 part 2 and #696 added lines).

**One rule, one place.** The plan check blocked #839 three times for a rule stated two ways, so this brief states each rule once. "Where each rule lives" says where. The verbatim ADR and protocol text in "The ADR and the dependent docs" is the one allowed copy of a rule and is the rule; everywhere else this brief refers to a rule by its name ("the turn rule") and does not restate it. Cite an entry of the Evidence section as E<n>; every excerpt was cut by the command printed above it, on the tree this brief was written on, and was re-run at the end.

## Problem

The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body, which drives its harness (an ACP child, or an HTTP attach to an OpenCode server, ADR 0014). #831 launches, registers and observes a Claude Code pane through the pane engine and gave its adapter a delivery (`HarnessPort::send_prompt`, which writes to the process's inbox socket; since #839 no verb or engine calls it, and a first message goes through the hub), but a Claude Code pane has **no body**, so `holler say --pane NAME` cannot reach it, the hub's holds and pane gate do not see a prompt to it, and an `interrupt` has no body to cancel through (E24). #831 closed its own acceptance with the `say` step written down as an ignored test that names #839 and #850 (E21).

## Scope

In this order (the plan items P1 to P7 are in "Plan"):

1. **Contract first.** ADR 0014 (points 7 to 11), ADR-0021 (section 5's crate rule, section 15 items 9, 18 and the new item 19, "Not built here", "Deferred to named stories", "Decisions taken"), `docs/protocol/v2.md`, the README harness recipe, `docs/pane-control.md`, `CHANGELOG.md`. One commit, before any test.
2. **`holler-body`:** the body config row for a Claude Code process, the attach driver over the one `HarnessPort`, the session manager's injection of that port, the `query/support` probe.
3. **`holler-cli`:** `body run` builds the one Claude Code harness the pane wiring builds and hands it to the body.
4. **`holler-pane`:** a single-process switch or reset re-points the body (the drive rule), and `doctor` records DRIVEN for a single process.
5. **Tests**, including the default-run scenario that replaces #831's ignored `say` step (Acceptance 5).

### Not in scope

- **The hub.** No file of `crates/holler-hub/src` or `crates/holler-proto/src` changes (D7, D11). The hub treats a Claude row as any attach row.
- **Any change to `holler-adapter-claude`.** It already implements everything the driver calls (E10, E11, E12).
- **The setup wizard (#803).** Deriving the body's config from the registry, including the new Claude rows, is the wizard's (E46); #803 has merged and builds rows for OpenCode panes only, so a follow-up story owns the Claude rows (see "Needs operator decision", N1). This story writes the row's grammar and its docs, and the operator can write a row by hand.
- **The roster's join of a row to a pane** (`roster_cmd::pane_label`, #648's display rule, E18): D8.
- **A new `holler body` verb or flag**, a new harness id, a new error code, a new wire field, a new log component.
- **Reading a reply from the process.** The driver streams no text (the turn rule).
- **An in-place interrupt, or any keypress** (the operator's decision of 2026-10-11, ADR-0021 section 15 item 9).
- **The opt-in scratch scenario of `pane_integration/scenario.rs`** (real Herdr, tmux and OpenCode). It starts no body and stays as it is (D10).

## Where this brief is the decision

### The facts that force the design (each is in the Evidence section)

- **F1. One choke point.** A prompt reaches a body only through the hub's `send_prompt`, which runs the hold and the pane gate before it writes the frame (E17). It does so for every driver of every body, so a Claude row needs nothing in the hub.
- **F2. The adapter already does the work, and only the CLI builds it.** `ClaudeHarness` implements delivery (`send_prompt`), process lookup by session (`locate_process`) and activity (`session_activity`, from `claude agents --json`) behind `HarnessPort` (E10, E11, E12), and `holler-cli`'s wiring builds it from the environment (E16). The adapter depends on `holler-pane` alone (E13).
- **F3. The body treats its drivers alike.** `Driver` is one enum with one place that chooses a driver (E4). The session task owns the queue, the turn id, presence, `Replace`'s cancel-then-prompt and the crash isolation that drops a driver after a turn that ended `Error` (E5, E6); a driver's refusal of a cancel is the wire's `-32004` (E7); a driver streams `DriverEvent`s ending in a `StopReason` (E9). An HTTP driver's cancel with nothing in flight applies at once (E8).
- **F4. The row grammar cannot say it.** Validation requires `endpoint` of every attach row (E3), and `claude` is already the body harness id of the ACP bridge on a spawn row (E2, E29).
- **F5. The crate graph.** `holler-body` depends on `holler-proto` alone; the hub on `holler-proto` and `holler-pane`; an adapter on `holler-pane` alone (E13, E14), and ADR-0021 says `holler-cli` is the crate that depends on the adapters (E15).
- **F6. `body run` has one caller, in the CLI** (E48), which can hand the body what it builds.
- **F7. A relaunch keeps the session and changes the pid.** The adapter finds a process by its session id (E11, E12), and `interrupt --pane` relaunches on the session of record (E19). A driver that finds the process by session on every call follows an interrupt with nothing re-pointed.
- **F8. A switch or reset of a single process leaves the body behind.** Its act is the relaunch, which stores the record (E22), and #839's plan, as merged, asks the hub nothing for it, takes no driver and writes no DRIVEN (E41, E42). Once a body exists, it would keep driving the session the pane left.
- **F9. The roster joins by the row's label, and #839 leaves that to #648 and #803** (E18, E43). With #803's one local body for the fleet the row is `<token label>/<pane name>` (E46; the merged wizard), so that rule pairs no row with a pane, for either harness. That is the display rule and not this story's.
- **F10. An id and a name cannot collide when the id is a UUID.** A name segment is 1 to 32 characters (E28); a UUID is 36.
- **F11. A default-run test can start a real hub, a real body and a fake harness** (E37, E38), and `holler-cli` declares each target by hand while `holler-body` does not (E47). The test kit's `FakeClaude` is a `HarnessPort` with processes, statuses and an inbox (E49).
- **F12. Nothing of the process's reply is readable.** The adapter reads no transcript contents (E50), and `claude agents --json` has no progress heartbeat (E34).

### The decisions

Each says what is decided, names the rule where one is stated (see "Where each rule lives"), and lists what it rejected.

**D1. One implementation of delivery and observation: the body drives a `HarnessPort` that `holler-cli` hands it.** The decision of the issue's item 1: reuse. The driver calls only `send_prompt`, `session_activity` and `locate_process` of a `HarnessPort`; `holler body run` builds the production one with the pane wiring's own function (E16) and passes it down `connection::run` to the session manager (E48); a test passes `FakeClaude` (E49). What the body may depend on is the crate rule (A1). Why: the frame, the hook-written socket file and the held-message window are the adapter's, and its frame is **Read** and not documented, so the opt-in real test guards one implementation (E33); the environment (`HOLLER_CLAUDE_BIN`, `HOME`, the state directory) is resolved once, by the wiring (E16), and the body runs with the same state directory as the pane verbs (E48). Rejected: **(a) a second implementation in `holler-body`** (the issue's own rejection unless reuse is impossible: it is possible); **(b) the body depends on `holler-adapter-claude`** (the adapter's environment lookups are private to the CLI's wiring and shared with the OpenCode adapter's, so they would be copied or moved, and the crate rule that only the CLI depends on adapters would give way); **(c) a process-wide `OnceLock` of the port** (a global the parallel tests of one binary would share); **(d) moving the adapter's frame into `holler-proto`** (the hub does not need it).

**D2. One row shape, chosen by the harness id and the mode, kept apart from the ACP bridge (the row rule, A2 point 7).** The pair is read in one function of `holler-body`, `SessionConfig::attaches_to_claude_code`, which config validation, the driver choice and the support probe all call (a source-scan test pins that no other place compares the harness id with `claude`). Rejected: **(a) a new harness id** (`claude-code`): ADR-0021 section 15 item 1 decided that no id is added, and `HARNESS_IDS` is pinned to `HarnessKind` (E29); **(b) a new `mode` value** (it would be a third transport in a grammar ADR 0012 gave two shapes, E2, with the same fields); **(c) a locator key** (`pid`, or the pane's name): a pid changes at every relaunch, and nothing verified says `agents --json` reports a pane's name (E12 reads `pid`, `sessionId`, `status` and `waitingFor` and nothing else), so the one locator is the session id, which the adapter already matches on (F7). Why the session id is a UUID is F10's.

**D3. The driver delivers and observes through the port, and a turn follows the turn rule (A2 points 8 and 9).** The turn is tracked by polling `session_activity` because it is the only evidence there is (F12, E34): no hook is installed for it, no transcript is read (E50) and no progress is invented, so the roster shows what a poll with no heartbeat knows (the turn rule says what that is). Every port call runs in `spawn_blocking` under one outer bound (`OP_BOUND`, 10 s), inside the adapter's own bounds (I5). Rejected: **(a) hook events** (a second channel with a second file to trust); **(b) reading the transcript for the reply** (the adapter's contract is that contents are never read, E50); **(c) a `Chunk` per poll to keep the roster `working`** (it would hide a wedged process, which the spike says shows `busy` like a working one, E34).

**D4. Cancel, replace, answer, and what `interrupt --pane` does (the cancel rule, A2 point 10, and the interrupt rule, A3).** `interrupt --pane NAME` and its text form are **unchanged** from #831, and A3 rewrites the reason that item 9 gives for them, since "it has no body" is no longer true. The body's refusal of a cancel is the cancel rule, and it rides the existing `-32004` of a driver's failed cancel (F3, E7), so there is no wire change. Rejected: **(a) the body relaunches the process** (a second owner of a process the engine owns, and a resident process doing a verb's work, I5); **(b) `interrupt --pane NAME TEXT` as a relaunch followed by a `say`** (the interrupt rule's reason, A3, which the refusal already acts on by telling the operator to say the text after); **(c) a signal** (it ends the process).

**D5. How the body is pointed at a pane's new session: the engine's drive step, extended to a single process (the drive rule, A4 item 19; the driver's side is A2 point 11).** This decision reverses #839's D3 item 8 as merged (E41, E42), and the operator accepted that on 2026-10-12: P5 rewrites the words that say a single process takes no driver (A4, A6). Neither `launch`, `relaunch`, `profile apply` nor any other verb starts, stops or restarts a body (I5; the registry is the one place that tracks panes, epic decision 2). Rejected: **(a) the body follows the pane by name** (no verified field, D2 (c)); **(b) no re-pointing**, so after a reset the body keeps a session the pane left and the gate refuses everything as `pane-shown-driven-mismatch` once DRIVEN is recorded; **(c) the engine writes the body's config file** (the wizard owns it, and the hub and the CLI would acquire a second way to configure a body); **(d) the body reads the pane registry** (a body is a peer of the hub, not a client of its store, and `PANE_METHODS` is frozen).

**D6. No Holler verb writes the body's row (item 19, A4).** An OpenCode pane's row is derived in the same way (E46). Rejected: a `holler body attach init --harness claude` (an unasked product surface; the existing verb is pure HTTP plus a file write, and a follow-up may add one).

**D7. The hub's holds and pane gate see a prompt to a Claude row exactly as to any row, and the hub changes nothing.** The hold is keyed `<label>/<session>`. Since #839 the pane gate is keyed on the harness session id that the session being prompted advertises: `holler_pane::gate::first_refusal` selects the panes whose session of record or recorded DRIVEN is that id, and the hub's `pane_gate` and `send_prompt` pass the advertised string on unchanged (E17). A Claude row advertises its row's session id (its session task's mirror of the row, E31), a UUID, so it reaches the gate as an OpenCode row's `ses_` id does; `say --pane` sends the pane's session of record, which the hub resolves by that id, and the CLI refuses a UUID typed as a SESSION (a name segment is at most 32 characters, F10). Proving that the gate and the hold see the row is a test (Acceptance 4 and 5), not code.

**D8. The roster row exists and its join is unchanged (the roster rule, A4 item 18).** The join stays #648's `pane_label`, which #839 leaves alone (E43) and which pairs no row under the wizard's one local body (F9). Changing it is the operator's call (Q1), not made here because it would change #648's pinned behaviour for both harnesses.

**D9. `query/support` for a Claude attach row is a PATH lookup (the support rule, A5).** Rejected: running `claude agents --json` from `support` (it would pass the adapter's port into a function that takes the configuration only, and into `holler body support`, which runs without a body).

**D10. Acceptance 4 of the issue is a default-run test, not an opt-in one.** The scratch scenario of `pane_integration/scenario.rs` runs real Herdr, tmux and OpenCode and starts no body (E21 names the `say` step; it lives in `pane_claude/mixed.rs`, on the fakes). The `say` step is met by a default-run test over a real hub, two real bodies, a fake OpenCode server and a fake `claude` program (Acceptance 5), so CI runs it; the ignored test is deleted. Nothing touches a live fleet, a real Herdr or a real Claude Code.

**D11. No wire change.** No field, no method, no error code, no golden, no log component. `Code::ALL` keeps its 17 entries; the method catalog is #839's 23 rows (E35), and the Claude driver implements #839's `session/rebind` with no new field, method or reason (the rebind rule).

**D12. Build guards.** No `unwrap`, `expect` or `panic` (the workspace denies them); an `#[allow]` carries `// #850` on the same line (E39); every touched file stays under 900 lines (E39; "Where the code goes" says where new code goes); a `## [Unreleased]` CHANGELOG bullet; the README's "Debug output" table is unchanged because no log event is added (the driver logs through the session task's `log_session` lines and the existing `attach_failed` warning).

### Where each rule lives

| Rule (its name) | The one place that states it |
|---|---|
| the crate rule | ADR-0021 section 5, the bullet A1 adds |
| the row rule | ADR 0014 point 7 (A2) |
| the delivery rule | ADR 0014 point 8 (A2) |
| the turn rule | ADR 0014 point 9 (A2) |
| the cancel rule | ADR 0014 point 10 (A2) |
| the rebind rule | ADR 0014 point 11 (A2) |
| the interrupt rule | ADR-0021 section 15 item 9, as A3 rewrites it |
| the drive rule | ADR-0021 section 15 item 19 (A4) |
| the roster rule | ADR-0021 section 15 item 18, the paragraph A4 appends |
| the support rule | `docs/protocol/v2.md` section 5.3, the paragraph A5 adds |
| the outer bound of every port call the driver makes (`OP_BOUND`) | D3 |

The guide, the README and the CHANGELOG describe these rules in a sentence and point at the ADR; the Acceptance section names the test that pins each rule and does not restate it.

## The ADR and the dependent docs

Amend first: one commit, before any test, with the texts below, which F copies. Where #839 rewrote a sentence an edit names, the edit names the merged words; a phrase that wraps across lines in the file is matched with line breaks and indentation read as one space. ADR text is written to be pasted; a block is the rule it carries.

### A1. ADR-0021 section 5, "Decided: the crate dependency rules" (E15)

Keep the bullet that begins "Each adapter crate depends on `holler-pane`" as it is, and add this bullet directly after it:

~~~~markdown
- `holler-body` depends on `holler-pane` (#850) for the `HarnessPort` trait and its types, and on no adapter crate: the body's driver for a Claude Code process drives a `HarnessPort` that `holler-cli` builds from the adapter and hands to `holler body run`, so delivery and observation have one implementation and the body holds no knowledge of the `claude` binary, its sockets or its environment.
~~~~

### A2. ADR 0014: the header, points 7 to 11, the Consequences

Add this line to the header block of `docs/adr/ADR-0014.md`, after the `Supersedes` line:

~~~~markdown
**Amended:** [#850](https://github.com/Performant-Labs/holler/issues/850) (2026-10-12): points 7 to 11 (attach to a Claude Code process) and the Consequences bullets that follow them.
~~~~

Add this section after point 6 and before `## Why`:

~~~~markdown
### Amendment (#850): attaching to a Claude Code process

Points 1 to 6 describe an attach to an OpenCode session, which lives behind an HTTP surface, so the row names an `endpoint`. A Claude Code pane (ADR-0021 section 15) is one interactive process with no endpoint. Points 7 to 11 say what attaching to it means. Points 1, 3, 5 and 6 hold unchanged, and point 4 holds with the inbox socket of point 8 as the process's own message surface, never its terminal.

7. **One row shape, chosen by the harness id and the mode.** An attach row whose `harness` is `claude` attaches to a Claude Code process. Where point 2 says an attach row names an `endpoint`, this row names none: it names `session_id`, the session that process holds, which must be a UUID in its 36-character form (lower-case hexadecimal digits in groups of 8, 4, 4, 4 and 12, joined by hyphens). Such a row with an `endpoint`, with `interrupt = "http"` or with no `session_id` is refused when the file is parsed, naming the key. The id `claude` still names the ACP bridge on a spawn row (ADR 0012), and a row cannot be both, since a spawn row has a `command` and an attach row ignores one. No harness id is added, and every attach row of any other harness id is an OpenCode HTTP row, as before.
8. **Delivery and observation have one implementation: the Claude Code adapter's.** The driver delivers a `session/prompt` with the adapter's `send_prompt`, one frame line written to the process's inbox socket, and reads the process's state with its `session_activity`, which reads `claude agents --json`. It opens no socket and runs no `claude` command of its own, and it never reads a login (ADR-0021 invariant I7). It finds the process that holds the row's session on every call, by that session id and never by a remembered pid, so a relaunch of the same session, which is how a Claude Code pane is interrupted, needs nothing from the body. The attach is that same lookup and fails closed, as point 2 says of an HTTP row: when no process holds the session the row is left out of presence and the attach is retried, and nothing is created or started.
9. **A turn.** A turn starts when the adapter accepts the message, and the row is `working` from then on. The driver then polls the process's state every 500 ms and ends the turn `end_turn` when the process is idle again after it was seen busy, or idle 5 s after the write without having been seen busy (a turn too short to see). It ends the turn `cancelled` when a different process holds the session than the one the message was written to (the pane was relaunched, which loses the turn in flight) and `error` when no process holds it. When the adapter refuses the message (it was held behind a dialog, the process holds another session, no socket is known), the turn ends `error` after one text chunk that carries the adapter's own message, which quotes nothing the process printed. A process waiting on a permission prompt or a dialog makes the row `input-required`, with nothing pending, and busy again makes it `working`. The driver reads nothing the process prints, so it streams no text: `say` returns an empty reply when the turn ends, and the pane shows the reply. It emits no progress of its own, so a long turn reads `stalled` in the roster after the stall threshold; `claude agents --json` has no heartbeat to say otherwise.
10. **Cancel, replace and answer.** The body cannot cancel a turn in a process it does not own, and a signal would end the process, so `session/cancel` while a turn is in flight is refused, naming `holler interrupt --pane NAME`, which relaunches the pane on its session of record (ADR-0021 section 15, item 9); with no turn in flight it applies at once, as it does for an HTTP row. `session/prompt` with `replace` is a cancel followed by a prompt, so it is refused while a turn is in flight and is a plain prompt when none is. `session/answer` is always refused, since a permission prompt is answered in the pane.
11. **`session/rebind`.** The request that re-points an attach session to another harness session on the same endpoint (ADR-0021 section 11, point 2) is served for a Claude Code row by the same session task and under its same refusals (`docs/protocol/v2.md`, "Re-pointing"). The task attaches the new session id first, and the check that the target exists is that attach (point 8): a process that holds the session must be listed by `claude agents --json` before the request's deadline. A target that no process holds is refused as an id the endpoint does not have, and a lookup still running at the deadline is dropped and refused as an endpoint that did not answer, the old binding kept in both. The process is never touched: the relaunch that gave the session to a new process is the pane engine's, and has finished before the request arrives.
~~~~

Add these two bullets at the end of the Consequences list:

~~~~markdown
- The body depends on no adapter: its driver for a Claude Code process drives the `HarnessPort` that `holler body run` is handed (ADR-0021 section 5).
- A body restarted with a row whose session no process holds any more is the failed attach of point 8: the row stays off the roster until a process holds that session again or the row is written afresh from the registry.
~~~~

### A3. ADR-0021 section 15 item 9: the interrupt rule (E23)

Make these two replacements in item 9 (each phrase is on one line of the file today):

- Replace `does not go to the hub (it has no body, item 18)` with `does not go to the hub (a relaunch is not an operation a body can run on a process it does not own, ADR 0014 point 10)`.
- Replace `` `interrupt-text-unsupported` in this story.`` with `` `interrupt-text-unsupported`: a relaunch is not an atomic cancel-and-redirect, since the new process must run its launch hook before a message can reach it, so the text is said after the interrupt.``

### A4. ADR-0021 section 15: item 18, the new item 19, "Not built here", "Deferred to named stories", "Decisions taken" (E24, E25)

#839 rewrote three clauses of item 18 and the "Deferred" bullet about DRIVEN, appended decisions 34 to 47, and wrote four present-tense statements that name #850 as future (E52, E42). The edits below name the merged words.

**Section 8, the single-process paragraph (E52).** Replace `Since #839, `reset --first` on a single process is refused `pane-not-driven` in the plan, before anything is stopped: no body drives a Claude Code pane until #850, and the hub's gate is the only path a first message takes (section 11, point 6).` with `Since #839 a first message goes only through the hub's gate (section 11, point 5); how a single process meets it is the drive rule (section 15, item 19).`

**Section 9, the `say`, `interrupt`, `answer` row of the refusals table (E52).** Replace `and `say` to a Claude Code pane is `pane-not-driven` until a body drives it (#850)` with `and `say` to a Claude Code pane is `pane-not-driven` when no body drives it`.

**Section 11, point 2 (E42).** Replace `Only the HTTP attach driver implements it; a body for another harness implements it in its own driver.` with `Only an attach driver implements it: the HTTP driver, and the Claude Code driver (ADR 0014 point 11); a spawn session refuses.`

**Section 11, point 6 (E42).** Replace `Only an OpenCode attach body exists today. A Claude Code pane (#831) is in this case until a body can drive it (#850), and its switch and reset, which are a relaunch, ask the hub nothing; the join is a string comparison of the advertised id with the session of record, so its session id joins the same way once a body advertises it, and that body implements `session/rebind` in its own driver.` with `A Claude Code pane (#831) is in this case while no body advertises its session of record; the join is the same string comparison for it (section 15, item 19).`

**Item 18.** In its last sentence replace `no roster row until a body drives its session (#850);` with `a roster row once its body runs (item 19);`, and append this paragraph after the sentence's end ("... show it now."):

~~~~markdown
    **As built (#850):** the body driver exists (item 19), and the `say` step of the scenario is a default-run test, `say_pane_scenario_test`, for an OpenCode pane and a Claude Code pane both, through the hub's prompt path with its holds and its pane gate in force. A Claude Code pane's body serves the row `<token label>/<pane name>`, which appears in `holler roster` as an OpenCode pane's does; the pane columns are joined to it by `roster_cmd::pane_label`, #648's display rule, which #850 does not change.
~~~~

**New item 19**, directly after item 18's paragraph and before "Not built here":

~~~~markdown
19. **The body of a Claude Code pane (#850).** A Claude Code pane has a body: a `[[session]]` row with `harness = "claude"`, `mode = "attach"` and the pane's session of record as `session_id` (ADR 0014 points 7 to 11), served by the `holler body run` that already serves the fleet's OpenCode panes. Nothing of Holler's starts, stops or restarts that body, and neither `launch`, `relaunch` nor `profile apply` touches it (I5, and epic decision 2: the registry is the one place that tracks panes): the row is derived from the registry by the operator or by the setup wizard (#803), as an OpenCode pane's row is.
    - **The drive rule.** A switch or a reset of a `SingleProcess` pane finds the body that drives it in the plan, exactly as for a server pane (section 11, point 3) with an empty server list, and when it finds one it re-points that body after the relaunch has located the process on the target session: `rebind`, which the body checks against `claude agents --json` (ADR 0014 point 11). The record's `last_observed.driven` is then the id the body acknowledged, written by a second compare-and-swap, because the relaunch stored the record itself. A failure of the `rebind` or of that write is a failure after the act, reported with the reconcile step. `doctor` records DRIVEN for a single process by the same join, where it used to skip the step. A pane with no body is `Driver::None`: its verbs work without the drive step, DRIVEN stays `None`, and `reset --first` is refused `pane-not-driven` before anything stops; a pane with a body sends and checks that first message through the hub's gate as a server pane does (section 11, point 5). An interrupt and a doctor repair keep the session, so the drive rule does not run for them.
~~~~

**"Not built here, and where it goes" (E24).** Replace `The body driver for `say` (#850), which also gives a Claude pane its roster row; the hub's join (#839);` with `The hub's join (#839);`.

**"Deferred to named stories" (E25).** Delete the sentence `A body for a Claude Code pane, so that `holler say --pane NAME TEXT` and `reset --first` reach it and DRIVEN exists for it: #850 (section 15, item 18).` from its bullet, so that the bullet begins ``pane-stalled` waits with #837's time source`.

**"Decisions taken" (E25).** In item 24, replace `The body driver is #850.` with `The body driver is #850 (item 19).` and append one new item (`main` ends at 49 at `ef881fc`: 48 is #803's wizard decision and 49 is #647 part 2's, so this story's decision is **50**; if another decision lands first, take the next free number when the PR is made):

~~~~markdown
**A Claude Code pane's body drives the `HarnessPort` that `holler-cli` hands it** (2026-10-12, #850; the operator decided on 2026-10-12 that #850 is in the #633 release and accepted that #839's drive step reaches a single process, which reverses #839's D3 item 8; the other choices were made under the operator's standing rules, so the operator may overturn them): the body holds no adapter, `claude` gets no second harness id, the body never relaunches a process, and #839's drive step reaches a single process. The rules are section 5, section 15 items 9 and 19 and ADR 0014 points 7 to 11; the alternatives rejected are in `docs/handoffs/850-brief.md`.
~~~~

### A5. `docs/protocol/v2.md`: the support rule, the attach section, the presence line (E35)

After the paragraph of section 5.3 that ends "... the moment it stops." add:

~~~~markdown
A harness id backed by an `attach`-mode row that attaches to a Claude Code process (`harness` `claude`, ADR 0014 point 7) has no endpoint to dial. Its probe is the spawn row's: `ok:true` with `how: "attach: claude"` when `claude` resolves on `PATH`, else `ok:false` with `reason: "claude not found on PATH"`. It does not run `claude`, and it does not say whether a process holds the row's session; `holler pane doctor` reports that.
~~~~

In "Attach-mode sessions (issue #195)", after its first paragraph ("... another process already owns."), add:

~~~~markdown
An attach row of harness `claude` is dispatched to the Claude Code attach driver instead (ADR 0014 points 7 to 11). Every bullet below holds for it, except that its `shutdown` drops its own state, since it has no loop to end.
~~~~

In the presence field list, replace `(the existing harness session being attached to)` with `(the existing harness session being attached to; for a Claude Code row, its session id, a UUID)`.

In "Re-pointing (#839)" (E35), after its last sentence ("... attaches the session of its configuration again."), add:

~~~~markdown
A `session/rebind` of a Claude Code row (ADR 0014 point 7) is served by the same task under the refusals above; ADR 0014 point 11 says what its existence check is.
~~~~

### A6. README, `docs/pane-control.md`, CHANGELOG

**README**, section "Harness recipes" (E36): add, after the ACP recipe's last bullet, this subsection:

~~~~markdown
### A Claude Code pane

A pane that `holler pane launch --harness claude` started runs one interactive Claude Code process, and `holler say --pane NAME` reaches it through a body whose row attaches to that process (ADR 0014, points 7 to 11). The row names the pane's session of record and no endpoint:

```toml
[[session]]
name = "c1r1"
harness = "claude"
mode = "attach"
session_id = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11"   # the pane's session of record
```

`holler pane get c1r1 --format=json` prints that session (`data.pane.session_of_record`). This `claude` is the pane's interactive process; the ACP bridge above is a spawn row with a `command`.
~~~~

**`docs/pane-control.md`** (E26): (1) in "The prompt verbs", replace the sentences from `` `holler say --pane NAME TEXT` does not reach a Claude Code pane yet`` to `` `holler pane reset PANE --first TEXT`.`` with `` `holler say --pane NAME TEXT` and `holler pane reset PANE --first TEXT` reach a Claude Code pane through its body (a `[[session]]` row, ADR 0014 points 7 to 11, written from the registry; nothing here starts it), and a switch or a reset re-points that body (ADR-0021 section 15, item 19). With no body running they are refused `pane-not-driven`.``; (2) in `holler pane reset PANE`, replace `` `--first` on a Claude Code pane is refused `pane-not-driven` until a body drives it (#850).`` with `` `--first` on a Claude Code pane goes through its body as on any pane, and is refused `pane-not-driven` with none.``; (3) in "The roster", replace `a Claude Code pane has none until a body drives its session ([#850](...)), and `pane list` shows it meanwhile` with `a Claude Code pane has one once its body runs, and `pane list` shows it always`; (4) delete the section 12 bullet ``a body that lets `holler say --pane NAME TEXT` reach a Claude Code pane (#850);``. `docs_pane_control_test` pins the guide to the code's tables and stays green.

**`CHANGELOG.md`**, `## [Unreleased]`, `### Enhancements`: one new bullet, linking #850, that says a Claude Code pane has a body (the row, no endpoint, delivery by the adapter, no reply text, no in-place interrupt) and that a switch or a reset re-points it; and in the #831 bullet (E27) the sentence from `` `holler say --pane NAME TEXT` does not reach a Claude Code pane yet`` to `(after [#839](...))` is replaced by `` `holler say --pane NAME TEXT` reaches a Claude Code pane through its body (#850).`` #839's own bullet is history and stays: `docs_pane_control_test` looks for the entry that links #839 and #828.

**The stale list: every present-tense statement that #850 makes false.** `docs_claude_body_test` (Acceptance 8) reads each file below as #839's `normalised` reads a file (comment markers dropped, whitespace as one space) and fails on a phrase listed for it. Each phrase is replaced by a pointer to ADR-0021 item 9 or 19 or to ADR 0014 points 7 to 11, restating nothing; the phrases A3 and A4 and the three edits above replace are in the list too.

| File | Phrases (E20, E21, E41, E42, E52; the lines are as on `main` at `ef881fc`) |
|---|---|
| `docs/adr/ADR-0021.md` | `it has no body, item 18`; `until a body drives its session`; `until a body drives it (#850)`; `no body drives a Claude Code pane until #850`; `until a body can drive it (#850)`; `Only the HTTP attach driver`; `The body driver for `say` (#850)`; `A body for a Claude Code pane, so that` |
| `docs/pane-control.md`, `CHANGELOG.md` | `does not reach a Claude Code pane yet`; `until a body drives it (#850)`; `until a body drives its session`; `reach a Claude Code pane (#850)` |
| `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs` | `no body exists to run the text after it until #850` (the doc of `INTERRUPT_TEXT_UNSUPPORTED`); `has no body for the hub to reach` (the module docs, `say_cmd.rs` lines 19 to 24 and `interrupt_cmd.rs`'s) |
| `crates/holler-pane/src/tx_switch.rs`, `tx_switch/plan.rs` | `no body drives such a pane until #850`; `No body drives a single process until #850` (the module doc and the comment of the single-process branch) |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs` | `which no body drives until #850`; `has no body until #850`; `observes no DRIVEN` |
| `crates/holler-body/src/session_manager.rs` | `Only the HTTP attach driver` (the doc of `SessionManager::rebind`, which now serves two drivers) |
| `crates/holler-cli/tests/pane_claude/{driven,lifecycle,rig,main,mixed}.rs` | `until a body drives a Claude Code pane (#850)`; `no body drives a Claude Code pane until #850`; `naming #839 and #850`; `needs #839 and #850`; `no body at all until #850` (`mixed.rs` also loses the ignored `say` step, and `main.rs` stops naming it) |

`config.rs`'s module grammar comment (E3) gains the Claude attach row by pointing at ADR 0014 point 7, restating nothing.

## Plan

Ordered work; T writes each test and sees it RED before F writes code, then GREEN.

- **P1. Contract first** (A1 to A6), one commit.
- **P2. The row.** `SessionConfig::attaches_to_claude_code`, the validation (the row rule), the parse tests (Acceptance 1). `holler-body`'s existing `config_test.rs` stays green unedited.
- **P3. The driver.** `holler-body` gains `holler-pane`; `claude_attach_driver` over the port (the delivery rule, the turn rule, the cancel rule); the `Driver` variant; `SessionManager::start_with` and `connection::run` carry the port; the support probe (Acceptance 2, 3).
- **P4. The CLI.** `body_cmd::run` builds the Claude harness only when a Claude row exists, hands it to `connection::run`, and warns once and passes none when it cannot be built (a configuration error, never a silent skip: the rows then fail their attach, each logging `attach_failed`).
- **P5. The drive rule.** A single process's plan finds its body (`find_driver` with an empty server list, `tx_switch/plan.rs`, E41), its switch and reset drive that body after the relaunch (`drive`, E22) and record DRIVEN by a second compare-and-swap, reconcile records DRIVEN for a single process (`reconcile.rs`, `reconcile/observe.rs`, `observe/process.rs`), the module docs say so (the module doc's sentence on port calls counts the drive step's), and the three tests #839 wrote for the no-body era are rewritten (Acceptance 6).
- **P6. The scenario and the end-to-end tests** (Acceptance 4, 5); the ignored `say` test is deleted.
- **P7. Docs, comments, CHANGELOG** as A6 says; the docs test (Acceptance 8).

## Where the code goes

Sizes were measured with `wc -l` on `main` at `ef881fc` (E40); `scripts/lint.sh` fails a file at 900 and warns at 600 (E39). A touched file above about 800 takes its new code in a sibling.

| File | Now | What | Rule |
|---|---|---|---|
| `crates/holler-body/Cargo.toml` | | `holler-pane` as a dependency; `holler-pane-testkit` as a dev-dependency | no feature list, so no consumer comment is needed |
| new `crates/holler-body/src/claude_attach_driver.rs` | 0 | `ClaudeAttachDriver`: `attach`, `prompt`, `cancel`, `answer`, `pending`, `status`, `session_id`, `shutdown`, over `Arc<dyn HarnessPort>`; the same public method shapes as `HttpAttachDriver` and the same event types (F3). It has no `rebind` method, as `HttpAttachDriver` has none: the session task's `rebind` calls `attach` (E53) | under 400 |
| new `crates/holler-body/src/claude_attach_driver/turn.rs` | 0 | the poll loop of the turn rule and its constants (`POLL`, `START_WAIT`, `OP_BOUND`) | under 300 |
| `crates/holler-body/src/lib.rs` | 30 | `pub mod claude_attach_driver;` | +1 |
| `crates/holler-body/src/config.rs` | 668 | `attaches_to_claude_code`; the validation arm of the row rule goes in a new `config/claude_row.rs` (under 120), called from the attach branch (E3) | config.rs +6 |
| `crates/holler-body/src/session_manager/driver.rs` | 78 | the `ClaudeAttach` variant, one arm in each of the six methods, and `spawn_or_attach` takes the optional port (E4) | about +25 |
| `crates/holler-body/src/session_manager.rs` | 441 | `start_with(registry, claude)`, `start` calls it with `None`; the port reaches each task; the doc of `rebind` stops saying only the HTTP driver re-points | about +12 |
| `crates/holler-body/src/session_manager/task.rs` | 794 | `Inner` holds the optional port, the two `spawn_or_attach` calls pass it, and `rebind` (E53) attaches through `Driver::spawn_or_attach` in place of `HttpAttachDriver::attach` and stores the `Driver` it returns; **wiring only, no new logic here** | at most +10: it ends a few lines past 800 and well under the guard's 900; anything more goes in a sibling in `task/`, which already holds `rebind_tests.rs` |
| `crates/holler-body/src/connection.rs` | 874 | `run` takes the optional port and passes it to `start_with` (E48) | **at most +2**; edit in place |
| `crates/holler-body/src/query.rs` | 380 | the Claude attach arm of `harness_support` (the support rule) | about +15 |
| `crates/holler-cli/src/body_cmd.rs` | 311 | build the harness when any row `attaches_to_claude_code`, pass it, warn once on failure | about +25 |
| `crates/holler-cli/src/pane/wiring.rs` | 508 | `claude_harness` becomes `pub(crate)` (E16) | +0 or +1 |
| `crates/holler-pane/src/tx_switch.rs` | 399 | the drive step for a single process after `relaunched`, calling `drive` (E22), and the module doc's sentences on a single process and on port calls | about +25 (#839 moved the plan out to `tx_switch/plan.rs`) |
| `crates/holler-pane/src/tx_switch/plan.rs` | 305 | the single-process branch of `plan` finds the body (`find_driver` with an empty server list) and gives `check_first` the driver it found, in place of `None` (E41) | about +20 |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs`, `reconcile/observe/process.rs` | 571, 554, 240 | `read_bodies` asks the hub for a single process too, `check_pane` hands `process::check` the `bodies`, and its record write is `record_with` with the DRIVEN found (the `record` helper, which pins `None`, goes) | about +10, +10, +15 (under the guard's 600 warning) |
| `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`, `tests/pane_claude/{main,mixed,lifecycle,rig}.rs` | 588, 205 | comments and module docs (A6's stale list); `mixed.rs` loses the ignored test | about -20 |
| new tests (Acceptance) | | each in a new file, except `tests/pane_claude/driven.rs`, which #839 wrote at 72 lines for the no-body era and Acceptance 6 rewrites and extends (it stays under 400) | `session_manager_test.rs` (875) is **not touched** |
| `crates/holler-cli/Cargo.toml` | | three `[[test]]` entries, `claude_attach_mode_test`, `say_pane_scenario_test` and `docs_claude_body_test`, each with a comment naming #850, beside `attach_mode_test`'s (E38; F11) | +about 20 |
| docs | | A1 to A6 | |

New code never goes into `task.rs` or `connection.rs` beyond the lines above. The `ClaudeAttachDriver` does not copy `HttpAttachDriver`'s SSE and poll loop (it has none); it reuses `DriverEvent`, `DriverEventStream::new`, `DriverError`, `Status` and `StopReason` (E9).

## Reuse

Extend these; write a new one only where "Where the code goes" says so. `Driver` and `session_manager`'s task, queue, presence and crash isolation (F3), unchanged; the public shape of `HttpAttachDriver` (E8's `cancel`, the answer and pending shapes); `DriverEventStream::new` (`pub(crate)`, E51); `ClaudeHarness` through `HarnessPort`, built by `wiring::claude_harness` (E16), never a second construction; `holler_pane::HarnessEndpoint::Process` and `PaneName::parse` for the endpoint (E30; a body session name has the pane name grammar); `holler_pane::pane::CLAUDE_CODE_COMMAND` for the program name the support probe looks up (the one place that name is written); `query::resolvable` for the PATH lookup; `FakeClaude` and `fault::FaultSwitch<ClaudeOp>` for the driver's tests (E49); the `attach_mode_test` rig, which includes the fake OpenCode server by `#[path]` (E37); `support::{Hub, Body, StateDir, wait_for}`; `FakeHub` (`with_body`, `set_state`, `hold`, `release`, `fail_send`, `faults`, `log`, `prompts`) and the `Rig` of `pane_claude/rig.rs`, which already holds one (E41); `holler_pane_testkit::fixture::session_id` is `ses_`-shaped and serves an OpenCode pane's fixtures, while a Claude pane's id is a UUID (`FakeClaude::new_session_id`).

## Acceptance (each is a test)

Test names are the preparer's and may be sharpened by T; each file is new unless it says otherwise. `holler-cli` targets need a `[[test]]` entry (E47, E38); `holler-body` targets are discovered.

1. **The row** (`crates/holler-body/tests/claude_row_test.rs`): `a_claude_attach_row_with_a_uuid_session_id_parses`; `a_claude_attach_row_with_an_endpoint_is_refused_naming_endpoint`; `..._with_interrupt_http_is_refused`; `..._without_a_session_id_is_refused`; `a_session_id_that_is_not_a_lowercase_uuid_is_refused` (a name-shaped id, a `ses_` id, an upper-case UUID); `a_claude_uuid_never_parses_as_a_routable_name` (F10); `a_claude_spawn_row_is_still_the_acp_bridge` and `an_opencode_attach_row_is_unchanged` (the existing `config_test.rs` also passes unedited); `only_one_function_compares_the_harness_id_with_claude` (a source scan of `crates/holler-body/src` with comments stripped and each file cut at its first `#[cfg(test)]`).
2. **The driver** (`crates/holler-body/tests/claude_attach_driver_test.rs`, over `FakeClaude`): `attach_fails_closed_when_no_process_holds_the_session`; `a_prompt_writes_one_frame_with_the_text_and_streams_no_text`; `a_turn_ends_end_turn_when_the_process_is_idle_again_after_busy`; `a_turn_too_short_to_see_ends_after_the_start_wait`; `a_relaunch_between_prompts_needs_nothing_from_the_driver`; `a_relaunch_during_a_turn_ends_it_cancelled`; `a_vanished_process_ends_the_turn_error`; `a_refused_delivery_ends_the_turn_error_after_one_chunk_that_quotes_no_process_output`; `a_permission_wait_is_input_required_with_nothing_pending`; `answer_is_always_refused`; `cancel_in_flight_is_refused_naming_the_pane_verb_and_cancel_when_idle_applies`; `replace_is_refused_in_flight_and_plain_when_idle`; `the_driver_never_calls_login_status` (the fake's call log has no `harness.login_status`, I7); `a_stalled_port_call_is_bounded` (a `stall` fault returns within `OP_BOUND`, I5).
3. **The session manager** (`crates/holler-body/tests/claude_session_test.rs`): the row is advertised `mode: attach`, `harness: claude`, `harness_session_id` the row's; it is omitted until attached; a second prompt queues and runs after the first; after a turn that ended `error` the next prompt re-attaches; `shutdown` touches nothing (the fake process still runs and nothing was written); two rows on one body share the one port and each finds its own process; a body started with no port refuses the row's attach naming the missing configuration; `rebind_to_a_held_session_swaps_and_advertises_it_and_to_an_unheld_one_is_refused_keeping_the_binding` (the session task's `rebind` over a Claude row, E53; the rebind rule; a row mid-turn is refused as any row is).
4. **The hub path, a typed address** (`crates/holler-cli/tests/claude_attach_mode_test.rs`, `[[test]]` `claude_attach_mode_test`; a real hub, a real `holler body run`, a fake `claude` program named by `HOLLER_CLAUDE_BIN` that prints a scripted `agents --json` document and records its argv, and a listener standing for the inbox socket, whose path the test writes to `<state dir>/claude-inbox/<pid>` under mode `0700` and `0600`, as the launch hook would): `say_reaches_a_claude_row_as_one_inbox_frame`; `an_operator_hold_on_the_row_refuses_say_and_writes_nothing` (exit 4); `interrupt_in_flight_is_refused_and_the_turn_runs_on`; `the_row_shows_in_the_roster_as_claude_attach_with_its_uuid`; `a_row_whose_session_no_process_holds_stays_off_the_roster`; `body_support_claude_is_a_path_lookup` (A5); `no_login_is_read_or_printed` (the fake program was run with `agents --json` and nothing else, and no credential variable name or value reached stdout, stderr or a log). The pane gate for a typed address is Acceptance 5.
5. **#831's `say` step** (`crates/holler-cli/tests/say_pane_scenario_test.rs`, `[[test]]` `say_pane_scenario_test`): one hub, two bodies (an OpenCode row on `attach_mode_test`'s fake server, a Claude row on the fake program and listener), two pane records written through the store client, each `session_of_record` the id its body advertises. The file rests on #839's join and `first_refusal` (E17): the OpenCode pane's fixtures are `ses_`-shaped and the Claude pane's is a UUID. `holler say --pane NAME TEXT` is delivered to each (the fake server saw `prompt_async`; the listener read one frame); an operator hold on a row refuses `say --pane` with exit 4 and writes nothing; after a row's body stops, `holler say --pane NAME TEXT` is refused `pane-not-driven` (exit 3, the hub's offline marker, E44) and writes nothing, for both harnesses; a parked pane's record refuses a bare `holler say <label>/<name>` at the hub's gate (`pane-parked`) and writes nothing, for both harnesses; after the park is lifted it is delivered. The ignored `say_delivers_a_message_to_each_pane_of_the_scenario` in `pane_claude/mixed.rs` is deleted.
6. **The drive rule** (`crates/holler-cli/tests/pane_claude/driven.rs`, which #839 wrote at 72 lines and #850 extends; its `Rig` already holds a `FakeHub`, E41). #839's two tests there pin the no-body era, with a body that advertises the pane's session of record and that nothing asks: `switch_and_reset_of_a_claude_pane_ask_the_hub_nothing_and_keep_driven` and `doctor_leaves_a_claude_pane_s_driven_as_stored`; both are rewritten (below), and `lifecycle.rs`'s `reset_first_on_a_claude_pane_is_not_driven_and_stops_nothing` stays as the no-body case of `reset --first`. The tests: `a_claude_switch_relaunches_then_rebinds_then_records_driven_in_a_second_write` (the call log: the hub's `advertised`, the store's list, [`prompt(Check)`], the relaunch, `rebind`, `cas_put` of DRIVEN, [`prompt(Send)`]; `HarnessPort::send_prompt` never called); `a_claude_reset_first_is_delivered_through_the_hub_after_the_rebind`; `a_claude_pane_with_no_body_switches_and_resets_without_a_drive_step` (the first rewritten test: no body is advertised, nothing is asked beyond `advertised`, DRIVEN stays as stored); `a_failed_rebind_after_the_relaunch_is_reported_acted_with_the_reconcile_step` (and the finding's remedy, `holler pane switch PANE <session of record>`, re-points the body; for a single process it relaunches the process on the session it holds first); `an_interrupt_moves_no_body_and_writes_no_driven`; `doctor_records_driven_for_a_claude_pane_and_reports_a_body_that_is_behind` (the second rewritten test; `FakeClaude::clear` moves SHOWN); `a_lost_second_write_is_healed_by_the_next_doctor_pass` (the relaunch stored the session of record, so the body now on it is the pane's by `driver_of`'s first rule and the pass records DRIVEN with no finding); and in `crates/holler-pane/tests/driven_single_process_test.rs` (new; `driven_test.rs` is #839's), `driver_of` over a single process (an empty server list: by the record, by the stored DRIVEN, by the target; none).
7. **No hub and no wire change.** `git diff --stat origin/main...HEAD` lists no file under `crates/holler-hub/src` or `crates/holler-proto/src` and no golden file; `scripts/golden-diff-summary.sh` lists none; `Code::ALL` still has 17 entries (the existing count test).
8. **Docs** (`crates/holler-cli/tests/docs_claude_body_test.rs`, `[[test]]` `docs_claude_body_test`): the files and phrases of A6's stale list, read as #839's `normalised` reads a file, hold none of those phrases; and `docs_pane_control_test` passes with the amended guide.
9. **Guards.** `bash scripts/lint.sh`; `cargo clippy --workspace --all-targets -- -D warnings` (as `.github/workflows/ci.yml` runs it); `cargo machete`; `bash scripts/changelog-check.sh`; the CI test command (`test.unit.command` in `.aftersight/pipeline.config.json`); `cargo test -p holler-cli --test wizard_scripts` (it pins the wizard's skill text and skips `docs/handoffs/`, so this brief is not scanned; it must pass).

## Blast radius

Crates: `holler-body` (driver, config row, session manager, query), `holler-cli` (`body_cmd`, wiring visibility, comments, tests), `holler-pane` (the plan's single-process branch, the drive step and reconcile), and docs. **Not touched:** `holler-hub`, `holler-proto`, `holler-adapter-claude`, `holler-pane-testkit` (unless one helper of `FakeClaude` is needed to hold a status, and then no conformance case changes). The existing `config_test.rs`, `http_attach_driver_test` and `session_manager_test.rs` stay green unedited; the fake OpenCode server (`http_attach_driver_test/fake_server.rs`) is reused unedited. `connection::run`'s signature changes and its one caller is `body_cmd` (E48). `holler-cli`'s `hold_single_path_test` stays green unedited: its scan of `.send_prompt(` reads `holler-pane` and `holler-cli` only, and the driver's call is in `holler-body`; `holler-hub`'s and `holler-proto`'s tests are not touched. No live fleet, no real Herdr, no running pane, no real Claude Code or login is used: the program that stands for `claude` is a script in a temporary directory, and the opt-in real test of the adapter is not extended.

## Clashes with stories in flight

State on 2026-10-12 against `main` at `ef881fc`: **no other story is in flight**, so there is no source clash and no merge order to keep. Merged, so the base: #831, #835 (PR #860), #839 (PR #863), #853 (PR #861), #665, #648, #667, #803 (PR #866), #647 part 2 (PR #872), #846 (PR #868), #696 (PR #874), #844 (PR #870), #845 (PR #873) and #871. The ADR's "Decisions taken" ends at 49, so this story's is 50 (A4). The one semantic leftover is the merged wizard's statements that a Claude pane has no body row (E46); N1 names it.

## What was re-checked after #839 and #835 merged

Each row is a claim this brief made while #839 and #835 were unmerged, with what the merged code, ADRs and docs showed on 2026-10-12 (`main` at `46b3549`). The rows name no rule; the rules are in "Where each rule lives".

| Row | The claim | What the merged code and text showed |
|---|---|---|
| **R1** (#839) | A4's edits name sentences #839 rewrites (E24, E25) | Item 18 changed three clauses (`until #839, through reset --first`, `reached ... through`, `since #839 moved it ... refused pane-not-driven until #850`) and kept the sentences A4 replaces; the "Deferred" bullet about DRIVEN went and a new bullet, `A body for a Claude Code pane, so that ... #850`, took its place; "Decisions taken" ended at 47 when #839 merged (decisions 34 to 47 are #839's); #803 has since taken 48 and #647 part 2 49, so the next free number is 50. Four present-tense statements that name #850 as future are new (E42, E52): A4 and A6 name each. |
| **R2** (#839) | ADR 0014 point 11 and the `session/rebind` text extend #839's, which said the Claude driver implements the request "as a relaunch of the process" | The merged ADR 0014 says only that a hub may ask an attach body to re-point a session (the Consequences bullet, E1); ADR-0021 section 11 point 2 says "a body for another harness implements it in its own driver" (E42); `docs/protocol/v2.md` names no harness (E35). The phrase "a relaunch of the process" survives only in #839's own brief (`docs/handoffs/839-brief.md`), which is not normative, so point 11 and the merged text agree: the relaunch is the engine's and has finished before the request arrives. The merged session task attaches an HTTP driver and maps every failed attach to `-32602` with the reason `session not found on the endpoint` and a late one to `-32004` with `endpoint did not answer` (E53), so point 11 uses those and the task attaches through `Driver::spawn_or_attach`; D11 holds. |
| **R3** (#839) | The drive rule replaces #839's D3 item 8 for a single process (E41) | `HubPort` has three required methods: `advertised() -> Vec<Advertised>`, `rebind(&Advertised, &str) -> String`, `prompt(&str, &str, PromptMode)`. `driver_of(pane, advertised, server_sessions: &[String], others: &[Pane], target: Option<&str>)` returns `Driver::{None, One(&Advertised), Ambiguous(Vec), Unproven(Vec)}`, and `elsewhere(advertised, driver, target)` returns the other bodies on a target. `switch` is `plan` (now in `tx_switch/plan.rs`, returning `Plan { record, driver }`), then `switch_in_place` or `relaunched(ports, request, &record)`; a single process's plan takes `driver: None` and calls `check_first` with none (E41). `tx_launch` is unchanged by #839: `relaunch_recorded` still stores the record itself and keeps the stored DRIVEN. The module doc counts at most thirteen port calls for an in-place run and puts a relaunch under `tx_launch`'s budget; the drive step adds `hub.advertised`, the store's list, `hub.rebind`, one `cas_put` and, with `--first`, two `hub.prompt` calls, each bounded by I5 on its own. Reconcile skips a single process in three places: `read_bodies` asks the hub only when a `ServerAndTui` pane is in scope, `check_pane` hands a single process to `process::check` with no `bodies`, and `record` pins DRIVEN to `None`. Nothing in the merged code or text forbids the drive step; `drive` is reusable as it is. **D5 still fits.** |
| **R4** (#839) | Acceptance 5 and 6 build on `FakeHub`, `fixture::session_id` and `first_refusal` | All exist: `FakeHub::{with_body, set_state, hold, release, fail_send, faults, bodies, log, prompts}`; `fixture::session_id(n)` is `ses_`-shaped, so a Claude pane's fixtures use a UUID; `first_refusal(&[Pane], &str) -> Option<(&Pane, PromptRefusal)>` with `PANE_PARKED`, `PANE_UNHEALTHY` and `PANE_SHOWN_DRIVEN_MISMATCH` in `holler_pane::gate`. `pane_claude/rig.rs` already holds a `FakeHub`, and `pane_claude/driven.rs` and `lifecycle.rs` pin the no-body era (Acceptance 6). |
| **R5** (#839) | D7: the gate is keyed on the advertised id, and a typed bare id is refused at the CLI | Confirmed (E17): `HoldGate.harness_session` carries the advertised string to `pane_gate`, which calls `first_refusal`; a UUID never parses as a routable name (F10), so `route_target_by` refuses it as a typed SESSION and the hub resolves it only for `--pane`. |
| **R6** (#839) | E41 to E44 quote #839's brief at `e23ed68` | Re-cut from merged code and ADR text. |
| **R7** (#839) | A `say --pane` to a Claude pane whose body is offline is `pane-not-driven` | Confirmed (E44): `-32004` with `data.reason: "offline"`, and `-32003` for a body that never advertised, both classify as `NotDriven` and print `pane-not-driven`, exit 3. Acceptance 5 now has the case. |
| **R8** (#835) | `HostPort` forwards are #835's; `wiring.rs`, the ADR, the guide and the CHANGELOG clash | #835 is merged (E45); this story adds no method to a port, and every `HubPort` method is required, as #835's rule asks. |
| **R9** (#803, merged) | #803's D13 says a Claude pane has no row | #803 merged (PR #866): the wizard builds one local body with attach rows for OpenCode panes only, and its skill and guide say a Claude pane has no body row until #850 (E46). This story does not edit the wizard; the follow-up is N1. |
| **R10** | The `.send_prompt(` scan, and the sizes | `hold_single_path_test` allows the call only in the two pass-throughs of `holler-pane` and `holler-cli` (`wiring/bind.rs`, `ports/router.rs`); `tx_switch.rs`'s call is gone; the driver's call is in `holler-body`, outside the scan. Sizes are E40. |
| **R11** (refresh to `ef881fc`) | The excerpts, the line numbers and the sizes | Every E excerpt was re-run: the lines of the adapter, the ADR, the guide, the CHANGELOG, the README, `mixed.rs`, `wiring.rs` and the test kit's `claude.rs` moved and are re-cut (E10, E11, E15, E21 to E27, E36, E40, E42, E44, E45, E49, E50, E52); ADR-0021 section 5's runner bullet now names `holler_pane::bounded` (#696) and the "Not built here" paragraph lost its wizard clause (E15, E24), and neither changes an edit of A1 or A4. `holler-body`, `holler-hub`, `holler-proto`, `body_cmd.rs`, `say_cmd.rs`, `interrupt_cmd.rs`, ADR 0014 and `docs/protocol/v2.md` are unchanged. `HarnessPort` gained `agent_view` (#647 part 2) and `HostPort` `attached_clients`; the driver calls neither, and `FakeClaude` implements both. Reconcile's three single-process skips (R3) hold. The runners' `exec.rs` files (#696) and the old `LinkedHost` fakes (#846) are not cited by this brief. |

## Depends on

- **#831** (merged, `b76f4d2`): the adapter, `HarnessEndpoint`, `Shape`, `AbortStyle`, `FakeClaude`, `relaunch_recorded`.
- **#839** (merged, PR #863, `46b3549`; the operator's decision: launch after it merges): the join of a body to its pane by the advertised id, the hub's re-keyed gate, `session/rebind`, `HubPort`, `driver_of`.
- #803 (PR #866), #647 part 2 (PR #872), #835 and #853 are merged; nothing is open that this story waits for.

## Pipeline

`rigor: in-session`: the pipeline's own agents review, no outside model. Phases: `O -> A -> T(author/RED) -> F -> T(verify/GREEN) -> A(anti-duplication) -> S`; Holler has no UI, so D and U never run (`uiSurface: false`). The branch is `issue-850-implementation`, in a fresh worktree cut from `origin/main` after #839 has merged, with the `.env` copy and the release link the repository's CLAUDE.md describes. F runs on `claude-opus-5-5` at `max`. Phase A's plan check reviews this brief against the dimensions of `docs/agent-overlays/architecture-reviewer.md`: choke points (D7), wire and protocol (D11), ADRs (A1 to A5), state and persistence (none: the row is config; the drive rule's second write is the existing compare-and-swap), concurrency (every port call in `spawn_blocking` under one bound, D3), identity and secrets (I7: Acceptance 2 and 4), size (Where the code goes), public repository (below).

## Rules for this story

See the epic (#633): public repository, so no personal host, tailnet or account names, no addresses and no secret anywhere (every example here is a placeholder: a UUID, the pane name `c1r1`); the amend-first rule; `rigor: in-session`; no story touches a live fleet, a real Herdr session, a running pane or a real Claude Code or OpenCode session. **A subscription login is never read, copied or logged (I7):** A2 point 8 says how this driver keeps it, Acceptance 2 and 4 pin it, and the fake program of the tests prints no credential. Commit once a change is complete and verified; each commit carries the `Co-Authored-By` trailer the hooks add.

## Questions only the operator can answer

None blocks the run: each was decided and is listed so the operator can overturn it.

- **Q1. Should #850 also change the roster's join (`pane_label`) so a row joins its pane by the advertised id?** Decided no (D8): it is #648's display rule, pinned by its tests, and #839 leaves it (E43). With #803's one local body no row joins any pane under it, for either harness. If the operator wants the pane columns on those rows, the change is one function and a refresh of #648's acceptance 10, and it is a separate story.
- **Q2. Should `interrupt --pane NAME TEXT` become a relaunch followed by a delivery?** Decided no (D4), so the redirect text is said after the interrupt.
- **Q3. Who writes the Claude row in the release?** #803 has merged and its wizard gives a Claude pane no row (E46), so until a follow-up story the operator writes the row by hand from the README recipe (A6). Decided: this story does not edit the wizard (but see N1).
- **Q4. Is a default-run acceptance enough for the issue's item 4?** Decided yes (D10): the opt-in scratch scenario runs real tools and starts no body, and the real-binary test of the delivery frame already exists in the adapter.

## Not verified

- **Real Claude Code.** Nothing here runs it. That a message written to a *relaunched* process's inbox socket is read once its launch hook has written the socket file is the adapter's opt-in real test (`#[ignore]`), not this story's; the driver only calls the adapter.
- **The poll interval (500 ms) and the start wait (5 s) of the turn rule** are the preparer's choices against the spike's 100 to 300 ms between an event and a status change (E34); no real turn was timed.
- **That `claude agents --json` reports the resumed session id after `--resume`** is the spike's verified finding ("new process, same id"), which the drive rule and the interrupt rule depend on; it is not re-run here.
- **That Claude Code accepts only a UUID as `--session-id`.** Holler mints UUIDs and a switch needs a transcript, whose file name is the id, so a non-UUID target cannot reach the row; the row's UUID rule is the strictest form and is Holler's choice.
- **The merged wizard** (#803) is read, not run; what it says about a Claude pane is E46 and N1.
- **Whether the CI runner has `bash` and `sh` for the fake `claude` program** of Acceptance 4 and 5: the other body tests already run `stub-acp` and shell helpers there; the program is a `#!/bin/sh` script.

## AI assistance

Prepared by an AI agent (Claude Code), at the operator's direction, on 2026-10-12, from the issue, the repository at `b76f4d2` and the read-only briefs and branches named above, and refreshed the same day against `main` at `46b3549` (#839 and #835 merged) and again against `main` at `ef881fc` (#803, #647 part 2, #696 and others merged). The implementation runs in the repository's coding pipeline, whose agents are AI agents; the operator reviews at the plan check and at the merge, and the PR carries the disclosure `CONTRIBUTING.md` asks for.

## Evidence (verbatim)

Every entry was cut by the command printed above it, run from the repository root of the worktree this brief was written in (a sibling worktree is read through `../<name>`), on `main` at `ef881fc` unless the heading names a commit; nothing was retyped. A command that runs a pager-free `sed -n` prints the lines named, so a reader can see what moved by re-running it. Each command was re-run after the last edit of this brief and its output compared with the block below.

### E1: ADR 0014's six points and its Consequences: what an attach row is today

Command: `sed -n '11,18p;27,31p' docs/adr/ADR-0014.md`

```text
A body config row may **attach** to a harness session that is already running under something else — a terminal-multiplexer pane, an operator's own `opencode serve` — instead of **spawning** and owning it. The two legacy repos wrote this as a matched pair of ADRs (one per side of the old split); this repo folds them into one, because in the single-binary architecture there is only one process making the choice.

1. **Spawn remains the default.** A config row with `mode = "spawn"` or no `mode` at all behaves exactly as [ADR 0012](ADR-0012.md) describes: the body execs `command` as an ACP v2 subprocess it fully owns, and `detach` or process exit tears that child down. Nothing about spawn changes.
2. **Attach is a first-class second transport, not a fallback.** A config row with `mode = "attach"` names an `endpoint` and a `session_id`; the body drives that session over its existing HTTP/ACP control surface and is **not its parent**. `session/new` is never called in attach mode — if `session_id` is missing, or the endpoint 404s on a lookup for it, the body fails closed with a precise error rather than creating a consolation session.
3. **Teardown is mode-aware, and attach's teardown does the least, not the most.** `detach`, a dropped hub connection, or this process exiting must never kill or replace an attached session — only the local control channel (the body's own connection into the endpoint) closes. This is the entire point of the mode: two things cannot both parent one process, so whichever one the operator is already watching (a Herdr pane, a terminal) must be the side that owns it, and Holler's body must be able to leave without taking it down.
4. **Holler still does not become a terminal multiplexer.** Attach never types into a PTY and never streams pane bytes on the Holler wire (`herdr pane send-keys` or equivalent stuffing is explicitly out of scope). Prompt and interrupt go through the same ACP/HTTP control surface an attached endpoint already exposes — the same surface a spawn-mode body uses as its interrupt fallback — never through the terminal the human is watching.
5. **Addressing is unaffected — this is the point of [ADR 0005](ADR-0005.md)/[ADR 0011](ADR-0011.md).** The Holler session name (e.g. `io/alpha`) is the circuit address regardless of mode; a harness-side session id (`ses_…`) is a **body-side locator**, carried on presence so the roster can distinguish attach from spawn, but it never changes who `say io/alpha` talks to. Since [ADR 0021](ADR-0021.md) section 11 (#839) the hub also accepts the `ses_…` id as an address, for its own `--pane` resolution and pane gate (the `holler` CLI's `SESSION` stays a name), and a `session/rebind` can change it; the name is still the identity.
6. **Presence carries `mode` and `harness_session_id` as optional fields**, not a protocol version bump — ADR 0004's envelope already ignores unknown keys, and these two are additive to the presence document the roster (ADR 0010) already consumes.
- Session-manager teardown branches on mode: spawn tears down its ACP child; attach drops only its own control channel and returns, leaving the harness process running exactly as it found it.
- Single-instance locking on `holler body run` still applies per body process; two bodies attaching the *same* `session_id` concurrently is undefined and the second one's lock attempt fails, same as today.
- `roster`/`status` surface `mode` and the harness session id so an operator can tell attach apart from spawn without it changing how they address the session. (Extended by [ADR 0021](ADR-0021.md) section 11 (#839): the hub also resolves that id as an address for `--pane`; a person's `SESSION` is still a name.)
- Discovering an attachable session automatically (rather than an operator pasting `endpoint` + `session_id`) is explicitly out of scope for v1 attach and remains a follow-on.
- Extended by [ADR 0021](ADR-0021.md) section 11 (#839): a hub may ask an attach-mode body to re-point one of its sessions to another harness session on the same endpoint (`session/rebind`). The session's name, and so its address, does not change.
```

### E2: ADR 0012: a row has two shapes; harness ids stay plain strings

Command: `sed -n '15p;25p' docs/adr/ADR-0012.md`

```text
3. **Out-of-tree adapters remain the escape hatch for a harness that does not speak ACP.** If a harness needs translation, that translation is someone else's binary on `command` (or behind an attach `endpoint`) — a separate repo with its own release cadence — never code `dlopen`ed into the Holler process. Holler only ever `exec`s a command or dials an HTTP endpoint; it never loads a harness's code into its own address space.
- `holler-body`'s harness ids remain plain strings end to end — no `enum Harness { OpenCode, Claude, … }` anywhere in the wire or config types — because a compiled-in enum is exactly the "Holler release per harness" cost this ADR refuses to pay.
```

### E3: The body's config grammar and the attach branch of its validation

Command: `sed -n '12,18p;316,340p' crates/holler-body/src/config.rs`

```text
//! mode = "spawn"                       # "spawn" (default) | "attach"
//! command = ["opencode", "acp"]        # spawn only; argv, no shell
//! cwd = "/path"                        # optional, spawn only
//! env = { KEY = "value" }              # optional, spawn only
//! interrupt = "acp"                    # optional: "acp" (default) | "http"; http needs endpoint
//! endpoint = "http://127.0.0.1:4096"   # attach: required; spawn: optional (HTTP interrupt fallback)
//! session_id = "ses_..."               # attach: required
            SessionMode::Attach => {
                if r.endpoint.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "endpoint",
                        reason: "attach mode requires `endpoint`".to_string(),
                    });
                }
                if r.session_id.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "session_id",
                        reason: "attach mode requires `session_id`".to_string(),
                    });
                }
                if command.take().is_some() {
                    warnings.push(format!(
                        "session {:?} (row {row}): attach mode ignores `command` — never spawned",
                        r.name
                    ));
                }
            }
        }
```

### E4: The body's one driver enum and the one place a driver is chosen

Command: `sed -n '12,34p' crates/holler-body/src/session_manager/driver.rs`

```text
use holler_proto::docs::PendingItem;

use crate::acp_driver::{AcpDriver, DriverError, DriverEventStream, StopReason};
use crate::config::{SessionConfig, SessionMode};
use crate::http_attach_driver::HttpAttachDriver;

/// Either driver, behind one type.
pub(super) enum Driver {
    Acp(AcpDriver),
    HttpAttach(HttpAttachDriver),
}

impl Driver {
    /// Bring up the right driver for `config`'s `mode` — `SessionMode::Spawn`
    /// spawns a fresh harness child ([`AcpDriver::spawn`]); `SessionMode::
    /// Attach` attaches to the already-existing OpenCode session named by
    /// `config.endpoint`/`config.session_id` ([`HttpAttachDriver::attach`]).
    pub(super) async fn spawn_or_attach(config: &SessionConfig) -> Result<Self, DriverError> {
        match config.mode {
            SessionMode::Spawn => AcpDriver::spawn(config).await.map(Driver::Acp),
            SessionMode::Attach => HttpAttachDriver::attach(config).await.map(Driver::HttpAttach),
        }
    }
```

### E5: The session task: a replace cancels first, and a failed cancel refuses it

Command: `sed -n '447,493p' crates/holler-body/src/session_manager/task.rs`

```text
async fn handle_replace(
    inner: &mut Inner,
    text: String,
    reply_tx: oneshot::Sender<PromptOutcome>,
    updates: Option<mpsc::UnboundedSender<String>>,
) {
    match cancel_current(inner).await {
        Some(Err(e)) => {
            let _ = reply_tx.send(PromptOutcome::Error(e));
            return;
        }
        Some(Ok(reason)) if inner.current_reply.is_some() => {
            // Close the cancelled turn out (and fire *its own* caller's
            // reply) without touching the FIFO queue yet — this replacement
            // runs ahead of it, per the issue's own "cancel → run ahead of
            // the queue" contract for `Replace`. `reason` is the driver's
            // real stop reason (issue #238), not a hardcoded `Cancelled` —
            // the turn being replaced may have already settled some other
            // way right as this `Replace` was processed.
            //
            // `publish_idle: false` — a replacement turn always starts right
            // after this (the unconditional `start_turn` call just below), so
            // this session is never actually free; publishing `Idle` here
            // would be a real, externally-observable lie (see
            // `finish_turn_no_dispatch`'s own doc for the race this closes).
            finish_turn_no_dispatch(inner, reason, false).await;
        }
        Some(Ok(_)) | None => {}
    }
    inner.replace_counter += 1;
    let id = format!("replace-{}", inner.replace_counter);
    // `Replace` (`interrupt SESSION TEXT`, issue #191) streams its reply
    // exactly like `say` — `updates` is the caller's own coalescer feed.
    start_turn(inner, id, text, reply_tx, updates).await;
}

/// Cancel the in-flight turn via the driver, if a driver exists at all.
/// `None` means "nothing to cancel" (no driver has ever been spawned for
/// this session yet); `Some(Ok(reason))` means the driver confirmed the turn
/// (if any) is settled, carrying the REAL `StopReason` it observed (issue
/// #238 — never synthesized) — the caller still must check whether a turn
/// was actually in flight (`inner.current_reply.is_some()`) before treating
/// it as a real cancellation to finish out.
async fn cancel_current(inner: &Inner) -> Option<Result<StopReason, String>> {
    let driver = inner.driver.as_ref()?;
    Some(driver.cancel().await.map_err(|e| e.message()))
}
```

### E6: The session task: a cancel, and the crash isolation that drops the driver

Command: `sed -n '371,395p;735,741p' crates/holler-body/src/session_manager/task.rs`

```text
async fn handle_cancel(inner: &mut Inner, reply_tx: oneshot::Sender<Result<(), String>>) {
    match cancel_current(inner).await {
        None => {
            let _ = reply_tx.send(Ok(()));
        }
        Some(Err(e)) => {
            let _ = reply_tx.send(Err(e));
        }
        Some(Ok(reason)) => {
            let _ = reply_tx.send(Ok(()));
            if inner.current_reply.is_some() {
                // Finish the turn with the REAL stop reason the driver
                // reported (issue #238) — never a hardcoded `Cancelled`. The
                // agent may have actually settled the turn some other way
                // (`end_turn`/`max_tokens`/`refusal`/…) right as this cancel
                // was processed; `docs/protocol/v2.md` §6 makes `stopReason`
                // the source of truth, carried verbatim, so inventing one
                // here would violate that contract even though "the caller
                // asked to cancel" — what the caller asked for and what
                // actually happened are not always the same thing.
                finish_turn(inner, reason).await;
            }
        }
    }
}
    inner.turn_started_at_ms = None;
    inner.last_update_at_ms = None;
    // Crash isolation (issue #189's own acceptance item): drop the driver so
    // the *next* prompt respawns a fresh one, never retried automatically.
    if matches!(reason, StopReason::Error) {
        inner.driver = None;
    }
```

### E7: The wire mapping of a refused cancel

Command: `sed -n '144,169p' crates/holler-body/src/prompt_dispatch.rs`

```text
/// Handle one inbound `session/cancel` request.
pub async fn handle_cancel(
    session_manager: Arc<SessionManager>,
    id: String,
    params: Cancel,
    outbound: mpsc::UnboundedSender<WsMessage>,
) {
    let Ok(cid) = CorrelationId::parse(&id) else { return };
    let name = match SessionName::parse(&params.session) {
        Ok(n) => n,
        Err(e) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("bad session name: {e}"));
            return;
        }
    };
    match session_manager.cancel(&name).await {
        Ok(Ok(())) => send_response(&outbound, &cid, &CancelResult { applied: true }),
        Ok(Err(msg)) => send_error(&outbound, &cid, Code::NotConnected, &msg),
        Err(SessionManagerError::UnknownSession) => {
            send_error(&outbound, &cid, Code::UnknownSession, &format!("no such session: {}", params.session));
        }
        Err(SessionManagerError::Gone) => {
            send_error(&outbound, &cid, Code::NotConnected, "the session task is gone");
        }
    }
}
```

### E8: The HTTP attach driver's cancel when nothing is in flight

Command: `sed -n '271,292p' crates/holler-body/src/http_attach_driver.rs`

```text
    /// Sends an HTTP interrupt directly (this driver's *primary* cancel
    /// path — there is no ACP channel to prefer over it) and waits (bounded
    /// by [`CANCEL_TIMEOUT`]) for the attached session's own
    /// `session.idle`/`session.error` event, returning the **real**
    /// [`StopReason`] it resolved with (never a synthesized `Cancelled`
    /// papering over some other real outcome — mirrors
    /// [`crate::acp_driver::AcpDriver::cancel`]'s own contract).
    pub async fn cancel(&self) -> Result<StopReason, DriverError> {
        let (done_tx, done_rx) = oneshot::channel();
        let already_settled = {
            let mut guard = lock(&self.shared);
            if guard.status == Status::Idle && guard.current_events.is_none() {
                Some(guard.last_stop_reason.unwrap_or(StopReason::Cancelled))
            } else {
                guard.interrupt_requested = true;
                guard.awaiting_done = Some(done_tx);
                None
            }
        };
        if let Some(reason) = already_settled {
            return Ok(reason);
        }
```

### E9: The driver event vocabulary a driver streams

Command: `sed -n '164,173p;193,204p' crates/holler-body/src/acp_driver.rs`

```text
/// One streamed event from an in-flight (or just-completed) turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    /// A chunk of the agent's streamed response text.
    Chunk(String),
    /// A foreground-work state transition.
    State(DriverState),
    /// The turn is over.
    Done(StopReason),
}
/// Why a turn ended, 1:1 with ACP v2's `stopReason` wire vocabulary, plus a
/// driver-local `Error` for a crashed/failed turn (never sent by an agent —
/// synthesized locally when the child dies or a cancel times out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    Cancelled,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Error,
}
```

### E10: The adapter's delivery: send_prompt

Command: `sed -n '302,322p' crates/holler-adapter-claude/src/lib.rs`

```text
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let (pane, pid) = at.process()?;
        let entry = self.running(at)?;
        if entry.session != session {
            return Err(PaneError::Refused {
                code: SESSION_MISMATCH,
                message: format!(
                    "the harness process {pid} of {pane} holds another session than the one \
                     named; nothing was delivered"
                ),
            });
        }
        let socket = inbox::socket_of(&self.config.socket_dir, pid)?;
        self.inbox.write_line(&socket, &inbox::frame(text))?;
        self.watch_held(pane, pid, &entry.status)
    }
```

### E11: The adapter's process lookup and activity

Command: `sed -n '324,330p;345,351p' crates/holler-adapter-claude/src/lib.rs`

```text
    fn session_activity(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
    ) -> Result<Activity, PaneError> {
        agents::activity(&self.running(at)?)
    }
    fn locate_process(&self, _name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        Ok(self
            .entries()?
            .into_iter()
            .find(|entry| entry.session == session)
            .map(|entry| entry.pid))
    }
```

### E12: What claude agents --json says and how an entry maps to an Activity

Command: `sed -n '27,35p;59,85p' crates/holler-adapter-claude/src/agents.rs`

```text
/// One process `claude agents --json` lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) pid: u32,
    pub(crate) session: String,
    pub(crate) status: String,
    pub(crate) waiting_for: Option<String>,
}

/// What the process of `entry` is doing (item 8): `idle`, `busy`, `waiting` on `input needed`
/// (a question) or on anything else a person answers (a permission prompt, a sandbox or a
/// worker request). A `waiting` on `dialog open` is [`HARNESS_DIALOG_OPEN`], and a status the
/// adapter does not know is `unavailable`: neither is ever taken for idle.
pub(crate) fn activity(entry: &Entry) -> Result<Activity, PaneError> {
    match (entry.status.as_str(), entry.waiting_for.as_deref()) {
        ("idle", _) => Ok(Activity::Idle),
        ("busy", _) => Ok(Activity::Busy),
        ("waiting", Some(INPUT_NEEDED)) => Ok(Activity::HoldingQuestion),
        ("waiting", Some(DIALOG_OPEN)) => Err(PaneError::Refused {
            code: HARNESS_DIALOG_OPEN,
            message: format!(
                "the Claude Code process {} waits on a dialog that a person must answer in its \
                 pane (a first-run or trust dialog)",
                entry.pid
            ),
        }),
        ("waiting", _) => Ok(Activity::WaitingPermission),
        _ => Err(PaneError::Unavailable {
            what: format!(
                "claude agents --json reported a status of the process {} that the adapter does \
                 not know",
                entry.pid
            ),
        }),
    }
}
```

### E13: The adapter's dependencies and the body's

Command: `sed -n '14,23p' crates/holler-adapter-claude/Cargo.toml`

```text
[dependencies]
# The port this crate implements and every type it speaks (`HarnessPort`, `HarnessEndpoint`,
# `SessionChoice`, `LoginStatus`, `PaneError`). The adapter depends on nothing else of Holler
# (ADR-0021 section 5).
holler-pane = { path = "../holler-pane" }
# `claude agents --json` and `claude auth status`, read; the `--settings` JSON and the inbox
# frame, written; for `agents`, `argv` and the port methods.
serde_json = { workspace = true }
# The randomness of a fresh session id (a version 4 UUID); for `new_session_id`.
getrandom = { workspace = true }
```

### E14: Every path dependency of the body, the hub and the pane crate

Command: `grep -n '^holler-' crates/holler-body/Cargo.toml crates/holler-hub/Cargo.toml crates/holler-pane/Cargo.toml`

```text
crates/holler-body/Cargo.toml:20:holler-proto = { path = "../holler-proto" }
crates/holler-hub/Cargo.toml:25:holler-proto = { path = "../holler-proto" }
crates/holler-hub/Cargo.toml:29:holler-pane = { path = "../holler-pane" }
crates/holler-hub/Cargo.toml:58:holler-pane-testkit = { path = "../holler-pane-testkit" }
crates/holler-pane/Cargo.toml:21:holler-proto = { path = "../holler-proto" }
```

### E15: ADR-0021 section 5: the crate dependency rules

Command: `sed -n '398,414p' docs/adr/ADR-0021.md`

```text
**Decided: the crate dependency rules.**

- `holler-pane` holds types, traits and the pure transaction engines (`tx_launch.rs` #644, `tx_switch.rs` #645, `tx_apply.rs`
  #664, `reconcile.rs` and `findings.rs` #647, `profile_snapshot.rs` and `profile_diff.rs` #662, `import.rs` #650), which work
  only through the ports. It depends on `serde`, `serde_json` and `holler-proto`, and takes no async runtime. Its one direct
  side effect is the bounded subprocess runner (`holler_pane::bounded`, #696), which runs a process with the standard
  library under a deadline; beside it, the launch engine sleeps between the health polls of a server it started
  (`wait_healthy`, #802), which touches nothing. **Amended by #696, with no contract change:** that runner is the one
  runner of the probe (`run_probe`, #663) and of the host, OpenCode and Claude adapters, which call it instead of keeping
  private copies. Each consumer sets only where the copies differed (what is kept of stdout and stderr, the kill's reach
  and the poll) and maps its typed error to its own texts, so no consumer's behaviour, error text or `ProbeResult`
  changed; the host adapter keeps its own signals to the panes' process groups and its fake-`kill` seam.
- Each adapter crate depends on `holler-pane` and implements one port. `holler-cli` depends on the adapter crates; `holler-hub`
  does not.
- `holler-pane-testkit` (empty today; #638 fills it) may depend on `holler-pane` and `serde_json`, and **must not depend on
  `holler-cli`**: it is a dev-dependency of both the hub and the CLI, so a normal dependency back would make a cycle. Its
  envelope conformance helper (#638) parses the CLI envelope with `serde_json` alone.
```

### E16: The wiring builds the one Claude Code harness from the environment

Command: `sed -n '234,247p' crates/holler-cli/src/pane/wiring.rs`

```text
/// The Claude Code harness (#831): the `claude` binary, Claude Code's configuration directory
/// and the socket directory under the Holler state directory, or the first configuration error
/// of the three, in that order.
fn claude_harness(env: Lookup<'_>) -> Result<Arc<dyn HarnessPort>, PaneError> {
    let bin = env::claude_bin(env)?;
    let config_dir = env::claude_config_dir(env)?;
    let state = holler_hub::state::resolve_state_dir().ok_or_else(|| PaneError::Unavailable {
        what: "no Holler state directory (HOLLER_STATE_DIR is unset and so is HOME) for the \
               Claude Code harness's socket files"
            .to_owned(),
    })?;
    let config = ClaudeConfig::new(bin, config_dir, state.join(CLAUDE_INBOX_DIR));
    Ok(Arc::new(ClaudeHarness::new(config)))
}
```

### E17: The hub's pane gate and its one choke point, send_prompt (as #839 keyed it)

Command: `sed -n '88,97p;113,115p' crates/holler-hub/src/circuit/dispatch.rs; sed -n '28,64p' crates/holler-hub/src/panes/gate.rs`

```text
/// `await`, so it is totally ordered against `hold`/`release` (see
/// `crate::holds`). `crates/holler-cli/tests/hold_single_path_test.rs` fails
/// if a second sender of `session/prompt` appears.
///
/// The pane-state gate (story #646, `crate::panes::gate::pane_gate`) is checked
/// here too, after the hold: an operator or default hold refuses first, with its
/// own kind. Since #839 it is keyed on the `harness_session_id` the session
/// advertises, however the prompt addressed the session, and selects the panes
/// whose session of record or recorded DRIVEN that id is; the session's name is
/// no key, and a session that advertises no id (spawn mode) is never pane-gated.
    if let (Some(panes), Some(id)) = (gate.panes.as_deref(), gate.harness_session) {
        crate::panes::gate::pane_gate(panes, id).map_err(SendPromptError::Refused)?;
    }
/// The pane-state gate: refuse a prompt to the session that advertises the harness session
/// `id` while a pane whose session of record or recorded DRIVEN is `id` is refused by the one
/// predicate. Fail closed across panes that record one id: any bad one refuses, and the first
/// bad one in name order (`list` is name-sorted) is the one named. One synchronous read with no
/// `await`, so it is totally ordered against a pane write, as `holds.admit` is against `hold`
/// and `release`.
///
/// The refusal is `-32011 session_held` with `data.hold_kind = "pane"` and `data.reason` the
/// pane's code (no `data.since`: the park's is epoch milliseconds, not the RFC 3339 the field
/// documents). `holler release` does not lift it; the remedy is `holler pane unpark` or `holler
/// pane doctor`.
///
/// A registry that cannot be read (`store-corrupt`, logged by the registry when it failed to
/// load) refuses nothing, as a hold file that cannot be read leaves the hub with no holds
/// (`crate::holds`): a broken pane file must not stop every prompt to every session.
pub(crate) fn pane_gate(panes: &PaneState, id: &str) -> Result<(), WireError> {
    let Ok(records) = panes.list() else {
        return Ok(());
    };
    let Some((pane, refusal)) = first_refusal(&records, id) else {
        return Ok(());
    };
    let code = refusal.code();
    let message = format!(
        "pane {} {}, so it refuses prompts ({code})",
        pane.name,
        pane_state_refusal(refusal)
    );
    let mut error = WireError::new(Code::SessionHeld, message, None).with_hold_kind("pane");
    if let Some(data) = error.data.as_mut() {
        data.reason = Some(code.as_str().to_owned());
    }
    Err(error)
}

#[cfg(test)]
mod tests;
```

### E18: The roster's join of a row to a pane

Command: `sed -n '316,333p' crates/holler-cli/src/roster_cmd.rs`

```text
/// The pane `hub` is joined to (acceptance 10): the one named like the row's [`pane_label`].
fn pane_of<'a>(hub: &Value, panes: &'a [PaneRow]) -> Option<&'a PaneRow> {
    let name = hub.get("name").and_then(Value::as_str)?;
    let label = pane_label(name);
    panes.iter().find(|pane| pane.name.as_str() == label)
}

/// The name of the pane a hub row named `name` is joined to (acceptance 10): the part of `name`
/// before its first `/`. That is the label of a row named `<label>/<session>` (ADR 0005; a label
/// has no `/`) and the whole name of a row with no label. So a labelled row is joined to the pane
/// named like its label, never like its session, and the pane's session of record plays no part.
///
/// This is the roster's relation between a row and a pane. The hub's prompt gate and `say --pane`
/// relate them by the harness session id the row's body advertises, which is the pane's session
/// of record or its recorded DRIVEN (ADR-0021 section 11, #839); this display join is #648's.
fn pane_label(name: &str) -> &str {
    name.split_once('/').map_or(name, |(label, _)| label)
}
```

### E19: interrupt --pane by abort style

Command: `sed -n '54,89p' crates/holler-cli/src/interrupt_cmd.rs`

```text
/// The interrupt of the pane `pane` (`--pane NAME [--profile P]`, with `text` its replacement
/// text): the pane gates of `say_cmd::resolve_pane_record` first (`pane-parked`,
/// `pane-unhealthy`, a SHOWN and DRIVEN mismatch), then by the abort style of its harness's
/// kind. In place (OpenCode): [`PaneInterrupt::Hub`], for the caller to send through the hub.
/// A relaunch (Claude Code): `interrupt-text-unsupported` with text; otherwise the pane is
/// relaunched on its session of record, a busy one included, through the pane engine, and the
/// answer is [`PaneInterrupt::Relaunched`] with the session its process holds.
pub fn interrupt_pane(
    ports: Ports<'_>,
    pane: &str,
    profile: Option<&str>,
    text: Option<&str>,
) -> Result<PaneInterrupt, PaneError> {
    let record = resolve_pane_record(ports, pane, profile)?;
    let session = session_of_record(&record)?;
    match record.harness.kind.abort_style() {
        AbortStyle::InPlace => Ok(PaneInterrupt::Hub { session }),
        AbortStyle::Relaunch if text.is_some() => Err(PaneError::Refused {
            code: INTERRUPT_TEXT_UNSUPPORTED,
            message: format!(
                "{} is interrupted by a relaunch on its session of record, which runs no \
                 replacement text; interrupt it without text, then say the text",
                record.name
            ),
        }),
        AbortStyle::Relaunch => {
            let keep = ProcessSession::Keep;
            let launched = relaunch_recorded(ports, &record, &keep, &TxOptions::default())
                .map_err(|failure| failure.error)?;
            let held = launched.pane.and_then(|pane| pane.session_of_record);
            Ok(PaneInterrupt::Relaunched {
                session: held.unwrap_or(session),
            })
        }
    }
}
```

### E20: The text form's refusal code and its stated reason

Command: `sed -n '240,247p' crates/holler-cli/src/say_cmd.rs`

```text
/// `interrupt --pane` with replacement text on a pane whose interrupt is a relaunch (a Claude
/// Code pane, #831): the relaunch drops the turn in flight, and no body exists to run the text
/// after it until #850. Refusal, exit 3.
pub const INTERRUPT_TEXT_UNSUPPORTED: RefusalCode =
    RefusalCode::from_static("interrupt-text-unsupported");
/// `answer --pane` on a single-process pane (a Claude Code pane, #831): nothing but a keypress
/// answers its question, and its launch removes the question tool. Refusal, exit 3.
pub const ANSWER_UNSUPPORTED: RefusalCode = RefusalCode::from_static("answer-unsupported");
```

### E21: The ignored say step of #831's scenario

Command: `sed -n '239,264p' crates/holler-cli/tests/pane_claude/mixed.rs`

```text
/// The scenario's `holler say` step (#831's acceptance, ADR-0021 section 15 item 18). It cannot
/// pass until the hub can reach both panes: the hub delivers only to a connected body addressed by
/// its advertised name, and `say --pane` resolves a pane to its session of record, which matches
/// no body's name until #839 joins a body's `harness_session_id` to `Pane.session_of_record`; a
/// Claude Code pane also has no body at all until #850. Run it with `--ignored` once both land.
#[test]
#[ignore = "blocked by #839 (the hub joins a body's harness session to Pane.session_of_record) and #850 (a body driver for a Claude Code pane): holler say --pane reaches no pane of either harness until both land"]
fn say_delivers_a_message_to_each_pane_of_the_scenario() {
    let rig = applied();
    let mut addresses = Vec::new();
    for pane in [OPENCODE, CLAUDE] {
        let address = rig
            .with_ports(|ports| resolve_pane_target(ports, pane, None))
            .unwrap_or_else(|e| panic!("{pane} resolves: {e}"));
        assert_eq!(
            Some(address.as_str()),
            rig.record(pane).session_of_record.as_deref()
        );
        addresses.push((pane, address));
    }
    panic!(
        "`holler say --pane` would send to the hub addresses {addresses:?}, which no body \
         advertises until #839 (and, for the Claude pane, #850): replace this line with the \
         real delivery through the hub, then assert the message reached each pane"
    );
}
```

### E22: A single process's switch and reset as #839 left them: the relaunch is the act and stores the record; the server's drive step and first message are the model

Command: `sed -n '229,243p;285,311p;313,337p;339,354p' crates/holler-pane/src/tx_switch.rs`

```text
/// Run one switch or reset over `ports` (see the module docs). `Ok` once the target was seen
/// shown (by the TUI, or held by the relaunched process) and the record names it.
pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure> {
    let plan = plan(ports, request)?;
    let pane = match plan.record.harness.kind.shape() {
        Shape::ServerAndTui => switch_in_place(ports, request, &plan)?,
        Shape::SingleProcess => relaunched(ports, request, &plan.record)?,
    };
    if let Some(text) = &request.first {
        prompt(ports, &pane, text)?;
    }
    Ok(Switched {
        pane,
        previous: plan.record.session_of_record,
    })
/// Step 4: re-point the pane's body to `target` (#839). Answers the DRIVEN the record gets:
/// the id the body acknowledged, or `None` when no body drives the pane. A body that already
/// advertises the target is not asked, and an acknowledgement of another id is `unavailable`.
fn drive(
    ports: Ports<'_>,
    driver: Option<&Advertised>,
    target: &str,
) -> Result<Option<String>, PaneError> {
    let Some(body) = driver else {
        return Ok(None);
    };
    if body.harness_session_id == target {
        return Ok(Some(target.to_owned()));
    }
    let acknowledged = ports.hub.rebind(body, target)?;
    if acknowledged != target {
        return Err(PaneError::Unavailable {
            what: format!(
                "the body {} acknowledged session {}, not {}",
                quoted(&body.name),
                quoted(&acknowledged),
                quoted(target)
            ),
        });
    }
    Ok(Some(acknowledged))
}
/// Steps 2 to 4 for a single process (#831): a relaunch of the pane as recorded, on the target
/// (switch, which must have a conversation) or on a fresh session (reset), through
/// `tx_launch::relaunch_recorded`, the one stop, run, wait, locate and record sequence. Returns
/// the record the relaunch stored.
fn relaunched(
    ports: Ports<'_>,
    request: &SwitchRequest,
    record: &Pane,
) -> Result<Pane, SwitchFailure> {
    let session = match &request.target {
        Target::Existing(id) => ProcessSession::Resume(id.clone()),
        Target::Fresh => ProcessSession::Fresh,
    };
    let launched = tx_launch::relaunch_recorded(ports, record, &session, &TxOptions::default())
        .map_err(|TxFailure { error, acted }| SwitchFailure {
            error,
            acted,
            created: None,
            unprompted: None,
        })?;
    let stored = launched.pane.ok_or_else(|| PaneError::Unavailable {
        what: format!("the relaunch of {} stored no record", record.name),
    })?;
    Ok(stored)
}
/// Step 6: queue `text` to the session of record of `pane`, the record the run stored, through
/// the hub's one prompt path (#839: `HubPort::prompt` with `Send`, accepted and returned). The
/// prompt goes only to the session just recorded (I2), so a failure here, a refusal in a race
/// included, leaves the TUI and the record in agreement: no reconcile step, nothing unrecorded.
fn prompt(ports: Ports<'_>, pane: &Pane, text: &str) -> Result<(), SwitchFailure> {
    let target = pane.session_of_record.clone().unwrap_or_default();
    ports
        .hub
        .prompt(&target, text, PromptMode::Send)
        .map_err(|error| SwitchFailure {
            error,
            acted: false,
            created: None,
            unprompted: Some(target.clone()),
        })
}
```

### E23: ADR-0021 section 15 item 9 (abort and interrupt)

Command: `sed -n '1492,1503p' docs/adr/ADR-0021.md`

```text
9. **Abort and interrupt.** `abort(endpoint, session)` for a kind whose `abort_style()` is `Relaunch` is refused
   `abort-unsupported` (open, exit 3): there is no in-place abort, and a signal is not one (SIGINT ends the process). `holler
   interrupt --pane NAME` for such a pane does not go to the hub (it has no body, item 18): after the same pane-state gates
   `resolve_pane_target` applies today (parked, unhealthy, shown and driven apart), the CLI relaunches the pane on its
   session of record through the pane engine. `interrupt` with replacement text on such a pane is refused
   `interrupt-text-unsupported` in this story. `answer --pane` on a Claude pane is refused `answer-unsupported` (nothing but a
   keypress answers a question; the launch removes the question tool, 11). **As built:**
   `interrupt_cmd::interrupt_pane(ports, pane, profile, text) -> PaneInterrupt::{Hub { session }, Relaunched { session }}` by
   the kind's `abort_style()` (a busy pane is relaunched too: that is the interrupt), and
   `answer_cmd::answer_pane_target(ports, pane, profile)`, both after `say_cmd::resolve_pane_record`, the pane gates of
   `resolve_pane_target` returning the record. `holler interrupt --pane NAME` prints `interrupted NAME: relaunched on its
   session of record "<session>"` for such a pane, exit 0.
```

### E24: ADR-0021 section 15 item 18 and the paragraph after it

Command: `sed -n '1598,1623p' docs/adr/ADR-0021.md`

```text
18. **`holler say` and the acceptance step: it needs #839, and more (checked 2026-10-11).** The scratch scenario's step "deliver
    a message to each through `holler say`" cannot be met by this story alone, for either harness:
    - The hub reaches a harness session only through a **body**: `say` sends `session/prompt` to a connected body that drives
      its harness, addressed by the body's advertised name (ADR 0014 point 5; `send_prompt` and `pane_gate` in
      `holler-hub/src/circuit/dispatch.rs`). `holler say --pane NAME` resolves the pane to its `session_of_record` and uses
      that as the address (`resolve_pane_target`), which matches no body's name: the id is an OpenCode `ses_…` or a Claude
      UUID, and the join from a body's `harness_session_id` to `Pane.session_of_record` is **#839** (DRIVEN, not built). So
      `say --pane`, its holds and its pane gate reach no real pane of either harness until #839.
    - A Claude Code pane additionally has **no body at all**: the hub constructs no adapter (decision 1 of section 5), and the
      adapter this story builds is called by the pane engine, not by a body. A body driver for it is the follow-up story **#850**.
    So this story proves delivery where it can, at `HarnessPort::send_prompt` (item 7) against `FakeClaude` and the opt-in
    real-binary test, and, until #839, through `reset --first`; the `say` step of the scenario is written down, marked blocked by #839 and
    #850, and **#831 does not close on it**. Nothing here routes a prompt around the hub's gate: the Claude pane's `say` is not
    given a CLI-side path, because that would be a second enforcement site for holds and parks. **As built (plan review W4 and
    W12):** delivery is proven at `HarnessPort::send_prompt`, on `FakeClaude`, in the conformance suite's case 9 and in the
    opt-in real test. `reset --first` reached the relaunched process (the endpoint of the record the relaunch stored) through
    the existing carve-out around the hub's prompt gate (section 8, the seam). Moving it to the hub's prompt path was #828's,
    which was closed on 2026-10-11 (not planned) as folded into #839: since #839 moved it, a Claude pane's `reset --first` is refused
    `pane-not-driven` until #850 (section 11, point 6). **The roster, as built:** #648 merged before this story's PR, so `holler roster` reads the registry and
    prints a pane's cells on the hub row joined to it. Its rows are the sessions that bodies serve, so a Claude Code pane has
    no roster row until a body drives its session (#850); `holler pane list` and `holler pane doctor` show it now.

**Not built here, and where it goes.** The body driver for `say` (#850), which also gives a Claude pane its roster row; the
hub's join (#839); `pane-stalled` (with #837's time source); the `PreToolUse` hook gate; a permission-mode, allowed-tools
and questions spec field; `--agent` for Claude; answering a permission prompt by hook; interrupt with replacement text; a
Windows or macOS socket directory (the note's "Not tested").
```

### E25: ADR-0021 Deferred and Decisions taken, the places #850 is named

Command: `sed -n '1651,1654p;1747,1748p' docs/adr/ADR-0021.md`

```text
- A body for a Claude Code pane, so that `holler say --pane NAME TEXT` and `reset --first` reach it and DRIVEN exists for it:
  #850 (section 15, item 18). `pane-stalled` waits
  with #837's time source, and the `PreToolUse` gate, a permission-mode, allowed-tools and questions spec field and `--agent`
  for Claude are later stories (section 15).
24. **`holler say --pane NAME TEXT` reaches a Claude Code pane through a body, after #839** (2026-10-11, #831, section 15, item 18): the
    hub gets no CLI-side path and no adapter of its own. The body driver is #850. Decided under the operator's standing rules.
```

### E26: The guide's passages that say a Claude Code pane has no body

Command: `sed -n '78,84p;287,289p;594,597p;1079p' docs/pane-control.md`

```text
- **The prompt verbs.** `holler interrupt --pane NAME` on a Claude Code pane relaunches it on its session of record (the
  operator's decision: no keypress, and a signal would end the process), dropping the turn in flight; with text it is
  refused `interrupt-text-unsupported`, and `holler answer --pane NAME CHOICE` is refused `answer-unsupported`.
  `holler say --pane NAME TEXT` does not reach a Claude Code pane yet: no body drives its session until
  [#850](https://github.com/Performant-Labs/holler/issues/850) (section 12), so it is refused `pane-not-driven`, and so
  is `holler pane reset PANE --first TEXT`.
- A pane's harness kind is never changed: `relaunch` with another `--harness`, or `profile apply` of a spec that names
session and DRIVEN recorded as for `switch`. A Claude Code pane is reset by a relaunch on a fresh session id, refused as
`switch` is (`session-holds-permission` included); `--first` on a Claude Code pane is refused `pane-not-driven` until a
body drives it (#850).
alpha or beta agent) shows `-` in all six. A roster row is joined to a pane when the part of its name before the first
`/` is the pane's name. The rows are the hub's, one per session a body serves, so a pane that no body serves has no row:
a Claude Code pane has none until a body drives its session ([#850](https://github.com/Performant-Labs/holler/issues/850)),
and `pane list` shows it meanwhile. The roster calls no adapter and shows no unmanaged pane (`pane list` does), and it
- a body that lets `holler say --pane NAME TEXT` reach a Claude Code pane (#850);
```

### E27: The CHANGELOG bullet of #831 that names #850

Command: `sed -n '63,85p' CHANGELOG.md`

```text
- Pane control, a Claude Code harness (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler pane launch PANE --harness claude --port-policy none` launches a pane whose harness is one interactive Claude
  Code process, recorded by its pid and with no port, and the pane verbs drive it
  ([#831](https://github.com/Performant-Labs/holler/issues/831); adopting the fleet's existing Claude Code panes is its
  last step and waits for [#835](https://github.com/Performant-Labs/holler/issues/835)). The launch starts Claude Code on
  a fresh session id with the seven environment variables that would override its login removed, so the process uses
  the login Claude Code holds (a spec that names one of them is refused `claude-login-env-refused`, and `holler pane
  doctor` reports `claude-login-missing` when that login is not the subscription's), accepts cross-session messages and
  leaves out the question tool, and writes the record only once `claude agents --json` lists the process idle on its
  session; otherwise it rolls back. Its `--model` and `--effort` are applied. A Claude Code process chooses its session
  when it starts, so `relaunch`, `switch` (to a conversation of the pane's directory), `reset` (a fresh session),
  doctor's `--fix` (only while it is idle) and `holler interrupt --pane NAME` (on its session of record: the operator's
  decision, no keypress) restart it. `switch` and `reset` also refuse a pane waiting on a permission prompt
  (`session-holds-permission`), an interrupt with text is `interrupt-text-unsupported` and `answer --pane` is
  `answer-unsupported`. `holler pane doctor` reports two new findings, `pane-waiting` and `claude-login-missing`.
  `holler say --pane NAME TEXT` does not reach a Claude Code pane yet, and `holler roster`, whose rows are the sessions
  that bodies serve, has no row for one: both need a body for its session
  ([#850](https://github.com/Performant-Labs/holler/issues/850), after
  [#839](https://github.com/Performant-Labs/holler/issues/839)). The new crate `holler-adapter-claude` implements the
  harness port (the binary is `HOLLER_CLAUDE_BIN`, else `claude` on `PATH`); every harness call is now keyed by an
  endpoint, a port or a process; and `HostPort::foreground_commands` with the one Claude Code command rule is built to
  #835's brief. [ADR 0021](docs/adr/ADR-0021.md) section 15 records the contract and what was built, and
  [`docs/pane-control.md`](docs/pane-control.md) documents the Claude Code pane.
```

### E28: The names a body session and a pane share, and the longest segment

Command: `sed -n '24p;34,38p' crates/holler-proto/src/vocab.rs`

```text
pub const MAX_SEGMENT_LEN: usize = 32;
    /// Succeeds iff the string is a single 1–32-char segment matching the
    /// grammar (no `/`). A `Label` never contains a `/`.
    pub fn parse(s: &str) -> Result<Self, NameError> {
        if s.is_empty() {
            return Err(NameError::Empty);
```

### E29: HARNESS_IDS and what its doc says of the two uses of claude

Command: `sed -n '270,283p' crates/holler-proto/src/vocab.rs`

```text
/// adapter). A new harness is a new config row + a new id, not a protocol bump.
/// Two of these ids also name a pane's harness (`holler_pane::pane::HarnessKind`):
/// `opencode` and `claude`.
///
/// The id `claude` has two uses that do not meet (ADR-0021 section 15, item 1):
/// a **pane** harness `claude` is Claude Code's interactive CLI and is read only
/// by the pane engine (`HarnessKind::Claude`, #831); a **body** harness `claude`
/// is the ACP bridge (ADR 0012) and is read only by `holler-body`. `HarnessKind`
/// is a second list beside this one, and a test (`holler-pane`'s
/// `tests/rework_test.rs`) pins every `HarnessKind` serde name to this list.
pub const HARNESS_IDS: &[&str] = &[
    "opencode", "claude", "codex", "grok", "hermes", "pi", "cursor", "copilot", "droid", "kimi",
    "qwen", "kilo", "goose",
];
```

### E30: HarnessEndpoint: a process is a pane and a pid

Command: `sed -n '52,59p;78,91p' crates/holler-pane/src/ports/process.rs`

```text
/// Where a pane's harness is reached (#831, ADR-0021 section 15 item 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessEndpoint {
    /// A harness server on this port of 127.0.0.1 (OpenCode).
    Port(u16),
    /// The one process of a single-process harness (Claude Code): the pane whose tmux session
    /// runs it, and its pid, which is its identity.
    Process { pane: PaneName, pid: u32 },
    /// The pane and the pid of a process endpoint; [`SERVER_UNSUPPORTED`] for a port, which a
    /// single-process harness has none of. The one conversion a single-process harness makes.
    pub fn process(&self) -> Result<(&PaneName, u32), PaneError> {
        match self {
            HarnessEndpoint::Process { pane, pid } => Ok((pane, *pid)),
            HarnessEndpoint::Port(port) => Err(PaneError::Refused {
                code: SERVER_UNSUPPORTED,
                message: format!(
                    "port {port} names a harness server: this harness is one process and has \
                     no port"
                ),
            }),
        }
    }
```

### E31: What a body advertises: the attach row's session id is its session task's mirror of the configuration

Command: `sed -n '407,422p' crates/holler-body/src/session_manager.rs`

```text
                let p = managed.presence.lock().unwrap_or_else(PoisonError::into_inner);
                if p.omitted_pending_attach {
                    return None;
                }
                Some(SessionAd {
                    name: name.as_str().to_string(),
                    harness: managed.config.harness.clone(),
                    state: p.state,
                    mode: match managed.config.mode {
                        SessionMode::Spawn => Mode::Spawn,
                        SessionMode::Attach => Mode::Attach,
                    },
                    harness_session_id: p.harness_session_id.clone(),
                    turn_started_at: p.turn_started_at.clone(),
                    last_update_at: p.last_update_at.clone(),
                    pending: p.pending.clone(),
```

### E32: query/support for an attach row today

Command: `sed -n '149,185p;286,296p' crates/holler-body/src/query.rs`

```text
        };
    }
    for s in &sessions {
        match s.mode {
            SessionMode::Attach => {
                let (endpoint, session_id) = (s.endpoint.as_deref(), s.session_id.as_deref());
                let ok = match (endpoint, session_id) {
                    (Some(endpoint), Some(session_id)) => {
                        probe_attach_session_live(endpoint, session_id)
                    }
                    // Defensive: `config::parse` already refuses an `attach`
                    // row missing either field, so this never happens for a
                    // config that reached this far — but a probe with
                    // nothing to dial is `ok:false`, not a fabricated `true`.
                    _ => false,
                };
                return Support {
                    feature: harness.to_string(),
                    kind: SupportKind::Harness,
                    ok,
                    how: ok.then(|| "attach".to_string()),
                    reason: (!ok).then(|| "attach endpoint did not answer".to_string()),
                };
            }
            SessionMode::Spawn => {
                if let Some(first) = s.command.as_ref().and_then(|c| c.first()) {
                    if resolvable(first) {
                        return Support {
                            feature: harness.to_string(),
                            kind: SupportKind::Harness,
                            ok: true,
                            how: Some(format!("spawn: {first}")),
                            reason: None,
                        };
                    }
                }
            }
/// `true` iff `cmd` is directly runnable: an existing file at an absolute or
/// relative path, or a bare name resolvable on `PATH`.
fn resolvable(cmd: &str) -> bool {
    if cmd.contains('/') {
        return Path::new(cmd).is_file();
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(cmd).is_file())
}
```

### E33: The spike: the inbox socket and the frame

Command: `sed -n '119,126p' docs/research/claude-code-pane-spike.md`

```text
### A. The inbox socket (Verified; frame **Read**)

- Every interactive session on macOS and Linux binds a unix socket, restricted to the OS user (mode `600`, in a private
  per-user directory; observed `/tmp/cc-socks/<pid>.sock`, the docs name `/tmp/cc-socks-<uid>` as the fallback). The
  docs say a script or hook may post into it, with an optional `{"type":"auth","token":"…"}` first line on Linux.
- The client prints the working frame in its own `--debug-file` startup log: `{"type":"user","message":{"role":"user",
  "content":"hello"}}`, one line, newline-terminated. A connection that sends no complete line within 30 s is closed.
  This is **Read**, not documented: it must be pinned by a conformance probe (the scripts are that probe).
```

### E34: The spike: the latency of a status change, and there is no heartbeat in agents --json

Command: `sed -n '208,209p;217,220p' docs/research/claude-code-pane-spike.md`

```text
- **Latency.** One `agents --json --cwd …` call takes about 90 ms. `status` changed within 100–300 ms of the event in
  every timeline (busy 277 ms after a socket write, a permission wait 1.3 s after the prompt, which is the model's own time).
- **There is no heartbeat.** The registry's `updatedAt` did not move in 6 s of waiting, and `agents --json` has no
  `statusUpdatedAt`. A wedged process and a process legitimately inside a 4-hour tool call both show `busy`. So the hub
  keeps its **own** observation history (first poll that saw this status) and combines it with the transcript's mtime
  (the path is documented) to say `stalled`. *Inferred, not tested over hours.* This is the case in the epic's comment of
```

### E35: v2.md: query/support for an attach harness, and the attach-mode section

Command: `sed -n '437,444p;535p;551,557p;603,605p' docs/protocol/v2.md`

```text
`attach` and `opencode-http` (`kind:"capability"`) are `ok:true`
unconditionally — both are implemented body-side, independent of any session
config. A **harness** id backed by an `attach`-mode session (issue #195) is
the one probe that is not purely local bookkeeping: it dials the configured
`endpoint` (`GET /session/{session_id}`, falling back to `/api/session/{id}`
— the same dual-prefix existence check `SessionManager` itself uses to
attach) and reports `ok:true` only while that endpoint actually answers —
`ok:false, reason:"attach endpoint did not answer"` the moment it stops.
- `mode` is `spawn` | `attach`. `harness_session_id` is present for `attach` (the existing harness session being attached to). A `session/rebind` changes it; the hub's cache and roster row are updated when the rebind is acknowledged (#839).
  - **Persistence:** holds are written atomically to `<state dir>/hub/holds.json` (mode `0600`). A corrupt file is moved aside to `holds.json.corrupt-<time>` and the hub starts with no holds (a `warn` event `hold_state_corrupt` names both files); a file the hub cannot read is never overwritten; an unwritable directory keeps the hold in force in memory and the control reply says `persisted: false`.

### Attach-mode sessions (issue #195)

`mode:"attach"` is dispatched to `HttpAttachDriver` instead of `AcpDriver`
(§`crates/holler-body/src/http_attach_driver.rs`) — it never spawns anything;
it attaches to a harness session another process already owns.
`control/query_remote` and `admin/query` refuse it with `-32601`. The rebind
is not persisted: a restarted body attaches the session of its configuration
again.
```

### E36: README: the harness recipe section a Claude Code row joins

Command: `sed -n '303,313p' README.md`

```text
## Harness recipes

A `[[session]]` row is config, not code (ADR 0012): pointing `harness` and `command` at a new
ACP-speaking adapter needs no Holler code change or release. For example, Claude Code itself
doesn't speak ACP — there is no `claude acp` subcommand — so a session that drives it goes through
[`@agentclientprotocol/claude-agent-acp`](https://www.npmjs.com/package/@agentclientprotocol/claude-agent-acp)
(npm), the actively-maintained ACP-project bridge for Claude Code:

```toml
[[session]]
name = "reviewer"
```

### E37: How a default-run test starts a real hub, a real body and a fake OpenCode server

Command: `sed -n '30,52p;57,64p' crates/holler-cli/tests/attach_mode_test.rs`

```text
#[path = "../../holler-body/tests/http_attach_driver_test/fake_server.rs"]
mod fake_server;
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use fake_server::{message_updated_assistant, part_updated_text, session_idle, FakeServer};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::http_attach_driver::HttpAttachDriver;
use holler_proto::SessionName;
use serde_json::Value;

use support::{stub_acp_bin, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

/// Serializes every test in this file so at most one is running at a time —
/// the same pattern `acp_driver_test.rs` uses for the identical reason (see
/// its own doc comment on its `SERIAL`): each test here spawns a real hub +
/// body process pair (plus a fake HTTP server task and, in several tests,
/// extra `std::thread`s racing them), and running all 8 fully concurrently
/// (`cargo test`'s default per-binary parallelism) measurably adds
/// scheduling/CPU contention on a shared CI runner — enough to threaten the

/// Run `holler ARGS... --json` against `state_path` directly (a bare
/// `PathBuf`, not `&StateDir`) — for callers that need to run a CLI command
/// from a background thread while the owning `StateDir` stays on the test's
/// main thread (mirrors `talk_test.rs`'s own `say_full`, duplicated here
/// since that helper lives in a different test binary target).
fn say_at(state_path: &Path, session: &str, text: &str) -> Output {
    Command::new(support::holler_bin())
```

### E38: The manifest entry of that test (holler-cli has autotests off)

Command: `sed -n '270,281p' crates/holler-cli/Cargo.toml`

```text
name = "attach_cli_test"
path = "tests/attach_cli_test.rs"

# `mode = "attach"` end to end (issue #195): real hub + body + a fake OpenCode
# HTTP server (reusing `holler-body`'s own `http_attach_driver_test/
# fake_server.rs` via a `#[path]` include). Lives here for the same
# `CARGO_BIN_EXE_stub-acp`/`CARGO_BIN_EXE_holler` reasons as every other
# real-subprocess integration test in this file.
[[test]]
name = "attach_mode_test"
path = "tests/attach_mode_test.rs"

```

### E39: The build guards: size and allow lines

Command: `sed -n '7,11p;43,47p' scripts/lint.sh`

```text
# 1. No blanket dead-code allows, and no #[allow(...)] without an issue link
#    on the same line. Exception: a forward-contract stub (a helper shipped
#    ahead of its first caller, e.g. the test harness's Hub::start/join) may
#    allow(dead_code) — but only if that allow carries a trailing `// #NNN`
#    issue link naming the story that lands the caller. Otherwise dead code
# 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
#    file cross 1k unnoticed).
while read -r n f; do
  if [ "$n" -ge 900 ]; then
    echo "lint: $f is $n lines — decompose before merging"
```

### E40: Sizes of every file this story touches or may grow (wc -l)

Command: `wc -l crates/holler-body/src/config.rs crates/holler-body/src/connection.rs crates/holler-body/src/session_manager.rs crates/holler-body/src/session_manager/driver.rs crates/holler-body/src/session_manager/task.rs crates/holler-body/src/query.rs crates/holler-body/src/lib.rs crates/holler-body/src/http_attach_driver.rs crates/holler-cli/src/body_cmd.rs crates/holler-cli/src/pane/wiring.rs crates/holler-cli/src/interrupt_cmd.rs crates/holler-cli/src/say_cmd.rs crates/holler-pane/src/tx_switch.rs crates/holler-pane/src/tx_switch/plan.rs crates/holler-pane/src/reconcile.rs crates/holler-pane/src/reconcile/observe.rs crates/holler-pane/src/reconcile/observe/process.rs crates/holler-cli/tests/session_manager_test.rs crates/holler-cli/tests/attach_mode_test.rs crates/holler-cli/tests/pane_claude/mixed.rs crates/holler-cli/tests/pane_claude/lifecycle.rs crates/holler-cli/tests/pane_claude/driven.rs crates/holler-cli/tests/pane_claude/rig.rs crates/holler-pane-testkit/src/claude.rs docs/adr/ADR-0014.md docs/adr/ADR-0021.md docs/protocol/v2.md docs/pane-control.md`

```text
   668 crates/holler-body/src/config.rs
   874 crates/holler-body/src/connection.rs
   441 crates/holler-body/src/session_manager.rs
    78 crates/holler-body/src/session_manager/driver.rs
   794 crates/holler-body/src/session_manager/task.rs
   380 crates/holler-body/src/query.rs
    30 crates/holler-body/src/lib.rs
   435 crates/holler-body/src/http_attach_driver.rs
   311 crates/holler-cli/src/body_cmd.rs
   508 crates/holler-cli/src/pane/wiring.rs
   205 crates/holler-cli/src/interrupt_cmd.rs
   588 crates/holler-cli/src/say_cmd.rs
   399 crates/holler-pane/src/tx_switch.rs
   305 crates/holler-pane/src/tx_switch/plan.rs
   571 crates/holler-pane/src/reconcile.rs
   554 crates/holler-pane/src/reconcile/observe.rs
   240 crates/holler-pane/src/reconcile/observe/process.rs
   875 crates/holler-cli/tests/session_manager_test.rs
   509 crates/holler-cli/tests/attach_mode_test.rs
   264 crates/holler-cli/tests/pane_claude/mixed.rs
   400 crates/holler-cli/tests/pane_claude/lifecycle.rs
    72 crates/holler-cli/tests/pane_claude/driven.rs
   466 crates/holler-cli/tests/pane_claude/rig.rs
   641 crates/holler-pane-testkit/src/claude.rs
    38 docs/adr/ADR-0014.md
  1833 docs/adr/ADR-0021.md
   849 docs/protocol/v2.md
  1082 docs/pane-control.md
 14410 total
```

### E41: #839 as merged: a single process takes no driver in the plan (D3 item 8) and has no drive step

Command: `sed -n '43,54p' crates/holler-pane/src/tx_switch/plan.rs; sed -n '56,66p' crates/holler-pane/src/tx_switch.rs`

```text
    if record.harness.kind.shape() == Shape::SingleProcess {
        check_idle(ports, &record, &at)?;
        if let Some(target) = &existing {
            check_unclaimed(ports, &record.name, target)?;
        }
        // No body drives a single process until #850, so the hub is not asked (D3 item 8).
        check_first(ports, request, &record, None)?;
        return Ok(Plan {
            record,
            driver: None,
        });
    }
//! **A single process (#831, ADR-0021 section 15 item 6).** A pane whose harness's shape is
//! `SingleProcess` (Claude Code) has no server to create a session on and no TUI to switch, so
//! its switch and its reset are relaunches. The plan refuses as above, except that a switch also
//! refuses a pane that is not idle (a relaunch drops the turn in flight) and does not ask for a
//! session list: whether the target has a conversation is the relaunch plan's
//! (`process_command` with a resume-only choice: `session-not-found`, before anything is
//! stopped; plan review W1). It asks the hub nothing: no body drives such a pane until #850, so
//! its `reset --first` is refused `pane-not-driven` in the plan. Steps 2 to 5 are then
//! `tx_launch::relaunch_recorded`: the pane's processes stopped, its process run on the target
//! (switch) or on a fresh session (reset), located, seen holding it and recorded with its new
//! pid, its stored DRIVEN kept.
```

### E42: #839 as merged: the scope limit of session/rebind, and a Claude pane until #850 (ADR-0021 section 11)

Command: `sed -n '1047,1048p;1113,1116p' docs/adr/ADR-0021.md`

```text
   time: the circuit has one reply slot, shared by `session/answer`, `query/*` and the rebind, and the hub serialises the
   three. Only the HTTP attach driver implements it; a body for another harness implements it in its own driver. Rejected:
   more than one body could drive the pane, or none can be shown to be its own, and the call names none of them (point 3). Only an OpenCode attach body exists
   today. A Claude Code pane (#831) is in this case until a body can drive it (#850), and its switch and reset, which are a relaunch, ask the hub nothing; the join is a string comparison of the
   advertised id with the session of record, so its session id joins the same way once a body advertises it, and that body
   implements `session/rebind` in its own driver.
```

### E43: #839 as merged: the roster's join is left to #648's display rule

Command: `sed -n '326,330p' crates/holler-cli/src/roster_cmd.rs`

```text
/// named like its label, never like its session, and the pane's session of record plays no part.
///
/// This is the roster's relation between a row and a pane. The hub's prompt gate and `say --pane`
/// relate them by the harness session id the row's body advertises, which is the pane's session
/// of record or its recorded DRIVEN (ADR-0021 section 11, #839); this display join is #648's.
```

### E44: #839 as merged: the offline marker, and which refusals are pane-not-driven

Command: `sed -n '1012,1016p' docs/adr/ADR-0021.md; sed -n '11,13p;47,51p' crates/holler-cli/src/pane/hub_refusal.rs`

```text
1. **The address is the advertised `harness_session_id`.** At the hub, `say`, `interrupt` and `answer` resolve a target
   that is a valid name (ADR 0005) by name, as before, and any other target as a harness session id, against the live
   attach sessions: no match is unknown, a match on an offline body is not connected, two matches are ambiguous. The hub
   marks the offline answer, `-32004 not_connected`, with an optional `data.reason` of `offline`; that is how a `--pane`
   verb tells a pane that no body drives from a prompt the body did not answer in time, which is the same code without the
//! A `-32004 not_connected` is `NotDriven` only when the hub marked it `offline` (P7): the same
//! code without the reason is a turn that outlived its wait, a cancelled prompt, an unconfirmed
//! cancel or a failed redirect, and keeps its text and its exit 1. A hub that predates the reason
    match Code::from_jsonrpc(error.code) {
        Some(Code::UnknownSession) => HubRefusal::NotDriven(message),
        Some(Code::NotConnected) if reason == Some(OFFLINE_REASON) => {
            HubRefusal::NotDriven(message)
        }
```

### E45: #835 as merged: the wiring forwards a HostPort method (read only)

Command: `sed -n '369,379p' crates/holler-cli/src/pane/wiring.rs`

```text
        Err(PaneError::NotImplemented)
    }

    fn foreground_commands(&self, _name: &PaneName) -> Result<Vec<String>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn attached_clients(&self, _name: &PaneName) -> Result<u32, PaneError> {
        Err(PaneError::NotImplemented)
    }
}
```

### E46: The merged wizard (#803): one local body, an attach row per OpenCode pane, none for a Claude pane yet

Command: `sed -n '77p;96p' agent-skills/herdr-setup/SKILL.md; sed -n '171,173p' docs/setup-wizard.md`

```text
body row and no roster row until #850, so Stage 10's `holler say` round trip covers OpenCode panes only.
   the registry (Stage 8). A Claude Code pane has no server and no body row yet (#850).
   rows are derived from the registry. A Claude Code pane has no body row until
   [#850](https://github.com/Performant-Labs/holler/issues/850), so it has no roster row either, and
   a `say` round trip reaches the OpenCode panes only.
```

### E47: holler-cli declares every test target by hand; holler-body does not

Command: `grep -n 'autotests' crates/holler-cli/Cargo.toml crates/holler-body/Cargo.toml`

```text
crates/holler-cli/Cargo.toml:14:autotests = false
crates/holler-cli/Cargo.toml:40:# Explicit integration-test targets. With autotests = false (set above, to keep
```

### E48: The only caller of the body's run, and its signature

Command: `sed -n '30,36p;159,173p' crates/holler-cli/src/body_cmd.rs; sed -n '101p;132,133p' crates/holler-body/src/connection.rs`

```text
fn body_state() -> Result<holler_hub::state::HubState, i32> {
    match holler_hub::state::resolve_state_dir() {
        Some(root) => Ok(holler_hub::state::HubState::from_root(root)),
        // `resolve_state_dir` already printed the refusal to stderr.
        None => Err(1),
    }
}
    let registry = holler_body::registry::SessionRegistry::from_sessions(parsed.sessions);

    // Issue #190: `connection::run` now owns the registry outright — it
    // starts a `SessionManager` over it (one task per session, for this
    // process's whole live lifetime), derives issue #185's `query/*`
    // config rows from the same registry internally, and drives every
    // `session/presence` from the manager's *live* state, not the
    // registry's own always-`idle` snapshot.
    match holler_body::connection::run(&state.root, registry) {
        holler_body::connection::RunExit::Ok => 0,
        holler_body::connection::RunExit::NotJoined
        | holler_body::connection::RunExit::AuthFailed(_)
        | holler_body::connection::RunExit::Io(_) => 1,
        holler_body::connection::RunExit::LockHeld(_) => 3,
    }
pub fn run(state_root: &Path, registry: SessionRegistry) -> RunExit {
        // socket — only the WS connection is retried, never a session.
        let session_manager = Arc::new(SessionManager::start(&registry));
```

### E49: The test kit's fake process: statuses, what a view holds, and its delivery

Command: `sed -n '136,162p;380,406p' crates/holler-pane-testkit/src/claude.rs`

```text
/// A process's status, as `claude agents --json` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeStatus {
    Idle,
    Busy,
    /// Waiting for a person; [`ProcessView::waiting_for`] says on what.
    Waiting,
}

/// What [`FakeClaude::process`] reports about one live process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessView {
    /// The pane whose tmux session runs it.
    pub pane: PaneName,
    pub pid: u32,
    /// The session it holds now.
    pub session_id: String,
    pub status: ClaudeStatus,
    /// What it waits on (`permission prompt`, `input needed`), while it waits.
    pub waiting_for: Option<String>,
    /// Every message delivered to it, oldest first.
    pub inbox: Vec<String>,
    /// Whether it accepts a cross-session message (Holler's launch does; a process run by
    /// hand does not).
    pub accept_inbound: bool,
}

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        self.faults.enter(ClaudeOp::SendPrompt)?;
        let (pane, pid) = at.process()?;
        self.lock().deliver(pane, pid, session, text)
    }

    fn session_activity(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
    ) -> Result<Activity, PaneError> {
        self.faults.enter(ClaudeOp::SessionActivity)?;
        let (pane, pid) = at.process()?;
        let world = self.lock();
        let process = world.find(pane, pid).ok_or_else(|| gone(pane, pid))?;
        Ok(match (process.status, process.waiting_for.as_deref()) {
            (ClaudeStatus::Idle, _) => Activity::Idle,
            (ClaudeStatus::Busy, _) => Activity::Busy,
            (ClaudeStatus::Waiting, Some(INPUT_NEEDED)) => Activity::HoldingQuestion,
            (ClaudeStatus::Waiting, _) => Activity::WaitingPermission,
        })
    }
```

### E50: The adapter reads no transcript contents

Command: `sed -n '46,50p' crates/holler-adapter-claude/src/lib.rs`

```text
//! **What it reads and writes on disk.** Whether a transcript file exists under Claude Code's
//! configuration directory, never its contents nor any other file there (the registry files
//! under `sessions/` are not a stable interface and outlive a killed process). The socket files
//! its launch hook writes under the adapter's own socket directory, which the reader trusts only
//! when the directory and the file are the user's alone.
```

### E51: The one place a driver builds its event stream

Command: `sed -n '320,333p' crates/holler-body/src/acp_driver.rs`

```text
pub struct DriverEventStream {
    rx: mpsc::UnboundedReceiver<DriverEvent>,
}

impl DriverEventStream {
    /// Builds a stream over an already-created channel receiver. `pub(crate)`
    /// so a second driver implementation (`http_attach_driver`, issue #194)
    /// can return this same concrete type from its own `prompt()` — the
    /// whole point being that `session_manager::task` (issue #195) can hold
    /// `Option<DriverEventStream>` regardless of which driver produced it.
    pub(crate) fn new(rx: mpsc::UnboundedReceiver<DriverEvent>) -> Self {
        Self { rx }
    }
}
```

### E52: ADR-0021: the present-tense statements #839 wrote that name #850 as future (section 8's single-process paragraph, and the refusals table's row)

Command: `sed -n '611,613p' docs/adr/ADR-0021.md; grep -o 'and `say` to a Claude Code pane is `pane-not-driven` until a body drives it (#850)' docs/adr/ADR-0021.md`

```text
and types nothing (I4). Since #839, `reset --first` on a single process is refused `pane-not-driven` in the plan, before
anything is stopped: no body drives a Claude Code pane until #850, and the hub's gate is the only path a first message
takes (section 11, point 6). A failure once the relaunch entered its act ends with the reconcile step, as a failure after
and `say` to a Claude Code pane is `pane-not-driven` until a body drives it (#850)
```

### E53: The session task's rebind as #839 built it: it attaches an HTTP driver, with the existence check and the refusals every row shares

Command: `sed -n '495,539p' crates/holler-body/src/session_manager/task.rs`

```text
/// The `data.reason` of a rebind that did not finish before its deadline: the
/// old binding is kept (issue #839, ADR 0021 section 11 point 2).
const REBIND_TIMED_OUT: &str = "endpoint did not answer";

/// Re-point this attach-mode session to `harness_session_id` (`session/rebind`,
/// issue #839), fail closed: nothing is written unless the new driver attached
/// (the existence check of `attach`, never `session/new`) before `deadline`.
/// Only then the new id goes to `config.session_id`, the one source every
/// later attach reads, the driver is swapped in, presence advertises the id,
/// and the old driver is shut down (which ends only this body's own loop).
/// An attach still running at the deadline is dropped, never swapped in.
async fn rebind(inner: &mut Inner, harness_session_id: String, deadline: tokio::time::Instant) -> Result<String, WireError> {
    if tokio::time::Instant::now() >= deadline {
        let message = "the rebind's deadline passed before the session took it";
        return Err(WireError::new(Code::NotConnected, message, Some(REBIND_TIMED_OUT)));
    }
    if inner.config.mode != SessionMode::Attach {
        let message = format!("{} is not an attach-mode session", inner.name.as_str());
        return Err(WireError::new(Code::InvalidParams, message, Some("not an attach session")));
    }
    if let Some(busy) = rebind_busy(inner) {
        return Err(busy);
    }
    let config = SessionConfig { session_id: Some(harness_session_id.clone()), ..inner.config.clone() };
    let attached = match tokio::time::timeout_at(deadline, HttpAttachDriver::attach(&config)).await {
        Ok(Ok(driver)) => driver,
        Ok(Err(e)) => {
            return Err(WireError::new(Code::InvalidParams, e.message(), Some("session not found on the endpoint")));
        }
        Err(_elapsed) => {
            let message = format!("the endpoint did not answer for {harness_session_id} before the rebind's deadline");
            return Err(WireError::new(Code::NotConnected, message, Some(REBIND_TIMED_OUT)));
        }
    };
    inner.config.session_id = Some(harness_session_id.clone());
    let old = inner.driver.replace(Driver::HttpAttach(attached));
    mirror_session_id(inner);
    inner.presence.lock().unwrap_or_else(PoisonError::into_inner).omitted_pending_attach = false;
    log_session("rebind", &inner.name, vec![("harness_session_id", harness_session_id.clone())]);
    notify_presence(inner);
    if let Some(old) = old {
        let _ = old.shutdown().await;
    }
    Ok(harness_session_id)
}
```
