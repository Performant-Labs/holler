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

use std::collections::{BTreeSet, HashMap};
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
    /// Each counted failure (bounded by [`MAX_RECORDED_REASONS`]): its reason
    /// code, so the trip can be logged with *why* the peer was locked out
    /// (issue #450), and the token id the peer named, so `hub status` can say
    /// which credentials it is failing with (issue #451).
    failures: Vec<Failure>,
}

/// One counted failure, as remembered for reporting.
#[derive(Debug, Clone)]
struct Failure {
    reason: &'static str,
    /// The token id the peer named in `circuit/authenticate`: unauthenticated
    /// client input, so it is clipped to [`MAX_TOKEN_ID_BYTES`] before it is
    /// stored. Empty when the failure named none.
    token_id: String,
}

/// Cap on the per-peer failure list. Only failures that count towards the trip
/// are recorded (one that arrives while the peer is already locked out just
/// restarts the cooldown), so the cap bites when the trip limit exceeds it;
/// the oldest entries are dropped first.
const MAX_RECORDED_REASONS: usize = 16;

/// The longest token id kept per recorded failure, in bytes. A minted id
/// (`tok_` plus 64 hex digits, see `token::mint_id`) is 68 bytes and is never
/// clipped; a clipped id is longer than any real one, so it cannot equal one
/// and never resolves to a label.
const MAX_TOKEN_ID_BYTES: usize = 128;

/// `id` cut to at most [`MAX_TOKEN_ID_BYTES`], on a character boundary.
fn clip_token_id(id: &str) -> &str {
    let mut end = id.len().min(MAX_TOKEN_ID_BYTES);
    while !id.is_char_boundary(end) {
        end -= 1;
    }
    &id[..end]
}

impl PeerState {
    /// Whether this entry still counts at `now`: a tripped peer until its
    /// cooldown lapses, an accumulating peer until its window lapses. The rule
    /// [`Lockout::drop_stale`] removes by and [`Lockout::snapshot`] hides by, so
    /// a snapshot never shows what the next `sweep` would drop.
    fn is_live(&self, now: u64, limits: LockoutLimits) -> bool {
        match self.tripped_since {
            Some(since) => now.saturating_sub(since) < limits.duration_ms,
            None => now.saturating_sub(self.window_start) < limits.window_ms,
        }
    }
}

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
        let o = self.record_failure_detailed(peer, "unspecified", "");
        o.locked_out
    }

    /// [`Self::record_failure`], reporting what happened (issue #450): the
    /// failure count in the window, whether this failure newly tripped the
    /// lockout, the recorded reasons, and any peers whose cooldown lapsed.
    /// `token_id` is the id the peer named (issue #451), remembered for
    /// `hub status` and clipped to [`MAX_TOKEN_ID_BYTES`]; pass an empty
    /// string when there was none.
    pub fn record_failure_detailed(&self, peer: &IpAddr, reason: &'static str, token_id: &str) -> FailureOutcome {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let cleared = Self::drop_stale(&mut map, now, self.limits);

        let entry = map.entry(*peer).or_insert(PeerState {
            window_start: now,
            count: 0,
            tripped_since: None,
            failures: Vec::new(),
        });

        let mut newly_tripped = false;
        if entry.tripped_since.is_some() {
            entry.tripped_since = Some(now);
        } else {
            if now.saturating_sub(entry.window_start) >= self.limits.window_ms {
                entry.window_start = now;
                entry.count = 1;
                entry.failures.clear();
            } else {
                entry.count += 1;
            }
            if entry.failures.len() >= MAX_RECORDED_REASONS {
                entry.failures.remove(0);
            }
            entry.failures.push(Failure { reason, token_id: clip_token_id(token_id).to_owned() });
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
            reasons: entry.failures.iter().map(|f| f.reason).collect(),
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

    /// A read-only view of the peers this lockout is tracking right now, for
    /// `hub status` (issue #451): who is locked out and for how much longer,
    /// and who has failures building up towards a trip.
    ///
    /// A pure read under the mutex. An entry whose cooldown or window has
    /// lapsed is left out by the rule [`Self::sweep`] drops it by, but it is
    /// never removed here, so looking at the state cannot swallow the
    /// `lockout_cleared why=expired` event `sweep` owes the log (issue #450).
    pub fn snapshot(&self) -> LockoutSnapshot {
        let now = self.clock.now_ms();
        let map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let mut peers: Vec<PeerRow> = map
            .iter()
            .filter(|(_, s)| s.is_live(now, self.limits))
            .map(|(ip, s)| PeerRow::of(ip, s, now, self.limits))
            .collect();
        peers.sort_by(|a, b| a.peer.cmp(&b.peer));
        LockoutSnapshot { peers }
    }

    /// Remove tripped peers whose cooldown has lapsed and accumulating peers
    /// whose window has lapsed with no trip; return the former.
    fn drop_stale(map: &mut HashMap<IpAddr, PeerState>, now: u64, limits: LockoutLimits) -> Vec<IpAddr> {
        let mut cleared = Vec::new();
        map.retain(|ip, s| {
            let live = s.is_live(now, limits);
            if !live && s.tripped_since.is_some() {
                cleared.push(*ip);
            }
            live
        });
        cleared
    }
}

/// Group reason codes into `(code, count)` pairs, in the order each code first
/// appears. The one grouping shared by the `lockout_tripped` log line
/// (`token_expiredx3`) and `hub status` (issue #451).
pub fn group_reasons(reasons: &[&'static str]) -> Vec<(&'static str, usize)> {
    let mut grouped: Vec<(&'static str, usize)> = Vec::new();
    for &reason in reasons {
        match grouped.iter_mut().find(|(seen, _)| *seen == reason) {
            Some((_, n)) => *n += 1,
            None => grouped.push((reason, 1)),
        }
    }
    grouped
}

/// A point-in-time, read-only view of the lockout state (issue #451), taken by
/// [`Lockout::snapshot`]: every peer that is locked out or has failures
/// accumulating in its window, sorted by peer address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockoutSnapshot {
    peers: Vec<PeerRow>,
}

/// One peer's row in a [`LockoutSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct PeerRow {
    peer: String,
    locked_out: bool,
    failures: u64,
    retry_after_secs: u64,
    reasons: Vec<(&'static str, usize)>,
    token_ids: Vec<String>,
}

impl PeerRow {
    /// The row for a peer that [`PeerState::is_live`] at `now`.
    fn of(ip: &IpAddr, s: &PeerState, now: u64, limits: LockoutLimits) -> Self {
        let retry_after_ms =
            s.tripped_since.map_or(0, |since| limits.duration_ms.saturating_sub(now.saturating_sub(since)));
        let reasons: Vec<&'static str> = s.failures.iter().map(|f| f.reason).collect();
        let token_ids: BTreeSet<&str> =
            s.failures.iter().map(|f| f.token_id.as_str()).filter(|id| !id.is_empty()).collect();
        Self {
            peer: ip.to_string(),
            locked_out: s.tripped_since.is_some(),
            failures: s.count,
            // Rounded up: a peer that is still locked out never reads 0.
            retry_after_secs: retry_after_ms.div_ceil(1000),
            reasons: group_reasons(&reasons),
            token_ids: token_ids.into_iter().map(str::to_owned).collect(),
        }
    }

    fn to_json(&self, labels: &HashMap<String, String>) -> serde_json::Value {
        let reasons: serde_json::Map<String, serde_json::Value> =
            self.reasons.iter().map(|(code, n)| ((*code).to_owned(), serde_json::Value::from(*n))).collect();
        let token_ids: Vec<serde_json::Value> =
            self.token_ids.iter().map(|id| serde_json::json!({ "id": id, "label": labels.get(id) })).collect();
        serde_json::json!({
            "peer": self.peer,
            "locked_out": self.locked_out,
            "failures": self.failures,
            "retry_after_secs": self.retry_after_secs,
            "reasons": reasons,
            "token_ids": token_ids,
        })
    }
}

impl LockoutSnapshot {
    /// No peer is locked out or failing.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// The `lockout` document of `hub status --json`: `{"peers": [...]}`,
    /// sorted by peer address. Each entry is `{peer, locked_out, failures,
    /// retry_after_secs, reasons: {code: count}, token_ids: [{id, label}]}`;
    /// `retry_after_secs` is 0 unless the peer is locked out. `labels` maps a
    /// token id to its label, and an id it does not hold (a token that is gone,
    /// or one the peer made up) gets `label: null`.
    pub fn to_json(&self, labels: &HashMap<String, String>) -> serde_json::Value {
        let peers: Vec<serde_json::Value> = self.peers.iter().map(|p| p.to_json(labels)).collect();
        serde_json::json!({ "peers": peers })
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
            let o = lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
            assert_eq!((o.count, o.max, o.newly_tripped, o.locked_out), (n, 5, false, false));
        }
        let o = lockout.record_failure_detailed(&ip, "bad_proof", "tok_a");
        assert!(o.newly_tripped && o.locked_out, "the 5th failure trips");
        assert_eq!(o.reasons, vec!["token_expired", "token_expired", "token_expired", "token_expired", "bad_proof"]);
        assert_eq!((o.duration_ms, o.retry_after_ms), (10_000, 10_000));
        let again = lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
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
            lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        }
        assert!(lockout.sweep().is_empty(), "still locked out: nothing has lapsed");
        clock.advance(10_000);
        assert_eq!(lockout.sweep(), vec![ip], "the lapsed cooldown is reported");
        assert!(lockout.sweep().is_empty(), "and only once");
        assert!(!lockout.is_locked_out(&ip));
        assert_eq!(lockout.record_failure_detailed(&ip, "token_expired", "tok_a").count, 1, "a fresh window");
    }

    /// Issue #450: `reset` says whether it lifted a real lockout, not just
    /// mere accumulating strikes.
    #[test]
    fn reset_reports_only_a_lifted_lockout() {
        let lockout = lockout_with(FakeClock::new());
        let ip = peer();
        lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        assert!(!lockout.reset(&ip), "one strike is not a lockout");
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        }
        assert!(lockout.reset(&ip), "a tripped peer's lockout is lifted");
    }

    // ---- Issue #451: read-only snapshot for `hub status` ----

    use serde_json::{json, Value};

    fn no_labels() -> HashMap<String, String> {
        HashMap::new()
    }

    fn peers_of(l: &Lockout, labels: &HashMap<String, String>) -> Vec<Value> {
        l.snapshot().to_json(labels)["peers"].as_array().unwrap().clone()
    }

    #[test]
    fn snapshot_of_a_quiet_hub_is_an_empty_peers_list() {
        let lockout = lockout_with(FakeClock::new());
        assert_eq!(lockout.snapshot().to_json(&no_labels()), json!({ "peers": [] }));
        assert!(lockout.snapshot().is_empty());
    }

    #[test]
    fn snapshot_shows_an_accumulating_peer_as_not_locked_out() {
        let lockout = lockout_with(FakeClock::new());
        let ip = peer();
        lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        lockout.record_failure_detailed(&ip, "token_unknown", "tok_b");
        let peers = peers_of(&lockout, &no_labels());
        assert_eq!(peers.len(), 1);
        let p = &peers[0];
        assert_eq!(p["peer"], "127.0.0.1");
        assert_eq!(p["locked_out"], false);
        assert_eq!(p["failures"], 2);
        assert_eq!(p["retry_after_secs"], 0);
        assert_eq!(p["reasons"], json!({ "token_expired": 1, "token_unknown": 1 }));
        assert_eq!(p["token_ids"], json!([{"id": "tok_a", "label": null}, {"id": "tok_b", "label": null}]));
    }

    #[test]
    fn snapshot_shows_a_tripped_peer_with_the_remaining_cooldown_rounded_up() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let ip = peer();
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        }
        clock.advance(3_500); // 6_500 ms remain -> 7 s
        let labels = HashMap::from([("tok_a".to_string(), "body-1".to_string())]);
        let p = &peers_of(&lockout, &labels)[0];
        assert_eq!(p["locked_out"], true);
        assert_eq!(p["failures"], 5);
        assert_eq!(p["retry_after_secs"], 7);
        assert_eq!(p["reasons"], json!({ "token_expired": 5 }));
        assert_eq!(p["token_ids"], json!([{"id": "tok_a", "label": "body-1"}]), "distinct ids only");
    }

    #[test]
    fn retry_after_secs_is_never_zero_while_locked_out() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let ip = peer();
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        }
        assert_eq!(peers_of(&lockout, &no_labels())[0]["retry_after_secs"], 10);
        clock.advance(9_999); // 1 ms remains
        let p = &peers_of(&lockout, &no_labels())[0];
        assert_eq!((p["locked_out"].clone(), p["retry_after_secs"].clone()), (json!(true), json!(1)));
    }

    #[test]
    fn a_lapsed_cooldown_or_window_no_longer_appears_in_the_snapshot() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let tripped: IpAddr = "10.0.0.1".parse().unwrap();
        let accumulating: IpAddr = "10.0.0.2".parse().unwrap();
        for _ in 0..5 {
            lockout.record_failure_detailed(&tripped, "token_expired", "tok_a");
        }
        lockout.record_failure_detailed(&accumulating, "token_expired", "tok_a");
        clock.advance(10_000);
        assert!(peers_of(&lockout, &no_labels()).is_empty(), "both the cooldown and the window have lapsed");
    }

    /// The snapshot is a pure read: it must not consume the lapse that
    /// `sweep` reports as `lockout_cleared why=expired` (#450).
    #[test]
    fn taking_a_snapshot_does_not_consume_the_sweep_clear_event() {
        let clock = FakeClock::new();
        let lockout = lockout_with(clock.clone());
        let ip = peer();
        for _ in 0..5 {
            lockout.record_failure_detailed(&ip, "token_expired", "tok_a");
        }
        clock.advance(10_000);
        assert!(peers_of(&lockout, &no_labels()).is_empty());
        assert_eq!(lockout.sweep(), vec![ip], "sweep still reports the lapsed peer after a snapshot");
    }

    #[test]
    fn snapshot_peers_are_sorted_by_address() {
        let lockout = lockout_with(FakeClock::new());
        for ip in ["10.0.0.9", "10.0.0.10", "10.0.0.2"] {
            lockout.record_failure_detailed(&ip.parse().unwrap(), "token_expired", "tok_a");
        }
        let order: Vec<String> = peers_of(&lockout, &no_labels()).iter().map(|p| p["peer"].as_str().unwrap().to_string()).collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted, "deterministic order by peer string");
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn recorded_token_ids_stay_bounded_by_the_reason_cap() {
        // Trip limit above the cap so all 40 failures are recorded, not just those before the trip.
        let lockout = Arc::new(Lockout {
            limits: LockoutLimits { max_failures: 100, window_ms: 10_000, duration_ms: 10_000 },
            peers: Mutex::new(HashMap::new()),
            clock: Box::new(FakeClock::new()),
        });
        let ip = peer();
        let ids: Vec<String> = (0..40).map(|n| format!("tok_{n:02}")).collect();
        for id in &ids {
            lockout.record_failure_detailed(&ip, "token_unknown", id);
        }
        let p = &peers_of(&lockout, &no_labels())[0];
        let listed = p["token_ids"].as_array().unwrap();
        assert!(listed.len() <= MAX_RECORDED_REASONS, "unauthenticated ids are capped: {}", listed.len());
        assert!(listed.iter().any(|v| v["id"] == "tok_39"), "the most recent id is kept");
        assert!(!listed.iter().any(|v| v["id"] == "tok_00"), "the oldest id is dropped");
    }
}
