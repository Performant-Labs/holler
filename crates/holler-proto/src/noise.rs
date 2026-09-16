//! The Noise XK handshake core (issue #338): `Noise_XK_25519_ChaChaPoly_
//! BLAKE2s` (3-message: `e,es` / `e,ee` / `s,se`), implemented on top of the
//! `snow` crate rather than hand-rolled — this module's whole job is to keep
//! `snow` itself an implementation detail nobody outside this file ever
//! touches, so `holler-hub`/`holler-body` only ever hand it raw key bytes
//! and hex-encoded wire messages.
//!
//! Wired into `circuit/authenticate` → `circuit/prove` in place of the old
//! transcript-signing challenge-response (issue #323): that scheme proved
//! possession of an Ed25519 signing key over a hub-issued nonce, but never
//! encrypted anything and never involved the hub's own identity in the
//! handshake itself (hub-key pinning was checked only afterward, at
//! `circuit/hello`). Noise XK subsumes both properties at once: the `K` in
//! XK means the **initiator already knows the responder's static public
//! key** — here, the body already has the hub's pinned key from `body join`
//! (issue #322) — so a body dialing a hub whose real private key does not
//! match what it pinned can never complete message 1 at all, regardless of
//! what `circuit/hello` would have said later. And the responder only learns
//! the initiator's static key from message 3's encrypted `s` — which is
//! exactly the body's X25519 identity registered at `circuit/join` (issue
//! #337) — so a genuine DH-authenticated proof of possession replaces the
//! Ed25519 signature.
//!
//! ## Replay: no extra nonce needed
//!
//! The old scheme's freshness came from a hub-minted nonce the body had to
//! sign; it might look like Noise XK needs an equivalent (see #321's design
//! doc, which lists "nonce" among a *future* transcript's binding fields).
//! It does not, for messages 1–3 specifically: every real connection attempt
//! builds a **fresh** [`HandshakeXk`] on both sides, and the responder's own
//! message 2 always carries a **freshly drawn ephemeral key** (`e`) that
//! nothing before it in the transcript determines — not something an
//! attacker can make the live hub re-emit from a captured prior session. A
//! captured message 3 is bound (via Noise's running transcript hash) to
//! *that* message 2, so replaying it against a fresh responder — whose
//! message 2 necessarily differs — fails to decrypt
//! ([`replay_rejected_by_fresh_responder_ephemeral`]). No captured triple
//! from one connection attempt is ever valid against another.
//!
//! ## What this module does not do (yet)
//!
//! It stops at a completed handshake — [`HandshakeXk::is_finished`],
//! [`HandshakeXk::remote_static_hex`], [`HandshakeXk::handshake_hash_hex`].
//! It does not expose a transport/session type: nothing in issue #338's
//! scope encrypts traffic *after* the handshake (that is future work), so
//! there is no caller for one yet. [`HandshakeXk::handshake_hash_hex`] is
//! exposed now specifically so #339's SAS derivation is an incremental
//! addition here later, not a redesign — the same discipline issues #322 and
//! #337 used landing the X25519 identities themselves ahead of this module.

use std::fmt;

/// The exact Noise protocol name this module implements. Fixed — never
/// negotiated, never read from the wire (a peer does not get to choose the
/// handshake pattern or ciphers; only [`crate::PROTOCOL_VERSION`] governs
/// compatibility).
pub const NOISE_PARAMS_STR: &str = "Noise_XK_25519_ChaChaPoly_BLAKE2s";

/// The length, in bytes, of an X25519 private or public key.
const KEY_LEN: usize = 32;

/// A scratch buffer large enough for any message this pattern produces with
/// an empty payload (measured: message 1 and 2 are 48 bytes, message 3 is 64
/// — the `e`/`ee`/`es`/`se` tokens plus one AEAD tag each), with generous
/// headroom.
const MAX_MESSAGE_LEN: usize = 256;

/// A fail-closed refusal from any step of the handshake: a malformed
/// prologue/key input, a decrypt/DH failure (the wrong static key, a
/// mismatched prologue, or a replayed/corrupted message), or an internal
/// state error. Every variant of the underlying `snow::Error` this wraps is
/// itself just an enum tag with no payload — never key material — so
/// formatting it into `message` cannot leak anything across the module
/// boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseError {
    pub message: String,
}

impl NoiseError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NoiseError {}

impl From<snow::Error> for NoiseError {
    fn from(e: snow::Error) -> Self {
        NoiseError::new(format!("noise handshake error: {e:?}"))
    }
}

fn params() -> Result<snow::params::NoiseParams, NoiseError> {
    NOISE_PARAMS_STR
        .parse()
        .map_err(|e: snow::Error| NoiseError::new(format!("bad noise params {NOISE_PARAMS_STR:?}: {e:?}")))
}

/// Build the Noise XK prologue: `"holler-noise-xk-v1" ‖ protocol_version ‖
/// token_id ‖ advertised_url`, `|`-joined. Mixed into the handshake hash on
/// both sides before message 1 — a captured handshake cannot be replayed
/// into a different token/endpoint/version context, because any difference
/// here makes the two sides' transcript hashes diverge from the very first
/// message ([`prologue_binding_rejects_a_different_context`]).
///
/// Mirrors the domain-separation discipline the #323 transcript builder this
/// module replaces used to apply (a distinct leading tag, `|`-joined fields,
/// the hub's own [`crate::PROTOCOL_VERSION`] never a value the peer claims)
/// for the property that used to live there.
pub fn build_prologue(protocol_version: u32, token_id: &str, advertised_url: &str) -> Vec<u8> {
    format!("holler-noise-xk-v1|{protocol_version}|{token_id}|{advertised_url}").into_bytes()
}

/// One party's live Noise XK handshake state, mid-handshake. Wraps
/// `snow::HandshakeState` so nothing outside this module ever names a `snow`
/// type.
pub struct HandshakeXk {
    inner: snow::HandshakeState,
}

impl HandshakeXk {
    /// Build the **initiator** side — the body: per XK's own `K`, it already
    /// knows the responder's (hub's) static public key, pinned at `body
    /// join` (issue #322).
    pub fn initiator(local_private: &[u8; KEY_LEN], remote_public: &[u8; KEY_LEN], prologue: &[u8]) -> Result<Self, NoiseError> {
        let inner = snow::Builder::new(params()?)
            .local_private_key(local_private)?
            .remote_public_key(remote_public)?
            .prologue(prologue)?
            .build_initiator()?;
        Ok(Self { inner })
    }

    /// Build the **responder** side — the hub. Unlike the initiator, it does
    /// not know the body's static key in advance (that asymmetry is the
    /// whole point of `K`): [`Self::remote_static_hex`] is `None` until
    /// message 3 completes.
    pub fn responder(local_private: &[u8; KEY_LEN], prologue: &[u8]) -> Result<Self, NoiseError> {
        let inner = snow::Builder::new(params()?)
            .local_private_key(local_private)?
            .prologue(prologue)?
            .build_responder()?;
        Ok(Self { inner })
    }

    /// Write this side's next handshake message (no application payload —
    /// issue #338's wiring carries no data beyond the handshake itself).
    /// `Err` only if it is not this side's turn or the state is already
    /// finished (a caller bug, never a peer's fault).
    pub fn write_message(&mut self) -> Result<Vec<u8>, NoiseError> {
        let mut buf = [0u8; MAX_MESSAGE_LEN];
        let len = self.inner.write_message(&[], &mut buf)?;
        Ok(buf[..len].to_vec())
    }

    /// Read the peer's next handshake message. `Err` on any failure shape —
    /// a wrong static key, a mismatched prologue, a corrupted or replayed
    /// message, or simply too short/long to be this pattern's next message —
    /// collapsed into one outcome, the same "every shape of not-a-valid-
    /// proof is one outcome to the caller" discipline the old
    /// `verify_signature` used.
    pub fn read_message(&mut self, message: &[u8]) -> Result<(), NoiseError> {
        let mut payload = [0u8; MAX_MESSAGE_LEN];
        self.inner.read_message(message, &mut payload)?;
        Ok(())
    }

    /// Whether all 3 messages have been exchanged (`is_handshake_finished`).
    pub fn is_finished(&self) -> bool {
        self.inner.is_handshake_finished()
    }

    /// The peer's static public key, hex-encoded, if learned yet. Always
    /// `Some` for the initiator (it had to supply this to build in the first
    /// place); `None` for the responder until message 3 completes.
    pub fn remote_static_hex(&self) -> Option<String> {
        self.inner.get_remote_static().map(hex::encode)
    }

    /// The handshake hash, hex-encoded — identical on both sides once
    /// finished, and the input a future SAS derivation (#339) would HKDF
    /// under a versioned label. Meaningful only after at least one message
    /// has been processed; this module's own callers only ever read it once
    /// [`Self::is_finished`] is `true`.
    pub fn handshake_hash_hex(&self) -> String {
        hex::encode(self.inner.get_handshake_hash())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #338
mod tests {
    use super::*;
    use x25519_dalek::{PublicKey, StaticSecret};

    /// A throwaway X25519 keypair for tests — any 32 bytes are a valid
    /// X25519 private scalar (RFC 7748 clamping happens internally), so a
    /// fixed-seed byte pattern is deterministic and sufficient; production
    /// code always uses a CSPRNG-drawn key instead (`holler_hub::identity`,
    /// `holler_body::x25519_identity`). The public half is derived via
    /// `x25519-dalek` — proven interoperable with `snow`'s own resolver
    /// during this issue's design verification (byte-for-byte identical
    /// public keys from the same private bytes, since both implement plain
    /// RFC 7748 X25519).
    fn keypair(seed: u8) -> ([u8; KEY_LEN], [u8; KEY_LEN]) {
        let private = [seed; KEY_LEN];
        let secret = StaticSecret::from(private);
        let public = *PublicKey::from(&secret).as_bytes();
        (private, public)
    }

    /// Run a full 3-message handshake between a fresh initiator and
    /// responder built from `hub`/`body` keypairs and `prologue`.
    fn run_handshake(
        hub: ([u8; KEY_LEN], [u8; KEY_LEN]),
        body: ([u8; KEY_LEN], [u8; KEY_LEN]),
        init_prologue: &[u8],
        resp_prologue: &[u8],
    ) -> Result<(HandshakeXk, HandshakeXk), NoiseError> {
        let mut initiator = HandshakeXk::initiator(&body.0, &hub.1, init_prologue)?;
        let mut responder = HandshakeXk::responder(&hub.0, resp_prologue)?;

        let msg1 = initiator.write_message()?;
        responder.read_message(&msg1)?;

        let msg2 = responder.write_message()?;
        initiator.read_message(&msg2)?;

        let msg3 = initiator.write_message()?;
        responder.read_message(&msg3)?;

        Ok((initiator, responder))
    }

    /// Acceptance: "Handshake completes successfully between two parties
    /// holding each other's static public keys."
    #[test]
    fn handshake_completes_between_two_real_keypairs() {
        let hub = keypair(1);
        let body = keypair(2);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");

        let (initiator, responder) = run_handshake(hub, body, &prologue, &prologue).expect("a matched handshake must succeed");

        assert!(initiator.is_finished(), "initiator must report finished");
        assert!(responder.is_finished(), "responder must report finished");

        // The responder learns the body's real static public key from
        // message 3 — this is the DH-authenticated proof of possession that
        // replaces the old Ed25519 signature.
        let expected_body_pub = hex::encode(body.1);
        assert_eq!(responder.remote_static_hex(), Some(expected_body_pub), "responder must learn the body's real static key");

        // The handshake hash matches on both sides (the SAS-derivation input, #339).
        assert_eq!(initiator.handshake_hash_hex(), responder.handshake_hash_hex(), "handshake hash must match on both sides");
        assert_eq!(initiator.handshake_hash_hex().len(), 64, "BLAKE2s hash is 32 bytes = 64 hex chars");
    }

    /// Acceptance: "Handshake fails closed on a wrong/missing static key" —
    /// the **hub**-identity half: a body that pins the wrong hub public key
    /// (a stale pin, or an impersonator) can never complete message 1, on
    /// EITHER the impersonator (who does not hold the pinned key's private
    /// half) or the real hub (whose real key does not match what this body
    /// pinned) — no prompt, no trust-on-first-use, fails at the very first
    /// message.
    #[test]
    fn wrong_hub_key_rejected_at_message_one() {
        let real_hub = keypair(3);
        let wrong_hub = keypair(30);
        let body = keypair(4);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");

        let mut initiator = HandshakeXk::initiator(&body.0, &wrong_hub.1, &prologue).expect("building the initiator itself cannot fail here");
        let mut responder = HandshakeXk::responder(&real_hub.0, &prologue).expect("building the responder itself cannot fail here");

        let msg1 = initiator.write_message().expect("writing message 1 itself cannot fail here");
        let result = responder.read_message(&msg1);
        assert!(result.is_err(), "a body pinning the wrong hub key must fail at message 1, not silently proceed");
    }

    /// Acceptance: "Handshake fails closed on a wrong/missing static key" —
    /// the **body**-identity half. The Noise handshake itself only proves
    /// "this initiator holds *some* X25519 private key matching the static
    /// public key it presented in message 3" — it has no notion of which
    /// key was *expected* for a given token, since XK's responder never
    /// knows the initiator's key in advance. That comparison is the caller's
    /// job ([`crate::circuit`]'s hub-side wiring, not this module): a
    /// handshake between a real hub and *some* body's real (self-consistent)
    /// keypair completes and reports a real, verifiable `remote_static_hex`
    /// — an attacker with no matching private key at all cannot produce
    /// message 3, which the crypto-layer test above already covers from the
    /// symmetric case. This test pins the contract precisely:
    /// `remote_static_hex` always reflects the *actual* key used, never a
    /// value the initiator merely claims — so an application-level `!=`
    /// comparison against the token's registered key is meaningful.
    #[test]
    fn remote_static_hex_reflects_the_key_actually_used_not_a_claim() {
        let hub = keypair(5);
        let real_body = keypair(6);
        let different_body = keypair(60);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");

        let (_, responder) = run_handshake(hub, real_body, &prologue, &prologue).expect("a matched handshake must succeed — pinned by the happy-path test above");
        let learned = responder.remote_static_hex();
        assert_eq!(learned, Some(hex::encode(real_body.1)), "must equal the real body's actual public key");
        assert_ne!(learned, Some(hex::encode(different_body.1)), "must not equal an unrelated body's public key — a caller's registered-key comparison is meaningful");
    }

    /// Acceptance: "Prologue includes token_id, advertised URL, protocol
    /// version; a handshake replayed against a different token/URL/version
    /// is rejected." Each field differing independently must break the
    /// handshake — mirrors `holler_proto::transcript`'s own
    /// `differs_on_every_field` test for the scheme this replaces.
    #[test]
    fn prologue_binding_rejects_a_different_context() {
        let hub = keypair(7);
        let body = keypair(8);
        let base = build_prologue(2, "tok_1", "wss://hub.example");

        let cases: [(&str, Vec<u8>); 3] = [
            ("protocol_version", build_prologue(3, "tok_1", "wss://hub.example")),
            ("token_id", build_prologue(2, "tok_2", "wss://hub.example")),
            ("advertised_url", build_prologue(2, "tok_1", "wss://other.example")),
        ];
        for (field, other_prologue) in cases {
            // The initiator built its prologue over one context; the
            // responder was told a different one — models a captured
            // message 1 replayed into a different token/endpoint/version.
            let result = run_handshake(hub, body, &base, &other_prologue);
            assert!(result.is_err(), "a mismatched {field} in the prologue must reject the handshake, not silently accept it");
        }

        // Sanity: the identical prologue on both sides still succeeds (rules
        // out "always rejects" as a false-positive explanation above).
        assert!(run_handshake(hub, body, &base, &base).is_ok(), "sanity: a matched prologue must still succeed");
    }

    /// Acceptance: "Existing #323 acceptance tests (no secret on wire,
    /// replay rejected) still pass or have a Noise-XK-equivalent
    /// replacement that asserts the same properties" — the replay half. A
    /// full transcript captured from one completed handshake attempt (all 3
    /// messages) is replayed, message by message, against a **fresh**
    /// responder for the same token/keys — modeling an attacker who
    /// intercepted a real body's real handshake and later replays it against
    /// a new connection attempt. Message 1 alone replays harmlessly (it
    /// proves nothing by itself), but the fresh responder's own message 2
    /// always carries a newly-drawn ephemeral key, so the captured message 3
    /// — bound via the running transcript hash to the *original* message 2 —
    /// cannot possibly be valid against this attempt's different message 2.
    #[test]
    fn replay_rejected_by_fresh_responder_ephemeral() {
        let hub = keypair(9);
        let body = keypair(10);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");

        // Attempt 1: a genuine, successful handshake. Capture messages 1 and 3.
        let mut initiator_1 = HandshakeXk::initiator(&body.0, &hub.1, &prologue).expect("building the initiator cannot fail here");
        let mut responder_1 = HandshakeXk::responder(&hub.0, &prologue).expect("building the responder cannot fail here");
        let captured_msg1 = initiator_1.write_message().expect("write message 1");
        assert!(responder_1.read_message(&captured_msg1).is_ok(), "sanity: attempt 1's message 1 must be accepted");
        let msg2_1 = responder_1.write_message().expect("write message 2");
        assert!(initiator_1.read_message(&msg2_1).is_ok(), "sanity: attempt 1's message 2 must be accepted");
        let captured_msg3 = initiator_1.write_message().expect("write message 3");
        assert!(responder_1.read_message(&captured_msg3).is_ok(), "sanity: attempt 1 must complete cleanly");
        assert!(responder_1.is_finished(), "sanity: attempt 1 must finish");

        // Attempt 2: a brand-new connection (fresh responder state), same
        // token/keys/prologue — the shape of a real reconnect. The attacker
        // replays attempt 1's captured message 1 into it.
        let mut responder_2 = HandshakeXk::responder(&hub.0, &prologue).expect("building the responder cannot fail here");
        let msg1_replay = responder_2.read_message(&captured_msg1);
        assert!(msg1_replay.is_ok(), "message 1 alone replays harmlessly — it proves nothing by itself yet");

        // The fresh responder writes its OWN message 2 (a live hub always
        // does this — an attacker cannot make it reuse attempt 1's message
        // 2). Now replay attempt 1's captured message 3 against it.
        let _fresh_msg2 = responder_2.write_message().expect("write message 2");
        let msg3_replay = responder_2.read_message(&captured_msg3);
        assert!(msg3_replay.is_err(), "a captured message 3 from an earlier attempt must not verify against a fresh attempt's different message 2");
        assert!(!responder_2.is_finished(), "the replayed attempt must never report finished");
    }

    /// Acceptance: "No key material (static or ephemeral private) ever
    /// leaves the module boundary in cleartext logs/errors." Drives every
    /// failure path this module exposes (wrong hub key, mismatched
    /// prologue, replayed message) and asserts neither party's private key
    /// hex ever appears in the resulting `NoiseError`'s message — the
    /// `snow::Error` this wraps is itself just enum tags with no key
    /// material (verified by inspecting `snow`'s own `Error` definition),
    /// but this test pins the property at this module's actual boundary
    /// rather than by inspection alone, mirroring
    /// `hub_private_key_never_appears_in_noisy_debug_output`'s discipline.
    #[test]
    fn private_key_material_never_appears_in_a_noise_error_message() {
        let hub = keypair(11);
        let wrong_hub = keypair(110);
        let body = keypair(12);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");
        let secrets = [hex::encode(hub.0), hex::encode(wrong_hub.0), hex::encode(body.0)];

        // Failure 1: wrong hub key at message 1.
        let mut bad_initiator = HandshakeXk::initiator(&body.0, &wrong_hub.1, &prologue).expect("building the initiator itself cannot fail here");
        let mut responder = HandshakeXk::responder(&hub.0, &prologue).expect("building the responder itself cannot fail here");
        let msg1 = bad_initiator.write_message().expect("write message 1");
        let failure_1 = responder.read_message(&msg1);
        assert!(failure_1.is_err(), "this case must fail — pinned by wrong_hub_key_rejected_at_message_one above");
        if let Err(e) = failure_1 {
            for secret_hex in &secrets {
                assert!(!e.message.contains(secret_hex), "a NoiseError must never contain private key hex: {}", e.message);
            }
        }

        // Failure 2: mismatched prologue.
        let other_prologue = build_prologue(2, "tok_1", "wss://other.example");
        let failure_2 = run_handshake(hub, body, &prologue, &other_prologue);
        assert!(failure_2.is_err(), "this case must fail — pinned by prologue_binding_rejects_a_different_context above");
        if let Err(e) = failure_2 {
            for secret_hex in &secrets {
                assert!(!e.message.contains(secret_hex), "a NoiseError must never contain private key hex: {}", e.message);
            }
        }
    }
}
