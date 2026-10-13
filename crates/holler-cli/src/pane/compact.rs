//! `holler pane compact PANE [--profile NAME]` (#878, ADR-0021 "Compaction"): compact the context
//! of a pane's session of record through the harness API, never by typing `/compact` into a TUI
//! (I4), on the compaction model the harness's own server config pins. The verb names no model
//! and has no flag for one (the operator's decision N1).
//!
//! - **Plan.** The record is resolved as `interrupt --pane` resolves it
//!   (`say_cmd::resolve_pane_record`): `PANE` and `--profile` are typed (`usage`), the record is
//!   read through the profile's scope when one is named (`profile-not-found`,
//!   `pane-not-in-profile`), a pane with no record is `pane-unmanaged` when Herdr shows it and
//!   `pane-not-found` otherwise (#832), a pane with no session of record is
//!   `session-not-found`, and a parked pane, one whose record says its harness is unhealthy and
//!   one whose SHOWN and DRIVEN differ are refused before the harness is reached.
//! - **Engine.** `holler_pane::tx_compact::compact`: the live health and idle checks, the
//!   harness's `compact` naming no model, and the shrink rule. It records nothing.
//! - **Output.** Text is one line, `compacted <pane>: <before> -> <after> tokens`. JSON data is
//!   `{"pane", "session", "tokens_before", "tokens_after"}`. Exit 0 only when the context shrank;
//!   a refusal is 3, a context that did not shrink or a failure 1, usage 2. Nothing is retried.
//!   A compaction is a model call and may take the harness's compaction bound (120 s for
//!   OpenCode); the verb prints nothing until it ends.

use clap::Args;
use holler_pane::tx_compact;
use holler_pane::{unmanaged, PaneError, PaneName, Ports};
use serde::Serialize;

use super::args::ProfileOpt;
use crate::output::{emit, ErrorBody, VerbCtx};
use crate::say_cmd::resolve_pane_record;

/// Compact the context of a pane's session, on the model its harness's config pins.
#[derive(Args, Debug)]
pub struct PaneCompact {
    /// The pane, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane compact`: compact the pane's session of record, then print both counts.
pub fn run(args: &PaneCompact, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = compact(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, result, Compacted::text)
}

/// What a run did. Its JSON keys are its four fields.
#[derive(Debug, Serialize)]
struct Compacted {
    pane: PaneName,
    session: String,
    tokens_before: u64,
    tokens_after: u64,
}

impl Compacted {
    /// The text form, one line.
    fn text(&self) -> String {
        format!(
            "compacted {}: {} -> {} tokens",
            self.pane, self.tokens_before, self.tokens_after
        )
    }
}

/// The record, resolved as the module docs say, then the engine.
fn compact(args: &PaneCompact, ports: Ports<'_>) -> Result<Compacted, PaneError> {
    let record = match resolve_pane_record(ports, &args.pane, args.profile.profile.as_deref()) {
        Err(PaneError::PaneNotFound { .. }) => return Err(unmanaged::missing(ports, &args.pane)),
        resolved => resolved?,
    };
    let outcome = tx_compact::compact(ports, &record)?;
    Ok(Compacted {
        pane: record.name,
        session: record.session_of_record.unwrap_or_default(),
        tokens_before: outcome.tokens_before,
        tokens_after: outcome.tokens_after,
    })
}
