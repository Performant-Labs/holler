//! WebSocket wire-frame helpers shared across the hub's connection paths
//! (`serve.rs`'s pre-auth admission, `circuit.rs`/`circuit/auth.rs`'s
//! authenticate/prove handling, `join.rs`) — split out of `serve.rs` (issue
//! #338's `send_error_with_reason` addition crossed the file-size guard) the
//! same way `control_server.rs` was split out of it earlier (issue #182).

use futures_util::{Sink, SinkExt};
use holler_proto::{Code, Envelope, WireError};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

/// Send an error envelope (echoing `id` when given) as a text frame, then let
/// the sink flush.
pub(crate) async fn send_error(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    id: Option<&str>,
    code: Code,
    message: &str,
) {
    send_error_with_reason(sink, id, code, message, None).await;
}

/// Like [`send_error`], but also sets `error.data.reason` (docs §8: the same
/// per-code disambiguator `-32003 unknown_session` already carries, e.g.
/// `"name held by another body"`) — so a caller can act on a structured value
/// instead of pattern-matching `message` text.
pub(crate) async fn send_error_with_reason(
    sink: &mut (impl Sink<Message, Error = WsError> + Unpin),
    id: Option<&str>,
    code: Code,
    message: &str,
    reason: Option<&'static str>,
) {
    let frame = match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
        Some(cid) => Envelope::error_frame(&cid, &WireError::new(code, message, reason)),
        None => Envelope::Error {
            id: id.map(str::to_owned),
            error: WireError::new(code, message, reason),
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
