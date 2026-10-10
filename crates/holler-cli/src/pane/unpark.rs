//! `holler pane unpark`: a stub (story #670). Story #646 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 646;

/// Return a parked pane to service.
#[derive(Args, Debug)]
pub struct PaneUnpark {
    /// Unpark only this pane (default with `--profile`: every pane of the profile).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane unpark`: refuse, naming the story that owns it.
pub fn run(_args: &PaneUnpark, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
