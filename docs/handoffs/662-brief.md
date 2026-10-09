# Brief: #662 profile-verbs (`holler profile create | delete | list | show`, the snapshot and the spec-versus-live diff)

Repo: Performant-Labs/holler. Issue: #662 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**THIS RUN IS 662a ONLY (decided by the MO, 2026-10-09).** Implement only the criteria tagged (a) and the 662a row of the Size check; 662b (`create`, `delete`) is a later run on the same branch name, started from `origin/main` after 662a merges. The PR says `Part of #662`, not `Closes #662`; the issue closes with 662b. Where this brief says "unless the MO keeps one run", the MO has decided: split.

**Branch:** `issue-662-implementation` (worktree `.claude/worktrees/0662-profile-verbs`, from `origin/main` at `3bdd129`).
**Review-rigor:** second-opinion, set by the operator through the MO (2026-10-09; the outside model is deepseek-v4-pro). The
issue's own Pipeline line says `rigor: in-session`; the MO's instruction raises it (see "Contradictions found", C1).
**Forward-compat:** done, see the table under "Files". **Design:** N/A (no UI surface).
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 3, 8 and 9, and its "Deferred to named stories" list, two of
whose items name #662 (the `port_policy` that `--from-current` writes; what makes a profile's pane "live"). This brief decides
both (Decisions 3 and 4) and makes a three-place doc edit to ADR-0021 to record them (Decision 14).
**Handoffs:** `docs/handoffs/662/handoff-<phase>.md`; the decision journal is `docs/handoffs/662/decisions.md`.
**Public repository:** no host, tailnet or account name appears in code, tests, docs, commits or the PR. Test data uses the
test kit's neutral names (`demo-c1r1`, `scratch`, `/srv/demo`, `demo-provider`).

## Size check

**F's file cap trips; a split is proposed (the MO decides).** One component family (the profile verbs and their two pure
helpers), but F edits **six production files** (four verb files, two `holler-pane` files) plus **three mechanical doc files**
(`CHANGELOG.md`, `docs/adr/ADR-0003.md`, `docs/adr/ADR-0021.md`); T edits **eight test files** and the
fixture (662a: `holler-pane/tests/profile_{snapshot,diff}_test.rs`, `profile_verbs/{rig,list,show}.rs`, `stub.rs`; 662b:
`profile_verbs/{create,delete}.rs`, `stub.rs` again). The production size is about 1,000 lines (`profile_diff.rs` ~260, `profile_snapshot.rs` ~70, `show.rs` ~200,
`list.rs` ~90, `create.rs` ~260, `delete.rs` ~150); tests about 1,200.

| Run | Scope | F's files | Closes |
|---|---|---|---|
| **662a** (pure core and read verbs) | `profile_snapshot.rs`, `profile_diff.rs`, `profile/list.rs`, `profile/show.rs`; the `show` row of ADR 0003; the `list`/`show` fixture lines and STUBS entries; ADR-0021 Decision 14 items (i) and (iii); CHANGELOG | 4 code + 3 docs | `Part of #662` |
| **662b** (write verbs) | `profile/create.rs`, `profile/delete.rs`; their ADR 0003 rows; their fixture lines and STUBS entries; ADR-0021 Decision 14 item (ii); CHANGELOG | 2 code + 3 docs | `Closes #662` |

662b uses only 662a's public API (`spec_from_pane`, `profile_from_panes`, `is_member`), so 662a goes first and 662b starts
from `origin/main` after 662a merges. Every acceptance criterion below is tagged **(a)** or **(b)**. If the MO keeps one run,
all of them apply and the PR closes #662.

## Problem

The four `holler profile` verbs that create, delete, list and show a profile are stubs that refuse with
`not implemented (story #662)`, and the two shared helpers they need, the snapshot of a live pane as a `ProfileSpec`
(`profile_snapshot.rs`) and the spec-versus-live comparison (`profile_diff.rs`), are empty files. Without them no profile can be
made from the running panes, so the migration (#650) cannot create its `fleet` profile, `holler profile apply` (#664) has no
comparison to plan from, and the PROPOSED `profile-drift` finding (#665) has nothing to call. This story fills the six files
against the frozen ports, coding only against `PaneStore`, `ProfileStore` and `output::emit()`, and is tested only on the test
kit's fakes (the verbs) and holler-pane's own fixture records (the two pure modules).

## Evidence (verbatim, as of `3bdd129`)

### E1. The issue (the source of truth), whole body

```text
GitHub issue #662 body (title: "feat(cli): holler profile create | delete | list | show")
Part of epic #633. (added 2026-10-08, profiles)

## Scope
Implement the four profile verbs in the stub files #637 pre-creates, coding only against the `ProfileStore`, `PaneStore` and `HerdrPort` traits and `output::emit()`:
- `holler profile create NAME [--from-current | --from PROFILE]`: an empty profile, a copy of another profile's specs (no panes attached; (amended 2026-10-08, review) a **detached copy**: its specs may name panes that belong to another profile, and no `Pane.profile` changes), or `--from-current`, which snapshots the live panes and their Herdr layout into specs and sets each member's `Pane.profile`. (amended 2026-10-08, review) `--from-current` reads everything from PaneStore (position, cwd, harness, model, effort, role, env names, ceilings, and (amended 2026-10-08, features) command, check, expect), which launch and import record, so it needs no Herdr call and does not depend on #636. The snapshot-to-spec mapping lives in `crates/holler-pane/src/profile_snapshot.rs` so the migration (#650) reuses it. A pane already in another profile is refused (`pane-in-other-profile`).
- `holler profile delete NAME [--keep-panes]`: refuses while any pane of NAME is live (`profile-has-live-panes`); `--keep-panes` first sets each member's `Pane.profile` to None by CAS (the panes keep running), then deletes.
- `holler profile list`: name, slug, pane count, live count, generation.
- `holler profile show NAME`: the spec, its live panes, and for each live pane every field where it differs from its spec. The spec-versus-live comparison lives in `crates/holler-pane/src/profile_diff.rs`, which `holler profile apply` (#664) and the PROPOSED doctor code `profile-drift` (#665) reuse; there is no separate `diff` verb (dropped by the operator, 2026-10-08).
- `--format=text|json` on all four via `output::emit()`; every refusal has a stable code (`profile-not-found`, `profile-exists`, `profile-has-live-panes`, `pane-in-other-profile`, `profile-conflict`).
- Positions are `GridPos` (#637): text prints `rRcC`, JSON `{"row", "col", "pos"}` row first; the snapshot takes positions from Herdr's snapshot through the adapter, never from a pane's name. (amended 2026-10-08, grid) (amended 2026-10-08, review) (Correction: from the Pane record's `herdr.grid`, which the adapter observed; never from a pane's name.)
- (amended 2026-10-08, features) `show` reports each pane's last probe result (`ok`, or `failed` with the missing expected string) and shows `command` and `check` as argv arrays.

## Acceptance (test kit only)
- Each verb's happy path and every refusal above, in both formats; every JSON output passes #638's envelope helper; exit codes are equal across formats.
- (amended 2026-10-08, review) `create --from P` makes a detached copy (no `Pane.profile` changes; a spec naming a pane of another profile is kept).
- (amended 2026-10-08, features) `show` reports `ok` and `failed (missing "qwen38")` from fake probe results.
- `create --from-current` on a fake layout yields one spec per live pane with the right `GridPos`, cwd, harness, model and effort, role, env NAMES and ceilings; `delete --keep-panes` leaves every pane running with `profile` None; `show` reports a pane that differs from its spec, field by field (position in rowcol), and reports nothing for a pane that matches.
- No profile ever contains an env value (I7).

Depends on: #637, #638. (amended 2026-10-08, review) (Was also #636: no longer needed, `--from-current` reads PaneStore.) It does **not** wait for the profile registry (#661) or the output module (#660): it codes against the trait and the signatures #637 fixed and the fakes from #638; #649 runs it against the real store.

## Blast radius
- crates/holler-cli/src/profile/create.rs, delete.rs, list.rs, show.rs
- crates/holler-pane/src/profile_snapshot.rs, profile_diff.rs (pre-created as stubs by #637)

## Scheduling
Wave 3, beside the verb stories (it has no dependency on another wave-3 story). The migration #650 starts after it, for `profile_snapshot.rs`.

## Pipeline
`rigor: in-session`.

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it without waiting for another story's code. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius; the shared hot spots in the epic are owned by one story each. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.
```

### E2. The epic's contract lines this story builds on (epic #633 body, section "The contract")

```
epic #633, "The contract": the port and the profile verbs
Port            ProfileStore get / list / cas_put(profile, expected_generation) / delete(name, expected_generation) / watch / log(name)
Helper trait    ProfileScope resolve(profile, pane) / edit_spec(profile, pane, edit, act)   // implemented in holler-cli/src/pane/profile_scope.rs
Profile verbs   holler profile  create NAME [--from-current | --from PROFILE] | delete NAME [--keep-panes] | list | show NAME
                                | apply NAME [--dry-run]                   (#664; REQUIRED, needed by #666; `show` reports spec-vs-live differences, so there is no `diff` verb)
                                  (amended 2026-10-08, features) apply refuses a pane that belongs to another profile unless --take-over (confirmed, C3)
                (PROPOSED, operator to confirm: rename | export | import; doctor code profile-drift; #665)
```
```
epic #633, "The contract": membership and output
                Membership (amended 2026-10-08, review) is Pane.profile only; a ProfileSpec may name a pane that belongs to another profile (a detached spec,
                  e.g. `create --from`); setting Pane.profile on a pane already in another profile is refused: pane-in-other-profile
Output          --format=text|json on every holler pane / holler profile verb and the roster; one envelope:
                  {"schema_version": 1, "ok": true|false, "data": ..., "error": null | {"code": "<stable kebab-case>", "message": "<one line>"}}
                watch emits NDJSON (one envelope per line); json mode prints only the JSON on stdout (diagnostics on stderr);
                exit codes identical in both formats (0 ok, 1 failure, 2 usage, 3 refusal) (amended 2026-10-09, decisions: was "1 refused/failed"; ADR-0021 section 9); every error has a stable code;
                one module holler-cli/src/output.rs (types and emit() fixed by #637), each verb supplies only its data
```

The epic's "Skeleton split" rulings 2, 3 and 4 (verbatim, epic body):
```
epic #633, "Skeleton split", rulings 2-4
2. **One verb, one file, one owning story, including its clap `Args` struct** (positionals and verb-specific flags), its ADR 0003 row, its `cli-surface.txt` line and its own `tests/pane_verbs/<verb>.rs` or `tests/profile_verbs/<verb>.rs`. The frozen shared files declare only the shared flag groups (`SpecFlags`, `ProfileOpt`, `SpecOnly`) and `--take-over`.
3. **Codes are constants in each verb's own file** built through `PaneError::Refused` or the closed `PaneError` set in `holler-pane/src/error.rs`; no verb edits that enum (infrastructure variants `timeout`, `pane-not-found`, `session-not-found`, `store-corrupt`, `unavailable` and `profile-exists`, `profile-has-live-panes` are already in it).
4. **Formatting:** the tree is not rustfmt-clean and CI has no fmt step. Read the rules' "`cargo fmt`" as: new `.rs` files pass `rustfmt --check --edition 2021`; existing files are not reformatted.
```

### E3. The stubs this story replaces

```
crates/holler-cli/src/profile/create.rs:1-19
//! `holler profile create`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Create a profile.
#[derive(Args, Debug)]
pub struct ProfileCreate {}

/// Run `holler profile create`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileCreate, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
```
crates/holler-cli/src/profile/show.rs:1-19
//! `holler profile show`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Show a profile and where live panes differ from it.
#[derive(Args, Debug)]
pub struct ProfileShow {}

/// Run `holler profile show`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileShow, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```

`delete.rs` and `list.rs` are the same 19 lines with `delete`/`list` in line 1, `ProfileDelete`/`ProfileList` as the struct and
`/// Delete a profile.` / `/// List profiles.` as its doc line (both still `const STORY: u32 = 662;`).

```
crates/holler-pane/src/profile_snapshot.rs:1-4
//! Snapshot of a live pane as a `ProfileSpec`, the half of the spec-versus-live
//! comparison that `profile show`, `profile apply` and the `profile-drift` finding
//! share. Empty stub declared by #637 so that no two stories edit `lib.rs`; story
//! #662 fills it.
```
```
crates/holler-pane/src/profile_diff.rs:1-4
//! Difference between a `ProfileSpec` and the live pane it describes, the other half
//! of the comparison `profile show`, `profile apply` and the `profile-drift` finding
//! share. Empty stub declared by #637 so that no two stories edit `lib.rs`; story
//! #662 fills it.
```

The modules are declared, and `lib.rs` is frozen and re-exports nothing from them, so their items are reached as
`holler_pane::profile_snapshot::...` and `holler_pane::profile_diff::...`:
```
crates/holler-pane/src/lib.rs:27-29
//! - `profile_snapshot`, `profile_diff`, `tx_apply`, `tx_launch`, `tx_switch`,
//!   `reconcile`, `findings`, `import` — empty stubs, declared here so that no two
//!   stories edit this file; each names the story that fills it.
```
```
crates/holler-pane/src/lib.rs:50-51
pub mod profile_diff;
pub mod profile_snapshot;
```
```
crates/holler-pane/src/lib.rs:58-75
// Flat re-exports so the common path is `holler_pane::{...}`, the way `holler_proto`
// does it. Nothing here shares a name with a root re-export of `holler_proto`
// (that crate's `Role` is the A2A role; the pane role is `PaneRole`). The error
// vocabulary beyond `PaneError` stays under `error::`.
pub use argv::{Argv, EnvVarName};
pub use error::PaneError;
pub use generation::next_generation;
pub use grid::GridPos;
pub use pane::{HerdrPane, Pane, PaneEvent, PaneId, PaneName};
pub use ports::{
    Cursor, HarnessPort, HerdrPort, HerdrSnapshot, HerdrSpec, HostPort, Key, PaneStore, Ports,
    Prober, SystemProber, Watch,
};
pub use probe::{run_probe, ProbeResult};
pub use profile::{
    Actor, Profile, ProfileChange, ProfileEvent, ProfileLogEntry, ProfileName, ProfileScope,
    ProfileSpec, ProfileStore, ResolvedScope, SpecEdit,
};
```

The dispatcher (frozen by #670; not edited):
```
crates/holler-cli/src/profile/mod.rs:39-51
/// Run a `holler profile` verb and return its exit code (0 ok, 1 failure, 2 usage, 3 refusal).
pub fn run(cmd: &ProfileCmd, ctx: &mut VerbCtx<'_>) -> i32 {
    match cmd {
        ProfileCmd::Create(args) => create::run(args, ctx),
        ProfileCmd::Delete(args) => delete::run(args, ctx),
        ProfileCmd::List(args) => list::run(args, ctx),
        ProfileCmd::Show(args) => show::run(args, ctx),
        ProfileCmd::Apply(args) => apply::run(args, ctx),
        ProfileCmd::Rename(args) => rename::run(args, ctx),
        ProfileCmd::Export(args) => export::run(args, ctx),
        ProfileCmd::Import(args) => import::run(args, ctx),
    }
}
```

### E4. The records (frozen by #637)

```
crates/holler-pane/src/profile.rs:148-220
/// Where a spec places its pane in Herdr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHerdr {
    pub workspace: String,
    pub grid: GridPos,
}

/// The machine side of a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHost {
    /// The project directory or worktree.
    pub cwd: String,
}

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

/// A named, stored set of panes. Every write is a compare-and-swap on
/// [`Profile::generation`] (see [`crate::generation`]).
///
/// On read, the stored `slug` must equal the slug derived from the name: a record
/// where they disagree is corrupt or forged and does not load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawProfile")]
pub struct Profile {
    pub name: ProfileName,
    /// The unique id derived from the name ([`ProfileName::slug`]).
    pub slug: String,
    /// Bumped on every change.
    pub generation: u64,
    pub panes: Vec<ProfileSpec>,
    /// When the profile was created (milliseconds since the Unix epoch).
    pub created: i64,
    /// When it last changed (milliseconds since the Unix epoch).
    pub updated: i64,
}
```
```
crates/holler-pane/src/profile.rs:68-75
    /// The unique id derived from the name: ASCII letters and digits in lower case,
    /// each run of anything else becoming one `-`, with no leading or trailing `-`.
    /// Two display names with the same slug are the same profile as far as
    /// uniqueness goes.
    pub fn slug(&self) -> String {
        slugify(&self.0)
    }
}
```
```
crates/holler-pane/src/pane.rs:91-115
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
crates/holler-pane/src/pane.rs:141-158
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

/// The role of a pane (and of a profile spec): an agent or the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneRole {
    Agent,
    Orchestrator,
}
```
```
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
```
```
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
```
```
crates/holler-pane/src/grid.rs:68-73
impl fmt::Display for GridPos {
    /// The `rRcC` form, row first: `r2c1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}c{}", self.row, self.col)
    }
}
```
```
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
```
crates/holler-pane/src/argv.rs:20-27
/// An argument vector: the program and its arguments, one string each.
///
/// It serializes as a JSON array of strings and reads back from one. Spaces and
/// shell metacharacters inside an element are data, never re-split. A JSON string
/// where an `Argv` is expected is `command-not-argv`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Argv(Vec<String>);
```
```
crates/holler-pane/src/argv.rs:83-105
/// The name of an environment variable: a name only, never `NAME=value`.
///
/// Parsing is the guard (see the module docs). It serializes as a plain string and
/// reads back through [`EnvVarName::parse`], so a record that holds a value does not
/// load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EnvVarName(String);

impl EnvVarName {
    /// Check `text` and keep it as a name.
    ///
    /// - a `=` anywhere: `profile-secret-refused` (the entry carries a value);
    /// - empty, or any whitespace or control character: `env-name-invalid`.
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        if text.contains('=') {
            return Err(PaneError::ProfileSecretRefused);
        }
        if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(PaneError::EnvNameInvalid);
        }
        Ok(Self(text.to_owned()))
    }
```

### E5. The ports (frozen by #637)

```
crates/holler-pane/src/ports.rs:54-82
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
```
```
crates/holler-pane/src/profile.rs:319-353
/// The registry of profiles, kept by the hub (#661) and faked by the test kit (#638).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// **Writes** are compare-and-swap on the profile's `generation` (see
/// [`crate::generation`]) and carry the [`Actor`] that made them; each appends one
/// [`ProfileLogEntry`]. A stale generation is `generation-conflict`.
pub trait ProfileStore: Send + Sync {
    /// The profile named `name`, or `None`.
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;

    /// Every profile.
    fn list(&self) -> Result<Vec<Profile>, PaneError>;

    /// Store `profile` if the stored one is still at `expected_generation` (0 for a
    /// new profile); returns the stored record with its bumped generation.
    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError>;

    /// Delete the profile `name` if it is still at `expected_generation`. A stale
    /// generation is `generation-conflict`; a profile that does not exist is
    /// `profile-not-found`, whatever `expected_generation` is (a missing profile is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError>;
```
```
crates/holler-pane/src/ports.rs:208-212
pub trait Prober: Send + Sync {
    /// Run `argv` (never through a shell) and look for every string of `expect` in
    /// its output, giving up after `timeout` (see [`crate::run_probe`]).
    fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult;
}
```
```
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
```
```
crates/holler-pane/src/generation.rs:18-34
/// The generation a record moves to when a write that read `expected` is applied
/// to a record now at `current`.
///
/// `Err(PaneError::Conflict)` when `expected != current`: the caller read an older
/// version and must read again before retrying. The counter cannot realistically
/// overflow; if it ever did the store is not trustworthy, so that is
/// `store-corrupt` rather than a wrap.
pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
    if current != expected {
        return Err(PaneError::Conflict);
    }
    current
        .checked_add(1)
        .ok_or_else(|| PaneError::StoreCorrupt {
            what: "a generation counter overflowed".to_owned(),
        })
}
```

### E6. The error codes this story raises (closed set; not edited)

```
crates/holler-pane/src/error.rs:429-446
    /// `profile-conflict`: the profile moved after the live change was made (the I8
    /// write order); `what` names the profile and the reconcile step. (#644/#663.)
    ProfileConflict { what: String },
    /// `profile-not-found`: no profile of that name; `what` is the name.
    /// (#644/#663.)
    ProfileNotFound { what: String },
    /// `profile-exists`: a profile with that name or slug already exists; `what` is
    /// the name. (#661.)
    ProfileExists { what: String },
    /// `profile-has-live-panes`: the profile cannot be deleted while panes of it are
    /// live; `what` names the profile. (#662.)
    ProfileHasLivePanes { what: String },
    /// `pane-not-in-profile`: a named pane does not belong to the profile the verb
    /// is scoped to; `what` names both. (#643/#663.)
    PaneNotInProfile { what: String },
    /// `pane-in-other-profile`: the pane already belongs to another profile (a pane
    /// belongs to at most one); `what` names both. (#661.)
    PaneInOtherProfile { what: String },
```
```
crates/holler-pane/src/error.rs:655-666
            PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
            PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
            PaneError::ProfileExists { what } => write!(f, "profile already exists: {what}"),
            PaneError::ProfileHasLivePanes { what } => {
                write!(f, "profile has live panes: {what}")
            }
            PaneError::PaneNotInProfile { what } => {
                write!(f, "pane is not in the profile: {what}")
            }
            PaneError::PaneInOtherProfile { what } => {
                write!(f, "pane belongs to another profile: {what}")
            }
```
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

### E7. The output module every verb prints through (#670/#676; not edited)

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
```
crates/holler-cli/src/output.rs:184-217
/// The two writers a verb prints to: the real `main` builds it over stdout and stderr, a test
/// over buffers.
pub struct Sink<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

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
```
crates/holler-cli/src/output.rs:240-243
/// Print an error (a failed `emit` with no data) and return its exit code.
pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
    emit(sink, format, Err::<(), _>(error), |()| String::new())
}
```
```
crates/holler-cli/src/output.rs:257-264
/// The error of a stub verb: code `not-implemented` (a runtime failure, exit 1), naming the story
/// that owns the verb.
pub fn not_implemented(story: u32) -> ErrorBody {
    ErrorBody {
        code: ErrorCode::from(&PaneError::NotImplemented),
        message: not_implemented_message(story),
    }
}
```

JSON key order is not a property of `serde_json::Value` maps in this workspace (the workspace declares `serde_json` without
`preserve_order`, and the test kit notes a `--workspace` build turns it on through feature unification):
```
Cargo.toml:45-48
# serde_json without `preserve_order`: holler-proto's canonical round-trips
# sort keys explicitly (tests/common) and the hub's `--json` status printer is
# hand-built (insertion order), so nothing relies on serde_json::Map's ordering.
serde_json = { version = "1" } # for holler-proto, holler-cli
```
```
crates/holler-pane-testkit/src/envelope.rs:21-23
//! 4. It has no other key. The smallest extra key is named. Map order is not used: it
//!    differs between builds, because `serde_json/preserve_order` (on in a `--workspace`
//!    build) keeps document order.
```

### E8. The hub's profile registry (#661, merged): what a real store answers

```
crates/holler-hub/src/profile/mod.rs:26-48
//! # The rules
//!
//! - **The slug is the identity** (`ProfileName::slug`). `get`, `delete` and `log` find a
//!   profile by the slug of the name they are given, so `NIGHT-SHIFT` finds `Night Shift`.
//!   A write of a name whose slug is filed under another live name is `profile-exists`,
//!   whatever the generation, and a submitted slug is replaced by the name's.
//! - **Every write is a compare-and-swap** on the generation: a create names 0 and is
//!   stored at 1, and a stale or an ahead generation is `generation-conflict`. A delete of a
//!   profile that does not exist is `profile-not-found`, whatever the generation.
//! - **The change log is append-only.** Each applied write appends one entry (when, the
//!   generation after the write, who, and what changed), and a refused write appends
//!   nothing. A delete is logged at the deleted generation + 1. The log is never cut: it is
//!   still readable after a delete and goes on across a re-create, so `log` is
//!   `profile-not-found` only for a name never created. No method takes a log.
//! - **A profile holds no secret** (I7): an env entry is an `EnvVarName`, whose own decode
//!   refuses a value, in a request and in the file alike.
//! - **The membership rule** (ADR-0021 §8): a pane belongs to at most one profile, and
//!   names only a profile that exists. [`check_membership`], the `pane/cas_put` hook, checks
//!   that the named profile exists. The refusal of a move from one profile to another
//!   (`pane-in-other-profile`) is not here: it needs the stored pane, so it runs inside the
//!   pane registry's compare-and-swap, under the pane lock ("Decisions taken", item 2). A
//!   spec may name a pane of another profile, or one that does not exist (a detached
//!   spec): that is not refused.
```
```
crates/holler-hub/src/profile/store.rs:190-206
    /// Store `profile` as `actor` if the stored record is still at `expected` (0 for a new
    /// profile), and log the write. The stored record is `profile` with the name's slug,
    /// the next generation and the hub's stamps: the submitted slug, generation, `created`
    /// and `updated` are ignored. Returns the stored record.
    pub(crate) fn cas_put(
        &self,
        profile: &Profile,
        expected: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        let mut guard = self.lock();
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let slug = profile.name.slug();
        let previous = table.entries.get(&slug);
        let stored = previous.and_then(ProfileEntry::profile);
        check_name(stored, &profile.name, &slug)?;
        let generation = next_generation(stored.map_or(0, |stored| stored.generation), expected)?;
```
```
crates/holler-hub/src/profile/store.rs:390-405
/// The name rule of a write (D6): the slug is a profile's identity, so a write of `name`
/// while its slug is filed under another live name is `profile-exists`, whatever the
/// generation. A display name changes only through a rename (#665). A tombstone holds no
/// name: after a delete, a create may spell the name another way, and its log goes on.
fn check_name(stored: Option<&Profile>, name: &ProfileName, slug: &str) -> Result<(), PaneError> {
    match stored {
        Some(stored) if stored.name != *name => Err(PaneError::ProfileExists {
            what: format!(
                "{:?} has the slug {slug:?} of the stored profile {:?}",
                name.as_str(),
                stored.name.as_str()
            ),
        }),
        _ => Ok(()),
    }
}
```
```
crates/holler-hub/src/profile/mod.rs:206-216
pub fn check_membership(pane: &Pane, profiles: &ProfileState) -> Result<(), PaneError> {
    let Some(name) = &pane.profile else {
        return Ok(());
    };
    match profiles.store.get(name)? {
        Some(_) => Ok(()),
        None => Err(PaneError::ProfileNotFound {
            what: name.to_string(),
        }),
    }
}
```

So on the real store, a create at expected generation 0 of a name that is already stored **under the same display name** is
`generation-conflict` (from `next_generation`), and only a different display name with the same slug is `profile-exists`.
A pane write naming a profile that does not exist is `profile-not-found` (so the profile must exist before a pane joins it).

### E9. The test kit (#638, merged): fakes, fixtures, seams

```
crates/holler-pane-testkit/src/fixture.rs:18-32
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
```
crates/holler-pane-testkit/src/fixture.rs:85-92
/// A valid, deterministic `ProfileSpec` for the pane named `pane`: workspace
/// `scratch`, grid `r1c1`, directory `/srv/demo`, the OpenCode harness with the port
/// policy `fixed`, the sample pane's model and context ceilings, an agent, and no env,
/// no command, no check and no expect. Two calls with one name return equal specs.
///
/// `pane` is plain text and is not checked, because a spec may name a pane that has no
/// record (a detached spec).
pub fn sample_spec(pane: &str) -> ProfileSpec {
```
```
crates/holler-pane-testkit/src/fixture.rs:116-121
/// A valid, deterministic `Profile` named `name`: its slug is the name's slug, its
/// generation 0, its specs one [`sample_spec`] per entry of `panes`, in order, and its
/// `created` and `updated` 0. Two calls with the same arguments return equal profiles.
///
/// `usage` when `name` is not a valid profile name.
pub fn sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError> {
```
```
crates/holler-pane-testkit/src/pane_store.rs:46-67
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
///
/// Two behaviours are the fake's own, not the port's:
///
/// - A `cas_put` is checked for its generation first and for membership second, so a
///   stale write that also changes the profile is `generation-conflict`.
/// - The fake does not check that the profile a pane names exists. The hub does that
///   in `check_membership`, outside the port, and answers `profile-not-found`.
```
```
crates/holler-pane-testkit/src/pane_store.rs:90-96
    /// A store holding `panes`, each created at expected generation 0 and so stored
    /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
    /// name are `generation-conflict`, as a second create would be.
    pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
        let store = Self::new();
        for pane in panes {
            store.put(&pane, Writer::Port(0))?;
```
```
crates/holler-pane-testkit/src/pane_store.rs:107-125
    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
        &self.faults
    }

    /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
    /// at 1 for a new record), without the membership rule, and publishes its event.
    /// It bypasses the faults and the call log. Returns the stored record.
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
        self.put(pane, Writer::Other)
    }

    /// Another writer removes the record named `name` and publishes its delete (an
    /// event with no record). It bypasses the faults and the call log.
    /// `pane-not-found` when there is no such record.
    pub fn concurrent_delete(&self, name: &PaneName) -> Result<(), PaneError> {
        self.remove(name, Writer::Other)
    }
```
```
crates/holler-pane-testkit/src/pane_store.rs:220-240
/// The membership rule of a `cas_put` (ADR-0021 section 8, decided to run inside the
/// registry's compare-and-swap): a pane stored with profile P cannot be written with
/// another profile Q. Leaving (`None`), joining from `None` and keeping P are allowed.
/// Its second caller is `FakeProfileScope`, which runs it for a `SpecEdit::Set` before
/// it writes the profile.
pub(crate) fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
    let current = stored.and_then(|pane| pane.profile.as_ref());
    match (current, submitted.profile.as_ref()) {
        (Some(current), Some(next)) if current.slug() != next.slug() => {
            Err(PaneError::PaneInOtherProfile {
                what: format!(
                    "{} is in profile {:?}, not {:?}",
                    submitted.name,
                    current.as_str(),
                    next.as_str()
                ),
            })
        }
        _ => Ok(()),
    }
}
```
```
crates/holler-pane-testkit/src/profile_store.rs:54-82
/// An in-memory `ProfileStore`.
///
/// It keeps the port's rules (`holler_pane::profile`; ADR-0021 sections 7 and 8):
///
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
/// - Each applied write appends one entry to the profile's change log: when, the
///   generation after the write, who, and what changed. A refused write appends
///   nothing and publishes no event.
/// - `delete` of a missing profile is `profile-not-found`, whatever the generation.
/// - `rename` is PROPOSED (#665) and answers `not-implemented`, changing nothing.
/// - `watch` follows the feed's rules: a cursor ahead of the head is `usage`, and a
///   watch from `Cursor(0)` yields the current state and then resumes from the head as
///   of that snapshot, as the hub's registry does.
///
/// It keeps the rules the conformance suite adds to the port, too (the suite's module
/// docs give the reasons):
///
/// - Records, events and the log are filed by the slug of the profile's name, and the
///   stored slug is always the name's: a submitted slug is ignored, as the submitted
///   generation is.
/// - The name rule: a `cas_put` of a name whose slug is stored under another name is
///   `profile-exists`, whatever the generation. It is checked before the generation,
///   so a create of such a name at 0 is `profile-exists`, not `generation-conflict`.
/// - A `Deleted` entry carries the deleted generation + 1.
/// - A profile's log is never cut: it is still readable after a delete and goes on
///   across a re-create. Only a name never created is `profile-not-found`.
///
```
```
crates/holler-pane-testkit/src/profile_store.rs:122-130
    /// A store holding `profiles`, each created by `actor` at expected generation 0
    /// and so stored at 1 with one `Created` entry, in order. Seeding bypasses the
    /// faults and the call log. Two seeds with one name are `generation-conflict`, as
    /// a second create would be, and a seed whose name has the slug of an earlier
    /// seed's other name is `profile-exists`.
    pub fn seeded(
        profiles: impl IntoIterator<Item = Profile>,
        actor: &Actor,
    ) -> Result<Self, PaneError> {
```
```
crates/holler-pane-testkit/src/profile_store.rs:150-154
    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<ProfileStoreOp> {
        &self.faults
    }
```
```
crates/holler-pane-testkit/src/profile_store.rs:326-341
/// The name rule of a port write: the slug of a profile is its identity, so a profile
/// whose slug is stored under another name cannot be written (`profile-exists`),
/// whatever the generation. A display name changes only through `rename` (#665), which
/// logs it as a rename.
fn check_name(stored: Option<&Profile>, submitted: &Profile, slug: &str) -> Result<(), PaneError> {
    match stored {
        Some(stored) if stored.name != submitted.name => Err(PaneError::ProfileExists {
            what: format!(
                "{:?} has the slug {slug:?} of the stored profile {:?}",
                submitted.name.as_str(),
                stored.name.as_str()
            ),
        }),
        _ => Ok(()),
    }
}
```
```
crates/holler-pane-testkit/src/fault.rs:72-77
    /// Fail the next call of `op` with `error`, once. The errors queued for one method
    /// come out in the order they were queued, and a call of another method leaves them
    /// queued. A standing fault answers first, also leaving them queued.
    pub fn fail_next(&self, op: Op, error: PaneError) {
        self.lock().queued.push((op, error));
    }
```
```
crates/holler-pane-testkit/src/fault.rs:85-88
    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
        self.lock().calls.clone()
    }
```
```
crates/holler-pane-testkit/src/prober.rs:44-66
impl FakeProber {
    /// Nothing scripted: every run answers `ProbeResult::Error` naming the argv, never
    /// `Ok`.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                scripted: HashMap::new(),
                calls: Vec::new(),
            }),
        }
    }

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
```
crates/holler-pane-testkit/src/herdr.rs:171-185
impl FakeHerdr {
    /// Serves the session `session`, with no workspace, [`Placement::Absolute`] and
    /// [`HerdrVersion::Protocol22`].
    pub fn new(session: &str) -> Self {
        Self {
            state: Mutex::new(State {
                session: session.to_owned(),
                workspaces: Vec::new(),
                placement: Placement::default(),
                version: HerdrVersion::default(),
                sent: Vec::new(),
            }),
            faults: FaultSwitch::new(),
        }
    }
```
```
crates/holler-pane-testkit/src/herdr.rs:209-213
    /// The fault switch of every port method and the log of the calls made through the
    /// port.
    pub fn faults(&self) -> &FaultSwitch<HerdrOp> {
        &self.faults
    }
```
```
crates/holler-pane-testkit/src/envelope.rs:197-202
/// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
/// code of the verb that wrote it.
///
/// Returns the envelope, or the fault of the first rule it breaks, in the order the
/// module doc lists.
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
```

`FakeHost::new()` (`host.rs:97`) and `FakeHarness::new()` (`harness.rs:170`) take no argument and have the same `faults()`
call log. A grep for `ASSUMPTION(#662)` or `ASSUMPTION (#662)` in `crates/holler-pane-testkit` finds nothing: no fake decided
anything on this story's behalf.

The in-process verb harness (shared by `pane_verbs` and `profile_verbs`; not edited):
```
crates/holler-cli/tests/verb_harness/mod.rs:23-36
static UNWIRED: Unwired = Unwired;

/// The ports every method of which answers `not-implemented` (see [`Unwired`]).
pub fn unwired_ports() -> Ports<'static> {
    Ports {
        pane_store: &UNWIRED,
        profile_store: &UNWIRED,
        herdr: &UNWIRED,
        host: &UNWIRED,
        harness: &UNWIRED,
        scope: &UNWIRED,
        prober: &UNWIRED,
    }
}
```
```
crates/holler-cli/tests/verb_harness/mod.rs:52-79
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
    let mut full = vec!["holler"];
    full.extend_from_slice(argv);
    let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{full:?} must parse: {e}"));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = {
        let mut ctx = VerbCtx {
            format,
            ports,
            sink: Sink {
                out: &mut out,
                err: &mut err,
            },
        };
        match &cli.command {
            Command::Pane(cmd) => pane::run(cmd, &mut ctx),
            Command::Profile(cmd) => profile::run(cmd, &mut ctx),
            other => panic!("not a pane or profile verb: {other:?}"),
        }
    };
    Outcome {
        code,
        out: String::from_utf8(out).expect("out is UTF-8"),
        err: String::from_utf8(err).expect("err is UTF-8"),
    }
}
```
```
crates/holler-cli/tests/profile_verbs/main.rs:6-23
//! In-process tests of the `holler profile` verbs (story #670, epic #633).
//!
//! One module per verb, so a verb story edits only `profile_verbs/<verb>.rs` and never
//! this file or the manifest. The harness that runs a verb over captured writers is
//! shared with `pane_verbs` (`tests/verb_harness/mod.rs`). The output API and the
//! shared flag validation are tested once, in `pane_verbs`.

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod apply;
mod create;
mod delete;
mod export;
mod import;
mod list;
mod rename;
mod show;
```
```
crates/holler-cli/src/pane/wiring.rs:49-51
/// A port set whose every method answers [`PaneError::NotImplemented`]. Its probe never passes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unwired;
```

### E10. The surface files a verb story owns (epic ruling 2)

```
docs/adr/ADR-0003.md:65-68
holler profile create                                             #662
holler profile delete                                             #662
holler profile list                                               #662
holler profile show                                               #662
```
```
docs/adr/ADR-0003.md:92-92
- **`holler pane` and `holler profile`** are pane control (epic #633): the hub keeps one registry of panes and profiles, and every change to a pane goes through these verbs. The rows above list only the flags the skeleton declares (the shared `--profile`, `--spec-only` and `--take-over`, the spec flags and the global `--format`) and no positionals; each owning story adds its verb's positionals and flags to its own row, with its own line in `cli-surface.txt`, and edits no other verb's. Rows are grouped by owning story, one story per block. The rows of `profile rename`, `export` and `import` are **proposed** (#665): the operator confirms them first. ADR-0021 (#634) ratifies the epic's contract.
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:1-11
# The normative, machine-readable CLI surface (ADR 0003 §"CLI"; issue #155 §5).
#
# One invocation per line:  <leaf verb path> | <arguments>
# Every line must parse with `Cli::try_parse_from`, and every leaf verb clap
# knows must appear on at least one line (both asserted by
# tests/cli_surface_test.rs). A verb ADR 0003 specifies that no story has
# implemented yet lives in cli-surface.pending.txt instead — move its lines
# here in the story that adds the verb.
#
# Cover every flag at least once. Tokens are split on whitespace; double
# quotes group a token.
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:157-163
# #662
profile create |
profile delete |
profile list |
profile list | --format=json
profile show |
profile show | --json
```
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:9-17
/// Every verb that is still a stub, with the story that owns it. The only place in the
/// shared process tests that names a stub's owning story.
///
/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
/// line**: two stories that delete whole groups, header included, delete adjacent lines,
/// and git reports that as a conflict. (`PANE_VERBS` and `PROFILE_VERBS` keep the verb
/// itself.)
pub const STUBS: &[(&str, &str, u32)] = &[
```
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:34-41
    // #650
    ("pane", "import", 650),
    // #662
    ("profile", "create", 662),
    ("profile", "delete", 662),
    ("profile", "list", 662),
    ("profile", "show", 662),
    // #664
```

`tests/pane_verbs/process/docs_rows.rs` (not edited) already groups `profile create`, `delete`, `list` and `show` under story 662 for its ADR-row and fixture-layout checks.

### E11. ADR-0021, the sections this story relies on or amends

```
docs/adr/ADR-0021.md:149-154
Giving both forms of `command` or of `check` is a usage error (exit 2). **`profile create --from-current`** copies, from the
`Pane` records in `PaneStore` (so it needs no Herdr call), each member's `herdr.workspace` and `herdr.grid`, `host.cwd`,
`harness.kind`, `model`, `role`, `env`, `context`, `command`, `probe.check` and `probe.expect`, and sets each member's
`Pane.profile`; a pane already in another profile is `pane-in-other-profile`. The mapping lives in
`holler-pane/src/profile_snapshot.rs` (#662), which the migration (#650) reuses. **Deferred to #662:** what `--from-current`
writes for `harness.port_policy`, which the `Pane` record does not hold (it records the port in use, not the policy).
```
```
docs/adr/ADR-0021.md:266-283
**Decided: how generations fence concurrent writers.**

- Every `Pane` and `Profile` carries `generation: u64`. A record that does not exist is at generation 0, so a create names
  `expected_generation: 0` and is stored at 1. Each applied write adds one. The store sets the generation; the one a client
  submits is ignored. The rule is `holler_pane::next_generation`, written once and used by every store and fake.
- A write whose expected generation is not the current one is `generation-conflict` and changes nothing. `delete` checks
  that the record exists first (`pane-not-found` or `profile-not-found`, whatever the generation) and the generation second.
- A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  writes nothing more, and prints the reconcile step (the pane doctor command line for that pane).
- **Generations fence records, not live acts.** Two verbs on the same pane can both act; only one records. The frozen
  `Pane` has no field to claim a pane before acting, so this ADR adds none; the reconcile pass (#647) is the safety net. A
  claim, if one is wanted, comes with the operation id (section 12).
- **Known gap:** a record deleted and then created again restarts at generation 1, so a writer still holding generation 1 of
  the old record can overwrite the new one. The contract fixes "0 for a new record", so this is recorded, not fixed.
- No store transaction spans the two registries. The membership rule is enforced on `pane/cas_put`: setting `Pane.profile`
  to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
  names a pane of another profile (a detached spec) is not refused.
```
```
docs/adr/ADR-0021.md:331-348
**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.

| Verb | Codes beyond the common ones |
|---|---|
| `pane list`, `pane watch` | `profile-not-found`, `pane-not-in-profile` |
| `pane get` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile` |
| `pane launch`, `pane relaunch` | `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `profile-secret-refused`, `probe-failed`, `herdr-version-unsupported`, `session-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-in-other-profile`; `relaunch` also `pane-not-found` |
| `pane switch`, `pane reset` | `pane-not-found`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645) for a pane that is not idle, holds a question or has an unhealthy server, and for the orchestrator's own pane |
| `pane park`, `pane unpark` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
| `pane close` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict` |
| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
| `pane import` | `profile-exists` (a profile `fleet` already exists); `command-not-argv` and disagreements are findings, not errors (#650) |
| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; open (#646) for an unhealthy pane and for SHOWN differing from DRIVEN |
| `profile create` | `profile-exists`, `profile-not-found` (for `--from`), `pane-in-other-profile`, `generation-conflict` |
| `profile delete` | `profile-not-found`, `profile-has-live-panes`, `generation-conflict` |
| `profile list` | none |
| `profile show` | `profile-not-found` |
```
```
docs/adr/ADR-0021.md:384-398
  | `profile-exists` | Refusal (3) | The name is taken; nothing went wrong. |
  | `profile-has-live-panes` | Refusal (3) | Delete is declined while panes of the profile are live. |
  | `pane-not-in-profile` | Refusal (3) | The `--profile` scope check declined a pane outside the profile. |
  | `pane-in-other-profile` | Refusal (3) | Ownership policy: a pane belongs to at most one profile (C3). |
  | `probe-failed` | Refusal (3) | The health gate declined to launch or relaunch, working as designed. |
  | `herdr-version-unsupported` | Refusal (3) | A fail-closed version gate, like ADR 0003's exit 3. |
  | `profile-not-found` | Refusal (3) | The named profile does not exist; the request was understood and declined. |
  | `pane-not-found` | Refusal (3) | The named pane does not exist. |
  | `session-not-found` | Refusal (3) | The named harness session does not exist. |
  | `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
  | `profile-conflict` | Failure (1) | The profile moved after the live change (the I8 write order); a race. |
  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. |
  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply). |
  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
```
```
docs/adr/ADR-0021.md:521-533
## Deferred to named stories

- The operation id and the executor of long work: #644, after a contract amendment (section 12).
- A periodic reconcile outside the hub: #647.
- `harness.port_policy` under `profile create --from-current`: #662.
- The roster's `--json` shape beside the envelope: #648.
- How the hub's DRIVEN session follows `session_of_record`: #649 and #654.
- The exact file layouts of `panes.json` and `profiles.json`, the long-poll window, and the feed's retained window: #639 and
  #661.
- What makes a profile's pane "live" for `profile-has-live-panes`: #662.
- Which closed failure code a mismatch observed after `act` carries (I3 says the verb exits 1, and under section 9 an open
  code is a refusal, exit 3): #644 and #645.
- `HerdrPort` and `HarnessPort` in their final form: #636 and #635, then #640 and #642.
```

### E12. Workspace rules that bind the code

```
Cargo.toml:19-27
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
cognitive_complexity = "deny"
too_many_lines = "deny"
struct_excessive_bools = "deny"
large_enum_variant = "warn"
```

`clippy.toml`: `cognitive-complexity-threshold = 15`, `too-many-lines-threshold = 100`. `scripts/lint.sh`: every `#[allow]`
carries a trailing `// #NNN`; a file fails at 900 lines. `holler-cli` has `serde`, `serde_json`, `holler-pane` as
dependencies and `holler-pane-testkit` as a dev-dependency already; `holler-pane` has `serde`, `serde_json`, `holler-proto`.
No dependency is added.

Observed by O on `3bdd129` (read-only commands):
```text
$ cargo fmt --all --check            -> exit 1 (the tree is not rustfmt-clean, e.g. crates/holler-body/src/acp_driver/answerable.rs)
$ rustfmt --check --edition 2021 crates/holler-cli/src/profile/{create,delete,list,show}.rs \
    crates/holler-pane/src/profile_{snapshot,diff}.rs crates/holler-cli/tests/profile_verbs/{create,delete,list,show}.rs \
    crates/holler-cli/tests/pane_verbs/process/stub.rs   -> exit 0
$ grep -rn 'ASSUMPTION *(#662)' crates/holler-pane-testkit   -> no output
```

## The public API (fixed here, so T writes RED tests against it and #650, #664 and #665 can rely on it)

### `crates/holler-pane/src/profile_snapshot.rs` (662a)

```rust
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
/// The port is copied as recorded: a record at port 0 gives `fixed:0`, which #644's `port_of_policy` refuses (`usage`).
pub fn spec_from_pane(pane: &Pane) -> ProfileSpec;

/// A new profile named `name` (slug `name.slug()`, generation 0, `created` and `updated` 0: the store
/// stamps them) with one `spec_from_pane` per entry of `panes`, in the order given.
pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile;
```

### `crates/holler-pane/src/profile_diff.rs` (662a)

```rust
use std::fmt;
use serde::Serialize;
use crate::argv::Argv;
use crate::grid::GridPos;
use crate::pane::Pane;
use crate::profile::{Profile, ProfileName, ProfileSpec};

/// A field of a `ProfileSpec`, named by its dotted JSON path. Serializes as that path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum SpecField {
    #[serde(rename = "herdr.workspace")] Workspace,
    #[serde(rename = "herdr.grid")] Grid,
    #[serde(rename = "host.cwd")] Cwd,
    #[serde(rename = "harness.kind")] HarnessKind,
    #[serde(rename = "harness.port_policy")] PortPolicy,
    #[serde(rename = "model.provider")] Provider,
    #[serde(rename = "model.model_id")] ModelId,
    #[serde(rename = "model.effort")] Effort,
    #[serde(rename = "role")] Role,
    #[serde(rename = "env")] Env,
    #[serde(rename = "context.soft")] ContextSoft,
    #[serde(rename = "context.hard")] ContextHard,
    #[serde(rename = "command")] Command,
    #[serde(rename = "check")] Check,
    #[serde(rename = "expect")] Expect,
}

impl SpecField {
    /// Every field, in the order `profile show` lists a spec and `diff_spec` compares (the order above).
    /// `PortPolicy` included (Decision 3).
    pub const ALL: [SpecField; 15];
    /// The dotted path, equal to the serde name (`"herdr.grid"`).
    pub const fn as_str(self) -> &'static str;
    /// The value of this field in `spec`. `harness.kind` and `role` take their text from serde
    /// (`serde_json::to_value(..)`'s string, as `pane/args.rs::parse_role` does; a non-string falls back to
    /// an empty text, never a panic), so no variant name is written a second time.
    pub fn value(self, spec: &ProfileSpec) -> FieldValue;
}

/// One field's value, typed so that it serializes exactly as the field does inside a `ProfileSpec`
/// (a grid through `GridPos`'s own `Serialize`, row first) and prints for a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// workspace, cwd, kind (`"opencode"`), port_policy, provider, model_id, effort, role (`"agent"`).
    Text(String),
    Grid(GridPos),
    /// context.soft, context.hard.
    Number(u32),
    /// env (names), expect, in stored order.
    List(Vec<String>),
    /// command, check; `None` serializes as `null`.
    Argv(Option<Argv>),
}

/// Text form: `Text` with every control character escaped (`char::escape_default`, so ESC prints as
/// `\u{1b}` and a newline as `\n`); `Grid` as `r2c1`; `Number` in decimal; `List` and `Argv(Some)` as a
/// compact JSON array of strings (`["opencode","serve"]`, never a shell-joined string); `Argv(None)` as `none`.
impl fmt::Display for FieldValue { /* ... */ }

/// One field where a live pane differs from its spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldDiff {
    pub field: SpecField,
    pub spec: FieldValue,
    pub live: FieldValue,
}

/// How one pane of a comparison stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneStatus {
    /// A spec and a live pane of its name, equal on every compared field.
    Matches,
    /// A spec and a live pane of its name that differ on at least one compared field.
    Differs,
    /// A spec with no live pane of its name.
    Missing,
    /// A live pane that no spec names.
    Extra,
}

/// One row of a comparison. `differences` is non-empty exactly when `status` is `Differs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneDiff {
    pub pane: String,
    pub status: PaneStatus,
    pub differences: Vec<FieldDiff>,
}

/// Whether `pane` is a member of `profile`: its `profile` has `profile.slug()` (slugs compared on both
/// sides, like the test kit's `belongs`). This is what "live pane of a profile" means (Decision 4).
pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool;

/// Every field where `live` differs from `spec`, in `SpecField::ALL` order; empty when
/// they match. The names are not compared (the caller pairs them). `env` and `expect` are compared as
/// sets (sorted, duplicates dropped); every other field by equality. Pure: it compares `spec` with
/// `profile_snapshot::spec_from_pane(live)`.
pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff>;

/// The comparison of `profile` with the panes in `live` (the caller decides which panes count as
/// live; `profile show` passes the profile's members). One row per spec, in the profile's order
/// (`Matches`/`Differs` against the pane of `live` with that name, else `Missing`), then one `Extra` row
/// per pane of `live` that no spec names, in the order of `live`.
pub fn diff_profile(profile: &Profile, live: &[Pane]) -> Vec<PaneDiff>;
```

### The verbs' clap arguments (each in its own file; `mod.rs` is not touched)

```rust
// create.rs (662b)
/// Create a profile: empty, a detached copy of another (--from), or a snapshot of every pane (--from-current).
#[derive(Args, Debug)]
pub struct ProfileCreate {
    /// The profile's name (spaces allowed).
    pub name: String,
    /// Snapshot every pane in the registry into specs and make each pane a member.
    #[arg(long, conflicts_with = "from")]
    pub from_current: bool,
    /// Copy the specs of PROFILE; no pane joins (a detached copy).
    #[arg(long, value_name = "PROFILE")]
    pub from: Option<String>,
}
// delete.rs (662b)
#[derive(Args, Debug)]
pub struct ProfileDelete {
    pub name: String,
    /// First detach every member pane (its `profile` becomes none; the pane keeps running).
    #[arg(long)]
    pub keep_panes: bool,
}
// list.rs (662a): `pub struct ProfileList {}` unchanged.
// show.rs (662a)
#[derive(Args, Debug)]
pub struct ProfileShow { pub name: String }
```
The names are `String` at clap time and go through `ProfileName::parse` in `run`, so a bad name is an envelope coded `usage`
(exit 2) in both formats, from the verb, not clap.

### What each verb prints

JSON `data` (always a `#[derive(Serialize)]` struct, never a hand-built `serde_json::Value` map, so key order is the struct's
in every build, E7):

| Verb | `data` |
|---|---|
| `list` | `[{"name","slug","panes","live","generation"}, ...]`, sorted by slug; `panes` = number of specs, `live` = number of members |
| `show` | `{"profile": <Profile as stored>, "comparison": [{"pane","status","differences":[{"field","spec","live"}],"probe"}]}`; `probe` is `null` (not live, or never run), `"ok"`, `{"failed":{"missing":[...]}}` or `{"error":"..."}` (the `ProbeResult` serde form of `Pane.probe.last`) |
| `create` | `{"profile": <Profile as stored>, "members": [pane names that joined]}` (`[]` for plain and `--from`) |
| `delete` | `{"name","slug","detached": [pane names whose profile was cleared]}` (`[]` without `--keep-panes`) |

Text (one result, ends with a newline; stored strings printed through `FieldValue`'s `Display`):

```text
list:    Some Profile (some-profile): 2 panes, 1 live, generation 3        (one line per profile; "no profiles" when none)
show:    profile Some Profile (some-profile): generation 3, 2 specs, 1 live
         spec demo-c1r1
           herdr.workspace: scratch                                          (one line per SpecField::ALL, "<path>: <value>")
           ...
           command: ["opencode","serve"]
         pane demo-c1r1: differs
           herdr.grid: spec r1c1, live r2c1                                  (one line per FieldDiff)
           probe: ok
         pane demo-c2r1: matches
           probe: failed (missing "qwen38")
         pane demo-c3r1: missing (no live pane)
         pane demo-c4r1: extra (no spec)
           probe: none
create:  created profile Some Profile (some-profile): 2 specs, generation 1
         members: demo-c1r1, demo-c2r1                                       (--from-current only; "members: none" if none)
         copied from Alpha; no pane joined                                   (--from only)
delete:  deleted profile Some Profile (some-profile)
         detached, still running: demo-c1r1, demo-c2r1                       (--keep-panes with members only)
```
The probe line: `ok`; `failed (missing "a", "b")` (each missing string in Rust `{:?}` quoting, comma-separated); `error
("reason")` (`{:?}` quoting); `none` when `probe.last` is `None`. `{:?}` escapes control characters, so no stored text reaches
the terminal raw.

## Behaviour (what F implements, in this order)

Every verb takes its ports from `ctx.ports` and prints once through `emit` (success) or `emit_error` (failure); a store error
passes through with its own code (`unavailable`, `timeout`, `store-corrupt`, `generation-conflict`) unless a step below maps it.
No verb calls `herdr`, `host`, `harness`, `scope` or `prober`, and none reads the process environment.

**list (662a).** `profile_store.list()`, then one `pane_store.list()`; for each profile (sorted by slug) count its specs and the
panes for which `is_member(pane, &profile.name)`.

**show (662a).** `ProfileName::parse(name)` (`usage`); `profile_store.get` (`None` -> `ProfileNotFound { what: <name {:?}> }`,
exit 3); `pane_store.list()` filtered by `is_member(_, &profile.name)`; `diff_profile(&profile, &members)`; each row's `probe` is the member's
`probe.last` (`None` for `Missing`). The probe is **read**, never run (Decision 6).

**create (662b).** Common: parse NAME (`usage`); `profile_store.get(NAME)` is `Some` -> `ProfileExists { what: "<name> (slug
<slug>)" }` (exit 3). The create write is `profile_store.cas_put(&new, 0, &actor)`; a `generation-conflict` from it (another
writer created the name between the `get` and the `put`, E8) is reported as `ProfileExists`; a `ProfileExists` from it passes
through. The actor is `Actor::parse("holler profile create")` (Decision 9).
- plain: `new` = a profile with no specs.
- `--from P`: parse P (`usage`); `get(P)` is `None` -> `ProfileNotFound` (checked after NAME's existence); `new.panes =
  P.panes.clone()` verbatim (a spec naming a pane of another profile is kept); **no pane is read or written**.
- `--from-current`:
  1. **Plan.** `panes = pane_store.list()`. Every pane whose `profile` is `Some(q)` with `q.slug() != NAME.slug()` is refused
     together, before anything is written: `PaneInOtherProfile { what: "demo-c1r1 is in profile \"Other\"; ..." }` (all such
     panes, `; `-joined, exit 3).
  2. **Profile first.** `new = profile_from_panes(&NAME, &panes)`; `cas_put(&new, 0, actor)` (the hub's membership hook needs the
     profile to exist before a pane can name it, E8).
  3. **Join.** For each pane in `panes` order: `pane_store.cas_put(&Pane { profile: Some(NAME), ..pane }, pane.generation)`,
     keeping each returned record.
  4. **Undo on a failed join** (error `E` at pane k): for each pane that already joined, `cas_put(&Pane { profile: None,
     ..returned }, returned.generation)`; then `profile_store.delete(NAME, stored.generation, actor)`. If every undo write
     succeeds, the verb fails with `E`'s code and the message `"<E>; nothing was kept (the profile and the memberships made
     so far were undone)"` (exit = `E`'s class: 3 for `pane-in-other-profile`, 1 for `generation-conflict`). If an undo write
     fails, the verb fails with `ProfileConflict { what: "<name>: pane <k> could not join (<E>) and the undo failed (<E2>);
     reconcile with: holler profile show <name> and holler profile delete <name> --keep-panes" }` (exit 1). One line.

**delete (662b).** Parse NAME (`usage`); `get` (`None` -> `ProfileNotFound`, exit 3); `members = pane_store.list()` filtered by
`is_member(_, &profile.name)`.
- `members` non-empty and no `--keep-panes` -> `ProfileHasLivePanes { what: "<name> has N live panes (a, b); run holler profile
  delete <name> --keep-panes to detach them" }` (exit 3), nothing written.
- `--keep-panes`: for each member, `cas_put(&Pane { profile: None, ..member }, member.generation)`. A failure stops the verb with
  the store's code and the message `"<E>; detached so far: a, b; the profile was not deleted; run the delete again"`; nothing is
  re-attached (Decision 8).
- Then `profile_store.delete(NAME, profile.generation, actor)` (actor `holler profile delete`). A `generation-conflict` here is
  reported as `ProfileConflict { what: "<name> changed after its panes were detached (a, b); run holler profile show <name>,
  then the delete again" }` (exit 1) when at least one pane was detached, and as `generation-conflict` (exit 1) otherwise.

## Contradictions found

- **C1 Rigor.** The issue's Pipeline line says `rigor: in-session`; the operator's instruction (through the MO) is
  `second-opinion`. The header follows the operator.
- **C2 `profile-conflict` is not in ADR-0021's table for these verbs.** The issue lists `profile-conflict` among the codes of
  the four verbs; ADR-0021 section 9 (E11) gives `profile create` "`profile-exists`, `profile-not-found` (for `--from`),
  `pane-in-other-profile`, `generation-conflict`" and `profile delete` "`profile-not-found`, `profile-has-live-panes`,
  `generation-conflict`", with no `profile-conflict`. Decided: both are used (Decision 7: `profile-conflict` only where a
  write already landed and the next one lost), and ADR-0021's two rows gain `profile-conflict` (Decision 14 (ii)). Also, the
  issue calls these five codes "refusals", but `class_of` (E6) makes `profile-conflict` and `generation-conflict` failures
  (exit 1); the code wins and the tests assert exit 1.
- **C3 Herdr.** The issue says the verbs code against `HerdrPort`, and that the snapshot "takes positions from Herdr's snapshot
  through the adapter"; its own corrections (2026-10-08) say `--from-current` "needs no Herdr call" and takes the position
  "from the Pane record's `herdr.grid`". ADR-0021 section 3 agrees with the corrections. Decided: no verb calls `HerdrPort`
  (tested, AC 3).
- **C4 `show`'s probe.** "`show` reports each pane's **last** probe result" vs "reports `ok` and `failed (missing "qwen38")`
  from **fake probe results**", which could mean the `FakeProber`. Decided: `show` reads `Pane.probe.last` and never runs a
  probe (Decision 6); the tests seed `probe.last` in the fake store and assert the `FakeProber` was never called.
- **C5 `cargo fmt --check`.** The MO asked for `cargo fmt --check`; it fails on `main` today (E12) and epic ruling 4 (E2) reads
  the rule as "new `.rs` files pass `rustfmt --check --edition 2021`; existing files are not reformatted". AC 14 uses
  `rustfmt --check --edition 2021` on every file this story writes (all of them pass today).
- **C6 Blast radius.** The issue lists only the six source files; epic ruling 2 adds each verb's ADR 0003 row, its
  `cli-surface.txt` line and its `tests/profile_verbs/<verb>.rs`, and `stub.rs`'s own doc (E10) tells a verb story to delete its
  STUBS entries. The fixture lines `profile create |`, `profile delete |`, `profile show |` stop parsing once NAME is required,
  so they must change. The Files section lists all of them, plus ADR-0021 (Decision 14) and `CHANGELOG.md`.
- **C7 I1 versus membership writes.** Invariant I1 says only `holler pane` verbs change a pane or its registration;
  `create --from-current` and `delete --keep-panes` write `Pane.profile`. Both the issue and ADR-0021 section 3 mandate it, so it
  is not changed here; the writes touch only `profile` (AC 6f and AC 7c assert every other field is unchanged).

## Decisions already made (MO)

1. **Split** as in the Size check (662a then 662b), unless the MO keeps one run.
2. **`--from-current` snapshots every pane in `PaneStore`**, in `list()` order (by name), with no Herdr call (C3). Zero panes
   gives a profile with no specs and `members: []`. A pane already in another profile refuses the whole create before any write
   (the issue's `pane-in-other-profile`), which also matches what the stores would refuse inside their compare-and-swap (E8,
   E9 `check_membership`). A pane whose `profile` already has NAME's slug joins like any other.
3. **`harness.port_policy` under `--from-current` is `fixed:<port>`** (ADR-0021's deferred item). The record holds the port in
   use and no policy (E11); `fixed:<port>` keeps that fact, so `apply` (#664) and the recreation (#666) can put the harness back
   on the same port. `fixed` alone (the fixture's value) would lose it. The policy grammar is otherwise #644's; this story
   defines only this one form, as `FIXED_PORT_POLICY_PREFIX` and `fixed_port_policy`. `port_policy` **is compared** (it is in
   `SpecField::ALL`, like every field): the live side is `fixed_port_policy(live.harness.port)`, and #644 (amended, `7195993`)
   makes `fixed:<port>` the only form, so this is a port comparison. A pane relaunched on another port shows as `differs` in
   `show`, `apply` (#664) and `profile-drift` (#665). A spec holding the bare `fixed` (the kit's `sample_spec`, E9) differs from
   every live pane; tests that need a match set `port_policy` to `fixed_port_policy(<port>)` (Follow-ups).
4. **A profile's live panes are its members**: the pane records whose `profile` has the profile's slug (`is_member`). A closed
   pane's record is removed (epic ruling 9), so a record is the registry's statement that the pane exists; reading Herdr or
   the harness would make `list`, `show` and `delete` depend on adapters this story must not call. `list`'s live count, `show`'s
   comparison and `delete`'s `profile-has-live-panes` all use this one definition. A detached spec naming a pane of another
   profile shows as `missing` (membership is `Pane.profile` only).
5. **One comparison for show, apply and drift.** `diff_spec` compares `spec` with `spec_from_pane(live)` on `ALL`, so
   `diff_spec(&spec_from_pane(p), p)` is always empty (AC 2c). That is what makes #650's "`profile show fleet` reports no
   difference" hold. `env` and `expect` compare as sets (a reorder is not drift); `command` and `check` compare as ordered
   argv.
6. **`show` never runs a probe** (C4): it reads `Pane.probe.last`. A read verb that executes stored argv would run a command as
   a side effect of looking, and N probes could exceed I5's bound.
7. **Codes.** `profile-exists` (exit 3) for a taken name or slug, including the create race; `profile-not-found` (3);
   `profile-has-live-panes` (3); `pane-in-other-profile` (3); `profile-conflict` (1) only when an earlier write of the same
   verb landed and a later one lost (create's failed undo; delete's conflict after a detach); `generation-conflict` (1)
   otherwise; `usage` (2) for a bad name. No new code, no `Refused`, no edit to `error.rs`.
8. **Undo policy.** `create --from-current` undoes everything on a failed join (I3: a failure records nothing), and reports
   `profile-conflict` with the reconcile commands if the undo itself fails. `delete --keep-panes` does not re-attach after a
   failed detach: detaching is what the operator asked for, a re-run converges, and an undo could itself fail.
9. **Actor** of every profile write: the literal verb, `holler profile create` / `holler profile delete` (a `ProfileLogEntry`'s
   actor may be "a verb such as `holler profile apply`", E4). Built with `Actor::parse` in `run`; its `Err` is mapped through
   `emit_error`, never unwrapped.
10. **Output.** Shapes as in "What each verb prints". JSON comes only from derived `Serialize` structs and `FieldValue` (typed),
    so a grid is `{"row":2,"col":1,"pos":"r2c1"}` in every build (E7). `list` sorts by slug itself (the port pins no order).
11. **Where the pure tests live.** Pure functions over records are tested in their own crate (`docs/testing.md:45`, "per-crate
    integration tests"), like the nine `crates/holler-pane/tests/<topic>_test.rs` files; only port-driven code needs the test
    kit, and these two modules call no port. AC 1 is `crates/holler-pane/tests/profile_snapshot_test.rs`, AC 2
    `crates/holler-pane/tests/profile_diff_test.rs`. Each starts with
    `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #662` and `mod common;`, and builds on
    `common::{pane, spec, profile, pane_name, profile_name}`, overridden per case with the neutral names (`demo-c1r2`, `/srv/demo`,
    `demo-provider`) wherever a test sets or asserts one. No manifest change: `crates/holler-pane/Cargo.toml` has no `[[test]]`
    or `autotests`, so a new `tests/*.rs` is discovered, and `serde_json` is already a dependency. `profile_verbs/show.rs` keeps
    the verb's own AC 5.
12. **Test rig, without editing `main.rs`.** `tests/profile_verbs/main.rs` is #670's; the shared rig (the fakes, a `ports()`,
    `run(argv, format)`, `assert_no_adapter_call()`) is its own file, `tests/profile_verbs/rig.rs`, declared from `list.rs` as
    `#[path = "rig.rs"] pub(crate) mod rig;` (662a; a `#[path]` in the non-`mod.rs` file `list.rs` resolves against its
    directory; the way #644 includes `launch_rig.rs`). `show.rs`, `create.rs`, `delete.rs` `use crate::list::rig`; #664/#665
    extend `rig.rs`, not `list.rs`. Each format runs on its own freshly seeded rig. Adapters are `FakeHerdr::new("scratch")`,
    `FakeHost::new()`, `FakeHarness::new()`, `FakeProber::new()` (asserted uncalled); `scope` is the harness's `Unwired`. One base
    rig should be agreed across #643 (`pane_verbs/list.rs`), #644 (`launch_rig.rs`) and this one (Follow-ups).
13. **A failing-Nth-write seam for the undo tests (662b).** The fakes fail only the *next* call of a method (E9 `fail_next`), so a
    join that fails on the second pane needs a seam: a test-local `PaneStore` in `create.rs` that delegates every method to a
    `FakePaneStore` and fails its N-th `cas_put` (and optionally the M-th) with a given `PaneError`. It holds no state of its own
    beyond the counter.
14. **ADR-0021 doc edit (F; three places, no other ADR text changes):** (i) section 3, lines 153-154: replace "**Deferred to
    #662:** what `--from-current` writes for `harness.port_policy`, which the `Pane` record does not hold (it records the port
    in use, not the policy)." with "`--from-current` writes `harness.port_policy` as `fixed:<port>`, the port the pane's harness
    uses (#662, `profile_snapshot::fixed_port_policy`); `profile_diff` compares it with `fixed:<port>` of the live port. A profile's **live
    panes** are the pane records whose `profile` names it (by slug)." (ii) section 9 table: add `profile-conflict` as the last code of the
    `profile create` row and of the `profile delete` row. (iii) "Deferred to named stories": delete the two bullets that name #662.
    662a makes (i) and (iii), 662b makes (ii).

## Acceptance criteria (each observable; (a) = run 662a, (b) = run 662b)

All commands run from the worktree. "Both formats" means the same argv run with `Format::Text` and `Format::Json` on two freshly
seeded rigs, with equal exit codes, and the JSON `out` passing `check_envelope(&out, code)` (E9). Test names are the ones T
writes; T may add cases but not drop these. A verb-test spec meant to match a seeded pane sets `harness.port_policy` to
`fixed_port_policy(<its port>)` (the kit's `sample_spec` says `fixed`, which differs under Decision 3).

1. **(a) Snapshot.** In `crates/holler-pane/tests/profile_snapshot_test.rs` (Decision 11; over `common::pane()`):
   - a. `snapshot_copies_every_spec_field_from_the_pane_record`: a pane with every field set (grid r2c1, cwd, workspace, model,
     role `orchestrator`, env `["ALPHA_TOKEN","BETA_URL"]`, context 1/2, command, `probe.check`, `probe.expect ["qwen38"]`,
     port 48100) gives a spec equal to one written out by hand per the mapping table, with `port_policy == "fixed:48100"`.
   - b. `snapshot_takes_the_position_from_the_record_not_the_name`: `demo-c1r2` recorded at `{row:2,col:1}` snapshots to
     `r2c1`; `demo-c2r1` recorded at `{row:1,col:1}` snapshots to `r1c1`.
   - c. `profile_from_panes_keeps_order_and_starts_at_generation_zero`.
2. **(a) Diff.** In `crates/holler-pane/tests/profile_diff_test.rs` (Decision 11; over `common::{pane, spec, profile}`):
   - a. `diff_reports_each_differing_field_once`: against `spec_from_pane(&p)`, changing grid, cwd and effort on the live pane
     gives exactly three `FieldDiff`s, in `ALL` order, with `spec`/`live` values; the grid one serializes (raw string, not re-parsed) to
     `{"field":"herdr.grid","spec":{"row":1,"col":1,"pos":"r1c1"},"live":{"row":2,"col":1,"pos":"r2c1"}}`.
   - b. `diff_compares_env_and_expect_as_sets_and_argv_in_order`.
   - c. `snapshot_round_trips_to_no_difference`: `diff_spec(&spec_from_pane(&p), &p)` is empty for `common::pane()` and for the
     fully populated pane of 1a (the `sample_pane` half is pinned at the verb level by AC 5b).
   - d. `diff_profile_classifies_matches_differs_missing_extra`: rows in spec order then extras; `differences` non-empty
     exactly for `Differs`.
   - e. `spec_field_values_serialize_like_the_spec`: for every `SpecField::ALL` entry, `serde_json::to_value(field.value(&spec))`
     equals the value at the field's dotted path in `serde_json::to_value(&spec)` (an absent `command`/`check` equals `null`),
     and `field.as_str()` equals its serde name. And `grep -nE '^[^/]*"(opencode|agent|orchestrator)"'
     crates/holler-pane/src/profile_diff.rs` prints nothing (no hand-written serde name, Warn 4).
   - f. `field_text_escapes_control_characters`: a cwd `"/srv/a\u{1b}[31mb\nc"` prints as `/srv/a\u{1b}[31mb\nc` (escaped,
     no raw ESC or newline); an argv prints as a JSON array.
   - g. `diff_compares_port_policy_as_the_live_port`: a spec at `fixed:48100` against a live port 48101 gives exactly one
     `FieldDiff` (`harness.port_policy`, spec `fixed:48100`, live `fixed:48101`); a spec at the bare `fixed` differs too.
   - h. `is_member_compares_slugs`: a pane whose `profile` is `SOME-PROFILE` is a member of `Some Profile`; one with `Other` or
     none is not.
3. **(a)+(b) No adapter, no probe, no environment.** Every verb test ends with the rig's `assert_no_adapter_call()`: the
   `faults().calls()` of `FakeHerdr`, `FakeHost` and `FakeHarness`, and `FakeProber::calls()`, are all empty (the rig's `scope`
   is `Unwired`, so a call to it would fail the verb). And
   `grep -nE 'ports\.(herdr|host|harness|prober|scope)|std::env|env::var' crates/holler-cli/src/profile/{create,delete,list,show}.rs crates/holler-pane/src/profile_snapshot.rs crates/holler-pane/src/profile_diff.rs`
   prints nothing.
4. **(a) list.**
   - a. `list_reports_name_slug_panes_live_generation_in_both_formats`: two profiles seeded (one with 2 specs and 1 member,
     one empty); JSON `data` equals `[{"name":..,"slug":..,"panes":2,"live":1,"generation":1}, ...]` sorted by slug; text has one
     line per profile in the form of "What each verb prints"; exit 0.
   - b. `list_counts_members_by_slug` (a pane whose `profile` is `SOME-PROFILE` counts for `Some Profile`).
   - c. `list_of_no_profiles`: text `no profiles`, JSON `data == []`, exit 0.
5. **(a) show.**
   - a. `show_reports_differences_field_by_field_in_both_formats`: a member at r2c1 against a spec at r1c1 gives text line
     `  herdr.grid: spec r1c1, live r2c1` and JSON row `status: "differs"` with that one difference; the raw JSON out contains
     `"live":{"row":2,"col":1,"pos":"r2c1"}`.
   - b. `show_reports_nothing_for_a_matching_pane`: status `matches`, `differences: []`, and no `spec ... live` line under it.
   - c. `show_reports_the_last_probe_result_without_running_one`: members with `probe.last = Ok` and `Failed { missing:
     ["qwen38"] }`: text lines `  probe: ok` and `  probe: failed (missing "qwen38")`; JSON `"probe":"ok"` and
     `"probe":{"failed":{"missing":["qwen38"]}}`; `FakeProber::calls()` empty.
   - d. `show_prints_command_and_check_as_argv_arrays`: text `  command: ["opencode","serve"]`; JSON arrays.
   - e. `show_lists_missing_and_extra_panes`.
   - f. `show_of_a_missing_profile_is_profile_not_found_in_both_formats`: exit 3, `error.code == "profile-not-found"`, nothing on
     `out` in text mode.
   - g. `show_passes_a_store_failure_through`: `FakePaneStore` with `Fault::Wedged` gives `timeout`, exit 1, both formats.
6. **(b) create.**
   - a. `create_makes_an_empty_profile_in_both_formats`: exit 0; the store holds NAME at generation 1 with no specs; JSON
     `data.members == []`; the pane store's call log has no `CasPut`.
   - b. `create_refuses_a_taken_name_or_slug`: `Some Profile` seeded; `create "Some Profile"` and `create "some-profile"` both exit
     3 with `profile-exists`; the store is unchanged (generation 1).
   - c. `create_reports_a_create_race_as_profile_exists`: `faults().fail_next(ProfileStoreOp::CasPut, PaneError::Conflict)` ->
     exit 3, `profile-exists`.
   - d. `create_from_makes_a_detached_copy`: `Alpha` has specs for `demo-c1r1` (a member of `Alpha`) and `demo-c9r9` (no
     record); `create Beta --from Alpha` gives `Beta.panes == Alpha.panes`; the pane store's call log has no `CasPut`; every pane
     record is unchanged (`demo-c1r1` still in `Alpha`).
   - e. `create_from_a_missing_profile_is_profile_not_found` (exit 3, nothing created).
   - f. `create_from_current_snapshots_every_pane_and_joins_it`: three panes at r1c1, r1c2, r2c1 with distinct cwd, model and
     effort, role, env names, ceilings, command, check and expect: one spec per pane equal to `spec_from_pane` of its record;
     each pane's record afterwards equals its seeded record except `profile == Some(NAME)` and `generation` + 1; JSON
     `data.members` lists the three names.
   - g. `create_from_current_refuses_a_pane_in_another_profile_and_writes_nothing`: one pane in `Other`: exit 3,
     `pane-in-other-profile`, message names the pane and `Other`; no profile NAME; no pane `CasPut` in the call log.
   - h. `create_from_current_undoes_everything_when_a_join_fails`: the Decision 13 seam fails the 2nd pane `cas_put` with
     `PaneError::Conflict`: exit 1, `generation-conflict`; afterwards no profile NAME (`get` is `None`) and every pane's
     `profile` is `None` (the first pane was put back).
   - i. `create_from_current_reports_profile_conflict_when_the_undo_fails`: the seam fails the 2nd join and the 3rd `cas_put`
     (the undo); exit 1, `profile-conflict`, the one-line message contains `holler profile delete` and `--keep-panes`.
   - j. `create_holds_env_names_never_values` (I7): after `--from-current` over panes with env `["ALPHA_TOKEN","BETA_URL"]`,
     every string in every `env` array of `data.profile.panes` and of the stored profile has no `=`; and decoding a
     `ProfileSpec` JSON with `"env":["TOKEN=s3cr3t"]` fails with `profile-secret-refused` whose text lacks `s3cr3t`.
   - k. `create_flags_conflict`: `verb_harness::parse::try_parse(&["profile","create","X","--from-current","--from","Y"])` is
     `Err` with `ErrorKind::ArgumentConflict`.
   - l. `create_with_a_bad_name_is_usage_in_both_formats`: `create "   "` exits 2, `error.code == "usage"`.
7. **(b) delete.**
   - a. `delete_removes_a_profile_without_members_in_both_formats`: exit 0, `get` is `None`, `data.detached == []`.
   - b. `delete_refuses_while_panes_are_live`: two members: exit 3, `profile-has-live-panes`, message names both panes and
     `--keep-panes`; profile and panes unchanged.
   - c. `delete_keep_panes_detaches_then_deletes`: exit 0; profile gone; both pane records still in the store, `profile == None`,
     every other field equal to before, generation + 1; `data.detached` lists both; text has the `detached, still running:`
     line; no adapter call (AC 3), so the panes "keep running".
   - d. `delete_of_a_missing_profile_is_profile_not_found` (exit 3).
   - e. `delete_conflict_without_members_is_generation_conflict`: `fail_next(ProfileStoreOp::Delete, PaneError::Conflict)` ->
     exit 1, `generation-conflict`.
   - f. `delete_conflict_after_a_detach_is_profile_conflict`: one member, `--keep-panes`, the same fault -> exit 1,
     `profile-conflict`; the member's `profile` is `None`; the profile still exists.
   - g. `delete_stops_at_a_failed_detach`: `fail_next(PaneStoreOp::CasPut, PaneError::Conflict)` with two members -> exit 1,
     `generation-conflict`; the profile still exists; `ProfileStoreOp::Delete` is not in the profile store's call log.
8. **(a)+(b) The stub cases are gone.** `grep -n 'assert_stub_routes' crates/holler-cli/tests/profile_verbs/{list,show}.rs` (a) and
   `.../{create,delete}.rs` (b) print nothing; in `crates/holler-cli/tests/pane_verbs/process/stub.rs` the `// #662` line remains and the four
   `("profile", "<verb>", 662)` lines are gone (`grep -c '662),' crates/holler-cli/tests/pane_verbs/process/stub.rs` prints `2` after 662a, which removes `list` and
   `show`, and `0` after 662b, which removes `create` and `delete`).
9. **(a)+(b) Surface.** `docs/adr/ADR-0003.md` rows read exactly (alignment with spaces, `#662` in the same column as today):
   `holler profile create NAME [--from-current | --from PROFILE]`, `holler profile delete NAME [--keep-panes]`,
   `holler profile list`, `holler profile show NAME`. The `# #662` group of `cli-surface.txt` is:
   `profile create | Demo`, `profile create | "Some Profile" --from-current`, `profile create | Copy --from Demo --format=json`,
   `profile delete | Demo`, `profile delete | Demo --keep-panes --json`, `profile list |`, `profile list | --format=json`,
   `profile show | Demo`, `profile show | "Some Profile" --json`. Then `cargo test -p holler-cli --test cli_surface_test`,
   `cargo test -p holler-cli --test docs_cli_test` and `cargo test -p holler-cli --test pane_cli_process` pass.
10. **(a)+(b) ADR-0021** carries Decision 14's text: `grep -n 'Deferred to #662' docs/adr/ADR-0021.md` prints nothing;
    `grep -n 'fixed:<port>' docs/adr/ADR-0021.md` prints the section 3 line; (b) `grep -n 'profile create. |.*profile-conflict' docs/adr/ADR-0021.md` and
    `grep -n 'profile delete. |.*profile-conflict' docs/adr/ADR-0021.md` each print one line (the section 9 rows).
11. **(a)+(b) Crate tests.** `cargo test -p holler-cli --test profile_verbs` and `cargo test -p holler-pane` pass, the latter
    running AC 1-2 (a: `cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test` lists every AC 1-2
    name); `cargo test --workspace` passes.
12. **(a)+(b) Lints.** `cargo clippy --workspace --all-targets -- -D warnings` and `bash scripts/lint.sh` pass (every `#[allow]`
    in a touched test file, the two `holler-pane/tests/profile_*_test.rs` headers and `profile_verbs/rig.rs` included, carries
    `// #662`; every touched file under 900 lines; every function under 100 lines).
13. **(a)+(b) No new `unsafe`, no new dependency.** `git diff origin/main -- '*.rs' | grep -n '^+.*unsafe'` prints nothing;
    `git diff origin/main -- '**/Cargo.toml' Cargo.lock` prints nothing; `cargo machete` reports nothing new.
14. **(a)+(b) Formatting.** `rustfmt --check --edition 2021` on every `.rs` file this run touched exits 0 (C5), the new
    `crates/holler-pane/tests/profile_{snapshot,diff}_test.rs` and `crates/holler-cli/tests/profile_verbs/rig.rs` included (a).
15. **(a)+(b) CHANGELOG.** `CHANGELOG.md` under `## [Unreleased]` / `### Enhancements` gains one entry (662a: the snapshot, the
    comparison, `profile list` and `profile show`; 662b: `profile create` and `profile delete`) linking
    `[#662](https://github.com/Performant-Labs/holler/issues/662)`; `bash scripts/changelog-check.sh` prints `changelog-check: ok`.

## Files

Production (F):
- (a) `crates/holler-pane/src/profile_snapshot.rs` (~70 lines): the API above.
- (a) `crates/holler-pane/src/profile_diff.rs` (~260 lines): the API above.
- (a) `crates/holler-cli/src/profile/list.rs` (~90), `show.rs` (~200): replace the stubs.
- (b) `crates/holler-cli/src/profile/create.rs` (~260), `delete.rs` (~150): replace the stubs.
- Docs: `docs/adr/ADR-0003.md` (the four rows), `docs/adr/ADR-0021.md` (Decision 14), `CHANGELOG.md`.

Tests (T):
- (a) `crates/holler-pane/tests/profile_snapshot_test.rs` (AC 1) and `profile_diff_test.rs` (AC 2): new, over `tests/common`
  (Decision 11); `tests/common/mod.rs` is not edited.
- `crates/holler-cli/tests/profile_verbs/rig.rs` (a; new, the shared rig, Decision 12), `list.rs` (a; AC 4, declares `rig` by
  `#[path]`), `show.rs` (a; AC 5), `create.rs` (b; also the Decision 13 seam), `delete.rs` (b). Each verb file replaces its stub
  case.
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: delete the #662 STUBS entries, keep `// #662`.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: the `# #662` group (AC 9).

Not touched: `profile/mod.rs`, `cli.rs`, `lib.rs`, `output.rs`, `wiring.rs`, `holler-pane/src/{lib,profile,pane,error,ports}.rs`,
every manifest, the test kit, `tests/profile_verbs/main.rs`, `tests/verb_harness/**`, `docs_rows.rs`.

**Blast radius:** the issue's six files, plus the verb-owned surface (ADR 0003 rows, the fixture group, `profile_verbs/<verb>.rs`,
the STUBS entries), ADR-0021 (Decision 14) and `CHANGELOG.md`.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `output::{emit, emit_error, ErrorBody, VerbCtx}` (E7) | every result and error | reuse; no output code in the verbs beyond their `text` closures |
| `holler_pane::error::class_of` via `emit` | exit codes | reuse; no table in the verbs |
| `PaneError` closed variants (E6) | every code | reuse; no `Refused`, no new code |
| `ProfileName::parse`, `slug()` (E4) | names, membership by slug | reuse; no second slug rule |
| `GridPos` `Display`/`Serialize` (E4) | `r2c1`, row-first JSON | reuse through `FieldValue::Grid` |
| `Argv`, `EnvVarName` (E4) | argv and env values | reuse; argv printed as JSON, never joined |
| `ProbeResult` serde form (E4) | show's `probe` | reuse as is |
| `spec_from_pane` | create, `diff_spec`, #650 | new here, one copy |
| `is_member` | list, show, delete (and #643, #663, #664/#665) | new here, one copy (not three inline filters) |
| `diff_profile` / `diff_spec` | show, #664, #665 | new here, one copy |
| `crates/holler-pane/tests/common` (`pane`, `spec`, `profile`, `pane_name`, `profile_name`) | AC 1-2 | reuse; not edited |
| test kit fakes, `sample_pane`/`sample_spec`/`sample_profile`, `check_envelope`, `verb_harness::run_verb_with`, `parse::try_parse` | every verb test | reuse; the only new test seams are `rig.rs` and the Decision 13 wrapper |

**Forward-compat (consumers of this story's API):**

| Consumer | Needs | Satisfied |
|---|---|---|
| #650 migration import | the snapshot mapping, to build `fleet`; "`profile show fleet` reports no difference" | `spec_from_pane`, `profile_from_panes`; AC 2c |
| #664 `profile apply` | a plan from the comparison, matching `show` on the same fixture, positions rowcol | `diff_profile` (pass the panes apply considers), `PaneStatus` (missing -> create, matches -> adopt, differs -> report), typed `FieldDiff` |
| #665 `profile-drift` (PROPOSED) | per-pane differences | `diff_spec`, `is_member` |
| #649 integration scenario | create from current, show, list, delete refused while live then `--keep-panes`, every JSON output parses | the verbs; codes and shapes pinned here |
| #644 launch/relaunch (amended `7195993`; **hard consumer**, merges after 662a) | relaunch's base spec; `port_of_policy` parses with the prefix and needs `port_of_policy(&fixed_port_policy(p)) == Ok(p)`; its T greps for the three signatures verbatim | `spec_from_pane`, `FIXED_PORT_POLICY_PREFIX`, `fixed_port_policy`, signatures **frozen** as written above. A record at port 0 snapshots to `fixed:0`, which #644 refuses (`usage`) |
| #663 `StoreScope::resolve(P, None)` | its member filter over `PaneStore::list` (slug compared inline today) | `is_member(pane, &P)`, to reuse when 662a merges first |
| #643 `pane watch --profile P` / `list --profile P` | its member filter (slug compared inline today) | `is_member(pane, &P)`, to reuse when 662a merges first |

## Out of scope

- `profile apply`, `rename`, `export`, `import` (#664, #665); the `profile-drift` finding (#665).
- Running a probe (#663's runner; `show` only reads `probe.last`).
- Wiring the real stores into the binary (`wiring.rs`, #649): until then the real `holler profile ...` answers `not-implemented`
  from `Unwired`, which `pane_cli_process` no longer asserts for these verbs (AC 8).
- Any Herdr, tmux or OpenCode call; any live fleet or real session.
- A pane filter for `--from-current`; redacting argv elements.

## Follow-ups (not this story; the orchestrator files them if wanted)

- **Port policy in the kit.** When the kit's `sample_spec` moves to `fixed:48100` (#644's kit follow-up), the profile_verbs
  tests drop their `port_policy` override; #664/#665 get the port comparison through `diff_spec` unchanged. A spec stored
  with a bare `fixed` before #644 shows `harness.port_policy` as differing until rewritten.
- **One base rig.** #643 (`pane_verbs/list.rs`), #644 (`pane_verbs/launch_rig.rs`) and this story (`profile_verbs/rig.rs`) each
  build a rig over the same seven fakes in two test targets. O agrees one base with #643's A W-3 and a later story moves it
  under `tests/verb_harness/`, included by `#[path]` from both targets.
- **Shared read-verb text forms** (for O to settle with #643's A W-2, before the second of #643, #647, #662 merges). This
  brief keeps its probe form, the issue's acceptance text. Divergences from #643's `pane get`: probe `failed (missing
  "qwen38")` here vs `failed missing=["qwen38"]`; absent marker `none` vs `-`; keys `host.cwd`/`herdr.grid` vs
  `project`/`pos`; escaping control characters only (`char::escape_default`, as `holler_proto::log::escape_field_value`) vs
  `text_value`'s `{:?}` quoting. JSON already agrees (both use the records' serde). `FieldValue`'s `Display` stays in
  holler-pane whatever is chosen (that crate cannot import a CLI helper).
- A cross-registry guard for `delete`: a pane that joins NAME between `delete`'s member listing and its delete leaves a pane
  naming a deleted profile (the two registries share no transaction, ADR-0021 section 8). Reconcile (#647) or doctor should
  report a pane whose `profile` names no profile.

## Test plan

RED (a compile error is not RED; the tester overlay): **the allowed approach** is that T first lands **signature stubs** with
the exact public API above and no logic: `profile_snapshot.rs` and `profile_diff.rs` with every item, bodies returning
wrong-but-typed values (`fixed_port_policy` -> `String::new()`, `spec_from_pane` -> a spec of empty strings at r1c1 with zero
ceilings, `profile_from_panes` -> no specs, `diff_spec`/`diff_profile` -> `Vec::new()`, `is_member` -> `false`, `value` ->
`FieldValue::Text(String::new())`, `Display` -> empty); the verb files with their new `Args` structs and `run` still returning
`emit_error(.., not_implemented(STORY))`. Then every behaviour test fails on its assertion: the verb tests on `exit 1,
not-implemented` where they expect 0/2/3 and data; the pure tests in `crates/holler-pane/tests/profile_{snapshot,diff}_test.rs`,
which the `holler-pane/src` signature stubs compile, on wrong values (2c and 5b pass vacuously against the stubs;
that is expected, and T journals which tests are RED and which pass for want of behaviour). The fixture lines of AC 9 do not parse against
today's `ProfileShow {}` / `ProfileCreate {}` / `ProfileDelete {}` (no positional) and parse once the signature stubs land;
they are a surface check, not a RED behaviour test. T journals the RED run with the failing assertions.

GREEN: F replaces the stubs; then `cargo test -p holler-cli --test profile_verbs`, `-p holler-pane`, `--test cli_surface_test`,
`--test docs_cli_test`, `--test pane_cli_process`, then the AC 11-15 gates.

Every verb test uses only the test kit's fakes and fixtures, in-process through `run_verb_with`; no subprocess except the
existing `pane_cli_process` target; no temporary directory is needed; no real session name appears. The pure tests (AC 1-2)
call no port, so they need no fake; they use holler-pane's own `tests/common` records (Decision 11), which is the issue's "test
kit only" in substance: no live fleet, session or adapter.

## Risks

- **Partial writes across two registries.** `create --from-current` writes the profile and then N panes; `delete --keep-panes`
  writes N panes and then the profile. The stores share no transaction (ADR-0021 section 8). Decision 8's undo and the
  `profile-conflict` reconcile message bound the damage; AC 6h, 6i, 7f, 7g pin each branch. A crash between writes leaves a
  state the next `show` reports (a member with no spec is `extra`).
- **The create race on the real store** is `generation-conflict`, not `profile-exists` (E8); Decision 7 maps it, AC 6c pins it.
- **JSON key order.** A `serde_json::Value` map would print `col` before `row` in a build without `preserve_order` (E7). The
  typed `FieldValue` and derived structs avoid it; AC 2a and 5a assert the raw string.
- **Terminal injection from stored strings.** `cwd`, `workspace`, model strings and a probe's error reason are not grammar-
  checked by the records. Text mode escapes control characters (`FieldValue` `Display`, `{:?}` for probe text); JSON escapes
  them by construction. AC 2f pins it.
- **Secrets.** Env entries are names by type (I7, AC 6j). Argv elements (`command`, `check`) are printed as stored; Holler has
  no redaction rule for them and the issue asks `show` to print them, so an operator who puts a token in an argv sees it in
  `show`. Recorded, not changed here.
- **No shell anywhere.** Argv is printed as a JSON array, never joined into a command line someone could paste into a shell
  with different splitting.
- **Port policy grammar.** `fixed:<port>` is the first concrete policy string, and #644 (amended, `7195993`) adopts it as the
  only form and pins the three `profile_snapshot` signatures verbatim; changing them breaks #644's pre-flight grep.
- **Two text forms for one value** until the read-verb forms are settled with #643 (Follow-ups); JSON is unaffected.
- **The real binary still answers `not-implemented`** for these verbs until #649 wires the stores; that is by design (E9 wiring
  docs) and not a regression.
