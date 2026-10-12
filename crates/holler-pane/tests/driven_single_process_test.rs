#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #850
//! `driver_of` over a single process (#850, Acceptance 6; ADR-0021 section 15 item 19, the drive
//! rule): a Claude Code pane has no server, so the plan, the drive step and reconcile pass an
//! **empty server list**, and the body that drives the pane is found by its evidence alone: the
//! session of record, the recorded DRIVEN, the switch's target. With no evidence there is no
//! body (the listed fallback needs a list), and a body of another pane's evidence is never a
//! candidate.
//!
//! `driver_of` is #839's pure join and is not changed by #850; these cases pin the property the
//! drive rule relies on (they pass before the story). Ids are UUIDs, as a Claude Code pane's are.

mod common;

use holler_pane::driven::{driver_of, Driver};
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::ports::Activity;
use holler_pane::Pane;

const SOR: &str = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11";
const OLD: &str = "1c7a9e30-8d2b-4f65-a1c4-3e5d7b9f2c42";
const TARGET: &str = "2d8bafa1-9e3c-4076-b2d5-4f6e8cafd353";
const OTHER_SOR: &str = "3e9cb0b2-af4d-4187-83e6-507f9dbae464";

fn pane(name: &str, sor: Option<&str>, driven: Option<&str>) -> Pane {
    let mut pane = common::pane();
    pane.name = common::pane_name(name);
    pane.hold = Hold::None;
    pane.harness.health = Health::Healthy;
    pane.session_of_record = sor.map(str::to_owned);
    pane.last_observed = LastObserved {
        shown: sor.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}

fn body(label: &str, id: &str) -> holler_pane::hub::Advertised {
    holler_pane::hub::Advertised {
        token: format!("tok_{label}"),
        name: format!("{label}/c1r1"),
        harness_session_id: id.to_owned(),
        state: Activity::Idle,
    }
}

/// The server list of a single process: there is none.
const NO_LIST: &[String] = &[];

#[test]
fn a_body_on_the_session_of_record_drives_the_pane() {
    let me = pane("hj-c1r2", Some(SOR), None);
    let bodies = [body("kiwi", SOR)];
    assert_eq!(driver_of(&me, &bodies, NO_LIST, &[], None), Driver::One(&bodies[0]));
}

#[test]
fn a_body_that_is_behind_is_found_by_the_stored_driven() {
    let me = pane("hj-c1r2", Some(SOR), Some(OLD));
    let bodies = [body("kiwi", OLD)];
    assert_eq!(driver_of(&me, &bodies, NO_LIST, &[], None), Driver::One(&bodies[0]));
}

#[test]
fn a_switch_target_names_the_body_that_already_drives_it() {
    let me = pane("hj-c1r2", Some(SOR), None);
    let bodies = [body("kiwi", TARGET)];
    assert_eq!(driver_of(&me, &bodies, NO_LIST, &[], Some(TARGET)), Driver::One(&bodies[0]));
    assert_eq!(driver_of(&me, &bodies, NO_LIST, &[], None), Driver::None, "a reset has no target");
}

#[test]
fn with_no_evidence_there_is_no_body_because_there_is_no_list_to_fall_back_on() {
    let me = pane("hj-c1r2", Some(SOR), None);
    let bodies = [body("kiwi", OLD)];
    assert_eq!(driver_of(&me, &bodies, NO_LIST, &[], None), Driver::None);
    assert_eq!(driver_of(&me, &[], NO_LIST, &[], None), Driver::None, "and none advertised is none");
}

#[test]
fn a_body_on_another_pane_s_session_is_never_this_pane_s() {
    let me = pane("hj-c1r2", Some(SOR), None);
    let other = pane("hj-c1r3", Some(OTHER_SOR), None);
    let bodies = [body("plum", OTHER_SOR)];
    assert_eq!(driver_of(&me, &bodies, NO_LIST, std::slice::from_ref(&other), None), Driver::None);
    assert_eq!(
        driver_of(&me, &bodies, NO_LIST, std::slice::from_ref(&other), Some(OTHER_SOR)),
        Driver::None,
        "not even as a switch's target: the exclusion rule comes first"
    );
}

#[test]
fn two_bodies_on_one_session_are_ambiguous() {
    let me = pane("hj-c1r2", Some(SOR), None);
    let bodies = [body("kiwi", SOR), body("plum", SOR)];
    assert_eq!(
        driver_of(&me, &bodies, NO_LIST, &[], None),
        Driver::Ambiguous(vec![&bodies[0], &bodies[1]])
    );
}
