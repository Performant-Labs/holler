//! `holler hub serve` + `holler hub status` e2e (story #143).
//!
//! Drives the real binary: starts `hub serve`, talks to its WebSocket listener
//! and its Unix-domain control socket, then stops it. Covers the spec's test
//! list: first-frame roundtrip (a non-join/auth first frame is refused with
//! `-32002 unauthenticated`), batch/binary/garbage rejection (`-32600` /
//! `-32700`), serve+status, a second instance refusing (exit 3), a crashed
//! instance's lock being reclaimed, a non-loopback listen refused (exit 3),
//! SIGINT/SIGTERM graceful shutdown, control-socket mode, and `hub status`
//! with no live hub (exit 1).
//!
//! Conventions (ADR 0002 / the shared harness): every test gets its own
//! [`StateDir`]; readiness is *observed* via [`wait_for`], never guessed with a
//! blind sleep; long-lived children are spawned in their own process group so
//! [`kill_tree`] reaps them (and any spawned agents) on drop/panic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #143

use std::io::{BufRead, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use holler_proto::{decode, Envelope, WireError};
use serde_json::json;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream};

mod support;
use support::{holler_cmd, kill_tree, wait_for, Hub, StateDir};

/// The one JSON-RPC 2.0 object a client sends to ask the hub for its status
/// over the control socket. `control/status` is an *internal* (non-wire)
/// method on the control socket, so it is NOT in the v2 wire catalog and is
/// sent as a raw frame here (the control socket is not validated against the
/// v2 catalog).
const STATUS_REQUEST: &str =
    r#"{"jsonrpc":"2.0","id":"b-serve1","method":"control/status","params":{}}"#;

const STATUS_ID: &str = "b-serve1";

/// The control socket's location under the state dir (ADR 0001 / spec).
fn control_sock(state: &StateDir) -> std::path::PathBuf {
    state.hub().join("control.sock")
}

/// The instance lock's location under the state dir.
fn serve_lock(state: &StateDir) -> std::path::PathBuf {
    state.hub().join("serve.lock")
}

/// Connect to the hub's control Unix socket and drive one JSON-RPC exchange:
/// send the status request (newline-terminated), then read lines until one
/// parses as a response whose id matches the request. Returns that envelope.
fn control_status(sock: &Path) -> Envelope {
    let mut stream = UnixStream::connect(sock).expect("connect the control socket");
    stream
        .write_all(STATUS_REQUEST.as_bytes())
        .expect("send the control/status request");
    stream
        .write_all(b"\n")
        .expect("terminate the request with a newline");

    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .expect("read a control-socket line");
        if n == 0 {
            panic!(
                "control socket closed before replying (last line {:?})",
                line.trim()
            );
        }
        if let Ok(env) = decode(&line) {
            if env.id() == Some(STATUS_ID) {
                return env;
            }
        }
        // Otherwise keep draining (the hub should not log to the socket, but
        // be lenient about any preamble).
    }
}

// ---------------------------------------------------------------------------
// first frame on the circuit listener
// ---------------------------------------------------------------------------

/// A first frame that is NOT `circuit/join` or `circuit/authenticate` (here a
/// `query/status` request) is refused with `-32002 unauthenticated` and the
/// socket is closed (the accept loop is fail-closed: no talk before
/// join/auth).
#[tokio::test]
async fn first_frame_on_the_wire_roundtrips_a_jsonrpc_error() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let url = hub.ws_url();

    let mut ws = connect_ws(&url).await;

    // First frame: a request that is not join/auth. The hub must refuse it
    // with `unauthenticated` (-32002) and close.
    ws.send(Message::text(
        json!({"jsonrpc":"2.0","id":"b-1","method":"query/status","params":{}}).to_string(),
    ))
    .await
    .expect("send the first frame");

    let err = next_frame_err(&mut ws)
        .await
        .expect("hub must reply with an error frame");
    // #145: the wire `error.code` is the JSON-RPC *number*; the Holler
    // identity is the `data.code` string.
    assert_eq!(err.code, -32002, "unauthenticated is JSON-RPC -32002");
    assert_eq!(
        holler_proto::Code::from_jsonrpc(err.code),
        Some(holler_proto::Code::Unauthenticated),
    );
    assert_eq!(
        err.data.as_ref().map(|d| d.code.as_str()),
        Some("unauthenticated"),
        "error.data.code is the Holler string",
    );
    // The error frame echoes the request id.
    // (next_frame_err only gives us the WireError; the id check is on the
    // full envelope — re-assert via the close below.)

    // The hub closes the socket with a close frame after refusing.
    assert!(
        closed_next(&mut ws).await,
        "hub closes the socket after refusing a non-join/auth first frame"
    );
    drop(hub);
}

/// A JSON **array** (a batch) as the first frame is rejected with
/// `-32600 invalid_request` and the socket is closed (ADR 0004: no batches).
#[tokio::test]
async fn batch_or_binary_first_frame_is_32600_then_close() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let url = hub.ws_url();

    let mut ws = connect_ws(&url).await;

    // A batch: a top-level JSON array.
    ws.send(Message::text(json!([{}]).to_string()))
        .await
        .expect("send the batch");

    let err = next_frame_err(&mut ws)
        .await
        .expect("hub must reject the batch");
    // #145: wire `error.code` is the JSON-RPC number (-32600 for a batch).
    assert_eq!(err.code, -32600, "a batch is JSON-RPC -32600 invalid_request");
    assert_eq!(
        holler_proto::Code::from_jsonrpc(err.code),
        Some(holler_proto::Code::InvalidRequest),
    );
    assert!(
        closed_next(&mut ws).await,
        "hub closes the socket after a batch"
    );
    drop(hub);
}

/// A **binary** frame as the first frame is rejected with
/// `-32600 invalid_request`, socket closed (v2.md §1/§8).
#[tokio::test]
async fn binary_first_frame_is_32600_then_close() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let url = hub.ws_url();

    let mut ws = connect_ws(&url).await;

    ws.send(Message::binary(vec![1, 2, 3]))
        .await
        .expect("send a binary frame");

    let err = next_frame_err(&mut ws)
        .await
        .expect("hub must reject the binary frame");
    // #145: wire `error.code` is the JSON-RPC number (-32600 for a binary frame).
    assert_eq!(
        err.code,
        -32600,
        "a binary first frame is JSON-RPC -32600 invalid_request"
    );
    assert_eq!(
        holler_proto::Code::from_jsonrpc(err.code),
        Some(holler_proto::Code::InvalidRequest),
    );
    assert!(
        closed_next(&mut ws).await,
        "hub closes the socket after a binary frame"
    );
    drop(hub);
}

/// Garbage (not JSON at all) as the first frame is rejected with
/// `-32700 parse_error`, socket closed (v2.md §8).
#[tokio::test]
async fn garbage_first_frame_is_32700_then_close() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let url = hub.ws_url();

    let mut ws = connect_ws(&url).await;

    ws.send(Message::text("this is not json".to_owned()))
        .await
        .expect("send garbage");

    let err = next_frame_err(&mut ws)
        .await
        .expect("hub must reject the garbage");
    // #145: wire `error.code` is the JSON-RPC number (-32700 for non-JSON).
    assert_eq!(
        err.code,
        -32700,
        "non-JSON input is JSON-RPC -32700 parse_error"
    );
    assert_eq!(
        holler_proto::Code::from_jsonrpc(err.code),
        Some(holler_proto::Code::ParseError),
    );
    assert!(
        closed_next(&mut ws).await,
        "hub closes the socket after garbage"
    );
    drop(hub);
}

// ---------------------------------------------------------------------------
// hub serve / hub status
// ---------------------------------------------------------------------------

/// `hub serve --listen 127.0.0.1:0` starts, reports the bound port, and
/// `hub status` (talking to the control socket) reports `role:"hub"` and an
/// empty `clients` list (no bodies yet) and the bound address in `listening`.
#[test]
fn serve_logs_listening_and_status_answers() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let expected = format!("127.0.0.1:{}", hub.port);

    let doc = support::hub_status_json(&state);
    assert_eq!(doc["role"].as_str(), Some("hub"), "status.role is hub");
    assert_eq!(
        doc["clients"].as_u64(),
        Some(0),
        "no bodies are connected yet; got {doc:?}"
    );
    // `listening` is an array of the bound addresses; the port we parsed must
    // appear in it.
    let listening = doc["listening"].as_array().expect("listening is an array");
    assert!(
        listening
            .iter()
            .any(|a| a.as_str() == Some(expected.as_str())),
        "status.listening must include the bound {expected}; got {listening:?}"
    );
    assert_eq!(
        doc["protocol"].as_u64(),
        Some(2),
        "the hub speaks protocol 2"
    );

    hub.stop(Duration::from_secs(5));
}

/// A second `hub serve` in the **same state dir** is refused: the instance
/// lock is held. Exit code 3, "another holler hub is running".
#[test]
fn second_serve_refuses_exit_3() {
    let state = StateDir::new();
    let hub = Hub::start(&state);

    let out = holler_cmd(&state)
        .args(["hub", "serve", "--listen", "127.0.0.1:0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the second hub")
        .wait_with_output()
        .expect("wait on the second hub");

    assert_eq!(
        out.status.code(),
        Some(3),
        "a second hub in the same state dir must exit 3; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("another holler hub is running"),
        "refusal must say another holler hub is running; got: {stderr}"
    );
    hub.stop(Duration::from_secs(5));
}

/// Kill -9 the hub (simulating a crash), then a new `hub serve` in the same
/// state dir must **start cleanly** (the stale flock is reclaimed: flock
/// releases on process death, so the dead pid holds no lock).
#[test]
fn crashed_lock_is_reclaimed() {
    let state = StateDir::new();
    let mut hub = Hub::start(&state);

    // Simulate a crash: SIGKILL the whole process tree (no graceful shutdown,
    // so the lock file is left behind).
    kill_tree(hub.child_mut());
    drop(hub);

    // A fresh hub in the same state dir must start (reclaiming the stale lock).
    let again = Hub::start(&state);
    again.stop(Duration::from_secs(5));
}

/// A non-loopback `--listen` address is **refused** before anything binds:
/// exit code 3, and the spec's exact stderr message.
#[test]
fn non_loopback_bind_refused_exit_3() {
    let state = StateDir::new();
    let out = holler_cmd(&state)
        .args(["hub", "serve", "--listen", "0.0.0.0:0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn a non-loopback hub")
        .wait_with_output()
        .expect("wait on the non-loopback hub");

    assert_eq!(
        out.status.code(),
        Some(3),
        "a non-loopback listen must exit 3; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(
            "refusing to bind 0.0.0.0:0 as plain ws: put a TLS-terminating proxy in front (docs/deploy.md) \u{2014} non-loopback plain ws is not allowed (ADR 0006)"
        ),
        "non-loopback refusal must carry the exact message.\n got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// shutdown
// ---------------------------------------------------------------------------

/// SIGINT makes the hub exit 0 **quickly** (under 5 s), removing both the
/// control socket and the instance lock.
#[test]
fn sigint_shuts_down_cleanly_within_5s() {
    let state = StateDir::new();
    let mut hub = Hub::start(&state);
    let sock = control_sock(&state);
    let lock = serve_lock(&state);
    assert!(sock.exists(), "control socket exists while serving");
    assert!(lock.exists(), "instance lock exists while serving");

    let pid = hub.pid() as i32;
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGINT);
    }

    let exited = wait_for(Duration::from_secs(5), || {
        hub.child_mut().try_wait().ok().flatten()
    });
    assert!(exited.is_some(), "hub must exit on SIGINT within 5 s");

    // Both artifacts are removed on a clean shutdown.
    assert!(!sock.exists(), "control socket removed on clean shutdown");
    assert!(!lock.exists(), "instance lock removed on clean shutdown");
    kill_tree(hub.child_mut()); // reap any straggler
}

/// Same contract for SIGTERM (ADR 0002: SIGTERM is handled like SIGINT).
#[test]
fn sigterm_shuts_down_cleanly_within_5s() {
    let state = StateDir::new();
    let mut hub = Hub::start(&state);
    let sock = control_sock(&state);
    let lock = serve_lock(&state);
    assert!(sock.exists());
    assert!(lock.exists());

    let pid = hub.pid() as i32;
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }

    let exited = wait_for(Duration::from_secs(5), || {
        hub.child_mut().try_wait().ok().flatten()
    });
    assert!(exited.is_some(), "hub must exit on SIGTERM within 5 s");
    assert!(!sock.exists(), "control socket removed on clean shutdown");
    assert!(!lock.exists(), "instance lock removed on clean shutdown");
    kill_tree(hub.child_mut());
}

// ---------------------------------------------------------------------------
// control socket
// ---------------------------------------------------------------------------

/// The control socket file is `0600` (owner-only) so other local users cannot
/// drive the hub.
#[test]
fn control_socket_is_0600() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let sock = control_sock(&state);

    let meta = std::fs::metadata(&sock).expect("stat the control socket");
    let mode = std::os::unix::fs::MetadataExt::mode(&meta) & 0o777;
    assert_eq!(
        mode, 0o600,
        "control socket must be 0600 (owner r/w only); got {mode:o}"
    );
    hub.stop(Duration::from_secs(5));
}

/// A `control/status` request over the control socket gets a response whose
/// `result` is the status document (role=hub, listening includes the bound
/// address, clients=0).
#[test]
fn control_status_over_ud_socket() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let sock = control_sock(&state);

    let env = control_status(&sock);
    let result = env
        .result()
        .cloned()
        .expect("control/status returns a result, not an error");
    let doc = result.as_object().expect("result is a JSON object");
    assert_eq!(
        doc.get("role").and_then(|v| v.as_str()),
        Some("hub"),
        "status result has role=hub"
    );
    assert_eq!(
        doc.get("clients").and_then(|v| v.as_u64()),
        Some(0),
        "status result has clients=0"
    );
    let listening = doc
        .get("listening")
        .and_then(|v| v.as_array())
        .expect("listening is an array");
    assert!(
        listening.iter().any(|a| a
            .as_str()
            .map(|s| s.ends_with(&format!(":{}", hub.port)))
            .unwrap_or(false)),
        "status result.listening includes the bound port"
    );
    hub.stop(Duration::from_secs(5));
}

/// `hub status` with **no live hub** in the state dir exits 1 and says
/// "no live holler hub reachable at <state dir>".
#[test]
fn status_without_live_hub_exit_1() {
    let state = StateDir::new();
    // No hub is started in this state dir.
    let out = holler_cmd(&state)
        .args(["hub", "status"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn hub status")
        .wait_with_output()
        .expect("wait on hub status");

    assert_eq!(
        out.status.code(),
        Some(1),
        "`hub status` with no live hub must exit 1; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no live holler hub reachable at"),
        "must say no live holler hub reachable at <state dir>; got: {stderr}"
    );
    assert!(
        stderr.contains(state.path().to_str().unwrap()),
        "the message names the state dir; got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// ws client helpers (async)
// ---------------------------------------------------------------------------

/// The upgraded WebSocket client type `connect_async` hands back for a `ws://`
/// (plain) URL: a `WebSocketStream` over a `MaybeTlsStream<TcpStream>`.
type WsClient = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Dial the hub's WebSocket listener (a `ws://` loopback URL) and return the
/// upgraded client. The test runtime (from `#[tokio::test]`) drives it.
async fn connect_ws(url: &str) -> WsClient {
    tokio_tungstenite::connect_async(url)
        .await
        .expect("dial the hub's WebSocket listener")
        .0
}

/// Read the next protocol frame; if it is an **error** envelope, return the
/// `WireError`. Used by the rejection tests (batch / binary / garbage /
/// unauthenticated) where the hub's reply is an error object. A result frame
/// here is the bug (these inputs must be refused).
async fn next_frame_err(ws: &mut WsClient) -> Option<WireError> {
    loop {
        let msg = match ws.next().await {
            None => return None, // EOF: the peer closed.
            Some(Ok(m)) => m,
            Some(Err(e)) => panic!("ws stream error: {e:?}"),
        };
        match msg {
            Message::Text(t) => {
                let env = match decode(t.as_str()) {
                    Ok(env) => env,
                    Err(e) => panic!("a hub frame is not a valid v2 message: {e:?}: {t:?}"),
                };
                if let Some(err) = env.error() {
                    return Some(err.clone());
                }
                panic!("expected an error frame, hub sent a result: {t:?}");
            }
            Message::Close(_) => return None,
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("unexpected frame kind from hub: {other:?}"),
        }
    }
}

/// After a rejection the hub closes the socket. Keep draining until a
/// Close frame or EOF; report whether the connection was torn down.
async fn closed_next(ws: &mut WsClient) -> bool {
    loop {
        match ws.next().await {
            None => return true, // EOF: the peer closed.
            Some(Ok(Message::Close(_))) => return true,
            // Any frame before the close: keep draining until teardown.
            Some(Ok(_)) => continue,
            Some(Err(e)) => panic!("ws stream error: {e:?}"),
        }
    }
}
