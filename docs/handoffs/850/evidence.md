# Evidence: #850 (facts in unchanged code that the change relies on)

Written by F (Phase 5). Each entry is a fact the diff or T's tests rely on that lives in code the diff does not change.

- **Fact:** The real adapter's `session_activity` of a process that no longer runs is `unavailable`, never an activity; the driver's poll classifies such a failed read with `locate_process` (a different holder is `cancelled`, none is `error`).
  **Source:** `crates/holler-adapter-claude/src/lib.rs:212-221` and `crates/holler-adapter-claude/src/lib.rs:324-330`
  **Verbatim excerpt:**
  > /// The entry of the process `at` names, which must run: `unavailable` otherwise.
  > fn running(&self, at: &HarnessEndpoint) -> Result<Entry, PaneError> {
  >     let (pane, pid) = at.process()?;
  >     self.entry(pid)?.ok_or_else(|| PaneError::Unavailable {
  > ...
  > fn session_activity(
  >     &self,
  >     at: &HarnessEndpoint,
  >     _session: &str,
  > ) -> Result<Activity, PaneError> {
  >     agents::activity(&self.running(at)?)
  > }

- **Fact:** The adapter's held-message watch does not run when the process was already waiting before the write, so a delivery to a process waiting on a dialog is reported delivered; the driver's next poll then reads the dialog as a failed read and ends the turn `error`.
  **Source:** `crates/holler-adapter-claude/src/lib.rs:227-230`
  **Verbatim excerpt:**
  > fn watch_held(&self, pane: &PaneName, pid: u32, before: &str) -> Result<(), PaneError> {
  >     if before == "waiting" {
  >         return Ok(());
  >     }

- **Fact:** A turn that ends `error` drops the session's driver, so the next prompt attaches again (the Claude row then finds whichever process holds the session now).
  **Source:** `crates/holler-body/src/session_manager/task.rs:745-749`
  **Verbatim excerpt:**
  > // Crash isolation (issue #189's own acceptance item): drop the driver so
  > // the *next* prompt respawns a fresh one, never retried automatically.
  > if matches!(reason, StopReason::Error) {
  >     inner.driver = None;
  > }

- **Fact:** A cancel the driver refuses reaches the wire as `-32004 not_connected` with the driver's message, so the Claude driver's refusal (naming `holler interrupt --pane NAME`) needs no wire change.
  **Source:** `crates/holler-body/src/prompt_dispatch.rs:159-161`
  **Verbatim excerpt:**
  > match session_manager.cancel(&name).await {
  >     Ok(Ok(())) => send_response(&outbound, &cid, &CancelResult { applied: true }),
  >     Ok(Err(msg)) => send_error(&outbound, &cid, Code::NotConnected, &msg),

- **Fact:** A `replace` whose cancel the driver refuses is answered with the refusal and runs no prompt.
  **Source:** `crates/holler-body/src/session_manager/task.rs:461-465`
  **Verbatim excerpt:**
  > match cancel_current(inner).await {
  >     Some(Err(e)) => {
  >         let _ = reply_tx.send(PromptOutcome::Error(e));
  >         return;
  >     }

- **Fact:** An answer refused with a message holding "unsupported elicitation" (the text of `DriverError::Unsupported`, which the Claude driver's `answer` returns) is `-32602 invalid_params` on the wire; the session task answers "nothing pending" itself, before the driver, unless the row is `input-required`.
  **Source:** `crates/holler-body/src/prompt_dispatch.rs:261-268`
  **Verbatim excerpt:**
  > fn send_answer_refusal(outbound: &mpsc::UnboundedSender<WsMessage>, cid: &CorrelationId, msg: &str) {
  >     if msg == "nothing pending to answer" {
  >         send_error_frame(outbound, cid, &WireError::nothing_pending());
  >     } else if msg.contains("did not resolve") || msg.contains("unsupported elicitation") {
  >         send_error(outbound, cid, Code::InvalidParams, msg);
  >     } else {
  >         send_error(outbound, cid, Code::NotConnected, msg);
  >     }

- **Fact:** A relaunch stores the record itself and keeps the stored DRIVEN, which is why the drive rule records DRIVEN by a second compare-and-swap after it.
  **Source:** `crates/holler-pane/src/tx_launch.rs:473-481`
  **Verbatim excerpt:**
  > /// The record R writes: what the act brought up and observed, and the spec's values. A
  > /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
  > /// profile, its profile.
  > fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
  >     let (hold, driven, profile) = match self.old {
  >         Some(old) => (
  >             old.hold.clone(),
  >             old.last_observed.driven.clone(),

- **Fact:** A pane name has a body session name's grammar, so the driver's `PaneName::parse` of the row's (already valid) name cannot refuse it.
  **Source:** `crates/holler-pane/src/pane.rs:35-42`
  **Verbatim excerpt:**
  > /// Parse a pane name; the grammar is `SessionName::parse`'s.
  > pub fn parse(text: &str) -> Result<Self, PaneError> {
  >     SessionName::parse(text)
  >         .map(Self)
