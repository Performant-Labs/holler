# Brief: #640 herdr-adapter, part 1 of 3: the pure Herdr protocol and grid core

Repo: Performant-Labs/holler. Issue: #640 (epic #633, wave 3). Rigor: in-session (the issue's Pipeline line). UI surface: no.
Kind: feature.

**Branch:** `issue-640-implementation` (from `origin/main` at `9d61c9f`).
**Review-rigor:** in-session. Every story in epic #633 runs in-session (operator, 2026-10-08). This part is pure functions
with no I/O, so that rigor is enough.
**Forward-compat:** done, see the table below. This part creates the functions that parts 2 and 3 and the verb stories use.
**Design (Phase 3):** N/A (no UI surface).
**PR wording:** the PR says `Part of #640`, **not** `Closes #640`. The issue closes with part 3.

## Scope and the split

The whole of #640 does not fit the implementer's scope cap (about 6 files, one component family per run). It needs:

- a Herdr wire protocol (requests, replies, error mapping, the version gate);
- the GridPos conversion (a walk of the split tree);
- `plan_splits`;
- a socket transport with deadlines;
- the `HerdrPort` implementation;
- a wire-level fake Herdr for the default test run;
- the conformance run;
- an opt-in scratch-server test;
- the contract doc updates that the test kit's `ASSUMPTION (#640)` comments ask for.

That comes to about 12 production and doc files and about 3,500 lines with tests. So #640 runs as **three sequential
sub-stories on the same issue**. Each is its own pipeline run and PR, and each starts from `origin/main` after the
previous one merges:

| Part | Delivers | Files (approx.) | Issue acceptance it covers |
|---|---|---|---|
| **1 (this brief)** | Pure core, no I/O: `protocol.rs` (request builders, reply decoding, Herdr error mapping, snapshot/export/pong parsers, version gate), `layout.rs` (the split tree and the tree walk, Herdr's tree to `GridPos`), `plan.rs` (`plan_splits`, `grid-unreachable`) | 5 crate files + CHANGELOG | `plan_splits` unit tests (empty to 2x4, one more cell, unreachable refused, split-only cases); version supported and unknown refused (parser level); the transposition/off-by-one guard at the conversion level |
| 2 | I/O: `transport.rs` (one request per Unix-socket connection, per-method deadline, `timeout`/`unavailable`), `adapter.rs` (`HerdrAdapter`: `connect` reads `ping` and refuses an unknown protocol; implements `HerdrPort` on part 1); dev-test wire fake (a simulated Herdr at the JSON level: split tree, base-36 ids, nesting rule, close gives the space to the sibling, `pane_not_found`, protocol 22/99) | 4 crate files + CHANGELOG | Passes `run_herdr_conformance` (default run, wire fake); `r2c1`/`r1c2` land in the right cell and read back the same; both fake versions; timeouts |
| 3 | Opt-in scratch test (`#[ignore]`, env-gated, own `herdr --session holler640-<hex> server` under a temp HOME, refuses the default session by name, runs the conformance suite plus create/run/send/read/close/snapshot); doc-only updates to `holler-pane` (`ports.rs` HerdrPort docs: key names, `grid-out-of-range`/`grid-unreachable` from `ensure_pane`; `error.rs` `GridOutOfRange` doc) and ADR-0021 §9/§10 rows; closes #640 | 1-2 test files, 2 doc-comment edits, ADR, CHANGELOG | The scratch-session acceptance; the doc updates that `ASSUMPTION (#640)` asks for |

The MO writes the briefs for parts 2 and 3 (`docs/handoffs/640-2-brief.md`, `640-3-brief.md`) when the previous part
merges. Their outlines are at the end of this brief so A can check forward-compat now.

## Problem

`holler-adapter-herdr` is an empty skeleton. Herdr has no grid: it has a binary tree of `right`/`down` splits, and rects
whose x (the column axis) comes first and is 0-based. Holler stores `GridPos { row, col }`, which is 1-based and row first.
Epic decision 7 makes this crate the **only** place that converts between the two. Before any socket code is written,
Holler needs the conversion, the split planner and the wire protocol as pure, exhaustively testable functions. A transposed
or off-by-one conversion has to be impossible to merge.

## Evidence (verbatim, as of `9d61c9f`)

The crate today:
```
crates/holler-adapter-herdr/src/lib.rs:1-5
//! `holler_adapter_herdr` — the Herdr adapter: it implements `holler_pane::HerdrPort`
//! over Herdr's local socket, and is the only place that converts a `GridPos` to
//! Herdr's own order and base (epic #633).
//!
//! Empty skeleton (story #637); story #640 fills it.
```
```
crates/holler-adapter-herdr/Cargo.toml:12-19
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise). The crate is an empty skeleton; its owning story adds the
# dependencies it uses.
[dependencies]

# Workspace lints (issue #149).
[lints]
workspace = true
```

The port and its types (frozen by #637; provisional until #636, which has now reported):
```
crates/holler-pane/src/ports.rs:84-99
/// Where `HerdrPort::ensure_pane` should put a pane.
pub struct HerdrSpec { pub session: String, pub workspace: String, pub grid: GridPos }
/// What Herdr reports about every pane it has.
pub struct HerdrSnapshot { pub panes: Vec<HerdrPane> }
```
```
crates/holler-pane/src/ports.rs:126-147  (doc comments shortened)
pub trait HerdrPort: Send + Sync {
    /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail loudly; never relocates a healthy pane.
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError>;
    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError>;
    fn close(&self, pane: &PaneId) -> Result<(), PaneError>;
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError>;
    /// Herdr's API version. An unknown version is `herdr-version-unsupported`.
    fn version(&self) -> Result<String, PaneError>;
}
```
```
crates/holler-pane/src/pane.rs:95-100
pub struct HerdrPane { pub session: String, pub workspace: String, pub pane_id: PaneId, pub grid: GridPos }
crates/holler-pane/src/grid.rs:34-41
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]        // note: no Ord, so no BTreeSet<GridPos>
pub struct GridPos { pub row: u16, pub col: u16 }
```
```
crates/holler-pane/src/error.rs:414-416, 451-453, 331
    /// `grid-out-of-range`: a row or column of zero, or above `u16::MAX`. ...
    GridOutOfRange { what: String },
    /// `herdr-version-unsupported`: Herdr reports an API version the adapter does not
    /// know; `message` names the version and the supported ones. (#640.)
    HerdrVersionUnsupported { message: String },
    pub const fn from_static(code: &'static str) -> Self {   // RefusalCode: an open code, raised as PaneError::Refused
```

The test kit's binding vocabulary and assumptions (`ASSUMPTION (#640)`; every one is an acceptance item below or in parts 2
and 3):
```
crates/holler-pane-testkit/src/herdr.rs:26-51
// ASSUMPTION (#640): only protocol 22 was tested. ...
pub const PROTOCOL_22_VERSION: &str = "0.9.1-preview.2026-09-21-0ff0f27e2226";
// ASSUMPTION (#640): provisional test vocabulary, like `GRID_UNREACHABLE` below.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
pub const UNSUPPORTED_VERSION: &str = "99.0.0-fake";
// ASSUMPTION (#640): `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` are the fake's own
// values. ... #640
// either declares the same values in its own file and asserts in its dev-tests that they
// equal the test kit's, or the fake takes #640's values before any verb story pins them.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");
```
```
crates/holler-pane-testkit/src/herdr.rs:187-191, 272-274, 295-298, 318-320, 358-360
// ASSUMPTION (#640): Herdr has no grid (spike section 2), so the fake is told a
// workspace's size. How the real adapter learns it (...) is #640's.
// ASSUMPTION (#640): a real workspace is created with a root pane (spike section 4, ...)
// ASSUMPTION (#640): the port's doc names keys `Enter` and `C-c`, but Herdr's own
// names are `enter` and `ctrl+c` (spike section 4).
// ASSUMPTION (#640): real Herdr gives a closed pane's space to its sibling (...) the suite asserts ids, not positions
// ASSUMPTION (#640): only `version()` refuses an unsupported build. Whether
// the adapter also refuses every other call after a failed version check is #640's.
// ASSUMPTION (#640): a session or a workspace the fake does not serve is
// `unavailable`. Whether the adapter creates a missing workspace (`workspace.create`) is #640's.
```
```
crates/holler-pane-testkit/src/herdr.rs:433-455  (the fake's split-only rule)
    /// Step 4, split-only placement: a free cell is reachable as the root of an empty
    /// workspace (`r1c1`), or by a `right` split of the pane on its left or a `down`
    /// split of the pane above it.
        let root = self.panes.is_empty() && (grid.row, grid.col) == (1, 1);
        let left = holds(grid.row, grid.col.saturating_sub(1));
        let above = holds(grid.row.saturating_sub(1), grid.col);
        if root || left || above { return Ok(()); }
        Err(PaneError::Refused { code: GRID_UNREACHABLE, message: format!("{grid} cannot be reached in workspace ...") })
```
```
crates/holler-pane-testkit/src/conformance/herdr.rs:49-53, 189-196, 337-339
// ASSUMPTION (#640): Herdr has no grid (spike section 2), so the port under test must
// know that the fixture's workspace is 2 rows by 1 column, or case 2's `r1c2` is not out
// of range. How the real adapter learns a workspace's size is #640's.
// ASSUMPTION (#640): `ensure_pane` outside the workspace is `grid-out-of-range` ...
// #640 updates both [the GridOutOfRange doc and the ensure_pane doc] when it finalizes `HerdrPort`. Once #640 decides where
// a workspace's extent comes from, it also updates ADR-0021 section 9's `profile apply` row ...
// ASSUMPTION (#640): the port's doc names the key `Enter`, but the suite presses Herdr's own name, `enter`
```

What real Herdr does (`docs/research/herdr-api-spike.md`, VERIFIED unless marked):
```
§3 (lines 106-111): A Unix stream socket, owner-only ... One JSON request per line, and one request per connection ...
   An unknown method is {"error":{"code":"invalid_request","message":"invalid request: unknown variant `pane.nope`, ..."}}
§4 (line 133): Close ... again: {"error":{"code":"pane_not_found","message":"pane w1:p5 not found"}}
§6 (lines 203-218): Do not convert a GridPos to cell numbers. ... walk layout.export: the root's chain of `down` splits gives
   the rows in order, and each row's chain of `right` splits gives its columns. ... Prefer the tree walk.
   A nested split inside one cell ... The tree walk reports "not a rows-of-columns tree" for those panes ...
   The adapter should refuse to assign a GridPos there (a finding, not a guess).
§7 (lines 228-246): Splits only ... The new pane is always the `second` child ... Ratio is the first child's share ...
   A split divides only the target pane's own cell. ... to reach an R-row by C-column grid, split the first pane `down` R-1
   times with ratios 1/R, 1/(R-1), ..., 1/2 (each time on the newest pane), then each row's first pane `right` with
   1/C, 1/(C-1), ..., 1/2. ... Choose one order and keep it (the epic stores rows of columns).
§12 (lines 365, 390-391): talk to the socket directly ... The adapter must never call server.stop, server.live_handoff,
   layout.apply with a tab_id, or the integration.* and plugin.* mutators.
§13 (lines 415-416, 450-454): ping returns {"type":"pong","version":"0.9.1-preview...","protocol":22,"capabilities":{...}} ...
   support exactly protocol 22 ... Refuse any other protocol with herdr-version-unsupported, and have the message name
   "Herdr protocol 22 (0.9.1)".
```
The spike's own tree walk (`scripts/spikes/herdr-grid.sh:32-39`), which `layout.rs` reproduces:
```
def chain(node, direction):
    if node["type"] == "split" and node["direction"] == direction:
        return chain(node["first"], direction) + chain(node["second"], direction)
    return [node]
for r, row in enumerate(chain(tree, "down"), 1):
    for c, cell in enumerate(chain(row, "right"), 1):
        by_tree[cell.get("pane_id")] = (r, c) if cell["type"] == "pane" else None
```
The 2x4 build that the spike verified (`herdr-grid.sh:55-57`): `R2C1=split R1C1 down 0.5`; `R1C2=split R1C1 right 0.25`;
`R1C3=split R1C2 right 0.3333`; `R1C4=split R1C3 right 0.5`; the same for row 2. The marker placed at row 2, column 3 read
back as tree `r2c3` and rect `{x:60,y:20}` (spike §6 lines 180-195).

**SCHEMA** (the bundled schema read offline with `herdr api schema --output <scratch file>`, which starts no server and
connects to none, protocol 22, schema_version 1). These are the exact field names the parsers rely on:
```
error_response:  {"id": string, "error": {"code": string, "message": string}}          (required: id, error)
result "pong":   {type:"pong", version:string, protocol:uint32, capabilities?}         (required: type, version, protocol)
result "session_snapshot": {type, snapshot: SessionSnapshot}
  SessionSnapshot required: version, protocol, workspaces[], tabs[], panes[], layouts[], agents[]
  WorkspaceInfo required: workspace_id, number, label (string), focused, pane_count, tab_count, active_tab_id, agent_status
  TabInfo required: tab_id, workspace_id, number, label, focused, pane_count, agent_status
  PaneInfo: pane_id, workspace_id, tab_id, ... (absent optional fields are omitted, spike §4)
result "layout_export": {type, layout: {workspace_id, tab_id, zoomed, focused_pane_id, root: LayoutNode}}
  LayoutNode = {"type":"pane", pane_id: string|null, label?, cwd?, command?, env?}
             | {"type":"split", direction: "right"|"down", ratio: float, first: LayoutNode, second: LayoutNode}
result "workspace_created": {type, workspace: WorkspaceInfo, tab: TabInfo, root_pane: PaneInfo}
result "pane_info": {type, pane: PaneInfo}                                            (pane.split's result)
result "pane_read": {type, read: {pane_id, workspace_id, tab_id, source, format, text, revision, truncated}}
result "ok":   {type:"ok"}
params: ping {}; session.snapshot {}; layout.export {tab_id?|pane_id?}; workspace.create {cwd?, env?, focus=false, label?};
  pane.split {target_pane_id?, direction (required), ratio?: float, focus=false, cwd?, env?};
  pane.send_text {pane_id, text}; pane.send_keys {pane_id, keys: [string]}; pane.close {pane_id};
  pane.read {pane_id, source: visible|recent|recent_unwrapped|detection (required), lines?: uint32, format?=text, strip_ansi?=true}
```

## The API this part creates (pinned; T writes against it, F fills it)

`crates/holler-adapter-herdr/src/lib.rs`: module docs (the conversion rule and the split model in prose), plus
`pub mod layout; pub mod plan; pub mod protocol;` and re-exports of the names below.

**`src/layout.rs`**, Herdr's split tree and the one conversion to `GridPos`:
```rust
pub enum Direction { Right, Down }                       // Herdr's only two; as_str() -> "right" | "down"
pub enum LayoutNode {
    Pane { pane_id: PaneId },
    Split { direction: Direction, ratio: f64, first: Box<LayoutNode>, second: Box<LayoutNode> },
}
pub struct GridMap { /* rows of slots; a slot holds a pane or a nested (non-grid) subtree */ }
impl GridMap {
    pub fn at(&self, cell: GridPos) -> Option<&PaneId>;          // None for a free cell or a nested slot
    pub fn position_of(&self, pane: &PaneId) -> Option<GridPos>;
    pub fn cells(&self) -> Vec<(GridPos, PaneId)>;               // placed panes, by row then column
    pub fn unplaced(&self) -> &[PaneId];                         // panes inside a nested slot, tree order
    pub fn rows(&self) -> u16;                                   // slots count, nested ones included
    pub fn cols_in(&self, row: u16) -> u16;                      // 0 for a row that does not exist
    pub fn is_empty(&self) -> bool;
}
impl Default for GridMap                                         // an empty workspace (no tree)
pub fn grid_of(root: &LayoutNode) -> GridMap;
```
**`src/plan.rs`**, the split planner:
```rust
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");
pub struct Extent { pub rows: u16, pub cols: u16 }              // Copy, Eq
pub struct Target { pub extent: Extent, pub cells: Vec<GridPos> }
pub enum Step {
    CreateRoot,                                                  // workspace.create: its root pane is r1c1
    Split { from: GridPos, direction: Direction, ratio: f64, creates: GridPos },
}
pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError>;
```
**`src/protocol.rs`**, Herdr's wire, with no I/O:
```rust
pub const SUPPORTED_PROTOCOLS: [u32; 1] = [22];
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
pub const ALLOWED_METHODS: [&str; 9] = ["ping", "session.snapshot", "layout.export", "workspace.create",
    "pane.split", "pane.send_text", "pane.send_keys", "pane.read", "pane.close"];
pub enum Request {
    Ping, SessionSnapshot, LayoutExport { tab_id: String }, WorkspaceCreate { label: String },
    Split { target: PaneId, direction: Direction, ratio: f64 },
    SendText { pane: PaneId, text: String }, SendKeys { pane: PaneId, keys: Vec<Key> },
    Read { pane: PaneId, lines: u32 }, Close { pane: PaneId },
}
impl Request {
    pub fn method(&self) -> &'static str;
    pub fn id(&self) -> String;                 // "holler:<method>"
    pub fn to_line(&self) -> String;            // one JSON object and exactly one trailing "\n"
}
pub fn decode_reply(request: &Request, line: &str) -> Result<serde_json::Value, PaneError>;  // the `result` object
pub struct ServerVersion { pub version: String, pub protocol: Option<u32> }
pub fn parse_pong(result: &Value) -> Result<ServerVersion, PaneError>;
pub fn check_supported(server: &ServerVersion) -> Result<(), PaneError>;
pub struct WorkspaceRef { pub workspace_id: String, pub label: String, pub grid_tab: Option<String> }
pub struct PaneRef { pub pane_id: PaneId, pub workspace_id: String, pub tab_id: String }
pub struct SessionState { pub workspaces: Vec<WorkspaceRef>, pub panes: Vec<PaneRef> }
impl SessionState { pub fn workspace(&self, label: &str) -> Result<Option<&WorkspaceRef>, PaneError>; }
pub fn parse_snapshot(result: &Value) -> Result<SessionState, PaneError>;
pub fn parse_layout_export(result: &Value) -> Result<LayoutNode, PaneError>;
pub fn parse_workspace_created(result: &Value) -> Result<(WorkspaceRef, PaneId), PaneError>;
pub fn parse_pane_info(result: &Value) -> Result<PaneId, PaneError>;
pub fn parse_read(result: &Value, max_lines: usize) -> Result<String, PaneError>;
pub fn expect_ok(result: &Value) -> Result<(), PaneError>;
```
F may add private helpers and derive traits (`Debug`, `Clone`, `PartialEq`; `Eq`/`Copy` where the fields allow). F may not
rename or drop any item above. If F finds a signature unworkable, F stops and reports. F does not silently change it.

## Acceptance criteria (part 1)

Each is a named test in `crates/holler-adapter-herdr/tests/`. `cargo test -p holler-adapter-herdr` runs them all by default.
None needs Herdr, a socket or the network.

**The conversion (`layout_test.rs`)**
1. `grid_of` on the spike's 2x4 tree (root `down` 0.5 over two `right` chains of four built with 0.25, 0.3333, 0.5; ids
   `w1:p1 w1:p3 w1:p4 w1:p5` in row 1 and `w1:p2 w1:p6 w1:p7 w1:p8` in row 2, as in spike §6) puts `w1:p7` at
   `GridPos { row: 2, col: 3 }`. `cells()` lists all eight row-first, and `.to_string()` of `w1:p7`'s cell is `"r2c3"`.
2. **Transposition and off-by-one guard.** In the same tree, `position_of(w1:p2) == r2c1` and `position_of(w1:p3) == r1c2`.
   `at(r1c2) == Some(w1:p3)`, `at(r2c1) == Some(w1:p2)`, `at(r3c1) == None` and `at(r1c5) == None`. No placed cell has a
   0 row or column. In GREEN, T records in its handoff that two mutants each fail at least one of AC 1 and 2: `grid_of`
   with `down` and `right` swapped, and `grid_of` counting from 0. The mutants are local and not committed.
3. A chain nested on either side flattens in order: `split(down, A, split(down, B, C))` and `split(down, split(down, A, B), C)`
   both give A r1c1, B r2c1, C r3c1. The same holds for `right` chains within a row.
4. Unaligned rows (row 1 split 0.5, row 2 split 0.3, spike §6) give `r1c2` and `r2c2` by the tree. Ratios never affect
   positions.
5. A nested slot: the 2x4 tree with `w1:p3` (r1c2) replaced by `split(down, w1:p3, w1:p9)`. `unplaced()` is
   `[w1:p3, w1:p9]`, `at(r1c2)` is `None`, `w1:p4` stays at `r1c3` (later columns keep their slot numbers, as the spike's
   walk does), and row 2 is unchanged.
6. Columns first: `split(right, split(down, A, C), split(down, B, D))` places nothing, and all four ids are unplaced.
   `GridMap::default()` is empty, with `rows() == 0`.

**The planner (`plan_splits_test.rs`).** Ratios are compared with a tolerance of 1e-9.

7. **Empty to 2x4**: an empty `GridMap`, `Extent { rows: 2, cols: 4 }` and all eight cells give exactly
   `[CreateRoot, Split{r1c1,Down,1/2,r2c1}, Split{r1c1,Right,1/4,r1c2}, Split{r1c2,Right,1/3,r1c3}, Split{r1c3,Right,1/2,r1c4},
   Split{r2c1,Right,1/4,r2c2}, Split{r2c2,Right,1/3,r2c3}, Split{r2c3,Right,1/2,r2c4}]`. That is the spike's verified
   recipe: new rows first, ascending, each a `down` split of the row above's first pane with ratio `1/(rows - r + 2)`. Then
   each row's new columns, ascending, each a `right` split of the cell on its left with ratio `1/(cols - c + 2)`. The input
   order of `cells` does not change the output.
8. **One more cell**: existing r1c1 and r2c1 (2x1), extent 2x2, target = existing + r1c2 gives
   `[Split{r1c1,Right,1/2,r1c2}]`. Existing r1c1 only, target {r1c2, r2c1}, extent 2x2 gives `[Down to r2c1, Right to r1c2]`:
   rows before columns.
9. **Idempotent**: a target whose every cell exists gives `Ok(vec![])`. That holds even when the map has unplaced panes.
   Duplicate cells in `target.cells` are ignored.
10. **Range first, `grid-out-of-range`**: extent 2x1 with r1c2, r3c1, `GridPos{row:0,col:1}` or `GridPos{row:1,col:0}` in
    the target is `PaneError::GridOutOfRange`. The `what` names the cell (`r1c2`) and the extent ("2 rows by 1 column").
    The range check runs before every other check, so a target that is both out of range and unreachable is out of range.
11. **Unreachable, `grid-unreachable`** (`PaneError::Refused` with `code == GRID_UNREACHABLE`,
    `class_of(code) == ErrorClass::Refusal`, the message names the cell, one line). Each of these cases returns that error
    and no partial plan:
    - (a) an empty map and a target without r1c1 (`r1c2` or `r2c1`): an empty workspace starts at r1c1;
    - (b) a gap: existing r1c1, target r1c3 in extent 1x3 with no r1c2, or r3c1 with no r2c1;
    - (c) a new row below a row that has more than one column: existing r1c1 and r1c2, target r2c1 or r2c2 in extent 2x2.
      Herdr's nesting rule would make it a non-grid, so the message says a row can be added only below a row of one pane;
    - (d) a new cell while the map has any unplaced pane.
12. **The split-only cases from #638** that hold for real Herdr: an empty workspace accepts only r1c1 (11a); an occupant is
    answered without a split (9); the range is checked first (10). The fake's `split_only_mode_refuses_an_absolute_placement`
    sequence `r1c1, r1c2, r2c2, r2c1` is **deliberately not** reproduced. Real Herdr nests the `r2c2` that a `down` split of
    `r1c2` makes (spike §7), so `plan_splits` refuses it under 11c. See Decision 6.
13. **Shared vocabulary**: `plan::GRID_UNREACHABLE.as_str() == holler_pane_testkit::herdr::GRID_UNREACHABLE.as_str()` and
    `protocol::SUPPORTED_VERSIONS == holler_pane_testkit::herdr::SUPPORTED_VERSIONS`. This resolves the
    `ASSUMPTION (#640)` at testkit `herdr.rs:42-47`.

**The protocol (`protocol_test.rs`)**

14. `to_line` gives one JSON object per request, ending in exactly one `\n` with no other newline. `{"id":"holler:<method>",
    "method":..., "params":{...}}` matches the schema params above. `Split` sends `target_pane_id`, `direction`
    ("right"|"down"), `ratio` and `"focus": false`. `WorkspaceCreate` sends `label` and `"focus": false`. `Read` sends
    `"source": "recent"`, `lines` and `"format": "text"`. `SendKeys` sends the key names **verbatim**: `Key::new("enter")`
    goes out as `"enter"`, and nothing is case-folded or translated (Decision 9).
15. **Method allow-list**: every `Request` variant's `method()` is in `ALLOWED_METHODS`, and `ALLOWED_METHODS` contains
    none of `server.stop`, `server.live_handoff`, `layout.apply`, `pane.move`, `pane.swap`, or any `plugin.*` or
    `integration.*` name.
16. `decode_reply`:
    - `{"id":"holler:pane.close","result":{"type":"ok"}}` gives `Ok(result)`.
    - `{"id":...,"error":{"code":"pane_not_found","message":"pane w1:p5 not found"}}` gives
      `PaneError::PaneNotFound { what: "w1:p5" }` (the id from the request).
    - `invalid_request` and any other code give `PaneError::Unavailable`, with a one-line `what` naming the method and
      Herdr's code.
    - Not JSON, an empty line, an id that is not the request's, or an object with neither `result` nor `error` gives
      `Unavailable`.
    - No `what`/`message` contains a newline, and none echoes `SendText`'s text.
17. `parse_pong` on the spike's verbatim pong gives `ServerVersion { version: "0.9.1-preview.2026-09-21-0ff0f27e2226",
    protocol: Some(22) }`, and `check_supported` accepts it. Protocol 99 gives `HerdrVersionUnsupported`, whose message
    names the reported version, the protocol `99` and `SUPPORTED_VERSIONS`, on one line. A pong without `protocol` gives
    `protocol: None`, which `check_supported` refuses the same way and names as unknown. A result whose `type` is not
    `pong` gives `Unavailable`.
18. `parse_snapshot` on a hand-written `session_snapshot` with the schema's required fields: two workspaces (labels
    `scratch` and `other`), the first with tabs numbered 2 and 1. `workspace("scratch")` gives its `WorkspaceRef`, with
    `grid_tab` set to the tab with the **lowest `number`**. `workspace("absent")` gives `Ok(None)`. Two workspaces labelled
    alike give `Unavailable`, naming the label and both ids. `panes` carries each `PaneInfo`'s `pane_id`, `workspace_id` and
    `tab_id`, and unknown extra fields are ignored.
19. `parse_layout_export` on the 2x4 export, with `cwd`, `label` and `command: null` on the panes, gives the `LayoutNode`
    tree of AC 1. A pane node whose `pane_id` is null or missing, an unknown `direction` such as `"left"`, or a `type` other
    than `layout_export` gives `Unavailable`.
20. `parse_workspace_created` gives the `WorkspaceRef` (its `grid_tab` is the created `tab.tab_id`) and `root_pane.pane_id`.
    `parse_pane_info` gives `pane.pane_id`. `parse_read` gives the last `max_lines` lines of `read.text` (as `str::lines`
    splits them, joined with `\n`, with no trailing newline). `max_lines == 0` gives `""`. `expect_ok` accepts only
    `type: "ok"`.

**The gates**

21. `cargo test -p holler-adapter-herdr`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets -- -D warnings` and the `cargo machete` CI step all pass. Every new `.rs` file
    passes `rustfmt --check --edition 2021` (epic ruling 4), and no existing file is reformatted. There is no `unsafe`. Every
    touched file stays under 900 lines.
22. `CHANGELOG.md` `[Unreleased]` / `### Enhancements` gets one entry linking #640 (and epic #633). The suggested text:
    "Herdr adapter, part 1 (#640): the pure Herdr protocol and grid core in `holler-adapter-herdr`, which reads a pane's
    row and column from Herdr's split tree, plans the right and down splits that reach a cell (`grid-unreachable` when
    Herdr cannot reach it without nesting), and gates Herdr's protocol version (22). No I/O yet; the socket adapter
    follows in part 2."

## Files, blast radius and size

All are within the issue's blast radius (`crates/holler-adapter-herdr/**`), plus `CHANGELOG.md` and the `Cargo.lock`
update that cargo makes.

| File | Change | Est. lines |
|---|---|---|
| `crates/holler-adapter-herdr/Cargo.toml` | deps `holler-pane`, `serde` (derive), `serde_json`; dev-dep `holler-pane-testkit`; one comment per dep (house style) | +15 |
| `crates/holler-adapter-herdr/src/lib.rs` | module docs, `pub mod` x3, re-exports | ~45 |
| `crates/holler-adapter-herdr/src/layout.rs` | new | ~180 |
| `crates/holler-adapter-herdr/src/plan.rs` | new | ~220 |
| `crates/holler-adapter-herdr/src/protocol.rs` | new | ~340 (if it passes ~800, split the parsers into `protocol/parse.rs`) |
| `crates/holler-adapter-herdr/tests/layout_test.rs` | new (T) | ~230 |
| `crates/holler-adapter-herdr/tests/plan_splits_test.rs` | new (T) | ~300 |
| `crates/holler-adapter-herdr/tests/protocol_test.rs` | new (T) | ~330 |
| `CHANGELOG.md` | one entry | +6 |

Size: about 800 production lines and about 860 test lines, so about 1,700 in all. That is 5 production files in one
component, inside the cap.

**Not touched:**

- `holler-pane/**` (frozen, #637). The doc-comment updates that `ASSUMPTION (#640)` asks for are part 3.
- `holler-pane-testkit/**` (#638's).
- `holler-cli/src/pane/wiring.rs` (#649 builds the adapter there; it keeps `Unwired` until then).
- `docs/adr/ADR-0021.md` (part 3).
- Every other crate.

The tests carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the top, as
`holler-pane-testkit/tests/*.rs` do. This crate does not set `autotests = false`, so `tests/*.rs` build without a
`[[test]]` entry.

## Reuse map (extend, do not duplicate)

| Need | Existing object | Recommendation |
|---|---|---|
| The port, its types and every error | `holler_pane::{HerdrPort, HerdrSpec, HerdrPane, HerdrSnapshot, Key, PaneId, GridPos, PaneError}`, `holler_pane::error::{RefusalCode, class_of, ErrorClass}` | **Extend/use**. No parallel error enum, `GridPos` or key type. Open codes go through `RefusalCode::from_static` (ADR-0021 §9). |
| The closest analogous feature | `holler-pane-testkit/src/herdr.rs` (`FakeHerdr`: placement rules, `GRID_UNREACHABLE`, version strings, the refusal wording) | **Mirror its vocabulary**. Codes and constant values are equal (AC 13), and the refusal message shape matches. Do **not** depend on the testkit at runtime; it is a dev-dependency only (ADR-0021 §5). |
| The conformance suite | `holler_pane_testkit::conformance::herdr::run_herdr_conformance` | Part 2 runs it. Part 1 makes its cases reachable: range first, idempotent, one pane per step. |
| The tree walk | `scripts/spikes/herdr-grid.sh` `derive()` | **Port it exactly** to `grid_of` (AC 1-6). Do not invent another rule. |
| `last_lines` for `read` | the testkit's private `last_lines` (`herdr.rs:520-525`) | It is private and dev-only, so `parse_read` re-implements the same 3-line rule. The behaviour must be identical (AC 20). Justified duplication. |
| The JSON-line socket client | `holler_hub::control::send_over` (`crates/holler-hub/src/control.rs:485-518`) | **Part 2's** pattern to follow (connect, set the timeout, write one line, read one line). Do not import `holler-hub`: an adapter depends only on `holler-pane` (ADR-0021 §5). |
| The new modules | none exist (the crate is empty) | **New, justified**: this crate is the only allowed home of the conversion (epic decision 7, ADR-0021 §10). |

## Decisions already made (MO)

1. **The split into three parts** (above). This brief is part 1. Parts 2 and 3 run on the same issue and branch name,
   after this part merges.
2. **Socket, not CLI** (spike §12): `layout.export` exists only on the socket. Part 1 builds socket requests only.
3. **The tree walk is the only conversion.** The down-chain of the root gives rows and each row's right-chain gives columns,
   both counted from 1 in tree order (first child first). Rects (x before y, 0-based) are **not** used for positions: they
   follow the client's terminal size, and they rank unaligned rows wrongly (spike §6).
4. **The workspace extent is adapter configuration.** Herdr has no grid, so the caller gives each workspace's `Extent` as
   rows by columns. That mirrors `FakeHerdr::with_workspace`. `plan_splits` takes it through `Target`, and part 2's
   `HerdrConfig` holds it per workspace label. Who supplies it is #649's wiring (from the profile's or the operator's
   configuration). A cell outside the extent is `grid-out-of-range`, as the conformance suite pins.
5. **One `ensure_pane` makes at most one pane, never an intermediate one.** `plan_splits` takes a set of target cells, so
   that one call can plan a whole layout (the 2x4 case for `profile apply`, #664). Every step creates exactly one target
   cell, and a cell that needs an unrequested pane first is `grid-unreachable`.
6. **Rows first, and a new row only below a row of one pane.** Herdr's nesting rule turns any other `down` split into a
   non-grid, which is `grid-unreachable`. This is **stricter than `FakeHerdr`'s split-only mode**, which accepts a `down`
   split under any cell (`r2c2` below `r1c2`). The test kit is #638's and is not changed here. See "For the operator".
7. **A non-grid workspace refuses new placement.** If any pane is unplaced, `plan_splits` refuses a new cell with
   `grid-unreachable` and still answers cells that exist. Positions of placed panes are still read (part 2's `snapshot`
   lists only placed panes).
8. **Version gate on `protocol`, the integer.** Support is exactly `[22]`. The version string is what `version()` returns
   and what a verb records as `host.herdr_api_version`: the adapter does not write the pane store, and #644 and #647
   record and show it. A missing `protocol` is unsupported, not garbled. Part 2's `connect` pings and refuses, and its
   `version()` pings and refuses again. Other methods do not re-check, which answers the testkit's `herdr.rs:318`
   assumption for this adapter.
9. **Keys go out verbatim, by Herdr's names** (`enter`, `ctrl+c`). There is no case-folding, which answers `herdr.rs:272`
   and `conformance/herdr.rs:337`. Part 3 fixes the `Key` doc in `ports.rs`.
10. **Error mapping.** Herdr `pane_not_found` is `pane-not-found` (`what` = the pane id). Every other Herdr error code, and
    every garbled or mismatched reply, is `unavailable` (ADR-0021 §9: "also a garbled reply"). Messages are one line and
    never echo typed text.
11. **Workspace identity.** `HerdrSpec.workspace` is a Herdr workspace's `label`, and the grid lives in its tab with the
    lowest `number`. Two workspaces with the same label are `unavailable`. Part 2: a configured workspace that Herdr lacks
    is created by `workspace.create {label, focus:false}` only when `r1c1` is asked for (`Step::CreateRoot`). That answers
    `herdr.rs:358`. An unconfigured workspace is `unavailable`, as in the fake.
12. **RED for a brand-new API.** The crate exports nothing, and the overlay says a compile error is not a valid RED. So
    T-red adds `layout.rs`, `plan.rs` and `protocol.rs` holding **only** the pinned signatures and constants, with
    placeholder bodies and no logic:
    - functions that return `Result` return `Err(PaneError::NotImplemented)`;
    - `grid_of` returns `GridMap::default()`;
    - `to_line`/`id` return `String::new()`;
    - `method` returns `""`.

    T marks each stub `// stub (#640 part 1): F fills`. F replaces the bodies, and T must not write logic. AC 13 passes at
    RED (it pins constants, not behaviour); every other AC fails on an assertion.

## Forward-compat

| Consumer | Needs from part 1 | Satisfied |
|---|---|---|
| #640 part 2 (adapter) | `Request`/`decode_reply`/parsers; `grid_of` for `snapshot` and for `ensure_pane`'s existing map; `plan_splits` with `Step::Split.from` resolved to a pane id at run time; `CreateRoot` for a missing workspace; the version gate | yes |
| #640 part 3 (scratch test, docs) | the constants, and the decisions above for the docs | yes |
| #644 launch/relaunch | `grid-out-of-range` and `grid-unreachable` as refusals (exit 3); a version string to record | yes |
| #664 `profile apply` | a multi-cell plan in rows-first order; the range checked before anything acts | yes |
| #647 doctor/reconcile, #650 import | positions read from the tree; **unplaced panes cannot appear in `HerdrSnapshot`** (`HerdrPane.grid` is required) | **needs discussion**: see "For the operator" |
| #649 wiring | part 2's `HerdrConfig` (socket path, session, per-workspace `Extent`, timeout) | yes (part 2) |

## Out of scope (part 1)

- Any I/O, socket or timeout. The `HerdrPort` impl. The wire fake and the conformance run (part 2).
- The scratch-Herdr test, the `holler-pane` doc comments and ADR-0021 (part 3).
- Wiring (#649), recording `host.herdr_api_version` (#644/#647), an argv launch without a shell (`layout.apply` into a
  staging tab plus `pane.move`, spike §7): it is not in `HerdrPort` and needs a contract amendment first.
- `events.subscribe`. Any change to `FakeHerdr`.

## Test plan

RED: T writes the three test files against the pinned API and adds the stub modules (Decision 12). `cargo test -p
holler-adapter-herdr` compiles. AC 1-12 and 14-20 fail on assertions: the empty `GridMap`, `NotImplemented` where a value
is expected, an empty `to_line`. AC 13 passes. T's handoff lists each test with its AC and the failing assertion.

GREEN: F fills the bodies. All of AC 1-20 pass, the gates of AC 21 pass, and T records the two conversion mutants (AC 2).
No test sleeps, spawns a process or opens a socket.

## Risks

- **The conversion is the core risk.** The tree walk is ported from a script that was verified against a real server.
  The hand-written JSON fixtures copy the spike's verified 2x4 build and the schema's field names. Part 3's scratch test
  is the real-server check.
- **Herdr's float ratio.** Herdr stores f32 (`0.3333` becomes `0.33329999...`), and positions never depend on ratios
  (AC 4).
- **The plan is stricter than the fake.** A verb test written against `FakeHerdr`'s split-only mode can accept a layout
  that the real adapter refuses. That is flagged below. It is not changed here.
- The constants are equal to the testkit's by test (AC 13). If the testkit's values change first, AC 13 fails loudly.
  That is intended.

## Handoff locations

T-red: `docs/handoffs/640/handoff-T-red.md`. F: `docs/handoffs/640/handoff-F.md`. T-green: `.../handoff-T-green.md`.
A: `.../handoff-A.md`, `.../handoff-A-dup.md`. S: `.../handoff-S.md`. Journal: `docs/handoffs/640/decisions.md`.

## Operating rules

- Read this brief, the Evidence and the Reuse map before any code. Extend the named objects; a parallel error type,
  `GridPos` or key type is an anti-duplication BLOCK.
- Test-first: T writes every test and only the stubs of Decision 12. F writes no tests.
- Never run, attach to or talk to a Herdr server, OpenCode, or a real tmux session; part 1 needs none.
- Conventional Commits (`feat(adapter-herdr): ...`). The hooks add the trailer. Stage files by explicit path. The PR says
  `Part of #640` and carries the AI disclosure required by `CONTRIBUTING.md`.
- Public repo: no personal hostnames, IPs or names in code, tests, fixtures or messages.

## For the operator (review, not blocking)

1. **Test-kit divergence (Decision 6).** `FakeHerdr`'s split-only mode accepts layouts that real Herdr nests, for example
   `r2c2` by a `down` split of `r1c2`. The proposed fix is a small follow-up on #638's test kit, tightening split-only to
   the rule of AC 11c so that verb tests cannot pass on layouts the adapter refuses.
2. **Unplaced panes have no place in `HerdrSnapshot`** (`HerdrPane.grid` is required). Part 2 leaves them out of
   `snapshot`, so reconcile (#647) and import (#650) cannot see a pane that sits in a nested split or another tab. The
   proposed fix is a contract amendment, amend-first in its own PR before #647, to add an `unplaced` list (or an optional
   grid).
3. **The issue's "run" in the scratch acceptance** maps to `send_text` plus `send_keys(["enter"])` in the scratch pane's
   shell. `HerdrPort` has no run method, and an argv launch without a shell needs a contract change.

Needs operator: nothing blocks part 1.

## Outlines of parts 2 and 3 (for forward-compat; the full briefs come later)

**Part 2: socket transport and `HerdrAdapter`** (about 4 crate files plus tests).

- `transport.rs`:
  - `trait Transport: Send + Sync { fn call(&self, req: &Request, deadline: Instant) -> Result<Value, PaneError> }`.
  - `UnixSocketTransport`: a new connection per request; read and write timeouts set to the remaining deadline.
  - Connect failure or a missing socket is `unavailable`, naming the socket path. An expired deadline is
    `timeout { op: "herdr.<method>" }`.
- `adapter.rs`:
  - `HerdrConfig { session, socket, workspaces: label to Extent, timeout = 10 s }`.
  - `HerdrAdapter::connect(config)` pings and applies the version gate.
  - `impl HerdrPort`:
    - `ensure_pane`: snapshot, export, `grid_of`, then `plan_splits(existing, existing + spec.grid)`, then execute at
      most one step, then re-export and confirm the new pane reads back at `spec.grid`, else `unavailable`. Never caches
      ids. A `spec.session` other than the served one is `unavailable`.
    - `snapshot`: placed panes of every workspace.
    - `read`: `pane.read recent`, trimmed.
- Dev-tests:
  - a wire fake implementing `Transport`, which simulates Herdr's JSON semantics and rejects a transposed or off-by-one
    split target;
  - `run_herdr_conformance` against it (default run);
  - `r2c1` and `r1c2` land and read back;
  - the version 22/99 refusal;
  - a real `UnixListener` in a temp dir for the transport: no reply gives `timeout` within the bound plus a margin; a
    garbled line or a missing socket gives `unavailable`.
- Adds the dev-dependency `tempfile`.

**Part 3: the scratch server and the contract docs.** `tests/scratch_herdr_test.rs` is `#[ignore]` and also needs
`HOLLER_HERDR_SCRATCH=1`. It skips with a message when `herdr` is not on PATH. It works like `herdr-lib.sh`:

- a temp HOME and XDG root, with every `HERDR_*` variable removed;
- session `holler640-<hex>`. The test refuses any other name, including the default session, and any socket outside the
  root;
- one server per conformance case, whose guard kills only its own PID and removes the root;
- then create, run (`send_text` and `enter`), send, read, close and snapshot one pane.

The doc-comment-only updates:

- `holler-pane/src/ports.rs`: `HerdrPort::ensure_pane` names `grid-out-of-range` (outside the configured extent) and
  `grid-unreachable`; `Key` names Herdr's own key names;
- `holler-pane/src/error.rs`: the `GridOutOfRange` doc adds "a cell outside the Herdr workspace's extent";
- ADR-0021 §9: `grid-unreachable` (open, #640) in the launch/relaunch and `profile apply` rows, and `grid-out-of-range` in
  the apply row;
- ADR-0021 §10: one sentence saying the extent is adapter configuration.

The PR closes #640.
