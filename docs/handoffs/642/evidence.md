# Evidence: #642 part 3 (F, Phase 6)

Source facts the diff relies on that live in **unchanged** code or in the brief's cuts of the
installed OpenCode 1.19.0 binary. Every excerpt is copied from this tree (branch
`issue-642-implementation`, base `7284001`), not retyped.

- **Fact:** `send_prompt` reuses the existence check `abort` makes: a 404 or a 400 to `GET /session/:id` is `session-not-found`, and any other answer, a 200 that is not that session included, is `unavailable`, all before anything is posted.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:574-594`
  **Verbatim excerpt:**
  > /// The existence check `GET /session/<id>`: `Ok` when the server holds that session. A 404,
  > /// or a 400 (a malformed id), is `session-not-found`. Any other answer, a 200 that is not
  > /// that session included, is `unavailable`.
  > fn known(call: &Call, path: &str, id: &str) -> Result<(), PaneError> {
  >     session_reply(call, path, id).map(drop)
  > }
  >
  > /// [`known`]'s request and rules, also returning the reply and its JSON object, the session,
  > /// for a step that reads more of it (`attach_tui` reads its `directory`).
  > fn session_reply(call: &Call, path: &str, id: &str) -> Result<(Reply, Value), PaneError> {
  >     let reply = call.send(GET_SESSION, path, None)?;
  >     if matches!(reply.status, 400 | 404) {
  >         return Err(PaneError::SessionNotFound {
  >             what: id.to_owned(),
  >         });
  >     }
  >     match json_of(&reply) {
  >         Some(session) if string_at(Some(&session), "id") == Some(id) => Ok((reply, session)),
  >         _ => Err(call.unexpected(GET_SESSION, &reply, "that session")),
  >     }
  > }

- **Fact:** `json_of` reads only a `200`, so `status_object` (and the `/config`, `/agent`, `/question`, `/permission`, `/session` and message-list reads) never takes an error status whose body is a JSON object for an answer (acceptance 8's "a `500` whose body is a JSON object" case).
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:697-704`
  **Verbatim excerpt:**
  > /// The JSON of a `200` reply; `None` for another status or a body that is not JSON.
  > fn json_of(reply: &Reply) -> Option<Value> {
  >     if reply.status == 200 {
  >         serde_json::from_slice(&reply.body).ok()
  >     } else {
  >         None
  >     }
  > }

- **Fact:** every request of a method goes through `Call::send_by`: a refused connection is `unavailable` naming only the port, no reply in time is `timeout` with the method's `op`, and a reply that is not HTTP is `unavailable` naming the route, the port and the client's own `why` text. So the new methods' refused and frozen cases (acceptance 6, and D8's `op` strings) come from this unchanged mapping, and `session_activity`'s best-effort list reads still fail on a refused or frozen server.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:488-514`
  **Verbatim excerpt:**
  > /// Refused is `unavailable`; no reply in time is `timeout` with the call's `op`; a
  > /// reply that is not HTTP is `unavailable` naming the route.
  > fn send_by(
  >     &self,
  >     route: Route,
  >     path: &str,
  >     body: Option<&Value>,
  >     until: Instant,
  > ) -> Result<Reply, PaneError> {
  >     let within = budget(self.request, until.min(self.deadline));
  >     match http::request(self.port, route.method, path, body, within) {
  >         Ok(reply) => Ok(reply),
  >         Err(HttpError::Refused) => Err(PaneError::Unavailable {
  >             what: format!("the harness server on port {}", self.port),
  >         }),
  >         Err(HttpError::TimedOut) => Err(self.timeout()),
  >         Err(HttpError::Garbled(why)) => Err(PaneError::Unavailable {
  >             what: format!(
  >                 "{} {} on port {}: {}",
  >                 route.method,
  >                 route.label,
  >                 self.port,
  >                 one_line(&why)
  >             ),
  >         }),
  >     }
  > }

- **Fact:** the `why` of a garbled reply is built from the client's own texts and I/O or parser errors, never from reply bytes, so a garbled answer to `prompt_async` cannot carry the prompt into a message either (D6's "by construction").
  **Source:** `crates/holler-adapter-opencode/src/http.rs:110-116`, `:173-175`, `:290-294` and `:319-322`
  **Verbatim excerpt:**
  > /// An I/O error while `doing` something: a timeout, or a connection that broke.
  > fn failed(doing: &str, error: &std::io::Error) -> HttpError {
  >     match error.kind() {
  >         ErrorKind::WouldBlock | ErrorKind::TimedOut => HttpError::TimedOut,
  >         _ => HttpError::Garbled(format!("cannot {doing}: {error}")),
  >     }
  > }

  > Err(error) => Err(HttpError::Garbled(format!(
  >     "not an HTTP/1.x reply ({error})"
  > ))),

  > return Err(HttpError::Garbled(if nothing_yet {
  >     "the server closed the connection without a reply".to_owned()
  > } else {
  >     "the connection closed inside the reply head".to_owned()
  > }));

  > return Err(HttpError::Garbled(format!(
  >     "the connection closed before the declared length ({} of {length} bytes)",
  >     self.buf.len()
  > )));

- **Fact:** the launch engine asks `verify_serving` right after the health wait and returns its answer as it is, and its rollback passes that error on unchanged after stopping what the pane owns (the server among it), so `agent-unusable` reaches the operator as exit 3 after acting and rolled back.
  **Source:** `crates/holler-pane/src/tx_launch/server.rs:100-107` and `crates/holler-pane/src/tx_launch.rs:726-742`
  **Verbatim excerpt:**
  >         .host
  >         .run(plan.name, &plan.spec.host.cwd, &seen.server)?;
  >     // No budget check here: the wait checks the budget's end before every ask, its first included.
  >     if !wait_healthy(ports.harness, plan.port, poll, budget.deadline())? {
  >         return Err(budget.expired());
  >     }
  >     budget.check()?;
  >     ports.harness.verify_serving(plan.port, &plan.serve)

  > fn roll_back(ports: Ports<'_>, name: &PaneName, made: &Made, error: PaneError) -> PaneError {
  >     let mut failed = Vec::new();
  >     if made.session {
  >         if let Err(stop) = ports.host.stop_owned(name) {
  >             failed.push(format!("host.stop_owned ({})", stop.code()));
  >         }
  >     }
  >     if let Some(pane) = &made.pane {
  >         if let Err(close) = closed(ports.herdr.close(pane)) {
  >             failed.push(format!("herdr.close ({})", close.code()));
  >         }
  >     }
  >     if failed.is_empty() {
  >         return error;
  >     }
  >     with_note(error, &format!("rollback failed: {}", failed.join(", ")))
  > }

- **Fact:** an agent key holds only ASCII letters, digits, `-` and `_`, so `{"default_agent":KEY}` needs no escaping beyond what `serde_json` does, and the argv element never ends in `;` (the one thing the host adapter's tmux escape changes).
  **Source:** `crates/holler-pane/src/argv.rs:138-146` and `crates/holler-adapter-host/src/tmux.rs:152-157`
  **Verbatim excerpt:**
  > impl AgentKey {
  >     /// Check `text` and keep it as an agent key: non-empty, and only ASCII letters,
  >     /// digits, `-` and `_` (so no whitespace, newline, `=` or `/`). Anything else is
  >     /// [`AGENT_KEY_INVALID`], in a message that states the rule and not the text.
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
  >         if text.is_empty() || !text.chars().all(allowed) {
  >             return Err(PaneError::Refused {
  >                 code: AGENT_KEY_INVALID,

  > pub(crate) fn escape(value: &str) -> String {
  >     match value.strip_suffix(';') {
  >         Some(head) => format!("{head}\\;"),
  >         None => value.to_owned(),
  >     }
  > }

- **Fact:** the test kit's `HarnessOp::VerifyServing` is `harness.verify_serving`, the string the new `OP_VERIFY_SERVING` equals (D8); `HarnessOp` has no variant for `send_prompt` or `session_activity`.
  **Source:** `crates/holler-pane-testkit/src/harness.rs:47-63`
  **Verbatim excerpt:**
  > pub enum HarnessOp {
  >     ServeCommand,
  >     VerifyServing,
  >     Health,
  >     CreateSession,
  >     ListSessions,
  >     Abort,
  >     AttachTui,
  >     SelectSession,
  >     ShownSession,
  > }
  >
  > impl PortOp for HarnessOp {
  >     fn as_str(self) -> &'static str {
  >         match self {
  >             HarnessOp::ServeCommand => "harness.serve_command",
  >             HarnessOp::VerifyServing => "harness.verify_serving",

- **Fact (OpenCode 1.19.0, cut by the brief from the installed binary):** OpenCode refuses a `default_agent` only when it is missing, a subagent or hidden (strictly `true`), and an agent defined in config without a `mode` gets `"all"`, which is why `usable` passes `primary` and `all` and reads `hidden` as `== true`.
  **Source:** `docs/handoffs/642-brief.md:1014-1019` (E33)
  **Verbatim excerpt:**
  > mode:o.Literals(["subagent","primary","all"])
  > if(!t)t=r[n]={name:n,mode:"all",permission:a.merge(g,l),options:{},native:!1}
  > t.mode=s.mode??t.mode
  > if(!t)throw Error(`default agent "${n.default_agent}" not found`)
  > if(t.mode==="subagent")throw Error(`default agent "${n.default_agent}" is a subagent`)
  > if(t.hidden===!0)throw Error(`default agent "${n.default_agent}" is hidden`)

- **Fact (OpenCode 1.19.0, cut by the brief from the installed binary):** a session's status is `idle`, `retry` or `busy`, and setting it `idle` deletes the session's entry, which is why `running` treats "an entry whose `type` is not `idle`" as running and no entry as idle.
  **Source:** `docs/handoffs/642-brief.md:1526` and `:1528` (E32)
  **Verbatim excerpt:**
  > var ds=s.Union([s.Struct({type:s.Literal("idle")}),s.Struct({type:s.Literal("retry"),attempt:hn,message:s.String,action:rn(s.Struct({reason:s.String,provider:s.String,title:s.String,message:s.String,label:s.String,link:rn(s.String)})),next:hn}),s.Struct({type:s.Literal("busy")})]).annotate({identifier:"SessionStatus"})

  > l=n.fn("SessionStatus.get")(function*(r){return(yield*re.get(o)).get(r)??{type:"idle"}}),i=n.fn("SessionStatus.list")(function*(){return new Map(yield*re.get(o))}),h=n.fn("SessionStatus.set")(function*(r,d){let f=yield*re.get(o);if(yield*e.publish(tn.Status,{sessionID:r,status:d}),d.type==="idle"){yield*e.publish(tn.Idle,{sessionID:r}),f.delete(r);return}f.set(r,d)})
