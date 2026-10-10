//! `holler profile create`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
//!
//! T's signature stub (#662b test plan): the arguments are the verb's final ones, so the tests
//! parse; `run` still refuses until F replaces it.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Create a profile: empty, a detached copy of another (--from), or a snapshot of every pane (--from-current).
#[derive(Args, Debug)]
pub struct ProfileCreate {
    /// The profile's name (spaces allowed).
    pub name: String,
    /// Snapshot every pane in the registry into specs and make each pane a member.
    #[arg(long, conflicts_with = "from")]
    pub from_current: bool,
    /// Copy the specs of PROFILE; no pane joins (a detached copy).
    #[arg(long, value_name = "PROFILE")]
    pub from: Option<String>,
}

/// Run `holler profile create`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileCreate, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
