//! `holler pane import`: a stub (story #670). Story #650 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 650;

/// Import the existing fleet into the pane registry.
#[derive(Args, Debug)]
pub struct PaneImport {}

/// Run `holler pane import`: refuse, naming the story that owns it.
pub fn run(_args: &PaneImport, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
