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
use holler_proto::noise::{
    HUB_UNAVAILABLE_REASON, KEY_MISMATCH_REASON, NOISE_MESSAGE_ONE_REJECTED_REASON, NO_PUBLIC_KEY_REASON, TOKEN_NOT_BOUND_REASON,
    TOKEN_UNKNOWN_REASON,
};
use holler_proto::{Authenticate, Code, CorrelationId, Envelope, Hello, HelloRole, WireError};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::config::SessionConfig;
use crate::identity::BodyIdentity;

use super::{send, timeout_next_envelope, Attempt};

/// Run the `circuit/authenticate` → `circuit/prove` Noise XK handshake
/// (issue #338) and await the final answer. A `-32002` at either step is
/// classified by [`refusal_attempt`] on its `error.data.reason` (issue #486):
/// `hub_unavailable` maps to [`Attempt::Dropped`] (retry with backoff), every
/// other refusal to [`Attempt::AuthFailed`] (no retry), as does a corrupt
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
        // Issue #486: one classifier for a `-32002` at either step, on the
        // structured `error.data.reason`, never on `message` text.
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => {
            return Err(refusal_attempt(error, &identity.hub_pubkey));
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
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => Err(refusal_attempt(error, &identity.hub_pubkey)),
        Envelope::Error { error, .. } => Err(Attempt::Dropped(format!("prove refused: {}", error.message))),
        _ => Err(Attempt::Dropped("unexpected reply to circuit/prove".to_string())),
    }
}

/// What the reconnect loop does with a `-32002 unauthenticated` refusal of
/// `circuit/authenticate` or `circuit/prove` (issue #486), decided on its
/// `error.data.reason` (docs §8), never on `error.message`, which the hub is
/// free to reword:
///
/// - `hub_unavailable` is the hub's own fault (issue #485: its token store or
///   identity key), says nothing about this body's credential, and is not
///   counted toward its lockout: [`Attempt::Dropped`], so the loop backs off,
///   reports `reconnecting` and tries again until the hub recovers.
/// - `noise_message_one_rejected` has exactly one cause (see
///   [`holler_proto::noise`]'s module doc): the hub key this body pinned at
///   `body join` is not the key of the hub it is talking to, so it stops as a
///   hub public key mismatch.
/// - Every other code stops the body, in plain words ([`refusal_words`]) with
///   the code and the hub's message: each one counts toward this peer's
///   lockout, so a retry would only lock the body out. That includes a code
///   this body does not know yet, from a newer hub.
/// - No reason at all (a hub older than #486) stops it with the hub's message
///   verbatim, as before.
fn refusal_attempt(error: WireError, pinned_hub_pubkey: &str) -> Attempt {
    let Some(code) = error.data.and_then(|data| data.reason) else {
        return Attempt::AuthFailed(error.message);
    };
    match code.as_str() {
        HUB_UNAVAILABLE_REASON => Attempt::Dropped(format!("hub unavailable (reason {code}): {}", error.message)),
        NOISE_MESSAGE_ONE_REJECTED_REASON => Attempt::AuthFailed(format!(
            "hub public key mismatch: this hub rejected this body's handshake — its real key does not match the one pinned at `body join` ({pinned_hub_pubkey}) — refusing to connect (re-pair with `body join` only if you trust this is an intentional hub key rotation)"
        )),
        _ => Attempt::AuthFailed(format!("{} (reason {code}); the hub said: {}", refusal_words(&code), error.message)),
    }
}

/// Plain words for a counted refusal code (issue #486): what happened and what
/// fixes it, so an operator does not have to decode the code. `body run`
/// prints them after `error: authentication failed: `, `body confirm` after
/// `error: `.
fn refusal_words(code: &str) -> &'static str {
    match code {
        TOKEN_NOT_BOUND_REASON => {
            "this hub no longer accepts this body's token: it was revoked, or never joined; mint a new token and re-run `body join`"
        }
        TOKEN_UNKNOWN_REASON => {
            "this hub has no such token: it was deleted, or this body is joined to a different hub; mint a new token and re-run `body join`"
        }
        NO_PUBLIC_KEY_REASON | KEY_MISMATCH_REASON => "this body's key does not match the key the hub registered at join; re-run `body join`",
        _ => "the hub refused this body's authentication",
    }
}

/// The bidirectional hello exchange (see the module doc's "Decisions made").
/// `configs` (issue #185) is this body's own session config — its harness
/// ids populate the hello's `harnesses` field, which is what the hub's
/// confirmation pass ([`crate::query`]'s hub-side counterpart, `holler_hub::
/// circuit::confirm_harnesses`) probes right after this exchange completes.
/// `role` (issue #508) is `Body` for the normal live loop
/// (`connection::connect_and_serve`, which always passes it explicitly) or
/// `Admin` for `crate::admin_client`'s one-shot hello — an admin hello
/// advertises no harnesses and no sessions (MO 7): it is a control client,
/// never a body a `session/prompt` could be routed to. `pub(crate)` (not
/// `pub(super)`) so `admin_client`, a sibling module of `connection` in this
/// crate, can call it too, without a second copy of the hub-key pinning
/// logic below.
///
/// Issue #322: the hub's own `Hello` document (the *result* of this body's
/// `circuit/hello` request) carries `hub_pubkey` — the hub's X25519 public
/// key. It is compared here, on **every** (re)connect, against the key
/// pinned at `body join` (`identity.hub_pubkey`). A mismatch — or a hub that
/// answers with none at all, once this body has ever pinned one — is a hard
/// failure ([`Attempt::AuthFailed`], no retry, no prompt): the whole point of
/// pinning is that this body never silently talks to a different hub.
pub(crate) async fn hello_exchange<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    identity: &BodyIdentity,
    configs: &[SessionConfig],
    role: HelloRole,
) -> Result<(), Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    // Issue #508: only a body hello advertises harnesses/sessions at all —
    // an admin client is a control connection, not a box that can host a
    // spawned/attached session (MO 7's exact wire shape).
    let harnesses = if role == HelloRole::Body {
        let mut h: Vec<String> = configs.iter().map(|c| c.harness.clone()).collect();
        h.sort();
        h.dedup();
        Some(h)
    } else {
        None
    };
    let sessions = (role == HelloRole::Body).then(Vec::new);

    let cid = CorrelationId::mint_body();
    let hello = Hello {
        protocol: holler_proto::PROTOCOL_VERSION,
        protocol_min: holler_proto::PROTOCOL_MIN,
        protocol_max: holler_proto::PROTOCOL_MAX,
        role,
        hostname: identity.hostname.clone(),
        token_id: Some(identity.token_id.clone()),
        client_id: Some(identity.client_id.clone()),
        features: Vec::new(),
        harnesses,
        harnesses_known: None,
        harnesses_confirmed: None,
        sessions,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #486
mod tests {
    use super::*;
    use holler_proto::WireError;

    const PINNED: &str = "aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11";

    fn refusal(message: &str, reason: Option<&'static str>) -> WireError {
        WireError::new(Code::Unauthenticated, message, reason)
    }

    /// The text of an `AuthFailed`; panics (naming what it was) on anything else.
    fn auth_failed(a: Attempt) -> String {
        match a {
            Attempt::AuthFailed(m) => m,
            Attempt::Dropped(m) => panic!("expected AuthFailed (stop), got Dropped (retry): {m}"),
            Attempt::Ended(_) => panic!("expected AuthFailed, got Ended"),
        }
    }

    /// #486: a hub-side fault is the hub's, not the credential's, so the body
    /// backs off and retries through the existing `Dropped` arm.
    #[test]
    fn hub_unavailable_is_dropped_and_retried() {
        let msg = "authentication failed: the token store could not be read";
        match refusal_attempt(refusal(msg, Some(holler_proto::noise::HUB_UNAVAILABLE_REASON)), PINNED) {
            Attempt::Dropped(text) => {
                assert!(text.starts_with("hub unavailable (reason hub_unavailable): "), "{text}");
                assert!(text.ends_with(msg), "the hub's message follows verbatim: {text}");
            }
            Attempt::AuthFailed(m) => panic!("hub_unavailable must be retried, not a stop: {m}"),
            Attempt::Ended(_) => panic!("hub_unavailable must be retried, got Ended"),
        }
    }

    /// #452 slice: a revoked token stops the body and says so in plain words,
    /// with the stable code and the hub's own message.
    #[test]
    fn token_not_bound_stops_with_plain_words_and_the_code() {
        let msg = "authentication failed: token tok_x is not bound";
        let text = auth_failed(refusal_attempt(refusal(msg, Some("token_not_bound")), PINNED));
        assert!(text.contains("this hub no longer accepts this body's token"), "{text}");
        assert!(text.contains("was revoked, or never joined"), "{text}");
        assert!(text.contains("mint a new token and re-run `body join`"), "{text}");
        assert!(text.contains(&format!("(reason token_not_bound); the hub said: {msg}")), "{text}");
    }

    #[test]
    fn token_unknown_stops_with_plain_words_and_the_code() {
        let msg = "authentication failed: no such token tok_x";
        let text = auth_failed(refusal_attempt(refusal(msg, Some("token_unknown")), PINNED));
        assert!(text.contains("this hub has no such token"), "{text}");
        assert!(text.contains("this body is joined to a different hub"), "{text}");
        assert!(text.contains(&format!("(reason token_unknown); the hub said: {msg}")), "{text}");
    }

    #[test]
    fn key_codes_stop_and_say_the_key_does_not_match() {
        for code in ["no_public_key", "key_mismatch"] {
            let msg = "authentication failed: static key mismatch";
            let text = auth_failed(refusal_attempt(refusal(msg, Some(code)), PINNED));
            assert!(text.contains("this body's key does not match the key the hub registered at join"), "{code}: {text}");
            assert!(text.contains(&format!("(reason {code}); the hub said: {msg}")), "{code}: {text}");
        }
    }

    /// Every other counted code stops too (Decision 5: a retry would be a
    /// counted failure and trip the body's own lockout), including a code a
    /// newer hub might add.
    #[test]
    fn every_other_reason_stops_with_the_generic_words() {
        for code in ["prove_timeout", "protocol_error", "handshake_failed", "auth_failed", "token_expired", "a_future_code"] {
            let msg = "authentication failed: something";
            let text = auth_failed(refusal_attempt(refusal(msg, Some(code)), PINNED));
            assert!(text.contains("the hub refused this body's authentication"), "{code}: {text}");
            assert!(text.contains(&format!("(reason {code}); the hub said: {msg}")), "{code}: {text}");
        }
    }

    /// A hub older than #486 sends no reason: stop with its message, today's behaviour.
    #[test]
    fn no_reason_stops_with_the_hubs_message_verbatim() {
        let msg = "authentication failed: token tok_x is not bound";
        assert_eq!(auth_failed(refusal_attempt(refusal(msg, None), PINNED)), msg);
    }

    #[test]
    fn noise_message_one_rejected_is_the_hub_key_mismatch() {
        let reason = Some(holler_proto::noise::NOISE_MESSAGE_ONE_REJECTED_REASON);
        let text = auth_failed(refusal_attempt(refusal("authentication failed: bad handshake message", reason), PINNED));
        assert!(text.starts_with("hub public key mismatch"), "{text}");
        assert!(text.contains(PINNED), "the text names the pinned key: {text}");
    }
}
