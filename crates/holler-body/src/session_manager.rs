//! Per-session tasks, the busy-turn queue, isolated cancel, and post-cancel
//! freshness (issue #189) — the runtime half of the busy-turn policy
//! (issue #150; the wire vocabulary itself landed in `holler-proto` via
//! PR #180).
//!
//! One tokio task per session, each owning its own [`crate::acp_driver::AcpDriver`]
//! and an `mpsc` mailbox of [`SessionCommand`]. **No shared mutable state
//! between session tasks** — the only thing two sessions' tasks ever touch in
//! common is a clone of the presence-changed [`broadcast`] sender (a cheap,
//! lock-free handle, not shared mutable state) and, for a caller of
//! [`SessionManager::presence_doc`], its own session's `Arc<Mutex<..>>`
//! presence snapshot (written by exactly one task, read by any number of
//! outside callers — never written by a second session's task).
//!
//! # Decisions I made
//!
//! - **Test file location**: the issue's own setup instructions name
//!   `crates/holler-body/tests/session_manager_test.rs`. Exactly like #188
//!   before it, that path is not reachable: the stub-acp process the tests
//!   spawn is a `holler-cli` `[[bin]]`, and `CARGO_BIN_EXE_stub-acp` only
//!   resolves inside `holler-cli`'s own test targets (see
//!   `crates/holler-cli/tests/acp_driver_test.rs`'s module doc for the full
//!   reasoning — it hit this exact wall first). This story's tests live at
//!   `crates/holler-cli/tests/session_manager_test.rs`, declared as its own
//!   `[[test]]` target, following that precedent.
//! - **`connection.rs` wiring is out of scope for this story.** `holler body
//!   run`'s connection loop (issue #182) has no incoming-request dispatch
//!   loop at all yet — it only ever *sends* (`session/presence` heartbeats);
//!   there is no `session/prompt` handler to plug a `SessionManager` into.
//!   Building that dispatch loop is real, separate work (naturally the
//!   "Talk" story, #190) — bolting a half-specified wire handler onto this
//!   story would mean guessing at #190's own design. This story ships
//!   `SessionManager` as a clean, fully unit-tested surface
//!   (`prompt`/`cancel`/`answer`/`replace`/`presence_doc`) that #190 drives
//!   directly; `connection::run` still takes its static `Vec<SessionAd>`
//!   snapshot for now.
//! - **`SessionCommand::Prompt`'s `reply_tx` resolves once, when the turn
//!   ends** (mirroring `holler_proto::docs::PromptResult`'s own doc comment:
//!   "sent **exactly once**, when the turn ends") — including for a *queued*
//!   prompt, whose caller (the future `session/prompt --queue` wire handler)
//!   is expected to hold the request open until its own turn eventually
//!   finishes, exactly like an immediately-dispatched one. Only an immediate
//!   refusal (`-32009 session_busy`, `-32007`-shaped queue-full) resolves the
//!   reply before any turn runs. This is *not* an ack-then-poll design.
//! - **`SessionManager::start` takes `&SessionRegistry`**, not an owned one
//!   (the issue's own sketch, `start(registry) -> SessionManager`, does not
//!   pin the ownership). A caller (the future `body run` wiring) keeps its
//!   registry around for other uses (e.g. building the CLI's `session/query`
//!   answers later); consuming it here would force an awkward clone at the
//!   call site for no benefit — this story only ever reads `SessionConfig`
//!   out of it once, at spawn time.
//! - **Presence's `pending` field** is populated from `AcpDriver::pending`
//!   (issue #151) via `task::set_state` every time a session's task moves to
//!   (or stays in) `InputRequired`, and cleared the instant it leaves that
//!   state — this story only adds the `working`/`input-required` timing
//!   fields and `turn_id`/`last_turn` issue #150/#142 already reserved room
//!   for on the wire type; `pending` itself landed with issue #151.
//! - **A driver crash (`StopReason::Error`) drops the `AcpDriver`** so the
//!   *next* `Prompt`/`Replace` command respawns a fresh one (the issue's own
//!   "restarts the driver on the next prompt" — not eagerly, and not as a
//!   retry of the failed turn itself, which is reported to its own caller as
//!   a normal terminal `PromptOutcome::Result{state: Failed, ..}`).

mod task;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use holler_proto::{Mode, Presence, SessionAd, SessionName, SessionState};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::config::{SessionConfig, SessionMode};
use crate::registry::SessionRegistry;
use task::SessionPresence;

/// The bounded FIFO queue depth for `queue:true` prompts (the issue's own
/// number). A 65th queued prompt for the same session is refused outright.
const QUEUE_CAP: usize = 64;
/// Per-session command mailbox capacity. Generous relative to `QUEUE_CAP`
/// (commands are small and short-lived — only a queued *prompt* is held for
/// any length of time, and that lives in the task's own `VecDeque`, not in
/// this channel) so a legitimate caller never sees mailbox backpressure as a
/// spurious refusal.
const MAILBOX_CAP: usize = 256;
/// How long [`SessionManager::shutdown`] waits for one session's task to end
/// (its own `AcpDriver::shutdown` is already bounded, ~2s) before moving on.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// One command a session's task consumes from its own mailbox. No variant
/// here is ever sent to (or read by) a *different* session's task.
pub enum SessionCommand {
    /// A plain or `--queue` prompt. `id` is the JSON-RPC id of the
    /// `session/prompt` call this belongs to (surfaced back on
    /// `presence_doc()`'s `turn_id`/`last_turn` once it resolves).
    Prompt {
        id: String,
        text: String,
        queue: bool,
        reply_tx: oneshot::Sender<PromptOutcome>,
        /// Mid-turn text chunks, forwarded as the driver emits them (issue
        /// #190's coalescer consumes this to build `session/update`
        /// notifications). `None` for a caller with no streaming consumer
        /// (e.g. the existing #189 unit tests) — chunks are then simply
        /// dropped, exactly like before this story.
        updates: Option<mpsc::UnboundedSender<String>>,
    },
    /// Cancel the in-flight turn only; queued prompts are untouched.
    Cancel { reply_tx: oneshot::Sender<Result<(), String>> },
    /// Resolve a held permission/elicitation. Replies `Ok(())` only once the
    /// driver reports `Working` again (or the turn ends outright).
    Answer {
        choice: String,
        reply_tx: oneshot::Sender<Result<(), String>>,
    },
    /// Cancel the in-flight turn (if any), then run `text` **ahead of** the
    /// queue — what `interrupt SESSION TEXT` maps to.
    Replace {
        text: String,
        reply_tx: oneshot::Sender<PromptOutcome>,
    },
    /// End this session's task: gracefully shut down its driver (if any) and
    /// return. Queued prompts are dropped without a reply (their `reply_tx`
    /// is simply never fired) — a shutting-down body has nowhere to deliver
    /// that answer anyway.
    Shutdown,
}

/// How a [`SessionCommand::Prompt`] or [`SessionCommand::Replace`] resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptOutcome {
    /// `-32009 session_busy`: refused immediately (`queue:false` hit a
    /// `working`/`input-required` session).
    Busy {
        state: String,
        turn_age_ms: u64,
        last_update_age_ms: u64,
    },
    /// `-32007 limit_exceeded`: the bounded FIFO queue was already full.
    QueueFull,
    /// The turn ran (immediately, or after sitting in the queue) and ended.
    Result {
        turn_id: String,
        stop_reason: String,
        state: SessionState,
    },
    /// The driver itself could not be brought up (or the connection ended
    /// mid-answer-wait) — reported as a one-line reason, never silently
    /// dropped.
    Error(String),
}

/// Why a call into [`SessionManager`] could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionManagerError {
    /// No session by that name is registered.
    UnknownSession,
    /// The session's task has already ended (mailbox closed, or the reply
    /// channel was dropped before answering).
    Gone,
}

impl std::fmt::Display for SessionManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSession => f.write_str("unknown session"),
            Self::Gone => f.write_str("session task is gone"),
        }
    }
}

impl std::error::Error for SessionManagerError {}

/// One registered session's manager-side handle: its (immutable) config, the
/// mailbox to its task, and the read-only presence snapshot that task keeps
/// current.
struct ManagedSession {
    config: SessionConfig,
    tx: mpsc::Sender<SessionCommand>,
    presence: Arc<Mutex<SessionPresence>>,
    join: JoinHandle<()>,
}

/// One task per registered session, each with its own mailbox and driver.
pub struct SessionManager {
    sessions: BTreeMap<SessionName, ManagedSession>,
    presence_tx: broadcast::Sender<SessionName>,
}

impl SessionManager {
    /// Spawn one task per session in `registry`. Every session starts `idle`
    /// with no driver up yet — the first `Prompt`/`Replace` it receives
    /// spawns the real `AcpDriver` (spawn-mode session config; see the
    /// module's own #188 dependency).
    pub fn start(registry: &SessionRegistry) -> Self {
        // Capacity is advisory (lossy) for a broadcast channel: a lagging
        // subscriber misses old presence-changed notices, never blocks a
        // session task's own send. 64 headroom for however many
        // sessions/changes land between two drains of a slow subscriber.
        let (presence_tx, _first_rx) = broadcast::channel(64);
        let mut sessions = BTreeMap::new();
        for (name, entry) in registry.iter() {
            let presence = Arc::new(Mutex::new(SessionPresence::default()));
            let (tx, rx) = mpsc::channel(MAILBOX_CAP);
            let inner = task::Inner::new(
                entry.config.clone(),
                presence.clone(),
                presence_tx.clone(),
                name.clone(),
            );
            let join = tokio::spawn(task::run(rx, inner));
            sessions.insert(
                name.clone(),
                ManagedSession { config: entry.config.clone(), tx, presence, join },
            );
        }
        Self { sessions, presence_tx }
    }

    /// Subscribe to "some session's presence changed" notices — the signal a
    /// connection loop (issue #182's `connection::run`, or its #190
    /// successor) uses to push `session/presence` immediately on any state
    /// change, rather than waiting for the next heartbeat. Lossy under load
    /// (a `broadcast` lag skips ahead) — a subscriber that misses one still
    /// gets the *next* change, and `presence_doc()` always reflects the
    /// latest state regardless of what this stream delivered.
    pub fn subscribe_presence_changes(&self) -> broadcast::Receiver<SessionName> {
        self.presence_tx.subscribe()
    }

    /// Send a prompt. `id` is the caller's JSON-RPC request id (echoed back
    /// in the eventual `PromptOutcome::Result`/`presence_doc()`'s `turn_id`).
    pub async fn prompt(
        &self,
        name: &SessionName,
        id: impl Into<String>,
        text: impl Into<String>,
        queue: bool,
    ) -> Result<PromptOutcome, SessionManagerError> {
        self.prompt_with_updates(name, id, text, queue, None).await
    }

    /// Send a prompt with a live channel for mid-turn text chunks (issue
    /// #190's `session/prompt` dispatch: the connection loop drains
    /// `updates` through a [`crate::reply_coalescer::Coalescer`] while
    /// awaiting the final [`PromptOutcome`] this call resolves to). A queued
    /// prompt's chunks arrive once its own turn is actually dispatched, same
    /// as an immediate one — nothing streams while it sits in the FIFO.
    pub async fn prompt_with_updates(
        &self,
        name: &SessionName,
        id: impl Into<String>,
        text: impl Into<String>,
        queue: bool,
        updates: Option<mpsc::UnboundedSender<String>>,
    ) -> Result<PromptOutcome, SessionManagerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.send(
            name,
            SessionCommand::Prompt { id: id.into(), text: text.into(), queue, reply_tx, updates },
        )
        .await?;
        reply_rx.await.map_err(|_| SessionManagerError::Gone)
    }

    /// Cancel the in-flight turn only (queued prompts are untouched).
    pub async fn cancel(&self, name: &SessionName) -> Result<Result<(), String>, SessionManagerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.send(name, SessionCommand::Cancel { reply_tx }).await?;
        reply_rx.await.map_err(|_| SessionManagerError::Gone)
    }

    /// Resolve a held permission/elicitation.
    pub async fn answer(
        &self,
        name: &SessionName,
        choice: impl Into<String>,
    ) -> Result<Result<(), String>, SessionManagerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.send(name, SessionCommand::Answer { choice: choice.into(), reply_tx }).await?;
        reply_rx.await.map_err(|_| SessionManagerError::Gone)
    }

    /// Cancel the in-flight turn (if any) and run `text` ahead of the queue —
    /// `interrupt SESSION TEXT`.
    pub async fn replace(
        &self,
        name: &SessionName,
        text: impl Into<String>,
    ) -> Result<PromptOutcome, SessionManagerError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.send(name, SessionCommand::Replace { text: text.into(), reply_tx }).await?;
        reply_rx.await.map_err(|_| SessionManagerError::Gone)
    }

    async fn send(&self, name: &SessionName, cmd: SessionCommand) -> Result<(), SessionManagerError> {
        let session = self.sessions.get(name).ok_or(SessionManagerError::UnknownSession)?;
        session.tx.send(cmd).await.map_err(|_| SessionManagerError::Gone)
    }

    /// Build this body's `session/presence` params from every session's live
    /// state — the dynamic replacement for
    /// [`crate::registry::SessionRegistry::presence_doc`]'s always-`idle`
    /// snapshot (that one has no driver behind it yet; this one does).
    pub fn presence_doc(&self, hostname: String) -> Presence {
        let sessions = self
            .sessions
            .iter()
            .map(|(name, managed)| {
                let p = managed.presence.lock().unwrap_or_else(PoisonError::into_inner);
                SessionAd {
                    name: name.as_str().to_string(),
                    harness: managed.config.harness.clone(),
                    state: p.state,
                    mode: match managed.config.mode {
                        SessionMode::Spawn => Mode::Spawn,
                        SessionMode::Attach => Mode::Attach,
                    },
                    harness_session_id: managed.config.session_id.clone(),
                    turn_started_at: p.turn_started_at.clone(),
                    last_update_at: p.last_update_at.clone(),
                    pending: p.pending.clone(),
                    turn_id: p.turn_id.clone(),
                    last_turn: p.last_turn.clone(),
                }
            })
            .collect();
        Presence { hostname, sessions }
    }

    /// Shut down every session's task: each gets a `Shutdown` command (its
    /// own driver, if up, gets a graceful `AcpDriver::shutdown`), bounded by
    /// [`SHUTDOWN_GRACE`] per session so one wedged session cannot hang the
    /// whole body's exit.
    pub async fn shutdown(self) {
        for (_, session) in self.sessions {
            let _ = session.tx.send(SessionCommand::Shutdown).await;
            let _ = tokio::time::timeout(SHUTDOWN_GRACE, session.join).await;
        }
    }
}
