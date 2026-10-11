# Evidence: #652, facts in unchanged code that the guide relies on

## F (Phase 5)

- **Fact:** a verb whose `--herdr-session` names a session other than the one the Herdr adapter is configured with answers `unavailable` (guide section 2: "The two must name the same session, or the verb answers `unavailable`").
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:180-185`
  **Verbatim excerpt:**
  > ```
  >         if spec.session != self.config.session {
  >             return Err(PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr session {:?}: this adapter serves only the session {:?}",
  >                     spec.session, self.config.session
  >                 ),
  > ```

- **Fact:** `profile show` prints a spec with no agent as `(default)`, never `none` (guide section 2).
  **Source:** `crates/holler-pane/src/profile_diff.rs:204`
  **Verbatim excerpt:**
  > ```
  >             FieldValue::Key(None) => f.write_str("(default)"),
  > ```

- **Fact:** the text result of `launch` and `relaunch` puts `, agent KEY` after `harness port N` (the guide's example line in section 2).
  **Source:** `crates/holler-cli/src/pane/launch.rs:399-408`
  **Verbatim excerpt:**
  > ```
  >                     "{} {} at {} in {}: session {}, harness port {}{pid}",
  > ```
  > ```
  >                     line.push_str(&format!(", agent {}", text_value(agent.as_str())));
  > ```

- **Fact:** a doctor finding line is `<subject> <kind>: <message>`, with no bracket for a `not-fixable` finding, `[fix failed: CODE]` for a failed fix, `[state]` otherwise, then `(run: <remedy>)` (guide section 5).
  **Source:** `crates/holler-cli/src/pane/doctor.rs:103-110`
  **Verbatim excerpt:**
  > ```
  >     let mut line = format!("{subject} {}: {}", finding.kind.code(), finding.message);
  >     match (finding.fix, &finding.fix_error) {
  >         (FixState::NotFixable, _) => {}
  >         (FixState::Failed, Some(error)) => line.push_str(&format!(" [fix failed: {}]", error.code)),
  >         (state, _) => line.push_str(&format!(" [{}]", state.as_str())),
  >     }
  >     if let Some(remedy) = &finding.remedy {
  >         line.push_str(&format!(" (run: {remedy})"));
  > ```

- **Fact:** a `not-fixable` (or failed) `shown-driven-mismatch` names `holler pane relaunch PANE` as its command (guide section 5).
  **Source:** `crates/holler-pane/src/findings.rs:136-139`
  **Verbatim excerpt:**
  > ```
  >                 FixState::Fixable | FixState::Skipped => {
  >                     pane.map(|pane| doctor_command(Some(pane), true))
  >                 }
  >                 FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
  > ```

- **Fact:** the PROPOSED profile verbs are stubs that answer `not-implemented` (guide section 3).
  **Source:** `crates/holler-cli/src/profile/export.rs:21`
  **Verbatim excerpt:**
  > ```
  >     emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
  > ```

## T (Phase 6)

Facts in unchanged code that `crates/holler-cli/tests/docs_pane_control_test.rs` relies on and does not compute.

- **Fact:** the test's hand-kept `APPLY_ACTIONS` and `APPLY_OUTCOMES` are exactly the words of `tx_apply`'s `Action::as_str` and `Outcome::as_str` (acceptance 15; neither enum has an `ALL` table).
  **Source:** `crates/holler-pane/src/tx_apply.rs:148-157` and `crates/holler-pane/src/tx_apply.rs:186-197`
  **Verbatim excerpt:**
  > ```
  >     pub const fn as_str(self) -> &'static str {
  >         match self {
  >             Action::Create => "create",
  >             Action::Adopt => "adopt",
  >             Action::Report => "report",
  >             Action::Relaunch => "relaunch",
  >             Action::Refuse => "refuse",
  >             Action::Observe => "observe",
  >         }
  >     }
  > ```
  > ```
  >     pub const fn as_str(self) -> &'static str {
  >         match self {
  >             Outcome::Created => "created",
  >             Outcome::Relaunched => "relaunched",
  >             Outcome::Adopted => "adopted",
  >             Outcome::Reported => "reported",
  >             Outcome::Refused => "refused",
  >             Outcome::Failed => "failed",
  >             Outcome::Skipped => "skipped",
  >             Outcome::Planned => "planned",
  >         }
  >     }
  > ```

- **Fact:** `pane import`'s `--herdr-session` is optional to clap and required at run time (`usage` without it), which is why the test adds `--herdr-session NAME` to the computed heading form of `pane import` (acceptance 17).
  **Source:** `crates/holler-cli/src/pane/import.rs:84-86` and `crates/holler-cli/src/pane/import.rs:152-158`
  **Verbatim excerpt:**
  > ```
  >     /// The Herdr session the panes live in. Required.
  >     #[arg(long, value_name = "NAME")]
  >     pub herdr_session: Option<String>,
  > ```
  > ```
  >         let herdr_session = args
  >             .herdr_session
  >             .clone()
  >             .filter(|session| !session.trim().is_empty())
  >             .ok_or_else(|| {
  >                 usage("--herdr-session is required: name the Herdr session the panes live in")
  >             })?;
  > ```

- **Fact:** `profile-drift` (the test's `PROPOSED_FINDING`) is a closed error code but not yet a `FindingKind` (#665 adds the kind).
  **Source:** `crates/holler-pane/src/error.rs:121` and `crates/holler-pane/src/findings.rs:3`
  **Verbatim excerpt:**
  > ```
  >             PaneCode::ProfileDrift => "profile-drift",
  > ```
  > ```
  > //! untrusted text a finding quotes. #665 adds only the `ProfileDrift` kind after #647 has
  > ```
