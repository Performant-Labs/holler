# Evidence: #802 (F, Phase 6 of the Workflow script)

Behaviour in **unchanged** code that the diff or T's tests rely on. Line numbers are at the F
working tree (branch `issue-802-implementation`, after F's round-3 changes: the `verify_serving`
contract text after the diff gate's B-1, no line moved in `tx_launch.rs`).

- **Fact:** a rollback stops the tmux session's processes when the act's session step ran, so the
  server started by `host.run` (after `made.session = true`) is stopped by the rollback of a later
  failure (a `verify_serving` refusal, the health wait's `timeout`, the act's `port-in-use`, or the
  session of record's call to a server gone after the health wait).
  **Source:** `crates/holler-pane/src/tx_launch.rs:721-727`
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
  **Source:** `crates/holler-pane/src/tx_launch.rs:573-576`
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

- **Fact:** the fake harness logs every `health` call and answers `true` only for a port whose
  server is running, so a bare `FakeHarness::new()` answers `false` and its call log counts the
  asks the shared `wait_healthy` made (T's two direct tests of the wait, round 2).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:346-349`
  **Verbatim excerpt:**
  > ```rust
  >     fn health(&self, port: u16) -> Result<bool, PaneError> {
  >         self.faults.enter(HarnessOp::Health)?;
  >         Ok(self.lock().state(port) == Some(ServerState::Running))
  >     }
  > ```

- **Fact:** `set_delay` makes every later call of that fake sleep before it answers, which is how
  `no_health_poll_starts_once_the_budget_has_run_out` makes the host's `run` return after the
  budget has run out, with no sleep in test code.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:79-83`
  **Verbatim excerpt:**
  > ```rust
  >     /// Make every call sleep for `delay` before it answers, as a slow port does
  >     /// (`None`: no delay).
  >     pub fn set_delay(&self, delay: Option<Duration>) {
  >         self.lock().delay = delay;
  >     }
  > ```

- **Fact:** the engine's next call to the server after `verify_serving` is the session of record's:
  `list_sessions` for a relaunch that kept one, else `create_session`. Both are inside `bring_up`,
  whose every error goes through `roll_back` (the `.map_err` at `tx_launch.rs:607-608`, in the diff),
  so a server gone after the health wait fails that call and is rolled back as a `verify_serving`
  refusal is.
  **Source:** `crates/holler-pane/src/tx_launch.rs:676-684`
  **Verbatim excerpt:**
  > ```rust
  > fn session_of_record(ports: Ports<'_>, plan: &Plan<'_>) -> Result<String, PaneError> {
  >     if let Some(kept) = plan.old.and_then(|old| old.session_of_record.as_deref()) {
  >         let listed = ports.harness.list_sessions(plan.port)?;
  >         if listed.iter().any(|session| session == kept) {
  >             return Ok(kept.to_owned());
  >         }
  >     }
  >     ports.harness.create_session(plan.port)
  > }
  > ```

- **Fact:** OpenCode's `create_session` sends `POST /session` through `Call::send` (`create`, called
  first by `create_session` at `lib.rs:297-299`).
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:451-452`
  **Verbatim excerpt:**
  > ```rust
  > fn create(call: &Call) -> Result<String, PaneError> {
  >     let reply = call.send(CREATE, CREATE.label, Some(&json!({})))?;
  > ```

- **Fact:** every request OpenCode's `Call` sends answers a refused connection (nothing listens: the
  server is gone) as `unavailable`, and no reply in time (a wedged server) as `timeout`. So the call
  that follows a `verify_serving` that sent nothing fails on a dead server.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:409-415`
  **Verbatim excerpt:**
  > ```rust
  >         let within = budget(self.request, until.min(self.deadline));
  >         match http::request(self.port, route.method, path, body, within) {
  >             Ok(reply) => Ok(reply),
  >             Err(HttpError::Refused) => Err(PaneError::Unavailable {
  >                 what: format!("the harness server on port {}", self.port),
  >             }),
  >             Err(HttpError::TimedOut) => Err(self.timeout()),
  > ```

- **Fact (T, Phase 7 round 3):** the fake harness's `create_session` first reaches the server on
  the port, so after `kill(port)` it fails rather than minting a session. This is what
  `a_server_gone_after_verify_fails_the_session_of_record_and_rolls_back` relies on to stand in
  for a real server that died after the health wait.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:351-356`
  **Verbatim excerpt:**
  > ```rust
  >     fn create_session(&self, port: u16) -> Result<String, PaneError> {
  >         self.faults.enter(HarnessOp::CreateSession)?;
  >         let mut world = self.lock();
  >         world.reach(port, HarnessOp::CreateSession)?;
  >         Ok(world.mint_session(port))
  >     }
  > ```

- **Fact (T, Phase 7 round 3):** reaching a killed server is `unavailable`, the same answer
  OpenCode's `Call::send_by` gives a refused connection (`lib.rs:409-415`, above), so the test's
  expected code `unavailable` (exit 1) matches the real adapter's.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:424-430`
  **Verbatim excerpt:**
  > ```rust
  >     fn reach(&self, port: u16, op: HarnessOp) -> Result<(), PaneError> {
  >         match self.state(port) {
  >             Some(ServerState::Running) => Ok(()),
  >             Some(ServerState::Frozen) => Err(frozen(op)),
  >             Some(ServerState::Killed) | None => Err(unreachable_server(port)),
  >         }
  >     }
  > ```
