# Brief: #506 remote-admin-client (story = #508 + #509)

Repo: Performant-Labs/holler. Issue: #506 (epic); this story implements #508 (hub) and #509 (CLI) together.
Rigor: second-opinion (see "Review rigor" below; the outside-review `.env` must be set up first). UI surface: no. Kind: feature.

**Branch:** `issue-508-implementation` (recommended: the Workflow's `issueNumber` is 508; the PR closes #508 and #509 and references #506).
**Forward-compat:** done, see table below (this story adds a wire contract that #437 hold/release and a later credential-separation story will use).
**Design (Phase 3):** N/A (no UI surface).
**Decision record:** ADR 0020 (`docs/adr/ADR-0020.md`) settles #507. This brief builds on it and does not reopen it.
**ADR 0020 clarification already on main:** commit `b7a517c` ("docs(adr): clarify ADR 0020 — remote admin credential is read locally, not a --token"), which landed after this brief was drafted and before this worktree branched, rewrote ADR 0020 §Decision/§Consequences to the `--server`-only shape (quoted under Evidence). This PR therefore does **not** edit `docs/adr/ADR-0020.md`.

## Problem

The hub-only verbs (`roster`, `say`, `interrupt`, `answer`, `wait`, `hub status`, `hub query`) can only reach a hub through its **local control Unix socket** under `HOLLER_STATE_DIR`. There is no network form, so an operator on a second machine has to SSH to the hub's host (the forge/pfleet wrapper in #506). ADR 0020 decided the remote form: new JSON-RPC methods on the **existing** hub WebSocket, authenticated with the **existing bound body credential** (`circuit/authenticate`, the Noise XK handshake), with no scope split, no new token type, no new error codes and no new lockout posture.

Two findings from the survey shape the design. They are not visible in the epic text:

1. **These verbs don't read hub files directly.** Each one is a one-shot JSON-RPC exchange with the live hub over `<state>/…control.sock` (`holler_hub::control::*`), served by one dispatcher, `control_server::dispatch_control`. So the remote form is a second **transport** for requests that already exist. It is not a reimplementation of the verbs.
2. **Reusing the body credential on a second socket kicks the real body off.** `handle_authenticated` supersedes any live socket for the same token, then registers the new socket as that body in the registry and roster. An admin connection that goes through this path unchanged would disconnect the body whose credential it borrowed, and would take over its roster rows. The admin connection therefore has to branch off **after** the Noise handshake and hello but **before** supersede, registration and the roster writes.

## Evidence (verbatim, as of `00d6120`)

Every verb is a control-socket exchange (client side):
```
crates/holler-hub/src/control.rs:391-398
fn send_over(
    path: &PathBuf,
    id_literal: &str,
    method: &str,
    params: Option<serde_json::Value>,
    timeout: std::time::Duration,
) -> Result<serde_json::Value, ControlError> {
    let stream = UnixStream::connect(path).map_err(|_| ControlError::NoLiveHub)?;
```
```
crates/holler-hub/src/control.rs  (public verb fns, each builds method+params+timeout then calls exchange/exchange_with_timeout)
pub fn status() -> Result<serde_json::Value, ControlError> {
    exchange("b-status", "control/status", None)
}
pub fn roster(all: bool, prefix: Option<&str>) -> Result<serde_json::Value, ControlError> {
    exchange("b-roster", "control/roster", Some(serde_json::json!({ "all": all, "prefix": prefix })))
}
pub fn say_with(session, text, queue, grant, timeout) -> ... {
    ... exchange_with_timeout("b-say", "control/say", Some(params), timeout + std::time::Duration::from_secs(5))
}
pub fn interrupt(session, text) -> ... { ... "control/interrupt" ... ceiling 60s (+600+5 with text) }
pub fn answer(session, choice) -> ... { exchange("b-answer", "control/answer", ...) }
pub fn wait(sessions, prefix, until, after, timeout) -> ... { ... "control/wait" ..., timeout + 5s }
pub fn query_local(method, params) -> ... { exchange("b-query-local", "control/query_local", ...) }
pub fn query_remote(target, method, params) -> ... { exchange("b-query-remote", "control/query_remote", ...) }
```
(Signatures shortened; the method names, params and timeouts are verbatim.)

The CLI verbs call these and render the returned `Value` (the output path is identical for local and remote):
```
crates/holler-cli/src/roster_cmd.rs:37-39
pub fn run(roster: &Roster, json: bool) -> RosterResult {
    let state_root = holler_hub::state::resolve_state_dir().unwrap_or_default();
    match holler_hub::control::roster(roster.all, roster.prefix.as_deref()) {
```

One server-side dispatcher; its method set is wider than ADR 0020's verb list:
```
crates/holler-hub/src/control_server.rs:95-108
    match method {
        Some("control/status") => { ... status_doc(registry, lockout) ... }
        Some("control/token_ping") => token_ping(&cid, &obj, registry).await,
        Some("control/caps") => { ... }
        Some("control/support") => hub_support(&cid, &obj, registry).await,
        Some("control/query_local") => hub_query_local(&cid, &obj, registry).await,
        Some("control/query_remote") => hub_query_remote(&cid, &obj, registry).await,
        Some(other) if other.starts_with("control/") => {
            dispatch_session_control(other, &cid, &obj, registry, roster).await
        }
crates/holler-hub/src/control_server.rs:130-161
        "control/say" => say(cid, obj, registry, roster).await,
        "control/interrupt" => interrupt(cid, obj, registry, roster).await,
        "control/answer" => answer(cid, obj, registry).await,
        "control/roster" => roster_control(cid, obj, roster, registry).await,
        "control/hold" => crate::control_hold::hold(cid, obj, registry, roster),
        "control/release" => crate::control_hold::release(cid, obj, registry, roster),
        "control/test_drop" if test_hooks_enabled() => test_drop(cid, obj, registry).await,
        "control/wait" => wait(cid, obj, roster).await,
        "control/revoke" => revoke(cid, obj, registry).await,
```

The authenticated WS path treats every authenticated socket as the body. It supersedes, registers and binds the roster:
```
crates/holler-hub/src/circuit.rs:275-277
    if let Some(old) = registry.find_by_token(&params.token_id).await {
        old.supersede().await;
    }
crates/holler-hub/src/circuit.rs:289-299
    let (mut cmd_rx, mut cancel_rx, seq) =
        registry.insert(&client_id, &record.label, &params.token_id, peer).await;
    registry.set_harnesses_advertised(&client_id, body_harnesses.clone()).await;
    ...
    roster.set_token(&params.token_id, &client_id);
    roster.set_label(&params.token_id, &record.label);
crates/holler-hub/src/circuit.rs:826-828   (every inbound frame refreshes the body's roster row)
        if let Some(token_id) = self.registry.token_id_for_client(self.client_id).await {
            self.roster.touch(&token_id, env.method().unwrap_or(""));
        }
crates/holler-hub/src/circuit.rs:862-865   (any other inbound request today)
            Envelope::Request { id, .. } => {
                send_error(self.sink, Some(id), Code::MethodNotFound, "unknown method").await;
                Ok(())
            }
```
`authenticate_and_hello` (circuit.rs:178-225) runs the Noise handshake and `hello_exchange` and returns before any of the above. That is the branch point. `hello_exchange` (circuit.rs:360) already parses the peer's `Hello` and currently keeps only `harnesses`.

Verified at `daf6633` (no file under `crates/` changed since `00d6120`). The signature and tail of the branch point:
```
crates/holler-hub/src/circuit.rs:178-185
async fn authenticate_and_hello<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    id: Option<&str>,
    params: &Authenticate,
    state: &HubState,
    deps: &AuthDeps<'_>,
) -> Option<(String, crate::token::Record, Vec<String>, String)>
crates/holler-hub/src/circuit.rs:218-224
    let body_harnesses = hello_exchange(sink, stream, &params.hostname, state).await.ok()?;

    // `finish_prove` only ever returns a `Bound` record (from `bound_record`,
    // which requires a `client_id` to be `bound`) — safe to unwrap the
    // invariant here.
    let client_id = record.client_id.clone().unwrap_or_default();
    Some((client_id, record, body_harnesses, sas))
crates/holler-hub/src/circuit.rs:360
async fn hello_exchange<Snk, St>(sink: &mut Snk, stream: &mut St, hostname: &str, state: &HubState) -> Result<Vec<String>, ()>
crates/holler-hub/src/circuit.rs:379
    let body_hello = params.clone().and_then(|v| serde_json::from_value::<Hello>(v).ok());
```
No `registry.insert`, supersede or `roster.set_*` runs before the branch point. **Two side effects do run before it, on the shared auth path**, and they apply to an admin connection exactly as to a body, because the hub cannot know the role until the hello:
```
crates/holler-hub/src/circuit/auth.rs:94   (every counted -32002 refusal, via refuse_unauthenticated)
    let outcome = key_ip(peer_ip).map(|ip| deps.lockout.record_failure_detailed(&ip, code, token_id));
crates/holler-hub/src/circuit/auth.rs:102-108   (the tail every -32002 refusal shares)
async fn refuse_and_close<Snk>(sink: &mut Snk, id: Option<&str>, message: &str, reason: Option<&'static str>, token_id: &str, deps: &AuthDeps<'_>)
...
    send_error_with_reason(sink, id, Code::Unauthenticated, message, reason).await;
    close(sink).await;
    deps.roster.clear(token_id);
crates/holler-hub/src/circuit/auth.rs:451-455   (a successful prove resets that (peer, token) lockout bucket)
    if let Some(ip) = key_ip(peer_ip) {
        if deps.lockout.reset(&ip, &params.token_id) {
            log_cleared(&LockoutKey { peer: ip, token_id: Some(params.token_id.clone()) }, "authenticated");
        }
    }
```
`Roster::clear(token_id)` (roster.rs:413-423) sets `conn_state = "gone"` on **every** row with that `token_id`. So a refused admin authentication marks the rows of the live body that owns the same credential `gone`. See Risks.

The lockout key (verbatim module doc):
```
crates/holler-hub/src/lockout.rs:9-21
//! **The key (issue #455)** is the peer's transport IP address and the token
//! id it claimed, so one token's failures lock out only that token from that
//! address: behind a reverse proxy (ADR 0006) every body shares the proxy's
//! address. Both are abuse-control inputs, never identity: the address is the
//! accept-time peer (no forwarded-address header is ever read), and the id is
//! unauthenticated input that only partitions a counter (clipped to
//! [`MAX_TOKEN_ID_BYTES`]); the claimed hostname is never part of the key.
//! The #184 flood guard stays: the failure that would create an address's
//! [`MAX_TOKEN_IDS_PER_PEER`]th live token bucket folds its buckets into one
//! **peer-wide** entry, tripped at once, which refuses every id from that
//! address and bounds the map. A peer-wide lockout refuses a new socket before
//! a frame is read ([`Lockout::is_locked_out`]); a token lockout, once the
//! peer names the token ([`Lockout::is_token_locked_out`]).
```
Defaults (lockout.rs:1-7): 5 failures in 10 min trip a 10 min lockout; loopback is not exempt (lockout.rs:26).

Liveness on the body path is "no frame of any kind from the peer within `liveness_timeout`", and pongs count as frames. Both timers have test overrides:
```
crates/holler-hub/src/circuit.rs:96-103
fn liveness_timeout() -> std::time::Duration {
    std::env::var("HOLLER_HUB_LIVENESS_TIMEOUT_MS")
        ...
        .unwrap_or(heartbeat_interval() * 3)
}
crates/holler-hub/src/circuit.rs:111-117
fn ws_ping_interval() -> std::time::Duration {
    std::env::var("HOLLER_WS_PING_INTERVAL_MS")
        ...
        .unwrap_or(std::time::Duration::from_secs(10))
}
crates/holler-hub/src/circuit.rs:656-658, 678, 688-689   (the body loop's select arms)
                        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {
                            self.last_frame_at = tokio::time::Instant::now();
                        }
                _ = tokio::time::sleep_until(self.last_frame_at + liveness_timeout()) => {
                _ = self.ws_ping.tick() => {
                    if self.send_ws_ping().await.is_err() {
```

The Unix control socket does **not** cancel a request when its client goes away. The handler awaits `dispatch_control` to completion and only notices the disconnect when the reply write fails, so a CLI killed mid-`say` today lets the turn finish and the reply is discarded (the MO 5 precedent):
```
crates/holler-hub/src/control_server.rs:24-50 (abridged to the loop)
pub async fn handle_control_conn(
    ...
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                ...
                let reply = dispatch_control(&line, &registry, &roster, &lockout).await;
                let bytes = format!("{reply}\n");
                if write_half.write_all(bytes.as_bytes()).await.is_err() {
                    return; // client went away.
                }
            }
            Ok(None) => return, // client closed.
            Err(_) => return,
        }
    }
}
```
It is spawned per connection (`serve.rs:565`, `tokio::spawn(crate::control_server::handle_control_conn(...))`). Note that it serves one request at a time per connection; MO 5's concurrent requests on one admin socket are new behaviour on the WS side only.

The hello carries a role, and today it is closed to two values:
```
crates/holler-proto/src/docs.rs:30-37
/// The two endpoint roles a `circuit/hello` (or `query/status`) document is
/// sent from. Wire values `"body"` / `"hub"` (docs §3).
#[serde(rename_all = "kebab-case")]
pub enum HelloRole {
    Body,
    Hub,
}
```
Other doc comments that state the two-role closure and must be updated with `HelloRole::Admin`: `docs.rs:175` (`/// The endpoint role: \`"body"\` or \`"hub"\`.`), `docs.rs:227-228` (`query/status` "One shape, two \`role\` values", since `Status.role` is also a `HelloRole`), and `methods.rs:40` (`/// Only the body may send (join / authenticate).` on `Direction::BodyToHub`, which the `admin/*` rows will use).

The codec does **not** enforce `Direction`. `holler_proto::methods::Direction` is referenced only by the `CATALOG` rows (no use in `crates/holler-proto/src` or `crates/holler-hub/src` outside `methods.rs`), and `decode_request` checks only catalog membership, the notification flag and the id form (frame.rs:117-140). So a `session/prompt` request sent by a peer decodes, and the `-32601` it gets comes from the hub's dispatch, not the decoder.

The codec rejects any method missing from the catalog, so the new methods need catalog rows:
```
crates/holler-proto/src/envelope/frame.rs:122-125
    // The method must be in the catalog.
    if crate::methods::find(method).is_none() {
        return Err(EnvelopeError::UnknownMethod(method.to_owned()));
    }
crates/holler-proto/src/methods.rs:46-48
/// The complete, closed v2 method catalog (15 rows).
#[rustfmt::skip]
pub const CATALOG: &[Method] = &[
```
`crates/holler-proto/tests/codec_test.rs:83` (`every_method_round_trips`) iterates `CATALOG` and panics on any method with no `canonical_frame`.

The "bound credential" is not a string. It is two files on the body machine, and the join secret in `--token ID:SECRET` is one-shot:
```
crates/holler-body/src/identity.rs:100-102
    pub fn path(state_root: &Path) -> std::path::PathBuf {
        state_root.join("body").join("credential.json")
    }
crates/holler-body/src/x25519_identity.rs:68-70
pub fn identity_path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("x25519_identity.key")
}
crates/holler-cli/src/cli.rs:526-538
pub struct Join {
    #[arg(long)]
    pub server: String,
    /// Join token, as ID:SECRET.
    #[arg(long)]
    pub token: String,
    ...
    #[arg(long = "hub-key")]
    pub hub_key: String,
}
```
`BodyIdentity` (identity.rs:40ff) holds `token_id`, `client_id`, `server_url` and the pinned `hub_pubkey`. The body's Noise initiator and hub-key-pinning hello are already written and can be reused:
```
crates/holler-body/src/connection/handshake.rs:55
pub(crate) async fn authenticate<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity, state_root: &Path) -> Result<String, Attempt>
crates/holler-body/src/connection/handshake.rs:208-213
pub(super) async fn hello_exchange<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    identity: &BodyIdentity,
    configs: &[SessionConfig],
) -> Result<(), Attempt>
   (builds `Hello { role: HelloRole::Body, ... }`, then compares hub_hello.hub_pubkey against identity.hub_pubkey; a mismatch is AuthFailed)
```
`crates/holler-body/src/join.rs` uses `server_address::{parse, loopback_only_check}`: a plaintext `ws://` to a non-loopback host is an exit-3 policy refusal, checked before any connect.
```
crates/holler-body/src/server_address.rs:160-168
pub fn loopback_only_check(addr: &ServerAddress) -> Option<&'static str> {
    if addr.scheme == "ws" && !addr.is_loopback() {
        Some(
            "a plaintext ws:// to a non-loopback host is refused; \
             off-loopback pairing is wss:// (ADR 0002)",
        )
    } else {
        None
    }
crates/holler-body/src/join.rs:84-87
    if let Some(reason) = loopback_only_check(&addr) {
        eprintln!("error: {reason}");
        return JoinExit::Policy;
    }
```
`parse` accepts `ws://` and `wss://` only (server_address.rs:87-95), so the loopback `ws://127.0.0.1:<port>` URL a test hub's `ws_url()` returns passes, and `ws://10.0.0.5:1` is refused.

The hub-side Noise prologue binds whatever `advertised_url` the client sends. The hub does not compare it with its own listen address:
```
crates/holler-hub/src/circuit/auth.rs:327
    let prologue = holler_proto::noise::build_prologue(holler_proto::PROTOCOL_VERSION, &params.token_id, &params.advertised_url);
crates/holler-body/src/connection/handshake.rs:70, 83   (the client side takes both from the identity)
    let prologue = holler_proto::noise::build_prologue(holler_proto::PROTOCOL_VERSION, &identity.token_id, &identity.server_url);
        advertised_url: identity.server_url.clone(),
```

The join secret is consumed on redeem, and ADR 0020 as it now stands on main takes no credential flag:
```
crates/holler-hub/src/token.rs:126-129
    /// Minted, not yet redeemed.
    Unused,
    /// Redeemed by a body; the join secret is consumed.
    Bound,
docs/adr/ADR-0020.md (Consequences, as of b7a517c)
- The remote forms of `roster`/`say`/`interrupt`/`answer`/`wait`/`hub status`/`hub query` (#509) take `--server URL` only; there is no `--token`/`--hub-key` flag on them, unlike `body join` — they read the local body identity instead of taking one on the command line.
```
(ADR 0019 contains no sentence saying "no reusable credential string". That phrase in the MO section below is O's summary of the credential model, not a quotation.)

Errors: `Refused` already exists, and the `*_cmd` renderers already branch on it:
```
crates/holler-hub/src/control.rs:18-31
/// Why a control exchange could not reach the live hub.
#[derive(Debug)]
pub enum ControlError {
    /// No socket at the expected path (the hub is not running, or the state
    /// dir is wrong) — the spec's `no live holler hub reachable at <dir>`.
    NoLiveHub,
    /// The socket was present but the exchange failed (I/O or a bad reply).
    Io(std::io::Error),
    /// The reply did not parse as a v2 envelope.
    BadReply(String),
    /// The hub answered with a JSON-RPC error (issue #182: `control/
    /// token_ping`'s `-32004 not_connected` is reported this way).
    Refused(holler_proto::WireError),
}
crates/holler-cli/src/say_cmd.rs:130-138
        Err(holler_hub::control::ControlError::NoLiveHub) => {
            err(format!("no live holler hub reachable at {}", state_root.display()), 1)
        }
        Err(holler_hub::control::ControlError::Refused(e)) => {
            ...
            let is_ambiguous = e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("ambiguous");
```
`say_cmd.rs` then routes `is_held` to `HELD_EXIT_CODE` (4, hold_cmd.rs:19), `SessionBusy` to `busy_hint`, and ambiguity to exit 2. `interrupt_cmd.rs:52`, `answer_cmd.rs:48`, `hold_cmd.rs:58` and `hub_cmd.rs:190` (`Refused(e) if is_ambiguous(&e)` → exit 2) match `Refused` the same way.

The only place a prompt is sent (hold enforcement) is guarded by a source-level test:
```
crates/holler-cli/tests/hold_single_path_test.rs:2-6   (line 1 is the `#![allow(...)]` attribute)
//! The session hold has one enforcement point (issue #442, umbrella decision
//! 3): every prompt reaches a body through `send_prompt` in
//! `crates/holler-hub/src/circuit/dispatch.rs`, and the hold is checked there.
//! A second way to put a `session/prompt` on a body's socket would be a way
//! around every hold, so this test fails when one appears.
```

The CLI surface is pinned by a fixture. Every leaf and flag combination must parse:
```
crates/holler-cli/tests/fixtures/cli-surface.txt:42-59 (roster|say|interrupt|answer|wait lines), 30-39 (hub status|hub query)
crates/holler-cli/tests/cli_surface_test.rs:1-13 (fixture parses; pending file does not; leaf set == clap's)
```

Test rigs that already exist: `tests/support/mod.rs` (`StateDir::hub()`/`body()`, `Hub::start`, `ws_url`, the `roster_json`/`say` runners) and `tests/support/raw_ws.rs` (`connect_ws`, `authenticate_on`, which runs a hand-rolled Noise handshake against a real hub).

What those rigs actually do (verified):
- `StateDir::hub()`/`body()` are the `hub/` and `body/` **subdirectories of one root** (`support/mod.rs:93-100`), and `HOLLER_STATE_DIR` is that root. A body state dir with no hub files therefore means a **separate** `StateDir` for the body, as the existing two-machine-style tests already do (`roster_cli_test.rs:189-193`: `let hub_state = StateDir::new(); ... let io_body_state = StateDir::new(); ... join(&io_body_state, &hub_state, &hub.ws_url(), ...)`). The control socket is `<root>/hub/control.sock` (`holler-hub/src/state.rs:67-68`).
- `raw_ws::authenticate_on` (`raw_ws.rs:104-119`) sends **only** `circuit/authenticate` message 1 and reads one frame, expecting a refusal. It never completes `circuit/prove` or the hello, so it cannot open a live admin socket by itself. The complete initiator (authenticate → prove → hello) exists only as private helpers in `hub_hygiene_test.rs` (`send_authenticate` at :83, `run_hello` at :127, `go_live` at :153), with the hello's `role` fixed to a body.
- A slow session: `stub-acp --slow` sets a 200 ms inter-chunk gap and `--chunks N` sets the chunk count (`tests/stub-acp/main.rs:287-291`), passed as `write_sessions_toml`'s extra argv (as `talk_test.rs:368` does with `&["--slow", "--chunks", "5"]`).

## Decisions already made (MO)

From ADR 0020 (not reopened):
- **Credential:** the existing bound body credential. No new token type. `hub token mint|list|revoke|delete` are unchanged.
- **Scope:** none. Every valid bound credential can drive every verb in scope. No ACL.
- **Wire:** new JSON-RPC 2.0 methods on the existing hub WebSocket. No second port or channel.
- **Error codes / rate limiting:** no new codes. The existing authenticate lockout applies because the handshake is the same one.

Filled in by O for this story (the A gate confirms or BLOCKs):

1. **Connection role = `circuit/hello` role `"admin"`.** Add `HelloRole::Admin` (wire `"admin"`). The client runs the normal `circuit/authenticate` → `circuit/prove` handshake, then sends its hello with `role: "admin"`. The hub makes `hello_exchange` return the peer's role alongside the harnesses. When the role is `admin`, `handle_authenticated` goes to a new **admin loop** and skips supersede, `registry.insert`, `set_harnesses_advertised`, `roster.set_token/set_label`, the confirm probes and `roster.touch`. A `body` role, or a hello that fails to parse, takes today's path unchanged. The rejected alternative was an extra `Authenticate` field: that struct is `deny_unknown_fields`, and the hello is already where docs §4 says the two endpoints differ ("distinguished only by `hello.role`").
2. **Wire methods: seven, one per verb, `admin/` namespace.** `admin/status`, `admin/roster`, `admin/say`, `admin/interrupt`, `admin/answer`, `admin/wait`, `admin/query`. Params and results are **byte-for-byte the same as the `control/*` counterparts**. `admin/query` takes `{target?, method, params}` and maps to `control/query_remote` when `target` is present and to `control/query_local` when it is absent. Catalog rows: `kind: Request`, `dir: BodyToHub`, legal only on an admin-role connection (documented in v2.md §4).
3. **Allowlist, not passthrough.** The hub maps exactly those seven `admin/*` names to the existing handlers in `control_server.rs`. Nothing else reachable through `dispatch_control` (`revoke`, `token_ping`, `test_drop`, `hold`, `release`, `caps`, `support`) is reachable over the network. ADR 0020 says hold/release join this surface once #437 lands, so they are out of scope here.
4. **Role separation on each socket.** On an admin connection, anything other than `admin/*` and `circuit/ping` gets `-32601` (and `session/presence`/`session/update` notifications are dropped without touching the roster). On a body connection, `admin/*` keeps today's `-32601`. A body socket never gains admin powers, and an admin socket can never publish presence.
5. **Hub admin loop behaviour.** Each `admin/*` request is serviced without blocking the socket: WS pings keep flowing, and a close or drop is noticed while a 600 s `say`/`wait` is in flight. Several concurrent requests on one admin socket are allowed. An admin connection is never timed out while a request is in flight. An idle admin connection may be closed on the existing liveness timeout. If the client disconnects mid-`say`, the hub finishes the turn and discards the reply, which is the same as a CLI killed mid-`say` on the Unix socket today. Log `admin_connected`/`admin_dropped` with `{token_id, label, peer, sas}`, plus one debug line per request. **Not** `conn_connected`, which operators read as "a body is up".
6. **Hub hello to an admin client:** unchanged (`hub_hello_doc`, including `hub_pubkey`), so the client pins the hub key with the existing code.
7. **Client location and reuse.** Add `holler_body::admin_client` (new module in holler-body, next to the Noise initiator it reuses): load `BodyIdentity` and the X25519 identity from the local state root, dial, call `connection::handshake::authenticate` unchanged, then run the hello. Give `handshake::hello_exchange` a role parameter (body: today's behaviour; admin: `role: Admin`, `harnesses: None`, `sessions: None`) so the hub-key pinning is **not** duplicated. Send one `admin/*` request, read its response (skipping pings), then close with a clean close frame.
8. **Transport switch without duplicated param-building.** Refactor `holler_hub::control` so each verb builds its request once, as a `ControlCall { method, params, timeout }` (for example `ControlCall::roster(all, prefix)`). The existing public fns (`roster`, `say_with`, the `_at` variants, and so on) become thin wrappers with unchanged signatures and behaviour. `holler-cli` gets one dispatch point (for example `crate::transport::call(&ControlCall, Option<&RemoteTarget>)`): local sends over the Unix socket as today, and remote rewrites `control/x` → `admin/x` and calls `admin_client`. holler-hub must not depend on holler-body; the glue lives in holler-cli.
9. **Errors map onto existing `ControlError`.** A hub JSON-RPC error is `ControlError::Refused(WireError)`, so the `*_cmd` rendering (exit codes, `session_busy`/`session_held` hints, ambiguity exit 2) is shared and unchanged. Add **one** variant for "could not reach / authenticate to the remote hub" that carries a plain message. Its exit code is 1, and its words reuse `handshake::refusal_words` for `-32002`. `NoLiveHub`'s "no live holler hub reachable at <dir>" must never appear in remote mode.

**Needs an explicit operator (MO) decision before A. O's recommended default is given; the brief is written to it:**

- **CLI credential flags. ADR 0020's Consequences are wrong on the facts here.** The ADR says remote verbs take "`--token` … the same credential `body join` uses". But `body join --token ID:SECRET` is a **one-time join secret** that `circuit/join` consumes. Since ADR 0019 there is no reusable credential string: the bound credential is the body's `credential.json` plus `x25519_identity.key`. A `--token ID:SECRET` on `roster` therefore cannot authenticate anything. **Recommended:** remote mode is selected by `--server <URL>` alone. The credential is **always** the body identity under the invoking process's `HOLLER_STATE_DIR` (the machine-B body's own state). The hub key is the one already pinned in that identity. **No `--token` and no `--hub-key`** on these verbs. `--server` goes through `server_address::parse` + `loopback_only_check` (exit 3 on plaintext non-loopback, the same as `body join`) and is used as the dial URL and the Noise `advertised_url`. If the operator agrees, a one-line clarifying amendment to ADR 0020 §Consequences (and to #509's wording) belongs in this PR. If the operator wants a different shape, amend this section before A runs.
- **The epic and #511 acceptance text conflict with ADR 0020.** #506's "a credential minted for this purpose cannot be used to join as a body…" and "read-only/read-write split … enforced by the hub", and #511 ACs 3–4, describe the **deferred** credential separation and scope split. This story does not satisfy them and must not claim to. The operator should edit #506/#511 to point those items at the follow-up issue ADR 0020 says will be filed. That is a GitHub edit, not code.

**Status of the flag decision (added at brief review round 1; the text above is unchanged):** ADR 0020 on main (commit `b7a517c`, quoted under Evidence) now states the `--server URL`-only shape with no `--token`/`--hub-key`, which is the same as O's recommended default. That ADR edit is therefore **not** part of this PR. The #509 wording edit and the #506/#511 edits are GitHub issue edits by the operator, not files in this PR (see Out of scope). The A gate must still get the operator's explicit confirmation that `b7a517c` is their decision before T starts. This amendment does not make that call.

### Clarifications (brief review round 1)

These add mechanism the decisions above leave implicit. They do not change any decision.

- **`--server` vs the joined URL (MO 7 + the flag default).** `connection::handshake::authenticate` stays unchanged. It takes the Noise prologue and `advertised_url` from `identity.server_url` (handshake.rs:70, 83). `admin_client` loads the `BodyIdentity`, clones it, and sets the clone's `server_url` to the parsed `--server` URL before calling `authenticate`. The same URL is dialled and advertised. The identity file on disk is never rewritten. A `--server` that differs from the URL the body joined with is allowed: the hub binds the prologue to whatever `advertised_url` the client sends and never compares it to its own address (auth.rs:327). The hub key still comes from the identity's pin.
- **Lockout and roster side effects are shared with the body (W-3).** Auth refusals happen before the hello, so the hub cannot tell an admin client from a body when it refuses one. An admin client on machine B uses machine B's body credential, so it has the same `(peer IP, token_id)` lockout key as machine B's body (lockout.rs:9-21). Five refused admin attempts in 10 min therefore lock that body's token out from that address. Every counted refusal also runs `roster.clear(token_id)` (auth.rs:108), which marks that body's live rows `gone`. The Risks section records this. Whether to mitigate it in this PR needs an operator decision, because a mitigation would change the shared auth path, and Out of scope excludes changes to the Noise handshake.
- **Line budget for the 900-line guard (W-4).** `scripts/lint.sh:46` fails at `>= 900` lines. circuit.rs is 891 lines, so it may grow by **at most 8 lines** net: the `hello_exchange` return-type change plus a single `if role == Admin { return admin::run(...).await; }` style branch, with everything else in `circuit/admin.rs`. control_server.rs is 879 lines, so it may grow by **at most 20 lines** net for the `pub(crate)` entry. If either budget is exceeded, the pre-agreed move is: for circuit.rs, move `hello_exchange` and `hub_hello_doc` (circuit.rs:360-468) into a new `circuit/hello.rs`; for control_server.rs, move `status_doc` and `read_listening` (control_server.rs:721-791) into a new `control_status.rs` sibling, following the `control_hold.rs` precedent. Both moves are verbatim, with no behaviour change.
- **Test rig for a hand-rolled admin socket (AC 4, 5, 16).** `raw_ws::authenticate_on` cannot finish a handshake (see Evidence). Add one helper to `tests/support/raw_ws.rs` that runs authenticate → prove → hello with a caller-chosen `role`. Build it by moving `hub_hygiene_test.rs`'s private `send_authenticate`/`run_hello` into `raw_ws.rs` and giving `run_hello` a role parameter, the same "moved verbatim" pattern #455 used for `raw_ws.rs`. Do not write a third copy of the initiator.

## Acceptance criteria (each observable)

Hub / wire:
1. `holler_proto::methods::CATALOG` contains the seven `admin/*` rows, and `codec_test::every_method_round_trips` passes with a canonical frame for each.
2. `HelloRole::Admin` serializes as `"admin"` and round-trips (a `docs_wire_test`/`golden_test` case).
3. **No supersede, no roster hijack.** With a body connected and one session live, a remote `holler roster --server <ws_url>` run from the **body's** state dir exits 0 and lists that session. Afterwards the body is still connected: the roster row stays `connected`, the body log has no `circuit/superseded`, and a later `say` to the session still gets its reply. Test: `remote_admin_does_not_supersede_the_body`.
4. An admin connection never creates, changes or refreshes a roster row. A hand-rolled admin socket (the new full-handshake `raw_ws` helper with hello `role:"admin"`, see Clarifications; `raw_ws::authenticate_on` alone cannot finish a handshake) that sends `session/presence` produces no roster row, and `last_heard` of the body's row (the roster `Row`'s `last_seen` field, roster.rs:162) is not moved by admin traffic.
5. Allowlist: on an admin socket, `control/revoke`, `control/test_drop`, `admin/revoke`, `admin/hold`, `admin/release` and `session/prompt` each get a `-32601` error (or a decode `UnknownMethod`). For `session/prompt` it is the dispatch-level `-32601`, because the codec does not enforce `Direction` (see Evidence). A body-role socket that sends `admin/roster` gets `-32601`. Plus a unit test that the hub's admin allowlist is exactly the seven names.
6. Hold still has one enforcement point: `hold_single_path_test` passes unmodified, and a remote `say` to a locally `hold`-ed session exits with the same `session_held` refusal text and code as the local `say`.
7. An unknown token, a wrong X25519 key, or a revoked token on the admin path is refused with `-32002` exactly as a body is, and counts toward the existing lockout (`hub status --json` lockout entry). There is no new code path around the lockout.
8. Concurrency: two remote admin clients plus one local `say` to the same session behave like today's local-only story. One gets the reply, the other gets `session_busy`, or both succeed with `--queue`. No panic and no hang. Per contender: without `--queue`, **each** of the three (remote A, remote B, local) exits either 0 with a reply or 1 with the `session_busy` refusal, at least one exits 0, and none runs past its own timeout. With `--queue` on all three, all three exit 0 with a reply. Afterwards the body is still connected and a fourth `say` succeeds.

CLI:
9. Each verb accepts `--server <URL>` and nothing else new: `roster`, `say`, `interrupt`, `answer`, `wait`, `hub status`, `hub query`. New lines in `tests/fixtures/cli-surface.txt` (at least one per verb, for example `roster | --server wss://hub.example.ts.net --json`), and `cli_surface_test` passes.
10. **Parity.** For each verb, `--json` stdout from the remote form is byte-identical to the local form against the same hub state. Tested at least for `roster --json`, `say --json`, `hub status --json` (after masking volatile fields such as elapsed times), `wait --json` and `hub query status --json`. Also tested for `interrupt --json` and `answer --json`, including one refusal each (for example an unknown session), whose stderr and exit code must match the local form. The masked set is exactly these fields, replaced with a fixed placeholder by one helper in `remote_admin_test.rs` before comparison: `roster` → `rows[*].last_seen`, `rows[*].last_update_at` (roster.rs:162-165); `say` → `elapsed_ms` (control_server.rs:272); `wait` → `rows[*].age_secs` (control_server.rs:575); `hub status` → `lockout.peers[*].retry_after_secs` (lockout.rs:541); `hub query status` → none. Every other field is compared byte for byte. If T finds another field that differs between two identical **local** runs, T names it with its source line in the T-red handoff, and the masked set is extended by that one field only.
11. The remote form needs **no hub state**. Every remote test runs the CLI with `HOLLER_STATE_DIR` set to the **body** state dir, which has no control socket and no hub files.
12. Omitting `--server` keeps today's behaviour. The existing `roster_cli_test`, `talk_test`, `interrupt_test`, `answer_cli_test`, `wait_test` and `query_test` pass unchanged.
13. Failure words and exit codes: `--server ws://10.0.0.5:1` gives exit 3 (plaintext non-loopback, the same message family as `body join`). A missing `body/credential.json` gives exit 1 with a message naming the path and `holler body join`. An unreachable server gives exit 1, "could not reach the hub at <url>: …". A hub-key mismatch gives exit 1, with the same words `body run` uses. An ambiguous session name on the remote path (`say`, and `hub query` with an ambiguous target) exits 2 with the same text as the local form (decision 9; say_cmd.rs:138, hub_cmd.rs:190).
14. `cargo test --workspace`, `bash scripts/lint.sh` (including the 900-line file guard) and `cargo clippy --workspace --all-targets -- -D warnings` are green.

Docs (in this PR because they are the standing spec, per CLAUDE.md):
15. `docs/protocol/v2.md` §3 (roles) and §4 (catalog table) document `role:"admin"` and the seven `admin/*` methods, with params and results given as "identical to the local control form". They state plainly that any bound credential can use them (ADR 0020). They also state that an admin client's failed authentications count against the same `(peer address, token_id)` lockout bucket as the body whose credential it uses, and that a refused authentication clears that token's roster rows. In other words, a misconfigured admin client on a body's machine can lock that body out (see Risks). `docs/deploy.md`, the migration note and the forge cross-link stay in #510.

Admin-loop liveness (MO 5), all in `remote_admin_test.rs`. The hub runs with `HOLLER_WS_PING_INTERVAL_MS` and `HOLLER_HUB_LIVENESS_TIMEOUT_MS` set low (for example 200 and 1000), and the target session is `stub-acp --slow --chunks N`, with N chosen so the turn lasts well over the liveness timeout:
16. (a) **Pings during an in-flight request.** On a hand-rolled admin socket, while an `admin/say` to the slow session is in flight, the client receives at least one WS `Ping` frame from the hub before the `admin/say` response, and the pongs it sends are accepted (the socket stays open and the response arrives). (b) **In-flight is never timed out.** A second admin socket sends `admin/say` to the slow session and then sends nothing and reads nothing (so it sends no pongs) for longer than the liveness timeout. It still receives the `admin/say` response afterwards. (c) **Idle is closed.** An admin socket that completes its hello, then sends no request and does not read, is closed by the hub within liveness timeout plus a bounded margin, and the hub logs `admin_dropped` for it. (d) **Concurrent requests on one socket.** One admin socket sends two `admin/*` requests with different ids without waiting (for example `admin/say` to the slow session and `admin/roster`). The `admin/roster` response arrives **before** the `admin/say` response, and each response carries its own request's id. (e) **Client drop mid-`say`.** An admin client that closes its socket (clean close, and separately an abrupt TCP drop) while its `admin/say` is in flight leaves the hub healthy. The body still receives the prompt, the turn completes (the session's roster row returns to its idle state with that turn in `last_turn`), no reply is written to the closed socket, and a subsequent remote `roster --server` and `say --server` both exit 0.
17. **Admin connection logs.** A successful remote verb produces exactly one `admin_connected` and one `admin_dropped` hub log event, each with `token_id`, `label`, `peer` and `sas` fields. It produces no `conn_connected` event for that connection. The body's own `conn_connected` count is unchanged by admin traffic.

## Files

Production:
- `crates/holler-proto/src/docs.rs` (`HelloRole::Admin`; also update the two-role doc comments at docs.rs:28-29, 175 and 227-228)
- `crates/holler-proto/src/methods.rs` (7 rows, doc comment count; `Direction::BodyToHub`'s "Only the body may send (join / authenticate)" at methods.rs:40 gains the admin client)
- `crates/holler-hub/src/circuit.rs` (branch in `handle_authenticated`; `hello_exchange` returns the role)
- new `crates/holler-hub/src/circuit/admin.rs` (the admin loop), so circuit.rs stays under 900 lines
- `crates/holler-hub/src/control_server.rs` (expose one `pub(crate)` entry that runs an allowlisted `control/*` method with parsed params and returns the reply, used by both the Unix socket and the admin loop)
- `crates/holler-hub/src/control.rs` (`ControlCall` refactor; public fns keep their signatures)
- `crates/holler-body/src/connection/handshake.rs` (role parameter on `hello_exchange`; `authenticate` visible to the new module)
- new `crates/holler-body/src/admin_client.rs`, and `crates/holler-body/src/lib.rs`
- `crates/holler-cli/src/cli.rs` (`--server` on 7 structs)
- new `crates/holler-cli/src/transport.rs`
- `crates/holler-cli/src/{roster,say,interrupt,answer,wait,query,hub}_cmd.rs` (route through transport)
- `docs/protocol/v2.md`
- Not touched: `docs/adr/ADR-0020.md`. Its `--server`-only clarification already landed on main at `b7a517c`.
- Only if a W-4 budget in Clarifications is exceeded: new `crates/holler-hub/src/circuit/hello.rs` and/or `crates/holler-hub/src/control_status.rs` (verbatim moves)

Tests:
- new `crates/holler-cli/tests/remote_admin_test.rs` (AC 3–8, 10–13, 16–17)
- `crates/holler-cli/tests/support/raw_ws.rs` (full-handshake helper with a `role` parameter) and `crates/holler-cli/tests/hub_hygiene_test.rs` (its private `send_authenticate`/`run_hello` move out to `raw_ws.rs`)
- `crates/holler-cli/tests/fixtures/cli-surface.txt`
- `crates/holler-proto/tests/codec_test.rs` (canonical frames)
- `crates/holler-proto/tests/docs_wire_test.rs` / `golden_test.rs` (Admin role)
- unit tests in `circuit/admin.rs` (allowlist)

Reuse map (extend, do not duplicate):
- **Closest analogous feature:** the control-socket verbs (`control.rs` client + `control_server.rs` dispatcher). Extend both. Do **not** write new roster/say/wait handlers; the admin loop calls the existing ones.
- Hub handshake: `authenticate_and_hello` unchanged. Branch after it.
- Client handshake: `connection::handshake::authenticate` unchanged; `hello_exchange` gains a role parameter, with no second copy of the pinning logic.
- Address policy: `server_address::{parse, loopback_only_check}`.
- Output rendering: the existing `*_cmd.rs` render fns, unchanged.
- Test rigs: `support::{StateDir, Hub, …}`, `support::raw_ws::{connect_ws, authenticate_on}`. Use a separate `StateDir` for hub and body. The admin-role full handshake goes in `raw_ws.rs` (see Clarifications).
- `hold_single_path_test`: must stay green without edits. If it needs editing, a second prompt path was added, and that is a BLOCK.

## Forward-compat

| consumer story | required capability | satisfied |
| --- | --- | --- |
| #437 hold/release remote (ADR 0020 Consequences) | add `admin/hold`, `admin/release` by extending the allowlist and adding catalog rows, no new mechanism | yes |
| future credential-separation / scope story (ADR 0020 "Not decided") | a single point where an admin request is authorised (the admin loop's allowlist dispatch) so a scope/credential-type check slots in without touching handlers; `HelloRole::Admin` marks admin sockets | yes |
| #510 docs | stable method names and flag shape | yes, once the MO flag decision above is recorded (ADR 0020 at `b7a517c` already states `--server` only; the operator's confirmation is pending, see MO) |
| #511 two-machine acceptance | `roster`/`say` from machine B with no SSH | yes for ACs 1–2; ACs 3–4 need the operator's edit (see MO) |

## Out of scope

- A new admin token type, a read-only/read-write split, new error codes, and any new lockout or rate-limit posture (ADR 0020 "Not decided here").
- `hold`/`release`, `hub caps`, `hub support`, `hub token *` over the network.
- `docs/deploy.md`, the migration note and the forge cross-link (#510). The manual two-machine run (#511).
- Any change to `body join`, `body run`, the Noise handshake, or the token store.
- `--token`/`--hub-key` flags on the admin verbs (unless the MO decision above changes the default).
- Editing `docs/adr/ADR-0020.md`. The clarification it needed landed on main at `b7a517c`.
- The #509 wording edit and the #506/#511 acceptance-text edits. These are GitHub issue edits by the operator, not files in this PR.

## Test plan

RED first (T):
- `codec_test::every_method_round_trips` and new `admin/*` decode tests fail: `UnknownMethod("admin/roster")`.
- `remote_admin_test::roster_remote_matches_local_json` fails at clap: `unexpected argument '--server'`. After the flag exists it fails at the hub: `-32601 unknown method` on a body-role path, or the hello role rejected.
- `remote_admin_does_not_supersede_the_body` pins the **supersede hazard**. T writes it so that it would fail against a naive implementation that just sends `admin/*` over a normal body-path socket (body row flips to `reconnecting`, or the body log shows `circuit/superseded`).
- An allowlist test drives a hand-rolled admin socket with `admin/revoke` and `control/test_drop`, expecting `-32601`. (Run the hub with `HOLLER_TEST_HOOKS=1` so that `test_drop` would exist locally. The test proves it is still not reachable remotely.)

GREEN: all of AC 1–17, plus the full existing suite unchanged. T's Tier-2 check: revert the role branch in spirit (send admin through the body path) and confirm AC 3/4 fail.

## Risks

- **Supersede / roster hijack** (above). This is the highest-impact failure: using the tool would knock a production lane offline. It is pinned by AC 3–4.
- **Accidental widening of the network surface.** If `admin/*` is implemented as a generic passthrough to `dispatch_control`, `control/revoke` and `control/test_drop` become network-reachable. Pinned by AC 5 and the allowlist unit test.
- **Hold bypass.** Any second `session/prompt` sender. Pinned by the unchanged `hold_single_path_test` and AC 6.
- **Accepted and documented, not mitigated (ADR 0020):** any holder of any bound body credential can drive every in-scope verb against every session on the hub, including `say` into other labels' sessions. `hub status` over the network exposes the lockout table (peer IPs, token labels) to any bound credential. v2.md must say this plainly (AC 15).
- **Long-request liveness through proxies.** `tailscale serve` may idle-close a silent socket during a 600 s `say`. The hub's WS pings on the admin loop cover this (MO 5), and #511 is where it gets verified for real. The hub-side half (pings flow during an in-flight request, in-flight is never timed out, idle is closed, a client drop mid-`say` is harmless) is pinned locally by AC 16. A sequential admin loop that blocks on the `say` reply fails AC 16(a) and 16(d).
- **File-size guard.** circuit.rs is 891 lines and control_server.rs 879. New code goes in new modules or the check fails. The line budget and the pre-agreed fallback moves are in Clarifications.
- **Shared lockout and roster fate with the body (verified; mitigation needs an operator decision).** An admin client's refused authentications land in the same `(peer IP, token_id)` lockout bucket as the body whose credential it uses, and each one runs `roster.clear(token_id)` on that body's rows (auth.rs:94, 108). A stale or misconfigured admin client on machine B can mark B's body `gone` and, after 5 failures in 10 min, lock B's token out from B's address. This has the same blast radius as the supersede hazard, but only after an authentication failure, never on success. AC 15 requires v2.md to say this. Changing it would touch the shared auth path, which Out of scope excludes, so it is left to the operator.
- **Flag-shape decision pending** (MO). If it flips to `--token`/`--hub-key`, AC 9/13 and the Files list change before T starts. ADR 0020 at `b7a517c` already records `--server` only, so a flip would also mean reverting that ADR text.

## Review rigor

`second-opinion`. This is a new authenticated network surface, and its failure modes are subtle correctness problems in reusing existing machinery: supersede, roster, hold path, allowlist. That is the playbook's stated trigger ("security-sensitive, subtle-correctness"). The permissive credential posture is ADR 0020 policy and does not lower the implementation risk. `panel` is not warranted: no new cryptography, the handshake is reused unchanged, and the policy questions a cross-vendor pair would argue over are explicitly deferred. **Prerequisite:** this clone has no `.env`. Before the run, set up `review-models.sh --tier outside-review=glm-5.3-flash`, `DUAL_REVIEW=1` and the Z.ai key in the run environment, then prove one real completion. If the operator declines that setup, run `in-session` and journal the downgrade and the reason in `decisions.md`.

## Handoff locations

T-red: `docs/handoffs/506-handoff-T-red.md` · F: `docs/handoffs/506-handoff-F.md` · journal: `docs/handoffs/506-decisions.md`.

## Operating rules

- Read this brief's Evidence and the files in the Reuse map before writing code. Extend the named objects. A parallel roster/say/wait handler, or a second copy of the hub-key pinning, is an anti-duplication BLOCK.
- F writes no tests. F implements against T's RED suite.
- Stage by explicit path. Never `git add .`.
- After the PR opens, add the CONTRIBUTING.md AI disclosure with `gh pr edit`.
