# Evidence: #647 the reconcile engine and `holler pane doctor`

Source facts the diff relies on that live in unchanged code. F's entries (Phase 6).

- **Fact:** on the fake harness, a call that reaches a frozen server answers `timeout` and one that reaches a killed (or never served) server answers `unavailable`; this is what tells `server-wedged` from `server-down` after a failed health check.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:365-371`
  **Verbatim excerpt:**
  > fn reach(&self, port: u16, op: HarnessOp) -> Result<(), PaneError> {
  >     match self.state(port) {
  >         Some(ServerState::Running) => Ok(()),
  >         Some(ServerState::Frozen) => Err(frozen(op)),
  >         Some(ServerState::Killed) | None => Err(unreachable_server(port)),
  >     }
  > }

- **Fact:** `shown_session` does not reach the server, so a TUI keeps showing its session while its server is frozen or dead (no mismatch is reported for a wedged or down server whose TUI still shows the session of record).
  **Source:** `crates/holler-pane-testkit/src/harness.rs:351-354`
  **Verbatim excerpt:**
  > fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
  >     self.faults.enter(HarnessOp::ShownSession)?;
  >     Ok(self.lock().tuis.get(pane).and_then(|tui| tui.shown.clone()))
  > }

- **Fact:** a missing tmux session is `pane-not-found` from `HostPort::ps`, the one outcome reconcile maps to `tmux-session-missing` (any other `ps` error is `observe-failed`).
  **Source:** `crates/holler-pane-testkit/src/host.rs:195-202`
  **Verbatim excerpt:**
  > fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
  >     self.faults.enter(HostOp::Ps)?;
  >     self.lock()
  >         .sessions
  >         .get(name)
  >         .map(|session| session.pids.clone())
  >         .ok_or_else(|| not_found(name))
  > }

- **Fact:** a wedged fake port answers `timeout` whose `op` is the method's `<port>.<method>` name; reconcile names the same calls with its own constants (`herdr.snapshot`, `herdr.version`, `host.ps`, ...).
  **Source:** `crates/holler-pane-testkit/src/fault.rs:119-125`
  **Verbatim excerpt:**
  > fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
  >     match &self.standing {
  >         Some(Fault::Wedged) => {
  >             return Err(PaneError::Timeout {
  >                 op: op.as_str().to_owned(),
  >             })
  >         }

- **Fact:** `findings::quoted` is `error::excerpt`: one value `{:?}`-quoted (control characters and quotes escaped) and cut to 64 characters.
  **Source:** `crates/holler-pane/src/error.rs:688-695`
  **Verbatim excerpt:**
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }

- **Fact:** a pane name is one segment of `[a-z0-9-]`, alphanumeric first and last, never a `/`, so a remedy built from constant words and a `PaneName` cannot carry a space, a shell metacharacter or a leading `-`.
  **Source:** `crates/holler-proto/src/vocab.rs:202-223`
  **Verbatim excerpt:**
  > fn check_segment(seg: &[u8]) -> Result<(), NameError> {
  >     if seg.is_empty() {
  >         return Err(NameError::Empty);
  >     }
  >     if seg.len() > MAX_SEGMENT_LEN {
  >         return Err(NameError::TooLong);
  >     }
  >     if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
  >         return Err(NameError::BadCharacter);
  >     }
  >     for &c in seg {
  >         if !(is_word(c) || c == b'-') {
  >             return Err(NameError::BadCharacter);
  >         }
  >     }
  >     Ok(())
  > }
  >
  > #[inline]
  > fn is_word(b: u8) -> bool {
  >     matches!(b, b'0'..=b'9' | b'a'..=b'z')
  > }

- **Fact:** every port is `Send + Sync` and `Ports` is `Copy`, so the pass hands `Ports` to one scoped thread per pane with no new dependency.
  **Source:** `crates/holler-pane/src/ports.rs:224-227`
  **Verbatim excerpt:**
  > /// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
  > /// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
  > #[derive(Clone, Copy)]
  > pub struct Ports<'a> {

- **Fact:** the hub asks a periodic writer of `last_observed` to skip a record that has not changed; reconcile writes only on a change (or a record's first observation).
  **Source:** `crates/holler-hub/src/panes/mod.rs:27-28`
  **Verbatim excerpt:**
  > //! - Each write rewrites the whole file and wakes every watcher. A periodic writer, such as
  > //!   reconcile writing `last_observed` (#647), must skip a record that has not changed.

- **Fact:** a fixture record starts with health `unknown` and `last_observed.at` 0, so a rig's first pass always writes (and `at == 0` is the "never observed" state the first-observation rule keys on).
  **Source:** `crates/holler-pane-testkit/src/fixture.rs:57-70`
  **Verbatim excerpt:**
  > harness: HarnessInfo {
  >     kind: HarnessKind::Opencode,
  >     port: SAMPLE_PORT,
  >     pid: None,
  >     health: Health::Unknown,
  > },
  > session_of_record: None,
  > role: PaneRole::Agent,
  > hold: Hold::None,
  > last_observed: LastObserved {
  >     shown: None,
  >     driven: None,
  >     at: 0,
  > },

- **Fact:** the verb's exit code is the class of the error's code (`class_of`), so `pane-not-found`, `profile-not-found` and `pane-not-in-profile` exit 3, `usage` exits 2 and `unavailable` exits 1; a completed pass is `Ok` and exits 0.
  **Source:** `crates/holler-cli/src/output.rs:336-340`
  **Verbatim excerpt:**
  > /// The exit code of an error: the exit code of its code's class, decided by
  > /// `holler_pane::error::class_of` (1 runtime failure, 2 usage, 3 refusal).
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }

- **Fact:** the clock the verb passes as `now_ms` is the workspace's one wall-clock reader.
  **Source:** `crates/holler-proto/src/clock.rs:33-40`
  **Verbatim excerpt:**
  > /// The current unix epoch in whole milliseconds. `0` on a clock error, same
  > /// discipline as [`now_secs`].
  > pub fn now_millis() -> i64 {
  >     std::time::SystemTime::now()
  >         .duration_since(std::time::UNIX_EPOCH)
  >         .map(|d| d.as_millis() as i64)
  >         .unwrap_or(0)
  > }
