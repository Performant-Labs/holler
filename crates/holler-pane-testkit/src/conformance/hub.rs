//! The `HubPort` conformance suite (#839, brief D10): [`run_hub_conformance`] runs the cases
//! [`hub_cases`] lists against any `HubPort`, each against a fresh hub. It runs against
//! `FakeHub`, and against the real client (`holler-cli`'s `wiring/hub.rs`) over a real hub and
//! body in `driven_e2e_test.rs`, so the fake the verbs' tests use is held to what the hub does.
//!
//! What it pins is the hub's prompt gate as `HubPort::prompt` with `Check` answers it: the
//! selection `holler_pane::gate::first_refusal` makes, by its two keys (a pane's session of
//! record and its recorded DRIVEN), fail closed across panes that record one id, the first bad
//! pane in name order named; and an id no live body advertises is `pane-not-driven`. No case
//! sends a prompt: a real body is never asked to run a turn.
//!
//! The fixture ([`HubFixture`]) is two harness session ids, each advertised by one live, idle
//! attach body with no hold, and a pane store that starts empty, through which a case writes the
//! records the gate reads. Pane names are neutral (`demo-c1r1`, `demo-c2r1`).

use std::sync::Arc;

use holler_pane::hub::{HubPort, PromptMode};
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::{Pane, PaneStore};

use super::{expect_code, run_cases, succeeds, Conformance};
use crate::fixture::{sample_pane, session_id};

/// What a case runs against beside the hub: the store the hub's gate reads, empty at the start,
/// and two ids, each advertised by one live, idle attach body with no hold.
pub struct HubFixture {
    pub panes: Arc<dyn PaneStore>,
    pub ids: [String; 2],
}

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn HubPort, &HubFixture) -> Result<(), String>;

/// The pane names of the cases, in name order.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";

/// The suite, in order: the one table that the runner iterates and [`hub_cases`] lists.
const CASES: [(&str, Case); 7] = [
    (
        "advertised-names-each-live-body",
        advertised_names_each_live_body,
    ),
    ("a-healthy-pane-is-admitted", a_healthy_pane_is_admitted),
    (
        "the-session-of-record-selects-a-pane",
        the_session_of_record_selects_a_pane,
    ),
    (
        "the-recorded-driven-selects-a-pane",
        the_recorded_driven_selects_a_pane,
    ),
    (
        "two-panes-on-one-id-fail-closed",
        two_panes_on_one_id_fail_closed,
    ),
    (
        "the-first-bad-pane-in-name-order-is-named",
        the_first_bad_pane_in_name_order_is_named,
    ),
    (
        "an-id-no-body-advertises-is-pane-not-driven",
        an_id_no_body_advertises_is_pane_not_driven,
    ),
];

/// The ids of the cases [`run_hub_conformance`] runs, in the order it runs them.
pub fn hub_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`hub_cases`], in order, each against a fresh hub and fixture, and return
/// every case that did not hold. `fresh` is called once per case; it returns the hub, its
/// fixture and a guard kept alive for that case only.
pub fn run_hub_conformance<S, K, F>(mut fresh: F) -> Conformance
where
    S: HubPort,
    F: FnMut() -> (S, HubFixture, K),
{
    let fresh = move || {
        let (hub, fixture, guard) = fresh();
        ((hub, fixture), guard)
    };
    run_cases(&CASES, fresh, |case, (hub, fixture)| case(hub, fixture))
}

// --- the cases ---

/// Case 1: `advertised` names both ids.
fn advertised_names_each_live_body(hub: &dyn HubPort, fixture: &HubFixture) -> Result<(), String> {
    let bodies = succeeds("advertised", hub.advertised())?;
    for id in &fixture.ids {
        if !bodies.iter().any(|body| &body.harness_session_id == id) {
            return Err(format!("advertised names no body on {id}: {bodies:?}"));
        }
    }
    Ok(())
}

/// Case 2: a healthy pane whose session of record is the first id is admitted by `Check`.
fn a_healthy_pane_is_admitted(hub: &dyn HubPort, fixture: &HubFixture) -> Result<(), String> {
    let id = &fixture.ids[0];
    put(fixture, pane(C1, id, Some(id))?)?;
    succeeds(
        "prompt (Check) to a healthy pane",
        hub.prompt(id, "hi", PromptMode::Check),
    )
}

/// Case 3: a parked pane on its session of record refuses `pane-parked`, naming the pane.
fn the_session_of_record_selects_a_pane(
    hub: &dyn HubPort,
    fixture: &HubFixture,
) -> Result<(), String> {
    let id = &fixture.ids[0];
    put(fixture, parked(pane(C1, id, Some(id))?))?;
    refused_naming(hub, id, "pane-parked", C1)
}

/// Case 4: a pane whose session of record no body advertises, and whose recorded DRIVEN is the
/// first id, refuses a prompt to that id `pane-shown-driven-mismatch`: the DRIVEN key.
fn the_recorded_driven_selects_a_pane(
    hub: &dyn HubPort,
    fixture: &HubFixture,
) -> Result<(), String> {
    let elsewhere = session_id(0x0c3);
    put(fixture, pane(C1, &elsewhere, Some(&fixture.ids[0]))?)?;
    refused_naming(hub, &fixture.ids[0], "pane-shown-driven-mismatch", C1)
}

/// Case 5: a healthy and an unhealthy pane that both record the first id: fail closed,
/// `pane-unhealthy`, naming the bad one.
fn two_panes_on_one_id_fail_closed(hub: &dyn HubPort, fixture: &HubFixture) -> Result<(), String> {
    let id = &fixture.ids[0];
    put(fixture, pane(C1, id, Some(id))?)?;
    put(fixture, unhealthy(pane(C2, id, Some(id))?))?;
    refused_naming(hub, id, "pane-unhealthy", C2)
}

/// Case 6: two bad panes on the first id: the first in name order is the one named.
fn the_first_bad_pane_in_name_order_is_named(
    hub: &dyn HubPort,
    fixture: &HubFixture,
) -> Result<(), String> {
    let id = &fixture.ids[0];
    put(fixture, parked(pane(C2, id, Some(id))?))?;
    put(fixture, unhealthy(pane(C1, id, Some(id))?))?;
    refused_naming(hub, id, "pane-unhealthy", C1)
}

/// Case 7: an id no live body advertises is `pane-not-driven`.
fn an_id_no_body_advertises_is_pane_not_driven(
    hub: &dyn HubPort,
    _: &HubFixture,
) -> Result<(), String> {
    let nobody = session_id(0x0dead);
    expect_code(
        "prompt (Check) to an id no body advertises",
        hub.prompt(&nobody, "hi", PromptMode::Check),
        "pane-not-driven",
    )
}

// --- helpers ---

/// A healthy, unparked pane `name` whose session of record and SHOWN are `record` and whose
/// recorded DRIVEN is `driven`.
fn pane(name: &str, record: &str, driven: Option<&str>) -> Result<Pane, String> {
    let mut pane = succeeds("sample_pane", sample_pane(name))?;
    pane.hold = Hold::None;
    pane.harness.health = Health::Healthy;
    pane.session_of_record = Some(record.to_owned());
    pane.last_observed = LastObserved {
        shown: Some(record.to_owned()),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    Ok(pane)
}

fn parked(mut pane: Pane) -> Pane {
    pane.hold = Hold::Parked {
        reason: "conformance".to_owned(),
        release_when: "after the case".to_owned(),
        since: 1,
    };
    pane
}

fn unhealthy(mut pane: Pane) -> Pane {
    pane.harness.health = Health::Unhealthy("server-down".to_owned());
    pane
}

/// Store `pane` as a new record.
fn put(fixture: &HubFixture, pane: Pane) -> Result<(), String> {
    succeeds("cas_put", fixture.panes.cas_put(&pane, 0)).map(drop)
}

/// A `Check` of `id` is refused with `code`, and its message names the pane `named`.
fn refused_naming(hub: &dyn HubPort, id: &str, code: &str, named: &str) -> Result<(), String> {
    let answer = hub.prompt(id, "hi", PromptMode::Check);
    let message = match &answer {
        Err(error) => error.to_string(),
        Ok(()) => String::new(),
    };
    expect_code("prompt (Check)", answer, code)?;
    if message.contains(named) {
        Ok(())
    } else {
        Err(format!("the refusal names {named}: {message:?}"))
    }
}
