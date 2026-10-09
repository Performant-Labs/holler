//! The exit class of an error code (#676, ADR-0021 section 9): which closed code is a
//! refusal (exit 3), which a runtime failure (exit 1), and `usage` (exit 2). One
//! function, `class_of`, decides; the CLI's output module and the test kit both call it.

use holler_pane::error::{class_of, ErrorClass, RefusalCode, ALL_CODES};
use holler_pane::PaneError;

use ErrorClass::{Failure, Refusal, Usage};

/// The decided class of every closed code: the brief's table, written out in full so
/// a change of class is a visible edit here and not only in `class_of`.
const DECIDED: [(&str, ErrorClass); 22] = [
    ("usage", Usage),
    ("grid-ambiguous", Refusal),
    ("grid-out-of-range", Refusal),
    ("command-not-argv", Refusal),
    ("env-name-invalid", Refusal),
    ("profile-secret-refused", Refusal),
    ("profile-exists", Refusal),
    ("profile-has-live-panes", Refusal),
    ("pane-not-in-profile", Refusal),
    ("pane-in-other-profile", Refusal),
    ("probe-failed", Refusal),
    ("herdr-version-unsupported", Refusal),
    ("profile-not-found", Refusal),
    ("pane-not-found", Refusal),
    ("session-not-found", Refusal),
    ("generation-conflict", Failure),
    ("profile-conflict", Failure),
    ("timeout", Failure),
    ("unavailable", Failure),
    ("store-corrupt", Failure),
    ("not-implemented", Failure),
    ("profile-drift", Failure),
];

/// A code added to `ALL_CODES` fails here until this test names its class.
#[test]
fn every_closed_code_has_the_decided_class() {
    let mut decided: Vec<&str> = DECIDED.iter().map(|(code, _)| *code).collect();
    let mut closed: Vec<&str> = ALL_CODES.to_vec();
    decided.sort_unstable();
    closed.sort_unstable();
    assert_eq!(
        decided, closed,
        "the table names every closed code, once each"
    );

    for (code, class) in DECIDED {
        assert_eq!(class_of(code), class, "the class of `{code}`");
    }
}

/// An open code is one a verb or adapter owns and raises as `Refused`: a refusal.
#[test]
fn an_open_code_is_a_refusal() {
    assert_eq!(class_of("quota-exceeded"), Refusal);

    const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");
    let refused = PaneError::Refused {
        code: QUOTA,
        message: "over quota".to_string(),
    };
    assert_eq!(class_of(refused.code()), Refusal);
}

/// A garbled code cannot be trusted (`from_wire` turns it into `unavailable`): a failure.
#[test]
fn a_malformed_code_is_a_failure() {
    assert_eq!(class_of("Not A Code"), Failure);
    assert_eq!(class_of(""), Failure);
}

/// ADR 0003's exit numbers, one per class.
#[test]
fn exit_codes_by_class() {
    assert_eq!(Usage.exit_code(), 2);
    assert_eq!(Refusal.exit_code(), 3);
    assert_eq!(Failure.exit_code(), 1);
}
