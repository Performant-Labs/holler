# Brief: #638 the pane test kit, slice a: the fault switch, the fake `PaneStore` and its conformance suite

Repo: Performant-Labs/holler. Issue: #638 (epic #633). Rigor: in-session. UI surface: no. Kind: feature (test kit).

**Branch:** `issue-638-implementation`, based on `939d79c` (`origin/main`: #637, #669, #670, ADR-0021 and the spikes
#635 and #636 merged). **Design (D):** N/A. **Decision record:** ADR-0021 (sections 2, 5, 7, 8) and the epic's
"Skeleton split" rulings and "Decisions 2026-10-09". The issue is the source of truth; where this brief differs from it, the
issue wins, except for the split below, which the issue does not forbid and which this run builds only the first part of.

## Size check and split

**The issue as written does not fit one run.** It asks for six fakes (`PaneStore`, `ProfileStore`, `ProfileScope`,
`HerdrPort`, `HostPort`, `HarnessPort`) plus a fake prober, a fault mechanism, six generic conformance suites, a JSON-envelope
checker and a mutation check per suite. Estimate, in Rust lines including the tests that prove them:

| Part | Modules | Lines (est.) |
|---|---|---|
| fault switch, change feed, sample fixture, `FakePaneStore`, `PaneStore` suite | 6 + `lib.rs` | ~1,550 |
| JSON-envelope helper | 1 | ~500 |
| `FakeProfileStore`, `FakeProfileScope`, their two suites | 4 | ~1,500 |
| `FakeHerdr`, `FakeProber`, the `HerdrPort` suite | 3 | ~1,050 |
| `FakeHost`, `FakeHarness`, their two suites | 4 | ~1,200 |
| **Total** | **18 + `lib.rs`** | **~5,800** |

That is about three times the ~2,000-line and six-module limit of one pipeline run, the limit #637 was blocked on three
times. **Proposed split, along the ports' seams** (each slice is its own pipeline run and PR; slices b to e need their own
issues, see "Needs operator"):

| Slice | Scope (one line) | Depends on |
|---|---|---|
| **638a (this brief)** | The crate wiring, the shared `FaultSwitch`, the generic change feed, `sample_pane`, `FakePaneStore`, `run_pane_store_conformance`, and the empty stub files of slices b to e | #637, #670 (merged) |
| 638b | The JSON-envelope helper (`envelope.rs`): one envelope or NDJSON lines, `schema_version` 1, `ok` against exit 0/1/2/3, `error` null exactly when `ok`, kebab-case code, one-line message, nothing else on stdout | 638a; #676 (`class_of`) |
| 638c | `FakeProfileStore` and `FakeProfileScope` with their suites (CAS with `Actor`, append-only log, change feed, `profile-exists`, `store-corrupt`; I8 `edit_spec`, membership, every-pane scope) | 638a |
| 638d | `FakeHerdr` (GridPos cells, opaque stable ids, split-only mode, two selectable versions, vanished pane) and `FakeProber`, with the `HerdrPort` suite (the `r2c1`/`r1c2` transposition case) | 638a |
| 638e | `FakeHost` and `FakeHarness` (shared data dir, `select-session` acked with no TUI, unknown-id abort acked, frozen server, session deleted under a TUI, shown session) with their suites | 638a |

b, c, d and e touch disjoint files (638a pre-creates each one's stub files, so none edits `lib.rs` or
`conformance/mod.rs`), so they can run in parallel once 638a merges. Only 638b adds a dependency line (`serde_json`). The
signatures and case tables fixed now for b to e are in the appendix, "Fixed for the later slices"; this run builds none of it.
Which wave-3 story needs which slice: #639/#649 (hub `PaneStore`) need a; every verb with `--format=json` (#643 onward)
needs b; #661, #662, #663 need c; #640 needs d; #641, #642 need e; #644 and #649 need all five. **#638 closes when 638e (or
the last slice) merges; this PR says "Part of #638", not "Closes".**

## Problem

`holler-pane-testkit` is an empty crate (`crates/holler-pane-testkit/src/lib.rs:1-6`). Every pane-control story is to be
tested against fakes from it, and every real implementation of a port is to run a conformance suite from it against
itself. The first port both sides need is `PaneStore`: the hub registry (#639, in review) implements it, the CLI's
`pane/*` client (#649) will, and every verb (#643 onward) reads and writes panes through it. This slice builds the
mechanism the other fakes reuse (a fault switch and a change feed), the `PaneStore` fake, and its conformance suite.

## Evidence (verbatim, as of `939d79c` unless marked)

The crate today:
```
crates/holler-pane-testkit/src/lib.rs:1-6
//! `holler_pane_testkit` — fakes of every `holler_pane` port and the conformance
//! suite each adapter must pass, so no test touches a real Herdr, tmux or OpenCode
//! (epic #633).
//!
//! Empty skeleton (story #637); story #638 fills it. It must not depend on
//! `holler-cli`: the hub and the CLI take it as a dev-dependency.
crates/holler-pane-testkit/Cargo.toml:12-19
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise). The crate is an empty skeleton; its owning story adds the
# dependencies it uses.
[dependencies]

# Workspace lints (issue #149).
[lints]
workspace = true
```
The consumers already dev-depend on it (no manifest edit needed outside the crate):
```
crates/holler-hub/Cargo.toml:55-58     # Issue #669 (epic #633): the fakes and the conformance suite of the pane ports
                                       # (empty in the skeleton; #638 fills it). Linked by `tests/pane_dispatch_test.rs`
                                       # (`testkit_links`) so #638, #639 and #661 add no manifest line.
                                       holler-pane-testkit = { path = "../holler-pane-testkit" }
crates/holler-cli/Cargo.toml:432-435   # #670: the fakes and conformance suite of holler-pane (empty until #638). ...
                                       holler-pane-testkit = { path = "../holler-pane-testkit" }
crates/holler-hub/tests/pane_dispatch_test.rs:40   use holler_pane_testkit as _;
crates/holler-cli/tests/pane_verbs/list.rs:9       use holler_pane_testkit as _;
```
The dependency rule:
```
docs/adr/ADR-0021.md:184-186
- `holler-pane-testkit` (empty today; #638 fills it) may depend on `holler-pane` and `serde_json`, and **must not depend on
  `holler-cli`**: it is a dev-dependency of both the hub and the CLI, so a normal dependency back would make a cycle. Its
  envelope conformance helper (#638) parses the CLI envelope with `serde_json` alone.
```
The port (frozen by #637):
```
crates/holler-pane/src/ports.rs:36-52
/// The stream `watch` returns: changes in order, each carrying its [`Cursor`].
/// - `watch(since)` yields every change after `since`. `Cursor(0)` starts from the
///   beginning: the store first yields a put for every record it holds now (the
///   current state), then every later change.
/// - Passing the cursor of the last event seen back as `since` resumes without a
///   gap or a repeat.
/// - `next()` blocks for at most I5's bound and yields one of three things:
///   - `Ok(Some(change))`: the next change;
///   - `Ok(None)` (the item, not the end of the iterator): **idle**, nothing happened
///     within the bound. ... the stream stays usable. ...
///   - `Err(..)`: a failure. `Err(PaneError::Timeout)` means the store did not
///     answer within the bound (a wedged store), never "idle". Any error ends the
///     stream (call `watch` again).
pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;
crates/holler-pane/src/ports.rs:62-82
pub trait PaneStore: Send + Sync {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;
    fn list(&self) -> Result<Vec<Pane>, PaneError>;
    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
    /// Remove a pane's record if it is still at `expected_generation`. ... A stale
    /// generation is `generation-conflict`; a record that does not exist is
    /// `pane-not-found`, whatever `expected_generation` is (a missing record is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
}
crates/holler-pane/src/ports.rs:30-34    pub struct Cursor(pub u64);   // store-wide, strictly increasing; Cursor(0) is "from the beginning"
crates/holler-pane/src/pane.rs:259-268   pub struct PaneEvent { pub cursor: Cursor, pub name: PaneName, pub pane: Option<Box<Pane>> }  // None = deleted
crates/holler-pane/src/pane.rs:240-242   /// The profile the pane belongs to; a pane belongs to at most one.
                                         pub profile: Option<ProfileName>,
```
The one CAS rule, to be called, not rewritten:
```
crates/holler-pane/src/generation.rs:17-34
/// The generation a record moves to when a write that read `expected` is applied
/// to a record now at `current`. `Err(PaneError::Conflict)` when `expected != current` ...
pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
    if current != expected { return Err(PaneError::Conflict); }
    current.checked_add(1).ok_or_else(|| PaneError::StoreCorrupt { what: "a generation counter overflowed".to_owned() })
}
crates/holler-pane/src/generation.rs:10-14 (module doc)
//! A record that does not exist yet is at generation `0`, so a create names
//! `expected_generation: 0`, and its first stored generation is `1`.
//! The rule is written here once; the fakes of #638 and the stores of #639 and #661
//! all call [`next_generation`] instead of writing it again.
```
The decided store semantics:
```
docs/adr/ADR-0021.md:265-269  A record that does not exist is at generation 0, so a create names `expected_generation: 0` and is
  stored at 1. Each applied write adds one. The store sets the generation; the one a client submits is ignored. ...
  A write whose expected generation is not the current one is `generation-conflict` and changes nothing. `delete` checks
  that the record exists first (`pane-not-found` or `profile-not-found`, whatever the generation) and the generation second.
docs/adr/ADR-0021.md:276-277  **Known gap:** a record deleted and then created again restarts at generation 1 ...
docs/adr/ADR-0021.md:278-280  The membership rule is enforced on `pane/cas_put`: setting `Pane.profile` to P when the stored
  pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that names a pane of another
  profile (a detached spec) is not refused.
docs/adr/ADR-0021.md:498-501  2. **`pane-in-other-profile` runs inside the pane registry's compare-and-swap (section 8).** It
  needs only the stored pane record ... `check_membership` (#669) keeps the check that the named profile exists. #661 makes
  this one-line change in the registry code #639 creates ...
docs/adr/ADR-0021.md:227      | `pane/list`, `profile/list` | an array of records (`pane/list` sorted by name) |
docs/adr/ADR-0021.md:249-254  **Fail closed.** ... a failed state: every method, `watch` included, answers `store-corrupt` ...
docs/adr/ADR-0021.md:105-110  `watch(Cursor(0))` first yields a put for every record held now; resuming from the last cursor
  seen neither repeats nor skips. `next()` yields `Ok(Some(event))`, `Ok(None)` for idle within the bound (the stream stays
  usable), or an error that ends the stream; `timeout` means a wedged store, never idle.
```
The real implementation the suite must be runnable against later (read-only, branch `issue-639-implementation`, worktree
`.claude/worktrees/0639-pane-registry`, not on `main`):
```
crates/holler-hub/src/panes/mod.rs:76      pub const WATCH_WAIT: Duration = Duration::from_secs(4);
crates/holler-hub/src/panes/mod.rs:81-87   pub struct PaneStoreOptions { pub watch_wait: Duration, pub feed_retained: usize }
crates/holler-hub/src/panes/mod.rs:118     pub fn load_with(state: &HubState, options: PaneStoreOptions) -> Self {
crates/holler-hub/src/panes/mod.rs:125     impl PaneStore for PaneState {
crates/holler-hub/src/panes/store.rs:135   self.table.lock().unwrap_or_else(PoisonError::into_inner)
crates/holler-hub/src/panes/store.rs:165-182  cas_put: current = stored generation or 0; stored = Pane { generation: next_generation(current, expected)?, ..pane.clone() }
crates/holler-hub/src/panes/store.rs:186-201  delete: missing -> PaneNotFound first, then next_generation(current, expected)?
crates/holler-hub/src/panes/feed.rs:86-98  check_since: a cursor ahead of the head is `usage`
crates/holler-hub/src/panes/feed.rs:100-118 select: since 0 -> a put per live record by cursor; else the ring's events after since
crates/holler-hub/src/state.rs:25 (main)   pub fn from_root(root: PathBuf) -> Self {
```
So a hub test builds a fresh, empty `PaneState` in a temp dir with a short `watch_wait`; the temp dir must outlive the case.
Note the hub's idle reply on an empty registry answers cursor 0 (`store.rs:261-284`), so a watcher resuming from 0 gets
the current state, not the intermediate writes: the suite's watch cases therefore read one change after each write (see
the case table), which every correct store satisfies.

Lint and test conventions the new code must follow:
```
Cargo.toml:19-30          unwrap_used, expect_used, panic, unreachable, cognitive_complexity, too_many_lines,
                          struct_excessive_bools = "deny"; dead_code = "deny"
clippy.toml:6-7           cognitive-complexity-threshold = 15; too-many-lines-threshold = 100
scripts/lint.sh:43-52     warn at 600 lines per .rs file, fail at 900
crates/holler-pane/tests/ports_test.rs:1   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
```
The existing in-test store this slice does **not** reuse (another crate's test-only code; see "Extend vs new"):
`crates/holler-pane/tests/common/mod.rs:107-162` (`MemPaneStore`, `lock().unwrap()`, an empty `watch`).

## Dependency direction

`holler-pane-testkit` -> `holler-pane` (normal dependency, this slice). Nothing else. `holler-hub` and `holler-cli` already
dev-depend on the testkit (Evidence); neither manifest changes. No cycle: the testkit never names `holler-cli`, `holler-hub`
or an adapter crate. Cargo.toml changes in this slice, all in `crates/holler-pane-testkit/Cargo.toml`:

```toml
description = "Fake ports and the conformance suites every pane adapter and store must pass"

[dependencies]
# The ports the fakes implement and the suites drive (`PaneStore`, `Pane`, `PaneError`,
# `next_generation`); for every fake and suite in this crate.
holler-pane = { path = "../holler-pane" }
```
No `[dev-dependencies]` (the tests need only the crate and `holler-pane`, which a test reaches through the crate's normal
dependency). `cargo machete` passes (the dependency is used). No feature list, so `scripts/lint.sh` check 5 does not apply.
Slice 638b later adds `serde_json = { workspace = true }`.

## Public API of slice a (exact; T writes tests against these, F implements them)

No flat re-exports in `lib.rs`: every item is reached by its module path, so later slices never edit `lib.rs`.

```rust
// crates/holler-pane-testkit/src/fault.rs
/// A port method a fault can target and the call log records.
pub trait PortOp: Copy + Eq + std::fmt::Debug + Send + Sync + 'static {
    /// "<port>.<method>", e.g. "pane_store.cas_put"; also the `op` of a wedged call's `timeout`.
    fn name(self) -> &'static str;
}
/// A standing fault: it applies to every call until cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op: <op name> }`.
    /// The fake answers at once (it does not wait out I5's bound); add `set_delay` to make a caller's timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt`, `unavailable`).
    Fail(PaneError),
}
pub struct FaultSwitch<Op: PortOp> { /* private: Mutex<state> */ }
impl<Op: PortOp> FaultSwitch<Op> {
    pub fn new() -> Self;
    pub fn set(&self, fault: Option<Fault>);              // standing fault on/off
    pub fn fail_next(&self, op: Op, error: PaneError);    // one-shot, queued per op, FIFO
    pub fn set_delay(&self, delay: Option<Duration>);     // every call sleeps this long first (a slow call)
    pub fn calls(&self) -> Vec<Op>;                       // every call made through the port, in order, failed ones included
    pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError>;  // record, sleep (lock not held), standing fault, one-shot
}
impl<Op: PortOp> Default for FaultSwitch<Op> { /* = new() */ }

// crates/holler-pane-testkit/src/fixture.rs
/// A valid, deterministic `Pane` named `name` at generation 0, grid r1c1, no profile, no session of record,
/// harness port 48100, health unknown, scratch session/workspace names (never a live session name).
/// `usage` when `name` is not a valid pane name.
pub fn sample_pane(name: &str) -> Result<Pane, PaneError>;

// crates/holler-pane-testkit/src/pane_store.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneStoreOp { Get, List, CasPut, Delete, Watch, WatchNext }
impl PortOp for PaneStoreOp { /* "pane_store.get" ... "pane_store.watch_next" */ }

/// An in-memory `PaneStore`. Not `Clone`: share it with `Arc` (a copy would split the store).
pub struct FakePaneStore { /* private: Arc<inner> so a Watch iterator owns a handle */ }
impl FakePaneStore {
    pub fn new() -> Self;                                                        // empty, idle wait zero
    pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError>;  // each created at expected 0 (stored at 1)
    pub fn set_idle_wait(&self, wait: Duration);    // how long next() waits for a write before Ok(None); default zero
    pub fn faults(&self) -> &FaultSwitch<PaneStoreOp>;
    /// Another writer: store `pane` unconditionally at the current generation + 1 (0 + 1 for a new record),
    /// publish its event, bypass faults and the call log. Returns the stored record.
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError>;
    /// Another writer removes the record (event with `pane: None`); `pane-not-found` if missing. Bypasses faults and the log.
    pub fn concurrent_delete(&self, name: &PaneName) -> Result<(), PaneError>;
}
impl Default for FakePaneStore { /* = new() */ }
impl PaneStore for FakePaneStore { /* see "Fake behaviour" */ }

// crates/holler-pane-testkit/src/conformance/mod.rs
/// One conformance case that did not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFailure { pub case: &'static str, pub detail: String }
/// Ok when every case held; otherwise every failure, in case order.
pub type Conformance = Result<(), Vec<CaseFailure>>;
pub mod pane_store;   // plus the stub submodules of slices c, d, e

// crates/holler-pane-testkit/src/conformance/pane_store.rs
/// The case ids `run_pane_store_conformance` runs, in order (the table below).
pub fn pane_store_cases() -> Vec<&'static str>;
/// Run every case against a fresh, empty store. `fresh` is called once per case and returns the store and a guard the
/// suite keeps alive for that case only (a `TempDir`, or `()` for the fake).
pub fn run_pane_store_conformance<S, K, F>(fresh: F) -> Conformance
where S: PaneStore, F: FnMut() -> (S, K);
```

How each implementation runs the suite (doc comment on `run_pane_store_conformance`, in a `text` fence so it is not a
doctest; the testkit cannot name the hub):
```text
// the fake (this slice):  assert_eq!(run_pane_store_conformance(|| (FakePaneStore::new(), ())), Ok(()));
// the hub (#649 or #661, holler-hub/tests/...):
//   run_pane_store_conformance(|| {
//       let dir = tempfile::tempdir().expect("temp dir");
//       let opts = PaneStoreOptions { watch_wait: Duration::from_millis(50), feed_retained: 1024 };
//       (PaneState::load_with(&HubState::from_root(dir.path().to_path_buf()), opts), dir)
//   })
// the CLI's pane/* client (#649): a fresh hub per case, the client as S, the hub's handle as K.
```
Library code returns `Result` everywhere (no `unwrap`/`expect`/`panic`/`assert!` in `src/`). A poisoned lock is taken with
`unwrap_or_else(PoisonError::into_inner)`, the hub's pattern (`store.rs:135` on the 0639 branch).

## Fake behaviour (`FakePaneStore`)

- One `Mutex` guards the records (a `BTreeMap<PaneName, Pane>`, so `list` is sorted by name), each record's last-change
  cursor, and the change log; one `Condvar` wakes waiting watchers. Every port method first calls
  `faults().enter(op)`; an error from it is returned and nothing changes.
- `cas_put`: `next_generation(stored generation or 0, expected)`; the stored record is the submitted pane with only the
  generation replaced. Then the membership rule: if the stored record has `profile: Some(P)` and the submitted one has
  `Some(Q)` with `P.slug() != Q.slug()`, `PaneError::PaneInOtherProfile { what }` naming the pane and both profiles, and
  nothing changes. `None` -> `Some(P)`, `Some(P)` -> `None` and `Some(P)` -> `Some(P)` are allowed. The fake does not check
  that P exists (the hub does that in `check_membership`, outside the port). Each applied write takes cursor `head + 1`
  and publishes a `PaneEvent` with the stored record. Cursor overflow is `store-corrupt` (as `next_generation`).
- `delete`: a missing record is `pane-not-found` whatever the generation; then `next_generation`; then remove and publish
  an event with `pane: None`.
- `watch(since)`: `usage` when `since` is ahead of the head (the hub's rule, `feed.rs:86-98` on 0639). From `Cursor(0)`:
  first one put per live record, each carrying that record's last-change cursor, in cursor order, then every later change.
  From any other cursor: every change after it, in order (the fake keeps its whole history, so it never collapses writes).
  Each `next()` calls `enter(WatchNext)` first; an error is yielded once and ends the stream (`None` afterwards). With
  nothing pending, `next()` waits up to the idle wait on the `Condvar` and yields `Ok(None)` if no write came.
- `src/feed.rs` holds the generic part (the change log with its head, cursor allocation and "events after `since`", and
  the `Watch` iterator over a polled source), crate-private and generic over the event type so slice 638c's profile store
  reuses it rather than writing a second feed.

## Conformance cases: `run_pane_store_conformance`

Each case gets a fresh empty store. Panes come from `sample_pane`. "Unchanged" means `get` returns the record exactly as
before and `list` is equal to before.

| # | Case id | What it asserts |
|---|---|---|
| 1 | `get-missing-is-none` | `get` of a name never stored is `Ok(None)`. |
| 2 | `list-empty` | `list` of an empty store is `Ok(vec![])`. |
| 3 | `create-at-zero-stored-at-one` | `cas_put(p, 0)` returns `p` with generation 1; `get` returns the same record. |
| 4 | `submitted-generation-ignored` | `cas_put` of `p` with `p.generation = 99` at expected 0 returns and stores generation 1. |
| 5 | `create-over-existing-conflicts` | With `p` stored at 1, `cas_put(p2, 0)` (same name) is `generation-conflict`; unchanged. |
| 6 | `update-bumps-by-one` | At 1, `cas_put(p2, 1)` returns `p2` at 2; `get` returns it. |
| 7 | `stale-generation-conflicts` | At 2, `cas_put(p3, 1)` is `generation-conflict`; unchanged at 2. |
| 8 | `expected-ahead-conflicts` | At 1, `cas_put(p2, 5)` is `generation-conflict`; on a missing name, `cas_put(q, 3)` is `generation-conflict` and `get(q)` stays `None`. |
| 9 | `list-sorted-by-name` | After creating `hj-c3r1`, `hj-c1r1`, `hj-c2r1` in that order, `list` returns the three stored records in name order (ADR-0021:227). |
| 10 | `delete-at-current-generation` | At 1, `delete(name, 1)` is `Ok`; `get` is `None`; `list` lacks it. |
| 11 | `delete-stale-conflicts` | At 2, `delete(name, 1)` is `generation-conflict`; the record is still there at 2. |
| 12 | `delete-missing-is-pane-not-found` | `delete` of a missing name at expected 0 and at expected 7 is `pane-not-found` both times (existence before generation). |
| 13 | `recreate-after-delete-starts-at-one` | Create, delete, then `cas_put(p, 0)` stores generation 1 (the known gap, ADR-0021:276-277, pinned). |
| 14 | `watch-from-zero-yields-current-state` | Create a, create b, update a, delete b, create c; `watch(Cursor(0))` yields puts of exactly a (generation 2) and c, each equal to the stored record, cursors strictly increasing, no event for b; then `Ok(None)`. |
| 15 | `watch-follows-each-change` | `watch(Cursor(0))` on an empty store yields `Ok(None)`; then after each of create a, update a, delete a, one `next()` yields exactly that change: puts carrying the stored record (generation 1, then 2), the delete with `pane: None`; cursors strictly increasing. |
| 16 | `watch-resumes-without-gap-or-repeat` | Create a and b; drain `watch(Cursor(0))` and keep the last event's cursor `c`; create d; `watch(c)` yields exactly d, then `Ok(None)`. |
| 17 | `watch-idle-is-ok-none-and-stays-usable` | After draining, `next()` is `Some(Ok(None))` (not `None`, not `Err`); a write after it is then yielded by the same iterator. |
| 18 | `failed-write-changes-nothing` | Resume a watch at the last cursor; a stale `cas_put`, a stale `delete` and a `delete` of a missing name each fail and leave `get`/`list` unchanged, and the watch then yields only `Ok(None)` (no event for a failed write). |
| 19 | `pane-in-other-profile` | Create a pane with `profile: Some("Alpha")`; `cas_put` with `Some("Beta")` at 1 is `pane-in-other-profile`, unchanged; `cas_put` with `None` at 1 succeeds (generation 2); then `Some("Beta")` at 2 succeeds (generation 3). |

The drain helper reads until `Ok(None)` with a limit of 1,000 items; hitting the limit, `None` (end of stream) or an `Err`
before `Ok(None)` is a failure of the case, with the reason in `detail`. A store's own `watch_wait` bounds each idle read;
the hub runs the suite with a short window (its concern, not the port's).

Case 19 is in the suite because ADR-0021 section 8 makes it a rule of `pane/cas_put` inside the registry's CAS. The hub's
`PaneState` on the 0639 branch does not do it yet; #661 adds it (ADR-0021:498-501), so the hub passes all 19 once #661 has
merged. That is expected, and the suite does not offer a way to skip a case.

## Acceptance criteria

Test names are what T authors (RED first). All test files start with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638`.

1. **Manifest.** `crates/holler-pane-testkit/Cargo.toml` has exactly the `holler-pane` dependency above with its comment
   and the new description; no other dependency, no `[dev-dependencies]`. `cargo machete` passes.
   `cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'` prints nothing.
2. **Layout.** `src/lib.rs` declares exactly `pub mod conformance; pub mod envelope; pub mod fault; mod feed; pub mod fixture;
   pub mod harness; pub mod herdr; pub mod host; pub mod pane_store; pub mod prober; pub mod profile_scope;
   pub mod profile_store;` with a module doc listing each and the slice that fills it, and no `pub use`.
   `src/conformance/mod.rs` declares `pub mod harness; pub mod herdr; pub mod host; pub mod pane_store;
   pub mod profile_scope; pub mod profile_store;`. The stub files (`envelope.rs`, `harness.rs`, `herdr.rs`, `host.rs`,
   `prober.rs`, `profile_scope.rs`, `profile_store.rs`, and `conformance/{harness,herdr,host,profile_scope,profile_store}.rs`)
   each hold only a `//!` doc naming the slice (638b to 638e) and what it will hold. Checked by A in the diff review.
3. **The fake passes its suite.** `tests/pane_store_conformance_test.rs`:
   - `the_fake_passes_the_pane_store_conformance_suite`: `run_pane_store_conformance(|| (FakePaneStore::new(), ()))` is
     `Ok(())`.
   - `the_suite_runs_the_documented_cases`: `pane_store_cases()` equals the 19 ids of the table, in order.
   - `the_guard_lives_for_the_case`: with `K` a type whose `Drop` sets a shared flag, the store built by `fresh` sees the
     flag unset during the case (a `PaneStore` wrapper that checks it on each call) and every guard is dropped by the end.
4. **Mutation check: a broken store fails, on the named case.** Same file; each mutant is a test-local wrapper around
   `FakePaneStore` that breaks one rule, and the test asserts the suite returns `Err` whose failures include the named case:
   - `a_store_without_cas_fails` (`cas_put` writes at whatever the current generation is) -> `stale-generation-conflicts`.
   - `a_store_that_keeps_the_submitted_generation_fails` (returns the submitted generation) -> `submitted-generation-ignored`.
   - `a_store_that_checks_generation_before_existence_fails` (missing name with expected != 0 -> `generation-conflict`)
     -> `delete-missing-is-pane-not-found`.
   - `a_store_that_repeats_on_resume_fails` (`watch(since)` delegates `watch(since - 1)`) -> `watch-resumes-without-gap-or-repeat`.
   - `a_store_that_ends_the_stream_when_idle_fails` (maps `Ok(None)` to end of iterator) -> `watch-idle-is-ok-none-and-stays-usable`.
   - `a_store_that_lets_a_pane_change_profile_fails` (clears the stored profile before the put) -> `pane-in-other-profile`.
5. **Faults can be injected and observed.** `tests/fake_pane_store_test.rs`:
   - `a_wedged_store_times_out_every_method`: `set(Some(Fault::Wedged))`; `get`, `list`, `cas_put`, `delete`, `watch` each
     return `PaneError::Timeout { op }` with `op` = `"pane_store.<method>"`; nothing was written; after `set(None)` they work.
   - `a_wedged_store_ends_an_open_watch`: an open watch's `next()` yields `Err(Timeout { op: "pane_store.watch_next" })`
     once, then `None`.
   - `a_corrupt_store_fails_closed_everywhere`: `set(Some(Fault::Fail(StoreCorrupt { .. })))`; every method, `watch`
     included, returns that error; after clearing, the records written before are intact.
   - `fail_next_is_one_shot_and_per_op`: `fail_next(CasPut, Unavailable { .. })`; the first `cas_put` fails and writes
     nothing; `get` is unaffected; the second `cas_put` succeeds. Two queued errors come out in order.
   - `a_slow_call_takes_at_least_the_delay`: `set_delay(Some(50 ms))`; `get` takes at least 50 ms (`Instant`).
   - `calls_are_recorded_in_order`: after `get`, a failed `cas_put`, `list`, `calls()` is `[Get, CasPut, List]`;
     `concurrent_put` and `concurrent_delete` add nothing.
   - `a_concurrent_put_makes_the_next_cas_stale`: read at g; `concurrent_put`; `cas_put(.., g)` is `generation-conflict`;
     a watch sees the concurrent write.
   - `a_concurrent_delete_makes_the_record_vanish`: after `concurrent_delete`, `get` is `None`, `delete(.., g)` is
     `pane-not-found`, and `concurrent_delete` of a missing name is `pane-not-found`.
   - `a_watch_ahead_of_the_head_is_usage`: `watch(Cursor(5))` on an empty store is `usage`.
   - `the_idle_wait_wakes_on_a_write`: `set_idle_wait(5 s)`; a thread writes after 50 ms; `next()` yields that event in
     well under 5 s. With the default (zero) idle wait, `next()` on an idle stream returns `Ok(None)` at once.
   - `seeded_stores_hold_each_pane_at_generation_one` and `sample_pane_is_valid_and_deterministic` (two calls are equal;
     `sample_pane("not a name")` is `usage`).
   - `the_fake_is_send_and_sync_and_its_watch_is_send` (a compile-time bound check).
6. **No other crate changes.** `git diff --name-only origin/main...HEAD` lists only Blast-radius paths. The hub's and the
   CLI's `testkit_links` tests still pass unchanged.
7. **CHANGELOG.** One entry under `## [Unreleased]` / `### Enhancements`, after the #670 entry (`CHANGELOG.md:57`): the
   pane test kit's first part, a fake pane registry with fault injection (a wedged or corrupt store, a one-shot error, a slow
   call, another writer's change) and a conformance suite any pane registry runs against itself; the profile, Herdr, host
   and harness fakes and the JSON-envelope checker follow; link [#638](https://github.com/Performant-Labs/holler/issues/638).
8. **Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`,
   `bash scripts/test-hooks.sh` pass. Every new `.rs` file passes `rustfmt --check --edition 2021`. No new `.rs` file
   reaches 600 lines; no function exceeds 100 lines or cognitive complexity 15 (one function per conformance case).

## Files

New, all under `crates/holler-pane-testkit/`:
- `src/fault.rs` (~130 lines), `src/feed.rs` (~150, crate-private), `src/fixture.rs` (~70), `src/pane_store.rs` (~200),
  `src/conformance/mod.rs` (~70: `CaseFailure`, `Conformance`, the crate-private `expect_code` and `drain` helpers the later
  suites reuse), `src/conformance/pane_store.rs` (~400, one function per case plus the runner and the case table).
- Stubs (one `//!` line each): `src/{envelope,harness,herdr,host,prober,profile_scope,profile_store}.rs`,
  `src/conformance/{harness,herdr,host,profile_scope,profile_store}.rs`.
- Tests: `tests/pane_store_conformance_test.rs` (~250, suite and mutants), `tests/fake_pane_store_test.rs` (~250, faults).

Changed: `crates/holler-pane-testkit/Cargo.toml`, `crates/holler-pane-testkit/src/lib.rs`, `CHANGELOG.md`.

## Extend vs new

- **Extend** the existing empty crate (`holler-pane-testkit`, registered by #637) and its existing manifest; implement the
  existing frozen trait (`PaneStore`); call `next_generation` for every CAS (`generation.rs:25`); return the closed
  `PaneError` variants (`Conflict`, `PaneNotFound`, `PaneInOtherProfile`, `Timeout`, `StoreCorrupt`, `Unavailable`,
  `Usage`), never a new code; compare profiles with `ProfileName::slug` (`profile.rs:72`).
- **New, no parallel path:** the fault switch and the feed are written once, generic, and are the only ones in the crate;
  slices c to e reuse them. Nothing in `holler-pane` or the hub is copied: the hub's `feed.rs` and `store.rs` are
  `pub(crate)` in another crate the testkit must not depend on, and the fake keeps its whole history, so it needs neither the
  hub's ring nor its collapse rule.
- **Accepted near-duplicate:** `crates/holler-pane/tests/common/mod.rs:107-162` (`MemPaneStore`) and `pane_json()` are
  test-only code of `holler-pane`. `holler-pane` cannot use the testkit (the testkit depends on it; a dev-dependency back
  builds a second copy of `holler-pane`, whose types would not match), and the file is outside this blast radius. It stays.

## Decisions already made (operator and epic)

- The testkit depends on `holler-pane` (and later `serde_json`) only, never `holler-cli` (ADR-0021:184-186, issue amendment
  2026-10-09).
- `PaneStore::delete(name, expected_generation)` exists and checks existence first (epic ruling 9, `ports.rs:73-78`).
- Generations: create at 0, stored at 1, submitted generation ignored, a stale one is `generation-conflict`, the known
  recreate gap (ADR-0021:265-277).
- `pane-in-other-profile` is checked inside the pane registry's CAS (ADR-0021 "Decisions taken" item 2).
- `Watch` items are `Result<Option<T>, PaneError>`; idle is `Ok(None)`; an error ends the stream; `timeout` is never idle
  (`ports.rs:36-52`). The idle cursor of the hub's long-poll is the hub's concern (#639), not the port's.
- Verbs run CLI-side against the ports; the hub is the store only (epic ruling 1).

## Decisions made in this brief

1. **The split** (top of this brief), with stub files pre-created so slices b to e run in parallel without touching
   `lib.rs` or `conformance/mod.rs`.
2. **A suite returns `Conformance` (`Result<(), Vec<CaseFailure>>`), it does not assert.** The workspace denies `panic` and
   `unwrap` in library code, and a value lets the mutation tests check which case failed.
3. **`fresh` returns `(store, guard)`**, so the hub can keep a temp dir alive per case without a wrapper type.
4. **The fake's watch from a cursor ahead of the head is `usage`**, as the hub does; it is not a suite case (the port does
   not say it).
5. **The fake answers a wedged call at once** with `timeout`; `set_delay` models slowness separately.
6. **The fake's membership check compares slugs** and does not check that the profile exists (the hub's `check_membership`
   does, outside the port).
7. **No flat re-exports**; module paths only.

## Out of scope

Everything in slices 638b to 638e (the appendix); any change to `holler-pane`, `holler-hub`, `holler-cli`, the adapters or
any other manifest; running the suite against the hub (#649, or #661 after its membership change); the hub's retained-window
collapse (a hub rule, not a port rule); protocol v2, the closed 22-row wire catalog and every golden file (unchanged);
`holler-cli`'s `autotests = false` (not touched).

## Test plan

RED (T): write the two test files against the API above; they fail to build (`FakePaneStore`, `run_pane_store_conformance`,
`FaultSwitch` do not exist). Confirm with `cargo test -p holler-pane-testkit` that the failure is the missing items, not a
typo. GREEN (F): the manifest, `lib.rs` and the stubs, then `fault.rs`, `feed.rs`, `fixture.rs`, `pane_store.rs`,
`conformance/`. Then the guards of AC 8. A (anti-duplication) checks that no second CAS rule or feed exists and that the
case functions share `expect_code` and `drain`.

## Risks

- **The hub fails case 19 until #661.** Expected and documented in the suite's doc comment; whoever wires the hub's run
  (#649 or #661) does it after #661's change.
- **A timing-sensitive test** (`the_idle_wait_wakes_on_a_write`, `a_slow_call_takes_at_least_the_delay`): assert only lower
  bounds and a generous upper bound (seconds), never an exact duration.
- **Stub files look speculative.** They follow #637's precedent (stub files so that no two stories edit one file); each names
  its slice.
- `conformance/pane_store.rs` is the largest file (~400 lines); if it nears 600, split the watch cases into
  `conformance/pane_store_watch.rs` (declared inside `conformance/pane_store.rs`, so `conformance/mod.rs` is untouched).

## Blast radius

`crates/holler-pane-testkit/**` (the issue's radius), `CHANGELOG.md` (house rule), `docs/handoffs/638*` (pipeline
artifacts). Not changed: any other crate, any other `Cargo.toml`, `Cargo.lock` beyond the testkit's new dependency edge,
any ADR, protocol doc, golden file or CLI fixture. The repository is public: no personal names in code, comments, tests or
the changelog.

---

## Appendix: fixed for the later slices (not built in this run)

Recorded so each later slice's brief starts from settled names and cases. Each later slice's O run may refine details, but
not the module names, which this slice creates.

### 638b: the JSON-envelope helper (`src/envelope.rs`; adds `serde_json`; needs #676 merged)

```rust
pub struct Envelope { pub schema_version: u64, pub ok: bool, pub data: serde_json::Value, pub error: Option<EnvelopeError> }
pub struct EnvelopeError { pub code: String, pub message: String }
pub enum EnvelopeFault { /* NotJson, TextBefore, TextAfter, NotAnObject, MissingKey(&'static str), UnknownKey(String),
    SchemaVersion, ExitCodeUnknown(i32), OkDisagreesWithExit, ErrorWhileOk, NoErrorWhenFailed, DataOnFailure,
    CodeNotKebab(String), MessageNotOneLine, ClassDisagreesWithExit, EmptyStream, NotLastFailure */ }
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault>;
pub fn check_ndjson(stdout: &str, exit_code: i32) -> Result<Vec<Envelope>, EnvelopeFault>;
```
Cases: stdout is exactly one JSON object and an optional final newline (text before -> `TextBefore`, after -> `TextAfter`);
keys exactly `schema_version`, `ok`, `data`, `error`; `schema_version` is the integer 1; exit codes 0, 1, 2, 3 only; `ok` is
true exactly for exit 0 and false for 1, 2 and 3; `error` is null exactly when `ok`; on failure `data` is null; `code`
passes `holler_pane::error::is_valid_code`; `message` is non-empty with no `\n` or `\r`; `class_of(code).exit_code()` equals
the exit code (#676; `usage` pairs with 2). NDJSON: every line is checked alone; with exit 0 every line is `ok`; otherwise
every line but the last is `ok` and the last carries the failure (the shape of `emit_stream`). Tests: a good envelope per
exit code; a broken envelope (`{`), text before it, an `ok`/exit mismatch (`ok: true` with exit 3; `ok: false` with exit 0),
`error` set while `ok`, a two-line message, `schema_version` 2, an extra `detail` key are each rejected with their fault.

### 638c: `FakeProfileStore`, `FakeProfileScope` (`src/profile_store.rs`, `src/profile_scope.rs`)

`FakeProfileStore` mirrors `FakePaneStore` (`FaultSwitch<ProfileStoreOp>`, the shared feed, `concurrent_put`,
`concurrent_delete`, `set_idle_wait`, a settable clock for `at`/`created`/`updated`). `run_profile_store_conformance(fresh)`
cases: create at 0 stored at 1 with one log entry `{generation 1, actor, Created}`; update bumps by one and logs
`Updated { summary }`; stale and ahead generations are `generation-conflict` with no log entry and no event; a different
name with the same slug at expected 0 is `profile-exists`; `delete` of a missing name is `profile-not-found` at any
generation, a stale one `generation-conflict`, a good one logs `Deleted` and publishes an event with `profile: None`; the log
is append-only and oldest first, and stays readable after a delete; `log` of a never-created name is `profile-not-found`;
`rename` is `not-implemented` (PROPOSED, #665); the watch cases 14 to 18 of the pane suite, for profiles; a profile decoded
from JSON with `"env": ["TOKEN=x"]` fails with `profile-secret-refused` and with `[" "]` with `env-name-invalid`, through the
one `EnvVarName` guard (no second scan); the store refuses nothing else about env (the type cannot hold a value).
Faults: wedged, `store-corrupt` everywhere, one-shot errors, slow calls.

`FakeProfileScope::new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor)`.
`run_profile_scope_conformance<S, F>(build: F)` with `F: FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S`: the suite
seeds the two fakes, builds the scope under test over them and inspects them. Cases: `resolve(P, None)` is every pane whose
`Pane.profile` is P (membership is `Pane.profile` only; a detached spec adds no pane); `resolve(P, Some(member))` is that
pane; `resolve(P, Some(non-member))` is `pane-not-in-profile`; a missing P is `profile-not-found`; `edit_spec(Some(P), ..)`
with a succeeding act bumps P's generation **once**, holds the edit (`Set` replaces the pane's entry, `Remove` drops it),
calls the act exactly once, and inside the act the store already shows the edit (profile first, I8); with a failing act it
returns the act's error, P's **specs are equal to before and its generation has moved by two** (edit, then the restoring
write; log shows both), and no pane record changed (ADR-0021 section 8, Decisions taken item 1); a conflict on the first
write (`fail_next(CasPut, Conflict)`) is `generation-conflict` with the act never called; another writer moving P during a
failing act makes the restore conflict and the call `profile-conflict` naming P, leaving the other writer's version;
`edit_spec(None, ..)` runs the act once and makes no profile store call (`calls()` empty) and returns `Ok(None)`; a missing
P is `profile-not-found` before the act; a pane that belongs to another profile is `pane-in-other-profile` before any
write. Mutants: a scope that writes the profile after the act; one that does not restore on a failed act.

### 638d: `FakeHerdr`, `FakeProber` (`src/herdr.rs`, `src/prober.rs`)

The fake works in `GridPos` cells only; Herdr's own rectangles and axis order are #640's alone (issue amendment
2026-10-08, grid; spike `docs/research/herdr-api-spike.md:47-50`). Workspaces are declared with a size
(`with_workspace(name, rows, cols)`); `ensure_pane` outside it is `grid-out-of-range`. Pane ids are opaque, minted as
`w<N>:p<M>` with a base-36 counter, never reused after a close, and unchanged for the other panes when one closes (spike
lines 44-46, 161-163). `ensure_pane` on an occupied cell returns the existing pane and never moves or replaces another pane
(the adapter must never `layout.apply` onto an existing tab, which replaces it, spike lines 51-53). Split-only mode accepts a
new cell only to the right of or below an existing pane, refusing any other placement as an absolute placement. `version()`
has two selectable versions: protocol 22 (`0.9.1-preview.2026-09-21-0ff0f27e2226`) answers its string; the other answers
`herdr-version-unsupported` naming "Herdr protocol 22 (0.9.1)" (spike lines 450-453). Faults: wedged, a vanished pane (its
shell exited: later calls on it are `pane-not-found`), a slow call; `send_text`/`send_keys` are recorded so verb tests can
assert I4. Suite `run_herdr_conformance(fresh)` (fixture: a scratch session with one 2x1 workspace): `r2c1` is accepted and
reads back as `r2c1` in `snapshot`; `r1c2` is `grid-out-of-range` (a transposing implementation fails); ensure is idempotent;
ids are unique and stable when a sibling closes; close of an unknown or closed pane is `pane-not-found`; `read` returns at
most `max_lines` lines. ASSUMPTION comments for #640 to confirm: how a real adapter learns a workspace's size, and what a
version beyond protocol 22 looks like (spike section 13: no second build was compared). `FakeProber` returns a scripted
`ProbeResult` per argv (each of `Ok`, `Failed`, `Error`).

### 638e: `FakeHost`, `FakeHarness` (`src/host.rs`, `src/harness.rs`)

`FakeHost`: `ensure_session` is idempotent; `run` records each `Argv`; `ps` lists the session's pids; `stop_owned` empties
them; faults as above. `FakeHarness` models the OpenCode spike (`docs/research/opencode-pane-spike.md`): servers by port, one
data directory shared by every server by default, so `list_sessions` on any port includes sessions created on another
(verified, lines 75-78); session ids `ses_` plus an opaque suffix; `select_session` and `abort` of an unknown id are
`session-not-found` (the adapter checks `GET /session/:id` first, lines 171-172, 237); `shown_session` is `Some(id)` while a
TUI shows the session, `None` with no TUI, on the home screen, or after the shown session was deleted (lines 137-142, 222).
Quirk switches model raw OpenCode for verb tests: `select-session` acknowledged with no TUI and the screen unchanged (lines
119-124), and `abort` of an unknown id acknowledged (lines 171-172); a frozen server (`SIGSTOP`, lines 189-191) makes `health`
`Ok(false)` and every other call `timeout`; a killed server makes `health` `Ok(false)` and other calls `unavailable`; a
session deleted under a TUI sends its `shown_session` to `None`. Suites check the port contract (the quirk switches are off),
so a store run with a quirk on is a mutant that fails them. ASSUMPTION comments for #642 to confirm: aborting a model turn,
whether Herdr exposes a pane's terminal title, and `select-session` across project directories (the spike's "Not verified").
