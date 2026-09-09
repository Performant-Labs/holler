//! `holler_cli` — the argument-parsing half of the single `holler` binary.
//!
//! The clap derive tree here is the normative CLI surface from ADR 0003:
//! implement exactly it, no unlisted aliases. The `main.rs` binary is a
//! thin entry point over this tree.

mod cli;
pub mod body_cmd;
pub mod hub_cmd;
pub mod interrupt_cmd;
pub mod query_cmd;
pub mod roster_cmd;
pub mod say_cmd;
pub mod time_fmt;
pub mod token_cmd;

pub use crate::cli::{
    Attach, AttachCommand, Body, BodyCommand, Cli, Cmd, Command, Delete, Hub, HubCommand,
    Interrupt, Join, List, Mint, Ping, Query, QueryResolution, Revoke, Roster, Run, Say, Serve,
    Status, Support, Target, Token, TokenCommand, Usage,
};
// NOTE (story #144): the global `--debug` / `--log-format` values are
// captured on `Cli` as `Option<String>` (not typed enums) so the logging
// resolver in `holler_proto::log` can distinguish "the flag was given" (it
// beats the environment) from "absent" (env, then the none/text defaults).
