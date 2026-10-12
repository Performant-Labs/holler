#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! The finding vocabulary of reconcile and `holler pane doctor` (#647 AC 25): eighteen kinds
//! with stable kebab-case codes, each equal to the kind's JSON form. A code is stable once
//! merged (ADR-0021 section 9), so the list is pinned in order; #832 appended
//! `unmanaged-stalled`, #831 `pane-waiting` and `claude-login-missing`, #665
//! `profile-drift`, a kind that is also an error code of the closed list, classed a failure,
//! and #647 part 2 `unattached` and `agent-cannot-dispatch` (ADR-0021 section 12).
//!
//! The one remedy table (`remedy_for`) is pinned here for the two kinds of part 2: the agent
//! a remedy names is the one the caller passes, as a parsed `AgentKey`, in the joined form
//! `--agent=KEY` (a key may start with `-`); `remedy`, the signature callers already use,
//! answers as the table does with no agent.

use std::collections::BTreeSet;

use holler_pane::error::{class_of, is_valid_code, ErrorClass};
use holler_pane::findings::{FindingKind, FixState};
use holler_pane::{AgentKey, PaneName};

const CODES: [&str; 18] = [
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
    "unattached",
    "agent-cannot-dispatch",
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

const FIX_STATES: [FixState; 5] = [
    FixState::NotFixable,
    FixState::Fixable,
    FixState::Skipped,
    FixState::Fixed,
    FixState::Failed,
];

fn pane() -> PaneName {
    PaneName::parse("demo-c1r1").unwrap()
}

fn key(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

/// `agent-cannot-dispatch` with one dispatcher names the relaunch that changes the pane's
/// agent to it (ADR-0021 decision 16: a relaunch changes the agent only when told to).
#[test]
fn agent_cannot_dispatch_names_the_relaunch_with_the_one_key() {
    let orchestrator = key("orchestrator");
    assert_eq!(
        FindingKind::AgentCannotDispatch
            .remedy_for(Some(&pane()), FixState::NotFixable, Some(&orchestrator))
            .as_deref(),
        Some("holler pane relaunch demo-c1r1 --agent=orchestrator")
    );
}

/// `AgentKey::parse` accepts a leading `-`, so the key is joined to its flag with `=`: a
/// separate `-x` would read as a flag (brief D8). The parse through the real clap tree is
/// `holler-cli`'s `pane_verbs::doctor::agent`.
#[test]
fn a_key_with_a_leading_dash_is_joined_to_the_flag() {
    let dash = key("-x");
    assert_eq!(
        FindingKind::AgentCannotDispatch
            .remedy_for(Some(&pane()), FixState::NotFixable, Some(&dash))
            .as_deref(),
        Some("holler pane relaunch demo-c1r1 --agent=-x")
    );
}

/// With no single key (none, or several) the table names no command, and a finding about no
/// pane names none either.
#[test]
fn agent_cannot_dispatch_without_a_key_or_a_pane_has_no_remedy() {
    assert_eq!(
        FindingKind::AgentCannotDispatch.remedy_for(Some(&pane()), FixState::NotFixable, None),
        None
    );
    assert_eq!(
        FindingKind::AgentCannotDispatch.remedy(Some(&pane()), FixState::NotFixable),
        None
    );
    assert_eq!(
        FindingKind::AgentCannotDispatch.remedy_for(
            None,
            FixState::NotFixable,
            Some(&key("orchestrator"))
        ),
        None
    );
}

/// No Holler verb attaches a tmux client to a Herdr pane (brief D11), so `unattached` names
/// no command in any state, with or without a key.
#[test]
fn unattached_never_has_a_remedy() {
    let orchestrator = key("orchestrator");
    for fix in FIX_STATES {
        for agent in [None, Some(&orchestrator)] {
            for at in [None, Some(pane())] {
                assert_eq!(
                    FindingKind::Unattached.remedy_for(at.as_ref(), fix, agent),
                    None,
                    "{fix:?} {agent:?} {at:?}"
                );
            }
        }
    }
}

/// `remedy` is `remedy_for` with no agent, and the agent changes the remedy of no other kind:
/// the old signature answers every kind as before, from the one table.
#[test]
fn remedy_is_the_table_with_no_agent_and_the_agent_touches_only_its_kind() {
    let orchestrator = key("orchestrator");
    for &kind in FindingKind::ALL {
        for fix in FIX_STATES {
            for at in [None, Some(pane())] {
                let old = kind.remedy(at.as_ref(), fix);
                assert_eq!(
                    kind.remedy_for(at.as_ref(), fix, None),
                    old,
                    "{kind:?} {fix:?} {at:?}"
                );
                if kind != FindingKind::AgentCannotDispatch {
                    assert_eq!(
                        kind.remedy_for(at.as_ref(), fix, Some(&orchestrator)),
                        old,
                        "{kind:?} {fix:?} {at:?}: the agent is only for agent-cannot-dispatch"
                    );
                }
            }
        }
    }
}
