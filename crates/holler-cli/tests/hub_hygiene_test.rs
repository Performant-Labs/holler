//! Story #184 — hub connection hygiene: the 8 RED acceptance tests.
//!
//! Raw `tokio-tungstenite` clients against a real `holler hub serve` process
//! (the same harness as `hub_serve_test`), asserting the on-the-wire close
//! codes and error shapes the spec requires:
//!
//!   - supersede-on-reauth: the old socket gets a `circuit/superseded`
//!     notification and a **1000** close, the new socket stays live.
//!   - revoke: a live socket is force-closed with **1008** the moment its
//!     token is revoked over the control socket.
//!   - an oversized frame closes **1009**.
//!   - a silent (never-authenticating) socket is closed with **1001** after the
//!     pre-auth timeout (reduced to 500 ms here).
//!   - the unauthenticated pre-auth cap rejects the 5th socket (cap 4).
//!   - 5 failed authentications within the window lock the peer's IP out (the
//!     next connection is refused with **1008**); the lockout lapses.
//!   - a slow (200 ms) token store does not stall a sibling socket's
//!     `circuit/ping` by more than 20 ms.
//!   - `hub status --json` reports each live client's `peer` address.
//!
//! Conventions (ADR 0002 / the shared harness): each test owns a [`StateDir`];
//! readiness is observed via [`wait_for`], never a blind sleep; a hub spawned
//! with custom env is torn down with [`Hub::stop`]/drop.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #184

use std::process::Stdio;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::frame::coding::CloseCode, Message},
    WebSocketStream,
};
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;

mod support;
use support::{holler_cmd, mint_token, wait_for, Hub, StateDir};

/// The upgraded client over a plain `ws://` URL (a `WebSocketStream` over a
/// `MaybeTlsStream<TcpStream>` — the same type `hub_serve_test` uses).
type WsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Spawn a hub with extra environment (the hygiene/lockout knobs). Builds on
/// the harness's `holler_cmd` base so the state dir / debug / log-format env
/// are inherited; `HOLLER_CLI_NO_UPDATE_CHECK` keeps a one-shot CLI child (the
/// `hub token …` / `hub status` invocations) from doing a slow update check.
#[allow(dead_code)] // #184
fn start_hub_with_env(state: &StateDir, env: &[(&str, &str)]) -> Hub {
    let mut cmd = holler_cmd(state);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.env("HOLLER_CLI_NO_UPDATE_CHECK", "1")
        .args(["hub", "serve", "--listen", "127.0.0.1:0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    support::make_own_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn `holler hub serve`");
    let stderr = child.stderr.take().expect("hub stderr is piped");
    let reader = std::io::BufReader::new(stderr);
    let mut reader = Some(reader);
    let port = wait_for(Duration::from_secs(10), || {
        let r = reader.as_mut()?;
        support::read_json_event_pub(r, "listening").map(|v| support::parse_port_pub(&v))
    });
    // Keep draining stderr for the rest of the hub's life — see
    // `support::drain_stderr_forever`'''s doc comment for why dropping the
    // reader here would eventually kill the hub process.
    if let Some(r) = reader {
        support::drain_stderr_forever(r);
    }
    let port = port.unwrap_or_else(|| {
        let _ = child.kill();
        panic!("hub did not report a listening port within 10s");
    });
    Hub::from_parts(port, child)
}

/// Dial the hub's WebSocket listener and split the client into sink + stream.
/// The writer half is a `SplitSink` and the reader half a `SplitStream`
/// (both `Unpin` — `WebSocketStream` is, and `futures`' split wrappers wrap an
/// `Unpin` inner), so they name their concrete types in the return tuple.
async fn connect_split(
    url: &str,
) -> (
    futures_util::stream::SplitSink<WsClient, Message>,
    futures_util::stream::SplitStream<WsClient>,
) {
    let ws: WsClient = connect_async(url)
        .await
        .expect("dial the hub's WebSocket listener")
        .0;
    let (sink, stream) = ws.split();
    (sink, stream)
}

/// A decoded inbound frame, as the test cares about it.
#[derive(Debug)]
enum Frame {
    /// A parsed envelope. `id`/`method`/`code` are `None` when absent;
    /// `is_error` distinguishes an error response from a result.
    Env {
        id: Option<String>,
        method: Option<String>,
        code: Option<String>,
        is_error: bool,
        result: Option<Value>,
    },
    /// A WebSocket close frame.
    Close(u16, String),
    /// The transport went away (the hub dropped the socket) with no frame.
    Eof,
}

impl Frame {
    fn close_code(&self) -> Option<u16> {
        match self {
            Frame::Close(code, _) => Some(*code),
            _ => None,
        }
    }
    fn method(&self) -> Option<&str> {
        match self {
            Frame::Env { method, .. } => method.as_deref(),
            _ => None,
        }
    }
    fn code(&self) -> Option<&str> {
        match self {
            Frame::Env { code, .. } => code.as_deref(),
            _ => None,
        }
    }
    fn is_error(&self) -> bool {
        matches!(self, Frame::Env { is_error: true, .. })
    }
    fn result(&self) -> Option<&Value> {
        match self {
            Frame::Env { result, .. } => result.as_ref(),
            _ => None,
        }
    }
}

/// Send one outbound frame and decode the next inbound frame. The hub's close
/// frames carry an explicit code + reason (never the empty-payload form), so
/// `next_frame` reports them via `Frame::Close` rather than surfacing a peer
/// close as a stream error (see `decode_next`'s note below).
async fn next_frame(
    sink: &mut futures_util::stream::SplitSink<WsClient, Message>,
    stream: &mut futures_util::stream::SplitStream<WsClient>,
    out: Message,
) -> Frame {
    let _ = sink.send(out).await;
    // `split()` does not auto-flush: `SinkExt::send` only enqueues into the
    // write buffer (and `poll_ready` returns Ready immediately without
    // draining it), so a large frame can sit unflushed in the client's
    // buffer until the kernel's send buffer fills and `poll_ready` blocks.
    // An explicit `flush` pushes the frame onto the wire so the hub can
    // process it (and, for an oversized frame, close the socket).
    let _ = sink.flush().await;
    decode_next(stream).await
}

/// Decode the next inbound frame without sending one (a peer close is the
/// first frame on an idle socket).
async fn decode_next(
    stream: &mut futures_util::stream::SplitStream<WsClient>,
) -> Frame {
    loop {
        match stream.next().await {
            None => return Frame::Eof,
            // The hub always sends close frames with an explicit code + reason,
            // so tungstenite surfaces them as `Ok(Close)`. A transport error
            // (aborted teardown) is not a frame — report it as EOF.
            Some(Err(_)) => return Frame::Eof,
            Some(Ok(Message::Text(t))) => {
                let v: Value =
                    serde_json::from_str(t.as_ref()).expect("a hub frame is valid JSON");
                return Frame::Env {
                    id: v.get("id").and_then(Value::as_str).map(str::to_owned),
                    method: v.get("method").and_then(Value::as_str).map(str::to_owned),
                    code: v.pointer("/data/code").and_then(|c| c.as_str()).map(str::to_owned),
                    is_error: v.get("error").is_some(),
                    result: v.get("result").cloned(),
                };
            }
            Some(Ok(Message::Close(frame))) => {
                return match frame {
                    Some(f) => Frame::Close(f.code.into(), f.reason.to_string()),
                    None => Frame::Eof,
                };
            }
            Some(Ok(Message::Ping(_)) | Ok(Message::Pong(_))) => continue,
            Some(Ok(other)) => panic!("unexpected inbound frame: {other:?}"),
        }
    }
}

/// Mint a token and authenticate a fresh WS client to the hub. Per protocol
/// v2 §3, a freshly-minted token is `unused` — it must be **redeemed** via
/// `circuit/join` (which binds it and returns a `credential`) before
/// `circuit/authenticate` will accept it. `join` is a one-shot bootstrap that
/// closes the socket after replying, so this helper uses **two** sockets:
/// socket 1 joins (redeems), socket 2 authenticates with the credential.
/// Returns the authenticated sink/stream (socket 2) plus the token id.
async fn auth_client(
    state: &StateDir,
    hub: &Hub,
    hostname: &str,
) -> (
    futures_util::stream::SplitSink<WsClient, Message>,
    futures_util::stream::SplitStream<WsClient>,
    String,
) {
    let (token_id, secret) = mint_token(state, "hygiene");

    // Socket 1: redeem the join secret (binds the token, returns a credential).
    let (_join_sink, _join_stream, credential) =
        join_and_get_credential(hub, &secret, hostname).await;

    // Socket 2: authenticate with the (now bound) credential.
    let (mut sink, mut stream) = connect_split(&hub.ws_url()).await;
    let auth = Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/authenticate",
                Some(json!({ "token_id": token_id, "credential": credential, "hostname": hostname })),
            ),
        )
            .unwrap_or_default()
    );
    match next_frame(&mut sink, &mut stream, auth).await {
        Frame::Env { code, is_error, .. } => {
            assert!(!is_error, "authenticate was refused (code {:?})", code);
        }
        other => panic!("expected an auth response, got {other:?}"),
    }

    (sink, stream, token_id)
}

/// Join on a bootstrap socket and return the credential. The hub closes the
/// join socket after replying (protocol v2 §3: join is a one-shot bootstrap),
/// so we read the join response via `decode_next` (no outbound frame) rather
/// than `next_frame` (which sends a close the hub has already sent). The
/// sink/stream are dropped by the caller.
async fn join_and_get_credential(
    hub: &Hub,
    secret: &str,
    hostname: &str,
) -> (
    futures_util::stream::SplitSink<WsClient, Message>,
    futures_util::stream::SplitStream<WsClient>,
    String,
) {
    let (mut sink, mut stream) = connect_split(&hub.ws_url()).await;
    let join = Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/join",
                Some(json!({ "secret": secret, "hostname": hostname })),
            ),
        )
            .unwrap_or_default()
    );
    let _ = sink.send(join).await;
    let _ = sink.flush().await;
    // The join response arrives as a text frame; the hub then closes the
    // socket. We read the response (the next frame is the join result, since
    // the hub sends the response before the close).
    let resp = decode_next(&mut stream).await;
    let Frame::Env { result: Some(doc), is_error, .. } = resp else {
        panic!("join did not return a result: {resp:?}");
    };
    assert!(!is_error, "join was refused");
    let credential = doc
        .get("credential")
        .and_then(Value::as_str)
        .expect("join result carries a credential")
        .to_owned();
    (sink, stream, credential)
}

/// The `circuit/ping` request frame (a round-trip canary).
fn ping_frame() -> Message {
    Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/ping",
                Some(json!({})),
            ),
        )
            .unwrap_or_default()
    )
}

// ---------------------------------------------------------------------------
// 1. supersede-on-reauth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reauth_supersedes_and_closes_old_socket_with_notification() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let url = hub.ws_url().clone();

    let (token_id, secret) = mint_token(&state, "supersede");
    let hostname = "b184-supersede";

    // First redeem the join secret on a bootstrap socket (protocol v2 §3:
    // join is a one-shot that closes after replying; it binds the token and
    // returns the long-lived credential that authenticate requires).
    let (_join_sink, _join_stream, credential) =
        join_and_get_credential(&hub, &secret, hostname).await;

    // Old socket: authenticate with the credential (claims the token).
    let (mut old_sink, mut old_stream) = connect_split(&url).await;
    let old_auth = Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/authenticate",
                Some(json!({ "token_id": token_id, "credential": &credential, "hostname": hostname })),
            ),
        )
            .unwrap_or_default()
    );
    match next_frame(&mut old_sink, &mut old_stream, old_auth).await {
        Frame::Env { is_error, .. } => assert!(!is_error, "old authenticate refused"),
        other => panic!("old: expected auth response, got {other:?}"),
    }

    // New socket: authenticate the same token (supersedes the old one).
    let (mut new_sink, mut new_stream) = connect_split(&url).await;
    let new_auth = Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/authenticate",
                Some(json!({ "token_id": token_id, "credential": &credential, "hostname": hostname })),
            ),
        )
            .unwrap_or_default()
    );
    match next_frame(&mut new_sink, &mut new_stream, new_auth).await {
        Frame::Env { is_error, .. } => assert!(!is_error, "new authenticate must succeed"),
        other => panic!("new: expected auth response, got {other:?}"),
    }

    // The old socket now receives a `circuit/superseded` notification…
    let notif = next_frame(&mut old_sink, &mut old_stream, Message::Close(None)).await;
    assert_eq!(
        notif.method(),
        Some("circuit/superseded"),
        "superseded socket must receive a circuit/superseded notification (got {notif:?})"
    );
    // …then a 1000 close frame.
    let close = decode_next(&mut old_stream).await;
    assert_eq!(
        close.close_code(),
        Some(CloseCode::Normal.into()),
        "superseded socket must close with 1000 (got {close:?})"
    );

    // The new socket is still alive (a ping round-trips).
    let pong = next_frame(&mut new_sink, &mut new_stream, ping_frame()).await;
    assert!(
        matches!(pong, Frame::Env { .. }),
        "the new socket must stay alive after the old one is superseded (got {pong:?})"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 2. revoke force-close
// ---------------------------------------------------------------------------

#[tokio::test]
async fn revoke_force_closes_live_connection() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    let hostname = "b184-revoke";
    let (mut sink, mut stream, token_id) = auth_client(&state, &hub, hostname).await;

    // Healthy: a ping round-trips before the revoke.
    let pre = next_frame(&mut sink, &mut stream, ping_frame()).await;
    assert!(matches!(pre, Frame::Env { .. }), "pre-revoke ping must round-trip");

    // Revoke the (bound, connected) token over the control socket.
    let out = holler_cmd(&state)
        .args(["--json", "hub", "token", "revoke", &token_id])
        .env("HOLLER_CLI_NO_UPDATE_CHECK", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `hub token revoke`")
        .wait_with_output()
        .expect("wait on `hub token revoke`");
    assert!(
        out.status.success(),
        "token revoke must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The live socket is force-closed with 1008 (the revoke watcher polls ~100 ms).
    // We are already inside the test's own async runtime, so there is no need
    // to bridge into a sync poll for this: `decode_next` awaits directly on
    // the same stream, bounded by a timeout.
    let close = tokio::time::timeout(Duration::from_secs(5), decode_next(&mut stream))
        .await
        .expect("the live connection must be force-closed after revoke");
    assert_eq!(
        close.close_code(),
        Some(CloseCode::Policy.into()),
        "a revoked live connection must close with 1008 (got {close:?})"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 3. oversized frame → 1009
// ---------------------------------------------------------------------------

#[tokio::test]
async fn oversized_frame_closes_1009() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    let hostname = "b184-oversized";
    let (mut sink, mut stream, _tid) = auth_client(&state, &hub, hostname).await;

    // An authenticated socket that sends a frame over the 2 MiB cap is closed
    // with 1009 (message too big).
    let big = holler_proto::encode(&holler_proto::Envelope::request(
        &holler_proto::CorrelationId::mint_hub(),
        "circuit/say",
        Some(json!({ "to": hostname, "text": "x".repeat(2 * 1024 * 1024 + 1) })),
    ))
    .unwrap_or_default();
    let _ = sink.send(Message::text(big)).await;
    let out = decode_next(&mut stream).await;
    assert_eq!(
        out.close_code(),
        Some(CloseCode::Size.into()),
        "an oversized frame must trigger a 1009 close (got {out:?})"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 4. silent socket → 1001 after the pre-auth timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn silent_socket_closed_after_pre_auth_timeout() {
    let state = StateDir::new();
    let hub = start_hub_with_env(&state, &[("HOLLER_PRE_AUTH_TIMEOUT_MS", "500")]);

    // A socket that opens but never authenticates is closed with 1001 after
    // the (reduced) 500 ms pre-auth timeout.
    let (_sink, mut stream) = connect_split(&hub.ws_url()).await;
    // Same reasoning as the revoke test above: await `decode_next` directly
    // on the test's own runtime, bounded by a timeout, instead of bridging
    // through a sync poll.
    let close = tokio::time::timeout(Duration::from_secs(3), decode_next(&mut stream))
        .await
        .expect("a silent socket must be closed after the pre-auth timeout");
    assert_eq!(
        close.close_code(),
        Some(CloseCode::Away.into()),
        "a silent socket must be closed with 1001 (got {close:?})"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 5. pre-auth cap rejects the 5th socket
// ---------------------------------------------------------------------------

#[tokio::test]
async fn preauth_cap_rejects_65th_socket() {
    let state = StateDir::new();
    let hub = start_hub_with_env(&state, &[("HOLLER_MAX_PREAUTH_CONNECTIONS", "4")]);
    let url = hub.ws_url().clone();

    // With a cap of 4, the first 4 unauthenticated sockets are admitted (the
    // WebSocket handshake completes); the 5th is refused before the handshake
    // and its TCP connection is dropped (a 1013 "try again" refusal).
    let mut admitted = 0usize;
    let mut refused = 0usize;
    for _ in 0..5 {
        match connect_async(&url).await {
            Ok(ws) => {
                // Admitted into the pre-auth pool; drop it to free a slot.
                let (mut sink, _stream) = ws.0.split();
                // Send a real (non-empty) first frame so the admitted socket
                // authenticates and releases its pre-auth semaphore slot
                // promptly (a silent holder would only free on the 20 s
                // timeout). `circuit/ping` as a first frame is refused
                // (unauthenticated) and does not consume a token.
                let _ = sink.send(ping_frame()).await;
                admitted += 1;
                drop(sink);
            }
            Err(_) => refused += 1,
        }
    }
    assert_eq!(admitted, 4, "the first 4 sockets must be admitted (cap 4)");
    assert_eq!(
        refused, 1,
        "the 5th socket must be refused (raw 1013) because the pre-auth cap is reached"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 6. five bad auths lock the peer out
// ---------------------------------------------------------------------------

#[tokio::test]
async fn five_bad_auths_lock_out_peer_for_window() {
    let state = StateDir::new();
    let hub = start_hub_with_env(
        &state,
        &[
            ("HOLLER_LOCKOUT_MAX_FAILURES", "5"),
            ("HOLLER_LOCKOUT_WINDOW_MS", "2000"),
            ("HOLLER_LOCKOUT_DURATION_MS", "2000"),
        ],
    );
    let url = hub.ws_url().clone();

    // Every failed `circuit/authenticate` from this (loopback) peer records a
    // lockout strike and is refused with a **bare 1008 close** — `refuse_auth`'s
    // `Unauthenticated` branch delivers the close without a JSON-RPC error
    // frame (a bad token id is an "unknown credential" refusal, not an
    // `InvalidRequest` error). The 5th strike trips the lockout, so all five
    // of these first-frame refusals carry 1008.
    for i in 0..5 {
        let (mut sink, mut stream) = connect_split(&url).await;
        let bad_auth = Message::text(
            holler_proto::encode(
                &holler_proto::Envelope::request(
                    &holler_proto::CorrelationId::mint_hub(),
                    "circuit/authenticate",
                    Some(json!({ "token_id": format!("nonexistent-{i}"), "credential": "bad" })),
                ),
            )
                .unwrap_or_default()
        );
        let _ = sink.send(bad_auth).await;
        // Read the next inbound frame: a failed authenticate is refused with a
        // bare 1008 close (no error frame — see `refuse_auth`).
        loop {
            let f = decode_next(&mut stream).await;
            if f.close_code().is_some() {
                assert_eq!(
                    f.close_code(),
                    Some(<u16>::from(CloseCode::Policy)),
                    "strike {i} must be refused with a bare 1008 close (got {f:?})"
                );
                break;
            }
            if matches!(f, Frame::Eof) {
                panic!("strike {i}: got EOF before a close frame");
            }
        }
    }

    // The 5th strike has tripped the lockout: a *fresh* connection is now
    // refused with 1008 **before the peer may even send a frame**. (The
    // handshake still completes; the refusal is the first frame the hub sends.)
    let (mut sink, mut stream) = connect_split(&url).await;
    let refused = next_frame(&mut sink, &mut stream, Message::Close(None)).await;
    assert_eq!(
        refused.close_code(),
        Some(<u16>::from(CloseCode::Policy)),
        "a locked-out peer's new connection must be refused with 1008 (got {refused:?})"
    );

    // After the 2 s duration lapses the lockout lifts: a new connection is
    // admitted — the handshake completes and the socket is *not* immediately
    // refused (an admitted socket stays open on idle until the default 20 s
    // pre-auth timeout, well beyond this 1 s window, so the absence of a
    // 1008 close within it is the admission signal).
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let (after_sink, mut after_stream) = connect_async(&url)
        .await
        .expect("the peer must be admitted again once the lockout lapses")
        .0
        .split();
    drop(after_sink);
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let first_after = decode_next(&mut after_stream).await;
    assert!(
        first_after.close_code() != Some(<u16>::from(CloseCode::Policy)),
        "the re-admitted socket must not be refused with 1008 after the lockout lapses (got {first_after:?})"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 7. a slow token store does not stall a sibling ping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn slow_token_store_does_not_stall_sibling_ping() {
    let state = StateDir::new();
    // `HOLLER_STORE_DELAY_MS` makes the hub's async token-store twins wait 200
    // ms before doing the real work — simulating a slow store on the
    // (blocking-pool-isolated) connection path.
    let hub = start_hub_with_env(&state, &[("HOLLER_STORE_DELAY_MS", "200")]);
    let url = hub.ws_url().clone();

    // A slow client: its `circuit/join` hits the slow store and parks ~200 ms.
    let (slow_id, slow_secret) = mint_token(&state, "slow");
    let (mut slow_sink, _slow_stream) = connect_split(&url).await;
    let _ = slow_sink
        .send(Message::text(
            holler_proto::encode(
                &holler_proto::Envelope::request(
                    &holler_proto::CorrelationId::mint_hub(),
                    "circuit/join",
                    Some(json!({ "secret": slow_secret, "hostname": "b184-slow" })),
                ),
            )
                .unwrap_or_default()
        ))
        .await;
    let _ = slow_id;

    // A sibling client: join (binds the token) on a bootstrap socket, then
    // authenticate on a fresh socket (its own store ops), and then time a
    // ping.
    let (sib_id, sib_secret) = mint_token(&state, "sibling");
    let (_sib_join_sink, _sib_join_stream, sib_credential) =
        join_and_get_credential(&hub, &sib_secret, "b184-sib").await;
    let (mut sib_sink, mut sib_stream) = connect_split(&url).await;
    let sib_auth = Message::text(
        holler_proto::encode(
            &holler_proto::Envelope::request(
                &holler_proto::CorrelationId::mint_hub(),
                "circuit/authenticate",
                Some(json!({ "token_id": sib_id, "credential": sib_credential, "hostname": "b184-sib" })),
            ),
        )
            .unwrap_or_default()
    );
    let _ = next_frame(&mut sib_sink, &mut sib_stream, sib_auth).await;

    // The sibling's ping must round-trip without being held up by the slow
    // store (its `spawn_blocking` op does not block the async read path).
    let t0 = std::time::Instant::now();
    let pong = next_frame(&mut sib_sink, &mut sib_stream, ping_frame()).await;
    let delay = t0.elapsed();
    assert!(matches!(pong, Frame::Env { .. }), "the sibling ping must round-trip");
    assert!(
        delay <= Duration::from_millis(20) + Duration::from_secs(1),
        "a 200 ms-slow store must not stall a sibling ping by >20 ms; observed {delay:?}"
    );

    hub.stop(Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// 8. peer address present in `hub status --json`
// ---------------------------------------------------------------------------

#[tokio::test]
async fn peer_addr_present_in_status_json() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    // Authenticate one client so `clients_detail` is non-empty.
    let (_sink, _stream, _tid) = auth_client(&state, &hub, "b184-peeraddr").await;

    let status = support::hub_status_json(&state);
    let detail = status
        .get("clients_detail")
        .and_then(Value::as_array)
        .expect("`hub status --json` must carry a clients_detail array");
    assert!(!detail.is_empty(), "an authenticated client must appear in the status");
    let peer = detail[0]
        .get("peer")
        .and_then(Value::as_str)
        .expect("each client must report a `peer` address");
    // Loopback (127.0.0.1:port or [::1]:port).
    assert!(
        peer.starts_with("127.0.0.1:") || peer.starts_with("[::1]:"),
        "`clients_detail[].peer` must be an ip:address string, got {peer}"
    );

    hub.stop(Duration::from_secs(5));
}
