//! What "healthy" observes (brief §2, C9, C12 and O2; C14 entries 24 and 28, and plan-review
//! warn 1). A recorded pane is healthy when its `health` answers `Ok(true)`, Herdr's snapshot
//! lists its `(session, pane_id)` pair (no other field, the label included: ADR-0021 section 2
//! after #808), and its TUI does not show another session than the session of record.

use holler_pane::tx_apply::{Applied, Outcome as StepOutcome};
use holler_pane::{HerdrPane, Pane, PaneError};
use holler_pane_testkit::harness::HarnessOp;

use super::rig::{
    assert_only_observed, cell, options, outcomes, request, spec_of, step, Rig, Seed, P,
};

/// One recorded pane of P at r1c1, healthy and matching its spec, as `role` decides.
fn one_pane(orchestrator: bool) -> (Rig, Pane) {
    let rig = Rig::new();
    let seed = Seed::at("pane-c", cell(1, 1), 48100);
    let seed = if orchestrator {
        seed.orchestrator()
    } else {
        seed
    };
    let c = rig.seed_live(&seed);
    rig.set_specs(P, vec![spec_of(&c)]);
    (rig, c)
}

/// Run the engine (live, no flags), assert `pane-c`'s outcome and, unless it was relaunched,
/// that nothing but apply's observations was called and nothing written; the result.
fn assert_outcome(rig: &Rig, outcome: StepOutcome, what: &str) -> Applied {
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(outcomes(&applied), vec![("pane-c", outcome)], "{what}");
    if outcome != StepOutcome::Relaunched {
        assert_only_observed(&calls);
        assert_eq!(rig.writes_since(before), vec![], "{what}: no write");
    }
    applied
}

/// Entry 24 (C9, C7 R2 NV-5 and NV-6): an orchestrator that matches its spec, whose `health`
/// answers `Ok(true)` and whose TUI shows its session of record, is still not healthy when Herdr
/// does not list its `(session, pane_id)`: after its Herdr pane vanished, and when its record
/// names another Herdr session beside a pane id the fake holds (which an apply matching the pane
/// id alone would adopt). Each time it is `refused` (`orchestrator-not-relaunched`).
#[test]
fn an_orchestrator_herdr_does_not_list_is_refused_not_adopted() {
    let (rig, c) = one_pane(true);
    rig.herdr.vanish(&c.herdr.pane_id).unwrap();
    assert_outcome(&rig, StepOutcome::Refused, "vanished");

    let (rig, c) = one_pane(true);
    let elsewhere = Pane {
        herdr: HerdrPane {
            session: "elsewhere".to_owned(),
            ..c.herdr.clone()
        },
        ..c
    };
    rig.panes.concurrent_put(&elsewhere).unwrap();
    let applied = assert_outcome(&rig, StepOutcome::Refused, "another Herdr session");
    assert_eq!(
        step(&applied, "pane-c").code.as_deref(),
        Some("orchestrator-not-relaunched")
    );
}

/// Plan-review warn 1 (#808): a pane Herdr reports with a label is still the recorded pane (the
/// label is observed, never recorded, and no comparison of a record with a snapshot entry looks
/// at it), so a healthy matching pane with a label is `adopted`, not relaunched.
#[test]
fn a_labelled_pane_is_still_listed_and_adopted() {
    let (rig, c) = one_pane(false);
    rig.herdr.rename(&c.herdr.pane_id, "hj-c1r1").unwrap();
    assert_outcome(&rig, StepOutcome::Adopted, "labelled");
}

/// Entry 28 (C12, O2): only a TUI that shows another session makes a pane unhealthy. On its home
/// screen (`Ok(None)`) and when `shown_session` fails (`Err`), the pane is `adopted` and nothing
/// live is touched; when it shows another session of its server, the agent is `relaunched`
/// (row f), and its TUI then shows the session of record again.
#[test]
fn the_shown_session_decides_only_on_another_session() {
    let (rig, c) = one_pane(false);
    rig.harness.navigate(&c.herdr.pane_id, None).unwrap();
    assert_outcome(&rig, StepOutcome::Adopted, "home screen");

    let (rig, _) = one_pane(false);
    rig.harness.faults().fail_next(
        HarnessOp::ShownSession,
        PaneError::Unavailable {
            what: "tui".to_owned(),
        },
    );
    assert_outcome(&rig, StepOutcome::Adopted, "unreadable");

    let (rig, c) = one_pane(false);
    let other = rig.harness.seed_session(48100);
    rig.harness
        .navigate(&c.herdr.pane_id, Some(&other))
        .unwrap();
    assert_outcome(&rig, StepOutcome::Relaunched, "another session");
    let after = rig.record("pane-c").unwrap();
    let shown = rig.harness.tui(&after.herdr.pane_id).and_then(|t| t.shown);
    assert_eq!(shown, after.session_of_record);
    assert_eq!(
        after.session_of_record, c.session_of_record,
        "the session kept"
    );
}
