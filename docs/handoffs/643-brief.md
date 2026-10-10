# Brief: #643 read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)

Repo: Performant-Labs/holler. Issue: #643 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**Branch:** `issue-643-implementation` (worktree `.claude/worktrees/0643-read-verbs`, from `origin/main` at `3bdd129`;
`origin/main` at `ce12cdb` merged in as `05e7337`, which brings #647's `holler_pane::reconcile::shown_differs`).
**Amendment 1 (2026-10-09, after A-dup's BLOCK D-1).** The MO ruled option (a1): SYNC is `shown_differs`'s answer
(SHOWN against `session_of_record`), DRIVEN is printed as stored and not compared. It changes Decision 3, AC 3, AC 19,
Decision 15 and the CHANGELOG lines, adds rows to the Reuse map, folds A's W-4 and W-9 into Decisions 2, 6, 7 and 11
and Risks, and lists A-dup's D-2 to D-6 as follow-ups. The ruling's facts are in Evidence, "Added for amendment 1".
**What amendment 1 is, and what the excerpts show (added after the outside review's B-1 to B-3).** Amendment 1 is a
change to **this brief only**: its commit, `d39ea9d`, touches `docs/handoffs/643-brief.md` and no other file
(`git show --stat d39ea9d`). It is the instruction for the next T and F round, **not** a diff that is already in the
tree. The production code on the branch is F's round-1 code: `git diff 06320ad HEAD -- crates/holler-cli/src/pane/list.rs
crates/holler-cli/src/pane/get.rs crates/holler-cli/src/pane/watch.rs` prints nothing. So every source excerpt in this
brief is the tree as it stands at `d39ea9d`, and the excerpts of `list.rs`, `get.rs` and the test helper under
"Added for amendment 1" show, on purpose, the round-1 SYNC rule (`SessionSync::of(&pane.last_observed)` at `list.rs:170`
and `get.rs:86`), the round-1 help (`list.rs:45-48`, `get.rs:26-27`) and the round-1 fixture helper that amendment 1
tells T and F to replace. Decision 3, Decision 15 and AC 3, 19 and 23 describe the tree **after** that round; against the
current tree they fail by design (Test plan, "RED for amendment 1"), and that failure is the RED T confirms before F
edits. Every line number cited for code that amendment 1 changes (Files, "Amendment 1's change"; the excerpts under
"The branch's current SYNC rule") is its position in the current, pre-amendment tree; F's edit moves those lines, and no
AC pins them (AC 24 checks file names only).
**Review rigor:** second-opinion, set by the orchestrator for this run. The issue's own `## Pipeline` line says
`rigor: in-session`, and an epic comment of 2026-10-08 put every story at in-session; the orchestrator raised this one
(see "Contradictions found", C1). The outside reviewer sees only this brief, so every fact it relies on is pasted below.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see the table under "Decisions".
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 3, 6, 9 and 10, and ADR 0003 (CLI surface). This brief
edits ADR 0003's three `#643` rows (epic ruling 2 makes them this story's) and does **not** edit ADR-0021: the design fits
it as it stands, §11 included (reconcile compares SHOWN with `session_of_record` until #649 wires DRIVEN).
**Handoffs:** `docs/handoffs/643/handoff-<phase>.md`; the decision journal is `docs/handoffs/643/decisions.md`.
**Public repository:** every name in code, tests, docs, commits and the PR is a neutral placeholder (`demo-*` panes, the
profile `demo`, workspace and session `scratch`, `/srv/demo`, `demo-provider/demo-model`, `localhost`). No personal host,
tailnet or account name appears anywhere.

## Problem

The three read verbs are stubs that refuse with `not implemented (story #643)`. Nothing in Holler can show the pane
registry: which pane sits where, which profile it belongs to, whether its harness server is healthy, and, above all,
whether the session its TUI shows (SHOWN) is the session the hub drives (DRIVEN). The epic's whole point is that a
SHOWN/DRIVEN mismatch is loud and cannot persist; these verbs are where it becomes visible. This story fills
`list` (a table), `get` (one pane in full: record, profile spec, context ceilings, command, probe) and `watch` (the
change feed as a stream, NDJSON in JSON mode), each scoped by `--profile`, each printing grid positions row first, and
each answering in the shared envelope under `--format=json`. They read the record only: SHOWN, DRIVEN and health come
from the record's `last_observed` and `harness`, and these verbs call no Herdr, tmux, harness or probe port. SYNC, the
mismatch flag, is the one rule on `main` (`holler_pane::reconcile::shown_differs`, which `pane doctor` also uses):
SHOWN against the session of record, which ADR-0021's I2 makes the session the hub drives.

## Evidence (verbatim, as of `3bdd129`; re-verified at `05e7337`)

**Re-verification at `05e7337`** (the merge of `origin/main` at `ce12cdb`). Every span quoted below is unchanged at the
same lines, except:
- the stubs (`pane/{list,get,watch}.rs`), the stub tests (`tests/pane_verbs/{list,get,watch}.rs`), the three `STUBS`
  rows, `ADR-0003.md:44-46` and `cli-surface.txt:104-111` show the state **before** this story; this branch has already
  replaced them (the current rows are AC 20's and Decision 13's). `stub.rs`'s `stub_verb_not_implemented` is now at
  `:70-82` (rows above it were deleted by this story and by #647);
- `docs/adr/ADR-0021.md` gained a line near :153 and two at :455-456 (§11, quoted under "Added for amendment 1"), so
  every span below from :154 on is one line later than at `3bdd129` and is cited at its current line.

The surface rows as the branch holds them now (verbatim, at `d39ea9d`; round 1 already made them AC 20's and
Decision 13's, and amendment 1 does not touch them):
```
docs/adr/ADR-0003.md:44-46
holler pane list [PANE] [--profile NAME]                          #643
holler pane get PANE [--profile NAME]                             #643
holler pane watch [PANE] [--profile NAME] [--since CURSOR] [--until-idle]   #643
crates/holler-cli/tests/fixtures/cli-surface.txt:104-113
# #643
pane list |
pane list | demo-c1r1
pane list | --profile demo
pane list | --format=json
pane get | demo-c1r1
pane get | demo-c1r1 --profile demo --format json
pane watch |
pane watch | demo-c1r1 --since 7 --until-idle
pane watch | --profile demo --json
crates/holler-cli/tests/pane_verbs/process/stub.rs:17-21
pub const STUBS: &[(&str, &str, u32)] = &[
    // #643
    // #644
    ("pane", "launch", 644),
    ("pane", "relaunch", 644),
```
So the `# #643` block is the nine lines of Decision 13 under its header (no `pane get |` line), and `STUBS` keeps the
`// #643` line with no `#643` row under it: AC 20's greps and `stub_verb_not_implemented` (now `stub.rs:70-82`) hold on
the current tree, and AC 22's `cargo test --workspace` does not depend on a row still to be removed.

### The stubs this story replaces (all three files, whole)

```
crates/holler-cli/src/pane/list.rs:1-23
//! `holler pane list`: a stub (story #670). Story #643 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 643;

/// List panes.
#[derive(Args, Debug)]
pub struct PaneList {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane list`: refuse, naming the story that owns it.
pub fn run(_args: &PaneList, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
`crates/holler-cli/src/pane/get.rs:1-23` and `crates/holler-cli/src/pane/watch.rs:1-23` are the same file with
`list` replaced by `get` / `watch`, the struct named `PaneGet` / `PaneWatch`, and the doc line `/// Show one pane.` /
`/// Stream pane changes.` (line 13). Their line 21 is `pub fn run(_args: &PaneGet, ctx: &mut VerbCtx<'_>) -> i32 {` and
`pub fn run(_args: &PaneWatch, ctx: &mut VerbCtx<'_>) -> i32 {`.

### The frozen dispatch and the shared flag (not edited)

```
crates/holler-cli/src/pane/mod.rs:3-6
//! **Frozen by story #670.** This file lists the verbs and dispatches them; it never changes
//! again. One verb is one file in this directory with its own clap `Args` struct and its own
//! `run`, owned by one story, so adding a verb's real behaviour edits that file and no other.
//! Until then each verb is a stub that refuses with `not implemented (story #NNN)`.
crates/holler-cli/src/pane/mod.rs:14-28
pub mod args;
pub mod close;
pub mod doctor;
pub mod get;
pub mod import;
pub mod launch;
pub mod list;
pub mod park;
pub mod profile_scope;
pub mod relaunch;
pub mod reset;
pub mod switch;
pub mod unpark;
pub mod watch;
pub mod wiring;
crates/holler-cli/src/pane/mod.rs:51-56
/// Run a `holler pane` verb and return its exit code (0 ok, 1 failure, 2 usage, 3 refusal).
pub fn run(cmd: &PaneCmd, ctx: &mut VerbCtx<'_>) -> i32 {
    match cmd {
        PaneCmd::List(args) => list::run(args, ctx),
        PaneCmd::Get(args) => get::run(args, ctx),
        PaneCmd::Watch(args) => watch::run(args, ctx),
```
Consequence: no new module file can be added under `pane/` (that needs a `pub mod` line in this frozen file), so code
the three verbs share lives in one of the three files (Decision 2).
```
crates/holler-cli/src/pane/args.rs:19-25
/// `--profile NAME`: scope the verb to a profile.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileOpt {
    /// Act on the panes of this profile; a named pane must belong to it.
    #[arg(long, value_name = "NAME")]
    pub profile: Option<String>,
}
```

### The output module the verbs call (frozen signatures; not edited)

```
crates/holler-cli/src/output.rs:191-199
/// What a `holler pane` or `holler profile` verb runs with.
///
/// `ports` is held by value: [`Ports`] is `Copy`. A verb takes `&mut VerbCtx` because writing to
/// the sink needs a mutable borrow.
pub struct VerbCtx<'a> {
    pub format: Format,
    pub ports: Ports<'a>,
    pub sink: Sink<'a>,
}
crates/holler-cli/src/output.rs:201-243
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
    match format {
        Format::Text => emit_text(sink, result, text),
        Format::Json => emit_json(sink, result),
    }
}

/// Print a stream of results, one line each, and return the exit code.
///
/// Text mode writes `text(item)` per item; JSON mode writes one envelope per line (NDJSON, which is
/// what `pane watch` prints). Every line is flushed as it is written. The stream ends at the first
/// error item, which is reported the way [`emit`] reports an error, and at the first write that
/// fails; lines already written stay written.
pub fn emit_stream<T: Serialize>(
    sink: &mut Sink<'_>,
    format: Format,
    items: impl Iterator<Item = Result<T, ErrorBody>>,
    text: impl Fn(&T) -> String,
) -> i32 {
    for item in items {
        let code = emit(sink, format, item, &text);
        if code != 0 {
            return code;
        }
    }
    0
}

/// Print an error (a failed `emit` with no data) and return its exit code.
pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
    emit(sink, format, Err::<(), _>(error), |()| String::new())
}
crates/holler-cli/src/output.rs:134-141
impl From<&PaneError> for ErrorBody {
    fn from(error: &PaneError) -> Self {
        Self {
            code: ErrorCode::from(error),
            message: error.to_string(),
        }
    }
}
crates/holler-cli/src/output.rs:4-8
//! - **Text mode** (the default): the data of a successful verb goes to `out`; an error goes to
//!   `err` as `error: <message>`, with nothing on `out`.
//! - **JSON mode** (`--format=json`, or `--json`): exactly one envelope goes to `out` per result,
//!   an error included, and nothing goes to `err`. `pane watch` is the one stream: one envelope
//!   per line ([`emit_stream`]).
```

### The ports the verbs read (frozen by #637; not edited)

```
crates/holler-pane/src/ports.rs:28-34
/// A position in a store's change sequence: a store-wide, strictly increasing
/// sequence number, one per change. `Cursor(0)` is "from the beginning".
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cursor(pub u64);
crates/holler-pane/src/ports.rs:36-52
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
crates/holler-pane/src/ports.rs:62-67
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;
crates/holler-pane/src/ports.rs:80-81
    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError>;
crates/holler-pane/src/ports.rs:224-235
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
crates/holler-pane/src/profile.rs:329-330
    /// The profile named `name`, or `None`.
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;
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

### The records the verbs print (frozen by #637; not edited)

```
crates/holler-pane/src/pane.rs:91-100
/// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
}
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
crates/holler-pane/src/pane.rs:165-190
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Hold {
    None,
    Parked {
        reason: String,
        /// When the park ends, in the verb's own words (a time or a condition).
        release_when: String,
        /// When the pane was parked (milliseconds since the Unix epoch).
        since: i64,
    },
    Drained,
}

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
crates/holler-pane/src/pane.rs:192-254
/// The model a pane runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    pub provider: String,
    pub model_id: String,
    pub effort: String,
}

/// The context ceilings a watchdog reads (a local model's window is smaller than a
/// hosted one's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCeilings {
    pub soft: u32,
    pub hard: u32,
}

/// The health probe of a pane (B1): the stored `check` argv, the strings its output
/// must contain, and the last result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneProbe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Argv>,
    #[serde(default)]
    pub expect: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<ProbeResult>,
}

/// One record per pane, owned by the hub. The key is [`Pane::name`]. Every write is
/// a compare-and-swap on [`Pane::generation`] (see [`crate::generation`]).
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
crates/holler-pane/src/pane.rs:256-268
/// One change to the pane store, as `PaneStore::watch` yields it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneEvent {
    /// The store-wide sequence number of this change; hand it back as the `since`
    /// of the next watch to resume without a gap or a repeat.
    pub cursor: Cursor,
    /// The pane the change concerns.
    pub name: PaneName,
    /// The record after the change; `None` when the record was deleted.
    #[serde(default)]
    pub pane: Option<Box<Pane>>,
}
crates/holler-pane/src/pane.rs:105-115
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
crates/holler-pane/src/probe.rs:15-26
/// What one run of a health probe found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeResult {
    /// The probe ran and every expected string was in its output.
    Ok,
    /// The probe ran and these expected strings were not in its output.
    Failed { missing: Vec<String> },
    /// The probe could not be run to a verdict (the program is missing, it timed
    /// out, ...); the reason is plain text.
    Error(String),
}
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
crates/holler-pane/src/profile.rs:40-61   (ProfileName::parse: every refusal is `usage`)
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        let name = text.trim();
        let refusal = if name.is_empty() {
            Some("a profile name must not be empty")
        } else if name.chars().count() > MAX_NAME_CHARS {
            Some("a profile name is at most 64 characters")
        } else if name.chars().any(char::is_control) {
            Some("a profile name must not contain control characters")
        } else if slugify(name).is_empty() {
            Some("a profile name needs at least one ASCII letter or digit")
        } else {
            None
        };
        match refusal {
            Some(rule) => Err(PaneError::Usage {
                message: format!("invalid profile name {}: {rule}", excerpt(text)),
            }),
            None => Ok(Self(name.to_owned())),
        }
    }
crates/holler-pane/src/pane.rs:34-42   (PaneName::parse: an invalid name is `usage`)
    /// Parse a pane name; the grammar is `SessionName::parse`'s.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        SessionName::parse(text)
            .map(Self)
            .map_err(|e| PaneError::Usage {
                message: format!("invalid pane name {}: {e}", excerpt(text)),
            })
    }
crates/holler-proto/src/vocab.rs:209-223   (the grammar SessionName::parse applies, 1-32 bytes, no `/`)
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
`ProfileSpec` (`crates/holler-pane/src/profile.rs:178-200`) derives `Serialize` and holds `pane: String`, `herdr {workspace,
grid}`, `host {cwd}`, `harness {kind, port_policy}`, `model`, `role`, `env`, `context: ContextCeilings`, `command?`,
`check?`, `expect`. `EnvVarName` (`crates/holler-pane/src/argv.rs:88-90`) is `#[derive(.., Serialize)] #[serde(transparent)]
pub struct EnvVarName(String);` and cannot hold a `=` (it is refused on parse and on decode).

### The error codes and their exit classes (closed set; not edited)

```
crates/holler-pane/src/error.rs:432-443
    /// `profile-not-found`: no profile of that name; `what` is the name.
    /// (#644/#663.)
    ProfileNotFound { what: String },
    ...
    /// `pane-not-in-profile`: a named pane does not belong to the profile the verb
    /// is scoped to; `what` names both. (#643/#663.)
    PaneNotInProfile { what: String },
crates/holler-pane/src/error.rs:457-458
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
crates/holler-pane/src/error.rs:639, 656, 662, 674
            PaneError::NotImplemented => f.write_str("not implemented"),
            PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
                write!(f, "pane is not in the profile: {what}")
            PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
crates/holler-pane/src/error.rs:266, 275, 289-291, 297-301   (class_of; exit 2 usage, 3 refusal, 1 failure)
pub fn class_of(code: &str) -> ErrorClass {
        PaneCode::Usage => ErrorClass::Usage,
        | PaneCode::ProfileNotFound
        | PaneCode::PaneNotFound
        | PaneCode::SessionNotFound => ErrorClass::Refusal,
        | PaneCode::Timeout
        | PaneCode::Unavailable
        | PaneCode::StoreCorrupt
        | PaneCode::NotImplemented
        | PaneCode::ProfileDrift => ErrorClass::Failure,
```
(`PaneCode::PaneNotInProfile` is in the same Refusal arm, `crates/holler-pane/src/error.rs:285`.)

### What the production binary wires today (not edited; #649 owns it)

```
crates/holler-cli/src/pane/wiring.rs:49-60
/// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unwired;

impl PaneStore for Unwired {
    fn get(&self, _name: &PaneName) -> Result<Option<Pane>, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        Err(PaneError::NotImplemented)
    }
```
`Wiring::connect` (`wiring.rs:31-33`) returns `Unwired` for every port, so in the real binary the filled verbs answer
`error: not implemented` (exit 1) until #649 wires the hub client.

### The test kit the tests use (merged by #638; not edited)

```
crates/holler-pane-testkit/src/pane_store.rs:46-60
/// An in-memory `PaneStore`.
///
/// It keeps the port's rules (`holler_pane::ports`; ADR-0021 sections 2, 6 and 8):
///
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
/// - `delete` of a missing record is `pane-not-found`, whatever the generation.
/// - `list` is sorted by name.
/// - A pane that belongs to a profile cannot move to another profile in one write
///   (`pane-in-other-profile`); it leaves its profile (`None`) first. Profiles are
///   compared by slug.
/// - `watch` follows the feed's rules: a cursor ahead of the head is `usage`, and a
///   watch from `Cursor(0)` yields the current state and then resumes from the head as
///   of that snapshot, as the hub's registry does.
crates/holler-pane-testkit/src/pane_store.rs:69-73
/// Every port method first passes [`FakePaneStore::faults`], and when that fails, it
/// returns the error and changes nothing. A watch's `next()` passes it too, as
/// [`PaneStoreOp::WatchNext`]. The fake keeps its whole history, so a watch never
/// collapses writes, and an idle `next()` waits [`FakePaneStore::set_idle_wait`] (zero
/// by default) before it yields `Ok(None)`.
crates/holler-pane-testkit/src/pane_store.rs:93, 103, 109, 116, 123
    pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
    pub fn set_idle_wait(&self, wait: Duration) {
    pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
    pub fn concurrent_delete(&self, name: &PaneName) -> Result<(), PaneError> {
crates/holler-pane-testkit/src/feed.rs:18-31
//! # What a watcher at `since` is owed
//!
//! 1. A cursor ahead of the head is `usage`, refused when the watch opens (the hub's
//!    rule). A cursor that passed once never fails later, because the head only grows.
//! 2. From `Cursor(0)`: one put for each live record, carrying the cursor of that
//!    record's last change, in cursor order. The stream then resumes from the head as
//!    of that snapshot, as the hub's does, so a record deleted before the snapshot
//!    leaves no event at all.
//! 3. From any other cursor: every change after it, in order.
//!
//! Each `next()` first calls the port's fault hook (the store's `watch_next` op), so a
//! fault set while the stream is open reaches it. It then yields the next change owed.
//! When none is owed, it waits up to the idle wait for a write and yields `Ok(None)`
//! (idle) if none came; the stream stays usable. The first error ends the stream.
crates/holler-pane-testkit/src/fault.rs:25-35
/// A standing fault: it applies to every call until it is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
    /// timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
    Fail(PaneError),
}
crates/holler-pane-testkit/src/fault.rs:68, 75, 86
    pub fn set(&self, fault: Option<Fault>) {
    pub fn fail_next(&self, op: Op, error: PaneError) {
    pub fn calls(&self) -> Vec<Op> {
crates/holler-pane-testkit/src/profile_scope.rs:34-38
/// - `resolve(P, None)` is the stored P and every pane whose record names P, in name
///   order. `resolve(P, Some(n))` is P and n, whose record must name P, or else
///   `pane-not-in-profile` (a pane with no record included). A missing P is
///   `profile-not-found`, checked first. Membership is `Pane.profile` alone, compared by
///   slug, so a spec of P adds no pane to its scope.
crates/holler-pane-testkit/src/profile_scope.rs:70
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
crates/holler-pane-testkit/src/profile_store.rs:127-130
    pub fn seeded(
        profiles: impl IntoIterator<Item = Profile>,
        actor: &Actor,
    ) -> Result<Self, PaneError> {
crates/holler-pane-testkit/src/fixture.rs:34-41
/// A valid, deterministic `Pane` named `name`: generation 0, grid `r1c1`, no profile,
/// no session of record, harness port 48100 with its health unknown, an agent with no
/// hold, no command and no probe. Its Herdr session and workspace are scratch names,
/// never a live session's. Its tmux session is the pane's name, as on a real pane, so
/// pick a neutral one such as `demo-c1r1`. Two calls with one name return equal panes.
///
/// `usage` when `name` is not a valid pane name.
pub fn sample_pane(name: &str) -> Result<Pane, PaneError> {
crates/holler-pane-testkit/src/fixture.rs:121
pub fn sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError> {
```
The other fakes a `Ports` needs, and the call logs Decision 9's "observes nothing" test reads:
`FakeHerdr::new(session: &str)` (`herdr.rs:174`) with `faults()` (`herdr.rs:211`), `FakeHost::new()` (`host.rs:97`) with
`faults()` (`host.rs:106`), `FakeHarness::new()` (`harness.rs:170`) with `faults()` (`harness.rs:179`), and
`FakeProber::new()` (`prober.rs:47`) with `calls()` (`prober.rs:64`). Every `faults()` is a `FaultSwitch` whose `calls()`
returns every call made through the port.

The envelope helper (#681), the only envelope check the tests use:
```
crates/holler-pane-testkit/src/envelope.rs:4-7
//! - [`check_envelope`] takes a verb's stdout and exit code. It returns the one envelope
//!   on stdout, or an [`EnvelopeFault`] naming the first rule the output breaks.
//! - [`check_ndjson`] does the same for an NDJSON stream (`pane watch`): one envelope per
//!   line, and only the last line may be a failure.
crates/holler-pane-testkit/src/envelope.rs:35-37
//! [`check_ndjson`] checks the exit code first, then drops one final `\n`. If nothing is
//! left, the stream is empty. Otherwise it checks each line alone with
//! [`check_envelope`], in order, and the first fault wins, so a blank line is `NotJson`.
crates/holler-pane-testkit/src/envelope.rs:202, 241
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
pub fn check_ndjson(stdout: &str, exit_code: i32) -> Result<Vec<Envelope>, EnvelopeFault> {
crates/holler-pane-testkit/src/envelope.rs:243-249
    let lines: Vec<&str> = match stdout.strip_suffix('\n').unwrap_or(stdout) {
        "" => Vec::new(),
        body => body.split('\n').collect(),
    };
    let Some((last, before)) = lines.split_last() else {
        return Err(EnvelopeFault::EmptyStream);
    };
```
(So `check_ndjson` refuses an empty stream; Decision 7 handles a watch that prints nothing.)

There is **no** `ASSUMPTION(#643)` or `ASSUMPTION (#643)` comment anywhere in the test kit (`grep -rn 'ASSUMPTION' crates`
finds only `#640`, `#642`, `#661/#663` and `#663` ones). The fake rules this story depends on are the ones quoted above.

### The in-process harness and the test files this story owns

```
crates/holler-cli/tests/verb_harness/mod.rs:52-54
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
crates/holler-cli/tests/verb_harness/mod.rs:57
    let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{full:?} must parse: {e}"));
crates/holler-cli/tests/verb_harness/mod.rs:38-44
/// What one verb run produced.
#[derive(Debug)]
pub struct Outcome {
    pub code: i32,
    pub out: String,
    pub err: String,
}
crates/holler-cli/tests/pane_verbs/main.rs:7-9
//! One module per verb, so a verb story edits only `pane_verbs/<verb>.rs` and never
//! this file or the manifest. The shared pieces: the harness that runs a verb over
//! captured writers (`tests/verb_harness/mod.rs`), the output API (`output_api`), the
crates/holler-cli/tests/pane_verbs/main.rs:26-35   (the modules `get`, `list`, `watch` are declared; main.rs line 1-5 carry
                                                     `#![allow(clippy::unwrap_used)] // #670` and the expect/panic/
                                                     unreachable/dead_code equivalents)
crates/holler-cli/tests/pane_verbs/list.rs:1-17
//! `holler pane list`: the stub case of story #670. Story #643 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

// The consumer of `holler-pane-testkit` (a dev-dependency declared by #670 so that
// #638 and #643-#647 add no manifest line). #643 replaces this with its real use of the
// fakes; the link itself is the assertion: this target does not build without it.
use holler_pane_testkit as _;

#[test]
fn testkit_links() {}

#[test]
fn pane_list_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "list"], 643);
}
```
`tests/pane_verbs/get.rs:1-9` and `watch.rs:1-9` hold only `assert_stub_routes(&["pane", "get"|"watch"], 643)`.
The manifest comment that names `testkit_links` (`crates/holler-cli/Cargo.toml:432-435`, not edited):
```
# #670: the fakes and conformance suite of holler-pane (empty until #638). Its
# consumer test is `testkit_links` in tests/pane_verbs/list.rs; #638 and
# #643-#647 use it from their own pane_verbs/<verb>.rs files.
holler-pane-testkit = { path = "../holler-pane-testkit" }
```

The process-level stub table, which says the verb story deletes its own rows:
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:9-21
/// Every verb that is still a stub, with the story that owns it. The only place in the
/// shared process tests that names a stub's owning story.
///
/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
/// line**: two stories that delete whole groups, header included, delete adjacent lines,
/// and git reports that as a conflict. (`PANE_VERBS` and `PROFILE_VERBS` keep the verb
/// itself.)
pub const STUBS: &[(&str, &str, u32)] = &[
    // #643
    ("pane", "list", 643),
    ("pane", "get", 643),
    ("pane", "watch", 643),
crates/holler-cli/tests/pane_verbs/process/stub.rs:76-88
/// Text mode: `error: not implemented (story #NNN)` on stderr, nothing on stdout, exit 1.
#[test]
fn stub_verb_not_implemented() {
    let mut failures = Vec::new();
    for &(namespace, verb, story) in STUBS {
        let out = holler(&[namespace, verb]);
        let line = format!("error: not implemented (story #{story})");
```
Once the verbs call `Unwired`, the real binary prints `error: not implemented` (the `NotImplemented` Display above, no story
number), so these three rows must go or `stub_verb_not_implemented` fails.

### The CLI surface rows this story owns

```
docs/adr/ADR-0003.md:44-46
holler pane list [--profile NAME]                                 #643
holler pane get [--profile NAME]                                  #643
holler pane watch [--profile NAME]                                #643
docs/adr/ADR-0003.md:92   (excerpt)
... each owning story adds its verb's positionals and flags to its own row, with its own line in `cli-surface.txt`, and
edits no other verb's. Rows are grouped by owning story, one story per block. ...
crates/holler-cli/tests/fixtures/cli-surface.txt:104-111
# #643
pane list |
pane list | --profile demo
pane list | --format=json
pane get |
pane get | --profile demo --format json
pane watch |
pane watch | --profile demo --json
```
`cli_surface_test` asserts every fixture line parses with `Cli::try_parse_from`; `docs_cli_test` parses every `holler ...`
line of `docs/**` after dropping `[...]` groups and cutting at two spaces (`crates/holler-cli/tests/docs_cli_test.rs:132-158`),
so `holler pane get PANE [--profile NAME]` is checked as `holler pane get PANE`. `docs/handoffs/**` is excluded.

### ADR-0021, the parts this story relies on

```
docs/adr/ADR-0021.md:126-129
- **Scoping** (`list`, `get`, `watch`, `doctor`, `switch`, `reset`, `park`, `unpark`, the `--pane` forms of `say`, `interrupt`
  and `answer`, and the roster): act on P's panes. A named pane outside P is `pane-not-in-profile`. The read verbs and
  `doctor`, `park` and `unpark` with no pane name mean every pane of P; `say`, `interrupt`, `answer`, `switch` and `reset`
  still need a pane name.
docs/adr/ADR-0021.md:332-333, 337-338
**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
| `pane list`, `pane watch` | `profile-not-found`, `pane-not-in-profile` |
| `pane get` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile` |
docs/adr/ADR-0021.md:364
- `pane watch` in JSON mode writes NDJSON: one envelope per line, flushed per line.
docs/adr/ADR-0021.md:409-411
- `schema_version` is an integer and is 1 now. Adding a field to the envelope or to a verb's `data` does not change it.
  Removing, renaming or retyping one does, and is a breaking change under ADR 0003's versioning (the JSON shape).
- Grid positions in `data` serialize through `GridPos`.
docs/adr/ADR-0021.md:428
- **Output** is always row first: text `r2c1`; JSON `{"row": 2, "col": 1, "pos": "r2c1"}`, in that key order. On input `pos`
docs/adr/ADR-0021.md:219-220
**Decided: `*/watch` is long-poll.** One request, one reply, whose `data` is `WatchReply { events, cursor }`: the batch
collected after `since`, and the cursor to resume from. An idle window answers `{"events": [], "cursor": <head>}`; ...
docs/adr/ADR-0021.md:165
| I5 | A verb returns in bounded time (default 10 s); long work returns an operation id. | ... |
```

### A helper to reuse, and how clap renders help here

```
crates/holler-cli/src/time_fmt.rs:7-11
/// Format a unix epoch second as a UTC `YYYY-MM-DD HH:MM:SS` string
/// (the `hub token list`/`mint`/`ping` EXPIRES column). No chrono
/// dependency: a manual civil calendar conversion (Howard Hinnant's
/// algorithm) is enough for epoch seconds.
pub fn format_epoch(secs: u64) -> String {
```
Observed by O on the binary built from `3bdd129` (`holler pane --help`, `holler pane list --help`, read-only): the struct's
doc comment line `/// List panes.` is the subcommand's about (`list      List panes`), and long help is printed unwrapped
(a 300-character `--debug` paragraph prints on one line), because the workspace's clap has only the `derive` feature
(`Cargo.toml`: `clap = { version = "4", features = ["derive"] }`). So a multi-paragraph doc comment on `PaneList` is what
`--help` prints, and a test can match substrings of it.

Workspace rules that bind the code (`Cargo.toml` `[workspace.lints]`, `clippy.toml`): clippy denies `unwrap_used`,
`expect_used`, `panic`, `unreachable`, `cognitive_complexity` (threshold 15), `too_many_lines` (100 per function) and
`struct_excessive_bools`; rustc denies `dead_code`. `scripts/lint.sh` fails a file at 900 lines and any `#[allow]` without a
trailing `// #NNN`. `holler-cli` already depends on `clap`, `serde`, `serde_json` and `holler-pane`, and has
`holler-pane-testkit` as a dev-dependency, so this story adds no dependency.

### Added for amendment 1 (verbatim, as of `05e7337`)

**The one SHOWN/DRIVEN rule on `main`** (#647, PR #701), public and reachable from `holler-cli`:
```
crates/holler-pane/src/reconcile.rs:171-181
/// Whether a pane whose session of record is `session_of_record` shows something else: the
/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
/// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
/// no mismatch (it is `no-session-of-record`).
///
/// The one form of this comparison: a pass calls it with what the TUI shows now, and a
/// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
/// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
    session_of_record.is_some_and(|record| shown != Some(record))
}
crates/holler-pane/src/lib.rs:52
pub mod reconcile;
crates/holler-cli/src/pane/doctor.rs:20   (holler-cli already imports this module)
use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
crates/holler-pane/src/reconcile/observe.rs:242-247   (doctor's own call)
    let Some(record) = pane.session_of_record.as_deref() else {
        return screen;
    };
    if !shown_differs(Some(record), shown) {
        return screen;
    }
```
**What reconcile writes, and what it leaves alone:**
```
crates/holler-pane/src/reconcile.rs:31-35
//! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
//!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
//! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
//!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
//!   the session the hub drives.
crates/holler-pane/src/reconcile/observe.rs:78-84
/// What the chain knows of the TUI's screen.
enum Screen {
    /// The session it shows; `None` is its home screen, or no TUI in the pane.
    Seen(Option<String>),
    /// `shown_session` failed: every rule that needs the screen is skipped.
    Unseen,
}
crates/holler-pane/src/reconcile/observe.rs:361-375   (in `record`)
    let health = server.health().recorded();
    if health.is_none() && matches!(screen, Screen::Unseen) {
        return None;
    }
    let mut next = pane.clone();
    if let Some(health) = health {
        next.harness.health = health;
    }
    if let Screen::Seen(shown) = screen {
        next.last_observed.shown.clone_from(shown);
    }
    if next == *pane && pane.last_observed.at != 0 {
        return None;
    }
    next.last_observed.at = request.now_ms;
docs/adr/ADR-0021.md:162   (I2)
| I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | ...
docs/adr/ADR-0021.md:453-456   (§11)
**Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
registered pane, and when it moves after a switch or reset.
Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub
drives, and leaves `last_observed.driven` as stored.
docs/handoffs/647/handoff-A-dup.md:111-115   (Notes for O)
2. **D-2 and D-3.** Tell #643, #644 and #663, and #640 for the adapter's `excerpt` copy, which helpers this branch adds:
   - `holler_pane::findings::quoted`;
   - `holler_pane::findings::doctor_command`;
   - `holler_pane::reconcile::shown_differs`.
   Each later story's A-dup gate should then reject its own copy.
```
So nothing on `main` writes `last_observed.driven` (reconcile's `record` writes only `harness.health`,
`last_observed.shown` and `at`), and a `shown: None` that reconcile wrote with `at > 0` is a home screen, not "could not
tell" (an unseen screen leaves `shown` as stored).

The spans above (`reconcile.rs:171-181`, `observe.rs:361-375`) were re-read in the working tree at `d39ea9d` for this
brief and are unchanged there. The grep behind "nothing writes `driven`", over every crate's `src/`, at `d39ea9d`:
```
$ grep -rnE 'last_observed\.driven|\.driven\s*=|driven:\s' crates/*/src
crates/holler-cli/src/pane/list.rs:154:    pub driven: Option<String>,
crates/holler-cli/src/pane/list.rs:169:            driven: pane.last_observed.driven.clone(),
crates/holler-load-test/src/hold_report.rs:95:    pub sessions_driven: usize,
crates/holler-pane-testkit/src/fixture.rs:68:            driven: None,
crates/holler-pane/src/pane.rs:187:    pub driven: Option<String>,
crates/holler-load-test/src/hold_load.rs:110:fn phase(cfg: &Config, hub: &Hub, label: &str, driven: &[String], held: &HashSet<String>, secs: u64) -> Phase {
crates/holler-load-test/src/hold_load.rs:338:    let driven: Vec<String> = all.iter().take(cfg.hold_sessions).cloned().collect();
crates/holler-load-test/src/hold_load.rs:341:        sessions_driven: driven.len(),
crates/holler-pane/src/reconcile.rs:33://! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
```
The hits are the two field declarations (`pane.rs:187`, `list.rs:154`), this story's own read of it (`list.rs:169`),
the test kit's sample pane, which sets it to `None` (`fixture.rs:68`), reconcile's doc line, and unrelated `holler-load-test`
names. No production code assigns it.

**The branch's current SYNC rule, which amendment 1 replaces** (F's round-1 code, unchanged since `06320ad`; this is the
code **before** amendment 1's T and F round, the code that round changes, not the code after it. The new body of
`SessionSync::of(pane: &Pane)` does not exist yet: Decision 3 specifies it, and F writes it):
```
crates/holler-cli/src/pane/list.rs:195-221
/// Whether the session a pane's TUI shows is the session the hub drives, as reconcile last
/// recorded both (`last_observed`). This is the one copy of the rule the read verbs print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSync {
    /// Both were observed, and they are the same session.
    Ok,
    /// Both were observed, and they differ: the pane shows one session while the hub
    /// drives another.
    Mismatch,
    /// One of them was not observed, so there is nothing to compare. This is not flagged:
    /// reconcile records `None` when it cannot tell, and flagging every such pane would
    /// make the flag noise.
    Unobserved,
}

impl SessionSync {
    /// The rule: [`Ok`](Self::Ok) when `shown` and `driven` are both set and equal,
    /// [`Mismatch`](Self::Mismatch) when both are set and differ, and
    /// [`Unobserved`](Self::Unobserved) when either is unset.
    pub fn of(observed: &LastObserved) -> Self {
        match (&observed.shown, &observed.driven) {
            (Some(shown), Some(driven)) if shown == driven => Self::Ok,
            (Some(_), Some(_)) => Self::Mismatch,
            _ => Self::Unobserved,
        }
    }
crates/holler-cli/src/pane/list.rs:170   (in `impl From<&Pane> for PaneRow`)
            sync: SessionSync::of(&pane.last_observed),
crates/holler-cli/src/pane/get.rs:86   (in `detail`)
        sync: SessionSync::of(&pane.last_observed),
crates/holler-cli/src/pane/list.rs:16
use holler_pane::pane::{Health, Hold, LastObserved};
crates/holler-cli/src/pane/list.rs:151-154   (PaneRow field docs)
    /// The session the pane's TUI shows, as reconcile last recorded it.
    pub shown: Option<String>,
    /// The session the hub drives, as reconcile last recorded it.
    pub driven: Option<String>,
crates/holler-cli/src/pane/list.rs:45-48   (PaneList's help)
/// (`pane get` shows the reason). SHOWN is the session the pane's TUI shows, and DRIVEN is
/// the session the hub drives. SYNC is `ok` when they are the same session, MISMATCH when
/// they differ, and `-` while either one is unobserved (nothing has recorded it yet, or
/// reconcile could not tell). HOLD is `none`, `parked` or `drained`.
crates/holler-cli/src/pane/list.rs:13   (module doc)
//! and they write nothing. SHOWN, DRIVEN and health are what reconcile last recorded.
crates/holler-cli/src/pane/get.rs:26-27   (PaneGet's help)
/// Every value is the pane registry's: SHOWN, DRIVEN and health are what reconcile last
/// recorded, and the verb observes nothing itself. With `--profile`, the pane must belong
crates/holler-cli/tests/pane_verbs/list.rs:112-122   (AC 3's fixture: `at: 0`, no session of record)
/// The sample pane `name` with reconcile's last observation `shown`/`driven`.
pub(crate) fn observed(name: &str, shown: Option<&str>, driven: Option<&str>) -> Pane {
    Pane {
        last_observed: LastObserved {
            shown: shown.map(str::to_owned),
            driven: driven.map(str::to_owned),
            at: 0,
        },
        ..pane(name)
    }
}
```
AC 3's tests are `list_flags_a_pane_whose_shown_and_driven_differ` (`tests/pane_verbs/list.rs:318`),
`get_flags_a_mismatch` (`get.rs:158`) and `watch_flags_a_mismatch` (`watch.rs:282`).
Every span in the block above is pre-amendment text, quoted as it stands so the reader can see what changes: the help
at `list.rs:45-48` still states the round-1 rule ("SYNC is `ok` when they are the same session ... while either one is
unobserved") and lacks `session of record`, `home screen` and `#649`, and `get.rs:26-27` and `list.rs:13` still say
DRIVEN is "what reconcile last recorded". Decision 15 replaces those words and AC 19 pins the new ones, so AC 19 fails on
this tree until F's round (that is its RED). Likewise the fixture helper above sets no `session_of_record` and `at: 0`;
the helper AC 3 needs (one that also sets `session_of_record` and `at`) is T's to write in that round, so it has no
excerpt yet.

**The other untrusted-text rule on `main` (A-dup's D-2), not adopted here:**
```
crates/holler-pane/src/findings.rs:328-334
/// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
/// control character and quote in it is escaped, and cut to 64 characters
/// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
/// included.
pub fn quoted(text: &str) -> String {
    excerpt(text)
}
crates/holler-pane/src/error.rs:688-695
pub(crate) fn excerpt(text: &str) -> String {
    const LIMIT: usize = 64;
    if text.chars().count() <= LIMIT {
        return format!("{text:?}");
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head:?}...")
}
```

**`watch`'s `data.pane` and `get`'s `data.pane` (A's W-4):**
```
crates/holler-cli/src/pane/watch.rs:94-103
/// One line of the stream, and the data of its envelope in JSON mode.
#[derive(Debug, Serialize)]
struct PaneChange {
    /// The change's place in the store's feed: pass it as `--since` to resume after it.
    cursor: Cursor,
    name: PaneName,
    change: ChangeKind,
    /// The pane after a put, as a `pane list` row; `null` after a delete.
    pane: Option<PaneRow>,
}
crates/holler-cli/src/pane/get.rs:63-67
/// The data of `pane get --format=json`. The four keys are always present.
#[derive(Debug, Serialize)]
struct PaneDetail {
    /// The record verbatim, in its own serde form.
    pane: Pane,
crates/holler-cli/src/pane/watch.rs:32-34   (PaneWatch's help)
/// `--format=json` prints NDJSON, one envelope per line, whose data is `{"cursor": N,
/// "name": NAME, "change": "put" or "delete", "pane": ROW or null}`, ROW being a `pane list`
/// row. A watch with nothing to print prints nothing, in JSON mode too.
```

**Where the code already goes past the brief (A's W-9; all in `list.rs`, the file and layer Decision 2 chose):**
```
crates/holler-cli/src/pane/list.rs:26-31
pub const COLUMNS: [&str; 9] = [
    "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
];

/// What a text cell or field holds when the record has no value.
pub const NO_VALUE: &str = "-";
crates/holler-cli/src/pane/list.rs:134, 180, 224, 234, 243, 261, 275, 284, 305, 311
pub fn profile_name(opt: &ProfileOpt) -> Result<Option<ProfileName>, PaneError> {
    pub fn cells(&self) -> [String; COLUMNS.len()] {
    pub fn text(self) -> &'static str {
pub fn health_word(health: &Health) -> &'static str {
pub fn hold_word(hold: &Hold) -> &'static str {
pub fn text_value(value: &str) -> String {
pub fn optional_text(value: Option<&str>) -> String {
pub fn json_text<T: Serialize + ?Sized>(value: &T) -> String {
fn acts_on_terminal(c: char) -> bool {
pub fn observed_at(ms: i64) -> String {
crates/holler-cli/src/pane/list.rs:261-272
pub fn text_value(value: &str) -> String {
    let plain = !value.is_empty()
        && value != NO_VALUE
        && !value
            .chars()
            .any(|c| acts_on_terminal(c) || c.is_whitespace() || matches!(c, '"' | '\\' | '='));
    if plain {
        value.to_owned()
    } else {
        format!("{value:?}")
    }
}
crates/holler-cli/src/pane/list.rs:279-283   (json_text's doc)
/// `value` as compact JSON for a text line (an argv, a list of strings, a spec). It is
/// `serde_json`'s form, except that every character that could act on a terminal (as for
/// [`text_value`]) is written as a JSON `\u` escape: `serde_json` escapes only the C0
/// controls, and leaves DEL, the C1 controls and the format characters raw. The line stays
/// valid JSON for the same value.
crates/holler-cli/src/pane/list.rs:305-307
fn acts_on_terminal(c: char) -> bool {
    c.is_control() || c.escape_debug().nth(1) == Some('u')
}
```
`docs/handoffs/643/evidence.md` ("Deviations from the brief", and Facts B-3 and NV-1) carries the source checks behind
these: `{:?}` of a `str` escapes with the same table `acts_on_terminal` reads (rustc 1.98.1, 39 characters agree), and
`json_text`'s `unwrap_or_default()` cannot be reached for an `Argv`, a `Vec<String>` or a `ProfileSpec`.

## Acceptance criteria

All run from the worktree. "In-process" tests use `verb_harness::run_verb_with` over the rig of Decision 10 (test-kit
fakes only); every `--format=json` check calls `holler_pane_testkit::envelope::check_envelope` or `check_ndjson` on the
captured `out` and exit code. Where an AC says "in both formats" the test runs the case with `Format::Text` and
`Format::Json` and asserts the two exit codes are equal (and equal to the stated code).

1. **List, text.** `list_prints_a_header_and_one_row_per_pane_sorted_by_name`: over `demo-c2r1` and `demo-c1r1` (seeded
   in that order), `pane list` exits 0, `err` is empty, and `out` is exactly three lines: the header whose cells, split on
   whitespace, are `PANE POS PROFILE PROJECT HEALTH SHOWN DRIVEN SYNC HOLD`, then the `demo-c1r1` row, then `demo-c2r1`.
   Each row's first cell is the pane name and its cells (sample panes) are `<name> r1c1 - /srv/demo unknown - - - none`.
   No line ends in a space.
2. **List, JSON.** `list_json_is_one_envelope_with_a_row_per_pane`: the same store with `Format::Json`: `check_envelope(out,
   0)` is `Ok`, `err` is empty, `data.panes` is an array of two objects in name order, each with exactly the keys `name`,
   `pos`, `profile`, `project`, `health`, `shown`, `driven`, `sync`, `hold`; `profile`, `shown` and `driven` are `null`,
   `health` is `"unknown"`, `hold` is `"none"`, `sync` is `"unobserved"`.
3. **SHOWN differs from the session of record: flagged (amendment 1).** The oracle is `shown_differs`'s answer
   (Decision 3). `list_flags_a_pane_whose_shown_differs_from_its_session_of_record` (replacing
   `list_flags_a_pane_whose_shown_and_driven_differ`): six sample panes, `demo-c1r1` to `demo-c6r1`, each with the
   `session_of_record` and `last_observed` of one row below (`at` in milliseconds; `1000` stands for any `at > 0`):

   | Pane | `session_of_record` | `shown` | `driven` | `at` | SHOWN | DRIVEN | SYNC (text) | `sync` (JSON) | Why |
   |---|---|---|---|---|---|---|---|---|---|
   | c1 | `ses-a` | `ses-b` | None | 1000 | `ses-b` | `-` | `MISMATCH` | `"mismatch"` | shows another session |
   | c2 | `ses-a` | `ses-a` | None | 1000 | `ses-a` | `-` | `ok` | `"ok"` | shows its session of record |
   | c3 | `ses-a` | None | None | 1000 | `-` | `-` | `MISMATCH` | `"mismatch"` | observed home screen (`shown_differs` doc) |
   | c4 | `ses-a` | None | None | 0 | `-` | `-` | `-` | `"unobserved"` | never observed |
   | c5 | None | `ses-a` | None | 1000 | `ses-a` | `-` | `-` | `"unobserved"` | no session of record to compare with |
   | c6 | `ses-a` | `ses-a` | `ses-b` | 1000 | `ses-a` | `ses-b` | `ok` | `"ok"` | DRIVEN is printed as stored, never compared |

   Text: each row's SHOWN, DRIVEN and SYNC cells are as above. JSON: `shown` and `driven` are the strings or `null`, and
   `sync` is as above. `get_flags_a_mismatch` asserts the same six SYNC values for `get` (text line `sync: MISMATCH`,
   `sync: ok` or `sync: -`; JSON `data.sync`), and `watch_flags_a_mismatch` for a `watch` line (text `sync=...`, JSON
   `data.pane.sync`). The fixture helper sets `session_of_record` and `at` as well as `shown` and `driven` (AC 3's
   current helper sets neither, Evidence, "Added for amendment 1").
4. **An unhealthy server is shown.** `list_shows_an_unhealthy_server`: a pane with `harness.health =
   Health::Unhealthy("server wedged")`: its HEALTH cell is `unhealthy`; its JSON `health` is `{"unhealthy": "server
   wedged"}`. `get` of it prints the line `health: unhealthy "server wedged"`, and its JSON `data.pane.harness.health`
   is `{"unhealthy": "server wedged"}`.
5. **Positions print row first.** `every_verb_prints_positions_row_first`: a pane **named `demo-c1r2`** at
   `GridPos { row: 2, col: 1 }`: list's POS cell is `r2c1`; `get` prints the line `pos: r2c1`; a watch line contains
   `pos=r2c1`. In JSON, the raw `out` of list contains `"pos":{"row":2,"col":1,"pos":"r2c1"}`, the raw `out` of get
   contains `"grid":{"row":2,"col":1,"pos":"r2c1"}`, and the raw watch line contains `"pos":{"row":2,"col":1,"pos":"r2c1"}`
   (compact, key order as written: row first). No POS cell, `pos:` line or `pos=` value of any test equals `c1r2`.
6. **`--profile` scopes each verb.** Store: `demo-c1r1` and `demo-c2r1` with `profile: demo`; `demo-c3r1` with `profile:
   other` while the profile `demo` also holds a spec for `demo-c3r1` (a detached spec); `demo-c4r1` with no profile.
   Profiles `demo` (specs for c1, c2, c3) and `other` (spec for c3) are seeded.
   - `list_profile_lists_only_the_profile_s_members`: `pane list --profile demo` lists exactly c1 and c2 (not c3, whose
     spec is in `demo` but whose record names `other`; not c4); their PROFILE cell is `demo`; JSON `profile` is `"demo"`.
   - `get_profile_member_is_shown`: `pane get demo-c1r1 --profile demo` exits 0.
   - `watch_profile_prints_only_member_changes`: `pane watch --profile demo --until-idle` prints exactly the puts of c1
     and c2.
   - `profile_refusals_exit_3_in_both_formats`, for each verb: `--profile nope` (no such profile) is `profile-not-found`;
     `list demo-c3r1 --profile demo`, `get demo-c3r1 --profile demo`, `get demo-c4r1 --profile demo` and `watch demo-c4r1
     --profile demo` are `pane-not-in-profile`; all exit 3 in both formats; text puts one `error: ...` line on `err` and
     nothing on `out`; JSON passes `check_envelope(out, 3)` with that `error.code`, and `err` is empty.
7. **`get` shows the whole record, ceilings included.** `get_shows_every_field_of_the_record`: a pane in profile `demo`
   with `model {demo-provider, demo-model, high}`, `env [ALPHA_TOKEN, BETA_URL]`, `context {soft 64000, hard 96000}`,
   `command ["opencode", "serve", "--port", "48100"]`, `probe {check ["curl", "-s", "http://127.0.0.1:48100/health"],
   expect ["ok"], last Failed {missing ["ok"]}}`, `session_of_record "ses-a"`, `hold Parked {reason "maintenance",
   release_when "after the deploy", since 0}`. Text `out` holds, each as one whole line: `pane: demo-c1r1`,
   `profile: demo`, `model: demo-provider/demo-model`, `effort: high`, `env: ALPHA_TOKEN BETA_URL`,
   `context: soft=64000 hard=96000`, `command: ["opencode","serve","--port","48100"]`,
   `probe-check: ["curl","-s","http://127.0.0.1:48100/health"]`, `probe-expect: ["ok"]`,
   `probe-last: failed missing=["ok"]`, `session-of-record: ses-a`, `hold: parked reason=maintenance until="after the
   deploy" since=never`. JSON: `data.pane` equals `serde_json::to_value(&pane)` with the stored generation;
   `data.profile` is `"demo"`; `data.spec` is the `demo` profile's spec for `demo-c1r1` (its `context` keys `soft` and
   `hard` present); `data.sync` is present.
8. **`get` of a pane with no profile still shows its ceilings.** `get_pane_without_a_profile_shows_its_ceilings`: text
   has `profile: -`, `spec: -` and the `context:` line; JSON `data.profile` and `data.spec` are `null` and
   `data.pane.context` is `{"soft":100000,"hard":150000}` (the sample ceilings).
9. **`get` of a missing pane.** `get_missing_pane_is_pane_not_found_in_both_formats`: `pane get demo-c9r9` (no
   `--profile`, empty store) exits 3 in both formats with code `pane-not-found`; the text `err` line is
   `error: pane not found: demo-c9r9`. `get_requires_a_pane_name`: `Cli::try_parse_from(["holler", "pane", "get"])` is an
   `Err` of kind `ErrorKind::MissingRequiredArgument`.
10. **Bad names are usage (exit 2) in both formats.** `bad_names_are_usage_in_both_formats`: `pane list --profile ""`,
    `pane get NOT_A_PANE!` and `pane watch --profile "  "` exit 2 with code `usage`, nothing on `out` in text mode.
11. **Store failures are runtime failures (exit 1) in both formats.** `store_failures_exit_1_in_both_formats`: with the
    pane store's standing fault `Fault::Fail(PaneError::Unavailable { what: "hub".into() })`, each of `list`, `get
    demo-c1r1` and `watch --until-idle` exits 1 with code `unavailable`; with `Fault::Wedged`, each exits 1 with code
    `timeout`. JSON passes `check_envelope(out, 1)` for list and get and `check_ndjson(out, 1)` for watch.
12. **`watch` from the start prints the current state once.** `watch_from_the_start_prints_each_live_pane_once`: seed
    `demo-c1r1`, `demo-c2r1` (cursors 1, 2); then through the port `cas_put` c1 (3), `cas_put` c2 (4), `delete` c1 (5),
    `concurrent_put` of a new `demo-c3r1` (6). `pane watch --until-idle` (JSON) exits 0; `check_ndjson(out, 0)` is `Ok`
    with exactly two envelopes, `data.cursor` 4 then 6, `data.name` `demo-c2r1` then `demo-c3r1`, `data.change` `"put"`.
13. **`watch --since` prints each later change once, in order.** `watch_since_prints_each_later_change_once`: the same
    store, `pane watch --since 2 --until-idle`: exactly four envelopes, cursors 3, 4, 5, 6 in that order, each cursor
    once; the cursor-5 line is `{"cursor":5,"name":"demo-c1r1","change":"delete","pane":null}` as its `data`. In text mode
    the same run prints four lines, the delete line being exactly `cursor=5 delete demo-c1r1`. `--since 6` prints nothing
    and exits 0.
14. **A change made while `watch` waits is printed exactly once.** `watch_prints_a_concurrent_change_exactly_once`: the
    AC 12 store with its idle wait set to 2 s (`set_idle_wait`); `pane watch --since 6 --until-idle` (JSON; 6 is the
    head, so nothing is owed yet) runs on a `std::thread::scope` thread over the same rig; the main thread polls the
    pane store's `faults().calls()` until it holds `PaneStoreOp::WatchNext` (bounded: fail after 5 s), then `cas_put`s
    `demo-c2r1` (cursor 7). The verb exits 0 and `check_ndjson(out, 0)` is `Ok` with exactly one envelope, cursor 7.
    The test builds a fresh rig and repeats the case 5 times.
15. **`watch` on a named pane; a store error ends the stream.** `watch_named_pane_follows_only_that_pane`: with the
    AC 12 store, `pane watch demo-c2r1 --since 0 --until-idle` prints only cursor-4 (`demo-c2r1`). `watch_ends_at_a_store_error`:
    `faults().fail_next(PaneStoreOp::WatchNext, PaneError::Unavailable { .. })`: the verb exits 1; JSON `out` is one
    failure envelope coded `unavailable` (`check_ndjson(out, 1)`); text `err` is one `error: unavailable: ...` line.
    `watch_since_ahead_of_the_head_is_usage`: `--since 99` on the AC 12 store exits 2, code `usage`, in both formats.
16. **`watch` that owes nothing.** `watch_with_nothing_owed_prints_nothing`: an empty store, `pane watch --until-idle`
    exits 0 with `out` empty in both formats (no envelope at all: Decision 7).
17. **The verbs observe nothing.** `read_verbs_call_no_adapter_or_probe`: after one successful run of each of `list`,
    `get demo-c1r1`, `list --profile demo`, `get demo-c1r1 --profile demo` and `watch --until-idle`, the rig's
    `FakeHerdr`, `FakeHost` and `FakeHarness` `faults().calls()` and `FakeProber::calls()` are all empty. And
    `grep -nE 'ports\.(herdr|host|harness|prober)' crates/holler-cli/src/pane/list.rs crates/holler-cli/src/pane/get.rs
    crates/holler-cli/src/pane/watch.rs` prints nothing.
18. **Text output cannot inject terminal control sequences.** `text_output_escapes_control_characters`: a pane whose
    `host.cwd` is `"/srv/demo\u{1b}[31mred\nfake"` and whose health is `Unhealthy("bad\rline")`: in each of list, get and
    a watch line, text `out` contains no `\u{1b}`, no `\r`, and exactly the expected number of `\n` (list: header + 1 row;
    get: one per field line; watch: 1), and the project value appears as `"/srv/demo\u{1b}[31mred\nfake"` (Rust's
    `{:?}` escaping: the backslash sequences are literal text). JSON output of the same pane passes the envelope helper.
19. **Output documented in `--help`.** `list_help_documents_the_columns_and_the_json_shape`: the rendered error of
    `Cli::try_parse_from(["holler", "pane", "list", "--help"])` (kind `DisplayHelp`) contains each of `POS`, `PROFILE`,
    `PROJECT`, `HEALTH`, `SHOWN`, `DRIVEN`, `SYNC`, `MISMATCH`, `HOLD`, `--format=json`, `"panes"`, `r2c1`,
    `session of record` and `home screen` (amendment 1: SYNC's rule), and `#649` (amendment 1: why DRIVEN is empty).
    `get_help_documents_the_json_shape`: `pane get --help` contains `"pane"`, `"profile"`, `"spec"`, `"sync"` and
    `context`. `watch_help_documents_the_stream`: `pane watch --help` contains `NDJSON`, `--since`, `--until-idle`,
    `"cursor"`, `"change"` and `pane get` (amendment 1, W-4: the help says the stream's "pane" is a row, not the record
    `pane get` prints).
20. **The surface rows.** ADR 0003's three `#643` rows are exactly:
    ```
    holler pane list [PANE] [--profile NAME]                          #643
    holler pane get PANE [--profile NAME]                             #643
    holler pane watch [PANE] [--profile NAME] [--since CURSOR] [--until-idle]   #643
    ```
    and the fixture's `# #643` block is exactly the nine lines of Decision 13. `cargo test -p holler-cli --test
    cli_surface_test`, `--test docs_cli_test` and `--test pane_cli_process` pass (the last one with the three `#643` rows
    removed from `STUBS` and the `// #643` line kept: `grep -n '// #643' crates/holler-cli/tests/pane_verbs/process/stub.rs`
    prints one line, and `grep -n '"pane", "list", 643\|"pane", "get", 643\|"pane", "watch", 643'` on that file prints
    nothing).
21. **No stub left.** `grep -nE 'not_implemented|const STORY' crates/holler-cli/src/pane/list.rs
    crates/holler-cli/src/pane/get.rs crates/holler-cli/src/pane/watch.rs` prints nothing.
22. **Quality gates** (the tester overlay's Tier 1, as CI runs them): `bash scripts/lint.sh`, `bash
    scripts/changelog-check.sh`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p holler-cli --test
    pane_verbs`, `cargo test --workspace`, `cargo machete` all pass. Formatting: `rustfmt --check --edition 2021` exits 0
    on each of the seven `.rs` files this story touches (the three verb files, the three test files, `process/stub.rs`;
    `cargo fmt --check` across the workspace fails on `3bdd129` already and is not the gate, see C4). No new `unsafe`:
    `grep -n unsafe` on the seven files prints nothing. No new dependency: `git diff --stat origin/main --
    Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` prints nothing. Every touched `.rs` file is under 900 lines and every
    function under clippy's 100-line threshold.
23. **CHANGELOG.** `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry for the three read verbs linking
    `[#643](https://github.com/Performant-Labs/holler/issues/643)`; it names no host or account. Amendment 1: the entry
    describes SYNC by `shown_differs`'s rule and DRIVEN as stored, in place of its two current sentences:
    - "SYNC reads `MISMATCH` when SHOWN and DRIVEN differ." becomes, in substance: "SYNC reads `MISMATCH` when the pane's
      TUI shows another session than its session of record, or its home screen: the rule `holler pane doctor` uses."
    - "The verbs observe nothing themselves: SHOWN, DRIVEN and health are what reconcile last recorded." becomes, in
      substance: "The verbs observe nothing themselves: SHOWN and health are what reconcile last recorded, and DRIVEN is
      printed as the record holds it, which is empty until the hub wiring (#649) records it."
    The entry still ends with the `[#643]` link, and `bash scripts/changelog-check.sh` passes. `pane doctor`'s and
    `profile list`/`show`'s entries from `main` stay as they are.
24. **Blast radius.** `git diff --name-only origin/main...HEAD` lists only the files under "Files" (plus `docs/handoffs/643*`).

## Files

Production (F):
- `crates/holler-cli/src/pane/list.rs` (rewrite, ~290 lines): `PaneList` args, `run`, and the shared view code of
  Decision 2 (`PaneRow`, `SessionSync`, `text_value`, `observed_at`, the table renderer).
- `crates/holler-cli/src/pane/get.rs` (rewrite, ~210 lines): `PaneGet` args, `run`, `PaneDetail`, the `key: value`
  renderer.
- `crates/holler-cli/src/pane/watch.rs` (rewrite, ~210 lines): `PaneWatch` args, `run`, `PaneChange`, the member filter,
  the line renderer.
- `CHANGELOG.md` (one entry, AC 23).

Surface (T, in the RED step, see the Test plan; the epic's ruling 2 assigns these to the verb's story):
- the three `Args` structs' fields (in the three files above; F keeps them and writes their docs);
- `docs/adr/ADR-0003.md` lines 44-46 (AC 20);
- `crates/holler-cli/tests/fixtures/cli-surface.txt` lines 104-111 (Decision 13);
- `crates/holler-cli/tests/pane_verbs/process/stub.rs` lines 19-21 deleted, line 18 `// #643` kept (AC 20).

Tests (T):
- `crates/holler-cli/tests/pane_verbs/list.rs` (rewrite, ~360 lines, with the rig of Decision 10),
- `crates/holler-cli/tests/pane_verbs/get.rs` (rewrite, ~260 lines),
- `crates/holler-cli/tests/pane_verbs/watch.rs` (rewrite, ~300 lines).

**Amendment 1's change, in the same files** (no new file; the code exists, at 316, 275 and 227 lines, and the tests at
545, 370 and 311; every line number below is the line's place in the current, pre-amendment tree, the code to be
changed):
- `crates/holler-cli/src/pane/list.rs`: `SessionSync::of` takes `&Pane` and calls `shown_differs` (Decision 3); its
  call site in `PaneRow::from` (:170); `SessionSync`'s docs (:195-221), `PaneRow`'s `shown`/`driven` docs (:151-154),
  `PaneList`'s help (:45-48) and the module doc (:6, :13); the `LastObserved` import (:16) goes if unused.
- `crates/holler-cli/src/pane/get.rs`: the call site in `detail` (:86), and the module doc and help (:1-3, :26-27).
- `crates/holler-cli/src/pane/watch.rs`: one help sentence (W-4, Decision 7).
- `CHANGELOG.md`: the two sentences of AC 23.
- T: AC 3's three tests and their fixture helper (`tests/pane_verbs/list.rs:112-122`, `:318`; `get.rs:158`;
  `watch.rs:282`), and AC 19's new substrings.

**Size check.** Production ~710 lines in three files, tests ~920 in three, plus ~25 lines of surface and changelog edits:
~1,655 in all. F edits by hand four files (three verb files and `CHANGELOG.md`); T edits six (three test files, the
fixture, the ADR rows, `stub.rs`) plus the three `Args` structs. One component family (the read verbs of
`holler-cli/src/pane/`), within F's ~6-file cap: **fits one run**, no split. If a test file nears 800 lines, move cases
into the next of the three files (all three are #643's); no new test file is possible without editing the frozen
`tests/pane_verbs/main.rs`.

**Blast radius:** the issue's three files, plus the epic-assigned surface of ruling 2 (ADR 0003 rows, fixture lines,
`tests/pane_verbs/{list,get,watch}.rs`), the three `STUBS` rows, and `CHANGELOG.md`. No other file; in particular not
`pane/mod.rs`, `pane/args.rs`, `output.rs`, `wiring.rs`, `main.rs`, `tests/pane_verbs/main.rs`, any `Cargo.toml`,
ADR-0021, or the test kit. Amendment 1 keeps this: it calls `holler_pane::reconcile::shown_differs` and edits no file of
`holler-pane` (`shown_differs`, its doc, `findings.rs`) and no ADR (the MO rejected option (b), which would have edited
`shown_differs`'s doc and ADR-0021 §11). The files `main` brought in by the merge (`05e7337`) are not this story's, and
`git diff --name-only origin/main...HEAD` (AC 24) does not list them once `origin/main` is at or past `ce12cdb`.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `output::{emit, emit_stream, emit_error, ErrorBody}` (`output.rs:201-243`, `134-141`) | every result and error; the exit code | reuse; no printing of its own, no exit-code table |
| `ProfileOpt` (`args.rs:19-25`) | `--profile` on all three | reuse (flattened, as the stubs do) |
| `PaneStore::{get, list, watch}`, `ProfileStore::get`, `ProfileScope::resolve` | the only port calls | reuse; frozen |
| `PaneName::parse`, `ProfileName::parse` | type the positional and `--profile`; refusals are `usage` | reuse |
| `GridPos` Display / Serialize (`grid.rs:68-73`, `129-138`) | POS text and JSON | reuse; no second formatter |
| `Pane`, `Health`, `Hold`, `ProbeResult`, `ProfileSpec`, `Cursor` serde | the JSON of `get` (record verbatim) and the row's `health`/`hold` | reuse their serde forms |
| `PaneError::{PaneNotFound, PaneNotInProfile}` | the codes this story raises itself | reuse closed variants; no new code |
| `time_fmt::format_epoch` (`time_fmt.rs:11`) | text timestamps | reuse |
| `serde_json::to_string` | argv and spec in text mode | reuse, inside `json_text` (Decision 6, W-9) |
| `verb_harness::run_verb_with` | every in-process test | reuse |
| test kit fakes, `fixture::{sample_pane, sample_profile}`, `envelope::{check_envelope, check_ndjson}` | every test | reuse; the kit is not edited |
| `holler_pane::reconcile::shown_differs` (`reconcile.rs:171-181`, `pub`; amendment 1) | SYNC's comparison: `session_of_record` against `last_observed.shown` | **reuse, called** from `SessionSync::of`; no comparison of the verbs' own, and `last_observed.driven` is never compared (Decision 3) |
| `PaneRow`, `SessionSync`, `text_value` and the other `pub` items of Decision 2 | **new**, `pub` in `list.rs`, used by `get.rs`, `watch.rs` and later by #648's roster | new, one copy; `SessionSync` keeps its three values and serde names and **extends** `shown_differs` with the `unobserved` case, it does not restate it |
| `holler_pane::findings::quoted` (`findings.rs:328-334`; A-dup's D-2) | not used | **decided non-fold**: `quoted` always quotes and cuts at 64 characters (`error::excerpt`), so folding onto it would quote every plain table cell (AC 1 pins bare cells) and cut a long cwd in `get` (AC 7 pins it uncut). One rule for both crates would be a new `holler-pane` helper with two faces (quoted and cut for messages; bare when plain and uncut for fields), added amend-first and changing no output of this story's. Follow-up, not this change |

## Decisions already made (O)

1. **Arguments.** `pane list [PANE] [--profile NAME]`, `pane get PANE [--profile NAME]`, `pane watch [PANE] [--profile
   NAME] [--since CURSOR] [--until-idle]`. The positional is `pane: Option<String>` (`String` for `get`) with
   `value_name = "PANE"`; `--since` is `Option<u64>`, `value_name = "CURSOR"`; `--until-idle` is a `bool`. Each struct
   still flattens `ProfileOpt`. Positionals and `--profile` stay strings at clap time and are typed in `run` with
   `PaneName::parse` / `ProfileName::parse`, so a bad name is an `emit`ted `usage` (exit 2, an envelope in JSON mode)
   like the spec flags' guards, not a clap error. Why `list` and `watch` take an optional pane: ADR-0021 §3 says "a named
   pane outside P is `pane-not-in-profile`" for the read verbs and §9 lists that code for `list` and `watch`; it can only
   arise with a named pane. `list PANE` without `--profile` and with no such pane is an empty table, exit 0 (§9 lists no
   `pane-not-found` for `list`); `watch PANE` without `--profile` waits for that pane to appear (likewise). `get` alone
   refuses a missing pane (`pane-not-found`, §9).
2. **Shared code lives in `list.rs`, as `pub` items** (the frozen `pane/mod.rs` admits no new module): `PaneRow` (the
   one-pane summary: `Serialize`, built `From<&Pane>`), `SessionSync`, `text_value(&str) -> String` and
   `observed_at(i64) -> String`. `get.rs` and `watch.rs` use them through `super::list::...`; #648's `roster_cmd.rs` can
   use them through `crate::pane::list::...`. No copy of any of them elsewhere. **As built (W-9, point 1):** seven more
   `pub` items share the same home and rule: `COLUMNS` and `NO_VALUE` (`list.rs:26-31`), `profile_name` (:134),
   `health_word` (:234), `hold_word` (:243), `optional_text` (:275) and `json_text` (:284), plus the methods
   `PaneRow::cells` (:180) and `SessionSync::text` (:224). `COLUMNS` and `PaneRow::cells` are the one definition behind
   both the `list` table and the `watch` line. This is accepted as the design, not drift: one copy each, same file and
   layer.
3. **SHOWN/DRIVEN and the flag (amendment 1, the MO's ruling (a1)).** SYNC is `holler_pane::reconcile::shown_differs`'s
   answer, the one rule on `main`, which `pane doctor` uses and whose doc names `pane get` and the roster as its readers
   (Evidence, "Added for amendment 1"). `SessionSync` keeps its three values and serde names (`rename_all =
   "snake_case"`: `"ok"`, `"mismatch"`, `"unobserved"`; text `ok`, `MISMATCH`, `-`), and `SessionSync::of(pane: &Pane)`
   stays the only place `list`, `get` and `watch` read SYNC (its two call sites are `PaneRow::from` and `get`'s
   `detail`; `watch` gets SYNC through `PaneRow`):
   - `Unobserved` when `pane.last_observed.at == 0` (never observed: `shown_differs`'s doc, "`0` is never observed"),
     or when `pane.session_of_record` is `None` (there is nothing to compare with; `shown_differs` answers `false`, and
     `ok` would claim a sync nobody checked; doctor reports that pane as `no-session-of-record`);
   - otherwise `Mismatch` when `shown_differs(pane.session_of_record.as_deref(), pane.last_observed.shown.as_deref())`
     is `true`, and `Ok` when it is `false`. So a `shown` of `None` with `at > 0` is `Mismatch`: reconcile wrote an
     observed home screen (`reconcile.rs:31-32`), and the TUI is not on the session the hub drives.

   The verbs compare nothing themselves: the comparison is the call. **DRIVEN is printed, not compared:** the DRIVEN
   column, the `driven` key and `get`'s `driven:` line keep printing `last_observed.driven` as stored. Nothing on `main`
   writes it (reconcile leaves it as stored, ADR-0021 §11), so it reads `-` / `null` on every real record until #649
   wires DRIVEN; #649 owns its writer. The help text says so (Decision 15). This replaces the round-1 rule (SHOWN
   against `last_observed.driven`, `None` on either side `unobserved`), whose reason, "`None` means reconcile could not
   tell", does not hold for the writer on `main`: an unseen screen leaves `shown` as stored (`observe.rs:369-371`). The
   verbs still never call an adapter (the issue: "the verbs do not observe anything themselves"), and SHOWN and DRIVEN
   still come from the record's `last_observed`.
4. **`list` output.** Rows sorted by pane name (the verb sorts; it does not rely on the store's order). Text: a header
   `PANE POS PROFILE PROJECT HEALTH SHOWN DRIVEN SYNC HOLD`, then one row per pane, columns left-aligned to the widest
   cell (width in `char`s), separated by two spaces, the last column unpadded, no trailing space; zero panes print the
   header alone. Cells: name; `GridPos` Display; profile or `-`; `host.cwd`; `healthy` / `unhealthy` / `unknown` (the
   reason is `get`'s); shown or `-`; driven or `-`; the SYNC value; `none` / `parked` / `drained`. JSON `data`:
   `{"panes": [PaneRow, ...]}` (an object, so a field can be added later without a `schema_version` bump, ADR-0021
   §9). `PaneRow` keys, in order: `name`, `pos` (`GridPos`), `profile` (string or `null`), `project`, `health` (the
   `Health` serde form), `shown` and `driven` (string or `null`), `sync`, `hold` (the `Hold` serde form). The `Option`
   fields of `PaneRow` serialize as `null` (no `skip_serializing_if`).
5. **`get` output.** JSON `data`: `{"pane": <the Pane record, its own serde form>, "profile": <name or null>, "spec":
   <ProfileSpec or null>, "sync": <SessionSync>}`. `pane` is the record verbatim, so every field ADR-0021 §1 lists is
   there (model, effort, env names, context ceilings, command, probe and its last result), with the record's own rule
   that an unset optional field is left out (ADR-0021 §1, "Optional fields are left out when absent"). The three
   top-level fields are always present. `profile` is the record's own `profile` field on both paths (as built,
   `get.rs:83-84`: `profile: pane.profile.clone()`, where `pane` is `resolve`'s with `--profile P` and the store's
   `get` without it, the same record either way); AC 7's `"demo"` is that field. `spec`: with `--profile P`, the entry of `resolve(P, Some(n)).profile.panes`
   whose `pane` equals the name; without `--profile`, if the record names a profile Q, `profile_store.get(Q)` and that
   entry; `null` when there is no profile, the profile record is gone (`Ok(None)`) or it holds no entry for the pane. A
   `profile_store.get` **error** fails the verb (exit 1), it is not turned into `spec: null`. Text: one `key: value` line
   per field, in this order: `pane`, `generation`, `pos`, `profile`, `project`, `herdr` (`session=<s> workspace=<w>
   pane-id=<id>`), `host`, `tmux`, `herdr-api-version`, `harness` (`opencode port=<p> pid=<pid or ->`), `health`
   (`healthy`, `unknown`, or `unhealthy <reason as text_value>`), `session-of-record`, `shown`, `driven`, `observed-at`,
   `sync`, `role`, `hold` (`none`, `drained`, or `parked reason=<v> until=<v> since=<v>`, each `<v>` through
   `text_value`, `since`'s being `observed_at(since)`, so a date with a space prints quoted), `model`
   (`<provider>/<model_id>`), `effort`, `env` (names separated by one space, or `-`), `context` (`soft=<n> hard=<n>`),
   `command`, `probe-check`, `probe-expect`, `probe-last` (`ok`, `failed missing=<JSON array>`, `error <reason as
   text_value>`, or `-`), `spec` (compact JSON of the `ProfileSpec`, or `-`). `--profile P` with a member: the pane from
   `resolve` is used (no second store read). `resolve` returning an empty `panes` (it should not) is
   `pane-not-in-profile`.
6. **Argv in text is a JSON array, never joined.** `command`, `probe-check`, `probe-expect` and `probe-last`'s
   `missing` print `serde_json::to_string` of the array: `["opencode","serve"]`. A space-joined rendering would be
   ambiguous (`"a b"` versus `a`, `b`) and, pasted into a shell, would re-split and expand: exactly what B2 forbids. The
   verbs spawn nothing and pass nothing to a shell. **As built (W-9, point 2):** the JSON goes through `json_text`
   (`list.rs:284-299`): `serde_json::to_string`'s value, with each character `acts_on_terminal` flags written as a JSON
   `\u` escape, since `serde_json` escapes only the C0 controls, `"` and `\`. The line is still valid JSON for the same
   value. Accepted as the design.
7. **`watch`.** `run` parses the names, then, with `--profile P`, calls `scope.resolve(P, pane)` first (so a missing P or
   a named non-member refuses before the stream opens, exit 3), then `pane_store.watch(Cursor(since.unwrap_or(0)))`; an
   error there is `emit_error`. The items go through `emit_stream`:
   - `Ok(Some(event))` that passes the filter (below) becomes one `PaneChange`: JSON `data` `{"cursor": <u64>, "name":
     <pane>, "change": "put"|"delete", "pane": <PaneRow or null>}` (W-4, below); text `cursor=<n> put <name> pos=<pos>
     profile=<v> project=<v> health=<v> shown=<v> driven=<v> sync=<v> hold=<v>` (the list cells as `key=value`), or
     `cursor=<n> delete <name>`;
   - `Ok(None)` (idle) prints nothing; with `--until-idle` it ends the stream, exit 0; without it the verb keeps polling
     (the port's `next()` blocks up to I5's bound, ADR-0021 §6's long-poll, so this is not a busy loop on a real store);
   - `Err(e)` ends the stream through `emit_stream`'s error item (the port: "any error ends the stream"); the verb does
     not reconnect, so a wedged store is loud (`timeout`, exit 1), and the user resumes with `--since <last cursor>`.
   The filter: with `PANE`, only events of that name; with `--profile P`, an event prints when its record names P
   (slug compared, as `ProfileName::slug`) **or** the pane was in P as of the last record this watch saw (so a pane
   leaving P, or deleted while in P, prints that one event); the "in P" set starts as the names `resolve` returned and
   is updated by each event. A watch that owes nothing prints nothing, in JSON mode too: there is no envelope for "no
   change", and `check_ndjson` (which refuses an empty stream) is applied only to runs that print at least one line
   (AC 16 checks the empty case directly).
   **`data.pane` is a row, not the record (amendment 1, A's W-4: kept, documented).** In `watch`, `data.pane` is a
   `PaneRow` (`watch.rs:94-103`), while `get`'s `data.pane` (`get.rs:63-67`) and the port's `PaneEvent.pane` are the
   full `Pane` record. The key keeps its name: renaming it (to `row`) would change `PaneChange`, AC 13's pinned delete
   line (`"pane":null`), the watch help and the tests, and it buys only a name. Keeping it costs one help sentence. So
   `pane watch --help` says, in substance, that "pane" is a `pane list` row, not the full record `pane get` prints, and
   that a script that needs the record runs `pane get` (AC 19 pins `pane get` in that help). After merge the name is
   public JSON (ADR-0021:409-410), so this decision is final for schema version 1.
8. **"Each change once" is the port's contract, not a verb-side dedupe.** The verb prints every event the stream
   yields, in order, and keeps no cursor filter of its own. From `Cursor(0)` the port yields the current state (one put
   per live record) and then every later change; from `--since N` every change after N (`feed.rs:18-31`). A dedupe in
   the verb could be tested only against a store that repeats, which no test-kit fake does, and the issue limits tests
   to the kit's fakes. AC 12-14 pin the observable property over the fake, including a write made while the verb waits.
9. **Read-only, and nothing observed.** The verbs call only `pane_store.get/list/watch`, `profile_store.get` (in `get`)
   and `scope.resolve`. Never `herdr`, `host`, `harness` or `prober` (AC 17). They write nothing.
10. **The test rig** lives in `tests/pane_verbs/list.rs` as `pub(crate) struct Rig` (a new test module needs a `mod`
    line in the frozen `tests/pane_verbs/main.rs`): `Arc<FakePaneStore>`, `Arc<FakeProfileStore>`, a `FakeProfileScope`
    built over clones of those two `Arc`s with the actor `test`, `FakeHerdr::new("scratch")`, `FakeHost::new()`,
    `FakeHarness::new()`, `FakeProber::new()`; `Rig::new(panes, profiles) -> Result<Rig, PaneError>` (seeding through
    `seeded`), `Rig::ports(&self) -> Ports<'_>`, `Rig::run(&self, argv, Format) -> Outcome` (over `run_verb_with`) and
    `Rig::assert_nothing_observed(&self)`. `get.rs` and `watch.rs` use `crate::list::Rig`. Every pane comes from
    `sample_pane` with neutral `demo-*` names, edited field by field. `testkit_links` and its `use holler_pane_testkit as
    _;` stay as they are, so the manifest comment that names `testkit_links` stays true (`Cargo.toml` is outside this
    blast radius). The three `assert_stub_routes` cases are deleted.
11. **Text values are escaped (security).** Stored strings reach a terminal in text mode: a cwd, a health reason, a
    session id, a profile name, a hold reason, a herdr id. `text_value(s)` returns `s` unchanged unless `s` is empty or
    contains any `char::is_control` character, any `char::is_whitespace` character, `"`, `\` or `=`, in which case it
    returns `format!("{s:?}")` (Rust's escaped, quoted form: ESC becomes the six characters `\u{1b}`, a newline `\n`).
    Every stored string printed in text mode goes through it, table cells included; the fixed words the verb writes
    (`-`, `ok`, `put`, ...) and the JSON-rendered values of Decision 6 do not. So no stored value can emit an escape
    sequence, a carriage return or an extra line, and a `key=value` line stays parseable. JSON mode is escaped by
    `serde_json`. **As built (W-9, point 3):** the trigger set is wider (`list.rs:261-272`, `:305-307`). `text_value`
    also quotes a stored `-`, so that it never reads as the empty value `-` (no pane or profile name can be `-`, so only
    other stored strings such as a session id or a cwd can print as `"-"`), and it quotes any character
    `acts_on_terminal` flags: `is_control`, or a character `{:?}` writes as a `\u{..}` escape (bidi and format
    controls, zero-width and combining characters, line separators, unassigned code points). Both helps say so. Accepted
    as the design.
12. **Timestamps in text.** `observed_at(ms)`: `never` for `ms <= 0`, else `format_epoch(ms / 1000)` followed by ` UTC`
    (the existing helper's form; the CLI has no local-time formatter and adds none). JSON keeps the record's raw
    milliseconds.
13. **The fixture block** (`cli-surface.txt`, replacing lines 105-111, header line 104 kept):
    ```
    pane list |
    pane list | demo-c1r1
    pane list | --profile demo
    pane list | --format=json
    pane get | demo-c1r1
    pane get | demo-c1r1 --profile demo --format json
    pane watch |
    pane watch | demo-c1r1 --since 7 --until-idle
    pane watch | --profile demo --json
    ```
    and the ADR 0003 rows are AC 20's. `pane get |` (no positional) no longer parses, so it leaves the fixture.
14. **Exit codes** come from `emit` through `class_of`: ok 0; `usage` 2 (a bad pane or profile name, `--since` ahead of
    the store's head); `profile-not-found`, `pane-not-in-profile`, `pane-not-found` 3; `unavailable`, `timeout`,
    `store-corrupt`, `not-implemented` 1. The same in both formats, because `emit` decides them (AC 6, 9-11, 15). No open
    code is declared; the verbs raise only `PaneError::PaneNotFound` (get, no profile) and `PaneError::PaneNotInProfile`
    (an empty resolve, Decision 5) themselves, both closed.
15. **`--help` is the documentation.** Each struct's doc comment is the long help (Evidence: clap prints the struct's doc
    comment, unwrapped). It names the columns and what each means (POS row first, `r2c1`; SHOWN is what reconcile last
    recorded; DRIVEN is the session the hub drives, printed as the record holds it, which nothing records until the hub
    wiring, #649; SYNC is `ok` when the TUI shows the pane's session of record, MISMATCH when it shows another session
    or its home screen, `-` when reconcile has not observed the pane or it has no session of record), that text is for
    people and `--format=json` for scripts, and the `data` shape of Decisions 4, 5 and 7 including `--since`,
    `--until-idle` and W-4's sentence. `get`'s help and the module docs say the same of SHOWN, DRIVEN and SYNC, and
    `SessionSync`'s and `PaneRow`'s docs (Evidence, "Added for amendment 1") stop calling DRIVEN "as reconcile last
    recorded it". AC 19 pins the words.
16. **Bounded time (I5).** `list` and `get` make at most three bounded port calls each. `watch` without `--until-idle` is
    a stream: each `next()` is bounded by the port and the verb runs until the user stops it, a write fails (a closed
    pipe ends it, `output.rs`), or the store errors. That is what "follows the change feed" means; I5's "returns in
    bounded time" is read per port call for this one verb (C5).

Forward-compat (consumers):

| Consumer | Needs | Satisfied |
|---|---|---|
| #648 roster columns (SHOWN/DRIVEN, POS, PROFILE, `-` for none) | the same cells and the same mismatch rule | `pub` `PaneRow`, `SessionSync`, `text_value` in `list.rs` (2, 3); the rule underneath is `shown_differs`, which the roster may also call directly |
| #649 hub wiring | the verbs to use only `PaneStore::{get,list,watch}`, `ProfileStore::get`, `ProfileScope::resolve` | yes (9); nothing here is wired |
| pfleet watchdog (#653) | the context ceilings of any pane from `get --format=json` | `data.pane.context.{soft,hard}` always; `data.spec.context` when it has a profile (5) |
| #652 operator guide | stable, documented output | `--help` (15); JSON shapes additive only (4) |
| #647 doctor (merged, PR #701) | the same SHOWN/DRIVEN rule | yes: doctor and the read verbs both call `shown_differs` (3), so they agree on every record |
| #649 DRIVEN writer | a DRIVEN column that prints what it writes, and a SYNC that does not depend on it yet | DRIVEN prints `last_observed.driven` as stored, and SYNC does not compare it (3); whether SYNC later also compares DRIVEN is #649's and ADR-0021 §11's to decide |
| #646 gate at `send_prompt` | the same rule | `shown_differs` (its doc names that gate) |

## Contradictions found

- **C1, rigor.** The issue's `## Pipeline` says `rigor: in-session`, and the epic comment of 2026-10-08 set every story to
  in-session. This brief is `second-opinion` because the orchestrator ordered it for this run; nothing in the design
  depends on which.
- **C2, blast radius.** The issue lists only `crates/holler-cli/src/pane/list.rs, get.rs, watch.rs`. The epic's ruling 2
  gives the same story the ADR 0003 rows, the fixture lines and `tests/pane_verbs/<verb>.rs`, and
  `process/stub.rs:12-16` tells the verb story to delete its `STUBS` rows; without that deletion
  `stub_verb_not_implemented` fails (the filled verbs answer `not implemented` with no story number over `Unwired`).
  This brief includes all of them and `CHANGELOG.md`, and nothing else.
- **C3, no `ASSUMPTION(#643)` in the test kit.** The orchestrator expected some; there are none. The fake rules the
  tests depend on are the documented ones quoted in Evidence (the feed's rules 1-3, `FakeProfileScope::resolve`). One of
  them is the fake's own and not pinned by the conformance suite: `resolve(P, Some(n))` for a pane **with no record** is
  `pane-not-in-profile` in the fake; the suite pins only panes with a record (in another profile, or in none). The tests
  therefore assert `pane-not-in-profile` only for panes that have a record (AC 6).
- **C4, formatting gate.** The orchestrator asked for `cargo fmt --check`. On `3bdd129`, `cargo fmt --all --check` and
  `cargo fmt -p holler-cli --check` both fail on files this story does not touch (for example
  `crates/holler-cli/src/cli.rs`, `crates/holler-body/src/acp_driver/auth.rs`). The epic's ruling 4 reads "`cargo fmt`" as
  "new files pass `rustfmt --check --edition 2021`; existing files are not reformatted". AC 22 uses that form on every
  touched `.rs` file (all seven pass it today).
- **C5, I5 and `watch`.** I5 says a verb returns in bounded time; the issue says `watch` follows the change feed, which
  does not end by itself. Resolved by Decision 16 (every port call is bounded; `--until-idle` gives scripts a bounded
  run); no ADR edit.
- **C6, "prints each change once" versus the port.** From `Cursor(0)` the port yields the current state, not the history
  before it (`ports.rs:38-40`, `feed.rs:22-25`), so earlier changes to a record collapse into its one current put. "Each
  change once" is therefore "every change the feed owes, once, no gap, no repeat", and `--since` is what reaches the
  history (AC 12, 13).
- **C7, the production binary.** Until #649, `Wiring::connect` hands out `Unwired`, so the real `holler pane list` prints
  `error: not implemented` (exit 1). That is correct for this wave and not fixed here.
- **C8, "SHOWN differing from DRIVEN" versus the rule on `main` (amendment 1).** The issue and the epic speak of a
  SHOWN/DRIVEN mismatch, and round 1 of this brief compared `last_observed.shown` with `last_observed.driven`. #647 then
  merged `shown_differs` (SHOWN against `session_of_record`), and ADR-0021 §11 now says that until #649 the mismatch is
  that comparison, because I2 makes the session of record the session the hub drives. Two public rules would make
  `pane doctor` and the read verbs disagree about the same record (A-dup's D-1), and the round-1 rule could never flag
  a real pane, since nothing writes `driven`. Resolved by the MO's ruling (a1), Decision 3. The issue's "SHOWN and
  DRIVEN come from the record's `last_observed`" still holds: both columns print from it.

## Out of scope

- Wiring the hub client (`wiring.rs`, #649); the real `ProfileScope` (#663); the roster (#648); reconcile and doctor
  (#647), which write `last_observed` and health; any change to `output.rs` (#660), `args.rs`, `pane/mod.rs`,
  `main.rs`, `tests/pane_verbs/main.rs`, any manifest, ADR-0021 or the test kit.
- Observing anything live: no Herdr, tmux, harness or probe call.
- Operator documentation beyond `--help` and the ADR 0003 rows: `docs/pane-control.md` (it does not exist yet) is #652's.
  `README.md` mentions no `holler pane` verb, so it has nothing to update.
- A verb-side cursor dedupe (Decision 8), reconnecting `watch` after an error, a heartbeat line on idle, a local-time
  formatter, colour.
- Touching any live fleet, Herdr session, tmux server or OpenCode server: every test is in-process over the fakes.

## Test plan

**RED** (a compile error or a parse panic is not RED, `docs/agent-overlays/tester.md`). T first lands the **surface**, with
no behaviour: the three `Args` structs gain exactly the fields of Decision 1 (doc comments one placeholder line each),
`run` stays the stub that answers `not_implemented(STORY)`, and T updates the ADR 0003 rows (AC 20), the fixture block
(Decision 13) and the three `STUBS` rows. Then `cargo test -p holler-cli --test cli_surface_test --test docs_cli_test
--test pane_cli_process` pass, and T writes the three test files. Every new in-process test then parses and fails on an
assertion about the missing behaviour, for example:
- AC 1: `assert_eq!(run.code, 0)` fails with `1`, because the stub answers `not-implemented`; its `err` is
  `error: not implemented (story #643)`.
- AC 2/12: `check_envelope(out, 0)` / `check_ndjson(out, 0)` fails with `OkDisagreesWithExit` (the stub's envelope has
  `ok: false`).
- AC 6/9/10/11/15: the expected exit 3, 2 or 1 with code `profile-not-found`, `pane-not-in-profile`, `pane-not-found`,
  `usage`, `unavailable` or `timeout` fails: every case answers `not-implemented` (exit 1); the exit-1 cases fail on the
  code.
- AC 17 holds at RED (the stub calls nothing), so T journals it as "holds vacuously at RED; GREEN must keep it"; it is
  meaningful only once the verbs read the stores, which AC 1 proves.
- AC 19: the help substrings (`SYNC`, `"panes"`, `NDJSON`, ...) are missing from the placeholder doc comments.
T journals the RED run with each failing assertion.

**RED for amendment 1** (the fresh run after the ruling). The surface and the verbs exist, and every AC but 3, 19 and
23 already holds and must keep holding. T re-authors AC 3's three tests and fixture helper and adds AC 19's new
substrings, then confirms RED against the unchanged production code, which still compares SHOWN with DRIVEN:
- AC 3, c1 (`shown: "ses-b"`, `driven: None`, `at > 0`, record `ses-a`): expected `MISMATCH` / `"mismatch"`, the code
  gives `-` / `"unobserved"`;
- AC 3, c2 (`shown` = record, `driven: None`): expected `ok`, the code gives `-`;
- AC 3, c3 (observed home screen): expected `MISMATCH`, the code gives `-`;
- AC 3, c6 (`driven: "ses-b"`, `shown: "ses-a"` = record): expected `ok`, the code gives `MISMATCH`;
- AC 19: `session of record`, `home screen` and `#649` are missing from `pane list --help`, and `pane get` from
  `pane watch --help`.
The same three AC 3 cases fail in `get_flags_a_mismatch` and `watch_flags_a_mismatch`. F then changes
`SessionSync::of`, its two call sites and the docs (Files, "Amendment 1's change"), and the CHANGELOG entry (AC 23).

**GREEN:** F replaces each `run` and adds the views and docs. Then `cargo test -p holler-cli --test pane_verbs` (AC 1-19,
AC 14 five times in-test), `--test pane_cli_process`, `--test cli_surface_test`, `--test docs_cli_test`, then the AC 22
gates. T-green also reruns `cargo test -p holler-cli --test pane_verbs watch` three times to look for a flake in AC 14.

## Risks

- **Terminal injection through text output (security).** Stored strings (a cwd, an adapter's health reason, a session
  id) are printed to a terminal. Without Decision 11 a value holding ESC could recolour or rewrite the screen, and a
  newline could forge a table row or a fake `key: value` line. `text_value` is the one choke point; AC 18 pins it. A
  later cell printed without it reopens the hole. **As built (W-9, point 4):** text mode does not pass Unicode bidi or
  other format characters through: `text_value` and `json_text` escape every character `acts_on_terminal` flags
  (Decisions 6 and 11). JSON mode still writes DEL, the C1 controls and bidi characters raw, because `write_envelope`
  (`output.rs`, #660) is plain `serde_json`; that is A's W-11, raised on #660 and outside this blast radius.
- **Argv rendering (security).** A space-joined `command` that someone pastes into a shell would re-split and expand
  `$(...)`; Decision 6 prints JSON arrays only, and AC 7 pins `command: ["opencode","serve","--port","48100"]`.
- **Secrets in output.** The verbs print only what the registry holds. Environment entries are `EnvVarName`s and cannot
  carry a value (I7). A stored `command` or probe argv is printed verbatim and is not scanned or redacted: by I7 and B2 a
  stored command holds no secret, and redaction by guess would hide real arguments. If an operator ever stores a token
  as an argument, `get` shows it; that is a data-entry fault the launch path (#644) should refuse, recorded as a
  follow-up idea, not fixed here. The tests use no secret-shaped values.
- **Busy loop on a non-blocking store.** `watch` without `--until-idle` relies on the port's `next()` blocking up to the
  long-poll window (ADR-0021 §6). A store that answers idle at once would make it spin. The fake's default idle wait is
  zero, so every test either passes `--until-idle` or sets an idle wait (AC 14).
- **Membership races in `watch --profile`.** The "in P" set starts from `resolve` and is updated per event; a change
  between `resolve` and the first event is still seen, because the stream carries every later record. With
  `--since N`, a delete in the replayed history of a pane that was in P only before the `resolve` is not printed.
  Accepted and documented in `--help`.
- **AC 14 timing.** It waits on the call log, not a sleep; the write either lands while the verb waits (woken by the
  condvar) or just before its poll takes the lock, and in both cases it is printed once. Bounded at 5 s; run 5 times.
- **Shared code in `list.rs`.** `get.rs` and `watch.rs` depend on `list.rs`'s `pub` items; a later edit to `list.rs`
  changes all three verbs. Intended (one copy); the file stays under ~300 lines.
- **Field naming is now public JSON.** `PaneRow` keys, `PaneChange` keys and `get`'s top-level keys become the stable
  surface (ADR-0021 §9: removing or renaming one is breaking). Decisions 4, 5 and 7 fix them; AC 2, 7 and 13 pin them.
- **A false MISMATCH after an unseen first observation (amendment 1; #647's code, not this story's).** `at > 0` does
  not prove the screen was seen. On a pane's first observation, `shown_session` can fail while the health check
  answers; `record` then writes the health, stamps `at` and leaves `shown` as stored, `None` on a fresh record
  (`observe.rs:361-375`). A reader following `shown_differs`'s doc ("provided `last_observed.at > 0`") then reads that
  unseen screen as the home screen and prints MISMATCH, while doctor's own pass skips the rule when the screen is unseen
  (`Screen::Unseen`, `observe.rs:78-84`). Rare, and it fails loud, not silent. Accepted here; follow-up for #647 below.
- **DRIVEN reads `-` on every real pane until #649.** A flagged row shows SHOWN beside a `-`; only `pane get` shows the
  session of record (`session-of-record:`). Every verb answers `not implemented` over `Unwired` until #649 (C7), so no
  user sees this before #649 takes over DRIVEN, and printing the stored value commits #649 to nothing.

## Follow-ups (not in this change)

From A-dup (`docs/handoffs/643/handoff-A-dup.md`) and F's round-3 handoff. None blocks this story; each needs a file
outside this blast radius or another story.
- **D-2, one terminal-text rule for both crates.** `text_value` (this story) and `holler_pane::findings::quoted` (#647)
  quote untrusted strings two ways, so after merge `pane doctor` prints a session id as `"ses-a"` and `pane get` as
  `ses-a`. Decided non-fold here (Reuse map). The single rule is a `holler-pane` helper with two faces, added
  amend-first.
- **D-3, one `pane_verbs` test rig.** This story's `Rig` (`tests/pane_verbs/list.rs`) and #647's doctor rig
  (`tests/pane_verbs/doctor/rig.rs`, private) share a binary. One rig with a store-only and a live-world constructor,
  through the frozen-file amend channel.
- **D-4, the `--profile` guard.** Keep `pane::list::profile_name` as the one guard; move it onto `ProfileOpt` in
  `args.rs` (frozen) through the amend channel and have `doctor` call it.
- **D-5, `verb_harness::parse::try_parse`.** T uses it in `tests/pane_verbs/get.rs` the next time that file changes
  (this run's AC 3 rework is such a change, so T may take it then; it is three lines).
- **D-6, `wait_for` in `verb_harness`.** Move `support::wait_for` into `verb_harness` so in-process tests share one
  bounded wait.
- **#647, the unseen first observation** (Risks): stamp `at` only when the screen was seen, or record that it was not.
- **W-5, W-6 and W-11** stay open as A recorded them (the empty `watch --until-idle` stream against `check_ndjson`; I5
  read per port call for `watch`; JSON mode writing DEL, C1 and bidi characters raw, #660).
