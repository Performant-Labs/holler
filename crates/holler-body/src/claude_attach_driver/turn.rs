//! One turn of a Claude Code process (#850, ADR 0014 point 9, the turn rule): the delivery, then
//! the poll that follows the process's state until the turn ends.
//!
//! - **Delivery.** `locate_process` finds the process that holds the session now, and
//!   `send_prompt` writes the message to it. A turn starts when the adapter accepts the message.
//!   When the adapter refuses it (held behind a dialog, the process holds another session, no
//!   socket is known), the turn ends `error` after one text chunk that carries the adapter's own
//!   message, which quotes nothing the process printed. When no process holds the session, the
//!   turn ends `error` with nothing written.
//! - **The poll**, every [`POLL`]: `session_activity` of the process the message was written to.
//!   Busy is `working`; a process waiting on a permission prompt or a question is `input-required`,
//!   with nothing pending, and busy again is `working`. Idle ends the turn once the process was
//!   seen busy (or waiting), or [`START_WAIT`] after the write without having been seen busy (a
//!   turn too short to see). Before it ends the turn, the poll asks `locate_process` who holds the
//!   session: the same process is `end_turn`, a different one is `cancelled` (the pane was
//!   relaunched, which loses the turn in flight), and none is `error` (a person's `/clear` or
//!   `/resume` moved the process away from it). A read of the process that fails, the process gone
//!   included, is classified by the same lookup: a different process is `cancelled`, and anything
//!   else is `error`.
//! - Every call runs under the parent's [`super::OP_BOUND`]: one that fails or does not answer in
//!   time ends the turn `error`, and the message is never written again.
//!
//! The driver emits no progress of its own, so a long turn reads `stalled` in the roster after
//! the stall threshold: `claude agents --json` has no heartbeat to say otherwise.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use holler_pane::ports::Activity;
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError, PaneName};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

use super::{lock, port_call, Shared};
use crate::acp_driver::{DriverEvent, DriverState, Status, StopReason};

/// How often the turn reads the process's state.
const POLL: Duration = Duration::from_millis(500);
/// How long after the write an idle process that was never seen busy still counts as about to
/// start the turn.
const START_WAIT: Duration = Duration::from_secs(5);

/// What a turn's task needs: the port, the pane and the session, the message, the state it
/// shares with the driver and the turn's event channel.
pub(super) struct Turn {
    pub(super) port: Arc<dyn HarnessPort>,
    pub(super) pane: PaneName,
    pub(super) session: String,
    pub(super) text: String,
    pub(super) shared: Arc<Mutex<Shared>>,
    pub(super) events: UnboundedSender<DriverEvent>,
}

/// Run the turn to its end: deliver, poll, then mark the driver idle with the turn's stop reason
/// and send `Done`.
pub(super) async fn run(turn: Turn) {
    let reason = match turn.deliver().await {
        Ok(pid) => turn.watch(pid).await,
        Err(reason) => reason,
    };
    {
        let mut shared = lock(&turn.shared);
        shared.status = Status::Idle;
        shared.last_stop_reason = Some(reason);
    }
    let _ = turn.events.send(DriverEvent::Done(reason));
}

impl Turn {
    /// The process that holds the session now, and the message written to it: its pid. A
    /// message the adapter refused is one `Chunk` with the adapter's message, then `error`.
    async fn deliver(&self) -> Result<u32, StopReason> {
        let Some(Ok(Some(pid))) = self.locate().await else {
            return Err(StopReason::Error);
        };
        let (port, at) = (Arc::clone(&self.port), self.endpoint(pid));
        let (session, text) = (self.session.clone(), self.text.clone());
        match port_call(move || port.send_prompt(&at, &session, &text)).await {
            Some(Ok(())) => Ok(pid),
            Some(Err(refusal)) => {
                let _ = self.events.send(DriverEvent::Chunk(refusal.to_string()));
                Err(StopReason::Error)
            }
            None => Err(StopReason::Error),
        }
    }

    /// Poll the process `pid` the message was written to until the turn ends (see the module
    /// docs), streaming each change between `working` and `input-required`.
    async fn watch(&self, pid: u32) -> StopReason {
        let written = Instant::now();
        let mut seen_busy = false;
        let mut waiting = false;
        loop {
            tokio::time::sleep(POLL).await;
            let activity = match self.activity(pid).await {
                Some(Ok(activity)) => activity,
                Some(Err(_)) | None => return self.settle(pid, StopReason::Error).await,
            };
            match activity {
                Activity::Idle if seen_busy || written.elapsed() >= START_WAIT => {
                    return self.settle(pid, StopReason::EndTurn).await;
                }
                Activity::Idle => {}
                Activity::Busy => {
                    seen_busy = true;
                    if waiting {
                        waiting = false;
                        self.state(Status::Working, DriverState::Working);
                    }
                }
                Activity::WaitingPermission | Activity::HoldingQuestion => {
                    seen_busy = true;
                    if !waiting {
                        waiting = true;
                        self.state(Status::InputRequired, DriverState::InputRequired);
                    }
                }
            }
        }
    }

    /// How a turn whose process `pid` was last seen idle (`held`: `end_turn`), or could not be
    /// read (`held`: `error`), ends: as `held` while `pid` still holds the session, `cancelled`
    /// when another process holds it (a relaunch), and `error` when none does or the lookup fails.
    async fn settle(&self, pid: u32, held: StopReason) -> StopReason {
        match self.locate().await {
            Some(Ok(Some(holder))) if holder == pid => held,
            Some(Ok(Some(_relaunched))) => StopReason::Cancelled,
            Some(Ok(None) | Err(_)) | None => StopReason::Error,
        }
    }

    /// The pid of the process that holds the session now: `None` when the call did not answer.
    async fn locate(&self) -> Option<Result<Option<u32>, PaneError>> {
        let (port, pane, session) = (Arc::clone(&self.port), self.pane.clone(), self.session.clone());
        port_call(move || port.locate_process(&pane, &session)).await
    }

    /// What the process `pid` is doing: `None` when the call did not answer.
    async fn activity(&self, pid: u32) -> Option<Result<Activity, PaneError>> {
        let (port, at, session) = (Arc::clone(&self.port), self.endpoint(pid), self.session.clone());
        port_call(move || port.session_activity(&at, &session)).await
    }

    /// The driver's status and the stream both move to `state`.
    fn state(&self, status: Status, state: DriverState) {
        lock(&self.shared).status = status;
        let _ = self.events.send(DriverEvent::State(state));
    }

    fn endpoint(&self, pid: u32) -> HarnessEndpoint {
        HarnessEndpoint::Process {
            pane: self.pane.clone(),
            pid,
        }
    }
}
