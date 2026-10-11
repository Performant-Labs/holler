# Evidence: #650, behaviour in unchanged code that the diff relies on

Written by F (Phase 6, implement). Each entry is one fact, its source and the exact lines. T appends its own entries at
Phase 7 (verify).

## F, Phase 6

- **Fact:** `insert_profile` is the one create write: a compare-and-swap at generation 0 whose `generation-conflict` becomes
  `profile-exists`; every other error passes through. The import's real run writes the profile through it.
  **Source:** `crates/holler-cli/src/profile/create.rs:208-219`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn insert_profile(
  >     store: &dyn ProfileStore,
  >     profile: &Profile,
  >     actor: &Actor,
  > ) -> Result<Profile, PaneError> {
  >     store
  >         .cas_put(profile, 0, actor)
  >         .map_err(|error| match error {
  >             PaneError::Conflict => taken(&profile.name),
  >             other => other,
  >         })
  > }
  > ```

- **Fact:** a profile name in a suggested command goes through `single_quoted`, so the undo's remedy prints `'fleet'`.
  **Source:** `crates/holler-cli/src/profile/delete.rs:206-208`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn single_quoted(text: &str) -> String {
  >     format!("'{}'", text.replace('\'', r"'\''"))
  > }
  > ```

- **Fact:** a record is a member of a profile when its `profile` has the profile's slug; the undo's step (a) deletes the
  failed write's record only when `get` shows it as a member of `fleet` (it landed).
  **Source:** `crates/holler-pane/src/profile_diff.rs:261-265`
  **Verbatim excerpt:**
  > ```rust
  > pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
  >     pane.profile
  >         .as_ref()
  >         .is_some_and(|own| own.slug() == profile.slug())
  > }
  > ```

- **Fact:** any well-formed code that is not closed is a refusal, so `import-blocked` (a `RefusalCode`) exits 3.
  **Source:** `crates/holler-pane/src/error.rs:266-273`
  **Verbatim excerpt:**
  > ```rust
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```

- **Fact:** the record's `host.name` is launch's constant, so an imported record names the host as a launched one does.
  **Source:** `crates/holler-pane/src/tx_launch.rs:74`
  **Verbatim excerpt:**
  > `pub const HOST_NAME: &str = "localhost";`

- **Fact:** the dry run's `data.profile` is `profile_from_panes(fleet, &records)`: generation, `created` and `updated` 0 (the
  store stamps them).
  **Source:** `crates/holler-pane/src/profile_snapshot.rs:74-83`
  **Verbatim excerpt:**
  > ```rust
  > pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile {
  >     Profile {
  >         name: name.clone(),
  >         slug: name.slug(),
  >         generation: 0,
  >         panes: panes.iter().map(spec_from_pane).collect(),
  >         created: 0,
  >         updated: 0,
  >     }
  > }
  > ```

- **Fact:** `holler_body::config`'s refusal of `sessions.toml` is one of four variants; `Invalid`'s `field` is a
  `&'static str` and `row` a 0-based index, while `Toml` and `Invalid.name`/`reason` carry text of the file. The import
  prints only the file, the row and the field.
  **Source:** `crates/holler-body/src/config.rs:140-154`
  **Verbatim excerpt:**
  > ```rust
  > pub enum ConfigError {
  >     /// The file failed to parse as TOML, or violated a `deny_unknown_fields`
  >     /// gate (an unknown top-level or per-session key). The message is the
  >     /// `toml` crate's own (it already names the offending key/line).
  >     Toml(String),
  >     /// A per-session validation failure: `row` is the 0-based index into the
  >     /// file's `[[session]]` array, `name` is that row's (possibly invalid)
  >     /// `name` value, `field` is the offending key, `reason` is human-
  >     /// readable.
  >     Invalid { row: usize, name: String, field: &'static str, reason: String },
  >     /// No config file was found via any discovery source.
  >     NoConfigFound,
  >     /// The file could not be read (permissions, not a file, etc).
  >     Io(String),
  > }
  > ```

- **Fact:** `toml::from_str` (0.8.23, the version `Cargo.lock` pins) answers a `de::Error` whose `span()` is the byte range of
  the error in the parsed text, or `None`; the launcher reader counts a line and column from it and never prints the
  error's text.
  **Source:** `~/.cargo/registry/src/<index>/toml-0.8.23/src/de.rs:68-72` (crates.io source, not the repository)
  **Verbatim excerpt:**
  > ```rust
  >     /// The start/end index into the original document where the error occurred
  >     #[cfg(feature = "parse")]
  >     pub fn span(&self) -> Option<std::ops::Range<usize>> {
  >         self.inner.span()
  >     }
  > ```

- **Fact:** the reconcile classification the import copies (`reconcile.rs` and `reconcile/observe.rs` are frozen, #647):
  `health` first; after `Ok(true)` the list is read; after `Ok(false)` one list call tells wedged (a `timeout`) from down,
  with reconcile's sentences.
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:153-183`
  **Verbatim excerpt:**
  > ```rust
  >     match ports.harness.health(port) {
  >         Err(error) => {
  >             findings.push(observe_failed(pane, HARNESS_HEALTH, &error));
  >             Server::Unknown
  >         }
  >         Ok(true) => match ports.harness.list_sessions(port) {
  >             Ok(ids) => Server::Healthy {
  >                 sessions: Some(ids),
  >             },
  >             Err(error) => {
  >                 findings.push(observe_failed(pane, HARNESS_LIST_SESSIONS, &error));
  >                 Server::Healthy { sessions: None }
  >             }
  >         },
  >         Ok(false) => {
  >             let wedged = matches!(
  >                 ports.harness.list_sessions(port),
  >                 Err(PaneError::Timeout { .. })
  >             );
  >             let (server, kind, state) = if wedged {
  >                 let state = "accepts a connection and does not answer";
  >                 (Server::Wedged, FindingKind::ServerWedged, state)
  >             } else {
  >                 let state = "does not pass its health check";
  >                 (Server::Down, FindingKind::ServerDown, state)
  >             };
  >             let message = format!("the harness server on port {port} {state}");
  >             findings.push(Finding::about(kind, pane, message).with_ports(vec![port]));
  >             server
  >         }
  >     }
  > ```

- **Fact:** reconcile's `observe-failed` message form, copied as the import's `failed_message`.
  **Source:** `crates/holler-pane/src/reconcile.rs:438-444`
  **Verbatim excerpt:**
  > ```rust
  > fn failed_message(op: &str, error: &PaneError) -> String {
  >     format!(
  >         "{op} failed ({}): {}",
  >         error.code(),
  >         embedded(&error.to_string())
  >     )
  > }
  > ```

- **Fact:** `HarnessKind` is closed to `opencode` and deserializes from its snake_case name, so `harness_kind` (a serde
  read of a `sessions.toml` row's `harness`) is `Some` exactly for `"opencode"`.
  **Source:** `crates/holler-pane/src/pane.rs:134-138`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case")]
  > pub enum HarnessKind {
  >     Opencode,
  > }
  > ```

- **Fact:** a `pane.env` NAME is kept only when `EnvVarName::parse` accepts it: no `=`, not empty, no whitespace or control
  character. A NAME with a trailing space (`export OC_MODEL = x`) is refused, which the grammar reports as its line.
  **Source:** `crates/holler-pane/src/argv.rs:101-109`
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

- **Fact:** the dry run's text prints each spec through `profile show`'s own `push_spec`, whose body the diff leaves
  unchanged (only its visibility widens).
  **Source:** `crates/holler-cli/src/profile/show.rs:136-143`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn push_spec(lines: &mut Vec<String>, spec: &ProfileSpec) {
  >     lines.push(format!("spec {}", text(&spec.pane)));
  >     lines.extend(
  >         SpecField::ALL
  >             .into_iter()
  >             .map(|field| format!("  {}: {}", field.as_str(), field.value(spec))),
  >     );
  > }
  > ```

- **Fact:** in text mode an error goes to `err` as one `error: <message>` line, with nothing on `out`; the import's usage
  errors and `import-blocked` rely on it.
  **Source:** `crates/holler-cli/src/output.rs:281-284`
  **Verbatim excerpt:**
  > ```rust
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  >         }
  > ```

- **Fact (for T, about two tests):** every call through a fake's port, `list` included, is appended to its call log, and
  the import rig's `records()` reads the store through the port. `a_failed_write_is_undone_and_answers_its_own_code` and
  `a_write_that_landed_and_timed_out_is_undone_too` call `rig.records()` before they read the log's last three calls, so
  that window ends with the rig's own `List`.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:93-97`, `crates/holler-pane-testkit/src/pane_store.rs:182-185`,
  `crates/holler-cli/tests/pane_verbs/import/rig.rs:256-258`, `crates/holler-cli/tests/pane_verbs/import/writes.rs:251-254`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  >             state.delay
  > ```
  > ```rust
  >     fn list(&self) -> Result<Vec<Pane>, PaneError> {
  >         self.faults.enter(PaneStoreOp::List)?;
  >         Ok(self.feed.read(|log| log.records().cloned().collect()))
  >     }
  > ```
  > ```rust
  >     pub fn records(&self) -> Vec<Pane> {
  >         self.world.panes.list().expect("the store answers")
  >     }
  > ```
  > ```rust
  >     assert!(rig.records().is_empty(), "the pane store is empty");
  >     assert!(rig.fleet().is_none(), "fleet is deleted");
  >     assert_eq!(
  >         pane_ops_tail(&rig, &mark, 3)[..],
  > ```
