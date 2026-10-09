# Handoff-S: Phase 10 - #639 the hub pane registry (spec audit, second pass)

**Date:** 2026-10-09, 11:06 MDT
**Branch:** issue-639-implementation at 83ed8fe (merge base 55dba00; `origin/main` is at 939d79c and `git merge-tree` is clean)
**Issue:** #639 (epic #633)
**Brief:** `docs/handoffs/639-brief.md`. O amended AC 25 and the radius in 639f130, after the operator's ruling.
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-F-rework.md`, `handoff-T-green.md`
(all three passes), `handoff-A-dup.md` (the re-review at 9289d30), `decisions.md` and `evidence.md`. Also the outside diff
gate's two rounds: `639-diff-result-r1.md` (BLOCK) and `-r2.md` (PASS). Both ran on deepseek-v4-pro, the model that
`CLAUDE.md` on `main` names. Both files are gitignored.
**Diff audited:** all of `git diff origin/main...HEAD`: the five files under `crates/holler-hub/src/panes/`, the five pane
test files, `docs/adr/ADR-0021.md` and `CHANGELOG.md`.
**Also read:** issue #639 and epic #633 (rulings 1-9); ADR-0021 §6-§8 and "Decisions taken"; `holler-pane/src/ports.rs:27-82`,
`reply.rs:119-149` and `pane.rs:32-72`; `holler-proto/src/atomic_file.rs` and `vocab.rs:75-90`; `holler-hub/src/profile/mod.rs:55-70`.
**Verdict:** PASS

## A precondition

Met.

- `handoff-A.md`: **PASS**, with 10 warns and no blocks.
- `handoff-A-dup.md`, re-reviewed after the rework at 9289d30: **PASS**, with 3 warns and no blocks.

## T precondition

Met.

- **RED** (`handoff-T-red.md`). Against the real tree, the build fails only on the items this story creates. Against a stub
  that has only that API, every new and amended test fails on an assertion.
- **GREEN, last recorded at 9289d30.**
  - The four pane binaries pass 10/7/8/14, and the workspace passes 1069 with 0 failed.
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test` and `wire_selftest` are clean.
  - 15 repeat runs had no failure.
  - Since 9289d30 only `decisions.md` and `handoff-A-dup.md` have changed, so that run covers the code at HEAD.
- **The two tests added after RED.** Each pins behaviour that already existed, so each has mutation evidence in place of a
  RED:
  - `an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head` (4a75b54) fails when an idle window answers
    `since`.
  - `pane_watch_with_a_cursor_ahead_of_the_head_is_usage` (9289d30) fails under F's M2.
  - I confirmed both by reading. Each test asserts a value its mutation changes, and no other test asserts that value.
- No blocking issues.

## Acceptance criteria

R is `pane_registry_test.rs`, Fd is `pane_feed_test.rs`, and H is `pane_handlers_test.rs`. Every named test exists (I
checked each name with grep).

| AC | Criterion (short) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | `cas_put(p, 0)` creates at generation 1, and `get` returns it | R `cas_put_creates_at_generation_one_and_get_returns_it` | MET |
| 2 | A stale generation is a conflict; the record, the file bytes and the head are unchanged | R `cas_put_with_a_stale_generation_is_a_conflict_and_changes_nothing` (expected 0, 1 and 3 against generation 2) | MET |
| 3 | Stored verbatim except `generation`; no session inferred | R `cas_put_stores_the_record_verbatim_except_generation` (submitted 7 and 42 ignored; `session_of_record: None` beside `shown: Some`) | MET |
| 4 | Missing is checked before generation; stale is a conflict; a delete removes the record; recreating starts at 1 | R `delete_checks_missing_before_generation` | MET |
| 5 | `list` returns the live records, sorted | R `list_returns_every_live_record_sorted_by_name` (created out of order, one deleted) | MET |
| 6 | A restart keeps the fields, `profile`, the `r2c1` grid JSON and the deletion; the next cursor is head + 1 | R `records_survive_a_restart_unchanged` | MET |
| 7 | A v1 record without `profile` loads as `None` | R `a_record_written_before_the_profile_field_loads_with_profile_none` | MET |
| 8 | Mode 0600, the D2 shape, no temp file left, the hub dir created by the write | R `the_file_is_written_atomically_at_mode_0600`. Its filter matches the temp name in `atomic_file.rs:20` | MET |
| 9 | A corrupt file: every method is `store-corrupt` with the file named; bytes and listing unchanged | R `a_corrupt_file_fails_closed_and_is_never_rewritten`: the brief's five cases plus distinct cursors. Each structural case passes every check but its own, so each case is needed | MET |
| 10 | An unreadable file (0o000): the same as AC 9 | R `an_unreadable_file_fails_closed_and_is_never_rewritten` | MET |
| 11 | The reason never echoes file content | R `the_corrupt_reason_never_echoes_file_content`: the sentinel as a wrong-typed value, an unknown key and the version. The log is built from the same `Problem` (`store.rs:324-328`) | MET |
| 12 | Unwritable dir: `unavailable`, old state kept, head unchanged, the retry succeeds | R `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` | MET |
| 13 | 16 racing writers: exactly one wins, and the file equals memory | R `racing_writers_exactly_one_wins` | MET |
| 14 | 8x25 read-modify-writes lose no update; 200 consecutive put events | R `concurrent_read_modify_write_loses_no_update` | MET |
| 15 | Two `pane/cas_put` requests race on a multi-thread runtime | H `two_cas_put_requests_racing_exactly_one_wins` | MET |
| 16 | A watch from 0 yields the live records by last change, then later writes | Fd `watch_from_zero_yields_every_current_record_then_later_changes` | MET |
| 17 | Every write once, in order, then an idle `Ok(None)` | Fd `the_feed_delivers_every_write_exactly_once_in_order` | MET |
| 18 | Resuming neither repeats nor skips | Fd `resuming_from_the_last_cursor_neither_repeats_nor_skips` | MET |
| 19 | A waiting watch wakes on the next write | Fd `a_waiting_watch_wakes_on_the_next_write` (20 s window, 10 s `recv_timeout`) | MET |
| 20 | A cursor ahead of the head is `usage` | Fd `a_cursor_ahead_of_the_store_is_usage` (port path); H `pane_watch_with_a_cursor_ahead_of_the_head_is_usage` (wire path) | MET |
| 21 | After a restart, an old cursor gets each changed pane once, deletions included | Fd `after_a_restart_an_old_cursor_gets_each_changed_pane_once_including_deletions` | MET |
| 22 | A watcher behind the retained window gets the compacted changes | Fd `a_watcher_behind_the_retained_window_gets_the_compacted_changes` (`feed_retained: 2`) | MET |
| 23 | The round trip, the D7 data shapes, and a request with no `params` | H `get_list_cas_put_delete_round_trip`; H `a_request_without_params_works_for_list_and_watch` | MET |
| 24 | Bad params answer their own code | H `bad_params_answer_their_code` | MET |
| 25 | One batch and its cursor; an idle reply is `{"events": [], "cursor": <head>}`, which is `since` except for a watch from 0 over an all-deleted registry | H `pane_watch_answers_one_batch_and_its_cursor` (idle where `since` is the head); H `an_idle_watch_from_zero_over_an_all_deleted_registry_answers_the_head` (the edge: `cursor: 2` for `since: 0`, after which B's create and delete both arrive) | MET |
| 26 | A corrupt registry answers `store-corrupt` as a result, not a JSON-RPC error | H `a_corrupt_registry_answers_store_corrupt_on_every_pane_method` | MET |
| 27 | `check_membership` runs before the CAS; an `Err` is the reply and nothing is written | `grep -n "check_membership(" crates/holler-hub/src/panes/` matches `handlers.rs:111`, which is before `store.cas_put` at `:112` and inside the blocking closure. Its `?` returns before the store is called. No behavioural test is possible until #661 | MET (grep and reading, as the brief specifies) |
| 28 | #669's test amended (D8) | `pane_dispatch_test.rs`: the three assertions changed, `fresh_deps` uses `short_opts()`, and every `profile/*` assertion is unchanged | MET |
| 29 | Build guards | T's Tier 1 at 9289d30, and F's and T's `rustfmt --check`. My own checks are under "Quality audit" | MET |
| 30 | Only blast-radius paths changed | See "Scope check". `pane_feed_test.rs` is approved in `decisions.md` (O entry), and the ADR-0021 hunks are §6 and "Decisions taken" item 7 only | MET |

## Spec compliance

**Decisions already made (epic, issue, #669): all implemented.**

- The hub is the store only, with five verbs, `delete` included.
- Every reply is a `PaneReply` sent as a JSON-RPC result, and `pane/watch` is a long-poll.
- Persistence and the fail-closed load are inside `PaneState::load`, which stays infallible.
- `profile` persists, and `herdr.grid` is stored as `GridPos`.
- `PaneState` does not derive `Clone`.
- No new error code was added.

**Brief decisions D1-D9: implemented.**

- **D1.** One mutex and one condvar. A write runs in this order: CAS, then the next document, then the save, then the commit
  to memory and the notify (`store.rs:165-211`).
- **D2.** The file, its mode and its shape. The validation also checks for distinct cursors.
- **D3.** Fail closed; one `pane_registry_corrupt` event; a problem never contains serde's message.
- **D4.** A failed write is `unavailable` and changes nothing.
- **D5.** Cursors, the 1024-event ring, and the 4 s `WATCH_WAIT`.
- **D6.** Rules 1-4. The reply's cursor is the head.
- **D7.** The data shapes. They match the ADR-0021 §6 table.
- **D9.** `check_membership` is called with #669's frozen signature, outside the pane lock.

**Documented deviations.** Each was routed by A, recorded in `handoff-F.md` and `decisions.md`, and stays inside the radius.
I accept all six:

1. `dispatch` takes `&Arc<ProfileState>` (A W-1), as `pane_dispatch.rs:71-74` allows.
2. `poll` and the `Watch` iterator are in `store.rs`, and `feed.rs` is pure (A W-4).
3. The file entry is the frozen `PaneEvent`, and a tombstone is written as `"pane": null` (A W-5).
4. `Problem::what(label, path)` replaces `corrupt_what`.
5. Entry cursors must be distinct.
6. A refused save logs `pane_registry_write_failed`. D4 left this open.

**The first pass's conflict is resolved.** D6, AC 25, ADR-0021 §6 (with "Decisions taken" item 7) and `Store::poll`
(`store.rs:272-276`) now all say the head. The one case where the head differs from `since` is pinned by a test, and the
ADR keeps a record of the decision on `main` after the handoffs are removed.

**The issue's own acceptance bullets** map to AC 6-7 (profile across a restart), AC 2 (CAS conflict), AC 6 (restart),
AC 13 and AC 15 (racing writers), AC 9-11 (a corrupt file) and AC 17 (every write once).

## Quality audit

**Correctness and failure handling**

- **CAS.** Every write goes through `holler_pane::next_generation` (`store.rs:172`, `:194`), and nothing else in
  `crates/holler-hub/src/` compares generations. `delete` checks for a missing record before the generation.
- **The outside gate's round-1 B-1 is a false positive, confirmed independently.**
  - `poll` hands its guard to `Condvar::wait_timeout` (`store.rs:278-282`), which releases the mutex for the wait.
  - If the lock were held across the window, `a_waiting_watch_wakes_on_the_next_write` would fail, as F's M1 showed.
  - The round-1 W-2 remediation would have broken rule 1 on the wire, and T's new handler test now pins that.
- **Spurious wake-ups.** `poll` re-runs `select` and recomputes the time left on every pass (`store.rs:265-283`), so a
  spurious wake only loops. Round 2's NV-3 does not hold.
- **Feed invariants.**
  - Every event after `floor` is in the ring, through eviction, after a load, and with a ring of 0 (`feed.rs:57-74`).
  - Rule 2 takes precedence over rule 3, and the head only grows, so a cursor that passes rule 1 once never fails it later.
- **Fail closed.**
  - The failed state is the `Err` arm of the mutex's content, checked before any record lookup. On a corrupt file,
    `delete` therefore answers `store-corrupt` and not `pane-not-found`.
  - No write path can reach the file in that state.
  - An empty file, trailing garbage, a non-object document or a directory at the path all fail closed.
- **No content echo.**
  - A problem names only a serde category with its line and column, a version, a validated pane name (1-32 characters
    of the session-name grammar, `vocab.rs:81-90`), cursor numbers, and an io error. The io error is the `ErrorKind` on a
    read, as D3 says, and the `Display` on a write, as D4 says.
  - The custom errors `write_atomic` can return name only the path.
- **Concurrency.** Store code is reached only through `spawn_blocking`, and a `JoinError` maps to `unavailable`. The
  membership check runs before the pane lock is taken.

**Build guards**

- `panes/` has no `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `unsafe`, and no `#[allow]` (grep).
- All eight new test-file `#![allow]` lines carry `// #639`.
- Sizes: `store.rs` 352, `mod.rs` 204, `persist.rs` 196, `handlers.rs` 131, `feed.rs` 127. The test files are 523, 417, 348,
  213 and 136. None reaches 600, so none is near 900.
- Clippy `-D warnings` is clean (T), so there is no dead code.

**Protocol.** No wire change.

- `holler-proto` and `holler-pane` are untouched, so `CATALOG` is still 22 (`methods.rs:154`) and no golden file changed.
- `pane/*` are control-socket methods, and `docs/protocol/v2.md:816` already notes them and points to ADR-0021.
- Only closed codes are used.

**Tests**

- The store tests drive a real store on throwaway state dirs.
- The handler tests parse the reply line back as a client does, and fail on a JSON-RPC error frame. #669's in-process
  connection tests still pin the forwarding.
- Every real-hub test in `holler-cli` loads the registry through `PaneDeps::load` at startup.
- No pane test sleeps (the only matches for `sleep` are in comments). Every wait is a bounded `recv_timeout`, a
  `tokio::time::timeout` or the 100 ms window.
- Races are asserted as invariants.
- The RED-first evidence is in `handoff-T-red.md`.

**Documentation**

- The `CHANGELOG.md` entry is under `[Unreleased]` / `### Enhancements`, links #639, and is accurate. It names the file, the
  fail-closed behaviour, both `error` events, the 4 s window, the 1024-event ring, and the `store-corrupt` answer to a file
  from a newer build.
- The merge kept every other entry (#637, #669, #634, #670).
- The module docs match the code, and the three notes the first pass found stale are fixed (3b5e7ee).
- No text outside the handoffs still says an idle window answers `cursor: since`. Sibling issues #638, #643, #649, #651 and
  #661 do not mention the idle cursor.

**Public-repository privacy.** I grepped the whole 4407-line diff, handoffs included, for:

- home paths, user and account names;
- host and tailnet names (`jupiter`, `ts.net`, `.local`, `.lan`);
- IPv4 addresses, e-mail addresses other than no-reply ones, and key and token patterns.

The only hits are the `sample_pane` fixture moved from #669's test (`127.0.0.1` and the bare variable name
`ANTHROPIC_API_KEY`), the hostname `kiwi` (a fixture in 28 files on `main`), and the generic `~/.cache/t639-ref` in T's
handoff. Clean.

**Commit and PR hygiene**

- Every subject is a Conventional Commit.
- All 18 branch commits use the GitHub no-reply address as author and committer.
- Every commit carries `Co-Authored-By`, without a session link (see the advisory notes).
- No PR exists yet.

## Scope check

- **Delivered:** exactly the brief's scope. That is:
  - the persistent, generation-fenced registry, its change feed, and the five `pane/*` handlers;
  - the D8 amendment of #669's test;
  - one CHANGELOG entry;
  - the operator-approved ADR-0021 amendment (§6 and "Decisions taken" item 7).
- **Changed paths.** `git diff --name-only 55dba00 HEAD` lists only radius paths: `panes/**`, the five pane test files,
  ADR-0021, `CHANGELOG.md` and `docs/handoffs/639*`.
- **Untouched.** `git diff --quiet` confirms that `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`,
  `profile/**`, `holler-pane`, `holler-proto`, `holler-cli`, `holler-body`, every `Cargo.toml`, `Cargo.lock`,
  `docs/protocol/`, `README.md` and every golden file are unchanged.
- **Over-delivery:** none of substance. Every addition is a warn A routed to F, plus two small additions: F reads the version
  before the full parse, and two handler tests pin the edge cases. One is the test the first pass asked for, and the other
  closes the coverage gap F flagged.
- **Decomposition:** `pane_feed_test.rs` was needed to keep each file under AC 29's 600-line cap. A pre-flagged it (W-9),
  and its approval is now in `decisions.md`.
- **Under-delivery:** none.

## Verdict

**PASS.**

- All 30 acceptance criteria are met. AC 27 is met by grep and reading, as the brief specifies. Every other criterion is
  met by a named test that asserts the behaviour.
- The work complies with the epic, the issue, the brief (D1-D9 and its six documented deviations) and ADR-0021 as
  amended.
- The quality, privacy and scope checks are clean.
- Ready for O.

## Advisory notes

None of these blocks the merge.

1. **Three D2 load checks have no test.** They are an entry cursor of 0, a cursor past the head, and a record at
   generation 0 (`persist.rs:183-193`).
   - AC 9's table did not ask for them, and the code is right by reading.
   - In any later edit, T can add three rows to `corrupt_cases()` (`pane_registry_test.rs:290-318`), so a regression in
     those checks would fail a test.
2. **A `JoinError`'s text reaches the client (A-dup finding 2).** `handlers.rs:81-83` puts its `Display` into `what`, and
   in tokio 1.53 that text carries a panic payload.
   - `token.rs:664-667` uses a fixed message instead.
   - It cannot happen today: nothing in the closure panics, and the hub denies `panic`, `unwrap` and `expect`.
   - It is a one-line change, for any later edit of `handlers.rs`.
3. **A from-zero watcher can miss a create and delete inside one poll (carried over from the first pass).** This only
   happens when the snapshot was empty.
   - Within one poll, rule 2 is taken again after each wake-up. If a pane is created and deleted before the watcher takes
     the lock back, it never sees either; two updates collapse into the latest put.
   - D6 specifies this, and the final state is always right. Across polls, the head cursor closes the gap.
   - #638's conformance suite and #649's client should not assert stricter from-zero semantics.
4. **#661's reuse seams (A-dup finding 1).**
   - `Doc<E>` has no slot for the profile log that ADR §7 puts in `profiles.json`.
   - `Store::poll` and `Feed` are typed on panes, and `NoParams` is private.
   - `profile/mod.rs:61-63`, #669's file, still says `check_membership` holds `pane-in-other-profile`, but ADR item 2 moved
     that into the pane CAS.
   - `mod.rs:19-22` names only that comparison as #661's edit here.
   - Route all of this into #661's brief.
5. **The two new `error` events are documented only in the CHANGELOG and the module docs.** They are
   `pane_registry_corrupt` and `pane_registry_write_failed`. README and `docs/` are outside this radius, and no pane verb
   ships yet. `docs/pane-control.md` (#652) is the natural home.
6. **A wedged save blocks every caller with no `timeout`.** That falls short of the port's I5 wording
   (`ports.rs:56-58`). A W-10(c) accepted it as an operating limit for a local state dir, and `mod.rs:28-31` states it.
7. **Test-helper near-copies (A-dup finding 3).** `Rig::on` and `fresh_deps` rebuild the same pair of handles.
   `pane_support/mod.rs:6-7` names three includers, but there are four.
8. **Process.** The edge-case test (4a75b54) and the doc refresh (3b5e7ee) were made in the O step, not by T and F. Both
   were verified afterwards (T at 95b7724, A-dup at 9289d30, and this audit), so nothing is unreviewed.
9. **Commit trailers and the PR's AI disclosure.**
   - The commits carry `Co-Authored-By` without the session link that `CONTRIBUTING.md` describes. The recent squash merges
     on `main` (#634, #670, #635, #636) lack it too, so this is a repo-wide gap.
   - After the script opens the PR, the run's agent adds the AI disclosure to the PR body with `gh pr edit`.

## Earlier pass (31d2102, ADVISORY-HOLD): what became of each item

- **The idle long-poll cursor: resolved.**
  - The operator chose option A: keep the head and amend the ADR.
  - 639f130 amends ADR-0021 §6 and adds "Decisions taken" item 7. The same commit changes AC 25 and the radius in the brief.
  - 4a75b54 adds the edge-case test.
- **The three stale doc statements: fixed** in 3b5e7ee (`mod.rs:19-22`, `persist.rs:17-21`, `feed.rs:35-38`).
- **Approval of `pane_feed_test.rs`: journalled** (`decisions.md`, O entry).
- **The `CHANGELOG.md` conflict with `main`: resolved** in merge 1d67ef3, keeping every entry.
- **Still for after the PR opens:** the AI disclosure (advisory note 9).
