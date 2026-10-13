# Evidence: #878 pane compact (facts in unchanged code the diff relies on)

## F, phase 5

- **Fact:** `check_idle` answers `Ok` for a record with no session of record, so the engine checks that case itself, before any port call.
  **Source:** `crates/holler-pane/src/tx_switch/plan.rs:255-262`
  **Verbatim excerpt:**
  > pub(crate) fn check_idle(
  >     ports: Ports<'_>,
  >     record: &Pane,
  >     at: &HarnessEndpoint,
  > ) -> Result<(), PaneError> {
  >     let Some(session) = &record.session_of_record else {
  >         return Ok(());
  >     };

- **Fact:** `resolve_pane_record` answers a pane with no record as `PaneNotFound`, which the verb re-asks as `unmanaged::missing` (#832's `pane-unmanaged`).
  **Source:** `crates/holler-cli/src/say_cmd.rs:425-427`
  **Verbatim excerpt:**
  > record.ok_or_else(|| PaneError::PaneNotFound {
  >     what: name.to_string(),
  > })

- **Fact:** `call_until` copies `timeouts.request` (5 s) into the `Call`, so the compaction's `Call` overrides `request` by struct update.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:336-343`
  **Verbatim excerpt:**
  > fn call_until(&self, port: u16, op: &'static str, deadline: Instant) -> Call {
  >     Call {
  >         port,
  >         op,
  >         deadline,
  >         request: self.config.timeouts.request,
  >     }
  > }

- **Fact:** `session_activity` builds its own ordinary `Call`, so the adapter's activity re-check is never given the compaction bound.
  **Source:** `crates/holler-adapter-opencode/src/prompt.rs:64`
  **Verbatim excerpt:**
  > let call = harness.call(port, OP_SESSION_ACTIVITY);

- **Fact:** the message-list reader reports a reply that is not a list with `unexpected_status`, quoting nothing of the body.
  **Source:** `crates/holler-adapter-opencode/src/prompt.rs:143`
  **Verbatim excerpt:**
  > _ => Err(call.unexpected_status(GET_MESSAGES, &reply, "a list of messages")),

- **Fact:** the existence check answers a 400 or 404 as `session-not-found`, before anything else is sent.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:637-641`
  **Verbatim excerpt:**
  > if matches!(reply.status, 400 | 404) {
  >     return Err(PaneError::SessionNotFound {
  >         what: id.to_owned(),
  >     });
  > }

- **Fact:** `activity_refusal` maps a busy session to `session-busy`, the code the fake and the adapter reuse.
  **Source:** `crates/holler-pane/src/tx_switch.rs:136`
  **Verbatim excerpt:**
  > Activity::Busy => Some((SESSION_BUSY, "is running a turn; try again once it ends")),
