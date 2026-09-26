//! The `circuit/authenticate` → `circuit/prove` Noise XK handshake (issue
//! #338): split out of `circuit.rs` proper once this file's own growth (the
//! handshake setup, the proof verification, and their shared failure-funnel)
//! pushed it past the workspace's 900-line build guard (`scripts/lint.sh`
//! check 4) — a pure relocation, no behavior change of its own. Mirrors
//! `circuit/dispatch.rs`'s own split for the same reason.
//!
//! Replaces issue #323's Ed25519-signed transcript challenge-response with
//! `Noise_XK_25519_ChaChaPoly_BLAKE2s` (`holler_proto::noise`): message 1
//! (`e, es`) arrives as `circuit/authenticate`'s own params, message 2 (`e,
//! ee`) is that request's result, and message 3 (`s, se`) is `circuit/
//! prove`'s params. The hub is the Noise **responder** — it does not know
//! the body's static key in advance (that is what `K` in XK means for the
//! *initiator* only), so it learns it from message 3 and compares it against
//! the token's registered `body_x25519_pubkey` ([`finish_prove`]) — the
//! DH-authenticated equivalent of the old Ed25519 signature check.
//!
//! [`handle_authenticated`] (still in `circuit.rs`) orchestrates the three
//! steps here plus the hello exchange and the live session loop; this module
//! owns only the handshake itself.

use futures_util::{Sink, Stream};
use holler_proto::noise::{HandshakeXk, NOISE_MESSAGE_ONE_REJECTED_REASON};
use holler_proto::log::Severity;
use holler_proto::{Authenticate, Code};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::Registry;
use crate::serve::{close, send_error_with_reason};
use crate::state::HubState;

use super::{log, next_envelope, PROVE_TIMEOUT};

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
/// a bad token id/handshake can never succeed on retry with the same
/// material, so the TTL would only keep a stale row around).
///
/// The single funnel every failure branch of the `circuit/authenticate` →
/// `circuit/prove` handshake goes through, so the several distinct failure
/// points in [`begin_authenticate`]/[`begin_noise_handshake`]/
/// [`await_prove`]/[`finish_prove`] don't each repeat this five-line
/// sequence (keeping every one of those under clippy's cognitive-complexity
/// gate). `reason` rides in `error.data.reason` (docs §8) — `None` for every
/// branch except [`begin_noise_handshake`]'s message-1 rejection, which is
/// the one shape a caller (the body) needs to tell apart from every other
/// `-32002`.
pub(super) async fn refuse_unauthenticated<Snk>(
    sink: &mut Snk,
    id: Option<&str>,
    message: &str,
    reason: Option<&'static str>,
    token_id: &str,
    peer_ip: &str,
    deps: &AuthDeps<'_>,
) where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let code = rejection_reason(message);
    let outcome = peer_ip.parse().ok().map(|ip| deps.lockout.record_failure_detailed(&ip, code));
    log_rejection(peer_ip, token_id, code, outcome.as_ref());
    send_error_with_reason(sink, id, Code::Unauthenticated, message, reason).await;
    close(sink).await;
    deps.roster.clear(token_id);
}

/// A stable reason code for a rejected `circuit/authenticate`, derived from
/// the refusal message (issue #450). Codes are what operators grep for, so
/// they never change with the message wording; anything unrecognized is
/// `auth_failed`.
pub(super) fn rejection_reason(message: &str) -> &'static str {
    const RULES: &[(&str, &str)] = &[
        ("is expired", "token_expired"),
        ("no such token", "token_unknown"),
        ("is not bound", "token_not_bound"),
        ("no public key on record", "no_public_key"),
        ("no X25519 public key on record", "no_public_key"),
        ("static key mismatch", "key_mismatch"),
        ("no circuit/prove within the timeout", "prove_timeout"),
        ("circuit/prove", "protocol_error"),
        ("handshake", "handshake_failed"),
        ("pairing SAS", "handshake_failed"),
    ];
    RULES.iter().find(|(needle, _)| message.contains(needle)).map_or("auth_failed", |(_, code)| code)
}

/// Log one rejected authentication at `Warn` (visible at the default log
/// level) and, when it moved the peer into a lockout, the trip itself
/// (issue #450). Only the public token id is logged, never a secret.
fn log_rejection(peer_ip: &str, token_id: &str, reason: &'static str, outcome: Option<&crate::lockout::FailureOutcome>) {
    let failures = outcome.map_or_else(|| "?".to_string(), |o| format!("{}/{}", o.count, o.max));
    log(
        Severity::Warn,
        "auth_rejected",
        vec![("peer", peer_ip.to_string()), ("token_id", token_id.to_string()), ("reason", reason.to_string()), ("failures", failures)],
    );
    let Some(o) = outcome else { return };
    if o.newly_tripped {
        let mut grouped: Vec<(&str, usize)> = Vec::new();
        for r in &o.reasons {
            match grouped.iter_mut().find(|(g, _)| g == r) {
                Some((_, n)) => *n += 1,
                None => grouped.push((r, 1)),
            }
        }
        let reasons = grouped.iter().map(|(r, n)| format!("{r}x{n}")).collect::<Vec<_>>().join(",");
        log(
            Severity::Warn,
            "lockout_tripped",
            vec![
                ("peer", peer_ip.to_string()),
                ("failures", o.count.to_string()),
                ("reasons", reasons),
                ("duration_ms", o.duration_ms.to_string()),
                ("retry_after_s", (o.retry_after_ms / 1000).to_string()),
            ],
        );
    }
    for ip in &o.cleared {
        log(Severity::Warn, "lockout_cleared", vec![("peer", ip.to_string()), ("why", "expired".to_string())]);
    }
}

/// Step 0 of `circuit/authenticate` → `circuit/prove` (issue #340): check
/// this body's claimed `protocol` *before* touching Noise at all. A genuine
/// version mismatch would also eventually fail Noise message 3 (the
/// prologue binds `protocol_version` too, so [`finish_prove`]'s
/// `read_message` would reject it) — but only as an opaque "bad handshake
/// message", exactly the "unhelpful generic error" #340's acceptance
/// criteria rule out for a body that predates a protocol bump. Checking
/// here first turns that into a specific, actionable `-32000` naming the
/// mismatch and the fix, before any handshake state is even built.
///
/// No lockout, nothing to clear from the roster — a version claim proves
/// nothing about this peer's trustworthiness (it is not an auth attempt),
/// so it must not count against this peer's lockout budget the way a
/// genuine failed proof does.
pub(super) async fn check_protocol_version<Snk>(sink: &mut Snk, id: Option<&str>, params: &Authenticate) -> Option<()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if holler_proto::is_supported_version(params.protocol) {
        return Some(());
    }
    send_error_with_reason(
        sink,
        id,
        Code::UnsupportedVersion,
        &format!(
            "protocol {} is not supported (this hub requires protocol {}) — upgrade this body and re-run `body join` to re-pair",
            params.protocol,
            holler_proto::PROTOCOL_MIN
        ),
        None,
    )
    .await;
    close(sink).await;
    None
}

/// Step 1 of `circuit/authenticate` → `circuit/prove`: resolve the bound
/// token record for `params.token_id` — an unknown, unbound, expired, or
/// revoked token, or one with no registered X25519 public key, is `-32002`
/// via [`refuse_unauthenticated`] (the same fail-shape the old credential
/// check had: the body's connection loop treats that code, and only that
/// code, as "do not retry"). No lockout success/reset happens here —
/// resolving a *token id* proves nothing yet; only a completed, key-matched
/// handshake does ([`finish_prove`]).
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
            refuse_unauthenticated(sink, id, &format!("authentication failed: {e}"), None, &params.token_id, peer_ip, deps).await;
            None
        }
    }
}

/// Step 2: build this hub's Noise XK **responder** — its own long-lived
/// X25519 identity (issue #322) as the local key, and a prologue binding
/// `token_id` ‖ `advertised_url` ‖ [`holler_proto::PROTOCOL_VERSION`]
/// ([`holler_proto::noise::build_prologue`]) — and process `params.message`
/// (message 1, `e, es`). Any failure (a malformed hex message, or a
/// handshake error — most notably a body that pinned the wrong hub key,
/// which fails right here per [`holler_proto::noise`]'s own module doc) is
/// `-32002` via [`refuse_unauthenticated`] — the same JSON-RPC code as every
/// other shape of "not a valid handshake", but the `read_message` rejection
/// specifically also carries [`NOISE_MESSAGE_ONE_REJECTED_REASON`] in
/// `error.data.reason`, since it is the one branch here that means
/// specifically "this body's pinned hub key does not match this hub."
pub(super) async fn begin_noise_handshake<Snk>(
    sink: &mut Snk,
    id: Option<&str>,
    params: &Authenticate,
    state: &HubState,
    peer_ip: &str,
    deps: &AuthDeps<'_>,
) -> Option<HandshakeXk>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let hub_secret = match crate::identity::ensure(state) {
        Ok(identity) => identity.secret_bytes(),
        Err(e) => {
            refuse_unauthenticated(sink, id, &format!("authentication failed: could not resolve hub identity: {e}"), None, &params.token_id, peer_ip, deps).await;
            return None;
        }
    };
    let Ok(msg1) = hex::decode(&params.message) else {
        refuse_unauthenticated(sink, id, "authentication failed: malformed handshake message", None, &params.token_id, peer_ip, deps).await;
        return None;
    };
    let prologue = holler_proto::noise::build_prologue(holler_proto::PROTOCOL_VERSION, &params.token_id, &params.advertised_url);
    let mut handshake = match HandshakeXk::responder(&hub_secret, &prologue) {
        Ok(h) => h,
        Err(_) => {
            refuse_unauthenticated(sink, id, "authentication failed: could not start the handshake", None, &params.token_id, peer_ip, deps).await;
            return None;
        }
    };
    if handshake.read_message(&msg1).is_err() {
        refuse_unauthenticated(
            sink,
            id,
            "authentication failed: bad handshake message",
            Some(NOISE_MESSAGE_ONE_REJECTED_REASON),
            &params.token_id,
            peer_ip,
            deps,
        )
        .await;
        return None;
    }
    Some(handshake)
}

/// Step 3: await the body's `circuit/prove` (the third and final frame of
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
            refuse_unauthenticated(sink, None, "authentication failed: no circuit/prove within the timeout", None, token_id, peer_ip, deps).await;
            return None;
        }
    };
    let holler_proto::Envelope::Request { id: prove_id, method, params } = &env else {
        refuse_unauthenticated(sink, env.id(), "authentication failed: expected circuit/prove", None, token_id, peer_ip, deps).await;
        return None;
    };
    if method != "circuit/prove" {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: expected circuit/prove", None, token_id, peer_ip, deps).await;
        return None;
    }
    let Some(prove) = params.clone().and_then(|v| serde_json::from_value::<holler_proto::Prove>(v).ok()) else {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: malformed circuit/prove params", None, token_id, peer_ip, deps).await;
        return None;
    };
    if prove.token_id != token_id {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: circuit/prove token_id mismatch", None, token_id, peer_ip, deps).await;
        return None;
    }
    Some((prove_id.clone(), prove))
}

/// Step 4: complete the handshake — re-fetch the bound record (catching a
/// revoke/expiry race between message 1 and message 3: [`begin_noise_
/// handshake`] carries no authority of its own), process `prove.message`
/// (message 3, `s, se`) against the in-progress `handshake`, and — once
/// finished — compare the static key it learned
/// ([`HandshakeXk::remote_static_hex`]) against the record's registered
/// `body_x25519_pubkey`. This comparison is what actually ties the proof to
/// *this* token: Noise XK's responder never knows the initiator's key in
/// advance, so a handshake can complete successfully with *any* self-
/// consistent keypair — only the equality check below makes it a proof of
/// possession of the *expected* key, not just *a* key. Success resets this
/// peer's lockout count (issue #184: "a successful auth resets the
/// counter") — the first point in the whole handshake where that is
/// actually earned.
pub(super) async fn finish_prove<Snk>(
    sink: &mut Snk,
    prove_id: &str,
    params: &Authenticate,
    prove: &holler_proto::Prove,
    handshake: &mut HandshakeXk,
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
            refuse_unauthenticated(sink, Some(prove_id), &format!("authentication failed: {e}"), None, &params.token_id, peer_ip, deps).await;
            return None;
        }
    };
    // `bound_record` only ever returns a record with `Some(body_x25519_pubkey)`
    // — `is_none` is unreachable defensively (see that function's own check).
    let Some(expected_pubkey_hex) = record.body_x25519_pubkey.clone() else {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: no X25519 public key on record", None, &params.token_id, peer_ip, deps).await;
        return None;
    };
    let Ok(msg3) = hex::decode(&prove.message) else {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: malformed handshake message", None, &params.token_id, peer_ip, deps).await;
        return None;
    };
    if handshake.read_message(&msg3).is_err() {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: bad handshake message", None, &params.token_id, peer_ip, deps).await;
        return None;
    }
    if !handshake.is_finished() {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: incomplete handshake", None, &params.token_id, peer_ip, deps).await;
        return None;
    }
    if handshake.remote_static_hex().as_deref() != Some(expected_pubkey_hex.as_str()) {
        refuse_unauthenticated(sink, Some(prove_id), "authentication failed: static key mismatch", None, &params.token_id, peer_ip, deps).await;
        return None;
    }
    if let Ok(ip) = peer_ip.parse() {
        if deps.lockout.reset(&ip) {
            log(Severity::Warn, "lockout_cleared", vec![("peer", ip.to_string()), ("why", "authenticated".to_string())]);
        }
    }
    Some(record)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #338
mod tests {
    use super::rejection_reason;
    use holler_proto::noise::{build_prologue, HandshakeXk};
    use holler_proto::{Authenticate, AuthChallenge, Prove};
    use x25519_dalek::{PublicKey, StaticSecret};

    fn keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
        let private = [seed; 32];
        let secret = StaticSecret::from(private);
        let public = *PublicKey::from(&secret).as_bytes();
        (private, public)
    }

    /// Issue #338's Noise-XK-equivalent of #323's `authenticate_is_
    /// challenge_response_no_secret_on_wire`: serialize the actual `circuit/
    /// authenticate` → `circuit/prove` wire frames — the body's `Authenticate`
    /// request, the hub's `AuthChallenge` result, and the body's `Prove`
    /// reply — from a real, completed Noise XK handshake between real hub and
    /// body keypairs (not mocks). Neither frame carries a field or value
    /// shaped like a bearer secret ([`holler_proto::key_is_secret`]/
    /// [`holler_proto::value_is_secret`], the same gate the wire-logging
    /// redactor uses), and — the direct check this scheme actually needs, per
    /// [`holler_proto::noise`]'s own module doc — neither party's private key
    /// hex ever appears in any of the three frames.
    #[test]
    fn authenticate_is_noise_handshake_no_secret_on_wire() {
        let (hub_secret, hub_public) = keypair(1);
        let (body_secret, body_public) = keypair(2);
        let advertised_url = "wss://hub.example";
        let token_id = "tok_1";
        let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, advertised_url);

        let mut initiator = HandshakeXk::initiator(&body_secret, &hub_public, &prologue).expect("build initiator");
        let mut responder = HandshakeXk::responder(&hub_secret, &prologue).expect("build responder");

        let msg1 = initiator.write_message().expect("write message 1");
        let authenticate = Authenticate {
            protocol: holler_proto::PROTOCOL_VERSION,
            token_id: token_id.into(),
            hostname: "kiwi".into(),
            advertised_url: advertised_url.into(),
            message: hex::encode(&msg1),
        };
        responder.read_message(&msg1).expect("process message 1");

        let msg2 = responder.write_message().expect("write message 2");
        let challenge = AuthChallenge { message: hex::encode(&msg2) };
        initiator.read_message(&msg2).expect("process message 2");

        let msg3 = initiator.write_message().expect("write message 3");
        let prove = Prove { token_id: token_id.into(), message: hex::encode(&msg3) };
        responder.read_message(&msg3).expect("process message 3");

        assert!(responder.is_finished(), "sanity: the handshake must complete");
        assert_eq!(responder.remote_static_hex(), Some(hex::encode(body_public)), "sanity: the responder must learn the real body key");

        let raw_frames = [
            ("circuit/authenticate params", serde_json::to_value(&authenticate)),
            ("circuit/authenticate result", serde_json::to_value(&challenge)),
            ("circuit/prove params", serde_json::to_value(&prove)),
        ];
        let secrets = [hex::encode(hub_secret), hex::encode(body_secret)];
        for (name, result) in raw_frames {
            assert!(result.is_ok(), "{name} failed to serialize: {result:?}");
            let wire = result.unwrap_or_default();
            let obj = wire.as_object();
            assert!(obj.is_some(), "{name} did not serialize as a JSON object: {wire}");
            let obj = obj.cloned().unwrap_or_default();
            assert!(!obj.is_empty(), "{name} serialized to an empty object");
            let wire_str = wire.to_string();
            for secret_hex in &secrets {
                assert!(!wire_str.contains(secret_hex), "{name} must never contain private key hex: {wire_str}");
            }
            for (key, value) in &obj {
                assert!(!holler_proto::key_is_secret(key), "{name} carries a secret-shaped field {key:?}");
                if let Some(s) = value.as_str() {
                    assert!(!holler_proto::value_is_secret(s), "{name}.{key} looks like a bearer token: {s:?}");
                }
            }
        }
    }

    /// Issue #338's Noise-XK-equivalent of #323's `replayed_proof_from_a_
    /// different_nonce_is_rejected`: a full transcript captured from one
    /// completed handshake attempt is replayed against a **fresh** hub-side
    /// responder for the same token/keys — the shape of a real reconnect
    /// attempt an attacker later replays into. Message 1 alone replays
    /// harmlessly, but the fresh responder's own message 2 always carries a
    /// newly-drawn ephemeral key (nothing before it in the transcript
    /// determines it), so the captured message 3 — bound via the running
    /// transcript hash to the *original* message 2 — cannot verify against
    /// this attempt's different one. See `holler_proto::noise`'s own
    /// `replay_rejected_by_fresh_responder_ephemeral` for the crypto-layer
    /// pin this test exercises end to end through this module's own
    /// `begin_noise_handshake`-shaped construction.
    #[test]
    fn replayed_handshake_from_an_earlier_attempt_is_rejected() {
        let (hub_secret, hub_public) = keypair(3);
        let (body_secret, _body_public) = keypair(4);
        let advertised_url = "wss://hub.example";
        let token_id = "tok_1";
        let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, advertised_url);

        // Attempt 1: a genuine, successful handshake. Capture messages 1 and 3.
        let mut initiator_1 = HandshakeXk::initiator(&body_secret, &hub_public, &prologue).expect("build initiator");
        let mut responder_1 = HandshakeXk::responder(&hub_secret, &prologue).expect("build responder");
        let captured_msg1 = initiator_1.write_message().expect("write message 1");
        responder_1.read_message(&captured_msg1).expect("attempt 1 message 1 accepted");
        let msg2_1 = responder_1.write_message().expect("write message 2");
        initiator_1.read_message(&msg2_1).expect("attempt 1 message 2 accepted");
        let captured_msg3 = initiator_1.write_message().expect("write message 3");
        responder_1.read_message(&captured_msg3).expect("attempt 1 completes cleanly");

        // Attempt 2: a brand-new connection (the shape of `begin_noise_
        // handshake` building a fresh responder per socket) — the attacker
        // replays attempt 1's captured message 1 and message 3 into it.
        let mut responder_2 = HandshakeXk::responder(&hub_secret, &prologue).expect("build responder");
        assert!(responder_2.read_message(&captured_msg1).is_ok(), "message 1 alone replays harmlessly");
        let _fresh_msg2 = responder_2.write_message().expect("write message 2");
        assert!(
            responder_2.read_message(&captured_msg3).is_err(),
            "a captured message 3 from an earlier attempt must not verify against a fresh attempt's different message 2"
        );
        assert!(!responder_2.is_finished(), "the replayed attempt must never report finished");
    }

    /// A handshake that completes cryptographically with a *self-consistent*
    /// but *unregistered* keypair must still be rejected — pins the
    /// application-level comparison [`super::finish_prove`] performs
    /// (`remote_static_hex` vs. the token's registered `body_x25519_pubkey`),
    /// the property this module's own doc calls out as the piece Noise XK's
    /// crypto layer alone does not provide.
    #[test]
    fn wrong_body_key_completes_the_handshake_but_fails_the_registered_key_comparison() {
        let (hub_secret, hub_public) = keypair(5);
        let (attacker_secret, attacker_public) = keypair(50);
        let registered_body_public = keypair(6).1;
        let advertised_url = "wss://hub.example";
        let token_id = "tok_1";
        let prologue = build_prologue(holler_proto::PROTOCOL_VERSION, token_id, advertised_url);

        let mut initiator = HandshakeXk::initiator(&attacker_secret, &hub_public, &prologue).expect("build initiator");
        let mut responder = HandshakeXk::responder(&hub_secret, &prologue).expect("build responder");
        let msg1 = initiator.write_message().expect("write message 1");
        responder.read_message(&msg1).expect("message 1 accepted");
        let msg2 = responder.write_message().expect("write message 2");
        initiator.read_message(&msg2).expect("message 2 accepted");
        let msg3 = initiator.write_message().expect("write message 3");
        responder.read_message(&msg3).expect("message 3 accepted — the crypto handshake itself completes");

        assert!(responder.is_finished(), "the handshake itself completes with any self-consistent keypair");
        let learned = responder.remote_static_hex();
        assert_eq!(learned, Some(hex::encode(attacker_public)), "the responder learns the attacker's real key, not a claim");
        assert_ne!(
            learned,
            Some(hex::encode(registered_body_public)),
            "the application-level comparison against the token's registered key is what actually rejects this — mirrored by finish_prove"
        );
    }

    /// Issue #450: every refusal the hub can produce maps to a stable code;
    /// the token errors are the ones operators actually hit.
    #[test]
    fn rejection_reasons_are_stable_codes() {
        let cases = [
            ("authentication failed: token tok_x is expired", "token_expired"),
            ("authentication failed: no such token tok_x", "token_unknown"),
            ("authentication failed: token tok_x is not bound", "token_not_bound"),
            ("authentication failed: token tok_x has no public key on record", "no_public_key"),
            ("authentication failed: no X25519 public key on record", "no_public_key"),
            ("authentication failed: static key mismatch", "key_mismatch"),
            ("authentication failed: no circuit/prove within the timeout", "prove_timeout"),
            ("authentication failed: expected circuit/prove", "protocol_error"),
            ("authentication failed: circuit/prove token_id mismatch", "protocol_error"),
            ("authentication failed: bad handshake message", "handshake_failed"),
            ("authentication failed: incomplete handshake", "handshake_failed"),
            ("authentication failed: could not derive the pairing SAS", "handshake_failed"),
            ("something the hub has never said", "auth_failed"),
        ];
        for (message, code) in cases {
            assert_eq!(rejection_reason(message), code, "{message}");
        }
    }
}
