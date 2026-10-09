# Brief: #639 the hub pane registry (persistent, generation-fenced, with the `pane/*` handlers)

Repo: Performant-Labs/holler. Issue: #639 (wave 2 of epic #633; depends on #637, merged at `f2602ba`, and #669, merged at
`af3d8df`). Rigor: second-opinion. UI surface: no. Kind: feature.

**Branch:** `issue-639-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633,
its "Skeleton split" rulings 1-9, and the merged `holler-pane` crate are fixed. The issue text of #639 is the source of truth
together with the epic. Where the merged code and the prose differ, this brief follows the merged code and says so under
"Decisions made in this brief".

**Size check:** fits one run. About 5 production files under `panes/` (roughly 900 lines in total, none over 400), 2 new test
files, a shared test helper, and a 3-assertion edit to #669's test. If the architecture review blocks on size, split it as
follows: 639a is the store, persistence and `get`/`list`/`cas_put`/`delete`; 639b is the change feed and `pane/watch`. The
seam is `feed.rs`.

**Needs operator:** none. One cross-story consequence is recorded as D9 (for #661). It does not block this story.

## Problem

The hub routes `pane/*` to `panes::dispatch`, which still answers every method with `not-implemented`. `PaneState::load`
returns empty state and keeps nothing. Wave-3 stories (#643, #644-#647, #650 and #649's client) need a real registry with
these properties:

- one record per pane, kept in a file under the state dir, so it survives a hub restart;
- every write is a compare-and-swap on `generation`;
- a change feed for `pane/watch`;
- a corrupt file fails closed instead of being reset.

## Evidence (verbatim, as of `af3d8df`)

The stub this story fills (the whole module today):
```
crates/holler-hub/src/panes/mod.rs:27-51
pub struct PaneState;
impl PaneState {
    pub fn load(_state: &HubState) -> Self { Self }
}
pub async fn dispatch(_method: &str, cid: &CorrelationId, _obj: &Value, _panes: &PaneState, _profiles: &ProfileState) -> String {
    reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented))
}
```
How the stub is wired in (no edit is needed in any of these places). `load` is infallible and runs once, before the first
connection. Each connection clones the `Arc` bundle. `forward` receives the **whole frame** as `obj`, not its `params`:
```
crates/holler-hub/src/pane_dispatch.rs:50-66   pub struct PaneDeps { pub panes: Arc<PaneState>, pub profiles: Arc<ProfileState> }
                                               PaneDeps::load(state) = Arc::new(PaneState::load(state)), Arc::new(ProfileState::load(state))
crates/holler-hub/src/pane_dispatch.rs:75-94   forward(method, cid, obj, deps): is_pane_method -> panes::dispatch(method, cid, obj, &deps.panes, &deps.profiles).await
crates/holler-hub/src/pane_dispatch.rs:98-101  pub(crate) fn reply_line(cid, reply: &PaneReply) -> String   // the one place a reply becomes a line
crates/holler-hub/src/serve.rs:366             let pane_deps = crate::pane_dispatch::PaneDeps::load(state);   // in build_shared_state, beside Holds::load (360)
crates/holler-hub/src/serve.rs:85              ensure_dirs(&state)   // <root>/hub exists before build_shared_state in a real hub
crates/holler-hub/src/control_server.rs:117    Some(m) if is_pane_method(m) || is_profile_method(m) => crate::pane_dispatch::forward(m, &cid, &obj, pane_deps).await,
crates/holler-hub/src/profile/mod.rs:67        pub fn check_membership(_pane: &Pane, _profiles: &ProfileState) -> Result<(), PaneError> { Ok(()) }   // "#639 calls it" on cas_put
```
`PANE_METHODS` contains five names. This story implements all five, `pane/delete` included (epic ruling 9, issue amendment
2026-10-09):
```
crates/holler-proto/src/methods.rs:115-121   "pane/get", "pane/list", "pane/cas_put", "pane/delete", "pane/watch"
```
The port, the cursor and the feed rules (frozen by #637):
```
crates/holler-pane/src/ports.rs:28-34   pub struct Cursor(pub u64);   // store-wide, strictly increasing, one per change; Cursor(0) = from the beginning
crates/holler-pane/src/ports.rs:36-52   pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;
   - watch(Cursor(0)) first yields a put for every record it holds now, then every later change
   - passing the cursor of the last event seen back as `since` resumes without a gap or a repeat
   - next() blocks at most I5's bound: Ok(Some) a change; Ok(None) idle (stream stays usable; a hub long-poll answers {events: [], cursor});
     Err ends the stream; Err(Timeout) is a wedged store, never "idle"
crates/holler-pane/src/ports.rs:62-82   pub trait PaneStore: Send + Sync {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;
    fn list(&self) -> Result<Vec<Pane>, PaneError>;
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;   // 0 for a new pane; returns the stored record
    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;   // missing -> pane-not-found, checked FIRST
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;  }
   "Every method is synchronous. Call from spawn_blocking (or a thread) in async code."
crates/holler-pane/src/generation.rs:25   pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError>   // Conflict when they differ; overflow is StoreCorrupt
```
The record, its event, and the wire types:
```
crates/holler-pane/src/pane.rs:225-254   #[serde(deny_unknown_fields)] pub struct Pane { name: PaneName, generation: u64, herdr: HerdrPane, host, harness,
    session_of_record: Option<String>, role, hold, last_observed, #[serde(default, skip_serializing_if = "Option::is_none")] profile: Option<ProfileName>,
    model, env: Vec<EnvVarName>, context, command: Option<Argv>, probe }
crates/holler-pane/src/pane.rs:257-268   pub struct PaneEvent { cursor: Cursor, name: PaneName, #[serde(default)] pane: Option<Box<Pane>> }   // None = deleted
crates/holler-pane/src/reply.rs:61,71    PaneReply::success(data: Value) / PaneReply::failure(&PaneError)
crates/holler-pane/src/reply.rs:115      pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError>   // "a request that carries no params decodes from {}"
crates/holler-pane/src/reply.rs:122-160  PaneGetParams { name }, PaneCasPutParams { pane, expected_generation }, PaneDeleteParams { name, expected_generation },
                                         WatchParams { #[serde(default)] since: Cursor }, WatchReply<E> { events: Vec<E>, cursor: Cursor }
crates/holler-pane/src/reply.rs:18-20    "The `data` of pane/get, list, cas_put, delete ... is not given a type here; the server stories (#639, #661) and the client (#649) agree on it"
```
The error codes used here. All of them are closed variants, and this story adds none:
```
crates/holler-pane/src/error.rs:332   Conflict                         // generation-conflict
crates/holler-pane/src/error.rs:365   PaneNotFound { what: String }    // pane-not-found
crates/holler-pane/src/error.rs:369-371 /// `store-corrupt`: a stored file or record cannot be read back; the store fails closed rather than dropping state. (#639/#661.)
                                      StoreCorrupt { what: String }
crates/holler-pane/src/error.rs:374   Unavailable { what: String }     // unavailable
```
The analogous hub object, and its persistence:
```
crates/holler-hub/src/holds.rs:29-50   holds live in <state dir>/hub/holds.json, written at 0600 through holler_proto::atomic_file::write_atomic;
                                       corrupt -> moved aside, start empty; unreadable -> start empty and stop writing
crates/holler-hub/src/holds.rs:293     self.shared.state.lock().unwrap_or_else(PoisonError::into_inner)
crates/holler-hub/src/holds.rs:249-260 fn emit(severity, method, fields) -> log::emit(&Event { component: Component::Control, direction: Direction::Local, .. })
crates/holler-hub/src/holds/tests.rs:34,130,153  unwritable / unreadable cases made with set_permissions(0o500 / 0o000)
crates/holler-proto/src/atomic_file.rs:66   pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()>
   temp file in the same dir, then rename: a reader sees the old or the new bytes, never a partial file; the temp file is removed on
   failure; no fsync (the hub's other state files make the same choice, #483)
```
The #669 test that this story must amend, because it asserts `not-implemented` for every pane method:
```
crates/holler-hub/tests/pane_dispatch_test.rs:125-141  every_pane_method_is_forwarded_to_the_stub_not_method_not_found   (asserts Err(NotImplemented) per PANE_METHODS)
crates/holler-hub/tests/pane_dispatch_test.rs:226-231  two_connections_share_the_one_pair_of_state_handles               (asserts pane/list -> NotImplemented)
crates/holler-hub/tests/pane_dispatch_test.rs:284-297  panes_dispatch_takes_both_handles_and_answers_not_implemented_as_a_result (pane/watch -> NotImplemented)
crates/holler-hub/tests/pane_dispatch_test.rs:47       const REPLY_WITHIN: Duration = Duration::from_secs(10);
```
The control client's default timeout. It sizes the long-poll window (D5):
```
crates/holler-hub/src/control.rs:16   const CLIENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);   // ControlCall.timeout is pub; #649 may raise it
```
Build facts, checked on this branch:
- `crates/holler-hub/Cargo.toml` has no `autotests = false`, so `crates/holler-hub/tests/*.rs` are discovered automatically and
  need no `[[test]]`.
- The dependencies needed are already declared: tokio (`rt`, `sync`, `time`, `macros`), `serde_json`, `holler-proto`,
  `holler-pane`, and the dev-dependencies `tempfile`, `libc` and `holler-pane-testkit`. `rstest` is **not** a hub
  dev-dependency, so use plain table loops.
- `panes/mod.rs`, `pane_dispatch.rs` and `pane_dispatch_test.rs` are rustfmt-clean today.
- `serve.rs` is 838 lines and `control_server.rs` is 837 (not touched).

## Acceptance criteria

Each item names the test that proves it. T authors them RED first. "Store tests" go in
`crates/holler-hub/tests/pane_registry_test.rs` and drive `PaneState` through the `PaneStore` trait from plain threads, with no
socket. "Handler tests" go in `crates/holler-hub/tests/pane_handlers_test.rs` and call `panes::dispatch` directly, then parse
the line. The socket forwarding is already pinned by #669.

**Store: CAS, records, the no-inference rule**
1. [ ] `cas_put_creates_at_generation_one_and_get_returns_it`: `cas_put(p, 0)` on a new name returns the record at generation
   1, and `get` returns an equal record.
2. [ ] `cas_put_with_a_stale_generation_is_a_conflict_and_changes_nothing`: `expected != current` (including `0` on an
   existing record) returns `Err(PaneError::Conflict)` (code `generation-conflict`). After it, the stored record, the file's
   bytes and the feed's head cursor are all unchanged.
3. [ ] `cas_put_stores_the_record_verbatim_except_generation`: the stored record equals the submitted one with only
   `generation` replaced. The submitted `generation` is ignored. A record with `session_of_record: None` and
   `last_observed.shown: Some(..)` keeps `None`, because the hub never infers a session (I6).
4. [ ] `delete_checks_missing_before_generation`: deleting a name that does not exist returns `pane-not-found` for any
   `expected_generation`, including after an earlier delete. A stale generation returns `generation-conflict`. A correct
   generation removes the record from `get` and `list`. A later `cas_put(p, 0)` recreates it at generation 1.
5. [ ] `list_returns_every_live_record_sorted_by_name`.

**Persistence**
6. [ ] `records_survive_a_restart_unchanged`: write two panes (one with `profile: Some(..)` and `herdr.grid` `r2c1`), delete a
   third, drop the state, then `PaneState::load` the same dir. `list` is equal, field for field: generations, `profile`, and
   the grid's JSON `{"row":2,"col":1,"pos":"r2c1"}`. The deleted pane stays deleted. The next write's event cursor is the
   pre-restart head + 1, so the cursor never goes backwards.
7. [ ] `a_record_written_before_the_profile_field_loads_with_profile_none`: a hand-written v1 file whose record has no
   `profile` key loads, and that record's `profile` is `None`.
8. [ ] `the_file_is_written_atomically_at_mode_0600` (unix): after a write, `<root>/hub/panes.json` has mode `0o600`, parses as
   the D2 document, and no `.panes.json.*.tmp` is left in the dir. A write also succeeds when `<root>/hub` did not exist yet.
9. [ ] `a_corrupt_file_fails_closed_and_is_never_rewritten`: a table over five cases (not JSON; `version: 2`; a record with an
   unknown field; two entries with the same name; an entry whose `name` differs from its record's `name`). After `load`, each
   of `get`, `list`, `cas_put`, `delete` and `watch` returns `PaneError::StoreCorrupt` (code `store-corrupt`) with a `what` that
   names the file. The file's bytes are identical after the write attempts, and no file was moved aside or created.
10. [ ] `an_unreadable_file_fails_closed_and_is_never_rewritten` (unix, `0o000`): same expectations as 9.
11. [ ] `the_corrupt_reason_never_echoes_file_content` (the secrets-absent assertion): a corrupt file holds a sentinel string
    in a field of the wrong type (for example `"port": "SENTINEL-639"`). The sentinel appears in neither the error's `what`
    nor its `Display`. The logged reason is built by the same function (D3), so this also covers the log.
12. [ ] `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` (unix, hub dir `0o500`): `cas_put` returns
    `PaneError::Unavailable` (code `unavailable`). `get` still returns the old record and the head cursor did not move. After
    the permission is restored, the same `cas_put` with the same `expected_generation` succeeds.

**Concurrency, asserted as invariants (no sleeps)**
13. [ ] `racing_writers_exactly_one_wins`: 16 threads released together by a `std::sync::Barrier` each `cas_put` the same pane
    with the generation they all read. Exactly one gets `Ok`, 15 get `Conflict`, the stored generation is base + 1, and a fresh
    `load` of the file equals memory.
14. [ ] `concurrent_read_modify_write_loses_no_update`: 8 threads each do 25 increments of `context.soft` by get, change,
    `cas_put`, retrying on `Conflict`. At the end `soft == base + 200` and `generation == base_gen + 200`. A watch from the
    starting cursor yields exactly 200 put events with consecutive cursors.
15. [ ] Handler test `two_cas_put_requests_racing_exactly_one_wins`: two `pane/cas_put` dispatch futures joined on a
    multi-thread runtime (`#[tokio::test(flavor = "multi_thread")]`). One reply is `ok`, the other is
    `error.code == "generation-conflict"`. This is the issue's "two verbs racing".

**The change feed**
16. [ ] `watch_from_zero_yields_every_current_record_then_later_changes`: one put per live record, ordered by the record's last
    change cursor, then the next write.
17. [ ] `the_feed_delivers_every_write_exactly_once_in_order`: from a cursor `c`, perform M writes that include an update to
    the same pane twice and one delete. The iterator yields exactly M events with cursors `c+1..=c+M`, each one matching its
    write (a delete has `pane: None`). The next `next()` is `Ok(None)` (idle), using a short window (D5).
18. [ ] `resuming_from_the_last_cursor_neither_repeats_nor_skips`.
19. [ ] `a_waiting_watch_wakes_on_the_next_write`: a thread blocks in `next()` while the main thread writes. The thread
    receives that event. The order does not matter: an event written before the wait starts is returned at once. Collect the
    result with a bounded `recv_timeout`, not a sleep.
20. [ ] `a_cursor_ahead_of_the_store_is_usage`: `watch(head + 1)` returns `PaneError::Usage`.
21. [ ] `after_a_restart_an_old_cursor_gets_each_changed_pane_once_including_deletions`: record cursor `c`, put A, put B, put A
    again, delete B, restart, then `watch(c)`. The result is exactly two events, ordered by cursor: A's latest put and B's
    delete (D6).
22. [ ] `a_watcher_behind_the_retained_window_gets_the_compacted_changes`: same as 21, but with no restart and
    `feed_retained: 2` (D5).

**Wire handlers** (data shapes per D7)
23. [ ] `get_list_cas_put_delete_round_trip`: `pane/cas_put` returns `data` = the stored record. `pane/get` returns the record,
    or `data: null` for an unknown name. `pane/list` returns a sorted array. `pane/delete` returns `data: null`. A request with
    no `params` member works for `pane/list` and `pane/watch`.
24. [ ] `bad_params_answer_their_code`: missing `name` returns `usage`. A record whose `command` is a shell string returns
    `command-not-argv`, because the guard's code survives `decode_params`. A missing pane on `pane/delete` returns
    `pane-not-found`. A stale generation on `pane/cas_put` returns `generation-conflict`.
25. [ ] `pane_watch_answers_one_batch_and_its_cursor`: `pane/watch {since}` returns
    `data = {"events": [...], "cursor": N}`, which parses as `WatchReply<PaneEvent>`. When the store is idle, the reply comes
    after the window with `{"events": [], "cursor": since}`.
26. [ ] `a_corrupt_registry_answers_store_corrupt_on_every_pane_method`: as a JSON-RPC **result** carrying a `PaneReply`, never
    as a JSON-RPC error.
27. [ ] `pane/cas_put` calls `crate::profile::check_membership` before the CAS. S verifies this with
    `grep -n "check_membership(" crates/holler-hub/src/panes/`. An `Err` from it is returned as the reply, and nothing is
    written.

**#669's test, amended (D8)**
28. [ ] In `pane_dispatch_test.rs`, the three assertions listed in Evidence now require a `PaneReply` **result** that is not
    `not-implemented` for each pane method: `pane/list` returns `Ok(Some([]))`, and the rest return a decode error or an empty
    batch. These tests build `PaneDeps` with the short watch window, so no test waits the full window. Every `profile/*`
    assertion is unchanged.

**Build guards**
29. [ ] The following all pass: `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo test --workspace`, `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`.
    - Every new `.rs` file passes `rustfmt --check --edition 2021`, and `panes/mod.rs` and `pane_dispatch_test.rs` stay
      rustfmt-clean. No other existing file is reformatted.
    - No new file reaches 600 lines.
    - Every `#[allow]` carries a trailing `// #639`.
    - `CATALOG.len()` is still 22, and no golden file changes.
    - `CHANGELOG.md` `## [Unreleased]` has an `### Enhancements` entry that links #639.
30. [ ] `git diff --name-only origin/main...HEAD` lists only Blast-radius paths. In particular there are no changes to
    `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `profile/**`, `holler-pane`, `holler-proto`, any `Cargo.toml` or
    `Cargo.lock`.

## Files

Production code, all in `crates/holler-hub/src/panes/` (the issue's radius):
- `mod.rs` (edit): module docs, `pub struct PaneState { store: Arc<store::Store> }` (no `Clone`), `PaneState::load` and
  `load_with(state, PaneStoreOptions)`, `pub struct PaneStoreOptions { pub watch_wait: Duration, pub feed_retained: usize }`
  with `Default`, `pub const WATCH_WAIT`, `impl PaneStore for PaneState` (each method delegates to `Store`), and `dispatch`,
  which only routes by method to `handlers`. The `dispatch` signature is unchanged.
- `store.rs` (new): `pub(crate) struct Store`, made of `Mutex<Result<Table, PaneError>>`, a `Condvar`, the path and the
  options.
  - `Table` is `{ head: Cursor, entries: BTreeMap<PaneName, Entry>, ring: feed::Ring }`, and
    `Entry { cursor: Cursor, pane: Option<Pane> }` (`None` is a tombstone, D6).
  - It also holds the CAS logic for `cas_put` and `delete`, and `get`/`list`.
- `persist.rs` (new): the file document (D2) and two functions. `load_doc(path) -> Result<Option<Doc>, PaneError>` treats a
  missing file as `None`. `save_doc(path, &Doc) -> Result<(), PaneError>` does `create_dir_all(parent)`, then `to_vec_pretty`,
  then `write_atomic(.., 0o600)`.
  - The corrupt-reason function `corrupt_what(path, problem)` (D3) and the validation of entries also live here.
  - The two functions are `pub(crate)` and generic over the document's serde type, so #661's `profiles.json` can call them
    from `profile/` without editing this directory.
- `feed.rs` (new): `pub(crate) struct Ring<E>` holds the bounded event buffer and its floor. The `poll` function
  `poll(store, since, wait) -> Result<WatchReply<PaneEvent>, PaneError>` implements the D5/D6 rules. The `Watch` iterator
  built on `poll` buffers one batch and yields `Ok(None)` when idle.
- `handlers.rs` (new): one async fn per method. Each one does `params = obj.get("params")`, treating absent or `null` as `{}`,
  then `decode_params`, then `tokio::task::spawn_blocking` on a clone of the `Arc<Store>`, then `PaneReply`, then
  `reply_line`. A `JoinError` maps to `unavailable`.

Tests:
- `crates/holler-hub/tests/pane_registry_test.rs` (new, store tests 1-14 and 16-22).
- `crates/holler-hub/tests/pane_handlers_test.rs` (new, 15 and 23-27).
- `crates/holler-hub/tests/pane_support/mod.rs` (new, shared via `mod pane_support;`).
  - It holds `sample_pane(name, profile)` (moved from `pane_dispatch_test.rs:317-338`), `temp_state()`, `short_opts()` and
    `outcome(line) -> Result<Option<Value>, PaneError>` (the existing `pane_outcome` parse-back).
  - Its first line is `#![allow(dead_code)] // #639`, the `holler-cli/tests/support` precedent, because not every binary uses
    every helper.
- `crates/holler-hub/tests/pane_dispatch_test.rs` (edit, D8).

Also `CHANGELOG.md` (one entry).

Reuse map (extend, do not duplicate):

| Need | Reuse |
|---|---|
| Atomic file write | `holler_proto::atomic_file::write_atomic` |
| CAS rule | `holler_pane::next_generation` (never re-implement it) |
| Params decoding | `holler_pane::decode_params` and the `reply.rs` params structs |
| Replies | `PaneReply::success`/`failure` and `crate::pane_dispatch::reply_line` |
| Mutex lock | `lock().unwrap_or_else(PoisonError::into_inner)` (`holds.rs:293`) |
| Logging | the `holds.rs` `emit` shape (`Component::Control`, `Direction::Local`) |
| Time | the registry stamps nothing; the `Pane` type is frozen |

A second CAS helper, a second code validator, or a copy of `sample_pane` in each test file is a rejection.

## Decisions already made (epic, issue, #669)

- The hub is the store only: `pane/*` are get/list/cas_put/delete/watch, and no adapter runs in the hub (epic ruling 1).
  `pane/delete` is in scope (ruling 9 and the issue's 2026-10-09 amendment).
- Replies are a `PaneReply` in a JSON-RPC **result**, and `*/watch` is long-poll with one `{events, cursor}` per request (#669
  decision 3, `reply.rs`).
- Persistence and corrupt-file behaviour live inside `PaneState::load`, which runs before the first connection (#669 decision
  4). Fail closed with a named error, and never reset silently (issue acceptance).
- `profile` persists like every other field, and profile storage is #661. `herdr.grid` is stored as `GridPos` and never
  converted (issue amendments).
- `PaneState` does not derive `Clone`. Shared state goes through `Arc` (#669 AC 3; pinned by
  `the_state_types_do_not_derive_clone...`).
- No new error code: everything maps to a closed variant (epic ruling 3).

## Decisions made in this brief (for operator review)

- **D1. Store shape and locking.**
  - One `std::sync::Mutex` guards the whole table, with one `Condvar` for watchers.
  - A write runs entirely under the lock, in this order: check the CAS (`next_generation`), build the next document,
    `save_doc`, and only then commit to memory, push the event and `notify_all`. A failed save therefore leaves memory, the
    file and the cursor all unchanged, and the file always holds a committed state.
  - No `.await` happens while the lock is held: all store code is synchronous, and the handlers reach it through
    `spawn_blocking`, as the port's docs require.
  - Lock order: the pane lock is never held while calling into `ProfileState`. `check_membership` runs before the store call.
- **D2. File:** `<state dir>/hub/panes.json` (beside `holds.json`), mode `0600`, pretty JSON. Its shape is
  `{"version": 1, "cursor": <head u64>, "entries": [{"name": "...", "cursor": <u64>, "pane": {<Pane>}?}]}`, with
  `deny_unknown_fields` and entries sorted by name.
  - Loading validates the version (1), unique names, `entry.name == pane.name`, `0 < entry.cursor <= cursor`, and
    `pane.generation >= 1`. Any violation counts as corrupt.
  - #661 uses the same layout in `profiles.json`, through the generic `persist.rs` functions.
  - Durability matches the hub's other state files: atomic rename, no fsync.
- **D3. Fail closed.**
  - A missing file is an empty registry. Any other read error, a parse failure or a failed validation puts the store in a
    failed state.
  - In that state every method, `watch` included, answers
    `StoreCorrupt { what: "pane registry <path>: <problem>" }`. The file is **left in place, never moved aside or
    rewritten**, unlike `holds.json`. Pane records gate every verb, so an empty registry would be a silent reset. The state
    clears only on a restart after the operator repairs the file.
  - One `Severity::Error` event `pane_registry_corrupt` (fields `path`, `problem`, `effect`) is logged at load. The hub still
    starts: other hub functions do not depend on panes, and `load`'s infallible signature is #669's.
  - `<problem>` is built only from the error category, line and column (`serde_json::Error::classify`/`line`/`column`), the
    version number, a duplicated or mismatched pane name, or the io `ErrorKind`. It never includes serde's message text, which
    can quote file content.
- **D4. Write failure** (unwritable dir or disk): the call returns
  `Unavailable { what: "pane registry <path>: not written: <io error>" }` and nothing changes. Holds keeps a hold in memory
  when the save fails, but here a write that would not survive a restart breaks the registry's purpose. Nothing blocks later
  writes, and a retry with the same generation succeeds.
- **D5. Feed: cursors and the long-poll window.**
  - Each put or delete takes `head + 1`. The head is persisted, so cursors keep increasing across restarts.
  - An in-memory ring keeps the last `feed_retained` events (default 1024), exact per write. It starts empty after a restart.
  - The long-poll window is `WATCH_WAIT = 4 s`, below the control client's default 5 s timeout and within I5's 10 s. #649 can
    read `holler_hub::panes::WATCH_WAIT` to size `ControlCall.timeout`.
  - The trait iterator's `next()` waits at most the same window.
  - `PaneStoreOptions` exists so tests can use a short window and a small ring. `load` uses the defaults.
- **D6. `watch(since)` rules**, evaluated under the lock and repeated after each `Condvar` wake until events exist or the
  window ends:
  1. If `since > head`: `usage`.
  2. If `since == 0`: a put for each live record, ordered by its entry cursor (the port's rule).
  3. If `since >= ring floor`: the ring's events after `since`. Every write is delivered once.
  4. Otherwise (after a restart, or when the watcher has fallen behind the ring): one event per entry whose cursor is greater
     than `since`, ordered by cursor. This is the latest state of each changed pane, and a **tombstone** yields a delete event.
     Intermediate writes are collapsed and documented as such. A watcher that keeps up never reaches this case.
  - The reply's `cursor` is always `head`, read at answer time. Resuming from it never repeats or skips anything.
  - Tombstones are persisted (an entry without `pane`), so a deletion is never lost across a restart. They are bounded by the
    number of distinct pane names, and a `cas_put(p, 0)` replaces one.
- **D7. `data` shapes** (`reply.rs` leaves these to #639 and #649):
  - `pane/get`: the `Pane`, or `null` (mirrors `Ok(None)`; the verb decides whether absence is an error).
  - `pane/list`: an array of `Pane` sorted by name.
  - `pane/cas_put`: the stored `Pane`.
  - `pane/delete`: `null`.
  - `pane/watch`: a `WatchReply<PaneEvent>`.
  - #649 reads these as `serde_json::from_value` of `into_result()`'s data.
- **D8. Blast radius widened by one test file.** `pane_dispatch_test.rs` is #669's file, but its three pane-method assertions
  would fail once the stub is replaced. Only those assertions change, plus the switch to `pane_support` for `sample_pane`. The
  issue's radius (`panes/**`) did not anticipate this.
- **D9. Membership hook (consequence for #661).** `pane/cas_put` calls `check_membership(&incoming, profiles)` with exactly
  #669's frozen signature, before the CAS and outside the pane lock.
  - The hook does not see the **stored** record, so #661's `pane-in-other-profile` rule cannot compare "was in A, now B"
    through it. That needs either a widened hook signature (an edit to `panes/handlers.rs`, outside #661's radius) or the
    comparison inside the pane CAS.
  - #639 does not implement the rule; it belongs to #661. **Note for the operator:** #661's brief must take one of those two
    routes.

## Out of scope

- `profile/*` handlers, the profile store, and the body of `check_membership` (#661). `profile/rename` (#665).
- Running #638's `PaneStore` conformance suite. #638 is not merged; once it is, its suite can be pointed at
  `PaneState::load_with` by #638 or #649, with no change here.
- Any change to `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, the frozen `holler-pane` types, `holler-proto`, ADR-0021
  (#634) or `docs/protocol/v2.md`. `pane/*` are control-socket methods outside the 22-row catalog.
- The client side of `pane/*` (#649), the CLI verbs (#643-#647, #650), and any adapter.
- Pruning tombstones, and fsync.

## Test plan

**RED (T):**
- The two new test files and `pane_support` will not compile until `PaneStoreOptions`, `PaneState::load_with` and
  `impl PaneStore for PaneState` exist. That compile failure is the RED for the store tests.
- The handler tests fail on `not-implemented`.
- The amended `pane_dispatch_test` assertions fail on `not-implemented`.
- Confirm with `cargo test -p holler-hub --test pane_registry_test --test pane_handlers_test --test pane_dispatch_test`. Check
  that no RED comes from a missing helper or a typo.

**GREEN (F):**
1. `persist.rs` and `store.rs`, which turn tests 1-14 green.
2. `feed.rs`, which turns 16-22 green.
3. `handlers.rs` and `dispatch`, which turn 15 and 23-28 green.
4. Then the AC 29 commands and the full `cargo test --workspace`.

Permission-based tests (8, 10, 12) are `#[cfg(unix)]` and follow `holds/tests.rs`. Every wait in a test is bounded
(`recv_timeout`, `tokio::time::timeout`, or the short `watch_wait`). There is no `thread::sleep`.

## Risks

- **Long-poll threads.** Each outstanding `pane/watch` occupies one blocking-pool thread for up to `WATCH_WAIT` (4 s), and the
  runtime's shutdown waits for it, so it stays bounded. Do not raise the window without revisiting this.
- **ABA after delete and recreate.** Generation restarts at 1, so a client that holds generation 1 of the old incarnation can
  overwrite the new one. The contract fixes this ("0 for a new pane"). It is recorded, not fixed.
- **File mode tests running as root.** Root bypasses mode bits. CI runs as non-root, as `holds/tests.rs` already relies on.
- **Complexity limits** (`cognitive_complexity` 15, `too_many_lines` 100). Keep the D6 rule selection in its own small
  function, and keep load validation in `persist.rs`, separate from parsing.
- **`deny_unknown_fields`.** A file written by a newer build with a new `Pane` field fails closed as `store-corrupt`. That is
  intended (epic serde policy), and the CHANGELOG entry says so.

## Blast radius

`crates/holler-hub/src/panes/**` (`mod.rs`, `store.rs`, `persist.rs`, `feed.rs`, `handlers.rs`);
`crates/holler-hub/tests/pane_registry_test.rs`, `pane_handlers_test.rs`, `pane_support/mod.rs` (new);
`crates/holler-hub/tests/pane_dispatch_test.rs` (D8 only); `CHANGELOG.md`; `docs/handoffs/639*` (pipeline artifacts).

Not changed: `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `profile/**`, `holler-pane`, `holler-proto`,
`holler-cli`, any `Cargo.toml`, `Cargo.lock`, any golden file, any ADR.
