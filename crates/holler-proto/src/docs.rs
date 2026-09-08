//! The per-method `params` / `result` types (docs §3–7).
//!
//! The envelope crate parses `params` as a raw `Value`; **this module is where
//! each method's schema is enforced** (the `invalid_params` path, docs §8).
//!
//! Two kinds of document, matching the body/hub endpoints:
//! - **hello** — the capabilities document sent after authenticate.
//! - **status** — the document returned by `query/status` (and used as the
//!   base for `query/caps`).
//!
//! The **presence** notification and the **prompt** request/result are the
//! A2A-bearing documents: `session/prompt.message` is an A2A `Message`
//! (inward `role:"user"`, outward `role:"agent"`), and a turn ends in one of
//! the A2A terminal states (the `stop_reason` → `state` mapping, ADR 0004).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::a2a::{Message, Part};

// ---------------------------------------------------------------------------
// hello
// ---------------------------------------------------------------------------

/// A `circuit/hello` document. The hub's and a body's differ only by `role`
/// and the two "what do you have" fields: a **body** lists `harnesses` +
/// `sessions`; a **hub** lists `harnesses_known` + `harnesses_confirmed`.
///
/// `protocol` must equal `2` or the hub answers `-32000 unsupported_version`
/// and closes the socket (no silent downgrade).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    /// The protocol version this endpoint speaks. Must be `2` in v2.
    pub protocol: u32,
    /// The lowest protocol version this endpoint can also speak.
    pub protocol_min: u32,
    /// The highest protocol version this endpoint can also speak.
    pub protocol_max: u32,
    /// `"body"` or `"hub"`.
    pub role: String,
    /// The hostname / label this endpoint is reachable by.
    pub hostname: String,
    /// Present on a **body** hello: the short token id (ADR 0006).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    /// Present on a **body** hello: the long-lived client id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// The protocol features this endpoint advertises (docs §9).
    #[serde(default)]
    pub features: Vec<String>,
    /// **Body**: the harnesses this box can spawn/attach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses: Option<Vec<String>>,
    /// **Hub**: harness ids seen in some body's advertisement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses_known: Option<Vec<String>>,
    /// **Hub**: harness ids a live body confirmed with `query/support ok:true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses_confirmed: Option<Vec<String>>,
    /// **Body**: the sessions this body hosts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<HelloSession>>,
}

/// One session advertised in a **body**'s hello.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloSession {
    /// The session name (`<session>` or `<label>/<session>`).
    pub name: String,
    /// The harness that hosts this session.
    pub harness: String,
    /// `"spawn"` or `"attach"`.
    pub mode: String,
    /// The existing harness session being attached to (attach mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// status / caps
// ---------------------------------------------------------------------------

/// A `query/status` answer (and the base for a `query/caps` answer). One
/// shape, two `role` values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub role: String,
    pub protocol: u32,
    pub protocol_min: u32,
    pub protocol_max: u32,
    pub hostname: String,
    /// **Body**: whether this body currently has a live socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// **Body**: the short token id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    /// **Hub**: the listening address (e.g. `ws://127.0.0.1:41807`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listening: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    /// **Body**: the harnesses this box implements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses: Option<Vec<String>>,
    /// **Hub**: harness ids seen in some body's advertisement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses_known: Option<Vec<String>>,
    /// **Hub**: `{id, bodies:[...]}` — confirmed by a live body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnesses_confirmed: Option<Vec<ConfirmedHarness>>,
    /// **Hub**: number of currently-connected bodies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bodies: Option<u32>,
    /// **Hub**: total number of sessions across bodies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<u32>,
    /// **Body**: the sessions this body hosts, with their live state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_list: Option<Vec<StatusSession>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfirmedHarness {
    pub id: String,
    /// The bodies that confirmed this harness with `query/support ok:true`.
    pub bodies: Vec<String>,
}

/// One session row in a **body**'s status document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusSession {
    pub name: String,
    pub harness: String,
    /// The A2A session state: `idle` | `working` | `input-required`.
    pub state: String,
}

/// `query/caps` = `status` plus an explicit `caps` map (docs §5.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Caps {
    /// The underlying status document.
    pub status: Status,
    /// Every known feature / harness → its support answer.
    #[serde(default)]
    pub caps: BTreeMap<String, Support>,
}

// ---------------------------------------------------------------------------
// support / protocol
// ---------------------------------------------------------------------------

/// The `params` of `query/support`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportParams {
    /// A feature or harness id (docs §9). Unknown → `-32006`.
    pub feature: String,
}

/// The `result` of `query/support` (docs §5.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    /// The id that was asked about.
    pub feature: String,
    /// `"feature"` | `"harness"` | `"capability"`.
    pub kind: String,
    /// Whether this endpoint supports it.
    pub ok: bool,
    /// How it is satisfied (e.g. `"opencode acp"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub how: Option<String>,
    /// Why it is not supported (`ok: false`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The `params` of `query/protocol`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolParams {
    /// A specific protocol version to test. `None` → report the range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
}

/// The `result` of `query/protocol` (docs §5.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolAnswer {
    /// The current protocol major version. Always `2` in v2.
    pub session: u32,
    /// The lowest protocol version this endpoint can also speak.
    pub min: u32,
    /// The highest protocol version this endpoint can also speak.
    pub max: u32,
    /// The `version` that was asked (when `params.version` was set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asked: Option<u32>,
    /// `true` iff `asked >= min && asked <= max`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
}

// ---------------------------------------------------------------------------
// presence (body → hub notification)
// ---------------------------------------------------------------------------

/// A `session/presence` notification (docs §7). Sent periodically (heartbeat)
/// and on every (re)connect; the peer treats each as authoritative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Presence {
    /// The body's hostname / label.
    pub hostname: String,
    /// The body's sessions and their current A2A session state.
    #[serde(default)]
    pub sessions: Vec<SessionAd>,
}

/// One session in a presence advertisement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionAd {
    /// The routable name (`<label>/<session>`).
    pub name: String,
    /// The harness hosting this session.
    pub harness: String,
    /// The A2A **session** state — one of `idle`, `working`, `input-required`
    /// (`idle` is Holler's addition, ADR 0005; it has no A2A task-state
    /// constant).
    pub state: String,
    /// `"spawn"` or `"attach"`.
    pub mode: String,
    /// The existing harness session being attached to (attach mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// prompt (hub → body request)
// ---------------------------------------------------------------------------

/// The `params` of `session/prompt` (docs §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prompt {
    /// The target session name.
    pub session: String,
    /// The user's message — an A2A `Message` with `role:"user"`.
    pub message: Message,
    /// Optional caller metadata (e.g. the originating CLI invocation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<BTreeMap<String, Value>>,
}

/// The `result` of `session/prompt` — sent **exactly once**, when the turn
/// ends. `message.parts` is the full reply (coalesced text + any interleaved
/// raw/url/data parts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptResult {
    /// The ACP `stopReason`, carried verbatim (the source of truth, ADR 0004).
    pub stop_reason: String,
    /// The A2A terminal state derived from `stop_reason` (docs §6 table).
    pub state: String,
    /// The agent's reply — an A2A `Message` with `role:"agent"`.
    pub message: Message,
}

// ---------------------------------------------------------------------------
// update / cancel
// ---------------------------------------------------------------------------

/// A `session/update` notification — one streamed chunk of the reply (docs
/// §6). A `text` part is a streamed delta (the coalescer merges consecutive
/// text parts); `raw`/`url`/`data` parts are kept as their own parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    /// The session the update belongs to.
    pub session: String,
    /// The prompt this update was produced by (the request's id).
    pub prompt_id: String,
    /// A per-prompt monotonic sequence number.
    pub seq: u64,
    /// The new part(s) — exactly the A2A parts for this chunk.
    pub parts: Vec<Part>,
}

/// The `params` of `session/cancel` (docs §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cancel {
    /// The session whose in-flight turn is being cancelled.
    pub session: String,
}

/// The `result` of `session/cancel` — sent **only after** the cancel was
/// applied (the acknowledgement the interruption path wanted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelResult {
    /// Always `true` (the cancel was applied).
    pub applied: bool,
}

/// The `params` of `circuit/join` (docs §3) — the one-time bootstrap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    /// The one-time join secret.
    pub secret: String,
    /// The hostname / label to claim.
    pub hostname: String,
}

/// The `result` of `circuit/join` — `{client_id, credential}`; the hub then
/// **closes the socket**.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinResult {
    pub client_id: String,
    pub credential: String,
}

/// The `params` of `circuit/authenticate` (docs §3) — the normal (re)connect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authenticate {
    /// The short token id.
    pub token_id: String,
    /// The long-lived credential (not the join secret).
    pub credential: String,
    /// The hostname / label to claim.
    pub hostname: String,
}

/// The `result` of `circuit/authenticate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthOk {
    pub ok: bool,
}

// ---------------------------------------------------------------------------
// ACP stop_reason → A2A terminal state (docs §6 table)
// ---------------------------------------------------------------------------

/// The ACP `stopReason` → A2A terminal-state mapping (docs §6, ADR 0004).
///
/// A2A's own mapping (spec §4.1.11) already covers `end_turn`, `maxTokens`,
/// `maxTaskRequests`, `interrupted`, and `failed`. The two ACP values the A2A
/// table **cannot** express — `refusal` (→ `rejected`) and `limit` (→
/// `failed`) — are Holler's addition, so this table is the single source of
/// truth for the body's derivation. It is **total**: every ACP stopReason
/// lands on an A2A terminal state, so the bridge is a pass-through.
pub const STOP_TO_STATE: &[(&str, &str)] = &[
    ("end_turn", "completed"),
    ("cancelled", "canceled"),
    ("error", "failed"),
    ("refusal", "rejected"),
    ("max_tokens", "failed"),
    ("max_turn_requests", "failed"),
    ("limit", "failed"),
];

/// The A2A terminal states a turn may end in (the codomain of
/// `STOP_TO_STATE`). `input-required` is *not* terminal (the turn pauses for
/// input); `idle` is a session, not a task, state.
pub const A2A_TERMINAL_STATES: &[&str] = &["completed", "canceled", "failed", "rejected"];

/// Derive the A2A terminal state from an ACP `stopReason` (docs §6 table).
///
/// **Total**: returns `Some` for every ACP stopReason in `STOP_TO_STATE`. For
/// a stopReason outside the table (an ACP version drift) it returns `None` —
/// the caller then answers `failed` and logs, so a turn can never end in a
/// non-terminal or unmapped state.
pub fn state_for_stop_reason(stop_reason: &str) -> Option<String> {
    STOP_TO_STATE
        .iter()
        .find(|(s, _)| *s == stop_reason)
        .map(|(_, st)| st.to_string())
}
