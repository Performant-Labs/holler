#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! The error taxonomy and the reply type (#637 AC 5): one closed set of kebab-case
//! codes, one validator, an open `Refused` for codes a verb or adapter owns, and
//! `PaneReply` as the wire form of a `PaneError` (round-tripped, never lossy for a
//! closed code).
//!
//! A `compile_fail` doctest on `PaneError::Refused` (the unvalidated variant cannot be
//! built outside the crate) completes AC 5; it lives with the type, see handoff-T-red.

use std::collections::BTreeSet;

use holler_pane::error::{is_valid_code, RefusalCode, ALL_CODES};
use holler_pane::{PaneError, PaneReply};
use serde_json::{json, Value};

/// The closed code set: the epic's list, no more and no fewer (a story fills
/// behaviour, never the list). `profile-drift` is a reconcile finding kind that is
/// listed for convenience.
const CLOSED: [&str; 22] = [
    "not-implemented",
    "usage",
    "grid-ambiguous",
    "grid-out-of-range",
    "command-not-argv",
    "env-name-invalid",
    "generation-conflict",
    "probe-failed",
    "profile-conflict",
    "profile-not-found",
    "profile-exists",
    "profile-has-live-panes",
    "pane-not-in-profile",
    "pane-in-other-profile",
    "profile-secret-refused",
    "herdr-version-unsupported",
    "timeout",
    "pane-not-found",
    "session-not-found",
    "store-corrupt",
    "unavailable",
    "profile-drift",
];

/// A code a verb owns, as a verb would declare it: a `const`, so no `Result` to handle.
const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");
/// `is_valid_code` is a `const fn`: a verb's code constant can be checked at build time.
const _: () = assert!(is_valid_code("a-b"));

/// Build the error a client gets for a reply carrying `code` (the parse-back path).
fn from_wire(code: &str, message: &str) -> PaneError {
    let reply: PaneReply = serde_json::from_value(json!({
        "ok": false,
        "data": null,
        "error": {"code": code, "message": message}
    }))
    .unwrap();
    reply.into_result().expect_err("an error reply is an Err")
}

/// Send `err` over the wire and read it back, through real JSON text.
fn over_the_wire(err: &PaneError) -> PaneError {
    let text = serde_json::to_string(&PaneReply::failure(err)).unwrap();
    let reply: PaneReply = serde_json::from_str(&text).unwrap();
    reply.into_result().expect_err("an error reply is an Err")
}

#[test]
fn error_codes_unique_and_kebab() {
    let all: Vec<&str> = ALL_CODES.to_vec();
    let unique: BTreeSet<&str> = all.iter().copied().collect();
    assert_eq!(
        unique.len(),
        all.len(),
        "ALL_CODES has a duplicate: {all:?}"
    );

    for code in &all {
        assert!(is_valid_code(code), "{code:?} is not kebab-case");
    }

    let closed: BTreeSet<&str> = CLOSED.iter().copied().collect();
    assert_eq!(
        unique, closed,
        "ALL_CODES must be exactly the epic's closed set"
    );
}

#[test]
fn is_valid_code_is_the_kebab_case_grammar() {
    for ok in ["a", "usage", "grid-ambiguous", "a-b-c", "quota-exceeded"] {
        assert!(is_valid_code(ok), "{ok:?}");
    }
    for bad in [
        "",
        "-",
        "-a",
        "a-",
        "a--b",
        "A",
        "a-B",
        "a_b",
        "a1",
        "1",
        "a b",
        "a.b",
        "a/b",
        "é",
        "a\n",
        "Not A Code",
        " a",
    ] {
        assert!(!is_valid_code(bad), "{bad:?}");
    }
}

#[test]
fn every_closed_code_has_a_variant_whose_code_and_display_are_set() {
    for code in CLOSED {
        let err = from_wire(code, "something specific happened");
        assert_eq!(err.code(), code, "parse-back of {code}");
        assert!(
            !matches!(err, PaneError::Refused { .. }),
            "{code} is closed and must map to its own variant, not Refused"
        );
        assert!(
            !err.to_string().trim().is_empty(),
            "Display of {code} is empty"
        );
    }
}

#[test]
fn the_two_not_found_codes_stay_distinct() {
    let pane = from_wire("pane-not-found", "no such pane");
    let session = from_wire("session-not-found", "no such session");
    assert_eq!(pane.code(), "pane-not-found");
    assert_eq!(session.code(), "session-not-found");
    assert_eq!(over_the_wire(&pane).code(), "pane-not-found");
    assert_eq!(over_the_wire(&session).code(), "session-not-found");
}

#[test]
fn pane_error_is_a_std_error() {
    fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
    assert_error::<PaneError>();
}

#[test]
fn directly_built_variants_report_their_closed_code() {
    assert_eq!(PaneError::NotImplemented.code(), "not-implemented");
    assert_eq!(PaneError::Conflict.code(), "generation-conflict");
    let timeout = PaneError::Timeout {
        op: "ensure_pane".to_string(),
    };
    assert_eq!(timeout.code(), "timeout");
    assert!(timeout.to_string().contains("ensure_pane"), "{timeout}");
    let down = PaneError::Unavailable {
        what: "herdr socket".to_string(),
    };
    assert_eq!(down.code(), "unavailable");
    assert!(down.to_string().contains("herdr socket"), "{down}");
}

#[test]
fn refusal_code_validates_what_it_holds() {
    assert_eq!(QUOTA.as_str(), "quota-exceeded");

    // A wire code that is not kebab-case is not a code.
    assert!(RefusalCode::parse("Not A Code".to_string()).is_err());
    assert!(RefusalCode::parse(String::new()).is_err());
    assert!(RefusalCode::parse("quota_exceeded".to_string()).is_err());
    // A closed code has one representation: its variant, never Refused.
    for code in CLOSED {
        assert!(
            RefusalCode::parse(code.to_string()).is_err(),
            "{code} is closed; a Refused must not carry it"
        );
    }
    // Anything else well-formed is fine.
    assert_eq!(
        RefusalCode::parse("adapter-quota".to_string())
            .unwrap()
            .as_str(),
        "adapter-quota"
    );
}

#[test]
fn a_refused_error_reports_its_own_code_and_survives_the_wire() {
    let refused = PaneError::Refused {
        code: QUOTA,
        message: "the provider quota is spent".to_string(),
    };
    assert_eq!(refused.code(), "quota-exceeded");
    assert!(
        refused.to_string().contains("the provider quota is spent"),
        "{refused}"
    );
    assert!(
        !ALL_CODES.contains(&refused.code()),
        "an open code is not in ALL_CODES"
    );

    let back = over_the_wire(&refused);
    assert_eq!(back.code(), "quota-exceeded");
    assert_eq!(format!("{back:?}"), format!("{refused:?}"));
}

#[test]
fn an_unknown_code_parses_to_refused_and_an_invalid_one_to_unavailable() {
    let err = from_wire("adapter-quota", "slow down");
    match &err {
        PaneError::Refused { code, message } => {
            assert_eq!(code.as_str(), "adapter-quota");
            assert_eq!(message, "slow down");
        }
        other => panic!("an unknown well-formed code must be Refused, got {other:?}"),
    }
    assert_eq!(err.code(), "adapter-quota");

    // A code off the wire that is not kebab-case cannot become a RefusalCode.
    for bad in ["Not A Code", "", "snake_case_code", "UPPER"] {
        let err = from_wire(bad, "garbled");
        assert_eq!(err.code(), "unavailable", "{bad:?}");
        assert!(!matches!(err, PaneError::Refused { .. }), "{bad:?}");
    }
}

#[test]
fn a_pane_error_round_trips_through_a_pane_reply_without_losing_fields() {
    let errors = [
        PaneError::NotImplemented,
        PaneError::Conflict,
        PaneError::Timeout {
            op: "send_keys".to_string(),
        },
        PaneError::Unavailable {
            what: "hub".to_string(),
        },
        PaneError::Refused {
            code: QUOTA,
            message: "m".to_string(),
        },
        from_wire("pane-not-found", "hj-c9r9"),
        from_wire("session-not-found", "ses_gone"),
        from_wire("profile-not-found", "Some Profile"),
    ];
    for err in errors {
        let back = over_the_wire(&err);
        assert_eq!(back.code(), err.code());
        assert_eq!(back.to_string(), err.to_string());
        assert_eq!(
            format!("{back:?}"),
            format!("{err:?}"),
            "structured fields must survive"
        );
    }
}

#[test]
fn a_failure_reply_is_ok_false_with_a_code_and_a_message() {
    let err = PaneError::Timeout {
        op: "snapshot".to_string(),
    };
    let json: Value = serde_json::to_value(PaneReply::failure(&err)).unwrap();
    assert_eq!(json["ok"], json!(false));
    assert!(json["data"].is_null());
    assert_eq!(json["error"]["code"], json!("timeout"));
    assert_eq!(json["error"]["message"], json!(err.to_string()));
    // Not the CLI envelope: no schema_version.
    assert!(json.get("schema_version").is_none());
}

#[test]
fn a_success_reply_carries_data_and_no_error() {
    let reply = PaneReply::success(json!({"panes": [], "cursor": 4}));
    let json: Value = serde_json::to_value(&reply).unwrap();
    assert_eq!(json["ok"], json!(true));
    assert_eq!(json["data"], json!({"panes": [], "cursor": 4}));
    assert!(json["error"].is_null());

    let back: PaneReply = serde_json::from_value(json).unwrap();
    let data = back.into_result().unwrap();
    assert_eq!(data, Some(json!({"panes": [], "cursor": 4})));
}
