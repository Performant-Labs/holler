# Evidence: #853, facts in unchanged code that the change relies on

Run handoff directory `docs/handoffs/853`. F's entries first; T appends its own at T-green.

## F (implementation)

- **Fact:** the Herdr adapter's planner places a workspace's new column-1 cells (its new rows) before every other new
  cell, each group in `(row, col)` order; `cell_key`'s `(workspace, col != 1, row, col)` mirrors this partition.
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:98-100`
  **Verbatim excerpt:**
  > ```rust
  >     // A new cell in column 1 is a new row; the rest are new columns. Both stay in order.
  >     let (rows, cols): (Vec<GridPos>, Vec<GridPos>) =
  >         cells.into_iter().partition(|cell| cell.col == 1);
  > ```

- **Fact:** the planner refuses a new row below a last row that holds more than one pane (`grid-unreachable`, "place
  rows before columns"), which is why the old `(workspace, row, col)` order failed and why one limit stays.
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:180-191`
  **Verbatim excerpt:**
  > ```rust
  >         let width = self.width(last).unwrap_or(0);
  >         if width != 1 {
  >             return Err(refuse(
  >                 cell,
  >                 &format!(
  >                     "a row can be added only below a row of one pane, and row {last} has {} \
  >                      (a down split there nests inside one cell, which is not a grid); place \
  >                      rows before columns",
  >                     count(usize::from(width), "pane")
  >                 ),
  >             ));
  >         }
  > ```

- **Fact:** every `refused` step carries its error (`error: Some(..)`), and its code is refusal-class: a `usage`
  answer is re-coded `spec-not-launchable`, and only a failure-class code makes a step `failed`. So the first refused
  step's code gives exit 3 through `class_of`, and `refused()`'s `filter_map` on `step.error` drops no refused step.
  **Source:** `crates/holler-pane/src/tx_apply.rs:528-549`
  **Verbatim excerpt:**
  > ```rust
  >     fn halted(&self, action: Action, halt: Halt) -> Step {
  >         let error = match class_of(halt.error.code()) {
  >             ErrorClass::Usage => not_launchable(&halt.error),
  >             ErrorClass::Refusal | ErrorClass::Failure => halt.error,
  >         };
  >         let outcome = if class_of(error.code()) == ErrorClass::Failure {
  >             Outcome::Failed
  >         } else {
  >             Outcome::Refused
  >         };
  >         let detail = match halt.note {
  >             Some(note) => format!("{error}; {note}"),
  >             None => error.to_string(),
  >         };
  >         Step {
  >             code: Some(error.code().to_owned()),
  >             detail: Some(detail),
  >             error: Some(error),
  >             acted: halt.acted,
  >             ..self.step(action, outcome)
  >         }
  >     }
  > ```

- **Fact:** an open (non-closed) well-formed code, such as `grid-unreachable`, `orchestrator-not-relaunched`,
  `spec-not-launchable` or a harness's `agent-mismatch`, is a refusal (exit 3).
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

- **Fact:** the verb's exit code comes from the error body's code class alone (no table in the verb), and in text mode
  an error is one `error: <message>` line on stderr.
  **Source:** `crates/holler-cli/src/output.rs:281-283` and `crates/holler-cli/src/output.rs:338-340`
  **Verbatim excerpt:**
  > ```rust
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  > ```
  > ```rust
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }
  > ```

- **Fact:** a failure envelope carries no `data`, which is why a refused run prints only its one line (decision 31).
  **Source:** `crates/holler-cli/src/output.rs:174-181`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn failure(error: ErrorBody) -> Self {
  >         Self {
  >             schema_version: SCHEMA_VERSION,
  >             ok: false,
  >             data: None,
  >             error: Some(error),
  >         }
  >     }
  > ```

- **Fact:** before `refused()` reads them, every acted step's detail already ends with P's reconcile step, once
  (`reconciled` runs first in `run`), so `refused()` carries `detail` as it is and builds no reconcile step itself.
  **Source:** `crates/holler-cli/src/profile/apply.rs:107-121`
  **Verbatim excerpt:**
  > ```rust
  > fn reconciled(mut applied: Applied) -> Applied {
  >     let reconcile = reconcile_step(Some(&applied.profile));
  >     for step in applied.steps.iter_mut().filter(|step| step.acted) {
  >         let detail = step
  >             .detail
  >             .take()
  >             .or_else(|| step.error.as_ref().map(ToString::to_string));
  >         step.detail = Some(match detail {
  >             Some(detail) if detail.contains(&reconcile) => detail,
  >             Some(detail) => format!("{detail}; {reconcile}"),
  >             None => reconcile.clone(),
  >         });
  >     }
  >     applied
  > }
  > ```

- **Fact:** a create refused by `ensure_pane` (`grid-unreachable`) is an acted refusal: `entered` is set before the
  act runs, `acted` is `entered`, and the act's first port call is `ensure_pane`. So `order.rs`'s refused `pane-n`
  carries its detail in the one line.
  **Source:** `crates/holler-pane/src/tx_launch.rs:605-617` and `crates/holler-pane/src/tx_launch.rs:679-680`
  **Verbatim excerpt:**
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
  > ```rust
  >     budget.check()?;
  >     let herdr = ports.herdr.ensure_pane(&plan.target)?;
  > ```

- **Fact:** the fake checks the range first, then answers an occupied cell with its pane in either placement, and only
  then applies the split-only rule, so the tightened `check_split` sees only free, in-range cells.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:442-451`
  **Verbatim excerpt:**
  > ```rust
  >     fn place(&mut self, grid: GridPos, placement: Placement) -> Result<PaneId, PaneError> {
  >         self.check_range(grid)?;
  >         if let Some(occupant) = self.occupant(grid) {
  >             return Ok(occupant.clone());
  >         }
  >         if placement == Placement::SplitOnly {
  >             self.check_split(grid)?;
  >         }
  >         self.mint(grid)
  >     }
  > ```

- **Fact:** the `HerdrPort` conformance suite places only `r1c1` and then `r2c1` (a 2-by-1 workspace), so its
  split-only run still passes on the tightened fake.
  **Source:** `crates/holler-pane-testkit/src/conformance/herdr.rs:24-27`
  **Verbatim excerpt:**
  > ```rust
  > //! workspace are the fixture's. Every case ensures `r1c1` before any other cell and
  > //! never closes a workspace's last pane, so the suite holds against a Herdr that places
  > //! a pane only by splitting an existing one (the fake's split-only placement) and
  > //! against real Herdr, which creates a workspace with its root pane. After a close, a
  > ```

## T (verify, GREEN)

Facts in unchanged code that T's tests rely on and F's entries do not cover. Excerpts copied by T from source.

- **Fact:** the steps are sorted with the stable `sort_by` on `cell_key`, over the specs in P's order, so two specs of
  one cell keep P's order (`order.rs`'s `dup-b` before `dup-a`).
  **Source:** `crates/holler-pane/src/tx_apply.rs:349`
  **Verbatim excerpt:**
  > ```rust
  >     chosen.sort_by(|(a, _), (b, _)| cell_key(a).cmp(&cell_key(b)));
  > ```

- **Fact:** a relaunch after a `--take-over` whose act fails or refuses puts the take-over note in the step's detail,
  which `refused.rs`'s `an_acted_refusal_after_a_take_over_carries_its_note` reads as `joined "<P>" before the
  relaunch`.
  **Source:** `crates/holler-pane/src/tx_apply/run.rs:123-129`
  **Verbatim excerpt:**
  > ```rust
  >     if let Err(failure) = tx_launch::relaunch(cx.ports, &request, cx.options) {
  >         let note = moved.map(|from| {
  >             let (from, to) = (quoted(from.as_str()), quoted(profile.as_str()));
  >             format!(
  >                 "{name} left profile {from} and joined {to} before the relaunch, and stays in {to}"
  >             )
  >         });
  > ```
