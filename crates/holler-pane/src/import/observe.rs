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
//! A chain makes at most four port calls, all reads, in this order: `host.ps`; once it has
//! answered, `host.foreground_commands` (#835); then `harness.health` and
//! `harness.list_sessions`. A tmux session that runs Claude Code
//! ([`is_claude_code_command`]) ends the chain after the second call: its pane is left
//! alone, so its server is never asked.

use std::thread::{self, ScopedJoinHandle};

use crate::findings::embedded;
use crate::pane::{is_claude_code_command, Health};
use crate::{HarnessEndpoint, PaneError, PaneName, Ports};

use super::findings::{Finding, ImportFinding};

/// The calls of a chain, named as `<port>.<method>` in a finding's message.
const HOST_PS: &str = "host.ps";
const HOST_FOREGROUND_COMMANDS: &str = "host.foreground_commands";
const HARNESS_HEALTH: &str = "harness.health";
const HARNESS_LIST_SESSIONS: &str = "harness.list_sessions";

/// The start of every `harness-unreadable` message: the pane is never recorded on a guess.
const UNREADABLE: &str = "what the pane's tmux session runs could not be read, so the pane is \
                          not recorded";

/// A pane to observe: its tmux session is named as the pane, and its server is on `port`
/// (`None` when the launcher table gives it no port, so there is no server to ask).
pub(super) struct Target<'a> {
    pub name: &'a PaneName,
    pub port: Option<u16>,
}

/// What one pane's chain observed.
pub(super) enum Observed {
    /// The pane's tmux session runs Claude Code: nothing more was asked about it.
    ClaudeCode,
    /// What was asked about any other pane.
    Seen(Chain),
    /// The chain did not finish (its thread could not start, or panicked).
    Lost,
}

/// A pane whose tmux session does not run Claude Code, or could not be read.
pub(super) struct Chain {
    /// `host.ps`, then `host.foreground_commands`: `Ok` when both answered, or the read that
    /// failed.
    pub tmux: Result<(), Unread>,
    /// The harness server; `None` when there was no port to ask.
    pub server: Option<Server>,
}

/// A read of the pane's tmux session that failed: the call, as `<port>.<method>`, and its
/// error.
pub(super) struct Unread {
    op: &'static str,
    error: PaneError,
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

/// One pane's chain: its tmux session, then, unless the session runs Claude Code, its server.
/// A session that could not be read is not known to run Claude Code, so its server is asked,
/// as for any pane.
fn observe(ports: Ports<'_>, target: &Target<'_>) -> Observed {
    let tmux = match read_tmux(ports, target.name) {
        Ok(true) => return Observed::ClaudeCode,
        Ok(false) => Ok(()),
        Err(unread) => Err(unread),
    };
    let server = target.port.map(|port| observe_server(ports, port));
    Observed::Seen(Chain { tmux, server })
}

/// Whether the tmux session `name` runs Claude Code: `host.ps`, then, once it has answered,
/// the command name of the foreground process of each of its tmux panes. Every other name, and
/// a session with no process, is not Claude Code.
fn read_tmux(ports: Ports<'_>, name: &PaneName) -> Result<bool, Unread> {
    ports
        .host
        .ps(name)
        .map_err(|error| Unread { op: HOST_PS, error })?;
    let commands = ports
        .host
        .foreground_commands(name)
        .map_err(|error| Unread {
            op: HOST_FOREGROUND_COMMANDS,
            error,
        })?;
    Ok(commands
        .iter()
        .any(|command| is_claude_code_command(command)))
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

/// What the reads of the pane's tmux session tell the report about `pane`:
/// `tmux-session-missing` when either answered `pane-not-found` (a session that does not
/// exist, or that ended between the two), `harness-unreadable` naming the call for any other
/// error. Either is a problem, so the pane is never recorded on a guess (#835).
pub(super) fn tmux_finding(pane: &PaneName, tmux: &Result<(), Unread>) -> Option<Finding> {
    match tmux {
        Ok(()) => None,
        Err(Unread {
            error: PaneError::PaneNotFound { .. },
            ..
        }) => Some(Finding::about(
            pane,
            ImportFinding::TmuxSessionMissing,
            format!("the tmux session {pane} does not exist"),
        )),
        Err(Unread { op, error }) => Some(Finding::about(
            pane,
            ImportFinding::HarnessUnreadable,
            format!("{UNREADABLE}: {}", failed_message(op, error)),
        )),
    }
}

/// What a chain that did not finish tells the report about `pane`: nothing was read, so
/// `harness-unreadable` (#835).
pub(super) fn lost_finding(pane: &PaneName) -> Finding {
    Finding::about(
        pane,
        ImportFinding::HarnessUnreadable,
        format!("{UNREADABLE}: its check did not finish"),
    )
}

/// The message of a failed call: the call, as `<port>.<method>`, the error's code and its
/// message (reconcile's form).
fn failed_message(op: &str, error: &PaneError) -> String {
    format!(
        "{op} failed ({}): {}",
        error.code(),
        embedded(&error.to_string())
    )
}
