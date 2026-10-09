//! `holler profile rename`: a stub (story #670). Story #665 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
//!
//! Proposed, not confirmed: #665 waits for the operator, so this verb and its ADR 0003 row
//! may be dropped.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 665;

/// Rename a profile. (proposed, #665: the operator confirms it first)
#[derive(Args, Debug)]
pub struct ProfileRename {}

/// Run `holler profile rename`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileRename, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
