# Handoff-F: Phase 5 - #639 the hub pane registry

**Date:** 2026-10-09
**Branch:** issue-639-implementation
**Issue:** #639

## What was done

All production code is in `crates/holler-hub/src/panes/`, the issue's radius. It totals 994 lines, and no file is over 340.

- `mod.rs` (edited; previously the #669 stub, now 202 lines):
  - Module docs: what the registry is, its four files, the operating limits (A's W-10 b and c), and why `PaneState` holds an
    `Arc` of its own.
  - `pub struct PaneState { store: Arc<Store> }`, which still does not derive `Clone`, with `load` and `load_with`.
  - `pub struct PaneStoreOptions { watch_wait, feed_retained }` with `Default`.
  - `pub const WATCH_WAIT` (4 s), whose doc states the client-timeout rule (W-10 a).
  - `impl PaneStore for PaneState`, which delegates to `Store`.
  - The crate-private `RegistryEntry` trait, with its impl for `PaneEvent`.
  - `dispatch`, which routes the five methods to `handlers`.
- `store.rs` (new, 340 lines):
  - `Store`: `Mutex<Result<Table, PaneError>>`, a `Condvar`, the path and the options.
  - `Table`: `{ head, entries: BTreeMap<PaneName, PaneEvent>, ring }`.
  - `get`, `list`, `cas_put` and `delete`. A write's `commit` saves first, then changes memory, then notifies.
  - `Store::watch` and `Store::poll` (the `Condvar` long-poll), and the `Feed` iterator, which is the `Watch`.
  - The fail-closed load, and `log_fault`, which emits the two `error` events.
- `persist.rs` (new, 194 lines):
  - The generic `Doc<E>`, `load_doc<E>` and `save_doc<E>`.
  - `Problem`, the D3 reason, with `Problem::what`.
  - The D2 entry checks, plus the check that entry cursors are distinct.
- `feed.rs` (new, 127 lines): `Ring<E>` and its floor, `check_since` (rule 1) and `select` (rules 1-4). The file is pure: no
  lock and no wait.
- `handlers.rs` (new, 131 lines): the generic runner `run` (params, decode, `spawn_blocking`, `PaneReply`, `reply_line`), five
  handlers of a few lines each, and `check_membership` called inside `pane/cas_put`'s blocking closure.
- `CHANGELOG.md`: an `## [Unreleased]` / `### Enhancements` entry that links #639. It names the file, the fail-closed behaviour,
  the two new `error` events, and the `store-corrupt` result of a file from a newer build.

## Design decisions

- **The next document is built without touching memory (D1).**
  - `Table::entries_with` builds a borrowed view (`Vec<&PaneEvent>`) of the current entries, with the change substituted
    or inserted in name order. The save serializes that view, and memory is changed only after the save succeeds.
  - I rejected two alternatives. Changing memory first and rolling it back on failure reverses D1's stated order, and a
    partial rollback is a new failure mode. Cloning the map would copy every record on every write.
- **The failed state is the `Err` arm of the mutex's content.**
  - `Mutex<Result<Table, PaneError>>` makes the table unreachable once the load has failed, so no method can bypass the
    failure.
  - Every method, `watch` and `pane/watch` included, answers the same `StoreCorrupt`. Nothing in that state can write, so
    the file is never rewritten or moved.
- **The version is read before the full parse.**
  - A file written by a newer build reports `version N is not one this hub reads (it reads version 1)`, instead of a data
    error at a line and column.
  - The full parse still applies `deny_unknown_fields`.
- **The problem text (D3)** is built only from:
  - the serde `Category` and its line and column (never serde's message);
  - the version number;
  - a validated pane name (for a duplicate, a mismatch, a cursor out of range, or generation 0);
  - the io `ErrorKind` on read, and the io error on a failed write.

  The fail-closed tests, run with `--nocapture`, show each of AC 9's six cases failing on its own check (see Tier 1).
- **Rule 2 wins over rule 3.** `since == 0` always yields the snapshot of live records, even when the ring still holds the
  whole history. This is the port's "current state first", and AC 16 pins it.
- **The ring's floor.** After a load, the floor is the loaded head. When an event is evicted, the floor becomes that event's
  cursor. With a ring of 0 the floor tracks the head, so a stale watcher gets rule 4 and nothing is lost.
- **The idle reply's cursor is the head (D6).** In every idle case except one, the head equals `since`. The exception is
  `since = 0` against a registry whose records are all deleted. There the reply is `{"events": [], "cursor": <head>}`, as D6
  says ("always head"). The draft ADR-0021 §6 sentence says an idle window answers `cursor: since`. Both resume correctly.
  I followed the brief.
- **The `Watch` iterator.** After an error, `Feed` drops its `Arc<Store>`, so the stream ends with `None` ("any error ends the
  stream"). Each poll's reply cursor becomes the next poll's `since`.
- **The deadline.** It is `Instant::now().checked_add(window)`, and the time left is recomputed after every wake-up, so a
  spurious wake-up never extends the window and a huge window cannot panic.
- **`pane/list` decodes into an empty `deny_unknown_fields` struct**, so unknown params are `usage`, as for every params struct
  in `reply.rs`. An absent `params` or `null` is `{}`.
- **An unknown method** in `dispatch` answers `not-implemented`. `forward` only sends the methods of `PANE_METHODS`, so this
  arm covers only a method added to that list before its handler exists.
- **Logging (A's W-3).**
  - There is one private helper, `log_fault`, for exactly two events: `pane_registry_corrupt` at load and
    `pane_registry_write_failed` on a refused save.
  - Both events are `Severity::Error`, a fault on the hub's own side (`log.rs:114-115`). Both use `Component::Control` and
    `Direction::Local`, and the same three fields (`path`, `problem`, `effect`).
  - It is not a generic `emit(severity, ..)` copy: the severity and the field set are fixed. A one-function-per-event
    pattern would have repeated the same `Event` literal twice.
  - D4 left open whether a failed save is logged. I log it, as `holds.rs` does (`hold_state_write_failed`), because an
    unwritable state dir silently degrades every pane verb otherwise.

## Reuse / extend-vs-new

- **Extended:**
  - the #669 stub module itself: `PaneState` and `dispatch` are the stub's own items, filled in place;
  - `holler_proto::atomic_file::write_atomic` for the save (at `0600`);
  - `holler_pane::next_generation` for both `cas_put` and `delete`;
  - `decode_params` and the `reply.rs` params structs (`PaneGetParams`, `PaneCasPutParams`, `PaneDeleteParams`,
    `WatchParams`);
  - `PaneReply::success`/`failure` through `crate::pane_dispatch::reply_line`;
  - `lock().unwrap_or_else(PoisonError::into_inner)` (`holds.rs:293`);
  - the `holds.rs` event shape (`Component::Control`, `Direction::Local`).
- **Reused rather than copied (A's W-5):** the frozen `PaneEvent` is both the file's entry and the in-memory entry. There is no
  new `Entry { cursor, pane }` struct.
- **New objects:** the brief's own file split (`store.rs`, `persist.rs`, `feed.rs`, `handlers.rs`) and the objects it names
  (`Store`, `Table`, `Ring<E>`, `Doc<E>`, `PaneStoreOptions`). Two additions:
  - `Problem` replaces the brief's `corrupt_what(path, problem)` (see Deviations, item 4).
  - `RegistryEntry` is the one trait through which `persist`'s checks and `feed`'s rules read an entry, so #661 reuses both
    with `ProfileEvent`.
- **What the diff does not contain:** a second CAS helper, a second code validator, a copy of `sample_pane`, or a new error
  code.

## Architecture notes for A

- **Layers.** Only the hub's `panes` module changed. New submodules: `store` (private), and `persist`, `feed` and `handlers`
  (all `pub(crate)`, so #661 can call `persist::{load_doc, save_doc}`, `feed::{Ring, select}` and `handlers::run` from
  `profile/`).
- **Dependency direction** (A's W-4):
  - `mod` depends on `store` and `handlers`.
  - `store` depends on `feed` and `persist`.
  - `feed` and `persist` depend only on the parent's `RegistryEntry` trait.
  - `handlers` depends on `store`, `crate::pane_dispatch::reply_line` and `crate::profile::check_membership`.
  - `feed` never reaches back into the store: the `Condvar` loop and the iterator are in `store.rs`.
- **Public interface:**
  - `PaneState` gains `load_with` and `impl PaneStore`.
  - New: `pub struct PaneStoreOptions` and `pub const WATCH_WAIT`.
  - `panes::dispatch`'s `profiles` parameter goes from `&ProfileState` to `&Arc<ProfileState>` (Deviations, item 1). Its only
    production caller, `forward`, already passes `&Arc`, so `pane_dispatch.rs` is unchanged.
- **No other changes:** no new dependency or manifest change, no wire or protocol change, no golden file, and no new error
  code. The registry uses the closed variants `Conflict`, `PaneNotFound`, `StoreCorrupt`, `Unavailable`, `Usage` and
  `NotImplemented`.
- **New state file:** `<state dir>/hub/panes.json`, mode `0600`, written atomically. New `error` events:
  `pane_registry_corrupt` and `pane_registry_write_failed`.
- **Concurrency:**
  - One mutex guards the table, and the lock is held across the save (D1).
  - The pane lock is never held while calling into `ProfileState`: `check_membership` runs first in the blocking closure,
    before `Store::cas_put` takes the lock.
  - Every store call from async code goes through `spawn_blocking`, and a `JoinError` is `unavailable`.
  - A pending `pane/watch` holds one blocking thread for up to 4 s.

## Deviations from spec / wireframe

1. **`dispatch(.., profiles: &Arc<ProfileState>)`**, where the brief's Files section says "The `dispatch` signature is
   unchanged".
   - This is A's W-1, routed to F. `check_membership` runs inside the `spawn_blocking` closure, before the CAS and outside
     the pane lock, so #661's lookup through the blocking `ProfileStore` port never runs on an executor thread.
   - `pane_dispatch.rs:71-74` sanctions the widening. No caller changed, the #669 tests compile unchanged, and AC 27's grep
     still matches (`handlers.rs`).
2. **`Store::poll` and the `Watch` iterator are in `store.rs`, not `feed.rs`** (A's W-4). `feed.rs` holds `Ring<E>` and the
   rule selection as a pure function.
3. **The file entry is `PaneEvent`** (A's W-5), not a separate `{name, cursor, pane?}` struct. The JSON shape is D2's, with one
   detail: a tombstone is written as `"pane": null` rather than with the member left out, because the frozen type has no
   `skip_serializing_if`. Both forms read back.
4. **`load_doc` and `save_doc` return a `Problem`, not a `PaneError`.**
   - The store logs the bare problem in the event's `problem` field and builds the error's `what` with
     `Problem::what(label, path)`, which replaces the brief's `corrupt_what(path, problem)`.
   - The registry label is a parameter, so #661 can say "profile registry" (W-5 b).
   - The `what` is still `pane registry <path>: <problem>` for a corrupt file and `pane registry <path>: not written: <io
     error>` for a refused write (D3, D4).
5. **D2's validation also requires distinct entry cursors** (A's W-5 d). T's sixth AC 9 case covers it.
6. **D4 logs** `pane_registry_write_failed` (see Design decisions, Logging).

## Tier 1 self-check (incl. tests now GREEN)

All of these ran on the final tree, after the last edit. Timestamps are omitted.

**T's tests now GREEN.** These are the four binaries' lines from the final `cargo test --workspace` run below. The targeted
`cargo test -p holler-hub --no-fail-fast --test pane_registry_test --test pane_feed_test --test pane_handlers_test --test pane_dispatch_test`
gave the same result earlier.
```
running 10 tests   (pane_dispatch_test)   test result: ok. 10 passed; 0 failed
running 7 tests    (pane_feed_test)       test result: ok. 7 passed; 0 failed
running 6 tests    (pane_handlers_test)   test result: ok. 6 passed; 0 failed
running 14 tests   (pane_registry_test)   test result: ok. 14 passed; 0 failed
```
All 30 new and amended tests are GREEN, and the 7 unchanged #669 tests still pass. I edited no test. Flakiness check: 25
repeat runs of each of the four binaries, 100 runs in all, with 0 failures. The repeats include `racing_writers_exactly_one_wins`,
`concurrent_read_modify_write_loses_no_update`, `a_waiting_watch_wakes_on_the_next_write` and
`two_cas_put_requests_racing_exactly_one_wins`.

**Each fail-closed case fails on its own check.** These are the `problem` fields of the events the tests log (`--nocapture`),
with the temp path shortened:
```
pane_registry_corrupt path=<tmp>/hub/panes.json problem=not valid JSON at line 1, column 2
pane_registry_corrupt ... problem=version 2 is not one this hub reads (it reads version 1)
pane_registry_corrupt ... problem=a value this hub cannot read at line 72, column 18          (unknown field in a record)
pane_registry_corrupt ... problem=hj-c1r1 has more than one entry
pane_registry_corrupt ... problem=the entry hj-c1r2 holds the record of hj-c1r1
pane_registry_corrupt ... problem=more than one entry has cursor 1                            (A's W-5 case)
pane_registry_corrupt ... problem=cannot be read: permission denied                           (0o000 file)
pane_registry_write_failed ... problem=not written: Permission denied (os error 13)            (0o500 hub dir)
pane_registry_corrupt ... problem=a value this hub cannot read at line 25, column 32          (SENTINEL as a wrong-typed value)
pane_registry_corrupt ... problem=a value this hub cannot read at line 8, column 22           (SENTINEL as an unknown field name)
pane_registry_corrupt ... problem=a value this hub cannot read at line 75, column 27          (SENTINEL as the version)
```

**AC 29 build guards:**
```
cargo build --workspace                                     exit=0
cargo clippy --workspace --all-targets -- -D warnings       exit=0  (0 warnings, 0 errors)
cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
                                                            exit=0  97 test binaries: 964 passed, 0 failed, 5 ignored
cargo machete                                               exit=0  "didn't find any unused dependencies"
bash scripts/lint.sh                                        exit=0  (no `lint:` failure; no warning names a panes/ file)
bash scripts/changelog-check.sh                             exit=0  "changelog-check: ok"
rustfmt --check --edition 2021 crates/holler-hub/src/panes/*.rs and the five pane test files
                                                            exit=0
```
The `--skip` is the one CI applies (`ci.yml:128`).

- **File sizes:** `panes/` is 994 lines in total: `store.rs` 340, `mod.rs` 202, `persist.rs` 194, `handlers.rs` 131,
  `feed.rs` 127. None reaches 600.
- **`#[allow]`:** there is none in `panes/`.
- **Unchanged:** `crates/holler-proto` and `crates/holler-pane`, so `CATALOG` and the golden files are as they were.
- **`git diff --name-only origin/main...HEAD` plus the uncommitted paths:**
  - `crates/holler-hub/src/panes/**`
  - `CHANGELOG.md`
  - `docs/handoffs/639/*`
  - T's five test files

  `pane_feed_test.rs` still needs O's radius approval (T's W-9).
- **rustdoc** (`cargo doc -p holler-hub --no-deps`) gives no warning for `panes/`. The 19 warnings it does print are all in
  existing modules.

## Evidence appendix

`docs/handoffs/639/evidence.md`: 12 entries, each with a `file:line` source and a verbatim excerpt. Every quoted line was
checked against its cited range by a script, with 0 mismatches.

## Tests that look wrong (for T)

None. All 30 new and amended tests passed on the first run against this implementation.

## Known issues

None against the acceptance criteria. For the record:

- AC 27 has no behavioural test, by design (`check_membership` is `Ok(())` until #661). S verifies it with
  `grep -n "check_membership(" crates/holler-hub/src/panes/`, which matches `handlers.rs`.
- A's W-7 b is for O and the operator, not for this story. #665's `profile/rename` needs a multi-record write under one pane
  lock, and this store offers only single-record writes. #665 will need `panes/store.rs` in its radius.

## Files changed

- `crates/holler-hub/src/panes/mod.rs`
- `crates/holler-hub/src/panes/store.rs`
- `crates/holler-hub/src/panes/persist.rs`
- `crates/holler-hub/src/panes/feed.rs`
- `crates/holler-hub/src/panes/handlers.rs`
- `CHANGELOG.md`

Pipeline artifacts, not production files: `docs/handoffs/639/handoff-F.md`, `docs/handoffs/639/evidence.md`, and an appended
entry in `docs/handoffs/639/decisions.md`.
