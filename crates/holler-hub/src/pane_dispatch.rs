//! The forwarding seam between the control socket and the pane and profile
//! registries (issue #669, epic #633, skeleton slice b).
//!
//! A `pane/*` or `profile/*` request used to fall through
//! [`crate::control_server`]'s dispatcher to `method_not_found`. The dispatcher now
//! has one arm for the two method lists of `holler_proto::methods` that calls
//! `forward`, and `forward` calls [`crate::panes::dispatch`] or
//! [`crate::profile::dispatch`]. These are plain functions, with no registry of
//! function pointers: the stories that fill the registries (#639 for panes, #661 for
//! profiles) edit their own modules and never this file, `control_server.rs` or
//! `serve.rs`.
//!
//! # State
//!
//! [`PaneDeps`] bundles the two state handles. `serve::build_shared_state` builds it
//! once, before the first connection is accepted, so each registry's own `load` has
//! run (and has decided what a corrupt file means) before any request is answered;
//! the accept loop then clones the bundle per control connection. Both handles are
//! `Arc`s of types that do not derive `Clone`: every connection shares the one
//! registry and none can fork a copy, which would defeat the compare-and-swap.
//!
//! Each dispatcher gets **both** handles. The pane side needs the profile registry
//! for the membership check ([`crate::profile::check_membership`]) and the profile
//! side needs the pane registry for `profile/rename`.
//!
//! # Replies
//!
//! A [`PaneReply`] is the JSON-RPC **result**, never a JSON-RPC error. This differs
//! from the other control methods, which answer a refusal with an error and a
//! `data.reason`: the pane codes are 22 kebab-case names that do not fit the closed
//! wire [`Code`] table (see `holler_pane::reply`). Only a request that never reaches
//! a handler stays a JSON-RPC error: framing errors, and a method in neither list.
//! `reply_line` is the one place a reply becomes a line.

use std::sync::Arc;

use holler_pane::PaneReply;
use holler_proto::methods::{is_pane_method, is_profile_method};
use holler_proto::{Code, CorrelationId};
use serde_json::Value;

use crate::control_server::{encode_error, encode_response};
use crate::panes::{self, PaneState};
use crate::profile::{self, ProfileState};
use crate::state::HubState;

/// The shared state of the pane and profile registries: one `Arc` per registry, built
/// once per hub process and cloned (it is only `Arc`s) per control connection.
#[derive(Clone)]
pub struct PaneDeps {
    /// The pane registry's state (#639).
    pub panes: Arc<PaneState>,
    /// The profile registry's state (#661).
    pub profiles: Arc<ProfileState>,
}

impl PaneDeps {
    /// Load both registries' state from `state`. Each `load` owns what a missing,
    /// unreadable or corrupt file means, so a registry's own story decides it.
    pub fn load(state: &HubState) -> Self {
        Self {
            panes: Arc::new(PaneState::load(state)),
            profiles: Arc::new(ProfileState::load(state)),
        }
    }
}

/// Answer one `pane/*` or `profile/*` request: route `method` to its registry's
/// dispatcher, giving it both state handles.
///
/// The `Arc` fields are passed as they are. They deref-coerce to the `&PaneState` and
/// `&ProfileState` of today's dispatchers, and a story whose handler must move an
/// owned handle into `spawn_blocking` can widen its own `dispatch` to `&Arc<_>`
/// without editing this file.
pub(crate) async fn forward(
    method: &str,
    cid: &CorrelationId,
    obj: &Value,
    deps: &PaneDeps,
) -> String {
    if is_pane_method(method) {
        panes::dispatch(method, cid, obj, &deps.panes, &deps.profiles).await
    } else if is_profile_method(method) {
        profile::dispatch(method, cid, obj, &deps.profiles, &deps.panes).await
    } else {
        // The caller's arm only forwards a method in one of the lists, so this is not
        // reached. It answers like every other unknown method rather than panicking.
        encode_error(
            cid,
            Code::MethodNotFound,
            format!("unknown control method: {method}"),
        )
    }
}

/// A [`PaneReply`] as one control-socket reply line: the JSON-RPC result of the
/// request that `cid` names.
pub(crate) fn reply_line(cid: &CorrelationId, reply: &PaneReply) -> String {
    // A reply is a bool and two optional values, so it always serializes.
    encode_response(cid, serde_json::to_value(reply).unwrap_or_default())
}
