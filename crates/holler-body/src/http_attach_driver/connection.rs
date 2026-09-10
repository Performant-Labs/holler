//! The background task behind [`super::HttpAttachDriver`] (issue #194):
//! sends prompts via `prompt_async`, reads the global `/event` SSE stream
//! (reconnecting with the hub-circuit backoff schedule while a turn is
//! running), and polls `/question`/`/permission` for the attached session
//! and its child sessions while a turn is running. Split out of
//! `http_attach_driver.rs` to stay under the workspace's 900-line-per-file
//! guard, mirroring `acp_driver`'s own `connection.rs` split.
//!
//! # Route forms (independently confirmed live — see `http_attach_driver.rs`'s
//! module doc for how)
//!
//! `prompt_async` is always bare (`/session/{id}/prompt_async` — the `/api`
//! form silently falls through to the SPA's HTML shell); `interrupt` is
//! always `/api`-prefixed (`/api/session/{id}/interrupt` — the bare form
//! silently falls through the same way). `/event`, `/question`, and
//! `/permission` are always bare (global, not session-scoped at all).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

use holler_proto::log::{Component, Direction as LogDirection, Event as LogEvent, Severity};

use crate::acp_driver::{DriverError, DriverEvent, DriverState, Status, StopReason};

use super::wire::{
    normalize_permission_reply, resolve_question_choices, BlockKind, OcEvent,
    OcPermissionRequest, OcQuestionRequest, PendingBlock,
};

/// How often the background loop polls `/question` and `/permission` while a
/// turn is running (issue #194/#382).
pub(super) const BLOCK_POLL_INTERVAL: Duration = Duration::from_millis(1_000);

pub(super) enum Command {
    Prompt(String),
}

/// State shared between [`super::HttpAttachDriver`]'s public methods and the
/// background loop, mirroring `acp_driver::connection::Shared`'s shape.
pub(super) struct Shared {
    pub(super) status: Status,
    pub(super) current_events: Option<mpsc::UnboundedSender<DriverEvent>>,
    pub(super) pending: Option<PendingBlock>,
    pub(super) interrupt_requested: bool,
    pub(super) last_stop_reason: Option<StopReason>,
    pub(super) awaiting_done: Option<oneshot::Sender<StopReason>>,
}

pub(super) fn lock(shared: &Arc<Mutex<Shared>>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether a turn is currently running — `Working` *or* `InputRequired`
/// (paused for a question/permission answer, but still "running" in the
/// sense this issue's spec means: the SSE reconnect loop and the
/// question/permission poll loop both stay active through an
/// input-required pause, not just plain `Working`). Only `Idle` turns this
/// off.
fn is_working(shared: &Arc<Mutex<Shared>>) -> bool {
    !matches!(lock(shared).status, Status::Idle)
}

type ByteStream = std::pin::Pin<Box<dyn Stream<Item = reqwest::Result<Vec<u8>>> + Send>>;

/// One real session as returned by `GET {endpoint}/session` (bare, never
/// prefixed — matching legacy's `list_sessions`), used here only to discover
/// child sessions of the attached one (issue #382's child-session caveat).
#[derive(Debug, Deserialize)]
struct OcSessionSummary {
    id: String,
    #[serde(rename = "parentID", default)]
    parent_id: Option<String>,
}

fn log_debug(method: &'static str, fields: Vec<(&'static str, String)>, frame: Option<String>) {
    holler_proto::log::emit(&LogEvent {
        component: Component::HttpAttach,
        severity: Severity::Debug,
        direction: LogDirection::Out,
        method,
        id: None,
        peer: None,
        fields,
        frame,
    });
}

fn log_local(method: &'static str, fields: Vec<(&'static str, String)>) {
    holler_proto::log::emit(&LogEvent {
        component: Component::HttpAttach,
        severity: Severity::Debug,
        direction: LogDirection::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

fn log_warn(method: &'static str, fields: Vec<(&'static str, String)>) {
    holler_proto::log::emit(&LogEvent {
        component: Component::HttpAttach,
        severity: Severity::Warn,
        direction: LogDirection::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

/// Milliseconds elapsed since `started`, as a log field value — the `ms` half
/// of the spec's "each HTTP call (method, path, status, ms)" (issue #197).
fn elapsed_ms(started: Instant) -> String {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX).to_string()
}

/// Drives one attached session for its whole lifetime (from `attach()` until
/// [`super::HttpAttachDriver::shutdown`] drops the command channel).
pub(super) async fn run(
    client: reqwest::Client,
    endpoint: String,
    session_id: String,
    mut command_rx: mpsc::UnboundedReceiver<Command>,
    shared: Arc<Mutex<Shared>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let mut message_roles: HashMap<String, String> = HashMap::new();
    let mut sse_stream: Option<ByteStream> = None;
    let mut buffer = String::new();
    let mut attempt: u32 = 0;
    // Whether the very first SSE connect has ever succeeded. Distinguishes
    // "bootstrapping" (always connect right away, no matter the turn state —
    // events must be ready essentially the moment `attach()` returns) from
    // "reconnecting after a drop" (only while a turn is running, per this
    // issue's own spec) — `attempt` alone can't carry that distinction since
    // it resets to 0 on every successful connect, including a reconnect.
    let mut ever_connected = false;
    let mut child_sessions: Vec<String> = Vec::new();

    let mut poll_interval = tokio::time::interval(BLOCK_POLL_INTERVAL);
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        let reconnect_wanted = sse_stream.is_none() && (!ever_connected || is_working(&shared));
        // Bootstrapping (never connected yet) happens with no delay — events
        // must be ready essentially the moment `attach()` returns, matching
        // the legacy driver's own synchronous connect. Every reconnect after
        // a drop (while a turn is running) uses the real jittered backoff.
        let delay = if ever_connected {
            crate::backoff::delay_random(attempt)
        } else {
            Duration::ZERO
        };

        tokio::select! {
            biased;

            _ = &mut shutdown_rx => return,

            cmd = command_rx.recv() => {
                match cmd {
                    None => return,
                    Some(Command::Prompt(text)) => {
                        send_prompt(&client, &endpoint, &session_id, &text, &shared).await;
                    }
                }
            }

            chunk = async {
                match sse_stream.as_mut() {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            }, if sse_stream.is_some() => {
                let still_connected = process_sse_chunk(
                    chunk,
                    &mut buffer,
                    &session_id,
                    &mut message_roles,
                    &shared,
                );
                if !still_connected {
                    sse_stream = None;
                }
            }

            _ = tokio::time::sleep(delay), if reconnect_wanted => {
                if let Some(stream) = connect_sse(&client, &endpoint).await {
                    sse_stream = Some(stream);
                    attempt = 0;
                    ever_connected = true;
                    log_local("sse", vec![("event", "connected".to_string())]);
                } else {
                    attempt = attempt.saturating_add(1);
                    log_warn("sse", vec![("event", "connect_failed".to_string()), ("attempt", attempt.to_string())]);
                }
            }

            _ = poll_interval.tick(), if is_working(&shared) => {
                refresh_child_sessions(&client, &endpoint, &session_id, &mut child_sessions).await;
                poll_pending_block(&client, &endpoint, &session_id, &child_sessions, &shared).await;
            }
        }
    }
}

/// Handles one poll of the SSE byte stream: appends `chunk`'s bytes to
/// `buffer`, splits complete `\n\n`-terminated frames off it, and dispatches
/// every `data: {json}` line matching `session_id` to [`handle_event`].
/// Returns `false` when the stream ended/errored (the caller should treat
/// this as a disconnect and clear its `Option<ByteStream>`), extracted out
/// of [`run`]'s own `tokio::select!` body to keep that function's cognitive
/// complexity under the workspace's clippy gate.
fn process_sse_chunk(
    chunk: Option<reqwest::Result<Vec<u8>>>,
    buffer: &mut String,
    session_id: &str,
    message_roles: &mut HashMap<String, String>,
    shared: &Arc<Mutex<Shared>>,
) -> bool {
    let Some(Ok(bytes)) = chunk else {
        log_warn("sse", vec![("event", "disconnected".to_string())]);
        return false;
    };
    buffer.push_str(&String::from_utf8_lossy(&bytes));
    while let Some(pos) = buffer.find("\n\n") {
        let frame = buffer[..pos].to_string();
        buffer.drain(..pos + 2);
        for line in frame.lines() {
            let Some(json_str) = line.strip_prefix("data: ") else { continue };
            let Ok(event) = serde_json::from_str::<OcEvent>(json_str) else { continue };
            if event.properties.session_id.as_deref() != Some(session_id) {
                continue;
            }
            log_debug("sse_event", vec![("event_type", event.kind.clone())], None);
            handle_event(event, message_roles, shared);
        }
    }
    true
}

async fn connect_sse(client: &reqwest::Client, endpoint: &str) -> Option<ByteStream> {
    let url = format!("{}/event", endpoint.trim_end_matches('/'));
    log_debug("sse_connect", vec![("path", "/event".to_string())], None);
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(err) => {
            log_warn("sse_connect", vec![("event", format!("error: {err}"))]);
            return None;
        }
    };
    if !response.status().is_success() {
        log_warn(
            "sse_connect",
            vec![("event", format!("non-success status: {}", response.status()))],
        );
        return None;
    }
    Some(Box::pin(response.bytes_stream().map_ok(|b| b.to_vec())))
}

async fn send_prompt(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    text: &str,
    shared: &Arc<Mutex<Shared>>,
) {
    // Always bare — confirmed live that `/api/session/{id}/prompt_async` is
    // not a real route (see `http_attach_driver.rs`'s module doc).
    let url = format!("{}/session/{session_id}/prompt_async", endpoint.trim_end_matches('/'));
    let body = serde_json::json!({ "parts": [{"type": "text", "text": text}] });
    log_debug(
        "prompt_async",
        vec![("method", "POST".to_string()), ("path", format!("/session/{session_id}/prompt_async"))],
        holler_proto::log::frame_at_noisy(&body.to_string()),
    );
    lock(shared).status = Status::Working;
    let started = Instant::now();
    match client.post(&url).json(&body).send().await {
        Ok(response) => {
            log_debug(
                "prompt_async",
                vec![
                    ("status", response.status().as_str().to_string()),
                    ("ms", elapsed_ms(started)),
                ],
                None,
            );
        }
        Err(err) => {
            log_warn("prompt_async", vec![("event", format!("error: {err}")), ("ms", elapsed_ms(started))]);
        }
    }
}

/// Refreshes `child_sessions` from `GET {endpoint}/session` (bare — issue
/// #382's child-session caveat): every real session whose `parentID` is the
/// attached session. Best-effort — a failed listing just leaves the
/// previous (possibly stale) child set in place rather than erroring the
/// whole poll tick.
async fn refresh_child_sessions(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    child_sessions: &mut Vec<String>,
) {
    let url = format!("{}/session", endpoint.trim_end_matches('/'));
    let Ok(response) = client.get(&url).send().await else { return };
    if !response.status().is_success() {
        return;
    }
    let Ok(sessions) = response.json::<Vec<OcSessionSummary>>().await else { return };
    *child_sessions = sessions
        .into_iter()
        .filter(|s| s.parent_id.as_deref() == Some(session_id))
        .map(|s| s.id)
        .collect();
}

/// One poll tick of the question/permission detection loop (issue #382): the
/// attached session and its known child sessions are all in scope. A poll
/// that finds nothing new (same pending id as last tick, or still nothing
/// pending) is silent: no duplicate state transition every tick.
async fn poll_pending_block(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    child_sessions: &[String],
    shared: &Arc<Mutex<Shared>>,
) {
    let found = poll_pending_question(client, endpoint, session_id, child_sessions)
        .await
        .or(poll_pending_permission(client, endpoint, session_id, child_sessions).await);

    let mut guard = lock(shared);
    let previous_id = guard.pending.as_ref().map(|p| p.id.clone());
    let found_id = found.as_ref().map(|p| p.id.clone());
    if previous_id == found_id {
        return;
    }

    let became_blocked = found.is_some();
    if let Some(block) = &found {
        log_local(
            "answer",
            vec![
                ("event", "blocked".to_string()),
                ("session", block.session_id.clone()),
            ],
        );
    }
    guard.pending = found;
    if became_blocked {
        guard.status = Status::InputRequired;
        if let Some(tx) = guard.current_events.as_ref() {
            let _ = tx.send(DriverEvent::State(DriverState::InputRequired));
        }
    } else {
        guard.status = Status::Working;
        if let Some(tx) = guard.current_events.as_ref() {
            let _ = tx.send(DriverEvent::State(DriverState::Working));
        }
    }
}

fn session_in_scope(candidate: &str, session_id: &str, child_sessions: &[String]) -> bool {
    candidate == session_id || child_sessions.iter().any(|c| c == candidate)
}

async fn poll_pending_question(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    child_sessions: &[String],
) -> Option<PendingBlock> {
    let url = format!("{}/question", endpoint.trim_end_matches('/'));
    log_debug("question_poll", vec![("path", "/question".to_string())], None);
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let requests: Vec<OcQuestionRequest> = response.json().await.ok()?;
    let request = requests
        .into_iter()
        .find(|r| session_in_scope(&r.session_id, session_id, child_sessions))?;
    if request.questions.is_empty() {
        return None;
    }
    let prompt = request
        .questions
        .first()
        .map(|q| q.question.clone())
        .unwrap_or_default();
    let options: Vec<Vec<String>> = request
        .questions
        .iter()
        .map(|q| q.options.iter().map(|o| o.label.clone()).collect())
        .collect();
    Some(PendingBlock {
        kind: BlockKind::Question,
        id: request.id,
        session_id: request.session_id,
        prompt,
        options,
    })
}

async fn poll_pending_permission(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
    child_sessions: &[String],
) -> Option<PendingBlock> {
    let url = format!("{}/permission", endpoint.trim_end_matches('/'));
    log_debug("permission_poll", vec![("path", "/permission".to_string())], None);
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let requests: Vec<OcPermissionRequest> = response.json().await.ok()?;
    let request = requests
        .into_iter()
        .find(|r| session_in_scope(&r.session_id, session_id, child_sessions))?;
    Some(PendingBlock {
        kind: BlockKind::Permission,
        id: request.id,
        session_id: request.session_id,
        prompt: request.permission,
        options: Vec::new(),
    })
}

fn handle_event(event: OcEvent, message_roles: &mut HashMap<String, String>, shared: &Arc<Mutex<Shared>>) {
    match event.kind.as_str() {
        "message.updated" => {
            if let Some(info) = event.properties.info {
                message_roles.insert(info.id, info.role);
            }
        }
        "message.part.updated" => {
            if let Some(part) = event.properties.part {
                if part.kind == "text" {
                    let is_assistant = message_roles
                        .get(&part.message_id)
                        .is_some_and(|role| role == "assistant");
                    if is_assistant {
                        if let Some(text) = part.text {
                            let guard = lock(shared);
                            if let Some(tx) = guard.current_events.as_ref() {
                                let _ = tx.send(DriverEvent::Chunk(text));
                            }
                        }
                    }
                }
            }
        }
        "session.status" => {
            if let Some(status) = event.properties.status {
                if status.kind == "busy" {
                    let mut guard = lock(shared);
                    guard.status = Status::Working;
                    if let Some(tx) = guard.current_events.as_ref() {
                        let _ = tx.send(DriverEvent::State(DriverState::Working));
                    }
                }
                // Deliberately not reacting to `status.kind == "idle"` here:
                // the distinct `session.idle`/`session.error` events below are
                // the single, unambiguous "this turn is over" signal.
            }
        }
        "session.idle" => finish_turn(shared, StopReason::EndTurn),
        "session.error" => finish_turn(shared, StopReason::Error),
        _ => {}
    }
}

/// Settles the current turn with `reason` (unless an interrupt was requested,
/// in which case the real reason is overridden to `Cancelled` — OpenCode's
/// event stream has no cancellation-specific event of its own): updates
/// `last_stop_reason`, fulfils a `cancel()` waiter if one is registered, and
/// emits `DriverEvent::Done` on the current prompt's event channel.
fn finish_turn(shared: &Arc<Mutex<Shared>>, reason: StopReason) {
    let mut guard = lock(shared);
    let was_cancelled = std::mem::take(&mut guard.interrupt_requested);
    let resolved = if was_cancelled { StopReason::Cancelled } else { reason };
    guard.status = Status::Idle;
    guard.last_stop_reason = Some(resolved);
    if let Some(done_tx) = guard.awaiting_done.take() {
        let _ = done_tx.send(resolved);
    }
    if let Some(tx) = guard.current_events.take() {
        let _ = tx.send(DriverEvent::Done(resolved));
    }
}

/// Direct POST for [`super::HttpAttachDriver::answer`] — bypasses the
/// background loop entirely, the same pattern
/// [`super::HttpAttachDriver::cancel`] already uses for its own direct POST.
pub(super) async fn post_answer(
    client: &reqwest::Client,
    endpoint: &str,
    pending: &PendingBlock,
    choice: &str,
) -> Result<(), DriverError> {
    match pending.kind {
        BlockKind::Permission => {
            let Some(reply) = normalize_permission_reply(choice) else {
                return Err(DriverError::Answer(format!(
                    "invalid permission choice {choice:?}; expected one of once/allow, always, reject/deny"
                )));
            };
            let url = format!("{}/permission/{}/reply", endpoint.trim_end_matches('/'), pending.id);
            let body = serde_json::json!({ "reply": reply });
            post_reply(client, &url, &body, "answer_permission").await
        }
        BlockKind::Question => {
            let Some(labels) = resolve_question_choices(choice, &pending.options) else {
                return Err(DriverError::Answer(format!(
                    "invalid question choice {choice:?}; expected {} comma-separated choice(s)",
                    pending.options.len()
                )));
            };
            let url = format!("{}/question/{}/reply", endpoint.trim_end_matches('/'), pending.id);
            let answers: Vec<Vec<&str>> = labels.iter().map(|l| vec![l.as_str()]).collect();
            let body = serde_json::json!({ "answers": answers });
            post_reply(client, &url, &body, "answer_question").await
        }
    }
}

async fn post_reply(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    method: &'static str,
) -> Result<(), DriverError> {
    log_debug(
        method,
        vec![("method", "POST".to_string()), ("path", url.to_string())],
        holler_proto::log::frame_at_noisy(&body.to_string()),
    );
    let started = Instant::now();
    let response = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|err| DriverError::Rpc(err.to_string()))?;
    log_debug(
        method,
        vec![("status", response.status().as_str().to_string()), ("ms", elapsed_ms(started))],
        None,
    );
    if !response.status().is_success() {
        return Err(DriverError::Rpc(format!("{method} returned HTTP {}", response.status())));
    }
    Ok(())
}

/// Direct POST for [`super::HttpAttachDriver::cancel`]'s interrupt call.
pub(super) async fn post_interrupt(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
) -> Result<(), DriverError> {
    // Always `/api`-prefixed — confirmed live that the bare
    // `/session/{id}/interrupt` is not a real route (see
    // `http_attach_driver.rs`'s module doc).
    let url = format!("{}/api/session/{session_id}/interrupt", endpoint.trim_end_matches('/'));
    log_debug(
        "interrupt",
        vec![("method", "POST".to_string()), ("path", format!("/api/session/{session_id}/interrupt"))],
        None,
    );
    let started = Instant::now();
    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|err| DriverError::Rpc(err.to_string()))?;
    log_debug(
        "interrupt",
        vec![("status", response.status().as_str().to_string()), ("ms", elapsed_ms(started))],
        None,
    );
    if !response.status().is_success() {
        return Err(DriverError::Rpc(format!("interrupt returned HTTP {}", response.status())));
    }
    Ok(())
}
