#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! The behaviors added by the REWORK pass (S's REWORK 2, the operator's D1 and D2, and
//! A-dup rows 2 and 3): an `env` value never travels back in an error, `Watch` tells
//! idle from failed, `HarnessKind` stays inside the vocabulary, and a delete of a
//! missing record is `not-found` whatever generation it names.

mod common;

use holler_pane::pane::HarnessKind;
use holler_pane::reply::{PaneCasPutParams, ProfileCasPutParams, WatchParams};
use holler_pane::{
    decode_params, Cursor, Pane, PaneError, PaneEvent, PaneReply, PaneStore, ProfileSpec, Watch,
};
use serde_json::{json, Value};

const SECRET: &str = "hunter2";

/// What a peer would see on the wire for `err`, secret or not.
fn wire(err: &PaneError) -> String {
    serde_json::to_string(&PaneReply::failure(err)).unwrap()
}

fn assert_refused_without_echo(err: &PaneError, code: &str, shape: &Value) {
    assert_eq!(err.code(), code, "{shape}");
    for text in [err.to_string(), format!("{err:?}"), wire(err)] {
        assert!(!text.contains(SECRET), "{shape}: {text}");
    }
    // The reply parses back to the same code (no `detail` smuggling the value either).
    let back = serde_json::from_str::<PaneReply>(&wire(err))
        .unwrap()
        .into_result()
        .unwrap_err();
    assert_eq!(back.code(), code, "{shape}");
}

/// The `env` shapes a caller can get wrong, with the code each must give.
fn bad_envs() -> Vec<(Value, &'static str)> {
    vec![
        (json!("TOKEN=hunter2"), "profile-secret-refused"),
        (json!("=hunter2"), "profile-secret-refused"),
        (json!(["A", "B=hunter2"]), "profile-secret-refused"),
        (json!("TOKEN"), "env-name-invalid"),
        (json!(""), "env-name-invalid"),
        (json!(["A", 5]), "env-name-invalid"),
        (json!([{"k": "hunter2"}]), "env-name-invalid"),
        (json!({"hunter2": 1}), "env-name-invalid"),
        (json!(12345), "env-name-invalid"),
        (json!(null), "env-name-invalid"),
        (json!(["A b"]), "env-name-invalid"),
    ]
}

#[test]
fn a_bad_env_in_a_spec_or_a_pane_is_refused_by_code_and_never_echoed() {
    for (env, code) in bad_envs() {
        let mut spec = common::spec_json();
        spec["env"] = env.clone();
        let err = serde_json::from_value::<ProfileSpec>(spec.clone()).unwrap_err();
        // The serde error text is what a `usage` message would be built from.
        assert!(!err.to_string().contains(SECRET), "{env}: {err}");

        let mut pane = common::pane_json();
        pane["env"] = env.clone();
        let err = serde_json::from_value::<Pane>(pane.clone()).unwrap_err();
        assert!(!err.to_string().contains(SECRET), "{env}: {err}");

        // Through the hub's decoder the guard's code survives.
        let mut one_spec = spec;
        one_spec["pane"] = json!("hj-c1r1");
        let params = json!({
            "profile": {"name": "P", "slug": "p", "generation": 1, "panes": [one_spec],
                        "created": 1, "updated": 1},
            "expected_generation": 0,
            "actor": "tester"
        });
        let err = decode_params::<ProfileCasPutParams>(params).unwrap_err();
        assert_refused_without_echo(&err, code, &env);

        let params = json!({"pane": pane, "expected_generation": 7});
        let err = decode_params::<PaneCasPutParams>(params).unwrap_err();
        assert_refused_without_echo(&err, code, &env);
    }
}

#[test]
fn a_good_env_is_still_a_list_of_names_and_an_absent_env_is_empty() {
    for env in [json!([]), json!(["OK", "TWO"])] {
        let mut pane = common::pane_json();
        pane["env"] = env.clone();
        let pane: Pane = serde_json::from_value(pane).unwrap();
        assert_eq!(serde_json::to_value(&pane.env).unwrap(), env);
    }
    let mut spec = common::spec_json();
    spec.as_object_mut().unwrap().remove("env");
    let spec: ProfileSpec = serde_json::from_value(spec).unwrap();
    assert!(spec.env.is_empty());
}

// ---------------------------------------------------------------------------
// D1: `Watch` says idle with `Ok(None)` and failure with `Err`
// ---------------------------------------------------------------------------

fn event(cursor: u64) -> PaneEvent {
    PaneEvent {
        cursor: Cursor(cursor),
        name: common::pane_name("hj-c1r1"),
        pane: None,
    }
}

/// How a long-poll consumer folds a watch: idle is skipped and the stream goes on, a
/// real change is collected, an error ends it and is returned.
fn drain(watch: Watch<PaneEvent>) -> (Vec<u64>, usize, Option<PaneError>) {
    let (mut seen, mut idle) = (Vec::new(), 0);
    for item in watch {
        match item {
            Ok(Some(e)) => seen.push(e.cursor.0),
            Ok(None) => idle += 1,
            Err(e) => return (seen, idle, Some(e)),
        }
    }
    (seen, idle, None)
}

#[test]
fn an_idle_watch_item_is_not_an_error_and_the_stream_stays_usable() {
    let items: Vec<Result<Option<PaneEvent>, PaneError>> =
        vec![Ok(None), Ok(Some(event(3))), Ok(None), Ok(Some(event(4)))];
    let (seen, idle, err) = drain(Box::new(items.into_iter()));
    assert_eq!(seen, [3, 4], "changes after an idle window still arrive");
    assert_eq!(idle, 2);
    assert!(err.is_none(), "idle is not a failure");
}

#[test]
fn a_timeout_from_the_store_is_a_failure_that_ends_the_stream() {
    let items: Vec<Result<Option<PaneEvent>, PaneError>> = vec![
        Ok(Some(event(1))),
        Err(PaneError::Timeout {
            op: "watch".to_owned(),
        }),
    ];
    let (seen, idle, err) = drain(Box::new(items.into_iter()));
    assert_eq!(seen, [1]);
    assert_eq!(idle, 0, "a wedged store is never reported as idle");
    assert_eq!(err.unwrap().code(), "timeout");
}

#[test]
fn watch_params_decode_from_an_empty_object_as_from_the_beginning() {
    // `typed_params`'s rule: a request without `params` is decoded from `{}`.
    let p: WatchParams = decode_params(json!({})).unwrap();
    assert_eq!(p.since, Cursor(0));
    let p: WatchParams = decode_params(json!({"since": 12})).unwrap();
    assert_eq!(p.since, Cursor(12));
}

// ---------------------------------------------------------------------------
// D2: a harness kind a record can hold is a harness the vocabulary knows
// ---------------------------------------------------------------------------

/// Every `HarnessKind`. The exhaustive match is the point: a new variant stops this
/// test compiling until it is listed here, and then the vocabulary check applies.
fn every_kind() -> Vec<HarnessKind> {
    match HarnessKind::Opencode {
        HarnessKind::Opencode => vec![HarnessKind::Opencode],
    }
}

#[test]
fn every_harness_kind_serde_name_is_in_the_protocol_vocabulary() {
    for kind in every_kind() {
        let name = serde_json::to_value(kind).unwrap();
        let name = name.as_str().expect("a harness kind is a plain string");
        assert!(
            holler_proto::HARNESS_IDS.contains(&name),
            "{name:?} is not in holler_proto::HARNESS_IDS: {:?}",
            holler_proto::HARNESS_IDS
        );
    }
    // And an id the vocabulary lacks does not load.
    let mut pane = common::pane_json();
    pane["harness"]["kind"] = json!("not-a-harness");
    assert!(serde_json::from_value::<Pane>(pane).is_err());
}

// ---------------------------------------------------------------------------
// A-dup row 2: a missing record is not-found, whatever generation is named
// ---------------------------------------------------------------------------

#[test]
fn deleting_a_missing_pane_is_pane_not_found_at_any_generation() {
    let store = common::MemPaneStore::with(vec![common::pane()]);
    let missing = common::pane_name("hj-c9r9");
    for generation in [0, 1, 7, u64::MAX] {
        let err = store.delete(&missing, generation).unwrap_err();
        assert_eq!(err.code(), "pane-not-found", "generation {generation}");
    }
    // A present record keeps the two outcomes apart.
    let name = common::pane_name("hj-c1r1");
    assert_eq!(
        store.delete(&name, 6).unwrap_err().code(),
        "generation-conflict"
    );
    store.delete(&name, 7).unwrap();
    assert_eq!(
        store.delete(&name, 7).unwrap_err().code(),
        "pane-not-found",
        "the record is gone, so the same call is now not-found, not a conflict"
    );
}
