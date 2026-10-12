//! What an import run observes of each pane (#650): its tmux session and its harness server,
//! one scoped thread per pane, so a run over wedged servers costs about one bound, not one per
//! pane (I5).
//!
//! The scaffold and the server's classification are reconcile's (`reconcile::observe_all`,
//! `reconcile::joined` and `observe_server`), copied on purpose: reconcile is #647's and
//! frozen, and it observes a pane through its record, which an import does not have yet. A
//! healthy server's list is trusted; an unhealthy server's list only tells a wedged server
//! (the list times out) from a down one.
//!
//! A chain makes at most three port calls, all reads: `host.ps`, `harness.health` and
//! `harness.list_sessions`.

use std::thread::{self, ScopedJoinHandle};

use crate::findings::embedded;
use crate::pane::Health;
use crate::{HarnessEndpoint, PaneError, PaneName, Ports};

use super::findings::{Finding, ImportFinding};

/// The calls of a chain, named as `<port>.<method>` in an `observe-failed` message.
const HOST_PS: &str = "host.ps";
const HARNESS_HEALTH: &str = "harness.health";
const HARNESS_LIST_SESSIONS: &str = "harness.list_sessions";

/// A pane to observe: its tmux session is named as the pane, and its server is on `port`
/// (`None` when the launcher table gives it no port, so there is no server to ask).
pub(super) struct Target<'a> {
    pub name: &'a PaneName,
    pub port: Option<u16>,
}

/// What one pane's chain observed.
pub(super) enum Observed {
    Seen {
        /// `host.ps`: `Ok` when the tmux session answered.
        tmux: Result<(), PaneError>,
        /// The harness server; `None` when there was no port to ask.
        server: Option<Server>,
    },
    /// The chain did not finish (its thread could not start, or panicked).
    Lost,
}

/// The pane's harness server, as the chain observed it.
pub(super) enum Server {
    /// It passed its health check; its list, or the error reading it.
    Healthy(Result<Vec<String>, PaneError>),
    /// It failed its health check, and its list timed out: it accepts and never answers.
    Wedged,
    /// It failed its health check, and its list did anything but time out.
    Down,
    /// The health check itself returned an error: nothing is known of the server.
    Unknown(PaneError),
}

/// Observe every target at once: one thread per target, the results in target order.
pub(super) fn observe_all(ports: Ports<'_>, targets: &[Target<'_>]) -> Vec<Observed> {
    thread::scope(|threads| {
        let chains: Vec<_> = targets
            .iter()
            .map(|target| {
                thread::Builder::new().spawn_scoped(threads, move || observe(ports, target))
            })
            .collect();
        chains
            .into_iter()
            .map(|chain| joined(chain).unwrap_or(Observed::Lost))
            .collect()
    })
}

/// What a thread returned, or `None` when it could not start or panicked.
fn joined<T>(thread: std::io::Result<ScopedJoinHandle<'_, T>>) -> Option<T> {
    thread.ok()?.join().ok()
}

/// One pane's chain: its tmux session, then its server.
fn observe(ports: Ports<'_>, target: &Target<'_>) -> Observed {
    let tmux = ports.host.ps(target.name).map(drop);
    let server = target.port.map(|port| observe_server(ports, port));
    Observed::Seen { tmux, server }
}

/// The server on `port`: its health check, then one list. A healthy server's list is read
/// for the session of record; an unhealthy server's only tells wedged from down.
fn observe_server(ports: Ports<'_>, port: u16) -> Server {
    let at = HarnessEndpoint::Port(port);
    match ports.harness.health(&at) {
        Err(error) => Server::Unknown(error),
        Ok(true) => Server::Healthy(ports.harness.list_sessions(&at)),
        Ok(false) => {
            let wedged = matches!(
                ports.harness.list_sessions(&at),
                Err(PaneError::Timeout { .. })
            );
            if wedged {
                Server::Wedged
            } else {
                Server::Down
            }
        }
    }
}

impl Server {
    /// The record's `harness.health`: `healthy` when the health check passed and the list
    /// was read; `Unhealthy("server-wedged")` or `Unhealthy("server-down")` as reconcile
    /// records them; `unknown` when a call returned an error.
    pub(super) fn health(&self) -> Health {
        match self {
            Server::Healthy(Ok(_)) => Health::Healthy,
            Server::Wedged => Health::Unhealthy(ImportFinding::ServerWedged.code().to_owned()),
            Server::Down => Health::Unhealthy(ImportFinding::ServerDown.code().to_owned()),
            Server::Healthy(Err(_)) | Server::Unknown(_) => Health::Unknown,
        }
    }

    /// The sessions of a healthy server whose list was read: the only list the import
    /// trusts.
    pub(super) fn sessions(&self) -> Option<&[String]> {
        match self {
            Server::Healthy(Ok(ids)) => Some(ids),
            Server::Healthy(Err(_)) | Server::Wedged | Server::Down | Server::Unknown(_) => None,
        }
    }

    /// What the server tells the report about `pane`, whose server is on `port`.
    pub(super) fn finding(&self, pane: &PaneName, port: u16) -> Option<Finding> {
        let (kind, message) = match self {
            Server::Healthy(Ok(_)) => return None,
            Server::Healthy(Err(error)) => (
                ImportFinding::ObserveFailed,
                failed_message(HARNESS_LIST_SESSIONS, error),
            ),
            Server::Unknown(error) => (
                ImportFinding::ObserveFailed,
                failed_message(HARNESS_HEALTH, error),
            ),
            Server::Wedged => (
                ImportFinding::ServerWedged,
                format!(
                    "the harness server on port {port} accepts a connection and does not answer"
                ),
            ),
            Server::Down => (
                ImportFinding::ServerDown,
                format!("the harness server on port {port} does not pass its health check"),
            ),
        };
        Some(Finding::about(pane, kind, message))
    }
}

/// What `host.ps` tells the report about `pane`: `tmux-session-missing` for `pane-not-found`
/// (a session that does not exist), `observe-failed` for any other error.
pub(super) fn tmux_finding(pane: &PaneName, tmux: &Result<(), PaneError>) -> Option<Finding> {
    match tmux {
        Ok(()) => None,
        Err(PaneError::PaneNotFound { .. }) => Some(Finding::about(
            pane,
            ImportFinding::TmuxSessionMissing,
            format!("the tmux session {pane} does not exist"),
        )),
        Err(error) => Some(Finding::about(
            pane,
            ImportFinding::ObserveFailed,
            failed_message(HOST_PS, error),
        )),
    }
}

/// The message of an `observe-failed` finding: the call, as `<port>.<method>`, the error's
/// code and its message (reconcile's form).
fn failed_message(op: &str, error: &PaneError) -> String {
    format!(
        "{op} failed ({}): {}",
        error.code(),
        embedded(&error.to_string())
    )
}
