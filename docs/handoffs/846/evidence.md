# Evidence: #846 (facts in unchanged code the fold relies on)

- **Fact:** `HostPort` is `Send + Sync`, so the shared `LinkedHost`'s host and link must be too.
  **Source:** `crates/holler-pane/src/ports.rs:371`
  **Verbatim excerpt:**
  > pub trait HostPort: Send + Sync {

- **Fact:** `FakeHarness::start_server` answers `None` for an argv it did not build, else the start's result (the refusal that `OnStart` propagates or swallows).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:234-237`
  **Verbatim excerpt:**
  > pub fn start_server(&self, argv: &Argv) -> Option<Result<u32, PaneError>> {
  >     let (name, port, agent) = described(argv)?;
  >     Some(self.lock().start(name, port, agent))
  > }

- **Fact:** `FakeHarnesses::start` tries the OpenCode fake then the Claude fake, and `stop` ends both fakes' processes of the pane and returns nothing (what `FakeHarnessesLink` calls).
  **Source:** `crates/holler-pane-testkit/src/harnesses.rs:46-57`
  **Verbatim excerpt:**
  > pub fn start(&self, argv: &Argv) -> Option<Result<u32, PaneError>> {
  >     self.opencode()
  >         .start_server(argv)
  >         .or_else(|| self.claude().start(argv))
  > }
  >
  > /// End every process of the pane `name` in both fakes, and no other pane's: its server is
  > /// killed (its sessions stay) and its Claude Code process stopped (its conversation stays).
  > pub fn stop(&self, name: &PaneName) {
  >     self.opencode().kill_servers_of(name);
  >     self.claude().stop(name);
  > }

- **Fact:** `pane_cli_process` builds only `verb_harness/parse.rs`, so the new `verb_harness/linked_host.rs` is not built there.
  **Source:** `crates/holler-cli/tests/pane_verbs/process/main.rs:27`
  **Verbatim excerpt:**
  > #[path = "../../verb_harness/parse.rs"]

- **Fact:** every target that builds `verb_harness` allows dead code crate-wide, so a ready-made link unused in one target needs no new `#[allow]`.
  **Source:** `crates/holler-cli/tests/pane_verbs/main.rs:5`, `crates/holler-cli/tests/profile_verbs/main.rs:5`, `crates/holler-cli/tests/pane_claude/main.rs:4`, `crates/holler-cli/tests/pane_integration/main.rs:5`
  **Verbatim excerpt:**
  > #![allow(dead_code)] // #670
  > #![allow(dead_code)] // #670
  > #![allow(dead_code)] // #831
  > #![allow(dead_code)] // #649

- **Fact:** the conformance result type the shared suite runner returns is `Result<(), Vec<CaseFailure>>`.
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:43`
  **Verbatim excerpt:**
  > pub type Conformance = Result<(), Vec<CaseFailure>>;
