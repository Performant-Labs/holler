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
