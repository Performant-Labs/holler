//! `HerdrAdapter`: the `HerdrPort` over a [`Transport`] (Herdr's local socket in
//! production).
//!
//! - **What it is told.** One Herdr session, that session's socket, and the rows and
//!   columns of each workspace it places panes in ([`HerdrConfig`]): Herdr has no grid,
//!   and nothing here finds a socket or reads an environment variable by itself.
//! - **One deadline per call.** Each port method takes its deadline once, on entry,
//!   `timeout` from then (I5's bound, [`DEFAULT_TIMEOUT`] unless configured), and every
//!   exchange of the call runs against that one deadline.
//! - **The version gate** runs at [`HerdrAdapter::connect`] and in every `version()`,
//!   so no adapter exists for a Herdr protocol it does not know. The other methods
//!   trust the gate that passed at connect.
//! - **`ensure_pane` is plan, act, observe.** It reads the workspace's grid off its
//!   grid tab, asks [`plan_splits`] for the one step that makes the cell (an occupied
//!   cell needs none, and a refusal is returned as it is), runs that step, and reads
//!   the tree back: the new pane must sit in the cell asked for. A pane that landed
//!   elsewhere is `unavailable` and is left where Herdr put it. The adapter never
//!   closes or moves a pane.
//! - **`snapshot` lists every workspace**, configured or not, by its label: the placed
//!   panes of each workspace's grid tab, by row and then column. A pane nested inside
//!   one cell, or in another tab, has no cell and is left out.
//! - **No state.** The adapter holds its config and its transport only, and caches no
//!   version, pane id or connection between calls.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use holler_pane::{HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId};
use serde_json::Value;

use crate::layout::{grid_of, GridMap};
use crate::plan::{plan_splits, Extent, Step, Target};
use crate::protocol::{
    check_supported, decode_reply, expect_ok, parse_layout_export, parse_pane_info, parse_pong,
    parse_read, parse_snapshot, parse_workspace_created, Request, ServerVersion, WorkspaceRef,
};
use crate::transport::{Transport, UnixSocketTransport};

/// I5's bound for one `HerdrPort` call.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// What the adapter is told (Herdr has no grid and no discoverable socket of Holler's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrConfig {
    /// The one Herdr session served.
    pub session: String,
    /// That session's socket, absolute.
    pub socket: PathBuf,
    /// Workspace label to rows by columns.
    pub workspaces: BTreeMap<String, Extent>,
    /// The bound of one `HerdrPort` call.
    pub timeout: Duration,
}

impl HerdrConfig {
    /// A config with no workspace and [`DEFAULT_TIMEOUT`].
    pub fn new(session: impl Into<String>, socket: impl Into<PathBuf>) -> Self {
        Self {
            session: session.into(),
            socket: socket.into(),
            workspaces: BTreeMap::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// This config, with the workspace `label` of `extent`.
    pub fn with_workspace(mut self, label: impl Into<String>, extent: Extent) -> Self {
        self.workspaces.insert(label.into(), extent);
        self
    }
}

/// The `HerdrPort` over `T`.
#[derive(Debug)]
pub struct HerdrAdapter<T = UnixSocketTransport> {
    config: HerdrConfig,
    transport: T,
}

impl HerdrAdapter<UnixSocketTransport> {
    /// Connect to the socket of `config`, as [`HerdrAdapter::connect_with`] does. This
    /// is the way production builds an adapter.
    pub fn connect(config: HerdrConfig) -> Result<Self, PaneError> {
        let transport = UnixSocketTransport::new(config.socket.clone());
        Self::connect_with(config, transport)
    }
}

impl<T: Transport> HerdrAdapter<T> {
    /// Connect over `transport`. A config the adapter cannot serve is `usage`, before
    /// any request. Then one `ping`: a Herdr protocol the adapter does not know is
    /// `herdr-version-unsupported`, and no adapter is made. Over a transport other than
    /// the socket this is a test seam; production uses [`HerdrAdapter::connect`].
    pub fn connect_with(config: HerdrConfig, transport: T) -> Result<Self, PaneError> {
        validate(&config)?;
        let adapter = Self { config, transport };
        adapter.supported_server(adapter.deadline()?)?;
        Ok(adapter)
    }

    /// The config it was connected with.
    pub fn config(&self) -> &HerdrConfig {
        &self.config
    }

    /// The deadline of a call that starts now.
    fn deadline(&self) -> Result<Instant, PaneError> {
        deadline_after(self.config.timeout)
    }

    /// The `result` of Herdr's reply to `request`.
    fn call(&self, request: &Request, deadline: Instant) -> Result<Value, PaneError> {
        decode_reply(request, &self.transport.exchange(request, deadline)?)
    }

    /// What `ping` reports, when its protocol is a supported one.
    fn supported_server(&self, deadline: Instant) -> Result<ServerVersion, PaneError> {
        let server = parse_pong(&self.call(&Request::Ping, deadline)?)?;
        check_supported(&server)?;
        Ok(server)
    }

    /// The grid read off the tree of the tab `tab`.
    fn grid(&self, tab: &str, deadline: Instant) -> Result<GridMap, PaneError> {
        let request = Request::LayoutExport {
            tab_id: tab.to_owned(),
        };
        let tree = parse_layout_export(&self.call(&request, deadline)?)?;
        Ok(grid_of(&tree))
    }

    /// The configured extent of `spec`'s workspace, once `spec` is in the session
    /// served. Neither check sends a request.
    fn extent_of(&self, spec: &HerdrSpec) -> Result<Extent, PaneError> {
        if spec.session != self.config.session {
            return Err(PaneError::Unavailable {
                what: format!(
                    "Herdr session {:?}: this adapter serves only the session {:?}",
                    spec.session, self.config.session
                ),
            });
        }
        self.config
            .workspaces
            .get(&spec.workspace)
            .copied()
            .ok_or_else(|| PaneError::Unavailable {
                what: format!(
                    "Herdr workspace {:?} of session {:?} is not configured, so its rows and \
                     columns are not known",
                    spec.workspace, spec.session
                ),
            })
    }

    /// The workspace labelled `label` as Herdr has it now. Two of that label are
    /// `unavailable`.
    fn workspace_grid(&self, label: &str, deadline: Instant) -> Result<WorkspaceGrid, PaneError> {
        let state = parse_snapshot(&self.call(&Request::SessionSnapshot, deadline)?)?;
        let Some(workspace) = state.workspace(label)? else {
            return Ok(WorkspaceGrid {
                tab: None,
                map: GridMap::default(),
            });
        };
        let tab = grid_tab(workspace)?;
        let map = self.grid(&tab, deadline)?;
        Ok(WorkspaceGrid {
            tab: Some(tab),
            map,
        })
    }

    /// Run `step`, the one step of the plan for `spec.grid`: the grid tab that holds the
    /// new pane, and the new pane.
    fn place(
        &self,
        step: &Step,
        workspace: &WorkspaceGrid,
        spec: &HerdrSpec,
        deadline: Instant,
    ) -> Result<(String, PaneId), PaneError> {
        match step {
            Step::CreateRoot => {
                let request = Request::WorkspaceCreate {
                    label: spec.workspace.clone(),
                };
                let (created, root) = parse_workspace_created(&self.call(&request, deadline)?)?;
                Ok((grid_tab(&created)?, root))
            }
            Step::Split {
                from,
                direction,
                ratio,
                ..
            } => {
                let (Some(tab), Some(target)) = (&workspace.tab, workspace.map.at(*from)) else {
                    return Err(PaneError::Unavailable {
                        what: format!(
                            "the plan for {} splits {from}, where workspace {:?} has no pane",
                            spec.grid, spec.workspace
                        ),
                    });
                };
                let request = Request::Split {
                    target: target.clone(),
                    direction: *direction,
                    ratio: *ratio,
                };
                let reply = self
                    .call(&request, deadline)
                    .map_err(|error| changed_under(error, target, spec))?;
                Ok((tab.clone(), parse_pane_info(&reply)?))
            }
        }
    }

    /// Read the tree of `tab` back: `made` must sit at `spec.grid`. Otherwise it is
    /// `unavailable`, and the pane is left where Herdr put it.
    fn confirm(
        &self,
        tab: &str,
        made: &PaneId,
        spec: &HerdrSpec,
        deadline: Instant,
    ) -> Result<(), PaneError> {
        let what = match self.grid(tab, deadline)?.position_of(made) {
            Some(landed) if landed == spec.grid => return Ok(()),
            Some(landed) => format!(
                "Herdr put the new pane {:?} at {landed}, not at {}; it is left where it landed",
                made.as_str(),
                spec.grid
            ),
            None => format!(
                "the new pane {:?} has no cell in workspace {:?}, so it is not at {}; it is left \
                 where Herdr put it",
                made.as_str(),
                spec.workspace,
                spec.grid
            ),
        };
        Err(PaneError::Unavailable { what })
    }
}

impl<T: Transport> HerdrPort for HerdrAdapter<T> {
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
        let deadline = self.deadline()?;
        let extent = self.extent_of(spec)?;
        let workspace = self.workspace_grid(&spec.workspace, deadline)?;
        let target = Target {
            extent,
            cells: vec![spec.grid],
        };
        let pane_id = match plan_splits(&workspace.map, &target)?.as_slice() {
            // An occupied cell: its pane, and no request that changes anything.
            [] => workspace
                .map
                .at(spec.grid)
                .cloned()
                .ok_or_else(|| PaneError::Unavailable {
                    what: format!(
                        "the plan for {} is empty, and workspace {:?} has no pane there",
                        spec.grid, spec.workspace
                    ),
                })?,
            [step] => {
                let (tab, made) = self.place(step, &workspace, spec, deadline)?;
                self.confirm(&tab, &made, spec, deadline)?;
                made
            }
            steps => {
                return Err(PaneError::Unavailable {
                    what: format!(
                        "the plan for {} has {} steps, and ensure_pane makes one pane",
                        spec.grid,
                        steps.len()
                    ),
                })
            }
        };
        Ok(HerdrPane {
            session: self.config.session.clone(),
            workspace: spec.workspace.clone(),
            pane_id,
            grid: spec.grid,
        })
    }

    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
        let deadline = self.deadline()?;
        let request = Request::SendText {
            pane: pane.clone(),
            text: text.to_owned(),
        };
        expect_ok(&self.call(&request, deadline)?)
    }

    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError> {
        let deadline = self.deadline()?;
        let request = Request::SendKeys {
            pane: pane.clone(),
            keys: keys.to_vec(),
        };
        expect_ok(&self.call(&request, deadline)?)
    }

    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError> {
        let deadline = self.deadline()?;
        // What Herdr does with `lines: 0` is unverified, so it is asked for one line at
        // least, and the reply is cut to `max_lines` all the same.
        let request = Request::Read {
            pane: pane.clone(),
            lines: u32::try_from(max_lines.max(1)).unwrap_or(u32::MAX),
        };
        parse_read(&self.call(&request, deadline)?, max_lines)
    }

    fn close(&self, pane: &PaneId) -> Result<(), PaneError> {
        let deadline = self.deadline()?;
        let request = Request::Close { pane: pane.clone() };
        expect_ok(&self.call(&request, deadline)?)
    }

    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        let deadline = self.deadline()?;
        let state = parse_snapshot(&self.call(&Request::SessionSnapshot, deadline)?)?;
        let mut panes = Vec::new();
        for workspace in &state.workspaces {
            // A workspace with no tab has no grid, so no pane of it has a cell.
            let Some(tab) = &workspace.grid_tab else {
                continue;
            };
            let cells = self.grid(tab, deadline)?.cells();
            panes.extend(cells.into_iter().map(|(grid, pane_id)| HerdrPane {
                session: self.config.session.clone(),
                workspace: workspace.label.clone(),
                pane_id,
                grid,
            }));
        }
        Ok(HerdrSnapshot { panes })
    }

    fn version(&self) -> Result<String, PaneError> {
        let version = self.supported_server(self.deadline()?)?.version;
        if version.is_empty() || version.contains(char::is_control) {
            return Err(PaneError::Unavailable {
                what: "Herdr reports a version that is empty or holds a control character"
                    .to_owned(),
            });
        }
        Ok(version)
    }
}

/// A configured workspace as Herdr has it: its grid tab, `None` when Herdr has no
/// workspace of the label, and the grid read off that tab's tree, empty when there is
/// none.
struct WorkspaceGrid {
    tab: Option<String>,
    map: GridMap,
}

/// The grid tab of `workspace`. A workspace with no tab has no grid: `unavailable`.
fn grid_tab(workspace: &WorkspaceRef) -> Result<String, PaneError> {
    workspace
        .grid_tab
        .clone()
        .ok_or_else(|| PaneError::Unavailable {
            what: format!(
                "Herdr workspace {:?} has no tab, so it has no grid",
                workspace.label
            ),
        })
}

/// What an error from the split of `target` means: `pane-not-found` is about the
/// pane split, not the one asked for, so it is `unavailable` (the layout changed while
/// the pane was placed). Any other error passes unchanged.
fn changed_under(error: PaneError, target: &PaneId, spec: &HerdrSpec) -> PaneError {
    match error {
        PaneError::PaneNotFound { .. } => PaneError::Unavailable {
            what: format!(
                "Herdr no longer has the pane {:?} that {} is split from: workspace {:?} \
                 changed while the pane was placed",
                target.as_str(),
                spec.grid,
                spec.workspace
            ),
        },
        other => other,
    }
}

/// Refuse with `usage`, before any request, a config the adapter cannot serve. The
/// checks run in this order, and the first that fails is the one answer: `session`,
/// `socket`, `timeout`, then each workspace in label order, `rows` before `cols`.
fn validate(config: &HerdrConfig) -> Result<(), PaneError> {
    if config.session.is_empty() {
        return Err(usage(
            "the Herdr config's session is empty; it names the one Herdr session served".to_owned(),
        ));
    }
    if !config.socket.is_absolute() {
        return Err(usage(format!(
            "the Herdr config's socket {:?} is relative; it must be an absolute path",
            config.socket
        )));
    }
    if config.timeout.is_zero() {
        return Err(usage(
            "the Herdr config's timeout is zero; it must be more than zero".to_owned(),
        ));
    }
    deadline_after(config.timeout)?;
    for (label, extent) in &config.workspaces {
        let field = match (extent.rows, extent.cols) {
            (0, _) => "rows",
            (_, 0) => "cols",
            _ => continue,
        };
        return Err(usage(format!(
            "the Herdr config's workspace {label:?} has 0 {field}; a workspace has at least one \
             row and one column"
        )));
    }
    Ok(())
}

/// The deadline of a call that starts now: `timeout` from now. A timeout too long to
/// add to the clock is the config's `usage`, never a panic.
fn deadline_after(timeout: Duration) -> Result<Instant, PaneError> {
    Instant::now().checked_add(timeout).ok_or_else(|| {
        usage(format!(
            "the Herdr config's timeout of {timeout:?} is too long to set a deadline with"
        ))
    })
}

/// `usage`: the config is malformed; `message` names the field.
fn usage(message: String) -> PaneError {
    PaneError::Usage { message }
}
