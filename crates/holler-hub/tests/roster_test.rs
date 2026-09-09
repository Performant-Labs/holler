#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #186
//! The hub's roster (issue #186): who can be hollered at right now, driven by
//! periodic presence, with the tri-state TTL from the dropped-connections memo
//! (`connected` → `reconnecting` → `gone` → pruned), the explicit-close-is-gone
//! rule, and the name-collision policy (revised for holler-server#383) that can
//! never strand a healthy body.
//!
//! These are **unit** tests with an **injected clock** (the story's RED list):
//! the hub's TTL sweep is driven by a `Clock` the test advances by exact
//! amounts, so the 45 s / 180 s / 360 s thresholds are deterministic — no
//! 15-second real-time heartbeat, no flaky wall-clock timing. The live
//! wiring (a real body's presence reaching the hub's roster) is covered by the
//! separate CLI test `roster_cli_test.rs`, which uses real subprocesses.
//!
//! The roster's entry points are **synchronous** (the live connection loop
//! calls them without an `await` while holding the roster's guard), so these
//! are plain `#[test]`s — a `Roster::new(&Config)` with the spec's default
//! thresholds and an injected (offset-0) clock is all a test needs. The
//! injected clock is expressed in whole seconds (the spec's thresholds are all
//! whole seconds: 45 / 180 / 360 / 300), and `Roster` compares
//! `now - last_seen` against the thresholds in whole seconds.

use holler_hub::roster::{Config, Roster};
use holler_proto::{
    docs::{LastTurn, PendingItem, PendingKind},
    Mode, Presence, SessionAd, SessionState,
};

const T: &str = "tok_0001";
const C: &str = "cli_0001";

/// Build a `Presence` ad with one session per `name`, every session
/// `idle`/`spawn` unless a test overrides a field. `hostname` is `h1`.
fn presence(h: &str, names: &[&str]) -> Presence {
    Presence {
        hostname: h.to_string(),
        sessions: names
            .iter()
            .map(|n| SessionAd {
                name: (*n).to_string(),
                harness: "opencode".to_string(),
                state: SessionState::Idle,
                mode: Mode::Spawn,
                harness_session_id: None,
                turn_started_at: None,
                last_update_at: None,
                pending: None,
                turn_id: None,
                last_turn: None,
            })
            .collect(),
    }
}

/// A single idle `working` session ad with the given name (for the `stalled`
/// derivation test).
fn working_ad(name: &str) -> SessionAd {
    SessionAd {
        name: name.to_string(),
        harness: "opencode".to_string(),
        state: SessionState::Working,
        mode: Mode::Spawn,
        harness_session_id: None,
        turn_started_at: Some("2026-09-09T00:00:00Z".to_string()),
        last_update_at: Some("2026-09-09T00:00:00Z".to_string()),
        pending: None,
        turn_id: None,
        last_turn: None,
    }
}

#[test]
fn presence_replaces_not_merges() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1", "b/1"]));
    // Second presence advertises only `b/1`. Replace (not merge) means the
    // first presence's `a/1` is no longer connected.
    r.advertise(T, &presence("h1", &["b/1"]));
    // `rows(None)` is the default listing, which hides `gone` rows; the dropped
    // `a/1` is gone but still in the map until pruned, so inspect it via
    // `rows(Some(true))`.
    let rows = r.rows(Some(true));
    assert_eq!(rows.len(), 2, "both names still exist in the map (no prune yet)");
    let a = rows.iter().find(|row| row.name == "a/1").unwrap();
    let b = rows.iter().find(|row| row.name == "b/1").unwrap();
    assert_eq!(a.conn_state, "gone", "absent-from-frame goes gone immediately");
    assert_eq!(b.conn_state, "connected");
    // And the default listing shows only the survivor.
    let default = r.rows(None);
    assert_eq!(default.len(), 1);
    assert_eq!(default[0].name, "b/1");
}

#[test]
fn absent_session_goes_gone_immediately() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1", "b/1"]));
    // A presence that drops `b/1` marks it `gone` the instant it arrives — no
    // TTL wait.
    r.advertise(T, &presence("h1", &["a/1"]));
    // `gone` rows are hidden from the default listing, so collect them from the
    // full map (`rows(Some(true))`): exactly `b/1` is gone, `a/1` is not.
    let rows = r.rows(Some(true));
    let gone: Vec<_> = rows.iter().filter(|row| row.conn_state == "gone").map(|row| row.name.clone()).collect();
    assert_eq!(gone, vec!["b/1"], "the absent session is gone now, `a/1` is not");
}

#[test]
fn different_token_cannot_steal_a_name() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    let other = "tok_0002";
    r.set_token(other, "cli_0002");
    // A different token claiming a name whose current holder is `connected`
    // is rejected for that row (the existing row keeps its `token_id`).
    let rejected = r.advertise(other, &presence("h2", &["a/1"]));
    assert_eq!(rejected, vec!["a/1".to_string()], "the connected holder's row is rejected");
    let rows = r.rows(None);
    let a = rows.iter().find(|row| row.name == "a/1").unwrap();
    assert_eq!(a.token_id, T, "the connected holder keeps the name");
    assert!(a.hostname == "h1", "the connected holder's locator is not clobbered");
}

#[test]
fn same_token_reclaims_its_name() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    // The same token re-advertising its own name is always accepted (a same-
    // token re-claim is a locator refresh, never a collision).
    let rejected = r.advertise(T, &presence("h1", &["a/1"]));
    assert!(rejected.is_empty(), "a same-token re-claim is never rejected");
    let rows = r.rows(None);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].conn_state, "connected");
}

#[test]
fn fresh_token_claims_name_from_reconnecting_holder() {
    // The holler-server#383 regression: a name held by a token that has
    // aged out of `connected` into `reconnecting` can be **claimed** by a
    // different token — the new claim replaces the row (name kept, locators
    // swapped) and the displaced holder's later presence for that name is
    // rejected.
    let cfg = Config {
        reconnect_secs: 10,
        ..Config::default()
    };
    let r = Roster::new(&cfg);
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    // Age the holder out of `connected` into `reconnecting`.
    r.advance(11);
    r.sweep();
    {
        let rows = r.rows(None);
        assert_eq!(rows[0].conn_state, "reconnecting");
    }
    // A different token now claims the same name. Because the holder is
    // `reconnecting` (not `connected`), the claim succeeds.
    let other = "tok_0002";
    r.set_token(other, "cli_0002");
    let rejected = r.advertise(other, &presence("h2", &["a/1"]));
    assert!(rejected.is_empty(), "a reconnecting holder does not block a new claim");
    {
        let rows = r.rows(None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].token_id, other, "the new claimant now owns the row");
        assert_eq!(rows[0].hostname, "h2");
    }
    // The displaced holder's *later* presence for that name is rejected.
    let rejected2 = r.advertise(T, &presence("h1", &["a/1"]));
    assert_eq!(rejected2, vec!["a/1".to_string()], "the displaced holder cannot reclaim the name it lost");
    {
        let rows = r.rows(None);
        assert_eq!(rows[0].token_id, other);
    }
}

#[test]
fn connected_holder_still_blocks_a_different_token() {
    // Even if the holder's `last_seen` is old (it just has not been swept
    // yet), while the row is still `connected` a different token cannot take
    // the name.
    let cfg = Config {
        reconnect_secs: 10,
        ..Config::default()
    };
    let r = Roster::new(&cfg);
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    // Do not sweep: the row is still `connected` (its `last_seen` is 0).
    let other = "tok_0002";
    r.set_token(other, "cli_0002");
    let rejected = r.advertise(other, &presence("h2", &["a/1"]));
    assert_eq!(rejected, vec!["a/1".to_string()], "a connected holder blocks a different token");
}

#[test]
fn rejected_claim_succeeds_on_next_presence_after_holder_gone() {
    // The holler-server#243 regression: a claim rejected while the holder is
    // `connected` succeeds automatically on the holder's *next* presence once
    // the holder has aged into `gone`. Presence repeats every 15 s, so the
    // claimant retries for free.
    let cfg = Config {
        reconnect_secs: 10,
        gone_secs: 40,
        ..Config::default()
    };
    let r = Roster::new(&cfg);
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    let other = "tok_0002";
    r.set_token(other, "cli_0002");
    // First attempt: the holder is `connected`, so the claim is rejected.
    let rejected = r.advertise(other, &presence("h2", &["a/1"]));
    assert_eq!(rejected, vec!["a/1".to_string()]);
    // The holder ages out to `gone` (past `gone_secs`), and the claimant's
    // next presence (its 15 s retry) lands after the sweep.
    r.advance(41);
    r.sweep();
    let rejected2 = r.advertise(other, &presence("h2", &["a/1"]));
    assert!(rejected2.is_empty(), "once the holder is gone, the claimant's retry succeeds");
    {
        let rows = r.rows(None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].token_id, other);
    }
}

#[test]
fn missed_heartbeats_connected_to_reconnecting_to_gone_to_pruned() {
    // The tri-state TTL, clock-driven: `connected` → (≥45 s) `reconnecting` →
    // (≥180 s) `gone` → (≥360 s) pruned (row removed).
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    assert_eq!(r.rows(None)[0].conn_state, "connected");

    r.advance(45); // exactly the reconnect threshold
    r.sweep();
    assert_eq!(r.rows(None)[0].conn_state, "reconnecting");

    r.advance(135); // 180 s since last_seen
    r.sweep();
    // `gone` rows are hidden from the default listing; the row is gone (and
    // still in the map until pruned), so inspect it via `rows(Some(true))`.
    assert_eq!(r.rows(Some(true))[0].conn_state, "gone");
    assert!(r.rows(None).is_empty(), "a gone row is hidden from the default listing");

    r.advance(180); // 360 s since last_seen
    r.sweep();
    assert!(r.rows(Some(true)).is_empty(), "the row is pruned at the prune threshold");
}

#[test]
fn any_frame_touch_flips_reconnecting_back_to_connected() {
    // holler-server#203: *any* frame from a body (presence, update, ping,
    // response) proves liveness — it touches `last_seen` on all the token's
    // rows and flips a `reconnecting` row back to `connected`. A successful
    // response (not just a heartbeat) is the case that fixed #203.
    let cfg = Config {
        reconnect_secs: 10,
        ..Config::default()
    };
    let r = Roster::new(&cfg);
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    r.advance(11);
    r.sweep();
    assert_eq!(r.rows(None)[0].conn_state, "reconnecting");
    // A non-presence frame (a `session/prompt` response) arrives.
    r.touch(T, "session/prompt");
    // The touch refreshed `last_seen`, so the next sweep no longer ages it out.
    r.sweep();
    assert_eq!(r.rows(None)[0].conn_state, "connected", "a touch flips reconnecting back to connected");
}

#[test]
fn touch_unknown_token_is_silent() {
    let r = Roster::new(&Config::default());
    // A touch for a token the hub has no rows for is a no-op (never a panic,
    // never an error) — the frame may predate a join, or name a token that was
    // already revoked.
    r.touch("tok_never_seen", "session/presence");
    assert!(r.rows(None).is_empty());
}

#[test]
fn touch_never_refreshes_other_token() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.set_token("tok_0002", "cli_0002");
    r.advertise(T, &presence("h1", &["a/1"]));
    r.advertise("tok_0002", &presence("h2", &["b/1"]));
    // Age both out of `connected`.
    r.advance(45);
    r.sweep();
    // Touching token A must refresh *only* A's row, never B's.
    r.touch(T, "session/presence");
    r.sweep();
    let rows = r.rows(None);
    let a = rows.iter().find(|row| row.name == "a/1").unwrap();
    let b = rows.iter().find(|row| row.name == "b/1").unwrap();
    assert_eq!(a.conn_state, "connected", "the touched token's row refreshed");
    assert_eq!(b.conn_state, "reconnecting", "another token's row was not refreshed by A's touch");
}

#[test]
fn explicit_close_is_gone_now() {
    // holler-server#80: an explicit close / `circuit/superseded` marks the
    // token's rows `gone` *immediately* — no TTL wait.
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1", "b/1"]));
    r.clear(T);
    let rows = r.rows(None);
    assert!(rows.iter().all(|row| row.conn_state == "gone"), "every row of a closed token is gone now");
}

#[test]
fn revoke_is_gone_now() {
    // A revoked token's rows go `gone` immediately — the token is dead, so its
    // presence is void; the rows linger (as `gone`) until the TTL prunes them.
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &presence("h1", &["a/1"]));
    r.clear(T);
    // The row is `gone` in the map (inspect via `rows(Some(true))`); it is
    // hidden from the default listing (and from collisions' "connected" check)
    // but present under `all`.
    let rows = r.rows(Some(true));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].conn_state, "gone");
    assert!(r.rows(None).is_empty(), "gone rows are excluded from the default listing");
}

#[test]
fn working_with_no_updates_shows_stalled_after_threshold() {
    // `stalled` is a hub-derived display state: a `working` row whose
    // `last_update_at` is older than the stall threshold shows as `stalled`.
    // The state itself stays `working` (the A2A name); only the displayed /
    // `session_busy` state flips. The threshold is 300 s by default; the test
    // shortens it and advances the clock past it.
    let cfg = Config {
        stall_secs: 60,
        ..Config::default()
    };
    let r = Roster::new(&cfg);
    r.set_token(T, C);
    r.advertise(T, &Presence { hostname: "h1".into(), sessions: vec![working_ad("a/1")] });
    {
        let rows = r.rows(None);
        assert_eq!(rows[0].state, "working", "a fresh working row is not stalled");
    }
    // 10 minutes with no update (the default `stall_secs` would be 300 s; here
    // 60 s). The row's `last_update_at` (2026-09-09T00:00:00Z) is now far past
    // the threshold.
    r.advance(600);
    let rows = r.rows(None);
    assert_eq!(rows[0].state, "stalled", "a working row with no update past the threshold is stalled");
}

/// A `SessionAd` carrying `pending` and a terminal `last_turn` (the roster's
/// `PENDING` and `LAST TURN` columns, issues #151 / #142).
fn input_required_ad(name: &str) -> SessionAd {
    SessionAd {
        name: name.to_string(),
        harness: "opencode".to_string(),
        state: SessionState::InputRequired,
        mode: Mode::Spawn,
        harness_session_id: None,
        turn_started_at: None,
        last_update_at: None,
        pending: Some(vec![PendingItem {
            id: "p1".to_string(),
            kind: PendingKind::Permission,
            prompt: "may use the network?".to_string(),
            options: vec!["allow".to_string(), "reject".to_string()],
        }]),
        turn_id: Some("h-000000000000000000000042".to_string()),
        last_turn: Some(LastTurn {
            turn_id: "h-000000000000000000000041".to_string(),
            state: SessionState::Completed,
            stop_reason: "end_turn".to_string(),
            ended_at: "2026-09-09T00:05:00Z".to_string(),
        }),
    }
}

#[test]
fn roster_json_shape() {
    // The `--json` roster carries the full row document: the wire vocabulary
    // `turn_id` / `last_turn` (issue #142) and `pending` (issue #151) are
    // present, and a terminal `last_turn` round-trips its four fields.
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.advertise(T, &Presence { hostname: "h1".into(), sessions: vec![input_required_ad("a/1")] });
    let rows = r.rows(None);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.state, "input-required");
    assert_eq!(row.turn_id.as_deref(), Some("h-000000000000000000000042"));
    let last = row.last_turn.as_ref().expect("a terminal last_turn is carried");
    assert_eq!(last.turn_id, "h-000000000000000000000041");
    assert_eq!(last.state.as_str(), "completed");
    assert_eq!(last.stop_reason, "end_turn");
    assert_eq!(last.ended_at, "2026-09-09T00:05:00Z");
    // `--json` carries the full `pending` array (not just the rendered column).
    let pending = row.pending.as_ref().expect("an input-required row carries pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, PendingKind::Permission);
    assert_eq!(pending[0].prompt, "may use the network?");
    assert_eq!(pending[0].options, vec!["allow", "reject"]);
}

/// Issue #236 (ADR 0005 §2 + §4): once a token's label is bound
/// ([`Roster::set_label`], the same call `circuit::handle_authenticated`
/// makes at auth time), the rows it advertises are named `<label>/<session>`,
/// and `--prefix`'s transitive match (`Roster::rows_matching`) narrows on
/// that qualification — a real label boundary, not a plain substring search.
#[test]
fn label_qualifies_row_names_and_prefix_filters_on_the_label_boundary() {
    let r = Roster::new(&Config::default());
    r.set_token(T, C);
    r.set_label(T, "io");
    r.advertise(T, &presence("h1", &["alpha", "beta"]));

    let other = "tok_0002";
    r.set_token(other, "cli_0002");
    r.set_label(other, "io-other");
    r.advertise(other, &presence("h2", &["gamma"]));

    let mut names: Vec<String> = r.rows(None).iter().map(|row| row.name.clone()).collect();
    names.sort();
    assert_eq!(
        names,
        vec!["io-other/gamma".to_string(), "io/alpha".to_string(), "io/beta".to_string()],
        "every row is qualified `<label>/<session>` once its token's label is bound"
    );

    // `--prefix io` matches `io/alpha` and `io/beta`, never `io-other/gamma` —
    // proving the match is on the `/`-delimited label boundary, not a plain
    // substring/`starts_with("io")` search (which `io-other/gamma` would also
    // satisfy).
    let mut io_names: Vec<String> = r.rows_matching(None, Some("io")).iter().map(|row| row.name.clone()).collect();
    io_names.sort();
    assert_eq!(io_names, vec!["io/alpha".to_string(), "io/beta".to_string()]);

    // A trailing slash (the ADR's own example spelling, `io/`) behaves
    // identically.
    let mut io_slash: Vec<String> =
        r.rows_matching(None, Some("io/")).iter().map(|row| row.name.clone()).collect();
    io_slash.sort();
    assert_eq!(io_slash, io_names, "a trailing slash on --prefix is equivalent to none");

    // `--prefix io-other` matches only its own row.
    let other_names: Vec<String> =
        r.rows_matching(None, Some("io-other")).iter().map(|row| row.name.clone()).collect();
    assert_eq!(other_names, vec!["io-other/gamma".to_string()]);

    // No prefix (`None`) is every row, same as `rows(None)`.
    assert_eq!(r.rows_matching(None, None).len(), 3);

    // A token that never had its label bound (e.g. a test — or, defensively,
    // a presence that somehow outraced the auth-time bind) still gets a row:
    // it just falls back to the bare name, not a panic or a dropped session.
    let unlabeled = "tok_0003";
    r.set_token(unlabeled, "cli_0003");
    r.advertise(unlabeled, &presence("h3", &["delta"]));
    let bare = r.rows(None).into_iter().find(|row| row.token_id == unlabeled).expect("the unlabeled row exists");
    assert_eq!(bare.name, "delta", "no bound label falls back to the bare name");
}
