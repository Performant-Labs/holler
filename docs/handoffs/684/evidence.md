# Evidence: #684 (test kit slice e)

Facts in unchanged code that the diff (F's production code) relies on. F, Phase 5.

- **Fact:** every fake's port method calls the fault switch's `enter` first. It logs the call before answering, then
  answers the standing fault or the oldest queued one-shot error. So a failed call is still in `calls()`, and a method
  that returns on `enter`'s error changes nothing.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:93-103`
  **Verbatim excerpt:**
  > ```
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

- **Fact:** a wedged port answers `PaneError::Timeout { op: op.as_str().to_owned() }`. The harness fake's frozen
  `timeout` (`harness.rs`, `fn frozen`) builds the same shape, so a frozen server and a wedged adapter both name the
  method in `op`.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:119-125`
  **Verbatim excerpt:**
  > ```
  >     fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
  >         match &self.standing {
  >             Some(Fault::Wedged) => {
  >                 return Err(PaneError::Timeout {
  >                     op: op.as_str().to_owned(),
  >                 })
  >             }
  > ```

- **Fact:** each fake's one lock line is slice a's idiom: a poisoned lock is taken over, not unwrapped.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:105-107`
  **Verbatim excerpt:**
  > ```
  >     fn lock(&self) -> MutexGuard<'_, State<Op>> {
  >         self.state.lock().unwrap_or_else(PoisonError::into_inner)
  >     }
  > ```

- **Fact:** `run_cases` calls `fresh` once per case, checks the case against `&subject`, and drops the subject before
  the guard. `run_harness_conformance` folds the rig into the subject (`(harness, rig)`), so the harness and its rig
  drop before the guard, and `run_cases` stays unchanged.
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:47-59`
  **Verbatim excerpt:**
  > ```
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
  >             outcome.err().map(|detail| CaseFailure { case, detail })
  > ```

- **Fact:** the code strings the suites pass to `expect_code` are the closed codes of the variants the fakes return:
  `usage`, `timeout`, `pane-not-found`, `session-not-found` and `unavailable`.
  **Source:** `crates/holler-pane/src/error.rs:101, 116-118, 120`
  **Verbatim excerpt:**
  > ```
  >             PaneCode::Usage => "usage",
  >             PaneCode::Timeout => "timeout",
  >             PaneCode::PaneNotFound => "pane-not-found",
  >             PaneCode::SessionNotFound => "session-not-found",
  >             PaneCode::Unavailable => "unavailable",
  > ```

- **Fact:** `pane-not-found` and `session-not-found` are refusals, and `timeout` and `unavailable` are failures. The
  harness suite's doc says the no-TUI `unavailable` is "a failure (exit 1), as a timeout's is".
  **Source:** `crates/holler-pane/src/error.rs:290-298`
  **Verbatim excerpt:**
  > ```
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  >         // Went wrong while doing the work: a race between writers, a bound that ran
  >         // out, something unreachable or unreadable, live state that disagrees with
  >         // its spec, or work the verb cannot do yet.
  >         PaneCode::GenerationConflict
  >         | PaneCode::ProfileConflict
  >         | PaneCode::Timeout
  >         | PaneCode::Unavailable
  > ```

- **Fact:** `PaneName` displays as the bare name and orders by it (it wraps `SessionName(String)`, which derives
  `Ord`). So `FakeHost::sessions()`, the keys of a `BTreeMap<PaneName, _>`, is sorted by name, and every `what` the
  fakes build from a name (`pane-not-found`, "in use by the server of ...") is the bare name.
  **Source:** `crates/holler-pane/src/pane.rs:31-32, 50-54`; `crates/holler-proto/src/vocab.rs:73-74`
  **Verbatim excerpt:**
  > ```
  > #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  > pub struct PaneName(SessionName);
  > ...
  > impl fmt::Display for PaneName {
  >     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
  >         f.write_str(self.as_str())
  >     }
  > }
  > ...
  > #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  > pub struct SessionName(String);
  > ```

- **Fact:** `HarnessPort::select_session` takes a pane and a session but no port. So the fake (and the suite's case
  14) checks for the pane's TUI before anything else: the TUI is what names the server to reach.
  **Source:** `crates/holler-pane/src/ports.rs:196`
  **Verbatim excerpt:**
  > ```
  >     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;
  > ```
