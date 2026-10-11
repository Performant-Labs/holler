# Decision journal: #808, a Herdr snapshot pane carries its label

Run handoff directory `docs/handoffs/808` · branch `issue-808-implementation` · rigor second-opinion · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The brief gate's rounds are in
`docs/handoffs/808-brief-result-r{1,2,3}.md` (round 1 BLOCK, round 2 BLOCK, round 3 PASS).

## A, up-front plan review: 2026-10-11T05:26-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 7 `warn` findings (`docs/handoffs/808/handoff-A.md`). The plan extends the
  existing objects and builds no parallel path:
  - the label is a field on `HerdrPane`, in the established optional-field serde form;
  - the adapter reads `PaneInfo` from replies it already receives (no new method, so I5 and the allow-list are unchanged);
  - the fakes' label hooks are shaped like their existing outside-world hooks;
  - O2 becomes a four-field comparison beside `same_pane`;
  - E11's Herdr double follows the launch rig's own per-run port wrappers.

  Clarification 6's choice of a field on `HerdrPane` stands: O2 is the only whole-value comparison in production code.

  The warns are:
  1. `FakeHerdr`'s `ensure_pane` must report the label as its snapshot does, checked by the conformance case.
     Otherwise E9's relaunch test cannot fail.
  2. ADR-0021 section 1's `herdr` row needs `label?`. Scope item 4 names section 1, clarification 5's list omits it,
     and #700 is the precedent.
  3. Read the label in `protocol.rs` through one helper whose doc states the exception to the parser rule.
  4. `FakeHerdr`'s copy of the `""` mapping is documented as `last_lines`'s copy is.
  5. Document the label case's hook as an exception to the suites' policy on events the port cannot cause, and keep
     the one `CASES` table.
  6. File sizes: `tx_launch.rs` 787 and `launch/rig.rs` 718 lines. The record serde tests go to `records_test.rs`.
  7. "A record carries no label" is held only at `Plan::record`. A follow-up for #650.
- **Assumed:** T reads `handoff-A.md` before authoring RED (the run context names it). There is no O in this
  automated path to fold the warns into the brief, so findings 1 and 5 reach F through T's tests and S's audit.
  #802 has not merged, so "Decisions taken" item 10 is still free (checked `gh issue view 802`: OPEN).
- **Hedged:** finding 2 conflicts with the letter of clarification 5 ("the PR edits exactly these places"). I rated it
  `warn` with a concrete one-cell fix rather than `block`: the edit adds no prose to the four pinned places and only
  keeps a ratified table true. If the operator holds clarification 5 literally, section 1 stays stale and the PR body
  should say so.
- **Evidence:**
  - Read: the brief (E1-E15) and the gate results r2 and r3.
  - ADR-0021: sections 1, 2 and 10, "Deferred" and "Decisions taken"; commits `6d19f91` and `c6485b0`.
  - Code: `pane.rs`, `ports.rs` and `tx_launch.rs` (lines 1-40, 660-700, 740-787); the adapter's `adapter.rs` and
    `protocol.rs`; the test kit's `herdr.rs` and its conformance `herdr.rs`, `harness.rs` and `mod.rs`;
    `herdr_conformance_test.rs`; `wire_herdr/mod.rs` and `serve.rs`; `adapter_conformance_test.rs`; `launch/rig.rs`;
    `records_test.rs`; `get.rs`; the hub's `store.rs`.
  - Greps: every `impl HerdrPort` and `HerdrPane {` literal; whole-value comparisons in `holler-pane/src`; `herdr`
    writes in `crates/*/src`; `herdr` in `holler-hub/src` (none); `HerdrPane` in `holler-proto/src` (none); golden
    files and `v2.md` (none).
  - `wc -l` over every touched file; `scripts/lint.sh` limits (600 warn, 900 fail).
