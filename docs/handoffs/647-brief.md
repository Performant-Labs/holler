# Brief: #647 the reconcile engine and `holler pane doctor` (observe reality, compare with the record, repair or report)

Repo: Performant-Labs/holler. Issue: #647 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**Branch:** `issue-647-implementation` (worktree `.claude/worktrees/0647-reconcile-doctor`, from `origin/main` at `3bdd129`).
**Review rigor:** second-opinion (the orchestrator's instruction for this run; the issue's own Pipeline line says
`in-session`, see "Contradictions found", C-1). The outside reviewer sees only this brief, so every fact below is pasted
from the code with its file and line.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see the table under "Decisions".
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 2, 3, 8, 9, 11 and 12, the epic's contract and "Skeleton
split" rulings. This story **closes one item ADR-0021 defers to it by name** (the hub timer, section 12 and "Deferred to
named stories") and edits ADR-0021 for it, in this change (Decision 1).
**Handoffs:** `docs/handoffs/647/handoff-<phase>.md`; the decision journal is `docs/handoffs/647/decisions.md`.
**Public repository:** no personal host, tailnet or account name goes into code, tests, docs, the CHANGELOG, commit
messages or the PR. Test data uses the test kit's neutral names (`demo-c1r1`, `scratch`, `localhost`, ports 48100-48199).

## Problem

On 2026-10-07 five records each claimed to know which session a pane was on, and nothing compared any of them with what
was really running: an order ran in a session the pane did not show, a three-day wedged OpenCode server went unnoticed, a
bare unregistered OpenCode ran in the orchestrator's pane, a hub entry survived its process, every launch left a stray
"ping" session, and a repair re-registered the wrong session because it guessed "the more recently active". The pane
registry (#639) now holds one record per pane with one `session_of_record`, but nothing observes reality against it:
`reconcile.rs` and `findings.rs` in `holler-pane` are empty stubs and `holler pane doctor` answers `not-implemented`.

This story builds the reconcile pass (a pure engine over the frozen ports), the typed findings it reports, and the
`holler pane doctor [PANE] [--fix] [--profile NAME]` verb that runs it. `--fix` repairs only what the record decides and
never changes `session_of_record`; everything else is reported with the exact command to run, or with a stated reason
why no command exists.

## Evidence (verbatim, as of `3bdd129`)

### The stubs this story fills

```
crates/holler-pane/src/findings.rs:1-3
//! The finding kinds reconcile and `holler pane doctor` report. Empty stub declared by
//! #637 so that no two stories edit `lib.rs`; story #647 fills it, and #665 adds only
//! the `ProfileDrift` kind after #647 has merged.
```
```
crates/holler-pane/src/reconcile.rs:1-3
//! The reconcile engine: observe Herdr, tmux and the harness, compare with the
//! registry and report; it never infers state from files (I6). Empty stub declared by
//! #637 so that no two stories edit `lib.rs`; story #647 fills it.
```
```
crates/holler-pane/src/lib.rs:52
pub mod reconcile;
crates/holler-pane/src/lib.rs:42
pub mod findings;
```
```
crates/holler-cli/src/pane/doctor.rs:1-23
//! `holler pane doctor`: a stub (story #670). Story #647 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 647;

/// Check panes against Herdr, tmux and the harness and report what differs.
#[derive(Args, Debug)]
pub struct PaneDoctor {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane doctor`: refuse, naming the story that owns it.
pub fn run(_args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
```
crates/holler-cli/tests/pane_verbs/doctor.rs:1-9
//! `holler pane doctor`: the stub case of story #670. Story #647 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn pane_doctor_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "doctor"], 647);
}
```
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:9-17, 32-33
/// Every verb that is still a stub, with the story that owns it. The only place in the
/// shared process tests that names a stub's owning story.
///
/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
/// line**: two stories that delete whole groups, header included, delete adjacent lines,
/// and git reports that as a conflict. (`PANE_VERBS` and `PROFILE_VERBS` keep the verb
/// itself.)
pub const STUBS: &[(&str, &str, u32)] = &[
    // #647
    ("pane", "doctor", 647),
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:146-148
# #647
pane doctor |
pane doctor | --profile demo --format=json
```
```
docs/adr/ADR-0003.md:61
holler pane doctor [--profile NAME]                               #647
docs/adr/ADR-0003.md:92 (excerpt)
... each owning story adds its verb's positionals and flags to its own row, with its own line in `cli-surface.txt`, and edits no other verb's. ...
```

### The frozen ports and records (holler-pane, #637; not edited here)

```
crates/holler-pane/src/ports.rs:62-82
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
```
```
crates/holler-pane/src/ports.rs:126-148
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
```
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
```
```
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
```
```
crates/holler-pane/src/ports.rs:226-235
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
```
Every port trait has `Send + Sync` as supertraits (`ports.rs:62, 126, 156, 176, 208`; `profile.rs:328, 380`), so `Ports<'a>`
can be copied into `std::thread::scope` threads (Decision 9).

```
crates/holler-pane/src/pane.rs:102-115
/// The machine side of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostInfo {
    /// The host's name.
    pub name: String,
    /// The tmux session name (equal to the pane's name).
    pub tmux: String,
    /// The project directory or worktree the pane works in.
    pub cwd: String,
    /// The Herdr API version, recorded by the Herdr adapter (#640) on connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub herdr_api_version: Option<String>,
}
```
```
crates/holler-pane/src/pane.rs:131-150
/// What the harness server last reported about its health.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    /// Unhealthy, with the reason.
    Unhealthy(String),
    Unknown,
}

/// The harness server of a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessInfo {
    pub kind: HarnessKind,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub health: Health,
}
```
```
crates/holler-pane/src/pane.rs:179-190
/// What reconcile last observed about which session the pane shows and which the hub
/// drives. Written by reconcile, never inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastObserved {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driven: Option<String>,
    /// When it was observed (milliseconds since the Unix epoch).
    pub at: i64,
}
```
```
crates/holler-pane/src/pane.rs:225-254
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub name: PaneName,
    /// Bumped on every change.
    pub generation: u64,
    pub herdr: HerdrPane,
    pub host: HostInfo,
    pub harness: HarnessInfo,
    /// THE session: the TUI is attached to it and the hub drives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_of_record: Option<String>,
    pub role: PaneRole,
    pub hold: Hold,
    pub last_observed: LastObserved,
    /// The profile the pane belongs to; a pane belongs to at most one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Recorded by launch/relaunch (#644) and import (#650), so
    /// `profile create --from-current` copies them from the store.
    pub model: ModelSpec,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
    pub env: Vec<EnvVarName>,
    pub context: ContextCeilings,
    /// The launch command: an argv array, never a shell string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Argv>,
    pub probe: PaneProbe,
}
```
```
crates/holler-pane/src/pane.rs:93-100
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
}
```
```
crates/holler-pane/src/profile.rs:268-273
/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}
crates/holler-pane/src/profile.rs:380-389
pub trait ProfileScope: Send + Sync {
    /// The profile and the panes of it a verb acts on. With no `pane`, every pane
    /// of the profile; with a named pane, just that one, which must belong to the
    /// profile (`pane-not-in-profile` otherwise). A missing profile is
    /// `profile-not-found`.
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError>;
```
```
crates/holler-pane/src/profile.rs:36-42
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProfileName(String);

impl ProfileName {
    /// Parse a display name (see the type docs for the rules).
    pub fn parse(text: &str) -> Result<Self, PaneError> {
```
Pane names are shell-safe by grammar (`PaneName` wraps `holler_proto::vocab::SessionName`, `pane.rs:31-42`):
```
crates/holler-proto/src/vocab.rs:202-223
fn check_segment(seg: &[u8]) -> Result<(), NameError> {
    if seg.is_empty() {
        return Err(NameError::Empty);
    }
    if seg.len() > MAX_SEGMENT_LEN {
        return Err(NameError::TooLong);
    }
    if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
        return Err(NameError::BadCharacter);
    }
    for &c in seg {
        if !(is_word(c) || c == b'-') {
            return Err(NameError::BadCharacter);
        }
    }
    Ok(())
}

#[inline]
fn is_word(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z')
}
```
`GridPos` output (`grid.rs`, frozen):
```
crates/holler-pane/src/grid.rs:68-73
impl fmt::Display for GridPos {
    /// The `rRcC` form, row first: `r2c1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}c{}", self.row, self.col)
    }
}
crates/holler-pane/src/grid.rs:129-138
impl Serialize for GridPos {
    /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("GridPos", 3)?;
        out.serialize_field("row", &self.row)?;
        out.serialize_field("col", &self.col)?;
        out.serialize_field("pos", &self.to_string())?;
        out.end()
    }
}
```
The errors used (closed set, not edited; `error.rs` is frozen, ruling 3):
```
crates/holler-pane/src/error.rs:453-471 (excerpt: the variants used, verbatim; "..." marks omitted lines)
    HerdrVersionUnsupported { message: String },
    ...
    Timeout { op: String },
    ...
    PaneNotFound { what: String },
    ...
    Unavailable { what: String },
    /// `profile-drift`: a live pane differs from its profile's spec. Listed for
    /// convenience: it is a reconcile finding kind (#647/#665), not an error a port
    /// returns.
    ProfileDrift { message: String },
crates/holler-pane/src/error.rs:686-695
/// `text` quoted for an error message, cut to 64 characters so an oversized input
/// cannot produce an oversized message.
pub(crate) fn excerpt(text: &str) -> String {
    const LIMIT: usize = 64;
    if text.chars().count() <= LIMIT {
        return format!("{text:?}");
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head:?}...")
}
```
`RefusalCode::from_static` refuses a closed code (`error.rs:331-337`), so finding kinds whose text equals a closed code
(`herdr-version-unsupported`, and #665's `profile-drift`) **cannot** be `RefusalCode`s: findings are their own enum.

### The CLI output module (#670/#676; called, not edited)

```
crates/holler-cli/src/output.rs:195-217
pub struct VerbCtx<'a> {
    pub format: Format,
    pub ports: Ports<'a>,
    pub sink: Sink<'a>,
}

/// Print one result and return the exit code: 0 ok, else the exit code of the error's class
/// (1 runtime failure, 2 usage, 3 refusal).
///
/// `text` renders the data for text mode (JSON mode serializes the data itself and never calls
/// it). The text ends with one newline, which `emit` adds when it is missing; empty text writes
/// nothing.
pub fn emit<T: Serialize>(
    sink: &mut Sink<'_>,
    format: Format,
    result: Result<T, ErrorBody>,
    text: impl FnOnce(&T) -> String,
) -> i32 {
```
```
crates/holler-cli/src/output.rs:134-141
impl From<&PaneError> for ErrorBody {
    fn from(error: &PaneError) -> Self {
        Self {
            code: ErrorCode::from(error),
            message: error.to_string(),
        }
    }
}
```
The envelope rules the JSON output must pass (test kit, #681):
```
crates/holler-pane-testkit/src/envelope.rs:25-33
//! 6. `ok` is the boolean `true` when the exit code is 0, and `false` otherwise.
//! 7. On success, `error` is `null`. `data` may be any value, `null` included.
//! 8. On failure, `error` is an object.
//! 9. On failure, `data` is `null`.
//! 10. `error` has exactly the keys `code` and `message`. A missing one is named first
//!     (`error.code`, then `error.message`), then the smallest extra one (`error.<key>`).
//! 11. `code` is a string that [`is_valid_code`] accepts.
//! 12. `message` is a string that is not blank and has no `\n` or `\r`.
//! 13. The exit code matches the class of the code, `class_of(code).exit_code()`.
```
The in-process verb harness the tests use (#670; not edited):
```
crates/holler-cli/tests/verb_harness/mod.rs:52-54
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
```
`holler-pane-testkit` is already a dev-dependency of `holler-cli` (`crates/holler-cli/Cargo.toml:432-435`, "#638 and
#643-#647 use it from their own pane_verbs/<verb>.rs files"), and `tests/pane_verbs/main.rs:34` already declares
`mod doctor;`. `holler-pane` has **no** dev-dependency on the test kit (`crates/holler-pane/Cargo.toml:14-21`), so the
engine's port-driven tests live in `tests/pane_verbs/doctor.rs` (Decision 12).

### The fakes the tests run against (holler-pane-testkit, #638; not edited)

```
crates/holler-pane-testkit/src/harness.rs:110-137
/// A call that *reaches* a port goes on when the server there runs. When the server is
/// frozen, the call answers `timeout` with the method's [`HarnessOp`] as its `op`, the
/// shape a wedged port answers; when it was killed or never served, the call answers
/// `unavailable` ("the harness server on port N"). Each method, in check order:
///
/// - `serve(name, port)`: on a frozen server, `timeout`; on one running for `name`, its
///   pid (idempotent); on one running for another pane, `unavailable` (the port is in
///   use). Otherwise it starts a server with a new pid, counted from 20 000 and never
///   reused. It creates no session: a fresh server has none
///   (opencode-pane-spike.md:69-70).
/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
///   never served, as a timed request that times out or is refused would tell.
/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
///   directory, or list that directory's sessions in creation order.
/// - `abort(port, session)` reaches the port. A session outside its data directory is
///   `session-not-found`; otherwise the abort is logged ([`FakeHarness::aborts`]).
/// - `attach_tui(pane, port, session)` reaches the port. A session outside its data
///   directory is `session-not-found`, and the pane keeps the TUI it had; otherwise the
///   pane's TUI, replacing any earlier one, shows the session.
/// - `select_session(pane, session)`: a pane with no TUI is `unavailable` ("no TUI in
///   pane P"), checked first because the TUI is what names the server. It then reaches
///   the TUI's port. A session outside that port's data directory is
///   `session-not-found`, and the screen stays; otherwise that TUI, and no other, shows
///   the session.
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
```
```
crates/holler-pane-testkit/src/harness.rs:104-108
/// It holds the servers by port, a data directory per port, the sessions of each data
/// directory in creation order, the TUI of each pane, the quirks that are on and the
/// aborts the servers acknowledged. Every port shares the data directory `"default"`,
/// as the live fleet's servers share one (opencode-pane-spike.md:75-77), until
/// [`FakeHarness::set_data_dir`] gives a port its own.
```
The scenario seams used by the incident tests:
```
crates/holler-pane-testkit/src/harness.rs:195-197, 201-203, 214-216, 221-223, 229-231, 236-243
    pub fn set_data_dir(&self, port: u16, dir: &str) {
        self.lock().dirs.insert(port, dir.to_owned());
    }
    pub fn freeze(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Frozen)
    }
    pub fn kill(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Killed)
    }
    pub fn seed_session(&self, port: u16) -> String {
        self.lock().mint_session(port)
    }
    pub fn delete_session(&self, session: &str) -> Result<(), PaneError> {
        self.lock().delete(session)
    }
    pub fn navigate(&self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError> {
        let mut world = self.lock();
        let port = world.tui_port(pane)?;
        if let Some(session) = session {
            world.known(port, session)?;
        }
        world.show(pane, session)
    }
```
```
crates/holler-pane-testkit/src/harness.rs:12-15
//! The host and harness fakes share no state. `FakeHost::stop_owned` does not stop a
//! server of this fake, and a harness pid is never in `FakeHost::ps`. A test that needs
//! the two to agree drives both, e.g. with a `HostPort` wrapper whose `stop_owned` also
//! calls [`FakeHarness::kill`].
```
```
crates/holler-pane-testkit/src/host.rs:54-61, 148-156
/// - `run` in a missing session is `pane-not-found`, and an empty argv is `usage`.
///   Otherwise it starts one process in the session and records the argv exactly as
///   given: an element is never joined to another or re-split, and nothing goes
///   through a shell.
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
    /// The tmux session `name` ended outside Holler (it was killed): the session and
    /// its processes are gone. `pane-not-found` when there is no such session.
    pub fn end_session(&self, name: &PaneName) -> Result<(), PaneError> {
        self.lock()
            .sessions
            .remove(name)
            .map(drop)
            .ok_or_else(|| not_found(name))
    }
```
```
crates/holler-pane-testkit/src/herdr.rs:223-229, 205-207
    /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is
    /// free, its id is never reused, and every later call naming it is
    /// `pane-not-found`. It bypasses the faults and the call log. `pane-not-found` when
    /// no such pane exists.
    pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock().remove(pane)
    }
    pub fn set_version(&self, version: HerdrVersion) {
        self.lock().version = version;
    }
crates/holler-pane-testkit/src/herdr.rs:314-330
    fn version(&self) -> Result<String, PaneError> {
        self.faults.enter(HerdrOp::Version)?;
        let version = self.lock().version;
        match version {
            HerdrVersion::Protocol22 => Ok(PROTOCOL_22_VERSION.to_owned()),
            // ASSUMPTION (#640): only `version()` refuses an unsupported build. Whether
            // the adapter also refuses every other call after a failed version check is
            // #640's.
            HerdrVersion::Unsupported => Err(PaneError::HerdrVersionUnsupported {
                message: format!(
                    "Herdr reports version {UNSUPPORTED_VERSION}; the supported one is \
                     {SUPPORTED_VERSIONS}"
                ),
            }),
        }
    }
```
```
crates/holler-pane-testkit/src/fault.rs:26-35, 79-88
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
    /// timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
    Fail(PaneError),
}
    /// Make every call sleep for `delay` before it answers, as a slow port does
    /// (`None`: no delay).
    pub fn set_delay(&self, delay: Option<Duration>) {
        self.lock().delay = delay;
    }

    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
        self.lock().calls.clone()
    }
```
```
crates/holler-pane-testkit/src/profile_scope.rs:34-38, 70
/// - `resolve(P, None)` is the stored P and every pane whose record names P, in name
///   order. `resolve(P, Some(n))` is P and n, whose record must name P, or else
///   `pane-not-in-profile` (a pane with no record included). A missing P is
///   `profile-not-found`, checked first. Membership is `Pane.profile` alone, compared by
///   slug, so a spec of P adds no pane to its scope.
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
```
```
crates/holler-pane-testkit/src/fixture.rs:34-41
/// A valid, deterministic `Pane` named `name`: generation 0, grid `r1c1`, no profile,
/// no session of record, harness port 48100 with its health unknown, an agent with no
/// hold, no command and no probe. Its Herdr session and workspace are scratch names,
/// never a live session's. Its tmux session is the pane's name, as on a real pane, so
/// pick a neutral one such as `demo-c1r1`. Two calls with one name return equal panes.
///
/// `usage` when `name` is not a valid pane name.
pub fn sample_pane(name: &str) -> Result<Pane, PaneError> {
```
**`grep -rn 'ASSUMPTION *(#647' crates/holler-pane-testkit` prints nothing**: the test kit made no assumption on this
story's behalf (C-2).

### The hub's rule for a periodic writer, and the ADR text this story relies on or edits

```
crates/holler-hub/src/panes/mod.rs:27-28
//! - Each write rewrites the whole file and wakes every watcher. A periodic writer, such as
//!   reconcile writing `last_observed` (#647), must skip a record that has not changed.
```
```
docs/adr/ADR-0021.md:342
| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
docs/adr/ADR-0021.md:452-453
**Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
registered pane, and when it moves after a switch or reset.
docs/adr/ADR-0021.md:457-460 (excerpt)
**Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. ... A crash between steps leaves state that
the next pane doctor run finds and reports (#644's acceptance). No verb leaves work running after it exits.
docs/adr/ADR-0021.md:470-471
**Deferred to #647:** the "hub timer" for reconcile. Under ruling 1 it has no home in the hub; a periodic pass runs in a
CLI or daemon context, decided in #647's brief.
docs/adr/ADR-0021.md:481
| `owned-sessions` | the sessions the launcher created for a pane | `session_of_record`; any other session on the pane's server is a stray-session finding from reconcile (#647) |
docs/adr/ADR-0021.md:524
- A periodic reconcile outside the hub: #647.
docs/adr/ADR-0021.md:171-173
**Decided (ruling 1): verbs run CLI-side, against the ports; the hub is the store only.** The hub holds the pane registry
and the profile registry and answers their store methods; it constructs no adapter and runs no verb. The adapters are
constructed in `holler-cli/src/pane/wiring.rs` (#670 creates it, #649 fills it).
docs/adr/ADR-0021.md:126-129 (scoping class)
- **Scoping** (`list`, `get`, `watch`, `doctor`, `switch`, `reset`, `park`, `unpark`, the `--pane` forms of `say`, `interrupt`
  and `answer`, and the roster): act on P's panes. A named pane outside P is `pane-not-in-profile`. The read verbs and
  `doctor`, `park` and `unpark` with no pane name mean every pane of P; `say`, `interrupt`, `answer`, `switch` and `reset`
  still need a pane name.
```

### What the unmerged adapters will do (not on `main`; quoted so the reviewer sees the dependency)

The amended #642 brief (branch `issue-642-implementation`, `docs/handoffs/642-brief.md:447, 484, 489-491`), which the
`FakeHarness` semantics above already fix: an OpenCode connection that is refused maps to `unavailable`, a frozen server
(connected, no answer) to `timeout`; `health` is `Ok(false)` for refused, timed out or garbled; `list_sessions` returns
**top-level sessions only** ("An element whose `parentID` is a string is a child (subagent) session and is left out").
The verb story signatures named by remedies: #645's issue fixes `switch PANE SESSION` and `reset PANE [--first TEXT]`;
#646's fixes `close PANE`; #644 (`relaunch`) has not fixed its positional yet (Risks, R-4).

## Acceptance criteria

All run from the worktree. Every test named below lives in `crates/holler-cli/tests/pane_verbs/doctor.rs` (or a
submodule of it, Decision 12) and uses only the test kit's fakes; `cargo test -p holler-cli --test pane_verbs doctor`
runs them. "The rig" is the test helper of Decision 12.

**Incidents (the issue's four, plus the two the epic also lists):**
1. `incident_tui_on_new_empty_session_while_hub_drives_the_old_one`: pane `demo-c1r1` with record S1, its TUI moved by
   `FakeHarness::navigate` to a fresh session S2 (`seed_session`). `doctor` (no `--fix`) exits 0 and reports exactly one
   `shown-driven-mismatch` for `demo-c1r1` with `session` S2 and `fix` `fixable`, plus one `stray-session` for S2; the
   stored record afterwards has `last_observed.shown == Some(S2)`, `last_observed.at ==` the request's `now_ms`, and
   `session_of_record == Some(S1)`.
2. `incident_three_day_wedged_server`: `FakeHarness::freeze(port)`. One `server-wedged` for the pane, remedy
   `holler pane relaunch demo-c1r1`; the record's `harness.health` becomes `{"unhealthy": "server-wedged"}`.
3. `incident_bare_unregistered_harness_in_the_orchestrators_pane`: an `orchestrator` pane whose TUI is attached to a
   second server (another port with its own data directory, `set_data_dir`) that no record names. One
   `tui-foreign-session` for that pane, remedy `holler pane relaunch <pane>`, `fix` `not-fixable`. Plus
   `unregistered_herdr_pane_is_reported`: a Herdr pane (made by `ensure_pane` on the fake) that no record names gives one
   `unregistered-herdr-pane` with its `grid` and `herdr_pane` id, `pane` null, `remedy` null, in a whole-fleet run only.
4. `incident_registered_pane_whose_process_died`: `FakeHarness::kill(port)`. One `server-down`, remedy relaunch. Plus
   `tmux_session_gone_is_reported` (`FakeHost::end_session` -> `tmux-session-missing`) and `herdr_pane_gone_is_reported`
   (`FakeHerdr::vanish` -> `herdr-pane-missing`).
5. `incident_stray_ping_session`: a session seeded on a pane's server that no record names gives one `stray-session`.
   With two panes on two ports sharing the fake's default data directory, the same stray is **one** finding whose
   `ports` lists both ports, sorted.
6. `incident_deleted_session_under_a_tui`: `delete_session(S1)` (the TUI goes home). Reports `session-of-record-missing`
   (remedy `holler pane reset demo-c1r1`) and `shown-driven-mismatch` with `fix` `not-fixable`.

**`--fix` (never changes the session of record; a second run reports nothing new):**
7. `fix_selects_the_session_of_record_and_never_changes_it`: on AC 1's rig, `doctor --fix` reports the mismatch with
   `fix` `fixed` and `remedy` null; afterwards `FakeHarness::tui(pane).shown == Some(S1)`, the stored record equals the
   record before the run in every field except `generation`, `last_observed` (`shown == Some(S1)`) and `harness.health`;
   `session_of_record == Some(S1)`. The harness call log holds exactly one `SelectSession`; no `CreateSession`, `Serve`,
   `AttachTui` or `Abort`; the Herdr call log holds no `SendText` or `SendKeys` (I4); the host call log holds only `Ps`.
8. `second_fix_run_reports_nothing_new`: a second `doctor --fix` after AC 7: its set of `(kind, pane, session)` is a subset
   of the first run's, holds no `shown-driven-mismatch`, and the pane store's call log for the second run holds no
   `CasPut` (nothing changed, nothing written: the hub rule at `panes/mod.rs:27-28`).
9. `fix_never_guesses_a_session`: record `None`, two sessions on the server, the TUI on one of them. `--fix` makes no
   `SelectSession`, writes no `session_of_record`, and reports `no-session-of-record` with remedy
   `holler pane reset demo-c1r1`; with record S1 deleted (AC 6's rig) `--fix` also makes no `SelectSession`.
10. `fix_skips_the_orchestrator_unless_named`: an orchestrator pane with a mismatch. `doctor --fix` reports it `skipped`
    with remedy `holler pane doctor <pane> --fix` and makes no `SelectSession`; `doctor <pane> --fix` fixes it.
11. `fix_failure_is_reported`: `fail_next(HarnessOp::SelectSession, Unavailable)`: the finding's `fix` is `failed`,
    `fix_error.code == "unavailable"`, remedy `holler pane relaunch demo-c1r1`; exit 0.

**Read-only without `--fix`:**
12. `doctor_without_fix_makes_only_read_calls`: on a rig with every incident at once, the harness log holds only
    `Health`, `ListSessions`, `ShownSession`; Herdr only `Snapshot`, `Version`; host only `Ps`; pane store only `List`
    (and `Get` for a named pane) and `CasPut`; profile store and prober none.
13. `unchanged_observation_writes_nothing`: two runs with no change between them: the second run's pane store log holds
    no `CasPut`, and every record's generation is unchanged by it.

**Scope (`[PANE]`, `--profile`; ADR-0021 section 3):**
14. `doctor_profile_reports_only_its_panes`: two profiles P and Q with a mismatch in each, built with
    `FakeProfileScope`. `doctor --profile P` reports only P's pane findings and no `unregistered-herdr-pane`; with Q's
    port on a data directory of its own (`set_data_dir`), a stray seeded there is not reported; and, with both ports on
    the shared default directory, the session of record of Q's pane, which P's server lists too, is **not** a stray.
15. `doctor_named_pane_scopes`: `doctor demo-c1r1` reports only that pane's findings. Refusals, each in both formats:
    an unknown pane -> `pane-not-found`, exit 3; `--profile P demo-c2r1` for a pane outside P -> `pane-not-in-profile`,
    exit 3; `--profile missing` -> `profile-not-found`, exit 3; `doctor BAD_NAME` -> `usage`, exit 2.
16. `doctor_envelope_and_exit_codes_match_across_formats`: for the rigs of AC 1, 2, 4, 14 and every refusal of AC 15,
    the JSON stdout passes `holler_pane_testkit::envelope::check_envelope(stdout, code)`, and the text-mode exit code
    equals the JSON-mode exit code.
17. `doctor_store_failure_is_an_error`: `FakePaneStore` `Fault::Fail(Unavailable)` -> one error envelope coded
    `unavailable`, exit 1, `data` null.

**Output (rowcol, format, safety):**
18. `finding_prints_rowcol`: pane `demo-c1r2` placed at row 2, col 1 with a mismatch. The text finding line starts with
    `demo-c1r2 r2c1 shown-driven-mismatch: `; `text.replace("demo-c1r2", "")` does not contain `c1r2`; in JSON the
    finding's `grid` is exactly `{"row":2,"col":1,"pos":"r2c1"}` and so is the pane summary's.
19. `json_data_shape_is_pinned`: the `data` of AC 1's run has exactly the keys `scope`, `fix_requested`, `herdr`, `hosts`,
    `panes`, `findings`; each finding exactly `kind`, `pane`, `grid`, `herdr_pane`, `session`, `ports`, `message`,
    `remedy`, `fix`, `fix_error` (absent values are `null`, never left out).
20. `herdr_version_shown_and_unsupported_reported`: with `HerdrVersion::Protocol22`, `data.herdr.version` equals
    `PROTOCOL_22_VERSION` and `data.hosts` lists each `(host.name, host.herdr_api_version)` of the panes in scope once;
    with `HerdrVersion::Unsupported` there is exactly one `herdr-version-unsupported` finding (pane null) whose message
    contains `SUPPORTED_VERSIONS`, the pass still checks every pane, exit 0.
21. `observe_failure_is_a_finding_not_an_abort`: `FakeHerdr` `Fault::Wedged` -> exactly two `observe-failed` findings,
    one whose message names `herdr.snapshot` and one naming `herdr.version`, and `data.herdr.version` is null; no `herdr-pane-missing` and no `unregistered-herdr-pane` is reported (nothing was
    observed); every pane is still checked; exit 0.
22. `doctor_output_holds_no_secret`: a pane with `command` `["prog", "--opt", "doctor-must-not-print-this-647"]`
    and `env` `["HLR_SENTINEL_647"]`, in every rig state: neither format's stdout or stderr contains
    `doctor-must-not-print-this-647` (the command is never printed). The sentinel is deliberately low-entropy and not
    after a `--token`-like flag, so the repo's gitleaks hook does not flag the test file as a credential.
23. `text_output_escapes_control_characters`: a record whose `host.name` is `"demo\u{1b}[2J\nhost"`: the text output
    contains no `\u{1b}` byte, and the host line is one line (the value is shown `{:?}`-quoted).
24. `observation_runs_concurrently`: four panes over a test-local `HarnessPort` wrapper around `FakeHarness` whose
    `health` increments an in-flight counter and waits (condvar, at most 2 s) until four calls are in flight, recording
    the maximum. The recorded maximum is 4. (An invariant, not a duration: a sequential engine records 1 and the test
    fails; tester overlay, "Timing races are asserted as invariants".)
25. `finding_kind_codes_are_stable`: `FindingKind::ALL` has 12 entries; every `code()` is unique, passes
    `holler_pane::error::is_valid_code`, and equals the kind's JSON serialization; the list equals, in order:
    `herdr-version-unsupported, observe-failed, unregistered-herdr-pane, herdr-pane-missing, tmux-session-missing,
    server-wedged, server-down, no-session-of-record, session-of-record-missing, tui-foreign-session,
    shown-driven-mismatch, stray-session`.
26. `doctor_remedies_parse`: each `holler pane doctor ...` remedy the rigs produce parses with `holler_cli::Cli::
    try_parse_from` (the relaunch and reset remedies are pinned as strings only, R-4).

**Surface and docs:**
27. ADR 0003 line 61 becomes `holler pane doctor [PANE] [--fix] [--profile NAME]`, then spaces to the existing column,
    then `#647` (`grep -c '^holler pane doctor \[PANE\] \[--fix\] \[--profile NAME\] *#647$' docs/adr/ADR-0003.md` prints
    `1`); the `# #647` group of `cli-surface.txt`
    is exactly the five lines of Decision 11; `("pane", "doctor", 647)` is gone from `process/stub.rs` and its `// #647`
    line stays. `cargo test -p holler-cli --test cli_surface_test`, `--test docs_cli_test` and `--test pane_cli_process`
    pass.
28. ADR-0021 carries Decision 1's three edits: `grep -c 'Deferred to #647' docs/adr/ADR-0021.md` prints `0`, and
    `grep -n 'Decided (#647)' docs/adr/ADR-0021.md` prints the section 12 line. No other ADR-0021 line changes.
29. `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry linking
    `[#647](https://github.com/Performant-Labs/holler/issues/647)`; `bash scripts/changelog-check.sh` prints
    `changelog-check: ok`.

**Quality gates:**
30. `cargo clippy --workspace --all-targets -- -D warnings` passes; `cargo test --workspace` passes; `bash scripts/lint.sh`
    passes (every touched file under 900 lines, every new `#[allow]` carries `// #647`).
31. Formatting (epic ruling 4: the tree is not rustfmt-clean and CI has no fmt step, so `cargo fmt --check` on the whole
    workspace fails today on files this story does not touch, C-6): `rustfmt --check --edition 2021` passes on
    `crates/holler-pane/src/reconcile.rs`, `findings.rs`, `crates/holler-cli/src/pane/doctor.rs`,
    `crates/holler-cli/tests/pane_verbs/doctor.rs` (and its submodules) and `tests/pane_verbs/process/stub.rs`; and
    `cargo fmt --all --check 2>/dev/null | grep '^Diff in' | grep -E 'reconcile|findings|pane/doctor|pane_verbs/doctor|process/stub'`
    prints nothing.
32. No new `unsafe`: `grep -rn unsafe crates/holler-pane/src crates/holler-cli/src/pane crates/holler-cli/tests/pane_verbs`
    prints nothing. No new dependency: `git diff origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` is empty;
    `cargo machete` passes.
33. The diff touches only the files listed under Files: `git diff --name-only origin/main` lists nothing else (the
    pipeline's own `docs/handoffs/647*` files excepted).

## Files

Production (F):
- `crates/holler-pane/src/findings.rs` (fill, ~240 lines): `FindingKind`, `FixState`, `FixError`, `Finding`, the remedy
  builder, the one sanitizer for untrusted text (Decision 8).
- `crates/holler-pane/src/reconcile.rs` (fill, ~450 lines): `ReconcileRequest`, `Report` and its summaries,
  `reconcile()`, the per-pane observation, the rules, the fix, the record write. If it nears 600 lines, the observation
  moves to `crates/holler-pane/src/reconcile/observe.rs` (`mod observe;` inside `reconcile.rs`; `lib.rs` is not touched).
- `crates/holler-cli/src/pane/doctor.rs` (replace, ~170 lines): the clap `Args` (`[PANE]`, `--fix`, `ProfileOpt`), `run`,
  the text renderer, the clock read.
- `docs/adr/ADR-0003.md` (one row), `docs/adr/ADR-0021.md` (Decision 1's three edits, ~12 lines), `CHANGELOG.md`.

Tests (T):
- `crates/holler-cli/tests/pane_verbs/doctor.rs` (replace, ~650 lines): AC 1-26. Fallback if it nears 800 lines:
  `crates/holler-cli/tests/pane_verbs/doctor/rig.rs` (`mod rig;` inside `doctor.rs`; `main.rs` already has `mod doctor;`).
- `crates/holler-cli/tests/fixtures/cli-surface.txt` (the `# #647` group), `crates/holler-cli/tests/pane_verbs/process/stub.rs`
  (delete one line).

**Blast radius:** the issue's (`reconcile.rs`, `findings.rs`, `doctor.rs` with its ADR 0003 row, `cli-surface.txt` line
and `tests/pane_verbs/doctor.rs`), plus `CHANGELOG.md`, plus two files the issue does not list, each required by the repo
itself (C-3): `docs/adr/ADR-0021.md` (it defers this decision to this story by name) and `tests/pane_verbs/process/stub.rs`
(its doc tells each verb story to delete its own entry, and `stub_verb_not_implemented` fails otherwise). No manifest,
no `lib.rs`, no test-kit file, no other verb's file. Six files for F (two of them a few lines of prose), three for T.

**Size estimate:** ~860 production lines, ~650 test lines, ~20 doc lines; one component family (the reconcile engine and
its verb). Fits one run.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `Ports` and the six port traits (`ports.rs`, `profile.rs`) | the engine's only access to the world; signatures frozen | reuse |
| `ProfileScope::resolve` | the `--profile` scope and the membership refusals | reuse (no membership check of our own) |
| `PaneStore::list/get/cas_put` | the records in scope, the all-records set for strays, the observation write | reuse |
| `PaneError` closed variants, `class_of` | the verb's errors and exit codes, `fix_error.code` | reuse, no new code, no `Refused` |
| `GridPos` `Display`/`Serialize` | every printed position | reuse, never format row/col by hand |
| `error::excerpt` (`pub(crate)`) | quoting untrusted values inside `holler-pane` messages | reuse |
| `output::emit`, `ErrorBody::from(&PaneError)` | the verb's output, both formats | reuse; the verb supplies only `Report` |
| `args::ProfileOpt` | `--profile` | reuse (flatten) |
| `PaneName::parse`, `ProfileName::parse` | the positional and `--profile` value | reuse |
| `verb_harness::run_verb_with`, `envelope::check_envelope`, `fixture::sample_pane`, the six fakes | tests | reuse; the rig composes them, copies none |
| `profile_diff.rs` (#662) | not used: `profile-drift` is #665's, added after this merges | n/a |
| `run_probe` / `Prober` | not used (Out of scope) | n/a |

## Decisions already made (O; the MO may overrule)

1. **The hub timer: dropped from the hub; no timer in this story (the issue's 2026-10-09 amendment).** Ruling 1 puts every
   adapter call CLI-side, so the hub cannot reconcile. A periodic pass is the CLI verb itself: the operator's scheduler
   runs `holler pane doctor --format=json` (without `--fix`) as often as wanted, and each run writes `last_observed` and
   `harness.health` where they changed. The engine (`holler_pane::reconcile::reconcile`) takes only `Ports` and a
   request, so a later daemon or `--every` loop can call it unchanged. A built-in loop is not built now because, with
   "once per run, no memory" (the issue), every tick would re-report every standing finding: useful only with
   suppression state, which the issue defers to a later story. F edits ADR-0021: (a) lines 470-471 become a
   "**Decided (#647): no hub timer.**" paragraph saying the above in three sentences; (b) the "Deferred to named
   stories" bullet at line 524 is removed; (c) after lines 452-453 one sentence: "Until #649 wires DRIVEN, reconcile
   (#647) compares SHOWN with `session_of_record`, which the hub drives under I2, and leaves `last_observed.driven` as
   stored." Any `holler ...` in a code span there must parse (`docs_cli_test`). The issue also says "raise it on #634":
   #634 is closed and merged, so after this PR merges the run agent posts one comment on #634 linking the ADR change
   (an outward action for the MO to confirm, not F's).
2. **"Once per episode" = once per run** (the issue's amendment): no state is stored, and the frozen `Pane` is unchanged.
   Within a run each finding appears once: the dedupe key is `(kind, pane, session, herdr_pane)`, except `stray-session`,
   whose key is `(kind, session)` with `pane` null and `ports` listing every in-scope server it was seen on (the fake and
   the live fleet share one data directory across servers, `harness.rs:104-108`). Findings are sorted by
   `(pane or "", kind in ALL order, session, herdr_pane)`.
3. **The findings (`FindingKind`, 12, codes = serde names, kebab-case; AC 25):**

   | Kind | Observed when | `--fix` | Remedy |
   |---|---|---|---|
   | `herdr-version-unsupported` | `herdr.version()` is `HerdrVersionUnsupported` (message kept; it names the supported ones) | no | none (upgrade Herdr; no holler verb) |
   | `observe-failed` | a read call fails with anything but the outcomes below; the message names the op (`herdr.snapshot`, `host.ps`, `harness.health`, ...) and the code; also a `generation-conflict` on the record write | no | `holler pane doctor <pane>` (or `holler pane doctor` with no pane) |
   | `unregistered-herdr-pane` | whole-fleet runs only: a snapshot pane whose `(session, pane_id)` no record names ("process with no record") | no | none (no holler verb adopts a pane; `pane import` is #650's) |
   | `herdr-pane-missing` | snapshot succeeded and lacks the record's `(session, pane_id)` | no | `holler pane relaunch <pane>` |
   | `tmux-session-missing` | `host.ps(name)` is `pane-not-found` | no | relaunch |
   | `server-wedged` | `health` false and `list_sessions` is `timeout` (frozen: accepts, never answers) | no | relaunch |
   | `server-down` | `health` false and `list_sessions` anything else (refused, dead, or an answer despite the failed health check) | no | relaunch |
   | `no-session-of-record` | `session_of_record` is `None` | no | `holler pane reset <pane>` |
   | `session-of-record-missing` | the server's list was read and lacks `session_of_record` | no | reset |
   | `tui-foreign-session` | the TUI shows X and the pane's own server's list (read) lacks X: a harness the record does not know is in the pane | no | relaunch |
   | `shown-driven-mismatch` | record S, the TUI shows something else (another session or the home screen `None`), and not `tui-foreign-session` | yes, when the server is healthy and lists S | `holler pane doctor <pane> --fix` while fixable or skipped; relaunch when not fixable or failed; none when fixed |
   | `stray-session` | a listed session that is the `session_of_record` of **no** registered pane (all records, not only the scope) | no | none (no holler verb deletes a harness session; never deleted by `--fix`, since that would guess whose it is) |

   "Registered pane with no process" is `server-down`, `tmux-session-missing` and `herdr-pane-missing`; "process with no
   record" is `unregistered-herdr-pane` and `tui-foreign-session` (C-4). A mismatch and a stray about the same session
   are both reported (they are about different things: the TUI and the server's data), which is what keeps AC 8 true:
   after a fix, the abandoned session is still the same stray finding, not a new one. No finding names a session to
   switch to: `reset` creates a fresh one, and choosing an existing one is the operator's (the 2026-10-07 lesson).
4. **The observation, per pane in scope, in order:** `host.ps(name)`; `harness.health(port)`; if `health` is true,
   `list_sessions(port)` (its failure is `observe-failed`); if false, `list_sessions(port)` only to tell wedged from
   down, and whatever it answers is not used by any other rule (an unhealthy server's list is not trusted);
   `shown_session(pane_id)`. Once per run: `herdr.version()` and `herdr.snapshot()`, and
   `pane_store.list()` (the all-records set for strays; in a `--profile` or `[PANE]` run too). A rule that needs a value
   that was not observed is skipped (no finding guessed from a missing observation).
5. **`--fix` repairs one thing: a `shown-driven-mismatch` it can repair from the record.** Conditions: `--fix` given; the
   server answered `health` true; its list holds S (`session_of_record`); and the pane is not `orchestrator`, unless the
   run names it (`doctor <pane> --fix`), since switching the orchestrator's own TUI under it is a deliberate act. The fix
   is plan -> act -> observe -> record (I3): `select_session(pane_id, S)`, then `shown_session(pane_id)`; equal to S is
   `fixed`, anything else `failed` (with `fix_error`). It never calls `create_session`, `serve`, `attach_tui`, `abort`,
   any `HostPort` writer or any `HerdrPort` writer, and never writes `session_of_record`. Parked and drained panes are
   fixed like any other (the fix sends no work).
6. **The record write (I6, and the hub's periodic-writer rule).** After observing (and fixing), the engine builds the
   pane's next `last_observed.shown` (the last observed value; the stored one when `shown_session` failed) and
   `harness.health` (`Healthy` for `health` true, `Unhealthy("server-wedged")` or `Unhealthy("server-down")` per
   Decision 3, unchanged when `health` failed). Only if either differs from the stored record does it call
   `cas_put(next, pane.generation)` with `last_observed.at = request.now_ms`; `last_observed.driven` stays as stored
   (Decision 1c). No other field changes. A `generation-conflict` (a verb wrote meanwhile) is an `observe-failed`
   finding with remedy `holler pane doctor <pane>`, not retried. So `at` is "when the current value was first observed".
   Writing `harness.health` goes beyond the issue's "write `last_observed`"; it is an observation of the same pass, in
   the same write, and a dead server would otherwise stay `healthy` in the record (I6).
7. **Exit codes: 0 whenever the pass completes, with or without findings.** ADR-0021 line 342: findings are kinds, not
   errors; and the envelope cannot carry `data` with a failure (`envelope.rs:25-33`, rules 6 and 9), so a non-zero exit
   would throw the report away. A script reads `data.findings`. The verb fails only when it cannot run the pass:
   `usage` (2) for a bad `PANE` or `--profile` value; `pane-not-found`, `pane-not-in-profile`, `profile-not-found` (3);
   `pane_store.list`/`get` or `scope.resolve` failing with `unavailable`, `timeout`, `store-corrupt` (1). The same code in
   both formats (AC 16).
8. **Security at the output boundary.** (a) **No shell, no format expansion:** remedies are built only from constant
   words and a `PaneName`, whose grammar is `[a-z0-9-]`, alphanumeric first and last (`vocab.rs:202-223`), so a remedy
   can never carry a shell metacharacter, a space or a leading `-`; no remedy interpolates a session id, a Herdr id, a
   profile name, a port or any adapter text; no remedy carries `--profile` (on `relaunch` it would turn a repair into a
   spec edit). (b) **Untrusted text** (session ids from the server, Herdr pane ids and version, host names and
   `herdr_api_version` from the record, adapter error messages) appears in `message` only through one sanitizer in
   `findings.rs`: `{:?}`-quoted via `error::excerpt` for single values; for an embedded adapter message, every control
   character escaped and cut to 200 characters, one line. The text renderer prints untrusted values `{:?}`-quoted too
   (AC 23). JSON fields carry raw values (serde escapes them). (c) **No secret:** the report holds no `command`, `probe`,
   `env`, `cwd` or `model` field (AC 22); `PaneError` messages are secret-free by contract (`error.rs:392-393`).
9. **Bounded time (I5): observe panes concurrently.** One `std::thread::scope` thread per pane in scope, plus one for the
   two Herdr calls; `Ports` is `Copy` and every port is `Send + Sync`. The run then takes about the slowest pane's chain
   (at most seven bounded calls with `--fix`: `ps`, `health`, `list_sessions`, `shown_session`, `select_session`,
   `shown_session`, `cas_put`), not the pane count times it. A thread that panics (it cannot by lint, but `join` returns a
   `Result`) becomes an `observe-failed` finding. No new dependency. The operation id for long work stays #644's
   (ADR-0021 section 12).
10. **The public API (fixed here, so T can write RED tests against it; additive later, `profile-drift` by #665).**
    ```rust
    // crates/holler-pane/src/findings.rs
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum FindingKind { HerdrVersionUnsupported, ObserveFailed, UnregisteredHerdrPane, HerdrPaneMissing,
        TmuxSessionMissing, ServerWedged, ServerDown, NoSessionOfRecord, SessionOfRecordMissing, TuiForeignSession,
        ShownDrivenMismatch, StraySession }
    impl FindingKind {
        pub const ALL: &'static [FindingKind];      // declaration order
        pub const fn code(self) -> &'static str;    // exhaustive match, no wildcard
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum FixState { NotFixable, Fixable, Skipped, Fixed, Failed }
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct FixError { pub code: String, pub message: String }
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Finding {
        pub kind: FindingKind, pub pane: Option<PaneName>, pub grid: Option<GridPos>, pub herdr_pane: Option<PaneId>,
        pub session: Option<String>, pub ports: Vec<u16>, pub message: String, pub remedy: Option<String>,
        pub fix: FixState, pub fix_error: Option<FixError>,
    }
    // crates/holler-pane/src/reconcile.rs
    #[derive(Debug, Clone, Copy)]
    pub struct ReconcileRequest<'a> { pub profile: Option<&'a ProfileName>, pub pane: Option<&'a PaneName>,
        pub fix: bool, pub now_ms: i64 }
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Report { pub scope: ScopeSummary, pub fix_requested: bool, pub herdr: HerdrSummary,
        pub hosts: Vec<HostSummary>, pub panes: Vec<PaneSummary>, pub findings: Vec<Finding> }
    pub struct ScopeSummary { pub profile: Option<ProfileName>, pub pane: Option<PaneName> }
    pub struct HerdrSummary { pub version: Option<String> }
    pub struct HostSummary { pub name: String, pub herdr_api_version: Option<String> }  // distinct, sorted
    pub struct PaneSummary { pub name: PaneName, pub grid: GridPos, pub session_of_record: Option<String>,
        pub shown: Option<String>, pub health: ObservedHealth }                       // sorted by name
    #[serde(rename_all = "kebab-case")] pub enum ObservedHealth { Healthy, Wedged, Down, Unknown }
    pub fn reconcile(ports: Ports<'_>, request: &ReconcileRequest<'_>) -> Result<Report, PaneError>;
    ```
    (Each summary struct derives `Debug, Clone, PartialEq, Eq, Serialize`.) No `skip_serializing_if` anywhere: absent
    values serialize as `null`, so the JSON shape is fixed (AC 19); it becomes part of ADR 0003's versioned `--json`
    surface. `now_ms` is the clock seam: the CLI passes `SystemTime::now()` in milliseconds, tests a constant.
11. **The CLI.** `PaneDoctor { pane: Option<String> /* value_name PANE */, #[arg(long)] fix: bool, #[command(flatten)]
    profile: ProfileOpt }`. `run` parses `pane` with `PaneName::parse` and `--profile` with `ProfileName::parse` (usage,
    exit 2), calls `reconcile(ctx.ports, ..)`, and hands `Ok(report)` or `Err(ErrorBody::from(&e))` to `output::emit`
    with the text renderer. Text, one line each, untrusted values `{:?}`-quoted:
    `herdr: version "<v>"` (or `herdr: version unknown`); `host "<name>": herdr_api_version "<v>"` (or
    `(not recorded)`); per finding `<subject> <kind>: <message>[ [<fix>]][ (run: <remedy>)]` with subject `<pane> <pos>`
    for a pane, `- <pos>` for an unregistered Herdr pane, `-` otherwise, and `<fix>` one of `fixable`, `skipped`,
    `fixed`, `fix failed: <code>` (nothing for `not-fixable`); last line `panes checked: N; findings: M; fixed: F`. The
    `# #647` fixture group becomes: `pane doctor |`, `pane doctor | --profile demo --format=json`,
    `pane doctor | demo-c1r1`, `pane doctor | demo-c1r1 --fix`, `pane doctor | --fix --profile demo`.
12. **Where the tests live.** All port-driven tests are in `tests/pane_verbs/doctor.rs` (the CLI test target already
    links the test kit), calling both `reconcile()` directly and the verb through `run_verb_with`. The rig builds
    `FakePaneStore`, `FakeProfileStore`, `FakeProfileScope` (over `Arc`s of the two stores), `FakeHerdr::new("scratch")
    .with_workspace("scratch", 3, 3)`, `FakeHost`, `FakeHarness` and `FakeProber`; for each pane it calls `ensure_pane`,
    `ensure_session`, `serve(name, 48100 + i)`, `create_session`, `attach_tui`, and seeds a `sample_pane` with the
    returned `HerdrPane`, port and `session_of_record`. Unit tests inside `holler-pane` would need a test-kit
    dev-dependency (a manifest edit and a dev-dependency cycle), so there are none; pure helpers are tested through the
    public API.
13. **Hosts and Herdr version.** `data.hosts` comes from the records in scope; `data.herdr.version` from the live
    `version()` call. Doctor does not compare them and writes neither (`host.herdr_api_version` is #640's to record).

Forward-compat (consumers):

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | "a crash between steps (the next `doctor` finds and reports it)"; the reconcile step it prints is `holler pane doctor <pane>` | yes (Decisions 3, 4); its `relaunch PANE` positional must match the remedy (R-4) |
| #645 switch/reset | remedy `holler pane reset <pane>`; doctor never picks a session for `switch` | yes; matches #645's `reset PANE` |
| #646 close | a closed pane's record is deleted, so no `herdr-pane-missing` for it | yes (only records are checked) |
| #648 roster | `last_observed.shown` and `harness.health` written by reconcile; fresh only as often as doctor runs; may call `reconcile()` itself | yes (Decisions 1, 6, 10) |
| #649 integration | "kill a server and watch `doctor` find it" (`server-down`), "delete a session under a TUI and watch it be reported" (`session-of-record-missing` + mismatch); DRIVEN wiring extends Decision 1c | yes, given #642's mapping (R-2) |
| #650 import | records with `session_of_record: None` are reported `no-session-of-record` | yes |
| #663 real `ProfileScope` | doctor calls only `resolve` | yes |
| #665 `profile-drift` | add one `FindingKind` variant (code `profile-drift`) and one call in `reconcile.rs` | yes (`ALL` is a slice; codes are not `RefusalCode`s) |

## Contradictions found

- **C-1 Rigor.** The issue's Pipeline line says `rigor: in-session`; this run is `second-opinion` on the orchestrator's
  instruction. The brief follows the orchestrator.
- **C-2 No test-kit assumptions.** The task expected `ASSUMPTION(#647)` markers in the test kit; there are none (grep
  above). The fakes' documented behaviour is used as the contract instead.
- **C-3 Blast radius vs. the repo's own rules.** The issue lists neither `docs/adr/ADR-0021.md` nor
  `tests/pane_verbs/process/stub.rs`. ADR-0021 defers the timer decision to this story by name (lines 470-471, 524), and
  the epic's precedent (the #642 plan review's B-1) is that a story settling a deferred ADR item edits the ADR in the same
  change; `stub.rs`'s doc tells each verb story to delete its entry, and its test fails otherwise. Both are included.
- **C-4 What the ports can observe.** "Registered pane with no process" and "process with no record" are not directly
  observable: `HostPort` cannot list sessions, and `FakeHost::ps` never contains a harness pid (`harness.rs:12-15`). They
  are mapped onto what the ports do expose (Decision 3). "DRIVEN" is not observable through any port before #649
  (ADR-0021 lines 452-453), so the mismatch compares SHOWN with `session_of_record` (I2) and `last_observed.driven` is left
  as stored (Decision 1c).
- **C-5 "The exact command to run" for everything else.** No holler verb removes a harness session, adopts an
  unregistered Herdr pane or upgrades Herdr, so `stray-session`, `unregistered-herdr-pane` and
  `herdr-version-unsupported` carry `remedy: null` and their message says why. Printing a raw OpenCode or Herdr command
  instead would put those tools in front of the operator again (the epic's goal is that only Holler reaches them).
- **C-6 `cargo fmt --check`.** Asked for by the orchestrator, but the tree is not rustfmt-clean (epic ruling 4; today
  `cargo fmt --all --check` reports diffs in, e.g., `crates/holler-body/src/acp_driver/answerable.rs`). AC 31 checks the
  touched files only.
- **C-7 "Raise it on #634".** #634 is closed; the decision is recorded by editing ADR-0021 and, after merge, one comment
  on #634 (Decision 1).

## Out of scope

- A periodic loop, a daemon or any hub-side reconcile (Decision 1). Suppressing findings across runs.
- `profile-drift` (#665), the health probe (`run_probe`, #663), comparing the record's grid with Herdr's, deleting stray
  sessions, adopting unregistered panes, restarting servers (`relaunch`, #644), creating sessions (`reset`, #645).
- The real adapters and the wiring (`wiring.rs`, #649); any edit to `holler-pane`'s frozen files, the test kit, a
  manifest, `lib.rs`, `cli.rs`, `output.rs` or another verb's file.
- Touching any live fleet, OpenCode server, Herdr server or tmux session: every test runs on the fakes.

## Test plan

**RED first** (tester overlay: a compile error is not RED). T first lands the type declarations of Decision 10 exactly
(enums, structs, derives, `FindingKind::ALL` and `code()`), a `reconcile()` whose body is
`Err(PaneError::NotImplemented)`, and the `PaneDoctor` `Args` of Decision 11 with `run` still returning
`emit_error(.., not_implemented(STORY))`. T then writes AC 1-26 and the fixture and `stub.rs` edits. Expected RED: every
behaviour test fails on its assertion (the verb exits 1 with `not-implemented` where 0 or 3 is expected; `reconcile()`
returns `Err` where a `Report` is expected); AC 24 fails with a recorded maximum of 0 or 1; AC 25 and the fixture lines
already pass (they pin vocabulary and parsing). T journals the RED run with the failing assertions.
**GREEN:** F fills `findings.rs`, `reconcile.rs` and `doctor.rs` and the docs; then `cargo test -p holler-cli --test
pane_verbs doctor`, the process, surface and docs targets (AC 27), and the AC 28-33 gates. AC 24 is repeated 20 times
(`for i in $(seq 20); do cargo test -p holler-cli --test pane_verbs observation_runs_concurrently || break; done`) to
surface a flake.

## Risks

- **R-1 Real adapters vs. fakes.** Wedged-vs-down rests on `list_sessions` answering `timeout` for a frozen server and
  `unavailable` for a refused one; the fake does (`harness.rs:110-113`) and #642's brief maps it so, but #642 is not
  merged. If it maps differently, a wedged server reads as `server-down`: still reported, with the same remedy.
- **R-2 Child sessions as strays.** If the OpenCode adapter returned subagent sessions, every working agent's children
  would be strays. #642's brief lists top-level sessions only (AC 11e there); the fake has no children.
- **R-3 Noise from Herdr panes that are not agents.** A plain shell pane in the Herdr workspace is an
  `unregistered-herdr-pane` in every whole-fleet run. That matches the epic's goal (every pane through Holler) but may be
  loud until #650/#666; `--profile` runs never report it.
- **R-4 Remedy commands of unmerged verbs.** `holler pane relaunch <pane>` assumes #644 takes the pane as its positional
  (`launch`/`relaunch` today parse only the spec flags), and `reset <pane>` follows #645's issue text. They cannot be
  parse-tested yet (AC 26 tests the `doctor` remedies only). The run should note this on #644 and #645; if #644 picks
  another form, the remedy constant in `findings.rs` changes with it.
- **R-5 Run time.** Concurrency bounds a run by the slowest pane's chain, up to seven bounded calls, so a fleet whose
  every adapter is wedged can take well over I5's 10 s; the operation id that would cover it is #644's. Threads are one
  per pane, which is fine for a registry of tens of panes.
- **R-6 Concurrent writers.** A verb writing a pane between doctor's read and its `cas_put` makes the write conflict; it
  is reported, not retried, and nothing is lost (the verb's record wins).
- **R-7 Text-output injection.** Session ids, Herdr ids and adapter messages are untrusted; Decision 8 and AC 23 keep a
  control sequence from reaching the operator's terminal, and AC 22 keeps the stored command (which may hold a token in
  an argv element) out of the output.
