//! `session/rebind` (issue #839, ADR 0021 section 11 point 2): the hub asks an attach-mode body
//! to re-point one of its own sessions to another harness session on the same endpoint.
//!
//! An additive request, hub to body (the catalog's 23rd row). The session's **name** does not
//! change, so its address and its hold key do not (ADR 0005 section 3: the name is the identity,
//! the locator changes). The body answers only after the new driver targets the new id and the
//! session's presence advertises it; a refusal keeps the old binding (`docs/protocol/v2.md`,
//! "Attach-mode sessions", "Re-pointing").
//!
//! The two types live here and not in `docs.rs`, which is at the file-size limit; they follow its
//! conventions (`deny_unknown_fields`, one golden file each under `tests/golden/docs/`).

use serde::{Deserialize, Serialize};

/// The `params` of `session/rebind`: which of the body's sessions to re-point, and to what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rebind {
    /// The body's bare session name (`alpha`, not `kiwi/alpha`).
    pub session: String,
    /// The harness session the body is to drive from now on.
    pub harness_session_id: String,
}

/// The `result` of `session/rebind`: sent only after the driver targets the new id and the
/// session's presence advertises it. `harness_session_id` is the id the body acknowledged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RebindResult {
    /// The body's bare session name, unchanged by the rebind.
    pub session: String,
    /// The harness session the body now drives.
    pub harness_session_id: String,
}
