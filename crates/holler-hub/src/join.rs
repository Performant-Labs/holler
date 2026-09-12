//! Server side of `circuit/join` (docs §3): redeem a one-time join secret,
//! register the body's Ed25519 public key, reply `{client_id}` (issue #323:
//! no credential is minted), and close the one-shot socket.
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
/// join secret and the body's Ed25519 public key to the token store, and on
/// success reply with `{client_id}` (issue #323: no credential is minted) and
/// close. A malformed `body_pubkey` is `-32602 invalid_params`, checked
/// **before** touching the store. A refusal (unknown / already-redeemed /
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
    // redacted (ADR 0002 — secrets never reach a log in the clear). The
    // body's public key is not a secret and is left in the clear.
    holler_proto::emit(&holler_proto::Event {
        component: holler_proto::Component::Wire,
        severity: holler_proto::Severity::Debug,
        direction: holler_proto::LogDirection::In,
        method: "circuit/join",
        id: None,
        peer: None,
        fields: vec![("secret", holler_proto::REDACTED.to_string()), ("body_pubkey", params.body_pubkey.clone())],
        frame: None,
    });
    if !is_valid_ed25519_pubkey_hex(&params.body_pubkey) {
        crate::serve::send_error(sink, id, Code::InvalidParams, "body_pubkey must be 64 lowercase hex chars (a 32-byte Ed25519 public key)").await;
        crate::serve::close(sink).await;
        return;
    }
    let outcome = crate::token::redeem_async(&params.secret, &params.hostname, &params.body_pubkey, state).await;
    match outcome {
        // Success: echo the join request's id and the `{client_id}` result,
        // then close (docs §3).
        Ok(client_id) => {
            let result = serde_json::to_value(JoinResult { client_id }).unwrap_or_default();
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

/// Whether `s` decodes as 64 lowercase hex chars forming a well-formed
/// Ed25519 public key (a valid compressed curve point) — checked before the
/// hub ever stores it, so a malformed key fails the join itself (`-32602`)
/// rather than surfacing later as an inexplicable `circuit/prove` failure.
fn is_valid_ed25519_pubkey_hex(s: &str) -> bool {
    let Ok(bytes) = hex::decode(s) else { return false };
    let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) else { return false };
    ed25519_dalek::VerifyingKey::from_bytes(&arr).is_ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #240
mod tests {
    use std::sync::{Arc, Mutex};

    use futures_util::sink::unfold;

    use super::*;
    use crate::state::HubState;

    /// A per-test scratch state dir under the OS temp dir, removed on drop.
    struct Tdir {
        root: std::path::PathBuf,
    }

    impl Tdir {
        fn new(tag: &str) -> Self {
            let unique = format!(
                "holler-join-{tag}-{}-{:x}",
                std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0),
            );
            let root = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&root).expect("create scratch state dir");
            Self { root }
        }
    }

    impl Drop for Tdir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// A minimal in-memory `Sink<Message>` that just records every frame it is
    /// sent, so a test can assert on `redeem_join`'s wire output without a real
    /// socket.
    fn recording_sink() -> (
        impl Sink<Message, Error = WsError> + Unpin,
        Arc<Mutex<Vec<Message>>>,
    ) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let sent_for_sink = sent.clone();
        let sink = Box::pin(unfold(sent_for_sink, |sent, msg: Message| async move {
            sent.lock().unwrap_or_else(|p| p.into_inner()).push(msg);
            Ok::<_, WsError>(sent)
        }));
        (sink, sent)
    }

    /// A well-formed (real, curve-valid) Ed25519 public key, hex-encoded —
    /// what a body would send as `Join::body_pubkey`.
    fn test_body_pubkey_hex(seed: u8) -> String {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        hex::encode(signing_key.verifying_key().to_bytes())
    }

    /// Issue #323's RED test: `redeem_join` against a freshly-minted, valid
    /// join secret registers the body's public key on the token record and
    /// replies with **only** `{client_id}` — no `credential` field exists on
    /// [`JoinResult`] any more, so this is a compile-time guarantee as much
    /// as a runtime one; the assertions below additionally prove the store
    /// really recorded the public key (the redeem ran for real, through the
    /// async wrapper) and that the raw wire JSON carries no `credential` key.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_registers_body_pubkey_and_returns_no_credential() {
        let dir = Tdir::new("ok");
        let state = HubState::from_root(dir.root.clone());
        let minted = crate::token::mint("kiwi", 3600, &state).expect("mint");
        let body_pubkey = test_body_pubkey_hex(7);

        let (mut sink, sent) = recording_sink();
        let params = Join {
            secret: minted.secret.clone(),
            hostname: "myhost".to_string(),
            body_pubkey: body_pubkey.clone(),
        };
        redeem_join(&mut sink, Some("b-req-1"), params, &state).await;

        let frames = sent.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(frames.len(), 2, "a response frame, then a close frame: {frames:?}");
        let text = frames[0].to_text().expect("first frame is text");
        let env: Envelope = holler_proto::decode(text).expect("first frame decodes as an envelope");
        let Envelope::Response { id, result } = env else {
            panic!("expected a Response envelope, got {env:?}");
        };
        assert_eq!(id, "b-req-1", "the response echoes the request id");
        let raw_result = result.expect("a success carries a result");
        assert!(raw_result.get("credential").is_none(), "the wire result must carry no credential field: {raw_result}");
        let result: JoinResult = serde_json::from_value(raw_result).expect("result decodes as JoinResult");
        assert!(result.client_id.starts_with("cli_"), "client_id: {}", result.client_id);
        assert!(frames[1].is_close(), "second frame is the close frame: {frames:?}");

        // …and the store now shows the token bound with the registered public
        // key (the redeem really ran, through the async wrapper, not a no-op).
        let records = crate::token::list(&state).expect("list");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].state, crate::token::TokenState::Bound);
        assert_eq!(records[0].body_pubkey.as_deref(), Some(body_pubkey.as_str()));
    }

    /// A malformed `body_pubkey` is `-32602 invalid_params`, before the store
    /// is ever touched — the token stays `unused`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn malformed_pubkey_is_invalid_params_and_token_stays_unused() {
        let dir = Tdir::new("badkey");
        let state = HubState::from_root(dir.root.clone());
        let minted = crate::token::mint("kiwi", 3600, &state).expect("mint");

        let (mut sink, sent) = recording_sink();
        let params = Join {
            secret: minted.secret.clone(),
            hostname: "myhost".to_string(),
            body_pubkey: "not-hex".to_string(),
        };
        redeem_join(&mut sink, Some("b-req-3"), params, &state).await;

        let frames = sent.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(frames.len(), 2, "an error frame, then a close frame: {frames:?}");
        let text = frames[0].to_text().expect("first frame is text");
        let env: Envelope = holler_proto::decode(text).expect("first frame decodes as an envelope");
        let Envelope::Error { error, .. } = env else {
            panic!("expected an Error envelope, got {env:?}");
        };
        assert_eq!(error.code, Code::InvalidParams.jsonrpc());

        let records = crate::token::list(&state).expect("list");
        assert_eq!(records[0].state, crate::token::TokenState::Unused, "a malformed pubkey must not consume the join secret");
    }

    /// A secret that matches no minted token is a `join_failed` error naming
    /// the store's refusal, then a close — the failure path also runs through
    /// `redeem_async` unchanged.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unknown_secret_is_join_failed_then_closes() {
        let dir = Tdir::new("bad");
        let state = HubState::from_root(dir.root.clone());
        // Mint one token so the store (and its pepper) exist, but present a
        // secret that matches nothing.
        let _ = crate::token::mint("kiwi", 3600, &state).expect("mint");

        let (mut sink, sent) = recording_sink();
        let params = Join {
            secret: "hlr_join_0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            hostname: "myhost".to_string(),
            body_pubkey: test_body_pubkey_hex(9),
        };
        redeem_join(&mut sink, Some("b-req-2"), params, &state).await;

        let frames = sent.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(frames.len(), 2, "an error frame, then a close frame: {frames:?}");
        let text = frames[0].to_text().expect("first frame is text");
        let env: Envelope = holler_proto::decode(text).expect("first frame decodes as an envelope");
        let Envelope::Error { id, error } = env else {
            panic!("expected an Error envelope, got {env:?}");
        };
        assert_eq!(id, Some("b-req-2".to_string()));
        assert_eq!(error.code, Code::JoinFailed.jsonrpc());
        assert!(frames[1].is_close());
    }
}
