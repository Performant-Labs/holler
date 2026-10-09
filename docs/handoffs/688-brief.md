# Brief: #688 the pane test kit, slice c (part 2): `FakeProfileScope` and the `ProfileScope` conformance suite

Repo: Performant-Labs/holler. Issue: #688 (part of #638, epic #633; the second half of #682). Rigor: in-session. UI
surface: no. Kind: feature (test kit).

**Branch:** `issue-688-implementation`, based on `9d61c9f` (`origin/main`: slices a, b, d and e of #638 and part 1 of slice c,
#682/#691, merged). **Design (D):** N/A (no UI surface). **Forward-compat:** done, see "Forward-compat" below (the real
`ProfileScope` of #663 runs this suite). **Decision record:** ADR-0021 section 8 ("the I8 write order") and its "Decisions
taken" items 1 and 2. The issue is the source of truth; its settled design is the appendix "Fixed for part 2" of the #682
brief, which is carried inline below because that brief never reached `main` (the issue's pointer to
`docs/handoffs/682-brief.md` is dead). Where this brief differs from that appendix, "Where the appendix is stale" says so.

**Needs operator:** none.

## Problem

`holler-pane-testkit` has a fake `ProfileStore` and a fake `PaneStore`, each with its conformance suite. Its
`profile_scope` modules are still one-line stubs. Every spec-editing verb (`launch`, `relaunch`, `close` with `--profile`)
goes through `ProfileScope::edit_spec`, and every scoping verb through `ProfileScope::resolve`, so their tests need an
in-memory `ProfileScope`, and the real one (#663, `holler-cli/src/pane/profile_scope.rs`) needs a suite that pins the I8
write order: the profile is written first, a failed act restores the specs by a second write (the generation moves by
two), and a conflict on that restoring write is `profile-conflict`. This run adds both, built over the two existing fakes.

## Evidence (verbatim, as of `9d61c9f`)

The port (frozen by #637):
```
crates/holler-pane/src/profile.rs:259-274
/// What a spec edit does to a profile's entry for one pane.
pub enum SpecEdit {
    /// Set the pane's entry to this spec (boxed: a spec is large).
    Set(Box<ProfileSpec>),
    /// Remove the pane's entry.
    Remove,
}
/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}

crates/holler-pane/src/profile.rs:373-404
/// The helper every `--profile` verb uses to scope itself to a profile and to edit a
/// spec in one transaction with the live change. Implemented in
/// `holler-cli/src/pane/profile_scope.rs` (#663); frozen by #637.
/// **Blocking.** ... An implementation is `Send + Sync`.
pub trait ProfileScope: Send + Sync {
    /// The profile and the panes of it a verb acts on. With no `pane`, every pane
    /// of the profile; with a named pane, just that one, which must belong to the
    /// profile (`pane-not-in-profile` otherwise). A missing profile is
    /// `profile-not-found`.
    fn resolve(&self, profile: &ProfileName, pane: Option<&PaneName>) -> Result<ResolvedScope, PaneError>;

    /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
    /// transaction (I8): the profile is written with a compare-and-swap on its
    /// generation first, then `act` runs, then the result is recorded; if `act`
    /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
    /// Returns the edited profile. With `profile: None` it runs only `act` and
    /// touches no profile (and returns `None`).
    fn edit_spec(&self, profile: Option<&ProfileName>, pane: &PaneName, edit: &SpecEdit,
                 act: &mut dyn FnMut() -> Result<(), PaneError>) -> Result<Option<Profile>, PaneError>;
}
crates/holler-pane/src/profile.rs:175-177   /// [`ProfileSpec::pane`] is plain text, not a [`PaneName`]: a spec may name a pane
                                             /// that does not exist or belongs to another profile (a detached spec ...). Membership is `Pane.profile` only.
crates/holler-pane/src/pane.rs:240-242       /// The profile the pane belongs to; a pane belongs to at most one.
                                             pub profile: Option<ProfileName>,
crates/holler-pane/src/ports.rs:227-235      pub struct Ports<'a> { pub pane_store: &'a dyn PaneStore, pub profile_store: &'a dyn ProfileStore, ...
                                             pub scope: &'a dyn ProfileScope, pub prober: &'a dyn Prober }
```
The decided write order and its compensation:
```
docs/adr/ADR-0021.md:285-302
**Decided (the epic's order, as the merged `ProfileScope::edit_spec` documents it): the I8 write order.** For a
spec-editing verb with `--profile P`:
1. **Plan.** Read P at generation `g` and the pane record; compute P with the edit (`SpecEdit::Set` or `SpecEdit::Remove`).
2. **Profile CAS first.** `ProfileStore::cas_put(P_edited, g, actor)`. A conflict here comes before anything live changed:
   the verb exits 1 with `generation-conflict` and can be run again.
3. **Act** (skipped with `--spec-only`), then observe.
4. **Record.** Write the pane record with its own compare-and-swap (`cas_put`, or `delete` for `close`).
5. **If the act fails, nothing is recorded in P:** the pane record is not written, and the profile is put back by a
   compensating `cas_put` of its previous specs at `g + 1`. P's specs then equal what they were before the verb, and its
   log shows the edit and its reversal.
6. **A conflict after the act fails loudly.** If the compensating write of step 5 conflicts (another writer moved P during
   the act), the verb exits 1 with `profile-conflict`, naming P, and prints the reconcile step: ...
With `profile: None`, `edit_spec` runs only the act and touches no profile.
docs/adr/ADR-0021.md:536-538  1. **The I8 compensation (section 8): the epic's order stays.** The profile is written first. A failed act restores the
   profile's specs by a second write, so the specs equal what they were but the generation has moved by two and the log
   shows the edit and its reversal. ... the conformance case in #638 [is] amended from "generation equal" to "specs equal".
docs/adr/ADR-0021.md:540-541  2. **`pane-in-other-profile` runs inside the pane registry's compare-and-swap (section 8).** ...
docs/adr/ADR-0021.md:167      | I8 | ... | Fail `act` in `edit_spec` on the fake: P's specs are equal to before and no pane record changed; succeed it: P holds the edit and the pane record names P. |
docs/adr/ADR-0021.md:190-192  **Decided: where the I8 transaction helper lives.** The trait `ProfileScope` is in `holler-pane/src/profile.rs` (frozen by
  #637). The real implementation is in `holler-cli/src/pane/profile_scope.rs` (#663) and the fake is in `holler-pane-testkit`
  (#638). The spec-editing verbs call `edit_spec` and do not write the profile themselves.
```
The error variants this run returns (all closed codes; no new code):
```
crates/holler-pane/src/error.rs:425,431,434,443,446,409
    Conflict,                               // generation-conflict
    ProfileConflict { what: String },       // profile-conflict: "the profile moved after the live change ... `what` names the profile"
    ProfileNotFound { what: String },       // profile-not-found
    PaneNotInProfile { what: String },      // pane-not-in-profile: "`what` names both"
    PaneInOtherProfile { what: String },    // pane-in-other-profile: "`what` names both"
    Usage { message: String },              // usage
```
What part 1 and slice a left, which this run reuses (all under `crates/holler-pane-testkit/`):
```
src/profile_scope.rs:1-3                 //! `FakeProfileScope`, a `ProfileScope` over a profile store and a pane store: ... Empty stub
                                         //! declared by #638 so that no two slices edit `lib.rs`; slice c (#682) fills it.
src/conformance/profile_scope.rs:1-2     //! The `ProfileScope` conformance suite. Empty stub ...; slice c (#682) fills it.
src/lib.rs:42 / src/conformance/mod.rs:21   pub mod profile_scope;   (both already declared; neither file changes)
src/profile_store.rs:127-136             pub fn seeded(profiles: impl IntoIterator<Item = Profile>, actor: &Actor) -> Result<Self, PaneError>
                                         // each stored at 1 with one Created entry; bypasses the faults and the call log
src/profile_store.rs:152                 pub fn faults(&self) -> &FaultSwitch<ProfileStoreOp>
src/profile_store.rs:156-162             /// Another writer stores `profile` unconditionally, at the stored generation + 1 ...
                                         pub fn concurrent_put(&self, profile: &Profile, actor: &Actor) -> Result<Profile, PaneError>
src/pane_store.rs:93-99                  pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError>  // each at 1; bypasses faults and call log
src/pane_store.rs:220-238                /// The membership rule of a `cas_put` ...: a pane stored with profile P cannot be written with
                                         /// another profile Q. Leaving (`None`), joining from `None` and keeping P are allowed.
                                         fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
                                             let current = stored.and_then(|pane| pane.profile.as_ref());
                                             match (current, submitted.profile.as_ref()) {
                                                 (Some(current), Some(next)) if current.slug() != next.slug() => {
                                                     Err(PaneError::PaneInOtherProfile { what: format!("{} is in profile {:?}, not {:?}",
                                                         submitted.name, current.as_str(), next.as_str()) }) }
                                                 _ => Ok(()), } }                       // private today
src/fault.rs:75,86-89                    pub fn fail_next(&self, op: Op, error: PaneError);
                                         /// Every call made through the port, oldest first, the failed ones included.
                                         pub fn calls(&self) -> Vec<Op>
src/fixture.rs                           pub fn sample_pane(name: &str) -> Result<Pane, PaneError>   // profile: None
                                         pub fn sample_spec(pane: &str) -> ProfileSpec
                                         pub fn sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError>
src/conformance/mod.rs:47,71,77,94       pub(crate) fn run_cases<S, K, C: Copy>(cases: &[(&'static str, C)], fresh: impl FnMut() -> (S, K),
                                             check: impl FnMut(C, &S) -> Result<(), String>) -> Conformance;
                                         pub(crate) fn succeeds / expect_code / expect_eq
src/conformance/pane_store.rs:418-425    fn pane_name(text: &str) -> Result<PaneName, String>            // private
                                         pub(super) fn profile_name(text: &str) -> Result<ProfileName, String>   // "The profile store suite reuses it."
src/conformance/profile_store.rs:59,62   type Shown = (Option<Profile>, Vec<Profile>, Vec<ProfileLogEntry>);  type Step = (u64, Actor, &'static str);
src/conformance/profile_store.rs:70-87   const CREATED, UPDATED, ...; const ACTOR = "conformance"; const ALPHA = "Demo Alpha"; BETA; GAMMA;
                                         const C1 = "demo-c1r1"; C2 = "demo-c2r1"; C3 = "demo-c3r1";      // all private
src/conformance/profile_store.rs:421-521 fn actor(); fn sample(name, panes); fn history(store, name) -> Vec<Step>;
                                         fn shown(store, name) -> Shown; fn unchanged(store, name, before, call)   // all private
tests/profile_store_conformance_test.rs:58-196   the mutant pattern: enum Break { Nothing, ... }, struct Mutant wrapping the fake,
                                         fn assert_suite_fails_on(broken, case), the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone
```
Lint conventions:
```
Cargo.toml (workspace lints)  unwrap_used, expect_used, panic, unreachable, cognitive_complexity, too_many_lines = "deny"; dead_code = "deny"
clippy.toml                   cognitive-complexity-threshold = 15; too-many-lines-threshold = 100
scripts/lint.sh:42-52         warn at 600 lines per .rs file, fail at 900
tests/*_test.rs line 1        #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682
```
Today's sizes of the files this run touches: `src/profile_scope.rs` 3, `src/conformance/profile_scope.rs` 2,
`src/conformance/profile_store.rs` 521, `src/conformance/pane_store.rs` 534, `src/pane_store.rs` 238, `CHANGELOG.md`.

## Where the appendix is stale (the merged code wins)

1. **The appendix lives nowhere on `main`.** It is carried inline here, unchanged in substance.
2. **Part 1's suite helpers are private** and its suite is split (`profile_store.rs`, `profile_store/log.rs`,
   `profile_store/watch.rs`). The appendix assumed the scope suite could call them. This run makes the helpers it needs
   `pub(super)` (see "Reuse refactors").
3. **`FakePaneStore` already has the membership rule** (`check_membership`, slug comparison). The scope's
   `pane-in-other-profile` check reuses it instead of restating it.
4. **The estimate.** The appendix said ~1,020 lines. Part 1 came in about 18% over its estimate (tests about 30% over), so
   this brief re-estimates bottom-up at ~1,500 (see "Size").
5. The stubs' doc lines say "slice c (#682) fills it"; the filled modules say #688.

## Public API (exact; T writes tests against these, F implements them)

No flat re-exports; `lib.rs` and `conformance/mod.rs` are not edited.

```rust
// crates/holler-pane-testkit/src/profile_scope.rs
/// A `ProfileScope` over any `ProfileStore` and `PaneStore`, keeping ADR-0021's I8 write order.
/// It never writes a pane record: recording the pane (ADR-0021 section 8, step 4) is the verb's, inside its act.
/// It has no fault switch of its own: a test injects faults into the stores it wraps.
pub struct FakeProfileScope {
    profiles: Arc<dyn ProfileStore>,
    panes: Arc<dyn PaneStore>,
    /// Who every profile write of this scope is logged as.
    actor: Actor,
}
impl FakeProfileScope {
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self;
}
impl ProfileScope for FakeProfileScope { /* see "Fake behaviour" */ }

// crates/holler-pane-testkit/src/conformance/profile_scope.rs
/// The ids of the cases `run_profile_scope_conformance` runs, in order (the table below).
pub fn profile_scope_cases() -> Vec<&'static str>;
/// Run every case. Per case the suite seeds a fresh `FakeProfileStore` and `FakePaneStore` with the fixture below
/// (seeding bypasses the call logs), then `build(profiles, panes)` makes the scope under test over them. The suite
/// drives the scope and inspects the two fakes.
pub fn run_profile_scope_conformance<S, F>(build: F) -> Conformance
where S: ProfileScope, F: FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S;
```

How each implementation runs the suite (doc comment on `run_profile_scope_conformance`, in a `text` fence, as on the other
suites):
```text
// the fake:
assert_eq!(run_profile_scope_conformance(|profiles, panes| FakeProfileScope::new(profiles, panes, actor)), Ok(()));
// the real scope (#663, in holler-cli's tests): the CLI's ProfileScope built over the two fakes.
```

Recommended internal shape (F may refine; not pinned by a test): `run_cases` with subject
`Result<Bench<S>, String>` and guard `()`, where `Bench { scope: S, profiles: Arc<FakeProfileStore>, panes:
Arc<FakePaneStore> }`; a seeding failure fails every case with its reason (no `unwrap` in `src/`). A case is
`fn(&dyn ProfileScope, &FakeProfileStore, &FakePaneStore) -> Result<(), String>` (or a small borrowed view struct).

## Fake behaviour (`FakeProfileScope`)

**`resolve(P, pane)`:**
1. `profiles.get(P)?`; `None` is `ProfileNotFound { what: P }`. Checked first.
2. `None` pane: `panes.list()?`, kept where `pane.profile` has P's slug, sorted by name. Membership is `Pane.profile` only, so
   a spec of P that names a pane of another profile, or a pane with no record, adds nothing.
3. `Some(n)`: `panes.get(n)?`. A record whose profile has P's slug is the one pane. Anything else, including no record,
   is `PaneNotInProfile { what }` naming the pane and P.
Returns `ResolvedScope { profile: the stored P, panes }`.

**`edit_spec(Some(P), pane, edit, act)`, the I8 order:**
1. `stored = profiles.get(P)?`; `None` is `ProfileNotFound { what: P }`, before anything else (no pane call, no act).
2. A `Set(spec)` whose `spec.pane != pane.as_str()` is `Usage { message }` (the fake's own guard against a verb that files
   a spec under the wrong pane), before any write.
3. `panes.get(pane)?`. A record whose profile is `Some(Q)` with `Q.slug() != P.slug()` is `PaneInOtherProfile`, through
   `crate::pane_store::check_membership(Some(&record), &Pane { profile: Some(P.clone()), ..record.clone() })`. No record, or
   `profile: None`, passes.
4. `edited` = `stored` with the edit: `Set(spec)` replaces the entry whose `spec.pane == pane.as_str()` **in place** (same
   index), or appends one; `Remove` drops it (the others keep their order). A `Remove` of an absent entry still writes
   (the I8 order stays uniform; the specs are unchanged and the generation moves by one).
5. `written = profiles.cas_put(&edited, stored.generation, &actor)?` — **the profile is written first**. A conflict here is
   `generation-conflict` and the act is never called. The scope does not retry.
6. `act()`. If it succeeds, return `Ok(Some(written))`.
7. If it fails with `e`: write `Profile { panes: stored.panes, ..written }` back with `profiles.cas_put(.., written.generation,
   &actor)`. If that succeeds, return `Err(e)`: the specs equal the ones before, the generation has moved by two, and the
   log shows the edit and its reversal. A `Conflict` there is `ProfileConflict { what }`, naming P and saying its specs
   were not restored after a failed act (the other writer's version stays). Any other error of the restoring write is
   returned as it is (the act's error is then lost; P keeps the edit).

**`edit_spec(None, ..)`** calls only `act()`, returns `Ok(None)` or the act's error, and makes no profile store call and no
pane store call.

Profile calls, in order: a successful edit is `[Get, CasPut]`, a failed act `[Get, CasPut, CasPut]`; the pane store sees
one `Get`. Each step is a small private function so that no function passes complexity 15 or 100 lines. Every path returns
`Result`: no `unwrap`, `expect`, `panic` or `assert!` in `src/`.

## Conformance cases: `run_profile_scope_conformance` (14)

**Seed fixture (per case).** `FakeProfileStore::seeded([Demo Alpha with specs [c1, c2, c3], Demo Beta with specs [c3]],
conformance)` — Alpha's c3 entry is a detached spec. `FakePaneStore::seeded` with panes (from `sample_pane`) `demo-c1r1` and
`demo-c2r1` with `profile = Demo Alpha`, `demo-c3r1` with `profile = Demo Beta`, `demo-c4r1` with `profile = None`.
`Demo Gamma` does not exist. Alpha is at `g = 1`. Below, `c1..c4` are those panes, `s'` is `sample_spec(c1)` with one
field changed, `ok` is an act that succeeds and counts its runs, and "unchanged" means `get(Alpha)` and `log(Alpha)` equal
what they were before (part 1's `shown`/`unchanged`). The log is read with part 1's `history` as `(generation, actor,
kind)`.

| # | Case id | What it asserts |
|---|---|---|
| 1 | `resolve-every-pane-of-the-profile` | `resolve(Alpha, None)` is the stored Alpha with panes `[c1, c2]`, each equal to the pane store's record: not c3 (detached spec, Beta's pane), not c4. |
| 2 | `resolve-named-member` | `resolve(Alpha, Some(c1))` is the stored Alpha with panes `[c1]`. |
| 3 | `resolve-non-member-is-pane-not-in-profile` | `resolve(Alpha, Some(c3))` and `resolve(Alpha, Some(c4))` are each `pane-not-in-profile`. |
| 4 | `resolve-missing-profile-is-profile-not-found` | `resolve(Gamma, None)` and `resolve(Gamma, Some(c1))` are each `profile-not-found`. |
| 5 | `edit-set-replaces-the-entry-and-bumps-once` | `edit_spec(Some(Alpha), c1, Set(s'), ok)` returns `Some(r)` with `r == get(Alpha)` and generation g + 1. c1's entry is `s'` at the same index, the other entries unchanged. The act ran once, the log gained exactly one `updated` entry at g + 1, and `panes.list()` is unchanged. |
| 6 | `edit-set-adds-a-missing-entry` | `Set(sample_spec(c4))` for c4 (no profile) appends that spec after the three entries, at g + 1. |
| 7 | `edit-remove-drops-the-entry` | `Remove` for c2 drops its entry at g + 1, and c1 and c3 keep their order. |
| 8 | `the-act-sees-the-edit` | Inside the act, `profiles.get(Alpha)` already holds the edited specs at g + 1: the profile is written first (I8). |
| 9 | `failed-act-restores-the-specs` | The act fails with `Unavailable { what: "act" }`. `edit_spec` returns exactly that error. `get(Alpha).panes` equals the panes before; **the generation is g + 2**; the log gained exactly two `updated` entries, at g + 1 and g + 2, with one actor; `panes.list()` is unchanged; the act ran once. |
| 10 | `first-write-conflict-is-generation-conflict` | After `profiles.faults().fail_next(CasPut, Conflict)`, `edit_spec(Some(Alpha), c1, Set(s'), ok)` is `generation-conflict`. The act ran 0 times and Alpha is unchanged. |
| 11 | `restore-conflict-is-profile-conflict` | The act calls `profiles.concurrent_put(alpha_other, other)` (Alpha with other specs) and then fails. `edit_spec` is `profile-conflict` and its message contains `Demo Alpha`; `get(Alpha)` is the other writer's version (g + 2, its specs); the act ran once. |
| 12 | `no-profile-runs-only-the-act` | `edit_spec(None, c1, Set(s'), ok)` is `Ok(None)`, the act ran once and `profiles.faults().calls()` is empty. With a failing act it returns the act's error and `calls()` is still empty. |
| 13 | `missing-profile-is-profile-not-found-before-the-act` | `edit_spec(Some(Gamma), c4, ..)` and `edit_spec(Some(Gamma), c1, ..)` (c1 belongs to Alpha) are each `profile-not-found`: the profile is checked before the pane. The act ran 0 times and `calls()` holds no `CasPut`. |
| 14 | `pane-in-other-profile-before-any-write` | `edit_spec(Some(Alpha), c3, Set(sample_spec(c3)), ok)` is `pane-in-other-profile`. The act ran 0 times, `calls()` holds no `CasPut`, and Alpha is unchanged. |

**`ASSUMPTION (#663)` comments** (the repo's form, `// ASSUMPTION (#640): ...`), each in the suite at the case or helper
concerned, and summarised in the suite's module docs; the fake carries the same comment at its code:
- the real scope can be built over any `ProfileStore` and `PaneStore` (the `build` closure);
- the scope writes no pane record: recording the pane is inside the verb's act, whose signature returns `()` (cases 5, 9);
- `Set` replaces an entry in place, keeping its index (case 5);
- the first write is not retried on a conflict (case 10), and a restore is one `cas_put` at g + 1 (cases 9, 11);
- `profile-not-found` comes before `pane-in-other-profile` (case 13);
- `edit_spec(None)` makes no profile store call (case 12).

**`ASSUMPTION (#661/#663)`:** the scope checks `pane-in-other-profile` itself before the profile write (case 14), as well
as the pane registry doing so inside its compare-and-swap (ADR-0021 "Decisions taken" item 2), so that nothing is written
to P and nothing live moves for a pane that cannot join P; membership compares slugs, as `FakePaneStore` does.

The suite pins no `what` text beyond case 11's profile name, no actor value (only that the edit and its reversal share
one), and no order between the scope's two pane-store calls.

## Acceptance criteria

T authors these tests (RED first). Every test file starts with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #688`.

- [ ] **AC1 The fake passes its suite.** `tests/profile_scope_conformance_test.rs`:
  `the_fake_passes_the_profile_scope_conformance_suite`:
  `run_profile_scope_conformance(|p, q| FakeProfileScope::new(p, q, actor))` is `Ok(())`.
  `the_suite_runs_the_documented_cases`: `profile_scope_cases()` equals the 14 ids above, in order.
- [ ] **AC2 Mutation check: the two scope mutants fail on the named cases.** Same file, following
  `tests/profile_store_conformance_test.rs:58-196`: `enum Break { Nothing, WritesAfterAct, NoRestore }`, `struct Mutant {
  inner: FakeProfileScope, broken: Break }` (`resolve` delegates), and `assert_suite_fails_on(broken, case)`. The mutants
  delegate to the fake and change only the order, so the test file holds no copy of the edit logic:
  - `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone` (`Break::Nothing`) is `Ok(())`.
  - `a_scope_that_writes_the_profile_after_the_act_fails` (`WritesAfterAct`: with a profile, run `act()?` first, then
    `inner.edit_spec(profile, pane, edit, &mut || Ok(()))`): fails on `the-act-sees-the-edit` **and** on
    `failed-act-restores-the-specs` (the generation moves by 0, not 2).
  - `a_scope_that_does_not_restore_on_a_failed_act_fails` (`NoRestore`: `inner.edit_spec(.., &mut || Ok(()))?`, then
    `act()?`): fails on `failed-act-restores-the-specs`.
- [ ] **AC3 Faults reach the scope through its stores.** `tests/fake_profile_scope_test.rs`:
  - `a_wedged_profile_store_times_out_resolve_and_edit_spec`: `Fault::Wedged` on the profile store makes both answer
    `Timeout { op: "profile_store.get" }`; the act ran 0 times. After `set(None)` both work.
  - `a_wedged_pane_store_times_out_resolve_and_edit_spec`: `resolve(P, None)` is `Timeout { op: "pane_store.list" }`,
    `resolve(P, Some(c1))` and `edit_spec(Some(P), c1, ..)` are `Timeout { op: "pane_store.get" }`; the act ran 0 times
    and the profile calls hold no `CasPut`.
  - `a_corrupt_profile_store_fails_closed`: `Fault::Fail(StoreCorrupt)` makes `resolve` and `edit_spec(Some)` answer it,
    and the act ran 0 times.
  - `a_failed_restore_returns_its_own_error`: the act queues `fail_next(CasPut, StoreCorrupt)` and fails with
    `Unavailable`. `edit_spec` is `store-corrupt`; Alpha stays at g + 1 with the edit.
  - `without_a_profile_a_wedged_profile_store_is_not_called`: `edit_spec(None, ..)` with the profile store wedged is
    `Ok(None)` and the act ran once.
  - `the_calls_follow_the_i8_order`: a successful edit leaves profile calls `[Get, CasPut]` and pane calls `[Get]`; a
    failed act leaves `[Get, CasPut, CasPut]`.
- [ ] **AC4 The fake's own rules.** Same file:
  - `a_set_whose_spec_names_another_pane_is_usage`: `Set(sample_spec(c2))` for c1 is `usage`; act 0 times, no `CasPut`.
  - `removing_an_absent_entry_still_writes_the_profile`: `Remove` for c4 on Alpha gives g + 1 with the specs unchanged.
  - `membership_compares_slugs`: a pane whose profile is `DEMO-ALPHA` is in `resolve(Demo Alpha, None)` and its
    `edit_spec(Some(Demo Alpha), ..)` passes the membership check.
  - `resolve_of_a_pane_with_no_record_is_pane_not_in_profile`.
  - `the_refusals_name_the_pane_and_the_profile`: the messages of `pane-not-in-profile` and `pane-in-other-profile`
    contain the pane's name and the profile's.
  - `the_log_carries_the_scopes_actor`: the `updated` entries of an edit and its reversal carry the actor given to `new`.
  - `the_scope_is_send_and_sync` (a compile-time bound check).
- [ ] **AC5 Earlier slices still hold.** `tests/pane_store_conformance_test.rs`, `tests/fake_pane_store_test.rs`,
  `tests/profile_store_conformance_test.rs` and `tests/fake_profile_store_test.rs` pass with not one line edited, which
  proves the visibility changes are behaviour-neutral.
- [ ] **AC6 Dependencies.** `git diff --quiet origin/main -- crates/holler-pane-testkit/Cargo.toml Cargo.toml Cargo.lock`
  succeeds. `cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'` prints nothing.
- [ ] **AC7 One rule, one place.** Each prints nothing:
  - `grep -rn "PaneInOtherProfile" crates/holler-pane-testkit/src | grep -v pane_store.rs` (the scope reuses
    `check_membership`);
  - `grep -rnE "^(pub\(super\) )?fn (actor|sample|history|shown|unchanged|pane_name|profile_name)\b" crates/holler-pane-testkit/src/conformance/profile_scope*`
    (the scope suite reuses part 1's helpers).
- [ ] **AC8 Layout.** `git diff --name-only origin/main...HEAD` lists only the files under "Blast radius"; `src/lib.rs`
  and `src/conformance/mod.rs` are absent.
- [ ] **AC9 CHANGELOG.** One entry at the end of `## [Unreleased]` / `### Enhancements` (after the #681 entry): the test
  kit's fake profile scope, which edits a profile's spec and makes the live change as one transaction in ADR-0021's
  order (profile written first; a failed live change restores the specs, so the generation moves by two; another
  writer's change in between is `profile-conflict`), and a 14-case conformance suite the real profile scope runs against
  itself, with the two broken scopes it rejects. Test code only: nothing a user runs changes. Link
  [#688](https://github.com/Performant-Labs/holler/issues/688). `bash scripts/changelog-check.sh` passes.
- [ ] **AC10 Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p holler-pane-testkit`, `cargo test --workspace`, `cargo machete`, `bash scripts/lint.sh` and
  `bash scripts/test-hooks.sh` pass. `rustfmt --check --edition 2021` passes on every new or changed `.rs` file. No `.rs`
  file in the diff reaches 800 lines (lint fails at 900). No function exceeds 100 lines or complexity 15.

## Files

Filled stubs (under `crates/holler-pane-testkit/`):
- `src/profile_scope.rs` (~230): `FakeProfileScope`, its `ProfileScope` impl, private step functions (`edited`, `restore`,
  the membership check call).
- `src/conformance/profile_scope.rs` (~650): module docs with the assumptions, the 14-row `CASES` table,
  `profile_scope_cases`, `run_profile_scope_conformance`, the bench and seed, the 14 cases, small helpers (`spec_of`,
  `counting` act). **If it nears 800**, move cases 9 to 14 into a child module `src/conformance/profile_scope/act.rs`
  (declared `mod act;` in the same file, case functions `pub(super)`), as part 1 did with `profile_store/log.rs`.

New: `tests/profile_scope_conformance_test.rs` (~220), `tests/fake_profile_scope_test.rs` (~380).

Reuse refactors (visibility only, behaviour-neutral):
- `src/conformance/profile_store.rs` (~+10): `actor`, `sample`, `history`, `shown`, `unchanged`, the types `Shown` and
  `Step`, and the constants `UPDATED`, `CREATED`, `ALPHA`, `BETA`, `GAMMA`, `C1`, `C2`, `C3` become `pub(super)`; each doc
  gains "The profile scope suite reuses it."
- `src/conformance/pane_store.rs` (±1): `pane_name` becomes `pub(super)`.
- `src/pane_store.rs` (±2): `check_membership` becomes `pub(crate)`; its doc names the scope as its second caller.

Plus `CHANGELOG.md` (+~10).

**Reuse map.** Closest analogous feature: `FakeProfileStore` + `run_profile_store_conformance` (part 1), and the mutant
harness of `tests/profile_store_conformance_test.rs`.
- **Extend** the two stubs. Implement the frozen `ProfileScope`. Return only closed `PaneError` variants (`Conflict` passed
  through from the store, `ProfileConflict`, `ProfileNotFound`, `PaneNotInProfile`, `PaneInOtherProfile`, `Usage`, and the
  stores' `Timeout`/`StoreCorrupt`/`Unavailable` passed through), never a new code.
- **Reuse, do not copy:** `FakeProfileStore` (`seeded`, `faults`, `concurrent_put`) and `FakePaneStore` (`seeded`,
  `faults`) as the stores under the scope; `check_membership` for `pane-in-other-profile`; `FaultSwitch`/`Fault`/`calls()`
  for every fault test; `run_cases`, `succeeds`, `expect_code`, `expect_eq` (`conformance/mod.rs`); part 1's `actor`,
  `sample`, `history`, `shown`, `unchanged` and name constants; `profile_name`, `pane_name`; `sample_pane`, `sample_spec`,
  `sample_profile`; `ProfileName::slug` for every membership comparison.
- **New, because no equivalent exists:** the scope itself (the I8 sequence and its compensation), the suite's two-store
  seed, and the edit application (`Set` in place / append, `Remove`). `ports_test.rs`'s `MemScope` in `holler-pane` is a
  test-local stand-in (it moves a `Set` entry to the end and does not restore) and is not reused.
- **Accepted near-duplicate:** the mutant harness (`Break`, `Mutant`, `assert_suite_fails_on`) repeats per test file, as
  in every suite of this crate: integration test files cannot share code without a `tests/common` module, and each
  suite's mutants differ.

## Size

| Part | Files | Lines (est.) |
|---|---|---|
| `FakeProfileScope` | `src/profile_scope.rs` | ~230 |
| `ProfileScope` suite, 14 cases | `src/conformance/profile_scope.rs` | ~650 |
| visibility refactors | `conformance/profile_store.rs`, `conformance/pane_store.rs`, `pane_store.rs` | ~+15 |
| tests: the suite + 2 mutants; the fake's own (13 tests) | 2 test files | ~600 |
| CHANGELOG | `CHANGELOG.md` | ~10 |
| **Total** | | **~1,500** |

**Fits one run.** Part 1 merged at ~2,100 lines in one run. The diff touches 8 files, but three are one-word to
ten-line visibility changes inside the same crate and one is the CHANGELOG: substantive code is 4 files in one component
family (the test kit's profile scope), within the F scope cap's intent. No split is proposed.

## Forward-compat

| Consumer | Needs | Satisfied |
|---|---|---|
| #663 (the real `ProfileScope`) | runs the suite over the two fakes via `build` | yes, subject to the `ASSUMPTION (#663)` points, which #663 confirms or amends here first |
| spec-editing verb stories (launch, relaunch, close) | an in-memory scope over fakes they already seed, with faults through the stores | yes (`FakeProfileScope::new` over `Arc<dyn …>`) |
| scoping verb stories | `resolve` over seeded fakes | yes |

## Decisions already made (operator, epic, ADR)

- Profile written first; a failed act restores the specs by a second write, the generation moves by two, and the log shows
  the edit and its reversal; a conflict on the restoring write is `profile-conflict` (ADR-0021 section 8, "Decisions
  taken" item 1). **No ADR change:** the ADR already says this, and the trait's "nothing is recorded" reads as "P's specs
  restored and no pane record written" in its light.
- `pane-in-other-profile` is enforced in the pane registry's CAS (item 2); the scope's earlier check is an addition, not a
  replacement.
- The fake lives in `holler-pane-testkit`, the real scope in `holler-cli` (#663) (ADR-0021:190-192). The testkit never
  names `holler-cli` or `holler-hub`.
- No slice edits `lib.rs` or `conformance/mod.rs`.

## Decisions made in this brief (operator may review)

1. **No fault switch on `FakeProfileScope`.** It composes two ports; faults are injected into the stores it wraps, which
   also exercises how a real scope fails (AC3). Every other fake has its own switch because it *is* a port's boundary.
2. **`profile-not-found` before `pane-in-other-profile`** (case 13 pins it with c1): "P must exist" is the first thing a
   `--profile` verb learns.
3. **The fake's own guards, not pinned by the suite:** a `Set` whose spec names another pane is `usage`; a `Remove` of an
   absent entry still writes; a pane with no record is `pane-not-in-profile` in `resolve`; a restoring write that fails
   with anything but `Conflict` returns its own error.
4. **The ADR's I8 row's success half** ("the pane record names P") is the verb's, inside its act; the suite pins that the
   scope itself writes no pane record (cases 5 and 9).
5. **Reuse goes through part 1's and slice a's files by visibility only** (three files), as part 1 did.

## Out of scope

The real `ProfileScope` (#663); any verb; any change to `holler-pane`, `holler-hub`, `holler-cli` or any manifest;
`rename` (#665); protocol v2 and golden files; ADR-0021.

## Test plan

RED (T): write the two test files against the API above. They fail to build because `FakeProfileScope`,
`run_profile_scope_conformance` and `profile_scope_cases` do not exist; confirm with `cargo test -p holler-pane-testkit`
that the errors are those missing items and not a typo, and that every other test target still builds and passes.
GREEN (F): first the three visibility refactors (earlier slices' tests green after them), then `profile_scope.rs`, then
the suite, then the CHANGELOG and AC10's guards. A (anti-duplication) checks AC7 and that the suite calls part 1's helpers
rather than local copies. S audits against AC1-AC10 and the issue.

## Risks

- **Suite size.** If `conformance/profile_scope.rs` nears 800 lines, use the `profile_scope/act.rs` child module above.
- **`CHANGELOG.md` conflicts** with any sibling #638 PR: textual, resolved by rebase.
- **#663 may want different answers** on the `ASSUMPTION (#663)` points. The suite fixes them now, as the store suites did
  for #639 and #661; a change is an amendment to this suite, made first.
- **The act closure borrows a fake** (`&FakeProfileStore`) while the scope holds an `Arc` of it: fine, every fake method
  takes `&self`; no lock is held across `act()` by the fake scope.

## Blast radius

`crates/holler-pane-testkit/src/{profile_scope.rs, conformance/profile_scope.rs, conformance/profile_scope/act.rs (only if
needed), conformance/profile_store.rs, conformance/pane_store.rs, pane_store.rs}`,
`crates/holler-pane-testkit/tests/{profile_scope_conformance_test.rs, fake_profile_scope_test.rs}`, `CHANGELOG.md`, and
`docs/handoffs/688*` (pipeline artifacts). All inside #638's radius, `crates/holler-pane-testkit/**`. Not changed: any
`Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/conformance/mod.rs`, any other crate, ADR, protocol doc or golden file. The
repository is public, so no personal or infrastructure names appear in code, comments, tests or the changelog; fixtures use
the neutral `Demo *` and `demo-c*r1` names.

## Handoff locations

Phase handoffs go to `docs/handoffs/688/` (`decisions.md`, `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`,
`handoff-T-green.md`, `handoff-A-dup.md`, `handoff-S.md`).
