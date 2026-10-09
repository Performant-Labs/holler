# Handoff-S: Phase 10 - #639 the hub pane registry (spec audit)

**Date:** 2026-10-09, 10:07 MDT
**Branch:** issue-639-implementation (at 31d2102; merge base af3d8df; `origin/main` at 55dba00)
**Issue:** #639 (epic #633)
**Brief:** `docs/handoffs/639-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md`, `evidence.md`, and the outside diff review `639-diff-result-r1.md` (deepseek-v4-pro, PASS, no BLOCK;
gitignored)
**Diff audited:** `git diff origin/main...HEAD` in full: the five files under `crates/holler-hub/src/panes/`, the five pane
test files, and `CHANGELOG.md`. Also read: ADR-0021 as merged on `origin/main` (094ebfa), `holler-pane/src/ports.rs`,
and `holler-proto/src/atomic_file.rs`.
**Verdict:** ADVISORY-HOLD

## A precondition

Met. `handoff-A.md` is **PASS** (10 warns, no blocks). `handoff-A-dup.md` is **PASS** (3 warns, no blocks).

## T precondition

Met.

- **RED.** Against the real tree, the build fails on exactly the items this story creates (`PaneStoreOptions`,
  `PaneState::load_with`, `impl PaneStore for PaneState`). Against a scratch stub that has only that API, every new and
  amended test fails on an assertion (`handoff-T-red.md`).
- **GREEN.** The four pane binaries pass 14/7/6/10. The workspace passes 964 with 0 failed, and clippy, `lint.sh`,
  `changelog-check.sh` and `cargo machete` are clean. Three mutations were each killed: no `notify_all`, an ignored save
  error, and no generation check in `delete` (`handoff-T-green.md`).
- No blocking issues.

## Acceptance criteria

The registry tests are in `pane_registry_test.rs` (R), the feed tests in `pane_feed_test.rs` (Fd), and the handler tests in
`pane_handlers_test.rs` (H).

| AC | Criterion (short) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | `cas_put(p, 0)` creates at generation 1; `get` returns it | R `cas_put_creates_at_generation_one_and_get_returns_it` | MET |
| 2 | A stale generation is `Conflict`; record, file bytes and head unchanged | R `cas_put_with_a_stale_generation_is_a_conflict_and_changes_nothing` (expected 0, 1 and 3 against generation 2; compares the bytes and `head()`) | MET |
| 3 | Stored verbatim except `generation`; no session inferred | R `cas_put_stores_the_record_verbatim_except_generation` (submitted 7 and 42 are ignored; `session_of_record: None` next to `shown: Some`) | MET |
| 4 | `delete` checks a missing record before the generation; recreate is at generation 1 | R `delete_checks_missing_before_generation` | MET |
| 5 | `list` returns live records, sorted | R `list_returns_every_live_record_sorted_by_name` (created out of order, one deleted) | MET |
| 6 | Restart keeps profile, `r2c1` grid JSON, generations and deletion; next cursor is head + 1 | R `records_survive_a_restart_unchanged` | MET |
| 7 | A v1 record with no `profile` key loads with `None` | R `a_record_written_before_the_profile_field_loads_with_profile_none` (hand-written D2 file) | MET |
| 8 | Mode 0600, D2 shape, no temp file left, hub dir created | R `the_file_is_written_atomically_at_mode_0600`. The filter matches `atomic_file.rs`'s `.<name>.<pid>.<n>.tmp`, so the leftover check is real | MET |
| 9 | Corrupt file: every method `store-corrupt`, `what` names the file, bytes and listing unchanged | R `a_corrupt_file_fails_closed_and_is_never_rewritten` covers six cases: the brief's five plus distinct cursors (A W-5). F's `--nocapture` output shows each case failing on its own check. Without its check, each structural case would load, so every case matters | MET |
| 10 | Unreadable file (0o000), same as 9 | R `an_unreadable_file_fails_closed_and_is_never_rewritten` | MET |
| 11 | The corrupt reason never echoes file content | R `the_corrupt_reason_never_echoes_file_content`: the sentinel as a wrong-typed value, as an unknown field name, and as the version. The logged `problem` is the same `Problem` | MET |
| 12 | Unwritable dir: `unavailable`, old state kept, head unchanged, retry succeeds | R `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` (T's ignored-save mutation is killed here) | MET |
| 13 | 16 racing writers: exactly one wins; the file equals memory | R `racing_writers_exactly_one_wins` | MET |
| 14 | 8x25 read-modify-write loses no update; 200 consecutive put events | R `concurrent_read_modify_write_loses_no_update` | MET |
| 15 | Two `pane/cas_put` dispatches race on a multi-thread runtime | H `two_cas_put_requests_racing_exactly_one_wins` | MET |
| 16 | Watch from 0: current records by last change, then later writes | Fd `watch_from_zero_yields_every_current_record_then_later_changes` | MET |
| 17 | Every write once, in order, then idle `Ok(None)` | Fd `the_feed_delivers_every_write_exactly_once_in_order` | MET |
| 18 | Resuming neither repeats nor skips | Fd `resuming_from_the_last_cursor_neither_repeats_nor_skips` | MET |
| 19 | A waiting watch wakes on the next write | Fd `a_waiting_watch_wakes_on_the_next_write` (20 rounds, a 20 s window against a 10 s `recv_timeout`; T's no-notify mutation is killed here) | MET |
| 20 | A cursor ahead of the head is `usage` | Fd `a_cursor_ahead_of_the_store_is_usage` | MET |
| 21 | After a restart, an old cursor gets each changed pane once, deletions included | Fd `after_a_restart_an_old_cursor_gets_each_changed_pane_once_including_deletions` (floor 5 > since 1, so rule 4 applies) | MET |
| 22 | A watcher behind the retained window gets the compacted changes | Fd `a_watcher_behind_the_retained_window_gets_the_compacted_changes` (`feed_retained: 2`; floor 3 > since 1) | MET |
| 23 | Round trip, D7 data shapes, no `params` | H `get_list_cas_put_delete_round_trip`, H `a_request_without_params_works_for_list_and_watch` | MET |
| 24 | Bad params answer their own code | H `bad_params_answer_their_code` | MET |
| 25 | `pane/watch` answers one batch and its cursor; idle answers `{"events": [], "cursor": since}` | H `pane_watch_answers_one_batch_and_its_cursor`, tested only where `since` equals the head (2). The code answers `cursor: head`, which is not `since` when `since` is 0 and every record has been deleted. No test covers that case | MET as tested. **Conflicts with D6 in one untested case: see Verdict** |
| 26 | A corrupt registry answers `store-corrupt` as a result | H `a_corrupt_registry_answers_store_corrupt_on_every_pane_method` (`outcome` fails on a JSON-RPC error frame) | MET |
| 27 | `check_membership` runs before the CAS; an `Err` is the reply and nothing is written | `grep -n "check_membership(" crates/holler-hub/src/panes/` matches `handlers.rs:111`, before `store.cas_put` at :112, inside the blocking closure. Its `?` reaches `run`, then `PaneReply::failure`. No behavioural test is possible until #661, by design | MET (grep and reading) |
| 28 | The #669 test is amended (D8) | `pane_dispatch_test.rs`: three assertions changed (two tests renamed); `fresh_deps` uses `short_opts()`; every `profile/*` assertion is unchanged | MET |
| 29 | Build guards | T-green Tier 1 and F's rustfmt run. My own checks: the largest new file is 523 lines; every new `#[allow]` has `// #639`; holler-proto and holler-pane are untouched, so `CATALOG` and the golden files are too; the CHANGELOG entry is under `[Unreleased]` / `### Enhancements` and links #639 | MET |
| 30 | The diff lists only blast-radius paths | The diff is `panes/**`, the five pane test files, `CHANGELOG.md` and `docs/handoffs/639*`. `pane_feed_test.rs` is not in the brief's list. The split was forced by AC 29's 600-line cap (one file measured 726 lines), A pre-flagged it as W-9, and T explained it. O's approval is **not yet in `decisions.md`** | MET in substance; **approval not journalled (O)** |

## Spec compliance

**Decisions already made (epic, issue, #669): all implemented.**

- The hub is the store only, with five verbs including `delete`.
- Every reply is a `PaneReply` sent as a JSON-RPC result, and `pane/watch` is a long-poll.
- Persistence and the fail-closed load are inside `PaneState::load`, which stays infallible.
- `profile` persists, and `herdr.grid` is stored as-is.
- `PaneState` does not derive `Clone`.
- No new error code was added.

**Brief decisions D1 to D9: implemented.** Each deviation below is documented in handoff-F, was routed by A, and stays
inside the radius. I accept all of them.

- `dispatch` takes `profiles: &Arc<ProfileState>` (A W-1), so `check_membership` runs on the blocking pool, outside the
  pane lock. `pane_dispatch.rs:71-74` sanctions this.
- `poll` and the `Watch` iterator are in `store.rs`, and `feed.rs` is pure (A W-4).
- `PaneEvent` is the file entry, inside a generic `Doc<E>`. A tombstone is written as `"pane": null`, and both forms read
  back (A W-5).
- `Problem::what(label, path)` replaces `corrupt_what`, and the `what` texts are D3's and D4's (A W-5b).
- Entry cursors must be distinct (A W-5d).
- A failed save logs `pane_registry_write_failed`. D4 left this open, and A W-3 asked F to decide.

**One open conflict: the idle long-poll cursor.** This is the reason for the hold.

- D6 says "The reply's `cursor` is always `head`".
- AC 25 says an idle reply is `{"events": [], "cursor": since}`.
- ADR-0021 §6, merged to `main` during this run (094ebfa, line 219), **decides** the AC 25 form: "An idle window answers
  `{"events": [], "cursor": since}`."
- The code follows D6: `store.rs:260-264` always answers `cursor: table.head`, and `feed.rs:32-33` documents this.

The two rules differ in one reachable case: `since` is 0 and every record has been deleted (head > 0, no live record).
Every test of an idle reply has `since == head`, so none tells the two apart. F documented the divergence, so it is not
silent. The brief itself is self-contradictory, though, and its radius excludes ADR-0021. As a result the story cannot make
the code and the standing ADR agree. Details and the proposed fix are under Verdict.

## Quality audit

**Correctness and failure handling**

- **CAS.** Every write goes through `holler_pane::next_generation` (`store.rs:171`, `:193`). `delete` checks for a missing
  record before the generation (`store.rs:188-192`), as the port requires.
- **Write order (D1).** `commit` saves first, then sets the head, the ring and the entries, then notifies
  (`store.rs:203-210`). A failed save changes nothing: AC 12, plus T's mutation.
- **Fail closed.** A failed load is the `Err` arm of `Mutex<Result<Table, PaneError>>`. Every method, `poll` included,
  returns that error, and no write path can reach the file (AC 9, 10, 26).
- **No content echo.** A `Problem` is built only from a serde category, line and column, the version number, validated pane
  names, cursor numbers, and the io error (`persist.rs:98-143`). serde's message is never used.
- **Concurrency.**
  - One mutex guards the table. The condvar wait recomputes its deadline after every wake-up, and it re-runs `select`
    before the deadline check, so a wake at the deadline still delivers its events (`store.rs:249-272`). The outside
    reviewer's W-1 does not hold.
  - The store is reached only through `spawn_blocking`, so the lock is never held across an `.await`.
  - A `JoinError` maps to `unavailable` (`handlers.rs:81-83`).
- **Feed.** The ring keeps the invariant "every event after the floor is held", through eviction and after a load
  (`feed.rs:57-74`). Rule selection is one pure function (`feed.rs:103-120`).
- **Restart.** Cursors never go backwards: the head is persisted, and the next write takes head + 1 (AC 6).

**Build guards**

- `panes/` has no `unwrap`, `expect`, `panic!`, `unreachable!` or `unsafe` (grep), and no `#[allow]`.
- Every test-file `#![allow]` carries `// #639`. The existing ones in `pane_dispatch_test.rs` keep `// #669`.
- Sizes: `store.rs` 340, `mod.rs` 202, `persist.rs` 194, `handlers.rs` 131, `feed.rs` 127. Tests: 523, 348, 332, 213, 136.
  No file is near 900.
- Clippy `-D warnings` is clean (T), so there is no dead code.

**Protocol.** Nothing changes. holler-proto and holler-pane are untouched. `pane/*` stays outside the 22-row `CATALOG`.
Only closed codes are used (`Conflict`, `PaneNotFound`, `StoreCorrupt`, `Unavailable`, `Usage`, `NotImplemented`), and no
golden file changes. `docs/protocol/v2.md` is out of scope: `main` already notes the methods, and ADR-0021 specifies them.

**Tests**

- The tests drive a real store on throwaway state dirs, and the handler tests parse the reply line back as a client would.
  #669's in-process connection tests still pin the socket forwarding.
- There is no `sleep` in any pane test (grep). Every wait is bounded, and races are asserted as invariants.
- RED-first evidence is in `handoff-T-red.md`.

**Documentation**

- The `CHANGELOG.md` entry is accurate. It names the file, the fail-closed behaviour, both new `error` events, the 4 s
  window, the 1024-event ring, and the `store-corrupt` answer for a file from a newer build.
- **Three doc statements went stale when ADR-0021 merged.** They are folded into the proposed fix.
  - `mod.rs:19-20` says #661 "edits nothing here".
  - `persist.rs:17-19` says #661 needs no edit "to this directory".
  - The ADR contradicts both. Its "Decisions taken" item 2 says #661 makes a one-line change in this registry's CAS. Its
    §7 says the profile change log is persisted in `profiles.json`, and `Doc<E>`'s `deny_unknown_fields` would refuse it.
    The brief's own D9 already said #661 needs an edit under `panes/`.
  - `feed.rs:37` cites "the draft ADR-0021 §7", but the ADR is merged.

**Public-repository privacy.** I grepped the whole diff for home paths, user, machine and account names, tailnet and
`ts.net` names, IPv4 addresses, private domains, and key or secret patterns. The only hits are in the `sample_pane` fixture
moved from #669's test: `host.name: "kiwi"` (a fixture hostname used across the repo, golden files included),
`127.0.0.1`, and the variable name `ANTHROPIC_API_KEY` with no value. Clean.

**Commit and PR hygiene**

- Subjects are Conventional Commits (`chore(#639): ...`, `docs(handoffs): ...`).
- The trailers are `Co-Authored-By: Claude ... <noreply@anthropic.com>` with no session link. That is the same as the
  recently merged #670 and #634 squash commits: a repo-wide gap, so it is advisory here.
- No PR exists yet. After the script opens it, the run's agent adds the `CONTRIBUTING.md` AI disclosure with
  `gh pr edit`.

## Scope check

- **Delivered:** exactly the brief's scope. That is the persistent, generation-fenced registry, its change feed, the five
  `pane/*` handlers, the D8 amendment of #669's test, and one CHANGELOG entry.
- **Untouched, as required:** `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `profile/**`, holler-pane,
  holler-proto, every `Cargo.toml`, `Cargo.lock`, the golden files and the ADRs.
- **Over-delivery:** none of substance. Every addition is a warn A routed to F: the `RegistryEntry` trait, the generic
  `Doc<E>`/`Ring<E>`/`handlers::run` for #661, the distinct-cursor check, and the write-failure event. The one exception is
  F's own small addition: the version is read before the full parse, so a newer file is reported by its version.
- **Decomposition:** `pane_feed_test.rs` is a necessary split (A W-9, explained by T). It only needs O's journal entry.
- **Under-delivery:** none.

## Verdict

**ADVISORY-HOLD**

**Defect.** The brief contradicts itself on the cursor of an idle long-poll reply.

- D6 says it is always the head.
- AC 25 says it is `since`.
- ADR-0021 §6, merged after the brief was written, "decides" the AC 25 form.
- F faithfully implemented D6 (`store.rs:260-264`, `feed.rs:32-33`).
- The rules differ when `since` is 0 and every record has been deleted. The code then answers `cursor: <head>` where the
  ADR says `cursor: 0`. No test pins either answer.
- The brief puts ADR-0021 out of scope and out of the radius, so no role in this run can bring code and ADR into agreement
  without a decision.
- The same mid-run merge made the three doc statements above stale.

**Conventions violated**

- The architecture-reviewer overlay: "A decision that contradicts or extends an ADR in `docs/adr/` needs the ADR updated
  in the same change, not silently ignored."
- CLAUDE.md: ADRs in `docs/adr/` are the standing spec.
- The handoffs that record the divergence are removed before push (pipeline-conventions §1). Merged as is, `main` would
  carry code and an ADR that disagree, with no record of why.

**Proposed fix (recommended: option A, keep the code and amend the ADR)**

1. **O:** widen the radius to `docs/adr/ADR-0021.md` and journal it. Merge `origin/main` into the branch first: that
   brings the ADR in, and the only conflict is `CHANGELOG.md` (keep every entry).
2. **F:** amend the §6 sentence (line 219), for example: "An idle window answers `{"events": [], "cursor": <head>}`.
   The head equals `since` except for a watch from 0 whose records have all been deleted; resuming from it delivers every
   later change exactly."
   - Why the head is right: with `since` (0), a from-zero watcher on an all-deleted registry stays in snapshot mode across
     idle polls. A pane created and deleted between two polls would never reach it, which breaks the frozen `Watch`
     contract ("then every later change", `holler-pane/src/ports.rs:38-40`).
   - O also corrects AC 25's wording in the journal.
3. **T:** add a handler test that pins the edge case.
   - `cas_put` A, delete A, then `pane/watch {since: 0}` answers `{"events": [], "cursor": 2}` after the window.
   - Then create and delete B, and `pane/watch {since: 2}` answers both events.
4. **F:** refresh the stale doc statements.
   - `mod.rs:19-20` and `persist.rs:17-19`: say what #661 extends here (the CAS membership rule and `Doc` for the
     profile log), not "edits nothing".
   - `feed.rs:37`: drop "draft".

**Option B (if the operator prefers the ADR text as written).**

- `Store::poll` answers `cursor: since` when the window ends with no events (`store.rs:260-264`).
- Update `feed.rs:32-33` to match.
- Pin the edge with the same test, expecting `cursor: 0`.
- Make the same doc refreshes as in option A.
- Trade-off: this accepts the weaker from-zero guarantee described in option A.

**O items under either option**

- Journal the approval of `crates/holler-hub/tests/pane_feed_test.rs` in the radius (AC 30, A W-9).
- Resolve the `CHANGELOG.md` conflict with `main`.
- Add the AI disclosure to the PR body.

Everything else is ready. ACs 1-24 and 26-29 are met by tests that assert the behaviour, and AC 25 is met as tested. The
quality, privacy and scope checks are clean. Once the operator decides, the remaining work is small.

## Advisory notes

None of these block the merge.

- **Rule 2 is re-run on every wake-up (D6 as written).** A from-zero watcher waiting on an empty snapshot sees the records
  that exist when it wakes.
  - A create followed by a delete (or an update) is invisible (or collapsed) within that one poll if both commit before
    the waiter takes the lock again.
  - After its first reply, the watcher resumes from the head and gets every change exactly. The final state is always
    right.
  - This is practically unreachable through CLI verbs, because each verb's reply round trip is far longer than a wake-up.
  - #638's conformance suite and #649's client should not assert stricter from-zero semantics. Under option A, the §6
    wording can say "the records held when the reply is made".
- **For #661's brief (A-dup W-2).** `Doc<E>` refuses unknown fields, so the profile log needs `Doc` extended. `Store::poll`
  and `Feed` are typed on `PaneEvent`. `NoParams` is private. #661's radius needs `panes/{persist,store,handlers}.rs`, and
  it should lift these parts, not copy them.
- **Test-helper near-copies (A-dup W-3).**
  - `Rig::on` and `fresh_deps` build the same pair of state handles.
  - `check_membership_accepts_any_pane` builds its own temp dir.
  - `pane_support/mod.rs:6-7` names three includers, but `pane_feed_test.rs` is a fourth.
  - All cosmetic; T can fold them into any later edit.
- **`WATCH_WAIT` (4 s) and the default client timeout (5 s).** `WATCH_WAIT`'s doc states the relationship, but no test pins
  it. A window raised to 5 s or more would time out every idle `pane/watch` on a default client. #649 sizes its timeout from
  the constant.
- **The two new `error` events are not in the README.** `pane_registry_corrupt` and `pane_registry_write_failed` appear
  only in the CHANGELOG and the module docs. The brief kept README and `docs/` out of the radius, and `hold_state_write_failed`
  is likewise undocumented outside the code. Worth a line when the pane verbs become user-facing (#649, #654).
- **Merge mechanics.** `main` also bumps `Cargo.lock` (2 lines, #670), with no overlap with this diff.
