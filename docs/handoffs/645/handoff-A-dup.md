# Handoff-A-dup: Phase 7 - #645a `pane switch` and `pane reset`  (anti-duplication gate, round 3)

**Date:** 2026-10-10 (01:40 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`)
**Diff base:** `bd5e825` (`origin/main` and the merge base; fetched again 2026-10-10 01:37 MDT, unchanged)   **Diff head:** `cce6f00` (the code is
`6dd44f3`'s, T-green round 6 PASS at `d2d3793`; `cce6f00` changes handoff docs only)
**This cycle:** everything since round 2 (`08d96df`, which reviewed `940e338`): F rounds 4-6 (`29fa0ff`, `a1f01aa`, `6dd44f3`;
two merges of `origin/main`, `abdcbb6` and `bd5e825`), and T's test from round 5 (`7435610`)
**Reuse map:** `docs/handoffs/645-brief.md`, "Reuse map (extend, do not duplicate)", lines 1052-1065
**Verdict:** PASS
**Supersedes:** round 2 (PASS, commit `08d96df`) and round 1 (BLOCK, `da53aba`); their full text is in git history

## Summary

PASS. This cycle adds no parallel path. The one production change is F's round-5 split of the private
`select_and_observe`: `switch` now calls `ports.harness.select_session` itself, at the `acted` boundary
(`tx_switch.rs:164-177`), and the observation is a private `observe` (`:297-312`). The calls, their order and the shared
pieces are unchanged: `shown_differs`, `quoted`, `doctor_command(Some(pane), true)` and the one copied `screen_text`.
Reconcile's repair (`reconcile/observe.rs:316-321`) still has the same shape: select, then observe. T's one new test
extends the story's own runner. Both merges kept both sides of ADR-0021, which still shows exactly the four AC-24 hunks.
Neither merge brought in a neighbour that this story should now reuse. #642 part 2's adapter checks session ids only for
OpenCode's title and URL path (`tui.rs:309-314`, `lib.rs:494-509`), and that check is not the engine's typed-input grammar.
The merges did change three of the carried warns. #642 part 2 makes the copied screen wording imprecise. Open PR #718
(#644) adds a third wording and a second "acted, so append the step" rule. #718's rig adds a general harness hook. Two
new warns are recorded: one test-builder repetition and one stale ADR word. None needs a change in 645a.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/tx_switch.rs:145-148, 314-322`, beside `reconcile/observe.rs:342-348`, `holler-cli/src/pane/profile_scope.rs:37-56`, and open #718's `tx_launch.rs` `observe_live` and `pane/launch.rs:428-436` | Carried from rounds 1 and 2 (F-4 and the `screen_text` copy), and widened this cycle. (a) #642 part 2, now merged, makes a `None` from `shown_session` also mean "cannot tell": a deleted session, a title that is not a whole id, or no TUI (ADR-0021 `:125-126`, item 3). Both copies of `screen_text` still call it "its home screen". The run still fails safe (`unavailable`, nothing recorded), and the brief requires the word-for-word copy. (b) Open #718 describes the same `None` as "no session", a third wording of one observation. (c) #718 decides "acted, so append the reconcile step" in the CLI (`failure_body`, on `TxFailure.acted`), while 645a decides it in the engine (`SwitchFailure::message`). The two steps differ for a reason that ADR section 8 states, but the lead-in `to reconcile, run ` and the append rule are spelled in two crates, three counting `reconcile_step`. None of this is filed (issue search at 01:34 MDT). | O files the F-4 fold: one `findings` owner for the lead-in, the append rule and the screen description. It is used by reconcile, `tx_switch` and `tx_launch`. It picks one wording for a `None` that may mean "cannot tell", and corrects `reconcile_step`'s doc (`profile_scope.rs:39-40`, "verbs call it rather than spell a step of their own"). The `screen_text` comment then names that issue. No change in 645a. |
| 2 | warn | `crates/holler-cli/tests/pane_verbs/switch.rs:433-467` (this cycle's new test) | `switch_observation_failure_names_the_reconcile_step` copies AC 8's build (`:417-425`) line for line, with only the op changed. `switch.rs` now has three `one_pane()` plus `fail_next(op, Timeout { op })` builds (`:363-370`, `:417-425`, `:437-445`). `reset.rs:212-220` already has the shape as `faulted(fault)`. Switch's harness op list `[Health, ListSessions, SelectSession, ShownSession]` is spelled twice (`:278-283`, `:454-459`). The test reuses `both`, `failed`, `Case`, `RECONCILE_P` and the rig, so this is repetition inside the story's own file, not a second runner. | When 645b edits these files, add a `faulted`-style build to `switch.rs`, or one generic over the setup that both files share, and one const for the op list. No change in 645a. |
| 3 | warn | `docs/adr/ADR-0021.md:485` | Raised by F in round 4 and not recorded by A until now. The code table says "the open codes so far (#640's `grid-unreachable`, merged; #645's and #646's, planned) are all refusals". Once 645a lands, "#645's ... planned" is stale. Row `:424` lists the three codes correctly, and "are all refusals" stays true, so this is not a contradiction. AC 24 allows no fifth hunk, so F was right to leave it. Open #718 adds three open codes (`pane-exists`, `grid-occupied`, `port-in-use`) and leaves this line as it is too. | Whichever of #644 and #645 lands second already has to re-resolve ADR-0021 (see Notes). In the same edit it rewrites the parenthetical to list each story's open codes as merged. Or O allows a one-word edit here before the merge. |
| 4 | warn | `docs/adr/ADR-0021.md:551-553` (section 12), against `:380-381` (the #645 paragraph) | Carried from round 2. Section 12 says a verb that times out "prints the reconcile step". The #645 paragraph says a failure before `select_session` prints no step, and AC 5 pins that for a timed-out health check (`tests/pane_verbs/switch.rs:363-373`). `park.rs:11-12` (#646) reads sections 8 and 12 as asking for a step "after a live act", and so does #718's section-8 paragraph ("every failure once the act has started"). The literal sentence is still broader than all three readings. | #718 already edits section 12 (its PROPOSED no-operation-id paragraph). Narrow the sentence there to "...and, once a live act has begun, prints the reconcile step". No change in 645a. |
| 5 | warn | `crates/holler-cli/tests/pane_verbs/switch.rs:111-185, 469-504` | Carried from rounds 1 and 2 (F-2), and widened by #718. `both` and `both_with` are the fourth both-format runner, beside `doctor/surface.rs:131`, `profile_verbs/rig.rs:112` and `park/rig.rs:220`. `WriterInSelect` is a delegating `HarnessPort`, the same pattern as doctor's `HealthGate` (`doctor/surface.rs:458-514`). Open #718 adds `tests/pane_verbs/launch/rig.rs` (711 lines, built on #643's `list::Rig`). That rig has its own `Calls` and a general `Rig::after(op, hook)`, "a test closure run once, right after a named `HarnessPort` method returns", which would replace both delegating wrappers. F-2's trigger, "a shared verb-test rig lands from #643 or #644", fires when #718 merges. F-2 is not filed. | File F-2 with the four runners, both wrappers and #718's hook in scope: one rig family for the `pane_verbs` binary. No change in 645a. |
| 6 | warn | `crates/holler-pane/src/tx_switch.rs:206-225` | Carried from rounds 1 and 2. The scoped-read arms (with `--profile`, through `scope.resolve`; else `pane_store.get`, where none is `pane-not-found`) still have four private copies: `reconcile.rs:218-242`, `pane/park.rs:300-319`, `pane/get.rs:93-130` and this one. `get` and `switch` both search the scope's answer by name, but they answer a scope that breaks its contract differently: `get` with `pane-not-in-profile`, `switch` with `pane-not-found`. Neither the real scope nor the fake can reach that arm. | Follow-up: one `holler-pane` function for "the records that `PANE` and `--profile` name", used by all four, with one answer for that arm. |
| 7 | warn | `crates/holler-cli/src/pane/switch.rs:97-101` | Carried from round 2. `request` types `--profile` inline. #643's `list::profile_name` (`pane/list.rs:142-144`, called at `get.rs:81`, `watch.rs:75` and `list.rs:98`) does the same, and so does doctor's inline copy. The behaviour is the same: a bad name is `usage`, exit 2. | When 645b edits this file (it adds `--first` there), call `super::list::profile_name(profile)?`. No change in 645a. |

## What this cycle changed, and why none of it is a parallel path

| Change | Where | Result |
|---|---|---|
| F round 5 (the outside gate's r4 B-1): `select_and_observe` split | `switch` calls `ports.harness.select_session(..)` at `tx_switch.rs:173-176`, mapped `acted`. The private `observe` (`:297-312`) does `shown_session`, then `shown_differs`, then `screen_text`. The same port calls in the same order, and the same shared helpers. The engine and reconcile's repair (`reconcile/observe.rs:320-321`) are still the only callers of `select_session` in `holler-pane`. | reused, unchanged |
| F round 5: doc comments | `SwitchFailure.acted` (`tx_switch.rs:114-117`), the act-boundary comment (`:164-167`), `read`'s `find` by name (`:206-211`), and `reset::run` (`pane/reset.rs:33-35`), which says it takes switch's `execute` | no code |
| T round 5: observation failure | `switch_observation_failure_names_the_reconcile_step` (`tests/pane_verbs/switch.rs:433-467`), on `one_pane`, `both`, `failed`, `Case` and `RECONCILE_P` | extends the story's own runner (warn 2 for its build) |
| F rounds 4 and 6: merges of `origin/main` (`abdcbb6`, then `bd5e825`) | ADR-0021 conflicts were resolved by keeping both sides. Section 9 keeps #714's `pane launch` row beside this story's `pane switch` row (`:424`). "Deferred" (`:621-633`) has this story's mismatch bullet and PROPOSED bullet, then #642's three bullets in `origin/main`'s order. `git diff origin/main -- docs/adr/ADR-0021.md` has 4 hunks. No bullet is duplicated and neither side lost text. | no story code |
| What the merges brought next to this story | #642 part 2: `tui.rs:309-314` `is_session_id` (OpenCode's title grammar, `ses_` plus `[0-9A-Za-z]+`) and `lib.rs:494-509` `session_path` (percent-encodes ids for a URL path). Both are private to the adapter, and both differ from `parse_session_id` (`tx_switch.rs:67-79`), the harness-neutral grammar for typed input that runs before any port is called. Every id that `is_session_id` accepts, at OpenCode's length of 30, passes `parse_session_id`. The adapter's `select_session` checks the session and watches the title itself (`attach.rs:63-96`). That happens behind the port, and the engine's P4 and O1 stay as the brief's Decision 6 and R-2 plan them: P4 also refuses a child session, and the adapter's own check, `known` (`lib.rs:426-443`), does not: it compares the status and the id and reads no `parentID`. In `holler-pane`, #713, #714 and #642 changed doc comments only. | nothing to reuse |

Stack checks on the merged tree:

- **One path.** In `holler-pane`, `select_session` is called only by the engine and by reconcile's repair. No prompt is
  sent, so nothing bypasses `send_prompt`.
- **Contract.** No frozen file differs from `origin/main`: `lib.rs`, `ports.rs`, `pane.rs`, `error.rs`, `findings.rs`,
  `reconcile*`, `pane/mod.rs`, `args.rs`, `output.rs`, `wiring.rs`, `tests/pane_verbs/main.rs` and `doctor/`. Neither do the
  test kit, the hub, the proto crate or any `Cargo.toml` or `Cargo.lock`. `doctor.rs` differs only at `:10`, where
  `mod rig;` became `pub(crate) mod rig;` (Decision 16). The closed code table is untouched. This story's three open codes
  are still the only ones in `holler-pane` (`grid-unreachable` is #640's).
- **Output.** `execute`'s usage path (`pane/switch.rs:85`) is the same `emit_error(.., ErrorBody::from(&error))` that
  `park.rs:228` and `watch.rs:66` use, and the result goes through `emit` (`:143`).
- **Size and naming.** Every touched file is under 800 lines: `tx_switch.rs` 334, `pane/switch.rs` 160, `pane/reset.rs` 45,
  the test `switch.rs` 649 and `reset.rs` 288, and `ADR-0021.md` 661. The diff's added lines outside `docs/handoffs/`
  contain no personal host, tailnet or account name; the only URLs are the repository's own issue links. There is one
  CHANGELOG entry for #645.

## The diff gate's r6 items routed to A-dup (O's journal entry, 01:40 MDT)

| Item | Checked against | Result |
|---|---|---|
| NV-1: `read` finds the pane by name | `tx_switch.rs:212-224` uses `.find(\|pane\| pane.name == request.pane)`, with the reason at `:206-211`. It is the same search as `get.rs:102-108`. S accepted the deviation from `into_iter().next()`. | holds (also warn 6) |
| NV-2: the `ServerDown` remedy is never `None` | `findings.rs:127-131`: `ServerDown` maps to `pane.map(relaunch_command)`, so the answer is `Some` for any `Some(pane)` and any `FixState`. `check_health`'s `None` arm (`tx_switch.rs:253`) cannot be reached with a named pane, as its doc (`:241-244`) says. AC 5 pins `run holler pane relaunch demo-c1r1` (`tests/pane_verbs/switch.rs:353-356`). | holds |
| NV-5: the #645 paragraph is inside section 8 | `ADR-0021.md:365`, between `### 8.` (`:305`) and `### 9.` (`:392`); `grep -n 'Switch and reset as built (#645)'` prints only that one line | holds |
| W-1, W-2 (the reviewer withdrew both) | `check_listed` runs only for `Existing` (`tx_switch.rs:199-202`). The usage path (`pane/switch.rs:85`) is the same `emit_error` form that `park.rs` and `watch.rs` use. | no finding |
| W-3: the "Deferred" bullet's order against Decision 17(c) | #644's edit is not on `main`, so A's round-2 form ("#644 to follow") applies, and S accepted it. Open #718 writes the mirror form. Whichever lands second writes 17(c)'s single form (Notes). | no finding |

## Notes for O (not findings)

- **Open PR #718 (#644 part 1, head `444c652`, updated 01:30 MDT) conflicts with this branch in
  `docs/adr/ADR-0021.md` only.** `git merge-tree --write-tree HEAD origin/issue-644-implementation` exits 1. CHANGELOG,
  `cli-surface.txt`, `stub.rs` and ADR 0003 auto-merge. This is R-1, and the brief's Decision 17(b) and (c) say how to
  resolve it. Whichever PR lands second does three things:
  - **Section 8.** Both PRs add an "as built" paragraph at the same anchor, after `:363`, and both state the `unavailable`
    decision. The second keeps both paragraphs and cites the first's sentence instead of restating it, as AC 24 requires
    ("stated once").
  - **Section 9.** #718 edits the `pane launch` row, the line above this branch's `pane switch` row. Keep both. #718 also
    widens the `unavailable` row with "or the live state a verb needs is not there after its act (#644)". That agrees
    with this story's decision, and the second PR can make it "(#644, #645)".
  - **"Deferred".** Both rewrite the mismatch-code bullet: #718 to "decided by #644 ...; #645 to follow", this branch to
    "... #645 for switch and reset; #644 to follow". The second writes Decision 17(c)'s single form, which names both.
    The same edit is the natural place for warn 3's line `:485`.
- `grid-unreachable` is declared twice, in `holler-pane-testkit/src/herdr.rs:51` and `holler-adapter-herdr/src/plan.rs:39`.
  That is #640's, not this story's. It is the precedent to keep in mind when O hands the brief's Forward-compat note to
  #646c: reuse `tx_switch::SERVER_UNHEALTHY`, `ORCHESTRATOR_PANE` and `parse_session_id`, and do not declare them a
  second time.
- **Follow-ups still to file** (none found by `gh issue list` at 01:34 MDT): F-1, F-3, the F-4 fold (warn 1), F-2 (warn
  5) and the scoped-read fold (warn 6). Today they are recorded only in `docs/handoffs/`.

## Patterns referenced

- `crates/holler-pane/src/reconcile/observe.rs:316-348` (select, observe, `screen_text`) and `reconcile.rs:218-242`
  (`resolve`); `crates/holler-pane/src/findings.rs:306, 332` (`doctor_command`, `quoted`).
- `crates/holler-cli/src/pane/profile_scope.rs:37-56` (`reconcile_step`), `list.rs:142-144`, `get.rs:93-130`,
  `park.rs:8-14, 300-319`, `watch.rs:66`.
- `crates/holler-adapter-opencode/src/attach.rs:1-17, 63-96, 202-216`, `tui.rs:259-271, 309-314`, `lib.rs:494-509` (#642 part
  2, merged).
- `docs/adr/ADR-0021.md:116-133` (`HarnessPort` as built), `:365-390` (the #645 paragraph), `:424`, `:485`, `:551-553`,
  `:621-633`.
- Tests: `crates/holler-cli/tests/pane_verbs/doctor/surface.rs:131, 458-514`, `park/rig.rs:220`,
  `tests/profile_verbs/rig.rs:112`, and `reset.rs:212-220`. On #718: `tests/pane_verbs/launch/rig.rs:1-18, 252`,
  `src/pane/launch.rs:428-436` and `holler-pane/src/tx_launch.rs` (`observe_live`).
