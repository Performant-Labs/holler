//! Token-domain errors shared across the hub store and the CLI (story #163).
//!
//! The hub's token **store** lives in `holler-hub` (`crate::token`), but its
//! error *types* live here: the store's async wrappers run their work in
//! `tokio::task::spawn_blocking`, whose closure is a `FnOnce` whose return
//! type is `Send` — so any error type that crosses that boundary must be
//! `Send + Sync + 'static`. Keeping these in `holler-proto` (the no-I/O crate
//! both roles link) guarantees the hub's and the CLI's error arms name the
//! same type, and keeps the types independent of the hub's async machinery.

/// A fail-closed refusal from the hub's token store (`mint` / `delete` /
/// `touch`). `list` and `redeem` return their own, more specific, error
/// types; this is the catch-all for the operations that can only fail on an
/// unresolvable state dir or an I/O failure on `tokens.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenError {
    /// The human-facing reason (the message the CLI prints before exit 1).
    pub message: String,
}

impl TokenError {
    /// Wrap a message in a fail-closed token-store refusal.
    #[inline]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TokenError {}

/// Why a redeem of a join secret was refused. Each maps to a distinct CLI
/// exit 3 with the spec's message (see `holler-hub::token`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedeemError {
    /// No token's secret-hmac matched the presented secret.
    NotFound,
    /// The secret matched a token that is already bound (it was consumed).
    AlreadyBound,
    /// The secret matched a token whose state is `revoked`.
    Revoked,
    /// The secret matched a token whose `expires` has passed.
    Expired,
}

impl RedeemError {
    /// The CLI's exit-3 stderr message for this refusal (the spec's wording).
    #[inline]
    pub fn message(self) -> &'static str {
        match self {
            RedeemError::NotFound => "no matching token",
            RedeemError::AlreadyBound => "token already redeemed",
            RedeemError::Revoked => "token revoked",
            RedeemError::Expired => "token expired",
        }
    }
}
