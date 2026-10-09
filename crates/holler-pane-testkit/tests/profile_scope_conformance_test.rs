#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #688
//! The fake profile scope passes its own conformance suite, the suite lists the cases
//! it documents, and a scope that breaks one rule fails the case that pins that rule
//! (the mutation check; #638, slice c part 2, #688).
//!
//! Each mutant is a wrapper around the fake that changes only the order of the I8
//! write sequence or adds one delegated refusal, so this file holds no copy of the
//! edit logic. The suite is the unit under test here: a suite that cannot tell a
//! broken scope from a good one proves nothing about the CLI's real scope later.

use std::sync::Arc;

use holler_pane::{
    Actor, PaneError, PaneName, Profile, ProfileName, ProfileScope, ResolvedScope, SpecEdit,
};
use holler_pane_testkit::conformance::profile_scope::{
    profile_scope_cases, run_profile_scope_conformance,
};
use holler_pane_testkit::pane_store::FakePaneStore;
use holler_pane_testkit::profile_scope::FakeProfileScope;
use holler_pane_testkit::profile_store::FakeProfileStore;

/// The 15 case ids of the brief's table, in order.
const DOCUMENTED_CASES: [&str; 15] = [
    "resolve-every-pane-of-the-profile",
    "resolve-named-member",
    "resolve-non-member-is-pane-not-in-profile",
    "resolve-missing-profile-is-profile-not-found",
    "edit-set-replaces-the-entry-and-bumps-once",
    "edit-set-adds-a-missing-entry",
    "edit-remove-drops-the-entry",
    "the-act-sees-the-edit",
    "failed-act-restores-the-specs",
    "first-write-conflict-is-generation-conflict",
    "restore-conflict-is-profile-conflict",
    "no-profile-runs-only-the-act",
    "missing-profile-is-profile-not-found-before-the-act",
    "pane-in-other-profile-before-any-write",
    "remove-of-a-detached-spec-is-not-refused",
];

fn actor() -> Actor {
    Actor::parse("scope-under-test").unwrap()
}

#[test]
fn the_fake_passes_the_profile_scope_conformance_suite() {
    let result = run_profile_scope_conformance(|profiles, panes| {
        FakeProfileScope::new(profiles, panes, actor())
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn the_suite_runs_the_documented_cases() {
    assert_eq!(profile_scope_cases(), DOCUMENTED_CASES.to_vec());
}

// --- mutation check: a broken scope fails, on the named case ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct scope: the wrapper itself must not be what fails the suite.
    Nothing,
    /// With a profile, the act runs first and the profile is written after it.
    WritesAfterAct,
    /// The profile is written first, but a failed act does not put it back.
    NoRestore,
    /// A `Remove` is refused with `pane-in-other-profile` when the pane is not in the
    /// profile, as if the membership check ran on every edit.
    MembershipOnRemove,
}

struct Mutant {
    inner: FakeProfileScope,
    broken: Break,
}

impl Mutant {
    fn new(broken: Break, profiles: Arc<FakeProfileStore>, panes: Arc<FakePaneStore>) -> Self {
        Self {
            inner: FakeProfileScope::new(profiles, panes, actor()),
            broken,
        }
    }
}

impl ProfileScope for Mutant {
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        self.inner.resolve(profile, pane)
    }

    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        let Some(named) = profile else {
            return self.inner.edit_spec(profile, pane, edit, act);
        };
        match self.broken {
            Break::WritesAfterAct => {
                act()?;
                self.inner.edit_spec(profile, pane, edit, &mut || Ok(()))
            }
            Break::NoRestore => {
                let written = self.inner.edit_spec(profile, pane, edit, &mut || Ok(()))?;
                act()?;
                Ok(written)
            }
            Break::MembershipOnRemove => {
                if matches!(edit, SpecEdit::Remove) {
                    if let Err(PaneError::PaneNotInProfile { what }) =
                        self.inner.resolve(named, Some(pane))
                    {
                        return Err(PaneError::PaneInOtherProfile { what });
                    }
                }
                self.inner.edit_spec(profile, pane, edit, act)
            }
            Break::Nothing => self.inner.edit_spec(profile, pane, edit, act),
        }
    }
}

/// The suite must reject `broken` and name `case` among its failures.
fn assert_suite_fails_on(broken: Break, case: &str) {
    let result =
        run_profile_scope_conformance(|profiles, panes| Mutant::new(broken, profiles, panes));
    let failures = result.expect_err("a broken scope must not pass the suite");
    assert!(
        failures.iter().any(|f| f.case == case),
        "expected case `{case}` among the failures of {broken:?}, got: {failures:?}"
    );
    assert!(
        failures.iter().all(|f| !f.detail.is_empty()),
        "every failure says why: {failures:?}"
    );
}

#[test]
fn the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone() {
    let result = run_profile_scope_conformance(|profiles, panes| {
        Mutant::new(Break::Nothing, profiles, panes)
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn a_scope_that_writes_the_profile_after_the_act_fails() {
    // The act no longer sees the edit, and a failed act moves the generation by 0.
    assert_suite_fails_on(Break::WritesAfterAct, "the-act-sees-the-edit");
    assert_suite_fails_on(Break::WritesAfterAct, "failed-act-restores-the-specs");
}

#[test]
fn a_scope_that_does_not_restore_on_a_failed_act_fails() {
    assert_suite_fails_on(Break::NoRestore, "failed-act-restores-the-specs");
}

#[test]
fn a_scope_that_checks_membership_on_a_remove_fails() {
    assert_suite_fails_on(
        Break::MembershipOnRemove,
        "remove-of-a-detached-spec-is-not-refused",
    );
}
