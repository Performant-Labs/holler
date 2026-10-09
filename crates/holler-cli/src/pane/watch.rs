//! `holler pane watch`: a stub (story #670). Story #643 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 643;

/// Stream pane changes.
#[derive(Args, Debug)]
pub struct PaneWatch {
    /// The one pane to watch.
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    /// Print the changes after this cursor.
    #[arg(long, value_name = "CURSOR")]
    pub since: Option<u64>,
    /// Stop when the stream is idle.
    #[arg(long)]
    pub until_idle: bool,
}

/// Run `holler pane watch`: refuse, naming the story that owns it.
pub fn run(_args: &PaneWatch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
