#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)] // #651
//! Acceptance: the pane an action targets is resolved from the registry — the
//! `HERDR_PANE_ID` the action process receives maps to the record whose
//! `herdr.pane_id` equals it, by pane name (so the action runs `holler pane
//! <verb> NAME`). The plugin never resolves a target any other way.

use herdr_holler::resolve_action_pane;
use holler_pane::{Pane, PaneId};
use holler_pane_testkit::fixture::sample_pane;

/// A sample fixture pane re-addressed to Herdr pane id `pane_id`.
fn pane_with_id(name: &str, pane_id: &str) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    pane.herdr.pane_id = PaneId::new(pane_id);
    pane
}

#[test]
fn resolves_the_registry_pane_by_herdr_pane_id() {
    let panes = [
        pane_with_id("demo-alpha", "w1:p1"),
        pane_with_id("demo-beta", "w1:p2"),
        pane_with_id("demo-gamma", "w2:p1"),
    ];
    let resolved = resolve_action_pane(&panes, "w1:p2");
    assert_eq!(
        resolved.as_ref().map(|name| name.as_str().to_owned()),
        Some("demo-beta".to_owned()),
        "HERDR_PANE_ID w1:p2 must resolve to the record whose herdr pane id it is"
    );
}

#[test]
fn an_unknown_herdr_pane_id_resolves_to_none() {
    let panes = [pane_with_id("demo-alpha", "w1:p1")];
    assert!(
        resolve_action_pane(&panes, "w9:p9").is_none(),
        "a Herdr pane id no record carries must not resolve"
    );
}
