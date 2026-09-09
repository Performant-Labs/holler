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

use tokio::net::{TcpListener, TcpStream, UnixListener};
use tokio::signal::unix::{signal, SignalKind};

use crate::state::{
    advertise_path, control_sock_path, ensure_dirs, resolve_state_dir, serve_lock_path, HubState,
};
use crate::{connection::Hygiene, lockout::Lockout};
use crate::registry::Registry;

use holler_proto::log::{Component, Direction, Event, Severity};

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

// The per-connection plumbing (pre-auth guard, ConnCtx, first-frame
// dispatcher, post-auth loop, and the end-to-end `handle_ws_conn`) lives in
// `conn.rs` so this file stays under the 900-line lint gate.
pub use crate::conn::{ConnCtx, PreauthGuard, handle_ws_conn};

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
