//! Protocol version constants (ADR 0003: a single protocol version).
//!
// The hub advertises `protocol_min = protocol_max = 2`. `2` is the version
//! this protocol (docs/protocol/v2.md) defines. `holler hello.protocol` must
//! equal `2` or the peer answers `-32000 unsupported_version` and closes the
//! socket — **no silent downgrade**.
//!
//! These are wire-level constants, so they live here (not in the body/hub
//! crates) so every endpoint that speaks the protocol shares one source of
//! truth.

/// The protocol version defined by `docs/protocol/v2.md`.
pub const PROTOCOL_VERSION: u32 = 2;

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
