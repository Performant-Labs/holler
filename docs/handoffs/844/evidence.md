# Evidence: #844

- **Fact:** `tmux_command` has exactly one caller in the adapter, the private `run` of the attach module, so adding `-u`
  in `tmux_command` reaches every tmux process `attach_tui`, `select_session` and `shown_session` start.
  **Source:** `crates/holler-adapter-opencode/src/attach.rs:216-217`
  **Verbatim excerpt:**
  > fn run(&self, args: &[String], until: Instant) -> Result<Option<String>, PaneError> {
  >     let ran = exec::capture(tui::tmux_command(self.tmux, args), self.op, until)?;
