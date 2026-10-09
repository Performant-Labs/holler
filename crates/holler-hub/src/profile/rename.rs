//! `profile/rename` (PROPOSED, #665): one hub-side transaction that renames a profile
//! and moves every member pane's `Pane.profile` to the new name.
//!
//! Skeleton (issue #669): [`dispatch`] answers `not-implemented`. [`super::dispatch`]
//! already routes `profile/rename` here, so #665 fills this file and edits no file of
//! #661.

use holler_pane::{PaneError, PaneReply};
use holler_proto::CorrelationId;
use serde_json::Value;

use super::ProfileState;
use crate::pane_dispatch::reply_line;
use crate::panes::PaneState;

/// Answer `profile/rename`: `not-implemented` until #665. `panes` is the pane
/// registry that the transaction updates alongside the profile.
pub async fn dispatch(
    cid: &CorrelationId,
    _obj: &Value,
    _profiles: &ProfileState,
    _panes: &PaneState,
) -> String {
    reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented))
}
