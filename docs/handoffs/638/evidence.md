# Evidence — #638 slice a (unchanged source the diff relies on)

## F (Phase 5, implement)

- **Fact:** the one compare-and-swap rule: a write applies only when `expected` equals the stored generation (0 for a missing record), and moves the record to `current + 1`; an overflow is `store-corrupt`. Every generation in the fake goes through it (`pane_store.rs` `put` and `remove`), and `concurrent_put` calls it as `next_generation(current, current)`.
  **Source:** `crates/holler-pane/src/generation.rs:25-34`
  **Verbatim excerpt:**
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

- **Fact:** the port fixes the order of `delete`'s two checks: a missing record is `pane-not-found` whatever the generation. The fake's `remove` checks existence first, and suite case 12 pins it.
  **Source:** `crates/holler-pane/src/ports.rs:73-78`
  **Verbatim excerpt:**
  > /// Remove a pane's record if it is still at `expected_generation`. `close`
  > /// uses it, so a closed pane's record does not outlive the pane. A stale
  > /// generation is `generation-conflict`; a record that does not exist is
  > /// `pane-not-found`, whatever `expected_generation` is (a missing record is
  > /// checked first, so no store has to guess which of the two to answer).
  > fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;

- **Fact:** the `Watch` contract the feed and suite cases 14 to 18 implement: `Cursor(0)` yields the current state first, resuming from the last cursor neither repeats nor skips, idle is the item `Ok(None)` and the stream stays usable, and any error ends the stream.
  **Source:** `crates/holler-pane/src/ports.rs:38-52`
  **Verbatim excerpt:**
  > /// - `watch(since)` yields every change after `since`. `Cursor(0)` starts from the
  > ///   beginning: the store first yields a put for every record it holds now (the
  > ///   current state), then every later change.
  > /// - Passing the cursor of the last event seen back as `since` resumes without a
  > ///   gap or a repeat.
  > /// - `next()` blocks for at most I5's bound and yields one of three things:
  > ///   - `Ok(Some(change))`: the next change;
  > ///   - `Ok(None)` (the item, not the end of the iterator): **idle**, nothing happened
  > ///     within the bound. This is an ordinary outcome, as in the hub's `control/wait`,
  > ///     and the stream stays usable. A hub long-poll that sees it answers
  > ///     `{events: [], cursor}`;
  > ///   - `Err(..)`: a failure. `Err(PaneError::Timeout)` means the store did not
  > ///     answer within the bound (a wedged store), never "idle". Any error ends the
  > ///     stream (call `watch` again).
  > pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;

- **Fact:** a `Cursor` is ordered (`Ord`), which `feed.rs` relies on to sort the snapshot by cursor and to compare `since` with the head, and the suite relies on to check that cursors strictly increase.
  **Source:** `crates/holler-pane/src/ports.rs:28-34`
  **Verbatim excerpt:**
  > /// A position in a store's change sequence: a store-wide, strictly increasing
  > /// sequence number, one per change. `Cursor(0)` is "from the beginning".
  > #[derive(
  >     Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
  > )]
  > #[serde(transparent)]
  > pub struct Cursor(pub u64);

- **Fact:** profiles are the same profile when their slugs are equal, so the fake's membership rule (`check_membership`) compares `ProfileName::slug`, not the display names.
  **Source:** `crates/holler-pane/src/profile.rs:68-74`
  **Verbatim excerpt:**
  > /// The unique id derived from the name: ASCII letters and digits in lower case,
  > /// each run of anything else becoming one `-`, with no leading or trailing `-`.
  > /// Two display names with the same slug are the same profile as far as
  > /// uniqueness goes.
  > pub fn slug(&self) -> String {
  >     slugify(&self.0)
  > }

- **Fact:** the suite's `expect_code` compares `PaneError::code()` with the kebab-case strings `generation-conflict`, `pane-not-found` and `pane-in-other-profile`, which are the codes of `PaneError::Conflict`, `PaneNotFound` and `PaneInOtherProfile`.
  **Source:** `crates/holler-pane/src/error.rs:404-409`, `crates/holler-pane/src/error.rs:420`, `crates/holler-pane/src/error.rs:427`, `crates/holler-pane/src/error.rs:431`, `crates/holler-pane/src/error.rs:101`, `crates/holler-pane/src/error.rs:108`, `crates/holler-pane/src/error.rs:112`
  **Verbatim excerpt:**
  > pub fn code(&self) -> &str {
  >     match self.classify() {
  >         Ok(closed) => closed.as_str(),
  >         Err(open) => open.as_str(),
  >     }
  > }
  >
  > PaneError::Conflict => Ok(PaneCode::GenerationConflict),
  > PaneError::PaneInOtherProfile { .. } => Ok(PaneCode::PaneInOtherProfile),
  > PaneError::PaneNotFound { .. } => Ok(PaneCode::PaneNotFound),
  >
  > PaneCode::GenerationConflict => "generation-conflict",
  > PaneCode::PaneInOtherProfile => "pane-in-other-profile",
  > PaneCode::PaneNotFound => "pane-not-found",

- **Fact:** a pane's tmux session name equals the pane's name, so `sample_pane` sets `host.tmux` to the name it is given (its Herdr session and workspace are the scratch name instead).
  **Source:** `crates/holler-pane/src/pane.rs:108-109`
  **Verbatim excerpt:**
  > /// The tmux session name (equal to the pane's name).
  > pub tmux: String,
