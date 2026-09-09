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
use crate::error::{Code, Error as WireError};

// ---------------------------------------------------------------------------
// enums (docs §3–7): closed string vocabularies as real types
// ---------------------------------------------------------------------------

/// The two endpoint roles a `circuit/hello` (or `query/status`) document is
/// sent from. Wire values `"body"` / `"hub"` (docs §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HelloRole {
    /// A body (a box running harnesses and sessions).
    Body,
    /// The hub (the router that bodies connect to).
    Hub,
}

/// A `query/support` answer's subject class (docs §5.3). Wire values
/// `"feature"` / `"harness"` / `"capability"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportKind {
    /// A protocol feature id (docs §9).
    Feature,
    /// A harness id (docs §9).
    Harness,
    /// A capability (a finer-grained feature of a harness).
    Capability,
}

/// The A2A **session** state a session is in (docs §7): `idle` (Holler's
/// addition, ADR 0005 — no A2A task-state equivalent), `working`, or
/// `input-required`. Distinct from the A2A *task* states (`TaskState`, a2a)
/// which a turn ends in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionState {
    /// The session is idle (no turn in flight) — Holler's addition.
    Idle,
    /// The session is mid-turn.
    Working,
    /// The turn is paused awaiting user input.
    InputRequired,
    /// The turn ended successfully (A2A `completed`).
    Completed,
    /// The turn was cancelled (A2A `canceled`).
    Canceled,
    /// The turn ended in an error (A2A `failed`).
    Failed,
    /// The turn was refused (A2A `rejected`; Holler's addition).
    Rejected,
}

/// How a session was established (docs §7): `spawn` a fresh harness session or
/// `attach` to an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Spawn a fresh harness session.
    Spawn,
    /// Attach to an existing harness session.
    Attach,
}

impl HelloRole {
    /// The wire string (`"body"` / `"hub"`).
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            HelloRole::Body => "body",
            HelloRole::Hub => "hub",
        }
    }
}
impl std::fmt::Display for HelloRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SupportKind {
    /// The wire string (`"feature"` / `"harness"` / `"capability"`).
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            SupportKind::Feature => "feature",
            SupportKind::Harness => "harness",
            SupportKind::Capability => "capability",
        }
    }
}
impl std::fmt::Display for SupportKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SessionState {
    /// The wire string (`"idle"`, `"working"`, `"input-required"`,
    /// `"completed"`, `"canceled"`, `"failed"`, `"rejected"`).
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            SessionState::Idle => "idle",
            SessionState::Working => "working",
            SessionState::InputRequired => "input-required",
            SessionState::Completed => "completed",
            SessionState::Canceled => "canceled",
            SessionState::Failed => "failed",
            SessionState::Rejected => "rejected",
        }
    }
}
impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Mode {
    /// The wire string (`"spawn"` / `"attach"`).
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Mode::Spawn => "spawn",
            Mode::Attach => "attach",
        }
    }
}
impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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
    /// The endpoint role: `"body"` or `"hub"`.
    pub role: HelloRole,
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
    /// How the session was established: `spawn` or `attach`.
    pub mode: Mode,
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
    pub role: HelloRole,
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
    pub state: SessionState,
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
    /// The subject class: `feature` | `harness` | `capability`.
    pub kind: SupportKind,
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

/// What kind of input a session is holding open while `input-required`
/// (issue #151). The two ACP driver prompts that pause a turn — `session/
/// request_permission` and `elicitation/create` — each map to `InputRequired`
/// (ADR-0004 / the ACP driver story #341), and the roster's `PENDING` column
/// (the roster story #340) must show the caller *which* of the two is being
/// asked. Wire strings are kebab-case to match the ACP method names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingKind {
    /// A permission prompt (ACP `session/request_permission`) — the caller
    /// picks one of the offered permission options (`allow` / `always` /
    /// `reject`).
    Permission,
    /// An elicitation (ACP `elicitation/create`) — the caller supplies a
    /// structured answer to the agent's question.
    Elicitation,
}

/// One held input item advertised in a presence row while a session is
/// `input-required` (issue #151) — the roster's `PENDING` column is rendered
/// from these. `prompt` is the human-readable question (the ACP permission
/// `title`, or the elicitation prompt); `options` are the answerable choices
/// (the ACP `PermissionOption` ids/names for a `Permission`, or the elicitation
/// schema fields). `id` is the stable handle the later `answer` request
/// (the session-manager story #342) resolves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingItem {
    /// Stable handle for this held item — what `answer SESSION <id>`
    /// (or its `once`/`always`/`reject` choices) names.
    pub id: String,
    /// Whether this is a permission or an elicitation.
    pub kind: PendingKind,
    /// The human-readable question being asked.
    pub prompt: String,
    /// The answerable choices (empty when the elicitation has no enumerable
    /// options).
    #[serde(default)]
    pub options: Vec<String>,
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
    pub state: SessionState,
    /// How the session was established: `spawn` or `attach`.
    pub mode: Mode,
    /// The existing harness session being attached to (attach mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_session_id: Option<String>,
    /// When the in-flight turn started (RFC 3339). Present only while
    /// `state` is `working` (issue #150 — the busy-turn policy's `--queue`
    /// hint and the roster's `stalled` derivation both read this).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_started_at: Option<String>,
    /// When the body last sent a `session/update` for the in-flight turn
    /// (RFC 3339). Present only while `state` is `working`; a session
    /// `working` with no update for `HOLLER_STALL_MS` displays as roster's
    /// derived `stalled` state (issue #150).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update_at: Option<String>,
    /// The held permission(s)/elicitation(s) being asked. Present **only**
    /// while `state` is `input-required` (issue #151) — it tells the roster
    /// and the CLI *what* is being asked, so `answer` has a target; absent
    /// for every other state, mirroring how the `working`-only timing fields
    /// above stay off the wire. The roster's `PENDING` column (the roster
    /// story #340) renders from this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<Vec<PendingItem>>,
    /// The JSON-RPC `id` of the current or most recent `session/prompt`
    /// (issue #142) — the `h-…` id `holler wait --after <turn_id>` reads
    /// back to avoid re-matching an already-seen turn. **Absent before the
    /// first prompt** a session has ever received; present (and unchanging)
    /// once a prompt has been sent, independent of `state`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// The outcome of the most recently **completed** turn (issue #142) —
    /// present once a turn has ended, and updated on every subsequent turn.
    /// `wait`'s default `--until` set and the roster's `LAST TURN` column
    /// (the roster story #340) both read this; it survives a reconnect
    /// (carried in presence, not just the one-shot `session/prompt`
    /// response) so a `wait --after` call made after a body drops and
    /// rejoins still sees the prior turn's outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<LastTurn>,
}

/// The outcome of a session's most recently completed turn (issue #142).
/// `state` reuses [`SessionState`]'s four A2A terminal variants (`completed`
/// | `canceled` | `failed` | `rejected`) rather than a parallel enum — it is
/// exactly the codomain of [`STOP_TO_STATE`] / [`A2A_TERMINAL_STATES`], and
/// ADR 0004's "one vocabulary" already settled that terminal turn outcomes
/// have one name each on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastTurn {
    /// The JSON-RPC id of the `session/prompt` this outcome belongs to —
    /// compared against `wait --after <turn_id>` to detect a *new* terminal
    /// turn rather than re-matching one already reported to the caller.
    pub turn_id: String,
    /// The A2A terminal state the turn ended in: `completed` | `canceled` |
    /// `failed` | `rejected`.
    pub state: SessionState,
    /// The ACP `stopReason`, carried verbatim (docs §6) — the source of
    /// truth `state` was derived from.
    pub stop_reason: String,
    /// When the turn ended (RFC 3339), matching the `turn_started_at` /
    /// `last_update_at` convention (issue #150).
    pub ended_at: String,
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
// ping (issue #182) — a hub-initiated liveness probe on a live circuit
// ---------------------------------------------------------------------------

/// The `result` of a `circuit/ping` a body answers (issue #182). `circuit/
/// ping` carries no params (an empty object decodes fine via
/// [`crate::typed_params`]'s "no params → empty object" rule) — the probe is
/// "are you there", not a query. `ts` is the body's own clock at answer time;
/// the asker (the hub, for `hub token ping`) derives `rtt_ms` from its own
/// elapsed wall-clock around the request, not from this field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PingAck {
    /// The body's hostname / label.
    pub hostname: String,
    /// The body's own unix-epoch milliseconds at answer time.
    pub ts: i64,
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
pub const STOP_TO_STATE: &[(&str, SessionState)] = &[
    ("end_turn", SessionState::Completed),
    ("cancelled", SessionState::Canceled),
    ("error", SessionState::Failed),
    ("refusal", SessionState::Rejected),
    ("max_tokens", SessionState::Failed),
    ("max_turn_requests", SessionState::Failed),
    ("limit", SessionState::Failed),
];

/// The A2A terminal states a turn may end in (the codomain of
/// `STOP_TO_STATE`). `input-required` is *not* terminal (the turn pauses for
/// input); `idle` is a session, not a task, state.
pub const A2A_TERMINAL_STATES: &[SessionState] = &[
    SessionState::Completed,
    SessionState::Canceled,
    SessionState::Failed,
    SessionState::Rejected,
];

/// Derive the A2A terminal state from an ACP `stopReason` (docs §6 table).
///
/// **Total**: returns `Some` for every ACP stopReason in `STOP_TO_STATE`. For
/// a stopReason outside the table (an ACP version drift) it returns `None` —
/// the caller then answers `failed` and logs, so a turn can never end in a
/// non-terminal or unmapped state.
pub fn state_for_stop_reason(stop_reason: &str) -> Option<SessionState> {
    STOP_TO_STATE
        .iter()
        .find(|(s, _)| *s == stop_reason)
        .map(|(_, st)| *st)
}

/// Parse a wire `state` string into a [`SessionState`].
///
/// Returns an `invalid_params` wire error when `s` is not one of the seven
/// documented states. The offending value rides in the error `reason` for
/// diagnostics. This is the one place a `state` string is read back to a
/// typed value (decoding a full document uses serde directly against the
/// enums); a hub or body that receives an unknown state should reject it
/// rather than invent a state.
pub fn parse_session_state(s: &str) -> Result<SessionState, WireError> {
    match s {
        "idle" => Ok(SessionState::Idle),
        "working" => Ok(SessionState::Working),
        "input-required" => Ok(SessionState::InputRequired),
        "completed" => Ok(SessionState::Completed),
        "canceled" => Ok(SessionState::Canceled),
        "failed" => Ok(SessionState::Failed),
        "rejected" => Ok(SessionState::Rejected),
        _ => Err(WireError::new(
            Code::InvalidParams,
            "unknown session state",
            Some("state"),
        )),
    }
}
