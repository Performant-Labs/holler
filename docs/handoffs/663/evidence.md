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

## The tests' fixture (added by T, Phase 7; excerpts copied by T from source at `3bdd129`)

- **Fact:** `FakeProfileStore::seeded` stores each profile at generation 1 and bypasses the faults and the call log, so
  the fixture of AC 2-7 starts at generation 1 with an empty call log, and AC 2's `[Get, CasPut, CasPut]` and AC 7's
  "generation 1, unchanged" are measured from there.
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:122-124`
  **Verbatim excerpt:**
  >     /// A store holding `profiles`, each created by `actor` at expected generation 0
  >     /// and so stored at 1 with one `Created` entry, in order. Seeding bypasses the
  >     /// faults and the call log. Two seeds with one name are `generation-conflict`, as

- **Fact:** `fail_next` fails only the next call of that method, once, so the fault AC 2 arms inside the act hits the
  restoring `cas_put` and nothing after it, and AC 4's first-write fault is spent by the first `cas_put`.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:72-74`
  **Verbatim excerpt:**
  >     /// Fail the next call of `op` with `error`, once. The errors queued for one method
  >     /// come out in the order they were queued, and a call of another method leaves them
  >     /// queued. A standing fault answers first, also leaving them queued.

- **Fact:** `concurrent_put` stores at the stored generation + 1 and bypasses the faults and the call log, so AC 3's act
  moves Demo Alpha past the generation the restoring write expects, and that write is the conflict.
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:156-159`
  **Verbatim excerpt:**
  >     /// Another writer stores `profile` unconditionally, at the stored generation + 1
  >     /// (or at 1 for a new profile), without the name rule, logs its entry with `actor`
  >     /// and publishes its event. It bypasses the faults and the call log. Returns the
  >     /// stored record.

## The reconcile step's builder (added by F, re-entry run; copied from the branch, which holds `0ad2d8a`'s #701)

- **Fact:** `doctor_command(None, false)` is exactly `holler pane doctor`: the one `const` spelling, with no pane and no
  `--fix`. So `reconcile_step(None)` is `to reconcile, run holler pane doctor`, and the profile form appends
  `--profile '<P>' and then holler profile show '<P>'` to the same line.
  **Source:** `crates/holler-pane/src/findings.rs:36, 306-316`
  **Verbatim excerpt:**
  > const DOCTOR: &str = "holler pane doctor";
  > pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String {
  >     let mut line = DOCTOR.to_owned();
  >     if let Some(pane) = pane {
  >         line.push(' ');
  >         line.push_str(pane.as_str());
  >     }
  >     if fix {
  >         line.push_str(" --fix");
  >     }
  >     line
  > }

- **Fact:** `findings` is a public module of `holler-pane`, so `holler-cli` can call `doctor_command`.
  **Source:** `crates/holler-pane/src/lib.rs:42`
  **Verbatim excerpt:**
  > pub mod findings;

- **Fact:** a pane-scoped doctor refuses a pane with no record. With `--profile P` the scope comes from
  `ProfileScope::resolve(P, Some(pane))`, which is `pane-not-in-profile` for such a pane (`StoreScope::member`, in the
  diff). Without `--profile` it is `pane-not-found`. This is why the step names no pane: after a failed `launch` of a new
  pane there is no record.
  **Source:** `crates/holler-pane/src/reconcile.rs:226-235`
  **Verbatim excerpt:**
  >     let scope = match (request.profile, request.pane) {
  >         (Some(profile), pane) => ports.scope.resolve(profile, pane)?.panes,
  >         (None, Some(name)) => {
  >             let pane = ports
  >                 .pane_store
  >                 .get(name)?
  >                 .ok_or_else(|| PaneError::PaneNotFound {
  >                     what: name.to_string(),
  >                 })?;
  >             vec![pane]

## The group kill on macOS (added by F, re-entry run; the architecture review's W-16)

These excerpts are copied from `origin/main` at `dc300ab` (#705, merged at 19:43 MDT on 2026-10-09). The files are not on
this branch, which is merged with `0ad2d8a`, so the gate may not be able to attach them from the branch. They are
evidence for `probe.rs`'s group kill on macOS, which the brief's Evidence H said nothing had yet run. That statement is
now stale. #705's `exec.rs` and `hermetic_test.rs` are byte-identical at its CI head `4155e06` and at `dc300ab` (blobs
`359ba09` and `ddd22ae` at both).

- **Fact:** the OpenCode adapter kills a process group with the same `kill -s KILL -- -<pgid>` form that `probe.rs`'s
  `kill_group` uses, through the `kill` program on `PATH`.
  **Source:** `crates/holler-adapter-opencode/src/exec.rs:32-34` (at `dc300ab`; `const KILL: &str = "kill";` is line 17)
  **Verbatim excerpt:**
  >     let group = format!("-{pgid}");
  >     let mut kill = Command::new(KILL);
  >     kill.args(["-s", "KILL", "--", group.as_str()]);

- **Fact:** the adapter starts its server in a process group of its own and, when it gives up, kills that group with
  `kill_group` before it kills and reaps the server itself, the order of `probe.rs`'s `kill_and_reap`.
  **Source:** `crates/holler-adapter-opencode/src/server.rs:106, 169-172` (at `dc300ab`)
  **Verbatim excerpt:**
  >         .process_group(0);
  > fn stop(child: &mut Child) {
  >     let _ = exec::kill_group(child.id(), OP_SERVE, KILL_BOUND);
  >     let _ = child.kill();
  >     let _ = child.wait();

- **Fact:** the test that exercises that form is not opt-in. Its server is a shell that records its own pid and starts a
  background `sleep 30`. After `serve` gives up, the test asserts that the shell's pid and the `sleep`'s pid are each
  gone within 2 s.
  **Source:** `crates/holler-adapter-opencode/tests/hermetic_test.rs:638-639, 645-646, 663-671` (at `dc300ab`)
  **Verbatim excerpt:**
  > #[test]
  > fn serve_kills_its_process_group_when_the_deadline_passes() {
  >     let scratch = Scratch::with_script("echo $$ > pid\nsleep 30 &\necho $! > child\nwait\n");
  >     let h = OpenCodeHarness::new(config("sh", scratch.path(), timeouts));
  >         let gone_by = Instant::now() + Duration::from_secs(2);
  >         while alive(pid) && Instant::now() < gone_by {
  >             std::thread::sleep(Duration::from_millis(20));
  >         }
  >         if alive(pid) {
  >             let _ = std::process::Command::new("kill")
  >                 .args(["-KILL", pid])
  >                 .status();
  >             panic!("the {file} process {pid} outlived serve's timeout");

- **Fact:** that test passed on CI's macOS runner: GitHub Actions run `38013074383`, job `114099775442`
  (`test (macos-latest)`, conclusion `success`, PR #705's head `4155e06`). The log line is from 19:27:00 MDT on 2026-10-09.
  **Source:** the job's log (`gh run view 38013074383 --repo Performant-Labs/holler --job 114099775442 --log`), line 641.
  This is not a repo file. The excerpt is the test output, without the log's own timestamp prefix.
  **Verbatim excerpt:**
  > test serve_kills_its_process_group_when_the_deadline_passes ... ok
