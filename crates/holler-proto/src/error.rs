//! The v2 error table (docs §8).
//!
//! JSON-RPC **error objects**: standard codes for framing
//! (`-32700`, `-32600`, `-32601`, `-32602`), application codes
//! `-32000..-32099` carrying `data.code` = the Holler string, and a
//! `reason` in `data.reason`.
//!
//! **`-32603` (Internal error) and `-32009` are reserved** and unused in v2.
//! An *unmatched response* (a response whose `id` matches no outstanding
//! request) has **no** JSON-RPC code in v2: the peer logs and discards it.

use serde::{Deserialize, Serialize};

/// One row of the error table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorDef {
    /// The JSON-RPC numeric `error.code`.
    pub jsonrpc_code: i64,
    /// The Holler string carried in `error.data.code`.
    pub data_code: &'static str,
    /// One-line description of when the error fires.
    pub when: &'static str,
}

/// The complete, closed v2 error table (13 rows).
#[rustfmt::skip]
pub const TABLE: &[ErrorDef] = &[
    // jsonrpc_code  data.code             when
    ErrorDef { jsonrpc_code: -32700, data_code: "parse_error",        when: "The frame is not JSON." },
    ErrorDef { jsonrpc_code: -32600, data_code: "invalid_request",    when: "A batch (array), a binary frame, or a missing/≠\"2.0\" jsonrpc field." },
    ErrorDef { jsonrpc_code: -32601, data_code: "method_not_found",   when: "Unknown method." },
    ErrorDef { jsonrpc_code: -32602, data_code: "invalid_params",     when: "Schema violation in params." },
    ErrorDef { jsonrpc_code: -32000, data_code: "unsupported_version",when: "hello.protocol ≠ 2. Socket closes; no silent downgrade." },
    ErrorDef { jsonrpc_code: -32001, data_code: "join_failed",        when: "circuit/join secret unknown/already-bound/invalidated/revoked/expired." },
    ErrorDef { jsonrpc_code: -32002, data_code: "unauthenticated",    when: "Bad or revoked credential; or a method sent before authenticate." },
    ErrorDef { jsonrpc_code: -32003, data_code: "unknown_session",    when: "prompt/cancel to a name not hosted here, or held by a different token." },
    ErrorDef { jsonrpc_code: -32004, data_code: "not_connected",      when: "A remote query/ping for a bound token with no live socket." },
    ErrorDef { jsonrpc_code: -32005, data_code: "session_superseded", when: "A new authenticate for the same token replaced this connection." },
    ErrorDef { jsonrpc_code: -32006, data_code: "unknown_feature",    when: "A query/support (or query/protocol version) id outside the vocabulary." },
    ErrorDef { jsonrpc_code: -32007, data_code: "limit_exceeded",     when: "A hub cap was hit (reserved; used by the caps story)." },
    ErrorDef { jsonrpc_code: -32008, data_code: "connection_lost",    when: "The body's socket dropped mid-turn (hub → CLI)." },
];

/// Look up a Holler `data.code` → its JSON-RPC code.
///
/// `None` for a `data.code` not in the table (which cannot be produced by the
/// v2 codec, since `Error::new` only ever returns a `Code` from this table).
pub fn jsonrpc_code_for(data_code: &str) -> Option<i64> {
    TABLE
        .iter()
        .find(|d| d.data_code == data_code)
        .map(|d| d.jsonrpc_code)
}

/// A JSON-RPC error code, paired with its Holler `data.code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    UnsupportedVersion,
    JoinFailed,
    Unauthenticated,
    UnknownSession,
    NotConnected,
    SessionSuperseded,
    UnknownFeature,
    LimitExceeded,
    ConnectionLost,
}

impl Code {
    /// The JSON-RPC numeric code for this Holler code.
    pub fn jsonrpc(self) -> i64 {
        TABLE[IDX[self as usize]].jsonrpc_code
    }

    /// The Holler string (`error.data.code`) for this code.
    pub fn data_code(self) -> &'static str {
        TABLE[IDX[self as usize]].data_code
    }
}

// Position of each Code in TABLE — keeps the mapping total and panic-free.
const IDX: [usize; 13] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/// The JSON-RPC `error` object (docs §8 / JSON-RPC 2.0 §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    /// The Holler error code.
    pub code: Code,
    /// A short, human-readable description.
    pub message: String,
    /// Optional structured data. v2 carries `{"code": <data_code>, "reason"?}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    /// Build an error with the conventional `data` object:
    /// `{"code": <data_code>}`, plus `{"reason": <reason>}` when `reason` is
    /// `Some`. `reason` is used, e.g., for `unknown_session` ("name held by
    /// another body") and `join_failed`.
    pub fn new(code: Code, message: impl Into<String>, reason: Option<&'static str>) -> Self {
        let mut obj = serde_json::Map::new();
        obj.insert("code".to_owned(), serde_json::json!(code.data_code()));
        if let Some(r) = reason {
            obj.insert("reason".to_owned(), serde_json::json!(r));
        }
        Self {
            code,
            message: message.into(),
            data: Some(serde_json::Value::Object(obj)),
        }
    }
}
