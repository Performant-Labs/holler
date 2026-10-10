# Brief: #640 herdr-adapter, part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows

Repo: Performant-Labs/holler. Issue: #640 (epic #633, wave 3), **part 3 of 3**. Rigor: second-opinion. UI surface: no.
Kind: feature (closing part).

**Branch:** `issue-640-implementation` (worktree `.claude/worktrees/0640-herdr-adapter`). Its tree equals `origin/main` at
`dc300ab` (2026-10-09 7:43 PM MDT), which holds part 1 (`3bdd129`, PR #699) and part 2 (`0ad2d8a`, PR #702).
**Review rigor:** second-opinion (MO, 2026-10-09). The issue's Pipeline line says `rigor: in-session`; parts 1 and 2 ran
at second-opinion too, and this part edits the contract crate's docs and ADR-0021, so it keeps that level.
**Design (D):** N/A (no UI surface).
**PR wording:** the PR says `Closes #640`. This is the last part.
**Handoffs:** `docs/handoffs/640/handoff-<phase>.md` and `docs/handoffs/640/decisions.md`. They overwrite part 2's
files of the same names; part 2's are in git history at `0ad2d8a` (the part-1 precedent).
**Public repository:** no personal host, account, path or IP in code, tests, docs, the CHANGELOG, commits or the PR.

## Scope

#640 runs as three sequential pipeline runs on one issue. The split is in the part-1 brief (`1571c6d:docs/handoffs/640-brief.md`,
"Outlines of parts 2 and 3") and the part-2 brief (`0ad2d8a:docs/handoffs/640-brief.md`, Scope table, row 3).

| Part | Delivers | State |
|---|---|---|
| 1 | Pure core: `protocol.rs`, `layout.rs`, `plan.rs` | merged (`3bdd129`) |
| 2 | `transport.rs`, `adapter.rs` (`HerdrAdapter`), the wire fake, the conformance run in the default test run | merged (`0ad2d8a`) |
| **3 (this brief)** | (a) an opt-in scratch-Herdr test (`#[ignore]`, env-gated, which refuses the default session by name); (b) doc-only edits to `holler-pane` (`ports.rs`: `Key` and `ensure_pane`; `error.rs`: `GridOutOfRange`, and `Timeout` for the `op` rule; `pane.rs` and `reconcile.rs`: who records `herdr_api_version`); (c) the ADR-0021 rows part 2 deferred here; (d) two small adapter follow-ups the part-2 reviews named: Herdr-sent text cut to 64 characters, and `timeout`'s `op` naming the port method. **Closes #640** | this run |

Issue acceptance covered by **this part**: "Against a scratch Herdr session (marked `#[ignore]`, opt-in): create, run,
send, read, close and snapshot a pane; nothing touches a non-scratch session (the test refuses the default session by
name)." Parts 1 and 2 covered the other three lines (their S handoffs: part 1 at `5133b9a`, part 2 on `main` at
`docs/handoffs/640/handoff-S.md`). Two issue lines still disagree with decisions taken in parts 1 and 2; see
"Contradictions with the issue text" and "For the operator", item 1.

## Problem

`holler-adapter-herdr` implements `HerdrPort` and passes the conformance suite against a simulated Herdr, but it has never
met a real Herdr server: four behaviours the wire fake models are INFERRED (the part-2 brief's Risks), and the issue's
last acceptance line is a test against a real, throwaway Herdr session. Part 2 also left written follow-ups that belong
to #640 and must land before it closes: the `holler-pane` docs still describe a provisional port (`Key` names `Enter` and
`C-c`, which Herdr does not use; `ensure_pane` does not name `grid-out-of-range` or `grid-unreachable`); ADR-0021 still
says "How the adapter learns a workspace's extent is #640's" and that the adapter records `host.herdr_api_version`, which
it does not; the adapter quotes text Herdr sent without the crate's 64-character cut; and the adapter's `timeout` names
a wire method (`herdr.layout.export`) where every other implementation and fake names the port method
(`herdr.ensure_pane`, `harness.health`, `host.ps`).

## Evidence (verbatim, as of `dc300ab`)

### E-1. The issue (verbatim, `gh issue view 640`, 2026-10-09)

```
## Scope
Implement `HerdrPort` in `holler-adapter-herdr` as the spike #636 recommends (socket or CLI). Pane ids are resolved to the registry's `herdr` fields; the adapter never caches a pane id past one call. The Herdr server runs on the hub's machine (epic decision 3), so the adapter uses the local socket and has no remote path. Timeouts on every call. (amended 2026-10-08, grid) This adapter is the **only** place that converts a `GridPos { row, col }` (1-based) to Herdr's native axis order and base and back, as #636 records them; if Herdr exposes no position, follow #636's fallback. No other crate knows Herdr's order.

(amended 2026-10-08, features) **Layout as splits**: `ensure_pane` places a pane at a `GridPos` by computing and issuing the right/down splits needed, through a pure function `plan_splits(existing, target)` in this crate, and fails loudly if the position cannot be reached. **API version**: detect Herdr's API version on connect (`version()`), adapt to the differences #636 lists, record it in `host.herdr_api_version` (doctor shows it), and refuse an unknown version with `herdr-version-unsupported`, naming the supported versions in the message.

## Acceptance
- Passes the conformance suite from #638.
- (amended 2026-10-08, grid) Unit tests against a fake Herdr that **rejects** a transposed or off-by-one position: placing panes at `r2c1` and `r1c2` lands each in the right cell, and the snapshot reads back the same `GridPos`; a deliberately swapped conversion fails these tests.
- (amended 2026-10-08, features) `plan_splits` unit tests: an empty workspace to 2 rows by 4 columns, adding one cell to a partial layout, an unreachable position refused; passes #638's split-only-mode case; each supported version works against the fake's two versions and an unknown one is refused with `herdr-version-unsupported`.
- Against a scratch Herdr session (marked `#[ignore]`, opt-in): create, run, send, read, close and snapshot a pane; nothing touches a non-scratch session (the test refuses the default session by name).
...
## Blast radius
- crates/holler-adapter-herdr/**

## Pipeline
`rigor: in-session`.
```

The epic's rule (`gh issue view 633`): "No story touches a live fleet, a real Herdr session, a running pane or a real
OpenCode session; verification is the test kit and scratch instances, and tests refuse real session names."

### E-2. The part-2 follow-ups this part owes (on `main`, `docs/handoffs/640/`)

```
docs/handoffs/640/handoff-S.md:184-190
2. **`Timeout.op` vocabulary diverges from `FakeHerdr`** (wire method `herdr.layout.export` vs port method
   `herdr.ensure_pane`). Decision 3 as written; carry it with Decision 20 into part 3's ADR-0021 §9/§10 rows and the
   operator's list (A finding 1, A-dup warn 2).
3. **Herdr-sent text is quoted with `{:?}`, not cut to 64 characters** (`adapter.rs:230-241`, `:372-374`, `:385-391`). One line always; length unbounded. Follow-up: make `protocol::excerpt` `pub(crate)` in part 3 or later
   (A-dup warn 1).
4. **Part 3 must write the ADR rows** for extent-from-config, the gate at `connect`/`version()`, and the `op` rule
   (A-dup warn 3); #642 should reuse this bounded-exchange design via a shared home, not a copy (A-dup warn 4).
```

```
docs/handoffs/640/handoff-A-dup.md:26 (finding 1, warn; the "Suggested fix" column)
A follow-up, or part 3 if its brief allows it: make `protocol::excerpt` `pub(crate)` (a visibility-only edit) and route these three quotes through it. Nothing needs to change in this part.
```

```
docs/handoffs/640/handoff-A-dup.md:28 (finding 3, warn)
... This part settles three things that the ADR leaves to #640 or does not state yet: a workspace's extent comes from configuration, per label (`HerdrConfig.workspaces`, `adapter.rs:51`); the version gate runs at `connect` and in `version()` only (`adapter.rs:95-100`, `:346-355`); and the `op` rule in finding 2. ...
| Part 3's brief names all three items when it writes the §9/§10 rows. |
```

Part 1's S (`5133b9a:docs/handoffs/640/handoff-S.md`, advisory 6) also records: "**A-dup W-1:** the `excerpt` copy. Its
64-character limit is not pinned by any test." (part 1's T-green: the mutant "`EXCERPT_LIMIT` 64 becomes 1000" survived).

### E-3. The adapter's quoting sites (`crates/holler-adapter-herdr/src/adapter.rs`, 445 lines)

```
crates/holler-adapter-herdr/src/adapter.rs:219-244
    /// Read the tree of `tab` back: `made` must sit at `spec.grid`. Otherwise it is
    /// `unavailable`, and the pane is left where Herdr put it.
    fn confirm(
        &self,
        tab: &str,
        made: &PaneId,
        spec: &HerdrSpec,
        deadline: Instant,
    ) -> Result<(), PaneError> {
        let what = match self.grid(tab, deadline)?.position_of(made) {
            Some(landed) if landed == spec.grid => return Ok(()),
            Some(landed) => format!(
                "Herdr put the new pane {:?} at {landed}, not at {}; it is left where it landed",
                made.as_str(),
                spec.grid
            ),
            None => format!(
                "the new pane {:?} has no cell in workspace {:?}, so it is not at {}; it is left \
                 where Herdr put it",
                made.as_str(),
                spec.workspace,
                spec.grid
            ),
        };
        Err(PaneError::Unavailable { what })
    }
```

```
crates/holler-adapter-herdr/src/adapter.rs:366-395
/// The grid tab of `workspace`. A workspace with no tab has no grid: `unavailable`.
fn grid_tab(workspace: &WorkspaceRef) -> Result<String, PaneError> {
    workspace
        .grid_tab
        .clone()
        .ok_or_else(|| PaneError::Unavailable {
            what: format!(
                "Herdr workspace {:?} has no tab, so it has no grid",
                workspace.label
            ),
        })
}

/// What an error from the split of `target` means: `pane-not-found` is about the
/// pane split, not the one asked for, so it is `unavailable` (the layout changed while
/// the pane was placed). Any other error passes unchanged.
fn changed_under(error: PaneError, target: &PaneId, spec: &HerdrSpec) -> PaneError {
    match error {
        PaneError::PaneNotFound { .. } => PaneError::Unavailable {
            what: format!(
                "Herdr no longer has the pane {:?} that {} is split from: workspace {:?} \
                 changed while the pane was placed",
                target.as_str(),
                spec.grid,
                spec.workspace
            ),
        },
        other => other,
    }
}
```

`made` comes from Herdr's `pane.split` or `workspace.create` reply, `target` from Herdr's `layout.export` tree, and
`workspace.label` from Herdr's `session.snapshot`. `spec.session`, `spec.workspace` and `config.*` are the caller's.
`grid_tab` is reached with `grid_tab: None` only from `session.snapshot` (`parse_workspace_created` always sets it):

```
crates/holler-adapter-herdr/src/protocol.rs:491-495
    let workspace = WorkspaceRef {
        workspace_id: workspace.string("workspace_id")?.to_owned(),
        label: workspace.string("label")?.to_owned(),
        grid_tab: Some(tab.string("tab_id")?.to_owned()),
    };
```

### E-4. The crate's message rule and its one `excerpt` (`crates/holler-adapter-herdr/src/protocol.rs`)

```
crates/holler-adapter-herdr/src/protocol.rs:20-22
//! - **Messages** are one line. They name the method and Herdr's code, quote what Herdr
//!   sent (cut to 64 characters), and never echo the text typed into a pane, Herdr's own
//!   error message (which can quote it) or a pane's screen.
```

```
crates/holler-adapter-herdr/src/protocol.rs:62-63
/// How many characters of a string Herdr sent a message quotes.
const EXCERPT_LIMIT: usize = 64;
```

```
crates/holler-adapter-herdr/src/protocol.rs:578-587
/// `text` that Herdr sent, quoted on one line and cut to [`EXCERPT_LIMIT`] characters,
/// so that a garbled reply can neither lengthen a message nor break it across lines.
fn excerpt(text: &str) -> String {
    let head: String = text.chars().take(EXCERPT_LIMIT).collect();
    if head.len() < text.len() {
        format!("{head:?}...")
    } else {
        format!("{head:?}")
    }
}
```

`protocol.rs` calls it at lines 290, 329, 372, 378, 467, 480 and 568. It is a copy of `holler_pane::error::excerpt`
(`crates/holler-pane/src/error.rs:688`, `pub(crate) fn excerpt(text: &str) -> String {`, the same 64-character rule),
which is crate-private in the frozen contract crate.

### E-5. How `timeout` names its `op`, everywhere

The Herdr transport names the wire method (its module doc, `transport.rs:18-19`: "its `op` naming the method
(`herdr.ping`)"), and the adapter passes it through unchanged:

```
crates/holler-adapter-herdr/src/transport.rs:268-273
    /// `timeout`, its `op` naming the method.
    fn timeout(&self) -> PaneError {
        PaneError::Timeout {
            op: format!("herdr.{}", self.method),
        }
    }
```

```
crates/holler-adapter-herdr/src/adapter.rs:112-115
    /// The `result` of Herdr's reply to `request`.
    fn call(&self, request: &Request, deadline: Instant) -> Result<Value, PaneError> {
        decode_reply(request, &self.transport.exchange(request, deadline)?)
    }
```

```
crates/holler-adapter-herdr/src/adapter.rs:90-100
impl<T: Transport> HerdrAdapter<T> {
    /// Connect over `transport`. A config the adapter cannot serve is `usage`, before
    /// any request. Then one `ping`: a Herdr protocol the adapter does not know is
    /// `herdr-version-unsupported`, and no adapter is made. Over a transport other than
    /// the socket this is a test seam; production uses [`HerdrAdapter::connect`].
    pub fn connect_with(config: HerdrConfig, transport: T) -> Result<Self, PaneError> {
        validate(&config)?;
        let adapter = Self { config, transport };
        adapter.supported_server(adapter.deadline()?)?;
        Ok(adapter)
    }
```

Every other implementation and fake names the port method:

```
crates/holler-pane-testkit/src/fault.rs:17-22
/// A port method that a fault can target and the call log records. Each fake has its
/// own enum of them, e.g. [`crate::pane_store::PaneStoreOp`].
pub trait PortOp: Copy + Eq + Debug + Send + Sync + 'static {
    /// `"<port>.<method>"`, e.g. `"pane_store.cas_put"`. It is also the `op` of the
    /// `timeout` a wedged call answers.
    fn as_str(self) -> &'static str;
```

```
crates/holler-pane-testkit/src/herdr.rs:68-69 (FakeHerdr's ops; the other five follow the same form)
            HerdrOp::EnsurePane => "herdr.ensure_pane",
            HerdrOp::SendText => "herdr.send_text",
```

```
crates/holler-adapter-opencode/src/lib.rs:18-21 (#642 part 1, merged in dc300ab, after part 2's Decision 3)
//! - Every method takes a deadline of [`Timeouts::call`] when it is entered, and every
//!   request in it uses the smaller of its own bound and what is left. Past the deadline it
//!   answers `timeout`, whose `op` is `"harness.<method>"` (the test kit's `HarnessOp`
//!   strings).
```

This excerpt is from #641's branch, not from `dc300ab`: at `dc300ab`, `crates/holler-adapter-host/src/lib.rs` is a
5-line stub. Read it with `git show ec55e02:crates/holler-adapter-host/src/lib.rs` (lines 134-137; `ec55e02` is on
`origin/issue-641-implementation`).

```
ec55e02:crates/holler-adapter-host/src/lib.rs:134-137 (#641, branch issue-641-implementation at ec55e02, not merged)
const OP_ENSURE_SESSION: &str = "host.ensure_session";
const OP_RUN: &str = "host.run";
const OP_STOP_OWNED: &str = "host.stop_owned";
const OP_PS: &str = "host.ps";
```

The reconcile engine (#647, merged) names each call the same way and embeds the error's own text after it:

```
crates/holler-pane/src/reconcile.rs:59-60
const HERDR_VERSION: &str = "herdr.version";
const HERDR_SNAPSHOT: &str = "herdr.snapshot";
```

```
crates/holler-pane/src/reconcile.rs:438-444
fn failed_message(op: &str, error: &PaneError) -> String {
    format!(
        "{op} failed ({}): {}",
        error.code(),
        embedded(&error.to_string())
    )
}
```

(`crates/holler-pane/src/reconcile/observe.rs:16`: "The calls of a chain, named as `<port>.<method>` in an
`observe-failed` message.") `PaneError::Timeout`'s `Display` is `timed out: {op}` (`error.rs:673`). So over `FakeHerdr`
doctor reports `herdr.version failed (timeout): timed out: herdr.version`, and over today's adapter
`herdr.version failed (timeout): timed out: herdr.ping`.

No test of `holler-adapter-herdr` asserts an adapter-level `op` today
(`grep -n '"herdr\.' crates/holler-adapter-herdr/tests/adapter_test.rs crates/holler-adapter-herdr/tests/adapter_conformance_test.rs`
prints nothing); the transport tests assert the wire form at the transport level.

### E-6. The `holler-pane` docs to edit (doc comments only)

```
crates/holler-pane/src/ports.rs:100-104
/// One key to press with `HerdrPort::send_keys`, by the name Herdr uses (for example
/// `Enter` or `C-c`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(String);
```

```
crates/holler-pane/src/ports.rs:126-129
pub trait HerdrPort: Send + Sync {
    /// Make a pane exist at `spec.grid` by issuing right/down splits, or fail
    /// loudly; never relocates a healthy pane.
    fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;
```

```
crates/holler-pane/src/ports.rs:146-147
    /// Herdr's API version. An unknown version is `herdr-version-unsupported`.
    fn version(&self) -> Result<String, PaneError>;
```

```
crates/holler-pane/src/error.rs:414-416
    /// `grid-out-of-range`: a row or column of zero, or above `u16::MAX`. `what`
    /// quotes the text that was refused and gives the bounds. (#637, `GridPos`.)
    GridOutOfRange { what: String },
```

```
crates/holler-pane/src/error.rs:454-456
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names it. (#638-#642.)
    Timeout { op: String },
```

```
crates/holler-pane/src/pane.rs:112
    /// The Herdr API version, recorded by the Herdr adapter (#640) on connect.
```

```
crates/holler-pane/src/reconcile.rs:105-106
/// One host of the panes in scope, as their records name it. Doctor shows the recorded
/// Herdr API version and writes neither field (#640 records it).
```

What the adapter actually emits for an out-of-range cell (part 1, merged):

```
crates/holler-adapter-herdr/src/plan.rs:231-240
/// `grid-out-of-range` for `cell`, naming the extent.
fn out_of_range(cell: GridPos, extent: Extent) -> PaneError {
    PaneError::GridOutOfRange {
        what: format!(
            "{cell} is outside the workspace, which is {} by {}",
            count(usize::from(extent.rows), "row"),
            count(usize::from(extent.cols), "column")
        ),
    }
}
```

Part 1 decided keys go out verbatim by Herdr's names (`1571c6d` brief, Decision 9: "**Keys go out verbatim, by Herdr's
names** (`enter`, `ctrl+c`). There is no case-folding ... Part 3 fixes the `Key` doc in `ports.rs`."), and the adapter
writes no record (Decision 8: "the adapter does not write the pane store, and #644 and #647 record and show it"). The
test kit asks #640 for exactly these doc edits:

```
crates/holler-pane-testkit/src/conformance/herdr.rs:189-196
// ASSUMPTION (#640): `ensure_pane` outside the workspace is `grid-out-of-range`
// (ADR-0021 sections 9 and 10, as #683 amends them; #638 amendment 2026-10-08, grid).
// The `holler-pane` docs still describe less: the `PaneError::GridOutOfRange` doc ("a row
// or column of zero, or above `u16::MAX`") and the `HerdrPort::ensure_pane` doc ("or
// fail loudly"). #640 updates both when it finalizes `HerdrPort`. Once #640 decides where
// a workspace's extent comes from, it also updates ADR-0021 section 9's `profile apply`
// row, and any other verb's row, when that verb can ask `ensure_pane` for a cell outside
// the extent.
```

Which verbs call `ensure_pane`: #644 (`launch`, `relaunch`) and #664 (`profile apply`: "it places panes through
`HerdrPort::ensure_pane`, which reaches the `GridPos` by splits (#640)", `gh issue view 664`). No merged verb calls it
yet (`grep -rn 'ensure_pane' crates/holler-cli/src` finds only the stub in `pane/wiring.rs:122`).

### E-7. ADR-0021 as it stands (`docs/adr/ADR-0021.md`, 562 lines)

```
docs/adr/ADR-0021.md:3
**Status:** accepted (the items marked **PROPOSED** below are not; they wait for the operator, see "Decisions taken")
```

```
docs/adr/ADR-0021.md:40
| `host` | `HostInfo { name, tmux, cwd, herdr_api_version? }` | `herdr_api_version` is recorded by the Herdr adapter (#640). |
```

```
docs/adr/ADR-0021.md:339
| `pane launch`, `pane relaunch` | `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `profile-secret-refused`, `probe-failed`, `herdr-version-unsupported`, `session-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-in-other-profile`; `relaunch` also `pane-not-found` |
```

```
docs/adr/ADR-0021.md:350
| `profile apply` | `profile-not-found`, `pane-in-other-profile` (without `--take-over`), `probe-failed` (per pane; the others proceed), `herdr-version-unsupported`, `generation-conflict`, `profile-conflict` |
```

```
docs/adr/ADR-0021.md:396
  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. |
```

```
docs/adr/ADR-0021.md:401
  | any other well-formed code (open, raised as `Refused`) | Refusal (3) | `Refused` is the channel for the codes a verb or adapter owns; the open codes planned so far (#645, #646) are all refusals. |
```

```
docs/adr/ADR-0021.md:425-427
- **`grid-out-of-range`:** a zero (`r0c1`, `c0r1`, `0,1`, `r2c0`) or a number above 65535; or, from `HerdrPort::ensure_pane`,
  a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort`
  conformance suite pins (#638 amendment 2026-10-08, grid; #683). How the adapter learns a workspace's extent is #640's.
```

```
docs/adr/ADR-0021.md:434-436
- Herdr API versions (B5): `HerdrPort::version()` is read on connect, recorded as `host.herdr_api_version` and shown by
  `doctor`. An unknown version is `herdr-version-unsupported`, and the message names the supported ones (#640, from #636's
  list).
```

```
docs/adr/ADR-0021.md:450-451
`HerdrPort::send_text` and `send_keys` exist in the provisional port. Under I4 no verb may use them to change which
session a pane shows or what it is doing; #640 and #646 name their permitted uses, and #645's tests fail on any keystroke.
```

```
docs/adr/ADR-0021.md:537
- `HerdrPort` and `HarnessPort` in their final form: #636 and #635, then #640 and #642.
```

The open code `grid-unreachable` merged in part 1 (`crates/holler-adapter-herdr/src/plan.rs`, `GRID_UNREACHABLE`) and
appears nowhere in ADR-0021 (`grep -n 'grid-unreachable' docs/adr/ADR-0021.md` prints nothing).

### E-8. What the adapter already does that the ADR must state (part 2, merged)

```
crates/holler-adapter-herdr/src/adapter.rs:4-12
//! - **What it is told.** One Herdr session, that session's socket, and the rows and
//!   columns of each workspace it places panes in ([`HerdrConfig`]): Herdr has no grid,
//!   and nothing here finds a socket or reads an environment variable by itself.
//! - **One deadline per call.** Each port method takes its deadline once, on entry,
//!   `timeout` from then (I5's bound, [`DEFAULT_TIMEOUT`] unless configured), and every
//!   exchange of the call runs against that one deadline.
//! - **The version gate** runs at [`HerdrAdapter::connect`] and in every `version()`,
//!   so no adapter exists for a Herdr protocol it does not know. The other methods
//!   trust the gate that passed at connect.
```

```
crates/holler-adapter-herdr/src/adapter.rs:43-54
/// What the adapter is told (Herdr has no grid and no discoverable socket of Holler's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrConfig {
    /// The one Herdr session served.
    pub session: String,
    /// That session's socket, absolute.
    pub socket: PathBuf,
    /// Workspace label to rows by columns.
    pub workspaces: BTreeMap<String, Extent>,
    /// The bound of one `HerdrPort` call.
    pub timeout: Duration,
}
```

The builder AC 12 uses already exists (part 2, merged):

```
crates/holler-adapter-herdr/src/adapter.rs:56-72
impl HerdrConfig {
    /// A config with no workspace and [`DEFAULT_TIMEOUT`].
    pub fn new(session: impl Into<String>, socket: impl Into<PathBuf>) -> Self {
        Self {
            session: session.into(),
            socket: socket.into(),
            workspaces: BTreeMap::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// This config, with the workspace `label` of `extent`.
    pub fn with_workspace(mut self, label: impl Into<String>, extent: Extent) -> Self {
        self.workspaces.insert(label.into(), extent);
        self
    }
}
```

`connect` (`adapter.rs:84`) and `connect_with` (`adapter.rs:95`) are inherent methods of `HerdrAdapter`, not
`HerdrPort` methods, so the test kit's `HerdrOp` has no entry for them.

The operator's ruling on the version line (part 2's `decisions.md:1`): "Brief-gate BLOCKED, overridden by Andre
Angelantoni ... Operator approved overriding the non-converging brief gate on 2026-10-09", so part 2's Decision 8 stands:
the gate is at connect and at `version()`.

### E-9. How the spike reached a real Herdr without touching a live one (`scripts/spikes/herdr-lib.sh`, spike #636)

```
scripts/spikes/herdr-lib.sh:4-13
# Every Herdr call made through this file goes to ONE scratch session that this file creates
# and later stops. Two independent guards keep a real Herdr session out of reach:
#   1. Herdr runs with HOME and every XDG_* directory pointed into a fresh `mktemp -d` root,
#      and with every inherited HERDR_* variable removed, so even Herdr's *default* session
#      resolves to a socket inside the scratch root (a path no real server listens on).
#   2. Every call also names the scratch session explicitly (`--session spike636-<random>`).
# spike_start refuses to continue unless the running server reports that session and a socket
# inside the scratch root. spike_stop (an EXIT trap) stops only the server PID this file
# started, kills only the private tmux server it created, and removes the scratch root.
```

```
scripts/spikes/herdr-lib.sh:21-25
# Unix socket paths are limited to ~108 bytes; keep the root short.
_tmp="${TMPDIR:-/tmp}"
[ "${#_tmp}" -le 40 ] || _tmp=/tmp
SPIKE_ROOT="$(mktemp -d "$_tmp/h636.XXXXXX")"
SPIKE_SESSION="spike636-$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
```

`herdr-lib.sh:39-50` writes `config.toml` with `onboarding = false`, `[terminal] default_shell = "/bin/sh"`,
`[update] version_check = false` and `manifest_check = false` (no network), plus two sidebar `rows` lines.

```
scripts/spikes/herdr-lib.sh:55-60
exec env -u HERDR_SOCKET_PATH -u HERDR_ENV -u HERDR_PANE_ID -u HERDR_TAB_ID -u HERDR_WORKSPACE_ID \\
  -u HERDR_CONFIG_PATH -u HERDR_HOME -u HERDR_SESSION -u TMUX \\
  HOME="$SPIKE_HOME" XDG_CONFIG_HOME="$SPIKE_HOME/.config" XDG_STATE_HOME="$SPIKE_HOME/.local/state" \\
  XDG_DATA_HOME="$SPIKE_HOME/.local/share" XDG_CACHE_HOME="$SPIKE_HOME/.cache" \\
  XDG_RUNTIME_DIR="$SPIKE_ROOT/run" SHELL=/bin/sh PS1='\$ ' \\
  herdr "\$@"
```

```
scripts/spikes/herdr-lib.sh:116-118 and :124-145
spike_start() {
  "$SPIKE_BIN/hx" --session "$SPIKE_SESSION" server >"$SPIKE_ROOT/server.log" 2>&1 &
  SPIKE_SERVER_PID=$!
...
spike_prove_target() {
  local st sess sock
  st="$(h status server --json)"
  sess="$(jq -r .session <<<"$st")"
  sock="$(jq -r .socket <<<"$st")"
  case "$sock" in "$SPIKE_ROOT"/*) ;; *) echo "spike: REFUSING, socket is outside the scratch root" >&2; exit 1 ;; esac
  [ "$sess" = "$SPIKE_SESSION" ] || { echo "spike: REFUSING, server reports another session" >&2; exit 1; }
  [ "$sock" = "$SPIKE_SOCKET" ] || { echo "spike: REFUSING, socket mismatch" >&2; exit 1; }
  case "$SPIKE_SESSION" in spike636-*) ;; *) echo "spike: REFUSING, not a spike session name" >&2; exit 1 ;; esac
  echo "target proven: session=$sess socket=$sock server_pid=$SPIKE_SERVER_PID" | spike_redact
}

spike_server_stop() { # stop only the server this file started, and wait for it to exit
  [ -n "$SPIKE_SERVER_PID" ] || return 0
  if kill -0 "$SPIKE_SERVER_PID" 2>/dev/null; then
    h server stop >/dev/null 2>&1 || true
    for _ in $(seq 1 50); do kill -0 "$SPIKE_SERVER_PID" 2>/dev/null || break; sleep 0.2; done
    kill -0 "$SPIKE_SERVER_PID" 2>/dev/null && kill "$SPIKE_SERVER_PID" 2>/dev/null || true
    wait "$SPIKE_SERVER_PID" 2>/dev/null || true
  fi
  SPIKE_SERVER_PID=""
}
```

```
docs/research/herdr-api-spike.md:28-29
- refuses to continue unless `herdr status server --json` reports that session and a socket inside the scratch
  root (`target proven: session=<scratch-session> socket=<scratch-root>/home/.config/herdr/sessions/<scratch-session>/herdr.sock`);
```

```
docs/research/herdr-api-spike.md:106-108
- A Unix stream socket, owner-only (`srw-------`). Default session: `$XDG_CONFIG_HOME/herdr/herdr.sock` (on Linux
  `~/.config/herdr/herdr.sock`); a named session: `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`.
  `herdr session list` prints each session's socket path. Inside a Herdr pane, `HERDR_SOCKET_PATH` names it.
```

The spike's operations table (`herdr-api-spike.md:129`) says `pane.send_text` is "literal text, no Enter".

```
docs/research/herdr-api-spike.md:481
- Behaviour on macOS (`shell_mode = "auto"` uses login shells there, DOCS). Everything here ran on Linux.
```

The four INFERRED behaviours of the wire fake that only a real server can check (part 2 brief, Decision 15 and Risks):
`pane_not_found` for `pane.send_text`, `pane.send_keys` and `pane.read` of a closed pane (conformance case 9 depends on
it); the last-pane close; the `tab_not_found` code; the ratio range. The machine that runs the pipeline has
`herdr 0.9.1-preview.2026-09-21-0ff0f27e2226` on `PATH`, the build the spike tested (spike §13, protocol 22).

### E-10. The suite's contract for a real-server run (`crates/holler-pane-testkit/src/conformance/herdr.rs`)

```
crates/holler-pane-testkit/src/conformance/herdr.rs:57-64
pub struct HerdrFixture<H> {
    /// The implementation under test.
    pub port: H,
    /// The scratch session: never the default session or a live one.
    pub session: String,
    /// The workspace of 2 rows by 1 column that the cases place their panes in.
    pub workspace: String,
}
```

```
crates/holler-pane-testkit/src/conformance/herdr.rs:122-139
/// Run every case of [`herdr_cases`], in order, each against a fresh fixture, and
/// return every case that did not hold.
///
/// `fresh` is called once per case. It returns the fixture and a guard that the suite
/// keeps alive for that case only: `()` for the fake, the scratch server's handle for
/// the adapter (#640). The fixture is dropped before its guard. How each implementation
/// runs the suite:
///
/// ```text
/// // the fake:
/// run_herdr_conformance(|| {
///     let port = FakeHerdr::new("scratch").with_workspace("scratch", 2, 1).expect("workspace");
///     (HerdrFixture { port, session: "scratch".into(), workspace: "scratch".into() }, ())
/// })
/// // the adapter (#640, opt-in, #[ignore]): a fresh scratch Herdr session per case whose
/// // workspace the adapter knows to be 2 rows by 1 column; the session's server handle
/// // as the guard; never the default or a live session.
/// ```
```

The suite has 11 cases (`conformance/herdr.rs:79`, `const CASES: [(&str, Case); 11]`). Case 10 types five lines and
asserts only "at most 2" read back; case 9 needs `pane-not-found` from `send_text`, `send_keys` and `read` of a closed
pane.

Part 2's socket run is the shape this part repeats with a real server in place of the served fake:

```
crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:52-63
/// The suite over a real socket: each case serves a fresh, empty fake, and the server
/// is the case's guard, dropped after the port.
fn run_over_a_socket(fakes: &mut Vec<Arc<WireHerdr>>) -> Conformance {
    run_herdr_conformance(|| {
        let fake = Arc::new(WireHerdr::new());
        fakes.push(Arc::clone(&fake));
        let served = serve(Arc::clone(&fake));
        let config = config(served.path());
        let port = HerdrAdapter::connect(config).expect("connect");
        (fixture(port), served)
    })
}
```

### E-11. The test seam the new adapter tests extend (`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs`, 650 lines)

```
crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:212-251
/// What a [`Tap`]'s hook does with one exchange.
pub enum Tapped {
    /// Hand this line (the request's, or a rewrite of it) to the fake, and return its
    /// answer.
    Forward(String),
    /// Return this reply line; the fake sees nothing.
    Reply(String),
}

/// The hook of a [`Tap`]: the fake, the request's line and the exchange's deadline.
type Hook = dyn Fn(&WireHerdr, String, Instant) -> Tapped + Send + Sync;

/// A transport in front of a fake whose hook sees each exchange first: to rewrite a
/// request, act on the fake, record the deadline or garble the reply (AC 16, 17, 28 and
/// 29 share it).
pub struct Tap {
...
impl Transport for Tap {
    fn exchange(&self, request: &Request, deadline: Instant) -> Result<String, PaneError> {
        match (self.hook)(&self.fake, request.to_line(), deadline) {
            Tapped::Forward(line) => Ok(self.fake.answer(&line)),
            Tapped::Reply(reply) => Ok(reply),
        }
    }
}
```

An existing test drives `confirm`'s "landed elsewhere" arm through the `Tap`: in
`adapter_test.rs:197-225` (`a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place`), the hook rewrites `pane.split`'s
`"direction":"down"` to `"right"`, so `ensure_pane(r2c1)` in a 2-by-2 workspace lands `w1:p2` at `r1c2`, and the test
asserts `what.contains("w1:p2") && what.contains("r1c2")`.

`adapter_test.rs` is 645 lines; `transport_test.rs` 431; `wire_herdr/mod.rs` 650.

AC 18 can name a pane `LONG`: `PaneId` does not validate its text, and the adapter parses Herdr's ids straight into it
(`protocol.rs:441`, `:457`, `:496`, `:502`: `PaneId::new(<..>.string("pane_id")?)`):

```
crates/holler-pane/src/pane.rs:79-83
impl PaneId {
    /// A pane id from Herdr's text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
```

`validate` (`adapter.rs:400-430`) checks a workspace label's extent only (zero rows or columns is `usage`), not its
length, so a configured label `LONG` is accepted.

### E-12. Build, CI and the platform rule

```
crates/holler-adapter-herdr/Cargo.toml:22-28
[dev-dependencies]
# `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS`, which the tests pin equal to this
# crate's own constants (#640 part 1, AC 13). A dev-dependency only (ADR-0021 section 5).
holler-pane-testkit = { path = "../holler-pane-testkit" }
# The scratch directory that holds each test's Unix socket (the transport tests and the
# wire fake's `serve`, #640 part 2). A dev-dependency only.
tempfile = { workspace = true }
```

`serde_json` is already a normal dependency (`Cargo.toml:20`), so tests can parse JSON with it.

```
.github/workflows/ci.yml:128
        run: cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
.github/workflows/ci.yml:200
        run: cargo test -p holler-cli --test body_run_test -- --ignored
```

So CI never runs an ignored test of this crate (`--ignored` is passed only for `holler-cli`'s `body_run_test`). CI's
matrix runs Linux and `macos-latest` (`ci.yml:19`, "`test (macos-latest)`").

The macOS lesson from part 2 (the fix that landed in `0ad2d8a`):

```
crates/holler-adapter-herdr/src/transport.rs:221-229
    /// Set a socket timeout (`set`) to the time left: `timeout` when there is none.
    ///
    /// macOS refuses `SO_RCVTIMEO`/`SO_SNDTIMEO` with `EINVAL` (`InvalidInput`) on a
    /// socket whose peer has already closed it (seen on CI's macOS runner, PR #702).
    /// That is not the exchange failing: on such a socket the read or write returns at
    /// once (the bytes still buffered, EOF, or a broken pipe), so the call goes ahead
    /// and its own result is the answer. std's one `InvalidInput` of its own, for a zero
    /// timeout, cannot happen here, since [`Exchange::left`] is never zero. Any other
    /// error is `unavailable`, as before.
```

Part 2's independence grep, which this part's harness would trip (it must read one env variable and strip `HERDR_*`):

```
docs/handoffs/640-brief.md:1426 (part 2's brief, on main)
38. `grep -rnE "TcpStream|TcpListener|47001|47002|tmux|std::env|env::var|\.config/herdr|HERDR_|unsafe" crates/holler-adapter-herdr/`
```

The baseline at `dc300ab`, over the whole crate (`src/`, `tests/` including `wire_herdr/` and `common/`, `Cargo.toml`),
each run from the repo root:

- `grep -rn 'Command::new' crates/holler-adapter-herdr/` prints nothing;
- `grep -rn 'std::env\|env::var' crates/holler-adapter-herdr/` prints nothing;
- part 2's AC 38 pattern above prints nothing.

So every match of these greps after this part comes from a file this part adds or changes.

## The test harness this part creates (pinned names; T writes it, it is test code)

`crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs`, a test module like `wire_herdr`. T may add private helpers;
the public items below are pinned so S can check them.

```rust
/// The variable that opts in; only the exact value "1" runs a scratch server.
pub const GATE_VAR: &str = "HOLLER_HERDR_SCRATCH";
/// Every scratch session's name is this prefix and 8 lower-case hex digits.
pub const NAME_PREFIX: &str = "holler640-";
/// Every scratch root's directory-name prefix.
pub const ROOT_PREFIX: &str = "h640.";
/// The longest socket path accepted, in bytes (macOS's sockaddr_un holds 104).
pub const SOCKET_PATH_LIMIT: usize = 100;

#[derive(Debug, PartialEq, Eq)]
pub enum Gate { Run, Skip }

/// Pure: `Run` exactly when `value` is `Some("1")`.
pub fn gate(value: Option<&str>) -> Gate;
/// Pure: `Ok` only for `NAME_PREFIX` + 8 of `[0-9a-f]`; `default` is refused by name.
pub fn check_name(name: &str) -> Result<(), String>;
/// A fresh name that passes `check_name` (std only, for example `RandomState`).
pub fn new_name() -> String;
/// Pure: `Ok` only when `socket` is absolute, has only normal components, lies under
/// `root` (by components) and is shorter than `SOCKET_PATH_LIMIT` bytes.
pub fn check_socket(root: &Path, socket: &Path) -> Result<(), String>;
/// Pure: read `herdr status server --json`'s output; `Ok(socket)` only when its
/// `session` is `name` and its `socket` passes `check_socket(root, ..)`.
pub fn prove(status_json: &str, name: &str, root: &Path) -> Result<PathBuf, String>;
/// Pure: the whole environment of every `herdr` the harness runs, built from the
/// inherited one: no `HERDR_*`, `TMUX` or `TMUX_PANE`; `HOME`, `XDG_CONFIG_HOME`,
/// `XDG_STATE_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_RUNTIME_DIR` inside
/// `root`; `SHELL=/bin/sh`; everything else as inherited.
pub fn scratch_env(root: &Path, inherited: impl IntoIterator<Item = (OsString, OsString)>)
    -> Vec<(OsString, OsString)>;
/// The opt-in, first statement of each ignored test: `None` (after printing one line
/// starting `skipped (HOLLER_HERDR_SCRATCH is not 1)`) unless the gate is `Run`; then
/// the absolute path of `herdr` on `PATH`, or a panic naming `herdr` and `PATH`.
pub fn opt_in() -> Option<PathBuf>;

/// One scratch Herdr server: its root, its session, its proven socket, its child process.
pub struct ScratchHerdr { /* private */ }
impl ScratchHerdr {
    /// Start a server, prove it (Decision 4), and return once its socket is proven.
    /// Any refusal or failure panics with the reason.
    pub fn start(herdr: &Path) -> Self;
    pub fn session(&self) -> &str;
    pub fn socket(&self) -> &Path;
}
/// Stops only this server, then removes its root (Decision 5). Never panics.
impl Drop for ScratchHerdr { /* ... */ }
```

`crates/holler-adapter-herdr/tests/scratch_herdr_test.rs` holds the two ignored tests and the default-run tests of the
harness (AC 1-8). The workspace label in both ignored tests is `holler640-grid`, configured as 2 rows by 1 column.

## Acceptance criteria

Each is observable: a test name with what it asserts, a command and its expected output, or a grep. "Default run" means
`cargo test -p holler-adapter-herdr` with no variable set, which is what CI runs.

**A. The harness's guards, proven in the default run** (pure functions; no process, socket, file or env read)

1. `the_gate_runs_only_on_exactly_1`: `gate(Some("1")) == Gate::Run`; `gate(None)`, `gate(Some(""))`, `gate(Some("0"))`,
   `gate(Some("true"))`, `gate(Some("1 "))` and `gate(Some("yes"))` are each `Gate::Skip`.
2. `the_name_guard_refuses_the_default_session_by_name`: `check_name("default")` is `Err` whose text contains
   `"default"`; so is `check_name("Default")`.
3. `the_name_guard_accepts_only_holler640_and_8_hex`: `Ok` for `holler640-0123abcd`; `Err` for `""`, `holler640-`,
   `holler640-0123abc` (7), `holler640-0123abcde` (9), `holler640-0123ABCD`, `holler640-0123abcg`, ` holler640-0123abcd`,
   `holler640-0123abcd/x`, `spike636-0123abcd` and `scratch`.
4. `generated_names_pass_the_guard_and_differ`: 64 calls of `new_name()` each pass `check_name`, and all 64 differ.
5. `the_socket_guard_keeps_the_socket_inside_the_root`, with `root = /r/h640.ab` (literal paths, no file is touched):
   `Ok` for `/r/h640.ab/home/.config/herdr/sessions/holler640-0123abcd/herdr.sock`; `Err` for
   `/r/h640.abc/herdr.sock` (a sibling sharing the prefix), `/r/other/herdr.sock`, `h640.ab/herdr.sock` (relative),
   `/r/h640.ab/../x/herdr.sock` and `/r/h640.ab/./herdr.sock` (non-normal components), the root itself, and a path under
   the root of 100 bytes or more.
6. `the_server_is_proven_by_its_own_status`, with `root = /r/h640.ab` and `name = holler640-0123abcd`: `prove` returns
   `Ok(<that socket>)` for `{"session":"holler640-0123abcd","socket":"/r/h640.ab/home/.config/herdr/sessions/holler640-0123abcd/herdr.sock","running":true}`
   (extra fields ignored), and `Err` for: `"session":"default"` (the text contains `default`), another `holler640-`
   name, a socket outside the root, a missing `session`, a missing `socket`, a non-string `socket`, and `not json`.
7. `the_scratch_env_carries_nothing_of_the_live_herdr`: given an inherited environment holding
   `HERDR_SOCKET_PATH=/live/herdr.sock`, `HERDR_SESSION=default`, `HERDR_SOMETHING_NEW=x`, `TMUX=/tmp/tmux-1/default,1,0`,
   `TMUX_PANE=%1`, `HOME=/home/someone`, `XDG_CONFIG_HOME=/home/someone/.config`, `XDG_RUNTIME_DIR=/run/user/1`,
   `PATH=/usr/bin:/bin` and `LANG=C.UTF-8`, `scratch_env(Path::new("/r/h640.ab"), ..)`:
   - holds no name starting `HERDR_`, and neither `TMUX` nor `TMUX_PANE`;
   - holds no value containing `/live`, `/home/someone` or `/run/user`;
   - sets each of `HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_RUNTIME_DIR` to a
     path under `/r/h640.ab`, each exactly once;
   - sets `SHELL=/bin/sh`, and keeps `PATH=/usr/bin:/bin` and `LANG=C.UTF-8` unchanged.
8. **One way to run `herdr`.** `grep -rn 'Command::new' crates/holler-adapter-herdr/` prints exactly one line, in
   `tests/scratch_herdr/mod.rs`. S reads that function: its arguments always start `--session <name>` with the name
   checked by `check_name`, it calls `env_clear()` and then sets exactly `scratch_env(root, std::env::vars_os())`, and it
   bounds the child (Decision 3). `grep -rn 'std::env\|env::var' crates/holler-adapter-herdr/tests/` matches only
   `tests/scratch_herdr/mod.rs`.
   Both greps print nothing at `dc300ab` (E-12, baseline), so only new code can match them. That code is constrained
   as follows. `scratch_herdr_test.rs`, `adapter_messages_test.rs` and `wire_herdr/**` spawn no process and read no
   environment. The ignored tests call `opt_in()`, which reads `GATE_VAR` inside `scratch_herdr/mod.rs`. AC 7 builds
   its inherited environment from literal pairs, not from `std::env`. AC 10's `env -i` is the shell's command, run by T
   outside the code.

**B. The gating, proven without a server**

9. **Ignored by default.** `cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --list --ignored` lists
   exactly `scratch_herdr_passes_the_conformance_suite` and
   `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane`; each carries
   `#[ignore = "opt-in: a scratch Herdr server; set HOLLER_HERDR_SCRATCH=1 (#640)"]`. The default run reports
   `2 ignored` for that target and runs neither. These are the only `#[ignore` lines in the crate
   (`grep -rn '#\[ignore' crates/holler-adapter-herdr/` prints exactly those two).
10. **Unset gate, `--ignored`.** Run the test binary with an empty environment, so neither the gate nor `PATH` exists:
    `env -i <the scratch_herdr_test binary> --ignored --nocapture` (T finds the binary under `target/debug/deps/`) passes
    both tests, and the output has one line starting `skipped (HOLLER_HERDR_SCRATCH is not 1)` per test. Since there is
    no `PATH`, this also shows that the gate runs before any lookup or spawn. In the code, `opt_in()` is the first
    statement of each ignored test (S reads it).
11. **Gate set, no `herdr`.** `env -i HOLLER_HERDR_SCRATCH=1 <the binary> --ignored` fails both tests, each with a
    message containing `herdr` and `PATH`. Asked to run, the test must not pass without having run.

**C. Against a real scratch Herdr** (run opt-in by T on the pipeline's machine at RED and at GREEN, Decision 7; never in CI)

The command is `HOLLER_HERDR_SCRATCH=1 cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --ignored
--test-threads=1`.

12. `scratch_herdr_passes_the_conformance_suite`: `run_herdr_conformance` returns `Ok(())`, where `fresh` starts one
    `ScratchHerdr` per case, connects `HerdrAdapter::connect(HerdrConfig::new(<its session>, <its socket>)
    .with_workspace("holler640-grid", Extent { rows: 2, cols: 1 }))` (the existing builder, `adapter.rs:67-71`, E-8),
    and returns the server as the case's guard (the shape of `adapter_conformance_test.rs:52-63`). All 11 cases hold.
13. `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane` (the issue's line), against one `ScratchHerdr`,
    workspace `holler640-grid` (2 by 1), in this order:
    1. `connect` succeeds; `version()` is a non-empty string on one line (the test prints it).
    2. **Create:** `a = ensure_pane(r1c1)` returns `HerdrPane { session: <scratch>, workspace: "holler640-grid", grid: r1c1, .. }`,
       and the snapshot's panes of that workspace are exactly `[a]`.
    3. **Run:** `send_text(a, "printf 'holler640-%s\n' ran")` (the `\n` is two characters, backslash and `n`, so the
       shell's `printf` prints the newline) then `send_keys(a, [enter])`. Polling `read(a, 50)` finds, within 10 s, a
       line that, trimmed, equals `holler640-ran`. The typed command line itself cannot match it.
    4. **Send:** `send_text(a, "holler640-typed")` with no Enter. Polling `read(a, 50)` finds, within 10 s, a line
       containing `holler640-typed`.
    5. `b = ensure_pane(r2c1)`. The snapshot's panes of `holler640-grid` are `a` at `r1c1` and `b` at `r2c1`, in any
       order, and nothing else. Every pane in the snapshot carries the scratch session.
    6. **Close:** `close(b)` is `Ok`. The snapshot's panes of the workspace are exactly `[a]` (an id check; `a`'s cell
       may grow). `close(b)` again and `read(b, 1)` are each `pane-not-found`.

    Polling (steps 3 and 4) waits 100 ms between reads and gives up at 10 s with a message quoting how many lines the
    last read returned and nothing of the screen.

    Step 6's two `pane-not-found` answers, like conformance case 9, rest on wire-fake behaviour that is INFERRED, not
    VERIFIED (E-9). This run is the first check against real Herdr. A difference is an AC 15 gap, not a reason to
    loosen the step.
14. **After the run.** T records in `handoff-T-red.md` and `handoff-T-green.md`: the `herdr --version` line, the
    duration of each test, `pgrep -f h640\.` printing nothing, and no `h640.*` directory left in the scratch base
    (Decision 2). If either check fails, that is a blocking issue.
15. **A real-Herdr gap is fixed here, not worked around.** If a case in AC 12 or 13 fails because real Herdr differs
    from the wire fake, T writes a default-run test that reproduces it through the wire fake (changing the fake to
    match real Herdr and marking that behaviour VERIFIED, with the scratch run as the evidence), and that test is RED.
    F then fixes the adapter in this crate's `src/`. A difference that needs a `holler-pane` contract change stops the
    run with `escalate` (Decision 8).

**D. The adapter's two follow-ups** (default run, in process, through the wire fake's `Tap`; a new test file)

16. `a_timeout_names_the_port_method`: through a `Tap` whose hook answers one chosen wire method with
    `Tapped::Fail(PaneError::Timeout { op: format!("herdr.{method}") })` and forwards everything else, each call gives
    exactly `Err(PaneError::Timeout { op: <the port op> })`:

    | Call | Wire method that times out | `op` returned |
    |---|---|---|
    | `connect_with` | `ping` | `herdr.connect` |
    | `ensure_pane(r1c1)`, workspace missing | `session.snapshot`; `workspace.create`; the confirming `layout.export` | `herdr.ensure_pane` |
    | `ensure_pane(r2c1)`, `r1c1` placed | the first `layout.export`; `pane.split`; the second `layout.export` | `herdr.ensure_pane` |
    | `send_text` / `send_keys` / `read` / `close` | `pane.send_text` / `pane.send_keys` / `pane.read` / `pane.close` | `herdr.send_text` / `herdr.send_keys` / `herdr.read` / `herdr.close` |
    | `snapshot`, one workspace with a tab | `session.snapshot`; `layout.export` | `herdr.snapshot` |
    | `version` | `ping` | `herdr.version` |

    `herdr.connect` is the one string in this table with no test-kit counterpart. `connect` and `connect_with` are
    inherent methods of `HerdrAdapter`, not `HerdrPort` methods (E-8), so `HerdrOp` has no `Connect`. The adapter spells
    it in the same `herdr.<method>` form (Decision 9). The seven port-method strings equal `HerdrOp::as_str` (Reuse map).

17. `every_other_error_passes_through_unchanged`: with the same hook answering `Tapped::Fail(PaneError::Unavailable {
    what: "injected-unavailable".into() })` for the same wire methods, each call returns that error exactly
    (`assert_eq!`). Also, part 2's AC 17 still holds: a `pane-not-found` from `pane.split` is still `unavailable`.
18. **Herdr-sent text is cut to 64 characters.** Let `LONG = "x".repeat(64) + "-TAIL-NOT-QUOTED"`. In each of the four
    cases below, the error is `PaneError::Unavailable { what }` where `what` contains `format!("{:?}...", "x".repeat(64))`,
    does not contain `TAIL-NOT-QUOTED`, and has no `\n`:
    - `a_misplaced_new_panes_id_is_cut_to_64`: the `confirm` "landed elsewhere" arm, as in AC 16 of part 2
      (`adapter_test.rs:197-225`), with Herdr's id for the new pane being `LONG` in the replies of `pane.split` and of the
      confirming `layout.export`;
    - `a_new_pane_with_no_cell_has_its_id_cut_to_64`: the `confirm` "no cell" arm, with only `pane.split`'s reply naming
      `LONG`;
    - `a_vanished_split_targets_id_is_cut_to_64`: `changed_under`, with Herdr's tree naming the split target `LONG` and
      the split answered `pane_not_found`;
    - `a_tabless_workspaces_label_is_cut_to_64`: `grid_tab` via `ensure_pane`, the configured and Herdr's workspace label
      being `LONG` with no tab in the snapshot.

    How each reply is rewritten (a `Tapped::Reply` built from `fake.answer(&line)`, for example) is T's choice.
19. **One `excerpt`, shared.** `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints exactly
    `crates/holler-adapter-herdr/src/protocol.rs:<n>:pub(crate) fn excerpt(text: &str) -> String {`. In `adapter.rs`,
    `grep -nE 'made\.as_str\(\)|target\.as_str\(\)|workspace\.label' crates/holler-adapter-herdr/src/adapter.rs` shows each
    remaining use as an argument of `excerpt(..)`, never of a `{:?}` placeholder. Values from the caller
    (`spec.session`, `spec.workspace`, `config.session`, `config.socket`, the labels in `validate`) keep `{:?}` (Decision 10).
20. **The transport is unchanged.** `git diff origin/main -- crates/holler-adapter-herdr/src/transport.rs` is empty.
    Part 2's transport tests still assert `herdr.ping` at the transport level and pass.

**E. The docs** (F's edits, in place; each grep is run from the repo root)

21. `holler-pane/src/ports.rs`, `Key`: `grep -nE 'C-c|Enter' crates/holler-pane/src/ports.rs` prints nothing (today it
    prints line 101), and the doc reads as D1 of Decision 11.
22. `holler-pane/src/ports.rs`, `ensure_pane`: its doc contains `grid-out-of-range` and `grid-unreachable` (D2).
23. `holler-pane/src/error.rs`: the `GridOutOfRange` doc contains `outside the Herdr workspace's extent` (D3); the
    `Timeout` doc contains `<port>.<method>` (D4).
24. `grep -rnE '#640 records it|recorded by the Herdr adapter' crates/ docs/adr/` prints nothing (D5, D6, A1).
25. ADR-0021 (A1-A9 of Decision 12): each "old" sentence is gone and each "new" one is present. Specifically,
    `grep -c 'grid-unreachable' docs/adr/ADR-0021.md` prints at least `4`, `grep -n "How the adapter learns a workspace's extent is #640's" docs/adr/ADR-0021.md`
    prints nothing, `grep -n 'is read on connect, recorded as' docs/adr/ADR-0021.md` prints nothing, and
    `grep -n '<port>.<method>' docs/adr/ADR-0021.md` prints the §9 `timeout` row. No other line of the ADR changes
    (`git diff --stat` shows only the lines of A1-A9).
26. `docs/testing.md` gains the section of Decision 13, naming `HOLLER_HERDR_SCRATCH`, the command of section C, and the
    rule that CI never runs it.
27. `CHANGELOG.md` `[Unreleased]` / `### Enhancements`: one entry right after the part-2 entry, linking #640 and #633
    (Decision 14). `bash scripts/changelog-check.sh` reports ok.

**F. Gates and shape**

28. All pass: `cargo test -p holler-adapter-herdr`, `cargo test -p holler-pane`, `cargo test --workspace` (CI is the final
    word; a local failure that is environmental, such as `holler-cli --test logging_test` finding a live hub, is shown
    passing with an isolated `HOLLER_STATE_DIR`), `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo fmt --check -p holler-adapter-herdr -p holler-pane`, `bash scripts/lint.sh`, `cargo machete`.
29. Part 2's ACs 35-37 and 39 still print nothing. Part 2's AC 38 is narrowed to production code, since the harness must
    read `GATE_VAR` and strip `HERDR_*`:
    `grep -rnE "TcpStream|TcpListener|47001|47002|tmux|std::env|env::var|\.config/herdr|HERDR_|unsafe" crates/holler-adapter-herdr/src crates/holler-adapter-herdr/Cargo.toml`
    prints nothing, and the same pattern over `crates/holler-adapter-herdr/tests/` matches only
    `tests/scratch_herdr/mod.rs` and `tests/scratch_herdr_test.rs`.
    The pattern prints nothing over the whole crate at `dc300ab` (E-12, baseline). `scratch_herdr_test.rs` matches
    through AC 7's literal inputs (`HERDR_SOCKET_PATH`, `HERDR_SESSION`, `HERDR_SOMETHING_NEW`, the value
    `/tmp/tmux-1/default,1,0`) and its assertions on them, not through any env read (AC 8). `adapter_messages_test.rs`,
    `wire_herdr/**`, `common/mod.rs` and part 2's test files match nothing.
30. Dependencies: `cargo tree -p holler-adapter-herdr -e normal --depth 1` lists only `holler-pane` and `serde_json`. No
    dependency line changes in any manifest; only the `tempfile` comment in `Cargo.toml:26-27` grows to name the
    scratch root.
31. Every touched file is under 900 lines (`wc -l`). No function trips `too_many_lines` (100) or `cognitive_complexity`
    (15).
32. **No platform-sensitive pattern** in the new tests (Risks): no socket option, no assertion on an `io::ErrorKind`
    from a socket or process call, no fixed sleep used for readiness, and every path compared after
    `fs::canonicalize` (macOS's `/tmp` is a link to `/private/tmp`). The default-run tests of section A touch no file.
    The canonicalize rule covers paths taken from the real file system, all on the opt-in path: the scratch base and
    root (Decision 2) and the proven socket (Decision 4). `check_socket` and `prove` stay pure. They compare by
    components only, call no `fs` function, and get paths that are either already canonical (from `ScratchHerdr::start`)
    or literal (the section A tests). S checks this rule by reading `tests/scratch_herdr/mod.rs` and
    `tests/scratch_herdr_test.rs`. The patterns listed here are the ones it looks for.

## Files (blast radius)

The issue's blast radius is `crates/holler-adapter-herdr/**`. Parts 1 and 2 put the `holler-pane` doc edits and the
ADR rows in part 3 by name (E-6, E-2), so this part also edits doc comments in `holler-pane`, ADR-0021, `docs/testing.md`
and `CHANGELOG.md`. No code outside `holler-adapter-herdr` changes.

| File | Change | Who | Est. lines |
|---|---|---|---|
| `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs` | new: the harness (pinned items above) | T | ~260 |
| `crates/holler-adapter-herdr/tests/scratch_herdr_test.rs` | new: AC 1-7 (default run), AC 12-13 (ignored) | T | ~260 |
| `crates/holler-adapter-herdr/tests/adapter_messages_test.rs` | new: AC 16-18 | T | ~230 |
| `crates/holler-adapter-herdr/tests/wire_herdr/mod.rs` | `Tapped::Fail(PaneError)`, and any real-Herdr fidelity fix (AC 15) | T | +10 |
| `crates/holler-adapter-herdr/Cargo.toml` | the `tempfile` comment names the scratch root (comment only) | T | +1 |
| `crates/holler-adapter-herdr/src/adapter.rs` | the `op` rule (Decision 9), `excerpt` at the four Herdr-sent quotes (Decision 10), module doc | F | ~+30 |
| `crates/holler-adapter-herdr/src/protocol.rs` | `fn excerpt` becomes `pub(crate) fn excerpt` (visibility only) | F | 1 |
| `crates/holler-adapter-herdr/src/*.rs` | only for an AC 15 fidelity fix, if the scratch run finds one | F | 0 expected |
| `crates/holler-pane/src/ports.rs` | doc comments D1, D2 | F | ~+6 |
| `crates/holler-pane/src/error.rs` | doc comments D3, D4 | F | ~+4 |
| `crates/holler-pane/src/pane.rs`, `crates/holler-pane/src/reconcile.rs` | doc comments D5, D6 | F | 2 |
| `docs/adr/ADR-0021.md` | A1-A9 (Decision 12), edited in place | F | ~12 changed |
| `docs/testing.md` | one section (Decision 13) | F | ~+14 |
| `CHANGELOG.md` | one entry (Decision 14) | F | +8 |

**Not touched:** `crates/holler-adapter-herdr/src/transport.rs` (AC 20); `holler-pane`'s code (only doc comments change);
`holler-pane-testkit/**` (its `ASSUMPTION (#640)` comments are a follow-up, "For the operator", item 4); `holler-cli/**`;
part 2's existing test files (`adapter_test.rs`, `transport_test.rs`, `adapter_conformance_test.rs`) and
`tests/common/mod.rs`; every other crate.

## Reuse map (extend, do not duplicate)

| Need | Existing object | Recommendation |
|---|---|---|
| Quote Herdr-sent text | `protocol::excerpt` (`protocol.rs:580`) | **Use**, made `pub(crate)`. No third copy. `holler_pane::error::excerpt` stays `pub(crate)` in the frozen crate (a public one is an amend-first change, out of scope). |
| Inject a reply, a rewrite or an error into one exchange | `wire_herdr::Tap`, `Tapped` (`wire_herdr/mod.rs:212-251`) | **Extend** with one variant, `Tapped::Fail(PaneError)`. No second interceptor (part 2's A finding 5). |
| A simulated Herdr for the default-run tests | `wire_herdr::WireHerdr` | **Use** as is; change it only to match real Herdr (AC 15). |
| The real-server conformance run | `run_herdr_conformance`, `HerdrFixture` (`conformance/herdr.rs`) | **Run it**, as `adapter_conformance_test.rs:52-63` does. Do not copy a case. |
| Isolating a real Herdr | `scripts/spikes/herdr-lib.sh` (E-9) | **Port the pattern** to Rust test code: the temp root, the `HERDR_*` removal, the explicit `--session`, the proof from `status server --json`, stop only the own PID. The script stays; the tests do not call it (it needs `jq` and `python3`, and it is bash). |
| A scratch directory | `tempfile` (dev-dependency) | **Use** `tempfile::Builder::new().prefix(ROOT_PREFIX).tempdir_in(<base>)`. |
| JSON parsing in `prove` | `serde_json` (normal dependency) | **Use**. |
| The port op names | `FakeHerdr`'s `HerdrOp::as_str` (`herdr.rs:68-74`) | The adapter's strings for the seven port methods must **equal** them (`herdr.<port method>`). The test kit's `HerdrOp` is not public API to import from `src/`, so the adapter spells them; AC 16 pins each. `herdr.connect` (`connect`, `connect_with`) is the documented exception. Those are not port methods and have no `HerdrOp` (E-8), so the string is the adapter's own, in the same form (Decision 9, AC 16). |

Placement of the harness: it is the first Rust code that starts a real Herdr. #649 and #667 will need a scratch Herdr
too. It stays in this crate's tests (blast radius) as one self-contained module with no dependency on the rest of the
crate's tests, so a later story can move it (to `holler-pane-testkit`, for example) rather than copy it. That decision is
#649's ("For the operator", item 5).

## Decisions already made (MO)

1. **Part 3 of 3, closing.** This brief replaces part 2's at `docs/handoffs/640-brief.md`. The PR says `Closes #640` and
   carries the AI disclosure `CONTRIBUTING.md` requires.
2. **The scratch root.** The base is `fs::canonicalize(std::env::temp_dir())` when that path is at most 40 bytes long,
   else `fs::canonicalize("/tmp")` (`herdr-lib.sh:21-23`; macOS's `TMPDIR` is long, and `/tmp` is a link). The root is a
   fresh `tempfile` directory `h640.XXXXXX` in it, canonicalized. Inside it: `home/` (with `home/.config/herdr/config.toml`,
   the spike's config without the sidebar rows: `onboarding = false`, `[terminal] default_shell = "/bin/sh"`,
   `[update] version_check = false`, `manifest_check = false`), `home/.local/state`, `home/.local/share`, `home/.cache`,
   `run/` (mode `0700`, `XDG_RUNTIME_DIR`), `work/` (the server's working directory), and `server.log` (its stdout and
   stderr). The session is `new_name()`.
3. **Every `herdr` invocation goes through one function** (AC 8): absolute path from `opt_in()`, arguments
   `--session <name> ...`, `env_clear()` then `scratch_env(root, std::env::vars_os())`, current directory `work/`,
   stdin null. A short command (`status server --json`, `server stop`) is bounded at 5 s: spawn, poll `try_wait` every
   50 ms, `kill` and `wait` on expiry. The server itself is `--session <name> server`, spawned and kept as the guard's
   child.
4. **Start and prove.** `ScratchHerdr::start` spawns the server, then polls every 100 ms for at most 15 s:
   - the child has exited: panic naming its exit status;
   - else run `status server --json`; when it exits 0 and `prove(stdout, name, root)` is `Ok(socket)` and that socket
     exists, compare `fs::canonicalize(socket)` with the proven path (equal, or panic) and return.

   On timeout, panic saying the server did not prove itself in 15 s. Polling for a condition with a pause is not a
   fixed sleep for readiness. The adapter only ever gets the proven socket.
5. **Stop only this server.** `Drop` runs `--session <name> server stop` (bounded, errors ignored), polls `try_wait` for
   at most 10 s, then `kill()` and `wait()` the child if it is still alive (its own PID only: `Child::kill` targets that
   process), and lets the `TempDir` remove the root. It never panics: a failure is one `eprintln!`. This is the
   spike's `spike_server_stop` (E-9). The harness never runs `session stop`, `session delete` or `server stop` without
   its own checked `--session`, and it never starts tmux.
6. **The gate.** `opt_in()` reads `GATE_VAR` with `std::env::var_os` (not `set_var`: no test mutates the environment).
   Not exactly `"1"`: print one line and return `None`, and the test returns (passes) having touched nothing. Exactly
   `"1"` and no `herdr` on `PATH` (searched by hand: each `PATH` entry joined with `herdr`, a file, executable):
   panic naming `herdr` and `PATH`. This is stricter than part 1's outline ("skips ... when `herdr` is not on PATH"),
   on purpose: an operator who asked for the real check must not get a pass for a check that did not run.
7. **T runs the scratch test on the pipeline's machine**, at RED (on the merged adapter, to find real-Herdr gaps before
   F starts) and at GREEN, with `--test-threads=1`. It is safe there for the reasons the spike was: the `HOME`/`XDG_*`
   root and the stripped `HERDR_*` make even Herdr's default session resolve inside the root, every call names the
   checked scratch session, and the guard stops only its own child. No other agent runs `herdr`. The operator's live
   Herdr, tmux sessions and OpenCode servers are never touched (epic rule, E-1). If the operator vetoes this ("For the
   operator", item 3), AC 12-14 become the operator's own check before merge.
8. **Real-Herdr gaps** (AC 15). Each difference the scratch run finds is fixed in this part when the fix stays inside
   `holler-adapter-herdr`: T makes the wire fake match real Herdr (that behaviour is then VERIFIED) and adds a
   default-run test that fails; F fixes `src/`. A gap that needs a change to `holler-pane` (the port, a type, a code) or
   to the test kit stops the run with `escalate`, naming the case and the real reply. No case is skipped or loosened.
9. **`timeout`'s `op` names the port method** (this changes part 2's Decision 3 at the adapter boundary only):
   - Every `PaneError::Timeout` a `HerdrPort` method returns has `op = "herdr.<port method>"`: `herdr.ensure_pane`,
     `herdr.send_text`, `herdr.send_keys`, `herdr.read`, `herdr.close`, `herdr.snapshot`, `herdr.version`.
     `connect` and `connect_with` give `herdr.connect`.
   - The adapter replaces the `op` of any `Timeout` from an exchange of that call, and changes no other error (AC 17).
   - The transport keeps `herdr.<wire method>` (AC 20). It is the transport's own vocabulary, under the adapter.
   - Why: since part 2 merged, #642's OpenCode adapter (`harness.<method>`) and #641's host adapter (`host.<method>`)
     both use the port method, as every fake of the test kit does (`PortOp`), and #647's reconcile names its calls the
     same way (E-5). The Herdr adapter was the one exception. A verb that reports a timeout (doctor's `observe-failed`,
     #644's exit 1) now says the same thing over the fake and over the real adapter. No test pins the old form at the
     adapter level (E-5), and the wire step that ran out is not lost from anything a caller could act on: the port
     call is what the verb retries or reports.
   - How F does it is F's choice (for example, the trait impl calls a private method and maps its error once), as
     long as each port method has one `op` and no `Timeout` escapes unrenamed.
10. **Herdr-sent text goes through `excerpt`** (part 2's A-dup finding 1, its recommended fix). `protocol::excerpt`
    becomes `pub(crate)`, with no other change to `protocol.rs`. `adapter.rs` quotes through it the four values that
    Herdr sent: `made` in both arms of `confirm`, `target` in `changed_under`, and `workspace.label` in `grid_tab`. The
    caller's values keep `{:?}`: the crate's rule (E-4) is about what Herdr sent, and they are Holler's own input,
    still one line. This kills part 1's surviving `EXCERPT_LIMIT` mutant through AC 18.
11. **The `holler-pane` doc edits** (doc comments only; no signature, derive, attribute or code line changes):
    - **D1** `ports.rs:100-101`, `Key`: "One key to press with `HerdrPort::send_keys`, by the name Herdr uses (for example
      `enter` or `ctrl+c`). It goes to Herdr as written: nothing case-folds or translates it."
    - **D2** `ports.rs:127-128`, `ensure_pane`: keep the sentence, then add: "An occupied cell answers its pane and makes
      none. A cell outside the workspace's rows and columns (Herdr has none, so they are the implementation's
      configuration) is `grid-out-of-range`; a cell that no single right or down split reaches is the open code
      `grid-unreachable`."
    - **D3** `error.rs:414-415`, `GridOutOfRange`: keep both sentences, then add: "Also, from `HerdrPort::ensure_pane`, a
      cell outside the Herdr workspace's extent; `what` then names the cell and the extent. (#640.)"
    - **D4** `error.rs:454-455`, `Timeout`: "`timeout`: an operation did not return within the bound of I5 (default
      10 s); `op` names the port method, as `<port>.<method>` (`herdr.ensure_pane`). (#638-#642.)"
    - **D5** `pane.rs:112`: "The Herdr API version, as `HerdrPort::version()` reports it, recorded by a verb (the Herdr
      adapter writes no record; ADR-0021 section 10)."
    - **D6** `reconcile.rs:105-106`: "One host of the panes in scope, as their records name it. Doctor shows the recorded
      Herdr API version and writes neither field (a verb records it, ADR-0021 section 10)."

    F may reflow a line to rustfmt's width or fix grammar, not change the meaning. `ports.rs:118-119`'s
    "**Provisional** until spike #636 reports" stays: whether `HerdrPort` is final is the epic's call while the
    unplaced-pane question is open (A9).
12. **The ADR-0021 edits**, each in place, each named here (the ADR's own rule: amend sentences, do not append a
    changelog). Old text is E-7's.
    - **A1** §1, line 40, the `host` row's note: "`herdr_api_version` is the string `HerdrPort::version()` reports,
      recorded by a verb, never by the Herdr adapter, which writes no record (section 10)."
    - **A2** §9, line 339, `pane launch`, `pane relaunch`: append to the codes "; open (#640) for a cell that no single
      split reaches, `grid-unreachable`".
    - **A3** §9, line 350, `profile apply`: add `grid-out-of-range` after `herdr-version-unsupported`, and append "; open
      (#640) for a cell that no single split reaches, `grid-unreachable`".
    - **A4** §9, line 396, the `timeout` class row's reason: "The I5 bound ran out while doing the work. Its `op` names
      the port method that ran out, as `<port>.<method>` (`herdr.ensure_pane`, `harness.health`), in every
      implementation and fake; an adapter's own wire steps do not appear in it."
    - **A5** §9, line 401: "the open codes planned so far (#645, #646) are all refusals" becomes "the open codes so far
      (#640's `grid-unreachable`, merged; #645's and #646's, planned) are all refusals".
    - **A6** §10, line 427: "How the adapter learns a workspace's extent is #640's." becomes "Herdr has no grid, so a
      workspace's extent is the Herdr adapter's configuration: rows by columns per workspace label
      (`HerdrConfig.workspaces`, #640), which the wiring supplies (#649). A cell inside the extent that no single right
      or down split reaches is the open code `grid-unreachable` (#640)."
    - **A7** §10, lines 434-436: "- Herdr API versions (B5): the Herdr adapter checks Herdr's protocol when it connects,
      so no adapter exists for an unknown one, and `HerdrPort::version()` checks it again on every call; the other
      methods trust the check made at connect (#640). An unknown version is `herdr-version-unsupported`, and the message
      names the supported ones (#640, from #636's list). The string `version()` returns is recorded as
      `host.herdr_api_version` by a verb, never by the adapter (**PROPOSED**: #644's launch and relaunch record it), and
      `doctor` shows it (#647)."
    - **A8** §11, line 451: "#640 and #646 name their permitted uses" becomes "#640 uses them only in its tests, which
      type into panes they created in a scratch Herdr session, #646 names the permitted uses".
    - **A9** "Deferred to named stories", line 537: "- `HarnessPort` in its final form: #635, then #642. `HerdrPort`:
      #640 implements it as merged; a pane with no cell (inside a nested split, or in another tab) has no place in
      `HerdrSnapshot`, which waits for a contract amendment before #647 and #650 need it."

    The ADR's `**Date:**` stays. No other ADR line changes.
13. **`docs/testing.md`**: a section "Opt-in: a scratch Herdr server (#640)" after "Beyond loopback: `interop.yml`"
    (about 14 lines): what the two tests check; the command of section C; `HOLLER_HERDR_SCRATCH=1` is required (any
    other value skips) and a missing `herdr` fails; what the isolation is (root, `HERDR_*` removed, `holler640-<hex>`
    session, the proof, stop own PID); CI never runs it; it needs `herdr` with protocol 22.
14. **CHANGELOG** (suggested text): "Herdr adapter, part 3 (epic #633): an opt-in test runs the `HerdrPort`
    conformance suite and a create, run, send, read, close and snapshot pass against a real, throwaway Herdr server
    (`HOLLER_HERDR_SCRATCH=1 cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --ignored`). It keeps
    Herdr's home and config in a temporary directory, removes every `HERDR_*` variable, and refuses any session but its
    own `holler640-<hex>` one, the default included. A Herdr call that runs out of time now reports `timeout` naming the
    port method (`herdr.ensure_pane`), as the other adapters do, and a message quotes at most 64 characters of what
    Herdr sent. Nothing is wired into a verb yet ([#649]), so nothing a user runs changes ([#640])."
15. **RED for this part.** No stubs are needed: every new behaviour is in code that exists.
    - AC 16-18 fail today on assertions: the `op` is the wire form, and the quote is the whole string.
    - AC 1-7 test the harness, which is test code. They pass at RED by construction, and T says so.
    - AC 9-11 are run by T at RED and pass. They check the gate, not F's work.
    - AC 12-13 are run by T at RED against the merged adapter. A pass means no gap. A failure starts AC 15.
    - The doc ACs 21-27 fail today (their old text is present).
    - T adds `Tapped::Fail` and writes every test. F writes no tests.
16. **Ordering in `scratch_herdr_test.rs`.** The two ignored tests use different servers and may run in parallel, but the
    documented command passes `--test-threads=1` so a slow machine starts one server at a time. Neither test depends on
    the other.

## Contradictions with the issue text (for S; none blocks the run)

1. **"passes #638's split-only-mode case"** (Acceptance, line 3). Part 1's Decision 6 refuses that case's `r2c2`-below-`r1c2`
   step as `grid-unreachable`, because real Herdr nests it (spike §7, VERIFIED). Part 1's S judged it "not a hold" and
   asked for the issue line to be amended before part 3 closes #640 (`5133b9a`, advisory 2). It has not been amended.
2. **"record it in `host.herdr_api_version` (doctor shows it)"** (Scope). The adapter writes no record (part 1's
   Decision 8; I1: the adapter holds no `PaneStore`). A7 and D5 say so, and the owner is PROPOSED ("For the operator",
   item 2).
3. **"an unknown one is refused"** (Acceptance, line 3). As part 2 read it and the operator's override kept it: refused at
   `connect` and `version()`. A7 writes that into the ADR.
4. **Rigor.** The issue says `in-session`; this part runs at second-opinion (header).
5. **Blast radius.** The issue lists only this crate. The doc edits outside it were assigned to part 3 by both earlier
   briefs (E-2, E-6), and no code outside the crate changes.

S audits against the issue as these readings state it. Items 1 and 2 need the issue text amended before the PR merges
("For the operator", item 1).

## Forward-compat

| Consumer | Needs from part 3 | Satisfied |
|---|---|---|
| #644 launch/relaunch | `grid-unreachable` in its ADR row; `timeout` with a port `op` to report | yes (A2, Decision 9). The `herdr_api_version` owner is PROPOSED (item 2) |
| #664 profile apply | `grid-out-of-range` and `grid-unreachable` in its row | yes (A3) |
| #647 doctor | `observe-failed` messages that read the same over the fake and the real adapter | yes (Decision 9; `herdr.snapshot`, `herdr.version`) |
| #649 wiring, #667 apply scenario | a way to start a scratch Herdr from Rust | yes, as a movable test module (Reuse map; "For the operator", item 5) |
| #651 display plugin | nothing | n/a |

## Out of scope

- Any change to `HerdrPort`, its types, `PaneError`'s variants or `holler-pane` code (only doc comments change).
- Making `holler_pane::error::excerpt` public (amend-first; item 4).
- The test kit's `ASSUMPTION (#640)` comments (`herdr.rs:26-47`, `:187-190`, `:272`, `:295`, `:318-320`, `:358-360`;
  `conformance/herdr.rs:49-53`, `:189-196`, `:337-339`), which this part answers but does not edit (#638's crate; item 4),
  and the split-only tightening of `FakeHerdr` (part 2's Decision 20).
- `HerdrSnapshot` for unplaced panes (A9 records it; a contract amendment).
- Recording `host.herdr_api_version` (a verb's; item 2), wiring (#649), `events.subscribe`, an argv launch without a
  shell, discovering a socket from `herdr session list` in production code.
- Running the scratch test in CI, or on any machine but the pipeline's, by T.

## Test plan

**RED** (Decision 15). T writes `tests/scratch_herdr/mod.rs`, `tests/scratch_herdr_test.rs`,
`tests/adapter_messages_test.rs` and `Tapped::Fail`, and updates the `Cargo.toml` comment. Then:

- `cargo test -p holler-adapter-herdr` compiles. Part 1 and part 2 tests stay green. AC 1-7 pass (stated). AC 16-18 fail
  on their assertions (T lists each failing assertion: the wire `op` seen, the full `LONG` in the message).
- AC 9-11 are run and pass. AC 12-14 are run opt-in against the merged adapter, and the result is recorded, with each
  failing case and Herdr's reply, which starts AC 15.
- The doc-AC greps (21-27) show the old text.

**GREEN.** F changes `adapter.rs`, `protocol.rs` (one word) and the docs. T re-runs everything:

- the default run;
- AC 9-11;
- AC 12-14 opt-in;
- the gates (AC 28-32).

T records two mutants in `handoff-T-green.md`: (a) `EXCERPT_LIMIT` 64 becomes 65, which must fail all four AC 18 tests;
(b) the `op` rename removed, which must fail AC 16 and no other new test.

Timing: no default-run test waits on time. The opt-in tests bound every wait (5 s per CLI call, 15 s start, 10 s stop,
10 s per poll) and assert only upper bounds.

## Risks

- **Touching the operator's Herdr.** This is the risk that matters. Three guards stand between the harness and a live
  server, each proven in the default run:
  - the environment (AC 7): no `HERDR_*`, and `HOME`/`XDG_*` inside the root, so Herdr's own default session resolves
    inside the root;
  - the name (AC 2-4): every call carries `--session holler640-<hex>`, and `default` is refused by name;
  - the proof (AC 5-6): the server must report that session and a socket inside the root before the adapter connects.

  The guard stops only its own child PID. The adapter itself has no default path and reads no environment variable
  (AC 29), so it can reach only the proven socket.
- **A wedged CLI or server.** Every `herdr` call is bounded and killed on expiry (Decision 3), and `Drop` cannot hang
  longer than 15 s or panic. An abandoned pane shell dies with its server's PTYs; AC 14's `pgrep` check catches one
  that does not.
- **macOS.** `TMPDIR` is long and `/tmp` is a link, so the base rule and `canonicalize` keep the socket path short and
  comparable (Decision 2, AC 32). Herdr on macOS may start login shells (spike §15). The read polls match a whole
  trimmed line, so a prompt or banner does not matter. The default-run tests are pure, so the macOS CI job runs them
  with no Herdr. No new test sets a socket option (the part-2 `EINVAL` lesson, E-12).
- **Real Herdr differs from the fake.** That is what the run is for. AC 15 turns each difference into a default-run
  test, and anything contract-level escalates.
- **The `op` change** reverses part 2's Decision 3 at the adapter boundary. It is cheap to reverse again (one mapping and
  AC 16), and the operator can veto it (item 6).
- **Flakiness of the opt-in run.** It is not in CI. The read polls are bounded, and the conformance cases are the
  suite's own.
- **File sizes.** `wire_herdr/mod.rs` goes from 650 to about 660 lines. The new files are split by topic (harness, its
  tests, adapter messages) to stay well under 900.

## Handoff locations

T-red: `docs/handoffs/640/handoff-T-red.md`. F: `.../handoff-F.md`. T-green: `.../handoff-T-green.md`.
A: `.../handoff-A.md`, `.../handoff-A-dup.md`. S: `.../handoff-S.md`. Journal: `docs/handoffs/640/decisions.md`.

## Operating rules

- Read this brief, the Evidence and the Reuse map before writing code. A second `excerpt`, a second exchange
  interceptor, or a copy of a conformance case is an anti-duplication BLOCK.
- Test-first: T writes every test, the harness and `Tapped::Fail`. F writes no tests.
- Only T runs `herdr`, and only through the harness with `HOLLER_HERDR_SCRATCH=1` (Decision 7). Nobody runs `herdr`
  by hand. Nobody attaches to, lists or stops another Herdr session, a tmux session, or an OpenCode server (the
  operator's ports 47001/47002 included).
- Conventional Commits (`feat(adapter-herdr): ...`, `docs(adr): ...` where a commit is docs only). The hooks add the
  trailer. Stage files by explicit path.
- The PR says `Closes #640` and carries the AI disclosure `CONTRIBUTING.md` requires.
- Public repo: no personal hostnames, IPs, user or machine names in code, tests, fixtures, messages or handoffs. A
  handoff that quotes a scratch path writes `<scratch-root>` for it, and `<home>` for a home directory.

## For the operator (review; only item 1 is needed before the PR merges)

1. **PROPOSED: amend issue #640** (outward-facing, so the brief does not do it), using the issue's own
   "(amended 2026-10-09, …)" marker:
   - Acceptance, line 3: "passes #638's split-only-mode case" becomes "passes the suite's split-only placement; a
     `down` split under a pane in a row of several is `grid-unreachable` (it nests in real Herdr; #640 part 1,
     Decision 6)".
   - Scope: "record it in `host.herdr_api_version` (doctor shows it)" becomes "return it from `version()` for a verb
     to record in `host.herdr_api_version` (doctor shows it); the adapter writes no record". Optionally also "the
     gate runs at connect and in `version()`" (part 2's item 4, which you ruled on 2026-10-09).

   Part 1's S asked for this before part 3 closes #640. Without it, part 3's S audits against an issue that contradicts
   two merged decisions.
2. **PROPOSED: who records `host.herdr_api_version`.** Today nobody does. #644's issue does not mention it, #647's doctor
   only shows it (`reconcile.rs:105-106`), and the adapter cannot write it. A7 marks #644 (launch and relaunch, which
   write `HostInfo`) as the PROPOSED owner. Say if it should be another story.
3. **T runs the scratch test on the pipeline's machine** (Decision 7). It starts one throwaway `herdr --session
   holler640-<hex> server` at a time, isolated as the spike was, and stops only that one. This follows your
   "use an isolated `--session`, never the human" practice. Say if you would rather run AC 12-14 yourself.
4. **Unfiled follow-ups, for a #638 test-kit issue:** tighten `FakeHerdr`'s split-only mode (part 1's W-7, part 2's
   Decision 20), and resolve its `ASSUMPTION (#640)` comments now that #640 has answered them. Separately, make
   `holler_pane::error::excerpt` public in the next amend-first `holler-pane` change, so the adapters stop keeping
   copies (part 1's A-dup W-1). None is filed. Say if you want them.
5. **Where a shared scratch-Herdr harness lives** when #649 or #667 needs one (the test kit is the obvious home). This
   part keeps it movable.
6. **Decision 9 (`op` = port method) reverses an MO decision of part 2**, on evidence that arrived after it (#642's and
   #641's adapters). It is reversible. Say if you want the wire method kept. Then AC 16, D4 and A4 change to state the
   Herdr exception instead.

Needs operator before merge: item 1. Nothing blocks the pipeline from starting.
