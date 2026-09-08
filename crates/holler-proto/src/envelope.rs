//! The JSON-RPC 2.0 envelope (docs §2): four shapes — **request** (`id` +
//! `method`), **notification** (no `id`, `method`), **response** (`id` +
//! `result`), **error** (`id` + `error`). `params` / `result` stay a raw
//! [`Value`] — per-method schema is the dispatcher's concern (docs §8).
//!
//! `decode` (one `RawFrame` + one `match` over key presence) lives in
//! [`envelope::frame`], `encode` in [`envelope::encode`], and the dispatcher's
//! typed-params helper in [`envelope::dispatch`]. This file keeps the
//! [`Envelope`] enum + accessors/constructors and the [`EnvelopeError`] type.

use serde_json::Value;

use crate::error::{Code, Error as WireError};
use crate::id::CorrelationId;

mod dispatch;
mod encode;
pub mod frame;

#[cfg(test)]
mod ctor_tests;
#[cfg(test)]
mod dispatch_tests;

pub use dispatch::typed_params;
pub use encode::encode;
pub use frame::decode;

/// A decoded JSON-RPC 2.0 frame (one of four shapes).
#[derive(Debug, Clone, PartialEq)]
pub enum Envelope {
    /// A call that expects exactly one response carrying `id`.
    Request {
        /// Correlation id (string on the wire; `h-`/`b-` prefixed).
        id: String,
        /// The method name.
        method: String,
        /// Method parameters (raw; validated per-method by the dispatcher).
        params: Option<Value>,
    },
    /// A call with **no** id and **no** response.
    Notification {
        method: String,
        params: Option<Value>,
    },
    /// A successful result. `id` echoes the request.
    Response { id: String, result: Option<Value> },
    /// An error result. `id` echoes the request (or is absent for an unkeyed
    /// error such as a parse error).
    Error {
        id: Option<String>,
        error: WireError,
    },
}

impl Envelope {
    /// `true` for request and notification frames (both carry a method).
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, Envelope::Request { .. } | Envelope::Notification { .. })
    }
    /// The method name, for a call frame; `None` for response/error.
    pub fn method(&self) -> Option<&str> {
        match self {
            Envelope::Request { method, .. } | Envelope::Notification { method, .. } => {
                Some(method)
            }
            _ => None,
        }
    }
    /// The correlation id, for frames that carry one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Envelope::Request { id, .. } | Envelope::Response { id, .. } => Some(id),
            Envelope::Error { id, .. } => id.as_deref(),
            Envelope::Notification { .. } => None,
        }
    }
    /// The params value, for a call frame.
    pub fn params(&self) -> Option<&Value> {
        match self {
            Envelope::Request { params, .. } | Envelope::Notification { params, .. } => {
                params.as_ref()
            }
            _ => None,
        }
    }
    /// The error object, for an `Error` frame.
    pub fn error(&self) -> Option<&WireError> {
        match self {
            Envelope::Error { error, .. } => Some(error),
            _ => None,
        }
    }
    /// The response result, for a `Response` frame.
    pub fn result(&self) -> Option<&Value> {
        match self {
            Envelope::Response { result, .. } => result.as_ref(),
            _ => None,
        }
    }
    // Constructors: the variants have private fields on purpose — a `Envelope`
    // is only ever *built* by the codec (`decode`) or here, so an emitter can't
    // build a structurally-invalid frame.
    /// Build a **request** frame (the `id` is carried; a response is expected).
    pub fn request(id: &CorrelationId, method: impl Into<String>, params: Option<Value>) -> Self {
        Envelope::Request { id: id.to_string(), method: method.into(), params }
    }
    /// Build a **notification** frame (no `id`, no response expected).
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Envelope::Notification { method: method.into(), params }
    }
    /// Build a **response** frame echoing the request's `id`.
    pub fn response(id: &CorrelationId, result: Option<Value>) -> Self {
        Envelope::Response { id: id.to_string(), result }
    }
    /// Build an **error** frame echoing the request's `id` with a
    /// [`WireError`]. Named `error_frame` (not `error`) because the `Envelope`
    /// accessors already own the bare `error` name.
    pub fn error_frame(id: &CorrelationId, error: &WireError) -> Self {
        Envelope::Error { id: Some(id.to_string()), error: error.clone() }
    }
    /// The four shapes, as names (for tests and the golden-file test, #149).
    pub fn shape_name(&self) -> &'static str {
        match self {
            Envelope::Request { .. } => "request",
            Envelope::Notification { .. } => "notification",
            Envelope::Response { .. } => "response",
            Envelope::Error { .. } => "error",
        }
    }
}

/// Why a frame failed the envelope check (each variant's meaning is in the
/// [`Display`] impl below).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Not valid JSON at all.
    Json(String),
    /// A top-level JSON array (a batch) — v2 rejects all batches.
    Batch,
    /// Not exactly one of request / notification / response / error.
    Shape,
    /// The `jsonrpc` marker is missing or not `"2.0"`.
    Version,
    /// A call used a method not in the v2 catalog.
    UnknownMethod(String),
    /// A request (id present) used a method the catalog marks a notification.
    RequestButNotification(String),
    /// A notification (no id) used a method the catalog marks a request.
    NotificationButRequest(String),
    /// A call/response id is not a valid correlation id.
    BadId,
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::Json(msg) => write!(f, "frame is not valid JSON: {msg}"),
            EnvelopeError::Batch => f.write_str("batches are not supported in v2"),
            EnvelopeError::Shape => f.write_str("frame is not a valid JSON-RPC 2.0 message"),
            EnvelopeError::Version => f.write_str("jsonrpc must be \"2.0\""),
            EnvelopeError::UnknownMethod(m) => write!(f, "unknown method {m:?}"),
            EnvelopeError::RequestButNotification(m) => {
                write!(f, "method {m:?} is a notification; a request carries an id")
            }
            EnvelopeError::NotificationButRequest(m) => {
                write!(f, "method {m:?} is a request; a notification carries no id")
            }
            EnvelopeError::BadId => f.write_str("correlation id is missing its h-/b- prefix"),
        }
    }
}
impl std::error::Error for EnvelopeError {}

impl EnvelopeError {
    /// The JSON-RPC code this envelope failure maps to (docs §8).
    pub fn code(&self) -> Code {
        match self {
            EnvelopeError::Json(_) => Code::ParseError,
            EnvelopeError::Batch | EnvelopeError::Shape | EnvelopeError::Version => {
                Code::InvalidRequest
            }
            EnvelopeError::UnknownMethod(_) => Code::MethodNotFound,
            EnvelopeError::RequestButNotification(_) | EnvelopeError::NotificationButRequest(_) => {
                Code::InvalidRequest
            }
            EnvelopeError::BadId => Code::InvalidRequest,
        }
    }
}
