# Evidence: #802 (F, Phase 6 of the Workflow script)

Behaviour in **unchanged** code that the diff or T's tests rely on. Line numbers are at the F
working tree (branch `issue-802-implementation`, after F's changes).

- **Fact:** a rollback stops the tmux session's processes when the act's session step ran, so the
  server started by `host.run` (after `made.session = true`) is stopped by the rollback of a later
  failure (a `verify_serving` refusal, the health wait's `timeout`, the act's `port-in-use`).
  **Source:** `crates/holler-pane/src/tx_launch.rs:708-714`
  **Verbatim excerpt:**
  > ```rust
  > fn roll_back(ports: Ports<'_>, name: &PaneName, made: &Made, error: PaneError) -> PaneError {
  >     let mut failed = Vec::new();
  >     if made.session {
  >         if let Err(stop) = ports.host.stop_owned(name) {
  >             failed.push(format!("host.stop_owned ({})", stop.code()));
  >         }
  >     }
  > ```

- **Fact:** a failure is `acted: true` exactly when `edit_spec` entered the act; a refusal from the
  plan (`serve_command`'s `agent-unsupported`, before `edit_spec`) stays `acted: false`.
  **Source:** `crates/holler-pane/src/tx_launch.rs:560-563`
  **Verbatim excerpt:**
  > ```rust
  >     let profile = edited.map_err(|error| TxFailure {
  >         error,
  >         acted: entered,
  >     })?;
  > ```

- **Fact:** the CLI appends the reconcile step only to a failure that reached the act, so the plan's
  `agent-unsupported` has no step and the act's `port-in-use` re-check has one.
  **Source:** `crates/holler-cli/src/pane/launch.rs:435-442`
  **Verbatim excerpt:**
  > ```rust
  > fn failure_body(failure: TxFailure, profile: Option<&ProfileName>) -> ErrorBody {
  >     let mut body = ErrorBody::from(&failure.error);
  >     let step = reconcile_step(profile);
  >     if failure.acted && !body.message.contains(step.as_str()) {
  >         body.message = format!("{}; {step}", body.message);
  >     }
  >     body
  > }
  > ```

- **Fact:** any well-formed open code is a refusal (exit 3), so `agent-unsupported` and the fake's
  `agent-mismatch` need no entry in the closed table.
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

- **Fact:** a `HarnessInfo` with `pid: None` serialises with no `pid` key, and a stored record with
  one still reads (AC 9); no serde change was needed.
  **Source:** `crates/holler-pane/src/pane.rs:148-149`
  **Verbatim excerpt:**
  > ```rust
  >     #[serde(default, skip_serializing_if = "Option::is_none")]
  >     pub pid: Option<u32>,
  > ```

- **Fact:** the host adapter's `stop_owned` sends TERM to every owned group, waits one grace, sends
  KILL to every group not done, and waits out the bound: the "kill with extreme prejudice" path the
  server now goes through (unchanged; already proven by `stop_owned_escalates_to_kill`).
  **Source:** `crates/holler-adapter-host/src/lib.rs:360-372`
  **Verbatim excerpt:**
  > ```rust
  >     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
  >         let call = Call::start(OP_STOP_OWNED, self.timeout);
  >         let Some(first) = self.listing(name, call)? else {
  >             return Ok(());
  >         };
  >         for &pgid in &first.owned {
  >             self.signal(Signal::Term, pgid, call)?;
  >         }
  >         let grace = deadline_after(self.stop_grace).min(call.deadline);
  >         let survivors = self.settle(name, first.owned, grace, call)?;
  >         for &pgid in &survivors {
  >             self.signal(Signal::Kill, pgid, call)?;
  >         }
  > ```

- **Fact:** `run` refuses an `argv[0]` that contains `=` (`env` would read it as an assignment).
  The OpenCode server's argv starts with the literal `env`, so it passes; the env pairs of
  `ProcessEnv::Isolated` come after `-i`, never first.
  **Source:** `crates/holler-adapter-host/src/lib.rs:454-461`
  **Verbatim excerpt:**
  > ```rust
  > fn check_argv(argv: &Argv) -> Result<(), PaneError> {
  >     let message = match argv.as_slice().first() {
  >         None => "an empty argv has no program to run",
  >         Some(program) if program.contains('=') => {
  >             "argv[0] contains '=', which env would read as a variable assignment"
  >         }
  >         Some(_) => return Ok(()),
  >     };
  > ```

- **Fact:** every fake port call is logged before its fault is answered, so a call that fails (the
  injected `ServeCommand` refusal of AC 2) still appears in `faults().calls()`.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:93-98`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  >             state.delay
  >         };
  > ```
