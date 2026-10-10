# Brief: #663 profile-scope-probe (the `--profile` helper and the probe runner)

Rigor: second-opinion. UI surface: no. Kind: feature.

Repo: Performant-Labs/holler. Issue: #663 (epic #633, wave 3).

**Branch:** `issue-663-implementation` (worktree `.claude/worktrees/0663-profile-scope-probe`, from `origin/main` at `3bdd129`).
**Review rigor:** second-opinion, set by the orchestrator for this run (the issue's own `Pipeline` line says `in-session`;
see "Contradictions found", C1). An outside model reviews this brief and the diff, and it sees only this brief, so every
fact the design rests on is pasted below, verbatim, with its file and lines.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see the table after the Decisions.
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 1, 2, 5, 8 and 12, and "Decisions taken", items 1 and 2.
This brief **amends ADR-0021 in place in this change** (AC 14; the architecture review's B-1 widened the blast radius by
`docs/adr/ADR-0021.md`, as #683's and #688's reviews did). The one point the ADR leaves open (Decision 5), the two
narrowings this story makes (Decisions 11 and 15), the probe verdict (Decision 14) and the reconcile step (Decisions 6-8) are
recorded in the rustdoc of the two files, in the run's `docs/handoffs/663/decisions.md`, and in ADR-0021 sections 1, 2 and 8
(AC 14).
**Handoffs:** `docs/handoffs/663/handoff-<phase>.md`; the decision journal is `docs/handoffs/663/decisions.md`.
**Public repository:** no personal host, tailnet or account name goes into code, tests, docs, the CHANGELOG, commit
messages or the PR. Tests use the test kit's neutral names only (`demo-c1r1` ... `demo-c4r1`, `Demo Alpha`, `Demo Beta`,
`Demo Gamma`) and commands that exist on every Linux and macOS machine (`printf`, `sh`, `sleep`, `true`, `false`, `yes`,
`kill`, `ps`).

## Problem

Two stubs stand where the epic's `--profile` verbs and health probe need real code. `crates/holler-cli/src/pane/profile_scope.rs`
is an empty file: there is no real `ProfileScope`, so no `--profile` verb can scope itself to a profile or edit a spec in one
transaction with its live change (I3, I8). `crates/holler-pane/src/probe.rs`'s `run_probe` always answers `Error`, so no pane
can pass a health probe. This story fills both: `StoreScope`, the real `ProfileScope` over any `ProfileStore` and `PaneStore`,
which passes the test kit's 15-case conformance suite; and `run_probe`, which runs an argv directly (no shell), bounded by a
timeout, with a capped output, killing the whole process group it started when it gives up. Every verb story codes against
the trait and the fakes, so nothing waits for this one; #649 wires `StoreScope` and `SystemProber` in.

## Evidence (verbatim, as of `3bdd129`)

### A. The issue (#663), the parts this brief relies on

```
issue #663 body:3-24
## Scope
Implement the `ProfileScope` trait that #637 fixes in `crates/holler-pane/src/profile.rs`, in `crates/holler-cli/src/pane/profile_scope.rs`. Every `holler pane` verb story calls the trait and tests with #638's fake, so none of them waits for this story.
- **Scoping** (list, get, watch, doctor, switch, reset, park, unpark, say, interrupt, answer, the roster): `resolve(profile, pane)` returns the panes to act on. A named pane must belong to the profile (`pane-not-in-profile`); with no pane name it means every pane of the profile (for the read verbs and doctor/park/unpark); a missing profile is `profile-not-found`.
- **Spec-editing transaction** (launch, relaunch, close): `edit_spec(profile, pane, edit, act)` reads the profile and its generation (plan), runs the verb's live change (act), observes, then writes the edited spec by CAS on the profile's generation (record), as one transaction (I3, I8). If the live change fails, nothing is recorded in the profile: its specs are put back to what they were. (amended 2026-10-09, decisions) The profile is written first, so a failed act restores the specs by a second write; the specs are equal to before but the generation has moved by two and the log shows the edit and its reversal (ADR-0021 section 8). A conflict on the profile's generation after the act follows the rule ADR-0021 (#634) fixes and fails loudly with `profile-conflict`. (amended 2026-10-08, review) The rule is now fixed in the epic: profile CAS first, then the live act, then record; on a conflict after the act the verb fails loudly and prints the reconcile step (the exact `holler pane doctor` command). `--spec-only` makes `edit_spec` skip the act. launch/relaunch add or update the pane's spec from the verb's flags; close removes it. The profile must exist; otherwise the verb refuses with one line.
- Without `--profile`, `resolve` and `edit_spec` never read or write any profile.
- (amended 2026-10-08, features) The **probe runner**: implement `run_probe(argv, expect, timeout) -> ProbeResult` in `crates/holler-pane/src/probe.rs` (signature fixed by #637): run the argv directly (no shell), bounded by the timeout, and report `Ok`, `Failed { missing }` (the expected strings not found in its output) or `Error(reason)`. Verb stories use #638's fake probe, so none waits for this.

## Acceptance (test kit only)
- Passes #638's `ProfileScope` conformance cases (the same cases #638's fake passes): membership refusal, every-pane scope, a failed act leaves the profile's specs equal to before (the generation has moved by two, ADR-0021 section 8), a successful act bumps it once, a stale generation gives `profile-conflict`, no profile is touched without `--profile`.
- (amended 2026-10-08, features) `run_probe` against a scratch command: all expected strings present gives `Ok`; one missing gives `Failed` naming it; a hung command gives `Error` within the timeout; no shell is spawned (an argv with `;` is passed literally).

Depends on: #637, #638. (It codes against the `ProfileStore` trait and #638's fake; the real registry #661 is wired in by #649.)

## Blast radius
- crates/holler-cli/src/pane/profile_scope.rs (pre-created as a stub by #637)
- crates/holler-pane/src/probe.rs (pre-created as a stub by #637) (amended 2026-10-08, features)

## Scheduling
Wave 3, beside the verb stories.

## Pipeline
`rigor: in-session`.
```

### B. The epic (#633): the contract lines, the invariants and the rulings this story is bound by

```
epic #633 body, the contract:76-80
Argv            Vec<String>             // every stored command is an argv array; nothing in Holler passes it through a shell;
                                        // a bare shell string where an array is expected is refused: command-not-argv
ProbeResult     Ok | Failed { missing: Vec<String> } | Error(reason)
                run_probe(argv, expect, timeout) -> ProbeResult   // holler-pane/src/probe.rs, signature fixed by #637, built by #663
                a failing probe refuses launch/relaunch and apply's create/relaunch with probe-failed
```
```
epic #633 body, the contract:86-100
Port            ProfileStore get / list / cas_put(profile, expected_generation) / delete(name, expected_generation) / watch / log(name)
Helper trait    ProfileScope resolve(profile, pane) / edit_spec(profile, pane, edit, act)   // implemented in holler-cli/src/pane/profile_scope.rs
Profile verbs   holler profile  create NAME [--from-current | --from PROFILE] | delete NAME [--keep-panes] | list | show NAME
                                | apply NAME [--dry-run]                   (#664; REQUIRED, needed by #666; `show` reports spec-vs-live differences, so there is no `diff` verb)
                                  (amended 2026-10-08, features) apply refuses a pane that belongs to another profile unless --take-over (confirmed, C3)
                (PROPOSED, operator to confirm: rename | export | import; doctor code profile-drift; #665)
--profile P     spec-editing  launch, relaunch, close: edit P's spec for that pane AND change the live pane as ONE transaction
                              (I3; the profile write is a CAS on P's generation; if the live change fails nothing is recorded in P);
                              P must exist or the verb refuses in one line; without --profile they never touch a profile
                              (amended 2026-10-08, review) I8 write order: profile CAS first, then the live act, then record; on a conflict after the act the
                              verb fails loudly (profile-conflict) and prints the reconcile step (the exact `holler pane doctor` command)
                              (amended 2026-10-08, features) --spec-only (requires --profile): edit P's spec and change nothing live (confirmed, C2)
                scoping       list, get, watch, doctor, switch, reset, park, unpark, say, interrupt, answer, roster: act on P's panes;
                              a named pane must belong to P or the verb refuses; read verbs and doctor/park/unpark with no pane = every
                              pane of P; say/interrupt/answer/switch/reset still need a pane name and only check membership
```
```
epic #633 body, the invariants:122-129
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
epic #633 body, "Skeleton split" rulings:175-185
Rulings that came out of the reviews and apply to every later story (also in #637, #669 and #670):
1. **Verbs run CLI-side against the ports; the hub is the store only** (`pane/*`/`profile/*` methods are get/list/cas_put/watch and the profile equivalents). Adapters are constructed in `holler-cli/src/pane/wiring.rs`.
2. **One verb, one file, one owning story, including its clap `Args` struct** (positionals and verb-specific flags), its ADR 0003 row, its `cli-surface.txt` line and its own `tests/pane_verbs/<verb>.rs` or `tests/profile_verbs/<verb>.rs`. The frozen shared files declare only the shared flag groups (`SpecFlags`, `ProfileOpt`, `SpecOnly`) and `--take-over`.
3. **Codes are constants in each verb's own file** built through `PaneError::Refused` or the closed `PaneError` set in `holler-pane/src/error.rs`; no verb edits that enum (infrastructure variants `timeout`, `pane-not-found`, `session-not-found`, `store-corrupt`, `unavailable` and `profile-exists`, `profile-has-live-panes` are already in it).
4. **Formatting:** the tree is not rustfmt-clean and CI has no fmt step. Read the rules' "`cargo fmt`" as: new `.rs` files pass `rustfmt --check --edition 2021`; existing files are not reformatted.
5. **`say`/`interrupt`/`answer --pane`/`--profile`** are declared by #670 and refused with "not implemented (story #646)"; #646 fills them. The refusal is plain stderr text, exit 1, on every format.
6. **`pane/*` and `profile/*` are hub control-socket methods, not v2 wire methods**: they stay outside the closed 22-row catalog, so #634 documents them as control-socket methods rather than reserving them in `docs/protocol/v2.md`.
7. **Env entries have one guard**, `EnvVarName` in `holler-pane`: `NAME=value` is `profile-secret-refused` (I7), an empty or whitespace name is `env-name-invalid`. #661 and #665 do not pre-scan raw JSON.
8. **Profile writes carry an actor.** `ProfileStore`'s writing methods take an `Actor`; the log entry is `{at, generation, actor, change}`; `Profile` has no `log` field.
9. **A closed pane's record is removed.** `PaneStore::delete(name, expected_generation)` and the `pane/delete` method (so `PANE_METHODS` has five names) are added to the contract; `close` (#646) uses it, #639 implements it, #638's fakes provide it.

```
```
epic #633 body, "Rules for the agent":196-198
## Rules for the agent

Holler's pipeline, hooks and branch rules apply (see `CLAUDE.md`). Each story edits only its Blast radius. **No story touches a live fleet, a real Herdr session, a running pane or a real OpenCode session**; verification is the test kit and scratch instances, and tests refuse real session names. Spikes #635 and #636 run only in scratch sessions. The operator does the live check and the cutover (#654), and the final recreation from profiles (#666) (amended 2026-10-08, profiles).
```
```
epic #633 body, "Decisions 2026-10-09":218-222
## Decisions 2026-10-09 (ADR-0021, #634)

Taken by the operator while ADR-0021 was written; the ADR records them in its "Decisions taken" section.
1. The I8 order stays: profile first, then the live act, then record. A failed act restores the profile's specs by a second write, so the specs are equal but the generation has moved by two (#663 and #638 amended).
2. `pane-in-other-profile` is checked inside the pane registry's compare-and-swap, not in the hook (#661 amended).
```

### C. ADR-0021, the sections the design follows

The probe runner's contract, the port table and the crate rule (`holler-pane`'s one side effect is the probe runner, with
the standard library):
```
docs/adr/ADR-0021.md:78-80
**`ProbeResult`** (B1) is `"ok"`, `{"failed": {"missing": [...]}}` or `{"error": "reason"}`. The signature
`run_probe(argv, expect, timeout) -> ProbeResult` is frozen; until #663 fills it, the stub answers `Error`, never `Ok`, so a
launch cannot pass a probe on a stub. The `Prober` trait and `SystemProber` let a test swap the runner.
```
Section 2's bound, which AC 14b narrows in place:
```
docs/adr/ADR-0021.md:86-88
All ports are traits in `holler-pane` (`src/ports.rs`, `src/profile.rs`), **synchronous, blocking and `Send + Sync`**: async
code calls them from `spawn_blocking` or a thread, and every method returns within I5's bound (default 10 s) or with
`timeout`. `Ports` bundles one `&dyn` of each, and a verb holds it.
```
```
docs/adr/ADR-0021.md:90-98
| Port | Methods (merged) | Implemented by |
|---|---|---|
| `PaneStore` | `get`, `list`, `cas_put(pane, expected_generation)`, `delete(name, expected_generation)`, `watch(since)` | the hub (#639); `delete` was added by ruling 9 |
| `ProfileStore` | `get`, `list`, `cas_put(profile, expected_generation, actor)`, `delete(name, expected_generation, actor)`, `watch(since)`, `log(name)`, and **PROPOSED** `rename(from, to, expected_generation, actor)` | the hub (#661); `rename` answers `not-implemented` until #665 is confirmed |
| `ProfileScope` | `resolve(profile, pane?)`, `edit_spec(profile?, pane, edit, act)` | `holler-cli/src/pane/profile_scope.rs` (#663; #670 creates the empty file) |
| `HerdrPort` | `ensure_pane`, `send_text`, `send_keys`, `read`, `close`, `snapshot`, `version` | `holler-adapter-herdr` (#640) |
| `HostPort` | `ensure_session`, `run`, `stop_owned`, `ps` | `holler-adapter-host` (#641) |
| `HarnessPort` | `serve`, `health`, `create_session`, `list_sessions`, `abort`, `attach_tui`, `select_session`, `shown_session` | `holler-adapter-opencode` (#642) |
| `Prober` | `run_probe(argv, expect, timeout)` | `SystemProber` (the free `run_probe`, #663) |
```
```
docs/adr/ADR-0021.md:176-181
**Decided: the crate dependency rules.**

- `holler-pane` holds types, traits and the pure transaction engines (`tx_launch.rs` #644, `tx_switch.rs` #645, `tx_apply.rs`
  #664, `reconcile.rs` and `findings.rs` #647, `profile_snapshot.rs` and `profile_diff.rs` #662, `import.rs` #650), which work
  only through the ports. It depends on `serde`, `serde_json` and `holler-proto`, and takes no async runtime. Its one direct
  side effect is the probe runner (`run_probe`, #663), which runs a process with the standard library.
```
The test kit cannot reach anything in `holler-cli` (so the fake cannot call `reconcile_step`; F1); its manifest has two
dependencies:
```
docs/adr/ADR-0021.md:184-186
- `holler-pane-testkit` (empty today; #638 fills it) may depend on `holler-pane` and `serde_json`, and **must not depend on
  `holler-cli`**: it is a dev-dependency of both the hub and the CLI, so a normal dependency back would make a cycle. Its
  envelope conformance helper (#638) parses the CLI envelope with `serde_json` alone.
```
```
crates/holler-pane-testkit/Cargo.toml:14-20
[dependencies]
# The ports the fakes implement and the suites drive (`PaneStore`, `Pane`, `PaneError`,
# `next_generation`); for every fake and suite in this crate.
holler-pane = { path = "../holler-pane" }
# Parses the verbs' `--format=json` output for `envelope.rs` (`check_envelope`,
# `check_ndjson`); the testkit cannot use the CLI's own `Envelope` type.
serde_json = { workspace = true }
```
```
docs/adr/ADR-0021.md:190-192
**Decided: where the I8 transaction helper lives.** The trait `ProfileScope` is in `holler-pane/src/profile.rs` (frozen by
#637). The real implementation is in `holler-cli/src/pane/profile_scope.rs` (#663) and the fake is in `holler-pane-testkit`
(#638). The spec-editing verbs call `edit_spec` and do not write the profile themselves.
```

Section 8's record-fence bullet, whose "pane doctor command line for that pane" AC 14e qualifies in place:
```
docs/adr/ADR-0021.md:273-275
- A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  writes nothing more, and prints the reconcile step (the pane doctor command line for that pane).
```

Section 8, the I8 write order, which `edit_spec` implements step by step (steps 1, 2, 3, 5 and 6 are the scope's; step 4
is the verb's, inside its act; step 2 is ADR lines 292-293, step 5 lines 296-298, step 6 lines 299-301):
```
docs/adr/ADR-0021.md:281-305
- No store transaction spans the two registries. The membership rule is enforced on `pane/cas_put`: setting `Pane.profile`
  to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
  names a pane of another profile (a detached spec) is not refused.

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

Section 12 (a timeout compensates and prints the reconcile step) and "Decisions taken", items 1 and 2:
```
docs/adr/ADR-0021.md:457-460
**Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step. A crash between steps leaves state that
the next pane doctor run finds and reports (#644's acceptance). No verb leaves work running after it exits.
```
```
docs/adr/ADR-0021.md:539-546
1. **The I8 compensation (section 8): the epic's order stays.** The profile is written first. A failed act restores the
   profile's specs by a second write, so the specs equal what they were but the generation has moved by two and the log
   shows the edit and its reversal. The acceptance of #663 and the conformance case in #638 are amended from "generation
   equal" to "specs equal".
2. **`pane-in-other-profile` runs inside the pane registry's compare-and-swap (section 8).** It needs only the stored pane
   record, so it runs under the pane lock where that record is at hand, and no writer can slip in between the check and the
   write. `check_membership` (#669) keeps the check that the named profile exists. #661 makes this one-line change in the
   registry code #639 creates, and #661's blast radius is widened to allow it.
```

The CLI rows the reconcile step names (ADR 0003; `pane doctor` declares only `--profile NAME` today, and #647 owns its
positionals; `profile show` gets its `NAME` positional from #662):
```
docs/adr/ADR-0003.md:61-68
holler pane doctor [--profile NAME]                               #647

holler pane import                                                #650

holler profile create                                             #662
holler profile delete                                             #662
holler profile list                                               #662
holler profile show                                               #662
```

### D. The frozen trait and types (`holler-pane`, #637; not edited here)

```
crates/holler-pane/src/profile.rs:259-273
/// What a spec edit does to a profile's entry for one pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecEdit {
    /// Set the pane's entry to this spec (boxed: a spec is large).
    Set(Box<ProfileSpec>),
    /// Remove the pane's entry.
    Remove,
}

/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}
```
```
crates/holler-pane/src/profile.rs:319-360
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

    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError>;

    /// The change log of the profile `name`, oldest first.
    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>;

```
```
crates/holler-pane/src/profile.rs:373-404
/// The helper every `--profile` verb uses to scope itself to a profile and to edit a
/// spec in one transaction with the live change. Implemented in
/// `holler-cli/src/pane/profile_scope.rs` (#663); frozen by #637.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
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

A profile name allows spaces and any non-control character (so a printed command line must quote it); a pane name is
`[a-z0-9-]` (the ADR 0005 grammar), so it needs no quoting:
```
crates/holler-pane/src/profile.rs:30-61
/// The display name of a profile: spaces allowed, e.g. `Some Profile`.
///
/// Parsing trims surrounding whitespace and refuses a name that is empty, longer
/// than 64 characters, has a control character, or has no ASCII letter or digit
/// (the slug would be empty, and the slug is persisted and unique-checked by #661).
/// All of these are `usage`. Serde goes through [`ProfileName::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProfileName(String);

impl ProfileName {
    /// Parse a display name (see the type docs for the rules).
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
```
```
crates/holler-proto/src/vocab.rs:200-223
/// The per-segment checks shared by all three name kinds: length limit,
/// word-char first/last, and no disallowed characters.
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

### E. The stubs this story fills (the whole of each file today)

```
crates/holler-cli/src/pane/profile_scope.rs:1-5
//! The real `ProfileScope` (epic #633): the helper every `--profile` verb uses to scope itself
//! to a profile and to edit a spec in one transaction with the live change.
//!
//! Empty in the CLI skeleton (story #670). Story #663 fills it; the trait is
//! `holler_pane::ProfileScope`, frozen by #637.
```
```
crates/holler-pane/src/probe.rs:1-36
//! The health probe of a pane (epic #633, B1): a `check` argv whose output must
//! contain every `expect` string.
//!
//! **Frozen by #637:** [`ProbeResult`] (which `Pane.probe.last` persists) and the
//! signature of [`run_probe`]. The body of [`run_probe`] is #663's; until it lands
//! the function is a stub that never reports success, so a launch that should be
//! refused with `probe-failed` cannot sail through on a stub.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::argv::Argv;

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

The `Prober` port and `SystemProber`, which call the free `run_probe` (unchanged; the runner is reached through them):
```
crates/holler-pane/src/ports.rs:202-222
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
```

`Argv` (the probe's argv type; an element is data and never re-split):
```
crates/holler-pane/src/argv.rs:20-43
/// An argument vector: the program and its arguments, one string each.
///
/// It serializes as a JSON array of strings and reads back from one. Spaces and
/// shell metacharacters inside an element are data, never re-split. A JSON string
/// where an `Argv` is expected is `command-not-argv`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Argv(Vec<String>);

impl Argv {
    /// An argv from its elements.
    pub fn new(parts: Vec<String>) -> Self {
        Self(parts)
    }

    /// The elements: the program first, then its arguments.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// The elements, consuming the argv.
    pub fn into_vec(self) -> Vec<String> {
        self.0
    }
```

The one existing test of the stub, which must stay green (a missing binary is still `Error`):
```
crates/holler-pane/tests/ports_test.rs:512-527
#[test]
fn run_probe_stub_never_reports_success() {
    // The free `run_probe` is a stub until #663 builds it: it must not say Ok, or a
    // launch that should refuse with probe-failed would sail through.
    let missing_binary = argv(&["/nonexistent/holler-probe-binary"]);
    let expect = ["qwen38".to_string()];
    let timeout = Duration::from_secs(1);

    assert!(!matches!(
        run_probe(&missing_binary, &expect, timeout),
        ProbeResult::Ok
    ));
    assert!(!matches!(
        SystemProber.run_probe(&missing_binary, &expect, timeout),
        ProbeResult::Ok
    ));
```

The unwired stand-ins #649 replaces with `StoreScope` and `SystemProber` (not touched here):
```
crates/holler-cli/src/pane/wiring.rs:203-227
impl ProfileScope for Unwired {
    fn resolve(
        &self,
        _profile: &ProfileName,
        _pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        Err(PaneError::NotImplemented)
    }

    fn edit_spec(
        &self,
        _profile: Option<&ProfileName>,
        _pane: &PaneName,
        _edit: &SpecEdit,
        _act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

impl Prober for Unwired {
    fn run_probe(&self, _argv: &Argv, _expect: &[String], _timeout: Duration) -> ProbeResult {
        ProbeResult::Error("not implemented".to_owned())
    }
}
```

### F. The error variants and exit classes the scope returns (closed set, not edited: ruling 3)

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
crates/holler-pane/src/error.rs:454-467
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names it. (#638-#642.)
    Timeout { op: String },
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
    /// `session-not-found`: no harness session of that id; `what` is the id.
    /// (#638-#642.)
    SessionNotFound { what: String },
    /// `store-corrupt`: a stored file or record cannot be read back; the store fails
    /// closed rather than dropping state. `what` names the store. (#639/#661.)
    StoreCorrupt { what: String },
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
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
```
crates/holler-pane/src/error.rs:651-656
            PaneError::Conflict => f.write_str(
                "the record changed since it was read (generation conflict); read it again and retry",
            ),
            PaneError::ProbeFailed { message } => write!(f, "health probe failed: {message}"),
            PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
            PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
```
```
crates/holler-pane/src/error.rs:673-677
            PaneError::Timeout { op } => write!(f, "timed out: {op}"),
            PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
            PaneError::SessionNotFound { what } => write!(f, "session not found: {what}"),
            PaneError::StoreCorrupt { what } => write!(f, "store corrupt: {what}"),
            PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),
```
The payload names (Decision 5 stretches `op` on purpose and says so), and `code()`, beside which F2 hoists the append
helper:
```
crates/holler-pane/src/error.rs:399-401
/// The payload names by convention: `what` is the thing the error is about (a
/// pane, a profile, a socket, the text that was refused), `message` is free text,
/// `op` is the operation that timed out.
```
```
crates/holler-pane/src/error.rs:494-497
impl PaneError {
    /// The kebab-case code of this error: one of [`ALL_CODES`], or the open code of a
    /// [`Refused`](PaneError::Refused).
    pub fn code(&self) -> &str {
```

### G. The test kit: the fake the real scope must match, the suite it must pass, and the seams the tests use

The fake's documented rules and its constructor (the real scope takes the same three arguments):
```
crates/holler-pane-testkit/src/profile_scope.rs:31-56
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
/// the verb's, inside its act.
pub struct FakeProfileScope {
```
```
crates/holler-pane-testkit/src/profile_scope.rs:65-77
impl FakeProfileScope {
    // ASSUMPTION (#663): a scope can be built over any `ProfileStore` and `PaneStore`;
    // the suite builds the one under test over the two fakes.
    /// A scope over `profiles` and `panes` that logs every profile write as `actor`,
    /// with no hook armed.
    pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
        Self {
            profiles,
            panes,
            actor,
            restore_hook: Mutex::new(None),
        }
    }
```

The fake's plan, restore and `edit_spec` (the `ASSUMPTION (#663)` comments are the points #663 confirms or decides):
```
crates/holler-pane-testkit/src/profile_scope.rs:128-231
    /// Section 8, step 1: `stored` with the edit, once the scope's guards pass. The pane
    /// record is read for every edit; only a `Set` is checked for membership.
    fn plan(
        &self,
        stored: &Profile,
        pane: &PaneName,
        edit: &SpecEdit,
    ) -> Result<Profile, PaneError> {
        if let SpecEdit::Set(spec) = edit {
            check_filed_under(spec, pane)?;
        }
        let record = self.panes.get(pane)?;
        if let (SpecEdit::Set(_), Some(record)) = (edit, record.as_ref()) {
            check_joins(record, &stored.name)?;
        }
        Ok(with_edit(stored, pane, edit))
    }

    /// Section 8, steps 5 and 6: `act` failed with `failure` after `written`, so put the
    /// specs of `stored` back by a compare-and-swap at `written`'s generation. Returns
    /// what `edit_spec` answers: `failure` once they are back, `profile-conflict` when
    /// another writer moved the profile first, or else the restoring write's own error.
    fn restore(
        &self,
        stored: Profile,
        written: Profile,
        pane: &PaneName,
        failure: PaneError,
    ) -> PaneError {
        // A statement of its own, so the lock is released before the hook runs.
        let hook = lock(&self.restore_hook).take();
        if let Some(hook) = hook {
            hook();
        }
        let restored = Profile {
            panes: stored.panes,
            ..written
        };
        // ASSUMPTION (#663): a restore is one compare-and-swap at g + 1, not retried.
        match self
            .profiles
            .cas_put(&restored, restored.generation, &self.actor)
        {
            Ok(_) => failure,
            Err(PaneError::Conflict) => PaneError::ProfileConflict {
                what: format!(
                    "{:?} was changed by another writer during the live change to {pane}, \
                     so its specs were not restored after that change failed ({failure}); \
                     the other writer's version stays",
                    restored.name.as_str()
                ),
            },
            // ASSUMPTION (#663), open: a restoring write that fails with anything but a
            // conflict returns its own error, so the profile keeps an edit nothing live
            // matches, the error does not name it, and the act's error is lost. ADR-0021
            // section 8 decides only the conflict (step 6); #663 decides this case, and
            // the fake and the suite's list are amended to match.
            Err(other) => other,
        }
    }
}

impl ProfileScope for FakeProfileScope {
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError> {
        let stored = self.stored(profile)?;
        let panes = match pane {
            None => self.members(&stored.name)?,
            Some(name) => vec![self.member(&stored.name, name)?],
        };
        Ok(ResolvedScope {
            profile: stored,
            panes,
        })
    }

    // ASSUMPTION (#663): the scope writes no pane record. Recording the pane is the
    // verb's, inside its act, whose signature returns `()`.
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
```
crates/holler-pane-testkit/src/profile_scope.rs:234-293
/// Whether the record `pane` names `profile`, compared by slug.
fn belongs(pane: &Pane, profile: &ProfileName) -> bool {
    pane.profile
        .as_ref()
        .is_some_and(|named| named.slug() == profile.slug())
}

/// The fake's own guard against a verb that files a spec under the wrong pane: `usage`
/// when `spec` names a pane other than `pane`.
fn check_filed_under(spec: &ProfileSpec, pane: &PaneName) -> Result<(), PaneError> {
    if spec.pane == pane.as_str() {
        Ok(())
    } else {
        Err(PaneError::Usage {
            message: format!(
                "a spec for the pane {:?} cannot be set as the spec of {pane}",
                spec.pane
            ),
        })
    }
}

// ASSUMPTION (#661/#663): the scope checks `pane-in-other-profile` itself for a `Set`,
// before the profile write, as well as the pane registry doing so inside its
// compare-and-swap (ADR-0021 "Decisions taken", item 2), so nothing is written to the
// profile and nothing live moves for a pane that cannot join it. It runs with or
// without `--spec-only`, since the scope cannot see that an act is empty. ADR-0021
// section 8, step 1 states it. A `Remove` is not checked: a detached spec stays
// removable.
/// The refusal of a `Set` for `record` in `profile`: the fake pane store's membership
/// rule (slugs compared), applied as if the record were written into `profile`.
fn check_joins(record: &Pane, profile: &ProfileName) -> Result<(), PaneError> {
    let joined = Pane {
        profile: Some(profile.clone()),
        ..record.clone()
    };
    check_membership(Some(record), &joined)
}

// ASSUMPTION (#663): a `Set` replaces an entry in place, keeping its index.
/// `stored` with `edit` made to the entry of `pane`: a `Set` replaces it in place or
/// appends one; a `Remove` drops it, the other entries keeping their order, and leaves
/// the specs as they are when there is none.
fn with_edit(stored: &Profile, pane: &PaneName, edit: &SpecEdit) -> Profile {
    let mut panes = stored.panes.clone();
    match edit {
        SpecEdit::Set(spec) => {
            let spec = ProfileSpec::clone(spec);
            match panes.iter_mut().find(|entry| entry.pane == pane.as_str()) {
                Some(entry) => *entry = spec,
                None => panes.push(spec),
            }
        }
        SpecEdit::Remove => panes.retain(|entry| entry.pane != pane.as_str()),
    }
    Profile {
        panes,
        ..stored.clone()
    }
}
```

The membership rule the fake's `check_joins` applies, and the hub's own copy (both private to their crates, so the CLI
cannot call either):
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
crates/holler-hub/src/panes/store.rs:338-359
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
    }
}
```

The suite: what it pins, what it leaves open for #663, and how #663's acceptance maps onto its cases:
```
crates/holler-pane-testkit/src/conformance/profile_scope.rs:28-64
//! **What the suite fixes that the port leaves open**, as the store suites did for #639 and
//! #661. Each point carries an `ASSUMPTION (#663)` comment at its case or helper, and the
//! fake the same comment at its code. #663 confirms or amends them here first:
//!
//! - the real scope can be built over any `ProfileStore` and `PaneStore` (the `build`
//!   closure);
//! - the scope writes no pane record: recording the pane is the verb's, inside its act,
//!   whose signature returns `()` (cases 5 and 9);
//! - a `Set` replaces an entry in place, keeping its index (case 5);
//! - the first write is not retried on a conflict (case 10), and a restore is one
//!   `cas_put` at g + 1 (cases 9 and 11);
//! - `profile-not-found` comes before `pane-in-other-profile` (case 13);
//! - `edit_spec(None, ..)` makes no profile store call (case 12);
//! - **open, for #663 to decide (no case pins it):** a restoring write that fails with
//!   anything but a conflict (`timeout`, `store-corrupt`, `unavailable`). The fake returns
//!   that error as it is, so the profile keeps an edit nothing live matches, the error does
//!   not name the profile, and the act's error is lost. ADR-0021 section 8 decides only the
//!   conflict (step 6). #663 may instead name the profile and its unrestored specs, or
//!   print the reconcile step; whichever it picks, the fake and this list are amended to
//!   match.
//!
//! `ASSUMPTION (#661/#663)`: for a `Set` for a pane of another profile, the scope checks
//! `pane-in-other-profile` itself before the profile write (case 14), as well as the pane
//! registry doing so inside its compare-and-swap (ADR-0021 "Decisions taken", item 2), so
//! that nothing is written to the profile and nothing live moves for a pane that cannot
//! join it. It compares slugs, as the fake pane store does, and it refuses with or without
//! `--spec-only`, since the scope cannot see that an act is empty. ADR-0021 section 8,
//! step 1 states it. A `Remove` is not checked (case 15): a detached spec stays removable
//! (ADR-0021 section 8, and section 9's `pane close` row).
//!
//! **#663's acceptance, by case.** "Membership refusal" is cases 3 and 14. "Every-pane
//! scope" is case 1 (and 2 for a named pane). "A failed act leaves the profile's specs
//! equal to before (the generation has moved by two)" is case 9. "A successful act bumps
//! it once" is case 5 (and 6, 7 and 15). "A stale generation gives `profile-conflict`" is
//! case 11, a stale *restore*; a stale *first write* is `generation-conflict` (case 10),
//! under ADR-0021 section 8, step 2. "No profile is touched without `--profile`" is case
//! 12. "The profile must exist" (`profile-not-found`) is cases 4 and 13.
```
```
crates/holler-pane-testkit/src/conformance/profile_scope.rs:177-247
// ASSUMPTION (#663): the real scope can be built over any `ProfileStore` and `PaneStore`,
// so `build` makes it over the two fakes the suite seeds.
/// Run every case of [`profile_scope_cases`], in order, and return every case that did
/// not hold.
///
/// Per case the suite seeds a fresh `FakeProfileStore` and `FakePaneStore` with the
/// fixture of the module docs (seeding bypasses the call logs), then `build(profiles,
/// panes)` makes the scope under test over them. The suite drives the scope and inspects
/// the two fakes. A fixture that cannot be seeded fails every case, with the reason. How
/// each implementation runs the suite (the test kit cannot name the CLI):
///
/// ```text
/// // the fake:
/// assert_eq!(run_profile_scope_conformance(|profiles, panes| FakeProfileScope::new(profiles, panes, actor)), Ok(()));
/// // the real scope (#663, in holler-cli's tests): the CLI's ProfileScope built over the two fakes.
/// ```
pub fn run_profile_scope_conformance<S, F>(mut build: F) -> Conformance
where
    S: ProfileScope,
    F: FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S,
{
    run_cases(
        &CASES,
        || (seed(&mut build), ()),
        |case, seeded| {
            let seeded = seeded.as_ref().map_err(String::clone)?;
            case(&Bench {
                scope: &seeded.scope,
                profiles: &seeded.profiles,
                panes: &seeded.panes,
            })
        },
    )
}

/// A fresh fixture and the scope `build` makes over it.
fn seed<S>(
    build: &mut impl FnMut(Arc<FakeProfileStore>, Arc<FakePaneStore>) -> S,
) -> Result<Seeded<S>, String> {
    let profiles = [sample(ALPHA, &[C1, C2, C3])?, sample(BETA, &[C3])?];
    let profiles = FakeProfileStore::seeded(profiles, &actor()?)
        .map_err(|e| format!("the fixture's profiles cannot be seeded: {e}"))?;
    let panes = seeded_panes().map_err(|e| format!("the fixture's panes cannot be seeded: {e}"))?;
    let (profiles, panes) = (Arc::new(profiles), Arc::new(panes));
    let scope = build(Arc::clone(&profiles), Arc::clone(&panes));
    Ok(Seeded {
        scope,
        profiles,
        panes,
    })
}

/// The fixture's pane store: c1 and c2 in Alpha, c3 in Beta and c4 in no profile.
fn seeded_panes() -> Result<FakePaneStore, PaneError> {
    let (alpha, beta) = (ProfileName::parse(ALPHA)?, ProfileName::parse(BETA)?);
    FakePaneStore::seeded([
        Pane {
            profile: Some(alpha.clone()),
            ..sample_pane(C1)?
        },
        Pane {
            profile: Some(alpha),
            ..sample_pane(C2)?
        },
        Pane {
            profile: Some(beta),
            ..sample_pane(C3)?
        },
        sample_pane(C4)?,
    ])
}
```

The cases the acceptance names most directly (9 failed act, 11 restore conflict, 12 no profile, 14 another profile):
```
crates/holler-pane-testkit/src/conformance/profile_scope/act.rs:24-63
// ASSUMPTION (#663): a restore is one `cas_put` at g + 1, and the scope writes no pane
// record.
/// Case 9: an act that fails with `unavailable` makes `edit_spec` answer exactly that
/// error. Alpha's specs equal what they were before, but its generation is g + 2: the
/// log gained two `updated` entries by one actor, the edit at g + 1 and its reversal at
/// g + 2 (ADR-0021 "Decisions taken", item 1). The act ran once and the pane store is
/// unchanged.
pub(super) fn failed_act_restores_the_specs(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let before = stored(b, &alpha)?;
    let log_before = history(b.profiles, &alpha)?;
    let panes_before = succeeds("pane_store.list", b.panes.list())?;
    let call = "edit_spec(Alpha, c1, Set(s')) with an act that fails";
    let (edited, runs) = edit_with(b, Some(&alpha), C1, &set(changed_spec(C1)), || {
        Err(act_failed())
    })?;
    expect_eq(&format!("what {call} answered"), edited, Err(act_failed()))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    let after = stored(b, &alpha)?;
    expect_eq(
        &format!("the specs after {call}"),
        &after.panes,
        &before.panes,
    )?;
    expect_eq(
        &format!("the generation after {call}"),
        after.generation,
        SEEDED + 2,
    )?;
    expect_eq(
        &format!("what the log gained after {call}"),
        gained(b, &alpha, &log_before)?,
        vec![(SEEDED + 1, UPDATED), (SEEDED + 2, UPDATED)],
    )?;
    expect_eq(
        &format!("pane_store.list after {call}"),
        succeeds("pane_store.list", b.panes.list())?,
        panes_before,
    )
}
```
```
crates/holler-pane-testkit/src/conformance/profile_scope/act.rs:83-138
// ASSUMPTION (#663): a restore is one `cas_put` at g + 1, not retried.
/// Case 11: during the act, another writer stores Alpha with other specs, and then the
/// act fails. The restoring write conflicts, so `edit_spec` is `profile-conflict` naming
/// Alpha, and Alpha is the other writer's version, at g + 2 with its specs (ADR-0021
/// section 8, step 6). The act ran once.
pub(super) fn restore_conflict_is_profile_conflict(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let (other, who) = (sample(ALPHA, &[C4])?, actor()?);
    let mut moved = None;
    let call = "edit_spec(Alpha, c1, Set(s')) when Alpha moves during an act that fails";
    let (edited, runs) = edit_with(b, Some(&alpha), C1, &set(changed_spec(C1)), || {
        moved = Some(b.profiles.concurrent_put(&other, &who));
        Err(act_failed())
    })?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    if let Some(put) = moved {
        succeeds("the other writer's concurrent_put inside the act", put)?;
    }
    match edited {
        Err(e) if e.code() == PROFILE_CONFLICT && !e.to_string().contains(ALPHA) => Err(format!(
            "{call}: the `{PROFILE_CONFLICT}` does not name {ALPHA:?}: {e}"
        )),
        edited => expect_code(call, edited, PROFILE_CONFLICT),
    }?;
    let now = stored(b, &alpha)?;
    expect_eq(
        &format!("the generation of Alpha after {call}"),
        now.generation,
        SEEDED + 2,
    )?;
    expect_eq(
        &format!("the specs of Alpha after {call}"),
        now.panes,
        other.panes,
    )
}

// ASSUMPTION (#663): without a profile the scope makes no profile store call.
/// Case 12: without a profile, `edit_spec` only runs the act: `Ok(None)` when the act
/// succeeds and the act's own error when it fails, the act running once each time, and
/// the profile store saw no call at all.
pub(super) fn no_profile_runs_only_the_act(b: &Bench<'_>) -> Result<(), String> {
    let call = "edit_spec(None, c1, Set(s'))";
    let (edited, runs) = edit(b, None, C1, &set(changed_spec(C1)))?;
    expect_eq(&format!("what {call} answered"), edited, Ok(None))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    let call = "edit_spec(None, c1, Set(s')) with an act that fails";
    let (edited, runs) = edit_with(b, None, C1, &set(changed_spec(C1)), || Err(act_failed()))?;
    expect_eq(&format!("what {call} answered"), edited, Err(act_failed()))?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 1)?;
    expect_eq(
        "the calls the profile store saw",
        b.profiles.faults().calls(),
        Vec::new(),
    )
}
```
```
crates/holler-pane-testkit/src/conformance/profile_scope/act.rs:160-176
// ASSUMPTION (#661/#663): the scope refuses a `Set` for a pane of another profile itself,
// before the profile write, as well as the pane registry inside its compare-and-swap
// (ADR-0021 section 8, step 1, and "Decisions taken", item 2), with or without
// `--spec-only`. A `Remove` is not checked (case 15).
/// Case 14: a `Set` for c3, whose record names Beta, is `pane-in-other-profile` before
/// anything is written or moved: the act never ran, the profile store saw no `cas_put`,
/// and Alpha is unchanged.
pub(super) fn pane_in_other_profile_before_any_write(b: &Bench<'_>) -> Result<(), String> {
    let alpha = profile_name(ALPHA)?;
    let before = shown(b.profiles, &alpha)?;
    let call = "edit_spec(Alpha, c3, Set(sample_spec(c3)))";
    let (edited, runs) = edit(b, Some(&alpha), C3, &set(sample_spec(C3)))?;
    expect_code(call, edited, IN_OTHER_PROFILE)?;
    expect_eq(&format!("the runs of the act of {call}"), runs, 0)?;
    no_write(b, call)?;
    unchanged(b.profiles, &alpha, &before, call)
}
```
```
crates/holler-pane-testkit/src/conformance/profile_scope/act.rs:201-206
/// The error the failing acts of the cases answer.
fn act_failed() -> PaneError {
    PaneError::Unavailable {
        what: "act".to_owned(),
    }
}
```

The seams the extra scope tests use: a one-shot fault on the next `cas_put` (armed inside the act, it fails the restoring
write, not the first one) and another writer's put:
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
crates/holler-pane-testkit/src/profile_store.rs:150-162
    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<ProfileStoreOp> {
        &self.faults
    }

    /// Another writer stores `profile` unconditionally, at the stored generation + 1
    /// (or at 1 for a new profile), without the name rule, logs its entry with `actor`
    /// and publishes its event. It bypasses the faults and the call log. Returns the
    /// stored record.
    pub fn concurrent_put(&self, profile: &Profile, actor: &Actor) -> Result<Profile, PaneError> {
        self.put(profile, Writer::Other, actor)
    }
```

The fake prober (verb tests use it, so no verb test runs a real probe):
```
crates/holler-pane-testkit/src/prober.rs:1-9
//! `FakeProber`, a `Prober` that answers a scripted `ProbeResult` per argv and records
//! every run, so no verb test waits for the real probe runner (#663) (slice d of #638,
//! #683).
//!
//! An argv nobody scripted answers [`ProbeResult::Error`], never [`ProbeResult::Ok`],
//! so a test that forgot to script a probe cannot pass it (the same reason the stub
//! `holler_pane::run_probe` never answers `Ok`). The fake has no fault switch: a fault
//! answers a `PaneError`, and a `Prober` answers a `ProbeResult`, so a probe that times
//! out or fails is simply a scripted `ProbeResult::Error`.
```

The CLI already has the test kit as a dev-dependency, so `profile_scope.rs`'s `#[cfg(test)]` module can run the suite with
no manifest change:
```
crates/holler-cli/Cargo.toml:431-435
[dev-dependencies]
# #670: the fakes and conformance suite of holler-pane (empty until #638). Its
# consumer test is `testkit_links` in tests/pane_verbs/list.rs; #638 and
# #643-#647 use it from their own pane_verbs/<verb>.rs files.
holler-pane-testkit = { path = "../holler-pane-testkit" }
```
A dev-dependency is in scope for a library crate's own `#[cfg(test)]` modules when they are built by `cargo test --lib`, so
the test module imports the suite as `holler_pane_testkit::conformance::profile_scope::run_profile_scope_conformance`
(verified at `3bdd129`: `crates/holler-pane-testkit/src/lib.rs:31` is `pub mod conformance;`,
`crates/holler-pane-testkit/src/conformance/mod.rs:21` is `pub mod profile_scope;`, and the function is `pub fn` at
`crates/holler-pane-testkit/src/conformance/profile_scope.rs:193`), and the fixture as
`holler_pane_testkit::fixture::{sample_profile, sample_pane, sample_spec}` (`fixture.rs:121`, `:41`, `:92`), with the
fault enums `holler_pane_testkit::profile_store::ProfileStoreOp` (`profile_store.rs:27`) and
`holler_pane_testkit::pane_store::PaneStoreOp` (`pane_store.rs:23`).

### H. Workspace rules that bind the code

`holler-pane` has three dependencies and no `libc`; nothing may be added (no new dependency, no new `unsafe`):
```
crates/holler-pane/Cargo.toml:12-21
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise).
[dependencies]
# The records, the guards and the reply derive Serialize/Deserialize.
serde = { workspace = true }
# `PaneReply.data` and the params structs carry `serde_json::Value`.
serde_json = { workspace = true }
# `PaneName` is a newtype over `holler_proto::vocab::SessionName` (the ADR 0005
# name grammar, reused rather than copied).
holler-proto = { path = "../holler-proto" }
```
```
Cargo.toml:19-30
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
cognitive_complexity = "deny"
too_many_lines = "deny"
struct_excessive_bools = "deny"
large_enum_variant = "warn"

[workspace.lints.rust]
dead_code = "deny"
```
`clippy.toml`: `cognitive-complexity-threshold = 15`, `too-many-lines-threshold = 100`. `scripts/lint.sh`:
```
scripts/lint.sh:21-26
if grep -rn --include='*.rs' -E '(#!?\[allow\(|cfg_attr\([^)]*allow\()' crates/ \
    | grep -vE '//\s*#[0-9]+' \
    | grep -v 'allow(clippy::assertions_on_constants)'; then
  echo "lint: every #[allow] needs a trailing '// #NNN' issue link"
  fail=1
fi
```
```
scripts/lint.sh:43-52
# 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
#    file cross 1k unnoticed).
while read -r n f; do
  if [ "$n" -ge 900 ]; then
    echo "lint: $f is $n lines — decompose before merging"
    fail=1
  elif [ "$n" -ge 600 ]; then
    echo "warn: $f is $n lines"
  fi
done < <(find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | grep -v ' total$')
```

The repo's precedent for a child in its own process group (a test-support helper, not reusable from library code):
```
crates/holler-cli/tests/support/mod.rs:782-795
pub fn make_own_process_group(cmd: &mut Command) {
    // `process_group(0)` is a safe API: it only records that the child should
    // be placed in a new process group (pgid == its own pid) at spawn time.
    // (The actual group placement is done by the kernel at `fork`+`exec`.)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}
```

Formatting: the tree is not rustfmt-clean, so `cargo fmt --check` cannot be this story's gate (ruling 4 above). Observed by
O at `3bdd129`: `cargo fmt --all --check` exits 1 with 4,049 `Diff in` hunks across the tree, while
`rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs crates/holler-cli/src/pane/profile_scope.rs` exits 0.

The process-group kill this design uses, observed by O on Linux (procps-ng 4.0.4 `/bin/kill`), on a scratch `sleep` it
started itself:
```
bash -c 'set -m; sleep 30 & pid=$!; /bin/kill -s KILL -- -$pid; echo "bin kill exit $?"; sleep 0.3; ps -o stat= -p $pid || echo gone'
-> bin kill exit 0
-> gone
```
The same `kill -s <SIG> -- -<pgid>` form is what #641's host adapter brief uses through the `kill` binary (it records the
macOS BSD `kill` accepting it); CI's macOS job runs this story's tests and so checks it there. #641's line, verbatim (its
brief on the unmerged branch `issue-641-implementation`, commit `770c948`):
```
docs/handoffs/641-brief.md:531-532 (at 770c948)
- **`kill` binary portability** (`kill -s TERM -- -PGID`): verified on Linux procps; macOS BSD `kill` accepts the same form.
  Real-tmux tests are opt-in, so a macOS difference cannot break CI.
```
That line is an assertion, not a macOS observation, and #641's tests that use it are opt-in, so nothing has yet run this
form on macOS. This story's AC 8f and 8g are not opt-in: they run on CI's macOS job and are the first macOS evidence. O
could not read the macOS `kill(1)` page from this Linux host.

### I. The consumers that code against this brief: #644's planned const and #696's scope

#644's amended brief (unmerged branch `issue-644-implementation`, commit `7195993`) declares the unscoped step in its own
`launch.rs`; Decision 8 hosts it here instead (W-1 of the architecture review), so #644's brief must be amended (F5):
```
docs/handoffs/644-brief.md:1557-1558 (at 7195993)
/// The step without a profile: #663's wording with the ADR 0003 row's optional `--profile` group dropped.
pub(crate) const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor";
```
```
docs/handoffs/644-brief.md:123-128 (at 7195993)
- **C-15 One reconcile step, two owners in the plan.** ADR-0021 section 8 step 6 and section 12 make the reconcile step every
  acting verb's concern, and #663's plan (I-3, decisions 5-8) makes `profile_scope::reconcile_step` the one copy and adds
  that step to the scope's own `profile-conflict`, restore-failure and first-write-timeout errors. A verb that also appended
  its own step would print it twice under the real scope; the fake scope adds none, so tests over the fake alone cannot see
  it. Resolution (decision 15): #644 calls #663's function, appends the step only to an error that does not already carry
  it, and pins that with the real `StoreScope` (AC 16k).
```
#696 (open), the story that later lifts this runner (Decision 20 keeps its mechanics apart for it):
```
issue #696, the title, then body:3-7
refactor(pane): one bounded subprocess runner shared by the adapters and run_probe

The adapter briefs each need a bounded subprocess runner (timeout, kill on timeout, captured stderr): #641 has `exec.rs` and `tmux.rs`, #642 plans a private equivalent for its tmux and `kill` calls, and #663 builds `run_probe` in `holler-pane` with the same job.

## Scope

When #663 lands, expose its bounded runner from `holler-pane` and switch the host and OpenCode adapters to it, removing the private copies. No behaviour change: the adapters' existing tests keep passing, including the fake-`kill` seam.
```

## Contradictions found

- **C1. Rigor.** The issue says `rigor: in-session` (A, line 24); this run is `second-opinion`, set by the orchestrator.
  The higher rigor wins; nothing in the design depends on it.
- **C2. "A stale generation gives `profile-conflict`" (issue acceptance) versus the suite and ADR-0021 section 8.** A stale
  *first* write is `generation-conflict` and nothing live moved (ADR section 8 step 2; suite case 10); only a stale
  *restoring* write after a failed act is `profile-conflict` (step 6; case 11). The suite's own docs (G, lines 61-63) map the
  acceptance this way. This brief follows the ADR and the suite.
- **C3. "`--spec-only` makes `edit_spec` skip the act" (issue scope).** The frozen `edit_spec` (D, lines 397-403) has no
  spec-only argument. Resolution: the verb (#644, #646) passes an act that does nothing (`&mut || Ok(())`); the scope cannot
  tell, which is why ADR section 8 step 1 runs the membership refusal "with or without `--spec-only`". No change to the trait.
- **C4. "Without `--profile`, `resolve` and `edit_spec` never read or write any profile" (issue scope).** `resolve` takes a
  required `&ProfileName`, so only `edit_spec` has a no-profile path (`profile: None`, case 12). Without `--profile` a verb
  does not call `resolve`. Nothing to build.
- **C5. The test kit says "#663 confirms or amends them here first" and "the fake and this list are amended to match"**
  (G, `conformance/profile_scope.rs` lines 30 and 45-47), but the test kit is outside #663's blast radius. Resolution: the
  test kit is not edited. The ASSUMPTIONs the suite pins are all confirmed (Decision 4), so the suite passes unchanged; the
  three points no case pins and on which `StoreScope` differs from the fake (Decisions 5, 6 and 7: the open point decided,
  the first-write timeout text, and the reconcile step in the conflict message) are a follow-up for the test kit (F1).
- **C6. Bounded time.** The trait says each method "returns within I5's bound (default 10 s)" (D, lines 377-379), but
  `edit_spec` makes up to four store calls, each bounded at 10 s by its own port, and runs the verb's act, which the scope
  cannot bound or cancel. Resolution: Decision 11 (a documented narrowing).
- **C7. "Returns within the timeout it is given" (the `Prober` docs, E) and "a hung command gives `Error` within the
  timeout" (issue).** Giving up at the deadline still takes the kill and the reap. Resolution: Decision 15 (returns at the
  deadline plus a cleanup bounded to 1 s; the tests allow 2 s of slack, which is that 1 s cleanup budget plus 1 s of
  scheduling margin for the macOS runner, so AC 8e's 2,300 ms is the 300 ms timeout + 1,000 ms cleanup + 1,000 ms margin).
- **C8. `holler-pane` describes itself as having "no I/O"** (its `Cargo.toml` line 6, quoted in H only in part:
  `description = "... (types and traits only; no async runtime, no I/O)"`, and `src/lib.rs` lines 6-7, "It holds **types and
  traits only**: no async runtime, no I/O, no behaviour behind a stub"), while ADR-0021 section 5 (C, lines 180-181) makes
  `run_probe` its one direct side effect. Both files are outside the blast radius; the wording is a follow-up (F4).
- **C9. "The exact pane doctor command line" (ADR section 8 step 6).** `pane doctor` declares only `--profile NAME` today and
  #647 owns its positionals (C, ADR 0003 line 61). Resolution: Decision 8 names `holler pane doctor --profile '<P>'`, which
  is exact today, and `holler profile show '<P>'` (the epic's `show NAME`; its positional lands with #662).
- **C10. A probe that exits non-zero with every expected string in its output (Decision 14).** The frozen docs define the
  outcomes by output and leave the exit status unstated: `ProbeResult::Ok` is "The probe ran and every expected string was
  in its output" (E, `probe.rs:19`), and the issue's acceptance (A) names `Ok`, `Failed` and `Error` by output and timeout
  only. `ProbeResult::Error`'s doc is an open list: "The probe could not be run to a verdict (the program is missing, it
  timed out, ...)" (E, `probe.rs:23-24`). Resolution: "ran" in `Ok`'s doc is read as "ran to completion, exit 0", and a
  non-zero exit (or an end by a signal) is one more way of not reaching a verdict, so it is `Error` with the reason `the
  probe exited with status <n>` (Decision 14; AC 8d). What this choice does and does not change for a caller: the epic
  refuses alike on every non-`Ok` result, "a failing probe refuses launch/relaunch and apply's create/relaunch with
  probe-failed" (B, epic contract line 80), so `Error` versus `Failed` changes only the persisted `Pane.probe.last` and
  its reason text, never whether a launch proceeds; `Error` versus `Ok` is the substantive part. Why not `Ok`: `expect`
  may be empty (`PaneProbe.expect` is a `Vec<String>`, ADR-0021 line 51; AC 8c), so `Ok` on any exit would let `["false"]`
  pass a health gate, and a check such as `curl -f` reports an HTTP error only by its exit status. Why not `Failed`:
  `Failed { missing }` names the missing expected strings, and here there are none. The rule is recorded in `probe.rs`'s
  rustdoc and in ADR-0021 section 1's `ProbeResult` paragraph, in this change (AC 14a).

## Acceptance criteria

All commands run from the worktree root.

1. **Conformance.** `cargo test -p holler-cli --lib pane::profile_scope::tests::store_scope_passes_the_profile_scope_conformance_suite`
   passes: it asserts `run_profile_scope_conformance(|profiles, panes| StoreScope::new(profiles, panes, actor)) == Ok(())`
   with `actor = Actor::parse("conformance")`, so all 15 cases of `profile_scope_cases()` hold against the real scope. This is
   the issue's first acceptance bullet, case by case as in G (`conformance/profile_scope.rs` lines 58-64).
2. **A restore that fails without a conflict (the open point, Decision 5).** Test `restore_failure_keeps_its_code_and_names_the_unrestored_edit`,
   run once per error: `Timeout { op: "profile_store.cas_put" }`, `Unavailable { what: "profile store" }`,
   `StoreCorrupt { what: "profiles.json" }`. Fixture (AC 2-7 share it): a `FakeProfileStore` seeded with
   `sample_profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"])` by the actor `conformance` (so at generation 1), and a
   `FakePaneStore` seeded with `sample_pane("demo-c1r1")` and `sample_pane("demo-c2r1")`, each with `profile` set to
   `Demo Alpha`; `s'` is `sample_spec("demo-c1r1")` with `context.soft` raised by one, as the suite's `changed_spec` builds it.
   The act arms `profiles.faults().fail_next(ProfileStoreOp::CasPut, <error>)` and returns `Err(PaneError::Unavailable {
   what: "act".into() })`. Then: `edit_spec(Some(Demo Alpha), demo-c1r1, Set(s'))` is `Err` whose `code()` is the injected
   error's (`timeout`, `unavailable`, `store-corrupt`); its `to_string()` contains `Demo Alpha`, `demo-c1r1`,
   `unavailable: act`, `holler pane doctor --profile 'Demo Alpha'` and `holler profile show 'Demo Alpha'`, and no `\n`; the act
   ran once; `profiles.faults().calls()` is exactly `[Get, CasPut, CasPut]` (one restore, not retried); and `get(Demo Alpha)`
   is at generation 2 holding `s'` (the edit stayed, which the message says). How the two spellings of the name meet these
   substring checks: the prose part writes the name with `{:?}`, so it reads `profile "Demo Alpha"` (double quotes), which
   contains `Demo Alpha`; the reconcile step writes it POSIX-single-quoted, `'Demo Alpha'` (Decision 8). The test asserts
   the unquoted name and the two single-quoted command lines, never a double-quoted form, so both spellings satisfy it.
3. **The restore conflict prints the reconcile step (Decision 7).** Test `restore_conflict_names_the_act_error_and_the_reconcile_step`:
   the act calls `profiles.concurrent_put(&sample_profile("Demo Alpha", &["demo-c4r1"])?, &actor)` and returns
   `Err(Unavailable { what: "act" })`. The answer's `code()` is `profile-conflict`, and its `to_string()` contains
   `Demo Alpha`, `demo-c1r1`, `unavailable: act`, `holler pane doctor --profile 'Demo Alpha'` and
   `holler profile show 'Demo Alpha'`, and no `\n` (the name's two spellings meet these checks as in AC 2).
4. **A first write that times out (Decision 6).** Test `first_write_timeout_says_the_edit_may_have_landed`: with
   `fail_next(CasPut, Timeout { op: "profile_store.cas_put" })` armed before the call, `edit_spec(Some(Demo Alpha),
   demo-c1r1, Set(s'))` is `Err` with `code()` `timeout`, the act ran 0 times, and the message contains `may hold the edit`
   and `holler pane doctor --profile 'Demo Alpha'`. With `fail_next(CasPut, Unavailable { what: "profile store" })` instead,
   the answer equals `Err(PaneError::Unavailable { what: "profile store".into() })` exactly (passed through) and the act ran 0
   times.
5. **The reconcile step quotes the profile name (Decision 8).** Test `reconcile_step_single_quotes_the_profile_name`:
   `reconcile_step(&ProfileName::parse("Demo Alpha")?)` equals exactly
   `to reconcile, run holler pane doctor --profile 'Demo Alpha' and then holler profile show 'Demo Alpha'`; for the name
   `It's $(id) Demo` it contains `--profile 'It'\''s $(id) Demo'` (Rust literal `"--profile 'It'\\''s $(id) Demo'"`) and no
   `\n`. The same test asserts `RECONCILE_STEP_UNSCOPED == "to reconcile, run holler pane doctor"` exactly (#644's text, I).
6. **A spec filed under another pane is `usage` (Decision 9).** Test `set_of_a_spec_for_another_pane_is_usage_before_any_write`:
   `edit_spec(Some(Demo Alpha), demo-c1r1, Set(sample_spec("demo-c2r1")))` is `Err` with `code()` `usage`, the act ran 0
   times, and `profiles.faults().calls()` holds no `CasPut`.
7. **A pane store that cannot be read fails before the profile write (Decision 10).** Test
   `pane_store_fault_fails_a_remove_before_the_profile_write`: with `panes.faults().fail_next(PaneStoreOp::Get,
   PaneError::Unavailable { what: "pane store".into() })` armed, `edit_spec(Some(Demo Alpha), demo-c2r1, Remove)` equals
   `Err(PaneError::Unavailable { what: "pane store".into() })`, the act ran 0 times, `profiles.faults().calls()` holds no
   `CasPut`, and `get(Demo Alpha)` is unchanged (generation 1).
8. **The probe runner** (`cargo test -p holler-pane --lib probe::tests`), each test a real child process of a harmless
   command; every path a test creates is under one fresh directory `std::env::temp_dir()/hlr-probe-663-<pid>-<n>`, removed
   by a guard's `Drop` (Decision 21):
   - a. `all_expected_strings_present_is_ok`: argv `["printf", "%s\n%s\n", "alpha", "beta"]`, expect `["alpha", "beta"]`,
     5 s: `ProbeResult::Ok`.
   - b. `one_missing_string_is_failed_naming_it`: the same argv, expect `["alpha", "gamma", "beta"]`:
     `Failed { missing: vec!["gamma"] }` exactly.
   - c. `no_expect_and_exit_zero_is_ok`: `["true"]`, `[]`: `Ok`.
   - d. `non_zero_exit_is_error_even_with_every_string`: `["sh", "-c", "printf qwen38; exit 3"]`, `["qwen38"]`:
     `Error(reason)` with `reason` containing `exited with status 3`. And `["false"]`, `[]`: `Error` containing
     `exited with status 1`.
   - e. `hung_command_is_error_at_the_timeout`: `["sleep", "30"]`, 300 ms: `Error` containing `timed out`; the elapsed time is
     at least 300 ms and less than 2,300 ms.
   - f. `timeout_kills_the_whole_process_group`: `["sh", "-c", "sleep 30 & echo $! > \"$0\"; wait", "<dir>/pid"]`, 500 ms:
     `Error` containing `timed out`; the file `<dir>/pid` exists (an assertion, so a stub fails here, not on a panic), and
     within 2 s of the return the pid it names is gone or a zombie (polled with `ps -o stat= -p <pid>`: empty output, a
     non-zero exit or a state starting with `Z`). The test sends no signal itself.
   - g. `background_child_holding_stdout_is_a_timeout`: `["sh", "-c", "sleep 30 & echo $! > \"$0\"; echo up", "<dir>/pid"]`,
     expect `["up"]`, 500 ms: the shell exits at once but its background `sleep` keeps stdout open, so the answer is `Error`
     containing `timed out` (never `Ok`), and the `sleep` is gone or a zombie within 2 s, as in f.
   - h. `argv_is_never_given_to_a_shell`: `["printf", "%s|", "a;", "$(touch <dir>/m1)", "x; touch <dir>/m2"]`, expect
     `["a;|$(touch ", "|x; touch "]`: `Ok`, and neither `<dir>/m1` nor `<dir>/m2` exists afterwards. And the one-element argv
     `["printf hello"]` (a space inside the element) is `Error` containing `could not be started` (no shell split it).
   - i. `output_over_the_cap_is_error`: `["yes"]`, `[]`, 10 s: `Error` containing `more than 1 MiB`, returned in under 5 s.
   - j. `reasons_never_echo_argv_or_output`: `["/nonexistent/hlr-663-SENTINELARG", "SECRET663ARG"]` gives `Error` whose
     reason contains `could not be started` and neither `SENTINELARG` nor `SECRET663ARG`;
     `["sh", "-c", "printf SECRET663OUT; exit 4"]` gives `Error` without `SECRET663OUT`; `["printf", "SECRET663OUT"]` with
     expect `["absent"]` gives exactly `Failed { missing: vec!["absent"] }`.
   - k. `empty_argv_and_zero_timeout_are_errors_without_a_process`: `Argv::new(vec![])` gives `Error` containing `empty`;
     `["sh", "-c", "touch \"$0\"", "<dir>/z"]` with `Duration::ZERO` gives `Error` containing `zero`, and `<dir>/z` does not
     exist.
   - l. `non_utf8_output_still_matches`: `["printf", "\\377alpha\\376"]` (printf writes the bytes `0xFF`, `alpha`, `0xFE`),
     expect `["alpha"]`: `Ok`.
   - m. The existing `cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success` still passes.
9. **No shell, no broad kill, two spawns.** On the code lines of `crates/holler-pane/src/probe.rs` above its `#[cfg(test)]`
   line (comments excluded, since the rustdoc explains why there is no `libc`):
   `sed -n '1,/#\[cfg(test)\]/p' crates/holler-pane/src/probe.rs | grep -vE '^\s*//' | grep -nE 'pkill|killall|pgrep|pidof|"sh"|"bash"|libc|unsafe'`
   prints nothing, and the same pipeline with `grep -c 'Command::new'` prints `2`: the probe's program (`argv[0]`) and
   `"kill"`. What the shell tokens match: `"sh"` and `"bash"` are matched with their double quotes, so they hit only a
   string literal that is exactly `sh` or `bash` (the program name a shell spawn would pass), never a word that merely
   contains the letters (`"finished"`, `"shell"` and `"push"` do not match); no reason text has to avoid any word. The same
   pipeline also checks the shell-invocation shapes the two tokens miss:
   `sed -n '1,/#\[cfg(test)\]/p' crates/holler-pane/src/probe.rs | grep -vE '^\s*//' | grep -nE '"(/usr)?/bin/(sh|bash|zsh|dash)"|"(zsh|dash)"|"-c"'`
   prints nothing (an absolute shell path, another shell's name, or a `-c` argument). Together with AC 8h (`;`, `$(...)` and
   a space inside an element arrive literally) this is the "no shell" evidence; the greps guard the source, 8h the
   behaviour. The greps are a guard against the plain form only: they cannot see a shell name built at run time
   (`format!`, string concatenation, `include_str!`, a constant from another file). AC 8h's behavioural test is the
   evidence that no shell runs.
10. **Quality gates** (the tester overlay's Tier 1, as CI runs them): `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`,
    `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p holler-pane`, `cargo test -p holler-cli --lib`,
    `cargo test -p holler-pane-testkit`, `cargo test --workspace`, `cargo test -p holler-cli --test docs_cli_test` and
    `cargo machete` all pass. Formatting: `rustfmt --check --edition 2021 crates/holler-pane/src/probe.rs
    crates/holler-cli/src/pane/profile_scope.rs` exits 0 (ruling 4: `cargo fmt --check` fails on the untouched tree, H). Every
    `#[allow]` added carries a trailing `// #663`. Both files stay under 600 lines and every function under 100 lines.
11. **No new `unsafe`, no new dependency.** `git diff origin/main...HEAD -- crates | grep -E '^\+' | grep -vE '^\+\s*//' | grep -nE '\bunsafe\b'`
    prints nothing;
    `git diff --name-only origin/main...HEAD -- '*Cargo.toml' Cargo.lock` prints nothing.
12. **Blast radius.** `git diff --name-only origin/main...HEAD` lists only `crates/holler-cli/src/pane/profile_scope.rs`,
    `crates/holler-pane/src/probe.rs`, `CHANGELOG.md`, `docs/adr/ADR-0021.md` and the pipeline's own `docs/handoffs/663*`
    files.
13. **CHANGELOG.** `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry for the `--profile` helper and the
    probe runner, linking `[#663](https://github.com/Performant-Labs/holler/issues/663)`; it names no host or account.
14. **ADR-0021 is amended in place (the architecture review's B-1; prose only).** `docs/adr/ADR-0021.md` gets the five edits
    below. Each is a sentence or two, made in place in the paragraph or step named, and cited `(#663)`. None changes the
    closed code list, `class_of` or any table's classes. **Merge hygiene:** #644, #647 and #662 also edit ADR-0021 in their
    own changes, so edit sentences in place (add a sentence, or extend one), never rewrite a whole section or paragraph, so
    a rebase conflict stays one hunk. The current text of each place is quoted in C.
    - a. **Section 1, the `ProbeResult` paragraph (ADR lines 78-80):** the verdict rule of Decisions 14, 17, 18 and 19:
      `ok` only when the program exited 0 and every `expect` string occurs in its stdout (bytes, case-sensitive); a non-zero
      exit or an end by a signal is `error`, whatever the output; the deadline passing, or more than 1 MiB of stdout, is
      `error`; no reason echoes an argv element or an output byte; the argv is never run through a shell.
    - b. **Section 2, after "every method returns within I5's bound (default 10 s) or with `timeout`" (ADR line 87):** two
      narrowings. `ProfileScope::edit_spec` is bounded by the sum of its port calls (at most four) plus the verb's own act
      (Decision 11). `run_probe` returns at its deadline plus at most 1 s for the kill and the reap (Decision 15). The
      sentence cites F4, which amends the frozen `Prober` and `ProfileScope` trait docs to match.
    - c. **Section 8, step 2 (ADR lines 292-293):** a first write that times out may have landed, so the act does not run,
      and the answer is `timeout` with the reconcile step (Decision 6).
    - d. **Section 8, steps 5-6 (ADR lines 296-301):** a restoring write that fails other than by a conflict keeps its own
      code, and its message names P, the unrestored edit, the act's error and the reconcile step (Decision 5). The step-6
      `profile-conflict` carries the reconcile step in its message (Decision 7).
    - e. **Section 8, step 6 (ADR lines 299-301) and the record-fence bullet (ADR lines 273-275):** the reconcile step's
      exact text is `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`, with P
      POSIX-single-quoted; it is profile-scoped until #647 gives `pane doctor` a pane positional (Decision 8, C9); the
      scope's own errors carry the step, and a verb appends it only to errors that do not (the rule #644 plans against, its
      C-15, quoted in I); without `--profile` the step is `to reconcile, run holler pane doctor`, the
      `RECONCILE_STEP_UNSCOPED` const in `holler-cli/src/pane/profile_scope.rs`, beside `reconcile_step` (Decision 8).

    Checks: `git diff origin/main...HEAD -- docs/adr/ADR-0021.md` has its hunks only at the places a-e name;
    `git diff origin/main...HEAD -- docs/adr/ADR-0021.md | grep -E '^[-+]\|'` prints nothing (no table row changed) and
    `... | grep -E '^[-+]#'` prints nothing (no heading changed); each of the five edits is cited `(#663)`; the added text
    contains `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`,
    `RECONCILE_STEP_UNSCOPED` and `1 MiB`. **Alternative** (as #683's review allowed): if the operator lands a-e as a
    separate `docs(adr)` PR merged before this run, this AC instead checks that the merged ADR says a-e, and AC 12 drops
    `docs/adr/ADR-0021.md`.

## Files

Production:
- `crates/holler-cli/src/pane/profile_scope.rs` (fill; ~230 production lines, ~220 test lines in its `#[cfg(test)] mod
  tests`): module docs (the I8 order, the decisions below, the bound of Decision 11), `pub struct StoreScope`, `StoreScope::new`,
  `impl ProfileScope for StoreScope`, `pub fn reconcile_step`, `pub const RECONCILE_STEP_UNSCOPED`, and private helpers
  (the edit, the membership check, the message augmentation, the quoting).
- `crates/holler-pane/src/probe.rs` (replace the stub body and its docs; ~190 production lines, ~260 test lines):
  `run_probe` and private helpers (spawn, the stdout reader thread, the wait, the group kill, the match). `ProbeResult` is
  unchanged.
- `CHANGELOG.md` (one entry, AC 13).
- `docs/adr/ADR-0021.md` (prose only: five in-place edits in sections 1, 2 and 8, AC 14).

Tests: inline `#[cfg(test)]` modules in the two files above (the blast radius has no test file; Decision 21).

**Size:** ~420 production lines and ~480 test lines in two files, plus the CHANGELOG and five ADR sentences: one component
family per file, four files in all, well under F's cap. **Fits one run**, no split.

**Blast radius:** the issue's two files plus `CHANGELOG.md` and `docs/adr/ADR-0021.md` (prose only, AC 14). No manifest, no
`Cargo.lock`, no test kit, no other crate.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `holler_pane::ProfileScope` (`profile.rs:380`) | implemented as is; frozen | extend (implement) |
| `holler_pane::{ProfileStore, PaneStore}` | the scope's only I/O | reuse |
| `holler_pane_testkit::conformance::profile_scope::run_profile_scope_conformance` | AC 1, unchanged | reuse, no copy of its cases |
| `FakeProfileStore`, `FakePaneStore`, `FaultSwitch::fail_next`, `concurrent_put`, `fixture::{sample_profile, sample_pane, sample_spec}` | the extra scope tests (AC 2-7) | reuse (dev-dependency only) |
| `FakeProfileScope` (`holler-pane-testkit/src/profile_scope.rs`) | the reference behaviour `StoreScope` matches on every pinned point | **not imported**: production code cannot depend on the test kit (ADR-0021 section 5). `StoreScope` re-implements the same rules on purpose (the epic's real-and-fake pattern); this is not duplication to remove |
| the membership rule (`testkit pane_store.rs:225`, `hub panes/store.rs:344`) | the `pane-in-other-profile` pre-check of a `Set` | **a third private copy** (~15 lines): both existing copies are private to their crates and `holler-pane` (where a shared one belongs) is frozen and outside the blast radius. Same rule, same message shape, slugs compared. Follow-up F2 hoists one copy into `holler-pane` |
| `holler_pane::{Prober, SystemProber}` (`ports.rs:208-222`) | unchanged; they call the new `run_probe` | reuse |
| `holler_pane::Argv` | the probe's argv; `as_slice()` | reuse |
| a bounded subprocess runner | none exists on `main` (#641's `exec.rs` is on an unmerged branch) | **new, private to `probe.rs`**. #696 (open) later exposes one runner from `holler-pane` and switches the adapters to it; this story does not solve #696 and adds no public runner API (Decision 20) |
| `holler-cli/tests/support/mod.rs::make_own_process_group` | precedent only (a test helper; a library cannot call it) | n/a, the same `process_group(0)` call |
| `holler-cli/src/pane/wiring.rs` | **not touched** (#649 owns it) | n/a |

## Decisions already made (O)

**The scope (`StoreScope`).**

1. **Type.** `pub struct StoreScope { profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor }` and
   `pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self`: the fake's shape (G), so
   the suite's `build` closure and #649's wiring construct it the same way. `Send + Sync` follows from the two traits'
   supertraits. Every profile write is logged as `actor`; which actor a verb run uses is #649's call.
2. **`resolve`** is the fake's (G, lines 34-38): read P (`profile-not-found` first); with no pane, `PaneStore::list` filtered
   to the records whose `profile` has P's slug, in name order; with a pane, `PaneStore::get` of it, which must name P by slug,
   else `pane-not-in-profile` (a pane with no record included), its `what` `"<pane> is not in profile \"<P>\""`. Membership is
   `Pane.profile` alone; a spec of P adds no pane to the scope.
3. **`edit_spec(Some(P), pane, edit, act)`**, in ADR-0021 section 8's order:
   1. Plan: `ProfileStore::get(P)` (`profile-not-found` if absent, before anything else). For a `Set`, the spec must name
      `pane` (Decision 9), checked before any pane-store call, as the fake does. Then `PaneStore::get(pane)`, for every edit
      (Decision 10); for a `Set`, a record that names another profile (by slug) is `pane-in-other-profile`, `what`
      `"<pane> is in profile \"<Q>\", not \"<P>\""`. Compute the edited specs: a `Set`
      replaces the entry whose `pane` matches in place (same index) or appends one; a `Remove` drops it, keeping the others'
      order, and still writes when there is none (G, lines 273-293).
   2. First write: `cas_put(edited, stored.generation, actor)`, once (Decision 6 for its failures).
   3. `act()`, once.
   4. Act `Ok`: return `Ok(Some(written))`, the record the first write returned.
   5. Act `Err(failure)`: one restoring `cas_put(Profile { panes: stored.panes, ..written }, written.generation, actor)`,
      not retried. It succeeds: return `Err(failure)` unchanged (case 9). It conflicts: Decision 7. It fails otherwise:
      Decision 5.

   `edit_spec(None, ..)` is `act().map(|()| None)`: the act runs once and no store is called at all (case 12).
4. **The suite's ASSUMPTIONs, all confirmed** (G, `conformance/profile_scope.rs` lines 32-40 and 49-56): built over any two
   stores; the scope writes no pane record (recording is the verb's, inside its act); a `Set` replaces in place; the first
   write is not retried; a restore is one `cas_put` at g + 1, not retried; `profile-not-found` before `pane-in-other-profile`;
   no store call without a profile; a `Set` for a pane of another profile is refused before any write, with or without
   `--spec-only`, and a `Remove` is not.
   **Where `StoreScope` deliberately differs from `FakeProfileScope`, and why the suite still passes unchanged.** On three
   points the two differ in the message only, never in the code: the open point (the fake's `Err(other) => other`, G, versus
   Decision 5's extended message), the first-write timeout (the fake returns it as is through `?`, versus Decision 6), and
   the restore conflict (the fake's text, versus Decision 7's text plus the reconcile step). No suite case pins any of the
   three: case 9 arms no fault, so its restore succeeds and answers the act's error exactly; case 10's first write
   conflicts, it does not time out; case 11 checks only the code `profile-conflict` and that the message contains
   `Demo Alpha` (the suite's `ALPHA`), which Decision 7's longer text still does. The suite's open-point note asks that "the
   fake and this list are amended to match" (G, `conformance/profile_scope.rs` lines 45-47); this story does not make that
   amendment, because the test kit is outside the issue's blast radius (C5), so until F1 lands the fake and the suite's
   docs describe the old open point. AC 2-4 pin `StoreScope`'s side.
5. **The open point: a restoring write that fails with anything but a conflict** (`timeout`, `unavailable`, `store-corrupt`,
   ...). ADR section 8 decides only the conflict. **Decided: keep the restoring write's own code, and extend its message.**
   The code stays because it is the most recent failure and the one that says what is wrong now (a wedged or unreachable or
   corrupt store), and because each of these codes is already a failure (exit 1) in `class_of` (F). The message gains, after
   the store's own text: `; profile "<P>" still holds the edit of <pane>, but the live change failed (<failure>); <reconcile
   step>` (the name in prose is written with `{:?}`, as the fake writes it; only the reconcile step's command line is
   shell-quoted, Decision 8). So the error names the profile, the unrestored edit, the act's error (which would otherwise be lost) and the
   reconcile step. The payload extended is the variant's one string (`op` of `Timeout`, `what` of `Unavailable` and
   `StoreCorrupt` and the other `what` variants, `message` of `Usage`, `ProbeFailed`, `HerdrVersionUnsupported`, `ProfileDrift`
   and `Refused`); a variant with no payload (`NotImplemented`, `Conflict`, `CommandNotArgv`, `EnvNameInvalid`,
   `ProfileSecretRefused`) cannot carry it and is returned as it is. Why not `profile-conflict`: no other writer is involved,
   and the code would send the operator looking for one. Why not the act's error: the act's failure is already handled by
   design; the store's is the new fault. AC 2 pins it.
   **The append helper matches every `PaneError` variant exhaustively, with no `_` arm**, so a variant a later amendment
   adds fails to compile there instead of passing through unextended (the payload-less variants are listed by name). Its
   rustdoc says that `Timeout.op` carries this context **on purpose**, a stretch of `error.rs`'s convention that "`op` is
   the operation that timed out" (F, `error.rs:399-401`), because `Timeout` has no other payload and its code must stay.
   F2 hoists the helper into `holler-pane` as a `PaneError` method in `error.rs`, beside `code()` (F, `error.rs:494-497`).
6. **The first write's failures.** `generation-conflict` is returned as it is: nothing live has moved and the verb can be run
   again (ADR step 2; case 10). `timeout` gets the same message extension as Decision 5 but worded for an unknown outcome:
   `; the write may have landed, so profile "<P>" may hold the edit of <pane>, and nothing live was changed; <reconcile
   step>` (a timed-out write may still have been applied; ADR section 12 asks a timeout to print the reconcile step). Every
   other first-write error is returned unchanged (`unavailable` and the rest: the store said no). AC 4 pins both.
7. **The restore conflict** is `PaneError::ProfileConflict { what }` with `what` = `"<P>" was changed by another writer during
   the live change to <pane>, so its specs were not restored after that change failed (<failure>); the other writer's
   version stays; <reconcile step>`. This is the fake's text (G, lines 172-179) plus the reconcile step ADR step 6 asks for.
   AC 3 pins it; case 11 pins the code and the profile name.
8. **The reconcile step** is one function, `pub fn reconcile_step(profile: &ProfileName) -> String`, returning exactly
   `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`. It is `pub` because the
   spec-editing verbs (#644, #646) print the same step for a pane-record conflict (ADR step 4). **Beside it, the unscoped
   form**, for a run without `--profile`, with #644's exact text (I, `644-brief.md:1558`):
   `pub const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor";`. The two forms are the whole set,
   and both live in `profile_scope.rs`: #644 and #646 import this one const (`super::profile_scope::RECONCILE_STEP_UNSCOPED`)
   rather than declaring their own, so #644's planned `pub(crate) const` in `launch.rs` is dropped (F5). This is additive:
   `reconcile_step(&ProfileName)` keeps the signature #644 pastes. The profile name is
   POSIX-single-quoted (`'` becomes `'\''`, the whole wrapped in `'...'`), always, because a profile name may hold spaces,
   quotes, `$(...)` or backticks (D, lines 30-61) and an operator pastes this line into a shell. `ProfileName` refuses
   control characters, so the message stays one line (ADR section 9: every message is one line). The pane name is
   `[a-z0-9-]` and is not quoted. It names the doctor form that exists today (C9).
9. **A `Set` whose spec names another pane** is `usage` before any write ("a spec for the pane \"<spec.pane>\" cannot be set
   as the spec of <pane>"), as the fake does (G, lines 241-254). No case pins it; AC 6 does.
10. **The pane record is read for every edit, before the first write**, as ADR section 8 step 1 says ("Read P at
    generation `g` and the pane record") and as the fake does (G, lines 128-144), although only a `Set` uses it. Reason: a
    pane store that cannot be read fails the verb before P is written, for a `Remove` too (a `close` whose act and record
    would fail on that store anyway), so there is no needless edit-and-restore round trip in P's log. AC 7 pins it.
11. **Bounded time (C6), a narrowing.** The scope adds no timer and no thread: it cannot cancel a blocking port call, and
    the act is the verb's. Its bound is the sum of its port calls (at most four, each within I5's bound or `timeout`) plus
    the act's own. Recorded in the module docs.
12. **No retries, no pane-record write, no `catch_unwind`.** A panicking act unwinds through `edit_spec` and leaves the
    first write in place (the verb's process dies; the next `pane doctor` run sees the spec and the live pane disagree).
    Catching it would need `UnwindSafe` bounds the frozen `&mut dyn FnMut` does not have. Recorded as a risk.

**The probe runner (`run_probe`).**

13. **Process setup.** `argv[0]` is the program (looked up on `PATH` by `std::process::Command`, as any `Command::new` does)
    and the rest are its arguments, each passed as one element: no shell, no joining, no splitting. stdin is null, stdout is
    piped, stderr is null (discarded). The child is put in a process group of its own with
    `std::os::unix::process::CommandExt::process_group(0)` (safe API, Rust 1.64; the toolchain is 1.98), so its group id is
    its pid. Environment and working directory are inherited from the calling process (a probe like `curl` needs `PATH`;
    `run_probe` has no cwd parameter). Unix only, as the rest of the workspace (`holler-hub` uses `std::os::unix`
    unconditionally).
14. **Verdict, in order.** Empty argv: `Error`, no process. Zero timeout: `Error`, no process. Spawn fails: `Error`. Output
    over the cap: `Error` (group killed). Deadline passes: `Error` (group killed). Exit not success (a non-zero code, or ended
    by a signal): `Error`, whatever the output, because a probe whose program failed has no verdict and a probe with no
    `expect` must not pass on `false` (the reading of the frozen docs this rests on, and why it changes no launch outcome
    against `Failed`, is C10). Exit 0: `Ok` when every `expect` string is in the output, else `Failed { missing }`.
15. **Timeout semantics (C7).** One deadline, `start + timeout`, taken before the spawn. The runner first waits, up to the
    deadline, for the reader thread to report end of stdout (`mpsc::Receiver::recv_timeout`), and only then polls
    `Child::try_wait` every 10 ms until the exit or the deadline. When the deadline passes (or the cap is hit) it kills the
    group **before reaping the leader**: until `try_wait` has returned `Some`, the leader is alive or a zombie, so its pid,
    which is the group id, cannot have been reused by another process. Once `try_wait` has returned `Some`, the runner never
    signals. Then `Child::kill` (the leader, in case the group kill could not run) and the leader is reaped by polling
    `try_wait`. The `kill` call and the reap share **one cleanup budget of 1 s**; a leader still unreaped after it (a process
    stuck in the kernel) is left as a zombie for the caller's exit, and the runner returns. Consequences, all
    documented: the answer comes at the deadline plus at most 1 s of cleanup (normally a few ms); a
    probe whose background child keeps stdout open after the leader exits is a timeout, never a verdict (AC 8g); the whole
    group is killed (AC 8f). Signal: `KILL` directly. A probe is a read-only check with nothing to clean up, so there is no
    `TERM` grace.
16. **The group kill** runs the `kill` binary from `PATH`: `kill -s KILL -- -<pid>` (stdin, stdout and stderr null), waited
    for with `try_wait` within Decision 15's one shared 1 s cleanup budget (the same budget the leader's reap draws on, not
    a second 1 s), and killed and reaped itself if that budget runs out. Its outcome is ignored (the group may already be
    gone). Why a binary: killing a group needs `kill(2)` with a negative pid, which `std` does not offer; `libc` would need
    `unsafe` (forbidden) and a new dependency of `holler-pane` (not allowed). The target is only ever the group of the child
    this call spawned (Decision 15): no name matching, no other pid.
17. **Output cap: 1 MiB of stdout** (`const MAX_OUTPUT: usize = 1 << 20`). The reader thread reads into a buffer; when the
    output would pass 1 MiB it stops reading and reports `Overflow`, and the runner kills the group and answers `Error`. So
    memory is bounded and a runaway writer (`yes`) ends at once instead of at the deadline (AC 8i). stderr is never read or
    kept.
18. **Matching.** On bytes: an `expect` string matches when its UTF-8 bytes occur anywhere in stdout (case-sensitive, no
    trimming), so output that is not UTF-8 still matches (AC 8l). `missing` lists the `expect` strings not found, in their
    `expect` order, duplicates kept as given. An empty `expect` string is always found.
19. **Reasons never echo an argv element or an output byte.** The `Error` reasons are fixed texts: `the probe argv is empty:
    there is no program to run`; `the probe timeout is zero`; `the probe program could not be started: <io::ErrorKind>`;
    `the probe wrote more than 1 MiB to stdout`; `the probe timed out after <n> ms`; `the probe exited with status <n>`; `the
    probe was ended by a signal`; `the probe's output could not be read`; `the probe could not start its output reader`. Why:
    a check argv is a stored command and may carry a token (a `curl -H` header); a probe's output may carry anything; and
    the result is persisted in `Pane.probe.last` on the hub and shown by verbs. `Failed.missing` holds only `expect`
    strings, which are spec values and non-secret by I7. The spawn error is reported by its `ErrorKind` only, never by
    `io::Error`'s text. AC 8j pins it.
20. **The runner stays private to `probe.rs`.** No new public item in `holler-pane` (the crate is frozen; ruling 3 and the
    amend-first rule). #696 (open) is where one bounded runner gets exposed and the adapters switch to it; this story does
    not pre-empt its design. Private helpers are split so that no function exceeds clippy's 100-line or complexity-15 gates.
    **The process mechanics are kept apart from the verdict**, so #696 can lift the runner without a rewrite (I, #696's "No
    behaviour change"):
    - the mechanics (spawn in its own group, the bounded and capped read, the deadline, the group kill, the reap) return a
      private outcome type: exit status and stdout bytes, timed out, overflow, spawn error by `io::ErrorKind`, or read error;
    - a separate private function maps that outcome to `ProbeResult` (the `expect` match of Decision 18 and the fixed
      reasons of Decision 19);
    - the rustdoc lists the three points #696 must parameterize, open on purpose here: **stderr** (discarded here; #696's body asks for "captured
      stderr"), **the signal and its grace** (`KILL` with no `TERM` grace here; #641's planned runner sends `TERM`,
      H), and **the kill program** (`kill` from `PATH` here, with no fake-`kill` seam, which #696 must keep for the
      adapters).
    No public item is added, so the first sentence of this decision holds.
21. **Tests are inline** `#[cfg(test)] mod tests` in the two files, since the blast radius has no test file, each module
    opening with `#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #663`. The probe tests make their
    scratch directory with `std::fs::create_dir` under `std::env::temp_dir()` (named `hlr-probe-663-<pid>-<counter>`) and
    remove it in a guard's `Drop`; `holler-pane` has no `tempfile` and gets none. They signal nothing themselves (AC 8f only
    reads `ps`), and every process they start is a child of the runner they test.

**Process.**

22. **ADR-0021 is amended in this change (the new AC); the test kit is not (F1).** Decisions 5, 6, 7, 8, 11, 14 (C10) and
    15 are recorded in the files' rustdoc, in `docs/handoffs/663/decisions.md` and in ADR-0021 (AC 14); the run files the
    follow-ups below.
23. **Commit and PR.** Conventional Commit subject (`feat(pane): ...`), the AI-assistance disclosure `CONTRIBUTING.md` asks
    for in the PR body, `Closes #663`.

Forward-compat (consumers):

| Consumer | Needs | Satisfied |
|---|---|---|
| #643 read verbs, #646 park/unpark, #647 doctor | `resolve(P, None)` = every member, `resolve(P, Some(n))` with the membership refusal | yes (2) |
| #645 switch/reset, #646 say/interrupt/answer | `resolve(P, Some(n))` to check membership only | yes (2) |
| #644 launch/relaunch | `edit_spec(P?, pane, Set, act)`, `--spec-only` as a no-op act, `profile-not-found` in one line, the reconcile step text for a pane-record conflict, scoped and unscoped | yes (3, 8; C3); imports `RECONCILE_STEP_UNSCOPED` from here (F5) |
| #646 close | `edit_spec(P?, pane, Remove, act)`; a detached spec is removable; a pane store that is down fails before P is written; the reconcile step, scoped and unscoped | yes (3, 8, 10) |
| #644, #664 apply | `Prober::run_probe` through `SystemProber` with a bounded, non-shell, non-echoing runner | yes (13-19) |
| #649 wiring | `StoreScope::new(Arc<dyn ProfileStore>, Arc<dyn PaneStore>, Actor)` with no I/O in the constructor; `SystemProber` unchanged | yes (1) |
| #696 one runner | a runner whose behaviour is pinned by tests, so it can be moved without a behaviour change | yes (AC 8; 20: mechanics apart from the verdict, its three open points in the rustdoc) |

## Out of scope

- Any verb (#643-#647), the wiring (#649), the hub (#661 is merged), the test kit (#638), ADR-0021 beyond AC 14's five
  edits, `holler-pane`'s other files (`lib.rs`, `ports.rs`, `error.rs`, `profile.rs`), any manifest.
- A public bounded-runner API and switching the adapters to it (#696).
- A probe working directory or environment of its own (the frozen signature has neither).
- Retrying a conflicting write; claiming a pane before acting (ADR section 8: generations fence records, not live acts).
- Touching any live fleet, OpenCode server, Herdr server or tmux session; every test is in-process or a harmless local
  child process.

## Follow-ups (not this story; the run files them as issues)

- **F1.** Test kit: amend `FakeProfileScope` and the suite's docs to Decisions 5, 6 and 7 (the open point decided, the
  first-write timeout text, the reconcile step in the conflict message), and pin them with cases. The test kit must not
  depend on `holler-cli` (C, ADR lines 184-186), so the fake cannot call `reconcile_step` where it is now. F1 takes one of
  two routes, named when it is filed:
  - **(a)** F2's `holler-pane` hoist takes `reconcile_step` (and `RECONCILE_STEP_UNSCOPED`) too, since it is pure formatting
    over a `ProfileName`, and the fake calls it, so the fake's messages match `StoreScope`'s exactly;
  - **(b)** F1 aligns only the fake's codes and its open-point behaviour, and states that verb stories pin the step text
    against the real `StoreScope` (as #644 does, its C-15 and AC 16k, I).

  A copy of the formatter and its quoting in the test kit is not the default: it would be a second duplicate in this epic,
  after the membership rule.
- **F2.** Hoist into `holler-pane` (amend-first; `holler-pane/**` is #637's): the membership rule, as one public function
  used by the hub, the test kit and `StoreScope` (three private copies after this story); and Decision 5's payload-append
  helper, as a `PaneError` method in `error.rs` beside `code()`. Under F1's route (a), `reconcile_step` and
  `RECONCILE_STEP_UNSCOPED` too.
- **F3.** (Withdrawn: its ADR-0021 items, Decisions 5, 6, 8, 11, 14 and 15, are now AC 14, in this change.)
- **F4.** `holler-pane`, under the amend-first rule: the `Cargo.toml` description and `lib.rs` docs drop "no I/O" (C8);
  rename `ports_test.rs`'s `run_probe_stub_never_reports_success` (the stub is gone; the test still holds); and amend the two
  frozen trait docs that contradict the shipped code: `Prober`'s "It returns within the `timeout` it is given"
  (`ports.rs:204-207`, E; Decision 15 adds up to 1 s of cleanup) and `ProfileScope`'s "Every method returns within I5's
  bound (default 10 s) or with [`PaneError::Timeout`]" (`profile.rs:377-379`, D; Decision 11). AC 14b's sentence cites F4.
- **F5 (cross-story, before #644 runs).** Amend #644's brief (`issue-644-implementation`, `7195993`) to import
  `super::profile_scope::RECONCILE_STEP_UNSCOPED` (Decision 8) instead of declaring `pub(crate) const RECONCILE_STEP_UNSCOPED`
  in `launch.rs` (its lines 1554 and 1557-1558, and its references at 1707, 1954, 2031 and 2157); #646's brief imports the
  same const. The operator amends #644's brief before its run; this story does not edit it.
- #696 is unchanged and stays open.

## Test plan

**RED** (a compile error is not RED, `docs/agent-overlays/tester.md`). T first lands, in `profile_scope.rs`, a stub with the
exact public items of Decisions 1 and 8: `StoreScope` with its three fields and `new`, an `impl ProfileScope` whose two
methods return `Err(PaneError::NotImplemented)`, `reconcile_step` returning `String::new()`, and `RECONCILE_STEP_UNSCOPED`
as `""`. `probe.rs` already holds a
stub (always `Error`). T then writes the tests, and the RED run shows:
- AC 1 fails on `assert_eq!(result, Ok(()))`: 15 case failures, each `not-implemented`.
- AC 2, 3, 4, 6 fail on the asserted code or message (`not-implemented` answered); AC 5 fails on the string equality; AC 7
  fails on the asserted code (`not-implemented`, not `unavailable`).
- AC 8a, 8b, 8c, 8h and 8l fail on their `assert_eq!` (expected `Ok` or `Failed`, got the stub's `Error`); 8d, 8i and 8k fail
  on the reason substring; 8e fails on the elapsed lower bound (the stub returns at once); 8f and 8g fail on
  `assert!(pid_file.exists())`.
- AC 8j fails on its positive assertions (`could not be started`, and `Failed { missing: ["absent"] }`); its absence
  assertions hold on the stub too, by design.
- Green on the stub, by design (a regression guard): AC 8m only.
T journals the RED run with each failure.

**GREEN:** `cargo test -p holler-cli --lib pane::profile_scope`, `cargo test -p holler-pane --lib probe`, then AC 9-12 and 14.
The timing tests assert lower bounds exactly and upper bounds with 2 s of slack (this repo's macOS runner has timing
flakes; the epic handoff notes them), and each probe test runs its own child, so they run in parallel safely. T-green also
checks that no `hlr-probe-663-*` directory is left under the temp dir and that no `sleep 30` started by the tests is still
running (`ps -o pid=,args= -u "$(id -u)"`, read-only).

## Risks

- **A background child that leaves the group** (`setsid`, or a double fork into a new group) escapes the group kill and
  may keep stdout open. The runner still returns at the deadline (it never joins the reader thread after a kill), but that
  thread stays blocked until the escapee closes the pipe: a leaked thread in a long-lived caller. Accepted: the callers are
  short-lived CLI verbs; documented in the rustdoc. The `Prober` docs allow an async caller to run the method from
  `spawn_blocking` or a thread (E, `ports.rs:204-205`), so a long-lived process that reuses `SystemProber` and repeatedly
  runs a probe that escapes its group would leak one blocked reader thread (and its pipe) per such run, without bound. No
  long-lived caller exists today (the hub runs no adapters, ruling 1); the rustdoc states the leak so that #696, which
  exposes the runner for wider use, inherits the warning.
- **The fake and the real scope differ in three messages until F1** (Decision 4's note: Decisions 5, 6 and 7). The codes
  are the same on all three paths, so a verb that matches on `code()` behaves alike over either; a verb test written
  against `FakeProfileScope` sees the fake's shorter texts, without the reconcile step, on those paths. Accepted for this
  story because the test kit is outside its blast radius (C5); F1 closes the window by route (a), or under route (b) verb
  stories pin the step text against `StoreScope`.
- **The `kill` binary missing from `PATH`.** The group kill is skipped, `Child::kill` still ends the leader, and its children
  are left. Linux (procps or util-linux) and macOS both ship `/bin/kill`. Accepted and documented.
- **Pid reuse** is closed by Decision 15's order (signal only before the leader is reaped). A refactor that calls `try_wait`
  before the end of stdout, then signals, reopens it; the module docs say so at the code.
- **A shell slipping in.** `Command::new(argv[0]).args(&argv[1..])` never invokes a shell; AC 8h proves `;`, `$(...)` and a
  space inside an element arrive literally, and AC 9 greps the source.
- **Secrets in a probe.** Covered by Decision 19 and AC 8j: no argv element or output byte in any reason.
- **Three copies of the membership rule** (Reuse map) can drift; the message shape and slug comparison are copied exactly,
  case 14 pins the code, and F2 removes the copies.
- **A panicking act** leaves the first write in place (Decision 12); `pane doctor` is the net.
- **Timing flakes on macOS CI** (AC 8e-8g): upper bounds carry 2 s of slack, lower bounds are exact; a rerun of the macOS job
  once is the epic's standing practice.
