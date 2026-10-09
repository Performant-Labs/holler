# Handoff-A-dup: Phase 7 - #639 the hub pane registry  (anti-duplication gate, re-review after rework)

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Diff base:** 55dba00 (merge base with `origin/main`, after merge 1d67ef3)   **Diff head:** 9289d30
**This cycle's delta:** 2188fe7..9289d30. The earlier pass reviewed `af3d8df...2188fe7` (31d2102); see "Earlier pass" below.
**Reuse map:** docs/handoffs/639-brief.md §Files "Reuse map"
**Verdict:** PASS

## Summary

PASS. This cycle added no production object and no parallel path. Its whole story delta is:

- doc comments in the four `panes/` files: `store.rs` (F's rework: the `Condvar` wait releases the lock, and rule 1 is
  re-checked on every poll), plus `mod.rs`, `persist.rs` and `feed.rs` (O's refresh after ADR-0021 merged);
- two handler tests in `pane_handlers_test.rs`, which reuse the file's existing `Rig`, `call`, `cas_put_params` and
  `sample_pane`;
- the ADR-0021 amendment (§6 and "Decisions taken" item 7), the brief's AC 25 and radius, and the `CHANGELOG.md` entry
  carried through the merge.

Every object the Reuse map named is still the only implementation. F rejected the outside gate's round-1 remediation (a
second lock, or `tokio::sync::Notify`), and that rejection kept the architecture intact: a second lock would have been the
drift, because D1 and ADR-0021 §7 ("One lock per registry") both require one lock.

My earlier W-1 is resolved: the code and ADR-0021 now agree on the idle cursor, in the same change, and a test pins the
edge case. Three warns remain, and none blocks:

- the rest of the earlier W-2 (#661's reuse seams);
- a new, low one: a `JoinError`'s text reaches the client;
- the earlier W-3 (test-helper near-copies), unchanged.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/src/panes/mod.rs:19-22`; `persist.rs:17-21`, `71-77`, `165-166`; `store.rs:261`, `287-319`; `handlers.rs:37-40`; `crates/holler-hub/src/profile/mod.rs:61-63` | **The rest of the earlier W-2: #661's reuse seams.** The refreshed docs now record the one edit that ADR-0021 "Decisions taken" item 2 (lines 501-504) sanctions: `pane-in-other-profile` inside `Store::cas_put`. That resolves part (b). They still read as if it is #661's only edit in `panes/` ("also adds one comparison here"), and three seams are open. **(a)** ADR §7 (lines 244-246) keeps the profile change log in `profiles.json`. `Doc<E>` is `{version, cursor, entries}` with `deny_unknown_fields`, and `check` refuses a name filed twice, so the log fits neither as a new member nor as entries: #661 must extend `Doc`. **(c)** `Store::poll` (the `Condvar` wait loop) and the `Feed` iterator are typed on `PaneEvent` and the pane `Store`, and `ProfileStore::watch` (`holler-pane/src/profile.rs:356`) needs both. **(d)** `NoParams` is private. Outside this radius, #669's doc on `check_membership` (`profile/mod.rs:61-63`) still says #661 fills it with `pane-in-other-profile`, which item 2 moved into the pane CAS. That stale sentence explains the outside gate's round-2 W-1. No parallel path exists in #639 today. | O: #661's brief puts `panes/{persist,store,handlers}.rs` in its radius. It extends `Doc` for the log, lifts `poll` and `Feed` to a generic form instead of copying them (a copy is a Phase-7 rejection for #661), makes `NoParams` `pub(crate)`, and corrects `profile/mod.rs:61-63`. Optional here: one clause in `mod.rs:19-22` that names the `poll`/`Feed` lift. |
| 2 | warn (low) | `crates/holler-hub/src/panes/handlers.rs:81-83` | **A `JoinError`'s text reaches the client.** `answer` builds `what: format!("the registry task did not finish ({join})")`. In the locked tokio (1.53.1, `src/runtime/task/error.rs:135-151`), a panicked task displays as `task {id} panicked with message {payload:?}`, so a panic payload would reach the control-socket client. The hub's precedent at the same boundary discards the `JoinError` for a fixed message (`token.rs:664-667`, "token store task panicked"). The module's own rule (`persist.rs:45-48`) is that a `what` is built only from safe parts. **It cannot happen today:** the hub denies `unwrap_used`, `expect_used` and `panic` (`Cargo.toml:19-22`), `panes/` has no indexing, and `check_membership` is `Ok(())`. The control server logs only the method and the id (`control_server.rs:61-66`), so no log line carries the text. #661's `check_membership` will run inside this closure. The outside gate's round-2 W-3 raised this, but hedged it on a tokio feature flag. The payload is rendered whenever it is a string. | Use a fixed message, as `token.rs` does. If wanted, add "panicked" or "was cancelled" from `join.is_panic()`, without the payload. This is one line, in any later edit of `handlers.rs`, #661's included. |
| 3 | warn | `crates/holler-hub/tests/pane_handlers_test.rs:39-45`; `pane_dispatch_test.rs:104-111`, `325-328`; `pane_support/mod.rs:6-7`, `63-66` | **The earlier W-3, unchanged: small test-helper near-copies.** `Rig::on` and `fresh_deps` both rebuild `(PaneState::load_with(.., short_opts()), ProfileState::load(..))` instead of using `pane_support::load`. `check_membership_accepts_any_pane` builds its own temp dir and `HubState`, beside the imported `temp_state()`. The `pane_support` doc names three includers, but `pane_feed_test.rs:10` is a fourth. This cycle's two tests reuse `Rig`, so they add no new copy. | Add `pane_support::deps(&HubState) -> PaneDeps` with the short window, and use it in `fresh_deps` and `Rig`. Use `temp_state()` in `check_membership_accepts_any_pane`, and fix the doc line. T can fold this into any later edit. |

Apart from these, there is no duplication and the extension is clean. Rework introduced no drift.

### Earlier pass (`af3d8df...2188fe7`, PASS with three warns): what became of each

- **W-1 (the idle cursor against the merged ADR-0021 §6): resolved.**
  - §6 (lines 218-223) and "Decisions taken" item 7 (line 513) were amended in this change (639f130). They are the ADR's
    only two hunks, both inside the widened radius.
  - `Store::poll` answers the head (`store.rs:273-276`), and the ADR now says the same.
  - `an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head` (`pane_handlers_test.rs:307-359`) pins the
    one case where the head and `since` differ.
  - "draft" is gone from `feed.rs:37-38`; ADR-0021 is `accepted` (line 3).
- **W-2 (#661's reuse seams): part (b) resolved.** The rest is carried forward as finding 1.
- **W-3 (test-helper near-copies): unchanged.** It is carried forward as finding 3.

### Checked and clean (this cycle)

- **F's rework (476ad22) is doc comments only, and they are accurate.**
  - `wait_timeout` takes the guard and hands it back (`store.rs:278-282`).
  - The head is assigned only at load (`store.rs:69`) and in `commit` (`store.rs:206`), always to `head + 1`
    (`store.rs:86-94`). "A cursor that passed rule 1 never fails it later" therefore holds.
  - `handlers::watch` passes `params.since` straight to `poll` (`handlers.rs:126-131`), so `select`'s rule-1 check is the
    only one on that path.
- **T's new test (9289d30) is not a duplicate.** `pane_watch_with_a_cursor_ahead_of_the_head_is_usage`
  (`pane_handlers_test.rs:361-386`) pins rule 1 on the `poll` path. `a_cursor_ahead_of_the_store_is_usage` pins the
  `Store::watch` path, which refuses the cursor before `poll` runs.
- **The Reuse map, row by row:**
  - `write_atomic`: `persist.rs:156`.
  - `next_generation`: `store.rs:172` and `store.rs:194`. Nothing else in `crates/holler-hub/src/` compares generations.
  - `decode_params`: `handlers.rs:50`.
  - `reply_line`: `handlers.rs:57` and `mod.rs:202`.
  - The poison-tolerant lock: `store.rs:135` and `store.rs:281`.
  - The `holds.rs` event shape: `store.rs:337-352`.
- **No second feed.** No `Condvar`, long-poll or watch window exists anywhere else in `crates/*/src/`. The CLI surface
  merged from `main` (#670) adds no `WATCH_WAIT` copy, no reply parse-back and no watch logic.
- **Radius.** `git diff --name-only 55dba00...HEAD` lists only radius paths: `panes/**`, the five pane test files,
  ADR-0021, `CHANGELOG.md` and `docs/handoffs/639*`. `git diff --quiet` confirms these are unchanged: `serve.rs`,
  `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `profile/**`, `holler-pane`, `holler-proto`, `holler-cli`, every
  `Cargo.toml`, `Cargo.lock` and `docs/protocol/v2.md`.
- **Merge.** `git merge-tree` against the current `origin/main` (939d79c, two research-doc commits ahead) is clean.
- **Size.** The largest file is 523 lines (`pane_registry_test.rs`). The largest production file is 352 (`store.rs`).
- **Public repository.**
  - All 17 branch commits carry the GitHub no-reply address as both author and committer.
  - This cycle's delta names no private host or person.
  - Every `#[allow]` in the diff carries `// #NNN`.

## Notes for O

- **Route finding 1 into #661's brief.** Item 2 already widens #661's radius into `store.rs`. The brief should also name
  `persist.rs` (the log in `Doc`), plus `store.rs` and `handlers.rs` (the generic `poll`/`Feed` lift and `NoParams`).
  The profile registry then extends these objects rather than copying them.
- **Finding 2 is a one-line change.** Leaving it to #661, which edits `handlers.rs` anyway, is reasonable.
- **The outside gate's round 2 (PASS).** Its NV-3 (a spurious wake-up could end the window early) does not hold.
  `poll` re-runs `select` and recomputes the time left after every wake (`store.rs:265-283`), so a spurious wake only
  loops.

## Patterns referenced

- `crates/holler-hub/src/token.rs:664-667` (the `JoinError` at the blocking-pool boundary); `crates/holler-hub/src/holds.rs`
  (persistence, the lock idiom, `emit`)
- `crates/holler-hub/src/{pane_dispatch.rs, profile/mod.rs, control_server.rs:44-66}`;
  `crates/holler-pane/src/profile.rs:295-359` (`ProfileLogEntry`, `ProfileEvent`, `ProfileStore`)
- `docs/adr/ADR-0021.md`: §6 (lines 218-223), §7 (lines 237-262), and "Decisions taken" items 2 and 7 (lines 501-504
  and 513)
- tokio 1.53.1, `src/runtime/task/error.rs:135-151` (the `Display` of `JoinError`)
