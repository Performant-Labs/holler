//! The pane-state gate of the hub's prompt path (story #646 part 3; keyed on the advertised
//! harness session id since #839, ADR-0021 section 11 point 1).
//!
//! `circuit::dispatch::send_prompt`, the one place a prompt reaches a body, calls
//! [`pane_gate`] after the session hold, and so does `talk::admit`, the advisory fast path of
//! `say` and of `control/say`'s `check_only`, so the question and the enforcement are one
//! function. The rule itself is `holler_pane::gate`'s: [`first_refusal`] selects, of the panes
//! whose session of record or recorded DRIVEN is the id being prompted, the first one its
//! predicate refuses (parked, unhealthy, or SHOWN and DRIVEN both observed and differing). The
//! `--pane` routing of the CLI and the test kit's `FakeHub` call the same functions; this file
//! keeps only the hub's words and its refusal frame.

use holler_pane::gate::{first_refusal, PromptRefusal};
use holler_pane::PaneStore as _;
use holler_proto::{Code, WireError};

use super::PaneState;

/// What the hub's refusal says a refused pane is, in a few words.
fn pane_state_refusal(refusal: PromptRefusal) -> &'static str {
    match refusal {
        PromptRefusal::Parked => "is parked",
        PromptRefusal::Unhealthy => "is unhealthy",
        PromptRefusal::ShownDrivenMismatch => "shows a session other than the one it drives",
    }
}

/// The pane-state gate: refuse a prompt to the session that advertises the harness session
/// `id` while a pane whose session of record or recorded DRIVEN is `id` is refused by the one
/// predicate. Fail closed across panes that record one id: any bad one refuses, and the first
/// bad one in name order (`list` is name-sorted) is the one named. One synchronous read with no
/// `await`, so it is totally ordered against a pane write, as `holds.admit` is against `hold`
/// and `release`.
///
/// The refusal is `-32011 session_held` with `data.hold_kind = "pane"` and `data.reason` the
/// pane's code (no `data.since`: the park's is epoch milliseconds, not the RFC 3339 the field
/// documents). `holler release` does not lift it; the remedy is `holler pane unpark` or `holler
/// pane doctor`.
///
/// A registry that cannot be read (`store-corrupt`, logged by the registry when it failed to
/// load) refuses nothing, as a hold file that cannot be read leaves the hub with no holds
/// (`crate::holds`): a broken pane file must not stop every prompt to every session.
pub(crate) fn pane_gate(panes: &PaneState, id: &str) -> Result<(), WireError> {
    let Ok(records) = panes.list() else {
        return Ok(());
    };
    let Some((pane, refusal)) = first_refusal(&records, id) else {
        return Ok(());
    };
    let code = refusal.code();
    let message = format!(
        "pane {} {}, so it refuses prompts ({code})",
        pane.name,
        pane_state_refusal(refusal)
    );
    let mut error = WireError::new(Code::SessionHeld, message, None).with_hold_kind("pane");
    if let Some(data) = error.data.as_mut() {
        data.reason = Some(code.as_str().to_owned());
    }
    Err(error)
}

#[cfg(test)]
mod tests;
