# Evidence: #851, `holler --version` prints the commit the binary was built from

Source facts in unchanged code that the diff and T's tests rely on. F wrote the entries below at Phase 5;
T appends its own at Phase 6.

## F (Phase 5)

- **Fact:** `hub status` (text) prints the `version` field of the live hub's status document as is, so its
  first line carries the commit once `control/status` does, with no change to `hub_cmd.rs`.
  **Source:** `crates/holler-cli/src/hub_cmd.rs:47-49`
  **Verbatim excerpt:**
  > ```rust
  >                 let version = doc.get("version").and_then(|v| v.as_str()).unwrap_or("?");
  >                 let protocol = doc.get("protocol").and_then(|v| v.as_u64()).map_or_else(|| "?".to_string(), |p| p.to_string());
  >                 println!("hub {version} (protocol {protocol})");
  > ```

- **Fact:** the precedent the new `holler_proto::build_info` copies: a process-wide value in a `OnceLock`, set
  once by `main` and ignoring a second set.
  **Source:** `crates/holler-proto/src/log.rs:257` and `crates/holler-proto/src/log.rs:305-306`
  **Verbatim excerpt:**
  > ```rust
  > static CONFIG: OnceLock<Config> = OnceLock::new();
  > ```
  > ```rust
  > pub fn init(config: Config) -> Config {
  >     let _ = CONFIG.set(config);
  > ```

- **Fact:** the live hub and the live body are this same binary, dispatched from `main` after the install at
  `main.rs:105`, so the hub's and the body's status documents read the installed string.
  **Source:** `crates/holler-cli/src/main.rs:302-303` and `crates/holler-cli/src/main.rs:342`
  **Verbatim excerpt:**
  > ```rust
  >         HubCommand::Serve(serve) => {
  >             holler_hub::serve::run(&serve.listen, serve.advertise.as_deref(), &serve.join_held)
  > ```
  > ```rust
  >         BodyCommand::Run(run) => holler_cli::body_cmd::run(run.config.as_deref()),
  > ```

- **Fact:** the hub's own `query/status` and `query/caps` (`hub query status`, `hub caps`) are built by
  `holler_hub::query::local_status` / `local_caps`, the producer the diff changes.
  **Source:** `crates/holler-hub/src/control_server.rs:225-230`
  **Verbatim excerpt:**
  > ```rust
  >         "query/status" => {
  >             let doc = crate::query::local_status(registry, read_listening_here()).await;
  >             encode_response(cid, serde_json::to_value(doc).unwrap_or_default())
  >         }
  >         "query/caps" => {
  >             let doc = crate::query::local_caps(registry, read_listening_here()).await;
  > ```

- **Fact:** a `query/status` or `query/caps` the hub forwards to a body (`hub query TARGET status`) is answered by
  `holler_body::query::local_status` / `local_caps` in the body's own process, the other producer the diff
  changes.
  **Source:** `crates/holler-body/src/dispatch.rs:39-41`
  **Verbatim excerpt:**
  > ```rust
  >         "query/status" => Ok(serde_json::to_value(crate::query::local_status(state_root, Some(identity), configs))
  >             .unwrap_or_default()),
  >         "query/caps" => Ok(serde_json::to_value(crate::query::local_caps(state_root, Some(identity), configs))
  > ```

- **Fact:** the ACP `clientInfo` version the body sends to its agent is a separate value that no verb prints;
  it stays the bare crate version (handoff-A finding 2).
  **Source:** `crates/holler-body/src/acp_driver/connection.rs:292`
  **Verbatim excerpt:**
  > ```rust
  >             v2::Implementation::new("holler-body", env!("CARGO_PKG_VERSION")),
  > ```

- **Fact:** the golden status documents hold the literal `"0.1.0"`, built from a `Status` the test constructs, so
  the producers' new value cannot change them.
  **Source:** `crates/holler-proto/tests/golden/docs/Status.hub.json:24` and
  `crates/holler-proto/tests/golden_test.rs:236`
  **Verbatim excerpt:**
  > ```json
  >   "version": "0.1.0"
  > ```
  > ```rust
  >             role: HelloRole::Hub, protocol: 2, protocol_min: 2, protocol_max: 2, version: "0.1.0".into(), hostname: "hub-host".into(),
  > ```

- **Fact:** release binaries are built from a checkout of the release tag, so the build script finds the tag's
  commit in git and needs no `HOLLER_GIT_SHA` (handoff-A finding 7).
  **Source:** `scripts/release-build.sh:10`
  **Verbatim excerpt:**
  > ```bash
  > # Usage (from the workspace root, on a checkout of the release tag):
  > ```

- **Fact:** the Homebrew formula installs the published release binary and does not build from source, so the
  brief's "the packaged release build (Homebrew) sets it" needs no change in this repository (handoff-A
  finding 7).
  **Source:** `CHANGELOG.md:805` (the 0.3.0 entry)
  **Verbatim excerpt:**
  > (fetches the real published release binary, real sha256-verified — not a source build). A

## T (Phase 6)

- **Fact:** `body caps` and `body query status` run without a live body, in the CLI process, through the body
  crate's producers, so `query_test::status_docs_carry_version` checks the CLI-installed string on those two verbs.
  **Source:** `crates/holler-cli/src/body_cmd.rs:269-273`
  **Verbatim excerpt:**
  > ```rust
  >             let doc = holler_body::query::local_status(&state.root, identity.as_ref(), &configs);
  >             doc_outcome(&doc, |doc| format!("body: role={:?}", doc.role))
  >         }
  >         Cmd::Caps => {
  >             let doc = holler_body::query::local_caps(&state.root, identity.as_ref(), &configs);
  > ```

- **Fact:** the body's `query/caps` document embeds the same `Status` its `query/status` builds, so the
  `status.version` that `body caps` prints is the field the diff changes in `local_status`.
  **Source:** `crates/holler-body/src/query.rs:86-87`
  **Verbatim excerpt:**
  > ```rust
  > pub fn local_caps(state_root: &Path, identity: Option<&BodyIdentity>, configs: &[SessionConfig]) -> Caps {
  >     let status = local_status(state_root, identity, configs);
  > ```
