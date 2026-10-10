//! The ports (epic #633): the traits that are the seams between pane control and the
//! outside world, and the `Ports` bundle a verb holds.
//!
//! The adapters implement them (`holler-adapter-herdr`, `-host`, `-opencode`), the
//! hub implements the stores, and the test kit (`holler-pane-testkit`) fakes all of
//! them. [`ProfileStore`] and [`ProfileScope`] live in [`crate::profile`].
//!
//! **Every port is synchronous and blocking, and `Send + Sync`.** In async code call
//! one from `spawn_blocking` (or a thread). Every method returns within I5's bound
//! (default 10 s) or with [`PaneError::Timeout`].
//!
//! **Frozen when #637 merges**, after which a change goes through the epic's
//! amend-first rule. [`HerdrPort`] and [`HarnessPort`], and the minimal data types
//! they take and return, stay provisional until the spikes #636 (Herdr) and #635
//! (OpenCode) report.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::argv::Argv;
use crate::error::PaneError;
use crate::grid::GridPos;
use crate::pane::{HerdrPane, Pane, PaneEvent, PaneId, PaneName};
use crate::probe::{run_probe, ProbeResult};
use crate::profile::{ProfileScope, ProfileStore};

/// A position in a store's change sequence: a store-wide, strictly increasing
/// sequence number, one per change. `Cursor(0)` is "from the beginning".
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cursor(pub u64);

/// The stream `watch` returns: changes in order, each carrying its [`Cursor`].
///
/// - `watch(since)` yields every change after `since`. `Cursor(0)` starts from the
///   beginning: the store first yields a put for every record it holds now (the
///   current state), then every later change.
/// - Passing the cursor of the last event seen back as `since` resumes without a
///   gap or a repeat.
/// - `next()` blocks for at most I5's bound and yields one of three things:
///   - `Ok(Some(change))`: the next change;
///   - `Ok(None)` (the item, not the end of the iterator): **idle**, nothing happened
///     within the bound. This is an ordinary outcome, as in the hub's `control/wait`,
///     and the stream stays usable. A hub long-poll that sees it answers
///     `{events: [], cursor}`;
///   - `Err(..)`: a failure. `Err(PaneError::Timeout)` means the store did not
///     answer within the bound (a wedged store), never "idle". Any error ends the
///     stream (call `watch` again).
pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;

/// The registry of panes, kept by the hub (#639) and faked by the test kit (#638).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// Every write is a compare-and-swap on the pane's `generation` (see
/// [`crate::generation`]); a stale one is `generation-conflict`.
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;

    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;

    /// Remove a pane's record if it is still at `expected_generation`. `close`
    /// uses it, so a closed pane's record does not outlive the pane. A stale
    /// generation is `generation-conflict`; a record that does not exist is
    /// `pane-not-found`, whatever `expected_generation` is (a missing record is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError>;

    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
}

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

/// One key to press with `HerdrPort::send_keys`, by the name Herdr uses (for example
/// `enter` or `ctrl+c`). It goes to Herdr as written: nothing case-folds or translates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(String);

impl Key {
    /// A key by its name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The key's name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// loudly; never relocates a healthy pane. An occupied cell answers its pane and
    /// makes none. A cell outside the workspace's rows and columns (Herdr has none, so
    /// they are the implementation's configuration) is `grid-out-of-range`; a cell that
    /// no single right or down split reaches is the open code `grid-unreachable`.
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

/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
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

/// The harness server of a pane and its sessions (the adapter is
/// `holler-adapter-opencode`, #642). **Provisional** until spike #635 reports.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
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

/// Runs a health probe. [`SystemProber`] is the real one; a test swaps in a fake.
///
/// **Blocking.** The method is synchronous. Call from `spawn_blocking` (or a thread)
/// in async code. It returns within the `timeout` it is given (a timeout is
/// [`ProbeResult::Error`], not a [`PaneError`], because the method returns a
/// [`ProbeResult`]). An implementation is `Send + Sync`.
pub trait Prober: Send + Sync {
    /// Run `argv` (never through a shell) and look for every string of `expect` in
    /// its output, giving up after `timeout` (see [`crate::run_probe`]).
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult;
}

/// The [`Prober`] that runs the real probe: it calls the free [`run_probe`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProber;

impl Prober for SystemProber {
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
        run_probe(argv, expect, timeout)
    }
}

/// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
/// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
#[derive(Clone, Copy)]
pub struct Ports<'a> {
    pub pane_store: &'a dyn PaneStore,
    pub profile_store: &'a dyn ProfileStore,
    pub herdr: &'a dyn HerdrPort,
    pub host: &'a dyn HostPort,
    pub harness: &'a dyn HarnessPort,
    pub scope: &'a dyn ProfileScope,
    pub prober: &'a dyn Prober,
}
