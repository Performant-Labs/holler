//! A simulated Herdr at the JSON level (#640 part 2, Decision 15): it reads request
//! lines with `serde_json` and answers them the way the Herdr spike
//! (`docs/research/herdr-api-spike.md`) saw Herdr answer, keeping its own split trees.
//! It is written from the spike, never from `src/`: it reads no tree with the adapter's
//! code and decodes no reply with it, so a bug in the adapter cannot hide in its oracle
//! (AC 35). Each test file that needs it declares `mod wire_herdr;`, and uses a part of
//! it, so the rest would trip `dead_code`.
//!
//! What the spike did not verify is marked INFERRED: closing a tab's last pane, the
//! `tab_not_found` code, the ratio range, and `pane_not_found` for `send_text`,
//! `send_keys` and `read` of a pane the fake does not hold (conformance case 9 needs it).
#![allow(dead_code)] // #640

pub mod serve;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use holler_adapter_herdr::layout::Direction;
use holler_adapter_herdr::protocol::Request;
use holler_adapter_herdr::transport::Transport;
use holler_pane::{PaneError, PaneId};
use holler_pane_testkit::herdr::{PROTOCOL_22_VERSION, UNSUPPORTED_VERSION};
use serde_json::{json, Map, Value};

/// The protocol a fake starts at, the one build the spike tested.
const PROTOCOL_22: u32 = 22;

/// Methods the adapter must never call (spike section 12): recorded, then refused.
const FORBIDDEN: [&str; 5] = [
    "server.stop",
    "server.live_handoff",
    "layout.apply",
    "pane.move",
    "pane.swap",
];

/// The `source`s `pane.read` accepts.
const SOURCES: [&str; 4] = ["visible", "recent", "recent_unwrapped", "detection"];

/// A Herdr error: its code and its message.
type Fault = (&'static str, String);

/// One Herdr server, at the JSON level. Share it with `Arc`.
pub struct WireHerdr {
    state: Mutex<State>,
}

/// Everything the fake holds.
struct State {
    protocol: Option<u32>,
    /// In Herdr's order.
    workspaces: Vec<Workspace>,
    /// The last workspace number handed out; never reused.
    last_workspace: u64,
    /// Each live pane's screen, by pane id.
    screens: BTreeMap<String, String>,
    /// Every request line received, parsed (a line that is not JSON as a string).
    requests: Vec<Value>,
}

struct Workspace {
    id: String,
    label: String,
    tabs: Vec<Tab>,
    last_tab: u64,
    /// The last pane number handed out in this workspace; never reused.
    last_pane: u64,
}

struct Tab {
    id: String,
    number: u64,
    root: Node,
}

/// A node of a tab's split tree.
#[derive(Clone)]
enum Node {
    Pane(String),
    Split {
        direction: &'static str,
        ratio: f64,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Default for WireHerdr {
    fn default() -> Self {
        Self::new()
    }
}

impl WireHerdr {
    /// A Herdr at protocol 22 with no workspace.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                protocol: Some(PROTOCOL_22),
                workspaces: Vec::new(),
                last_workspace: 0,
                screens: BTreeMap::new(),
                requests: Vec::new(),
            }),
        }
    }

    /// What every later `ping` reports: `Some(22)` (the default), `Some(99)`, or `None`
    /// for a `pong` without `protocol`.
    pub fn set_protocol(&self, protocol: Option<u32>) {
        self.lock().protocol = protocol;
    }

    /// A workspace labelled `label`, made as `workspace.create` makes one; its root pane.
    pub fn create_workspace(&self, label: &str) -> PaneId {
        let mut state = self.lock();
        let (_, root) = state.create_workspace(label);
        PaneId::new(root)
    }

    /// A second tab (number 2, then 3, ...) in the first workspace labelled `label`,
    /// with its own root pane, which it returns.
    pub fn add_tab(&self, label: &str) -> PaneId {
        let mut state = self.lock();
        let w = state
            .workspaces
            .iter()
            .position(|workspace| workspace.label == label)
            .expect("add_tab: no workspace of that label");
        PaneId::new(state.add_tab(w))
    }

    /// The outside world splits `target` (ratio 0.5); the new pane.
    pub fn split(&self, target: &PaneId, direction: Direction) -> PaneId {
        let direction = match direction {
            Direction::Right => "right",
            Direction::Down => "down",
        };
        let mut state = self.lock();
        let new = state
            .split(target.as_str(), direction, 0.5)
            .expect("split: no pane of that id");
        PaneId::new(new)
    }

    /// The grid tab's root (the tab with the lowest `number`) of the first workspace
    /// labelled `label`, as `layout.export` gives it.
    pub fn tree(&self, label: &str) -> Option<Value> {
        let state = self.lock();
        let workspace = state.workspaces.iter().find(|w| w.label == label)?;
        let tab = workspace.tabs.iter().min_by_key(|tab| tab.number)?;
        Some(tab.root.to_json())
    }

    /// Every request line received, parsed, oldest first.
    pub fn requests(&self) -> Vec<Value> {
        self.lock().requests.clone()
    }

    /// The `method` of each request received, oldest first (`""` for one without).
    pub fn methods(&self) -> Vec<String> {
        self.requests()
            .iter()
            .map(|request| {
                request
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            })
            .collect()
    }

    /// One request line in, one reply line out, with no newline.
    pub fn answer(&self, line: &str) -> String {
        let mut state = self.lock();
        let Ok(Value::Object(request)) = serde_json::from_str::<Value>(line.trim_end()) else {
            state.requests.push(Value::String(line.to_owned()));
            let error = json!({"code": "invalid_request", "message": "invalid request"});
            return json!({"id": "", "error": error}).to_string();
        };
        state.requests.push(Value::Object(request.clone()));
        let id = request.get("id").cloned().unwrap_or_else(|| json!(""));
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let no_params = Map::new();
        let params = request
            .get("params")
            .and_then(Value::as_object)
            .unwrap_or(&no_params);
        let reply = match state.handle(method, params) {
            Ok(result) => json!({"id": id, "result": result}),
            Err((code, message)) => json!({"id": id, "error": {"code": code, "message": message}}),
        };
        reply.to_string()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// In process: the line the adapter built goes to [`WireHerdr::answer`]; the deadline
/// is unused.
impl Transport for WireHerdr {
    fn exchange(&self, request: &Request, _deadline: Instant) -> Result<String, PaneError> {
        Ok(self.answer(&request.to_line()))
    }
}

/// What a [`Tap`]'s hook does with one exchange.
pub enum Tapped {
    /// Hand this line (the request's, or a rewrite of it) to the fake, and return its
    /// answer.
    Forward(String),
    /// Return this reply line; the fake sees nothing.
    Reply(String),
    /// Fail the exchange with this error, as the transport would; the fake sees
    /// nothing (#640 part 3, AC 16-17).
    Fail(PaneError),
}

/// The hook of a [`Tap`]: the fake, the request's line and the exchange's deadline.
type Hook = dyn Fn(&WireHerdr, String, Instant) -> Tapped + Send + Sync;

/// A transport in front of a fake whose hook sees each exchange first: to rewrite a
/// request, act on the fake, record the deadline, garble the reply or fail the exchange
/// (part 2's AC 16, 17, 28 and 29 and part 3's AC 16-18 share it).
pub struct Tap {
    fake: Arc<WireHerdr>,
    hook: Box<Hook>,
}

impl Tap {
    pub fn new(
        fake: Arc<WireHerdr>,
        hook: impl Fn(&WireHerdr, String, Instant) -> Tapped + Send + Sync + 'static,
    ) -> Self {
        Self {
            fake,
            hook: Box::new(hook),
        }
    }
}

impl Transport for Tap {
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError> {
        match (self.hook)(&self.fake, request.to_line(), deadline) {
            Tapped::Forward(line) => Ok(self.fake.answer(&line)),
            Tapped::Reply(reply) => Ok(reply),
            Tapped::Fail(error) => Err(error),
        }
    }
}

// --- Herdr's behaviour ---

impl State {
    /// The result of `method`, or Herdr's error.
    fn handle(&mut self, method: &str, params: &Map<String, Value>) -> Result<Value, Fault> {
        match method {
            "ping" => Ok(self.pong()),
            "session.snapshot" => Ok(self.snapshot()),
            "layout.export" => self.layout_export(params),
            "workspace.create" => Ok(self.workspace_create(params)),
            "pane.split" => self.pane_split(params),
            "pane.send_text" => self.send_text(params),
            "pane.send_keys" => self.send_keys(params),
            "pane.read" => self.read(params),
            "pane.close" => self.close(params),
            other if is_forbidden(other) => Err(invalid(format!(
                "invalid request: the simulated Herdr refuses `{other}`"
            ))),
            other => Err(invalid(format!(
                "invalid request: unknown variant `{other}`"
            ))),
        }
    }

    fn version(&self) -> &'static str {
        if self.protocol == Some(PROTOCOL_22) {
            PROTOCOL_22_VERSION
        } else {
            UNSUPPORTED_VERSION
        }
    }

    fn pong(&self) -> Value {
        let mut pong = json!({"type": "pong", "version": self.version(), "capabilities": {}});
        if let Some(protocol) = self.protocol {
            pong["protocol"] = json!(protocol);
        }
        pong
    }

    fn snapshot(&self) -> Value {
        let (mut workspaces, mut tabs, mut panes) = (Vec::new(), Vec::new(), Vec::new());
        for (i, workspace) in self.workspaces.iter().enumerate() {
            workspaces.push(workspace_json(workspace, i + 1));
            for tab in &workspace.tabs {
                tabs.push(tab_json(workspace, tab));
                for pane in tab.root.panes() {
                    panes.push(pane_json(workspace, tab, &pane));
                }
            }
        }
        json!({"type": "session_snapshot", "snapshot": {
            "version": self.version(), "protocol": self.protocol,
            "workspaces": workspaces, "tabs": tabs, "panes": panes,
            "layouts": [], "agents": []
        }})
    }

    fn layout_export(&self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let tab_id = field(params, "tab_id")?;
        // INFERRED: the code for an unknown tab (the adapter maps any code but
        // `pane_not_found` to `unavailable`).
        let (workspace, tab) = self
            .workspaces
            .iter()
            .find_map(|w| Some((w, w.tabs.iter().find(|tab| tab.id == tab_id)?)))
            .ok_or_else(|| ("tab_not_found", format!("tab {tab_id} not found")))?;
        Ok(json!({"type": "layout_export", "layout": {
            "workspace_id": workspace.id, "tab_id": tab.id, "zoomed": false,
            "focused_pane_id": tab.root.first_pane(), "root": tab.root.to_json()
        }}))
    }

    fn workspace_create(&mut self, params: &Map<String, Value>) -> Value {
        let label = params.get("label").and_then(Value::as_str).unwrap_or("");
        let (w, root) = self.create_workspace(label);
        let workspace = &self.workspaces[w];
        let tab = &workspace.tabs[0];
        json!({"type": "workspace_created",
            "workspace": workspace_json(workspace, w + 1),
            "tab": tab_json(workspace, tab),
            "root_pane": pane_json(workspace, tab, &root)})
    }

    fn pane_split(&mut self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let target = field(params, "target_pane_id")?;
        let direction = match field(params, "direction")? {
            "right" => "right",
            "down" => "down",
            other => {
                return Err(invalid(format!(
                    "invalid request: unknown variant `{other}`, expected `right` or `down`"
                )))
            }
        };
        // INFERRED: the range Herdr accepts.
        let ratio = match params.get("ratio") {
            None => 0.5,
            Some(ratio) => ratio
                .as_f64()
                .filter(|ratio| *ratio > 0.0 && *ratio < 1.0)
                .ok_or_else(|| invalid("invalid request: ratio is not in (0, 1)".to_owned()))?,
        };
        let new = self.split(target, direction, ratio)?;
        let (w, t) = self.locate(&new).ok_or_else(|| not_found(&new))?;
        let workspace = &self.workspaces[w];
        Ok(json!({"type": "pane_info", "pane": pane_json(workspace, &workspace.tabs[t], &new)}))
    }

    fn send_text(&mut self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let pane = field(params, "pane_id")?;
        let text = field(params, "text")?.to_owned();
        self.screen(pane)?.push_str(&text);
        Ok(ok())
    }

    fn send_keys(&mut self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let pane = field(params, "pane_id")?;
        let keys: Vec<String> = params
            .get("keys")
            .and_then(Value::as_array)
            .and_then(|keys| keys.iter().map(|k| k.as_str().map(str::to_owned)).collect())
            .ok_or_else(|| invalid("invalid request: keys is not a list of names".to_owned()))?;
        let screen = self.screen(pane)?;
        for key in keys {
            if key == "enter" {
                screen.push('\n');
            }
        }
        Ok(ok())
    }

    fn read(&mut self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let pane = field(params, "pane_id")?.to_owned();
        let source = field(params, "source")?.to_owned();
        if !SOURCES.contains(&source.as_str()) {
            return Err(invalid(format!(
                "invalid request: unknown variant `{source}`"
            )));
        }
        let lines = params.get("lines").and_then(Value::as_u64);
        let format = params
            .get("format")
            .and_then(Value::as_str)
            .unwrap_or("text");
        let screen = self.screen(&pane)?.clone();
        let (w, t) = self.locate(&pane).ok_or_else(|| not_found(&pane))?;
        let workspace = &self.workspaces[w];
        Ok(json!({"type": "pane_read", "read": {
            "pane_id": pane, "workspace_id": workspace.id, "tab_id": workspace.tabs[t].id,
            "source": source, "format": format, "text": last_lines(&screen, lines),
            "revision": screen.len(), "truncated": false
        }}))
    }

    fn close(&mut self, params: &Map<String, Value>) -> Result<Value, Fault> {
        let pane = field(params, "pane_id")?.to_owned();
        let (w, t) = self.locate(&pane).ok_or_else(|| not_found(&pane))?;
        let tabs = &mut self.workspaces[w].tabs;
        // The sibling takes the space (spike section 7). INFERRED: a tab's last pane
        // closes the tab, and a workspace's last tab closes the workspace.
        match tabs[t].root.clone().without(&pane) {
            Some(root) => tabs[t].root = root,
            None => {
                tabs.remove(t);
            }
        }
        if self.workspaces[w].tabs.is_empty() {
            self.workspaces.remove(w);
        }
        self.screens.remove(&pane);
        Ok(ok())
    }

    // --- the model under the methods ---

    /// A new workspace with one tab and its root pane: its index and the root pane.
    fn create_workspace(&mut self, label: &str) -> (usize, String) {
        self.last_workspace += 1;
        self.workspaces.push(Workspace {
            id: format!("w{}", self.last_workspace),
            label: label.to_owned(),
            tabs: Vec::new(),
            last_tab: 0,
            last_pane: 0,
        });
        let w = self.workspaces.len() - 1;
        let root = self.add_tab(w);
        (w, root)
    }

    /// A new tab in workspace `w`, numbered after its last, with a root pane.
    fn add_tab(&mut self, w: usize) -> String {
        let root = self.new_pane(w);
        let workspace = &mut self.workspaces[w];
        workspace.last_tab += 1;
        workspace.tabs.push(Tab {
            id: format!("{}:t{}", workspace.id, workspace.last_tab),
            number: workspace.last_tab,
            root: Node::Pane(root.clone()),
        });
        root
    }

    /// A new pane id in workspace `w`: `w<N>:p<M>`, `M` in upper-case base 36 (spike
    /// section 5). It has an empty screen.
    fn new_pane(&mut self, w: usize) -> String {
        let workspace = &mut self.workspaces[w];
        workspace.last_pane += 1;
        let id = format!("{}:p{}", workspace.id, base36(workspace.last_pane));
        self.screens.insert(id.clone(), String::new());
        id
    }

    /// Split `target`'s leaf: only the target's cell changes, and the new pane is the
    /// `second` child (spike section 7).
    fn split(
        &mut self,
        target: &str,
        direction: &'static str,
        ratio: f64,
    ) -> Result<String, Fault> {
        let (w, t) = self.locate(target).ok_or_else(|| not_found(target))?;
        let new = self.new_pane(w);
        self.workspaces[w].tabs[t]
            .root
            .split_leaf(target, direction, ratio, &new);
        Ok(new)
    }

    /// The workspace and tab indices of `pane`.
    fn locate(&self, pane: &str) -> Option<(usize, usize)> {
        self.workspaces
            .iter()
            .enumerate()
            .find_map(|(w, workspace)| {
                let t = workspace.tabs.iter().position(|tab| tab.root.holds(pane))?;
                Some((w, t))
            })
    }

    /// The screen of a live `pane`; `pane_not_found` for any other id.
    fn screen(&mut self, pane: &str) -> Result<&mut String, Fault> {
        self.screens.get_mut(pane).ok_or_else(|| not_found(pane))
    }
}

impl Node {
    fn to_json(&self) -> Value {
        match self {
            Node::Pane(id) => json!({"type": "pane", "pane_id": id}),
            Node::Split {
                direction,
                ratio,
                first,
                second,
            } => json!({"type": "split", "direction": direction, "ratio": ratio,
                "first": first.to_json(), "second": second.to_json()}),
        }
    }

    /// Every pane under this node, `first` before `second`.
    fn panes(&self) -> Vec<String> {
        match self {
            Node::Pane(id) => vec![id.clone()],
            Node::Split { first, second, .. } => {
                let mut panes = first.panes();
                panes.extend(second.panes());
                panes
            }
        }
    }

    fn first_pane(&self) -> String {
        match self {
            Node::Pane(id) => id.clone(),
            Node::Split { first, .. } => first.first_pane(),
        }
    }

    fn holds(&self, pane: &str) -> bool {
        self.panes().iter().any(|id| id == pane)
    }

    /// Replace the leaf `target` with a split of it and `new`.
    fn split_leaf(&mut self, target: &str, direction: &'static str, ratio: f64, new: &str) {
        match self {
            Node::Pane(id) if id == target => {
                *self = Node::Split {
                    direction,
                    ratio,
                    first: Box::new(Node::Pane(target.to_owned())),
                    second: Box::new(Node::Pane(new.to_owned())),
                };
            }
            Node::Pane(_) => {}
            Node::Split { first, second, .. } => {
                first.split_leaf(target, direction, ratio, new);
                second.split_leaf(target, direction, ratio, new);
            }
        }
    }

    /// This tree without the leaf `pane`, its parent split replaced by the sibling;
    /// `None` when the leaf was the whole tree.
    fn without(self, pane: &str) -> Option<Node> {
        match self {
            Node::Pane(id) if id == pane => None,
            Node::Pane(id) => Some(Node::Pane(id)),
            Node::Split {
                direction,
                ratio,
                first,
                second,
            } => match (first.without(pane), second.without(pane)) {
                (Some(first), Some(second)) => Some(Node::Split {
                    direction,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            },
        }
    }
}

fn workspace_json(workspace: &Workspace, number: usize) -> Value {
    let pane_count: usize = workspace
        .tabs
        .iter()
        .map(|tab| tab.root.panes().len())
        .sum();
    json!({"workspace_id": workspace.id, "number": number, "label": workspace.label,
        "focused": false, "pane_count": pane_count, "tab_count": workspace.tabs.len(),
        "active_tab_id": workspace.tabs.first().map(|tab| tab.id.clone()),
        "agent_status": "idle"})
}

fn tab_json(workspace: &Workspace, tab: &Tab) -> Value {
    json!({"tab_id": tab.id, "workspace_id": workspace.id, "number": tab.number,
        "label": tab.number.to_string(), "focused": false,
        "pane_count": tab.root.panes().len(), "agent_status": "idle"})
}

fn pane_json(workspace: &Workspace, tab: &Tab, pane: &str) -> Value {
    json!({"pane_id": pane, "workspace_id": workspace.id, "tab_id": tab.id,
        "focused": false, "cwd": "/scratch", "agent_status": "idle", "revision": 0})
}

fn ok() -> Value {
    json!({"type": "ok"})
}

/// The string field `key` of `params`; `invalid_request` when it is missing.
fn field<'a>(params: &'a Map<String, Value>, key: &str) -> Result<&'a str, Fault> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("invalid request: missing field `{key}`")))
}

fn invalid(message: String) -> Fault {
    ("invalid_request", message)
}

/// Herdr's answer about a pane it does not hold (spike section 4, "Close").
fn not_found(pane: &str) -> Fault {
    ("pane_not_found", format!("pane {pane} not found"))
}

fn is_forbidden(method: &str) -> bool {
    FORBIDDEN.contains(&method)
        || method.starts_with("integration.")
        || method.starts_with("plugin.")
}

/// The last `lines` lines of `screen` (all of them for `None`), joined with `\n`.
fn last_lines(screen: &str, lines: Option<u64>) -> String {
    let all: Vec<&str> = screen.lines().collect();
    let keep = lines.map_or(all.len(), |n| usize::try_from(n).unwrap_or(usize::MAX));
    all[all.len().saturating_sub(keep)..].join("\n")
}

/// `n` in base 36, upper case: `1` ... `9`, `A` ... `Z`, `10` (spike section 5).
fn base36(mut n: u64) -> String {
    let mut digits = Vec::new();
    loop {
        let digit = char::from_digit((n % 36) as u32, 36).expect("a base-36 digit");
        digits.push(digit.to_ascii_uppercase());
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.into_iter().rev().collect()
}
