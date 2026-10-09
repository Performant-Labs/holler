# Evidence: #683 the pane test kit, slice d (`FakeHerdr`, `FakeProber`, the `HerdrPort` conformance suite)

Source facts in **unchanged** code that this change, or T's tests, rely on. Each entry has its source location and a
verbatim excerpt (base `e410e9d`; none of these files is touched by the diff).

## F (Phase 6, implementation)

- **Fact:** every fake port method calls the shared fault switch first. The switch records the call in the log before
  any fault is answered, sleeps the delay without holding its lock, and then answers the standing fault or the oldest
  one-shot error queued for that op. So a failed `send_text`/`send_keys` is still in `faults().calls()`, which is how
  I4's "no keystroke" is checked.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:93-103`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  >             state.delay
  >         };
  >         if let Some(delay) = delay {
  >             thread::sleep(delay);
  >         }
  >         self.lock().take_fault(op)
  >     }
  > ```

- **Fact:** a wedged switch answers `Timeout { op }` with the op's `as_str()` (`"herdr.<method>"` for this fake), and
  a standing `Fail(e)` answers `e`, each before any one-shot error.
  **Source:** `crates/holler-pane-testkit/src/fault.rs:119-128`
  **Verbatim excerpt:**
  > ```rust
  >     fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
  >         match &self.standing {
  >             Some(Fault::Wedged) => {
  >                 return Err(PaneError::Timeout {
  >                     op: op.as_str().to_owned(),
  >                 })
  >             }
  >             Some(Fault::Fail(error)) => return Err(error.clone()),
  >             None => {}
  >         }
  > ```

- **Fact:** the shared runner calls `fresh` once per case, runs the case against the subject, then drops the subject
  before the guard. So `run_herdr_conformance` builds a fresh fixture per case and drops the fixture before its guard
  (AC 1, `the_suite_builds_a_fresh_fixture_per_case`).
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:52-61`
  **Verbatim excerpt:**
  > ```rust
  >     let failures: Vec<CaseFailure> = cases
  >         .iter()
  >         .filter_map(|&(case, run)| {
  >             let (subject, guard) = fresh();
  >             let outcome = check(run, &subject);
  >             drop(subject);
  >             drop(guard);
  >             outcome.err().map(|detail| CaseFailure { case, detail })
  >         })
  >         .collect();
  > ```

- **Fact:** `expect_code` passes only on an error with exactly the code asked for. A success, or another code, is a
  failure whose detail names the call, so every failure the suite reports has a non-empty detail (AC 2).
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:82-89`
  **Verbatim excerpt:**
  > ```rust
  >     match result {
  >         Err(e) if e.code() == code => Ok(()),
  >         Err(e) => Err(format!(
  >             "{call}: expected `{code}`, got `{}`: {e}",
  >             e.code()
  >         )),
  >         Ok(_) => Err(format!("{call}: expected `{code}`, but it succeeded")),
  >     }
  > ```

- **Fact:** `GRID_UNREACHABLE` is a valid open code: `RefusalCode::from_static` asserts at const evaluation that the
  literal is kebab-case and not a closed code, so `grid-unreachable` would fail the build if it were either. The
  build passes.
  **Source:** `crates/holler-pane/src/error.rs:331-337`
  **Verbatim excerpt:**
  > ```rust
  >     pub const fn from_static(code: &'static str) -> Self {
  >         assert!(
  >             is_valid_code(code) && !is_closed_code(code),
  >             "RefusalCode::from_static: not a kebab-case code, or a closed code"
  >         );
  >         Self(Cow::Borrowed(code))
  >     }
  > ```

- **Fact:** any well-formed open code is a refusal (exit 3), so `class_of(GRID_UNREACHABLE.as_str())` is
  `ErrorClass::Refusal` without any change to `class_of` (AC 3, AC 8(c)).
  **Source:** `crates/holler-pane/src/error.rs:266-273`
  **Verbatim excerpt:**
  > ```rust
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };
  > ```

- **Fact:** a `GridPos` displays row first as `r<row>c<col>`. The fake's messages format cells with it (`{grid}`), and
  suite case 1 asserts `grid.to_string() == "r2c1"`.
  **Source:** `crates/holler-pane/src/grid.rs:68-73`
  **Verbatim excerpt:**
  > ```rust
  > impl fmt::Display for GridPos {
  >     /// The `rRcC` form, row first: `r2c1`.
  >     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
  >         write!(f, "r{}c{}", self.row, self.col)
  >     }
  > }
  > ```

- **Fact:** `GridPos`'s fields are public, so a caller can write a zero cell. The fake's range check therefore treats
  row 0 or column 0 as outside the workspace (`grid-out-of-range`), and T's test passes `GridPos { row: 0, col: 1 }`.
  `GridPos` has no `Ord`, which is why the fake keys its panes by a `(row, col)` tuple.
  **Source:** `crates/holler-pane/src/grid.rs:31-35`
  **Verbatim excerpt:**
  > ```rust
  > /// The fields are public so a cell can be written `GridPos { row: 2, col: 1 }`; a
  > /// zero there is not a cell, and [`GridPos::parse`] and the serde reader both
  > /// refuse it.
  > #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  > pub struct GridPos {
  > ```

- **Fact:** `Argv` derives `Hash` and `Eq` over its element vector, so `FakeProber` keys its script on the argv itself
  (an argv matches only when it is equal element by element).
  **Source:** `crates/holler-pane/src/argv.rs:25-27`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
  > #[serde(transparent)]
  > pub struct Argv(Vec<String>);
  > ```

- **Fact:** `PaneId` derives `Ord`, which the suite's `by_id` uses to compare two pane lists in any order (case 4).
  **Source:** `crates/holler-pane/src/pane.rs:74-77`
  **Verbatim excerpt:**
  > ```rust
  > /// Herdr's own identifier of a pane (opaque to Holler, e.g. `p_12`).
  > #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
  > #[serde(transparent)]
  > pub struct PaneId(String);
  > ```

- **Fact:** the port's own doc names keys `Enter` and `C-c`, while Herdr's names are `enter` and `ctrl+c` (spike
  section 4). This is why the fake treats `enter` in any case as a line break, and the suite presses only `enter`
  (ASSUMPTION 5).
  **Source:** `crates/holler-pane/src/ports.rs:100-104`
  **Verbatim excerpt:**
  > ```rust
  > /// One key to press with `HerdrPort::send_keys`, by the name Herdr uses (for example
  > /// `Enter` or `C-c`).
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(transparent)]
  > pub struct Key(String);
  > ```

- **Fact:** the `HerdrPort::ensure_pane` doc says only "or fail loudly". It does not yet name the workspace-extent
  `grid-out-of-range` that suite case 2 pins, which is why ASSUMPTION 9 hands the doc to #640 (`holler-pane` is out of
  this slice's scope).
  **Source:** `crates/holler-pane/src/ports.rs:125-129`
  **Verbatim excerpt:**
  > ```rust
  > /// Only the adapter converts a [`GridPos`] to Herdr's own order and base.
  > pub trait HerdrPort: Send + Sync {
  >     /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail
  >     /// loudly; never relocates a healthy pane.
  >     fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;
  > ```

- **Fact:** `Prober::run_probe` returns a `ProbeResult`, not a `Result<_, PaneError>`, so a `FaultSwitch` (whose
  faults are `PaneError`s) cannot drive it. `FakeProber` has no fault switch, and a timeout or failure is a scripted
  `ProbeResult::Error` (brief, Decision 9).
  **Source:** `crates/holler-pane/src/ports.rs:208-212`
  **Verbatim excerpt:**
  > ```rust
  > pub trait Prober: Send + Sync {
  >     /// Run `argv` (never through a shell) and look for every string of `expect` in
  >     /// its output, giving up after `timeout` (see [`crate::run_probe`]).
  >     fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult;
  > }
  > ```

- **Fact:** ADR-0021's I4 row binds the fake Herdr to record every `send_text`/`send_keys`. The fake keeps both
  records: the payloads that reached a pane (`sent()`) and every attempt (`faults().calls()`).
  **Source:** `docs/adr/ADR-0021.md:163`
  **Verbatim excerpt:**
  > | I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the switch and reset tests fail if any keystroke was sent (#645). |
