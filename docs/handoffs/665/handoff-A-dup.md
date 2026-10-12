# Handoff-A-dup: Phase 7 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)  (anti-duplication gate)

**Date:** 2026-10-11 (22:02 MDT)
**Branch:** `issue-665-implementation`
**Diff base:** `bda1069`   **Diff head:** `fa1677c`
**Reuse map:** no `survey.md`. "Reuse expectations for Phase 7" in `docs/handoffs/665/handoff-A.md` stands in for it.
**Verdict:** PASS

## Summary

**PASS. There are no block findings and three warns.** F extended each object the reuse expectations named, and built no parallel production path:

- **The hub transaction** is one `profile::store::Store::rename`, which serves both the port (`None`) and `profile/rename` (`Some(&PaneState)`). It is built from the existing write rules:
  - `next_generation`, `log_entry`, `ProfileEntry::next` and `entry::stamp` are reused;
  - `entry.rs` is unchanged;
  - `check_name`'s message is now the shared `taken`.
- **The twin stores were extended in place, under the same names.** In both stores, `entries_with`, `save` and `commit` now take a slice of changes, and `apply` and `after` were added. The pane-only additions are `Locked` and `Store::write`. Both are needed for the restore, which is a save of the committed table, not a byte copy.
- **The handler** is `panes::handlers::run` with a closure, like every other `profile/*` method.
- **The CLI verbs reuse existing pieces:** `insert_profile`, create's `taken`, `profile_from_panes`, `decode_params`, `GridPos::parse`, `write_atomic` and `show::text`. The shared `Document` lives in `export.rs` and is reached as `super::export::...`.
- **`drift.rs` is `profile show`'s comparison, literally:** `diff_profile` over `is_member` members, plus the existing finding helpers.
- **The fake's rename** is one `feed.write` built from the fake's own helpers.
- **D6 holds.** No port method, no `Ports` field and no `bind.rs` edit.

The three warns:

1. **A test helper was added without removing the copy it duplicates.** `pane_support::request` is a near-copy of `profile_handlers_test.rs`'s private `line`.
2. **`import` and `export` skip the shared count helper.** They print `1 specs` because they do not use `super::list::count`, which every sibling profile verb uses.
3. **The `Locked` handle's write methods accept any `PaneEvent`s,** not only the moves that `Locked::moves` built.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/tests/pane_support/mod.rs:205-231`; `crates/holler-hub/tests/profile_handlers_test.rs:30,67-88` | **The new shared request helper sits next to the copy it was taken from.** `pane_support::request` (#665, T) is `profile_handlers_test.rs`'s private `line` line for line. Only the correlation id differs (`h-request-1` against `h-profile-1`). `REPLY_WITHIN` is now declared in `pane_support` and again privately in `profile_handlers_test.rs:30` and `pane_handlers_test.rs:24`. `pane_handlers_test.rs:53-71` is a pane-only variant of the same function. The new copy is in the right place: the shared module, which A's warn 6 asked for, and both rename targets use it. The hub tests already kept one copy of this helper per file before #665, so there was no dominant pattern to break, and this is not a block. The two per-file `Rig`s in `profile_rename_test.rs` and `profile_rename_faults_test.rs` follow the hub tests' convention of a `Rig` per file, so they need no action. | Delete `line` and `REPLY_WITHIN` from `profile_handlers_test.rs` and call `pane_support::request`. The file is already in the Blast radius and already includes `mod pane_support;`. Optionally, fold `pane_handlers_test.rs`'s `call` and its `REPLY_WITHIN` the same way. This is test-only, and no assertion changes. |
| 2 | warn | `crates/holler-cli/src/profile/import.rs:116`, `crates/holler-cli/src/profile/export.rs:272` | **The new verbs print the spec count without the shared count helper.** They write `"{} specs"` from a raw number. Every sibling profile verb pluralizes through `super::list::count` instead (`create.rs:339`, `show.rs:126`, `list.rs:82`, `delete.rs:163`, `apply.rs:146`). As a result, `profile import` and `profile export` print `1 specs`, while `profile create` and `profile show` print `1 spec` for the same profile. The tests pin the inconsistency at `tests/profile_verbs/import.rs:80,159` and `tests/profile_verbs/export.rs:318`. F's handoff says this follows T's pinned strings. The brief's `<N> specs` is the same template `create`'s doc comment uses, and `create` renders it through `count()`. | Render with `count(n, "spec")` from `super::list` in both `render` functions, and have T update the three pinned strings. Or, if `1 specs` is intended, record that decision in `decisions.md` so S does not read it as drift. |
| 3 | warn (low) | `crates/holler-hub/src/panes/store.rs:375-457`; `crates/holler-hub/src/panes/mod.rs:71,140` | **The `Locked` handle can write any pane events, not just a rename's moves.** `Locked` is crate-visible through `PaneState::locked`. Its `save(&[PaneEvent])` and `apply(Vec<PaneEvent>)` accept any events, so a future crate caller could write records that skip `cas_put`'s compare-and-swap and `refuse_profile_move`. The module docs exempt only a rename's moves from those rules ("The pane half of a rename"). Today there is one caller, `profile::store::Store::rename`, and it passes exactly what `Locked::moves` built, so the code is correct as it stands. This is about how tight the API is, not about duplication. | Make `moves` return an opaque `Moves` newtype, which only `Locked::moves` can construct, and have `save`, `restore` and `apply` take it. Another option is to narrow `Locked` to one `move_members` operation. It is optional and can be a follow-up. |

**No duplication in production code; the extension is clean.** Every reuse expectation from Phase 3 is met:

| # | Expectation | Where it holds |
|---|---|---|
| 1 | Membership through `is_member` | `panes/store.rs:402` (the moves) and `reconcile/drift.rs:51`. The exact `== Some(to)` at `:402` checks "already names NEW", not membership: a rename that changes only the spelling must still respell its members, and H5 pins that. |
| 2 | The hub's write rules | `profile/store.rs`: `next_generation` :517, `entry::stamp` folded over OLD's and NEW's entries :519, `log_entry` and `ProfileEntry::next` :536-537 and :549-550. `entry.rs` is unchanged. |
| 3 | The twins extended in place | `entries_with` (:118 / `panes` :109), `commit` (:327 / :238), `apply` (:335 / :245), `save` (:347 / :256) and `after` (:133 / :119). The restore is `Store::write` of the committed table, and a failed restore is logged through `log_fault` (`panes/store.rs:433-446`). |
| 4 | The handler pattern | `profile/rename.rs:24-42`: `panes::handlers::run` over `ProfileRenameParams`. |
| 5 | One `Store::rename` | `profile/store.rs:290-323`. The port passes `None` (`profile/mod.rs:160-169`), and the handler passes `Some(&panes)`. |
| 6 | The fake's rename | `profile_store.rs:239-291`: one `feed.write`, using the fake's own `entry`, `record` and `next_generation`. `taken` is shared with `check_name`. |
| 7 | The conformance helpers | `conformance/profile_store/rename.rs:18-25` imports the suite's helpers. `open` and `last_cursor` were widened to `pub(super)` in `watch.rs`, not copied. |
| 8 | The CLI reuse | `import.rs:30-31,78,84,86`, `export.rs:140,167,254,275`, `rename.rs:22,69`, and `emit` in every `run`. `profile/mod.rs` changes by one doc line. |
| 9 | Drift reuse | `drift.rs:41,51,57,65,76,80`. `reconcile.rs` gets one `mod drift;` (:47), one call (:203) and one doc sentence (:18-20). |
| 10 | The doctor rig | `doctor/rig.rs:440`: `profile_from_panes` over the seeds' records. |
| 11 | D6 | `wiring/bind.rs`, `ports.rs`, `lib.rs`, `cli.rs` and `docs/protocol/v2.md` are untouched (`git diff --stat` is empty for them). |

**Checked, and not a finding:**
- **The fake's `move_record` repeats the hub's checks,** and `taken` is spelled in both crates. The fake is deliberately independent of `holler-hub`, and the two `check_name`s already matched before #665.
- **Each new verb builds `profile-not-found` inline,** which is what `show`, `delete` and `create` already do.
- **The per-file `name` and `stored` test helpers** follow `create.rs` and `delete.rs`.
- **`block_save`** is a new three-line local helper. No shared helper for it exists, and a directory works where a permission test would not (as root).
- **No touched file is near the 900-line lint failure.** The largest are `profile/store.rs` (562) and `panes/store.rs` (516).
- **No personal infrastructure names** appear in the diff, the docs, the tests or the commit metadata.

## Notes for F

None is required, since the verdict is PASS.
- **Warn 1** is T's: a test-only fold.
- **Warn 2** needs one decision. Either F switches to `count()` and T updates the three strings, or the literal form is recorded as intended.
- **Warn 3** is optional and can be a follow-up.
