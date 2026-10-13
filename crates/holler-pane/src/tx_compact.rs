//! The engine of `holler pane compact` (#878, ADR-0021 "Compaction"): compact the context of a
//! pane's session of record through the harness API, never by typing `/compact` into a TUI
//! (I4), on the model the harness's own config pins.
//!
//! The CLI resolves the record first (`say_cmd::resolve_pane_record`, the `interrupt_pane`
//! precedent), which owns `pane-not-found`, the profile's refusals, a pane with no session of
//! record and the pane-state refusals (`pane-parked`, `pane-unhealthy`,
//! `pane-shown-driven-mismatch`). [`compact`] owns only, in this order:
//!
//! 1. the endpoint of the record (`HarnessInfo::endpoint`), and a record with no session of
//!    record is `session-not-found` before any port is called (the CLI refused it already;
//!    this is the engine's defence);
//! 2. **health**, live (I6): `server-unhealthy` with doctor's remedy;
//! 3. **idle**: `session-busy`, `session-holds-question`, `session-holds-permission`;
//! 4. **the act**: `HarnessPort::compact` with no model, so the harness runs the compaction on
//!    its own pin; what it refuses or fails with is passed on as it is;
//! 5. **the shrink rule**: a context that did not shrink is `unavailable` (exit 1, the closed
//!    code of a mismatch observed after the act, I3), naming both counts.
//!
//! It reads no record, writes none (a compaction records nothing in the registry), applies no
//! pane-state predicate and calls no Herdr, host or hub port. It does not retry.

use crate::findings::quoted;
use crate::ports::CompactOutcome;
use crate::tx_switch::{check_health, check_idle};
use crate::{Pane, PaneError, Ports};

/// Compact the context of the session of record of `record`, in the module's order, and answer
/// the harness's counts when the context shrank.
pub fn compact(ports: Ports<'_>, record: &Pane) -> Result<CompactOutcome, PaneError> {
    let at = record.harness.endpoint(&record.name)?;
    let Some(session) = &record.session_of_record else {
        return Err(PaneError::SessionNotFound {
            what: format!("{} has no session of record", record.name),
        });
    };
    check_health(ports, record, &at)?;
    check_idle(ports, record, &at)?;
    let outcome = ports.harness.compact(&at, session, None)?;
    if outcome.tokens_after < outcome.tokens_before {
        return Ok(outcome);
    }
    Err(PaneError::Unavailable {
        what: format!(
            "the context of session {} of {} did not shrink: {} tokens before the compaction, \
             {} after",
            quoted(session),
            record.name,
            outcome.tokens_before,
            outcome.tokens_after
        ),
    })
}
