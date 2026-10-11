# Handoff-A: Phase 3 - #649 hub wiring and the scratch integration scenario  (up-front plan review, re-review after the audit amendment)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `0066ac5`)
**Brief reviewed:** `docs/handoffs/649-brief.md` at `0066ac5` (sha256 `edc4d31dc8b0525d...`), with the work it blesses at `046026f`   **Reuse map:** no separate file. The brief's Evidence (E1 to E70) and D11 serve as the map, as in the first pass.   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

This pass replaces the first one (PASS on the brief at `cfcb974`, committed in `b88fb15`). The audit amendment rewrote D3, D4, D5, D10, D11, D13 and D17 and added D21 to D30, so the plan was reviewed again in full.

## Summary

**Verdict: BLOCK, on one finding.** The audit's new read clause in D3 says every failure of a store read answers `unavailable`. That includes a read the hub accepted and never answered in time. ADR-0021, the frozen port traits and the test kit's fakes all say that case is `timeout`. Merged verb tests already rely on it. The work the brief itself blesses (`046026f`) pins `timeout` too. D17 amends no ADR sentence to cover the change. The fix is one clause in D3; Notes for O gives the wording.

**The rest of the plan is sound.**

- The audit took in the first pass's binding warns:
  - W-1 is now D10 (5) and (6);
  - W-2 is D22;
  - W-3 is D11;
  - W-8 is D1 and D27;
  - W-11 is D24 and D25;
  - W-13 is D5 to D7 and D26.
- The objects and layers are unchanged and still right:
  - the wiring fills #670's `Wiring`, the one construction site of ADR-0021 ruling 1;
  - `StoreScope` and `SystemProber` are reused;
  - no adapter enters `holler-hub`;
  - the store clients sit CLI-side over `holler_pane::reply`.
- D4's eager first watch request gives the client the same at-once refusal of a cursor ahead of the head that the hub's in-process `watch()` has (`panes/store.rs:242-245`). It does this without a new wire method.
- D11's three generalisations parameterise each rig in place, and every default stays. D22 composes `Hub` and `StateDir` without copying them.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-1 | **block** | D3 (audit), the read clause: "for a read (`get`, `list`, `watch`, `log`), every variant is `unavailable` naming the hub" | pattern consistency (the port contract); ADRs | **What D3 says.** A read the hub accepted but did not answer within the call's timeout (`ControlError::Io` of kind `WouldBlock` or `TimedOut`) answers `unavailable`. **What the contract says: `timeout`.** ADR-0021: line 95-96 (every port method returns within I5's bound "or with `timeout`"); line 217 (for a watch, "`timeout` means a wedged store"); line 285 (I5: "a wedged fake port makes the verb exit with `timeout`"); line 666 (`timeout`'s `op` is `<port>.<method>` "in every implementation and fake"); line 743 (section 12). The frozen trait docs say the same: `ports.rs:8-10`, `56-60` and `64-67`, and `profile.rs:324-327`. **The fakes agree.** The test kit's wedged fault answers every op, reads included, with `timeout` (`fault.rs:28-32`). Merged verb tests pin it: `pane list`, `pane get` and `pane watch --until-idle` (`tests/pane_verbs/list.rs:510`), `profile list` (`tests/profile_verbs/list.rs:104`) and `profile show` (`tests/profile_verbs/show.rs:292`) all exit 1 with `timeout` against a wedged store. **The cost.** The real client would answer the same condition with another code than the fake. The conformance rule of ADR section 2 (line 117) exists to prevent exactly that. `pane watch` would also report a wedged hub as one that cannot be reached. **The plan disagrees with itself.** The WIP the Blast radius blesses pins the ADR's answer, not D3's (`tests/pane_integration/fake_hub.rs:17` and `a_read_the_hub_never_answers_is_timeout_within_the_bound`, `:237`). D17 amends no ADR sentence for the change. | **Amend D3's read clause** (wording in Notes for O). Writes stay as written. The amended clause matches the WIP test, the trial's `store.rs` and the fakes. **Not recommended:** keeping D3 as it is. That needs amendments to ADR-0021 (95-96, 217, 285, 666, 743), to the frozen trait docs (under the amend-first rule) and to the test kit's wedged model. It also needs changes to three verb tests. All of these are outside the Blast radius. |
| W-A | warn (binding on F; inside the Blast radius) | D17's list of ADR-0021 sentences | ADRs | **A missed sentence.** Lines 489-492 end "...not of its DRIVEN session, which until #649 is not kept in step with it". That still gives #649 the DRIVEN work that D18 (a) moves to #654. D17 lists 180-181, 511, 522, 733-738, 785, 824 and 832-833, but not this one. | **Amend it** with the other DRIVEN sentences: point it at #654. |
| W-B | warn (binding on F; carried from the first pass's W-7 (b) and W-8, not restated in D17) | D17's "Wiring as built (#649)" paragraph; D12; D18 (c); D1 | choke points; ADRs | **The first message.** With the real harness wired, and once #642 part 3 has merged, `pane reset --first` delivers a real prompt through `HarnessPort::send_prompt`. That path bypasses the hub's prompt path, so it also bypasses the hub's park and hold gate. ADR line 489-491 records this seam on purpose, and `tx_switch.rs` checks no park state before it sends. #649 is the story that makes the bypass live. **The prompt verbs.** `say`, `interrupt` and `answer` with `--pane` now make D2's status exchange and read the real registry (E58). | **The paragraph says both.** First, the first-message seam is live from this story, and #646 owns closing it (D18 (c)). Second, the `--pane` path of the prompt verbs now runs over the real wiring. |
| W-C | warn (binding on F; documentation only) | D8 (b) and (c), the resolver's registry and Herdr lookups | I5 (bounds) | **When the resolver runs.** The OpenCode adapter calls the resolver after it has fixed its method's deadline (`attach.rs:169`, in `attach_tui`, `select_session` and `shown_session`). **What it costs.** When the bindings (a) miss, (b) is one `pane/list` with a 10 s bound. (c) is one Herdr snapshot, with a 10 s bound plus the lazy adapter's connect ping on a first use. So a TUI method that resolves through (b) or (c) can run past its own 10 s bound when the hub or Herdr is slow. **Precedent.** ADR section 2 already records two methods that are "bounded otherwise", `edit_spec` and `run_probe` (line 96). | **Keep (a) first**, so a launch never nests a port call (D8). **Record the third case** in the paragraph, beside section 2's two: such a TUI method is bounded by its own bound plus those port calls. **Add no knob** (D9). |
| W-D | warn (binding reading of D10 (5)) | D10 (5): the binary "gets exactly `HOLLER_STATE_DIR`, ... and the logging variables, nothing inherited" | identity and secrets (scratch only) | **The list leaves out `PATH` and the scratch `HOME` and `XDG_*`.** **Without `PATH`:** the binary's `TmuxHost` finds `tmux` and `kill` on `PATH` (E27), and so does the OpenCode `TmuxConfig` for `tmux`. With no `PATH`, they fall back to libc's default search path. **Without `HOME`:** a child process that wants a home falls back to the account's real one (`getpwuid`). **What already holds it.** The first pass's W-1 (b) gave the binary the scratch `HOME` and `XDG_*`. The WIP's `binary_env` does too, pinned by `the_binary_env_is_built_from_nothing_inherited` (`guards.rs`). | **Read "exactly" as including** a `PATH` the test chooses and the scratch `HOME` and `XDG_*`. The `PATH` holds the directories of `tmux` and `opencode`, then `/usr/bin:/bin`. **This is what the WIP does**, and nothing is inherited. Write both into D10 (5) when the brief is amended. |
| W-E | warn (binding on F and T; carried from the first pass's W-13) | D6; the trial at `046026f` | fail closed (cross-cutting) | **D6 is right; the trial is not.** For a malformed `HOLLER_TMUX_SOCKET`, the trial gives the harness `TmuxSocket::Default`, which is the operator's live tmux server (`src/pane/wiring.rs:65`). It also makes the host `Unwired`, which answers `not-implemented` (`:63`). **Why the tests miss it.** The WIP's environment tests check `env::tmux_socket` on its own and one store-only verb. Neither covers this path. | **For F:** the host and the harness each answer `usage` naming `HOLLER_TMUX_SOCKET` at first use. Neither falls back to the default socket. **For T:** one default-run test pins this over the wiring (or over `connect` with an injected lookup). A host or harness call with a relative value answers `usage` and never reaches the default socket. |
| W-F | warn (binding; carried from the first pass) | The first pass's W-4, W-5, W-6, W-9 (dispatch and op names) and W-12 | pattern consistency; reuse | **Status.** The amended brief does not restate these warns, and nothing in it contradicts them. **The WIP honours each one.** **(1) W-4:** `WiredHarness` forwards all eleven `HarnessPort` methods, and a test pins it. **(2) W-5:** the resolver matches the configured Herdr session and the pane id, and refuses two matches as `unavailable`. **(3) W-6:** `clients.rs` runs both store conformance suites against a real hub. **(4) W-9:** `store.rs` sends through `transport::call(None, ..)` with the test kit's op names. **(5) W-12:** the two decorators run the Herdr and host suites. | **They stay binding** on F's rewrite of the trial and on T. **D3's dispatch.** D3's "by `holler_hub::control::run`" is met through `transport::call(None, ..)`. That is MO 8's one dispatch point, and today it is the only caller of `control::run` in the CLI. |
| W-G | warn (low) | D21: raw HTTP in the scenario | layering (another crate's non-interface) | **The boundary.** `holler_adapter_opencode::http` is "not an interface: code outside the crate reaches OpenCode only through `HarnessPort`" (`lib.rs:96-99`). **Where the scenario crosses it.** It goes through the included rig's `raw` for `DELETE /session/:id`; no port deletes a session, so that use is needed. In the WIP it also does so for `POST /session` and `GET /session`, which `HarnessPort::create_session` and `list_sessions` already do. | **Use raw HTTP only where no port can.** That means the deletes and the model guard's providers read, through the rig's `raw`, and never through a second client. **Use the port for the rest.** Create and list sessions through an `OpenCodeHarness` built in the test, as D15's test does. |
| W-H | warn (low; for T) | D28, the observable for "touches no adapter" | testability | **The claim.** D28 says "any adapter use would fail differently from `unavailable`". **Where it fails.** That holds for Herdr, which answers `usage` (D5). It does not hold for tmux or OpenCode: both answer `unavailable` when absent (D7). | **Observe it as the WIP's `no_hub.rs` does.** The message names the missing hub and its state dir. The traps stay untouched: a held Herdr listener, a tmux socket path, and an `opencode` that leaves a mark when run. |
| W-I | warn (low) | D22: `StateDir::at` in `tests/support/mod.rs` | size and structure | **The size.** The file is 849 lines on `main` and 857 with the WIP, past the overlay's line of about 800. D22 names the one constructor it adds. | **Nothing else lands there** in this story. The target's own helpers stay in `pane_integration/` and `process/main.rs`. |
| W-J | warn (low; follow-up for O) | D1: the scope's actor, `holler pane` | naming; pattern consistency | **The pattern.** Each profile verb logs its own actor (`holler profile create`, `holler profile delete`). **What D1 does.** Every `--profile` edit made by a pane verb logs `holler pane`. So the profile log cannot tell `launch --profile` from `close --profile`. `connect()` takes no argument (D1), so a per-verb actor would need another seam. | **Accept it for this story.** A follow-up can let the verb name its actor. |

**Checked and found consistent**, beyond the table:

- **The protocol.** No wire field changes and no golden file changes. `ControlCall`'s fields are public, and `say_cmd.rs:157-166` already sets one. So a CLI-side `ControlCall` literal in `store.rs`'s one `call` needs no new constructor in `holler-hub`, which is outside the Blast radius.
- **`profile/rename`.** The hub routes it to `profile/rename.rs` (`profile/mod.rs:188`), whose `not-implemented` comes back as a `PaneReply`, so it passes through `into_result` as D3 says.
- **#642 part 3 (PR #823).** It leaves `OpenCodeConfig` unchanged. It edits `real_opencode/rig.rs` in hunks other than #649's, so whichever merges second adapts, as the first pass's W-3 said.
- **No adapter in the hub.** `holler-hub` gains no dependency (D16).

## Notes for O

**What to amend (required, B-1).** Replace D3's read clause with:

> For a read (`get`, `list`, `log`, and each watch request), an `Io` whose kind is `WouldBlock` or `TimedOut` (the hub took the request and did not answer within the call's timeout) is `timeout` naming the store method (`pane_store.get`, `pane_store.list`, `pane_store.watch_next`, `profile_store.get`, `profile_store.list`, `profile_store.log`, `profile_store.watch_next`), as ADR-0021 sections 2 and 12 and the test kit's wedged fault have it. Every other variant of a read is `unavailable` naming the hub.

Leave the write clause as it is.

**In the same edit (recommended, so the next pass has nothing left to note):**

- **W-D:** add the test-chosen `PATH` and the scratch `HOME` and `XDG_*` to D10 (5)'s list.
- **W-A:** add ADR line 489-492 to D17.
- **W-B and W-C:** name both in D17's paragraph.
- **W-5:** in D8 (b), add "in the adapter's configured Herdr session".
- **W-H:** reword D28's observable for tmux and OpenCode.

**Recovery.** Amend the brief, then start a **fresh** run at the plan review. Do not use `resumeFromRunId`: it replays this cached verdict.

**Noticed, for T-red.** The trial in `src/pane/wiring/` already passes many of the WIP's tests. RED evidence must therefore come from one of two places:

- the rows the audit changed, which the trial fails:
  - D4's eager first request;
  - D5's dropped `HERDR_SESSION` fallback;
  - D23's grid workspaces whose r1c1 is launched first;
  - W-E's fail-closed tmux socket;
- or from a tree with the trial stubbed out.

**Phase 7 checks first, once F has rewritten the trial:**

- **The trial is gone:**
  - no `TRIAL` header;
  - no `eprintln!` (the trial prints `TRIAL ...` lines to stderr from `bind.rs`);
  - no `Unwired` field in `Wiring`;
  - no `HERDR_SESSION` fallback;
  - no fallback to the default tmux socket;
  - D4's eager first request is in place.
- **The manifest** carries a comment naming #649 on the three adapter dependencies (D16).
- **No copy in `pane_integration`** of any of these:
  - the Herdr guards;
  - the rig's dead-end config, model guard, port picker and `on_path`;
  - the private tmux server;
  - `Hub` and `StateDir`.
- **`LinkedHost`.** The one in `pane_integration/bind.rs` is a second small copy of the launch rig's glue (`tests/pane_verbs/launch/rig.rs:663`). It is acceptable as test-local glue; a third copy moves to the test kit.
- **One wire client.** No `UnixStream` in `holler-cli/src` outside `holler_hub::control`, and no new direct caller of `control::run`.
- **One implementation per port** in the wiring, and every delegating impl forwards every method.

## Patterns referenced

- `docs/adr/ADR-0021.md`:
  - section 2: the bound and its two exceptions (95-99), the conformance rule (117), and `Watch` (215-218);
  - the I5 row (285);
  - section 5: ruling 1 and the crate rules (290-310);
  - section 8: the first-message seam (484-492);
  - the error table's `timeout` and `unavailable` rows (666-667);
  - section 12 (740-766).
- `crates/holler-pane/src/ports.rs` and `profile.rs`: the frozen contracts. `crates/holler-pane-testkit/src/fault.rs`: the wedged fault.
- The merged verb tests that pin `timeout` for a wedged store:
  - `crates/holler-cli/tests/pane_verbs/list.rs:510`;
  - `tests/profile_verbs/list.rs:104`;
  - `tests/profile_verbs/show.rs:292`.
- `crates/holler-hub/src/control.rs` (`send_over`, `ControlError`, the `_at` variants) and `crates/holler-cli/src/transport.rs` (MO 8's dispatch point), with `say_cmd.rs:150-176` as the precedent for the `NoLiveHub` text and the field set.
- The work in the tree at `046026f`:
  - `src/pane/wiring.rs` and `src/pane/wiring/{store,env,bind}.rs` (the trial);
  - `tests/pane_integration/*`;
  - the three rig generalisations;
  - `holler-adapter-opencode/src/attach.rs:160-176`, the resolver's call site.
