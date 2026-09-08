//! The `decode` half of the codec: validate and classify a wire frame.
//!
//! `decode` decodes into one private [`RawFrame`] struct (a single serde
//! deserialization, one `match`) rather than reaching into the raw `Map`
//! field-by-field. It classifies by **key presence** for `method` / `id` /
//! `result` / `error` (not `Option::is_some` — a wire `null` deserialises to
//! `None`, so a `result: null` response is indistinguishable from an absent
//! `result` via the `Option`s; key presence is the only signal that separates
//! them). The envelope checks run in order (docs §2 + §8): JSON parses → not a
//! batch → `jsonrpc == "2.0"` → exactly one shape → (for calls) the method is
//! in the catalog and the id kind matches the method kind.

use serde::Deserialize;
use serde_json::Value;

use crate::error::Error as WireError;
use crate::id::CorrelationId;
use crate::methods::{is_notification, is_request};

use super::{Envelope, EnvelopeError};

/// The raw wire fields of a JSON-RPC 2.0 frame, decoded in one pass.
///
/// Every member is optional (absent on the wire → `None`); the **shape** is
/// derived from which keys are present (request = `id`+`method`, notification
/// = `method`, response = `id`+`result`, error = `id`+`error`). `id` is a
/// `Value` so a numeric id (invalid in v2) is *captured* rather than failing
/// the parse — the `match` in [`decode`] then rejects it as
/// [`EnvelopeError::BadId`] instead of a `Json` parse error. `error` is a
/// `Value` for the same reason (the `match` decodes it into a
/// [`WireError`]).
///
/// Note: this struct's `Option` fields cannot see a wire `null` (serde_json
/// maps a `null` member to `None`), so the shape is *not* derived from
/// `is_some` here — [`decode`] classifies by **key presence** in the raw
/// map instead (a `result: null` is a valid response whose `result` key is
/// present). `jsonrpc` is validated directly off the map by `decode` (its
/// value is fixed by the spec), not read through this struct.
#[derive(Debug, Deserialize)]
struct RawFrame {
    id: Option<Value>,
    method: Option<Value>,
    params: Option<Value>,
    result: Option<Value>,
    error: Option<Value>,
}

/// Validate and decode a single JSON-RPC 2.0 frame from a wire string.
pub fn decode(frame: &str) -> Result<Envelope, EnvelopeError> {
    // A top-level array is a batch (docs §1/§2) — reject before parsing.
    let trimmed = frame.trim_start();
    if trimmed.starts_with('[') {
        return Err(EnvelopeError::Batch);
    }
    // Read the frame's raw members (one serde pass). The **shape** is then
    // classified off *key presence* in the map, not off `Option::is_some`:
    // serde_json maps a wire `null` to `None`, so a `result: null` (a *valid*
    // JSON-RPC response) is indistinguishable from an absent `result` via the
    // [`RawFrame`] `Option`s. Key presence is the only signal that separates
    // them (docs §2: a response carries `id`+`result`, even if null).
    let mut frame: serde_json::Map<String, Value> =
        serde_json::from_str(frame).map_err(|e| EnvelopeError::Json(e.to_string()))?;

    // jsonrpc version.
    match frame.get("jsonrpc").and_then(Value::as_str) {
        Some("2.0") => {}
        _ => return Err(EnvelopeError::Version),
    }
    // A frame with both `result` and `error` is not exactly one shape.
    if frame.contains_key("result") && frame.contains_key("error") {
        return Err(EnvelopeError::Shape);
    }

    // Key presence for the shape `match` (a present-but-null member is still
    // `true`; a numeric/non-string `id` is still "present" and rejected as
    // `BadId` downstream). Snapshotted *before* the `take` below, which
    // moves the map's entries out (so the map is empty and `is_some` would
    // otherwise report false for every member).
    let has_id = frame.contains_key("id");
    let has_result = frame.contains_key("result");
    let has_error = frame.contains_key("error");

    // The wire fields (serde reads them from the map into one struct).
    let raw: RawFrame = serde_json::from_value(Value::Object(std::mem::take(&mut frame)))
        .map_err(|e| EnvelopeError::Json(e.to_string()))?;

    match (
        raw.method.as_ref(),
        has_id,
        has_result,
        has_error,
    ) {
        // A call (method present): a request if it carries an id, else a
        // notification. `params` is read off `raw` (serde filled it from the
        // wire `params` member).
        (Some(Value::String(method)), true, ..) => {
            decode_request(method, raw.id.as_ref(), raw.params.as_ref())
        }
        (Some(Value::String(method)), false, ..) => {
            decode_notification(method, raw.params.as_ref())
        }
        // A `method` that is not a string (including a present `null`) is a
        // framing error.
        (Some(_), ..) => Err(EnvelopeError::Shape),
        // A response (result key present, no method) — a `result: null` is a
        // valid response; `has_result` (key presence) says so even though the
        // deserialised `raw.result` is `None` for a null.
        (None, _, true, false) => decode_response(raw.id.as_ref(), raw.result),
        // An error (error key present, no method, no result).
        (None, _, false, true) => decode_error(raw.error.as_ref(), raw.id.as_ref()),
        // No method, no result, no error (an empty object, a stray member).
        (None, ..) => Err(EnvelopeError::Shape),
    }
}

/// The request arm of `decode` (a call that carries an id).
fn decode_request(
    method: &str,
    id: Option<&Value>,
    params: Option<&Value>,
) -> Result<Envelope, EnvelopeError> {
    // The method must be in the catalog.
    if crate::methods::find(method).is_none() {
        return Err(EnvelopeError::UnknownMethod(method.to_owned()));
    }
    // A request is a method the catalog does *not* mark a notification.
    if is_notification(method) {
        return Err(EnvelopeError::RequestButNotification(method.to_owned()));
    }
    // The id must be a string (a numeric id is rejected, #145) and must be a
    // valid correlation id.
    let id = match id {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(EnvelopeError::BadId),
    };
    if CorrelationId::parse(&id).is_err() {
        return Err(EnvelopeError::BadId);
    }
    Ok(Envelope::Request { id, method: method.to_owned(), params: params.cloned() })
}

/// The notification arm of `decode` (a call without an id).
fn decode_notification(
    method: &str,
    params: Option<&Value>,
) -> Result<Envelope, EnvelopeError> {
    // The method must be in the catalog.
    if crate::methods::find(method).is_none() {
        return Err(EnvelopeError::UnknownMethod(method.to_owned()));
    }
    // A notification is a method the catalog does *not* mark a request.
    if is_request(method) {
        return Err(EnvelopeError::NotificationButRequest(method.to_owned()));
    }
    Ok(Envelope::Notification { method: method.to_owned(), params: params.cloned() })
}

/// The response arm of `decode` (`id` present, `result` present, no method).
fn decode_response(id: Option<&Value>, result: Option<Value>) -> Result<Envelope, EnvelopeError> {
    // A response carries exactly one id.
    let id = match id {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(EnvelopeError::BadId),
    };
    if CorrelationId::parse(&id).is_err() {
        return Err(EnvelopeError::BadId);
    }
    Ok(Envelope::Response { id, result })
}

/// The error arm of `decode` (`error` present, no method, no result).
fn decode_error(error: Option<&Value>, id: Option<&Value>) -> Result<Envelope, EnvelopeError> {
    // A present `error` must decode into a JSON-RPC error object.
    let error: WireError = match error {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| EnvelopeError::Json(e.to_string()))?
        }
        None => return Err(EnvelopeError::Shape),
    };
    // id may be absent (an unkeyed error, e.g. a parse error). A present id
    // must be a string (a numeric one is rejected, #145).
    let id = match id {
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err(EnvelopeError::BadId),
        None => None,
    };
    if let Some(id) = &id {
        if CorrelationId::parse(id).is_err() {
            return Err(EnvelopeError::BadId);
        }
    }
    Ok(Envelope::Error { id, error })
}
