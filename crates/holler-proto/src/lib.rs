//! `holler_proto` — the v2 protocol core: types and codec only, no I/O.
//!
//! This crate is the Rust side of `docs/protocol/v2.md`. It owns:
//!
//! - [`a2a`] — the A2A 1.0 object model (`Message`, `Part`, `Role`,
//!   `TaskState`), reused verbatim on the inward wire (ADR 0004).
//! - [`envelope`] — the JSON-RPC 2.0 envelope and its codec (framing checks,
//!   no batches, `method_not_found`).
//! - [`id`] — correlation ids (`h-`/`b-` prefixed ULIDs; the two sides never
//!   collide).
//! - [`methods`] — the method catalog (13 methods, their kinds and
//!   directions).
//! - [`error`] — the error table (JSON-RPC codes + Holler `data.code`).
//! - [`docs`] — the per-method `params`/`result` types (hello, status,
//!   presence, prompt, …) and the `stop_reason` → A2A-state mapping.
//! - [`names`] — the `SessionName` grammar (ADR 0005) and the feature /
//!   harness id vocabulary.
//! - [`features`] — the single protocol version constant (ADR 0003).
//!
//! **This crate has no network, OS, or async dependency** — only
//! `serde`/`serde_json`. The hub and body both link it.

pub mod a2a;
pub mod docs;
pub mod envelope;
pub mod error;
pub mod features;
pub mod id;
pub mod methods;
pub mod names;

// Flat re-exports so the common path is `holler_proto::{...}`.
pub use a2a::{Message, Part, Role, TaskState};
pub use docs::{
    state_for_stop_reason, AuthOk, Authenticate, Cancel, CancelResult, Caps, ConfirmedHarness,
    Hello, HelloSession, Join, JoinResult, Presence, Prompt, PromptResult, ProtocolAnswer,
    ProtocolParams, SessionAd, Status, StatusSession, Support, SupportParams, Update,
    A2A_TERMINAL_STATES, STOP_TO_STATE,
};
pub use envelope::{decode, decode_call, encode, shape_name, Envelope, EnvelopeError};
pub use error::{Code, Error as WireError, ErrorDef, TABLE};
pub use features::{is_supported_version, PROTOCOL_MAX, PROTOCOL_MIN, PROTOCOL_VERSION};
pub use id::{CorrelationId, CorrelationIdError};
pub use methods::{find, is_notification, is_request, Direction, Method, MethodKind, CATALOG};
pub use names::{NameError, SessionName, Vocab, FEATURES, HARNESS_IDS};
