# Handoff-A: Phase 3 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (up-front plan review, third pass)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `93fb653`, base `3bdd129`; `origin/main` has moved to `ce12cdb`)
**Brief reviewed:** `docs/handoffs/663-brief.md` (unchanged since `d7e0421`, sha256 `02ca563a...`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" table (this run has no survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, with one block and three warns. This pass replaces the second pass's PASS (`2dad8bf`).

The run came back to Phase 3 because the outside diff gate blocked (round 1) and F reported `archChanged: true`. The brief
has not changed, but `origin/main` moved under it:

- **The block (B-2) comes from that move.** #701 (#647 part 1) merged at 18:56 MDT, after this brief's last amendment
  (18:14) and after the second pass (18:30). It put a public builder of the doctor command line on main,
  `holler_pane::findings::doctor_command`. Its rustdoc calls it the reconcile step that other verbs print, built there
  "rather than spelling it again". The plan still builds its own spelling, writes that spelling into ADR-0021 as the
  unscoped step, and promises it to #644.
- **The fix is O's, not F's.** #647's reviews already marked #663's copy for rejection and left the fix to O. F cannot fix
  it in this run, because the branch's base has no `doctor_command`.
- **The diff gate's two blocks are not plan drift.** Its B-1 asks for a signal that the brief's own pid-reuse rule
  forbids. The brief should state the rule for that path (W-13), and F should not take that remediation. Its B-2 fix names
  an API that std does not have (W-14).

## Findings

Numbering continues from the earlier passes (B-1, and W-1 to W-12).

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-2 | block | Decision 8 ("The two forms are the whole set, and both live in `profile_scope.rs`", brief:1808); AC 5 (brief:1537); AC 14e (brief:1682); F5; the Reuse map; the Evidence "as of `3bdd129`" | duplication; ADRs; pattern consistency | **The doctor command line now has one owner on main, and the plan builds a second one.**<br>**On main:**<br>- #701 added `pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String` (`crates/holler-pane/src/findings.rs:303-316` at `ce12cdb`).<br>- Its rustdoc says: "It is the reconcile step another verb prints after a failure (ADR-0021 sections 8 and 12), so a verb builds it here rather than spelling it again."<br>- Its `const DOCTOR` (`findings.rs:36`) is main's only production spelling of `holler pane doctor`.<br>**The review record on main:**<br>- #647's A, W-4 (`docs/handoffs/647/handoff-A.md:31`), had the builder made `pub` for #663 and #644.<br>- Its A-dup, D-3 (`handoff-A-dup.md:38`), names this brief's `RECONCILE_STEP_UNSCOPED` and `reconcile_step` as the copy. Its direction: "O tells #663 and #644 to build the unscoped step from `findings::doctor_command(None, false)`. O also decides who owns the profile form."<br>- The same review adds: "Each later story's A-dup gate should then reject its own copy" (`:115`).<br>**The plan builds the copy:**<br>- Decision 8 makes `profile_scope.rs` the home of "the whole set", and AC 5 pins the const as a literal.<br>- AC 14e writes the const into ADR-0021 as the unscoped step (branch ADR line 322).<br>- F5 has #644 import the const.<br>- The Reuse map has no `doctor_command` row, because the brief predates it.<br>That is a parallel path with no written justification, which is a Phase 7 BLOCK under this run's own rule.<br>**Stale ADR text:** AC 14e's "profile-scoped until #647 gives `pane doctor` a pane positional" is already false: ADR 0003 now reads `holler pane doctor [PANE] [--fix] [--profile NAME]` (line 61). F's "naming the pane once `pane doctor` takes one (#647) is a follow-up" (branch ADR line 319) is false in the same way.<br>**Why F cannot take it:**<br>- On the branch's base `3bdd129`, `findings.rs` is a 3-line stub with no `doctor_command`, and F does not rebase.<br>- AC 5 and F5 pin the forms.<br>- D-3 makes the choice O's. | Amend the brief per "Notes for O", items 1 to 6, on a branch rebased onto `ce12cdb`. Then start a fresh run. |
| W-13 | warn | Decision 15 (the pid-reuse rule); Decision 19 (the reason list); AC 14a; the Risks | cross-cutting (process safety); ADRs | **The brief does not decide the `try_wait` error, and the outside gate read the gap as a defect.**<br>**The gap:** Decision 15 lets the runner signal only while the leader is unreaped, and it assumes that "until `try_wait` has returned `Some`, the leader is alive or a zombie". An `Err` breaks that premise, and neither Decision 15 nor Decision 19's reason list covers it.<br>**What F built:** F sends no signal on that path and answers a new fixed reason, "the probe's exit status could not be read" (handoff-F, deviation 3; `probe.rs:241-250`). That follows Decision 15.<br>**The gate's B-1:** it calls this a leak and asks for `kill_and_reap` on that path. Its premise, that the child "remains unreaped", is wrong:<br>- In std 1.98.1, Unix `Child::try_wait` is `waitpid(pid, WNOHANG)` (`library/std/src/sys/process/unix/unix.rs:1042-1062`; the pidfd branch runs only when a pidfd was requested).<br>- Its error means the kernel holds no unwaited child with that pid. The child was reaped elsewhere in the process (SIGCHLD set to ignore, or another waiter), so its pid may already be reused.<br>- Signalling the group then reopens the race Decision 15 closes.<br>- No crate's `src` on main installs a SIGCHLD handler or calls `waitpid`, so Holler's own processes never reach this path.<br>**Why a warn:** the code is right. The gap is in the spec the outside gate reads. | **Ruling for F:** keep the current behaviour. Do not adopt the gate's B-1 remediation unless O amends Decision 15's rule.<br>**In the brief:**<br>- **Decision 15:** add "a `try_wait` error (the leader was reaped elsewhere; its pid may be reused) sends no signal; the answer is `Error`, and any group member left stays, as with Decision 16's missing `kill`".<br>- **Decision 19:** add F's two reasons, `the probe's exit status could not be read` and `the probe timeout is too large`.<br>- **The Risks and AC 14a:** name this path as one more documented exception to section 12's "no verb leaves work running", beside the escaped child and the missing `kill`.<br>- **The Evidence:** quote std's `try_wait` source. For the gate's NV-1, also quote `ExitStatus::code`'s doc: "On Unix, this will return None if the process was terminated by a signal." |
| W-14 | warn | Decision 15's cleanup budget (no plan change) | n/a (implementation) | **The diff gate's B-2 is not plan drift, and its fix is not available.**<br>- `kill_and_reap` uses `checked_add(CLEANUP).unwrap_or_else(Instant::now)` (`probe.rs:256-258`). It falls back to a zero budget only if `Instant::now() + 1 s` cannot be represented, which cannot happen in practice.<br>- Even then, the effect is Decision 15's documented degraded path: a leader left unreaped for the caller's exit.<br>- The gate's suggested `saturating_add` does not exist. std 1.98.1's `Instant` has `checked_add`, `checked_sub` and `saturating_duration_since`, and its `+` panics on overflow. | No plan change.<br>Optionally, F measures the budget as a `Duration` from a start `Instant` (`elapsed()`), which needs no addition, or keeps `checked_add` with a comment that says why the fallback cannot be reached.<br>Either way, put the std fact in `evidence.md`, so the next diff round sees it. |
| W-15 | warn | AC 10 ("Both files stay under 600 lines", brief:1650) | file structure | **AC 10's 600-line limit leaves no room for the B-2 fix.**<br>- `profile_scope.rs` is at 597 lines (handoff-F), 296 of them T's test module.<br>- B-2's fix adds at least an import, and option (ii) adds a test.<br>- `scripts/lint.sh` only warns at 600 and fails at 900 (brief H, `lint.sh:43-52`). The 600 rule is the brief's own, sized on an estimate of about 450 lines. | In the B-2 amendment, either set AC 10 for this file to lint.sh's real gate (under 900, with the 600 warning accepted and journaled), or name the trim. |

### The earlier passes' findings

- **B-1: resolved, and still resolved.** ADR-0021 merges cleanly with `ce12cdb`: a three-way `git merge-file` of the base,
  branch and main copies reports no conflict. Only `CHANGELOG.md` conflicts, and the fix is to keep both entries.
- **W-1: superseded by B-2.** The second pass resolved W-1 by hosting the const in `profile_scope.rs`, when
  `doctor_command` did not exist yet.
- **W-5: stands.** It is accepted for Phase 7.
- **W-7 to W-10 and W-12 (2): taken by F at F time**, as the second pass allowed. The brief still has the earlier wording.
  Folding them in is optional (Notes for O, item 8).
- **W-11: still pre-ruled for Phase 7.** F's `Scratch` and `assert_gone_within_2s` are private and minimal, as asked.
- **W-12 (1) and (3): O's, at Phase 11.**

### Also checked (no finding)

- **The profile-scoped step is right on main, for a reason the brief should state.** On main (`reconcile.rs:222-242`):
  - with `--profile P` and a pane, doctor runs `ProfileScope::resolve(P, Some(pane))`, which answers
    `pane-not-in-profile` for a pane with no record;
  - without `--profile`, a named pane with no record is `pane-not-found`.

  A failed `launch --profile P` of a new pane leaves no record, so both pane forms refuse (exit 3) exactly where the step
  must work. `holler pane doctor --profile P` runs, and `holler profile show P` (ADR 0003 line 68, #703) reports the spec
  that has no live pane.
- **No other reuse candidate on main.**
  - No crate's `src` has a POSIX shell-quoting helper, so `single_quoted` duplicates nothing.
  - `findings::quoted` quotes with `{:?}` for a terminal. That is not shell-safe, and it has another job.
- **#701 and #703 change nothing else the plan rests on.**
  - `error.rs` is untouched, so `with_context`'s exhaustive match still compiles after a rebase.
  - `reconcile.rs:219-221` expects exactly the plan's `resolve` refusals.
  - ADR-0021's new text in sections 3, 11 and 12 does not overlap AC 14's hunks.
- **No wire change.** `ProbeResult`'s wire shape is unchanged, so no golden file or protocol doc moves.

## Notes for O

Amend the brief, then start a **fresh** run. Do not use `resumeFromRunId`, which replays this verdict.

1. **Re-base the plan on `ce12cdb`.**
   - Rebase the branch first. Only `CHANGELOG.md` conflicts: keep both entries.
   - Move the Evidence to the new base, and add `findings.rs:303-316` (`doctor_command`), `findings.rs:12-18` (no remedy
     carries a profile), ADR 0003 lines 61 and 68, and `reconcile.rs:222-242`.
2. **The Reuse map.** Add a row for `holler_pane::findings::doctor_command` (#701), the doctor command line's one builder,
   marked **reuse**. Both reconcile-step forms are built from `doctor_command(None, false)`, and `profile_scope.rs` spells
   `holler pane doctor` nowhere else.
3. **Decision 8, the profile form (D-3's first choice). Recommended: compose on top.** Keep the profile form in
   `profile_scope.rs`, built on the builder:
   `to reconcile, run <doctor_command(None, false)> --profile '<P>' and then holler profile show '<P>'`.
   The output is byte-identical, so AC 5's scoped text does not change. This is D-3's "#663 composing on top of it":
   - `findings.rs`'s remedies never carry a profile (#647's Decision 8(a));
   - the POSIX quoting has one caller;
   - the blast radius stays as it is.

   The alternative, a `profile` parameter on `doctor_command`, widens the blast radius into #647's file.
4. **Decision 8, the unscoped form (D-3's second choice). Pick one:**
   - **(i), recommended: a function built on `doctor_command(None, false)`.** For example, `reconcile_step(profile:
     Option<&ProfileName>)` covers both forms and removes #644's branch; a second function also works. AC 5 changes from a
     const equality to a call. F5 tells #644 (and #646) to call the function, and #644 declares no const.
   - **(ii): keep `pub const RECONCILE_STEP_UNSCOPED` as a literal.** A `const` cannot call `doctor_command`.
     - Justify the literal in writing in the Reuse map.
     - Pin it with a test that it equals `format!("to reconcile, run {}", doctor_command(None, false))`.
     - That test passes on the current code, so the brief must call it a regression guard (as AC 8m is), not RED.
5. **AC 14e's ADR text.**
   - Name `findings::doctor_command` as the builder the step is made from.
   - Replace "until #647 gives `pane doctor` a pane positional", and F's follow-up sentence at branch ADR line 319, with the
     reason in "Also checked": on main a pane-scoped doctor refuses a pane with no record or outside P, which is the
     failed-`launch` case, so the step stays profile-scoped (or bare) on purpose.
   - Name the unscoped form as item 4 decides.
   - F5 and the cross-story note to #644 say the same thing.
6. **AC 10.** Set the size rule for `profile_scope.rs` (W-15), or name the trim.
7. **W-13 and W-14.**
   - Write the `try_wait` rule into Decisions 15 and 19, the Risks and AC 14a, with the std evidence.
   - Tell F to keep the no-signal behaviour.
   - Without that rule in the brief, the next outside diff round is likely to raise B-1 again.
8. **Optional.** Fold W-7 (a)-(c), W-8 and W-9's wording, as F built them (handoff-F, design decision 1 and deviation 5),
   into Decisions 5 to 7 and AC 14. The brief then matches the code the gate compares it with.

**A pipeline observation for the operator (not a finding on the plan).** This re-entry came through `nextPhase` case 7
with `archChanged: true` (`coding-pipeline.workflow.mjs:1179`). On that route the driver skips the diff rework classifier
(`:4642`), so T-red and F get no rework note, and F's role doc reads no `*-diff-result-*.md` file. Had this pass been a
PASS, the diff gate's findings would have reached F only through this handoff. If that is a defect, it belongs in
Aftersight.

## Patterns referenced

- `crates/holler-pane/src/findings.rs:12-18, 36, 303-316` at `ce12cdb`: `doctor_command` and the remedy rule.
- `crates/holler-pane/src/reconcile.rs:219-242` at `ce12cdb`: how `[PANE]` and `--profile` resolve.
- `docs/handoffs/647/handoff-A.md:31` (W-4) and `docs/handoffs/647/handoff-A-dup.md:38, 111-115` (D-3), at `ce12cdb`.
- `docs/adr/ADR-0003.md:61, 68`, and ADR-0021 sections 8 and 12, at `ce12cdb` and on the branch.
- std 1.98.1 (rust-docs): `sys/process/unix/unix.rs:1042-1062` (`Child::try_wait`), `ExitStatus::code`, and `Instant`'s
  method list.
