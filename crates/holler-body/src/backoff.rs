//! The reconnect backoff schedule (issue #182 step 5): AWS "full jitter",
//! base 1s, cap 30s, unbounded retries.
//!
//! `delay(attempt)` is pure and injectable-RNG so the schedule itself is
//! unit-testable without sleeping (`backoff_caps_at_30s` in the issue's own
//! RED test list) — the caller (`connection.rs`) is the only place that
//! actually sleeps.

use std::time::Duration;

/// The base delay (issue #182: "base 1s").
pub const BASE_MS: u64 = 1_000;
/// The cap every delay is bounded by (issue #182: "cap 30s").
pub const CAP_MS: u64 = 30_000;

/// AWS full-jitter: `sleep = random_between(0, min(cap, base * 2^attempt))`.
/// `attempt` is the zero-based retry count (0 = the first reconnect attempt).
/// `rand_u64` supplies a uniform `u64` in `0..bound` — injected so the
/// schedule is testable without depending on a real RNG's distribution.
pub fn delay(attempt: u32, rand_u64: impl FnOnce(u64) -> u64) -> Duration {
    // `1u64 << n` overflows past n=63; capping the shift at 20 (2^20 * 1000ms
    // is already far past `CAP_MS`) keeps this branch-free and always exact.
    let exp = BASE_MS.saturating_mul(1u64 << attempt.min(20));
    let bound = exp.min(CAP_MS);
    Duration::from_millis(rand_u64(bound.max(1)))
}

/// `delay` using the process's real CSPRNG (`getrandom`) for the jitter.
/// Fails closed to the *unjittered* bound (never `0`, never a hang) if the OS
/// call errs — an extremely rare condition this crate has no good recovery
/// from besides "wait the full window this once".
pub fn delay_random(attempt: u32) -> Duration {
    delay(attempt, |bound| {
        let mut buf = [0u8; 8];
        match getrandom::fill(&mut buf) {
            Ok(()) => u64::from_le_bytes(buf) % bound,
            Err(_) => bound.saturating_sub(1).max(1),
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #182
mod tests {
    use super::*;

    /// The issue's own RED test: 20 attempts, all jittered delays ≤ 30s.
    #[test]
    fn backoff_caps_at_30s() {
        for attempt in 0..20u32 {
            // A deterministic top-of-range "rng" is still a valid jitter
            // sample (full jitter's range includes its own upper bound), and
            // it is the sharpest test of the cap: if capping were broken this
            // is the value that would exceed it.
            let d = delay(attempt, |bound| bound.saturating_sub(1));
            assert!(
                d <= Duration::from_millis(CAP_MS),
                "attempt {attempt} produced {d:?}, over the 30s cap"
            );
        }
    }

    #[test]
    fn backoff_grows_before_the_cap() {
        let early = delay(0, |bound| bound);
        let later = delay(3, |bound| bound);
        assert!(later >= early, "the schedule must not shrink before the cap");
    }

    #[test]
    fn backoff_is_never_zero_width() {
        // `rand_u64` returning 0 is a legal jitter draw; the bound passed to
        // it must still be at least 1 (a zero bound would make every delay 0,
        // defeating backoff entirely).
        let d = delay(0, |bound| {
            assert!(bound >= 1);
            0
        });
        assert_eq!(d, Duration::from_millis(0));
    }
}
