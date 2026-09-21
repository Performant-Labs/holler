//! The `circuit/authenticate` → `circuit/prove` Noise XK handshake (issue
//! #338, replacing #323's Ed25519-signed transcript challenge-response) and
//! the bidirectional `circuit/hello` exchange with hub-key pinning (issue
//! #322): split out of `connection.rs` proper once this file's own growth
//! pushed it past the workspace's 900-line build guard (`scripts/lint.sh`
//! check 4) — a pure relocation, no behavior change of its own. Mirrors this
//! crate's own `connection/session_dispatch.rs` split, and `holler-hub`'s
//! `circuit/auth.rs` split for the hub side of the same handshake.
//!
//! This body is the Noise **initiator**: per XK's own `K`, it already knows
//! the hub's static public key — `identity.hub_pubkey`, pinned at `body
//! join` (issue #322) — so it can build message 1 before ever hearing from
//! the hub. A hub that presents a different real key than what was pinned
//! (a rotated key, or an impersonator) makes the *hub's* own message 1
//! processing fail, surfacing here as a wire `-32002` from `circuit/
//! authenticate` — see [`holler_proto::noise`]'s own module doc for why.

use std::path::Path;

use futures_util::{Sink, Stream};
use holler_proto::{Authenticate, Code, CorrelationId, Envelope, Hello, HelloRole};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::config::SessionConfig;
use crate::identity::BodyIdentity;

use super::{send, timeout_next_envelope, Attempt};

/// Run the `circuit/authenticate` → `circuit/prove` Noise XK handshake
/// (issue #338) and await the final answer. `-32002` at either step maps to
/// [`Attempt::AuthFailed`] (no retry — the body's connection loop treats
/// that code, and only that code, as "do not retry"), as does a corrupt
/// local X25519 identity or pinned hub key (re-running `body join` is the
/// only fix, so backing off and retrying the same broken local state would
/// just spin). Every other failure (connect-adjacent decode errors, a
/// foreign error code, a closed socket, a 10s timeout, a local handshake
/// step failing to process the hub's own message) maps to
/// [`Attempt::Dropped`] (retry with backoff).
///
/// On success, returns the pairing SAS (issue #339,
/// [`holler_proto::sas::derive_sas`]) derived from this now-completed
/// handshake's hash — the caller ([`super::connect_and_serve`]) logs it
/// alongside `conn_connected` so the operator can compare it against the
/// hub's own console. A [`holler_proto::sas::SasError`] here would mean this
/// handshake's own `handshake_hash_hex` was malformed, which cannot happen
/// for a real, just-finished [`holler_proto::noise::HandshakeXk`] — still
/// mapped to [`Attempt::Dropped`] (retry) rather than unwrapped, the same
/// fail-closed discipline every other near-impossible step in this fn
/// follows.
pub(crate) async fn authenticate<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity, state_root: &Path) -> Result<String, Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let x25519_identity = crate::x25519_identity::ensure(state_root).map_err(|e| {
        Attempt::AuthFailed(format!("this body's X25519 identity is corrupt or could not be resolved: {e} — re-run `body join`"))
    })?;
    let body_secret = x25519_identity.secret_bytes();

    let hub_pubkey: [u8; 32] = hex::decode(&identity.hub_pubkey)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| Attempt::AuthFailed("this body's pinned hub public key is corrupt — re-run `body join`".to_string()))?;

    let prologue = holler_proto::noise::build_prologue(holler_proto::PROTOCOL_VERSION, &identity.token_id, &identity.server_url);
    let mut handshake = holler_proto::noise::HandshakeXk::initiator(&body_secret, &hub_pubkey, &prologue)
        .map_err(|e| Attempt::Dropped(format!("could not start the handshake: {e}")))?;

    // Step 1: `circuit/authenticate` names the token, this body's own belief
    // of the address it is dialing, and Noise message 1 (`e, es`); the hub's
    // result is message 2 (`e, ee`), not an ack.
    let cid = CorrelationId::mint_body();
    let msg1 = handshake.write_message().map_err(|e| Attempt::Dropped(format!("write handshake message 1: {e}")))?;
    let params = Authenticate {
        protocol: holler_proto::PROTOCOL_VERSION,
        token_id: identity.token_id.clone(),
        hostname: identity.hostname.clone(),
        advertised_url: identity.server_url.clone(),
        message: hex::encode(msg1),
    };
    let params = serde_json::to_value(params).map_err(|e| Attempt::Dropped(format!("encode auth: {e}")))?;
    let req = Envelope::request(&cid, "circuit/authenticate", Some(params));
    send(sink, &req).await.map_err(|_| Attempt::Dropped("send auth: socket closed".to_string()))?;

    let env = timeout_next_envelope(stream)
        .await
        .ok_or_else(|| Attempt::Dropped("no answer to circuit/authenticate".to_string()))?;
    let msg2 = match env {
        Envelope::Response { id, result } if id == cid.as_str() => {
            let challenge: holler_proto::AuthChallenge = result
                .and_then(|v| serde_json::from_value(v).ok())
                .ok_or_else(|| Attempt::Dropped("malformed circuit/authenticate challenge".to_string()))?;
            hex::decode(&challenge.message).map_err(|e| Attempt::Dropped(format!("malformed handshake message 2: {e}")))?
        }
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => {
            // A `-32002` refusal of `circuit/authenticate` itself (as opposed
            // to `circuit/prove`, below) can only mean the hub rejected Noise
            // message 1 — this body's very first handshake message. Per
            // `holler_proto::noise`'s own module doc, that specific rejection
            // has exactly one cause: this body built message 1 against a hub
            // static key that does not match the hub it is actually talking
            // to. The hub marks that one cause with `error.data.reason` (see
            // `holler_proto::noise::NOISE_MESSAGE_ONE_REJECTED_REASON`) so
            // this disambiguates on a structured value, not `error.message`
            // text the hub is free to reword.
            let is_hub_key_mismatch = error
                .data
                .as_deref()
                .and_then(|d| d.reason.as_deref())
                == Some(holler_proto::noise::NOISE_MESSAGE_ONE_REJECTED_REASON);
            return Err(Attempt::AuthFailed(if is_hub_key_mismatch {
                format!(
                    "hub public key mismatch: this hub rejected this body's handshake — its real key does not match the one pinned at `body join` ({}) — refusing to connect (re-pair with `body join` only if you trust this is an intentional hub key rotation)",
                    identity.hub_pubkey
                )
            } else {
                error.message
            }));
        }
        // Issue #340: a `-32000` here means this hub's protocol floor is
        // higher than what this body claimed — a hard re-pair event (#321's
        // "Protocol break handling"), not a transient drop. The hub's own
        // message already names the mismatch and the fix (re-run `body
        // join` on an upgraded body), so it is surfaced verbatim rather than
        // retried with backoff.
        Envelope::Error { error, .. } if error.code == Code::UnsupportedVersion.jsonrpc() => {
            return Err(Attempt::AuthFailed(error.message));
        }
        Envelope::Error { error, .. } => {
            return Err(Attempt::Dropped(format!("authenticate refused: {}", error.message)));
        }
        _ => return Err(Attempt::Dropped("unexpected reply to circuit/authenticate".to_string())),
    };

    // Step 2: process message 2, write message 3 (`s, se`, carrying this
    // body's own static key encrypted), and answer `circuit/prove`. The
    // private key never leaves this function.
    handshake.read_message(&msg2).map_err(|e| Attempt::Dropped(format!("handshake message 2 rejected: {e}")))?;
    let msg3 = handshake.write_message().map_err(|e| Attempt::Dropped(format!("write handshake message 3: {e}")))?;
    let prove_cid = CorrelationId::mint_body();
    let prove_params = holler_proto::Prove {
        token_id: identity.token_id.clone(),
        message: hex::encode(msg3),
    };
    let prove_params = serde_json::to_value(prove_params).map_err(|e| Attempt::Dropped(format!("encode prove: {e}")))?;
    let prove_req = Envelope::request(&prove_cid, "circuit/prove", Some(prove_params));
    send(sink, &prove_req).await.map_err(|_| Attempt::Dropped("send prove: socket closed".to_string()))?;

    let env = timeout_next_envelope(stream)
        .await
        .ok_or_else(|| Attempt::Dropped("no answer to circuit/prove".to_string()))?;
    match env {
        Envelope::Response { id, .. } if id == prove_cid.as_str() => holler_proto::sas::derive_sas(&handshake.handshake_hash_hex())
            .map_err(|e| Attempt::Dropped(format!("could not derive the pairing SAS: {e}"))),
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
        // Issue #340: defense-in-depth mirror of the hub's own hello-level
        // check (a genuine version mismatch normally never reaches this far
        // — it is already caught at `circuit/authenticate`, above — but this
        // stays for the same reason the hub_pubkey check stays even though
        // Noise already enforces it: belt and suspenders).
        Some(Envelope::Error { error, .. }) if error.code == Code::UnsupportedVersion.jsonrpc() => {
            return Err(Attempt::AuthFailed(error.message));
        }
        _ => return Err(Attempt::Dropped("no answer to circuit/hello".to_string())),
    };
    if !holler_proto::is_supported_version(hub_hello.protocol) {
        return Err(Attempt::AuthFailed(format!(
            "protocol mismatch: this hub speaks protocol {}, this body requires protocol {} — upgrade this body (or the hub) so both sides speak a matching protocol, then re-pair with `body join`",
            hub_hello.protocol,
            holler_proto::PROTOCOL_MIN
        )));
    }
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
