# Handoff-A: Phase 3 - #662b profile write verbs (`holler profile create`, `holler profile delete`)  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `79fc998`; this run is 662b only)
**Brief reviewed:** `docs/handoffs/662-brief.md` (sha256 `c8ac6f6a5ec99f39...`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 1513-1526 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: 0 blocks, 4 warns. The plan extends the objects `main` already has, at the layer that ADR-0021 and the issue give it.
It builds no parallel path.

- **Thin verbs over the ports.** `create` and `delete` are CLI verbs over the two store ports, like 662a's `list` and `show`:
  - the name is parsed in `run`;
  - each verb prints once, through `emit` or `emit_error`;
  - codes come from the closed `PaneError` set, and exit codes from `class_of`;
  - the data is derived `Serialize` structs.

  The plan touches no frozen file (`profile/mod.rs`, `output.rs`, `error.rs`, the ports) and no manifest. It calls no
  adapter, scope or prober.
- **Reuse of 662a.**
  - Every new record comes from `profile_from_panes`. Its doc already calls it the builder of a new profile (generation 0,
    the store stamps it).
  - Membership is `is_member` in all three places.
  - Plurals use `list::count`.
- **Write order.** Both verbs follow ADR-0021 section 8's two-registries rule:
  - the profile is written before any pane joins it, because the hub's `check_membership` requires it;
  - every pane leaves before the profile is deleted.

  The `pane-in-other-profile` check runs before any write, mirroring the I8 plan step. The pane registry's own check stays
  the authority.
- **The create race.** B7's mapping matches the real store (`holler-hub/src/profile/store.rs:194-206`: the name rule
  first, then `next_generation`). It also matches the fake, which checks the name rule first.
- **The three new objects** are `insert_profile`, `shell_word` and the N-th-call seam. None has a counterpart on `main`, and
  the brief justifies each in writing. One verb file sharing a helper with a sibling follows 662a's documented
  `super::list::count` precedent (`profile/mod.rs` is frozen).
- **I5.** The per-call bound satisfies I5 (ADR-0021 "Decisions taken", item 3). The undo stops at its first failed step, so
  a wedged store costs at most two bounds, not N.

The four warns are about the record and about later reuse. None changes the code F writes now.

- **W-1, the most consequential.** B7 and the Forward-compat table promise #650 a reuse that the crate graph forbids.
- **W-2.** This run widens two meanings in ADR-0021: `profile-conflict`, and the I1 exception for `delete --keep-panes`.
  AC 6 forbids writing either into the ADR.
- **W-3** concerns the name and home of `shell_word`.
- **W-4** concerns where the test seam lives.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | B7 (lines 1353-1357) and the Forward-compat row for #650 (line 1536) | dependency direction; reuse across stories | B7 says #650 and #665 "call it rather than re-deriving it". The #650 row says #650 is "Satisfied by `insert_profile`; the join order and undo of B2 are the pattern to follow". But ADR-0021 section 5 puts #650's transaction engine in `holler-pane/src/import.rs`, and that stub's doc says it will "create the `fleet` profile and its panes". `holler-pane` cannot depend on `holler-cli` (section 5's crate rules), so the engine cannot call `crate::profile::create::insert_profile` at any visibility. "The pattern to follow" then invites #650 to copy the race mapping and the join and undo into `holler-pane`: the parallel path B7 means to prevent. The placement is right for this run. The issue gives #662 no `holler-pane` file that may call a port (`profile_snapshot.rs` is documented as calling none), and #665's `profile import` can reach the helper. | Amend B7 and the #650 row: `insert_profile` serves `holler-cli` callers, that is #665, and #650 only if its CLI verb makes the profile write. Add a Follow-up: when #650's engine needs the create write, #650 moves `insert_profile` down into `holler-pane` as a spec'd refactor-to-extend, with the join and undo if it reuses them. One copy remains, and `create.rs` calls it; #650 does not copy anything. To keep that move mechanical, F keeps `insert_profile` and the join and undo functions free of CLI types. They take `&dyn ProfileStore` and `&dyn PaneStore` and return `PaneError` or plain values, and only `run` builds the `ErrorBody`. B7's signature already does this. |
| 2 | warn | Decision 7 and C2 (lines 1213-1219, 1251-1254); B4's "undo step failed" row (line 1340); AC 2i (lines 1435-1438); C7 (lines 1230-1232); AC 6 (lines 1476-1479) | ADR consistency | (a) ADR-0021 defines `profile-conflict` as "The profile moved after the live change (the I8 write order); a race" (section 9, line 395; `error.rs:429` says the same). The two rows that Decision 14 (ii) edits will list it for two verbs that make no live change. AC 2i pins it for a failed pane undo, where the profile did not move. The closest rule, in section 8 (line 302), codes a pane-record conflict as `generation-conflict`. Decision 7's wider meaning ("an earlier write of the same verb landed and a later one lost") is written only in the brief. C2 covers the rows, not the definition, and a stable code's meaning is fixed once it merges (line 329). (b) `delete --keep-panes` writes `Pane.profile`, which makes it a second exception to I1 ("Only pane verbs change a pane or its registration"). Section 3 records the first exception (`--from-current`, lines 149-152), but ADR-0021 never mentions `--keep-panes`. C7 still cites section 3 for both. AC 6 ("and nothing else") forbids recording either one. | Relax AC 6 to allow two more ADR-0021 edits in this change. Neither is near #663's hunks (sections 1, 2, 5, 8 and 12) or #644's (section 9's launch row). (i) Widen the `profile-conflict` Reason, for example: "A later write of a multi-write verb, or its compensation, lost after an earlier write landed (after the live change in the I8 write order, or after the membership writes of `profile create` and `profile delete`); a race. The message names the reconcile step." (ii) After section 3's `--from-current` sentence, add: "`profile delete --keep-panes` first sets each member's `Pane.profile` to none by compare-and-swap (the panes keep running), then deletes the profile (#662)." In Decision 7, also say why a failed pane undo is `profile-conflict` and not section 8's `generation-conflict`: the code tells a script whether partial state remains. If AC 6 stays as it is, file (i) and (ii) as a follow-up. |
| 3 | warn | B1, `shell_word` in `delete.rs` (lines 1307-1313); the Follow-up "One `shell_word`" (lines 1550-1552) | naming; duplication (future) | No production helper for POSIX quoting exists on `main`; the only one is the test-local `sh_quote` (`tests/multiword_command_test.rs:63`). So B1 duplicates nothing that has merged. But #663's branch has the same body as a private `single_quoted` (`pane/profile_scope.rs:261` at `1d6a5ab`), so two names and two copies will exist once both merge. The follow-up says whichever merges second folds them, which cannot happen if 662b merges second: that file is #663's and outside 662b's blast radius. On placement, `list.rs`'s module doc (lines 8-9) makes `list.rs` the home of the profile verbs' shared helpers ("because the frozen `profile/mod.rs` admits no new module"), and `list.rs` is in #662's blast radius. B1 opens a second home without that note. | Name it `single_quoted`, as #663 does, so the fold is a move and not a rename. Either put it beside `count` in `list.rs`, under the existing sentence, or give `delete.rs`'s module doc the same sentence. Give the follow-up an owner: #663 if it merges second (it can import `crate::profile::...`), otherwise a filed issue. |
| 4 | warn | Decision 13 and C11: the N-th-call seam, local to `tests/profile_verbs/create.rs` (lines 1285-1289, 1499-1501) | test structure; duplication (future) | The new double is justified: `FaultSwitch` fails only the next call of a method (`fault.rs:72-77`), and no `PaneStore` wrapper exists under `holler-cli/tests`. But a seam local to `create.rs` cannot be shared. `delete.rs` needs the same double to reach B4's "detached so far: a" branch, a failure at the second detach that AC 3g leaves untested. #664's apply tests will need it too. The shared rig is `rig.rs`, which "#664 and #665 extend". | Put the seam in `rig.rs` as `pub(crate)`. It still delegates to `FakePaneStore` and holds only a counter and a fixed plan; `create.rs` uses it from there. Note a follow-up for #638, which owns the test kit: an N-th-call plan in `FaultSwitch` is the eventual home, shared by every fake. |

## Notes for O

PASS: no amendment is required before T. If the MO amends the brief anyway, W-1 and W-2 cost least now, because both
change only the brief's text.

- W-1 corrects B7 and the #650 row and adds a Follow-up.
- W-2 relaxes AC 6 to allow two ADR-0021 edits, or files them as a follow-up.

Neither changes F's code or any AC 2-3 test.

I checked the outside model's needs-verification items against the worktree:

- NV-1 and NV-2 hold: `profile_snapshot.rs:70-81` sets `slug = name.slug()`, `generation` 0, and `created` and `updated` 0.
- NV-5 holds: `rig.rs:57-67` sets `scope: &UNWIRED`.
- NV-6 holds: `PaneName` serializes as a string (`pane.rs:62-66`), and its grammar is ADR 0005's lowercase letters, digits
  and `-`.
- NV-8 holds: `docs_cli_test.rs:140-155` drops the bracketed group before it splits on the pipe. The AC 5 rows therefore
  normalise to `holler profile create NAME` and `holler profile delete NAME`.
- NV-9 holds: `single_quoted` is private on #663's branch.

The outside model's W-1 to W-3 are about test coverage, which is T's call.

One point outside A's dimensions, for T and S. B2's reasoning ("a join that answered `timeout` may have landed") applies
equally to the profile write inside `insert_profile`. If that write times out after landing, the verb reports `timeout`
while a profile with specs and no members may be stored. Running the verb again then gives `profile-exists`. The brief does
not say whether the message should mention this.

## Patterns referenced

- `docs/adr/ADR-0021.md`:
  - section 3 (lines 149-155), section 5 (lines 170-193), section 8 (lines 265-306) and section 9 (lines 322-402);
  - "Decisions taken", item 3.
- The thin-verb pattern: `crates/holler-cli/src/profile/{list,show,mod}.rs`, `crates/holler-cli/src/pane/doctor.rs` and
  `crates/holler-cli/src/output.rs`.
- The records, the ports and the stores:
  - `crates/holler-pane/src/{profile_snapshot,profile,ports,import}.rs`;
  - `crates/holler-hub/src/profile/store.rs:190-258`.
- The test fakes and the rig: `crates/holler-pane-testkit/src/fault.rs` and `crates/holler-cli/tests/profile_verbs/rig.rs`.
- In flight, used as evidence of intent and not as patterns:
  - #663's `crates/holler-cli/src/pane/profile_scope.rs` at `1d6a5ab`;
  - #663's brief's list of ADR-0021 edits (sections 1, 2, 5, 8 and 12).
