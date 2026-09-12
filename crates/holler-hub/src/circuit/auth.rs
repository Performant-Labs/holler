//! The `circuit/authenticate` → `circuit/prove` challenge-response (issue
//! #323): split out of `circuit.rs` proper once this file's own growth (the
//! nonce challenge, the proof verification, and their shared failure-funnel)
//! pushed it past the workspace's 900-line build guard (`scripts/lint.sh`
//! check 4) — a pure relocation, no behavior change of its own. Mirrors
//! `circuit/dispatch.rs`'s own split for the same reason.
//!
//! [`handle_authenticated`] (still in `circuit.rs`) orchestrates the three
//! steps here plus the hello exchange and the live session loop; this module
//! owns only the challenge-response itself.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::{Sink, Stream};
use holler_proto::{Authenticate, Code};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::Registry;
use crate::serve::{close, send_error};
use crate::state::HubState;

use super::{next_envelope, PROVE_TIMEOUT};

/// [`super::handle_authenticated`]'s shared hub-wide dependencies, bundled
/// purely to keep that fn's own argument count under clippy's
/// `too_many_arguments` gate (issue #184 added `peer`/`lockout` alongside
/// the pre-existing `registry`/`roster` — the same "bundle unrelated shared
/// state into one struct" discipline `CommandChannels` uses in `circuit.rs`).
/// Every field is a shared reference, so `AuthDeps` itself is `Copy` —
/// passing it to a helper never moves it out of the caller.
#[derive(Clone, Copy)]
pub struct AuthDeps<'a> {
    pub registry: &'a Registry,
    pub roster: &'a std::sync::Arc<crate::roster::Roster>,
    pub peer: &'a str,
    pub lockout: &'a std::sync::Arc<crate::lockout::Lockout>,
}

/// Refuse this connection as `-32002 unauthenticated`: record a lockout
/// failure (keyed by the transport IP, never the claimed hostname — that is
/// unauthenticated input), send the wire refusal, close the socket, and drop
/// the roster row (issue #186: an unauthenticated body is permanently gone —
/// a bad token id/signature can never succeed on retry with the same
/// material, so the TTL would only keep a stale row around).
///
/// The single funnel every failure branch of the `circuit/authenticate` →
/// `circuit/prove` handshake (issue #323) goes through, so the four-plus
/// distinct failure points in [`begin_authenticate`]/[`await_prove`]/
/// [`finish_prove`] don't each repeat this five-line sequence (keeping every
/// one of those under clippy's cognitive-complexity gate).
pub(super) async fn refuse_unauthenticated<Snk>(
    sink: &mut Snk,
    id: Option<&str>,
    message: &str,
    token_id: &str,
    peer_ip: &str,
    deps: &AuthDeps<'_>,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if let Ok(ip) = peer_ip.parse() {
        deps.lockout.record_failure(&ip);
    }
    send_error(sink, id, Code::Unauthenticated, message).await;
    close(sink).await;
    deps.roster.clear(token_id);
}

/// Step 1 of `circuit/authenticate` → `circuit/prove` (issue #323): resolve
/// the bound token record for `params.token_id` — an unknown, unbound,
/// expired, or revoked token, or one with no registered public key, is
/// `-32002` via [`refuse_unauthenticated`] (the same fail-shape the old
/// credential check had: the body's connection loop treats that code, and
/// only that code, as "do not retry"). No lockout success/reset happens
/// here — resolving a *token id* proves nothing yet; only a verified
/// signature does ([`finish_prove`]).
pub(super) async fn begin_authenticate<Snk>(
    sink: &mut Snk,
    id: Option<&str>,
    params: &Authenticate,
    state: &HubState,
    peer_ip: &str,
    deps: &AuthDeps<'_>,
) -> Option<crate::token::Record>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    match crate::token::bound_record_async(&params.token_id, state).await {
        Ok(record) => Some(record),
        Err(e) => {
            refuse_unauthenticated(sink, id, &format!("authentication failed: {e}"), &params.token_id, peer_ip, deps).await;
            None
        }
    }
}

/// The length, in bytes, of a `circuit/authenticate` challenge nonce.
const NONCE_BYTES: usize = 32;

/// Draw a fresh, single-use `circuit/authenticate` challenge nonce from the
/// OS CSPRNG.
pub(super) fn random_nonce() -> Result<[u8; NONCE_BYTES], getrandom::Error> {
    let mut buf = [0u8; NONCE_BYTES];
    getrandom::fill(&mut buf)?;
    Ok(buf)
}

/// Step 2: await the body's `circuit/prove` (the second and last frame of
/// this handshake) within [`PROVE_TIMEOUT`], and return its parsed params.
/// Any shape failure (timeout, wrong method, unparsable params, or a
/// `token_id` that does not match the preceding `circuit/authenticate`) is
/// `-32002` via [`refuse_unauthenticated`] — a peer that has not yet proven
/// anything never learns *which* part of the handshake it got wrong.
pub(super) async fn await_prove<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    token_id: &str,
    peer_ip: &str,
    deps: &AuthDeps<'_>,
) -> Option<(String, holler_proto::Prove)>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let env = match tokio::time::timeout(PROVE_TIMEOUT, next_envelope(stream)).await {
        Ok(Some(env)) => env,
        _ => {
            refuse_unauthenticated(sink, None, "authentication failed: no circuit/prove within the timeout", token_id, peer_ip, deps).await;
            return None;
        }
    };
    let holler_proto::Envelope::Request { id: prove_id, method, params } = &env else {
        refuse_unauthenticated(sink, env.id(), "authentication failed: expected circuit/prove", token_id, peer_ip, deps).await;
        return None;
    };
    if method != "circuit/prove" {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: expected circuit/prove", token_id, peer_ip, deps).await;
        return None;
    }
    let Some(prove) = params.clone().and_then(|v| serde_json::from_value::<holler_proto::Prove>(v).ok()) else {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: malformed circuit/prove params", token_id, peer_ip, deps).await;
        return None;
    };
    if prove.token_id != token_id {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: circuit/prove token_id mismatch", token_id, peer_ip, deps).await;
        return None;
    }
    Some((prove_id.clone(), prove))
}

/// Step 3: verify the body's proof — re-fetch the bound record (catching a
/// revoke/expiry race between the challenge and the proof: the nonce
/// challenge itself carries no authority), rebuild the exact transcript this
/// connection's nonce/hostname/`advertised_url` imply
/// ([`holler_proto::transcript::build`]), and check the Ed25519 signature
/// against the record's registered `body_pubkey`. Success resets this peer's
/// lockout count (issue #184: "a successful auth resets the counter") — the
/// first point in the whole handshake where that is actually earned.
pub(super) async fn finish_prove<Snk>(
    sink: &mut Snk,
    prove_id: &str,
    params: &Authenticate,
    prove: &holler_proto::Prove,
    nonce_hex: &str,
    state: &HubState,
    deps: &AuthDeps<'_>,
) -> Option<crate::token::Record>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let peer_ip = deps.peer.rsplit_once(':').map(|(ip, _)| ip).unwrap_or(deps.peer);
    let record = match crate::token::bound_record_async(&params.token_id, state).await {
        Ok(r) => r,
        Err(e) => {
            refuse_unauthenticated(sink, Some(prove_id), &format!("authentication failed: {e}"), &params.token_id, peer_ip, deps).await;
            return None;
        }
    };
    // `bound_record` only ever returns a record with `Some(body_pubkey)` —
    // `is_none` is unreachable defensively (see that function's own check).
    let Some(pubkey_hex) = record.body_pubkey.clone() else {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: no public key on record", &params.token_id, peer_ip, deps).await;
        return None;
    };
    let transcript = holler_proto::transcript::build(
        holler_proto::PROTOCOL_VERSION,
        "body",
        nonce_hex,
        &params.token_id,
        &params.advertised_url,
    );
    if !verify_signature(&pubkey_hex, &transcript, &prove.signature) {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: bad signature", &params.token_id, peer_ip, deps).await;
        return None;
    }
    if let Ok(ip) = peer_ip.parse() {
        deps.lockout.reset(&ip);
    }
    Some(record)
}

/// Verify an Ed25519 signature (hex) over `transcript` against a hex-encoded
/// public key. `false` on any malformed input (bad hex, wrong length, an
/// invalid curve point) as well as a genuine signature mismatch — every
/// shape of "not a valid proof" is one outcome to the caller.
fn verify_signature(pubkey_hex: &str, transcript: &[u8], signature_hex: &str) -> bool {
    let Ok(pubkey_bytes) = hex::decode(pubkey_hex) else { return false };
    let Ok(pubkey_arr) = <[u8; 32]>::try_from(pubkey_bytes.as_slice()) else { return false };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&pubkey_arr) else { return false };
    let Ok(sig_bytes) = hex::decode(signature_hex) else { return false };
    let Ok(sig_arr) = <[u8; 64]>::try_from(sig_bytes.as_slice()) else { return false };
    let signature = Signature::from_bytes(&sig_arr);
    verifying_key.verify(transcript, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::verify_signature;
    use ed25519_dalek::{Signer, SigningKey};

    fn keypair(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    /// Issue #323's `proof_bound_to_transcript_rejects_role_or_version_
    /// mismatch`: a signature computed over a transcript with a different
    /// protocol version or role than the hub itself reconstructs (always
    /// [`holler_proto::PROTOCOL_VERSION`] and `"body"`) must fail to verify —
    /// the hub never trusts a version/role the peer claims, only the ones it
    /// builds itself.
    #[test]
    fn proof_bound_to_transcript_rejects_role_or_version_mismatch() {
        let signing_key = keypair(42);
        let pubkey_hex = hex::encode(signing_key.verifying_key().to_bytes());
        let real_transcript = holler_proto::transcript::build(2, "body", "deadbeef", "tok_1", "wss://hub");

        // A signature over the *right* transcript verifies.
        let good_sig = signing_key.sign(&real_transcript);
        assert!(verify_signature(&pubkey_hex, &real_transcript, &hex::encode(good_sig.to_bytes())));

        // A signature over a transcript with a different protocol version…
        let wrong_version = holler_proto::transcript::build(3, "body", "deadbeef", "tok_1", "wss://hub");
        let sig_wrong_version = signing_key.sign(&wrong_version);
        assert!(
            !verify_signature(&pubkey_hex, &real_transcript, &hex::encode(sig_wrong_version.to_bytes())),
            "a signature over a mismatched protocol_version must not verify against the real transcript"
        );

        // …or a different role (e.g. a future "hub" signature)…
        let wrong_role = holler_proto::transcript::build(2, "hub", "deadbeef", "tok_1", "wss://hub");
        let sig_wrong_role = signing_key.sign(&wrong_role);
        assert!(
            !verify_signature(&pubkey_hex, &real_transcript, &hex::encode(sig_wrong_role.to_bytes())),
            "a signature over a mismatched role must not verify against the real transcript"
        );
    }

    /// A signature from a keypair other than the one whose public key is on
    /// record never verifies — the base case every other rejection builds on.
    #[test]
    fn wrong_keypair_never_verifies() {
        let registered = keypair(1);
        let attacker = keypair(2);
        let pubkey_hex = hex::encode(registered.verifying_key().to_bytes());
        let transcript = holler_proto::transcript::build(2, "body", "aa", "tok_1", "wss://hub");
        let sig = attacker.sign(&transcript);
        assert!(!verify_signature(&pubkey_hex, &transcript, &hex::encode(sig.to_bytes())));
    }
}
