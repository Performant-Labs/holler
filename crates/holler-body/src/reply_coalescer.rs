//! The reply coalescer (issue #190): buffer a session's streamed reply
//! chunks and flush them as one `session/update` per window instead of one
//! per chunk — the body-side half of "the circuit actually talks" without
//! flooding the wire with a frame per token.
//!
//! Pure logic, no I/O and no internal clock: every method that cares about
//! time takes an explicit `Instant` (an injected clock, per the issue's own
//! RED test list — "unit, injected clock"), so the whole flush/window/cap
//! state machine is deterministic under test without a single real sleep.
//! The async glue that actually drives this off a real timer and turns a
//! flush into a `session/update` frame lives in
//! [`crate::connection`]/`prompt_dispatch` — this module only ever answers
//! "what should happen", never "wait until it does".
//!
//! One [`Coalescer`] instance belongs to exactly one in-flight prompt on one
//! session. Two sessions (or two successive turns on the same session) each
//! get their own instance — nothing here is shared, so "never merge two
//! sessions" holds by construction rather than by a runtime check.
//!
//! # Decisions I made
//!
//! - **`Part` payload only ever holds `Content::Text`.** The only
//!   [`crate::acp_driver::DriverEvent`] variant [`crate::session_manager`]
//!   forwards mid-turn is `Chunk(String)` (a streamed text delta) — there is
//!   no raw/url/data chunk event to coalesce yet. [`Coalescer::flush`] still
//!   returns `Vec<Part>` (not a bare `String`) to match the wire shape
//!   (`Update.parts`) exactly and stay a straight drop-in the day a non-text
//!   streamed part exists.
//! - **The byte cap is measured in UTF-8 bytes of the buffered text**, not
//!   chars or chunks — `HOLLER_COALESCE_MAX_BYTES`'s own name and the issue's
//!   "≥ 64 KiB" wording are both byte-denominated.

use std::time::{Duration, Instant};

use holler_proto::a2a::Part;

/// The coalescing window: how long a buffer may sit after its *first*
/// buffered chunk before it must flush. `HOLLER_COALESCE_MS` overrides the
/// 50ms default (tests inject a clock, but keep the real default reachable
/// from the same knob the async integration reads).
pub fn window() -> Duration {
    std::env::var("HOLLER_COALESCE_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(50))
}

/// The byte cap: a buffer at or above this many bytes flushes immediately
/// (including the chunk that crossed it). `HOLLER_COALESCE_MAX_BYTES`
/// overrides the 64 KiB default.
pub fn max_bytes() -> usize {
    std::env::var("HOLLER_COALESCE_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(64 * 1024)
}

/// What [`Coalescer::push`] tells the caller to do right away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    /// The chunk was buffered; no flush is needed yet (a window/turn-end
    /// flush may still come later).
    Buffered,
    /// The buffer hit the byte cap **including this chunk** — flush
    /// immediately. The caller should treat this exactly like a window flush
    /// (drain, reset, emit one `session/update`).
    CapHit,
}

/// One session's in-flight reply buffer for one prompt. See the module doc
/// for the "one instance per turn, never shared" contract.
pub struct Coalescer {
    buf: String,
    /// When the *first* chunk of the current (unflushed) window was pushed.
    /// `None` means the buffer is idle (empty, nothing pending) — a flush
    /// (window, cap, or turn-end) always clears this back to `None`.
    window_start: Option<Instant>,
}

impl Default for Coalescer {
    fn default() -> Self {
        Self::new()
    }
}

impl Coalescer {
    pub fn new() -> Self {
        Self { buf: String::new(), window_start: None }
    }

    /// Buffer one streamed text chunk, arriving at `now`. Consecutive chunks
    /// are merged (appended) into the one buffered string — never re-split
    /// into separate parts on flush. Order is preserved: `push` always
    /// appends, never reorders.
    pub fn push(&mut self, text: &str, now: Instant) -> PushOutcome {
        if self.window_start.is_none() {
            self.window_start = Some(now);
        }
        self.buf.push_str(text);
        if self.buf.len() >= max_bytes() {
            PushOutcome::CapHit
        } else {
            PushOutcome::Buffered
        }
    }

    /// `true` iff nothing is buffered right now.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// When this buffer's window flush is due, or `None` if the buffer is
    /// currently idle (nothing buffered — there is no window to wait out).
    /// The deadline is anchored to the *first* chunk of the window, not
    /// extended by later chunks in the same window.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.window_start.map(|start| start + window())
    }

    /// `true` iff the window flush is due at `now` (buffered, and `now` has
    /// reached [`Coalescer::next_deadline`]). `false` while idle, and `false`
    /// before the deadline — a flush must not fire early.
    pub fn due(&self, now: Instant) -> bool {
        match self.next_deadline() {
            Some(deadline) => now >= deadline,
            None => false,
        }
    }

    /// Drain the buffer into one merged text [`Part`], resetting the window
    /// (the next `push` after this starts a fresh window). Used by every
    /// flush trigger (window, cap, turn-end) — turn-end and idle callers get
    /// `None` for free when there is nothing buffered (never emits an empty
    /// `session/update`).
    pub fn flush(&mut self) -> Option<Vec<Part>> {
        if self.buf.is_empty() {
            return None;
        }
        let text = std::mem::take(&mut self.buf);
        self.window_start = None;
        Some(vec![Part::text_part(text)])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #190
mod tests {
    //! In-crate unit tests for the pure state machine, kept alongside the
    //! logic they cover. The issue's own named RED list
    //! (`crates/holler-body/tests/coalescer_test.rs`) is the black-box
    //! surface these back — see that file for the full 14-case suite; this
    //! module has a lighter smoke pass so `cargo test -p holler-body`
    //! (without the `holler-cli` integration target) still exercises the
    //! state machine.
    use super::*;

    #[test]
    fn buffers_without_flushing_below_cap_and_window() {
        let mut c = Coalescer::new();
        let t0 = Instant::now();
        assert_eq!(c.push("a", t0), PushOutcome::Buffered);
        assert!(!c.is_empty());
        assert!(!c.due(t0));
    }

    #[test]
    fn cap_hit_flush_includes_triggering_chunk() {
        let mut c = Coalescer::new();
        let t0 = Instant::now();
        let big = "x".repeat(max_bytes() - 1);
        assert_eq!(c.push(&big, t0), PushOutcome::Buffered);
        assert_eq!(c.push("y", t0), PushOutcome::CapHit);
        let parts = c.flush().expect("cap-triggered flush is non-empty");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text(), Some(format!("{big}y")).as_deref());
        assert!(c.is_empty());
        assert_eq!(c.next_deadline(), None);
    }
}
