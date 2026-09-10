#![allow(clippy::unwrap_used, clippy::expect_used)] // #190: test assertions
//! RED tests for the reply coalescer (issue #190), ported by name from the
//! holler-client history (hlrclnt-1604…1617). Unit-level, injected clock: no
//! real sleeping anywhere in this file — every case advances a
//! `std::time::Instant` by hand and asserts against
//! [`holler_body::reply_coalescer::Coalescer`] directly.

use std::time::{Duration, Instant};

use holler_body::reply_coalescer::{max_bytes, window, Coalescer, PushOutcome};

fn text_of(parts: &[holler_proto::a2a::Part]) -> String {
    parts.iter().filter_map(|p| p.text()).collect::<Vec<_>>().join("")
}

#[test]
fn single_chunk_buffered() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    assert_eq!(c.push("hello", t0), PushOutcome::Buffered);
    assert!(!c.is_empty());
    // Not due immediately — the window has not elapsed yet.
    assert!(!c.due(t0));
}

#[test]
fn window_from_first_chunk() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    c.push("a", t0);
    let deadline_after_first = c.next_deadline().expect("buffered has a deadline");
    // A second chunk arriving mid-window does not push the deadline out.
    c.push("b", t0 + Duration::from_millis(10));
    assert_eq!(c.next_deadline(), Some(deadline_after_first));
    assert_eq!(deadline_after_first, t0 + window());
}

#[test]
fn open_window_not_released_early() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    c.push("a", t0);
    // One tick before the deadline: still not due.
    let just_before = t0 + window() - Duration::from_millis(1);
    assert!(!c.due(just_before));
}

#[test]
fn release_after_window() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    c.push("a", t0);
    let at_deadline = t0 + window();
    assert!(c.due(at_deadline));
    let past_deadline = t0 + window() + Duration::from_millis(5);
    assert!(c.due(past_deadline));
}

#[test]
fn order_preserved() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    c.push("one-", t0);
    c.push("two-", t0);
    c.push("three", t0);
    let parts = c.flush().expect("non-empty flush");
    assert_eq!(text_of(&parts), "one-two-three");
}

#[test]
fn byte_cap_flushes_immediately_including_trigger() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    let filler = "x".repeat(max_bytes() - 3);
    assert_eq!(c.push(&filler, t0), PushOutcome::Buffered);
    // This chunk crosses the cap; the flush the caller takes afterward must
    // include it, not just what was buffered before it.
    assert_eq!(c.push("abc", t0), PushOutcome::CapHit);
    let parts = c.flush().expect("cap flush is non-empty");
    assert_eq!(text_of(&parts), format!("{filler}abc"));
}

#[test]
fn capped_flush_opens_fresh_window() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    let filler = "x".repeat(max_bytes());
    assert_eq!(c.push(&filler, t0), PushOutcome::CapHit);
    c.flush();
    assert_eq!(c.next_deadline(), None); // idle right after the capped flush.
    let t1 = t0 + Duration::from_millis(200);
    c.push("fresh", t1);
    assert_eq!(c.next_deadline(), Some(t1 + window()));
}

#[test]
fn turn_end_flushes_stragglers() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    c.push("straggler", t0);
    // The window has not elapsed — a window-driven caller would not flush
    // yet, but the turn-end path always calls flush() regardless of `due`.
    assert!(!c.due(t0));
    let parts = c.flush().expect("turn-end flush drains whatever is left");
    assert_eq!(text_of(&parts), "straggler");
}

#[test]
fn idle_end_is_noop() {
    let mut c = Coalescer::new();
    assert!(c.is_empty());
    assert_eq!(c.flush(), None);
}

#[test]
fn no_text_lost_across_turn() {
    let mut c = Coalescer::new();
    let mut t = Instant::now();
    let mut sent = String::new();
    let mut received = String::new();
    for i in 0..500 {
        let chunk = format!("chunk-{i}-");
        sent.push_str(&chunk);
        if c.push(&chunk, t) == PushOutcome::CapHit {
            received.push_str(&text_of(&c.flush().expect("cap flush is non-empty")));
        }
        t += Duration::from_millis(1);
        if c.due(t) {
            if let Some(parts) = c.flush() {
                received.push_str(&text_of(&parts));
            }
        }
    }
    // Turn end: drain whatever straggled past the last window/cap flush.
    if let Some(parts) = c.flush() {
        received.push_str(&text_of(&parts));
    }
    assert_eq!(received, sent);
}

#[test]
fn many_updates_fewer_frames_e2e() {
    let mut c = Coalescer::new();
    let t0 = Instant::now();
    let mut frames = 0usize;
    // 200 tiny chunks, all arriving well inside one window (1ms apart, a 50ms
    // default window) — they must coalesce into a single flush, not 200.
    for i in 0..200 {
        let t = t0 + Duration::from_micros(i as u64 * 100);
        c.push("x", t);
    }
    let due_at = t0 + window() + Duration::from_millis(1);
    if c.due(due_at) && c.flush().is_some() {
        frames += 1;
    }
    assert_eq!(frames, 1, "200 chunks inside one window must coalesce into exactly one frame");
}

#[test]
fn next_deadline_none_when_idle() {
    let c = Coalescer::new();
    assert_eq!(c.next_deadline(), None);
}

/// Issue #297: `no_text_lost_across_turn` above proves correctness at 500
/// chunks; this drives a full order of magnitude further (1000) to prove the
/// coalescer holds up under the volume a real long turn can actually
/// produce, and adds the two properties that test doesn't check — every
/// chunk flushed exactly once (no duplication, not just no loss) and the
/// buffer never grows past the byte cap between flushes (bounded per-session
/// state over the whole turn), on top of the same "every byte flushed,
/// order preserved" proof.
#[test]
fn thousand_chunks_all_flushed_exactly_once_in_order_state_stays_bounded() {
    let mut c = Coalescer::new();
    let mut t = Instant::now();
    let mut sent = String::new();
    let mut received = String::new();
    let mut flush_count = 0usize;
    let mut max_flush_bytes = 0usize;
    for i in 0..1000 {
        let chunk = format!("chunk-{i}-");
        sent.push_str(&chunk);
        if c.push(&chunk, t) == PushOutcome::CapHit {
            let parts = c.flush().expect("cap flush is non-empty");
            let text = text_of(&parts);
            max_flush_bytes = max_flush_bytes.max(text.len());
            received.push_str(&text);
            flush_count += 1;
        }
        t += Duration::from_millis(1);
        if c.due(t) {
            if let Some(parts) = c.flush() {
                let text = text_of(&parts);
                max_flush_bytes = max_flush_bytes.max(text.len());
                received.push_str(&text);
                flush_count += 1;
            }
        }
    }
    // Turn end: drain whatever straggled past the last window/cap flush.
    if let Some(parts) = c.flush() {
        received.push_str(&text_of(&parts));
        flush_count += 1;
    }

    assert_eq!(
        received, sent,
        "every one of 1000 streamed chunks must be flushed, in streaming order, with none dropped"
    );
    // Reassembling by naive concatenation (above) already proves nothing was
    // duplicated too: a duplicated chunk would make `received` longer than
    // `sent` even if every char in `sent` also appears in `received`, and
    // `assert_eq!` on the full strings catches that — an extra copy of any
    // `chunk-N-` shifts every following comparison out of alignment.
    assert!(flush_count > 1, "1000 chunks spanning many windows must not collapse into a single flush");
    assert!(
        max_flush_bytes <= max_bytes(),
        "a single flush must never exceed the byte cap — the buffer is drained on every trigger \
         (window, cap, or turn-end), so per-session state never grows unboundedly over the course \
         of the turn (max_flush_bytes={max_flush_bytes}, cap={})",
        max_bytes()
    );
}

#[test]
fn never_merges_two_sessions() {
    // Each session owns its own `Coalescer` instance — nothing is shared, so
    // pushing to one never leaks into the other's buffer.
    let mut alpha = Coalescer::new();
    let mut beta = Coalescer::new();
    let t0 = Instant::now();
    alpha.push("alpha-text", t0);
    beta.push("beta-text", t0);
    let alpha_parts = alpha.flush().expect("alpha has buffered text");
    let beta_parts = beta.flush().expect("beta has buffered text");
    assert_eq!(text_of(&alpha_parts), "alpha-text");
    assert_eq!(text_of(&beta_parts), "beta-text");
}

#[test]
fn sibling_end_does_not_disturb() {
    // Ending (flushing/draining) one session's coalescer at turn-end must not
    // touch a sibling session's still-open window.
    let mut alpha = Coalescer::new();
    let mut beta = Coalescer::new();
    let t0 = Instant::now();
    alpha.push("alpha-chunk", t0);
    beta.push("beta-chunk", t0);
    // Alpha's turn ends first.
    let alpha_parts = alpha.flush().expect("alpha turn-end flush");
    assert_eq!(text_of(&alpha_parts), "alpha-chunk");
    // Beta's window/buffer is untouched by alpha's end.
    assert!(!beta.is_empty());
    assert_eq!(beta.next_deadline(), Some(t0 + window()));
    let beta_parts = beta.flush().expect("beta still has its own buffered text");
    assert_eq!(text_of(&beta_parts), "beta-chunk");
}
