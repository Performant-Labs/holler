//! One-time release grants (issue #460): an opaque, in-memory capability that
//! lets exactly one prompt through a *default* hold.
//!
//! Nothing here is persisted (a hub restart voids every grant, which fails
//! closed), nothing knows who mints or presents one, and a grant is bound to
//! one session key. Time is `std::time::Instant`, passed in so the unit tests
//! can drive expiry without sleeping.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a used or expired grant is remembered, so a late `say --grant`
/// can be told `used` or `expired` rather than `unknown`.
const TOMBSTONE: Duration = Duration::from_secs(600);

/// The most grants (live and remembered) held at once; minting past it is
/// refused after expired ones have been dropped.
pub const MAX_GRANTS: usize = 50_000;

/// The longest TTL a grant may be minted with.
pub const MAX_TTL: Duration = Duration::from_secs(24 * 3600);

/// The default TTL when none is given.
pub const DEFAULT_TTL: Duration = Duration::from_secs(60);

/// Why a presented grant is not honoured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantError {
    Unknown,
    Expired,
    Used,
    OtherSession,
}

impl GrantError {
    /// The `data.reason` of the `-32012 invalid_grant` refusal.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Expired => "expired",
            Self::Used => "used",
            Self::OtherSession => "other_session",
        }
    }
}

/// Why a grant could not be minted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MintError {
    /// The table is full of live grants.
    Full,
    /// The system could not supply random bytes.
    NoRandomness,
}

struct Rec {
    key: String,
    expires: Instant,
    used_at: Option<Instant>,
}

#[derive(Default)]
pub(super) struct Grants {
    by_id: HashMap<String, Rec>,
}

impl Grants {
    /// Mint a grant for `key` valid until `now + ttl`.
    pub(super) fn mint(&mut self, key: &str, ttl: Duration, now: Instant) -> Result<String, MintError> {
        self.purge(now);
        if self.by_id.len() >= MAX_GRANTS {
            return Err(MintError::Full);
        }
        let mut raw = [0u8; 16];
        getrandom::fill(&mut raw).map_err(|_| MintError::NoRandomness)?;
        let id = format!("gnt_{}", hex::encode(raw));
        self.by_id.insert(id.clone(), Rec { key: key.to_owned(), expires: now + ttl.min(MAX_TTL), used_at: None });
        Ok(id)
    }

    /// Would `id` be honoured for `key` right now?
    pub(super) fn check(&self, id: &str, key: &str, now: Instant) -> Result<(), GrantError> {
        let rec = self.by_id.get(id).ok_or(GrantError::Unknown)?;
        if rec.key != key {
            return Err(GrantError::OtherSession);
        }
        if rec.used_at.is_some() {
            return Err(GrantError::Used);
        }
        if now >= rec.expires {
            return Err(GrantError::Expired);
        }
        Ok(())
    }

    /// [`Grants::check`], and on success mark the grant used: it cannot be
    /// honoured again.
    pub(super) fn consume(&mut self, id: &str, key: &str, now: Instant) -> Result<(), GrantError> {
        self.check(id, key, now)?;
        if let Some(rec) = self.by_id.get_mut(id) {
            rec.used_at = Some(now);
        }
        Ok(())
    }

    /// Live (unused, unexpired) grants for `key`.
    pub(super) fn live_for(&self, key: &str, now: Instant) -> usize {
        self.by_id.values().filter(|r| r.key == key && r.used_at.is_none() && now < r.expires).count()
    }

    fn purge(&mut self, now: Instant) {
        self.by_id.retain(|_, r| r.used_at.unwrap_or(r.expires) + TOMBSTONE > now);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #460
mod tests {
    use super::*;

    #[test]
    fn a_grant_is_honoured_once_for_its_own_session() {
        let mut g = Grants::default();
        let t0 = Instant::now();
        let id = g.mint("io/alpha", Duration::from_secs(60), t0).unwrap();
        assert!(id.starts_with("gnt_") && id.len() == 4 + 32);
        assert_eq!(g.check(&id, "io/beta", t0), Err(GrantError::OtherSession));
        assert_eq!(g.check("gnt_nope", "io/alpha", t0), Err(GrantError::Unknown));
        assert_eq!(g.live_for("io/alpha", t0), 1);
        assert_eq!(g.consume(&id, "io/alpha", t0), Ok(()));
        assert_eq!(g.consume(&id, "io/alpha", t0), Err(GrantError::Used));
        assert_eq!(g.live_for("io/alpha", t0), 0);
    }

    #[test]
    fn an_unused_grant_expires_at_its_ttl_and_a_wrong_session_does_not_consume_it() {
        let mut g = Grants::default();
        let t0 = Instant::now();
        let id = g.mint("io/alpha", Duration::from_secs(5), t0).unwrap();
        // A refused attempt for another session leaves it live.
        assert_eq!(g.consume(&id, "io/beta", t0), Err(GrantError::OtherSession));
        assert_eq!(g.check(&id, "io/alpha", t0 + Duration::from_secs(4)), Ok(()));
        assert_eq!(g.check(&id, "io/alpha", t0 + Duration::from_secs(5)), Err(GrantError::Expired));
        assert_eq!(g.consume(&id, "io/alpha", t0 + Duration::from_secs(6)), Err(GrantError::Expired));
    }

    #[test]
    fn forgotten_grants_become_unknown_and_ids_are_unique() {
        let mut g = Grants::default();
        let t0 = Instant::now();
        let a = g.mint("io/alpha", Duration::from_secs(1), t0).unwrap();
        let b = g.mint("io/alpha", Duration::from_secs(1), t0).unwrap();
        assert_ne!(a, b);
        let much_later = t0 + TOMBSTONE + Duration::from_secs(10);
        g.mint("io/alpha", Duration::from_secs(1), much_later).unwrap();
        assert_eq!(g.check(&a, "io/alpha", much_later), Err(GrantError::Unknown));
    }
}
