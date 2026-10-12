# Evidence: #648, the roster shows pane, SHOWN and DRIVEN

Source facts the change relies on that the diff does not show. Each excerpt is verbatim from the worktree at
`4bc2088` plus F's uncommitted change (none of these files is edited by F).

## F (Phase 6, implement)

- **Fact:** `pane list`'s row turns a record into nine text cells, one per entry of `COLUMNS`, in `COLUMNS` order, and
  `COLUMNS` holds the roster's six column names among its nine. The roster takes each of its six cells at its name's
  position in `COLUMNS`.
  **Source:** `crates/holler-cli/src/pane/list.rs:34-36`, `crates/holler-cli/src/pane/list.rs:268-280`
  **Verbatim excerpt:**
  > ```rust
  > pub const COLUMNS: [&str; 9] = [
  >     "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
  > ];
  > ```
  > ```rust
  >     pub fn cells(&self) -> [String; COLUMNS.len()] {
  >         [
  >             text_value(self.name.as_str()),
  >             self.pos.to_string(),
  >             optional_text(self.profile.as_ref().map(ProfileName::as_str)),
  >             text_value(&self.project),
  >             health_word(&self.health).to_owned(),
  >             optional_text(self.shown.as_deref()),
  >             optional_text(self.driven.as_deref()),
  >             self.sync.text().to_owned(),
  >             hold_word(&self.hold).to_owned(),
  >         ]
  >     }
  > ```

- **Fact:** `PaneRow`'s JSON keys are its fields; an empty value is `null`, `pos` is `GridPos`'s serde form and `health`
  is `Health`'s. The roster's six JSON values are these fields (`name` as `pane`), serialized the same way.
  **Source:** `crates/holler-cli/src/pane/list.rs:224-238`
  **Verbatim excerpt:**
  > ```rust
  > /// One pane in summary: a row of `pane list`, and the `pane` of a `pane watch` put.
  > ///
  > /// Its JSON keys are its fields, in this order. An empty value is `null`, never left out.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
  > pub struct PaneRow {
  >     pub name: PaneName,
  >     /// The grid cell, row first, in `GridPos`'s own serde form.
  >     pub pos: GridPos,
  >     pub profile: Option<ProfileName>,
  >     /// The directory the pane works in (`host.cwd`).
  >     pub project: String,
  >     /// What the harness server last reported, in `Health`'s serde form.
  >     pub health: Health,
  >     /// The session the pane's TUI showed when reconcile last observed it.
  >     pub shown: Option<String>,
  > ```

- **Fact:** a `--profile` value that is not a valid profile name is `usage` (exit 2); the roster reuses this parser.
  **Source:** `crates/holler-cli/src/pane/list.rs:218-222`
  **Verbatim excerpt:**
  > ```rust
  > /// The `--profile` a verb is scoped to, typed: `usage` (exit 2) when it is not a valid
  > /// profile name.
  > pub fn profile_name(opt: &ProfileOpt) -> Result<Option<ProfileName>, PaneError> {
  >     opt.profile.as_deref().map(ProfileName::parse).transpose()
  > }
  > ```

- **Fact:** `--profile P` with no pane named resolves to P's stored profile (`profile-not-found` when there is none)
  and every pane whose record names P, in name order; the roster's join set under `--profile` is this list, and the
  rows keep the hub reply's order.
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:77-90`, `crates/holler-cli/src/pane/profile_scope.rs:160-169`
  **Verbatim excerpt:**
  > ```rust
  >     fn stored(&self, profile: &ProfileName) -> Result<Profile, PaneError> {
  >         let stored = self.profiles.get(profile)?;
  >         stored.ok_or_else(|| PaneError::ProfileNotFound {
  >             what: profile.to_string(),
  >         })
  >     }
  >
  >     /// Every pane whose record names `profile` (by slug), in name order.
  >     fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
  >         let mut members = self.panes.list()?;
  >         members.retain(|pane| belongs(pane, profile));
  >         members.sort_by(|x, y| x.name.cmp(&y.name));
  >         Ok(members)
  >     }
  > ```
  > ```rust
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
  > ```

- **Fact:** the roster's pair rule is the one `say --pane` applies to a recorded pair: a mismatch only when both
  `shown` and `driven` are known and differ.
  **Source:** `crates/holler-cli/src/say_cmd.rs:357-364`
  **Verbatim excerpt:**
  > ```rust
  >     } else {
  >         let seen = &record.last_observed;
  >         let (Some(shown), Some(driven)) = (seen.shown.as_deref(), seen.driven.as_deref()) else {
  >             return None;
  >         };
  >         if shown == driven {
  >             return None;
  >         }
  > ```

- **Fact:** `--format=json` and a bare `--json` resolve to the same `Format::Json`; only an explicit `--format=json`
  sets `json_explicit`, which is what the roster reads to print the envelope.
  **Source:** `crates/holler-cli/src/output.rs:64-76`
  **Verbatim excerpt:**
  > ```rust
  > pub fn resolve_format(json: bool, format: Option<Format>) -> Result<FormatChoice, ErrorBody> {
  >     match (json, format) {
  >         (true, Some(Format::Text)) => {
  >             Err(usage_body("--json cannot be combined with --format=text"))
  >         }
  >         (_, Some(format)) => Ok(FormatChoice {
  >             format,
  >             json_explicit: format == Format::Json,
  >         }),
  >         (true, None) => Ok(FormatChoice {
  >             format: Format::Json,
  >             json_explicit: false,
  >         }),
  > ```

- **Fact:** `emit_error` in text mode writes `error: <message>` to stderr and nothing to stdout (the bytes
  `print_leaf_result` wrote before); in JSON mode it writes one failure envelope to stdout. The exit code is the class
  of the error's code.
  **Source:** `crates/holler-cli/src/output.rs:241-243`, `crates/holler-cli/src/output.rs:281-284`,
  `crates/holler-cli/src/output.rs:288-299`, `crates/holler-cli/src/output.rs:336-340`
  **Verbatim excerpt:**
  > ```rust
  > pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
  >     emit(sink, format, Err::<(), _>(error), |()| String::new())
  > }
  > ```
  > ```rust
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  >         }
  > ```
  > ```rust
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
  >         }
  > ```
  > ```rust
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }
  > ```

- **Fact:** `emit`'s text mode adds a newline only when the text lacks one, and flushes; the roster's table always ends
  in one, so its bytes are the table's. A failed write turns exit 0 into 1.
  **Source:** `crates/holler-cli/src/output.rs:318-334`
  **Verbatim excerpt:**
  > ```rust
  > fn write_line(to: &mut dyn Write, line: &str) -> io::Result<()> {
  >     to.write_all(line.as_bytes())?;
  >     if !line.ends_with('\n') {
  >         to.write_all(b"\n")?;
  >     }
  >     to.flush()
  > }
  >
  > /// The exit code of a verb that wrote `written`: `code`, except that output nobody received
  > /// cannot be a success.
  > fn settle(written: io::Result<()>, code: i32) -> i32 {
  >     if written.is_err() && code == 0 {
  >         1
  >     } else {
  >         code
  >     }
  > }
  > ```

- **Fact:** a well-formed code outside the closed set is a refusal (exit 3), so `remote-policy-refused` keeps the
  `--server` policy refusal at exit 3 under `--format=json`.
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

- **Fact:** `RefusalCode::from_static` is checked when the constant is evaluated: a malformed or closed code fails the
  build, so `REMOTE_POLICY_REFUSED` cannot panic at run time.
  **Source:** `crates/holler-pane/src/error.rs:331-337`
  **Verbatim excerpt:**
  > ```rust
  >     pub const fn from_static(code: &'static str) -> Self {
  >         assert!(
  >             is_valid_code(code) && !is_closed_code(code),
  >             "RefusalCode::from_static: not a kebab-case code, or a closed code"
  >         );
  >         Self(Cow::Borrowed(code))
  >     }
  > ```

- **Fact:** with `--server`, the transport refuses a malformed address or a plaintext `ws://` to a host that is not
  loopback before any dial, as `RemotePolicyRefused`, whose text is the message the roster has always printed.
  **Source:** `crates/holler-cli/src/transport.rs:40-45`, `crates/holler-hub/src/control.rs:52`
  **Verbatim excerpt:**
  > ```rust
  > fn remote(server: &str, call: &ControlCall) -> Result<serde_json::Value, ControlError> {
  >     let addr = holler_body::server_address::parse(server)
  >         .map_err(|e| ControlError::RemotePolicyRefused(format!("bad --server address: {e}")))?;
  >     if let Some(reason) = holler_body::server_address::loopback_only_check(&addr) {
  >         return Err(ControlError::RemotePolicyRefused(reason.to_string()));
  >     }
  > ```
  > ```rust
  >             ControlError::RemotePolicyRefused(m) | ControlError::RemoteUnavailable(m) => write!(f, "{m}"),
  > ```

- **Fact:** `Wiring::connect`, the roster's deferred source in `main.rs`, makes one `control/status` exchange with the
  local hub (any failure is `unavailable`) and then builds the ports from the environment with no adapter I/O; the
  pane store it lends is the hub's registry over the control socket.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:108-115`, `crates/holler-cli/src/pane/wiring/store.rs:118-132`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn connect() -> Result<Self, PaneError> {
  >         store::reach_hub()?;
  >         let actor = Actor::parse(ACTOR)?;
  >         let lookup = |name: &str| {
  >             std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
  >         };
  >         Ok(Self::from_env(&lookup, actor))
  >     }
  > ```
  > ```rust
  > pub(crate) fn reach_hub() -> Result<(), PaneError> {
  >     let status = ControlCall::status();
  >     crate::transport::call(None, &status)
  >         .map(drop)
  >         .map_err(|error| hub_unavailable(status.method, &error))
  > }
  >
  > impl PaneStore for HubPaneStore {
  >     fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
  >         PANE_GET.request(&PaneGetParams { name: name.clone() })
  >     }
  >
  >     fn list(&self) -> Result<Vec<Pane>, PaneError> {
  >         PANE_LIST.request(&json!({}))
  >     }
  > ```

- **Fact:** the hub's `control/roster` reply is `{"rows": rows}`, built on each call, so `roster --json` prints an
  object whose one key is `rows`.
  **Source:** `crates/holler-hub/src/control_server.rs:471-474`
  **Verbatim excerpt:**
  > ```rust
  >     encode_response(
  >         cid,
  >         serde_json::json!({ "rows": rows }),
  >     )
  > ```

## T (Phase 7, verify)

Source facts the tests (`roster_panes_test` and its modules) rely on that live in unchanged code. Excerpts copied by T
from the worktree at `f7f2bcf`.

- **Fact:** the fixture rows are the hub's own `Row`, serialized, so the reply the tests hand `render` has the real
  reply's keys; a session hold is flattened into the row and absent when the session is not held (the tests seed no
  hold, W-6).
  **Source:** `crates/holler-hub/src/roster.rs:137-140`, `crates/holler-hub/src/roster.rs:174-179`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, PartialEq, serde::Serialize)]
  > pub struct Row {
  >     /// The routable name (`label/session`).
  >     pub name: String,
  > ```
  > ```rust
  >     /// The session hold (issue #442), filled in from the hub's hold registry
  >     /// when the row is read by `control/roster`; the roster itself never holds
  >     /// it (a hold belongs to the session name, not to a row). Flattened, and
  >     /// absent for a session that is not held.
  >     #[serde(flatten)]
  >     pub hold: holler_proto::SessionHold,
  > ```

- **Fact:** `check_envelope` accepts a failure envelope only when the error code's class agrees with the exit code, so
  `hubless::a_server_refused_by_policy_exits_3_in_every_mode` passing at exit 3 proves the roster's code is a
  refusal-class code, and `with_no_hub_...` at exit 1 a failure-class one.
  **Source:** `crates/holler-pane-testkit/src/envelope.rs:202-203`, `crates/holler-pane-testkit/src/envelope.rs:342-347`
  **Verbatim excerpt:**
  > ```rust
  > pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
  >     check_exit_code(exit_code)?;
  > ```
  > ```rust
  >     let [code, message] = exact_members(error, ERROR_KEYS, "error.")?;
  >     let code = kebab_code(code)?;
  >     let message = one_line_message(message)?;
  >     if class_of(&code).exit_code() != exit_code {
  >         return Err(EnvelopeFault::ClassDisagreesWithExit);
  >     }
  > ```

- **Fact:** the test kit's `FakeProfileScope` resolves `--profile P` with no pane to P's members in name order, as the
  real scope does, so the `profile::` tests' expected row order (the reply's, not the store's) discriminates.
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:105-115`, `crates/holler-pane-testkit/src/profile_scope.rs:191-200`
  **Verbatim excerpt:**
  > ```rust
  >     /// Every pane whose record names `profile`, in name order.
  >     fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
  >         let mut members: Vec<Pane> = self
  >             .panes
  >             .list()?
  >             .into_iter()
  >             .filter(|pane| belongs(pane, profile))
  >             .collect();
  >         members.sort_by(|x, y| x.name.cmp(&y.name));
  >         Ok(members)
  >     }
  > ```
  > ```rust
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
  > ```

- **Fact:** the message the policy-refusal tests expect as "today's line" is the transport's fixed loopback rule.
  **Source:** `crates/holler-body/src/server_address.rs:159-164`
  **Verbatim excerpt:**
  > ```rust
  > pub fn loopback_only_check(addr: &ServerAddress) -> Option<&'static str> {
  >     if addr.scheme == "ws" && !addr.is_loopback() {
  >         Some(
  >             "a plaintext ws:// to a non-loopback host is refused; \
  >              off-loopback pairing is wss:// (ADR 0002)",
  >         )
  > ```

- **Fact:** `registry::the_mismatch_rule_is_say_pane_targets_rule` compares the roster with `say --pane`'s public
  resolver, which takes the ports, a pane name and an optional profile.
  **Source:** `crates/holler-cli/src/say_cmd.rs:301-305`
  **Verbatim excerpt:**
  > ```rust
  > pub fn resolve_pane_target(
  >     ports: Ports<'_>,
  >     pane: &str,
  >     profile: Option<&str>,
  > ) -> Result<String, PaneError> {
  > ```
