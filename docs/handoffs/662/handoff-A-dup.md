# Handoff-A-dup: Phase 7 - #662b profile write verbs (`holler profile create`, `holler profile delete`)  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`)
**Diff base:** `ce12cdb` (the merge base with `origin/main`)   **Diff head:** `5396f54`
**Reuse map:** `docs/handoffs/662-brief.md`, section "Reuse map (extend, do not duplicate)"
**Verdict:** PASS

## Summary

PASS: no blocks, five warns. F reused every object the Reuse map named and built no parallel path.

**Reused, as the map named:**

- `emit`, `ErrorBody`, `ErrorCode` and `VerbCtx`, with `emit` called once per verb, in the shape `list.rs` and `show.rs`
  use.
- The closed `PaneError` set, with no new code and no `error.rs` edit.
- `ProfileName::parse`, `slug()` and `Actor::parse`.
- `profile_from_panes` for every new record (`create.rs:120`, `:143`, `:173`), so no `Profile` is built by hand.
- `is_member` for the plan check, `delete`'s members and B2's re-read (`create.rs:160`, `:319`; `delete.rs:93`). There
  is no inline slug filter.
- `list::count`, and the shared rig.

**New, and justified in writing:**

- `insert_profile` (B7).
- `single_quoted` (B1's helper, under #663's name, as Phase 3 W-3 asked).
- The `NthCasPut` seam (Decision 13 and C11), now in `rig.rs` as Phase 3 W-4 asked.

**New beyond the brief, each defined once and used by both verbs:** `detach` and `pane_list`. Neither has a counterpart
on `main`.

**Phase 3 W-1 is honoured:** `insert_profile`, `join`, `undo` and `detach` take only the ports and `holler-pane` types
and return `PaneError`.

`origin/main` has moved three commits past the base (`0ad2d8a` #640 part 2, `dc300ab` #642 part 1, `e327569` #641).
All three are adapter crates, and none adds a helper this diff copies.

The warns record copies that this diff, #663 and #664 will multiply if nobody consolidates them, and the ADR gap from
Phase 3. None of them changes F's code.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `src/profile/create.rs:137-139`; `src/profile/delete.rs:83-94`; `src/profile/show.rs:79-90` (on `main`) | **The lookup "the profile or `profile-not-found`" is now written three times in `profile/`, and the members filter twice.** The lookup is `profile_store.get(name)?.ok_or_else(\|\| PaneError::ProfileNotFound { what: format!("{:?}", ..) })`: in `show.rs` (on `main`), in `delete.rs`'s `plan`, and in `create.rs`'s `copy_of`. The members filter is `pane_store.list()?.into_iter().filter(is_member).collect()`, in `show.rs` and in `delete.rs`. `main` has no object to extend: the idiom sits inline in `show.rs`'s private `view`, `show.rs` is "Not touched", and B4 reuses its `{name:?}` form on purpose. F applied "neither verb copies the other's code" to `detach`, `pane_list` and `single_quoted`, but not to this lookup, which `create.rs` and `delete.rs` share. The next consumer is #664: `profile apply` needs the same lookup and the same members, and would add a fourth copy. | No change in this PR. In #664's brief, add one `pub(crate)` lookup (for example `stored_profile(&dyn ProfileStore, &ProfileName) -> Result<Profile, PaneError>`) in the shared-helper home. Add a members helper there too, or put `profile_diff::members(&[Pane], &ProfileName) -> Vec<Pane>` beside `is_member`. Then fold `show`, `delete`, `create --from` and `apply` onto them. |
| 2 | warn | `src/profile/create.rs:253-255`; `src/profile/delete.rs:161`, `:177` | **Suggested command lines are written in more than one place.** `holler profile delete '<P>' --keep-panes` is formatted in `create.rs`'s `JoinFailed::body` and in `delete.rs`'s `has_live_panes`. `holler profile show '<P>'` is formatted in `JoinFailed::body` and in `delete_failed`. #663's `reconcile_step` (`pane/profile_scope.rs:55` at `4bbe607`, not merged) adds a third copy of the `profile show` line. The codebase's pattern is one builder per command line that a remedy names: `holler_pane::findings::doctor_command` (`findings.rs:303-305`: "a verb builds it here rather than spelling it again"), and #663's `reconcile_step` ("verbs call it rather than spell a step of their own"). `main` has no builder for profile-verb commands to extend: `findings.rs` builds only pane-verb commands, and its module doc rules out a profile. B4 also fixes these texts. So this is a warn. | No change now. When the `single_quoted` fold happens (finding 3), add two builders beside it: `profile_show_command(&ProfileName)` and `profile_delete_keep_panes_command(&ProfileName)`. Point both verbs and #663's `reconcile_step` at them. The message texts do not change. |
| 3 | warn | `src/profile/delete.rs:206-208` (and the module doc at `:16-17`); #663's `pane/profile_scope.rs:270` at `4bbe607` | **Phase 3 W-3 is applied, but the duplicate is still coming.** The helper is `pub(crate) single_quoted`, with #663's name and a documented home. #663's branch (`4bbe607`, F done, not merged) still has a private `single_quoted` with the same body. Whichever story merges second puts a second copy on `main`. Also, `profile/` now has two shared-helper homes: `list.rs` holds `count`, and `delete.rs` holds `detach`, `single_quoted` and `pane_list`. Both are documented. The cause is that `profile/mod.rs` is frozen and `list.rs` was outside the blast radius. No issue for the fold is filed yet. | For the MO, depending on which story merges first. **662b first:** #663's Phase 7 (or its rebase) deletes its private copy and imports `crate::profile::delete::single_quoted`. `profile_scope.rs` is #663's own file, so it can make that change. **#663 first:** 662b's rebase cannot reach #663's private copy without leaving its blast radius, so file an issue. The issue moves one copy to a shared home, together with finding 2's builders. When `profile/mod.rs` unfreezes, one shared module in `profile/` replaces the two homes. |
| 4 | warn | `docs/adr/ADR-0021.md:346-347` (changed) against `:395` (not changed); section 3, `:149-155` | **Phase 3 W-2 is still open, and the diff now relies on the wider meaning.** The two section 9 rows gain `profile-conflict`. But the code table still defines it as "The profile moved after the live change (the I8 write order); a race." In this diff, neither verb makes a live change. `create`'s failed undo (`create.rs:249-261`) answers `profile-conflict` when a pane's undo write failed and the profile did not move. Section 3 still does not record `delete --keep-panes` as a second exception to I1. The overlay says to update the ADR in the same change. This case is a warn, not a block, for the same reasons as at Phase 3: the brief records the wider meaning (C2, Decision 7), AC 6 forbids the edit, and F and T both list it as a follow-up. **No issue is filed yet.** | Before merge, the MO either relaxes AC 6 or files the follow-up. Relaxing AC 6 means two sentences in ADR-0021, which change no code or test. (i) Widen the `profile-conflict` Reason at `:395` to "a later write of a multi-write verb, or its compensation, lost after an earlier write landed". (ii) After section 3's `--from-current` sentence, record `delete --keep-panes`'s writes to `Pane.profile`. Neither sentence is near #663's ADR hunks. |
| 5 | warn | `tests/profile_verbs/create.rs:33-51`; `tests/profile_verbs/delete.rs:17-25` | **The test files repeat helpers.** `stored(rig, text)` is defined in both files. The two files also count writes in two different ways: `pane_writes` and `profile_writes` in `create.rs`, and `has_call` in `delete.rs`. `rig.rs` is the designated place to extend: #664 and #665 extend it, and this diff already moved the seam and `assert_message_contains` there. Neither helper copies anything on `main`. The per-file `argv` helper (`create.rs:28`) is the workspace's established idiom: six copies predate this diff, including `show.rs:15`. So `argv` is consistent and is not part of this finding. | No change now. When #664 extends `rig.rs`, move `stored` and one write-count helper there, so `apply.rs` does not become the third copy. |

## Checked and not a finding

- **`insert_profile` (`create.rs:208-219`)** is the first profile create write in the workspace. No production code
  calls `ProfileStore::cas_put(.., 0, ..)`. The would-be callers are still stubs: `profile/import.rs`, `profile/apply.rs`,
  `pane/import.rs` and `holler-pane/src/import.rs`. The hub's `profile/rename.rs` is a `not-implemented` skeleton.
- **`detach` (`delete.rs:149-155`)** is the only production write of `Pane.profile`. A grep for `profile: None` and
  `profile: Some(` in `crates/*/src` finds only this diff and the stores' own internals. It serves both
  `delete --keep-panes` and each leave step of `create`'s undo, so the leave write exists once.
- **`pane_list` (`delete.rs:212-219`)**: nothing else joins pane names. The other `", "` joins in `holler-cli` join
  other things (`hub_cmd.rs`, `show.rs:188`), and the `"none"` at `show.rs:183` is a probe result.
- **`single_quoted` against `findings::quoted` (`findings.rs:332`)**: the purposes differ. `quoted` is a `{:?}` excerpt
  of an untrusted value, and `single_quoted` makes one POSIX shell word. `main` has no production POSIX quoting. Its
  only copies are test-local: `tests/multiword_command_test.rs:63`, and `holler-adapter-host/tests/common/mod.rs:155`,
  which #641 added.
- **The `pane-in-other-profile` plan check (`create.rs:156-178`)** is composed from `is_member`, as the map's "no inline
  slug filter" asks. The store-side checks stay the authority, in their own layers: the hub's private
  `refuse_profile_move` (`holler-hub/src/panes/store.rs:337-357`) and the test kit's `pub(crate) check_membership`
  (`pane_store.rs:225-240`). The check before any write is the brief's carried Decision 2, and it mirrors ADR-0021
  section 8 step 1.
- **The `ErrorBody` built by hand (`create.rs:242-248`, `delete.rs:132-138`)** has the form B4 and E7 prescribe.
  `output.rs` is frozen and has no helper that keeps the code and changes the message. Its own `not_implemented` and
  `usage_body` build `ErrorBody` the same way. #663's private `with_context` edits a `PaneError` payload instead. It is
  not on `main`, so it can only be a later fold candidate.
- **`NthCasPut` (`rig.rs:240-306`)** is the one planned new double, and its only state is the counter. The other
  delegating `PaneStore`s, `GuardWatcher` and `Mutant` in `holler-pane-testkit/tests/pane_store_conformance_test.rs`,
  live in another crate's test target and break the contract on purpose. `FaultSwitch` has no N-th-call API: it has
  only `set`, `fail_next`, `set_delay` and `calls`. `run_both` behaves as before, because it delegates to
  `run_both_with(.., Rig::run)`.
- **Layering and structure:**
  - The change is in the CLI verb layer only. No frozen file, manifest, test-kit, `holler-pane` or hub file changes.
  - The new edges are `create -> delete`, `create -> list` and `delete -> list`, with no cycle.
  - The largest touched file has 564 lines, against a lint limit of 900.
  - No `#[allow]` is added, and no protocol, golden or `holler-proto` file is touched.
  - The diff and the commit metadata name no host, account or tailnet. The identity is the no-reply address.

## Out of scope but noticed (for #663's own Phase 7, not this diff)

- #663's private `belongs` (`pane/profile_scope.rs:219-222` at `4bbe607`) nearly copies `profile_diff::is_member`, which
  has been on `main` since 662a.
- #663's `StoreScope::stored` (`:77-81`) gives `profile-not-found` the unquoted `what` `profile.to_string()`. 662a and
  662b use `{:?}` (B1). A script that matches the message would see two forms.

## Notes for F

None. The verdict is PASS. Findings 1-5 are follow-ups:

- **#664's brief:** findings 1 and 5.
- **The `single_quoted` fold:** findings 2 and 3, owned by whichever of #663 and 662b merges second, or by a filed issue.
- **The MO, before merge:** finding 4, which means relaxing AC 6 or filing the ADR-0021 follow-up.
