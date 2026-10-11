# Evidence — #649 hub wiring (F, Phase 5)

Facts from code the diff does not change, which the wiring (`crates/holler-cli/src/pane/wiring.rs`
and `wiring/{store,env,bind}.rs`) relies on. One entry per fact; each excerpt is copied verbatim.

- **Fact:** the store clients' one dispatch point, `transport::call(None, ..)`, is the local control exchange
  `holler_hub::control::run` (W-9: no second direct caller of `control::run`).
  **Source:** `crates/holler-cli/src/transport.rs:33-37`
  **Verbatim excerpt:**
  > pub fn call(server: Option<&str>, call: &ControlCall) -> Result<serde_json::Value, ControlError> {
  >     match server {
  >         None => holler_hub::control::run(call),
  >         Some(server) => remote(server, call),
  >     }

- **Fact:** a control exchange resolves its socket from `HOLLER_STATE_DIR` when each call is made, so a unit-like
  client reaches the hub of that moment, and with no resolvable state dir there is no hub (`NoLiveHub`).
  **Source:** `crates/holler-hub/src/state.rs:38-41` and `crates/holler-hub/src/control.rs:470-473`
  **Verbatim excerpt:**
  > pub fn resolve_state_dir() -> Option<PathBuf> {
  >     if let Some(dir) = std::env::var_os("HOLLER_STATE_DIR") {
  >         return Some(PathBuf::from(dir));
  >     }
  >
  > (control.rs)
  >     let path = match resolve_state_dir() {
  >         Some(dir) => control_sock_path(&HubState::from_root(dir)),
  >         None => return Err(ControlError::NoLiveHub),
  >     };

- **Fact:** how each `ControlError` variant arises in one exchange, which D3's mapping (`store.rs`, `call` and
  `timed_out`) reads: a failed connect is `NoLiveHub` (nothing sent); the call's `timeout` is the socket's read
  timeout, so no reply in time is an `Io` from `read_line` (on Linux `WouldBlock`); a connection closed with no reply
  is `BadReply`; a JSON-RPC error frame is `Refused`. A failed `write_all` is also `Io`, which a client cannot tell
  from a lost reply, hence a write's `Io` is `timeout`.
  **Source:** `crates/holler-hub/src/control.rs:492-513`
  **Verbatim excerpt:**
  >     let stream = UnixStream::connect(path).map_err(|_| ControlError::NoLiveHub)?;
  >     stream.set_read_timeout(Some(timeout)).map_err(ControlError::Io)?;
  >
  >     let cid = CorrelationId::parse(id_literal).expect("a well-formed literal control id");
  >     let req = holler_proto::Envelope::request(&cid, method, params);
  >     let bytes = format!("{}\n", holler_proto::encode(&req).expect("encode the control request"));
  >
  >     let mut stream = stream;
  >     stream.write_all(bytes.as_bytes()).map_err(ControlError::Io)?;
  >     stream.flush().map_err(ControlError::Io)?;
  >
  >     let mut line = String::new();
  >     let n = std::io::BufReader::new(stream)
  >         .read_line(&mut line)
  >         .map_err(ControlError::Io)?;
  >     if n == 0 {
  >         return Err(ControlError::BadReply("the hub closed the socket before replying".into()));
  >     }
  >
  >     let env = holler_proto::decode(&line).map_err(|e| ControlError::BadReply(e.to_string()))?;
  >     if let Some(error) = env.error() {
  >         return Err(ControlError::Refused(error.clone()));

- **Fact:** `connect`'s status exchange (`store::reach_hub`) uses `ControlCall::status()` unchanged, so it keeps the
  control client's own 5 s (E60), while the store requests set their own 10 s.
  **Source:** `crates/holler-hub/src/control.rs:73-75`
  **Verbatim excerpt:**
  >     pub fn status() -> Self {
  >         Self { method: "control/status", params: None, timeout: CLIENT_TIMEOUT }
  >     }

- **Fact:** the hub's long-poll window, from which a `*/watch` request's timeout is sized (`WATCH_TIMEOUT` = 4 s + 5 s).
  **Source:** `crates/holler-hub/src/panes/mod.rs:82`
  **Verbatim excerpt:**
  > pub const WATCH_WAIT: Duration = Duration::from_secs(4);

- **Fact:** the bindings' coupling (D8 (a)): the launch engine calls `ensure_pane`, then `ensure_session` with the
  pane's name, then `attach_tui` with the Herdr pane id, and writes the record only after the act. These two lines are
  the only calls of `ensure_pane` and `ensure_session` in `holler-pane/src` and `holler-cli/src` outside the wiring's
  own decorators (grep), so every `ensure_session` the wiring sees follows its own `ensure_pane`.
  **Source:** `crates/holler-pane/src/tx_launch.rs:650-668` and `:615`
  **Verbatim excerpt:**
  >     let herdr = ports.herdr.ensure_pane(&plan.target)?;
  >     if !seen.listed.iter().any(|p| same_pane(p, &herdr)) {
  >         made.pane = Some(herdr.pane_id.clone());
  >     }
  >     budget.check()?;
  >     let cwd = &plan.spec.host.cwd;
  >     ports.host.ensure_session(plan.name, cwd)?;
  >     made.session = true;
  >     if let Some(command) = &plan.spec.command {
  >         budget.check()?;
  >         ports.host.run(plan.name, cwd, command)?;
  >     }
  >     server::start(ports, plan, seen, budget, boot_poll)?;
  >     budget.check()?;
  >     let session = session_of_record(ports, plan)?;
  >     budget.check()?;
  >     ports
  >         .harness
  >         .attach_tui(&herdr.pane_id, plan.port, &session)?;
  >
  > (line 615, in `act_live`, after `bring_up` returned)
  >     let stored = ports.pane_store.cas_put(&record, expected)?;

- **Fact:** the OpenCode adapter fixes a TUI method's deadline before it calls the `tui_session` lookup, and returns
  the lookup's error as it is: the basis of W-C's third bounded-otherwise case, recorded in the ADR paragraph.
  **Source:** `crates/holler-adapter-opencode/src/attach.rs:96-102` and `:163-169`
  **Verbatim excerpt:**
  > pub(crate) fn shown_session(
  >     harness: &OpenCodeHarness,
  >     pane: &PaneId,
  > ) -> Result<Option<String>, PaneError> {
  >     let deadline = deadline_after(harness.config.timeouts.call);
  >     let tui = Tui::of(harness, pane, OP_SHOWN_SESSION, deadline)?;
  >
  > (lines 163-169)
  >     fn of(
  >         harness: &'a OpenCodeHarness,
  >         pane: &PaneId,
  >         op: &'static str,
  >         deadline: Instant,
  >     ) -> Result<Self, PaneError> {
  >         let session = (harness.config.tui_session)(pane)?;

- **Fact:** building the Herdr adapter validates its config and then pings Herdr (so the wiring builds it lazily,
  D2); the validation also refuses a relative socket as `usage`.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:125-130` and `:490-495`
  **Verbatim excerpt:**
  >     pub fn connect_with(config: HerdrConfig, transport: T) -> Result<Self, PaneError> {
  >         validate(&config)?;
  >         let adapter = Self { config, transport };
  >         adapter.run_as(OP_CONNECT, |deadline| adapter.supported_server(deadline))?;
  >         Ok(adapter)
  >     }
  >
  > (lines 490-495)
  >     if !config.socket.is_absolute() {
  >         return Err(usage(format!(
  >             "the Herdr config's socket {:?} is relative; it must be an absolute path",
  >             config.socket
  >         )));
  >     }

- **Fact:** building the tmux host does no I/O and takes `tmux` and `kill` from `PATH` (D6), so `connect` builds it
  eagerly.
  **Source:** `crates/holler-adapter-host/src/lib.rs:177-188`
  **Verbatim excerpt:**
  >     /// A host on `socket`, with `tmux` and `kill` from `PATH`, no config file, a 10 s
  >     /// bound and a 2 s stop grace. Does no I/O.
  >     pub fn new(socket: TmuxSocket) -> Self {
  >         Self {
  >             socket,
  >             tmux: PathBuf::from("tmux"),
  >             kill: PathBuf::from("kill"),
  >             config: None,
  >             timeout: DEFAULT_TIMEOUT,
  >             stop_grace: DEFAULT_STOP_GRACE,
  >         }
  >     }

- **Fact:** building the OpenCode harness starts nothing (D2), so `connect` builds it eagerly.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:307-310`
  **Verbatim excerpt:**
  >     /// An adapter with `config`. It starts nothing until a method is called.
  >     pub fn new(config: OpenCodeConfig) -> Self {
  >         Self { config }
  >     }
