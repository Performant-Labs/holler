# Handoff-A-dup: Phase 7 - Anti-duplication gate (#660 conformance suite)

**Date:** 2026-10-09, 11:45 PM MDT
**Branch:** `issue-0660-output` (run worktree `0660-output`)
**Diff base:** `519947a`   **Diff head:** working tree (uncommitted) on opener `2f122b9`
**Diff:** exactly two files, tests only — `crates/holler-cli/tests/pane_verbs/output_api.rs` (+301/−9),
`crates/holler-cli/tests/pane_verbs/process/stub.rs` (+29). Production code changed zero lines.
**Reuse map:** `docs/handoffs/0660-output/survey.md` §Reuse & Analogous-Feature map
**F handoff:** `handoff-F.md` (honest no-op) · **T-green:** `handoff-T-green.md` (GREEN, Copy-pin repair)

VERDICT: PASS

## Summary

T extended exactly the two objects the Reuse map named (`pane_verbs/output_api.rs` and
`pane_verbs/process/stub.rs`); there is no parallel path. The #638 checker is now the conformance
authority for every #660 assertion (7 `check_envelope`/`check_ndjson` call sites in `output_api.rs`
plus one in `stub.rs`), `one_envelope` was not grown a single line — the one place the `ALL_CODES`
parity test used it was **replaced** by the checker — and every raw-text golden lands precisely on
the two things the checker deliberately does not pin (key order, compactness). The zero-production-
change shape is the operator-recorded green-on-contact outcome (decisions ~10:45/11:05 PM), not
drift: `git diff 519947a --name-only` is exactly the two test files, no `src/**`, no manifest, no
new targets, no added `unsafe`.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/output_api.rs` (753 lines) | Over the 600-line warn gate (`scripts/lint.sh`), under the 900 fail — standing advisory already carried by T-red/T-green. The #660 conformance tail (lines 488–753) is the natural `pane_verbs/` submodule split my plan-review finding 1 blessed, if the file grows again. | Split when next touched; not this run. |
| 2 | note | `output_api.rs:139,172,326,425` | Pre-existing #670 tests still assert shape through the minimal `one_envelope` — correct per the brief (the `ALL_CODES` table was the named extension point, and rewriting old tests was out of scope), but the file now holds both assertion styles. | Advisory only: wire the four old call sites to the checker whenever a future story touches them; no action for #660. |

No `block` findings. No duplication; the extension is clean.

## Checklist verification (the four a-dup questions)

**1. The plan-review finding-3 checkpoint — checker as THE authority, `one_envelope` not grown: HELD.**

- `git diff 519947a -- crates/holler-cli/tests/verb_harness/` is **empty**; `one_envelope`
  (`tests/verb_harness/mod.rs:82-97`) is byte-identical to base and its doc (:10-12) still declares
  it "deliberately minimal" with the #638 helper as the full validator. It grew no rule.
- The `ALL_CODES` parity test's JSON leg was **replaced**, not supplemented: the old
  `one_envelope(&out)` + three shape asserts (diff, removed lines at old :247-249) became
  `check_envelope(&out, json)` (new `output_api.rs:260`) plus a code-carried assert on the
  checker-returned envelope (:261-266) — strictly stronger (framing, exact key set,
  `schema_version` integer 1, one-line message, rule-13 class↔exit re-derivation), with no local
  rule text.
- The four remaining `one_envelope` call sites (:139, :172, :326, :425) sit inside pre-existing
  #670-era tests this diff never touched — the migration is additive, exactly as briefed.
- The new #660 section (:488-753) uses only `check_envelope`/`check_ndjson` (:520, :556, :582,
  :609, :632, :660) — the same import shape as the sibling checker consumers
  (`list.rs:13,154,161,198-200`, `get.rs:10,267-269`, `watch.rs:13,186`, `doctor/surface.rs:11,134`,
  `doctor/rig.rs:24,335`). `output_api.rs` was the one outlier my plan review named; it no longer is.

**2. No parallel-path duplication: CLEAN.**

- No new helper re-implements checker logic. The only shape assertions beside checker calls are
  raw-text goldens, and each lands exactly on what the checker deliberately does not pin — its rule
  4 doc says "Map order is not used" (`holler-pane-testkit/src/envelope.rs:21-23`), and it never
  checks compactness: the three outcome goldens pin the exact envelope line
  (`output_api.rs:523-526`, `:585-590`, `:591-596` region), continuing the pre-existing
  compact-golden pattern (`:184-198`). Legitimate.
- The GridPos golden's `data`-internal key order (row < col < pos, asserted on byte positions,
  `:673-684`) cannot live in the checker — rule 7 accepts `data` as any value
  (`envelope.rs:26`) — and the brief's acceptance bullet demands the golden. Legitimate.
- No copied rule tables: the parity test still iterates the one `ALL_CODES` and derives expected
  exits via `class_of` (pre-existing imports, :20, :235-236); the class↔exit re-check now flows
  through checker rule 13, not a local table. `is_valid_code` is imported only for the
  pre-existing validator-agreement test (:53-76).
- The signature-pin type aliases (`EmitU8`/`EmitStreamU8`/`EmitUsageError`, :699-709) restate the
  #637-fixed signatures — that restatement IS the acceptance bullet's compile pin, not a duplicate
  rule set. The direct `Sink` struct literal in that test (:718-721) is deliberate: it pins the
  fixed type's public field shape.
- `process/stub.rs` (:217-243) reuses the pre-existing `holler`/`STUBS` helpers and the checker; it
  adds no local envelope parsing.

**3. Drift from the plan: NONE beyond the recorded deviation.**

- Six acceptance bullets ↔ eight new + one extended test, one-to-one: goldens ×3 (:499, :535, :571
  region), NDJSON ×2 (:603, :625 region), forced-diagnostic-in-json-mode (`stub.rs:218`), `ALL_CODES`
  extension (:234), GridPos row-first (:651 region), signature compile pins (:712). The survey's four
  gaps close one-for-one.
- The zero-production-change shape is the operator-recorded green-on-contact continuation
  (decisions ~10:45 PM, ~11:05 PM) — a reviewed decision under the brief's RED policy, not drift.
- Nothing else slipped in: `git diff 519947a --name-only` → exactly the two files; manifest diff
  empty; no `src/**`, `holler-pane/**`, `holler-pane-testkit/**`, `cli.rs`, `main.rs`, or
  `verb_harness/` changes; no new files, so no new top-level test target; zero added `unsafe` lines.
- The one t-green edit (the `Format` Copy move-and-use pin, :735-741) is a recorded, in-boundary
  test repair (handoff-T-green §Copy-pin, decisions ~11:30 PM) — a stronger pin, no new rule text.

**4. Boundary compliance: HELD.**

- Exactly the two permitted files, both in the brief's expected set
  (`pane_verbs/output_api.rs`, `pane_verbs/process/*`). No `Cargo.toml` edit (the testkit was
  already a dev-dep, as the survey promised). No submodule split was needed, so `main.rs` module
  declarations are untouched — consistent with plan-review finding 1 (a split is permitted when
  the size gate demands, never required).
- `output_api.rs` at 753 lines is under the 900-line fail gate (finding 1 above carries the warn).

## Notes for F

None — PASS. No parallel path to fold.

## For S / the operator (carried, not re-litigated here)

- The STUBS-riding diagnostic test expires when the last `pane` stub goes live (hedged in
  `stub.rs:229`'s expect message); the `--debug noisy` reading of "diagnostic" is a recorded
  assumption (decisions ~10:50 PM) the operator may still confirm.
- Workspace-suite flake under sibling-cargo load (`remote_admin_test`, `body_run_test`) is the
  journal's named pre-existing stability issue (~11:05/11:22 PM) — if it recurs at the merge-gate
  suite run it is not this diff's regression.
- Crate-wide pre-existing fmt drift (~2,916 hunks under this machine's rustfmt 1.9.0) predates the
  run; this run's two files are fmt-clean (T-green Tier 2). Whoever owns repo hygiene must
  reconcile before any fmt-gated CI on this branch.

## Patterns referenced

`crates/holler-pane-testkit/src/envelope.rs` (the 13-rule authority; rule 4's no-map-order doc,
rule 7's any-`data`) · `crates/holler-cli/tests/pane_verbs/list.rs`, `get.rs`, `watch.rs`,
`doctor/surface.rs` (the checker-consumer pattern this diff joins) ·
`crates/holler-cli/tests/verb_harness/mod.rs:82-97` (`one_envelope`, deliberately minimal,
untouched) · `output_api.rs:184-198` (the pre-existing compact-golden pattern the new goldens
continue) · `docs/handoffs/0660-output/{brief.md, survey.md, decisions.md}`.
