//! The profile registry's place in the hub (epic #633; story #661 fills it).
//!
//! Skeleton (issue #669): [`ProfileState`] holds nothing and [`dispatch`] answers
//! every `profile/*` method `not-implemented`, so a request reaches a stub and not
//! `method_not_found`. #661 gives `ProfileState` its records, its persistence, its
//! change log and its change feed, and `dispatch` its six handlers (`profile/get`,
//! `list`, `cas_put`, `delete`, `watch`, `log`). It edits this directory and nothing
//! else. `profile/rename` (PROPOSED, #665) is routed to [`rename`], so that #665
//! edits only `rename.rs`.
//!
//! The module root is `mod.rs`, which no other hub module is: the root has to sit
//! inside the `profile/**` blast radius that #661 owns, and a sibling `profile.rs`
//! would not.

pub mod rename;

use holler_pane::{Pane, PaneError, PaneReply};
use holler_proto::CorrelationId;
use serde_json::Value;

use crate::pane_dispatch::reply_line;
use crate::panes::PaneState;
use crate::state::HubState;

/// The profile registry's state, shared by every control connection behind one `Arc`
/// ([`crate::pane_dispatch::PaneDeps`]).
///
/// It does not derive `Clone`. A copy per connection would split the registry, and a
/// split registry cannot make a compare-and-swap hold: share the `Arc`, never the
/// value.
pub struct ProfileState;

impl ProfileState {
    /// Load the registry's state from the state dir: empty in the skeleton. #661
    /// reads its file here, once, before the first connection, and owns what a
    /// corrupt or unreadable file means (fail closed, never a silent reset).
    pub fn load(_state: &HubState) -> Self {
        Self
    }
}

/// Answer one `profile/*` request: `not-implemented` until #661 (and #665 for
/// `profile/rename`).
///
/// `panes` is the pane registry, which `profile/rename` reaches to move a profile's
/// member panes to the new name. `profile/watch` is long-poll, one `{events, cursor}`
/// reply per request, so the stub does not stream.
pub async fn dispatch(
    method: &str,
    cid: &CorrelationId,
    obj: &Value,
    profiles: &ProfileState,
    panes: &PaneState,
) -> String {
    match method {
        "profile/rename" => rename::dispatch(cid, obj, profiles, panes).await,
        _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
    }
}

/// The membership check of a pane write: may `pane` be recorded with the `profile` it
/// names? A plain function that `pane/cas_put` (#639) calls and #661 fills with the
/// profile rules (`pane-in-other-profile`, a profile that does not exist).
///
/// Skeleton (issue #669): accepts every pane. Until #661 fills it, nothing refuses a
/// pane that names a profile.
pub fn check_membership(_pane: &Pane, _profiles: &ProfileState) -> Result<(), PaneError> {
    Ok(())
}
