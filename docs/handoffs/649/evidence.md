# Evidence — #649 hub wiring

Facts from code the diff does not change, which the wiring (`crates/holler-cli/src/pane/wiring.rs`
and `wiring/{store,env,bind}.rs`) and its tests rely on. One entry per fact; each excerpt is copied
verbatim. The diff gate shows only the first 12,000 bytes of this file, so the sections run newest
first: add a later round's entries above them.

## The source that round 2 of the diff gate asked for (T, third cycle)

Round 2 could not check that `bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists`
(`crates/holler-cli/tests/pane_integration/bind.rs`) compiles, because the signatures it relies on were not
shown. None of these files is in the diff. The test target builds and the test passes at this tree
(`cargo test -p holler-cli --test pane_integration bind::launch_resolves_`: `ok. 1 passed`).

- **Fact:** `run_verb_with` takes a `Ports<'_>` by value, with no bound of its own beyond `Ports`.
  **Source:** `crates/holler-cli/tests/verb_harness/mod.rs:54`
  **Verbatim excerpt:**
  > pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {

- **Fact:** `Ports` is seven `&dyn` references, one per port, so the test must fill all seven, including
  `profile_store`, `scope` and `prober`, which a launch's assertions do not read. Its only requirement on
  a value is that it implements the port's trait.
  **Source:** `crates/holler-pane/src/ports.rs:356-367`
  **Verbatim excerpt:**
  > /// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
  > /// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
  > #[derive(Clone, Copy)]
  > pub struct Ports<'a> {
  >     pub pane_store: &'a dyn PaneStore,
  >     pub profile_store: &'a dyn ProfileStore,
  >     pub herdr: &'a dyn HerdrPort,
  >     pub host: &'a dyn HostPort,
  >     pub harness: &'a dyn HarnessPort,
  >     pub scope: &'a dyn ProfileScope,
  >     pub prober: &'a dyn Prober,
  > }

- **Fact:** `Send + Sync` is a supertrait of `Prober` and of `ProfileScope`, so the test kit's
  `impl Prober for FakeProber` and `impl ProfileScope for FakeProfileScope` compile only if both fakes are
  `Send + Sync`. Both impls exist in the test kit, which the workspace builds.
  **Source:** `crates/holler-pane/src/ports.rs:340`, `crates/holler-pane/src/profile.rs:384`,
  `crates/holler-pane-testkit/src/prober.rs:81`, `crates/holler-pane-testkit/src/profile_scope.rs:190`
  **Verbatim excerpt:**
  > pub trait Prober: Send + Sync {
  >
  > (profile.rs)
  > pub trait ProfileScope: Send + Sync {
  >
  > (prober.rs)
  > impl Prober for FakeProber {
  >
  > (profile_scope.rs)
  > impl ProfileScope for FakeProfileScope {

- **Fact:** the constructors the test calls: `FakeProber::new()` takes nothing and scripts nothing, and
  `FakeProfileScope::new` takes the two stores as `Arc<dyn ..>` and an `Actor`. The test passes
  `profiles.clone()` and `panes.clone()` (an `Arc<FakeProfileStore>` and an `Arc<FakePaneStore>`, which
  coerce), and both fakes implement their store trait.
  **Source:** `crates/holler-pane-testkit/src/prober.rs:44-47`, `crates/holler-pane-testkit/src/profile_scope.rs:70`,
  `crates/holler-pane-testkit/src/profile_store.rs:249`, `crates/holler-pane-testkit/src/pane_store.rs:176`
  **Verbatim excerpt:**
  > impl FakeProber {
  >     /// Nothing scripted: every run answers `ProbeResult::Error` naming the argv, never
  >     /// `Ok`.
  >     pub fn new() -> Self {
  >
  > (profile_scope.rs)
  >     pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
  >
  > (profile_store.rs)
  > impl ProfileStore for FakeProfileStore {
  >
  > (pane_store.rs)
  > impl PaneStore for FakePaneStore {

## The source that round 2 of the diff gate asked for (F, third cycle)

Round 2's B-2 read the closure in `from_env` as one that borrows a local. The closure is `move`. The tree
builds, and clippy `-D warnings` is clean. With `move` removed, `cargo check -p holler-cli` fails with
`E0373` ("closure may outlive the current function, but it borrows `herdr_config`").

- **Fact:** neither `HerdrConfig` nor `PaneError` has a lifetime or a type parameter, so a
  `Result<HerdrConfig, PaneError>` holds no borrow that is not `'static`. In `from_env`,
  `move || connect_herdr(&herdr_config)` (`wiring.rs`, in the diff) takes `herdr_config` into the closure
  by value, and `&herdr_config` then borrows the closure's own field. So the closure is `'static`, as
  `LazyHerdr::new` (`bind.rs`, in the diff) requires (B-2).
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:73-74` and `crates/holler-pane/src/error.rs:402-403`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq)]
  > pub struct HerdrConfig {
  >
  > (`error.rs`, lines 402-403)
  > #[derive(Debug, Clone, PartialEq, Eq)]
  > pub enum PaneError {

## The source that round 1 of the diff gate asked for (F, second cycle)

Round 1 asked to see these sources (its NV-1, NV-2, NV-4 and NV-5) and blocked on two claims they refute
(its B-1 and B-2). The excerpt cap of that round attached none of them, so each entry below carries the lines
it relies on.

- **Fact:** a reply's `data` is optional, and `into_result` returns a success's `data` as it is. So the
  `null` the hub sends for a `get` of a missing record (E14, E15) reaches `Method::request` (`store.rs`, in
  the diff) as no data, and `request` decodes it from JSON `null`. For `get`, the decode target is the
  method's own return type, `Option<Pane>` or `Option<Profile>`, never a bare `Pane` or `Profile` (B-1).
  **Source:** `crates/holler-pane/src/reply.rs:50-57` and `crates/holler-pane/src/reply.rs:94-102`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > pub struct PaneReply {
  >     pub ok: bool,
  >     #[serde(default)]
  >     pub data: Option<Value>,
  >     #[serde(default)]
  >     pub error: Option<ReplyError>,
  > }
  >
  > (lines 94-102)
  >     pub fn into_result(self) -> Result<Option<Value>, PaneError> {
  >         match (self.ok, self.error) {
  >             (true, None) => Ok(self.data),
  >             (false, Some(e)) => Err(PaneError::from_wire(e.code, e.message, e.detail)),
  >             (true, Some(_)) | (false, None) => Err(PaneError::Unavailable {
  >                 what: "the reply is malformed: `ok` and `error` disagree".to_owned(),
  >             }),
  >         }
  >     }

- **Fact:** decoding JSON `null` as an `Option<T>` gives `Ok(None)`. Serde's `Option<T>` asks the
  deserializer for an option, and `serde_json`'s `Value` answers `Null` with `visit_none` (B-1). This is
  library source at the versions `Cargo.lock` pins (`serde_core` 1.0.229, `serde_json` 1.0.151), outside
  this repository, so the gate cannot attach it.
  **Source:** `serde_core-1.0.229/src/de/impls.rs:916-927` and `serde_json-1.0.151/src/value/de.rs:317-325`
  **Verbatim excerpt:**
  > impl<'de, T> Deserialize<'de> for Option<T>
  > where
  >     T: Deserialize<'de>,
  > {
  >     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  >     where
  >         D: Deserializer<'de>,
  >     {
  >         deserializer.deserialize_option(OptionVisitor {
  >             marker: PhantomData,
  >         })
  >     }
  >
  > (`serde_json`, lines 317-325, in `impl<'de> serde::Deserializer<'de> for Value` of line 282)
  >     fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
  >     where
  >         V: Visitor<'de>,
  >     {
  >         match self {
  >             Value::Null => visitor.visit_none(),
  >             _ => visitor.visit_some(self),
  >         }
  >     }

- **Fact:** the hub decodes the params of `pane/list` and `profile/list` as `NoParams`, an empty struct
  that refuses any member and is documented as taking "only `{}`". It is `pub(crate)` to the hub, and
  `holler_pane::reply` has no params struct for a list: its params structs are the nine quoted last. So the
  client's `json!({})` is the documented form (B-2, NV-2).
  **Source:** `crates/holler-hub/src/panes/handlers.rs:37-41`, `crates/holler-hub/src/panes/handlers.rs:96-99`, `crates/holler-hub/src/profile/handlers.rs:36-39` and `crates/holler-pane/src/reply.rs:122-195`
  **Verbatim excerpt:**
  > /// The params of a method that takes none (`pane/list`, and `profile/list` from
  > /// `profile/`, #661): only `{}`.
  > #[derive(Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub(crate) struct NoParams {}
  >
  > (lines 96-99)
  > /// `pane/list`: every record, sorted by name.
  > pub(crate) async fn list(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
  >     run(cid, obj, move |_: NoParams| store.list()).await
  > }
  >
  > (`profile/handlers.rs`, lines 36-39)
  > /// `profile/list`: every record.
  > pub(crate) async fn list(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
  >     run(cid, obj, move |_: NoParams| store.list()).await
  > }
  >
  > (`reply.rs`, lines 122, 129, 137, 146, 163, 170, 179, 188 and 195: every `pub struct` named `...Params`)
  > pub struct PaneGetParams {
  > pub struct PaneCasPutParams {
  > pub struct PaneDeleteParams {
  > pub struct WatchParams {
  > pub struct ProfileGetParams {
  > pub struct ProfileCasPutParams {
  > pub struct ProfileDeleteParams {
  > pub struct ProfileLogParams {
  > pub struct ProfileRenameParams {

- **Fact:** the hub reads a request's absent or `null` `params` as `{}` before it decodes them, so a
  request with no params and one with `{}` are the same request. `decode_params`' doc names `pane/list`
  and `profile/list` as accepting a request without `params` this way (B-2).
  **Source:** `crates/holler-hub/src/panes/handlers.rs:51`, `crates/holler-hub/src/panes/handlers.rs:61-68` and `crates/holler-pane/src/reply.rs:111-117`
  **Verbatim excerpt:**
  >     let reply = match decode_params::<P>(params_of(obj)) {
  >
  > (lines 61-68)
  > /// The request's `params`, with an absent member or `null` read as `{}` (see
  > /// `holler_pane::decode_params`).
  > fn params_of(obj: &Value) -> Value {
  >     match obj.get("params") {
  >         None | Some(Value::Null) => Value::Object(Map::new()),
  >         Some(params) => params.clone(),
  >     }
  > }
  >
  > (`reply.rs`, lines 111-117)
  > /// A request that carries no `params` decodes from `{}`, as
  > /// `holler_proto::typed_params` does: the caller passes an empty JSON object for it,
  > /// so `pane/list`, `profile/list` and the `*/watch` methods (whose `since` defaults
  > /// to `Cursor(0)`) accept a request without `params`.
  > pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError> {
  >     serde_json::from_value(params).map_err(|e| PaneError::from_decode(&e))
  > }

- **Fact:** the first case of each store conformance suite is a `get` of a name never stored, which must
  answer `Ok(None)`, and each suite runs every case of its table. `clients.rs` (in the diff) runs both
  suites over the two clients against a real hub, a fresh one per case, so B-1's failure would fail them;
  both pass (NV-1).
  **Source:** `crates/holler-pane-testkit/src/conformance/pane_store.rs:43-44`, `crates/holler-pane-testkit/src/conformance/pane_store.rs:89-90`, `crates/holler-pane-testkit/src/conformance/pane_store.rs:123-131`, `crates/holler-pane-testkit/src/conformance/profile_store.rs:94-95` and `crates/holler-pane-testkit/src/conformance/profile_store.rs:187-195`
  **Verbatim excerpt:**
  > const CASES: [(&str, Case); 19] = [
  >     ("get-missing-is-none", get_missing_is_none),
  >
  > (lines 89-90)
  > /// Run every case of [`pane_store_cases`], in order, each against a fresh, empty
  > /// store, and return every case that did not hold.
  >
  > (lines 123-131)
  > /// Case 1: `get` of a name never stored is `Ok(None)`.
  > fn get_missing_is_none(store: &dyn PaneStore) -> Result<(), String> {
  >     let missing = pane_name(C1)?;
  >     expect_eq(
  >         "get of a name never stored",
  >         succeeds("get", store.get(&missing))?,
  >         None,
  >     )
  > }
  >
  > (`profile_store.rs`, lines 94-95)
  > const CASES: [(&str, Case); 23] = [
  >     ("get-missing-is-none", get_missing_is_none),
  >
  > (`profile_store.rs`, lines 187-195)
  > /// Case 1: `get` of a name never stored is `Ok(None)`.
  > fn get_missing_is_none(store: &dyn ProfileStore) -> Result<(), String> {
  >     let missing = profile_name(ALPHA)?;
  >     expect_eq(
  >         "get of a name never stored",
  >         succeeds("get", store.get(&missing))?,
  >         None,
  >     )
  > }

- **Fact:** `HarnessPort` has eleven methods, and `send_prompt` and `session_activity` are the only two
  with a default body: a `prompt-unsupported` refusal, and `Idle`. A wrapper that left either to its
  default would refuse every prompt, or report every session idle. The `Wired` harness (`bind.rs`, in the
  diff) forwards all eleven (W-4, NV-4).
  **Source:** `crates/holler-pane/src/ports.rs:230-310`
  **Verbatim excerpt:** (lines 230, 239-244, 265, 268, 271, 274, 277, 280, 283, 286, 293-299 and 306-310; the
  doc comments between the methods are left out)
  > pub trait HarnessPort: Send + Sync {
  >     fn serve_command(
  >         &self,
  >         name: &PaneName,
  >         port: u16,
  >         spec: &ServeSpec,
  >     ) -> Result<Argv, PaneError>;
  >     fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError>;
  >     fn health(&self, port: u16) -> Result<bool, PaneError>;
  >     fn create_session(&self, port: u16) -> Result<String, PaneError>;
  >     fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError>;
  >     fn abort(&self, port: u16, session: &str) -> Result<(), PaneError>;
  >     fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError>;
  >     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;
  >     fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
  >     fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
  >         let _ = (port, session, text);
  >         Err(PaneError::Refused {
  >             code: PROMPT_UNSUPPORTED,
  >             message: "this harness does not queue a prompt".to_owned(),
  >         })
  >     }
  >     fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
  >         let _ = (port, session);
  >         Ok(Activity::Idle)
  >     }
  > }

- **Fact:** the hub's `pane/cas_put` runs the membership check before it writes. A pane that names a
  profile is stored only when that profile exists (`profile-not-found` otherwise), and a pane outside any
  profile is not checked. That is why `clients.rs` (in the diff) gives the hub of each pane-suite case the
  two profiles that its case `pane-in-other-profile` moves a pane between (NV-5).
  **Source:** `crates/holler-hub/src/panes/handlers.rs:101-116` and `crates/holler-hub/src/profile/mod.rs:206-216`
  **Verbatim excerpt:**
  > /// `pane/cas_put`: the stored record. The membership check runs first, on the blocking
  > /// pool and outside the pane lock (D1, D9). It runs off the executor because #661's check
  > /// reads the profile registry through its blocking port. A refusal is the reply, and
  > /// nothing is written.
  > pub(crate) async fn cas_put(
  >     cid: &CorrelationId,
  >     obj: &Value,
  >     store: Arc<Store>,
  >     profiles: Arc<ProfileState>,
  > ) -> String {
  >     run(cid, obj, move |params: PaneCasPutParams| {
  >         check_membership(&params.pane, &profiles)?;
  >         store.cas_put(&params.pane, params.expected_generation)
  >     })
  >     .await
  > }
  >
  > (`profile/mod.rs`, lines 206-216)
  > pub fn check_membership(pane: &Pane, profiles: &ProfileState) -> Result<(), PaneError> {
  >     let Some(name) = &pane.profile else {
  >         return Ok(());
  >     };
  >     match profiles.store.get(name)? {
  >         Some(_) => Ok(()),
  >         None => Err(PaneError::ProfileNotFound {
  >             what: name.to_string(),
  >         }),
  >     }
  > }

- **Fact:** the Herdr adapter refuses an `ensure_pane` into a workspace its config does not list, as
  `unavailable`, before it sends a request of its own. So a `HOLLER_HERDR_WORKSPACES` that is not UTF-8,
  which `connect` reads as the empty value (no workspace; `wiring.rs` and `env.rs`, in the diff), fails
  every launch closed, and no Herdr pane is made.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:345-347`, `crates/holler-adapter-herdr/src/adapter.rs:145-154`, `crates/holler-adapter-herdr/src/adapter.rs:221-224` and `crates/holler-adapter-herdr/src/adapter.rs:177-199`
  **Verbatim excerpt:**
  >     fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
  >         self.run_as(OP_ENSURE_PANE, |deadline| self.ensure(spec, deadline))
  >     }
  >
  > (lines 145-154: `run_as` only fixes the deadline and maps a timeout's `op`)
  >     fn run_as<V>(
  >         &self,
  >         op: &'static str,
  >         body: impl FnOnce(Instant) -> Result<V, PaneError>,
  >     ) -> Result<V, PaneError> {
  >         body(self.deadline()?).map_err(|error| match error {
  >             PaneError::Timeout { .. } => PaneError::Timeout { op: op.to_owned() },
  >             other => other,
  >         })
  >     }
  >
  > (lines 221-224)
  >     /// `ensure_pane` against `deadline`: plan the one step that makes `spec.grid` (an
  >     /// occupied cell needs none), run it, and confirm the new pane sits there.
  >     fn ensure(&self, spec: &HerdrSpec, deadline: Instant) -> Result<HerdrPane, PaneError> {
  >         let extent = self.extent_of(spec)?;
  >
  > (lines 177-199)
  >     /// The configured extent of `spec`'s workspace, once `spec` is in the session
  >     /// served. Neither check sends a request.
  >     fn extent_of(&self, spec: &HerdrSpec) -> Result<Extent, PaneError> {
  >         if spec.session != self.config.session {
  >             return Err(PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr session {:?}: this adapter serves only the session {:?}",
  >                     spec.session, self.config.session
  >                 ),
  >             });
  >         }
  >         self.config
  >             .workspaces
  >             .get(&spec.workspace)
  >             .copied()
  >             .ok_or_else(|| PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr workspace {:?} of session {:?} is not configured, so its rows and \
  >                      columns are not known",
  >                     spec.workspace, spec.session
  >                 ),
  >             })
  >     }

## The wiring's own facts (F, Phase 5)

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
