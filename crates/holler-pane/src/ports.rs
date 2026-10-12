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
//! the spike #635 (OpenCode) confirmed it, and #642 built it. Amended since:
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
//!   process of a tmux session (how the import tells a pane that runs Claude Code). It is
//!   required so that the compiler finds every implementation, a wrapper port included: a
//!   default answers silently in place of a port a wrapper forgot to forward;
//! - #831's, a second harness (ADR-0021 section 15): every [`HarnessPort`] method keyed by a
//!   server's `port: u16` is keyed by a [`HarnessEndpoint`] instead (a port, or a single
//!   process), the change of the port that #802 decision 2 set as the pattern, so a call
//!   site that forgets the endpoint does not compile; `shown_session` and `attach_tui` take
//!   one too. Four default-armed methods for a single-process harness, each defaulting to
//!   the refusal [`PROCESS_UNSUPPORTED`] ([`HarnessPort::process_command`],
//!   [`HarnessPort::new_session_id`], [`HarnessPort::locate_process`],
//!   [`HarnessPort::login_status`]); [`ServeSpec`] gains the model and the directory; and
//!   [`Activity::WaitingPermission`]. [`HarnessRouter`] is the one router of the two
//!   shapes, so [`Ports`] keeps one harness.

mod process;
mod router;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::argv::{AgentKey, Argv};
use crate::error::{PaneError, RefusalCode};
use crate::grid::GridPos;
use crate::pane::{HerdrPane, ModelSpec, Pane, PaneEvent, PaneId, PaneName};
use crate::probe::{run_probe, ProbeResult};
use crate::profile::{ProfileScope, ProfileStore};

use self::process::process_unsupported;
pub use self::process::{
    HarnessEndpoint, LoginStatus, SessionChoice, ABORT_UNSUPPORTED, INBOUND_HELD,
    PROCESS_UNSUPPORTED, SERVER_UNSUPPORTED, SESSION_MISMATCH,
};
pub use self::router::HarnessRouter;

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

/// What a pane's harness must run with: the part of the pane's spec that
/// [`HarnessPort::serve_command`] applies to a server's argv and
/// [`HarnessPort::verify_serving`] checks on the running server (#802), and that
/// [`HarnessPort::process_command`] applies to a single process's argv (#831).
///
/// `#[non_exhaustive]`: the environment names and the context ceilings join it later with
/// no change to a signature. A new field of the pane's spec for its harness goes here, never
/// into a new parameter. Outside this crate a spec is built only from [`ServeSpec::default`]
/// and its builders, so a struct literal does not compile there, with or without
/// `..Default::default()`; the launch engine, inside this crate, builds it with every field
/// named.
///
/// The model and the directory (#831) are set by the launch engine only for a
/// single-process harness: an OpenCode server gets `None` for both until #829 applies the
/// model to it, so `serve_command`'s rule (never an argv without what the spec asks for)
/// holds for OpenCode unchanged.
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
    /// leaves the server's own default. A single-process harness that cannot apply an agent
    /// refuses one ([`AGENT_UNSUPPORTED`]).
    pub agent: Option<AgentKey>,
    /// The model a single process runs (`Pane.model`; #831): its model id and effort reach
    /// the argv, the provider is recorded and not passed. `None` for a server harness.
    pub model: Option<ModelSpec>,
    /// The directory a single process works in (`Pane.host.cwd`; #831), which also names
    /// where its conversations are kept. `None` for a server harness, which takes its
    /// directory from `HostPort::run`.
    pub cwd: Option<String>,
}

impl ServeSpec {
    /// This spec, with `agent` as the server's agent.
    #[must_use]
    pub fn with_agent(mut self, agent: AgentKey) -> Self {
        self.agent = Some(agent);
        self
    }

    /// This spec, with `model` as the model a single process runs.
    #[must_use]
    pub fn with_model(mut self, model: ModelSpec) -> Self {
        self.model = Some(model);
        self
    }

    /// This spec, with `cwd` as the directory a single process works in.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
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
    /// The default answers an empty [`PaneObservation`]: an implementation that cannot
    /// tell says so with `None` fields and never guesses, so one that does not override
    /// this keeps compiling and reports nothing. `pane-not-found` when Herdr has no such
    /// pane.
    fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError> {
        let _ = pane;
        Ok(PaneObservation::default())
    }

    /// [`HerdrPort::observe_pane`] for several panes at once: one observation for each of
    /// `panes`, in the same order, where a pane that cannot be observed (Herdr does not have
    /// it, or the call failed) reports an empty one. A list of many unmanaged panes costs one
    /// call to Herdr when the implementation can read them together; the default asks about
    /// each pane in turn.
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
    /// ([`crate::pane::is_claude_code_command`]), and a Claude Code launch to refuse a session
    /// that already runs one (`process-in-use`, #831).
    fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>;
}

/// The harness of a pane and its sessions: an OpenCode server and its TUI (the adapter is
/// `holler-adapter-opencode`, #642), or one Claude Code process (`holler-adapter-claude`,
/// #831). **Final:** the spike #635 confirmed it unchanged; ADR-0021 section 2, "`HarnessPort`
/// as built (#642)", records what the OpenCode adapter decided, #802 amended how a server
/// starts (the port builds its argv, and [`HostPort::run`] starts it in the pane's tmux
/// session, so [`HostPort::stop_owned`] stops it), and #831 keyed every call by a
/// [`HarnessEndpoint`] and added the four methods of a single-process harness (section 15).
///
/// A harness of one shape answers the other shape's calls with a stable refusal: a server
/// harness answers the four process methods [`PROCESS_UNSUPPORTED`] (their default), and a
/// single-process harness answers a server's and a TUI's steps [`SERVER_UNSUPPORTED`]. The
/// engines choose their steps by the kind's shape (`HarnessKind::shape`), and
/// [`HarnessRouter`] sends each call to the harness of its endpoint, so neither refusal is
/// met on a well-formed path.
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

    /// Whether the server at `at` honours `spec`. The launch engine asks once the server is
    /// healthy, before any session is created on it (#802). `Ok` only when the server
    /// honours every field of `spec`; a server that does not is a refusal with an open code
    /// the implementation owns. The engine answers every error by rolling back (the host
    /// then stops the server). An implementation that cannot check a field it cannot apply
    /// refuses it, as [`HarnessPort::serve_command`] does ([`AGENT_UNSUPPORTED`] for an
    /// agent).
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
    fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError>;

    /// Whether the harness at `at` answers: the server on a port, or a single process that
    /// still runs (its entry is listed for its pid).
    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError>;

    /// Create a session on the server at `at`; returns its id.
    fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError>;

    /// The session ids on the server at `at`.
    fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError>;

    /// Abort the running turn of `session` at `at`. A harness whose kind's abort style is a
    /// relaunch (`AbortStyle::Relaunch`) refuses it with its own code (Claude Code's
    /// `abort-unsupported`): it has no in-place abort, and a signal is not one.
    fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError>;

    /// Attach the TUI of `pane` to `session` on the server at `at`.
    fn attach_tui(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
        session: &str,
    ) -> Result<(), PaneError>;

    /// Switch the TUI of `pane` to `session`.
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;

    /// The session the pane `pane`, whose harness is at `at`, shows, if it can tell: the
    /// session its TUI shows, or the one a single process holds now (which follows a person's
    /// `/clear` or `/resume`). `None` when it cannot tell, or nothing is shown.
    fn shown_session(
        &self,
        pane: &PaneId,
        at: &HarnessEndpoint,
    ) -> Result<Option<String>, PaneError>;

    /// Queue `text` as a prompt to `session` at `at`, through the harness API, never by
    /// typing into a TUI (I4). `pane reset --first` calls it (#645).
    ///
    /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
    /// prompts says so as a stable refusal, which the real adapter replaces (#642).
    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let _ = (at, session, text);
        Err(PaneError::Refused {
            code: PROMPT_UNSUPPORTED,
            message: "this harness does not queue a prompt".to_owned(),
        })
    }

    /// What `session` at `at` is doing now (#645: `pane reset` refuses a conversation that
    /// is not [`Activity::Idle`]). A single process holds one session at a time, so its
    /// activity is the process's.
    ///
    /// The default answers [`Activity::Idle`], so an impl that does not report activity
    /// refuses nothing.
    fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
        let _ = (at, session);
        Ok(Activity::Idle)
    }

    /// The argv that starts the single process of `name`, holding the session `session`
    /// names, with `spec` applied (#831), for [`HostPort::run`] to start. Pure, like
    /// [`HarnessPort::serve_command`]: it starts no process and sends nothing, so its
    /// refusals belong to a plan. It never returns an argv that would start the process
    /// without what `spec` asks for, and a choice it cannot honour is refused
    /// (`session-not-found` for a [`SessionChoice::Resume`] of a session with no
    /// conversation).
    ///
    /// Default-armed, refusing [`PROCESS_UNSUPPORTED`]: only a single-process harness
    /// answers it, and the engines ask it only for a kind of that shape.
    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        let _ = (name, spec, session);
        Err(process_unsupported("process_command"))
    }

    /// A fresh session id for a single process's launch or reset (#831): the randomness is
    /// the harness's, the engine takes none. Default-armed, refusing [`PROCESS_UNSUPPORTED`].
    fn new_session_id(&self) -> Result<String, PaneError> {
        Err(process_unsupported("new_session_id"))
    }

    /// The pid of the running single process of `name` that holds `session`, or `None`
    /// (#831): how a launch learns the pid it records, after which the pid is the process's
    /// identity. Default-armed, refusing [`PROCESS_UNSUPPORTED`].
    fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        let _ = (name, session);
        Err(process_unsupported("locate_process"))
    }

    /// Whether this harness's processes use the subscription login (#831, I7): only that
    /// fact is read, never the login. Default-armed, refusing [`PROCESS_UNSUPPORTED`] like
    /// the other three, never a silent answer (plan review W9): a wrapper that forgot to
    /// forward it fails loudly.
    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        Err(process_unsupported("login_status"))
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
    /// Waiting for a person to answer a permission prompt (#831; Claude Code's `waiting`
    /// with `permission prompt`, `sandbox request` or `worker request`). Holler does not
    /// answer one in this story.
    WaitingPermission,
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
