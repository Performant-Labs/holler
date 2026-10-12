# Handoff-A-dup (re-run after the rework): Phase 7 - `holler profile rename`, `export`, `import` and `profile-drift` (#665)  (anti-duplication gate)

**Date:** 2026-10-11 (23:32 MDT)
**Branch:** `issue-665-implementation`
**Diff base:** `fa1677c` (the head the last pass reviewed; the whole story is `bda1069..c63908e`)   **Diff head:** `c63908e`
**Reuse map:** no `survey.md`. "Reuse expectations for Phase 7" in `docs/handoffs/665/handoff-A.md` stands in for it.
**Verdict:** PASS

The earlier pass (22:02 MDT, on `fa1677c`) follows this section, unchanged.

## Summary

**PASS. There are no block findings and three warns: one new and two carried over.** The rework cycle covered F rework 1 and 2, T-green rework 1 and 2, and T's test-only round. It changed 10 code and docs files. Every change extends an object that already existed, and the cycle built no parallel path:

- **The spec count now goes through `super::list::count`**, the profile verbs' shared helper, in both `render` functions (`export.rs:41,302` and `import.rs:37,126`). This closes the last pass's warn 2.
- **The redaction extends `spec_refused`.** That is the one place that already mapped a refused spec, and it gains two arms (`export.rs:193-209`). `Document::specs` now sends the position step through it too (`:147-149`), so there is one redaction path, not two. The member's path comes from `SpecField::Grid.as_str()`, the same path `profile show` and the drift finding use, not from a string literal.
- **The tests reuse existing helpers:**
  - `BARE_DOCUMENT` was made `pub(crate)`, not copied (`tests/profile_verbs/export.rs:140`).
  - `assert_message_contains`, `assert_never_printed`, `refused`, `with_spec`, `write_document` and `stored` all existed before this cycle.
  - Conformance case 23 reads the log through the suite's own `entries` (`conformance/profile_store.rs:499`).
  - H1 waits on `holler_proto::clock::now_millis`, the clock the hub stamps with (`profile/store.rs:62,518`).
- **Nothing else moved.**
  - `profile/store.rs` changed only by a doc rewrap.
  - `panes/store.rs` (with `Locked` and the twins), `drift.rs`, the fake, the ADRs and `grid.rs` are byte-identical to the last pass.
  - D6 still holds: `git diff --stat bda1069..HEAD` is empty for `bind.rs`, `ports.rs`, `lib.rs`, `cli.rs`, `docs/protocol/v2.md` and `docs/orchestrating.md`.
- **Phase 3's reuse expectation 8 still holds in the two rewritten verbs:**
  - `insert_profile` (`import.rs:93`), `taken` (`:87`) and `profile_from_panes` (`:89-92`);
  - `decode_params` (`export.rs:153`), `GridPos::parse` (`:180`), `write_atomic` (`:279`) and `show::text` (`:301`);
  - `emit` in both `run` functions.

The new warn is that **`GRID_FORMS` and `GRID_BOUNDS` copy holler-pane's private `FORMS` and `BOUNDS`, and no test ties the copies to the originals.**

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/src/profile/export.rs:54-60`, used at `:197-202`; the originals are at `crates/holler-pane/src/grid.rs:23-27` | **The import's grid hint copies holler-pane's text, and nothing pins the two together.** `GRID_FORMS` and `GRID_BOUNDS` match `grid.rs`'s private `FORMS` and `BOUNDS` character for character.<br><br>**Why this is not a block:**<ul><li>It is message text, not a parallel path. Positions still parse through `GridPos::parse` (expectation 8), and the codes are still holler-pane's.</li><li>The copy is deliberate and recorded, in `decisions.md` (F rework 1) and in the doc comment at `:54-56`. S's REWORK 2(b) asked for "the accepted forms or bounds" in the message, and making the originals public would edit `grid.rs`, which is outside the Blast radius.</li><li>Cutting the hint out of `GridPos`'s own message is not safe: the `pos`-label message has no hint, and an excerpt can contain `" ("`.</li><li>The CLI already restates the forms in user-facing text (`pane/args.rs:48`, the `--grid` help).</li></ul>**The risk:** `grid.rs` says the grid type is "defined here once". If holler-pane changes the accepted forms or the bounds, `holler profile import` keeps printing the old hint and every test stays green. `a_refused_position_is_named_by_its_spec_number_not_quoted` (`tests/profile_verbs/import/refusals.rs:107-122`) checks only for `herdr.grid of spec 2`. | **Preferred** (a follow-up, or at the rebase if the main session admits `grid.rs` to the Blast radius):<ul><li>make the two constants public in holler-pane, for example as `GridPos::FORMS` and `GridPos::BOUNDS`, with `parse`, `from_cells` and the deserializer using them;</li><li>then delete the CLI's two copies.</li></ul>**In scope now** (test-only, for T): pin the copy. For a text position and for `r0c1`, assert that the hint in the import refusal's message equals the hint in `GridPos::parse`'s message for the same input, taking the text after the last `" ("` in each. The CLI tests already import `GridPos`. |
| 2 | warn (carried over, test-only) | `crates/holler-hub/tests/pane_support/mod.rs:205-231`; `crates/holler-hub/tests/profile_handlers_test.rs:30,57-88`; `crates/holler-hub/tests/pane_handlers_test.rs:24,48-71` | **The last pass's warn 1 is unchanged.** `pane_support::request` is still a near-copy of `profile_handlers_test.rs`'s private `line`, and `REPLY_WITHIN` is still declared three times. The rework did not touch these files, and S's advisory 6 lists this as optional. | As before: delete `line` and `REPLY_WITHIN` from `profile_handlers_test.rs` and call `pane_support::request`. Optionally, fold `pane_handlers_test.rs`'s `call` in the same way. |
| 3 | warn (carried over, low) | `crates/holler-hub/src/panes/store.rs:375-457`; `crates/holler-hub/src/panes/mod.rs:71,140` | **The last pass's warn 3 is unchanged.** `Locked`'s `save` and `apply` accept any `PaneEvent`s, not only the moves that `Locked::moves` built. The one caller passes exactly the moves, so the code is correct today. F's handoff (rework 1, "Known issues") leaves it as a follow-up. | Optional follow-up: an opaque `Moves` newtype that only `Locked::moves` can construct, taken by `save`, `restore` and `apply`. |

**No duplication in production code; the extension is clean.**

**Checked, and not a finding:**
- **`spec_refused`'s `other => other` arm** existed before this cycle and was reviewed in the last pass. It matches its neighbours (`profile/create.rs:217`, `profile/delete.rs:183`). F records, as a known issue, that it relies on holler-pane's guards keeping their no-echo rule. If the operator wants the redaction enforced by the compiler, `pane/profile_scope.rs::with_context` (`:274-309`) is the repository's precedent for an exhaustive match with no catch-all. This is not drift.
- **H1's wait** (`crates/holler-hub/tests/profile_rename_test.rs:138-142`) is a condition wait on the hub's own clock. There is no holler-hub test helper it could reuse: a grep for `fn wait*` and `fn until*` under `crates/holler-hub/tests` finds nothing. holler-cli's `wait_for` lives in another crate's test support. So the wait is not a near-copy.
- **The new import test** (`tests/profile_verbs/import.rs:184-206`) does not duplicate `export_leaves_out_absent_optional_members` (`tests/profile_verbs/export.rs:229-233`).
  - The export test starts from a spec in memory. The import test starts from a file that leaves out `env` and `expect`.
  - Both use the one `BARE_DOCUMENT`.
- **The ADRs stay true.** The rework changes refusal messages only. ADR-0021's "Profile files as built" (`:301-312`) and its section 9 rows name codes, not messages. "Every message ... never echoes a secret" (`:776`) is what the rework enforces.
- **No touched file is near the 900-line lint failure.** The largest are `profile/store.rs` (562 lines), `profile_rename_test.rs` (478) and `tests/profile_verbs/export.rs` (364). `src/profile/export.rs` is 306.
- **No personal infrastructure names** appear in the added lines or in the commit metadata. The only matches are the rule sentences in the brief and in S's handoff.

## Notes for F

None is required, since the verdict is PASS.
- **Warn 1:** the in-scope half is a test, so it belongs to T. The preferred half is a follow-up for the main session.
- **Warns 2 and 3** remain optional, as S's advisory 6 says.

---

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
