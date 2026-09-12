//! The `circuit/authenticate` → `circuit/prove` challenge-response (issue
//! #323) and the bidirectional `circuit/hello` exchange with hub-key pinning
//! (issue #322): split out of `connection.rs` proper once this file's own
//! growth pushed it past the workspace's 900-line build guard
//! (`scripts/lint.sh` check 4) — a pure relocation, no behavior change of
//! its own. Mirrors this crate's own `connection/session_dispatch.rs` split,
//! and `holler-hub`'s `circuit/auth.rs` split for the hub side of the same
//! handshake.

use futures_util::{Sink, Stream};
use holler_proto::{Authenticate, Code, CorrelationId, Envelope, Hello, HelloRole};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::config::SessionConfig;
use crate::identity::BodyIdentity;

use super::{send, timeout_next_envelope, Attempt};

/// Run the `circuit/authenticate` → `circuit/prove` challenge-response
/// (issue #323) and await the final answer. `-32002` at either step maps to
/// [`Attempt::AuthFailed`] (no retry, per the issue — the body's connection
/// loop treats that code, and only that code, as "do not retry"); every
/// other failure (connect-adjacent decode errors, a foreign error code, a
/// closed socket, a 10s timeout, a corrupt local signing key) maps to
/// [`Attempt::Dropped`] (retry with backoff) except the corrupt-key case,
/// which is also unretryable (re-running `body join` is the only fix, so
/// backing off and trying the same broken key again would just spin).
pub(super) async fn authenticate<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity) -> Result<(), Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let Some(signing_key) = identity.signing_key() else {
        return Err(Attempt::AuthFailed(
            "this body's persisted signing key is corrupt or missing — re-run `body join`".to_string(),
        ));
    };

    // Step 1: `circuit/authenticate` names the token and this body's own
    // belief of the address it is dialing; the hub's result is a fresh
    // nonce challenge, not an ack.
    let cid = CorrelationId::mint_body();
    let params = Authenticate {
        token_id: identity.token_id.clone(),
        hostname: identity.hostname.clone(),
        advertised_url: identity.server_url.clone(),
    };
    let params = serde_json::to_value(params).map_err(|e| Attempt::Dropped(format!("encode auth: {e}")))?;
    let req = Envelope::request(&cid, "circuit/authenticate", Some(params));
    send(sink, &req).await.map_err(|_| Attempt::Dropped("send auth: socket closed".to_string()))?;

    let env = timeout_next_envelope(stream)
        .await
        .ok_or_else(|| Attempt::Dropped("no answer to circuit/authenticate".to_string()))?;
    let nonce = match env {
        Envelope::Response { id, result } if id == cid.as_str() => {
            let challenge: holler_proto::AuthChallenge = result
                .and_then(|v| serde_json::from_value(v).ok())
                .ok_or_else(|| Attempt::Dropped("malformed circuit/authenticate challenge".to_string()))?;
            challenge.nonce
        }
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => {
            return Err(Attempt::AuthFailed(error.message));
        }
        Envelope::Error { error, .. } => {
            return Err(Attempt::Dropped(format!("authenticate refused: {}", error.message)));
        }
        _ => return Err(Attempt::Dropped("unexpected reply to circuit/authenticate".to_string())),
    };

    // Step 2: sign the transcript over the hub's own nonce and answer
    // `circuit/prove`. The private key never leaves this function.
    let transcript = holler_proto::transcript::build(
        holler_proto::PROTOCOL_VERSION,
        "body",
        &nonce,
        &identity.token_id,
        &identity.server_url,
    );
    let signature = ed25519_dalek::Signer::sign(&signing_key, &transcript);
    let prove_cid = CorrelationId::mint_body();
    let prove_params = holler_proto::Prove {
        token_id: identity.token_id.clone(),
        signature: hex::encode(signature.to_bytes()),
    };
    let prove_params = serde_json::to_value(prove_params).map_err(|e| Attempt::Dropped(format!("encode prove: {e}")))?;
    let prove_req = Envelope::request(&prove_cid, "circuit/prove", Some(prove_params));
    send(sink, &prove_req).await.map_err(|_| Attempt::Dropped("send prove: socket closed".to_string()))?;

    let env = timeout_next_envelope(stream)
        .await
        .ok_or_else(|| Attempt::Dropped("no answer to circuit/prove".to_string()))?;
    match env {
        Envelope::Response { id, .. } if id == prove_cid.as_str() => Ok(()),
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => {
            Err(Attempt::AuthFailed(error.message))
        }
        Envelope::Error { error, .. } => Err(Attempt::Dropped(format!("prove refused: {}", error.message))),
        _ => Err(Attempt::Dropped("unexpected reply to circuit/prove".to_string())),
    }
}

/// The bidirectional hello exchange (see the module doc's "Decisions made").
/// `configs` (issue #185) is this body's own session config — its harness
/// ids populate the hello's `harnesses` field, which is what the hub's
/// confirmation pass ([`crate::query`]'s hub-side counterpart, `holler_hub::
/// circuit::confirm_harnesses`) probes right after this exchange completes.
///
/// Issue #322: the hub's own `Hello` document (the *result* of this body's
/// `circuit/hello` request) carries `hub_pubkey` — the hub's X25519 public
/// key. It is compared here, on **every** (re)connect, against the key
/// pinned at `body join` (`identity.hub_pubkey`). A mismatch — or a hub that
/// answers with none at all, once this body has ever pinned one — is a hard
/// failure ([`Attempt::AuthFailed`], no retry, no prompt): the whole point of
/// pinning is that this body never silently talks to a different hub.
pub(super) async fn hello_exchange<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    identity: &BodyIdentity,
    configs: &[SessionConfig],
) -> Result<(), Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let mut harnesses: Vec<String> = configs.iter().map(|c| c.harness.clone()).collect();
    harnesses.sort();
    harnesses.dedup();

    let cid = CorrelationId::mint_body();
    let hello = Hello {
        protocol: holler_proto::PROTOCOL_VERSION,
        protocol_min: holler_proto::PROTOCOL_MIN,
        protocol_max: holler_proto::PROTOCOL_MAX,
        role: HelloRole::Body,
        hostname: identity.hostname.clone(),
        token_id: Some(identity.token_id.clone()),
        client_id: Some(identity.client_id.clone()),
        features: Vec::new(),
        harnesses: Some(harnesses),
        harnesses_known: None,
        harnesses_confirmed: None,
        sessions: Some(Vec::new()),
        hub_pubkey: None,
    };
    let params = serde_json::to_value(hello).map_err(|e| Attempt::Dropped(format!("encode hello: {e}")))?;
    let req = Envelope::request(&cid, "circuit/hello", Some(params));
    send(sink, &req).await.map_err(|_| Attempt::Dropped("send hello: socket closed".to_string()))?;

    let hub_hello = match timeout_next_envelope(stream).await {
        Some(Envelope::Response { id, result }) if id == cid.as_str() => result
            .and_then(|v| serde_json::from_value::<Hello>(v).ok())
            .ok_or_else(|| Attempt::Dropped("malformed hub circuit/hello reply".to_string()))?,
        _ => return Err(Attempt::Dropped("no answer to circuit/hello".to_string())),
    };
    match hub_hello.hub_pubkey {
        Some(seen) if seen == identity.hub_pubkey => {}
        Some(seen) => {
            return Err(Attempt::AuthFailed(format!(
                "hub public key mismatch: pinned {} but this hub presented {seen} — refusing to connect (re-pair with `body join` only if you trust this is an intentional hub key rotation)",
                identity.hub_pubkey
            )));
        }
        None => {
            return Err(Attempt::AuthFailed(
                "the hub sent no public key in its circuit/hello — refusing to connect to an unpinned hub".to_string(),
            ));
        }
    }

    // The hub's own hello: a request we must answer with `{}`.
    match timeout_next_envelope(stream).await {
        Some(Envelope::Request { id, method, .. }) if method == "circuit/hello" => {
            let cid = CorrelationId::parse(&id).map_err(|e| Attempt::Dropped(format!("bad hub hello id: {e}")))?;
            let ack = Envelope::response(&cid, Some(serde_json::json!({})));
            send(sink, &ack).await.map_err(|_| Attempt::Dropped("send hello ack: socket closed".to_string()))
        }
        _ => Err(Attempt::Dropped("no hub-initiated circuit/hello".to_string())),
    }
}
