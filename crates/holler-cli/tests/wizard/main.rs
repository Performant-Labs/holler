//! Tests of the setup wizard's helper scripts (`agent-skills/herdr-setup/lib/`), epic #726.
//!
//! One module per story, each in its own file and owned by that story alone:
//! `instance` (#727), `preflight` (#728), `start` (#729), `herdr` (#730), `ownership` (#731).
//! Run one with `cargo test -p holler-cli --test wizard_scripts <module>::`.

mod herdr;
mod instance;
mod ownership;
mod preflight;
mod proof;
mod skilltext;
mod skilltext_772b;
mod skilltext_772a;
mod skilltext_772c;
mod skilltext_w3_stage1;
mod skilltext_w3_stage8;
mod skilltext_w4_stage2;
mod skilltext_838;
mod skilltext_w3_stage3;
mod skilltext_w4_stage8;
mod skilltext_w5_basics;
mod skilltext_w5_workspace;
mod skilltext_w6_preflight;
mod skilltext_w5_stage0;
mod skilltext_w5_rename;
mod start;
