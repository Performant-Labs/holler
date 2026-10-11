//! Short Authentication String (SAS) derivation (issue #339, part of #321):
//! HKDF-SHA256 over the Noise XK handshake hash `h`
//! ([`crate::noise::HandshakeXk::handshake_hash_hex`]), under the versioned,
//! distinct label [`SAS_LABEL`] — never transmitted, computed independently
//! by both hub and body from their own copy of the (identical, once the
//! handshake completes) handshake hash.
//!
//! ## What this closes
//!
//! Per #321's "SAS — and the direction it actually protects": the pinned hub
//! static key already gives body→hub authenticity, but the hub has no way to
//! tell *which* body redeemed a join token — if the join line leaked before
//! the intended body used it, an attacker's body could complete `circuit/
//! join` first. Both sides displaying this SAS lets the operator, standing at
//! both machines during pairing, confirm by eye that they completed the
//! *same* handshake with each other, closing that gap.
//!
//! ## Domain separation
//!
//! Per #321's own domain-separation principle (this issue's acceptance
//! criterion, not a separate one): [`SAS_LABEL`] is this derivation's only
//! HKDF `info` parameter, distinct and versioned, so a future derived-key
//! operation over the same handshake hash can never collide with this one by
//! construction — the same discipline #321's design doc calls out for
//! iTerm2's own `relay-auth-ed25519` vs `iterm2-relay-delete` labels.
//! [`hkdf_expand_labeled`] takes the label as a parameter specifically so
//! [`label_changes_the_derived_output`] can prove this at the HKDF layer
//! itself, not just by inspecting the constant.
//!
//! ## Format
//!
//! 6 decimal digits, rendered `NNN-NNN` (e.g. `482-019`) — short enough to
//! read aloud or type without error, familiar to any operator who has ever
//! compared a TOTP/2FA code, and (`u32 % 1_000_000` over 4 HKDF-output bytes)
//! close to the full ~4-billion range's uniformity: the bias from the modulo
//! is under 1 in 4 billion per bucket, negligible next to the ~1-in-a-million
//! collision floor 6 digits themselves already set. That floor is the
//! relevant one for what a SAS defends against here: an operator manually
//! comparing two short strings once, at pairing time, not a value meant to
//! resist offline brute force the way a key does.

use hkdf::Hkdf;
use sha2::Sha256;

/// This derivation's HKDF `info` label. Exact value pinned by issue #339's
/// own acceptance criterion; [`derive_sas`] uses no other label, ever.
pub const SAS_LABEL: &[u8] = b"holler-sas-v1";

/// HKDF output length, in bytes, before formatting: 4 bytes → a `u32` → a
/// 6-digit decimal SAS (see this module's own doc for the format rationale).
const SAS_OKM_LEN: usize = 4;

/// A fail-closed refusal: a malformed `handshake_hash_hex` (not valid hex),
/// or an internal HKDF expand error. `hkdf`'s own `expand` only fails when
/// the requested output length exceeds its hash's limit
/// (255 × 32 bytes for SHA-256) — unreachable at [`SAS_OKM_LEN`], but still
/// surfaced as `Err` rather than `unwrap`/`expect` (workspace lint: both are
/// `deny`), the same fail-closed discipline [`crate::noise::NoiseError`]
/// uses for its own "structurally near-impossible but still not a panic"
/// failure paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SasError {
    pub message: String,
}

impl SasError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for SasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SasError {}

/// HKDF-SHA256(salt=None, ikm) → `expand(label, out_len)`. Private, and
/// generic over `label` (rather than hardcoding [`SAS_LABEL`] inline) purely
/// so this module's own tests can call it with a *different* label to prove
/// domain separation at the HKDF layer — no production caller outside
/// [`derive_sas`] ever exists, since a hardcoded-label public fn is the whole
/// point: nothing outside this module can accidentally derive under the
/// wrong label.
fn hkdf_expand_labeled(ikm: &[u8], label: &[u8], out_len: usize) -> Result<Vec<u8>, SasError> {
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut okm = vec![0u8; out_len];
    hk.expand(label, &mut okm)
        .map_err(|e| SasError::new(format!("hkdf expand failed: {e}")))?;
    Ok(okm)
}

/// Derive the pairing SAS from a completed Noise XK handshake's hash
/// (`handshake_hash_hex`, i.e.
/// [`crate::noise::HandshakeXk::handshake_hash_hex`]'s return value on
/// either side — identical on both once [`crate::noise::HandshakeXk::
/// is_finished`]). Never transmitted: both hub and body call this on their
/// own local handshake state and compare the *result* out of band (by eye).
///
/// `Err` only for a malformed `handshake_hash_hex` (not valid hex) — never
/// reachable from a real [`crate::noise::HandshakeXk`], whose
/// `handshake_hash_hex` is always `hex::encode` of a real hash, but a caller
/// could in principle hand this fn any string.
pub fn derive_sas(handshake_hash_hex: &str) -> Result<String, SasError> {
    let ikm = hex::decode(handshake_hash_hex).map_err(|e| SasError::new(format!("malformed handshake hash hex: {e}")))?;
    let okm = hkdf_expand_labeled(&ikm, SAS_LABEL, SAS_OKM_LEN)?;
    let n = u32::from_be_bytes([okm[0], okm[1], okm[2], okm[3]]) % 1_000_000;
    Ok(format!("{:03}-{:03}", n / 1000, n % 1000))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #339
mod tests {
    use super::*;
    use crate::noise::{build_prologue, HandshakeXk};
    use x25519_dalek::{PublicKey, StaticSecret};

    /// A throwaway X25519 keypair for tests — mirrors `noise`'s own
    /// test-only `keypair` helper (private to that module's test mod, so
    /// duplicated here rather than shared, the same convention `holler-hub`'s
    /// `circuit/auth.rs` tests already use for their own copy).
    fn keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
        let private = [seed; 32];
        let secret = StaticSecret::from(private);
        let public = *PublicKey::from(&secret).as_bytes();
        (private, public)
    }

    /// Run a full 3-message Noise XK handshake between fresh `hub`/`body`
    /// keypairs, returning both sides' finished handshake state — mirrors
    /// `noise`'s own private `run_handshake` test helper.
    fn run_handshake(hub: ([u8; 32], [u8; 32]), body: ([u8; 32], [u8; 32]), prologue: &[u8]) -> (HandshakeXk, HandshakeXk) {
        let mut initiator = HandshakeXk::initiator(&body.0, &hub.1, prologue).expect("build initiator");
        let mut responder = HandshakeXk::responder(&hub.0, prologue).expect("build responder");

        let msg1 = initiator.write_message().expect("write message 1");
        responder.read_message(&msg1).expect("process message 1");
        let msg2 = responder.write_message().expect("write message 2");
        initiator.read_message(&msg2).expect("process message 2");
        let msg3 = initiator.write_message().expect("write message 3");
        responder.read_message(&msg3).expect("process message 3");

        (initiator, responder)
    }

    /// Acceptance: "Both hub and body independently derive the same SAS from
    /// a completed handshake (never transmitted)." Runs one real handshake,
    /// derives the SAS from each side's own `handshake_hash_hex`
    /// independently, and checks they match — the actual property the
    /// operator's by-eye comparison depends on.
    #[test]
    fn same_handshake_both_sides_derive_identical_sas() {
        let hub = keypair(1);
        let body = keypair(2);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");
        let (initiator, responder) = run_handshake(hub, body, &prologue);

        assert!(initiator.is_finished(), "sanity: handshake must complete");
        assert!(responder.is_finished(), "sanity: handshake must complete");
        assert_eq!(initiator.handshake_hash_hex(), responder.handshake_hash_hex(), "sanity: hashes match on both sides");

        let sas_body_side = derive_sas(&initiator.handshake_hash_hex()).expect("derive on the body's own handshake state");
        let sas_hub_side = derive_sas(&responder.handshake_hash_hex()).expect("derive on the hub's own handshake state");
        assert_eq!(sas_body_side, sas_hub_side, "both sides must derive the identical SAS from a completed handshake");
    }

    /// Acceptance: "SAS uses a versioned, distinct HKDF label
    /// (`holler-sas-v1`)." Pins the exact constant value — a rename or a
    /// version bump here is a deliberate, reviewed decision, not a silent
    /// drift, since every future consumer keys off this exact string.
    #[test]
    fn sas_label_is_exactly_holler_sas_v1() {
        assert_eq!(SAS_LABEL, b"holler-sas-v1");
    }

    /// Acceptance: "Two handshakes with different static keys ... produce
    /// different SAS values." Same prologue, disjoint keypairs on both sides.
    #[test]
    fn different_static_keys_produce_different_sas() {
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");
        let (_, responder_a) = run_handshake(keypair(3), keypair(4), &prologue);
        let (_, responder_b) = run_handshake(keypair(5), keypair(6), &prologue);

        let sas_a = derive_sas(&responder_a.handshake_hash_hex()).expect("derive a");
        let sas_b = derive_sas(&responder_b.handshake_hash_hex()).expect("derive b");
        assert_ne!(sas_a, sas_b, "different static keys must produce different SAS values");
    }

    /// Acceptance: "Two handshakes with different ... nonces produce
    /// different SAS values." This handshake pattern has no explicit
    /// caller-supplied nonce field; the equivalent freshness is `snow`'s own
    /// CSPRNG-drawn ephemeral key per `write_message` call (see `noise`'s own
    /// module doc, "Replay: no extra nonce needed") — so two handshakes
    /// between the *same* static keypairs, run independently, already differ
    /// exactly the way two differently-nonced handshakes would: the
    /// handshake hash folds in both ephemeral keys, which are never reused.
    #[test]
    fn different_ephemeral_nonces_produce_different_sas() {
        let hub = keypair(7);
        let body = keypair(8);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");

        let (_, responder_1) = run_handshake(hub, body, &prologue);
        let (_, responder_2) = run_handshake(hub, body, &prologue);

        assert_ne!(
            responder_1.handshake_hash_hex(),
            responder_2.handshake_hash_hex(),
            "sanity: two independent handshakes between the same static keys draw different ephemerals"
        );
        let sas_1 = derive_sas(&responder_1.handshake_hash_hex()).expect("derive 1");
        let sas_2 = derive_sas(&responder_2.handshake_hash_hex()).expect("derive 2");
        assert_ne!(sas_1, sas_2, "different ephemeral keys (the nonce-equivalent freshness) must produce different SAS values");
    }

    /// Acceptance (#321's domain-separation principle, this issue's own
    /// criterion): changing the HKDF label changes the derived output, so a
    /// future derived-key operation over the same handshake hash can never
    /// collide with this one by construction. Exercises the HKDF layer
    /// directly via [`hkdf_expand_labeled`] (the fn [`derive_sas`] itself
    /// calls), rather than just asserting on the [`SAS_LABEL`] constant.
    #[test]
    fn label_changes_the_derived_output() {
        let hub = keypair(9);
        let body = keypair(10);
        let prologue = build_prologue(2, "tok_1", "wss://hub.example");
        let (_, responder) = run_handshake(hub, body, &prologue);
        let ikm = hex::decode(responder.handshake_hash_hex()).expect("valid hex handshake hash");

        let under_sas_label = hkdf_expand_labeled(&ikm, SAS_LABEL, SAS_OKM_LEN).expect("expand under holler-sas-v1");
        let under_other_label = hkdf_expand_labeled(&ikm, b"holler-some-future-derivation-v1", SAS_OKM_LEN).expect("expand under a different label");
        assert_ne!(under_sas_label, under_other_label, "a different HKDF label must change the derived output");

        // And derive_sas itself is exactly this call under SAS_LABEL —
        // proven by formatting under_sas_label the same way derive_sas does
        // and comparing to derive_sas's own result, so this test would fail
        // if derive_sas ever drifted from hkdf_expand_labeled(_, SAS_LABEL, _).
        let n = u32::from_be_bytes([under_sas_label[0], under_sas_label[1], under_sas_label[2], under_sas_label[3]]) % 1_000_000;
        let expected = format!("{:03}-{:03}", n / 1000, n % 1000);
        assert_eq!(derive_sas(&responder.handshake_hash_hex()).expect("derive"), expected);
    }

    /// A malformed (non-hex) handshake hash is a fail-closed `Err`, never a
    /// panic — this module's own lints deny `unwrap`/`expect`/`panic` in
    /// non-test code, so this pins the boundary check actually fires.
    #[test]
    fn malformed_handshake_hash_hex_is_rejected() {
        assert!(derive_sas("not valid hex").is_err());
    }
}
