//! One session's actor task: owns its own `Option<AcpDriver>`, its own FIFO
//! prompt queue, and the mailbox loop that drives both. Nothing here is ever
//! reached from a different session's task — the only things crossing a task
//! boundary are the per-command `oneshot` replies (owned by the caller) and a
//! clone of the presence-changed `broadcast::Sender` (see the parent
//! module's doc comment for why that clone is not "shared mutable state").

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use futures_util::StreamExt;
use holler_proto::docs::{LastTurn, PendingItem};
use holler_proto::{log, SessionName, SessionState};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::{PromptOutcome, SessionCommand, QUEUE_CAP};
use crate::acp_driver::{AcpDriver, DriverEvent, DriverEventStream, DriverState, StopReason};
use crate::config::SessionConfig;

/// The live, presence-facing snapshot of one session — written only by that
/// session's own task, read by [`super::SessionManager::presence_doc`] (and
/// any number of other outside readers) through the shared `Arc<Mutex<_>>`.
#[derive(Debug, Clone)]
pub(super) struct SessionPresence {
    pub(super) state: SessionState,
    pub(super) turn_started_at: Option<String>,
    pub(super) last_update_at: Option<String>,
    pub(super) turn_id: Option<String>,
    pub(super) last_turn: Option<LastTurn>,
    /// The held permission(s)/elicitation(s) (issue #151) — `Some` only
    /// while `state` is `InputRequired`, mirroring `SessionAd.pending`'s own
    /// wire contract. Populated from [`AcpDriver::pending`] by [`set_state`]
    /// every time this session transitions to (or stays in)
    /// `InputRequired`, and cleared the moment it leaves that state.
    pub(super) pending: Option<Vec<PendingItem>>,
}

impl Default for SessionPresence {
    /// Every session starts `idle` — no turn has run yet (mirrors
    /// [`crate::registry::SessionRegistry::from_sessions`]'s own baseline).
    /// Hand-written rather than `#[derive(Default)]` because `SessionState`
    /// (a `holler_proto` type) has no `Default` impl of its own to derive
    /// against — implementing one here would be an orphan-rule violation
    /// (neither this crate's trait nor this crate's type).
    fn default() -> Self {
        Self {
            state: SessionState::Idle,
            turn_started_at: None,
            last_update_at: None,
            turn_id: None,
            last_turn: None,
            pending: None,
        }
    }
}

/// A prompt sitting in the FIFO queue: its own `reply_tx` is held until it is
/// dispatched *and* that turn ends (never replied early with a "queued" ack —
/// see the parent module's "Decisions I made").
struct QueuedPrompt {
    id: String,
    text: String,
    reply_tx: oneshot::Sender<PromptOutcome>,
    updates: Option<mpsc::UnboundedSender<String>>,
}

/// This task's entire private state. Nothing here is behind a lock — it is
/// only ever touched by the one task that owns it (the presence snapshot it
/// publishes to the outside world is the sole exception, and that is a
/// write-only relationship from this side).
pub(super) struct Inner {
    config: SessionConfig,
    name: SessionName,
    driver: Option<AcpDriver>,
    current_stream: Option<DriverEventStream>,
    current_turn_id: Option<String>,
    current_reply: Option<oneshot::Sender<PromptOutcome>>,
    /// Where this turn's mid-turn text chunks go, if anyone is listening
    /// (issue #190). `None` for a caller that only wants the final result.
    current_updates: Option<mpsc::UnboundedSender<String>>,
    turn_started_at_ms: Option<Instant>,
    last_update_at_ms: Option<Instant>,
    queue: VecDeque<QueuedPrompt>,
    presence: Arc<Mutex<SessionPresence>>,
    presence_tx: broadcast::Sender<SessionName>,
    /// Monotonic per-session counter for `Replace`'s synthesized turn id (a
    /// `Replace` command carries no `id` of its own — see the issue's own
    /// `SessionCommand::Replace{text, reply_tx}` shape).
    replace_counter: u64,
}

impl Inner {
    pub(super) fn new(
        config: SessionConfig,
        presence: Arc<Mutex<SessionPresence>>,
        presence_tx: broadcast::Sender<SessionName>,
        name: SessionName,
    ) -> Self {
        Self {
            config,
            name,
            driver: None,
            current_stream: None,
            current_turn_id: None,
            current_reply: None,
            current_updates: None,
            turn_started_at_ms: None,
            last_update_at_ms: None,
            queue: VecDeque::new(),
            presence,
            presence_tx,
            replace_counter: 0,
        }
    }
}

/// The session's whole task lifetime: consume commands and driver events
/// until `Shutdown` (or the mailbox closes because every [`super::SessionManager`]
/// handle to it was dropped), then gracefully end the driver, if any.
pub(super) async fn run(mut mailbox: mpsc::Receiver<SessionCommand>, mut inner: Inner) {
    loop {
        tokio::select! {
            biased;
            cmd = mailbox.recv() => {
                match cmd {
                    None | Some(SessionCommand::Shutdown) => {
                        if let Some(driver) = inner.driver.take() {
                            let _ = driver.shutdown().await;
                        }
                        return;
                    }
                    Some(SessionCommand::Prompt { id, text, queue, reply_tx, updates }) => {
                        handle_prompt(&mut inner, id, text, queue, reply_tx, updates).await;
                    }
                    Some(SessionCommand::Cancel { reply_tx }) => {
                        handle_cancel(&mut inner, reply_tx).await;
                    }
                    Some(SessionCommand::Answer { choice, reply_tx }) => {
                        handle_answer(&mut inner, choice, reply_tx).await;
                    }
                    Some(SessionCommand::Replace { text, reply_tx, updates }) => {
                        handle_replace(&mut inner, text, reply_tx, updates).await;
                    }
                }
            }
            event = poll_stream(&mut inner.current_stream) => {
                handle_stream_event(&mut inner, event).await;
            }
        }
    }
}

/// Await the next event on `stream`, or pend forever when there is none —
/// lets an absent turn's "branch" sit inert in the `select!` above instead of
/// needing a separate `if current_stream.is_some()` guard on every iteration.
async fn poll_stream(stream: &mut Option<DriverEventStream>) -> Option<DriverEvent> {
    match stream {
        Some(s) => s.next().await,
        None => std::future::pending().await,
    }
}

async fn handle_prompt(
    inner: &mut Inner,
    id: String,
    text: String,
    queue: bool,
    reply_tx: oneshot::Sender<PromptOutcome>,
    updates: Option<mpsc::UnboundedSender<String>>,
) {
    let current_state = inner.presence.lock().unwrap_or_else(PoisonError::into_inner).state;
    if current_state == SessionState::Idle {
        start_turn(inner, id, text, reply_tx, updates).await;
        return;
    }
    // Issue #151: `input-required` is `queue`-immune — a held permission/
    // elicitation is answered, not appended behind. `--queue` only ever
    // relieves a plain `working` busy session (the `!queue` arm just below);
    // an `input-required` prompt is always the immediate `Busy` refusal, even
    // with `queue:true`. The hub's own `talk::say` already gates this before
    // ever forwarding the request (never sends a queued prompt to a session
    // it knows is input-required) — this is the body's own fail-closed
    // backstop for a caller that skips that gate (a direct wire client, or a
    // race against a state change the hub's cache hasn't caught up to yet).
    if !queue || current_state == SessionState::InputRequired {
        let (state, turn_age_ms, last_update_age_ms) = busy_ages(inner, current_state);
        let _ = reply_tx.send(PromptOutcome::Busy { state, turn_age_ms, last_update_age_ms });
        return;
    }
    if inner.queue.len() >= QUEUE_CAP {
        let _ = reply_tx.send(PromptOutcome::QueueFull);
        return;
    }
    inner.queue.push_back(QueuedPrompt { id, text, reply_tx, updates });
}

/// The `-32009 session_busy` refusal's `data` trio: the session's current A2A
/// state and how long it has held that state / gone without an update.
fn busy_ages(inner: &Inner, state: SessionState) -> (String, u64, u64) {
    let now = Instant::now();
    let age_ms = |since: Option<Instant>| {
        since.map(|t| u64::try_from(now.duration_since(t).as_millis()).unwrap_or(u64::MAX)).unwrap_or(0)
    };
    let state_str = match state {
        SessionState::InputRequired => "input-required",
        // `handle_prompt` only reaches here for `Working`/`InputRequired`
        // (the `Idle` case dispatches instead) — `Working` is the fallback
        // for any other value so this never panics on a future state.
        _ => "working",
    }
    .to_string();
    (state_str, age_ms(inner.turn_started_at_ms), age_ms(inner.last_update_at_ms))
}

async fn handle_cancel(inner: &mut Inner, reply_tx: oneshot::Sender<Result<(), String>>) {
    match cancel_current(inner).await {
        None => {
            let _ = reply_tx.send(Ok(()));
        }
        Some(Err(e)) => {
            let _ = reply_tx.send(Err(e));
        }
        Some(Ok(reason)) => {
            let _ = reply_tx.send(Ok(()));
            if inner.current_reply.is_some() {
                // Finish the turn with the REAL stop reason the driver
                // reported (issue #238) — never a hardcoded `Cancelled`. The
                // agent may have actually settled the turn some other way
                // (`end_turn`/`max_tokens`/`refusal`/…) right as this cancel
                // was processed; `docs/protocol/v2.md` §6 makes `stopReason`
                // the source of truth, carried verbatim, so inventing one
                // here would violate that contract even though "the caller
                // asked to cancel" — what the caller asked for and what
                // actually happened are not always the same thing.
                finish_turn(inner, reason).await;
            }
        }
    }
}

async fn handle_answer(inner: &mut Inner, choice: String, reply_tx: oneshot::Sender<Result<(), String>>) {
    let current_state = inner.presence.lock().unwrap_or_else(PoisonError::into_inner).state;
    if current_state != SessionState::InputRequired {
        let _ = reply_tx.send(Err("nothing pending to answer".to_string()));
        return;
    }
    let answer_result = {
        let Some(driver) = inner.driver.as_ref() else {
            let _ = reply_tx.send(Err("nothing pending to answer".to_string()));
            return;
        };
        driver.answer(&choice).await
    };
    if let Err(e) = answer_result {
        let _ = reply_tx.send(Err(e.message()));
        return;
    }
    // "responds `applied` only after the driver reports `Working` again"
    // (the issue's own contract) — drive this session's own event stream
    // until that happens, or the turn ends outright.
    loop {
        match poll_stream(&mut inner.current_stream).await {
            Some(DriverEvent::State(DriverState::Working)) => {
                set_state(inner, SessionState::Working);
                let _ = reply_tx.send(Ok(()));
                return;
            }
            Some(DriverEvent::State(DriverState::InputRequired)) => {
                // A chained gate: still not "Working" — keep waiting.
                set_state(inner, SessionState::InputRequired);
            }
            // Same rationale as `handle_stream_event`'s `Chunk` arm (issue
            // #210): a chunk arriving while waiting for the driver to
            // resume to `Working` is still real activity, not silence.
            Some(DriverEvent::Chunk(_)) => touch_last_update(inner),
            Some(DriverEvent::Done(reason)) => {
                finish_turn(inner, reason).await;
                // The answer itself was applied; the turn simply ended right
                // after resuming rather than idling in `Working` first.
                let _ = reply_tx.send(Ok(()));
                return;
            }
            None => {
                let _ = reply_tx.send(Err("connection ended while awaiting resume".to_string()));
                return;
            }
        }
    }
}

async fn handle_replace(
    inner: &mut Inner,
    text: String,
    reply_tx: oneshot::Sender<PromptOutcome>,
    updates: Option<mpsc::UnboundedSender<String>>,
) {
    match cancel_current(inner).await {
        Some(Err(e)) => {
            let _ = reply_tx.send(PromptOutcome::Error(e));
            return;
        }
        Some(Ok(reason)) if inner.current_reply.is_some() => {
            // Close the cancelled turn out (and fire *its own* caller's
            // reply) without touching the FIFO queue yet — this replacement
            // runs ahead of it, per the issue's own "cancel → run ahead of
            // the queue" contract for `Replace`. `reason` is the driver's
            // real stop reason (issue #238), not a hardcoded `Cancelled` —
            // the turn being replaced may have already settled some other
            // way right as this `Replace` was processed.
            finish_turn_no_dispatch(inner, reason).await;
        }
        Some(Ok(_)) | None => {}
    }
    inner.replace_counter += 1;
    let id = format!("replace-{}", inner.replace_counter);
    // `Replace` (`interrupt SESSION TEXT`, issue #191) streams its reply
    // exactly like `say` — `updates` is the caller's own coalescer feed.
    start_turn(inner, id, text, reply_tx, updates).await;
}

/// Cancel the in-flight turn via the driver, if a driver exists at all.
/// `None` means "nothing to cancel" (no driver has ever been spawned for
/// this session yet); `Some(Ok(reason))` means the driver confirmed the turn
/// (if any) is settled, carrying the REAL `StopReason` it observed (issue
/// #238 — never synthesized) — the caller still must check whether a turn
/// was actually in flight (`inner.current_reply.is_some()`) before treating
/// it as a real cancellation to finish out.
async fn cancel_current(inner: &Inner) -> Option<Result<StopReason, String>> {
    let driver = inner.driver.as_ref()?;
    Some(driver.cancel().await.map_err(|e| e.message()))
}

/// Spawn (if needed) and dispatch one turn. Returns `false` (having already
/// replied `PromptOutcome::Error` to `reply_tx`) when the driver could not
/// even be brought up — never recurses into the queue itself (an `async fn`
/// calling back into [`dispatch_next_queued`], which calls this, would be
/// unbounded recursion the compiler refuses to size); [`dispatch_next_queued`]
/// is the one place that loops over queued items on a spawn failure.
async fn start_turn(
    inner: &mut Inner,
    id: String,
    text: String,
    reply_tx: oneshot::Sender<PromptOutcome>,
    updates: Option<mpsc::UnboundedSender<String>>,
) -> bool {
    if inner.driver.is_none() {
        match AcpDriver::spawn(&inner.config).await {
            Ok(d) => inner.driver = Some(d),
            Err(e) => {
                let _ = reply_tx.send(PromptOutcome::Error(e.message()));
                return false;
            }
        }
    }
    let stream = {
        let Some(driver) = inner.driver.as_ref() else {
            let _ = reply_tx.send(PromptOutcome::Error("driver unavailable".to_string()));
            return false;
        };
        driver.prompt(&text).await
    };
    let turn_id = id.clone();
    inner.current_stream = Some(stream);
    inner.current_turn_id = Some(id);
    inner.current_reply = Some(reply_tx);
    inner.current_updates = updates;
    inner.turn_started_at_ms = Some(Instant::now());
    inner.last_update_at_ms = inner.turn_started_at_ms;
    let ts = log::timestamp();
    {
        let mut p = inner.presence.lock().unwrap_or_else(PoisonError::into_inner);
        p.state = SessionState::Working;
        p.turn_started_at = Some(ts.clone());
        p.last_update_at = Some(ts);
        p.pending = None;
        p.turn_id = Some(turn_id);
    }
    notify_presence(inner);
    true
}

async fn handle_stream_event(inner: &mut Inner, event: Option<DriverEvent>) {
    match event {
        // The stream ended without a `Done` (the sender was dropped without
        // ever sending one) — treat conservatively as a crashed turn rather
        // than hanging forever with a `current_reply` nobody will ever fire.
        None => finish_turn(inner, StopReason::Error).await,
        // Real turn activity with no state change (the common case for a
        // long streamed reply) — bump the staleness clock the same way a
        // state transition does, or `HOLLER_STALL_MS` false-alarms a
        // healthy, actively-streaming turn as stalled (issue #210), and
        // forward the chunk to whoever is streaming this turn's updates
        // (issue #190's coalescer), if anyone is.
        Some(DriverEvent::Chunk(text)) => {
            touch_last_update(inner);
            if let Some(tx) = &inner.current_updates {
                // A dropped receiver (the connection task ended, or nobody
                // was ever listening) is not a reason to fail the turn — the
                // driver keeps streaming regardless of whether anyone reads
                // the chunks; only the final `PromptOutcome` matters to it.
                let _ = tx.send(text);
            }
        }
        Some(DriverEvent::State(DriverState::Working)) => set_state(inner, SessionState::Working),
        Some(DriverEvent::State(DriverState::InputRequired)) => {
            set_state(inner, SessionState::InputRequired);
        }
        Some(DriverEvent::Done(reason)) => finish_turn(inner, reason).await,
    }
}

fn set_state(inner: &mut Inner, state: SessionState) {
    // Issue #151: `pending` mirrors `state` — populated from the driver's
    // own held block only while entering/staying `InputRequired`, and
    // cleared the instant this leaves that state (a resumed `Working`, or a
    // turn that ended outright). Read before the lock below so this never
    // holds the presence mutex while also locking the driver's own.
    let pending: Option<Vec<PendingItem>> = if state == SessionState::InputRequired {
        inner.driver.as_ref().map(|d| d.pending()).filter(|items| !items.is_empty())
    } else {
        None
    };
    {
        let mut p = inner.presence.lock().unwrap_or_else(PoisonError::into_inner);
        p.state = state;
        p.pending = pending;
    }
    // Shares its "bump the staleness clock + notify" half with the `Chunk`
    // arms below — a state transition is real activity too, and this is the
    // one place both kinds of activity funnel through (issue #210).
    touch_last_update(inner);
}

/// Bump `last_update_at`/`last_update_at_ms` to "now" and notify presence
/// watchers — the single place any evidence of real turn activity (a
/// streamed `Chunk`, or a state transition via [`set_state`]) goes through,
/// so the two paths that used to update this timestamp independently can't
/// drift out of sync again (issue #210).
fn touch_last_update(inner: &mut Inner) {
    inner.last_update_at_ms = Some(Instant::now());
    let ts = log::timestamp();
    {
        let mut p = inner.presence.lock().unwrap_or_else(PoisonError::into_inner);
        p.last_update_at = Some(ts);
    }
    notify_presence(inner);
}

/// Close out the current turn (mark idle, record `last_turn`, fire its
/// caller's `reply_tx`) without dispatching the next queued prompt. Split
/// out of [`finish_turn`] for `Replace`, which needs this half only — it
/// dispatches its *own* replacement turn immediately after, ahead of the
/// queue, rather than letting the queue go first.
async fn finish_turn_no_dispatch(inner: &mut Inner, reason: StopReason) {
    let a2a_state = holler_proto::state_for_stop_reason(reason.as_wire_str())
        .unwrap_or(SessionState::Failed);
    let turn_id = inner.current_turn_id.take().unwrap_or_default();
    let ended_at = log::timestamp();
    let last_turn = LastTurn {
        turn_id: turn_id.clone(),
        state: a2a_state,
        stop_reason: reason.as_wire_str().to_string(),
        ended_at,
    };
    {
        let mut p = inner.presence.lock().unwrap_or_else(PoisonError::into_inner);
        p.state = SessionState::Idle;
        p.turn_started_at = None;
        p.last_update_at = None;
        p.pending = None;
        p.turn_id = Some(turn_id.clone());
        p.last_turn = Some(last_turn.clone());
    }
    inner.current_stream = None;
    inner.current_updates = None;
    inner.turn_started_at_ms = None;
    inner.last_update_at_ms = None;
    // Crash isolation (issue #189's own acceptance item): drop the driver so
    // the *next* prompt respawns a fresh one, never retried automatically.
    if matches!(reason, StopReason::Error) {
        inner.driver = None;
    }
    notify_presence(inner);
    if let Some(reply_tx) = inner.current_reply.take() {
        let _ = reply_tx.send(PromptOutcome::Result {
            turn_id: last_turn.turn_id,
            stop_reason: last_turn.stop_reason,
            state: last_turn.state,
        });
    }
}

async fn finish_turn(inner: &mut Inner, reason: StopReason) {
    finish_turn_no_dispatch(inner, reason).await;
    dispatch_next_queued(inner).await;
}

/// After a turn ends (normally — never after a `Replace`'s own cancel, which
/// deliberately skips this via [`finish_turn_no_dispatch`]), dispatch the
/// next FIFO-queued prompt, if any. This is the sole dispatch point for
/// queued prompts, so "next queued prompt runs only after `Done(Cancelled)`
/// is observed" holds by construction — this function is only ever reached
/// after a turn has fully settled.
async fn dispatch_next_queued(inner: &mut Inner) {
    // A spawn failure for one queued item must not strand the rest of the
    // FIFO behind it — try the next one instead of leaving the queue stuck.
    while let Some(item) = inner.queue.pop_front() {
        if start_turn(inner, item.id, item.text, item.reply_tx, item.updates).await {
            return;
        }
    }
}

fn notify_presence(inner: &Inner) {
    let _ = inner.presence_tx.send(inner.name.clone());
}
