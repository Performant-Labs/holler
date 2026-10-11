//! `holler pane ...` (epic #633): the verbs that change a pane. Holler is the only thing that does.
//!
//! **Frozen by story #670.** This file lists the verbs and dispatches them; it never changes
//! again. One verb is one file in this directory with its own clap `Args` struct and its own
//! `run`, owned by one story, so adding a verb's real behaviour edits that file and no other.
//! Until then each verb is a stub that refuses with `not implemented (story #NNN)`.
//!
//! - [`args`]: the flag groups the verbs share, defined once (`SpecFlags`, `ProfileOpt`, `SpecOnly`).
//! - [`wiring`]: the ports a verb runs against (story #649 builds the real ones).
//! - [`profile_scope`]: the helper every `--profile` verb uses (story #663).
//!
//! The verb modules are named after the verb; their `Args` structs are `Pane<Verb>`.

pub mod args;
pub mod close;
pub mod doctor;
pub mod get;
pub mod import;
pub mod launch;
pub mod list;
pub mod park;
pub mod profile_scope;
pub mod relaunch;
pub mod reset;
pub mod switch;
pub mod unpark;
pub mod watch;
pub mod wiring;

use clap::Subcommand;
use holler_pane::unmanaged;

use crate::output::{emit_error, ErrorBody, VerbCtx};

/// The `holler pane` verbs.
#[derive(Subcommand, Debug)]
pub enum PaneCmd {
    List(list::PaneList),
    Get(get::PaneGet),
    Watch(watch::PaneWatch),
    Launch(launch::PaneLaunch),
    Relaunch(relaunch::PaneRelaunch),
    Switch(switch::PaneSwitch),
    Reset(reset::PaneReset),
    Park(park::PanePark),
    Unpark(unpark::PaneUnpark),
    Close(close::PaneClose),
    Doctor(doctor::PaneDoctor),
    Import(import::PaneImport),
}

/// Whether `arg` could be a Herdr pane id rather than a pane name: it has a `:` and is not a
/// valid pane name (Herdr ids look like `w1:p3`).
pub(crate) fn is_herdr_pane_id_shaped(arg: &str) -> bool {
    arg.contains(':') && holler_pane::PaneName::parse(arg).is_err()
}

/// The pane a verb that **acts on** a pane was given, as typed; `None` for a verb that
/// reads, creates or adopts panes, and for an acting verb given no pane (every pane in scope).
fn acting_pane(cmd: &PaneCmd) -> Option<&str> {
    match cmd {
        PaneCmd::Relaunch(args) => Some(&args.pane),
        PaneCmd::Switch(args) => Some(&args.pane),
        PaneCmd::Reset(args) => Some(&args.pane),
        PaneCmd::Park(args) => args.pane.as_deref(),
        PaneCmd::Unpark(args) => args.pane.as_deref(),
        PaneCmd::Close(args) => args.pane.as_deref(),
        PaneCmd::List(_)
        | PaneCmd::Get(_)
        | PaneCmd::Watch(_)
        | PaneCmd::Launch(_)
        | PaneCmd::Doctor(_)
        | PaneCmd::Import(_) => None,
    }
}

/// Run a `holler pane` verb and return its exit code (0 ok, 1 failure, 2 usage, 3 refusal).
///
/// No verb that acts on a pane accepts an unmanaged one (story #832): a Herdr pane no record
/// names is visible, not drivable, so each such verb refuses it the same way
/// (`pane-unmanaged`, exit 3) instead of reporting a missing record. A verb does that where
/// it finds no record, through `unmanaged::missing`, so a pane that has a record costs no
/// extra read. The one case a verb cannot see is a Herdr pane **id** (`w1:p3`), which is not
/// a valid pane name and would be a `usage` error before any lookup: it is checked here.
pub fn run(cmd: &PaneCmd, ctx: &mut VerbCtx<'_>) -> i32 {
    if let Some(arg) = acting_pane(cmd).filter(|arg| is_herdr_pane_id_shaped(arg)) {
        if let Err(error) = unmanaged::refuse_if_unmanaged(ctx.ports, arg) {
            return emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error));
        }
    }
    match cmd {
        PaneCmd::List(args) => list::run(args, ctx),
        PaneCmd::Get(args) => get::run(args, ctx),
        PaneCmd::Watch(args) => watch::run(args, ctx),
        PaneCmd::Launch(args) => launch::run(args, ctx),
        PaneCmd::Relaunch(args) => relaunch::run(args, ctx),
        PaneCmd::Switch(args) => switch::run(args, ctx),
        PaneCmd::Reset(args) => reset::run(args, ctx),
        PaneCmd::Park(args) => park::run(args, ctx),
        PaneCmd::Unpark(args) => unpark::run(args, ctx),
        PaneCmd::Close(args) => close::run(args, ctx),
        PaneCmd::Doctor(args) => doctor::run(args, ctx),
        PaneCmd::Import(args) => import::run(args, ctx),
    }
}
