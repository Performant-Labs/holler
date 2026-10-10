# Evidence: #645a switch-reset (F, Phase 5 of the pipeline / Phase 6 of the Workflow script)

Facts in **unchanged** code that the diff (`tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs`, the ADR-0021 edits) or T's
tests rely on. Each excerpt is copied from the tree at `a912c4b` plus F's uncommitted change (none of these files is in
F's change).

## What the engine calls in `holler-pane`

- **Fact:** the one remedy table maps `ServerDown` to the relaunch command for a named pane, whatever the `FixState`, so
  P3's `FindingKind::ServerDown.remedy(Some(&pane), FixState::NotFixable)` is `Some("holler pane relaunch <pane>")` and
  its `doctor_command` fallback is never taken.
  **Source:** `crates/holler-pane/src/findings.rs:127-131` and `:319-320`
  **Verbatim excerpt:**
  > ```
  >             FindingKind::HerdrPaneMissing
  >             | FindingKind::TmuxSessionMissing
  >             | FindingKind::ServerWedged
  >             | FindingKind::ServerDown
  >             | FindingKind::TuiForeignSession => pane.map(relaunch_command),
  > ```
  > ```
  > fn relaunch_command(pane: &PaneName) -> String {
  >     format!("{RELAUNCH} {pane}")
  > ```

- **Fact:** `doctor_command(Some(pane), true)` is `holler pane doctor <pane> --fix`, the reconcile step
  `SwitchFailure::message` appends after `; to reconcile, run `.
  **Source:** `crates/holler-pane/src/findings.rs:306-316`
  **Verbatim excerpt:**
  > ```
  > pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String {
  >     let mut line = DOCTOR.to_owned();
  >     if let Some(pane) = pane {
  >         line.push(' ');
  >         line.push_str(pane.as_str());
  >     }
  >     if fix {
  >         line.push_str(" --fix");
  >     }
  >     line
  > }
  > ```

- **Fact:** `findings::quoted` is `error::excerpt`: the text `{:?}`-quoted (so every control character is escaped) and cut
  at 64 characters with `...`. So a typed session id (at most `SESSION_ID_MAX = 64`) is never cut, and the escape case of
  AC 12 prints `\u{1b}`, never a raw ESC.
  **Source:** `crates/holler-pane/src/findings.rs:332-334` and `crates/holler-pane/src/error.rs:688-695`
  **Verbatim excerpt:**
  > ```
  > pub fn quoted(text: &str) -> String {
  >     excerpt(text)
  > ```
  > ```
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }
  > ```

- **Fact:** `shown_differs(Some(target), shown)` is true for any screen but the target, the home screen (`None`) included.
  **Source:** `crates/holler-pane/src/reconcile.rs:179-181`
  **Verbatim excerpt:**
  > ```
  > pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
  >     session_of_record.is_some_and(|record| shown != Some(record))
  > }
  > ```

- **Fact:** the private `screen_text` that `tx_switch.rs` copies word for word (reconcile declares `observe` private, so
  it cannot be called from `tx_switch.rs`).
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:342-348`
  **Verbatim excerpt:**
  > ```
  > /// How a message names what the TUI shows.
  > fn screen_text(shown: Option<&str>) -> String {
  >     shown.map_or_else(
  >         || "its home screen".to_owned(),
  >         |session| format!("session {}", quoted(session)),
  >     )
  > }
  > ```

- **Fact:** the exit class of every code the verbs raise: an open code (the three `RefusalCode`s) is a refusal (3);
  `pane-not-found`, `session-not-found`, `pane-not-in-profile` and `profile-not-found` are refusals (3);
  `generation-conflict`, `timeout` and `unavailable` are failures (1). This is why a mismatch after the act must be the
  closed `unavailable`, not an open code (Decision 9).
  **Source:** `crates/holler-pane/src/error.rs:266-273`, `:285`, `:289-291`, `:295-298`
  **Verbatim excerpt:**
  > ```
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```
  > ```
  >         | PaneCode::PaneNotInProfile
  > ```
  > ```
  >         | PaneCode::ProfileNotFound
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  > ```
  > ```
  >         PaneCode::GenerationConflict
  >         | PaneCode::ProfileConflict
  >         | PaneCode::Timeout
  >         | PaneCode::Unavailable
  > ```

- **Fact:** how the errors the engine returns display, which is the start of every failure message: `Refused` is its
  message alone, `Unavailable` is prefixed `unavailable: `, and a record conflict has its own fixed text.
  **Source:** `crates/holler-pane/src/error.rs:651-653`, `:673-679`
  **Verbatim excerpt:**
  > ```
  >             PaneError::Conflict => f.write_str(
  >                 "the record changed since it was read (generation conflict); read it again and retry",
  >             ),
  > ```
  > ```
  >             PaneError::Timeout { op } => write!(f, "timed out: {op}"),
  >             PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
  >             PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
  >             PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
  >             PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
  >             PaneError::ProfileDrift { message } => write!(f, "profile drift: {message}"),
  >             PaneError::Refused { message, .. } => f.write_str(message),
  > ```

## The ports' contracts the engine relies on

- **Fact:** `cas_put` returns the record as stored, with its bumped generation, which is what `Switched.pane` (and so
  JSON `data.pane`) carries.
  **Source:** `crates/holler-pane/src/ports.rs:69-71`
  **Verbatim excerpt:**
  > ```
  >     /// Store `pane` if the stored one is still at `expected_generation` (0 for a
  >     /// new pane); returns the stored record with its bumped generation.
  >     fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
  > ```

- **Fact:** `resolve(P, Some(n))` answers just the pane `n`, which must belong to P; so P1's `find` by name picks that
  pane, and differs from `into_iter().next()` only for a scope that breaks its contract.
  **Source:** `crates/holler-pane/src/profile.rs:381-389`
  **Verbatim excerpt:**
  > ```
  >     /// The profile and the panes of it a verb acts on. With no `pane`, every pane
  >     /// of the profile; with a named pane, just that one, which must belong to the
  >     /// profile (`pane-not-in-profile` otherwise). A missing profile is
  >     /// `profile-not-found`.
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError>;
  > ```

- **Fact:** the hub registry's compare-and-swap checks only the generation and the membership rule, nothing about
  `session_of_record`, which is why the ADR-0021 paragraph says P5 (`session-of-other-pane`) is a read in the verb and
  not the authority.
  **Source:** `crates/holler-hub/src/panes/store.rs:170-175`
  **Verbatim excerpt:**
  > ```
  >     pub(crate) fn cas_put(&self, pane: &Pane, expected: u64) -> Result<Pane, PaneError> {
  >         let mut guard = self.lock();
  >         let table = guard.as_mut().map_err(|err| err.clone())?;
  >         let current = table.record(&pane.name);
  >         let generation = next_generation(current.map_or(0, |stored| stored.generation), expected)?;
  >         refuse_profile_move(current, pane)?;
  > ```

- **Fact:** doctor's remedy for a pane with no session of record, or a missing one, is `holler pane reset <pane>`, which
  this story makes a working verb (AC 15).
  **Source:** `crates/holler-pane/src/findings.rs:132-134`
  **Verbatim excerpt:**
  > ```
  >             FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
  >                 pane.map(reset_command)
  >             }
  > ```

## The CLI's output, which `emit_outcome` and `execute` go through

- **Fact:** text mode writes a failure as `error: <message>` on `err`; JSON mode puts the message on one line and takes
  the exit code from the code's class, the same in both modes.
  **Source:** `crates/holler-cli/src/output.rs:281-284`, `:288-297`
  **Verbatim excerpt:**
  > ```
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  >         }
  > ```
  > ```
  > fn emit_json<T: Serialize>(sink: &mut Sink<'_>, result: Result<T, ErrorBody>) -> i32 {
  >     match result {
  >         Ok(data) => settle(write_envelope(sink, &Envelope::success(data)), 0),
  >         Err(error) => {
  >             let code = exit_code(&error);
  >             let error = ErrorBody {
  >                 message: one_line(&error.message),
  >                 ..error
  >             };
  > ```

- **Fact:** until #649 the real binary's ports answer `not-implemented`, so the CHANGELOG's "the real verbs answer
  `not-implemented`" holds (checked by running the built binary: exit 1 `not-implemented`, and exit 2 `usage` for a bad
  argument, before any port).
  **Source:** `crates/holler-cli/src/pane/wiring.rs:8-9`
  **Verbatim excerpt:**
  > ```
  > //! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
  > //! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
  > ```

## The test kit's fakes, which T's ACs rely on

- **Fact:** the fake's `health` is true only for a running server, so a killed or frozen one makes P3 refuse with
  `server-unhealthy` after exactly one `Health` call (AC 5, AC 17).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:287-290`
  **Verbatim excerpt:**
  > ```
  >     fn health(&self, port: u16) -> Result<bool, PaneError> {
  >         self.faults.enter(HarnessOp::Health)?;
  >         Ok(self.lock().state(port) == Some(ServerState::Running))
  >     }
  > ```

- **Fact:** with `SelectAckedWithoutTui` on and no TUI, `select_session` answers `Ok(())` and changes nothing, so only
  O1's observation catches the mismatch (ACs 7 and 19).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:336-341`
  **Verbatim excerpt:**
  > ```
  >     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
  >         self.faults.enter(HarnessOp::SelectSession)?;
  >         let mut world = self.lock();
  >         if !world.tuis.contains_key(pane) && world.quirks.contains(&Quirk::SelectAckedWithoutTui) {
  >             return Ok(());
  >         }
  > ```
