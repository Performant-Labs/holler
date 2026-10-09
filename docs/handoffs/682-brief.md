# Brief: #682 the pane test kit, slice c (part 1): `FakeProfileStore` and the `ProfileStore` conformance suite

Repo: Performant-Labs/holler. Issue: #682 (slice c of #638, epic #633). Rigor: in-session. UI surface: no. Kind: feature
(test kit).

**Branch:** `issue-682-implementation`, based on `e410e9d` (`origin/main`: #637, #669, #670, ADR-0021, #639, #676 and slice a
of #638 merged). **Design (D):** N/A. **Decision record:** ADR-0021 sections 5, 7 and 8 and its "Decisions taken" items 1
and 4. The issue is the source of truth. Where this brief differs from it, the section "Decisions made in this brief" says
so and why.

## Size check and split

**The issue as written does not fit one run.** Slice a of #638 was estimated at ~1,550 lines and merged at 2,118 (src 1,381,
tests 737: `fault.rs` 134, `feed.rs` 260, `fixture.rs` 73, `pane_store.rs` 259, `conformance/mod.rs` 137,
`conformance/pane_store.rs` 518, `tests/` 426 + 311). Using those actual sizes, slice c comes out as follows:

| Part | Files | Lines (est.) |
|---|---|---|
| `FakeProfileStore` (log, clock, name rule, 8 ops) | `src/profile_store.rs` | ~360 |
| `ProfileStore` suite, 23 cases | `src/conformance/profile_store.rs` + `src/conformance/profile_store/watch.rs` | ~700 |
| profile fixtures, reuse refactors (`Writer`, `lock`, generic watch helpers) | `fixture.rs`, `feed.rs`, `pane_store.rs`, `conformance/pane_store.rs` | ~+80 net |
| tests: the store suite + 6 mutants, the fake's own mechanisms | 2 test files | ~640 |
| **Part 1 subtotal (this run)** | | **~1,780** |
| `FakeProfileScope` (resolve, I8 `edit_spec`) | `src/profile_scope.rs` | ~220 |
| `ProfileScope` suite, 14 cases | `src/conformance/profile_scope.rs` | ~450 |
| tests: the scope suite + 2 mutants, the fake's own | 2 test files | ~350 |
| **Part 2 subtotal (next run)** | | **~1,020** |
| **Total** | | **~2,800** |

At ~2,800 lines the whole slice is a third larger than slice a, which was already at the top of what one run holds. **This
run builds part 1 only:** `FakeProfileStore`, `run_profile_store_conformance`, the profile fixtures and the store-side
mutation check. **Part 2** is `FakeProfileScope` and `run_profile_scope_conformance` with the I8 cases and the two scope
mutants. Its API and its case table are settled in the appendix "Fixed for part 2" so that its O run starts from them. Part 2
needs part 1 (it seeds `FakeProfileStore` and uses `sample_profile`). #661 needs part 1 only. #663 needs part 2. **This PR
says "Part of #682", not "Closes #682".** See "Needs operator".

## Problem

`holler-pane-testkit` has a fake `PaneStore` and its suite (slice a). Its profile modules are empty stubs. The hub's profile
registry (#661) has to run a `ProfileStore` conformance suite against itself, and every verb story that reads or writes a
profile needs an in-memory `ProfileStore` with fault injection. This run adds both, mirroring `FakePaneStore` and
`run_pane_store_conformance` in structure and naming. It reuses the shared fault switch and change feed instead of copying them.

## Evidence (verbatim, as of `e410e9d`)

The port (frozen by #637):
```
crates/holler-pane/src/profile.rs:328-371
pub trait ProfileStore: Send + Sync {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;
    fn list(&self) -> Result<Vec<Profile>, PaneError>;
    /// Store `profile` if the stored one is still at `expected_generation` (0 for a
    /// new profile); returns the stored record with its bumped generation.
    fn cas_put(&self, profile: &Profile, expected_generation: u64, actor: &Actor) -> Result<Profile, PaneError>;
    /// ... A stale generation is `generation-conflict`; a profile that does not exist is
    /// `profile-not-found`, whatever `expected_generation` is (a missing profile is checked first ...).
    fn delete(&self, name: &ProfileName, expected_generation: u64, actor: &Actor) -> Result<(), PaneError>;
    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError>;
    /// The change log of the profile `name`, oldest first.
    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>;
    /// PROPOSED (#665): ... Until #665 is confirmed an implementation answers `not-implemented`.
    fn rename(&self, from: &ProfileName, to: &ProfileName, expected_generation: u64, actor: &Actor) -> Result<Profile, PaneError>;
}
crates/holler-pane/src/profile.rs:207-220   pub struct Profile { pub name: ProfileName, pub slug: String, pub generation: u64,
                                              pub panes: Vec<ProfileSpec>, pub created: i64, pub updated: i64 }   // no `log` field
crates/holler-pane/src/profile.rs:276-289   pub enum ProfileChange { Created, Updated { summary: String }, Renamed { from: ProfileName }, Deleted }
crates/holler-pane/src/profile.rs:293-302   pub struct ProfileLogEntry { pub at: i64, /// The generation the profile had after the write.
                                              pub generation: u64, pub actor: Actor, pub change: ProfileChange }
crates/holler-pane/src/profile.rs:306-317   pub struct ProfileEvent { pub cursor: Cursor, pub name: ProfileName, pub profile: Option<Box<Profile>> }  // None = deleted
crates/holler-pane/src/profile.rs:72-74     pub fn slug(&self) -> String   // ASCII alnum lower-cased, other runs -> one `-`
crates/holler-pane/src/profile.rs:111-140   pub struct Actor(String); pub fn parse(text: &str) -> Result<Self, PaneError>
crates/holler-pane/src/profile.rs:178-200   pub struct ProfileSpec { pub pane: String, pub herdr: SpecHerdr, pub host: SpecHost,
                                              pub harness: SpecHarness, pub model: ModelSpec, pub role: PaneRole,
                                              pub env: Vec<EnvVarName>, pub context: ContextCeilings,
                                              pub command: Option<Argv>, pub check: Option<Argv>, pub expect: Vec<String> }
crates/holler-pane/src/generation.rs:25-34  pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError>  // Conflict when !=, overflow store-corrupt
crates/holler-pane/src/argv.rs:97-105       EnvVarName::parse: a `=` anywhere -> ProfileSecretRefused; empty, whitespace or control -> EnvNameInvalid
crates/holler-pane/src/error.rs:406,425,434,437   NotImplemented, Conflict ("generation-conflict"), ProfileNotFound { what }, ProfileExists { what }
```
The decided store semantics:
```
docs/adr/ADR-0021.md:268-272  A record that does not exist is at generation 0, so a create names `expected_generation: 0` and is
  stored at 1. Each applied write adds one. The store sets the generation; the one a client submits is ignored. ...
  A write whose expected generation is not the current one is `generation-conflict` and changes nothing. `delete` checks
  that the record exists first (`pane-not-found` or `profile-not-found`, whatever the generation) and the generation second.
docs/adr/ADR-0021.md:245      The profile change log is persisted in `profiles.json` with the records, never inside a `Profile`.
docs/adr/ADR-0021.md:184-186  `holler-pane-testkit` ... may depend on `holler-pane` and `serde_json`, and **must not depend on `holler-cli`**
docs/adr/ADR-0021.md:544-545  4. **`profile rename`, ... `ProfileStore::rename` ... (#665) stay PROPOSED** until wave 4.
```
What slice a left, which this run extends and reuses (all under `crates/holler-pane-testkit/`):
```
src/lib.rs:51-62                 pub mod conformance; ... mod feed; pub mod fixture; ... pub mod profile_scope; pub mod profile_store;
src/profile_store.rs:1-3         //! `FakeProfileStore`, the in-memory `ProfileStore` ... Empty stub ...; slice c (#682) fills it.
src/conformance/profile_store.rs:1-2   //! The `ProfileStore` conformance suite. Empty stub ...; slice c (#682) fills it.
src/fault.rs:81-85               pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static { fn as_str(self) -> &'static str; }
src/fault.rs:103-170             FaultSwitch<Op>: new, set(Option<Fault>), fail_next(op, error), set_delay, calls(), pub(crate) enter(op)
src/feed.rs:40-54                pub(crate) trait Change: Clone + Send + 'static { type Key: Ord + Send + 'static; type Record;
                                   fn key(&self) -> Self::Key; fn cursor(&self) -> Cursor; fn record(&self) -> Option<&Self::Record>; }
src/feed.rs:6-8                  //! ... the fake profile store (#682) implements it for `ProfileEvent`, so the crate has one feed.
src/feed.rs:69-71, 81-95         Log::get(&key); Log::append(make: impl FnOnce(Cursor) -> E) (overflow is store-corrupt, changes nothing)
src/feed.rs:140-192              Feed::new, set_idle_wait, read, write (wakes watchers on Ok), watch(&Arc<Self>, since, &Arc<FaultSwitch<Op>>, next_op)
src/feed.rs:218-220              fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T>   // private; poison taken over
src/pane_store.rs:83-100         enum Writer { Port(u64), Other } + fn expected(self, current: u64) -> u64   // private
src/pane_store.rs:150-169        put: stored = log.get(name); next_generation(current, writer.expected(current))?; membership if Port; append
src/pane_store.rs:173-188        remove: missing -> PaneNotFound first, then next_generation, then append { pane: None }
src/pane_store.rs:224-239        impl Change for PaneEvent { type Key = PaneName; type Record = Pane; ... }
src/fixture.rs:11,15,24          const SAMPLE_PORT = 48100; const SCRATCH = "scratch"; pub fn sample_pane(name) -> Result<Pane, PaneError>
src/conformance/mod.rs:47-137    pub(crate) run_cases, succeeds, expect_code, expect_eq, next_item, drain; CaseFailure, Conformance
src/conformance/pane_store.rs:419-421   fn profile_name(text) -> Result<ProfileName, String>              // private today
src/conformance/pane_store.rs:479-518   fn expect_change(&mut Watch<PaneEvent>, &PaneName, Option<&Pane>, after) -> Result<Cursor, String>;
                                        fn changes(Vec<PaneEvent>) -> Vec<(PaneName, Option<Pane>)>; fn cursors(&[PaneEvent]) -> Vec<Cursor>;
                                        fn increasing(&[Cursor]) -> Result<(), String>                  // private, PaneEvent-typed today
tests/pane_store_conformance_test.rs:153-263   the mutant pattern: enum Break, struct Mutant wrapping the fake, assert_suite_fails_on
```
The env guard's JSON-decode leg is already pinned in `holler-pane` (the testkit has no `serde_json` until #681):
```
crates/holler-pane/tests/argv_env_test.rs:96-102
fn no_profile_spec_or_pane_field_can_hold_an_environment_value() {
    let mut spec = common::spec_json();
    spec["env"] = json!(["ANTHROPIC_API_KEY", "TOKEN=hunter2"]);
    let msg = err_text(serde_json::from_value::<ProfileSpec>(spec));
    assert!(msg.contains("profile-secret-refused"), "{msg}");
crates/holler-pane/tests/argv_env_test.rs:57-69   env_var_name_accepts_names_and_refuses_values_and_blanks (the blank and `=` codes)
```
Lint conventions:
```
Cargo.toml:19-30      unwrap_used, expect_used, panic, unreachable, cognitive_complexity, too_many_lines, struct_excessive_bools = "deny"; dead_code = "deny"
clippy.toml:6-7       cognitive-complexity-threshold = 15; too-many-lines-threshold = 100
scripts/lint.sh:43-52 warn at 600 lines per .rs file, fail at 900
tests/pane_store_conformance_test.rs:1   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
```

## Dependency direction

`holler-pane-testkit` -> `holler-pane` only. **`crates/holler-pane-testkit/Cargo.toml` does not change** in this run: no
`serde_json`, which #681 (slice b, running in parallel) adds to the same `[dependencies]` block, and no `[dev-dependencies]`.
The testkit never names `holler-cli`, `holler-hub` or an adapter crate. The hub's profile code (`crates/holler-hub/src/profile/`)
is still a stub. It was read for reference only and nothing depends on it.

## Public API of part 1 (exact; T writes tests against these, F implements them)

No flat re-exports. Every item is reached by its module path, and `lib.rs` and `conformance/mod.rs` are not edited.

```rust
// crates/holler-pane-testkit/src/profile_store.rs
/// A method of the `ProfileStore` port, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileStoreOp { Get, List, CasPut, Delete, Watch, /** one next() of an open watch */ WatchNext, Log, Rename }
impl PortOp for ProfileStoreOp {
    // "profile_store.get", "profile_store.list", "profile_store.cas_put", "profile_store.delete",
    // "profile_store.watch", "profile_store.watch_next", "profile_store.log", "profile_store.rename"
    fn as_str(self) -> &'static str;
}

/// An in-memory `ProfileStore`. Not `Clone`: share it behind an `Arc` (a copy would split the store).
pub struct FakeProfileStore {
    feed: Arc<Feed<ProfileEvent>>,
    faults: Arc<FaultSwitch<ProfileStoreOp>>,
    /// Each profile's change log, by slug, oldest first. Locked only inside a `feed.write` (feed lock first).
    history: Mutex<BTreeMap<String, Vec<ProfileLogEntry>>>,
    /// The clock every write stamps (`created`, `updated`, the log entry's `at`), in ms since the epoch.
    now: AtomicI64,
}
impl FakeProfileStore {
    pub fn new() -> Self;                                    // empty, no fault, idle wait zero, clock 0
    /// Each profile created at expected generation 0 (stored at 1, one `Created` entry by `actor`), in order.
    /// Bypasses the faults and the call log. A second seed with the same name is `generation-conflict`; one with
    /// another name and the same slug is `profile-exists`.
    pub fn seeded(profiles: impl IntoIterator<Item = Profile>, actor: &Actor) -> Result<Self, PaneError>;
    pub fn set_idle_wait(&self, wait: Duration);             // = FakePaneStore::set_idle_wait
    /// Set the clock: every later write stamps `millis` (default 0, so a test is deterministic).
    pub fn set_now(&self, millis: i64);
    pub fn faults(&self) -> &FaultSwitch<ProfileStoreOp>;
    /// Another writer stores `profile` at the stored generation + 1 (1 for a new profile), without the name
    /// rule, logs its entry with `actor` and publishes its event. Bypasses the faults and the call log.
    pub fn concurrent_put(&self, profile: &Profile, actor: &Actor) -> Result<Profile, PaneError>;
    /// Another writer deletes `name` (logs `Deleted` with `actor`, publishes `profile: None`).
    /// `profile-not-found` when missing. Bypasses the faults and the call log.
    pub fn concurrent_delete(&self, name: &ProfileName, actor: &Actor) -> Result<(), PaneError>;
}
impl Default for FakeProfileStore { /* = new() */ }
impl ProfileStore for FakeProfileStore { /* see "Fake behaviour" */ }
impl Change for ProfileEvent { type Key = String /* the slug */; type Record = Profile; /* key = name.slug(), record = profile.as_deref() */ }

// crates/holler-pane-testkit/src/conformance/profile_store.rs
/// The ids of the cases `run_profile_store_conformance` runs, in order (the table below).
pub fn profile_store_cases() -> Vec<&'static str>;
/// Run every case, each against a fresh, empty store; `fresh` returns the store and a guard kept alive for that
/// case only (a `TempDir` for the hub, `()` for the fake). Same contract as `run_pane_store_conformance`.
pub fn run_profile_store_conformance<S, K, F>(fresh: F) -> Conformance
where S: ProfileStore, F: FnMut() -> (S, K);
mod watch;   // src/conformance/profile_store/watch.rs: the five watch cases (19-23), see "Files"

// crates/holler-pane-testkit/src/fixture.rs (added beside sample_pane)
/// A valid, deterministic `ProfileSpec` for the pane named `pane` (plain text, not checked: a spec may be detached):
/// workspace "scratch", grid r1c1, cwd "/srv/demo", harness opencode with port_policy "fixed", the sample pane's
/// model and context, role agent, no env, no command, no check, no expect. Two calls with one name are equal.
pub fn sample_spec(pane: &str) -> ProfileSpec;
/// A valid, deterministic `Profile` named `name`: slug `name`'s slug, generation 0, one `sample_spec` per entry of
/// `panes` in order, created and updated 0. `usage` when `name` is not a valid profile name.
pub fn sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError>;
```

How each implementation runs the suite (doc comment on `run_profile_store_conformance`, in a `text` fence, as on the pane
suite):
```text
// the fake:  assert_eq!(run_profile_store_conformance(|| (FakeProfileStore::new(), ())), Ok(()));
// the hub's profile registry (#661, in holler-hub/tests/): a fresh registry in a temp dir per case with a short
// watch window, the temp dir as the guard.
```

**Reuse refactors (behaviour-neutral, inside the crate):**
- `src/feed.rs`: `Writer` (with `expected`) moves here from `pane_store.rs:83-100` as `pub(crate) enum Writer`, and `lock`
  (`feed.rs:218`) becomes `pub(crate)`. `pane_store.rs` imports `Writer` from `crate::feed` and keeps no copy.
- `src/conformance/pane_store.rs`: `profile_name`, `expect_change`, `changes`, `cursors` and `increasing` become
  `pub(super)`. The last four become generic over `crate::feed::Change`, and the pane cases do not change:
  ```rust
  pub(super) fn expect_change<E>(watch: &mut Watch<E>, key: &E::Key, record: Option<&E::Record>, after: &str) -> Result<Cursor, String>
  where E: Change, E::Key: Debug, E::Record: PartialEq + Debug;
  pub(super) fn changes<E>(events: Vec<E>) -> Vec<(E::Key, Option<E::Record>)> where E: Change, E::Record: Clone;  // sorted by key
  pub(super) fn cursors<E: Change>(events: &[E]) -> Vec<Cursor>;
  pub(super) fn increasing(cursors: &[Cursor]) -> Result<(), String>;
  ```
  Each one's doc says the profile store suite reuses it. These helpers stay in `pane_store.rs` rather than moving to
  `conformance/mod.rs`, because the issue keeps `mod.rs` out of every slice's diff.

## Fake behaviour (`FakeProfileStore`)

- **Key.** Records, events and the log are filed by `profile.name.slug()`. The stored `slug` is always the name's slug, and
  a submitted `slug` is ignored, as the submitted generation is.
- **Every port method** first calls `faults.enter(op)`. When that fails, the method returns the error and changes nothing.
  `rename` passes the switch too, so a wedged store's `rename` is `timeout`.
- **`cas_put(profile, expected, actor)`** runs inside one `feed.write`: `stored = log.get(&slug)`.
  1. *Name rule* (a port writer only, and checked **before** the generation): if `stored` is `Some` and
     `stored.name != profile.name`, the answer is `ProfileExists { what }`, naming both display names and the slug.
     The rule holds whatever the generation.
  2. `generation = next_generation(stored generation or 0, writer.expected(current))?`.
  3. `next = Profile { generation, slug, created: stored.created or now, updated: now, ..profile.clone() }`.
  4. `log.append(ProfileEvent { cursor, name: next.name, profile: Some(next) })?`, and only then the log entry
     `{ at: now, generation, actor, change }`, where `change` is `Created` when nothing was stored and otherwise
     `Updated { summary: format!("pane specs: {} -> {}", stored.panes.len(), next.panes.len()) }`.
  Returns `next`.
- **`delete(name, expected, actor)`**: a missing profile is `ProfileNotFound { what: name }` first. Then
  `generation = next_generation(current, expected)?`, then an event `{ name: stored.name, profile: None }`, then the entry
  `{ at: now, generation, actor, Deleted }`. The `Deleted` entry carries the deleted generation + 1, because each applied
  write adds one.
- **`log(name)`**: the slug's entries, oldest first, kept across a delete and a re-create (append-only; never cut). No
  entry at all is `ProfileNotFound { what: name }`.
- **`rename(..)`**: after the switch, `Err(PaneError::NotImplemented)`, changing nothing.
- **`list`**: every live record, in slug order (the fake's order, which the suite does not pin). **`get`**: the live
  record by slug.
- **`watch`**: `Feed::watch(&self.feed, since, &self.faults, ProfileStoreOp::WatchNext)`, with the feed's rules unchanged.
- **Env**: the fake runs no env check of its own (no `'='` scan). A `Profile` holds `EnvVarName`s, which cannot carry a value.
- `concurrent_*` and `seeded` use `Writer::Other` and `Writer::Port(0)`, exactly as `FakePaneStore`'s do. Every library
  path returns `Result`: no `unwrap`, `expect`, `panic` or `assert!` in `src/`.

## Conformance cases: `run_profile_store_conformance` (23)

Each case gets a fresh, empty store. Profiles come from `sample_profile` under neutral names (`Demo Alpha`, `Demo Beta`,
`Demo Gamma`; pane names `demo-c1r1`...). The suite's actor is `Actor::parse("conformance")`.

- **"Stored as"**: the record a write returns equals the submitted profile with `generation` set and with `created` and
  `updated` taken from the store's own reply. The suite pins no timestamp value and no slug spelling beyond `name.slug()`.
- **"Unchanged"**: `get`, `list` and `log` of the name are each equal to before.
- **"History"**: `log` read as `(generation, actor, kind)` with `kind` one of created, updated, renamed or deleted. Every
  `Updated` summary must be non-empty and one line (no `\n` or `\r`). The text itself is the store's choice.

| # | Case id | What it asserts |
|---|---|---|
| 1 | `get-missing-is-none` | `get` of a name never stored is `Ok(None)`. |
| 2 | `list-empty` | `list` of an empty store is `Ok(vec![])`. |
| 3 | `create-at-zero-stored-at-one` | `cas_put(p, 0, actor)` returns `p` stored as generation 1, and `get` returns that same record. History is `[(1, actor, created)]`. |
| 4 | `submitted-generation-ignored` | `p.generation = 99` at expected 0 is returned and stored at generation 1. |
| 5 | `create-over-existing-conflicts` | With `p` at 1, `cas_put(p2, 0)` with the same name is `generation-conflict`. Unchanged, and the history still has one entry. |
| 6 | `same-slug-other-name-is-profile-exists` | With `Demo Alpha` at 1, `cas_put` of `DEMO-ALPHA` (same slug) at expected 0 is `profile-exists`. Unchanged, and `list` holds the one record. |
| 7 | `update-bumps-by-one` | At 1, `cas_put(p2, 1)` returns `p2` stored as 2, and `get` returns it. History is `[(1, created), (2, updated)]` with the case's actor. |
| 8 | `stale-generation-conflicts` | At 2, `cas_put(p3, 1)` is `generation-conflict`. Unchanged; the history gets no entry. |
| 9 | `expected-ahead-conflicts` | At 1, `cas_put(p2, 5)` is `generation-conflict` and unchanged. For a name never stored, `cas_put(q, 3)` is `generation-conflict`, `get(q)` stays `None` and `log(q)` is `profile-not-found`. |
| 10 | `list-holds-every-profile` | After creating Gamma, Alpha and Beta, `list` (sorted by slug in the suite) equals the three stored records. |
| 11 | `delete-at-current-generation` | At 1, `delete(p, 1, actor)` is `Ok`, `get` is `None` and `list` is empty. History is `[(1, created), (2, deleted)]`: the log stays readable. |
| 12 | `delete-stale-conflicts` | At 2, `delete(p, 1)` is `generation-conflict`. Unchanged, so the record is still at 2 and the history unchanged. |
| 13 | `delete-missing-is-profile-not-found` | `delete` of a missing name at expected 0 and at 7 is `profile-not-found` both times: existence is checked before the generation. |
| 14 | `recreate-after-delete-starts-at-one` | Create, delete, then `cas_put(p, 0)` stores generation 1 (the known gap, ADR-0021 section 8). History is `[(1, created), (2, deleted), (1, created)]`. |
| 15 | `log-is-append-only-and-oldest-first` | Create and update twice, then read `L1` (3 entries). Update again, then read `L2`: `L2[..3] == L1`, it has 4 entries, generations 1 to 4 in order and `at` non-decreasing. After a delete the log is still readable, with 5 entries and `deleted` last. |
| 16 | `log-of-never-created-is-profile-not-found` | `log` of a name never stored is `profile-not-found`. |
| 17 | `rename-is-not-implemented` | With `p` at 1, `rename(p, q, 1, actor)` is `not-implemented` (PROPOSED, #665). `p` is unchanged, `get(q)` is `None` and the history unchanged. #665 replaces this case. |
| 18 | `env-is-names-only` | `EnvVarName::parse("TOKEN=x")` is `profile-secret-refused`, and `parse(" ")` and `parse("")` are `env-name-invalid`: one guard, the type. A profile whose spec has env `[ANTHROPIC_API_KEY, TOKEN, lower.dotted]` is stored and read back with that env verbatim, so the store adds no env check of its own. |
| 19 | `watch-from-zero-yields-current-state` | Create a and b, update a, create c, delete b. `watch(Cursor(0))` then yields puts of exactly a (generation 2) and c, keyed by slug and each equal to the stored record, with cursors strictly increasing and no event for b; then `Ok(None)`. |
| 20 | `watch-follows-each-change` | On an empty store, `watch(Cursor(0))` yields `Ok(None)`. After each of create, update and delete, one `next()` yields exactly that change: puts carrying the stored record, then `profile: None` for the delete, with cursors strictly increasing. |
| 21 | `watch-resumes-without-gap-or-repeat` | Create a and b, drain `watch(Cursor(0))` and keep the last cursor `c`. Create d; `watch(c)` then yields exactly d, then `Ok(None)`. |
| 22 | `watch-idle-is-ok-none-and-stays-usable` | After a drain, `next()` is `Some(Ok(None))`. A write after that is yielded by the same iterator. |
| 23 | `failed-write-changes-nothing` | A watch is resumed at the last cursor. A stale `cas_put`, a stale `delete`, a `delete` of a missing name, a same-slug create (`profile-exists`) and a `rename` (`not-implemented`) each fail and leave `p` unchanged, history included. The watch then yields only `Ok(None)`: a refused write gets no log entry and no event. |

Cases 19 to 23 mirror cases 14 to 18 of the pane suite. The watch is keyed by slug, and they reuse the generic helpers.

## Acceptance criteria

T authors these tests (RED first). Every test file starts with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682`.

- [ ] **AC1 The fake passes its suite.** `tests/profile_store_conformance_test.rs`:
  `the_fake_passes_the_profile_store_conformance_suite`: `run_profile_store_conformance(|| (FakeProfileStore::new(), ()))`
  is `Ok(())`. `the_suite_runs_the_documented_cases`: `profile_store_cases()` equals the 23 ids above, in order.
- [ ] **AC2 Mutation check: a broken store fails on the named case.** In the same file, `enum Break` and `struct Mutant`
  wrap `FakeProfileStore` as `tests/pane_store_conformance_test.rs:153-263` does, with `assert_suite_fails_on(broken, case)`:
  - `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone` (`Break::Nothing`) is `Ok(())`.
  - `a_store_that_skips_the_generation_check_fails`: `cas_put` writes at whatever generation is current. Fails on
    `stale-generation-conflicts`. This is the mutant the issue names.
  - `a_store_that_keeps_the_submitted_generation_fails`: fails on `submitted-generation-ignored`.
  - `a_store_that_checks_generation_before_existence_fails`: a missing name with expected != 0 answers
    `generation-conflict`. Fails on `delete-missing-is-profile-not-found`.
  - `a_store_that_lets_a_same_slug_name_overwrite_fails`: on a stored slug with another name it calls `concurrent_put`.
    Fails on `same-slug-other-name-is-profile-exists`.
  - `a_store_that_forgets_the_log_on_delete_fails`: `log` of a name `get` does not find is `profile-not-found`. Fails on
    `log-is-append-only-and-oldest-first`.
  - `a_store_that_repeats_on_resume_fails`: `watch(since)` delegates to `since - 1`. Fails on
    `watch-resumes-without-gap-or-repeat`.
- [ ] **AC3 Faults can be injected and observed.** `tests/fake_profile_store_test.rs`:
  - `a_wedged_store_times_out_every_method`: `get`, `list`, `cas_put`, `delete`, `watch`, `log` and `rename` each
    answer `Timeout { op: "profile_store.<method>" }`. Nothing was written, and after `set(None)` they work.
  - `a_wedged_store_ends_an_open_watch`: `Err(Timeout { op: "profile_store.watch_next" })` once, then `None`.
  - `a_corrupt_store_fails_closed_everywhere`: `Fault::Fail(StoreCorrupt)` makes all seven methods answer that error.
    After it is cleared, the records and the log are intact.
  - `fail_next_is_one_shot_and_per_op`: `fail_next(CasPut, Unavailable)`. The first `cas_put` fails and writes neither
    a record nor a log entry. `get` is unaffected and the second `cas_put` succeeds.
  - `a_slow_call_takes_at_least_the_delay`: `set_delay(50 ms)`, and `log` takes at least 50 ms (lower bound only).
  - `calls_are_recorded_in_order`: after `get`, a failed `cas_put`, `log` and `list`, `calls()` is
    `[Get, CasPut, Log, List]`. `seeded`, `concurrent_put` and `concurrent_delete` add nothing to it.
  - `port_op_names_are_port_dot_method`: all 8 `as_str` values.
- [ ] **AC4 The fake's own rules.** Same file:
  - `the_clock_stamps_created_updated_and_at`: `set_now(1_000)` and create, so `created = updated = 1_000` and the
    entry's `at` is 1_000. `set_now(2_000)` and update, so `created` stays 1_000, `updated` is 2_000 and `at` is 2_000.
    `set_now(3_000)` and delete, so the `Deleted` entry has `at` 3_000 and generation 3. A new store stamps 0.
  - `an_update_summarises_the_spec_count`: one spec, then two, gives `Updated { summary: "pane specs: 1 -> 2" }`.
  - `the_submitted_slug_is_replaced_by_the_names_slug`: a `Profile` built with `slug: "wrong"` is stored with
    `name.slug()`.
  - `a_same_slug_other_name_is_profile_exists_at_any_generation`: `DEMO-ALPHA` at expected 1 (current) over
    `Demo Alpha` is still `profile-exists`.
  - `a_concurrent_put_makes_the_next_cas_stale`: read at g, then `concurrent_put(.., other)`. `cas_put(.., g)` is
    `generation-conflict`, the log's last entry has the other actor, and a watch sees the write.
  - `a_concurrent_put_creates_a_profile_at_one` (with a `Created` entry).
  - `a_concurrent_delete_makes_the_profile_vanish`: `get` is `None` and `delete(.., g)` is `profile-not-found`; the log
    keeps a `Deleted` entry with the other actor; `concurrent_delete` of a missing name is `profile-not-found`.
  - `a_watch_ahead_of_the_head_is_usage`: `watch(Cursor(5))` on an empty store.
  - `the_idle_wait_wakes_on_a_write`: `set_idle_wait(5 s)`. A thread writes after 50 ms, and `next()` yields it well
    under 5 s.
  - `seeded_stores_hold_each_profile_at_generation_one`: each seed has one `Created` entry by the seed actor, and the call
    log is empty. `seeding_the_same_name_twice_is_a_conflict`. `seeding_a_same_slug_name_is_profile_exists`.
  - `sample_profile_is_valid_and_deterministic`: two calls are equal; the slug is the name's; the specs follow `panes`
    in order; `sample_profile("!!", &[])` is `usage`. Also `sample_spec("demo-c1r1").pane == "demo-c1r1"`.
  - `the_fake_is_send_and_sync_and_its_watch_is_send` (a compile-time bound check).
- [ ] **AC5 Slice a still holds.** `tests/pane_store_conformance_test.rs` and `tests/fake_pane_store_test.rs` pass
  unchanged (not one line of either file edited), which proves the `Writer`, `lock` and helper refactors are
  behaviour-neutral.
- [ ] **AC6 Dependencies.** `git diff --quiet origin/main -- crates/holler-pane-testkit/Cargo.toml` succeeds (manifest
  untouched). `cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'` prints
  nothing.
- [ ] **AC7 One env guard, one feed, one writer enum.** Each of these prints nothing:
  - `grep -rnE "contains\('='\)|contains\(\"=\"\)" crates/holler-pane-testkit/src`
  - `grep -rn "enum Writer" crates/holler-pane-testkit/src | grep -v feed.rs`
  - `grep -rn "Condvar" crates/holler-pane-testkit/src | grep -v feed.rs`
- [ ] **AC8 Layout.** `git diff --name-only origin/main...HEAD` lists only the files under "Blast radius". `src/lib.rs` and
  `src/conformance/mod.rs` are absent. `src/profile_scope.rs` and `src/conformance/profile_scope.rs` are still the
  one-line stubs.
- [ ] **AC9 CHANGELOG.** There is one entry at the end of `## [Unreleased]` / `### Enhancements`, after the #676 entry
  (`CHANGELOG.md:82-89`). It reads: the test kit's fake profile registry, with the faults (wedged, corrupt, one-shot,
  slow, another writer), a settable clock, the append-only change log, and a 23-case conformance suite any profile
  registry runs against itself (compare-and-swap, same-slug refusal, delete, the log, `rename` not implemented yet, env
  names only, the watch stream). The fake profile scope follows. Test code only. Link
  [#682](https://github.com/Performant-Labs/holler/issues/682). `bash scripts/changelog-check.sh` passes.
- [ ] **AC10 Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p holler-pane-testkit`, `cargo test --workspace`, `cargo machete`, `bash scripts/lint.sh` and
  `bash scripts/test-hooks.sh` pass. `rustfmt --check --edition 2021` passes on every new or changed `.rs` file. No `.rs`
  file in the diff reaches 600 lines. No function exceeds 100 lines or cognitive complexity 15 (one function per case).

## Files

New (all under `crates/holler-pane-testkit/`):
- `src/conformance/profile_store/watch.rs` (~230): cases 19 to 23, `open` and `last_cursor` for `ProfileStore`. It is
  declared as `mod watch;` in `src/conformance/profile_store.rs`, and its case functions are `pub(super)`.
- `tests/profile_store_conformance_test.rs` (~260), `tests/fake_profile_store_test.rs` (~380).

Filled stubs:
- `src/profile_store.rs` (~360).
- `src/conformance/profile_store.rs` (~480): the runner, the 23-row `CASES` table, cases 1 to 18, and helpers such as
  `sample`, `put`, `shown`, `unchanged`, `history` and `revised` (which changes one spec field).

Changed: `src/feed.rs` (+~25: `Writer`, `pub(crate) lock`), `src/pane_store.rs` (−~20: imports `Writer`),
`src/conformance/pane_store.rs` (±~10: five helpers `pub(super)`, four of them generic), `src/fixture.rs` (+~60:
`sample_spec`, `sample_profile`), `CHANGELOG.md`.

Not changed: `Cargo.toml` (crate and workspace), `src/lib.rs`, `src/conformance/mod.rs`, `src/profile_scope.rs`,
`src/conformance/profile_scope.rs`, every other crate.

## Extend vs new

- **Extend** the existing stubs (`profile_store.rs` and `conformance/profile_store.rs`) and fixture module. Implement the
  frozen `ProfileStore`. Call `next_generation` for every CAS. Return only closed `PaneError` variants (`Conflict`,
  `ProfileNotFound`, `ProfileExists`, `NotImplemented`, `Timeout`, `StoreCorrupt`, `Unavailable`, `Usage`), never a new code.
- **Reuse, do not copy:** `FaultSwitch` and `PortOp` (`fault.rs`); `Feed`, `Log`, `Change` and the `Watch` stream (`feed.rs`);
  `Writer` and `lock`, moved into `feed.rs` so the two fakes share one; `run_cases`, `succeeds`, `expect_code`,
  `expect_eq`, `next_item` and `drain` (`conformance/mod.rs`); `profile_name` and the four watch helpers
  (`conformance/pane_store.rs`, now generic); `ProfileName::slug` for every key and comparison; `EnvVarName` as the only
  env guard.
- **New, because no equivalent exists:** the per-slug change log beside the feed (`history`), the clock, the name rule
  and `rename`'s refusal.
- **Accepted near-duplicate:** the two fakes' `ProfileStore`/`PaneStore` impl blocks have the same shape (enter, then one
  write path). They implement different traits, and a macro would cost more clarity than it saves.

## Decisions already made (operator, epic, ADR)

- The testkit depends on `holler-pane` (and `serde_json` from #681) only, never `holler-cli` or `holler-hub`
  (ADR-0021:184-186).
- Writes take an `Actor` and append `ProfileLogEntry { at, generation, actor, change }`. `Profile` has no `log` field, so
  the log is read only through `log()` (#638 amendment 2026-10-09, `profile.rs:4-7`).
- Generations: a create names 0 and is stored at 1; the submitted generation is ignored; a stale or ahead one is
  `generation-conflict`; `delete` checks existence first (ADR-0021:268-272).
- `rename` stays PROPOSED and answers `not-implemented` (ADR-0021 "Decisions taken" item 4, `profile.rs:361-363`).
- The no-secret refusal is the one `EnvVarName` guard: `NAME=value` is `profile-secret-refused`, a blank or whitespace name
  `env-name-invalid`, with no second scan (#661 amendment 2026-10-09).
- No slice edits `lib.rs` or `conformance/mod.rs` (#682 scope).

## Decisions made in this brief

1. **The split.** This run is part 1 (the store). Part 2 (the scope, with the I8 cases) is fixed in the appendix.
2. **The name rule comes before the generation check**, so a same-slug, other-name create at 0 is `profile-exists` and not
   `generation-conflict`. The fake applies it at every generation, because a display-name change through `cas_put` would be
   a rename without a `Renamed` entry. The suite pins only expected 0, which is the issue's wording.
3. **The store files everything by slug** and stores `name.slug()` whatever slug was submitted, as with the generation.
4. **A `Deleted` entry carries the deleted generation + 1** (ADR-0021:269, "each applied write adds one"), so a profile's
   log generations rise strictly within one life of the profile.
5. **The log is per slug and append-only across a delete and a re-create.** The log of a deleted profile stays readable.
   Only a name that was never created is `profile-not-found`.
6. **The suite is neutral on timestamps and the summary's wording.** It compares records with `created` and `updated` taken
   from the store's own reply, checks only that a summary is one non-empty line, and that `at` does not decrease. The fake
   uses a settable clock (`set_now`, default 0): `created` at create time, kept on update, and `updated` and `at` set to now.
   Its summary is `pane specs: <before> -> <after>`.
7. **`seeded`, `concurrent_put` and `concurrent_delete` take an `&Actor`**, which `FakePaneStore`'s do not, because every
   profile write is logged with a "who".
8. **The env leg runs without JSON.** The testkit has no `serde_json` until #681, which runs in parallel and adds it to the
   same manifest block. The JSON-decode refusal is already pinned in `holler-pane`
   (`argv_env_test.rs:96-102`). Case 18 pins the guard's codes and that the store stores env names verbatim. A JSON round
   trip can be added once #681 has merged.
9. **`list` order is not pinned** for profiles (ADR-0021 pins only `pane/list`). The suite sorts by slug before comparing.
10. **Reuse goes through slice a's own files** (`feed.rs`, `pane_store.rs`, `conformance/pane_store.rs`, `fixture.rs`), never
    through `lib.rs` or `conformance/mod.rs`. Slices b, d and e do not use the feed or the pane suite.

## Out of scope

Part 2 (the appendix): `FakeProfileScope`, `run_profile_scope_conformance`, the I8 cases and the two scope mutants. Also
out of scope: slices b, d and e; any change to `holler-pane`, `holler-hub`, `holler-cli` or any manifest; running the suite
against the hub (#661 does that); `rename`'s real behaviour (#665); protocol v2 and every golden file (unchanged).

## Test plan

RED (T): write the two test files against the API above. They fail to build because `FakeProfileStore`, `ProfileStoreOp`,
`run_profile_store_conformance`, `profile_store_cases` and `sample_profile` do not exist. Confirm with
`cargo test -p holler-pane-testkit` that the errors are the missing items and not a typo, and that slice a's tests still
build and pass. GREEN (F): first the refactors (`feed.rs`, `pane_store.rs`, `conformance/pane_store.rs`), with slice a's
tests green after them. Then `fixture.rs`, `profile_store.rs`, `conformance/profile_store.rs` and `profile_store/watch.rs`,
the CHANGELOG, and the guards of AC10. A (anti-duplication) checks AC7 and that the profile suite calls the shared helpers
rather than local copies.

## Risks

- **A sibling slice may also append to `src/fixture.rs` or `CHANGELOG.md`.** Slices d and e run in parallel. A conflict
  there is a textual one at merge time, and the second PR to merge rebases. No code depends on the order.
- **The suite file sizes.** `conformance/profile_store.rs` is estimated at ~480 lines. If it nears 600, move cases 15 to
  18 into a second child module beside `watch.rs` (`profile_store/log.rs`), declared in the same file. `mod.rs` is still
  not touched.
- **Timing tests** (`the_idle_wait_wakes_on_a_write`, `a_slow_call_takes_at_least_the_delay`) assert only lower bounds and
  a generous upper bound, never an exact duration.
- **#661 may want to answer differently** on decisions 2 to 5. The suite fixes them now, as the pane suite fixed the pane
  rules for #639. A change to any of them is an amendment to this suite, made first.

## Blast radius

`crates/holler-pane-testkit/src/{profile_store.rs, conformance/profile_store.rs, conformance/profile_store/watch.rs,
feed.rs, pane_store.rs, conformance/pane_store.rs, fixture.rs}`,
`crates/holler-pane-testkit/tests/{profile_store_conformance_test.rs, fake_profile_store_test.rs}`, `CHANGELOG.md`, and
`docs/handoffs/682*` (pipeline artifacts). This is wider than the issue's list (two stubs and the CHANGELOG): the
conformance stub, its child module and the tests are what the scope requires, and the four slice-a files are the reuse the
issue asks for. All of them are inside #638's radius, `crates/holler-pane-testkit/**`. Not changed: any `Cargo.toml`,
`Cargo.lock`, any other crate, ADR, protocol doc or golden file. The repository is public, so no personal names appear in
code, comments, tests or the changelog.

---

## Appendix: fixed for part 2 (not built in this run)

Recorded so that part 2's O run starts from settled names and cases. Files: `src/profile_scope.rs` and
`src/conformance/profile_scope.rs` (the stubs), plus `tests/profile_scope_conformance_test.rs` and
`tests/fake_profile_scope_test.rs`. Estimate: ~1,020 lines.

```rust
// src/profile_scope.rs
/// A `ProfileScope` over any `ProfileStore` and `PaneStore`. It never writes a pane record: recording the pane
/// (ADR-0021 section 8, step 4) is the verb's, inside or after its act.
pub struct FakeProfileScope { profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor }
impl FakeProfileScope {
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self;
}
impl ProfileScope for FakeProfileScope { /* resolve, edit_spec as below */ }

// src/conformance/profile_scope.rs
pub fn profile_scope_cases() -> Vec<&'static str>;
/// Per case: fresh `FakeProfileStore` and `FakePaneStore` seeded with the fixture below (seeding bypasses the call
/// logs), then `build(profiles, panes)` makes the scope under test over them. The suite drives the scope and inspects
/// the fakes.
pub fn run_profile_scope_conformance<S, F>(build: F) -> Conformance
where S: ProfileScope, F: FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S;
```

**The fake's `edit_spec(Some(P), pane, edit, act)`, in the I8 order (ADR-0021:285-302, "Decisions taken" item 1):**
1. `stored = profiles.get(P)?`, and a missing profile is `profile-not-found`, before anything else.
2. `panes.get(pane)?`: a record whose `profile` is `Some(Q)` with `Q.slug() != P.slug()` is `pane-in-other-profile`. No
   record, or `None`, passes.
3. `edited` is `stored` with the edit applied. `Set(spec)` replaces the entry whose `spec.pane == pane.as_str()` in place,
   or appends one; `Remove` drops it.
4. `written = profiles.cas_put(&edited, stored.generation, &actor)?`. This is the **profile written first**; a conflict
   here is `generation-conflict` and the act is never called.
5. `act()`. If it succeeds, return `Ok(Some(written))`.
6. If it fails with `e`, write `Profile { panes: stored.panes, ..written }` back with
   `profiles.cas_put(.., written.generation, &actor)`. If that succeeds, return `Err(e)`: the specs are equal to before,
   the generation has moved by two, and the log shows the edit and its reversal. A `Conflict` there is
   `ProfileConflict { what }`, naming P and saying its specs were not restored; the other writer's version stays. Any
   other error from the restoring write is returned as it is.

`edit_spec(None, ..)` calls only `act()`, returns `Ok(None)` or the act's error, and makes no profile store call.
`resolve(P, None)` returns `P` and every pane whose `Pane.profile` has P's slug, sorted by name: membership is `Pane.profile`
only, so a detached spec adds no pane. `resolve(P, Some(n))` returns that pane if it is a member and is
`pane-not-in-profile` otherwise (including a pane with no record). A missing P is `profile-not-found`.

The trait's doc (`profile.rs:391-396`, "if `act` fails nothing is recorded") reads in the light of the ADR's decision: "nothing
recorded" means P's specs are restored and no pane record is written, not that the generation is unchanged.

**Seed fixture:** `Demo Alpha` with specs `[demo-c1r1, demo-c2r1, demo-c3r1]` (the c3 entry is a detached spec) and
`Demo Beta` with `[demo-c3r1]`. Panes `demo-c1r1` and `demo-c2r1` have `profile = Demo Alpha`, `demo-c3r1` has
`profile = Demo Beta`, and `demo-c4r1` has `profile = None`. `Demo Gamma` does not exist. Alpha is at generation g = 1.

| # | Case id | What it asserts |
|---|---|---|
| 1 | `resolve-every-pane-of-the-profile` | `resolve(Alpha, None)` is the stored Alpha with panes `[c1, c2]`: not c3 (detached spec, Beta's pane), not c4. |
| 2 | `resolve-named-member` | `resolve(Alpha, Some(c1))` has panes `[c1]`. |
| 3 | `resolve-non-member-is-pane-not-in-profile` | `resolve(Alpha, Some(c3))` and `resolve(Alpha, Some(c4))` are each `pane-not-in-profile`. |
| 4 | `resolve-missing-profile-is-profile-not-found` | `resolve(Gamma, None)` and `resolve(Gamma, Some(c1))` are each `profile-not-found`. |
| 5 | `edit-set-replaces-the-entry-and-bumps-once` | `edit_spec(Some(Alpha), c1, Set(s'), ok)` returns `Some(r)` with `r == get(Alpha)` and generation g + 1. c1's entry is `s'` at the same index and the other entries are unchanged. The act ran once, the log gained one `Updated` entry, and the pane list is unchanged. |
| 6 | `edit-set-adds-a-missing-entry` | `Set` for c4 (no profile) appends its spec at g + 1. |
| 7 | `edit-remove-drops-the-entry` | `Remove` for c2 drops its entry at g + 1, and the other entries keep their order. |
| 8 | `the-act-sees-the-edit` | Inside the act, `profiles.get(Alpha)` already holds the edited specs at g + 1: the profile is written first (I8). |
| 9 | `failed-act-restores-the-specs` | The act fails with `Unavailable { what: "act" }`. `edit_spec` returns exactly that error. `get(Alpha).panes` equals the panes before; **the generation is g + 2**; the log gained exactly two `Updated` entries, at g + 1 and g + 2; the pane list is unchanged; the act ran once. |
| 10 | `first-write-conflict-is-generation-conflict` | `profiles.faults().fail_next(CasPut, Conflict)` makes `edit_spec` answer `generation-conflict`. The act ran 0 times and Alpha is unchanged (`get` and `log`). |
| 11 | `restore-conflict-is-profile-conflict` | The act calls `profiles.concurrent_put(alpha_other, other)` and then fails. `edit_spec` is `profile-conflict`, its `what` contains `Demo Alpha`, and `get(Alpha)` is the other writer's version (g + 2, its specs). The act ran once. |
| 12 | `no-profile-runs-only-the-act` | `edit_spec(None, c1, Set(s'), ok)` is `Ok(None)`, with the act run once and `profiles.faults().calls()` empty. With a failing act it returns the act's error and `calls()` is still empty. |
| 13 | `missing-profile-is-profile-not-found-before-the-act` | `edit_spec(Some(Gamma), ..)` is `profile-not-found`. The act ran 0 times and `calls()` holds no `CasPut`. |
| 14 | `pane-in-other-profile-before-any-write` | `edit_spec(Some(Alpha), c3, ..)` is `pane-in-other-profile`. The act ran 0 times, `calls()` holds no `CasPut`, and Alpha is unchanged. |

**Mutants (the issue's two), as wrappers or small `ProfileScope` impls in `tests/profile_scope_conformance_test.rs`:**
- `a_scope_that_writes_the_profile_after_the_act_fails`: it runs the act, then the CAS. It fails on `the-act-sees-the-edit`
  and `failed-act-restores-the-specs` (the generation moves by 0, not 2).
- `a_scope_that_does_not_restore_on_a_failed_act_fails`: it fails on `failed-act-restores-the-specs`.

Plus `the_unbroken_wrapper_passes...`, `the_fake_passes_the_profile_scope_conformance_suite` and
`the_suite_runs_the_documented_cases`. The fake's own tests: a restoring write that fails with `unavailable` returns that
error; `resolve` of a pane with no record is `pane-not-in-profile`; the scope is `Send + Sync`.
