# Brief: #640 herdr-adapter, part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr

Repo: Performant-Labs/holler. Issue: #640 (epic #633, wave 3), **part 2 of 3**. Rigor: second-opinion. UI surface: no.
Kind: feature.

**Branch:** `issue-640-implementation`. Its tree equals `origin/main` at `3bdd129` (part 1, PR #699, merged
2026-10-09 4:13 PM MDT).
**Review-rigor:** second-opinion (MO, 2026-10-09). This raises the issue's `rigor: in-session` line for this part,
because this part adds the crate's first I/O: a socket, deadlines and a worker thread.
**Design (Phase 3):** N/A (no UI surface).
**PR wording:** the PR says `Part of #640`, **not** `Closes #640`. Part 3 closes the issue.

## Scope

#640 runs as three sequential pipeline runs on the same issue (the split is in the part-1 brief, kept in git history
at `1571c6d:docs/handoffs/640-brief.md`):

| Part | Delivers | State |
|---|---|---|
| 1 | Pure core, no I/O: `protocol.rs`, `layout.rs`, `plan.rs` | **merged** (`3bdd129`) |
| **2 (this brief)** | I/O: `transport.rs` (one request per Unix-socket connection, one deadline per port call, `timeout`/`unavailable`), `adapter.rs` (`HerdrAdapter`: `connect` pings and refuses an unknown protocol, and it implements `HerdrPort` on part 1). Also a dev-test wire fake: a simulated Herdr at the JSON level that passes `run_herdr_conformance` in the default test run | this run |
| 3 | An opt-in scratch-Herdr test (`#[ignore]`, env-gated, which refuses the default session by name); doc-only edits to `holler-pane` (`ports.rs` `Key` and `ensure_pane` docs, the `error.rs` `GridOutOfRange` doc); the ADR-0021 §9/§10 rows. Closes #640 | later |

Issue acceptance covered by **this part**: "Passes the conformance suite from #638"; "Unit tests against a fake Herdr
that **rejects** a transposed or off-by-one position: placing panes at `r2c1` and `r1c2` lands each in the right cell,
and the snapshot reads back the same `GridPos`; a deliberately swapped conversion fails these tests"; "each supported
version works against the fake's two versions and an unknown one is refused with `herdr-version-unsupported`" (at the
adapter level; part 1 covered the parser level). Not this part: the scratch-session acceptance (part 3).

How this part reads "an unknown one is refused" (a clarification of Decision 8, which is unchanged): `connect` and
`connect_with` refuse an unknown protocol, so no `HerdrAdapter` ever exists for one, and `version()` checks again on
every call. The other methods run against the gate that passed at connect. The port line "An unknown version is
`herdr-version-unsupported`" is the doc comment of `HerdrPort::version` itself (`crates/holler-pane/src/ports.rs:146-147`),
not of the whole trait. Whether the issue text should be amended to say this is the operator's call ("For the
operator", item 4).

## Problem

`holler-adapter-herdr` can build Herdr requests, read Herdr's split tree and plan splits, but it cannot talk to Herdr,
and nothing implements `holler_pane::HerdrPort` except the test kit's in-memory `FakeHerdr`. The verb stories (#644 and
later) and the wiring (#649) need a real `HerdrPort`. It must:

- talk to Herdr's local socket within I5's bound;
- turn every socket fault into `timeout` or `unavailable`;
- refuse a Herdr protocol it does not know;
- place panes only by the splits that `plan_splits` decides, and confirm each placement by reading the tree back.

All of this has to be proven without ever touching a real Herdr server.

## Evidence (verbatim, as of `3bdd129`)

### The issue's acceptance (verbatim, `gh issue view 640`)
```
## Acceptance
- Passes the conformance suite from #638.
- (amended 2026-10-08, grid) Unit tests against a fake Herdr that **rejects** a transposed or off-by-one position: placing panes at `r2c1` and `r1c2` lands each in the right cell, and the snapshot reads back the same `GridPos`; a deliberately swapped conversion fails these tests.
- (amended 2026-10-08, features) `plan_splits` unit tests: an empty workspace to 2 rows by 4 columns, adding one cell to a partial layout, an unreachable position refused; passes #638's split-only-mode case; each supported version works against the fake's two versions and an unknown one is refused with `herdr-version-unsupported`.
- Against a scratch Herdr session (marked `#[ignore]`, opt-in): create, run, send, read, close and snapshot a pane; nothing touches a non-scratch session (the test refuses the default session by name).
...
## Blast radius
- crates/holler-adapter-herdr/**
```
The issue's scope says: "the adapter never caches a pane id past one call", "the adapter uses the local socket and
has no remote path", and "Timeouts on every call".

### The crate as merged (part 1)
```
crates/holler-adapter-herdr/Cargo.toml:12-29
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise).
[dependencies]
# The port this crate implements and every type it speaks (`PaneError`, `GridPos`,
# `PaneId`, `Key`, `RefusalCode`); for `layout`, `plan` and `protocol`. The adapter
# depends on nothing else of Holler (ADR-0021 section 5).
holler-pane = { path = "../holler-pane" }
# Builds and reads Herdr's JSON requests and replies (`Value`); for `protocol`.
serde_json = { workspace = true }

[dev-dependencies]
# `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS`, which the tests pin equal to this
# crate's own constants (#640 part 1, AC 13). A dev-dependency only (ADR-0021 section 5).
holler-pane-testkit = { path = "../holler-pane-testkit" }

# Workspace lints (issue #149).
[lints]
workspace = true
```
```
crates/holler-adapter-herdr/src/lib.rs:24-39
//! Part 1 of story #640 is this pure core, with no I/O:
//!
//! - [`layout`] — Herdr's split tree and the one conversion from it to a `GridPos`.
//! - [`plan`] — the right/down splits that reach a cell, or `grid-unreachable`.
//! - [`protocol`] — Herdr's wire: requests, reply decoding, the version gate.
//!
//! The socket transport and the `HerdrPort` implementation follow in part 2.
//!
//! Nothing is re-exported flat: `layout::Direction` and `protocol::SessionState` would
//! share a root name with `holler_proto`'s root re-exports, so a crate that uses both
//! would see two of each. Every item is reached by its module path, as in
//! `holler_pane_testkit`.

pub mod layout;
pub mod plan;
pub mod protocol;
```
Files today: `src/{lib.rs 39, layout.rs 222, plan.rs 268, protocol.rs 595}` lines, and
`tests/{layout_test.rs 207, plan_splits_test.rs 352, protocol_test.rs 679, common/mod.rs 86}` lines.

**`protocol.rs`: what the adapter calls.**
```
crates/holler-adapter-herdr/src/protocol.rs:39-42
/// The protocol versions the adapter supports.
pub const SUPPORTED_PROTOCOLS: [u32; 1] = [22];
/// What a refusal names as supported.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
```
```
crates/holler-adapter-herdr/src/protocol.rs:110-152
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:170-198
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:262-296
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:298-332
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:334-387
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:447-451
/// Read a `layout_export` result into its split tree.
pub fn parse_layout_export(result: &Value) -> Result<LayoutNode, PaneError> {
    let layout = result_of(result, "layout_export")?.object("layout", "layout")?;
    layout_node(layout.object("root", "layout node")?)
}
```
```
crates/holler-adapter-herdr/src/protocol.rs:485-514
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
```
```
crates/holler-adapter-herdr/src/protocol.rs:589-595
/// The last `max_lines` lines of `screen` (as `str::lines` splits it), joined with
/// `"\n"`, with no newline after the last: `""` for none. The test kit's `FakeHerdr`
/// reads a screen the same way; its copy is private and dev-only, so it is not shared.
fn last_lines(screen: &str, max_lines: usize) -> String {
    let skip = screen.lines().count().saturating_sub(max_lines);
    screen.lines().skip(skip).collect::<Vec<_>>().join("\n")
}
```
So `parse_read(result, 0)` is `""` whatever the reply's text (AC 25).
`ALLOWED_METHODS` (`protocol.rs:49-57`) is built from the same private `Method` table as `Request::method`
(`protocol.rs:82-107`): `ping`, `session.snapshot`, `layout.export`, `workspace.create`, `pane.split`,
`pane.send_text`, `pane.send_keys`, `pane.read` and `pane.close`. Part-1 tests `ac15_*` (`tests/protocol_test.rs:159`,
`:179`) pin it.

**`layout.rs`: the one conversion.**
```
crates/holler-adapter-herdr/src/layout.rs:28-58
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// A `right` split: the new pane is the `second` child, right of the split one.
    Right,
    /// A `down` split: the new pane is the `second` child, below the split one.
    Down,
}

impl Direction {
    /// Herdr's own name: `right` or `down`.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Right => "right",
            Direction::Down => "down",
        }
    }
}

/// A node of Herdr's `layout.export` tree, reduced to what a position depends on.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// A pane.
    Pane { pane_id: PaneId },
    /// A split of one cell: `first` keeps `ratio` of it, and `second` takes the rest.
    Split {
        direction: Direction,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}
```
```
crates/holler-adapter-herdr/src/layout.rs:82-108, 143-145
impl GridMap {
    /// The pane at `cell`; `None` for a free cell or an unplaced slot.
    pub fn at(&self, cell: GridPos) -> Option<&PaneId> {
        match self.rows.get(index(cell.row)?)?.get(index(cell.col)?)? {
            Slot::Pane(pane) => Some(pane),
            Slot::Unplaced => None,
        }
    }

    /// Where `pane` sits; `None` when it is unplaced or not in the map.
    pub fn position_of(&self, pane: &PaneId) -> Option<GridPos> {
        self.placed()
            .find(|(_, placed)| *placed == pane)
            .map(|(cell, _)| cell)
    }

    /// The placed panes, by row then column.
    pub fn cells(&self) -> Vec<(GridPos, PaneId)> {
        self.placed()
            .map(|(cell, pane)| (cell, pane.clone()))
            .collect()
    }

    /// The panes of every unplaced slot, in tree order.
    pub fn unplaced(&self) -> &[PaneId] {
        &self.unplaced
    }
...
/// Read the grid off Herdr's tree: the root's chain of `down` splits gives the rows,
/// and each row's chain of `right` splits gives its slots, both counted from 1.
pub fn grid_of(root: &LayoutNode) -> GridMap {
```
`GridMap` derives `Debug, Clone, Default, PartialEq` (`layout.rs:73`). `GridMap::default()` is the empty workspace.
The base is applied in exactly two private functions, `number` (`layout.rs:209-211`, `index + 1`) and `index`
(`layout.rs:215-217`, `number - 1`).

**`plan.rs`: the planner.**
```
crates/holler-adapter-herdr/src/plan.rs:38-106
/// `grid-unreachable`: Herdr cannot reach the cell by splits without nesting.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");

/// Where an empty workspace starts: its root pane.
const ROOT: GridPos = GridPos { row: 1, col: 1 };

/// A workspace's size, as the caller configures it (Herdr has no grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub rows: u16,
    pub cols: u16,
}

/// The cells wanted, inside an extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub extent: Extent,
    pub cells: Vec<GridPos>,
}

/// One Herdr call of a plan.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `workspace.create`: its root pane is r1c1.
    CreateRoot,
    /// Split the pane at `from`; the new pane lands at `creates`.
    Split {
        from: GridPos,
        direction: Direction,
        ratio: f64,
        creates: GridPos,
    },
}

/// The steps that take `existing` to a map holding every cell of `target`: the cells
/// that exist need none, and each step makes one new cell, rows first. The input order
/// of `target.cells` does not change the plan.
pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError> {
    let extent = target.extent;
    let mut cells = target.cells.clone();
    cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
    cells.dedup();
    if let Some(&outside) = cells.iter().find(|cell| !contains(extent, **cell)) {
        return Err(out_of_range(outside, extent));
    }
    cells.retain(|cell| existing.at(*cell).is_none());
    let Some(&first) = cells.first() else {
        return Ok(Vec::new());
    };
    let unplaced = existing.unplaced().len();
    if unplaced > 0 {
        return Err(refuse(
            first,
            &format!(
                "the workspace is not a grid ({} in a split nested inside one cell), and no \
                 pane is added to a workspace that is not one",
                count(unplaced, "pane")
            ),
        ));
    }
    // A new cell in column 1 is a new row; the rest are new columns. Both stay in order.
    let (rows, cols): (Vec<GridPos>, Vec<GridPos>) =
        cells.into_iter().partition(|cell| cell.col == 1);
    let mut grid = Widths::of(existing);
    rows.into_iter()
        .chain(cols)
        .map(|cell| grid.add(cell, extent))
        .collect()
}
```
Each `Widths::add` produces exactly one step for one cell, or refuses. With one target cell the plan is therefore
`Ok([])`, `Ok([one step])` or an error. `Step::CreateRoot` comes only for `r1c1` on an empty map
(`plan.rs:134-143`). The ratio is `1 / (size - number + 2)` (`plan.rs:226-228`).

### The port, its types and errors (`holler-pane`, frozen by #637)
```
crates/holler-pane/src/ports.rs:84-98
/// Where `HerdrPort::ensure_pane` should put a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSpec {
    pub session: String,
    pub workspace: String,
    pub grid: GridPos,
}

/// What Herdr reports about every pane it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSnapshot {
    pub panes: Vec<HerdrPane>,
}
```
```
crates/holler-pane/src/ports.rs:118-148
/// Herdr, reached over its local socket (the adapter is `holler-adapter-herdr`,
/// #640). **Provisional** until spike #636 reports.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// Only the adapter converts a [`GridPos`] to Herdr's own order and base.
pub trait HerdrPort: Send + Sync {
    /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail
    /// loudly; never relocates a healthy pane.
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;

    /// Type `text` into the pane.
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;

    /// Press `keys` in the pane.
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError>;

    /// The last `max_lines` lines of the pane's screen.
    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError>;

    /// Close the pane.
    fn close(&self, pane: &PaneId) -> Result<(), PaneError>;

    /// Every pane Herdr has, with its position.
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError>;

    /// Herdr's API version. An unknown version is `herdr-version-unsupported`.
    fn version(&self) -> Result<String, PaneError>;
}
```
```
crates/holler-pane/src/pane.rs:74-100
/// Herdr's own identifier of a pane (opaque to Holler, e.g. `p_12`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneId(String);

impl PaneId {
    /// A pane id from Herdr's text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id, verbatim.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
}
```
```
crates/holler-pane/src/grid.rs:34-40, 68-73
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    /// The row, from 1.
    pub row: u16,
    /// The column, from 1.
    pub col: u16,
...
impl fmt::Display for GridPos {
    /// The `rRcC` form, row first: `r2c1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}c{}", self.row, self.col)
    }
}
```
```
crates/holler-pane/src/error.rs:406-416, 451-467, 491
    NotImplemented,
    /// `usage`: the request is malformed: a missing or conflicting argument, a bad
    /// name, a params object that does not decode. (#637, every verb.)
    Usage { message: String },
    /// `grid-ambiguous`: a grid position that can be read more than one way, or not
    /// at all. `what` quotes the text that was refused and says what to write
    /// instead. (#637, `GridPos`.)
    GridAmbiguous { what: String },
    /// `grid-out-of-range`: a row or column of zero, or above `u16::MAX`. `what`
    /// quotes the text that was refused and gives the bounds. (#637, `GridPos`.)
    GridOutOfRange { what: String },
...
    /// `herdr-version-unsupported`: Herdr reports an API version the adapter does not
    /// know; `message` names the version and the supported ones. (#640.)
    HerdrVersionUnsupported { message: String },
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names it. (#638-#642.)
    Timeout { op: String },
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
    /// `session-not-found`: no harness session of that id; `what` is the id.
    /// (#638-#642.)
    SessionNotFound { what: String },
    /// `store-corrupt`: a stored file or record cannot be read back; the store fails
    /// closed rather than dropping state. `what` names the store. (#639/#661.)
    StoreCorrupt { what: String },
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
...
    Refused { code: RefusalCode, message: String },
```
```
crates/holler-pane/src/error.rs:288-298  (inside class_of)
        | PaneCode::HerdrVersionUnsupported
        | PaneCode::ProfileNotFound
        | PaneCode::PaneNotFound
        | PaneCode::SessionNotFound => ErrorClass::Refusal,
        // Went wrong while doing the work: a race between writers, a bound that ran
        // out, something unreachable or unreadable, live state that disagrees with
        // its spec, or work the verb cannot do yet.
        PaneCode::GenerationConflict
        | PaneCode::ProfileConflict
        | PaneCode::Timeout
        | PaneCode::Unavailable
```
So `pane-not-found` and `herdr-version-unsupported` exit 3 (refusal), and `timeout` and `unavailable` exit 1 (failure).

### The conformance suite this part must pass (`holler-pane-testkit`, #638/#683)
```
crates/holler-pane-testkit/src/conformance/herdr.rs:49-64
// ASSUMPTION (#640): Herdr has no grid (spike section 2), so the port under test must
// know that the fixture's workspace is 2 rows by 1 column, or case 2's `r1c2` is not out
// of range. How the real adapter learns a workspace's size is #640's.
// ASSUMPTION (#640): a real workspace is created with a root pane (spike section 4), and
// the fake's starts empty. Every case holds either way.
/// What one case runs against: a port serving the scratch Herdr session `session`, in
/// which the workspace `workspace` is 2 rows by 1 column and holds no pane, or only its
/// root pane at `r1c1` (a real Herdr workspace is created with one).
pub struct HerdrFixture<H> {
    /// The implementation under test.
    pub port: H,
    /// The scratch session: never the default session or a live one.
    pub session: String,
    /// The workspace of 2 rows by 1 column that the cases place their panes in.
    pub workspace: String,
}
```
```
crates/holler-pane-testkit/src/conformance/herdr.rs:122-152
/// Run every case of [`herdr_cases`], in order, each against a fresh fixture, and
/// return every case that did not hold.
///
/// `fresh` is called once per case. It returns the fixture and a guard that the suite
/// keeps alive for that case only: `()` for the fake, the scratch server's handle for
/// the adapter (#640). The fixture is dropped before its guard. How each implementation
/// runs the suite:
///
/// ```text
/// // the fake:
/// run_herdr_conformance(|| {
///     let port = FakeHerdr::new("scratch").with_workspace("scratch", 2, 1).expect("workspace");
///     (HerdrFixture { port, session: "scratch".into(), workspace: "scratch".into() }, ())
/// })
/// // the adapter (#640, opt-in, #[ignore]): a fresh scratch Herdr session per case whose
/// // workspace the adapter knows to be 2 rows by 1 column; the session's server handle
/// // as the guard; never the default or a live session.
/// ```
pub fn run_herdr_conformance<H, K, F>(fresh: F) -> Conformance
where
    H: HerdrPort,
    F: FnMut() -> (HerdrFixture<H>, K),
{
    run_cases(&CASES, fresh, |case, fixture| {
        case(&Scratch {
            port: &fixture.port,
            session: &fixture.session,
            workspace: &fixture.workspace,
        })
    })
}
```
```
crates/holler-pane-testkit/src/conformance/mod.rs:40-41
/// `Ok` when every case held; otherwise every failure, in case order.
pub type Conformance = Result<(), Vec<CaseFailure>>;
```
```
crates/holler-pane-testkit/src/conformance/herdr.rs:79-115
const CASES: [(&str, Case); 11] = [
    (
        "ensure-r2c1-reads-back-as-r2c1",
        ensure_r2c1_reads_back_as_r2c1,
    ),
    (
        "ensure-r1c2-is-grid-out-of-range",
        ensure_r1c2_is_grid_out_of_range,
    ),
    ("ensure-is-idempotent", ensure_is_idempotent),
    (
        "ensure-never-moves-another-pane",
        ensure_never_moves_another_pane,
    ),
    ("closed-id-is-never-reused", closed_id_is_never_reused),
    (
        "ids-unique-and-stable-when-a-sibling-closes",
        ids_unique_and_stable_when_a_sibling_closes,
    ),
    (
        "close-unknown-is-pane-not-found",
        close_unknown_is_pane_not_found,
    ),
    (
        "close-twice-is-pane-not-found",
        close_twice_is_pane_not_found,
    ),
    (
        "calls-on-a-closed-pane-are-pane-not-found",
        calls_on_a_closed_pane_are_pane_not_found,
    ),
    (
        "read-returns-at-most-max-lines",
        read_returns_at_most_max_lines,
    ),
    ("version-is-reported", version_is_reported),
];
```
The suite never constructs the port: `run_herdr_conformance` (above) takes it from the fixture that `fresh` returns,
so the adapter's `connect` runs inside `fresh`, once per case.

The 11 cases (`conformance/herdr.rs:79-115`), in order: `ensure-r2c1-reads-back-as-r2c1`,
`ensure-r1c2-is-grid-out-of-range`, `ensure-is-idempotent`, `ensure-never-moves-another-pane`,
`closed-id-is-never-reused`, `ids-unique-and-stable-when-a-sibling-closes`, `close-unknown-is-pane-not-found` (id
`w999:p999`), `close-twice-is-pane-not-found`, `calls-on-a-closed-pane-are-pane-not-found`,
`read-returns-at-most-max-lines` (types 5 lines with `send_text` + `send_keys([enter])` and reads 2), and
`version-is-reported`. Two of them close panes in ways that real Herdr reshapes:
```
crates/holler-pane-testkit/src/conformance/herdr.rs:246-278
/// Case 5: an id is never reused. After `b` at `r2c1` closes, the pane made there again
/// has an id that is neither `a`'s nor `b`'s.
fn closed_id_is_never_reused(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    s.close(&b.pane_id)?;
    let c = s.ensure(R2C1)?;
    new_id(
        "the pane made at r2c1 after its pane closed",
        &c.pane_id,
        &[&a.pane_id, &b.pane_id],
    )
}

/// Case 6: ids are unique, and an id does not change when its sibling closes. After `a`
/// at `r1c1` closes, the workspace lists `b`'s id and not `a`'s. Ids only: real Herdr
/// gives a closed pane's space to its sibling, so `b`'s cell may change.
fn ids_unique_and_stable_when_a_sibling_closes(s: &Scratch<'_>) -> Result<(), String> {
    let a = s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?;
    new_id("the pane made at r2c1", &b.pane_id, &[&a.pane_id])?;
    s.close(&a.pane_id)?;
    let ids: Vec<PaneId> = s.panes()?.into_iter().map(|pane| pane.pane_id).collect();
    if ids.contains(&b.pane_id) && !ids.contains(&a.pane_id) {
        return Ok(());
    }
    Err(format!(
        "after closing {} (r1c1), expected the workspace's ids to include {} (r2c1) and \
         not the closed one; got {ids:?}",
        a.pane_id.as_str(),
        b.pane_id.as_str()
    ))
}
```
Cases 7 to 9, which depend on what Herdr answers about a closed or unknown pane (added by the amender, round 2,
review NV-7; verbatim):
```
crates/holler-pane-testkit/src/conformance/herdr.rs:280-324
/// Case 7: closing an id that Herdr never handed out is `pane-not-found`.
fn close_unknown_is_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    expect_code(
        &format!("close({UNKNOWN_ID}), an id never handed out"),
        s.port.close(&PaneId::new(UNKNOWN_ID)),
        NOT_FOUND,
    )
}

/// Case 8: closing a pane succeeds once, and closing it again is `pane-not-found`. The
/// workspace keeps `a`, so no case empties a real workspace.
fn close_twice_is_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?.pane_id;
    s.close(&b)?;
    expect_code(
        &format!("close({}) again", b.as_str()),
        s.port.close(&b),
        NOT_FOUND,
    )
}

/// Case 9: a closed pane answers nothing: `send_text`, `send_keys` and `read` of it are
/// each `pane-not-found`.
fn calls_on_a_closed_pane_are_pane_not_found(s: &Scratch<'_>) -> Result<(), String> {
    s.ensure(R1C1)?;
    let b = s.ensure(R2C1)?.pane_id;
    s.close(&b)?;
    let closed = b.as_str();
    expect_code(
        &format!("send_text({closed}, \"x\") after it closed"),
        s.port.send_text(&b, "x"),
        NOT_FOUND,
    )?;
    expect_code(
        &format!("send_keys({closed}, [enter]) after it closed"),
        s.port.send_keys(&b, &[enter()]),
        NOT_FOUND,
    )?;
    expect_code(
        &format!("read({closed}, 1) after it closed"),
        s.port.read(&b, 1),
        NOT_FOUND,
    )
}
```
`NOT_FOUND` is `"pane-not-found"`, `UNKNOWN_ID` is `"w999:p999"`, and case 10 types `TYPED_LINES = 5` lines and reads
`READ_LINES = 2` (`conformance/herdr.rs:40-47`). Case 10 (`:326-354`) presses `Key::new("enter")` after each
`send_text` and passes when `read` returns at most 2 lines. Case 11 (`:356-365`) passes when `version()` is non-empty
and holds no `\n` or `\r`. The suite reads a fixture's panes by label:
```
crates/holler-pane-testkit/src/conformance/herdr.rs:392-401
    /// The workspace's panes: those of `snapshot` in the fixture's session and
    /// workspace, in the order the snapshot lists them.
    fn panes(&self) -> Result<Vec<HerdrPane>, String> {
        let snapshot = succeeds("snapshot", self.port.snapshot())?;
        Ok(snapshot
            .panes
            .into_iter()
            .filter(|pane| pane.session == self.session && pane.workspace == self.workspace)
            .collect())
    }
```
So case 9 passes only if the wire fake answers `pane_not_found` to `pane.send_text`, `pane.send_keys` and `pane.read`
of a closed pane, not just to `pane.close` (Decision 15, "Unknown panes").

The test kit's `ASSUMPTION (#640)` comments that this part answers:
```
crates/holler-pane-testkit/src/herdr.rs:26-40
// ASSUMPTION (#640): only protocol 22 was tested. How other versions differ is
// unverified (spike section 13: no second build was compared), so `UNSUPPORTED_VERSION`
// is invented.
/// The version string of the one Herdr build the spike tested (protocol 22).
pub const PROTOCOL_22_VERSION: &str = "0.9.1-preview.2026-09-21-0ff0f27e2226";

// ASSUMPTION (#640): provisional test vocabulary, like `GRID_UNREACHABLE` below.
/// What a `herdr-version-unsupported` message names as supported (spike section 13).
/// Provisional test vocabulary: a verb test compares against this constant, never the
/// literal.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";

/// The version string of the fake's unsupported build. Invented: no second build was
/// compared.
pub const UNSUPPORTED_VERSION: &str = "99.0.0-fake";
```
```
crates/holler-pane-testkit/src/herdr.rs:318-320, 358-360
            // ASSUMPTION (#640): only `version()` refuses an unsupported build. Whether
            // the adapter also refuses every other call after a failed version check is
            // #640's.
...
        // ASSUMPTION (#640): a session or a workspace the fake does not serve is
        // `unavailable`. Whether the adapter creates a missing workspace
        // (`workspace.create`) is #640's.
```
The fake's split-only rule, which is looser than real Herdr (see "For the operator", item 1):
```
crates/holler-pane-testkit/src/herdr.rs:433-445
    /// Step 4, split-only placement: a free cell is reachable as the root of an empty
    /// workspace (`r1c1`), or by a `right` split of the pane on its left or a `down`
    /// split of the pane above it. No call places a pane at a cell (spike section 7).
    fn check_split(&self, grid: GridPos) -> Result<(), PaneError> {
        // Row 0 and column 0 hold no pane, so nothing is left of the first column or
        // above the first row.
        let holds = |row: u16, col: u16| self.panes.contains_key(&(row, col));
        let root = self.panes.is_empty() && (grid.row, grid.col) == (1, 1);
        let left = holds(grid.row, grid.col.saturating_sub(1));
        let above = holds(grid.row.saturating_sub(1), grid.col);
        if root || left || above {
            return Ok(());
        }
```
The base-36 id rule the wire fake copies (it is private to the test kit, so the wire fake re-implements it):
```
crates/holler-pane-testkit/src/herdr.rs:506-518
/// `n` in base 36 with upper-case letters, as Herdr numbers its panes (spike section
/// 5): `1` to `9`, `A` to `Z`, then `10`.
fn base36(n: u64) -> String {
    let lowest_first: String =
        std::iter::successors(Some(n), |rest| (*rest >= 36).then_some(rest / 36))
            .filter_map(|rest| char::from_digit(u32::try_from(rest % 36).ok()?, 36))
            .collect();
    lowest_first
        .chars()
        .rev()
        .map(|digit| digit.to_ascii_uppercase())
        .collect()
}
```

### What real Herdr does on the wire (`docs/research/herdr-api-spike.md`, spike #636)
```
docs/research/herdr-api-spike.md:104-112
**Wire** (VERIFIED, `herdr-ops.sh`):

- A Unix stream socket, owner-only (`srw-------`). Default session: `$XDG_CONFIG_HOME/herdr/herdr.sock` (on Linux
  `~/.config/herdr/herdr.sock`); a named session: `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`.
  `herdr session list` prints each session's socket path. Inside a Herdr pane, `HERDR_SOCKET_PATH` names it.
- One JSON request per line, and **one request per connection**: a second request on the same connection is
  refused (`ConnectionResetError` / broken pipe). `events.subscribe` is the exception: the connection stays open
  and streams events.
- An unknown method is `{"error":{"code":"invalid_request","message":"invalid request: unknown variant `pane.nope`, expected one of ..."}}`.
```
```
docs/research/herdr-api-spike.md:124, 126, 129-131, 133   (rows of the operations table)
| Create a workspace | yes | `workspace.create {cwd?, label?, env?, focus?}`; `workspace create --cwd D --label L --no-focus` | `{type: "workspace_created", workspace: WorkspaceInfo, tab: TabInfo, root_pane: PaneInfo}` | ids `w<N>`, `w<N>:t<M>`, `w<N>:p<M>`, never reused |
| Split | yes | `pane.split {target_pane_id, direction: "right"\|"down", ratio?, cwd?, env?, focus?}`; `pane split ID --direction right --ratio 0.25 --no-focus` | `{type: "pane_info", pane: PaneInfo}`; new pane id in `.result.pane.pane_id` | no `command` param: the new pane runs the default shell. `left`/`up` are `invalid_request` |
| Send text | yes | `pane.send_text {pane_id, text}`; `pane send-text ID TEXT` | `{type: "ok"}` | literal text, no Enter |
| Send keys | yes | `pane.send_keys {pane_id, keys: ["enter"]}`; `pane send-keys ID enter` | `{type: "ok"}` | logical key names (`enter`, `esc`, `ctrl+c`, ...) |
| Read | yes | `pane.read {pane_id, source: "visible"\|"recent"\|"recent_unwrapped"\|"detection", lines?, format?: "text"\|"ansi", strip_ansi?}`; `pane read ID --source recent --lines 4` (plain text) | `{type: "pane_read", read: {pane_id, workspace_id, tab_id, source, format, text, revision, truncated}}` | `revision` increases with output |
| Close | yes | `pane.close {pane_id}`; `pane close ID` | `{type: "ok"}`; again: `{"error":{"code":"pane_not_found","message":"pane w1:p5 not found"}}`, exit 1 | the sibling takes the space; a shell that exits closes its pane too (`pane_exited`) |
```
```
docs/research/herdr-api-spike.md:161-163
The number after `p` is **not decimal**: after `w1:p9` came `w1:pA` ... `w1:pF`, `w1:pG` (base 36 on the saved
counter `next_public_pane_number`). Treat ids as opaque strings; never parse or predict them, read them from
responses. A workspace's `number` field is its display ordinal (it was `2` for `w3`), not its id.
```
```
docs/research/herdr-api-spike.md:228-239, 247-248
- **Splits only, no absolute cells.** A tab's layout is a binary tree. An inner node is a split with `direction`
  `right` (side by side) or `down` (stacked) and a `ratio`; a leaf is a pane. No call places a pane at a cell.
  `pane.split` accepts only `right` and `down`: `--direction left` exits 2, and a socket `"left"` is
  `invalid_request: unknown variant 'left', expected 'right' or 'down'`. The new pane is always the `second`
  child (right of or below the target). VERIFIED.
- **Ratio** is the **first** child's share: splitting a 120-wide pane `right` with `ratio 0.25` leaves the target
  30 wide and gives the new pane 90. It is stored as a 32-bit float (`0.3333` comes back as
  `0.33329999446868896` in events). VERIFIED.
- **The nesting rule: a split divides only the target pane's own cell.** After the 2x4 grid, `split r1c2 down`
  changed only `r1c2` (`{x:30,y:0,w:30,h:20}` became `{x:30,y:0,w:30,h:10}`) and added the new pane at
  `{x:30,y:10,w:30,h:10}`. No other rect changed. In the same way, splitting the top pane `right` left the full-width
  bottom pane alone. VERIFIED.
...
- **Close** gives the closed pane's space to its sibling subtree (closing `r2c4` made `r2c3` 60 wide). It can turn
  a regular grid into an irregular one.
```
```
docs/research/herdr-api-spike.md:375-376, 390-391
4. The socket path is the only configuration: `<config-home>/herdr/sessions/<name>/herdr.sock` or the default.
   Take it from configuration or `herdr session list`.
...
The adapter must never call `server.stop`, `server.live_handoff`, `layout.apply` with a `tab_id`, or the
`integration.*` and `plugin.*` mutators.
```
```
docs/research/herdr-api-spike.md:415-416, 450-454
- From the running server: `ping` returns `{"type":"pong","version":"0.9.1-preview.2026-09-21-0ff0f27e2226",
  "protocol":22,"capabilities":{"live_handoff":true,"detached_server_daemon":false,"endpoint_protocol_generation":1,"surface_interest":true,"health_check":true}}`.
...
**Which versions #640 should support:** exactly **protocol 22 with schema version 1** (Herdr 0.9.1). INFERRED to
be what the live fleet runs: the live server's executable is the same file as the tested binary, which predates
the server's start; the live server itself was not queried. Read `ping` on connect and record the version string. Refuse any other protocol with
`herdr-version-unsupported`, and have the message name "Herdr protocol 22 (0.9.1)". Widen the set only after
```
The spike's own raw socket client, which a real Herdr answered (it does **not** half-close the write side):
```
scripts/spikes/herdr-lib.sh:66-89
# Raw socket client: newline-delimited JSON, one request per connection.
# hsock.py SOCKET REQUEST [STREAM_SECONDS]: without STREAM_SECONDS it prints the first line;
# with it, it prints every line received in that many seconds (for events.subscribe).
cat >"$SPIKE_BIN/hsock.py" <<'EOF'
import socket, sys
path, req = sys.argv[1], sys.argv[2]
stream = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(stream if stream else 10)
s.connect(path)
s.sendall(req.encode() + b"\n")
buf = b""
try:
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
        if not stream and b"\n" in buf:
            buf = buf.split(b"\n", 1)[0] + b"\n"
            break
except socket.timeout:
    pass
sys.stdout.write(buf.decode())
```

### The pattern to follow, and the rules
The hub's control-socket client is the closest existing JSON-line Unix-socket client. Follow it, but do not import it.
`holler-hub` is off limits to an adapter (ADR-0021 §5, below).
```
crates/holler-hub/src/control.rs:485-518
fn send_over(
    path: &PathBuf,
    id_literal: &str,
    method: &str,
    params: Option<serde_json::Value>,
    timeout: std::time::Duration,
) -> Result<serde_json::Value, ControlError> {
    let stream = UnixStream::connect(path).map_err(|_| ControlError::NoLiveHub)?;
    stream.set_read_timeout(Some(timeout)).map_err(ControlError::Io)?;

    let cid = CorrelationId::parse(id_literal).expect("a well-formed literal control id");
    let req = holler_proto::Envelope::request(&cid, method, params);
    let bytes = format!("{}\n", holler_proto::encode(&req).expect("encode the control request"));

    let mut stream = stream;
    stream.write_all(bytes.as_bytes()).map_err(ControlError::Io)?;
    stream.flush().map_err(ControlError::Io)?;

    let mut line = String::new();
    let n = std::io::BufReader::new(stream)
        .read_line(&mut line)
        .map_err(ControlError::Io)?;
    if n == 0 {
        return Err(ControlError::BadReply("the hub closed the socket before replying".into()));
    }

    let env = holler_proto::decode(&line).map_err(|e| ControlError::BadReply(e.to_string()))?;
    if let Some(error) = env.error() {
        return Err(ControlError::Refused(error.clone()));
    }
    env.result()
        .cloned()
        .ok_or_else(|| ControlError::BadReply("the hub reply carried no result".into()))
}
```
That client has three gaps this part must not copy:

- `UnixStream::connect` is unbounded.
- `SO_RCVTIMEO` bounds each `read`, not the whole reply, so a server that drips one byte at a time never times out.
- `read_line` grows without limit until it sees a newline.
```
docs/adr/ADR-0021.md:182-183
- Each adapter crate depends on `holler-pane` and implements one port. `holler-cli` depends on the adapter crates; `holler-hub`
  does not.
docs/adr/ADR-0021.md:395-396
  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. |
  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply). |
docs/adr/ADR-0021.md:424-426
- **`grid-out-of-range`:** a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535; or, from `HerdrPort::ensure_pane`,
  a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort`
  conformance suite pins (#638 amendment 2026-10-08, grid; #683). How the adapter learns a workspace's extent is #640's.
```
```
Cargo.toml:19-30   (workspace lints; clippy.toml sets too-many-lines-threshold = 100, cognitive-complexity-threshold = 15)
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
cognitive_complexity = "deny"
too_many_lines = "deny"
struct_excessive_bools = "deny"
large_enum_variant = "warn"

[workspace.lints.rust]
dead_code = "deny"
Cargo.toml:160-161
# scratch state dirs for the test harness.
tempfile = "3"
```
`tempfile` is already a workspace dependency and a dev-dependency of `holler-hub`, `holler-body` and `holler-cli`.
`scripts/lint.sh` fails any file over 900 lines. It also fails any `#[allow(...)]` without a trailing `// #NNN`. The
baseline grep `grep -rnE "TcpStream|TcpListener|47001|47002|tmux|std::env|env::var|\.config/herdr|HERDR_|thread::sleep|unsafe|Mutex|RefCell" crates/holler-adapter-herdr/`
prints nothing today. `cargo fmt --check -p holler-adapter-herdr` exits 0 today. Workspace-wide `cargo fmt --check`
exits 1 on files outside this crate (epic ruling 4: the tree is not rustfmt-clean).

## The API this part creates (pinned; T writes against it, F fills it)

`src/lib.rs`:

- The "Part 1 of story #640" paragraph (lines 24-30) becomes a list of all five modules: the three part-1 modules,
  plus `transport` (the socket and its deadline) and `adapter` (`HerdrAdapter`, the `HerdrPort`).
- Add `pub mod adapter; pub mod transport;`.
- Still **no flat re-exports**: the rule at `lib.rs:32-35` stands, so items are reached as
  `holler_adapter_herdr::adapter::HerdrAdapter`.

**`src/transport.rs`**: one request, one connection, one deadline.
```rust
/// The most bytes a reply line may hold; a longer one is `unavailable`.
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

/// How the adapter reaches Herdr: one request out, one reply line back, by a deadline.
pub trait Transport: Send + Sync {
    /// Send `request.to_line()` and return Herdr's reply line without its newline, or
    /// `timeout` (`op` = "herdr.<method>") once `deadline` passes, or `unavailable`.
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError>;
}
impl<T: Transport + ?Sized> Transport for Arc<T> { /* forwards */ }

/// Herdr's local Unix socket: a new connection per request (spike section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixSocketTransport { /* socket: PathBuf */ }
impl UnixSocketTransport {
    pub fn new(socket: impl Into<PathBuf>) -> Self;
    pub fn socket(&self) -> &Path;
}
impl Transport for UnixSocketTransport { /* Decisions 2-4 */ }
```
**`src/adapter.rs`**: the `HerdrPort`.
```rust
/// I5's bound for one `HerdrPort` call.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// What the adapter is told (Herdr has no grid and no discoverable socket of Holler's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrConfig {
    pub session: String,                         // the one Herdr session served
    pub socket: PathBuf,                         // that session's socket, absolute
    pub workspaces: BTreeMap<String, Extent>,    // workspace label -> rows by columns
    pub timeout: Duration,                       // per HerdrPort call
}
impl HerdrConfig {
    pub fn new(session: impl Into<String>, socket: impl Into<PathBuf>) -> Self; // no workspace, DEFAULT_TIMEOUT
    pub fn with_workspace(self, label: impl Into<String>, extent: Extent) -> Self;
}

pub struct HerdrAdapter<T = UnixSocketTransport> { /* config: HerdrConfig, transport: T; nothing else */ }
impl HerdrAdapter<UnixSocketTransport> {
    pub fn connect(config: HerdrConfig) -> Result<Self, PaneError>;  // = connect_with(config, UnixSocketTransport::new(socket))
}
impl<T: Transport> HerdrAdapter<T> {
    pub fn connect_with(config: HerdrConfig, transport: T) -> Result<Self, PaneError>;
    pub fn config(&self) -> &HerdrConfig;
}
impl<T: Transport> HerdrPort for HerdrAdapter<T> { /* Decisions 7-13 */ }
```
F may add private helpers and derive traits. F may not rename or drop a pinned item or change a pinned signature. If F
finds one unworkable, F stops and reports. F writes no request JSON by hand: every request is a part-1 `Request`.

**The wire fake** (test code, T's): `tests/wire_herdr/mod.rs` (the simulated Herdr) and `tests/wire_herdr/serve.rs`
(serving it on a temporary Unix socket). A directory with a `mod.rs` is not a test target, so each test file that needs
it declares `mod wire_herdr;`. Pinned API:
```rust
pub struct WireHerdr { /* Mutex<State>; not Clone, share it with Arc */ }
impl WireHerdr {
    pub fn new() -> Self;                                     // protocol 22, no workspace
    pub fn set_protocol(&self, protocol: Option<u32>);        // Some(22) default; Some(99); None = pong without `protocol`
    pub fn create_workspace(&self, label: &str) -> PaneId;    // as `workspace.create` would; its root pane
    pub fn add_tab(&self, label: &str) -> PaneId;             // a second tab (number 2) with its own root pane
    pub fn split(&self, target: &PaneId, direction: Direction) -> PaneId; // the outside world splits a pane (ratio 0.5)
    pub fn tree(&self, label: &str) -> Option<serde_json::Value>; // the grid tab's root, as layout.export gives it
    pub fn requests(&self) -> Vec<serde_json::Value>;         // every request line received, parsed, oldest first
    pub fn methods(&self) -> Vec<String>;                     // their `method`s
    pub fn answer(&self, line: &str) -> String;               // one request line in, one reply line out (no newline)
}
impl Transport for WireHerdr { /* Ok(self.answer(&request.to_line())), the deadline unused */ }
// serve.rs
pub struct Served { /* TempDir, socket path, stop flag, thread */ }
impl Served { pub fn path(&self) -> &Path; }                   // Drop stops the thread and removes the directory
pub fn serve(fake: Arc<WireHerdr>) -> Served;
```

## Acceptance criteria

All tests run in the default `cargo test -p holler-adapter-herdr`. None needs Herdr, a network, or any socket outside a
`tempfile::tempdir()`. Test names are pinned. A test file starts with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640`, as the part-1 tests do.

**The transport** (`tests/transport_test.rs`). Each test has its own `UnixListener` bound in a `tempfile::tempdir()`
at the short path `<dir>/h.sock`, with a server thread written in the test:

1. `exchange_writes_the_request_line_and_returns_the_reply_line`: the server reads to `\n` and answers
   `{"id":"holler:ping","result":{"type":"pong","version":"v","protocol":22}}\n`. `exchange(&Request::Ping, now+5s)`
   returns that line without the `\n`. The test compares the returned `String` with the expected line as text and
   decodes nothing (Decision 14: `exchange` returns the raw line). The server received exactly
   `Request::Ping.to_line()`.
2. `each_request_opens_its_own_connection`: two exchanges give two accepted connections, each carrying exactly one line.
3. `a_missing_or_non_socket_path_is_unavailable_naming_it`: a path with no file, and a path that is a regular file,
   each give `PaneError::Unavailable` whose `what` contains the path's display form.
4. `a_socket_path_too_long_for_the_os_is_unavailable`: a 200-byte path inside the tempdir gives `Unavailable`, with
   no panic.
5. `a_silent_server_is_timeout_by_the_deadline`: the server accepts and never writes, holding the connection open
   until the test ends. A deadline of now+300ms gives `PaneError::Timeout { op: "herdr.ping" }`, and the call returns
   in under 300ms plus 2s.
6. `a_dripping_server_is_timeout_by_the_deadline`: the server writes one byte with no newline every 50ms. A deadline
   of now+300ms gives `Timeout` in under 300ms plus 2s, so the deadline bounds the whole reply, not one read. (The
   server's 50ms pause simulates slowness. It is not a readiness wait.)
7. `a_server_that_closes_without_replying_is_unavailable`.
8. `a_reply_over_the_limit_is_unavailable`: the server writes `MAX_REPLY_BYTES + 1` bytes with no newline. The result
   is `Unavailable`, whose `what` names the limit.
9. `a_reply_ended_by_eof_without_a_newline_is_returned`: the server writes a JSON line with no `\n` and closes.
   `exchange` returns it.
10. `a_non_utf8_reply_is_unavailable`.
11. `a_passed_deadline_is_timeout_without_connecting`: `exchange(&Request::Ping, Instant::now())` gives `Timeout`. A
    non-blocking `accept()` afterwards is `WouldBlock`.
12. `no_transport_error_echoes_typed_text`: a `Request::SendText` with the text `typed-secret-text`, against a silent
    server, a closing server and a missing path. No `Display` or `Debug` of the error contains `typed-secret-text`.

12a. `one_wire_condition_gives_one_answer_at_the_deadline` (Decision 3, "One answer per call"): against one silent
    server (it accepts and never writes), 20 sequential exchanges of `Request::Ping`, each with a deadline of
    now+50ms. (The short deadline is the rationale only: it makes Decision 3's two expiries race. The test asserts
    only observable results, never which expiry fired.) Every result is exactly
    `PaneError::Timeout { op: "herdr.ping" }` (never `Unavailable`), each call returns in under 50ms plus 2s, and the
    test ends without a panic on any thread (an abandoned worker's failed send is silent).
    **Worker lifetime** (amender round 2, review B-5; it checks Decision 3's "a worker abandoned on a timeout ends by
    itself once its socket timeouts fire", and changes no design: the caller still never joins a worker). The server
    keeps every connection it accepts. After the 20 exchanges, the test drains the listener with non-blocking
    `accept()` until `WouldBlock`, then reads each kept connection with a 2s read timeout. Each one must reach EOF
    (a read of 0 bytes, after at most the one ping line) within that 2s, never a read timeout. EOF means the worker
    dropped its end of the socket, so no worker outlived its call by more than 2s. At most 20 connections are
    accepted: a worker that finds no time left does not connect (Decision 3), so fewer is fine.

**The adapter over the wire fake** (`tests/adapter_test.rs`, in-process through `Arc<WireHerdr>` unless named):

13. `r2c1_and_r1c2_land_in_their_cells_and_read_back` (the issue's grid acceptance). Config workspace `w` is 2x2 and
    the fake starts empty. In order:
    - `ensure r1c1` gives `w1:p1`.
    - `ensure r2c1` gives `w1:p2` at `r2c1`. `fake.tree("w")` equals
      `{"type":"split","direction":"down","ratio":0.5,"first":{"type":"pane","pane_id":"w1:p1"},"second":{"type":"pane","pane_id":"w1:p2"}}`.
    - `ensure r1c2` gives `w1:p3`. The tree equals
      `{"type":"split","direction":"down","ratio":0.5,"first":{"type":"split","direction":"right","ratio":0.5,"first":{"type":"pane","pane_id":"w1:p1"},"second":{"type":"pane","pane_id":"w1:p3"}},"second":{"type":"pane","pane_id":"w1:p2"}}`.
    - `snapshot()` lists `w1:p1` r1c1, `w1:p3` r1c2 and `w1:p2` r2c1, in that order, and `.to_string()` of each `grid`
      reads `r1c1`, `r1c2`, `r2c1`.
    - The recorded `pane.split` params are `{"target_pane_id":"w1:p1","direction":"down","ratio":0.5,"focus":false}` and
      then `{"target_pane_id":"w1:p1","direction":"right","ratio":0.5,"focus":false}`.
14. **Swapped and off-by-one conversions fail** (the issue's "a deliberately swapped conversion fails these tests"). In
    GREEN, T applies each mutant below locally (never committed), runs AC 13, AC 16 and AC 20, and records in
    `handoff-T-green.md` which of them fails:
    - (a) `grid_of` with `Direction::Down` and `Direction::Right` swapped;
    - (b) `layout.rs` `number` returning `index` (0-based);
    - (c) the adapter sending `Direction::Right` where the plan says `Down`.

    Each mutant must fail at least AC 13. Beyond AC 13 (corrected, amender round 2, review B-1: the round-1 text
    required mutant (c) to fail AC 16, which the derivation below shows cannot happen):
    - mutant (a) must also fail AC 16 (its swapped `grid_of` accepts the rewritten `right` split as `r2c1`);
    - mutant (b) must also fail AC 20 (its read-back puts the created root pane at `r0c0`);
    - mutant (c) fails AC 13 and is **not** required to fail AC 16. AC 16's transport already turns `down` into
      `right`, so under (c) AC 16 sees the same wire and the same `Unavailable` it expects. T records the observed
      AC 16 result for (c) all the same.

    **The oracle per mutant** (amender, round 1; derived from merged `layout.rs:130-210`, `plan.rs` and Decision 11,
    not from a run). "Fails" means an AC 13 assertion fails on the value below. T records the observed value next to
    each one in `handoff-T-green.md`:

    | Mutant | First AC 13 assertion that fails | Observed instead |
    |---|---|---|
    | (a) `grid_of` reads rows from the `right` chain and slots from the `down` chain | bullet 2: `ensure r2c1` gives `w1:p2` at `r2c1` | `Err(PaneError::Unavailable)` whose `what` names `w1:p2` and `r1c2`: the read-back of the `down` split puts `w1:p2` at `r1c2`. (Bullet 1 holds: a lone leaf is `r1c1` either way.) |
    | (b) `number` returns the 0-based index | bullet 1: `ensure r1c1` gives `w1:p1` | `Err(PaneError::Unavailable)` whose `what` names `w1:p1` and `r0c0`: `cell_at(0, 0)` is `r0c0`, so the read-back after `workspace.create` puts the root pane at `r0c0`. |
    | (c) the adapter sends `right` for a planned `Down` | bullet 2: `ensure r2c1` gives `w1:p2` at `r2c1` | `Err(PaneError::Unavailable)` whose `what` names `w1:p2` and `r1c2`: Herdr (the fake) makes a `right` split, and the read-back refuses it. |

    Derivation behind the AC 16 and AC 20 requirements above (amender round 1, re-checked in round 2 against merged
    `layout.rs:143-217`): mutant (c) does **not** fail AC 16. AC 16's transport already turns `down` into
    `right`, so under mutant (c) its rewrite has nothing to change, and AC 16 still sees the `Unavailable` it expects.
    The mutant that does fail AC 16 is (a): its swapped `grid_of` reads the rewritten `right` split as rows, puts `w1:p2`
    at `r2c1`, and `ensure r2c1` answers `Ok` where AC 16 expects `Unavailable`. Mutant (b) fails AC 20's second half
    (`ensure r1c1`, the same `r0c0` read-back as above). T records what each mutant does to AC 16 and AC 20 as observed.
15. `a_closed_panes_space_goes_to_its_sibling`: in a 2x1, `a` is at r1c1 and `b` at r2c1. After `close(a)`, the fake's
    tree is the leaf `b`, and `snapshot()` lists `b` at `r1c1`.
16. `a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place`: a test transport rewrites `"direction":"down"` to
    `"right"` in a `pane.split` line before the fake answers it. In a 2x2 with `r1c1` present, `ensure r2c1` gives
    `Unavailable`, whose `what` names `w1:p2` and `r1c2`. Afterwards `snapshot()` still lists `w1:p2` at `r1c2`, and no
    `pane.close` was sent.
17. `a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found`: a test transport closes the target
    (it answers a `Request::Close` line to the fake) just before forwarding a `pane.split`. `ensure r2c1` gives
    `Unavailable`, not `PaneNotFound`.
18. `an_occupied_cell_sends_no_mutating_request`: `ensure r1c1` twice. The second call records only
    `session.snapshot` and `layout.export`.
19. `nesting_is_refused_before_any_split`. Both cases use a 2x2. In each, the call records no `pane.split` and no
    `workspace.create`:
    - (a) The fake makes `r1c2` by `fake.split(p1, Right)`. `ensure r2c1` is `Refused` with
      `code == plan::GRID_UNREACHABLE`.
    - (b) Then `fake.split(r1c2's pane, Down)` nests a slot. `snapshot()` lists only `w1:p1` (unplaced panes are left
      out), `ensure r1c1` answers `w1:p1`, and `ensure r2c1` is `grid-unreachable`.
20. `a_missing_workspace_is_created_only_for_r1c1`: a 2x1 with the fake empty. `ensure r2c1` is `grid-unreachable`,
    and the only method recorded is `session.snapshot`. Then `ensure r1c1` sends exactly one `workspace.create` with
    params `{"label":"w","focus":false}` and returns its root pane at `r1c1`.
21. `a_pane_in_another_tab_neither_lists_nor_places`: `fake.add_tab("w")` adds a second tab with a pane. `snapshot()`
    does not list that pane, and `ensure` in `w` still splits only the grid tab (the tab with the lowest `number`).
22. `snapshot_lists_every_workspace_by_label`: a workspace the config does not name is listed too, with its label and
    tree positions. `session` is the config's on every pane.
    - **Which string** (clarification, amender round 2, review B-3; Decision 12 already pins it): `HerdrPane.workspace`
      is the workspace's **label** as Herdr reports it, never its `w<N>` id. The conformance suite's own filter relies on
      this: `Scratch::panes` keeps the panes whose `workspace == self.workspace`, the label (`conformance/herdr.rs:392-401`).
    - **Duplicate labels** (an added case, Decision 12 "Duplicate labels do not fail the snapshot"): with two Herdr
      workspaces labelled `w` (ids `w1` and `w2`, each with a root pane), `snapshot()` is `Ok` and lists both root
      panes, `w1:p1` then `w2:p1` (Herdr's workspace order), each with `workspace == "w"` and `grid` `r1c1`. The two are
      told apart by `pane_id` only, whose `w<N>:` prefix Herdr assigns; the adapter does not parse it.
23. `session_and_workspace_errors_send_nothing`:
    - a `spec.session` other than the config's gives `Unavailable` and records no request;
    - a `spec.workspace` the config does not name gives `Unavailable` naming it, and records no request;
    - two Herdr workspaces labelled `w` give `Unavailable` from `ensure_pane`.
24. `keys_go_out_verbatim`: `send_keys(p, [Key::new("enter"), Key::new("ctrl+c")])` records `keys == ["enter","ctrl+c"]`.
25. `read_asks_for_recent_text_and_trims`: after 5 typed lines, `read(p, 2)` records params
    `{"pane_id":p,"source":"recent","lines":2,"format":"text"}` and returns the last two lines. `read(p, 0)` sends
    `"lines":1` and returns `""`.
26. `version_and_the_gate`:
    - `connect_with` against protocol 22 is `Ok`, and `version()` equals `holler_pane_testkit::herdr::PROTOCOL_22_VERSION`.
    - Against `Some(99)` (version `UNSUPPORTED_VERSION`) and against `None`, `connect_with` is
      `HerdrVersionUnsupported`. Each message is one line, names the reported version and contains
      `protocol::SUPPORTED_VERSIONS`. The 99 message contains `99`.
    - After a successful connect, `fake.set_protocol(Some(99))` makes `version()` `HerdrVersionUnsupported`, while
      `send_text` still succeeds (Decision 8).
27. `config_is_validated_before_any_request`: each of these gives `PaneError::Usage` from `connect_with`, and the fake
    records no request: a relative `socket`, an empty `session`, a zero `timeout`, an `Extent` with 0 rows or
    0 columns. Each `message` is one line and holds the substrings Decision 6 ("Order and message") names for that
    case. Two more cases:
    - two workspaces with invalid extents, `a` (0 rows) and `b` (0 columns): the one `Usage` names `a` and `rows`, and
      not `b`, so validation stops at the first invalid entry in label order;
    - an empty `session` together with a relative `socket`: the message names `session`, by the check order.
28. `one_deadline_covers_every_exchange_of_a_call`: a test transport records the `deadline` of every exchange. Every
    exchange of one `ensure_pane` (`r2c1` after `r1c1`), and of one `snapshot()`, carries the same `Instant` `d`, and
    `before + timeout <= d <= after + timeout`.
    **How it is measured** (amender round 2, review B-2):
    - The test transport wraps an `Arc<WireHerdr>`, forwards every exchange to it, and pushes each exchange's
      `deadline` onto a shared `Vec<Instant>` before forwarding.
    - The config's `timeout` is a known value, `DEFAULT_TIMEOUT` (or any fixed value the test names); `timeout` in the
      inequality is that value.
    - For each of the two measured calls, the test clears the `Vec`, takes `before = Instant::now()` on the line
      immediately before the call and `after = Instant::now()` on the line immediately after it returns, then reads
      the `Vec`.
    - `d` is the first recorded deadline. The assertions are: the `Vec` is not empty; every entry equals `d` (`==` on
      `Instant`); and `before + timeout <= d && d <= after + timeout`. `Instant` is monotonic, and Decision 3 takes
      the deadline as `Instant::now() + timeout` once on entry, between `before` and `after`, so both bounds hold
      exactly, with no tolerance and no wall-clock time.
    - The `ensure r1c1` that sets up the `r2c1` call is not measured.
29. `a_garbled_reply_is_unavailable_everywhere`: a test transport answers `not json` for every request.
    `connect_with` is `Unavailable`. After a good connect, with garbling switched on, each of the seven methods is
    `Unavailable`, and nothing panics.
30. `the_fake_numbers_panes_in_base_36`: after nine panes in a workspace, the tenth split gives `w1:pA`, and the adapter
    returns it verbatim.

**Conformance** (`tests/adapter_conformance_test.rs`):

31. `the_adapter_passes_the_suite_from_an_empty_herdr`: `run_herdr_conformance` with a fresh `WireHerdr` per case, no
    workspace in it, and `HerdrConfig::new("scratch", <unused absolute path>).with_workspace("scratch", Extent{rows:2,cols:1})`
    over `connect_with` gives `Ok(())`.
32. `the_adapter_passes_the_suite_with_a_root_pane`: the same, but `fake.create_workspace("scratch")` runs first, so
    the workspace holds its root pane. Gives `Ok(())`.
33. `the_adapter_passes_the_suite_over_a_real_socket`: each case serves a fresh `WireHerdr` with `serve()`, and
    `HerdrAdapter::connect(config with socket = served.path())` makes the port, with the `Served` as the guard. Gives
    `Ok(())`.
    **Construction order inside `fresh`** (amender round 2, review B-4; every step is on the pinned API):
    1. `let fake = Arc::new(WireHerdr::new());` (empty, as in AC 31), and keep an `Arc::clone(&fake)` in a list that
       the closure captures, for AC 34;
    2. `let served = serve(Arc::clone(&fake));` (`serve` binds the `UnixListener` before it spawns its thread and
       returns, so the `connect` below never races the bind and needs no readiness wait);
    3. `let config = HerdrConfig::new("scratch", served.path()).with_workspace("scratch", Extent { rows: 2, cols: 1 });`
       (`served.path()` is a `&Path`, which `impl Into<PathBuf>` copies, so the borrow of `served` ends here);
    4. `let port = HerdrAdapter::connect(config).expect("connect");` (its `ping` goes over the served socket);
    5. return `(HerdrFixture { port, session: "scratch".into(), workspace: "scratch".into() }, served)`, moving
       `served` in as the guard.

    The runner drops the fixture before its guard (`conformance/herdr.rs:122-152`), so the port goes before the server
    stops. The adapter caches no connection (Decision 2), so nothing outlives the `Served`.
34. `no_case_calls_a_method_off_the_allow_list`: every method recorded by every fake of AC 31-33 is in
    `protocol::ALLOWED_METHODS`.

**Independence of the fake and the shape of the code** (greps, run from the worktree root):

35. `grep -nE "grid_of|parse_|decode_reply|plan_splits|check_supported" crates/holler-adapter-herdr/tests/wire_herdr/*.rs`
    prints nothing. The fake reads request lines with `serde_json` and models Herdr itself.
36. `grep -n "json!" crates/holler-adapter-herdr/src/adapter.rs crates/holler-adapter-herdr/src/transport.rs` prints
    nothing. Every request is a part-1 `Request`.
37. `grep -nE "Mutex|RefCell|Cell<|OnceLock|static mut" crates/holler-adapter-herdr/src/adapter.rs` prints nothing.
    The adapter keeps no state and caches no id or version (issue scope; I6).
38. `grep -rnE "TcpStream|TcpListener|47001|47002|tmux|std::env|env::var|\.config/herdr|HERDR_|unsafe" crates/holler-adapter-herdr/`
    prints nothing. That means no environment variable is read, no default Herdr path is derived, no TCP is used and
    there is no `unsafe`.
39. `grep -n "thread::sleep" crates/holler-adapter-herdr/src/*.rs` prints nothing.

**The gates**

40. All of these pass: `cargo test -p holler-adapter-herdr`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check -p holler-adapter-herdr`,
    `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` and `cargo machete`.
41. Dependencies:
    - `cargo tree -p holler-adapter-herdr -e normal --depth 1` lists only `holler-pane` and `serde_json`.
    - The only new manifest line is the dev-dependency `tempfile = { workspace = true }`, with a comment naming its
      consumer (house style).
    - `grep -n "holler-hub" crates/holler-adapter-herdr/Cargo.toml` prints nothing.
42. Every touched file is under 900 lines (`wc -l`). No function trips `too_many_lines` (100) or
    `cognitive_complexity` (15).
43. `CHANGELOG.md` `[Unreleased]` / `### Enhancements` gets one entry, right after the part-1 entry, linking #640 and
    epic #633. Suggested text: "Herdr adapter, part 2: `holler-adapter-herdr` now implements `HerdrPort` over Herdr's
    local socket, one request per connection with one deadline per call (10 s by default; `timeout` when it runs out,
    `unavailable` for a missing socket or a garbled or oversized reply). It refuses a Herdr protocol other than 22
    when it connects. It places a pane only by the splits the planner decides, reads the tree back to confirm it
    landed in the cell asked for, and creates a configured workspace that Herdr lacks only for `r1c1`. It passes the
    `HerdrPort` conformance suite against a simulated Herdr in the default test run. Nothing is wired into a verb yet
    (#649)."

## Files (blast radius: `crates/holler-adapter-herdr/**`, plus `CHANGELOG.md`)

| File | Change | Est. lines |
|---|---|---|
| `crates/holler-adapter-herdr/Cargo.toml` | dev-dep `tempfile = { workspace = true }` with a consumer comment | +3 |
| `crates/holler-adapter-herdr/src/lib.rs` | module docs list five modules; `pub mod adapter; pub mod transport;` | ~+12 |
| `crates/holler-adapter-herdr/src/transport.rs` | new | ~230 |
| `crates/holler-adapter-herdr/src/adapter.rs` | new | ~420 |
| `crates/holler-adapter-herdr/tests/wire_herdr/mod.rs` | new (T): the simulated Herdr | ~550 |
| `crates/holler-adapter-herdr/tests/wire_herdr/serve.rs` | new (T): serves it on a temp socket | ~90 |
| `crates/holler-adapter-herdr/tests/transport_test.rs` | new (T) | ~380 |
| `crates/holler-adapter-herdr/tests/adapter_test.rs` | new (T) | ~650 (split by topic if it passes ~800) |
| `crates/holler-adapter-herdr/tests/adapter_conformance_test.rs` | new (T) | ~120 |
| `CHANGELOG.md` | one entry | +8 |
| `Cargo.lock` | only if cargo changes it (`tempfile` is already locked) | 0 |

That is about 650 production lines (two new files in one component) and about 1,800 test lines. `tests/common/mod.rs`
and the part-1 sources are **not** changed.

**Not touched:**

- `holler-pane/**` (frozen; its doc edits are part 3);
- `holler-pane-testkit/**` (#638's; `FakeHerdr` is not changed);
- `holler-cli/src/pane/wiring.rs` (#649);
- `docs/adr/ADR-0021.md` (part 3);
- every other crate.

## Reuse map (extend, do not duplicate)

| Need | Existing object | Recommendation |
|---|---|---|
| Requests, reply decoding, the error mapping, the version gate, the parsers | `protocol::{Request, decode_reply, parse_pong, check_supported, parse_snapshot, parse_layout_export, parse_workspace_created, parse_pane_info, parse_read, expect_ok, SessionState::workspace}` | **Use**. No second encoder or decoder. `pane_not_found` reaches the adapter already as `PaneNotFound { what: <pane id> }`. |
| Tree to `GridPos` | `layout::grid_of`, `GridMap::{at, position_of, cells}` | **Use**, for the existing map, the read-back confirm and `snapshot`. Nothing else in `src/` reads a tree. |
| Which split to make | `plan::plan_splits`, `Target`, `Extent`, `Step` | **Use** with a one-cell `Target`. Do not re-check range or reachability in the adapter. |
| The port and its types | `holler_pane::{HerdrPort, HerdrSpec, HerdrPane, HerdrSnapshot, Key, PaneId, GridPos, PaneError}` | **Use**. No parallel error type. |
| A JSON-line Unix-socket client | `holler_hub::control::send_over` (`control.rs:485-518`) | **Pattern only**: no dependency on `holler-hub` (ADR-0021 §5). Close its three gaps (see Evidence). |
| A port fake to mirror | `holler_pane_testkit::herdr::FakeHerdr` (unavailable wording, base-36 ids, `last_lines`) | The wire fake **mirrors its vocabulary**, at the JSON level. `base36` is private to the test kit, so the wire fake re-implements those 10 lines (justified duplication in test code). Use the test kit's `PROTOCOL_22_VERSION` and `UNSUPPORTED_VERSION` constants. |
| The conformance suite | `holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture}` | **Run it** (AC 31-33). Do not copy its cases. |
| Temp dirs | `tempfile` (workspace dep) | **Use** as a dev-dependency. No hand-made temp paths. |

## Decisions already made (MO)

1. **Part 2 of 3.** This brief replaces the part-1 brief at `docs/handoffs/640-brief.md`. The PR says `Part of #640`.
2. **One request per connection; no half-close.** For every exchange, the transport:
   - connects to the configured path;
   - writes `request.to_line()` with `write_all`;
   - reads to the first `\n`, or to EOF;
   - closes.

   It never calls `shutdown(Write)`: the spike's client did not, and whether Herdr answers after a half-close is
   unverified. Bytes after the first `\n` are ignored. EOF with some bytes and no newline returns those bytes (AC 9),
   and `decode_reply` judges them. EOF with none is `unavailable`. A reply that passes `MAX_REPLY_BYTES` (16 MiB)
   before a newline is `unavailable`, and the transport stops reading. That covers the biggest legitimate reply, a
   `pane.read` of many lines, and still bounds memory.
3. **One deadline per `HerdrPort` call, and it bounds everything.**
   - Each port method computes `deadline = Instant::now() + config.timeout` once, on entry, and passes it to every
     exchange of that call (AC 28). This is the "per-method deadline": I5 bounds the port call, not each socket round
     trip. A call never takes a fresh deadline part-way.
   - `UnixSocketTransport::exchange` runs the connect, write and read on a short-lived worker thread. The worker sets
     the socket's read and write timeouts to the time left before each syscall and retries `Interrupted`.
   - The caller waits with `mpsc::Receiver::recv_timeout(time left)`. So the call returns by the deadline even when
     `connect` blocks (the full listen backlog of a wedged server, which std cannot bound) or the server drips bytes
     (AC 6). Expiry is `Timeout { op: format!("herdr.{}", request.method()) }`.
   - **"Time left"** (clarification, amender round 2, review NV-2; it restates the two bullets above and adds no rule):
     both the worker and the caller compute it the same way, `deadline.saturating_duration_since(Instant::now())`, at
     the moment of each wait, from the one `deadline` passed to `exchange`. The worker computes it before each socket
     syscall and the caller before `recv_timeout`. Neither adds a margin or subtracts an epsilon, and neither keeps a
     timeout of its own.
   - **One answer per call** (clarification, amender round 1; it adds a rule and changes none above; AC 12a):
     - The worker sends at most one result on the channel and then ends. When that send fails because the caller has
       stopped waiting (the receiver is dropped), the worker drops the result silently: no panic, no retry, no log.
     - The caller's answer is what `recv_timeout` gives it. A result received before expiry is returned as it is,
       whatever it is. `RecvTimeoutError::Timeout` is `Timeout { op }`. `RecvTimeoutError::Disconnected` (the worker
       ended without sending) is `unavailable`.
     - The worker's socket timeouts come from the same deadline, and its `WouldBlock`/`TimedOut` is the same
       `Timeout { op }` (Decision 4). So a server that stays silent or drips gives `Timeout` whichever expiry fires
       first: the worker's and the caller's cannot give different variants for one wire condition. When the time left
       before a syscall is zero, the worker reports `Timeout` without making the syscall (std refuses a zero socket
       timeout).
     - Different answers come only from different wire conditions: for example, a server that closes just as the
       deadline passes is `unavailable` if the close is seen first and `timeout` otherwise.
   - A deadline already passed returns `timeout` without connecting (AC 11).
   - A worker abandoned on a timeout ends by itself once its socket timeouts fire. The one exception is a connect that
     stays blocked, which ends when the server accepts or dies.
   - Failing to spawn the worker is `unavailable`.
4. **Transport error mapping.**
   - A connect failure is `Unavailable`, naming the socket path and the `io::ErrorKind`. That covers a missing path, a
     path that is not a socket, a refused connection, a path too long for `sockaddr_un` (about 104 bytes on macOS,
     108 on Linux) and a permission error.
   - A write failure, a reply that is not UTF-8, an empty EOF and an oversized reply are `Unavailable`.
   - `WouldBlock` or `TimedOut` from a socket timeout is `Timeout`, with the same `op` as above.
   - No message carries the request line or any reply bytes (AC 12). Typed text never reaches an error.
5. **The socket path is configuration only.**
   - `HerdrConfig.socket` must be absolute (a relative path is `usage`).
   - The adapter derives no default path and reads no environment variable (no `HOME`, `XDG_*` or `HERDR_*`, AC 38).
   - So nothing in this crate can find the operator's live Herdr by itself. #649 supplies the path from configuration
     or from `herdr session list` (spike section 12, item 4).
6. **`HerdrConfig` and its validation.**
   - The session's extents are configuration: a label maps to an `Extent`, which answers the test kit's "how the real
     adapter learns a workspace's size" (part-1 Decision 4).
   - `connect`/`connect_with` refuse with `usage`, before any request (AC 27): an empty `session`, a relative `socket`,
     a zero `timeout`, or an `Extent` with a zero dimension.
   - An empty `workspaces` map is allowed (every `ensure_pane` is then `unavailable`).
   - **Order and message** (clarification, amender round 1; the rules above are unchanged; AC 27). The checks run in
     this order and stop at the first that fails, so one call gives one `Usage`: `session`, then `socket`, then
     `timeout`, then every entry of `workspaces` in the map's own order (a `BTreeMap`, so by label). Within an `Extent`,
     `rows` is checked before `cols`. `PaneError::Usage.message` is one line and names what was refused:
     - an empty session: the field name `session`;
     - a relative socket: the field name `socket` and the path's display form;
     - a zero timeout: the field name `timeout`;
     - a zero dimension: the workspace's label and the field name `rows` or `cols`, whichever is zero (`rows` when both
       are).

     The full wording is F's. The tests assert only these substrings.
7. **`connect` pings and gates.** It validates the config, then runs `ping`, `parse_pong` and `check_supported` under
   one deadline. A good server gives a `HerdrAdapter` holding only `config` and `transport` (AC 37). The adapter caches
   no version and no pane id.
8. **`version()` pings and gates again** and returns the version string. A version string that is empty or holds a
   control character is `unavailable`, so the conformance case "a non-empty string on one line" can only fail for a
   real reason. The other methods do **not** re-check the version (AC 26), which answers the test kit's
   `herdr.rs:318-320` assumption for this adapter: the gate is at connect time and at `version()`.
   Where the string comes from (clarification, amender round 2, review NV-4; Decision 7 already says nothing is
   cached): `connect`'s `ServerVersion` is used for `check_supported` only and then dropped. `version()` sends its
   own `Request::Ping` under its own deadline, runs `decode_reply`, `parse_pong` and `check_supported` on that reply,
   and returns that reply's `version` field. AC 26's first bullet holds because the wire fake answers every `ping`
   from its current `set_protocol` setting (Decision 15), so two pings with no `set_protocol` between them report the
   same version. AC 26's third bullet relies on the same rule in the other direction.
9. **One adapter serves one session.** A `spec.session` that is not `config.session` is `unavailable`, and no request
   is sent. Every `HerdrPane` the adapter returns carries `config.session`.
10. **Workspaces, by label.** (Part-1 Decision 11, built on `SessionState::workspace`.)
    - A `spec.workspace` with no entry in `config.workspaces` is `unavailable`, and no request is sent. That answers
      `herdr.rs:358-360` the same way the fake does.
    - A configured label that Herdr lacks is an empty `GridMap`. The workspace is created by `workspace.create {label,
      focus: false}` only when the plan is `[Step::CreateRoot]`, which is only for `r1c1` (AC 20).
    - A configured label that Herdr has twice is `unavailable` (the merged rule).
    - A workspace that has no tab (`grid_tab: None`) is `unavailable`.
11. **`ensure_pane` is plan, act, observe.**
    1. Check the session, the configured workspace and the deadline.
    2. Read the existing map: `session.snapshot`, then `SessionState::workspace(label)`. When the workspace exists,
       `layout.export {tab_id: grid_tab}`, `parse_layout_export` and `grid_of`; otherwise `GridMap::default()`.
    3. `plan_splits(&map, &Target { extent, cells: vec![spec.grid] })`. Its errors (`grid-out-of-range` first, then
       `grid-unreachable`) are returned unchanged.
    4. An empty plan means the cell is occupied. Return `map.at(spec.grid)` as the `HerdrPane`, with no mutating
       request (AC 18).
    5. A plan of one step executes it:
       - `CreateRoot` sends `workspace.create`, which gives the new grid tab and its root pane through
         `parse_workspace_created`.
       - `Split { from, direction, ratio, .. }` sends `pane.split { target: map.at(from), direction, ratio }`, and
         `parse_pane_info` gives the new id. A `PaneNotFound` from this split becomes `unavailable` ("the layout
         changed while the pane was placed"; AC 17). The caller asked for a cell, not for that pane. (Clarification,
         amender round 2, review NV-5: `decode_reply` of the `pane.split` reply already gives
         `PaneError::PaneNotFound { what }`, so the adapter matches that one variant on that one result and returns
         `Unavailable` in its place. Every other error from the split passes through unchanged, and a `PaneNotFound`
         from `send_text`, `send_keys`, `read` or `close` is never rewritten, Decision 13.)
       - A plan of more than one step cannot come from a one-cell target (Evidence, `plan.rs`). If it ever does, it is
         `unavailable` and no step runs.
    6. Observe: `layout.export` of the grid tab again, then `grid_of`. `position_of(new id)` must be `spec.grid`.
       Otherwise the result is `unavailable`, naming the new id and where it landed, or saying it has no position. The
       pane is **left where Herdr put it**: the adapter never closes or moves any pane, the misplaced one included
       (AC 16). Healing it is reconcile's job (#647).
    7. Return `HerdrPane { session: config.session, workspace: spec.workspace, pane_id: new id, grid: spec.grid }`.
12. **`snapshot` lists every workspace's grid tab.** It sends one `session.snapshot`. Then, for each workspace in
    Herdr's order that has a grid tab, it sends `layout.export` and runs `grid_of`, and `cells()` (row, then column)
    gives one `HerdrPane { session: config.session, workspace: label, pane_id, grid }` per placed pane.
    - **Every** workspace is listed, configured or not (AC 22), since the port says "every pane Herdr has". Import
      (#650) and doctor (#647) need to see panes Holler did not make.
    - Duplicate labels do not fail the snapshot. Pane ids are unique in a session (`w<N>:p<M>`), so a listing stays
      unambiguous. Only `ensure_pane` needs a unique label.
    - Unplaced panes and the panes of other tabs are left out. `HerdrPane.grid` is required, so there is nowhere to put
      them (part-1 "For the operator" item 2).
13. **The single-request methods.**
    - `send_text`, `send_keys` and `close` each send one request, checked with `expect_ok`. Keys go out verbatim
      (part-1 Decision 9; AC 24).
    - `read` sends `pane.read` with `source: "recent"`, `format: "text"` and `lines = max_lines` clamped to
      `1..=u32::MAX`, then `parse_read(result, max_lines)` trims the reply (AC 25). The clamp exists because what Herdr
      does with `lines: 0` is unverified. `read(p, 0)` still makes the call, so a closed pane is still
      `pane-not-found`. (Clarification, amender round 2, review NV-6: there are two values. `Request::Read.lines` is
      the clamped `u32`, `max(max_lines, 1)` saturated at `u32::MAX`. `parse_read` gets the caller's own `max_lines`,
      unclamped. So `read(p, 0)` sends `"lines":1` and `parse_read(result, 0)` gives `""` (`protocol.rs:589-595`).)
    - `pane_not_found` arrives through `decode_reply` as `PaneNotFound { what: <the pane id> }`.
14. **The transport returns the reply line, not a decoded value.** This changes the part-1 brief's outline, which had
    `call(...) -> Result<Value, _>`. It puts `Request::to_line` and `decode_reply` on every tested path, including the
    in-process one. The wire fake sees only `request.to_line()` (AC 35).
15. **The wire fake is Herdr at the JSON level** (test code, written by T, re-implemented and never derived from
    `src/`). Its rules:
    - **Requests.** It answers one line per request and echoes the request's `id`. A line that is not JSON gives
      `{"id":"","error":{"code":"invalid_request","message":"invalid request"}}`. An unknown method gives
      `invalid_request` with "unknown variant". It records every request it receives.
    - **`ping`.** It answers `{"type":"pong","version":<PROTOCOL_22_VERSION | UNSUPPORTED_VERSION>,"protocol":<22 | 99>,"capabilities":{}}`,
      with `protocol` left out when it is set to `None`.
    - **Ids.** Workspaces are `w<N>`, counted from 1 and never reused. Tabs are `w<N>:t<M>`, with `number` 1, 2, and so
      on. Panes are `w<N>:p<base36 counter>`: the counter is per workspace, only goes up, and uses upper case
      (`w1:p9`, then `w1:pA`).
    - **`workspace.create`.** It gives `workspace_created` with `workspace` (`workspace_id`, `number`, `label`,
      `focused`, `pane_count`, `tab_count`, `active_tab_id`, `agent_status`), `tab` and `root_pane`.
    - **`session.snapshot`.** It gives `session_snapshot` with `version`, `protocol`, `workspaces`, `tabs`, `panes`
      (each with `pane_id`, `workspace_id` and `tab_id`), `layouts: []` and `agents: []`.
    - **`layout.export {tab_id}`.** It gives `layout_export` with `layout: {workspace_id, tab_id, zoomed: false,
      focused_pane_id, root}`. Its nodes are exactly `{"type":"pane","pane_id":..}` and
      `{"type":"split","direction":..,"ratio":..,"first":..,"second":..}`. An unknown tab is the error code
      `tab_not_found` (the fake's choice; the adapter maps it to `unavailable` either way).
    - **`pane.split`.** It replaces the target's leaf with `split(direction, ratio, target, new)` (the nesting rule: only
      the target's cell changes). An unknown target is `pane_not_found` with the message `pane <id> not found`. A
      direction other than `right` or `down`, or a ratio outside `(0, 1)`, is `invalid_request`.
    - **`pane.close`.** It replaces the parent split with the sibling subtree (the sibling takes the space). Closing a
      tab's last pane closes the tab, and closing a workspace's last tab closes the workspace (INFERRED; no case relies
      on it). An unknown pane is `pane_not_found`.
    - **The screen.** `send_text` appends the text to the pane's screen. In `send_keys`, `enter` appends `\n` and every
      other key appends nothing. `pane.read` gives `pane_read` with the last `lines` lines of the screen. A `source`
      outside the four names is `invalid_request`.
    - **Unknown panes** (clarification, amender round 2, review NV-7; conformance case 9 needs it, Evidence
      `conformance/herdr.rs:280-324`): `pane.send_text`, `pane.send_keys` and `pane.read` of a pane id the fake does not
      hold (never handed out, or closed) answer `pane_not_found` with the message `pane <id> not found`, as
      `pane.close` and `pane.split` do. The spike verified this code for `pane.close` only (spike table, "Close"); for
      the other three it is INFERRED, and part 3's scratch test is the real check.
    - **Forbidden methods.** Any method on the spike's forbidden list is recorded and answered `invalid_request`, and
      AC 34 catches it.
    - **Serving.** `serve()` accepts one connection at a time on `<tempdir>/h.sock`, answers one line, and closes the
      connection. Its `Drop` sets a stop flag, connects once to wake `accept`, joins the thread and removes the
      directory.
16. **Three conformance fixtures** (AC 31-33): an empty Herdr, a workspace that holds only its root pane (the two
    starting states the suite allows), and the real socket transport. So the suite proves the adapter, the transport
    and the fake together.
17. **RED for a new API** (the part-1 pattern; the tester overlay says a compile error is not RED).
    - T adds `src/transport.rs` and `src/adapter.rs` holding **only** the pinned items:
      - `exchange`, `connect` and `connect_with` return `Err(PaneError::NotImplemented)`;
      - every `HerdrPort` method returns `Err(PaneError::NotImplemented)`;
      - `HerdrConfig::new`/`with_workspace`/`config`/`UnixSocketTransport::new`/`socket` are plain constructors and
        accessors, which T may write in full.
    - Each stub body is marked `// stub (#640 part 2): F fills`.
    - T writes the wire fake and every test.
    - F replaces the stub bodies and writes no tests.
18. **Dependencies.** The only addition is the dev-dependency `tempfile`. It is already in the workspace and is used
    by three crates for exactly this (scratch directories). The worker thread and channel come from `std`. There is no
    new normal dependency, no `serde`, no async runtime and no `libc`/`unsafe` (a bounded `connect` through
    `SO_SNDTIMEO` would need both, which is why Decision 3 uses a thread).
19. **No lock in the adapter.** Two concurrent `ensure_pane` calls on one workspace can both plan from the same tree.
    The observe step then reports `unavailable` for a pane that lands elsewhere. Preventing the race belongs to the
    verbs, which serialize through the registry's compare-and-swap. A lock here would be a wait outside the deadline,
    and it would not stop another process.
20. **The test-kit divergence is mentioned, not filed** (MO). `FakeHerdr`'s split-only mode accepts a `down` split
    under any pane (Evidence, `herdr.rs:433-445`), so `r2c2` below `r1c2` passes there, while real Herdr nests it and
    this adapter refuses it (`grid-unreachable`). A verb test written against `FakeHerdr` can therefore pass on a layout
    the adapter refuses. Filing a test-kit follow-up waits for the operator's word.

## Forward-compat

| Consumer | Needs from part 2 | Satisfied |
|---|---|---|
| Part 3 (scratch test) | `HerdrAdapter::connect(HerdrConfig)` with a scratch socket path, and `run_herdr_conformance` over it with the server handle as the guard | yes (AC 33 is the same shape) |
| #649 wiring | `HerdrConfig` (session, socket, per-label `Extent`, timeout) and `connect` | yes. **Note:** `connect` needs a live server, so #649 decides whether to connect lazily for verbs that do not touch Herdr |
| #644 launch/relaunch | `version()` to record `host.herdr_api_version`; `grid-out-of-range`/`grid-unreachable` as refusals; `ensure_pane` bounded by I5 | yes |
| #647 doctor, #650 import | `snapshot` of every workspace, positions from the tree | yes, except unplaced panes (operator item 2) |
| #664 `profile apply` | one cell per `ensure_pane`, in the rows-first order that apply's plan takes from `plan_splits` | yes |

## Out of scope

- Part 3: the opt-in scratch-Herdr test, the `holler-pane` doc edits (`Key`, `ensure_pane`, `GridOutOfRange`) and the
  ADR-0021 rows.
- Wiring (#649), recording `host.herdr_api_version` (#644/#647), `events.subscribe`, and an argv launch without a shell
  (`layout.apply` into a staging tab plus `pane.move`), which needs a contract amendment first.
- Any change to `FakeHerdr`, `holler-pane`, part 1's modules or `tests/common/mod.rs`.
- Discovering the socket path, reading `herdr session list`, and checking who owns the socket.

## Test plan

**RED.** T writes the stubs of Decision 17, the wire fake and every test file. Then
`cargo test -p holler-adapter-herdr` must compile and run. Part-1 tests stay green. Every new AC test fails on an
assertion:

- an `expect` on `connect_with` or `exchange` that meets `NotImplemented`;
- a conformance result of `Err([...11 failures])`;
- an expected `Usage` or `Timeout` that gets `NotImplemented`.

Some "no request was recorded" halves pass at RED, because nothing runs. T lists each one, and each test still fails on
its error-code half. T's handoff lists every test with its AC and its failing assertion. The grep ACs 35-39 pass at RED
by construction, and T says so.

**GREEN.** F fills the stubs. All of AC 1-43 (AC 12a included) pass. T records the mutants of AC 14 in `handoff-T-green.md`. Timing tests
assert an upper bound only: deadline plus 2s, for slow macOS CI runners. No test waits on a fixed sleep for readiness,
and the drip server's pauses (AC 6) simulate a slow peer.

## Risks

- **Never touching the live Herdr.** Production code has no default path and reads no environment variable (AC 38).
  Every test socket is inside a `tempfile::tempdir()`, and the in-process fake opens none. The session name in tests is
  `scratch`, and no test runs `herdr`. No test opens TCP, so the OpenCode ports and tmux are out of reach.
- **Socket path length.** `sockaddr_un` limits paths to about 104 bytes on macOS and 108 on Linux. A macOS temp dir
  plus `/h.sock` stays under that. A path that is too long is `unavailable`, not a panic (AC 4).
- **The deadline.**
  - `SO_RCVTIMEO` alone bounds a single read. The worker thread and `recv_timeout` bound the whole exchange (AC 5-6).
  - An abandoned worker can outlive the call by up to one socket timeout, or longer if it is stuck in `connect`. That
    costs a thread, not a wrong answer.
  - The bounded `connect` itself has no test. Filling a listen backlog behaves differently per OS, so it is covered by
    design only.
- **A timeout on a mutating request leaves its outcome unknown.** The split may still happen after `timeout` is
  returned. The adapter never retries. The next `ensure_pane` re-reads the tree and answers the occupant, or the
  verb's reconcile finds it.
- **Partial and oversized replies.**
  - The transport reads raw chunks into a buffer capped at 16 MiB, so a `read_line` with no newline cannot grow
    without limit (AC 8).
  - A truncated reply at EOF is passed to `decode_reply`, which turns invalid JSON into `unavailable` (AC 9).
  - Partial writes are handled by `write_all`.
- **The fake's fidelity.** Where the wire fake models a behaviour the spike did not verify (last-pane close, the
  `tab_not_found` code, the ratio range), it says INFERRED, and no AC depends on it. Part 3's scratch test is the
  real-server check. One exception (amender round 2, review NV-7): `pane_not_found` for `send_text`, `send_keys` and
  `read` of a closed pane is INFERRED, and AC 31-33 depend on it through conformance case 9. It is what the port
  requires of every implementation, and part 3 checks it against real Herdr.
- **The adapter is stricter than `FakeHerdr`** (Decision 20). The adapter follows the spike, and the test kit is not
  changed here.
- **File size and complexity.** `adapter_test.rs` is the largest file, so T splits it by topic past about 800 lines.
  `ensure_pane` is split into helpers (read the map, run a step, confirm) to stay under clippy's 100-line and
  complexity-15 limits.

## Contradictions found (merged part 1 against the part-1 brief's outline)

1. **No flat re-exports.** The old brief said `lib.rs` would carry "re-exports of the names below". Merged `lib.rs:32-35`
   re-exports nothing, on purpose (name clashes with `holler_proto`). This brief follows the merged code: module paths
   only.
2. **No `serde` dependency.** The old brief's file table listed "deps `holler-pane`, `serde` (derive), `serde_json`".
   Merged `Cargo.toml:14-20` has only `holler-pane` and `serde_json`. This part adds no `serde`, and `HerdrConfig` is
   not `Deserialize`: #649 builds it.
3. **`Request` has no `PartialEq`** (`protocol.rs:110-112`: `#[derive(Clone)]` plus a hand-written `Debug` that hides
   `SendText`'s text). So tests compare `to_line()` output or the fake's parsed JSON, never `Request` values.
4. **The transport's shape changes from the old outline**, from `call(...) -> Result<Value, _>` to
   `exchange(...) -> Result<String, _>` (Decision 14). The old outline was not merged code. The change only moves
   `decode_reply` into the adapter.
5. **Snapshot scope.** The old outline said "`snapshot`: placed panes of every workspace". This brief keeps that
   (Decision 12) and rules out narrowing it to configured workspaces.
6. **Brief path and rigor.** The old brief said parts 2 and 3 would use `docs/handoffs/640-2-brief.md` and `640-3-brief.md`
   and that every #633 story runs in-session. Per the MO, this part uses `docs/handoffs/640-brief.md` and runs at
   **second-opinion**.
7. **`cargo fmt --check`.** Workspace-wide it exits 1 on pre-existing files outside this crate. The gate is therefore
   `cargo fmt --check -p holler-adapter-herdr`, which passes today (epic ruling 4).
8. **`tests/common/mod.rs`** exists (86 lines, part 1's tree builders) although the old brief's file table did not list
   it. This part leaves it alone and puts the wire fake in its own `tests/wire_herdr/` module.

No contradiction affects behaviour. Every part-1 signature the old outline relied on (`plan_splits`, `Step::CreateRoot`,
`Step::Split.from`, `SessionState::workspace`, `WorkspaceRef.grid_tab`, the parsers, `check_supported`) merged as
outlined.

## Handoff locations

T-red: `docs/handoffs/640/handoff-T-red.md`. F: `docs/handoffs/640/handoff-F.md`. T-green: `.../handoff-T-green.md`.
A: `.../handoff-A.md`, `.../handoff-A-dup.md`. S: `.../handoff-S.md`. Journal: `docs/handoffs/640/decisions.md`.

## Operating rules

- Read this brief, the Evidence and the Reuse map before writing code. A second request encoder or reply decoder, a
  second tree walk, or a parallel error type is an anti-duplication BLOCK.
- Test-first: T writes every test, the wire fake and only the stubs of Decision 17. F writes no tests.
- Never run, attach to or talk to a Herdr server, OpenCode (the operator's servers on ports 47001/47002 included), or a
  real tmux session (tmux session `O` included). This part needs none of them.
- Conventional Commits (`feat(adapter-herdr): ...`). The hooks add the trailer. Stage files by explicit path.
- The PR says `Part of #640` and carries the AI disclosure that `CONTRIBUTING.md` requires.
- Public repo: no personal hostnames, IPs, account or machine names in code, tests, fixtures or messages. Never print
  or write a credential.

## For the operator (review, not blocking)

1. **Test-kit divergence** (Decision 20): `FakeHerdr`'s split-only mode is looser than real Herdr. A small #638
   follow-up would tighten it to "a new row only below a row of one pane". It is **not filed**; say if you want it.
2. **Unplaced panes cannot appear in `HerdrSnapshot`** (`HerdrPane.grid` is required). They are left out, as are the
   panes of non-grid tabs. #647 and #650 would need a contract amendment (an `unplaced` list or an optional grid)
   before they can see them.
3. **`connect` needs a running server** (it pings). #649 decides whether verbs that never touch Herdr construct the
   adapter lazily.
4. **The issue's version line** (raised by the round-1 brief review, B-3). The issue says "an unknown one is refused
   with `herdr-version-unsupported`". Under Decision 8 the refusal comes from `connect` and `version()` only, and the
   other methods trust the gate that passed at connect (see Scope). The reviewer asks that either the issue text be
   amended to say so, or every call refuse after a failed check. Either is a change to an MO decision or to the issue,
   so the amender changed neither. Decision 8 stands unless you rule otherwise.
5. **"Each supported version ... the fake's two versions"** (raised by the round-2 brief review, W-2; not blocking).
   The issue's line can be read as "at least two supported versions". This brief reads it as the fake's two builds:
   protocol 22, which is supported, and protocol 99 (`UNSUPPORTED_VERSION`), which is refused (Scope; AC 26). The
   adapter supports exactly one protocol (`SUPPORTED_PROTOCOLS = [22]`, spike section 13). If you want the issue text
   to say so, it could be amended together with item 4. The amender changed neither the issue nor the reading.

Needs operator: item 4 (the round-1 reviewer blocks on it). Nothing else blocks part 2.
