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

/// The shared lockout state.
pub struct Lockout {
    limits: LockoutLimits,
    /// Peer IP → (the `now` of the most recent tripping, the count that
    /// tripped). A tripped peer stays in the map until its cooldown lapses.
    tripped: Mutex<HashMap<IpAddr, (u64, u64)>>,
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
            tripped: Mutex::new(HashMap::new()),
            clock: Box::new(RealClock),
        })
    }

    /// A new lockout with an injected clock (for deterministic tests).
    #[doc(hidden)]
    pub fn with_clock(clock: Box<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            limits: LockoutLimits::default(),
            tripped: Mutex::new(HashMap::new()),
            clock,
        })
    }

    /// Whether `peer` is currently locked out (cooldown not yet elapsed).
    pub fn is_locked_out(&self, peer: &IpAddr) -> bool {
        let now = self.clock.now_ms();
        let map = self.tripped.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(peer) {
            Some((since, _)) => now.saturating_sub(*since) < self.limits.duration_ms,
            None => false,
        }
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
    /// authentication does **not** clear a failure count (a lockout that
    /// expired does, by the cooldown itself).
    pub fn record_failure(&self, peer: &IpAddr) -> bool {
        let now = self.clock.now_ms();
        let mut map = self.tripped.lock().unwrap_or_else(|e| e.into_inner());
        // Drop tripped peers whose cooldown has lapsed.
        map.retain(|_, (since, _)| now.saturating_sub(*since) < self.limits.duration_ms);

        // Already locked out: refresh the cooldown and report (re-)trip.
        if let Some(tripped) = map.get_mut(peer) {
            if now.saturating_sub(tripped.0) < self.limits.duration_ms {
                tripped.0 = now;
                return true;
            }
        }

        // Not (yet) locked out: accumulate failures within the window.
        // We model the window as "failures whose `now` is within
        // window_ms of the most recent failure". For the simple counter model
        // we store the count and the time of the first failure in the current
        // window; when the window lapses we reset the count.
        let tripped = map.entry(*peer).or_insert((now, 0));
        let (window_start, count) = tripped;
        if now.saturating_sub(*window_start) >= self.limits.window_ms {
            // The window has lapsed: start a fresh window at this failure.
            *window_start = now;
            *count = 1;
        } else {
            *count += 1;
        }
        if *count >= self.limits.max_failures {
            // Tripped: the peer is now refused for the cooldown.
            *window_start = now;
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
