#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #460
//! Join held and the one-time release grant, hub side (issue #460, on the
//! session hold #437), against a real hub started with `--join-held`, a real
//! body and `stub-acp`, including a real hub restart. Tags:
//! `test-grp-concurrency` (the grant races) and lifecycle (the rest).
//!
//! Like `hold_hub_test.rs`, no fixed sleep is used as synchronisation: waits
//! poll an observable outcome, races synchronise on a barrier, and the one
//! test that has to let a grant's TTL pass says so.

mod support;

use std::sync::{Arc, Barrier};
use std::time::Duration;

use holler_hub::control::{self, ControlError};
use serde_json::Value;
use support::hold_rig::{assert_held, is_delivered, is_held, say_at_retrying, Rig, SESSION};

/// A refusal of `code` (and `data.reason` / `data.hold_kind` when given).
fn refusal(res: &Result<Value, ControlError>) -> Option<&holler_proto::WireError> {
    match res {
        Err(ControlError::Refused(e)) => Some(e),
        _ => None,
    }
}

fn is_invalid_grant(res: &Result<Value, ControlError>, reason: &str) -> bool {
    refusal(res).is_some_and(|e| e.code == -32012 && e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some(reason))
}

fn hold_kind(res: &Result<Value, ControlError>) -> Option<String> {
    refusal(res)?.data.as_ref()?.hold_kind.clone()
}

fn say_with(rig: &Rig, s: &str, queue: bool, grant: &str) -> Result<Value, ControlError> {
    control::say_with_at(rig.root(), s, "hi", queue, Some(grant), Duration::from_secs(60))
}

fn grant(rig: &Rig, s: &str) -> String {
    control::release_once_at(rig.root(), s, None).unwrap()["grant"].as_str().unwrap().to_string()
}

const JOIN_HELD: &[&str] = &["--join-held"];

// --- 1. nothing changes unless asked ------------------------------------------------

#[test]
fn without_the_option_nothing_joins_held() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let row = rig.row(SESSION).unwrap();
    for f in ["hold", "hold_reason", "held_since", "hold_kind", "hold_default"] {
        assert!(row.get(f).is_none(), "{f} must be absent on a session that joined open: {row}");
    }
    assert!(is_delivered(&rig.say(SESSION, false)));
    // The plain hold still works exactly as before, and reports no default hold.
    rig.hold(SESSION, Some("drain")).unwrap();
    let held = rig.row(SESSION).unwrap();
    assert_eq!((held["hold_kind"].as_str(), held.get("hold_default")), (Some("operator"), None));
    assert_held(rig.say(SESSION, false), Some("drain"));
}

// --- 2. join held -------------------------------------------------------------------

#[test]
fn a_session_joins_held_and_every_delivery_variant_is_refused() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let row = rig.row(SESSION).unwrap();
    assert_eq!(row["hold"], true);
    assert_eq!(row["hold_kind"], "default");
    assert_eq!(row["hold_reason"], "held on join");
    assert!(row["held_since"].as_str().is_some());
    assert!(row.get("hold_default").is_none(), "a default hold alone is not `under` anything");
    for queue in [false, true] {
        let res = rig.say(SESSION, queue);
        assert_eq!(hold_kind(&res).as_deref(), Some("default"), "{res:?}");
        assert_held(res, Some("held on join"));
    }
    let redirect = control::interrupt_at(rig.root(), SESSION, Some("do this instead"));
    assert_eq!(hold_kind(&redirect).as_deref(), Some("default"));
    assert_held(redirect, Some("held on join"));
    // interrupt without text still works on a session that joined held.
    assert_eq!(control::interrupt_at(rig.root(), SESSION, None).unwrap()["applied"], true);
}

#[test]
fn only_sessions_matching_the_pattern_join_held() {
    let rig = Rig::start_with_args(
        &[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"]), ("gamma", &["--chunks", "1"])],
        &[],
        &["--join-held", "b/al*", "--join-held", "b/gam?a"],
    );
    assert_eq!(rig.row("b/alpha").unwrap()["hold"], true);
    assert_eq!(rig.row("b/gamma").unwrap()["hold"], true);
    assert!(rig.row("b/beta").unwrap().get("hold").is_none());
    assert!(is_delivered(&rig.say("b/beta", false)));
    assert_held(rig.say("b/alpha", false), Some("held on join"));
}

// --- 3. the grant -------------------------------------------------------------------

#[test]
fn a_grant_lets_exactly_one_prompt_through_and_the_session_is_held_again() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])], &[], JOIN_HELD);
    let minted = control::release_once_at(rig.root(), SESSION, None).unwrap();
    let g = minted["grant"].as_str().unwrap().to_string();
    assert!(g.starts_with("gnt_"));
    assert_eq!((minted["default_held"].as_bool(), minted["operator_held"].as_bool(), minted["ttl_ms"].as_u64()), (Some(true), Some(false), Some(60_000)));

    // Only the grant gets through: plain, --queue and a grant for another session do not.
    assert!(is_held(&rig.say(SESSION, false)));
    assert!(is_held(&rig.say(SESSION, true)));
    let other = say_with(&rig, "b/beta", false, &g);
    assert!(is_invalid_grant(&other, "other_session"), "{other:?}");
    // The refused attempts did not spend it.
    let ok = say_with(&rig, SESSION, false, &g);
    assert!(is_delivered(&ok), "{ok:?}");
    // A second use, a plain say and a new attempt are all refused now.
    assert!(is_invalid_grant(&say_with(&rig, SESSION, false, &g), "used"));
    assert!(is_held(&rig.say(SESSION, false)));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "held again");
    assert!(is_invalid_grant(&say_with(&rig, SESSION, false, "gnt_00000000000000000000000000000000"), "unknown"));

    // A grant works with --queue too, and each grant is for one prompt.
    let g2 = grant(&rig, SESSION);
    assert!(is_delivered(&say_with(&rig, SESSION, true, &g2)));
    assert!(is_invalid_grant(&say_with(&rig, SESSION, true, &g2), "used"));
    // The other session's own hold is untouched by all of this.
    assert_held(rig.say("b/beta", false), Some("held on join"));
}

#[test]
fn an_unused_grant_expires_and_the_session_is_held_again() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let g = control::release_once_at(rig.root(), SESSION, Some(Duration::from_millis(300))).unwrap()["grant"].as_str().unwrap().to_string();
    // The TTL is the thing under test, so let it pass (with a wide margin).
    std::thread::sleep(Duration::from_millis(900));
    let late = say_with(&rig, SESSION, false, &g);
    assert!(is_invalid_grant(&late, "expired"), "{late:?}");
    assert!(is_held(&rig.say(SESSION, false)));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

#[test]
fn a_hub_restart_voids_a_live_grant_and_keeps_the_session_held() {
    let mut rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let g = control::release_once_at(rig.root(), SESSION, Some(Duration::from_secs(600))).unwrap()["grant"].as_str().unwrap().to_string();
    rig.restart_hub();
    support::wait_for(Duration::from_secs(90), || rig.row(SESSION).filter(|r| r["conn_state"] == "connected")).unwrap_or_else(|| panic!("the body re-joined\n{}", rig.diagnostics()));
    let row = rig.row(SESSION).unwrap();
    assert_eq!((row["hold"].as_bool(), row["hold_kind"].as_str()), (Some(true), Some("default")), "the default hold persisted");
    let res = say_with(&rig, SESSION, false, &g);
    assert!(is_invalid_grant(&res, "unknown"), "the grant is void after a restart: {res:?}");
    assert!(is_held(&rig.say(SESSION, false)));
}

// --- 4. operator hold beats a grant ---------------------------------------------------

#[test]
fn an_operator_hold_beats_a_grant_and_the_error_says_so() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let g = grant(&rig, SESSION);
    rig.hold(SESSION, Some("drain")).unwrap();
    let row = rig.row(SESSION).unwrap();
    assert_eq!((row["hold_kind"].as_str(), row["hold_default"].as_bool(), row["hold_reason"].as_str()), (Some("operator"), Some(true), Some("drain")));

    let res = say_with(&rig, SESSION, false, &g);
    assert_eq!(hold_kind(&res).as_deref(), Some("operator"), "the error names the operator hold: {res:?}");
    assert_held(res, Some("drain"));
    // A grant minted while the operator hold is set says it will not help.
    assert_eq!(control::release_once_at(rig.root(), SESSION, None).unwrap()["operator_held"], true);

    // Releasing the operator hold falls back to the default hold, not to open;
    // the grant was not spent by the refused attempt and now works.
    let out = rig.release(SESSION).unwrap();
    assert_eq!((out["lifted"].as_str(), out["still_held"].as_bool()), (Some("operator"), Some(true)));
    assert!(is_held(&rig.say(SESSION, false)));
    assert!(is_delivered(&say_with(&rig, SESSION, false, &g)));
    // A second release lifts the default hold; the session is open, and stays open.
    let out = rig.release(SESSION).unwrap();
    assert_eq!((out["lifted"].as_str(), out["still_held"].as_bool()), (Some("default"), Some(false)));
    assert!(is_delivered(&rig.say(SESSION, false)));
    std::thread::sleep(Duration::from_millis(900)); // a few presence beats (300ms each)
    assert!(rig.row(SESSION).unwrap().get("hold").is_none(), "presence does not re-hold a released session");
}

#[test]
fn a_released_default_hold_is_reapplied_when_the_hub_restarts_with_the_option() {
    let mut rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    assert_eq!(rig.release(SESSION).unwrap()["lifted"], "default");
    assert!(is_delivered(&rig.say(SESSION, false)));
    rig.restart_hub();
    support::wait_for(Duration::from_secs(90), || rig.row(SESSION).filter(|r| r["conn_state"] == "connected")).expect("the body re-joined");
    // Its first presence to the restarted hub is a join.
    let held = support::wait_for(Duration::from_secs(30), || rig.row(SESSION).filter(|r| r["hold"] == true));
    assert!(held.is_some(), "the session joined held again after the restart");
}

#[test]
fn releasing_or_minting_for_an_unknown_session_is_refused() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let res = control::release_once_at(rig.root(), "b/nope", None);
    assert!(matches!(&res, Err(ControlError::Refused(e)) if e.code == -32003), "{res:?}");
    let zero = control::release_once_at(rig.root(), SESSION, Some(Duration::ZERO));
    assert!(matches!(&zero, Err(ControlError::Refused(e)) if e.code == -32602), "a zero ttl is invalid: {zero:?}");
}

// --- 5. concurrency ------------------------------------------------------------------

/// Four senders race for one grant (two present it, two do not, in both
/// delivery variants): exactly one prompt is delivered, and the session is held
/// again before anything else can be accepted. Repeated many times.
#[test]
fn racing_senders_get_exactly_one_prompt_through_a_grant() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    for round in 0..20 {
        let g = grant(&rig, SESSION);
        let barrier = Arc::new(Barrier::new(4));
        let senders: Vec<_> = (0..4)
            .map(|i| {
                let (root, b, g) = (rig.root().to_path_buf(), barrier.clone(), g.clone());
                std::thread::spawn(move || {
                    b.wait();
                    let grant = (i % 2 == 0).then_some(g.as_str());
                    // Busy is retried (nothing delivered); every other outcome is final.
                    let deadline = std::time::Instant::now() + Duration::from_secs(30);
                    loop {
                        let res = control::say_with_at(&root, SESSION, "race", i >= 2, grant, Duration::from_secs(60));
                        let busy = matches!(&res, Err(ControlError::Refused(e)) if e.code == -32009);
                        if !busy || std::time::Instant::now() > deadline {
                            return res;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                })
            })
            .collect();
        let results: Vec<_> = senders.into_iter().map(|t| t.join().unwrap()).collect();
        let delivered = results.iter().filter(|r| is_delivered(r)).count();
        assert_eq!(delivered, 1, "round {round}: exactly one prompt must be delivered: {results:?}");
        for r in results.iter().filter(|r| !is_delivered(r)) {
            assert!(is_held(r) || is_invalid_grant(r, "used"), "round {round}: a loser was neither held nor used-grant: {r:?}");
        }
        assert!(is_held(&rig.say(SESSION, false)), "round {round}: the session must be held again");
        rig.wait_row(SESSION, |r| r["state"] == "idle");
    }
}

/// Minting, spending, holding and releasing at once never leaves the hub in a
/// state where a plain say gets through a default hold.
#[test]
fn a_plain_say_never_gets_through_a_default_hold_under_churn() {
    let rig = Rig::start_with_args(&[("alpha", &["--chunks", "1"])], &[], JOIN_HELD);
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let churn = {
        let (root, stop) = (rig.root().to_path_buf(), stop.clone());
        std::thread::spawn(move || {
            while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = control::hold_at(&root, SESSION, Some("x"));
                let _ = control::release_at(&root, SESSION); // peels the operator hold only
            }
        })
    };
    for round in 0..30 {
        let res = say_at_retrying(rig.root(), SESSION, "plain", false);
        assert!(is_held(&res), "round {round}: a plain say got through a default hold: {res:?}");
    }
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    churn.join().unwrap();
    assert_eq!(rig.row(SESSION).unwrap()["hold_kind"].as_str().map(|_| true), Some(true));
}
