//! Switch and reset move the pane's body with it (#839, D3 steps 1 to 5; Acceptance 1, 2 and
//! 8): the plan finds the body (`hub.advertised`), the act moves the TUI, the observation sees
//! it, `hub.rebind` re-points the body, and one compare-and-swap records the session of record,
//! SHOWN and DRIVEN together. A failure before the rebind leaves the body where it was; a failed
//! rebind is a failure after the act that writes nothing.

use std::sync::Arc;

use holler_cli::output::Format;
use holler_pane::hub::PromptMode;
use holler_pane::ports::Activity;
use holler_pane::PaneError;
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::harness::HarnessOp as H;
use holler_pane_testkit::hub::HubOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::{advertised, body, body_name, drive};
use crate::doctor::rig::{Rig, Seed};
use crate::switch::{
    argv, both, both_with, data, failed, failed_before_the_act, with_s2, Case, SeamHarness,
    SeamLog, P, RECONCILE_P,
};
use crate::verb_harness::run_verb_with;

/// A rig with `P` driven by its body and a second session S2 on its server.
fn driven_with_s2() -> (Rig, String) {
    let (rig, s2) = with_s2(&[Seed::new(P, 1, 1)]);
    drive(&rig, P);
    (rig, s2)
}

/// `pane switch P <the case's S2>`.
fn switch_to() -> impl Fn(&Rig, &String) -> Vec<String> + Copy {
    |_, s2| argv(&["pane", "switch", P, s2])
}

fn reset_p(_: &Rig, (): &()) -> Vec<String> {
    argv(&["pane", "reset", P])
}

/// I2 extended to DRIVEN: the session of record, SHOWN, the id the body advertises and the
/// recorded DRIVEN are one id, written by one `cas_put`.
fn assert_all_agree<T>(case: &Case<T>, target: &str) {
    let f = case.format;
    let after = case.record(P);
    assert_eq!(
        after.session_of_record.as_deref(),
        Some(target),
        "{f:?}: the record"
    );
    assert_eq!(
        after.last_observed.shown.as_deref(),
        Some(target),
        "{f:?}: SHOWN"
    );
    assert_eq!(
        after.last_observed.driven.as_deref(),
        Some(target),
        "{f:?}: DRIVEN"
    );
    assert_eq!(case.shown(P).as_deref(), Some(target), "{f:?}: the TUI");
    let body = advertised(&case.rig, &body_name(P));
    assert_eq!(body.harness_session_id, target, "{f:?}: the body follows");
    assert_eq!(
        after.generation,
        case.before(P).generation + 1,
        "{f:?}: one write"
    );
    let writes = case
        .calls
        .panes
        .iter()
        .filter(|op| **op == PaneStoreOp::CasPut)
        .count();
    assert_eq!(writes, 1, "{f:?}: one cas_put: {:?}", case.calls.panes);
}

/// Acceptance 1 and 2, switch: the body advertising the old session is re-pointed to the target
/// after the observation and before the record; the rebind names the target, which is not yet of
/// record at that moment.
#[test]
fn a_switch_re_points_the_body_between_the_observation_and_the_record() {
    let cases = both(driven_with_s2, switch_to());
    data(&cases);
    for case in &cases {
        assert_all_agree(case, &case.setup);
        assert_eq!(
            case.calls.hub,
            [HubOp::Advertised, HubOp::Rebind],
            "{:?}",
            case.format
        );
        let rebind = case.rig.hub.log().remove(0);
        assert_eq!(
            (rebind.op, rebind.id.as_str()),
            (HubOp::Rebind, case.setup.as_str())
        );
        assert!(
            !rebind.of_record,
            "the rebind comes before the record write"
        );
        let select = case
            .calls
            .harness
            .iter()
            .position(|op| *op == H::SelectSession);
        let shown = case
            .calls
            .harness
            .iter()
            .position(|op| *op == H::ShownSession);
        assert!(
            select < shown && shown.is_some(),
            "{:?}",
            case.calls.harness
        );
    }
}

/// Acceptance 1 and 2, reset: the same order for a session the run creates.
#[test]
fn a_reset_re_points_the_body_to_the_session_it_created() {
    let build = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1)]);
        drive(&rig, P);
        (rig, ())
    };
    let cases = both(build, reset_p);
    data(&cases);
    for case in &cases {
        let new = case.record(P).session_of_record.expect("a new session");
        assert_ne!(new, case.rig.live(P).session);
        assert_all_agree(case, &new);
        assert_eq!(
            case.calls.hub,
            [HubOp::Advertised, HubOp::Rebind],
            "{:?}",
            case.format
        );
        let created = case
            .calls
            .harness
            .iter()
            .position(|op| *op == H::CreateSession);
        assert!(created.is_some(), "{:?}", case.calls.harness);
    }
}

/// The idempotent re-run: a body that already advertises the target is the driver (the target
/// rule) and is not asked to rebind; its id is the acknowledgement.
#[test]
fn a_switch_to_the_session_the_body_already_drives_does_not_rebind() {
    let build = || {
        let (rig, s2) = with_s2(&[Seed::new(P, 1, 1)]);
        body(&rig, &body_name(P), &s2, Activity::Idle);
        (rig, s2)
    };
    let cases = both(build, switch_to());
    data(&cases);
    for case in &cases {
        assert_all_agree(case, &case.setup);
        assert_eq!(
            case.calls.hub,
            [HubOp::Advertised],
            "{:?}: no rebind",
            case.format
        );
    }
}

/// D3 step 4's order, by faults: a `select_session` or a `shown_session` that fails leaves the
/// body where it was (the rebind comes after the observation), and the failure is after the act.
#[test]
fn a_failure_before_the_rebind_leaves_the_body_where_it_was() {
    for op in [H::SelectSession, H::ShownSession] {
        let build = move || {
            let (rig, s2) = driven_with_s2();
            let error = PaneError::Unavailable {
                what: "the TUI".to_owned(),
            };
            rig.harness.faults().fail_next(op, error);
            (rig, s2)
        };
        let cases = both(build, switch_to());
        for message in failed(&cases, 1, "unavailable") {
            assert!(message.ends_with(RECONCILE_P), "{op:?}: {message:?}");
        }
        for case in &cases {
            let old = &case.rig.live(P).session;
            assert_eq!(
                &advertised(&case.rig, &body_name(P)).harness_session_id,
                old
            );
            assert!(
                !case.calls.hub.contains(&HubOp::Rebind),
                "{op:?}: {:?}",
                case.calls.hub
            );
            assert_eq!(
                case.record(P),
                case.before(P).clone(),
                "{op:?}: nothing recorded"
            );
        }
    }
}

/// Acceptance 8: a refused rebind (the body is busy, the id is not on its endpoint, the endpoint
/// is silent) comes after the act: it fails with the refusal's class, says how to reconcile, and
/// records nothing; a wedged hub is a timeout.
#[test]
fn a_refused_rebind_is_a_failure_after_the_act_that_records_nothing() {
    let refusals = [
        (
            PaneError::Unavailable {
                what: "the body refused the rebind: session busy".to_owned(),
            },
            1,
            "unavailable",
        ),
        (
            PaneError::Unavailable {
                what: "session not found on the endpoint".to_owned(),
            },
            1,
            "unavailable",
        ),
        (
            PaneError::Timeout {
                op: "hub.rebind".to_owned(),
            },
            1,
            "timeout",
        ),
    ];
    for (error, exit, code) in refusals {
        let build = move || {
            let (rig, s2) = driven_with_s2();
            rig.hub.faults().fail_next(HubOp::Rebind, error.clone());
            (rig, s2)
        };
        let cases = both(build, switch_to());
        for message in failed(&cases, exit, code) {
            assert!(message.ends_with(RECONCILE_P), "acted: {message:?}");
        }
        for case in &cases {
            assert_eq!(
                case.record(P),
                case.before(P).clone(),
                "{code}: the record is unchanged"
            );
            assert!(
                !case.calls.panes.contains(&PaneStoreOp::CasPut),
                "{code}: no write"
            );
            let old = &case.rig.live(P).session;
            assert_eq!(
                &advertised(&case.rig, &body_name(P)).harness_session_id,
                old
            );
        }
    }
}

/// Acceptance 8: a driver that runs a turn or holds a question is refused in the plan, for
/// switch and reset alike, with nothing moved and the body not asked.
#[test]
fn a_busy_or_questioning_driver_is_refused_before_anything_moves() {
    let states = [
        (Activity::Busy, "session-busy"),
        (Activity::HoldingQuestion, "session-holds-question"),
    ];
    for (state, code) in states {
        let build = move || {
            let (rig, s2) = with_s2(&[Seed::new(P, 1, 1)]);
            body(&rig, &body_name(P), &rig.live(P).session, state);
            (rig, s2)
        };
        let cases = both(build, switch_to());
        failed_before_the_act(&cases, 3, code);
        let resets = both(
            move || {
                let (rig, _) = build();
                (rig, ())
            },
            reset_p,
        );
        failed_before_the_act(&resets, 3, code);
        for case in &cases {
            assert_nothing_moved(case);
        }
        for case in &resets {
            assert_nothing_moved(case);
        }
    }
}

fn assert_nothing_moved<T>(case: &Case<T>) {
    let f = case.format;
    case.assert_unchanged();
    for op in [H::CreateSession, H::SelectSession] {
        assert!(!case.calls.harness.contains(&op), "{f:?}: no {op:?}");
    }
    assert!(
        !case.calls.hub.contains(&HubOp::Rebind),
        "{f:?}: the body is not asked"
    );
}

/// Acceptance 8: one mapping of `Activity` to a refusal (`activity_refusal`) drives both the
/// harness's activity check (`check_idle`, a reset with no body) and the driver check (a body's
/// state): the four variants give the same answer through either.
#[test]
fn the_activity_table_is_the_same_through_the_harness_and_the_body() {
    let table = [
        (Activity::Idle, None),
        (Activity::Busy, Some("session-busy")),
        (Activity::HoldingQuestion, Some("session-holds-question")),
        (
            Activity::WaitingPermission,
            Some("session-holds-permission"),
        ),
    ];
    for (state, want) in table {
        let by_body = {
            let rig = Rig::new(&[Seed::new(P, 1, 1)]);
            body(&rig, &body_name(P), &rig.live(P).session, state);
            rig.verb(&["pane", "reset", P], Format::Json)
        };
        let log = SeamLog::default();
        let by_harness = {
            let rig = Rig::new(&[Seed::new(P, 1, 1)]);
            let seam = SeamHarness::new(&rig, P, &log, state, None);
            run_verb_with(&["pane", "reset", P], Format::Json, rig.ports_with(&seam))
        };
        for (how, run) in [("the body", &by_body), ("the harness", &by_harness)] {
            let got = (run.code != 0).then(|| {
                let envelope = check_envelope(&run.out, run.code).expect("an envelope");
                envelope.error.expect("an error").code
            });
            assert_eq!(got.as_deref(), want, "{state:?} through {how}: {run:?}");
        }
    }
}

/// A switch still queries no activity of the harness (E85): the driver's state comes from the
/// hub's advertisement, and `HarnessPort::session_activity` stays reset's alone.
#[test]
fn a_driven_switch_asks_the_harness_for_no_activity() {
    type Setup = (String, Arc<SeamLog>);
    let build = || {
        let (rig, s2) = driven_with_s2();
        (rig, (s2, Arc::new(SeamLog::default())))
    };
    let words = |_: &Rig, (s2, _): &Setup| argv(&["pane", "switch", P, s2]);
    let exec = |rig: &Rig, (_, log): &Setup, argv: &[&str], format| {
        let seam = SeamHarness::new(rig, P, log, Activity::Busy, None);
        run_verb_with(argv, format, rig.ports_with(&seam))
    };
    let cases = both_with(build, words, exec);
    data(&cases);
    for case in &cases {
        assert!(case.setup.1.activity().is_empty(), "no session_activity");
        assert!(case.setup.1.prompts().is_empty(), "no send_prompt");
        assert!(!case
            .rig
            .hub
            .log()
            .iter()
            .any(|c| c.mode == Some(PromptMode::Send)));
    }
}
