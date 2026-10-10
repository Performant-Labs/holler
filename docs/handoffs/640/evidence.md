# Evidence: #640 part 3 (facts in unchanged code that the diff relies on)

Written by F (the run's implementation phase). T appends its own entries at T-green. This file replaces part 2's
`evidence.md`, as the brief's Handoffs line says for part 2's files; part 2's is in git at `0ad2d8a`.

- **Fact:** the socket transport is this crate's only producer of `PaneError::Timeout`, and its `op` names the wire
  method. So every `timeout` that reaches the adapter's `run_as` carries `herdr.<wire method>`, which `run_as` replaces
  with the call's own `op` (AC 16). The transport itself is unchanged (AC 20).
  **Source:** `crates/holler-adapter-herdr/src/transport.rs:111-113` and `:268-273`
  **Verbatim excerpt:**
  > ```rust
  >         match answered.recv_timeout(exchange.remaining()) {
  >             Ok(result) => result,
  >             Err(RecvTimeoutError::Timeout) => Err(exchange.timeout()),
  > ```
  > ```rust
  >     /// `timeout`, its `op` naming the method.
  >     fn timeout(&self) -> PaneError {
  >         PaneError::Timeout {
  >             op: format!("herdr.{}", self.method),
  >         }
  >     }
  > ```

- **Fact:** production's `connect` goes through `connect_with`, so a `ping` that runs out of time while connecting over
  the real socket is `timeout` with `op` `herdr.connect` too, not only over the test seam.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:102-109`
  **Verbatim excerpt:**
  > ```rust
  > impl HerdrAdapter<UnixSocketTransport> {
  >     /// Connect to the socket of `config`, as [`HerdrAdapter::connect_with`] does. This
  >     /// is the way production builds an adapter.
  >     pub fn connect(config: HerdrConfig) -> Result<Self, PaneError> {
  >         let transport = UnixSocketTransport::new(config.socket.clone());
  >         Self::connect_with(config, transport)
  >     }
  > }
  > ```

- **Fact:** nothing reads the text of a `timeout`'s `op`. `PaneError` hands it out as the error's detail and prints
  it, so renaming it changes what a message says, not what any code does (Decision 9).
  **Source:** `crates/holler-pane/src/error.rs:562` and `:676`
  **Verbatim excerpt:**
  > ```rust
  >             PaneError::Timeout { op } => Some(op),
  > ```
  > ```rust
  >             PaneError::Timeout { op } => write!(f, "timed out: {op}"),
  > ```

- **Fact:** the seven port-method `op`s the adapter now spells (`OP_ENSURE_PANE` to `OP_VERSION`) are the test kit's
  `HerdrOp` strings, which `adapter_messages_test.rs` takes its expected values from.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:66-76`
  **Verbatim excerpt:**
  > ```rust
  >     fn as_str(self) -> &'static str {
  >         match self {
  >             HerdrOp::EnsurePane => "herdr.ensure_pane",
  >             HerdrOp::SendText => "herdr.send_text",
  >             HerdrOp::SendKeys => "herdr.send_keys",
  >             HerdrOp::Read => "herdr.read",
  >             HerdrOp::Close => "herdr.close",
  >             HerdrOp::Snapshot => "herdr.snapshot",
  >             HerdrOp::Version => "herdr.version",
  >         }
  >     }
  > ```

- **Fact:** `excerpt` cuts at 64 characters and appends `...` only when it cut, so for a value of at most 64 characters
  it returns exactly the `{:?}` form the four sites used before. Short ids are quoted as before (part 2's
  `adapter_test.rs` assertions such as `what.contains("w1:p2")` keep holding), and only a longer value changes (AC 18).
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:62-63` and `:578-587`
  **Verbatim excerpt:**
  > ```rust
  > /// How many characters of a string Herdr sent a message quotes.
  > const EXCERPT_LIMIT: usize = 64;
  > ```
  > ```rust
  > /// `text` that Herdr sent, quoted on one line and cut to [`EXCERPT_LIMIT`] characters,
  > /// so that a garbled reply can neither lengthen a message nor break it across lines.
  > pub(crate) fn excerpt(text: &str) -> String {
  >     let head: String = text.chars().take(EXCERPT_LIMIT).collect();
  >     if head.len() < text.len() {
  >         format!("{head:?}...")
  >     } else {
  >         format!("{head:?}")
  >     }
  > }
  > ```

- **Fact:** the four values now quoted through `excerpt` are Herdr's own text, read straight from its replies: a new
  pane's id from `pane.split`'s `pane_info` (`made` in `confirm`), a pane id from a `layout.export` tree node (`target`
  in `changed_under`), and a workspace label from `session.snapshot` (`grid_tab`). The caller's values (`spec.session`,
  `spec.workspace`, the config's) keep `{:?}` (Decision 10).
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:499-502`, `:453-458` and `:431-434`
  **Verbatim excerpt:**
  > ```rust
  > /// Read a `pane_info` result (what `pane.split` returns).
  > pub fn parse_pane_info(result: &Value) -> Result<PaneId, PaneError> {
  >     let pane = result_of(result, "pane_info")?.object("pane", "pane")?;
  >     Ok(PaneId::new(pane.string("pane_id")?))
  > ```
  > ```rust
  > /// Read one node of a layout tree, and the nodes under it.
  > fn layout_node(node: Object<'_>) -> Result<LayoutNode, PaneError> {
  >     match node.string("type")? {
  >         "pane" => Ok(LayoutNode::Pane {
  >             pane_id: PaneId::new(node.string("pane_id")?),
  >         }),
  > ```
  > ```rust
  >     Ok(WorkspaceRef {
  >         workspace_id: workspace_id.to_owned(),
  >         label: workspace.string("label")?.to_owned(),
  >         grid_tab,
  > ```

- **Fact:** the label `snapshot` returns in `HerdrPane.workspace` (`adapter.rs:383`, `workspace: workspace.label.clone()`)
  must stay whole, because the conformance suite finds a fixture's panes by exact label. That line is a returned value,
  not message text, so it is not an `excerpt` (AC 19's exemption, A's finding 5).
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

- **Fact:** D2's and D3's new doc text matches what the adapter answers for a cell outside the configured extent:
  `plan_splits` refuses it before anything else, with `grid-out-of-range` naming the cell and the extent.
  **Source:** `crates/holler-adapter-herdr/src/plan.rs:75-81` and `:230-239`
  **Verbatim excerpt:**
  > ```rust
  > pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError> {
  >     let extent = target.extent;
  >     let mut cells = target.cells.clone();
  >     cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
  >     cells.dedup();
  >     if let Some(&outside) = cells.iter().find(|cell| !contains(extent, **cell)) {
  >         return Err(out_of_range(outside, extent));
  > ```
  > ```rust
  > /// `grid-out-of-range` for `cell`, naming the extent.
  > fn out_of_range(cell: GridPos, extent: Extent) -> PaneError {
  >     PaneError::GridOutOfRange {
  >         what: format!(
  >             "{cell} is outside the workspace, which is {} by {}",
  >             count(usize::from(extent.rows), "row"),
  >             count(usize::from(extent.cols), "column")
  >         ),
  >     }
  > }
  > ```

- **Fact:** D1's "It goes to Herdr as written": `send_keys` puts each key's own text on the wire, with no mapping.
  **Source:** `crates/holler-adapter-herdr/src/protocol.rs:186-189`
  **Verbatim excerpt:**
  > ```rust
  >             Request::SendKeys { pane, keys } => json!({
  >                 "pane_id": pane.as_str(),
  >                 "keys": keys.iter().map(Key::as_str).collect::<Vec<_>>()
  >             }),
  > ```

- **Fact:** D5's and A1's "the Herdr adapter writes no record": the adapter holds its config and its transport and
  nothing else, so no pane store is within its reach.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:95-100`
  **Verbatim excerpt:**
  > ```rust
  > /// The `HerdrPort` over `T`.
  > #[derive(Debug)]
  > pub struct HerdrAdapter<T = UnixSocketTransport> {
  >     config: HerdrConfig,
  >     transport: T,
  > }
  > ```

- **Fact:** `docs/testing.md`'s "no workflow passes `--ignored` for this crate": CI's default run passes no
  `--ignored`, and its only `--ignored` run is `holler-cli`'s `body_run_test`.
  **Source:** `.github/workflows/ci.yml:128` and `:200`
  **Verbatim excerpt:**
  > ```yaml
  >         run: cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
  > ```
  > ```yaml
  >         run: cargo test -p holler-cli --test body_run_test -- --ignored
  > ```

- **Fact:** `docs/testing.md`'s sibling convention: the host adapter's real-tmux tests are opt-in through `#[ignore]`
  alone (no variable) and pass, skipping, when `tmux` is missing. This file is on `main` (merged in `e327569`, #641),
  not on this branch's base `dc300ab`, so the source below is read with `git show origin/main:<path>`.
  **Source:** `origin/main:crates/holler-adapter-host/tests/real_tmux_test.rs:94-102` and `:206`
  **Verbatim excerpt:**
  > ```rust
  > fn tmux_available() -> bool {
  >     let found = Command::new("tmux")
  >         .arg("-V")
  >         .output()
  >         .is_ok_and(|o| o.status.success());
  >     if !found {
  >         eprintln!("skipped: tmux not found");
  >     }
  >     found
  > ```
  > ```rust
  > #[ignore = "needs tmux; run with --ignored"]
  > ```
