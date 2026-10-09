//! `holler pane doctor`: a stub (story #670). Story #647 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 647;

/// Check panes against Herdr, tmux and the harness and report what differs.
#[derive(Args, Debug)]
pub struct PaneDoctor {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane doctor`: refuse, naming the story that owns it.
pub fn run(_args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
