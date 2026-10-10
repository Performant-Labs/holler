# Brief: #645a switch-reset (`holler pane switch` and `reset`: the session of record changes by API, the TUI follows)

Repo: Performant-Labs/holler. Issue: #645 (epic #633, wave 3), **part 1 of 2 (645a)**. Rigor: second-opinion. UI surface: no.
Kind: feature.

**Branch:** `issue-645-implementation` (worktree `.claude/worktrees/0645-switch-reset`, from `origin/main` at `ce12cdb`).
**Review rigor:** second-opinion (the orchestrator's instruction for this run; the issue's Pipeline line says `in-session`, see
C-1). The outside reviewer sees only this brief, so every fact below is pasted from the code with its file and line, as of
`ce12cdb`.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see "Forward-compat".
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 3, 8, 9, 11, 12 and "Deferred to named stories". This story
decides one item ADR-0021 defers to it by name (the failure code of a mismatch observed after the act, shared with #644) and
edits ADR-0021 in the same change (Decision 17).
**Handoffs:** `docs/handoffs/645/handoff-<phase>.md`; decision journal `docs/handoffs/645/decisions.md`.
**Public repository:** no personal host, tailnet or account name goes into code, tests, docs, the CHANGELOG, commits or the PR.
Test data uses the test kit's neutral names only (`demo-c1r1`, `demo-c2r1`, profile `demo`, Herdr session `scratch`, ports
48100-48199). No test touches a live fleet, a real Herdr, tmux or OpenCode.

## The split (645a now, 645b later), and why

The issue asks for three things the frozen ports cannot do: refuse a pane that **is not idle**, refuse one that **has a held
question**, and `reset --first TEXT` (**queue the first message** to the new session). `HarnessPort` (Evidence B-2) has no
method that reports whether a session is busy or holds a question, and none that sends a prompt; no other port does either,
and the test kit's `FakeHarness` models none of it (Evidence F-1). The facts live on the hub: its roster row carries the
session's `state` (`idle`/`working`/`input-required`) and its held questions (`pending`), and every prompt passes through one
choke point, `circuit::dispatch::send_prompt`, where holds and the busy check are enforced (Evidence I). A pane's DRIVEN
session is not wired to `session_of_record` until #649 (ADR-0021 section 11, Evidence G-6). So:

- **645a (this brief, one run):** the `tx_switch` engine and both verbs, with every refusal the frozen ports can observe
  (unknown pane, outside the profile, the orchestrator's pane, an unhealthy server, a target the pane's server does not have,
  a target that is another pane's session of record), plan -> act -> observe -> record, and no keystroke. `reset` here has no
  `--first`.
- **645b (later; PROPOSED, operator to confirm, see "PROPOSED"):** the not-idle and held-question refusals and `--first`.
  It needs, first, a contract amendment in its own small story (the #700 precedent, epic decision 8) that gives a verb a way
  to read the activity of a pane's driven session and to queue a prompt **through the hub's one prompt path**, and the
  DRIVEN wiring of #649.

Shipping 645a without the activity refusals is safe before 645b: until #649 the real binary's ports are `Unwired`, whose every
method answers `not-implemented` (Evidence E-6), so no one can run either verb against a live pane. The ordering rule that
closes the window is P2 below (645b before #649's scenario step "reset one with a first message").

## Dependencies (all merged on `origin/main` at `ce12cdb`)

| Story | What 645a uses | Evidence |
|---|---|---|
| #637 (the `holler-pane` crate) | the ports, `Pane`, `PaneError`, `RefusalCode`, the `tx_switch.rs` stub | B, C, D |
| #670, #676 (CLI skeleton, exit 3 for refusals) | `VerbCtx`, `emit`, `ErrorBody`, `ProfileOpt`, the stubs, the fixture, ADR 0003 rows | A, E |
| #638 (the test kit, all slices) | `FakeHarness`, `FakeHerdr`, `FakePaneStore`, `FakeProfileScope`, `check_envelope` | F |
| #647 part 1 (reconcile and doctor, PR #701) | `findings::doctor_command`, `findings::quoted`, `reconcile::shown_differs`, the doctor test rig | D-6, F-6 |

Not needed: #663 (the real `ProfileScope`; the verbs call the trait, tests use the fake), #644 (launch), #642 (the real
OpenCode adapter), #700 (`opencode_agent`; see Forward-compat). The epic's wave table gates #645 on #637 and #638 only.

## Size check

| File | Lines (est.) |
|---|---|
| `crates/holler-pane/src/tx_switch.rs` (engine, codes, session-id guard, failure message) | ~260 |
| `crates/holler-cli/src/pane/switch.rs` (Args, run, the shared `emit_outcome`) | ~130 |
| `crates/holler-cli/src/pane/reset.rs` (Args, run) | ~50 |
| `crates/holler-cli/tests/pane_verbs/switch.rs` (rig include, wrappers, switch cases) | ~480 |
| `crates/holler-cli/tests/pane_verbs/reset.rs` (reset cases) | ~300 |
| `process/stub.rs` (-2), `cli-surface.txt` (~6), `ADR-0003.md` (2), `ADR-0021.md` (~20), `CHANGELOG.md` (~8) | ~36 |
| **Total** | **~1,250** |

One run. No file may reach 900 lines (`scripts/lint.sh` check 4); if `tests/pane_verbs/switch.rs` nears 800, its shared
helpers move to `tests/pane_verbs/switch/support.rs` (a submodule of the verb's own test file, as `doctor/` does).

## Problem

`holler pane switch` and `holler pane reset` are stubs that answer `not-implemented (story #645)`, and
`holler-pane/src/tx_switch.rs` is empty. Nothing yet changes which session a registered pane shows the way epic #633 requires:
by the harness API, with the record and the TUI changed together, observed before anything is recorded, and never by typing
into the TUI (I4). `pane doctor` (#647, merged) already names `holler pane reset <pane>` as the remedy for a pane with no
session of record and for one whose session of record was deleted, so that remedy runs a stub today.

## Contradictions found (each resolved under "Decisions")

- **C-1** The issue's Pipeline line says `rigor: in-session`; the orchestrator runs it at `second-opinion`. The higher rigor
  applies.
- **C-2** The issue's refusals of a pane that "is not idle" or "has a held question", and `--first`, have no port in the frozen
  contract (B-2, F-1, I). Resolved by the split above (Decision 1).
- **C-3** "never targets the orchestrator's own pane unless asked as the operator": the CLI has no operator identity. Resolved
  as an explicit intent flag, `--as-operator` (Decision 4), the same "deliberate act" rule doctor's `--fix` applies (D-7).
- **C-4** ADR-0021 defers "which closed failure code a mismatch observed after `act` carries" to #644 and #645 (G-7). Decided
  here: `unavailable` (Decision 9).
- **C-5** ADR-0021 section 4's I2 test idea compares the driven session too (G-2), but no port observes DRIVEN before #649
  (G-6, D-7). 645a checks SHOWN against `session_of_record` and leaves `last_observed.driven` as stored, as reconcile does.
- **C-6** The issue's blast radius names three files; ruling 2 also gives the verb story its own tests, its ADR 0003 row, its
  `cli-surface.txt` lines and its `stub.rs` entries (E-4, E-5, E-7), and the ADR rule adds ADR-0021 and the CHANGELOG.

## Evidence (verbatim, as of `ce12cdb`)

### A. The stubs this story fills

```
crates/holler-pane/src/tx_switch.rs:1-2
//! The switch/reset transaction (the session of record changes, the TUI follows). Empty
//! stub declared by #637 so that no two stories edit `lib.rs`; story #645 fills it.
crates/holler-pane/src/lib.rs:56
pub mod tx_switch;
```
```
crates/holler-cli/src/pane/switch.rs:1-23
//! `holler pane switch`: a stub (story #670). Story #645 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 645;

/// Switch the session a pane shows and the hub drives.
#[derive(Args, Debug)]
pub struct PaneSwitch {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane switch`: refuse, naming the story that owns it.
pub fn run(_args: &PaneSwitch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
`crates/holler-cli/src/pane/reset.rs:1-23` is the same with `PaneReset`, doc "Start a pane on a fresh session." and
"Run `holler pane reset`". The frozen dispatcher already routes both:
```
crates/holler-cli/src/pane/mod.rs:42-43
    Switch(switch::PaneSwitch),
    Reset(reset::PaneReset),
crates/holler-cli/src/pane/mod.rs:59-60
        PaneCmd::Switch(args) => switch::run(args, ctx),
        PaneCmd::Reset(args) => reset::run(args, ctx),
```
```
crates/holler-cli/tests/pane_verbs/switch.rs:1-9
//! `holler pane switch`: the stub case of story #670. Story #645 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn pane_switch_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "switch"], 645);
}
```
(`tests/pane_verbs/reset.rs:1-9` is the same for `reset`.)

### B. The ports (frozen by #637; not edited here)

**B-1** `crates/holler-pane/src/ports.rs:62-82`:
```
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;

    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
    ...
}
```
**B-2** `crates/holler-pane/src/ports.rs:176-200` (the whole harness port; there is no busy, question or prompt method):
```
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
**B-3** `crates/holler-pane/src/ports.rs:126-135` (the only keystroke methods in the contract; I4 forbids them here):
```
pub trait HerdrPort: Send + Sync {
    ...
    /// Type `text` into the pane.
    fn send_text(&self, pane: &PaneId, text: &str) -> Result<(), PaneError>;

    /// Press `keys` in the pane.
    fn send_keys(&self, pane: &PaneId, keys: &[Key]) -> Result<(), PaneError>;
```
**B-4** `crates/holler-pane/src/ports.rs:226-235`:
```
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
**B-5** `crates/holler-pane/src/profile.rs:268-273, 380-389`:
```
/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}
...
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

### C. The record (`crates/holler-pane/src/pane.rs`, frozen)

```
crates/holler-pane/src/pane.rs:131-158
pub enum Health {
    Healthy,
    /// Unhealthy, with the reason.
    Unhealthy(String),
    Unknown,
}
...
pub struct HarnessInfo {
    pub kind: HarnessKind,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub health: Health,
}
...
pub enum PaneRole {
    Agent,
    Orchestrator,
}
```
```
crates/holler-pane/src/pane.rs:160-164
/// A pane's hold state **as a field of the pane record**: whether `park`/`unpark`
/// has parked it, or it is drained. It is **not** the prompt hold of `holler hold`
/// (`holler_proto::SessionHold`). Any refusal of a prompt that is derived from pane
/// state belongs at `send_prompt`, the one choke point every prompt passes through,
/// not in a verb (#646's brief states this).
```
```
crates/holler-pane/src/pane.rs:183-190
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
crates/holler-pane/src/pane.rs:227-239
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
```
`HerdrPane` (`pane.rs:95-100`) has `pub pane_id: PaneId`, the id every `HarnessPort` TUI method takes. A pane name is a
`SessionName` (`pane.rs:31-42`), whose grammar is shell-safe:
```
crates/holler-proto/src/vocab.rs:209-223
    if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
        return Err(NameError::BadCharacter);
    }
    for &c in seg {
        if !(is_word(c) || c == b'-') {
            return Err(NameError::BadCharacter);
        }
    }
...
fn is_word(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z')
}
```

### D. Codes, output and the helpers reused

**D-1** Open codes (`crates/holler-pane/src/error.rs:314-337`):
```
    /// A code declared as a constant, checked when the constant is evaluated:
    /// a literal that is not kebab-case, or is a closed code, fails the build.
    /// A verb or an adapter declares the code it owns with
    /// `const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");`,
    /// so the call site needs no `Result`.
    ...
    pub const fn from_static(code: &'static str) -> Self {
```
and the variant `Refused { code: RefusalCode, message: String }` (`error.rs:491`), displayed as its message alone
(`error.rs:679`: `PaneError::Refused { message, .. } => f.write_str(message),`). The closed variants this story raises:
```
crates/holler-pane/src/error.rs:651-653, 675, 677
            PaneError::Conflict => f.write_str(
                "the record changed since it was read (generation conflict); read it again and retry",
            ),
            PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
            PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
```
`PaneError::PaneNotFound { what }`, `PaneError::Usage { message }` and `PaneError::Timeout { op }` are also closed variants
(`error.rs:409, 456, 458`).

**D-2** Exit classes: `class_of` (`error.rs:266-273`) makes any well-formed open code a refusal:
```
pub fn class_of(code: &str) -> ErrorClass {
    let Some(closed) = PaneCode::parse(code) else {
        return if is_valid_code(code) {
            ErrorClass::Refusal
        } else {
            ErrorClass::Failure
        };
    };
```
**D-3** Output (`crates/holler-cli/src/output.rs:125-141, 191-217`):
```
pub struct ErrorBody {
    /// The stable code a script matches on.
    pub code: ErrorCode,
    /// One line for a person. In JSON mode [`emit`] puts it on one line.
    pub message: String,
}

impl From<&PaneError> for ErrorBody {
    fn from(error: &PaneError) -> Self {
        Self {
            code: ErrorCode::from(error),
            message: error.to_string(),
        }
    }
}
...
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
`ErrorCode::from(&PaneError)` is `output.rs:118-122`. Module docs (`output.rs:4-8`): text mode writes data to `out` and an
error to `err` as `error: <message>`; JSON mode writes exactly one envelope to `out` and nothing to `err`.

**D-4** `--profile` (`crates/holler-cli/src/pane/args.rs:19-25`):
```
/// `--profile NAME`: scope the verb to a profile.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileOpt {
    /// Act on the panes of this profile; a named pane must belong to it.
    #[arg(long, value_name = "NAME")]
    pub profile: Option<String>,
}
```
**D-5** The merged analogue of argument typing (`crates/holler-cli/src/pane/doctor.rs:46-62`):
```
fn pass(args: &PaneDoctor, ports: Ports<'_>) -> Result<Report, PaneError> {
    let pane = args.pane.as_deref().map(PaneName::parse).transpose()?;
    let profile = args
        .profile
        .profile
        .as_deref()
        .map(ProfileName::parse)
        .transpose()?;
    let request = ReconcileRequest {
        profile: profile.as_ref(),
        pane: pane.as_ref(),
        fix: args.fix,
        now_ms: now_millis(),
    };
    reconcile(ports, &request)
}
```
**D-6** The reconcile step and quoting, already in `holler-pane` for other verbs (`crates/holler-pane/src/findings.rs:302-315,
328-334`):
```
/// The `holler pane doctor` command line for `pane` (every pane when `None`), with `--fix`
/// when `fix`. It is the reconcile step another verb prints after a failure (ADR-0021
/// sections 8 and 12), so a verb builds it here rather than spelling it again.
pub fn doctor_command(pane: Option<&PaneName>, fix: bool) -> String {
...
/// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
/// control character and quote in it is escaped, and cut to 64 characters
/// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
/// included.
pub fn quoted(text: &str) -> String {
```
and the comparison (`crates/holler-pane/src/reconcile.rs:172-181`):
```
/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
/// `shown: None` is the home screen, a mismatch too. ...
/// The one form of this comparison: ...
pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
    session_of_record.is_some_and(|record| shown != Some(record))
}
```
**D-7** What reconcile writes, which this story's gates rely on, and its `--fix` analogue (`reconcile.rs:21-36`):
```
//!    never creates, starts, attaches, aborts or deletes anything, never types into a pane
//!    (I4) and never writes `session_of_record`. The orchestrator's pane is repaired only by
//!    a pass that names it.
//! 4. **Record** what was observed, in one compare-and-swap of the pane's record.
//!
//! **What the record write means.** Other verbs gate on these fields (#645, #646, #648):
//!
//! - `harness.health` is written from this pass's health check: `healthy`,
//!   `{"unhealthy": "server-wedged"}` or `{"unhealthy": "server-down"}`. A failed check
//!   leaves it as stored.
//! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
//!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
//! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
//!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
//!   the session the hub drives.
```
Its repair is select then observe (`crates/holler-pane/src/reconcile/observe.rs:319-321`, private to reconcile):
```
    let pane_id = &pane.herdr.pane_id;
    let selected = ports.harness.select_session(pane_id, record);
    let after = ports.harness.shown_session(pane_id);
```
**D-8** Doctor already prints `reset` as a remedy (`crates/holler-cli/tests/pane_verbs/doctor.rs:272-276`, AC 6 of #647):
```
    let missing = one(&report, K::SessionOfRecordMissing, PANE);
    assert_eq!(
        missing.remedy.as_deref(),
        Some("holler pane reset demo-c1r1")
    );
```

### E. The CLI surface pieces this story edits (ruling 2)

**E-1** `crates/holler-cli/tests/fixtures/cli-surface.txt:125-129`:
```
# #645
pane switch |
pane switch | --profile demo
pane reset |
pane reset | --profile demo --format=json
```
and its header (`cli-surface.txt:4-5, 10`): "Every line must parse with `Cli::try_parse_from`, and every leaf verb clap knows must
appear on at least one line" ... "Cover every flag at least once."
**E-2** `docs/adr/ADR-0003.md:51-52`:
```
holler pane switch [--profile NAME]                               #645
holler pane reset [--profile NAME]                                #645
```
**E-3** `docs/adr/ADR-0003.md:92` (excerpt): "... each owning story adds its verb's positionals and flags to its own row, with
its own line in `cli-surface.txt`, and edits no other verb's."
**E-4** `crates/holler-cli/tests/pane_verbs/process/stub.rs:12-27` (excerpt):
```
/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
/// line**: ...
pub const STUBS: &[(&str, &str, u32)] = &[
    ...
    // #645
    ("pane", "switch", 645),
    ("pane", "reset", 645),
```
**E-5** The flag matrix tolerates required positionals (`crates/holler-cli/tests/pane_verbs/process/flags.rs:6-7`): "A flag
is *accepted* when the parse succeeds or only a required positional is missing (the sibling stories add those)".
**E-6** The real binary's ports until #649 (`crates/holler-cli/src/pane/wiring.rs:8-9`):
```
//! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
//! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
```
**E-7** `docs_cli_test` parses every `holler ...` command in `README.md` and `docs/**/*.md` except `docs/handoffs/`
(`crates/holler-cli/tests/docs_cli_test.rs:5, 37, 54`), so the ADR edits below use no inline `holler ...` code span except the
ADR 0003 rows, which must parse.

### F. The test kit (`holler-pane-testkit`, a dev-dependency of `holler-cli`)

**F-1** `FakeHarness` semantics (`crates/holler-pane-testkit/src/harness.rs:122-137`):
```
/// - `create_session(port)` and `list_sessions(port)` reach the port, then mint an id
///   (`ses_` and 26 hex digits, 30 characters; treat it as opaque) in the port's data
///   directory, or list that directory's sessions in creation order.
/// ...
/// - `select_session(pane, session)`: a pane with no TUI is `unavailable` ("no TUI in
///   pane P"), checked first because the TUI is what names the server. It then reaches
///   the TUI's port. A session outside that port's data directory is
///   `session-not-found`, and the screen stays; otherwise that TUI, and no other, shows
///   the session.
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: ...
```
`health` is `Ok(self.lock().state(port) == Some(ServerState::Running))` (`harness.rs:287-290`): `false` for a frozen, killed
or never-served port. Scenario and inspection methods (they bypass the faults and the call log): `seed_session(port) -> String`
(`harness.rs:218-223`), `delete_session(&str)` ("every TUI showing it goes to its home screen", `harness.rs:225-231`),
`freeze(port)` / `kill(port)` (`harness.rs:199-216`), `close_tui(&PaneId)` (`harness.rs:245-252`), `tui(&PaneId) ->
Option<TuiView { port, shown }>` (`harness.rs:260-263`), `set_quirk(Quirk, bool)` with
```
crates/holler-pane-testkit/src/harness.rs:62-65
pub enum Quirk {
    /// `select_session` on a pane with no TUI answers `Ok(())` and changes nothing
    /// (opencode-pane-spike.md:122-124).
    SelectAckedWithoutTui,
```
The fake has no busy state, no question state and no prompt method: its `HarnessOp` enum is exactly `Serve, Health,
CreateSession, ListSessions, Abort, AttachTui, SelectSession, ShownSession` (`harness.rs:33-42`).
**F-2** Every keystroke is logged (`crates/holler-pane-testkit/src/herdr.rs:55-63`):
```
pub enum HerdrOp {
    EnsurePane,
    SendText,
    SendKeys,
    Read,
    Close,
    Snapshot,
    Version,
}
```
**F-3** Faults and the call log (`crates/holler-pane-testkit/src/fault.rs:72-77, 85-88`):
```
    /// Fail the next call of `op` with `error`, once. ...
    pub fn fail_next(&self, op: Op, error: PaneError) {
...
    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
```
`PaneStoreOp` is `Get, List, CasPut, Delete, Watch, WatchNext` (`pane_store.rs:23-31`). Another writer:
```
crates/holler-pane-testkit/src/pane_store.rs:113-116
    /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
    /// at 1 for a new record), without the membership rule, and publishes its event.
    /// It bypasses the faults and the call log. Returns the stored record.
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
```
**F-4** `FakeProfileScope::resolve` (`crates/holler-pane-testkit/src/profile_scope.rs:33-37`): "`resolve(P, Some(n))` is P and
n, whose record must name P, or else `pane-not-in-profile` (a pane with no record included). A missing P is
`profile-not-found`, checked first."
**F-5** The envelope helper (`crates/holler-pane-testkit/src/envelope.rs:197-202`): "Checks that `stdout` is exactly one
envelope that agrees with `exit_code`, the exit code of the verb that wrote it." `pub fn check_envelope(stdout: &str,
exit_code: i32) -> Result<Envelope, EnvelopeFault>`.
**F-6** The doctor rig (#647), a live world on the fakes (`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:1-8`):
```
//! The doctor rig (#647, brief Decision 12): a live world on the test kit's fakes, with one
//! record per pane that matches it.
//!
//! For each seeded pane it makes a Herdr pane (`ensure_pane`), a tmux session
//! (`ensure_session`), a harness server on its own port (`serve`, 48100 + index), a
//! session on it (`create_session`) and a TUI showing that session (`attach_tui`), then
//! stores a `sample_pane` naming all of them, with that session as its session of record.
//! A freshly built rig is a healthy fleet: doctor reports nothing about it.
```
Its API, used as is: `Seed::new(name, row, col)`, `.orchestrator()`, `.in_profile(p)`, `.data_dir(d)` (`rig.rs:62-85`);
`Rig::new(&[Seed])`, `ports()`, `ports_with(&dyn HarnessPort)`, `live(name) -> &Live { name, herdr, port, session }`,
`pane_id(name)`, `record(name)`, `rewrite(name, change)`, `run(&ReconcileRequest)`, `verb(argv, Format) -> Outcome`,
`mark() -> Calls`, `calls_since(&Calls) -> Calls` with `Calls { panes, profiles, herdr, host, harness, probes }`
(`rig.rs:99-273`); `whole(fix)`, `kinds(report)`, `one(report, kind, pane)` (`rig.rs:282, 307, 317`). The module is private to
`doctor.rs` (`doctor.rs:10`: `mod rig;`). Rig panes share the fake's default data directory unless `.data_dir` is given.
**F-7** The in-process runner (`crates/holler-cli/tests/verb_harness/mod.rs:53-54`): `pub fn run_verb_with(argv: &[&str],
format: Format, ports: Ports<'_>) -> Outcome`; `Outcome { code, out, err }` (`mod.rs:40-44`); `try_parse` is
`crate::verb_harness::parse::try_parse` (`parse.rs:24`).

### G. ADR-0021 (the standing spec; line numbers as of `ce12cdb`)

**G-1** Scoping (`ADR-0021.md:126-129`):
```
- **Scoping** (`list`, `get`, `watch`, `doctor`, `switch`, `reset`, `park`, `unpark`, the `--pane` forms of `say`, `interrupt`
  and `answer`, and the roster): act on P's panes. A named pane outside P is `pane-not-in-profile`. The read verbs and
  `doctor`, `park` and `unpark` with no pane name mean every pane of P; `say`, `interrupt`, `answer`, `switch` and `reset`
  still need a pane name.
```
**G-2** Invariants (`ADR-0021.md:162-164`):
```
| I2 | `session_of_record` is the only session a pane's TUI shows and the hub drives. | After each of launch, relaunch, switch and reset on the fakes, the fake TUI's shown session, the driven session and `session_of_record` are equal. |
| I3 | Every verb is plan, act, observe, record; a mismatch fails loudly and records nothing. | Make the fake TUI show a different session after `act`: the verb exits 1 with a code, and the stored record and its generation are unchanged. |
| I4 | No verb changes a session by typing into a TUI. | The fake Herdr records every `send_text`/`send_keys`; the switch and reset tests fail if any keystroke was sent (#645). |
```
**G-3** Crate rule (`ADR-0021.md:179-181`): "`holler-pane` holds types, traits and the pure transaction engines (`tx_launch.rs`
#644, `tx_switch.rs` #645, ...), which work only through the ports. It depends on `serde`, `serde_json` and `holler-proto`, and
takes no async runtime."
**G-4** Generations (`ADR-0021.md:274-276`):
```
- A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  writes nothing more, and prints the reconcile step (the pane doctor command line for that pane).
```
**G-5** Codes (`ADR-0021.md:327-330` and the row this story owns, `ADR-0021.md:340`):
```
is not an error a port returns). No story edits that list (ruling 3). A verb or adapter declares any other code it owns as a
constant in its own file through `RefusalCode::from_static`, which refuses a malformed or closed code at build time, and
raises it as `PaneError::Refused`. **Stable** means a code is never renamed or reused for another meaning once it has
merged; a code whose condition disappears is retired, not recycled. Every message is one line and never echoes a secret.
...
| `pane switch`, `pane reset` | `pane-not-found`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645) for a pane that is not idle, holds a question or has an unhealthy server, and for the orchestrator's own pane |
```
and the preamble (`ADR-0021.md:332`): "Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until
its story lands, `not-implemented`."
**G-6** Section 11 (`ADR-0021.md:450-456`):
```
`HerdrPort::send_text` and `send_keys` exist in the provisional port. Under I4 no verb may use them to change which
session a pane shows or what it is doing; #640 and #646 name their permitted uses, and #645's tests fail on any keystroke.

**Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
registered pane, and when it moves after a switch or reset.
Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub
drives, and leaves `last_observed.driven` as stored.
```
**G-7** Section 12 and the deferral (`ADR-0021.md:460-463, 535-536`):
```
**Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step. A crash between steps leaves state that
the next pane doctor run finds and reports (#644's acceptance). No verb leaves work running after it exits.
...
- Which closed failure code a mismatch observed after `act` carries (I3 says the verb exits 1, and under section 9 an open
  code is a refusal, exit 3): #644 and #645.
```

### H. The OpenCode spike (`docs/research/opencode-pane-spike.md`, #635)

```
:66        GET  /session/status           -> 200 {} when nothing runs; {"ses_…":{"type":"busy"}} while busy
:75-78     **Observed (verified): two servers that share a data directory share one session store.** A session created on the
           second scratch server is readable through the first. The live fleet runs several `opencode serve` processes as one
           user with one data directory, so every server lists every pane's sessions. That is how a "most recently active"
           guess can pick another pane's session.
:122-124   - **Caveat 2: no acknowledgement.** On a server with **no** TUI attached, `select-session` still answers
           `200 true`. A `true` therefore proves nothing about the screen, and Holler must confirm the switch by observing
           the TUI (capability 4).
:237-239   4. **`select_session`:** check `GET /session/:id` (404 means `session-not-found`), call `POST /tui/select-session`,
              then wait (bounded, about 2 s) until `shown_session` equals the id, or fail loudly and record nothing (I3).
              Exactly one TUI per server, so the broadcast reaches only that pane.
:222       | TUI whose session is deleted | verified: it leaves the session within about 110 ms, ... goes to the home screen
           (title `OpenCode`), **stays running**, and creates no replacement session. ...
```
`/session/status` (line 66) is the busy signal 645b could use on the harness side; the spike never ran a model, so no
question state of OpenCode's was observed.

### I. The hub's prompt path and activity state (why 645b goes through the hub; not edited here)

```
crates/holler-hub/src/holds.rs:16-19
//! - **One enforcement point.** [`Holds::check`] is called from exactly one
//!   place that can deliver a prompt, `circuit::dispatch::send_prompt`; the
//!   test `tests/hold_single_path_test.rs` (holler-cli) fails if a second
//!   `session/prompt` sender appears in the hub.
crates/holler-hub/src/talk.rs:212-215
    // Issue #442: a held session refuses new work before anything else is
    // done for it (no turn id moved, no TalkLog line). This is a fast path
    // only: the enforcement that cannot be raced is the check in
    // `circuit::dispatch::send_prompt`, which every prompt passes through.
crates/holler-hub/src/roster.rs:145-148, 166-168
    /// The A2A session state (`idle` / `working` / `input-required`) — the
    /// *stored* value. [`Roster::rows`] displays `stalled` in its place when a
    /// `working` row's `last_update_at` has aged past the stall threshold.
    pub state: String,
...
    /// The held permission(s)/elicitation(s) (issue #151) — the `PENDING`
    /// column; `--json` carries the full array.
    pub pending: Option<Vec<PendingItem>>,
```

### J. The issue (#645, verbatim excerpts)

> `switch PANE SESSION`: point the pane at an existing session (TUI and registry together). `reset PANE [--first TEXT]`: create
> a fresh session of record through the API, switch the TUI to it, record it, then queue the first message to it, all in one
> transaction. Refuses a pane that is not idle, has a held question, or whose server is unhealthy (as pfleet epic #266 story 271
> demands of reset), and never targets the orchestrator's own pane unless asked as the operator.
>
> (amended 2026-10-08, profiles) `--profile P` is a **scoping** flag here: both verbs still need a pane name and refuse one
> outside P (`pane-not-in-profile`), through the `ProfileScope` trait (#637). Both take `--format=text|json` via
> `output::emit()`; every refusal has a stable code. Tests use only #638's fakes and envelope helper; no new dependency.
>
> Acceptance: No call types into a TUI (the fake records any keystroke and the test fails if one occurs). Refusals are named; a
> switch to a deleted session fails and changes nothing; the first message lands in the new session and nowhere else (the
> "order ran in the wrong session" incident, as a test). A pane outside P is refused and nothing changes; every refusal and
> success passes the envelope helper; exit codes equal across formats.

## The public API (fixed here, so T can write RED tests against it)

`crates/holler-pane/src/tx_switch.rs` (reached as `holler_pane::tx_switch::...`; `lib.rs` already declares it and is not
edited). Names, fields and signatures are binding; private helpers are F's choice. No new dependency, no I/O of its own.

```rust
use crate::error::RefusalCode;
use crate::{Pane, PaneError, PaneName, Ports, ProfileName};

/// `orchestrator-pane`: the pane's role is `orchestrator` and the caller did not pass `--as-operator`. Refusal, exit 3.
pub const ORCHESTRATOR_PANE: RefusalCode = RefusalCode::from_static("orchestrator-pane");
/// `server-unhealthy`: the pane's harness server does not answer `health`. Refusal, exit 3.
pub const SERVER_UNHEALTHY: RefusalCode = RefusalCode::from_static("server-unhealthy");
/// `session-of-other-pane`: the switch target is another pane's session of record. Refusal, exit 3.
pub const SESSION_OF_OTHER_PANE: RefusalCode = RefusalCode::from_static("session-of-other-pane");
/// The longest session id `parse_session_id` accepts.
pub const SESSION_ID_MAX: usize = 64;

/// A harness session id typed by a person: 1..=SESSION_ID_MAX ASCII letters, digits, `_` or `-`. Anything else is
/// `usage`, and the message quotes the text with `findings::quoted`.
pub fn parse_session_id(text: &str) -> Result<String, PaneError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `switch`: a session the pane's server already has.
    Existing(String),
    /// `reset`: a session this run creates.
    Fresh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchRequest {
    pub pane: PaneName,
    pub profile: Option<ProfileName>,
    pub target: Target,
    pub as_operator: bool,
    /// Milliseconds since the Unix epoch, written as `last_observed.at`.
    pub now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switched {
    /// The record as `cas_put` stored it.
    pub pane: Pane,
    /// The session of record before the run.
    pub previous: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchFailure {
    pub error: PaneError,
    /// `select_session` was called (the TUI may have moved): the message carries the reconcile step.
    pub acted: bool,
    /// The session `reset` created and did not record (it stays on the server).
    pub created: Option<String>,
}
impl From<PaneError> for SwitchFailure { /* acted: false, created: None */ }
impl SwitchFailure {
    /// `error`'s text, then `; session <quoted id> was created and is not recorded` when `created`, then
    /// `; to reconcile, run <findings::doctor_command(Some(pane), true)>` when `acted`. One line.
    pub fn message(&self, pane: &PaneName) -> String;
}

pub fn switch(ports: Ports<'_>, request: &SwitchRequest) -> Result<Switched, SwitchFailure>;
```

`crates/holler-cli/src/pane/switch.rs` (replaces A):
```rust
/// Point a pane at an existing session: its TUI and its record together.
#[derive(Args, Debug)]
pub struct PaneSwitch {
    /// The pane, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// The harness session to show and record (it must exist on the pane's server).
    #[arg(value_name = "SESSION")]
    pub session: String,
    /// Allow the orchestrator's own pane (the operator's deliberate act).
    #[arg(long)]
    pub as_operator: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}
pub fn run(args: &PaneSwitch, ctx: &mut VerbCtx<'_>) -> i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb { Switch, Reset }
/// Print a run's result through `output::emit` (shared with reset.rs).
pub(crate) fn emit_outcome(ctx: &mut VerbCtx<'_>, verb: Verb, pane: &PaneName,
                           result: Result<Switched, SwitchFailure>) -> i32;
```
`crates/holler-cli/src/pane/reset.rs` (replaces A): `PaneReset { pane: String (positional PANE), as_operator: bool,
profile: ProfileOpt }`, doc "Start a pane on a fresh session: create it, show it, record it.", and `pub fn run(args:
&PaneReset, ctx: &mut VerbCtx<'_>) -> i32`, which reuses `super::switch::{emit_outcome, Verb}` (no copy).

## Behaviour

### The CLI (`switch.rs`, `reset.rs`), before the engine

1. `PaneName::parse(&args.pane)`; `ProfileName::parse` on `--profile`; for `switch`, `tx_switch::parse_session_id(&args.session)`.
   Each failure is `usage` (exit 2) printed through `emit`, before any port call (D-5's pattern).
2. Build `SwitchRequest { pane, profile, target, as_operator: args.as_operator, now_ms: holler_proto::clock::now_millis() }`
   (`Target::Existing(id)` for switch, `Target::Fresh` for reset), call `tx_switch::switch(ctx.ports, &request)`, then
   `emit_outcome`.

### `tx_switch::switch`, in this order

| # | Step | Port call | On failure |
|---|---|---|---|
| P0 | Re-check an `Existing` id | none | `parse_session_id` fails -> `usage` (2) |
| P1 | Read the record (generation `g`) | with `profile: Some(P)`: `scope.resolve(&P, Some(&pane))`, record = `panes[0]`; else `pane_store.get(&pane)` | the error as is (`profile-not-found`, `pane-not-in-profile`: 3); `get` -> `None` is `PaneNotFound { what: pane }` (3). Nothing else is called. |
| P2 | Role | none | `role == Orchestrator && !as_operator` -> `Refused { ORCHESTRATOR_PANE, "demo-c1r1 is the orchestrator's pane; pass --as-operator to change it" }` (3). Nothing else is called. |
| P3 | Server healthy | `harness.health(record.harness.port)` | `Ok(false)` -> `Refused { SERVER_UNHEALTHY, "the harness server of <pane> on port <port> does not answer; run holler pane relaunch <pane>" }` (3); `Err(e)` -> `e` (`timeout`/`unavailable`: 1) |
| P4 | `Existing` only: the server has it | `harness.list_sessions(port)` | target not listed -> `SessionNotFound { what: <quoted target> }` (3); `Err(e)` -> `e` |
| P5 | `Existing` only: nobody else's | `pane_store.list()` | a record `r` with `r.name != pane` and `r.session_of_record == Some(target)` -> `Refused { SESSION_OF_OTHER_PANE, "<quoted target> is the session of record of <r.name>" }` (3), naming the first such record in list order; `Err(e)` -> `e` |
| A1 | `Fresh` only: create | `harness.create_session(port)` -> `target` | `e`, `acted: false`, `created: None` |
| A2 | Switch the TUI | `harness.select_session(&record.herdr.pane_id, &target)` | `e`, `acted: true`, `created` = A1's id for reset |
| O1 | Observe SHOWN | `harness.shown_session(&record.herdr.pane_id)` | `Err(e)` -> `e`; `shown_differs(Some(&target), shown)` -> `Unavailable { what: "the TUI of <pane> shows <quoted shown, or 'its home screen'>, not <quoted target>" }` (1); both with `acted: true`, `created` as A2 |
| R | Record | `pane_store.cas_put(&next, g)` | `e` (`generation-conflict`: 1), `acted: true`, `created` as A2; no retry, no second write |

`next` is the record read at P1, **cloned**, with exactly these fields set: `session_of_record = Some(target)`,
`last_observed.shown = Some(target)`, `last_observed.at = request.now_ms` (`last_observed.driven` kept), and `harness.health =
Health::Healthy` (P3 observed it). Every other field is the read record's, so a field added later (#700) is preserved.
`Switched { pane: <cas_put's answer>, previous: <record.session_of_record at P1> }`.

Steps P0-P5 make no write and no live change: a refusal there leaves the record, its generation and the TUI exactly as they
were. No step calls any `HerdrPort` or `HostPort` method, `serve`, `attach_tui` or `abort`. A failure after A2 is **not
compensated** (no second select): the record still names the previous session, and the printed reconcile step (`holler pane
doctor <pane> --fix`, D-6) selects it again, which is doctor's own repair (D-7).

### Output (`emit_outcome`)

- **Data** (JSON `data`): `{"verb": "switch" | "reset", "pane": <the stored Pane>, "previous": "<id>" | null}`. The pane's grid
  serializes through `GridPos`.
- **Text**, one line on stdout (session ids through `findings::quoted`):
  - `switched demo-c1r1 to session "ses_…" (was "ses_…")`, or `(was none)`;
  - `reset demo-c1r1 to a new session "ses_…" (was "ses_…")`, or `(was none)`.
- **Errors**: `ErrorBody { code: ErrorCode::from(&failure.error), message: failure.message(&pane) }` through `emit`: text mode
  `error: <message>` on stderr; JSON one envelope; the exit code is `class_of`'s in both.

## Acceptance criteria

Tests live in `crates/holler-cli/tests/pane_verbs/switch.rs` and `reset.rs` (target `pane_verbs`) and run with
`cargo test -p holler-cli --test pane_verbs switch` / `... reset`. They use only the test-kit fakes through the doctor rig
(Decision 16), the in-process runner and test-local `HarnessPort` wrappers that delegate to the rig's `FakeHarness`. "Calls"
means `rig.calls_since(&mark)` with `mark` taken just before the verb. `P` is `demo-c1r1`, `Q` is `demo-c2r1`; `S1` is P's
rig session. Every case that runs the verb runs it in **both** formats on fresh rigs and asserts: the same exit code; JSON
`check_envelope(out, code)` is `Ok`, `err` is empty, and `envelope.error.code` is the code named; text `out` is empty on failure
and `err` is one `error: ...` line containing the named code's message.

**switch**
1. `switch_moves_the_tui_and_the_record_together`: `S2 = rig.harness.seed_session(port_of_P)`; `pane switch P S2` exits 0.
   Text `out` is `switched demo-c1r1 to session "S2" (was "S1")\n`. Afterwards `harness.tui(P).shown == Some(S2)` and the record
   has `session_of_record == Some(S2)`, `last_observed.shown == Some(S2)`, `last_observed.driven` as before,
   `harness.health == Healthy`, `generation == before + 1`, and every other field equal to before. JSON `data.verb == "switch"`,
   `data.previous == "S1"`, `data.pane.session_of_record == "S2"`.
2. `switch_and_reset_type_nothing` (I4): on AC 1 and AC 14, `calls.herdr` and `calls.host` are empty (so no `SendText` or
   `SendKeys`); switch's `calls.harness == [Health, ListSessions, SelectSession, ShownSession]` and `calls.panes == [Get, List,
   CasPut]`; reset's `calls.harness == [Health, CreateSession, SelectSession, ShownSession]` and `calls.panes == [Get, CasPut]`.
   Every refusal case below also asserts `calls.herdr` and `calls.host` empty.
3. `switch_to_a_deleted_session_changes_nothing`: seed S2, `delete_session(S2)`; `switch P S2` exits 3 `session-not-found`; no
   `SelectSession` in calls; the record equals the one before (generation included); the TUI still shows S1.
4. `switch_to_another_panes_session_is_refused` (the "order ran in the wrong session" incident): rig P and Q on the shared data
   directory; `switch P <Q's S>` exits 3 `session-of-other-pane`; the message contains `demo-c2r1`; no `SelectSession`; both
   records unchanged; both TUIs unchanged.
5. `switch_refuses_an_unhealthy_server`: after `harness.kill(port)`, and on a fresh rig after `harness.freeze(port)`, `switch P S2`
   exits 3 `server-unhealthy`, with no `ListSessions` or `SelectSession`, record unchanged. With
   `harness.faults().fail_next(Health, Timeout { op: "harness.health" })` it exits 1 `timeout`.
6. `switch_refuses_the_orchestrators_pane_unless_as_operator`: P seeded `.orchestrator()`: `switch P S2` exits 3
   `orchestrator-pane`, `calls.harness` empty, record unchanged; `switch P S2 --as-operator` exits 0 as AC 1.
7. `switch_mismatch_after_select_records_nothing` (I3): `close_tui(P)` and `set_quirk(SelectAckedWithoutTui, true)`; `switch P
   S2` exits 1 `unavailable`; the message contains `its home screen` and `to reconcile, run holler pane doctor demo-c1r1
   --fix`; the record equals the one before (generation included).
8. `switch_select_failure_names_the_reconcile_step`: `fail_next(SelectSession, Timeout { op: "harness.select_session" })`:
   exit 1 `timeout`, message ends with the reconcile step of AC 7, record unchanged.
9. `switch_record_conflict_after_the_act`: a wrapper harness whose `select_session` first calls `rig.panes.concurrent_put` of
   P's record with `hold` changed (another writer), then delegates; run through `rig.ports_with(&wrapper)`: exit 1
   `generation-conflict`, the message ends with the reconcile step; the stored record is the other writer's
   (`session_of_record == Some(S1)`, the changed `hold`); the TUI shows S2.
10. `switch_in_a_profile`: P `.in_profile("demo")`, Q in none: `switch P S2 --profile demo` exits 0; `switch Q <seeded> --profile
    demo` exits 3 `pane-not-in-profile` with `calls.harness` empty; `--profile nope` exits 3 `profile-not-found`; records
    unchanged on both refusals.
11. `switch_unknown_pane`: `switch demo-c9r9 S2` exits 3 `pane-not-found`, `calls.harness` empty.
12. `switch_usage`: `switch BAD_NAME S2`, and each SESSION of `"ses x"`, `""`, `"ses_\u{1b}[31m"` and 65 `a`s, exits 2 `usage`
    with no port call at all (every `calls.*` empty); the message for the escape case contains no raw `\u{1b}`.
13. `switch_to_the_current_session_is_idempotent`: `switch P S1` exits 0, `previous == "S1"`, generation `+1`, TUI shows S1.

**reset**
14. `reset_creates_a_fresh_session_and_switches_to_it`: `pane reset P` exits 0; the new id N is not S1, is in
    `list_sessions(port)`, `tui(P).shown == Some(N)`, the record names N (fields as AC 1), `data.previous == "S1"`; text
    `reset demo-c1r1 to a new session "N" (was "S1")\n`.
15. `reset_is_doctors_remedy` (D-8): (a) with the record's `session_of_record` set to `None` by `rig.rewrite`, and (b) after
    `delete_session(S1)`, `reset P` exits 0 (`previous` null in (a)); afterwards a `rig.run(&whole(false))` report has no
    `no-session-of-record`, `session-of-record-missing` or `shown-driven-mismatch` finding for P. And the remedy string doctor
    prints for (b) (`holler pane reset demo-c1r1`) parses with `try_parse` (split on spaces, `holler` dropped).
16. `reset_leaves_the_old_session_as_a_stray` (Decision 18): after AC 14, `rig.run(&whole(false))` has exactly one finding,
    `stray-session` for S1.
17. `reset_refusals_create_nothing`: orchestrator without `--as-operator` (3 `orchestrator-pane`), killed server (3
    `server-unhealthy`), `--profile demo` for a pane outside it (3 `pane-not-in-profile`), unknown pane (3 `pane-not-found`):
    no `CreateSession` in calls and `list_sessions(port)` unchanged.
18. `reset_failure_after_create_names_the_unrecorded_session`: `fail_next(SelectSession, Unavailable { what: "tui" })`: exit
    1 `unavailable`; the message contains the created id (quoted), `is not recorded`, and the reconcile step; the record is
    unchanged; the created id is in `list_sessions(port)`, and `rig.run(&whole(false))` reports it as a `stray-session`.
19. `reset_mismatch_records_nothing`: as AC 7 for reset: exit 1 `unavailable`, record unchanged, message has the created id.

**Surface and hygiene**
20. `process/stub.rs` no longer lists `("pane", "switch", 645)` or `("pane", "reset", 645)` and keeps `// #645`;
    `cargo test -p holler-cli --test pane_cli_process` passes.
21. `cli-surface.txt`'s `# #645` block is exactly:
    ```
    pane switch | demo-c1r1 ses_0001
    pane switch | demo-c1r1 ses_0001 --profile demo
    pane switch | demo-c1r1 ses_0001 --as-operator --format=json
    pane reset | demo-c1r1
    pane reset | demo-c1r1 --profile demo --format=json
    pane reset | demo-c1r1 --as-operator
    ```
    and `cargo test -p holler-cli --test cli_surface_test --test docs_cli_test` passes.
22. ADR 0003 rows 51-52 read `holler pane switch PANE SESSION [--profile NAME] [--as-operator]` and `holler pane reset PANE
    [--profile NAME] [--as-operator]`, each ending in `#645` in the column the other rows use.
23. `help_names_the_arguments`: `pane switch --help` contains `PANE`, `SESSION` and `--as-operator`; `pane reset --help`
    contains `PANE` and `--as-operator` and not `--first`.
24. ADR-0021 holds exactly the edits of Decision 17: `git diff origin/main -- docs/adr/ADR-0021.md` touches only row 340, the
    end of section 11 (after line 456) and the "Deferred to named stories" list; `grep -c "#645" docs/adr/ADR-0021.md` is
    higher than on `origin/main`; `docs_cli_test` passes.
25. `cargo clippy --workspace --all-targets -- -D warnings` is clean; `rustfmt --check --edition 2021` passes on
    `tx_switch.rs`, `switch.rs`, `reset.rs` and both test files; `bash scripts/lint.sh` passes (every file < 900 lines);
    `cargo test --workspace` passes; no new dependency in any `Cargo.toml` (`git diff origin/main -- '*Cargo.toml'` empty).
26. `CHANGELOG.md` has one `### Enhancements` entry under `[Unreleased]` for 645a ("Part of #645"), and
    `bash scripts/changelog-check.sh` passes.

## Files

**Production:** `crates/holler-pane/src/tx_switch.rs` (fill), `crates/holler-cli/src/pane/switch.rs`, `reset.rs` (replace).
**Tests:** `crates/holler-cli/tests/pane_verbs/switch.rs`, `reset.rs` (replace their stub cases); `process/stub.rs` (delete two
entries). **Docs/fixtures:** `crates/holler-cli/tests/fixtures/cli-surface.txt` (the `# #645` block), `docs/adr/ADR-0003.md`
(rows 51-52), `docs/adr/ADR-0021.md` (Decision 17), `CHANGELOG.md`. Nothing else: no frozen file (`lib.rs`, `ports.rs`,
`pane.rs`, `error.rs`, `mod.rs`, `args.rs`, `output.rs`, `wiring.rs`, `tests/pane_verbs/main.rs`, any `Cargo.toml`), no #647
file (`findings.rs`, `reconcile*.rs`, `doctor.rs`, `doctor/*`).

### Reuse map (extend, do not duplicate)

| Need | Reuse | Not |
|---|---|---|
| The object to extend | the `tx_switch.rs` stub (fill it) and the two verb stubs (replace them); the analogous feature is doctor's `--fix` repair (D-7: select, observe, record) | a new module or crate |
| The reconcile step | `holler_pane::findings::doctor_command(Some(&pane), true)` (D-6, written for exactly this) | a new step string; #663's profile-scoped `reconcile_step` (not on `main`, and profile-shaped) |
| Quoting untrusted ids | `holler_pane::findings::quoted` (D-6) | a new quoting helper |
| The SHOWN comparison | `holler_pane::reconcile::shown_differs` (D-6) | an inline comparison |
| Scope and membership | `ports.scope.resolve` (B-5) | a membership check of its own |
| Output | `output::{emit, ErrorBody, ErrorCode, VerbCtx}` (D-3), `ProfileOpt` (D-4) | a renderer of its own |
| Test world | the doctor rig, included with `#[path = "doctor/rig.rs"] pub(crate) mod rig;` in `switch.rs`; `reset.rs` uses `crate::switch::rig` (Decision 16) | a new rig; editing `doctor.rs` to export it |
| Envelopes, parsing | `check_envelope` (F-5), `run_verb_with` and `try_parse` (F-7) | own JSON checks |
| Clock | `holler_proto::clock::now_millis` (already a `holler-pane` and `holler-cli` dependency) | a new clock |

## Decisions already made (O)

1. **Split into 645a/645b** (C-2), for the reasons under "The split". 645a ships the verbs with every refusal the frozen ports
   can observe and no `--first`; 645b is PROPOSED (P1-P3).
2. **One engine for both verbs.** `reset` is a switch to a session it creates (`Target::Fresh`); the two differ only in A1 and
   in P4-P5, which a fresh id cannot fail.
3. **Plan order** (P1-P5) is cheapest and most local first: the record, the role, then the server, then the target. Every
   refusal precedes any write or live change, so "a refusal changes nothing" holds by construction.
4. **`--as-operator`** (C-3) on both verbs is an intent gate, not authentication, like doctor's rule that only a pass naming
   the orchestrator's pane repairs it (D-7). Without it a pane whose `role` is `orchestrator` is `orchestrator-pane`.
5. **Health is observed live** (`health(port)`, I6), not read from `harness.health`, which may be stale; a successful run
   writes `Healthy` because it just observed it. An `Err` from `health` is passed through as a failure, not turned into a
   refusal.
6. **The target must be listed by the pane's own server** (P4), so a deleted session or a child (subagent) session is
   `session-not-found` before anything moves (AC 3), rather than relying on `select_session` alone.
7. **The target must not be another pane's session of record** (P5): servers that share a data directory list every pane's
   sessions (H, lines 75-78), so P4 alone would let `switch` point P at Q's conversation, which is the wrong-session incident.
   A session no record names (a stray) is allowed: choosing it is the operator's decision, as doctor's design says.
8. **No park gate.** `Hold` is park state, and a prompt refusal derived from pane state belongs at `send_prompt` (C,
   `pane.rs:160-164`); switch and reset send no work. 645b's `--first` will meet park through the hub's prompt path.
9. **A mismatch observed after the act is `unavailable`** (C-4; exit 1). I3 needs a failure and an open code is a refusal
   (D-2); of the closed failure codes, only `unavailable` means "the live state the verb needs is not there". #644's brief
   (on its own branch, not `main`) decides the same for launch and relaunch.
10. **No compensation after the act.** The record still names the previous session, and doctor `--fix` selects it again; a
    second `select_session` from the verb could fail the same way and would hide the first error. ADR-0021 G-4 asks for
    "writes nothing more, and prints the reconcile step".
11. **The record write**: clone-and-set of four fields (Behaviour), one `cas_put` at P1's generation, never retried
    (G-4). `last_observed.driven` is not touched (C-5, G-6).
12. **No Herdr or host call at all**, so I4 holds by construction and AC 2 can assert empty logs, a stronger check than "no
    `SendText`/`SendKeys`".
13. **No run budget of its own.** I5 is met by the per-call bound (G-7: "every port call is bounded by I5 (default 10 s) or ends
    in `timeout`"); a run makes at most seven port calls. A port `timeout` is passed through with the reconcile step when
    `acted`.
14. **`SESSION` grammar** (`parse_session_id`): `[A-Za-z0-9_-]{1,64}`. OpenCode ids are `ses_` plus 26 `[0-9A-Za-z]`
    characters and the fake's are `ses_` plus 26 hex digits, so both pass; whitespace, control characters and shell
    metacharacters never reach a message or a port. Ids from the harness (reset's new id, SHOWN) are quoted, never trusted.
15. **Output** as under "Output": `previous` lets a script undo a switch with a second switch; the stored `Pane` is the
    proof of what was recorded.
16. **The rig is the doctor rig, included by path**, not copied and not exported by editing `doctor.rs` (a #647 file). Two
    compilations of one file in one test crate are harmless (`tests/pane_verbs/main.rs` allows `dead_code`). If #643 or #644
    has put a shared rig on `main` when T starts, T uses the doctor rig anyway (it is the one that builds a live world with
    TUIs) and notes the consolidation as a follow-up.
17. **ADR-0021, edited in this change** (the stack rule; F writes it):
    - (a) **Row 340** (G-5), which today reads "open (#645) for a pane that is not idle, holds a question or has an unhealthy
      server, and for the orchestrator's own pane", becomes: `| `pane switch`, `pane reset` | `pane-not-found`,
      `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645): `orchestrator-pane`,
      `server-unhealthy`, and for `switch` `session-of-other-pane`; **PROPOSED** (645b): a pane that is not idle or holds a
      question |`.
    - (b) **Section 11**, after the sentence ending "and leaves `last_observed.driven` as stored." (line 456), a new paragraph
      headed "**Switch and reset as built (#645).**" stating Decisions 2-13 and 18 in prose: one engine; the plan's refusals;
      act `select_session`; observe `shown_session`; a mismatch is `unavailable` and records nothing; the four fields written
      in one compare-and-swap; no Herdr or host call; no compensation, the reconcile step is the pane doctor command line for
      the pane with `--fix`; a session `reset` created but did not record is named in the message; after a successful `reset`
      the previous session stays on the server and doctor reports it as a stray session. It names commands in prose only, no
      inline `holler ...` span (E-7).
    - (c) **"Deferred to named stories"**, the bullet at lines 535-536 (G-7), becomes "Which closed failure code a mismatch
      observed after `act` carries: decided, `unavailable` (exit 1): #645 for switch and reset (section 11); #644 for launch
      and relaunch." If #644's in-place edit of this bullet is on `main` by then, F merges the two into one bullet naming both
      stories and both sections.
    - (d) **"Deferred to named stories"**, one new bullet: "**PROPOSED (#645, pending the operator):** the refusal of a pane that
      is not idle or holds a question, and `reset --first`: 645b, after a contract amendment gives the verbs a way to read the
      activity of a pane's driven session and to queue a prompt through the hub's one prompt path, and after #649 points
      DRIVEN at `session_of_record`."
    No other ADR-0021 line changes; the I4 row (G-2) already names this story's tests.
18. **After a successful `reset` the previous session stays** on the server: no port deletes a session (B-2), and deleting a
    conversation is not this verb's call. Doctor then reports it as `stray-session` on every pass (AC 16). Recorded as a
    follow-up (F-1), not solved here.
19. **Rigor `second-opinion`** (C-1), the orchestrator's instruction; the issue's `in-session` line is not edited here.

## PROPOSED (operator to confirm; nothing in 645a waits for these)

- **P1. 645b's design.** Route the not-idle and held-question refusals and `--first` through the hub, not the harness API: a
  harness-side `prompt` would be a second prompt path that skips the hold and busy checks the hub enforces at its one choke
  point (I); inside the hub `hold_single_path_test` guards that single path, and a CLI-side sender would sit outside
  what it can see. Concretely, a small contract amendment story first (as #700 was
  for `opencode_agent`): a port (or a `Ports` member) through which a verb reads a pane's driven-session activity (the
  roster's `state` and `pending`) and queues a prompt by pane name; the test kit fakes it and records each queued prompt
  with its session, so "the first message lands in the new session and nowhere else" is a test; #649 wires the real one.
  The alternative (amend `HarnessPort` with a status call over `/session/status`, H line 66, and a prompt call) is weaker:
  it bypasses holds, and OpenCode's question state was never observed by the spike.
- **P2. Order.** #649's scenario includes "reset one with a first message" (issue #649), so 645b must land before that step,
  or 645b's `--first` and refusals become part of #649. Either way, 645b (or #649 with it) must merge before #654 (the live
  cutover), since 645a's `reset` and `switch` do not yet refuse a busy pane.
- **P3. The issue and epic text.** #645's body should say it is split, with 645a's scope; the epic's wave table should list
  645b and its dependencies (the amendment story and #649's DRIVEN wiring). The orchestrator files or edits these; this
  brief edits neither.

## Forward-compat

| Consumer | What it needs from 645a | Satisfied |
|---|---|---|
| #647 doctor (merged) | `holler pane reset <pane>` runs and clears `no-session-of-record` / `session-of-record-missing` | yes (AC 15) |
| 645b | the engine to add a plan step (activity) and a post-record step (queue the prompt) | yes: `SwitchRequest` gains fields additively; the steps sit between P3 and P4 and after R |
| #649 wiring, DRIVEN | a record whose `session_of_record` is the session to drive after a switch or reset | yes; `last_observed.driven` left for #649 |
| #700 `opencode_agent` | a record write that keeps fields it does not know | yes: clone-and-set (Behaviour) |
| #646 `say --pane`, #648 roster | `last_observed.shown` and `at` written after a switch or reset | yes |
| #653 pfleet `o reset` | a scriptable verb: stable codes, JSON envelope, `previous` | yes |
| #644 launch | nothing; two engines with their own failure types (`TxFailure` there carries no `created`) | n/a |

## Out of scope

- `reset --first TEXT`, the not-idle and held-question refusals (645b, P1-P3).
- Deleting or retiring the previous session after `reset` (F-1); aborting a turn (`abort`).
- Attaching or relaunching a TUI that is gone (`select_session` on a pane with no TUI is `unavailable`; relaunch is #644's).
- DRIVEN (`last_observed.driven`) and the hub's driven session (#649); the real adapters and wiring (#640-#642, #649).
- Any change to a frozen file, to the test kit, or to #647's files.

## Follow-ups (the orchestrator files them; none blocks 645a)

- **F-1** Every successful `reset` leaves the previous session as a permanent `stray-session` finding (AC 16). A way to retire
  a superseded session (a recorded list of retired ids, or a harness delete) needs a contract change; doctor stays noisy
  until then.
- **F-2** If a shared verb-test rig lands from #643 or #644, consolidate it with the doctor rig (Decision 16).

## Test plan

**RED first** (T): replace the two stub test files with ACs 1-19 and 23 against the API above; add the `tx_switch.rs` items
as `todo!()`-free stubs that return `Err(PaneError::NotImplemented.into())` so the crate compiles; replace the two CLI files
with the real `Args` structs whose `run` calls the stub engine. Each AC then fails on its assertion: the verbs exit 1
`not-implemented` where 0 or 3 is expected. ACs 20-22 are edited in RED (the stub entries, the fixture block, the ADR 0003
rows) because the new positionals make the old fixture lines fail to parse. **GREEN** (F): fill the engine and the verbs, the
ADR-0021 edits and the CHANGELOG; T then runs `cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test
cli_surface_test --test docs_cli_test`, `cargo test --workspace`, clippy, rustfmt on the new files and `scripts/lint.sh`.

## Security-sensitive spots (for the reviewers)

- `SESSION` is user text that reaches a port and a message: the grammar (Decision 14) runs before any port call (AC 12).
- Ids from the harness and SHOWN are untrusted: every message and text line quotes them with `findings::quoted` (escapes
  control characters, cuts to 64).
- The reconcile step is built from a validated `PaneName` (shell-safe grammar, C) and constant words (D-6), so it is safe
  to paste.
- No keystroke, no shell, no argv joined or split (AC 2).

## Risks

- **R-1 ADR-0021 adjacency.** #644's planned edit of row 339 is the line above row 340, and both stories edit the "Deferred"
  bullet at 535-536; whichever merges second resolves a small text conflict (Decision 17(c) says how). #642 part b also
  plans an ADR-0021 edit; F re-reads the merged ADR at GREEN and edits by anchor text, not by line number.
- **R-2 A real adapter differs from the fake.** The real `select_session` waits (bounded) for the title to confirm (H, lines
  237-239), so O1 is a second confirmation; if the real `shown_session` can answer "unknown", it is a mismatch here
  (`unavailable`), which fails safe.
- **R-3 The window before 645b.** Once #649 wires real ports, 645a's verbs could switch a busy pane. P2 is the mitigation.
- **R-4 `stub.rs` and the fixture are shared files** edited by every verb story; the `// #NNN` line rule (E-4) keeps the
  conflicts away, and the fixture block is this story's own.
