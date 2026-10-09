//! `holler_adapter_herdr` — the Herdr adapter: it implements `holler_pane::HerdrPort`
//! over Herdr's local socket, and is the only place that converts a `GridPos` to
//! Herdr's own order and base (epic #633).
//!
//! Part 1 of story #640 is the pure core, with no I/O:
//!
//! - [`layout`] — Herdr's split tree and the one conversion from it to a `GridPos`.
//! - [`plan`] — the right/down splits that reach a cell, or `grid-unreachable`.
//! - [`protocol`] — Herdr's wire: requests, reply decoding, the version gate.
//!
//! Nothing is re-exported flat: `layout::Direction` and `protocol::SessionState` would
//! share a root name with `holler_proto`'s (A, W-1), so every item is reached by its
//! module path, as in `holler_pane_testkit`.

pub mod layout;
pub mod plan;
pub mod protocol;
