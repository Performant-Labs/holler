//! Herdr's wire, with no I/O (the Herdr spike, `docs/research/herdr-api-spike.md`,
//! sections 3, 4, 12 and 13; protocol 22, schema version 1): the request lines, the
//! reply decoder, the parsers of the results the adapter reads, and the version gate.
//!
//! - **Requests.** One JSON object per line: `{"id":"holler:<method>","method":...,
//!   "params":{...}}`. Only the methods of [`ALLOWED_METHODS`] can be built, so never
//!   `server.stop`, `server.live_handoff`, `layout.apply`, `pane.move`, `pane.swap`, or a
//!   `plugin.*` or `integration.*` method (spike section 12). A new pane or workspace is
//!   made with `focus: false`, so the operator's focus never moves. Keys go out by
//!   Herdr's own names, verbatim (`enter`, `ctrl+c`): nothing is case-folded or
//!   translated.
//! - **Replies.** [`decode_reply`] gives the `result` object of the reply to a request.
//!   Herdr's `pane_not_found` is `pane-not-found`, naming the request's pane. Any other
//!   Herdr error, and a reply that is not a JSON object, carries another request's id or
//!   is neither a result object nor an error, is `unavailable` (ADR-0021 section 9: "also
//!   a garbled reply").
//! - **Parsers.** Each accepts one result `type`, and refuses any other, or a field it
//!   reads that is missing or of the wrong type, as `unavailable`. They ignore every
//!   field they do not read, so a field Herdr adds changes nothing.
//! - **Messages** are one line. They name the method and Herdr's code, quote what Herdr
//!   sent (cut to 64 characters), and never echo the text typed into a pane, Herdr's own
//!   error message (which can quote it) or a pane's screen.
//! - **The version gate** ([`check_supported`]). `ping`'s integer `protocol` decides
//!   support: exactly [`SUPPORTED_PROTOCOLS`]. Any other, or none, is
//!   `herdr-version-unsupported`, naming the reported version and [`SUPPORTED_VERSIONS`].
//!   The version string is what `HerdrPort::version` reports.
//! - **The grid tab** ([`SessionState::workspace`]). A workspace is found by its `label`,
//!   which must be unique, and its grid is the tree of its tab with the lowest `number`.
//!   A tab's `number` is a display ordinal, not an id, so reordering a workspace's tabs
//!   (`tab.move`) changes which tab that is.

use std::fmt;

use holler_pane::{Key, PaneError, PaneId};
use serde_json::{json, Map, Value};

use crate::layout::{Direction, LayoutNode};

/// The protocol versions the adapter supports.
pub const SUPPORTED_PROTOCOLS: [u32; 1] = [22];
/// What a refusal names as supported.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";

/// The number of methods the adapter may call.
const METHOD_COUNT: usize = 9;

/// Every method the adapter may call, and no other. It is built from the same table as
/// [`Request::method`], so no request can name a method that is not on it.
pub const ALLOWED_METHODS: [&str; METHOD_COUNT] = {
    let mut names = [""; METHOD_COUNT];
    let mut i = 0;
    while i < METHOD_COUNT {
        names[i] = Method::ALL[i].as_str();
        i += 1;
    }
    names
};

/// Herdr's error code for a pane it does not have (spike section 4).
const PANE_NOT_FOUND: &str = "pane_not_found";

/// How many characters of a string Herdr sent a message quotes.
const EXCERPT_LIMIT: usize = 64;

/// A method the adapter may call: the one table [`ALLOWED_METHODS`] and
/// [`Request::method`] are both built from.
#[derive(Debug, Clone, Copy)]
enum Method {
    Ping,
    SessionSnapshot,
    LayoutExport,
    WorkspaceCreate,
    PaneSplit,
    PaneSendText,
    PaneSendKeys,
    PaneRead,
    PaneClose,
}

impl Method {
    /// Every method, in the order of [`ALLOWED_METHODS`].
    const ALL: [Method; METHOD_COUNT] = [
        Method::Ping,
        Method::SessionSnapshot,
        Method::LayoutExport,
        Method::WorkspaceCreate,
        Method::PaneSplit,
        Method::PaneSendText,
        Method::PaneSendKeys,
        Method::PaneRead,
        Method::PaneClose,
    ];

    /// Herdr's name for the method.
    const fn as_str(self) -> &'static str {
        match self {
            Method::Ping => "ping",
            Method::SessionSnapshot => "session.snapshot",
            Method::LayoutExport => "layout.export",
            Method::WorkspaceCreate => "workspace.create",
            Method::PaneSplit => "pane.split",
            Method::PaneSendText => "pane.send_text",
            Method::PaneSendKeys => "pane.send_keys",
            Method::PaneRead => "pane.read",
            Method::PaneClose => "pane.close",
        }
    }
}

/// One request to Herdr. Its `Debug` form never shows the text of a `SendText`.
#[derive(Clone)]
pub enum Request {
    /// `ping`: the server's version and protocol.
    Ping,
    /// `session.snapshot`: the workspaces, tabs and panes.
    SessionSnapshot,
    /// `layout.export` of one tab: its split tree.
    LayoutExport { tab_id: String },
    /// `workspace.create`, unfocused: a workspace with one root pane.
    WorkspaceCreate { label: String },
    /// `pane.split` of `target`, unfocused: the new pane is the split's `second` child.
    Split {
        target: PaneId,
        direction: Direction,
        ratio: f64,
    },
    /// `pane.send_text`: literal text, with no Enter.
    SendText { pane: PaneId, text: String },
    /// `pane.send_keys`, by Herdr's own key names.
    SendKeys { pane: PaneId, keys: Vec<Key> },
    /// `pane.read` of the `recent` output, as plain text.
    Read { pane: PaneId, lines: u32 },
    /// `pane.close`.
    Close { pane: PaneId },
}

impl Request {
    /// Herdr's method name.
    pub fn method(&self) -> &'static str {
        self.kind().as_str()
    }

    /// The request id: `holler:<method>`.
    pub fn id(&self) -> String {
        format!("holler:{}", self.method())
    }

    /// One JSON object and exactly one trailing newline.
    pub fn to_line(&self) -> String {
        let request = json!({"id": self.id(), "method": self.method(), "params": self.params()});
        format!("{request}\n")
    }

    /// The method, from the one table.
    fn kind(&self) -> Method {
        match self {
            Request::Ping => Method::Ping,
            Request::SessionSnapshot => Method::SessionSnapshot,
            Request::LayoutExport { .. } => Method::LayoutExport,
            Request::WorkspaceCreate { .. } => Method::WorkspaceCreate,
            Request::Split { .. } => Method::PaneSplit,
            Request::SendText { .. } => Method::PaneSendText,
            Request::SendKeys { .. } => Method::PaneSendKeys,
            Request::Read { .. } => Method::PaneRead,
            Request::Close { .. } => Method::PaneClose,
        }
    }

    /// The `params` object, with the field names of Herdr's schema.
    fn params(&self) -> Value {
        match self {
            Request::Ping | Request::SessionSnapshot => json!({}),
            Request::LayoutExport { tab_id } => json!({"tab_id": tab_id}),
            Request::WorkspaceCreate { label } => json!({"label": label, "focus": false}),
            Request::Split {
                target,
                direction,
                ratio,
            } => json!({
                "target_pane_id": target.as_str(),
                "direction": direction.as_str(),
                "ratio": ratio,
                "focus": false
            }),
            Request::SendText { pane, text } => json!({"pane_id": pane.as_str(), "text": text}),
            Request::SendKeys { pane, keys } => json!({
                "pane_id": pane.as_str(),
                "keys": keys.iter().map(Key::as_str).collect::<Vec<_>>()
            }),
            Request::Read { pane, lines } => json!({
                "pane_id": pane.as_str(),
                "source": "recent",
                "lines": lines,
                "format": "text"
            }),
            Request::Close { pane } => json!({"pane_id": pane.as_str()}),
        }
    }

    /// The pane the request names, which Herdr's `pane_not_found` is about; `None` for a
    /// request that names no pane.
    fn pane(&self) -> Option<&PaneId> {
        match self {
            Request::Split { target: pane, .. }
            | Request::SendText { pane, .. }
            | Request::SendKeys { pane, .. }
            | Request::Read { pane, .. }
            | Request::Close { pane } => Some(pane),
            Request::Ping
            | Request::SessionSnapshot
            | Request::LayoutExport { .. }
            | Request::WorkspaceCreate { .. } => None,
        }
    }
}

impl fmt::Debug for Request {
    /// Every field, except that a `SendText` shows its text's length, so that typed text
    /// never reaches a log line or an assertion message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Request::Ping => f.write_str("Ping"),
            Request::SessionSnapshot => f.write_str("SessionSnapshot"),
            Request::LayoutExport { tab_id } => f
                .debug_struct("LayoutExport")
                .field("tab_id", tab_id)
                .finish(),
            Request::WorkspaceCreate { label } => f
                .debug_struct("WorkspaceCreate")
                .field("label", label)
                .finish(),
            Request::Split {
                target,
                direction,
                ratio,
            } => f
                .debug_struct("Split")
                .field("target", target)
                .field("direction", direction)
                .field("ratio", ratio)
                .finish(),
            Request::SendText { pane, text } => f
                .debug_struct("SendText")
                .field("pane", pane)
                .field("text", &format_args!("<{} bytes>", text.len()))
                .finish(),
            Request::SendKeys { pane, keys } => f
                .debug_struct("SendKeys")
                .field("pane", pane)
                .field("keys", keys)
                .finish(),
            Request::Read { pane, lines } => f
                .debug_struct("Read")
                .field("pane", pane)
                .field("lines", lines)
                .finish(),
            Request::Close { pane } => f.debug_struct("Close").field("pane", pane).finish(),
        }
    }
}

/// The `result` object of a reply to `request`, or the error it maps to.
pub fn decode_reply(request: &Request, line: &str) -> Result<Value, PaneError> {
    let method = request.method();
    let garbled = |what: &str| unavailable(format!("Herdr's reply to {method} {what}"));
    let Ok(Value::Object(mut reply)) = serde_json::from_str::<Value>(line) else {
        return Err(garbled("is not a JSON object"));
    };
    if reply.get("id").and_then(Value::as_str) != Some(request.id().as_str()) {
        return Err(garbled("does not carry that request's id"));
    }
    match (reply.remove("result"), reply.remove("error")) {
        (Some(result @ Value::Object(_)), None) => Ok(result),
        (None, Some(error)) => Err(herdr_error(request, &error)),
        _ => Err(garbled("is neither a result object nor an error")),
    }
}

/// What Herdr's error reply to `request` maps to: `pane_not_found` about the request's
/// pane is `pane-not-found`, and anything else is `unavailable`, naming the method and
/// Herdr's code but never Herdr's message.
fn herdr_error(request: &Request, error: &Value) -> PaneError {
    let method = request.method();
    match (error.get("code").and_then(Value::as_str), request.pane()) {
        (Some(PANE_NOT_FOUND), Some(pane)) => PaneError::PaneNotFound {
            what: pane.as_str().to_owned(),
        },
        (Some(code), _) => unavailable(format!(
            "Herdr answered {method} with the error {}",
            excerpt(code)
        )),
        (None, _) => unavailable(format!(
            "Herdr answered {method} with an error that has no code"
        )),
    }
}

/// What `ping` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerVersion {
    pub version: String,
    pub protocol: Option<u32>,
}

/// Read a `pong` result. A missing `protocol`, or one that is not a protocol number, is
/// `None`: unknown, which [`check_supported`] refuses, not a garbled reply.
pub fn parse_pong(result: &Value) -> Result<ServerVersion, PaneError> {
    let pong = result_of(result, "pong")?;
    Ok(ServerVersion {
        version: pong.string("version")?.to_owned(),
        protocol: pong
            .fields
            .get("protocol")
            .and_then(Value::as_u64)
            .and_then(|protocol| u32::try_from(protocol).ok()),
    })
}

/// Accept exactly the supported protocols; refuse the rest with `herdr-version-unsupported`.
pub fn check_supported(server: &ServerVersion) -> Result<(), PaneError> {
    let protocol = match server.protocol {
        Some(protocol) if SUPPORTED_PROTOCOLS.contains(&protocol) => return Ok(()),
        Some(protocol) => format!("protocol {protocol}"),
        None => "an unknown protocol".to_owned(),
    };
    Err(PaneError::HerdrVersionUnsupported {
        message: format!(
            "Herdr reports version {} with {protocol}; the supported one is {SUPPORTED_VERSIONS}",
            excerpt(&server.version)
        ),
    })
}

/// A Herdr workspace, by label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRef {
    pub workspace_id: String,
    pub label: String,
    /// The tab whose tree is the workspace's grid: the one with the lowest `number`.
    pub grid_tab: Option<String>,
}

/// A pane of a Herdr snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneRef {
    pub pane_id: PaneId,
    pub workspace_id: String,
    pub tab_id: String,
}

/// What `session.snapshot` reports, reduced to what the adapter reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub workspaces: Vec<WorkspaceRef>,
    pub panes: Vec<PaneRef>,
}

impl SessionState {
    /// The workspace labelled `label`; two with one label are `unavailable`.
    pub fn workspace(&self, label: &str) -> Result<Option<&WorkspaceRef>, PaneError> {
        let labelled: Vec<&WorkspaceRef> = self
            .workspaces
            .iter()
            .filter(|workspace| workspace.label == label)
            .collect();
        match labelled.as_slice() {
            [] => Ok(None),
            [one] => Ok(Some(*one)),
            many => {
                let ids: Vec<String> = many
                    .iter()
                    .map(|workspace| excerpt(&workspace.workspace_id))
                    .collect();
                Err(unavailable(format!(
                    "Herdr has {} workspaces labelled {} ({}), and a workspace is found by its \
                     label",
                    many.len(),
                    excerpt(label),
                    ids.join(", ")
                )))
            }
        }
    }
}

/// Read a `session_snapshot` result.
pub fn parse_snapshot(result: &Value) -> Result<SessionState, PaneError> {
    let snapshot = result_of(result, "session_snapshot")?.object("snapshot", "snapshot")?;
    let tabs = snapshot
        .list("tabs", "snapshot tab")?
        .into_iter()
        .map(tab_ref)
        .collect::<Result<Vec<_>, _>>()?;
    let workspaces = snapshot
        .list("workspaces", "snapshot workspace")?
        .into_iter()
        .map(|workspace| workspace_ref(workspace, &tabs))
        .collect::<Result<_, _>>()?;
    let panes = snapshot
        .list("panes", "snapshot pane")?
        .into_iter()
        .map(pane_ref)
        .collect::<Result<_, _>>()?;
    Ok(SessionState { workspaces, panes })
}

/// A tab of a snapshot, as far as finding a workspace's grid tab needs.
struct TabRef<'a> {
    tab_id: &'a str,
    workspace_id: &'a str,
    number: u64,
}

/// Read one tab of a snapshot.
fn tab_ref(tab: Object<'_>) -> Result<TabRef<'_>, PaneError> {
    Ok(TabRef {
        tab_id: tab.string("tab_id")?,
        workspace_id: tab.string("workspace_id")?,
        number: tab.get("number", Value::as_u64)?,
    })
}

/// Read one workspace of a snapshot, with its grid tab among `tabs`.
fn workspace_ref(workspace: Object<'_>, tabs: &[TabRef<'_>]) -> Result<WorkspaceRef, PaneError> {
    let workspace_id = workspace.string("workspace_id")?;
    let grid_tab = tabs
        .iter()
        .filter(|tab| tab.workspace_id == workspace_id)
        .min_by_key(|tab| tab.number)
        .map(|tab| tab.tab_id.to_owned());
    Ok(WorkspaceRef {
        workspace_id: workspace_id.to_owned(),
        label: workspace.string("label")?.to_owned(),
        grid_tab,
    })
}

/// Read one pane of a snapshot.
fn pane_ref(pane: Object<'_>) -> Result<PaneRef, PaneError> {
    Ok(PaneRef {
        pane_id: PaneId::new(pane.string("pane_id")?),
        workspace_id: pane.string("workspace_id")?.to_owned(),
        tab_id: pane.string("tab_id")?.to_owned(),
    })
}

/// Read a `layout_export` result into its split tree.
pub fn parse_layout_export(result: &Value) -> Result<LayoutNode, PaneError> {
    let layout = result_of(result, "layout_export")?.object("layout", "layout")?;
    layout_node(layout.object("root", "layout node")?)
}

/// Read one node of a layout tree, and the nodes under it.
fn layout_node(node: Object<'_>) -> Result<LayoutNode, PaneError> {
    match node.string("type")? {
        "pane" => Ok(LayoutNode::Pane {
            pane_id: PaneId::new(node.string("pane_id")?),
        }),
        "split" => Ok(LayoutNode::Split {
            direction: direction(node.string("direction")?)?,
            ratio: node.get("ratio", Value::as_f64)?,
            first: Box::new(layout_node(node.object("first", "layout node")?)?),
            second: Box::new(layout_node(node.object("second", "layout node")?)?),
        }),
        other => Err(unavailable(format!(
            "Herdr sent a layout node of type {}",
            excerpt(other)
        ))),
    }
}

/// The direction Herdr names `name`.
fn direction(name: &str) -> Result<Direction, PaneError> {
    [Direction::Right, Direction::Down]
        .into_iter()
        .find(|direction| direction.as_str() == name)
        .ok_or_else(|| {
            unavailable(format!(
                "Herdr sent a split direction {}, which is neither right nor down",
                excerpt(name)
            ))
        })
}

/// Read a `workspace_created` result: the workspace and its root pane.
pub fn parse_workspace_created(result: &Value) -> Result<(WorkspaceRef, PaneId), PaneError> {
    let created = result_of(result, "workspace_created")?;
    let workspace = created.object("workspace", "created workspace")?;
    let tab = created.object("tab", "created tab")?;
    let root = created.object("root_pane", "created root pane")?;
    let workspace = WorkspaceRef {
        workspace_id: workspace.string("workspace_id")?.to_owned(),
        label: workspace.string("label")?.to_owned(),
        grid_tab: Some(tab.string("tab_id")?.to_owned()),
    };
    Ok((workspace, PaneId::new(root.string("pane_id")?)))
}

/// Read a `pane_info` result (what `pane.split` returns).
pub fn parse_pane_info(result: &Value) -> Result<PaneId, PaneError> {
    let pane = result_of(result, "pane_info")?.object("pane", "pane")?;
    Ok(PaneId::new(pane.string("pane_id")?))
}

/// Read a `pane_read` result: its last `max_lines` lines.
pub fn parse_read(result: &Value, max_lines: usize) -> Result<String, PaneError> {
    let read = result_of(result, "pane_read")?.object("read", "pane read")?;
    Ok(last_lines(read.string("text")?, max_lines))
}

/// Accept only `type: "ok"`.
pub fn expect_ok(result: &Value) -> Result<(), PaneError> {
    result_of(result, "ok").map(|_| ())
}

/// An object Herdr sent, and what it is, for the message when a field is wrong.
#[derive(Clone, Copy)]
struct Object<'a> {
    fields: &'a Map<String, Value>,
    what: &'static str,
}

impl<'a> Object<'a> {
    /// `value` as an object; `unavailable` when it is not one.
    fn of(value: &'a Value, what: &'static str) -> Result<Self, PaneError> {
        value
            .as_object()
            .map(|fields| Self { fields, what })
            .ok_or_else(|| unavailable(format!("Herdr's {what} is not a JSON object")))
    }

    /// The field `key` as `read` takes it; `unavailable` when it is missing or of
    /// another type.
    fn get<T>(self, key: &str, read: fn(&'a Value) -> Option<T>) -> Result<T, PaneError> {
        self.fields
            .get(key)
            .and_then(read)
            .ok_or_else(|| unavailable(format!("Herdr's {} has no usable {key}", self.what)))
    }

    /// The string field `key`.
    fn string(self, key: &str) -> Result<&'a str, PaneError> {
        self.get(key, Value::as_str)
    }

    /// The object field `key`, which is a `what`.
    fn object(self, key: &str, what: &'static str) -> Result<Self, PaneError> {
        self.get(key, Value::as_object)
            .map(|fields| Self { fields, what })
    }

    /// The list field `key`, whose entries are each a `what`.
    fn list(self, key: &str, what: &'static str) -> Result<Vec<Self>, PaneError> {
        self.get(key, Value::as_array)?
            .iter()
            .map(|entry| Self::of(entry, what))
            .collect()
    }
}

/// `result` as an object whose `type` is `kind`; `unavailable` otherwise.
fn result_of<'a>(result: &'a Value, kind: &'static str) -> Result<Object<'a>, PaneError> {
    let object = Object::of(result, kind)?;
    match object.fields.get("type").and_then(Value::as_str) {
        Some(found) if found == kind => Ok(object),
        found => Err(unavailable(format!(
            "Herdr sent a result of type {} where {kind} was expected",
            found.map_or_else(|| "(none)".to_owned(), excerpt)
        ))),
    }
}

/// `unavailable`: Herdr's reply was garbled, or Herdr refused; `what` says which.
fn unavailable(what: String) -> PaneError {
    PaneError::Unavailable { what }
}

/// `text` that Herdr sent, quoted on one line and cut to [`EXCERPT_LIMIT`] characters,
/// so that a garbled reply can neither lengthen a message nor break it across lines.
pub(crate) fn excerpt(text: &str) -> String {
    let head: String = text.chars().take(EXCERPT_LIMIT).collect();
    if head.len() < text.len() {
        format!("{head:?}...")
    } else {
        format!("{head:?}")
    }
}

/// The last `max_lines` lines of `screen` (as `str::lines` splits it), joined with
/// `"\n"`, with no newline after the last: `""` for none. The test kit's `FakeHerdr`
/// reads a screen the same way; its copy is private and dev-only, so it is not shared.
fn last_lines(screen: &str, max_lines: usize) -> String {
    let skip = screen.lines().count().saturating_sub(max_lines);
    screen.lines().skip(skip).collect::<Vec<_>>().join("\n")
}
