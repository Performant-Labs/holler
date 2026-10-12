//! A body is a pane's only on evidence (#839, P4, P6 and P8; Acceptance 20, the verbs' side;
//! `driver_of`'s table is `holler-pane`'s `driven_test.rs`).
//!
//! The doctor rig's panes share one data directory, so each pane's server lists every pane's
//! sessions: a shared listing, where a listed body proves nothing. A body that cannot be shown to
//! be the pane's own is refused `pane-not-driven` with the two-step remedy, before anything moves,
//! and a switch onto a session another live body already advertises is refused
//! `session-driven-elsewhere`.

use holler_pane::ports::Activity;
use holler_pane_testkit::harness::HarnessOp as H;
use holler_pane_testkit::hub::HubOp;

use super::{advertised, body, body_name, drive};
use crate::doctor::rig::{Rig, Seed};
use crate::switch::{argv, both, data, failed_before_the_act, Case, P, Q};

/// Two bodies on the shared listing that no pane's record or recorded DRIVEN names (20 (a)).
const A: &str = "kiwi/sa";
const B: &str = "lime/sb";

/// P and Q on one shared listing, and two listed bodies with no evidence; returns their ids.
fn two_unproven_bodies() -> (Rig, (String, String)) {
    let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
    let port = rig.live(P).port;
    let (a, b) = (
        rig.harness.seed_session(port),
        rig.harness.seed_session(port),
    );
    body(&rig, A, &a, Activity::Idle);
    body(&rig, B, &b, Activity::Idle);
    (rig, (a, b))
}

/// Nothing moved, and no body was asked to re-point.
fn assert_nothing_moved<T>(case: &Case<T>) {
    let f = case.format;
    case.assert_unchanged();
    for op in [H::CreateSession, H::SelectSession] {
        assert!(!case.calls.harness.contains(&op), "{f:?}: no {op:?}");
    }
    assert!(
        !case.calls.hub.contains(&HubOp::Rebind),
        "{f:?}: no rebind: {:?}",
        case.calls.hub
    );
}

/// 20 (a): with two unproven bodies, `switch` and `reset` of P are refused `pane-not-driven`,
/// naming each body as `label/session` with its id, and the manual step; nothing moves.
#[test]
fn unproven_bodies_are_refused_naming_each_and_the_two_steps() {
    let fresh = |rig: &Rig, _: &(String, String)| {
        let s2 = rig.harness.seed_session(rig.live(P).port);
        argv(&["pane", "switch", P, &s2])
    };
    let reset = |_: &Rig, _: &(String, String)| argv(&["pane", "reset", P]);
    for cases in [
        both(two_unproven_bodies, fresh),
        both(two_unproven_bodies, reset),
    ] {
        let [text, _] = &cases;
        let (a, b) = &text.setup;
        for message in failed_before_the_act(&cases, 3, "pane-not-driven") {
            for named in [A, B, a.as_str(), b.as_str()] {
                assert!(message.contains(named), "names {named}: {message:?}");
            }
            let step = format!("holler pane switch {P} ");
            assert!(message.contains(&step), "the manual step: {message:?}");
        }
        for case in &cases {
            assert_nothing_moved(case);
        }
    }
}

/// 20 (a), the remedy works: `pane switch P <the id one body advertises>` pairs P with that body
/// by the target rule (no rebind; the record, SHOWN and DRIVEN become that id in one write), and
/// the next switch finds it by the session of record and re-points that body only.
#[test]
fn the_two_step_switch_pairs_the_pane_with_the_body_it_names() {
    let (rig, (a, b)) = two_unproven_bodies();
    let pair = rig.verb(&["pane", "switch", P, &a], holler_cli::output::Format::Json);
    assert_eq!(pair.code, 0, "{pair:?}");
    let record = rig.record(P);
    assert_eq!(record.session_of_record.as_deref(), Some(a.as_str()));
    assert_eq!(record.last_observed.driven.as_deref(), Some(a.as_str()));
    assert!(
        !rig.hub.faults().calls().contains(&HubOp::Rebind),
        "the pairing re-points nothing"
    );

    let s2 = rig.harness.seed_session(rig.live(P).port);
    let next = rig.verb(
        &["pane", "switch", P, &s2],
        holler_cli::output::Format::Json,
    );
    assert_eq!(next.code, 0, "{next:?}");
    assert_eq!(
        advertised(&rig, A).harness_session_id,
        s2,
        "P's body follows"
    );
    assert_eq!(
        advertised(&rig, B).harness_session_id,
        b,
        "the other body is untouched"
    );
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(s2.as_str())
    );
}

/// 20 (b), the second plan check's block: P has no body; Q's body is behind on an id the shared
/// server lists and no evidence names. P never claims it: `switch` and `reset` of P are refused
/// `pane-not-driven`, and Q's body is never asked to re-point.
#[test]
fn a_pane_with_no_body_never_re_points_another_pane_s_behind_body() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
        let cfg = rig.harness.seed_session(rig.live(Q).port);
        body(&rig, &body_name(Q), &cfg, Activity::Idle);
        (rig, cfg)
    };
    let fresh = |rig: &Rig, _: &String| {
        let s2 = rig.harness.seed_session(rig.live(P).port);
        argv(&["pane", "switch", P, &s2])
    };
    let reset = |_: &Rig, _: &String| argv(&["pane", "reset", P]);
    for cases in [both(build, fresh), both(build, reset)] {
        failed_before_the_act(&cases, 3, "pane-not-driven");
        for case in &cases {
            assert_nothing_moved(case);
            assert_eq!(
                advertised(&case.rig, &body_name(Q)).harness_session_id,
                case.setup
            );
        }
    }
}

/// 20 (b), the same state with Q's body on Q's recorded DRIVEN: the exclusion rule removes it for
/// P, so P's switch proceeds with no drive step (DRIVEN `None`), and a switch of P onto that id is
/// refused `session-driven-elsewhere`, naming the body.
#[test]
fn a_body_another_pane_records_is_not_this_pane_s_and_is_not_switched_onto() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
        let d2 = rig.harness.seed_session(rig.live(Q).port);
        body(&rig, &body_name(Q), &d2, Activity::Idle);
        rig.rewrite(Q, |q| q.last_observed.driven = Some(d2.clone()));
        (rig, d2)
    };
    let fresh = |rig: &Rig, _: &String| {
        let s2 = rig.harness.seed_session(rig.live(P).port);
        argv(&["pane", "switch", P, &s2])
    };
    let cases = both(build, fresh);
    data(&cases);
    for case in &cases {
        assert_eq!(
            case.record(P).last_observed.driven,
            None,
            "no body drives P"
        );
        assert_eq!(case.calls.hub, [HubOp::Advertised], "no drive step");
        assert_eq!(
            advertised(&case.rig, &body_name(Q)).harness_session_id,
            case.setup
        );
    }
    let onto = |_: &Rig, d2: &String| argv(&["pane", "switch", P, d2]);
    let cases = both(build, onto);
    for message in failed_before_the_act(&cases, 3, "session-driven-elsewhere") {
        assert!(
            message.contains(&body_name(Q)),
            "names the body: {message:?}"
        );
    }
    cases.iter().for_each(assert_nothing_moved);
}

/// 20 (d) rows 1 and 2 through the verbs. Row 2 (a body on Q's recorded DRIVEN) is above. Row 1,
/// a body on Q's session of record: the plan's existing refusal comes first (D3 step 1, "the
/// existing refusals; then" the driver), so the switch is `session-of-other-pane`.
#[test]
fn a_switch_onto_another_pane_s_record_is_refused_by_the_existing_rule_first() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
        drive(&rig, Q);
        (rig, ())
    };
    let words = |rig: &Rig, (): &()| argv(&["pane", "switch", P, &rig.live(Q).session]);
    let cases = both(build, words);
    failed_before_the_act(&cases, 3, "session-of-other-pane");
    cases.iter().for_each(assert_nothing_moved);
}

/// 20 (e), P6: P's own body drives it, and a second live body (an operator's attach row on a
/// session Holler does not own) advertises the switch's target: refused
/// `session-driven-elsewhere`, naming that body, and nothing moves. When the body on the target is
/// P's own driver, it is the idempotent re-run (`order.rs`).
#[test]
fn a_switch_onto_a_session_another_body_drives_is_refused() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1)]);
        drive(&rig, P);
        let target = rig.harness.seed_session(rig.live(P).port);
        body(&rig, "ops/manual", &target, Activity::Idle);
        (rig, target)
    };
    let words = |_: &Rig, target: &String| argv(&["pane", "switch", P, target]);
    let cases = both(build, words);
    for message in failed_before_the_act(&cases, 3, "session-driven-elsewhere") {
        assert!(
            message.contains("ops/manual"),
            "names the body: {message:?}"
        );
    }
    for case in &cases {
        assert_nothing_moved(case);
        assert_eq!(
            advertised(&case.rig, "ops/manual").harness_session_id,
            case.setup
        );
    }
}

/// 20 (c), the fallback where it is safe: a pane whose server lists only its own sessions (a data
/// directory of its own) and one body on a listed id that is neither its record nor its recorded
/// DRIVEN (a body that reverted): one `switch` finds it and re-points it.
#[test]
fn on_an_unshared_listing_a_reverted_body_is_found_and_re_pointed() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1).data_dir("p-own"), Seed::new(Q, 1, 2)]);
        let configured = rig.harness.seed_session(rig.live(P).port);
        body(&rig, &body_name(P), &configured, Activity::Idle);
        (rig, ())
    };
    let words = |rig: &Rig, (): &()| argv(&["pane", "switch", P, &rig.live(P).session]);
    let cases = both(build, words);
    data(&cases);
    for case in &cases {
        let sor = case.rig.live(P).session.clone();
        assert_eq!(advertised(&case.rig, &body_name(P)).harness_session_id, sor);
        assert_eq!(
            case.record(P).last_observed.driven.as_deref(),
            Some(sor.as_str())
        );
        assert!(case.calls.hub.contains(&HubOp::Rebind));
    }
}
