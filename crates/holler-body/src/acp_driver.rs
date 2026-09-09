//! The ACP v2 spawn driver (issue #188): launch the configured harness as a
//! child over stdio using the official `agent-client-protocol` 2.1.0 typed
//! client, run prompts, stream updates, cancel cooperatively, and surface the
//! ACP v2 `requires_action` state as a real, answerable `blocked` state.
//!
//! Split across three files to stay under the workspace's 900-line-per-file
//! guard (`scripts/lint.sh`): this file owns the public [`AcpDriver`] API;
//! `connection.rs` owns the background connection task and the state shared
//! with its concurrently-running SDK handlers; `pending.rs` owns building and
//! resolving a held-open permission/elicitation request; `answerable.rs` owns
//! the pure index-or-label resolution logic.
//!
//! # Decisions I made (documented per the story's own instructions)
//!
//! - **`session/prompt`'s response carries no `stopReason` in the real 2.1.0
//!   crate.** The issue's spec describes the prompt response itself resolving
//!   to `{stopReason}`; the actual pinned schema (`agent-client-protocol-schema`
//!   1.7.0, `v2::PromptResponse`) has no such field — completion is reported
//!   independently via a `state_update` `idle` notification carrying
//!   `stopReason` (confirmed against the crate's own `v2_one_shot_client`
//!   example, which derives completion the same way). [`DriverEvent::Done`]
//!   is therefore driven off that `idle` notification, not off the prompt
//!   request's own response. The `tests/stub-acp` fixture (issue #130) was
//!   fixed in this story to emit that notification (it previously only put
//!   `stopReason` on the prompt response, which a real ACP v2 client never
//!   reads for this purpose) — see `send_idle_state` there.
//! - **The stub's `initialize` response used the wrong field name.** The real
//!   `v2::AgentCapabilities` type deserializes the session surface under
//!   `capabilities.session`, not a top-level `agentCapabilities` key. Fixed in
//!   the same stub-fixture pass.
//! - **`stop_reason_to_task_state_mapping_is_total` does exist** (in
//!   `holler-proto`'s `codec_test.rs`, against `holler_proto::docs::
//!   state_for_stop_reason` / the `STOP_TO_STATE` table) — reused, not
//!   reinvented, exactly as the issue asks. This story does not call it
//!   itself: [`StopReason`] carries the ACP wire vocabulary 1:1
//!   (`end_turn|cancelled|max_tokens|max_turn_requests|refusal`, plus a
//!   driver-local `error` for a crashed/failed turn — `state_for_stop_reason`
//!   already has an `"error"` entry for exactly this) and exposes it via
//!   [`StopReason::as_wire_str`]. The session-manager story (#189), the actual
//!   consumer of [`DriverEvent::Done`], is where
//!   `state_for_stop_reason(event.as_wire_str())` belongs — this crate has no
//!   A2A task/session-state concept of its own to hang that call off yet.
//! - **Attach mode is out of scope.** [`AcpDriver::spawn`] only implements
//!   `SessionMode::Spawn` (the issue's own title: "ACP v2 **spawn** driver").
//!   An `attach`-mode `SessionConfig` is refused with `DriverError::Startup`.
//! - **The `interrupt = "http"` fallback is not implemented in this pass.**
//!   None of the issue's own RED test list exercises it, and it needs an HTTP
//!   client dependency this story did not otherwise need. `cancel()` always
//!   uses the ACP `session/cancel` path. Flagged as a follow-up, not silently
//!   dropped.
//! - **Elicitation field order is the schema's `BTreeMap` (property-name sort)
//!   order**, not source declaration order — `v2::ElicitationSchema::properties`
//!   is a `BTreeMap<String, _>`, so declaration order is not recoverable from
//!   the typed request at all. A multi-field `answer()` resolves segments
//!   against fields in that sorted order; the stub fixture names its fields so
//!   the sort order is the natural one (`color`, then `size`).
//! - **A multi-select elicitation field accepts exactly one selection per
//!   comma segment** (wrapped as a one-element `StringArray` on reply). The
//!   RED list's own wording — "one segment per field" — does not ask for a
//!   second delimiter for multiple selections *within* one multi-select field;
//!   supporting that is a natural follow-up, not required here.
//! - **An "unsupported" elicitation (`Url` mode, or any non-enum form field)
//!   is still surfaced as a normal pending block** (status flips to
//!   `InputRequired`, a [`DriverEvent::State`] fires immediately) so it is
//!   never silently dropped — but [`AcpDriver::answer`] against it always
//!   fails closed with [`DriverError::Unsupported`] (no reply is ever sent on
//!   the caller's behalf). `cancel()` still resolves it (`Cancel` action),
//!   same as any other pending block.

mod answerable;
mod connection;
mod pending;

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use agent_client_protocol::schema::v2;
use agent_client_protocol::{AcpAgentConfig, Agent, V2ConnectionTo};
use futures_util::Stream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::config::{SessionConfig, SessionMode};
use answerable::{resolve_choice, OptionSet};
use connection::{lock, Ready, Shared};
use pending::{reply_cancelled, send_resolved_reply};

/// How long [`AcpDriver::spawn`] waits for `initialize` + `session/new` to
/// complete before treating the child as hung. `HOLLER_ACP_TIMEOUT_MS`
/// overrides the 10s default (the issue's own spec).
const DEFAULT_STARTUP_TIMEOUT_MS: u64 = 10_000;
/// How long [`AcpDriver::cancel`] waits for the agent's cancelled `idle`
/// state_update before force-killing the child (the issue's own spec).
const CANCEL_TIMEOUT: Duration = Duration::from_secs(5);
/// How long [`AcpDriver::shutdown`] waits for a graceful `session/close` +
/// connection teardown before force-killing the child (the issue's own spec).
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// One streamed event from an in-flight (or just-completed) turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverEvent {
    /// A chunk of the agent's streamed response text.
    Chunk(String),
    /// A foreground-work state transition.
    State(DriverState),
    /// The turn is over.
    Done(StopReason),
}

/// A foreground-work state transition streamed mid-turn (A2A's own naming —
/// never a `Blocked` variant, per the issue's own instruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Working,
    InputRequired,
}

/// The driver's current status (mirrors [`DriverState`] plus the initial/final
/// `Idle` the wire's `state_update` never has to name — there's simply
/// nothing in flight).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    InputRequired,
}

/// Why a turn ended, 1:1 with ACP v2's `stopReason` wire vocabulary, plus a
/// driver-local `Error` for a crashed/failed turn (never sent by an agent —
/// synthesized locally when the child dies or a cancel times out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    Cancelled,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Error,
}

impl StopReason {
    fn from_acp(reason: Option<&v2::StopReason>) -> Self {
        match reason {
            Some(v2::StopReason::EndTurn) => Self::EndTurn,
            Some(v2::StopReason::Cancelled) => Self::Cancelled,
            Some(v2::StopReason::MaxTokens) => Self::MaxTokens,
            Some(v2::StopReason::MaxTurnRequests) => Self::MaxTurnRequests,
            Some(v2::StopReason::Refusal) => Self::Refusal,
            // An unknown/future stop reason, or none reported at all: treat as
            // a plain end-of-turn rather than inventing a stricter meaning.
            Some(v2::StopReason::Other(_)) | None => Self::EndTurn,
            #[allow(unreachable_patterns)] // #188: `v2::StopReason` is #[non_exhaustive]
            Some(_) => Self::EndTurn,
        }
    }

    /// The ACP wire string this variant corresponds to — exactly the
    /// vocabulary `holler_proto::docs::STOP_TO_STATE` /
    /// `state_for_stop_reason` key on (that table is where the A2A
    /// `SessionState` mapping actually lives; see the module doc's
    /// "Decisions I made" for why this driver does not duplicate it). `Error`
    /// (a driver-local synthesized reason — a crash or a cancel timeout, never
    /// sent by an agent) maps to the same `"error"` key that table already
    /// carries.
    pub fn as_wire_str(self) -> &'static str {
        match self {
            Self::EndTurn => "end_turn",
            Self::Cancelled => "cancelled",
            Self::MaxTokens => "max_tokens",
            Self::MaxTurnRequests => "max_turn_requests",
            Self::Refusal => "refusal",
            Self::Error => "error",
        }
    }
}

/// Every fail-closed refusal this driver reports. The CLI/session-manager
/// layer maps these to its own exit/log story; this crate only distinguishes
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    /// `spawn` could not bring the child up (hung startup, an `attach`-mode
    /// config, or a transport/process failure).
    Startup(String),
    /// The child exited (or the connection failed) while a turn was in
    /// flight, or while a caller was waiting on it.
    Crashed(String),
    /// `cancel()` could not confirm the agent settled the in-flight turn.
    Cancel(String),
    /// `answer()`/`cancel()` observed no pending permission/elicitation.
    NothingPending,
    /// A pending elicitation this driver does not resolve (`Url` mode, or a
    /// non-enum form field) — reported explicitly, never silently dropped or
    /// auto-answered.
    Unsupported(String),
    /// `answer()`'s `choice` did not resolve against the pending item's own
    /// options. Fails closed: no reply was sent.
    Answer(String),
    /// The typed SDK itself reported an error sending a request/reply.
    Rpc(String),
}

impl DriverError {
    /// A one-line, human-readable reason (mirrors `ConfigError::message`'s
    /// convention elsewhere in this crate).
    pub fn message(&self) -> String {
        match self {
            Self::Startup(m) => format!("acp driver: startup failed: {m}"),
            Self::Crashed(m) => format!("acp driver: agent crashed: {m}"),
            Self::Cancel(m) => format!("acp driver: cancel failed: {m}"),
            Self::NothingPending => "acp driver: nothing pending to answer".to_string(),
            Self::Unsupported(m) => format!("acp driver: unsupported elicitation: {m}"),
            Self::Answer(m) => format!("acp driver: choice did not resolve: {m}"),
            Self::Rpc(m) => format!("acp driver: rpc error: {m}"),
        }
    }
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for DriverError {}

/// A `Stream<Item = DriverEvent>` over the receiving half of the per-prompt
/// event channel. `tokio::sync::mpsc::UnboundedReceiver` does not implement
/// `Stream` itself (that needs the `tokio-stream` crate, not a workspace
/// dependency for one adapter), so this is the minimal hand-rolled wrapper —
/// `poll_recv` already has exactly `Stream::poll_next`'s shape.
pub struct DriverEventStream {
    rx: mpsc::UnboundedReceiver<DriverEvent>,
}

impl Stream for DriverEventStream {
    type Item = DriverEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<DriverEvent>> {
        self.rx.poll_recv(cx)
    }
}

/// A live ACP v2 spawn-mode driver: one spawned harness child, one ACP
/// session, for the lifetime of this value.
pub struct AcpDriver {
    session: v2::SessionId,
    connection: V2ConnectionTo<Agent>,
    shared: Arc<Mutex<Shared>>,
    join_handle: JoinHandle<()>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl Drop for AcpDriver {
    fn drop(&mut self) {
        // A caller that drops the driver without an explicit `shutdown()`
        // (a panicking test, an early return, …) must not leak the spawned
        // child process, its connection task, or its `async-process`
        // blocking-pool I/O threads. `abort()` ends the background
        // `connection::run` task; the SDK's own transport guard tears down
        // the child (and, on unix, its whole process group) when the future
        // it owns is dropped, which the runtime does once the aborted task is
        // polled to completion. This is synchronous best-effort cleanup — a
        // `Drop` impl cannot `.await` a graceful `session/close` the way
        // `shutdown()` does.
        self.join_handle.abort();
    }
}

impl AcpDriver {
    /// Spawn the configured harness and bring up one ACP v2 session.
    ///
    /// Only `SessionMode::Spawn` is supported (see the module doc). Startup
    /// (the child process coming up, `initialize`, and `session/new`) is
    /// bounded by `HOLLER_ACP_TIMEOUT_MS` (default 10s); a hang there kills
    /// the child and returns `DriverError::Startup`.
    pub async fn spawn(config: &SessionConfig) -> Result<Self, DriverError> {
        if config.mode != SessionMode::Spawn {
            return Err(DriverError::Startup(
                "attach mode is not implemented by this driver (spawn-only, issue #188)"
                    .to_string(),
            ));
        }
        let argv = config
            .command
            .as_ref()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| {
                DriverError::Startup("spawn mode requires a non-empty `command`".to_string())
            })?;

        let mut agent_config = AcpAgentConfig::new(argv[0].clone());
        if argv.len() > 1 {
            agent_config = agent_config.args(argv[1..].to_vec());
        }
        if let Some(env) = &config.env {
            agent_config = agent_config.envs(env.clone());
        }
        // The SDK's `AcpAgentConfig` has no process-cwd field of its own — ACP
        // agents take their working directory from `session/new`'s own `cwd`
        // param instead (see the module doc's "Decisions I made").
        let session_cwd = match &config.cwd {
            Some(cwd) => PathBuf::from(cwd),
            None => std::env::current_dir().map_err(|e| {
                DriverError::Startup(format!("cannot resolve current directory: {e}"))
            })?,
        };

        let shared = Arc::new(Mutex::new(Shared {
            status: Status::Idle,
            pending: None,
            current_events: None,
            awaiting_done: None,
        }));

        let (ready_tx, ready_rx) = oneshot::channel::<Result<Ready, String>>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let task_shared = shared.clone();
        let join_handle: JoinHandle<()> = tokio::spawn(async move {
            connection::run(
                agent_config,
                session_cwd,
                task_shared,
                ready_tx,
                shutdown_rx,
            )
            .await;
        });

        let timeout_ms: u64 = std::env::var("HOLLER_ACP_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_STARTUP_TIMEOUT_MS);

        match tokio::time::timeout(Duration::from_millis(timeout_ms), ready_rx).await {
            Ok(Ok(Ok(ready))) => Ok(Self {
                session: ready.session_id,
                connection: ready.connection,
                shared,
                join_handle,
                shutdown_tx: Mutex::new(Some(shutdown_tx)),
            }),
            Ok(Ok(Err(reason))) => {
                join_handle.abort();
                Err(DriverError::Startup(reason))
            }
            Ok(Err(_dropped)) => {
                join_handle.abort();
                Err(DriverError::Startup(
                    "connection task ended before signalling readiness".to_string(),
                ))
            }
            Err(_timed_out) => {
                // Kill the still-hung child by ending its owning task; the
                // SDK's `AcpAgent` transport installs a guard that tears down
                // the spawned process group when the connection future is
                // dropped (which `abort` forces).
                join_handle.abort();
                Err(DriverError::Startup(format!(
                    "no response within {timeout_ms}ms (HOLLER_ACP_TIMEOUT_MS)"
                )))
            }
        }
    }

    /// Submit a text prompt and stream its updates until the turn resolves.
    ///
    /// Only one turn may be in flight at a time (the SDK does not gate this
    /// locally either — see `v2::V2Session::send_prompt_blocks`'s own doc);
    /// callers are expected to wait for a prior turn's `Done` before prompting
    /// again (`prompt_after_cancel_is_fresh_turn` is exactly this contract
    /// after a `cancel()`).
    pub async fn prompt(&self, text: &str) -> DriverEventStream {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut guard = lock(&self.shared);
            guard.current_events = Some(tx);
            guard.status = Status::Working;
        }
        let session = self.session();
        // Fire-and-forget the acceptance ack: v2's `session/prompt` response
        // only means "accepted", not "done" (see the module doc). A failure
        // here (e.g. the child already died) surfaces as a `Done(Error)` on
        // the stream via the crash watcher, not as a hang.
        let result = self
            .connection
            .send_request_to(Agent, v2::PromptRequest::new(session, vec![text.into()]))
            .block_task()
            .await;
        if result.is_err() {
            let mut guard = lock(&self.shared);
            guard.status = Status::Idle;
            if let Some(tx) = guard.current_events.take() {
                let _ = tx.send(DriverEvent::Done(StopReason::Error));
            }
        }
        DriverEventStream { rx }
    }

    /// Ask the agent to cancel its current foreground work, resolve any
    /// pending permission/elicitation with the `Cancelled`/`Cancel` outcome,
    /// and wait (bounded) for the agent's own cancelled `idle` state_update.
    ///
    /// The driver's status is `Idle` only after that response is observed —
    /// this is the contract that makes it impossible to start the next prompt
    /// on a turn the agent still considers open. On timeout, the child is
    /// force-killed and `DriverError::Cancel` is returned.
    pub async fn cancel(&self) -> Result<(), DriverError> {
        // Resolve any pending block with the cancelled outcome first (per the
        // ACP v2 cancellation contract: a client sending `session/cancel`
        // MUST resolve every pending `session/request_permission` with
        // `Cancelled`).
        {
            let mut guard = lock(&self.shared);
            if let Some(pending) = guard.pending.take() {
                reply_cancelled(pending.responder);
            }
            if guard.status == Status::Idle && guard.current_events.is_none() {
                // Nothing in flight: a no-op, not a 5s stall waiting for an
                // `idle` state_update that will never arrive.
                return Ok(());
            }
        }

        let (done_tx, done_rx) = oneshot::channel();
        {
            let mut guard = lock(&self.shared);
            guard.awaiting_done = Some(done_tx);
        }

        self.session_cancel()
            .map_err(|e| DriverError::Cancel(format!("sending session/cancel: {e}")))?;

        match tokio::time::timeout(CANCEL_TIMEOUT, done_rx).await {
            Ok(Ok(_stop_reason)) => {
                lock(&self.shared).status = Status::Idle;
                Ok(())
            }
            Ok(Err(_recv_dropped)) => Err(DriverError::Cancel(
                "connection ended while awaiting the cancelled state_update".to_string(),
            )),
            Err(_timed_out) => {
                self.join_handle.abort();
                Err(DriverError::Cancel(format!(
                    "no cancelled state_update within {CANCEL_TIMEOUT:?}; child force-killed"
                )))
            }
        }
    }

    /// Resolve a pending permission/elicitation. `choice` is a 0-based index
    /// or an exact (case-insensitive) label/key match for a single pending
    /// item; for a multi-field elicitation it is comma-separated, one segment
    /// per field, resolved independently and in order. Fails closed: any
    /// unresolved segment, or a segment-count mismatch, sends no reply at all
    /// (the pending item stays open and can be retried).
    pub async fn answer(&self, choice: &str) -> Result<(), DriverError> {
        let pending = {
            let mut guard = lock(&self.shared);
            guard.pending.take().ok_or(DriverError::NothingPending)?
        };

        if let Some(reason) = pending.unsupported.clone() {
            // Held open, not consumed: an unsupported item can never resolve,
            // but it must not vanish from `pending` either (that would read as
            // silently dropped/auto-answered to a caller polling `status`).
            lock(&self.shared).pending = Some(pending);
            return Err(DriverError::Unsupported(reason));
        }

        let option_sets: Vec<OptionSet> =
            pending.fields.iter().map(|f| f.options.clone()).collect();
        let resolved = match resolve_choice(&option_sets, choice) {
            Ok(values) => values,
            Err(reason) => {
                lock(&self.shared).pending = Some(pending);
                return Err(DriverError::Answer(reason));
            }
        };

        let field_names: Vec<(String, bool)> = pending
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.multi))
            .collect();
        send_resolved_reply(pending.responder, field_names, resolved)
    }

    /// The driver's current status.
    pub fn status(&self) -> Status {
        lock(&self.shared).status
    }

    /// Close the session gracefully, then end the connection (which kills the
    /// child's whole process tree via the SDK's own transport guard). Bounded
    /// by `SHUTDOWN_GRACE`; a hang past that force-kills instead.
    pub async fn shutdown(&self) -> Result<(), DriverError> {
        let _ = self.session_close().await;
        if let Some(tx) = lock_option(&self.shutdown_tx).take() {
            let _ = tx.send(());
        }
        match tokio::time::timeout(SHUTDOWN_GRACE, wait_join(&self.join_handle)).await {
            Ok(()) => Ok(()),
            Err(_timed_out) => {
                self.join_handle.abort();
                Ok(())
            }
        }
    }

    fn session(&self) -> v2::SessionId {
        self.session.clone()
    }

    fn session_cancel(&self) -> Result<(), agent_client_protocol::Error> {
        self.connection
            .send_notification_to(Agent, v2::CancelSessionNotification::new(self.session()))
    }

    async fn session_close(
        &self,
    ) -> Result<v2::CloseSessionResponse, agent_client_protocol::Error> {
        self.connection
            .send_request_to(Agent, v2::CloseSessionRequest::new(self.session()))
            .block_task()
            .await
    }
}

fn lock_option<T>(m: &Mutex<Option<T>>) -> MutexGuard<'_, Option<T>> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Await a `JoinHandle<()>` without surfacing a join error as anything other
/// than "the task ended" — `shutdown`'s own bounded wait only cares whether
/// the task is still running, not why it stopped.
async fn wait_join(handle: &JoinHandle<()>) {
    // `JoinHandle` is not `Clone`; abort-safety here only needs to *observe*
    // completion, not consume the handle, so poll it directly rather than
    // awaiting a moved clone.
    struct WaitJoin<'a>(&'a JoinHandle<()>);
    impl std::future::Future for WaitJoin<'_> {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<()> {
            if self.0.is_finished() {
                return Poll::Ready(());
            }
            // No native "notify on finish" without consuming the handle;
            // a short re-poll interval keeps this bounded and cheap relative
            // to `SHUTDOWN_GRACE`.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
    WaitJoin(handle).await;
}
