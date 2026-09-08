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
//! - [`vocab`] — the name kinds (`Label`, `SessionName`, `RoutableName`; ADR
//!   0005) and the feature / harness id vocabulary.
//! - [`version`] — the single protocol version constant (ADR 0003).
//! - [`log`] — debug logging for both roles (story #144): the
//!   `--debug`/`HOLLER_DEBUG` and `--log-format`/`HOLLER_LOG_FORMAT` dials,
//!   severity, components, secret redaction, and the stderr banner.
//!
//! **This crate has no network or async dependency** — only
//! `serde`/`serde_json` (plus `time` for the log emission timestamp). The
//! hub and body both link it.

pub mod a2a;
pub mod docs;
pub mod envelope;
pub mod error;
pub mod id;
pub mod log;
pub mod methods;
pub mod token;
pub mod version;
pub mod vocab;

// Flat re-exports so the common path is `holler_proto::{...}`.
pub use a2a::{Content, Message, Part, Role, TaskState};
pub use docs::{
    state_for_stop_reason, AuthOk, Authenticate, Cancel, CancelResult, Caps, ConfirmedHarness,
    Hello, HelloRole, HelloSession, Join, JoinResult, Mode, Presence, Prompt, PromptResult,
    ProtocolAnswer, ProtocolParams, SessionAd, SessionState, Status, StatusSession, Support,
    SupportKind, SupportParams, Update, A2A_TERMINAL_STATES, STOP_TO_STATE,
};
pub use envelope::{decode, encode, Envelope, EnvelopeError, typed_params};
pub use error::{Code, Error as WireError, ErrorData};
pub use id::{CorrelationId, CorrelationIdError};
pub use token::RedeemError;
pub use token::TokenError;
pub use log::{
    emit, emit_banner, get, init, key_is_secret, redact, redact_frame, resolve, value_is_secret,
    Component, Config, DebugLevel, Direction as LogDirection, Event, LogFormat, REDACTED, Severity,
};
pub use methods::{find, is_notification, is_request, Direction, Method, MethodKind, CATALOG};
pub use version::{is_supported_version, PROTOCOL_MAX, PROTOCOL_MIN, PROTOCOL_VERSION};
pub use vocab::{
    Label, NameError, RoutableName, SessionName, Vocab, FEATURES, HARNESS_IDS, MAX_SEGMENT_LEN,
};
