//! Protocol version constants (ADR 0003: a single protocol version).
//!
//! The hub advertises `protocol_min = protocol_max = 3`. `3` is the version
//! this protocol (docs/protocol/v2.md) defines. `holler hello.protocol` must
//! equal `3` or the peer answers `-32000 unsupported_version` and closes the
//! socket — **no silent downgrade**.
//!
//! Issue #340: bumped from `2` (the pre-Noise-XK version) once #338/#339
//! landed the Noise XK handshake + pairing SAS — a deliberate **hard
//! re-pair event** for any body still speaking the old challenge-response
//! protocol (#321's "Protocol break handling"). `circuit/authenticate`'s
//! own `protocol` field ([`crate::docs::Authenticate`]) is checked against
//! this range *before* any Noise processing begins, so a mismatched peer
//! gets a specific `-32000` refusal instead of an opaque handshake failure;
//! `circuit/hello`'s `protocol` field is checked the same way as a second,
//! defense-in-depth layer (mirroring the `hub_pubkey` pinning check's own
//! "the crypto already enforces this, but the check stays" precedent).
//!
//! These are wire-level constants, so they live here (not in the body/hub
//! crates) so every endpoint that speaks the protocol shares one source of
//! truth.

/// The protocol version defined by `docs/protocol/v2.md`.
pub const PROTOCOL_VERSION: u32 = 3;

/// The lowest protocol version this build can also speak (ADR 0003: `min ==
/// max`).
pub const PROTOCOL_MIN: u32 = PROTOCOL_VERSION;

/// The highest protocol version this build can also speak (ADR 0003: `min ==
/// max`).
pub const PROTOCOL_MAX: u32 = PROTOCOL_VERSION;

/// Whether `version` is within the range this build speaks
/// (`PROTOCOL_MIN..=PROTOCOL_MAX`). Used by `query/protocol {version}` and by
/// the hello check.
#[inline]
pub fn is_supported_version(version: u32) -> bool {
    (PROTOCOL_MIN..=PROTOCOL_MAX).contains(&version)
}
