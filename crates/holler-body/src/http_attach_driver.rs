//! HTTP attach driver (issue #194, replaying legacy `holler-client`'s
//! `src/http_attach_driver.rs`, issue #100/ADR-0005 / holler-server
//! ADR-0017): drives an OpenCode session that **another process owns**
//! (a Herdr pane, or a bare `opencode serve`) over that process's own HTTP
//! control surface, instead of spawning and owning a child the way
//! [`crate::acp_driver::AcpDriver`] does.
//!
//! This module never calls `session/new` (`POST /session` / `POST
//! /api/session`) and never execs anything — see [`HttpAttachDriver::attach`]
//! for the fail-closed existence check that enforces this.
//!
//! Split across three files to stay under the workspace's 900-line-per-file
//! guard (`scripts/lint.sh`), mirroring `acp_driver`'s own split: this file
//! owns the public [`HttpAttachDriver`] API; `connection.rs` owns the
//! background SSE/poll loop and the state shared with it; `wire.rs` owns the
//! pure OpenCode wire shapes and choice-resolution logic.
//!
//! # Real OpenCode HTTP paths — independently re-verified live in this
//! environment against a real `opencode serve` v1.18.20 (`opencode --version`),
//! by starting `opencode serve --port <N>`, `POST`ing a real session, and
//! curling every route this driver uses. **This is not the legacy driver's
//! own doc comment restated on faith — every path/status below was curled
//! for real during this issue's implementation**, and two of them
//! (`prompt_async`/`interrupt`) turned out to disagree with a naive reading
//! of the legacy driver's "pick one prefix, use it everywhere" framing (see
//! below) — this driver hardcodes the two exact routes that were actually
//! observed to work, not a resolved-prefix guess.
//!
//! - **Existence check**: `GET {endpoint}/session/{session_id}` — confirmed
//!   `200` for a real session, `404` for a bogus id. `GET
//!   {endpoint}/api/session/{session_id}` also confirmed `200` for the same
//!   real session — **both forms genuinely work** as an existence probe, so
//!   [`HttpAttachDriver::attach`] tries the bare form first and falls back to
//!   `/api` on a `404` of the first, same as this issue's own spec. Any
//!   non-2xx from *both* attempts is [`crate::acp_driver::DriverError::NotFound`]
//!   — fail closed, never fall through to `POST /session` (verified this is
//!   never called by [`HttpAttachDriver::attach`]).
//! - **Prompt**: confirmed live that `POST {endpoint}/session/{session_id}/prompt_async`
//!   (bare, **not** `/api`-prefixed) is the real route — `204` immediately,
//!   genuinely fire-and-forget. `POST {endpoint}/api/session/{id}/prompt_async`
//!   is **not** a real route: it fell through to the SPA's catch-all and
//!   returned a `200` of the web UI's own `index.html`, not a driver success.
//!   This driver therefore always calls the bare form, regardless of which
//!   prefix answered the existence check.
//! - **Interrupt**: confirmed the exact mirror image — `POST
//!   {endpoint}/api/session/{session_id}/interrupt` (**`/api`-prefixed**) is
//!   the real route (`204`); the bare `POST {endpoint}/session/{id}/interrupt`
//!   also fell through to the SPA's HTML shell (`200`, not a real interrupt).
//!   This driver always calls the `/api`-prefixed form.
//!
//!   Together these two confirm the legacy driver's own doc comment was
//!   right about *which* form each individual route needs, but a
//!   single-resolved-prefix design (this issue's own initial framing) would
//!   have silently no-op'd `interrupt` on any endpoint whose existence check
//!   happened to resolve via the bare form — a real, verified footgun this
//!   driver avoids by hardcoding each route to its own confirmed form
//!   instead of sharing one resolved prefix between them.
//! - **Observing the reply**: the legacy driver's own claim that no
//!   per-session SSE stream (`GET {endpoint}/api/session/{id}/event`)
//!   actually emits anything was **not** independently re-checked here
//!   (it needs a real in-flight turn to prove a negative on); what *was*
//!   directly confirmed live is that the **global**, always-bare (never
//!   `/api`-prefixed) event stream, `GET {endpoint}/event`, connects and
//!   immediately emits a real event (`server.connected`) as a
//!   newline-delimited `data: {json}\n\n` SSE frame — this driver reads that
//!   stream and filters every frame's `properties.sessionID` against the
//!   attached session. Event types this
//!   driver reacts to: `message.updated` (tracks message id → role),
//!   `message.part.updated` (a text delta, only surfaced for the assistant
//!   role) → [`crate::acp_driver::DriverEvent::Chunk`]; `session.status`
//!   `busy` → [`crate::acp_driver::DriverEvent::State`]`(Working)`;
//!   `session.idle` → `Done(EndTurn)` (or `Done(Cancelled)` if this driver's
//!   own [`HttpAttachDriver::cancel`] requested it); `session.error` →
//!   `Done(Error)`.
//! - **Reconnect** (new in this issue — the legacy driver has no reconnect
//!   logic of its own): if the SSE connection drops while a turn is running,
//!   the background loop reconnects using the same full-jitter, base-1s,
//!   cap-30s backoff schedule [`crate::backoff`] gives the hub's own circuit
//!   connection (issue #182). It always attempts one initial connect right
//!   away regardless of turn state (so events are ready the moment
//!   `attach()` returns), then only keeps retrying while a turn is running.
//!
//! # Pending questions/permissions (holler-server#382, copy-adapted from the
//! legacy driver's own polling)
//!
//! OpenCode's `question`/`permission` tools genuinely block the agent's turn
//! until answered; whether they ever appear on `/event` was not
//! independently re-checked here (needs a real pending question to prove a
//! negative on), so this driver polls, matching the legacy driver's own
//! choice. `GET {endpoint}/question` / `GET {endpoint}/permission` (bare,
//! global, never `/api`-prefixed) were confirmed live to return `[]` with no
//! question/permission pending; this driver polls them every
//! [`connection::BLOCK_POLL_INTERVAL`] while a turn is
//! running, for the attached session **and its child sessions** (this
//! issue's own extension over the legacy driver — `refresh_child_sessions`
//! in `connection.rs`, from `GET {endpoint}/session`'s own `parentID`
//! field). A pending item flips [`Status::InputRequired`] and is surfaced
//! via [`HttpAttachDriver::pending`], mirroring
//! [`crate::acp_driver::AcpDriver::pending`]'s own "read the live state, no
//! cached snapshot" contract. [`HttpAttachDriver::answer`] resolves `choice`
//! against the real pending request's shape exactly as the legacy driver
//! does (a 0-based option index or exact label per question, comma-separated
//! for more than one question; `once`/`always`/`reject` plus aliases for a
//! permission) and POSTs the reply directly, bypassing the background loop.

mod connection;
mod wire;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use holler_proto::docs::{PendingItem, PendingKind};

use crate::acp_driver::{DriverError, DriverEventStream, Status, StopReason};
use crate::config::SessionConfig;

use connection::{lock, Command, Shared};
use wire::BlockKind;

/// How long [`HttpAttachDriver::cancel`] waits for the attached session's own
/// `session.idle`/`session.error` event before giving up (there is no child
/// process to force-kill on timeout the way [`crate::acp_driver::AcpDriver`]
/// can — this driver never owned one).
const CANCEL_TIMEOUT: Duration = Duration::from_secs(10);

/// A live attach-mode driver for one already-existing OpenCode session.
///
/// Mirrors [`crate::acp_driver::AcpDriver`]'s public shape
/// (`prompt`/`cancel`/`answer`/`status`/`pending`/`shutdown`, and returning
/// the very same [`DriverEventStream`]/[`DriverError`]/[`Status`]/
/// [`StopReason`] types) so `session_manager::SessionManager` (issue #195)
/// can wrap whichever of the two a session's `mode` selects without a second
/// code path.
pub struct HttpAttachDriver {
    command_tx: mpsc::UnboundedSender<Command>,
    connection_task: tokio::task::JoinHandle<()>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    client: reqwest::Client,
    endpoint: String,
    session_id: String,
    shared: Arc<Mutex<Shared>>,
}

impl HttpAttachDriver {
    /// Attaches to `config`'s already-existing OpenCode session.
    ///
    /// Fails closed — [`DriverError::NotFound`], never `session/new`, never
    /// exec `command` — if `endpoint`/`session_id` are missing (defensive;
    /// [`crate::config`] already validates this at load time) or if neither
    /// path prefix's existence check returns success.
    pub async fn attach(config: &SessionConfig) -> Result<Self, DriverError> {
        let endpoint = config
            .endpoint
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| DriverError::NotFound("no endpoint configured".to_string()))?;
        let session_id = config
            .session_id
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| DriverError::NotFound("no session_id configured".to_string()))?;

        let client = reqwest::Client::new();
        check_exists(&client, &endpoint, &session_id).await?;

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let shared = Arc::new(Mutex::new(Shared {
            status: Status::Idle,
            current_events: None,
            pending: None,
            interrupt_requested: false,
            last_stop_reason: None,
            awaiting_done: None,
        }));

        let task_client = client.clone();
        let task_endpoint = endpoint.clone();
        let task_session_id = session_id.clone();
        let task_shared = shared.clone();
        let connection_task = tokio::spawn(async move {
            connection::run(
                task_client,
                task_endpoint,
                task_session_id,
                command_rx,
                task_shared,
                shutdown_rx,
            )
            .await;
        });

        Ok(Self {
            command_tx,
            connection_task,
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
            client,
            endpoint,
            session_id,
            shared,
        })
    }

    /// Submit a text prompt and stream its updates until the turn resolves —
    /// the same non-blocking contract as
    /// [`crate::acp_driver::AcpDriver::prompt`]: returns once the request has
    /// been handed to the background task, not once the turn completes.
    pub async fn prompt(&self, text: &str) -> DriverEventStream {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut guard = lock(&self.shared);
            guard.current_events = Some(tx);
            guard.status = Status::Working;
        }
        let _ = self.command_tx.send(Command::Prompt(text.to_string()));
        DriverEventStream::new(rx)
    }

    /// Sends an HTTP interrupt directly (this driver's *primary* cancel
    /// path — there is no ACP channel to prefer over it) and waits (bounded
    /// by [`CANCEL_TIMEOUT`]) for the attached session's own
    /// `session.idle`/`session.error` event, returning the **real**
    /// [`StopReason`] it resolved with (never a synthesized `Cancelled`
    /// papering over some other real outcome — mirrors
    /// [`crate::acp_driver::AcpDriver::cancel`]'s own contract).
    pub async fn cancel(&self) -> Result<StopReason, DriverError> {
        let (done_tx, done_rx) = oneshot::channel();
        let already_settled = {
            let mut guard = lock(&self.shared);
            if guard.status == Status::Idle && guard.current_events.is_none() {
                Some(guard.last_stop_reason.unwrap_or(StopReason::Cancelled))
            } else {
                guard.interrupt_requested = true;
                guard.awaiting_done = Some(done_tx);
                None
            }
        };
        if let Some(reason) = already_settled {
            return Ok(reason);
        }

        connection::post_interrupt(&self.client, &self.endpoint, &self.session_id)
            .await
            .map_err(|e| DriverError::Cancel(format!("sending interrupt: {e}")))?;

        match tokio::time::timeout(CANCEL_TIMEOUT, done_rx).await {
            Ok(Ok(reason)) => Ok(reason),
            Ok(Err(_dropped)) => Err(DriverError::Cancel(
                "connection ended while awaiting the idle/error event".to_string(),
            )),
            Err(_timed_out) => Err(DriverError::Cancel(format!(
                "no idle/error event within {CANCEL_TIMEOUT:?}"
            ))),
        }
    }

    /// Answers whichever question/permission is currently blocking this
    /// session's turn, resolving `choice` against the real pending request's
    /// shape and POSTing the reply directly — bypassing the background loop
    /// entirely, the same pattern [`HttpAttachDriver::cancel`] uses for its
    /// own direct POST. Deliberately does not clear `pending` itself: the
    /// background poll loop is the single source of truth for it and will
    /// observe this id is gone on its very next tick.
    pub async fn answer(&self, choice: &str) -> Result<(), DriverError> {
        let pending = lock(&self.shared).pending.clone();
        let Some(pending) = pending else {
            return Err(DriverError::NothingPending);
        };
        connection::post_answer(&self.client, &self.endpoint, &pending, choice).await
    }

    /// The driver's current status.
    pub fn status(&self) -> Status {
        lock(&self.shared).status
    }

    /// The currently-held question/permission, if any — empty when nothing
    /// is pending (never a vacuous placeholder), mirroring
    /// [`crate::acp_driver::AcpDriver::pending`]'s own "read the live state"
    /// contract. A multi-question OpenCode request is flattened to its first
    /// question's prompt/options here — [`PendingItem`] has no multi-question
    /// shape of its own; [`HttpAttachDriver::answer`] still resolves the full
    /// comma-separated `choice` against every question.
    pub fn pending(&self) -> Vec<PendingItem> {
        lock(&self.shared)
            .pending
            .as_ref()
            .map(|p| {
                vec![PendingItem {
                    id: p.id.clone(),
                    kind: match p.kind {
                        BlockKind::Permission => PendingKind::Permission,
                        BlockKind::Question => PendingKind::Elicitation,
                    },
                    prompt: p.prompt.clone(),
                    options: p.options.first().cloned().unwrap_or_default(),
                }]
            })
            .unwrap_or_default()
    }

    /// The OpenCode session id this driver is attached to.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Detaches from this session WITHOUT touching it (issue #101/#194): ends
    /// this driver's own background loop. No HTTP `DELETE`, no process kill
    /// (there is none — this driver never owned one), no `session/new`
    /// replacement. The attached OpenCode process is completely unaffected —
    /// a fake test server still answers after this call
    /// (`shutdown_does_not_touch_fake_server`).
    pub async fn shutdown(&self) -> Result<(), DriverError> {
        if let Some(tx) = lock_option(&self.shutdown_tx).take() {
            let _ = tx.send(());
        }
        self.connection_task.abort();
        Ok(())
    }
}

/// The fail-closed existence check — `GET {endpoint}/session/{id}` first,
/// falling back to `GET {endpoint}/api/session/{id}` on a `404` of the first
/// (both forms were independently confirmed live to work as an existence
/// probe — see the module doc). Any non-2xx from both is
/// [`DriverError::NotFound`] — this function makes no other HTTP call, in
/// particular never `POST /session`. Unlike an earlier draft of this driver,
/// nothing about *which* form answered is remembered or reused: `prompt_async`
/// and `interrupt` were independently confirmed to each need one specific,
/// fixed form regardless of which prefix the existence check used (see the
/// module doc's "Real OpenCode HTTP paths" section).
async fn check_exists(
    client: &reqwest::Client,
    endpoint: &str,
    session_id: &str,
) -> Result<(), DriverError> {
    let base = endpoint.trim_end_matches('/');
    let v1_url = format!("{base}/session/{session_id}");
    if let Ok(response) = client.get(&v1_url).send().await {
        if response.status().is_success() {
            return Ok(());
        }
    }
    let v2_url = format!("{base}/api/session/{session_id}");
    match client.get(&v2_url).send().await {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => Err(DriverError::NotFound(format!(
            "HTTP {} from both {v1_url} and {v2_url}",
            response.status()
        ))),
        Err(err) => Err(DriverError::NotFound(format!(
            "could not reach {endpoint}: {err}"
        ))),
    }
}

fn lock_option<T>(m: &Mutex<Option<T>>) -> MutexGuard<'_, Option<T>> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
