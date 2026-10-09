# Brief: #684 the pane test kit, slice e: `FakeHost`, `FakeHarness` and their conformance suites

Repo: Performant-Labs/holler. Issue: #684 (slice e of #638, epic #633). Rigor: in-session. UI surface: no. Kind: feature
(test kit).

**Branch:** `issue-684-implementation`, based on `e410e9d` (`origin/main`: #637, #669, #670, ADR-0021, #639, #676, the
spikes #635 and #636, and slice a of #638 merged). **Design (D):** N/A. **Decision record:** ADR-0021 sections 5 and 7, the
epic's "Skeleton split" rulings, and the OpenCode spike (`docs/research/opencode-pane-spike.md`), which is the source of every
fact the harness fake models. The issue is the source of truth; where this brief adjusts its wording to the merged code or to
the spike, the adjustment is listed under "Decisions made in this brief".

## Size check

**Fits one run; no split.** F writes four source files (`src/host.rs`, `src/harness.rs`, `src/conformance/host.rs`,
`src/conformance/harness.rs`, all stubs today) and one CHANGELOG entry, which is under F's cap of about six files and one
component family (test-kit fakes). T writes four test files. No manifest, `lib.rs` or `conformance/mod.rs` changes.

| File | Lines (est.) |
|---|---|
| `src/host.rs` (`FakeHost`, `HostOp`) | ~220 |
| `src/harness.rs` (`FakeHarness`, `HarnessOp`, `Quirk`, `ServerState`, views) | ~520 |
| `src/conformance/host.rs` (9 cases) | ~240 |
| `src/conformance/harness.rs` (`HarnessRig`, 15 cases) | ~420 |
| `tests/` (4 files: two suites with their mutants, two fake-only files) | ~1,100 |
| **Total** | **~2,500** |

That is about twice the issue's estimate of ~1,200 lines. The issue's figure came from #638's table, which put slice a at
~1,550 lines, and slice a landed at ~2,160 (1,381 source, 41 `lib.rs`, 737 tests) in one passing run. This slice has fewer
source modules (4 against 6) and a similar total, so it is the same size as a run that worked. **Fallback if F's scope cap
fires anyway:** e1 = `FakeHost` with its suite (host files only), e2 = `FakeHarness` with its suite; they share nothing but
the merged `fault` and `conformance` modules, so either order works. If `src/harness.rs` nears 600 lines, move its private
state (`World` and its helpers) to `src/harness/world.rs`, declared by `mod world;` inside `harness.rs`, so `lib.rs` stays
untouched.

## Problem

`holler-pane-testkit` has fakes and a conformance suite for `PaneStore` only (slice a). The host adapter (#641), the OpenCode
adapter (#642) and every verb that starts, switches, aborts or observes a harness (#644, #645, #646, #647, #649) need an
in-memory `HostPort` and `HarnessPort` to test against, and the two adapters need a suite to prove they keep the port
contract. The harness fake must behave like OpenCode as the spike observed it (servers sharing one data directory, an
acknowledged `select-session` with no TUI, an acknowledged abort of an unknown id, a frozen server that accepts and never
answers, the shown session read from the TUI, a deleted session sending the TUI home) so verb tests meet the real failure
modes, while the suites pin the contract the adapters must provide on top of those quirks.

## Evidence (verbatim, as of `e410e9d`)

The stubs this slice fills (each is exactly these lines today):
```
crates/holler-pane-testkit/src/host.rs:1-3
//! `FakeHost`, the in-memory `HostPort`: tmux sessions, the commands run in them and
//! their processes, with fault injection. Empty stub declared by #638 so that no two
//! slices edit `lib.rs`; slice e (#684) fills it.
crates/holler-pane-testkit/src/harness.rs:1-4
//! `FakeHarness`, the in-memory `HarnessPort` modelled on the OpenCode spike: servers
//! by port sharing one data directory, sessions, the session a TUI shows, and switches
//! for raw OpenCode's quirks. Empty stub declared by #638 so that no two slices edit
//! `lib.rs`; slice e (#684) fills it.
crates/holler-pane-testkit/src/conformance/host.rs:1-2
//! The `HostPort` conformance suite. Empty stub declared by #638 so that no two slices
//! edit `conformance/mod.rs`; slice e (#684) fills it.
crates/holler-pane-testkit/src/conformance/harness.rs:1-2
//! The `HarnessPort` conformance suite. Empty stub declared by #638 so that no two
//! slices edit `conformance/mod.rs`; slice e (#684) fills it.
```
Already declared, so this slice edits neither file:
```
crates/holler-pane-testkit/src/lib.rs:35       pub mod harness;
crates/holler-pane-testkit/src/lib.rs:37       pub mod host;
crates/holler-pane-testkit/src/conformance/mod.rs:17   pub mod harness;
crates/holler-pane-testkit/src/conformance/mod.rs:19   pub mod host;
crates/holler-pane-testkit/Cargo.toml:14-17    [dependencies]
                                               # The ports the fakes implement and the suites drive (...)
                                               holler-pane = { path = "../holler-pane" }
```
The ports (merged; signatures the fakes implement exactly):
```
crates/holler-pane/src/ports.rs:156-168
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;
    /// Run `argv` (never through a shell) in the session `name`.
    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>;
    /// Stop the processes the session `name` owns.
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;
    /// The process ids running in the session `name`.
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
}
crates/holler-pane/src/ports.rs:176-200
pub trait HarnessPort: Send + Sync {
    /// Start the harness server for `name` on `port`; returns its process id.
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>;
    /// Whether the server on `port` answers.
    fn health(&self, port: u16) -> Result<bool, PaneError>;
    /// Create a session on the server at `port`; returns its id.
    fn create_session(&self, port: u16) -> Result<String, PaneError>;
    /// The session ids on the server at `port`.
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError>;
    /// Abort the running turn of `session`.
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError>;
    /// Attach the TUI of `pane` to `session` on the server at `port`.
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError>;
    /// Switch the TUI of `pane` to `session`.
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;
    /// The session the TUI of `pane` shows, if it can tell.
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
}
crates/holler-pane/src/ports.rs:150-155 and 170-175 (both traits)
/// **Blocking.** Every method is synchronous. ... Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
```
Note what the merged traits do **not** have, which shapes this brief: `select_session` takes no port (the TUI's server is
known from its attach); `health` is a `bool` (no reason); there is no `delete_session`; session ids are plain `String`s.

The types they use:
```
crates/holler-pane/src/argv.rs:25-43    pub struct Argv(Vec<String>);  derives Debug, Clone, PartialEq, Eq, Hash, Serialize
                                        pub fn new(parts: Vec<String>) -> Self;  pub fn as_slice(&self) -> &[String];
crates/holler-pane/src/pane.rs:31-47    pub struct PaneName(SessionName);  derives Clone, PartialEq, Eq, Hash, PartialOrd, Ord
                                        pub fn parse(text: &str) -> Result<Self, PaneError>;  Display = the name
crates/holler-pane/src/pane.rs:74-89    pub struct PaneId(String);  derives Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ...
                                        pub fn new(id: impl Into<String>) -> Self;  pub fn as_str(&self) -> &str;
```
The closed codes this slice returns (no new code; `error.rs` is #637's and frozen):
```
crates/holler-pane/src/error.rs:409    Usage { message: String },                 // "usage"
crates/holler-pane/src/error.rs:456    Timeout { op: String },                    // "timeout"
crates/holler-pane/src/error.rs:458    PaneNotFound { what: String },             // "pane-not-found"
crates/holler-pane/src/error.rs:459-461 /// `session-not-found`: no harness session of that id; `what` is the id. (#638-#642.)
                                       SessionNotFound { what: String },
crates/holler-pane/src/error.rs:465-467 /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
                                       /// socket, a harness. `what` names it. (#638-#642.)
                                       Unavailable { what: String },
crates/holler-pane/src/error.rs:291    | PaneCode::SessionNotFound => ErrorClass::Refusal,     (pane-not-found too, line 290)
crates/holler-pane/src/error.rs:297-298 | PaneCode::Timeout | PaneCode::Unavailable  ... => ErrorClass::Failure,
```
What slice a left to reuse (call, do not copy):
```
crates/holler-pane-testkit/src/fault.rs:17-23
/// A port method that a fault can target and the call log records. Each fake has its
/// own enum of them, e.g. [`crate::pane_store::PaneStoreOp`].
pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static {
    /// `"<port>.<method>"`, e.g. `"pane_store.cas_put"`. It is also the `op` of the
    /// `timeout` a wedged call answers.
    fn as_str(self) -> &'static str;
}
crates/holler-pane-testkit/src/fault.rs:25-35   pub enum Fault { Wedged, Fail(PaneError) }
crates/holler-pane-testkit/src/fault.rs:68,75,81,86   pub fn set / fail_next / set_delay / calls
crates/holler-pane-testkit/src/fault.rs:93      pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError>
crates/holler-pane-testkit/src/conformance/mod.rs:31-41
pub struct CaseFailure { pub case: &'static str, pub detail: String }
pub type Conformance = Result<(), Vec<CaseFailure>>;
crates/holler-pane-testkit/src/conformance/mod.rs:47-51
pub(crate) fn run_cases<S, K, C: Copy>(
    cases: &[(&'static str, C)],
    mut fresh: impl FnMut() -> (S, K),
    mut check: impl FnMut(C, &S) -> Result<(), String>,
) -> Conformance {
crates/holler-pane-testkit/src/conformance/mod.rs:71    pub(crate) fn succeeds<T>(call: &str, result: Result<T, PaneError>) -> Result<T, String>
crates/holler-pane-testkit/src/conformance/mod.rs:77-81 pub(crate) fn expect_code<T>(call: &str, result: Result<T, PaneError>, code: &str) -> Result<(), String>
crates/holler-pane-testkit/src/conformance/mod.rs:94    pub(crate) fn expect_eq<T: PartialEq + Debug>(what: &str, got: T, want: T) -> Result<(), String>
```
The pattern to mirror, in structure and naming:
```
crates/holler-pane-testkit/src/pane_store.rs:22-44     pub enum PaneStoreOp { Get, ... }  impl PortOp ("pane_store.get", ...)
crates/holler-pane-testkit/src/pane_store.rs:76-79     pub struct FakePaneStore { feed: .., faults: Arc<FaultSwitch<PaneStoreOp>> }
crates/holler-pane-testkit/src/pane_store.rs:128-132   pub fn faults(&self) -> &FaultSwitch<PaneStoreOp>
crates/holler-pane-testkit/src/pane_store.rs:134-146   concurrent_put / concurrent_delete: "another writer", bypass faults and the call log
crates/holler-pane-testkit/src/pane_store.rs:197-201   fn get(..) { self.faults.enter(PaneStoreOp::Get)?; ... }
crates/holler-pane-testkit/src/conformance/pane_store.rs:24     type Case = fn(&dyn PaneStore) -> Result<(), String>;
crates/holler-pane-testkit/src/conformance/pane_store.rs:40-78  const CASES: [(&str, Case); 19] = [ ("get-missing-is-none", get_missing_is_none), ... ];
crates/holler-pane-testkit/src/conformance/pane_store.rs:82-84  pub fn pane_store_cases() -> Vec<&'static str>
crates/holler-pane-testkit/src/conformance/pane_store.rs:110-116 pub fn run_pane_store_conformance<S, K, F>(fresh: F) -> Conformance
                                                                 where S: PaneStore, F: FnMut() -> (S, K)
crates/holler-pane-testkit/src/fixture.rs:24-31       sample_pane: pane_id: PaneId::new(format!("{SCRATCH}:{name}")), SAMPLE_PORT 48100
crates/holler-pane-testkit/tests/pane_store_conformance_test.rs:153-174  enum Break { Nothing, ... }  struct Mutant { inner, broken }
crates/holler-pane-testkit/tests/pane_store_conformance_test.rs:251-263  fn assert_suite_fails_on(broken, case)
```
The spike facts the harness fake models (`docs/research/opencode-pane-spike.md`, merged by #635):
```
:69-70    A fresh server has **no** session of its own: no "ping" session.
:71-72    After the server is killed and restarted on a new port with the same data directory, `GET /session/:id` still returns it.
:75-77    two servers that share a data directory share one session store. A session created on the second scratch server is
          readable through the first. The live fleet runs several `opencode serve` processes as one user with one data directory
:100-101  An **unknown** session id makes `attach` exit with status 1 and `Error: Session not found: ses_…`
:117      An unknown id answers `404 NotFoundError` and the TUI stays where it was.
:119-121  **Caveat 1: broadcast.** Two TUIs attached to one server *both* switched on one `select-session`.
:122-124  **Caveat 2: no acknowledgement.** On a server with **no** TUI attached, `select-session` still answers `200 true`.
:139-141  `OC | <session title>` while a session is shown. ... `OpenCode` on the home screen and right after the shown session is deleted.
:150      `ses_` plus 26 characters is 30 characters.
:151-152  Whether **Herdr** exposes a pane's terminal title the way tmux does is #636's question and is unverified here.
:171-172  **Unknown id:** `200 true`, the same as success. Holler must check `GET /session/:id` (404) first
:179      **Unverified:** aborting a model turn (streaming, or waiting on a provider) needs a model and was not run.
:189-191  **Frozen (SIGSTOP):** a TCP connect still succeeds (the kernel accepts it), and `GET /global/health` gets **no answer**
:192      **Killed (SIGKILL):** the connection is refused within about 6 ms.
:193-194  **The TUI reports neither.** While its server was frozen or dead, the TUI stayed alive with its title unchanged
:195-196  **Restart:** a new `serve` on the same port and data directory ... The session of record was still there, and the **old**
          TUI received a `show-toast` from the new server, so it re-subscribed by itself.
:222      TUI whose session is deleted | ... goes to the home screen (title `OpenCode`), **stays running**, and creates no
          replacement session. `select-session` to a live session brings it back.
:237      `select_session`: check `GET /session/:id` (404 means `session-not-found`), call `POST /tui/select-session`, then wait
:272-273  `select-session` to a session from **another project directory** than the TUI's `--dir`. ... (Not verified)
```
Consumers that set requirements on these fakes (issues, read 2026-10-09): #641 "Passes the conformance suite from #638 ...
a missing session is a typed error ... No call uses `pkill`/`killall`"; #642 "Passes the conformance suite from #638";
#644 "the fake host records argv, never a shell line"; #647 incidents "the TUI on a new empty session while the hub drives the
old one; a 3-day wedged server; a bare unregistered harness in the orchestrator's pane; a registered pane whose process died".

Lint and test conventions:
```
Cargo.toml:20-30          unwrap_used, expect_used, panic, unreachable, cognitive_complexity, too_many_lines,
                          struct_excessive_bools = "deny"; dead_code = "deny"
clippy.toml               cognitive-complexity-threshold = 15; too-many-lines-threshold = 100
scripts/lint.sh:43-52     warn at 600 lines per .rs file, fail at 900
tests/pane_store_conformance_test.rs:1   #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
CHANGELOG.md:42-50        the slice a entry ("Pane control, the test kit's first part ... ([#638](...))")
```

## Dependency direction

`holler-pane-testkit` -> `holler-pane` only, unchanged. This slice adds no dependency (the fakes need `std` only) and does not
edit `Cargo.toml`, so `cargo machete` is unaffected. The testkit never names `holler-cli`, `holler-hub` or an adapter crate.

## Public API of slice e (exact; T writes tests against these, F implements them)

No item is re-exported; everything is reached by its module path. Every type below that holds state uses one
`Mutex` taken with `unwrap_or_else(PoisonError::into_inner)` (slice a's pattern, `fault.rs:105-107`) and is `Send + Sync`.

```rust
// crates/holler-pane-testkit/src/host.rs
/// A method of the `HostPort` port, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostOp { EnsureSession, Run, StopOwned, Ps }
impl PortOp for HostOp { /* "host.ensure_session", "host.run", "host.stop_owned", "host.ps" */ }

/// An in-memory `HostPort`: tmux sessions by pane name, the processes in them, every argv run.
pub struct FakeHost { /* private: Mutex<HostState>, FaultSwitch<HostOp> */ }
impl FakeHost {
    pub fn new() -> Self;                                    // no session, no fault
    pub fn faults(&self) -> &FaultSwitch<HostOp>;
    /// Every session, sorted by name.                        (inspection: bypasses faults and the call log)
    pub fn sessions(&self) -> Vec<PaneName>;
    /// The cwd the session was created with, or `None` when there is no such session.
    pub fn cwd(&self, name: &PaneName) -> Option<String>;
    /// Every argv a successful `run` started, oldest first, with its session; never cleared (not by
    /// `stop_owned`, not by `end_session`).
    pub fn runs(&self) -> Vec<(PaneName, Argv)>;
    /// A process exited by itself (a crash): `pid` leaves the session's `ps`. `pane-not-found` when the
    /// session does not exist or does not hold `pid`.          (scenario: bypasses faults and the call log)
    pub fn exit_process(&self, name: &PaneName, pid: u32) -> Result<(), PaneError>;
    /// The tmux session ended outside Holler (it was killed): the session and its pids are gone.
    /// `pane-not-found` when there is no such session.
    pub fn end_session(&self, name: &PaneName) -> Result<(), PaneError>;
}
impl Default for FakeHost { /* = new() */ }
impl HostPort for FakeHost { /* see "Behaviour: FakeHost" */ }

// crates/holler-pane-testkit/src/harness.rs
/// A method of the `HarnessPort` port, as a fault targets it and the call log records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarnessOp { Serve, Health, CreateSession, ListSessions, Abort, AttachTui, SelectSession, ShownSession }
impl PortOp for HarnessOp { /* "harness.serve", "harness.health", "harness.create_session", "harness.list_sessions",
                               "harness.abort", "harness.attach_tui", "harness.select_session", "harness.shown_session" */ }

/// A behaviour of raw OpenCode that an adapter must hide (off by default; the suite fails a harness with one on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quirk {
    /// `select_session` on a pane with no TUI answers `Ok(())` and changes nothing (spike :122-124).
    SelectAckedWithoutTui,
    /// `abort` of an id the server does not know answers `Ok(())` (spike :171-172).
    AbortUnknownAcked,
}

/// The state of the harness server on a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerState {
    Running,
    /// SIGSTOPped: accepts a connection and never answers (spike :189-191).
    Frozen,
    /// Dead: refuses the connection (spike :192). Its sessions stay in the data directory.
    Killed,
}

/// What `server` reports about the server on a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerView { pub name: PaneName, pub pid: u32, pub state: ServerState }

/// What `tui` reports about the TUI in a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiView { pub port: u16, pub shown: Option<String> }

/// An in-memory `HarnessPort` modelled on the OpenCode spike.
pub struct FakeHarness { /* private: Mutex<World>, FaultSwitch<HarnessOp> */ }
impl FakeHarness {
    pub fn new() -> Self;                                     // no server, every port on data dir "default", no quirk
    pub fn faults(&self) -> &FaultSwitch<HarnessOp>;
    pub fn set_quirk(&self, quirk: Quirk, on: bool);
    /// Give the server on `port` its own data directory (default: "default", shared by every port). Sessions
    /// already created stay where they are.                  (configuration: bypasses faults and the call log)
    pub fn set_data_dir(&self, port: u16, dir: &str);
    /// SIGSTOP / SIGCONT / SIGKILL the server on `port`.     (scenario: bypasses faults and the call log)
    /// `unavailable` (the "unreachable" error below) when no server was ever served on `port`;
    /// `freeze` and `thaw` of a killed server are `unavailable` too; `kill` of a killed one is `Ok`.
    pub fn freeze(&self, port: u16) -> Result<(), PaneError>;
    pub fn thaw(&self, port: u16) -> Result<(), PaneError>;
    pub fn kill(&self, port: u16) -> Result<(), PaneError>;
    /// Another client created a session in `port`'s data directory (a stray or "ping" session). Needs no
    /// running server. Returns its id.
    pub fn seed_session(&self, port: u16) -> String;
    /// Another client deleted `session` (`DELETE /session/:id`): it leaves its data directory and every TUI
    /// showing it goes to the home screen (`shown` = `None`) and stays attached (spike :222).
    /// `session-not-found` when no data directory holds it.
    pub fn delete_session(&self, session: &str) -> Result<(), PaneError>;
    /// A person moved the TUI by hand: `Some(id)` shows that session, `None` goes to the home screen. No TUI
    /// in `pane` is `unavailable` (the no-TUI error below); an id not in the TUI's data dir is `session-not-found`.
    pub fn navigate(&self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError>;
    /// The TUI process in `pane` exited. `unavailable` (no-TUI error) when there is none.
    pub fn close_tui(&self, pane: &PaneId) -> Result<(), PaneError>;
    /// Inspection (bypasses faults and the call log):
    pub fn server(&self, port: u16) -> Option<ServerView>;
    pub fn tui(&self, pane: &PaneId) -> Option<TuiView>;
    /// Every abort the server acknowledged, oldest first, as (port, session).
    pub fn aborts(&self) -> Vec<(u16, String)>;
}
impl Default for FakeHarness { /* = new() */ }
impl HarnessPort for FakeHarness { /* see "Behaviour: FakeHarness" */ }

// crates/holler-pane-testkit/src/conformance/host.rs
pub fn host_cases() -> Vec<&'static str>;
pub fn run_host_conformance<S, K, F>(fresh: F) -> Conformance
where S: HostPort, F: FnMut() -> (S, K);

// crates/holler-pane-testkit/src/conformance/harness.rs
/// What a harness under test gives the suite for one case: two free ports whose servers share ONE data directory
/// (as the live fleet's do), and two panes in which a TUI can be attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRig { pub ports: [u16; 2], pub panes: [PaneId; 2] }
impl HarnessRig {
    /// Ports 48100 and 48101 (the spike's scratch range), panes "scratch:demo-c1r1" and "scratch:demo-c2r1"
    /// (the fixture's `pane_id` form, `fixture.rs:31`). Enough for the fake; a real adapter supplies its own.
    pub fn sample() -> Self;
}
pub fn harness_cases() -> Vec<&'static str>;
pub fn run_harness_conformance<S, K, F>(fresh: F) -> Conformance
where S: HarnessPort, F: FnMut() -> (S, HarnessRig, K);
```

`run_harness_conformance` reuses `run_cases` unchanged by folding the rig into the subject:
`run_cases(&CASES, || { let (h, rig, k) = fresh(); ((h, rig), k) }, |case, (h, rig)| case(h, rig))`, with
`type Case = fn(&dyn HarnessPort, &HarnessRig) -> Result<(), String>`. The host suite has
`type Case = fn(&dyn HostPort) -> Result<(), String>` and calls `run_cases` exactly as the pane-store suite does.

How each implementation runs the suites (doc comments, in `text` fences so they are not doctests):
```text
// the fakes:
assert_eq!(run_host_conformance(|| (FakeHost::new(), ())), Ok(()));
assert_eq!(run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ())), Ok(()));
// holler-adapter-host (#641): a private tmux server per case (`tmux -S <tempdir>/tmux.sock`), the adapter on it as S,
//   the tempdir and the tmux server's handle as K, so dropping K kills every process the case started.
// holler-adapter-opencode (#642): per case, a scratch dir with the spike's isolated env and dead-end provider
//   (opencode-pane-spike.md:16-35), two free ports from 48100-48199 whose servers share that one data directory,
//   two panes of a private tmux server; the scratch dir and the process groups as K.
```

### Exact error values (T may assert equality on these)

| Situation | Error |
|---|---|
| host: no session `name` (`run`, `ps`; `exit_process`/`end_session`) | `PaneError::PaneNotFound { what: name.to_string() }` |
| host: `run` with an empty argv | `PaneError::Usage { message: "an empty argv has no program to run".to_owned() }` |
| harness: server on `port` killed or never served ("unreachable") | `PaneError::Unavailable { what: format!("the harness server on port {port}") }` |
| harness: server on `port` frozen | `PaneError::Timeout { op: <the method's HarnessOp>.as_str().to_owned() }` (the same shape a wedged port answers) |
| harness: `serve` on a port running another pane's server | `PaneError::Unavailable { what: format!("port {port} is in use by the server of {owner}") }` |
| harness: no TUI in `pane` ("no-TUI") | `PaneError::Unavailable { what: format!("no TUI in pane {}", pane.as_str()) }` |
| harness: session id not in the data directory | `PaneError::SessionNotFound { what: session.to_owned() }` |

## Behaviour: `FakeHost`

Every port method calls `self.faults.enter(HostOp::..)?` first; an error from it is returned and nothing changes.

- `ensure_session(name, cwd)`: creates the session with `cwd` and no pids when it is missing; on an existing session it is
  `Ok` and changes nothing (not the cwd, not the pids). Idempotent.
- `run(name, argv)`: missing session is `pane-not-found`; an empty argv is `usage`; otherwise mints one pid, adds it to the
  session and appends `(name, argv.clone())` to the run log. The argv is stored as given: elements are never joined or
  re-split (a test can pass `"a b; rm -rf x"` as one element and read it back as one).
- `stop_owned(name)`: empties the session's pids and keeps the session; on a missing session it is `Ok` (nothing is owned,
  nothing to stop). It never touches another session.
- `ps(name)`: missing session is `pane-not-found`; otherwise the session's pids in the order they started.
- Pids are minted from one counter starting at 10_000, are never reused (not after `stop_owned`, `exit_process` or
  `end_session`) and are distinct across sessions. An `ensure_session` adds no pid (the fake has no shell process).

## Behaviour: `FakeHarness`

State: servers by port (`ServerView`), a data directory name per port (default `"default"`), the sessions of each data
directory in creation order, a TUI per pane (`TuiView`), the quirks that are on, the abort log, a session counter and a pid
counter. Every port method calls `self.faults.enter(HarnessOp::..)?` first. The fault switch models a wedged **adapter**
(every method, `health` and `shown_session` included, answers `timeout`); `freeze` models a wedged **server** behind a
working adapter. They are separate on purpose.

"Reach `port`" below means: `Running` -> continue; `Frozen` -> the frozen `timeout`; `Killed` or never served -> the
unreachable `unavailable`.

| Method | Behaviour, in check order |
|---|---|
| `serve(name, port)` | no server or `Killed`: start `Running` with a fresh pid (counter from 20_000, never reused) and return it; `Frozen`: frozen `timeout`; `Running` for the same `name`: return its pid (idempotent); `Running` for another name: the in-use `unavailable`. Creates no session. |
| `health(port)` | `Ok(true)` when `Running`; `Ok(false)` when `Frozen`, `Killed` or never served (the spike: a timed GET that times out or is refused, :189-192). Never an error except through the fault switch. |
| `create_session(port)` | reach `port`; mint `format!("ses_{n:026x}")` (n from 1; 30 characters, under the spike's 40-character title limit, :150) in `port`'s data dir; return it. Ids are opaque: tests must not parse the suffix. |
| `list_sessions(port)` | reach `port`; every session of `port`'s data directory, in creation order. By default that is every session made through any port (spike :75-77). |
| `abort(port, session)` | reach `port`; known in `port`'s data dir: log `(port, session)`, `Ok`; unknown: `Quirk::AbortUnknownAcked` on -> log it, `Ok`; off -> `session-not-found`. ASSUMPTION comment (below). |
| `attach_tui(pane, port, session)` | reach `port`; `session` not in `port`'s data dir: `session-not-found`, and the pane's TUI (if any) is unchanged; else the pane's TUI is `{ port, shown: Some(session) }` (replacing any earlier one). |
| `select_session(pane, session)` | no TUI in `pane`: `Quirk::SelectAckedWithoutTui` on -> `Ok(())`, nothing changes; off -> the no-TUI `unavailable`. Then reach the TUI's port; `session` not in that port's data dir: `session-not-found`, screen unchanged; else that TUI's `shown` = `Some(session)`. Only `pane`'s TUI changes. ASSUMPTION comment (below). |
| `shown_session(pane)` | the TUI's `shown`, or `Ok(None)` when there is no TUI. It does not reach the server: the shown session is read from the TUI, which keeps its screen while its server is frozen or dead (spike :193-194). ASSUMPTION comment (below). |

Scenario controls, as listed in the API: `freeze`/`thaw`/`kill` change only the server's state; a TUI on a frozen or killed
server keeps its `shown`. After `kill`, `serve` on the port starts a new server with a new pid and every session of the data
directory is still listed and attachable, and the old TUI still shows its session (spike :195-196). `delete_session` sends
every TUI showing that id home; a later `select_session` to a live session brings it back (spike :222). `seed_session` is
the stray "ping" session and the session a person starts by hand on the home screen (#647's incidents).

**ASSUMPTION comments**, one line each, verbatim prefix `// ASSUMPTION (#642 to confirm):`, at the method named:
- `abort`: "abort stops a model turn as it stops a shell command; the spike verified only a shell command
  (opencode-pane-spike.md:179, 267)."
- `shown_session`: "the shown session is read from the pane's terminal title; the spike read it through tmux, and whether
  Herdr exposes a pane's terminal title is unverified (opencode-pane-spike.md:151-152, 269)."
- `select_session`: "select-session switches a TUI whose --dir is another project directory than the session's; the spike
  did not try it (opencode-pane-spike.md:272-273). The fake has no project directories and always switches."

## Conformance cases: `run_host_conformance`

Each case gets a fresh host. Names are neutral (`demo-c1r1`, `demo-c2r1`); `cwd` is `std::env::temp_dir()` as a string (it
exists on any machine, so a real tmux adapter can use it); the argv is `["sleep", "30"]` (harmless if a real adapter runs it,
and the case's guard kills it). The cases assert only what a real tmux adapter can also satisfy: a real session may hold a
shell pid, so no case asserts that `ps` is empty after `ensure_session`.

| # | Case id | What it asserts | Mutant it catches (`tests/host_conformance_test.rs`) |
|---|---|---|---|
| 1 | `ps-of-missing-session-is-pane-not-found` | `ps` of a name never ensured is `pane-not-found`. | `ps` of a missing session answers `Ok(vec![])` (`PsOfMissingIsEmpty`) |
| 2 | `run-in-missing-session-is-pane-not-found` | `run` in a name never ensured is `pane-not-found`, and `ps` of it is still `pane-not-found` afterwards (the run created nothing). | `run` creates the missing session first (`RunCreatesMissingSession`) |
| 3 | `run-adds-a-process` | ensure; `before = ps`; `run(sleep 30)`; `ps` holds a pid not in `before`. | `run` starts nothing (`RunIsNoop`) |
| 4 | `ensure-session-is-idempotent` | ensure; run; `pids = ps`; ensure again (same cwd) is `Ok`; `ps` still holds every pid of `pids`. | a second ensure recreates the session (`EnsureRecreates`) |
| 5 | `run-empty-argv-is-usage` | ensure; `before = ps`; `run(Argv::new(vec![]))` is `usage`; `ps` equals `before`. | an empty argv runs something (`RunsEmptyArgv`) |
| 6 | `stop-owned-stops-every-owned-process` | ensure; run twice; `stop_owned` is `Ok`; ensure again; `ps` holds neither pid the runs added. | `stop_owned` stops nothing (`StopIsNoop`) |
| 7 | `stop-owned-of-missing-session-is-ok` | `stop_owned` of a name never ensured is `Ok`. | `stop_owned` of a missing session fails (`StopMissingFails`) |
| 8 | `stop-owned-leaves-other-sessions` | ensure a and b; run in each; `stop_owned(a)`; `ps(b)` still holds b's run pid. | `stop_owned` stops every session, the broad kill #641 forbids (`StopsEverySession`) |
| 9 | `ps-lists-only-its-session` | ensure a and b; run in each; `ps(a)` lacks b's run pid. | `ps` lists every session's pids (`PsListsEverySession`) |

## Conformance cases: `run_harness_conformance`

Each case gets a fresh harness and its rig: `p0`, `p1` = `rig.ports`, `pane0`, `pane1` = `rig.panes`; `serve` uses the names
`demo-c1r1` (p0) and `demo-c2r1` (p1). `UNKNOWN` = `"ses_zzzzzzzzzzzzzzzzzzzzzzzzzz"` (`ses_` plus 26 `z`: well formed, and
never minted by the fake, whose suffix is hex). Lists are compared as sorted sets (the port fixes no order; OpenCode lists
most recently updated first, spike :62). The quirks are off: the suite checks the contract an adapter provides on top of raw
OpenCode, so a fake with a quirk on is a mutant.

| # | Case id | What it asserts | Mutant it catches (`tests/harness_conformance_test.rs`) |
|---|---|---|---|
| 1 | `health-of-unserved-port-is-false` | `health(p0)` on a fresh harness is `Ok(false)`. | `health` always `Ok(true)` (`HealthAlwaysTrue`) |
| 2 | `serve-then-healthy` | `serve(demo-c1r1, p0)` returns a pid > 0; `health(p0)` is `Ok(true)`. | (sanity: every mutant must still pass it) |
| 3 | `fresh-server-has-no-sessions` | after `serve`, `list_sessions(p0)` is empty (no "ping" session, spike :69-70). | `serve` creates a session (`PingSession`) |
| 4 | `create-session-is-listed` | serve; two `create_session(p0)` return non-empty, distinct ids; `list_sessions(p0)` holds both. | `create_session` returns the first id again (`ReusesSessionId`) |
| 5 | `sessions-shared-across-servers` | serve p0 and p1; create on p1; `list_sessions(p0)` holds it (one data directory, spike :75-77). | per-port stores: the fake with `set_data_dir(p1, "other")` |
| 6 | `calls-to-unserved-port-are-unavailable` | on a fresh harness, `create_session(p0)`, `list_sessions(p0)`, `abort(p0, UNKNOWN)` and `attach_tui(pane0, p0, UNKNOWN)` are each `unavailable` (reachability is checked before existence). | (pins the order; `HealthAlwaysTrue` does not affect it) |
| 7 | `abort-known-session` | serve; create a; `abort(p0, a)` is `Ok`. | (sanity) |
| 8 | `abort-unknown-is-session-not-found` | serve; `abort(p0, UNKNOWN)` is `session-not-found` (spike :171-172). | the fake with `Quirk::AbortUnknownAcked` on |
| 9 | `shown-without-tui-is-none` | `shown_session(pane0)` on a fresh harness is `Ok(None)`. | (sanity) |
| 10 | `attach-shows-the-session` | serve; create a; `attach_tui(pane0, p0, a)` is `Ok`; `shown_session(pane0)` is `Some(a)`. | (sanity) |
| 11 | `attach-unknown-is-session-not-found` | serve; create a; `attach_tui(pane0, p0, UNKNOWN)` is `session-not-found`; `shown_session(pane0)` is `None`: it did not fall back to a (the "guess" of the 2026-10-07 incidents; spike :100-101). | attach of an unknown id shows the latest session (`AttachFallsBackToLatest`) |
| 12 | `select-switches-the-shown-session` | serve; create a, b; attach pane0 to a; `select_session(pane0, b)` is `Ok`; shown is `Some(b)`. | (sanity) |
| 13 | `select-unknown-is-session-not-found` | as 12, then `select_session(pane0, UNKNOWN)` is `session-not-found`; shown is still `Some(b)` (spike :117). | select of an unknown id goes home (`SelectUnknownGoesHome`) |
| 14 | `select-without-tui-fails` | serve; create a; `select_session(pane0, a)` with no TUI is `unavailable`; `shown_session(pane0)` is still `None` (spike :122-124). | the fake with `Quirk::SelectAckedWithoutTui` on |
| 15 | `select-reaches-only-its-pane` | serve p0 and p1; create a, b; attach pane0 to a on p0 and pane1 to a on p1; `select_session(pane0, b)`; pane0 shows b, pane1 still shows a. | select switches every TUI (`SelectBroadcasts`, spike :119-121) |

Frozen and killed servers, a deleted session, hand navigation and a closed TUI cannot be caused through the port, so the
suite cannot drive them; `tests/fake_harness_test.rs` pins them on the fake (AC 6).

## Acceptance criteria

Test names are what T authors (RED first). Every test file starts with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684`. Integration-test helpers that go unused fail
`dead_code`, so T keeps only helpers a test calls.

1. **The host fake passes its suite.** `tests/host_conformance_test.rs`:
   - [ ] `the_fake_passes_the_host_conformance_suite`: `run_host_conformance(|| (FakeHost::new(), ()))` is `Ok(())`.
   - [ ] `the_host_suite_runs_the_documented_cases`: `host_cases()` equals the 9 ids of the host table, in order.
2. **Host mutation check.** Same file; `enum Break { Nothing, PsOfMissingIsEmpty, RunCreatesMissingSession, RunIsNoop,
   EnsureRecreates, RunsEmptyArgv, StopIsNoop, StopMissingFails, StopsEverySession, PsListsEverySession }` and a
   `Mutant { inner: FakeHost, broken: Break }` implementing `HostPort` (breaks built from the fake's own port methods plus
   `end_session` and `sessions`); `assert_suite_fails_on(broken, case)` as in slice a (`Err`, the named case among the
   failures, every `detail` non-empty).
   - [ ] `the_unbroken_host_wrapper_passes` (`Break::Nothing` is `Ok(())`).
   - [ ] one test per break, named `a_host_whose_<break in words>_fails` (e.g. `a_host_whose_stop_owned_stops_every_session_fails`),
     each naming the case of the host table's last column.
3. **The harness fake passes its suite.** `tests/harness_conformance_test.rs`:
   - [ ] `the_fake_passes_the_harness_conformance_suite`: `run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ()))`
     is `Ok(())`.
   - [ ] `the_harness_suite_runs_the_documented_cases`: `harness_cases()` equals the 15 ids of the harness table, in order.
   - [ ] `the_sample_rig_is_two_ports_and_two_panes`: `HarnessRig::sample()` is ports `[48100, 48101]` and panes
     `scratch:demo-c1r1`, `scratch:demo-c2r1`.
4. **Harness mutation check.** Same file.
   - [ ] `a_harness_that_acks_select_without_a_tui_fails` (the fake with `Quirk::SelectAckedWithoutTui` on) ->
     `select-without-tui-fails`.
   - [ ] `a_harness_that_acks_abort_of_an_unknown_id_fails` (`Quirk::AbortUnknownAcked` on) -> `abort-unknown-is-session-not-found`.
   - [ ] `a_harness_whose_servers_do_not_share_a_data_dir_fails` (`set_data_dir(48101, "other")` before the case) ->
     `sessions-shared-across-servers`.
   - [ ] With `enum Break { Nothing, HealthAlwaysTrue, PingSession, ReusesSessionId, AttachFallsBackToLatest,
     SelectUnknownGoesHome, SelectBroadcasts }` and a `Mutant { inner: FakeHarness, broken }` implementing `HarnessPort`:
     `the_unbroken_harness_wrapper_passes`, and one `a_harness_whose_<break in words>_fails` per break, each naming the case
     of the harness table's last column. (`PingSession` = `serve` then `inner.seed_session(port)`; `SelectBroadcasts` =
     select on every rig pane that `inner.tui(..)` shows attached; `AttachFallsBackToLatest` = on `session-not-found`,
     attach to the last of `inner.list_sessions(port)`; `SelectUnknownGoesHome` = on `session-not-found`,
     `inner.navigate(pane, None)` before returning the error.)
5. **`FakeHost` mechanisms.** `tests/fake_host_test.rs`:
   - [ ] `a_wedged_host_times_out_every_method`: `Fault::Wedged`; the four methods answer `Timeout { op: "host.<method>" }`;
     after `set(None)` nothing was created and the methods work.
   - [ ] `host_op_names_are_port_dot_method` (the four strings).
   - [ ] `calls_are_recorded_in_order`: `ensure_session`, a failed `run`, `ps` -> `[EnsureSession, Run, Ps]`; `exit_process`,
     `end_session` and the inspection methods add nothing.
   - [ ] `run_records_each_argv_verbatim`: two runs (`["opencode","serve","--port","48100"]` and `["echo","a b; rm -rf x"]`)
     are in `runs()` in order, the second with 2 elements; still there after `stop_owned` and after `end_session`.
   - [ ] `a_refused_run_records_nothing`: a run in a missing session, an empty argv and a run failed by `fail_next(Run, ..)`
     leave `runs()` empty and `ps` unchanged.
   - [ ] `ensure_session_keeps_the_first_cwd`: `cwd` is the first one after a second ensure with another cwd;
     `sessions()` is sorted by name.
   - [ ] `pids_are_distinct_and_never_reused`: across two sessions and after `stop_owned`, no pid repeats; the first is 10_000.
   - [ ] `a_process_that_exits_leaves_ps`: `exit_process` removes just that pid; an unknown pid or session is `pane-not-found`.
   - [ ] `an_ended_session_is_missing_until_ensured_again`: after `end_session`, `ps` and `run` are `pane-not-found`,
     `stop_owned` is `Ok`; `ensure_session` recreates it with no pids; `end_session` of a missing name is `pane-not-found`.
   - [ ] `the_fake_host_is_send_and_sync` (a compile-time bound check).
6. **`FakeHarness` mechanisms.** `tests/fake_harness_test.rs`:
   - [ ] `a_wedged_harness_times_out_every_method`: `Fault::Wedged`; all eight methods, `health` and `shown_session`
     included, answer `Timeout { op: "harness.<method>" }`.
   - [ ] `harness_op_names_are_port_dot_method` (the eight strings) and `calls_are_recorded_in_order` (the scenario and
     inspection methods add nothing).
   - [ ] `session_ids_are_ses_prefixed_and_distinct`: each id starts with `ses_`, is 30 characters, and no two are equal.
   - [ ] `a_frozen_server_answers_health_false_and_times_out_its_calls`: with pane0 attached to a on p0, `freeze(p0)`;
     `health(p0)` is `Ok(false)`; `serve`, `create_session`, `list_sessions`, `abort`, `attach_tui` on p0 and
     `select_session(pane0, ..)` are `Timeout { op: "harness.<method>" }`; `shown_session(pane0)` is still `Some(a)`;
     a server on p1 is unaffected.
   - [ ] `thaw_brings_a_frozen_server_back`: after `thaw(p0)` every call works and the TUI still shows a.
   - [ ] `a_killed_server_answers_health_false_and_is_unavailable`: `kill(p0)`; `health` `Ok(false)`; the calls are the
     unreachable `unavailable`; `shown_session(pane0)` is still `Some(a)`; `freeze` and `thaw` of it are `unavailable`.
   - [ ] `serving_a_killed_port_again_keeps_the_sessions`: `serve` after `kill` returns a different pid, `health` is
     `Ok(true)`, `list_sessions` still holds a, pane0 still shows a, and `select_session` to b works.
   - [ ] `serve_on_a_running_port`: the same name returns the same pid; another name is the in-use `unavailable`.
   - [ ] `controls_of_an_unserved_port_are_unavailable`: `freeze`, `thaw`, `kill` of a port never served.
   - [ ] `a_session_deleted_under_a_tui_sends_it_home`: `delete_session(a)`; `shown_session(pane0)` is `None`; `tui(pane0)`
     is still `Some` on p0; `list_sessions` lacks a; `select_session(pane0, b)` shows b again; `delete_session(UNKNOWN)` is
     `session-not-found`.
   - [ ] `navigating_by_hand_changes_the_shown_session`: `navigate(pane0, Some(b))` shows b, `None` goes home; an unknown
     id is `session-not-found`; a pane with no TUI is the no-TUI `unavailable`.
   - [ ] `closing_the_tui_leaves_no_shown_session`: after `close_tui(pane0)`, `tui` is `None`, `shown_session` is `Ok(None)`,
     `select_session` is the no-TUI `unavailable`.
   - [ ] `select_without_a_tui_is_acked_with_the_quirk`: quirk off by default (`unavailable`); on -> `Ok(())`, `tui(pane0)`
     still `None`.
   - [ ] `abort_of_an_unknown_id_is_acked_with_the_quirk`: off by default (`session-not-found`, nothing logged); on ->
     `Ok(())` and `aborts()` holds it; a known abort is logged as `(port, id)`.
   - [ ] `separate_data_dirs_do_not_share_sessions`: with `set_data_dir(p1, "other")`, a session created on p1 is not in
     `list_sessions(p0)`, and `abort`/`attach_tui` on p0 for it are `session-not-found`.
   - [ ] `a_seeded_session_is_listed_and_bypasses_the_log`: `seed_session(p0)` (no server needed) is listed once p0 is
     served; `calls()` holds only the calls made through the port.
   - [ ] `the_fake_harness_is_send_and_sync`.
7. **ASSUMPTION comments.** `grep -c 'ASSUMPTION (#642 to confirm)' crates/holler-pane-testkit/src/harness.rs` prints `3`
   (abort, shown_session, select_session).
8. **Dependency rule.** `crates/holler-pane-testkit/Cargo.toml` is unchanged; `cargo tree -p holler-pane-testkit -e normal
   --prefix none | grep -E '^holler-(cli|hub|adapter)'` prints nothing.
9. **No other crate changes.** `git diff --name-only origin/main...HEAD` lists only Blast-radius paths; `lib.rs` and
   `conformance/mod.rs` are untouched.
10. **CHANGELOG.** One entry under `## [Unreleased]` / `### Enhancements`, placed after the slice a entry
    (`CHANGELOG.md:42-50`) or after the last test-kit entry if a sibling slice (#681, #682, #683) merged first: the test kit's
    host and harness fakes. A fake host (tmux sessions, the argv of every command run, the processes a session owns) and a
    fake OpenCode harness modelled on the spike (servers sharing one data directory, a frozen or killed server, a session
    deleted under a TUI, and switches for raw OpenCode's acknowledged select with no TUI and acknowledged abort of an unknown
    session), each with a conformance suite an adapter runs against itself (9 host cases, 15 harness cases); a fake with a
    quirk on fails the suite. Test code only. Link [#684](https://github.com/Performant-Labs/holler/issues/684).
11. **Guards.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
    `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` and `bash scripts/test-hooks.sh` pass. Every
    new or filled `.rs` file passes `rustfmt --check --edition 2021`. No `.rs` file reaches 600 lines; no function exceeds
    100 lines or cognitive complexity 15 (one function per case); no `unwrap`, `expect`, `panic!`, `unreachable!` or
    `assert!` in `src/`.

## Files

Filled (stubs today), all under `crates/holler-pane-testkit/`: `src/host.rs`, `src/harness.rs` (optionally plus
`src/harness/world.rs`, see Size check), `src/conformance/host.rs`, `src/conformance/harness.rs`.
New tests: `tests/host_conformance_test.rs`, `tests/fake_host_test.rs`, `tests/harness_conformance_test.rs`,
`tests/fake_harness_test.rs`. Changed: `CHANGELOG.md`.

## Extend vs new

- **Extend:** fill the four stubs slice a created; implement the frozen `HostPort` and `HarnessPort` traits as merged; give
  each fake a `FaultSwitch<Op>` and call `enter` first in every port method (`fault.rs:93`), with `HostOp`/`HarnessOp`
  implementing the merged `PortOp::as_str` (`fault.rs:17-23`); build both suites on `run_cases`, `succeeds`, `expect_code`
  and `expect_eq` (`conformance/mod.rs:47-100`) and return `Conformance`/`CaseFailure`; mirror `FakePaneStore` and its suite
  in naming (`<port>Op`, `Fake<Port>`, `faults()`, `<port>_cases()`, `run_<port>_conformance`, the `CASES` table, `Break` and
  `Mutant` in the tests); return closed `PaneError` variants only.
- **New, no parallel path:** none of the above is written again. The harness's frozen `timeout` uses the same
  `Timeout { op: op.as_str() }` shape the fault switch answers. The `Mutex` + `PoisonError::into_inner` lock line appears once
  per fake, as in `fault.rs:105-107` and `feed.rs:218-219` (a one-line idiom; no shared helper is added, because one would
  need a `conformance/mod.rs` or `lib.rs` edit).
- **Not reused, on purpose:** the change feed (`feed.rs`): neither port has a `watch`. `sample_pane`: the host and harness
  ports take a `PaneName`, a `PaneId` and a port, not a `Pane`; `HarnessRig::sample` uses the fixture's `pane_id` form and
  port range instead.

## Decisions already made (operator, epic, issue)

- The testkit depends on `holler-pane` only (ADR-0021:184-186; issue acceptance item 2).
- Module and file names are fixed by slice a; this slice edits neither `lib.rs` nor `conformance/mod.rs` (issue scope).
- The suites check the port contract with the quirks off, so a fake with a quirk on is a mutant (issue scope).
- Selecting or aborting an unknown id is `session-not-found`, because the adapter checks `GET /session/:id` first (issue,
  spike :171-172, :237); `shown_session` is `None` with no TUI, on the home screen and after the shown session is deleted.
- No story touches a live fleet, a running pane or a real OpenCode or tmux session (epic rules).

## Decisions made in this brief

1. **Frozen means the calls addressed to that server, not `shown_session`.** The issue says a frozen server makes "every
   other call `timeout`". `shown_session` takes a pane, not a port, and the spike observed that the TUI keeps its title while
   its server is frozen or dead (:193-194), so it keeps answering. This matters to #647: a wedged server whose TUI still shows
   the right session must be found by `health`, not by SHOWN. The same holds for a killed server.
2. **A wedged adapter and a frozen server are different switches.** `faults().set(Some(Fault::Wedged))` makes every method
   (health included) `timeout`; `freeze(port)` makes `health` `Ok(false)`. Both are needed: #642's own wedge versus
   OpenCode's.
3. **`health` carries no reason** (the merged port is `Result<bool, _>`, not the spike's `Healthy | Unhealthy(reason)`). A
   caller that must tell frozen from killed reads the code of its next call to that server: `timeout` or `unavailable`. The
   fake models both. Changing the port is out of scope (amend-first rule).
4. **A deleted session is a scenario control (`delete_session`), not a quirk switch**: it is the contract (`shown_session`
   goes to `None`), and the port has no delete method. The two quirk switches are exactly the issue's two.
5. **`select_session` on a pane with no TUI is `unavailable`** (the no-TUI error), checked before the session id because
   the merged method has no port, so the TUI is what names the server. Its class is a failure (exit 1), the same as a timeout.
6. **Host: a missing session is `pane-not-found`** for `run` and `ps` (#641's "a missing session is a typed error"; the
   tmux session is named by the `PaneName`, and `session-not-found` stays the harness session's code), while `stop_owned` of
   a missing session is `Ok` (nothing is owned, so `relaunch` and `close` after a crash do not fail). An empty argv is `usage`.
7. **The host suite asserts only what real tmux can satisfy**: a pid appears after `run` and is gone after `stop_owned`; it
   never asserts that a fresh session's `ps` is empty or that the session survives `stop_owned`. The fake's own empty `ps`
   and surviving session are pinned in the fake-only tests.
8. **`HarnessRig`**: the harness suite takes its ports and panes from `fresh`, so #642 can run it on free ports and real
   scratch panes. `fresh` must give two servers that share one data directory, as the live fleet does.
9. **Ids**: `ses_` plus 26 lowercase hex digits from a counter (30 characters, deterministic, under the 40-character title
   limit). Pids are deterministic counters (host from 10_000, harness from 20_000).
10. **Not modelled**: select broadcast to every TUI on a server (the adapter keeps one TUI per server, spike :126; the suite
    still requires a select to reach only its pane), title truncation and the default-title ambiguity (the port returns an
    id, not a title), basic auth, the boot race, busy/idle status, and memory growth.
11. **`PortOp`'s method is `as_str`** (merged), not `name` as #638's brief appendix wrote it.

## Out of scope

Slices b to d (#681, #682, #683); any change to `holler-pane` (the traits, `PaneError`, the fixture's crate), the hub, the
CLI, an adapter crate or any manifest; running the suites against the real adapters (#641, #642 do that); amending
`HarnessPort` (a reason on `health`, a port on `select_session`, a typed session id); the items of decision 10.

## Test plan

RED (T): write the four test files against the API above; they fail to build (`FakeHost`, `FakeHarness`,
`run_host_conformance`, `run_harness_conformance`, `HarnessRig` do not exist). Confirm with
`cargo test -p holler-pane-testkit` that the errors are the missing items, not typos. GREEN (F): `host.rs`, then
`conformance/host.rs`, then `harness.rs`, then `conformance/harness.rs`, then the CHANGELOG entry, then the guards of AC 11.
A (anti-duplication): no second fault switch, case runner or assertion helper; no new error code; the frozen `timeout` has
the fault switch's shape.

## Risks

- **CHANGELOG conflicts with the sibling slices** (#681 to #683 each add an entry in the same place). Resolve by rebasing and
  keeping every entry; the placement rule in AC 10 allows either order.
- **`harness.rs` near the 600-line warning.** The fallback is `src/harness/world.rs` (Size check). The table-driven method
  docs above are long; keep the per-method code short by sharing one private `reach(port, op)` and one
  `session_exists(dir, id)`.
- **A mutant that also breaks an earlier case** is fine (the assertion is "the named case is among the failures"), but a
  mutant must not break `serve-then-healthy`'s setup in a way that fails every case before its own: the wrappers break only
  the method named.
- **The decisions about codes (5, 6) bind #641 and #642.** They are recorded here and in the suites' doc comments; an adapter
  that needs a different code amends the suite in its own story.

## Blast radius

`crates/holler-pane-testkit/src/{host,harness}.rs` (and `src/harness/world.rs` only if the fallback is used),
`crates/holler-pane-testkit/src/conformance/{host,harness}.rs`, the four new files under `crates/holler-pane-testkit/tests/`,
`CHANGELOG.md`, and `docs/handoffs/684*` (pipeline artifacts). Not changed: `lib.rs`, `conformance/mod.rs`, the testkit's
`Cargo.toml`, `Cargo.lock`, any other crate, ADR, protocol doc, golden file or CLI fixture. The repository is public: no
personal names in code, comments, tests or the changelog.
