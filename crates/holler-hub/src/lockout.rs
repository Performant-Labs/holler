//! Failed-authentication lockout (issues #184, #455).
//!
//! After [`DEFAULT_MAX_FAILURES`] failed authentications (`circuit/authenticate`
//! rejected) within [`DEFAULT_WINDOW_MS`] ms, a lockout refuses (close
//! **1008**) for [`DEFAULT_DURATION_MS`] ms. The tunables are read from the
//! environment (`HOLLER_LOCKOUT_MAX_FAILURES`, `HOLLER_LOCKOUT_WINDOW_MS`,
//! `HOLLER_LOCKOUT_DURATION_MS`), defaulting to 5 / 10 min / 10 min.
//!
//! **The key (issue #455)** is the peer's transport IP address and the token
//! id it claimed, so one token's failures lock out only that token from that
//! address: behind a reverse proxy (ADR 0006) every body shares the proxy's
//! address. Both are abuse-control inputs, never identity: the address is the
//! accept-time peer (no forwarded-address header is ever read), and the id is
//! unauthenticated input that only partitions a counter (clipped to
//! [`MAX_TOKEN_ID_BYTES`]); the claimed hostname is never part of the key.
//! The #184 flood guard stays: the failure that would create an address's
//! [`MAX_TOKEN_IDS_PER_PEER`]th live token bucket folds its buckets into one
//! **peer-wide** entry, tripped at once, which refuses every id from that
//! address and bounds the map. A peer-wide lockout refuses a new socket before
//! a frame is read ([`Lockout::is_locked_out`]); a token lockout, once the
//! peer names the token ([`Lockout::is_token_locked_out`]).
//!
//! `PeerState::tripped_since` is carried over from PR #289, which fixed an
//! earlier version that inferred "tripped" from map membership and so refused
//! a peer right after its **first** failure, long before `max_failures`.
//! Loopback is **not** exempt (the spec is explicit, and the test suite
//! relies on it).

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Default: how many auth failures within the window trip the lockout.
pub const DEFAULT_MAX_FAILURES: u64 = 5;
/// Default: the window (ms) in which failures are counted.
pub const DEFAULT_WINDOW_MS: u64 = 10 * 60 * 1000; // 10 min
/// Default: the duration (ms) a tripped entry is refused.
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

/// One lockout entry's failure-accumulation / trip state: a token bucket, or
/// an address's peer-wide entry (issue #455). `tripped_since` is `None` while
/// failures merely accumulate in the current window — distinct from `count`
/// reaching `max_failures`, which is when the entry actually trips. A
/// peer-wide entry is tripped from its creation.
#[derive(Debug, Clone)]
struct PeerState {
    window_start: u64,
    count: u64,
    tripped_since: Option<u64>,
    /// Each counted failure (bounded by [`MAX_RECORDED_REASONS`]): its reason
    /// code, so the trip can be logged with *why* it was locked out (issue
    /// #450), and the token id the peer named, so `hub status` can say which
    /// credentials it is failing with (issue #451).
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

/// Cap on an entry's failure list. Only failures that count towards the trip
/// are recorded (one that arrives while the entry is already locked out just
/// restarts the cooldown), so the cap bites when the trip limit exceeds it, or
/// when token buckets fold into a peer-wide entry; the oldest entries are
/// dropped first.
const MAX_RECORDED_REASONS: usize = 16;

/// The longest token id kept per recorded failure, in bytes. A minted id
/// (`tok_` plus 64 hex digits, see `token::mint_id`) is 68 bytes and is never
/// clipped; a clipped id is longer than any real one, so it cannot equal one
/// and never resolves to a label.
const MAX_TOKEN_ID_BYTES: usize = 128;

/// The failure that would open an address's `MAX_TOKEN_IDS_PER_PEER`th live
/// token bucket trips the whole address instead (issue #455), which bounds an
/// id-cycling client to `8 × max_failures` counted failures per window. Fixed,
/// deliberately not an environment variable.
const MAX_TOKEN_IDS_PER_PEER: usize = 8;

/// `id` cut to at most [`MAX_TOKEN_ID_BYTES`], on a character boundary.
fn clip_token_id(id: &str) -> &str {
    let mut end = id.len().min(MAX_TOKEN_ID_BYTES);
    while !id.is_char_boundary(end) {
        end -= 1;
    }
    &id[..end]
}

impl PeerState {
    /// An entry first counted at `now`, with nothing counted yet.
    fn new(now: u64) -> Self {
        Self { window_start: now, count: 0, tripped_since: None, failures: Vec::new() }
    }

    /// Whether this entry still counts at `now`: a tripped entry until its
    /// cooldown lapses, an accumulating one until its window lapses. The rule
    /// [`Lockout::drop_stale`] removes by and [`Lockout::snapshot`] hides by, so
    /// a snapshot never shows what the next `sweep` would drop.
    fn is_live(&self, now: u64, limits: LockoutLimits) -> bool {
        match self.tripped_since {
            Some(since) => now.saturating_sub(since) < limits.duration_ms,
            None => now.saturating_sub(self.window_start) < limits.window_ms,
        }
    }

    /// Whether this entry refuses at `now`: tripped, and its cooldown not yet
    /// elapsed. Accumulating failures never refuse (the PR #289 rule).
    fn refuses(&self, now: u64, limits: LockoutLimits) -> bool {
        self.tripped_since.is_some_and(|since| now.saturating_sub(since) < limits.duration_ms)
    }

    /// The time left on the cooldown at `now`, in milliseconds; 0 unless tripped.
    fn retry_after_ms(&self, now: u64, limits: LockoutLimits) -> u64 {
        self.tripped_since.map_or(0, |since| limits.duration_ms.saturating_sub(now.saturating_sub(since)))
    }

    /// Count `failure` against this entry at `now`; `true` if it tripped the
    /// entry. A failure while already tripped only restarts the cooldown.
    fn count_failure(&mut self, failure: Failure, now: u64, limits: LockoutLimits) -> bool {
        if self.tripped_since.is_some() {
            self.tripped_since = Some(now);
            return false;
        }
        if now.saturating_sub(self.window_start) >= limits.window_ms {
            self.window_start = now;
            self.count = 1;
            self.failures.clear();
        } else {
            self.count += 1;
        }
        if self.failures.len() >= MAX_RECORDED_REASONS {
            self.failures.remove(0);
        }
        self.failures.push(failure);
        if self.count >= limits.max_failures {
            self.tripped_since = Some(now);
            return true;
        }
        false
    }
}

/// Everything the lockout holds for one address (issue #455): its token
/// buckets, or the one peer-wide entry they folded into. Never both, so no
/// bucket can be created while the peer-wide entry is live.
#[derive(Debug)]
enum PeerEntry {
    /// Clipped token id ([`clip_token_id`]) → that token's bucket.
    Tokens(HashMap<String, PeerState>),
    /// The whole address, tripped from its creation.
    Wide(PeerState),
}

impl PeerEntry {
    /// Count `failure` (its token id already clipped) against this address
    /// at `now`. The one place a bucket is created, and the one place buckets
    /// fold into the peer-wide entry.
    fn record(&mut self, failure: Failure, now: u64, limits: LockoutLimits) -> FailureOutcome {
        match self {
            // A failure while the peer-wide entry is live restarts its cooldown.
            Self::Wide(wide) => {
                let newly_tripped = wide.count_failure(failure, now, limits);
                FailureOutcome::of(wide, newly_tripped, true, now, limits)
            }
            Self::Tokens(buckets)
                if buckets.contains_key(&failure.token_id) || buckets.len() + 1 < MAX_TOKEN_IDS_PER_PEER =>
            {
                let bucket = buckets.entry(failure.token_id.clone()).or_insert_with(|| PeerState::new(now));
                let newly_tripped = bucket.count_failure(failure, now, limits);
                FailureOutcome::of(bucket, newly_tripped, false, now, limits)
            }
            // This failure would create the address's MAX_TOKEN_IDS_PER_PEER-th bucket.
            Self::Tokens(buckets) => {
                let wide = fold_buckets(std::mem::take(buckets), failure, now);
                let outcome = FailureOutcome::of(&wide, true, true, now, limits);
                *self = Self::Wide(wide);
                outcome
            }
        }
    }
}

/// Fold an address's token buckets, and the failure that would have opened
/// its [`MAX_TOKEN_IDS_PER_PEER`]th, into one peer-wide entry tripped at `now`
/// (issue #455): the counts summed, the failure lists concatenated bucket by
/// bucket (oldest window first, then by id, so map order never shows) with the
/// tripping failure last, capped at [`MAX_RECORDED_REASONS`] (oldest dropped).
fn fold_buckets(buckets: HashMap<String, PeerState>, failure: Failure, now: u64) -> PeerState {
    let mut buckets: Vec<(String, PeerState)> = buckets.into_iter().collect();
    buckets.sort_by(|(a_id, a), (b_id, b)| (a.window_start, a_id).cmp(&(b.window_start, b_id)));
    let count = buckets.iter().fold(1u64, |sum, (_, s)| sum.saturating_add(s.count));
    let mut failures: Vec<Failure> = buckets.into_iter().flat_map(|(_, s)| s.failures).collect();
    failures.push(failure);
    let excess = failures.len().saturating_sub(MAX_RECORDED_REASONS);
    PeerState { window_start: now, count, tripped_since: Some(now), failures: failures.split_off(excess) }
}

/// What a lockout entry covers (issue #455): one token id from one address,
/// or the whole address; [`Scope::as_str`] names it in `hub status` and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Token,
    Peer,
}

impl Scope {
    /// `Peer` for the whole address, else `Token`.
    pub(crate) fn of(peer_wide: bool) -> Self {
        if peer_wide { Self::Peer } else { Self::Token }
    }

    /// The scope's name: `token` or `peer`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Peer => "peer",
        }
    }
}

/// The key of one lockout entry (issue #455): the address, and the token id
/// it claimed for a token bucket, or `None` for the address's peer-wide
/// entry. What [`Lockout::sweep`] and [`FailureOutcome::cleared`] report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockoutKey {
    pub peer: IpAddr,
    pub token_id: Option<String>,
}

/// The peer, then ` token_id=<id>` for a token bucket. The id is
/// unauthenticated client input, so every char that is not printable ASCII
/// is shown as `?`: a key can be put in a log line as it is.
impl fmt::Display for LockoutKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(id) = &self.token_id else { return write!(f, "{}", self.peer) };
        let shown: String = id.chars().map(|c| if c.is_ascii_graphic() { c } else { '?' }).collect();
        write!(f, "{} token_id={shown}", self.peer)
    }
}

/// What one recorded failure did to its lockout entry (issue #450).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureOutcome {
    /// Failures counted in the current window, including this one. For a
    /// peer-wide entry, its folded buckets' counts plus the tripping failure.
    pub count: u64,
    /// The limit at which a token bucket trips.
    pub max: u64,
    /// This failure moved the entry from accumulating to locked out, or
    /// created the tripped peer-wide entry.
    pub newly_tripped: bool,
    /// The entry is locked out after this failure (newly or already).
    pub locked_out: bool,
    /// The failure landed on (or created) the address's peer-wide entry (#455).
    pub peer_wide: bool,
    /// The reasons of the failures counted so far, oldest first.
    pub reasons: Vec<&'static str>,
    /// Cooldown length, and how long until it lapses, in milliseconds.
    pub duration_ms: u64,
    pub retry_after_ms: u64,
    /// Entries whose cooldown had lapsed and were dropped by this call.
    pub cleared: Vec<LockoutKey>,
}

impl FailureOutcome {
    /// The outcome of a failure that left `state` as it is (`cleared` is the caller's).
    fn of(state: &PeerState, newly_tripped: bool, peer_wide: bool, now: u64, limits: LockoutLimits) -> Self {
        Self {
            count: state.count,
            max: limits.max_failures,
            newly_tripped,
            locked_out: state.tripped_since.is_some(),
            peer_wide,
            reasons: state.failures.iter().map(|f| f.reason).collect(),
            duration_ms: limits.duration_ms,
            retry_after_ms: state.retry_after_ms(now, limits),
            cleared: Vec::new(),
        }
    }
}

/// The shared lockout state.
pub struct Lockout {
    limits: LockoutLimits,
    /// Peer IP → that address's token buckets or peer-wide entry (see
    /// [`PeerState`]'s own doc for why `tripped_since` must be an explicit
    /// field rather than inferred from map membership — the exact PR #289
    /// regression this design avoids).
    peers: Mutex<HashMap<IpAddr, PeerEntry>>,
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

    /// Whether `peer` is locked out as a whole: its peer-wide entry (issue
    /// #455) is tripped and its cooldown not yet elapsed. What a new socket
    /// is refused for before the hub reads a frame. A locked-out token bucket
    /// does not count here; see [`Self::is_token_locked_out`].
    pub fn is_locked_out(&self, peer: &IpAddr) -> bool {
        let now = self.clock.now_ms();
        let map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        matches!(map.get(peer), Some(PeerEntry::Wide(wide)) if wide.refuses(now, self.limits))
    }

    /// Whether `token_id` is locked out from `peer` (issue #455): its own
    /// bucket is tripped, or the whole address is. `token_id` is clipped the
    /// way it was when its failures were recorded. Accumulating failures that
    /// have not reached `max_failures` never lock anything out.
    pub fn is_token_locked_out(&self, peer: &IpAddr, token_id: &str) -> bool {
        let now = self.clock.now_ms();
        let map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(peer) {
            Some(PeerEntry::Wide(wide)) => wide.refuses(now, self.limits),
            Some(PeerEntry::Tokens(buckets)) => buckets.get(clip_token_id(token_id)).is_some_and(|b| b.refuses(now, self.limits)),
            None => false,
        }
    }

    /// Reset `token_id`'s bucket for `peer` (a successful authentication
    /// clears that token's in-window strikes, and lifts its lockout if it is
    /// locked out). Only that bucket: a success for one token never clears
    /// another's, nor the address's peer-wide entry (issue #455). Returns
    /// `true` if a lockout (a trip, not mere accumulating strikes) was lifted.
    pub fn reset(&self, peer: &IpAddr, token_id: &str) -> bool {
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let Some(PeerEntry::Tokens(buckets)) = map.get_mut(peer) else { return false };
        let removed = buckets.remove(clip_token_id(token_id));
        if buckets.is_empty() {
            map.remove(peer);
        }
        removed.is_some_and(|s| s.tripped_since.is_some())
    }

    /// Record an authentication failure for `token_id` from `peer`, and report
    /// what happened (issue #450): the failure count in the window, whether
    /// this failure newly tripped a lockout, the recorded reasons, and any
    /// entries whose cooldown lapsed. It counts against the token's bucket, or
    /// the address's peer-wide entry ([`PeerEntry::record`], issue #455).
    /// `token_id` is the id the peer named (issue #451), remembered for `hub
    /// status` and clipped to [`MAX_TOKEN_ID_BYTES`]; empty when there was none.
    pub fn record_failure_detailed(&self, peer: &IpAddr, reason: &'static str, token_id: &str) -> FailureOutcome {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let cleared = Self::drop_stale(&mut map, now, self.limits);
        let failure = Failure { reason, token_id: clip_token_id(token_id).to_owned() };
        let entry = map.entry(*peer).or_insert_with(|| PeerEntry::Tokens(HashMap::new()));
        FailureOutcome { cleared, ..entry.record(failure, now, self.limits) }
    }

    /// Drop lapsed entries now and report the keys whose *cooldown* lapsed
    /// (issue #450's `lockout_cleared reason=expired`). Called on every
    /// accepted connection, so a lapse is reported promptly even if the
    /// peer never returns.
    pub fn sweep(&self) -> Vec<LockoutKey> {
        let now = self.clock.now_ms();
        let mut map = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        Self::drop_stale(&mut map, now, self.limits)
    }

    /// A read-only view of the entries this lockout is tracking right now,
    /// for `hub status` (issue #451): which tokens (or whole addresses, issue
    /// #455) are locked out and for how much longer, and which have failures
    /// building up towards a trip.
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
            .flat_map(|(ip, entry)| match entry {
                PeerEntry::Wide(wide) => vec![(ip, Scope::Peer, wide)],
                PeerEntry::Tokens(buckets) => buckets.values().map(|s| (ip, Scope::Token, s)).collect(),
            })
            .filter(|(_, _, s)| s.is_live(now, self.limits))
            .map(|(ip, scope, s)| PeerRow::of(ip, scope, s, now, self.limits))
            .collect();
        peers.sort_by(|a, b| (&a.peer, a.scope.as_str(), &a.token_ids).cmp(&(&b.peer, b.scope.as_str(), &b.token_ids)));
        LockoutSnapshot { peers }
    }

    /// Remove tripped entries whose cooldown has lapsed and accumulating ones
    /// whose window has lapsed with no trip, and any address left with no
    /// entry; return the keys of the former.
    fn drop_stale(map: &mut HashMap<IpAddr, PeerEntry>, now: u64, limits: LockoutLimits) -> Vec<LockoutKey> {
        let mut cleared = Vec::new();
        let mut keep = |s: &PeerState, peer: IpAddr, token_id: Option<&String>| {
            let live = s.is_live(now, limits);
            if !live && s.tripped_since.is_some() {
                cleared.push(LockoutKey { peer, token_id: token_id.cloned() });
            }
            live
        };
        map.retain(|ip, entry| match entry {
            PeerEntry::Wide(wide) => keep(wide, *ip, None),
            PeerEntry::Tokens(buckets) => {
                buckets.retain(|id, s| keep(s, *ip, Some(id)));
                !buckets.is_empty()
            }
        });
        cleared
    }
}

/// Group reason codes into `(code, count)` pairs, in the order each code first
/// appears. The one grouping shared by the `lockout_tripped` log line
/// (`token_not_boundx3`) and `hub status` (issue #451).
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
/// [`Lockout::snapshot`]: every entry that is locked out or has failures
/// accumulating in its window, sorted by peer address, then scope, then token
/// id (issue #455).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockoutSnapshot {
    peers: Vec<PeerRow>,
}

/// One entry's row in a [`LockoutSnapshot`]: a token bucket or a peer-wide entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PeerRow {
    peer: String,
    scope: Scope,
    locked_out: bool,
    failures: u64,
    retry_after_secs: u64,
    reasons: Vec<(&'static str, usize)>,
    token_ids: Vec<String>,
}

impl PeerRow {
    /// The row for an entry that [`PeerState::is_live`] at `now`.
    fn of(ip: &IpAddr, scope: Scope, s: &PeerState, now: u64, limits: LockoutLimits) -> Self {
        let reasons: Vec<&'static str> = s.failures.iter().map(|f| f.reason).collect();
        let token_ids: BTreeSet<&str> =
            s.failures.iter().map(|f| f.token_id.as_str()).filter(|id| !id.is_empty()).collect();
        Self {
            peer: ip.to_string(),
            scope,
            locked_out: s.tripped_since.is_some(),
            failures: s.count,
            // Rounded up: an entry that is still locked out never reads 0.
            retry_after_secs: s.retry_after_ms(now, limits).div_ceil(1000),
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
            "scope": self.scope.as_str(),
            "locked_out": self.locked_out,
            "failures": self.failures,
            "retry_after_secs": self.retry_after_secs,
            "reasons": reasons,
            "token_ids": token_ids,
        })
    }
}

impl LockoutSnapshot {
    /// Nothing is locked out or failing.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// The `lockout` document of `hub status --json`: `{"peers": [...]}`, one
    /// entry per token bucket or peer-wide entry, sorted by peer address, then
    /// scope, then token id. Each is `{peer, scope, locked_out, failures,
    /// retry_after_secs, reasons: {code: count}, token_ids: [{id, label}]}`:
    /// `scope` is `token` (its `token_ids` is that one id) or `peer` (the whole
    /// address; its `token_ids` are the ids its folded failures named, issue
    /// #455), and `retry_after_secs` is 0 unless the entry is locked out.
    /// `labels` maps a token id to its label, and an id it does not hold (a
    /// token that is gone, or one the peer made up) gets `label: null`.
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
mod tests;
