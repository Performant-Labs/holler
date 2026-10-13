# Evidence: #831 adopt (F, Phase 5)

Facts in unchanged code that the diff and T's tests rely on.

- **Fact:** the real Claude Code adapter's `shown_session` ignores the Herdr pane id and matches the process by pid only, answering `None` for a pid `agents --json` does not list, so the adoption can walk every foreground pid (the shell's included) and count only the ones it knows.
  **Source:** `crates/holler-adapter-claude/src/lib.rs:297-300`
  **Verbatim excerpt:**
  > fn shown_session(&self, _: &PaneId, at: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
  >     let (_, pid) = at.process()?;
  >     Ok(self.entry(pid)?.map(|entry| entry.session))
  > }

- **Fact:** the harness router sends `shown_session` to the harness the endpoint names, so a `HarnessEndpoint::Process` reaches the Claude Code harness and never an OpenCode server.
  **Source:** `crates/holler-pane/src/ports/router.rs:106-112`
  **Verbatim excerpt:**
  > fn shown_session(
  >     &self,
  >     pane: &PaneId,
  >     at: &HarnessEndpoint,
  > ) -> Result<Option<String>, PaneError> {
  >     self.by(at).shown_session(pane, at)
  > }

- **Fact:** membership is the record's profile slug equal to the profile's slug; the undo's check that a failed record is this run's combines it with `Pane`'s equality.
  **Source:** `crates/holler-pane/src/profile_diff.rs:275-279`
  **Verbatim excerpt:**
  > pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
  >     pane.profile
  >         .as_ref()
  >         .is_some_and(|own| own.slug() == profile.slug())
  > }

- **Fact:** `Pane` derives `PartialEq`, so `Pane { generation: 0, ..landed } == *planned` compares every field.
  **Source:** `crates/holler-pane/src/pane.rs:418-420`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct Pane {

- **Fact:** the shared bounded runner spawns nothing once the deadline has passed, so the per-pane `ps` calls of `foreground_pids`, which share one deadline, stop at it and the call is `timeout` with no partial list.
  **Source:** `crates/holler-pane/src/bounded.rs:161-163`
  **Verbatim excerpt:**
  > pub fn run(command: &mut Command, spec: &Spec, deadline: Instant) -> Result<Ran, RunError> {
  >     if Instant::now() >= deadline {
  >         return Err(RunError::Expired);
