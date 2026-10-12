#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! The one pane-state predicate and the one selection of the prompt gate (#839, brief D1;
//! Acceptance 4 and 14): `holler_pane::gate`.
//!
//! - `prompt_refusal` decides the three refusals for the hub's gate and for `--pane` alike:
//!   parked, unhealthy, SHOWN differing from DRIVEN (both observed), checked in that order.
//! - `shown_driven_pair_differs` is the pair rule the predicate and `holler roster` share.
//! - `first_refusal` is the selection the hub's `pane_gate` and the test kit's `FakeHub` both
//!   call: the panes whose session of record **or recorded DRIVEN** is the id being prompted,
//!   judged in the slice's order (the store lists name-sorted), the first bad one named.
//!
//! Every id is `ses_`-shaped (D8): a fixture that passes only on a name-shaped session proves
//! nothing about a real pane.

mod common;

use holler_pane::gate::{
    first_refusal, prompt_refusal, shown_driven_pair_differs, PromptRefusal, PANE_PARKED,
    PANE_SHOWN_DRIVEN_MISMATCH, PANE_UNHEALTHY,
};
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::Pane;

const A: &str = "ses_000000000000000000000000a1";
const B: &str = "ses_000000000000000000000000b2";
const OLD: &str = "ses_000000000000000000000000c3";

/// A healthy, unparked pane `name` with `sor` as its session of record and the observation
/// `shown`/`driven`.
fn pane(name: &str, sor: Option<&str>, shown: Option<&str>, driven: Option<&str>) -> Pane {
    let mut pane = common::pane();
    pane.name = common::pane_name(name);
    pane.hold = Hold::None;
    pane.harness.health = Health::Healthy;
    pane.session_of_record = sor.map(str::to_owned);
    pane.last_observed = LastObserved {
        shown: shown.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}

fn parked(mut pane: Pane) -> Pane {
    pane.hold = Hold::Parked {
        reason: "disk full".to_owned(),
        release_when: "later".to_owned(),
        since: 5,
    };
    pane
}

fn unhealthy(mut pane: Pane) -> Pane {
    pane.harness.health = Health::Unhealthy("server-down".to_owned());
    pane
}

fn observed(shown: Option<&str>, driven: Option<&str>) -> LastObserved {
    LastObserved {
        shown: shown.map(str::to_owned),
        driven: driven.map(str::to_owned),
        at: 1,
    }
}

#[test]
fn the_three_codes_are_the_routed_verbs_codes() {
    assert_eq!(PANE_PARKED.as_str(), "pane-parked");
    assert_eq!(PANE_UNHEALTHY.as_str(), "pane-unhealthy");
    assert_eq!(
        PANE_SHOWN_DRIVEN_MISMATCH.as_str(),
        "pane-shown-driven-mismatch"
    );
    assert_eq!(PromptRefusal::Parked.code(), PANE_PARKED);
    assert_eq!(PromptRefusal::Unhealthy.code(), PANE_UNHEALTHY);
    assert_eq!(
        PromptRefusal::ShownDrivenMismatch.code(),
        PANE_SHOWN_DRIVEN_MISMATCH
    );
}

/// The pair rule: both sides observed and different. One side absent is never a mismatch.
#[test]
fn the_pair_differs_only_when_both_sides_are_observed_and_differ() {
    let table = [
        (Some(A), Some(B), true),
        (Some(A), Some(A), false),
        (Some(A), None, false),
        (None, Some(B), false),
        (None, None, false),
    ];
    for (shown, driven, differs) in table {
        assert_eq!(
            shown_driven_pair_differs(&observed(shown, driven)),
            differs,
            "shown {shown:?}, driven {driven:?}"
        );
    }
}

/// Parked, then unhealthy, then the pair: a pane that is all three is refused as parked.
#[test]
fn the_predicate_checks_parked_then_unhealthy_then_the_pair() {
    let healthy = pane("demo-c1r1", Some(A), Some(A), Some(A));
    assert_eq!(prompt_refusal(&healthy), None);
    let table = [
        (parked(healthy.clone()), PromptRefusal::Parked),
        (unhealthy(healthy.clone()), PromptRefusal::Unhealthy),
        (
            pane("demo-c1r1", Some(A), Some(A), Some(OLD)),
            PromptRefusal::ShownDrivenMismatch,
        ),
        (
            parked(unhealthy(pane("demo-c1r1", Some(A), Some(A), Some(OLD)))),
            PromptRefusal::Parked,
        ),
        (
            unhealthy(pane("demo-c1r1", Some(A), Some(A), Some(OLD))),
            PromptRefusal::Unhealthy,
        ),
    ];
    for (record, want) in table {
        assert_eq!(prompt_refusal(&record), Some(want), "{record:?}");
    }
    let one_side = pane("demo-c1r1", Some(A), Some(A), None);
    assert_eq!(
        prompt_refusal(&one_side),
        None,
        "one side absent: no mismatch"
    );
}

/// The first key: the session of record.
#[test]
fn the_selection_keys_on_the_session_of_record() {
    let panes = [parked(pane("demo-c1r1", Some(A), Some(A), Some(A)))];
    let (named, why) = first_refusal(&panes, A).expect("refused");
    assert_eq!(named.name.as_str(), "demo-c1r1");
    assert_eq!(why, PromptRefusal::Parked);
    assert_eq!(
        first_refusal(&panes, B),
        None,
        "another id is not this pane's"
    );
}

/// The second key: the recorded DRIVEN. A body that is behind advertises an id that no record
/// names as a session of record; once a pass recorded it as the pane's DRIVEN, the pair refuses
/// it (the window the ADR states: with no DRIVEN recorded nothing selects the pane).
#[test]
fn the_selection_keys_on_the_recorded_driven_too() {
    let behind = [pane("demo-c1r1", Some(A), Some(A), Some(OLD))];
    let (named, why) = first_refusal(&behind, OLD).expect("the DRIVEN key selects the pane");
    assert_eq!(named.name.as_str(), "demo-c1r1");
    assert_eq!(why, PromptRefusal::ShownDrivenMismatch);

    let never_observed = [pane("demo-c1r1", Some(A), Some(A), None)];
    assert_eq!(
        first_refusal(&never_observed, OLD),
        None,
        "no DRIVEN recorded: the id selects no pane (the stated window)"
    );
}

/// The session's name is no key any more: a pane whose session of record happens to be the
/// text `alpha` is not selected by an id, and an id that no pane names selects nothing.
#[test]
fn an_id_no_pane_names_selects_nothing() {
    let panes = [
        parked(pane(
            "demo-c1r1",
            Some("alpha"),
            Some("alpha"),
            Some("alpha"),
        )),
        parked(pane("demo-c2r1", Some(B), Some(B), Some(B))),
    ];
    assert_eq!(first_refusal(&panes, A), None);
}

/// Two panes that record one id are judged together, fail closed: any bad one refuses, and the
/// first bad one in the slice's order (name order, as the store lists) is the one named, whether
/// it sorts before or after the good one, and whichever key selected it.
#[test]
fn two_panes_on_one_id_fail_closed_and_the_first_bad_is_named() {
    let good = pane("demo-c1r1", Some(A), Some(A), Some(A));
    let bad_later = parked(pane("demo-c2r1", Some(A), Some(A), Some(A)));
    let panes = [good.clone(), bad_later];
    let (named, why) = first_refusal(&panes, A).expect("any bad pane refuses");
    assert_eq!(
        (named.name.as_str(), why),
        ("demo-c2r1", PromptRefusal::Parked)
    );

    let first_bad = unhealthy(pane("demo-c1r1", Some(A), Some(A), Some(A)));
    let second_bad = parked(pane("demo-c2r1", Some(A), Some(A), Some(A)));
    let panes = [first_bad, second_bad];
    let (named, why) = first_refusal(&panes, A).expect("refused");
    assert_eq!(
        (named.name.as_str(), why),
        ("demo-c1r1", PromptRefusal::Unhealthy),
        "the first bad pane in order is named"
    );

    // One pane records the id as its session of record, the other as its stale DRIVEN.
    let by_driven = pane("demo-c1r1", Some(B), Some(B), Some(A));
    let panes = [by_driven, good];
    let (named, why) = first_refusal(&panes, A).expect("the DRIVEN key refuses");
    assert_eq!(
        (named.name.as_str(), why),
        ("demo-c1r1", PromptRefusal::ShownDrivenMismatch)
    );
}

#[test]
fn healthy_panes_on_the_id_admit_it() {
    let panes = [
        pane("demo-c1r1", Some(A), Some(A), Some(A)),
        pane("demo-c2r1", Some(A), Some(A), None),
    ];
    assert_eq!(first_refusal(&panes, A), None);
}
