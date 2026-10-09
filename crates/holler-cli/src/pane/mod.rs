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

use crate::output::VerbCtx;

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

/// Run a `holler pane` verb and return its exit code (0 ok, 1 failure, 2 usage, 3 refusal).
pub fn run(cmd: &PaneCmd, ctx: &mut VerbCtx<'_>) -> i32 {
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
