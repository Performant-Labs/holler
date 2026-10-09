# Evidence — #669 (F, Phase 6/Implement)

- **Fact:** `encode_response` wraps any JSON value as the JSON-RPC **result** of the request `cid`, and it is `pub(crate)`: `pane_dispatch::reply_line` carries a `PaneReply` as a result through it, the way `control_hold.rs` already calls it.
  **Source:** `crates/holler-hub/src/control_server.rs:705-708`
  **Verbatim excerpt:**
  > ```
  > pub(crate) fn encode_response(cid: &holler_proto::CorrelationId, result: serde_json::Value) -> String {
  >     let env = Envelope::response(cid, Some(result));
  >     holler_proto::encode(&env).unwrap_or_default()
  > }
  > ```

- **Fact:** `encode_error` builds a JSON-RPC **error** frame from a closed wire `Code` (the convention the pane methods deliberately do not use for domain outcomes). `pane_dispatch::forward` uses it only for its defensive not-in-either-list branch, with `Code::MethodNotFound`, like `dispatch_session_control`.
  **Source:** `crates/holler-hub/src/control_server.rs:710-713`
  **Verbatim excerpt:**
  > ```
  > pub(crate) fn encode_error(cid: &holler_proto::CorrelationId, code: Code, message: String) -> String {
  >     let env = Envelope::error_frame(cid, &WireError::new(code, message, None));
  >     holler_proto::encode(&env).unwrap_or_default()
  > }
  > ```

- **Fact:** `PaneReply::failure` builds `ok: false` with no `data` and an error whose `code` is the `PaneError`'s own code, so the stubs' reply is exactly `{ok:false, data:null, error:{code, message}}` and parses back through `into_result`.
  **Source:** `crates/holler-pane/src/reply.rs:71-81`
  **Verbatim excerpt:**
  > ```
  >     pub fn failure(error: &PaneError) -> Self {
  >         Self {
  >             ok: false,
  >             data: None,
  >             error: Some(ReplyError {
  >                 code: error.code().to_owned(),
  >                 message: error.to_string(),
  >                 detail: error.detail().map(str::to_owned),
  >             }),
  >         }
  >     }
  > ```

- **Fact:** `PaneError::NotImplemented` has the closed wire code `not-implemented` (so `PaneReply::into_result` rebuilds `Err(PaneError::NotImplemented)` from it, which the tests assert).
  **Source:** `crates/holler-pane/src/error.rs:95`
  **Verbatim excerpt:**
  > ```
  >             PaneCode::NotImplemented => "not-implemented",
  > ```

- **Fact:** `PaneError::NotImplemented` classifies as the closed code `PaneCode::NotImplemented`.
  **Source:** `crates/holler-pane/src/error.rs:414`
  **Verbatim excerpt:**
  > ```
  >             PaneError::NotImplemented => Ok(PaneCode::NotImplemented),
  > ```

- **Fact:** `is_pane_method` and `is_profile_method` match the exact method lists, not a `pane/` or `profile/` prefix, so `pane/frobnicate` is in neither and still falls through to `method_not_found`.
  **Source:** `crates/holler-proto/src/methods.rs:135-143`
  **Verbatim excerpt:**
  > ```
  > /// Whether `method` is one of [`PANE_METHODS`].
  > pub fn is_pane_method(method: &str) -> bool {
  >     PANE_METHODS.contains(&method)
  > }
  >
  > /// Whether `method` is one of [`PROFILE_METHODS`].
  > pub fn is_profile_method(method: &str) -> bool {
  >     PROFILE_METHODS.contains(&method)
  > }
  > ```
