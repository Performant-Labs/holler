//! `holler pane reset PANE [--as-operator] [--profile NAME]` (story #645): start a pane on a
//! fresh session. The run creates it on the pane's harness server, shows it in the TUI and
//! records it, through the engine `pane switch` uses (`holler_pane::tx_switch`, with a
//! `Target::Fresh`). It is the remedy `pane doctor` names for a pane with no session of record
//! and for one whose session of record is gone.
//!
//! The arguments, the output and the exit codes are `pane switch`'s (`super::switch`), and the
//! text line is `reset <pane> to a new session "<id>" (was "<id>")`. A failure after the
//! session was created names it: it stays on the server unrecorded, and doctor reports it as a
//! stray session. After a successful reset the previous session stays on the server too, as a
//! stray: no port deletes a session.

use clap::Args;
use holler_pane::tx_switch::Target;

use super::args::ProfileOpt;
use super::switch::{execute, Verb};
use crate::output::VerbCtx;

/// Start a pane on a fresh session: create it, show it, record it.
#[derive(Args, Debug)]
pub struct PaneReset {
    /// The pane, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// Allow the orchestrator's own pane (the operator's deliberate act).
    #[arg(long)]
    pub as_operator: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane reset`: move the pane to a session the run creates, then print the result.
pub fn run(args: &PaneReset, ctx: &mut VerbCtx<'_>) -> i32 {
    execute(
        ctx,
        Verb::Reset,
        &args.pane,
        &args.profile,
        args.as_operator,
        Ok(Target::Fresh),
    )
}
