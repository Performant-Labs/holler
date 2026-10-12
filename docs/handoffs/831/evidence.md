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

## F, the rework after S's REWORK (the script's phase 6; `origin/main` at `037405f` merged)

- **Fact:** the roster prints the hub's rows only, each joined to the pane named like its label, if any: a registry pane
  that no hub row names has no roster row. So a Claude Code pane, which no body serves until #850, is not in the roster
  (ADR-0021 section 15 item 18's "The roster, as built", `docs/pane-control.md` section 4, the CHANGELOG entry).
  **Source:** `crates/holler-cli/src/roster_cmd.rs:173-181`
  **Verbatim excerpt:**
  > ```rust
  >     let hub_rows = reply.get("rows").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
  >     let rows: Vec<Joined<'_>> = hub_rows
  >         .iter()
  >         .map(|hub| Joined {
  >             hub,
  >             pane: pane_of(hub, &registered.panes),
  >         })
  >         .filter(|row| profile.is_none() || row.pane.is_some())
  >         .collect();
  > ```

- **Fact:** the cells the roster prints for a joined pane read no harness port (`health` only), so a Claude Code record,
  which has none, renders like an OpenCode one once a row joins it.
  **Source:** `crates/holler-cli/src/pane/list.rs:248-262`
  **Verbatim excerpt:**
  > ```rust
  > impl From<&Pane> for PaneRow {
  >     fn from(pane: &Pane) -> Self {
  >         Self {
  >             name: pane.name.clone(),
  >             pos: pane.herdr.grid,
  >             profile: pane.profile.clone(),
  >             project: pane.host.cwd.clone(),
  >             health: pane.harness.health.clone(),
  >             shown: pane.last_observed.shown.clone(),
  >             driven: pane.last_observed.driven.clone(),
  >             sync: SessionSync::of(pane),
  >             hold: pane.hold.clone(),
  >         }
  >     }
  > }
  > ```

- **Fact:** the hub refuses a pane registry file it cannot parse as `store-corrupt`, so an older `holler` that cannot read
  a Claude Code record fails the whole registry closed (`docs/pane-control.md` section 11, the downgrade bullet).
  **Source:** `crates/holler-hub/src/panes/persist.rs:143` and `crates/holler-hub/src/panes/store.rs:328-336`
  **Verbatim excerpt:**
  > ```rust
  >     let doc: Doc<E> = serde_json::from_slice(&bytes).map_err(|e| Problem::parse(&e))?;
  > ```
  > ```rust
  > fn load_table(path: &Path, retained: usize) -> Result<Table, PaneError> {
  >     let doc = persist::load_doc::<PaneEvent>(path).map_err(|problem| {
  >         log_fault("pane_registry_corrupt", path, &problem, CORRUPT_EFFECT);
  >         PaneError::StoreCorrupt {
  >             what: problem.what(LABEL, path),
  >         }
  >     })?;
  >     Ok(Table::from_doc(doc, retained))
  > }
  > ```

- **Fact:** a launch's pane id is bound to a name by claiming the one pending offer, so two relaunches at once (doctor's
  `--fix`, one thread per pane) can pair one pane's id with the other's name; only the OpenCode harness is given the TUI
  lookup that reads those pairs (`wiring/bind.rs`'s module doc, the `Bindings` paragraph).
  **Source:** `crates/holler-cli/src/pane/wiring/bind.rs:169-176` and `crates/holler-cli/src/pane/wiring.rs:163-175`,
  `:201-205`
  **Verbatim excerpt:**
  > ```rust
  >     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
  >         let offered = self.bindings.claim();
  >         self.inner.ensure_session(name, cwd)?;
  >         if let Some(pane) = offered {
  >             self.bindings.bind(pane, name.clone());
  >         }
  >         Ok(())
  >     }
  > ```
  > ```rust
  >         let lookup = tui_session(
  >             &session,
  >             bindings.clone(),
  >             Arc::clone(&panes),
  >             Arc::clone(&lazy),
  >         );
  >         Self {
  >             herdr: BindingHerdr::new(lazy, bindings.clone()),
  >             host: BindingHost::new(Arc::new(tmux_host(&tmux)), bindings),
  >             harness: HarnessRouter::new(
  >                 WiredHarness::new(opencode_harness(env, &tmux, lookup)),
  >                 WiredHarness::new(claude_harness(env)),
  >             ),
  > ```
  > ```rust
  > fn opencode_harness(
  >     env: Lookup<'_>,
  >     socket: &Result<HostSocket, PaneError>,
  >     tui_session: Resolver<PaneId, PaneName>,
  > ) -> Result<Arc<dyn HarnessPort>, PaneError> {
  > ```

- **Fact:** the binary's default is found by looking the given program name up in each absolute `PATH` directory, so
  passing `CLAUDE_CODE_COMMAND` (whose value is `claude`) instead of the literal changes no behaviour (REWORK 2).
  **Source:** `crates/holler-cli/src/pane/wiring/env.rs:137-145`
  **Verbatim excerpt:**
  > ```rust
  > fn program_bin(env: Lookup<'_>, variable: &str, program: &str) -> Result<PathBuf, PaneError> {
  >     if let Some(value) = env(variable) {
  >         return absolute(variable, value);
  >     }
  >     let path = env("PATH").unwrap_or_default();
  >     std::env::split_paths(&path)
  >         .filter(|dir| dir.is_absolute())
  >         .map(|dir| dir.join(program))
  >         .find(|candidate| is_executable(candidate))
  > ```

## T, phase 7

- **Fact:** the exit codes the `pane_claude` tests assert (3 for each plan refusal such as `port-policy-unsupported`,
  `process-in-use`, `claude-login-env-refused`, `harness-change-refused`; 2 for a bad `--harness`) come from the error
  class alone, in unchanged code: usage 2, refusal 3, failure 1.
  **Source:** `crates/holler-pane/src/error.rs:239-245`
  **Verbatim excerpt:**
  > ```rust
  >     pub const fn exit_code(self) -> i32 {
  >         match self {
  >             ErrorClass::Usage => 2,
  >             ErrorClass::Refusal => 3,
  >             ErrorClass::Failure => 1,
  >         }
  >     }
  > ```

## T, phase 7, test-only rework (S "Rework 1")

- **Fact:** the roster turns every registry record into `pane list`'s row with no filter, so a Claude Code record
  (`HarnessKind::Claude`) is joined like any other. `roster_panes/registry.rs`'s
  `a_claude_pane_record_fills_its_row_as_pane_list_prints_it` pins this, and a mutant that filters Claude records here
  fails it. The code is from #648 and unchanged by this story (`git diff origin/main` of the file is empty).
  **Source:** `crates/holler-cli/src/roster_cmd.rs:262-267`
  **Verbatim excerpt:**
  > ```rust
  >     fn of(panes: &[Pane]) -> Self {
  >         Self {
  >             panes: panes.iter().map(PaneRow::from).collect(),
  >             note: None,
  >         }
  >     }
  > ```
