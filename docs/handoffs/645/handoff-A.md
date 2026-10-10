# Handoff-A: Phase 3 - #645a `pane switch` and `pane reset`  (up-front plan review, round 2)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `8cf4f00`, with `origin/main` at
`dc300ab` merged in)
**Brief reviewed:** `docs/handoffs/645-brief.md` (1,253 lines, as amended in `8cf4f00`)   **Reuse map:** the brief's "Reuse
map (extend, do not duplicate)", lines 1052-1065 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS
**Supersedes:** round 1 (BLOCK, commit `2bbb237`; its full text is in git history)

## Summary

PASS. Round 1's one block is fixed the way this crate already shares a test rig. `doctor.rs:10` becomes `pub(crate) mod
rig;`, and both verb files reach the rig as `crate::doctor::rig`. That is the `profile_verbs` pattern on `main`
(`crate::list::rig`). I built a scratch copy of this tree with that edit and probe tests in `switch.rs` and `reset.rs`.
`cargo clippy -p holler-cli --test pane_verbs -- -D warnings` is clean, and the probes and every doctor test pass. All six
round-1 warns are now in the brief, and the amendments add no drift. Two new warns, both small: a follow-up to fold O1's
private copy of `screen_text`, and the wording of the "Deferred" bullet if #645 lands before #644.

## Round 1, disposition

| # | Round 1 finding | Now | Where in the brief |
|---|---|---|---|
| 1 | block: a second `#[path]` load of `doctor/rig.rs` fails `clippy::duplicate_mod` under AC 25 | fixed, and verified on the tree (below) | Decision 16, F-8, Reuse map "Test world", Files, AC preamble, Size check, Test plan, R-6, Forward-compat row for #647 part 2 |
| 2 | warn: P5 is a read-check that two concurrent switches can both pass | resolved: stated as a pre-check, the gap is recorded, the fix is a follow-up, and the ADR paragraph records it too | P5, Decision 7, R-5, F-3, Decision 17(b) |
| 3 | warn: `holler pane relaunch` spelled outside the remedy table | resolved: P3 uses `FindingKind::ServerDown.remedy(Some(&pane), FixState::NotFixable)`, with a `doctor_command` fallback and no `unwrap` | P3, D-9, Reuse map |
| 4 | warn: the "as built" paragraph in section 11, while #644's is in section 8 | resolved: section 8, with the `unavailable` decision stated once; section 11 gets one sentence; AC 24 updated; AC 22 puts the verb's own flag before `[--profile NAME]` | Decision 9, Decision 17, AC 22, AC 24 |
| 5 | warn: two owners of the "to reconcile, run" lead-in | resolved: follow-up F-4, and F spells the lead-in exactly | F-4 |
| 6 | warn: stable codes and grammar for later verbs | resolved: a Forward-compat row for #646c | Forward-compat |
| 7 | warn: `panes[0]` can panic | resolved: `into_iter().next()`, and `None` is `pane-not-found` | P1 |

Optional, on row 2: F-3 has no ordering, while P2 gives 645b "before #654". Once #649 wires real ports, the race R-5
describes can produce the wrong-session state. O may give F-3 (or the doctor-finding alternative) the same "before #654"
constraint when filing it.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | O1 (line 919): "`tx_switch.rs` keeps a private copy of that two-arm helper" (reconcile's `screen_text`) | duplication | The copy is justified in writing, so Phase 7 will pass it. But nothing keeps it in step with the original, and no follow-up names folding it. If #647 part 2 rewords either copy, every test still passes and doctor and switch describe a screen differently, which is the outcome the brief says the copy prevents. | Widen F-4 to "one owner for the reconcile-step sentence and the screen wording", in `findings` beside `doctor_command` and `quoted`. F puts a one-line comment on the copy naming its original and F-4. |
| 2 | warn | Decision 17(c) (lines 1138-1141): the "Deferred to named stories" bullet | ADR consistency | (c) always writes the two-story form: "section 8: #644 for launch and relaunch; #645 for switch and reset". If #645 merges first (#644 is still on its branch at `7195993`), the standing spec credits #644 with a decision that section 8 does not yet hold for launch and relaunch. | When #644 is not on `main`, write "section 8: #645 for switch and reset; #644 to follow for launch and relaunch", mirroring #644's own "#645 to follow" (644-brief decision 20(d)). The second story to land writes the two-story form. AC 24 is unchanged. |

### Finding detail

**1. The screen wording has two copies and no fold.**

- The original: `crates/holler-pane/src/reconcile/observe.rs:342-348`, a private `fn screen_text(shown: Option<&str>)`
  (`its home screen`, or `session <quoted>`), in a #647 file. `reconcile` declares `observe` as a private module
  (`reconcile.rs:45`). So exporting it would mean visibility edits in two #647 production files, both of which #647 part 2 is
  likely to touch. A private copy is the right call for 645a, and round 1 accepted it on the condition that the wording
  matches.
- The gap: AC 7 pins switch's wording (`its home screen`). Doctor's wording is pinned only by doctor's own tests. Neither test
  compares the two. The follow-ups cover the rig (F-2) and the reconcile-step sentence (F-4), but not this.
- Why F-4 is the home: both pieces are message text that several verbs print. F-4 already names the owner, a `findings`
  function beside `doctor_command` (and `quoted`, which `screen_text` calls). #663's own follow-ups F1(a) and F2 point the
  same way, hoisting `reconcile_step` into `holler-pane` beside `doctor_command` (663-brief, `issue-663-implementation` at
  `e46b427`). One fold then covers all three.

**2. The "Deferred" bullet when #645 lands first.**

- Today, lines 535-536 defer the mismatch code to "#644 and #645". Decision 17(b) handles both landing orders for the section
  8 paragraph: the first to land states the decision, and the second cites it. Decision 17(c) handles only the case where
  #644 lands first ("If #644's in-place edit ... is on `main` by then").
- When #645 lands first, the two-story form says the code is decided in section 8 for launch and relaunch. But section 8 would
  then hold only #645's paragraph. The rule in that paragraph is general ("a mismatch observed after the act is the closed
  failure `unavailable`"), so this is a matter of accuracy, not a contradiction. One clause fixes it.

## What I verified on the tree (the evidence for PASS)

**The fix, built.** I made a scratch copy of `HEAD` (`8cf4f00`, by `git archive`) in the session scratchpad, outside the
repo, with its own `target/` seeded by `scripts/seed-target-dir.sh`. The worktree was not edited.

- The edit: `crates/holler-cli/tests/pane_verbs/doctor.rs:10`, `mod rig;` to `pub(crate) mod rig;`. Nothing else in that
  file changed.
- The probes:
  - `switch.rs` uses `crate::doctor::rig::{whole, Rig, Seed}`.
  - `reset.rs` uses `crate::doctor::rig::{kinds, whole, Rig, Seed}`.
  - Neither file declares a `rig` module.
- `cargo clippy -p holler-cli --test pane_verbs -- -D warnings`: clean (exit 0).
- `cargo test -p holler-cli --test pane_verbs -- switch reset doctor`: 36 passed, 0 failed. Doctor's tests are unaffected,
  because `doctor/read_only.rs:11` and `doctor/surface.rs:18` reach the rig as `super::rig`.
- No enabled lint objects to a `pub(crate)` module inside a private module. `[workspace.lints]` (`Cargo.toml:19-30`)
  enables no nursery or pedantic group, which is where `redundant_pub_crate` lives. `profile_verbs/list.rs:4-5` does the same
  on `main` and passes CI.

**What the probes also confirmed, for T:**

- Every rig item and field the ACs name can be reached from a sibling file: `rig.harness` and `rig.panes`, `seed_session`,
  `delete_session`, `kill`, `freeze`, `close_tui`, `tui`, `set_quirk(Quirk::SelectAckedWithoutTui)`, `faults().fail_next`,
  `concurrent_put`, `rewrite`, `run(&whole(false))`, `mark` and `calls_since`, `verb` with `check_envelope`, and
  `Seed::in_profile` and `.orchestrator()`.
- The call log records `health`. AC 2's port sequence logs as `[Health, ListSessions, SelectSession, ShownSession]` and
  `[Get, List, CasPut]`, with `herdr` and `host` empty (the outside review's NV-5).
- The rig's panes share one data directory, so P's server lists Q's session. AC 4 therefore reaches P5's
  `session-of-other-pane` rather than stopping at P4.
- Decision 18 and AC 16 hold: after a reset-like move (create, select, record the new session), a whole-fleet pass reports
  exactly one finding, `stray-session`.

**Facts checked in the source:**

- **The remedy.** For `ServerDown`, `remedy(Some(&pane), _)` is `pane.map(relaunch_command)` whatever the `FixState`
  (`findings.rs:121-133`). `ServerWedged` maps the same way, so one call covers both states that make the fake's `health`
  answer `false`, frozen and killed.
- **The exit codes.** `class_of` (`error.rs:266-303`) maps `unavailable`, `timeout` and `generation-conflict` to exit 1, and
  `session-not-found`, `pane-not-found`, `pane-not-in-profile`, `profile-not-found` and every open code to exit 3.
  Decision 9 and every exit code in the ACs agree (the outside review's NV-3).
- **The new codes.** `orchestrator-pane`, `server-unhealthy` and `session-of-other-pane` collide with no closed code and no
  finding kind. The only open code declared so far is `grid-unreachable`. Neither #644's nor #646's current brief plans a
  rival code or flag for these conditions.
- **The ADR anchors** are where Decision 17 puts its edits:
  - Section 8 ends at line 306, before `### 9.` at line 308.
  - Row 340 is the switch and reset row, and line 456 is section 11's last sentence.
  - The "Deferred" bullet is at lines 535-536.
  - #644's decision 20(c) also puts its paragraph in section 8.
- **The registry gap.** The hub's compare-and-swap enforces only the generation and the membership rule
  (`holler-hub/src/panes/store.rs:163-175, 338-352`). R-5's statement of the gap is correct.
- **The shapes fit their neighbours.**
  - `SwitchRequest` owns its fields, as #644's `LaunchRequest` does, and carries `now_ms: i64`, as the merged
    `ReconcileRequest` does.
  - No session-id type exists anywhere. The frozen contract carries ids as `String`, and `PaneId` is an unvalidated newtype.
    So `parse_session_id` plus P0's re-check sits at the contract's own level.
  - The usage message follows `PaneName::parse`'s quoting through `excerpt`.
- **`origin/main` moved during this review** to `e327569` (#641, the host adapter). That commit touches only
  `holler-adapter-host`, `Cargo.lock` and `CHANGELOG.md`, so nothing the brief quotes changed. AC 26's CHANGELOG entry will
  need an ordinary merge.

## Notes for T and F (facts, not findings)

- `rig.record(name)` reads through the logged port (`doctor/rig.rs:182`: "a `get` through the port, so it is in the log").
  `rig.rewrite` calls it. So take AC 2's `mark` after any `record` or `rewrite` call, or `calls.panes` gains a `Get`.
- `PaneError::Timeout { op }` takes a `String` (`error.rs:456`), so write `op: "harness.health".to_owned()`.
- `SESSION_ID_MAX = 64` equals the cut in `error::excerpt` (`error.rs:688-695`), which `findings::quoted` calls. So a typed id
  is never truncated in a message, which is a good reason to give in its doc comment (the outside review's NIT-2).
- `clippy.toml` denies `cognitive_complexity` above 15 and functions over 100 lines (`too_many_lines`). P0 to R written as
  one function would very likely exceed the complexity limit. Reconcile's split into `repair`, `select` and `record`
  (`reconcile/observe.rs:289-386`) is the model for the helpers.

## Patterns referenced

- `crates/holler-cli/tests/profile_verbs/list.rs:4-5` and `show.rs:11`: a shared test module declared once and reached by
  crate path (#662, merged). Also `crates/holler-cli/tests/pane_verbs/doctor.rs:9-11`, `doctor/rig.rs`, and
  `doctor/{read_only,surface}.rs` (`super::rig`).
- `crates/holler-pane/src/reconcile/observe.rs:289-386`: the select, observe and record steps, the clone-and-set write with
  one compare-and-swap, and `screen_text` (`:342-348`).
- `crates/holler-pane/src/findings.rs:111-143, 302-332`: the one remedy table, `doctor_command` and `quoted`.
- `crates/holler-pane/src/error.rs:266-303`: `class_of`, and so the exit code of every code this story raises.
- `docs/adr/ADR-0021.md`: section 8 (lines 265-306), row 340, section 11 (450-456), "Deferred to named stories" (528-537),
  and "Decisions taken" item 2 (547-550).
