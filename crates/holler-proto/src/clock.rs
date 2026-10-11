//! Wall-clock timestamp helpers (issue #207).
//!
//! `now_secs`/`now_millis` were hand-rolled identically 5+ times across
//! `holler-body` and `holler-hub` (plus a third, `u64`-returning variant in
//! `holler-hub::token`) instead of living once here — both crates already
//! depend on this crate for the wire types the resulting timestamps end up
//! in (`PingAck.ts`, `ConnectionState.since`/`last_frame_at`, …). Reading the
//! wall clock is pure computation (`SystemTime::now()`), so it fits this
//! crate's "no network or async dependency" charter exactly like the `time`
//! crate use in [`crate::log`] already does.
//!
//! `holler-hub::token::now_secs` keeps its own `u64`-returning public
//! signature (its `Record` fields — `created`/`expires`/`bound_at`/
//! `last_seen` — and `Roster`'s `Clock`/`AtomicU64` machinery are `u64`
//! throughout the on-disk token store and the roster's manual test clock);
//! changing that public surface to `i64` would ripple through persisted JSON
//! fields and roster arithmetic well beyond this issue's "pure refactor, no
//! behavior change" scope. It now delegates to [`now_secs`] internally
//! (cast back to `u64`, always non-negative) so there is exactly one place
//! in the whole workspace that actually calls `SystemTime::now()` for
//! seconds-resolution timestamps.

/// The current unix epoch in whole seconds. `0` on a clock error (a system
/// clock set before 1970 — never expected in practice, and every call site
/// already treated this as the harmless fallback).
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The current unix epoch in whole milliseconds. `0` on a clock error, same
/// discipline as [`now_secs`].
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #207
mod tests {
    use super::*;

    #[test]
    fn now_secs_is_a_plausible_unix_timestamp() {
        // Any time after this crate was written; guards against an
        // accidental `0`/overflow regression without pinning an exact value.
        assert!(now_secs() > 1_700_000_000);
    }

    #[test]
    fn now_millis_is_roughly_now_secs_times_1000() {
        let secs = now_secs();
        let millis = now_millis();
        assert!(millis >= secs * 1000);
        assert!(millis < (secs + 2) * 1000);
    }
}
