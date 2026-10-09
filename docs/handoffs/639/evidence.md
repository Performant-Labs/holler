# Evidence: #639 the hub pane registry

Source facts that the diff and T's tests rely on but that live in unchanged code. Each excerpt is copied verbatim from the
cited lines.

## F (Phase 5, implement)

- **Fact:** The only production caller of `panes::dispatch` passes both handles as `&Arc<_>`. Widening `dispatch`'s
  `profiles` parameter to `&Arc<ProfileState>` (A's W-1) therefore compiles without editing `pane_dispatch.rs`, and
  `&deps.panes` still deref-coerces to `&PaneState`.
  **Source:** `crates/holler-hub/src/pane_dispatch.rs:52-54` and `crates/holler-hub/src/pane_dispatch.rs:82`
  **Verbatim excerpt:**
  > ```rust
  >     pub panes: Arc<PaneState>,
  >     /// The profile registry's state (#661).
  >     pub profiles: Arc<ProfileState>,
  > ```
  > ```rust
  >         panes::dispatch(method, cid, obj, &deps.panes, &deps.profiles).await
  > ```

- **Fact:** #669's seam documents that a story may widen its own `dispatch` to `&Arc<_>` to move an owned handle into
  `spawn_blocking`, without editing that file.
  **Source:** `crates/holler-hub/src/pane_dispatch.rs:71-74`
  **Verbatim excerpt:**
  > ```rust
  > /// The `Arc` fields are passed as they are. They deref-coerce to the `&PaneState` and
  > /// `&ProfileState` of today's dispatchers, and a story whose handler must move an
  > /// owned handle into `spawn_blocking` can widen its own `dispatch` to `&Arc<_>`
  > /// without editing this file.
  > ```

- **Fact:** The membership hook accepts every pane until #661 fills it, so no test can make `pane/cas_put` refuse through it
  yet. AC 27 is verified by grep.
  **Source:** `crates/holler-hub/src/profile/mod.rs:67-68`
  **Verbatim excerpt:**
  > ```rust
  > pub fn check_membership(_pane: &Pane, _profiles: &ProfileState) -> Result<(), PaneError> {
  >     Ok(())
  > ```

- **Fact:** A registry file entry is exactly a `PaneEvent`. It refuses unknown fields, and its `pane` defaults to `None`, so
  a tombstone is written as `"pane": null` and also reads back when the member is absent.
  **Source:** `crates/holler-pane/src/pane.rs:257-268`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct PaneEvent {
  >     /// The store-wide sequence number of this change; hand it back as the `since`
  >     /// of the next watch to resume without a gap or a repeat.
  >     pub cursor: Cursor,
  >     /// The pane the change concerns.
  >     pub name: PaneName,
  >     /// The record after the change; `None` when the record was deleted.
  >     #[serde(default)]
  >     pub pane: Option<Box<Pane>>,
  > }
  > ```

- **Fact:** `PaneName`, and the `SessionName` it wraps, orders as its string. The store's `BTreeMap<PaneName, _>` therefore
  yields `list` and the file's entries sorted by name.
  **Source:** `crates/holler-pane/src/pane.rs:31-32` and `crates/holler-proto/src/vocab.rs:73-74`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  > pub struct PaneName(SessionName);
  > ```
  > ```rust
  > #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  > pub struct SessionName(String);
  > ```

- **Fact:** The compare-and-swap rule, which both `cas_put` and `delete` call and neither re-implements, is a conflict when
  the generations differ and a checked increment otherwise.
  **Source:** `crates/holler-pane/src/generation.rs:25-30`
  **Verbatim excerpt:**
  > ```rust
  > pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
  >     if current != expected {
  >         return Err(PaneError::Conflict);
  >     }
  >     current
  >         .checked_add(1)
  > ```

- **Fact:** A request without `params` is decoded from `{}`, which the handler supplies. `decode_params` maps a guard's coded
  serde error back to its own code (for example `command-not-argv`), and any other decode failure to `usage`.
  **Source:** `crates/holler-pane/src/reply.rs:111-116` and `crates/holler-pane/src/error.rs:534-539`
  **Verbatim excerpt:**
  > ```rust
  > /// A request that carries no `params` decodes from `{}`, as
  > /// `holler_proto::typed_params` does: the caller passes an empty JSON object for it,
  > /// so `pane/list`, `profile/list` and the `*/watch` methods (whose `since` defaults
  > /// to `Cursor(0)`) accept a request without `params`.
  > pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError> {
  >     serde_json::from_value(params).map_err(|e| PaneError::from_decode(&e))
  > ```
  > ```rust
  >     pub(crate) fn from_decode(err: &serde_json::Error) -> PaneError {
  >         let text = err.to_string();
  >         match PaneCode::split_prefix(&text) {
  >             Some((code, rest)) => PaneError::from_closed(code, rest.to_owned(), None),
  >             None => PaneError::Usage { message: text },
  >         }
  > ```

- **Fact:** A success reply whose `data` is JSON `null` reads back as `Ok(None)`: `data` is an `Option<Value>` with a serde
  default, and `into_result` returns it unchanged. That is why `pane/get` of an unknown name and `pane/delete` both answer
  `Ok(None)` in the tests.
  **Source:** `crates/holler-pane/src/reply.rs:53-54` and `crates/holler-pane/src/reply.rs:96`
  **Verbatim excerpt:**
  > ```rust
  >     #[serde(default)]
  >     pub data: Option<Value>,
  > ```
  > ```rust
  >             (true, None) => Ok(self.data),
  > ```

- **Fact:** `reply_line` is the one place a `PaneReply` becomes a line. It is sent as a JSON-RPC result.
  **Source:** `crates/holler-hub/src/pane_dispatch.rs:98-100`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn reply_line(cid: &CorrelationId, reply: &PaneReply) -> String {
  >     // A reply is a bool and two optional values, so it always serializes.
  >     encode_response(cid, serde_json::to_value(reply).unwrap_or_default())
  > ```

- **Fact:** The save writes a temporary file in the same directory and then renames it. When the rename fails, the temporary
  file is removed. A failed save therefore leaves the old file as it was, with no `.panes.json.*.tmp` behind.
  **Source:** `crates/holler-proto/src/atomic_file.rs:66-68`
  **Verbatim excerpt:**
  > ```rust
  > pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
  >     let tmp = write_temp(path, bytes, mode, false)?;
  >     std::fs::rename(&tmp, path).inspect_err(|_| discard(&tmp))
  > ```

- **Fact:** `Severity::Error` is defined as a fault on the emitter's own side. That is why both registry events
  (`pane_registry_corrupt` and `pane_registry_write_failed`) are `error`. `emit` writes through `eprintln!`, so a test run
  captures the output.
  **Source:** `crates/holler-proto/src/log.rs:114-115` and `crates/holler-proto/src/log.rs:553`
  **Verbatim excerpt:**
  > ```rust
  > /// `Error` is a fault on the emitter's own side, such as a hub that cannot
  > /// read its own token store (issue #485), where `Warn` is a peer's failure.
  > ```
  > ```rust
  >     eprintln!("{line}");
  > ```

- **Fact:** The registry file's path is derived from the state dir: `hub_dir` is `<root>/hub`. A real hub creates that
  directory before it builds the registries' state. The tests' `temp_state()` does not create it, which is why `save_doc`
  creates the directory itself.
  **Source:** `crates/holler-hub/src/state.rs:26`, `crates/holler-hub/src/serve.rs:85` and
  `crates/holler-hub/src/serve.rs:366`
  **Verbatim excerpt:**
  > ```rust
  >         let hub_dir = root.join("hub");
  > ```
  > ```rust
  >     if let Err(e) = ensure_dirs(&state) {
  > ```
  > ```rust
  >     let pane_deps = crate::pane_dispatch::PaneDeps::load(state);
  > ```
