#![allow(clippy::unwrap_used)] // #831
#![allow(clippy::expect_used)] // #831
#![allow(clippy::panic)] // #831
#![allow(dead_code)] // #831
//! In-process tests of a Claude Code pane under the `holler pane` and `holler profile` verbs
//! (#831, ADR-0021 section 15), over the test kit's fakes with `FakeHarnesses` as the harness
//! port. A target of its own, so the `pane_verbs` and `profile_verbs` targets build without the
//! Claude harness while the endpoint change lands first (plan review, notes for F, 3).
//!
//! - `rig`: the fakes, the linked host and the seeds;
//! - `spec_flags`: `--harness` among the spec flags;
//! - `launch`: `pane launch --harness claude` and its plan's refusals (W10, W11, W13);
//! - `lifecycle`: relaunch, switch, reset, interrupt, answer, park and close (W1, W4, the
//!   operator's abort decision);
//! - `doctor`: the single-process chain, the relaunch repair and the new findings (W2, W3);
//! - `mixed`: one profile with both harnesses, and the acceptance scenario, its `say` step
//!   ignored and naming #839 and #850 (item 18);
//! - `driven`: a Claude Code pane asks the hub nothing and keeps its stored DRIVEN (#839);
//! - `compact`: `pane compact` on a Claude Code pane is `compact-unsupported` after the engine's
//!   checks, and on an OpenCode pane it compacts through the router (#878).

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod compact;
mod doctor;
mod driven;
mod launch;
mod lifecycle;
mod mixed;
mod rig;
mod spec_flags;
