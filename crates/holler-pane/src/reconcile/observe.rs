//! One pane's chain in a reconcile pass (#647): observe the pane's tmux session, harness
//! server and TUI, compare them with its record, repair a mismatch with `--fix`, and record
//! what was observed. A pass runs one chain per pane, each on a thread of its own; the
//! parent module has the rules and what the record write means.
//!
//! A chain makes at most seven port calls, each bounded by I5: `ps`, `health`,
//! `list_sessions`, `shown_session`, and with `--fix` `select_session` and `shown_session`
//! again, then one `cas_put`.

use crate::findings::{quoted, Finding, FindingKind, FixError, FixState};
use crate::pane::PaneRole;
use crate::{Pane, PaneError, Ports};

use super::{failed_message, shown_differs, ObservedHealth, PaneSummary, ReconcileRequest};

/// The calls of a chain, named as `<port>.<method>` in an `observe-failed` message.
const HOST_PS: &str = "host.ps";
const HARNESS_HEALTH: &str = "harness.health";
const HARNESS_LIST_SESSIONS: &str = "harness.list_sessions";
const HARNESS_SHOWN_SESSION: &str = "harness.shown_session";
const PANE_STORE_CAS_PUT: &str = "pane_store.cas_put";

/// What one pane's chain found and observed.
pub(super) struct Checked {
    pub(super) findings: Vec<Finding>,
    pub(super) summary: PaneSummary,
    /// The sessions of the pane's server, when it passed its health check and its list was
    /// read: the only list the stray rule trusts.
    pub(super) listed: Option<Vec<String>>,
}

/// The pane's harness server, as the chain observed it.
enum Server {
    /// It passed its health check; `sessions` is its list, `None` when reading it failed.
    Healthy { sessions: Option<Vec<String>> },
    /// It failed its health check, and its list timed out: it accepts and never answers.
    Wedged,
    /// It failed its health check, and its list did anything but time out.
    Down,
    /// The health check itself failed: nothing is known of the server.
    Unknown,
}

impl Server {
    /// The sessions of a healthy server, when they were read: the only list a rule trusts
    /// (an unhealthy server's answer is not).
    fn sessions(&self) -> Option<&[String]> {
        match self {
            Server::Healthy { sessions } => sessions.as_deref(),
            Server::Wedged | Server::Down | Server::Unknown => None,
        }
    }

    /// Whether the server's trusted list holds `session`.
    fn has(&self, session: &str) -> bool {
        self.sessions()
            .is_some_and(|ids| ids.iter().any(|id| id == session))
    }

    fn health(&self) -> ObservedHealth {
        match self {
            Server::Healthy { .. } => ObservedHealth::Healthy,
            Server::Wedged => ObservedHealth::ServerWedged,
            Server::Down => ObservedHealth::ServerDown,
            Server::Unknown => ObservedHealth::Unknown,
        }
    }

    /// The trusted list, owned.
    fn into_sessions(self) -> Option<Vec<String>> {
        match self {
            Server::Healthy { sessions } => sessions,
            Server::Wedged | Server::Down | Server::Unknown => None,
        }
    }
}

/// What the chain knows of the TUI's screen.
enum Screen {
    /// The session it shows; `None` is its home screen, or no TUI in the pane.
    Seen(Option<String>),
    /// `shown_session` failed: every rule that needs the screen is skipped.
    Unseen,
}

impl Screen {
    fn shown(&self) -> Option<&str> {
        match self {
            Screen::Seen(shown) => shown.as_deref(),
            Screen::Unseen => None,
        }
    }
}

/// Run the chain of `pane`: observe, compare, repair, record.
pub(super) fn check_pane(ports: Ports<'_>, request: &ReconcileRequest<'_>, pane: &Pane) -> Checked {
    let mut findings = Vec::new();
    check_tmux(ports, pane, &mut findings);
    let server = observe_server(ports, pane, &mut findings);
    let screen = observe_screen(ports, pane, &mut findings);
    check_session_of_record(pane, &server, &mut findings);
    let screen = check_screen(ports, request, pane, &server, screen, &mut findings);
    findings.extend(record(ports, request, pane, &server, &screen));
    Checked {
        findings,
        summary: summary(pane, screen.shown(), server.health()),
        listed: server.into_sessions(),
    }
}

/// The chain of `pane` did not finish (its thread could not start, or panicked): one
/// `observe-failed`, and nothing is concluded.
pub(super) fn lost(pane: &Pane) -> Checked {
    let message = "the pane was not observed: its check did not finish".to_owned();
    Checked {
        findings: vec![Finding::about(FindingKind::ObserveFailed, pane, message)],
        summary: summary(pane, None, ObservedHealth::Unknown),
        listed: None,
    }
}

fn summary(pane: &Pane, shown: Option<&str>, health: ObservedHealth) -> PaneSummary {
    PaneSummary {
        name: pane.name.clone(),
        grid: pane.herdr.grid,
        session_of_record: pane.session_of_record.clone(),
        shown: shown.map(str::to_owned),
        health,
    }
}

/// `tmux-session-missing` when the pane's tmux session (named as the pane) is gone.
fn check_tmux(ports: Ports<'_>, pane: &Pane, findings: &mut Vec<Finding>) {
    match ports.host.ps(&pane.name) {
        Ok(_) => {}
        Err(PaneError::PaneNotFound { .. }) => {
            let message = format!("the tmux session {} does not exist", pane.name);
            findings.push(Finding::about(
                FindingKind::TmuxSessionMissing,
                pane,
                message,
            ));
        }
        Err(error) => findings.push(observe_failed(pane, HOST_PS, &error)),
    }
}

/// The pane's server: its health check, then its list. A healthy server's list is read for
/// the rules; an unhealthy server's only tells a wedged server (the list times out) from a
/// down one, and is not trusted further.
fn observe_server(ports: Ports<'_>, pane: &Pane, findings: &mut Vec<Finding>) -> Server {
    let port = pane.harness.port;
    match ports.harness.health(port) {
        Err(error) => {
            findings.push(observe_failed(pane, HARNESS_HEALTH, &error));
            Server::Unknown
        }
        Ok(true) => match ports.harness.list_sessions(port) {
            Ok(ids) => Server::Healthy {
                sessions: Some(ids),
            },
            Err(error) => {
                findings.push(observe_failed(pane, HARNESS_LIST_SESSIONS, &error));
                Server::Healthy { sessions: None }
            }
        },
        Ok(false) => {
            let wedged = matches!(
                ports.harness.list_sessions(port),
                Err(PaneError::Timeout { .. })
            );
            let (server, kind, state) = if wedged {
                let state = "accepts a connection and does not answer";
                (Server::Wedged, FindingKind::ServerWedged, state)
            } else {
                let state = "does not pass its health check";
                (Server::Down, FindingKind::ServerDown, state)
            };
            let message = format!("the harness server on port {port} {state}");
            findings.push(Finding::about(kind, pane, message).with_ports(vec![port]));
            server
        }
    }
}

/// What the TUI in the pane shows.
fn observe_screen(ports: Ports<'_>, pane: &Pane, findings: &mut Vec<Finding>) -> Screen {
    match ports.harness.shown_session(&pane.herdr.pane_id) {
        Ok(shown) => Screen::Seen(shown),
        Err(error) => {
            findings.push(observe_failed(pane, HARNESS_SHOWN_SESSION, &error));
            Screen::Unseen
        }
    }
}

/// `no-session-of-record`, or `session-of-record-missing` when the pane's healthy server
/// listed its sessions without it.
fn check_session_of_record(pane: &Pane, server: &Server, findings: &mut Vec<Finding>) {
    let Some(record) = pane.session_of_record.as_deref() else {
        let message = "the record names no session of record, and doctor never picks one";
        findings.push(Finding::about(
            FindingKind::NoSessionOfRecord,
            pane,
            message.to_owned(),
        ));
        return;
    };
    if server.sessions().is_some() && !server.has(record) {
        let port = pane.harness.port;
        let message = format!(
            "the harness server on port {port} does not have the session of record {}",
            quoted(record)
        );
        let finding = Finding::about(FindingKind::SessionOfRecordMissing, pane, message);
        findings.push(finding.with_session(Some(record)).with_ports(vec![port]));
    }
}

/// The TUI's rules: `tui-foreign-session` when it shows a session its pane's own healthy
/// server does not have, or else `shown-driven-mismatch` when it shows another than the
/// session of record, repaired by [`repair`] when the record decides it. Returns the screen
/// at the end of the chain: after an attempted fix, what the TUI showed then.
fn check_screen(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    pane: &Pane,
    server: &Server,
    screen: Screen,
    findings: &mut Vec<Finding>,
) -> Screen {
    let Screen::Seen(shown) = &screen else {
        return screen;
    };
    let shown = shown.as_deref();
    if let (Some(session), Some(ids)) = (shown, server.sessions()) {
        if !ids.iter().any(|id| id == session) {
            findings.push(foreign_session(pane, session));
            return screen;
        }
    }
    let Some(record) = pane.session_of_record.as_deref() else {
        return screen;
    };
    if !shown_differs(Some(record), shown) {
        return screen;
    }
    let repair = repair(ports, request, pane, server, record);
    let tense = if repair.state == FixState::Fixed {
        "showed"
    } else {
        "shows"
    };
    let message = format!(
        "the TUI {tense} {}, not the session of record {}",
        screen_text(shown),
        quoted(record)
    );
    let finding = Finding::about(FindingKind::ShownDrivenMismatch, pane, message);
    findings.push(
        finding
            .with_session(shown)
            .with_fix(repair.state, repair.error),
    );
    repair.screen.unwrap_or(screen)
}

/// `tui-foreign-session` for `pane`, whose TUI shows `session`.
fn foreign_session(pane: &Pane, session: &str) -> Finding {
    let port = pane.harness.port;
    let message = format!(
        "the TUI shows session {}, which the pane's own server (port {port}) does not have: \
         a harness the record does not know is in the pane",
        quoted(session)
    );
    Finding::about(FindingKind::TuiForeignSession, pane, message)
        .with_session(Some(session))
        .with_ports(vec![port])
}

/// What `--fix` did about a mismatch.
struct Repair {
    state: FixState,
    error: Option<FixError>,
    /// What the TUI showed after the act; `None` when nothing was done.
    screen: Option<Screen>,
}

/// Plan the repair of a mismatch from the record: fixable when the pane's healthy server has
/// the session of record `record`; done only with `--fix`, and on the orchestrator's pane
/// only by a pass that names it (switching the orchestrator's own TUI under it is a
/// deliberate act).
fn repair(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    pane: &Pane,
    server: &Server,
    record: &str,
) -> Repair {
    let state = if !server.has(record) {
        FixState::NotFixable
    } else if !request.fix {
        FixState::Fixable
    } else if pane.role == PaneRole::Orchestrator && request.pane != Some(&pane.name) {
        FixState::Skipped
    } else {
        return select(ports, pane, record);
    };
    Repair {
        state,
        error: None,
        screen: None,
    }
}

/// Act and observe (I3): select `record` in the pane's TUI, then read what it shows. Fixed
/// only when the act succeeded and the TUI is then seen showing `record`.
fn select(ports: Ports<'_>, pane: &Pane, record: &str) -> Repair {
    let pane_id = &pane.herdr.pane_id;
    let selected = ports.harness.select_session(pane_id, record);
    let after = ports.harness.shown_session(pane_id);
    let (state, error) = match (&selected, &after) {
        (Err(error), _) | (Ok(()), Err(error)) => (FixState::Failed, Some(FixError::from(error))),
        (Ok(()), Ok(shown)) if shown.as_deref() == Some(record) => (FixState::Fixed, None),
        (Ok(()), Ok(shown)) => {
            let message = format!(
                "after selecting {}, the TUI shows {}",
                quoted(record),
                screen_text(shown.as_deref())
            );
            let code = FindingKind::ShownDrivenMismatch.code().to_owned();
            (FixState::Failed, Some(FixError { code, message }))
        }
    };
    Repair {
        state,
        error,
        screen: Some(after.map_or(Screen::Unseen, Screen::Seen)),
    }
}

/// How a message names what the TUI shows.
fn screen_text(shown: Option<&str>) -> String {
    shown.map_or_else(
        || "its home screen".to_owned(),
        |session| format!("session {}", quoted(session)),
    )
}

/// Record what the chain observed (`harness.health`, `last_observed.shown`, stamped
/// `request.now_ms`) when it differs from the record or is the record's first observation
/// (`at == 0`). One compare-and-swap at the generation the pass read, never retried; a
/// failed write is an `observe-failed` finding.
fn record(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    pane: &Pane,
    server: &Server,
    screen: &Screen,
) -> Option<Finding> {
    let health = server.health().recorded();
    if health.is_none() && matches!(screen, Screen::Unseen) {
        return None;
    }
    let mut next = pane.clone();
    if let Some(health) = health {
        next.harness.health = health;
    }
    if let Screen::Seen(shown) = screen {
        next.last_observed.shown.clone_from(shown);
    }
    if next == *pane && pane.last_observed.at != 0 {
        return None;
    }
    next.last_observed.at = request.now_ms;
    let error = ports.pane_store.cas_put(&next, pane.generation).err()?;
    let message = match error {
        PaneError::Conflict => format!(
            "{PANE_STORE_CAS_PUT} failed ({}): another writer changed the record during this \
             pass, so what this pass observed was not recorded",
            error.code()
        ),
        _ => failed_message(PANE_STORE_CAS_PUT, &error),
    };
    Some(Finding::about(FindingKind::ObserveFailed, pane, message))
}

/// `observe-failed` for the call `op` of `pane`'s chain.
fn observe_failed(pane: &Pane, op: &str, error: &PaneError) -> Finding {
    Finding::about(FindingKind::ObserveFailed, pane, failed_message(op, error))
}
