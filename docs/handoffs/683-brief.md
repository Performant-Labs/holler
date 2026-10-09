# Brief: #683 the pane test kit, slice d: `FakeHerdr`, `FakeProber` and the `HerdrPort` conformance suite

Repo: Performant-Labs/holler. Issue: #683 (slice d of #638, epic #633). Rigor: in-session. UI surface: no. Kind: feature
(test kit).

**Branch:** `issue-683-implementation`, based on `e410e9d` (`origin/main`: #637, #669, #670, ADR-0021, #639, #676, the spikes
#635 and #636, and slice a of #638 merged). **Design (D):** N/A. **Decision record:** ADR-0021 (sections 2, 4, 5, 10, 11),
the epic's decision 7 (grid notation) and the spike `docs/research/herdr-api-spike.md`. The issue is the source of truth;
where this brief adjusts its wording to the merged code, the adjustment is listed under "Decisions made in this brief".

**Amended after plan review (A, BLOCK, `docs/handoffs/683/handoff-A.md`):** this change also amends ADR-0021 sections 9
and 10 so `grid-out-of-range` covers `ensure_pane`'s workspace-extent refusal (AC 8). That records the operator's existing
decision (#638 amendment 2026-10-08, grid; #683); it is not a new ruling. A's warns 2 to 5 are folded in below, and warn 6
is accepted for this slice.

## Size check

**Fits one run.** Honest estimate, in Rust lines including the tests (slice a came in about 35% over its estimate of
~1,550, at 2,118 lines, so this is sized with that margin in mind):

| File | Kind | Lines (est.) |
|---|---|---|
| `src/herdr.rs` (`FakeHerdr`, `HerdrOp`, `Placement`, `HerdrVersion`, `Sent`, the constants) | stub filled | ~370 |
| `src/prober.rs` (`FakeProber`, `ProbeCall`) | stub filled | ~90 |
| `src/conformance/herdr.rs` (`HerdrFixture`, the runner, 11 cases) | stub filled | ~340 |
| `tests/herdr_conformance_test.rs` (suite in both modes, case list, fresh-per-case, 10 mutants) | new | ~380 |
| `tests/fake_herdr_test.rs` (the fake's own mechanisms) | new | ~380 |
| `tests/fake_prober_test.rs` | new | ~90 |
| **Total** | 3 modules + 3 test files | **~1,650** |
| `docs/adr/ADR-0021.md` (sections 9 and 10, AC 8) | doc, two lines edited | ~2 (not Rust; outside the total) |

That is above the ~1,050 the #638 split table gave this slice (that figure under-counted the mutation tests), but under the
~2,000-line, six-module limit of one pipeline run, and no file comes near the 600-line warning of `scripts/lint.sh`. No
further split.

## Problem

`holler-pane-testkit` has a fake `PaneStore` and its suite (slice a), but `herdr.rs`, `prober.rs` and
`conformance/herdr.rs` are empty stubs. Every verb story (#644 launch, #645 switch/reset with its I4 "no keystroke" test,
#646 close, #647 doctor) needs a fake Herdr that behaves like the real one where it matters (opaque, stable, never-reused
pane ids; positions as `GridPos` cells; no relocation of a healthy pane; `pane-not-found` after a close; a version gate), and
#640's real adapter needs a conformance suite that fails a transposing implementation. #644 and #663's callers need a fake
`Prober` so no verb waits for #663's real runner.

## Evidence (verbatim, as of `e410e9d`)

The port and its data types (frozen by #637, "provisional" until the spike, which has now reported):
```
crates/holler-pane/src/ports.rs:84-91
/// Where `HerdrPort::ensure_pane` should put a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSpec {
    pub session: String,
    pub workspace: String,
    pub grid: GridPos,
}
crates/holler-pane/src/ports.rs:93-98
pub struct HerdrSnapshot {
    pub panes: Vec<HerdrPane>,
}
crates/holler-pane/src/ports.rs:100-116
/// One key to press with `HerdrPort::send_keys`, by the name Herdr uses (for example
/// `Enter` or `C-c`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(String);
impl Key { pub fn new(name: impl Into<String>) -> Self; pub fn as_str(&self) -> &str; }
crates/holler-pane/src/ports.rs:118-148
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
crates/holler-pane/src/ports.rs:202-212
/// Runs a health probe. [`SystemProber`] is the real one; a test swaps in a fake.
/// ... It returns within the `timeout` it is given (a timeout is
/// [`ProbeResult::Error`], not a [`PaneError`], because the method returns a
/// [`ProbeResult`]). An implementation is `Send + Sync`.
pub trait Prober: Send + Sync {
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult;
}
crates/holler-pane/src/ports.rs:226-235   pub struct Ports<'a> { ..., pub herdr: &'a dyn HerdrPort, ..., pub prober: &'a dyn Prober }
crates/holler-pane/src/pane.rs:74-89     pub struct PaneId(String);  // "opaque to Holler"; PaneId::new(impl Into<String>), as_str(); derives Hash, Ord, Eq
crates/holler-pane/src/pane.rs:91-100    pub struct HerdrPane { pub session: String, pub workspace: String, pub pane_id: PaneId, pub grid: GridPos }
crates/holler-pane/src/probe.rs:16-26    pub enum ProbeResult { Ok, Failed { missing: Vec<String> }, Error(String) }
crates/holler-pane/src/probe.rs:31-36    /// **Stub (#637):** ... This one always answers [`ProbeResult::Error`], never [`ProbeResult::Ok`].
crates/holler-pane/src/argv.rs:25-27     #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)] pub struct Argv(Vec<String>);  // Argv::new, as_slice
crates/holler-pane/src/grid.rs:29-40     /// A cell of the layout grid, 1-based: `row` counts down, `col` counts across.
                                         #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]   // no Ord
                                         pub struct GridPos { pub row: u16, pub col: u16 }    // public fields: a zero can be written
crates/holler-pane/src/grid.rs:68-73     impl fmt::Display for GridPos { ... write!(f, "r{}c{}", self.row, self.col) }
```
**Note: the merged prober is a trait, `Prober::run_probe`, not a free function to fake.** The issue's "fake `run_probe`"
(#638's wording) is therefore `impl Prober for FakeProber`.

The error codes the fakes return (closed set; no new closed code):
```
crates/holler-pane/src/error.rs:414-416  /// `grid-out-of-range`: a row or column of zero, or above `u16::MAX`. ... (#637, `GridPos`.)
                                         GridOutOfRange { what: String }          // doc names only the GridPos guard (ASSUMPTION 9)
crates/holler-pane/src/error.rs:451-453  /// `herdr-version-unsupported`: Herdr reports an API version the adapter does not
                                         /// know; `message` names the version and the supported ones. (#640.)
                                         HerdrVersionUnsupported { message: String },
crates/holler-pane/src/error.rs:454-458  Timeout { op: String },  PaneNotFound { what: String },
crates/holler-pane/src/error.rs:465-467  Unavailable { what: String },   // "the Herdr socket ... cannot be reached"
crates/holler-pane/src/error.rs:409      Usage { message: String },
crates/holler-pane/src/error.rs:491      Refused { code: RefusalCode, message: String },   // an open code an adapter or a verb owns
crates/holler-pane/src/error.rs:316-337  /// A code declared as a constant, checked when the constant is evaluated: a literal that is not
                                         /// kebab-case, or is a closed code, fails the build. ...
                                         pub const fn from_static(code: &'static str) -> Self { assert!(is_valid_code(code) && !is_closed_code(code), ...); ... }
crates/holler-pane/src/error.rs:266-273  pub fn class_of(code: &str) -> ErrorClass   // any other well-formed (open) code is ErrorClass::Refusal
```

The shared mechanism this slice reuses, not copies (slice a):
```
crates/holler-pane-testkit/src/fault.rs:17-23   pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static { fn as_str(self) -> &'static str; }
crates/holler-pane-testkit/src/fault.rs:25-35   pub enum Fault { Wedged, Fail(PaneError) }   // Wedged answers Timeout { op: op.as_str() } at once
crates/holler-pane-testkit/src/fault.rs:54-108  impl FaultSwitch<Op>: new, set, fail_next, set_delay, calls, pub(crate) enter
crates/holler-pane-testkit/src/fault.rs:93-103  pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError>  // record, sleep (lock not held), standing fault, one-shot
crates/holler-pane-testkit/src/conformance/mod.rs:31-41    pub struct CaseFailure { pub case: &'static str, pub detail: String }; pub type Conformance
crates/holler-pane-testkit/src/conformance/mod.rs:47-67    pub(crate) fn run_cases<S, K, C: Copy>(cases: &[(&'static str, C)], fresh: impl FnMut() -> (S, K),
                                                           check: impl FnMut(C, &S) -> Result<(), String>) -> Conformance
crates/holler-pane-testkit/src/conformance/mod.rs:71-100   pub(crate) fn succeeds / expect_code / expect_eq
crates/holler-pane-testkit/src/pane_store.rs:20-44         pub enum PaneStoreOp { Get, ... }  impl PortOp ... "pane_store.get" ...   // the naming to mirror
crates/holler-pane-testkit/src/conformance/pane_store.rs:23-24    type Case = fn(&dyn PaneStore) -> Result<(), String>;
crates/holler-pane-testkit/src/conformance/pane_store.rs:38-40    const CASES: [(&str, Case); 19] = [ ... ];  // the one table runner and *_cases() share
crates/holler-pane-testkit/src/conformance/pane_store.rs:80-84    pub fn pane_store_cases() -> Vec<&'static str>
crates/holler-pane-testkit/src/conformance/pane_store.rs:110-116  pub fn run_pane_store_conformance<S, K, F>(fresh: F) -> Conformance where S: PaneStore, F: FnMut() -> (S, K)
crates/holler-pane-testkit/tests/pane_store_conformance_test.rs:251-263  fn assert_suite_fails_on(broken: Break, case: &str)  // the mutation-check pattern
crates/holler-pane-testkit/src/lib.rs:24,36,39   //! - [`herdr`] and [`prober`] — `FakeHerdr` and `FakeProber` (slice d, #683).   pub mod herdr;   pub mod prober;
crates/holler-pane-testkit/src/conformance/mod.rs:14,18    //! - [`herdr`] — `HerdrPort` (slice d, #683).   pub mod herdr;
crates/holler-pane-testkit/src/herdr.rs:1-4, src/prober.rs:1-2, src/conformance/herdr.rs:1-2   // the stubs, `//!` docs only
crates/holler-pane-testkit/Cargo.toml:14-17  [dependencies] holler-pane = { path = "../holler-pane" }   // only dependency; unchanged here
```

The Herdr facts the fake models (`docs/research/herdr-api-spike.md`, all VERIFIED there unless marked):
```
44-46   A pane is `w<N>:p<M>` (for example `w1:p7`). Ids are opaque, never reused after close, survive resize, zoom, swap,
        close of a sibling, client attach and detach, and a full server restart. A move into another workspace gives a new id.
47-50   Herdr has no grid cell and no row or column index. It reports each pane's rectangle ... x (column axis) first, then
        y (row axis); 0-based ... #640 converts a `GridPos` by ranking (section 6).
51-53   Splits only (`right` or `down`) ... `layout.apply` ... applied to an existing tab it **replaces the tab and kills its panes**.
124     workspace.create ... `{type: "workspace_created", workspace: WorkspaceInfo, tab: TabInfo, root_pane: PaneInfo}`
130     Send keys ... logical key names (`enter`, `esc`, `ctrl+c`, ...)
133     Close ... again: `{"error":{"code":"pane_not_found","message":"pane w1:p5 not found"}}` ... the sibling takes the space;
        a shell that exits closes its pane too (`pane_exited`)
161-163 The number after `p` is **not decimal**: after `w1:p9` came `w1:pA` ... `w1:pF`, `w1:pG` (base 36 on the saved counter
        `next_public_pane_number`). Treat ids as opaque strings; never parse or predict them
228-232 Splits only, no absolute cells. ... `pane.split` accepts only `right` and `down` ... The new pane is always the `second`
        child (right of or below the target).
247-248 **Close** gives the closed pane's space to its sibling subtree ... It can turn a regular grid into an irregular one.
415-416 `ping` returns `{"type":"pong","version":"0.9.1-preview.2026-09-21-0ff0f27e2226","protocol":22, ...}`
427-429 **Differences between versions: UNVERIFIED.** One build is installed on the test machine ... no second version was compared.
450-454 **Which versions #640 should support:** exactly **protocol 22 with schema version 1** (Herdr 0.9.1). ... Refuse any other
        protocol with `herdr-version-unsupported`, and have the message name "Herdr protocol 22 (0.9.1)".
```
The decisions the fake and suite enforce (ADR-0021):
```
docs/adr/ADR-0021.md:163      | I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the
                              switch and reset tests fail if any keystroke was sent (#645). |
docs/adr/ADR-0021.md:184-186  `holler-pane-testkit` ... may depend on `holler-pane` and `serde_json`, and **must not depend on `holler-cli`**
docs/adr/ADR-0021.md:377      | `grid-out-of-range` | Refusal (3) | The `GridPos` guard declined a row or column outside its bounds. |
docs/adr/ADR-0021.md:421      - **`grid-out-of-range`:** a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535.
                              // both name only the GridPos guard; AC 8 adds ensure_pane's workspace-extent condition
docs/adr/ADR-0021.md:424-430  The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base.
                              `HerdrPort::ensure_pane` reaches a position by right and down splits (B4, `plan_splits`, #640) or fails
                              loudly, and never relocates a healthy pane. ... An unknown version is `herdr-version-unsupported`, and the
                              message names the supported ones
```
Lint and test conventions:
```
Cargo.toml:19-30          unwrap_used, expect_used, panic, unreachable, cognitive_complexity, too_many_lines,
                          struct_excessive_bools = "deny"; [workspace.lints.rust] dead_code = "deny"
clippy.toml:6-7           cognitive-complexity-threshold = 15; too-many-lines-threshold = 100
scripts/lint.sh:43-52     warn at 600 lines per .rs file, fail at 900
crates/holler-pane-testkit/tests/pane_store_conformance_test.rs:1   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
CHANGELOG.md:8-10         ## [Unreleased] / ### Enhancements; the last entry ends at :89 (#676)
```

## Dependency direction

`holler-pane-testkit` -> `holler-pane` only (unchanged). No manifest changes: everything needed (`HerdrPort`, `HerdrSpec`,
`HerdrSnapshot`, `HerdrPane`, `Key`, `PaneId`, `GridPos`, `PaneError`, `error::{RefusalCode, class_of}`, `Prober`,
`ProbeResult`, `Argv`) is in `holler-pane`. The tests reach `holler_pane` through the crate's normal dependency, as slice a's
do. `cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'` prints nothing.

## Public API (exact; T writes tests against these, F implements them)

No flat re-exports; every item is reached by its module path (`holler_pane_testkit::herdr::FakeHerdr`). `lib.rs` and
`conformance/mod.rs` are not edited.

```rust
// crates/holler-pane-testkit/src/herdr.rs

/// The version string of the one Herdr build the spike tested (protocol 22).
pub const PROTOCOL_22_VERSION: &str = "0.9.1-preview.2026-09-21-0ff0f27e2226";
/// What a `herdr-version-unsupported` message names as supported (spike lines 450-453). Provisional test vocabulary
/// (ASSUMPTION 7): verb tests compare against this constant, never the literal.
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";
/// The version string of the fake's unsupported build. Invented: no second build was compared (ASSUMPTION, below).
pub const UNSUPPORTED_VERSION: &str = "99.0.0-fake";
/// The open code split-only mode refuses an absolute placement with (an `ErrorClass::Refusal`). Provisional test
/// vocabulary (ASSUMPTION 7): verb tests compare against `GRID_UNREACHABLE.as_str()`, never the literal.
pub const GRID_UNREACHABLE: RefusalCode = RefusalCode::from_static("grid-unreachable");

/// A method of the `HerdrPort`, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HerdrOp { EnsurePane, SendText, SendKeys, Read, Close, Snapshot, Version }
impl PortOp for HerdrOp { /* "herdr.ensure_pane", "herdr.send_text", "herdr.send_keys", "herdr.read",
                             "herdr.close", "herdr.snapshot", "herdr.version" */ }

/// How `ensure_pane` may place a new pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Any free cell inside the workspace (the default).
    #[default] Absolute,
    /// Only a split of an existing pane: see "Fake behaviour".
    SplitOnly,
}

/// Which Herdr build `version()` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HerdrVersion {
    /// Protocol 22: `version()` is `Ok(PROTOCOL_22_VERSION)` (the default).
    #[default] Protocol22,
    /// A build the adapter does not know: `version()` is `herdr-version-unsupported`.
    Unsupported,
}

/// One `send_text` or `send_keys` that reached a pane: its payload. A failed attempt (unknown or closed pane, or stopped
/// by a fault) is not here; every attempt is in `faults().calls()` as `HerdrOp::SendText`/`HerdrOp::SendKeys`, and that
/// log is the check for I4's "no keystroke".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    Text { pane: PaneId, text: String },
    Keys { pane: PaneId, keys: Vec<Key> },
}

/// An in-memory `HerdrPort` serving one Herdr session. Not `Clone`: share it behind an `Arc`.
pub struct FakeHerdr { /* private: Mutex<state>, FaultSwitch<HerdrOp> */ }
impl FakeHerdr {
    /// Serves the session `session`, with no workspace, `Placement::Absolute`, `HerdrVersion::Protocol22`.
    pub fn new(session: &str) -> Self;
    /// Declare a workspace of `rows` by `cols` cells, with no pane. Workspaces are numbered 1, 2, ... in declaration
    /// order. A name already declared is `usage`.
    pub fn with_workspace(self, name: &str, rows: u16, cols: u16) -> Result<Self, PaneError>;
    pub fn set_placement(&self, placement: Placement);
    pub fn set_version(&self, version: HerdrVersion);
    /// The fault switch of every port method and the log of the calls made through the port.
    pub fn faults(&self) -> &FaultSwitch<HerdrOp>;
    /// The payloads of every `send_text` and `send_keys` that reached a pane, oldest first. Omits failed sends: to assert
    /// I4's "no keystroke", check that `faults().calls()` holds no `HerdrOp::SendText` or `HerdrOp::SendKeys`, not that
    /// `sent()` is empty (a verb that tried to type and failed would pass that).
    pub fn sent(&self) -> Vec<Sent>;
    /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is free, its id is never reused, and
    /// every later call naming it is `pane-not-found`. Bypasses the faults and the call log. `pane-not-found` if unknown.
    pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError>;
    /// The pane's program prints `text` (what `read` then shows). Bypasses the faults and the call log.
    /// `pane-not-found` if unknown.
    pub fn print(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;
}
impl HerdrPort for FakeHerdr { /* see "Fake behaviour" */ }

// crates/holler-pane-testkit/src/prober.rs

/// One run of the fake probe, as it was called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCall { pub argv: Argv, pub expect: Vec<String>, pub timeout: Duration }

/// A `Prober` that answers a scripted `ProbeResult` per argv. Not `Clone`: share it behind an `Arc`.
pub struct FakeProber { /* private: Mutex<{ HashMap<Argv, ProbeResult>, Vec<ProbeCall> }> */ }
impl FakeProber {
    /// Nothing scripted: every run answers `ProbeResult::Error` naming the argv, never `Ok`.
    pub fn new() -> Self;
    /// Answer `result` for every run of exactly `argv` (element-wise equal). Scripting the argv again replaces it.
    pub fn script(&self, argv: Argv, result: ProbeResult);
    /// Every run, oldest first, with its `expect` and `timeout`.
    pub fn calls(&self) -> Vec<ProbeCall>;
}
impl Default for FakeProber { /* = new() */ }
impl Prober for FakeProber { /* record the call, then the scripted result or the Error */ }

// crates/holler-pane-testkit/src/conformance/herdr.rs

/// What one case runs against: a port serving the scratch Herdr session `session`, in which the workspace `workspace`
/// is 2 rows by 1 column and holds no pane, or only its root pane at r1c1 (a real Herdr workspace is created with one).
pub struct HerdrFixture<H> { pub port: H, pub session: String, pub workspace: String }

/// The case ids `run_herdr_conformance` runs, in order.
pub fn herdr_cases() -> Vec<&'static str>;
/// Run every case of `herdr_cases`, in order, each against a fresh fixture, and return every case that did not hold.
/// `fresh` is called once per case and returns the fixture and a guard kept alive for that case only (`()` for the
/// fake; the scratch server's handle for #640). The fixture is dropped before its guard.
pub fn run_herdr_conformance<H, K, F>(fresh: F) -> Conformance
where H: HerdrPort, F: FnMut() -> (HerdrFixture<H>, K);
```

How each implementation runs the suite (doc comment on `run_herdr_conformance`, in a `text` fence, not a doctest):
```text
// the fake:
run_herdr_conformance(|| {
    let port = FakeHerdr::new("scratch").with_workspace("scratch", 2, 1).expect("workspace");
    (HerdrFixture { port, session: "scratch".into(), workspace: "scratch".into() }, ())
})
// the adapter (#640, opt-in, #[ignore]): a fresh scratch Herdr session per case whose workspace the adapter knows to be
// 2 rows by 1 column; the session's server handle as the guard; never the default or a live session.
```

Library code returns `Result` everywhere: no `unwrap`, `expect`, `panic!`, `unreachable!` or `assert!` in `src/` (the
`assert!` inside `RefusalCode::from_static` runs at compile time for the `const`). A poisoned lock is taken with
`unwrap_or_else(PoisonError::into_inner)`, as `fault.rs:105-107` does.

## Fake behaviour (`FakeHerdr`)

Every port method calls `faults().enter(op)` first; an error from it is returned and nothing changes (slice a's contract).
Then one `Mutex` guards the state: the session name, the workspaces in declaration order (name, number, rows, cols, the next
pane number, the panes), the placement, the version and the `sent` log. A pane is `{ id, grid, screen: String }`.

- **`ensure_pane(spec)`**, checked in this order:
  1. `spec.session` is not the fake's session, or `spec.workspace` is not declared: `PaneError::Unavailable { what }`
     naming the session and workspace (ASSUMPTION 6).
  2. Range: `1 <= row <= rows` and `1 <= col <= cols`, else `PaneError::GridOutOfRange { what }` naming the cell (`r1c2`)
     and the workspace's size ("2 rows by 1 column"). A zero (`GridPos` fields are public) is out of range too.
  3. Occupied: the cell already holds a pane: return that pane's `HerdrPane`, change nothing, in either placement mode.
     Never move, replace or renumber another pane (spike 51-53).
  4. `Placement::SplitOnly`: accept the cell only if the workspace has no pane and the cell is `r1c1` (its root), or a pane
     sits immediately left of it (`row, col - 1`: a `right` split) or immediately above it (`row - 1, col`: a `down`
     split). Anything else is `PaneError::Refused { code: GRID_UNREACHABLE, message }`, the message naming the cell and
     saying Herdr places a pane only by a right or down split of an existing one (spike 228-232).
  5. Mint the id `w<N>:p<M>`: N is the workspace's number and M its pane counter, which starts at 1, is written in base 36
     with upper-case letters (`1`..`9`, `A`..`Z`, then `10`), and only ever goes up (spike 161-163), so an id is never
     reused after a close or a vanish. A counter overflow is `unavailable` and creates nothing (no wrapping). Insert the
     pane with an empty screen; return `HerdrPane { session, workspace, pane_id, grid }`.
- **`close(pane)`**: remove it; an unknown or already-closed id is `PaneError::PaneNotFound { what: <the id> }` (spike 133).
  The other panes keep their ids and their cells (ASSUMPTION 4).
- **`send_text(pane, text)`**: append `text` to the pane's screen (a shell echoes what is typed) and push
  `Sent::Text`. **`send_keys(pane, keys)`**: append `"\n"` for each key whose name is `enter` in any case, nothing for any
  other key, and push one `Sent::Keys` with all the keys. Both: an unknown id is `pane-not-found` and records nothing in
  `sent` (the attempt is still in `faults().calls()`). So `sent()` omits failed sends, and `faults().calls()` is the check
  for I4's "no keystroke" (#645 asserts no `SendText`/`SendKeys` op there); both docs say so.
- **`read(pane, max_lines)`**: the last `max_lines` lines of the screen (`str::lines`), joined with `"\n"`, no trailing
  newline; `max_lines == 0` gives `""`. Unknown id: `pane-not-found`.
- **`snapshot()`**: every pane, workspaces in declaration order, then by row, then by column, each as
  `HerdrPane { session, workspace, pane_id, grid }`. The fake never reports another position for a pane than the cell it
  was created in.
- **`version()`**: `Protocol22` answers `Ok(PROTOCOL_22_VERSION)`; `Unsupported` answers
  `PaneError::HerdrVersionUnsupported { message }` with a message naming `UNSUPPORTED_VERSION` and `SUPPORTED_VERSIONS`.
  The selected version affects `version()` only (ASSUMPTION 8).
- **Faults:** wedged, `Fail(e)`, one-shot and slow calls come from the shared `FaultSwitch<HerdrOp>`; the vanished pane is
  `vanish`. `vanish` and `print` bypass the faults and the call log, as `FakePaneStore::concurrent_*` do.

`FakeProber` takes no `FaultSwitch`: `PortOp` faults answer a `PaneError`, and a `Prober` answers a `ProbeResult`, so a
timeout or a failure is simply a scripted `ProbeResult::Error`. An unscripted argv answers
`ProbeResult::Error(format!("no probe scripted for {:?}", argv.as_slice()))`, so a test that forgot to script cannot pass a
probe (the same reason `probe.rs:31-36` never answers `Ok`).

### ASSUMPTION comments for #640 to confirm (each as an `// ASSUMPTION (#640):` comment at the code it concerns)

1. **A workspace's size.** Herdr has no grid (spike 47-50); the fake is told the size. How the real adapter learns it (the
   profile's extent, configuration, the tree walk) is #640's. The suite's `r1c2` case needs the adapter under test to know
   the fixture workspace is 2 rows by 1 column. (In `herdr.rs` at `with_workspace`, and in `conformance/herdr.rs` at
   `HerdrFixture`.)
2. **Versions beyond protocol 22** are unverified (spike 427-429); `UNSUPPORTED_VERSION` is invented. (At the constants.)
3. **The root pane.** A real workspace is created with a root pane (spike 124); the fake's starts empty. The suite's cases
   hold either way. (At `with_workspace` and `HerdrFixture`.)
4. **After a close** real Herdr gives the space to the sibling (spike 247-248), so the cell the tree walk reads for a
   surviving pane can change; the fake keeps every pane in its cell, and the suite asserts ids, not positions, after a
   close. (At `close`.)
5. **Key names.** The port's doc says `Enter`/`C-c`; Herdr's logical names are `enter`/`ctrl+c` (spike 130). The fake
   treats `enter` in any case as a line break. (At `send_keys`, and repeated at suite case 10, which presses Herdr's
   own name `Key::new("enter")` so #640's real adapter needs no case-folding the port does not ask for.)
6. **A session or workspace the fake does not serve** is `unavailable`; whether the adapter creates a missing workspace
   (`workspace.create`) is #640's. (At `ensure_pane`.)
7. **`GRID_UNREACHABLE` (`grid-unreachable`) and `SUPPORTED_VERSIONS` are provisional test vocabulary:** the fake's own
   values, raised or named by no verb or adapter yet, and tested by no suite case, so they do not bind #640 today. A merged
   code is never renamed (ADR-0021 section 9), so a verb test (#644 and later) compares against the constants
   (`herdr::GRID_UNREACHABLE.as_str()`, `herdr::SUPPORTED_VERSIONS`), never the literal. #640 either declares the same
   values in its own file and asserts in its dev-tests that they equal the test kit's, or the fake takes #640's values
   before any verb story pins them. (At `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS`.)
8. **Only `version()` refuses** an unsupported build; whether the adapter also refuses every other call after a failed
   version check is #640's. (At `version`.)
9. **`ensure_pane` outside the workspace is `grid-out-of-range`** (ADR-0021 sections 9 and 10, as AC 8 amends them; #638
   amendment 2026-10-08, grid). The `holler-pane` docs still describe less: the `PaneError::GridOutOfRange` variant doc
   (`error.rs:414-416`, "a row or column of zero, or above `u16::MAX`") and the `HerdrPort::ensure_pane` doc
   (`ports.rs:127-128`, "or fail loudly"). `holler-pane` is out of scope here; #640 must update both docs to match when it
   finalizes `HerdrPort`. (In `conformance/herdr.rs` at suite case 2.)

## Conformance cases: `run_herdr_conformance`

Each case gets a fresh fixture (`HerdrFixture`: a 2-row by 1-column workspace with no pane, or only its root at `r1c1`).
"The workspace's panes" means the panes of `snapshot()` whose `session` and `workspace` are the fixture's. Every case
ensures `r1c1` before any other cell, so the suite also runs against split-only placement. One function per case, in one
`const CASES: [(&str, Case); 11]` table that the runner iterates and `herdr_cases()` lists (slice a's pattern); `Case` is
`fn` over a crate-private view `{ port: &dyn HerdrPort, session: &str, workspace: &str }`. The cases reuse `run_cases`,
`succeeds`, `expect_code` and `expect_eq` from `conformance/mod.rs`.

| # | Case id | What it asserts | Mutant it must catch |
|---|---|---|---|
| 1 | `ensure-r2c1-reads-back-as-r2c1` | `ensure r1c1` (a), then `ensure r2c1` (b) succeeds; `b.grid` is row 2, col 1 and `b.session`/`b.workspace` are the spec's; the snapshot lists `b.pane_id` with `grid == GridPos { row: 2, col: 1 }` and `grid.to_string() == "r2c1"`. | `Transposes`, `ReadsBackTransposed` |
| 2 | `ensure-r1c2-is-grid-out-of-range` | `ensure r1c2` is `grid-out-of-range`; `ensure r3c1` is `grid-out-of-range`; the workspace's panes are equal before and after. Carries ASSUMPTION 9. | `Transposes` |
| 3 | `ensure-is-idempotent` | `ensure r1c1` twice returns equal `HerdrPane`s; the workspace has exactly one pane at `r1c1`, with that id. | `CreatesOnEveryEnsure` |
| 4 | `ensure-never-moves-another-pane` | a = `r1c1`, b = `r2c1`, then `ensure r1c1` again returns a; the workspace's panes are exactly a and b (any order), each in its cell. | `RebuildsTheWorkspace` |
| 5 | `closed-id-is-never-reused` | a = `r1c1`, b = `r2c1`, `close b`, c = `ensure r2c1`: `c.pane_id` differs from a's and b's. | `IdsFromPosition` |
| 6 | `ids-unique-and-stable-when-a-sibling-closes` | a = `r1c1`, b = `r2c1`, ids differ; `close a`; the workspace's panes include `b.pane_id` and not `a.pane_id` (ids only, ASSUMPTION 4). | `IdsFromSnapshotOrder` |
| 7 | `close-unknown-is-pane-not-found` | `close(PaneId::new("w999:p999"))` is `pane-not-found`. | `CloseIsIdempotent` |
| 8 | `close-twice-is-pane-not-found` | a = `r1c1`, b = `r2c1`; `close b` is `Ok`; `close b` again is `pane-not-found` (the workspace keeps a, so no case empties a real workspace). | `CloseIsIdempotent` |
| 9 | `calls-on-a-closed-pane-are-pane-not-found` | a = `r1c1`, b = `r2c1`, `close b`; `send_text(b, "x")`, `send_keys(b, [Key::new("enter")])` and `read(b, 1)` are each `pane-not-found`. | `ClosedPaneStillAnswers` |
| 10 | `read-returns-at-most-max-lines` | a = `r1c1`; five times `send_text(a, "echo line<i>")` then `send_keys(a, [Key::new("enter")])` (Herdr's own key name, spike 130; ASSUMPTION 5 repeated here); `read(a, 2)` succeeds and has at most 2 lines (`str::lines().count()`). | `ReadIgnoresMaxLines` |
| 11 | `version-is-reported` | `version()` is `Ok` with a non-empty string holding no `\n` or `\r`. | the fake set to `HerdrVersion::Unsupported` |

The suite presses keys only by Herdr's own names (`enter`), never `Enter`; the fake's own tests (AC 3) may keep `Enter`
to cover its case-insensitivity.
Case 10 types into a scratch pane through the port; that is the suite exercising the port, not a verb (I4 binds verbs).
Against the fake the screen then holds five lines, so a `read` that ignores `max_lines` returns five and fails.

## Acceptance criteria

Test names are what T authors (RED first). Every test file starts with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #683`.

1. **The fake passes its suite, in both placements.** `tests/herdr_conformance_test.rs`:
   - `the_fake_passes_the_herdr_conformance_suite`: the `text` example above is `Ok(())`.
   - `the_fake_in_split_only_mode_passes_the_suite`: the same with `set_placement(Placement::SplitOnly)` is `Ok(())`.
   - `the_suite_runs_the_documented_cases`: `herdr_cases()` equals the 11 ids of the table, in order.
   - `the_suite_builds_a_fresh_fixture_per_case`: `fresh` is called 11 times and 11 guards are dropped by the end.
2. **Mutation check: each mutant fails, on its named case.** Same file. Each mutant is a test-local wrapper
   `Mutant { inner: FakeHerdr, broken: Break }` implementing `HerdrPort` (no mutant switch in the fake), run in absolute
   placement; `assert_suite_fails_on(broken, case)` asserts `Err` whose failures include `case` and every `detail` is
   non-empty (slice a's helper, `pane_store_conformance_test.rs:251-263`):
   - `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone` (`Break::Nothing` is `Ok(())`).
   - `a_transposing_herdr_fails` (`ensure_pane` swaps row and col before calling the fake and swaps the returned grid
     back) -> `ensure-r1c2-is-grid-out-of-range` (it also fails `ensure-r2c1-reads-back-as-r2c1`; assert both).
   - `a_herdr_that_reads_back_transposed_fails` (`snapshot` swaps each pane's row and col) -> `ensure-r2c1-reads-back-as-r2c1`.
   - `a_herdr_that_creates_on_every_ensure_fails` (an occupied cell's pane is closed and a new one made) -> `ensure-is-idempotent`.
   - `a_herdr_that_rebuilds_the_workspace_fails` (creating a pane first closes and re-ensures every other pane of the
     workspace, so they get new ids: `layout.apply` onto an existing tab) -> `ensure-never-moves-another-pane`.
   - `a_herdr_with_position_ids_fails` (outward id `"<workspace>:<grid>"`, mapped back through the snapshot) ->
     `closed-id-is-never-reused`.
   - `a_herdr_that_renumbers_on_close_fails` (outward id `"n<index in the snapshot>"`, mapped back by index) ->
     `ids-unique-and-stable-when-a-sibling-closes`.
   - `a_herdr_whose_close_is_idempotent_fails` (`close` of an unknown id is `Ok`) -> `close-twice-is-pane-not-found` and
     `close-unknown-is-pane-not-found` (assert both).
   - `a_herdr_whose_closed_panes_still_answer_fails` (ids it closed answer `Ok(())`/`Ok("")`) ->
     `calls-on-a-closed-pane-are-pane-not-found`.
   - `a_herdr_that_ignores_max_lines_fails` (`read(pane, usize::MAX)`) -> `read-returns-at-most-max-lines`.
   - `an_unsupported_herdr_fails_the_version_case` (the fake with `set_version(HerdrVersion::Unsupported)`, no wrapper) ->
     `version-is-reported`.
3. **The fake's own mechanisms.** `tests/fake_herdr_test.rs`:
   - `a_workspace_bounds_ensure_pane`: in a 2x3 workspace `r2c3` is `Ok`; `r3c1`, `r1c4` and `GridPos { row: 0, col: 1 }`
     are `grid-out-of-range`, and the message names the cell.
   - `an_undeclared_workspace_or_another_session_is_unavailable` and `declaring_a_workspace_twice_is_usage`.
   - `ids_are_minted_per_workspace_in_base_36_and_never_reused`: in a 1x12 workspace, `c1`..`c12` get `w1:p1`..`w1:p9`,
     `w1:pA`, `w1:pB`, `w1:pC`; after `close w1:p3`, `ensure r1c3` gets `w1:pD`; the first pane of a second workspace is
     `w2:p1`; in a 1x36 workspace the 36th pane is `w1:p10`.
   - `ensure_on_an_occupied_cell_returns_the_occupant`: equal `HerdrPane`, no new pane, other panes unchanged.
   - `closing_a_pane_leaves_its_siblings_ids_and_cells`.
   - `split_only_mode_refuses_an_absolute_placement`: 2x2, empty: `r2c2` is `Refused` with code
     `GRID_UNREACHABLE.as_str()` and `class_of(code) == ErrorClass::Refusal`; `r1c1` then is `Ok`; `r2c2` is still refused;
     `r1c2` (right of `r1c1`) and then `r2c2` (below `r1c2`) are `Ok`; `r2c1` (below `r1c1`) is `Ok`.
   - `split_only_mode_starts_an_empty_workspace_at_r1c1`: `r1c2` and `r2c1` on an empty workspace are refused.
   - `split_only_mode_checks_the_range_first`: `r3c1` in a 2x1 is `grid-out-of-range`, not `grid-unreachable`.
   - `each_selectable_version_is_observable`: default `Ok(PROTOCOL_22_VERSION)`; `Unsupported` gives
     `herdr-version-unsupported` whose message contains `SUPPORTED_VERSIONS` and `UNSUPPORTED_VERSION`; back to
     `Protocol22` gives `Ok` again; `ensure_pane` and `snapshot` work under either.
   - `a_wedged_herdr_times_out_every_method`: each of the seven methods is `Timeout { op: "herdr.<method>" }`; after
     `set(None)` the snapshot is empty (nothing was created).
   - `a_failed_ensure_creates_nothing` (`fail_next(HerdrOp::EnsurePane, ..)`) and `a_slow_call_takes_at_least_the_delay`
     (lower bound only).
   - `a_vanished_pane_is_pane_not_found_everywhere`: after `vanish(b)`, `read`/`send_text`/`send_keys`/`close` of b are
     `pane-not-found`, the snapshot lacks b, a keeps its id; `ensure` at b's cell mints a new id; `vanish` of an unknown id
     is `pane-not-found`; `vanish` adds nothing to `calls()`.
   - `send_text_and_send_keys_are_recorded_in_order`: `sent()` is `[Text { .. }, Keys { .. }]` in call order; a send to an
     unknown pane, and one stopped by `fail_next`, is in `calls()` but not in `sent()`; `ensure_pane`, `read` and `print` add nothing to `sent()`.
   - `read_returns_the_last_lines_of_the_screen`: after `print(a, "a\nb\nc\n")`, `read(a, 2) == "b\nc"`,
     `read(a, 10) == "a\nb\nc"`, `read(a, 0) == ""`; `send_text(a, "ls")` plus `send_keys(a, [Enter])` adds the line `ls`.
   - `snapshot_lists_panes_by_workspace_then_row_then_col`.
   - `port_op_names_are_herdr_dot_method` and `the_fake_is_send_and_sync_and_a_dyn_herdr_port`.
4. **The fake probe.** `tests/fake_prober_test.rs`:
   - `a_scripted_argv_answers_each_probe_result`: three argvs scripted `Ok`, `Failed { missing: vec!["qwen38".into()] }`,
     `Error("down".into())` each answer theirs, through `&dyn Prober`.
   - `an_unscripted_argv_answers_error_never_ok` (also an argv that differs from a scripted one in one element).
   - `scripting_an_argv_again_replaces_its_result`.
   - `every_run_is_recorded_with_its_expect_and_timeout` (`calls()` in order, unscripted runs included).
   - `the_fake_prober_is_send_and_sync`.
5. **No other crate changes.** `git diff --name-only origin/main...HEAD` lists only Blast-radius paths; `Cargo.toml`
   files and `Cargo.lock` are unchanged; slice a's tests pass unchanged.
6. **CHANGELOG.** One entry under `## [Unreleased]` / `### Enhancements`, after the #676 entry (`CHANGELOG.md:89`): the pane
   test kit gains a fake Herdr (panes placed in row-and-column cells of a declared workspace, ids that are never reused and
   do not change when another pane closes, a split-only mode, a supported and an unsupported version, a wedged server, a
   pane whose shell exited, a slow call, and a record of everything typed into a pane) and a fake health probe answering a
   scripted result per command; a conformance suite the Herdr adapter runs against itself rejects an adapter that swaps rows
   and columns; test code only, nothing a user runs changes; link [#683](https://github.com/Performant-Labs/holler/issues/683).
7. **Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`, `bash scripts/test-hooks.sh` pass. Every new
   or changed `.rs` file passes `rustfmt --check --edition 2021`. No `.rs` file reaches 600 lines; no function exceeds 100
   lines or cognitive complexity 15 (one function per case; `ensure_pane` split into helpers: lookup, range, split rule,
   mint). No `unwrap`/`expect`/`panic!`/`unreachable!`/`assert!` in `src/`; no `dead_code` allow.
8. **ADR-0021 records `ensure_pane`'s `grid-out-of-range`** (F makes the doc edit in this PR; S checks it). This records the
   operator's existing decision (#638 amendment 2026-10-08, grid; #683), not a new ruling, and follows #639 and #676, which
   amended ADR-0021 in the same PR as their code. Minimal wording, in the style of the neighbouring lines:
   - (a) Section 10's `grid-out-of-range` bullet (line 421) keeps its text and adds the second condition, for example:
     "a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535; or, from `HerdrPort::ensure_pane`, a cell outside its
     workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort` conformance suite
     pins (#638 amendment 2026-10-08, grid; #683)."
   - (b) Section 9's class-table reason for `grid-out-of-range` (line 377) names both sources, for example: "The `GridPos`
     guard declined a row or column outside its bounds, or the Herdr port declined a cell outside the workspace."
   - (c) Nothing else changes: the class stays Refusal (exit 3), the closed code list (`ALL_CODES`) and `class_of` and its
     tests are untouched, and no other ADR line is edited (no new "Decisions taken" item: this is not a new decision).
     `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` shows exactly those two lines changed.

## Files

Filled (stubs from slice a; keep each `//!` doc, rewritten to describe the module instead of the stub):
`crates/holler-pane-testkit/src/herdr.rs`, `src/prober.rs`, `src/conformance/herdr.rs`.
New: `crates/holler-pane-testkit/tests/herdr_conformance_test.rs`, `tests/fake_herdr_test.rs`, `tests/fake_prober_test.rs`.
Changed: `CHANGELOG.md`, `docs/adr/ADR-0021.md` (sections 9 and 10, AC 8).

## Extend vs new

- **Extend:** fill slice a's stub files (no `lib.rs` or `conformance/mod.rs` edit); implement the frozen `HerdrPort` and
  `Prober` traits; reuse `FaultSwitch`/`Fault`/`PortOp` for every Herdr fault and the call log; reuse `run_cases`,
  `succeeds`, `expect_code`, `expect_eq` and `Conformance`/`CaseFailure` for the suite; return the closed `PaneError`
  variants (`GridOutOfRange`, `PaneNotFound`, `HerdrVersionUnsupported`, `Timeout`, `Unavailable`, `Usage`) and one open code
  built with `RefusalCode::from_static` (the mechanism `error.rs:316-337` provides for exactly this); format positions with
  `GridPos`'s own `Display`; key the prober on `Argv`'s own `Hash`/`Eq`.
- **New, no parallel path:** no second fault mechanism, case runner, error helper or grid formatter. `FakeHerdr` needs no
  change feed (`HerdrPort` has no `watch`), so `feed.rs` is not touched. The base-36 formatter and the screen's line
  slicing are the only new helpers, private to `herdr.rs`.
- **Not reused, on purpose:** `fixture::sample_pane` (a `Pane` record; the Herdr fake deals in `HerdrPane` and needs none).

## Decisions already made (operator, epic, ADR)

- The fake works in `GridPos` cells only; only #640 knows Herdr's order and base (epic decision 7; ADR-0021:424-427).
- Workspaces are declared with a size, `ensure_pane` outside it is `grid-out-of-range`, `r2c1` is accepted and `r1c2`
  refused in a 2x1 workspace so a transposing implementation fails (#638 amendments 2026-10-08 grid and review; #683).
  This extends what ADR-0021 section 10 says `grid-out-of-range` means (there, only the `GridPos` guard's zero or
  over-65535), so this change records it there and in section 9's reason (AC 8).
- Ids `w<N>:p<M>`, base 36, never reused, stable across a sibling's close; an occupied cell returns its pane; split-only
  mode; two versions with protocol 22's string and the "Herdr protocol 22 (0.9.1)" message; faults wedged, vanished, slow;
  `send_text`/`send_keys` recorded for I4 (#683, from the spike and ADR-0021:163).
- `FakeProber` returns a scripted `ProbeResult` per argv (#683; #638 amendment 2026-10-08 features).
- The testkit depends on `holler-pane` only (ADR-0021:184-186).

## Decisions made in this brief

1. **The prober is the merged `Prober` trait**, so the fake is `impl Prober for FakeProber`; no free function is faked.
2. **The suite's fixture carries its names:** `HerdrFixture { port, session, workspace }`, because #640's scratch session has a
   random name and the suite cannot hard-code one. The workspace may hold its root pane at `r1c1`; the cases are written
   to pass either way.
3. **Every case ensures `r1c1` first** and never closes a workspace's last pane, so the suite runs against split-only
   placement and against real Herdr's root-pane workspaces. The fake passes it in both placements. This is the
   "split-only-mode case" #640's acceptance names: case 1 run against a split-only Herdr. Refusing an absolute placement is
   a test of the fake's mode, not a port case, because a real adapter may reach more cells with several splits.
4. **The split-only refusal is an open code, `grid-unreachable`** (a refusal, exit 3), not a closed code: the closed list
   is frozen, and `Refused` is where an adapter's own code goes. It is provisional test vocabulary, compared by constant,
   never by literal (ASSUMPTION 7).
5. **The fake starts every workspace empty** (no root pane), so verb tests see an empty Herdr; ASSUMPTION 3 records the
   difference.
6. **After a close the suite checks ids, not positions** (ASSUMPTION 4), and the fake keeps every pane in its cell.
7. **The fake's screen echoes what is typed** (`enter` as a line break) and `print` adds a program's output, so `read` has
   lines to cut; the suite's `read` case types five lines through the port.
8. **The unsupported version affects `version()` only**; a session or workspace the fake does not serve is `unavailable`;
   a duplicate workspace name is `usage`. All three are the fake's own choices, marked for #640.
9. **`FakeProber` has no fault switch** (a `Prober` answers a `ProbeResult`, not a `PaneError`); an unscripted argv
   answers `Error`, never `Ok`.
10. **Blast radius:** the issue lists `herdr.rs`, `prober.rs` and `CHANGELOG.md`; the suite's stub `conformance/herdr.rs`
    (also created by slice a for this slice) and three new test files are added, within #638's `crates/holler-pane-testkit/**`.
    `docs/adr/ADR-0021.md` is added for AC 8 (plan review, A's block).
11. **The copied test scaffolding is accepted for this slice** (A's warn 6): `herdr_conformance_test.rs` copies slice a's
    `CaseGuard` and `assert_suite_fails_on`, because each integration-test file is its own crate. A later slice (#682,
    #684) that would copy them again moves `CaseGuard` and an `assert_fails_on(result: Conformance, case)` into
    `crates/holler-pane-testkit/tests/support/mod.rs` instead.

## Out of scope

Slices b, c and e (#681, #682, #684); any change to `holler-pane` (including the provisional `HerdrPort` and its key-name
doc), `holler-hub`, `holler-cli`, the adapter crates or any manifest; the real adapter and `plan_splits` (#640); Herdr's
rectangles, events, `layout.apply`, `pane.move` and restart behaviour (the fake has no restart); the real probe runner
(#663); running the suite against a real Herdr (#640, opt-in, scratch session only).

## Test plan

RED (T): write the three test files against the API above. They fail to build (`FakeHerdr`, `FakeProber`,
`run_herdr_conformance` and the rest do not exist); confirm with `cargo test -p holler-pane-testkit` that every error is a
missing item (E0432/E0433/E0425), not a typo, and that slice a's two test files still compile on their own
(`cargo test -p holler-pane-testkit --test fake_pane_store_test --test pane_store_conformance_test`). The mutants cannot be
run until the suite exists: T-green runs each and repairs any that fails on a different case than its named one.
GREEN (F): `herdr.rs`, then `conformance/herdr.rs`, then `prober.rs`, then the CHANGELOG and the ADR-0021 edit of AC 8;
then the guards of AC 7.
A (anti-duplication): no second fault switch, runner or `expect_*` helper; the cases call the shared ones.

## Risks

- **A mutant fails on another case than the one named** (for example `RebuildsTheWorkspace` also failing case 6). The
  assertion is "includes the named case", so extra failures are fine; a mutant that does not fail its case means the case
  is too weak and is strengthened, not the mutant weakened.
- **Real Herdr differs from the fake** where the spike says so (root pane, sibling takes the space, key names). Each is an
  ASSUMPTION comment, and the suite avoids depending on them; #640 confirms.
- **Timing:** `a_slow_call_takes_at_least_the_delay` asserts a lower bound only.
- **`tests/herdr_conformance_test.rs` is the largest file** (~380 lines with eleven mutant breaks); if it nears 600, move the
  id-mapping mutants' translation helper into `tests/herdr_mutants/mod.rs` (a test-only module), not into `src/`.

## Blast radius

`crates/holler-pane-testkit/src/herdr.rs`, `src/prober.rs`, `src/conformance/herdr.rs`,
`crates/holler-pane-testkit/tests/{herdr_conformance_test,fake_herdr_test,fake_prober_test}.rs` (and
`tests/herdr_mutants/mod.rs` only if the risk above applies), `CHANGELOG.md`, `docs/adr/ADR-0021.md` (the two lines of
AC 8 only), `docs/handoffs/683*` (pipeline artifacts).
Not changed: `lib.rs`, `conformance/mod.rs`, slice a's files, any `Cargo.toml`, `Cargo.lock`, any other crate (including
`holler-pane`: ASSUMPTION 9 hands its two doc lines to #640), any other ADR or ADR-0021 line, protocol doc or golden file. The repository is public: no personal names in code, comments, tests or the changelog; fixture names
are neutral (`scratch`, `w1:p1`), never a live session's.
