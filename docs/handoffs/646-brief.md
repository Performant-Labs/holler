# Brief: #646 part 1 of 3 (646a): `holler pane park` and `holler pane unpark`

Repo: Performant-Labs/holler. Issue: #646 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**Branch:** `issue-646-implementation` (worktree `.claude/worktrees/0646-park-close-routing`, from `origin/main` at `ce12cdb`).
**Review rigor:** second-opinion (the orchestrator's instruction for this run; the issue's Pipeline line says `in-session`,
see "Contradictions found", C-1). The outside reviewer sees only this brief, so every fact below about existing code is
pasted from the source with its file and line, as of `ce12cdb`.
**Design (D):** N/A (no UI surface). **Forward-compat:** done, see the table after "Decisions".
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) sections 1, 3, 8 and 9, and the epic's "Skeleton split" rulings.
This part **extends ADR-0021 section 3** with how park and unpark behave, and edits it in this change (Decision 12).
**Handoffs:** `docs/handoffs/646/handoff-<phase>.md`; the decision journal is `docs/handoffs/646/decisions.md`.
**Public repository:** no personal host, tailnet or account name goes into code, tests, docs, the CHANGELOG, commit
messages or the PR. Test data uses the test kit's neutral names (`demo-c1r1`, `Demo Alpha`, `scratch`, `localhost`).
**PR wording:** this part is "Part of #646"; it does not close the issue (part 3 closes it).

## Problem

A pane record carries a `hold` (`none`, `parked {reason, release_when, since}` or `drained`), and watchers are meant to
skip a parked pane, but nothing can set it: `holler pane park` and `holler pane unpark` are stubs that answer
`not-implemented`. This part makes them real record changes, scoped by `--profile` the way ADR-0021 section 3 says, with
the `--format=json` envelope and the closed exit codes.

## The split, and why this brief covers part 1 only

Issue #646 holds three pieces of work with different dependencies and different risk. This brief is for **646a** only.

| Part | Scope | Can start | Why apart |
|---|---|---|---|
| **646a (this brief)** | `pane park`, `pane unpark` | now: needs #637, #638, #670, #676, all merged | record-only, no adapter call, no unmerged code |
| 646b | `pane close` (with `--profile`, `--spec-only`, `PaneStore::delete`, `stop_owned`, `HerdrPort::close`) | after **#663** merges | imports #663's reconcile step (E-9), which is not on `main` |
| 646c | `say`/`interrupt`/`answer` `--pane`/`--profile` routing, the pane-state prompt gate | after an architecture decision (E-10) | the address of a pane's driven session is deferred by ADR-0021 to #649/#654; the hub's enforcement point sits in files at the 900-line cap; `answer` does not pass through `send_prompt` |

Size is the third reason: the two most recent verb briefs in this epic that took a whole story are 2,028 lines (#663) and
2,209 lines (#644); #646 whole would be larger, past the length at which the outside review stays reliable. 646b and 646c
get their own briefs. Nothing in 646a depends on how they are decided, and neither depends on 646a's code except the
`park`/`unpark` names (Forward-compat).

## Dependencies

Merged on `origin/main` (`ce12cdb`): #637 (the `holler-pane` crate, `Hold`, `PaneStore`, `ProfileScope`), #638 (the test
kit: `FakePaneStore`, `FakeProfileStore`, `FakeProfileScope`, `check_envelope`), #670 (the CLI skeleton, `output.rs`,
the stubs), #676 (refusals exit 3). #647 (doctor, the analogous merged verb) is on `main` too. Not needed by 646a: #663
(the real `ProfileScope`; 646a calls only the trait and tests over the fake), #649 (the real wiring), #643/#644/#645.

## Evidence (verbatim, as of `ce12cdb`)

### E-1. The stubs this part replaces

```
crates/holler-cli/src/pane/park.rs:1-23
//! `holler pane park`: a stub (story #670). Story #646 owns the real verb: it replaces this
//! file and adds the verb's own positionals and flags to the arguments below, edits its own
//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.

use clap::Args;

use super::args::ProfileOpt;
use crate::output::{emit_error, not_implemented, VerbCtx};

/// The story that owns this verb.
const STORY: u32 = 646;

/// Take a pane out of service until it is unparked.
#[derive(Args, Debug)]
pub struct PanePark {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane park`: refuse, naming the story that owns it.
pub fn run(_args: &PanePark, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
`crates/holler-cli/src/pane/unpark.rs` has the same lines 1-12 with `unpark` for `park`, then:
```
crates/holler-cli/src/pane/unpark.rs:13-23
/// Return a parked pane to service.
#[derive(Args, Debug)]
pub struct PaneUnpark {
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane unpark`: refuse, naming the story that owns it.
pub fn run(_args: &PaneUnpark, ctx: &mut VerbCtx<'_>) -> i32 {
    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
}
```
```
crates/holler-cli/tests/pane_verbs/park.rs:1-9
//! `holler pane park`: the stub case of story #670. Story #646 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn pane_park_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "park"], 646);
}
```
`crates/holler-cli/tests/pane_verbs/unpark.rs:1-9` is the same with `unpark`.

The frozen dispatch (not edited; it already routes both verbs to these files):
```
crates/holler-cli/src/pane/mod.rs:21,26,44-45,61-62
pub mod park;
pub mod unpark;
    Park(park::PanePark),
    Unpark(unpark::PaneUnpark),
        PaneCmd::Park(args) => park::run(args, ctx),
        PaneCmd::Unpark(args) => unpark::run(args, ctx),
```
The shared flag group (frozen, #670; reused, not edited):
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

### E-2. The record field this part writes, and the store it writes through (frozen, #637)

```
crates/holler-pane/src/pane.rs:160-177
/// A pane's hold state **as a field of the pane record**: whether `park`/`unpark`
/// has parked it, or it is drained. It is **not** the prompt hold of `holler hold`
/// (`holler_proto::SessionHold`). Any refusal of a prompt that is derived from pane
/// state belongs at `send_prompt`, the one choke point every prompt passes through,
/// not in a verb (#646's brief states this).
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
```
```
crates/holler-pane/src/pane.rs:223-230,237-239
/// One record per pane, owned by the hub. The key is [`Pane::name`]. Every write is
/// a compare-and-swap on [`Pane::generation`] (see [`crate::generation`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub name: PaneName,
    /// Bumped on every change.
    pub generation: u64,
    pub role: PaneRole,
    pub hold: Hold,
    pub last_observed: LastObserved,
```
```
crates/holler-pane/src/ports.rs:62-71
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;

    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
```
Nothing on `main` sets `Hold::Drained` (`grep -rn Drained crates --include=*.rs` finds only `pane.rs`).

### E-3. The scope helper this part calls (frozen trait, #637; real implementation #663, fake #638)

```
crates/holler-pane/src/profile.rs:268-273
/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}
```
```
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
crates/holler-pane-testkit/src/profile_scope.rs:34-36
/// - `resolve(P, None)` is the stored P and every pane whose record names P, in name
///   order. `resolve(P, Some(n))` is P and n, whose record must name P, or else
///   `pane-not-in-profile` (a pane with no record included). A missing P is
```

### E-4. The ADR text this part implements, and the row it keeps

```
docs/adr/ADR-0021.md:126-130
- **Scoping** (`list`, `get`, `watch`, `doctor`, `switch`, `reset`, `park`, `unpark`, the `--pane` forms of `say`, `interrupt`
  and `answer`, and the roster): act on P's panes. A named pane outside P is `pane-not-in-profile`. The read verbs and
  `doctor`, `park` and `unpark` with no pane name mean every pane of P; `say`, `interrupt`, `answer`, `switch` and `reset`
  still need a pane name.
- **Neither**: `pane import` takes no `--profile`; a real run creates the profile `fleet` itself (#650).
```
```
docs/adr/ADR-0021.md:332-333,341
**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
| `pane park`, `pane unpark` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
```
Exit classes this part relies on (ADR-0021 section 9's table, decided once in `holler_pane::error::class_of`):
```
docs/adr/ADR-0021.md:379,387,391-392,394,397-398
  | `usage` | Usage (2) | The request is malformed; ADR 0003's exit 2. |
  | `pane-not-in-profile` | Refusal (3) | The `--profile` scope check declined a pane outside the profile. |
  | `profile-not-found` | Refusal (3) | The named profile does not exist; the request was understood and declined. |
  | `pane-not-found` | Refusal (3) | The named pane does not exist. |
  | `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply). |
  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
```
The generation fence (ADR-0021 section 8):
```
docs/adr/ADR-0021.md:272-273
- A write whose expected generation is not the current one is `generation-conflict` and changes nothing. `delete` checks
  that the record exists first (`pane-not-found` or `profile-not-found`, whatever the generation) and the generation second.
```

### E-5. The output module (#670/#676; called, not edited)

```
crates/holler-cli/src/output.rs:125-140
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
```
```
crates/holler-cli/src/output.rs:195-212
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
The error texts this part's messages embed:
```
crates/holler-pane/src/error.rs:651-653,674
            PaneError::Conflict => f.write_str(
                "the record changed since it was read (generation conflict); read it again and retry",
            ),
            PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),
```

### E-6. The analogous merged verb: `pane doctor` (#647), the shape to copy

```
crates/holler-cli/src/pane/doctor.rs:40-62
/// Run `holler pane doctor`: one reconcile pass, printed in the run's format.
pub fn run(args: &PaneDoctor, ctx: &mut VerbCtx<'_>) -> i32 {
    let report = pass(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, report, render)
}

/// Type the arguments (a bad pane or profile name is `usage`) and run the pass now.
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
Doctor's `PANE` positional:
```
crates/holler-cli/src/pane/doctor.rs:30-32
    /// Check only this pane (default: every pane in scope).
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
``` Doctor quotes
every untrusted value it prints with this helper, which this part reuses:
```
crates/holler-pane/src/findings.rs:328-334
/// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
/// control character and quote in it is escaped, and cut to 64 characters
/// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
/// included.
pub fn quoted(text: &str) -> String {
    excerpt(text)
}
```
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
Doctor's test rig is private to its module (`crates/holler-cli/tests/pane_verbs/doctor.rs:10`: `mod rig;`) and builds a
whole live world on every fake, so park cannot reuse it without editing `doctor.rs` (outside this blast radius).

### E-7. The fakes and the test harness (#638, #670; used, not edited)

```
crates/holler-pane-testkit/src/pane_store.rs:90-99,107-111
    /// A store holding `panes`, each created at expected generation 0 and so stored
    /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
    /// name are `generation-conflict`, as a second create would be.
    pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
        let store = Self::new();
        for pane in panes {
            store.put(&pane, Writer::Port(0))?;
        }
        Ok(store)
    }
    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
        &self.faults
    }
```
```
crates/holler-pane-testkit/src/fault.rs:72-77,85-88
    /// Fail the next call of `op` with `error`, once. The errors queued for one method
    /// come out in the order they were queued, and a call of another method leaves them
    /// queued. A standing fault answers first, also leaving them queued.
    pub fn fail_next(&self, op: Op, error: PaneError) {
        self.lock().queued.push((op, error));
    }
    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
        self.lock().calls.clone()
    }
```
```
crates/holler-pane-testkit/src/profile_scope.rs:70
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
`sample_profile(name: &str, panes: &[&str]) -> Result<Profile, PaneError>` is at `fixture.rs:121`;
`FakeProfileStore::seeded(profiles, actor: &Actor)` at `profile_store.rs:127-130`.
```
crates/holler-pane-testkit/src/envelope.rs:197-202
/// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
/// code of the verb that wrote it.
///
/// Returns the envelope, or the fault of the first rule it breaks, in the order the
/// module doc lists.
pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
```
```
crates/holler-cli/tests/verb_harness/mod.rs:52-57
/// Run `holler <argv...>` in-process through `pane::run` or `profile::run`, with the
/// given output format and ports.
pub fn run_verb_with(argv: &[&str], format: Format, ports: Ports<'_>) -> Outcome {
    let mut full = vec!["holler"];
    full.extend_from_slice(argv);
    let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{full:?} must parse: {e}"));
```
`FakeHerdr::new(session)`, `FakeHost::new()`, `FakeHarness::new()` each expose `faults()` with the same `calls()` log
(`herdr.rs:174,211`, `host.rs:97,106`, `harness.rs:170,179`); `FakeProber::calls()` is at `prober.rs:64`.

### E-8. The surface files this part edits (its own rows only)

```
docs/adr/ADR-0003.md:54-55
holler pane park [--profile NAME]                                 #646
holler pane unpark [--profile NAME]                               #646
docs/adr/ADR-0003.md:92 (excerpt)
... each owning story adds its verb's positionals and flags to its own row, with its own line in `cli-surface.txt`, and edits no other verb's. ...
```
```
crates/holler-cli/tests/fixtures/cli-surface.txt:131-135
# #646
pane park |
pane park | --profile demo
pane unpark |
pane unpark | --profile demo
```
The fixture's rule (`cli-surface.txt:4`): "Every line must parse with `Cli::try_parse_from`". `docs_cli_test` parses every
`holler ...` row in the docs after dropping `[optional]` groups (`tests/docs_cli_test.rs:14`, "Placeholders are normalised
(`<x>` → `x`, `[optional]` dropped, `a|b` → ..."); `docs/handoffs/` is excluded from that scan
(`docs_cli_test.rs:219-220`).
```
crates/holler-cli/tests/pane_verbs/process/stub.rs:28-31
    // #646
    ("pane", "park", 646),
    ("pane", "unpark", 646),
    ("pane", "close", 646),
```
The rule for that table (`stub.rs:12-16`): "A verb story deletes its own entries when its verb stops being a stub and
**keeps its `// #NNN` line**". The process flag tests accept a missing required argument:
```
crates/holler-cli/tests/pane_verbs/process/parse.rs:28-32
/// `Ok` when `holler <argv...>` is accepted: it parses, or only a required positional is missing.
pub fn accepted(argv: &[&str]) -> Result<(), String> {
    match try_parse(argv) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::MissingRequiredArgument => Ok(()),
```
used by `flags.rs:22-27` (`accepted(&["pane", verb])` and `accepted(&["pane", verb, "--profile", "demo"])` for every
pane verb but `import`).

### E-9. Why `close` waits for #663 (646b; quoted so the split can be checked)

`main`'s `profile_scope.rs` is a doc-only stub:
```
crates/holler-cli/src/pane/profile_scope.rs:1-5
//! The real `ProfileScope` (epic #633): the helper every `--profile` verb uses to scope itself
//! to a profile and to edit a spec in one transaction with the live change.
//!
//! Empty in the CLI skeleton (story #670). Story #663 fills it; the trait is
//! `holler_pane::ProfileScope`, frozen by #637.
```
#663's brief (unmerged branch `issue-663-implementation`, commit `93fb653`) puts the reconcile step that `close` must
print there:
```
docs/handoffs/663-brief.md:1804-1809 (at 93fb653)
8. **The reconcile step** is one function, `pub fn reconcile_step(profile: &ProfileName) -> String`, returning exactly
   `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`. It is `pub` because the
   spec-editing verbs (#644, #646) print the same step for a pane-record conflict (ADR step 4). **Beside it, the unscoped
   form**, for a run without `--profile`, with #644's exact text (I, `644-brief.md:1558`):
   `pub const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor";`. The two forms are the whole set,
   and both live in `profile_scope.rs`: #644 and #646 import this one const (`super::profile_scope::RECONCILE_STEP_UNSCOPED`)
```

### E-10. Why the routed prompt verbs need their own design pass (646c; quoted so the split can be checked)

```
docs/adr/ADR-0021.md:453-456
**Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
registered pane, and when it moves after a switch or reset.
Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub
drives, and leaves `last_observed.driven` as stored.
```
The hub's prompt choke point is reached from a file one line under the 900-line build guard (`wc -l`: `circuit.rs` 898,
`live.rs` 867; `scripts/lint.sh:46`: `if [ "$n" -ge 900 ]; then`):
```
crates/holler-hub/src/circuit.rs:737-740
            LiveCommand::Say { request_id, session, message, queue, replace, grant, reply } => {
                let key = crate::holds::session_key(self.label, &session);
                let gate = HoldGate { holds: self.registry.holds(), key: &key, grant: grant.as_deref() };
                match dispatch::send_prompt(self.sink, gate, &request_id, &session, message, queue, replace).await {
```
`answer` never reaches `send_prompt`; its handler goes to `talk::answer`:
```
crates/holler-hub/src/control_server.rs:428
    match crate::talk::answer(registry, session, choice, timeout).await {
```

## Contradictions found

- **C-1. Rigor.** The issue says `rigor: in-session`; this run is `second-opinion`, set by the orchestrator. The higher
  rigor wins; nothing in the design depends on it.
- **C-2. One issue, three parts.** The issue's blast radius lists close, the prompt verbs and the hub gate with park and
  unpark. This part edits only park and unpark (Files). The issue's acceptance bullets that 646a meets are "Park and unpark
  round-trip", "`park --profile P` parks every pane of P", "JSON passes the envelope helper; exit codes equal across
  formats"; the others (close, `say --pane`, `say --queue`, `close --spec-only`) are 646b's and 646c's.
- **C-3. "A parked pane is skipped by watchers" (issue scope).** The watchers are outside this blast radius: pfleet's
  watchdog (#653), the roster (#648), and any prompt gate (646c). This part records the hold; each consumer reads `hold`
  (`pane get`/`list`, #643). Whether a parked pane refuses a prompt is decided in 646c, not here (Decision 10).
- **C-4. `Hold::Drained` has no writer.** No verb or story on `main` sets it (E-2). Park and unpark leave a drained pane
  as it is (Decision 4); a later story that defines draining decides how it meets park.

## Acceptance criteria

All commands run from the worktree root. Every in-process test below runs `run_verb_with` (E-7) over a **rig** of fakes
(Decision 11): a `FakePaneStore`, a `FakeProfileStore`, a `FakeProfileScope` over those two, and `FakeHerdr`,
`FakeHost`, `FakeHarness` and `FakeProber`, whose call logs every test asserts are empty at its end (AC 9). The seeded
panes are `sample_pane` records (stored at generation 1); `R` = `"disk full"`, `W` = `"after the cleanup"`.

1. **Park, then unpark, one pane (text).** Test `park_then_unpark_round_trips_one_pane`. Seed `demo-c1r1`. Take
   `t0 = now_millis()`, run `pane park demo-c1r1 --reason "disk full" --release-when "after the cleanup"`, take `t1`.
   Exit 0, `err` empty, `out` is exactly `demo-c1r1: parked (reason "disk full", release when "after the cleanup")\n`. The
   stored record has `hold == Parked { reason: R, release_when: W, since }` with `t0 <= since <= t1`, generation 2, and
   equals the seeded record in every other field. Then `pane unpark demo-c1r1`: exit 0, `out` is exactly
   `demo-c1r1: unparked\n`, the record's `hold` is `Hold::None`, generation 3, every other field equal to the seed.
2. **The same in JSON.** Test `park_and_unpark_json_pass_the_envelope_helper`. The same two runs with `Format::Json`:
   `err` empty; `check_envelope(&out, 0)` is `Ok`; park's `data` is
   `{"panes":[{"name":"demo-c1r1","changed":true,"generation":2,"hold":{"parked":{"reason":"disk full","release_when":"after the cleanup","since":<since>}}}]}`
   (`since` the stored value) and unpark's is `{"panes":[{"name":"demo-c1r1","changed":true,"generation":3,"hold":"none"}]}`.
3. **Idempotent, nothing written.** Test `park_and_unpark_leave_a_pane_already_in_that_state`. (a) Seed `demo-c1r1`
   already parked (`reason "old"`, `release_when "later"`, `since 5`): `pane park demo-c1r1 --reason R --release-when W`
   exits 0; `out` is `demo-c1r1: already parked (reason "old", release when "later")\n`; the record is unchanged
   (generation 1, reason `old`, `since` 5); `faults().calls()` of the pane store holds no `CasPut`; JSON `changed` is
   `false`. (b) Seed `demo-c2r1` with `Hold::None`: `pane unpark demo-c2r1` exits 0, `out` `demo-c2r1: not parked\n`, no
   `CasPut`. (c) Seed `demo-c3r1` with `Hold::Drained`: park prints `demo-c3r1: drained, left as it is\n` and unpark prints
   `demo-c3r1: not parked\n`; both exit 0 with no `CasPut`.
4. **Every pane of a profile.** Test `park_and_unpark_with_a_profile_take_every_member_in_name_order`. Profile store
   seeded with `sample_profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"])` and `sample_profile("Demo Beta", &["demo-c4r1"])`
   (actor `test`). Pane store seeded with `demo-c2r1` and `demo-c1r1` (in that order) whose `profile` is `Demo Alpha`,
   `demo-c3r1` with no profile, `demo-c4r1` in `Demo Beta`. `pane park --profile "Demo Alpha" --reason R --release-when W`
   exits 0; `out` has exactly two lines, `demo-c1r1: parked ...` then `demo-c2r1: parked ...`; both records are parked at
   generation 2; `demo-c3r1` and `demo-c4r1` are unchanged at generation 1; JSON `data.panes` lists the two in name order.
   `pane unpark --profile "Demo Alpha"` returns both to `Hold::None` (generation 3) and leaves the other two as they were.
   A profile with no member panes (the profile store also seeded with `sample_profile("Demo Empty", &[])`):
   `pane park --profile "Demo Empty" --reason R --release-when W` exits 0, text `no panes in profile "Demo Empty"\n`, JSON
   `{"panes":[]}`, and no `CasPut`.
5. **Membership and missing profile are refusals, and nothing is written.** Test
   `park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile`, over AC 4's seed, each run in both formats:
   `pane park demo-c3r1 --profile "Demo Alpha" --reason R --release-when W` exits 3 with code `pane-not-in-profile`;
   `pane park --profile "Demo Gamma" --reason R --release-when W` exits 3 with `profile-not-found`; `pane unpark demo-c4r1
   --profile "Demo Alpha"` exits 3 with `pane-not-in-profile`. No `CasPut` in the pane store's log; every record unchanged.
   A named member with the profile (`pane park demo-c1r1 --profile "Demo Alpha" ...`) parks only `demo-c1r1`.
6. **A missing pane.** Test `park_and_unpark_refuse_a_pane_with_no_record`. With no profile, `pane park demo-c9r9 --reason R
   --release-when W` and `pane unpark demo-c9r9` exit 3 with code `pane-not-found`; text `err` is
   `error: pane not found: demo-c9r9\n`; no `CasPut`.
7. **Usage, before any store call.** Test `park_and_unpark_usage_errors_touch_no_store`. Each exits 2 with code `usage`
   (text: `err` starts with `error: `; JSON: `check_envelope(&out, 2)` is `Ok`), and every fake's call log is empty
   (pane store and profile store included): (a) neither PANE nor `--profile` (`pane park --reason R --release-when W`;
   `pane unpark`), the message saying a PANE or `--profile NAME` is needed; (b) an invalid pane name
   (`pane park a/b --reason R --release-when W` and `pane unpark a/b`; `PaneName::parse` goes through `SessionName::parse`,
   which refuses a `/`: `crates/holler-proto/src/vocab.rs:86-87`, `if s.contains('/') { return Err(NameError::Slash); }`); (c) an invalid profile name (`--profile "   "`); (d) for park,
   `--reason` or `--release-when` that is blank after trimming, holds a control character (`"a\nb"`, `"a\u{1b}b"`), or is
   longer than 200 characters (201 `x`; 200 `x` is accepted, AC 1's shape). (e) A missing `--reason` or `--release-when`
   is clap's: `Cli::try_parse_from(["holler", "pane", "park", "demo-c1r1", "--reason", "r"])` is
   `Err` with kind `MissingRequiredArgument` (asserted directly; `run_verb_with` panics on a clap error, E-7).
8. **Store failures fail, with the pane named.** Test `park_failures_name_the_pane_and_stop`. (a) `fail_next(CasPut,
   PaneError::Conflict)` on the pane store, then `pane park demo-c1r1 --reason R --release-when W`: exit 1, code
   `generation-conflict`, message exactly `demo-c1r1: the record changed since it was read (generation conflict); read it
   again and retry`; the record unchanged. (b) A standing `Fault::Fail(PaneError::Unavailable { what: "pane store".into() })`
   on the pane store: exit 1, code `unavailable`. (c) **Stop at the first error, profile-wide:** three members of `Demo
   Alpha` (`demo-c1r1`, `demo-c2r1`, `demo-c3r1`) over a test-local `PaneStore` wrapper that delegates to the fake and
   answers `Conflict` on its second `cas_put` (the scope is built over the same wrapper): exit 1, code
   `generation-conflict`, message exactly `demo-c2r1: the record changed since it was read (generation conflict); read it
   again and retry; parked by this run before it: demo-c1r1; not reached: demo-c3r1`; `demo-c1r1` is parked (generation
   2), `demo-c2r1` and `demo-c3r1` are unchanged; the wrapper saw exactly two `cas_put` calls. Running the same command
   again with no fault exits 0, prints `demo-c1r1: already parked ...` and parks the other two (AC 3's idempotence is what
   makes the rerun safe). (d) The same three cases for `unpark` (its words: `unparked by this run before it`).
9. **No live act, no profile write.** In every AC above, the call logs of `FakeHerdr`, `FakeHost`, `FakeHarness` and
   `FakeProber` are empty, and the profile store's log has no `CasPut` and no `Delete` (a rig helper,
   `assert_no_live_call_and_no_profile_write`, is called at the end of each test).
10. **Exit codes equal across formats; every JSON output passes the helper.** Every case of AC 5-8 runs in both formats
    and asserts the same exit code; every JSON run's `out` passes `check_envelope(&out, code)` and its `err` is empty.
11. **The surface.** (a) `docs/adr/ADR-0003.md` rows 54-55 read exactly
    `holler pane park [PANE] --reason TEXT --release-when WHEN [--profile NAME]` and
    `holler pane unpark [PANE] [--profile NAME]`, each followed by **at least two spaces** and then `#646` (unpark padded
    to keep `#646` in its current column; park's text is longer than that column, so two spaces). Two spaces matter:
    `docs_cli_test` cuts a row at its first double space (`docs_cli_test.rs:135`, `for sep in ["  ", " (", " — ", " -- "]`),
    so with one space `#646` would be parsed as an argument. Row 56 (`close`) is untouched. (b) The four fixture lines become
    `pane park | demo-c1r1 --reason "disk full" --release-when "after the cleanup"`,
    `pane park | --profile demo --reason r --release-when w --format=json`, `pane unpark | demo-c1r1`,
    `pane unpark | --profile demo` (still under `# #646`, the `close` and prompt lines below untouched). (c) In
    `process/stub.rs` the `park` and `unpark` entries are deleted; `// #646` and the `close` entry stay. (d) These pass:
    `cargo test -p holler-cli --test pane_cli_process`, `--test cli_surface_test`, `--test docs_cli_test`, and
    `--test pane_verbs` (the `close` stub case still passes, untouched).
12. **ADR-0021 edited in this change.** `grep -c '^\*\*Park and unpark as built (#646).\*\*' docs/adr/ADR-0021.md` is `1`;
    the paragraph is the text of Decision 12, inserted after line 130 (the "Neither" bullet), and row 341 is unchanged.
13. **Hygiene.** `cargo clippy -p holler-cli --all-targets -- -D warnings` is clean; every new or rewritten `.rs` file
    passes `rustfmt --check --edition 2021` (epic ruling 4: existing files are not reformatted); `bash scripts/lint.sh`
    passes (no file at 900 lines or more; `park.rs`, `unpark.rs` and each test file well under); no `Cargo.toml` changes
    (`git diff --stat origin/main -- '*Cargo.toml'` is empty); no `unsafe`; `CHANGELOG.md` has one new entry under
    `## [Unreleased]` / `### Enhancements` and `bash scripts/changelog-check.sh` passes.

## Files

**Production (2):**
- `crates/holler-cli/src/pane/park.rs` (rewritten): `PanePark { pane: Option<String>, reason: String, release_when:
  String, profile: ProfileOpt }`, the shared hold-change engine (Decision 9) and its text renderer; about 200 lines.
- `crates/holler-cli/src/pane/unpark.rs` (rewritten): `PaneUnpark { pane: Option<String>, profile: ProfileOpt }` and a
  `run` that calls park's engine with the unpark change; about 40 lines.

**Tests (3):**
- `crates/holler-cli/tests/pane_verbs/park.rs` (rewritten): AC 1-3, 4-10 for park, 7e; declares `pub(crate) mod rig;`.
- `crates/holler-cli/tests/pane_verbs/park/rig.rs` (new): the rig of Decision 11 and
  `assert_no_live_call_and_no_profile_write`; about 120 lines.
- `crates/holler-cli/tests/pane_verbs/unpark.rs` (rewritten): unpark's cases of AC 3-10, over `crate::park::rig`.

**Surface and docs (5):** `docs/adr/ADR-0003.md` (rows 54-55 only), `docs/adr/ADR-0021.md` (one paragraph, Decision 12),
`crates/holler-cli/tests/fixtures/cli-surface.txt` (lines 132-135 only),
`crates/holler-cli/tests/pane_verbs/process/stub.rs` (lines 29-30 only), `CHANGELOG.md` (one entry).

**Not edited:** `pane/mod.rs`, `pane/args.rs`, `output.rs`, `cli.rs`, `close.rs`, `prompt_target.rs`, the
`say`/`interrupt`/`answer` files, `holler-pane/**`, `holler-pane-testkit/**`, `holler-hub/**`, any manifest, and
`tests/pane_verbs/main.rs` (its `mod park;` and `mod unpark;` already exist; the `park/` directory is reached from
`park.rs`'s own `mod rig;`).

### Reuse and analogous-feature map (extend, do not duplicate)

| Need | Reuse (object extended or called) | Not |
|---|---|---|
| The verb's argument struct | **extend `PanePark` and `PaneUnpark`** (E-1) with their own fields, keeping `#[command(flatten)] profile: ProfileOpt` | no new flag group; `args.rs` untouched |
| Verb shape: parse, type, act on ports, emit | `doctor.rs::run`/`pass` (E-6): `ErrorBody::from`, `output::emit`, `PaneName::parse`, `ProfileName::parse` | no hand-written envelope |
| Scope: every member, or one checked member | `ProfileScope::resolve` (E-3) through `ctx.ports.scope` | no membership check of its own (the scope's is the one rule) |
| The record write | `PaneStore::cas_put` at the record's generation (E-2) | no retry; no read-modify loop |
| Quoting untrusted text | `holler_pane::findings::quoted` (E-6) | no second quoting helper |
| The clock | `holler_proto::clock::now_millis` (E-6) | — |
| Unpark | **calls park's engine** (Decision 9) | no second copy of the loop, the scope logic or the renderer |
| Tests | `run_verb_with`, `check_envelope`, the #638 fakes and `sample_pane`/`sample_profile` (E-7) | no subprocess for the in-process cases; no new fake |

## Decisions already made (O; the MO may overrule)

1. **What park and unpark change: `hold` only, by one compare-and-swap per pane.** The verb sets the field on the record it
   read (from `resolve` or `get`) and writes it with `cas_put(&changed, record.generation)`. It makes no adapter call, so it
   never moves a pane's Herdr position or touches its processes (the issue: "Park, unpark and close never move a pane's
   Herdr position"), and never writes a profile. I3 reduces to plan and record: there is no act to observe.
2. **The arguments.** `park [PANE] --reason TEXT --release-when WHEN [--profile NAME]`; `unpark [PANE] [--profile NAME]`.
   `--reason` and `--release-when` are required clap flags (`Hold::Parked` has both as plain `String`s, E-2). `PANE` is
   optional at clap level, like doctor's (E-6); a run with neither `PANE` nor `--profile` is refused by the verb as `usage`
   (exit 2) before any store call. It is not a clap `required_unless_present` rule, because ADR 0003's row is parsed by
   `docs_cli_test` with its `[optional]` groups dropped (E-8), and `holler pane park --reason TEXT --release-when WHEN`
   must still parse.
3. **Scoping (ADR-0021 section 3, E-4).** `PANE` alone: `pane_store.get` (none is `pane-not-found`). `PANE` with
   `--profile P`: `scope.resolve(P, Some(PANE))` (membership refused there). `--profile P` alone: `scope.resolve(P, None)`,
   every member, in the name order `resolve` returns (the fake sorts by name, E-3; the verb sorts again by name so the order
   does not depend on the implementation). Without `--profile` the verb never reads a profile.
4. **Idempotent, never an error for "already".** Park changes only a pane whose hold is `none`; unpark only a parked one.
   A pane already in the asked state, or drained, is left exactly as it is (no write; a parked pane keeps its reason,
   release condition and `since`) and reported with `changed: false`. Reasons: ADR-0021 row 341 (E-4) plans no open code
   for these verbs; a profile-wide run over panes some of which are already parked must not fail; and a run that stopped
   part way can be run again (Decision 7). Re-parking to change a reason is unpark then park.
5. **The text guards (park).** `--reason` and `--release-when` are trimmed and stored trimmed; blank, holding a control
   character (`char::is_control`), or longer than 200 characters is `usage`. One line each keeps every message and text line
   one line (ADR-0021 section 9: "Every message is one line"); the cap bounds a value that the hub stores and every reader
   prints. They are not secrets and are not treated as such; text output still quotes them with `findings::quoted` (64
   characters, escaped), JSON carries them raw.
6. **`since`** is `now_millis()` at the moment the verb builds the change, once per run (every pane of one run gets the
   same `since`).
7. **A profile-wide run stops at the first error.** No transaction spans pane records (ADR-0021 section 8). The verb
   writes the members one by one in name order; on the first failed write it stops and answers that error's code, with the
   message `<pane>: <error text>`, followed, when the run had more than one pane, by `; parked by this run before it:
   <names or none>; not reached: <names or none>` (`unparked` for unpark). The envelope cannot carry data with a failure, so
   the message is where the partial result is reported; Decision 4 makes the rerun safe. A pane's write error is never
   retried. Errors before any write (`resolve`, `get`, `usage`) are answered as they are.
8. **Output.** `data` is `{"panes": [{"name", "changed", "generation", "hold"}]}` in name order; `hold` is `Hold`'s own
   serde form and `generation` the record's after the run. Text is one line per pane: `<name>: parked (reason <q>, release
   when <q>)`, `<name>: already parked (reason <q>, release when <q>)`, `<name>: drained, left as it is`, `<name>:
   unparked`, `<name>: not parked` (`<q>` = `quoted(..)`); an empty scope is `no panes in profile <q>`. Exit codes are
   `class_of`'s (E-4): 0, 2 `usage`, 3 `pane-not-found`/`profile-not-found`/`pane-not-in-profile`, 1 the store failures.
9. **One engine, in `park.rs`.** `park.rs` holds a `pub(super)` engine taking the typed target (pane, profile), the change
   (`Park { reason, release_when, since }` or `Unpark`) and `Ports`, returning the report or an `ErrorBody`; and the
   renderer. `unpark.rs` types its arguments and calls it. Both files are #646's (ruling 2: one verb, one file), so the
   engine stays private to the `pane` module; nothing in `holler-pane` changes (it is frozen, and the engine is a CLI verb's).
10. **No prompt gate here.** Whether a parked or unhealthy pane refuses a prompt, and where (the issue points at
    `send_prompt`, E-2's doc), is 646c's design, after the address question of E-10. This part writes `hold` and nothing
    reads it yet in Holler.
11. **The test rig is this part's own, small, and shared by the two verbs.** `tests/pane_verbs/park/rig.rs` builds the
    stores, the scope and the four live fakes behind `Arc`s and lends `Ports`; it offers `seed(panes)`, `record(name)` and
    the AC 9 check. Doctor's rig is private to `doctor.rs` and builds a live world park does not need (E-6); #644's brief
    already plans a follow-up that consolidates the rigs (its decision 25: "a follow-up consolidates the rigs into one
    module"). No production test switch: AC 8c's failure is a test-local `PaneStore` wrapper.
12. **ADR-0021 is extended, so it is edited in this change.** One paragraph after line 130 (the "Neither" bullet of
    section 3), with exactly this text:

    > **Park and unpark as built (#646).** `pane park [PANE] --reason TEXT --release-when WHEN` and `pane unpark [PANE]`
    > change only the pane record's `hold`, by one compare-and-swap per pane; they call no adapter and write no profile, so
    > they never move a pane in Herdr or touch its processes, and I3 reduces to plan and record. `park` sets `{"parked":
    > {reason, release_when, since}}` on a pane whose hold is `none` (`since` is the verb's clock); `unpark` sets `none` on a
    > parked pane. Both leave a pane already in that state, or `drained`, as it is and report it unchanged (exit 0). With
    > `--profile` and no pane name they take P's panes in name order and stop at the first failed write, whose message
    > names the panes changed before it and those not reached; the rerun is safe. The reason and the release condition are
    > one line each, not blank and at most 200 characters (`usage` otherwise). Which verbs and prompts read a parked pane
    > is decided by their stories (the watchdog, the roster #648, the prompt gate of #646).

    Section 9's row 341 stays as it is: the verbs answer only its codes and the common ones. No other ADR-0021 line is
    edited (the in-flight #644 and #663 edit sections 1, 2, 8, 9 row 338 and 12, none of them adjacent to line 130).
    ADR 0003: only rows 54-55; line 94 (the `--pane` refusal sentence) stays until 646c.
13. **Commit and PR.** Conventional Commit subject `feat(cli): holler pane park and unpark (#646 part 1 of 3)`; the PR body
    carries the AI-assistance disclosure `CONTRIBUTING.md` asks for and says "Part of #646".

## Forward-compat (the consumers of this part)

| Consumer | Needs | Satisfied |
|---|---|---|
| 646b `close` | the `// #646` stub line and the `close` entry kept; `close.rs` untouched | yes (AC 11c) |
| 646c routed prompts and the gate | a record whose `hold` is set by a verb, in `Hold`'s serde form | yes; 646c decides whether `parked` gates a prompt |
| #649 scenario ("park one") | `holler pane park NAME --reason TEXT --release-when WHEN` over the real wiring | yes; the verb uses only `PaneStore` and `ProfileScope` through `Ports`, which #649 wires |
| #648 roster, pfleet watchdog (#653) | `hold` readable from `pane get`/`list` (#643) | yes; the record is the hub's, unchanged in shape |
| #645 switch/reset | nothing; whether they refuse a parked pane is #645's | — (noted as a question for #645's brief) |
| #663 real `ProfileScope` | `resolve(P, None)` and `resolve(P, Some(n))` only | yes; the verb sorts by name itself (Decision 3) |
| Rig consolidation (#644's follow-up) | a rig small enough to fold into a shared one | yes (Decision 11) |

## Out of scope

- `pane close`, `--spec-only` and the `close` fixture lines (646b); the `say`/`interrupt`/`answer` routing, the pane-state
  gate, `say --queue`, the `PANE_FORM_REFUSAL` constant and ADR 0003 line 94 (646c).
- Any hub file, `holler-pane` file or test-kit file; any manifest; the real wiring (#649).
- Drain semantics (no story defines them; C-4); making reconcile or doctor skip parked panes (#647's files).
- Touching any live fleet, Herdr server, tmux session or OpenCode server: every test is in-process over the fakes.

## Follow-ups (the orchestrator files them; none blocks this part)

- **F1.** 646b brief (`close`), to start once #663 has merged (E-9).
- **F2.** 646c brief (routed prompts and the gate), after the operator or the architecture review picks how a pane maps to
  the hub session it drives and where the gate sits (E-10). **PROPOSED, operator to confirm.**
- **F3.** #645's brief: say whether `switch`/`reset` refuse a parked pane.

## Test plan

**RED** (a compile error is not RED, `docs/agent-overlays/tester.md`). T first lands, in `park.rs` and `unpark.rs`, the
argument structs of Decision 2 with `run` still answering `not_implemented(STORY)`, and the rig; then the tests. The RED run
shows: AC 1-6 and 8 fail on the exit code (1, `not-implemented`, where 0, 3 or the asserted code is expected) or on the
envelope's code; AC 7a-d fail on the exit code (1, not 2); AC 4 and AC 8c fail on the records. Green on the stub by design:
AC 7e (clap), AC 9 (no call is made by a stub), and AC 11's parsing tests once the fixture and ADR rows are changed with
the structs. T journals each failure.

**GREEN:** `cargo test -p holler-cli --test pane_verbs park`, `... unpark`, then `--test pane_cli_process`,
`--test cli_surface_test`, `--test docs_cli_test`, then AC 12-13. No timing assertion except AC 1's `t0 <= since <= t1`
bracket, which cannot flake.

## Risks

- **clap's error precedence in `flags.rs`.** With required `--reason`, `pane park --spec-only --profile demo` must still be
  `UnknownArgument`, not `MissingRequiredArgument`, for `spec_only_parses_on_launch_relaunch_and_close_only` (E-8). clap
  reports an unknown argument during parsing, before it validates required ones; AC 11d's `pane_cli_process` run is the
  check.
- **The rig grows a fourth copy** beside doctor's and #644's planned one. Accepted (Decision 11) with the follow-up #644
  already plans.
- **Partial profile-wide park** on a failure in the middle. Accepted: no store transaction spans records (ADR-0021
  section 8); the message names what changed and what did not, and the verb is idempotent (Decisions 4, 7).
- **`since` from the CLI's clock**, not the hub's. A skewed client clock writes a skewed `since`. Accepted: the record has
  no hub-stamped time (the hub "stamps nothing", `panes/mod.rs:9`), and `since` is informational.
