# Testing

This is a **stub**. The authoritative test-catalog contract lives in the
**master testing issue**
[#168](https://github.com/Performant-Labs/holler/issues/168) — one issue per
test case, the `hlr-NNNN` ID scheme, the label axes, and the `Automation`
grammar the runner parses. This page exists so the contract has a stable
in-repo home; the full doc content (prose walkthrough, authoring guide,
selection/`exec` reference) is **story 4** of the test-catalog chain and is not
written here yet.

## What is fixed today (by #168)

- **IDs** — `hlr-NNNN`, leading digit = group (1000 Invocation … 1900 Load,
  plus **2000 Protocol & wire conformance** — the 11th group added in #168).
  One ID per `#[test]` fn; a criterion `#[case]` arm shares its fn's ID.
- **Label axes** — `test-case`; `test-auto`/`test-manual`;
  `test-hub`/`test-body`; exactly one `test-grp-*`; a closed `test-cat-*`
  (`smoke`/`regression`/`acceptance`/`unit`); an open `test-tag-*` set.
- **`Automation` grammar** — `tests/<file>.rs [(<fn>)]` and
  `crates/<crate>/tests|src/<…>.rs (<fn>)`, `; `-separated, fn-addressable
  (parentheses, not `::`); `manual…` never runs; `test-tag-interop` runs with
  `-- --ignored`; unrecognized forms fall back to whole-workspace `cargo test`
  (and the case must say so).

## The two decisions this chain hinges on

1. **Eleventh group.** Protocol/wire-conformance tests (JSON-RPC envelope
   round-trips, A2A fixture conformance) get their own `protocol` group (range
   2000) rather than being folded into Network failure & recovery.
2. **ID granularity.** One `hlr-NNNN` per `#[test]` fn, not per `#[case]` —
   criterion cases are parameterizations of one behavior, and the `Automation`
   grammar is fn-addressable (cargo cannot select a single rstest case).

## Chain position

Story **2 of 4** in the test-catalog infrastructure chain: (1) label set,
merged; (2) **this contract**; (3) the runner port (`test-run.rb` /
`test_selection.rb`); (4) the full testing docs. This stub is the only
in-repo deliverable of story 2.
