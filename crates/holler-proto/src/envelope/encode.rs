//! The `encode` half of the codec: serialise an [`Envelope`] back to its
//! wire form.

use serde_json::Value;

use super::Envelope;

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
