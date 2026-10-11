# Handoff-A: Phase 3 - #649 hub wiring and the scratch integration scenario  (up-front plan review, fifth pass: the re-entry after the diff gate's round-1 BLOCK, F having reported `archChanged`)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `1c05f75`)
**Brief reviewed:** `docs/handoffs/649-brief.md` at `451e2b3` (sha256 `afe88dc4916d7e09...`).
- The brief has not changed since the fourth pass.
- `origin/main` is still `5d366b9` (#650), the base the fourth pass read. I fetched during this pass, and nothing has merged since.

**Reuse map:** no separate file. As in the earlier passes, the brief's Evidence (E1 to E70) and D11 serve as the map.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This pass replaces the fourth one (PASS, `eb94951`). Git history keeps that pass's text.

**Why phase 3 ran again.**
- T-green passed at 13:46 MDT. The outside diff gate then returned BLOCK at 13:47 MDT (round 1, `docs/handoffs/649-diff-result-r1.md`).
- F had reported `archChanged`. So `nextPhase(7, 'BLOCK')` routed here, the pipeline's "A (drift re-check) if architecture changed", and did not classify the findings.
- The chain now resumes: T-red, F, T-green, then diff gate round 2.

**This handoff is the only channel for the round-1 findings.**
- On this route the driver gives them to neither T nor F. The classifier is skipped when `archChanged` is set, so there is no `tNote` or `fNote`.
- The result file is gitignored, sits outside `docs/handoffs/649/`, and no role doc names it.

## Summary

**Verdict: PASS.** I reviewed three things.

1. **The plan.** The brief and the base are both unchanged, so the fourth pass's PASS stands.
   - #828 and #829, which the brief and now the ADR name as owners, are still open.
   - #654's body was edited at 13:34 MDT. The edit rewrites the cutover procedure only and changes nothing the plan builds on.
2. **F's architecture change, the reason for this re-entry.** I found no drift.
   - `Wired<P>` generalises D24's `WiredHarness` to cover W-E's fail-closed host. It is one wrapper, not a parallel second one.
   - Every decorator forwards every port method, the two default-armed ones included (W-4).
   - The ADR edits are the ones D17 and W-N asked for, made in the same change.
3. **The round-1 diff gate's BLOCK.** Both of its code blocks are false positives.
   - The code refutes them, and so do tests I re-ran against a real hub at HEAD.
   - Its third item it lowered to a WARN itself. W-S says what T and F do with each item.

**One new warn binds F (W-T, low).** `connect` reads a value that is not UTF-8 as unset. A non-UTF-8 `HOLLER_TMUX_SOCKET` therefore falls back to tmux's default server, which D6 forbids for a malformed value.

**A BLOCK would be the wrong tool here.** The brief is unchanged, so the driver would stop the run as `needs-amendment` at the next phase-4 entry (`decideArchReReview`). Nothing in the brief needs amending, so W-S and W-T are binding warns.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-S | warn (**binding on F and T**: what each does with the round-1 diff BLOCK) | D3's clients (`wiring/store.rs`) and D13's round trips (`pane_integration/clients.rs`), as `649-diff-result-r1.md` reads them | wire and protocol; pattern consistency | **B-1 ("`get` of a missing record fails as `unavailable`") is false.**<ul><li>`get` returns `Result<Option<Pane>, _>`, so `request`'s `T` is `Option<Pane>` (`store.rs:126-128`, `:265-272`).</li><li>`PaneReply.data` is `#[serde(default)] Option<Value>` (`holler-pane/src/reply.rs:50-57`). So the hub's `"data": null` becomes `Value::Null`, and `from_value::<Option<Pane>>(Null)` is `Ok(None)`.</li><li>Both conformance suites have this exact case, "Case 1: `get` of a name never stored is `Ok(None)`" (`holler-pane-testkit/src/conformance/pane_store.rs:123-131`, `profile_store.rs:187-195`).</li><li>Both pass over the clients against a real hub. I re-ran them at `1c05f75`: 2 passed, 0 failed, 66 s.</li></ul>**B-2 ("`list` sends `{}`, not no params") is false.**<ul><li>The hub decodes `pane/list` and `profile/list` as `NoParams`, which is `#[serde(deny_unknown_fields)] struct NoParams {}` and documented as "only `{}`" (`holler-hub/src/panes/handlers.rs:37-41`, `:97-99`; `profile/handlers.rs:37-39`).</li><li>The hub reads an absent or `null` `params` as `{}` (`handlers.rs:61-68`; `holler-pane/src/reply.rs:111-114`). So `json!({})` is the documented form, not a fabrication.</li><li>`holler_pane::reply` has no typed struct for it, and the hub's `NoParams` is `pub(crate)`.</li><li>Every `list` call in the suites passes.</li></ul>**"B-3" the reviewer lowered to a WARN itself (its W-1).** The two hang tests bound the 10 s only from above: `took < 13 s` against a 14 s hang (`fake_hub.rs:222-248`). No test shows that a store request outlasts the control client's default 5 s, as the watch row does for long polls (`fake_hub.rs:377-388`).<br>**The rest of the review is settled.**<ul><li>The reviewer concludes "fine" or "not a BLOCK" on B-4 to B-9, NV-3, NV-7 and its W-2, W-3 and W-5.</li><li>Its W-4 is settled by the green default run: every `EVERY_VERB` row parses, or the no-hub tests would fail (W-P).</li></ul> | **F changes no production code for B-1 or B-2.** A `request_opt`, or `None` params, would only churn a client that conforms.<br>**F adds the refuting facts to `docs/handoffs/649/evidence.md`.** The diff gate passes that file to `dual-review.sh` with `--evidence` (the driver's `buildDiffGateCaptureTaskPrompt`), so round 2's reviewer sees what NV-1, NV-2, NV-4 and NV-5 asked for:<ul><li>`reply.rs:50-57` and `:94-117`;</li><li>`panes/handlers.rs:37-41`, `:61-68` and `:89-99`;</li><li>the two conformance Case 1 excerpts and `clients.rs:53-65`;</li><li>`ports.rs:230-310`, `HarnessPort` with its two default methods (NV-4);</li><li>`panes/handlers.rs:101-116`, `cas_put`'s membership check (NV-5).</li></ul>**T owes no RED for B-1 or B-2.** These are T's call, not binding:<ul><li>a row in which a read and a write answered after 6 s succeed, with `Answer::After`, which pins the 10 s from below (D9, I5);</li><li>NIT-2: `scratch/mod.rs:406` repeats the labels of `WORKSPACES` (`:94`);</li><li>NV-6: the sentinel variables are removed on drop rather than restored.</li></ul> |
| W-T | warn (**binding on F**, low; A's reading of D6 and the first pass's W-13) | **D6**, "neither falls back to `TmuxSocket::Default`", and W-13, "configuration errors ... fail closed" | cross-cutting concerns (configuration, fail closed) | **A value that is not UTF-8 reads as unset.**<ul><li>`Wiring::connect` passes `&\|name\| std::env::var(name).ok()` (`wiring.rs:104`).</li><li>`std::env::var` answers `Err(NotUnicode)` for such a value, and `.ok()` turns it into `None`. Every reader in `env.rs` treats `None` as unset.</li></ul>**The effect, variable by variable:**<ul><li>`HOLLER_TMUX_SOCKET` becomes `TmuxSocket::Default`, the operator's live tmux server (`env.rs:78-83`). D6 and W-E forbid exactly that for a malformed value.</li><li>`HOLLER_HERDR_SOCKET` falls through to `HERDR_SOCKET_PATH` (`env.rs:121-126`). T-green pinned against that fall-through for a relative value.</li><li>`HOLLER_OPENCODE_BIN` falls back to the `PATH` search.</li><li>`HOLLER_OPENCODE_SERVE_ARGS` is silently dropped.</li></ul>**Reach.** The scenario cannot hit this, because D10 (4) gives the binary ASCII scratch paths. Only an operator's environment can. | **Read a value that is not UTF-8 as malformed, never as unset.**<ul><li>For example: `std::env::var_os(name).map(\|v\| v.into_string().unwrap_or_default())`.</li><li>Each variable's empty-value rule then answers it: `usage` naming the variable, at the port's first use (D26).</li><li>The change is one line in `connect`, inside the Blast radius, and `Lookup` keeps D24's signature.</li></ul>T may pin it as a row of D6's test (`wired.rs`), setting the value with `OsStr::from_bytes` under `with_env`'s lock. **W-T is not an architecture change.** |
| W-O | warn (binding at phase 7; carried) | **D32** as the fourth pass restated it | reuse | **Holds.** There are still three `LinkedHost` copies and no fourth:<ul><li>`pane_integration/bind.rs:422`;</li><li>`profile_verbs/apply/rig.rs:636`;</li><li>`pane_verbs/launch/rig.rs:663`.</li></ul>No consolidation issue is open. A search of open issues for `LinkedHost` finds only #649. | Unchanged: phase 7 checks that there is no fourth copy. O files the consolidation (Notes for O). |
| W-U | warn (not binding on this run; O files it in Aftersight) | the driver's diff-gate rework route (`claude-pipeline-v0.1.7`) | none (pipeline) | **(a) The findings are dropped on two routes.**<ul><li>With `archChanged` set, `coding-pipeline.workflow.mjs:5508` skips the classifier and sets no note, then routes to phase 4. T-red and F follow with only their generic tasks.</li><li>A `production` classification also respawns F with "Perform Phase 6's work." and no findings. Only `mixed` carries them to F (`buildMixedReworkNoteForF`).</li><li>The findings survive only in a gitignored file outside the run's handoff directory.</li></ul>**(b) The task text does not fit a re-entry.** Phase 4's task says "before any code exists", but on this route the code exists.<br>**(c) A cannot route a code finding through BLOCK here.** On an unchanged brief, a phase-4 BLOCK stops the run as `needs-amendment`. | Carry `extractBlockingFindings`' output into `context.fNote` and `context.tNote` on every diff-gate BLOCK route. Give phase 4 a re-entry task that names the gate, its result file and the re-check's scope. |

**Checked against F's change (the drift re-check), and consistent.**

- **Wired ports.** `Wired<P: ?Sized>` with `impl HarnessPort for Wired<dyn HarnessPort>` and `impl HostPort for Wired<dyn HostPort>` (`bind.rs:235-322`). `WiredHarness::new` keeps D24's signature.
  - The alternatives F rejected were a second `WiredHost` struct and a `BindingHost` over a `Result`. Each would have been a copy of the pattern or a change to a D24 signature.
  - `LazyHerdr` stays separate, because it builds on first use and a `Wired` port is built at once. Both are named in the plan.
- **W-4: every method is forwarded.**
  - The `Wired` harness forwards all eleven `HarnessPort` methods, `send_prompt` and `session_activity` included (`bind.rs:297-303`). Those two are the trait's only default methods (`ports.rs:293`, `:306`).
  - `BindingHerdr` and `LazyHerdr` forward all seven `HerdrPort` methods.
  - `BindingHost` and the `Wired` host forward all four `HostPort` methods.
- **D3 and W-9: one mapping.**
  - `call` and `timed_out` (`store.rs:342-378`) are the one mapping, sent through `transport::call(None, ..)`. There is still no second caller of `control::run`.
  - The second pass's B-1 clause is in place: `Io` with `WouldBlock` or `TimedOut` is `timeout` for a read.
  - The typed params of `holler_pane::reply` are used throughout, plus the literal `{}` for the two `NoParams` methods.
- **The no-hub message** follows the CLI's own pattern: `resolve_state_dir().unwrap_or_default()` and "no live holler hub reachable at {}". The same pattern is in `say_cmd.rs:156,172`, `roster_cmd.rs:40,51`, `transport.rs:47` and five more commands.
- **Layers.**
  - Only `holler-cli` changes. `holler-hub`, `pane/mod.rs`, `main.rs` and `say_cmd.rs` are untouched.
  - No wire, golden-file or port change. The CLI's new uses of the hub are `control`, `state` and `panes::WATCH_WAIT`, all in the dependency direction that already exists.
- **ADR-0021, in the same change.** The edits include:
  - D17's sentences;
  - W-N (a) to (c): "#829", `--herdr-session` "required by `launch`", and the Consequences bullet;
  - the read-timeout wording;
  - W-B: #828 and the operator's stopgap;
  - W-C's section-2 pointer;
  - W-Q (a) and (b), on `profile apply` under `holler pane` and the `tx_apply.rs` observation bound.

  #828 and #829 are open.
- **D11 and W-3: the rigs are included, not copied.** The three rigs come in by `#[path]` (`scratch/mod.rs:38-44`). The adapter-host crate's one-helper pin (`fake_tmux_test.rs:785`) still holds after T-red's D11 repair.
- **Size.** The wiring files run from 237 to 410 lines. `tests/support/mod.rs` is still 857 lines (W-I). `lint.sh` was clean at T-green.
- **Public repository.** The diff has no personal names. The only home path is the generic `/home/u` in `env` rows.
- **Secrets.** No variable carries a credential. `serve_args`' error quotes none of the value. `ProcessEnv::Inherit` leaves credentials to the tmux server's environment (#654).
- **The fourth pass's other items.**
  - W-N, W-P and W-Q are done.
  - W-R is met by its second option: `scenario.rs:7` carries the exact opt-in command.

## Notes for T and F (this route carries nothing else)

- **T-red**
  - **B-1 and B-2:** no RED is owed for either. W-S's evidence settles both.
  - **W-T** is the one new behaviour of this cycle. If T pins it, a non-UTF-8 `HOLLER_TMUX_SOCKET` in D6's test is a valid RED against HEAD.
  - **Optional rows:** the 10 s lower bound, NIT-2 and NV-6 (all in W-S) are T's call.
- **F**
  - **Code:** W-T's one-line fix in `connect`.
  - **Evidence:** W-S's entries in `evidence.md`.
  - **What not to change:** nothing for B-1 or B-2.
  - **`archChanged`:** on A's reading, neither change touches architecture, so F's `archChanged` for this cycle is false unless F changes more. A round-2 BLOCK would then be classified (test-only, mixed or production) rather than routed back here.

## Notes for O

PASS, so nothing is required for this run.

**Follow-ups still open from the fourth pass:**
- comment on #825 that `profile apply` logs as `holler pane` through the wiring (W-Q (a)). #825 has no comments;
- file the `LinkedHost` consolidation (W-O);
- #653 and #828: the automated `pfleet o reset` caller is not covered by the operator's stopgap;
- #803 does not list #649.

**New:** file W-U in Aftersight (per the operator's rule, pipeline bugs go to Aftersight, not Playbook).

## Patterns referenced

- **The clients' contract:**
  - `crates/holler-pane/src/reply.rs` (46-117);
  - `crates/holler-hub/src/panes/handlers.rs` (37-68, 89-116);
  - `crates/holler-hub/src/profile/handlers.rs` (37-39);
  - `crates/holler-hub/src/control.rs` (492-513, via `evidence.md`).
- **The conformance suites:** `crates/holler-pane-testkit/src/conformance/{pane_store.rs:123-131, profile_store.rs:187-195}`, run by `crates/holler-cli/tests/pane_integration/clients.rs`.
- **The ports:** `crates/holler-pane/src/ports.rs` (the four traits; defaults at 293 and 306).
- **The CLI's no-hub pattern:** `crates/holler-cli/src/{say_cmd.rs:156-172, roster_cmd.rs:40-51, transport.rs:47}`.
- **The driver:** `.aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs` (1095-1215 `nextPhase`; 3690-3708 the diff gate's evidence; 5376-5720 the loop), and `workflow-coding-pipeline.md` (1104-1112, 1775-1778, 1888).
