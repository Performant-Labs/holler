# Evidence: #831, a Claude Code harness for panes

Source facts in **unchanged** code that F's change or T's tests rely on, for the diff gate's reviewer, who sees the diff
and not the repo. One entry per fact; each excerpt is copied, not paraphrased. T appends its own at Phase 6.

## F, phase 5

- **Fact:** a code outside the closed list that is valid kebab-case is a refusal (exit 3), so every new open code of
  this story (`port-policy-unsupported`, `process-in-use`, `harness-dialog-open`, ...) is a refusal without being
  listed anywhere, and `pane launch`'s flag check can tell the grammar's `usage` from the kind's refusal by class.
  **Source:** `crates/holler-pane/src/error.rs:266-273`
  **Verbatim excerpt:**
  > ```rust
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```

- **Fact:** `PaneError::SessionNotFound`, which the Claude adapter answers for a switch (`SessionChoice::Resume`) to a
  session with no transcript, is the closed code `session-not-found`, a refusal: the same code a server pane's switch
  gives an unknown target (plan review W1).
  **Source:** `crates/holler-pane/src/error.rs:118`, `:289-291`, `:528`
  **Verbatim excerpt:**
  > ```rust
  >             PaneCode::SessionNotFound => "session-not-found",
  > ```
  > ```rust
  >         | PaneCode::ProfileNotFound
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  > ```
  > ```rust
  >             PaneError::SessionNotFound { .. } => Ok(PaneCode::SessionNotFound),
  > ```

- **Fact:** the one shown/driven comparison, which a single process's launch (`tx_launch/process.rs`, the session the
  process is seen holding) and reconcile's single-process chain reuse rather than restate: a pane with a session of
  record differs when what it shows is anything else, `None` included.
  **Source:** `crates/holler-pane/src/reconcile.rs:210-212`
  **Verbatim excerpt:**
  > ```rust
  > pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
  >     session_of_record.is_some_and(|record| shown != Some(record))
  > }
  > ```

- **Fact:** `HostPort::run` starts the argv in a new window of the pane's tmux session, in `cwd`, and tags the window
  with the pid it started, so `stop_owned` (every owned process group, `TERM` then `KILL`) stops the Claude Code process
  as it stops an OpenCode server: the launch's rollback and a relaunch's first step rely on this.
  **Source:** `crates/holler-adapter-host/src/lib.rs:351-365` and `:367-370`
  **Verbatim excerpt:**
  > ```rust
  >     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
  >         let call = Call::start(OP_RUN, self.timeout);
  >         if let Reply::Refused(refusal, what) = self.run_tmux(&tmux::panes(name), call)? {
  >             return Err(refused(name, refusal, what));
  >         }
  >         check_argv(argv)?;
  >         check_cwd(name, cwd)?;
  >         let (pid, window) = match self.run_tmux(&tmux::new_window(name, cwd, argv), call)? {
  >             Reply::Done(stdout) => {
  >                 tmux::parse_new_window(&stdout).ok_or_else(|| unavailable(BAD_NEW_WINDOW))?
  >             }
  >             Reply::Refused(refusal, what) => return Err(refused(name, refusal, what)),
  >         };
  >         self.tag(name, window, pid, call)
  >     }
  > ```
  > ```rust
  >     /// `TERM` to every owned group, one shared grace, `KILL` to every group not done, and
  >     /// the rest of the bound for those (Decision 4). A session missing at the first
  >     /// listing owns nothing: `Ok`.
  >     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
  > ```

- **Fact:** tmux receives the argv after `env --`, element by element, and only a trailing `;` is escaped, so the Claude
  argv (`env -u ... HOLLER_CLAUDE_SOCKET_DIR=<dir> claude ... --settings <json> ...`) runs as `env -- env -u ...`, never
  through a shell: its `--settings` JSON and the hook's fixed text reach Claude Code as single arguments.
  **Source:** `crates/holler-adapter-host/src/tmux.rs:70-94` and `:166-176`
  **Verbatim excerpt:**
  > ```rust
  > /// `run`'s window: detached, printing its pid and id, starting in `cwd` (escaped:
  > /// tmux format-expands a start directory), with the argv after `env --` and each element
  > /// escaped (Decisions 3 and 13; #802). The caller has checked that `cwd` is an absolute
  > /// path to an existing directory. `env --` keeps a one-element argv from reaching tmux as
  > /// a single argument, which tmux would hand to a shell.
  > pub(crate) fn new_window(name: &PaneName, cwd: &str, argv: &Argv) -> Vec<String> {
  > ```
  > ```rust
  >         "--",
  >         "env",
  >         "--",
  >     ]);
  >     args.extend(argv.as_slice().iter().map(|element| escape(element)));
  >     args
  > }
  > ```
  > ```rust
  > /// A value the adapter did not write, made safe for tmux's command line (Decision 13).
  > /// tmux ends a command at every argument that ends in `;`, after `--` too, and reads a
  > /// trailing `\;` as a literal `;` (`man tmux`, PARSING SYNTAX; E1, R1). So a value that
  > /// ends in `;` gets a `\` before that `;`, and nothing else changes: `x;` is `x\;`, `y\;`
  > /// is `y\\;`, and `;` is `\;`.
  > pub(crate) fn escape(value: &str) -> String {
  > ```

- **Fact:** the Holler state directory the wiring puts `claude-inbox/` under is `HOLLER_STATE_DIR`, else `$HOME/.holler`,
  and the function answers `None` (it never guesses) with neither set; the CLI finds the hub through the same directory.
  **Source:** `crates/holler-hub/src/state.rs:33-41`
  **Verbatim excerpt:**
  > ```rust
  > /// `HOLLER_STATE_DIR` wins; the default is `~/.holler`. A missing `$HOME` with
  > /// no override is a fail-closed refusal — we refuse to guess a state location.
  > /// Returns `None` (after printing the refusal) in that case so the **caller**
  > /// (the CLI's `hub serve` leaf, a bin) owns the exit; a lib helper never exits
  > /// (an exit here would mask the code from the caller).
  > pub fn resolve_state_dir() -> Option<PathBuf> {
  >     if let Some(dir) = std::env::var_os("HOLLER_STATE_DIR") {
  >         return Some(PathBuf::from(dir));
  >     }
  > ```
