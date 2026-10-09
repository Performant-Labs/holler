//! `holler pane close`: a stub (story #670). Story #646 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::{ProfileOpt, SpecOnly};
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 646;

/// Close a pane and remove its record.
#[derive(Args, Debug)]
pub struct PaneClose {
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane close`: refuse, naming the story that owns it.
pub fn run(_args: &PaneClose, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
