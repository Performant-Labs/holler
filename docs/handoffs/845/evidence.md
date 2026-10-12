# Evidence: #845 switch-after-relaunch

## F (Phase 5)

- **Fact:** a re-send through `Call::send_by` is bounded by the smaller of the `request` bound and what is left before the poll's own end (`until`) and the call's deadline, so a re-send cannot carry `select_session` past `settle`.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:520`
  **Verbatim excerpt:**
  > let within = budget(self.request, until.min(self.deadline));

- **Fact:** `budget` cuts a bound to what is left before a deadline, and to zero once it has passed.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:762-764`
  **Verbatim excerpt:**
  > fn budget(own: Duration, deadline: Instant) -> Duration {
  >     own.min(deadline.saturating_duration_since(Instant::now()))
  > }

- **Fact:** the crate's one poll runs `step` with its end (`settle` cut to the method's deadline), ends on the step's first `Some` or `Err`, and otherwise ends `timeout` when less than one `SETTLE_POLL` is left; so an `Err` from the re-send inside the step ends `select_session` at once.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:661-670`
  **Verbatim excerpt:**
  > let until = deadline_after(settle).min(deadline);
  > loop {
  >     if let Some(seen) = step(until)? {
  >         return Ok(seen);
  >     }
  >     if Instant::now() + SETTLE_POLL >= until {
  >         return Err(PaneError::Timeout { op: op.to_owned() });
  >     }
  >     std::thread::sleep(SETTLE_POLL);
  > }

- **Fact:** `switch` and `reset` reach the adapter through one `select_session` call in `tx_switch`, which is unchanged, so the re-send is inside that one port call.
  **Source:** `crates/holler-pane/src/tx_switch.rs:244`
  **Verbatim excerpt:**
  > .select_session(&record.herdr.pane_id, &target)

- **Fact:** doctor's `--fix` repair also calls the same `select_session`.
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:375`
  **Verbatim excerpt:**
  > let selected = ports.harness.select_session(pane_id, record);
