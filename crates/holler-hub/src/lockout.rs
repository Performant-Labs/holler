//! Failed-authentication lockout (issue #184).
//!
//! A peer that repeatedly fails to authenticate (`circuit/authenticate`
//! rejects with an error) is refused for a cooldown: after
//! [`DEFAULT_MAX_FAILURES`] failures within [`DEFAULT_WINDOW_MS`] ms, new
//! connections from that peer are refused (close **1008**, before reading a
//! frame) for [`DEFAULT_DURATION_MS`] ms. The key is the peer's transport IP
//! address (never the claimed hostname — that is unauthenticated input at
//! the point a lockout decision is made). The tunables are read from the
//! environment (`HOLLER_LOCKOUT_MAX_FAILURES`, `HOLLER_LOCKOUT_WINDOW_MS`,
//! `HOLLER_LOCKOUT_DURATION_MS`), defaulting to 5 / 10 min / 10 min.
//!
//! This is a fresh build against current `main` (issue #184's spec), but the
//! `PeerState`/`tripped_since` design below is carried over verbatim from
//! `fix/224-spurious-lockout-oneshot-panic` (PR #289) — a real bug that PR
//! found and fixed on the (stale, unmerged) `feat/184-hub-registry` branch:
//! an earlier version used one untyped `(window_start, count)` tuple for
//! both "accumulating failures" and "actually tripped", so the very
//! **first** recorded failure already put an entry in the map and
//! `is_locked_out` (which only checked entry presence) refused that peer's
//! very next connection — long before `max_failures` was ever reached. See
//! the `peers` field doc below for the full account. Loopback is **not**
//! exempt (the spec is explicit, and the test suite relies on it).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Default: how many auth failures within the window trip the lockout.
pub const DEFAULT_MAX_FAILURES: u64 = 5;
/// Default: the window (ms) in which failures are counted.
pub const DEFAULT_WINDOW_MS: u64 = 10 * 60 * 1000; // 10 min
/// Default: the duration (ms) a tripped peer is refused.
pub const DEFAULT_DURATION_MS: u64 = 10 * 60 * 1000; // 10 min

/// The resolved lockout tunables. `PartialEq` lets `hub status` report
/// whether an operator overrode any default (`limits.lockout`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LockoutLimits {
    pub max_failures: u64,
    pub window_ms: u64,
    pub duration_ms: u64,
}

impl Default for LockoutLimits {
    fn default() -> Self {
        Self::resolve()
    }
}

impl LockoutLimits {
    /// Resolve the tunables from the environment, falling back to defaults.
    pub fn resolve() -> Self {
        Self {
            max_failures: std::env::var("HOLLER_LOCKOUT_MAX_FAILURES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_MAX_FAILURES),
            window_ms: std::env::var("HOLLER_LOCKOUT_WINDOW_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_WINDOW_MS),
            duration_ms: std::env::var("HOLLER_LOCKOUT_DURATION_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_DURATION_MS),
        }
    }
}

/// Per-peer failure-accumulation / trip state. `tripped_since` is `None`
/// while the peer merely has failures accumulating in the current window —
/// distinct from `count` reaching `max_failures`, which is when the peer
/// actually trips.
#[derive(Debug, Clone, Copy)]
struct PeerState {
    window_start: u64,
    count: u64,
    tripped_since: Option<u64>,
}

/// The shared lockout state.
pub struct Lockout {
    limits: LockoutLimits,
    /// Peer IP → failure/trip state (see [`PeerState`]'s own doc for why
    /// `tripped_since` must be an explicit field rather than inferred from
    /// map membership — the exact PR #289 regression this design avoids).
    peers: Mutex<HashMap<IpAddr, PeerState>>,
    /// A clock source: monotonic `now` in milliseconds. Tests inject a fake
    /// that advances deterministically; production uses the real monotonic
    /// clock.
    clock: Box<dyn Clock>,
}

impl Lockout {
    /// A new lockout with the default (monotonic wall) clock.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            limits: LockoutLimits::resolve(),
            peers: Mutex::new(HashMap::new()),
            clock: Box::new(RealClock),
        })
    }

    /// A new lockout with an injected clock (for deterministic tests).
    #[doc(hidden)]
    pub fn with_clock(clock: Box<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            limits: LockoutLimits::resolve(),
            peers: Mutex::new(HashMap::new()),
            clock,
        })
    }

    /// The resolved limits this lockout is enforcing (`hub status`'s
    /// `limits.lockout`).
    pub fn limits(&self) -> LockoutLimits {
        self.limits
    }

    /// Whether `peer` is currently locked out (cooldown not yet elapsed).
    /// `false` for a peer with in-window failures that have not yet reached
    /// `max_failures` — only an actual trip (`tripped_since: Some`) refuses.
    pub fn is_locked_out(&self, peer: &IpAddr) -> bool {
        let now = self.clock.now_ms();
        let map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(peer).and_then(|s| s.tripped_since) {
            Some(since) => now.saturating_sub(since) < self.limits.duration_ms,
            None => false,
        }
    }

    /// Reset the failure count for `peer` (a successful authentication
    /// clears any in-window strikes). If the peer is currently locked out,
    /// the cooldown is also lifted.
    pub fn reset(&self, peer: &IpAddr) {
        self.peers.lock().unwrap_or_else(|e| e.into_inner()).remove(peer);
    }

    /// Record an authentication failure for `peer`. Returns `true` if this
    /// failure **tripped** the lockout (the peer is now refused for the
    /// cooldown).
    pub fn record_failure(&self, peer: &IpAddr) -> bool {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        // Drop stale entries: a tripped peer whose cooldown has lapsed, or a
        // merely-accumulating peer whose window has lapsed with no trip.
        map.retain(|_, s| match s.tripped_since {
            Some(since) => now.saturating_sub(since) < self.limits.duration_ms,
            None => now.saturating_sub(s.window_start) < self.limits.window_ms,
        });

        let entry = map.entry(*peer).or_insert(PeerState {
            window_start: now,
            count: 0,
            tripped_since: None,
        });

        if entry.tripped_since.is_some() {
            entry.tripped_since = Some(now);
            return true;
        }

        if now.saturating_sub(entry.window_start) >= self.limits.window_ms {
            entry.window_start = now;
            entry.count = 1;
        } else {
            entry.count += 1;
        }
        if entry.count >= self.limits.max_failures {
            entry.tripped_since = Some(now);
            true
        } else {
            false
        }
    }
}

/// A monotonic millisecond clock. The default source is the process's
/// monotonic clock; tests inject a fake that advances deterministically.
#[doc(hidden)]
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// The default (real) monotonic clock.
#[doc(hidden)]
pub struct RealClock;

#[doc(hidden)]
impl Clock for RealClock {
    fn now_ms(&self) -> u64 {
        static ANCHOR: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let anchor = ANCHOR.get_or_init(Instant::now);
        u64::try_from(anchor.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // #184 — test-only fixture parsing (a fixed literal IP)
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Clone)]
    struct FakeClock(Arc<AtomicU64>);
    impl Clock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }
    impl FakeClock {
        fn new() -> Self {
            Self(Arc::new(AtomicU64::new(0)))
        }
        fn advance(&self, ms: u64) {
            self.0.fetch_add(ms, Ordering::SeqCst);
        }
    }

    fn lockout_with(clock: FakeClock) -> Arc<Lockout> {
        Arc::new(Lockout {
            limits: LockoutLimits { max_failures: 5, window_ms: 10_000, duration_ms: 10_000 },
            peers: Mutex::new(HashMap::new()),
            clock: Box::new(clock),
        })
    }

    fn peer() -> IpAddr {
        "127.0.0.1".parse().unwrap()
    }

    #[test]
    fn a_single_failure_does_not_lock_out_the_peer() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock);
        let ip = peer();

        assert!(!lockout.is_locked_out(&ip), "fresh peer is never locked out");
        let tripped = lockout.record_failure(&ip);
        assert!(!tripped, "1 failure (of 5 required) must not trip the lockout");
        assert!(!lockout.is_locked_out(&ip), "1 of 5 failures must not lock the peer out");
    }

    #[test]
    fn exactly_max_failures_are_required_to_trip() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock);
        let ip = peer();

        for i in 1..5 {
            let tripped = lockout.record_failure(&ip);
            assert!(!tripped, "failure {i} of 5 must not trip the lockout yet");
            assert!(!lockout.is_locked_out(&ip), "failure {i} of 5 must not lock the peer out yet");
        }
        let tripped = lockout.record_failure(&ip);
        assert!(tripped, "the 5th failure must trip the lockout");
        assert!(lockout.is_locked_out(&ip), "a tripped peer must be locked out");
    }

    #[test]
    fn reset_clears_in_window_failures() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock);
        let ip = peer();

        for _ in 0..4 {
            assert!(!lockout.record_failure(&ip));
        }
        lockout.reset(&ip);
        assert!(!lockout.is_locked_out(&ip));
        for _ in 0..4 {
            assert!(!lockout.record_failure(&ip));
        }
        assert!(!lockout.is_locked_out(&ip));
    }

    #[test]
    fn trip_lapses_after_duration_and_window_resets_after_window_ms() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let ip = peer();

        for _ in 0..5 {
            lockout.record_failure(&ip);
        }
        assert!(lockout.is_locked_out(&ip));
        clock.advance(10_001);
        assert!(!lockout.is_locked_out(&ip), "the trip must lapse after duration_ms");

        for _ in 0..4 {
            assert!(!lockout.record_failure(&ip));
        }
        clock.advance(10_001);
        for _ in 0..4 {
            assert!(!lockout.record_failure(&ip));
        }
        assert!(!lockout.is_locked_out(&ip));
    }
}
