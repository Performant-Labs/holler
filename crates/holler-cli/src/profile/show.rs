//! `holler profile show`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
//!
//! SIGNATURE STUB (#662 T, RED): the verb's `Args` as the brief fixes them; `run` still
//! refuses. F replaces `run`.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Show a profile and where live panes differ from it.
#[derive(Args, Debug)]
pub struct ProfileShow {
    /// The profile's name (spaces allowed).
    pub name: String,
}

/// Run `holler profile show`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileShow, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
