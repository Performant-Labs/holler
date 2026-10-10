//! `holler pane launch PANE`: story #644. **RED stub (#644, T):** the verb's own positional and
//! `--herdr-session` are declared, and `run` still refuses with `not-implemented`; F fills it.

use clap::Args;

use super::args::{ProfileOpt, SpecFlags, SpecOnly};
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 644;

/// Create a pane.
#[derive(Args, Debug)]
pub struct PaneLaunch {
    /// The pane's name (also its tmux session name), e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// The Herdr session the pane lives in. Required unless --spec-only.
    #[arg(long, value_name = "NAME")]
    pub herdr_session: Option<String>,
    /// Boxed so `PaneCmd` (inside the frozen `Command`) stays under clippy's
    /// `large_enum_variant` bound now that the verb has its own positional.
    #[command(flatten)]
    pub spec: Box<SpecFlags>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane launch`. stub (#644 RED): F fills
pub fn run(_args: &PaneLaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
