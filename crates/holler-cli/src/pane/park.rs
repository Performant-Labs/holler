//! `holler pane park`: a stub (story #670). Story #646 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 646;

/// Take a pane out of service until it is unparked.
#[derive(Args, Debug)]
pub struct PanePark {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane park`: refuse, naming the story that owns it.
pub fn run(_args: &PanePark, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
