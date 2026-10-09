//! The wire form of the hub's `pane/*` and `profile/*` control methods (epic #633,
//! decision 8 of #637): the reply type and the params structs, shared by the servers
//! (#639, #661) and the client (#649).
//!
//! **Two error conventions, one split.** The v2 wire protocol reports a refusal as a
//! JSON-RPC error frame from the closed `holler_proto::Code` table. The 22
//! kebab-case pane codes do not fit that table, so these methods answer with a
//! [`PaneReply`] as a JSON-RPC **result** instead (ADR-0021, #634, ratifies this):
//!
//! - framing errors and an unknown method stay JSON-RPC errors (the hub's existing
//!   `method_not_found` and friends);
//! - every outcome of a request that reaches a `pane/*` or `profile/*` handler is a
//!   `PaneReply`, including params that do not decode ([`decode_params`]).
//!
//! `PaneReply` is not the CLI's `--format=json` envelope and carries no
//! `schema_version`. `*/watch` are long-poll: one [`WatchReply`] per request.
//!
//! The `data` of `pane/get`, `list`, `cas_put`, `delete` and of `profile/get`, `list`,
//! `cas_put`, `delete`, `log` and `rename` is not given a type here; the server stories
//! (#639, #661) and the client (#649) agree on it against ADR-0021.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PaneError;
use crate::pane::{Pane, PaneName};
use crate::ports::Cursor;
use crate::profile::{Actor, Profile, ProfileName};

/// The error half of a [`PaneReply`].
///
/// `code` is a closed code or an open one; `message` is the human text
/// (`PaneError`'s `Display`); `detail` is the single payload string of the closed
/// variants that carry one (a `Timeout`'s `op`, a `PaneNotFound`'s `what`, ...), so the
/// client can rebuild the exact error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// The reply to a `pane/*` or `profile/*` request: `{"ok": bool, "data": ..., "error":
/// {code, message, detail?}}`.
///
/// Unknown fields are ignored on read: a client never writes a reply back, so there
/// is nothing to lose (the records inside `data` are stricter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneReply {
    pub ok: bool,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub error: Option<ReplyError>,
}

impl PaneReply {
    /// A successful reply carrying `data`.
    pub fn success(data: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// A failed reply: the error's code, its message and, for a closed variant that
    /// carries one, its payload.
    pub fn failure(error: &PaneError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ReplyError {
                code: error.code().to_owned(),
                message: error.to_string(),
                detail: error.detail().map(str::to_owned),
            }),
        }
    }

    /// The reply as a `Result`: the data of a success, or the error of a failure.
    ///
    /// The parse-back rule: a closed code maps to its own [`PaneError`] variant,
    /// any other well-formed code to [`PaneError::Refused`], and a code that is not
    /// well-formed to [`PaneError::Unavailable`]. A reply whose `ok` and `error`
    /// disagree is malformed and is also `unavailable`.
    ///
    /// A closed variant that carries a payload takes it from `detail`, or from the
    /// whole `message` when there is none; a closed variant without a payload (such
    /// as `not-implemented` or `generation-conflict`) is rebuilt from the code alone,
    /// and the peer's message text is not kept.
    pub fn into_result(self) -> Result<Option<Value>, PaneError> {
        match (self.ok, self.error) {
            (true, None) => Ok(self.data),
            (false, Some(e)) => Err(PaneError::from_wire(e.code, e.message, e.detail)),
            (true, Some(_)) | (false, None) => Err(PaneError::Unavailable {
                what: "the reply is malformed: `ok` and `error` disagree".to_owned(),
            }),
        }
    }
}

/// Decode the `params` of a request into `T`, the one way a handler does it.
///
/// A guard of this crate that refuses its input (a bare-string command, an env entry
/// with a value, a zero grid cell, ...) maps to its own code; any other failure to
/// decode is `usage`. The handler answers the error with [`PaneReply::failure`].
///
/// A request that carries no `params` decodes from `{}`, as
/// `holler_proto::typed_params` does: the caller passes an empty JSON object for it,
/// so `pane/list`, `profile/list` and the `*/watch` methods (whose `since` defaults
/// to `Cursor(0)`) accept a request without `params`.
pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError> {
    serde_json::from_value(params).map_err(|e| PaneError::from_decode(&e))
}

/// Params of `pane/get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneGetParams {
    pub name: PaneName,
}

/// Params of `pane/cas_put`: the record, and the generation the caller read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneCasPutParams {
    pub pane: Pane,
    pub expected_generation: u64,
}

/// Params of `pane/delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneDeleteParams {
    pub name: PaneName,
    pub expected_generation: u64,
}

/// Params of `pane/watch` and `profile/watch`: resume after `since` (0, the default,
/// is from the beginning).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchParams {
    #[serde(default)]
    pub since: Cursor,
}

/// The `data` of a `pane/watch` or `profile/watch` reply: the batch of events the
/// long-poll collected, and the cursor to resume from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchReply<E> {
    pub events: Vec<E>,
    pub cursor: Cursor,
}

/// Params of `profile/get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileGetParams {
    pub name: ProfileName,
}

/// Params of `profile/cas_put`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCasPutParams {
    pub profile: Profile,
    pub expected_generation: u64,
    pub actor: Actor,
}

/// Params of `profile/delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDeleteParams {
    pub name: ProfileName,
    pub expected_generation: u64,
    pub actor: Actor,
}

/// Params of `profile/log`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileLogParams {
    pub name: ProfileName,
}

/// Params of `profile/rename` (PROPOSED, #665).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRenameParams {
    pub from: ProfileName,
    pub to: ProfileName,
    pub expected_generation: u64,
    pub actor: Actor,
}
