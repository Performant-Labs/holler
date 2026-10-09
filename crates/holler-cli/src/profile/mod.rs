//! `holler profile ...` (epic #633): a profile is a named set of pane specs, kept in the same
//! registry as the panes.
//!
//! **Frozen by story #670.** This file lists the verbs and dispatches them; it never changes
//! again. One verb is one file in this directory with its own clap `Args` struct and its own
//! `run`, owned by one story. Until then each verb is a stub that refuses with
//! `not implemented (story #NNN)`. No profile verb takes `--profile`: the profile is the verb's
//! own argument, so there is no `args.rs` here; `--take-over` is `apply`'s own flag.
//!
//! The verb modules are named after the verb; their `Args` structs are `Profile<Verb>`.
//! `rename`, `export` and `import` are proposed (#665) and wait for the operator to confirm them.

pub mod apply;
pub mod create;
pub mod delete;
pub mod export;
pub mod import;
pub mod list;
pub mod rename;
pub mod show;

use clap::Subcommand;

use crate::output::VerbCtx;

/// The `holler profile` verbs.
#[derive(Subcommand, Debug)]
pub enum ProfileCmd {
    Create(create::ProfileCreate),
    Delete(delete::ProfileDelete),
    List(list::ProfileList),
    Show(show::ProfileShow),
    Apply(apply::ProfileApply),
    Rename(rename::ProfileRename),
    Export(export::ProfileExport),
    Import(import::ProfileImport),
}

/// Run a `holler profile` verb and return its exit code (0 ok, 1 refused or failed, 2 usage).
pub fn run(cmd: &ProfileCmd, ctx: &mut VerbCtx<'_>) -> i32 {
    match cmd {
        ProfileCmd::Create(args) => create::run(args, ctx),
        ProfileCmd::Delete(args) => delete::run(args, ctx),
        ProfileCmd::List(args) => list::run(args, ctx),
        ProfileCmd::Show(args) => show::run(args, ctx),
        ProfileCmd::Apply(args) => apply::run(args, ctx),
        ProfileCmd::Rename(args) => rename::run(args, ctx),
        ProfileCmd::Export(args) => export::run(args, ctx),
        ProfileCmd::Import(args) => import::run(args, ctx),
    }
}
