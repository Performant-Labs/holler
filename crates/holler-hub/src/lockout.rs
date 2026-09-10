//! Failed-authentication lockout (story #184).
//!
//! A peer that repeatedly fails to authenticate (`circuit/join` or
//! `circuit/authenticate` reject with an error) is refused for a cooldown:
//! after [`DEFAULT_MAX_FAILURES`] failures within [`DEFAULT_WINDOW_MS`] ms,
//! new connections from that peer are refused (close **1008**) for
//! [`DEFAULT_DURATION_MS`] ms. The key is the peer's IP address (the
//! transport peer, not the claimed hostname). The tunables are read from the
//! environment (`HOLLER_LOCKOUT_MAX_FAILURES`, `HOLLER_LOCKOUT_WINDOW_MS`,
//! `HOLLER_LOCKOUT_DURATION_MS`), defaulting to 5 / 10 min / 10 min.

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

/// The resolved lockout tunables. `PartialEq` lets `hub status` report whether
/// an operator overrode any default (`limits.lockout` vs the defaults).
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
/// actually trips (see the struct's doc comment for why this must be its own
/// field rather than inferred from map membership).
#[derive(Debug, Clone, Copy)]
struct PeerState {
    /// The start of the current failure-counting window.
    window_start: u64,
    /// Failures recorded within the current window.
    count: u64,
    /// `Some(since)` once this peer has actually tripped the lockout (`count`
    /// reached `max_failures`); the peer is refused until `since +
    /// duration_ms`. `None` while failures are merely accumulating.
    tripped_since: Option<u64>,
}

/// The shared lockout state.
pub struct Lockout {
    limits: LockoutLimits,
    /// Peer IP → failure/trip state. An entry survives in the map only while
    /// it is either still accumulating within its window or still tripped
    /// and not yet cooled down (`record_failure` prunes stale entries).
    ///
    /// `is_locked_out` and the "already tripped" branch of `record_failure`
    /// key off `PeerState::tripped_since` specifically, **not** off map
    /// membership. An earlier version of this file used one untyped
    /// `(u64, u64)` tuple for both the "accumulating failures" state and the
    /// "actually tripped" state, so `is_locked_out` (which only checked
    /// whether an entry existed and was recent) could not tell "this peer has
    /// 1 failure recorded" from "this peer tripped the lockout" — the
    /// **first** recorded failure for any peer already put an entry in the
    /// map, so `is_locked_out` refused that peer's very next connection
    /// immediately, and `record_failure`'s own re-trip branch (which checked
    /// the same untyped entry) reported a trip on the peer's **second**
    /// failure rather than its `max_failures`-th. That is the mechanism
    /// behind issue #224's reported symptom ("auth refused: too many
    /// failures" on what the client considers close to a first attempt): not
    /// that a single failure alone tripped the map (a single call to
    /// `record_failure` in isolation does return `false`, as the issue noted
    /// it must), but that first failure's mere presence in the map was
    /// already indistinguishable from a real trip to every caller that
    /// checked the map afterward — `is_locked_out` on the next accepted
    /// socket, and `record_failure` itself on the next failure. `PeerState`
    /// with an explicit `tripped_since: Option<u64>` field fixes that: only a
    /// real trip (count reaching `max_failures`) sets it, and both
    /// `is_locked_out` and the re-trip branch check it explicitly instead of
    /// inferring it from presence.
    peers: Mutex<HashMap<IpAddr, PeerState>>,
    /// A clock source: monotonic `now` in milliseconds. A real `Instant`-based
    /// source is injected so tests can advance time deterministically; the
    /// default source is the process's monotonic clock.
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
            limits: LockoutLimits::default(),
            peers: Mutex::new(HashMap::new()),
            clock,
        })
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

    /// Reset the failure count for `peer` (a successful authentication clears
    /// any in-window strikes, per the spec: "A successful auth resets the
    /// counter"). If the peer is currently locked out, the cooldown is also
    /// lifted (the successful auth proves the peer is no longer attacking).
    pub fn reset(&self, peer: &IpAddr) {
        self.peers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(peer);
    }

    /// Record an authentication failure for `peer`. Returns `true` if this
    /// failure **tripped** the lockout (the peer is now refused for the
    /// cooldown). Failures are counted within the sliding window; a peer is
    /// tripped once `max_failures` failures accumulate within `window_ms`.
    ///
    /// This story keeps a simple, total model: a peer that already has an
    /// un-lapsed tripped cooldown is re-tripped (its cooldown is refreshed),
    /// and otherwise its in-window failure count is tracked until it reaches
    /// `max_failures`, at which point it is tripped. A successful
    /// authentication clears the failure count (see [`Self::reset`]).
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

        // Already tripped (and not lapsed — `retain` above would have pruned
        // a lapsed trip): refresh the cooldown and report (re-)trip.
        if entry.tripped_since.is_some() {
            entry.tripped_since = Some(now);
            return true;
        }

        // Not (yet) tripped: accumulate failures within the window. We model
        // the window as "failures whose `now` is within window_ms of the
        // most recent failure": store the count and the time of the first
        // failure in the current window; when the window lapses, start over.
        if now.saturating_sub(entry.window_start) >= self.limits.window_ms {
            entry.window_start = now;
            entry.count = 1;
        } else {
            entry.count += 1;
        }
        if entry.count >= self.limits.max_failures {
            // Tripped: the peer is now refused for the cooldown.
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
        // `Instant`'s epoch is arbitrary but monotonic; `elapsed` since a
        // process-start anchor is a stable, non-wrapping millisecond value.
        // The anchor is a `static` `Instant` captured once.
        static ANCHOR: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let anchor = ANCHOR.get_or_init(Instant::now);
        anchor.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // #224 — test-only fixture parsing (a fixed literal IP)
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A deterministic clock a test advances by hand. `Clone`s share the same
    /// underlying counter (via the inner `Arc`), so a test can hand one clone
    /// to a `Lockout` (which owns it as a `Box<dyn Clock>`) and keep another
    /// to call `advance` on.
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
            limits: LockoutLimits {
                max_failures: 5,
                window_ms: 10_000,
                duration_ms: 10_000,
            },
            peers: Mutex::new(HashMap::new()),
            clock: Box::new(clock),
        })
    }

    fn peer() -> IpAddr {
        "127.0.0.1".parse().unwrap()
    }

    /// Regression for issue #224: a peer's **first** recorded failure must
    /// not be indistinguishable from an actual trip. Before the `PeerState`
    /// split, `record_failure`'s only side effect on failure #1 was to
    /// insert a `(now, 1)` entry into the same map `is_locked_out` read, and
    /// `is_locked_out` treated *any* entry as "tripped" — so the very next
    /// connection attempt from this peer, even though only one failure (of
    /// five required) had been recorded, was refused as if the lockout had
    /// tripped.
    #[test]
    fn a_single_failure_does_not_lock_out_the_peer() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock);
        let ip = peer();

        assert!(!lockout.is_locked_out(&ip), "fresh peer is never locked out");
        let tripped = lockout.record_failure(&ip);
        assert!(!tripped, "1 failure (of 5 required) must not trip the lockout");
        assert!(
            !lockout.is_locked_out(&ip),
            "a peer with 1 of 5 failures recorded must not be reported as locked out \
             (issue #224: is_locked_out must not treat map presence as a trip)"
        );
    }

    /// The lockout must require exactly `max_failures` (not 2, not any
    /// number less than the configured threshold) before it trips — the
    /// second regression angle on the same bug: `record_failure`'s own
    /// "already tripped" branch used to key off the same untyped map entry,
    /// so a peer's **second** failure (not its fifth) hit the re-trip branch
    /// and was reported as a trip.
    #[test]
    fn exactly_max_failures_are_required_to_trip() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock);
        let ip = peer();

        for i in 1..5 {
            let tripped = lockout.record_failure(&ip);
            assert!(!tripped, "failure {i} of 5 must not trip the lockout yet");
            assert!(
                !lockout.is_locked_out(&ip),
                "failure {i} of 5 must not lock the peer out yet"
            );
        }
        let tripped = lockout.record_failure(&ip);
        assert!(tripped, "the 5th failure must trip the lockout");
        assert!(lockout.is_locked_out(&ip), "a tripped peer must be locked out");
    }

    /// A successful auth (`reset`) clears an in-window failure count that
    /// has not yet tripped.
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
        // The count was cleared: another 4 failures must not trip it either.
        for _ in 0..4 {
            assert!(!lockout.record_failure(&ip));
        }
        assert!(!lockout.is_locked_out(&ip));
    }

    /// A tripped lockout lapses after `duration_ms`, and a peer whose
    /// failures are merely accumulating (not yet tripped) has its window
    /// reset once `window_ms` elapses without reaching `max_failures`.
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

        // A fresh window: 4 failures, then let the window lapse, then 4 more
        // — must not trip (the window reset, so the count never reaches 5
        // within a single window).
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
