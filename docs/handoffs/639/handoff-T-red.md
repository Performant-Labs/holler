# Handoff-T-red: Phase 4 - #639 the hub pane registry

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Brief / wireframe reviewed:** docs/handoffs/639-brief.md (no wireframe: no UI surface); docs/handoffs/639/handoff-A.md

## A precondition
Confirmed: A returned PASS on the plan (Phase 3, handoff-A.md, 10 warns, no blocks).

## Tests authored
All in `crates/holler-hub/tests/` (auto-discovered; the hub crate has no `autotests = false`). 27 new tests plus 3 amended #669 tests.

New files, staged by explicit path:
- `pane_support/mod.rs` (136 lines): shared helpers. `sample_pane(name, profile)` (moved from `pane_dispatch_test.rs`), `name`, `temp_state` (does not create `<root>/hub`), `short_opts` (100 ms window, default ring), `load`, `create`, `drain`, `head`, `dir_listing`, `registry_file`, and the one reply parse-back (`pane_outcome` moved from `pane_dispatch_test.rs`, plus `outcome(line)` over it). `head()` finds the feed head through the public port alone (binary search on "`watch(n)` is accepted iff `n <= head`").
- `pane_registry_test.rs` (523 lines), store tests, tier: integration over a real state dir, no socket.
- `pane_feed_test.rs` (213 lines), the feed tests. **Split out of the brief's file** because the brief's single file measured 726 lines after rustfmt (AC 29: no new file reaches 600). This is A's finding W-9. It adds one path outside the brief's file list, so **O must record the approval in decisions.md** or AC 30's radius check fails.
- `pane_handlers_test.rs` (332 lines), handler tests: `panes::dispatch` called directly, reply line parsed back.

| Test | Pins | Tier |
|---|---|---|
| `cas_put_creates_at_generation_one_and_get_returns_it` | AC 1 | integration (store) |
| `cas_put_with_a_stale_generation_is_a_conflict_and_changes_nothing` | AC 2 (stored record, file bytes, head cursor unchanged; expected 0, 1 and 3 against a record at 2) | integration |
| `cas_put_stores_the_record_verbatim_except_generation` | AC 3 (submitted generation ignored, `session_of_record` stays `None` next to `shown: Some`) | integration |
| `delete_checks_missing_before_generation` | AC 4 | integration |
| `list_returns_every_live_record_sorted_by_name` | AC 5 | integration |
| `records_survive_a_restart_unchanged` | AC 6 (profile, `r2c1` grid JSON, generations, deleted stays deleted, head survives a trailing delete, next cursor = head + 1) | integration |
| `a_record_written_before_the_profile_field_loads_with_profile_none` | AC 7 (hand-written D2 file) | integration |
| `the_file_is_written_atomically_at_mode_0600` (unix) | AC 8 (mode, D2 shape, no `.panes.json.*.tmp`, `<root>/hub` created by the write) | integration |
| `a_corrupt_file_fails_closed_and_is_never_rewritten` | AC 9: table of **six** cases, the brief's five plus "two entries share a cursor" (A's W-5); each of get/list/cas_put/delete/watch is `store-corrupt` with `what` naming `panes.json`; bytes and directory listing unchanged | integration |
| `an_unreadable_file_fails_closed_and_is_never_rewritten` (unix) | AC 10 | integration |
| `the_corrupt_reason_never_echoes_file_content` | AC 11 (sentinel as a wrong-typed value, as an unknown field NAME, and as the version) | integration |
| `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` (unix) | AC 12 | integration |
| `racing_writers_exactly_one_wins` | AC 13 (16 threads, Barrier) | integration |
| `concurrent_read_modify_write_loses_no_update` | AC 14 (8x25, 200 consecutive put events) | integration |
| `two_cas_put_requests_racing_exactly_one_wins` | AC 15 (multi-thread runtime, tokio Barrier) | handler |
| `watch_from_zero_yields_every_current_record_then_later_changes` | AC 16 | integration |
| `the_feed_delivers_every_write_exactly_once_in_order` | AC 17 | integration |
| `resuming_from_the_last_cursor_neither_repeats_nor_skips` | AC 18 | integration |
| `a_waiting_watch_wakes_on_the_next_write` | AC 19 (20 rounds, 20 s window, bounded `recv_timeout`) | integration |
| `a_cursor_ahead_of_the_store_is_usage` | AC 20 | integration |
| `after_a_restart_an_old_cursor_gets_each_changed_pane_once_including_deletions` | AC 21 (D6 rule 4) | integration |
| `a_watcher_behind_the_retained_window_gets_the_compacted_changes` | AC 22 (`feed_retained: 2`) | integration |
| `get_list_cas_put_delete_round_trip` | AC 23 | handler |
| `a_request_without_params_works_for_list_and_watch` | AC 23 (absent and `null` params) | handler |
| `bad_params_answer_their_code` | AC 24 | handler |
| `pane_watch_answers_one_batch_and_its_cursor` | AC 25 (including the idle `{events: [], cursor: since}`) | handler |
| `a_corrupt_registry_answers_store_corrupt_on_every_pane_method` | AC 26 (as a JSON-RPC result) | handler |
| Amended in `pane_dispatch_test.rs`: `every_pane_method_is_forwarded_to_the_registry_not_method_not_found` (renamed), `two_connections_share_the_one_pair_of_state_handles`, `panes_dispatch_takes_both_handles_and_answers_as_a_result` (renamed) | AC 28: `pane/list` -> `Ok(Some([]))`; `pane/get`, `cas_put`, `delete` with no params -> `usage`; `pane/watch` -> `{"events": [], "cursor": 0}`. `fresh_deps` builds the registry with `short_opts()`. `profile/*` assertions untouched. `sample_pane`/`pane_outcome` now come from `pane_support` (A's W-2), and `check_membership_accepts_any_pane` calls the moved `sample_pane(name, profile)`. | handler / in-process connection |

**AC 27 has no behavioural test, deliberately.** `check_membership` is `Ok(())` until #661, so no test can make it refuse. S verifies it with `grep -n "check_membership(" crates/holler-hub/src/panes/`, as the brief says.

No `thread::sleep` anywhere. Every wait is `recv_timeout`, `tokio::time::timeout`, or the 100 ms `watch_wait` of `short_opts()`.

## RED confirmation
Command: `cargo test -p holler-hub --no-fail-fast --test pane_registry_test --test pane_feed_test --test pane_handlers_test --test pane_dispatch_test`

**1. Against the real tree (what F starts from): compile failure, as the brief's test plan predicts.** The four binaries stop at 50, 25, 5 and 5 errors. I grouped all of them: every one is an unresolved item that this story creates, and none is a typo, a missing helper or a missing `[[test]]` (the hub crate has none):
```
error[E0432]: unresolved import `holler_hub::panes::PaneStoreOptions`
error[E0422]: cannot find struct ... `PaneStoreOptions` in module `holler_hub::panes`      (x2)
error[E0599]: no associated function ... named `load_with` found for struct `PaneState`      (x5)
error[E0599]: no method named `get` / `list` / `cas_put` / `delete` / `watch` found for `PaneState`
```
That is the whole set: `PaneStoreOptions`, `PaneState::load_with`, and `impl PaneStore for PaneState`. (`WATCH_WAIT` is not referenced by any test.)

**2. Against a scratch stub that adds only that API surface** (every method answers `NotImplemented`; scratch copy at `~/.cache/t639-ref`, outside the repo, never committed): every test compiles and every new or amended test fails on an assertion or an unwrap of the missing behaviour, for example
```
pane/get without params must be a usage error, got Err(NotImplemented)
assertion `left == right` failed: an idle registry answers an empty batch ... left: Err(NotImplemented) right: Ok(Some({"cursor":0,"events":[]}))
unreadable: expected store-corrupt, got NotImplemented
```
Result: 3 of the 10 `pane_dispatch_test` tests fail (the 3 amended ones; the 7 unchanged, `profile/*` and plumbing tests pass), and 7/7 feed, 6/6 handler, 14/14 registry tests fail. Many store tests fail first at their setup `cas_put(..).unwrap()` with `NotImplemented`: it is the missing behaviour, not a harness error, but those tests do not reach their own distinguishing assertion until F's store exists.

**3. Satisfiability check (beyond what RED needs).** I wrote a throwaway reference implementation in the same scratch copy (never delivered, not production quality) to prove the tests can go green and are not self-contradictory: all 27 new and 3 amended tests pass, 10/10 in `pane_dispatch_test`; 25 repeat runs of the registry, feed and handler binaries had no failure; `cargo clippy -p holler-hub --tests` is clean for the test files. F still writes the real implementation from the brief.

Other checks on the committed-to-be test files: `rustfmt --edition 2021 --check` clean on all five; `bash scripts/lint.sh` reports no `lint:` failure; every `#[allow]` carries `// #639`; no file reaches 600 lines (523 max).

## Ready for F
Confirmed: RED is valid. F may implement against these tests. F must provide exactly `PaneStoreOptions { watch_wait: Duration, feed_retained: usize }` with `Default` (tests use `..PaneStoreOptions::default()`), `PaneState::load_with(&HubState, PaneStoreOptions) -> PaneState` (infallible), `impl PaneStore for PaneState`, and `holler_hub::panes::dispatch` with its current parameter shape or `&Arc<ProfileState>` (the tests pass `&Arc<..>` handles, which fit either).

Points for O and F:
- **O:** record the approval of `crates/holler-hub/tests/pane_feed_test.rs` in the radius (A's W-9), or AC 30 fails.
- **O:** the sixth AC 9 case (entry cursors must be distinct) is A's W-5. The brief's D2 validation list does not have it, so F must add it to `load` or O drops the case from `corrupt_cases()`.
- **F:** the idle-window tests rely on `next()` returning `Ok(None)` after `watch_wait` and then still being usable (AC 16-17); the long-poll handler must answer `{"events": [], "cursor": <head>}` when idle.
- **F:** `a_waiting_watch_wakes_on_the_next_write` uses a 20 s window with a 10 s bounded wait: a store that only wakes at the window's end fails it.
