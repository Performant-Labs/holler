# Evidence: #645a switch-reset (F, Phase 5 of the pipeline / Phase 6 of the Workflow script)

Facts in **unchanged** code that the diff (`tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs`, the ADR-0021 edits) or T's
tests rely on. Each excerpt is copied from the tree at `a912c4b` plus F's uncommitted change (none of these files is in
F's change). Round 2 merged `origin/main` (`d9eabbb`), which changed none of the source files quoted here, so every line
number still held (`git diff origin/main` on them was empty). Round 4 merged `origin/main` (`abdcbb6`), whose #640 part 3
added three doc-comment lines to `crates/holler-pane/src/error.rs` (now lines 415-417 and 457-458) and changed no excerpt's text. The
`error.rs` citations from line 651 on moved down by three and are updated. Every excerpt below was checked against its cited
lines on the merged tree. Round 5 merges nothing: since `abdcbb6`, `origin/main` has moved only by #715 (`cec1f82`), which
changes `docs/handoffs/0660-output/decisions.md` alone, and so none of the files quoted here. Every excerpt was checked again.
Round 6 merges `origin/main` (`bd5e825`, #642 part 2), which changed no excerpt's text but moved four citations: ADR-0021
by 20 lines (its section 2 gained "`HarnessPort` as built (#642)"), `crates/holler-pane/src/ports.rs` from line 175 on by
one (a doc line of `HarnessPort`'s), and `crates/holler-adapter-opencode/tests/hermetic_test.rs` by one. They are updated,
and round 5's `select_session` entry now cites the adapter's built `select_session` (`attach.rs`) in place of the spike's
plan for it. Every excerpt below was checked against its cited lines on the merged tree.

## What the engine calls in `holler-pane`

- **Fact:** the one remedy table maps `ServerDown` to the relaunch command for a named pane, whatever the `FixState`, so
  P3's `FindingKind::ServerDown.remedy(Some(&pane), FixState::NotFixable)` is `Some("holler pane relaunch <pane>")` and
  its `doctor_command` fallback is never taken.
  **Source:** `crates/holler-pane/src/findings.rs:127-131` and `:319-320`
  **Verbatim excerpt:**
  > ```
  >             FindingKind::HerdrPaneMissing
  >             | FindingKind::TmuxSessionMissing
  >             | FindingKind::ServerWedged
  >             | FindingKind::ServerDown
  >             | FindingKind::TuiForeignSession => pane.map(relaunch_command),
  > ```
  > ```
  > fn relaunch_command(pane: &PaneName) -> String {
  >     format!("{RELAUNCH} {pane}")
  > ```

- **Fact:** `doctor_command(Some(pane), true)` is `holler pane doctor <pane> --fix`, the reconcile step
  `SwitchFailure::message` appends after `; to reconcile, run `.
  **Source:** `crates/holler-pane/src/findings.rs:306-316`
  **Verbatim excerpt:**
  > ```
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
  > ```

- **Fact:** `findings::quoted` is `error::excerpt`: the text `{:?}`-quoted (so every control character is escaped) and cut
  at 64 characters with `...`. So a typed session id (at most `SESSION_ID_MAX = 64`) is never cut, and the escape case of
  AC 12 prints `\u{1b}`, never a raw ESC.
  **Source:** `crates/holler-pane/src/findings.rs:332-334` and `crates/holler-pane/src/error.rs:691-698`
  **Verbatim excerpt:**
  > ```
  > pub fn quoted(text: &str) -> String {
  >     excerpt(text)
  > }
  > ```
  > ```
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }
  > ```

- **Fact:** `shown_differs(Some(target), shown)` is true for any screen but the target, the home screen (`None`) included.
  **Source:** `crates/holler-pane/src/reconcile.rs:179-181`
  **Verbatim excerpt:**
  > ```
  > pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
  >     session_of_record.is_some_and(|record| shown != Some(record))
  > }
  > ```

- **Fact:** the private `screen_text` that `tx_switch.rs` copies word for word (reconcile declares `observe` private, so
  it cannot be called from `tx_switch.rs`).
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:342-348`
  **Verbatim excerpt:**
  > ```
  > /// How a message names what the TUI shows.
  > fn screen_text(shown: Option<&str>) -> String {
  >     shown.map_or_else(
  >         || "its home screen".to_owned(),
  >         |session| format!("session {}", quoted(session)),
  >     )
  > }
  > ```

- **Fact:** the exit class of every code the verbs raise: an open code (the three `RefusalCode`s) is a refusal (3);
  `pane-not-found`, `session-not-found`, `pane-not-in-profile` and `profile-not-found` are refusals (3);
  `generation-conflict`, `timeout` and `unavailable` are failures (1). This is why a mismatch after the act must be the
  closed `unavailable`, not an open code (Decision 9).
  **Source:** `crates/holler-pane/src/error.rs:266-273`, `:285`, `:289-291`, `:295-298`
  **Verbatim excerpt:**
  > ```
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```
  > ```
  >         | PaneCode::PaneNotInProfile
  > ```
  > ```
  >         | PaneCode::ProfileNotFound
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  > ```
  > ```
  >         PaneCode::GenerationConflict
  >         | PaneCode::ProfileConflict
  >         | PaneCode::Timeout
  >         | PaneCode::Unavailable
  > ```

- **Fact:** how the errors the engine returns display, which is the start of every failure message: `Refused` is its
  message alone, `Unavailable` is prefixed `unavailable: `, and a record conflict has its own fixed text.
  **Source:** `crates/holler-pane/src/error.rs:654-656`, `:676-682`
  **Verbatim excerpt:**
  > ```
  >             PaneError::Conflict => f.write_str(
  >                 "the record changed since it was read (generation conflict); read it again and retry",
  >             ),
  > ```
  > ```
  >             PaneError::Timeout { op } => write!(f, "timed out: {op}"),
  >             PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
  >             PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
  >             PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
  >             PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
  >             PaneError::ProfileDrift { message } => write!(f, "profile drift: {message}"),
  >             PaneError::Refused { message, .. } => f.write_str(message),
  > ```

## The ports' contracts the engine relies on

- **Fact:** `cas_put` returns the record as stored, with its bumped generation, which is what `Switched.pane` (and so
  JSON `data.pane`) carries.
  **Source:** `crates/holler-pane/src/ports.rs:69-71`
  **Verbatim excerpt:**
  > ```
  >     /// Store `pane` if the stored one is still at `expected_generation` (0 for a
  >     /// new pane); returns the stored record with its bumped generation.
  >     fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
  > ```

- **Fact:** `resolve(P, Some(n))` answers just the pane `n`, which must belong to P; so P1's `find` by name picks that
  pane, and differs from `into_iter().next()` only for a scope that breaks its contract.
  **Source:** `crates/holler-pane/src/profile.rs:381-389`
  **Verbatim excerpt:**
  > ```
  >     /// The profile and the panes of it a verb acts on. With no `pane`, every pane
  >     /// of the profile; with a named pane, just that one, which must belong to the
  >     /// profile (`pane-not-in-profile` otherwise). A missing profile is
  >     /// `profile-not-found`.
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError>;
  > ```

- **Fact:** the hub registry's compare-and-swap checks only the generation and the membership rule, nothing about
  `session_of_record`, which is why the ADR-0021 paragraph says P5 (`session-of-other-pane`) is a read in the verb and
  not the authority.
  **Source:** `crates/holler-hub/src/panes/store.rs:170-175`
  **Verbatim excerpt:**
  > ```
  >     pub(crate) fn cas_put(&self, pane: &Pane, expected: u64) -> Result<Pane, PaneError> {
  >         let mut guard = self.lock();
  >         let table = guard.as_mut().map_err(|err| err.clone())?;
  >         let current = table.record(&pane.name);
  >         let generation = next_generation(current.map_or(0, |stored| stored.generation), expected)?;
  >         refuse_profile_move(current, pane)?;
  > ```

- **Fact:** doctor's remedy for a pane with no session of record, or a missing one, is `holler pane reset <pane>`, which
  this story makes a working verb (AC 15).
  **Source:** `crates/holler-pane/src/findings.rs:132-134`
  **Verbatim excerpt:**
  > ```
  >             FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
  >                 pane.map(reset_command)
  >             }
  > ```

## The CLI's output, which `emit_outcome` and `execute` go through

- **Fact:** text mode writes a failure as `error: <message>` on `err`; JSON mode puts the message on one line and takes
  the exit code from the code's class, the same in both modes.
  **Source:** `crates/holler-cli/src/output.rs:281-284`, `:288-297`
  **Verbatim excerpt:**
  > ```
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  >         }
  > ```
  > ```
  > fn emit_json<T: Serialize>(sink: &mut Sink<'_>, result: Result<T, ErrorBody>) -> i32 {
  >     match result {
  >         Ok(data) => settle(write_envelope(sink, &Envelope::success(data)), 0),
  >         Err(error) => {
  >             let code = exit_code(&error);
  >             let error = ErrorBody {
  >                 message: one_line(&error.message),
  >                 ..error
  >             };
  >             settle(write_envelope(sink, &Envelope::<()>::failure(error)), code)
  > ```

- **Fact:** until #649 the real binary's ports answer `not-implemented`, so the CHANGELOG's "the real verbs answer
  `not-implemented`" holds (checked by running the built binary: exit 1 `not-implemented`, and exit 2 `usage` for a bad
  argument, before any port).
  **Source:** `crates/holler-cli/src/pane/wiring.rs:8-9`
  **Verbatim excerpt:**
  > ```
  > //! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
  > //! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
  > ```

## The test kit's fakes, which T's ACs rely on

- **Fact:** the fake's `health` is true only for a running server, so a killed or frozen one makes P3 refuse with
  `server-unhealthy` after exactly one `Health` call (AC 5, AC 17).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:287-290`
  **Verbatim excerpt:**
  > ```
  >     fn health(&self, port: u16) -> Result<bool, PaneError> {
  >         self.faults.enter(HarnessOp::Health)?;
  >         Ok(self.lock().state(port) == Some(ServerState::Running))
  >     }
  > ```

- **Fact:** with `SelectAckedWithoutTui` on and no TUI, `select_session` answers `Ok(())` and changes nothing, so only
  O1's observation catches the mismatch (ACs 7 and 19).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:336-341`
  **Verbatim excerpt:**
  > ```
  >     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
  >         self.faults.enter(HarnessOp::SelectSession)?;
  >         let mut world = self.lock();
  >         if !world.tuis.contains_key(pane) && world.quirks.contains(&Quirk::SelectAckedWithoutTui) {
  >             return Ok(());
  >         }
  > ```

## Added by T (Phase 7, GREEN)

- **Fact:** every port of the fake harness shares one data directory unless a seed gives it its own, and `Seed::new`
  gives none; so P's server lists Q's session, and only P5 (not P4) can refuse `switch P <Q's session>` (AC 4).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:104-108`, `crates/holler-cli/tests/pane_verbs/doctor/rig.rs:61`
  **Verbatim excerpt:**
  > ```
  > /// It holds the servers by port, a data directory per port, the sessions of each data
  > /// directory in creation order, the TUI of each pane, the quirks that are on and the
  > /// aborts the servers acknowledged. Every port shares the data directory `"default"`,
  > /// as the live fleet's servers share one (opencode-pane-spike.md:75-77), until
  > /// [`FakeHarness::set_data_dir`] gives a port its own.
  > ```
  > ```
  >     /// An agent pane at `r<row>c<col>`, in no profile, on the shared data directory.
  > ```

- **Fact:** `concurrent_put` stores the other writer's record at the stored generation + 1, outside the call log, so the
  engine's `cas_put` at the generation it read meets `generation-conflict`, and the stored record is the other writer's
  at `before + 1` (AC 9).
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:113-118`
  **Verbatim excerpt:**
  > ```
  >     /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
  >     /// at 1 for a new record), without the membership rule, and publishes its event.
  >     /// It bypasses the faults and the call log. Returns the stored record.
  >     pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
  >         self.put(pane, Writer::Other)
  >     }
  > ```

## Added by F (round 2, after the anti-duplication BLOCK)

These are the facts the four new sentences of the "Switch and reset as built (#645)" paragraph rely on. The paragraph is
at `docs/adr/ADR-0021.md:365-390` on the tree with `origin/main` (`bd5e825`) merged (`:345-370` when round 2 merged
`d9eabbb`). The ADR lines quoted below are #663's text on `origin/main`. This change does not edit them, so the diff does
not show them.

- **Fact:** on `origin/main`, ADR-0021's generations rule says a verb whose record write meets `generation-conflict` after
  the act prints one of the two forms of step 6, which name no pane. The #645 paragraph says how switch and reset differ
  from this, and why.
  **Source:** `docs/adr/ADR-0021.md:314-319`
  **Verbatim excerpt:**
  > ```
  > - A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  >   between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  >   writes nothing more, and prints the reconcile step (the pane doctor command line; the profile-scoped or the bare form of
  >   step 6, which names no pane, for the reason given there). With `--profile` the record step runs inside the act, so a
  >   record conflict also restores P's specs (step 5) before the verb prints the reconcile step (#663, confirming the
  >   assumption of #638).
  > ```

- **Fact:** step 6 gives the two forms, and it names no pane because a pane-scoped doctor refuses a pane with no record or
  outside P, which is what a failed launch of a new pane leaves. Its `profile show` part reports a spec with no live pane.
  **Source:** `docs/adr/ADR-0021.md:351-357`
  **Verbatim excerpt:**
  > ```
  >    The reconcile step is exactly `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`,
  >    with P POSIX-single-quoted, and without `--profile` it is `to reconcile, run holler pane doctor`. `reconcile_step` in
  >    `holler-cli/src/pane/profile_scope.rs` builds both (`reconcile_step(None)` is the second) on
  >    `holler_pane::findings::doctor_command(None, false)`, the doctor command line's one builder (#701). The step names no
  >    pane on purpose: a pane-scoped doctor refuses a pane with no record or outside P (`pane-not-found` or
  >    `pane-not-in-profile`, in `holler-pane/src/reconcile.rs`), which is what a failed `launch` of a new pane leaves, so the
  >    step runs the doctor over every pane of P (or every pane) and `profile show` reports the spec that has no live pane.
  > ```

- **Fact:** the step switch and reset print, `holler pane doctor <pane> --fix`, has no `--profile`. A doctor run named that
  way reads the pane's record by name. It refuses only a pane with no record (`pane-not-found`), never one outside a
  profile. Switch and reset read that record in their plan, so the reason step 6 gives does not apply to them.
  **Source:** `crates/holler-pane/src/reconcile.rs:226-236`
  **Verbatim excerpt:**
  > ```
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
  >         }
  > ```

- **Fact:** `holler pane doctor <pane> --fix` is the pane doctor's own remedy for a pane whose TUI does not show its session
  of record, while a fix can repair it. That is the paragraph's "the pane doctor's own remedy".
  **Source:** `crates/holler-pane/src/findings.rs:135-141`
  **Verbatim excerpt:**
  > ```
  >             FindingKind::ShownDrivenMismatch => match fix {
  >                 FixState::Fixable | FixState::Skipped => {
  >                     pane.map(|pane| doctor_command(Some(pane), true))
  >                 }
  >                 FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
  >                 FixState::Fixed => None,
  >             },
  > ```

## Added by T (Phase 7, GREEN, round 2)

- **Fact:** a fault queued with `fail_next(HarnessOp::CreateSession, ..)` fails the fake's `create_session` before any
  session is minted, so `reset_create_failure_changes_nothing` sees a failed create that left no session behind.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:292-297`, `crates/holler-pane-testkit/src/fault.rs:72-77`
  **Verbatim excerpt:**
  > ```
  >     fn create_session(&self, port: u16) -> Result<String, PaneError> {
  >         self.faults.enter(HarnessOp::CreateSession)?;
  >         let mut world = self.lock();
  >         world.reach(port, HarnessOp::CreateSession)?;
  >         Ok(world.mint_session(port))
  >     }
  > ```
  > ```
  >     /// Fail the next call of `op` with `error`, once. The errors queued for one method
  >     /// come out in the order they were queued, and a call of another method leaves them
  >     /// queued. A standing fault answers first, also leaving them queued.
  >     pub fn fail_next(&self, op: Op, error: PaneError) {
  >         self.lock().queued.push((op, error));
  >     }
  > ```

## Added by F (round 3, after the outside diff gate's BLOCK)

The diff gate's round 2 asked for these under NV-1: whether `resolve(P, Some(n))` can answer more than one pane, and
whether a named pane outside P is always `pane-not-in-profile`. They are copied from the tree at `4443dd4`. None of the
three files is in this change (`git diff origin/main` on each is empty).

- **Fact:** the real `ProfileScope`, `StoreScope` (#663; #649 wires it in), answers `resolve(P, Some(n))` with exactly one
  pane: the record `PaneStore::get(n)` returns, and only when that record names P. Anything else, a pane with no record
  included, is `pane-not-in-profile`. So P1's `find` by name and `into_iter().next()` take the same pane, and no answer
  of `resolve(P, Some(n))` lacks the pane `n`.
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:159-173`, `:92-100`; `crates/holler-pane/src/ports.rs:63-64`
  **Verbatim excerpt:**
  > ```
  > impl ProfileScope for StoreScope {
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError> {
  >         let stored = self.stored(profile)?;
  >         let panes = match pane {
  >             None => self.members(&stored.name)?,
  >             Some(name) => vec![self.member(&stored.name, name)?],
  >         };
  >         Ok(ResolvedScope {
  >             profile: stored,
  >             panes,
  >         })
  > ```
  > ```
  >     /// The pane `name`, whose record must name `profile`: else `pane-not-in-profile`.
  >     fn member(&self, profile: &ProfileName, name: &PaneName) -> Result<Pane, PaneError> {
  >         match self.panes.get(name)? {
  >             Some(pane) if belongs(&pane, profile) => Ok(pane),
  >             _ => Err(PaneError::PaneNotInProfile {
  >                 what: format!("{name} is not in profile {:?}", profile.as_str()),
  >             }),
  >         }
  >     }
  > ```
  > ```
  >     /// The pane named `name`, or `None`.
  >     fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;
  > ```

- **Fact:** the test kit's `FakeProfileScope`, which every `--profile` test of this story runs on, answers the same way:
  one pane for a named pane, else `pane-not-in-profile`, a pane with no record included.
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:190-204`, `:117-126`
  **Verbatim excerpt:**
  > ```
  > impl ProfileScope for FakeProfileScope {
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError> {
  >         let stored = self.stored(profile)?;
  >         let panes = match pane {
  >             None => self.members(&stored.name)?,
  >             Some(name) => vec![self.member(&stored.name, name)?],
  >         };
  >         Ok(ResolvedScope {
  >             profile: stored,
  >             panes,
  >         })
  > ```
  > ```
  >     /// The pane `name`, whose record must name `profile`: `pane-not-in-profile`
  >     /// otherwise, a pane with no record included.
  >     fn member(&self, profile: &ProfileName, name: &PaneName) -> Result<Pane, PaneError> {
  >         match self.panes.get(name)? {
  >             Some(pane) if belongs(&pane, profile) => Ok(pane),
  >             _ => Err(PaneError::PaneNotInProfile {
  >                 what: format!("{name} is not in profile {:?}", profile.as_str()),
  >             }),
  >         }
  >     }
  > ```

## Added by F (round 5, after the outside diff gate's r4 BLOCK)

The r4 gate asked for these under B-1 and W-1 (whether a failed `select_session` counts as "called"), NV-1 (the shape of
an OpenCode session id) and NV-3 (what the rig's call log covers). They are copied from the tree at `48f2395`, apart from
the `attach.rs` excerpts of the first entry, which round 6 copied from the tree with `bd5e825` merged. None of these files
is in this change, and `git diff origin/main` on each is empty.

- **Fact:** a failed `select_session` may still have moved the TUI, and its error does not say whether it did. The port's
  answer is only `Ok(())` or an error. The OpenCode adapter's `select_session` (#642 part 2, on `origin/main` since
  `bd5e825`; the spike's plan at `opencode-pane-spike.md:237-239`, built) sends the switch request, then watches the TUI's
  title until it shows the session. A `timeout` of that watch, or a TUI found gone during it (`no TUI in pane P`), comes
  after the request that moves the screen. The same `no TUI in pane P`, and a `timeout` under the method's one deadline,
  can also come before anything is sent. This is why `acted` is set on the call's own failure as well as on every
  failure after it.
  **Source:** `crates/holler-pane/src/ports.rs:199-200`, `crates/holler-adapter-opencode/src/attach.rs:7-8`, `:74-95`, `:202-204`
  **Verbatim excerpt:**
  > ```
  >     /// Switch the TUI of `pane` to `session`.
  >     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;
  > ```
  > ```
  > //! - **One deadline per method**, taken at its entry: every tmux call and every request of
  > //!   the method runs within it, and past it the answer is `timeout` with the method's `op`.
  > ```
  > ```
  >     let deadline = deadline_after(config.timeouts.call);
  >     let tui = Tui::of(harness, pane, OP_SELECT_SESSION, deadline)?;
  >     let port = match tui.query(deadline)? {
  >         Query::Live { start_command, .. } => tui::attach_port(&start_command),
  >         Query::Dead(_) | Query::NoPane => None,
  >     }
  >     .ok_or_else(|| tui.no_tui())?;
  >     let call = harness.call_until(port, OP_SELECT_SESSION, deadline);
  >     known(&call, &session_path(session), session)?;
  >     let reply = call.send(SELECT, SELECT.label, Some(&json!({ "sessionID": session })))?;
  >     if reply.status == 404 {
  >         return Err(PaneError::SessionNotFound {
  >             what: session.to_owned(),
  >         });
  >     }
  >     if json_of(&reply) != Some(Value::Bool(true)) {
  >         return Err(call.unexpected(SELECT, &reply, "true"));
  >     }
  >     match tui.watch(port, session, config.timeouts.settle)? {
  >         Seen::Shown => Ok(()),
  >         Seen::Dead(_) | Seen::Gone => Err(tui.no_tui()),
  >     }
  > ```
  > ```
  >     /// Query the pane every `SETTLE_POLL`, through the crate's one poll, until it is a live
  >     /// attach to the server on `port` whose title shows `id`, a dead pane or no pane, within
  >     /// `settle` and the method's deadline. None of them in time is `timeout`.
  > ```

- **Fact:** an OpenCode session id is `ses_` and 26 characters, 30 in all (the spike measured this). The OpenCode adapter's
  tests (#642) take the 26 to be `[0-9A-Za-z]`, and the fake mints `ses_` and 26 hex digits. So every id from either
  passes `parse_session_id`'s `[A-Za-z0-9_-]{1,64}`, which is the claim in that function's doc comment.
  **Source:** `docs/research/opencode-pane-spike.md:149-151`, `crates/holler-adapter-opencode/tests/hermetic_test.rs:26-27`, `crates/holler-pane-testkit/src/harness.rs:122-124`
  **Verbatim excerpt:**
  > ```
  > **So the caveat is a naming rule Holler must own:** give every session of record a unique, non-default title of
  > 40 characters or fewer that maps back to its id. The simplest such title is the id itself: `ses_` plus 26 characters
  > is 30 characters. Never set `OPENCODE_DISABLE_TERMINAL_TITLE` in a pane. Whether **Herdr** exposes a pane's
  > ```
  > ```
  > /// A session id of OpenCode's shape: `ses_` and 26 of `[0-9A-Za-z]`.
  > const ID: &str = "ses_0123456789abcdefABCDEFghij";
  > ```
  > ```
  > /// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
  > ///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
  > ///   directory, or list that directory's sessions in creation order.
  > ```

- **Fact:** the rig's `mark` and `calls_since` read the call log of every fake the verb's ports hold: the pane store,
  the profile store, Herdr, the host, the harness and the prober. A run through `ports_with(&wrapper)` keeps the rig's
  own Herdr and host. Each fake records a call before it answers, failed calls included, and `send_text` and `send_keys`
  are recorded the same way. So empty `calls.herdr` and `calls.host` mean that no Herdr or host method was called, and
  so no keystroke was sent (AC 2's I4 check in `both_with`).
  **Source:** `crates/holler-cli/tests/pane_verbs/doctor/rig.rs:249-272`, `:159-170`; `crates/holler-pane-testkit/src/fault.rs:85-97`; `crates/holler-pane-testkit/src/herdr.rs:259-260`, `:270-271`
  **Verbatim excerpt:**
  > ```
  >     /// Every call made through every port so far.
  >     pub fn mark(&self) -> Calls {
  >         Calls {
  >             panes: self.panes.faults().calls(),
  >             profiles: self.profiles.faults().calls(),
  >             herdr: self.herdr.faults().calls(),
  >             host: self.host.faults().calls(),
  >             harness: self.harness.faults().calls(),
  >             probes: self.prober.calls().len(),
  >         }
  >     }
  >
  >     /// The calls made after `mark`.
  >     pub fn calls_since(&self, mark: &Calls) -> Calls {
  >         let now = self.mark();
  >         Calls {
  >             panes: now.panes[mark.panes.len()..].to_vec(),
  >             profiles: now.profiles[mark.profiles.len()..].to_vec(),
  >             herdr: now.herdr[mark.herdr.len()..].to_vec(),
  >             host: now.host[mark.host.len()..].to_vec(),
  >             harness: now.harness[mark.harness.len()..].to_vec(),
  >             probes: now.probes - mark.probes,
  >         }
  >     }
  > ```
  > ```
  >     /// The ports over the rig's fakes, with `harness` in place of the rig's own.
  >     pub fn ports_with<'a>(&'a self, harness: &'a dyn HarnessPort) -> Ports<'a> {
  >         Ports {
  >             pane_store: &*self.panes,
  >             profile_store: &*self.profiles,
  >             herdr: &self.herdr,
  >             host: &self.host,
  >             harness,
  >             scope: &self.scope,
  >             prober: &self.prober,
  >         }
  >     }
  > ```
  > ```
  >     /// Every call made through the port, oldest first, the failed ones included.
  >     pub fn calls(&self) -> Vec<Op> {
  >         self.lock().calls.clone()
  >     }
  >
  >     /// What a fake calls first in every port method. It records the call, sleeps for
  >     /// the delay (without holding the lock, so other calls proceed), and then answers
  >     /// the standing fault if there is one, or else the oldest error queued for `op`.
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  >             state.delay
  > ```
  > ```
  >     fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
  >         self.faults.enter(HerdrOp::SendText)?;
  > ```
  > ```
  >     fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
  >         self.faults.enter(HerdrOp::SendKeys)?;
  > ```
