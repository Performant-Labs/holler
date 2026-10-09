//! `holler profile apply`: a stub (story #670). Story #664 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 664;

/// Converge the live panes on a profile.
#[derive(Args, Debug)]
pub struct ProfileApply {
    /// Take over a pane that belongs to another profile.
    #[arg(long)]
    pub take_over: bool,
}

/// Run `holler profile apply`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileApply, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
