//! `holler_hub` — the hub role of the single `holler` binary (ADR 0001).
//!
//! On the skeleton (story #127) this crate was empty. Story #143 lands the
//! hub's `serve` lifecycle: the loopback WebSocket listener (ADR 0006), the
//! per-state-dir instance lock, the Unix-domain control socket, and the
//! `control/status` answer that `holler hub status` reads.

pub mod auth_io;
pub mod conn;
pub mod control;
pub mod control_io;
pub mod connection;
pub mod handshake;
pub mod lockout;
pub mod registry;
pub mod serve;
pub mod state;
pub mod token;
pub mod wire_io;
