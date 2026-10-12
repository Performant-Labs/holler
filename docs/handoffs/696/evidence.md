# Evidence: #696 (unchanged source the diff relies on)

## F

- **Fact:** the OpenCode adapter's `one_line`, which the moved `cannot` and `Ran::reason` still call, turns every control character into a space; it is unchanged.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:748-753`
  **Verbatim excerpt:**
  > /// `text` on one line: every control character becomes a space (ADR-0021 section 9).
  > fn one_line(text: &str) -> String {
  >     text.chars()
  >         .map(|c| if c.is_control() { ' ' } else { c })
  >         .collect()
  > }

- **Fact:** the OpenCode call site still receives a `Command` by value from `tui::tmux_command` (unchanged), which the new private `capture` in `attach.rs` takes by value and lends to the runner as `&mut`.
  **Source:** `crates/holler-adapter-opencode/src/tui.rs:117-118`
  **Verbatim excerpt:**
  > pub fn tmux_command(tmux: &TmuxConfig, args: &[String]) -> Command {
  >     let mut command = Command::new(&tmux.tmux_bin);

- **Fact:** the host adapter maps `Failed` to `PaneError` in one place, unchanged, so keeping `Failed::{Expired, Unavailable(String)}` and its texts in `exec.rs` keeps every host error as it was.
  **Source:** `crates/holler-adapter-host/src/lib.rs:445-450`
  **Verbatim excerpt:**
  > fn failed(self, failed: Failed) -> PaneError {
  >     match failed {
  >         Failed::Expired => self.timeout(),
  >         Failed::Unavailable(what) => PaneError::Unavailable { what },
  >     }
  > }

- **Fact:** the host's tmux call site is unchanged and still passes `&mut Command` to `exec::run`, which now wraps the shared runner.
  **Source:** `crates/holler-adapter-host/src/lib.rs:246`
  **Verbatim excerpt:**
  > let ran = exec::run(&mut command, call.deadline).map_err(|failed| call.failed(failed))?;
