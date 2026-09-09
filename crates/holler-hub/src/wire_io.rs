//! Per-connection wire I/O helpers (split out of `serve.rs` so that file
//! stays under the 900-line lint gate): sending error/close/response frames
//! through a connection's outbound channel, the failed-auth lockout strike,
//! and the peer-IP / monotonic-clock helpers those rely on.

use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedSender;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{
    protocol::frame::coding::CloseCode, protocol::CloseFrame, Message,
};

use holler_proto::{Envelope, WireError};
use holler_proto::Code;

use crate::lockout::Lockout;

/// Record a failed authentication for `peer` (the transport peer IP). Returns
/// `true` if this failure **tripped** the lockout (the caller then closes with
/// 1008 instead of the usual error + close). A peer with no parseable IP
/// (should not happen on loopback TCP) is never locked out.
pub fn record_failure(peer: &str, lockout: &Arc<Lockout>) -> bool {
    match peer_ip(peer) {
        Some(ip) => lockout.record_failure(&ip),
        None => false,
    }
}

/// The peer IP parsed out of a `SocketAddr` string (the lockout key). `None`
/// if `peer` is not a parseable address.
pub fn peer_ip(peer: &str) -> Option<std::net::IpAddr> {
    peer.parse::<std::net::SocketAddr>()
        .ok()
        .map(|sa| sa.ip())
}

/// A monotonic millisecond timestamp (unix-ish, from process start) for
/// registry `last_frame_at`. (The registry stores relative monotonic values;
/// they are for freshness, not wall-clock reporting.)
pub fn now_ms() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static ANCHOR: OnceLock<Instant> = OnceLock::new();
    let anchor = ANCHOR.get_or_init(Instant::now);
    anchor.elapsed().as_millis() as u64
}

/// Close a socket with a specific WebSocket close code and reason (the
/// supersede/revoke/lockout/frame-cap codes from #184).
///
/// The close is queued on the outbound writer channel as a `Message::Close`.
/// The per-connection writer task (see `handle_ws_conn`) drains that channel
/// into the socket; on a close message it writes the frame, flushes it, and
/// drops the socket — so the peer's client observes a proper close frame on
/// the wire rather than a bare TCP reset.
pub fn close_code(tx: &UnboundedSender<Message>, code: u16, reason: &str) {
    let frame = CloseFrame {
        code: CloseCode::from(code),
        reason: reason.into(),
    };
    let _ = tx.send(Message::Close(Some(frame)));
}

/// Send an error envelope (echoing `id` when given) as a text frame through
/// the writer channel.
pub fn send_error(tx: &UnboundedSender<Message>, id: Option<&str>, code: Code, message: &str) {
    let frame = match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
        Some(cid) => Envelope::error_frame(&cid, &WireError::new(code, message, None)),
        None => Envelope::Error {
            id: id.map(str::to_owned),
            error: WireError::new(code, message, None),
        },
    };
    let text = holler_proto::encode(&frame).unwrap_or_default();
    let _ = tx.send(Message::text(text));
}

/// Close a raw TCP stream before a WebSocket handshake (used when the
/// pre-auth cap is reached; the socket is refused before it can talk).
pub async fn close_raw_stream(mut stream: TcpStream) {
    let _ = stream.shutdown().await;
}

/// Send a normal (1000) close frame through the writer channel (queued as a
/// `Message::Close` — see [`close_code`]). The frame carries an explicit code
/// + reason: an *empty* close frame (no payload) is a protocol error.
pub fn close(tx: &UnboundedSender<Message>) {
    let frame = CloseFrame {
        code: CloseCode::Normal,
        reason: "".into(),
    };
    let _ = tx.send(Message::Close(Some(frame)));
}

/// Send a response envelope (echoing `id`) with a `result` payload through the
/// writer channel.
pub fn send_result(tx: &UnboundedSender<Message>, id: Option<&str>, result: Option<&serde_json::Value>) {
    let Some(cid) = id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) else {
        return;
    };
    let frame = Envelope::response(&cid, result.cloned());
    let text = holler_proto::encode(&frame).unwrap_or_default();
    let _ = tx.send(Message::text(text));
}
