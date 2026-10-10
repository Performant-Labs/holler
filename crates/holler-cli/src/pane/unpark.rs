//! `holler pane unpark [PANE] [--profile NAME]` (story #646, part 1): return parked panes to
//! service by setting the `hold` of their records back to `none`.
//!
//! It is park's run with the opposite change, through park's engine (`super::park`, whose
//! module doc has the rules): one compare-and-swap per pane, no adapter call and no profile
//! write. A pane that is not parked (its hold is `none`, or it is drained) is left as it is and
//! printed as `not parked`. With `--profile` and no pane name it takes every pane of the profile
//! in name order and stops at the first failed write, whose message ends `unparked by this run
//! before it: <names or none>; not reached: <names or none>`.

use clap::Args;

use super::args::ProfileOpt;
use super::park::{run_hold_change, HoldChange, HoldRequest, HoldTarget};
use crate::output::VerbCtx;

/// The verb, as its usage message names it.
const VERB: &str = "unpark";

/// Return a parked pane to service.
#[derive(Args, Debug)]
pub struct PaneUnpark {
    /// Unpark only this pane (default with `--profile`: every pane of the profile).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane unpark`: unpark the panes in scope and print what each one is now.
pub fn run(args: &PaneUnpark, ctx: &mut VerbCtx<'_>) -> i32 {
    let request = HoldTarget::parse(VERB, args.pane.as_deref(), args.profile.profile.as_deref())
        .map(|target| HoldRequest {
            target,
            change: HoldChange::Unpark,
        });
    run_hold_change(request, ctx)
}
