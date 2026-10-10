//! `holler profile delete`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
//!
//! T's signature stub (#662b test plan): the arguments are the verb's final ones, so the tests
//! parse; `run` still refuses until F replaces it.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Delete a profile.
#[derive(Args, Debug)]
pub struct ProfileDelete {
    /// The profile's name.
    pub name: String,
    /// First detach every member pane (its `profile` becomes none; the pane keeps running).
    #[arg(long)]
    pub keep_panes: bool,
}

/// Run `holler profile delete`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileDelete, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
