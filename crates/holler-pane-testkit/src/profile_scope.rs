//! `FakeProfileScope`, the in-memory `ProfileScope` that verb tests run against instead
//! of the CLI's (#663): membership, the every-pane scope and the spec-edit transaction,
//! in ADR-0021's I8 write order, over any `ProfileStore` and `PaneStore` (slice c of
//! #638, #688).
//!
//! The fake passes the `ProfileScope` conformance suite
//! ([`crate::conformance::profile_scope`]), built over the fake profile and pane stores.
//! It has no fault switch of its own: it is not a port's boundary but a composition of
//! two ports, so a test injects faults into the stores it wraps, which also shows how a
//! real scope meets them. What only the fake has is
//! [`FakeProfileScope::before_next_restore`], a one-shot hook that lets another writer
//! move the profile between a failed act and the restoring write, the one moment no
//! store fault can reach.
//!
//! Where the fake decides what the port and ADR-0021 leave open, an `ASSUMPTION (#663)`
//! comment says so at the code concerned. The suite's module docs list them all.

use std::sync::{Arc, Mutex};

use holler_pane::{
    Actor, Pane, PaneError, PaneName, PaneStore, Profile, ProfileName, ProfileScope, ProfileSpec,
    ProfileStore, ResolvedScope, SpecEdit,
};

use crate::feed::lock;
use crate::pane_store::check_membership;

/// What [`FakeProfileScope::before_next_restore`] arms.
type RestoreHook = Box<dyn FnOnce() + Send>;

/// A `ProfileScope` over any `ProfileStore` and `PaneStore`, keeping ADR-0021's I8 write
/// order (section 8).
///
/// - `resolve(P, None)` is the stored P and every pane whose record names P, in name
///   order. `resolve(P, Some(n))` is P and n, whose record must name P, or else
///   `pane-not-in-profile` (a pane with no record included). A missing P is
///   `profile-not-found`, checked first. Membership is `Pane.profile` alone, compared by
///   slug, so a spec of P adds no pane to its scope.
/// - `edit_spec(Some(P), n, edit, act)` reads P (`profile-not-found` before anything
///   else) and n's record, then writes P with the edit by a compare-and-swap at P's
///   generation, and only then runs `act`. A conflict on that first write is
///   `generation-conflict`, and the act never runs. A `Set` replaces n's entry in place
///   or appends one; a `Remove` drops it, and still writes when there is none. If the
///   act fails, the scope puts P's specs back by a second compare-and-swap, so the
///   generation moves by two and P's log shows the edit and its reversal; a conflict
///   there is `profile-conflict`. It returns P as the first write stored it.
/// - `edit_spec(None, ..)` runs only the act and calls neither store.
///
/// Its own guards, both before anything is written: a `Set` whose spec names another
/// pane is `usage`, and a `Set` for a pane whose record belongs to another profile is
/// `pane-in-other-profile`, by the fake pane store's membership rule. A `Remove` is never
/// refused for membership: a detached spec stays removable.
///
/// It never writes a pane record: recording the pane (ADR-0021 section 8, step 4) is
/// the verb's, inside its act.
pub struct FakeProfileScope {
    profiles: Arc<dyn ProfileStore>,
    panes: Arc<dyn PaneStore>,
    /// Who every profile write of this scope is logged as.
    actor: Actor,
    /// The one-shot hook of `before_next_restore`, if armed.
    restore_hook: Mutex<Option<RestoreHook>>,
}

impl FakeProfileScope {
    // ASSUMPTION (#663): a scope can be built over any `ProfileStore` and `PaneStore`;
    // the suite builds the one under test over the two fakes.
    /// A scope over `profiles` and `panes` that logs every profile write as `actor`,
    /// with no hook armed.
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
        Self {
            profiles,
            panes,
            actor,
            restore_hook: Mutex::new(None),
        }
    }

    /// Run `hook` once, after the next act that fails in an `edit_spec` with a profile
    /// and before the scope's restoring write; then it is dropped. A failing act without
    /// a profile, a first write that fails and an act that succeeds leave it armed.
    ///
    /// A verb test makes another writer move the profile there, typically
    /// `move || { profiles.concurrent_put(&other, &other_actor).unwrap(); }` over its own
    /// `Arc<FakeProfileStore>`, so the restore really conflicts and `edit_spec` answers
    /// `profile-conflict` (ADR-0021 section 8, step 6). Arming it again replaces an
    /// unused hook. The hook runs with no lock of the scope held, so it may arm the next
    /// one.
    pub fn before_next_restore(&self, hook: impl FnOnce() + Send + 'static) {
        let replaced = lock(&self.restore_hook).replace(Box::new(hook));
        // Dropped here, after the lock: what a replaced hook captured runs no drop code
        // under it.
        drop(replaced);
    }

    /// The profile `profile` as stored: `profile-not-found` when there is none.
    fn stored(&self, profile: &ProfileName) -> Result<Profile, PaneError> {
        self.profiles
            .get(profile)?
            .ok_or_else(|| PaneError::ProfileNotFound {
                what: profile.to_string(),
            })
    }

    /// Every pane whose record names `profile`, in name order.
    fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
        let mut members: Vec<Pane> = self
            .panes
            .list()?
            .into_iter()
            .filter(|pane| belongs(pane, profile))
            .collect();
        members.sort_by(|x, y| x.name.cmp(&y.name));
        Ok(members)
    }

    /// The pane `name`, whose record must name `profile`: `pane-not-in-profile`
    /// otherwise, a pane with no record included.
    fn member(&self, profile: &ProfileName, name: &PaneName) -> Result<Pane, PaneError> {
        match self.panes.get(name)? {
            Some(pane) if belongs(&pane, profile) => Ok(pane),
            _ => Err(PaneError::PaneNotInProfile {
                what: format!("{name} is not in profile {:?}", profile.as_str()),
            }),
        }
    }

    /// Section 8, step 1: `stored` with the edit, once the scope's guards pass. The pane
    /// record is read for every edit; only a `Set` is checked for membership.
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

    /// Section 8, steps 5 and 6: `act` failed with `failure` after `written`, so put the
    /// specs of `stored` back by a compare-and-swap at `written`'s generation. Returns
    /// what `edit_spec` answers: `failure` once they are back, `profile-conflict` when
    /// another writer moved the profile first, or else the restoring write's own error.
    fn restore(
        &self,
        stored: Profile,
        written: Profile,
        pane: &PaneName,
        failure: PaneError,
    ) -> PaneError {
        // A statement of its own, so the lock is released before the hook runs.
        let hook = lock(&self.restore_hook).take();
        if let Some(hook) = hook {
            hook();
        }
        let restored = Profile {
            panes: stored.panes,
            ..written
        };
        // ASSUMPTION (#663): a restore is one compare-and-swap at g + 1, not retried.
        match self
            .profiles
            .cas_put(&restored, restored.generation, &self.actor)
        {
            Ok(_) => failure,
            Err(PaneError::Conflict) => PaneError::ProfileConflict {
                what: format!(
                    "{:?} was changed by another writer during the live change to {pane}, \
                     so its specs were not restored after that change failed ({failure}); \
                     the other writer's version stays",
                    restored.name.as_str()
                ),
            },
            // ASSUMPTION (#663), open: a restoring write that fails with anything but a
            // conflict returns its own error, so the profile keeps an edit nothing live
            // matches, the error does not name it, and the act's error is lost. ADR-0021
            // section 8 decides only the conflict (step 6); #663 decides this case, and
            // the fake and the suite's list are amended to match.
            Err(other) => other,
        }
    }
}

impl ProfileScope for FakeProfileScope {
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

    // ASSUMPTION (#663): the scope writes no pane record. Recording the pane is the
    // verb's, inside its act, whose signature returns `()`.
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        // ASSUMPTION (#663): without a profile the scope makes no store call at all.
        let Some(profile) = profile else {
            return act().map(|()| None);
        };
        // ASSUMPTION (#663): `profile-not-found` comes before `pane-in-other-profile`.
        let stored = self.stored(profile)?;
        let edited = self.plan(&stored, pane, edit)?;
        // ASSUMPTION (#663): the first write is not retried on a conflict.
        let written = self
            .profiles
            .cas_put(&edited, stored.generation, &self.actor)?;
        match act() {
            Ok(()) => Ok(Some(written)),
            Err(failure) => Err(self.restore(stored, written, pane, failure)),
        }
    }
}

/// Whether the record `pane` names `profile`, compared by slug.
fn belongs(pane: &Pane, profile: &ProfileName) -> bool {
    pane.profile
        .as_ref()
        .is_some_and(|named| named.slug() == profile.slug())
}

/// The fake's own guard against a verb that files a spec under the wrong pane: `usage`
/// when `spec` names a pane other than `pane`.
fn check_filed_under(spec: &ProfileSpec, pane: &PaneName) -> Result<(), PaneError> {
    if spec.pane == pane.as_str() {
        Ok(())
    } else {
        Err(PaneError::Usage {
            message: format!(
                "a spec for the pane {:?} cannot be set as the spec of {pane}",
                spec.pane
            ),
        })
    }
}

// ASSUMPTION (#661/#663): the scope checks `pane-in-other-profile` itself for a `Set`,
// before the profile write, as well as the pane registry doing so inside its
// compare-and-swap (ADR-0021 "Decisions taken", item 2), so nothing is written to the
// profile and nothing live moves for a pane that cannot join it. It runs with or
// without `--spec-only`, since the scope cannot see that an act is empty. ADR-0021
// section 8, step 1 states it. A `Remove` is not checked: a detached spec stays
// removable.
/// The refusal of a `Set` for `record` in `profile`: the fake pane store's membership
/// rule (slugs compared), applied as if the record were written into `profile`.
fn check_joins(record: &Pane, profile: &ProfileName) -> Result<(), PaneError> {
    let joined = Pane {
        profile: Some(profile.clone()),
        ..record.clone()
    };
    check_membership(Some(record), &joined)
}

// ASSUMPTION (#663): a `Set` replaces an entry in place, keeping its index.
/// `stored` with `edit` made to the entry of `pane`: a `Set` replaces it in place or
/// appends one; a `Remove` drops it, the other entries keeping their order, and leaves
/// the specs as they are when there is none.
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
