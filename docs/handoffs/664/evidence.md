# Evidence: #664 `holler profile apply` (unchanged code the diff relies on)

Phase F, 2026-10-11. Each entry is behavior outside the diff that `tx_apply` or the verb depends on. The first run quoted
the branch at `e773945`; the fresh run (after A round 4) re-cut every entry, T's two included, from the branch at
`d7c92de`, whose cited files are byte-identical to `origin/main` `7284001` (#802 merged; the branch changes none of
them). #802 moved several `tx_launch.rs` lines, so the line numbers below differ from the first run's.

- **Fact:** `tx_launch::launch` refuses a name that already has a record (`pane-exists`) from one registry read, before
  any probe, Herdr or harness call. A `create` step whose record appeared after apply's re-read is answered by this
  check, so apply leaves it to launch and does not repeat it.
  **Source:** `crates/holler-pane/src/tx_launch.rs:198-201`
  **Verbatim excerpt:**
  > ```rust
  >     if ports.pane_store.get(name)?.is_some() {
  >         let message = format!("{name} already has a record; relaunch it instead");
  >         return Err(refused(PANE_EXISTS, message).into());
  >     }
  > ```

- **Fact:** `tx_launch::relaunch` takes the Herdr session from the record, so a `relaunch` step needs no
  `--herdr-session`. That is why §4's gate counts only spec panes with no record.
  **Source:** `crates/holler-pane/src/tx_launch.rs:224`
  **Verbatim excerpt:**
  > ```rust
  >     let session = old.herdr.session.clone();
  > ```

- **Fact:** a relaunch records the profile its request names, or else the old record's. A relaunch with
  `profile: Some(P)` therefore writes a record in P whatever profile the old record named (row f without a take-over).
  **Source:** `crates/holler-pane/src/tx_launch.rs:369`
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
  step's answer through it, so `orchestrator-not-relaunched`, `spec-not-launchable`, `pane-exists`, `grid-occupied`,
  `port-in-use` and a harness's own `verify_serving` code (the test kit's `agent-mismatch`) are `refused` steps, never
  `failed`.
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

### Added by F in the fresh run (C25: the reconcile step on an acted step)

- **Fact:** a launch or relaunch that fails once `edit_spec` has entered its act answers a `TxFailure` whose `acted` is
  true: `run` sets `entered` before it calls the act, `act_live`, and maps `edit_spec`'s error with `acted: entered`.
  `acted` is what the CLI keys the reconcile step on. Apply copies it onto the step (`Halt::from(TxFailure)`, in the
  diff), for a `refused` step as for a `failed` one.
  **Source:** `crates/holler-pane/src/tx_launch.rs:167-174`; `crates/holler-pane/src/tx_launch.rs:568-580`
  **Verbatim excerpt:**
  > ```rust
  > /// Why a run failed, and whether it got as far as the act.
  > #[derive(Debug, Clone, PartialEq, Eq)]
  > pub struct TxFailure {
  >     pub error: PaneError,
  >     /// `edit_spec` entered the act, or a step after it failed: the CLI prints the reconcile
  >     /// step.
  >     pub acted: bool,
  > }
  > ```
  > ```rust
  >     let (mut entered, mut acted) = (false, None);
  >     let mut act = || {
  >         entered = true;
  >         acted = Some(act_live(ports, plan, &seen, budget, options)?);
  >         Ok(())
  >     };
  >     let edited = ports
  >         .scope
  >         .edit_spec(plan.profile, plan.name, &edit, &mut act);
  >     let profile = edited.map_err(|error| TxFailure {
  >         error,
  >         acted: entered,
  >     })?;
  > ```

- **Fact:** since #802 the act can answer a refusal after it has changed things live. `act_live` calls `bring_up`, which
  has ensured the Herdr pane and the tmux session and run the spec's command before it calls `server::start`; `start`
  re-checks the port (`port-in-use`), runs the server's argv, waits for its health and then returns the harness's
  `verify_serving` answer. Either refusal is an open code (a `refused` step) whose `TxFailure.acted` is true, which is
  why the verb adds the reconcile step to a `refused` step's detail, not only a `failed` one's.
  **Source:** `crates/holler-pane/src/tx_launch.rs:611`; `crates/holler-pane/src/tx_launch.rs:662`; `crates/holler-pane/src/tx_launch/server.rs:87-107`
  **Verbatim excerpt:**
  > ```rust
  >     let live = bring_up(ports, plan, seen, budget, options.boot_poll, &mut made)
  > ```
  > ```rust
  >     server::start(ports, plan, seen, budget, boot_poll)?;
  > ```
  > ```rust
  > pub(super) fn start(
  >     ports: Ports<'_>,
  >     plan: &Plan<'_>,
  >     seen: &Seen,
  >     budget: &Budget,
  >     poll: Duration,
  > ) -> Result<(), PaneError> {
  >     budget.check()?;
  >     if ports.harness.health(plan.port)? {
  >         return Err(port_in_use(plan.port));
  >     }
  >     budget.check()?;
  >     ports
  >         .host
  >         .run(plan.name, &plan.spec.host.cwd, &seen.server)?;
  >     // No budget check here: the wait checks the budget's end before every ask, its first included.
  >     if !wait_healthy(ports.harness, plan.port, poll, budget.deadline())? {
  >         return Err(budget.expired());
  >     }
  >     budget.check()?;
  >     ports.harness.verify_serving(plan.port, &plan.serve)
  > ```

- **Fact:** the reconcile step the verb appends is `reconcile_step(Some(P))`, one line naming the doctor run over P and
  `profile show` of P, with P single-quoted. The verb's guard compares the detail with this exact text.
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:49-56`
  **Verbatim excerpt:**
  > ```rust
  > pub fn reconcile_step(profile: Option<&ProfileName>) -> String {
  >     let doctor = doctor_command(None, false);
  >     let Some(profile) = profile else {
  >         return format!("to reconcile, run {doctor}");
  >     };
  >     let name = single_quoted(profile.as_str());
  >     format!("to reconcile, run {doctor} --profile {name} and then holler profile show {name}")
  > }
  > ```

- **Fact:** `pane launch` and `pane relaunch` add the reconcile step to an acted failure after `"; "`, and only when the
  message does not already hold it (the profile scope's own errors carry it). The verb's `reconciled` follows this form,
  applied to a step's detail.
  **Source:** `crates/holler-cli/src/pane/launch.rs:434-442`
  **Verbatim excerpt:**
  > ```rust
  > /// A failure's error body: its code and text, and the reconcile step when it reached the act.
  > fn failure_body(failure: TxFailure, profile: Option<&ProfileName>) -> ErrorBody {
  >     let mut body = ErrorBody::from(&failure.error);
  >     let step = reconcile_step(profile);
  >     if failure.acted && !body.message.contains(step.as_str()) {
  >         body.message = format!("{}; {step}", body.message);
  >     }
  >     body
  > }
  > ```

## Added by T (Phase 7, 2026-10-11)

*(Re-cut by F in the fresh run: #802 added the `serve_command` read at the head of `observe`, so the first entry's lines
moved from 288-291 to 340-346 and its excerpt now shows that read. The facts and their wording are T's, unchanged.)*

- **Fact:** Herdr's `version` is called by the launch transaction's own observation (steps 3 to 6, shared by launch
  and relaunch), never by apply's up-front observation, which calls only `snapshot`, `health` and `shown_session`. In
  `row_f_take_over_whose_relaunch_fails_stays_in_p` the only launch or relaunch is pane-f's relaunch (pane-z is an
  adopt), so `fail_next(HerdrOp::Version, ..)` fails that relaunch and nothing else.
  **Source:** `crates/holler-pane/src/tx_launch.rs:340-346`
  **Verbatim excerpt:**
  > ```rust
  >     fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
  >         let server = ports
  >             .harness
  >             .serve_command(self.name, self.port, &self.serve)?;
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

## Added by T (Phase 7, fresh run, 2026-10-11)

- **Fact:** a test-kit fake records each call in the fault switch's log *before* it sleeps the configured delay, and
  the sleep holds no lock, so other calls proceed meanwhile. So in `a_panes_health_and_shown_session_are_observed_at_once`
  (400 ms per harness call), a call made on another thread is already in `calls()` when the first call returns, but a
  call chained after it is not: the test's "the other call was made" check separates one-thread-per-call from a chain.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:85-103` (unchanged on this branch)
  **Verbatim excerpt:**
  > ```rust
  >     /// Every call made through the port, oldest first, the failed ones included.
  >     pub fn calls(&self) -> Vec<Op> {
  >         self.lock().calls.clone()
  >     }
  >
  >     /// What a fake calls first in every port method. It records the call, sleeps for
  >     /// the delay (without holding the lock, so other calls proceed), and then answers
  >     /// the standing fault if there is one, or else the oldest error queued for `op`.
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
