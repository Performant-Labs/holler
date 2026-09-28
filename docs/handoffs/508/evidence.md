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

## T re-entry (Phase 6 verify/GREEN, addressing handoff-S REWORK item 4)

- **Fact:** `circuit.rs`'s body-role session loop already refuses any decoded-but-unrecognised `Envelope::Request` with `-32601` (`Code::MethodNotFound`) via its own trailing catch-all match arm in `handle_inbound` — unchanged by this feature. `body_socket_sending_admin_roster_gets_method_not_found` (AC 5, second bullet) relies on this existing arm to prove a body-role socket sending `admin/roster` is refused, without needing any new hub-side role check.
  **Source:** `crates/holler-hub/src/circuit.rs:869-872`
  **Verbatim excerpt:**
  > ```
  >             Envelope::Request { id, .. } => {
  >                 send_error(self.sink, Some(id), Code::MethodNotFound, "unknown method").await;
  >                 Ok(())
  >             }
  > ```

## T round-2 (test-only rework, #678: AC 3/4/6/7/8/10/11/13/16 coverage)

- **Fact:** the roster `Row`'s `last_seen: u64` field is a plain epoch-seconds clock, touched by `Registry::mark_connected`/`touch`/etc. — `remote_admin_traffic_never_moves_the_bodys_last_seen` (AC 4) reads it straight off `roster --json` and asserts it is bit-identical before/after admin traffic, relying on this field being the one and only clock the "must never move" claim is about.
  **Source:** `crates/holler-hub/src/roster.rs:162`
  **Verbatim excerpt:**
  > ```
  >     pub last_seen: u64,
  > ```

- **Fact:** `refusal_words` gives the exact plain-words prefix for each counted `-32002` reason code this rework's AC 7/13 tests assert on verbatim: `token_unknown` → "this hub has no such token…", `token_not_bound` → "this hub no longer accepts this body's token…", and `no_public_key`/`key_mismatch` → "this body's key does not match the key the hub registered at join…". These strings are produced by the existing body-path handshake code the admin client reuses unchanged (MO 7), not anything this rework's tests or F's admin-path code define themselves.
  **Source:** `crates/holler-body/src/connection/handshake.rs:182-193`
  **Verbatim excerpt:**
  > ```
  > fn refusal_words(code: &str) -> &'static str {
  >     match code {
  >         TOKEN_NOT_BOUND_REASON => {
  >             "this hub no longer accepts this body's token: it was revoked, or never joined; mint a new token and re-run `body join`"
  >         }
  >         TOKEN_UNKNOWN_REASON => {
  >             "this hub has no such token: it was deleted, or this body is joined to a different hub; mint a new token and re-run `body join`"
  >         }
  >         NO_PUBLIC_KEY_REASON | KEY_MISMATCH_REASON => "this body's key does not match the key the hub registered at join; re-run `body join`",
  > ```

- **Fact:** `token_cmd::revoke` calls `holler_hub::control::revoke_live(id)` — a revoked token's *already-live* connection (this rework's `remote_admin_revoked_token_is_refused_and_counted_in_lockout` joins and connects a real body **before** revoking its token) is actively torn down by `hub token revoke` itself, not left dangling until its next reconnect attempt.
  **Source:** `crates/holler-cli/src/token_cmd.rs:248`
  **Verbatim excerpt:**
  > ```
  >     let _ = holler_hub::control::revoke_live(id);
  > ```

- **Fact:** `talk::say`'s reply `message_id` is `format!("m-{request_id}")`, where `request_id` is a freshly minted `CorrelationId` per call — so it is never stable across two otherwise-identical `say` invocations, local or remote. `remote_say_wait_and_hub_query_json_match_local_after_masking` (AC 10) masks `message.messageId` on this basis, per the brief's own rule for a newly discovered volatile field.
  **Source:** `crates/holler-hub/src/talk.rs:458`
  **Verbatim excerpt:**
  > ```
  >         message_id: format!("m-{request_id}"),
  > ```

- **Fact:** `circuit/admin.rs`'s own `frame_outcome` treats **any** inbound WS `Ping`/`Pong`/`Frame` as `FrameOutcome::Continue { fresh_frame: now }`, which refreshes `last_frame_at` exactly like a real request would — so a raw test socket that *reads* during the idle window (even only to observe a keepalive Ping) has this WS client library auto-answer with a Pong, which itself refreshes the hub's liveness clock and would prevent the idle-close AC 16(c) pins. `idle_admin_socket_is_closed_within_the_liveness_timeout` therefore reads nothing at all until after sleeping past the configured timeout.
  **Source:** `crates/holler-hub/src/circuit/admin.rs:159`
  **Verbatim excerpt:**
  > ```
  >         Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => FrameOutcome::Continue { fresh_frame: now },
  > ```

- **Fact:** `stub-acp`'s streamed chunk text is the fixed, deterministic `format!("stub chunk {index}")` — no timestamp, no random id — so two identical `say` turns (same chunk count) against the same session always produce byte-identical `text`/`message.parts[].text`, which is what lets `remote_say_wait_and_hub_query_json_match_local_after_masking` (AC 10) compare local vs. remote `say --json` for equality once only `elapsed_ms`/`message.messageId` are masked.
  **Source:** `crates/holler-cli/tests/stub-acp/main.rs:798`
  **Verbatim excerpt:**
  > ```
  >                         "content": { "type": "text", "text": format!("stub chunk {index}") }
  > ```
