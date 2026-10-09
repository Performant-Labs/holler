//! The pane registry's place in the hub (epic #633; story #639 fills it).
//!
//! Skeleton (issue #669): [`PaneState`] holds nothing and [`dispatch`] answers every
//! `pane/*` method `not-implemented`, so a request reaches a stub and not
//! `method_not_found`. #639 gives `PaneState` its records, its persistence and its
//! change feed, and `dispatch` its five handlers (`pane/get`, `list`, `cas_put`,
//! `delete`, `watch`). It edits this directory and nothing else.
//!
//! The module root is `mod.rs`, which no other hub module is: the root has to sit
//! inside the `panes/**` blast radius that #639 owns, and a sibling `panes.rs` would
//! not.

use holler_pane::{PaneError, PaneReply};
use holler_proto::CorrelationId;
use serde_json::Value;

use crate::pane_dispatch::reply_line;
use crate::profile::ProfileState;
use crate::state::HubState;

/// The pane registry's state, shared by every control connection behind one `Arc`
/// ([`crate::pane_dispatch::PaneDeps`]).
///
/// It does not derive `Clone`. A copy per connection would split the registry, and a
/// split registry cannot make a compare-and-swap hold: share the `Arc`, never the
/// value.
pub struct PaneState;

impl PaneState {
    /// Load the registry's state from the state dir: empty in the skeleton. #639
    /// reads its file here, once, before the first connection, and owns what a
    /// corrupt or unreadable file means (fail closed, never a silent reset).
    pub fn load(_state: &HubState) -> Self {
        Self
    }
}

/// Answer one `pane/*` request: `not-implemented` until #639.
///
/// `profiles` is the profile registry: `pane/cas_put` calls
/// [`crate::profile::check_membership`] with it. `pane/watch` is long-poll, one
/// `{events, cursor}` reply per request, so the stub does not stream.
pub async fn dispatch(
    _method: &str,
    cid: &CorrelationId,
    _obj: &Value,
    _panes: &PaneState,
    _profiles: &ProfileState,
) -> String {
    reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented))
}
