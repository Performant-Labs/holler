# Evidence — #682 part 1 (`FakeProfileStore` and the `ProfileStore` suite)

Source facts the diff relies on that live in code the diff does not change (or in unchanged lines of a changed
file). Line numbers are as of the F phase on `issue-682-implementation`.

## F (Phase 6, implement)

- **Fact:** Every write's compare-and-swap is `next_generation`: an expected generation other than the current one is
  `generation-conflict`, and a match moves the record to `current + 1` (overflow is `store-corrupt`).
  **Source:** `crates/holler-pane/src/generation.rs:25-34`
  **Verbatim excerpt:**
  > ```rust
  > pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
  >     if current != expected {
  >         return Err(PaneError::Conflict);
  >     }
  >     current
  >         .checked_add(1)
  >         .ok_or_else(|| PaneError::StoreCorrupt {
  >             what: "a generation counter overflowed".to_owned(),
  >         })
  > }
  > ```

- **Fact:** `Log::append` takes the next cursor before it changes anything, so a cursor overflow fails with
  `store-corrupt` and leaves the feed as it was. The fake appends the profile's log entry only after `append` returns
  `Ok`, so a write whose event cannot be published logs nothing.
  **Source:** `crates/holler-pane-testkit/src/feed.rs:80-97`
  **Verbatim excerpt:**
  > ```rust
  >     /// Publish the change that `make` builds from the next cursor: add it to the
  >     /// history, and file or remove its record. A cursor overflow is `store-corrupt`
  >     /// and changes nothing.
  >     pub(crate) fn append(&mut self, make: impl FnOnce(Cursor) -> E) -> Result<(), PaneError> {
  >         let overflowed = || PaneError::StoreCorrupt {
  >             what: "the change cursor overflowed".to_owned(),
  >         };
  >         let cursor = Cursor(self.head.0.checked_add(1).ok_or_else(overflowed)?);
  >         let change = make(cursor);
  >         if change.record().is_some() {
  >             self.live.insert(change.key(), change.clone());
  >         } else {
  >             self.live.remove(&change.key());
  >         }
  >         self.history.push(change);
  >         self.head = cursor;
  >         Ok(())
  >     }
  > ```

- **Fact:** `Feed::write` runs the whole write under the feed's lock and wakes watchers only when it succeeds, so a
  refused write (the name rule, a stale or ahead generation, a missing profile) publishes no event.
  **Source:** `crates/holler-pane-testkit/src/feed.rs:165-176`
  **Verbatim excerpt:**
  > ```rust
  >     /// `change` applied to the log, under the lock. When it succeeds, the watchers
  >     /// are woken.
  >     pub(crate) fn write<T>(
  >         &self,
  >         change: impl FnOnce(&mut Log<E>) -> Result<T, PaneError>,
  >     ) -> Result<T, PaneError> {
  >         let result = change(&mut lock(&self.log));
  >         if result.is_ok() {
  >             self.changed.notify_all();
  >         }
  >         result
  >     }
  > ```

- **Fact:** The feed files live records in a `BTreeMap` by the event's key and lists them in key order. With
  `Change::key` = the name's slug, the fake's `get` finds a profile by slug (any display name with that slug) and its
  `list` is in slug order.
  **Source:** `crates/holler-pane-testkit/src/feed.rs:64-78`
  **Verbatim excerpt:**
  > ```rust
  >     /// The last change of each live record, by key. It is always a put, because a
  >     /// delete removes the record's entry.
  >     live: BTreeMap<E::Key, E>,
  > }
  >
  > impl<E: Change> Log<E> {
  >     /// The live record filed under `key`, if any.
  >     pub(crate) fn get(&self, key: &E::Key) -> Option<&E::Record> {
  >         self.live.get(key).and_then(|change| change.record())
  >     }
  >
  >     /// Every live record, in key order.
  >     pub(crate) fn records(&self) -> impl Iterator<Item = &E::Record> {
  >         self.live.values().filter_map(|change| change.record())
  >     }
  > ```

- **Fact:** `FaultSwitch::enter` records the call before it answers any fault, so a failed call still appears in
  `calls()` (`calls_are_recorded_in_order` expects the failed `CasPut`). The fake calls it first in every port method,
  `rename` included, and `seeded`/`concurrent_*` never call it.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:93-103`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  >             state.delay
  >         };
  >         if let Some(delay) = delay {
  >             thread::sleep(delay);
  >         }
  >         self.lock().take_fault(op)
  >     }
  > ```

- **Fact:** Two display names with one slug are the same profile as far as uniqueness goes, which is what the name
  rule (`profile-exists`) and filing by slug rest on.
  **Source:** `crates/holler-pane/src/profile.rs:68-74`
  **Verbatim excerpt:**
  > ```rust
  >     /// The unique id derived from the name: ASCII letters and digits in lower case,
  >     /// each run of anything else becoming one `-`, with no leading or trailing `-`.
  >     /// Two display names with the same slug are the same profile as far as
  >     /// uniqueness goes.
  >     pub fn slug(&self) -> String {
  >         slugify(&self.0)
  >     }
  > ```

- **Fact:** A profile name with no ASCII letter or digit is refused as `usage`, so `sample_profile("!!", &[])` is
  `usage` without a check of its own.
  **Source:** `crates/holler-pane/src/profile.rs:50-58`
  **Verbatim excerpt:**
  > ```rust
  >         } else if slugify(name).is_empty() {
  >             Some("a profile name needs at least one ASCII letter or digit")
  >         } else {
  >             None
  >         };
  >         match refusal {
  >             Some(rule) => Err(PaneError::Usage {
  >                 message: format!("invalid profile name {}: {rule}", excerpt(text)),
  >             }),
  > ```

- **Fact:** `EnvVarName::parse` is the one env guard: a `=` anywhere is `profile-secret-refused`, and an empty name or
  one with whitespace or a control character is `env-name-invalid`. The fake adds no env scan (AC7), and case 18 pins
  these codes.
  **Source:** `crates/holler-pane/src/argv.rs:97-105`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         if text.contains('=') {
  >             return Err(PaneError::ProfileSecretRefused);
  >         }
  >         if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
  >             return Err(PaneError::EnvNameInvalid);
  >         }
  >         Ok(Self(text.to_owned()))
  >     }
  > ```

- **Fact:** The frozen log entry's `generation` is documented as the generation after the write. This is the field
  A's W-1 weighs against ADR-0021's "each applied write adds one"; the fake and the suite give a `Deleted` entry the
  deleted generation + 1 (brief decision 4).
  **Source:** `crates/holler-pane/src/profile.rs:298-299`
  **Verbatim excerpt:**
  > ```rust
  >     /// The generation the profile had after the write.
  >     pub generation: u64,
  > ```

- **Fact:** The suite compares refusals by code string. `Conflict` is `generation-conflict`, and `ProfileNotFound` and
  `ProfileExists` are `profile-not-found` and `profile-exists`.
  **Source:** `crates/holler-pane/src/error.rs:513-517`
  **Verbatim excerpt:**
  > ```rust
  >             PaneError::Conflict => Ok(PaneCode::GenerationConflict),
  >             PaneError::ProbeFailed { .. } => Ok(PaneCode::ProbeFailed),
  >             PaneError::ProfileConflict { .. } => Ok(PaneCode::ProfileConflict),
  >             PaneError::ProfileNotFound { .. } => Ok(PaneCode::ProfileNotFound),
  >             PaneError::ProfileExists { .. } => Ok(PaneCode::ProfileExists),
  > ```

- **Fact:** The shared runner gives every case a fresh subject from `fresh` and drops the subject before its guard, so
  each profile case starts from an empty store.
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:52-59`
  **Verbatim excerpt:**
  > ```rust
  >     let failures: Vec<CaseFailure> = cases
  >         .iter()
  >         .filter_map(|&(case, run)| {
  >             let (subject, guard) = fresh();
  >             let outcome = check(run, &subject);
  >             drop(subject);
  >             drop(guard);
  >             outcome.err().map(|detail| CaseFailure { case, detail })
  > ```
