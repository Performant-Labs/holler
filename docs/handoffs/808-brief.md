# [refactor(pane): a Herdr snapshot pane carries its label, the join key for import (amend-first for #650)](https://github.com/Performant-Labs/holler/issues/808)

Part of epic #633. This is the amend-first contract change that #650 (`holler pane import`) needs; #650's plan review blocked because nothing in the frozen contract joins a pane's **name** to its live Herdr pane.

## Operator decision (2026-10-11)

`holler pane import` finds the live Herdr pane that belongs to a pane name by the **Herdr pane label**. Not by the name's grid cell (the epic forbids it: `hj-c1r2` is a name, not a position), not by the foreground process, not by a pane id read from a launcher file (a file-derived fact, I6, and ids do not survive a Herdr restart). Herdr's `PaneInfo` carries `label?`, and labels persist across Herdr restarts (spike #636). The setup wizard's `herdr.sh label` verb already sets them.

## Scope

Amend the provisional `HerdrPort` / `HerdrSnapshot` contract, in its own PR (ADR-0021 section 2: "amended first, in their own PR"):

1. **`holler-pane`:** a pane Herdr reports now carries its label. Put it on the type the snapshot returns (`HerdrSnapshot.panes` is `Vec<HerdrPane>`, `crates/holler-pane/src/pane.rs:95` and `ports.rs:99`) as `label: Option<String>`. Constraints, which decide between "a field on `HerdrPane`" and "a separate snapshot-entry type": (a) **the persisted record format does not change** (`panes.json` written before this change still reads, and a record written by `launch` is byte-identical to today's; use `#[serde(default, skip_serializing_if = "Option::is_none")]` if the field is on `HerdrPane`); (b) **no reconcile or doctor behavior changes** because a snapshot entry now has a label (anything that compares a record's `HerdrPane` with a snapshot entry must ignore the label); (c) the label is an **observation** (I6): `launch` and `relaunch` do not write one.
2. **`holler-adapter-herdr`:** read `label` from `pane.list` / `pane.get` (the `PaneInfo` shape) and report it in `snapshot()` and in what `ensure_pane` returns. A pane with no label reports `None`. An empty-string label reports `None`.
3. **`holler-pane-testkit`:** `FakeHerdr` can be given a label for a pane (a test hook, like its other recordings) and reports it in `snapshot()`; its conformance suite gets a case (fake and wire-fake only) that a labelled pane is reported with its label and an unlabelled one with `None`.
4. **ADR-0021:** sections 1 and 2 (the `HerdrPane`/`HerdrSnapshot` types and the "provisional" note), section 10 (names are not positions; the label is the join key from a name to a live pane), and the Deferred bullet at lines 740-741 (the snapshot amendment "before #647 and #650 need it": this is that amendment). Record the decision above.
5. **A pane with no cell** (inside a nested split, or in another tab) still has no place in the snapshot's grid. State in the ADR that it is **out of scope here and a finding for the importer** (#650 reports it and records nothing), so #650 does not have to invent a rule.

Out of scope: `holler pane import` itself (#650), `pane.process_info`, `cwd`, `title`, and any use of the label outside import.

## Acceptance (tests first)

- A `HerdrSnapshot` entry reports `label: Some("x")` for a pane Herdr labels `x`, and `None` for no label and for an empty label (adapter test against the wire fake, and fake test).
- A `panes.json` written before this change deserializes unchanged, and a record written by `launch` serializes to the same bytes as before (a test over a stored fixture).
- `doctor` / reconcile output on a world where every Herdr pane is labelled is identical to the same world with no labels (a regression test over the existing doctor rig).
- The conformance case passes for `FakeHerdr` and the wire fake.
- ADR-0021 edits as above; `CHANGELOG.md` has an `## [Unreleased]` entry linking this issue.

**RED note for T:** the first tests construct a labelled pane through the new field or hook, so RED may be a compile error at the new API; that is valid for a contract amendment. T states this in `handoff-T-red.md` and also writes the behavior tests (the adapter reports the label, the stored fixture still reads, the doctor regression) so they fail on assertions once it compiles.

## Blast radius

`crates/holler-pane/src/{pane.rs,ports.rs,lib.rs}` and its tests; `crates/holler-adapter-herdr/**`; `crates/holler-pane-testkit/**`; call sites and doubles that build a `HerdrPane` in `crates/holler-cli/tests/**` (add `label: None` or use a constructor); `docs/adr/ADR-0021.md`; `CHANGELOG.md`. No hub, protocol or `holler-body` change.

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-<N>-implementation`. This amends different ports than #802 (`HerdrPort`, not `HarnessPort`) but both edit ADR-0021 and `CHANGELOG.md`; expect to merge `main` in before the squash.

## Depends on / blocks

Depends on nothing open. Blocks #650.

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.


## Clarifications after the first review run (2026-10-11) — these pin the points above and override them where they differ

1. **Where the label comes from.** The adapter's `snapshot()` (`crates/holler-adapter-herdr/src/adapter.rs:371`) builds each `HerdrPane` from the grid cells of a workspace's grid tab (`self.grid(tab, deadline)?.cells()`); it skips a workspace with no grid tab and reports no pane that has no cell. That behavior is **unchanged and is the answer for a pane with no cell**: it is not in the snapshot, so import (#650) reports it as a finding and records nothing. The label is read wherever Herdr reports it for a pane (the `pane.get`/`pane.list` `PaneInfo` `label`, or the session snapshot); the survey must establish which call from the spike (`docs/research/*herdr*`, #636) and from `crates/holler-adapter-herdr/src/protocol.rs`, and quote it. If reading labels needs one extra `pane.list` call per snapshot, that is acceptable; it must stay within the call's I5 bound.
2. **One coercion point.** A function in the adapter's protocol layer (`protocol.rs`), `label_of(raw: Option<&str>) -> Option<String>`, maps `None` and the empty string to `None` and anything else to `Some(text)` (no trimming). **Both** `snapshot()` and what `ensure_pane` returns use it, so the two can never disagree. A test calls it directly (`None`, `""`, `"x"`) and one test per caller proves the caller uses it.
3. **Names of the fakes.** "The wire fake" is `WireHerdr` in `crates/holler-adapter-herdr/tests/wire_herdr/mod.rs` (an in-process fake Herdr socket server used by the adapter's own tests). `FakeHerdr` is in `crates/holler-pane-testkit/src/herdr.rs`. The conformance case in `holler-pane-testkit/src/conformance/` runs against `FakeHerdr` (the testkit's own conformance test) and against the real adapter over `WireHerdr` (`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs`). The case needs a hook to label a pane, offered by `FakeHerdr` as a method and by `WireHerdr` as a field of the pane it serves; the conformance suite receives the hook as a closure parameter, as its other optional hooks are.
4. **The byte-pinned fixture.** The "record written by launch is unchanged" test compares against a **hard-coded literal JSON string committed in the test**, copied from a record produced by the code on `main` before this change (the survey records the commit it was produced at). It does not regenerate the fixture with the new code. A second test reads that literal and a version with `"label":"x"` and asserts both deserialize.
5. **ADR-0021 text, written out** (the PR edits exactly these places):
   - Section 2 (the ports table and the "`HerdrPort`, and the data types it takes and returns, stay provisional" note): add after the note, as a new sentence: "A pane Herdr reports carries its label (`HerdrPane.label`, an optional, observed value: `launch` and `relaunch` never write one, and no comparison of a record with a snapshot entry looks at it). It is the join key from a pane name to a live Herdr pane (#808, operator decision 2026-10-11)."
   - Section 10: add a bullet "A pane's name is not its position and not its identity in Herdr: the importer (#650) finds a pane's live Herdr pane by `HerdrPane.label == name`. A pane with no cell has no place in the snapshot and is a finding of the importer, never recorded."
   - "Deferred to named stories", the `HerdrPort` bullet (lines ~740-741, "waits for a contract amendment before #647 and #650 need it"): replace with "the label is added by #808; a pane with no cell stays out of the snapshot (the importer reports it)".
   - "Decisions taken": append "10. A Herdr snapshot pane carries its label, the join key for import (operator, 2026-10-11, #808)."
6. **`HerdrPane` equality.** If the field is on `HerdrPane` (which derives `PartialEq, Eq`), the brief names every place that compares a record's `HerdrPane` with a snapshot entry and shows each ignores the label; if any cannot, the label goes on a separate snapshot-entry type instead and `HerdrPane` stays unchanged. The survey decides by reading `reconcile.rs`, `observe.rs` and the doctor code.
7. **Evidence.** The reviewers cannot see the repo; the brief quotes the excerpts it relies on: `HerdrPane`/`HerdrSnapshot` (`pane.rs:93-100`, `ports.rs:96-101`), `HerdrPort::ensure_pane`/`snapshot`, `Adapter::snapshot`, and the `PaneInfo` parsing, with file and line.

## Survey and evidence (brief gate, round 1 amendment, 2026-10-11)

The outside review of round 1 blocked this brief because the survey that clarifications 1, 4, 6 and 7 call for was not in it. This section is that survey; the review's findings are named below by their ids (B-, NV-, W-, NIT-). Every source it relies on is quoted verbatim, with file and line, as of this branch's base, `main` at `92fbc6e4186acc717f4aa2b6465c55a266b9043c` (the branch's commits before T touch only `docs/handoffs/`). This section adds to the brief. It changes neither the operator's decision nor any scope item, acceptance criterion or clarification above; where a line number cited above is off, the right one is given here.

In short:

- `HerdrSnapshot.panes` is a `Vec<HerdrPane>`, and the persisted record `Pane` holds a `HerdrPane` as its `herdr` field (E1, E2).
- The label comes from the `PaneInfo` objects of `session.snapshot`, `pane.split` and `workspace.create`, which the adapter already requests: no new method and no new call (E4).
- Every comparison in reconcile and doctor ignores the label. One comparison in the launch engine would see it, and it can ignore it. By clarification 6's rule the label is therefore a field on `HerdrPane` (E5).
- Constraints (b) and (c) then need two changes in `crates/holler-pane/src/tx_launch.rs`, which the blast radius did not list. E9 adds them, and four tests, to the scope and the acceptance criteria.

### E1. `HerdrSnapshot` holds `HerdrPane`s (B-1, NV-1)

`HerdrSnapshot` has one field, `panes: Vec<HerdrPane>`. The struct is `crates/holler-pane/src/ports.rs:96-101` and the field is line 100 (scope item 1 cites line 99, the `struct` line). A field added to `HerdrPane` is therefore on every snapshot entry, with no other type between them.

`crates/holler-pane/src/ports.rs:96-101`
~~~~text
96| /// What Herdr reports about every pane it has.
97| #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
98| #[serde(deny_unknown_fields)]
99| pub struct HerdrSnapshot {
100|     pub panes: Vec<HerdrPane>,
101| }
~~~~

`crates/holler-pane/src/pane.rs:91-100`
~~~~text
91| /// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
92| /// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
93| #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
94| #[serde(deny_unknown_fields)]
95| pub struct HerdrPane {
96|     pub session: String,
97|     pub workspace: String,
98|     pub pane_id: PaneId,
99|     pub grid: GridPos,
100| }
~~~~

The two implementations the conformance suite runs against build every snapshot entry, and every `ensure_pane` answer, as a `HerdrPane` literal, so each sets the new field in exactly those places: the adapter at `crates/holler-adapter-herdr/src/adapter.rs:247-252` and `:381-386` (E3), and `FakeHerdr`:

`crates/holler-pane-testkit/src/herdr.rs:246-257`
~~~~text
246|     fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
247|         self.faults.enter(HerdrOp::EnsurePane)?;
248|         let mut state = self.lock();
249|         let placement = state.placement;
250|         let pane_id = state.workspace_mut(spec)?.place(spec.grid, placement)?;
251|         Ok(HerdrPane {
252|             session: spec.session.clone(),
253|             workspace: spec.workspace.clone(),
254|             pane_id,
255|             grid: spec.grid,
256|         })
257|     }
~~~~

`crates/holler-pane-testkit/src/herdr.rs:302-311`
~~~~text
302|     fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
303|         self.faults.enter(HerdrOp::Snapshot)?;
304|         let state = self.lock();
305|         let panes = state
306|             .workspaces
307|             .iter()
308|             .flat_map(|workspace| workspace.listed(&state.session))
309|             .collect();
310|         Ok(HerdrSnapshot { panes })
311|     }
~~~~

`crates/holler-pane-testkit/src/herdr.rs:479-487`
~~~~text
479|     /// Every pane of the workspace, by row and then by column, as `snapshot` lists it.
480|     fn listed<'a>(&'a self, session: &'a str) -> impl Iterator<Item = HerdrPane> + 'a {
481|         self.panes.iter().map(move |(&(row, col), pane)| HerdrPane {
482|             session: session.to_owned(),
483|             workspace: self.name.clone(),
484|             pane_id: pane.id.clone(),
485|             grid: GridPos { row, col },
486|         })
487|     }
~~~~

The other `HerdrPane` literals are in tests and in the test kit's fixture, all inside the blast radius above.

### E2. The persisted record is `Pane`, whose `herdr` field is a `HerdrPane` (B-2, NV-2)

`panes.json` is one document whose entries each carry a whole `Pane` record (`crates/holler-hub/src/panes/persist.rs:4-15`). `Pane.herdr` is a `HerdrPane` (`crates/holler-pane/src/pane.rs:224-233`), so a `label` field on `HerdrPane` is part of the persisted form. Records refuse unknown fields, and a `panes.json` holding a field the running build does not know fails to load (`pane.rs:4-9`, `persist.rs:30-37`). The established form of an optional record field is the one constraint (a) names, `#[serde(default, skip_serializing_if = "Option::is_none")]` (`pane.rs:8-9`; `HostInfo.herdr_api_version`, `pane.rs:112-115`). With it, a pre-change record (no `label` key) reads as `label: None`, and a record with `label: None` writes no `label` key, so its bytes are today's.

`crates/holler-hub/src/panes/persist.rs:4-15`
~~~~text
4| //! The pane registry's file is `<state dir>/hub/panes.json`, beside `holds.json`:
5| //!
6| //! ```text
7| //! {"version": 1, "cursor": <head>,
8| //!  "entries": [{"cursor": <c>, "name": "<pane>", "pane": {<the Pane record>} or null}, ...]}
9| //! ```
10| //!
11| //! - `cursor` is the change feed's head, so cursors keep increasing across a restart.
12| //! - An entry is the last change filed under a name, a `PaneEvent`: a put carries the record
13| //!   and a delete leaves a tombstone (`"pane": null`). A deletion therefore survives a
14| //!   restart, and a watcher that resumes across one still learns of it (`feed.rs`, rule 4).
15| //!   Entries are sorted by name.
~~~~

`crates/holler-hub/src/panes/persist.rs:30-37`
~~~~text
30| //! # Loading fails closed (D3)
31| //!
32| //! A missing file is an empty registry. Anything else that keeps the file from being read
33| //! back exactly is a [`Problem`]:
34| //!
35| //! - an unreadable file;
36| //! - a document that does not parse, including a record with a field this build does not
37| //!   know (the records refuse unknown fields);
~~~~

`crates/holler-pane/src/pane.rs:4-9`
~~~~text
4| //! The JSON field names are the contract: #639 persists these forms, #649 reads them
5| //! over the control socket. The records refuse unknown fields (`deny_unknown_fields`):
6| //! a record that is read, changed and written back by a hub or a client that does not
7| //! know a field would otherwise drop that field silently; refusing is loud and
8| //! fails closed. Optional fields default to absent on read and are left out when
9| //! they are `None`. Every timestamp is milliseconds since the Unix epoch as `i64`.
~~~~

`crates/holler-pane/src/pane.rs:112-115`
~~~~text
112|     /// The Herdr API version, as `HerdrPort::version()` reports it, recorded by a verb
113|     /// (the Herdr adapter writes no record; ADR-0021 section 10).
114|     #[serde(default, skip_serializing_if = "Option::is_none")]
115|     pub herdr_api_version: Option<String>,
~~~~

`crates/holler-pane/src/pane.rs:224-233`
~~~~text
224| /// One record per pane, owned by the hub. The key is [`Pane::name`]. Every write is
225| /// a compare-and-swap on [`Pane::generation`] (see [`crate::generation`]).
226| #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
227| #[serde(deny_unknown_fields)]
228| pub struct Pane {
229|     pub name: PaneName,
230|     /// Bumped on every change.
231|     pub generation: u64,
232|     pub herdr: HerdrPane,
233|     pub host: HostInfo,
~~~~

**What constraint (c) requires of the launch engine.** The record's `herdr` is exactly the `ensure_pane` answer: `bring_up` takes it (`crates/holler-pane/src/tx_launch.rs:616`), carries it as `Live.herdr` (`:643-648`), and `Plan::record` writes `herdr: live.herdr` (`:327`). Scope item 2 makes that answer carry the pane's label. A relaunch in place gets the pane already in the cell (an occupied cell answers its pane, `crates/holler-adapter-herdr/src/adapter.rs:221-231`), and that pane may carry a label: the operator decision notes that the setup wizard's `herdr.sh label` verb already sets them. So `Plan::record` must write `label: None` for launch and relaunch to write no label (constraint (c)). Without it, that relaunch's record would carry the label: its bytes would not be today's (constraint (a)), and a build without the field would refuse to load `panes.json` (`persist.rs:30-37`).

`crates/holler-pane/src/tx_launch.rs:615-619`
~~~~text
615|     budget.check()?;
616|     let herdr = ports.herdr.ensure_pane(&plan.target)?;
617|     if !seen.listed.iter().any(|p| same_pane(p, &herdr)) {
618|         made.pane = Some(herdr.pane_id.clone());
619|     }
~~~~

`crates/holler-pane/src/tx_launch.rs:643-648`
~~~~text
643|     Ok(Live {
644|         herdr,
645|         pid,
646|         session,
647|         shown,
648|     })
~~~~

`crates/holler-pane/src/tx_launch.rs:311-327`
~~~~text
311|     /// The record R writes: what the act brought up and observed, and the spec's values. A
312|     /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
313|     /// profile, its profile.
314|     fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
315|         let (hold, driven, profile) = match self.old {
316|             Some(old) => (
317|                 old.hold.clone(),
318|                 old.last_observed.driven.clone(),
319|                 self.profile.or(old.profile.as_ref()).cloned(),
320|             ),
321|             None => (Hold::None, None, self.profile.cloned()),
322|         };
323|         let spec = self.spec;
324|         Pane {
325|             name: self.name.clone(),
326|             generation: self.old.map_or(0, |old| old.generation),
327|             herdr: live.herdr,
~~~~

### E3. Where the adapter builds a `HerdrPane` (NV-3, NV-8, NV-9)

`snapshot()`, in full: one `session.snapshot`, then for each workspace with a grid tab one `layout.export` of that tab, and one `HerdrPane` per placed cell. `cells()` returns `Vec<(GridPos, PaneId)>`, the placed panes only. A pane in a split nested inside a cell sits in an `Unplaced` slot, and a pane in any tab but the grid tab (the workspace's tab with the lowest `number`) is in no tree `snapshot()` reads. Neither is in the snapshot. That is clarification 1's "a pane with no cell", and this PR leaves it so.

`crates/holler-adapter-herdr/src/adapter.rs:371-390`
~~~~text
371|     fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
372|         self.run_as(OP_SNAPSHOT, |deadline| {
373|             let state = parse_snapshot(&self.call(&Request::SessionSnapshot, deadline)?)?;
374|             let mut panes = Vec::new();
375|             for workspace in &state.workspaces {
376|                 // A workspace with no tab has no grid, so no pane of it has a cell.
377|                 let Some(tab) = &workspace.grid_tab else {
378|                     continue;
379|                 };
380|                 let cells = self.grid(tab, deadline)?.cells();
381|                 panes.extend(cells.into_iter().map(|(grid, pane_id)| HerdrPane {
382|                     session: self.config.session.clone(),
383|                     workspace: workspace.label.clone(),
384|                     pane_id,
385|                     grid,
386|                 }));
387|             }
388|             Ok(HerdrSnapshot { panes })
389|         })
390|     }
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:24-26`
~~~~text
24| //! - **`snapshot` lists every workspace**, configured or not, by its label: the placed
25| //!   panes of each workspace's grid tab, by row and then column. A pane nested inside
26| //!   one cell, or in another tab, has no cell and is left out.
~~~~

`crates/holler-adapter-herdr/src/layout.rs:62-68`
~~~~text
62| enum Slot {
63|     /// A pane, at the slot's row and column.
64|     Pane(PaneId),
65|     /// A slot whose panes have no position: a split nested inside the row's cell, or a
66|     /// slot past the last row or column a `GridPos` can count. Its panes are in
67|     /// [`GridMap::unplaced`].
68|     Unplaced,
~~~~

`crates/holler-adapter-herdr/src/layout.rs:98-103`
~~~~text
98|     /// The placed panes, by row then column.
99|     pub fn cells(&self) -> Vec<(GridPos, PaneId)> {
100|         self.placed()
101|             .map(|(cell, pane)| (cell, pane.clone()))
102|             .collect()
103|     }
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:27-30`
~~~~text
27| //! - **The grid tab** ([`SessionState::workspace`]). A workspace is found by its `label`,
28| //!   which must be unique, and its grid is the tree of its tab with the lowest `number`.
29| //!   A tab's `number` is a display ordinal, not an id, so reordering a workspace's tabs
30| //!   (`tab.move`) changes which tab that is.
~~~~

`ensure_pane` is `ensure` under one deadline. `ensure` reads the workspace through `session.snapshot` and `layout.export` (`workspace_grid`), then either answers an occupied cell's pane and changes nothing, or makes one pane by `workspace.create` (read by `parse_workspace_created`) or `pane.split` (read by `parse_pane_info`). It returns one `HerdrPane` literal. That literal (`adapter.rs:247-252`) and the one in `snapshot()` (`adapter.rs:381-386`) are the two places clarification 2's `label_of` feeds. `label_of` itself is new code, so there is nothing of it to quote yet.

`crates/holler-adapter-herdr/src/adapter.rs:327-330`
~~~~text
327| impl<T: Transport> HerdrPort for HerdrAdapter<T> {
328|     fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError> {
329|         self.run_as(OP_ENSURE_PANE, |deadline| self.ensure(spec, deadline))
330|     }
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:193-209`
~~~~text
193|     /// The workspace labelled `label` as Herdr has it now. Two of that label are
194|     /// `unavailable`.
195|     fn workspace_grid(&self, label: &str, deadline: Instant) -> Result<WorkspaceGrid, PaneError> {
196|         let state = parse_snapshot(&self.call(&Request::SessionSnapshot, deadline)?)?;
197|         let Some(workspace) = state.workspace(label)? else {
198|             return Ok(WorkspaceGrid {
199|                 tab: None,
200|                 map: GridMap::default(),
201|             });
202|         };
203|         let tab = grid_tab(workspace)?;
204|         let map = self.grid(&tab, deadline)?;
205|         Ok(WorkspaceGrid {
206|             tab: Some(tab),
207|             map,
208|         })
209|     }
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:211-253`
~~~~text
211|     /// `ensure_pane` against `deadline`: plan the one step that makes `spec.grid` (an
212|     /// occupied cell needs none), run it, and confirm the new pane sits there.
213|     fn ensure(&self, spec: &HerdrSpec, deadline: Instant) -> Result<HerdrPane, PaneError> {
214|         let extent = self.extent_of(spec)?;
215|         let workspace = self.workspace_grid(&spec.workspace, deadline)?;
216|         let target = Target {
217|             extent,
218|             cells: vec![spec.grid],
219|         };
220|         let pane_id = match plan_splits(&workspace.map, &target)?.as_slice() {
221|             // An occupied cell: its pane, and no request that changes anything.
222|             [] => workspace
223|                 .map
224|                 .at(spec.grid)
225|                 .cloned()
226|                 .ok_or_else(|| PaneError::Unavailable {
227|                     what: format!(
228|                         "the plan for {} is empty, and workspace {:?} has no pane there",
229|                         spec.grid, spec.workspace
230|                     ),
231|                 })?,
232|             [step] => {
233|                 let (tab, made) = self.place(step, &workspace, spec, deadline)?;
234|                 self.confirm(&tab, &made, spec, deadline)?;
235|                 made
236|             }
237|             steps => {
238|                 return Err(PaneError::Unavailable {
239|                     what: format!(
240|                         "the plan for {} has {} steps, and ensure_pane makes one pane",
241|                         spec.grid,
242|                         steps.len()
243|                     ),
244|                 })
245|             }
246|         };
247|         Ok(HerdrPane {
248|             session: self.config.session.clone(),
249|             workspace: spec.workspace.clone(),
250|             pane_id,
251|             grid: spec.grid,
252|         })
253|     }
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:265-271`
~~~~text
265|             Step::CreateRoot => {
266|                 let request = Request::WorkspaceCreate {
267|                     label: spec.workspace.clone(),
268|                 };
269|                 let (created, root) = parse_workspace_created(&self.call(&request, deadline)?)?;
270|                 Ok((grid_tab(&created)?, root))
271|             }
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:286-294`
~~~~text
286|                 let request = Request::Split {
287|                     target: target.clone(),
288|                     direction: *direction,
289|                     ratio: *ratio,
290|                 };
291|                 let reply = self
292|                     .call(&request, deadline)
293|                     .map_err(|error| changed_under(error, target, spec))?;
294|                 Ok((tab.clone(), parse_pane_info(&reply)?))
~~~~

### E4. Where the label comes from: the `PaneInfo` Herdr already sends (clarification 1; NV-4, NV-5)

**The spike** (`docs/research/herdr-api-spike.md`; its section 4 rows are VERIFIED unless marked, line 120):

- `session.snapshot` returns `panes[PaneInfo]` (line 134), and `PaneInfo` carries `label?` (line 137). An absent optional field is omitted, never null (lines 137 and 432).
- `pane.split` returns `pane_info` with a `PaneInfo` (line 126), and `workspace.create` returns `root_pane: PaneInfo` (line 124).
- `pane.rename {pane_id, label}` sets a label, and the label persists across a restart (line 140).
- The spike read labels back from `session.snapshot`'s panes: the grid probe sets them with `pane rename` and reads `label` off each of the snapshot's `panes` (`scripts/spikes/herdr-grid.sh:15`, `:19-28`), and its VERIFIED output lists them (lines 183-191).

`docs/research/herdr-api-spike.md:120`
~~~~text
120| equivalent `herdr --session <s> ...` command. All rows are VERIFIED by `herdr-ops.sh` unless marked.
~~~~

`docs/research/herdr-api-spike.md:124`
~~~~text
124| | Create a workspace | yes | `workspace.create {cwd?, label?, env?, focus?}`; `workspace create --cwd D --label L --no-focus` | `{type: "workspace_created", workspace: WorkspaceInfo, tab: TabInfo, root_pane: PaneInfo}` | ids `w<N>`, `w<N>:t<M>`, `w<N>:p<M>`, never reused |
~~~~

`docs/research/herdr-api-spike.md:126`
~~~~text
126| | Split | yes | `pane.split {target_pane_id, direction: "right"\|"down", ratio?, cwd?, env?, focus?}`; `pane split ID --direction right --ratio 0.25 --no-focus` | `{type: "pane_info", pane: PaneInfo}`; new pane id in `.result.pane.pane_id` | no `command` param: the new pane runs the default shell. `left`/`up` are `invalid_request` |
~~~~

`docs/research/herdr-api-spike.md:134`
~~~~text
134| | Snapshot | yes | `session.snapshot {}`; `api snapshot` | `{type: "session_snapshot", snapshot: {version, protocol, workspaces[], tabs[], panes[PaneInfo], layouts[{workspace_id, tab_id, zoomed, area, focused_pane_id, panes[{pane_id, focused, rect{x,y,width,height}}], splits[{id, direction, ratio, rect}]}], agents[], focused_*}}` | split `id`s (`split_3_011`) encode a tree path and are renumbered on relayout: not stable |
~~~~

`docs/research/herdr-api-spike.md:137`
~~~~text
137| | Pane info | yes | `pane.get {pane_id}`, `pane.list {workspace_id?}`; `pane get ID` | `PaneInfo`: `pane_id, terminal_id, workspace_id, tab_id, focused, cwd, foreground_cwd, label?, title?, agent?, display_agent?, agent_status, agent_session?, state_labels?, tokens?, scroll, revision, restore_error?` | absent optional fields are omitted, not null |
~~~~

`docs/research/herdr-api-spike.md:140`
~~~~text
140| | Rename (label) | yes | `pane.rename {pane_id, label}`; `pane rename ID LABEL` | `pane_info` | label persists across restart |
~~~~

`docs/research/herdr-api-spike.md:432`
~~~~text
432| - Optional fields are omitted rather than null (`agent`, `label`, `title`, `tokens`, `tty`).
~~~~

`scripts/spikes/herdr-grid.sh:15`
~~~~text
15| label() { h pane rename "$1" "$2" >/dev/null; }
~~~~

`scripts/spikes/herdr-grid.sh:19-28`
~~~~text
19| derive() { # derive TAB_ID
20|   local snap tree
21|   snap="$(h api snapshot)"
22|   tree="$(hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"tab_id\":\"$1\"}}")"
23|   python3 -I -c '
24| import json, sys
25| snap = json.loads(sys.argv[1])["result"]["snapshot"]
26| tree = json.loads(sys.argv[2])["result"]["layout"]["root"]
27| tab = sys.argv[3]
28| labels = {p["pane_id"]: p.get("label") for p in snap["panes"]}
~~~~

`docs/research/herdr-api-spike.md:183-191`
~~~~text
183| pane    label        herdr rect {x,y,w,h}     rank (row,col) tree (row,col)
184| w1:p1   r1c1         {0,0,30,20}              r1c1           r1c1
185| w1:p3   r1c2         {30,0,30,20}             r1c2           r1c2
186| w1:p4   r1c3         {60,0,30,20}             r1c3           r1c3
187| w1:p5   r1c4         {90,0,30,20}             r1c4           r1c4
188| w1:p2   r2c1         {0,20,30,20}             r2c1           r2c1
189| w1:p6   r2c2         {30,20,30,20}            r2c2           r2c2
190| w1:p7   marker       {60,20,30,20}            r2c3           r2c3
191| w1:p8   r2c4         {90,20,30,20}            r2c4           r2c4
~~~~

**The adapter today** (`crates/holler-adapter-herdr/src/protocol.rs`). It has no `PaneInfo` type. It may call nine methods, and `pane.list` and `pane.get` are not among them (lines 44-45, 94-107); a test fails any conformance run that sends a method off that list (`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:80-98`). It already parses every pane of `session.snapshot` (`parse_snapshot`, `pane_ref`) and the `PaneInfo` of `pane.split` and `workspace.create`, reading ids only, and its parsers ignore every field they do not read (lines 17-19).

`crates/holler-adapter-herdr/src/protocol.rs:17-19`
~~~~text
17| //! - **Parsers.** Each accepts one result `type`, and refuses any other, or a field it
18| //!   reads that is missing or of the wrong type, as `unavailable`. They ignore every
19| //!   field they do not read, so a field Herdr adds changes nothing.
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:44-45`
~~~~text
44| /// The number of methods the adapter may call.
45| const METHOD_COUNT: usize = 9;
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:94-107`
~~~~text
94|     /// Herdr's name for the method.
95|     const fn as_str(self) -> &'static str {
96|         match self {
97|             Method::Ping => "ping",
98|             Method::SessionSnapshot => "session.snapshot",
99|             Method::LayoutExport => "layout.export",
100|             Method::WorkspaceCreate => "workspace.create",
101|             Method::PaneSplit => "pane.split",
102|             Method::PaneSendText => "pane.send_text",
103|             Method::PaneSendKeys => "pane.send_keys",
104|             Method::PaneRead => "pane.read",
105|             Method::PaneClose => "pane.close",
106|         }
107|     }
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:343-356`
~~~~text
343| /// A pane of a Herdr snapshot.
344| #[derive(Debug, Clone, PartialEq, Eq)]
345| pub struct PaneRef {
346|     pub pane_id: PaneId,
347|     pub workspace_id: String,
348|     pub tab_id: String,
349| }
350|
351| /// What `session.snapshot` reports, reduced to what the adapter reads.
352| #[derive(Debug, Clone, PartialEq, Eq)]
353| pub struct SessionState {
354|     pub workspaces: Vec<WorkspaceRef>,
355|     pub panes: Vec<PaneRef>,
356| }
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:386-405`
~~~~text
386| /// Read a `session_snapshot` result.
387| pub fn parse_snapshot(result: &Value) -> Result<SessionState, PaneError> {
388|     let snapshot = result_of(result, "session_snapshot")?.object("snapshot", "snapshot")?;
389|     let tabs = snapshot
390|         .list("tabs", "snapshot tab")?
391|         .into_iter()
392|         .map(tab_ref)
393|         .collect::<Result<Vec<_>, _>>()?;
394|     let workspaces = snapshot
395|         .list("workspaces", "snapshot workspace")?
396|         .into_iter()
397|         .map(|workspace| workspace_ref(workspace, &tabs))
398|         .collect::<Result<_, _>>()?;
399|     let panes = snapshot
400|         .list("panes", "snapshot pane")?
401|         .into_iter()
402|         .map(pane_ref)
403|         .collect::<Result<_, _>>()?;
404|     Ok(SessionState { workspaces, panes })
405| }
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:438-445`
~~~~text
438| /// Read one pane of a snapshot.
439| fn pane_ref(pane: Object<'_>) -> Result<PaneRef, PaneError> {
440|     Ok(PaneRef {
441|         pane_id: PaneId::new(pane.string("pane_id")?),
442|         workspace_id: pane.string("workspace_id")?.to_owned(),
443|         tab_id: pane.string("tab_id")?.to_owned(),
444|     })
445| }
~~~~

`crates/holler-adapter-herdr/src/protocol.rs:485-503`
~~~~text
485| /// Read a `workspace_created` result: the workspace and its root pane.
486| pub fn parse_workspace_created(result: &Value) -> Result<(WorkspaceRef, PaneId), PaneError> {
487|     let created = result_of(result, "workspace_created")?;
488|     let workspace = created.object("workspace", "created workspace")?;
489|     let tab = created.object("tab", "created tab")?;
490|     let root = created.object("root_pane", "created root pane")?;
491|     let workspace = WorkspaceRef {
492|         workspace_id: workspace.string("workspace_id")?.to_owned(),
493|         label: workspace.string("label")?.to_owned(),
494|         grid_tab: Some(tab.string("tab_id")?.to_owned()),
495|     };
496|     Ok((workspace, PaneId::new(root.string("pane_id")?)))
497| }
498|
499| /// Read a `pane_info` result (what `pane.split` returns).
500| pub fn parse_pane_info(result: &Value) -> Result<PaneId, PaneError> {
501|     let pane = result_of(result, "pane_info")?.object("pane", "pane")?;
502|     Ok(PaneId::new(pane.string("pane_id")?))
503| }
~~~~

`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:80-98`
~~~~text
80| #[test]
81| fn no_case_calls_a_method_off_the_allow_list() {
82|     let mut fakes = Vec::new();
83|     let _ = run_in_process(false, &mut fakes);
84|     let _ = run_in_process(true, &mut fakes);
85|     let _ = run_over_a_socket(&mut fakes);
86|
87|     assert!(!fakes.is_empty());
88|     for fake in &fakes {
89|         let methods = fake.methods();
90|         assert!(!methods.is_empty(), "a case sent nothing");
91|         for method in methods {
92|             assert!(
93|                 ALLOWED_METHODS.contains(&method.as_str()),
94|                 "{method} is not on the allow-list"
95|             );
96|         }
97|     }
98| }
~~~~

**Survey result (clarification 1).** The label is read from the `PaneInfo` objects Herdr already sends in reply to calls the adapter already makes: the `panes` of `session.snapshot` (for `snapshot()`, and for `ensure_pane`'s answer about an occupied cell, since `ensure` has read `session.snapshot` in `workspace_grid`), and the `PaneInfo` of `pane.split` and of `workspace.create`'s `root_pane` (for a pane `ensure_pane` makes). No method joins the allow-list and no call is added, so clarification 1's "one extra `pane.list` call per snapshot" does not arise. Scope item 2's `pane.list` / `pane.get` are not called; clarification 1 already lets the survey choose "the session snapshot" instead. (`layout.export`'s pane nodes also carry `label?`, spike line 135; the survey keeps to `PaneInfo`, the shape scope item 2 and the operator decision name.) Round 2: today the adapter parses these `panes` and then drops them, so the label still has to be threaded to each `HerdrPane`; E10 pins how, with no new call.

**I5 (NV-5).** A port call takes one deadline on entry, I5's 10 s unless configured, and every exchange of the call runs against it (`crates/holler-adapter-herdr/src/adapter.rs:7-9`, `:50-51`, `:134-146`); `snapshot()` runs inside `run_as(OP_SNAPSHOT, ..)` (`:371-372`). Reading the label adds no exchange, so neither the bound nor the number of exchanges changes.

`crates/holler-adapter-herdr/src/adapter.rs:7-9`
~~~~text
7| //! - **One deadline per call.** Each port method takes its deadline once, on entry,
8| //!   `timeout` from then (I5's bound, [`DEFAULT_TIMEOUT`] unless configured), and every
9| //!   exchange of the call runs against that one deadline.
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:50-51`
~~~~text
50| /// I5's bound for one `HerdrPort` call.
51| pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
~~~~

`crates/holler-adapter-herdr/src/adapter.rs:134-146`
~~~~text
134|     /// Run `body` as the call `op`: against the one deadline of the call, taken now, and
135|     /// with a `timeout` from any exchange of it named `op`, where the transport names the
136|     /// wire method that ran out. Every other error passes unchanged.
137|     fn run_as<V>(
138|         &self,
139|         op: &'static str,
140|         body: impl FnOnce(Instant) -> Result<V, PaneError>,
141|     ) -> Result<V, PaneError> {
142|         body(self.deadline()?).map_err(|error| match error {
143|             PaneError::Timeout { .. } => PaneError::Timeout { op: op.to_owned() },
144|             other => other,
145|         })
146|     }
~~~~

### E5. Every comparison of a `HerdrPane` with a snapshot entry (clarification 6; B-3, W-1, W-2)

**Method.** Every caller of `HerdrPort::snapshot` and `HerdrPort::ensure_pane` outside tests and the test kit (`grep -rn 'snapshot()\|ensure_pane(' crates/*/src`), and all of `reconcile.rs`, `reconcile/observe.rs` (clarification 6's `observe.rs`) and the doctor code. Outside tests and the test kit the snapshot is read in two places only, reconcile (`crates/holler-pane/src/reconcile.rs:262`) and the launch engine (`crates/holler-pane/src/tx_launch.rs:292`, `:685`), and `ensure_pane` is called only by the launch engine (`tx_launch.rs:616`). `crates/holler-hub/src` does not mention `herdr` at all (`grep -rni herdr` finds nothing): the hub stores records whole.

| # | Site | What it compares | The label |
|---|---|---|---|
| 1 | `crates/holler-pane/src/reconcile.rs:326-347`, `missing_herdr_panes` | each record with the snapshot, by the key `(session, pane_id)`; the finding and its message are built from the record | ignored |
| 2 | `crates/holler-pane/src/reconcile.rs:349-369`, `unregistered_herdr_panes` | each snapshot entry with the records, by the key `(session, pane_id)`; the finding copies only the entry's `pane_id` and `grid` (`crates/holler-pane/src/findings.rs:269-276`), and its message quotes `pane_id`, `session` and `workspace` | ignored |
| 3 | `crates/holler-pane/src/reconcile/observe.rs:365-374`, `record` | the record with its own clone (`next == *pane`), never with a snapshot entry | equal by construction |
| 4 | `crates/holler-cli/src/pane/doctor.rs:55-61` | nothing: doctor runs `reconcile` and prints its `Report`, whose types hold no `HerdrPane` (`crates/holler-pane/src/reconcile.rs:75-125`; `Finding`, `crates/holler-pane/src/findings.rs:211-239`, holds `herdr_pane: Option<PaneId>` and `grid`) | not present |
| 5 | `crates/holler-pane/src/tx_launch.rs:441-458`, `refuse_occupied` | each snapshot entry's `session`, `workspace` and `grid` with the target; `same_pane` with the relaunched pane's record | ignored |
| 6 | `crates/holler-pane/src/tx_launch.rs:486-490`, `occupant` | each record with an occupant, by `same_pane` | ignored |
| 7 | `crates/holler-pane/src/tx_launch.rs:617`, `bring_up` | the plan's snapshot entries with the `ensure_pane` answer, by `same_pane` | ignored |
| 8 | `crates/holler-pane/src/tx_launch.rs:685`, `observe_live` (O2) | `snapshot()?.panes.contains(herdr)`: the whole `HerdrPane`, by its derived `PartialEq`, against the `ensure_pane` answer the record is written from | **compared, as written** |
| 9 | `crates/holler-pane/src/tx_launch.rs:726-728`, `close_old` | the record's `workspace` and `grid` with the target; `same_pane` | ignored |

`same_pane` is the join key, session and pane id (`tx_launch.rs:750-753`).

`crates/holler-pane/src/reconcile.rs:326-347`
~~~~text
326| /// `herdr-pane-missing` for each pane of `scope` whose Herdr session and pane id the
327| /// snapshot lacks.
328| fn missing_herdr_panes(snapshot: &HerdrSnapshot, scope: &[Pane]) -> Vec<Finding> {
329|     let live: BTreeSet<(&str, &PaneId)> = snapshot
330|         .panes
331|         .iter()
332|         .map(|herdr| (herdr.session.as_str(), &herdr.pane_id))
333|         .collect();
334|     scope
335|         .iter()
336|         .filter(|pane| !live.contains(&(pane.herdr.session.as_str(), &pane.herdr.pane_id)))
337|         .map(|pane| {
338|             let message = format!(
339|                 "Herdr has no pane {} in session {}, where the record puts this pane",
340|                 quoted(pane.herdr.pane_id.as_str()),
341|                 quoted(&pane.herdr.session)
342|             );
343|             Finding::about(FindingKind::HerdrPaneMissing, pane, message)
344|                 .with_herdr_pane(&pane.herdr)
345|         })
346|         .collect()
347| }
~~~~

`crates/holler-pane/src/reconcile.rs:349-369`
~~~~text
349| /// `unregistered-herdr-pane` for each pane of the snapshot that no record names.
350| fn unregistered_herdr_panes(snapshot: &HerdrSnapshot, records: &[Pane]) -> Vec<Finding> {
351|     let named: BTreeSet<(&str, &PaneId)> = records
352|         .iter()
353|         .map(|pane| (pane.herdr.session.as_str(), &pane.herdr.pane_id))
354|         .collect();
355|     snapshot
356|         .panes
357|         .iter()
358|         .filter(|herdr| !named.contains(&(herdr.session.as_str(), &herdr.pane_id)))
359|         .map(|herdr| {
360|             let message = format!(
361|                 "Herdr pane {} (session {}, workspace {}) is in no pane record; no holler verb adopts a pane",
362|                 quoted(herdr.pane_id.as_str()),
363|                 quoted(&herdr.session),
364|                 quoted(&herdr.workspace)
365|             );
366|             Finding::new(FindingKind::UnregisteredHerdrPane, message).with_herdr_pane(herdr)
367|         })
368|         .collect()
369| }
~~~~

`crates/holler-pane/src/findings.rs:269-276`
~~~~text
269|     /// The same finding, about the Herdr pane `herdr` and at its position.
270|     pub(crate) fn with_herdr_pane(self, herdr: &HerdrPane) -> Self {
271|         Self {
272|             herdr_pane: Some(herdr.pane_id.clone()),
273|             grid: Some(herdr.grid),
274|             ..self
275|         }
276|     }
~~~~

`crates/holler-pane/src/reconcile/observe.rs:365-374`
~~~~text
365|     let mut next = pane.clone();
366|     if let Some(health) = health {
367|         next.harness.health = health;
368|     }
369|     if let Screen::Seen(shown) = screen {
370|         next.last_observed.shown.clone_from(shown);
371|     }
372|     if next == *pane && pane.last_observed.at != 0 {
373|         return None;
374|     }
~~~~

`crates/holler-cli/src/pane/doctor.rs:55-61`
~~~~text
55|     let request = ReconcileRequest {
56|         profile: profile.as_ref(),
57|         pane: pane.as_ref(),
58|         fix: args.fix,
59|         now_ms: now_millis(),
60|     };
61|     reconcile(ports, &request)
~~~~

`crates/holler-pane/src/reconcile.rs:75-125`
~~~~text
75| /// What one reconcile pass found.
76| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
77| pub struct Report {
78|     pub scope: ScopeSummary,
79|     pub fix_requested: bool,
80|     pub herdr: HerdrSummary,
81|     /// The hosts the records in scope name, each once, sorted by name.
82|     pub hosts: Vec<HostSummary>,
83|     /// The panes in scope, sorted by name.
84|     pub panes: Vec<PaneSummary>,
85|     /// Sorted by pane (the findings about no pane first), then by kind in
86|     /// [`FindingKind::ALL`] order, then by session and Herdr pane.
87|     pub findings: Vec<Finding>,
88| }
89|
90| /// The scope the pass ran over: a profile's panes, one pane, both, or (neither) every pane.
91| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
92| pub struct ScopeSummary {
93|     pub profile: Option<ProfileName>,
94|     pub pane: Option<PaneName>,
95| }
96|
97| /// What Herdr reported about itself.
98| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
99| pub struct HerdrSummary {
100|     /// Herdr's API version; `None` when it was not observed or is unsupported (a finding
101|     /// says which).
102|     pub version: Option<String>,
103| }
104|
105| /// One host of the panes in scope, as their records name it. Doctor shows the recorded
106| /// Herdr API version and writes neither field (a verb records it, ADR-0021 section 10).
107| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
108| pub struct HostSummary {
109|     pub name: String,
110|     pub herdr_api_version: Option<String>,
111| }
112|
113| /// One pane in scope, as the pass observed it.
114| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
115| pub struct PaneSummary {
116|     pub name: PaneName,
117|     /// Where its record places it.
118|     pub grid: GridPos,
119|     pub session_of_record: Option<String>,
120|     /// The session the TUI showed at the end of the pane's chain (after a fix, the one it
121|     /// showed then). `None` is its home screen or no TUI, or that it was not observed (an
122|     /// `observe-failed` finding then names `harness.shown_session`).
123|     pub shown: Option<String>,
124|     pub health: ObservedHealth,
125| }
~~~~

`crates/holler-pane/src/findings.rs:211-239`
~~~~text
211| /// One finding of a reconcile pass.
212| #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
213| pub struct Finding {
214|     pub kind: FindingKind,
215|     /// The pane it is about; `None` for Herdr itself, a stray session or an unregistered
216|     /// Herdr pane.
217|     pub pane: Option<PaneName>,
218|     /// Where that pane is, as its record places it, or where the unregistered Herdr pane
219|     /// is.
220|     pub grid: Option<GridPos>,
221|     /// The Herdr pane it is about: the one Herdr lacks (`herdr-pane-missing`) or the one no
222|     /// record names (`unregistered-herdr-pane`).
223|     pub herdr_pane: Option<PaneId>,
224|     /// The session it is about: the stray, the missing session of record, the foreign one,
225|     /// or the one a mismatched TUI shows (`None` there is its home screen).
226|     pub session: Option<String>,
227|     /// The harness servers, by port and sorted, whose state or sessions it reports: every
228|     /// in-scope server a stray session was listed on, and the pane's own server for a
229|     /// wedged or down server, a missing session of record and a foreign session.
230|     pub ports: Vec<u16>,
231|     /// One line for a person; untrusted values in it are quoted.
232|     pub message: String,
233|     /// The command to run (see [`FindingKind::remedy`]); `None` when no Holler verb repairs
234|     /// it, or it was fixed.
235|     pub remedy: Option<String>,
236|     pub fix: FixState,
237|     /// Why the fix failed, when it did.
238|     pub fix_error: Option<FixError>,
239| }
~~~~

`crates/holler-pane/src/tx_launch.rs:289-294`
~~~~text
289|     fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
290|         let probe = probe(ports, self.spec, probe_timeout)?;
291|         let herdr_api_version = ports.herdr.version()?;
292|         let listed = ports.herdr.snapshot()?.panes;
293|         let own = self.old.map(|old| &old.herdr);
294|         refuse_occupied(ports, &listed, &self.target, own)?;
~~~~

`crates/holler-pane/src/tx_launch.rs:441-458`
~~~~text
441| /// Step 5: `grid-occupied` when the target cell holds a Herdr pane other than `own`. Whether the
442| /// run is refused is the snapshot's alone. The records are read only to word the refusal, joined
443| /// to each occupant by Herdr session and pane id whatever cell a record stores, so a stale record
444| /// is still named.
445| fn refuse_occupied(
446|     ports: Ports<'_>,
447|     listed: &[HerdrPane],
448|     target: &HerdrSpec,
449|     own: Option<&HerdrPane>,
450| ) -> Result<(), PaneError> {
451|     let at_target = |p: &&HerdrPane| {
452|         p.session == target.session && p.workspace == target.workspace && p.grid == target.grid
453|     };
454|     let occupants: Vec<&HerdrPane> = listed
455|         .iter()
456|         .filter(at_target)
457|         .filter(|p| own.is_none_or(|own| !same_pane(own, p)))
458|         .collect();
~~~~

`crates/holler-pane/src/tx_launch.rs:486-490`
~~~~text
486|     let names: Vec<&str> = records
487|         .iter()
488|         .filter(|record| same_pane(&record.herdr, pane))
489|         .map(|record| record.name.as_str())
490|         .collect();
~~~~

`crates/holler-pane/src/tx_launch.rs:663-665`
~~~~text
663| /// O1 and O2: the TUI shows the session of record (`shown_differs`, the one form of that
664| /// comparison), and Herdr still lists the pane at its cell. Either mismatch is `unavailable`
665| /// (decision 14). Returns what the TUI showed.
~~~~

`crates/holler-pane/src/tx_launch.rs:684-691`
~~~~text
684|     budget.check()?;
685|     if !ports.herdr.snapshot()?.panes.contains(herdr) {
686|         return Err(unavailable(format!(
687|             "the Herdr pane {} of {name} is gone from {}",
688|             quoted(herdr.pane_id.as_str()),
689|             cell(&herdr.workspace, herdr.grid)
690|         )));
691|     }
~~~~

`crates/holler-pane/src/tx_launch.rs:726-728`
~~~~text
726|     let moved = plan.target.workspace != old.herdr.workspace || plan.target.grid != old.herdr.grid;
727|     let listed = seen.listed.iter().any(|p| same_pane(p, &old.herdr));
728|     if !moved || !listed || same_pane(&old.herdr, now) {
~~~~

`crates/holler-pane/src/tx_launch.rs:750-753`
~~~~text
750| /// Whether two Herdr panes are one: the same Herdr session and pane id (the join key).
751| fn same_pane(a: &HerdrPane, b: &HerdrPane) -> bool {
752|     a.session == b.session && a.pane_id == b.pane_id
753| }
~~~~

Tests compare whole values too: the conformance suite's cases 3 and 4 compare `ensure_pane` answers with snapshot entries (`crates/holler-pane-testkit/src/conformance/herdr.rs:217-226`, `:231-244`). They label no pane, so every label in them is `None` and they are unchanged. A case that labels a pane and compares whole values holds only when the implementation's `ensure_pane` and `snapshot` report the same label for that pane.

`crates/holler-pane-testkit/src/conformance/herdr.rs:217-226`
~~~~text
217| fn ensure_is_idempotent(s: &Scratch<'_>) -> Result<(), String> {
218|     let first = s.ensure(R1C1)?;
219|     let second = s.ensure(R1C1)?;
220|     expect_eq("the second ensure_pane(r1c1)", &second, &first)?;
221|     expect_eq(
222|         "the workspace's panes after ensure_pane(r1c1) twice",
223|         s.panes()?,
224|         vec![first],
225|     )
226| }
~~~~

`crates/holler-pane-testkit/src/conformance/herdr.rs:231-244`
~~~~text
231| fn ensure_never_moves_another_pane(s: &Scratch<'_>) -> Result<(), String> {
232|     let a = s.ensure(R1C1)?;
233|     let b = s.ensure(R2C1)?;
234|     expect_eq(
235|         "ensure_pane(r1c1) after r2c1 was made",
236|         &s.ensure(R1C1)?,
237|         &a,
238|     )?;
239|     expect_eq(
240|         "the workspace's panes, in any order",
241|         by_id(s.panes()?),
242|         by_id(vec![a, b]),
243|     )
244| }
~~~~

**Survey result (clarification 6).** Rows 1 to 4, every comparison in `reconcile.rs`, `observe.rs` and the doctor code, ignore the label as the code stands, so reconcile and doctor cannot change because a snapshot entry has one (constraint (b)). Outside them one comparison would see it: row 8, the launch engine's O2 check. It can ignore the label by comparing the four fields `HerdrPane` has today (`session`, `workspace`, `pane_id`, `grid`) in place of the whole value, and constraint (b) says it must ("anything that compares a record's `HerdrPane` with a snapshot entry must ignore the label"). Every comparison can ignore the label, so by clarification 6's rule **the label is a field on `HerdrPane`** (`label: Option<String>`, `#[serde(default, skip_serializing_if = "Option::is_none")]`, constraint (a)), and no separate snapshot-entry type is made. That type would also leave `HerdrPane` unchanged, so `ensure_pane`, which returns a `HerdrPane`, could not report the label that scope item 2 and clarification 2 require of its answer. The acceptance criteria above are written for this design and stand as written (W-2).

`crates/holler-pane/src/ports.rs:135`
~~~~text
135|     fn ensure_pane(&self, spec: &HerdrSpec) -> Result<HerdrPane, PaneError>;
~~~~

Row 8 matters in practice: if a pane's label changes between the `ensure_pane` answer and the O2 snapshot (the operator or the wizard renames it meanwhile), the whole-value check fails the run `unavailable` and rolls it back though the pane sits where it should; the four-field check does not.

ADR-0021 (W-1): clarification 5's section-2 sentence, "no comparison of a record with a snapshot entry looks at it", is true of the code once row 8 compares the four fields, and this PR makes both changes. Its text is unchanged.

### E6. The fakes and the conformance suite (NV-6, NV-7, NIT-2)

**`WireHerdr`** is `crates/holler-adapter-herdr/tests/wire_herdr/mod.rs`, a simulated Herdr at the JSON level, written from the spike and never from `src/` (lines 1-7). The adapter's tests use it in process as a `Transport` (lines 204-210) and over a real Unix socket (`wire_herdr::serve`), and run the conformance suite against it both ways (`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:2-18`). It serves nine methods and refuses every other as unknown or forbidden (lines 261-279), so it has no `pane.rename`, `pane.get` or `pane.list`. Every `PaneInfo` it sends, in `session.snapshot` (lines 297-313), `workspace.create` (line 338) and `pane.split` (line 363), comes from `pane_json`, which sends no `label` (lines 603-606). Clarification 3's label "field of the pane it serves" therefore reaches the adapter only if `pane_json` sends it.

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:1-7`
~~~~text
1| //! A simulated Herdr at the JSON level (#640 part 2, Decision 15): it reads request
2| //! lines with `serde_json` and answers them the way the Herdr spike
3| //! (`docs/research/herdr-api-spike.md`) saw Herdr answer, keeping its own split trees.
4| //! It is written from the spike, never from `src/`: it reads no tree with the adapter's
5| //! code and decodes no reply with it, so a bug in the adapter cannot hide in its oracle
6| //! (AC 35). Each test file that needs it declares `mod wire_herdr;`, and uses a part of
7| //! it, so the rest would trip `dead_code`.
~~~~

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:204-210`
~~~~text
204| /// In process: the line the adapter built goes to [`WireHerdr::answer`]; the deadline
205| /// is unused.
206| impl Transport for WireHerdr {
207|     fn exchange(&self, request: &Request, _deadline: Instant) -> Result<String, PaneError> {
208|         Ok(self.answer(&request.to_line()))
209|     }
210| }
~~~~

`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:2-18`
~~~~text
2| //! `HerdrAdapter` passes #638's `HerdrPort` conformance suite (#640 part 2, AC 31-34)
3| //! against the wire fake, from the two starting states the suite allows (an empty
4| //! Herdr, and a workspace holding only its root pane), in process and over a real Unix
5| //! socket. The suite's cases are run, never copied.
6|
7| mod wire_herdr;
8|
9| use std::path::Path;
10| use std::sync::Arc;
11|
12| use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
13| use holler_adapter_herdr::plan::Extent;
14| use holler_adapter_herdr::protocol::ALLOWED_METHODS;
15| use holler_pane_testkit::conformance::herdr::{run_herdr_conformance, HerdrFixture};
16| use holler_pane_testkit::conformance::Conformance;
17| use wire_herdr::serve::serve;
18| use wire_herdr::WireHerdr;
~~~~

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:261-279`
~~~~text
261|     fn handle(&mut self, method: &str, params: &Map<String, Value>) -> Result<Value, Fault> {
262|         match method {
263|             "ping" => Ok(self.pong()),
264|             "session.snapshot" => Ok(self.snapshot()),
265|             "layout.export" => self.layout_export(params),
266|             "workspace.create" => Ok(self.workspace_create(params)),
267|             "pane.split" => self.pane_split(params),
268|             "pane.send_text" => self.send_text(params),
269|             "pane.send_keys" => self.send_keys(params),
270|             "pane.read" => self.read(params),
271|             "pane.close" => self.close(params),
272|             other if is_forbidden(other) => Err(invalid(format!(
273|                 "invalid request: the simulated Herdr refuses `{other}`"
274|             ))),
275|             other => Err(invalid(format!(
276|                 "invalid request: unknown variant `{other}`"
277|             ))),
278|         }
279|     }
~~~~

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:297-313`
~~~~text
297|     fn snapshot(&self) -> Value {
298|         let (mut workspaces, mut tabs, mut panes) = (Vec::new(), Vec::new(), Vec::new());
299|         for (i, workspace) in self.workspaces.iter().enumerate() {
300|             workspaces.push(workspace_json(workspace, i + 1));
301|             for tab in &workspace.tabs {
302|                 tabs.push(tab_json(workspace, tab));
303|                 for pane in tab.root.panes() {
304|                     panes.push(pane_json(workspace, tab, &pane));
305|                 }
306|             }
307|         }
308|         json!({"type": "session_snapshot", "snapshot": {
309|             "version": self.version(), "protocol": self.protocol,
310|             "workspaces": workspaces, "tabs": tabs, "panes": panes,
311|             "layouts": [], "agents": []
312|         }})
313|     }
~~~~

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:603-606`
~~~~text
603| fn pane_json(workspace: &Workspace, tab: &Tab, pane: &str) -> Value {
604|     json!({"pane_id": pane, "workspace_id": workspace.id, "tab_id": tab.id,
605|         "focused": false, "cwd": "/scratch", "agent_status": "idle", "revision": 0})
606| }
~~~~

**`FakeHerdr`** is `crates/holler-pane-testkit/src/herdr.rs`. Its hooks for the outside world acting on a pane, `vanish` and `print`, bypass the faults and the call log (lines 129-133, 223-236); a label hook is one more of them. A pane is a `LivePane { id, screen }` (lines 165-169). The doctor rig of acceptance 3 runs on `FakeHerdr` (`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:1-2`, `:103`, `:124-126`), so its regression test labels the rig's panes through that hook.

`crates/holler-pane-testkit/src/herdr.rs:129-138`
~~~~text
129| /// Every port method first passes [`FakeHerdr::faults`], and when that fails, it
130| /// returns the error and changes nothing. The call log there holds every attempt, and
131| /// it is where a test checks I4's "no keystroke". `vanish` and `print` are the outside
132| /// world acting on a pane: they bypass the faults and the call log.
133| ///
134| /// It is not `Clone`: share it behind an `Arc`, because a copy would split the session.
135| pub struct FakeHerdr {
136|     state: Mutex<State>,
137|     faults: FaultSwitch<HerdrOp>,
138| }
~~~~

`crates/holler-pane-testkit/src/herdr.rs:165-169`
~~~~text
165| /// One pane that exists: its id and the text its screen shows.
166| struct LivePane {
167|     id: PaneId,
168|     screen: String,
169| }
~~~~

`crates/holler-pane-testkit/src/herdr.rs:223-236`
~~~~text
223|     /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is
224|     /// free, its id is never reused, and every later call naming it is
225|     /// `pane-not-found`. It bypasses the faults and the call log. `pane-not-found` when
226|     /// no such pane exists.
227|     pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError> {
228|         self.lock().remove(pane)
229|     }
230|
231|     /// The pane's program prints `text`, which `read` then shows. It bypasses the faults
232|     /// and the call log. `pane-not-found` when no such pane exists.
233|     pub fn print(&self, pane: &PaneId, text: &str) -> Result<(), PaneError> {
234|         self.lock().pane_mut(pane)?.screen.push_str(text);
235|         Ok(())
236|     }
~~~~

`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:1-2`
~~~~text
1| //! The doctor rig (#647, brief Decision 12): a live world on the test kit's fakes, with one
2| //! record per pane that matches it.
~~~~

`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:103`
~~~~text
103|     pub herdr: FakeHerdr,
~~~~

`crates/holler-cli/tests/pane_verbs/doctor/rig.rs:124-126`
~~~~text
124|         let herdr = FakeHerdr::new(SCRATCH)
125|             .with_workspace(SCRATCH, 3, 3)
126|             .expect("the rig's workspace");
~~~~

**The conformance suite has no optional hook today (NV-7).** `run_herdr_conformance` takes one closure, `fresh`, which returns the fixture (`port`, `session`, `workspace`) and a guard. The other five runners take only `fresh`, or `build` for the profile scope. So the label hook is the suite's first optional closure parameter: clarification 3's "as its other optional hooks are" has no precedent to follow, and its instruction, a closure parameter, stands as written.

`crates/holler-pane-testkit/src/conformance/herdr.rs:57-64`
~~~~text
57| pub struct HerdrFixture<H> {
58|     /// The implementation under test.
59|     pub port: H,
60|     /// The scratch session: never the default session or a live one.
61|     pub session: String,
62|     /// The workspace of 2 rows by 1 column that the cases place their panes in.
63|     pub workspace: String,
64| }
~~~~

`crates/holler-pane-testkit/src/conformance/herdr.rs:140-152`
~~~~text
140| pub fn run_herdr_conformance<H, K, F>(fresh: F) -> Conformance
141| where
142|     H: HerdrPort,
143|     F: FnMut() -> (HerdrFixture<H>, K),
144| {
145|     run_cases(&CASES, fresh, |case, fixture| {
146|         case(&Scratch {
147|             port: &fixture.port,
148|             session: &fixture.session,
149|             workspace: &fixture.workspace,
150|         })
151|     })
152| }
~~~~

`crates/holler-pane-testkit/src/conformance/pane_store.rs:113`
~~~~text
113| pub fn run_pane_store_conformance<S, K, F>(fresh: F) -> Conformance
~~~~

`crates/holler-pane-testkit/src/conformance/profile_store.rs:177`
~~~~text
177| pub fn run_profile_store_conformance<S, K, F>(fresh: F) -> Conformance
~~~~

`crates/holler-pane-testkit/src/conformance/host.rs:90`
~~~~text
90| pub fn run_host_conformance<S, K, F>(fresh: F) -> Conformance
~~~~

`crates/holler-pane-testkit/src/conformance/harness.rs:142`
~~~~text
142| pub fn run_harness_conformance<S, K, F>(mut fresh: F) -> Conformance
~~~~

`crates/holler-pane-testkit/src/conformance/profile_scope.rs:193`
~~~~text
193| pub fn run_profile_scope_conformance<S, F>(mut build: F) -> Conformance
~~~~

**NIT-2.** Scope item 3's conformance case also labels one pane with the empty string and expects `None`. Acceptance 1 already requires that of the fake test and of the adapter test; the case checks it through the one case both implementations run.

### E7. The byte-pinned fixture (NV-10, W-3)

- **Provenance.** Clarification 4's "code on `main` before this change" is `92fbc6e4186acc717f4aa2b6465c55a266b9043c`, this branch's base. T produces the literal from that code and names the commit in a comment beside it. The literal is T's RED artifact, so it is not quoted here.
- **Round trip (W-3).** Clarification 4's second test also writes each record it read back to JSON and reads that again: the result equals the record first read. Written with the `serde_json` function that produced the literal, the pre-change record's JSON also equals the literal byte for byte, since `label: None` writes no key. The hub does this to every record: it loads `panes.json` once at startup and saves the whole document on every write (`crates/holler-hub/src/panes/persist.rs:1-2`).

### E8. Small clarifications (NIT-1; line numbers)

- **NIT-1.** `HerdrPane.label` is `None` both when Herdr sends no `label` (Herdr omits an absent one, spike line 432) and when it sends `""`: `label_of` folds the two (clarification 2). The field's doc comment says so, so that a reader does not take `None` for a distinction Herdr makes. E13 (round 2) gives what the struct's and the field's doc comments say.
- **ADR-0021 line numbers.** The "Deferred to named stories" `HerdrPort` bullet is `docs/adr/ADR-0021.md:739-740` (scope item 4 says 740-741). The section-2 note is line 111. "Decisions taken" ends at item 9 (lines 770-771), so clarification 5's item 10 is the next number.

`docs/adr/ADR-0021.md:111`
~~~~text
111| `HerdrPort`, and the data types it takes and returns, stay **provisional** until the spike #636 (Herdr) reports.
~~~~

`docs/adr/ADR-0021.md:739-740`
~~~~text
739| - `HerdrPort`: #640 implements it as merged; a pane with no cell (inside a nested split, or in another tab) has no place in
740|   `HerdrSnapshot`, which waits for a contract amendment before #647 and #650 need it.
~~~~

`docs/adr/ADR-0021.md:770-771`
~~~~text
770| 9. **A verb records `host.herdr_api_version`** (operator, 2026-10-11, section 10): #644's `launch` and `relaunch` write it; the
771|    Herdr adapter writes no record.
~~~~

### E9. Added to the scope, the blast radius and the acceptance criteria

These follow from the survey and from the constraints above; none removes or narrows anything.

- **Scope and blast radius:** `crates/holler-pane/src/tx_launch.rs`, for two changes: `Plan::record` writes `label: None` (E2, constraint (c)), and O2 compares the four fields (E5 row 8, constraint (b)); with them, the launch tests that cover the two changes (`crates/holler-cli/tests/pane_verbs/launch/`). Nothing in reconcile, doctor or the hub changes.
- **Acceptance**, added to the list above:
  - A relaunch in place of a pane whose `ensure_pane` answer carries a label writes a record with `herdr.label: None`, whose JSON has no `label` key (E2).
  - A launch passes O2 when the O2 snapshot reports a label for its pane that the `ensure_pane` answer did not (E5 row 8), for example through a `HerdrPort` double around `FakeHerdr`. E11 (round 2) pins the comparison O2 makes and the three tests that hold it.
  - The round trip of E7.
  - The conformance case's empty-label pane (E6, NIT-2).

## Round 2 amendment (brief gate, 2026-10-11)

The outside review of round 2 blocked the brief on two findings, B-1 and B-2. In this section the finding ids (B-, NV-, W-, NIT-) are round 2's: round 2 reuses round 1's numbers for other findings, and E1 to E9 name round 1's. Like E1 to E9, this section only adds. It changes neither the operator's decision nor any scope item, acceptance criterion or clarification, and where it pins something E1 to E9 left open, it says so. Sources are quoted as of the branch's base, `92fbc6e` (the branch's commits still touch only `docs/handoffs/`).

In short:

- Today the adapter parses the `panes` of `session.snapshot` and drops them: nothing in its `src` reads `SessionState.panes`. So the label has to be threaded to each `HerdrPane` (B-1). E10 pins how. The parsers keep each pane's label as Herdr sent it, and the two `HerdrPane` literals pass it through `label_of`; the one in `snapshot()` finds each placed pane's label by joining the pane to the snapshot's `panes` by pane id. There is still no new method and no new call.
- O2 compares four fields (`session`, `workspace`, `pane_id`, `grid`), not whole values, and three tests hold it to that: a label-only difference passes, a vanished pane fails (AC 5a's existing test), and the pane's id listed at another place fails (E11; B-2, W-1).
- `run_herdr_conformance` gains one parameter, the optional label hook, with the signature in E12 (W-2).
- E13 is NIT-1 and E14 is NV-2. NIT-2 is not applied (end of this section).

### E10. How each `HerdrPane` gets its label (B-1, NV-1, NV-3, NV-4)

**Today.** `parse_snapshot` reads every pane of `session.snapshot` into `SessionState.panes` (`protocol.rs:386-405`, E4), and nothing in the adapter's `src` reads that field: `grep -rn 'PaneRef\|\.panes\b' crates/holler-adapter-herdr/src` finds only the type and its parser (`protocol.rs:345`, `:355`, `:439-440`). `snapshot()` builds each `HerdrPane` from `cells()`, which gives a cell and a pane id only (E3). `ensure` reads its workspace through `workspace_grid`, which keeps the grid tab and the grid and drops the rest of its `session.snapshot` read:

`crates/holler-adapter-herdr/src/adapter.rs:406-412`
~~~~text
406| /// A configured workspace as Herdr has it: its grid tab, `None` when Herdr has no
407| /// workspace of the label, and the grid read off that tab's tree, empty when there is
408| /// none.
409| struct WorkspaceGrid {
410|     tab: Option<String>,
411|     map: GridMap,
412| }
~~~~

So E4 is right that the label of every pane the adapter reports is in a reply it already receives, and B-1 is right that no value reaching the two `HerdrPane` literals carries it. The diff threads it as follows. The post-diff source does not exist before F, so the diff gate checks the diff against this text (NV-1).

1. **The parsers keep the label as Herdr sent it.** `PaneRef` gains `pub label: Option<String>`, which `pane_ref` reads. `parse_pane_info` returns the pane's id and its label. `parse_workspace_created` returns the workspace, the root pane's id and the root pane's label. Each reads `label` off the `PaneInfo` object it already holds (the snapshot's pane, `pane_info`'s `pane`, `workspace_created`'s `root_pane`) the way `parse_pong` reads its optional `protocol`, straight off the object's fields: a `label` that is absent, or is not a string, is `None`. It does not use `Object::string` (`protocol.rs:541-544`), which refuses a missing field as `unavailable`. Herdr leaves an absent label out (spike line 432), and no reply that parses today may fail because of its label (clarification 1 keeps `snapshot()`'s behavior; constraint (b)).

`crates/holler-adapter-herdr/src/protocol.rs:305-317`
~~~~text
305| /// Read a `pong` result. A missing `protocol`, or one that is not a protocol number, is
306| /// `None`: unknown, which [`check_supported`] refuses, not a garbled reply.
307| pub fn parse_pong(result: &Value) -> Result<ServerVersion, PaneError> {
308|     let pong = result_of(result, "pong")?;
309|     Ok(ServerVersion {
310|         version: pong.string("version")?.to_owned(),
311|         protocol: pong
312|             .fields
313|             .get("protocol")
314|             .and_then(Value::as_u64)
315|             .and_then(|protocol| u32::try_from(protocol).ok()),
316|     })
317| }
~~~~

2. **`label_of` is `pub`, in `protocol.rs` beside the pane parsers (NV-4).** Its signature is clarification 2's, `pub fn label_of(raw: Option<&str>) -> Option<String>`. It is public like the parsers: the crate tests its protocol layer from `crates/holler-adapter-herdr/tests/protocol_test.rs` through the public module (`crates/holler-adapter-herdr/src/lib.rs:40`, `pub mod protocol;`), and its `src` has no `#[cfg(test)]` module. Its direct test goes there: `None` and `""` give `None`, `"x"` gives `Some("x")`, and `" "` gives `Some(" ")` (no trimming).

`crates/holler-adapter-herdr/tests/protocol_test.rs:10-14`
~~~~text
10| use holler_adapter_herdr::protocol::{
11|     check_supported, decode_reply, expect_ok, parse_layout_export, parse_pane_info, parse_pong,
12|     parse_read, parse_snapshot, parse_workspace_created, PaneRef, Request, ServerVersion,
13|     WorkspaceRef, ALLOWED_METHODS, SUPPORTED_PROTOCOLS, SUPPORTED_VERSIONS,
14| };
~~~~

3. **The two `HerdrPane` literals call `label_of`, and nothing else does** (clarification 2: "**Both** `snapshot()` and what `ensure_pane` returns use it").
   - `snapshot()`, the literal at `adapter.rs:381-386`: for each placed `(grid, pane_id)` of `cells()`, `label` is `label_of` of the label of the `PaneRef` in `state.panes` (the call's one `session.snapshot` read, line 373) whose `pane_id` is that pane id. That is the `pane_id`-to-label join. The panes listed, their order and their other four fields stay today's.
   - `ensure`, the literal at `adapter.rs:247-252`: `label` is `label_of` of the label of the pane it answers. For an occupied cell (lines 221-231), that is the pane's label in `workspace_grid`'s `session.snapshot` read (line 196), which `WorkspaceGrid` now also carries, looked up the way `snapshot()` looks it up (one lookup serves both, for example a `SessionState` method that gives a pane id's label). For a split (lines 272-295), it is the label `parse_pane_info` read. For a new workspace's root pane (lines 265-271), it is the label `parse_workspace_created` read. `place` (lines 257-263) returns the new pane's label with its id.

4. **Added to the scope, an edge case: a placed pane that the `session.snapshot` read does not list.** `layout.export` is read after `session.snapshot`, so a pane made between the two reads has a cell and no `PaneRef`. It is reported as today, at its cell, with `label: None`, by `snapshot()` and in `ensure_pane`'s answer for an occupied cell. The label never adds, drops or fails an entry: dropping the pane or failing the call would change what `snapshot()` reports (clarification 1), and so what reconcile and doctor see (constraint (b)), and reading again would add a call (E4).

5. **No method is added (NV-3).** Reading a field of a reply already received sends no request. `Method` and its table (`protocol.rs:65-108`), `METHOD_COUNT` (9, lines 44-45) and `ALLOWED_METHODS` (lines 47-57) are unchanged, and so are the nine methods `WireHerdr` serves (`wire_herdr/mod.rs:261-279`, E6). The allow-list test (`adapter_conformance_test.rs:80-98`, E4) drives the adapter's in-process and socket runs, which run the new conformance case once they pass E12's hook, and the test itself does not change.

**Tests, added to the acceptance criteria.** The first three are in `crates/holler-adapter-herdr/tests/adapter_test.rs`, over `WireHerdr` in process, with each pane labelled through `WireHerdr`'s label (clarification 3); the last is in `protocol_test.rs`:

- **The join (B-1).** A workspace whose placed panes are labelled `x`, `y` and `""` plus one not labelled, and a pane labelled `z` in a second tab (`WireHerdr::add_tab`, `wire_herdr/mod.rs:125`). `snapshot()` lists the four placed panes at their cells with `Some("x")`, `Some("y")`, `None` and `None`, each its own pane's label, and does not list the second tab's pane. A join that gave a cell another pane's label fails on `x` and `y`. The `""` pane is clarification 2's test of `snapshot()`.
- **The occupied cell.** `ensure_pane` of a cell whose pane is labelled `x` answers `label: Some("x")`, and of one whose pane is labelled `""` answers `label: None` (clarification 2's test of `ensure_pane`).
- **The edge case (item 4).** A `Tap` whose hook, on the `layout.export` request, splits a pane through `WireHerdr::split` (`wire_herdr/mod.rs:136`) and then forwards the request: `snapshot()` succeeds and lists the new pane at its cell with `label: None`. A `Tap`'s hook may act on the fake:

`crates/holler-adapter-herdr/tests/wire_herdr/mod.rs:227-229`
~~~~text
227| /// A transport in front of a fake whose hook sees each exchange first: to rewrite a
228| /// request, act on the fake, record the deadline, garble the reply or fail the exchange
229| /// (part 2's AC 16, 17, 28 and 29 and part 3's AC 16-18 share it).
~~~~

- **The parsers**, in `protocol_test.rs`: `parse_snapshot`'s `PaneRef`s carry each pane's label as sent (`"x"` and `""` as they are; an absent one, and a non-string, as `None`), and `parse_pane_info` and `parse_workspace_created` return their pane's label. The `PaneRef` literal at `protocol_test.rs:488-492` gains `label: None`, since the panes of its fixture carry no label (lines 443-448).

### E11. O2 compares four fields, and three tests hold it to that (B-2, W-1)

**The comparison (B-2).** O2 (`tx_launch.rs:684-691`, E5) holds when the O2 snapshot lists an entry equal to the `ensure_pane` answer on `session`, `workspace`, `pane_id` and `grid`. Those are the four fields `HerdrPane` has today, so they are exactly what today's whole-value `contains` compares. O2 does not compare the label and no longer uses `HerdrPane`'s derived `PartialEq` (`Vec::contains`). Its message is unchanged. Its four-field helper sits beside `same_pane` (`tx_launch.rs:750-753`), which stays the join key of session and pane id.

**Where a test can act between `ensure_pane` and O2.** `bring_up` calls `ensure_pane` (line 616), later `attach_tui`, and right after it `observe_live` (O1, then O2):

`crates/holler-pane/src/tx_launch.rs:635-642`
~~~~text
635|     budget.check()?;
636|     let session = session_of_record(ports, plan)?;
637|     budget.check()?;
638|     ports
639|         .harness
640|         .attach_tui(&herdr.pane_id, plan.port, &session)?;
641|     let shown = observe_live(ports, plan.name, budget, &herdr, &session)?;
642|     budget.check()?;
~~~~

The launch rig runs a test's closure right after a named harness method returns. AC 5a's test uses it after `attach_tui` to vanish the pane, and the run then fails with O2's `unavailable`, which names the pane:

`crates/holler-cli/tests/pane_verbs/launch/rig.rs:256-262`
~~~~text
256|     /// Run `hook` once, right after the next call of `op` returns.
257|     pub fn after(&self, op: HarnessOp, hook: impl FnOnce(&Fakes, &Called) + Send + 'static) {
258|         self.hooks
259|             .lock()
260|             .unwrap_or_else(PoisonError::into_inner)
261|             .push((op, Box::new(hook)));
262|     }
~~~~

`crates/holler-cli/tests/pane_verbs/launch.rs:225-245`
~~~~text
225| /// AC 5a: a Herdr pane that vanishes after the attach is observed, and nothing is recorded.
226| #[test]
227| fn launch_fails_when_its_herdr_pane_vanishes() {
228|     let rig = Rig::new();
229|     let vanished = Arc::new(Mutex::new(None));
230|     let seen = vanished.clone();
231|     rig.after(HarnessOp::AttachTui, move |f, c| {
232|         let pane = c.pane.clone().unwrap();
233|         f.herdr.vanish(&pane).unwrap();
234|         *seen.lock().unwrap() = Some(pane);
235|     });
236|     let run = rig.run(LAUNCH, Format::Json);
237|     assert_eq!(run.code, 1, "{run:?}");
238|     let (code, message) = error_of(&run);
239|     assert_eq!(code, "unavailable");
240|     let pane: PaneId = vanished.lock().unwrap().clone().expect("the attach ran");
241|     assert!(message.contains(pane.as_str()), "names the pane: {message}");
242|     assert_eq!(rig.record(PANE), None);
243|     rig.assert_matches(&run, PANE);
244|     rig.assert_no_keystroke();
245| }
~~~~

**The tests** (acceptance; they make E9's O2 bullet concrete and remove nothing):

1. **A label-only difference passes O2 (B-2's assertion; W-1).** In `crates/holler-cli/tests/pane_verbs/launch.rs`, over the launch rig, a hook after `HarnessOp::AttachTui` labels the new pane `x` through `FakeHerdr`'s label hook (scope item 3). The `ensure_pane` answer then has `label: None` and the O2 snapshot's entry has `Some("x")`, with the four fields equal. The launch succeeds, and its record's `herdr.label` is `None`. The test ends, as the launch cases do, with `assert_matches`, which already compares the record with the snapshot field by field, not by whole value (`launch/rig.rs:485-489`, below). Against today's whole-value `contains` the run fails `unavailable` ("the Herdr pane ... is gone from ..."), so the test fails if `contains`, or any whole-value comparison, comes back (W-1).
2. **A pane that is gone fails O2.** AC 5a's `launch_fails_when_its_herdr_pane_vanishes` (above) stays as it is and passes. An O2 that always passed would fail it (B-2: "the implementation could make O2 always pass").
3. **The pane's id listed at another place fails O2.** A launch whose O2 snapshot lists the answer's pane id at another `grid` fails `unavailable` and records nothing; so does one whose O2 snapshot lists it in another `workspace`, and one in another `session` (one case each). An O2 that compared only the join key (`same_pane`) passes test 2 and fails the `grid` and `workspace` cases. `FakeHerdr` never moves a pane, and the rig hands it to the verb as `Ports.herdr` itself, so these cases add to the launch rig a `HerdrPort` double around `FakeHerdr` whose `snapshot()` reports the pane so once a hook after `attach_tui` arms it (the plan's earlier snapshot, `tx_launch.rs:292`, stays the fake's). The test kit's mutants wrap the fake the same way (`Mutant { inner: FakeHerdr, .. }`, `crates/holler-pane-testkit/tests/herdr_conformance_test.rs:201-205`, its `HerdrPort` impl at `:296`).

`crates/holler-pane-testkit/src/herdr.rs:120-122`
~~~~text
120| /// base 36 (spike section 5). The counter only goes up, so an id is never reused after
121| /// a close or a vanish. `close` and [`FakeHerdr::vanish`] free a pane's cell, and no
122| /// other pane is ever moved, replaced or renumbered.
~~~~

`crates/holler-cli/tests/pane_verbs/launch/rig.rs:279-287`
~~~~text
279|         f(Ports {
280|             pane_store: self.fakes.panes.as_ref(),
281|             profile_store: self.fakes.profiles.as_ref(),
282|             herdr: &self.fakes.herdr,
283|             host: &host,
284|             harness: &harness,
285|             scope,
286|             prober: &self.fakes.prober,
287|         })
~~~~

`crates/holler-cli/tests/pane_verbs/launch/rig.rs:485-489`
~~~~text
485|         let listed = self.herdr_panes().into_iter().any(|p| {
486|             p.pane_id == record.herdr.pane_id
487|                 && p.grid == record.herdr.grid
488|                 && p.workspace == record.herdr.workspace
489|         });
~~~~

**Why not a unit test of `observe_live` (W-1).** `observe_live` is private to `tx_launch.rs`, and `holler-pane` keeps its tests under `tests/`: its one inline test module calls itself a departure from that pattern. Tests 1 to 3 run `observe_live` through the launch engine instead.

`crates/holler-pane/src/probe.rs:315-316`
~~~~text
315|     //! `Drop`. The tests signal nothing themselves; `ps` is only read. Inline, a deliberate
316|     //! departure from this crate's `tests/` pattern: the blast radius has no test file, and
~~~~

### E12. The label hook's parameter on `run_herdr_conformance` (W-2)

**What decides it.** Clarification 3 makes the hook a closure parameter of the suite, and scope item 3 runs the label case against `FakeHerdr` and the wire fake only. `run_herdr_conformance` has nine callers, all in the blast radius: `crates/holler-pane-testkit/tests/herdr_conformance_test.rs:55`, `:61`, `:144`, `:386`, `:402` and `:480` (`FakeHerdr`, or a wrapper of it); `crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:40` and `:55` (`WireHerdr`); and `crates/holler-adapter-herdr/tests/scratch_herdr_test.rs:221`, the opt-in run against a real Herdr, which scope item 3 leaves out of the label case:

`crates/holler-adapter-herdr/tests/scratch_herdr_test.rs:213-229`
~~~~text
213| #[test]
214| #[ignore = "opt-in: a scratch Herdr server; set HOLLER_HERDR_SCRATCH=1 (#640)"]
215| fn scratch_herdr_passes_the_conformance_suite() {
216|     let Some(herdr) = opt_in() else {
217|         return;
218|     };
219|     let begun = Instant::now();
220|
221|     let result = run_herdr_conformance(|| {
222|         let server = ScratchHerdr::start(&herdr);
223|         let fixture = HerdrFixture {
224|             port: connect(&server),
225|             session: server.session().to_owned(),
226|             workspace: GRID.to_owned(),
227|         };
228|         (fixture, server)
229|     });
~~~~

A hook for the adapter's runs must reach the case's `WireHerdr`, and only what `fresh` returns can bring it there. In process, the fake becomes the adapter's transport, and `HerdrAdapter` exposes only its config (`adapter.rs:124-127`). Over the socket, it moves into the server thread, and `Served` keeps no handle to it:

`crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:39-48`
~~~~text
39| fn run_in_process(root_pane: bool, fakes: &mut Vec<Arc<WireHerdr>>) -> Conformance {
40|     run_herdr_conformance(|| {
41|         let fake = Arc::new(WireHerdr::new());
42|         if root_pane {
43|             fake.create_workspace(SCRATCH);
44|         }
45|         fakes.push(Arc::clone(&fake));
46|         let port =
47|             HerdrAdapter::connect_with(config(Path::new(UNUSED_SOCKET)), fake).expect("connect");
48|         (fixture(port), ())
~~~~

`crates/holler-adapter-herdr/tests/wire_herdr/serve.rs:20-27`
~~~~text
20| /// A served fake. Dropping it stops the server thread and removes the directory.
21| pub struct Served {
22|     /// Removed when this drops, after the thread has stopped.
23|     _dir: TempDir,
24|     path: PathBuf,
25|     stop: Arc<AtomicBool>,
26|     thread: Option<JoinHandle<()>>,
27| }
~~~~

**The signature, pinned** (new code, not a quote):

```rust
pub fn run_herdr_conformance<H, K, F>(
    fresh: F,
    label: Option<&dyn Fn(&HerdrFixture<H>, &K, &PaneId, &str) -> Result<(), PaneError>>,
) -> Conformance
```

- The hook sets the label of the pane `PaneId` in the case's Herdr to the text, `""` included, as Herdr's `pane.rename` does (spike line 140), and returns `pane-not-found` for a pane that Herdr does not hold, as `FakeHerdr`'s `vanish` and `print` do (E6). It receives the fixture, whose `port` is the `FakeHerdr` (or its wrapper) in the test kit's runs, and the case's guard, which in the adapter's runs carries the case's `Arc<WireHerdr>` (beside `Served`, over the socket).
- Every caller is updated. The scratch run passes `None`, and with `None` the label case is not run. The other eight pass a hook: the test kit's runs over `FakeHerdr` and over that test file's wrappers of it (`GuardWatcher` and `Mutant`, each holding an `inner: FakeHerdr`, lines 90-91 and 201-202), and the adapter's two runners, `run_in_process` and `run_over_a_socket`, which make its three runs over `WireHerdr`. So `the_suite_builds_a_fresh_fixture_per_case` still finds one fixture built and one guard dropped per documented case, once `DOCUMENTED_CASES` (lines 23-36) lists the new case:

`crates/holler-pane-testkit/tests/herdr_conformance_test.rs:160-172`
~~~~text
160|     assert_eq!(result, Ok(()));
161|     assert_eq!(built.load(Ordering::SeqCst), DOCUMENTED_CASES.len());
162|     assert!(calls.load(Ordering::SeqCst) > 0, "the suite drove the port");
163|     assert_eq!(
164|         after_drop.load(Ordering::SeqCst),
165|         0,
166|         "no call may reach the port after its guard was dropped"
167|     );
168|     assert_eq!(
169|         dropped.load(Ordering::SeqCst),
170|         DOCUMENTED_CASES.len(),
171|         "every guard is dropped by the end of the run"
172|     );
~~~~

- `run_cases` hands `check` the subject, never the guard. The runner can keep it unchanged by handing it the pair of fixture and guard as the subject and `()` as the guard: a tuple drops its fields in order, so the fixture still drops before its guard (`conformance/herdr.rs:125-127`: "The fixture is dropped before its guard").

`crates/holler-pane-testkit/src/conformance/mod.rs:43-61`
~~~~text
43| /// Run every case of `cases`, each against a fresh subject, and gather the failures in
44| /// case order. `fresh` returns the subject and a guard that is kept alive for that case
45| /// only (a temporary directory, say); the subject is dropped before its guard. `check`
46| /// runs one case against the subject.
47| pub(crate) fn run_cases<S, K, C: Copy>(
48|     cases: &[(&'static str, C)],
49|     mut fresh: impl FnMut() -> (S, K),
50|     mut check: impl FnMut(C, &S) -> Result<(), String>,
51| ) -> Conformance {
52|     let failures: Vec<CaseFailure> = cases
53|         .iter()
54|         .filter_map(|&(case, run)| {
55|             let (subject, guard) = fresh();
56|             let outcome = check(run, &subject);
57|             drop(subject);
58|             drop(guard);
59|             outcome.err().map(|detail| CaseFailure { case, detail })
60|         })
61|         .collect();
~~~~

### E13. The `HerdrPane` doc comments (NIT-1)

`HerdrPane`'s doc comment (`pane.rs:91-92`, E1) adds that a pane Herdr reports also carries its label. The `label` field's doc comment is written like `HostInfo.herdr_api_version`'s (`pane.rs:112-115`, E2) and says three things. The label is an observation: what Herdr reports for the pane, read by the adapter through `label_of`. `launch` and `relaunch` never write one, so a record's `herdr.label` is `None` and its JSON has no `label` key. And `None` stands both for "Herdr sent no label" and for "Herdr sent an empty one" (clarification 2; E8's NIT-1 bullet).

### E14. The record write and the byte test (NV-2)

`Plan::record` writes `herdr: HerdrPane { label: None, ..live.herdr }` where it writes `herdr: live.herdr` today (`tx_launch.rs:327`, E2). It is the one place in production code that sets a record's `herdr`: outside the test kit's fixture (`crates/holler-pane-testkit/src/fixture.rs:46`), `grep -rn 'herdr: ' crates/*/src` finds no other `Pane` literal that sets it, and the other `Pane` literals in `src` copy an existing record's `herdr` unchanged (`..pane.clone()` or `..record.clone()`, for example `crates/holler-hub/src/panes/store.rs:176-178`). The record of clarification 4's byte test comes from the launch engine run with a constant clock, which `TxOptions` provides for tests, so every run writes the same record:

`crates/holler-pane/src/tx_launch.rs:89-92`
~~~~text
89| /// `now_ms` is a function, where `ReconcileRequest.now_ms` is a value taken once: a run stamps
90| /// `last_observed.at` when it writes the record, after the TUI was observed, which can be up to
91| /// the budget after the verb started, so the engine reads the clock there. A test passes a
92| /// constant.
~~~~

The diff's lines and the literal (T's RED artifact, E7) are the diff gate's evidence.

### E15. Added to the scope and the acceptance criteria (round 2)

None of it removes or narrows anything, and all of it is inside the blast radius (with E9's `tx_launch.rs`).

- **Scope:** E10's threading (items 1 to 3) and its edge case (item 4); E11's four-field O2; E12's hook parameter and its nine updated callers; E13's doc comments; E14's `Plan::record`.
- **Acceptance**, added to the list above:
  - E10's adapter tests (the join, the occupied cell, the pane made between the two reads), its parser tests, and `label_of`'s direct test with `" "`.
  - E11's three O2 tests (test 2 is AC 5a's, unchanged).
  - E12's conformance runs: `FakeHerdr`'s, its wrappers', and the adapter's three over `WireHerdr` run the label case with a hook; the scratch run passes `None`.

**Not applied: round 2's NIT-2** (reword clarification 5's ADR-0021 sentence as "no comparison, after #808's four-field O2 check, looks at it"). Clarification 5 writes the ADR text out and says the PR edits exactly those places, so the wording is the issue's, not the brief's to change. The sentence is also never false on `main`: the PR is squash-merged, so it lands in the same commit as the four-field O2 check that makes it true (E5's ADR-0021 paragraph; E11).
