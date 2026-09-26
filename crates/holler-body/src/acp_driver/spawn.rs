//! The two spawn attempts `AcpDriver::spawn` (in the parent module) tries in
//! order — ACP v2, then the v1 fallback (issue #362) — split out purely to
//! keep `acp_driver.rs` under the workspace's 900-line file-size guard
//! (`scripts/lint.sh`); this is not a new concern, it is the same "how do we
//! bring up a connection and wait for readiness" logic `acp_driver.rs` had
//! inline before the v1 fallback made it long enough to need two attempts.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::AcpAgentConfig;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use holler_proto::log::Direction as LogDirection;

use super::auth::{self, AuthProgress};
use super::connection::{self, Ready, Shared};
use super::connection_v1::{self, ReadyV1};
use super::{log_debug, log_warn, AcpDriver, Conn, DriverError, Status};

/// The outcome of one spawn attempt that did not produce a ready driver
/// (issue #362's v1 fallback).
pub(super) enum SpawnAttempt {
    /// The v2 attempt's `initialize` failed specifically because the peer
    /// only negotiated ACP v1 — retry with [`AcpDriver::spawn_v1`], not a
    /// fatal [`DriverError`].
    FallbackToV1 { reason: String },
    /// Every other startup failure (a bad command, a timeout, a crash) —
    /// fatal in either protocol mode.
    Fatal(DriverError),
}

/// Whether a v2 `initialize` failure is specifically the crate's own
/// negotiated-version guard refusing a peer that only speaks v1 — as opposed
/// to a transport failure, a crash, or any other startup error, which are
/// fatal regardless of protocol. Matches the exact wording of
/// `agent_client_protocol`'s `required_protocol_version` error (the crate's
/// own `V2ConnectionTo::send_request`/`send_request_to` version guard,
/// `jsonrpc/protocol_compat.rs`) — narrowly scoped to this one call site
/// (`connection::do_handshake`'s `initialize` request) rather than a general
/// error-classification rule, since that is the only place this driver can
/// observe this specific failure.
pub(super) fn is_v1_negotiation_failure(reason: &str) -> bool {
    reason.contains("required ACP protocol version") && reason.contains("peer negotiated")
}

impl AcpDriver {
    /// One ACP v2 spawn attempt: builds a fresh `Shared`, runs
    /// `connection::run` in a background task, and waits (bounded by
    /// `timeout_ms`) for it to signal readiness or a startup failure.
    pub(super) async fn spawn_v2(
        agent_config: AcpAgentConfig,
        session_cwd: PathBuf,
        timeout_ms: u64,
    ) -> Result<Self, SpawnAttempt> {
        let shared = Arc::new(Mutex::new(Shared {
            status: Status::Idle,
            pending: None,
            current_events: None,
            awaiting_done: None,
            last_stop_reason: None,
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

        match tokio::time::timeout(Duration::from_millis(timeout_ms), ready_rx).await {
            Ok(Ok(Ok(ready))) => {
                log_debug(
                    LogDirection::In,
                    "spawn",
                    vec![("event", "spawned".to_string()), ("protocol", "v2".to_string())],
                    None,
                );
                Ok(Self {
                    conn: Conn::V2 {
                        session: ready.session_id,
                        connection: ready.connection,
                    },
                    shared,
                    join_handle: Mutex::new(Some(join_handle)),
                    shutdown_tx: Mutex::new(Some(shutdown_tx)),
                })
            }
            Ok(Ok(Err(reason))) => {
                join_handle.abort();
                if is_v1_negotiation_failure(&reason) {
                    Err(SpawnAttempt::FallbackToV1 { reason })
                } else {
                    log_warn("spawn", vec![("event", format!("startup_failed: {reason}"))]);
                    Err(SpawnAttempt::Fatal(DriverError::Startup(reason)))
                }
            }
            Ok(Err(_dropped)) => {
                join_handle.abort();
                log_warn("spawn", vec![("event", "connection task ended before readiness".to_string())]);
                Err(SpawnAttempt::Fatal(DriverError::Startup(
                    "connection task ended before signalling readiness".to_string(),
                )))
            }
            Err(_timed_out) => {
                // Kill the still-hung child by ending its owning task; the
                // SDK's `AcpAgent` transport installs a guard that tears down
                // the spawned process group when the connection future is
                // dropped (which `abort` forces).
                join_handle.abort();
                log_warn("spawn", vec![("event", format!("startup_timeout: {timeout_ms}ms"))]);
                Err(SpawnAttempt::Fatal(DriverError::Startup(format!(
                    "no response within {timeout_ms}ms (HOLLER_ACP_TIMEOUT_MS)"
                ))))
            }
        }
    }

    /// The ACP v1 fallback's own [`Self::spawn_v2`] (issue #362) — spawns a
    /// **fresh** child (the v2 attempt's child already exited or was
    /// force-killed by that attempt's own `join_handle.abort()`) and runs
    /// `connection_v1::run` instead. `auth_method` is the session's
    /// configured ACP auth method id (issue #439); the whole handshake,
    /// including any `authenticate` and the one retried `session/new`, stays
    /// inside this attempt's single `timeout_ms` window.
    pub(super) async fn spawn_v1(
        agent_config: AcpAgentConfig,
        session_cwd: PathBuf,
        timeout_ms: u64,
        auth_method: Option<String>,
    ) -> Result<Self, SpawnAttempt> {
        let shared = Arc::new(Mutex::new(Shared {
            status: Status::Idle,
            pending: None,
            current_events: None,
            awaiting_done: None,
            last_stop_reason: None,
        }));

        let (ready_tx, ready_rx) = oneshot::channel::<Result<ReadyV1, String>>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let progress = Arc::new(AuthProgress::default());
        // The one place every v1 startup failure passes through: what became
        // of a configured `auth_method` is appended here, from how far the
        // handshake's auth flow got, so no failure path is silent about it.
        let configured = auth_method.clone();
        let stage_progress = progress.clone();
        let annotate = move |reason: String| match &configured {
            Some(id) => format!("{reason}{}", auth::v1_stage_suffix(id, stage_progress.get())),
            None => reason,
        };

        let task_shared = shared.clone();
        let join_handle: JoinHandle<()> = tokio::spawn(async move {
            connection_v1::run(
                agent_config,
                session_cwd,
                auth_method,
                progress,
                task_shared,
                ready_tx,
                shutdown_rx,
            )
            .await;
        });

        match tokio::time::timeout(Duration::from_millis(timeout_ms), ready_rx).await {
            Ok(Ok(Ok(ready))) => {
                log_debug(
                    LogDirection::In,
                    "spawn",
                    vec![("event", "spawned".to_string()), ("protocol", "v1".to_string())],
                    None,
                );
                Ok(Self {
                    conn: Conn::V1 {
                        session: ready.session_id,
                        connection: ready.connection,
                    },
                    shared,
                    join_handle: Mutex::new(Some(join_handle)),
                    shutdown_tx: Mutex::new(Some(shutdown_tx)),
                })
            }
            Ok(Ok(Err(reason))) => {
                join_handle.abort();
                let reason = annotate(reason);
                log_warn("spawn", vec![("event", format!("startup_failed (v1 fallback): {reason}"))]);
                Err(SpawnAttempt::Fatal(DriverError::Startup(reason)))
            }
            Ok(Err(_dropped)) => {
                join_handle.abort();
                log_warn("spawn", vec![("event", "v1 fallback connection task ended before readiness".to_string())]);
                Err(SpawnAttempt::Fatal(DriverError::Startup(annotate(
                    "connection task ended before signalling readiness (v1 fallback)".to_string(),
                ))))
            }
            Err(_timed_out) => {
                join_handle.abort();
                log_warn("spawn", vec![("event", format!("startup_timeout (v1 fallback): {timeout_ms}ms"))]);
                Err(SpawnAttempt::Fatal(DriverError::Startup(annotate(format!(
                    "no response within {timeout_ms}ms (HOLLER_ACP_TIMEOUT_MS, v1 fallback)"
                )))))
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #439
mod tests {
    use super::is_v1_negotiation_failure;

    /// Issue #439: an auth-required failure must never be classified as the v1
    /// negotiation failure, or a v2 auth failure would silently fall back to
    /// v1 and authenticate there.
    #[test]
    fn auth_required_is_never_a_v1_negotiation_failure() {
        let canonical = agent_client_protocol::Error::auth_required();
        let with_data = agent_client_protocol::Error::auth_required()
            .data(serde_json::json!({ "hint": "required ACP protocol version" }));
        // Only `data` carrying BOTH phrases the classifier needs would defeat
        // it; that is adapter-controlled text and out of scope here (the
        // classifier predates #439), so the test asserts the realistic shapes.
        let with_both = agent_client_protocol::Error::auth_required()
            .data(serde_json::json!({ "hint": "required ACP protocol version; peer negotiated 1" }));
        assert!(
            is_v1_negotiation_failure(&with_both.to_string()),
            "documents the known limit: adapter `data` with both phrases matches the classifier"
        );
        for reason in ["Authentication required".to_string(), canonical.to_string(), with_data.to_string()] {
            assert!(!is_v1_negotiation_failure(&reason), "{reason}");
        }
        assert!(is_v1_negotiation_failure(
            "required ACP protocol version 2 but peer negotiated 1; use a matching implementation"
        ));
    }
}
