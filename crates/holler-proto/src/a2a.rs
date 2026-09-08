//! The A2A 1.0 object model used by `session/prompt` (ADR 0004).
//!
//! Holler reuses A2A's JSON binding **verbatim** for the two A2A-bearing
//! methods: `session/prompt.message` (inward, `role:"user"`) and
//! `session/prompt`'s `result.message` (outward, `role:"agent"`). The wire
//! shapes here are the A2A v1.0 spec's JSON schema, so an Holler `Message` /
//! `Part` is byte-identical to an A2A `Message` / `Part`.
//!
//! A2A's JSON binding serialises enums as their **protocol-constant strings**
//! (e.g. `"ROLE_USER"`, `"TASK_STATE_WORKING"`), not the Rust type names.
//!
//! The types are split across submodules: the enums live in [`a2a::enums`],
//! the part (and its flat wire form) in [`a2a::part`]; this file keeps the
//! `Message` and re-exports everything at the crate root.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod enums;
pub mod part;

pub use enums::{Role, TaskState};
pub use part::{Content, Part};

/// A single A2A message — a list of parts plus sender/task context.
///
/// Field names here are the A2A JSON keys (`messageId`, `contextId`,
/// `taskId`, `referenceTaskIds`). The enum types are `SCREAMING_SNAKE_CASE`
/// on the wire; the *struct* field names are camelCase to match A2A's JSON
/// binding exactly (this is what lets a Holler `Message` be byte-identical to
/// an A2A `Message` when re-serialised).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    // `skip_serializing_if` on every `Option`: an unset field is absent from
    // the wire (A2A's JSON binding omits unset members), which is what keeps
    // a re-encoded `Message` byte-identical to the fixture it came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub role: Role,
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
    // A2A's `Message.extensions` is a list of extension URIs (the per-URI
    // payload lives in `metadata`, keyed by the same URI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<String>>,
}
