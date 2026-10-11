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
