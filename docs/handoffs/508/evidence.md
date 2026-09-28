# Evidence — #508 (F, Phase 6/Implement)

- **Fact:** `crate::token::Record` carries `token_id: String` and `label: String` fields directly, independent of the `params.token_id` the wire request also carries — `circuit/admin.rs`'s `admin_connected`/`admin_dropped` log lines read these two fields off the `&Record` `handle_authenticated` already resolved, rather than needing `params.token_id` threaded in separately.
  **Source:** `crates/holler-hub/src/token.rs:144-148`
  **Verbatim excerpt:**
  > ```
  > pub struct Record {
  >     /// The operator-facing id: `tok_<32hex>`.
  >     pub token_id: String,
  >     /// The unique-per-hub label the token was minted for (ADR 0005).
  >     pub label: String,
  > ```

- **Fact:** `Registry` derives `Clone` (and `Default`), so cloning it into the admin loop's `AdminDeps` (and again into each spawned per-request task) is a cheap `Arc`-style clone, not a deep copy of the live-circuit table.
  **Source:** `crates/holler-hub/src/live.rs:408-409`
  **Verbatim excerpt:**
  > ```
  > #[derive(Clone, Default)]
  > pub struct Registry {
  > ```

- **Fact:** `AuthDeps` has exactly four fields — `registry`, `roster`, `peer`, `lockout` — so destructuring all four in `handle_authenticated` (`let AuthDeps { registry, roster, peer, lockout } = deps;`) is exhaustive and needed no `..` before this change, which only ever bound the first three.
  **Source:** `crates/holler-hub/src/circuit/auth.rs:48-53`
  **Verbatim excerpt:**
  > ```
  > pub struct AuthDeps<'a> {
  >     pub registry: &'a Registry,
  >     pub roster: &'a std::sync::Arc<crate::roster::Roster>,
  >     pub peer: &'a str,
  >     pub lockout: &'a std::sync::Arc<crate::lockout::Lockout>,
  > }
  > ```

- **Fact:** `hub_query_local`/`hub_query_remote` (the two handlers `admin/query` maps onto) read `params.target`/`params.method`/`params.params` off the **whole** request object passed in as `obj` (i.e. `obj.get("params")...`), not off a pre-stripped params-only value — so handing them the full decoded envelope `Value` from an `admin/query` frame (as `circuit/admin.rs::handle_request` does) is the same shape they already expect from the Unix control socket's `control/query_local`/`control/query_remote`, unchanged.
  **Source:** `crates/holler-hub/src/control_server.rs:211-213`
  **Verbatim excerpt:**
  > ```
  > async fn hub_query_local(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
  >     let Some(method) = obj.get("params").and_then(|p| p.get("method")).and_then(|v| v.as_str()) else {
  >         return encode_error(cid, Code::InvalidParams, "control/query_local needs params.method".to_string());
  > ```

- **Fact:** `connection::handshake::authenticate` is `pub(crate)` (crate-visible), already exposed for `crate::confirm` to call directly on its own short-lived connection — the new `holler_body::admin_client` module (a sibling top-level module, not a submodule of `connection`) reuses that same existing visibility; only `hello_exchange` needed widening (`pub(super)` → `pub(crate)`) in this change.
  **Source:** `crates/holler-body/src/connection/handshake.rs:55`
  **Verbatim excerpt:**
  > ```
  > pub(crate) async fn authenticate<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity, state_root: &Path) -> Result<String, Attempt>
  > ```

- **Fact:** `BodyIdentity::path` is a public fn that derives the exact `<state>/body/credential.json` path from a state root — `admin_client::load_identity`'s `NotJoined` error names this same path (via this fn) rather than hardcoding the `"body/credential.json"` join itself a second time.
  **Source:** `crates/holler-body/src/identity.rs:100-102`
  **Verbatim excerpt:**
  > ```
  >     pub fn path(state_root: &Path) -> std::path::PathBuf {
  >         state_root.join("body").join("credential.json")
  >     }
  > ```

## Rework round 1 (F, Phase 6 redo — handoff-S REWORK item 1)

- **Fact:** `holler_hub::wire::send_error_with_reason` (a different module in this same crate, `holler-hub`) already builds an unkeyed `Envelope::Error { id, error }` struct literal directly — proof that `Envelope`'s `Error` variant fields are constructible from outside the `holler-proto` crate, not only through `Envelope::error_frame` (which requires a valid `&CorrelationId` and always sets `id: Some(..)`). `circuit/admin.rs`'s new `reply_decode_error` (handoff-S item 1) reuses this exact pattern to send an unkeyed error frame when a decode-failed frame's raw JSON has no usable `id`.
  **Source:** `crates/holler-hub/src/wire.rs:33-39`
  **Verbatim excerpt:**
  > ```
  >     let frame = match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
  >         Some(cid) => Envelope::error_frame(&cid, &WireError::new(code, message, reason)),
  >         None => Envelope::Error {
  >             id: id.map(str::to_owned),
  >             error: WireError::new(code, message, reason),
  >         },
  >     };
  > ```

- **Fact:** `EnvelopeError::code()` maps `UnknownMethod` to `Code::MethodNotFound` (the `-32601` handoff-S item 1 calls for) — this mapping is unchanged source `reply_decode_error` calls into via `err.code()`, not something the rework diff itself defines.
  **Source:** `crates/holler-proto/src/envelope.rs:176-190`
  **Verbatim excerpt:**
  > ```
  >     pub fn code(&self) -> Code {
  >         match self {
  >             EnvelopeError::Json(_) => Code::ParseError,
  >             EnvelopeError::Batch | EnvelopeError::Shape | EnvelopeError::Version => {
  >                 Code::InvalidRequest
  >             }
  >             EnvelopeError::UnknownMethod(_) => Code::MethodNotFound,
  > ```
