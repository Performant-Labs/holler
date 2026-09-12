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
    /// **Hub only** (issue #322): the hub's long-lived X25519 static public
    /// key, hex-encoded. A body pins this at `body join` (from the join
    /// line's out-of-band copy, never from the wire) and compares it here on
    /// every subsequent hello — a mismatch is a hard failure, never a prompt,
    /// never trust-on-first-use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_pubkey: Option<String>,
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub role: HelloRole,
    pub protocol: u32,
    pub protocol_min: u32,
    pub protocol_max: u32,
    /// The running binary's version (issue #319), `env!("CARGO_PKG_VERSION")` —
    /// never hand-rolled or hardcoded independently.
    pub version: String,
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
    /// **Body**: the sessions this body hosts, with their live state. Wire
    /// key `"sessions"` (docs §5.1, issue #250) — see the hand-written
    /// [`Deserialize`] impl below for why this can't just be
    /// `#[serde(rename = "sessions")]` like the `Serialize` side.
    #[serde(rename = "sessions", skip_serializing_if = "Option::is_none")]
    pub session_list: Option<Vec<StatusSession>>,
}

// `sessions` (the hub's session *count*, a `u32`) and `session_list` (a
// body's session *list*, wire-renamed to the same key `"sessions"` — docs
// §5.1, issue #250) can't both derive `Deserialize` normally: the derive
// macro identifies an incoming field by matching the wire key string, and
// two Rust fields claiming the identical wire key `"sessions"` makes it
// generate two identical match arms — the second is `unreachable_pattern`
// dead code, and whichever arm the derive keeps wins for *every* incoming
// `"sessions"` key regardless of its JSON shape (silently misrouting or
// dropping the other role's payload). The two are mutually exclusive on the
// wire — a hub sends a number, a body sends an array, per docs §5.1's own
// examples, never both — so this hand-written impl routes on the JSON
// shape instead of on declared field identity, then defers every other
// field to `serde_json::from_value` (inferring each field's type from
// `Status`'s own definition, so it can't drift from it silently).
impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        fn take<T: serde::de::DeserializeOwned, E: serde::de::Error>(
            obj: &mut serde_json::Map<String, Value>,
            key: &str,
        ) -> Result<Option<T>, E> {
            match obj.remove(key) {
                Some(v) => serde_json::from_value(v).map(Some).map_err(E::custom),
                None => Ok(None),
            }
        }
        fn require<T: serde::de::DeserializeOwned, E: serde::de::Error>(
            obj: &mut serde_json::Map<String, Value>,
            key: &str,
        ) -> Result<T, E> {
            take(obj, key)?.ok_or_else(|| E::custom(format!("Status: missing field `{key}`")))
        }

        let mut value = Value::deserialize(deserializer)?;
        let obj = value.as_object_mut().ok_or_else(|| D::Error::custom("Status: expected a JSON object"))?;

        let (sessions, session_list): (Option<u32>, Option<Vec<StatusSession>>) = match obj.remove("sessions") {
            None => (None, None),
            Some(arr @ Value::Array(_)) => (None, Some(serde_json::from_value(arr).map_err(D::Error::custom)?)),
            Some(num @ Value::Number(_)) => (Some(serde_json::from_value(num).map_err(D::Error::custom)?), None),
            Some(other) => return Err(D::Error::custom(format!("Status: `sessions` must be a number or an array, got {other}"))),
        };

        Ok(Status {
            role: require(obj, "role")?,
            protocol: require(obj, "protocol")?,
            protocol_min: require(obj, "protocol_min")?,
            protocol_max: require(obj, "protocol_max")?,
            version: require(obj, "version")?,
            hostname: require(obj, "hostname")?,
            connected: take(obj, "connected")?,
            token_id: take(obj, "token_id")?,
            listening: take(obj, "listening")?,
            features: take(obj, "features")?.unwrap_or_default(),
            harnesses: take(obj, "harnesses")?,
            harnesses_known: take(obj, "harnesses_known")?,
            harnesses_confirmed: take(obj, "harnesses_confirmed")?,
            bodies: take(obj, "bodies")?,
            sessions,
            session_list,
        })
    }
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
    /// The attach endpoint (issue #195: `attach`-mode sessions only —
    /// `None` for `spawn`). `#[serde(default)]` on deserialize so an older
    /// peer's response (from before this field existed) still parses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
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

impl ProtocolParams {
    /// Parse and validate `query/protocol`'s optional `version` (docs §5.4)
    /// out of the raw request `params`, shared by both the hub's and the
    /// body's dispatch (issue #251): absent `params`, or `params` present but
    /// with no `version` key, is `Ok(None)` — no version asked, never a
    /// rejection. A `version` that is present but is not a positive integer
    /// — `0`, negative, non-numeric, or too large to fit `u32` — is the
    /// documented `-32006 unknown_feature` refusal (docs §5.4: "`n` must be a
    /// positive integer, else `-32006 unknown_feature`") instead of silently
    /// falling through as "no version asked". Only a `version` that parses
    /// as a valid positive `u32` reaches [`local_protocol`]-style range
    /// checking as `Ok(Some(v))`.
    pub fn parse_version(params: Option<&Value>) -> Result<Option<u32>, WireError> {
        let Some(raw) = params.and_then(|p| p.get("version")) else {
            return Ok(None);
        };
        match raw.as_u64().and_then(|v| u32::try_from(v).ok()) {
            Some(0) | None => Err(WireError::new(
                Code::UnknownFeature,
                format!("query/protocol version must be a positive integer, got {raw}"),
                Some("version"),
            )),
            Some(v) => Ok(Some(v)),
        }
    }
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
    /// When the in-flight turn started (RFC 3339). Present from the start of
    /// a turn until it ends — i.e. through both `working` and any
    /// `input-required` pause within it, not only while `state` is `working`
    /// (issue #213: the timing stays meaningful across the pause too, so
    /// clearing it on `input-required` would throw away information a
    /// consumer might want). Absent while `idle`. The busy-turn policy's
    /// `--queue` hint and the roster's `stalled` derivation read this (issue
    /// #150).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_started_at: Option<String>,
    /// When the body last saw real activity for the in-flight turn — a
    /// `session/update` chunk *or* a `working`/`input-required` state
    /// transition (RFC 3339; issue #210 closed the gap where only state
    /// transitions bumped this, leaving an actively-streaming turn looking
    /// stale). Present from the start of a turn until it ends, through any
    /// `input-required` pause within it, matching `turn_started_at` above
    /// (issue #213) — absent while `idle`. A session `working` with no
    /// update for `HOLLER_STALL_MS` displays as roster's derived `stalled`
    /// state (issue #150).
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
    /// `say --queue` (issue #150/#190): append behind a busy session's
    /// current turn instead of refusing with `-32009 session_busy`. Absent on
    /// the wire (and defaulted on decode) when `false`, so this addition
    /// keeps every pre-#190 `Prompt` fixture byte-identical.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub queue: bool,
    /// `interrupt SESSION TEXT` (issue #191): cancel the in-flight turn (if
    /// any) and run this prompt **ahead of** the queue — the body-side
    /// `SessionCommand::Replace` a `session/cancel` + this flag together
    /// implement. Absent on the wire (and defaulted on decode) when `false`,
    /// same discipline as `queue` — every pre-#191 `Prompt` fixture stays
    /// byte-identical. Mutually exclusive with `queue` in practice (the hub
    /// never sets both), but the wire does not enforce that itself.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace: bool,
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

/// The `params` of `session/answer` (issue #151): resolve a held
/// `input-required` permission/elicitation. `choice` is a 0-based index, an
/// exact (case-insensitive) label/key match, or — for a multi-field
/// elicitation — a comma-separated list, one segment per field, in the
/// order the pending item's `options` were declared (`holler_body::
/// acp_driver::answerable::resolve_choice` is the resolution logic this
/// mirrors on the wire).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The target session name.
    pub session: String,
    /// The caller's choice (index | label/key | comma-separated segments).
    pub choice: String,
}

/// The `result` of `session/answer` — sent **only after** the driver
/// resumed the turn (the ACP `session/request_permission`/`elicitation/
/// create` reply went out and the agent reported `working` again, or the
/// turn ended outright). A session with nothing pending answers
/// `-32010 nothing_pending` instead of this result (docs §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerResult {
    /// Always `true` (the answer was applied).
    pub applied: bool,
}

/// The `params` of `circuit/join` (docs §3) — the one-time bootstrap.
///
/// Issue #323: the body registers its own long-lived **public** key at join
/// instead of receiving a bearer credential back — the private half never
/// leaves the body (generated and persisted locally, `holler_body::identity`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    /// The one-time join secret.
    pub secret: String,
    /// The hostname / label to claim.
    pub hostname: String,
    /// The body's Ed25519 public key, hex-encoded (32 bytes). Stored against
    /// the token; used to verify every later `circuit/prove`.
    pub body_pubkey: String,
}

/// The `result` of `circuit/join` — `{client_id}`; the hub then **closes the
/// socket**. Issue #323: no `credential` — proof of possession of the
/// registered `body_pubkey` (via `circuit/authenticate` → `circuit/prove`)
/// replaces presenting a bearer secret on every reconnect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinResult {
    pub client_id: String,
}

/// The `params` of `circuit/authenticate` (docs §3) — the **first** frame of
/// the normal (re)connect's now two-step challenge-response (issue #323): the
/// body names the token and the address it believes it is dialing; the hub's
/// **result** is not a bare ack but an [`AuthChallenge`] — a fresh nonce the
/// body must sign and return via `circuit/prove`. No secret crosses the wire
/// here (the join secret is spent, one-time, at `circuit/join`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authenticate {
    /// The short token id.
    pub token_id: String,
    /// The hostname / label to claim.
    pub hostname: String,
    /// The URL this body believes it is dialing (its own configured
    /// `--server` / persisted `server_url`) — bound into the
    /// [`crate::transcript::build`] transcript `circuit/prove` signs, so a
    /// captured proof cannot be replayed against a different endpoint.
    pub advertised_url: String,
}

/// The `result` of `circuit/authenticate` (issue #323) — a fresh, single-use
/// challenge, not an ack. The body must answer with `circuit/prove`
/// (params: [`Prove`]) within the hub's prove timeout, or the socket is
/// closed as unauthenticated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthChallenge {
    /// A fresh, hex-encoded random nonce, unique to this connection attempt
    /// and never persisted or reused — a signature over one nonce cannot be
    /// replayed against a challenge minted for a different attempt.
    pub nonce: String,
}

/// The `params` of `circuit/prove` (issue #323) — the **second** frame of the
/// authenticate handshake: the body proves possession of the private key
/// matching the `body_pubkey` it registered at join, by signing the
/// [`crate::transcript::build`] transcript over the hub's own
/// [`AuthChallenge::nonce`] (never a value the body chooses).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prove {
    /// The short token id (must match the preceding `circuit/authenticate`).
    pub token_id: String,
    /// The Ed25519 signature over the transcript, hex-encoded (64 bytes).
    pub signature: String,
}

/// The `result` of `circuit/authenticate`'s handshake — sent as the response
/// to `circuit/prove`, once the signature verifies.
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

