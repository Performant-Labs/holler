#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! The finding vocabulary of reconcile and `holler pane doctor` (#647 AC 25): sixteen kinds
//! with stable kebab-case codes, each equal to the kind's JSON form. A code is stable once
//! merged (ADR-0021 section 9), so the list is pinned in order; #832 appended
//! `unmanaged-stalled`, #831 `pane-waiting` and `claude-login-missing`, and #665
//! `profile-drift`, a kind that is also an error code of the closed list, classed a failure.

use std::collections::BTreeSet;

use holler_pane::error::{class_of, is_valid_code, ErrorClass};
use holler_pane::findings::FindingKind;

const CODES: [&str; 16] = [
    "herdr-version-unsupported",
    "observe-failed",
    "unregistered-herdr-pane",
    "herdr-pane-missing",
    "tmux-session-missing",
    "server-wedged",
    "server-down",
    "no-session-of-record",
    "session-of-record-missing",
    "tui-foreign-session",
    "shown-driven-mismatch",
    "stray-session",
    "unmanaged-stalled",
    "pane-waiting",
    "claude-login-missing",
    "profile-drift",
];

#[test]
fn finding_kind_codes_are_stable() {
    let codes: Vec<&str> = FindingKind::ALL.iter().map(|k| k.code()).collect();
    assert_eq!(codes, CODES, "the kinds, in declaration order");

    let unique: BTreeSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len(), "a duplicate code: {codes:?}");

    for kind in FindingKind::ALL {
        assert!(
            is_valid_code(kind.code()),
            "{:?} is not kebab-case",
            kind.code()
        );
        let json = serde_json::to_value(kind).unwrap();
        assert_eq!(json, kind.code(), "{kind:?} serializes as its code");
    }
}

#[test]
fn profile_drift_is_still_a_failure_class_code() {
    assert_eq!(class_of("profile-drift"), ErrorClass::Failure);
    assert_eq!(class_of("profile-drift").exit_code(), 1);
}
