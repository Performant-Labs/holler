#![allow(clippy::unwrap_used)] // #670
#![allow(clippy::expect_used)] // #670
#![allow(clippy::panic)] // #670
#![allow(clippy::unreachable)] // #670
#![allow(dead_code)] // #670
//! In-process tests of the `holler profile` verbs (story #670, epic #633).
//!
//! One module per verb, so a verb story edits only `profile_verbs/<verb>.rs` and never
//! this file or the manifest. The harness that runs a verb over captured writers is
//! shared with `pane_verbs` (`tests/verb_harness/mod.rs`). The output API and the
//! shared flag validation are tested once, in `pane_verbs`.

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod apply;
mod create;
mod delete;
mod export;
mod import;
mod list;
mod rename;
mod show;
