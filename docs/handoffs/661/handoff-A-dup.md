# Handoff-A-dup: Phase 7 - #661 the hub profile registry  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-661-implementation
**Diff base:** 9d61c9f (origin/main, the merge base)   **Diff head:** 9026762
**Reuse map:** docs/handoffs/661-brief.md, "Reuse map (extend, do not duplicate)"
**Verdict:** PASS

## Summary

PASS. F extended the #639 pane registry through the seam #639 documented for #661, and copied none of its shared parts. The profile code calls these directly:

- `persist::{Doc, VERSION, load_doc, save_doc}` and `Problem::what`
- `feed::{Ring, select, check_since}`
- `handlers::{run, NoParams}`
- `PaneStoreOptions` and `WATCH_WAIT`
- `log_fault`, `next_generation`, `reply_line` and `now_millis`

A grep sweep of `profile/` finds no second loader, writer, select, ring, CAS helper, params decoder or reply builder.

The one new object, `profile::store::Store`, is the brief's written justification ("New object, justified"; D10). I diffed it against `panes::store` with the type names normalised. Each mirrored item differs from its pane twin only in three ways: the key (slug, not pane name), the entry type, and `poll`'s `&entry.event` projection. So the mirror hides no behavioural drift.

The #639 edits stay within the plan: D7's one comparison, two `pub(crate)` openings, and doc lines.

All four findings are `warn`. The one that needs action before merge is finding 2: the follow-up issue that makes the two stores one generic store still does not exist, and this run has no O agent to open it.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/src/profile/store.rs:79` | **`Table::from_doc` is a mirror my Phase 3 list missed.** It mirrors `panes/store.rs:67`. My Phase 3 finding 1 said a mirrored item outside its list is a block. I rule this one inside the accepted mirror. It is the half of `load_table` (which is on the list) that builds the table, and the pane `load_table` calls its own `from_doc` the same way (`panes/store.rs:335`). My Phase 3 line range for `load_table` (base `:323-331`) simply left out the item it calls. It differs from its twin only in the key (`entry.slug`, not `entry.name`), and F disclosed it in handoff-F. It is none of the brief's block categories: a second loader, writer, select, ring, CAS helper, params decoder, reply builder, or a copy of a `pane_support` helper. | No change in this story. Add it to the follow-up in finding 2. |
| 2 | warn | `crates/holler-hub/src/profile/store.rs:76-388` | **The mirror is about 160 lines, and the follow-up issue still does not exist.** The accepted mirror is now 13 items: `Table::{from_doc, next_cursor, entries_with}`, `Store`, `open`, `lock`, `read`, `commit`, `save`, `watch`, `poll`, `Feed` with its `Iterator`, and `load_table`. The `WRITE_EFFECT` text is also duplicated (`profile/store.rs:64`, `panes/store.rs:54`). The brief said about 60 lines; my Phase 3 estimate was about 120. The size is still justified: making #639's store generic would exceed the issue's one-comparison limit on `panes/**`. But Phase 3's action is still open: no follow-up issue exists, and the "Twin of" lines carry no issue number. `panes/persist.rs:17-21` also still says `profiles.json` holds `ProfileEvent` entries; they are `ProfileEntry`. A BLOCK here would send F to fold the twin, which would go past that same limit. So this stays a warn, owned by the run's merging agent or the operator. | Before merge, the run's merging agent or the operator opens the follow-up issue and links it from the PR description. Its scope: (1) move `RegistryEntry`, `persist`, `feed`, `run`/`NoParams`, `log_fault` and the options into a neutral `crate::registry` module; (2) make the store generic over the key, the file entry and the entry-to-event projection (the only differences the normalised diff shows); (3) collapse the 13 twins and `WRITE_EFFECT`; (4) fix `panes/persist.rs:17-21`; (5) put its own number on the "Twin of" lines. |
| 3 | warn | `crates/holler-hub/tests/profile_registry_test.rs:30,34`; `profile_persistence_test.rs:23,27`; `profile_feed_test.rs:20` | **Two small test helpers are copied into each file.** `pname` (`ProfileName::parse(..).unwrap()`) appears three times and `who` (`Actor::parse(..).unwrap()`) twice. They are the profile twins of `pane_support::name`, and they sit beside `pane_support::actor`, which the same files import. `pane_membership_test.rs` also writes `ProfileName::parse(..).unwrap()` inline six times. Not a block: no existing `pane_support` helper is copied, and each body is one line. | In the follow-up, or the next change to these files: add `profile_name(text)` beside `pane_support::name` and an actor-by-name helper beside `actor`, then delete the local copies. |
| 4 | warn | `crates/holler-hub/tests/profile_handlers_test.rs:33-104` | **The handler test's rig near-copies the pane handler test's rig.** `Rig`, the free `line`, `put_params`, `pane_put_params` and `as_profile` near-copy `pane_handlers_test.rs:26-79` (`Rig`, `call`, `cas_put_params`, `as_pane`): about 45 lines. `pane_put_params` has the same body as `cas_put_params`. Not a block, for three reasons. A local rig per binary is #639's convention (`pane_handlers_test.rs::Rig`, `pane_dispatch_test.rs::Conn`/`fresh_deps`). The Reuse map named only `pane_support` for test helpers. D11 limits `pane_handlers_test.rs` to AC 43. The profile `Rig` is a superset of the pane one: it routes both method prefixes and can return the raw line. | In the follow-up: move the profile `Rig` (two registries, prefix-routed `line`/`call`) into `pane_support`, and delete the copy in `pane_handlers_test.rs`. |

No other duplication; the extension is clean. What I checked:

- **Shared parts are reused, not copied.**
  - `profile/` imports each shared item directly (`profile/store.rs:45-54`, `profile/handlers.rs:26`, `profile/entry.rs:51`, `profile/mod.rs:84-85`).
  - `handlers.rs` is six closures over `panes::handlers::run`. It has no `params_of`, `to_data` or `answer` of its own.
  - All logging goes through `log_fault`; there is no `log::emit` in `profile/`.
  - The only env guard is `EnvVarName`'s decode inside `decode_params`.
- **`entry::check` repeats none of `persist`'s checks.** It adds only the profile rules: the event's slug, a non-empty log, the live name, and the last log entry. `persist::check` and `check_entry` still enforce, through `RegistryEntry for ProfileEntry`: unique slugs, unique cursors, the cursor range, the record's slug equal to the entry's slug, and a generation of at least 1.
- **The #639 edits are the planned ones.**
  - `refuse_profile_move` is the D7 comparison. It runs after `next_generation` and under the pane lock, and its name differs from `profile::check_membership` (A finding 6).
  - `NoParams` is now `pub(crate)`.
  - `log_fault` is `pub(crate)` through `pub(crate) use store::log_fault;`, and `mod store` stays private (A finding 2).
  - The doc lines are fixed (A finding 7).
  - No other #639 logic changed. No #639 test assertion changed beyond AC 42 and AC 43.
- **`ProfileState`, `load_with`, the trait impl and `dispatch` follow `PaneState`'s shape.** That is a second port with the same structure, as the brief's Files list specifies: pattern consistency, not duplication.
- **The hub's rules and the testkit fake's are twins by design.**
  - The pairs: `refuse_profile_move` and the fake's `check_membership` (testkit `pane_store.rs:223`); `check_name` and the fake's `check_name` (`profile_store.rs:330`); `log_entry` and the fake's `entry` (`profile_store.rs:353`).
  - The testkit is a dev-dependency, so production code cannot call it.
  - #637 left these rules out of the port on purpose ("the rules the port leaves open, which every profile registry (#661) keeps too"). Each implementation keeps its own copy, and one conformance suite checks both.
  - `holler-pane` is outside AC 45. If the project ever wants a single copy, its home is `holler-pane` beside `next_generation`. That is a decision about the port, not about this story.
- **`pane_support` was extended, not copied (A finding 3).**
  - `drain` is generic, and `head_by` has `head` and `profile_head` as one-line wrappers. There is no `drain_profiles`.
  - #639's `pane_registry_test.rs` and `pane_feed_test.rs` are untouched.
  - The profile feed and persistence tests keep per-file projection helpers (`summary`, `entry`, `history_since_a_cursor`, `assert_collapsed`, `corrupt_cases`, `assert_every_method_is_store_corrupt`). These twin their pane test files' local helpers, which is #639's convention.
- **No near-copy of the stack's named candidates:** the `token.rs` operations, `Lockout`, `Roster`, the `log(Severity, ...)` helper, `Hub`, `Body`, `mint_token`, `join`, `wait_for` and `StateDir`.
- **Sizes and changelog.**
  - The largest production file is `profile/store.rs` at 415 lines. The largest test file is `profile_handlers_test.rs` at 577. No touched file is near 800.
  - `CHANGELOG.md` has one #661 entry.

## Notes for F

None: the verdict is PASS, and findings 1-4 need nothing from F.

Two Phase 3 items are still open, and the run's merging agent or the operator must act on them before merge:
- Phase 3 finding 1, carried forward as finding 2 above: open the follow-up issue.
- Phase 3 finding 8: post the cross-story notes on #647, #662, #663 and #664.

The operator-review flags on D4, D9, D10 and D12 still stand.
