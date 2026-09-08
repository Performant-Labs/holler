//! `holler_cli` — the argument-parsing half of the single `holler` binary.
//!
//! The clap derive tree here is the normative CLI surface from ADR 0003:
//! implement exactly it, no unlisted aliases. The `main.rs` binary is a
//! thin entry point over this tree.

mod cli;

pub use crate::cli::{
    Attach, AttachCommand, Body, BodyCommand, Cli, Command, Hub, HubCommand, Interrupt, Join, List,
    Mint, Ping, Query, Revoke, Roster, Run, Say, Serve, Status, Support, Token, TokenCommand,
};
