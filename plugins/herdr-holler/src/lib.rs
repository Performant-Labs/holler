//! The Herdr display plugin's library (issue #651, epic #633): read the hub's
//! pane registry and show it in Herdr's sidebar. **Display only** — the crate's
//! sources and its manifest may name no Herdr method beyond
//! `pane.report_metadata`, `workspace.report_metadata` and the read-only set
//! (`ping`, `session.snapshot`, `pane.get`, `pane.list`, `pane.read`,
//! `events.subscribe`); `tests/display_only_allowlist.rs` fails the crate if
//! they do. Every action the manifest offers runs a `holler pane` verb; the
//! plugin itself never changes a pane, a session or a layout.
//!
//! One [`Reporter::refresh`] reads `pane/list` from the hub's control socket
//! through the merged one-shot client (`holler_hub::control::run`) and sends the
//! reports (`report` builds them) over the plugin's own minimal Herdr client
//! (`herdr`). A failed hub read never leaves the last facts reading as current:
//! the panes reported last time are re-reported `unknown`.

mod herdr;
mod report;

use std::path::PathBuf;
use std::time::Duration;

use holler_hub::control::{self, ControlCall};
use holler_hub::state::{control_sock_path, resolve_state_dir, HubState};
use holler_pane::{Pane, PaneId, PaneName, PaneReply};
use serde_json::Value;

use herdr::{Client, Failure, Method};

/// The env var naming Herdr's session socket in a plugin process.
const HERDR_SOCKET_VAR: &str = "HERDR_SOCKET_PATH";

/// The hub's read-only registry list: the one control method the plugin calls.
const PANE_LIST: &str = "pane/list";

/// How long the hub read may take (the control client's one-shot timeout).
const HUB_TIMEOUT: Duration = Duration::from_secs(5);

/// The two peers one refresh talks to, resolved from the environment a Herdr
/// plugin process receives.
#[derive(Debug, Clone)]
pub struct Endpoints {
    /// Herdr's session socket: the `HERDR_SOCKET_PATH` the plugin process is
    /// given.
    pub herdr_socket: PathBuf,
    /// The hub's control socket: `<HOLLER_STATE_DIR>/hub/control.sock`, the
    /// same resolution `holler_hub::control::run` makes (the hub read honours
    /// the ambient `HOLLER_STATE_DIR`).
    pub hub_control_socket: PathBuf,
}

impl Endpoints {
    /// Resolve both endpoints from the environment. `Err(PlugError::Env)`
    /// naming the missing variable when either is absent.
    pub fn from_env() -> Result<Self, PlugError> {
        let herdr_socket = match std::env::var_os(HERDR_SOCKET_VAR) {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => {
                return Err(PlugError::Env(format!(
                    "{HERDR_SOCKET_VAR} is not set (Herdr sets it for a plugin's process)"
                )))
            }
        };
        let state_dir = resolve_state_dir().ok_or_else(|| {
            PlugError::Env("HOLLER_STATE_DIR is not set and $HOME is unavailable".to_owned())
        })?;
        Ok(Self {
            herdr_socket,
            hub_control_socket: control_sock_path(&HubState::from_root(state_dir)),
        })
    }
}

/// Runs refreshes, remembering which panes it last reported so a failed hub
/// read re-reports exactly those as `unknown` — never the previous facts again.
pub struct Reporter {
    herdr: Client,
    hub_control_socket: PathBuf,
    /// The Herdr pane ids of the last hub read that succeeded.
    reported: Vec<PaneId>,
    /// Why the last hub read failed; `None` after one that succeeded.
    hub_error: Option<String>,
}

impl Reporter {
    /// A reporter talking to `endpoints`.
    pub fn new(endpoints: Endpoints) -> Self {
        Self {
            herdr: Client::new(endpoints.herdr_socket),
            hub_control_socket: endpoints.hub_control_socket,
            reported: Vec::new(),
            hub_error: None,
        }
    }

    /// One refresh: read `pane/list` from the hub's control socket, then send
    /// one `pane.report_metadata` per registry pane addressed by its
    /// `herdr.pane_id` (tokens `pos`, `project`, `shown`, `driven`, `sync`,
    /// `hold`; `ttl_ms` set) and one `workspace.report_metadata` per mapped
    /// workspace (token `profile`; **no** `ttl_ms`), all under the source id
    /// `holler`. When the hub read fails, send for every pane shown last time
    /// a `pane.report_metadata` with `state_labels: ["unknown"]`, every token
    /// `unknown`, and `ttl_ms` — and nothing else.
    ///
    /// `Err(PlugError::Herdr)` when a report did not land: at once when the
    /// exchange itself fails, or after the other reports when Herdr refused
    /// some. A failed hub read is not an error: it is the `unknown` refresh,
    /// and [`Reporter::hub_error`] says why.
    pub fn refresh(&mut self) -> Result<Refreshed, PlugError> {
        let records = match self.read_registry() {
            Ok(records) => records,
            Err(cause) => {
                self.hub_error = Some(cause);
                let reports = self
                    .reported
                    .iter()
                    .map(|pane_id| (Method::PaneReport, report::unknown_report(pane_id)))
                    .collect();
                self.send_all(reports)?;
                return Ok(Refreshed {
                    pane_reports: self.reported.len(),
                    workspace_reports: 0,
                    unknown: true,
                });
            }
        };
        self.hub_error = None;
        self.reported = records
            .iter()
            .map(|record| record.herdr.pane_id.clone())
            .collect();
        let workspaces = report::workspace_reports(&records);
        let workspace_reports = workspaces.len();
        let reports = records
            .iter()
            .map(|record| (Method::PaneReport, report::pane_report(record)))
            .chain(
                workspaces
                    .into_iter()
                    .map(|params| (Method::WorkspaceReport, params)),
            )
            .collect();
        self.send_all(reports)?;
        Ok(Refreshed {
            pane_reports: records.len(),
            workspace_reports,
            unknown: false,
        })
    }

    /// Why the last refresh could not read the hub, when it could not.
    pub fn hub_error(&self) -> Option<&str> {
        self.hub_error.as_deref()
    }

    /// The registry, as `pane/list` answers it; any failure is its cause.
    ///
    /// `control::run` dials the socket the ambient `HOLLER_STATE_DIR` names, so
    /// the read is refused when that is not this reporter's hub socket: the
    /// plugin never reports one hub's panes while it names another.
    fn read_registry(&self) -> Result<Vec<Pane>, String> {
        let dialled = control::sock_path();
        if dialled != self.hub_control_socket {
            return Err(format!(
                "the hub's client dials {dialled:?}, not this reporter's hub socket {:?}",
                self.hub_control_socket
            ));
        }
        let call = ControlCall {
            method: PANE_LIST,
            params: None,
            timeout: HUB_TIMEOUT,
        };
        let result = control::run(&call)
            .map_err(|e| format!("{PANE_LIST} on {:?} failed: {e}", self.hub_control_socket))?;
        let reply: PaneReply = serde_json::from_value(result)
            .map_err(|e| format!("the {PANE_LIST} reply is not a pane reply: {e}"))?;
        let data = reply
            .into_result()
            .map_err(|e| format!("the hub refused {PANE_LIST}: {e}"))?
            .ok_or_else(|| format!("the {PANE_LIST} reply carried no data"))?;
        serde_json::from_value(data)
            .map_err(|e| format!("the {PANE_LIST} data is not a list of panes: {e}"))
    }

    /// Send every report. A fault stops the refresh at once; a refusal stops
    /// only its own report, and the refresh fails after the rest are sent.
    fn send_all(&self, reports: Vec<(Method, Value)>) -> Result<(), PlugError> {
        let total = reports.len();
        let mut refused = Vec::new();
        for (method, params) in reports {
            match self.herdr.send(method, params) {
                Ok(()) => {}
                Err(Failure::Refused(why)) => refused.push(why),
                Err(Failure::Fault(why)) => return Err(PlugError::Herdr(why)),
            }
        }
        match refused.first() {
            None => Ok(()),
            Some(first) => Err(PlugError::Herdr(format!(
                "{} of {total} reports were refused; the first: {first}",
                refused.len()
            ))),
        }
    }
}

/// What one refresh did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refreshed {
    /// `pane.report_metadata` reports sent.
    pub pane_reports: usize,
    /// `workspace.report_metadata` reports sent.
    pub workspace_reports: usize,
    /// The hub read failed; the panes shown last time were re-reported
    /// `unknown`.
    pub unknown: bool,
}

/// Why a refresh could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlugError {
    /// An endpoint could not be resolved from the environment.
    Env(String),
    /// The Herdr report exchange failed (socket, framing or reply).
    Herdr(String),
}

impl std::fmt::Display for PlugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlugError::Env(why) => write!(f, "cannot resolve the endpoints: {why}"),
            PlugError::Herdr(why) => write!(f, "cannot report to Herdr: {why}"),
        }
    }
}

impl std::error::Error for PlugError {}

/// The registry pane a Herdr action's `HERDR_PANE_ID` addresses: the record
/// whose `herdr.pane_id` equals `herdr_pane_id`, returned by pane name so the
/// action runs a `holler pane` verb on it. `None` when no record matches.
pub fn resolve_action_pane(panes: &[Pane], herdr_pane_id: &str) -> Option<PaneName> {
    panes
        .iter()
        .find(|record| record.herdr.pane_id.as_str() == herdr_pane_id)
        .map(|record| record.name.clone())
}
