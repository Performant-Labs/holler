//! The JSON-RPC 2.0 envelope (docs §2).
//!
//! Four wire shapes: **request** (`id` present, `method`), **notification**
//! (no `id`, `method`), **response** (`id` present, `result`), **error**
//! (`id` present, `error`).
//!
//! `Envelope` is an *internal* type: `decode` validates the framing (JSON
//! parses, not a batch, exactly one shape, `jsonrpc == "2.0"`, the method is
//! in the catalog, and the id kind matches the method kind), and `encode`
//! writes the matching shape. **`params` / `result` stay a raw `Value`** here
//! — per-method schema is the dispatcher's concern (docs §8,
//! `invalid_params`), so this module stays method-agnostic.

use serde_json::Value;

use crate::error::{Code, Error as WireError};
use crate::id::CorrelationId;
use crate::methods::{is_notification, is_request};

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
        matches!(
            self,
            Envelope::Request { .. } | Envelope::Notification { .. }
        )
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

    // ------------------------------------------------------------------
    // Constructors
    //
    // The variants have private fields on purpose: a `Envelope` is only ever
    // *built* by the codec (`decode`) or by these constructors. The hub (and
    // any other emitter) uses these to mint outbound frames — it cannot build
    // a structurally-invalid frame (a notification with an id, a response
    // with no id, a double error) because the constructors enforce the shape.
    // ------------------------------------------------------------------

    /// Build a **request** frame (the `id` is carried; a response is expected).
    ///
    /// `method` is the catalog method name (e.g. `"control/status"`).
    /// `params` is `None` for parameterless requests.
    pub fn request(id: &CorrelationId, method: impl Into<String>, params: Option<Value>) -> Self {
        Envelope::Request {
            id: id.to_string(),
            method: method.into(),
            params,
        }
    }

    /// Build a **notification** frame (no `id`, no response expected).
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Envelope::Notification {
            method: method.into(),
            params,
        }
    }

    /// Build a **response** frame (result success) echoing the request's `id`.
    /// `result` is `Some` for a success payload.
    pub fn response(id: &CorrelationId, result: Option<Value>) -> Self {
        Envelope::Response {
            id: id.to_string(),
            result,
        }
    }

    /// Build an **error** frame echoing the request's `id` with a `WireError`.
    /// (The JSON-RPC "error object" shape: `id` present, `error` present, no
    /// `result`.) Named `error_frame` (not `error`) because the `Envelope`
    /// accessors already own the bare `error` name (the `error()` getter).
    pub fn error_frame(id: &CorrelationId, error: &WireError) -> Self {
        Envelope::Error {
            id: Some(id.to_string()),
            error: error.clone(),
        }
    }
}

/// Why a frame failed the envelope check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The frame is not valid JSON at all.
    Json(String),
    /// A batch (top-level JSON array) — v2 rejects all batches.
    Batch,
    /// A frame that is not exactly one of request / notification / response /
    /// error (e.g. an empty object, or one carrying both `result` and `error`).
    Shape,
    /// The `jsonrpc` marker is missing or not `"2.0"`.
    Version,
    /// A call used a method that is not in the v2 catalog. The offending
    /// method name is owned (it came off the wire).
    UnknownMethod(String),
    /// A request (id present) used a method the catalog marks a notification.
    RequestButNotification(String),
    /// A notification (no id) used a method the catalog marks a request.
    NotificationButRequest(String),
    /// A call/response id string is not a valid correlation id (no
    /// `h-`/`b-` prefix).
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

/// Validate and decode a single JSON-RPC 2.0 frame from a wire string.
///
/// The envelope checks, in order (docs §2 + §8): JSON parses → not a batch →
/// `jsonrpc == "2.0"` → exactly one shape → (for calls) the method is in the
/// catalog and the id kind matches the method kind.
pub fn decode(frame: &str) -> Result<Envelope, EnvelopeError> {
    let value: Value =
        serde_json::from_str(frame).map_err(|e| EnvelopeError::Json(e.to_string()))?;

    // No batches: a top-level array is a JSON-RPC batch (docs §1/§2).
    if value.is_array() {
        return Err(EnvelopeError::Batch);
    }
    let obj = match value {
        Value::Object(o) => o,
        _ => return Err(EnvelopeError::Shape),
    };

    // jsonrpc version.
    match obj.get("jsonrpc") {
        Some(Value::String(s)) if s == "2.0" => {}
        Some(_) => return Err(EnvelopeError::Version),
        None => return Err(EnvelopeError::Version),
    }

    let has_id = matches!(
        obj.get("id"),
        Some(Value::String(_)) | Some(Value::Number(_))
    );
    let has_method = obj.contains_key("method");
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");

    // Take the method as an owned `String` (a fresh allocation, no borrow of
    // `obj`) so the later `&'static str` catalog name never borrows a `&method`
    // that dies before the envelope is returned.
    let method = match obj.get("method") {
        Some(Value::String(m)) => m.clone(),
        Some(_) if has_method => return Err(EnvelopeError::Shape),
        _ => return response_or_error_or_shape(&obj, has_result, has_error),
    };

    // The method must be in the catalog.
    if crate::methods::find(&method).is_none() {
        return Err(EnvelopeError::UnknownMethod(method.clone()));
    }
    let params = obj.get("params").cloned();

    if has_id {
        // A request.
        if is_notification(&method) {
            return Err(EnvelopeError::RequestButNotification(method.clone()));
        }
        let id = match obj.get("id") {
            Some(Value::String(i)) => i.clone(),
            _ => return Err(EnvelopeError::BadId),
        };
        if CorrelationId::parse(&id).is_err() {
            return Err(EnvelopeError::BadId);
        }
        Ok(Envelope::Request {
            id,
            method: method.clone(),
            params,
        })
    } else {
        // A notification.
        if is_request(&method) {
            return Err(EnvelopeError::NotificationButRequest(method.clone()));
        }
        Ok(Envelope::Notification {
            method: method.clone(),
            params,
        })
    }
}

/// The response / error / shape arms of `decode`, split out so the call arms
/// can own `method` without the `Map` borrow extending across a later match.
fn response_or_error_or_shape(
    obj: &serde_json::Map<String, Value>,
    has_result: bool,
    has_error: bool,
) -> Result<Envelope, EnvelopeError> {
    if has_result {
        // A response.
        let id = match obj.get("id") {
            Some(Value::String(i)) => i.clone(),
            _ => return Err(EnvelopeError::BadId),
        };
        if CorrelationId::parse(&id).is_err() {
            return Err(EnvelopeError::BadId);
        }
        if has_error {
            return Err(EnvelopeError::Shape);
        }
        Ok(Envelope::Response {
            id,
            result: obj.get("result").cloned(),
        })
    } else if has_error {
        // An error. id may be absent (an unkeyed error, e.g. a parse error).
    // `has_error` was computed above as `obj.contains_key("error")`, so this
    // arm runs only when the key is present. A JSON object with the key set to
    // `null` (`"error": null`) is not a valid JSON-RPC error object, so
    // surfacing it as a framing error is the correct, spec-faithful behaviour —
    // there is no valid input that reaches this arm with `None`.
    #[allow(clippy::unreachable)] // #149: proven by the `has_error` gate above (see comment); a bare `unreachable!` is denied by the workspace lints
    let error: WireError = match obj.get("error") {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| EnvelopeError::Json(e.to_string()))?
        }
        None => return Err(EnvelopeError::Shape),
    };
        let id = match obj.get("id") {
            Some(Value::String(i)) => Some(i.clone()),
            Some(_) => return Err(EnvelopeError::BadId),
            None => None,
        };
        if let Some(id) = &id {
            if CorrelationId::parse(id).is_err() {
                return Err(EnvelopeError::BadId);
            }
        }
        Ok(Envelope::Error { id, error })
    } else {
        // No method, no result, no error.
        Err(EnvelopeError::Shape)
    }
}

/// Serialize an envelope back to its wire form (one JSON object, `jsonrpc`
/// first). The named entry for the "encode a frame" direction that pairs with
/// `decode`.
pub fn encode(env: &Envelope) -> Result<String, serde_json::Error> {
    let mut m: serde_json::Map<String, Value> = serde_json::Map::new();
    m.insert("jsonrpc".to_owned(), Value::from("2.0"));
    match env {
        Envelope::Request { id, method, params } => {
            m.insert("id".to_owned(), Value::from(id.clone()));
            m.insert("method".to_owned(), Value::from(method.clone()));
            if let Some(p) = params {
                m.insert("params".to_owned(), p.clone());
            }
        }
        Envelope::Notification { method, params } => {
            m.insert("method".to_owned(), Value::from(method.clone()));
            if let Some(p) = params {
                m.insert("params".to_owned(), p.clone());
            }
        }
        Envelope::Response { id, result } => {
            m.insert("id".to_owned(), Value::from(id.clone()));
            m.insert("result".to_owned(), result.clone().unwrap_or(Value::Null));
        }
        Envelope::Error { id, error } => {
            if let Some(i) = id {
                m.insert("id".to_owned(), Value::from(i.clone()));
            }
            let ev: Value = serde_json::to_value(error)?;
            m.insert("error".to_owned(), ev);
        }
    }
    serde_json::to_string(&Value::Object(m))
}

impl Envelope {
    /// Decode a call frame into a typed, catalog-checked tuple: the method,
    /// the params, and (for a request) the id. Convenience for dispatch.
    pub fn decode_call(&self) -> Result<(&str, Option<&Value>, Option<&str>), EnvelopeError> {
        match self {
            Envelope::Request { id, method, params } => {
                Ok((method, params.as_ref(), Some(id)))
            }
            Envelope::Notification { method, params } => {
                Ok((method, params.as_ref(), None))
            }
            _ => Err(EnvelopeError::Shape),
        }
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

#[cfg(test)]
mod ctor_tests {
    use super::*;
    use crate::error::{Code, Error as WireError};
    use serde_json::json;

    /// A well-formed `h-` id (the codec accepts any non-empty `h-`/`b-` prefix).
    fn id() -> CorrelationId {
        CorrelationId::parse("h-1TEST").expect("valid id")
    }

    #[test]
    fn request_ctor_roundtrips_and_shape() {
        let env = Envelope::request(&id(), "circuit/ping", Some(json!({"n":1})));
        assert!(matches!(env, Envelope::Request { .. }));
        assert!(env.is_call(), "a request is a call frame");
        // The `id` accessor and the encoded `id` field both carry the id.
        assert_eq!(env.id().unwrap(), "h-1TEST");
        let s = encode(&env).expect("encode");
        assert!(s.contains("\"method\":\"circuit/ping\""));
        // Re-decoding a request yields a request.
        let back = decode(&s).expect("decode");
        assert!(matches!(back, Envelope::Request { .. }));
    }

    #[test]
    fn notification_ctor_has_no_id() {
        let env = Envelope::notification("circuit/superseded", None);
        assert!(matches!(env, Envelope::Notification { .. }));
        assert!(env.id().is_none(), "a notification carries no id");
        assert!(env.is_call(), "a notification is a call frame");
    }

    #[test]
    fn response_ctor_carries_id_and_result() {
        let env = Envelope::response(&id(), Some(json!({"ok":true})));
        assert!(matches!(env, Envelope::Response { .. }));
        assert_eq!(env.id().unwrap(), "h-1TEST");
        assert!(env.result().is_some());
        assert!(env.error().is_none());
    }

    #[test]
    fn error_ctor_carries_id_and_error() {
        let err = WireError::new(
            Code::InvalidRequest,
            "bad request",
            Some("the id must be h-/b- prefixed"),
        );
        let env = Envelope::error_frame(&id(), &err);
        assert!(matches!(env, Envelope::Error { .. }));
        assert_eq!(env.id().unwrap(), "h-1TEST");
        assert_eq!(env.error().unwrap().code, Code::InvalidRequest);
        assert!(env.result().is_none());
    }
}
