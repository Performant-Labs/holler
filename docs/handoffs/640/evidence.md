# Evidence: #640 part 2 (facts in unchanged code that the diff relies on)

Written by F (Phase 5/6 of the run). T appends its own entries at T-green.

- **Fact:** `decode_reply` turns Herdr's `pane_not_found` about the request's own pane into `PaneError::PaneNotFound`, and every other Herdr error into `unavailable`. `adapter::changed_under` therefore rewrites only that one variant on the `pane.split` result (AC 17), and a `pane-not-found` from `send_text`, `send_keys`, `read` or `close` reaches the caller unchanged (conformance cases 7 to 9).
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:279-296`
  **Verbatim excerpt:**
  > ```rust
  > /// What Herdr's error reply to `request` maps to: `pane_not_found` about the request's
  > /// pane is `pane-not-found`, and anything else is `unavailable`, naming the method and
  > /// Herdr's code but never Herdr's message.
  > fn herdr_error(request: &Request, error: &Value) -> PaneError {
  >     let method = request.method();
  >     match (error.get("code").and_then(Value::as_str), request.pane()) {
  >         (Some(PANE_NOT_FOUND), Some(pane)) => PaneError::PaneNotFound {
  >             what: pane.as_str().to_owned(),
  >         },
  > ```

- **Fact:** `plan_splits` answers an occupied cell with no step (`Ok(Vec::new())`) after the range check, so `ensure_pane`'s `[]` arm finds the cell's pane in the map (AC 18). A cell outside the extent is refused first (`grid-out-of-range`, conformance case 2).
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:75-86`
  **Verbatim excerpt:**
  > ```rust
  > pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError> {
  >     let extent = target.extent;
  >     let mut cells = target.cells.clone();
  >     cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
  >     cells.dedup();
  >     if let Some(&outside) = cells.iter().find(|cell| !contains(extent, **cell)) {
  >         return Err(out_of_range(outside, extent));
  >     }
  >     cells.retain(|cell| existing.at(*cell).is_none());
  >     let Some(&first) = cells.first() else {
  >         return Ok(Vec::new());
  >     };
  > ```

- **Fact:** `Step::CreateRoot` is planned only when the grid the plan builds on is empty, and only for `r1c1`. The planner's grid is built from the map's rows, so a workspace Herdr already has (a tree with at least one leaf, hence at least one row) never gets `CreateRoot`. `ensure_pane` sends `workspace.create` only on that step, so it never creates a second workspace of a label Herdr has (AC 20).
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:113-117` and `:132-143`
  **Verbatim excerpt:**
  > ```rust
  > impl Widths {
  >     /// The grid of `map`.
  >     fn of(map: &GridMap) -> Self {
  >         Self((1..=map.rows()).map(|row| map.cols_in(row)).collect())
  >     }
  > ```
  > ```rust
  >     /// The one split that makes `cell`, a cell that is not in the grid yet.
  >     fn step_to(&self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
  >         if self.0.is_empty() {
  >             if cell == ROOT {
  >                 return Ok(Step::CreateRoot);
  >             }
  > ```

- **Fact:** `grid_of` makes one row per link of the root's `down` chain, and a node that is not a `down` split is one link, so any tree (a lone leaf included) gives at least one row. This is the premise of the entry above, and it is the read-back that `adapter::confirm` uses (AC 13, 16).
  **Source:** `crates/holler-adapter-herdr/src/layout.rs:145-149` and `:171-185`
  **Verbatim excerpt:**
  > ```rust
  > pub fn grid_of(root: &LayoutNode) -> GridMap {
  >     let mut map = GridMap::default();
  >     for (row, row_node) in chain(root, Direction::Down).into_iter().enumerate() {
  >         let slots = chain(row_node, Direction::Right)
  >             .into_iter()
  > ```
  > ```rust
  > fn chain(node: &LayoutNode, direction: Direction) -> Vec<&LayoutNode> {
  >     match node {
  >         LayoutNode::Split {
  >             direction: split,
  >             first,
  >             second,
  >             ..
  >         } if *split == direction => {
  >             let mut links = chain(first, direction);
  >             links.extend(chain(second, direction));
  >             links
  >         }
  >         _ => vec![node],
  >     }
  > }
  > ```

- **Fact:** `parse_workspace_created` always sets the created workspace's `grid_tab` to the created tab, so after `CreateRoot` the read-back exports that tab, and `adapter::grid_tab` finds no missing tab there.
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:486-497`
  **Verbatim excerpt:**
  > ```rust
  > pub fn parse_workspace_created(result: &Value) -> Result<(WorkspaceRef, PaneId), PaneError> {
  >     let created = result_of(result, "workspace_created")?;
  >     let workspace = created.object("workspace", "created workspace")?;
  >     let tab = created.object("tab", "created tab")?;
  >     let root = created.object("root_pane", "created root pane")?;
  >     let workspace = WorkspaceRef {
  >         workspace_id: workspace.string("workspace_id")?.to_owned(),
  >         label: workspace.string("label")?.to_owned(),
  >         grid_tab: Some(tab.string("tab_id")?.to_owned()),
  >     };
  > ```

- **Fact:** `SessionState::workspace` refuses a label that two Herdr workspaces share, so `ensure_pane` is `unavailable` for duplicate labels (AC 23) while `snapshot`, which walks `state.workspaces` directly, lists both (AC 22).
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:358-368`
  **Verbatim excerpt:**
  > ```rust
  > impl SessionState {
  >     /// The workspace labelled `label`; two with one label are `unavailable`.
  >     pub fn workspace(&self, label: &str) -> Result<Option<&WorkspaceRef>, PaneError> {
  >         let labelled: Vec<&WorkspaceRef> = self
  >             .workspaces
  >             .iter()
  >             .filter(|workspace| workspace.label == label)
  >             .collect();
  >         match labelled.as_slice() {
  >             [] => Ok(None),
  >             [one] => Ok(Some(*one)),
  > ```

- **Fact:** `parse_read` cuts the reply to the caller's `max_lines`, and 0 lines gives `""`. So `read(p, 0)` can ask Herdr for one line (the clamp) and still return `""` (AC 25).
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:506-509` and `:592-595`
  **Verbatim excerpt:**
  > ```rust
  > pub fn parse_read(result: &Value, max_lines: usize) -> Result<String, PaneError> {
  >     let read = result_of(result, "pane_read")?.object("read", "pane read")?;
  >     Ok(last_lines(read.string("text")?, max_lines))
  > }
  > ```
  > ```rust
  > fn last_lines(screen: &str, max_lines: usize) -> String {
  >     let skip = screen.lines().count().saturating_sub(max_lines);
  >     screen.lines().skip(skip).collect::<Vec<_>>().join("\n")
  > }
  > ```

- **Fact:** the port bounds every method by I5 (default 10 s) and returns `Timeout` otherwise. That is why each port method takes one deadline on entry, and why the transport bounds a whole exchange rather than one read.
  **Source:** `crates/holler-pane/src/ports.rs:118-123`
  **Verbatim excerpt:**
  > ```rust
  > /// Herdr, reached over its local socket (the adapter is `holler-adapter-herdr`,
  > /// #640). **Provisional** until spike #636 reports.
  > ///
  > /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
  > /// thread) in async code. Every method returns within I5's bound (default 10 s) or
  > /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
  > ```

- **Fact:** the conformance suite finds a fixture's panes by `HerdrPane.workspace == <label>`, so `snapshot` must report a workspace by its label, never its `w<N>` id.
  **Source:** `crates/holler-pane-testkit/src/conformance/herdr.rs:394-401`
  **Verbatim excerpt:**
  > ```rust
  >     fn panes(&self) -> Result<Vec<HerdrPane>, String> {
  >         let snapshot = succeeds("snapshot", self.port.snapshot())?;
  >         Ok(snapshot
  >             .panes
  >             .into_iter()
  >             .filter(|pane| pane.session == self.session && pane.workspace == self.workspace)
  >             .collect())
  >     }
  > ```
