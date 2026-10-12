#![allow(clippy::unwrap_used)] // #667
#![allow(clippy::expect_used)] // #667
#![allow(clippy::panic)] // #667
#![allow(clippy::unreachable)] // #667
#![allow(dead_code)] // #667
//! The apply rehearsal (#667, epic #633): the real `holler` binary's `profile apply`, and the
//! `pane` and `profile` verbs around it, against a scratch Herdr, a private tmux server and
//! OpenCode with the dead-end provider only, all started by the test, through a real hub. It
//! checks the live result in Herdr's own snapshot, tmux and the registry, and backs step 0 of
//! the operator story #666. Declared by #670.
//!
//! **Two kinds of test.** The default run (CI) is pure pins (`pins`, and the scratch guards
//! this target shares with `pane_integration`, pinned in that target's `guards.rs`). The
//! scenario (`scenario`, steps 1 to 5 and 7 of the issue) and the probe test (`probe`, step 6)
//! are `#[ignore]` and run only with `HOLLER_TEST_PANE_INTEGRATION=1` exactly, one test at a
//! time (the scratch set plants a credential sentinel in this process's environment):
//!
//! ```text
//! HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1
//! ```
//!
//! Opted in, a missing `herdr`, `tmux`, `opencode` or `curl` is a failure, not a skip.
//!
//! **The scratch set is #649's**, included whole (`scratch`, with `out` at this crate's root,
//! as `scratch/mod.rs` needs it), so "649" is the family name of every scratch object here:
//! the Herdr session is `holler649-<8 hex>` and every pane is `hlr649-...`. Its guards refuse
//! any other session or pane name, and any `--herdr-session` but the scratch one.

#[path = "../support/mod.rs"]
mod support;

#[path = "../pane_integration/out.rs"]
mod out;

#[path = "../pane_integration/scratch/mod.rs"]
mod scratch;

mod pins;
mod plan;
mod probe;
mod scenario;

pub use out::{output_of, Out};
