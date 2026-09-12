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
use tokio::net::{TcpListener, TcpSocket, TcpStream, UnixListener};
use tokio::signal::unix::{signal, SignalKind};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

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

    // Issue #322: generate (or load) the hub's long-lived X25519 identity
    // keypair before anything binds — `hub token mint` also calls this
    // (whichever runs first creates the file), so a hub that has never
    // served yet but has already minted a token still answers with the same
    // key a body pinned earlier.
    if let Err(e) = crate::identity::ensure(&state) {
        warn(Component::Control, "hub_identity_failed", format!("cannot resolve hub identity: {e}"));
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

// Positioned write at offset 0 (the file was just truncated, so a plain
// `Write` would also land at 0 — `write_at`/`seek_write` are used anyway so
// this never depends on the file's current cursor position). The two traits
// are unix's `std::os::unix::fs::FileExt::write_at` and windows'
// `std::os::windows::fs::FileExt::seek_write` — same effect, different
// names, so the OS is picked by `cfg` rather than importing an absent trait
// unconditionally (the compile break this mirrors: holler-client#60).
#[cfg(unix)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_at(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

#[cfg(windows)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    file.seek_write(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

/// Bind a single WS listen address with `SO_REUSEADDR` set (issue #259): a
/// hub restarted immediately on the exact same fixed port — the previous
/// process's own connections are killed out from under it (`kill_tree`'s
/// `SIGKILL`), not closed gracefully — otherwise races the OS's per-4-tuple
/// `TIME_WAIT` teardown for those connections' local port and can lose with
/// `EADDRINUSE`, even though the *listening* socket itself was already fully
/// reclaimed (`kill_tree` blocks on `wait()`, so the old process is reaped
/// before the new one ever spawns). `tokio::net::TcpListener::bind` does not
/// set this itself, so it is set explicitly via [`TcpSocket`] rather than
/// relying on the underlying mio/socket2 default, which is not part of
/// tokio's public contract.
///
/// A short bind-retry with backoff (5 attempts, 20ms/40ms/80ms/160ms between
/// them, ~300ms worst case) is layered on top: `SO_REUSEADDR` covers the
/// `TIME_WAIT` case but not a residual window where the kernel is still
/// tearing down the killed process's socket state after `wait()` returns —
/// belt-and-suspenders against exactly the "hub did not report listening
/// within 10s" flake #259 measured at ~10% with a single unretried bind.
async fn bind_one_ws_listener(addr: &SocketAddr) -> std::io::Result<TcpListener> {
    const MAX_ATTEMPTS: u32 = 5;
    let mut backoff = std::time::Duration::from_millis(20);
    // Seeded with a real (never-surfaced-unless-every-attempt-fails) error
    // rather than `Option<io::Error>` + a loop-invariant `.expect()`: the loop
    // below always runs at least once and overwrites this on every failing
    // attempt, but clippy's `expect_used` gate (workspace-wide, `-D warnings`)
    // does not know that invariant, and an `Option` here would need one to
    // unwrap it at the end.
    let mut last_err = std::io::Error::other("bind_one_ws_listener: unreachable placeholder");
    for attempt in 1..=MAX_ATTEMPTS {
        let socket = if addr.is_ipv4() { TcpSocket::new_v4()? } else { TcpSocket::new_v6()? };
        socket.set_reuseaddr(true)?;
        match socket.bind(*addr) {
            Ok(()) => match socket.listen(1024) {
                Ok(listener) => return Ok(listener),
                Err(e) => last_err = e,
            },
            Err(e) => last_err = e,
        }
        if attempt < MAX_ATTEMPTS {
            tokio::time::sleep(backoff).await;
            backoff *= 2;
        }
    }
    Err(last_err)
}

/// Bind every WS listen address in `addrs`, returning the bound
/// [`TcpListener`]s and their actual `addr:port` strings. Split out of
/// [`serve_forever`] (issue #184) purely to keep that fn under clippy's
/// `too_many_lines` gate once the connection-hygiene setup grew it — no
/// behavior change.
async fn bind_ws_listeners(addrs: &[SocketAddr]) -> Result<(Vec<TcpListener>, Vec<String>), i32> {
    let mut ws_listeners = Vec::new();
    let mut bound_addrs = Vec::new();
    for addr in addrs {
        match bind_one_ws_listener(addr).await {
            Ok(l) => match l.local_addr() {
                Ok(actual) => {
                    bound_addrs.push(actual.to_string());
                    ws_listeners.push(l);
                }
                Err(e) => {
                    warn(Component::Wire, "bind_addr_report_failed", format!("cannot report the bound address: {e}"));
                    return Err(1);
                }
            },
            Err(e) => {
                warn(Component::Wire, "bind_failed", format!("failed to bind {addr}: {e}"));
                return Err(1);
            }
        }
    }
    Ok((ws_listeners, bound_addrs))
}

/// The per-process hub-wide handles the accept loop shares across every
/// connection: the live registry, connection-hygiene limits/lockout/
/// pre-auth semaphore (issue #184), and the roster. Split out of
/// [`serve_forever`] purely to keep that fn under clippy's `too_many_lines`
/// gate — no behavior change.
struct SharedState {
    registry: crate::live::Registry,
    hygiene: crate::hygiene::HygieneLimits,
    lockout: std::sync::Arc<crate::lockout::Lockout>,
    preauth_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    roster: std::sync::Arc<crate::roster::Roster>,
}

fn build_shared_state() -> SharedState {
    // The live-circuit registry (issue #182): one per hub process, shared by
    // every accepted WS connection (recording/removing itself as it
    // authenticates/disconnects) and every control-socket connection (`hub
    // token ping` reaches a body's socket through it).
    let registry = crate::live::Registry::new();
    // Issue #184's connection hygiene: resolved once per process (the same
    // "fixed for the hub's whole life" discipline the roster's `Config`
    // already uses), then shared by every accepted socket.
    let hygiene = crate::hygiene::HygieneLimits::resolve();
    let lockout = crate::lockout::Lockout::new();
    let preauth_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(hygiene.max_preauth_connections));
    // The roster (issue #186): one per hub process, shared by every accepted
    // WS connection (which advertises/touches/clears it as presence and frames
    // arrive and circuits open/close), the control socket (which reads it for
    // `holler roster`), and the periodic sweep task (issue #255). The system
    // clock drives the 45/180/360 s sweep in production (the unit tests
    // inject a manual clock).
    let roster = std::sync::Arc::new(crate::roster::Roster::with_system_clock(&crate::roster::Config::from_env()));
    SharedState { registry, hygiene, lockout, preauth_semaphore, roster }
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
    let (ws_listeners, bound_addrs) = match bind_ws_listeners(&addrs).await {
        Ok(pair) => pair,
        Err(code) => return code,
    };

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
    let SharedState { registry, hygiene, lockout, preauth_semaphore, roster } = build_shared_state();

    // The roster TTL sweep task (issue #255): `Roster::sweep()` was
    // previously only exercised by the unit tests against an injected clock
    // — nothing in the running hub ever called it, so roster rows never aged
    // `connected → reconnecting → gone` in production. This spawns a
    // dedicated task, sibling to the accept loop, that sleeps
    // `roster.sweep_ms()` then calls `roster.sweep()`, wired into the same
    // signal-triggered stop-channel shutdown as the accept loop (a second
    // `oneshot` so the one signal arm can trip both).
    let (sweep_stop_tx, sweep_stop_rx) = tokio::sync::oneshot::channel::<()>();
    let sweep_handle = {
        let roster = roster.clone();
        tokio::spawn(async move {
            sweep_loop(roster, sweep_stop_rx).await;
        })
    };
    let accept_handle = tokio::spawn(async move {
        accept_loop(uds, ws_listeners, state, registry, roster, hygiene, lockout, preauth_semaphore, stop_rx).await;
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

    // Park on a signal; when one arrives, trip both stop channels so the
    // accept loop's `select!` and the sweep loop's `select!` both unblock
    // and wind down.
    let mut stop_tx = Some(stop_tx);
    let mut sweep_stop_tx = Some(sweep_stop_tx);
    tokio::select! {
        _ = sig_int.recv() => {
            let _ = stop_tx.take().map(|t| t.send(()));
            let _ = sweep_stop_tx.take().map(|t| t.send(()));
        }
        _ = sig_term.recv() => {
            let _ = stop_tx.take().map(|t| t.send(()));
            let _ = sweep_stop_tx.take().map(|t| t.send(()));
        }
    }
    // Make sure both channels are tripped even if both signal receivers were
    // consumed (defensive; one of the arms above already ran).
    let _ = stop_tx.take().map(|t| t.send(()));
    let _ = sweep_stop_tx.take().map(|t| t.send(()));
    // Wait for both tasks to wind down, then return 0 for a clean shutdown.
    let _ = accept_handle.await;
    let _ = sweep_handle.await;
    0
}

/// The roster TTL sweep task (issue #255): sleep [`crate::roster::Roster::sweep_ms`]
/// then call [`crate::roster::Roster::sweep`], repeating until the stop
/// channel is tripped (a SIGINT/SIGTERM arrived) — the same
/// select-on-a-oneshot shutdown shape as [`accept_loop`], so the sweep task
/// winds down alongside the accept loop rather than lingering past it.
async fn sweep_loop(
    roster: std::sync::Arc<crate::roster::Roster>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            // A signal arrived: wind down (stop sweeping) and return.
            _ = &mut stop_rx => return,
            _ = tokio::time::sleep(std::time::Duration::from_millis(roster.sweep_ms())) => {
                roster.sweep();
            }
        }
    }
}

/// Poll the WS and control listeners, spawning a task per connection, until
/// the stop channel is tripped (a SIGINT/SIGTERM arrived) or a listener fails.
#[allow(clippy::too_many_arguments)] // #184: 5 shared hub-wide handles, threaded straight to the connection tasks
async fn accept_loop(
    uds: UnixListener,
    ws_listeners: Vec<TcpListener>,
    state: HubState,
    registry: crate::live::Registry,
    roster: std::sync::Arc<crate::roster::Roster>,
    hygiene: crate::hygiene::HygieneLimits,
    lockout: std::sync::Arc<crate::lockout::Lockout>,
    preauth_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
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
                let (stream, addr) = match res {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
                tokio::spawn(handle_ws_conn(
                    stream,
                    addr,
                    state.clone(),
                    registry.clone(),
                    roster.clone(),
                    hygiene,
                    lockout.clone(),
                    preauth_semaphore.clone(),
                ));
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

/// Handle one WebSocket (circuit) connection. The first frame decides the fate
/// of the socket (the fail-closed accept loop, story #143):
///
/// - **not a v2 message** (garbage / batch / shape error) → the matching
///   `-32700`/`-32600` error, then close.
/// - **`circuit/join`** → redeemed via [`crate::join::redeem_join`], which
///   replies with `{client_id}` (issue #323: no credential; or a
///   `join_failed` error) and closes the one-shot socket.
/// - **anything else** (a request or notification that is not join) →
///   `-32002 unauthenticated`, then close.
///
/// The socket is **always** closed after the first frame on this story: a join
/// is a one-shot bootstrap (docs §3), so there is no talk yet.
///
/// The server-side WebSocket opening handshake itself lives in
/// [`crate::ws_handshake`] (issue #184 — split out of this file once the
/// connection-hygiene accept-path additions pushed it past the 900-line
/// build guard).
use crate::ws_handshake::server_handshake;

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
            Some(Ok(Message::Close(_))) => {
                close(sink).await;
                return None;
            }
            // Issue #184: a frame/message over the hygiene cap surfaces here
            // as tungstenite's own `Capacity` error (it does not itself send
            // a close frame on this path — see `WebSocketConfig::max_frame_size`'s
            // own doc) — close explicitly with **1009** (message too big).
            // Any other stream error is an ordinary abrupt teardown.
            Some(Err(WsError::Capacity(_))) => {
                close_with_code(sink, 1009, "message too big").await;
                return None;
            }
            Some(Err(_)) => {
                close(sink).await;
                return None;
            }
        }
    }
}

/// The three hygiene gates a fresh socket passes through, in order, before
/// its first frame is even decoded (issue #184): the failed-auth lockout,
/// the unauthenticated-connection-cap semaphore, then the pre-auth-timeout
/// read of the first frame. `Err(())` means the caller (`handle_ws_conn`)
/// should stop — this fn has already sent (and flushed) whatever close the
/// refusal called for. Split out purely to keep `handle_ws_conn`'s own
/// cognitive complexity under the workspace threshold.
async fn admit_preauth(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    stream: &mut (impl Stream<Item = Result<Message, WsError>> + Unpin),
    addr: SocketAddr,
    peer: &str,
    hygiene: crate::hygiene::HygieneLimits,
    lockout: &std::sync::Arc<crate::lockout::Lockout>,
    preauth_semaphore: &std::sync::Arc<tokio::sync::Semaphore>,
) -> Result<(String, tokio::sync::OwnedSemaphorePermit), ()> {
    // Issue #184's failed-auth lockout: refused *before reading a frame* —
    // the WS upgrade itself is not a "frame" in this sense (a close code is
    // only expressible once the socket has upgraded, so the handshake must
    // still have completed by the time this runs), but nothing past that
    // point is read from a locked-out peer.
    if lockout.is_locked_out(&addr.ip()) {
        warn(Component::Wire, "lockout_refused", format!("peer={peer} refused: locked out"));
        close_with_code(sink, 1008, "auth refused: too many failures").await;
        return Err(());
    }

    // Issue #184's unauthenticated-connection cap: a semaphore around the
    // whole pre-auth phase (this point through the end of the auth
    // handshake — `circuit::handle_authenticated` releases it the moment
    // `hello_exchange` succeeds, or it is dropped here on any refusal/error
    // path below). Over the cap: refuse at once, before reading a frame.
    let preauth_permit = match preauth_semaphore.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            close_with_code(sink, 1013, "try again later: too many unauthenticated connections").await;
            return Err(());
        }
    };

    // Issue #184's pre-auth timeout: a socket that has sent nothing at all
    // within this budget is closed. Read the first frame (skipping
    // ping/pong; a close or EOF ends the socket) under that bound.
    match tokio::time::timeout(hygiene.pre_auth_timeout(), read_first_text_frame(sink, stream)).await {
        Ok(Some(t)) => Ok((t, preauth_permit)),
        Ok(None) => Err(()),
        Err(_) => {
            close(sink).await;
            Err(())
        }
    }
}

#[allow(clippy::too_many_arguments)] // #184: 4 shared hub-wide handles, unavoidable at the accept boundary
async fn handle_ws_conn(
    stream: TcpStream,
    addr: SocketAddr,
    state: HubState,
    registry: crate::live::Registry,
    roster: std::sync::Arc<crate::roster::Roster>,
    hygiene: crate::hygiene::HygieneLimits,
    lockout: std::sync::Arc<crate::lockout::Lockout>,
    preauth_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
) {
    let peer = addr.to_string();
    let ws = match server_handshake(stream, hygiene.ws_config()).await {
        Some(ws) => ws,
        None => return, // not a WebSocket client (or it left mid-handshake).
    };
    let (mut sink, mut stream) = ws.split();

    let (first, preauth_permit) =
        match admit_preauth(&mut sink, &mut stream, addr, &peer, hygiene, &lockout, &preauth_semaphore).await {
            Ok(pair) => pair,
            Err(()) => return,
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
        // secret and registers its Ed25519 public key; the hub redeems it for
        // a `client_id` (issue #323: no credential), replies, and closes the
        // socket. (The normal `circuit/authenticate` → `circuit/prove` path —
        // re-authenticating an existing body — is dispatched below.)
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
        // A returning body re-authenticating by proving possession of its
        // registered private key (issue #323's `circuit/authenticate` →
        // `circuit/prove` challenge-response). On success this holds the
        // socket for the whole live session (hello exchange, presence, ping);
        // it only returns once the circuit ends. Split into its own fn to
        // keep this dispatch's cognitive complexity under the workspace
        // threshold.
        Some("circuit/authenticate") => {
            dispatch_authenticate(&mut sink, &mut stream, &env, &state, &registry, &roster, &peer, &lockout, preauth_permit).await;
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
#[allow(clippy::too_many_arguments)] // #184: threads the hygiene/lockout/permit straight from `handle_ws_conn`
async fn dispatch_authenticate(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    stream: &mut (impl Stream<Item = Result<Message, WsError>> + Unpin),
    env: &Envelope,
    state: &HubState,
    registry: &crate::live::Registry,
    roster: &std::sync::Arc<crate::roster::Roster>,
    peer: &str,
    lockout: &std::sync::Arc<crate::lockout::Lockout>,
    preauth_permit: tokio::sync::OwnedSemaphorePermit,
) {
    let params = match holler_proto::typed_params::<holler_proto::Authenticate>(env) {
        Ok(p) => p,
        Err(e) => {
            send_error(sink, env.id(), Code::InvalidParams, &e.message).await;
            close(sink).await;
            return;
        }
    };
    let deps = crate::circuit::AuthDeps { registry, roster, peer, lockout };
    crate::circuit::handle_authenticated(sink, stream, env.id(), params, state, deps, Some(preauth_permit)).await;
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

/// Send a WS close frame carrying an explicit close `code`/`reason` and flush
/// it (issue #184: an oversized frame closes **1009**, a pre-auth-cap refusal
/// **1013**, a lockout refusal **1008**, supersede **1000**, revoke **1008**
/// — the plain [`close`] above only ever sends a codeless close, which is
/// right for every *other* teardown path but not these operator/registry/
/// hygiene-initiated ones).
pub(crate) async fn close_with_code(sink: &mut (impl Sink<Message, Error = WsError> + Unpin), code: u16, reason: &'static str) {
    use tokio_tungstenite::tungstenite::protocol::frame::{coding::CloseCode, CloseFrame};
    let frame = Message::Close(Some(CloseFrame { code: CloseCode::from(code), reason: reason.into() }));
    let _ = sink.send(frame).await;
    let _ = sink.flush().await;
}

// The control-socket **server** side (its own dispatch and the `hub token
// ping` live probe) lives in [`crate::control_server`] — moved out of this
// file (issue #182) to keep `serve.rs` under the file-size guard as the
// authenticated live-session path (`circuit/authenticate` → `circuit/hello` →
// presence) landed here.
