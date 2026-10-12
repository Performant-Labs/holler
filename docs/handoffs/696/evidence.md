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

## T (green)

- **Fact:** `seams_test.rs` relies on `login_status` mapping `authMethod` `claude.ai` to `LoginStatus::Ok` and a non-JSON answer to `Unavailable`; both are unchanged adapter logic.
  **Source:** `crates/holler-adapter-claude/src/lib.rs:71` and `:348-358`
  **Verbatim excerpt:**
  > const SUBSCRIPTION: &str = "claude.ai";
  > Some(SUBSCRIPTION) => Ok(LoginStatus::Ok),

- **Fact:** `uncapped_output_test.rs` relies on the host's refusal text being `tmux <subcommand>: <reason>`, with no length limit, and on `ps` returning that refusal as `unavailable` for a non-missing-session stderr.
  **Source:** `crates/holler-adapter-host/src/lib.rs:251-252` and `:401`
  **Verbatim excerpt:**
  > let what = format!("tmux {subcommand}: {}", ran.reason());
  > Reply::Refused(refusal, what) => Err(refused(name, refusal, what)),

- **Fact:** `tmux_capture_test.rs` relies on the OpenCode adapter quoting `tmux`'s first stderr line, whole, in `unavailable` (no length limit, only `one_line`).
  **Source:** `crates/holler-adapter-opencode/src/attach.rs:230-232` and `crates/holler-adapter-opencode/src/lib.rs:748-753`
  **Verbatim excerpt:**
  > what: format!("tmux failed for pane {}: {}", self.pane, ran.reason()),
  > fn one_line(text: &str) -> String {
