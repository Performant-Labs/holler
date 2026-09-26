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
#[derive(Debug, Clone)]
struct PeerState {
    window_start: u64,
    count: u64,
    tripped_since: Option<u64>,
    /// The reason code of each counted failure (bounded by
    /// [`MAX_RECORDED_REASONS`]), so the trip can be logged with *why* the
    /// peer was locked out (issue #450).
    reasons: Vec<&'static str>,
}

/// Cap on the per-peer reason list; a peer hammering past its trip keeps the
/// most recent entries only.
const MAX_RECORDED_REASONS: usize = 16;

/// What one recorded failure did to its peer's lockout state (issue #450).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureOutcome {
    /// Failures counted in the current window, including this one.
    pub count: u64,
    /// The limit at which the peer trips.
    pub max: u64,
    /// This failure moved the peer from accumulating to locked out.
    pub newly_tripped: bool,
    /// The peer is locked out after this failure (newly or already).
    pub locked_out: bool,
    /// The reasons of the failures counted so far, oldest first.
    pub reasons: Vec<&'static str>,
    /// Cooldown length, and how long until it lapses, in milliseconds.
    pub duration_ms: u64,
    pub retry_after_ms: u64,
    /// Peers whose cooldown had lapsed and were dropped by this call.
    pub cleared: Vec<IpAddr>,
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
    /// Returns `true` if a lockout (a trip, not mere accumulating strikes)
    /// was lifted.
    pub fn reset(&self, peer: &IpAddr) -> bool {
        let removed = self.peers.lock().unwrap_or_else(|e| e.into_inner()).remove(peer);
        removed.is_some_and(|s| s.tripped_since.is_some())
    }

    /// Record an authentication failure for `peer`. Returns `true` if this
    /// failure **tripped** the lockout (the peer is now refused for the
    /// cooldown).
    pub fn record_failure(&self, peer: &IpAddr) -> bool {
        let o = self.record_failure_detailed(peer, "unspecified");
        o.locked_out
    }

    /// [`Self::record_failure`], reporting what happened (issue #450): the
    /// failure count in the window, whether this failure newly tripped the
    /// lockout, the recorded reasons, and any peers whose cooldown lapsed.
    pub fn record_failure_detailed(&self, peer: &IpAddr, reason: &'static str) -> FailureOutcome {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let cleared = Self::drop_stale(&mut map, now, self.limits);

        let entry = map.entry(*peer).or_insert(PeerState {
            window_start: now,
            count: 0,
            tripped_since: None,
            reasons: Vec::new(),
        });

        let mut newly_tripped = false;
        if entry.tripped_since.is_some() {
            entry.tripped_since = Some(now);
        } else {
            if now.saturating_sub(entry.window_start) >= self.limits.window_ms {
                entry.window_start = now;
                entry.count = 1;
                entry.reasons.clear();
            } else {
                entry.count += 1;
            }
            if entry.reasons.len() >= MAX_RECORDED_REASONS {
                entry.reasons.remove(0);
            }
            entry.reasons.push(reason);
            if entry.count >= self.limits.max_failures {
                entry.tripped_since = Some(now);
                newly_tripped = true;
            }
        }
        FailureOutcome {
            count: entry.count,
            max: self.limits.max_failures,
            newly_tripped,
            locked_out: entry.tripped_since.is_some(),
            reasons: entry.reasons.clone(),
            duration_ms: self.limits.duration_ms,
            retry_after_ms: entry
                .tripped_since
                .map_or(0, |since| self.limits.duration_ms.saturating_sub(now.saturating_sub(since))),
            cleared,
        }
    }

    /// Drop lapsed entries now and report the peers whose *cooldown* lapsed
    /// (issue #450's `lockout_cleared reason=expired`). Called on every
    /// accepted connection, so a lapse is reported promptly even if the
    /// peer never returns.
    pub fn sweep(&self) -> Vec<IpAddr> {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        Self::drop_stale(&mut map, now, self.limits)
    }

    /// Remove tripped peers whose cooldown has lapsed and accumulating peers
    /// whose window has lapsed with no trip; return the former.
    fn drop_stale(map: &mut HashMap<IpAddr, PeerState>, now: u64, limits: LockoutLimits) -> Vec<IpAddr> {
        let mut cleared = Vec::new();
        map.retain(|ip, s| match s.tripped_since {
            Some(since) => {
                let live = now.saturating_sub(since) < limits.duration_ms;
                if !live {
                    cleared.push(*ip);
                }
                live
            }
            None => now.saturating_sub(s.window_start) < limits.window_ms,
        });
        cleared
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

    /// Issue #450: the detailed outcome reports the in-window count, the
    /// reasons, and exactly one `newly_tripped` at the trip.
    #[test]
    fn detailed_outcome_counts_reasons_and_reports_the_trip_once() {
        let lockout = lockout_with(FakeClock::new());
        let ip = peer();
        for n in 1..5u64 {
            let o = lockout.record_failure_detailed(&ip, "token_expired");
            assert_eq!((o.count, o.max, o.newly_tripped, o.locked_out), (n, 5, false, false));
        }
        let o = lockout.record_failure_detailed(&ip, "bad_proof");
        assert!(o.newly_tripped && o.locked_out, "the 5th failure trips");
        assert_eq!(o.reasons, vec!["token_expired", "token_expired", "token_expired", "token_expired", "bad_proof"]);
        assert_eq!((o.duration_ms, o.retry_after_ms), (10_000, 10_000));
        let again = lockout.record_failure_detailed(&ip, "token_expired");
        assert!(again.locked_out && !again.newly_tripped, "a failure while locked out is not a second trip");
    }

    /// Issue #450: a lapsed cooldown is reported once, by `sweep`, and the
    /// peer starts a fresh window afterwards.
    #[test]
    fn a_lapsed_cooldown_is_reported_once_by_sweep() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let ip = peer();
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired");
        }
        assert!(lockout.sweep().is_empty(), "still locked out: nothing has lapsed");
        clock.advance(10_000);
        assert_eq!(lockout.sweep(), vec![ip], "the lapsed cooldown is reported");
        assert!(lockout.sweep().is_empty(), "and only once");
        assert!(!lockout.is_locked_out(&ip));
        assert_eq!(lockout.record_failure_detailed(&ip, "token_expired").count, 1, "a fresh window");
    }

    /// Issue #450: `reset` says whether it lifted a real lockout, not just
    /// mere accumulating strikes.
    #[test]
    fn reset_reports_only_a_lifted_lockout() {
        let lockout = lockout_with(FakeClock::new());
        let ip = peer();
        lockout.record_failure_detailed(&ip, "token_expired");
        assert!(!lockout.reset(&ip), "one strike is not a lockout");
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired");
        }
        assert!(lockout.reset(&ip), "a tripped peer's lockout is lifted");
    }
}
