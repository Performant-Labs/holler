//! The real `ProfileScope` (epic #633): the helper every `--profile` verb uses to scope itself
//! to a profile and to edit a spec in one transaction with the live change.
//!
//! Empty in the CLI skeleton (story #670). Story #663 fills it; the trait is
//! `holler_pane::ProfileScope`, frozen by #637.
//!
//! **RED stub (#663, Phase 4).** The public items below have the exact shape of the brief's
//! Decisions 1 and 8 so the tests compile; their bodies are placeholders that F replaces.

use std::sync::Arc;

use holler_pane::{
    Actor, PaneError, PaneName, PaneStore, Profile, ProfileName, ProfileScope, ProfileStore,
    ResolvedScope, SpecEdit,
};

/// The reconcile step of a run without `--profile` (Decision 8). RED stub: empty.
pub const RECONCILE_STEP_UNSCOPED: &str = "";

/// The reconcile step for the profile `profile` (Decision 8). RED stub: empty.
pub fn reconcile_step(profile: &ProfileName) -> String {
    let _ = profile;
    String::new()
}

/// The `ProfileScope` over any `ProfileStore` and `PaneStore` (Decision 1).
pub struct StoreScope {
    profiles: Arc<dyn ProfileStore>,
    panes: Arc<dyn PaneStore>,
    actor: Actor,
}

impl StoreScope {
    /// A scope over `profiles` and `panes` that logs every profile write as `actor`.
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
        Self {
            profiles,
            panes,
            actor,
        }
    }
}

impl ProfileScope for StoreScope {
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        let _ = (&self.profiles, &self.panes, &self.actor, profile, pane);
        Err(PaneError::NotImplemented)
    }

    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        let _ = (profile, pane, edit, act);
        Err(PaneError::NotImplemented)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #663
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;

    use holler_pane::{
        Actor, Pane, PaneError, PaneName, Profile, ProfileName, ProfileScope, ProfileSpec,
        ProfileStore, SpecEdit,
    };
    use holler_pane_testkit::conformance::profile_scope::run_profile_scope_conformance;
    use holler_pane_testkit::fixture::{sample_pane, sample_profile, sample_spec};
    use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
    use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

    use super::{reconcile_step, StoreScope, RECONCILE_STEP_UNSCOPED};

    const ALPHA: &str = "Demo Alpha";
    const C1: &str = "demo-c1r1";
    const C2: &str = "demo-c2r1";
    const C4: &str = "demo-c4r1";
    const DOCTOR: &str = "holler pane doctor --profile 'Demo Alpha'";
    const SHOW: &str = "holler profile show 'Demo Alpha'";

    /// What `edit_spec` answered.
    type Edited = Result<Option<Profile>, PaneError>;

    /// The fixture of AC 2-7: Demo Alpha (c1, c2) at generation 1 by `conformance`, and
    /// the panes c1 and c2 whose records name Demo Alpha; the scope under test over them.
    struct Bench {
        profiles: Arc<FakeProfileStore>,
        panes: Arc<FakePaneStore>,
        scope: StoreScope,
    }

    fn actor() -> Actor {
        Actor::parse("conformance").unwrap()
    }

    fn alpha() -> ProfileName {
        ProfileName::parse(ALPHA).unwrap()
    }

    fn pane(name: &str) -> PaneName {
        PaneName::parse(name).unwrap()
    }

    fn member(name: &str) -> Pane {
        Pane {
            profile: Some(alpha()),
            ..sample_pane(name).unwrap()
        }
    }

    fn bench() -> Bench {
        let seeded = sample_profile(ALPHA, &[C1, C2]).unwrap();
        let profiles = Arc::new(FakeProfileStore::seeded([seeded], &actor()).unwrap());
        let panes = Arc::new(FakePaneStore::seeded([member(C1), member(C2)]).unwrap());
        let scope = StoreScope::new(profiles.clone(), panes.clone(), actor());
        Bench {
            profiles,
            panes,
            scope,
        }
    }

    /// `s'`: the sample spec of `name` with `context.soft` raised by one, as the suite's
    /// `changed_spec` builds it.
    fn changed_spec(name: &str) -> ProfileSpec {
        let mut spec = sample_spec(name);
        spec.context.soft = spec.context.soft.saturating_add(1);
        spec
    }

    fn set(spec: ProfileSpec) -> SpecEdit {
        SpecEdit::Set(Box::new(spec))
    }

    fn act_failed() -> PaneError {
        PaneError::Unavailable {
            what: "act".to_owned(),
        }
    }

    /// `edit_spec(Some(Demo Alpha), pane, edit)` with an act that counts its runs and
    /// answers `inside()`; returns the answer and the runs.
    fn edit_alpha(
        b: &Bench,
        name: &str,
        edit: &SpecEdit,
        mut inside: impl FnMut() -> Result<(), PaneError>,
    ) -> (Edited, usize) {
        let runs = Cell::new(0);
        let mut act = || {
            runs.set(runs.get() + 1);
            inside()
        };
        let edited = b
            .scope
            .edit_spec(Some(&alpha()), &pane(name), edit, &mut act);
        (edited, runs.get())
    }

    fn stored_alpha(b: &Bench) -> Profile {
        b.profiles
            .get(&alpha())
            .unwrap()
            .expect("Demo Alpha is stored")
    }

    fn code_of(edited: &Edited) -> &str {
        match edited {
            Err(e) => e.code(),
            Ok(ok) => panic!("expected an error, got Ok({ok:?})"),
        }
    }

    fn message_of(edited: &Edited) -> String {
        edited
            .as_ref()
            .err()
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    /// The message names Demo Alpha, c1, the act's error and the reconcile step, on one line.
    fn assert_names_the_edit_and_the_step(message: &str) {
        for part in [ALPHA, C1, "unavailable: act", DOCTOR, SHOW] {
            assert!(message.contains(part), "{part:?} missing from {message:?}");
        }
        assert!(
            !message.contains('\n'),
            "the message is not one line: {message:?}"
        );
    }

    // AC 1: all 15 cases of #638's suite hold against the real scope.
    #[test]
    fn store_scope_passes_the_profile_scope_conformance_suite() {
        let actor = actor();
        let result = run_profile_scope_conformance(|profiles, panes| {
            StoreScope::new(profiles, panes, actor.clone())
        });
        assert_eq!(result, Ok(()));
    }

    // AC 2 (Decision 5): a restore that fails without a conflict keeps its own code and
    // names the profile, the unrestored edit, the act's error and the reconcile step.
    #[test]
    fn restore_failure_keeps_its_code_and_names_the_unrestored_edit() {
        let faults = [
            PaneError::Timeout {
                op: "profile_store.cas_put".to_owned(),
            },
            PaneError::Unavailable {
                what: "profile store".to_owned(),
            },
            PaneError::StoreCorrupt {
                what: "profiles.json".to_owned(),
            },
        ];
        for fault in faults {
            let b = bench();
            let expected_code = fault.code().to_owned();
            let profiles = b.profiles.clone();
            let (edited, runs) = edit_alpha(&b, C1, &set(changed_spec(C1)), || {
                profiles
                    .faults()
                    .fail_next(ProfileStoreOp::CasPut, fault.clone());
                Err(act_failed())
            });
            assert_eq!(code_of(&edited), expected_code, "answer: {edited:?}");
            assert_names_the_edit_and_the_step(&message_of(&edited));
            assert_eq!(runs, 1, "the act ran once ({expected_code})");
            assert_eq!(
                b.profiles.faults().calls(),
                vec![
                    ProfileStoreOp::Get,
                    ProfileStoreOp::CasPut,
                    ProfileStoreOp::CasPut
                ],
                "one restore, not retried ({expected_code})"
            );
            let now = stored_alpha(&b);
            assert_eq!(now.generation, 2, "the edit stayed ({expected_code})");
            assert_eq!(now.panes[0], changed_spec(C1), "Demo Alpha holds s'");
        }
    }

    // AC 3 (Decision 7): the restore conflict is profile-conflict and prints the step.
    #[test]
    fn restore_conflict_names_the_act_error_and_the_reconcile_step() {
        let b = bench();
        let other = sample_profile(ALPHA, &[C4]).unwrap();
        let profiles = b.profiles.clone();
        let (edited, runs) = edit_alpha(&b, C1, &set(changed_spec(C1)), || {
            profiles.concurrent_put(&other, &actor()).unwrap();
            Err(act_failed())
        });
        assert_eq!(code_of(&edited), "profile-conflict", "answer: {edited:?}");
        assert_names_the_edit_and_the_step(&message_of(&edited));
        assert_eq!(runs, 1);
    }

    // AC 4 (Decision 6): a first write that times out may have landed; any other
    // first-write error passes through. The act never runs.
    #[test]
    fn first_write_timeout_says_the_edit_may_have_landed() {
        let b = bench();
        let timeout = PaneError::Timeout {
            op: "profile_store.cas_put".to_owned(),
        };
        b.profiles
            .faults()
            .fail_next(ProfileStoreOp::CasPut, timeout);
        let (edited, runs) = edit_alpha(&b, C1, &set(changed_spec(C1)), || Ok(()));
        assert_eq!(code_of(&edited), "timeout", "answer: {edited:?}");
        assert_eq!(runs, 0, "the act never ran after a timed-out first write");
        let message = message_of(&edited);
        for part in ["may hold the edit", DOCTOR] {
            assert!(message.contains(part), "{part:?} missing from {message:?}");
        }

        let b = bench();
        let down = PaneError::Unavailable {
            what: "profile store".to_owned(),
        };
        b.profiles
            .faults()
            .fail_next(ProfileStoreOp::CasPut, down.clone());
        let (edited, runs) = edit_alpha(&b, C1, &set(changed_spec(C1)), || Ok(()));
        assert_eq!(edited, Err(down), "passed through unchanged");
        assert_eq!(runs, 0);
    }

    // AC 5 (Decision 8): the step quotes the profile name POSIX-style; the unscoped
    // form is #644's exact text.
    #[test]
    fn reconcile_step_single_quotes_the_profile_name() {
        assert_eq!(
            reconcile_step(&alpha()),
            "to reconcile, run holler pane doctor --profile 'Demo Alpha' \
             and then holler profile show 'Demo Alpha'"
        );
        let tricky = reconcile_step(&ProfileName::parse("It's $(id) Demo").unwrap());
        assert!(
            tricky.contains("--profile 'It'\\''s $(id) Demo'"),
            "not single-quoted: {tricky:?}"
        );
        assert!(!tricky.contains('\n'), "not one line: {tricky:?}");
        assert_eq!(
            RECONCILE_STEP_UNSCOPED,
            "to reconcile, run holler pane doctor"
        );
    }

    // AC 6 (Decision 9): a Set whose spec names another pane is usage before any write.
    #[test]
    fn set_of_a_spec_for_another_pane_is_usage_before_any_write() {
        let b = bench();
        let (edited, runs) = edit_alpha(&b, C1, &set(sample_spec(C2)), || Ok(()));
        assert_eq!(code_of(&edited), "usage", "answer: {edited:?}");
        assert_eq!(runs, 0);
        assert!(
            !b.profiles
                .faults()
                .calls()
                .contains(&ProfileStoreOp::CasPut),
            "the profile was written: {:?}",
            b.profiles.faults().calls()
        );
    }

    // AC 7 (Decision 10): the pane record is read for every edit before the first write,
    // so a pane store that cannot be read fails a Remove before the profile moves.
    #[test]
    fn pane_store_fault_fails_a_remove_before_the_profile_write() {
        let b = bench();
        let down = PaneError::Unavailable {
            what: "pane store".to_owned(),
        };
        b.panes.faults().fail_next(PaneStoreOp::Get, down.clone());
        let (edited, runs) = edit_alpha(&b, C2, &SpecEdit::Remove, || Ok(()));
        assert_eq!(edited, Err(down));
        assert_eq!(runs, 0);
        assert!(
            !b.profiles
                .faults()
                .calls()
                .contains(&ProfileStoreOp::CasPut),
            "the profile was written: {:?}",
            b.profiles.faults().calls()
        );
        assert_eq!(stored_alpha(&b).generation, 1, "Demo Alpha is unchanged");
    }
}
