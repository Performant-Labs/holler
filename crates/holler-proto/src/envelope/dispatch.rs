//! The dispatch helper: project a call frame's raw `params` into the
//! method's typed params `T`.
//!
//! A peer's `params` is a raw [`Value`] (the envelope is method-agnostic);
//! the **dispatcher** is where per-method schema is validated (docs §8,
//! `invalid_params`). This helper gives the dispatcher a single typed entry
//! point for the request side so a method's params type is deserialised in
//! one place rather than re-stated in every arm of a giant `match`.

use serde_json::Value;

use crate::error::{Code, Error as WireError};

/// Decode a call frame's raw `params` (`&Value`, or `None` for a
/// parameterless method) into the method's typed params `T`.
///
/// A `params == None` decodes into `T` from an empty object — a method
/// with no required params deserialises from `{}`. A wire `params` that
/// is not the shape the method declares returns
/// [`Code::InvalidParams`] (docs §8) with the serde detail as the reason.
pub fn typed_params<T>(params: Option<&Value>) -> Result<T, WireError>
where
    T: serde::de::DeserializeOwned,
{
    // `WireError::new` takes a `&'static str` reason (the wire `data.reason`
    // is a fixed phrase per code); the serde detail is not carried here.
    let reason = || WireError::new(Code::InvalidParams, "invalid params", None);
    match params {
        Some(p) => serde_json::from_value(p.clone()).map_err(|_| reason()),
        None => {
            serde_json::from_value(Value::Object(serde_json::Map::new())).map_err(|_| reason())
        }
    }
}
