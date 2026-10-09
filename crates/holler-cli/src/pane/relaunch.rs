//! `holler pane relaunch`: a stub (story #670). Story #644 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::{ProfileOpt, SpecFlags, SpecOnly};
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 644;

/// Launch a pane again, replacing its process.
#[derive(Args, Debug)]
pub struct PaneRelaunch {
    #[command(flatten)]
    pub spec: SpecFlags,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane relaunch`: refuse, naming the story that owns it.
pub fn run(_args: &PaneRelaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
