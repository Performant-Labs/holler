//! The v2 error table (docs §8).
//!
//! JSON-RPC **error objects**: standard codes for framing
//! (`-32700`, `-32600`, `-32601`, `-32602`), application codes
//! `-32000..-32099` carrying `data.code` = the Holler string, and a
//! `reason` in `data.reason`.
//!
//! **`-32603` (Internal error) is reserved** and unused in v2.
//! An *unmatched response* (a response whose `id` matches no outstanding
//! request) has **no** JSON-RPC code in v2: the peer logs and discards it.
//!
//! There is **one source of truth**: the [`Code`] enum. Each variant carries
//! its own JSON-RPC number ([`Code::jsonrpc`]) and its Holler string
//! ([`Code::data_code`]) in a single `match` each, so there is no parallel
//! table or identity array to drift out of order. (The old `TABLE` const and
//! the hand-maintained `IDX` identity array are gone, #145.)

use serde::{Deserialize, Serialize};

/// A JSON-RPC error code, the single source of truth for the v2 error table.
///
/// The enum is **not** serialised as a string: on the wire, an error object
/// carries the numeric JSON-RPC `code` (see [`Error`]); the Holler identity
/// travels as a string in `data.code`. This type is the in-process mapping
/// between the two, and a peer's numeric code is mapped back to a variant by
/// [`Code::from_jsonrpc`] (unknown numbers stay raw on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    SessionBusy,
}

impl Code {
    /// All the codes in the closed v2 table, in table order.
    ///
    /// The codec and tests iterate this to assert uniqueness and range over
    /// the one source of truth (there is no separate table to check).
    pub const ALL: [Code; 14] = [
        Code::ParseError,
        Code::InvalidRequest,
        Code::MethodNotFound,
        Code::InvalidParams,
        Code::UnsupportedVersion,
        Code::JoinFailed,
        Code::Unauthenticated,
        Code::UnknownSession,
        Code::NotConnected,
        Code::SessionSuperseded,
        Code::UnknownFeature,
        Code::LimitExceeded,
        Code::ConnectionLost,
        Code::SessionBusy,
    ];

    /// The JSON-RPC numeric code for this Holler code (docs §8).
    #[inline]
    pub const fn jsonrpc(self) -> i64 {
        match self {
            Code::ParseError => -32700,
            Code::InvalidRequest => -32600,
            Code::MethodNotFound => -32601,
            Code::InvalidParams => -32602,
            Code::UnsupportedVersion => -32000,
            Code::JoinFailed => -32001,
            Code::Unauthenticated => -32002,
            Code::UnknownSession => -32003,
            Code::NotConnected => -32004,
            Code::SessionSuperseded => -32005,
            Code::UnknownFeature => -32006,
            Code::LimitExceeded => -32007,
            Code::ConnectionLost => -32008,
            Code::SessionBusy => -32009,
        }
    }

    /// The Holler string (`error.data.code`) for this code.
    #[inline]
    pub const fn data_code(self) -> &'static str {
        match self {
            Code::ParseError => "parse_error",
            Code::InvalidRequest => "invalid_request",
            Code::MethodNotFound => "method_not_found",
            Code::InvalidParams => "invalid_params",
            Code::UnsupportedVersion => "unsupported_version",
            Code::JoinFailed => "join_failed",
            Code::Unauthenticated => "unauthenticated",
            Code::UnknownSession => "unknown_session",
            Code::NotConnected => "not_connected",
            Code::SessionSuperseded => "session_superseded",
            Code::UnknownFeature => "unknown_feature",
            Code::LimitExceeded => "limit_exceeded",
            Code::ConnectionLost => "connection_lost",
            Code::SessionBusy => "session_busy",
        }
    }

    /// Map a (possibly foreign) JSON-RPC numeric code back to the matching
    /// variant, or `None` if it is not in the v2 table. A peer that sends a
    /// number we do not recognise still decodes: the [`Error`] keeps the raw
    /// number in its `code` field; this is only the typed recovery.
    #[inline]
    pub fn from_jsonrpc(code: i64) -> Option<Code> {
        Code::ALL.iter().find(|c| c.jsonrpc() == code).copied()
    }
}

/// The `error.data` object on the wire: the Holler identity plus an optional
/// one-line reason. `code` is a **string** here (it is the `data.code`
/// string, not the JSON-RPC number — that number lives on [`Error::code`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorData {
    /// The Holler string, e.g. `"unauthenticated"`.
    pub code: String,
    /// Optional one-line reason (e.g. `unknown_session` "name held by
    /// another body", `join_failed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// `session_busy` only: the session's current A2A state (`working` or
    /// `stalled`) at refusal time, so the caller's hint names what it hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// `session_busy` only: milliseconds since the in-flight turn started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_age_ms: Option<u64>,
    /// `session_busy` only: milliseconds since the last `session/update`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update_age_ms: Option<u64>,
}

/// The JSON-RPC `error` object (docs §8 / JSON-RPC 2.0 §6).
///
/// `code` is the **numeric** JSON-RPC code (JSON-RPC 2.0 §5.1); the Holler
/// identity rides in `data.code` as a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    /// The JSON-RPC numeric code (e.g. `-32002`). A foreign/unknown number
    /// is kept as-is; see [`Code::from_jsonrpc`].
    pub code: i64,
    /// A short, human-readable description.
    pub message: String,
    /// Optional structured data: `{"code": <data_code>, "reason"?}`.
    ///
    /// Boxed (issue #150): `ErrorData` grew a `session_busy`-only trio of
    /// fields, which pushed `Result<T, WireError>` past clippy's
    /// `result_large_err` threshold everywhere `WireError` is the error type
    /// — a bare `Option<ErrorData>` would make every such `Result` this
    /// error's full size even on the common, data-less path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<ErrorData>>,
}

impl Error {
    /// Build a conformant error from a known [`Code`]: the numeric JSON-RPC
    /// code plus the conventional `data` object (`{"code": <data_code>}`, and
    /// `{"reason": <reason>}` when `reason` is `Some`).
    pub fn new(code: Code, message: impl Into<String>, reason: Option<&'static str>) -> Self {
        Self {
            code: code.jsonrpc(),
            message: message.into(),
            data: Some(Box::new(ErrorData {
                code: code.data_code().to_owned(),
                reason: reason.map(str::to_owned),
                state: None,
                turn_age_ms: None,
                last_update_age_ms: None,
            })),
        }
    }

    /// Build a `-32009 session_busy` refusal (issue #150): `say` refuses a
    /// `working`/`stalled` session unless `--queue` opts in. The ages are
    /// the caller's next-command hint — "how long has this been stuck".
    pub fn session_busy(state: impl Into<String>, turn_age_ms: u64, last_update_age_ms: u64) -> Self {
        Self {
            code: Code::SessionBusy.jsonrpc(),
            message: "session is busy".to_owned(),
            data: Some(Box::new(ErrorData {
                code: Code::SessionBusy.data_code().to_owned(),
                reason: None,
                state: Some(state.into()),
                turn_age_ms: Some(turn_age_ms),
                last_update_age_ms: Some(last_update_age_ms),
            })),
        }
    }
}

impl From<Code> for Error {
    /// The bare error for a code: numeric `code`, the Holler string as the
    /// message, and `data.code` set (no reason).
    fn from(code: Code) -> Self {
        Self {
            code: code.jsonrpc(),
            message: code.data_code().to_owned(),
            data: Some(Box::new(ErrorData {
                code: code.data_code().to_owned(),
                reason: None,
                state: None,
                turn_age_ms: None,
                last_update_age_ms: None,
            })),
        }
    }
}
