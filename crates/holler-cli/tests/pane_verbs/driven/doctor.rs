//! `pane doctor` observes DRIVEN (#839, D3 "Reconcile"; Acceptance 6, 19 and 20's doctor cases):
//! one `hub.advertised` per pass, `last_observed.driven` recorded with `shown` in the same write,
//! and `shown-driven-mismatch` also reported when the observed DRIVEN is neither SHOWN nor the
//! session of record. A body that is behind is `not-fixable` with the finding's own remedy, a
//! `pane switch` back to the session of record; a body that cannot be told is reported with no
//! remedy. Once DRIVEN is recorded, the hub's gate sees the body that is behind (19).

use holler_cli::output::Format;
use holler_pane::findings::{FindingKind as K, FixState};
use holler_pane::hub::{HubPort, PromptMode};
use holler_pane::ports::Activity;
use holler_pane_testkit::fault::Fault;
use holler_pane_testkit::fixture::session_id;
use holler_pane_testkit::hub::HubOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::{advertised, body, body_name};
use crate::doctor::rig::{json_data, json_findings, keys, of_kind, one, whole, Rig, Seed};
use crate::switch::{P, Q};

/// One pane P whose body is behind: it advertises `old`, while the record and the TUI are on the
/// session of record. `listed` puts `old` on P's server (a body that reverted to its configured
/// session); otherwise `old` is an id the server no longer lists (a relaunch onto a new session).
fn behind(listed: bool) -> (Rig, String) {
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    let old = if listed {
        rig.harness.seed_session(rig.live(P).port)
    } else {
        session_id(77)
    };
    body(&rig, &body_name(P), &old, Activity::Idle);
    (rig, old)
}

fn switch_remedy(rig: &Rig) -> String {
    format!("holler pane switch {P} {}", rig.live(P).session)
}

/// A reverted body (its configured session, listed by P's own server): exactly one
/// `shown-driven-mismatch`, `not-fixable`, whose remedy is the switch back; DRIVEN recorded in the
/// pass's one write; the five `FixState` names unchanged. The switch then re-points the body.
#[test]
fn a_reverted_body_is_reported_with_the_switch_remedy_and_the_switch_repairs_it() {
    let (rig, old) = behind(true);
    let mark = rig.mark();
    let run = rig.verb(&["pane", "doctor"], Format::Json);
    let data = json_data(&run);
    let calls = rig.calls_since(&mark);
    assert_eq!(calls.hub, [HubOp::Advertised], "one observation per pass");
    let writes = calls
        .panes
        .iter()
        .filter(|op| **op == PaneStoreOp::CasPut)
        .count();
    assert_eq!(
        writes, 1,
        "DRIVEN goes in the pass's one write: {:?}",
        calls.panes
    );
    let found = json_findings(&data, "shown-driven-mismatch");
    assert_eq!(found.len(), 1, "{data}");
    assert_eq!(found[0]["fix"], "not-fixable");
    assert_eq!(found[0]["remedy"], switch_remedy(&rig).as_str());
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(old.as_str())
    );
    let names = [
        FixState::NotFixable,
        FixState::Fixable,
        FixState::Skipped,
        FixState::Fixed,
        FixState::Failed,
    ]
    .map(FixState::as_str);
    assert_eq!(
        names,
        ["not-fixable", "fixable", "skipped", "fixed", "failed"],
        "no new state"
    );

    let sor = rig.live(P).session.clone();
    let switched = rig.verb(&["pane", "switch", P, &sor], Format::Json);
    assert_eq!(switched.code, 0, "{switched:?}");
    assert_eq!(advertised(&rig, &body_name(P)).harness_session_id, sor);
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(sor.as_str())
    );
    let report = rig.run(&whole(false));
    assert!(
        of_kind(&report, K::ShownDrivenMismatch).is_empty(),
        "{:?}",
        keys(&report)
    );
}

/// A relaunch onto a new session (E129): the body is on an id the restarted server no longer
/// lists, and the record's stored DRIVEN finds it (rule (ii)). The same case with no stored
/// DRIVEN finds no body and reports nothing: the stated window, pinned so it cannot change
/// silently.
#[test]
fn a_body_left_behind_by_a_relaunch_is_found_by_the_stored_driven_only() {
    let (rig, old) = behind(false);
    rig.rewrite(P, |p| p.last_observed.driven = Some(old.clone()));
    let report = rig.run(&whole(false));
    let mismatch = one(&report, K::ShownDrivenMismatch, P);
    assert_eq!(mismatch.fix, FixState::NotFixable);
    assert_eq!(
        mismatch.remedy.as_deref(),
        Some(switch_remedy(&rig).as_str())
    );
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(old.as_str())
    );
    let sor = rig.live(P).session.clone();
    assert_eq!(rig.verb(&["pane", "switch", P, &sor], Format::Json).code, 0);
    assert_eq!(advertised(&rig, &body_name(P)).harness_session_id, sor);

    let (window, _) = behind(false);
    let report = window.run(&whole(false));
    assert!(
        of_kind(&report, K::ShownDrivenMismatch).is_empty(),
        "{:?}",
        keys(&report)
    );
    assert_eq!(
        window.record(P).last_observed.driven,
        None,
        "nothing observed"
    );
}

/// A body on the session of record agrees: no finding, and DRIVEN is recorded with SHOWN. A TUI
/// moved by hand while the body drives the record is the mismatch `--fix` repairs, as before.
#[test]
fn an_agreeing_body_is_recorded_and_a_moved_tui_is_still_fixable() {
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    super::drive(&rig, P);
    let report = rig.run(&whole(false));
    assert!(
        of_kind(&report, K::ShownDrivenMismatch).is_empty(),
        "{:?}",
        keys(&report)
    );
    let sor = rig.live(P).session.clone();
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(sor.as_str())
    );

    rig.move_tui_to_new_session(P);
    let report = rig.run(&whole(false));
    let mismatch = one(&report, K::ShownDrivenMismatch, P);
    assert_eq!(mismatch.fix, FixState::Fixable);
    assert_eq!(
        mismatch.remedy.as_deref(),
        Some(format!("holler pane doctor {P} --fix").as_str())
    );
}

/// A failed `hub.advertised` is an `observe-failed` finding naming it, and DRIVEN is left as
/// stored (the rule for every failed read).
#[test]
fn a_failed_advertised_is_observe_failed_and_leaves_driven_as_stored() {
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    let stored = session_id(5);
    rig.rewrite(P, |p| p.last_observed.driven = Some(stored.clone()));
    rig.hub.faults().set(Some(Fault::Wedged));
    let report = rig.run(&whole(false));
    let failed = of_kind(&report, K::ObserveFailed);
    assert!(
        failed.iter().any(|f| f.message.contains("hub.advertised")),
        "{:?}",
        keys(&report)
    );
    assert_eq!(
        rig.record(P).last_observed.driven.as_deref(),
        Some(stored.as_str())
    );
}

/// 20 (a) through doctor: on a shared listing, two bodies no evidence names leave every pane they
/// could be unproven: one `shown-driven-mismatch` per pane, `not-fixable`, no remedy, and DRIVEN
/// recorded `None` (a stale stored value is overwritten: nothing is observed).
#[test]
fn unproven_bodies_are_reported_per_pane_with_no_remedy() {
    let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
    let port = rig.live(P).port;
    body(
        &rig,
        "kiwi/sa",
        &rig.harness.seed_session(port),
        Activity::Idle,
    );
    body(
        &rig,
        "lime/sb",
        &rig.harness.seed_session(port),
        Activity::Idle,
    );
    rig.rewrite(P, |p| p.last_observed.driven = Some(session_id(5)));
    let report = rig.run(&whole(false));
    for pane in [P, Q] {
        let mismatch = one(&report, K::ShownDrivenMismatch, pane);
        assert_eq!(mismatch.fix, FixState::NotFixable, "{pane}");
        assert_eq!(
            mismatch.remedy, None,
            "{pane}: no verb repairs it unattended"
        );
        assert!(
            mismatch.message.contains("kiwi/sa"),
            "{pane}: {}",
            mismatch.message
        );
        assert_eq!(rig.record(pane).last_observed.driven, None, "{pane}");
    }
}

/// 20 (b) through doctor: a pane with no body, beside another pane's behind body that no evidence
/// names, records DRIVEN `None` and is reported with no remedy; with that body on the other
/// pane's recorded DRIVEN instead, it is that pane's, found by rule (ii), with the switch remedy.
#[test]
fn another_pane_s_behind_body_is_never_this_pane_s() {
    let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
    let cfg = rig.harness.seed_session(rig.live(Q).port);
    body(&rig, &body_name(Q), &cfg, Activity::Idle);
    let report = rig.run(&whole(false));
    let p = one(&report, K::ShownDrivenMismatch, P);
    assert_eq!((p.fix, p.remedy.clone()), (FixState::NotFixable, None));
    assert_eq!(rig.record(P).last_observed.driven, None);

    let rig = Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]);
    let d2 = rig.harness.seed_session(rig.live(Q).port);
    body(&rig, &body_name(Q), &d2, Activity::Idle);
    rig.rewrite(Q, |q| q.last_observed.driven = Some(d2.clone()));
    let report = rig.run(&whole(false));
    let about_p: Vec<_> = of_kind(&report, K::ShownDrivenMismatch)
        .into_iter()
        .filter(|f| f.pane.as_ref().is_some_and(|n| n.as_str() == P))
        .collect();
    assert!(
        about_p.is_empty(),
        "P has no body and is not told otherwise: {:?}",
        keys(&report)
    );
    let q = one(&report, K::ShownDrivenMismatch, Q);
    let remedy = format!("holler pane switch {Q} {}", rig.live(Q).session);
    assert_eq!(q.remedy.as_deref(), Some(remedy.as_str()));
}

/// Acceptance 19 on the fakes: a body that is behind is not gated until DRIVEN is recorded (the
/// stated window); after `pane doctor` records it, the hub refuses it as
/// `pane-shown-driven-mismatch`; after `pane switch` re-points it, a prompt to the session of
/// record is delivered.
#[test]
fn the_gate_sees_a_behind_body_once_doctor_records_it() {
    let (rig, old) = behind(true);
    assert_eq!(
        rig.hub.prompt(&old, "hi", PromptMode::Check),
        Ok(()),
        "the stated window"
    );
    rig.run(&whole(false));
    let refused = rig
        .hub
        .prompt(&old, "hi", PromptMode::Check)
        .expect_err("now gated");
    assert_eq!(refused.code().to_string(), "pane-shown-driven-mismatch");
    let sor = rig.live(P).session.clone();
    assert_eq!(rig.verb(&["pane", "switch", P, &sor], Format::Json).code, 0);
    assert_eq!(rig.hub.prompt(&sor, "hi", PromptMode::Send), Ok(()));
    assert_eq!(rig.hub.prompts().len(), 1);
}
