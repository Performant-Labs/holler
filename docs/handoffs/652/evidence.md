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

## F rework (after S ADVISORY-HOLD; #649 and #832 merged)

- **Fact:** with no hub every verb is `unavailable` naming the hub and touches no adapter; a missing or malformed variable never fails `connect`; the five read verbs need only the hub.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:22-27`
  **Verbatim excerpt:**
  > ```
  > //! **What `connect` needs: the hub, and nothing else (D2).** It makes one `control/status`
  > //! exchange, and any failure of it (no hub, an error, no answer within 5 s) is `unavailable`
  > //! naming the hub, so with no hub every verb exits 1 and touches no adapter. It touches no Herdr,
  > //! tmux or OpenCode, and a configuration variable that is missing or malformed never fails it
  > //! (D26): the error is kept, and the port it configures answers it at its first use. So `pane list`,
  > //! `pane get`, `pane watch`, `profile list` and `profile show` work with nothing but the hub.
  > ```

- **Fact:** `HOLLER_HERDR_SESSION` names the Herdr session; Herdr's `HERDR_SESSION` is not read; unset, each Herdr call answers `usage`.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:33`
  **Verbatim excerpt:**
  > ```
  > //! | `HOLLER_HERDR_SESSION` | the one Herdr session the panes live in (Herdr's `HERDR_SESSION` is not read) | each Herdr call answers `usage` naming it |
  > ```

- **Fact:** the Herdr adapter refuses a spec in a session other than its configured one with `unavailable`, naming both.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:180-188`
  **Verbatim excerpt:**
  > ```
  >     fn extent_of(&self, spec: &HerdrSpec) -> Result<Extent, PaneError> {
  >         if spec.session != self.config.session {
  >             return Err(PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr session {:?}: this adapter serves only the session {:?}",
  >                     spec.session, self.config.session
  >                 ),
  >             });
  >         }
  > ```

- **Fact:** every pane of the adapter's snapshot is stamped with the adapter's configured session, so an import filtered by a different `--herdr-session` sees no pane.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:262-264`
  **Verbatim excerpt:**
  > ```
  >         HerdrPane {
  >             session: self.config.session.clone(),
  >             workspace: workspace.to_owned(),
  > ```

- **Fact:** the import looks only in the session `--herdr-session` names, and no pane with the label is the problem `pane-unlabelled`.
  **Source:** `docs/adr/ADR-0021.md:853-855`
  **Verbatim excerpt:**
  > - **Decided (#650): the import's join.** It looks only in the Herdr session `--herdr-session` names (the snapshot filtered
  >   by `session`, as launch's occupancy check is), so a pane of the same label in another Herdr session is no candidate. No
  >   pane with the label is the problem `pane-unlabelled`, and two or more the problem `pane-label-ambiguous` (naming their

- **Fact:** a pane's model, effort, environment names and ceilings are recorded, not applied (#829).
  **Source:** `docs/adr/ADR-0021.md:604-605`
  **Verbatim excerpt:**
  > (launch writes none, relaunch keeps the stored one). The model, effort, environment names and ceilings are recorded, not
  > applied: no port carries them yet (#829; they will join the `ServeSpec`). A failed step rolls back what the run made

- **Fact:** `reset --first` bypasses the hub's park and hold gate, and the operator does not use it on a parked or held pane (#828 was folded into #839, still open).
  **Source:** `docs/adr/ADR-0021.md:365-367`
  **Verbatim excerpt:**
  > part 3 the first message of `pane reset --first` is a real prompt through `HarnessPort::send_prompt`, outside the hub's
  > prompt path and so outside its park and hold gate (section 8, "The prompt and activity seam"): this wiring makes that path
  > live, and closing it is #828's. Until #828 lands, the operator does not use `reset --first` on a parked or held pane.

- **Fact:** `pane list` (whole registry) shows unmanaged panes in a second table, and lists the records with a warning when Herdr cannot be read; acting verbs refuse them with `pane-unmanaged`.
  **Source:** `crates/holler-cli/src/pane/list.rs:70-76`
  **Verbatim excerpt:**
  > ```
  > /// A list of the whole registry also shows the **unmanaged** panes: every pane of the Herdr
  > /// session that no record names (a Claude Code session set up by hand, say), in a second
  > /// table after a blank line with the columns PANE-ID, LABEL, POS, PROJECT, STATE (always
  > /// `unmanaged`) and IDLE (how long its screen has not changed, `-` when Herdr does not
  > /// report it). `unmanaged` is a visible state, not a hold: no verb that acts on a pane accepts
  > /// such a pane (`pane-unmanaged`, exit 3). If Herdr cannot be read, the registry's panes are
  > /// still listed and a warning line says the unmanaged ones are not.
  > ```

- **Fact:** `unmanaged-stalled` has no default threshold, is whole-fleet only, and cannot fire against Herdr 0.8.2, which reports no screen-change time.
  **Source:** `docs/adr/ADR-0021.md:977` and `docs/adr/ADR-0021.md:1003`
  **Verbatim excerpt:**
  > - **The time of the last screen change is whatever Herdr reports, and Herdr reports none.** Herdr 0.8.2 (protocol 20) has no
  >
  >   There is no default: the threshold is the operator's, and without it nothing is reported. Whole-fleet runs only, as
