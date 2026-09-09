//! `holler hub serve` — the hub's loopback WebSocket listener, instance lock,
//! and Unix-domain control socket (story #143).
//!
//! Loopback plain `ws` only (ADR 0006): a non-loopback `--listen` is refused
//! with exit 3 before anything binds. One `hub serve` per state dir: an
//! advisory `flock` on `<state>/hub/serve.lock` (holding the hub's PID) makes a
//! second `hub serve` exit 3; the flock releases on process death. One-shot CLI
//! commands talk to the live process over the control Unix socket
//! `<state>/hub/control.sock` (mode 0600), speaking the same JSON-RPC 2.0
//! envelope as the wire (reusing `holler-proto`), newline-delimited.
//! SIGINT/SIGTERM trigger a graceful shutdown (exit 0 within 5 s). The
//! pre-auth path, supersede/revoke/lockout close codes, and the per-
//! connection reader/writer pair are story #184.
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use holler_proto::{decode, Envelope};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream, UnixListener};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::state::{
    advertise_path, control_sock_path, ensure_dirs, resolve_state_dir, serve_lock_path, HubState,
};
use crate::{connection::Hygiene, lockout::Lockout};
use crate::registry::{ConnectionHandle, Registry};

use holler_proto::log::{Component, Direction, Event, Severity};
use holler_proto::Code;

/// Emit a `Warn` event on the given component (story #144).
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

/// Internal `control/…` method names (ADR 0006; not in the v2 wire catalog).
#[allow(dead_code)] // #143 forward-declared for a later story that reads the catalog
pub const CONTROL_METHODS: &[&str] = &["control/status"];

/// Run the hub: validate, lock, bind, serve, then shut down gracefully.
/// Returns the exit code (0 clean, 1 runtime failure, 3 fail-closed refusal).
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

/// Validate that a `--listen` address is loopback (ADR 0006). `None` if not.
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

/// Acquire the instance lock. `None` if held or unopenable (exits 3).
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

/// A held advisory lock (dropped on process death or clean shutdown).
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

/// Bind and serve until a signal (async). Returns the exit code.
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
    //
    // The live-connection registry (#184), the connection hygiene guard
    // (frame cap / pre-auth timeout / pre-auth cap), and the failed-auth
    // lockout are shared across the accept loop and every connection task.
    let registry = Registry::new();
    let hygiene = Hygiene::new();
    let lockout = Lockout::new();
    // A `hub token revoke`/`delete` runs in a separate CLI process that edits
    // the token store's file; the watcher polls the store and force-closes
    // (1008) any live socket whose token was revoked (see control_io).
    tokio::spawn(crate::control_io::registry_revoke_watcher(registry.clone(), state.clone()));
    let mut sig_int = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sig_term = signal(SignalKind::terminate()).expect("install SIGTERM handler");

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    let accept_handle = tokio::spawn(async move {
        accept_loop(uds, ws_listeners, stop_rx, registry, hygiene, lockout, state.clone()).await;
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

/// Poll the listeners, spawning a task per connection, until stopped.
async fn accept_loop(
    uds: UnixListener,
    ws_listeners: Vec<TcpListener>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
    registry: Arc<Registry>,
    hygiene: Arc<Hygiene>,
    lockout: Arc<Lockout>,
    state: HubState,
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
                tokio::spawn(handle_control_conn(stream, registry.clone()));
            }
            res = (&mut accept_any) => {
                let (stream, peer) = match res {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
                let peer_str = peer.to_string();
                // Pre-auth hygiene (#184): a socket that opens but does not
                // authenticate within the cap is refused with 1013 before it is
                // even allowed to do the WebSocket handshake.
                if !hygiene.try_begin_preauth() {
                    warn(
                        Component::Wire,
                        "preauth_cap_exceeded",
                        format!("refusing pre-auth connection from {peer_str} (cap reached)"),
                    );
                    close_raw_stream(stream).await;
                    hygiene.end_preauth();
                    continue;
                }
                tokio::spawn(handle_ws_conn(
                    stream,
                    peer_str,
                    state.clone(),
                    registry.clone(),
                    hygiene.clone(),
                    lockout.clone(),
                ));
            }
        }
    }
}

/// A future that resolves when any WS listener has a pending accept.
struct AcceptAny<'a> {
    listeners: &'a Vec<TcpListener>,
    idx: usize,
}

impl Future for AcceptAny<'_> {
    type Output = Result<(TcpStream, SocketAddr), std::io::Error>;
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let n = self.listeners.len().max(1);
        let mut idx = self.idx;
        for _ in 0..n {
            let l = &self.listeners[idx % n];
            match l.poll_accept(cx) {
                Poll::Ready(Ok((stream, addr))) => {
                    self.idx = (idx + 1) % n;
                    return Poll::Ready(Ok((stream, addr)));
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

// The WebSocket opening-handshake helpers live in `handshake.rs` (the file
// stays under the 900-line lint gate); `server_handshake` is used by the
// accept loop below.
pub use crate::handshake::server_handshake;

/// Handle one WebSocket (circuit) connection end to end (story #184).
/// The first frame decides the socket's fate: non-v2 → codec error + close;
/// non-join/auth → `-32002 unauthenticated` + close; join/auth → register
/// (superseding any older socket on the same token) and enter the post-auth
/// loop. Hygiene guards the handshake (pre-auth cap → 1013, timeout → 1001,
/// failed auth → lockout strike → 1008 on the next attempt).
pub struct PreauthGuard {
    hygiene: Arc<Hygiene>,
    armed: bool,
}

impl Drop for PreauthGuard {
    fn drop(&mut self) {
        if self.armed {
            self.hygiene.end_preauth();
        }
    }
}

/// Close a pre-auth socket after a refused authentication (1008 = bare close,
/// anything else = codec error + close). The writer task delivers the frames.
async fn refuse_auth(ctx: &mut ConnCtx<'_>, id: Option<&str>, wire: holler_proto::WireError) {
    // The lockout refusal carries the 1008 close code (the `Unauthenticated`
    // / `-32002` data code): deliver it as a bare close frame, not an error.
    if wire.code == Code::Unauthenticated.jsonrpc() {
        close_code(ctx.tx, 1008, "auth refused: too many failures");
    } else {
        send_error(ctx.tx, id, Code::from_jsonrpc(wire.code).unwrap_or(Code::InvalidRequest), &wire.message);
        close(ctx.tx);
    }
    // A close was queued: the connection task must wait for the writer to flush
    // it (FIN after the frame) before it drops the socket read half.
    ctx.close_pending = true;
    // No `writer.abort()` here: the writer task flushes the close frame it just
    // queued (see `handle_ws_conn`) and then drops the socket, so the peer's
    // client sees a proper close frame rather than a raw TCP reset.
    // Let the caller wait for that flush before it drops the socket read half.
    ctx.finish().await;
}

/// The per-connection context for the first-frame dispatcher (outbound channel,
/// peer, state, registry, lockout, guard). `pub` because `auth_io` borrows it.
pub struct ConnCtx<'a> {
    pub(crate) tx: &'a UnboundedSender<Message>,
    pub(crate) peer: &'a str,
    pub(crate) state: &'a HubState,
    pub(crate) registry: &'a Arc<Registry>,
    pub(crate) lockout: &'a Arc<Lockout>,
    pub(crate) guard: &'a mut PreauthGuard,
    /// Set when a close was queued (refusal/lockout/timeout/bad frame).
    pub(crate) close_pending: bool,
    /// The writer-flush oneshot receiver: signaled once a queued close is on
    /// the wire, so `finish` can await it before dropping the read half.
    pub(crate) flushed: tokio::sync::oneshot::Receiver<()>,
    /// The registry force-close oneshot receiver: signaled on supersede/revoke,
    /// so `finish` can await the flush before dropping the read half.
    pub(crate) force_close: tokio::sync::oneshot::Receiver<()>,
    /// The socket's read half: `finish` polls it briefly after a queued close
    /// is flushed, to drain the peer's close-ack so the socket drops with a
    /// clean FIN instead of a RST (a RST would discard the close frame sitting
    /// in the peer's kernel buffer before its user-space client could read it).
    pub(crate) ws_read: Option<SplitStream<WebSocketStream<TcpStream>>>,
}

impl ConnCtx<'_> {
    /// Await the close flush before this task returns (so the read half is
    /// dropped — and the peer FIN'd — only after a queued close is on the wire).
    /// Waits on `flushed` if `close_pending`, or on `force_close` (then
    /// `flushed`) if the registry force-closed us. A clean peer disconnect
    /// signals neither, so the select returns immediately (the sender drops on
    /// deregister).
    ///
    /// Once a close is on the wire, drain the peer's close-ack (and the
    /// peer's FIN) by polling the read half briefly, so that dropping the
    /// socket sends a clean FIN rather than a RST. A RST (which the kernel
    /// sends when the socket is dropped while the peer's close-ack is still
    /// unread in its receive buffer) would discard the close frame we just
    /// sent, in the peer's kernel buffer, before its user-space client could
    /// read it — so the peer sees a bare reset/EOF instead of the close code.
    /// The drain is bounded (the close-ack, if the peer replies, is already on
    /// the loopback wire by the time our close is flushed), so it cannot stall.
    async fn finish(&mut self) {
        tokio::select! {
            r = std::pin::Pin::new(&mut self.flushed), if self.close_pending => {
                let _ = r;
            }
            r = std::pin::Pin::new(&mut self.force_close) => {
                if r.is_ok() {
                    let _ = std::pin::Pin::new(&mut self.flushed).await;
                }
            }
        }
        // A close is on the wire (or the socket already ended): drain the
        // peer's close-ack so the socket drops with a FIN, not a RST. No-op if
        // the read half was already handed to the post-auth loop (`ws_read` is
        // then `None` and that loop has finished).
        if self.close_pending {
            drain_close_ack(&mut self.ws_read).await;
        }
    }
}

/// After a close frame is on the wire, drain the peer's close-ack (and the
/// peer's FIN) so that dropping the socket's read half sends a clean FIN
/// instead of a RST. The kernel sends a RST when a socket is dropped with
/// unread data still in its receive buffer; that RST discards the close frame
/// we just flushed, in the *peer's* kernel buffer, before the peer's
/// user-space client could read it. We poll the read half until it is quiet
/// (a short deadline bounds the wait — on loopback the peer's close-ack is
/// already on the wire by the time our close flushed) or it ends. Dropping
/// `ws_read` after the drain is the teardown that ends the socket.
async fn drain_close_ack(ws_read: &mut Option<SplitStream<WebSocketStream<TcpStream>>>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(50);
    loop {
        // A quiet beat (or the deadline) means the peer's ack is in.
        if std::time::Instant::now() >= deadline {
            break;
        }
        let res = tokio::select! {
            res = ws_read.as_mut().expect("read half present").next() => res,
            _ = tokio::time::sleep_until(tokio::time::Instant::from(deadline)) => break,
        };
        match res {
            // The peer's close-ack is in (or a stray ping/pong): the handshake
            // is complete, so stop draining.
            Some(Ok(Message::Close(_))) => break,
            // The stream ended (the peer FIN'd, or a read error): done.
            None | Some(Err(_)) => break,
            // In-flight data/frames: keep draining.
            Some(Ok(_)) => {}
        }
    }
}

async fn dispatch_auth(
    env: &Envelope,
    method: &str,
    ctx: &mut ConnCtx<'_>,
    force_close: &Arc<tokio::sync::oneshot::Sender<()>>,
) {
    // `ctx` is passed **by `&mut`** (not destructured — destructuring would
    // shadow the binding and make a later `ctx.` access a second, conflicting
    // mutable borrow). `handle_join`/`handle_auth` read `ctx.tx`/`ctx.state`/
    // `ctx.registry`/`ctx.peer`/`ctx.flushed`/`ctx.lockout`/`ctx.guard`
    // individually through the `&mut ctx` reference; `refuse_auth`/
    // `post_auth_loop` flag `ctx.close_pending` and await `ctx.finish()`.
    // The *admission* lockout check (refuse a tripped peer's new socket) lives
    // in `handle_ws_conn`, before the first frame is read — so a tripped peer is
    // refused with 1008 the instant its socket is accepted, even if it closes
    // immediately or sends no frame (a close/EOF first frame never reaches this
    // function's method dispatch). What remains here is the *strike* handling:
    // the per-method failure records a lockout strike
    // strike; if it trips the lockout the socket closes with 1008, otherwise
    // it receives the codec's own error and closes.
    let res: Result<ConnectionHandle, (Code, String)> = match method {
        "circuit/join" => {
            match handle_join(env, ctx, ctx.tx, ctx.state, ctx.registry, ctx.peer, force_close).await {
                Ok((handle, _client_id)) => Ok(handle),
                Err(e) => Err(e),
            }
        }
        "circuit/authenticate" => {
            handle_auth(env, ctx, ctx.tx, ctx.state, ctx.registry, ctx.peer, force_close).await
        }
        // Any other method on a fresh socket is unauthenticated: reply with the
        // codec's own error (echoing the request's id) and close. This is a
        // codec error, NOT a lockout refusal — so, unlike a tripped lockout
        // (`refuse_auth`'s 1008 branch), it sends the JSON-RPC error frame
        // first (the wire contract: an unknown/early method roundtrips a
        // `-32002 unauthenticated` error before the close).
        _ => {
            ctx.guard.armed = false;
            send_error(ctx.tx, env.id(), Code::Unauthenticated, "not authenticated");
            close(ctx.tx);
            ctx.close_pending = true;
            ctx.finish().await;
            return;
        }
    };
    match res {
        Ok(handle) => {
            ctx.guard.armed = false;
            // The post-auth loop drains inbound frames until the peer closes, the
            // registry force-closes this socket (supersede / revoke — the registry
            // signals `ctx.force_close`, so the connection task flushes the close
            // before dropping the read half), or the stream errors.
            // Hand the read half to the post-auth loop (moved out of `ctx`);
            // `finish` then drains the peer's close-ack before the socket's
            // read half is dropped (see `drain_close_ack`).
            let ws_read = ctx.ws_read.take().expect("read half present");
            post_auth_loop(ws_read, ctx, handle, ctx.peer.to_string()).await;
        }
        Err((code, message)) => {
            ctx.guard.armed = false;
            // A strike that trips the lockout → 1008; otherwise the codec's error.
            let wire = if record_failure(ctx.peer, ctx.lockout) {
                holler_proto::WireError::new(
                    Code::Unauthenticated,
                    "auth refused: too many failures".to_string(),
                    None,
                )
            } else {
                holler_proto::WireError::new(code, message, None)
            };
            refuse_auth(ctx, env.id(), wire).await;
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn handle_ws_conn(
    stream: TcpStream,
    peer: String,
    state: HubState,
    registry: Arc<Registry>,
    hygiene: Arc<Hygiene>,
    lockout: Arc<Lockout>,
) {
    let limits = *hygiene.limits();
    let timeout = limits.pre_auth_timeout();
    // The handshake runs to completion first (no timeout on it — it is a
    // bounded read of the upgrade request, not the open-ended pre-auth window).
    // The pre-auth *timeout* is then raced against the first *frame* read below.
    let ws = match server_handshake(stream, limits.ws_config()).await {
        Some(ws) => ws,
        // Not a WebSocket client (or it left mid-handshake): nothing to close.
        None => return,
    };
    // The handshake succeeded; the socket is now a WebSocket. It still counts
    // as pre-auth until join/authenticate completes. Split the socket: the
    // write half goes to a spawned task that drains the outbound channel into
    // the socket; the reader half stays here.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let (ws_write, ws_read) = ws.split();
    // The writer drains the outbound channel into the socket. Two teardown
    // rules matter:
    //   (1) Once it sends a *close* message it must write that frame **and
    //       flush it before the socket is dropped** — dropping the write half
    //       (or a `writer.abort()` from another task) would RST the TCP
    //       connection *before* the close frame is on the wire, so the peer's
    //       client fails with a protocol error (`ResetWithoutClosingHandshake`)
    //       instead of a clean close.
    //   (2) The *connection* task must not drop its socket read half until the
    //       writer has flushed a close, or the socket's read side FINs the peer
    //       *before* the close frame leaves the socket and the peer observes a
    //       bare EOF instead of the close code.
    // The writer therefore signals `flushed` (to the connection task, via the
    // `ConnCtx`'s `flushed` receiver) the moment a close is on the wire (or the
    // socket died on its own); the connection task awaits that before dropping
    // its read half (see `ConnCtx::finish`). We never `abort()` the writer from
    // the connection task: the writer terminates itself, flushing any queued
    // close first.
    let (flushed_tx, flushed_rx) = tokio::sync::oneshot::channel::<()>();
    let _writer = tokio::spawn(async move {
        let mut rx = rx;
        let mut ws_write = ws_write;
        // Whether we sent a close through the channel (a `Close`, or a raw
        // `Frame` close — `close_code`/`close` queue these). When true, we
        // signal `flushed` after flushing that close (the connection task is
        // waiting on it before it drops the read half). When false (a clean
        // peer disconnect / a normal channel drain), the connection task did
        // *not* await `flushed`, so we must NOT signal it — doing so would panic
        // ("called after complete") once the receiver was already dropped by a
        // clean teardown.
        let mut close_sent = false;
        loop {
            let Some(msg) = rx.recv().await else {
                break; // all senders dropped (connection task ended).
            };
            // Decide *before* moving the message in: a close is terminal and
            // must be flushed to the wire before the socket is dropped, so the
            // peer's client sees a real close frame.
            let is_close = matches!(&msg, Message::Close(_) | Message::Frame(_));
            if is_close {
                close_sent = true;
            }
            let s = ws_write.send(msg).await;
            if s.is_err() {
                break; // socket write failed (peer left).
            }
            if is_close {
                break;
            }
        }
        // Flush the last frame to the wire (a close, or the in-flight text on a
        // clean teardown). Signal `flushed` only if a close was queued: the
        // connection task awaits it *only* when `close_pending` (or a registry
        // force-close) is set; on a clean peer disconnect it never does, so a
        // spurious send would hit a dropped receiver (and, on the rare path
        // where the receiver is still alive, panic "called after complete").
        let _ = ws_write.flush().await;
        if close_sent {
            let _ = flushed_tx.send(());
        }
    });
    let mut conn_guard = PreauthGuard {
        hygiene,
        armed: true,
    };
    // The connection's `force_close` oneshot: the *sender* is held as an `Arc`
    // (a tokio oneshot `Sender` is not `Clone`; only the `Arc` wrapper is) and
    // a clone of that `Arc` is moved into the registry entry (via `ConnInfo`,
    // in `handle_join`/`handle_auth`). The registry can then `map.remove` the
    // entry on supersede/revoke and call `send()` on the dereferenced sender
    // (the `Arc` was moved out of the entry, so the sender is not behind a
    // shared reference). The *receiver* lives in `ctx.force_close`;
    // `ConnCtx::finish` awaits it (in a `select!` alongside `flushed`). A
    // clean deregister drops the registry entry (and its `Arc` clone), which
    // resolves the receiver as `Err` if the connection task's `Arc` is also
    // dropped — the "no force-close" case — so the teardown never stalls.
    let (force_close_tx, force_close_rx) = tokio::sync::oneshot::channel::<()>();
    let force_close = Arc::new(force_close_tx);
    let mut ctx = ConnCtx {
        tx: &tx,
        peer: &peer,
        state: &state,
        registry: &registry,
        lockout: &lockout,
        guard: &mut conn_guard,
        close_pending: false,
        flushed: flushed_rx,
        force_close: force_close_rx,
        ws_read: Some(ws_read),
    };
    // The failed-auth lockout is checked *before* any frame is read, so a
    // tripped peer is refused with 1008 the instant its socket is accepted —
    // regardless of what it would have sent (or whether it sent anything).
    // This runs ahead of the first-frame read (and the pre-auth timeout) so that
    // even a locked-out peer that closes the socket immediately, or never sends
    // a frame, is still answered with the 1008 refusal rather than a bare drop.
    // The refusal is terminal: it cannot authenticate, so disarm the guard (no
    // lockout strike), flag the queued close, and wait for the writer to flush
    // it before the read half is dropped.
    if let Some(peer_ip) = peer_ip(&peer) {
        if lockout.is_locked_out(&peer_ip) {
            warn(
                Component::Token,
                "auth_lockout",
                format!("refusing {peer}: lockout active"),
            );
            close_code(&tx, 1008, "auth refused: too many failures");
            ctx.close_pending = true;
            ctx.guard.armed = false;
            ctx.finish().await;
            return;
        }
    }
    // Race the first-frame read against the pre-auth timeout. Both arms borrow
    // `ctx.ws_read` (it is `Unpin`), so neither moves it. On timeout we close
    // through the outbound channel (a 1001 going-away frame the writer
    // delivers), then wait for the writer to flush it before the read half is
    // dropped (FIN after the frame).
    let first = tokio::select! {
        biased;
        r = read_first_frame(ctx.ws_read.as_mut().unwrap(), &tx) => r,
        _ = tokio::time::sleep(timeout) => {
            warn(
                Component::Wire,
                "preauth_timeout",
                format!("closing {peer}: no join/auth within {timeout:?}"),
            );
            close_code(&tx, 1001, "auth timeout");
            ctx.close_pending = true;
            ctx.guard.armed = false;
            ctx.finish().await;
            return;
        }
    };
    // A failed first frame is not an authentication attempt: no lockout strike.
    ctx.guard.armed = false;
    // `read_first_frame` already sent the close frame(s) for a bad first frame
    // (or the peer closed). It did not flag `close_pending` (it is a helper), so
    // flag it here and wait for the writer to flush before dropping the read half.
    let Some(first) = first else {
        ctx.close_pending = true;
        ctx.finish().await;
        return;
    };
    // Run the text frame through the codec.
    let env = match decode(&first) {
        Ok(e) => e,
        Err(e) => {
            // A framing failure (parse / batch / shape / version / unknown
            // method / bad id): reply with the codec's own code, then close.
            send_error(&tx, None, e.code(), &e.to_string());
            ctx.close_pending = true;
            ctx.finish().await;
            return;
        }
    };
    // Decide by method (the fail-closed rule).
    let method = match env.method() {
        Some(m) => m,
        // A response or error as the *first* frame is not a join/auth: refuse.
        None => {
            send_error(&tx, env.id(), Code::Unauthenticated, "not authenticated");
            close(&tx);
            ctx.close_pending = true;
            ctx.finish().await;
            return;
        }
    };
    // Dispatch the (now pre-auth-cleared) first frame: on success it runs the
    // post-auth loop (ending when the peer leaves or is revoked/superseded), and
    // on refusal it queues a close and waits for the writer to flush it. The
    // `force_close` sender is passed through so the join/auth handlers can store
    // it on this connection's registry entry (the registry signals it on a
    // supersede/revoke). The read half lives in `ctx.ws_read`; the post-auth
    // loop takes it out of `ctx` when it starts, and `finish` (below) drains
    // the peer's close-ack through it if a close was queued on a pre-auth path.
    dispatch_auth(&env, method, &mut ctx, &force_close).await;
    // `dispatch_auth` may have queued a close (a refusal / lockout, or a
    // revoke/supersede in the post-auth loop). If so, wait for the writer to
    // flush it before we drop the read half below (FIN after the frame).
    ctx.finish().await;
}

/// Read the first substantive inbound frame (skipping pings). `None` on
/// close/EOF/binary (does not count as an auth failure).
async fn read_first_frame<S: AsyncRead + AsyncWrite + Unpin>(
    ws_read: &mut SplitStream<WebSocketStream<S>>,
    tx: &UnboundedSender<Message>,
) -> Option<String> {
    loop {
        match ws_read.next().await {
            None => return None, // peer closed before sending anything.
            Some(Ok(Message::Text(t))) => return Some(t.to_string()),
            Some(Ok(Message::Binary(_))) => {
                send_error(tx, None, Code::InvalidRequest, "a binary frame is not a v2 message");
                close(tx);
                return None;
            }
            Some(Ok(Message::Ping(_)))
            | Some(Ok(Message::Pong(_)))
            | Some(Ok(Message::Frame(_))) => continue,
            Some(Ok(Message::Close(_))) | Some(Err(_)) => return None,
        }
    }
}

/// The post-auth read loop: drain inbound frames until the peer closes, the
/// registry force-closes (1000/1008), or the stream errors.
async fn post_auth_loop<S: AsyncRead + AsyncWrite + Unpin>(
    mut ws_read: SplitStream<WebSocketStream<S>>,
    ctx: &mut ConnCtx<'_>,
    handle: ConnectionHandle,
    _peer: String,
) {
    // Inbound frames are simply drained (any frame keeps the socket alive;
    // per-connection liveness tracking is a later story). The loop ends when
    // the peer closes, the registry force-closes the socket, or the stream
    // errors. The handle's Drop deregisters the connection on any exit.
    //
    // The loop takes `&mut ConnCtx` (not a `&Arc<Registry>`) so that the
    // connection task's `finish()` (run by the caller after this loop returns)
    // can await the registry's `force_close` signal: when the registry
    // force-closes *this* socket (supersede / revoke) it signals `ctx`'s
    // `force_close` receiver, and `finish` then waits for that close to reach
    // the wire before the read half is dropped. A *peer*-initiated close (or a
    // read error) leaves `force_close` un-signaled and `close_pending` false, so
    // `finish` returns immediately (the sender drops on deregister): there is
    // nothing to flush, and the peer never replies to a close, so waiting would
    // stall the teardown.
    while let Some(msg) = ws_read.next().await {
        match msg {
            // A text frame after auth is a request (or a stray response); drain.
            Ok(Message::Text(_))
            | Ok(Message::Ping(_))
            | Ok(Message::Pong(_))
            | Ok(Message::Frame(_)) => {}
            // A binary frame after auth is not a v2 message; end the socket.
            Ok(Message::Binary(_)) => break,
            // Peer closed, the registry closed us (supersede / revoke), or the
            // stream errored. If the registry closed *us*, it signaled `ctx`'s
            // `force_close` receiver (so `finish` flushes our close before the
            // read half is dropped); a peer-initiated close leaves it un-signaled
            // (and the deregister drops its sender), so `finish` returns at once.
            // The handle's Drop deregisters us.
            //
            // A protocol violation (oversized frame) surfaces here as
            // `Err(tungstenite::Error::Capacity(_))`: the socket was already
            // split into read/write halves before this loop runs, so
            // tungstenite has no write half of its own to send its usual
            // auto-close on — the read side can only ever hand back an
            // error. We send the 1009 ourselves through the writer channel,
            // or the peer never sees a close frame at all (it just hangs
            // until the TCP connection eventually resets). Flag
            // `close_pending` so `finish` waits for the writer to flush the
            // 1009 before the read half is dropped (a RST would discard the
            // close frame in the peer's kernel buffer).
            Err(tokio_tungstenite::tungstenite::Error::Capacity(_)) => {
                ctx.close_pending = true;
                close_code(
                    ctx.tx,
                    tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Size.into(),
                    "message too big",
                );
                break;
            }
            Ok(Message::Close(_)) | Err(_) => break,
        }
    }
    drop(handle);
}

// The two pre-auth handlers (`circuit/join` → `handle_join`,
// `circuit/authenticate` → `handle_auth`) live in `auth_io.rs` so this file
// stays under the 900-line lint gate.
pub use crate::auth_io::{handle_auth, handle_join};

// The per-connection wire I/O helpers (error/close/response frame senders,
// the failed-auth lockout strike, and the peer-IP / clock helpers) live in
// `wire_io.rs` so this file stays under the 900-line lint gate.
pub use crate::wire_io::{
    close, close_code, close_raw_stream, now_ms, peer_ip, record_failure, send_error, send_result,
};

// The control-socket **server** side (the `handle_control_conn` loop, the
// `control/status` dispatch, and the status-doc builder) lives in
// `control_io.rs` so this file stays under the 900-line lint gate.
pub use crate::control_io::handle_control_conn;
