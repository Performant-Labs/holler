//! Unit tests for the failed-authentication lockout (issues #184, #450, #451,
//! #455). Moved out of `lockout.rs` (issue #455) on the `holds/tests.rs`
//! pattern; the tests #455 changes on purpose say so in their own doc.

#![allow(clippy::unwrap_used)] // #184 — test-only fixture parsing (a fixed literal IP)

use super::*;
use serde_json::{json, Value};
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

/// A lockout with `limits` and an injected clock. `peers` is built with
/// `Default`, so the test does not depend on the map's internal shape.
fn lockout_limited(limits: LockoutLimits, clock: FakeClock) -> Arc<Lockout> {
    Arc::new(Lockout { limits, peers: Default::default(), clock: Box::new(clock) })
}

fn lockout_with(clock: FakeClock) -> Arc<Lockout> {
    lockout_limited(LockoutLimits { max_failures: 5, window_ms: 10_000, duration_ms: 10_000 }, clock)
}

fn peer() -> IpAddr {
    "127.0.0.1".parse().unwrap()
}

/// The one token id the single-token tests fail with.
const TOK: &str = "tok_a";

/// One counted failure for [`TOK`]; `true` if that token is now locked out.
fn fail(l: &Lockout, ip: &IpAddr) -> bool {
    l.record_failure_detailed(ip, "token_not_bound", TOK).locked_out
}

fn bucket(ip: IpAddr, id: &str) -> LockoutKey {
    LockoutKey { peer: ip, token_id: Some(id.to_string()) }
}

#[test]
fn a_single_failure_does_not_lock_out_the_peer() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();

    assert!(!lockout.is_token_locked_out(&ip, TOK), "fresh token is never locked out");
    assert!(!fail(&lockout, &ip), "1 failure (of 5 required) must not trip the lockout");
    assert!(!lockout.is_token_locked_out(&ip, TOK), "1 of 5 failures must not lock the token out");
}

#[test]
fn exactly_max_failures_are_required_to_trip() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();

    for i in 1..5 {
        assert!(!fail(&lockout, &ip), "failure {i} of 5 must not trip the lockout yet");
        assert!(!lockout.is_token_locked_out(&ip, TOK), "failure {i} of 5 must not lock the token out yet");
    }
    assert!(fail(&lockout, &ip), "the 5th failure must trip the lockout");
    assert!(lockout.is_token_locked_out(&ip, TOK), "a tripped token must be locked out");
}

/// Changed by #455: `reset` takes the token id.
#[test]
fn reset_clears_in_window_failures() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();

    for _ in 0..4 {
        assert!(!fail(&lockout, &ip));
    }
    lockout.reset(&ip, TOK);
    assert!(!lockout.is_token_locked_out(&ip, TOK));
    for _ in 0..4 {
        assert!(!fail(&lockout, &ip));
    }
    assert!(!lockout.is_token_locked_out(&ip, TOK));
}

#[test]
fn trip_lapses_after_duration_and_window_resets_after_window_ms() {
    let clock = FakeClock::new();
    let lockout = lockout_with(clock.clone());
    let ip = peer();

    for _ in 0..5 {
        fail(&lockout, &ip);
    }
    assert!(lockout.is_token_locked_out(&ip, TOK));
    clock.advance(10_001);
    assert!(!lockout.is_token_locked_out(&ip, TOK), "the trip must lapse after duration_ms");

    for _ in 0..4 {
        assert!(!fail(&lockout, &ip));
    }
    clock.advance(10_001);
    for _ in 0..4 {
        assert!(!fail(&lockout, &ip));
    }
    assert!(!lockout.is_token_locked_out(&ip, TOK));
}

/// Issue #450: the detailed outcome reports the in-window count, the
/// reasons, and exactly one `newly_tripped` at the trip.
#[test]
fn detailed_outcome_counts_reasons_and_reports_the_trip_once() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    for n in 1..5u64 {
        let o = lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
        assert_eq!((o.count, o.max, o.newly_tripped, o.locked_out), (n, 5, false, false));
    }
    let o = lockout.record_failure_detailed(&ip, "bad_proof", "tok_a");
    assert!(o.newly_tripped && o.locked_out, "the 5th failure trips");
    assert!(!o.peer_wide, "one token's failures trip only that token");
    assert_eq!(o.reasons, vec!["token_not_bound", "token_not_bound", "token_not_bound", "token_not_bound", "bad_proof"]);
    assert_eq!((o.duration_ms, o.retry_after_ms), (10_000, 10_000));
    let again = lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    assert!(again.locked_out && !again.newly_tripped, "a failure while locked out is not a second trip");
}

/// Issue #450: a lapsed cooldown is reported once, by `sweep`, and the
/// token starts a fresh window afterwards. Changed by #455: `sweep` returns
/// the [`LockoutKey`] of the lapsed bucket.
#[test]
fn a_lapsed_cooldown_is_reported_once_by_sweep() {
    let clock = FakeClock::new();
    let lockout = lockout_with(clock.clone());
    let ip = peer();
    for _ in 0..5 {
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    }
    assert!(lockout.sweep().is_empty(), "still locked out: nothing has lapsed");
    clock.advance(10_000);
    assert_eq!(lockout.sweep(), vec![bucket(ip, "tok_a")], "the lapsed cooldown is reported");
    assert!(lockout.sweep().is_empty(), "and only once");
    assert!(!lockout.is_token_locked_out(&ip, "tok_a"));
    assert_eq!(lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a").count, 1, "a fresh window");
}

/// Issue #450: `reset` says whether it lifted a real lockout, not just
/// mere accumulating strikes. Changed by #455: `reset` takes the token id.
#[test]
fn reset_reports_only_a_lifted_lockout() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    assert!(!lockout.reset(&ip, "tok_a"), "one strike is not a lockout");
    for _ in 0..5 {
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    }
    assert!(lockout.reset(&ip, "tok_a"), "a tripped token's lockout is lifted");
    assert!(!lockout.is_token_locked_out(&ip, "tok_a"));
}

// ---- Issue #451: read-only snapshot for `hub status` ----

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

/// Changed by #455: two token ids from one address are two token-scope
/// entries, one failure each.
#[test]
fn snapshot_shows_an_accumulating_peer_as_not_locked_out() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    lockout.record_failure_detailed(&ip, "token_unknown", "tok_b");
    let peers = peers_of(&lockout, &no_labels());
    assert_eq!(peers.len(), 2, "one entry per token id: {peers:?}");
    for (p, (id, reason)) in peers.iter().zip([("tok_a", "token_not_bound"), ("tok_b", "token_unknown")]) {
        assert_eq!(p["peer"], "127.0.0.1");
        assert_eq!(p["scope"], "token");
        assert_eq!(p["locked_out"], false);
        assert_eq!(p["failures"], 1);
        assert_eq!(p["retry_after_secs"], 0);
        assert_eq!(p["reasons"], json!({ reason: 1 }));
        assert_eq!(p["token_ids"], json!([{"id": id, "label": null}]));
    }
}

#[test]
fn snapshot_shows_a_tripped_peer_with_the_remaining_cooldown_rounded_up() {
    let clock = FakeClock::new();
    let lockout = lockout_with(clock.clone());
    let ip = peer();
    for _ in 0..5 {
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    }
    clock.advance(3_500); // 6_500 ms remain -> 7 s
    let labels = HashMap::from([("tok_a".to_string(), "body-1".to_string())]);
    let p = &peers_of(&lockout, &labels)[0];
    assert_eq!(p["locked_out"], true);
    assert_eq!(p["scope"], "token");
    assert_eq!(p["failures"], 5);
    assert_eq!(p["retry_after_secs"], 7);
    assert_eq!(p["reasons"], json!({ "token_not_bound": 5 }));
    assert_eq!(p["token_ids"], json!([{"id": "tok_a", "label": "body-1"}]), "distinct ids only");
}

#[test]
fn retry_after_secs_is_never_zero_while_locked_out() {
    let clock = FakeClock::new();
    let lockout = lockout_with(clock.clone());
    let ip = peer();
    for _ in 0..5 {
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
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
        lockout.record_failure_detailed(&tripped, "token_not_bound", "tok_a");
    }
    lockout.record_failure_detailed(&accumulating, "token_not_bound", "tok_a");
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
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_a");
    }
    clock.advance(10_000);
    assert!(peers_of(&lockout, &no_labels()).is_empty());
    assert_eq!(lockout.sweep(), vec![bucket(ip, "tok_a")], "sweep still reports the lapsed bucket after a snapshot");
}

/// Changed by #455: the sort key is (peer, scope, token id), so two ids from
/// one address are ordered too.
#[test]
fn snapshot_peers_are_sorted_by_address() {
    let lockout = lockout_with(FakeClock::new());
    for (ip, id) in [("10.0.0.9", "tok_b"), ("10.0.0.10", "tok_a"), ("10.0.0.2", "tok_a"), ("10.0.0.9", "tok_a")] {
        lockout.record_failure_detailed(&ip.parse().unwrap(), "token_not_bound", id);
    }
    let order: Vec<(String, String, String)> = peers_of(&lockout, &no_labels())
        .iter()
        .map(|p| {
            let s = |v: &Value| v.as_str().unwrap().to_string();
            (s(&p["peer"]), s(&p["scope"]), s(&p["token_ids"][0]["id"]))
        })
        .collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted, "deterministic order by (peer string, scope, id)");
    assert_eq!(order.len(), 4);
}

/// Changed by #455: many ids from one address now go through the peer-wide
/// entry, whose folded failure list is still capped.
#[test]
fn recorded_token_ids_stay_bounded_by_the_reason_cap() {
    // Trip limit above every count here, so only the id cap can trip.
    let limits = LockoutLimits { max_failures: 100, window_ms: 10_000, duration_ms: 10_000 };
    let lockout = lockout_limited(limits, FakeClock::new());
    let ip = peer();
    // 7 ids x 3 failures = 21 recorded failures across 7 buckets, then an 8th id folds them.
    for n in 0..7 {
        for _ in 0..3 {
            lockout.record_failure_detailed(&ip, "token_unknown", &format!("tok_{n:02}"));
        }
    }
    let o = lockout.record_failure_detailed(&ip, "token_unknown", "tok_07");
    assert!(o.peer_wide && o.newly_tripped, "the 8th id trips the address: {o:?}");
    let peers = peers_of(&lockout, &no_labels());
    assert_eq!(peers.len(), 1, "{peers:?}");
    let p = &peers[0];
    assert_eq!(p["failures"], 22, "the folded counts are summed");
    let listed = p["token_ids"].as_array().unwrap();
    assert!(listed.len() <= MAX_RECORDED_REASONS, "unauthenticated ids are capped: {}", listed.len());
    let reasons: u64 = p["reasons"].as_object().unwrap().values().map(|v| v.as_u64().unwrap()).sum();
    assert!(reasons as usize <= MAX_RECORDED_REASONS, "the folded failure list is capped: {reasons}");
}

// ---- Issue #455: the lockout is keyed by (peer, token id) ----

/// Criterion 1: one token's failures lock out that token only, not a sibling
/// token from the same address, and not the address.
#[test]
fn a_tripped_token_locks_out_only_itself() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    let mut last = None;
    for _ in 0..5 {
        last = Some(lockout.record_failure_detailed(&ip, "token_not_bound", "tok_x"));
    }
    let o = last.unwrap();
    assert!(o.newly_tripped && !o.peer_wide, "a token-scope trip: {o:?}");
    assert!(lockout.is_token_locked_out(&ip, "tok_x"), "the failing token is locked out");
    assert!(!lockout.is_token_locked_out(&ip, "tok_y"), "a sibling token from the same address is not");
    assert!(!lockout.is_locked_out(&ip), "the address as a whole is not");
}

/// Criterion 2: the 8th distinct token id from one address trips it
/// peer-wide; the buckets fold into one entry, and further ids add none.
#[test]
fn the_eighth_distinct_token_id_locks_out_the_whole_address() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    assert_eq!(MAX_TOKEN_IDS_PER_PEER, 8);
    for n in 0..7 {
        let o = lockout.record_failure_detailed(&ip, "token_unknown", &format!("tok_{n}"));
        assert!(!o.peer_wide && !o.locked_out, "id {n} alone trips nothing: {o:?}");
    }
    assert!(!lockout.is_locked_out(&ip), "7 ids do not lock out the address");
    let peers = peers_of(&lockout, &no_labels());
    assert_eq!(peers.len(), 7, "seven token buckets: {peers:?}");
    assert!(peers.iter().all(|p| p["scope"] == "token"), "{peers:?}");

    let o = lockout.record_failure_detailed(&ip, "token_unknown", "tok_7");
    assert!(o.peer_wide && o.newly_tripped && o.locked_out, "the 8th distinct id trips the address: {o:?}");
    assert!(lockout.is_locked_out(&ip), "the address is locked out as a whole");
    assert!(lockout.is_token_locked_out(&ip, "tok_never_seen"), "a peer-wide lockout covers every token id");

    let one_row = |l: &Lockout| {
        let peers = peers_of(l, &no_labels());
        assert_eq!(peers.len(), 1, "one entry for the address: {peers:?}");
        peers[0].clone()
    };
    let p = one_row(&lockout);
    assert_eq!((p["peer"].clone(), p["scope"].clone(), p["locked_out"].clone()), (json!("127.0.0.1"), json!("peer"), json!(true)));
    assert!(p["token_ids"].as_array().unwrap().len() <= MAX_RECORDED_REASONS, "{p}");

    for n in 8..1_008 {
        lockout.record_failure_detailed(&ip, "token_unknown", &format!("tok_{n}"));
    }
    let p = one_row(&lockout);
    assert_eq!(p["scope"], "peer");
    assert!(p["token_ids"].as_array().unwrap().len() <= MAX_RECORDED_REASONS, "{p}");
}

/// Criterion 2, the other address: a peer-wide trip on one address does not
/// touch another.
#[test]
fn a_peer_wide_trip_is_confined_to_its_address() {
    let lockout = lockout_with(FakeClock::new());
    let flooder = peer();
    let other: IpAddr = "10.0.0.2".parse().unwrap();
    for n in 0..8 {
        lockout.record_failure_detailed(&flooder, "token_unknown", &format!("tok_{n}"));
    }
    assert!(lockout.is_locked_out(&flooder));
    assert!(!lockout.is_locked_out(&other));
    assert!(!lockout.is_token_locked_out(&other, "tok_0"));
}

/// Criterion 3: a success for one token never clears another token's strikes.
#[test]
fn reset_of_one_token_keeps_another_tokens_strikes() {
    let lockout = lockout_with(FakeClock::new());
    let ip = peer();
    for _ in 0..4 {
        lockout.record_failure_detailed(&ip, "token_not_bound", "tok_x");
    }
    lockout.record_failure_detailed(&ip, "token_not_bound", "tok_y");
    assert!(!lockout.reset(&ip, "tok_y"), "tok_y had strikes, not a lockout");
    let o = lockout.record_failure_detailed(&ip, "token_not_bound", "tok_x");
    assert_eq!(o.count, 5, "tok_x kept its four strikes: {o:?}");
    assert!(o.newly_tripped, "so its fifth failure trips it");
}

/// Criterion 3: a [`LockoutKey`] renders the peer and, for a bucket, the
/// token id with every non-ASCII-graphic char as `?` (log-injection safety).
#[test]
fn lockout_key_display_is_log_safe() {
    let ip = peer();
    assert_eq!(bucket(ip, "tok\n\x1b").to_string(), "127.0.0.1 token_id=tok??");
    assert_eq!(bucket(ip, "tok_a").to_string(), "127.0.0.1 token_id=tok_a");
    assert_eq!(bucket(ip, "tok é").to_string(), "127.0.0.1 token_id=tok??");
    assert_eq!(LockoutKey { peer: ip, token_id: None }.to_string(), "127.0.0.1", "a peer-wide key is the bare peer");
}

/// Criterion 3: `sweep` reports a lapsed peer-wide entry by its key.
#[test]
fn sweep_reports_a_lapsed_peer_wide_entry() {
    let clock = FakeClock::new();
    let lockout = lockout_with(clock.clone());
    let ip = peer();
    for n in 0..8 {
        lockout.record_failure_detailed(&ip, "token_unknown", &format!("tok_{n}"));
    }
    clock.advance(10_000);
    assert_eq!(lockout.sweep(), vec![LockoutKey { peer: ip, token_id: None }]);
    assert!(!lockout.is_locked_out(&ip));
}
