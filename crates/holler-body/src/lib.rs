//! `holler_body` — the body role of the single `holler` binary (ADR 0001).
//!
//! Story #176 lands the body's one-shot bootstrap over the wire: `body join`
//! (redeem a one-time join secret for a `client_id` + credential and persist
//! the identity), `body detach` (forget the identity), and `body status`
//! (report this process's own identity). The protocol/body talk stories fill
//! the rest in.

pub mod detach;
pub mod identity;
pub mod join;
pub mod server_address;
pub mod status;
