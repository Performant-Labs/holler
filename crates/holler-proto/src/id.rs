//! Correlation ids.
//!
//! A JSON-RPC `id` is a **string** on the wire, but v2 (ADR 0004) mints them
//! with a prefix that marks the minting side:
//!
//! - `h-` — minted by the **hub**
//! - `b-` — minted by the **body**
//!
//! The prefix is a **minting convention, not a wire field a peer must
//! validate**: a peer never rejects a message *because* of the prefix. It
//! exists so the two sides' id spaces **cannot collide** — a `b-` id can
//! never equal an `h-` id — and so an unmatched response can be triaged.
//!
//! The body after the prefix is a **ULID** (monotonic, time-ordered). This
//! crate has no ULID dependency; `CorrelationId` carries the minted string and
//! the `h-`/`b-` minting helpers format a supplied ULID body. A `CorrelationId`
//! is parsed **leniently** (any non-empty `h-`/`b-`-prefixed string) — the
//! grammar the codec *validates* is that the prefix is exactly `h` or `b`.

use std::fmt;

/// A correlation id: a minted, prefixed string.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Parse a wire string into a `CorrelationId`.
    ///
    /// Succeeds iff the string is non-empty and starts with `h-` or `b-`.
    /// (The body after the prefix is treated opaquely here; the *strict*
    /// ULID-body grammar is asserted by the minters and by the tests.)
    pub fn parse(s: &str) -> Result<Self, CorrelationIdError> {
        if s.is_empty() {
            return Err(CorrelationIdError::Empty);
        }
        let ok = s.starts_with("h-") || s.starts_with("b-");
        if !ok {
            return Err(CorrelationIdError::NoPrefix);
        }
        Ok(Self(s.to_owned()))
    }

    /// The minted string, verbatim.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` for a hub-minted id (`h-…`).
    #[inline]
    pub fn is_hub(&self) -> bool {
        self.0.starts_with("h-")
    }

    /// `true` for a body-minted id (`b-…`).
    #[inline]
    pub fn is_body(&self) -> bool {
        self.0.starts_with("b-")
    }

    /// Mint a **hub** id from a ULID body: `h-<ulid>`.
    pub fn mint_hub(ulid_body: &str) -> Self {
        Self(format!("h-{ulid_body}"))
    }

    /// Mint a **body** id from a ULID body: `b-<ulid>`.
    pub fn mint_body(ulid_body: &str) -> Self {
        Self(format!("b-{ulid_body}"))
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CorrelationId({:?})", self.0)
    }
}

impl From<&CorrelationId> for String {
    fn from(id: &CorrelationId) -> String {
        id.0.clone()
    }
}
impl From<CorrelationId> for String {
    fn from(id: CorrelationId) -> String {
        id.0
    }
}

/// Why a wire string was not a valid correlation id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrelationIdError {
    /// The string was empty.
    Empty,
    /// The string did not start with `h-` or `b-`.
    NoPrefix,
}

impl std::fmt::Display for CorrelationIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorrelationIdError::Empty => f.write_str("correlation id is empty"),
            CorrelationIdError::NoPrefix => f.write_str("correlation id missing h-/b- prefix"),
        }
    }
}
impl std::error::Error for CorrelationIdError {}
