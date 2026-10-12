//! The one classification of a refusal the hub answers a prompt with (#839, D5 (a)): which of
//! the few cases a pane verb tells apart. It decides no code and no exit; each caller maps what it
//! needs (P5):
//!
//! - the `--pane` arms of `say`, `interrupt` and `answer` act on [`HubRefusal::NotDriven`] alone,
//!   as `pane-not-driven` (exit 3), and leave every other case to the arms they had (the pane
//!   hold's `PaneHeld`, the held refusal with exit 4, the hub's own text with exit 1);
//! - the hub port's `prompt` (`wiring/hub.rs`), which `pane reset --first` alone reaches, maps
//!   each case to the open code its run ends with.
//!
//! A `-32004 not_connected` is `NotDriven` only when the hub marked it `offline` (P7): the same
//! code without the reason is a turn that outlived its wait, a cancelled prompt, an unconfirmed
//! cancel or a failed redirect, and keeps its text and its exit 1. A hub that predates the reason
//! never sends it, so its offline body falls through the same way, which is safe.

use holler_pane::error::RefusalCode;
use holler_proto::error::OFFLINE_REASON;
use holler_proto::{Code, WireError};

/// What a refusal of a prompt is, for a pane verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubRefusal {
    /// The hub knows no live body for the target (`-32003`, plain or `ambiguous`), or only an
    /// offline one (`-32004` with `data.reason: "offline"`). The hub's text.
    NotDriven(String),
    /// The hub's pane gate (`-32011` with `hold_kind: "pane"`): `code` is the pane's code, a
    /// well-formed open code, and `message` the hub's.
    PaneGate { code: String, message: String },
    /// An operator or a default hold on the session (`-32011` with `hold_kind` `operator` or
    /// `default`; absent, from a hub that predates default holds, means `operator`), with the
    /// operator's reason when one was given.
    Held {
        kind: String,
        reason: Option<String>,
    },
    /// The session is busy or holds a question (`-32009`). The hub's text.
    Busy(String),
    /// Anything else: an invalid grant, a bare `-32004`, a malformed refusal.
    Other,
}

/// Classify `error`, the refusal the hub answered a prompt with (see the module docs).
pub fn classify(error: &WireError) -> HubRefusal {
    let data = error.data.as_deref();
    let reason = data.and_then(|d| d.reason.as_deref());
    let message = error.message.clone();
    match Code::from_jsonrpc(error.code) {
        Some(Code::UnknownSession) => HubRefusal::NotDriven(message),
        Some(Code::NotConnected) if reason == Some(OFFLINE_REASON) => {
            HubRefusal::NotDriven(message)
        }
        Some(Code::SessionBusy) => HubRefusal::Busy(message),
        Some(Code::SessionHeld) => held(data.and_then(|d| d.hold_kind.as_deref()), reason, message),
        _ => HubRefusal::Other,
    }
}

/// A `-32011 session_held` by its `hold_kind`.
fn held(kind: Option<&str>, reason: Option<&str>, message: String) -> HubRefusal {
    match kind {
        Some("pane") => match reason.map(|code| RefusalCode::parse(code.to_owned())) {
            Some(Ok(code)) => HubRefusal::PaneGate {
                code: code.as_str().to_owned(),
                message,
            },
            Some(Err(_)) | None => HubRefusal::Other,
        },
        Some(kind @ ("operator" | "default")) => HubRefusal::Held {
            kind: kind.to_owned(),
            reason: reason.map(str::to_owned),
        },
        None => HubRefusal::Held {
            kind: "operator".to_owned(),
            reason: reason.map(str::to_owned),
        },
        Some(_) => HubRefusal::Other,
    }
}
