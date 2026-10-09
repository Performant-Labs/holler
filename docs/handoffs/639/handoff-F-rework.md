# Handoff-F: Phase 6 (rework round 1) - #639 the hub pane registry

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Issue:** #639
**Rework input:** the outside diff gate's round-1 review of HEAD 6654971 (`docs/handoffs/639-diff-result-r1.md`, deepseek-v4-pro,
gitignored). Its verdict was "BLOCK: B-1 and W-2".

## What was done

This is a rework round, not a fresh implementation pass. Neither blocking finding holds: each describes behaviour the code
does not have, and the proof is below. No logic changed. The one production edit is a set of doc comments that state, at the
lines the reviewer misread, the two invariants it got wrong. The next diff-gate round sees the diff and `evidence.md`, but not
this handoff, so that is where the explanation has to be.

- `crates/holler-hub/src/panes/store.rs` (doc comments only):
  - The module docs now say that the watchers wait without holding the lock, and point to `Store::poll`.
  - `Store::poll`'s docs say that `Condvar::wait_timeout` releases the lock for the wait, and name the test that pins this.
  - They also say that `feed::select` re-checks rule 1 on every pass, because `pane/watch` passes the client's cursor straight
    to `poll`, and that a cursor that passed once never fails later, because the head only grows.
- `docs/handoffs/639/evidence.md` (two entries): the std `Condvar` text, quoted verbatim, and `WatchParams`/`Cursor`, which show
  that any `u64` decodes as `since`.
- `docs/handoffs/639/decisions.md`: this round's entry.

## Finding by finding

| Finding | Verdict | Proof |
|---|---|---|
| **B-1** (and W-1, W-4, which say the same thing): `poll` keeps the table mutex locked across the long-poll wait, so one `pane/watch` blocks `get`, `list`, `cas_put` and `delete` for the whole window | **False positive** | std: `wait_timeout` "will atomically unlock the mutex" and re-acquires it on return (see `evidence.md`). The reviewer's own B-4 says the same thing ("atomically drops the guard and re-acquires it ... That is correct pattern"). The suite already pins this: `a_waiting_watch_wakes_on_the_next_write` (`pane_feed_test.rs:128-154`) parks a watcher in a 20 s window, writes, and needs the event within 10 s. **Mutation M1** made B-1 true: `thread::sleep(left)` under the guard instead of `wait_timeout`. That test then **FAILED** (`idle`, after 20.00 s). **Probe** (10 rounds, a watcher parked in a 30 s window): `get` 31-39 µs, `list` 9-20 µs, a second watch 16-20 µs, `cas_put` 135-178 µs, and the parked watcher woke at the write. Under M1, the same `get` blocked for 2.70 s of a 3 s window. |
| **W-2**: `poll`'s second `check_since` can answer `usage` for a validly created iterator if a concurrent write advances the head | **False positive** | The head only grows. After load, `commit` (`store.rs:206`) is the only place it is assigned, always to `next_cursor()`, which is head + 1 with an overflow check. So once `since <= head` holds, it keeps holding: a concurrent write puts `since` further behind the head, never ahead of it. The iterator's next `since` is the reply's cursor, which is the head at answer time. **Probe**: one iterator polled while 4 writers raced 250 CAS writes each. It got 1000 events with consecutive cursors 5..=1004, and no error, repeat or skip. |
| W-2's remediation: answer an empty batch for `since > head` in `poll`, and drop the check in `select` | **Rejected: it breaks D6 rule 1 on the wire** | `handlers::watch` passes `params.since` straight to `Store::poll`, and `WatchParams` accepts any `u64` (`evidence.md`). **Mutation M2** applied the remediation literally. `pane/watch {since: head + 1}` then answered `{"cursor": 1, "events": []}` instead of `usage`, which hands the client a cursor lower than the one it sent. |
| NV-1 (the `Watch` iterator is `Send`) | Agreed | The guard is local to `poll`, and `Feed` holds only `Arc<Store>`, a `Cursor` and a `VecDeque`. |
| W-3 | No issue (the reviewer's own conclusion) | |
| NIT-1 (`checked_add` fallback) | No change | Deliberate: a huge window cannot panic (handoff-F, "The deadline"). |
| NIT-2 (no timeout on `spawn_blocking`) | No change | This is a documented operating limit (`mod.rs`, "Operating limits"): a wedged save blocks every caller with no `timeout`, and the client's own timeout bounds the verb. |
| NIT-3 | No issue (the reviewer's own conclusion) | |

## Design decisions

- **Document, do not restructure.** The reviewer offered two alternatives: a separate mutex and `Condvar` for the watchers, or
  `tokio::sync::Notify`. Both would fix a problem the code does not have, and both contradict D1 ("One `std::sync::Mutex`
  guards the whole table, with one `Condvar` for watchers"), which A passed. A second lock also reopens a lost-wake-up window
  between a watcher's check and its wait: a write landing in that gap notifies nobody, unless a change counter is added under
  the second lock. One lock closes that window by construction.
- **The doc comment names the test file**, as `token.rs:102`, `holds.rs:18` and `circuit/dispatch.rs:79` already do. It does
  not give a line number, which would go stale.

## Reuse / extend-vs-new

No new object. This round edits only the doc comments of `Store`, which this story added.

## Architecture notes for A

None. No layer, interface, dependency or behaviour changed.

## Deviations from spec / wireframe

None. D1 and D6 stand as written.

## Tier 1 self-check (incl. tests now GREEN)

These ran on the final tree, after the probe file had been deleted:
```
cargo build --workspace                                     exit=0
cargo clippy --workspace --all-targets -- -D warnings       exit=0  (no warning, no error)
cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
                                                            exit=0  102 test binaries: 1068 passed, 0 failed, 5 ignored
  pane_dispatch_test 10 passed, pane_feed_test 7 passed, pane_handlers_test 7 passed, pane_registry_test 14 passed
cargo machete                                               exit=0
bash scripts/lint.sh                                        exit=0  (no line names a panes/ file)
bash scripts/changelog-check.sh                             exit=0  "changelog-check: ok"
rustfmt --check --edition 2021 crates/holler-hub/src/panes/*.rs and the pane test files
                                                            exit=0
cargo doc -p holler-hub --no-deps                           no warning in panes/ (with or without --document-private-items)
```
`store.rs` is 352 lines.

**The probes.** They ran in a throwaway test file that was never staged and is now deleted. It used `thread::sleep` only to
let a watcher park, which a delivered test may not do. Both mutations were reverted with `git checkout`, and
`grep -rn MUTATION crates/` finds nothing.

## Evidence appendix

`docs/handoffs/639/evidence.md` now has 14 entries; this round added two. A script checked every new excerpt line against its
cited lines and found 0 mismatches. The std lines were checked against the toolchain's own rendered source (Rust 1.98.1). The
file is 9,675 bytes, under the 12,000-byte appendix cap in `dual-review.sh`. The gate cannot attach the std file, because it
refuses paths outside the repo, so the reviewer sees that entry as an author's claim. The gate's own rules make an unattached
claim a needs-verification finding at most, never a BLOCK.

## Tests that look wrong (for T)

None are wrong. There is one **coverage gap**. No delivered test pins D6 rule 1 on the `pane/watch` wire path. AC 20's
`a_cursor_ahead_of_the_store_is_usage` goes through `PaneStore::watch`, which refuses the cursor in `Store::watch` before
`poll` ever runs. With W-2's remediation applied (M2), all 38 pane tests still passed while `pane/watch {since: head + 1}`
answered an empty batch. A handler test that sends `pane/watch` with `since = head + 1` and expects `usage` at once would
pin it. Whether to add it is T's decision.

## Known issues

- None against the acceptance criteria.
- If the outside gate raises B-1 again, it is not converging on a false positive. The std text, a delivered test, a mutation
  that kills that test and the timings above all settle the question. The operator should rule on it rather than loop F.

## Files changed

- `crates/holler-hub/src/panes/store.rs` (doc comments only)

Pipeline artifacts, not production files: `docs/handoffs/639/handoff-F-rework.md`, two entries appended to
`docs/handoffs/639/evidence.md`, and one entry appended to `docs/handoffs/639/decisions.md`.
