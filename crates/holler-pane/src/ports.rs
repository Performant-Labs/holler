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
//! amend-first rule. [`HerdrPort`], and the minimal data types it takes and returns,
//! stay provisional until the spike #636 (Herdr) reports. [`HarnessPort`] is final:
//! the spike #635 (OpenCode) confirmed it, and #642 built it. Four amendments since:
//!
//! - #645's two default-armed seam methods, [`HarnessPort::send_prompt`] and
//!   [`HarnessPort::session_activity`], whose defaults keep every impl that does not
//!   override them as it was;
//! - #802's: `serve` became the pure [`HarnessPort::serve_command`], which takes a
//!   [`ServeSpec`] and returns the server's argv, beside the required
//!   [`HarnessPort::verify_serving`]; and [`HostPort::run`] takes the directory it starts
//!   its program in. The host starts and stops a pane's processes, its harness server
//!   among them;
//! - #832's: [`HerdrPort::observe_pane`] and its batch form [`HerdrPort::observe_panes`],
//!   what Herdr says about a pane that no record names;
//! - #835's: the required [`HostPort::foreground_commands`], the command name of each pane
//!   process of a tmux session (how the import tells a pane that runs Claude Code), and
//!   [`HerdrPort::observe_pane`] lost its default. Both are required so that the compiler
//!   finds every implementation, a wrapper port included: a default answers silently in place
//!   of a port a wrapper forgot to forward.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::argv::{AgentKey, Argv};
use crate::error::{PaneError, RefusalCode};
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

/// What a pane's harness server must run with: the part of the pane's spec that
/// [`HarnessPort::serve_command`] applies to the server's argv and
/// [`HarnessPort::verify_serving`] checks on the running server (#802).
///
/// `#[non_exhaustive]`: the model, the effort, the environment names and the context
/// ceilings join it later with no change to a signature. A new field of the pane's spec
/// for its server goes here, never into a new parameter. Outside this crate a spec is
/// built only from [`ServeSpec::default`] and its builders, so a struct literal does not
/// compile there, with or without `..Default::default()`; the launch engine, inside this
/// crate, builds it with every field named.
///
/// ```compile_fail,E0639
/// let spec = holler_pane::ServeSpec { agent: None };
/// ```
/// ```compile_fail,E0639
/// let spec = holler_pane::ServeSpec { agent: None, ..Default::default() };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ServeSpec {
    /// The OpenCode agent the server runs as its default (`Pane.opencode_agent`); `None`
    /// leaves the server's own default.
    pub agent: Option<AgentKey>,
}

impl ServeSpec {
    /// This spec, with `agent` as the server's agent.
    #[must_use]
    pub fn with_agent(mut self, agent: AgentKey) -> Self {
        self.agent = Some(agent);
        self
    }
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
///
/// [`HerdrPort::observe_pane`] has no default (#835), so an implementation that leaves it
/// out, a wrapper port included, does not compile:
///
/// ```compile_fail,E0046
/// use holler_pane::{HerdrPane, HerdrPort, HerdrSnapshot, HerdrSpec, Key, PaneError, PaneId};
///
/// struct NoObservation;
///
/// impl HerdrPort for NoObservation {
///     fn ensure_pane(&self, _: &HerdrSpec) -> Result<HerdrPane, PaneError> {
///         Err(PaneError::NotImplemented)
///     }
///     fn send_text(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn send_keys(&self, _: &PaneId, _: &[Key]) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn read(&self, _: &PaneId, _: usize) -> Result<String, PaneError> {
///         Ok(String::new())
///     }
///     fn close(&self, _: &PaneId) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
///         Ok(HerdrSnapshot { panes: Vec::new() })
///     }
///     fn version(&self) -> Result<String, PaneError> {
///         Ok(String::new())
///     }
/// }
/// ```
pub trait HerdrPort: Send + Sync {
    /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail
    /// loudly; never relocates a healthy pane. An occupied cell answers its pane and
    /// makes none. A cell outside the workspace's rows and columns (Herdr has none, so
    /// they are the implementation's configuration) is `grid-out-of-range`; a cell that
    /// no single right or down split reaches is the open code `grid-unreachable`. The
    /// answer carries the pane's label as `snapshot` reports it (#808).
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;

    /// Type `text` into the pane.
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;

    /// Press `keys` in the pane.
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError>;

    /// The last `max_lines` lines of the pane's screen.
    fn read(&self, pane: &PaneId, max_lines: usize) -> Result<String, PaneError>;

    /// Close the pane.
    fn close(&self, pane: &PaneId) -> Result<(), PaneError>;

    /// Every pane Herdr has, with its position and its label (`HerdrPane.label`, #808).
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError>;

    /// Herdr's API version. An unknown version is `herdr-version-unsupported`.
    fn version(&self) -> Result<String, PaneError>;

    /// What Herdr says about the pane `pane` that a record does not hold: its working
    /// directory and when its screen last changed (#832). It is an observation (I6), read
    /// fresh and never stored; `pane list` and `pane get` show it for a pane no record
    /// names.
    ///
    /// An implementation that cannot tell says so with `None` fields (an empty
    /// [`PaneObservation`]) and never guesses. `pane-not-found` when Herdr has no such pane.
    ///
    /// Required since #835, with no default. The default answered an empty observation, so a
    /// wrapper port that did not forward it reported nothing, silently, over the real wiring;
    /// now the compiler finds every implementation.
    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError>;

    /// [`HerdrPort::observe_pane`] for several panes at once: one observation for each of
    /// `panes`, in the same order, where a pane that cannot be observed (Herdr does not have
    /// it, or the call failed) reports an empty one. A list of many unmanaged panes costs one
    /// call to Herdr when the implementation can read them together; the default asks about
    /// each pane in turn. A wrapper port overrides it to forward the batch, so that a list
    /// stays one call to the port it wraps.
    fn observe_panes(&self, panes: &[PaneId]) -> Vec<PaneObservation> {
        panes
            .iter()
            .map(|pane| self.observe_pane(pane).unwrap_or_default())
            .collect()
    }
}

/// What [`HerdrPort::observe_pane`] reports about one pane. Both fields are optional
/// because Herdr may not report them: Herdr 0.8.2 (protocol 20) reports no time at which a
/// screen last changed, so the real adapter leaves `last_screen_change` empty until a
/// source exists (#832).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PaneObservation {
    /// The pane's working directory, as Herdr reports it.
    pub cwd: Option<String>,
    /// When the pane's screen last changed (milliseconds since the Unix epoch), as Herdr
    /// reports it; never inferred from a file or from earlier reads.
    pub last_screen_change: Option<i64>,
}

impl PaneObservation {
    /// An observation with the pane's working directory.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// An observation with the time the pane's screen last changed.
    #[must_use]
    pub fn with_last_screen_change(mut self, at_ms: i64) -> Self {
        self.last_screen_change = Some(at_ms);
        self
    }
}

/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// No method has a default (#835), so an implementation that answers only the four methods
/// that came before `foreground_commands`, a wrapper port included, does not compile:
///
/// ```compile_fail,E0046
/// use holler_pane::{Argv, HostPort, PaneError, PaneName};
///
/// struct PidsOnly;
///
/// impl HostPort for PidsOnly {
///     fn ensure_session(&self, _: &PaneName, _: &str) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn run(&self, _: &PaneName, _: &str, _: &Argv) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn stop_owned(&self, _: &PaneName) -> Result<(), PaneError> {
///         Ok(())
///     }
///     fn ps(&self, _: &PaneName) -> Result<Vec<u32>, PaneError> {
///         Ok(Vec::new())
///     }
/// }
/// ```
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;

    /// Run `argv` (never through a shell) in the session `name`, starting it in `cwd`,
    /// which need not be the directory the session was made in (#802: the launch engine
    /// passes the pane spec's directory, for the spec's command and for the harness
    /// server alike).
    fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError>;

    /// Stop the processes the session `name` owns: what `run` started there, the pane's
    /// harness server among them (#802).
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;

    /// The process ids running in the session `name`.
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;

    /// The command name of the foreground process of every live pane in the session `name`,
    /// one per pane, in the order tmux lists them (#835): what tmux reports as
    /// `#{pane_current_command}`, the basename of that process's `argv[0]`. Names only, never
    /// an argv, a command line or a program's configuration (I7). A session with no pane
    /// process answers an empty list; a missing session is `pane-not-found`, as `ps` answers.
    ///
    /// The import reads it to leave alone a pane that runs Claude Code
    /// ([`crate::pane::is_claude_code_command`]).
    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>;
}

/// The harness server of a pane and its sessions (the adapter is
/// `holler-adapter-opencode`, #642). **Final:** the spike #635 confirmed it unchanged;
/// ADR-0021 section 2, "`HarnessPort` as built (#642)", records what the adapter decided,
/// and #802 amended how a server starts: the port builds its argv, and [`HostPort::run`]
/// starts it in the pane's tmux session, so [`HostPort::stop_owned`] stops it.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HarnessPort: Send + Sync {
    /// The argv that starts the harness server of `name` on `port` with `spec` applied,
    /// for [`HostPort::run`] to start. Pure: it starts no process, binds no port, writes
    /// no file and sends nothing.
    ///
    /// It never returns an argv that would start a server without what `spec` asks for.
    /// When `spec.agent` is `Some(key)`, some element of the argv contains the key's
    /// text; an implementation that cannot apply the agent refuses with
    /// [`AGENT_UNSUPPORTED`].
    fn serve_command(
        &self,
        name: &PaneName,
        port: u16,
        spec: &ServeSpec,
    ) -> Result<Argv, PaneError>;

    /// Whether the server running on `port` honours `spec`. The launch engine asks once
    /// the server is healthy, before any session is created on it (#802). `Ok` only when
    /// the server honours every field of `spec`; a server that does not is a refusal with
    /// an open code the implementation owns. The engine answers every error by rolling
    /// back (the host then stops the server). An implementation that cannot check a field
    /// it cannot apply refuses it, as [`HarnessPort::serve_command`] does
    /// ([`AGENT_UNSUPPORTED`] for an agent).
    ///
    /// It answers for the spec, not for whether the server is up: the engine's health wait
    /// has just seen the server answer, and a server gone since then fails the engine's
    /// next call to it (the session of record's), which rolls back the same way. So an
    /// implementation sends a request only for what it must check on the server, and that
    /// request answers an unreachable server as any call does (`unavailable`, `timeout`).
    /// A spec that the implementation's own argv already settles is answered `Ok(())` with
    /// nothing sent. That is the OpenCode adapter's answer to a spec without an agent: its
    /// argv applies none, so the server keeps its own default.
    ///
    /// Required, with no default: a harness that starts servers must say whether the
    /// running one honours the spec.
    fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError>;

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

    /// Queue `text` as a prompt to `session` on the server at `port`, through the
    /// harness API, never by typing into a TUI (I4). `pane reset --first` calls it (#645).
    ///
    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
    fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
        let _ = (port, session, text);
        Err(PaneError::Refused {
            code: PROMPT_UNSUPPORTED,
            message: "this harness does not queue a prompt".to_owned(),
        })
    }

    /// What `session` on the server at `port` is doing now (#645: `pane reset` refuses a
    /// conversation that is not [`Activity::Idle`]).
    ///
    /// The default answers [`Activity::Idle`], so an impl that does not report activity
    /// refuses nothing.
    fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
        let _ = (port, session);
        Ok(Activity::Idle)
    }
}

/// `prompt-unsupported`: the default of [`HarnessPort::send_prompt`], for a harness that
/// does not queue a prompt. Refusal, exit 3.
pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported");

/// `agent-unsupported`: [`HarnessPort::serve_command`] or [`HarnessPort::verify_serving`]
/// of a harness that cannot apply the agent a [`ServeSpec`] asks for at all (#802). A harness
/// that applies agents refuses one its running server cannot use with a code of its own (the
/// OpenCode adapter's `agent-unusable`, #642 part 3). Refusal, exit 3: the harness
/// understood the request and declines it; nothing went wrong.
pub const AGENT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("agent-unsupported");

/// What a harness session is doing, as [`HarnessPort::session_activity`] reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// Waiting for a prompt.
    Idle,
    /// Running a turn.
    Busy,
    /// Waiting for an answer to a question it asked.
    HoldingQuestion,
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
