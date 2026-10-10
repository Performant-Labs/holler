# Evidence: #641 host adapter

Facts in unchanged code that the diff relies on. F's entries (Phase 6); T appends its own at T-green.

- **Fact:** `HostPort` is a synchronous `Send + Sync` trait with exactly these four methods; `TmuxHost` implements it as is (the signatures are frozen by #637).
  **Source:** `crates/holler-pane/src/ports.rs:156-168`
  **Verbatim excerpt:**
  > ```
  > pub trait HostPort: Send + Sync {
  >     /// Make the tmux session `name` exist, working in `cwd`.
  >     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;
  >
  >     /// Run `argv` (never through a shell) in the session `name`.
  >     fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>;
  >
  >     /// Stop the processes the session `name` owns.
  >     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;
  >
  >     /// The process ids running in the session `name`.
  >     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
  > }
  > ```

- **Fact:** the test kit names the port's methods `host.ensure_session`, `host.run`, `host.stop_owned` and `host.ps`. The adapter's `Timeout.op` literals (`OP_*` in `src/lib.rs`) copy these strings, because the kit is only a dev-dependency (ADR-0021 section 5); the default-run timeout test compares them with `HostOp::as_str`.
  **Source:** `crates/holler-pane-testkit/src/host.rs:35-44`
  **Verbatim excerpt:**
  > ```
  > impl PortOp for HostOp {
  >     fn as_str(self) -> &'static str {
  >         match self {
  >             HostOp::EnsureSession => "host.ensure_session",
  >             HostOp::Run => "host.run",
  >             HostOp::StopOwned => "host.stop_owned",
  >             HostOp::Ps => "host.ps",
  >         }
  >     }
  > }
  > ```

- **Fact:** the fake refuses a missing session before an empty argv, and words the empty-argv refusal "an empty argv has no program to run". The adapter's `run` keeps that order (the read of the session's directory comes before `check_argv`) and that text.
  **Source:** `crates/holler-pane-testkit/src/host.rs:208-214`
  **Verbatim excerpt:**
  > ```
  >     fn start(&mut self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
  >         let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
  >         if argv.as_slice().is_empty() {
  >             return Err(PaneError::Usage {
  >                 message: "an empty argv has no program to run".to_owned(),
  >             });
  >         }
  > ```

- **Fact:** the conformance suite reads a missing session as `pane-not-found` for `run` and `ps` and as `Ok` for `stop_owned`, and it ensures the session again before reading `ps` after `stop_owned`. The adapter's error mapping follows the first two, and its docs' relaunch order (`stop_owned`, then `ensure_session`, then `run`) follows the third.
  **Source:** `crates/holler-pane-testkit/src/conformance/host.rs:10-20`
  **Verbatim excerpt:**
  > ```
  > //! - A missing session is `pane-not-found` for `run` and `ps`, which is how this suite
  > //!   reads #641's "a missing session is a typed error". The tmux session is named by the
  > //!   pane's name, so the pane's code fits; `session-not-found` stays the code of a
  > //!   harness session. An empty argv is `usage`.
  > //! - `stop_owned` of a missing session is `Ok`: nothing is owned, so nothing is
  > //!   stopped, and a `relaunch` or `close` after a crash does not fail on it (case 7).
  > //!
  > //! The cases assert only what a real tmux session can also satisfy. A real session may
  > //! hold a shell process, so no case asserts that a fresh session's `ps` is empty; and it
  > //! may end with its last process, so a case ensures the session again before it reads
  > //! `ps` after `stop_owned`. Every list of pids is read as a set: the port fixes no order.
  > ```

- **Fact:** a pane name segment is only `[0-9a-z]` and `-`, and starts and ends with `[0-9a-z]`. So a name never begins with `-` (it cannot become a flag) and holds no `;`, `#`, `:` or `=`; `src/tmux.rs` puts it into `-s NAME`, `-t =NAME` and `-t =NAME:` unescaped.
  **Source:** `crates/holler-proto/src/vocab.rs:202-223`
  **Verbatim excerpt:**
  > ```
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
  > ```

- **Fact:** `PaneName` displays as the name verbatim, which is what `format!("={name}")`, `format!("={name}:")` and `PaneNotFound { what: name.to_string() }` produce.
  **Source:** `crates/holler-pane/src/pane.rs:50-54`
  **Verbatim excerpt:**
  > ```
  > impl fmt::Display for PaneName {
  >     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
  >         f.write_str(self.as_str())
  >     }
  > }
  > ```
