//! The A2A v1.0 enums: sender/role and task state.
//!
//! A2A's JSON binding serialises enums as their **protocol-constant strings**
//! (e.g. `"ROLE_USER"`, `"TASK_STATE_WORKING"`), not the Rust type names.
//! Both enums here use `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` to
//! match that exactly.

use serde::{Deserialize, Serialize};

/// A2A's sender/role enum, as A2A's JSON binding encodes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    /// The human user (an inward `prompt.message`).
    RoleUser,
    /// The agent (an outward reply / an `update` part).
    RoleAgent,
    /// A2A's default (unset) role.
    RoleUnspecified,
}

/// A2A task state (ADR 0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    TaskStateUnspecified,
    TaskStateSubmitted,
    TaskStateWorking,
    TaskStateInputRequired,
    TaskStateCompleted,
    TaskStateCanceled,
    TaskStateFailed,
    TaskStateRejected,
}
