//! Tests of the setup wizard's helper scripts (`agent-skills/setup-wizard/lib/`), epic #726.
//!
//! One module per story, each in its own file and owned by that story alone:
//! `instance` (#727), `preflight` (#728), `start` (#729), `herdr` (#730), `ownership` (#731).
//! Run one with `cargo test -p holler-cli --test wizard_scripts <module>::`.

mod herdr;
mod instance;
mod ownership;
mod preflight;
mod proof;
mod start;
