//! Server side of `circuit/join` (docs §3): redeem a one-time join secret for
//! a `{client_id, credential}` and close the one-shot socket.
//!
//! This is the hub's half of the body's `body join` bootstrap (story #176). It
//! lives in its own module (out of `serve.rs`) so the `serve` lifecycle file
//! stays under the 900-line lint budget while the join machinery grows. The
//! low-level `send_error`/`close` frame helpers it relies on remain in
//! `serve.rs` (they are shared with the unauthenticated/parse-error paths).

use futures_util::{Sink, SinkExt};
use holler_proto::{Code, Envelope, Join, JoinResult};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::state::HubState;

/// Redeem a `circuit/join` (docs §3) on a fresh socket: present the one-time
/// join secret to the token store, and on success reply with the
/// `{client_id, credential}` and close. A refusal (unknown / already-redeemed /
/// revoked / expired) is a `join_failed` error carrying the store's own
/// message (the body prints it verbatim), then the socket closes.
pub(crate) async fn redeem_join<S>(
    sink: &mut S,
    id: Option<&str>,
    params: Join,
    state: &HubState,
) where
    S: Sink<Message, Error = WsError> + Unpin,
{
    // The join secret crosses the wire; if it is noisy-logged it must be
    // redacted (ADR 0002 — secrets never reach a log in the clear).
    holler_proto::emit(&holler_proto::Event {
        component: holler_proto::Component::Wire,
        severity: holler_proto::Severity::Debug,
        direction: holler_proto::LogDirection::In,
        method: "circuit/join",
        id: None,
        peer: None,
        fields: vec![("secret", holler_proto::REDACTED.to_string())],
        frame: None,
    });
    let outcome = crate::token::redeem(&params.secret, &params.hostname, state);
    match outcome {
        // Success: echo the join request's id and the `{client_id, credential}`
        // result, then close (docs §3).
        Ok((client_id, credential)) => {
            let result = serde_json::to_value(JoinResult {
                client_id,
                credential,
            })
            .unwrap_or_default();
            let frame = match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
                Some(cid) => Envelope::response(&cid, Some(result)),
                None => Envelope::Response {
                    id: id.map(str::to_owned).unwrap_or_default(),
                    result: Some(result),
                },
            };
            let text = holler_proto::encode(&frame).unwrap_or_default();
            if sink.send(Message::text(text)).await.is_err() {
                return;
            }
            let _ = sink.flush().await;
        }
        // A refused redeem: the hub's one-line message goes back (the body
        // prints it verbatim, exit 1).
        Err(e) => {
            crate::serve::send_error(sink, id, Code::JoinFailed, e.message()).await;
        }
    }
    // The one-shot handshake ends here: the hub closes after replying.
    crate::serve::close(sink).await;
}
