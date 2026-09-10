//! The background connection task: the shared state between the SDK's
//! concurrently-running handlers and the driver's own methods, and the
//! `initialize` → `session/new` → idle-until-shutdown lifecycle that runs for
//! the whole life of one `AcpDriver`.
//!
//! Split out of `acp_driver.rs` to keep both under the workspace's 900-line
//! file-size guard — this half owns "how do we talk to the SDK", while the
//! parent owns the public `AcpDriver` API and `pending.rs` owns "what does a
//! held-open request mean".

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_client_protocol::schema::v2;
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, Responder, V2ConnectionTo};
use holler_proto::docs::PendingKind;
use tokio::sync::{mpsc, oneshot};

use holler_proto::log::Direction as LogDirection;

use super::pending::{
    chunk_text, elicitation_fields, permission_fields, PendingBlock, PendingResponder,
};
use super::{log_debug, log_warn, DriverEvent, DriverState, Status, StopReason};

/// State shared between the connection's background handlers (which run
/// concurrently with the driver's public methods) and those methods
/// themselves. Every critical section here is synchronous — no `.await` is
/// ever taken while the lock is held — so a plain `std::sync::Mutex` is
/// enough; nothing here needs the async-aware kind.
pub(super) struct Shared {
    pub(super) status: Status,
    pub(super) pending: Option<PendingBlock>,
    /// The current turn's event sink, if a turn is in flight. Cleared the
    /// moment that turn resolves (an `idle` state_update, or a crash).
    pub(super) current_events: Option<mpsc::UnboundedSender<DriverEvent>>,
    /// Set by `cancel()` while it waits for the cancelled `idle` state_update;
    /// fired (and cleared) by the notification handler, or by the crash
    /// watcher if the connection dies first.
    pub(super) awaiting_done: Option<oneshot::Sender<StopReason>>,
    /// The most recent real `StopReason` this driver has observed (issue
    /// #238): every path that settles a turn — the notification handler's
    /// `Idle` arm and the crash watcher — records it here too, not just on
    /// `awaiting_done`/`current_events`. `cancel()`'s "nothing in flight"
    /// no-op path reads this so it reports the turn's *actual* last outcome
    /// instead of inventing `Cancelled` for a turn that may have ended some
    /// other way entirely (e.g. it already resolved `end_turn` before
    /// `cancel()` was even called).
    pub(super) last_stop_reason: Option<StopReason>,
}

pub(super) fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    // A poisoned lock (a prior critical section panicked) still holds usable
    // state — every critical section here is a handful of synchronous field
    // updates with no invariant that a panic mid-way could leave torn in a way
    // that matters for a test/production process going on to report the
    // error. Recovering rather than propagating the poison keeps `answer`/
    // `cancel`/the notification handlers from cascading one bug into a
    // permanently unusable driver.
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the background connection task hands back once `initialize` +
/// `session/new` succeed (or the reason they didn't).
pub(super) struct Ready {
    pub(super) session_id: v2::SessionId,
    pub(super) connection: V2ConnectionTo<Agent>,
}

/// Route one inbound `session/update` notification: a text chunk feeds the
/// current turn's event sink, a state transition updates `status` and (for
/// `running`/`requires_action`) also feeds the event sink. Everything else
/// (tool-call updates, plans, usage, …) is outside this story's scope and is
/// ignored — the driver only promises `Chunk`/`State`/`Done`.
fn handle_notification(shared: &Arc<Mutex<Shared>>, note: v2::UpdateSessionNotification) {
    match note.update {
        v2::SessionUpdate::AgentMessageChunk(chunk) => {
            if let Some(text) = chunk_text(&chunk.content) {
                log_debug(
                    LogDirection::In,
                    "session/update",
                    vec![("chunk_len", text.len().to_string())],
                    holler_proto::log::frame_at_noisy(&serde_json::json!({ "chunk": text }).to_string()),
                );
                let guard = lock(shared);
                if let Some(tx) = &guard.current_events {
                    let _ = tx.send(DriverEvent::Chunk(text));
                }
            }
        }
        v2::SessionUpdate::StateUpdate(state) => handle_state_update(shared, state),
        _ => {}
    }
}

fn handle_state_update(shared: &Arc<Mutex<Shared>>, state: v2::StateUpdate) {
    match state {
        v2::StateUpdate::Running(_) => {
            log_debug(LogDirection::In, "state_update", vec![("state", "working".to_string())], None);
            let mut guard = lock(shared);
            guard.status = Status::Working;
            if let Some(tx) = &guard.current_events {
                let _ = tx.send(DriverEvent::State(DriverState::Working));
            }
        }
        v2::StateUpdate::RequiresAction(_) => {
            log_debug(LogDirection::In, "state_update", vec![("state", "input_required".to_string())], None);
            let mut guard = lock(shared);
            // The permission/elicitation *request* handler (`store_pending`)
            // already flipped status to `InputRequired` and emitted this same
            // event the moment the request itself arrived — ACP always raises
            // that request before (or, for a well-behaved agent, alongside)
            // this `requires_action` state_update. Only emit here if that
            // hasn't already happened (status was still `Working`), so a
            // caller draining the event stream sees exactly one
            // `State(InputRequired)` per gate, not a duplicate.
            let already_reported = guard.status == Status::InputRequired;
            guard.status = Status::InputRequired;
            if !already_reported {
                if let Some(tx) = &guard.current_events {
                    let _ = tx.send(DriverEvent::State(DriverState::InputRequired));
                }
            }
        }
        v2::StateUpdate::Idle(idle) => {
            let stop_reason = StopReason::from_acp(idle.stop_reason.as_ref());
            log_debug(
                LogDirection::In,
                "state_update",
                vec![("state", "idle".to_string()), ("stop_reason", stop_reason.as_wire_str().to_string())],
                None,
            );
            let mut guard = lock(shared);
            guard.status = Status::Idle;
            guard.last_stop_reason = Some(stop_reason);
            if let Some(tx) = guard.current_events.take() {
                let _ = tx.send(DriverEvent::Done(stop_reason));
            }
            if let Some(done_tx) = guard.awaiting_done.take() {
                let _ = done_tx.send(stop_reason);
            }
        }
        v2::StateUpdate::Other(_) => {}
        // `StateUpdate` is `#[non_exhaustive]`.
        _ => {}
    }
}

/// Stash a newly-arrived permission/elicitation request as the pending block,
/// flip status to `InputRequired`, and — if a turn is in flight — emit the
/// transition immediately (before this handler returns), per the issue's own
/// "surfaced immediately, not after a poll" requirement.
fn store_pending(shared: &Arc<Mutex<Shared>>, block: PendingBlock) {
    log_debug(
        LogDirection::In,
        "request_permission_or_elicitation",
        vec![("kind", format!("{:?}", block.kind))],
        None,
    );
    let mut guard = lock(shared);
    guard.pending = Some(block);
    guard.status = Status::InputRequired;
    if let Some(tx) = &guard.current_events {
        let _ = tx.send(DriverEvent::State(DriverState::InputRequired));
    }
}

fn handle_permission_request(
    shared: &Arc<Mutex<Shared>>,
    request: v2::RequestPermissionRequest,
    responder: Responder<v2::RequestPermissionResponse>,
) {
    let (fields, unsupported) = permission_fields(&request);
    store_pending(
        shared,
        PendingBlock {
            fields,
            responder: PendingResponder::Permission(responder),
            unsupported,
            kind: PendingKind::Permission,
            prompt: request.title.clone(),
        },
    );
}

fn handle_elicitation_request(
    shared: &Arc<Mutex<Shared>>,
    request: v2::CreateElicitationRequest,
    responder: Responder<v2::CreateElicitationResponse>,
) {
    let (fields, unsupported) = elicitation_fields(&request);
    store_pending(
        shared,
        PendingBlock {
            fields,
            responder: PendingResponder::Elicitation(responder),
            unsupported,
            kind: PendingKind::Elicitation,
            prompt: request.message.clone(),
        },
    );
}

/// `initialize` + `session/new`, plus the crash watcher. Split out of
/// [`run`] so the handshake's own error path is a plain `?` rather than
/// hand-threading a result through the outer `tokio::select!` shell.
async fn do_handshake(
    connection: &V2ConnectionTo<Agent>,
    cwd: &Path,
    shared: &Arc<Mutex<Shared>>,
) -> Result<v2::SessionId, agent_client_protocol::Error> {
    log_debug(LogDirection::Out, "initialize", vec![], None);
    let initialize = connection
        .send_request(v2::InitializeRequest::new(
            agent_client_protocol::schema::ProtocolVersion::V2,
            v2::Implementation::new("holler-body", env!("CARGO_PKG_VERSION")),
        ))
        .block_task()
        .await?;
    if initialize.capabilities.session.is_none() {
        log_warn("initialize", vec![("event", "agent did not advertise the v2 session capability".to_string())]);
        return Err(agent_client_protocol::Error::invalid_params()
            .data("agent did not advertise the v2 session capability"));
    }

    log_debug(LogDirection::Out, "session/new", vec![], None);
    let opened = connection
        .build_session(cwd)
        .start_session()
        .block_task()
        .await?;
    let session = opened.into_session();
    let session_id = session.session_id().clone();

    // Crash watcher: if the transport closes at any point (mid-turn, or while
    // a caller is waiting), surface it as `Done(Error)` / a resolved `cancel`
    // wait rather than a silent hang. `connection.spawn` ties this task's
    // lifetime to the connection's own, so it is cleaned up automatically when
    // the connection ends normally too.
    let watch_shared = shared.clone();
    let watch_conn = connection.clone();
    connection.spawn(async move {
        watch_conn.incoming_closed().await;
        // No pid/exit status is available here — see `AcpDriver::spawn`'s own
        // doc comment on why the pinned SDK never surfaces it to this crate.
        log_warn("spawn", vec![("event", "child_exit: connection closed".to_string())]);
        let mut guard = lock(&watch_shared);
        guard.last_stop_reason = Some(StopReason::Error);
        if let Some(tx) = guard.current_events.take() {
            let _ = tx.send(DriverEvent::Done(StopReason::Error));
        }
        if let Some(done_tx) = guard.awaiting_done.take() {
            let _ = done_tx.send(StopReason::Error);
        }
        guard.status = Status::Idle;
        Ok(())
    })?;

    Ok(session_id)
}

/// The connection's whole lifetime, run as one background tokio task: builds
/// the typed v2 client (notification + the two answerable-blocking request
/// handlers), spawns the child, brings up the session, reports readiness (or
/// the startup failure) through `ready_tx`, then idles until `shutdown_rx`
/// fires or the transport closes on its own.
pub(super) async fn run(
    agent_config: AcpAgentConfig,
    cwd: PathBuf,
    shared: Arc<Mutex<Shared>>,
    ready_tx: oneshot::Sender<Result<Ready, String>>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    let agent = AcpAgent::new(agent_config);
    let notif_shared = shared.clone();
    let perm_shared = shared.clone();
    let elic_shared = shared.clone();
    let handshake_shared = shared.clone();
    let mut shutdown_rx = shutdown_rx;

    let _ = Client
        .v2()
        .name("holler-body")
        .on_receive_notification(
            move |note: v2::UpdateSessionNotification, _cx: V2ConnectionTo<Agent>| {
                let shared = notif_shared.clone();
                async move {
                    handle_notification(&shared, note);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            move |request: v2::RequestPermissionRequest,
                  responder: Responder<v2::RequestPermissionResponse>,
                  _cx: V2ConnectionTo<Agent>| {
                let shared = perm_shared.clone();
                async move {
                    handle_permission_request(&shared, request, responder);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            move |request: v2::CreateElicitationRequest,
                  responder: Responder<v2::CreateElicitationResponse>,
                  _cx: V2ConnectionTo<Agent>| {
                let shared = elic_shared.clone();
                async move {
                    handle_elicitation_request(&shared, request, responder);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, move |connection: V2ConnectionTo<Agent>| async move {
            match do_handshake(&connection, &cwd, &handshake_shared).await {
                Ok(session_id) => {
                    let _ = ready_tx.send(Ok(Ready {
                        session_id,
                        connection: connection.clone(),
                    }));
                    tokio::select! {
                        _ = &mut shutdown_rx => {}
                        _ = connection.incoming_closed() => {}
                    }
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                }
            }
            Ok(())
        })
        .await;
}
