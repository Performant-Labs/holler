# Decisions — #649 hub wiring and the integration scenario with the real adapters against a scratch Herdr, tmux and OpenCode

## A (Phase 3, up-front plan review) — 2026-10-11T10:52:22-06:00
- **Decided:** PASS on `docs/handoffs/649-brief.md` at `cfcb974` (sha256 `d746deeb057f7858...`), with no blocks and fifteen warns, eleven of them binding. The full detail is in `handoff-A.md`.
  - **The objects and the layers are right.**
    - #670's `Wiring` is filled; it is ADR-0021 ruling 1's one construction site.
    - `StoreScope` and `SystemProber` are reused.
    - The hub gets no adapter, as ADR section 5 requires.
    - The store client sits CLI-side, over `holler_pane::reply`'s types.
    - The resolver meets the precondition the OpenCode adapter leaves to wiring.
    - No new object has an analogue it should extend instead.
  - **W-1 (binding):** a fifth scenario guard. The test starts the private tmux server itself, with a scratch config and only the rig's isolated pairs (scratch `HOME`/`XDG_*` holding the dead-end config, `PATH`, `TERM`, the `OPENCODE_DISABLE_*` flags). The binary runs with the same scratch `HOME`/`XDG_*`. Reason: D7's `Inherit` makes the tmux server's environment the only source of OpenCode's environment.
  - **W-2 (binding; Blast radius exception approved):** the three stub tests of `tests/pane_verbs/process/stub.rs` run against a live hub. D2 and AC 1 turn their no-hub `not-implemented` into `unavailable`.
  - **W-3 (binding reading of D11; Blast radius exception approved):** each included rig is generalised in place by parameters or visibility, never copied. The files that may change for this purpose only:
    - `holler-adapter-herdr/tests/scratch_herdr/mod.rs`;
    - `holler-adapter-opencode/tests/real_opencode/rig.rs`;
    - `holler-adapter-host/tests/real_tmux_test.rs`, plus a new `tests/real_tmux/mod.rs`;
    - `holler-cli/tests/support/mod.rs` (one `StateDir` constructor).
  - **W-4 (binding):** every delegating `HarnessPort` overrides `send_prompt` and `session_activity`, so #642 part 3 is not masked.
  - **W-5 (binding):** the resolver's registry lookup and the bindings use the join key, the Herdr session and the pane id.
  - **W-6 (binding):** D13's round trips are the test kit's `PaneStore` and `ProfileStore` conformance suites over the clients against a real hub.
  - **W-7 (binding):** seven ADR-0021 sentences that give #649 work D5, D7 and D18 now defer are amended in the same change.
  - **W-9 (binding):** one mapping from `ControlError` to `PaneError`, split into reads and writes; send through `transport::call(None, ..)`.
  - **W-11 (binding):** child modules under `src/pane/wiring/`, declared from `wiring.rs`. They count as `wiring.rs` for the Blast radius; `pane/mod.rs` is untouched.
  - **W-12 (binding):** D8's pin runs in the default run, and the decorators run the `HerdrPort` and `HostPort` suites.
  - **W-13 (binding):** configuration errors appear at first use and fail closed; a relative `HOLLER_TMUX_SOCKET` never falls back to the default server.
  - **Not binding:**
    - W-8: `say_cmd.rs:252` is a third caller of `connect`;
    - W-10: in-process tests race on `HOLLER_STATE_DIR`;
    - W-14: the launch budget, and the dependency on #642 part 3;
    - W-15: small points.
- **Assumed:**
  - **tmux environment.** A new tmux session gets the server's global environment, plus from the creating client only the `update-environment` variables (tmux's documented behaviour). So the environment the server starts with is what the OpenCode processes inherit. I did not probe it here.
  - **OpenCode config sources.** OpenCode reads its global config, auth and session store through `XDG_CONFIG_HOME` and `XDG_DATA_HOME`, as the adapter rig and the spike assume. It also reads a project-level `opencode.json` in the working directory. I did not probe either.
  - **Herdr pane ids.** Herdr numbers pane ids within a session (`w<N>:p<M>`, spike line 44), so two sessions can share one. This is inferred from the format, not probed.
  - **#642 part 3.** It overrides `send_prompt` and `session_activity` in `OpenCodeHarness`, as its brief and ADR-0021 section 8 say. It has not merged.
  - **Opt-in runs.** T or F may run the opt-in scenario on the operator's machine during this run. That is why W-1 is binding.
- **Hedged:**
  - **W-1 is a warn, not a block.** D12, AC 4 and the story's rules already require the outcome. The finding supplies the mechanism and changes no decision.
  - **W-2, W-3 and W-11 approve Blast radius exceptions** (the #651 precedent) instead of blocking for an amendment. Each is the only way to meet a recorded decision: D2 with AC 1, D10 with D11, and the 900-line lint.
  - **W-5 narrows D8 (b)'s literal text.** It drops only records the adapter could not serve anyway.
  - **W-6's CI cost** (tens of seconds per suite, from `WATCH_WAIT` and the suites' drains) is estimated, not measured.
  - **W-14's budget risk** is not measured.
- **Evidence:**
  - **Read in full:**
    - the brief and its outside review `649-brief-result-r1.md` (PASS, NV-1 to NV-19);
    - `holler-cli/src/pane/wiring.rs`, `src/main.rs`, `src/transport.rs`;
    - `holler-pane/src/ports.rs`;
    - `holler-hub/src/control.rs`;
    - `holler-adapter-herdr/tests/scratch_herdr/mod.rs`;
    - `holler-adapter-opencode/tests/real_opencode/rig.rs`;
    - `holler-cli/tests/pane_verbs/process/{stub,legacy_verbs}.rs`;
    - `holler-cli/src/pane/watch.rs`;
    - `holler-pane-testkit/src/conformance/mod.rs`.
  - **Read in part:**
    - `holler-pane/src/error.rs` (395-714) and `tx_launch.rs` (560-831);
    - `holler-adapter-opencode/src/lib.rs` (1-469) and `attach.rs` (1-120);
    - `holler-adapter-herdr/src/{lib,adapter}.rs`;
    - `holler-adapter-host/tests/real_tmux_test.rs` (1-122);
    - `holler-cli/src/say_cmd.rs` (225-300);
    - `holler-cli/tests/support/mod.rs`;
    - `holler-hub/src/{state,panes/mod}.rs`;
    - ADR-0021 (92-520, 728-842);
    - `docs/testing.md` (439-449);
    - `docs/research/herdr-api-spike.md` (40-46, 395-402);
    - `agent-skills/herdr-setup/lib/herdr.sh` (84-95);
    - `holler-cli/Cargo.toml` (380-522).
  - **Greps:**
    - every `impl <Port> for`;
    - every caller of `Wiring::connect` and `Unwired`;
    - `control::run` callers;
    - `NoLiveHub` renderings;
    - `#[path = "../../..` includes;
    - every `#649` in ADR-0021;
    - `HOLLER_TEST_*` hooks in the hub;
    - the run-conformance call sites.
  - **Parallel branches:** the Blast radii of #642 and #644 (`git show issue-642-implementation:docs/handoffs/642-brief.md`, and the same for #644).

## A (Phase 3, up-front plan review, re-review after the audit amendment) — 2026-10-11T11:51:08-06:00
- **Decided:** BLOCK on `docs/handoffs/649-brief.md` at `0066ac5` (sha256 `edc4d31dc8b0525d...`), together with the work it blesses at `046026f`. One block and ten warns. The full detail is in `handoff-A.md`, which replaces the first pass's.
  - **B-1 (block): D3's new read clause.** It maps every failure of a store read to `unavailable`, including a read the hub accepted and did not answer in time.
    - **What it contradicts.** ADR-0021 says that case is `timeout`: lines 95-96, 217, 285 (I5), 666 and 743. So do the frozen trait docs (`ports.rs`, `profile.rs`). The test kit's wedged fault answers it with `timeout`, and three merged verb tests rely on that. The brief's own blessed WIP test pins `timeout` too (`fake_hub.rs:237`).
    - **What is missing.** D17 amends no ADR sentence for the change.
    - **The fix.** One clause: a read whose `Io` is `WouldBlock` or `TimedOut` is `timeout` naming the store method.
  - **The rest holds.** The first pass's W-1, W-2, W-3, W-8, W-11 and W-13 are now decisions in the brief. W-4, W-5, W-6, W-9 and W-12 stay binding; the brief does not restate them, and the WIP honours them.
  - **New warns:**
    - W-A: ADR line 489-492 is missing from D17;
    - W-B: the paragraph records that `reset --first` now delivers a real prompt outside the hub's gate, and that the `--pane` prompt verbs run over the wiring;
    - W-C: the resolver's nested port calls exceed a TUI method's bound;
    - W-D: D10 (5)'s binary environment needs a test-chosen `PATH` and the scratch `HOME` and `XDG_*`;
    - W-E: the trial falls back to the default tmux socket for the harness; D6 forbids it;
    - W-G: raw HTTP only where no port can;
    - W-H: D28's observable does not tell tmux or OpenCode from the hub by code;
    - W-I: `tests/support/mod.rs` is past 800 lines;
    - W-J: the scope's single actor (a follow-up).
- **Assumed:**
  - **The read timeout surfaces as `WouldBlock` or `TimedOut`.** `send_over` sets a read timeout and maps the `read_line` error to `ControlError::Io`. On Linux an expired `SO_RCVTIMEO` is `EAGAIN`, so the kind is `WouldBlock`. The WIP's hang test exercises it; I did not run it.
  - **The audit's reasoning.** It is not in the tree or on the issue (no comments). I read D3's read clause as written: "every variant".
  - **Linux defaults.** A child process with no `HOME` falls back to `getpwuid`. A `Command` run with no `PATH` searches libc's default path. Both are standard Linux behaviour; I did not probe either.
- **Hedged:**
  - **Why B-1 is a block, not a binding warn.** D3's "every variant" is unambiguous, and A cannot override an operator decision by a reading. The two ways out differ in reach: conforming D3 is one clause, while amending the ADR touches files outside the Blast radius. So the operator decides.
  - **Readings, not overrides.** W-D reads D10 (5)'s "exactly" as allowing what the test sets. W-F keeps the first pass's binding warns.
  - **W-C's worst case** is estimated from the bounds, not measured.
- **Evidence:**
  - **Read in full:**
    - the amended brief, and its diff from `cfcb974`;
    - the trial `src/pane/wiring.rs` and `src/pane/wiring/{store,env,bind}.rs`;
    - `tests/pane_integration/{main,clients,fake_hub,bind,env,guards,no_hub,store_only,watch,scenario}.rs` and `scratch/mod.rs`;
    - `holler-adapter-host/tests/real_tmux/mod.rs`;
    - `holler-hub/src/control.rs`, `holler-cli/src/transport.rs` and `holler-hub/src/profile/rename.rs`.
  - **Read in part:**
    - `ports.rs` (1-120) and `profile.rs` (322-332);
    - `holler-pane-testkit/src/fault.rs`;
    - the diffs of the three rig generalisations, and of `process/{main,stub}.rs` and `support/mod.rs`;
    - ADR-0021 (88-120, 205-222, 280-330, 484-513, 655-680, 738-770, 780-834);
    - `holler-adapter-opencode/src/{lib.rs (86-104, 211-232), attach.rs (155-200)}`;
    - `pane/watch.rs` (60-95) and `panes/store.rs` (225-266);
    - `profile_scope.rs` (135-215), `reconcile/observe.rs` (160-180) and `say_cmd.rs` (150-260);
    - the wedged-store verb tests (`pane_verbs/list.rs:500-525`, `profile_verbs/{list.rs:95-125, show.rs:290-302}`);
    - `pane_verbs/launch/rig.rs` (`LinkedHost`);
    - `scripts/lint.sh`.
  - **Greps:**
    - `UnixListener` in tests and sources;
    - `ControlCall {` and `ControlCall::` in the CLI;
    - `PaneError::Timeout` consumers;
    - `Fault::Wedged` in the CLI tests;
    - every `#649` in ADR-0021;
    - `const ACTOR`;
    - `pub struct Out`;
    - `LinkedHost`.
  - **Line counts:** every file the work touches.
  - **#642 part 3 (PR #823):** its changes to `rig.rs`, `ports.rs` and `OpenCodeConfig` (unchanged).
  - **The issue:** its body equals the brief's, except that the issue carries an index in place of the Evidence section.

## A (Phase 3, up-front plan review, third pass: the A-review amendment and the merge of #642 part 3 and #644) — 2026-10-11T12:12:43-06:00
- **Decided:** BLOCK on `docs/handoffs/649-brief.md` at `6c07882` (sha256 `2da3f9e27edee976...`). The merge `3dff970` left the brief unchanged. I read it against the merged tree. One block and three warns. The full detail is in `handoff-A.md`, which replaces the second pass's.
  - **The amendment is right.** B-1 is fixed in the recommended wording, and a new acceptance line pins it. W-A to W-J are folded in. D31 and D32 carry the first pass's binding warns and the phase 7 checklist.
  - **B-2 (block): the first message's path has a closed owner.**
    - **What the plan says.** D18 (c), D17's list item and the W-B sentence of the new ADR paragraph give "whether the first message moves to the hub's prompt path" to #646.
    - **What is true.** #646 closed on 2026-10-10 at 12:54 MDT, before the brief was written. The question was never in its scope (its issue, brief and handoffs). #642 part 3 added an ADR sentence today ("#646 and #649 decide whether it moves") that D17 does not list.
    - **Why it matters.** #649 makes `reset --first` deliver real prompts outside the hub's park, hold and health gate (`tx_switch.rs:218` against `circuit/dispatch.rs:87,163`). As written, no open story would own closing that.
    - **Why A cannot fix it.** F cannot follow D17 and stay truthful. Naming an owner is the operator's call.
  - **W-K:** three more deferrals D17 points at no live owner:
    - model, effort and env would become "#642 only", but #642 closed today and lists them as untouched;
    - the `--herdr-session` default names no story;
    - #654's body does not list the DRIVEN wiring as work.
  - **W-L:** #642's S advisory 2 asks #649 to run the `OPENCODE_CONFIG_CONTENT` pair through a real `tmux new-window`. D15's test is the one place that can, if it also runs a spec with an agent.
  - **W-M:** facts the merge settled. #642 part 3 is on the branch, so D19's contingency is met. OpenCode 1.19.0 leaves a dead-end prompt's session in `retry`, which reads as `Busy`, so D12's "unverified" is settled.
  - **The merge changes nothing the plan builds on:**
    - `HarnessPort` still has eleven methods;
    - `OpenCodeConfig`, `Timeouts` and `TmuxConfig` are unchanged;
    - the resolver's call site is unchanged;
    - the rig's #649 generalisations survive;
    - there are still two copies of `LinkedHost`;
    - the tree type-checks.
- **Assumed:**
  - **The operator does not know #646 is closed.** Three sources assume it is open, so this is a stale pointer, not a deliberate choice:
    - the brief's W-B text, adopted today from my second pass;
    - #642 part 3's new ADR sentence;
    - #642's handoff-A.
  - **No owner exists elsewhere.** No open issue owns the first message's path or the model, effort and env. I grepped open issue titles and #654's body and comments, but did not read every open issue's body.
  - **#649 makes the path live.** It is the first story where `reset --first` can run in production: before it, `Unwired` answers `not-implemented` at the first port call.
- **Hedged:**
  - **Why B-2 is a block, not a binding warn.** The brief names #646 three times, explicitly. By the second pass's rule, A cannot replace an operator-adopted pointer by a reading. Also, the fix needs an outward act (filing an issue) or an operator decision (a permanent carve-out). The overlay's first rule (one prompt choke point) is what is at stake.
  - **The recommended owner** is a follow-up story that routes the first message through the hub's prompt path, and #654/#653 depending on it is a suggestion. The operator may instead decide now that the carve-out is permanent; the handoff lists that as the cheaper option.
  - **W-L is about test scope, which is T's.** I raised it only because it is an upstream hand-off to this story that the brief predates.
- **Evidence:**
  - **Read in full:**
    - the amended brief and its diff from `0066ac5`;
    - the trial `src/pane/wiring.rs` and `src/pane/wiring/{store,env,bind}.rs`;
    - `tests/pane_integration/scenario.rs`.
  - **Diffs read:** the merge `6c07882..3dff970` for `ports.rs`, the OpenCode adapter's `lib.rs`, `tui.rs` and `server.rs` (signatures), `real_opencode/rig.rs` and ADR-0021.
  - **Read in part:**
    - ADR-0021 at `3dff970`: 478-526, 540-569, 595-626, 776-790, 826-836, 864-891, and every `#649` mention;
    - `docs/handoffs/642/{handoff-S.md (255-300), decisions.md (300-322), handoff-A.md}`;
    - `tx_switch.rs` and `circuit/dispatch.rs` (greps);
    - `prompt.rs:51`.
  - **GitHub:**
    - #642, #646, #654, #684 and #825: states and closing times;
    - #646's body and comments, and #654's body and comments, for the first message and DRIVEN;
    - #647: its state, and PR #701 (doctor, part 1);
    - the open issue list (titles).
  - **Greps:**
    - `docs/handoffs/0646-implementation/` for the first message;
    - every `impl <Port> for` in the CLI tests (the `LinkedHost` count);
    - `OpenCodeConfig`, `Timeouts` and `TmuxConfig` fields;
    - `HarnessPort`'s methods (11);
    - `tui_session` call sites.
  - **Build:** `cargo check --tests -p holler-cli -p holler-adapter-opencode -p holler-adapter-host -p holler-adapter-herdr` on `3dff970`, clean. I ran no tests.

## A (Phase 3, up-front plan review, fourth pass: the third-pass amendment and the merges of #664 and #650) — 2026-10-11T12:36:29-06:00
- **Decided:** PASS on `docs/handoffs/649-brief.md` at `451e2b3` (sha256 `afe88dc4916d7e09...`), read against the tree at `d0c1531`. The merges `f59ba21` (#664) and `d0c1531` (#650) left the brief unchanged, and the issue's body matches it from Scope to Pipeline. No blocks and five warns, three of them binding. The full detail is in `handoff-A.md`, which replaces the third pass's.
  - **B-2 is closed.** #828 is open (filed 12:14 MDT) and scoped to the first message's path. The brief names it in D17 (both ADR sentences), D18 (c), the W-B sentence and the Depends on line.
  - **The rest of the third pass is resolved:** W-K (b) and (c) through D18 (b) and #654's comment (12:14 MDT), W-L through D15's second spec, and W-M. W-K (a) is half done (W-N (a)).
  - **W-N (binding, F): three ADR edits.**
    - (a) Lines 547-548 name #829, not D17's "#642 only".
    - (b) Lines 568-569 use D5's wording: the flag is required by `launch` and needed by `pane import` and by apply to create a pane. D18 (b)'s "every verb that reaches Herdr" is too broad.
    - (c) The Consequences bullet (916-917, "never a real Herdr, tmux or OpenCode") is amended for the opt-in scratch scenarios.
  - **W-O (binding, phase 7): a third `LinkedHost`.** #664 added `profile_verbs/apply/rig.rs:636`, which triggers D32's "a third copy moves to the test kit"; the test kit is outside the Blast radius. Restated: #649's copy stays as test-local glue, no fourth copy, and O files the consolidation that #664's gate asked for.
  - **W-P (binding, T): two no-hub rows no longer parse.** `EVERY_VERB`'s `["profile", "apply"]` and `["pane", "import"]` need `NAME` (#664) and `--from` (#650). Bare, clap exits 2 before `connect`. These failures are not RED evidence. D2's stub list is out of date; D22 is unchanged.
  - **W-Q (F's paragraph and O's follow-ups):**
    - apply's creates and relaunches write P through `StoreScope`, so they log `holler pane`, outside #825's premise;
    - apply's observation bound (`tx_apply.rs:40-50`) meets W-C's case through `shown_session`;
    - import reports a `--herdr-session` that differs from `HOLLER_HERDR_SESSION` as `pane-unlabelled` for every pane.
  - **W-R (low):** `docs/testing.md` has no section for the new opt-in gate, and the precedent is mixed.
  - **The merges change nothing the plan builds on:**
    - `ports.rs` is untouched, so `HarnessPort` still has eleven methods;
    - every sentence D17 lists is still in ADR-0021;
    - neither merge sends a prompt;
    - the WIP's store clients, `LazyHerdr` and bindings are safe for apply's threads and sequential launches;
    - import calls no TUI method;
    - the tree type-checks.
- **Assumed:**
  - **Binding readings reach F and T through this handoff**, as the first pass's did before D31 carried them into the brief.
  - **The adapter's snapshot covers only its configured session.** I read this from `adapter.rs:388-406` (one `SessionSnapshot` request on the configured socket, each pane stamped with the configured session); I did not run it.
  - **No consolidation issue exists for `LinkedHost`.** I read the titles of the open issues from #800 up, not every open issue's body.
  - **Clap rejects the two bare rows.** I inferred exit 2 from the required arguments (`apply.rs:55`, a `String` positional; `import.rs:83`, a `String` long option). I ran no test.
- **Hedged:**
  - **Why W-N is a binding warn, not a block like B-2.** The brief itself names #829 twice (D18 (f), Depends on), so F has a reading that is both compliant and true. A chooses between two operator statements and overrides neither. Items (b) and (c) only add wording, inside the Blast radius.
  - **W-O restates A's own note (D32),** which A may do. It changes no operator decision, and the test kit's docs prescribe test-local wrappers.
  - **W-Q (b)'s worst case** is estimated from the bounds, not measured.
  - **W-R is not binding:** `docs/testing.md` documents #640's gate but not #642's.
  - **This review spans a merge.** `d0c1531` landed while I was reviewing (12:24 MDT); I re-checked everything above against it.
- **Evidence:**
  - **Read in full:**
    - the brief at `451e2b3`, and its diff from `f59ba21`;
    - `pane_integration/no_hub.rs`;
    - `wiring/bind.rs` and `wiring/store.rs` (the trial);
    - `tx_apply/observe.rs` (1-190);
    - `pane_verbs/process/stub.rs` (1-120, plus the #650 diff).
  - **Diffs read:**
    - the stats of `be87a09..f59ba21` and `451e2b3..d0c1531`;
    - both merges' changes to ADR-0021, ADR-0003, `stub.rs` and `cli-surface.txt`.
  - **Read in part:**
    - ADR-0021 at `d0c1531`: 180-190, 205-222, 478-572, 810-828, 862-934, and every `#649` and `#646` mention;
    - `profile/apply.rs` (1-120) and `pane/import.rs` (60-100);
    - `tx_apply.rs` (36-62), `tx_apply/run.rs` (44-70, 106-126) and `tx_launch.rs` (462-468, 570-578);
    - `import/observe.rs` (1-40);
    - `profile_scope.rs` (170-215);
    - `attach.rs` (greps of the resolver and deadline);
    - `adapter.rs` (254-266, 388-418);
    - `holler-pane-testkit/src/{host.rs (1-40), harness.rs (1-30)}`;
    - the three `LinkedHost` copies;
    - `pane_integration/main.rs` (1-40);
    - `scripts/lint.sh` (1-56);
    - `docs/testing.md` (greps);
    - `docs/handoffs/664/{handoff-A.md, handoff-A-dup.md}` (greps).
  - **GitHub:**
    - #828 and #829: bodies, states and creation times;
    - #654: its comment;
    - #825, #653 and #803: bodies;
    - #667 and #650: states;
    - open issues from #800 up: titles;
    - #649's body, compared with the brief.
  - **Greps:**
    - every `impl <Port> for` and `struct LinkedHost` in `crates/`;
    - `herdr_session` fields in the CLI's verbs;
    - `ports.scope`, `edit_spec` and actor constants in `holler-pane` and the CLI;
    - TUI and harness calls in the import engine;
    - `allow(dead_code)` in the test trees;
    - line counts of every file the work touches.
  - **Build:** `cargo check --tests -p holler-cli -p holler-pane -p holler-adapter-opencode -p holler-adapter-host -p holler-adapter-herdr`, clean at `f59ba21` and at `d0c1531`. I ran no tests.

## T (Phase 4, author / RED) — 2026-10-11T13:05:00-06:00
- **Decided:** RED is valid: 8 of 58 default-run tests fail against the trial of `046026f`, each on a row D32 names. The rows are D4's eager first request (3 tests), D5's dropped `HERDR_SESSION` fallback (3 tests), D5's zero-extent and repeated-label checks (1), and D6/W-E's fail-closed tmux socket (new `wired.rs`, 1). The full list and outputs are in `handoff-T-red.md`.
  - **W-P applied:** the no-hub rows of `pane import` and `profile apply` carry arguments that parse. D28's `PATH` there is an empty directory.
  - **Scenario rewritten to the brief:**
    - sessions are made and listed through the test's own `HarnessPort` (W-G); only deletes are raw;
    - D12's `Busy` is asserted after `reset --first`;
    - D21's breakage gets one `doctor` that finds both;
    - D23's r1c1-first launches;
    - D15 runs a second spec, with an agent (W-L);
    - D28's sentinel is planted and grepped in output, files and `ps`;
    - D10 (6)'s TERM/KILL teardown.
  - **The probe** `zz_probe_attach_after_run` is deleted.
  - **D11 repair:** the WIP's move of the private tmux server broke `holler-adapter-host`'s default test `fake_tmux_test::hosts_are_built_by_one_helper…`. The one `TmuxHost::new` is back in `real_tmux_test.rs`; `real_tmux/mod.rs` builds only servers. `fake_tmux_test.rs` is untouched.
  - **The binary's environment** in the scenario now carries a test-chosen `LANG=C.UTF-8`. This is A's W-D reading: chosen, not inherited.
  - **`pane switch` in the scenario is sent again on `timeout`**, for up to 20 s, with the tries printed.
  - **The redundant test** `an_empty_watch_batch_is_idle…` is merged into the eager-request test.
- **Assumed:**
  - **D21's "pids tmux list-panes (and HostPort::ps) give for C2's session"** means the server's pids: the `@holler-pid` tags of the windows `run` made, intersected with `ps`. Killing every pane of the session would also kill the TUI's pane, and could destroy the session, which is a different breakage from "its server down".
  - **`ANTHROPIC_API_KEY`, `OPENAI_API_KEY` and `OPENROUTER_API_KEY`** are the names to plant D28's sentinel under.
- **Hedged:**
  - **Two findings outside the Blast radius, for O to file.**
    - (1) The OpenCode adapter's TUI query fails without a UTF-8 locale: tmux writes its tabs as `_`, so every `attach_tui` reads "no tmux session". Reproduced with plain tmux; `-u` or `LANG` fixes it.
    - (2) A `pane switch` right after `pane relaunch` times out until the relaunched TUI accepts `/tui/select-session`. Seen 3/3 in the scenario and 2/3 in a probe; one sent 8 s later always succeeds.
  - **The scenario works around (2) with a bounded retry** and does not hide it. If O or the operator wants the scenario strict instead, it fails until that race is fixed elsewhere.
  - **Not reformatted.** Local rustfmt reports diffs across the whole crate, and CI runs no fmt check.
  - **One flake.** `real_opencode_test::send_prompt_queues…` failed once in a long `--ignored` run and passed alone.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_integration`: 50 passed, 8 failed, 4 ignored, run twice.
  - `HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1`: 4/4 pass against the trial, run three times for the main scenario. No `/tmp/hlr649-*` directory and no `hlr649` process left.
  - `cargo clippy -p holler-cli --test pane_integration -- -D warnings` and `-p holler-adapter-host --tests`: clean. `bash scripts/lint.sh`: exit 0.
  - `cargo test -p holler-cli --test pane_cli_process`: 36/36. `-p holler-adapter-host`: all pass, `real_tmux_test --ignored` 9/9. Herdr scratch `--ignored`: 2/2.
  - Probes, temporary and removed: tmux `display-message` with and without `LANG`, and the switch after relaunch with and without an 8 s wait.

## F (Phase 6, implement) — 2026-10-11T13:30:43-06:00
- **Decided:** the trial of `046026f` is rewritten to the house style under D24's names, and `connect` is the real one. All 58 default-run tests pass, T's 8 RED rows included, and the 4 opt-in scenarios pass on scratch instances. The full detail is in `handoff-F.md`.
  - **`store.rs`:** a table of 12 `Method` rows (wire name, `op`, read or write, bound) over D24's `call`, the one D3 mapping. Its `write` is an `Effect` enum, not a bool. Params are the typed `holler_pane::reply` structs.
  - **The watch is eager (D4):** `watch()` makes the first request and keeps the batch for the first `next()`.
  - **`env.rs`:** no `HERDR_SESSION` fallback. Zero extents, repeated labels, empty items and labels holding `=` are `usage` naming `HOLLER_HERDR_WORKSPACES`. Every path a variable gives must be absolute.
  - **`bind.rs`:** one generic `Wired<P>` serves both W-4's harness and W-E's fail-closed host. `WiredHarness` (D24's name, same `new`) and the new `WiredHost` are aliases of it.
  - **The bindings:** each failed `ensure_pane` offers no id. An `ensure_session` claims the offer whatever its outcome, and binds only on success.
  - **The resolver:** (b) and (c) both use the join key (session and pane id). An unconfigured session matches no record.
  - **ADR-0021:**
    - the "Wiring as built (#649)" paragraph in section 5;
    - D17's sentence edits and W-N (a) to (c);
    - a section 2 pointer for W-C's third bounded-otherwise case.
  - **The rest:** the CHANGELOG entry, and the manifest comment (D16).
- **Assumed:**
  - **Every `ensure_session` follows its own `ensure_pane`.** The engine calls the two only in `tx_launch::bring_up`, back to back (grep, evidence). A future caller that calls `ensure_session` alone binds nothing, which is safe.
  - **An unconfigured Herdr session never matches a record.** The resolver then gets `""`, and no launch or import writes a record with an empty Herdr session. The predicate also refuses `""` explicitly.
  - **The suites' watch cases tolerate the eager form.** I read them: each case either drains, or reads the first `next()` before its write. Both suites passed against a real hub, which confirms it.
- **Hedged:**
  - **Environment edges D5 to D7 do not decide**, each recorded as a deviation:
    - an empty session is none;
    - every path must be absolute, D6's rule applied to all;
    - XDG rules for `XDG_CONFIG_HOME`;
    - the empty workspace list is none;
    - relative `PATH` entries are skipped;
    - no serve-args value is quoted.
    Each refuses or is stricter, and none falls back.
  - **The workspace run stopped at one failure that is not this phase's.** `wizard_scripts`' rename scan hits the brief's E64 line and A's first-pass decisions line, both pipeline scratch that Phase 11 deletes. So the crates after `holler-cli` were run separately with `--no-fail-fast`.
  - **Two tests T may want to add:** the empty workspace list, and the absolute-path rule for the socket and binary variables (in `handoff-F.md`).
- **Evidence:**
  - `cargo build -p holler-cli` and `cargo clippy -p holler-cli --all-targets -- -D warnings`: clean.
  - `cargo test -p holler-cli --test pane_integration`: 58 passed, 0 failed, 4 ignored.
  - `HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1`: 4 passed in 57 s. No scratch directory or process was left.
  - `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: 1607 passed, and 1 failed (the scratch scan above), then it stopped.
  - The rest, `--no-fail-fast` on `holler-hub`, `holler-load-test`, `holler-pane`, `holler-pane-testkit` and `holler-proto`: 702 passed, 0 failed.
  - Doc-tests: ok. `docs_cli_test`, after the ADR edit: 3/3.
  - `scripts/lint.sh`: exit 0. `changelog-check`: ok. `cargo machete`: clean. `rustfmt --check` on the four wiring files: clean.
  - Read:
    - `control.rs` (the client, 1-140 and 419-519) and `transport.rs`;
    - `ports.rs`, `reply.rs` and `error.rs`;
    - `profile_scope.rs`;
    - the adapters' public constructors and configs;
    - `attach.rs` (the resolver's call site);
    - `tx_launch.rs` (600-720);
    - both store conformance suites' watch cases;
    - ADR-0021 whole.

## T (Phase 7, verify GREEN + Tier 2), 2026-10-11

- **Decided:**
  - **GREEN, no blocking issue.** The `pane_integration` default run is 59/0/4 and the opt-in run 4/0. Every Phase-4 RED row passes.
  - **Added two of F's suggested rows, in `env.rs`.** One new test pins the absolute-path rule for `HOLLER_HERDR_SOCKET`, `HERDR_SOCKET_PATH` and `HOLLER_OPENCODE_BIN`, and that a bad `HOLLER_HERDR_SOCKET` does not fall through to `HERDR_SOCKET_PATH`. The existing rows gained the empty session and the empty workspace list. Each is documented operator-facing behaviour (the wiring's module-doc table), and each is a cheap unit row.
- **Assumed:**
  - **The `wizard_scripts` rename-scan failure is out of scope.** Its only hits are under `docs/handoffs/`, which Phase 11 removes before the push. `git grep` outside that directory finds nothing.
- **Hedged:**
  - **`with_env` mutates the process environment.** It is safe today under the state-dir lock, but recorded as an advisory.
- **Evidence:**
  - Mutations M1 to M7 (lazy watch, a lost write mapped to `unavailable`, a zero extent, relative paths, the `HERDR_SESSION` fallback, no binding offer, the host falling back to `Unwired`): each was caught by the named test, then restored with `git checkout`.
  - `cargo test --workspace --no-fail-fast -- --skip roster_…`: 2309 passed, 1 failed (the scratch scan), 32 ignored.
  - Clippy `-D warnings`, `docs_cli_test` 3/3, `wire_selftest` 3/3, `lint.sh`, `changelog-check` and `cargo machete`: all clean.

## A (Phase 3, up-front plan review, fifth pass: the re-entry after the diff gate's round-1 BLOCK) — 2026-10-11T13:59:29-06:00
- **Decided:** PASS on `docs/handoffs/649-brief.md` at `451e2b3` (sha256 `afe88dc4916d7e09...`), read against HEAD `1c05f75`. No blocks and four warns. The full detail is in `handoff-A.md`, which replaces the fourth pass's.
  - **Why phase 3 ran again.** The outside diff gate returned BLOCK at 13:47 MDT (round 1). F had reported `archChanged`, so the driver routed here, the drift re-check, without classifying the findings.
  - **The plan holds.** The brief is unchanged, and `origin/main` is still `5d366b9`. #828 and #829 are open. #654's body edit at 13:34 MDT is to the cutover procedure only.
  - **F's change has no drift.**
    - `Wired<P>` extends D24's `WiredHarness` to W-E's host: one wrapper, not two.
    - W-4 holds for all three decorators.
    - D3 and W-9 are one mapping, through `transport::call(None, ..)`.
    - The no-hub text matches the CLI's own pattern.
    - The ADR edits match D17 and W-N.
    - The rigs are included by `#[path]`. There is no fourth `LinkedHost`.
  - **W-S (binding on F and T): the round-1 BLOCK.**
    - Its B-1 (a `get` of a missing record) and B-2 (`{}` for `list`) are false. `Option<T>` decodes `null` as `None`, and the hub's `NoParams` is "only `{}`".
    - Conformance Case 1 pins B-1, and both suites pass against a real hub.
    - F changes no code for either. F adds the refuting facts to `evidence.md`, which round 2 receives through `--evidence`.
    - The reviewer's own WARN (the 10 s bound pinned only from above) and its nits are T's call.
  - **W-T (binding on F, low): a value that is not UTF-8 reads as unset.**
    - `connect`'s `std::env::var(name).ok()` makes it `None`.
    - So a non-UTF-8 `HOLLER_TMUX_SOCKET` falls back to tmux's default server, which D6 forbids for a malformed value.
    - The fix is one line: read such a value as malformed.
  - **W-O is carried (no fourth copy).**
  - **W-U (for O to file in Aftersight):**
    - on the `archChanged` route, and on a `production` classification, the diff gate's findings reach neither T nor F;
    - phase 4's task text says "before any code exists";
    - a phase-4 BLOCK on an unchanged brief stops the run.
- **Assumed:**
  - **F reported `archChanged: true`.** I inferred this from the route: `nextPhase(7, 'BLOCK')` reaches phase 4 only then. F's handoff has no "Architecture changed" line.
  - **T-red and F read `handoff-A.md` on this route.** The run-context block tells every phase to read prior handoffs, and names this file.
  - **`std::env::var` answers `Err(NotUnicode)` for a value that is not UTF-8.** This is the standard library's documented behaviour. I did not run a probe.
- **Hedged:**
  - **Why PASS and binding warns, not BLOCK.** With the brief unchanged, a BLOCK stops the run as `needs-amendment` at the next phase-4 entry (`decideArchReReview`), and nothing in the brief needs amending. W-S and W-T go to F through this handoff instead.
  - **W-S binds evidence, not code.** This is the one way round 2's reviewer sees the source it asked for (NV-1, NV-2, NV-4, NV-5).
  - **W-T is A's reading of D6 and the first pass's W-13.** D6 does not name the non-UTF-8 case. The scenario cannot reach it, because D10 (4) gives the binary ASCII paths.
  - **The #654 edit.** I read it as a diff of the body, and did not read #652's guide.
- **Evidence:**
  - **Read in full:**
    - `wiring.rs` and `wiring/{store,bind,env}.rs` at HEAD;
    - `649-diff-result-r1.md`;
    - `handoff-F.md`, `handoff-T-green.md` and `evidence.md`.
  - **Diffs read:** ADR-0021 `d0c1531..HEAD`; the story's file list and line counts against `origin/main`.
  - **Read in part:**
    - the driver: `coding-pipeline.workflow.mjs` 1095-1215, 3690-3708 and 5376-5720;
    - `workflow-coding-pipeline.md` 1104-1112, 1765-1800 and 1875-1890;
    - `panes/handlers.rs` 37-116 and `panes/mod.rs` 180-215;
    - `profile/handlers.rs` 37-39 and `reply.rs` 46-117;
    - `ports.rs` (the trait methods);
    - conformance `pane_store.rs` 120-131 and `profile_store.rs` 185-196;
    - `fake_hub.rs` 41-52, 215-300 and 377-398;
    - `wired.rs` 1-98 and `main.rs` `with_env`;
    - the no-hub text in `say_cmd.rs` and `roster_cmd.rs`;
    - `fake_tmux_test.rs` 785-797 and `scratch/mod.rs` (the `#[path]` lines, 94 and 406).
  - **Ran:** `cargo test -p holler-cli --test pane_integration clients::` at `1c05f75`: 2 passed, 0 failed, 66.41 s.
  - **GitHub:**
    - #828, #829, #825 and #654: all open;
    - #654's body edit of 13:34 MDT, diffed against the version before it;
    - #825's comments: none;
    - open issues mentioning `LinkedHost`: only #649.
  - **Greps:**
    - `struct LinkedHost` in `crates/`;
    - the `support::` helpers in `pane_integration`;
    - `#[path` includes;
    - the `TmuxHost::new` sites;
    - personal names in the added lines;
    - `diff-result` in the role docs and the driver;
    - the evidence argument of the diff gate.
  - **`git fetch origin`:** `HEAD..origin/main` is empty.

## T (Phase 4, author / RED, second cycle: the re-entry after the diff gate's round-1 BLOCK) — 2026-10-11T14:05:56-06:00

- **Decided:**
  - **W-T is pinned as a second row of D6's test**, `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server`, with the absolute value `/tmp/hlr649-\xff/tmux.sock`. It fails at HEAD on its assertion: the host's `ensure_session` succeeds on tmux's default server in the `TMUX_TMPDIR` trap.
  - **The two rows share one body** (`assert_the_socket_is_usage_never_the_default_server(&OsStr)`) rather than a copy; `with_env` takes `AsRef<OsStr>` values to carry the non-UTF-8 one.
  - **The trap kills any tmux server a regression starts in it** (`kill_trapped_servers`), because the RED itself starts one.
  - **All three optional rows taken.** The 6 s lower-bound pin (`fake_hub::a_read_and_a_write_answered_after_6_s_succeed`, the reviewer's W-1); NIT-2 (`herdr_snapshot`'s labels come from `WORKSPACES`); NV-6 (the sentinels are restored on drop, not removed).
  - **No RED for B-1 or B-2**, per A's W-S.
- **Assumed:**
  - **F takes A's form of the fix** (a non-UTF-8 value becomes empty, which `absolute` refuses as `usage`). The row asserts the outcome (`usage` naming the variable), so any fix with that outcome passes; a lossy conversion would not.
- **Hedged:**
  - **The 6 s row is a regression pin, not a RED:** it passes at HEAD. A local mutation (`CALL_TIMEOUT` = 5 s, reverted with `git checkout` at once) shows it fails then, so it discriminates.
  - **The 6 s row holds the state-dir lock for about 12 s** (two sequential replies). It adds that much to the target's wall time; the default run took 119 s.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_integration`: 60 passed, 1 failed (the W-T row), 4 ignored, 119.18 s.
  - Mutation run of the 6 s row: `the read waits past 5 s: Err(Timeout { op: "pane_store.get" })`.
  - `cargo clippy -p holler-cli --all-targets -- -D warnings` clean; `bash scripts/lint.sh` exit 0.
  - Opt-in `scenario::the_two_grid_spellings…` on scratch instances: 1 passed, 15 s; no `/tmp/hlr649-*` or `hlr649` process left.
  - After the RED run: the pid `host.ps` reported (778017) is gone; no tmux process names the trap.

## F (Phase 6, implement, second cycle: the re-entry after the diff gate's round-1 BLOCK) — 2026-10-11T14:25:57-06:00
- **Decided:**
  - **W-T, as A sized it.** `connect`'s lookup is `std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())`.
    - A value that is not UTF-8 is the empty string, never `None`.
    - T-red's row `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server` passes, and the trap stays untouched.
  - **The two empty-value messages now say "empty or not UTF-8":** `absolute`'s and the no-session one.
    - So an operator whose value is not UTF-8 is not told it is `""`, or that none is set.
    - No test pins either text (`git grep`).
  - **Docs:**
    - `connect`, `Lookup`, both module docs, and one ADR sentence in "Wiring as built (#649)" (D17);
    - the first cycle's ambiguous rustdoc link `[`env`]` is now `[`mod@env`]`.
  - **W-S: eight entries in `evidence.md`**, under "The source that round 1 of the diff gate asked for":
    - B-1: `PaneReply` and `into_result`; serde's `Option` and `serde_json`'s `deserialize_option` (library source at the locked versions); conformance Case 1 in both suites;
    - B-2: `NoParams`, `params_of`, `decode_params`' doc, and the nine params structs of `holler_pane::reply`;
    - NV-4: `HarnessPort`'s eleven methods and its two defaults;
    - NV-5: `pane/cas_put`'s membership check;
    - and the Herdr adapter's refusal of an unlisted workspace, which backs the docs' workspace sentence.
  - **No code change for B-1 or B-2** (A's W-S).
  - **`archChanged: false`.** The change is a local closure, two private messages and docs: no public item, type, module or dependency.
- **Assumed:**
  - **`OsString::into_string` fails exactly when the value is not UTF-8.** This is std's documented behaviour, and T's row confirms it for the socket.
  - **Round 2's diff gate attaches none of the new citations.**
    - Its 24 KB excerpt cap is used up by the brief's own citations: round 1 omitted 27 cited files.
    - So each entry quotes the lines it relies on.
- **Hedged:**
  - **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace, the same as unset, not as `usage`.**
    - It fails closed: the adapter refuses every launch before any request of its own (evidence).
    - Covering it would need either an in-band marker that every reader checks, or a new `Lookup` type that breaks D24's signature and T's tests. Neither is worth it for this edge.
    - It is recorded in the docs and as a known issue.
  - **`XDG_CONFIG_HOME`, `HOME` and `PATH` read as before.** A non-UTF-8 `XDG_CONFIG_HOME` reads as empty, which the XDG rule takes as unset (`$HOME/.config`). It matters only when neither socket variable is set.
  - **The message changes go beyond A's one line.** They change no tested behaviour.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_integration`, final code: 61 passed, 0 failed, 4 ignored. `wired::`: 2 passed. After the last doc-only edits, `wired::`, `env::` and `no_hub::`: 17 passed.
  - Opt-in, `--ignored --test-threads=1`: 4 passed in 82 s. No scratch directory or process was left.
  - `cargo test --workspace --no-fail-fast -- --skip roster_…`: 2312 passed, 1 failed, 32 ignored. The failure is the rename scan over `docs/handoffs/`, as in both earlier cycles.
  - Clean: clippy `-D warnings`, `docs_cli_test` 3/3, `wire_selftest` 3/3, `lint.sh`, `changelog-check`, `cargo machete` and `rustfmt --check`. Rustdoc reports no diagnostic in a wiring file.
  - The evidence quotes: each quoted line checked against its file by a script, and the line ranges read with `cat -n`.
  - **Read:**
    - `handoff-A.md` (fifth pass) and `handoff-T-red.md`;
    - `649-diff-result-r1.md`, and its prompt's evidence and excerpt sections;
    - `dual-review.sh` (the excerpt cap and `--evidence`) and the driver's `buildDiffGateCaptureTaskPrompt`;
    - T's `wired.rs`, `env.rs` and `main.rs` (`with_env`).

## T (Phase 7, verify GREEN, second cycle: the re-entry after the diff gate's round-1 BLOCK) — 2026-10-11T14:38:21-06:00
- **Decided:**
  - **GREEN, no blocking issue.** The W-T row and the 6 s pin pass. The default run is 61/0/4 and the opt-in run 4/4.
  - **One table value added:** `""` in `env::the_serve_args_are_a_json_array_of_strings_default_none`.
    - F's new docs say the empty value (which is how a non-UTF-8 value reads) is `usage` for every variable but the workspaces.
    - The serve arguments' empty value was the one such case with no test.
  - **No second process-level non-UTF-8 row.** The one closure in `connect` is pinned by the W-T row, and `env.rs` pins each reader's empty value.
- **Assumed:**
  - CI runs no `rustfmt`, as `ci.yml` and `lint.sh` show. So the test tree's existing bare-rustfmt drift in `support/` is not this story's.
- **Hedged:**
  - A non-UTF-8 `HOLLER_HERDR_WORKSPACES` is left as F's known issue: it fails closed, but the message does not name the variable. Advisory, not blocking.
- **Evidence:**
  - **M8**, a lossy conversion in `connect`: the W-T row FAILED (`host.ensure_session … got Ok(())`). The file was restored, and `git status` was clean.
  - **Workspace** (`--no-fail-fast`, CI's skip): 2312 passed, 1 failed, 32 ignored. The failure is the rename scan, whose only hits are under `docs/handoffs/`.
  - **Clean:** clippy `-D warnings`, `lint.sh`, `changelog-check`, `cargo machete`, `docs_cli_test` 3/3, `wire_selftest` 3/3, and `rustfmt` on `wiring.rs`.
  - **Leftovers:** none after the opt-in and mutation runs (no `/tmp/hlr649-*/`, no `hlr649` process).

## T (Phase 7, verify GREEN, third cycle: test-only rework after the diff gate's round-2 findings) — 2026-10-11T14:49:25-06:00
- **Decided:**
  - **GREEN, no blocking issue.** Both round-2 findings say the code "will not compile", and the build refutes both. `cargo build -p holler-cli --all-targets` and clippy `-D warnings` are clean, and the D8 test passes.
  - **Test-side fix: evidence, not a rewrite.** The D8 test was correct. The reviewer could not see `run_verb_with`, `Ports`, the `Send + Sync` supertraits or the fakes' constructors. Four verbatim entries were added to `evidence.md`, plus a two-line comment in the test saying why the profile store, the scope and the prober are there.
  - **`wiring.rs` closure: no change, and none was made (it is production code).** `move` moves the owned `Result` into the closure, so the closure is `'static`. F should answer the finding in prose.
- **Assumed:**
  - The opt-in run need not be repeated, because no production or harness code changed since its 4/4 at `22aa539`.
- **Hedged:**
  - The round-3 reviewer may again lack source for the closure. The closure is in the diff, so this is left as an advisory for F rather than a T entry.
- **Evidence:**
  - Default run: 61/0/4.
  - Workspace: 2312 passed, 1 failed (the rename scan, scratch only), 32 ignored.
  - `docs_cli_test` 3/3 and `wire_selftest` 3/3; `lint.sh`, `changelog-check` and `machete` clean.
  - The excerpts were checked against source by script. One range was corrected from 356-368 to 356-367.

## F (Phase 6, implement, third cycle: the production half of the mixed rework after the diff gate's round-2 BLOCK) — 2026-10-11T15:00:59-06:00
- **Decided:**
  - **B-2 (`wiring.rs`, `from_env`'s `LazyHerdr` closure) needs no code change.** The closure is
    `move || connect_herdr(&herdr_config)`. It owns the `Result<HerdrConfig, PaneError>` it captures, so it is
    `'static`, as `LazyHerdr::new` requires. The build is clean.
  - **A two-line comment at the closure states that rule.** Round 3's reviewer sees the diff and `evidence.md`, never
    the handoff, and a third consecutive diff-gate BLOCK stops the run. No behaviour change.
  - **One evidence entry for B-2:** the declarations of `HerdrConfig` and `PaneError`, neither with a lifetime or type
    parameter.
  - **`evidence.md` reordered newest first; every entry carried over verbatim.** The gate shows only the first 12,000
    bytes. In the old order, round 3 would have seen neither T's B-1 entries nor the B-2 entry.
  - T's section is unchanged in content, and moved to the top. Within round 1's section, the B-2 entries now come
    before the conformance entry.
- **Assumed:**
  - Round 3 is assembled as round 2 was: `--evidence <handoff dir>/evidence.md`, the default 12,000-byte cap, no
    `--handoff`, and source excerpts used up by the brief's own citations first. This is read from the driver and
    `dual-review.sh`, and from round 2's prompt.
- **Hedged:**
  - A fresh round-3 reviewer may raise something new. The first cycle's ten entries are now past the cap, though six
    of them repeat the brief's own Evidence.
  - The tail of the 12,000-byte window is a partial conformance excerpt. Round 2 had the same, with no finding on it.
- **Evidence:**
  - **Mutation:** with `move` removed, `cargo check -p holler-cli` fails `E0373` ("closure may outlive the current
    function, but it borrows `herdr_config`"). The file was restored with `git checkout`, and `git status` was clean.
  - **Clean at the staged tree:** `cargo build -p holler-cli --all-targets`; `cargo clippy --workspace --all-targets
    -- -D warnings`; `rustfmt --check` on `wiring.rs`; `scripts/lint.sh` (exit 0; the only warning is the existing
    one on `holler-pane/src/error.rs`).
  - **Workspace** (CI's command, with `--no-fail-fast`): 151 binaries; 2312 passed, 1 failed, 32 ignored.
    `pane_integration` is 61/0/4. The failure is the rename scan, whose hits are only under `docs/handoffs/`.
  - **Evidence quotes:** each checked by script against its file and line. The new file was rendered through the
    gate's `awk` cap to place the cut.
  - **Nothing lost:** a sorted-line diff between the old and the new `evidence.md` shows only the replaced title and
    intro removed.
  - **Read:**
    - `649-diff-result-r2.md` and its prompt's evidence, diff and excerpt sections;
    - `dual-review.sh` (`build_evidence_section`, `EVIDENCE_MAX_BYTES`, `DEFAULT_EXCLUDES`, `--handoff`);
    - the driver's diff-gate loop, `shouldEscalate` and `PER_GATE_BLOCK_THRESHOLD`;
    - `handoff-T-green.md` and `handoff-T-red.md`.

## T (Phase 7, verify GREEN, fourth pass: F's third cycle) — 2026-10-11T15:12:01-06:00
- **Decided:**
  - **PASS.** F's only production change is a two-line comment in `wiring.rs`. No code line differs, and the suite is
    GREEN at `4d7f58a`.
  - **No test change.** F flagged none. A comment has no behaviour to pin, and the compiler pins the `move`.
- **Assumed:**
  - The opt-in `scenario::*` run need not repeat, since no code line changed after `22aa539`, where it passed 4/4.
- **Hedged:**
  - Round 3 of the diff gate may still ask for first-cycle facts that are now past the 12,000-byte evidence cap. That
    is a gate-input gap for O to file, not a defect in the code.
- **Evidence:**
  - `git show 4d7f58a -- crates/`: two added comment lines.
  - Repeated F's mutation: without `move`, `cargo check -p holler-cli` fails `E0373`. Restored with `git checkout`,
    and the tree was clean.
  - The B-2 excerpts match `adapter.rs:73-74` and `error.rs:402-403`. A `LC_ALL=C` sorted-line diff of `evidence.md`
    (`13010f4` against HEAD) loses only the old title and intro.
  - Workspace: 151 binaries; 2312 passed, 1 failed (the rename scan, `docs/handoffs/` hits only), 32 ignored.
    `pane_integration` 61/0/4, `docs_cli_test` 3/3, `wire_selftest` 3/3. Build, clippy `-D warnings`, `lint.sh`,
    `changelog-check` and `machete` are clean.

## A (Phase 8, anti-duplication gate) — 2026-10-11T15:26:46-06:00
- **Decided:** PASS on the diff `5d366b9..c13400a`. No blocks and four low warns, none binding on this run. The full
  detail is in `handoff-A-dup.md`.
  - **No parallel path.**
    - F filled #670's `Wiring` in place and reused `StoreScope`, `SystemProber`, `transport::call(None, ..)`,
      `holler_pane::reply` and the adapters' constructors.
    - The new objects are D24's names, plus `Wired<P>` and the `Lookup` alias.
    - The store clients, the environment readers, the bindings and the resolver have no analogue to extend.
  - **The tests reuse what D11 and D32 name:** the three rigs by `#[path]`, generalised in place, and `support`'s
    `Hub`, `StateDir` and `wait_for`. There are still three `LinkedHost`s, and no fourth.
  - **The warns:**
    - W-1: the scratch module's `alive` is a third copy of the host adapter tests' `kill -0` helper.
    - W-2: `real_tmux::tmux_on_path` repeats the rig's `on_path`; `isolated` could take the tmux binary.
    - W-3: `wired.rs`'s `trap_dir` copies `no_hub.rs`'s OpenCode trap, and `Scratch::harness`'s resolver is
      `rig::sessions_for(&[])`.
    - W-4: the client's `Feed<E>` is the third instance of the hub's private, twinned poll iterator.
    - W-O is carried: the `LinkedHost` consolidation is still unfiled.
- **Assumed:**
  - **The brief's D11, D24, D31 and D32 are the reuse map**, as in every A pass of this run. There is no `survey.md`.
  - **T-green's workspace run covers D11's "default-run tests pass unchanged".** I re-ran no test. `c13400a`
    changes only the brief, so the tree is the one T-green verified.
- **Hedged:**
  - **W-1 to W-3 are warns, not blocks.**
    - Each is a few lines of test glue, not one of the overlay's helpers and not a production object.
    - W-1 follows from D11's own limit on edits to the adapter test directories.
    - W-2 was in the blessed trial `046026f`, and the host adapter's tests cannot reach the OpenCode rig.
  - **W-4 is a warn, not a block.**
    - The map does not name the hub's twins.
    - They are private to `holler-hub`, which is outside the Blast radius (D16).
    - The client's version adds D4's eager first request.
- **Evidence:**
  - **Read in full:**
    - `wiring.rs` and `wiring/{store,bind,env}.rs`;
    - `pane_integration/{main,bind,fake_hub,no_hub,store_only,wired,guards,env,clients,watch,scenario}.rs` and
      `scratch/mod.rs`;
    - `real_tmux/mod.rs`;
    - `handoff-A.md` (fifth pass), `handoff-F.md`, `handoff-T-red.md`, `handoff-T-green.md` and
      `649-diff-result-r3.md`.
  - **Diffs read:** `scratch_herdr/mod.rs`, `real_opencode/rig.rs`, `real_tmux_test.rs`, `support/mod.rs` and
    `pane_verbs/process/{main,stub}.rs`.
  - **Read in part:**
    - the hub's `panes/store.rs` (270-340) and `profile/store.rs` (330-385);
    - `import/assess.rs` (140-200) and `holler-body/src/query.rs` (280-305);
    - `pane_verbs/launch/rig.rs` (640-800) and `transport.rs`;
    - the rig's `on_path`, `signal_group`, `config` and `sessions_for`.
  - **Greps:**
    - every `impl <Port> for` in `crates/`;
    - `struct LinkedHost`, and every `start_server(` here and at the merge base;
    - `PaneReply` users; `split_paths`, `XDG_CONFIG_HOME` and the no-hub text in `src`;
    - `same_pane` and the label joins;
    - `alive` and `kill -s 0` helpers; `UnixListener` in the tests;
    - `control::run` and `ControlCall {` in `holler-cli/src`;
    - re-declared `support` helpers, and `thread::sleep` in `pane_integration`;
    - env-lock helpers;
    - `TRIAL`, `eprintln!` and `"HERDR_SESSION"` in the wiring.
  - **Git:**
    - `git fetch origin`: `origin/main` is `4366fb8` (#830, `docs/research/` and `scripts/spikes/` only);
    - the Blast radius checked against `git diff --name-only 5d366b9 HEAD`;
    - `git diff 4d7f58a c13400a -- crates/` is empty.
  - **GitHub:** `gh issue list --search LinkedHost --state all` finds only #649 and the closed #802.

## S (Phase 10, spec audit) — 2026-10-11T15:46:09-06:00
- **Decided:**
  - **REWORK, test-only.** No `src/` change is needed. The full detail is in `handoff-S.md`.
  - **The production code meets every decision of the brief.** Fourteen of the 15 acceptance criteria are met, three of
    them with an advisory. AC 3 is partly met.
  - **Two rows are missing in `pane_integration/fake_hub.rs`:**
    - (1) D3's row "a write with no hub is `unavailable`" is unpinned, though D31 W-9 binds T to pin every row and the
      module doc claims each is pinned. A mutant that adds `NoLiveHub` to `timed_out`'s write arm survives.
    - (2) AC 3 says every other failure of a read is `unavailable` *naming the hub*. The lost-reply and JSON-RPC read
      rows assert only the code.
  - **One item blocks the merge, for O, not F or T:** CI's `wizard_scripts` rename scan fails on three tracked lines
    under `docs/handoffs/` that quote the setup wizard's pre-#820 directory path. Those are the brief's E64 Source
    (stale since #820), `decisions.md:66` and `handoff-T-green.md:48`. The release keeps handoffs on the branch unless
    `removeHandoffsBeforePr: true` (Aftersight #558), and `main` shows this repo keeps them.
- **Assumed:**
  - The opt-in scenario's 4 of 4 at `22aa539` still stands. Since then, `crates/` changed only by comments and one
    default-run table value. I did not re-run it.
  - This run's PR phase does not pass `removeHandoffsBeforePr`. The CLAUDE.md template omits it, and #650 and #664
    landed their handoffs on `main`.
- **Hedged:**
  - **D28's sentinel check is nearly vacuous** under D10's exact environments: the binary and the tmux server never
    hold the sentinel. AC 15 holds by construction, because the wiring reads no credential variable. I kept this as an
    advisory with a recommendation for #829, not an ADVISORY-HOLD.
  - The scenario's C1 `doctor` assertion and its `pane get`-based end state are advisories, not REWORK.
- **Evidence:**
  - **Read in full:**
    - the brief at `c13400a`;
    - every handoff in `docs/handoffs/649/`;
    - the three outside-gate results;
    - `wiring.rs` and `wiring/{store,env,bind}.rs`;
    - every file of `pane_integration/`;
    - `real_tmux/mod.rs`;
    - the diffs of the three adapter rigs, `support/mod.rs`, `pane_verbs/process/{main,stub}.rs`, ADR-0021,
      `CHANGELOG.md` and `Cargo.toml`.
  - **Read in part:**
    - `holler-hub/src/control.rs` (`send_over` and the error display);
    - `transport.rs`;
    - `reconcile.rs` and `findings.rs` (strays carry no pane);
    - `skilltext_w5_rename.rs`;
    - `docs_cli_test.rs`;
    - the release's PR-phase handoff removal (`coding-pipeline.workflow.mjs:4734-4754`) and `pipeline-conventions.md`
      §1.
  - **Checks (read-only):**
    - `git merge-tree` against `origin/main` `4366fb8`: clean;
    - `wc -l` on every touched `.rs` file: maximum 579;
    - grep of the added lines for `allow(`, panics, prints, private names, IPs and key shapes;
    - `git grep` for the rename-scan needles at HEAD and on `main`;
    - `gh issue view 649`, diffed with the brief: only D10's `LANG` clause differs;
    - `gh issue list` for the unfiled follow-ups.

## T (Phase 7, verify GREEN, fifth pass: S's test-only rework) — 2026-10-11T15:51:45-06:00
- **Decided:**
  - **PASS.** Only `crates/holler-cli/tests/pane_integration/fake_hub.rs` changed. No production code changed.
  - (1) The one-read no-hub test is replaced by `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub`. It
    covers all five reads and five writes, each `unavailable` with `no live holler hub reachable at <state dir>`.
    Replacing the test, not adding one beside it, keeps the suite the same size.
  - (2) The lost-read and JSON-RPC-error tests now also assert that the message names the hub by the fake hub's own
    state dir, a fresh temporary path that no transport or refusal text can supply.
  - The helpers became `writes()`/`reads()` tables and one `against()` runner returning an `Outcome` that carries the
    state dir.
  - T rewrote its own rename-scan line in `handoff-T-green.md`. The brief's E64 line (O's) and `decisions.md:66` (in
    A's entry) are not T's to edit, so they are left for O.
- **Assumed:**
  - Asserting the literal `no live holler hub reachable at` pins behaviour, not wording: it is the CLI's existing text,
    already pinned by six other test files.
  - The opt-in scenario need not re-run, since no production line changed after `22aa539`.
- **Hedged:**
  - The `Refused` and `BadReply` assertions check `holler hub` plus the state dir, not the full phrase, so they
    allow rewording.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_integration fake_hub`: 15 passed.
  - M9 (`NoLiveHub` in `timed_out`'s write arm) fails the new no-hub test on `pane/cas_put: timed out`. M10
    (`hub_unavailable`'s second arm without the hub) fails both strengthened tests. `store.rs` was restored with
    `git checkout` after each, and `git status` was clean.
  - New `evidence.md` section, verbatim: `state.rs:38-40` and `control.rs:492`.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check` and `machete` are clean. Workspace counts are in
    `handoff-T-green.md`.

## A (Phase 8, anti-duplication gate, second pass: S's test-only rework) — 2026-10-11T16:07:27-06:00
- **Decided:** PASS on this cycle's diff `c13400a..98280ad`, read with the whole story `5d366b9..98280ad`. In `crates/`,
  the cycle's diff is one file, `pane_integration/fake_hub.rs`. No blocks, and two new warns: W-5 is binding at the
  merge with `origin/main`, and W-6 is a nit. The first pass's W-1 to W-4 and W-O carry over unchanged. The full detail
  is in `handoff-A-dup.md`, which replaces the first pass's.
  - **No parallel path, and one duplicate folded.**
    - T's `against` runner, over the `reads()` and `writes()` tables, replaces the two identical `run` closures of
      `each_read` and `each_write`.
    - `Outcome` and `assert_names_the_hub` have no analogue: `verb_harness::Outcome` is a verb run's result, and no
      other test pins `hub_unavailable`'s second arm.
    - The production `Method` table is private, and a table of the test's own is the right oracle.
  - **W-5 (binding at the merge with `origin/main`).**
    - `main` moved to `652841b` at 15:35 MDT (#832), and the trial merge conflicts in `CHANGELOG.md`. So the PR needs
      that merge.
    - #832 added `HerdrPort::observe_pane` and `observe_panes`, both with default bodies. ADR-0021 section 14 says "A
      wrapper port must delegate it", and #832 added the forward to every delegating port on `main`.
    - #649's `BindingHerdr` and `LazyHerdr` are not on `main`, so after the merge they answer from the defaults. Under
      the real wiring, every unmanaged pane would show no working directory.
    - CI on the merged tree fails only case 13 over `BindingHerdr`. Nothing pins `LazyHerdr` or `observe_panes`. So a
      fix to `BindingHerdr` alone passes CI and still masks the adapter in production.
    - The fix is both methods in both wrappers, with three pins.
  - **W-6 (nit, not binding):** `fake_hub.rs:349-350` builds the no-hub text inline. `no_hub.rs` already has
    `NO_HUB` and `names_the_missing_hub`, and `fake_hub.rs:423` has a third spelling.
- **Assumed:**
  - **The PR merges only after `origin/main` is merged into the branch**, since `CHANGELOG.md` conflicts. W-5 is
    written for that merge.
  - **CI's result on the merged tree is read from the code, not run.**
    - Case 13 meets `Ok` from the trait default, and `expect_code` turns that into "expected `pane-not-found`, but it
      succeeded".
    - No test runs over `LazyHerdr`'s or `observe_panes`' forwards.
    - I did not build the trial merge `112bb17`.
  - **`store_only.rs`'s assertions hold on the merged tree**, by reading. `pane list` with no Herdr configured
    degrades, with `unmanaged_unavailable: "usage"`, and still exits 0 with the rows.
- **Hedged:**
  - **Why W-5 is a binding warn, not a block.**
    - Phase 7 blocks a parallel path or a drive-by edit, and W-5 is neither. It is drift against a base that moved after
      F wrote the code: on the branch's own base, both wrappers forward every method.
    - A BLOCK routes to F, whose role has no step for merging `origin/main`. The fix cannot compile before that merge,
      because the trait methods do not exist on the branch.
    - Every merge of `main` in this run was made at the operator's level (`3dff970`, `f59ba21`, `d0c1531`).
    - So W-5 goes to O, as the first item of Notes for O. I recommend routing the merge and the fix through F, T-green,
      A-dup and S, so the gates review the tree that lands.
  - **`observe_panes` is A's reading.** The ADR's literal rule names `observe_pane` ("it"), and #832's own test wrappers
    forward only that one. Forwarding `observe_panes` too keeps the ADR's "one call", which only the production chain
    can lose.
  - **W-6 is a nit.** It is a two-line check in a sibling module, not an overlay helper, and the inline form is the
    strictest of the three.
  - **This journal is append-only, so A did not reword its first entry (`decisions.md:66`).** That line is one of the
    two rename-scan hits left. S gave it to O.
- **Evidence:**
  - **Read in full:**
    - `pane_integration/fake_hub.rs` and `store_only.rs`;
    - `src/pane/wiring/bind.rs`;
    - `handoff-S.md`, the first `handoff-A-dup.md`, and every entry of `decisions.md` from A's first A-dup pass on.
  - **Diffs read:**
    - `c13400a..98280ad`, all of it;
    - on `5d366b9..origin/main`: `ports.rs`, `wiring.rs`, `conformance/herdr.rs`, `pane/list.rs`, `pane/get.rs`, and
      the file list.
  - **Read in part:**
    - `pane_integration/{no_hub.rs (100-170), main.rs (1-162), bind.rs (100-160, 240-320)}`;
    - `verb_harness/mod.rs` (1-142) and `store.rs` (65-125, 330-395);
    - on `main`: `unmanaged.rs` (1-110, greps), `adapter.rs` (405-460), `herdr.rs` (`FakeHerdr::observe_pane`),
      `conformance/mod.rs` (`expect_code`), ADR-0021 (901-916) and `ports_test.rs` (`TestHerdr`);
    - the driver's phase-8 routing (`nextPhase` case 8) and its PR phase (greps for a merge of `main`);
    - `skilltext_w5_rename.rs` (the scan's needles) and `pipeline-conventions.md` section 1.
  - **Ran:**
    - `cargo clippy -p holler-cli --test pane_integration -- -D warnings`: clean.
    - The five reworked `fake_hub::` tests: 5 passed.
  - **Git:**
    - `git fetch origin`: `origin/main` is `652841b`.
    - `git merge-tree --write-tree origin/main HEAD` gives tree `112bb17`, with a conflict in `CHANGELOG.md` only. In
      it, `wiring.rs:306` has `Unwired::observe_pane`, and `bind.rs` has neither method.
    - `git grep` at the rename-scan needles: two hits left.
  - **Greps:**
    - `impl … HerdrPort for` and `fn observe_pane(s)` on HEAD and on `main`;
    - `struct Outcome`, `type Call`, `fn against`, the per-method tables, and the no-hub text in the tests;
    - `struct LinkedHost`.
  - **GitHub (16:07 MDT):**
    - `gh issue list --search LinkedHost --state all`: #649 and the closed #802 only;
    - `--search observe_pane`: no issue ties it to #649;
    - #832: closed at 15:35 MDT.

## S (Phase 10, spec audit, second pass) — 2026-10-11T16:21:00-06:00
- **Decided:**
  - **ADVISORY-HOLD.** The full detail is in `handoff-S.md`, which replaces the first pass's.
  - **On its base (`5d366b9`), the branch meets the brief.** All 15 acceptance criteria are met.
    - The first pass's test-only REWORK is closed. T's `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub`
      and the two `…_naming_the_hub` assertions pin every row of D3's table.
    - No production line has changed since the first pass.
  - **The hold is for two defects in the source of truth.** Neither is F's or T's to fix.
    - (1) **The brief predates #832.** It merged at 15:35 MDT, and the PR cannot merge without `main` (a
      `CHANGELOG.md` conflict). After that merge, `BindingHerdr` and `LazyHerdr` answer `observe_pane` and
      `observe_panes` from the trait defaults:
      - this breaks D31 W-4 and ADR-0021 section 14 on `main`;
      - unmanaged panes show no working directory under the real wiring;
      - CI fails case 13 over `BindingHerdr` only (A-dup's W-5, confirmed against the code).
    - (2) **The brief file fails CI's `wizard_scripts` rename scan.** E64's Source (`649-brief.md:1197`) cites the
      setup wizard's skill directory as it was before #820, and so does `decisions.md:66`.
  - **The proposed fix, for the operator:**
    - merge `origin/main`, keeping both CHANGELOG entries;
    - amend the brief and the issue: W-5 as a decision with T's three pins, #832 in Depends on, E64's path, D10's
      `LANG` clause in the issue, and D17's #829;
    - correct `decisions.md:66` and journal the correction;
    - relaunch at phase 4, and re-run the opt-in scenario on the merged tree.
- **Assumed:**
  - This run does not pass `removeHandoffsBeforePr`. No file in the worktree sets it, and `main` keeps every story's
    handoffs.
  - The workspace run that T left running would show only the rename-scan failure. The rework is one file of one test
    binary, and that binary compiled and passed its changed module.
  - GitHub runs no `pull_request` workflow while a PR has a merge conflict, so CI would first run after the merge.
- **Hedged:**
  - **ADVISORY-HOLD rather than REWORK.**
    - Defect 1 is base drift, not a brief written wrong. But no pipeline role can act on it: the forwards cannot
      compile before the merge, F's role has no merge step, every merge of `main` in this run was the operator's,
      and on a production S REWORK the driver gives F no finding (`coding-pipeline.workflow.mjs:5650`).
    - Defect 2 is the brief's own line, which only O may edit.
    - A REWORK would spend a cycle with no route to the fix.
  - **ADVISORY-HOLD rather than PASS.** A PASS would push a conflicting branch, and the merge after the PR would add
    the W-5 code outside the gates, where CI catches only half of it.
- **Evidence:**
  - **Read in full:**
    - the brief's decisions and acceptance criteria (lines 1-122) and E64;
    - `handoff-A.md`, `handoff-A-dup.md`, `handoff-T-red.md`, `handoff-T-green.md`, `handoff-F.md` and the first
      `handoff-S.md`;
    - `pane_integration/{fake_hub,main,store_only}.rs`;
    - `src/pane/wiring.rs`, `wiring/bind.rs` and `wiring/store.rs`.
  - **Diffs read:**
    - `9cafe0d..HEAD` (`fake_hub.rs`, `decisions.md`, `evidence.md`);
    - `5d366b9..origin/main`: the file list, `ports.rs`, `wiring.rs` and `pane/mod.rs`.
  - **Read in part:**
    - on `main`: `conformance/herdr.rs` (case 13), `unmanaged.rs` (`observe_panes` call), `pane/list.rs` (the
      unmanaged gap), `adapter.rs` (the two methods), ADR-0021 (section 14) and the CHANGELOG head;
    - `pane_integration/bind.rs` (the test list and the `LazyHerdr` every-method test);
    - `skilltext_w5_rename.rs` (the scan and its filters);
    - `ci.yml` (the triggers and the test command);
    - the release driver: phase 10's routing, the phase-6 task text, and the PR phase (`:4700-4800`);
    - `workflow-coding-pipeline.md` (the ADVISORY-HOLD flow) and `pipeline-conventions.md` section 1;
    - `feature-implementor.md` (F's rules).
  - **Git:**
    - `git ls-remote origin refs/heads/main`: `652841b`;
    - `git merge-tree --write-tree origin/main HEAD`: tree `2e527e2`, with a conflict in `CHANGELOG.md` only. Its
      `bind.rs` has neither new method;
    - the authors of `3dff970`, `f59ba21` and `d0c1531`.
  - **Greps (read-only):**
    - the rename-scan needles with the test's filters: two hits;
    - the diff for panics, prints, `unsafe` and `#[allow]`;
    - the whole diff, and the lines added since the first pass, for IPs, hosts, home paths, accounts and key shapes;
    - `wc -l` on every touched `.rs` file: maximum 857;
    - the test names of the target: 61 default-run tests and 4 opt-in.
  - **GitHub (16:15 MDT):**
    - `gh issue view 649`: last edited 12:16 MDT. Its top half differs from the brief only by D10's `LANG` clause;
    - `gh issue view 832`: closed at 15:35 MDT;
    - `gh issue list --search` for `LinkedHost`, `observe_pane`, the UTF-8 locale, `select-session` and
      `wizard_scripts`: none of the carried follow-ups is filed.

## O (brief amendment after the S advisory-hold) — 2026-10-11T16:30:00-06:00
- **Decided:** the citation of the setup wizard's `lib/herdr.sh` in A's first entry (line 66) is corrected to the renamed `agent-skills/herdr-setup/` directory (#820): the CI scan `no_old_command_or_directory_outside_the_rename_note` fails on the old path in any tracked file. The excerpt is unchanged. D33 is added for #832; E64's Source and Depends on are corrected; the run resumes at phase 5.

## T (Phase 4, T-red, D33 cycle) — 2026-10-11T16:32:37-06:00
- **Decided:**
  - Pinned D33 with the three pins it names, plus two more rows:
    - `observe::the_lazy_herdr_passes_the_herdr_conformance_suite`, so case 13 covers `LazyHerdr`;
    - `observe_pane` added to `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method`;
    - one recording check each for `BindingHerdr` and `LazyHerdr`;
    - a third recording check over production's chain, `BindingHerdr` over `LazyHerdr`;
    - a failed-build row: one empty observation per pane, with the build attempted once.
  - Put the new rows in a new `pane_integration/observe.rs`, not in `bind.rs`. `bind.rs` is 580 lines, and lint
    warns at 600.
  - Added a pin to `store_only` for D33's last sentence: `"unmanaged_unavailable": "usage"` in JSON, and the text
    warning naming `HOLLER_HERDR_SESSION`. It passes at HEAD; it is a pin, not a RED.
  - Proceeded on A's fifth-pass PASS, although no A pass has run on the amendment.
- **Assumed:** D33 adds no design for A to review. It is A-dup's W-5, adopted verbatim with S's pins, and O's entry
  resumes the run here.
- **Hedged:**
  - The recording inner port answers its other seven methods with `unavailable`, because the checks never call them.
    A fix that made `observe_pane` go through `snapshot` would surface as that error rather than as a silent pass.
  - I did not add the scenario's "own panes never appear in the unmanaged table" check (S's advisory 4). It is
    opt-in, and the scenario re-run is T-green's.
- **Evidence:**
  - Default run on the merged tree: 59 passed, 7 failed (exactly the D33 rows, each on its assertion), 4 ignored.
  - Mutant with `observe_pane` forwarded only: the three recording rows fail on the call log (three `observe_pane`
    calls against one `observe_panes`).
  - Local trial of D33's fix: 24 of 24 pass, then the source was restored (no diff under `src/`).
  - Clippy (`-D warnings`) and `lint.sh` are clean.
  - `evidence.md` has a new section with the #832 source the tests rely on.
