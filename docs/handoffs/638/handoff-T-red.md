# Handoff-T-red: Phase 4 - #638 slice a: the fault switch, the fake `PaneStore` and its conformance suite

**Date:** 2026-10-09
**Branch:** issue-638-implementation
**Brief / wireframe reviewed:** docs/handoffs/638-brief.md, docs/handoffs/638/handoff-A.md (no wireframe: no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (Phase 3, `handoff-A.md`, 12 warns, none blocking).

## Tests authored

Two files, staged by explicit path, both starting with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638`.
Both are integration tests in `crates/holler-pane-testkit/tests/`. The crate does not set `autotests = false`, so no `[[test]]`
entry is needed. Tier: integration for all of them (they drive the crate's public API from outside; nothing needs a process
or a hub). No unit tier is cheaper because `src/` does not exist yet and the brief puts these in `tests/`.

### `tests/pane_store_conformance_test.rs` (AC 3 and AC 4)

| Test | Pins |
|---|---|
| `the_fake_passes_the_pane_store_conformance_suite` | AC 3: the fake and the suite agree. `run_pane_store_conformance(\|\| (FakePaneStore::new(), ()))` is `Ok(())`. |
| `the_suite_runs_the_documented_cases` | AC 3: `pane_store_cases()` is the 19 ids of the brief's table, in order. |
| `the_guard_lives_for_the_case` | AC 3: `fresh` is called once per case; no store call happens after its case's guard dropped; every guard is dropped by the end. |
| `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone` | Test quality: the mutant wrapper with nothing broken passes, so each mutant below fails because of its one break and not because of the wrapper. |
| `a_store_without_cas_fails` | AC 4: failures include `stale-generation-conflicts`. |
| `a_store_that_keeps_the_submitted_generation_fails` | AC 4: `submitted-generation-ignored`. |
| `a_store_that_checks_generation_before_existence_fails` | AC 4: `delete-missing-is-pane-not-found`. |
| `a_store_that_repeats_on_resume_fails` | AC 4: `watch-resumes-without-gap-or-repeat`. |
| `a_store_that_ends_the_stream_when_idle_fails` | AC 4: `watch-idle-is-ok-none-and-stays-usable`. |
| `a_store_that_lets_a_pane_change_profile_fails` | AC 4: `pane-in-other-profile`. Every mutant test also asserts each failure carries a non-empty `detail`. |

### `tests/fake_pane_store_test.rs` (AC 5)

| Test | Pins |
|---|---|
| `a_wedged_store_times_out_every_method` | `Fault::Wedged`: `get`, `list`, `cas_put`, `delete`, `watch` return `Timeout { op: "pane_store.<method>" }`, nothing written, works after `set(None)`. |
| `a_wedged_store_ends_an_open_watch` | `next()` yields `Err(Timeout { op: "pane_store.watch_next" })` once, then `None` even after the fault is cleared. |
| `a_corrupt_store_fails_closed_everywhere` | `Fault::Fail(StoreCorrupt)` on every method, `watch` included; earlier records intact after clearing. |
| `fail_next_is_one_shot_and_per_op` | one-shot, only the targeted op, nothing written by the failed call, second call succeeds. |
| `queued_one_shot_errors_come_out_in_order` | FIFO for two queued errors on one op. |
| `a_slow_call_takes_at_least_the_delay` | `set_delay(50 ms)`: lower bound only. |
| `calls_are_recorded_in_order` | `[Get, CasPut(failed), List]`; `concurrent_put` / `concurrent_delete` add nothing. |
| `a_seeded_store_starts_with_an_empty_call_log` | A's W-3: `seeded` bypasses the call log. |
| `port_op_names_are_port_dot_method` | A's W-12: `PortOp::as_str`, all six names. |
| `a_concurrent_put_makes_the_next_cas_stale` | the stale `cas_put` is `generation-conflict`; an open watch yields the concurrent write. |
| `a_concurrent_put_creates_a_record_at_one` | `concurrent_put` on a new name stores generation 1. |
| `a_concurrent_delete_makes_the_record_vanish` | `get` is `None`; `delete` and a second `concurrent_delete` are `pane-not-found`; a watch sees a delete event with `pane: None`. |
| `a_watch_ahead_of_the_head_is_usage` | `watch(Cursor(5))` on an empty store is `usage`. |
| `a_watch_at_the_head_is_idle_not_usage` | the boundary: a cursor equal to the head is accepted and idle. |
| `a_watch_from_zero_resumes_from_the_head_as_of_the_snapshot` | A's W-5: create a, c, b, delete b; `watch(0)` yields puts of a and c only. |
| `the_idle_wait_wakes_on_a_write` | `set_idle_wait(5 s)`, a thread writes after 50 ms, `next()` yields it in under 4 s. |
| `the_default_idle_wait_is_zero` | default `next()` on an idle stream is `Ok(None)` at once (upper bound 2 s). |
| `seeded_stores_hold_each_pane_at_generation_one` | generation 1 each, `list` sorted by name. |
| `seeding_the_same_name_twice_is_a_conflict` | each seed is a create at expected 0 (derived from the brief's "each created at expected 0"). |
| `sample_pane_is_valid_and_deterministic` | equal on two calls; name, generation 0, no profile, no session of record, port 48100, grid `r1c1`; `sample_pane("not a name")` is `usage`. |
| `sample_pane_never_names_a_live_session` | A's W-6: no session/workspace/tmux field starts with `hj`. |
| `the_fake_is_send_and_sync_and_its_watch_is_send` | compile-time bounds (`FakePaneStore`, `FaultSwitch<PaneStoreOp>`, `Watch<PaneEvent>`). |

Not tested here, by design: AC 1, 2, 6, 7, 8 (manifest, layout, blast radius, CHANGELOG, guards) are checked by `cargo machete`,
`cargo tree`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `git diff --name-only` and A's diff review, not by a Rust test.
Case 14's reordered write sequence (W-5) and the one-case-table rule (W-11) live in F's suite code; the suite test only checks
the ids and that the fake passes. The membership/generation order (W-10) is left unpinned, as A left it to O.

## RED confirmation

Command: `cargo test -p holler-pane-testkit` (run in the worktree). Result: neither test binary builds. Every error is a
missing item of the API the brief fixes; there is no typo, type error or setup error:

```
error[E0432]: unresolved import `holler_pane`                       (x2: the manifest has no dependency yet, AC 1)
error[E0433]: cannot find `conformance` in `holler_pane_testkit`    (pane_store_conformance_test.rs:14)
error[E0432]: unresolved import `holler_pane_testkit::pane_store`   (x2: FakePaneStore, PaneStoreOp)
error[E0432]: unresolved import `holler_pane_testkit::fault`        (fake_pane_store_test.rs:12)
error[E0432]: unresolved import `holler_pane_testkit::fixture`      (fake_pane_store_test.rs:13)
error: could not compile `holler-pane-testkit` (test "fake_pane_store_test") due to 4 previous errors
error: could not compile `holler-pane-testkit` (test "pane_store_conformance_test") due to 3 previous errors
```

This is the RED the brief's Test plan prescribes ("they fail to build ... the failure is the missing items"). In Rust a test of
an API that does not exist cannot fail on an assertion first, and T may not add production stubs. Because a resolution error
hides every later type error, I checked the tests a second way, in a throwaway crate outside the repository
(`<scratchpad>/stub`, never committed, not shared with F):

1. A signature-only stub of the brief's API (every method answers `NotImplemented`, the suite returns a one-failure `Err`):
   both files compile and type-check, and the tests then fail on assertions (e.g. `left: Err(NotImplemented) right: Ok(None)`,
   `left: "x" right: "pane_store.get"`). The two bound-check tests pass on the stub, as expected; `the_fake_is_send_and_sync_and_its_watch_is_send` is a compile-time check and cannot be RED at run time.
2. A throwaway working fake (fault switch, fixture, store, no suite): all 22 tests of `fake_pane_store_test.rs` pass. So they are
   satisfiable and pin no behavior the brief does not give.
3. `rustfmt --check --edition 2021` on both files passes; clippy with `-D warnings`, `cognitive_complexity` and `too_many_lines` reports nothing in them. Sizes: 423 and 308 lines (under the 600 warning).

Not exercised against a real suite: the six mutants. They were type-checked and read against the brief's rules, but only F's
suite can show that each fails on its named case. T-green must run them and, if a mutant fails on a different case, fix the mutant (it is a test).

## Ready for F

Confirmed: RED is valid. The failure is exactly the missing items; the tests compile against the brief's API and are satisfiable.
F may implement against these tests. F must follow, in addition to the brief: `PortOp::as_str` (not `name`), `demo-*` pane
names, one `(id, fn)` case table behind `pane_store_cases()`, case 14 reordered (create a, b, update a, create c, delete b),
the watch resuming from the head as of the snapshot, and `seeded` / `concurrent_*` bypassing the call log. The manifest must add
`holler-pane` (the tests import it directly).
