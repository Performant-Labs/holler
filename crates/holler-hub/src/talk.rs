//! `say` (issue #190): the hub-side orchestration behind `control/say`.
//!
//! `holler say io/alpha "…"` finally does something real end to end: resolve
//! the target name against the live registry (issue #190's roster stand-in —
//! see [`crate::live`]'s module doc), a fail-closed busy check, send
//! `session/prompt` on the token's live socket, collect the reply, append
//! the TalkLog, and hand the CLI a result it can print or `--json`.
//!
//! # Decisions I made
//!
//! - **TalkLog is written once, after the exchange settles**, not
//!   incrementally as each `session/update` arrives. The spec's own shape
//!   (`{ts, prompt_id, text}` for the prompt line, one `{ts, prompt_id, seq,
//!   parts}` per update, one `{ts, prompt_id, stopReason}` for done) is
//!   still produced exactly — [`crate::circuit`] already timestamps each
//!   `session/update` the instant it arrives (`SeenUpdate::ts`) and hands the
//!   whole ordered list back here — this only changes *when the bytes hit
//!   disk*, not their content or order. True incremental durability (surviving
//!   a hub crash mid-turn) would need the TalkLog writer to live inside the
//!   connection task itself; flagged as a real gap, not a silent one, but out
//!   of this story's reach given `say` already blocks the whole exchange in
//!   one control-socket round trip.
//! - **The busy check reads the same registry cache `say` itself resolves
//!   the name against** (see [`crate::live`]) rather than a fresh roundtrip —
//!   consistent with the issue's own "the body enforces the same rule, so a
//!   race still fails closed" (the body's own [`SessionManager`] rejects a
//!   stale-busy race even if this cache is a beat behind).
//! - **`last_turn` is updated on the cached presence, best-effort.** The
//!   issue asks for "the roster row's `last_turn`"; without #186's real
//!   roster row, the closest analogue is this story's own presence cache —
//!   updated here so a `say` immediately followed by `roster` (once #186
//!   lands and reads this same cache, or its replacement) reflects the turn
//!   that just ran instead of stale data.

use std::time::{Duration, Instant};

use holler_proto::{Content, Message, Part, Role, SessionState};

use crate::live::{LiveHandle, Registry, ResolveOutcome, SayReply};
use crate::roster::Roster;
use crate::state::{talklog_dir, talklog_path, HubState};

/// Why [`say`] could not produce a reply.
pub enum SayError {
    /// No live session matched `session` at all.
    UnknownSession,
    /// A session by this name exists but its body is not currently
    /// connected (issue #190's `not_connected` case). Carries a
    /// human-readable detail (issue #192, rule 4): when the roster still
    /// remembers the name, this names its `conn_state` and (for
    /// `reconnecting`) how long ago it was last seen — e.g. `"io/alpha is
    /// reconnecting (last seen 32s ago)"` — rather than a bare "not
    /// connected" that hides whether the body might still come back.
    NotConnected(String),
    /// More than one live session matched a bare name; `candidates` are
    /// `<label>/<session>` strings for the CLI to list.
    Ambiguous(Vec<String>),
    /// The target session is `working`/`stalled` and `--queue` was not
    /// given (or was refused outright on `input-required`).
    Busy { state: String, turn_age_ms: u64, last_update_age_ms: u64 },
    /// The target session is `input-required`, holding a question (issue
    /// #151: `--queue` never overrides this — a held permission/elicitation
    /// is answered, not queued behind). `question` is the pending item's own
    /// prompt text when the cached presence carries one (empty roster
    /// caches — e.g. a body that predates issue #151 — still refuse, just
    /// without that detail).
    InputRequired { question: Option<String>, turn_age_ms: u64, last_update_age_ms: u64 },
    /// The body answered with some other JSON-RPC refusal.
    Refused(holler_proto::WireError),
    /// The socket dropped mid-turn.
    ConnectionLost,
    /// No reply arrived within the caller's timeout budget.
    Timeout,
    /// The turn was cancelled before it completed.
    Cancelled,
}

impl SayError {
    /// A one-line, human-readable reason (issue #191's own need: `interrupt
    /// SESSION TEXT`'s redirect phase shares this error shape but has no
    /// `say`-specific wording of its own to fall back on for the variants
    /// that can't actually recur once the cancel step already confirmed the
    /// session idle — `Busy`/`InputRequired`/`Ambiguous` are still covered
    /// so this stays total).
    pub fn message(&self) -> String {
        match self {
            Self::UnknownSession => "unknown session".to_string(),
            Self::NotConnected(detail) => detail.clone(),
            Self::Ambiguous(candidates) => format!("ambiguous session: candidates are {}", candidates.join(", ")),
            Self::Busy { state, .. } => format!("session_busy: {state}"),
            Self::InputRequired { question, .. } => {
                question.clone().unwrap_or_else(|| "waiting for an answer".to_string())
            }
            Self::Refused(err) => err.message.clone(),
            Self::ConnectionLost => "io disconnected mid-turn; ask again".to_string(),
            Self::Timeout => "no reply within the timeout".to_string(),
            Self::Cancelled => "prompt was interrupted before it completed".to_string(),
        }
    }
}

/// Why [`answer`] could not resolve a held permission/elicitation (issue
/// #151). Mirrors [`SayError`]'s own resolve-target shapes; the body-side
/// refusal (nothing pending, an unresolved choice, …) always arrives as
/// [`AnswerError::Refused`] — the hub does not reinterpret those codes, it
/// only forwards them (see `crate::control_server::answer`).
pub enum AnswerError {
    /// No live session matched `session` at all.
    UnknownSession,
    /// A session by this name exists but its body is not currently
    /// connected.
    NotConnected,
    /// More than one live session matched a bare name; `candidates` are
    /// `<label>/<session>` strings for the CLI to list.
    Ambiguous(Vec<String>),
    /// The body answered with a JSON-RPC error (`-32010 nothing_pending`,
    /// `-32602 invalid_params`, …) — forwarded verbatim.
    Refused(holler_proto::WireError),
    /// No reply arrived within the caller's timeout budget (or the
    /// connection ended first) — the closest analogue to `say`'s own
    /// `ConnectionLost`/`Timeout` split, collapsed to one variant since
    /// `answer` has no streamed `session/update`s to have already recorded.
    ConnectionLost,
}

/// A successful [`answer`] outcome.
pub struct AnswerOutcome {
    pub session: String,
    pub applied: bool,
}

/// Resolve `session`'s held permission/elicitation with `choice` (issue
/// #151): resolve the target against the live registry exactly like
/// [`say`], then forward a `session/answer {session, choice}` request over
/// that body's own socket via [`crate::live::LiveHandle::query`] — the same
/// generic per-connection request/response forward `hub query TARGET …`
/// (issue #185) already uses, reused here rather than adding a dedicated
/// `LiveCommand` variant for what is, on the wire, just one more
/// request/response pair on the same connection.
pub async fn answer(
    registry: &Registry,
    session: &str,
    choice: &str,
    timeout: Duration,
) -> Result<AnswerOutcome, AnswerError> {
    let name = holler_proto::RoutableName::parse(session).map_err(|_| AnswerError::UnknownSession)?;
    let (handle, ad) = match registry.resolve_session(&name).await {
        ResolveOutcome::Found(h, ad) => (h, ad),
        ResolveOutcome::Unknown => return Err(AnswerError::UnknownSession),
        ResolveOutcome::NotConnected => return Err(AnswerError::NotConnected),
        ResolveOutcome::Ambiguous(candidates) => return Err(AnswerError::Ambiguous(candidates)),
    };
    let params = serde_json::json!({ "session": ad.name, "choice": choice });
    match handle.query("session/answer", Some(params), timeout).await {
        None => Err(AnswerError::ConnectionLost),
        Some(Err(e)) => Err(AnswerError::Refused(e)),
        Some(Ok(value)) => {
            let applied = value.get("applied").and_then(|v| v.as_bool()).unwrap_or(false);
            Ok(AnswerOutcome { session: format!("{}/{}", handle.hostname, ad.name), applied })
        }
    }
}

/// A successful `say` outcome.
pub struct SayOutcome {
    pub session: String,
    pub message: Message,
    pub stop_reason: String,
    pub state: String,
    pub updates: usize,
    pub elapsed_ms: u64,
}

/// Send one prompt to `session` and wait for its reply. `state` is the
/// live hub's own `HubState` (for the TalkLog path); `label` is this call's
/// own request id source (an `h-…` [`holler_proto::CorrelationId`] is minted
/// internally per call, so `say` and any concurrent `say` never share a
/// `prompt_id`). `roster` gets this turn's `turn_id`/`last_turn` written
/// straight onto the row (issue #142) — ahead of dispatch and the instant the
/// response lands — so the roster is authoritative even between presence
/// beats (`prompt_response_updates_last_turn_before_next_presence`); the next
/// real presence overwrites the row anyway, so this is a best-effort
/// forward-fill, not the row's system of record.
pub async fn say(
    registry: &Registry,
    roster: &Roster,
    state: &HubState,
    session: &str,
    text: &str,
    queue: bool,
    timeout: Duration,
) -> Result<SayOutcome, SayError> {
    let name = holler_proto::RoutableName::parse(session).map_err(|_| SayError::UnknownSession)?;
    let (handle, ad) = match registry.resolve_session(&name).await {
        ResolveOutcome::Found(h, ad) => (h, ad),
        ResolveOutcome::Unknown => return Err(SayError::UnknownSession),
        ResolveOutcome::NotConnected => return Err(SayError::NotConnected(not_connected_detail(roster, session))),
        ResolveOutcome::Ambiguous(candidates) => return Err(SayError::Ambiguous(candidates)),
    };

    // The busy check (issue #150's policy, enforced here as the hub's own
    // fail-closed gate — the body's `SessionManager` enforces it again,
    // independently, so a race still fails closed).
    match ad.state {
        SessionState::InputRequired => {
            let question = ad.pending.as_ref().and_then(|items| items.first()).map(|item| item.prompt.clone());
            let turn_age_ms = age_ms(ad.turn_started_at.as_deref());
            let last_update_age_ms = age_ms(ad.last_update_at.as_deref());
            return Err(SayError::InputRequired { question, turn_age_ms, last_update_age_ms });
        }
        SessionState::Working if !queue => {
            let turn_age_ms = age_ms(ad.turn_started_at.as_deref());
            let last_update_age_ms = age_ms(ad.last_update_at.as_deref());
            // `stalled` is a roster-derived display state (docs.rs's own
            // comment on `SessionAd.last_update_at`: "working with no update
            // for HOLLER_STALL_MS displays as roster's derived stalled
            // state") — #186 (roster) owns the real derivation; this is the
            // minimal stand-in `say`'s own busy hint needs (see the module
            // doc's "Decisions I made").
            let state = if last_update_age_ms >= stall_threshold_ms() { "stalled" } else { "working" };
            return Err(SayError::Busy { state: state.to_string(), turn_age_ms, last_update_age_ms });
        }
        _ => {}
    }

    let kind = if queue { TurnKind::Queue } else { TurnKind::Plain };
    send_turn(&TurnCtx { registry, roster, state }, &handle, &ad, text, kind, timeout).await
}

/// `interrupt SESSION TEXT`'s own redirect turn (issue #191): unlike [`say`],
/// no busy check runs here — the caller (`crate::interrupt::interrupt`) has
/// already confirmed the matching `session/cancel`'s `{applied:true}`, so the
/// session is expected to be idle by the time this sends `session/prompt
/// {replace:true}`. `handle`/`ad` are the same live-registry resolution the
/// cancel step already performed (resolving a second time would risk a
/// benign-but-pointless re-race against the registry's own presence cache).
pub async fn send_replace_turn(
    registry: &Registry,
    roster: &Roster,
    state: &HubState,
    handle: &LiveHandle,
    ad: &holler_proto::SessionAd,
    text: &str,
    timeout: Duration,
) -> Result<SayOutcome, SayError> {
    send_turn(&TurnCtx { registry, roster, state }, handle, ad, text, TurnKind::Replace, timeout).await
}

/// `queue`/`replace` collapsed into one enum purely to keep [`send_turn`]
/// under clippy's too-many-arguments gate — the two booleans were never
/// both meaningful at once (a redirect is never `--queue`d).
#[derive(Clone, Copy)]
enum TurnKind {
    /// A plain `say` (no `--queue`, no redirect): the busy check already ran
    /// in [`say`] before this turn is sent.
    Plain,
    /// `say --queue`: append behind the session's current turn instead of
    /// refusing with `session_busy`.
    Queue,
    /// `interrupt SESSION TEXT`'s own redirect (`Prompt.replace`).
    Replace,
}

impl TurnKind {
    fn queue(self) -> bool {
        matches!(self, Self::Queue)
    }
    fn replace(self) -> bool {
        matches!(self, Self::Replace)
    }
}

/// The shared hub-side context every [`send_turn`] call needs — bundled
/// purely to keep that function's own argument count under clippy's
/// too-many-arguments gate once issue #142 added `roster` alongside
/// `registry`/`state`; each field is exactly as independent as before, just
/// passed as one struct instead of three positional refs.
struct TurnCtx<'a> {
    registry: &'a Registry,
    roster: &'a Roster,
    state: &'a HubState,
}

/// The shared tail of [`say`]/[`send_replace_turn`]: mint a request id, send
/// `session/prompt` (a plain one, a `--queue`d one, or — issue #191 — a
/// `replace:true` redirect), collect the reply, append the TalkLog, and hand
/// back a [`SayOutcome`]. Split out once #191 needed a second caller that
/// skips the busy check but shares everything after it.
async fn send_turn(
    ctx: &TurnCtx<'_>,
    handle: &LiveHandle,
    ad: &holler_proto::SessionAd,
    text: &str,
    kind: TurnKind,
    timeout: Duration,
) -> Result<SayOutcome, SayError> {
    let TurnCtx { registry, roster, state } = *ctx;
    let request_id = holler_proto::CorrelationId::mint_hub().as_str().to_string();
    let message = user_message(&request_id, text);
    let started = Instant::now();

    // Issue #142: the roster's own `turn_id` moves the instant this turn is
    // dispatched, ahead of the body's next presence heartbeat — so a `wait
    // --after <prev turn_id>` started right after this call already sees a
    // fresh in-flight turn rather than the stale one it was watermarked on.
    roster.set_turn_id(&handle.token_id, ad.name.as_str(), &request_id);

    append_talklog(state, &handle.hostname, ad.name.as_str(), &TalkLine::Prompt {
        prompt_id: request_id.clone(),
        text: text.to_string(),
    });

    let reply = handle
        .say(request_id.clone(), ad.name.clone(), Box::new(message), kind.queue(), kind.replace(), timeout)
        .await;

    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    match reply {
        None => Err(SayError::Timeout),
        Some(SayReply::ConnectionLost) => Err(SayError::ConnectionLost),
        Some(SayReply::Refused(err)) => {
            if err.code == holler_proto::Code::SessionBusy.jsonrpc() {
                let (state_str, turn_age_ms, last_update_age_ms) = err
                    .data
                    .as_ref()
                    .map(|d| {
                        (
                            d.state.clone().unwrap_or_default(),
                            d.turn_age_ms.unwrap_or(0),
                            d.last_update_age_ms.unwrap_or(0),
                        )
                    })
                    .unwrap_or_default();
                Err(SayError::Busy { state: state_str, turn_age_ms, last_update_age_ms })
            } else {
                Err(SayError::Refused(err))
            }
        }
        Some(SayReply::Result { message, stop_reason, state: turn_state, updates }) => {
            for u in &updates {
                append_talklog(state, &handle.hostname, ad.name.as_str(), &TalkLine::Update {
                    prompt_id: request_id.clone(),
                    seq: u.seq,
                    parts: u.parts.clone(),
                    ts: u.ts.clone(),
                });
            }
            append_talklog(state, &handle.hostname, ad.name.as_str(), &TalkLine::Done {
                prompt_id: request_id.clone(),
                stop_reason: stop_reason.clone(),
            });
            if stop_reason == "cancelled" {
                return Err(SayError::Cancelled);
            }
            let last_turn = holler_proto::docs::LastTurn {
                turn_id: request_id.clone(),
                state: holler_proto::state_for_stop_reason(&stop_reason).unwrap_or(SessionState::Failed),
                stop_reason: stop_reason.clone(),
                ended_at: holler_proto::log::timestamp(),
            };
            let mut updated_ad = ad.clone();
            updated_ad.last_turn = Some(last_turn.clone());
            registry.replace_session(handle, updated_ad).await;
            // Issue #142: write `last_turn` onto the roster row directly, the
            // instant this response lands — the roster is authoritative even
            // between presence beats (see this fn's own doc comment).
            roster.set_last_turn(&handle.token_id, ad.name.as_str(), last_turn);
            Ok(SayOutcome {
                session: format!("{}/{}", handle.hostname, ad.name),
                message: *message,
                stop_reason,
                state: turn_state,
                updates: updates.len(),
                elapsed_ms,
            })
        }
    }
}

/// Build `say`'s `not_connected` detail message (issue #192, rule 4):
/// `session` as the caller typed it (a bare name or `<label>/<session>`)
/// resolved against every row the roster still remembers, live or not.
/// `reconnecting` names the row's own qualified name and how long ago it was
/// last seen (`"io/alpha is reconnecting (last seen 32s ago)"`) — explicit
/// about the hub's own v1 policy: **no queuing** here, the operator must
/// re-`say` once the body reconnects. Any other remembered `conn_state`
/// (`gone`) still gets a slightly richer message than the bare fallback; no
/// row at all (never seen, or pruned) falls back to the original #190
/// wording so `say_to_disconnected_body_is_not_connected`'s own assertion
/// (`err.contains("not connected")`) keeps holding either way.
fn not_connected_detail(roster: &Roster, session: &str) -> String {
    match roster.find_by_name(session) {
        Some(row) if row.conn_state == "reconnecting" => {
            let age = roster.age_secs(&row);
            format!("{} is reconnecting (last seen {age}s ago)", row.name)
        }
        Some(row) => format!("{} is not connected ({})", row.name, row.conn_state),
        None => format!("{session}'s body is not connected"),
    }
}

/// How long a `working` session may go without a `session/update` before
/// `say`'s busy hint calls it `stalled` instead of `working`.
/// `HOLLER_STALL_MS` overrides the 30s default.
fn stall_threshold_ms() -> u64 {
    std::env::var("HOLLER_STALL_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(30_000)
}

fn age_ms(rfc3339: Option<&str>) -> u64 {
    let Some(s) = rfc3339 else { return 0 };
    let Ok(then) = time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339) else {
        return 0;
    };
    let now = time::OffsetDateTime::now_utc();
    u64::try_from((now - then).whole_milliseconds()).unwrap_or(0)
}

fn user_message(request_id: &str, text: &str) -> Message {
    Message {
        message_id: format!("m-{request_id}"),
        context_id: None,
        task_id: None,
        role: Role::RoleUser,
        parts: vec![Part::text_part(text)],
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

enum TalkLine {
    Prompt { prompt_id: String, text: String },
    Update { prompt_id: String, seq: u64, parts: Vec<Part>, ts: String },
    Done { prompt_id: String, stop_reason: String },
}

/// Append one line to `<state>/hub/talklog/<label>__<session>.jsonl`
/// (issue #190 spec). Best-effort: a write failure is not a reason to fail
/// the `say` itself (the reply already happened; logging is observability).
fn append_talklog(state: &HubState, label: &str, session: &str, line: &TalkLine) {
    let _ = std::fs::create_dir_all(talklog_dir(state));
    let path = talklog_path(state, label, session);
    let json = match line {
        TalkLine::Prompt { prompt_id, text } => serde_json::json!({
            "ts": holler_proto::log::timestamp(),
            "prompt_id": prompt_id,
            "text": text,
        }),
        TalkLine::Update { prompt_id, seq, parts, ts } => serde_json::json!({
            "ts": ts,
            "prompt_id": prompt_id,
            "seq": seq,
            "parts": parts_to_json(parts),
        }),
        TalkLine::Done { prompt_id, stop_reason } => serde_json::json!({
            "ts": holler_proto::log::timestamp(),
            "prompt_id": prompt_id,
            "stopReason": stop_reason,
        }),
    };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{json}");
    }
}

fn parts_to_json(parts: &[Part]) -> serde_json::Value {
    serde_json::to_value(parts).unwrap_or(serde_json::Value::Null)
}

/// The reply text (concatenation of its text parts) — what `say` prints on
/// stdout without `--json`.
pub fn reply_text(message: &Message) -> String {
    message
        .parts
        .iter()
        .filter_map(|p| match &p.content {
            Some(Content::Text(t)) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
