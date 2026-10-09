# Handoff-T-red: Phase 4 - #661 the hub profile registry (RED)

**Date:** 2026-10-09
**Branch:** issue-661-implementation
**Brief / wireframe reviewed:** docs/handoffs/661-brief.md; no wireframe (no UI surface). Also docs/handoffs/661/handoff-A.md (findings 3, 4, 5, 6 are addressed to T and F).

## A precondition
Confirmed: A returned PASS on the plan (Phase 3, `docs/handoffs/661/handoff-A.md`, 8 warns, no blocks).

## Tests authored
All under `crates/holler-hub/tests/`, all at the cheapest sufficient tier: store tests drive `ProfileState` through the `ProfileStore` trait from plain threads (no socket); handler tests call `profile::dispatch` / `panes::dispatch` directly; no process-level test is needed (no CLI surface). Every wait is bounded (`recv_timeout`, `tokio::time::timeout`, `short_opts()`); there is no `thread::sleep`.

| File (new) | Lines | Tests | AC |
|---|---|---|---|
| `profile_registry_test.rs` | 447 | `the_registry_passes_the_profile_store_conformance_suite` (23 cases, also pins the case count), `profiles_are_filed_by_slug_and_the_submitted_slug_is_ignored`, `the_name_rule_runs_before_the_generation`, `a_delete_is_logged_at_the_deleted_generation_plus_one` (also each entry's actor), `the_log_survives_delete_and_recreate_and_a_restart`, `the_hub_stamps_the_times_and_ignores_the_submitted_ones`, `a_refused_write_changes_nothing_and_logs_nothing` (table of 5), `an_update_summary_is_one_line_counting_specs`, `a_spec_pane_name_holding_a_newline_cannot_reach_the_summary`, `the_log_cannot_be_rewritten`, `racing_writers_exactly_one_wins` (16 threads, Barrier), `concurrent_read_modify_write_loses_no_update` (8 x 25) | 1-9, 18, 19 |
| `profile_persistence_test.rs` | 404 | `records_logs_and_tombstones_survive_a_restart_unchanged`, `the_file_is_written_atomically_at_mode_0600` (unix), `loading_never_writes` (bytes, dir listing, mtime), `a_corrupt_file_fails_closed_and_is_never_rewritten` (10-row table), `an_unreadable_file_fails_closed_and_is_never_rewritten` (unix), `a_stored_secret_value_fails_closed_without_echoing_it`, `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` (unix), `the_two_registries_fail_independently` | 10-17 |
| `profile_feed_test.rs` | 192 | `the_feed_delivers_every_write_exactly_once_in_order` (includes a delete by another spelling and a re-create), `watch_from_zero_yields_every_current_profile_then_later_changes`, `a_waiting_watch_wakes_on_the_next_write`, `a_cursor_ahead_of_the_store_is_usage`, `after_a_restart_an_old_cursor_gets_each_changed_profile_once_including_deletions`, `a_watcher_behind_the_retained_window_gets_the_compacted_changes` | 20-25 |
| `profile_handlers_test.rs` | 577 | `get_list_cas_put_delete_log_round_trip`, `a_request_without_params_works_for_list_and_watch`, `bad_params_answer_their_code`, `a_secret_value_is_refused_over_the_wire_and_never_echoed`, `profile_watch_answers_one_batch_and_its_cursor`, `the_log_cannot_be_rewritten_over_the_wire`, `two_cas_put_requests_racing_exactly_one_wins`, `a_corrupt_registry_answers_store_corrupt_on_every_profile_method` (rename still `not-implemented`), `a_detached_spec_is_accepted`, `pane_cas_put_naming_a_missing_profile_is_profile_not_found`, `a_pane_outside_any_profile_never_reads_the_profile_registry` | 26-33, 38, 39 |
| `pane_membership_test.rs` | 190 | `the_hub_pane_registry_passes_the_pane_store_conformance_suite` (19 cases), `moving_a_pane_to_another_profile_is_refused_inside_the_cas`, `keeping_the_profile_or_respelling_it_with_the_same_slug_is_allowed`, `joining_a_profile_from_none_is_allowed`, `a_stale_generation_wins_over_the_profile_rule`, `two_writers_joining_different_profiles_exactly_one_wins_and_the_loser_is_refused_on_retry` | 34-37 |

Amended: `pane_dispatch_test.rs` (AC 42: `fresh_deps` uses `load_with(.., short_opts())`; `every_profile_method_is_forwarded_to_the_registry_not_method_not_found` with the per-method table; two-connection `profile/list` -> `Ok(Some([]))`; direct dispatch `profile/get` -> usage, `profile/rename` -> not-implemented; `check_membership_accepts_any_pane` replaced by `check_membership_requires_the_named_profile_to_exist` (AC 40); module doc updated). `pane_handlers_test.rs` (AC 43: the round-trip test creates `Night Shift` in the rig's profile registry first; nothing else changed). `pane_support/mod.rs` extended (see departures).

AC 41 and 44-45 are S-side greps and build guards, not tests.

### Departures from the brief's Files list (all recorded here)
1. **Persistence tests live in their own file**, `profile_persistence_test.rs` (AC 10-17), not in `profile_registry_test.rs`. With them the registry file was 836 lines (lint warns at 600, fails at 900); split, the largest new file is 577. The brief's "four new test files" is five. No `Cargo.toml` change: `holler-hub` does not set `autotests = false`.
2. **A finding 3 applied.** No `drain_profiles` and no second binary search. `pane_support::drain` is now generic over the event (`fn drain<E: Debug>(&mut Watch<E>)`, `Watch` is the port's alias), and `head`'s search is `head_by(opens)`, with `head(&PaneState)` and `profile_head(&ProfileState)` as one-line wrappers. `pane_registry_test.rs` and `pane_feed_test.rs` are untouched and, with a throwaway stub in place, still pass (14 and 7 tests).
3. **A finding 5 applied.** AC 13's corrupt table has a tenth row: a live entry whose `event.name` differs from its record's `name` (same slug, different spelling). F's `ProfileEntry` `try_from` must refuse it. The table is otherwise the brief's nine rows.
4. Extra `pane_support` helpers beyond the brief's list: `profiles_doc(state)` and `write_profiles_doc(state, doc)` (used by two files), `profile_head`, `head_by`.
5. Two extra tests beyond the AC list, each a distinct behavior: `keeping_the_profile_or_respelling_it_with_the_same_slug_is_allowed` and `joining_a_profile_from_none_is_allowed` (AC 35's "allowed" half, so the rule is not over-broad), and `a_spec_pane_name_holding_a_newline_cannot_reach_the_summary` (AC 8's last sentence). `a_stale_generation_wins_over_the_profile_rule` also passes today, by design: it pins an ordering that must survive F's change.

## RED confirmation

Order of events, per A finding 4: AC 34-37 were run for an assertion RED **before** `pane_support` was extended.

**1. `pane_membership_test.rs` against today's code (`cargo test -p holler-hub --test pane_membership_test`):**
```
test a_stale_generation_wins_over_the_profile_rule ... ok
test joining_a_profile_from_none_is_allowed ... ok
test keeping_the_profile_or_respelling_it_with_the_same_slug_is_allowed ... ok
test moving_a_pane_to_another_profile_is_refused_inside_the_cas ... FAILED
test two_writers_joining_different_profiles_exactly_one_wins_and_the_loser_is_refused_on_retry ... FAILED
test the_hub_pane_registry_passes_the_pane_store_conformance_suite ... FAILED
test result: FAILED. 3 passed; 3 failed
```
- AC 34: `the pane registry fails 1 case(s): ["pane-in-other-profile"]` with detail `cas_put moving a pane of Alpha to Beta: expected `pane-in-other-profile`, but it succeeded`. **Exactly the one expected case; the brief's escalate condition does not apply** (no other pane conformance case fails, so no #639 defect).
- AC 35: `called Result::unwrap_err() on an Ok value: Pane { .. generation: 2 .. profile: Some("Day Shift") }` (the move was accepted).
- AC 37: the loser's retry at generation 2 returned `Ok` (`generation: 3`, `profile: Some("Night Shift")`) instead of `pane-in-other-profile`.
- The three that pass are allowed-behavior and ordering guards, not RED tests (departure 5).

**2. After extending `pane_support` (`cargo test -p holler-hub --no-run`).** Every test binary that includes `pane_support` (all nine) now fails to compile, which is the transient compile-RED A finding 4 predicted, including the unamended #639 binaries `pane_registry_test` and `pane_feed_test`. It is not a regression in them. The complete set of distinct error causes, with no other error kind:
```
error[E0599]: no associated function or constant named `load_with` found for struct `ProfileState`   (5 sites)
error[E0599]: no method named `get`|`list`|`cas_put`|`delete`|`watch`|`log` found for struct/reference `ProfileState`
error[E0277]: the trait bound `ProfileState: ProfileStore` is not satisfied   (1)
```
That is exactly `load_with` and the `impl ProfileStore for ProfileState` the brief names; there is no typo, missing helper, or missing `[[test]]` entry.

**3. Type-check and execution of the tests, without writing production code.** To prove the tests compile against the intended API and fail on behavior rather than on a hidden error, I temporarily appended a do-nothing `load_with` and a `ProfileStore` impl that answers `not-implemented` to `crates/holler-hub/src/profile/mod.rs`, built and ran, then reverted it with `git checkout -- crates/holler-hub/src` (`git status` shows no production file changed). Results with the stub:
- `profile_registry_test` 0/12, `profile_persistence_test` 0/8, `profile_feed_test` 0/6, `profile_handlers_test` 0/11 pass (all fail on the first registry call, as expected of a stub).
- `pane_dispatch_test`: 4 fail (`every_profile_method_is_forwarded_to_the_registry_...`, `two_connections_share_...`, `profile_dispatch_takes_both_handles_...`, `check_membership_requires_...`), 6 pass (every unamended `pane/*` assertion and the plumbing tests). `pane_handlers_test`: only the round-trip fails (profile creation), 7 pass. `pane_registry_test` 14/14 and `pane_feed_test` 7/7 pass, so the `drain`/`head` refactor broke nothing.
- `cargo clippy -p holler-hub --all-targets -- -D warnings` was clean with the stub except `drop_non_drop` on `drop(store)`, which only the stub's empty type triggers (the real `ProfileState` holds an `Arc`).
- Limit of this check: it proves compilation and that nothing passes vacuously, not that every assertion is right. The hand-written-file rows (AC 13, 15) and the exact file layout (AC 11) are built from the D2 layout in the brief and from a real file written by the registry; if F's layout reads differently from D2, the test is fixed in the GREEN round (the tests are mine).

`bash scripts/lint.sh` passes (rc 0); no new file reaches 600 lines (largest: `profile_handlers_test.rs`, 577). The new and touched test files are `rustfmt --edition 2021` clean. No `#[allow]` lacks its `// #661` link. Nothing is committed; the test files are staged by explicit path.

## Ready for F
Confirmed: RED is valid. F may implement against these tests. Notes for F:
- `ProfileState::load_with(&HubState, PaneStoreOptions)` and `impl ProfileStore for ProfileState` are the only compile blockers. `rename` returning `not-implemented` is what the suite's `rename-is-not-implemented` case and AC 32 need.
- Error text: a `store-corrupt` `what` must start with `profile registry <path-of-profiles.json>` (AC 13 asserts the prefix, matching the brief's `profile registry <path>: ...`).
- File layout is D2 exactly: `{"version":1,"cursor":N,"entries":[{"slug","event":{"cursor","name","profile"|null},"log":[...]}]}`, entries sorted by slug. A `ProfileChange` serializes as `"created"`, `"deleted"`, or `{"updated":{"summary":..}}`; the corrupt-table row for "tombstone last log entry is not Deleted" writes `"created"`.
- A live entry's `event.name` must equal its record's `name` (A finding 5, AC 13 row 7).
- A32: `profile/rename` must answer `not-implemented` even when `profiles.json` is corrupt (it is routed to `rename.rs` without touching the store).
- Update summaries are `pane specs: <before> -> <after> (<a> added, <r> removed, <c> changed)`; specs are matched by `ProfileSpec.pane`.
