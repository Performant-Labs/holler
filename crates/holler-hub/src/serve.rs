//! `holler hub serve` — the hub's loopback WebSocket listener, instance lock,
//! and Unix-domain control socket (story #143).
//!
//! The hub listens on **loopback plain `ws` only** (ADR 0006). A non-loopback
//! `--listen` address is refused with exit 3 before anything binds; off-loopbox
//! reachability is a TLS-terminating proxy in front of a loopback listener, not
//! a flag.
//!
//! One `hub serve` per state dir: an advisory `flock` on `<state>/hub/serve.lock`
//! (holding the hub's PID) makes a second `hub serve` in the same state dir
//! exit 3 ("another holler hub is running"). The flock releases on process
//! death, so a crashed hub's lock is reclaimed by the next start.
//!
//! One-shot CLI commands (`hub status`, later `roster`/`say`/…) talk to the
//! live process over the control Unix socket `<state>/hub/control.sock` (mode
//! 0600). The socket speaks the **same JSON-RPC 2.0 envelope as the wire**
//! (reusing `holler-proto`), newline-delimited, with internal `control/…`
//! methods (`control/status` here; `control/roster`/`say`/… in later stories).
//!
//! SIGINT **and** SIGTERM trigger a graceful shutdown: stop accepting, close
//! the sockets, remove the control socket and the instance lock, and exit 0
//! within 5 s.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::task::{Context, Poll};

use futures_util::{Future, Sink, SinkExt, Stream, StreamExt};
use holler_proto::{decode, Envelope, Join, WireError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixListener};
use tokio::signal::unix::{signal, SignalKind};
use tokio_tungstenite::{
    tungstenite::{
        handshake::server::{create_response, Request as WsRequest, Response as WsResponse},
        protocol::Role,
        Error as WsError, Message,
    },
    WebSocketStream,
};

use crate::state::{
    advertise_path, control_sock_path, ensure_dirs, resolve_state_dir, serve_lock_path, HubState,
};

use holler_proto::log::{Component, Direction, Event, Severity};
use holler_proto::Code;

/// Emit a `Warn` refusal/failure event on the given component and log it
/// (story #144: every hub module reports through the shared logging core
/// rather than a raw `eprintln!`). `method` is the fixed event name; `msg` is
/// the human-readable detail (kept verbatim so operators see the same text
/// they saw before this story, and so any test asserting on a specific
/// substring keeps matching — `render_text` appends `reason=<msg>`).
fn warn(component: Component, method: &'static str, msg: String) {
    holler_proto::log::emit(&Event {
        component,
        severity: Severity::Warn,
        direction: Direction::Local,
        method,
        id: None,
        peer: None,
        fields: vec![("reason", msg)],
        frame: None,
    });
}

/// The internal `control/…` method names answered on the control socket
/// (ADR 0006 / the control socket is internal, non-wire — these are NOT in the
/// v2 wire catalog, so the control dispatch does not go through `decode`).
#[allow(dead_code)] // #143 forward-declared for a later story that reads the catalog
pub const CONTROL_METHODS: &[&str] = &["control/status"];

/// Run the hub: validate, lock, bind, serve, then shut down gracefully.
///
/// Returns the **exit code** the caller (the CLI's `hub serve` leaf, a bin —
/// the only place a helper's outcome may turn into an exit) should exit with.
/// A clean signal shutdown returns 0; a runtime failure 1; a fail-closed policy
/// refusal (non-loopback bind, or another hub holds the lock) 3. The hub is a
/// lib, so this *returns* the code rather than exiting (a helper must
/// panic/return, never exit — an exit here would mask the code from the caller).
// Building the tokio runtime is infallible in practice (it only fails if the
// OS refuses to allocate the thread pool), so the `.expect` is unreachable.
#[allow(clippy::expect_used)] // #143
pub fn run(listen: &[String], advertise: Option<&str>) -> i32 {
    // A missing state dir (no `HOLLER_STATE_DIR` and no `$HOME`) is a
    // fail-closed refusal (the bin will exit 3 on this 3).
    let state = match resolve_state_dir() {
        Some(dir) => HubState::from_root(dir),
        None => return 3,
    };
    if let Err(e) = ensure_dirs(&state) {
        warn(
            Component::Control,
            "state_dir_create_failed",
            format!("cannot create state dir {}: {e}", state.root.display()),
        );
        return 1;
    }

    // 1. Refuse any non-loopback listen address before anything binds.
    let addrs: Vec<SocketAddr> = match listen.iter().map(|s| validate_loopback(s)).collect() {
        Some(a) => a,
        None => return 3,
    };

    // 2. Acquire the per-state-dir instance lock (a second hub exits 3).
    let lock = match acquire_lock(&state) {
        Some(g) => g,
        None => return 3,
    };

    // 3-5. Bind the listeners and serve — this is async (tokio's TCP bind is
    // async), so it runs inside the runtime. `serve_forever` returns the exit
    // code (0 on a clean signal shutdown, 1 on a fatal bind error); on a 0 we
    // tear down the artifacts and propagate 0, otherwise we propagate the code.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build the tokio runtime");
    let code = rt.block_on(serve_forever(addrs, advertise, state.clone(), lock));

    if code == 0 {
        // Graceful teardown: remove both artifacts. (The lock guard is dropped
        // above with `state`; a non-zero code skips this, but the flock is
        // released on process death regardless.)
        let _ = std::fs::remove_file(control_sock_path(&state));
        let _ = std::fs::remove_file(serve_lock_path(&state));
    }
    code
}

/// Validate that a `--listen HOST:PORT` address is loopback. Returns `None`
/// (after printing the refusal) if it is not a valid or not a loopback
/// address; `Some(SocketAddr)` (with the port, possibly 0) otherwise. The name
/// `localhost` is **never** resolved (ADR 0006) — it is not a loopback literal
/// and is refused.
fn validate_loopback(addr: &str) -> Option<SocketAddr> {
    let parsed = match addr.parse::<SocketAddr>() {
        Ok(a) => a,
        Err(_) => {
            warn(
                Component::Control,
                "listen_addr_invalid",
                format!("{addr} is not a valid host:port address"),
            );
            return None;
        }
    };
    let loopback = match parsed.ip() {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback(),
    };
    if !loopback {
        warn(
            Component::Control,
            "non_loopback_bind_refused",
            format!(
                "refusing to bind {addr} as plain ws: put a TLS-terminating proxy in front (docs/deploy.md) \u{2014} non-loopback plain ws is not allowed (ADR 0006)"
            ),
        );
        return None;
    }
    Some(parsed)
}

/// Acquire the instance lock. On success return `Some` guard (held for the
/// process lifetime). On a held or unopenable lock, print the refusal and
/// return `None` (the caller exits 3). The `flock` releases on process death,
/// so a crashed hub's lock is reclaimed.
fn acquire_lock(state: &HubState) -> Option<LockGuard> {
    use fs4::FileExt;
    let path = serve_lock_path(state);
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .truncate(true) // the lock file holds only the *current* holder's PID.
        .write(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            warn(
                Component::Control,
                "instance_lock_open_failed",
                format!("cannot open instance lock {}: {e}", path.display()),
            );
            return None;
        }
    };
    // Fully-qualified so we call fs4's flock (not std's inherent `try_lock`,
    // which shadows the trait method and uses a distinct `TryLockError` type).
    match FileExt::try_lock(&file) {
        Ok(()) => {}
        Err(fs4::TryLockError::WouldBlock) => {
            // Another live hub holds the lock. Best-effort: name its pid.
            let pid = std::fs::read_to_string(&path).unwrap_or_default();
            warn(
                Component::Control,
                "instance_lock_held",
                format!(
                    "another holler hub is running (pid {pid}) against {}",
                    state.root.display()
                ),
            );
            return None;
        }
        Err(e) => {
            warn(
                Component::Control,
                "instance_lock_failed",
                format!("cannot lock instance file {}: {e}", path.display()),
            );
            return None;
        }
    }
    // Write our pid so a later refusal can name it.
    let _ = write_pid(&file);
    Some(LockGuard { file, path })
}

/// A held advisory lock. Dropped only if the process dies without a clean
/// shutdown (the flock releases on death); a clean shutdown removes the file
/// explicitly before dropping.
struct LockGuard {
    file: std::fs::File,
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Fully-qualified: use fs4's flock `unlock` (not std's inherent one).
        let _ = fs4::FileExt::unlock(&self.file);
        let _ = std::fs::remove_file(&self.path);
    }
}

fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_at(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

/// Bind the listeners and serve until a signal. Runs inside the tokio runtime
/// (tokio's TCP bind is async). Returns the exit code: 0 on a clean signal
/// shutdown (the caller tears down), 1 on a fatal bind error. A helper returns
/// the code rather than exiting (exiting is the bin's job).
// Installing the SIGINT/SIGTERM handlers only fails if the OS refuses, so the
// two `.expect`s are unreachable.
#[allow(clippy::expect_used)] // #143
async fn serve_forever(
    addrs: Vec<SocketAddr>,
    advertise: Option<&str>,
    state: HubState,
    _lock: LockGuard,
) -> i32 {
    // 3. Bind the WebSocket listeners (port 0 → the OS picks a free port).
    //
    // We deliberately do NOT emit the `listening` readiness event here: the
    // control socket is bound a few lines below, and "listening" must mean the
    // hub is *fully* ready (control socket up, accept loop running) so a caller
    // that observes the event can immediately use the control socket without a
    // "file not found" race. The event is emitted once, after both are up.
    let mut ws_listeners = Vec::new();
    let mut bound_addrs = Vec::new();
    for addr in &addrs {
        match TcpListener::bind(addr).await {
            Ok(l) => match l.local_addr() {
                Ok(actual) => {
                    bound_addrs.push(actual.to_string());
                    ws_listeners.push(l);
                }
                Err(e) => {
                    warn(
                        Component::Wire,
                        "bind_addr_report_failed",
                        format!("cannot report the bound address: {e}"),
                    );
                    return 1;
                }
            },
            Err(e) => {
                warn(
                    Component::Wire,
                    "bind_failed",
                    format!("failed to bind {addr}: {e}"),
                );
                return 1;
            }
        }
    }

    // Record the bound addresses so a `control/status` (same process) can
    // report them in its `listening` array.
    let _ = std::fs::write(
        state.hub_dir.join("listening.json"),
        serde_json::to_string_pretty(&bound_addrs).unwrap_or_default(),
    );

    // Persist the advertise address (used by `token mint`'s join line).
    if let Some(adv) = advertise {
        if let Err(e) = std::fs::write(
            advertise_path(&state),
            format!("{{\"advertise\":\"{adv}\"}}\n"),
        ) {
            warn(
                Component::Control,
                "advertise_persist_failed",
                format!("could not persist advertise config: {e}"),
            );
        }
    }

    // 4. Bind the control Unix socket (mode 0600), removing a stale one first.
    let sock_path = control_sock_path(&state);
    let _ = std::fs::remove_file(&sock_path);
    let uds = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            warn(
                Component::Control,
                "control_socket_bind_failed",
                format!("failed to bind control socket {}: {e}", sock_path.display()),
            );
            return 1;
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&sock_path, std::fs::Permissions::from_mode(0o600));
    }

    // 5. Drive the accept loops until SIGINT/SIGTERM, then return so the
    // caller can tear down and exit 0.
    //
    // A signal (SIGINT or SIGTERM) trips a `oneshot` stop channel; the
    // accept-loop task — otherwise parked in a `select!` on the listeners —
    // watches that channel as a third branch so the signal can interrupt it
    // without waiting for the next connection. The `Sender` stays here; the
    // `Receiver` moves into the spawned accept-loop task.
    let mut sig_int = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sig_term = signal(SignalKind::terminate()).expect("install SIGTERM handler");

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    // The live-circuit registry (issue #182): one per hub process, shared by
    // every accepted WS connection (recording/removing itself as it
    // authenticates/disconnects) and every control-socket connection (`hub
    // token ping` reaches a body's socket through it).
    let registry = crate::live::Registry::new();
    // The roster (issue #186): one per hub process, shared by every accepted
    // WS connection (which advertises/touches/clears it as presence and frames
    // arrive and circuits open/close) and the control socket (which reads it
    // for `holler roster` and runs the TTL sweep). The system clock drives the
    // 45/180/360 s sweep in production (the unit tests inject a manual clock).
    let roster = std::sync::Arc::new(crate::roster::Roster::with_system_clock(&crate::roster::Config::from_env()));
    let accept_handle = tokio::spawn(async move {
        accept_loop(uds, ws_listeners, state, registry, roster, stop_rx).await;
    });

    // 6. Only now — WS listeners bound, control socket bound + mode 0600,
    // accept loop running — is the hub fully ready. Emit the readiness event(s)
    // on stderr (one per bound address; the harness parses `addr`). Emitting
    // here (not in the bind loop) means a caller that sees `listening` can use
    // the control socket immediately, with no "socket not found" window.
    //
    // This is deliberately a raw `eprintln!`, not `log::emit` (#144): the test
    // harness (`tests/support`) parses this exact `{"event":"listening",…}`
    // shape regardless of `--log-format`/`--debug`, so it must stay a fixed,
    // unredacted, always-on wire contract rather than a rendered log line.
    for actual in &bound_addrs {
        eprintln!(r#"{{"event":"listening","addr":"{actual}"}}"#);
    }

    // Park on a signal; when one arrives, trip the stop channel so the
    // accept loop's `select!` unblocks and winds down.
    let mut stop_tx = Some(stop_tx);
    tokio::select! {
        _ = sig_int.recv() => {
            let _ = stop_tx.take().map(|t| t.send(()));
        }
        _ = sig_term.recv() => {
            let _ = stop_tx.take().map(|t| t.send(()));
        }
    }
    // Make sure the channel is tripped even if both signal receivers were
    // consumed (defensive; one of the arms above already ran).
    let _ = stop_tx.take().map(|t| t.send(()));
    // Wait for the accept loop to wind down, then return 0 for a clean shutdown.
    let _ = accept_handle.await;
    0
}

/// Poll the WS and control listeners, spawning a task per connection, until
/// the stop channel is tripped (a SIGINT/SIGTERM arrived) or a listener fails.
async fn accept_loop(
    uds: UnixListener,
    ws_listeners: Vec<TcpListener>,
    state: HubState,
    registry: crate::live::Registry,
    roster: std::sync::Arc<crate::roster::Roster>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let mut accept_any = AcceptAny {
        listeners: &ws_listeners,
        idx: 0,
    };
    loop {
        tokio::select! {
            // A signal arrived: wind down (stop accepting) and return.
            _ = &mut stop_rx => return,
            res = uds.accept() => {
                let (stream, _addr) = match res {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
                tokio::spawn(crate::control_server::handle_control_conn(stream, registry.clone(), roster.clone()));
            }
            res = (&mut accept_any) => {
                let stream = match res {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                tokio::spawn(handle_ws_conn(stream, state.clone(), registry.clone(), roster.clone()));
            }
        }
    }
}

/// A future that resolves when **any** of the WS listeners has a pending
/// accept. We poll the listeners round-robin: `tokio::select!` in the caller
/// drives this, and a `TcpListener::try_accept` returning `WouldBlock` means
/// "nothing pending on this one yet", so we move to the next.
struct AcceptAny<'a> {
    listeners: &'a Vec<TcpListener>,
    idx: usize,
}

impl Future for AcceptAny<'_> {
    type Output = Result<TcpStream, std::io::Error>;
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let n = self.listeners.len().max(1);
        let mut idx = self.idx;
        for _ in 0..n {
            let l = &self.listeners[idx % n];
            match l.poll_accept(cx) {
                Poll::Ready(Ok((stream, _addr))) => {
                    self.idx = (idx + 1) % n;
                    return Poll::Ready(Ok(stream));
                }
                Poll::Ready(Err(e)) => {
                    self.idx = (idx + 1) % n;
                    return Poll::Ready(Err(e));
                }
                // No connection pending on this listener; `poll_accept`
                // registered `cx` with the reactor, so we will be woken when
                // one arrives.
                Poll::Pending => {}
            }
            idx += 1;
        }
        self.idx = idx % n;
        Poll::Pending
    }
}

/// Handle one WebSocket (circuit) connection. The first frame decides the fate
/// of the socket (the fail-closed accept loop, story #143):
///
/// - **not a v2 message** (garbage / batch / shape error) → the matching
///   `-32700`/`-32600` error, then close.
/// - **`circuit/join`** → redeemed via [`crate::join::redeem_join`], which
///   replies with the `{client_id, credential}` (or a `join_failed` error) and
///   closes the one-shot socket.
/// - **anything else** (a request or notification that is not join) →
///   `-32002 unauthenticated`, then close.
///
/// The socket is **always** closed after the first frame on this story: a join
/// is a one-shot bootstrap (docs §3), so there is no talk yet.
///
/// Perform the **server side** of the WebSocket opening handshake over an
/// already-accepted loopback `TcpStream`, and return the upgraded
/// `WebSocketStream`.
///
/// tokio-tungstenite ships only a *client* async handshake (`connect_async`);
/// its `handshake::server` machine is blocking (`std::io::Read`/`Write`), which
/// a tokio `TcpStream` does not implement. So we do the handshake by hand on
/// the async socket: read the upgrade `GET` (until the blank line), parse it,
/// compute the `Sec-WebSocket-Accept` via tungstenite's own (tested)
/// `create_response`, write the `101` back with `AsyncWrite`, and hand the
/// socket — plus any bytes we over-read — to `from_partially_read` so no frame
/// data is lost. A client that sends a non-WebSocket `GET` (or nothing) is
/// dropped (returns `None`).
async fn server_handshake(mut stream: TcpStream) -> Option<WebSocketStream<TcpStream>> {
    // 1. Drain the HTTP upgrade request: read until the end of the header
    //    section (`\r\n\r\n`). A WebSocket `GET` has no body, so the bytes
    //    after the blank line (if any) are already the first WS frame.
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        match stream.read(&mut tmp).await {
            Ok(0) => return None, // peer closed before sending a request.
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => return None,
        }
        // The header section ends at the first `\r\n\r\n`.
        if let Some(pos) = find_blank_line(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            return None; // header too large: not a sane upgrade request.
        }
    };

    // `head` is the request (up to and including the blank line); `rest` is
    // any bytes already received past the blank line (the start of a frame).
    let (head, rest) = buf.split_at(header_end);

    // 2. Parse the request and build the 101 response via tungstenite's own
    //    (correct-GUID) accept-key computation.
    let request: WsRequest = match parse_upgrade_request(head) {
        Some(r) => r,
        None => return None, // not a parseable HTTP request (or bad method/headers).
    };
    let response: WsResponse = match create_response(&request) {
        Ok(r) => r,
        Err(_) => return None, // rejected the upgrade (bad key/headers/method).
    };

    // 3. Serialise the 101 response to bytes and write it to the peer.
    let bytes = serialize_response(&response);
    if stream.write_all(&bytes).await.is_err() {
        return None;
    }
    if stream.flush().await.is_err() {
        return None;
    }

    // 4. Wrap the (already-upgraded) socket, folding in any over-read bytes so
    //    the first frame is not lost. No handshake is repeated.
    Some(WebSocketStream::from_partially_read(stream, rest.to_vec(), Role::Server, None).await)
}

/// The offset (exclusive) just past the first `\r\n\r\n` in `buf`, or `None`.
fn find_blank_line(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Parse a raw HTTP upgrade request (the bytes up to the blank line) into a
/// tungstenite server `Request`. Returns `None` if it does not parse as an
/// HTTP/1.1 `GET` with headers (a non-WebSocket peer).
fn parse_upgrade_request(head: &[u8]) -> Option<WsRequest> {
    use httparse::Request as RawRequest;
    // httparse parses in place; the header fields borrow from `head`, so the
    // scratch buffer must outlive the `Request` we build. 64 headers is far
    // more than a WebSocket upgrade carries (Host/Connection/Upgrade/Version/Key).
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut raw = RawRequest::new(&mut headers);
    match raw.parse(head).ok()? {
        httparse::Status::Complete(_) => {}
        // An incomplete parse (a header line we read too early) is not a
        // usable request.
        httparse::Status::Partial => return None,
    }
    let method = raw.method?;
    let path = raw.path.unwrap_or("/");
    // A WebSocket upgrade is always HTTP/1.1 (httparse reports the numeric
    // version, `1`); the response below is emitted as HTTP/1.1.
    let mut builder = http::Request::builder()
        .method(method)
        .uri(path)
        .version(http::Version::HTTP_11);
    for h in raw.headers.iter() {
        // httparse's header name/value borrow from `head`; the `http` crate
        // wants owned (or `'static`) header parts, so copy via `from_bytes`
        // (the same route tungstenite's own handshake takes).
        let name = match http::header::HeaderName::from_bytes(h.name.as_bytes()) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let value = match http::header::HeaderValue::from_bytes(h.value) {
            Ok(v) => v,
            Err(_) => continue,
        };
        builder = builder.header(name, value);
    }
    builder.body(()).ok()
}

/// Serialise a tungstenite `101` `Response<()>` to the exact bytes an HTTP
/// writer would emit (status line, each header, then a blank line). We avoid
/// tungstenite's `write_response` (it needs a *sync* `Write`, which a tokio
/// `TcpStream` is not) by emitting the bytes ourselves and writing with
/// `AsyncWrite`.
fn serialize_response(response: &WsResponse) -> Vec<u8> {
    use http::header::HeaderName;
    let mut out = Vec::new();
    out.extend_from_slice(b"HTTP/1.1 101 Switching Protocols\r\n");
    for (name, value) in response.headers() {
        let name: &HeaderName = name;
        let value = value.as_bytes();
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out
}

/// Read frames off a freshly-upgraded socket until a *text* frame arrives,
/// skipping ping/pong (and the untyped `Frame`), replying to a `Binary` first
/// frame with `invalid_request`, and closing the socket (returning `None`) on a
/// close or EOF. Returns the first text frame's payload.
async fn read_first_text_frame<S>(
    sink: &mut S,
    stream: &mut (impl Stream<Item = Result<Message, WsError>> + Unpin),
) -> Option<String>
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    loop {
        match stream.next().await {
            None => {
                // Peer closed before sending anything.
                close(sink).await;
                return None;
            }
            Some(Ok(Message::Text(t))) => return Some(t.to_string()),
            Some(Ok(Message::Binary(_))) => {
                // A binary first frame is not a v2 message.
                send_error(sink, None, Code::InvalidRequest, "a binary frame is not a v2 message").await;
                close(sink).await;
                return None;
            }
            Some(Ok(Message::Ping(_)))
            | Some(Ok(Message::Pong(_)))
            | Some(Ok(Message::Frame(_))) => continue,
            Some(Ok(Message::Close(_))) | Some(Err(_)) => {
                close(sink).await;
                return None;
            }
        }
    }
}

async fn handle_ws_conn(
    stream: TcpStream,
    state: HubState,
    registry: crate::live::Registry,
    roster: std::sync::Arc<crate::roster::Roster>,
) {
    let ws = match server_handshake(stream).await {
        Some(ws) => ws,
        None => return, // not a WebSocket client (or it left mid-handshake).
    };
    let (mut sink, mut stream) = ws.split();

    // Read the first frame (skipping ping/pong; a close or EOF ends the socket).
    let first = match read_first_text_frame(&mut sink, &mut stream).await {
        Some(t) => t,
        None => return,
    };

    // `first` is a text frame; run it through the codec.
    let env = match decode(&first) {
        Ok(e) => e,
        Err(e) => {
            // A framing failure (parse / batch / shape / version / unknown
            // method / bad id). Reply with the codec's own code, then close.
            send_error(&mut sink, None, e.code(), &e.to_string()).await;
            close(&mut sink).await;
            return;
        }
    };

    // Decide by method (the fail-closed rule). A response or error as the
    // *first* frame on a fresh socket is not a join/auth: refuse it as
    // unauthenticated (the `None` arm).
    match env.method() {
        // The one-shot bootstrap (docs §3): the body presents a one-time join
        // secret; the hub redeems it for a `client_id` + long-lived
        // `credential`, replies, and closes the socket. (The normal
        // `circuit/authenticate` path — re-authenticating an existing body —
        // is a later story; on a fresh socket it is still refused below.)
        Some("circuit/join") => {
            let params = match holler_proto::typed_params::<Join>(&env) {
                Ok(p) => p,
                Err(e) => {
                    send_error(&mut sink, env.id(), Code::InvalidParams, &e.message).await;
                    close(&mut sink).await;
                    return;
                }
            };
            crate::join::redeem_join(&mut sink, env.id(), params, &state).await;
        }
        // A returning body re-authenticating with its persisted credential
        // (issue #182). On success this holds the socket for the whole live
        // session (hello exchange, presence, ping); it only returns once the
        // circuit ends. Split into its own fn to keep this dispatch's
        // cognitive complexity under the workspace threshold.
        Some("circuit/authenticate") => {
            dispatch_authenticate(&mut sink, &mut stream, &env, &state, &registry, &roster).await;
        }
        // Any other method on a fresh socket, or a response/error where a
        // request is expected: unauthenticated.
        _ => {
            send_error(
                &mut sink,
                env.id(),
                Code::Unauthenticated,
                "not authenticated",
            )
            .await;
            close(&mut sink).await;
        }
    }
}

/// The `circuit/authenticate` arm of [`handle_ws_conn`], split out to keep
/// that dispatch's cognitive complexity under the workspace threshold. Parses
/// the params (a bad shape is `-32602 invalid_params`) and hands off to
/// [`crate::circuit::handle_authenticated`], which owns the entire live
/// session from here.
async fn dispatch_authenticate(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    stream: &mut (impl Stream<Item = Result<Message, WsError>> + Unpin),
    env: &Envelope,
    state: &HubState,
    registry: &crate::live::Registry,
    roster: &std::sync::Arc<crate::roster::Roster>,
) {
    let params = match holler_proto::typed_params::<holler_proto::Authenticate>(env) {
        Ok(p) => p,
        Err(e) => {
            send_error(sink, env.id(), Code::InvalidParams, &e.message).await;
            close(sink).await;
            return;
        }
    };
    crate::circuit::handle_authenticated(sink, stream, env.id(), params, state, registry, roster)
        .await;
}

/// Send an error envelope (echoing `id` when given) as a text frame, then let
/// the sink flush.
pub(crate) async fn send_error(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    id: Option<&str>,
    code: Code,
    message: &str,
) {
    let frame = match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
        Some(cid) => Envelope::error_frame(&cid, &WireError::new(code, message, None)),
        None => Envelope::Error {
            id: id.map(str::to_owned),
            error: WireError::new(code, message, None),
        },
    };
    let text = holler_proto::encode(&frame).unwrap_or_default();
    if sink.send(Message::text(text)).await.is_err() {
        return;
    }
    let _ = sink.flush().await;
}

/// Send a WS close frame and flush it to the peer (the peer's client auto-replies
/// to a close frame; flushing guarantees ours is on the wire before we drop).
pub(crate) async fn close(sink: &mut (impl Sink<Message, Error = WsError> + Unpin)) {
    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.flush().await;
}

// The control-socket **server** side (its own dispatch and the `hub token
// ping` live probe) lives in [`crate::control_server`] — moved out of this
// file (issue #182) to keep `serve.rs` under the file-size guard as the
// authenticated live-session path (`circuit/authenticate` → `circuit/hello` →
// presence) landed here.
