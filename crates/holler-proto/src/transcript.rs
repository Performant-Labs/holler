//! The `circuit/authenticate` → `circuit/prove` proof transcript (issue
//! #323): the exact bytes a body's Ed25519 signature covers.
//!
//! The hub issues a fresh, single-use `nonce` in its `circuit/authenticate`
//! reply (a challenge); the body answers `circuit/prove` with a signature
//! over this transcript. Both sides build it with this one function so they
//! can never drift: the hub reconstructs it from the values *it* generated
//! or received on this connection (never trusting a value the body repeats
//! back in the `circuit/prove` frame itself, `token_id` aside), and the body
//! builds the identical bytes to sign.
//!
//! Binding `protocol_version` and `role` (docs #323's "Why not a symmetric
//! HMAC challenge") means a proof cannot be replayed as if it authenticated a
//! different protocol version or the *other* role — the same asymmetric
//! primitive Noise XK ([#321](https://github.com/Performant-Labs/holler/issues/321))
//! will need, so this shape is not thrown away once that lands. Binding
//! `advertised_url` means a proof captured on one connection cannot be
//! replayed to convince a *different* endpoint it is talking to an
//! authenticated body, even before Noise gives the wire itself channel
//! binding. Because the hub always mints a fresh `nonce` per connection
//! attempt and never persists or reuses one, a captured signature is also
//! useless against any *other* attempt: [`build`]'s output differs the
//! moment the nonce differs.
//!
//! The leading domain-separation tag (`"holler-auth-v1"`) keeps this
//! signature scheme from ever colliding with a signature over the same bytes
//! minted for an unrelated purpose (standard signature hygiene).

/// Build the transcript a body's `circuit/prove` signature must cover:
/// `"holler-auth-v1" ‖ protocol_version ‖ role ‖ nonce ‖ token_id ‖
/// advertised_url`, `|`-joined. `nonce` is the hub's hex-encoded challenge;
/// `role` is always `"body"` in v2 (the hub never signs anything here — see
/// ADR 0007/0008's amendment).
pub fn build(protocol_version: u32, role: &str, nonce_hex: &str, token_id: &str, advertised_url: &str) -> Vec<u8> {
    format!("holler-auth-v1|{protocol_version}|{role}|{nonce_hex}|{token_id}|{advertised_url}").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::build;

    #[test]
    fn differs_on_every_field() {
        let base = build(2, "body", "aa", "tok_1", "wss://hub");
        assert_ne!(base, build(3, "body", "aa", "tok_1", "wss://hub"), "protocol_version must be bound");
        assert_ne!(base, build(2, "hub", "aa", "tok_1", "wss://hub"), "role must be bound");
        assert_ne!(base, build(2, "body", "bb", "tok_1", "wss://hub"), "nonce must be bound");
        assert_ne!(base, build(2, "body", "aa", "tok_2", "wss://hub"), "token_id must be bound");
        assert_ne!(base, build(2, "body", "aa", "tok_1", "wss://other"), "advertised_url must be bound");
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(
            build(2, "body", "deadbeef", "tok_7f3a", "wss://uranus.example"),
            build(2, "body", "deadbeef", "tok_7f3a", "wss://uranus.example"),
        );
    }
}
