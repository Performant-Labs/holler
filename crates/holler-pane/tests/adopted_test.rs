#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! Behaviors the implementation adopted from the architecture review (#637 rows 2, 4,
//! 6-9), which the RED suite left to F: unknown-field refusal, the slug check on read,
//! the shared helpers (`next_generation`, `decode_params`, `Argv::from_json`), the
//! params and event JSON, and the malformed-reply rule.

mod common;

use holler_pane::reply::{PaneCasPutParams, PaneGetParams, ProfileCasPutParams, WatchParams};
use holler_pane::{
    decode_params, next_generation, Argv, Cursor, EnvVarName, GridPos, Pane, PaneError, PaneEvent,
    PaneReply, Profile, ProfileEvent, ProfileName, ProfileSpec, WatchReply,
};
use serde_json::{json, Value};

#[test]
fn records_and_params_refuse_an_unknown_field() {
    // A peer that does not know a field would drop it on the next CAS write-back.
    let mut pane = common::pane_json();
    pane["surprise"] = json!(1);
    assert!(serde_json::from_value::<Pane>(pane).is_err());

    let mut pane = common::pane_json();
    pane["host"]["surprise"] = json!(1);
    assert!(
        serde_json::from_value::<Pane>(pane).is_err(),
        "nested record"
    );

    let mut spec = common::spec_json();
    spec["surprise"] = json!(1);
    assert!(serde_json::from_value::<ProfileSpec>(spec).is_err());

    let mut profile = common::profile_json();
    profile["log"] = json!([]);
    assert!(serde_json::from_value::<Profile>(profile).is_err());

    let params = json!({"name": "hj-c1r1", "extra": true});
    assert!(serde_json::from_value::<PaneGetParams>(params).is_err());
    assert!(serde_json::from_value::<WatchParams>(json!({"since": 1, "x": 1})).is_err());

    // The reply envelope is the exception: a client never writes it back.
    let reply: PaneReply = serde_json::from_value(json!({"ok": true, "data": 1, "x": 2})).unwrap();
    assert_eq!(reply.into_result().unwrap(), Some(json!(1)));
}

#[test]
fn a_profile_whose_stored_slug_disagrees_with_its_name_does_not_load() {
    let mut profile = common::profile_json();
    profile["slug"] = json!("other-slug");
    assert!(serde_json::from_value::<Profile>(profile).is_err());
}

#[test]
fn the_slug_is_ascii_only_and_a_non_ascii_letter_is_a_separator() {
    let slug = |s: &str| ProfileName::parse(s).unwrap().slug();
    assert_eq!(slug("Caf\u{e9} Ops 2"), "caf-ops-2");
    assert_eq!(slug("  --Night__Shift!!  "), "night-shift");
    // Nothing ASCII-alphanumeric to derive a slug from: refused with `usage`.
    let err = ProfileName::parse("\u{e9}\u{e8}").unwrap_err();
    assert_eq!(err.code(), "usage");
}

#[test]
fn next_generation_is_the_one_compare_and_swap_rule() {
    assert_eq!(
        next_generation(0, 0).unwrap(),
        1,
        "a create names 0 and lands at 1"
    );
    assert_eq!(next_generation(7, 7).unwrap(), 8);
    assert_eq!(
        next_generation(7, 6).unwrap_err().code(),
        "generation-conflict"
    );
    assert_eq!(
        next_generation(0, 1).unwrap_err().code(),
        "generation-conflict"
    );
    // A counter that cannot advance is a corrupt store, not a silent wrap to 0.
    assert_eq!(
        next_generation(u64::MAX, u64::MAX).unwrap_err().code(),
        "store-corrupt"
    );
}

#[test]
fn decode_params_keeps_a_guard_code_and_calls_everything_else_usage() {
    let mut spec = common::spec_json();
    spec["command"] = json!("ls -l");
    let params = json!({"profile": {
        "name": "P", "slug": "p", "generation": 1, "panes": [spec], "created": 1, "updated": 1
    }, "expected_generation": 0, "actor": "tester"});
    let err = decode_params::<ProfileCasPutParams>(params).unwrap_err();
    assert_eq!(err.code(), "command-not-argv");

    let mut pane = common::pane_json();
    pane["env"] = json!(["TOKEN=abc123"]);
    let err = decode_params::<PaneCasPutParams>(json!({"pane": pane, "expected_generation": 7}))
        .unwrap_err();
    assert_eq!(err.code(), "profile-secret-refused");
    assert!(!err.to_string().contains("abc123"), "{err}");

    // Missing field, wrong type, unknown field: all `usage`.
    for bad in [
        json!({}),
        json!({"name": 7}),
        json!({"name": "hj-c1r1", "x": 1}),
    ] {
        let err = decode_params::<PaneGetParams>(bad).unwrap_err();
        assert_eq!(err.code(), "usage");
    }
    let ok: PaneGetParams = decode_params(json!({"name": "hj-c1r1"})).unwrap();
    assert_eq!(ok.name.as_str(), "hj-c1r1");
}

#[test]
fn argv_from_json_separates_bad_json_from_a_non_array() {
    let argv = Argv::from_json(r#"["a","b c"]"#).unwrap();
    assert_eq!(argv.as_slice(), ["a".to_owned(), "b c".to_owned()]);
    assert_eq!(
        Argv::from_json(r#""ls -l""#).unwrap_err().code(),
        "command-not-argv"
    );
    assert_eq!(
        Argv::from_json(r#"["a", 1]"#).unwrap_err().code(),
        "command-not-argv"
    );
    assert_eq!(Argv::from_json("[\"a\"").unwrap_err().code(), "usage");
    assert_eq!(Argv::from_json("").unwrap_err().code(), "usage");
}

#[test]
fn watch_params_and_replies_use_the_documented_json() {
    // `since` defaults to 0: from the beginning.
    let p: WatchParams = serde_json::from_value(json!({})).unwrap();
    assert_eq!(p.since, Cursor(0));
    let p: WatchParams = serde_json::from_value(json!({"since": 41})).unwrap();
    assert_eq!(p.since, Cursor(41));

    let pane = common::pane();
    let put = PaneEvent {
        cursor: Cursor(3),
        name: pane.name.clone(),
        pane: Some(Box::new(pane.clone())),
    };
    let del = PaneEvent {
        cursor: Cursor(4),
        name: pane.name.clone(),
        pane: None,
    };
    let reply = WatchReply {
        events: vec![put.clone(), del.clone()],
        cursor: Cursor(4),
    };
    let json: Value = serde_json::to_value(&reply).unwrap();
    assert_eq!(json["cursor"], json!(4));
    assert_eq!(json["events"][0]["cursor"], json!(3));
    assert_eq!(json["events"][0]["pane"]["name"], json!("hj-c1r1"));
    assert!(json["events"][1]["pane"].is_null(), "null is a deletion");
    let back: WatchReply<PaneEvent> = serde_json::from_value(json).unwrap();
    assert_eq!(back, reply);

    let profile = common::profile();
    let event = ProfileEvent {
        cursor: Cursor(9),
        name: profile.name.clone(),
        profile: Some(Box::new(profile)),
    };
    let back: ProfileEvent = serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
    assert_eq!(back, event);
}

#[test]
fn a_reply_whose_ok_and_error_disagree_is_unavailable() {
    for bad in [
        json!({"ok": true, "error": {"code": "timeout", "message": "m"}}),
        json!({"ok": false}),
        json!({"ok": false, "data": 1}),
    ] {
        let reply: PaneReply = serde_json::from_value(bad.clone()).unwrap();
        let err = reply.into_result().unwrap_err();
        assert_eq!(err.code(), "unavailable", "{bad}");
    }
    // A well-formed success without data is `Ok(None)`.
    let reply: PaneReply = serde_json::from_value(json!({"ok": true})).unwrap();
    assert_eq!(reply.into_result().unwrap(), None);
    assert!(matches!(
        PaneReply::failure(&PaneError::Conflict).into_result(),
        Err(PaneError::Conflict)
    ));
}

#[test]
fn grid_accepts_an_absent_pos_and_leading_zeros_and_env_refuses_control_characters() {
    let g: GridPos = serde_json::from_value(json!({"row": 2, "col": 1})).unwrap();
    assert_eq!((g.row, g.col), (2, 1));
    assert_eq!(GridPos::parse("r02c001").unwrap(), g);

    for bad in ["A\u{7}B", "A\tB", "A\nB", " A", "A "] {
        assert_eq!(
            EnvVarName::parse(bad).unwrap_err().code(),
            "env-name-invalid",
            "{bad:?}"
        );
    }
}
