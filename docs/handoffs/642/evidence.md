# Evidence: #642b, behaviour in unchanged code that the diff relies on

## F (Phase 5)

- **Fact:** `http::request` with no time left answers `TimedOut` before it connects, so `attach_tui` with
  `timeouts.call = 0` answers `timeout` from its first request, the existence check (AC 30(e)).
  **Source:** `crates/holler-adapter-opencode/src/http.rs:101-108`
  **Verbatim excerpt:**
  > ```rust
  > fn left(deadline: Instant) -> Result<Duration, HttpError> {
  >     let left = deadline.saturating_duration_since(Instant::now());
  >     if left.is_zero() {
  >         Err(HttpError::TimedOut)
  >     } else {
  >         Ok(left)
  >     }
  > }
  > ```

- **Fact:** the connect is the first step of a request and takes its bound from `left`, and a refused connect is
  `HttpError::Refused`.
  **Source:** `crates/holler-adapter-opencode/src/http.rs:118-121`
  **Verbatim excerpt:**
  > ```rust
  > fn connect(port: u16, deadline: Instant) -> Result<TcpStream, HttpError> {
  >     let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
  >     TcpStream::connect_timeout(&addr, left(deadline)?).map_err(|error| match error.kind() {
  >         ErrorKind::ConnectionRefused => HttpError::Refused,
  > ```

- **Fact:** every request of a `Call` maps a refusal to `unavailable` ("the harness server on port N") and a client
  timeout to `timeout` with the call's `op`. `attach_tui`'s existence check on an unbound port (case 6, AC 6) and the
  `select_session` requests rely on it.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:361-367`
  **Verbatim excerpt:**
  > ```rust
  >         let within = budget(self.request, until.min(self.deadline));
  >         match http::request(self.port, route.method, path, body, within) {
  >             Ok(reply) => Ok(reply),
  >             Err(HttpError::Refused) => Err(PaneError::Unavailable {
  >                 what: format!("the harness server on port {}", self.port),
  >             }),
  >             Err(HttpError::TimedOut) => Err(self.timeout()),
  > ```

- **Fact:** `Call::unexpected` is one line that names the route, the port and the status and quotes at most 60 bytes of
  the body. `attach_tui` uses it for a session reply without a `directory` (AC 30(d)), and `select_session` for a
  `POST /tui/select-session` reply that is not `true` (AC 29(e)).
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:386-399`
  **Verbatim excerpt:**
  > ```rust
  >     /// The `unavailable` of a reply that is not the `wanted` answer of `route`: one line
  >     /// naming the route, the port and the status, quoting at most 60 bytes of the body.
  >     fn unexpected(&self, route: Route, reply: &Reply, wanted: &str) -> PaneError {
  >         PaneError::Unavailable {
  >             what: format!(
  >                 "{} {} on port {} answered {}, not {wanted}: \"{}\"",
  >                 route.method,
  >                 route.label,
  >                 self.port,
  >                 reply.status,
  >                 excerpt(&reply.body)
  >             ),
  >         }
  >     }
  > ```

- **Fact:** `json_of` is `None` for a status other than 200 and for a body that is not JSON. So `select_session`'s check
  `json_of(&reply) != Some(Value::Bool(true))` refuses the web app's 200 HTML (AC 29(e)), and `session_reply` reads only
  a 200's object.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:511-518`
  **Verbatim excerpt:**
  > ```rust
  > /// The JSON of a `200` reply; `None` for another status or a body that is not JSON.
  > fn json_of(reply: &Reply) -> Option<Value> {
  >     if reply.status == 200 {
  >         serde_json::from_slice(&reply.body).ok()
  >     } else {
  >         None
  >     }
  > }
  > ```

- **Fact:** `one_line` turns every control character into a space. The pane id in every TUI message, tmux's first stderr
  line (`Ran::reason`) and the runner's spawn errors go through it, so each message is one line.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:540-545`
  **Verbatim excerpt:**
  > ```rust
  > /// `text` on one line: every control character becomes a space (ADR-0021 section 9).
  > fn one_line(text: &str) -> String {
  >     text.chars()
  >         .map(|c| if c.is_control() { ' ' } else { c })
  >         .collect()
  > }
  > ```

- **Fact:** a pane name, which is what the `tui_session` resolver answers, obeys ADR 0005's segment grammar (lowercase
  ASCII letters and digits, `-` only between them), so `exact_target` and the builders put it into a tmux target with no
  escape.
  **Source:** `crates/holler-pane/src/pane.rs:35-37`
  **Verbatim excerpt:**
  > ```rust
  >     /// Parse a pane name; the grammar is `SessionName::parse`'s.
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         SessionName::parse(text)
  > ```
  **Source:** `crates/holler-proto/src/vocab.rs:13-15`
  **Verbatim excerpt:**
  > ```rust
  > //! Segment grammar (ADR 0005): lowercase ASCII letters and digits only; `-`
  > //! is allowed **between** word chars, but a segment must not start or end
  > //! with one. The 32-char per-segment limit (docs §3) is enforced here.
  > ```
