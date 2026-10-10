//! The real `ProfileScope` (epic #633, #663): the helper every `--profile` verb uses to scope
//! itself to a profile and to edit a pane's spec in one transaction with the live change (I3,
//! I8). [`StoreScope`] implements the frozen `holler_pane::ProfileScope` over any `ProfileStore`
//! and `PaneStore` (#649 wires it in). It re-implements the test kit's `FakeProfileScope` on
//! purpose, since production code cannot depend on the test kit, and passes the same suite.
//!
//! **`edit_spec(Some(P), pane, edit, act)`** keeps ADR-0021 section 8's write order
//! (`edit_spec(None, ..)` runs only the act and calls neither store):
//!
//! 1. *Plan.* Read P at generation g (`profile-not-found` first) and the pane's record, for every
//!    edit, so a pane store that cannot be read fails before P moves. A `Set` whose spec names
//!    another pane is `usage`; a `Set` for a pane in another profile is `pane-in-other-profile`.
//!    A `Remove` is never refused for membership: a detached spec stays removable.
//! 2. *Write P first*, by one compare-and-swap at g: a conflict is `generation-conflict` and the
//!    act never runs. A `timeout` may have landed, so its message says so and carries the
//!    reconcile step; any other error passes through as it is.
//! 3. *Act*, once. The scope writes no pane record: recording the pane is the verb's, inside its
//!    act, so a pane-record conflict is an act failure like any other.
//! 4. *If the act fails*, put P's specs back by one compare-and-swap at g + 1, not retried, and
//!    answer the act's own error (P's generation has then moved by two). A conflict there is
//!    `profile-conflict`; any other error keeps its own code. Both messages name P, the pane, the
//!    act's error and the reconcile step, on one line (the fake's three messages are shorter).
//!
//! **Bound (a narrowing of the trait's I5 bound).** No timer and no thread: the scope cannot
//! cancel a blocking port call, and the act is the verb's, so `edit_spec` returns within the sum
//! of its port calls (at most four, each within I5's bound or `timeout`) plus the act's own time.
//! It retries nothing and catches no panic: a panicking act leaves the first write in place.

use std::sync::Arc;

use holler_pane::{
    Actor, Pane, PaneError, PaneName, PaneStore, Profile, ProfileName, ProfileScope, ProfileSpec,
    ProfileStore, ResolvedScope, SpecEdit,
};

/// The reconcile step of a run without `--profile`: the bare pane doctor command line, which (like
/// [`reconcile_step`]) names no pane. The spec-editing verbs (#644, #646) print this one.
pub const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor";

/// The reconcile step for `profile`, one line for an operator to paste into a shell, with the
/// name POSIX-single-quoted (it may hold spaces, quotes or `$(...)`, but no control character):
/// `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`.
/// The scope's errors carry it; a spec-editing verb prints it for a pane-record conflict.
pub fn reconcile_step(profile: &ProfileName) -> String {
    let name = single_quoted(profile.as_str());
    format!("{RECONCILE_STEP_UNSCOPED} --profile {name} and then holler profile show {name}")
}

/// The real [`ProfileScope`] over any [`ProfileStore`] and [`PaneStore`]; building it does no I/O.
pub struct StoreScope {
    profiles: Arc<dyn ProfileStore>,
    panes: Arc<dyn PaneStore>,
    /// Who every profile write of this scope is logged as.
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

    /// The profile `profile` as stored: `profile-not-found` when there is none.
    fn stored(&self, profile: &ProfileName) -> Result<Profile, PaneError> {
        let stored = self.profiles.get(profile)?;
        stored.ok_or_else(|| PaneError::ProfileNotFound {
            what: profile.to_string(),
        })
    }

    /// Every pane whose record names `profile` (by slug), in name order.
    fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
        let mut members = self.panes.list()?;
        members.retain(|pane| belongs(pane, profile));
        members.sort_by(|x, y| x.name.cmp(&y.name));
        Ok(members)
    }

    /// The pane `name`, whose record must name `profile`: else `pane-not-in-profile`.
    fn member(&self, profile: &ProfileName, name: &PaneName) -> Result<Pane, PaneError> {
        match self.panes.get(name)? {
            Some(pane) if belongs(&pane, profile) => Ok(pane),
            _ => Err(PaneError::PaneNotInProfile {
                what: format!("{name} is not in profile {:?}", profile.as_str()),
            }),
        }
    }

    /// Step 1: `stored` with `edit` made, once the guards pass; the pane record is read for every
    /// edit, and only a `Set` is checked for membership.
    fn plan(
        &self,
        stored: &Profile,
        pane: &PaneName,
        edit: &SpecEdit,
    ) -> Result<Profile, PaneError> {
        if let SpecEdit::Set(spec) = edit {
            check_filed_under(spec, pane)?;
        }
        let record = self.panes.get(pane)?;
        if let (SpecEdit::Set(_), Some(record)) = (edit, record.as_ref()) {
            check_joins(record, &stored.name)?;
        }
        Ok(with_edit(stored, pane, edit))
    }

    /// Step 4: the act failed with `failure` after `written`, so put the specs of `stored` back at
    /// `written`'s generation. The answer is `failure` once they are back, else the write's error.
    fn restore(
        &self,
        stored: Profile,
        written: Profile,
        pane: &PaneName,
        failure: PaneError,
    ) -> PaneError {
        let back = Profile {
            panes: stored.panes,
            ..written
        };
        let Err(error) = self.profiles.cas_put(&back, back.generation, &self.actor) else {
            return failure;
        };
        let (name, step) = (back.name.as_str(), reconcile_step(&back.name));
        if matches!(error, PaneError::Conflict) {
            return PaneError::ProfileConflict {
                what: format!(
                    "{name:?} was changed by another writer during the live change to {pane}, so \
                     its specs were not restored after that change or its record failed \
                     ({failure}); the other writer's version stays; {step}"
                ),
            };
        }
        // A timed-out restore is an unknown outcome: it may have landed.
        let holds = match error {
            PaneError::Timeout { .. } => "may still hold",
            _ => "still holds",
        };
        let context = format!(
            "profile {name:?} {holds} the edit of {pane}, but the live change or its record \
             failed ({failure}); {step}"
        );
        with_context(error, &context)
    }
}

impl ProfileScope for StoreScope {
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        let stored = self.stored(profile)?;
        let panes = match pane {
            None => self.members(&stored.name)?,
            Some(name) => vec![self.member(&stored.name, name)?],
        };
        Ok(ResolvedScope {
            profile: stored,
            panes,
        })
    }

    /// **Bound:** this implementation does not meet the trait's "returns within I5's bound
    /// (default 10 s)". It returns within the sum of its port calls (at most four: P, the pane
    /// record, the edit and the restore, each within I5's bound or `timeout`) plus the act's own
    /// time, which it can neither bound nor cancel. The write order is the module docs'.
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        let Some(profile) = profile else {
            return act().map(|()| None);
        };
        let stored = self.stored(profile)?;
        let edited = self.plan(&stored, pane, edit)?;
        let written = self
            .profiles
            .cas_put(&edited, stored.generation, &self.actor)
            .map_err(|error| may_have_landed(error, &stored.name, pane))?;
        match act() {
            Ok(()) => Ok(Some(written)),
            Err(failure) => Err(self.restore(stored, written, pane, failure)),
        }
    }
}

/// Step 2's error: nothing live has moved, so it is returned as it is (a conflict can simply be
/// run again), except a `timeout`, an unknown outcome: it says the edit may have landed.
fn may_have_landed(error: PaneError, profile: &ProfileName, pane: &PaneName) -> PaneError {
    if !matches!(error, PaneError::Timeout { .. }) {
        return error;
    }
    let context = format!(
        "the write may have landed, so profile {:?} may hold the edit of {pane}, and nothing live \
         was changed; {}",
        profile.as_str(),
        reconcile_step(profile)
    );
    with_context(error, &context)
}

/// Whether the record `pane` names `profile`, compared by slug.
fn belongs(pane: &Pane, profile: &ProfileName) -> bool {
    let named = pane.profile.as_ref();
    named.is_some_and(|named| named.slug() == profile.slug())
}

/// `usage` when `spec`, set as the spec of `pane`, names another pane (a verb's mistake).
fn check_filed_under(spec: &ProfileSpec, pane: &PaneName) -> Result<(), PaneError> {
    if spec.pane == pane.as_str() {
        return Ok(());
    }
    let named = &spec.pane;
    let message = format!("a spec for the pane {named:?} cannot be set as the spec of {pane}");
    Err(PaneError::Usage { message })
}

/// `pane-in-other-profile` when `record` is in a profile other than `profile` (slugs compared):
/// the pane registry's rule (ADR-0021 "Decisions taken", item 2), checked for a `Set` before any
/// write. A private copy, with the hub's text, of the hub's and the test kit's private copies.
fn check_joins(record: &Pane, profile: &ProfileName) -> Result<(), PaneError> {
    match record.profile.as_ref() {
        Some(current) if current.slug() != profile.slug() => {
            let (current, next) = (current.as_str(), profile.as_str());
            let what = format!("{} is in profile {current:?}, not {next:?}", record.name);
            Err(PaneError::PaneInOtherProfile { what })
        }
        _ => Ok(()),
    }
}

/// `stored` with `edit` made to the entry of `pane`: a `Set` replaces it in place or appends one;
/// a `Remove` drops it, the others keeping their order, and changes nothing when there is none.
fn with_edit(stored: &Profile, pane: &PaneName, edit: &SpecEdit) -> Profile {
    let mut panes = stored.panes.clone();
    match edit {
        SpecEdit::Set(spec) => {
            let spec = ProfileSpec::clone(spec);
            match panes.iter_mut().find(|entry| entry.pane == pane.as_str()) {
                Some(entry) => *entry = spec,
                None => panes.push(spec),
            }
        }
        SpecEdit::Remove => panes.retain(|entry| entry.pane != pane.as_str()),
    }
    Profile {
        panes,
        ..stored.clone()
    }
}

/// `text` POSIX-single-quoted, read back by a shell as one word: each `'` becomes the four
/// characters `'\''`, every other character stays, and the whole is wrapped in `'...'`.
fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// `error` with `; <context>` appended to its one string payload, so the code stays; a variant
/// with no payload is returned as it is. Every variant is named, with no catch-all arm, so a new
/// one fails to compile here instead of passing through unextended. `Timeout.op` carries the
/// context **on purpose**, a stretch of the convention that `op` is the operation that timed out:
/// `Timeout` has no other payload, and its code must stay.
fn with_context(mut error: PaneError, context: &str) -> PaneError {
    match &mut error {
        PaneError::Usage { message: text }
        | PaneError::ProbeFailed { message: text }
        | PaneError::HerdrVersionUnsupported { message: text }
        | PaneError::ProfileDrift { message: text }
        | PaneError::Refused { message: text, .. }
        | PaneError::GridAmbiguous { what: text }
        | PaneError::GridOutOfRange { what: text }
        | PaneError::ProfileConflict { what: text }
        | PaneError::ProfileNotFound { what: text }
        | PaneError::ProfileExists { what: text }
        | PaneError::ProfileHasLivePanes { what: text }
        | PaneError::PaneNotInProfile { what: text }
        | PaneError::PaneInOtherProfile { what: text }
        | PaneError::PaneNotFound { what: text }
        | PaneError::SessionNotFound { what: text }
        | PaneError::StoreCorrupt { what: text }
        | PaneError::Unavailable { what: text }
        | PaneError::Timeout { op: text } => {
            text.push_str("; ");
            text.push_str(context);
        }
        PaneError::NotImplemented
        | PaneError::CommandNotArgv
        | PaneError::EnvNameInvalid
        | PaneError::Conflict
        | PaneError::ProfileSecretRefused => {}
    }
    error
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
