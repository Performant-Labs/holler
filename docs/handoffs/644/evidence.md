# Evidence: #644 launch-relaunch (behaviour outside the diff that the change relies on)

Each entry quotes unchanged source at the merge base `d9eabbb` this branch is cut from.

## F (implementation)

- **Fact:** a closed code rebuilt through `from_wire` gets its own variant back, with the `detail` string as its payload. The engine's `with_note` relies on this to add the rollback note and keep the code.
  **Source:** `crates/holler-pane/src/error.rs:565-566`, `crates/holler-pane/src/error.rs:599-601`
  **Verbatim excerpt:**
  > ```rust
  >     fn from_closed(code: PaneCode, message: String, detail: Option<String>) -> PaneError {
  >         let text = detail.unwrap_or(message);
  > ```
  > ```rust
  >     pub(crate) fn from_wire(code: String, message: String, detail: Option<String>) -> PaneError {
  >         if let Some(closed) = PaneCode::parse(&code) {
  >             return PaneError::from_closed(closed, message, detail);
  > ```

- **Fact:** `detail()` is the one string payload a variant carries. It is `None` for the payload-less variants and for `Refused`, which `with_note` extends through its `message` instead.
  **Source:** `crates/holler-pane/src/error.rs:535-542`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn detail(&self) -> Option<&str> {
  >         match self {
  >             PaneError::NotImplemented
  >             | PaneError::CommandNotArgv
  >             | PaneError::EnvNameInvalid
  >             | PaneError::Conflict
  >             | PaneError::ProfileSecretRefused
  >             | PaneError::Refused { .. } => None,
  > ```

- **Fact:** O1's comparison is reconcile's: with a session of record, any other SHOWN differs, the home screen (`None`) included.
  **Source:** `crates/holler-pane/src/reconcile.rs:179-180`
  **Verbatim excerpt:**
  > ```rust
  > pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
  >     session_of_record.is_some_and(|record| shown != Some(record))
  > ```

- **Fact:** `findings::quoted` is `{:?}` quoting cut to 64 characters, so a quoted plain id still contains the id. Every test asserts only that the id is contained.
  **Source:** `crates/holler-pane/src/findings.rs:332-333`, `crates/holler-pane/src/error.rs:688-691`
  **Verbatim excerpt:**
  > ```rust
  > pub fn quoted(text: &str) -> String {
  >     excerpt(text)
  > ```
  > ```rust
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  > ```

- **Fact:** `edit_spec(None, ..)` runs only the act and touches no profile store in the real scope (the fake does the same, brief F-7). So a launch without `--profile` makes no profile call (AC 16f).
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:187-188`
  **Verbatim excerpt:**
  > ```rust
  >         let Some(profile) = profile else {
  >             return act().map(|()| None);
  > ```

- **Fact:** the fake scope's restore-conflict message embeds the act's error and carries no reconcile step, so the verb appends the step once (AC 16h, 16j). The real scope's message carries the step (brief K-4), so the verb appends none (AC 16k).
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:172-177`
  **Verbatim excerpt:**
  > ```rust
  >             Err(PaneError::Conflict) => PaneError::ProfileConflict {
  >                 what: format!(
  >                     "{:?} was changed by another writer during the live change to {pane}, \
  >                      so its specs were not restored after that change failed ({failure}); \
  >                      the other writer's version stays",
  > ```

- **Fact:** the fake harness's `health` is true only for a running server, so a frozen server on the port passes step 6 and `serve` then answers `timeout` on it (AC 4a). A killed server's port is served again with a new pid (relaunch, AC 21).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:287-289`, `crates/holler-pane-testkit/src/harness.rs:377-384`
  **Verbatim excerpt:**
  > ```rust
  >     fn health(&self, port: u16) -> Result<bool, PaneError> {
  >         self.faults.enter(HarnessOp::Health)?;
  >         Ok(self.lock().state(port) == Some(ServerState::Running))
  > ```
  > ```rust
  >                 ServerState::Running if server.name == *name => return Ok(server.pid),
  >                 ServerState::Running => {
  >                     return Err(PaneError::Unavailable {
  >                         what: format!("port {port} is in use by the server of {}", server.name),
  >                     })
  >                 }
  >                 ServerState::Frozen => return Err(frozen(HarnessOp::Serve)),
  >                 ServerState::Killed => {}
  > ```

- **Fact:** the pane store ignores the generation a record carries and stores a create at 1. So the engine passes `generation: 0` (launch) or the read generation (relaunch) and calls no `next_generation` itself.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:50-52`
  **Verbatim excerpt:**
  > ```rust
  > /// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
  > ///   create names generation 0 and is stored at 1, the submitted generation is
  > ///   ignored, and a stale or an ahead generation is `generation-conflict`.
  > ```

- **Fact:** the verbs reuse the read verbs' `--profile` parser and text sanitizers rather than writing their own.
  **Source:** `crates/holler-cli/src/pane/list.rs:142`, `crates/holler-cli/src/pane/list.rs:280`, `crates/holler-cli/src/pane/list.rs:294`
  **Verbatim excerpt:**
  > ```rust
  > pub fn profile_name(opt: &ProfileOpt) -> Result<Option<ProfileName>, PaneError> {
  > ```
  > ```rust
  > pub fn text_value(value: &str) -> String {
  > ```
  > ```rust
  > pub fn optional_text(value: Option<&str>) -> String {
  > ```

## T (tests)

- **Fact:** the launch rig (`tests/pane_verbs/launch/rig.rs`) takes its fakes from #643's `crate::list::Rig`: the two stores are seeded in order (each pane a create, so stored at generation 1, per F's pane-store entry above), the fake scope runs over those same two `Arc` stores, and the rig then replaces only `herdr` (with the `main` workspace) and wraps the host and harness. So the "P at generation 1" and "the record at generation 1" seeds the tests assert from come from this unchanged constructor.
  **Source:** `crates/holler-cli/tests/pane_verbs/list.rs:39`, `crates/holler-cli/tests/pane_verbs/list.rs:52-59`, `crates/holler-pane-testkit/src/pane_store.rs:93-96`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) struct Rig {
  > ```
  > ```rust
  >     pub fn new(
  > ```
  > ```rust
  >         let panes = Arc::new(FakePaneStore::seeded(panes)?);
  >         let profiles = Arc::new(FakeProfileStore::seeded(profiles, &actor)?);
  >         let scope = FakeProfileScope::new(profiles.clone(), panes.clone(), actor);
  > ```
  > ```rust
  >     pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
  >         let store = Self::new();
  >         for pane in panes {
  >             store.put(&pane, Writer::Port(0))?;
  > ```
