//! `holler pane doctor`: a stub (story #670). Story #647 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
//!
//! RED scaffold (#647 test plan): the arguments of the brief's Decision 11 are declared so
//! the RED suite parses them; `run` still refuses until F fills it.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 647;

/// Check panes against Herdr, tmux and the harness and report what differs.
#[derive(Args, Debug)]
pub struct PaneDoctor {
    /// Check only this pane (default: every pane in scope).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    /// Repair what the record decides (selects the session of record; never changes it).
    #[arg(long)]
    pub fix: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane doctor`: refuse, naming the story that owns it.
pub fn run(_args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
