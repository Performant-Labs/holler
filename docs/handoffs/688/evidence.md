# Evidence: #688 the pane test kit, slice c part 2 (`FakeProfileScope` and the `ProfileScope` suite)

Source facts that the diff and T's tests rely on but that live in unchanged code. Line numbers are as of the F diff
(the ADR's after F's three-line insertion at section 8, step 1).

## F (Phase 6, implementation)

- **Fact:** the frozen port says the profile is written first, a conflict after `act` is `profile-conflict`, and
  `edit_spec` with no profile runs only `act` and returns `None`; an implementation is `Send + Sync`.
  **Source:** `crates/holler-pane/src/profile.rs:380-404`
  **Verbatim excerpt:**
  > pub trait ProfileScope: Send + Sync {
  > ...
  >     /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
  >     /// transaction (I8): the profile is written with a compare-and-swap on its
  >     /// generation first, then `act` runs, then the result is recorded; if `act`
  >     /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
  >     /// Returns the edited profile. With `profile: None` it runs only `act` and
  >     /// touches no profile (and returns `None`).

- **Fact:** ADR-0021 allows a detached spec (a spec naming a pane of another profile), which is why case 15 pins that
  removing one is not refused.
  **Source:** `docs/adr/ADR-0021.md:282-283`
  **Verbatim excerpt:**
  > to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
  > names a pane of another profile (a detached spec) is not refused.

- **Fact:** ADR-0021 section 9 gives `pane close` (the verb that sends `SpecEdit::Remove`) no `pane-in-other-profile`.
  **Source:** `docs/adr/ADR-0021.md:341` (line 338 on `origin/main`)
  **Verbatim excerpt:**
  > | `pane close` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict` |

- **Fact:** the scope's `pane-in-other-profile` comes only from the fake pane store's rule, which compares slugs and
  whose message names the pane and both profiles (T's `the_refusals_name_the_pane_and_the_profile` relies on it). Only
  its visibility and doc changed in this diff; the body is unchanged.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:225-240`
  **Verbatim excerpt:**
  > pub(crate) fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
  >     let current = stored.and_then(|pane| pane.profile.as_ref());
  >     match (current, submitted.profile.as_ref()) {
  >         (Some(current), Some(next)) if current.slug() != next.slug() => {
  >             Err(PaneError::PaneInOtherProfile {
  >                 what: format!(
  >                     "{} is in profile {:?}, not {:?}",

- **Fact:** the fake profile store's `cas_put` passes the fault switch first, so a queued one-shot error fails that one
  call (case 10's `fail_next(CasPut, Conflict)`, T's `a_failed_restore_returns_its_own_error`).
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:260-268`
  **Verbatim excerpt:**
  > fn cas_put(
  >     &self,
  >     profile: &Profile,
  >     expected_generation: u64,
  >     actor: &Actor,
  > ) -> Result<Profile, PaneError> {
  >     self.faults.enter(ProfileStoreOp::CasPut)?;
  >     self.put(profile, Writer::Port(expected_generation), actor)

- **Fact:** a one-shot error fails only the next call of its method, and a standing `Wedged` fault answers
  `Timeout { op }` with the method's name (AC3's `profile_store.get`, `pane_store.list`, `pane_store.get`).
  **Source:** `crates/holler-pane-testkit/src/fault.rs:119-133`
  **Verbatim excerpt:**
  > fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
  >     match &self.standing {
  >         Some(Fault::Wedged) => {
  >             return Err(PaneError::Timeout {
  >                 op: op.as_str().to_owned(),
  >             })
  >         }
  >         Some(Fault::Fail(error)) => return Err(error.clone()),
  >         None => {}
  >     }
  >     match self.queued.iter().position(|(queued, _)| *queued == op) {
  >         Some(at) => Err(self.queued.remove(at).1),
  >         None => Ok(()),

- **Fact:** seeding the profile store goes through the port writer at expected generation 0, so each seed is stored at
  1 with one `Created` entry (g = 1 in the suite), without passing the fault switch.
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:127-136`
  **Verbatim excerpt:**
  > pub fn seeded(
  >     profiles: impl IntoIterator<Item = Profile>,
  >     actor: &Actor,
  > ) -> Result<Self, PaneError> {
  >     let store = Self::new();
  >     for profile in profiles {
  >         store.put(&profile, Writer::Port(0), actor)?;
  >     }
  >     Ok(store)

- **Fact:** the pane store's seed bypasses the faults and the call log, so a case's pane call log starts empty.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:90-99`
  **Verbatim excerpt:**
  > /// A store holding `panes`, each created at expected generation 0 and so stored
  > /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
  > /// name are `generation-conflict`, as a second create would be.

- **Fact:** another writer's `concurrent_put` stores at the stored generation + 1 regardless of what the caller read,
  which is what makes the scope's restoring write conflict in case 11 and in the hook tests (Alpha at g + 2).
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:156-162`
  **Verbatim excerpt:**
  > /// Another writer stores `profile` unconditionally, at the stored generation + 1
  > /// (or at 1 for a new profile), without the name rule, logs its entry with `actor`
  > /// and publishes its event. It bypasses the faults and the call log. Returns the
  > /// stored record.

- **Fact:** every applied write of a stored profile logs an `Updated` entry (the edit and the restore of cases 5 and 9
  each add one).
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:191-196`
  **Verbatim excerpt:**
  > let change = match stored {
  >     None => ProfileChange::Created,
  >     Some(stored) => ProfileChange::Updated {
  >         summary: summary(stored, &next),
  >     },
  > };

- **Fact:** the scope's hook mutex is read through the crate's shared helper, which takes over a poisoned lock (no
  `unwrap` in `src/`).
  **Source:** `crates/holler-pane-testkit/src/feed.rs:243-245`
  **Verbatim excerpt:**
  > pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
  >     mutex.lock().unwrap_or_else(PoisonError::into_inner)
  > }

- **Fact:** the suite runner gives each case a fresh subject (here the seeded fixture and its scope) and drops it before
  its guard, so no case sees another case's writes or call logs.
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:47-67`
  **Verbatim excerpt:**
  > pub(crate) fn run_cases<S, K, C: Copy>(
  >     cases: &[(&'static str, C)],
  >     mut fresh: impl FnMut() -> (S, K),
  >     mut check: impl FnMut(C, &S) -> Result<(), String>,
  > ) -> Conformance {
  >     let failures: Vec<CaseFailure> = cases
  >         .iter()
  >         .filter_map(|&(case, run)| {
  >             let (subject, guard) = fresh();
  >             let outcome = check(run, &subject);
  >             drop(subject);
  >             drop(guard);

- **Fact:** `expect_code` compares the code string, so the suite checks `pane-in-other-profile` without naming the
  variant (AC7's first grep).
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:77-90`
  **Verbatim excerpt:**
  > pub(crate) fn expect_code<T>(
  >     call: &str,
  >     result: Result<T, PaneError>,
  >     code: &str,
  > ) -> Result<(), String> {
  >     match result {
  >         Err(e) if e.code() == code => Ok(()),

- **Fact:** a `profile-conflict` displays its `what`, so case 11's check that the error's text names `Demo Alpha` reads
  the `what` of any implementation.
  **Source:** `crates/holler-pane/src/error.rs:655`
  **Verbatim excerpt:**
  > PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
