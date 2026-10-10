# Brief: #662b profile write verbs (`holler profile create`, `holler profile delete`)

Repo: Performant-Labs/holler. Issue: #662 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**THIS RUN IS 662b ONLY.** 662a (`profile list`, `profile show`, `profile_snapshot.rs`, `profile_diff.rs`) merged to `main` as
PR #703 (`ce12cdb`). This run implements `holler profile create` and `holler profile delete` and the (b) items the 662a brief
deferred to it, nothing else. **The PR says `Closes #662`** (662b closes the issue).

**Branch:** `issue-662-implementation` (worktree `.claude/worktrees/0662-profile-verbs`, from `origin/main` at `ce12cdb`).
**Review-rigor:** second-opinion (operator through the MO; the outside model is deepseek-v4-pro). The issue's own Pipeline line
says `rigor: in-session`; the operator's instruction wins (C1 below).
**Design:** N/A (no UI surface). **Handoffs:** `docs/handoffs/662/handoff-<phase>.md`; journal `docs/handoffs/662/decisions.md`.
**Public repository:** no host, tailnet or account name appears in code, tests, docs, commits or the PR. Test data uses the
test kit's neutral names (`demo-c1r1`, `scratch`, `/srv/demo`, `demo-provider`, profiles `Some Profile`, `Alpha`, `Other`).

**Does 662b wait for #663? No: the run can start now.** #663 (branch `issue-663-implementation`, `93fb653`, no PR yet) is
not a dependency:
1. The epic gives #662 only #637, #638 and #670 (E2, lines 148 and 172), all merged.
2. No 662b code calls `ProfileScope` or uses a #663 item. `create` and `delete` find a profile's members through
   `profile_store.get`, `pane_store.list()` and 662a's `is_member` (E8), and the rig's `scope` is `Unwired` (E11), so a scope
   call would fail every test.
3. #663's `reconcile_step` / `RECONCILE_STEP_UNSCOPED` (E15) are the pane-doctor step for a record write that loses after a
   *live* act. `create` and `delete` make no live act; their remedies name profile verbs (carried behaviour, below).
4. Files: #663 touches `pane/profile_scope.rs`, `probe.rs`, `CHANGELOG.md` and ADR-0021 **prose only, no table row** (E15).
   662b touches two **table rows** of ADR-0021 section 9. No shared hunk; `CHANGELOG.md` is the usual keep-both-entries
   conflict for whichever merges second.

The one soft overlap is the POSIX single-quoting of a profile name in a suggested command (Decision B1): #663 has a private
`single_quoted` (E15); 662b adds its own `pub(crate)` copy. Folding them is a Follow-up, not a blocker.

## Size check

F edits **2 production files** (`crates/holler-cli/src/profile/create.rs` ~300 lines, `delete.rs` ~170) and **3 doc files**
(`docs/adr/ADR-0003.md` two rows, `docs/adr/ADR-0021.md` two rows, `CHANGELOG.md` one entry). T edits
`crates/holler-cli/tests/profile_verbs/create.rs` (~450, with the Decision 13 seam), `delete.rs` (~250),
`crates/holler-cli/tests/pane_verbs/process/stub.rs` (2 lines out) and `crates/holler-cli/tests/fixtures/cli-surface.txt`
(the `# #662` group). This is the 662b row of 662a's Size check. No split.

## Problem

`holler profile create` and `holler profile delete` are still the #670 stubs that refuse with `not implemented (story #662)`
(E3). Without `create`, no profile can be made, from nothing, from another profile, or from the running panes, so
`holler pane launch --profile P` (#644) has no P to name, the scenario of #649 cannot start, and `profile apply` (#664) has
nothing to apply. Without `delete`, a profile can never be removed, and no pane can leave a profile without a `pane` verb.
This run fills the two files against the frozen ports (`ProfileStore`, `PaneStore`) and `output::emit()`, reusing 662a's
`profile_from_panes`, `is_member` and `count`, and tests them only on the test kit's fakes.

## Evidence (verbatim, as of `ce12cdb` unless marked)

Every quote below was checked against the worktree at `ce12cdb`; E1 and E2 are the GitHub issue and epic bodies as
`gh issue view` prints them (line numbers are the body's own); E15 is from the #663 branch at `93fb653`; "carried" quotes are
the merged 662a brief, `git show 31062ce:docs/handoffs/662-brief.md`.

### E1. Issue #662, whole body (the source of truth; unchanged since 662a, no comments)

```
gh issue view 662, body lines 1-36
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

### E2. Epic #633: the contract lines, the dependency rows and the rulings this run builds on

```
gh issue view 633, body lines 88-90
Port            ProfileStore get / list / cas_put(profile, expected_generation) / delete(name, expected_generation) / watch / log(name)
Helper trait    ProfileScope resolve(profile, pane) / edit_spec(profile, pane, edit, act)   // implemented in holler-cli/src/pane/profile_scope.rs
Profile verbs   holler profile  create NAME [--from-current | --from PROFILE] | delete NAME [--keep-panes] | list | show NAME
```
```
gh issue view 633, body lines 109-115
                Membership (amended 2026-10-08, review) is Pane.profile only; a ProfileSpec may name a pane that belongs to another profile (a detached spec,
                  e.g. `create --from`); setting Pane.profile on a pane already in another profile is refused: pane-in-other-profile
Output          --format=text|json on every holler pane / holler profile verb and the roster; one envelope:
                  {"schema_version": 1, "ok": true|false, "data": ..., "error": null | {"code": "<stable kebab-case>", "message": "<one line>"}}
                watch emits NDJSON (one envelope per line); json mode prints only the JSON on stdout (diagnostics on stderr);
                exit codes identical in both formats (0 ok, 1 failure, 2 usage, 3 refusal) (amended 2026-10-09, decisions: was "1 refused/failed"; ADR-0021 section 9); every error has a stable code;
                one module holler-cli/src/output.rs (types and emit() fixed by #637), each verb supplies only its data
```
```
gh issue view 633, body lines 124-131
Invariants      I1 only `holler pane` verbs change a pane or its registration
                I2 session_of_record is the only session a pane's TUI shows and the hub drives
                I3 every verb is plan -> act -> observe -> record; a mismatch fails loudly and records nothing
                I4 no verb changes a session by typing into a TUI
                I5 a verb returns in bounded time (default 10 s); long work returns an operation id
                I6 state is observed from Herdr, tmux and the harness, never inferred from files
                I7 a profile holds names and values that are not secrets, never a secret        (amended 2026-10-08, profiles)
                I8 an edit through --profile and the live change are one transaction           (amended 2026-10-08, profiles)
```
```
gh issue view 633, body lines 148-148
| 3 | #640 Herdr adapter, #641 host adapter, #642 OpenCode adapter, #643 read verbs, #644 launch/relaunch, #645 switch/reset, #646 park/close/routed say, #647 reconcile and doctor; #660 `--format=json` output module, #661 profile registry, #662 `holler profile create/delete/list/show`, #663 the `--profile` helper and the probe runner (amended 2026-10-08, profiles); #651 Herdr display plugin (optional, moved from wave 4 (amended 2026-10-08, review)) | #637 and #638 merged; (amended 2026-10-09, agent) #642, #644 and #647 also need #700; #640 also needs #636; #642 also needs #635; #661 also needs #639 (amended 2026-10-08, profiles); #651 needs #639 and #636 only (amended 2026-10-08, review) |
```
```
gh issue view 633, body lines 172-172
- #643, #644, #645, #646, #647, #650, #660, #662, #663, #664, #665 (CLI verbs and output): #637 and #670.
```
```
gh issue view 633, body lines 179-181
2. **One verb, one file, one owning story, including its clap `Args` struct** (positionals and verb-specific flags), its ADR 0003 row, its `cli-surface.txt` line and its own `tests/pane_verbs/<verb>.rs` or `tests/profile_verbs/<verb>.rs`. The frozen shared files declare only the shared flag groups (`SpecFlags`, `ProfileOpt`, `SpecOnly`) and `--take-over`.
3. **Codes are constants in each verb's own file** built through `PaneError::Refused` or the closed `PaneError` set in `holler-pane/src/error.rs`; no verb edits that enum (infrastructure variants `timeout`, `pane-not-found`, `session-not-found`, `store-corrupt`, `unavailable` and `profile-exists`, `profile-has-live-panes` are already in it).
4. **Formatting:** the tree is not rustfmt-clean and CI has no fmt step. Read the rules' "`cargo fmt`" as: new `.rs` files pass `rustfmt --check --edition 2021`; existing files are not reformatted.
```
```
gh issue view 633, body lines 186-186
9. **A closed pane's record is removed.** `PaneStore::delete(name, expected_generation)` and the `pane/delete` method (so `PANE_METHODS` has five names) are added to the contract; `close` (#646) uses it, #639 implements it, #638's fakes provide it.
```

### E3. The two stubs this run replaces, and the frozen dispatcher (not edited)

`mod.rs` declares every verb module `pub`, so a `pub(crate)` item of `create.rs` or `delete.rs` is reachable crate-wide.

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
crates/holler-cli/src/profile/delete.rs:1-19
//! `holler profile delete`: a stub (story #670). Story #662 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 662;

/// Delete a profile.
#[derive(Args, Debug)]
pub struct ProfileDelete {}

/// Run `holler profile delete`: refuse, naming the story that owns it.
pub fn run(_args: &ProfileDelete, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
```
crates/holler-cli/src/profile/mod.rs:13-20
pub mod apply;
pub mod create;
pub mod delete;
pub mod export;
pub mod import;
pub mod list;
pub mod rename;
pub mod show;
```
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

### E4. The records (frozen by #637; not edited)

```
crates/holler-pane/src/profile.rs:30-35
/// The display name of a profile: spaces allowed, e.g. `Some Profile`.
///
/// Parsing trims surrounding whitespace and refuses a name that is empty, longer
/// than 64 characters, has a control character, or has no ASCII letter or digit
/// (the slug would be empty, and the slug is persisted and unique-checked by #661).
/// All of these are `usage`. Serde goes through [`ProfileName::parse`].
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
crates/holler-pane/src/profile.rs:106-117

/// Who made a profile write: a non-empty name of at most 64 characters (a person, a
/// verb such as `holler profile apply`, a watchdog). It becomes the "who" of a log
/// entry, so control characters are refused (`usage`), and surrounding whitespace is
/// trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Actor(String);

impl Actor {
    /// Parse an actor name (see the type docs).
    pub fn parse(text: &str) -> Result<Self, PaneError> {
```
```
crates/holler-pane/src/profile.rs:209-221
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
crates/holler-pane/src/pane.rs:223-254
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

### E5. The ports and the generation rule (frozen; not edited)

```
crates/holler-pane/src/ports.rs:54-79
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

```
```
crates/holler-pane/src/profile.rs:328-354
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

### E6. The closed error set: the variants used here, their text, and their exit class (not edited)

`profile-exists`, `profile-not-found`, `profile-has-live-panes`, `pane-in-other-profile` are refusals (exit 3);
`generation-conflict` (`PaneError::Conflict`) and `profile-conflict` are failures (exit 1); `timeout` is a failure (exit 1).

```
crates/holler-pane/src/error.rs:423-446
    /// `generation-conflict`: a compare-and-swap write named a generation the record
    /// has moved past (the CAS conflict; see [`crate::generation`]).
    Conflict,
    /// `probe-failed`: the health probe of a pane did not pass; `message` says what
    /// was missing. (#644/#663.)
    ProbeFailed { message: String },
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
crates/holler-pane/src/error.rs:655-667
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
            PaneError::ProfileSecretRefused => f.write_str(
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

### E7. The output module (not edited)

`ErrorBody`'s two fields are `pub` and `ErrorCode::from(&PaneError)` is public, so a verb can report error `E`'s code with
its own one-line message: `ErrorBody { code: ErrorCode::from(&e), message: ... }`. `emit` derives the exit code from the code.

```
crates/holler-cli/src/output.rs:126-141
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
crates/holler-cli/src/output.rs:191-214
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
```
```
crates/holler-cli/src/output.rs:240-243
/// Print an error (a failed `emit` with no data) and return its exit code.
pub fn emit_error(sink: &mut Sink<'_>, format: Format, error: ErrorBody) -> i32 {
    emit(sink, format, Err::<(), _>(error), |()| String::new())
}
```

### E8. 662a's code on `main` that this run reuses (not edited)

`profile_from_panes` and `spec_from_pane` (the snapshot), `is_member` (membership by slug), `list.rs`'s shared `count`, and
the `profile-not-found` form `show` uses. A pane name needs no quoting in a command line (`findings.rs`, #647).

```
crates/holler-pane/src/profile_snapshot.rs:183-183
```
```
crates/holler-pane/src/profile_snapshot.rs:207-218
```
```
crates/holler-pane/src/profile_diff.rs:254-265
/// Whether `pane` is a member of `profile`: its `profile` has `profile.slug()`. Slugs are
/// compared on both sides, since the slug is a profile's identity (a pane whose profile
/// is `SOME-PROFILE` is a member of `Some Profile`), like the test kit's fakes do.
///
/// This is what "a live pane of a profile" means (ADR-0021 section 3): `profile list`'s
/// live count, `profile show`'s comparison and `profile delete`'s refusal
/// `profile-has-live-panes` all use it.
pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
    pane.profile
        .as_ref()
        .is_some_and(|own| own.slug() == profile.slug())
}
```
```
crates/holler-cli/src/profile/list.rs:91-99
/// `n` and `noun`, with the noun in the plural unless `n` is 1: `1 pane`, `2 panes`,
/// `0 specs`.
pub(crate) fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
```
```
crates/holler-cli/src/profile/show.rs:77-90
fn view(args: &ProfileShow, ports: Ports<'_>) -> Result<ProfileView, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let profile = ports
        .profile_store
        .get(&name)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", name.as_str()),
        })?;
    let members: Vec<Pane> = ports
        .pane_store
        .list()?
        .into_iter()
        .filter(|pane| is_member(pane, &profile.name))
        .collect();
```
```
crates/holler-pane/src/findings.rs:12-15
//! - **Remedies** ([`FindingKind::remedy`]) name only the verb that owns each repair
//!   (`pane relaunch`, `pane reset`, `pane doctor`), built from constant words and a
//!   [`PaneName`], whose grammar has no space, shell metacharacter or leading `-`. No remedy
//!   carries a session id, a Herdr id, a port, a profile or any adapter text, and none is a
```
```
crates/holler-pane/src/error.rs:456-456
    Timeout { op: String },
```

### E9. The hub's registries (#661, #639, merged): what the real stores answer (not edited)

So on the real store: `get` finds a profile by slug; a create at 0 of a name stored **under the same display name** is
`generation-conflict` (`next_generation(1, 0)`), and under another spelling of the same slug `profile-exists`; a pane that
belongs to P cannot be written into Q in one write (`pane-in-other-profile`, inside the pane CAS); a pane write naming a profile
that does not exist is `profile-not-found`, so the profile must exist **before** a pane joins it.

```
crates/holler-hub/src/profile/store.rs:162-166
    /// The profile filed under the slug of `name`, or `None`. Another spelling of the name
    /// finds it (D6).
    pub(crate) fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.read(|table| table.record(name).cloned())
    }
```
```
crates/holler-hub/src/profile/store.rs:190-205
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
crates/holler-hub/src/panes/store.rs:337-357

/// The membership rule of a write (#661; ADR-0021 §8 and "Decisions taken", item 2): a
/// pane stored in one profile cannot be written into another, `pane-in-other-profile`.
/// Leaving (`None`), joining from `None` and keeping the profile are allowed, so a move
/// takes two writes, leave and then join. Profiles are compared by slug, the profile
/// registry's identity, so another spelling of the same profile keeps it. Whether the
/// profile exists is the `pane/cas_put` hook's check (`crate::profile::check_membership`).
fn refuse_profile_move(stored: Option<&Pane>, next: &Pane) -> Result<(), PaneError> {
    let current = stored.and_then(|pane| pane.profile.as_ref());
    match (current, next.profile.as_ref()) {
        (Some(current), Some(other)) if current.slug() != other.slug() => {
            Err(PaneError::PaneInOtherProfile {
                what: format!(
                    "{} is in profile {:?}, not {:?}",
                    next.name,
                    current.as_str(),
                    other.as_str()
                ),
            })
        }
        _ => Ok(()),
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

### E10. The test kit's fakes and fault switch (#638, merged; not edited)

`fail_next` fails only the **next** call of a method (that is why the undo tests need Decision 13's seam). The fakes' `put`
applies the membership rule only for port writes; seeding uses the port path at 0. `FakeProfileStore::get` looks up by slug.
`grep -rn 'ASSUMPTION *(#662)' crates/holler-pane-testkit` prints nothing (checked at `ce12cdb`).

```
crates/holler-pane-testkit/src/pane_store.rs:50-56
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
/// - `delete` of a missing record is `pane-not-found`, whatever the generation.
/// - `list` is sorted by name.
/// - A pane that belongs to a profile cannot move to another profile in one write
///   (`pane-in-other-profile`); it leaves its profile (`None`) first. Profiles are
```
```
crates/holler-pane-testkit/src/pane_store.rs:64-67
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
crates/holler-pane-testkit/src/pane_store.rs:130-136
        self.feed.write(|log| {
            let stored = log.get(&pane.name);
            let current = stored.map_or(0, |stored| stored.generation);
            let generation = next_generation(current, writer.expected(current))?;
            if let Writer::Port(_) = writer {
                check_membership(stored, pane)?;
            }
```
```
crates/holler-pane-testkit/src/pane_store.rs:187-190
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.faults.enter(PaneStoreOp::CasPut)?;
        self.put(pane, Writer::Port(expected_generation))
    }
```
```
crates/holler-pane-testkit/src/profile_store.rs:58-60
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
```
```
crates/holler-pane-testkit/src/profile_store.rs:73-78
/// - Records, events and the log are filed by the slug of the profile's name, and the
///   stored slug is always the name's: a submitted slug is ignored, as the submitted
///   generation is.
/// - The name rule: a `cas_put` of a name whose slug is stored under another name is
///   `profile-exists`, whatever the generation. It is checked before the generation,
///   so a create of such a name at 0 is `profile-exists`, not `generation-conflict`.
```
```
crates/holler-pane-testkit/src/profile_store.rs:250-253
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.faults.enter(ProfileStoreOp::Get)?;
        Ok(self.feed.read(|log| log.get(&name.slug()).cloned()))
    }
```
```
crates/holler-pane-testkit/src/fault.rs:27-35
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
    /// timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
    Fail(PaneError),
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

### E11. The rig 662a built, the verb harness, and the two stub test files this run replaces

`rig.rs` is #662's own file (662a); `main.rs` is #670's and is not edited.

```
crates/holler-cli/tests/profile_verbs/rig.rs:29-45
/// The fakes a profile verb runs over.
pub(crate) struct Rig {
    pub panes: FakePaneStore,
    pub profiles: FakeProfileStore,
    pub herdr: FakeHerdr,
    pub host: FakeHost,
    pub harness: FakeHarness,
    pub prober: FakeProber,
}

impl Rig {
    /// A rig whose pane store holds `panes` and whose profile store holds `profiles`,
    /// each seeded in order (so each record is stored at generation 1).
    pub fn new(
        panes: impl IntoIterator<Item = Pane>,
        profiles: impl IntoIterator<Item = Profile>,
    ) -> Self {
```
```
crates/holler-cli/tests/profile_verbs/rig.rs:57-81
    pub fn ports(&self) -> Ports<'_> {
        Ports {
            pane_store: &self.panes,
            profile_store: &self.profiles,
            herdr: &self.herdr,
            host: &self.host,
            harness: &self.harness,
            scope: &UNWIRED,
            prober: &self.prober,
        }
    }

    /// Run `holler <argv...>` over this rig.
    pub fn run(&self, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(argv, format, self.ports())
    }

    /// No Herdr, host, harness or probe call was made through this rig (AC 3).
    pub fn assert_no_adapter_call(&self) {
        assert_eq!(self.herdr.faults().calls(), vec![], "no Herdr call");
        assert_eq!(self.host.faults().calls(), vec![], "no host call");
        assert_eq!(self.harness.faults().calls(), vec![], "no harness call");
        assert_eq!(self.prober.calls(), vec![], "no probe run");
    }
}
```
```
crates/holler-cli/tests/profile_verbs/rig.rs:91-94
/// Run `argv` with `Format::Text` on `seed()` and with `Format::Json` on another
/// `seed()`. Asserts the exit codes are equal, the JSON output passes the test kit's
/// envelope checker, and neither run called an adapter or the prober.
pub(crate) fn run_both(seed: impl Fn() -> Rig, argv: &[&str]) -> Both {
```
```
crates/holler-cli/tests/profile_verbs/rig.rs:114-116
/// Assert `both` is a failure coded `code` at exit `exit`: text mode prints nothing on
/// `out` and the message on `err`; JSON mode prints the envelope with that code.
pub(crate) fn assert_failure(both: &Both, code: &str, exit: i32) {
```
```
crates/holler-cli/tests/profile_verbs/rig.rs:132-160
/// The sample pane `name` (a valid name is a test's own constant).
pub(crate) fn pane(name: &str) -> Pane {
    sample_pane(name).unwrap()
}

/// The sample pane `name`, a member of `profile`.
pub(crate) fn member(name: &str, profile: &str) -> Pane {
    Pane {
        profile: Some(ProfileName::parse(profile).unwrap()),
        ..pane(name)
    }
}

/// A spec equal on every compared field to a seeded `pane(name)`: the kit's
/// `sample_spec` with the port policy of the sample panes' one port (Decision 3). `name`
/// is plain text, as a spec's pane is, so it need not be a valid pane name.
pub(crate) fn matching_spec(name: &str) -> ProfileSpec {
    let mut spec = sample_spec(name);
    spec.harness.port_policy = fixed_port_policy(pane("demo-c1r1").harness.port);
    spec
}

/// A profile named `name` holding `specs`, in order.
pub(crate) fn profile(name: &str, specs: Vec<ProfileSpec>) -> Profile {
    Profile {
        panes: specs,
        ..sample_profile(name, &[]).unwrap()
    }
}
```
```
crates/holler-cli/tests/profile_verbs/list.rs:4-5
#[path = "rig.rs"]
pub(crate) mod rig;
```
```
crates/holler-cli/tests/verb_harness/parse.rs:23-26
/// `Cli::try_parse_from` of `holler <argv...>`.
pub fn try_parse(argv: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(full(argv))
}
```
```
crates/holler-cli/tests/verb_harness/mod.rs:52-54
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
```
```
crates/holler-cli/tests/profile_verbs/create.rs:1-9
//! `holler profile create`: the stub case of story #670. Story #662 owns the real verb
//! and replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn profile_create_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["profile", "create"], 662);
}
```
```
crates/holler-cli/tests/profile_verbs/delete.rs:1-9
//! `holler profile delete`: the stub case of story #670. Story #662 owns the real verb
//! and replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn profile_delete_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["profile", "delete"], 662);
}
```

### E12. The surface files a verb story owns (epic ruling 2): STUBS, the fixture, the ADR 0003 rows

`docs_cli_test` parses every `holler ...` row of ADR 0003 (and every backticked `holler ...` span under `docs/`) against the
clap tree after the normalisation below, so the bare rows `holler profile create` / `delete` stop parsing once `NAME` is
required. `cli_surface_test` parses every fixture line with `Cli::try_parse_from`. Both must change here.

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
crates/holler-cli/tests/pane_verbs/process/stub.rs:33-38
    // #650
    ("pane", "import", 650),
    // #662
    ("profile", "create", 662),
    ("profile", "delete", 662),
    // #664
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:3-8
# One invocation per line:  <leaf verb path> | <arguments>
# Every line must parse with `Cli::try_parse_from`, and every leaf verb clap
# knows must appear on at least one line (both asserted by
# tests/cli_surface_test.rs). A verb ADR 0003 specifies that no story has
# implemented yet lives in cli-surface.pending.txt instead — move its lines
# here in the story that adds the verb.
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:160-166
# #662
profile create |
profile delete |
profile list |
profile list | --format=json
profile show | Demo
profile show | "Some Profile" --json
```
```
docs/adr/ADR-0003.md:65-68
holler profile create                                             #662
holler profile delete                                             #662
holler profile list                                               #662
holler profile show NAME                                          #662
```
```
crates/holler-cli/tests/docs_cli_test.rs:14-18
//! Placeholders are normalised (`<x>` → `x`, `[optional]` dropped, `a|b` →
//! `a`, `…` dropped, a trailing annotation after two spaces / ` (` / ` — `
//! cut). A command whose leaf verb is in `cli-surface.pending.txt` is
//! tolerated (specified, not yet implemented) and reported, never silently
//! skipped.
```

### E13. ADR-0021: what 662a recorded, the two-registries rule, and the section 9 rows this run edits

"Deferred to named stories" (lines 528-537 at `ce12cdb`) names no `#662` item any more (662a's Decision 14 (iii)).

```
docs/adr/ADR-0021.md:149-155
Giving both forms of `command` or of `check` is a usage error (exit 2). **`profile create --from-current`** copies, from the
`Pane` records in `PaneStore` (so it needs no Herdr call), each member's `herdr.workspace` and `herdr.grid`, `host.cwd`,
`harness.kind`, `model`, `role`, `env`, `context`, `command`, `probe.check` and `probe.expect`, and sets each member's
`Pane.profile`; a pane already in another profile is `pane-in-other-profile`. The mapping lives in
`holler-pane/src/profile_snapshot.rs` (#662), which the migration (#650) reuses. `--from-current` writes `harness.port_policy`
as `fixed:<port>`, the port the pane's harness uses (#662, `profile_snapshot::fixed_port_policy`); `profile_diff` compares it
with `fixed:<port>` of the live port. A profile's **live panes** are the pane records whose `profile` names it (by slug).
```
```
docs/adr/ADR-0021.md:282-284
- No store transaction spans the two registries. The membership rule is enforced on `pane/cas_put`: setting `Pane.profile`
  to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
  names a pane of another profile (a detached spec) is not refused.
```
```
docs/adr/ADR-0021.md:332-333
**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
```
```
docs/adr/ADR-0021.md:346-349
| `profile create` | `profile-exists`, `profile-not-found` (for `--from`), `pane-in-other-profile`, `generation-conflict` |
| `profile delete` | `profile-not-found`, `profile-has-live-panes`, `generation-conflict` |
| `profile list` | none |
| `profile show` | `profile-not-found` |
```

### E14. Workspace lints and the CHANGELOG (662a's entry is the last of `[Unreleased]` / `### Enhancements`)

`clippy.toml`: `cognitive-complexity-threshold = 15`, `too-many-lines-threshold = 100`. `scripts/lint.sh`: every `#[allow]`
carries a trailing `// #NNN`; a file fails at 900 lines. `bash scripts/changelog-check.sh` prints `changelog-check: ok` at
`ce12cdb`. `rustfmt --check --edition 2021` on `crates/holler-cli/src/profile/{create,delete}.rs`,
`crates/holler-cli/tests/profile_verbs/{create,delete,rig}.rs` and `crates/holler-cli/tests/pane_verbs/process/stub.rs` exits 0
at `ce12cdb`.

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
```
CHANGELOG.md:8-10
## [Unreleased]

### Enhancements
```
```
CHANGELOG.md:182-183
- Pane control, `holler profile list` and `holler profile show` (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `profile list` prints every profile, sorted by slug, with its number of specs, its number of live panes and its
```
```
CHANGELOG.md:192-194
  [ADR 0021](docs/adr/ADR-0021.md) records both choices. Until the hub's stores are wired into the binary
  ([#649](https://github.com/Performant-Labs/holler/issues/649)), the real `holler profile list` and `show` still
  answer `not-implemented` ([#662](https://github.com/Performant-Labs/holler/issues/662)).
```

### E15. #663 (branch `issue-663-implementation` at `93fb653`, not merged): its blast radius, its ADR rule, its quoting

```
docs/handoffs/663-brief.md:1657-1659
12. **Blast radius.** `git diff --name-only origin/main...HEAD` lists only `crates/holler-cli/src/pane/profile_scope.rs`,
    `crates/holler-pane/src/probe.rs`, `CHANGELOG.md`, `docs/adr/ADR-0021.md` and the pipeline's own `docs/handoffs/663*`
    files.
```
```
docs/handoffs/663-brief.md:1687-1689
    Checks: `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` has its hunks only at the places a-e name;
    `git diff origin/main...HEAD -- docs/adr/ADR-0021.md | grep -E '^[-+]\|'` prints nothing (no table row changed) and
    `... | grep -E '^[-+]#'` prints nothing (no heading changed); each of the five edits is cited `(#663)`; the added text
```
```
crates/holler-cli/src/pane/profile_scope.rs:36-47
/// The reconcile step of a run without `--profile`: the bare pane doctor command line, which (like
/// [`reconcile_step`]) names no pane. The spec-editing verbs (#644, #646) print this one.
pub const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor";

/// The reconcile step for `profile`, one line for an operator to paste into a shell, with the
/// name POSIX-single-quoted (it may hold spaces, quotes or `$(...)`, but no control character):
/// `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`.
/// The scope's errors carry it; a spec-editing verb prints it for a pane-record conflict.
pub fn reconcile_step(profile: &ProfileName) -> String {
    let name = single_quoted(profile.as_str());
    format!("{RECONCILE_STEP_UNSCOPED} --profile {name} and then holler profile show {name}")
}
```
```
crates/holler-cli/src/pane/profile_scope.rs:259-263
/// `text` POSIX-single-quoted, read back by a shell as one word: each `'` becomes the four
/// characters `'\''`, every other character stays, and the whole is wrapped in `'...'`.
fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}
```

## Decisions carried over from the 662a brief (verbatim; they bind this run)

Quoted from `git show 31062ce:docs/handoffs/662-brief.md` (the final, A-passed version). "E4", "E7", "E8", "E9", "E11", "AC 3",
"AC 6f", "AC 7c" inside these quotes refer to **that** brief's numbering; this brief's own Evidence above re-quotes every fact
they rely on (its E4 = this E4/E5; its E6 = this E6; its E7 = this E7; its E8 = this E9; its E9 = this E10/E11; its E11 = this E13). Where a
later decision of this brief (B1-B8) refines a carried text, the refinement is named there and wins.

```
31062ce:docs/handoffs/662-brief.md:1577-1597
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
```
```
31062ce:docs/handoffs/662-brief.md:1603-1604
The names are `String` at clap time and go through `ProfileName::parse` in `run`, so a bad name is an envelope coded `usage`
(exit 2) in both formats, from the verb, not clap.
```
```
31062ce:docs/handoffs/662-brief.md:1608-1609
JSON `data` (always a `#[derive(Serialize)]` struct, never a hand-built `serde_json::Value` map, so key order is the struct's
in every build, E7):
```
```
31062ce:docs/handoffs/662-brief.md:1615-1616
| `create` | `{"profile": <Profile as stored>, "members": [pane names that joined]}` (`[]` for plain and `--from`) |
| `delete` | `{"name","slug","detached": [pane names whose profile was cleared]}` (`[]` without `--keep-panes`) |
```
```
31062ce:docs/handoffs/662-brief.md:1635-1639
create:  created profile Some Profile (some-profile): 2 specs, generation 1
         members: demo-c1r1, demo-c2r1                                       (--from-current only; "members: none" if none)
         copied from Alpha; no pane joined                                   (--from only)
delete:  deleted profile Some Profile (some-profile)
         detached, still running: demo-c1r1, demo-c2r1                       (--keep-panes with members only)
```
```
31062ce:docs/handoffs/662-brief.md:1647-1649
Every verb takes its ports from `ctx.ports` and prints once through `emit` (success) or `emit_error` (failure); a store error
passes through with its own code (`unavailable`, `timeout`, `store-corrupt`, `generation-conflict`) unless a step below maps it.
No verb calls `herdr`, `host`, `harness`, `scope` or `prober`, and none reads the process environment.
```
```
31062ce:docs/handoffs/662-brief.md:1658-1689
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
```
```
31062ce:docs/handoffs/662-brief.md:1693-1694
- **C1 Rigor.** The issue's Pipeline line says `rigor: in-session`; the operator's instruction (through the MO) is
  `second-opinion`. The header follows the operator.
```
```
31062ce:docs/handoffs/662-brief.md:1695-1701
- **C2 `profile-conflict` is not in ADR-0021's table for these verbs.** The issue lists `profile-conflict` among the codes of
  the four verbs; ADR-0021 section 9 (E11) gives `profile create` "`profile-exists`, `profile-not-found` (for `--from`),
  `pane-in-other-profile`, `generation-conflict`" and `profile delete` "`profile-not-found`, `profile-has-live-panes`,
  `generation-conflict`", with no `profile-conflict`. Decided: both are used (Decision 7: `profile-conflict` only where a
  write already landed and the next one lost), and ADR-0021's two rows gain `profile-conflict` (Decision 14 (ii)). Also, the
  issue calls these five codes "refusals", but `class_of` (E6) makes `profile-conflict` and `generation-conflict` failures
  (exit 1); the code wins and the tests assert exit 1.
```
```
31062ce:docs/handoffs/662-brief.md:1702-1705
- **C3 Herdr.** The issue says the verbs code against `HerdrPort`, and that the snapshot "takes positions from Herdr's snapshot
  through the adapter"; its own corrections (2026-10-08) say `--from-current` "needs no Herdr call" and takes the position
  "from the Pane record's `herdr.grid`". ADR-0021 section 3 agrees with the corrections. Decided: no verb calls `HerdrPort`
  (tested, AC 3).
```
```
31062ce:docs/handoffs/662-brief.md:1716-1718
- **C7 I1 versus membership writes.** Invariant I1 says only `holler pane` verbs change a pane or its registration;
  `create --from-current` and `delete --keep-panes` write `Pane.profile`. Both the issue and ADR-0021 section 3 mandate it, so it
  is not changed here; the writes touch only `profile` (AC 6f and AC 7c assert every other field is unchanged).
```
```
31062ce:docs/handoffs/662-brief.md:1723-1726
2. **`--from-current` snapshots every pane in `PaneStore`**, in `list()` order (by name), with no Herdr call (C3). Zero panes
   gives a profile with no specs and `members: []`. A pane already in another profile refuses the whole create before any write
   (the issue's `pane-in-other-profile`), which also matches what the stores would refuse inside their compare-and-swap (E8,
   E9 `check_membership`). A pane whose `profile` already has NAME's slug joins like any other.
```
```
31062ce:docs/handoffs/662-brief.md:1735-1739
4. **A profile's live panes are its members**: the pane records whose `profile` has the profile's slug (`is_member`). A closed
   pane's record is removed (epic ruling 9), so a record is the registry's statement that the pane exists; reading Herdr or
   the harness would make `list`, `show` and `delete` depend on adapters this story must not call. `list`'s live count, `show`'s
   comparison and `delete`'s `profile-has-live-panes` all use this one definition. A detached spec naming a pane of another
   profile shows as `missing` (membership is `Pane.profile` only).
```
```
31062ce:docs/handoffs/662-brief.md:1746-1749
7. **Codes.** `profile-exists` (exit 3) for a taken name or slug, including the create race; `profile-not-found` (3);
   `profile-has-live-panes` (3); `pane-in-other-profile` (3); `profile-conflict` (1) only when an earlier write of the same
   verb landed and a later one lost (create's failed undo; delete's conflict after a detach); `generation-conflict` (1)
   otherwise; `usage` (2) for a bad name. No new code, no `Refused`, no edit to `error.rs`.
```
```
31062ce:docs/handoffs/662-brief.md:1750-1752
8. **Undo policy.** `create --from-current` undoes everything on a failed join (I3: a failure records nothing), and reports
   `profile-conflict` with the reconcile commands if the undo itself fails. `delete --keep-panes` does not re-attach after a
   failed detach: detaching is what the operator asked for, a re-run converges, and an undo could itself fail.
```
```
31062ce:docs/handoffs/662-brief.md:1753-1755
9. **Actor** of every profile write: the literal verb, `holler profile create` / `holler profile delete` (a `ProfileLogEntry`'s
   actor may be "a verb such as `holler profile apply`", E4). Built with `Actor::parse` in `run`; its `Err` is mapped through
   `emit_error`, never unwrapped.
```
```
31062ce:docs/handoffs/662-brief.md:1756-1757
10. **Output.** Shapes as in "What each verb prints". JSON comes only from derived `Serialize` structs and `FieldValue` (typed),
    so a grid is `{"row":2,"col":1,"pos":"r2c1"}` in every build (E7). `list` sorts by slug itself (the port pins no order).
```
```
31062ce:docs/handoffs/662-brief.md:1767-1773
12. **Test rig, without editing `main.rs`.** `tests/profile_verbs/main.rs` is #670's; the shared rig (the fakes, a `ports()`,
    `run(argv, format)`, `assert_no_adapter_call()`) is its own file, `tests/profile_verbs/rig.rs`, declared from `list.rs` as
    `#[path = "rig.rs"] pub(crate) mod rig;` (662a; a `#[path]` in the non-`mod.rs` file `list.rs` resolves against its
    directory; the way #644 includes `launch_rig.rs`). `show.rs`, `create.rs`, `delete.rs` `use crate::list::rig`; #664/#665
    extend `rig.rs`, not `list.rs`. Each format runs on its own freshly seeded rig. Adapters are `FakeHerdr::new("scratch")`,
    `FakeHost::new()`, `FakeHarness::new()`, `FakeProber::new()` (asserted uncalled); `scope` is the harness's `Unwired`. One base
    rig should be agreed across #643 (`pane_verbs/list.rs`), #644 (`launch_rig.rs`) and this one (Follow-ups).
```
```
31062ce:docs/handoffs/662-brief.md:1774-1777
13. **A failing-Nth-write seam for the undo tests (662b).** The fakes fail only the *next* call of a method (E9 `fail_next`), so a
    join that fails on the second pane needs a seam: a test-local `PaneStore` in `create.rs` that delegates every method to a
    `FakePaneStore` and fails its N-th `cas_put` (and optionally the M-th) with a given `PaneError`. It holds no state of its own
    beyond the counter.
```
```
31062ce:docs/handoffs/662-brief.md:1778-1784
14. **ADR-0021 doc edit (F; three places, no other ADR text changes):** (i) section 3, lines 153-154: replace "**Deferred to
    #662:** what `--from-current` writes for `harness.port_policy`, which the `Pane` record does not hold (it records the port
    in use, not the policy)." with "`--from-current` writes `harness.port_policy` as `fixed:<port>`, the port the pane's harness
    uses (#662, `profile_snapshot::fixed_port_policy`); `profile_diff` compares it with `fixed:<port>` of the live port. A profile's **live
    panes** are the pane records whose `profile` names it (by slug)." (ii) section 9 table: add `profile-conflict` as the last code of the
    `profile create` row and of the `profile delete` row. (iii) "Deferred to named stories": delete the two bullets that name #662.
    662a makes (i) and (iii), 662b makes (ii).
```

Carried items that do **not** bind this run: 662a's Decisions 1, 3, 5, 6, 11 (split, port policy, the comparison, `show`'s
probe, where the pure tests live) are done and merged; Decision 14 (i) and (iii) are on `main` (E13: section 3's
`fixed:<port>` sentence; no `#662` bullet left under "Deferred to named stories"). Only Decision 14 **(ii)** is this run's.

## Decisions made for 662b (O, 2026-10-09; each refines or completes a carried text)

- **B1. How names print in messages.** In an error's prose, a profile name is `{:?}`-quoted, the form `show` already uses for
  `profile-not-found` (E8, `show.rs:83`): `"Some Profile"`. In a **command line a message suggests**, the name is
  POSIX-single-quoted so it pastes into a shell as one word even with spaces, quotes or `$(...)`: `holler profile delete 'Some
  Profile' --keep-panes`. This is #663's convention for its reconcile step (E15). One helper, in `delete.rs`:
  `pub(crate) fn shell_word(text: &str) -> String`, body `format!("'{}'", text.replace('\'', r"'\''"))`; `create.rs` calls it
  as `super::delete::shell_word`. Pane names print as they are (E8, `findings.rs:14`: no space, no shell metacharacter). This
  refines the carried messages, which show `<name>` unquoted.
- **B2. The undo also covers the pane whose join failed.** A join that answered `timeout` or `unavailable` may have landed (a
  store cannot say). So on a failed join of pane k the undo is, in this order: (1) `pane_store.get(k)`; if the record exists
  and `is_member(&record, &NAME)`, `cas_put(&Pane { profile: None, ..record }, record.generation)`; a missing or non-member
  record needs no write; (2) each pane that joined, in reverse join order, `cas_put(&Pane { profile: None, ..returned },
  returned.generation)`; (3) `profile_store.delete(&NAME, stored.generation, &actor)`. The undo **stops at its first failed
  step** and reports the carried `profile-conflict` message, so the profile still exists whenever a member may remain, and
  the message's remedy (`holler profile delete '<name>' --keep-panes`) is always runnable. This widens the carried "for each
  pane that already joined" by step (1); it costs one `get` and changes nothing when the failed join did not land.
- **B3. The order of `create`'s checks**, all before the first write: parse NAME (`usage`); with `--from`, parse PROFILE
  (`usage`); build the actor (`Actor::parse("holler profile create")`, its `Err` through `emit_error`); `profile_store.get(NAME)`
  (`Some` -> `profile-exists`); with `--from`, `profile_store.get(PROFILE)` (`None` -> `profile-not-found`); with
  `--from-current`, `pane_store.list()` and the plan check. So `create X --from X` is `profile-exists` when X exists and
  `profile-not-found` when it does not. `delete`'s order: parse NAME, actor, `get(NAME)`, `pane_store.list()`, then writes.
- **B4. The exact messages.** `<name>` is the stored profile's display name when there is a stored record, else NAME as parsed;
  `{name:?}` means `format!("{:?}", name.as_str())`; `<q>` is `shell_word(name.as_str())`; `<E>`, `<E2>` are `PaneError`
  `Display` texts; `<k>` is the name of the pane whose join failed; `<list>` is pane names joined by `", "`, or `none`
  when empty.

  | Case | Error (code, exit) | `what` / message |
  |---|---|---|
  | NAME stored (by `get`) | `ProfileExists` (3) | `{name:?} (slug <slug>)` |
  | create race: `cas_put` answered `Conflict` | `ProfileExists` (3) | `{NAME:?} (slug <slug>)` |
  | create race: `cas_put` answered `ProfileExists` | passes through (3) | the store's own |
  | `--from` source missing; `delete` of a missing profile | `ProfileNotFound` (3) | `{name:?}` |
  | `--from-current` plan: panes in other profiles | `PaneInOtherProfile` (3) | per pane `<pane> is in profile {other:?}` (`other` = that pane's own profile name), joined by `"; "`, in `list()` order |
  | join of pane k failed with `E`, undo complete | `E`'s code | message `<E>; nothing was kept (the profile and the memberships made so far were undone)` |
  | undo step failed with `E2` | `ProfileConflict` (1) | `{name:?}: pane <k> could not join (<E>) and the undo failed (<E2>); reconcile with: holler profile show <q> and holler profile delete <q> --keep-panes` |
  | `delete` with members, no `--keep-panes` | `ProfileHasLivePanes` (3) | `{name:?} has <count(n, "live pane")> (<list>); run holler profile delete <q> --keep-panes to detach them` |
  | detach of a member failed with `E` | `E`'s code | message `<E>; detached so far: <list>; the profile was not deleted; run the delete again` |
  | `delete` answered `Conflict` after >= 1 detach | `ProfileConflict` (1) | `{name:?} changed after its panes were detached (<list>); run holler profile show <q>, then the delete again` |
  | `delete` answered `Conflict`, nothing detached | passes through (1) | the store's own |

  A message that keeps `E`'s code is built as `ErrorBody { code: ErrorCode::from(&e), message }` (E7); the others through
  `ErrorBody::from(&PaneError::...)`. Every message is one line (no `\n` is written by the verb).
- **B5. Data comes from the stored records.** `create`'s `data.profile` is the record `cas_put` returned (generation 1, the
  store's slug and stamps); `members` the joined pane names in join order (`pane_store.list()` order, by name). `delete`'s
  `name` and `slug` are the record `get` returned; `detached` is in member order. Text: the first line uses the stored name and
  slug; `copied from <source name>` uses the stored source's name; counts use 662a's `count` (`1 spec`, `2 specs`).
- **B6. `delete --keep-panes` of a profile with no members** is a plain delete: `detached: []` and no second text line.
- **B7. One create write, reusable.** `create.rs` exposes `pub(crate) fn insert_profile(store: &dyn ProfileStore, profile:
  &Profile, actor: &Actor) -> Result<Profile, PaneError>`: `store.cas_put(profile, 0, actor)` with `PaneError::Conflict` mapped
  to the `ProfileExists` row of B4 and every other error passed through. All three forms of `create` write through it, so the
  race mapping exists once; #650 (import's `fleet`, its `profile-exists`) and #665 (`profile import`) call it rather than
  re-deriving it.
- **B8. Why `create` and `delete` do not use `ProfileScope::resolve`** (662a A's warn 3): the issue limits #662 to
  `ProfileStore`, `PaneStore` and `HerdrPort`; `ProfileScope` is the helper of the `--profile` verbs, and the profile verbs take
  NAME as a positional; and the rig asserts `scope` is never called. Membership is `is_member` in every path, the one copy
  `StoreScope::resolve` (#663) is also asked to reuse, so the two paths cannot diverge.

## Contradictions found (this run)

C1 (rigor), C2 (`profile-conflict` missing from ADR-0021's rows; closed by Decision 14 (ii)), C3 (no Herdr call) and C7 (I1
versus the membership writes) are carried verbatim above and still hold. New:
- **C8 Undo scope.** The carried create behaviour undoes "each pane that already joined"; a join that timed out may have
  joined. B2 adds the re-read of that pane. Tested by AC 2m.
- **C9 Quoting.** The carried messages print `<name>` bare, which is not pasteable for `Some Profile`. B1 quotes. Tested by
  AC 2i, 3b, 3f.
- **C10 Blast radius.** The issue lists only the source files (E1). Epic ruling 2 (E2) gives each verb its ADR 0003 row, its
  `cli-surface.txt` lines and its `tests/profile_verbs/<verb>.rs`, and `stub.rs`'s own doc (E12) tells a verb story to delete its
  STUBS entries; the bare fixture lines and ADR rows stop parsing once NAME is required (E12). ADR-0021's rows are Decision
  14 (ii); `CHANGELOG.md` is repo practice. "Files" lists all of them.
- **C11 The seam.** Carried Decision 13 has the seam fail the N-th `cas_put`; AC 2m (B2) also needs it to **apply** the N-th
  write and then answer an error. Its plan is fixed when it is built; its only mutable state is still the counter.

## Behaviour (what F implements)

The carried "create (662b)" and "delete (662b)" paragraphs, with B1-B7 applied. In short:

**create.** Checks (B3). Then:
- plain: `insert_profile(profile_from_panes(&NAME, &[]))` (no specs; slug, generation 0 and stamps from 662a's builder, so no
  `Profile` is built by hand).
- `--from P`: `insert_profile(Profile { panes: source.panes.clone(), ..profile_from_panes(&NAME, &[]) })`, the specs verbatim;
  no pane is read or written.
- `--from-current`: plan (every pane whose `profile` is `Some(q)` with `q.slug() != NAME.slug()` refuses the whole create);
  `insert_profile(profile_from_panes(&NAME, &panes))`; join each pane in `list()` order with
  `cas_put(&Pane { profile: Some(NAME), ..pane }, pane.generation)`; on a failed join, the undo of B2.
- Print once: `emit(.., Ok(Created { profile, members }), render)`.

**delete.** Checks (B3). `members` = `pane_store.list()` filtered by `is_member(_, &profile.name)`. Members and no
`--keep-panes` -> `profile-has-live-panes`, nothing written. With `--keep-panes`, detach each member (`cas_put(&Pane { profile:
None, ..member }, member.generation)`), stopping at the first failure (no re-attach). Then `profile_store.delete(&profile.name,
profile.generation, &actor)` with actor `holler profile delete`; its `Conflict` maps per B4. Print once.

Neither verb calls `herdr`, `host`, `harness`, `scope` or `prober`, or reads the environment. JSON `data` is a derived
`Serialize` struct (`Created { profile: Profile, members: Vec<PaneName> }`, `Deleted { name: ProfileName, slug: String,
detached: Vec<PaneName> }`; text-only fields `#[serde(skip)]`); `PaneName` serializes as a string. Each function stays under
100 lines and cognitive complexity 15 (split the undo into its own function).

## Acceptance criteria (all (b); tests first; each observable)

All commands run from the worktree. "Both formats" = the rig's `run_both` (E11): the same argv in `Format::Text` and
`Format::Json` on two freshly seeded rigs, equal exit codes, the JSON passing `check_envelope`, and no adapter or probe call.
A test that sets a fault does so inside the seed closure. Test names are the ones T writes; T may add cases but not drop these.

1. **No adapter, no probe, no environment.** Every test ends with `assert_no_adapter_call()` (directly or via `run_both`), and
   `grep -nE 'ports\.(herdr|host|harness|prober|scope)|std::env|env::var' crates/holler-cli/src/profile/create.rs crates/holler-cli/src/profile/delete.rs`
   prints nothing.
2. **create** (`crates/holler-cli/tests/profile_verbs/create.rs`):
   - a. `create_makes_an_empty_profile_in_both_formats`: exit 0; the store holds NAME at generation 1 with no specs; JSON
     `data.members == []` and `data.profile.generation == 1`; text first line `created profile Demo (demo): 0 specs, generation 1`;
     `rig.panes.faults().calls()` has no `PaneStoreOp::CasPut`.
   - b. `create_refuses_a_taken_name_or_slug`: `Some Profile` seeded; `create "Some Profile"` and `create some-profile` both exit
     3 with `profile-exists` in both formats; the message contains `"Some Profile" (slug some-profile)`; the store still holds it
     at generation 1.
   - c. `create_reports_a_create_race_as_profile_exists`: `profiles.faults().fail_next(ProfileStoreOp::CasPut, PaneError::Conflict)`
     -> exit 3, `profile-exists`, both formats.
   - d. `create_from_makes_a_detached_copy`: `Alpha` has specs for `demo-c1r1` (a member of `Alpha`) and `demo-c9r9` (no record);
     `create Beta --from Alpha` gives `Beta.panes == Alpha.panes`; no `PaneStoreOp::CasPut` in the pane call log; every pane
     record unchanged (`demo-c1r1` still in `Alpha`); text has `copied from Alpha; no pane joined`.
   - e. `create_from_a_missing_profile_is_profile_not_found`: exit 3, both formats; `get(Beta)` is `None`.
   - f. `create_from_current_snapshots_every_pane_and_joins_it`: three panes at r1c1, r1c2, r2c1 with distinct cwd, model and
     effort, role, env names, ceilings, command, check and expect; the stored profile's `panes` equal
     `profile_from_panes(&NAME, &seeded_records).panes`; each pane record afterwards equals its seeded record except
     `profile == Some(NAME)` and `generation` + 1; JSON `data.members` lists the three names in `list()` order (by name);
     text has one `members: ` line naming the three, comma-separated, in the same order.
   - g. `create_from_current_refuses_a_pane_in_another_profile_and_writes_nothing`: one pane in `Other`: exit 3,
     `pane-in-other-profile`, message names the pane and `"Other"`; `get(NAME)` is `None`; no `ProfileStoreOp::CasPut` and no
     `PaneStoreOp::CasPut` in the call logs.
   - h. `create_from_current_undoes_everything_when_a_join_fails`: the Decision 13 seam fails the 2nd pane `cas_put` with
     `PaneError::Conflict`: exit 1, `generation-conflict`, message contains `nothing was kept`; afterwards `get(NAME)` is `None`
     and every pane's `profile` is `None`.
   - i. `create_from_current_reports_profile_conflict_when_the_undo_fails`: NAME `Some Profile`; the seam fails the 2nd
     `cas_put` (the join of pane 2) and the 3rd (the undo of pane 1; pane 2's re-read shows no membership, so it needs no
     write): exit 1, `profile-conflict`, a one-line message containing `holler profile delete 'Some Profile' --keep-panes`;
     `get(NAME)` is still `Some` (the undo stopped before the delete).
   - j. `create_holds_env_names_never_values` (I7): after `--from-current` over panes with env `["ALPHA_TOKEN","BETA_URL"]`, no
     string in any `env` array of `data.profile.panes` or of the stored profile contains `=`; and decoding a `ProfileSpec` JSON
     with `"env":["TOKEN=s3cr3t"]` fails with `profile-secret-refused` whose text lacks `s3cr3t`.
   - k. `create_flags_conflict`: `verb_harness::parse::try_parse(&["profile","create","X","--from-current","--from","Y"])` is
     `Err` with `ErrorKind::ArgumentConflict`.
   - l. `create_with_a_bad_name_is_usage_in_both_formats`: `create "   "` and `create Demo --from "   "` exit 2,
     `error.code == "usage"`, nothing written.
   - m. `create_from_current_undoes_a_join_that_landed_but_timed_out` (B2): the seam **applies** the 2nd pane `cas_put` and then
     answers `PaneError::Timeout { op: "pane/cas_put".into() }`: exit 1, `timeout`, message contains `nothing was kept`;
     afterwards `get(NAME)` is `None` and both panes' `profile` is `None` (pane 2 was re-read and cleared).
3. **delete** (`crates/holler-cli/tests/profile_verbs/delete.rs`):
   - a. `delete_removes_a_profile_without_members_in_both_formats`: exit 0; `get` is `None`; JSON `data == {"name":"Alpha",
     "slug":"alpha","detached":[]}`; text `deleted profile Alpha (alpha)` only. With `--keep-panes`, the same (B6).
   - b. `delete_refuses_while_panes_are_live`: `Some Profile` with two members: exit 3, `profile-has-live-panes`, message
     contains both pane names, `2 live panes` and `holler profile delete 'Some Profile' --keep-panes`; no `CasPut` in the pane
     call log and no `Delete` in the profile call log.
   - c. `delete_keep_panes_detaches_then_deletes`: exit 0; profile gone; both pane records still stored with `profile == None`,
     every other field equal to before, generation + 1; `data.detached` lists both; text has `detached, still running: <a>, <b>`.
   - d. `delete_of_a_missing_profile_is_profile_not_found`: exit 3, both formats.
   - e. `delete_conflict_without_members_is_generation_conflict`: `fail_next(ProfileStoreOp::Delete, PaneError::Conflict)` ->
     exit 1, `generation-conflict`.
   - f. `delete_conflict_after_a_detach_is_profile_conflict`: `Some Profile` with one member, `--keep-panes`, the same fault ->
     exit 1, `profile-conflict`, message contains `holler profile show 'Some Profile'`; the member's `profile` is `None`; the
     profile still exists.
   - g. `delete_stops_at_a_failed_detach`: two members, `fail_next(PaneStoreOp::CasPut, PaneError::Conflict)` -> exit 1,
     `generation-conflict`, message contains `detached so far: none`; the profile still exists; both panes still members; no
     `ProfileStoreOp::Delete` in the profile call log.
   - h. `delete_counts_members_by_slug`: a pane whose `profile` is `SOME-PROFILE` blocks `delete "Some Profile"` (exit 3).
4. **The stub cases are gone.** `grep -n 'assert_stub_routes' crates/holler-cli/tests/profile_verbs/create.rs crates/holler-cli/tests/profile_verbs/delete.rs`
   prints nothing; in `crates/holler-cli/tests/pane_verbs/process/stub.rs` the `// #662` line remains and
   `grep -c '662),' crates/holler-cli/tests/pane_verbs/process/stub.rs` prints `0`.
5. **Surface.** `docs/adr/ADR-0003.md` lines 65-66 read exactly `holler profile create NAME [--from-current | --from PROFILE]`
   and `holler profile delete NAME [--keep-panes]`, each padded with spaces so `#662` stays in column 67 as on lines 67-68.
   The `# #662` group of `cli-surface.txt` is exactly: `profile create | Demo`, `profile create | "Some Profile" --from-current`,
   `profile create | Copy --from Demo --format=json`, `profile delete | Demo`, `profile delete | Demo --keep-panes --json`,
   `profile list |`, `profile list | --format=json`, `profile show | Demo`, `profile show | "Some Profile" --json`. Then
   `cargo test -p holler-cli --test cli_surface_test`, `--test docs_cli_test` and `--test pane_cli_process` pass.
6. **ADR-0021 (Decision 14 (ii)).** `grep -n 'profile create. |.*generation-conflict., .profile-conflict. |$' docs/adr/ADR-0021.md`
   and `grep -n 'profile delete. |.*generation-conflict., .profile-conflict. |$' docs/adr/ADR-0021.md` each print one line (the
   section 9 rows, lines 346-347 at `ce12cdb`); `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` changes only those two
   rows, each gaining one more code, `profile-conflict`, after its last one, and nothing else.
7. **Crate tests.** `cargo test -p holler-cli --test profile_verbs` passes, listing every AC 2-3 name; `cargo test --workspace`
   passes.
8. **Lints.** `cargo clippy --workspace --all-targets -- -D warnings` and `bash scripts/lint.sh` pass (every `#[allow]` in a
   touched file carries `// #662`; every touched file under 900 lines; every function under 100 lines).
9. **No new `unsafe`, no new dependency.** `git diff origin/main...HEAD -- '*.rs' | grep -n '^+.*unsafe'` prints nothing;
   `git diff origin/main...HEAD -- '*Cargo.toml' Cargo.lock` prints nothing.
10. **Formatting.** `rustfmt --check --edition 2021` on every `.rs` file this run touched exits 0 (epic ruling 4).
11. **CHANGELOG.** `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry after 662a's (E14) for `holler profile
    create NAME [--from-current | --from PROFILE]` and `holler profile delete NAME [--keep-panes]`, saying the real binary still
    answers `not-implemented` until #649 wires the stores, linking `[#662](https://github.com/Performant-Labs/holler/issues/662)`;
    it names no host or account; `bash scripts/changelog-check.sh` prints `changelog-check: ok`.
12. **Blast radius.** `git diff --name-only origin/main...HEAD` lists only the files under "Files" plus the pipeline's own
    `docs/handoffs/662*` files.

## Files

Production (F): `crates/holler-cli/src/profile/create.rs`, `crates/holler-cli/src/profile/delete.rs` (replace the stubs);
`docs/adr/ADR-0003.md` (lines 65-66), `docs/adr/ADR-0021.md` (lines 346-347), `CHANGELOG.md` (one entry).

Tests (T): `crates/holler-cli/tests/profile_verbs/create.rs` (AC 2, with the Decision 13 seam, a test-local `PaneStore` that
delegates to a `FakePaneStore` and, by a plan keyed on the N-th `cas_put`, either fails it with a given `PaneError` or applies
it and then answers one; built into `Ports { pane_store: &seam, ..rig.ports() }`), `crates/holler-cli/tests/profile_verbs/delete.rs`
(AC 3), `crates/holler-cli/tests/pane_verbs/process/stub.rs` (delete the two `("profile", ..., 662)` lines, keep `// #662`),
`crates/holler-cli/tests/fixtures/cli-surface.txt` (the `# #662` group). T may add helpers to `profile_verbs/rig.rs` (#662's
file) without changing an existing helper's behaviour.

Not touched: `profile/mod.rs`, `profile/list.rs`, `profile/show.rs`, `cli.rs`, `lib.rs`, `output.rs`, `pane/wiring.rs`,
`pane/profile_scope.rs`, every `holler-pane` source file, every manifest, the test kit, `tests/profile_verbs/main.rs`,
`tests/verb_harness/**`, `docs_rows.rs`.

**Blast radius:** the issue's `create.rs` and `delete.rs`, plus the verb-owned surface (ADR 0003 rows, the fixture group,
`profile_verbs/{create,delete}.rs`, the STUBS entries), ADR-0021's two rows and `CHANGELOG.md`.

## Reuse map (extend, do not duplicate)

| Object | Use | Extend or new |
|---|---|---|
| `output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx}` (E7) | every result and error | reuse |
| `PaneError` closed variants, `class_of` via `emit` (E6) | every code and exit | reuse; no `Refused`, no new code, no `error.rs` edit |
| `ProfileName::parse`, `slug()`, `Actor::parse` (E4) | names, slugs, actors | reuse |
| `profile_snapshot::profile_from_panes` (E8) | every new record (`&[]` for plain and `--from`), `--from-current`'s specs | reuse; no second mapping, no hand-built `Profile` |
| `profile_diff::is_member` (E8) | the plan check (by slug), `delete`'s members, B2's re-read | reuse; no inline slug filter |
| `profile::list::count` (E8) | `2 specs`, `2 live panes` | reuse (`super::list::count`); no fifth pluralizer |
| `insert_profile` (B7) | the one create write and its race mapping | new here, `pub(crate)` |
| `shell_word` (B1) | name in a suggested command | new here, `pub(crate)`; see Follow-ups (#663's private copy) |
| rig (`Rig`, `run_both`, `assert_failure`, `pane`, `member`, `matching_spec`, `profile`) (E11) | every test | reuse |
| `FakePaneStore`/`FakeProfileStore` fault switch (E10), `verb_harness::parse::try_parse` (E11) | faults, AC 2k | reuse; the seam is the only new test double |

## Forward-compat (consumers of this run)

| Consumer | Needs from 662b | Satisfied by |
|---|---|---|
| #644 launch/relaunch | a P for `--profile P` to name (`profile-not-found` otherwise); `--from-current` specs whose `port_policy` is `fixed:<port>`, which its `port_of_policy` parses | `create`; `profile_from_panes` (662a), unchanged |
| #646 park/unpark/close | `close` removes a pane's record (epic ruling 9), so a closed pane stops being a member and stops blocking `delete`; `close --profile` edits specs, which `create --from` copies verbatim | membership is the record (Decision 4); `delete` reads members at run time |
| #664 apply | profiles to apply, including detached copies from `--from` (specs naming panes of other profiles, which apply refuses without `--take-over`); `rig.rs` to extend | `create --from` keeps specs verbatim (AC 2d); `rig.rs` unchanged in behaviour |
| #665 rename/export/import (PROPOSED) | `profile import` creates a profile and answers `profile-exists` | `insert_profile` (B7) |
| #650 migration | creates `fleet` (`profile-exists` if taken) and sets each pane's profile | `insert_profile`; the join order and undo of B2 are the pattern to follow |
| #649 integration scenario | `create --from-current`, `delete` refused while live, then `--keep-panes`; every JSON output parses | AC 2f, 3b, 3c; shapes and codes pinned here |
| #663 `StoreScope` | nothing; it may later reuse `shell_word` (Follow-ups) | no change needed |

## Out of scope

- `profile apply`, `rename`, `export`, `import` (#664, #665); any edit to `list` or `show`.
- Wiring the real stores into the binary (`wiring.rs`, #649): until then the real `holler profile create`/`delete` answer
  `not-implemented` from `Unwired`, which `pane_cli_process` no longer asserts for them (AC 4).
- Any Herdr, tmux or OpenCode call; any live fleet or real session.
- A pane filter for `--from-current`; redacting argv elements; a cross-registry transaction.

## Follow-ups (not this story; the orchestrator files them if wanted)

- **One `shell_word`.** #663's private `single_quoted` (E15) and this run's `delete::shell_word` have the same body. Whichever
  of #663 and 662b merges second (or a later story) moves one copy to a shared home (for example `output.rs`'s owner, or
  `holler-pane` beside `ProfileName`) and deletes the other.
- **A pane naming a deleted profile.** A pane that joins NAME between `delete`'s member listing and its delete leaves a pane
  naming a profile that no longer exists (no transaction spans the two registries, ADR-0021 section 8, E13). Reconcile (#647)
  or doctor should report it (carried from 662a).
- **One base rig** (carried from 662a): `profile_verbs/rig.rs`, #643's and #644's rigs and #647's `pane_verbs/doctor/rig.rs`.

## Test plan

RED (a compile error is not RED; the tester overlay): T first lands **signature stubs** in `create.rs` and `delete.rs`: the
carried `ProfileCreate`/`ProfileDelete` structs, `insert_profile` returning `Err(PaneError::NotImplemented)`, `shell_word`
returning `String::new()`, and `run` still `emit_error(.., not_implemented(STORY))`. Then every AC 2-3 test fails on its
assertion (exit 1 `not-implemented` where it expects 0, 2 or 3 and data), except AC 2k (a clap property, which passes once the
structs exist) and the decode half of 2j (a property of `EnvVarName`, already true); T journals which pass for want of
behaviour. The AC 5 fixture lines parse once the structs land; they are a surface check. T journals the RED run.

GREEN: F replaces the stubs; then `cargo test -p holler-cli --test profile_verbs`, `--test cli_surface_test`,
`--test docs_cli_test`, `--test pane_cli_process`, then the AC 6-12 gates.

Every test is in-process through `run_verb_with` over the test kit's fakes; no subprocess beyond the existing
`pane_cli_process` target, no temporary directory, no real session name.

## Risks

- **Partial writes across two registries.** `create --from-current` writes the profile then N panes; `delete --keep-panes`
  writes N panes then the profile. No transaction spans them. B2's undo and the `profile-conflict` remedies bound the damage;
  AC 2h, 2i, 2m, 3f, 3g pin each branch. A crash between writes leaves a state `show` reports (a member with no spec is
  `extra`; a profile with specs and no members shows `missing`).
- **The create race on the real store** is `generation-conflict`, not `profile-exists` (E9); B7 maps it once; AC 2c pins it.
- **Merge order.** #644 (in flight) edits only the `pane launch`, `pane relaunch` row of ADR-0021 section 9 (line 339 at
  `ce12cdb`), not the two profile rows, so the rows merge cleanly; `CHANGELOG.md`
  conflicts with whichever of #644, #646, #663 merges first; the run's own agent keeps both entries and re-runs
  `bash scripts/changelog-check.sh`.
- **Secrets.** Env entries are names by type (I7, AC 2j). Argv elements are copied as stored; Holler has no redaction rule for
  them (recorded in 662a, unchanged).
- **The real binary still answers `not-implemented`** for these verbs until #649 wires the stores; by design.

## Open questions for the operator

None blocking. B1 (quoting) and B2 (the wider undo) are O's calls under the standing "decide, don't ask" instruction; the
operator may reverse either before T starts without changing any other decision.
