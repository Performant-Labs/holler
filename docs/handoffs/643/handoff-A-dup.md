# Handoff-A-dup: Phase 7 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (anti-duplication gate, amendment 1)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (worktree `.claude/worktrees/0643-read-verbs`)
**Diff base:** `d525032` (amendment 1's brief; this cycle's code is `d525032..c65993b`; merge base with `origin/main` is
`ce12cdb`)   **Diff head:** `c65993b`
**`origin/main` now:** `e327569` (#641, merged at 20:06:15 MDT), three commits past the merge base, all in adapter crates
**Reuse map:** `docs/handoffs/643-brief.md`, "Reuse map (extend, do not duplicate)" (lines 1348-1365), plus
`handoff-A.md`'s W-12 (`holler_pane::profile_diff::is_member`)
**Verdict:** PASS

Round 1 of this gate (BLOCK on D-1) is in git: `git show d6d22d7:docs/handoffs/643/handoff-A-dup.md`. This file
replaces it.

## Summary

PASS, with no block and no new warn. F folded D-1 the way the MO's ruling (a1) and the Reuse map said:

- `SessionSync::of(&Pane)` now calls `holler_pane::reconcile::shown_differs`.
- It is the only reader of that rule from a record in the workspace. It has two call sites, `PaneRow::from` and
  `get`'s `detail`, and `watch` reads SYNC through `PaneRow`.
- `last_observed.driven` is printed and never compared.

F also folded W-12. `watch --profile` membership is now `holler_pane::profile_diff::is_member`, and the inline slug
comparison (`Members::names_profile`) is gone.

On the test side, T extended the existing `observed()` fixture in place and wrote one `SYNC_WANT` table that the three
verbs' AC 3 tests share, as A's Phase 3 notes asked. This cycle adds no new object.

`main` moved three commits during the run: #640 part 2, #642 part 1 and #641. They touch only adapter crates,
`Cargo.lock` and `CHANGELOG.md`. `holler-cli` depends on none of those crates, and the verbs copy nothing from them.
The branch merges with `e327569` without a conflict.

## Findings

No duplication; the extension is clean. No drift was introduced during the rework.

### Checked this cycle (no finding)

- **The D-1 fold (A's Phase 3 note 3, second check).**
  - **One call.** `shown_differs` has one caller outside reconcile, `SessionSync::of` (`list.rs:235`). Reconcile's own
    pass is the other (`reconcile/observe.rs:245`).
  - **No comparison of the verbs' own.** The guard `observed.at <= 0 || record.is_none()` (`list.rs:233`) decides only
    that there is nothing to compare. The comparison itself is the call. The guard comes from two places on `main`:
    - the reader's proviso in `shown_differs`'s doc, "provided `last_observed.at > 0`" (`reconcile.rs:176-178`);
    - doctor's own `let Some(record) = .. else` (`observe.rs:242-244`).
  - **DRIVEN is never compared.** `src` reads `last_observed.driven` in two places, `PaneRow::from` (`list.rs:179`)
    and `get`'s `driven:` line (`get.rs:178`), and both only print it.
  - **The JSON shape is unchanged.** `SessionSync` keeps its three variants and `rename_all = "snake_case"`
    (`list.rs:209-221`), so `PaneRow`'s keys and the JSON values stay as they were.
  - **Nothing better to call.** `holler-pane` has no public function that gives a record's three-way SYNC answer.
    The public functions in `reconcile.rs` and `findings.rs` are `shown_differs`, `reconcile`, `quoted`,
    `doctor_command` and `FindingKind::remedy`.
- **The W-12 fold (A's Phase 3 note 3, first check).**
  - `Members` holds the `ProfileName` (`watch.rs:198-204`) and calls `is_member` (`watch.rs:215`).
  - No `.slug()` comparison is left in `holler-cli/src`.
  - `is_member` now has three callers: `profile list`, `profile show` and `pane watch`.
- **W-13.**
  - The guard is `at <= 0`. That matches `shown_differs`'s proviso and `observed_at`, which prints `never` for the
    same values (`list.rs:328-335`).
  - `list.rs` now spells the predicate twice, and the two spellings agree. They are not a separate helper to fold:
    W-14 already tracks moving the whole record-reader rule into `holler-pane`.
- **Tests.**
  - `observed()` gained `record` and `at` in place (`tests/pane_verbs/list.rs:112-130`) and is still the one fixture.
    `a_stored_dash_prints_apart_from_the_empty_value` now uses it and no longer sets `session_of_record` by hand
    (`tests/pane_verbs/get.rs:376`).
  - `SYNC_WANT` (`tests/pane_verbs/list.rs:333-340`) is one table, read by the AC 3 tests of `list`, `get` and `watch`.
  - Neither existing fixture source is copied. The test kit has no builder for an observed pane: `fixture.rs` exports
    `sample_pane`, `sample_spec` and `sample_profile`. The doctor rig derives a session of record from a live world
    (`doctor/rig.rs:386-390`, private), which is a different job.
  - The one inline closure, `as_json` (`tests/pane_verbs/list.rs:367-373`), has no matching helper anywhere.
- **`main`'s three new commits.**
  - The only new helpers with a related name are tmux's `escape` and `escape_cwd`
    (`holler-adapter-host/src/tmux.rs:155-172`, `pub(crate)`).
  - They escape a value for tmux's command parser (`;` and `#`), not a stored string printed to a terminal. Also,
    `holler-cli` does not depend on that crate.
  - So D-2 still counts three rules.
- **The overlay's stack checks.**
  - Since the merge base, the branch touches no prompt path, wire format, golden file, persisted state, hub code,
    frozen file, manifest or ADR.
  - The Phase 7 candidates are neither touched nor copied: `token.rs`, `Lockout`, `Roster`, `log(Severity, ...)`,
    `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`.
  - The largest touched file is `tests/pane_verbs/list.rs`, at 587 lines.
  - No added line names a personal host or account, and every commit on the branch uses the GitHub no-reply address.
- **Every Reuse-map row, re-checked after the rework.**
  - Output goes only through `emit`, `emit_stream`, `emit_error` and `ErrorBody::from`.
  - `ProfileOpt` is flattened in all three structs.
  - The only port calls are `PaneStore::{get, list, watch}`, `ProfileStore::get` and `ProfileScope::resolve`. The
    AC 17 and AC 21 greps print nothing.
  - Names are typed with `PaneName::parse` and `ProfileName::parse`.
  - Positions go through `GridPos`, and JSON uses the records' own serde forms.
  - The verbs raise only the closed `PaneNotFound` and `PaneNotInProfile`.
  - `observed_at` calls `format_epoch`.
  - `findings::quoted` is not used, which is the brief's decided non-fold.
- **Merge.**
  - `git merge-tree --write-tree origin/main HEAD` exits 0 against `e327569`.
  - `git diff --name-only origin/main...HEAD` lists the brief's ten files plus `docs/handoffs/643*`.

### Carried (not re-rated; the brief lists these as follow-ups)

- **D-2**, widened by W-16: three rules print a stored string: `text_value`, `findings::quoted` and `FieldValue`'s
  `Display`.
- **D-3**: two fakes rigs in `pane_verbs`. This cycle's change to `observed()` stays inside #643's own rig and does not
  make the overlap worse.
- **D-4**: `profile_name` versus doctor's inline `--profile` guard.
- **D-5**: `help()` (`tests/pane_verbs/get.rs:19-29`) still rebuilds `verb_harness::parse::try_parse`. The file changed
  this cycle and T left `help()` alone, which the brief allows ("T may take it then"). The fix is still three lines.
- **D-6**: AC 14's inline poll versus `support::wait_for`, unchanged.
- **The rest:**
  - W-14: #648 should call `SessionSync::of`, and the record-reader rule moves into `holler-pane` before #646c.
  - W-15: a row cannot explain its own SYNC.
  - W-5, W-6 and W-11, unchanged.
  - #647's unseen first observation, which can show a false MISMATCH.

## Notes for F

None (PASS).

## Notes for O

There is no O on this automated path. These notes are for the run's agent and the MO.

1. **Bring `origin/main` (`e327569`) into the branch before the PR merges.** It merges without a conflict.
2. **File the follow-ups when the PR merges.** None has an issue yet. The list is in `handoff-A.md`, Notes for O, point
   2.

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs:171-181` (`shown_differs`) and `reconcile/observe.rs:225-262` (doctor's call).
- `crates/holler-pane/src/profile_diff.rs:254-265` (`is_member`) and its callers in
  `crates/holler-cli/src/profile/{list,show}.rs`.
- `crates/holler-pane-testkit/src/fixture.rs`, `crates/holler-cli/tests/pane_verbs/doctor/rig.rs` and
  `crates/holler-cli/tests/verb_harness/parse.rs`.
- On `origin/main`: `crates/holler-adapter-host/src/tmux.rs:155-172`.
