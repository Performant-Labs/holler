# Evidence: #663 (behaviour in unchanged code that the diff relies on)

Written by F (Phase 5). Every excerpt is copied from the tree at `3bdd129` (none of these files is in the diff).

## The scope (`crates/holler-cli/src/pane/profile_scope.rs`)

- **Fact:** `PaneError` has 23 variants: 18 carry exactly one string payload (`message`, `what` or `op`; `Refused` has
  `message` beside its code) and 5 carry none. `with_context` names the same set, and the compiler checks that it is
  exhaustive, since it has no catch-all arm.
  **Source:** `crates/holler-pane/src/error.rs:535-561`
  **Verbatim excerpt:**
  >     pub(crate) fn detail(&self) -> Option<&str> {
  >         match self {
  >             PaneError::NotImplemented
  >             | PaneError::CommandNotArgv
  >             | PaneError::EnvNameInvalid
  >             | PaneError::Conflict
  >             | PaneError::ProfileSecretRefused
  >             | PaneError::Refused { .. } => None,
  >             PaneError::Usage { message }
  >             | PaneError::ProbeFailed { message }
  >             | PaneError::HerdrVersionUnsupported { message }
  >             | PaneError::ProfileDrift { message } => Some(message),
  >             PaneError::GridAmbiguous { what }
  >             | PaneError::GridOutOfRange { what }
  >             | PaneError::ProfileConflict { what }
  >             | PaneError::ProfileNotFound { what }
  >             | PaneError::ProfileExists { what }
  >             | PaneError::ProfileHasLivePanes { what }
  >             | PaneError::PaneNotInProfile { what }
  >             | PaneError::PaneInOtherProfile { what }
  >             | PaneError::PaneNotFound { what }
  >             | PaneError::SessionNotFound { what }
  >             | PaneError::StoreCorrupt { what }
  >             | PaneError::Unavailable { what } => Some(what),
  >             PaneError::Timeout { op } => Some(op),
  >         }
  >     }

- **Fact:** `Refused`'s `message` is a public field of the variant, so `with_context` can extend it in place and keep
  the code (`detail()` above returns `None` for `Refused`, so its message is not reachable through `detail()`).
  **Source:** `crates/holler-pane/src/error.rs:491`
  **Verbatim excerpt:**
  >     Refused { code: RefusalCode, message: String },

- **Fact:** the `Display` of the errors the scope extends prints the payload after a fixed prefix, so a context
  appended to the payload shows in `to_string()`, and the act's error of AC 2 and AC 3 renders as `unavailable: act`.
  **Source:** `crates/holler-pane/src/error.rs:673-677`
  **Verbatim excerpt:**
  >             PaneError::Timeout { op } => write!(f, "timed out: {op}"),
  >             PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
  >             PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
  >             PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
  >             PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),

- **Fact:** the restore conflict's `profile-conflict` prints its `what` after a fixed prefix.
  **Source:** `crates/holler-pane/src/error.rs:655`
  **Verbatim excerpt:**
  >             PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),

- **Fact:** every code the scope answers after a profile write (`generation-conflict`, `profile-conflict`, `timeout`,
  `unavailable`, `store-corrupt`) is a failure (exit 1), so keeping the restoring write's own code (Decision 5) never
  turns a failure into a refusal.
  **Source:** `crates/holler-pane/src/error.rs:295-301`
  **Verbatim excerpt:**
  >         PaneCode::GenerationConflict
  >         | PaneCode::ProfileConflict
  >         | PaneCode::Timeout
  >         | PaneCode::Unavailable
  >         | PaneCode::StoreCorrupt
  >         | PaneCode::NotImplemented
  >         | PaneCode::ProfileDrift => ErrorClass::Failure,

- **Fact:** `ProfileStore::cas_put` returns the stored record with its bumped generation, so the restoring write's
  expected generation is the first write's returned `generation` (g + 1).
  **Source:** `crates/holler-pane/src/profile.rs:335-342`
  **Verbatim excerpt:**
  >     /// Store `profile` if the stored one is still at `expected_generation` (0 for a
  >     /// new profile); returns the stored record with its bumped generation.
  >     fn cas_put(
  >         &self,
  >         profile: &Profile,
  >         expected_generation: u64,
  >         actor: &Actor,
  >     ) -> Result<Profile, PaneError>;

- **Fact:** a profile name holds no control character (so no `\n` or `\r`), so the single-quoted reconcile step is one
  line.
  **Source:** `crates/holler-pane/src/profile.rs:46-49`
  **Verbatim excerpt:**
  >         } else if name.chars().count() > MAX_NAME_CHARS {
  >             Some("a profile name is at most 64 characters")
  >         } else if name.chars().any(char::is_control) {
  >             Some("a profile name must not contain control characters")

- **Fact:** the fake profile store files and finds a profile by its slug, so the stored record carries the stored
  spelling of the name, which is the one the scope's messages and reconcile step print.
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:250-253`
  **Verbatim excerpt:**
  >     fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
  >         self.faults.enter(ProfileStoreOp::Get)?;
  >         Ok(self.feed.read(|log| log.get(&name.slug()).cloned()))
  >     }

- **Fact:** the fakes' call log records every call, the failed ones included, so AC 2's `[Get, CasPut, CasPut]` counts
  the restoring write that the injected fault failed.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:85-87`
  **Verbatim excerpt:**
  >     /// Every call made through the port, oldest first, the failed ones included.
  >     pub fn calls(&self) -> Vec<Op> {
  >         self.lock().calls.clone()

## The probe runner (`crates/holler-pane/src/probe.rs`)

- **Fact:** the real `Prober` calls the free `run_probe`, so every caller that holds a `Prober` reaches the new runner
  with no other change.
  **Source:** `crates/holler-pane/src/ports.rs:218-222`
  **Verbatim excerpt:**
  > impl Prober for SystemProber {
  >     fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
  >         run_probe(argv, expect, timeout)
  >     }
  > }
