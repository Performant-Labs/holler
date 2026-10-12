#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682
//! The fake profile registry passes its own conformance suite, the suite lists the
//! cases it documents, and a registry that breaks one rule fails the case that pins
//! that rule (the mutation check; #638, slice c part 1, #682).
//!
//! Each mutant is a wrapper around the fake that breaks exactly one rule. The suite
//! is the unit under test here: a suite that cannot tell a broken store from a good
//! one proves nothing about the hub's profile registry later.

use holler_pane::{
    Actor, Cursor, PaneError, Profile, ProfileEvent, ProfileLogEntry, ProfileName, ProfileStore,
    Watch,
};
use holler_pane_testkit::conformance::profile_store::{
    profile_store_cases, run_profile_store_conformance,
};
use holler_pane_testkit::profile_store::FakeProfileStore;

/// The 28 case ids of the brief's table, in order (#665 replaced `rename-is-not-implemented`
/// with the six `rename-*` cases at the end).
const DOCUMENTED_CASES: [&str; 28] = [
    "get-missing-is-none",
    "list-empty",
    "create-at-zero-stored-at-one",
    "submitted-generation-ignored",
    "create-over-existing-conflicts",
    "same-slug-other-name-is-profile-exists",
    "update-bumps-by-one",
    "stale-generation-conflicts",
    "expected-ahead-conflicts",
    "list-holds-every-profile",
    "delete-at-current-generation",
    "delete-stale-conflicts",
    "delete-missing-is-profile-not-found",
    "recreate-after-delete-starts-at-one",
    "log-is-append-only-and-oldest-first",
    "log-of-never-created-is-profile-not-found",
    "env-is-names-only",
    "watch-from-zero-yields-current-state",
    "watch-follows-each-change",
    "watch-resumes-without-gap-or-repeat",
    "watch-idle-is-ok-none-and-stays-usable",
    "failed-write-changes-nothing",
    "rename-moves-the-record",
    "rename-logs-renamed-and-closes-the-old-log",
    "rename-onto-a-deleted-name-continues-its-log",
    "rename-of-one-slug-is-one-put",
    "rename-feed-is-deletion-then-put",
    "rename-refusals-change-nothing",
];

#[test]
fn the_fake_passes_the_profile_store_conformance_suite() {
    let result = run_profile_store_conformance(|| (FakeProfileStore::new(), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn the_suite_runs_the_documented_cases() {
    assert_eq!(profile_store_cases(), DOCUMENTED_CASES.to_vec());
}

// --- mutation check: a broken store fails, on the named case ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A correct store: the wrapper itself must not be what fails the suite.
    Nothing,
    /// `cas_put` writes at whatever the current generation is.
    NoCas,
    /// `cas_put` returns the generation the caller submitted.
    KeepsSubmittedGeneration,
    /// `delete` of a missing name with `expected != 0` is `generation-conflict`.
    GenerationBeforeExistence,
    /// `cas_put` over a stored slug with another name overwrites it.
    SameSlugOverwrites,
    /// `log` of a name `get` does not find is `profile-not-found`: a delete
    /// forgets the log.
    ForgetsLogOnDelete,
    /// `watch(since)` resumes one change early.
    RepeatsOnResume,
    /// `rename` is a create of the new name and a delete of the old (#665): a new record at
    /// generation 1 with a `created` entry, its `created` and log restarted.
    RenameAsCreateAndDelete,
    /// `rename` renames whatever the stored generation is (#665).
    RenameSkipsCas,
    /// `rename` onto a name another live profile holds deletes that profile first (#665).
    RenameOverwritesTaken,
}

struct Mutant {
    inner: FakeProfileStore,
    broken: Break,
}

impl Mutant {
    fn new(broken: Break) -> Self {
        Self {
            inner: FakeProfileStore::new(),
            broken,
        }
    }
}

impl ProfileStore for Mutant {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.inner.get(name)
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        self.inner.list()
    }

    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        match self.broken {
            Break::NoCas => {
                let current = self
                    .inner
                    .get(&profile.name)?
                    .map_or(0, |stored| stored.generation);
                self.inner.cas_put(profile, current, actor)
            }
            Break::KeepsSubmittedGeneration => {
                let stored = self.inner.cas_put(profile, expected_generation, actor)?;
                Ok(Profile {
                    generation: profile.generation,
                    ..stored
                })
            }
            Break::SameSlugOverwrites => match self.inner.get(&profile.name)? {
                // The fake files by slug, so `get` finds the stored record of another
                // display name with the same slug.
                Some(stored) if stored.name != profile.name => {
                    self.inner.concurrent_put(profile, actor)
                }
                _ => self.inner.cas_put(profile, expected_generation, actor),
            },
            _ => self.inner.cas_put(profile, expected_generation, actor),
        }
    }

    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError> {
        if self.broken == Break::GenerationBeforeExistence
            && expected_generation != 0
            && self.inner.get(name)?.is_none()
        {
            return Err(PaneError::Conflict);
        }
        self.inner.delete(name, expected_generation, actor)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        match self.broken {
            Break::RepeatsOnResume => self.inner.watch(Cursor(since.0.saturating_sub(1))),
            _ => self.inner.watch(since),
        }
    }

    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        if self.broken == Break::ForgetsLogOnDelete && self.inner.get(name)?.is_none() {
            return Err(PaneError::ProfileNotFound {
                what: name.to_string(),
            });
        }
        self.inner.log(name)
    }

    fn rename(
        &self,
        from: &ProfileName,
        to: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        match self.broken {
            Break::RenameAsCreateAndDelete => {
                let stored = self
                    .inner
                    .get(from)?
                    .ok_or_else(|| PaneError::ProfileNotFound {
                        what: from.to_string(),
                    })?;
                let created = self.inner.cas_put(
                    &Profile {
                        name: to.clone(),
                        slug: to.slug(),
                        ..stored
                    },
                    0,
                    actor,
                )?;
                self.inner.delete(from, expected_generation, actor)?;
                Ok(created)
            }
            Break::RenameSkipsCas => {
                let current = self.inner.get(from)?.map_or(0, |stored| stored.generation);
                self.inner.rename(from, to, current, actor)
            }
            Break::RenameOverwritesTaken => {
                if let Some(taken) = self.inner.get(to)? {
                    if taken.slug != from.slug() {
                        self.inner.concurrent_delete(to, actor)?;
                    }
                }
                self.inner.rename(from, to, expected_generation, actor)
            }
            _ => self.inner.rename(from, to, expected_generation, actor),
        }
    }
}

/// The suite must reject `broken` and name `case` among its failures.
fn assert_suite_fails_on(broken: Break, case: &str) {
    let result = run_profile_store_conformance(|| (Mutant::new(broken), ()));
    let failures = result.expect_err("a broken store must not pass the suite");
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
    let result = run_profile_store_conformance(|| (Mutant::new(Break::Nothing), ()));
    assert_eq!(result, Ok(()));
}

#[test]
fn a_store_that_skips_the_generation_check_fails() {
    assert_suite_fails_on(Break::NoCas, "stale-generation-conflicts");
}

#[test]
fn a_store_that_keeps_the_submitted_generation_fails() {
    assert_suite_fails_on(
        Break::KeepsSubmittedGeneration,
        "submitted-generation-ignored",
    );
}

#[test]
fn a_store_that_checks_generation_before_existence_fails() {
    assert_suite_fails_on(
        Break::GenerationBeforeExistence,
        "delete-missing-is-profile-not-found",
    );
}

#[test]
fn a_store_that_lets_a_same_slug_name_overwrite_fails() {
    assert_suite_fails_on(
        Break::SameSlugOverwrites,
        "same-slug-other-name-is-profile-exists",
    );
}

#[test]
fn a_store_that_forgets_the_log_on_delete_fails() {
    assert_suite_fails_on(
        Break::ForgetsLogOnDelete,
        "log-is-append-only-and-oldest-first",
    );
}

#[test]
fn a_store_that_repeats_on_resume_fails() {
    assert_suite_fails_on(
        Break::RepeatsOnResume,
        "watch-resumes-without-gap-or-repeat",
    );
}

#[test]
fn a_rename_that_is_a_create_and_a_delete_fails() {
    assert_suite_fails_on(Break::RenameAsCreateAndDelete, "rename-moves-the-record");
    assert_suite_fails_on(
        Break::RenameAsCreateAndDelete,
        "rename-logs-renamed-and-closes-the-old-log",
    );
}

#[test]
fn a_rename_that_skips_the_generation_check_fails() {
    assert_suite_fails_on(Break::RenameSkipsCas, "rename-refusals-change-nothing");
}

#[test]
fn a_rename_that_overwrites_a_taken_name_fails() {
    assert_suite_fails_on(
        Break::RenameOverwritesTaken,
        "rename-refusals-change-nothing",
    );
}
