//! Per-connection plumbing (split from `serve.rs` so that file stays under
//! the 900-line lint gate): the pre-auth guard, the per-connection context,
//! the first-frame dispatcher, the post-auth drain loop, and the
//! end-to-end `handle_ws_conn` entry point.

use std::sync::Arc;

use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use holler_proto::{decode, Envelope};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::UnboundedSender;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::connection::Hygiene;
use crate::lockout::Lockout;
use crate::registry::{ConnectionHandle, Registry};
use crate::state::HubState;
use crate::wire_io::{close, close_code, peer_ip, record_failure, send_error};

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

/// Close a pre-auth socket after a refused authentication. A failed
/// `circuit/authenticate` is refused with a **bare 1008 close** — no
/// JSON-RPC error frame. (A bad credential is an "unknown credential"
/// refusal, not a codec error, so the wire contract for an auth failure is the
/// close alone; the peer's own client does not surface an error frame it was
/// not expecting. Any other pre-auth refusal — a framing/codec failure —
/// roundtrips the codec's error frame *and* a close.) The writer task delivers
/// the queued frames.
async fn refuse_auth(ctx: &mut ConnCtx<'_>, id: Option<&str>, wire: holler_proto::WireError) {
    if wire.code == Code::Unauthenticated.jsonrpc() {
        close_code(ctx.tx, 1008, "auth refused: too many failures");
    } else {
        send_error(
            ctx.tx,
            id,
            Code::from_jsonrpc(wire.code).unwrap_or(Code::InvalidRequest),
            &wire.message,
        );
        close(ctx.tx);
    }
    ctx.close_pending = true;
    ctx.finish().await;
}

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
    /// so `finish` can await the flush before dropping the read half AND
    /// `post_auth_loop` can race it against the read half. Set by `handle_auth`
    /// before the post-auth loop starts (from the receiver returned by
    /// `register`).
    pub(crate) force_close: tokio::sync::oneshot::Receiver<()>,
    /// The socket's read half: `finish` polls it briefly after a queued close
    /// is flushed, to drain the peer's close-ack so the socket drops with a
    /// clean FIN instead of a RST.
    pub(crate) ws_read: Option<SplitStream<WebSocketStream<TcpStream>>>,
}

impl ConnCtx<'_> {
    /// Await the close flush before this task returns (so the read half is
    /// dropped — and the peer FIN'd — only after a queued close is on the wire).
    async fn finish(&mut self) {
        // `biased` is critical: it polls `flushed` first. On a supersede /
        // revoke both oneshots are resolved, so a non-biased select would
        // poll *both* receivers and then the `force_close` arm would poll
        // `self.flushed` a second time — a double-poll of an already-
        // resolved oneshot, which panics ("called after complete"). With
        // `biased`, a resolved `flushed` always wins, so the `force_close`
        // arm only runs when `flushed` is still pending and safe to await.
        //
        // The `if r.is_ok()` guard distinguishes a *real* force-close signal
        // (supersede/revoke: the registry's sender is still alive and resolved
        // the oneshot with `Ok`) from a *clean disconnect* (the sender was
        // dropped on deregister, resolving the oneshot with `Err`). Only a
        // real signal means a close was queued that must be flushed, so we
        // await `flushed` only in that case. A clean disconnect resolves
        // `force_close` by drop and must not stall on a `flushed` that was
        // never signaled.
        tokio::select! {
            biased;
            r = std::pin::Pin::new(&mut self.flushed) => {
                let _ = r;
            }
            r = std::pin::Pin::new(&mut self.force_close) => {
                if r.is_ok() {
                    let _ = std::pin::Pin::new(&mut self.flushed).await;
                }
            }
        }
        if self.close_pending {
            drain_close_ack(&mut self.ws_read).await;
        }
    }
}

/// After a close frame is on the wire, drain the peer's close-ack (and the
/// peer's FIN) so that dropping the socket's read half sends a clean FIN
/// instead of a RST.
// The read half is `Some` here by construction (set in `handle_ws_conn` and
// only taken out by `dispatch_auth` on the success path, which then hands it
// to the post-auth loop and never calls `drain_close_ack` again).
#[allow(clippy::expect_used)] // #184
async fn drain_close_ack(ws_read: &mut Option<SplitStream<WebSocketStream<TcpStream>>>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(50);
    loop {
        if std::time::Instant::now() >= deadline {
            break;
        }
        let res = tokio::select! {
            res = ws_read.as_mut().expect("read half present").next() => res,
            _ = tokio::time::sleep_until(tokio::time::Instant::from(deadline)) => break,
        };
        match res {
            Some(Ok(Message::Close(_))) => break,
            None | Some(Err(_)) => break,
            Some(Ok(_)) => {}
        }
    }
}

// `ctx.ws_read` is `Some` on the `Ok` arm (the reader is still held; the
// post-auth loop takes it out), so the `.expect` is unreachable.
#[allow(clippy::expect_used)] // #184
async fn dispatch_auth(
    env: &Envelope,
    method: &str,
    ctx: &mut ConnCtx<'_>,
) -> Option<tokio::sync::oneshot::Receiver<()>> {
    if method == "circuit/join" {
        crate::auth_io::handle_join(env, ctx, ctx.tx, ctx.state, ctx.registry, ctx.peer).await;
        ctx.guard.armed = false;
        return None;
    }
    if method != "circuit/authenticate" {
        ctx.guard.armed = false;
        send_error(ctx.tx, env.id(), Code::Unauthenticated, "not authenticated");
        close(ctx.tx);
        ctx.close_pending = true;
        ctx.finish().await;
        return None;
    }
    match crate::auth_io::handle_auth(env, ctx, ctx.tx, ctx.state, ctx.registry, ctx.peer).await {
        Ok((handle, dropped_rx, fc_rx)) => {
            ctx.guard.armed = false;
            if let Some(ip) = peer_ip(ctx.peer) {
                ctx.lockout.reset(&ip);
            }
            let ws_read = ctx.ws_read.take().expect("read half present");
            ctx.force_close = fc_rx;
            let hostname = ctx
                .registry
                .get(handle.token_id())
                .map(|c| c.hostname)
                .unwrap_or_default();
            post_auth_loop(ws_read, ctx, handle, &hostname).await;
            Some(dropped_rx)
        }
        Err((code, message)) => {
            ctx.guard.armed = false;
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
            None
        }
    }
}

#[allow(clippy::too_many_lines, clippy::expect_used, clippy::unwrap_used)] // #184
pub async fn handle_ws_conn(
    stream: TcpStream,
    peer: String,
    state: HubState,
    registry: Arc<Registry>,
    hygiene: Arc<Hygiene>,
    lockout: Arc<Lockout>,
) {
    let limits = *hygiene.limits();
    let timeout = limits.pre_auth_timeout();
    let ws = match crate::handshake::server_handshake(stream, limits.ws_config()).await {
        Some(ws) => ws,
        None => return,
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let (ws_write, ws_read) = ws.split();
    let (flushed_tx, flushed_rx) = tokio::sync::oneshot::channel::<()>();
    let _writer = tokio::spawn(async move {
        let mut rx = rx;
        let mut ws_write = ws_write;
        let mut close_sent = false;
        loop {
            let Some(msg) = rx.recv().await else {
                break;
            };
            let is_close = matches!(&msg, Message::Close(_) | Message::Frame(_));
            if is_close {
                close_sent = true;
            }
            let s = ws_write.send(msg).await;
            if s.is_err() {
                break;
            }
            if is_close {
                break;
            }
        }
        let _ = ws_write.flush().await;
        if close_sent {
            let _ = flushed_tx.send(());
        }
    });
    let mut conn_guard = PreauthGuard {
        hygiene,
        armed: true,
    };
    let mut ctx = ConnCtx {
        tx: &tx,
        peer: &peer,
        state: &state,
        registry: &registry,
        lockout: &lockout,
        guard: &mut conn_guard,
        close_pending: false,
        flushed: flushed_rx,
        force_close: tokio::sync::oneshot::channel::<()>().1,
        ws_read: Some(ws_read),
    };
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
    ctx.guard.armed = false;
    let Some(first) = first else {
        ctx.close_pending = true;
        ctx.finish().await;
        return;
    };
    // A framing failure (parse / batch / shape / version / unknown method /
    // bad id) is not an authentication attempt: no lockout strike. Reply with
    // the codec's own error and **close** the socket (the wire contract: a
    // failed first frame roundtrips a codec error *then* a close). The close
    // must be queued here — not just the error — because `finish` awaits the
    // writer's `flushed` signal, which is only emitted once a close frame is
    // on the wire. Queuing only the error (no close) would leave `flushed`
    // pending forever and stall the connection task (the batch/garbage first-
    // frame tests hang on this).
    let env = match decode(&first) {
        Ok(e) => e,
        Err(e) => {
            ctx.guard.armed = false;
            send_error(&tx, None, e.code(), &e.to_string());
            close(&tx);
            ctx.close_pending = true;
            ctx.finish().await;
            return;
        }
    };
    let method = match env.method() {
        Some(m) => m,
        None => {
            send_error(&tx, env.id(), Code::Unauthenticated, "not authenticated");
            close(&tx);
            ctx.close_pending = true;
            ctx.finish().await;
            return;
        }
    };
    let _dropped_rx = dispatch_auth(&env, method, &mut ctx).await;
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
            None => return None,
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

/// Dispatch a post-auth application message to its handler.
fn dispatch_post_auth(ctx: &ConnCtx<'_>, env: &Envelope, method: &str, hostname: &str) {
    match method {
        "circuit/ping" => {
            crate::auth_io::handle_ping(env, ctx.tx, hostname);
        }
        _ => {
            send_error(
                ctx.tx,
                env.id(),
                Code::MethodNotFound,
                &format!("no handler for {method}"),
            );
        }
    }
}

/// The post-auth read loop: dispatch inbound application frames until the
/// peer closes, the registry force-closes (1000/1008), or the stream errors.
async fn post_auth_loop<S: AsyncRead + AsyncWrite + Unpin>(
    mut ws_read: SplitStream<WebSocketStream<S>>,
    ctx: &mut ConnCtx<'_>,
    handle: ConnectionHandle,
    hostname: &str,
) {
    loop {
        tokio::select! {
            biased;
            r = &mut ctx.force_close => {
                let _ = r;
                break;
            }
            msg = ws_read.next() => {
                let Some(msg) = msg else { break };
                match msg {
                    Ok(Message::Text(text)) => {
                        let env = match decode(&text) {
                            Ok(e) => e,
                            Err(e) => {
                                send_error(ctx.tx, None, e.code(), &e.to_string());
                                break;
                            }
                        };
                        if let Some(method) = env.method() {
                            dispatch_post_auth(ctx, &env, method, hostname);
                        }
                    }
                    Ok(Message::Ping(_))
                    | Ok(Message::Pong(_))
                    | Ok(Message::Frame(_)) => {}
                    Ok(Message::Binary(_)) => break,
                    Err(tokio_tungstenite::tungstenite::Error::Capacity(_)) => {
                        ctx.close_pending = true;
                        close_code(
                            ctx.tx,
                            tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Size
                                .into(),
                            "message too big",
                        );
                        break;
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                }
            }
        }
    }
    drop(handle);
}
