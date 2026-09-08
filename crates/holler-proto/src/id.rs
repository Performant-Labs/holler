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
//! The body after the prefix is a **ULID** (Crockford Base32, time-ordered
//! then random). The [`mint_hub`](Self::mint_hub) /
//! [`mint_body`](Self::mint_body) constructors generate a real ULID, so a
//! minted id is always a well-formed `h-<26 chars>` / `b-<26 chars>`.
//!
//! Parsing is **lenient**: any non-empty `h-`/`b-`-prefixed string is
//! accepted. The strict ULID-body grammar (length 26) is asserted by the
//! minters, not re-checked on parse — a peer never rejects an id for the
//! body's grammar.
//!
//! Wire shape (ADR 0004 §5): the JSON-RPC `id` member is a **string**. The
//! grammar for a minted id is `^(h|b)-[0-9A-HJKMNP-TV-Z]{26}$` (Crockford
//! Base32, upper). Numeric ids are rejected (see
//! [`crate::envelope::EnvelopeError::BadId`]).

use std::fmt;

use ulid::Ulid;

/// A correlation id: a minted, prefixed string.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Parse a wire string into a `CorrelationId`.
    ///
    /// Succeeds iff the string is non-empty and starts with `h-` or `b-`.
    /// The body after the prefix is treated **opaquely** (any non-empty
    /// string is accepted) — the minters are where the strict ULID-body
    /// grammar is asserted, and a peer never rejects an id for the body's
    /// grammar (a wire id that is a valid prefix but a malformed body is
    /// still a distinct id, and that is the point of the prefix).
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

    /// Mint a **hub** correlation id: `h-<ULID>`.
    ///
    /// The body after the prefix is a real ULID (Crockford Base32, 26
    /// chars, time-ordered then random) generated via the `ulid` crate.
    /// Two ids minted in the same millisecond are still distinct (the
    /// random part differs), and minted ids are lexicographically ordered
    /// by mint time.
    pub fn mint_hub() -> Self {
        Self(format!("h-{}", Ulid::new()))
    }

    /// Mint a **body** correlation id: `b-<ULID>`.
    ///
    /// See [`CorrelationId::mint_hub`] for the body's properties.
    pub fn mint_body() -> Self {
        Self(format!("b-{}", Ulid::new()))
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
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
