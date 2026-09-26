//! The session-hold vocabulary (issue #441, umbrella #437): the additive,
//! optional roster fields that say a session is held.
//!
//! A hold is **hub** state: an operator (`holler hold`) tells the hub to refuse
//! new work to a session, and the hub enforces it at the one point every prompt
//! passes through. A body never advertises one, so these fields are **not**
//! part of the `session/presence` document ([`crate::SessionAd`] rejects
//! unknown fields); they ride on the hub's roster rows (`holler roster
//! --json`). They are additive and optional, so there is no protocol version
//! bump (the same reasoning as ADR 0014's `mode` / `harness_session_id`), and a
//! roster row for a session that is not held carries none of them, byte for
//! byte as before.

use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !*b
}

/// The hold fields of one roster row. Flatten it into the row
/// (`#[serde(flatten)]`); a session that is not held serialises to nothing.
///
/// - `hold` is present (and `true`) only while the session is held; absent
///   means not held.
/// - `hold_reason` is the operator's free text, present only when one was given.
/// - `held_since` (RFC 3339) is when the hold was set, present whenever `hold`
///   is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHold {
    /// `true` while the session is held.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold: bool,
    /// Why, in the operator's words. Absent when none was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_reason: Option<String>,
    /// When the hold was set (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_since: Option<String>,
    /// Which hold the three fields above describe (issue #460): `operator`
    /// (`holler hold`) or `default` (the session joined held). Present whenever
    /// `hold` is; a hub that predates default holds never sends it, which means
    /// `operator`. When both holds are on a session it names the operator hold
    /// (the one that beats a release grant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_kind: Option<String>,
    /// `true` only when a default hold sits under an operator hold (releasing
    /// the operator hold then leaves the session held by default).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold_default: bool,
}

impl SessionHold {
    /// This hold, labelled with its kind (`operator` or `default`).
    pub fn of_kind(mut self, kind: &str) -> Self {
        self.hold_kind = Some(kind.to_owned());
        self
    }

    /// A held session: `reason` is the operator's text (if any), `since` the
    /// RFC 3339 time the hold was set.
    pub fn held(reason: Option<String>, since: String) -> Self {
        Self { hold: true, hold_reason: reason, held_since: Some(since), hold_kind: None, hold_default: false }
    }
}
