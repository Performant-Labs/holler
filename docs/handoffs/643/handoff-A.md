# Handoff-A: Phase 3 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (up-front plan review, amendment 1)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (at d525032; `origin/main` at 0ad2d8a, merge base ce12cdb)
**Brief reviewed:** docs/handoffs/643-brief.md (amendment 1, sha256 53058e77..., last changed in d525032)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" (lines 1348-1365)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

Round 2 of this review (PASS, on the unamended brief) is in git: `git show 04e6af2:docs/handoffs/643/handoff-A.md`.
This file replaces it.

## Why this pass ran

The last run's A-dup BLOCKed on D-1. `SessionSync::of` was a second SHOWN/DRIVEN rule next to
`holler_pane::reconcile::shown_differs` (#647). The MO ruled (a1), the brief was amended, and the outside brief gate
passed on its second round. This is the fresh run's Phase 3. No production code has changed since 06320ad. This pass
reviews amendment 1, which is the plan for T's and F's next round. It checks it against the tree as it is now, which
includes #647 and #662 part 1 (both merged in as 05e7337).

## Summary

PASS, with no blocks and five new warns. Amendment 1 resolves D-1 the way `main` and ADR-0021 stand. `SessionSync`
stays the CLI's three-state view, and the comparison is a call to `shown_differs`, the one rule on `main`. The verbs
read `last_observed` the way that function's doc tells a record reader to. DRIVEN is printed and never compared. The
change stays in the three verb files. It touches no frozen file, no `holler-pane` file and no ADR. ADR-0021 needs no
edit, because §11 and I2 already say what the verbs now do.

The gap is in the Reuse map. The same merge brought in #662 part 1, and the map does not name any of its public
helpers. One of them, `holler_pane::profile_diff::is_member`, is the rule that `watch.rs`'s `Members::names_profile`
restates (W-12). F should fold it this cycle, inside `watch.rs`. W-13 is a one-character guard F should also take.
W-14 to W-16 need no code change here: W-16 needs one `evidence.md` entry this cycle, and the rest are follow-ups.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-12 | warn (new; act on it this cycle) | Decision 7's filter, "an event prints when its record names P (slug compared, as `ProfileName::slug`)"; `watch.rs:194-227` (`Members`; `names_profile` at :221-226); the Reuse map has no row for it | duplication (one rule, two copies) | **The `--profile` filter restates `holler_pane::profile_diff::is_member`.** #662 part 1 (ce12cdb, merged at 19:13 MDT, in this branch through 05e7337) made `is_member(pane, profile)` public (`profile_diff.rs:254-265`). Its doc: "This is what 'a live pane of a profile' means (ADR-0021 section 3): `profile list`'s live count, `profile show`'s comparison and `profile delete`'s refusal `profile-has-live-panes` all use it." `profile/list.rs:59` and `profile/show.rs:89` call it. `Members::names_profile` makes the same comparison: `pane.profile.as_ref().is_some_and(\|p\| p.slug() == self.slug)`. The two agree today, so this is not another D-1. But amendment 1 merged #662 and added Reuse-map rows only for #647's helpers. And the last A-dup ran at 19:08 MDT, before #662 merged, so its "every other place on `main` writes the slug comparison inline" is out of date. #663's unmerged scope plans one more copy (`belongs`, `crates/holler-cli/src/pane/profile_scope.rs:209-213` in its worktree). | F, this cycle: `Members` keeps the scoped `ProfileName` instead of its slug, and `names_profile` becomes `is_member(pane, &self.profile)`. No AC changes, and nothing can be RED: the behaviour does not change, and `watch_profile_prints_only_member_changes` and `watch_profile_prints_a_pane_leaving_the_profile_once` pin it. Record it in `evidence.md` under "Deviations from the brief", since Decision 7 names only `ProfileName::slug`. A-dup checks it. |
| W-13 | warn (new; act on it this cycle) | Decision 3, "`Unobserved` when `pane.last_observed.at == 0`" | contract of the reused object | **The guard restates `shown_differs`'s proviso with a different predicate.** The doc's proviso is "provided `last_observed.at > 0` (`0` is never observed)" (`reconcile.rs:176-178`). `at` is an `i64`, and nothing validates it (`pane.rs:188-189`; no check in the hub's `panes/`). `observed_at`, in the same file, prints `never` for `ms <= 0` (`list.rs:309-316`). With `at == 0` as the only guard, a record with a negative `at` would print `observed-at: never` and `sync: MISMATCH` in the same `pane get` view. No writer produces a negative `at` (reconcile stamps the clock), so this is about consistency, not a live bug. | F: write the guard as `at <= 0`, which is the doc's "provided `at > 0`". Then `SessionSync::of` and `observed_at` agree on what "never observed" means. AC 3's `at` values (0 and 1000) pass either way. T may add an `at: -1` row, but it is not required. Record it in `evidence.md` as a deviation from the letter of Decision 3, and quote the doc. |
| W-14 | warn (new; follow-up) | Decision 3 (the record-reader form lives in `SessionSync::of`); the Forward-compat row for #648 ("the roster may also call [`shown_differs`] directly") | duplication across crates (in future) | **The record-reader form of the rule will have one copy per reader.** `shown_differs` is only the comparison. Its doc leaves the `at > 0` proviso to each reader of a record, and each reader still has to decide what a pane with no session of record shows. Here that is `SessionSync::of`, which is right for `holler-cli`. The gate at `send_prompt` (#646c) lives in `holler-hub`, which cannot use `holler-cli`, so it will write the form again. That gate is deferred until its architecture decision, E-10 (`646-brief.md:32`). If #648 calls `shown_differs` directly, as the Forward-compat row allows, `holler-cli` gets a second copy as well. | No change here: `holler-pane` is outside this blast radius, and ruling (a1) keeps it there. Follow-ups: (1) #648's roster uses `pane::list::SessionSync::of`, not `shown_differs` directly. (2) Before #646c writes its gate, add the record-reader form to `holler-pane` through an amend-first change. It is one function over `&Pane` that answers "cannot compare", "differs" or "same", and `SessionSync::of` then calls it. Do it together with the #647 follow-up on an unseen first observation (Risks), since both concern what a record's `at` proves. |
| W-15 | warn (new, low) | Decision 3's three values; Decision 4's `PaneRow` keys (AC 2 pins exactly nine) | contract shape | **A row cannot explain its own SYNC.** SYNC now comes from `session_of_record`, `last_observed.shown` and `at`. `PaneRow` carries `shown` and `driven`, but neither `session_of_record` nor `at`, and under (a1) `driven` is `null` on every real record until #649. So a script reading `pane list` or `watch` JSON sees `"sync": "mismatch"` and no field naming the session the pane should show. It also cannot tell whether `"unobserved"` means "never observed" or "no session of record" (AC 3's rows c4 and c5). Only `pane get` shows the session of record. | No change now: ruling (a1) and AC 2 keep the nine keys. Adding a `session_of_record` key to `PaneRow` later is additive (ADR-0021 §9: no `schema_version` change). It belongs to #648 (the roster's SHOWN/DRIVEN columns) or to #649 (DRIVEN's writer). Do not rename a value: that change would not be additive. |
| W-16 | warn (new; evidence this cycle, follow-up after) | The Reuse map's `findings::quoted` row; follow-up D-2 | duplication across stories (terminal text) | **There are three rules for printing a stored string, not two.** D-2 counts `text_value` (#643) and `findings::quoted` (#647). #662 part 1 adds a third, the `Display` of `profile_diff::FieldValue` (`profile_diff.rs:174-205`). It escapes only `is_control` characters and never quotes, so a space, an `=` or a bidi override prints as it is. `profile show` prints its stored strings through it (`profile/show.rs:196-198`). Next to it is `write_json_strings` (`profile_diff.rs:211-222`, private), which is `json_text` with a narrower trigger set. The same `ProbeResult` also prints two ways: `probe-last: failed missing=["ok"]` or `-` in `pane get` (AC 7), and `probe: failed (missing "ok")` or `none` in `profile show` (`show.rs:181-194`). `text_value` cannot fold onto `FieldValue`. AC 18 and the `key=value` watch line need values quoted, and dropping the bidi escaping (round 1's W-8) would weaken terminal safety. So it is a non-fold, for the same reasons as `quoted`. | F, this cycle: add one `evidence.md` entry that records the non-fold and these facts. The outside diff gate, A-dup and S read the brief, and the brief does not mention #662. After merge: widen D-2 to cover all three rules and both probe-result renderings, and note that `profile show` writes bidi characters raw. That last point is #662's to fix, and it is the same kind of problem as W-11. |

### Carried from the last run

- **Closed by amendment 1:**
  - D-1 / W-1: Decision 3 adopts `shown_differs`.
  - W-4: kept and documented. Decision 7 and AC 19 pin `pane get` in the watch help.
  - W-9: folded into Decisions 2, 6 and 11 and the Risks bullet.
  - W-7 and W-8 were already closed in round 2.
- **Follow-ups, as the brief lists them:**
  - D-2, widened by W-16;
  - D-3;
  - D-4, which was W-10 (W-12 is the same kind of fix, but inside this blast radius);
  - D-5 and D-6;
  - #647's unseen first observation.
- **Still open, unchanged:**
  - W-5: an empty `watch --until-idle` stream fails `check_ndjson`.
  - W-6: I5 is read per port call for `watch`.
  - W-11: JSON mode writes DEL, C1 and bidi characters raw (#660).

## Checked and consistent with existing patterns (no finding)

- **The right object is extended.** `SessionSync` stays the CLI's presentation of the rule: its three values, its
  serde names and its words. The comparison itself is a call to `shown_differs` (`reconcile.rs:171-181`, `pub`;
  `pub mod reconcile` at `lib.rs:52`), whose doc names `pane get` and the roster as readers. There is one call site per
  path (`PaneRow::from` and `get`'s `detail`), and `watch` reads SYNC through `PaneRow`.
- **AC 3's table matches the rule.** Each of its six rows is what `shown_differs` answers under Decision 3's guards.
  Row c3, an observed home screen, is a mismatch in the doc's own words. AC 1 and AC 2 still read `unobserved`,
  because `sample_pane` has `at: 0` and no session of record (`fixture.rs:63-70`).
- **DRIVEN is printed, not compared.** That fits ADR-0021 §11 (:453-456), I2 (:162) and §1's "written by reconcile,
  never inferred" (:45).
  - Nothing on `main` writes `last_observed.driven`. I re-checked at 0ad2d8a. The only hits are the field itself, the
    test kit's `None`, reconcile's doc line, and unrelated load-test names.
  - Nothing on `main` reads the rule from a record yet. `shown_differs` has no caller outside reconcile, and nothing
    in `holler-hub/src` reads `last_observed`.
- **The false MISMATCH after an unseen first observation is in the right place.** The verbs follow the reader contract
  exactly. The defect is the writer, which stamps `at` while the screen is unseen (`observe.rs:361-375`). That is
  #647's code, as the brief's Risks and Follow-ups say. A workaround in the reader would contradict `shown_differs`'s
  doc.
- **No ADR edit is needed.** ADR-0021 does not state the read verbs' SYNC. §11 states the rule for reconcile, and I2
  makes it the same rule for every reader. Amendment 1 does not change the ADR 0003 rows.
- **Dependency direction and layers are unchanged.** `holler-cli` gains imports from `holler_pane::reconcile`, which
  doctor already imports, and under W-12 from `holler_pane::profile_diff`, which profile list and show already import.
  There is no new crate dependency, frozen file, manifest, wire format or golden file.
- **Size.** The production files are 316, 275 and 227 lines, and the test files 545, 370 and 311. Amendment 1 adds a
  few dozen lines, so every file stays well under 800.
- **Blast radius.** `git diff --name-only origin/main...HEAD` lists exactly the brief's Files, plus `docs/handoffs/643*`.
  `git merge-tree --write-tree origin/main HEAD` exits 0, so the branch merges cleanly with 0ad2d8a (#640 part 2,
  which touches adapter files and `CHANGELOG.md`).
- **Public repository.** The brief's fixtures use `ses-a`, `ses-b`, `demo-*` and `/srv/demo`. A grep of the brief finds
  no personal host, tailnet or account name.

## Notes for T and F

1. **T (RED for amendment 1):**
   - Re-author AC 3 by extending the existing `observed()` helper in `tests/pane_verbs/list.rs` in place, so that it
     also sets `session_of_record` and `at`. Do not add a second helper.
   - Keep `sync_rig()` as the one AC 3 fixture, shared by the `list`, `get` and `watch` tests.
   - W-12 has nothing to be RED. W-13's `at: -1` row is optional.
2. **F:**
   - Make the brief's changes, plus W-12 (`is_member` in `watch.rs`) and W-13 (`at <= 0`).
   - Add `evidence.md` entries for W-12, W-13 and W-16. The file is at 11,696 bytes, and the gate reads only the first
     12,000. Put the new entries first, and turn older entries into pointers where needed, as round 2 did.
   - Name the Reuse-map objects you extended in `handoff-F.md`. `archChanged` is false: no boundary moves.
3. **A-dup:**
   - Check W-12 first: it is in scope and costs three lines.
   - Then check the D-1 fold: one call to `shown_differs`, no comparison of the verbs' own, and DRIVEN never compared.

## Notes for O

There is no O on this automated path. These notes are for the operator or the MO.

1. **No re-plan is needed.** Amendment 1 is consistent with `main` and ADR-0021.
2. **No follow-up has an issue yet** (`gh issue list`, checked at 19:40 MDT). The run's agent or the MO should file
   them when the PR merges. Otherwise they exist only in this directory. The list:
   - D-2 (widened by W-16), D-3, D-4, D-5 and D-6;
   - #647's unseen first observation;
   - W-14, before #646c;
   - W-15, for #648 or #649;
   - W-11, for #660.
3. **The branch is one commit behind `origin/main`** (0ad2d8a). It merges without a conflict. Bring it in before the PR
   merges.

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs` (the module doc at :26-40, `shown_differs` at :171-181), and
  `reconcile/observe.rs:78-93, 224-266, 350-386`.
- `crates/holler-pane/src/profile_diff.rs:174-222, 254-265`, and its callers `crates/holler-cli/src/profile/{list,show}.rs`.
- `crates/holler-cli/src/pane/{list,get,watch}.rs` at d525032 (unchanged since 06320ad), and
  `tests/pane_verbs/list.rs` (`observed`, `sync_rig`).
- `docs/adr/ADR-0021.md` :42, :45, :162, :345 and :453-456.
- Two parallel worktrees, read at about 19:35 MDT as evidence of plans, not merged code:
  `.claude/worktrees/0646-park-close-routing` (brief at 876f87e) and `.claude/worktrees/0663-profile-scope-probe`
  (at 1d6a5ab).
