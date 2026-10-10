//! `holler pane reset PANE [--as-operator] [--profile NAME] [--first TEXT]` (story #645): start
//! a pane on a fresh session. The run creates it on the pane's harness server, shows it in the
//! TUI and records it, through the engine `pane switch` uses (`holler_pane::tx_switch`, with a
//! `Target::Fresh`), then, with `--first`, queues `TEXT` to it through the harness API. It is
//! the remedy `pane doctor` names for a pane with no session of record and for one whose
//! session of record is gone.
//!
//! The arguments, the output and the exit codes are `pane switch`'s (`super::switch`), and the
//! text line is `reset <pane> to a new session "<id>" (was "<id>")`. A pane whose session of
//! record is running a turn or holds a question is refused before anything moves
//! (`session-busy`, `session-holds-question`). A failure after the session was created names
//! it: it stays on the server unrecorded, and doctor reports it as a stray session. A first
//! message that does not land fails the run after the record, which stands. After a
//! successful reset the previous session stays on the server too, as a stray: no port deletes
//! a session.

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
    /// The first message, queued to the new session once it is recorded.
    #[arg(long, value_name = "TEXT")]
    pub first: Option<String>,
}

/// Run `holler pane reset`: move the pane to a session the run creates, then print the result.
/// It takes `pane switch`'s own path, [`execute`], which prints through `emit_outcome`, with a
/// fresh target, so none of switch's code is copied here.
pub fn run(args: &PaneReset, ctx: &mut VerbCtx<'_>) -> i32 {
    execute(
        ctx,
        Verb::Reset,
        &args.pane,
        &args.profile,
        args.as_operator,
        Ok(Target::Fresh),
        args.first.clone(),
    )
}
