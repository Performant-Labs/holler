# Evidence: #640 part 1 (F, Phase 6 implement)

Facts outside the diff that the implementation, or T's tests, rely on. Each excerpt is copied from the source at
`09ada91`.

- **Fact:** `GridPos` derives `PartialEq`, `Eq` and `Hash` but not `Ord`, so `plan_splits` sorts target cells with
  `sort_unstable_by_key(|cell| (cell.row, cell.col))` and cannot put them in a `BTreeSet`.
  **Source:** `crates/holler-pane/src/grid.rs:34-40`
  **Verbatim excerpt:**
  > ```
  > #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  > pub struct GridPos {
  >     /// The row, from 1.
  >     pub row: u16,
  >     /// The column, from 1.
  >     pub col: u16,
  > }
  > ```

- **Fact:** a `GridPos` displays as `r<row>c<col>`, row first. AC 1 (`"r2c3"`), every refusal's "names the cell"
  assertion and every planner message rely on it.
  **Source:** `crates/holler-pane/src/grid.rs:68-73`
  **Verbatim excerpt:**
  > ```
  > impl fmt::Display for GridPos {
  >     /// The `rRcC` form, row first: `r2c1`.
  >     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
  >         write!(f, "r{}c{}", self.row, self.col)
  >     }
  > }
  > ```

- **Fact:** a well-formed code that is not closed, such as `grid-unreachable`, is classed as a refusal (exit 3). AC 11
  asserts `class_of(code) == ErrorClass::Refusal`, and `plan.rs` adds nothing to the error table.
  **Source:** `crates/holler-pane/src/error.rs:266-273`
  **Verbatim excerpt:**
  > ```
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```

- **Fact:** `PaneError`'s `Display` prints a `Refused` error's message alone, and prefixes `unavailable` and
  `herdr-version-unsupported` with fixed text. So the one-line, no-typed-text guarantees of AC 16 and 17 rest only on
  the `what` and `message` strings the adapter builds.
  **Source:** `crates/holler-pane/src/error.rs:670-679`
  **Verbatim excerpt:**
  > ```
  >             PaneError::HerdrVersionUnsupported { message } => {
  >                 write!(f, "unsupported Herdr version: {message}")
  >             }
  >             PaneError::Timeout { op } => write!(f, "timed out: {op}"),
  >             PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
  >             PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
  >             PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
  >             PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
  >             PaneError::ProfileDrift { message } => write!(f, "profile drift: {message}"),
  >             PaneError::Refused { message, .. } => f.write_str(message),
  > ```

- **Fact:** `holler-pane`'s quoting helper `excerpt` is `pub(crate)`, so the adapter cannot call it. `protocol.rs` has
  its own private `excerpt` with the same 64-character rule (the same reason the brief gives for `last_lines`).
  **Source:** `crates/holler-pane/src/error.rs:686-695`
  **Verbatim excerpt:**
  > ```
  > /// `text` quoted for an error message, cut to 64 characters so an oversized input
  > /// cannot produce an oversized message.
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }
  > ```

- **Fact:** the test kit's two constants that AC 13 pins equal to the adapter's.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:36` and `crates/holler-pane-testkit/src/herdr.rs:51`
  **Verbatim excerpt:**
  > ```
  > pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
  > pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");
  > ```

- **Fact:** the test kit's `last_lines`, which `protocol::parse_read` re-implements with identical behaviour (AC 20).
  It is private and in a dev-only crate (ADR-0021 section 5), so it cannot be shared.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:520-525`
  **Verbatim excerpt:**
  > ```
  > /// The last `max_lines` lines of `screen` (as `str::lines` splits it), joined with
  > /// `"\n"`, with no newline after the last: `""` for none.
  > fn last_lines(screen: &str, max_lines: usize) -> String {
  >     let skip = screen.lines().count().saturating_sub(max_lines);
  >     screen.lines().skip(skip).collect::<Vec<_>>().join("\n")
  > }
  > ```

- **Fact:** the test kit's private `count` helper, which `plan.rs` mirrors (taking `usize`) so that the
  `grid-out-of-range` wording matches `FakeHerdr`'s ("2 rows by 1 column", AC 10).
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:497-504`
  **Verbatim excerpt:**
  > ```
  > /// `n` `noun`s, in the singular for one: "2 rows", "1 column".
  > fn count(n: u16, noun: &str) -> String {
  >     if n == 1 {
  >         format!("1 {noun}")
  >     } else {
  >         format!("{n} {noun}s")
  >     }
  > }
  > ```

- **Fact:** `Key::as_str` and `PaneId::as_str` return the text verbatim. That is why `SendKeys` sends `enter`,
  `ctrl+c`, `Enter` and `C-c` as given (AC 14, Decision 9), and why ids go out unchanged.
  **Source:** `crates/holler-pane/src/ports.rs:106-116` and `crates/holler-pane/src/pane.rs:79-89`
  **Verbatim excerpt:**
  > ```
  > impl Key {
  >     /// A key by its name.
  >     pub fn new(name: impl Into<String>) -> Self {
  >         Self(name.into())
  >     }
  >
  >     /// The key's name.
  >     pub fn as_str(&self) -> &str {
  >         &self.0
  >     }
  > }
  > ```
  > ```
  > impl PaneId {
  >     /// A pane id from Herdr's text.
  >     pub fn new(id: impl Into<String>) -> Self {
  >         Self(id.into())
  >     }
  >
  >     /// The id, verbatim.
  >     pub fn as_str(&self) -> &str {
  >         &self.0
  >     }
  > }
  > ```

- **Fact:** the spike's tree walk, which `layout::grid_of` ports. `chain` flattens a split of the given direction as
  first chain then second chain. Rows come from the root's `down` chain and slots from each row's `right` chain, both
  counted from 1. A slot that is not a pane gets no position.
  **Source:** `scripts/spikes/herdr-grid.sh:32-39`
  **Verbatim excerpt:**
  > ```
  > def chain(node, direction):
  >     if node["type"] == "split" and node["direction"] == direction:
  >         return chain(node["first"], direction) + chain(node["second"], direction)
  >     return [node]
  > by_tree = {}
  > for r, row in enumerate(chain(tree, "down"), 1):
  >     for c, cell in enumerate(chain(row, "right"), 1):
  >         by_tree[cell.get("pane_id")] = (r, c) if cell["type"] == "pane" else None
  > ```

- **Fact:** the spike's verified 2x4 recipe, which `plan_splits` reproduces for AC 7. The new row is split from the row
  above with `down 0.5`, and then each row's columns are split from the pane on the left with `0.25`, `0.3333` and
  `0.5`.
  **Source:** `scripts/spikes/herdr-grid.sh:55-57`
  **Verbatim excerpt:**
  > ```
  > R2C1="$(split "$R1C1" down 0.5)"
  > R1C2="$(split "$R1C1" right 0.25)"; R1C3="$(split "$R1C2" right 0.3333)"; R1C4="$(split "$R1C3" right 0.5)"
  > R2C2="$(split "$R2C1" right 0.25)"; R2C3="$(split "$R2C2" right 0.3333)"; R2C4="$(split "$R2C3" right 0.5)"
  > ```

- **Fact:** a split's `ratio` is the first child's (the split pane's) share, and a split divides only the target pane's
  own cell. These are the basis of `share()` and of AC 11c: a `down` split in a row of two nests.
  **Source:** `docs/research/herdr-api-spike.md:233-239`
  **Verbatim excerpt:**
  > ```
  > - **Ratio** is the **first** child's share: splitting a 120-wide pane `right` with `ratio 0.25` leaves the target
  >   30 wide and gives the new pane 90. It is stored as a 32-bit float (`0.3333` comes back as
  >   `0.33329999446868896` in events). VERIFIED.
  > - **The nesting rule: a split divides only the target pane's own cell.** After the 2x4 grid, `split r1c2 down`
  >   changed only `r1c2` (`{x:30,y:0,w:30,h:20}` became `{x:30,y:0,w:30,h:10}`) and added the new pane at
  >   `{x:30,y:10,w:30,h:10}`. No other rect changed. In the same way, splitting the top pane `right` left the full-width
  >   bottom pane alone. VERIFIED.
  > ```
