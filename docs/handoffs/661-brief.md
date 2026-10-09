# Brief: #661 the hub profile registry (storage, compare-and-swap, change log, feed, `profile/*` handlers, the membership rule)

Repo: Performant-Labs/holler. Issue: #661 (wave 3 of epic #633). It depends on #637 (`f2602ba`), #669 (`af3d8df`),
#639 (`2a6f349`) and #638 slice c part 1 (`9d61c9f`, `FakeProfileStore` and its conformance suite), all merged. Rigor:
in-session. UI surface: no. Kind: feature.

**Branch:** `issue-661-implementation`. **Design (D):** N/A (no UI). **Forward-compat:** done, see the table under
"Forward compatibility". **Decision record:** the contract section of epic #633, its "Skeleton split" rulings 1-9, its
"Decisions 2026-10-09" (item 2: `pane-in-other-profile` is checked inside the pane registry's compare-and-swap, not in
the hook), ADR-0021 sections 6, 7 and 8, and the merged `holler-pane` crate are fixed. The issue text of #661 (with its
2026-10-08 and 2026-10-09 amendments) is the source of truth together with the epic. Where the merged code and the prose
differ, this brief follows the merged code and says so under "Decisions made in this brief".

**Size check:** fits one run (O's judgement against the F scope cap). The production change is one component family, the
hub's registries: four files under `profile/` (one edited, three new, about 800 lines in total, none over 450), one
comparison of about 15 lines in `panes/store.rs`, and two one-word visibility changes in `panes/` (D10). Tests: four new
test files, one shared-helper extension and two amended #639/#669 test files, about 1,700 lines. Total about 2,500 lines.
If A blocks on size, split at the membership seam: **661a** is the profile registry (everything except AC 34-41) and
**661b** is the membership rule (`check_membership`, the in-CAS comparison, `pane_membership_test.rs`, and the
`pane_handlers_test.rs` and `check_membership` amendments). This brief covers both; 661b is self-contained once 661a has
merged.

**Needs operator:** none. Decisions D4, D9, D10 and D12 are flagged for operator review; none blocks the run.

## Problem

The hub routes `profile/*` to `profile::dispatch`, which answers every method `not-implemented`. `ProfileState::load`
returns a unit struct that keeps nothing, and `check_membership` accepts every pane. The pane registry's compare-and-swap
does not refuse moving a pane from one profile to another. The profile verbs (#662, #663, #664) code against the
`ProfileStore` trait and #638's fake; #649 wires in the real store. They need:

- one record per profile, filed by slug, kept in `<state dir>/hub/profiles.json`, so it survives a hub restart;
- every write a compare-and-swap on `generation`, with a unique display name and a unique slug;
- an append-only change log per profile (who, when, the new generation, what changed), persisted with the records;
- a change feed for `profile/watch`;
- a corrupt file that fails closed instead of being reset;
- the six `profile/*` handlers (`get`, `list`, `cas_put`, `delete`, `watch`, `log`);
- the membership rule: a pane belongs to at most one profile (`pane-in-other-profile`, inside the pane CAS), and a pane
  names only a profile that exists (`profile-not-found`, in the hook).

## Evidence (verbatim, as of `9d61c9f`)

The stub this story fills (the whole module today, docs elided):
```
crates/holler-hub/src/profile/mod.rs:15      pub mod rename;
crates/holler-hub/src/profile/mod.rs:31-40   pub struct ProfileState;
                                             impl ProfileState { pub fn load(_state: &HubState) -> Self { Self } }
crates/holler-hub/src/profile/mod.rs:48-59   pub async fn dispatch(method: &str, cid: &CorrelationId, obj: &Value, profiles: &ProfileState, panes: &PaneState) -> String {
                                                 match method {
                                                     "profile/rename" => rename::dispatch(cid, obj, profiles, panes).await,
                                                     _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
                                                 } }
crates/holler-hub/src/profile/mod.rs:67-69   pub fn check_membership(_pane: &Pane, _profiles: &ProfileState) -> Result<(), PaneError> { Ok(()) }
crates/holler-hub/src/profile/rename.rs      `profile/rename` (PROPOSED, #665): answers not-implemented. Not edited by this story.
```
How it is wired in (no edit needed in any of these):
```
crates/holler-hub/src/pane_dispatch.rs:50-66   PaneDeps { panes: Arc<PaneState>, profiles: Arc<ProfileState> }; PaneDeps::load builds both with ::load(state)
crates/holler-hub/src/pane_dispatch.rs:75-94   forward: is_profile_method -> profile::dispatch(method, cid, obj, &deps.profiles, &deps.panes).await   // obj is the WHOLE frame
crates/holler-hub/src/pane_dispatch.rs:98-101  pub(crate) fn reply_line(cid, reply: &PaneReply) -> String
crates/holler-proto/src/methods.rs:125-133     PROFILE_METHODS = profile/get, list, cas_put, delete, watch, log, rename (PROPOSED)   // fixed by #637, outside CATALOG
```
The membership hook's one caller (#639), on the blocking pool and outside the pane lock:
```
crates/holler-hub/src/panes/handlers.rs:104-115
pub(crate) async fn cas_put(cid, obj, store: Arc<Store>, profiles: Arc<ProfileState>) -> String {
    run(cid, obj, move |params: PaneCasPutParams| {
        check_membership(&params.pane, &profiles)?;
        store.cas_put(&params.pane, params.expected_generation)
    }).await }
```
The pane CAS this story amends (the one comparison goes after `next_generation`):
```
crates/holler-hub/src/panes/store.rs:165-182
pub(crate) fn cas_put(&self, pane: &Pane, expected: u64) -> Result<Pane, PaneError> {
    let mut guard = self.lock();
    let table = guard.as_mut().map_err(|err| err.clone())?;
    let current = table.record(&pane.name).map_or(0, |stored| stored.generation);
    let stored = Pane { generation: next_generation(current, expected)?, ..pane.clone() };
    let change = PaneEvent { cursor: table.next_cursor()?, name: pane.name.clone(), pane: Some(Box::new(stored.clone())) };
    self.commit(table, change)?;
    Ok(stored) }
```
The #639 pieces built for reuse by #661 (their own module docs say so):
```
crates/holler-hub/src/panes/mod.rs:19-22     "#661's profile registry reuses `persist`, `feed` and `handlers::run` from `profile/`, through the
                                             crate-private `RegistryEntry` trait. #661 also adds one comparison here ... (`store.rs`), under the pane lock"
crates/holler-hub/src/panes/mod.rs:45-48     pub(crate) mod feed; pub(crate) mod handlers; pub(crate) mod persist; mod store;
crates/holler-hub/src/panes/mod.rs:76        pub const WATCH_WAIT: Duration = Duration::from_secs(4);
crates/holler-hub/src/panes/mod.rs:81-87     pub struct PaneStoreOptions { pub watch_wait: Duration, pub feed_retained: usize }   // Default: WATCH_WAIT, 1024
crates/holler-hub/src/panes/mod.rs:152-159   pub(crate) trait RegistryEntry: Clone { fn name(&self) -> &str; fn cursor(&self) -> Cursor; fn record(&self) -> Option<(&str, u64)>; }
crates/holler-hub/src/panes/persist.rs:73-77 pub(crate) struct Doc<E> { version: u64, cursor: Cursor, entries: Vec<E> }   // deny_unknown_fields; VERSION = 1; FILE_MODE 0o600
crates/holler-hub/src/panes/persist.rs:89    pub(crate) struct Problem(String);   pub(crate) fn what(&self, label, path) -> "{label} {path}: {problem}"
crates/holler-hub/src/panes/persist.rs:100   Problem::parse: only category ("not valid JSON" / "a value this hub cannot read" ...), line, column — never serde's text
crates/holler-hub/src/panes/persist.rs:128   pub(crate) fn load_doc<E: RegistryEntry + DeserializeOwned>(path) -> Result<Option<Doc<E>>, Problem>   // missing file = Ok(None)
crates/holler-hub/src/panes/persist.rs:150   pub(crate) fn save_doc<E: Serialize>(path, &Doc<E>) -> Result<(), Problem>   // create_dir_all, then write_atomic(.., 0o600)
crates/holler-hub/src/panes/persist.rs:160-196 check: unique name(), unique cursor(), 0 < cursor <= head, record name == entry name(), record generation >= 1
crates/holler-hub/src/panes/feed.rs:47-84    pub(crate) struct Ring<E> (new(head, capacity), push(event))
crates/holler-hub/src/panes/feed.rs:88       pub(crate) fn check_since(since, head) -> Result<(), PaneError>   // ahead of head = usage
crates/holler-hub/src/panes/feed.rs:103-120  pub(crate) fn select(since, head, &Ring<E>, entries: impl Iterator<Item = &E>) -> Result<Vec<E>, PaneError>  // rules 1-4
crates/holler-hub/src/panes/handlers.rs:38-40 #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct NoParams {}      // private today (D10)
crates/holler-hub/src/panes/handlers.rs:44   pub(crate) async fn run<P, T, F>(cid, obj, work: F) -> String   // params ({} if absent) -> decode_params -> spawn_blocking -> PaneReply
crates/holler-hub/src/panes/store.rs:261-284 Store::poll (the long-poll wait on the Condvar, lock released while waiting)
crates/holler-hub/src/panes/store.rs:290-319 struct Feed + impl Iterator (the Watch iterator over poll)
crates/holler-hub/src/panes/store.rs:337-352 fn log_fault(method, path, problem, effect)   // private today (D10)
```
The port and records (frozen by #637):
```
crates/holler-pane/src/profile.rs:72-74     ProfileName::slug(&self) -> String      // lower-case ASCII alnum, runs of anything else -> one '-'
crates/holler-pane/src/profile.rs:207-220   #[serde(try_from = "RawProfile")] pub struct Profile { name, slug, generation, panes: Vec<ProfileSpec>, created: i64, updated: i64 }
                                            // RawProfile is deny_unknown_fields; decode refuses a slug that differs from the name's. No `log` field.
crates/holler-pane/src/profile.rs:178-200   ProfileSpec { pane: String /* plain text: a detached spec may name any pane */, ..., env: Vec<EnvVarName>, ... }
crates/holler-pane/src/profile.rs:278-289   enum ProfileChange { Created, Updated { summary: String }, Renamed { from }, Deleted }
crates/holler-pane/src/profile.rs:295-302   ProfileLogEntry { at: i64, generation: u64, actor: Actor, change: ProfileChange }
crates/holler-pane/src/profile.rs:308-317   ProfileEvent { cursor: Cursor, name: ProfileName, #[serde(default)] profile: Option<Box<Profile>> }
crates/holler-pane/src/profile.rs:328-371   trait ProfileStore: Send + Sync { get, list, cas_put(profile, expected, actor), delete(name, expected, actor), watch(since), log(name), rename(..) /* PROPOSED */ }
                                            "Every method is synchronous. Call from spawn_blocking (or a thread) in async code."
crates/holler-pane/src/reply.rs:162-199     ProfileGetParams { name }, ProfileCasPutParams { profile, expected_generation, actor },
                                            ProfileDeleteParams { name, expected_generation, actor }, ProfileLogParams { name }; WatchParams/WatchReply shared
crates/holler-pane/src/error.rs:436-449     ProfileExists { what }, PaneInOtherProfile { what }, ProfileSecretRefused (raised by EnvVarName's decode), ProfileNotFound { what }
crates/holler-proto/src/clock.rs:35         pub fn now_millis() -> i64    // 0 on a clock error
```
ADR-0021 (accepted) on this story:
```
docs/adr/ADR-0021.md §6   data: profile/get = record or null; list = array; cas_put = stored record; delete = null;
                          watch = WatchReply<ProfileEvent>; log = array of ProfileLogEntry, oldest first
docs/adr/ADR-0021.md §7   "the profile registry is <state dir>/hub/profiles.json ... 0600 through write_atomic ... The profile change log is
                          persisted in profiles.json with the records, never inside a Profile." One lock per registry; save before commit;
                          fail closed, file left in place; "The pane registry never calls into the profile registry while it holds its own lock."
docs/adr/ADR-0021.md §8   "setting Pane.profile to P when the stored pane already belongs to another profile is pane-in-other-profile, and P
                          must exist. A spec that names a pane of another profile (a detached spec) is not refused."
"Decisions taken" 2       the comparison runs inside the pane registry's CAS under the pane lock; check_membership keeps the existence check.
```
The test kit's rules this registry must keep (`crates/holler-pane-testkit/src/conformance/profile_store.rs:12-37`, and the
fake's docs at `src/profile_store.rs:56-90`):
```
- The slug is the identity. A write of a name whose slug is stored under another name is profile-exists, checked BEFORE the
  generation (a create of such a name at 0 is profile-exists, not generation-conflict). A stored record's slug is its name's.
- A Deleted entry carries the deleted generation + 1.
- A profile's log is never cut: readable after a delete, goes on across a re-create. Only log of a never-created name is
  profile-not-found.
- list has no order. The suite pins no timestamp and no Updated wording (a summary must be one non-empty line).
- Submitted slug, generation (and, in the fake, created/updated) are ignored; the store sets them.
```
The pane suite's membership case (`crates/holler-pane-testkit/src/conformance/pane_store.rs:384-408`, "The hub's registry
gains it with #661, so the hub passes every case only once #661 has merged") and the fake's rule it mirrors
(`src/pane_store.rs:130-136` checks the generation first, then `check_membership`; `:219-238` compares **slugs**:
`(Some(current), Some(next)) if current.slug() != next.slug()` is refused, leaving (`None`), joining from `None` and keeping
the profile are allowed).

There is **no `ASSUMPTION (#661)` comment anywhere in the test kit** (`grep -rn "ASSUMPTION" crates/` finds only #640 and
#642 ones). The rules above, which the suite's module docs list as "the rules the port leaves open, which every profile
registry (#661) keeps too", take their place as acceptance criteria (AC 1-6).

Tests this story must amend (they assert the stubs):
```
crates/holler-hub/tests/pane_dispatch_test.rs:104-111  fresh_deps: profiles: Arc::new(ProfileState::load(&state))    // 4 s window
crates/holler-hub/tests/pane_dispatch_test.rs:157-171  every_profile_method_is_forwarded_to_the_stub_not_method_not_found   (NotImplemented for all 7)
crates/holler-hub/tests/pane_dispatch_test.rs:240-244  two_connections_...: profile/list -> NotImplemented
crates/holler-hub/tests/pane_dispatch_test.rs:308-321  profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub (profile/get, profile/rename -> NotImplemented)
crates/holler-hub/tests/pane_dispatch_test.rs:325-340  check_membership_accepts_any_pane (Some("No Such Profile") -> Ok)
crates/holler-hub/tests/pane_handlers_test.rs:142      pane/cas_put of sample_pane("hj-c1r1", Some("Night Shift")) through the hook, with an empty profile registry
```
Build facts, checked on this branch: `holler-hub` already depends on `holler-pane`, `holler-proto`, `serde`, `serde_json`,
`tokio`, and dev-depends on `tempfile`, `libc` and `holler-pane-testkit` (so no manifest line is added; `rstest` is not a
dependency, use plain table loops). Every file under `src/panes/`, `src/profile/` and `tests/pane_*` is rustfmt-clean today.
`serve.rs` is 838 lines and `control_server.rs` 837 (not touched). Clippy denies `cognitive_complexity` (threshold 15) and
`too_many_lines` (100).

## Acceptance criteria

Each item names the test that proves it; T authors them RED first. "Store tests" drive `ProfileState` through the
`ProfileStore` trait from plain threads, no socket. "Handler tests" call `profile::dispatch` (or `panes::dispatch`)
directly and parse the line with `pane_support::outcome`.

**The conformance suites, and the rules the test kit fixes** (`tests/profile_registry_test.rs`)
1. [ ] `the_registry_passes_the_profile_store_conformance_suite`: `run_profile_store_conformance(|| { fresh temp dir;
   (ProfileState::load_with(&state, short_opts()), dir) })` returns `Ok(())`, all 23 cases.
2. [ ] `profiles_are_filed_by_slug_and_the_submitted_slug_is_ignored`: a `Profile` built with `slug: "wrong"` (struct
   literal, through the trait) is stored with `slug == name.slug()`; `get` of another spelling with the same slug
   (`"NIGHT-SHIFT"` for `"Night Shift"`) returns the stored record; the file's entry is keyed by the slug.
3. [ ] `the_name_rule_runs_before_the_generation`: with `Night Shift` at 1, `cas_put("NIGHT-SHIFT", 0)` and
   `cas_put("NIGHT-SHIFT", 1)` are both `profile-exists`, and nothing changes (record, log, file bytes, head).
4. [ ] `a_delete_is_logged_at_the_deleted_generation_plus_one`: create, update (2), delete at 2: the log is
   `[(1, Created), (2, Updated), (3, Deleted)]`.
5. [ ] `the_log_survives_delete_and_recreate_and_a_restart`: create, delete, create again, restart: `log` is
   `[(1, Created), (2, Deleted), (1, Created)]` before and after the restart, and `get` holds the second life at 1. `log`
   of a name never created is `profile-not-found`, also after a refused create at a non-zero generation.
6. [ ] `the_hub_stamps_the_times_and_ignores_the_submitted_ones`: a create submitted with `created: 5, updated: 6` is
   stored with `created == updated` within `[t0, t1]` (`now_millis()` read before and after the call); an update keeps
   `created` and moves `updated` (`>=` the old one); each log entry's `at` lies within its call's window, and `at` never
   decreases along a log (D4).

**Store: CAS and the change log**
7. [ ] `a_refused_write_changes_nothing_and_logs_nothing`: table over stale `cas_put`, `cas_put` ahead, stale `delete`,
   `delete` of a missing name, the same-slug create: each returns its code and leaves `get`, `list`, `log`, the file's
   bytes and the head cursor unchanged.
8. [ ] `an_update_summary_is_one_line_counting_specs`: an update from specs `[a, b]` to `[b', c]` logs
   `Updated { summary }` where summary is exactly `pane specs: 2 -> 2 (1 added, 1 removed, 1 changed)` (D5); an update
   that changes no spec logs `pane specs: 2 -> 2 (0 added, 0 removed, 0 changed)`. A spec pane name holding a newline
   cannot reach the summary (it holds counts only).
9. [ ] `the_log_cannot_be_rewritten`: after three writes, a fourth `log` read starts with the first three entries
   unchanged (prefix-equal). Over the wire (AC 30), a `profile/cas_put` whose `profile` carries a `log` member, or whose
   params carry an extra `log` member, is `usage` and the log and file are unchanged. There is no API that takes a log.

**Persistence** (`<state dir>/hub/profiles.json`, D2)
10. [ ] `records_logs_and_tombstones_survive_a_restart_unchanged`: write two profiles (one with two specs, an env name list
    and a `GridPos` `r2c1`), delete a third, drop the state, `ProfileState::load_with` the same dir: `list` is equal field
    for field, every `log` is equal, the deleted one stays deleted with its log, and the next write's event cursor is the
    pre-restart head + 1.
11. [ ] `the_file_is_written_atomically_at_mode_0600` (unix): after a write, `profiles.json` has mode `0o600`, parses as
    the D2 document, no `.profiles.json.*.tmp` is left, and a write succeeds when `<root>/hub` did not exist yet.
12. [ ] `loading_never_writes`: loading a valid file, and every read (`get`, `list`, `log`, `watch`), leaves the file's
    bytes and the dir listing unchanged; only an applied `cas_put` or `delete` writes (no timer, no per-heartbeat write).
13. [ ] `a_corrupt_file_fails_closed_and_is_never_rewritten`: a table over: not JSON; `version: 2`; a record with an
    unknown field; two entries with one slug; an entry whose `slug` differs from its record's slug; a tombstone whose
    event `name` has another slug; an entry with an empty `log`; a live entry whose last log generation differs from the
    record's; a tombstone whose last log entry is not `Deleted`. After `load`, each of `get`, `list`, `cas_put`, `delete`,
    `watch` and `log` returns `PaneError::StoreCorrupt` with a `what` naming the file (`profile registry <path>: ...`); the
    file's bytes are identical after the write attempts; nothing was moved aside or created.
14. [ ] `an_unreadable_file_fails_closed_and_is_never_rewritten` (unix, `0o000`): as AC 13.
15. [ ] `a_stored_secret_value_fails_closed_without_echoing_it` (I7 at rest): a hand-written file whose spec has
    `"env": ["TOKEN=SENTINEL-661"]` loads as `store-corrupt`; `SENTINEL-661` appears in neither the error's `what` nor its
    `Display` (the logged reason is built by the same `Problem`).
16. [ ] `an_unwritable_directory_refuses_the_write_and_keeps_the_old_state` (unix, hub dir `0o500`): `cas_put` and
    `delete` return `unavailable`; `get`, `log` and the head are unchanged; after the permission is restored the same
    call with the same `expected_generation` succeeds.
17. [ ] `the_two_registries_fail_independently`: a corrupt `profiles.json` leaves every `pane/*` method working for panes
    with `profile: None`; a corrupt `panes.json` leaves every `profile/*` method working.

**Concurrency, asserted as invariants (no sleeps)**
18. [ ] `racing_writers_exactly_one_wins`: 16 threads released by a `Barrier` each `cas_put` the same profile at the
    generation they all read: exactly one `Ok`, 15 `generation-conflict`, the generation is base + 1, the log gained
    exactly one entry, and a fresh `load` equals memory.
19. [ ] `concurrent_read_modify_write_loses_no_update`: 8 threads x 25 increments of the first spec's `context.soft`
    (get, change, `cas_put`, retry on conflict): `soft == base + 200`, generation `== base_gen + 200`, the log has 200 new
    `Updated` entries with consecutive generations.

**The change feed** (`tests/profile_feed_test.rs`)
20. [ ] `the_feed_delivers_every_write_exactly_once_in_order`: from cursor `c`, M writes including two updates of one
    profile, a delete and a re-create: exactly M events, cursors `c+1..=c+M`, each matching its write (a delete has
    `profile: None` and the stored display name); the next `next()` is `Ok(None)` within the short window.
21. [ ] `watch_from_zero_yields_every_current_profile_then_later_changes`, ordered by each entry's last cursor.
22. [ ] `a_waiting_watch_wakes_on_the_next_write` (bounded `recv_timeout`, no sleep).
23. [ ] `a_cursor_ahead_of_the_store_is_usage`.
24. [ ] `after_a_restart_an_old_cursor_gets_each_changed_profile_once_including_deletions` (feed rule 4, from tombstones).
25. [ ] `a_watcher_behind_the_retained_window_gets_the_compacted_changes` (`feed_retained: 2`).

**Wire handlers** (`tests/profile_handlers_test.rs`; data shapes per ADR-0021 §6)
26. [ ] `get_list_cas_put_delete_log_round_trip`: `profile/cas_put` returns the stored record; `profile/get` returns it,
    or `data: null` for an unknown name; `profile/list` an array; `profile/delete` `null`; `profile/log` an array of
    `ProfileLogEntry` oldest first, carrying the `actor` of each request. `profile/list` and `profile/watch` work with no
    `params` member.
27. [ ] `bad_params_answer_their_code`: missing `name` or `actor` is `usage`; an unknown params member is `usage`
    (`profile/list` with `{"x":1}` too); a stale generation is `generation-conflict`; a same-slug create is
    `profile-exists`; `profile/delete` and `profile/log` of a never-created name are `profile-not-found`.
28. [ ] `a_secret_value_is_refused_over_the_wire_and_never_echoed` (I7): `profile/cas_put` with a spec env
    `["TOKEN=SENTINEL-661"]` answers `profile-secret-refused`; `[" "]` answers `env-name-invalid`; the reply line does not
    contain `SENTINEL-661`; nothing is written (no file, no log). The hub adds no pre-scan of the raw JSON (the code comes
    from `EnvVarName`'s decode through `decode_params`; S checks there is no `"="` scan in `profile/`).
29. [ ] `profile_watch_answers_one_batch_and_its_cursor`: `data` parses as `WatchReply<ProfileEvent>`; idle replies
    `{"events": [], "cursor": <head>}` after the window.
30. [ ] `the_log_cannot_be_rewritten_over_the_wire` (the wire half of AC 9).
31. [ ] `two_cas_put_requests_racing_exactly_one_wins` (`#[tokio::test(flavor = "multi_thread")]`).
32. [ ] `a_corrupt_registry_answers_store_corrupt_on_every_profile_method`, as a JSON-RPC **result** carrying a
    `PaneReply`, never a JSON-RPC error. `profile/rename` still answers `not-implemented` (it is #665's, routed to
    `rename.rs`, which is not edited).
33. [ ] `a_detached_spec_is_accepted`: with pane `hj-c1r1` stored in profile `Night Shift`, `profile/cas_put` of profile
    `Day Shift` whose spec names `hj-c1r1` (and a spec naming a pane that does not exist) succeeds; the pane record's
    `profile` is still `Night Shift`. The profile store never reads the pane registry on a write.

**The membership rule** (`tests/pane_membership_test.rs` for the CAS half, `tests/profile_handlers_test.rs` for the hook)
34. [ ] `the_hub_pane_registry_passes_the_pane_store_conformance_suite`: `run_pane_store_conformance(|| (PaneState::
    load_with(&state, short_opts()), dir))` returns `Ok(())`, all 19 cases, `pane-in-other-profile` included. RED today
    fails exactly that one case (see Test plan).
35. [ ] `moving_a_pane_to_another_profile_is_refused_inside_the_cas`: through the `PaneStore` trait (no hook): a pane
    stored with `Night Shift` written with `Day Shift` at the current generation is `pane-in-other-profile` (the `what`
    names both profiles); the record, file bytes and head are unchanged. Leaving (`None`) then joining `Day Shift`
    succeeds at 2 and 3. Keeping the profile, and a spelling with the same slug (`night shift`), are allowed.
36. [ ] `a_stale_generation_wins_over_the_profile_rule`: the same move at a stale generation is `generation-conflict`
    (the CAS is checked first, as in the fake).
37. [ ] `two_writers_joining_different_profiles_exactly_one_wins_and_the_loser_is_refused_on_retry`: from `None` at 1,
    two threads write `Night Shift` and `Day Shift` at 1 (Barrier): one `Ok`, one `generation-conflict`; the loser
    re-reads and retries at 2 and gets `pane-in-other-profile`. This is why the check sits under the pane lock.
38. [ ] `pane_cas_put_naming_a_missing_profile_is_profile_not_found` (handler test via `panes::dispatch`): nothing is
    written; with the profile created first, the same request succeeds.
39. [ ] `a_pane_outside_any_profile_never_reads_the_profile_registry`: with a corrupt `profiles.json`, `pane/cas_put` of a
    pane with `profile: None` succeeds, and one naming a profile answers `store-corrupt` (fail closed, never accepted).
40. [ ] `check_membership` (amended #669 test `check_membership_requires_the_named_profile_to_exist`): `None` is `Ok`;
    `Some("No Such Profile")` is `ProfileNotFound`; after creating it, `Ok`.
41. [ ] S checks with `grep -n "PaneInOtherProfile" crates/holler-hub/src/panes/store.rs` (one hit, in the CAS path) and
    `grep -rn "PaneInOtherProfile" crates/holler-hub/src/profile/` (no hit: the hook does not do the comparison).

**#639's and #669's tests, amended (D11)**
42. [ ] `pane_dispatch_test.rs`: `fresh_deps` loads `ProfileState::load_with(&state, short_opts())`; the profile-method
    table (renamed `every_profile_method_is_forwarded_to_the_registry_not_method_not_found`) expects `profile/list` ->
    `Ok(Some([]))`, `profile/watch` -> `{"events": [], "cursor": 0}`, `profile/get|cas_put|delete|log` -> `usage`,
    `profile/rename` -> `not-implemented`; `profile/list` in the two-connection test -> `Ok(Some([]))`; the direct
    dispatch test expects `profile/get` -> `usage` and `profile/rename` -> `not-implemented`; AC 40 replaces
    `check_membership_accepts_any_pane`. The module doc's "every `profile/*` assertion is unchanged" is updated. Every
    `pane/*` assertion is unchanged.
43. [ ] `pane_handlers_test.rs`: the round-trip test creates profile `Night Shift` in the rig's profile registry before it
    writes the pane that names it. No other assertion changes.

**Build guards**
44. [ ] `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
    `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` all pass. Every new `.rs` file passes
    `rustfmt --check --edition 2021` and every touched file stays rustfmt-clean. No file reaches 600 lines. Every
    `#[allow]` carries a trailing `// #661`. `CATALOG.len()` is still 22; no golden file and no `docs/protocol/v2.md`
    change. `CHANGELOG.md` `## [Unreleased]` has an `### Enhancements` entry linking #661 (text under Files).
45. [ ] `git diff --name-only origin/main...HEAD` lists only Blast-radius paths: no `serve.rs`, `control_server.rs`,
    `pane_dispatch.rs`, `lib.rs`, `profile/rename.rs`, `holler-pane`, `holler-proto`, `holler-pane-testkit`, any
    `Cargo.toml` or `Cargo.lock`, any ADR.

## Files

Production, `crates/holler-hub/src/`:
- `profile/mod.rs` (edit, 69 -> about 190 lines): module docs (the layout, the rules, the operating limits, mirroring
  `panes/mod.rs`); `pub struct ProfileState { store: Arc<store::Store> }` (no `Clone`); `ProfileState::load(state)` =
  `load_with(state, PaneStoreOptions::default())`; `pub fn load_with(state: &HubState, options: PaneStoreOptions)`;
  `impl ProfileStore for ProfileState` (each method delegates to `Store`; `rename` returns `Err(NotImplemented)` and touches
  nothing); `dispatch` routes the six methods to `handlers` and `profile/rename` to `rename::dispatch` (signature
  unchanged); `check_membership` filled (D8). Declares `mod entry; mod handlers; mod store;` beside `pub mod rename;`.
- `profile/store.rs` (new, about 400 lines): `Table { head, entries: BTreeMap<String /*slug*/, ProfileEntry>, ring:
  Ring<ProfileEvent> }`; `Store { path, options, table: Mutex<Result<Table, PaneError>>, changed: Condvar }`; `open`,
  `get`, `list`, `cas_put`, `delete`, `log`, `commit`, `save`, `poll`, the `Feed` iterator, `load_table`. Label
  `"profile registry"`, log events `profile_registry_corrupt` and `profile_registry_write_failed` through #639's
  `log_fault`.
- `profile/entry.rs` (new, about 130 lines): the file's entry type `ProfileEntry` (D2) with its load checks (serde
  `try_from`), `impl RegistryEntry` for `ProfileEntry` and for `ProfileEvent`, the stamp clamp (D4) and the update
  summary (D5).
- `profile/handlers.rs` (new, about 110 lines): one async fn per method, each `crate::panes::handlers::run` with a closure
  over a clone of the `Arc<Store>`.
- `panes/store.rs` (edit, about +15 lines): the membership comparison inside `cas_put` (D7), and `log_fault` becomes
  `pub(crate)` (D10).
- `panes/mod.rs` (edit, one word): `mod store;` -> `pub(crate) mod store;` (D10).
- `panes/handlers.rs` (edit, one word): `struct NoParams` -> `pub(crate) struct NoParams` (D10).

Tests, `crates/holler-hub/tests/`:
- `profile_registry_test.rs` (new, AC 1-19).
- `profile_feed_test.rs` (new, AC 20-25).
- `profile_handlers_test.rs` (new, AC 26-33, 38-39).
- `pane_membership_test.rs` (new, AC 34-37). It must compile against today's code (it uses only `PaneState`, the
  `PaneStore` trait and the test kit), so its RED is an assertion RED.
- `pane_support/mod.rs` (extend, about +80 lines): profile helpers that reuse `short_opts()`: `load_profiles(state)`,
  `profiles_file(state)`, `actor()`, `profile(name, panes)` (wrapping `holler_pane_testkit::fixture::sample_profile`),
  `create_profile(store, name)`, `drain_profiles(watch)`. Its module doc gains the profile tests as includers.
- `pane_dispatch_test.rs` (amend, AC 42), `pane_handlers_test.rs` (amend, AC 43).

`CHANGELOG.md`, `## [Unreleased]` / `### Enhancements`, one entry:
> The hub's profile registry ([#661](https://github.com/Performant-Labs/holler/issues/661), epic #633): profiles are kept in
> `<state dir>/hub/profiles.json` (mode 0600, written atomically), filed by slug, every write a compare-and-swap on the
> generation, with an append-only change log per profile and a change feed, behind the `profile/get`, `list`, `cas_put`,
> `delete`, `watch` and `log` control methods. A corrupt or unreadable file fails closed (`store-corrupt`) and is never
> rewritten; a file written by a newer build with an unknown field fails closed too. A pane write that names a profile
> that does not exist is refused (`profile-not-found`), and moving a pane from one profile to another in one write is
> refused (`pane-in-other-profile`): it must leave its profile first.

### Reuse map (extend, do not duplicate)

Closest analogous feature: **the #639 pane registry** (`crates/holler-hub/src/panes/`). Recommendation: **extend** it. The
profile registry is a second instance of the same object shape, built from #639's shared parts; the only new object is the
profile `Store`, justified below.

| Need | Reuse |
|---|---|
| File format, load, save, atomic write, 0600, never-quote-the-file | `panes::persist::{Doc, VERSION, load_doc, save_doc, Problem}` through `RegistryEntry` (no second loader, no second writer) |
| Feed rules 1-4, ring, cursor-ahead check | `panes::feed::{Ring, select, check_since}` |
| Handler pipeline (params, decode, `spawn_blocking`, reply) | `panes::handlers::run` and `NoParams`; `pane_dispatch::reply_line` |
| Store options and long-poll window | `panes::PaneStoreOptions` and `panes::WATCH_WAIT` (D3) |
| CAS rule | `holler_pane::next_generation` (never re-implemented) |
| Params types | `holler_pane::reply::{ProfileGetParams, ProfileCasPutParams, ProfileDeleteParams, ProfileLogParams, WatchParams, WatchReply}` |
| No-secret guard | `EnvVarName`'s decode inside `decode_params` (epic ruling 7: no raw-JSON pre-scan) |
| Fault logging | `panes::store::log_fault` |
| Clock | `holler_proto::clock::now_millis` |
| Mutex poisoning | `lock().unwrap_or_else(PoisonError::into_inner)` |
| Test helpers | `pane_support` (`temp_state`, `short_opts`, `outcome`, `dir_listing`), `holler_pane_testkit::fixture::sample_profile` / `sample_spec`, both conformance runners |

**New object, justified:** `profile::store::Store` mirrors `panes::store::Store`'s shape (lock, `read`, `commit`, `save`,
`poll`, the `Feed` iterator, about 60 lines that differ only in the table and event types). #639's `Store` is concrete over
the pane table; making it generic would rewrite #639's working store, which the issue limits to one comparison. The
mirrored `poll` loop and `Feed` iterator carry a doc line naming their pane twin. A-dup: this mirroring is the brief's
decision (D10), not a parallel path; anything beyond it (a second loader, writer, select, ring, CAS helper, params decoder,
reply builder, or a copy of a `pane_support` helper) is a BLOCK.

## Decisions already made (epic, issue, ADR)

- The hub is the store only; `profile/*` are get/list/cas_put/delete/watch/log; `profile/rename` stays PROPOSED and
  `rename.rs` is #665's (epic ruling 1, ADR "Decisions taken" 4).
- Replies are a `PaneReply` in a JSON-RPC result; `*/watch` is long-poll (ADR §6). `pane/*` and `profile/*` are
  control-socket methods outside the 22-row catalog: no `docs/protocol/v2.md` change (ruling 6).
- The file is `<state dir>/hub/profiles.json`, 0600, `write_atomic`, no fsync, `"version": 1`; the log is persisted with
  the records, never in a `Profile` (ADR §7). Fail closed, file left in place (ADR §7).
- Writes carry an `Actor`; the log entry is `{at, generation, actor, change}` (ruling 8).
- One env guard, `EnvVarName` (ruling 7).
- `pane-in-other-profile` runs in the pane CAS under the pane lock; the hook keeps the existence check (ADR "Decisions
  taken" 2; issue amendment 2026-10-09). A detached spec is not refused (issue amendment 2026-10-08).
- The test kit's four store rules (Evidence) and its two conformance suites are the contract.
- No new error code (ruling 3): `profile-exists`, `profile-not-found`, `pane-in-other-profile`,
  `profile-secret-refused`, `env-name-invalid`, `generation-conflict`, `store-corrupt`, `unavailable`, `usage` are all
  closed variants already.

## Decisions made in this brief (for operator review where flagged)

- **D1. Store shape and locking** (as #639's D1). One `std::sync::Mutex` over the whole table, one `Condvar`. A write runs
  under the lock: the name rule, `next_generation`, build the record and its log entry, build the next document,
  `save_doc`, then commit to memory, push the event, `notify_all`. A failed save changes nothing. No `.await` under the
  lock. The profile store never takes the pane lock.
- **D2. File layout.** `{"version": 1, "cursor": <head>, "entries": [ProfileEntry...]}`, entries sorted by slug, where
  `ProfileEntry = {"slug": "<slug>", "event": {<ProfileEvent>: cursor, name, profile or null}, "log": [<ProfileLogEntry>...]}`
  (`deny_unknown_fields`). `RegistryEntry for ProfileEntry`: `name()` = slug, `cursor()` = `event.cursor`, `record()` =
  the record's `(slug, generation)`, so #639's `check` enforces unique slugs, unique cursors, the cursor range, record slug
  == entry slug and generation >= 1. `ProfileEntry`'s serde `try_from` adds: `event.name.slug() == slug` (tombstones too),
  a non-empty `log`, a live record's last log generation equal to its generation, and a tombstone's last log entry
  `Deleted`. A violation is a serde data error, so `Problem::parse` reports only its line and column. In memory the table
  keeps the same `ProfileEntry` per slug; the ring holds `ProfileEvent`s only (no logs), and `select` gets
  `entries.values().map(|e| &e.event)` (`RegistryEntry for ProfileEvent` uses `cursor()` and `record().is_some()`). The
  layout is documented in `profile/entry.rs`'s module docs (ADR-0021 delegates it to this story; no ADR edit).
- **D3. Options.** `ProfileState::load_with` takes #639's `PaneStoreOptions` (the fields are registry-generic) and uses
  `WATCH_WAIT` (4 s) and a 1024-event ring by default. No second options type.
- **D4. Timestamps (operator review).** The hub stamps them; the submitted `created`, `updated`, `slug` and `generation`
  are ignored. A create sets `created = updated = now`; an update keeps `created` and sets `updated = max(now, stored
  updated)`; a re-create after a delete starts a new `created`. A log entry's `at = max(now, last at of that log)`, so a
  clock step backwards never makes a log's `at` decrease (the suite requires non-decreasing). `now` is
  `holler_proto::clock::now_millis()`.
- **D5. Update summary.** `pane specs: <before> -> <after> (<a> added, <r> removed, <c> changed)`, specs matched by
  `ProfileSpec.pane`. Counts only: one line by construction, and no free text from a request reaches the log. A write of
  the same name is the only kind of update (a different name with the same slug is `profile-exists`; a rename is #665).
- **D6. Lookups by slug.** `get`, `delete` and `log` look a name up by its slug, as the fake does, so `get("NIGHT-SHIFT")`
  returns `Night Shift`. A write requires the exact stored name (the name rule) against the **live** record only: after a
  delete, a same-slug create under a new spelling is allowed and its log continues the old one. `list` is in slug order
  (not pinned). A delete's event carries the stored display name.
- **D7. The in-CAS rule.** In `panes::store::Store::cas_put`, after `next_generation` and before building the event:
  `(Some(stored), Some(next))` with `stored.slug() != next.slug()` is `PaneInOtherProfile { what: "<pane> is in profile
  \"<stored>\", not \"<next>\"" }`. Generation first, then the rule, as the fake. It applies on every path into the pane
  CAS (the trait and `pane/cas_put`). Compared by slug, consistent with the profile store's identity.
- **D8. The hook.** `check_membership(pane, profiles)`: `pane.profile == None` -> `Ok` without touching the profile
  registry; otherwise `profiles.store.get(name)?` -> `Some` is `Ok`, `None` is `ProfileNotFound { what: name }`, a failed
  registry propagates `store-corrupt`. It runs where #639 put it: on the blocking pool, before the pane lock is taken, so
  the lock order of ADR §7 holds and no lock is nested.
- **D9. The existence check is strict (operator review).** Every `pane/cas_put` naming a profile requires it to exist, not
  only one that changes `Pane.profile` (the hook cannot see the stored pane). Consequence: after a profile is deleted, a
  pane still naming it cannot be rewritten (for example by reconcile, #647) until its `profile` is cleared. #662's
  `delete --keep-panes` therefore clears `Pane.profile` on the member panes; this is noted for #662 and #647 in the
  forward-compat table. The alternative (check existence only on a change) would need the stored pane in the hook, which
  ADR "Decisions taken" 2 rules out.
- **D10. Small edits in #639's code (operator review).** Besides the D7 comparison, two visibility changes so that nothing
  is copied: `NoParams` and `log_fault` become `pub(crate)` and `mod store` becomes `pub(crate) mod store` in
  `panes/mod.rs`. No #639 logic changes beyond D7. The `poll`/`Feed` mirroring in `profile/store.rs` is accepted (see the
  Reuse map) rather than generalising #639's store in this run; a follow-up may make them generic.
- **D11. Blast radius widened by test files.** `pane_dispatch_test.rs` (#669) and `pane_handlers_test.rs` (#639) assert
  the stubs or write a pane naming a profile that does not exist; only the assertions in Evidence change.
  `pane_support/mod.rs` gains profile helpers instead of a second support module.
- **D12. Rename is not pre-built (operator review).** #661 adds no rename primitive to the profile `Store`. When #665 is
  confirmed, its hub transaction will need one (and a pane-side bulk update), so #665 will edit `profile/store.rs`, not
  only `rename.rs` as #669 planned. Building it now would be code for a PROPOSED story.

## Forward compatibility

| Consumer | Needs | Satisfied |
|---|---|---|
| #649 (CLI client of `profile/*`) | ADR §6 data shapes; `WATCH_WAIT` to size its timeout | yes (AC 26, 29; the window is `panes::WATCH_WAIT`) |
| #662 (`profile create/delete/list/show`) | CAS, `profile-exists`, `profile-not-found`, log via `log()` | yes; note D9: `delete --keep-panes` must clear `Pane.profile` on member panes |
| #663 (`ProfileScope`, I8 order) | CAS on generation; a compensating write logs `Updated` with specs equal | yes (AC 8: a restoring write logs a normal `Updated`) |
| #664, #667 (`profile apply`) | `pane-in-other-profile` refusal unless `--take-over` (which leaves then joins) | yes (AC 35: leave then join is allowed) |
| #647 (reconcile) | pane writes keep working for panes in a profile | yes while the profile exists; needs discussion per D9 for a deleted profile |
| #650 (import creates `fleet`) | `profile/cas_put` at 0 | yes |
| #665 (rename, PROPOSED) | an atomic rename across both registries | needs discussion (D12) |

## Out of scope

- `profile/rename` and `ProfileStore::rename` beyond `not-implemented` (#665). `profile-has-live-panes` (#662, CLI side).
- The client side of `profile/*` and the real `ProfileScope` (#649, #663); any CLI verb.
- Validating a profile's specs against each other (duplicate panes or grid cells) or against the pane registry.
- Pruning tombstones or logs; fsync; generalising #639's `Store`.
- Any change to `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `holler-pane`, `holler-proto`, the test
  kit, ADR-0021 or `docs/protocol/v2.md`.

## Test plan

**RED (T):**
- `pane_membership_test.rs` compiles today. Run `cargo test -p holler-hub --test pane_membership_test`: AC 34 must fail
  with exactly one failing case, `pane-in-other-profile`, and AC 35-37 fail because the move is accepted. **If any other
  pane conformance case fails, T stops the run with `escalate`, naming the case ids**: that is a #639 defect outside this
  story, not something to fix here.
- `profile_registry_test.rs`, `profile_feed_test.rs`, `profile_handlers_test.rs`, the amended `pane_dispatch_test.rs` and
  `pane_handlers_test.rs` do not compile until `ProfileState::load_with` and `impl ProfileStore for ProfileState` exist:
  that is their RED. T confirms no RED comes from a typo or a missing helper (`cargo test -p holler-hub --no-run` errors
  name only `load_with` and the `ProfileStore` impl).

**GREEN (F), in order:**
1. `profile/entry.rs`, `profile/store.rs`, `ProfileState` and the trait impl: AC 1-19.
2. The feed (`poll`, `Feed`): AC 20-25.
3. `profile/handlers.rs` and `dispatch`: AC 26-33, 42.
4. D7 in `panes/store.rs`, D8 in `check_membership`, D10 visibility: AC 34-41, 43.
5. AC 44-45 and the full `cargo test --workspace`.

Permission tests (AC 11, 14, 16) are `#[cfg(unix)]` and follow `pane_registry_test.rs`. Every wait is bounded
(`recv_timeout`, `tokio::time::timeout`, or `short_opts()`'s 100 ms window); no `thread::sleep`.

## Risks

- **Whole-file rewrite with growing logs.** Every write rewrites `profiles.json` including every log. Profile writes are
  operator-paced (verbs, not heartbeats), so this is fine; recorded in the module's operating limits.
- **Time-of-check gap across registries.** The hook checks the profile exists, then the pane CAS writes; a profile
  deleted in between leaves a pane naming a deleted profile. ADR §8 has no cross-registry transaction; reconcile is the
  net. Recorded, not fixed.
- **ABA after delete and re-create** (ADR §8 known gap): a writer holding generation 1 of the old life can overwrite the
  new one. Pinned by the suite, not fixed.
- **File-mode tests as root** bypass mode bits; CI runs as non-root (#639 relies on the same).
- **Complexity limits.** Keep the name rule, the record build, the log entry and the summary in small functions; keep
  `ProfileEntry`'s load checks in `entry.rs`.
- **`deny_unknown_fields`.** A file from a newer build with a new field fails closed as `store-corrupt` (intended; the
  CHANGELOG says so).

## Blast radius

`crates/holler-hub/src/profile/{mod.rs, store.rs, entry.rs, handlers.rs}`; `crates/holler-hub/src/panes/store.rs` (D7,
D10), `panes/mod.rs` and `panes/handlers.rs` (D10, one word each); `crates/holler-hub/tests/profile_registry_test.rs`,
`profile_feed_test.rs`, `profile_handlers_test.rs`, `pane_membership_test.rs` (new), `pane_support/mod.rs` (extend),
`pane_dispatch_test.rs` and `pane_handlers_test.rs` (D11 only); `CHANGELOG.md`; `docs/handoffs/661*` (pipeline artifacts).

Not changed: `profile/rename.rs`, `serve.rs`, `control_server.rs`, `pane_dispatch.rs`, `lib.rs`, `holler-pane`,
`holler-proto`, `holler-pane-testkit`, `holler-cli`, any `Cargo.toml`, `Cargo.lock`, any golden file, any ADR,
`docs/protocol/`.
