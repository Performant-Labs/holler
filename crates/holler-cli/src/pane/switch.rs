//! `holler pane switch PANE SESSION [--as-operator] [--profile NAME]` (story #645).
//! **RED stub** (the pipeline's test-first phase): the verb's real arguments, and a body
//! that still answers `not-implemented`. F replaces the body with the engine call
//! (`holler_pane::tx_switch::switch`) and `emit_outcome`.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 645;

/// Point a pane at an existing session: its TUI and its record together.
#[derive(Args, Debug)]
pub struct PaneSwitch {
    /// The pane, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// The harness session to show and record (it must exist on the pane's server).
    #[arg(value_name = "SESSION")]
    pub session: String,
    /// Allow the orchestrator's own pane (the operator's deliberate act).
    #[arg(long)]
    pub as_operator: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane switch` (RED stub: refuse, naming the story that owns it).
pub fn run(_args: &PaneSwitch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
