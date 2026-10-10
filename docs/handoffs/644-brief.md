# Brief: #644 launch-relaunch (`holler pane launch` and `relaunch`, the transaction that creates the session of record)

Repo: Performant-Labs/holler. Issue: #644 (epic #633, wave 3, on the critical path #637 -> #638 -> **#644** -> #649).
Rigor: second-opinion. UI surface: no. Kind: feature.

**Depends on #662a and #663, merged to `main` first** (see "Dependencies"): the run starts from a `main` that contains both.
**Branch:** `issue-644-implementation` (worktree `.claude/worktrees/0644-launch-relaunch`; the brief was written on `3bdd129`,
and the branch is rebased onto the `main` that holds #662a and #663 before T starts).
**Amended 2026-10-09 (restart after #662a, #663 and #646a merged):** #662a and #663 are on `main`. #663 merged `reconcile_step` as `pub fn reconcile_step(profile: Option<&ProfileName>) -> String` in `crates/holler-cli/src/pane/profile_scope.rs`, with `reconcile_step(None)` returning `to reconcile, run holler pane doctor`. The `RECONCILE_STEP_UNSCOPED` constant this brief once planned does not exist and is not added: wherever the brief says `reconcile_step(&P)` it now means `reconcile_step(Some(&P))`, and the unscoped step is `reconcile_step(None)`. Section I's signatures are checked against that merged code at RED.
**Review-rigor:** second-opinion (operator instruction for this run; the outside model is deepseek-v4-pro and sees only this
brief). The issue's own Pipeline line says `rigor: in-session`; see Contradiction C-1.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see the table under "Forward-compat".
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 8, 9, 10, 11, 12 and "Deferred to named stories", which name
#644 for two open decisions (the operation id; the code of a mismatch observed after the act). This story **decides the
mismatch code and proposes the operation-id answer** (PROPOSED, pending the operator's confirmation, decision 1), and edits
ADR-0021 in the same change, only in lines no in-flight story edits (decision 20).
**Amended 2026-10-09 after the plan review** (`docs/handoffs/644/handoff-A.md`, BLOCK on four findings, six warns): every
finding is resolved below; the decisions it changed are 1, 5, 10, 11, 15 and 20, and 22-26 are new.
**Handoffs:** `docs/handoffs/644/handoff-<phase>.md`; decision journal `docs/handoffs/644/decisions.md`.
**Public repository:** no personal host, tailnet or account name goes into code, tests, docs, the CHANGELOG, commits or the PR.
Test data uses only `demo-c1r1`/`demo-c2r1` (panes), `demo` (profile), `scratch` (Herdr session), `main` (Herdr
workspace), `/srv/demo` (directory) and ports 48100-48102 (the test kit's scratch range). No test touches a live fleet, a real
Herdr, tmux or OpenCode: every port is a test-kit fake, and the real binary in the process tests still runs over `Unwired`.

## Dependencies (merged to `main` before this run starts)

| Story | What #644 uses from it | Where it is pasted |
|---|---|---|
| **#662a** (profile verbs, pure core + `list`/`show`) | `holler_pane::profile_snapshot::{spec_from_pane, FIXED_PORT_POLICY_PREFIX, fixed_port_policy}`: relaunch's base spec and the one `fixed:` literal | Evidence I-1, I-2 |
| **#663** (the real `ProfileScope` and the probe runner) | `holler_cli::pane::profile_scope::{reconcile_step, StoreScope}`: the one reconcile step, and which errors already carry it | Evidence I-3, I-4 |

Both are being implemented in parallel with this brief and are **not merged** when it is written. Their APIs are pasted
verbatim from their reviewed briefs (Evidence I, with the branch and commit) as the contract, **to be re-verified against the
merged code at RED**: T's first act is to grep the merged `profile_snapshot.rs` and `profile_scope.rs` for each pasted
signature. A name or signature that differs is a stop (`preflight-failed`, reported to the MO with the diff), not something T
or F adapts silently. #644 writes no copy of either API, so the order is strict: #662a and #663 merge, then this run starts.

## Size check

| File | Who | Lines (est.) |
|---|---|---|
| `crates/holler-pane/src/tx_launch.rs` (the engine: plan, the relaunch rules, probe, act, observe, rollback, record; the open codes; `port_of_policy`) | T-red (stubs), F | ~560 |
| `crates/holler-cli/src/pane/launch.rs` (Args, flag-to-spec overlay, output and the reconcile-step rule) | T-red (Args), F | ~260 |
| `crates/holler-cli/src/pane/relaunch.rs` (Args, base selection; reuses launch.rs helpers) | T-red (Args), F | ~120 |
| `crates/holler-cli/tests/pane_verbs/launch.rs` (the rig: fakes, the linked host, hook wrappers, the registry-equals-fakes check; launch cases) | T | ~720 |
| `crates/holler-cli/tests/pane_verbs/relaunch.rs` (relaunch cases; reuses the rig) | T | ~480 |
| `crates/holler-cli/tests/pane_verbs/process/stub.rs` (delete the two `#644` entries, keep the `// #644` line) | T-red | -2 |
| `crates/holler-cli/tests/fixtures/cli-surface.txt` (the `# #644` lines gain the positional; one `--herdr-session` line) | T-red | ~12 |
| `docs/adr/ADR-0003.md` (rows 48-49) | T-red | 2 |
| `docs/adr/ADR-0021.md` (a section 8 note, section 9 row 338, a section 12 PROPOSED note, one "Deferred" bullet in place) | F | ~25 |
| `CHANGELOG.md` (one entry) | F | ~8 |
| **Total** | | **~2,190** (production ~940, tests ~1,200, docs ~45) |

Three production files of one component family (the launch transaction and its two verbs), two test files, and five small
surface or doc edits that the epic assigns to the verb story (ruling 2: "its ADR 0003 row, its `cli-surface.txt` line and its own
`tests/pane_verbs/<verb>.rs`"; `stub.rs` says a verb story deletes its own entries). **Fits one run.** Fallbacks, journalled by
F rather than splitting the story: if `tx_launch.rs` nears 800 lines, F shares the launch and relaunch acts through one step
runner instead of adding a file (the engine's blast radius is that one file); if `tests/pane_verbs/launch.rs` nears 800 lines,
its `rig` module moves to `tests/pane_verbs/launch_rig.rs`, included from `launch.rs` by `#[path]` (still the verb story's own
test code; `tests/pane_verbs/main.rs` is not edited). No file may reach 900 lines (`scripts/lint.sh` check 4). If the plan
review asks for a split anyway, the natural one is **644a** (engine + `launch` without `--profile`) then **644b** (`relaunch` and every `--profile`/`--spec-only` path).

## Problem

`holler pane launch` and `holler pane relaunch` are stubs that answer `not-implemented (story #644)`, and
`holler-pane/src/tx_launch.rs` is empty. Nothing yet creates a pane the way epic #633 requires: Holler ensures the Herdr pane and
the tmux session, starts the harness server, checks it, **creates the session of record through the harness API** (never a
"ping" session, never "the most recent"), attaches the TUI to exactly that session, observes that the TUI shows it, and only then
writes the pane record with one compare-and-swap. A failed step must roll back what this run created and leave the record (and,
with `--profile`, the profile's specs) as it was, with a named error and exit code. With `--profile P` the same verbs also edit
P's spec for the pane as one transaction with the live change (I8), and `--spec-only` edits the spec and changes nothing live.
#644 is the next serial step of the epic's critical path, so #649 (wiring), #664 (`profile apply`) and #667 build on it.

## Contradictions found (the issue against the frozen code and ADR-0021; each is resolved under "Decisions")

- **C-1 Rigor.** The issue's Pipeline line says `rigor: in-session`; this run is `second-opinion` by the operator's instruction.
  The header follows the operator.
- **C-2 Operation id.** The issue's Scope says both verbs "return within the bounded time (invariant I5) with an operation id
  for long work". ADR-0021 section 12 (Evidence G-5) says an operation id "needs two things the frozen contract does not have"
  (an executor that outlives the CLI and a place to record progress; `Pane` refuses unknown fields and `PANE_METHODS` has no
  operation method) and must come "with an amendment to the contract first". Either option (a) or (b) edits frozen files outside
  this blast radius (`holler-pane/src/pane.rs`, `holler-proto/src/methods.rs`, the hub). Decision 1.
- **C-3 "A failed launch leaves P unchanged"** (issue acceptance) against ADR-0021 "Decisions taken" item 1 (Evidence G-8): a
  failed *act* restores P's specs by a second write, so the specs are equal but the generation moves by two. The two agree only
  for failures **before** the profile write. Decision 9 states which failures are which, and the tests assert exactly that.
- **C-4 What `command` runs.** The issue's acceptance says "the fake host records argv, never a shell line", but
  `HarnessPort::serve(name, port)` takes no argv (Evidence B-5): the only port that runs an argv is `HostPort::run`. Neither the
  epic nor ADR-0021 says what a pane's `command` is for. Decision 6.
- **C-5 Model, effort and env "launched with".** No port method carries a model, an effort or environment names
  (`serve(name, port)`, `attach_tui(pane, port, session)`). C1 says these are *recorded* by launch/relaunch; nothing in the frozen
  ports can *apply* them. Decision 7.
- **C-6 The Herdr session.** `HerdrPort::ensure_pane` takes `HerdrSpec { session, workspace, grid }` (Evidence B-2), but neither
  `SpecFlags` nor `ProfileSpec` has a Herdr session. Decision 4.
- **C-7 Port policy.** `ProfileSpec.harness.port_policy` is a free string "How the harness server's port is chosen" (Evidence
  C-6), and `Pane.harness.port` is a `u16`; no grammar exists anywhere. The test kit's sample spec uses `"fixed"` with no port.
  Decision 5.
- **C-8 Herdr pane to tmux session.** No port call connects a Herdr pane's terminal to the pane's tmux session (`ensure_pane`
  makes a pane; `HostPort::ensure_session` makes a tmux session; `attach_tui` respawns a pane of the *tmux* session, per #642's
  brief). The only way within the frozen ports would be `send_text`, which I4 forbids. This is a contract gap for #640/#649,
  not something #644 can close. Decision 13, Risks.
- **C-9 Resolver precondition from #642.** #642's brief (Evidence H-2, not yet on main) says "#644 passes the session name (and
  directory) explicitly from its plan". The frozen `HarnessPort` methods take only `(PaneName, port)` or `(PaneId, ...)`: there is
  no channel to pass them. #644 calls the ports as frozen; #649 must make its resolvers answer before the record exists.
  Forward-compat, Risks.
- **C-10 The crash-then-doctor acceptance.** "A crash between steps (the next `doctor` finds and reports it)": `pane doctor` is
  #647's and still a stub. #644 can prove only its half: no record is written before the last step, and what a crash leaves is
  observable from the fakes (AC 7); the next launch refuses the leftover Herdr pane's cell (`grid-occupied`, decision 12).
  #647's plan reports such a pane as `unregistered-herdr-pane` with remedy "none (no holler verb adopts a pane; `pane import`
  is #650's)" (647-brief line 904), so today **no verb frees that cell**. Follow-up for #647 (Follow-ups).
- **C-11 Relaunch and the session of record.** "`relaunch`: the same after stopping only what the pane owns" read literally
  creates a new session on every relaunch, which discards the conversation; `reset` (#645) is the verb for a new session.
  Decision 8.
- **C-12 `cargo fmt --check`.** The workspace is not rustfmt-clean on `main` (`cargo fmt --all --check` exits 1 on
  `crates/holler-body/src/acp_driver/answerable.rs`, checked at `3bdd129`), and epic ruling 4 says "new `.rs` files pass
  `rustfmt --check --edition 2021`; existing files are not reformatted". The fmt gate is scoped to the touched files (AC 30).
- **C-13 Where the open codes live.** Epic ruling 3 says codes are constants "in each verb's own file". The engine that raises
  them is `holler-pane/src/tx_launch.rs`, which cannot see `holler-cli`. They are declared in `tx_launch.rs` (this story's file)
  and the verbs re-use them.
- **C-14 "Confirm SHOWN equals DRIVEN"** (issue Scope) against the frozen `LastObserved` ("Written by reconcile, never
  inferred", C-4; ADR-0021 section 1 and I6). No port can see the hub's DRIVEN session before #649 wires it (ADR-0021 section
  11, "Deferred to #649 and #654"). Resolution (decision 22): the check is SHOWN == `session_of_record` (I2: the hub drives
  the session of record), the reading #647's plan uses; launch writes `last_observed.driven = None` and relaunch keeps the
  stored value, so the record never claims a DRIVEN nobody observed.
- **C-15 One reconcile step, two owners in the plan.** ADR-0021 section 8 step 6 and section 12 make the reconcile step every
  acting verb's concern, and #663's plan (I-3, decisions 5-8) makes `profile_scope::reconcile_step` the one copy and adds
  that step to the scope's own `profile-conflict`, restore-failure and first-write-timeout errors. A verb that also appended
  its own step would print it twice under the real scope; the fake scope adds none, so tests over the fake alone cannot see
  it. Resolution (decision 15): #644 calls #663's function, appends the step only to an error that does not already carry
  it, and pins that with the real `StoreScope` (AC 16k).
- **C-16 The doctor form.** ADR-0021 asks for "the exact pane doctor command line", but `pane doctor` takes no pane positional
  until #647 lands, and #663's step names `holler pane doctor --profile '<P>'` (I-3, its C9). Resolution (decision 15): with a
  profile, #663's text exactly; without one, `holler pane doctor` (the same ADR 0003 row with its optional group dropped).

## Evidence (verbatim, as of `3bdd129`; the outside reviewer cannot read the repo)

Every excerpt is pasted from the file at the lines named. Labels (A-1, B-2, ...) are what the rest of the brief cites.
Sections A-H are on `main`; section I is from the two dependencies' reviewed briefs on their branches (not yet merged).
Section J was added at the brief review (round 1), pasted from this branch at `d92bf0a`, after #663 merged.

### A. The stubs this story fills

**A-1** The engine file, empty:
```
crates/holler-pane/src/tx_launch.rs:1-2
//! The launch/relaunch transaction (plan, act, observe, record; I3 and I8). Empty
//! stub declared by #637 so that no two stories edit `lib.rs`; story #644 fills it.
```
**A-2** `launch`, a stub (its whole file):
```
crates/holler-cli/src/pane/launch.rs:1-27
//! `holler pane launch`: a stub (story #670). Story #644 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::{ProfileOpt, SpecFlags, SpecOnly};
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 644;

/// Create a pane.
#[derive(Args, Debug)]
pub struct PaneLaunch {
    #[command(flatten)]
    pub spec: SpecFlags,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane launch`: refuse, naming the story that owns it.
pub fn run(_args: &PaneLaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
**A-3** `relaunch`, a stub (its `use` lines and `STORY` constant are as in A-2):
```
crates/holler-cli/src/pane/relaunch.rs:13-27
/// Launch a pane again, replacing its process.
#[derive(Args, Debug)]
pub struct PaneRelaunch {
    #[command(flatten)]
    pub spec: SpecFlags,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane relaunch`: refuse, naming the story that owns it.
pub fn run(_args: &PaneRelaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```

### B. The ports (frozen by #637; `HerdrPort` and `HarnessPort` "provisional" until #640/#642 close them)

**B-1** `PaneStore`:
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
**B-2** What `ensure_pane` takes:
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
```
**B-2a** What `snapshot` returns (each listed pane carries its own `session`, `workspace`, `pane_id` and `grid`, C-1):
```
crates/holler-pane/src/ports.rs:93-98
/// What Herdr reports about every pane it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSnapshot {
    pub panes: Vec<HerdrPane>,
}
```
**B-3** `HerdrPort`:
```
crates/holler-pane/src/ports.rs:125-148
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
```
**B-4** `HostPort`:
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
**B-5** `HarnessPort`:
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
**B-6** `Prober` and the `Ports` bundle a verb holds:
```
crates/holler-pane/src/ports.rs:208-212, 224-235 (excerpt; lines 214-222 are `SystemProber`, which calls the free `run_probe`)
pub trait Prober: Send + Sync {
    /// Run `argv` (never through a shell) and look for every string of `expect` in
    /// its output, giving up after `timeout` (see [`crate::run_probe`]).
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult;
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
```
**B-7** `ProfileScope` (the I8 helper; the verb calls `edit_spec` and never writes the profile itself):
```
crates/holler-pane/src/profile.rs:380-404
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

    /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
    /// transaction (I8): the profile is written with a compare-and-swap on its
    /// generation first, then `act` runs, then the result is recorded; if `act`
    /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
    /// Returns the edited profile. With `profile: None` it runs only `act` and
    /// touches no profile (and returns `None`).
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError>;
}
```

### C. The records

**C-1** `HerdrPane` (also what `ensure_pane` returns):
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
**C-2** `HostInfo`:
```
crates/holler-pane/src/pane.rs:103-115
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
**C-3** `HarnessInfo`:
```
crates/holler-pane/src/pane.rs:142-150
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
**C-4** `LastObserved`:
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
**C-5** `PaneProbe` and `Pane`:
```
crates/holler-pane/src/pane.rs:212-254
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
```
**C-6** `SpecHarness` and `ProfileSpec`:
```
crates/holler-pane/src/profile.rs:164-200
/// The harness side of a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHarness {
    pub kind: HarnessKind,
    /// How the harness server's port is chosen (the `--port-policy` flag).
    pub port_policy: String,
}

/// One pane of a profile: names and non-secret values only (I7).
///
/// [`ProfileSpec::pane`] is plain text, not a [`PaneName`]: a spec may name a pane
/// that does not exist or belongs to another profile (a detached spec, e.g.
/// `profile create --from`). Membership is `Pane.profile` only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSpec {
    pub pane: String,
    pub herdr: SpecHerdr,
    pub host: SpecHost,
    pub harness: SpecHarness,
    pub model: ModelSpec,
    pub role: PaneRole,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
    pub env: Vec<EnvVarName>,
    pub context: ContextCeilings,
    /// The launch command: an argv array, never a shell string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Argv>,
    /// The health probe argv, e.g. `["curl","-s","http://127.0.0.1:8095/v1/models"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Argv>,
    /// Strings the probe output must contain.
    #[serde(default)]
    pub expect: Vec<String>,
}
```
**C-7** `SpecEdit`:
```
crates/holler-pane/src/profile.rs:259-266
/// What a spec edit does to a profile's entry for one pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecEdit {
    /// Set the pane's entry to this spec (boxed: a spec is large).
    Set(Box<ProfileSpec>),
    /// Remove the pane's entry.
    Remove,
}
```

### D. Guards, codes and exit classes

**D-1** `ProbeResult` and the `run_probe` stub (#663 fills it; tests use the fake prober, F-18/F-19):
```
crates/holler-pane/src/probe.rs:15-36
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

/// Run the health probe `argv` (never through a shell) and look for every string of
/// `expect` in its output, giving up after `timeout`.
///
/// **Stub (#637):** the real runner is built by #663. This one always answers
/// [`ProbeResult::Error`], never [`ProbeResult::Ok`].
pub fn run_probe(argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
    let _ = (argv, expect, timeout);
    ProbeResult::Error("the probe runner is not implemented yet (story #663)".to_owned())
}
```
**D-2** `Argv::from_json` (what `--command-json` / `--check-json` go through):
```
crates/holler-pane/src/argv.rs:45-58
    /// Read an argv from JSON text, as the CLI's `--command-json` and `--check-json`
    /// flags need: text that is not JSON is `usage`; JSON that is not an array of
    /// strings (a bare string most of all) is `command-not-argv`.
    pub fn from_json(text: &str) -> Result<Self, PaneError> {
        serde_json::from_str::<Argv>(text).map_err(|e| {
            if e.is_data() {
                PaneError::CommandNotArgv
            } else {
                PaneError::Usage {
                    message: format!("not valid JSON: {e}"),
                }
            }
        })
    }
```
**D-3** `GridPos::parse`:
```
crates/holler-pane/src/grid.rs:42-45 (excerpt; an unreadable or ambiguous text is `GridAmbiguous`, a zero or a number above
65535 is `GridOutOfRange`)
impl GridPos {
    /// Parse `r2c1`, `c1r2` or `2,1` (see the module docs for the exact rules).
    pub fn parse(text: &str) -> Result<GridPos, PaneError> {
```
**D-4** `GridPos` display:
```
crates/holler-pane/src/grid.rs:68-73
impl fmt::Display for GridPos {
    /// The `rRcC` form, row first: `r2c1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}c{}", self.row, self.col)
    }
}
```
**D-5** `class_of`, the one place that decides an exit class (0 ok, 1 failure, 2 usage, 3 refusal):
```
crates/holler-pane/src/error.rs:266-303
pub fn class_of(code: &str) -> ErrorClass {
    let Some(closed) = PaneCode::parse(code) else {
        return if is_valid_code(code) {
            ErrorClass::Refusal
        } else {
            ErrorClass::Failure
        };
    };
    match closed {
        PaneCode::Usage => ErrorClass::Usage,
        // Understood and declined: a guard, a policy or a gate said no, the name is
        // taken, or the request named something that does not exist.
        PaneCode::GridAmbiguous
        | PaneCode::GridOutOfRange
        | PaneCode::CommandNotArgv
        | PaneCode::EnvNameInvalid
        | PaneCode::ProfileSecretRefused
        | PaneCode::ProfileExists
        | PaneCode::ProfileHasLivePanes
        | PaneCode::PaneNotInProfile
        | PaneCode::PaneInOtherProfile
        | PaneCode::ProbeFailed
        | PaneCode::HerdrVersionUnsupported
        | PaneCode::ProfileNotFound
        | PaneCode::PaneNotFound
        | PaneCode::SessionNotFound => ErrorClass::Refusal,
        // Went wrong while doing the work: a race between writers, a bound that ran
        // out, something unreachable or unreadable, live state that disagrees with
        // its spec, or work the verb cannot do yet.
        PaneCode::GenerationConflict
        | PaneCode::ProfileConflict
        | PaneCode::Timeout
        | PaneCode::Unavailable
        | PaneCode::StoreCorrupt
        | PaneCode::NotImplemented
        | PaneCode::ProfileDrift => ErrorClass::Failure,
    }
}
```
**D-6** `RefusalCode`, how a story declares an open code:
```
crates/holler-pane/src/error.rs:305-320, 331 (excerpt; the body asserts the literal is a valid, non-closed code)
/// A code an adapter or a verb owns: well-formed ([`is_valid_code`]) and not one
/// of the closed codes ([`ALL_CODES`]), so each code has exactly one representation.
///
/// The field is private and there is no `From<&str>`, `From<String>` or `Default`:
/// the only ways to get one are [`RefusalCode::from_static`] (for a `const`) and
/// [`RefusalCode::parse`] (for a code read off the wire). A [`PaneError::Refused`]
/// therefore cannot carry an unvalidated code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefusalCode(Cow<'static, str>);

impl RefusalCode {
    /// A code declared as a constant, checked when the constant is evaluated:
    /// a literal that is not kebab-case, or is a closed code, fails the build.
    /// A verb or an adapter declares the code it owns with
    /// `const QUOTA: RefusalCode = RefusalCode::from_static("quota-exceeded");`,
    /// so the call site needs no `Result`.
    pub const fn from_static(code: &'static str) -> Self {
```
**D-7**, **D-8** Two `PaneError` messages the reconcile step is appended to (a `profile-conflict` from #663's scope already
carries the step inside `what`, I-3):
```
crates/holler-pane/src/error.rs:651-653, 655, 673
            PaneError::Conflict => f.write_str(
                "the record changed since it was read (generation conflict); read it again and retry",
            ),
            PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
            PaneError::Timeout { op } => write!(f, "timed out: {op}"),
```

### E. The CLI pieces (owned by #670, frozen; this story calls them)

**E-1** `SpecOnly` (clap enforces "requires `--profile`"):
```
crates/holler-cli/src/pane/args.rs:27-33
/// `--spec-only`: edit a profile's spec and change nothing live.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecOnly {
    /// Edit the profile's spec for the pane and change nothing live.
    #[arg(long, requires = "profile")]
    pub spec_only: bool,
}
```
**E-2** `SpecValues` and `SpecFlags::validate` (the guards and their codes; `SpecFlags` itself is the flag list in ADR 0003's
SPEC FLAGS line):
```
crates/holler-cli/src/pane/args.rs:89-119 (excerpt)
/// The spec flags, typed: what [`SpecFlags::validate`] returns.
///
/// The fields that need no check here (`project`, `workspace`, `model`, `effort`, `port_policy`,
/// `expect`) are the user's strings, as given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecValues {
    pub project: Option<String>,
    pub workspace: Option<String>,
    pub grid: Option<GridPos>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub role: Option<PaneRole>,
    pub env: Vec<EnvVarName>,
    pub ctx_soft: Option<u32>,
    pub ctx_hard: Option<u32>,
    pub port_policy: Option<String>,
    pub command: Option<Argv>,
    pub check: Option<Argv>,
    pub expect: Vec<String>,
}

impl SpecFlags {
    /// Type the flags with the guards of `holler-pane`, or refuse with the guard's own code:
    ///
    /// - `--grid`: `grid-ambiguous` (not a cell, or ambiguous) or `grid-out-of-range`;
    /// - `--env`: `profile-secret-refused` for `NAME=value` (the value is never echoed) and
    ///   `env-name-invalid` for an empty name or one with whitespace;
    /// - `--command-json` and `--check-json`: `command-not-argv` for JSON that is not an array of
    ///   strings, and `usage` for text that is not JSON;
    /// - `--role`: `usage` for anything but `agent` or `orchestrator`.
    pub fn validate(&self) -> Result<SpecValues, PaneError> {
```
(The body, lines 120-138, maps each flag to its field, through `GridPos::parse`, `parse_role`, `EnvVarName::parse` and
`argv_of` for the four guarded ones, and copies the rest as given.)
**E-3** `ErrorBody` (fields public, so a verb can append to `message`):
```
crates/holler-cli/src/output.rs:125-141
/// The `error` member of a failed envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
```
**E-4** `VerbCtx` and `emit`:
```
crates/holler-cli/src/output.rs:191-217
/// What a `holler pane` or `holler profile` verb runs with.
///
/// `ports` is held by value: [`Ports`] is `Copy`. A verb takes `&mut VerbCtx` because writing to
/// the sink needs a mutable borrow.
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
    match format {
        Format::Text => emit_text(sink, result, text),
        Format::Json => emit_json(sink, result),
    }
}
```
**E-5** `emit_error`:
```
crates/holler-cli/src/output.rs:240-243
/// Print an error (a failed `emit` with no data) and return its exit code.
pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
    emit(sink, format, Err::<(), _>(error), |()| String::new())
}
```
**E-6** The real wiring is still the not-implemented port set (#649 fills it), so the real binary never reaches an adapter:
```
crates/holler-cli/src/pane/wiring.rs:29-33
impl Wiring {
    /// Build the ports of a run. Fails when something they need cannot be reached.
    pub fn connect() -> Result<Self, PaneError> {
        Ok(Self { ports: Unwired })
    }
```
**E-7** The in-process harness a verb test uses with the fakes:
```
crates/holler-cli/tests/verb_harness/mod.rs:52-54 (excerpt; the body parses with `Cli::try_parse_from`, runs `pane::run`
or `profile::run` over a `VerbCtx` with the given ports, and returns `Outcome { code, out, err }`)
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
```
**E-8** "Accepted" in the shared parse tests tolerates a missing required positional:
```
crates/holler-cli/tests/verb_harness/parse.rs:28-33 (excerpt)
/// `Ok` when `holler <argv...>` is accepted: it parses, or only a required positional is missing.
pub fn accepted(argv: &[&str]) -> Result<(), String> {
    match try_parse(argv) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::MissingRequiredArgument => Ok(()),
```
**E-9** The verb test target allows these lints for every module (so the tests may `unwrap` and `panic`):
```
crates/holler-cli/tests/pane_verbs/main.rs:1-8
#![allow(clippy::unwrap_used)] // #670
#![allow(clippy::expect_used)] // #670
#![allow(clippy::panic)] // #670
#![allow(clippy::unreachable)] // #670
#![allow(dead_code)] // #670
//! In-process tests of the `holler pane` verbs (story #670, epic #633).
//!
//! One module per verb, so a verb story edits only `pane_verbs/<verb>.rs` and never
```
**E-10** The stub table of the process tests (a verb story deletes its own entries and keeps its `// #NNN` line):
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:9-24
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
    // #644
    ("pane", "launch", 644),
    ("pane", "relaunch", 644),
```
**E-11** The fixture lines this story owns (each must parse with `Cli::try_parse_from`):
```
crates/holler-cli/tests/fixtures/cli-surface.txt:113-123
# #644
pane launch |
pane launch | --profile demo
pane launch | --profile demo --spec-only
pane launch | --project /srv/demo --workspace main --grid r2c1 --model provider/model-id --effort high --role agent
pane launch | --env ALPHA_TOKEN --env BETA_URL --ctx-soft 100000 --ctx-hard 150000 --port-policy fixed
pane launch | --command-arg opencode --command-arg serve --check-arg curl --check-arg http://127.0.0.1:1/health --expect ok --expect ready
pane launch | --command-json [] --check-json [] --format=json
pane relaunch |
pane relaunch | --profile demo --spec-only --grid c1r2
pane relaunch | --command-json [] --check-json []
```
**E-12** The ADR 0003 rows this story owns:
```
docs/adr/ADR-0003.md:48-49
holler pane launch [SPEC FLAGS] [--profile NAME] [--spec-only]    #644
holler pane relaunch [SPEC FLAGS] [--profile NAME] [--spec-only]  #644
```
**E-13** `docs_cli_test` turns every `holler ...` row in `docs/**` (except `docs/handoffs/`) into an argv and parses it; optional
groups `[...]` are dropped, so a required positional must appear bare in the row and must parse as a plain string:
```
crates/holler-cli/tests/docs_cli_test.rs:130-132, 141-146 (excerpt)
/// Turn a documented form into an argv: strip prompt/annotation, resolve
/// placeholders, drop optional groups, pick the first of `a|b` alternatives.
fn normalise(raw: &str) -> Vec<String> {
    // Drop optional groups [ ... ], innermost first so nesting such as
    // `[--advertise HOST[:PORT]]` collapses cleanly.
    while let Some(close) = s.find(']') {
        let Some(open) = s[..close].rfind('[') else { break };
        s.replace_range(open..=close, "");
    }
```

### F. The test kit (`holler-pane-testkit`, a dev-dependency of `holler-cli`)

**F-1** The fault switch every fake shares:
```
crates/holler-pane-testkit/src/fault.rs:67-88
    /// Turn a standing fault on (`Some`) or off (`None`).
    pub fn set(&self, fault: Option<Fault>) {
        self.lock().standing = fault;
    }

    /// Fail the next call of `op` with `error`, once. The errors queued for one method
    /// come out in the order they were queued, and a call of another method leaves them
    /// queued. A standing fault answers first, also leaving them queued.
    pub fn fail_next(&self, op: Op, error: PaneError) {
        self.lock().queued.push((op, error));
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
**F-2** Fixture constants (scratch names; port 48100):
```
crates/holler-pane-testkit/src/fixture.rs:18-41
/// The harness port of every sample pane.
const SAMPLE_PORT: u16 = 48100;

/// The Herdr session and workspace of every sample pane and spec: a scratch name,
/// never the name of a live session.
const SCRATCH: &str = "scratch";

/// The grid cell of every sample pane and spec.
const SAMPLE_GRID: GridPos = GridPos { row: 1, col: 1 };

/// The project directory of every sample pane and spec.
const SAMPLE_CWD: &str = "/srv/demo";

/// The port policy of every sample spec.
const SAMPLE_PORT_POLICY: &str = "fixed";

/// A valid, deterministic `Pane` named `name`: generation 0, grid `r1c1`, no profile,
/// no session of record, harness port 48100 with its health unknown, an agent with no
/// hold, no command and no probe. Its Herdr session and workspace are scratch names,
/// never a live session's. Its tmux session is the pane's name, as on a real pane, so
/// pick a neutral one such as `demo-c1r1`. Two calls with one name return equal panes.
///
/// `usage` when `name` is not a valid pane name.
pub fn sample_pane(name: &str) -> Result<Pane, PaneError> {
```
**F-3** `FakePaneStore` rules:
```
crates/holler-pane-testkit/src/pane_store.rs:48-67
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
///
/// Two behaviours are the fake's own, not the port's:
///
/// - A `cas_put` is checked for its generation first and for membership second, so a
///   stale write that also changes the profile is `generation-conflict`.
/// - The fake does not check that the profile a pane names exists. The hub does that
///   in `check_membership`, outside the port, and answers `profile-not-found`.
```
**F-4** Another writer's put:
```
crates/holler-pane-testkit/src/pane_store.rs:113-117
    /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
    /// at 1 for a new record), without the membership rule, and publishes its event.
    /// It bypasses the faults and the call log. Returns the stored record.
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
        self.put(pane, Writer::Other)
```
**F-5** `FakeProfileScope` rules (I8 order, the restore, the guards):
```
crates/holler-pane-testkit/src/profile_scope.rs:31-54
/// A `ProfileScope` over any `ProfileStore` and `PaneStore`, keeping ADR-0021's I8 write
/// order (section 8).
///
/// - `resolve(P, None)` is the stored P and every pane whose record names P, in name
///   order. `resolve(P, Some(n))` is P and n, whose record must name P, or else
///   `pane-not-in-profile` (a pane with no record included). A missing P is
///   `profile-not-found`, checked first. Membership is `Pane.profile` alone, compared by
///   slug, so a spec of P adds no pane to its scope.
/// - `edit_spec(Some(P), n, edit, act)` reads P (`profile-not-found` before anything
///   else) and n's record, then writes P with the edit by a compare-and-swap at P's
///   generation, and only then runs `act`. A conflict on that first write is
///   `generation-conflict`, and the act never runs. A `Set` replaces n's entry in place
///   or appends one; a `Remove` drops it, and still writes when there is none. If the
///   act fails, the scope puts P's specs back by a second compare-and-swap, so the
///   generation moves by two and P's log shows the edit and its reversal; a conflict
///   there is `profile-conflict`. It returns P as the first write stored it.
/// - `edit_spec(None, ..)` runs only the act and calls neither store.
///
/// Its own guards, both before anything is written: a `Set` whose spec names another
/// pane is `usage`, and a `Set` for a pane whose record belongs to another profile is
/// `pane-in-other-profile`, by the fake pane store's membership rule. A `Remove` is never
/// refused for membership: a detached spec stays removable.
///
/// It never writes a pane record: recording the pane (ADR-0021 section 8, step 4) is
```
**F-6** The one-shot hook that makes the restore conflict:
```
crates/holler-pane-testkit/src/profile_scope.rs:79-89
    /// Run `hook` once, after the next act that fails in an `edit_spec` with a profile
    /// and before the scope's restoring write; then it is dropped. A failing act without
    /// a profile, a first write that fails and an act that succeeds leave it armed.
    ///
    /// A verb test makes another writer move the profile there, typically
    /// `move || { profiles.concurrent_put(&other, &other_actor).unwrap(); }` over its own
    /// `Arc<FakeProfileStore>`, so the restore really conflicts and `edit_spec` answers
    /// `profile-conflict` (ADR-0021 section 8, step 6). Arming it again replaces an
    /// unused hook. The hook runs with no lock of the scope held, so it may arm the next
    /// one.
    pub fn before_next_restore(&self, hook: impl FnOnce() + Send + 'static) {
```
**F-7** `FakeProfileScope::edit_spec`:
```
crates/holler-pane-testkit/src/profile_scope.rs:209-231
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        // ASSUMPTION (#663): without a profile the scope makes no store call at all.
        let Some(profile) = profile else {
            return act().map(|()| None);
        };
        // ASSUMPTION (#663): `profile-not-found` comes before `pane-in-other-profile`.
        let stored = self.stored(profile)?;
        let edited = self.plan(&stored, pane, edit)?;
        // ASSUMPTION (#663): the first write is not retried on a conflict.
        let written = self
            .profiles
            .cas_put(&edited, stored.generation, &self.actor)?;
        match act() {
            Ok(()) => Ok(Some(written)),
            Err(failure) => Err(self.restore(stored, written, pane, failure)),
        }
    }
```
**F-8** Host and harness fakes share no state (so a relaunch test links them):
```
crates/holler-pane-testkit/src/harness.rs:11-15
//!
//! The host and harness fakes share no state. `FakeHost::stop_owned` does not stop a
//! server of this fake, and a harness pid is never in `FakeHost::ps`. A test that needs
//! the two to agree drives both, e.g. with a `HostPort` wrapper whose `stop_owned` also
//! calls [`FakeHarness::kill`].
```
**F-9** `FakeHarness` rules:
```
crates/holler-pane-testkit/src/harness.rs:110-138
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
///
```
**F-10** `FakeHarness` scenario methods:
```
crates/holler-pane-testkit/src/harness.rs:199-216
    /// SIGSTOP the server on `port`: it is frozen. `unavailable` when no server was
    /// ever served on `port`, or it was killed.
    pub fn freeze(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Frozen)
    }

    /// SIGCONT the server on `port`: it runs again. `unavailable` when no server was
    /// ever served on `port`, or it was killed.
    pub fn thaw(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Running)
    }

    /// SIGKILL the server on `port`: it is dead, its sessions stay in the data
    /// directory, and a TUI attached to it keeps its screen. `Ok` on a server already
    /// killed; `unavailable` when no server was ever served on `port`.
    pub fn kill(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Killed)
    }
```
**F-11** `FakeHarness` inspection:
```
crates/holler-pane-testkit/src/harness.rs:254-263
    /// The server last served on `port`, or `None` when none ever was. A launch test
    /// (#644) reads it to see which pane's server runs on a port, with which pid.
    pub fn server(&self, port: u16) -> Option<ServerView> {
        self.lock().servers.get(&port).cloned()
    }

    /// The TUI in `pane`, or `None` when there is none.
    pub fn tui(&self, pane: &PaneId) -> Option<TuiView> {
        self.lock().tuis.get(pane).cloned()
    }
```
**F-12** `FakeHost` rules:
```
crates/holler-pane-testkit/src/host.rs:46-61
/// An in-memory `HostPort`: tmux sessions by pane name, the processes in them and every
/// argv run.
///
/// It keeps the port's rules as the conformance suite pins them for the tmux adapter
/// (#641):
///
/// - `ensure_session` creates a missing session working in `cwd`; on an existing one it
///   is `Ok` and changes nothing, neither the cwd nor the processes.
/// - `run` in a missing session is `pane-not-found`, and an empty argv is `usage`.
///   Otherwise it starts one process in the session and records the argv exactly as
///   given: an element is never joined to another or re-split, and nothing goes
///   through a shell.
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
```
**F-13** The argv log:
```
crates/holler-pane-testkit/src/host.rs:124-127
    /// Every argv a successful `run` started, oldest first, with its session. The log
    /// is never cleared, not by `stop_owned` and not by `end_session`.
    pub fn runs(&self) -> Vec<(PaneName, Argv)> {
        self.lock().runs.clone()
```
**F-14** `FakeHost::stop_owned`:
```
crates/holler-pane-testkit/src/host.rs:187-193
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
        self.faults.enter(HostOp::StopOwned)?;
        if let Some(session) = self.lock().sessions.get_mut(name) {
            session.pids.clear();
        }
        Ok(())
    }
```
**F-15** `FakeHerdr` rules:
```
crates/holler-pane-testkit/src/herdr.rs:112-127
/// An in-memory `HerdrPort` serving one Herdr session.
///
/// A test declares each workspace with a size ([`FakeHerdr::with_workspace`]).
/// `ensure_pane` checks, in this order: the session and the workspace (one the fake
/// does not serve is `unavailable`), the range (a cell outside the workspace is
/// `grid-out-of-range`), an occupant (an occupied cell answers its pane and changes
/// nothing, in either placement), the placement (see [`Placement`]), and then it mints
/// the next id. An id is `w<N>:p<M>`: the workspace's number, then its pane counter in
/// base 36 (spike section 5). The counter only goes up, so an id is never reused after
/// a close or a vanish. `close` and [`FakeHerdr::vanish`] free a pane's cell, and no
/// other pane is ever moved, replaced or renumbered.
///
/// A pane's screen shows what is typed into it (`enter` breaks a line) and what
/// [`FakeHerdr::print`] adds, and `read` returns its last lines. An unknown, closed or
/// vanished id is `pane-not-found` everywhere. Every `send_text` and `send_keys` that
/// reaches a pane is in [`FakeHerdr::sent`].
```
**F-16** A Herdr pane that vanishes:
```
crates/holler-pane-testkit/src/herdr.rs:223-229
    /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is
    /// free, its id is never reused, and every later call naming it is
    /// `pane-not-found`. It bypasses the faults and the call log. `pane-not-found` when
    /// no such pane exists.
    pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock().remove(pane)
    }
```
**F-17** The version string `FakeHerdr::version()` reports by default:
```
crates/holler-pane-testkit/src/herdr.rs:29-30
/// The version string of the one Herdr build the spike tested (protocol 22).
pub const PROTOCOL_22_VERSION: &str = "0.9.1-preview.2026-09-21-0ff0f27e2226";
```
**F-18**, **F-19** `FakeProber` (an unscripted argv answers `Error`, never `Ok`; the earlier citation of lines 283-321 was
wrong, the file has 95 lines):
```
crates/holler-pane-testkit/src/prober.rs:5-6, 56-65 (excerpt)
//! An argv nobody scripted answers [`ProbeResult::Error`], never [`ProbeResult::Ok`],
//! so a test that forgot to script a probe cannot pass it (the same reason the stub
    /// Answer `result` for every run of exactly `argv` (equal element by element).
    /// Scripting the argv again replaces its result.
    pub fn script(&self, argv: Argv, result: ProbeResult) {
        self.lock().scripted.insert(argv, result);
    }

    /// Every run, oldest first, with its `expect` and `timeout`; the unscripted runs are
    /// included.
    pub fn calls(&self) -> Vec<ProbeCall> {
        self.lock().calls.clone()
    }
```
**F-20** The envelope checker every `--format=json` test uses (14 numbered rules; the ones this story leans on):
```
crates/holler-pane-testkit/src/envelope.rs:12, 25-26, 31-33 (excerpt)
//! 0. The exit code is 0, 1, 2 or 3. This is checked before stdout is read.
//! 6. `ok` is the boolean `true` when the exit code is 0, and `false` otherwise.
//! 7. On success, `error` is `null`. `data` may be any value, `null` included.
//! 11. `code` is a string that [`is_valid_code`] accepts.
//! 12. `message` is a string that is not blank and has no `\n` or `\r`.
//! 13. The exit code matches the class of the code, `class_of(code).exit_code()`.
```
```
crates/holler-pane-testkit/src/envelope.rs:202-202
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
```
**F-21** Another writer's profile put:
```
crates/holler-pane-testkit/src/profile_store.rs:157-163
    /// (or at 1 for a new profile), without the name rule, logs its entry with `actor`
    /// and publishes its event. It bypasses the faults and the call log. Returns the
    /// stored record.
    pub fn concurrent_put(&self, profile: &Profile, actor: &Actor) -> Result<Profile, PaneError> {
        self.put(profile, Writer::Other, actor)
    }

```

### G. ADR-0021 (the standing spec)

**G-1** Section 8, the I8 write order:
```
docs/adr/ADR-0021.md:285-305
**Decided (the epic's order, as the merged `ProfileScope::edit_spec` documents it): the I8 write order.** For a
spec-editing verb with `--profile P`:

1. **Plan.** Read P at generation `g` and the pane record; compute P with the edit (`SpecEdit::Set` or `SpecEdit::Remove`).
   A `Set` for a pane whose record belongs to another profile is refused here with `pane-in-other-profile`, before anything
   is written or moved, with or without `--spec-only` (the scope cannot see that an act is empty); a `Remove` is not (a
   detached spec stays removable), and the pane registry's check ("Decisions taken", item 2) stays the authority (#688).
2. **Profile CAS first.** `ProfileStore::cas_put(P_edited, g, actor)`. A conflict here comes before anything live changed:
   the verb exits 1 with `generation-conflict` and can be run again.
3. **Act** (skipped with `--spec-only`), then observe.
4. **Record.** Write the pane record with its own compare-and-swap (`cas_put`, or `delete` for `close`).
5. **If the act fails, nothing is recorded in P:** the pane record is not written, and the profile is put back by a
   compensating `cas_put` of its previous specs at `g + 1`. P's specs then equal what they were before the verb, and its
   log shows the edit and its reversal.
6. **A conflict after the act fails loudly.** If the compensating write of step 5 conflicts (another writer moved P during
   the act), the verb exits 1 with `profile-conflict`, naming P, and prints the reconcile step: the exact pane doctor command
   line, and the profile show command for P. A pane-record conflict in step 4 is `generation-conflict`, handled the same way.

With `profile: None`, `edit_spec` runs only the act and touches no profile. This order deliberately makes the edit visible
in P before the live change, so a concurrent profile editor is refused before anything live moves. The cost is a second
profile write on a failed act; see "Decisions taken", item 1.
```
**G-2** Section 9, the table's preamble and the codes of these two verbs (the `pane switch` row at line 339 shows the
"open (#645) for ..." form):
```
docs/adr/ADR-0021.md:331-332, 338
**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
| `pane launch`, `pane relaunch` | `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `profile-secret-refused`, `probe-failed`, `herdr-version-unsupported`, `session-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-in-other-profile`; `relaunch` also `pane-not-found` |
```
**G-3** Section 10, positions and versions:
```
docs/adr/ADR-0021.md:429-435
- The Herdr adapter (#640) is the only code that converts a `GridPos` to Herdr's own order and base. `HerdrPort::ensure_pane`
  reaches a position by right and down splits (B4, `plan_splits`, #640) or fails loudly, and never relocates a healthy pane.
  `relaunch` moves a pane only when `--grid` is given. A pane's position comes from Herdr's snapshot, never from its name:
  `hj-c1r2` sits at `r2c1`.
- Herdr API versions (B5): `HerdrPort::version()` is read on connect, recorded as `host.herdr_api_version` and shown by
  `doctor`. An unknown version is `herdr-version-unsupported`, and the message names the supported ones (#640, from #636's
  list).
```
**G-4** Section 11, who chooses the session:
```
docs/adr/ADR-0021.md:439-447
[ADR 0014](ADR-0014.md) stays in force, unchanged. What changes is **who chooses the session**. For a pane in the registry,
Holler creates the session of record: the pane verbs start the harness server (`HarnessPort::serve`), create the session over
its API (`create_session`) and attach the TUI to it (`attach_tui`). The body that drives that session is still not its
parent, so ADR 0014's teardown rule (point 3) and its rule against typing into a PTY (point 4, which I4 restates for pane
verbs) apply as they are. The difference is that the session id comes from `session_of_record`, which a verb writes after it
observes the session. It is no longer pasted by an operator or guessed by a repair. That answers ADR 0014's open "automatic
discovery of attachable sessions (e.g. from a Herdr pane)" for registered panes: there is no discovery; the registry names
the session. Attach mode with an operator-written `endpoint` and `session_id` stays for sessions Holler does not own (a
session with no pane record).
```
**G-5** Section 12, the operation id:
```
docs/adr/ADR-0021.md:455-468
### 12. Long-running work and the operation id (I5)

**Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step. A crash between steps leaves state that
the next pane doctor run finds and reports (#644's acceptance). No verb leaves work running after it exits.

**Decided (operator, 2026-10-09): the operation id is deferred to #644, with an amendment to the contract first.** It needs two things the frozen contract
does not have: an executor that outlives the CLI process, and a place to record an operation's progress. `Pane` refuses
unknown fields and has no operation field, and `PANE_METHODS` has no operation methods. Two options are on record for #644 to choose from, after the OpenCode and Herdr spikes show real launch times:
(a) the verb starts a detached worker process of the same binary and returns an id, the worker records progress in a new
hub-held operation store (new control-socket methods beside `pane/*`), and the hub still runs no adapter; or (b) a hub-side
executor for long work only, which narrows ruling 1. Until one is chosen, I5 is satisfied by the per-call bound and the
loud `timeout`, not by an operation id.
```
**G-6**, **G-7** Two items "Deferred to named stories" that name #644:
```
docs/adr/ADR-0021.md:523-523
- The operation id and the executor of long work: #644, after a contract amendment (section 12).
```
```
docs/adr/ADR-0021.md:531-532
- Which closed failure code a mismatch observed after `act` carries (I3 says the verb exits 1, and under section 9 an open
  code is a refusal, exit 3): #644 and #645.
```
**G-8** "Decisions taken", item 1:
```
docs/adr/ADR-0021.md:539-542
1. **The I8 compensation (section 8): the epic's order stays.** The profile is written first. A failed act restores the
   profile's specs by a second write, so the specs equal what they were but the generation has moved by two and the log
   shows the edit and its reversal. The acceptance of #663 and the conformance case in #638 are amended from "generation
   equal" to "specs equal".
```

### H. The spikes and the sibling adapter brief

**H-1** Real timings (OpenCode spike) and the only argv launch Herdr has (Herdr spike):
```
docs/research/opencode-pane-spike.md:47
| 2 | A TUI started attached to that server and that session | **works** | `opencode attach http://127.0.0.1:<port> --dir <dir> --session <id>` shows that session in about 1.6-1.7 s; an unknown id exits 1 (verified) |

docs/research/opencode-pane-spike.md:69-70
**Observed (verified).** The server is healthy 630-740 ms after spawn, warm or cold, including with a fresh config
directory. A fresh server has **no** session of its own: no "ping" session. A created session keeps the title

docs/research/opencode-pane-spike.md:195-196
- **Restart:** a new `serve` on the same port and data directory was healthy in about 645 ms. The session of record
  was still there, and the **old** TUI received a `show-toast` from the new server, so it re-subscribed by itself.

docs/research/herdr-api-spike.md:249-251
- **`layout.apply`** builds a whole tree in one call. A pane node may carry `command` (argv), `cwd`, `env` and
  `label`. Into a new tab it is the only way to start an argv **without a shell** (VERIFIED: `process_info` shows
  `sleep 601` as the foreground process and `shell_pid == foreground_process_group_id`). Onto an existing
```
**H-2** #642's decisions that bind #644 (follow-up #695, open, owns "who stops the OpenCode server `serve` started: the host
adapter, by recorded pid"):
```
docs/handoffs/642-brief.md:764-773 (branch issue-642-implementation at b102cb7; NOT on main: the #642 brief, already plan-reviewed)
2. **`serve` never adopts a running server.** A port that already answers is `unavailable`. The fake is more lenient (same
   pane on a running port returns the old pid), but the adapter cannot tell whose server answers or its pid without a
   process-table search, which #641's rules forbid by spirit (no matching processes by name). The conformance suite does not
   test idempotence. #644 must check `health` and the recorded pid before calling `serve`.
   **Who stops the server `serve` starts (W-1; MO decision):** the host adapter (#641), by the recorded pid, killing the
   server's process group **and its descendants** (OpenCode runs shell commands in sessions of their own, spike 175-177,
   244, so a group kill alone misses them). `serve` already returns a pid that is the server's process-group id. `HarnessPort`
   gains no stop method. This is a forward-compat fact for #641 and #644; the orchestrator files a follow-up issue for it
   (no number yet), and #641's brief is not changed by this amendment. Until it lands, a relaunch onto a running server
   fails at `serve` with "port N is in use", loudly, never by adopting it.

docs/handoffs/642-brief.md:810-813 (same branch)
14. **Resolver precondition (W-2).** #644 writes the pane record once, after the act (ADR-0021 §8 step 4, line 292), so a
    registry-backed resolver cannot answer for a new pane during launch. Each resolver must answer for a pane being launched
    before its record exists; #644 passes the session name (and directory) explicitly from its plan, and #649 builds
    resolvers that consult it. Recorded in the `Resolver` doc, the crate docs and the ADR note; the API does not change.
```
**H-3** The one wall-clock helper (#207 folded 5+ hand-written copies into it; `holler-pane` already depends on
`holler-proto`, `crates/holler-pane/Cargo.toml:21`):
```
crates/holler-proto/src/clock.rs:33-40
/// The current unix epoch in whole milliseconds. `0` on a clock error, same
/// discipline as [`now_secs`].
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
```

### I. The dependencies' APIs (the contract; NOT on `main` when this was written; re-verified against merged code at RED)

**I-1** #662a's `profile_snapshot.rs`, the one Pane-to-spec mapping (ADR-0021 section 3 puts it there):
```
docs/handoffs/662-brief.md:1436-1455 (branch issue-662-implementation at 5b47d82; the #662 brief, scoped to 662a)
use crate::pane::Pane;
use crate::profile::{Profile, ProfileName, ProfileSpec};

/// The prefix of the port policy that pins a harness to one port.
pub const FIXED_PORT_POLICY_PREFIX: &str = "fixed:";

/// The port policy that pins a pane's harness to `port`: `fixed:<port>`, e.g. `fixed:48100`.
pub fn fixed_port_policy(port: u16) -> String;

/// The spec that reproduces `pane` as its record holds it. Pure; no port is called.
/// pane                <- pane.name (as text)          herdr.workspace <- pane.herdr.workspace
/// herdr.grid          <- pane.herdr.grid (never parsed from the name)
/// host.cwd            <- pane.host.cwd                harness.kind    <- pane.harness.kind
/// harness.port_policy <- fixed_port_policy(pane.harness.port)
/// model <- pane.model   role <- pane.role   env <- pane.env (names, in order)   context <- pane.context
/// command <- pane.command   check <- pane.probe.check   expect <- pane.probe.expect (in order)
/// Not copied: generation, herdr.session, herdr.pane_id, host.name, host.tmux, host.herdr_api_version,
/// harness.pid, harness.health, session_of_record, hold, last_observed, profile, probe.last.
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec;
```
**I-2** #662's decision on the policy, which leaves the rest of the grammar to #644:
```
docs/handoffs/662-brief.md:1724-1729 (same branch and commit)
3. **`harness.port_policy` under `--from-current` is `fixed:<port>`** (ADR-0021's deferred item). The record holds the port in
   use and no policy (E11); `fixed:<port>` keeps that fact, so `apply` (#664) and the recreation (#666) can put the harness back
   on the same port. `fixed` alone (the fixture's value) would lose it. The policy grammar is otherwise #644's; this story
   defines only this one form, as `FIXED_PORT_POLICY_PREFIX` and `fixed_port_policy`. `port_policy` is **not compared**
   (`SpecField::COMPARED` omits it), because a live pane has no policy to compare and a spec's `fixed` or `auto` would
   otherwise always differ.
```
**I-3** #663's reconcile step and which scope errors already carry it:
```
docs/handoffs/663-brief.md:1527-1533, 1540-1555 (branch issue-663-implementation at aaf8fb5; the #663 brief)
5. **The open point: a restoring write that fails with anything but a conflict** (`timeout`, `unavailable`, `store-corrupt`,
   ...). ADR section 8 decides only the conflict. **Decided: keep the restoring write's own code, and extend its message.**
   The code stays because it is the most recent failure and the one that says what is wrong now (a wedged or unreachable or
   corrupt store), and because each of these codes is already a failure (exit 1) in `class_of` (F). The message gains, after
   the store's own text: `; profile "<P>" still holds the edit of <pane>, but the live change failed (<failure>); <reconcile
   step>` (the name in prose is written with `{:?}`, as the fake writes it; only the reconcile step's command line is
   shell-quoted, Decision 8). So the error names the profile, the unrestored edit, the act's error (which would otherwise be lost) and the
6. **The first write's failures.** `generation-conflict` is returned as it is: nothing live has moved and the verb can be run
   again (ADR step 2; case 10). `timeout` gets the same message extension as Decision 5 but worded for an unknown outcome:
   `; the write may have landed, so profile "<P>" may hold the edit of <pane>, and nothing live was changed; <reconcile
   step>` (a timed-out write may still have been applied; ADR section 12 asks a timeout to print the reconcile step). Every
   other first-write error is returned unchanged (`unavailable` and the rest: the store said no). AC 4 pins both.
7. **The restore conflict** is `PaneError::ProfileConflict { what }` with `what` = `"<P>" was changed by another writer during
   the live change to <pane>, so its specs were not restored after that change failed (<failure>); the other writer's
   version stays; <reconcile step>`. This is the fake's text (G, lines 172-179) plus the reconcile step ADR step 6 asks for.
   AC 3 pins it; case 11 pins the code and the profile name.
8. **The reconcile step** is one function, `pub fn reconcile_step(profile: Option<&ProfileName>) -> String` (as merged on main by #663), returning for
   `Some(P)` exactly `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`, and for `None`
   exactly `to reconcile, run holler pane doctor`. It is `pub` because the
   spec-editing verbs (#644, #646) print the same step for a pane-record conflict (ADR step 4); one copy. The profile name is
   POSIX-single-quoted (`'` becomes `'\''`, the whole wrapped in `'...'`), always, because a profile name may hold spaces,
   quotes, `$(...)` or backticks (D, lines 30-61) and an operator pastes this line into a shell. `ProfileName` refuses
   control characters, so the message stays one line (ADR section 9: every message is one line). The pane name is
   `[a-z0-9-]` and is not quoted. It names the doctor form that exists today (C9).
```
Its decision 3.5 (same brief, line 1518) is the one path where the act's own error comes back with no step: "It succeeds:
return `Err(failure)` unchanged (case 9)." Its follow-up F1 (line 1654) will later make `FakeProfileScope` add the step too.
**I-4** #663's real scope, which AC 16k runs the verb over (its file is `crates/holler-cli/src/pane/profile_scope.rs`):
```
docs/handoffs/663-brief.md:1499-1500 (same branch and commit)
1. **Type.** `pub struct StoreScope { profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor }` and
   `pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self`: the fake's shape (G), so
```

### J. Excerpts added at the brief review, round 1 (pasted from this branch at `d92bf0a`, which holds the merged #663)

**J-1** #663's merged `StoreScope::edit_spec` (the real scope AC 16k runs over): it writes only the profile, through
`ProfileStore::cas_put` at the stored generation, and never a pane record, so the generation of a pane record is applied by
the pane store alone (F-3) and the engine calls no `next_generation`:
```
crates/holler-cli/src/pane/profile_scope.rs:180-200
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        let Some(profile) = profile else {
            return act().map(|()| None);
        };
        let stored = self.stored(profile)?;
        let edited = self.plan(&stored, pane, edit)?;
        let written = self
            .profiles
            .cas_put(&edited, stored.generation, &self.actor)
            .map_err(|error| may_have_landed(error, &stored.name, pane))?;
        match act() {
            Ok(()) => Ok(Some(written)),
            Err(failure) => Err(self.restore(stored, written, pane, failure)),
        }
    }
```
**J-2** Where a fake's delay is applied, and what bypasses it (AC 24's derivation; the rig's call-log assertions):
```
crates/holler-pane-testkit/src/fault.rs:90-103
    /// What a fake calls first in every port method. It records the call, sleeps for
    /// the delay (without holding the lock, so other calls proceed), and then answers
    /// the standing fault if there is one, or else the oldest error queued for `op`.
    pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
        let delay = {
            let mut state = self.lock();
            state.calls.push(op);
            state.delay
        };
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        self.lock().take_fault(op)
    }

crates/holler-pane-testkit/src/harness.rs:139-143
/// Every port method first passes [`FakeHarness::faults`], which models a wedged
/// *adapter*: under `Fault::Wedged` every method answers `timeout`, `health` and
/// `shown_session` included. [`FakeHarness::freeze`] models a wedged *server* behind a
/// working adapter. A method that fails changes nothing. The configuration, scenario
/// and inspection methods bypass the faults and the call log.

crates/holler-pane-testkit/src/harness.rs:214-216
    pub fn kill(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Killed)
    }
```
**J-3** `FakeHerdr::version()` and `set_version` (AC 14):
```
crates/holler-pane-testkit/src/herdr.rs:36
pub const SUPPORTED_VERSIONS: &str = "Herdr protocol 22 (0.9.1)";

crates/holler-pane-testkit/src/herdr.rs:204-207
    /// Which build `version()` reports from now on.
    pub fn set_version(&self, version: HerdrVersion) {
        self.lock().version = version;
    }

crates/holler-pane-testkit/src/herdr.rs:313-327
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
**J-4** `docs_cli_test`'s whole `normalise` and the parse call (E-13): a two-space annotation such as `    #644` is cut, the
`[...]` groups are dropped, so the ADR 0003 row `holler pane launch PANE [--herdr-session NAME] [SPEC FLAGS] [--profile NAME]
[--spec-only]    #644` becomes the argv `holler pane launch PANE`, which `Cli::try_parse_from` accepts with `PANE` as the
positional's plain `String` value (`PaneName::parse` runs only in `run`, never at clap time):
```
crates/holler-cli/tests/docs_cli_test.rs:132-157
fn normalise(raw: &str) -> Vec<String> {
    let mut s = raw.to_string();
    // Cut a trailing annotation: two+ spaces, " (", or " — ".
    for sep in ["  ", " (", " — ", " -- "] {
        if let Some(i) = s.find(sep) {
            s.truncate(i);
        }
    }
    // Drop optional groups [ ... ], innermost first so nesting such as
    // `[--advertise HOST[:PORT]]` collapses cleanly.
    while let Some(close) = s.find(']') {
        let Some(open) = s[..close].rfind('[') else { break };
        s.replace_range(open..=close, "");
    }
    // <placeholder> -> placeholder
    s = s.replace(['<', '>'], "");
    // ellipses
    s = s.replace('…', "").replace("...", "");
    // a|b alternatives -> a  (on whole tokens)
    let toks: Vec<String> = split_args(&s)
        .into_iter()
        .map(|t| t.split('|').next().unwrap_or("").to_string())
        .filter(|t| !t.is_empty())
        .collect();
    toks
}

crates/holler-cli/tests/docs_cli_test.rs:187-188
            match Cli::try_parse_from(&argv) {
                Ok(_) => {}
```
**J-5** The two shared process tests that run these verbs with no `PANE` (Risks 4; #670's file, not edited here). Which
clap error wins once `PANE` is required is **not** shown by this excerpt: T-red confirms it by running them (Risks 4).
```
crates/holler-cli/tests/pane_verbs/process/usage.rs:58-61
fn spec_only_requires_profile() {
    for verb in ["launch", "relaunch", "close"] {
        let text = holler(&["pane", verb, "--spec-only"]);
        assert_plain_usage_error(&text, "--profile");

crates/holler-cli/tests/pane_verbs/process/usage.rs:123-131
fn command_arg_and_command_json_are_mutually_exclusive() {
    for (verb, a, b, json_like) in [
        ("launch", "--command-arg", "--command-json", "[]"),
        ("relaunch", "--command-arg", "--command-json", "[]"),
        ("launch", "--check-arg", "--check-json", "[]"),
        ("relaunch", "--check-arg", "--check-json", "[]"),
    ] {
        let text = holler(&["pane", verb, a, "x", b, json_like]);
        assert_plain_usage_error(&text, a);
```

## The public API (fixed here, so T can write RED tests against it)

`crates/holler-pane/src/tx_launch.rs` (reached as `holler_pane::tx_launch::...`; `lib.rs` already declares `pub mod tx_launch`
and is not edited). Names, fields and signatures are binding; private helpers are F's choice. No new dependency (the clock is
`holler_proto::clock::now_millis`, H-3, a crate `holler-pane` already depends on), and no I/O of its own (every effect goes
through `Ports`). No Pane-to-spec mapping and no `"fixed:"` literal: both are #662a's (I-1).

```rust
use std::time::Duration;
use crate::error::RefusalCode;
use crate::profile_snapshot::FIXED_PORT_POLICY_PREFIX; // #662a (I-1)
use crate::{Pane, PaneError, PaneName, Ports, Profile, ProfileName, ProfileSpec};

/// `pane-exists`: `launch` of a name that already has a record (relaunch it instead). A refusal, exit 3.
pub const PANE_EXISTS: RefusalCode = RefusalCode::from_static("pane-exists");
/// `grid-occupied`: the target cell already holds a Herdr pane (never adopted, recorded or not). A refusal, exit 3.
pub const GRID_OCCUPIED: RefusalCode = RefusalCode::from_static("grid-occupied");
/// `port-in-use`: a harness server already answers on the port before this verb started one (never adopted). A refusal, exit 3.
pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
/// The budget of one whole verb run (I5's default bound), checked before every step.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
/// The timeout handed to `Prober::run_probe`.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// `HostInfo.name` of every pane recorded here (epic decision 3: Herdr, tmux and the harness run on the hub's machine).
pub const HOST_NAME: &str = "localhost";
/// The `op` of the `timeout` a run answers when its budget runs out.
pub const OP_LAUNCH: &str = "pane.launch";
pub const OP_RELAUNCH: &str = "pane.relaunch";

/// The port a policy names: the policy's grammar beyond #662a's one form is this story's (I-2). Only
/// `FIXED_PORT_POLICY_PREFIX` followed by `<port>` exists (decision 5), `<port>` in 1..=65535 in canonical decimal (digits
/// only, no leading zero), so `port_of_policy(&fixed_port_policy(p)) == Ok(p)` for every such `p` and an accepted policy
/// equals `fixed_port_policy` of its port. Anything else (the bare `fixed`, `fixed:048100`, `fixed:0`) is `usage`, and the
/// message names the accepted form.
pub fn port_of_policy(policy: &str) -> Result<u16, PaneError>;

#[derive(Debug, Clone, Copy)]
pub struct TxOptions {
    pub budget: Duration,        // DEFAULT_BUDGET
    pub probe_timeout: Duration, // PROBE_TIMEOUT
    pub now_ms: fn() -> i64,     // holler_proto::clock::now_millis (H-3); tests pass a constant
}
impl Default for TxOptions { /* the defaults above */ }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub name: PaneName,
    pub herdr_session: Option<String>, // required for a live launch (usage otherwise); unused with spec_only
    pub spec: ProfileSpec,             // the effective spec, complete; the engine checks spec.pane == name (usage)
    pub profile: Option<ProfileName>,
    pub spec_only: bool,               // only with Some(profile); the CLI's clap rule guarantees it, the engine re-checks (usage)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaunchRequest {
    pub record: Pane,                  // as the caller read it; its generation is the expected generation of the record write
    pub spec: ProfileSpec,             // the effective spec, complete
    pub grid_given: bool,              // the caller was asked to place the pane (`--grid`); the engine derives the move itself (decision 11)
    pub profile: Option<ProfileName>,
    pub spec_only: bool,               // only with Some(profile); the CLI's clap rule guarantees it, the engine re-checks (usage), as for LaunchRequest
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launched {
    pub pane: Option<Pane>,            // the stored record (None with spec_only)
    pub profile: Option<Profile>,      // P as edit_spec stored it (None without --profile)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxFailure {
    pub error: PaneError,
    pub acted: bool,                   // edit_spec entered the act, or a step after it failed (B10): the CLI prints the reconcile step
}
impl From<PaneError> for TxFailure { /* acted: false */ }

pub fn launch(ports: Ports<'_>, request: &LaunchRequest, options: &TxOptions) -> Result<Launched, TxFailure>;
pub fn relaunch(ports: Ports<'_>, request: &RelaunchRequest, options: &TxOptions) -> Result<Launched, TxFailure>;
```

`crates/holler-cli/src/pane/launch.rs` (replaces A-2; `mod.rs` is frozen and already dispatches `launch::run`):

```rust
#[derive(Args, Debug)]
pub struct PaneLaunch {
    /// The pane's name (also its tmux session name), e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub name: String,                  // a plain String at clap time (E-13: the ADR row's bare `PANE` must parse); typed by PaneName::parse in run
    /// The Herdr session the pane lives in. Required unless --spec-only.
    #[arg(long, value_name = "NAME")]
    pub herdr_session: Option<String>,
    #[command(flatten)] pub spec: SpecFlags,
    #[command(flatten)] pub profile: ProfileOpt,
    #[command(flatten)] pub spec_only: SpecOnly,
}
pub fn run(args: &PaneLaunch, ctx: &mut VerbCtx<'_>) -> i32;

/// Overlay the typed flags on `base` (None: every required field must come from the flags). Shared with relaunch.rs.
pub(crate) fn effective_spec(base: Option<&ProfileSpec>, name: &PaneName, values: &SpecValues) -> Result<ProfileSpec, PaneError>;
/// Print a run's result through output::emit. When `acted`, it appends "; " and the run's reconcile step to the message,
/// unless the message already contains that exact step (decision 15): with `profile: Some(P)` the step is #663's
/// `super::profile_scope::reconcile_step(Some(&P))` (I-3), else `reconcile_step(None)`. No quoting code and no constant of its own.
/// The check is `let step = reconcile_step(profile); if !message.contains(step.as_str()) { append "; " + step }`: a
/// substring test for the whole output of that one call, never for a prefix such as "to reconcile" (so the scope's
/// `Some(P)` step and the verb's step for the same run compare equal, and nothing else does).
pub(crate) fn emit_outcome(ctx: &mut VerbCtx<'_>, verb: Verb, name: &PaneName, profile: Option<&ProfileName>,
                           result: Result<Launched, TxFailure>) -> i32;
pub(crate) enum Verb { Launch, Relaunch }
```

`crates/holler-cli/src/pane/relaunch.rs` (replaces A-3): `PaneRelaunch { name: String (positional PANE), spec, profile,
spec_only }` (no `--herdr-session`: the record names it) and `pub fn run(args: &PaneRelaunch, ctx: &mut VerbCtx<'_>) -> i32`,
which reuses `launch::{effective_spec, emit_outcome}` (no copy). Neither file defines a reconcile-step function.

## Behaviour (what F implements and T tests)

Every port call goes through `Ports`; nothing types into a pane (no `send_text`, no `send_keys`: I4), nothing runs a shell, and
no argv is joined or split. Before every step marked *live* below, the engine checks the budget (`options.budget` since the
engine was entered); when it has run out it stops, rolls back as for a failed step, and answers
`Timeout { op: OP_LAUNCH | OP_RELAUNCH }` (exit 1).

### `launch NAME`, the CLI (`launch.rs`), before the engine

1. `PaneName::parse(&args.name)` (`usage`, exit 2, on a bad name). `args.spec.validate()` (E-2: `grid-ambiguous`,
   `grid-out-of-range`, `profile-secret-refused`, `env-name-invalid`, `command-not-argv`, `usage`; exit 3 or 2).
   `ProfileName::parse` on `--profile` (`usage`).
2. With `--profile P`: `ports.profile_store.get(&P)`; `None` is `ProfileNotFound { what: P }` (exit 3) and nothing else is
   called. The base is P's entry whose `pane` equals NAME, if any.
3. `effective_spec(base, &name, &values)` (decision 10): a missing required field, a bad `--model` (not `PROVIDER/ID` with
   both halves non-empty), `--ctx-soft` above `--ctx-hard`, a port policy `port_of_policy` refuses, or `--expect` with no
   check, is `usage` (exit 2), one line naming every missing or bad flag.
4. `tx_launch::launch(ctx.ports, &request, &TxOptions::default())`, then `emit_outcome`.

### `tx_launch::launch`, live (not `spec_only`), in this order

| # | Step | Port call(s) | On failure |
|---|---|---|---|
| 0 | The spec names the pane (also with `spec_only`) | none | `spec.pane != name` -> `usage` (2) "the spec names <spec.pane>, not <name>". Nothing is called. |
| 1 | The name is free | `pane_store.get(name)` | `Some` -> `Refused { PANE_EXISTS }` (3). Nothing else is called. |
| 2 | Herdr session given | none | `None` -> `usage` (2) "--herdr-session is required unless --spec-only" |
| 3 | Probe (only when `spec.check` is `Some`) | `prober.run_probe(check, &spec.expect, options.probe_timeout)` | `Failed { missing }` -> `ProbeFailed { message: "missing \"a\", \"b\"" }`; `Error(r)` -> `ProbeFailed { message: r }` (3). A failing result is **not** stored (the issue: a failing probe "changes nothing"; decision 8). A passing one (`Ok`) is kept in memory and written only at R, as `probe.last: Some(Ok)`; with no check, `probe.last` is `None`. |
| 4 | Herdr version (B5) | `herdr.version()` | the error as is (`herdr-version-unsupported`, 3) |
| 5 | Target cell | `herdr.snapshot()`, `pane_store.list()` | an **occupant** is any `p` in `snapshot.panes` with `p.session == herdr_session && p.workspace == spec.herdr.workspace && p.grid == spec.herdr.grid` (a per-pane filter on the fields each `HerdrPane` carries, B-2a/C-1; the snapshot itself is not keyed). Any occupant -> `Refused { GRID_OCCUPIED }` (3); the message names the occupant's id and the record that names it, or says no record does (decision 12; the join rule is under "The occupant's record" below). So the pane A1 makes is always this run's (`created`). |
| 6 | Port free | `harness.health(port)` | `true` -> `Refused { PORT_IN_USE }` (3), never adopted (H-2) |
| 7 | Edit P and act (I8) | `scope.edit_spec(profile.as_ref(), &name, &SpecEdit::Set(spec), &mut act)` | P is written first (B-7, G-1). Its errors are returned as is: `profile-not-found`, `pane-in-other-profile`, `generation-conflict` (first write), `profile-conflict` (restore conflicted). |

**The occupant's record** (step 5's message): a record `r` from `pane_store.list()` names an occupant `p` when
`r.herdr.session == p.session && r.herdr.pane_id == p.pane_id` (`Pane.herdr` is a `HerdrPane`, C-5, so a record carries the
Herdr session as well as the id; the pane id alone is the join key within one Herdr session). A linear scan of the list is
enough. The message names, for each occupant in snapshot order, its id and every record naming it in `list()` order (sorted by
name, F-3), comma-separated; with none it says no record names it. Two records naming one Herdr pane should not exist; if they
do, both are named and the answer is still `grid-occupied` (the duplicate is doctor's, #647). Illustrative wording (F's
choice; a test asserts only that the occupant's id and the record's name, or the "no record" phrase, appear): `r2c1 in main
holds Herdr pane w1:p2, recorded as demo-c2r1`.

**The two predicates are two steps, on purpose.** (1) *Whether* the launch is refused is decided by the snapshot alone: the
occupant predicate (session, workspace, grid) over `snapshot.panes`; no record is read for it, and a cell with no occupant is
free whatever any record says. (2) *How* the refusal is worded is the join (session, pane id) from each occupant found in (1)
to the records; it never decides a refusal. The join compares no position: a record whose `herdr.session` and
`herdr.pane_id` equal the occupant's is named **even when its stored `herdr.workspace` or `herdr.grid` differs from
`spec.herdr`** (a stale record: the Herdr pane is where the snapshot says, G-3 "a pane's position comes from Herdr's
snapshot", and the stale position is doctor's to report, #647). Conversely, a record whose stored cell equals `spec.herdr`
but whose pane id no occupant has is not named and refuses nothing (AC 9b pins both).

Steps 0-6 make no write and no live change, so a refusal there leaves P and the registry exactly as they were, generation
included. The **act** (the closure; `acted` becomes true when `edit_spec` enters it, so a budget `timeout` always carries the
reconcile step, as ADR-0021 section 12 asks):

| # | Step (live) | Port call | On failure |
|---|---|---|---|
| A1 | Herdr pane | `herdr.ensure_pane(&HerdrSpec { session, workspace, grid })` -> `hp` | roll back, the error |
| A2 | tmux session | `host.ensure_session(&name, &spec.host.cwd)` | roll back, the error |
| A3 | The pane's command (decision 6; only when `spec.command` is `Some`) | `host.run(&name, &command)` | roll back, the error |
| A4 | Harness server | `harness.serve(&name, port)` -> `pid` | roll back, the error (a frozen server answers `timeout`) |
| A5 | Health | `harness.health(port)` | `false` -> roll back, `Unavailable { what: "the harness server on port N is not healthy after it was started" }` (1). Nothing after A5 runs: no session exists, so no order can reach the pane. |
| A6 | Session of record | `harness.create_session(port)` -> `sid` | roll back, the error |
| A7 | TUI | `harness.attach_tui(&hp.pane_id, port, &sid)` | roll back, the error (`session-not-found` is 3) |
| O1 | Observe SHOWN | `harness.shown_session(&hp.pane_id)` | not `Some(sid)` -> roll back, `Unavailable { what: "the TUI of NAME shows <shown or 'no session'>, not the session of record <sid>" }` (1) (decision 14) |
| O2 | Observe the Herdr pane | `herdr.snapshot()` | `hp.pane_id` not listed at `grid` -> roll back, `Unavailable { what: "the Herdr pane <id> of NAME is gone" }` (1) |
| R | Record | `pane_store.cas_put(&pane, 0)` | the error as is (`generation-conflict`, 1); **no rollback** (another writer owns the record now; ADR-0021 section 8 "writes nothing more") |

**Rollback** (best effort, in this order, each only if its step ran): `host.stop_owned(&name)` (A2 ran); `herdr.close(&hp.pane_id)`
(A1 ran and `created`; a `pane-not-found` from `close` counts as closed, since a vanished pane is already gone). A rollback
call that fails does not hide the step's error: the message gains "; rollback failed: <code>".
**What a rollback leaves, by design** (no port call ends a tmux session or deletes a harness session, Out of scope): the tmux
session A2 made **stays**, with its processes stopped (`FakeHost::sessions()` still holds the name, `FakeHost::ps(name)` is
`[]`); a session A6 created stays in the port's data directory. If `stop_owned` fails, the session's processes stay too (the
message names the code); if `close` fails, the Herdr pane stays. These are the expected observable states after a failed
launch, reported by doctor (#647, Risks 2), and AC 8 asserts the first.
Then `edit_spec` restores P's specs (F-5), the record is not written, and the CLI prints the reconcile step (`acted`).

**The record written at R**: `name`; `herdr` = `hp`; `host` = `{ name: HOST_NAME, tmux: name, cwd: spec.host.cwd,
herdr_api_version: Some(<step 4's string>) }`; `harness` = `{ kind: Opencode, port, pid: Some(pid), health: Healthy }`;
`session_of_record: Some(sid)`; `role`; `hold: None`; `last_observed: { shown: Some(sid), driven: None, at: (options.now_ms)() }`
(`shown` is O1's observation; `driven` is never inferred, C-14, decision 22); `profile: request.profile`; `model`, `env`,
`context`, `command` from the spec; `probe: { check: spec.check, expect: spec.expect, last: spec.check.as_ref().map(|_| ProbeResult::Ok) }` (`ProbeResult::Ok`, D-1, not `Result::Ok`).

**`spec_only`**: `scope.edit_spec(Some(P), &name, &SpecEdit::Set(spec), &mut || Ok(()))` and nothing else: no probe, no Herdr,
host, harness or pane-store call by the engine, no record. `Launched { pane: None, profile: Some(P) }`.
**Where the `spec_only` branch sits** (the order, exactly): step 0 (`spec.pane == name`, `usage` otherwise) runs first; then,
when `spec_only`, `profile` is `None` -> `usage` (the engine's re-check of the clap rule, `acted: false`, nothing called);
then the `edit_spec` call above, and the engine returns. **Steps 1-6, the act (A1-O2) and R do not run with `spec_only`**:
in particular step 1's `pane_store.get` is not made by the engine, so `--spec-only` on a name that already has a record is
not `pane-exists` (it edits P's spec and changes nothing live). The only pane-store call in a `spec_only` run is the scope's
own read of the pane's record inside `edit_spec` (F-5: it "reads P ... and n's record"; `pane-in-other-profile` comes from
there), never a `List`, a `CasPut` or a `Delete`.

### `relaunch NAME`, the CLI (`relaunch.rs`), before the engine

1. As launch's step 1. Then `ports.pane_store.get(&name)`: `None` is `PaneNotFound { what: name }` (3).
2. With `--profile P`: `profile_store.get(&P)` (`profile-not-found`, 3). Base = P's entry for NAME if any, else
   `profile_snapshot::spec_from_pane(&record)` (#662a, I-1). Without `--profile`: base = `spec_from_pane(&record)`.
3. `effective_spec(Some(&base), ...)`; `grid_given` = `--grid` was given. The CLI checks no relaunch rule itself: the engine
   does (E0), so every caller (#664's `apply` included) is held to them.
4. `tx_launch::relaunch(...)`, then `emit_outcome`.

### `tx_launch::relaunch`, live

**E0, the relaunch rules, the engine's first plan step** (no port call; nothing before it): `spec.pane != record.name` ->
`usage` "the spec names <spec.pane>, not <name>" (also with `spec_only`). Then, unless `spec_only` (decision 11; with it
nothing live changes, so P may drift on purpose, AC 16g):
- the effective `host.cwd` differs from `record.host.cwd` -> `usage` "relaunch cannot change a pane's directory; close it and
  launch it again";
- the effective cell differs from the record's and `!grid_given` -> `usage` naming both positions. "Both positions" means the
  **whole** position on each side, whether one field or both differ: the record's `record.herdr.workspace` and
  `record.herdr.grid`, and the effective `spec.herdr.workspace` and `spec.herdr.grid` (grids in the `rRcC` form, D-4), for
  example `relaunch moves a pane only with --grid: the record is at main r2c1, the flags give other r2c1`. `--workspace`
  without `--grid` is one instance of this rule, not a separate one.

A "cell" is the pair (workspace, grid) in the record's Herdr session. **The move** is derived, never passed: the run moves
the pane exactly when the effective cell differs from the record's (which E0 allows only with `grid_given`); `--grid` naming
the record's own cell is no move.

Steps 3 (probe), 4 (version) as launch; step 5 (target cell) as launch with `herdr_session = record.herdr.session` and the
target = the effective cell (effective workspace and grid), where the record's own pane is not "occupied": a `p` with
`p.session == record.herdr.session && p.pane_id == record.herdr.pane_id` is dropped from the occupants before the check (the
same join key as "The occupant's record"); step 6 (port free) only when the effective port differs from `record.harness.port`.
Then `edit_spec(...)` with this act:

| # | Step (live) | Port call | On failure |
|---|---|---|---|
| B1 | Stop what the pane owns | `host.stop_owned(&name)` | the error (record unchanged) |
| B2 | Old server gone | `harness.health(record.harness.port)` | `true` -> `Unavailable { what: "the harness server on port N still answers after the pane's processes were stopped (#695)" }` (1); nothing else runs |
| B3 | Herdr pane | `herdr.ensure_pane(<effective cell>)` -> `hp` | roll back, the error. Without a move this is the record's cell: it answers the record's pane, or, if that pane vanished (F-16), makes a new one there, whose id is recorded. |
| B4-B7 | as A2-A5 | | as A2-A5 |
| B8 | Session of record (decision 8) | `harness.list_sessions(port)`; `create_session(port)` only when the record has no session of record or the list lacks it | roll back, the error. B8 fixes this run's `sid`: **kept** = `record.session_of_record` when it is `Some(s)` and the list (at the effective `port`) contains `s`; otherwise **new** = the id `create_session` returns. B9, O1 and R all use that one `sid`. |
| B9, O1, O2 | as A7, O1, O2, with B8's `sid` (B9 attaches to it; O1 expects `shown_session == Some(sid)`) | | as there |
| R | Record | `pane_store.cas_put(&pane, record.generation)` | as launch's R |
| B10 | Old Herdr pane (only on a move) | `herdr.close(&record.herdr.pane_id)` when step 5's snapshot listed it and it is not `hp`; `pane-not-found` counts as closed | see below: it never fails the act |

**B10 runs after R, on purpose** (decision 23). Before R, a failed close would roll the run back and skip R while the new
pane already holds the TUI, so the record would keep naming the old pane and cell. After R the record names the pane that
holds the TUI, and the old pane is a leftover with no record, which doctor (#647) reports. B10 is the act's last call, runs
even when the budget has run out (one bounded call, after the record), and its error is **captured, not returned by the
act**: the act answers `Ok`, so `edit_spec` keeps P's edit. The engine then returns `Err(TxFailure { error: Unavailable {
what: "relaunched NAME at <cell>, but its old Herdr pane <id> was not closed (<close's code>: <close's message>)" }, acted:
true })`, with the record and P already stored: exit 1 whatever the close's own code (decision 14: the live state is not
what the verb needs), and the reconcile step is appended.

The record written keeps `name`, `hold`, `last_observed.driven` (decision 22) and (without `--profile`) `profile` from
`record`, and takes everything else as launch does (`last_observed.shown` is O1's `sid`). With `--profile P` it names P.
**`spec_only`** is as launch's (after E0's `spec.pane` check), in the same order: E0's `spec.pane` check, then `spec_only`
with `profile: None` -> `usage` (the engine's re-check of the clap rule, `acted: false`, as for `LaunchRequest`), then
`edit_spec(Some(P), &record.name, &SpecEdit::Set(spec), &mut || Ok(()))`; E0's cwd and cell rules, steps 3-6, B1-B10 and R do
not run, and the engine makes no port call of its own (the scope's own read of the record inside `edit_spec`, F-5, is
the scope's). A relaunch that fails after B1 and before R leaves the record
unchanged, although its processes were stopped: that is what the issue asks ("leaves the record unchanged"), and doctor
(#647) reports the pane as down.

### Output (`emit_outcome`)

- **Data** (JSON `data`; the envelope is `output::emit`'s): `{"verb": "launch" | "relaunch", "pane": <Pane> | null,
  "profile": null | {"name": <P>, "slug": <slug>, "generation": <n>}, "spec_only": <bool>}`. The pane's grid serializes through
  `GridPos` (`{"row": 2, "col": 1, "pos": "r2c1"}`, D-4).
- **Text**, one line on stdout:
  - `launched demo-c1r1 at r2c1 in main: session ses_…, harness port 48100 (pid 20000)` (`relaunched` for relaunch), then
    `; profile "demo" is at generation 2` when P was named;
  - `--spec-only`: `updated the spec of demo-c1r1 in profile "demo" (generation 2); nothing live changed`.
- **Errors**: `ErrorBody { code: error.code(), message }` (E-3), `message` = the error's own text, plus, when `acted` and
  the text does not already contain the step, `"; " + step`, where step is `profile_scope::reconcile_step(Some(&P))` (I-3) when the
  run named P, else `profile_scope::reconcile_step(None)`. So a `profile-conflict`, restore failure or first-write `timeout` from the
  real scope (I-3, decisions 5-7) prints the scope's step once, and an act error the scope returned unchanged (I-3, "case 9")
  gets it from the verb. Text mode writes `error: <message>` to stderr; JSON mode one envelope; the exit code is `class_of`'s
  in both (D-5).

## Acceptance criteria (each observable: a named test, a command, or a grep with its expected output)

Tests live in `crates/holler-cli/tests/pane_verbs/launch.rs` and `relaunch.rs` (the `pane_verbs` target, E-9) and run with
`cargo test -p holler-cli --test pane_verbs`. They use only the test-kit fakes (and, in AC 16k only, #663's `StoreScope`
over them, I-4) and `run_verb_with` (E-7), through a rig in `launch.rs` (`pub(crate) mod rig`) that relaunch.rs reuses
(decision 25: #643 and #647 plan rigs over the same fakes; no shared rig exists on `main` today, so if `crate::list::rig` or
`Rig` (#643) is on the `main` this run starts from, T builds on its fakes and adds only the linked host and the hooks;
otherwise this rig stays private to `launch.rs`/`relaunch.rs` and the consolidation is a follow-up):

- `FakePaneStore` and `FakeProfileStore` behind `Arc` (the profile store seeded with `sample_profile("demo", &[])`, so P is at
  generation 1 with no spec), `FakeProfileScope` over both (actor `holler pane`), `FakeHerdr::new("scratch")` with workspace
  `main` of 3 by 3 (absolute placement), `FakeHarness` behind `Arc`, `FakeProber`, and a **linked host**: a `HostPort` wrapper
  over `FakeHost` whose `stop_owned(name)` also calls `FakeHarness::kill(p)` for each rig port p (48100-48102) whose
  `server(p).name` is `name` (F-8 recommends exactly this);
- hook wrappers over the harness and Herdr fakes that run a test closure after a named method returns (to freeze a server
  after `serve`, vanish a Herdr pane at `attach_tui`, make another writer `concurrent_put` a record, or panic);
- `assert_matches(name)`: the registry equals what the fakes observe (AC 2); `assert_no_keystroke()`: `FakeHerdr`'s call log
  holds no `SendText` and no `SendKeys`; `assert_untouched()`: the Herdr, host, harness and prober logs are empty and neither
  store's log holds a `CasPut` or `Delete`.

`LAUNCH` below is `pane launch demo-c1r1 --herdr-session scratch --project /srv/demo --workspace main --grid r2c1 --model
demo-provider/demo-model --effort medium --ctx-soft 100000 --ctx-hard 150000 --port-policy fixed:48100`. Every test ends with
`assert_no_keystroke()`, and every test except AC 6 and AC 7 ends with `assert_matches`. **A call-log assertion counts only
the calls the verb run made**: the rig records each log's length before the run (a test's own setup, such as serving a port or
placing a Herdr pane through a fake's port method, is logged too and is not part of the assertion). The fakes' scenario,
configuration and inspection methods (`FakeHarness::{kill, freeze, thaw, delete_session, server, tui}`, `FakeHerdr::vanish`,
the stores' `concurrent_put`) bypass the faults and the call log (J-2, F-4, F-16, F-21), so a call log never shows them: the
linked host's `kill` on `stop_owned` adds no harness entry, and no test asserts a log entry for a scenario method.

1. **Happy path** (`launch_records_what_the_fakes_show`): `LAUNCH` exits 0, stderr empty, stdout one line containing `r2c1`
   and the session id. The record (generation 1) has `session_of_record == Some(sid)` where `harness.list_sessions(48100)` is
   exactly `[sid]` (**no ping session**); `harness.pid == Some(FakeHarness::server(48100).pid)`; `herdr.pane_id` is the id
   `FakeHerdr::snapshot()` lists at `r2c1`; `FakeHarness::tui(pane_id).shown == Some(sid)`; `last_observed.shown ==
   Some(sid)` and `last_observed.driven == None` (never inferred, decision 22); `harness.health == Healthy`; `host == {
   name: "localhost", tmux: "demo-c1r1", cwd: "/srv/demo", herdr_api_version: Some(PROTOCOL_22_VERSION) }` (F-17). The harness call log is exactly `[Health, Serve,
   Health, CreateSession, AttachTui, ShownSession]` and the Herdr one `[Version, Snapshot, EnsurePane, Snapshot]`.
2. **The registry equals the fakes** (`assert_matches`, applied at the end of every case named above): with a record, the
   Herdr snapshot lists `record.herdr.pane_id` at `record.herdr.grid`, `FakeHarness::server(port)` is `Running` with
   `record.harness.pid`, the TUI of the pane shows `session_of_record`, and `FakeHost::sessions()` holds the name; with no record,
   the snapshot holds no pane this run created, no rig port has a `Running` server served for the name, and `FakeHost::ps(name)`
   is empty or `pane-not-found`.
3. **C1 recorded** (`launch_records_model_effort_env_and_ceilings`): `LAUNCH` with `--model p1/m1 --effort high --env ALPHA_TOKEN
   --env BETA_URL --ctx-soft 1000 --ctx-hard 2000 --role orchestrator` stores `model == {p1, m1, high}`, `env == [ALPHA_TOKEN,
   BETA_URL]`, `context == {1000, 2000}`, `role == Orchestrator`.
4. **A wedged server aborts before any order is possible.**
   a. `launch_onto_a_frozen_server_times_out_before_any_session`: the rig serves 48100 for `demo-c1r1` and freezes it, then
      `LAUNCH` exits 1 with code `timeout`; the harness log holds no `CreateSession` and no `AttachTui`; no record.
   b. `launch_aborts_when_the_server_never_gets_healthy`: a hook freezes 48100 right after `serve`; exit 1, code
      `unavailable`; no `CreateSession`, no `AttachTui`; the Herdr pane this run created is closed (`Close` in the log); no record.
5. **A vanished Herdr pane.**
   a. `launch_fails_when_its_herdr_pane_vanishes`: a hook vanishes the new pane (F-16) when `attach_tui` returns; exit 1,
      code `unavailable`, message names the pane id; no record.
   b. `relaunch_recreates_a_vanished_pane_at_its_cell`: after `LAUNCH`, the rig vanishes the pane; `pane relaunch demo-c1r1`
      exits 0; the record's pane id is the new one the snapshot lists at `r2c1`, the grid is unchanged, the Herdr log of the
      relaunch holds no `Close`.
6. **A stale generation.**
   a. `relaunch_fails_on_a_stale_generation`: a hook makes another writer `concurrent_put` the record (F-4) during the act;
      exit 1, code `generation-conflict`; stderr ends with `; to reconcile, run holler pane doctor` (no profile named); the
      stored record is the other writer's.
   b. `launch_record_conflict_fails_loudly`: a hook makes another writer create `demo-c1r1`'s record during the act; exit 1,
      `generation-conflict`, stderr ends with `; to reconcile, run holler pane doctor`; the live pane, tmux session and server are left
      (ADR-0021 section 8: "writes nothing more"). AC 6 and AC 7 are the only cases where the registry may differ from the
      fakes, and each hands the difference to doctor (#647) through the printed reconcile step or the leftovers.
7. **A crash between steps** (`a_crash_mid_launch_leaves_no_record`): a hook panics inside `serve`; the run, inside
   `std::panic::catch_unwind(AssertUnwindSafe(..))`, is `Err`; `pane_store.get(demo-c1r1)` is `None` and the pane store's log
   holds no `CasPut`; the snapshot lists the new Herdr pane at `r2c1` and `FakeHost::sessions()` holds `demo-c1r1`: exactly the
   "Herdr pane and tmux session with no record" a doctor run (#647) reports (C-10).
8. **A failed step rolls back, with a named error** (`a_failed_attach_rolls_back`): `fail_next(AttachTui, SessionNotFound)`;
   exit 3, code `session-not-found`; stderr carries `to reconcile, run holler pane doctor` once; the Herdr log holds `Close` of the created pane and the
   snapshot is empty; `FakeHost::ps(demo-c1r1)` is `[]` while `FakeHost::sessions()` still holds `demo-c1r1` (the tmux
   session is a known leftover: no port call ends it, "What a rollback leaves"); `server(48100).state == Killed`; no record.
9. **Name, cell and port guards** (each exits 3 and leaves every store unwritten):
   a. `launch_of_a_recorded_name_is_pane_exists`: the store holds `demo-c1r1`; code `pane-exists`; `assert_untouched()`.
   b. `launch_refuses_a_cell_another_record_holds`: `demo-c2r1`'s record names the Herdr pane at `r2c1`; code
      `grid-occupied`; the message contains that Herdr pane's id and `demo-c2r1` (the join rule of "The occupant's record");
      the Herdr log is `[Version, Snapshot]`; no host or harness call. The same with `demo-c2r1`'s record naming that
      Herdr pane's id but a stale stored grid (`r3c1`): still `grid-occupied`, and the message still names `demo-c2r1`
      (the join compares session and pane id only). `launch_ignores_a_stale_record_at_a_free_cell`: a record
      (`demo-c2r1`) whose stored `herdr.grid` is `r2c1` but whose pane id the snapshot does not list, and no Herdr pane at
      `r2c1`: `LAUNCH` exits 0 (the cell is free by the snapshot).
   c. `launch_never_adopts_an_unrecorded_pane`: a Herdr pane exists at `r2c1` with no record; code `grid-occupied`, the
      message says no record names it; that pane is still listed (never closed); no host or harness call.
   d. `launch_never_adopts_a_running_server`: 48100 runs for `demo-c2r1`; code `port-in-use`; the harness log is `[Health]`.
10. **Probe** (B1):
    a. `a_failing_probe_refuses_before_any_step`: `LAUNCH --profile demo --check-arg curl --check-arg http://127.0.0.1:8095/v1/models
       --expect qwen38`, the prober scripted `Failed { missing: ["qwen38"] }`; exit 3, `probe-failed`, the message contains
       `qwen38`; the Herdr, host and harness logs are empty, the pane store holds no record, P is still at generation 1.
    b. `an_unscripted_probe_refuses`: no script (F-18: `Error`); exit 3, `probe-failed`.
    c. `a_passing_probe_is_recorded`: scripted `Ok`; exit 0; `record.probe == { check: Some([...]), expect: ["qwen38"],
       last: Some(Ok) }`; the prober's one call has `timeout == PROBE_TIMEOUT` and the given argv and expect.
11. **Argv, never a shell** (B2):
    a. `a_command_string_is_command_not_argv`: `LAUNCH --command-json '"opencode serve"'` (the JSON string) exits 3 with
       `command-not-argv`; `assert_untouched()`.
    b. `the_command_reaches_the_host_as_argv`: `LAUNCH --command-arg prog --command-arg 'a b' --command-arg '$(id);x'`; exit 0;
       `FakeHost::runs() == [(demo-c1r1, ["prog", "a b", "$(id);x"])]`; `record.command` is the same argv. Without a command,
       `runs()` is empty.
12. **Secrets never echoed** (I7): `an_env_value_is_refused_and_not_echoed`: `LAUNCH --env TOKEN=s3cr3t644`, in text and JSON;
    exit 3, `profile-secret-refused`; `s3cr3t644` is in neither stdout nor stderr; `assert_untouched()`.
13. **Grid** (decision 7 of the epic):
    a. `every_grid_form_reaches_herdr_as_row_2_col_1`: for `c1r2`, `r2c1` and `2,1` (fresh rig each): exit 0; the snapshot lists
       the pane at `GridPos { row: 2, col: 1 }`; `record.herdr.grid` is the same; text stdout contains `r2c1`; JSON
       `data.pane.herdr.grid == {"row": 2, "col": 1, "pos": "r2c1"}`.
    b. `an_ambiguous_grid_is_refused_before_any_step`: `--grid 21`; exit 3, `grid-ambiguous`; `assert_untouched()`.
    c. `a_cell_outside_the_workspace_is_out_of_range`: `--grid r9c1`; exit 3, `grid-out-of-range` (from `ensure_pane`); no record.
14. **Herdr version** (B5): `an_unsupported_herdr_is_refused`: `set_version(Unsupported)` (J-3); exit 3,
    `herdr-version-unsupported`, the message contains `SUPPORTED_VERSIONS`; the Herdr log is `[Version]`.
15. **Required flags**: `launch_names_every_missing_flag`: `pane launch demo-c1r1 --herdr-session scratch`; exit 2, `usage`,
    one line naming `--project`, `--workspace`, `--grid`, `--model`, `--effort`, `--ctx-soft`, `--ctx-hard`, `--port-policy`;
    `assert_untouched()`. `--port-policy fixed` (no port), `--port-policy fixed:048100` and `--ctx-soft 2 --ctx-hard 1` are
    each `usage`, exit 2. `a_live_launch_needs_a_herdr_session`: `LAUNCH` without `--herdr-session`; exit 2.
    `port_policy_round_trips_with_the_snapshot` (engine, decision 5): for p in {1, 80, 48100, 65535},
    `port_of_policy(&profile_snapshot::fixed_port_policy(p)) == Ok(p)`; `port_of_policy("fixed:0")` and `("fixed:65536")` are
    `usage`.
16. **Profiles** (I3, I8):
    a. `launch_with_profile_adds_the_spec_and_bumps_once`: `LAUNCH --profile demo`; exit 0; P at generation 2 holds exactly
       one spec, equal to the effective spec (`pane: "demo-c1r1"`, workspace `main`, grid `r2c1`, cwd `/srv/demo`, harness
       `{opencode, "fixed:48100"}`, the model, role `agent`, ceilings); `record.profile == Some("demo")`; P's last log entry
       is at generation 2.
    b. `relaunch_with_profile_and_model_updates_the_spec`: after (a), `pane relaunch demo-c1r1 --profile demo --model
       demo-provider/other-model`; exit 0; P at generation 3, its spec's `model_id == "other-model"`, the record's too.
    c. `a_failed_act_restores_the_profile_specs`: `LAUNCH --profile demo` with `fail_next(AttachTui, ..)`; P's specs equal the
       seeded (empty) list, at generation 3 (G-8), and its log shows the edit and its reversal; no record.
    d. `a_refusal_leaves_the_profile_at_its_generation`: AC 10a's case: P at generation 1, the profile store's log holds no
       `CasPut`.
    e. `a_missing_profile_is_refused`: `LAUNCH --profile nope`; exit 3, `profile-not-found`; `assert_untouched()`.
    f. `without_profile_no_profile_is_touched`: `LAUNCH`; the profile store's call log is empty.
    g. `spec_only_changes_the_profile_and_nothing_live`: `pane launch demo-c1r1 --profile demo --spec-only` with the spec flags
       and no `--herdr-session`; exit 0; P at generation 2 holds the spec; the Herdr, host, harness and prober logs are empty;
       the pane store holds no record, and its log holds only the scope's own `Get` of `demo-c1r1` (F-5): no `List`, no
       `CasPut` (the engine skips steps 1-6, "Where the `spec_only` branch sits"); stdout says `nothing live changed`. The same for `pane relaunch demo-c1r1 --profile
       demo --spec-only --model demo-provider/m2` on a launched pane: P gains the spec, the record and every fake are unchanged.
    h. `a_profile_conflict_after_the_act_fails_loudly`: `scope.before_next_restore(..)` makes another writer
       `concurrent_put` P (F-6, F-21), and `fail_next(AttachTui, ..)`; exit 1, code `profile-conflict`; the message contains
       `profile_scope::reconcile_step(Some(&demo))` (I-3: `to reconcile, run holler pane doctor --profile 'demo' and then holler
       profile show 'demo'`), and `to reconcile, run` occurs in it **exactly once** (no second step of either form); no record.
    i. `relaunch_refuses_a_pane_of_another_profile`: the record of `demo-c1r1` names profile `other` (seeded too);
       `pane relaunch demo-c1r1 --profile demo`; exit 3, `pane-in-other-profile`; the host and harness logs are empty (only the
       plan's Herdr reads ran), and neither store is written.
    j. `the_reconcile_step_quotes_the_profile`: AC 16h's case with a profile named `it's` (seeded); the message contains
       `holler pane doctor --profile 'it'\''s'` and `holler profile show 'it'\''s'` exactly once each (the quoting is #663's
       function's; this pins that the verb uses it and adds no quoting of its own).
    k. `a_step_the_real_scope_printed_is_not_repeated` (C-15): the rig with `holler_cli::pane::profile_scope::StoreScope::new`
       (I-4) over the rig's two `Arc` stores (actor `holler pane`) in place of `FakeProfileScope`; a hook after `serve` makes
       another writer `concurrent_put` P, and `fail_next(AttachTui, ..)`; exit 1, `profile-conflict`; the message contains
       `reconcile_step(Some(&demo))`, and `to reconcile, run` occurs exactly once (the scope's own step, I-3 decision 7; the verb
       appended none). With the same rig and no conflict (only the `fail_next`), the restore succeeds, the error is the act's
       (`session-not-found`, exit 3) and `to reconcile, run` again occurs exactly once (the verb's).
17. **JSON and exit codes** (`exit_codes_equal_across_formats`): for the cases of AC 1, 8, 9a, 10a, 11a, 13b, 15, 16e and 6a,
    the JSON run's stdout passes `check_envelope(stdout, code)` (F-20), its stderr is empty, and its exit code equals the text
    run's.
18. **Relaunch keeps the position without `--grid`** (`relaunch_without_grid_keeps_the_position`): after `LAUNCH`,
    `pane relaunch demo-c1r1` exits 0; `record.herdr` (id and grid) is unchanged; the relaunch's Herdr log holds no `Close`.
19. **Relaunch with `--grid` moves** (`relaunch_with_grid_moves_the_pane`): `pane relaunch demo-c1r1 --grid c1r3`; exit 0; the
    record's grid is `r3c1` with the new id the snapshot lists there; the old id is closed and gone from the snapshot; stdout
    contains `r3c1`. `pane relaunch demo-c1r1 --workspace other` (no `--grid`) and `--project /srv/other` are `usage`, exit 2,
    with `assert_untouched()` after the CLI's read; the `--workspace other` message contains `main`, `other` and `r2c1` (both
    whole positions, the record's and the effective one). These usage refusals now come from the engine's E0, with the same
    text, so this AC is unchanged.
    b. On the engine, as #664 would call it (`tx_launch::relaunch(rig.ports(), &request, &TxOptions::default())` with the
       record of a launched pane): `relaunch_engine_refuses_a_cell_change_without_grid` (spec at `r3c1`, `grid_given: false`),
       `relaunch_engine_refuses_a_cwd_change` (spec `host.cwd` `/srv/other`) and `relaunch_engine_refuses_a_spec_for_another_pane`
       (`spec.pane` `demo-c2r1`) each return `Err(TxFailure { error: Usage { .. }, acted: false })` with `assert_untouched()`.
       With `spec_only: true` and a profile, the first two are accepted (P gains the drifted spec, nothing live), the third is
       still `usage`. `launch_engine_refuses_a_spec_for_another_pane`: the same for `tx_launch::launch`.
       `engine_refuses_spec_only_without_a_profile`: `tx_launch::launch` and `tx_launch::relaunch`, each with `spec_only:
       true` and `profile: None` (a state the request types can express though clap cannot), return `Err(TxFailure { error:
       Usage { .. }, acted: false })` with `assert_untouched()` and the pane store's log empty.
    c. `relaunch_records_the_move_before_closing_the_old_pane` (decision 23): `pane relaunch demo-c1r1 --profile demo --grid
       c1r3` after `LAUNCH --profile demo`, with `fail_next(HerdrOp::Close, Unavailable { .. })`; exit 1, `unavailable`; the
       message contains `was not closed` and the old pane's id, and the reconcile step once; the record (generation 2) names
       the new pane at `r3c1`; P holds the spec at `r3c1` (not restored); the old pane is still listed by the snapshot.
20. **Relaunch and the session of record** (decision 8): `relaunch_keeps_the_session_of_record`: the record's session is the
    same id after relaunch and `list_sessions(48100)` still has exactly one; `relaunch_replaces_a_deleted_session`: after
    `FakeHarness::delete_session(sid)`, relaunch records a new id and `CreateSession` is in the log. In both, the `sid` B8
    fixes (kept in the first, new in the second) is the one B9 attaches, the one `FakeHarness::tui(pane_id).shown` holds after
    the run, and `record.session_of_record == record.last_observed.shown == Some(sid)`; in the first, the relaunch's harness
    log holds no `CreateSession`. `relaunch_keeps_the_stored_driven` (decision 22): before the relaunch, another writer
    `concurrent_put`s the record with `last_observed.driven = Some("ses_driven_by_hub")` (F-4); after it, the record's
    `last_observed.driven` is still `Some("ses_driven_by_hub")`; after a plain launch then relaunch it stays `None`.
21. **Relaunch stops only what the pane owns** (`relaunch_leaves_other_panes_alone`): `demo-c1r1` on 48100 and `demo-c2r1`
    (`--grid r1c2 --port-policy fixed:48101`) are launched; relaunching `demo-c1r1` leaves `demo-c2r1`'s `ps`, its server pid
    (still `Running`) and its record (generation) unchanged; `server(48100)` has a new pid.
22. **Until #695: a surviving server fails loudly** (`relaunch_fails_when_the_old_server_survives`): the rig with a plain
    `FakeHost` (not linked); exit 1, `unavailable`, the message contains `still answers`; the record's generation is unchanged.
23. **Relaunch of a missing pane**: `relaunch_of_a_missing_pane_is_refused`: exit 3, `pane-not-found`; `assert_untouched()`.
24. **Budget** (I5, decision 17), on the engine: `the_budget_bounds_a_slow_launch`: `tx_launch::launch(rig.ports(), &request,
    &TxOptions { budget: 300 ms, .. })` with `FakeHarness::faults().set_delay(Some(200 ms))` returns
    `Err(TxFailure { error: Timeout { op: "pane.launch" }, acted: true })` in under 3 s, with no record and the created Herdr
    pane closed.
    **The request and the call sequence the bound is derived from.** `request` is what `LAUNCH` builds: name `demo-c1r1`,
    `herdr_session: Some("scratch")`, cell `main r2c1`, port 48100, **no check, no command, no profile**, `spec_only: false`.
    The delay is on the harness fake's switch only: `FaultSwitch::enter`, which every port method of `FakeHarness` calls
    first, records the call and sleeps (J-2); the test sets no delay on any other fake (the prober is not called: no check); the scenario and inspection methods (`kill`, `server`) bypass the faults and the call log (J-2). So the run is,
    with t the time since the engine was entered: steps 0-5 (no harness call, t ~ 0); step 6 `health(48100)` (delayed,
    t ~ 200 ms, answers `false`); `edit_spec(None, ..)` enters the act; budget check before A1 (t ~ 200 < 300, passes),
    A1 `ensure_pane` (not delayed); check before A2 (passes), A2 `ensure_session`; A3 skipped (no command); check before A4
    (t ~ 200, passes), A4 `serve` (delayed, t ~ 400 ms); **check before A5 (t ~ 400 >= 300): `timeout`**. Rollback:
    `host.stop_owned` (the linked host's extra `FakeHarness::kill(48100)` is a scenario method: no delay, no log entry) and
    `herdr.close` of A1's pane (not delayed); no profile, so no restore. Expected wall time is about 400 ms (two delayed
    calls), each budget check has about 100 ms of margin on either side, and 3 s is over seven times the expected time. The
    test also asserts that sequence: the harness log of the run is exactly `[Health, Serve]` (no `Health` after `Serve`, no
    `CreateSession`, no `AttachTui`), and the Herdr log of the run is `[Version, Snapshot, EnsurePane, Close]` with `Close`
    naming the pane `EnsurePane` returned; the snapshot then holds no pane.
25. **No keystroke, no shell, in the source**:
    `grep -nE 'send_text|send_keys' crates/holler-pane/src/tx_launch.rs crates/holler-cli/src/pane/launch.rs crates/holler-cli/src/pane/relaunch.rs`
    prints nothing; `grep -nE '"(sh|bash|zsh)"|"-c"|Command::new|std::process' <the same three files>` prints nothing.
26. **The surface.** `cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process` passes
    (the fixture, ADR 0003 rows and stub table are updated as under Files). `grep -nE '"(re)?launch", 644' crates/holler-cli/tests/pane_verbs/process/stub.rs`
    prints nothing, and `grep -c '^    // #644$' crates/holler-cli/tests/pane_verbs/process/stub.rs` prints `1`.
27. **ADR-0021 updated** (decision 20): `grep -n 'PROPOSED (#644' docs/adr/ADR-0021.md` prints the section 12 note;
    `grep -n 'Launch and relaunch as built (#644)' docs/adr/ADR-0021.md` prints the section 8 note; `grep -n 'open (#644)'
    docs/adr/ADR-0021.md` prints the section 9 row; `grep -n '#644, after a contract amendment' docs/adr/ADR-0021.md` still
    prints its "Deferred" bullet (left as is: the deferral stays open until the operator confirms); and
    `git diff origin/main -- docs/adr/ADR-0021.md` touches no line of sections 3, 11 or the "Deferred to #647" paragraph, nor the
    "Deferred" bullets of #647 and #662.
28. **CHANGELOG**: `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry linking
    `[#644](https://github.com/Performant-Labs/holler/issues/644)`; `bash scripts/changelog-check.sh` prints `changelog-check: ok`.
29. **No new `unsafe`, no new dependency**: `git diff origin/main -- crates | grep -nE '^\+.*\bunsafe\b'` prints nothing;
    `git diff --stat origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` prints nothing.
30. **Quality gates**: `rustfmt --check --edition 2021 crates/holler-pane/src/tx_launch.rs crates/holler-cli/src/pane/launch.rs
    crates/holler-cli/src/pane/relaunch.rs crates/holler-cli/tests/pane_verbs/launch.rs crates/holler-cli/tests/pane_verbs/relaunch.rs`
    exits 0 (C-12: a workspace-wide `cargo fmt --check` already fails on `main`); `cargo clippy --workspace --all-targets -- -D
    warnings` exits 0; `cargo test -p holler-pane`, `cargo test -p holler-cli --test pane_verbs` and `cargo test --workspace`
    pass; `bash scripts/lint.sh` passes (every `#[allow]` carries `// #644`; no file at 900 lines); every new function is under
    clippy's 100-line and complexity-15 thresholds.
31. **Public repo**: S checks the diff and the PR text against the operator's private list of personal host, tailnet and account
    names (not reproduced here); the test data uses only the neutral names in the header.

## Files

Production:
- `crates/holler-pane/src/tx_launch.rs` (fill; the API above). The issue's blast radius.
- `crates/holler-cli/src/pane/launch.rs`, `crates/holler-cli/src/pane/relaunch.rs` (replace the stubs). The issue's blast radius.

Tests (the verb story's own, epic ruling 2):
- `crates/holler-cli/tests/pane_verbs/launch.rs` (replace the stub cases; keep `pane_launch_accepts_every_spec_flag`, which
  still holds because a missing positional is "accepted", E-8), with `pub(crate) mod rig`.
- `crates/holler-cli/tests/pane_verbs/relaunch.rs` (replace the stub cases; keep `pane_relaunch_accepts_every_spec_flag`).

Surface and docs (assigned to the verb story by the epic or the file itself):
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: delete `("pane", "launch", 644)` and `("pane", "relaunch", 644)`, keep
  `// #644` (E-10).
- `crates/holler-cli/tests/fixtures/cli-surface.txt`, the `# #644` group (E-11): every line gains the positional `demo-c1r1`,
  every flag stays covered, and one line adds `--herdr-session scratch`, e.g. `pane launch | demo-c1r1 --herdr-session scratch
  --project /srv/demo --workspace main --grid r2c1 --model provider/model-id --effort high --role agent`.
- `docs/adr/ADR-0003.md` rows 48-49 (E-12), e.g. `holler pane launch PANE [--herdr-session NAME] [SPEC FLAGS] [--profile NAME]
  [--spec-only]    #644` and `holler pane relaunch PANE [SPEC FLAGS] [--profile NAME] [--spec-only]  #644` (J-4: the bare `PANE`
  parses as a string, E-13).
- `docs/adr/ADR-0021.md` (decision 20).
- `CHANGELOG.md` (AC 28).

**Not touched** (frozen or another story's): `holler-pane/src/{lib,pane,profile,ports,probe,error,grid,argv}.rs`,
`holler-pane/src/{profile_snapshot,profile_diff}.rs` (#662a's), the test kit, `holler-cli/src/{cli,lib,main,output}.rs`,
`pane/{mod,args,wiring,profile_scope}.rs` (`profile_scope.rs` is #663's), `tests/pane_verbs/main.rs`, `tests/verb_harness/**`,
every `Cargo.toml` and `Cargo.lock`.

### Reuse map (extend, do not duplicate)

| Object | Use | Extend or new |
|---|---|---|
| `Ports` and every port trait (B-1 to B-7) | the engine's only effects | reuse, unchanged |
| `ProfileScope::edit_spec` (B-7) | the I8 transaction; the verb never writes P itself (ADR-0021 section 5) | reuse |
| `SpecFlags::validate`, `SpecValues`, `ProfileOpt`, `SpecOnly` (E-1, E-2) | the flags and their guards | reuse; `effective_spec` adds only the overlay and the required-field and range checks validate does not make |
| `GridPos`, `Argv`, `EnvVarName`, `PaneName`, `ProfileName` | parsing and the guards' codes | reuse |
| `output::{emit, emit_error, ErrorBody, VerbCtx}` (E-3 to E-5) | all printing and exit codes | reuse; no table of codes |
| `holler_pane::error::{class_of, RefusalCode}` (D-5, D-6) | the three open codes; the exit class | reuse |
| `ProbeResult`, `Prober` (D-1, B-6) | the probe | reuse; the engine calls `ports.prober`, never the free `run_probe` |
| `next_generation` | not called: the stores apply it (F-3 for the pane store; the real scope writes only the profile, J-1) | n/a |
| `launch::{effective_spec, emit_outcome}` | shared by relaunch.rs | new in launch.rs, used by both (no copy in relaunch.rs) |
| `profile_snapshot::spec_from_pane` (#662a, I-1) | relaunch's base | reuse; no second Pane-to-spec mapping anywhere (ADR-0021 section 3) |
| `profile_snapshot::{FIXED_PORT_POLICY_PREFIX, fixed_port_policy}` (#662a, I-1) | `port_of_policy`'s prefix; its round-trip test | reuse; `port_of_policy` is the grammar's one parser, with no `"fixed:"` literal of its own |
| `profile_scope::reconcile_step` (#663, I-3) | the step with a profile | reuse; no quoting helper here |
| `profile_scope::StoreScope` (#663, I-4) | AC 16k only | reuse in a test |
| `holler_proto::clock::now_millis` (H-3) | `TxOptions::default().now_ms` | reuse; no clock code here |
| the test kit's fakes, `sample_profile`, `check_envelope`, `run_verb_with` (F-1 to F-21, E-7) | every test | reuse; the rig's linked host and hooks are test-local wrappers, as F-8 recommends; #643's rig if it is on `main` (decision 25) |

## Decisions already made (O)

1. **No operation id in this story: PROPOSED, pending the operator's confirmation** (C-2). Both options in ADR-0021 section 12
   need frozen files outside this blast radius, and the spikes (H-1) show a launch is short: health 630-740 ms after spawn,
   the TUI shows its session in 1.6-1.7 s, a restart healthy in about 645 ms, so a launch is about 3 s of a 10 s bound. I5 is
   met by a per-run budget (decision 17), the per-call bounds and a loud `timeout`. "Neither option yet" is not one the
   operator listed (G-5), so ADR-0021 records it as **PROPOSED (#644)** and its "Deferred" bullet stays; the MO asks the
   operator, and on a yes a one-line follow-up marks it decided. Nothing in the code depends on the answer.
2. **Where the work is.** The engine (`tx_launch.rs`) is pure over `Ports` (ADR-0021 section 5: "the pure transaction engines
   ..., which work only through the ports"); the verbs parse, build the effective spec, call the engine with
   `TxOptions::default()`, and print. Both verbs call `edit_spec` with or without a profile (`None` runs only the act, F-7), so
   there is one code path.
3. **`host.name` is `"localhost"`** (`HOST_NAME`): epic decision 3 puts Herdr, tmux and the harness on the hub's machine;
   reading the real host name would need a dependency or `unsafe`, and would put a machine name into records and output.
4. **The Herdr session is a launch flag**, `--herdr-session NAME` (C-6; verb-local flags are the owning story's, epic ruling 2),
   required for a live launch, ignored with `--spec-only`. Relaunch takes it from the record. It is not stored in a profile
   spec (`ProfileSpec` has no field for it); #664 needs its own source (Forward-compat).
5. **Port policy grammar: `fixed:<port>` only** (C-7). The form and its prefix are #662a's (`FIXED_PORT_POLICY_PREFIX`,
   `fixed_port_policy`, I-1, I-2); the grammar beyond it is this story's, and today it is that one form, parsed by
   `port_of_policy` (1..=65535, canonical decimal), so a spec says which port its server uses, `--from-current`'s output is
   launchable as is, and `port_of_policy(&fixed_port_policy(p)) == Ok(p)`. Any other text, including the bare `fixed` of the
   test kit's `sample_spec`, is `usage`. No allocator ("next free port") is built here.
6. **`command` is an extra argv run in the pane's tmux session** (C-4): when the spec has one, `host.run(name, command)` runs it
   after `ensure_session` and before `serve`, and it is recorded. The harness server itself is started only by
   `HarnessPort::serve` (ADR-0021 section 11, G-4), whose adapter owns OpenCode's own argv. `stop_owned` stops the command on
   rollback and on relaunch.
7. **Model, effort, env names and ceilings are recorded, not applied** (C-5): no frozen port carries them, so the record says
   what the pane was launched with (C1) and applying them to the OpenCode process is a follow-up for #642/#649 (for example the
   adapter's serve environment from the record). The brief does not invent a port parameter.
8. **Relaunch keeps the session of record** (C-11) when the restarted server still lists it (`list_sessions`, observed, never
   "the most recent"); otherwise (no session of record, or it was deleted) it creates one through the API. A new conversation
   is `reset`'s (#645). A **failing probe's result is not stored**: a refusal changes nothing.
9. **"P unchanged"** (C-3): a refusal before the profile write (steps 0-6 and E0, the CLI's checks) leaves P exactly as it was,
   generation included (AC 16d); a failed act leaves P's specs equal and its generation two higher (G-8, AC 16c).
10. **The effective spec** (`effective_spec`): the base (P's entry for the pane, or, for relaunch, #662a's
    `profile_snapshot::spec_from_pane(record)`, I-1: the one mapping, so `profile show` and `apply` never see false drift
    between two copies) with
    every given flag replacing its field; a repeatable flag given at least once (`--env`, `--expect`, `--command-arg`,
    `--check-arg`) replaces the whole list; `--role` defaults to `agent` for a new spec. With no base, `--project`,
    `--workspace`, `--grid`, `--model`, `--effort`, `--ctx-soft`, `--ctx-hard` and `--port-policy` are required. `--model` is
    split at its first `/` and both halves must be non-empty; `--ctx-soft` must not exceed `--ctx-hard`; `--expect` needs a
    check. Each of these is `usage` (exit 2), one line naming every bad or missing flag.
11. **Relaunch moves a pane only with `--grid`, and the engine enforces it** (G-3, issue): without it the effective workspace
    and grid must equal the record's (`usage` otherwise, naming both); `--workspace` alone is refused that way. A different
    `--project` is refused (the tmux session keeps its directory, #641's `ensure_session`, and a record claiming another would
    be false). Both rules, and `spec.pane == record.name`, are E0, the engine's first plan step, not CLI checks: #664's `apply`
    calls the engine with specs that `--spec-only` let drift (AC 16g), and a rule held only by one caller would let another
    record a false cwd or leave the old Herdr pane open and unrecorded. The request carries `grid_given`, not a computed
    "move" flag that could contradict the spec; the move is derived (the cell differs). The codebase's pattern agrees: the
    scope runs its own guards before any write (F-5), and the engine already re-checks `spec_only`. With `--spec-only` the cwd
    and cell rules do not apply: nothing live changes.
12. **An occupied target cell is refused, never adopted** (`grid-occupied`, 3), whether or not a record names the occupant:
    a pane no record names may be a crash's leftover or a pane a person made before cutover (#654), and recording it would let a
    later `close` (#646) close a pane Holler did not make. This matches "never adopt a running server" (H-2). #647's plan
    reports such a leftover with no remedy, so today the operator frees the cell by hand in Herdr (C-10, follow-up for #647).
    Relaunch treats the record's own pane at its own cell as its pane, not as an occupant. Rollback
    closes only a pane the pre-act snapshot did not hold.
13. **No keystroke, and no Herdr-to-tmux link here** (C-8): the engine never calls `send_text` or `send_keys` (I4; AC 25 and
    every test). Making the Herdr pane show the tmux session needs a port call the contract lacks; it is a follow-up for
    #640/#649 (for example `ensure_pane` starting the client by `layout.apply` argv, H-1), not built here.
14. **A mismatch observed after the act is `unavailable`** (G-7): I3 says exit 1, and an open code is a refusal (exit 3), so it
    must be a closed failure code. Of `generation-conflict`, `profile-conflict`, `timeout`, `unavailable`, `store-corrupt`,
    `not-implemented` and `profile-drift`, only `unavailable` fits "the live state the verb needs is not there"
    (`profile-drift` is a reconcile finding about profiles). The message names what was expected and what was seen. #645
    should use the same; ADR-0021 records it.
15. **The reconcile step: one function, one owner, one doctor form, one appending rule** (C-15, C-16).
    - *Function and owner:* #663's `profile_scope::reconcile_step(Some(&P))` (I-3), called as is. #644 adds no function and no
      quoting of its own (the profile name's POSIX quoting is #663's, AC 16j).
    - *Doctor form:* #663's, exactly: `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show
      '<P>'` when the run named P; else `profile_scope::reconcile_step(None)` = `to reconcile, run holler pane doctor` (the same ADR 0003
      row, `holler pane doctor [--profile NAME]`, with the optional group dropped; it parses today). Neither names the pane:
      `pane doctor` has no pane positional until #647; naming it then is a change to #663's function (follow-up), not here.
    - *Who appends:* the scope adds the step to its own `profile-conflict`, restore-failure and first-write-`timeout` errors
      (I-3, decisions 5-7); the verb appends `"; " + step` to a failure with `acted` only when the message does not already
      contain that exact step (the same function's output, so the check is exact). So the act's error that the scope returns
      unchanged after a successful restore (I-3, "case 9"), a record `generation-conflict`, a budget `timeout` and B10's
      failure get the verb's step; the scope's own errors keep theirs; nothing is printed twice. A first-write
      `generation-conflict` (`acted: false`, nothing live moved) gets none. AC 16h, 16j and 16k pin it (16k over the real
      `StoreScope`, because the fake adds no step and could not show a duplicate). When #663's follow-up F1 makes the fake add
      the step too, the rule still holds.
16. **A record conflict after the act is not rolled back** (ADR-0021 section 8: "fails loudly, exits 1, writes nothing more");
    another writer owns the record and reconcile is the safety net. With a profile, `edit_spec` still restores P's specs,
    because the act returned an error.
17. **The budget**: `DEFAULT_BUDGET` (10 s, I5) from entering the engine, checked before every live step; when it runs out the
    run rolls back and answers `timeout` with `op` `pane.launch` or `pane.relaunch`. The worst case is the budget plus one port
    call's own bound plus the rollback calls; every port call is itself bounded (B-1 to B-6 docs).
18. **The probe** runs after the name check and before every Herdr, host and harness call and before the profile write, with
    `PROBE_TIMEOUT` (5 s). `Failed { missing }` and `Error(reason)` are both `probe-failed` (3); only the operator's own expected
    strings or the prober's reason are quoted, never the probe's output.
19. **Three open codes, all refusals (exit 3)**: `pane-exists`, `grid-occupied`, `port-in-use`, declared in `tx_launch.rs`
    (C-13). The closed list is not edited (ruling 3).
20. **ADR-0021 is updated in this change, only in lines no in-flight story edits** (F, about 25 lines). #647 (in flight) edits
    section 11's end (after lines 452-453), section 12's "Deferred to #647" paragraph (lines 470-471) and removes the
    "Deferred" bullet at line 524; #662a (merged first) rewrites section 3's lines 153-154 and removes the bullets naming
    #662; 662b adds `profile-conflict` to the `profile create` and `profile delete` rows of section 9. So this story edits:
    (a) **section 12**: one new paragraph after line 468 (the operator's paragraph), headed "**PROPOSED (#644, pending the
    operator's confirmation):**", with decision 1 and the timings; line 469 (blank) separates it from #647's 470-471;
    (b) **section 9, row 338** only (G-2), in the table's convention (`unavailable` and `timeout` are common, not repeated): it adds
    "; open (#644) for a name that already has a record, an occupied target cell and a port a server already answers on"
    (`pane-exists`, `grid-occupied`, `port-in-use`);
    (c) **section 8**: a short "**Launch and relaunch as built (#644).**" paragraph after line 305 recording decisions 4, 6-8,
    11, 12, 14, 15, 22 and 23; for the port policy it cites section 3's `fixed:<port>` sentence (#662's) and adds only that
    launch accepts that one form, never restating it;
    (d) **the mismatch-code bullet** of "Deferred" (lines 531-532), marked decided in place ("decided by #644: `unavailable`,
    section 8; #645 to follow"). The operation-id bullet (line 523) is **not** edited: the deferral stays open while decision 1
    is PROPOSED, and the line is adjacent to #647's line 524.
    **Formatting trap:** `docs_cli_test` (E-13) parses every inline `holler ...` code span under `docs/`. `pane doctor` takes
    no pane positional until #647, and a placeholder such as `'<P>'` may not parse as written, so the ADR text names the
    reconcile commands in prose or inside a fenced block whose language is not `text`/`bash`/`sh`/`shell`/`console` (for
    example ` ```none `), never as an inline `holler pane doctor ...` span. `cargo test -p holler-cli --test docs_cli_test`
    (AC 26) catches a slip.
21. **Test seams** are the fakes and test-local wrappers only (F-8): the linked host, the hooks, `catch_unwind` for a crash, and
    `TxOptions` for the budget. No production test-only switch, no sleep-based race: AC 24 asserts an invariant (`timeout`, no
    record) and a generous upper bound.
22. **`last_observed.driven` is never inferred** (C-14; the frozen `LastObserved` doc, C-4; ADR-0021 section 1 and I6). Launch
    writes `driven: None`; relaunch keeps `record.last_observed.driven` as stored; `shown` is O1's observation in both. The
    issue's "confirm SHOWN equals DRIVEN" is checked as O1's SHOWN == `session_of_record` (I2) until #649 wires DRIVEN, the
    reading #647's plan uses. Writing `driven` would need an amend-first change to the frozen contract.
23. **Relaunch records before it closes the old Herdr pane** (B10 after R). Reason: the record must name the pane that holds
    the TUI. Failure semantics: a failed close never fails the act (P keeps its edit, the record is stored) and the run exits 1
    with `unavailable` naming the old pane, and the reconcile step; the old pane is a leftover with no record, which doctor reports. The
    other order would leave the record naming the old pane while the new one holds the TUI, with P restored: a wrong record,
    which is worse than an extra pane.
24. **The lost-update window on P is an accepted risk.** The CLI reads P for the base (relaunch step 2, launch step 2) and
    `edit_spec` reads P again before its compare-and-swap (F-7). The CAS on P's generation guards the window between
    `edit_spec`'s own read and write; it does **not** guard the earlier window (the CLI's read to `edit_spec`'s read, which
    the probe alone can stretch to 5 s), because `SpecEdit::Set` carries no expected generation and the frozen trait cannot
    take one. In that window another writer's change to **this pane's** entry is overwritten with no conflict (other panes'
    entries are kept: `Set` replaces one entry in place). Accepted because both writes are in P's log (who and when), the
    overwriting spec is the one the operator just asked for, and closing it needs a trait change; follow-up with #663 for an
    expected generation on the edit (Risks 6).
25. **The test rig stays this story's own unless a shared one is on `main`.** #643 (`crate::list::Rig`) and #647
    (`doctor/rig.rs`) plan rigs over the same fakes in the same test target, and none is on `main` today. If #643's rig is on
    the `main` this run starts from, T builds on it and adds only the linked host and the hooks; otherwise the rig is private
    to `launch.rs`/`relaunch.rs` (`pub(crate) mod rig`), #645 and #646 reuse whichever rig is on `main` when they start, and
    a follow-up consolidates the rigs into one module. No fourth copy is planned here.
26. **#662a and #663 first** ("Dependencies"). The alternative, #644 writing #662a's `profile_snapshot` API into its own blast
    radius, would make two stories edit one file at once. Strict order, and no copy of either API here.

## Forward-compat (the consumers of this story)

| Consumer | Needs | Satisfied |
|---|---|---|
| #649 wiring | the verbs over real ports; `TxOptions::default()` | yes; but #649 must (a) make #642's resolvers answer for a pane before its record exists (C-9), from the `PaneName` and `PaneId` the engine passes; (b) close the Herdr-to-tmux link (C-8); (c) apply model, effort and env (decision 7) |
| #664 `profile apply` | create or relaunch a pane from a spec | `tx_launch::launch` and `relaunch` take a complete spec and enforce the relaunch rules themselves (E0, decision 11), so apply passes `grid_given` and gets `usage` for a cwd change; apply must supply the Herdr session (decision 4) and specs whose `port_policy` is `fixed:<port>` (decision 5) |
| #662 (`--from-current`, `show`) | one Pane-to-spec mapping | a dependency, not a consumer: #644 calls `spec_from_pane` and the prefix (I-1) and writes no second mapping, so `diff_spec` and relaunch agree |
| #663 / #646 | one reconcile step | #644 calls `profile_scope::reconcile_step` and appends only where the scope did not (decision 15); #646 can follow the same rule |
| #647 doctor | the leftovers of a crash, a record conflict or a failed B10; the reconcile step's command | the leftovers are observable (AC 6b, 7, 19c); the step names `holler pane doctor [--profile '<P>']`, which parses today. **Gap:** #647's plan reports an unrecorded Herdr pane (`unregistered-herdr-pane`) with no remedy, and launch refuses its cell (decision 12), so no verb frees it: follow-up for #647 |
| #645, #646 tests | a rig over the same fakes | whichever rig is on `main` when they start (decision 25) |
| #645 switch/reset | the mismatch code | `unavailable` (decision 14) |
| #695 host adapter | relaunch stops the old server through `stop_owned` | relaunch checks it (B2) and fails loudly until #695 lands (AC 22) |
| #646 close | a record with `host.tmux`, `harness.pid`, `herdr.pane_id` | yes |

## Out of scope

- The real adapters and their wiring (#640, #641, #642, #649); `pane doctor` (#647); `close`, `park` (#646); `switch`,
  `reset` (#645); `profile apply` (#664).
- An operation id and an executor of long work (decision 1); a port allocator; stopping a TUI; deleting a harness session on
  rollback (no port call exists); ending a tmux session (no port call exists).
- Any edit to a frozen file listed under Files, to the test kit, or to any manifest.
- Touching any live system: OpenCode servers, Herdr, tmux, the operator's sessions.

## Follow-ups (the orchestrator files them; none blocks this story)

- **Herdr pane to tmux session** (C-8): a contract amendment so `ensure_pane` (or a new call) starts the pane's client by argv.
- **Apply model, effort and env to the harness** (decision 7): #642/#649.
- **A session delete for rollback**: a failed attach leaves the created session on the server; reconcile (#647) reports it as a
  stray until a `HarnessPort` call can delete it. The same call would remove the old server's session left by a relaunch that
  changes the port (Risks 9).
- **Test kit**: `sample_spec`'s `port_policy` `"fixed"` is not launchable under decision 5; align it with `fixed:48100` when #664
  needs it. `FakeHarness::serve`'s re-serve of the same pane (H-2) is already a known divergence.
- **The operation id**: the operator confirms or rejects decision 1's PROPOSED note; on a yes, a one-line edit marks it
  decided and drops its "Deferred" bullet (after #647's removal of line 524 has merged). If a measured launch later exceeds the
  budget, options (a) and (b) stay on record.
- **A cell a crash left** (#647; plan-review finding 8): no verb frees a Herdr pane with no record, and launch refuses its cell.
  #647 (or #650's `pane import`) should offer the way out, for example a doctor remedy that closes an unrecorded pane the
  operator names.
- **An expected generation on `edit_spec`** (#663; decision 24): close the lost-update window between a verb's read of P and
  the scope's.
- **The reconcile step names the pane** once #647's `pane doctor [PANE]` lands (#663's function; decision 15).
- **One `pane_verbs` rig** (decision 25): fold #643's, #644's and #647's rigs into one shared module.

## Test plan

**RED first (T).** A compile error is not RED (`docs/agent-overlays/tester.md`). T first checks the dependencies on the
`main` the branch was rebased onto (Dependencies): `grep -n 'pub fn spec_from_pane(pane: &Pane) -> ProfileSpec\|pub const
FIXED_PORT_POLICY_PREFIX: &str = "fixed:"\|pub fn fixed_port_policy(port: u16) -> String'
crates/holler-pane/src/profile_snapshot.rs` prints three lines, and `grep -n 'pub fn reconcile_step(profile: Option<&ProfileName>)
-> String\|pub struct StoreScope\|pub fn new(' crates/holler-cli/src/pane/profile_scope.rs` prints each; anything else is a
stop (`preflight-failed`). Then T lands, before the tests:
- `tx_launch.rs` with every public item of the API above: the constants with their values, `TxOptions` and its `Default`, the
  request, outcome and failure types, `From<PaneError> for TxFailure`, and stub bodies with no logic: `launch` and `relaunch`
  return `Err(TxFailure { error: PaneError::NotImplemented, acted: false })`, `port_of_policy` returns
  `Err(PaneError::NotImplemented)`. Each stub carries `// stub (#644 RED): F fills`.
- `launch.rs` and `relaunch.rs` with the new `Args` (positional `PANE`; `--herdr-session` on launch) and `run` still answering
  `not_implemented(644)`; the fixture lines, ADR 0003 rows and the `stub.rs` deletion (Files), so the surface tests stay green.
  The `pub(crate)` items (`effective_spec`, `emit_outcome`, `Verb`) are F's: a stub of one with no
  caller would trip `dead_code = "deny"`; the tests that need the unscoped text write the literal.

Then every behaviour test fails on its assertion (exit 1 `not-implemented` where 0, 2 or 3 is expected; AC 24 on the stub's
`NotImplemented`), while AC 26 and the kept flag tests pass. T journals the RED run in `handoff-T-red.md` with each failing
test and its first assertion.

**GREEN (T verify).** `cargo test -p holler-cli --test pane_verbs` (all of AC 1-24), then AC 25-30 as written. T-green also runs
each new test 20 times (`for i in $(seq 20); do cargo test -p holler-cli --test pane_verbs -q -- launch:: relaunch:: || break; done`)
to surface a flake, and confirms the process tests (`pane_cli_process`) still pass with the positional in place.

## Security-sensitive spots (called out for the reviewers)

- **Argv splitting.** A command or check reaches a port only as an `Argv` (D-2, E-2): `--command-arg` elements are kept one per
  element, `--command-json` must be a JSON array (a string is `command-not-argv`, AC 11a), and the engine never joins, splits
  or re-quotes an element. `HostPort::run` and `Prober::run_probe` get the argv exactly (AC 10c, 11b).
- **Shell and format expansion.** Nothing here runs a shell (AC 25). Values that reach tmux (`--project` as the session's cwd,
  the command's elements) are passed **raw**: escaping tmux's `;` and `#` is the host adapter's job (#641's Decision 13), and
  pre-escaping here would double-escape. The one string a person pastes, the reconcile step, comes from #663's
  `reconcile_step`, which quotes the only free-text part (the profile name) for a POSIX shell (I-3); this story adds no
  quoting of its own (decision 15, AC 16j).
- **Secrets in output.** Env entries are names only; `NAME=value` is refused by `EnvVarName` without echoing it (AC 12). No
  message quotes a probe's output, a command's elements or an env value; it may quote session ids, pane ids, ports and the
  operator's own `--expect` strings, none of which is a secret. No credential exists in any test.
- **Public repository.** Only the neutral names of the header appear in code, tests, docs, the CHANGELOG and the PR (AC 31).
- **No live system.** Every test runs over the fakes; the real binary in the process tests still runs over `Unwired` (E-6), so
  `holler pane launch` on the pipeline host cannot reach the operator's Herdr, tmux or OpenCode until #649 wires them.

## Risks

1. **Contract gaps the fakes hide** (C-5, C-8, C-9): on the fakes a launch is complete; on real adapters the Herdr pane does not
   show the tmux session, the model is not applied, and #642's resolvers cannot answer before the record exists. #644 goes
   green and #649 is where these surface. They are named as follow-ups and in Forward-compat so they are not lost.
2. **Leftovers after a failed attach**: the created OpenCode session and the tmux session cannot be removed through the ports,
   and the TUI respawned into the tmux session is not one of `stop_owned`'s processes (#641's ownership rule). Reconcile reports
   them; the reconcile step is printed every time.
3. **The FakeHarness and the real adapter differ on `serve`** (H-2): the fake re-serves the same pane; the real one refuses a
   port in use. Step 6 refuses before `serve` in both, and relaunch's B2 checks that the old server is gone, so the verb behaves
   the same over either.
4. **Clap's error order**: the shared process test `command_arg_and_command_json_are_mutually_exclusive` (#670's file,
   `tests/pane_verbs/process/usage.rs`) runs `pane launch` with no `PANE` (J-5). Clap 4 validates conflicts before required
   arguments, so its conflict message should still win, and `spec_only_requires_profile` only needs `--profile` in the message.
   T-red confirms both with `cargo test -p holler-cli --test pane_cli_process`. If either fails, T stops and reports it rather
   than editing #670's shared test (an edit outside the blast radius needs the MO's decision).
5. **The budget's upper bound** is the budget plus one call's own bound plus rollback calls, not a hard 10 s. Accepted and
   documented in ADR-0021 (decision 17).
6. **Two readers, two guards**: the record is read by the CLI and written with that generation, so a writer in between is
   caught by the record's compare-and-swap (AC 6a), after the live change. P is read twice too, and only the second read is
   guarded: a lost update of this pane's entry in P is possible and accepted (decision 24).
7. **The dependencies move under this brief**: #662a's and #663's APIs are pasted from reviewed but unmerged briefs (I).
   T's first RED step re-verifies them and stops on a difference (Test plan).
8. **A vanished pane's cell on the real Herdr adapter** (B3, AC 5b): relaunch without a move calls `ensure_pane` at the
   record's cell, and on the fake a vanished pane's cell is free, so a new pane is made there (F-15, F-16). Whether the real
   `HerdrPort::ensure_pane` (provisional until #640 closes it, B-3: "make a pane exist at `spec.grid` by issuing right/down
   splits, or fail loudly") can recreate a pane at a cell a vanished pane freed, rather than failing loudly because the
   splits that made it no longer apply, is not shown by any evidence here. #644 relies on the port's contract as written;
   #640's conformance suite or #649's wiring is where a divergence surfaces, and a failure there is loud (B3 rolls back).
9. **A port change on relaunch leaves the old server's session** (B8): when the effective port differs from
   `record.harness.port`, the old server is stopped (B1, B2) but its sessions stay in its data directory (F-10), and the new
   server lists none, so B8 creates a new session of record. No port call deletes the old one; it is a leftover doctor (#647)
   reports, as the rollback's created session is (Follow-ups: "A session delete for rollback" covers this case too).
10. **Size**: the two test files are the largest; the rig's wrappers keep each case short. The 800-line fallback is in the size
   check.
