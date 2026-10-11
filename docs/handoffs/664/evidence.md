# Evidence: #664 `holler profile apply` (unchanged code the diff relies on)

Phase F, 2026-10-11. Each entry is behavior outside the diff that `tx_apply` or the verb depends on, quoted from the
branch at `e773945` (which is `origin/main` `263a2d7` plus this story's tests).

- **Fact:** `tx_launch::launch` refuses a name that already has a record (`pane-exists`) from one registry read, before
  any probe, Herdr or harness call. A `create` step whose record appeared after apply's re-read is answered by this
  check, so apply leaves it to launch and does not repeat it.
  **Source:** `crates/holler-pane/src/tx_launch.rs:182-185`
  **Verbatim excerpt:**
  > ```rust
  >     if ports.pane_store.get(name)?.is_some() {
  >         let message = format!("{name} already has a record; relaunch it instead");
  >         return Err(refused(PANE_EXISTS, message).into());
  >     }
  > ```

- **Fact:** `tx_launch::relaunch` takes the Herdr session from the record, so a `relaunch` step needs no
  `--herdr-session`. That is why §4's gate counts only spec panes with no record.
  **Source:** `crates/holler-pane/src/tx_launch.rs:208`
  **Verbatim excerpt:**
  > ```rust
  >     let session = old.herdr.session.clone();
  > ```

- **Fact:** a relaunch records the profile its request names, or else the old record's. A relaunch with
  `profile: Some(P)` therefore writes a record in P whatever profile the old record named (row f without a take-over).
  **Source:** `crates/holler-pane/src/tx_launch.rs:319`
  **Verbatim excerpt:**
  > ```rust
  >                 self.profile.or(old.profile.as_ref()).cloned(),
  > ```

- **Fact:** the test kit's profile scope answers the act's own error unchanged once it has put P's specs back. A
  `create` that runs out of budget therefore reaches apply as `timeout` with op `pane.launch` (entry 26's assertion).
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:167-171`
  **Verbatim excerpt:**
  > ```rust
  >         match self
  >             .profiles
  >             .cas_put(&restored, restored.generation, &self.actor)
  >         {
  >             Ok(_) => failure,
  > ```

- **Fact:** the registry's membership rule refuses a write that moves a pane from one profile to another, and allows
  leaving (`None`) and joining from `None`. So a take-over must leave and then join, two writes.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:225-239`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
  >     let current = stored.and_then(|pane| pane.profile.as_ref());
  >     match (current, submitted.profile.as_ref()) {
  >         (Some(current), Some(next)) if current.slug() != next.slug() => {
  >             Err(PaneError::PaneInOtherProfile {
  >                 what: format!(
  >                     "{} is in profile {:?}, not {:?}",
  >                     submitted.name,
  >                     current.as_str(),
  >                     next.as_str()
  >                 ),
  >             })
  >         }
  >         _ => Ok(()),
  >     }
  > ```

- **Fact:** `class_of` sorts every open code (a well-formed code that is not closed) as a refusal. Apply sorts each
  step's answer through it, so `orchestrator-not-relaunched`, `spec-not-launchable`, `pane-exists`, `grid-occupied` and
  `port-in-use` are `refused` steps, never `failed`.
  **Source:** `crates/holler-pane/src/error.rs:267-273`
  **Verbatim excerpt:**
  > ```rust
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```

- **Fact:** `diff_profile`'s rows begin with one row per spec, in P's order, before any `Extra` row. Apply zips them
  with P's specs, which pairs each spec with its own row.
  **Source:** `crates/holler-pane/src/profile_diff.rs:293-298`
  **Verbatim excerpt:**
  > ```rust
  >     let rows = profile.panes.iter().map(|spec| {
  >         match live.iter().find(|pane| pane.name.as_str() == spec.pane) {
  >             Some(pane) => compared(spec, pane),
  >             None => row(&spec.pane, PaneStatus::Missing),
  >         }
  >     });
  > ```

- **Fact:** the verb's exit code is the class of the printed error's code. A `failed` step must therefore carry a
  failure-class code to exit 1 (§5), which is why apply turns an observation error of another class into
  `unavailable`. The test kit's envelope checker also fails an envelope whose exit code disagrees with its code's class.
  **Source:** `crates/holler-cli/src/output.rs:338-340`; `crates/holler-pane-testkit/src/envelope.rs:33`
  **Verbatim excerpt:**
  > ```rust
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }
  > ```
  > ```rust
  > //! 13. The exit code matches the class of the code, `class_of(code).exit_code()`.
  > ```

- **Fact:** `findings::quoted` is the crate's excerpt quoting (Rust `{:?}` form, cut to 64 characters). Apply passes
  every spec pane, profile name, session and Herdr id through it before the text reaches a message.
  **Source:** `crates/holler-pane/src/findings.rs:332-334`
  **Verbatim excerpt:**
  > ```rust
  > pub fn quoted(text: &str) -> String {
  >     excerpt(text)
  > }
  > ```

## Added by T (Phase 7, 2026-10-11)

- **Fact:** Herdr's `version` is called by the launch transaction's own observation (steps 3 to 6, shared by launch
  and relaunch), never by apply's up-front observation, which calls only `snapshot`, `health` and `shown_session`. In
  `row_f_take_over_whose_relaunch_fails_stays_in_p` the only launch or relaunch is pane-f's relaunch (pane-z is an
  adopt), so `fail_next(HerdrOp::Version, ..)` fails that relaunch and nothing else.
  **Source:** `crates/holler-pane/src/tx_launch.rs:288-291`
  **Verbatim excerpt:**
  > ```rust
  >     fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
  >         let probe = probe(ports, self.spec, probe_timeout)?;
  >         let herdr_api_version = ports.herdr.version()?;
  >         let listed = ports.herdr.snapshot()?.panes;
  > ```

- **Fact:** the test kit's split-only Herdr refuses a free cell that no split reaches (`grid-unreachable`). So
  `row_b_missing_panes_are_created_by_splits_in_grid_order`, whose P lists its cells in reverse, passes only when apply
  creates in `(workspace, row, col)` order.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:85-88`
  **Verbatim excerpt:**
  > ```rust
  >     /// Only a cell that a split reaches, as in real Herdr (spike section 7): the root
  >     /// `r1c1` of an empty workspace, or a free cell right of a pane (a `right` split) or
  >     /// below one (a `down` split). Any other free cell is refused with
  >     /// [`GRID_UNREACHABLE`].
  > ```
