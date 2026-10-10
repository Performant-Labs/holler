//! `holler pane reset PANE [--as-operator] [--profile NAME]` (story #645). **RED stub**
//! (the pipeline's test-first phase): the verb's real arguments, and a body that still
//! answers `not-implemented`. F replaces the body with the engine call and
//! `super::switch::emit_outcome`.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 645;

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

/// Run `holler pane reset` (RED stub: refuse, naming the story that owns it).
pub fn run(_args: &PaneReset, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
