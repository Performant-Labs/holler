# Handoff-A-dup: #650 - Import today's panes from the launcher's files  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** `issue-650-implementation`
**Diff base:** `243f910` (A's plan review; the story's code is all after it; merge base with `origin/main` is `7284001`)   **Diff head:** `f5c75d3`
**Reuse map:** the brief's evidence E1 to E46 (`docs/handoffs/650-brief.md`) and the "Deliberate copies" list in `docs/handoffs/650/handoff-A.md`
**Verdict:** PASS

## Summary

PASS: 0 `block`, 2 `warn`. F extended or reused every object the map named. The production diff has no parallel path. Both
warns are in the test rig and can be fixed later.

**Reused, not rebuilt** (production):

- `insert_profile`, for the profile write, so the create race maps to `profile-exists` in one place.
- `profile_from_panes`, for the planned profile.
- `push_spec`, for the dry run's spec text. It is widened, visibility only. `text` was not widened because nothing calls it,
  and no copy of it exists.
- `holler_body::config::load`, for `sessions.toml`. There is no second parser.
- `FindingKind::code` and `PaneCode::as_str`, for the six shared finding codes. No code is retyped.
- `findings::quoted` and `embedded`, for untrusted text.
- `profile_diff::is_member`, for the undo's check that a write landed.
- `single_quoted`, `pane_list` and `count`, for the remedy and summary text.
- `pane::list::{optional_text, NO_VALUE}`, for the text cells.
- `tx_launch::HOST_NAME`.
- The `RefusalCode::from_static` constant pattern, for `IMPORT_BLOCKED`, as `tx_launch` and `tx_switch` declare theirs.

**New, as the brief names it:**

- `ImportFinding`, which copies `FindingKind`'s pattern (E29).
- The readers of `pane.env`, the launcher table, `last-session` and `owned-sessions`.
- The import's undo, which deletes where `create` detaches (operator decision 5).

I looked for existing code each of these could have reused and found none:

- an env-file reader (`holler-load-test`'s `parse_env` reads CLI flag pairs, in another binary);
- a helper that turns a byte offset into a line and column;
- an `hj-` constant;
- a public `Pane` builder. Launch's `record` is a private method of the frozen `tx_launch.rs`, and it records other values.

**Copies.** The only copies are the deliberate ones from the frozen `reconcile.rs` and `reconcile/observe.rs`. There, each
item is private or bound to a `&Pane` record:

- the thread-per-pane scaffold (`observe_all`, `joined`);
- the server classification (`observe_server`'s `Ok(false)` arm);
- `failed_message`;
- the call-name constants;
- the tmux classification and the "not observed" message, which join item 3 (`:29`) tells the import to classify "as
  reconcile's own tmux check" does.

The copy is narrower than reconcile's. The import cannot take the public `ObservedHealth`, because it records `unknown` for
`Ok(true)` followed by a failed list (brief `:34`, T decision 2). `ObservedHealth::Healthy` cannot tell that case apart, and
`recorded()` is private.

**Tests.**

- The rig extends doctor's world as the brief says. `Seed::unrecorded` is the only edit to `doctor/rig.rs`.
- The rig reuses `Rig`, `Seed`, `Calls`, `mark`, `calls_since`, `unregistered_herdr_pane` and the fake's `rename`.
- `ExtraHerdr` and `NthCasPut` are justified in writing (A's list, `handoff-T-red.md:84-88`). `ExtraHerdr` appends a pane in
  another session, which `DisplacingHerdr` cannot do: it only moves the one pane it has.

**The reuse already paid off.** `origin/main` moved after this branch's base: #644 merged as `b8242d7`. It added
`SpecField::OpencodeAgent` (`profile_diff.rs`). The import prints specs through `push_spec` and builds the profile through
`spec_from_pane`, so the merged tree prints and compares that field with no change here. The import's spec-text test checks
for specific lines, not a field count. A trial `git merge-tree --write-tree HEAD origin/main` is clean. The only files both
sides change are `CHANGELOG.md`, `docs/adr/ADR-0003.md` and `docs/adr/ADR-0021.md`.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/import/rig.rs:371`, `:383`, `:393` | Three assertion helpers re-implement `pub` helpers of the same test target (`pane_verbs`), all reachable from this rig:<br>- `findings` has the same body as `doctor::rig::json_findings` (`doctor/rig.rs:357-364`), renamed. The rig already imports `Calls`, `Rig` and `Seed` from that module.<br>- `data_of` makes the same three checks as `doctor::rig::json_data` (`:345-354`): exit 0, a valid envelope, nothing on err.<br>- `error_of` has the same signature and body as `launch::rig::error_of` (`launch/rig.rs:606-613`), which `relaunch.rs:16` already imports.<br>No handoff justifies these three: T-red justifies only `ExtraHerdr` and `NthCasPut`. Not a block: `pane_verbs` has no dominant pattern for reading an envelope (close, launch and doctor each keep a `data_of` or `json_data`; switch and park read it inline), and the copies are test-only. | Delete the three. Use `use crate::doctor::rig::{json_data, json_findings};` and `use crate::launch::rig::error_of;`, or re-export them under the local names. No other file changes. Fold this into the next T pass if one runs; otherwise make it a small follow-up. |
| 2 | warn | `crates/holler-cli/tests/pane_verbs/import/seams.rs:16`, `:67` | Each seam is a justified copy, but there are now enough copies to plan consolidating them:<br>- `pane_verbs` has two pass-through `HerdrPort` wrappers: `launch/displaced.rs::DisplacingHerdr` and `import/seams.rs::ExtraHerdr`. Each has seven delegating methods and differs only in `snapshot`.<br>- The workspace has three failing-N-th-`cas_put` seams: `profile_verbs/rig.rs::NthCasPut` (`:257`), `pane_verbs/park/failures.rs::FailNthCasPut` (`:24`) and `pane_verbs/import/seams.rs::NthCasPut`, a verbatim copy of the first.<br>They exist because `profile_verbs` is a separate target and the test kit's `src/` was frozen for this story. | A follow-up issue, not this story. When the test kit unfreezes, give `holler-pane-testkit` a failing-N-th-write `PaneStore` wrapper (`Fail` and `ApplyThenFail`) and a `HerdrPort` pass-through with a snapshot hook. Then replace the three seams and the two wrappers with them. |

There is no duplication in the production diff, and the extension is clean.

**Checked, no finding:**

- **How `pane_line` prints a role.** It writes `PaneRole`'s serde name, as `profile_diff`'s `serde_text` does and as
  `args.rs::parse_role` reads one. The only other role-text function is `get.rs`'s private `role_word`, so no shared helper
  exists to reuse.
- **A pane verb calling profile-verb helpers.** The brief directs it (E5, E26), and the helpers are `pub(crate)` for outside
  callers (`insert_profile`'s doc says so).
- **`assert_no_store_write`.** It is on A's deliberate list. Doctor's `Calls` is not launch's, and launch's
  `assert_untouched` forbids the adapter reads that a dry run must make.
- **`one_pane()` in both `import/join.rs` and `import/sources.rs`.** It is a two-line fixture builder.
- **`line_col` in `import/sources.rs`.** It is the test's own oracle for the line and column, rightly not the production
  function.
- **File sizes.** Every touched `.rs` file is under 600 lines (the largest is `import/rig.rs` at 515 and `assess.rs` at 411).
- **Frozen files and contracts.** No frozen file, manifest, protocol, golden or hub file is touched. ADR-0021 is updated in
  the same change.

## Notes for O

- Both warns are follow-ups. Neither needs F.
  - Finding 1 is a test-only edit of about five minutes, confined to `import/rig.rs` and the three test files that import
    from it.
  - Finding 2 needs the test kit's freeze lifted first.
- The PR's CI runs on the merge with `origin/main`, which now includes #644. The trial merge is clean, but only CI verifies
  the merged build.

## Patterns referenced

- `crates/holler-pane/src/reconcile.rs` (`observe_all`, `joined`, `ObservedHealth`, `failed_message`) and
  `reconcile/observe.rs` (`check_tmux`, `observe_server`, `lost`, the call-name constants): the source of the deliberate
  copies.
- `crates/holler-pane/src/findings.rs` (`FindingKind`, `quoted`, `embedded`) and `error.rs` (`PaneCode`,
  `RefusalCode::from_static`): the single sources of the codes.
- `crates/holler-cli/src/profile/{create,delete,list,show}.rs` and `pane/list.rs`: `insert_profile`, `create`'s undo,
  `single_quoted`, `pane_list`, `count`, `push_spec`, `optional_text`.
- `crates/holler-cli/tests/pane_verbs/{doctor/rig,launch/rig,launch/displaced,close/rig,park/failures,relaunch}.rs` and
  `profile_verbs/rig.rs`: the test helpers and seams compared in findings 1 and 2.
- The #644 diff (`7284001..b8242d7`, `profile_diff.rs`) and `git merge-tree` against `origin/main`.
