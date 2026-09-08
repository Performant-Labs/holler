//! `holler_hub` — the hub role of the single `holler` binary (ADR 0001).
//!
//! On the skeleton (story #127) this crate was empty. Story #143 lands the
//! hub's `serve` lifecycle: the loopback WebSocket listener (ADR 0006), the
//! per-state-dir instance lock, the Unix-domain control socket, and the
//! `control/status` answer that `holler hub status` reads.

pub mod control;
pub mod serve;
pub mod state;
pub mod token;
