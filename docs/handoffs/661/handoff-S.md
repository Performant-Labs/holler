# Handoff-S: Phase 8 - #661 the hub profile registry (spec audit)

**Date:** 2026-10-09
**Branch:** issue-661-implementation (at 1329d3b; merge base 9d61c9f = origin/main)
**Issue:** #661 (wave 3 of epic #633)
**Brief:** `docs/handoffs/661-brief.md`
**Handoffs reviewed:** `handoff-A.md` (Phase 3), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md`
**Also read:** issue #661 (body and its 2026-10-08 and 2026-10-09 amendments), epic #633 (contract, rulings 1-9, "Decisions 2026-10-09"), ADR-0021 §6-§8 and "Decisions taken"
**Diff audited:** `git diff origin/main...HEAD`, 24 files. I read the whole production diff line by line: `profile/{mod,store,entry,handlers}.rs` in full, plus the three `panes/` edits. I also read all five new test files and the three amended ones. For context I read the unchanged #639 code the change builds on (`panes/{mod,store,persist,feed,handlers}.rs`), the test kit's fake membership rule (`holler-pane-testkit/src/pane_store.rs:219-238`), and `holler_proto::atomic_file`'s temp-file naming.

## A precondition

Met. `handoff-A.md` (Phase 3) is **PASS** with 8 warns and no blocks. `handoff-A-dup.md` (Phase 7) is **PASS** with 4 warns and no blocks.

## T precondition

Met. `handoff-T-green.md` reports **no blocking issues**.

RED came first:
- AC 34-37 failed on assertions before `pane_support` was extended. The pane suite failed exactly one case, `pane-in-other-profile`, so the brief's escalate rule did not apply.
- The other binaries were compile-RED. The only errors were `load_with` and the missing `ProfileStore` impl.
- A reverted stub showed 0/12, 0/8, 0/6 and 0/11 passing, so no profile test passed vacuously.

GREEN followed:
- The workspace run had 1297 passed and 0 failed, with CI's own `--skip roster_stays_accurate_under_concurrent_body_load` (`ci.yml:128`).
- The five new binaries passed 5 times in a row.
- T disabled `refuse_profile_move` and then `check_name`; each change turned tests RED.

Git order: the tests landed in 6a75064 (t-red) before production code landed in 67dee55 (f). F's commit touches no test file.

## Acceptance criteria

All tests are under `crates/holler-hub/tests/`.

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Profile conformance suite, 23 cases | `profile_registry_test.rs::the_registry_passes_the_profile_store_conformance_suite`. It also pins `profile_store_cases().len() == 23`. | MET |
| 2 | Filed by slug; submitted slug ignored | `::profiles_are_filed_by_slug_and_the_submitted_slug_is_ignored`: `slug: "wrong"` is stored as `night-shift`, `get("NIGHT-SHIFT")` returns the record, and the file entry has `"slug": "night-shift"`. | MET |
| 3 | Name rule before generation | `::the_name_rule_runs_before_the_generation`: `NIGHT-SHIFT` at 0 **and at 1** both return `profile-exists`, and the record, log, file bytes and head are unchanged. The test at 1 only passes if the order is right. | MET |
| 4 | Delete logged at gen + 1 | `::a_delete_is_logged_at_the_deleted_generation_plus_one`: `[(1,created),(2,updated),(3,deleted)]`, with actors alice, bob and carol. | MET |
| 5 | Log survives delete, re-create and restart | `::the_log_survives_delete_and_recreate_and_a_restart`: before and after a restart, plus `profile-not-found` for a never-created name even after a refused create at 5. | MET |
| 6 | Hub stamps times (D4) | `::the_hub_stamps_the_times_and_ignores_the_submitted_ones`: create and update windows, `created` kept, `at` never decreasing. | MET (assumes the wall clock does not step back; see advisory) |
| 7 | A refused write changes nothing | `::a_refused_write_changes_nothing_and_logs_nothing`: a 5-row table checking each code plus list, log, file bytes and head. | MET |
| 8 | One-line counting summary (D5) | `::an_update_summary_is_one_line_counting_specs` checks the exact strings `2 -> 2 (1 added, 1 removed, 1 changed)` and `(0, 0, 0)`. `::a_spec_pane_name_holding_a_newline_cannot_reach_the_summary` covers the last sentence. | MET |
| 9 | Log cannot be rewritten | `::the_log_cannot_be_rewritten` (prefix-equal after a 4th write); AC 30 is the wire half. | MET |
| 10 | Records, logs and tombstones survive a restart | `profile_persistence_test.rs::records_logs_and_tombstones_survive_a_restart_unchanged`: env names, `r2c1` read back as `{"row":2,"col":1,...}`, the tombstone, and the next cursor = head + 1. | MET |
| 11 | Atomic, mode 0600 | `::the_file_is_written_atomically_at_mode_0600`: the hub dir is missing at first, then mode `0o600`, the D2 layout and sort order, and no `.profiles.json.*.tmp` left. The filter matches `write_atomic`'s `.<name>.<pid>.<n>.tmp` (`atomic_file.rs:114-123`). | MET |
| 12 | Loading never writes | `::loading_never_writes`: bytes, dir listing **and mtime** are unchanged after load plus get, list, log and watch. | MET |
| 13 | Corrupt file fails closed, never rewritten | `::a_corrupt_file_fails_closed_and_is_never_rewritten` uses 10 rows: the brief's 9 plus A finding 5's live-name row. All six methods answer `store-corrupt` with `what` starting `profile registry <path>`, and bytes and listing are unchanged. F disabled each of `entry::check`'s five checks in turn, and each time this test failed. | MET |
| 14 | Unreadable file (unix) | `::an_unreadable_file_fails_closed_and_is_never_rewritten` (skips under root, as #639 does) | MET |
| 15 | Stored secret, never echoed | `::a_stored_secret_value_fails_closed_without_echoing_it`: neither `what` nor `Display` contains `SENTINEL-661`. The log uses the same `Problem`. | MET |
| 16 | Unwritable dir (unix) | `::an_unwritable_directory_refuses_the_write_and_keeps_the_old_state`: `unavailable` for both writes; get, log and head unchanged; the same write at the same generation succeeds once the dir is writable again. | MET |
| 17 | The two registries fail independently | `::the_two_registries_fail_independently` (port level). The wire `pane/cas_put` half is `profile_handlers_test.rs::a_pane_outside_any_profile_never_reads_the_profile_registry`. | MET (`watch` is not exercised on either side; see advisory) |
| 18 | 16 racing writers, exactly one wins | `profile_registry_test.rs::racing_writers_exactly_one_wins`: Barrier, (1, 15), gen + 1, log + 1, and a reload equal to memory. | MET |
| 19 | Read-modify-write loses no update | `::concurrent_read_modify_write_loses_no_update`: 8 x 25, `soft` +200, gen +200, 200 consecutive `Updated` entries. | MET |
| 20 | Feed delivers each write once, in order | `profile_feed_test.rs::the_feed_delivers_every_write_exactly_once_in_order`. It includes a delete spelled `DAY SHIFT` whose event carries the stored `Day Shift`, and a re-create. | MET |
| 21 | From 0, current state then later changes | `::watch_from_zero_yields_every_current_profile_then_later_changes` (cursor order `[2, 5]`, then `Delta` at 6) | MET |
| 22 | A waiting watch wakes | `::a_waiting_watch_wakes_on_the_next_write`: 20 rounds with a 20 s window and a 10 s `recv_timeout`, no sleep. | MET |
| 23 | Cursor ahead is usage | `::a_cursor_ahead_of_the_store_is_usage` | MET |
| 24 | After a restart, one event per changed profile | `::after_a_restart_an_old_cursor_gets_each_changed_profile_once_including_deletions` | MET |
| 25 | Behind the retained window | `::a_watcher_behind_the_retained_window_gets_the_compacted_changes` (`feed_retained: 2`) | MET |
| 26 | Wire round trip and data shapes | `profile_handlers_test.rs::get_list_cas_put_delete_log_round_trip` and `::a_request_without_params_works_for_list_and_watch` | MET |
| 27 | Bad params answer their code | `::bad_params_answer_their_code`: 7 `usage` rows, `profile-not-found` for delete and log, `generation-conflict` for a stale put and a stale delete, `profile-exists`. | MET |
| 28 | Secret refused over the wire (I7) | `::a_secret_value_is_refused_over_the_wire_and_never_echoed`: `profile-secret-refused` and `env-name-invalid`; the reply line has no sentinel; no file and no log are written. S check: `grep -rnE "'='\|\"=\"\|contains(.=..." crates/holler-hub/src/profile/` finds nothing, so there is no pre-scan. | MET |
| 29 | `profile/watch` returns one batch and its cursor | `::profile_watch_answers_one_batch_and_its_cursor`: parses as `WatchReply<ProfileEvent>`, idle `{"events":[],"cursor":2}`, cursor ahead is `usage`. | MET |
| 30 | Log not rewritable over the wire | `::the_log_cannot_be_rewritten_over_the_wire`: `log` inside `profile` and beside it are both `usage`; log and file bytes unchanged. | MET |
| 31 | Two wire writers racing | `::two_cas_put_requests_racing_exactly_one_wins` (`multi_thread`, tokio Barrier) | MET |
| 32 | Corrupt registry answers `store-corrupt` on every method | `::a_corrupt_registry_answers_store_corrupt_on_every_profile_method`: `outcome` fails on a JSON-RPC error frame; the file and listing are unchanged; `profile/rename` still answers `not-implemented`. | MET |
| 33 | A detached spec is accepted | `::a_detached_spec_is_accepted`: the pane record stays in `Night Shift` at generation 1. | MET |
| 34 | Pane conformance suite, 19 cases | `pane_membership_test.rs::the_hub_pane_registry_passes_the_pane_store_conformance_suite` (pins `len() == 19`). RED failed exactly `pane-in-other-profile`. | MET |
| 35 | Move refused inside the CAS | `::moving_a_pane_to_another_profile_is_refused_inside_the_cas`: port only, so it would fail if the rule were in the hook; `what` names both profiles; record, bytes, listing and head unchanged; leave then join at 2 and 3. `::keeping_the_profile_or_respelling_it_with_the_same_slug_is_allowed` and `::joining_a_profile_from_none_is_allowed` cover the allowed cases. | MET |
| 36 | A stale generation wins | `::a_stale_generation_wins_over_the_profile_rule` (0 and 5). This is an ordering guard: it fails if the rule moves before `next_generation`. | MET |
| 37 | Two joiners: one wins, the loser is refused on retry | `::two_writers_joining_different_profiles_exactly_one_wins_and_the_loser_is_refused_on_retry` | MET |
| 38 | Hook: missing profile is `profile-not-found` | `profile_handlers_test.rs::pane_cas_put_naming_a_missing_profile_is_profile_not_found`: no pane and no `panes.json`; the same request succeeds once the profile exists. | MET |
| 39 | A pane with no profile never reads the profile registry | `::a_pane_outside_any_profile_never_reads_the_profile_registry`: `None` succeeds; a pane naming a profile gets `store-corrupt` and is not stored. | MET |
| 40 | `check_membership` amended | `pane_dispatch_test.rs::check_membership_requires_the_named_profile_to_exist` | MET |
| 41 | Rule placement greps | `grep -n PaneInOtherProfile crates/holler-hub/src/panes/store.rs` has one hit, `:348`, in `refuse_profile_move`, called from `cas_put` at `:175`. `grep -rn PaneInOtherProfile crates/holler-hub/src/profile/` has no hit. | MET |
| 42 | `pane_dispatch_test.rs` amendments | Diff read: `fresh_deps` uses `load_with(.., short_opts())`; the renamed table test expects list `[]`, watch `{"events":[],"cursor":0}`, get, cas_put, delete and log `usage`, rename `not-implemented`; the two-connection list is `Ok(Some([]))`; the direct-dispatch get is `usage` and rename `not-implemented`; the module doc is updated. No `pane/*` assertion changed. | MET |
| 43 | `pane_handlers_test.rs` amendment | +3 lines only: `create_profile(&rig.profiles, "Night Shift")` before the pane write | MET |
| 44 | Build guards | See the next table. | MET |
| 45 | Blast radius | `git diff --name-only` lists only blast-radius paths plus `profile_persistence_test.rs` (T departure 1, a size split). A grep for `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `rename.rs`, `holler-pane/`, `holler-proto/`, `holler-pane-testkit/`, `holler-cli`, `Cargo.*`, `docs/adr`, `docs/protocol`, goldens, `persist.rs` and `feed.rs` finds nothing. | MET |

AC 44 evidence:

| Guard | Evidence |
|---|---|
| Tier 1 commands | T-green table: workspace tests, clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test`, `wire_selftest` |
| Formatting | `rustfmt --edition 2021 --check` rc 0 on every touched `.rs` (F and T) |
| File sizes | `wc -l`: largest file 577 (`profile_handlers_test.rs`); largest production file 415 (`profile/store.rs`) |
| `#[allow]` | Every new `#![allow]` carries `// #661`; there is none in `src/` |
| Catalog, goldens, protocol | `holler-proto`, the goldens and `v2.md` are untouched, so `CATALOG` stays at 22 |
| CHANGELOG | Has the `## [Unreleased]` / `### Enhancements` entry for #661 |

Issue #661's own acceptance list is covered by the rows above:
- the conformance suite: AC 1
- CAS conflict: AC 7 and 27
- restart: AC 5 and 10
- two racers: AC 18 and 31
- corrupt file fails closed with a named error: AC 13, 14 and 32
- append-only log: AC 9 and 30
- the feed delivers each write once: AC 20
- I7: AC 15 and 28
- at most one profile, on `Pane.profile`: AC 35 and 37
- detached spec: AC 33
- grid stored as `GridPos`, never converted: AC 10, and `profile/` has no grid code

## Spec compliance

**Decisions already made (epic, issue, ADR).** All are implemented as stated:
- The hub is a store only. `rename` returns `NotImplemented` through the port and `rename.rs` on the wire, and touches nothing; `rename.rs` is not in the diff.
- Replies are a `PaneReply` in a JSON-RPC result, through #639's `run` and `reply_line`. Watch is long-poll. There is no `v2.md` change.
- `<state dir>/hub/profiles.json` is written by `persist::save_doc`, which means `write_atomic` at 0600, no fsync, and `"version": 1`.
- The log is persisted beside the record in `ProfileEntry`, never in `Profile`. Loading fails closed and leaves the file in place.
- Each log entry is `{at, generation, actor, change}` with the request's `Actor`.
- `EnvVarName` is the only env guard; there is no pre-scan.
- `pane-in-other-profile` runs in the pane CAS under the pane lock. The hook only checks that the profile exists. A detached spec is not refused.
- No new error code; `holler-pane` is untouched.

**Decisions made in the brief:**
- **D1:** one `Mutex` and one `Condvar`. Writes run in the order name rule, `next_generation`, build, `save_doc`, commit, `notify_all`. There is no `.await` under the lock, and the profile store never names `panes::store::Store`.
- **D2:** the layout matches exactly. `RegistryEntry for ProfileEntry` returns (slug, `event.cursor`, record slug and generation), so `persist::check` enforces unique slugs, unique cursors, the cursor range and generation >= 1. `entry::check` adds the event's slug, a non-empty log, the last-log rules, and A finding 5's live-name rule. Its messages are `&'static str`, so no content is echoed. `RegistryEntry for ProfileEvent` is documented as read only through `cursor()` and `record().is_some()`.
- **D3:** `PaneStoreOptions`, `WATCH_WAIT` and the 1024-event ring are reused; there is no second options type.
- **D4:** one stamp per write, `max(now, last log at, live updated)`, used for `at`, `updated` and a create's `created`. This is a disclosed tightening of D4's two clamps: it gives the same values for a live record, because that record's `updated` always equals its last `at`.
- **D5:** the exact format, with specs matched by `ProfileSpec.pane`. It holds counts only.
- **D6:** get, delete and log look up by slug. The name rule checks only against the live record, a delete event carries the stored display name, and `list` is in slug order.
- **D7:** `refuse_profile_move` is called after `next_generation`, compares slugs, and its `what` matches the fake's word for word (`holler-pane-testkit/src/pane_store.rs:223-238`).
- **D8:** `check_membership` returns `Ok` on `None` without reading the registry. Otherwise it calls `store.get(name)?`: `None` gives `ProfileNotFound` and a failed registry gives `store-corrupt`. It runs on the blocking pool before the pane lock, so no lock is nested.
- **D9:** the check is strict.
- **D10:** see deviations below.
- **D11:** only the listed assertions changed.
- **D12:** there is no rename primitive in the store.

**Deviations. Each was disclosed and accepted, none was silent:**
1. **D10's wording.** `mod store` stays private and only `log_fault` is opened, through `pub(crate) use store::log_fault;`. A finding 2 asked for this and handoff-F records it under "Deviations". The pane `Store`'s writes therefore stay behind `PaneState`. It is narrower than the brief and keeps the brief's intent.
2. **CHANGELOG text.** It holds all of the brief's content plus the two `error` events, `unavailable`, `profile-exists`, the env refusal and `rename` (handoff-F).
3. **An extra fail-closed condition.** A live record's log must not end with `Deleted`. The registry never writes that, so the check cannot refuse a file the registry wrote (handoff-F).
4. **Test layout.** T's fifth test file is `profile_persistence_test.rs`, split out to stay under the 600-line lint warn. T also made `drain` generic and added `head_by`, per A finding 3, with no #639 test touched.

The brief has no contradiction that F had to work around, so there is no ADVISORY-HOLD.

## Quality audit

**Correctness and failure handling:**
- A refused write builds nothing and takes no cursor.
- A failed save returns `unavailable` before the commit, so memory, file and head stay unchanged (AC 16).
- A failed load leaves the store as `Err(StoreCorrupt)` for the life of the process. `log_fault("profile_registry_corrupt")` is called once at load, and nothing ever writes the file (AC 13 and 14).
- The serde errors raised by `try_from` and `EnvVarName` reach the client only as `Problem::parse` category, line and column (`persist.rs:98-112`), so I7 holds at rest.
- Concurrency is one lock with save before commit; no update is lost (AC 18, 19, 31, 37).
- Lock order matches ADR-0021 §7: the hook takes and releases the profile lock before the pane CAS takes the pane lock.
- Poisoning is handled as in #639 (`unwrap_or_else(PoisonError::into_inner)`).
- The real hub loads the registry once through the unchanged `PaneDeps::load` (`pane_dispatch.rs:63`, `serve.rs:366`).

**Build guards:** the production files have no `unwrap(`, `expect(`, `panic!`, `unreachable!`, `todo!`, `dbg!` or `println!`, and no `unsafe` (grep over `src/profile/*.rs` and `src/panes/*.rs`). There is no `#[allow]` in `src/`. Clippy `-D warnings` is clean, so there is no dead code. No file is near 900 lines; the largest is 577.

**Protocol:** no change. This is correct: ruling 6 puts `profile/*` outside the catalog.

**Tests:**
- The tiers are the cheapest that suffice: store tests go through the port from threads, and handler tests call `profile::dispatch` or `panes::dispatch` directly.
- No new behaviour crosses processes. There is no CLI client until #649, and #669's tests already pin the socket forwarding.
- There is no `thread::sleep`; every wait is bounded.
- The concurrency tests assert invariants.
- RED-first evidence is in handoff-T-red.
- Mutation spot checks are in handoff-T-green (`refuse_profile_move`, `check_name`) and handoff-F (the five `entry::check` conditions).
- The three tests that pass without the change were disclosed. They pin allowed behaviour and the check order, so each still fails on a wrong implementation.

**Documentation:**
- `CHANGELOG.md` has an `## [Unreleased]` / `### Enhancements` entry linking #661 and #633. It also names both new `error` events.
- Module docs cover the layout, rules, operating limits and lock order.
- **Gap (advisory, see below):** the README "Debug output" section does not list `profile_registry_corrupt` or `profile_registry_write_failed`.

**Public-repository privacy:** I grepped the diff's added lines for IPv4 addresses, `ts.net`, tailnet and tailscale, personal and account names, email domains, `/home/` and `/Users/`, `.local`, `kiwi`, and key or secret patterns. The only hit was a false positive (`sk-` inside `dusk-shift`). The test names are neutral (`Night Shift`, `alice`, `bob`, `carol`, `mallory`, `t661`, `SENTINEL-661`). No secret is logged: the events carry only the path, the category or position problem, and the effect.

**Commit and PR hygiene:**
- All six subjects are Conventional Commits that the repo's `.githooks/commit-msg` regex accepts (`chore(#661): …`, `docs(handoffs): …`).
- The author and committer are the GitHub no-reply address.
- The trailers are `Co-Authored-By: Claude <noreply@anthropic.com>` with **no session link**. These are the Workflow script's own commits; CLAUDE.md records that the script's commits lack the repo's disclosure.
- No PR exists yet, so the `CONTRIBUTING.md` AI disclosure must be added with `gh pr edit` after the script opens it (advisory).

## Scope check

F delivered the brief's scope exactly:
- the profile registry: storage, CAS, change log, feed and the six handlers;
- the D7 comparison;
- the D8 hook;
- the D10 openings, made narrower as A asked;
- doc-line fixes in `panes/mod.rs` and `panes/store.rs` (A finding 7), inside the blast radius.

Nothing was under-delivered, and there is no unrelated refactor or extra feature. The `profile/` production files total 909 lines against the brief's estimate of about 800, and none is over 450. Accepted decompositions:
- `profile_persistence_test.rs`, split out for the 600-line lint warn (T departure 1);
- the generic `drain`/`head_by` in `pane_support` (A finding 3).

## Verdict

**PASS**

All 45 acceptance criteria and the issue's own acceptance list have proving tests that assert behaviour, not implementation. Every brief and epic decision is implemented as stated. The four deviations are disclosed, and each is narrower than or equal to the brief's intent. Build guards, fail-closed handling, concurrency, privacy and scope are clean.

## Advisory notes

None of these block the merge. Items 1-3 are pre-merge actions for the run's merging agent or the operator, carried from A and A-dup. F cannot do them.

1. **Open the store-generalisation follow-up issue and link it from the PR** (A Phase 3 finding 1, A-dup finding 2). Its scope:
   - lift `RegistryEntry`, `persist`, `feed`, `run`/`NoParams`, `log_fault` and the options into a neutral module;
   - make the store generic, collapsing the 13 "Twin of `panes::store::…`" items and `WRITE_EFFECT`;
   - fix the stale doc line `panes/persist.rs:17-21`, which says `profiles.json` holds `ProfileEvent` entries when they are `ProfileEntry`;
   - put the new issue's number on the "Twin of" lines;
   - fold the test-side copies into `pane_support`: `pname` x3, `who` x2, and the profile handler `Rig` (A-dup findings 3-4).
2. **Post the cross-story notes** (A finding 8) on #647, #662, #663 and #664:
   - (a) Under D9, a pane that names a deleted profile can be rewritten only with `profile: None`. So #662's `delete --keep-panes` must clear `Pane.profile`, and #647's reconcile must handle `profile-not-found`.
   - (b) Membership compares slugs, so a profile's panes must be selected by `ProfileName::slug()`, never by `==` on the name.

   The PR description should also correct the brief's Risks line "reconcile is the net" to say it holds only if #647 handles that code.
3. **PR disclosure:** after the script opens the PR, add the `CONTRIBUTING.md` AI disclosure with `gh pr edit`, and make sure the squash commit carries the `Co-Authored-By` trailer. The script's commits have no session link.
4. **README "Debug output" gap.** The F overlay says to update README and `docs/` for a new log event, but `profile_registry_corrupt` and `profile_registry_write_failed` appear only in `CHANGELOG.md` and the module docs. I did not make this REWORK or a HOLD, for three reasons:
   - The brief's blast radius (AC 45) excludes README, so F could not edit it without breaking an AC.
   - #639 merged with the identical gap for `pane_registry_corrupt` and `pane_registry_write_failed`.
   - No CLI path writes `profiles.json` until #649 and #662.

   Recommendation: one small docs follow-up, or fold it into item 1. It would add all four registry fault events to README's "Debug output" section, as a short table beside the authentication events, and widen the `control` component row ("Requests received on …") to mention registry faults. The operator may overrule this and hold for it; my reasoning is recorded here so that choice is easy.
5. **AC 17 completeness.** `the_two_registries_fail_independently` does not exercise `watch` on either side ("every method"). This is not material: each registry's watch reads only its own table, and a cross-file mix-up would already fail the get, list and log calls the test makes. Add the two `watch` calls when the file is next touched.
6. **AC 6** uses real `now_millis()` windows, so it assumes the wall clock does not step backwards mid-test (noted by T). The production stamp is safe either way (D4 clamp).
7. **Operator review flags** D4, D9, D10 and D12 still stand, as the brief says.
8. **Cosmetic:** `profile/mod.rs:66-68` (carried from #669's stub) says its `mod.rs` root is one "which no other hub module is". `panes/mod.rs` is a `mod.rs` root too, and its own docs say the same.
