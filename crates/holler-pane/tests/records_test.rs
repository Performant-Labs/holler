#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! Serde forms of the stored records (#637 AC 4): `Pane`, `Profile`, `ProfileSpec`,
//! and the compare-and-swap rule a `PaneStore` enforces on `generation`.
//!
//! The JSON here is the epic's contract section. #639 and #661 persist these forms
//! and #649 reads them over the control socket, so the field names are the contract.

mod common;

use common::{pane, pane_json, profile, profile_json, spec_json, MemPaneStore};
use holler_pane::{Pane, PaneStore, Profile, ProfileSpec};
use serde_json::{json, Value};

/// The value at a JSON pointer, `null` when absent (an absent optional equals null).
fn at(v: &Value, pointer: &str) -> Value {
    v.pointer(pointer).cloned().unwrap_or(Value::Null)
}

#[test]
fn pane_full_round_trips() {
    let want = pane_json();
    let pane: Pane = serde_json::from_value(want.clone()).unwrap();

    assert_eq!(pane.name.as_str(), "hj-c1r1");
    assert_eq!(pane.generation, 7);
    assert_eq!(
        pane.profile.as_ref().map(|p| p.as_str()),
        Some("Some Profile")
    );

    assert_eq!(serde_json::to_value(&pane).unwrap(), want);
}

#[test]
fn pane_without_profile_loads_as_none() {
    let mut json = pane_json();
    json.as_object_mut().unwrap().remove("profile");
    let pane: Pane = serde_json::from_value(json).unwrap();
    assert!(pane.profile.is_none());

    // Stored back and loaded again it is still the same record with no profile.
    let again: Pane = serde_json::from_value(serde_json::to_value(&pane).unwrap()).unwrap();
    assert_eq!(again, pane);
    assert!(again.profile.is_none());
}

#[test]
fn pane_variants_round_trip() {
    // (pointer, replacement): every enum arm and every optional the full record leaves out.
    #[rustfmt::skip]
    let cases: Vec<(&str, Value)> = vec![
        ("/role", json!("orchestrator")),
        ("/hold", json!("none")),
        ("/hold", json!("drained")),
        ("/harness/health", json!("unknown")),
        ("/harness/health", json!({"unhealthy": "connection refused"})),
        ("/harness/pid", Value::Null),
        ("/session_of_record", Value::Null),
        ("/last_observed/shown", Value::Null),
        ("/last_observed/driven", Value::Null),
        ("/host/herdr_api_version", Value::Null),
        ("/command", Value::Null),
        ("/probe/check", Value::Null),
        ("/probe/last", Value::Null),
        ("/probe/last", json!("ok")),
        ("/probe/last", json!({"error": "timed out"})),
        ("/probe/expect", json!([])),
        ("/env", json!([])),
    ];

    for (pointer, replacement) in cases {
        let mut json = pane_json();
        *json
            .pointer_mut(pointer)
            .expect("pointer exists in the sample") = replacement.clone();
        let pane: Pane = serde_json::from_value(json)
            .unwrap_or_else(|e| panic!("{pointer} = {replacement}: {e}"));

        let stored = serde_json::to_value(&pane).unwrap();
        assert_eq!(at(&stored, pointer), replacement, "{pointer}");
        let again: Pane = serde_json::from_value(stored).unwrap();
        assert_eq!(again, pane, "{pointer} = {replacement}");
    }
}

#[test]
fn pane_refuses_malformed_fields() {
    #[rustfmt::skip]
    let cases: Vec<(&str, Value)> = vec![
        ("/name", json!("Not A Pane Name")),
        ("/name", json!("")),
        ("/herdr/grid", json!({"row": 1, "col": 1, "pos": "r2c2"})),
        ("/herdr/grid", json!({"row": 0, "col": 1, "pos": "r0c1"})),
        ("/generation", json!(-1)),
        ("/last_observed/at", json!("yesterday")),
        ("/env", json!(["NAME=value"])),
        ("/command", json!("opencode --port 8095")),
        ("/role", json!("janitor")),
        ("/profile", json!("")),
    ];
    for (pointer, bad) in cases {
        let mut json = pane_json();
        *json.pointer_mut(pointer).unwrap() = bad.clone();
        assert!(
            serde_json::from_value::<Pane>(json).is_err(),
            "{pointer} = {bad} must be refused"
        );
    }
}

#[test]
fn profile_round_trips() {
    let want = profile_json();
    let profile: Profile = serde_json::from_value(want.clone()).unwrap();

    assert_eq!(profile.name.as_str(), "Some Profile");
    assert_eq!(profile.slug, "some-profile");
    assert_eq!(profile.generation, 3);
    assert_eq!(profile.panes.len(), 2);

    assert_eq!(serde_json::to_value(&profile).unwrap(), want);
}

#[test]
fn profile_has_no_log_field_so_cas_put_cannot_rewrite_history() {
    // The append-only log is read only through `ProfileStore::log`.
    let stored = serde_json::to_value(profile()).unwrap();
    assert!(
        stored.get("log").is_none(),
        "Profile must not serialize a log: {stored}"
    );

    // And a `log` smuggled into a write does not become part of the record.
    let mut json = profile_json();
    json["log"] = json!([{"actor": "mallory", "change": "rewrote history"}]);
    if let Ok(loaded) = serde_json::from_value::<Profile>(json) {
        assert!(serde_json::to_value(&loaded).unwrap().get("log").is_none());
    }
}

#[test]
fn profile_spec_round_trips() {
    let want = spec_json();
    let spec: ProfileSpec = serde_json::from_value(want.clone()).unwrap();
    assert_eq!(spec.pane, "hj-c1r1");
    assert_eq!(serde_json::to_value(&spec).unwrap(), want);
}

#[test]
fn profile_spec_variants_round_trip() {
    #[rustfmt::skip]
    let cases: Vec<(&str, Value)> = vec![
        ("/role", json!("orchestrator")),
        ("/command", Value::Null),
        ("/check", Value::Null),
        ("/expect", json!([])),
        ("/env", json!([])),
    ];
    for (pointer, replacement) in cases {
        let mut json = spec_json();
        *json.pointer_mut(pointer).unwrap() = replacement.clone();
        let spec: ProfileSpec = serde_json::from_value(json)
            .unwrap_or_else(|e| panic!("{pointer} = {replacement}: {e}"));
        let stored = serde_json::to_value(&spec).unwrap();
        assert_eq!(at(&stored, pointer), replacement, "{pointer}");
        let again: ProfileSpec = serde_json::from_value(stored).unwrap();
        assert_eq!(again, spec, "{pointer}");
    }
}

#[test]
fn a_profile_spec_may_name_a_pane_that_does_not_exist() {
    // A detached spec (`profile create --from`): the pane name is plain data, so a
    // name that is not a valid *pane name* is still stored verbatim as a String.
    let mut json = spec_json();
    json["pane"] = json!("detached spec, not a pane");
    let spec: ProfileSpec = serde_json::from_value(json).unwrap();
    assert_eq!(spec.pane, "detached spec, not a pane");
}

#[test]
fn cas_put_with_a_stale_generation_is_a_generation_conflict() {
    let store = MemPaneStore::with(vec![pane()]); // stored at generation 7
    let mut edited = pane();
    edited.hold = serde_json::from_value(json!("drained")).unwrap();

    // The caller read generation 6; the store is already at 7.
    let err = store
        .cas_put(&edited, 6)
        .expect_err("a stale write must fail");
    assert_eq!(err.code(), "generation-conflict");
    assert!(!err.to_string().is_empty());
    assert_eq!(
        store.get(&edited.name).unwrap().unwrap(),
        pane(),
        "a refused write leaves the record unchanged"
    );

    // The same write naming the generation it read succeeds and bumps it.
    let stored = store.cas_put(&edited, 7).unwrap();
    assert_eq!(stored.generation, 8);
    assert_eq!(store.get(&edited.name).unwrap().unwrap(), stored);
}

#[test]
fn delete_with_a_stale_generation_is_a_generation_conflict_and_a_current_one_removes() {
    let store = MemPaneStore::with(vec![pane()]);
    let name = pane().name;

    let err = store.delete(&name, 6).expect_err("stale delete");
    assert_eq!(err.code(), "generation-conflict");
    assert!(store.get(&name).unwrap().is_some());

    store.delete(&name, 7).unwrap();
    assert!(store.get(&name).unwrap().is_none());
    assert!(store.list().unwrap().is_empty());
}
