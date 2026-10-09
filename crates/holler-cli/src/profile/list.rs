//! `holler profile list`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// List profiles.
#[derive(Args, Debug)]
pub struct ProfileList {}

/// Run `holler profile list`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileList, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
