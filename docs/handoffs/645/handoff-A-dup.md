# Handoff-A-dup: Phase 7 - #645a `pane switch` and `pane reset`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`)
**Diff base:** `dc300ab` (the merge base with `origin/main`)   **Diff head:** `583e9d4` (F's code is `fa59fe7`)
**Reuse map:** `docs/handoffs/645-brief.md`, "Reuse map (extend, do not duplicate)", lines 1052-1065 (no separate `survey.md`)
**Verdict:** BLOCK

## Summary

BLOCK, on one finding that is not a parallel path. F extended every object the Reuse map names. The engine fills the
`tx_switch.rs` stub. It follows doctor's `--fix` repair step for step and reuses `doctor_command`, `quoted`,
`shown_differs`, the remedy table and `ProfileScope::resolve`. The verbs replace their stubs, `reset.rs` reuses
`switch::execute`, and the tests run on the doctor rig, declared once. The one copy, `screen_text`, is the one the brief
justifies (O1).

The block comes from `origin/main` moving. It is now at `d9eabbb`, and #663 there rewrote the ADR-0021 section 8 rule that
this change implements: after a record conflict after the act, a verb prints a reconcile step that **names no pane**. This
branch's section 8 paragraph and its code (AC 9) print the pane doctor command line **for the pane, with `--fix`**. The
merge is textually clean, so git will not flag it. The merged standing spec would then contradict itself, about 60 lines
apart in one section. The fix is one sentence in the #645 paragraph, written after merging `origin/main`. No code or test
changes.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | block | `docs/adr/ADR-0021.md:321-323`, against `origin/main`'s `ADR-0021.md:294-299` (merged tree: 358-360 against 294-299) | Once merged, ADR-0021 section 8 gives two rules for the step printed after a post-act `generation-conflict`. #663's generations bullet says the step names no pane: the profile-scoped or bare form of step 6. The #645 paragraph says it is the pane doctor command line for the pane with `--fix`, and AC 9 (`switch.rs:476-478`) pins that. The merge is clean (`git merge-tree --write-tree HEAD origin/main` gives tree `5bec458`), so nothing downstream catches it. | Merge `origin/main`, then extend the #645 paragraph's reconcile-step sentence to state the pane-scoped form and why it differs from step 6 (Notes for F). The code stays. |
| 2 | warn | `crates/holler-pane/src/tx_switch.rs:141-144` and `:299-307` | The merged tree has two message helpers spelled twice. Both copies are justified in the brief. (a) The lead-in `to reconcile, run `: `origin/main`'s `holler-cli/src/pane/profile_scope.rs:37-56` has `reconcile_step`, whose doc says "verbs call it rather than spell a step of their own". (b) `screen_text`, word for word from `reconcile/observe.rs:342-348`. Neither fold is filed. F-4 still calls `reconcile_step` "planned ... on `issue-663-implementation`" and covers only (a). The comment on (b) says "a follow-up of #645" and names no issue. `docs/handoffs/` is deleted before the push, so after the merge only that comment records the follow-up. | O files one issue, an owner in `findings` beside `doctor_command` for both the lead-in and the screen wording, called by `reconcile_step`, `SwitchFailure::message` and reconcile. The `screen_text` comment then names that issue. No change in 645a. |
| 3 | warn | `crates/holler-pane/src/tx_switch.rs:197-210` | This is the third private copy of the scoped-read arms. With `--profile`, read through `scope.resolve`; for a named pane, `pane_store.get`, else `pane-not-found`. The other copies are `reconcile.rs:222-243` (#647) and, on `origin/main`, `holler-cli/src/pane/park.rs:304-319` (#646), whose doc (`:18`) says it copies reconcile's arms. The brief specifies this read (P1), and it uses the seam the map names, so it is not a block. | Follow-up: one `holler-pane` function for "the records `PANE` and `--profile` name", used by all three, so "a named pane with no record is `pane-not-found`" has one source. |
| 4 | warn | `crates/holler-cli/tests/pane_verbs/switch.rs:31-223` | This runner (`both`, `both_with`, `failed`, `data`, `Case`) is the fourth both-format runner. The other three are `doctor/surface.rs:131` `both_formats` (private), `profile_verbs/rig.rs:94` `run_both` and, on `origin/main`, `pane_verbs/park/rig.rs:220` `run_both`. #646's `run_both` and `assert_failure` keep #662's names "so a later consolidation of the rigs is mechanical". This runner sits on the doctor rig as planned and is richer: a fresh rig per format, a before-snapshot, call marks, a clock window and the I4 check. No shared runner exists, so it is not a block. Its names and argument order differ (`failed(cases, exit, code)` against `assert_failure(both, code, exit)`). The trigger for F-2 has now fired: #643's read-verb rig (`list.rs`) and #646's park rig are on `origin/main`. | File F-2 with this runner in its scope. No change in 645a. |

### Finding detail

**1. Two rules for one reconcile step, on the tree that will land.**

- **On this branch's base** (`dc300ab`, `ADR-0021.md:274-276`), the generations bullet ends "prints the reconcile step (the
  pane doctor command line for that pane)". The brief quotes it as G-4. The code and the #645 paragraph implement it.
- **On `origin/main`** (`d9eabbb`, #663, `ADR-0021.md:294-299`), the same bullet reads: "...and prints the reconcile step
  (the pane doctor command line; the profile-scoped or the bare form of step 6, which names no pane, for the reason given
  there)". Step 6 (`:331-337`) gives the two forms, `to reconcile, run holler pane doctor --profile '<P>' and then holler
  profile show '<P>'` and `to reconcile, run holler pane doctor`. It names `reconcile_step` as their builder, and explains
  that "a pane-scoped doctor refuses a pane with no record or outside P ..., which is what a failed `launch` of a new pane
  leaves". The bullet is the general rule for any verb, not just the spec-editing list below it ("A verb takes its expected
  generation when it plans ...").
- **On the merged tree**, line 296 says the step names no pane, and lines 358-360 say switch and reset print "the pane
  doctor command line for the pane with `--fix`". Section 12 (`:522`) also points at "the reconcile step" of section 8 for
  a `timeout`. For AC 9 the two rules prescribe different lines: `switch_record_conflict_after_the_act` asserts the message
  ends with `; to reconcile, run holler pane doctor demo-c1r1 --fix`.
- **Which rule is right for these two verbs.** The pane-scoped `--fix` form, in my reading. Step 6's reason does not apply:
  the plan read the pane's record, and with `--profile P` the pane is in P, so a doctor run for it is never refused. And
  `--fix` is the repair. It is the remedy table's own form for a fixable mismatch (`findings.rs:135-138`) and doctor's own
  select-then-observe (`reconcile/observe.rs:316-340`). The bare form repairs nothing by itself. #663 widened the bullet
  for launch while #645 was not on `main`. Whoever lands second reconciles, and that is this change. AC 24 allows edits only
  at the end of section 8, so the #645 paragraph is where the exception goes. Editing the bullet itself is outside AC 24,
  and it is #663's text.
- **The other option**, if O rules for step 6's form, is a behaviour change. `holler-pane` cannot call `holler-cli`'s
  `reconcile_step`, so the step would move into `emit_outcome`. `RECONCILE_P` changes in ACs 7, 8, 9, 18 and 19, and the
  printed step loses `--fix`. I do not recommend it.

**2-4** are recorded so that O can file them. They change nothing in 645a. F-1 and F-3 from the brief also still need
filing: `docs/handoffs/` is deleted before the push.

## What F extended (why there is no parallel path)

| Reuse map row | In the diff | Result |
|---|---|---|
| Object to extend: the stubs; analogue doctor's `--fix` (select, observe, record) | `tx_switch.rs` filled: select, then `shown_session`, then one compare-and-swap of a clone, at the generation it read (`:151-175, 280-319`), as `reconcile/observe.rs:318-386`; both verb stubs replaced; no new module, crate or dependency | extended |
| Reconcile step: `doctor_command(Some(&pane), true)` | `tx_switch.rs:141-144` | reused (lead-in: finding 2) |
| Quoting: `findings::quoted` | every id in a message or text line: `tx_switch.rs:73, 138, 255, 272, 294, 305`; `pane/switch.rs:159` | reused |
| SHOWN comparison: `reconcile::shown_differs` | `tx_switch.rs:286` (more uniform than reconcile's own `==` at `observe.rs:324`) | reused |
| Scope and membership: `ports.scope.resolve` | `tx_switch.rs:199-204`; no membership check of its own. The `find` by name (F's deviation 1) is stricter than `next()` and adds no logic of its own | reused (finding 3) |
| Output: `emit`, `ErrorBody`, `ErrorCode`, `VerbCtx`, `ProfileOpt` | `pane/switch.rs:29, 85, 138-143`; no renderer of its own; `reset.rs` reuses `switch::{execute, Verb}`, the `profile/list.rs` and `park.rs` way of sharing within the frozen `mod.rs` | reused |
| Remedy: `FindingKind::ServerDown.remedy(..)` | `tx_switch.rs:235-237`, with a `doctor_command` fallback and no `unwrap`; `holler pane relaunch` is spelled only in `findings.rs` | reused |
| Screen wording: as reconcile's `screen_text` | `tx_switch.rs:299-307`, word for word, with a comment naming the original | justified copy (O1; finding 2) |
| Test world: the doctor rig, declared once | `doctor.rs:10` is the only `doctor.rs` change (`pub(crate) mod rig;`); `switch.rs:22` and `reset.rs:11` use `crate::doctor::rig`; `reset.rs:12` uses `crate::switch::{..}`; no rig declared or copied | reused (finding 4) |
| Envelopes, parsing; clock | `check_envelope`, `run_verb_with`, `try_parse`; `holler_proto::clock::now_millis` | reused |

Stack checks:

- **One path.** The engine and reconcile's repair are the only production callers of `select_session`. No Herdr or host
  call is made, and no prompt is sent, so nothing bypasses `send_prompt`.
- **Codes and contract.** The three codes are open, declared with `from_static`. There is no closed-code, wire, golden or
  persistence change, and no frozen file is touched.
- **Size and naming.** Every touched file is under 800 lines (`tx_switch.rs` 319, test `switch.rs` 599). No personal
  infrastructure name appears.

## Notes for F

1. Merge `origin/main` into `issue-645-implementation` (`git fetch origin` first; it was `d9eabbb` at this review, and the
   merge is clean). The paragraph is then written against the section 8 that will land. This also does the merge T-green
   lists as needed before the PR.
2. In `docs/adr/ADR-0021.md`, edit **only** the "Switch and reset as built (#645)" paragraph. After "...the message ends
   with the reconcile step, the pane doctor command line for the pane with `--fix`, which selects that session again.", add
   a sentence of this substance, in prose with no inline `holler ...` code span (E-7):
   "That is the reconcile step of both verbs after any failure once the act has begun, a record conflict or a `timeout`
   included. It names the pane, unlike the two forms of step 6 that the generations rule above gives. Those name no pane
   because a failed launch can leave a pane with no record. A switched or reset pane always has one, so a pane doctor run
   for it is never refused, and `--fix` is the repair the remedy table gives a fixable mismatch."
3. Do not edit the generations bullet or step 6. They are #663's text, and AC 24 allows only the switch and reset row of
   section 9 (brief row 340; line numbers move with the merge), the end of section 8, the end of section 11 and the
   "Deferred" list.
4. No code or test change: `SwitchFailure::message` and `RECONCILE_P` stay as they are.
5. Check on the merged tree, with `CARGO_BUILD_JOBS=4`:
   - `cargo test -p holler-cli --test docs_cli_test --test pane_verbs`.
   - `grep -n 'Switch and reset as built (#645)' docs/adr/ADR-0021.md` prints one line, between `### 8.` and `### 9.`.
   - `git diff origin/main -- docs/adr/ADR-0021.md` touches only the four places AC 24 allows.
   - `bash scripts/changelog-check.sh`.

## Patterns referenced

- `origin/main:docs/adr/ADR-0021.md:285-343` (section 8 as #663 left it) and `:522` (section 12), against this branch's
  `:265-327`.
- `origin/main:crates/holler-cli/src/pane/profile_scope.rs:37-56` (`reconcile_step`) and
  `crates/holler-pane/src/findings.rs:111-143, 303-334` (the remedy table, `doctor_command`, `quoted`).
- `crates/holler-pane/src/reconcile/observe.rs:316-348` (select, observe, `screen_text`) and `reconcile.rs:222-243`
  (`resolve`).
- `origin/main:crates/holler-cli/src/pane/park.rs:1-45, 300-319` (`in_scope`, and sharing within the frozen `mod.rs`).
- `crates/holler-cli/tests/pane_verbs/doctor/rig.rs`, `doctor/surface.rs:129-155`, `tests/profile_verbs/rig.rs:83-130` and
  `origin/main:crates/holler-cli/tests/pane_verbs/park/rig.rs:1-12, 187-272` (the rigs and their runners).
