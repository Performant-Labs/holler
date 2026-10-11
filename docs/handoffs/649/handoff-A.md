# Handoff-A: Phase 3 - #649 hub wiring and the scratch integration scenario  (up-front plan review, third pass: the A-review amendment and the merge of #642 part 3 and #644)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `3dff970`)
**Brief reviewed:** `docs/handoffs/649-brief.md` at `6c07882`. The merge `3dff970` left it unchanged (sha256 `2da3f9e27edee976...`). The brief blesses the work at `046026f`. It was read against the tree after the merge of `origin/main`: #642 part 3 (#823), #644 (#822) and the #802 follow-ups.   **Reuse map:** no separate file. The brief's Evidence (E1 to E70) and D11 serve as the map, as in the first two passes.   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

This pass replaces the second one (BLOCK on the brief at `0066ac5`, committed in `59433aa`). It reviews the amendment `6c07882`, then what the merge `3dff970` changed under the plan.

## Summary

**Verdict: BLOCK, on one finding (B-2).** The amendment fixes B-1 with the recommended wording and folds in every warn, but it leaves one open question with no owner.

- **What #649 makes live.** Once #649 wires the real harness, `pane reset --first` delivers a real prompt outside the hub's park, hold and health gate. ADR-0021 records this path as a carve-out, and whether it stays is still undecided.
- **Who the plan says decides.** D18 (c), D17 and the "Wiring as built (#649)" paragraph all give that decision to #646.
- **Why that fails.** #646 closed on 2026-10-10 at 12:54 MDT, a day before this brief was written, and the question was never in its scope.
- **The result.** After #649 merges, the bypass is live and no open story owns closing it. F cannot follow D17 and still write a true ADR sentence.
- **The fix.** The operator names an owner, then the brief changes the same pointer in three places. Notes for O gives the wording.

**The rest holds.**

- **B-1 is fixed.** D3 now maps a read the hub accepted and did not answer to `timeout` naming the store method. A new acceptance line pins it.
- **Every warn of the second pass is folded in:**
  - D6 takes W-E;
  - D8 takes W-5 and W-C;
  - D10 (5) takes W-D;
  - D17 takes W-A, W-B and W-C;
  - D18 (e) takes W-J, with #825 open as its follow-up;
  - D21 takes W-G, D22 takes W-I and D28 takes W-H;
  - D31 carries the first pass's binding warns, and D32 is the phase 7 checklist.
- **The merge changes nothing the plan builds on** (listed below the table), and the work the brief blesses type-checks on the merged tree.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-2 | **block** | **The owner of the first message's path**, in four places: D18 (c), "(c) Moving the first message to the hub's prompt path (#646)"; D17's "lines 832-833 (the first message's path is #646's alone, D18c)"; the W-B sentence D17 puts in the new paragraph, "#649 is the story that makes the bypass live and #646 owns closing it (D18c)"; and an ADR sentence #642 part 3 added today, which D17 does not list | choke points; ADRs | **The owner is a closed story.** #646 closed as completed on 2026-10-10 at 12:54 MDT. The brief was first written on 2026-10-11 at 10:31 MDT (`cfcb974`). #646's issue scoped it to the gate at `circuit::dispatch::send_prompt`. Its brief and handoffs (`docs/handoffs/0646-implementation/`) never mention `reset --first` or the first message. **A sentence D17 misses.** #642 part 3 added this to ADR section 8 today: "Since #642 part 3 the adapter sends that first message for real, and the gate still does not see this path; #646 and #649 decide whether it moves" (line 524-525 at `3dff970`). **The bypass.** `tx_switch.rs:218` sends the first message through `ports.harness.send_prompt`, and nothing on that path checks park, hold or health. The hub's gate is the overlay's one prompt choke point (`circuit/dispatch.rs:87` and `:163`). There a `say --pane` to the same parked pane is refused. **This story makes it live.** Before #649, `Unwired` answers `not-implemented`, so the path cannot run. After #649, it posts to a real OpenCode through #642 part 3's `prompt_async`. **The outcome as written.** The ADR would say a closed story owns closing the bypass, and #649 does not decide it either (D18 (c)). So the carve-out becomes permanent by default, and the cutover (#654) and the MO adoption (#653) come next. **Why a block, not a binding warn.** The brief names #646 three times, explicitly, so F has no reading that is both compliant and true. Naming another owner is an operator decision, which A cannot make by a reading (the second pass's B-1 rule). | **Name an open owner.** File a follow-up story for the first message's path (recommended wording in Notes for O). Put its number in D18 (c), in D17's list (both ADR sentences, located by their words) and in the W-B sentence. **Keep D18 (c)'s substance:** #649 neither moves the first message nor decides whether it moves. **Not recommended:** writing "no story owns it yet" into the ADR. That records the gap but does not close it. |
| W-K | warn (fold into the same amendment) | **D17's other deferrals.** Its ADR edits would point three more deferrals at no live owner. | ADRs | **(a) Model, effort and env.** Section 8 says: "The model, effort, environment names and ceilings are recorded, not applied: no port carries them yet (#642, #649; they will join the `ServeSpec`)" (line 547-548). D17 turns this into "#642 only". But #642 closed today at 11:56 MDT, and its S handoff lists "the model and effort" among what it left untouched. **(b) The `--herdr-session` default.** Section 8 says "...the wiring (#649) defaults it from the Herdr adapter's one configured session" (line 569). D5 and D17 record it as deferred, and D18 (b) names no story. **(c) DRIVEN.** D18 (a) gives it to #654, which is open. But #654's body lists no DRIVEN wiring as work: its acceptance only observes "no SHOWN/DRIVEN mismatch". It is also operator-run. | **Fold into the same amendment:** (a) and (b) each name an open owner, or the ADR says plainly that no story owns them yet. (c) Add the DRIVEN wiring to #654's body as work, or file the story #654 depends on. None of these changes a decision of this story. |
| W-L | warn (low; for T and O) | **D15**, and #642's S handoff, advisory 2 | reuse (the one composition that can test it) | **A hand-off the brief predates.** #642's S advisory 2 concerns the `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}` pair as it goes through the host's real `tmux new-window`. It is "sound by reading, but no test runs it. #649 should check it end to end, for example with `GET /config` of a server that `HostPort::run` started". **Only D15's test can.** The adapter's rig may not depend on the host adapter (ADR section 5). **What the WIP does.** It runs D15 only with `ServeSpec::default()` (`scenario.rs:289` and `:298`). With no agent, `verify_serving` sends nothing. | **Extend D15.** The test also runs one server whose `ServeSpec` carries an agent: `plan`, the key #642's real test uses. Then `verify_serving` makes its `GET /config` and `GET /agent` checks against the argv tmux started. Test scope is T's call; this finding records the hand-off so it is not dropped. |
| W-M | warn (low; information for T) | **D12, D19 and the Depends on line** | none (the facts are now settled) | **(a) #642 part 3 is on the branch** (`3dff970`). D19's contingency is met, and "All merged except #642 part 3" is out of date. **(b) D12 calls the dead-end provider's effect on the session "unverified for the real binary".** #642 part 3 has since verified it, in two parts. OpenCode 1.19.0 retries the refused provider (status `retry`) until the turn is aborted (`real_opencode/rig.rs:9-12`; #642 handoff-S, row 11). The adapter reads `retry` as `Busy` (`lib.rs:621`, `running`; `prompt.rs:51`). **For the scenario:** after `reset --first`, C2's activity reads `Busy` until step 6 kills its server. That is the expected outcome, not a D9 finding. D12's rule that the later steps on C2 need no idle session still holds. | **Optional in the amendment:** update D12's and D19's wording. Neither change affects what T or F builds. |

**Checked against the merge, and consistent.** The merge touched only three files of the Blast radius: `real_opencode/rig.rs`, ADR-0021 and `CHANGELOG.md`.

- **`HarnessPort`.** Only a doc comment changed. It still has eleven methods, so D31's "all eleven methods forwarded" holds.
- **`OpenCodeConfig`, `Timeouts` and `TmuxConfig`.** Unchanged, so D7's mapping covers every field.
- **The resolver.** Its one call site is unchanged (`attach.rs:169`), so D8 and the third bounded-otherwise case still hold.
- **The rig.** #649's generalisations of `real_opencode/rig.rs` survive the merge: `CONFIG`, `on_path`, `binaries`, `no_model_can_be_reached`, `lay_out` and `isolated_vars` are `pub`. #642 added `serve_with` and the forwarded `send_prompt` and `session_activity`.
- **`LinkedHost`.** There are still two copies (`pane_verbs/launch/rig.rs:663` and `pane_integration/bind.rs:422`). #644 added no third, so D32 stands.
- **#644's `--agent`.** It changes `launch.rs`, `relaunch.rs` and `args.rs`, all outside the wiring. D5's deferral of the `--herdr-session` default still leaves `launch.rs` untouched.
- **Compilation.** `cargo check --tests` passes on the merged tree for `holler-cli` and the three adapter crates.
- **Protocol.** No wire field or golden file changes, and `holler-hub` gains no dependency.
- **ADR line numbers.** They moved by about 47 lines. D17 now locates each sentence by its words, and each of its sentences is still there.
  - Exception: the line 547-548 sentence's replacement text is now wrong (W-K (a)).
  - The section 8 sentence in B-2 is new.

## Notes for O

**What to amend (required, B-2).**

1. **File a follow-up story** for the first message's path. Recommended title, leading with the better design:

   > feat(pane): the first message of `pane reset --first` goes through the hub's one prompt path, so the park, hold and health gate sees it (ADR-0021 section 8's carve-out)

   Consider making #654 (the cutover) and #653 (the MO adoption) depend on it. Those are the stories that take the bypass to the live fleet and to automated callers.

   **The cheaper option.** The operator decides now that the carve-out is permanent. If so, D17 records that decision in section 8 and in the deferred list, and the overlay's choke-point rule names the exception. Not recommended: it leaves two paths for prompts to reach a body.

2. **Put the follow-up's number (#N) in the brief:**
   - **D18 (c):** "(c) Moving the first message to the hub's prompt path, or deciding that it stays where it is (#N; #646, which built the gate, closed on 2026-10-10 without it in scope)."
   - **D17's list:** both ADR sentences, located by their words, become "#N's to decide":
     - section 8's "...the gate still does not see this path; #646 and #649 decide whether it moves" (new in #642 part 3);
     - the deferred list's "...whether the first message later moves to the hub's one prompt path is #646's and #649's to decide".
   - **D17's W-B sentence:** "...#649 is the story that makes the bypass live and #N owns closing it (D18c)".

**In the same edit (recommended, so the next pass has nothing left to note).**

- **W-K (a) and (b):** name an owner for each, or say "no story owns it yet". **(c):** add the DRIVEN wiring to #654's body, or file the story it depends on.
- **W-L:** extend D15 with one server whose `ServeSpec` carries an agent.
- **W-M:** update D12's "unverified" and the Depends on line.

**Recovery.** Amend the brief, then start a **fresh** run at the plan review. Do not use `resumeFromRunId`: it replays this cached verdict.

**Unchanged from the second pass, for T-red and phase 7.** D32 already holds my notes:

- **RED evidence.** It comes from the rows the trial fails:
  - D4's eager request;
  - D5's dropped fallback;
  - D23's r1c1-first launches;
  - W-E's fail-closed tmux socket;
  - or, failing those, a tree with the trial stubbed out.
- **Phase 7 checks first:** the trial is gone, the manifest comment is in place, `pane_integration` holds no copies, and there is one wire client.
- **The WIP still has the trial's known gaps:**
  - the scenario creates and lists sessions with raw HTTP (`scenario.rs:38` and `:121`), where D21 now says to use `HarnessPort`;
  - the grid test launches into empty workspaces (`:246`), where D23 says r1c1 comes first;
  - the probe `zz_probe_attach_after_run` is still there (`:319`), and D24 says to delete it.

## Patterns referenced

- `docs/adr/ADR-0021.md` at `3dff970`. Section 8 has:
  - the reset engine's park rule (lines 480-486);
  - the prompt and activity seam, with #642 part 3's new sentence (501-526);
  - the unapplied model, effort and env (547-548);
  - the `--herdr-session` default (568-569);
  - #646's prompt gate (609-624);
  - DRIVEN (780-785) and the deferred list (868-891).
- `crates/holler-hub/src/circuit/dispatch.rs`: `pane_state_refusal` (87) and `send_prompt` (163), the choke point. `crates/holler-pane/src/tx_switch.rs:218`: the first message's direct send.
- `crates/holler-adapter-opencode/src/{lib.rs, prompt.rs, attach.rs}` after #642 part 3, and `tests/real_opencode/rig.rs` after the merge.
- The issues and their closures: #646 (closed 2026-10-10 at 12:54 MDT) with `docs/handoffs/0646-implementation/`; #642 (closed 2026-10-11 at 11:56 MDT) with `docs/handoffs/642/handoff-S.md` (out of scope: "the model and effort"; advisory 2); #654 and #825 (open).
- The trial and the WIP at `046026f`, as merged: `src/pane/wiring.rs`, `src/pane/wiring/{store,env,bind}.rs` and `tests/pane_integration/scenario.rs`.
