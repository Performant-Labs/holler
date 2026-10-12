//! `holler_cli` — the argument-parsing half of the single `holler` binary.
//!
//! The clap derive tree here is the normative CLI surface from ADR 0003:
//! implement exactly it, no unlisted aliases. The `main.rs` binary is a
//! thin entry point over this tree.

mod cli;
pub mod answer_cmd;
pub mod attach_cmd;
pub mod body_cmd;
pub mod hold_cmd;
pub mod hub_cmd;
pub mod interrupt_cmd;
pub mod output;
pub mod pane;
pub mod profile;
pub mod prompt_target;
pub mod query_cmd;
pub mod roster_cmd;
pub mod say_cmd;
pub mod time_fmt;
pub mod token_cmd;
pub mod transport;
pub mod wait_cmd;

pub use crate::cli::{
    Answer, Attach, AttachCommand, Body, BodyCommand, Cli, Cmd, Command, Confirm, Delete, Hold, Hub,
    HubCommand, Init, Interrupt, Join, List, Mint, Ping, Query, QueryResolution, Release, Revoke, Roster,
    Run, Say, Serve, Sessions, Status, Support, Target, Token, TokenCommand, Usage, Wait,
};
// NOTE (story #144): the global `--debug` / `--log-format` values are
// captured on `Cli` as `Option<String>` (not typed enums) so the logging
// resolver in `holler_proto::log` can distinguish "the flag was given" (it
// beats the environment) from "absent" (env, then the none/text defaults).

/// What `holler --version` prints after `holler `: the workspace version and the commit the
/// binary was built from, for example `0.4.0 (bda1069)` (issue #851). The commit is the token
/// `build.rs` sets: the checkout's `HEAD`, with `-dirty` when tracked files had changed, else
/// `HOLLER_GIT_SHA`, else `unknown`. `main` installs the same string as
/// [`holler_proto::build_info::version`], which the `version` field of every status document
/// the hub and the body print carries.
pub const VERSION: &str =
    concat!(env!("CARGO_PKG_VERSION"), " (", env!("HOLLER_BUILD_COMMIT"), ")");
