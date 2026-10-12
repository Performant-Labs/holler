//! `profile-drift` (#665): a pane in scope whose record differs from its profile's spec on at
//! least one field of `profile_diff`'s table (ADR-0021 section 3, "Profile files as built").
//!
//! It is `profile show`'s comparison, literally: [`diff_profile`] over the profile and its
//! members in scope ([`is_member`], by slug), so the two agree. Only a `Differs` row is drift: a
//! spec with no pane (`Missing`) and a member with no spec (`Extra`) are `profile apply`'s
//! business. A pane that two specs of its profile name is judged by its first differing row.
//!
//! One finding per drifted pane per pass, naming every differing field in `SpecField::ALL`
//! order and no value (a value can be a path or a command), with no remedy: which side is
//! right is the operator's choice. A pass keeps no memory, so the same finding comes back in
//! every pass while the difference stands: "once per episode" is once per pass.
//!
//! The profiles are read with one `profile_store.list`, only when a pane in scope names a
//! profile, so a pass over panes in no profile calls no profile store. A failed read is one
//! `observe-failed` finding naming the call, and no drift is concluded from it. A pane that
//! names a profile that does not exist is not drift, and is not reported here.

use crate::findings::{quoted, Finding, FindingKind};
use crate::profile_diff::{diff_profile, is_member, PaneDiff, PaneStatus};
use crate::{Pane, Profile, ProfileStore};

use super::failed_message;

/// The read of the profiles, named as `<port>.<method>` in an `observe-failed` message.
const PROFILE_STORE_LIST: &str = "profile_store.list";

/// The `profile-drift` findings of the panes of `scope` (see the module docs), or the
/// `observe-failed` finding of a read of the profiles that failed.
pub(super) fn findings(profiles: &dyn ProfileStore, scope: &[Pane]) -> Vec<Finding> {
    if scope.iter().all(|pane| pane.profile.is_none()) {
        return Vec::new();
    }
    match profiles.list() {
        Ok(stored) => stored
            .iter()
            .flat_map(|profile| drifted(profile, scope))
            .collect(),
        Err(error) => vec![Finding::new(
            FindingKind::ObserveFailed,
            failed_message(PROFILE_STORE_LIST, &error),
        )],
    }
}

/// One finding for each member of `profile` in `scope` that has a differing row in the
/// comparison of `profile` with those members.
fn drifted(profile: &Profile, scope: &[Pane]) -> Vec<Finding> {
    let members: Vec<Pane> = scope
        .iter()
        .filter(|pane| is_member(pane, &profile.name))
        .cloned()
        .collect();
    if members.is_empty() {
        return Vec::new();
    }
    let rows = diff_profile(profile, &members);
    members
        .iter()
        .filter_map(|pane| {
            let row = rows
                .iter()
                .find(|row| row.status == PaneStatus::Differs && row.pane == pane.name.as_str())?;
            let message = message(profile, row);
            Some(Finding::about(FindingKind::ProfileDrift, pane, message))
        })
        .collect()
}

/// `differs from profile <name> on <field>, <field> (see holler profile show)`: the profile's
/// name quoted, the fields by their dotted paths, and no value.
fn message(profile: &Profile, row: &PaneDiff) -> String {
    let fields: Vec<&str> = row
        .differences
        .iter()
        .map(|difference| difference.field.as_str())
        .collect();
    format!(
        "differs from profile {} on {} (see holler profile show)",
        quoted(profile.name.as_str()),
        fields.join(", ")
    )
}
