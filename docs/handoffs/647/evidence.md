# Evidence appendix: #647 part 2 (`unattached` and `agent-cannot-dispatch`)

Source facts in **unchanged** code that F's change (Phase 5) relies on. Line numbers are the run's branch after F's change.

- **Fact:** the agent check's rule (3), "the server is healthy and its trusted list holds the session of record", is `Server::has`, which reads only a healthy server's list that was read; a wedged, down or unknown server, or a healthy one whose list failed (`sessions: None`), never passes it.
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:61-72`
  **Verbatim excerpt:**
  > ```rust
  >     fn sessions(&self) -> Option<&[String]> {
  >         match self {
  >             Server::Healthy { sessions } => sessions.as_deref(),
  >             Server::Wedged | Server::Down | Server::Unknown => None,
  >         }
  >     }
  >
  >     /// Whether the server's trusted list holds `session`.
  >     fn has(&self, session: &str) -> bool {
  >         self.sessions()
  >             .is_some_and(|ids| ids.iter().any(|id| id == session))
  >     }
  > ```

- **Fact:** `check_tmux` (which now makes the `attached_clients` call) runs for every pane before the chain branches on the harness's shape, so a Claude Code pane is asked its client count too, and the agent check sits on the server path only.
  **Source:** `crates/holler-pane/src/reconcile/observe.rs:118`, `:130`
  **Verbatim excerpt:**
  > ```rust
  >     check_tmux(ports, pane, &mut findings);
  > ```
  > ```rust
  >     if pane.harness.kind.shape() == Shape::SingleProcess {
  > ```

- **Fact:** a finding built with `Finding::about` gets its kind's remedy with no agent and `NotFixable`, so `agent-cannot-dispatch` has no remedy until `with_agent` sets it from the one table.
  **Source:** `crates/holler-pane/src/findings.rs:335-342`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn about(kind: FindingKind, pane: &Pane, message: String) -> Self {
  >         Self {
  >             pane: Some(pane.name.clone()),
  >             grid: Some(pane.herdr.grid),
  >             remedy: kind.remedy(Some(&pane.name), FixState::NotFixable),
  >             ..Self::new(kind, message)
  >         }
  >     }
  > ```

- **Fact:** a pass sorts every chain's findings and drops exact duplicates, so each standing finding is reported once per pass.
  **Source:** `crates/holler-pane/src/reconcile.rs:261-263`
  **Verbatim excerpt:**
  > ```rust
  >     findings.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
  >     // A finding observed twice (Herdr listing one pane twice) is reported once.
  >     findings.dedup();
  > ```

- **Fact:** `AgentKey::parse` accepts a leading `-` (only ASCII letters, digits, `-` and `_`), which is why the remedy joins the key to `--agent` with `=`.
  **Source:** `crates/holler-pane/src/argv.rs:143-144`
  **Verbatim excerpt:**
  > ```rust
  >         let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
  >         if text.is_empty() || !text.chars().all(allowed) {
  > ```

- **Fact:** `relaunch`'s `--agent` is a plain long option with a value (no `allow_hyphen_values`), checked by `AgentKey::parse` after clap, so the joined form `--agent=-x` is the one that parses as the value `-x`.
  **Source:** `crates/holler-cli/src/pane/args.rs:62-63`, `:141`
  **Verbatim excerpt:**
  > ```rust
  >     #[arg(long, value_name = "KEY")]
  >     pub agent: Option<String>,
  > ```
  > ```rust
  >             agent: self.agent.as_deref().map(AgentKey::parse).transpose()?,
  > ```

- **Fact:** a single-process harness's one conversion of an endpoint answers a port `server-unsupported`, which both Claude implementations of `agent_view` call first (plan review W-4).
  **Source:** `crates/holler-pane/src/ports/process.rs:80-91`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn process(&self) -> Result<(&PaneName, u32), PaneError> {
  >         match self {
  >             HarnessEndpoint::Process { pane, pid } => Ok((pane, *pid)),
  >             HarnessEndpoint::Port(port) => Err(PaneError::Refused {
  >                 code: SERVER_UNSUPPORTED,
  > ```

- **Fact:** the OpenCode adapter's existence check (`known`, the one `abort` makes, and the first read of `agent_view`) answers a 404 or 400 as `session-not-found`.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:615-627`
  **Verbatim excerpt:**
  > ```rust
  > fn known(call: &Call, path: &str, id: &str) -> Result<(), PaneError> {
  >     session_reply(call, path, id).map(drop)
  > }
  > ```
  > ```rust
  >     let reply = call.send(GET_SESSION, path, None)?;
  >     if matches!(reply.status, 400 | 404) {
  >         return Err(PaneError::SessionNotFound {
  >             what: id.to_owned(),
  >         });
  >     }
  > ```

- **Fact:** `Call::unexpected_status`, which the shared message-list reader now uses, names the route, the port and the status and quotes no part of the body.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:576-588`
  **Verbatim excerpt:**
  > ```rust
  >     fn unexpected_status(&self, route: Route, reply: &Reply, wanted: &str) -> PaneError {
  >         PaneError::Unavailable {
  >             what: self.answered(route, reply, wanted),
  >         }
  >     }
  >
  >     /// `<method> <route> on port <port> answered <status>, not <wanted>`.
  >     fn answered(&self, route: Route, reply: &Reply, wanted: &str) -> String {
  >         format!(
  >             "{} {} on port {} answered {}, not {wanted}",
  >             route.method, route.label, self.port, reply.status
  >         )
  >     }
  > ```

- **Fact:** the bound of the message read (plan review W-7): a reply that takes more than 64 MiB off the connection is refused by the transport, and the call answers it `unavailable` naming the route, so reconcile reports `observe-failed`.
  **Source:** `crates/holler-adapter-opencode/src/http.rs:40`, `:228`; `crates/holler-adapter-opencode/src/lib.rs:542-550`
  **Verbatim excerpt:**
  > ```rust
  > const MAX_REPLY: usize = 64 << 20;
  > ```
  > ```rust
  >     HttpError::Garbled(format!("the reply is longer than {MAX_REPLY} bytes"))
  > ```
  > ```rust
  >             Err(HttpError::Garbled(why)) => Err(PaneError::Unavailable {
  >                 what: format!(
  >                     "{} {} on port {}: {}",
  >                     route.method,
  >                     route.label,
  >                     self.port,
  >                     one_line(&why)
  >                 ),
  >             }),
  > ```

- **Fact:** the tmux adapter's strict count parser (`parse_attached`) reads each line with the existing `decimal`, which takes ASCII digits only within `u32`, so an empty, signed, fractional or word value fails the answer.
  **Source:** `crates/holler-adapter-host/src/tmux.rs:237-242`
  **Verbatim excerpt:**
  > ```rust
  > fn decimal(text: &str) -> Option<u32> {
  >     if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
  >         return None;
  >     }
  >     text.parse().ok()
  > }
  > ```

- **Fact:** a refused tmux call of the session (missing session, window or server) is `pane-not-found`, anything else `unavailable` with tmux's first stderr line; `attached_clients` goes through it as `ps` does.
  **Source:** `crates/holler-adapter-host/src/lib.rs:485-493`
  **Verbatim excerpt:**
  > ```rust
  > fn refused(name: &PaneName, refusal: Refusal, what: String) -> PaneError {
  >     if refusal.is_missing() {
  >         PaneError::PaneNotFound {
  >             what: name.to_string(),
  >         }
  >     } else {
  >         PaneError::Unavailable { what }
  >     }
  > }
  > ```

- **Fact:** the fake harness's `reach`, which `FakeHarness::agent_view` calls before it looks at the session (as `abort` does), answers a frozen server `timeout` with the method's op and a killed or never-started one `unavailable`.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:502-508`
  **Verbatim excerpt:**
  > ```rust
  >     fn reach(&self, port: u16, op: HarnessOp) -> Result<(), PaneError> {
  >         match self.state(port) {
  >             Some(ServerState::Running) => Ok(()),
  >             Some(ServerState::Frozen) => Err(frozen(op)),
  >             Some(ServerState::Killed) | None => Err(unreachable_server(port)),
  >         }
  >     }
  > ```

- **Fact:** every doctor rig pane records no agent (`sample_pane`), so rules (1) to (3) hold for each healthy rig pane and every pass asks for the agent view (the read-only assertions admit `AgentView`).
  **Source:** `crates/holler-pane-testkit/src/fixture.rs:79`
  **Verbatim excerpt:**
  > ```rust
  >         opencode_agent: None,
  > ```

- **Fact:** `holler pane doctor` exits 0 with or without findings, so the scratch scenarios' `json(&["pane", "doctor"], 0)` still holds with each scratch pane's `unattached`.
  **Source:** `crates/holler-cli/src/pane/doctor.rs:5-6`
  **Verbatim excerpt:**
  > ```rust
  > //! - **Exit codes.** A pass that completes exits 0, with or without findings: findings are
  > //!   kinds in the report, not errors (ADR-0021 section 9), and an envelope cannot carry data
  > ```

- **Fact (added by T, Phase 6):** a scratch set's OpenCode root is laid out with only the rig's dead-end configuration, which names no agent, so a scratch `--pure` server lists only OpenCode's own agents and `agent-cannot-dispatch` cannot fire there (rule 5); that is why `assert_end_state` (acceptance 18) admits only one `unattached` per scratch pane.
  **Source:** `crates/holler-cli/tests/pane_integration/scratch/mod.rs:314`; `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:69-86`, `:526-531`
  **Verbatim excerpt:**
  > ```rust
  >         rig::lay_out(&oc_root);
  > ```
  > ```rust
  > pub const CONFIG: &str = r#"{
  >   "$schema": "https://opencode.ai/config.json",
  >   "enabled_providers": ["deadend"],
  >   "disabled_providers": ["opencode"],
  >   "model": "deadend/none",
  >   "small_model": "deadend/none",
  >   "autoupdate": false,
  >   "share": "disabled",
  >   "provider": {
  >     "deadend": {
  >       "npm": "@ai-sdk/openai-compatible",
  >       "name": "dead end (closed local port)",
  >       "options": { "baseURL": "http://127.0.0.1:9/v1" },
  >       "models": { "none": { "name": "none" } }
  >     }
  >   }
  > }
  > "#;
  > ```
  > ```rust
  > pub fn lay_out(root: &Path) {
  >     for sub in ["home", "data", "config/opencode", "state", "cache", "proj"] {
  >         std::fs::create_dir_all(root.join(sub)).unwrap();
  >     }
  >     std::fs::write(root.join("config/opencode/opencode.json"), CONFIG).unwrap();
  > }
  > ```

## Added by F in the S REWORK round (Phase 6 call, at `af34a4f`)

Source facts in unchanged code that the quoting of the W-3 clause (S's item 1) relies on.

- **Fact:** the repository's rule for a profile name in a command that a message suggests is POSIX single quoting (#662 B1), so the command pastes into a shell as one word even for a name with a quote or `$(...)`.
  **Source:** `crates/holler-cli/src/profile/delete.rs:202-208`
  **Verbatim excerpt:**
  > ```rust
  > /// `text` POSIX-single-quoted, read back by a shell as one word: each `'` becomes the four
  > /// characters `'\''`, every other character stays, and the whole is wrapped in `'...'`. A
  > /// profile name in a command that a message suggests goes through it (#662 B1), so the command
  > /// pastes into a shell as it is, even for `Some Profile` or a name with a quote or `$(...)`.
  > pub(crate) fn single_quoted(text: &str) -> String {
  >     format!("'{}'", text.replace('\'', r"'\''"))
  > }
  > ```

- **Fact:** a message that both names a profile and suggests a command with it names it `{:?}`-quoted in the prose and POSIX-single-quoted in the command, the shape the W-3 clause now has (the same holds at `delete.rs:173-183`, `create.rs:251-259`, `import/writes.rs:76-86` and `:179-192`).
  **Source:** `crates/holler-cli/src/profile/delete.rs:158-167`
  **Verbatim excerpt:**
  > ```rust
  > fn has_live_panes(name: &ProfileName, members: &[Pane]) -> PaneError {
  >     PaneError::ProfileHasLivePanes {
  >         what: format!(
  >             "{:?} has {} ({}); run holler profile delete {} --keep-panes to detach them",
  >             name.as_str(),
  >             count(members.len(), "live pane"),
  >             pane_list(members.iter().map(|pane| &pane.name)),
  >             single_quoted(name.as_str())
  >         ),
  >     }
  > ```

- **Fact:** the reconcile step a profile verb prints suggests `--profile` with P single-quoted, and its test pins the escaping of a name with a quote and `$(...)`.
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:54-55`, `:555-559`
  **Verbatim excerpt:**
  > ```rust
  >     let name = single_quoted(profile.as_str());
  >     format!("to reconcile, run {doctor} --profile {name} and then holler profile show {name}")
  > ```
  > ```rust
  >         let tricky = reconcile_step(Some(&ProfileName::parse("It's $(id) Demo").unwrap()));
  >         assert!(
  >             tricky.contains("--profile 'It'\\''s $(id) Demo'"),
  >             "not single-quoted: {tricky:?}"
  >         );
  > ```

- **Fact:** ADR-0021 section 3 states that suggested step with P POSIX-single-quoted.
  **Source:** `docs/adr/ADR-0021.md:530-531`
  **Verbatim excerpt:**
  > The reconcile step is exactly `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`,
  > with P POSIX-single-quoted, and without `--profile` it is `to reconcile, run holler pane doctor`.

- **Fact:** a `ProfileName` is at most 64 characters and has no control character, and a stored one (a record's `profile`) deserializes through `parse`. So `single_quoted` puts no control character into a message, and `quoted` never cuts the name.
  **Source:** `crates/holler-pane/src/profile.rs:27-28`, `:42-54`, `:101-105`
  **Verbatim excerpt:**
  > ```rust
  > /// The longest a profile name or an actor may be, in characters.
  > const MAX_NAME_CHARS: usize = 64;
  > ```
  > ```rust
  >         } else if name.chars().count() > MAX_NAME_CHARS {
  >             Some("a profile name is at most 64 characters")
  >         } else if name.chars().any(char::is_control) {
  >             Some("a profile name must not contain control characters")
  > ```
  > ```rust
  > impl<'de> Deserialize<'de> for ProfileName {
  >     fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
  >         deserialize_parsed(deserializer, ProfileName::parse)
  >     }
  > }
  > ```

- **Fact:** `quoted` is `error::excerpt`: Rust `{:?}` quoting, that is double quotes, inside which a shell still expands `$` and a backtick, cut at 64 characters.
  **Source:** `crates/holler-pane/src/error.rs:691-698`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }
  > ```

- **Fact:** doctor's text output prints a finding's message as it is, so the operator sees the single-quoted argument unchanged (the JSON form carries the same string).
  **Source:** `crates/holler-cli/src/pane/doctor.rs:128`
  **Verbatim excerpt:**
  > ```rust
  >     let mut line = format!("{subject} {}: {}", finding.kind.code(), finding.message);
  > ```
