//! The attach driver of a Claude Code process (#850, ADR 0014 points 7 to 11): it drives the
//! interactive Claude Code process of a pane that the pane engine launched (ADR-0021 section 15)
//! and that this body does not own, as [`crate::http_attach_driver::HttpAttachDriver`] drives an
//! OpenCode session another process owns.
//!
//! **One implementation of delivery and observation (point 8).** The driver holds no knowledge
//! of the `claude` binary, its sockets or its environment: it drives the
//! [`holler_pane::HarnessPort`] that `holler body run` was handed (the Claude Code adapter, which
//! `holler-cli` builds as its pane wiring does; ADR-0021 section 5), and calls only three of its
//! methods. `locate_process` finds the process that holds the row's session, by that session id
//! and never by a remembered pid, so a relaunch of the same session (how a Claude Code pane is
//! interrupted) needs nothing from the body. `send_prompt` writes the message, one frame line to
//! the process's inbox socket. `session_activity` reads the process's state from `claude agents
//! --json`. It never calls `login_status` (ADR-0021 invariant I7), and it reads nothing the
//! process prints.
//!
//! **Every port call is blocking** (the port's contract), so each runs in `spawn_blocking` under
//! one outer bound, [`OP_BOUND`] (I5's 10 s), around the adapter's own shorter bounds. A bound
//! that fires stops the wait, not the blocked thread, which finishes on its own; the turn ends
//! `error` and the driver never sends the message again, delivered or not.
//!
//! **A turn (point 9)** is the child module `turn`'s poll loop. It costs one `claude agents
//! --json` run a tick (`session_activity`) while a turn is in flight and none while the row is
//! idle, plus one `locate_process` run when the turn ends, to tell an idle process that still
//! holds the session from a relaunched or a cleared one. The delivery itself is a lookup, a
//! write, and the adapter's own short watch for a message held behind a dialog.
//!
//! **Cancel and answer (point 10).** The body cannot cancel a turn in a process it does not own,
//! so [`ClaudeAttachDriver::cancel`] with a turn in flight is refused, naming `holler interrupt
//! --pane NAME`; with none it applies at once. A permission prompt is answered in the pane, so
//! [`ClaudeAttachDriver::answer`] is always refused. The session task turns both into the
//! existing wire refusals: a refused cancel is `-32004`, and so is a `replace` in flight, which
//! is a cancel followed by a prompt.
//!
//! The attach is the same lookup and fails closed ([`DriverError::NotFound`]): no process holds
//! the session, so the session task leaves the row off the roster and retries, and nothing is
//! created or started. `session/rebind` (point 11) is the session task's, which attaches the new
//! id through this same `attach`.

mod turn;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use holler_pane::{HarnessPort, PaneName};
use holler_proto::docs::PendingItem;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::acp_driver::{DriverError, DriverEventStream, Status, StopReason};
use crate::config::SessionConfig;

/// The outer bound of every port call the driver makes (I5): a `claude` that does not answer
/// ends the call's wait, never the session task.
pub(crate) const OP_BOUND: Duration = Duration::from_secs(10);

/// What the driver and its turn task share.
struct Shared {
    status: Status,
    /// How the last turn ended: what a cancel with no turn in flight reports, as
    /// [`crate::http_attach_driver::HttpAttachDriver::cancel`] does (issue #238: the real stop
    /// reason, never a synthesized one).
    last_stop_reason: Option<StopReason>,
}

/// A live attach-mode driver for the Claude Code process that holds one session.
///
/// Mirrors [`crate::http_attach_driver::HttpAttachDriver`]'s public shape (`attach`, `prompt`,
/// `cancel`, `answer`, `status`, `pending`, `session_id`, `shutdown`) and returns the same
/// [`DriverEventStream`], [`DriverError`], [`Status`] and [`StopReason`] types, so the session
/// task drives it as any other driver.
pub struct ClaudeAttachDriver {
    port: Arc<dyn HarnessPort>,
    /// The row's name read as a pane name (the grammar is the same): the pane whose process the
    /// port locates.
    pane: PaneName,
    session_id: String,
    shared: Arc<Mutex<Shared>>,
    /// The task that watches the turn in flight, if one ran.
    turn: Mutex<Option<JoinHandle<()>>>,
}

impl ClaudeAttachDriver {
    /// Attach to the Claude Code process that holds `config`'s session: one `locate_process`
    /// through `port`. Fails closed, [`DriverError::NotFound`], when no process holds it or the
    /// lookup fails or outlives [`OP_BOUND`]; nothing is created, started or written.
    pub async fn attach(config: &SessionConfig, port: Arc<dyn HarnessPort>) -> Result<Self, DriverError> {
        let session_id = config
            .session_id
            .clone()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| DriverError::NotFound("no session_id configured".to_owned()))?;
        let pane = PaneName::parse(config.name.as_str())
            .map_err(|error| DriverError::Startup(format!("the row's name is no pane name: {error}")))?;
        let (lookup, at, id) = (Arc::clone(&port), pane.clone(), session_id.clone());
        match port_call(move || lookup.locate_process(&at, &id)).await {
            Some(Ok(Some(_pid))) => {}
            Some(Ok(None)) => {
                return Err(DriverError::NotFound(format!(
                    "no Claude Code process holds session {session_id} (claude agents lists none)"
                )));
            }
            Some(Err(error)) => {
                return Err(DriverError::NotFound(format!(
                    "the lookup of the Claude Code process of session {session_id} failed: {error}"
                )));
            }
            None => {
                return Err(DriverError::NotFound(format!(
                    "the lookup of the Claude Code process of session {session_id} did not \
                     answer within {OP_BOUND:?}"
                )));
            }
        }
        Ok(Self {
            port,
            pane,
            session_id,
            shared: Arc::new(Mutex::new(Shared {
                status: Status::Idle,
                last_stop_reason: None,
            })),
            turn: Mutex::new(None),
        })
    }

    /// Deliver `text` and stream the turn until it ends, the same non-blocking contract as
    /// [`crate::http_attach_driver::HttpAttachDriver::prompt`]: it returns once the turn's task
    /// runs, not once the turn completes. The stream carries no reply text (the driver reads
    /// nothing the process prints): the state changes of the turn, then its `Done`, after one
    /// `Chunk` with the adapter's own message when the adapter refused the message.
    pub async fn prompt(&self, text: &str) -> DriverEventStream {
        let (events, rx) = mpsc::unbounded_channel();
        lock(&self.shared).status = Status::Working;
        let task = tokio::spawn(turn::run(turn::Turn {
            port: Arc::clone(&self.port),
            pane: self.pane.clone(),
            session: self.session_id.clone(),
            text: text.to_owned(),
            shared: Arc::clone(&self.shared),
            events,
        }));
        if let Some(previous) = lock(&self.turn).replace(task) {
            previous.abort();
        }
        DriverEventStream::new(rx)
    }

    /// With no turn in flight, the stop reason of the last turn (`Cancelled` when none ran), at
    /// once. With one in flight, refused: the body cannot cancel a turn in a process it does not
    /// own, and a signal would end the process; `holler interrupt --pane NAME` relaunches the pane
    /// on its session of record (ADR-0021 section 15, item 9). The turn runs on.
    pub async fn cancel(&self) -> Result<StopReason, DriverError> {
        let shared = lock(&self.shared);
        if shared.status == Status::Idle {
            return Ok(shared.last_stop_reason.unwrap_or(StopReason::Cancelled));
        }
        Err(DriverError::Cancel(format!(
            "this body does not own the Claude Code process of {pane}, so it cannot cancel its \
             turn; run `holler interrupt --pane {pane}`, which relaunches the pane on its session \
             of record",
            pane = self.pane
        )))
    }

    /// Always refused: a permission prompt or a question of a Claude Code process is answered by
    /// a person in its pane, and Holler types nothing into a pane (I4).
    pub async fn answer(&self, _choice: &str) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(format!(
            "a Claude Code process's permission prompt is answered in its pane {}, not through \
             the body",
            self.pane
        )))
    }

    /// The driver's current status.
    pub fn status(&self) -> Status {
        lock(&self.shared).status
    }

    /// Always empty: what the process waits on is answered in its pane, so nothing is pending
    /// here even while the row is `input-required`.
    pub fn pending(&self) -> Vec<PendingItem> {
        Vec::new()
    }

    /// The Claude Code session this driver is attached to.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Detach WITHOUT touching the process: end this driver's own turn task, if one runs. No
    /// message, no signal, no relaunch; the process runs on as it was.
    pub async fn shutdown(&self) -> Result<(), DriverError> {
        if let Some(task) = lock(&self.turn).take() {
            task.abort();
        }
        Ok(())
    }
}

impl Drop for ClaudeAttachDriver {
    /// A driver dropped with a turn in flight leaves no task polling the process.
    fn drop(&mut self) {
        if let Some(task) = lock(&self.turn).take() {
            task.abort();
        }
    }
}

/// One blocking port call, in `spawn_blocking` under [`OP_BOUND`]: `None` when the bound fired
/// (the call's thread finishes on its own) or the call's thread panicked.
async fn port_call<T: Send + 'static>(call: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    match tokio::time::timeout(OP_BOUND, tokio::task::spawn_blocking(call)).await {
        Ok(Ok(answer)) => Some(answer),
        Ok(Err(_)) | Err(_) => None,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
